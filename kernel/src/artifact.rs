use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest as _, Sha256};

use crate::{BufferAccess, KernelAbi, KernelArgument, LoweringError, LoweringErrorKind};

const ELF64_HEADER_BYTES: usize = 64; const ELF64_SECTION_BYTES: usize = 64; const ELF64_SYMBOL_BYTES: usize = 24;
const ELF_CLASS_64: u8 = 2; const ELF_DATA_LITTLE_ENDIAN: u8 = 1; const ELF_OSABI_AMDGPU_HSA: u8 = 64;
// CUDA cubins have used multiple architecture-specific ELF OS-ABI tags.
// LLVM names 51 (0x33) as ELFOSABI_CUDA and 41 (0x29) as
// ELFOSABI_CUDA_V2. NVIDIA ptxas 13.3 emits 65 (0x41), paired with EM_CUDA
// and a cubin ABI version of 8. Keep the accepted set explicit: accepting
// every architecture-specific OS-ABI value would weaken artifact identity.
const ELF_OSABI_CUDA: u8 = 51; const ELF_OSABI_CUDA_V2: u8 = 41; const ELF_OSABI_CUDA_TOOLKIT_13_3: u8 = 65;
const ELF_MACHINE_AMDGPU: u16 = 224; const ELF_MACHINE_CUDA: u16 = 190; const ELF_SECTION_NOTE: u32 = 7;
const ELF_SECTION_NO_BITS: u32 = 8; const ELF_SECTION_SYMBOLS: u32 = 2; const ELF_SECTION_DYNAMIC_SYMBOLS: u32 = 11;
const ELF_UNDEFINED_SECTION: u16 = 0; const SYMBOL_BIND_GLOBAL: u8 = 1; const SYMBOL_BIND_WEAK: u8 = 2;
const SYMBOL_TYPE_OBJECT: u8 = 1; const SYMBOL_TYPE_FUNCTION: u8 = 2; const AMDGPU_METADATA_NOTE: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactDigest([u8; 32]);

impl ArtifactDigest {
	#[must_use]
	pub fn of(bytes: &[u8]) -> Self { digest(bytes) }

	#[must_use]
	pub const fn bytes(self) -> [u8; 32] { self.0 }

	#[must_use]
	pub fn to_hex(self) -> String { let mut result = String::with_capacity(64); for byte in self.0 {
			use std::fmt::Write as _;
			write!(result, "{byte:02x}").expect("writing to String");
		}
		result } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HsaKernelArgument { pub name: Option<String>, pub offset: u64, pub size: u64, pub value_kind: String,
	pub address_space: Option<String>, }

impl HsaKernelArgument {
	#[must_use]
	pub fn is_hidden(&self) -> bool { self.value_kind.starts_with("hidden_") }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HsaKernelMetadata { pub name: String, pub symbol: String, pub arguments: Vec<HsaKernelArgument>,
	pub kernarg_segment_size: u64, pub kernarg_segment_alignment: u64, pub group_segment_fixed_size: u64,
	pub private_segment_fixed_size: u64, pub maximum_workgroup_size: u64, pub wavefront_size: u64, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectedHsaco { pub digest: ArtifactDigest, pub elf_abi_version: u8, pub code_object_version: u8,
	pub elf_flags: u32, pub target_id: String, pub kernel: HsaKernelMetadata, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectedCubin { pub digest: ArtifactDigest, pub elf_abi_version: u8, pub elf_flags: u32, pub sm: u8,
	pub entry_symbol: String, pub text_bytes: u64, }

#[derive(Clone, Debug)]
struct ElfSection { name: String, kind: u32, flags: u64, offset: usize, size: usize, link: usize, entry_size: usize, }

#[derive(Clone, Debug)]
struct ElfSymbol { name: String, binding: u8, kind: u8, section: u16, size: u64, }

#[derive(Debug)]
struct ElfFile<'a> {
	bytes: &'a [u8],
	os_abi: u8, abi_version: u8, machine: u16, flags: u32, sections: Vec<ElfSection>, }

impl<'a> ElfFile<'a> {
	fn parse(bytes: &'a [u8]) -> Result<Self, LoweringError> {
		if bytes.len() < ELF64_HEADER_BYTES || bytes.get(0..4) != Some(b"\x7fELF") {
			return Err(artifact_format("artifact is not an ELF file"));
		}
		if bytes[4] != ELF_CLASS_64 || bytes[5] != ELF_DATA_LITTLE_ENDIAN { return Err(artifact_format(
				"artifact must be a little-endian ELF64 file",
			)); }
		let section_offset = usize_from_u64(read_u64(bytes, 40)?)?;
		let section_entry_size = usize::from(read_u16(bytes, 58)?); let section_count = usize::from(read_u16(bytes, 60)?);
		let string_section_index = usize::from(read_u16(bytes, 62)?); if section_entry_size != ELF64_SECTION_BYTES {
			return Err(artifact_format(format!(
				"unexpected ELF64 section-header size {section_entry_size}"
			))); }
		let table_bytes = section_entry_size .checked_mul(section_count) .and_then(|size| section_offset.checked_add(size))
			.ok_or_else(|| artifact_format("section table range overflowed"))?;
		if table_bytes > bytes.len() || string_section_index >= section_count {
			return Err(artifact_format("section table is out of bounds"));
		}
		let raw_string_section = raw_section( bytes, section_offset, section_entry_size, string_section_index, )?;
		if raw_string_section.kind == ELF_SECTION_NO_BITS { return Err(artifact_format(
				"ELF section-name string table has no file data",
			)); }
		let string_offset = usize_from_u64(raw_string_section.offset)?;
		let string_size = usize_from_u64(raw_string_section.size)?;
		let strings = subslice(bytes, string_offset, string_size)?;

		let mut sections = Vec::with_capacity(section_count); for index in 0..section_count {
			let raw = raw_section(bytes, section_offset, section_entry_size, index)?; sections.push(ElfSection {
				name: read_string(strings, usize_from_u32(raw.name)?)?.to_owned(), kind: raw.kind, flags: raw.flags,
				offset: usize_from_u64(raw.offset)?, size: usize_from_u64(raw.size)?, link: usize_from_u32(raw.link)?,
				entry_size: usize_from_u64(raw.entry_size)?, }); }
		for section in &sections { if section.kind != ELF_SECTION_NO_BITS { subslice(bytes, section.offset, section.size)?; }
		}
		Ok(Self { bytes, os_abi: bytes[7], abi_version: bytes[8], machine: read_u16(bytes, 18)?, flags: read_u32(bytes, 48)?,
			sections, }) }

	fn section(&self, name: &str) -> Option<&ElfSection> { self.sections.iter().find(|section| section.name == name) }

	fn section_data(&self, section: &ElfSection) -> Result<&'a [u8], LoweringError> {
		if section.kind == ELF_SECTION_NO_BITS { return Err(artifact_format(format!(
				"ELF section `{}` has no file data",
				section.name ))); }
		subslice(self.bytes, section.offset, section.size) }

	fn symbols(&self) -> Result<Vec<ElfSymbol>, LoweringError> { let mut symbols = Vec::new();
		for section in &self.sections { if !matches!( section.kind, ELF_SECTION_SYMBOLS | ELF_SECTION_DYNAMIC_SYMBOLS ) {
				continue; }
			if section.entry_size != ELF64_SYMBOL_BYTES || section.link >= self.sections.len() {
				return Err(artifact_format(format!(
					"invalid symbol table section {}",
					section.name ))); }
			let table = self.section_data(section)?; let strings = self.section_data(&self.sections[section.link])?;
			for entry in table.chunks_exact(ELF64_SYMBOL_BYTES) {
				let name = read_string(strings, usize_from_u32(read_u32(entry, 0)?)?)?; if name.is_empty() { continue; }
				symbols.push(ElfSymbol { name: name.to_owned(), binding: entry[4] >> 4, kind: entry[4] & 0x0f,
					section: read_u16(entry, 6)?, size: read_u64(entry, 16)?, }); } }
		symbols.sort_by(|left, right| { (&left.name, left.section, left.binding, left.kind, left.size).cmp(&( &right.name,
				right.section, right.binding, right.kind, right.size, )) }); symbols.dedup_by(|left, right| {
			left.name == right.name && left.section == right.section && left.binding == right.binding && left.kind == right.kind
				&& left.size == right.size }); Ok(symbols) } }

#[derive(Clone, Copy, Debug)]
struct RawSection { name: u32, kind: u32, flags: u64, offset: u64, size: u64, link: u32, entry_size: u64, }

fn raw_section( bytes: &[u8], table_offset: usize, entry_size: usize, index: usize,
) -> Result<RawSection, LoweringError> { let offset = index .checked_mul(entry_size)
		.and_then(|value| table_offset.checked_add(value))
		.ok_or_else(|| artifact_format("section header offset overflowed"))?;
	let entry = subslice(bytes, offset, entry_size)?; Ok(RawSection { name: read_u32(entry, 0)?, kind: read_u32(entry, 4)?,
		flags: read_u64(entry, 8)?, offset: read_u64(entry, 24)?, size: read_u64(entry, 32)?, link: read_u32(entry, 40)?,
		entry_size: read_u64(entry, 56)?, }) }

/// Structurally inspect an HSACO and match its complete entry ABI.
pub fn inspect_hsaco( bytes: &[u8], expected_target_id: &str, expected_code_object_version: u8,
	expected_abi: &KernelAbi, ) -> Result<InspectedHsaco, LoweringError> { let mut inspections = inspect_hsaco_bundle(
		bytes, expected_target_id, expected_code_object_version, std::iter::once(expected_abi), )?; Ok(inspections .pop()
		.expect("one requested HSACO ABI produces one inspection"))
}

/// Structurally inspect one multi-entry HSACO and match every requested ABI.
///
/// The ELF symbol table and AMDGPU MessagePack metadata are decoded once. The
/// returned inspections preserve the requested ABI order.
pub fn inspect_hsaco_bundle<'a>(
	bytes: &[u8], expected_target_id: &str, expected_code_object_version: u8,
	expected_abis: impl IntoIterator<Item = &'a KernelAbi>,
) -> Result<Vec<InspectedHsaco>, LoweringError> { let elf = ElfFile::parse(bytes)?;
	if elf.os_abi != ELF_OSABI_AMDGPU_HSA || elf.machine != ELF_MACHINE_AMDGPU { return Err(artifact_mismatch(format!(
			"expected AMDGPU-HSA ELF, got OS ABI {} machine {}",
			elf.os_abi, elf.machine ))); }
	let code_object_version = elf .abi_version .checked_add(2)
		.ok_or_else(|| artifact_format("code-object version overflowed"))?;
	if code_object_version != expected_code_object_version { return Err(artifact_mismatch(format!(
			"HSACO code-object version {code_object_version} does not match expected {expected_code_object_version}"
		))); }
	let symbols = elf.symbols()?; audit_symbols(&symbols)?; let defined_symbols = symbols .iter() .filter(|symbol| {
			symbol.section != ELF_UNDEFINED_SECTION && matches!(symbol.binding, SYMBOL_BIND_GLOBAL | SYMBOL_BIND_WEAK) })
		.map(|symbol| (symbol.name.as_str(), symbol.kind)) .collect::<BTreeSet<_>>(); let metadata = hsa_metadata(&elf)?;
	let target_id = metadata
		.string("amdhsa.target")
		.or_else(|| metadata.string(".amdhsa.target"))
		.ok_or_else(|| artifact_format("AMDGPU metadata omits its target ID"))?
		.to_owned(); if !target_ids_match(&target_id, expected_target_id) { return Err(artifact_mismatch(format!(
			"HSACO target `{target_id}` does not match expected `{expected_target_id}`"
		))); }
	let kernels = metadata
		.array("amdhsa.kernels")
		.or_else(|| metadata.array(".amdhsa.kernels"))
		.ok_or_else(|| artifact_format("AMDGPU metadata omits its kernel list"))?;
	let mut kernels_by_name = BTreeMap::new(); for value in kernels { let kernel = value .as_map()
			.ok_or_else(|| artifact_format("AMDGPU kernel metadata entry is not a map"))?;
		let name = kernel
			.string(".name")
			.ok_or_else(|| artifact_format("AMDGPU kernel metadata omits `.name`"))?;
		if kernels_by_name.insert(name, kernel).is_some() { return Err(artifact_format(format!(
				"AMDGPU metadata repeats kernel `{name}`"
			))); } }
	let artifact_digest = digest(bytes); expected_abis .into_iter() .map(|expected_abi| { require_indexed_symbol(
				&defined_symbols, &expected_abi.entry_symbol, SYMBOL_TYPE_FUNCTION, )?;
			let descriptor = format!("{}.kd", expected_abi.entry_symbol);
			require_indexed_symbol(&defined_symbols, &descriptor, SYMBOL_TYPE_OBJECT)?; let kernel = kernels_by_name
				.get(expected_abi.entry_symbol.as_str()) .ok_or_else(|| { artifact_mismatch(format!(
						"HSACO metadata has no kernel named `{}`",
						expected_abi.entry_symbol )) })?; let kernel = parse_hsa_kernel(kernel)?;
			validate_hsa_abi(&kernel, expected_abi)?; Ok(InspectedHsaco { digest: artifact_digest,
				elf_abi_version: elf.abi_version, code_object_version, elf_flags: elf.flags, target_id: target_id.clone(), kernel,
			}) }) .collect() }

fn require_indexed_symbol(symbols: &BTreeSet<(&str, u8)>, name: &str, kind: u8) -> Result<(), LoweringError> {
	if symbols.contains(&(name, kind)) { Ok(()) } else { Err(artifact_mismatch(format!(
			"artifact has no defined global symbol `{name}` of kind {kind}"
		))) } }

fn target_ids_match(actual: &str, expected: &str) -> bool {
	fn components(value: &str) -> Option<(&str, BTreeMap<&str, bool>)> {
		let value = value.rsplit_once("--").map_or(value, |(_, target)| target);
		let mut fields = value.split(':');
		let processor = fields.next()?; if processor.is_empty() { return None; }
		let mut features = BTreeMap::new(); for field in fields {
			let (name, polarity) = field.split_at(field.len().checked_sub(1)?); let enabled = match polarity {
				"+" => true,
				"-" => false,
				_ => return None, }; if name.is_empty() || features.insert(name, enabled).is_some() { return None; } }
		Some((processor, features)) }

	components(actual) == components(expected) }

/// Structurally inspect a cubin's target and exact entry symbol.
pub fn inspect_cubin( bytes: &[u8], expected_sm: u8, expected_entry_symbol: &str,
) -> Result<InspectedCubin, LoweringError> { let elf = ElfFile::parse(bytes)?;
	if !is_cuda_os_abi(elf.os_abi) || elf.machine != ELF_MACHINE_CUDA { return Err(artifact_mismatch(format!(
			"expected NVIDIA CUDA ELF, got OS ABI {} machine {}",
			elf.os_abi, elf.machine ))); }
	let sm = cubin_sm(elf.os_abi, elf.abi_version, elf.flags)?; if sm != expected_sm {
		return Err(artifact_mismatch(format!(
			"cubin targets sm_{sm}, expected sm_{expected_sm}"
		))); }
	let symbols = elf.symbols()?; audit_symbols(&symbols)?;
	let symbol = require_symbol(&symbols, expected_entry_symbol, SYMBOL_TYPE_FUNCTION)?;
	let text_name = format!(".text.{expected_entry_symbol}");
	let text = elf .section(&text_name)
		.ok_or_else(|| artifact_mismatch(format!("cubin has no `{text_name}` code section")))?;
	if text.size == 0 || text.flags & 0x4 == 0 || symbol.size == 0 { return Err(artifact_mismatch(
			"cubin entry point has no nonempty executable code",
		)); }
	Ok(InspectedCubin { digest: digest(bytes), elf_abi_version: elf.abi_version, elf_flags: elf.flags, sm,
		entry_symbol: expected_entry_symbol.to_owned(),
		text_bytes: u64::try_from(text.size).map_err(|_| artifact_format("cubin text size is out of range"))?,
	}) }

const fn is_cuda_os_abi(os_abi: u8) -> bool { matches!( os_abi,
		ELF_OSABI_CUDA | ELF_OSABI_CUDA_V2 | ELF_OSABI_CUDA_TOOLKIT_13_3 ) }

fn cubin_sm(os_abi: u8, abi_version: u8, flags: u32) -> Result<u8, LoweringError> {
	// CUDA ELF ABIs through the 0x33/0x29 tags store EF_CUDA_SM in the low
	// byte. The toolkit-13.3 ABI 8 layout identified by OS-ABI 0x41 stores
	// the decimal SM in the next byte and uses the low byte for ABI flags.
	// Decode only those observed, discriminated layouts instead of scanning
	// flags for a value that happens to equal the requested processor.
	let encoded = if os_abi == ELF_OSABI_CUDA_TOOLKIT_13_3 && abi_version >= 8 { (flags >> 8) & 0xff } else { flags & 0xff
	};
	u8::try_from(encoded).map_err(|_| artifact_format("cubin SM flag is out of range"))
}

fn validate_hsa_abi(metadata: &HsaKernelMetadata, expected: &KernelAbi) -> Result<(), LoweringError> {
	if metadata.symbol != format!("{}.kd", expected.entry_symbol) {
		return Err(artifact_mismatch(format!(
			"kernel descriptor `{}` does not match `{}`",
			metadata.symbol, expected.entry_symbol ))); }
	if metadata.kernarg_segment_alignment != u64::from(expected.argument_alignment) { return Err(artifact_mismatch(format!(
			"kernarg alignment {} does not match expected {}",
			metadata.kernarg_segment_alignment, expected.argument_alignment ))); }
	if metadata.maximum_workgroup_size < u64::from(expected.workgroup_lanes) { return Err(artifact_mismatch(format!(
			"kernel maximum workgroup size {} is smaller than required {}",
			metadata.maximum_workgroup_size, expected.workgroup_lanes ))); }
	if metadata.kernarg_segment_size < u64::from(expected.argument_bytes) { return Err(artifact_mismatch(format!(
			"kernarg segment {} is smaller than explicit ABI {}",
			metadata.kernarg_segment_size, expected.argument_bytes ))); }
	if metadata.arguments.len() < expected.arguments.len() { return Err(artifact_mismatch(
			"HSACO metadata omits explicit kernel arguments",
		)); }
	for (index, (actual, expected)) in metadata .arguments .iter() .zip(&expected.arguments) .enumerate()
	{
		// AMDHSA argument names are optional metadata and full LTO may omit
		// them. When present they remain a useful contradiction check; the
		// executable ABI itself is fixed by order, offset, size, value kind,
		// and address space below.
		let offset = u64::try_from(index) .ok() .and_then(|value| value.checked_mul(8))
			.ok_or_else(|| artifact_format("argument offset overflowed"))?;
		if actual.offset != offset || actual.size != 8 || actual.is_hidden() { return Err(artifact_mismatch(format!(
				"explicit argument {index} has unexpected metadata"
			))); }
		match expected { KernelArgument::Buffer { access, dtype: _, alignment: _, } => {
				if actual.value_kind != "global_buffer" || actual.address_space.as_deref() != Some("global") {
					return Err(artifact_mismatch(format!(
						"buffer argument {index} is not a global buffer"
					))); }
				let expected_prefix = match access {
					BufferAccess::Read => "input_",
					BufferAccess::Write => "output_",
				}; if actual .name .as_deref() .is_some_and(|name| !name.starts_with(expected_prefix))
				{ return Err(artifact_mismatch(format!(
						"buffer argument {index} has an unexpected name"
					))); } }
			KernelArgument::FaultFlag => { if actual .name .as_deref()
					.is_some_and(|name| name != "fault_flag")
					|| actual.value_kind != "global_buffer"
					|| actual.address_space.as_deref() != Some("global")
				{ return Err(artifact_mismatch(
						"fault-flag argument metadata does not match",
					)); } }
			KernelArgument::ElementCount => { if actual .name .as_deref()
					.is_some_and(|name| name != "element_count")
					|| actual.value_kind != "by_value"
				{ return Err(artifact_mismatch(
						"element-count argument metadata does not match",
					)); } }
			KernelArgument::RunId => {
				if actual.name.as_deref().is_some_and(|name| name != "run_id")
					|| actual.value_kind != "by_value"
				{
					return Err(artifact_mismatch("run-id argument metadata does not match"));
				} }
			KernelArgument::LoopIteration => { if actual .name .as_deref()
					.is_some_and(|name| name != "loop_iteration")
					|| actual.value_kind != "by_value"
				{ return Err(artifact_mismatch(
						"loop-iteration argument metadata does not match",
					)); } } } }
	if metadata.arguments[expected.arguments.len()..] .iter() .any(|argument| !argument.is_hidden())
	{ return Err(artifact_mismatch(
			"HSACO contains an unexpected non-hidden trailing argument",
		)); }
	Ok(()) }

fn require_symbol<'a>(symbols: &'a [ElfSymbol], name: &str, kind: u8) -> Result<&'a ElfSymbol, LoweringError> {
	symbols .iter() .find(|symbol| { symbol.name == name && symbol.kind == kind && symbol.section != ELF_UNDEFINED_SECTION
				&& matches!(symbol.binding, SYMBOL_BIND_GLOBAL | SYMBOL_BIND_WEAK) }) .ok_or_else(|| { artifact_mismatch(format!(
				"artifact has no defined global symbol `{name}` of kind {kind}"
			)) }) }

fn audit_symbols(symbols: &[ElfSymbol]) -> Result<(), LoweringError> { let mut undefined = BTreeSet::new();
	for symbol in symbols { if symbol.section == ELF_UNDEFINED_SECTION
			&& matches!(symbol.binding, SYMBOL_BIND_GLOBAL | SYMBOL_BIND_WEAK)
		{ undefined.insert(symbol.name.clone()); } }
	if !undefined.is_empty() { return Err(artifact_mismatch(format!(
			"artifact has unresolved global symbols: {}",
			undefined.into_iter().collect::<Vec<_>>().join(", ")
		))); }
	Ok(()) }

fn hsa_metadata(elf: &ElfFile<'_>) -> Result<MsgMap, LoweringError> {
	for section in &elf.sections { if section.kind != ELF_SECTION_NOTE { continue; }
		let notes = elf.section_data(section)?; let mut cursor = 0usize; while cursor < notes.len() {
			let names = usize_from_u32(read_u32(notes, cursor)?)?;
			let descriptions = usize_from_u32(read_u32(notes, cursor + 4)?)?; let note_type = read_u32(notes, cursor + 8)?;
			cursor = cursor .checked_add(12)
				.ok_or_else(|| artifact_format("note offset overflowed"))?;
			let name = subslice(notes, cursor, names)?; cursor = align_four( cursor.checked_add(names)
					.ok_or_else(|| artifact_format("note name overflowed"))?,
			)?; let description = subslice(notes, cursor, descriptions)?; cursor = align_four( cursor.checked_add(descriptions)
					.ok_or_else(|| artifact_format("note description overflowed"))?,
			)?; let name_end = name .iter() .position(|byte| *byte == 0) .unwrap_or(name.len());
			if note_type == AMDGPU_METADATA_NOTE && &name[..name_end] == b"AMDGPU" {
				let mut decoder = MsgDecoder::new(description); let value = decoder.decode()?; if !decoder.is_finished() {
					return Err(artifact_format("trailing bytes in AMDGPU metadata"));
				}
				return value .into_map()
					.ok_or_else(|| artifact_format("AMDGPU metadata root is not a map"));
			} } }
	Err(artifact_format("HSACO has no AMDGPU metadata note"))
}

fn parse_hsa_kernel(map: &MsgMap) -> Result<HsaKernelMetadata, LoweringError> { let arguments = map
		.array(".args")
		.ok_or_else(|| artifact_format("kernel metadata omits `.args`"))?
		.iter() .map(|argument| { let argument = argument .as_map()
				.ok_or_else(|| artifact_format("kernel argument is not a map"))?;
			Ok(HsaKernelArgument {
				name: argument.string(".name").map(str::to_owned),
				offset: argument
					.unsigned(".offset")
					.ok_or_else(|| artifact_format("kernel argument omits `.offset`"))?,
				size: argument
					.unsigned(".size")
					.ok_or_else(|| artifact_format("kernel argument omits `.size`"))?,
				value_kind: argument
					.string(".value_kind")
					.ok_or_else(|| artifact_format("kernel argument omits `.value_kind`"))?
					.to_owned(),
				address_space: argument.string(".address_space").map(str::to_owned),
			}) }) .collect::<Result<Vec<_>, LoweringError>>()?; Ok(HsaKernelMetadata {
		name: required_string(map, ".name")?,
		symbol: required_string(map, ".symbol")?,
		arguments,
		kernarg_segment_size: required_unsigned(map, ".kernarg_segment_size")?,
		kernarg_segment_alignment: required_unsigned(map, ".kernarg_segment_align")?,
		group_segment_fixed_size: required_unsigned(map, ".group_segment_fixed_size")?,
		private_segment_fixed_size: required_unsigned(map, ".private_segment_fixed_size")?,
		maximum_workgroup_size: required_unsigned(map, ".max_flat_workgroup_size")?,
		wavefront_size: required_unsigned(map, ".wavefront_size")?,
	}) }

fn required_string(map: &MsgMap, key: &str) -> Result<String, LoweringError> { map.string(key) .map(str::to_owned)
		.ok_or_else(|| artifact_format(format!("kernel metadata omits `{key}`")))
}

fn required_unsigned(map: &MsgMap, key: &str) -> Result<u64, LoweringError> { map.unsigned(key)
		.ok_or_else(|| artifact_format(format!("kernel metadata omits `{key}`")))
}

fn digest(bytes: &[u8]) -> ArtifactDigest { ArtifactDigest(Sha256::digest(bytes).into()) }

fn artifact_format(message: impl Into<String>) -> LoweringError { LoweringError::new(LoweringErrorKind::ArtifactFormat, message) }

fn artifact_mismatch(message: impl Into<String>) -> LoweringError { LoweringError::new(LoweringErrorKind::ArtifactMismatch, message) }

fn subslice(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8], LoweringError> { let end = offset
		.checked_add(size)
		.ok_or_else(|| artifact_format("byte range overflowed"))?;
	bytes.get(offset..end)
		.ok_or_else(|| artifact_format("byte range is out of bounds"))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, LoweringError> {
	let bytes: [u8; 2] = subslice(bytes, offset, 2)? .try_into()
		.map_err(|_| artifact_format("invalid u16 range"))?;
	Ok(u16::from_le_bytes(bytes)) }

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, LoweringError> {
	let bytes: [u8; 4] = subslice(bytes, offset, 4)? .try_into()
		.map_err(|_| artifact_format("invalid u32 range"))?;
	Ok(u32::from_le_bytes(bytes)) }

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, LoweringError> {
	let bytes: [u8; 8] = subslice(bytes, offset, 8)? .try_into()
		.map_err(|_| artifact_format("invalid u64 range"))?;
	Ok(u64::from_le_bytes(bytes)) }

fn read_string(bytes: &[u8], offset: usize) -> Result<&str, LoweringError> { let rest = bytes .get(offset..)
		.ok_or_else(|| artifact_format("string offset is out of bounds"))?;
	let end = rest .iter() .position(|byte| *byte == 0)
		.ok_or_else(|| artifact_format("string is not NUL terminated"))?;
	std::str::from_utf8(&rest[..end]).map_err(|_| artifact_format("ELF string table is not UTF-8"))
}

fn usize_from_u32(value: u32) -> Result<usize, LoweringError> { usize::try_from(value).map_err(|_| artifact_format("u32 does not fit usize")) }

fn usize_from_u64(value: u64) -> Result<usize, LoweringError> { usize::try_from(value).map_err(|_| artifact_format("u64 does not fit usize")) }

fn align_four(value: usize) -> Result<usize, LoweringError> { value.checked_add(3) .map(|value| value & !3)
		.ok_or_else(|| artifact_format("note alignment overflowed"))
}

type MsgMap = BTreeMap<String, MsgValue>;

trait MsgMapExt { fn string(&self, key: &str) -> Option<&str>; fn unsigned(&self, key: &str) -> Option<u64>;
	fn array(&self, key: &str) -> Option<&[MsgValue]>; }

impl MsgMapExt for MsgMap { fn string(&self, key: &str) -> Option<&str> { match self.get(key) {
			Some(MsgValue::String(value)) => Some(value), _ => None, } }

	fn unsigned(&self, key: &str) -> Option<u64> { match self.get(key) { Some(MsgValue::Unsigned(value)) => Some(*value),
			Some(MsgValue::Signed(value)) => u64::try_from(*value).ok(), _ => None, } }

	fn array(&self, key: &str) -> Option<&[MsgValue]> { match self.get(key) { Some(MsgValue::Array(value)) => Some(value),
			_ => None, } } }

#[derive(Clone, Debug, PartialEq, Eq)]
enum MsgValue { Nil, Boolean(bool), Unsigned(u64), Signed(i64), String(String), Array(Vec<Self>), Map(MsgMap), }

impl MsgValue { fn as_map(&self) -> Option<&MsgMap> { match self { Self::Map(value) => Some(value), _ => None, } }

	fn into_map(self) -> Option<MsgMap> { match self { Self::Map(value) => Some(value), _ => None, } } }

#[derive(Debug)]
struct MsgDecoder<'a> {
	bytes: &'a [u8],
	cursor: usize, depth: usize, }

impl<'a> MsgDecoder<'a> {
	fn new(bytes: &'a [u8]) -> Self {
		Self { bytes, cursor: 0, depth: 0, } }

	fn is_finished(&self) -> bool { self.cursor == self.bytes.len() }

	fn decode(&mut self) -> Result<MsgValue, LoweringError> { if self.depth >= 128 {
			return Err(artifact_format("MessagePack nesting exceeds 128 levels"));
		}
		self.depth += 1; let marker = self.byte()?; let value = match marker {
			0x00..=0x7f => MsgValue::Unsigned(u64::from(marker)), 0x80..=0x8f => self.map(usize::from(marker & 0x0f))?,
			0x90..=0x9f => self.array(usize::from(marker & 0x0f))?, 0xa0..=0xbf => self.string(usize::from(marker & 0x1f))?,
			0xc0 => MsgValue::Nil, 0xc2 => MsgValue::Boolean(false), 0xc3 => MsgValue::Boolean(true),
			0xcc => MsgValue::Unsigned(u64::from(self.byte()?)), 0xcd => MsgValue::Unsigned(u64::from(self.be_u16()?)),
			0xce => MsgValue::Unsigned(u64::from(self.be_u32()?)), 0xcf => MsgValue::Unsigned(self.be_u64()?),
			0xd0 => MsgValue::Signed(i64::from(i8::from_be_bytes([self.byte()?]))),
			0xd1 => MsgValue::Signed(i64::from(i16::from_be_bytes(self.take_array()?))),
			0xd2 => MsgValue::Signed(i64::from(i32::from_be_bytes(self.take_array()?))),
			0xd3 => MsgValue::Signed(i64::from_be_bytes(self.take_array()?)), 0xd9 => { let size = usize::from(self.byte()?);
				self.string(size)? }
			0xda => { let size = usize::from(self.be_u16()?); self.string(size)? }
			0xdb => { let size = usize_from_u32(self.be_u32()?)?; self.string(size)? }
			0xdc => { let size = usize::from(self.be_u16()?); self.array(size)? }
			0xdd => { let size = usize_from_u32(self.be_u32()?)?; self.array(size)? }
			0xde => { let size = usize::from(self.be_u16()?); self.map(size)? }
			0xdf => { let size = usize_from_u32(self.be_u32()?)?; self.map(size)? }
			0xe0..=0xff => MsgValue::Signed(i64::from(i8::from_be_bytes([marker]))), _ => { return Err(artifact_format(format!(
					"unsupported MessagePack marker 0x{marker:02x}"
				))); } }; self.depth -= 1; Ok(value) }

	fn array(&mut self, size: usize) -> Result<MsgValue, LoweringError> { let mut values = Vec::with_capacity(size);
		for _ in 0..size { values.push(self.decode()?); }
		Ok(MsgValue::Array(values)) }

	fn map(&mut self, size: usize) -> Result<MsgValue, LoweringError> { let mut values = MsgMap::new(); for _ in 0..size {
			let MsgValue::String(key) = self.decode()? else {
				return Err(artifact_format("MessagePack map key is not a string"));
			}; if values.insert(key.clone(), self.decode()?).is_some() { return Err(artifact_format(format!(
					"duplicate MessagePack map key `{key}`"
				))); } }
		Ok(MsgValue::Map(values)) }

	fn string(&mut self, size: usize) -> Result<MsgValue, LoweringError> { let bytes = self.take(size)?;
		let value = std::str::from_utf8(bytes).map_err(|_| artifact_format("MessagePack string is not UTF-8"))?;
		Ok(MsgValue::String(value.to_owned())) }

	fn byte(&mut self) -> Result<u8, LoweringError> { let byte = self .bytes .get(self.cursor) .copied()
			.ok_or_else(|| artifact_format("truncated MessagePack value"))?;
		self.cursor += 1; Ok(byte) }

	fn take(&mut self, size: usize) -> Result<&'a [u8], LoweringError> {
		let bytes = subslice(self.bytes, self.cursor, size)?; self.cursor = self .cursor .checked_add(size)
			.ok_or_else(|| artifact_format("MessagePack cursor overflowed"))?;
		Ok(bytes) }

	fn take_array<const SIZE: usize>(&mut self) -> Result<[u8; SIZE], LoweringError> { self.take(SIZE)? .try_into()
			.map_err(|_| artifact_format("truncated MessagePack integer"))
	}

	fn be_u16(&mut self) -> Result<u16, LoweringError> { Ok(u16::from_be_bytes(self.take_array()?)) }

	fn be_u32(&mut self) -> Result<u32, LoweringError> { Ok(u32::from_be_bytes(self.take_array()?)) }

	fn be_u64(&mut self) -> Result<u64, LoweringError> { Ok(u64::from_be_bytes(self.take_array()?)) } }
