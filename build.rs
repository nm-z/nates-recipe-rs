use std::{
	env,
	error::Error,
	fs, io,
	path::{Path, PathBuf},
	process::Command,
	sync::{
		Mutex,
		atomic::{AtomicUsize, Ordering},
	},
	thread,
};
#[derive(Clone, Copy, PartialEq, Eq)]
struct FloatLayout {
	sign: u8,
	exp: u8,
	man: u8,
}
impl FloatLayout {
	const fn new(sign: u8, exp: u8, man: u8) -> Self { Self { sign, exp, man } }
	const fn bits(self) -> u8 { self.sign + self.exp + self.man }
	fn unpack(self, bits: u64) -> f64 {
		let exponent_limit = (1u64 << self.exp) - 1;
		let mantissa_limit = 1u64 << self.man;
		let exponent = bits >> self.man & exponent_limit;
		let mantissa = bits & (mantissa_limit - 1);
		let magnitude = match (exponent, mantissa) {
			(value, 0) if value == exponent_limit => f64::INFINITY,
			(value, _) if value == exponent_limit => f64::NAN,
			(0, 0) => 0.0,
			(0, value) => 2.0f64.powi(1 - ((1i32 << (self.exp - 1)) - 1)) * value as f64 / mantissa_limit as f64,
			(value, man) => 2.0f64.powi(value as i32 - ((1i32 << (self.exp - 1)) - 1)) * (1.0 + man as f64 / mantissa_limit as f64),
		};
		if bits >> (self.exp + self.man) != 0 { -magnitude } else { magnitude }
	}
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct FloatFormat {
	arithmetic: FloatLayout,
	storage: FloatLayout,
}
impl FloatFormat {
	const FP8: Self = Self::native(1, 5, 2);
	const FP16: Self = Self::native(1, 5, 10);
	const FP32: Self = Self::native(1, 8, 23);
	const FP64: Self = Self::native(1, 11, 52);
	const BF16: Self = Self::native(1, 8, 7);
	const TF32: Self = Self { arithmetic: FloatLayout::new(1, 8, 10), storage: FloatLayout::new(1, 8, 23) };
	const fn native(sign: u8, exp: u8, man: u8) -> Self {
		let layout = FloatLayout::new(sign, exp, man);
		Self { arithmetic: layout, storage: layout }
	}
	const fn bytes(self) -> usize { self.storage.bits().div_ceil(8) as usize }
	fn pack(self, value: f64) -> u64 {
		let rounded = self.arithmetic.unpack(self.arithmetic.pack_from(value));
		let bits = match self.storage.bits() {
			64 => rounded.to_bits(),
			32 => u64::from((rounded as f32).to_bits()),
			16 if self == Self::FP16 => self.storage.pack_from(rounded),
			16 => u64::from((rounded as f32).to_bits() >> 16),
			8 => self.storage.pack_from(rounded),
			_ => unreachable!(),
		};
		bits
	}
	fn unpack(self, bits: u64) -> f64 {
		match self.storage.bits() {
			64 => f64::from_bits(bits),
			32 => f64::from(f32::from_bits(bits as u32)),
			16 if self == Self::FP16 => self.arithmetic.unpack(bits),
			16 => f64::from(f32::from_bits((bits as u32) << 16)),
			8 => self.arithmetic.unpack(bits),
			_ => unreachable!(),
		}
	}
}
impl FloatLayout {
	fn pack_from(self, value: f64) -> u64 {
		let sign = value.to_bits() >> 63 << (self.exp + self.man);
		let exponent_limit = (1u64 << self.exp) - 1;
		let mantissa_limit = 1u64 << self.man;
		if value.is_nan() {
			return sign | exponent_limit << self.man | 1u64 << (self.man - 1);
		}
		if value.is_infinite() {
			return sign | exponent_limit << self.man;
		}
		if value == 0.0 {
			return sign;
		}
		let bias = (1i32 << (self.exp - 1)) - 1;
		let magnitude = value.abs();
		let mut exponent = magnitude.log2().floor() as i32;
		if exponent < 1 - bias {
			let mantissa = (magnitude / 2.0f64.powi(1 - bias) * mantissa_limit as f64).round_ties_even() as u64;
			return if mantissa == mantissa_limit { sign | 1u64 << self.man } else { sign | mantissa };
		}
		let mut mantissa = ((magnitude / 2.0f64.powi(exponent) - 1.0) * mantissa_limit as f64).round_ties_even() as u64;
		if mantissa == mantissa_limit {
			mantissa = 0;
			exponent += 1
		}
		let stored = exponent + bias;
		if stored >= exponent_limit as i32 { sign | exponent_limit << self.man } else { sign | (stored as u64) << self.man | mantissa }
	}
}
#[derive(Clone, Copy)]
struct IntFormat {
	bits: u8,
}
impl IntFormat {
	const INT1: Self = Self { bits: 1 };
	const INT4: Self = Self { bits: 4 };
	const INT8: Self = Self { bits: 8 };
	const fn bytes(self) -> usize { self.bits.div_ceil(8) as usize }
	fn pack(self, value: f64) -> u64 { (value.round_ties_even() as i64).clamp(-(1i64 << (self.bits - 1)), (1i64 << (self.bits - 1)) - 1) as u64 & ((1u64 << self.bits) - 1) }
}
type BuildResult<T> = Result<T, Box<dyn Error>>;
const PARALLEL: &str = r#"@grid.count = internal addrspace(1) global i32 0, align 4
@grid.phase = internal addrspace(1) global i32 0, align 4
declare i32 @llvm.amdgcn.workgroup.id.x()
declare i32 @recipe.workgroup.size.x()
define internal i32 @global_id() #1 { entry:
%lane = call i32 @llvm.amdgcn.workitem.id.x() %group = call i32 @llvm.amdgcn.workgroup.id.x()
%width = call i32 @recipe.workgroup.size.x() %base = mul i32 %group, %width %id = add i32 %base, %lane ret i32 %id }
define internal void @grid_barrier(i32 %threads) #1 { entry: call void @llvm.amdgcn.s.barrier()
%lane = call i32 @llvm.amdgcn.workitem.id.x() %leader = icmp eq i32 %lane, 0
br i1 %leader, label %arrive, label %joined arrive: %width = call i32 @recipe.workgroup.size.x()
%groups = udiv i32 %threads, %width %phase = load atomic i32, ptr addrspace(1) @grid.phase acquire, align 4
%prior = atomicrmw add ptr addrspace(1) @grid.count, i32 1 acq_rel %limit = sub i32 %groups, 1
%last = icmp eq i32 %prior, %limit br i1 %last, label %release, label %wait release:
store atomic i32 0, ptr addrspace(1) @grid.count release, align 4 %next = xor i32 %phase, 1
store atomic i32 %next, ptr addrspace(1) @grid.phase release, align 4 br label %joined wait:
%seen = load atomic i32, ptr addrspace(1) @grid.phase acquire, align 4 %ready = icmp ne i32 %seen, %phase
br i1 %ready, label %joined, label %wait joined: call void @llvm.amdgcn.s.barrier() ret void }"#;
const AMD_WIDTH: &str = r#"declare ptr addrspace(4) @llvm.amdgcn.dispatch.ptr()
define internal i32 @recipe.workgroup.size.x() #1 { entry: %args = call ptr addrspace(4) @llvm.amdgcn.dispatch.ptr()
%address = getelementptr i8, ptr addrspace(4) %args, i32 4 %value = load i16, ptr addrspace(4) %address, align 2
%width = zext i16 %value to i32 ret i32 %width }"#;
fn parallel_ir(ir: String, width: &str) -> String {
	let mut ir = ir.replace("call i32 @llvm.amdgcn.workitem.id.x()", "call i32 @global_id()").replace("call void @llvm.amdgcn.s.barrier()", "call void @grid_barrier(i32 %threads)");
	let target = ir.find("target triple").and_then(|start| ir[start..].find('\n').map(|end| start + end + 1)).expect("kernel target triple is absent");
	ir.insert_str(target, &format!("{}\n", PARALLEL.replace("declare i32 @recipe.workgroup.size.x()", width)));
	ir.replace("recipe.local.id.x", "llvm.amdgcn.workitem.id.x").replace("recipe.group.id.x", "llvm.amdgcn.workgroup.id.x").replace("recipe.local.barrier", "llvm.amdgcn.s.barrier")
}
fn word(text: String, from: &str, to: &str) -> String {
	let (mut output, mut rest) = (String::with_capacity(text.len()), text.as_str());
	while let Some(index) = rest.find(from) {
		let end = index + from.len();
		let identifier = |value: char| value.is_ascii_alphanumeric() || value == '_' || value == '.';
		let bounded = rest[..index].chars().next_back().is_none_or(|value| !identifier(value)) && rest[end..].chars().next().is_none_or(|value| !identifier(value));
		output.push_str(&rest[..index]);
		output.push_str(if bounded { to } else { from });
		rest = &rest[end..];
	}
	output.push_str(rest);
	output
}
fn numeric_region(ir: &str) -> BuildResult<(usize, usize)> {
	let start = ir.find("; NUMERIC BEGIN").ok_or_else(|| io::Error::other("numeric operation block is absent"))?;
	let end = ir[start..].find("; NUMERIC END").map(|offset| start + offset + "; NUMERIC END".len()).ok_or_else(|| io::Error::other("numeric operation block is incomplete"))?;
	Ok((start, end))
}
fn math_names(ir: &str, bits: u8) -> [&'static str; 5] {
	match (ir.contains("@__nv_exp("), ir.contains("@exp("), bits) {
		(true, _, 64) => ["__nv_exp", "__nv_tanh", "__nv_cos", "__nv_sin", "__nv_log"],
		(true, _, _) => ["__nv_expf", "__nv_tanhf", "__nv_cosf", "__nv_sinf", "__nv_logf"],
		(false, true, 64) => ["exp", "tanh", "cos", "sin", "log"],
		(false, true, _) => ["expf", "tanhf", "cosf", "sinf", "logf"],
		(false, false, 64) => ["__ocml_exp_f64", "__ocml_tanh_f64", "__ocml_cos_f64", "__ocml_sin_f64", "__ocml_log_f64"],
		(false, false, _) => ["__ocml_exp_f32", "__ocml_tanh_f32", "__ocml_cos_f32", "__ocml_sin_f32", "__ocml_log_f32"],
	}
}
fn native_numeric(ir: &str, llvm: &str, format: FloatFormat) -> String {
	let bits = format.storage.bits();
	let rounded = format == FloatFormat::TF32;
	let wide = bits < 32;
	let math_type = if wide { "float" } else { llvm };
	let intrinsic = if wide {
		"f32"
	} else if bits == 64 {
		"f64"
	} else {
		"f32"
	};
	let math = math_names(ir, if wide { 32 } else { bits });
	let mut block = format!("; NUMERIC BEGIN\ndeclare {math_type} @llvm.sqrt.{intrinsic}({math_type}) declare {math_type} @llvm.fabs.{intrinsic}({math_type}) declare {math_type} @llvm.floor.{intrinsic}({math_type})\ndeclare {math_type} @{}({math_type}) declare {math_type} @{}({math_type}) declare {math_type} @{}({math_type}) declare {math_type} @{}({math_type}) declare {math_type} @{}({math_type})\n", math[0], math[1], math[2], math[3], math[4]);
	if rounded {
		block.push_str("define internal float @recipe.round(float %value) #1 { entry: %bits = bitcast float %value to i32 %absolute = and i32 %bits, 2147483647 %special = icmp uge i32 %absolute, 2139095040 %shifted = lshr i32 %bits, 13 %least = and i32 %shifted, 1 %bias = add i32 4095, %least %biased = add i32 %bits, %bias %masked = and i32 %biased, -8192 %encoded = select i1 %special, i32 %bits, i32 %masked %result = bitcast i32 %encoded to float ret float %result }\n")
	}
	for (name, opcode) in [("add", "fadd"), ("sub", "fsub"), ("mul", "fmul"), ("div", "fdiv")] {
		if rounded { block.push_str(&format!("define internal float @recipe.{name}(float %left, float %right) #1 {{ entry: %left.format = call float @recipe.round(float %left) %right.format = call float @recipe.round(float %right) %wide = {opcode} float %left.format, %right.format %result = call float @recipe.round(float %wide) ret float %result }}\n")) } else { block.push_str(&format!("define internal {llvm} @recipe.{name}({llvm} %left, {llvm} %right) #1 {{ entry: %result = {opcode} {llvm} %left, %right ret {llvm} %result }}\n")) }
	}
	if rounded {
		block.push_str("define internal float @recipe.neg(float %value) #1 { entry: %format = call float @recipe.round(float %value) %wide = fneg float %format %result = call float @recipe.round(float %wide) ret float %result }\n")
	} else {
		block.push_str(&format!("define internal {llvm} @recipe.neg({llvm} %value) #1 {{ entry: %result = fneg {llvm} %value ret {llvm} %result }}\n"))
	}
	for predicate in ["oeq", "oge", "ogt", "ole", "olt", "one", "ord"] {
		if rounded { block.push_str(&format!("define internal i1 @recipe.{predicate}(float %left, float %right) #1 {{ entry: %left.format = call float @recipe.round(float %left) %right.format = call float @recipe.round(float %right) %result = fcmp {predicate} float %left.format, %right.format ret i1 %result }}\n")) } else { block.push_str(&format!("define internal i1 @recipe.{predicate}({llvm} %left, {llvm} %right) #1 {{ entry: %result = fcmp {predicate} {llvm} %left, %right ret i1 %result }}\n")) }
	}
	if rounded {
		block.push_str("define internal float @recipe.from.u1(i1 %value) #1 { entry: %wide = uitofp i1 %value to float %result = call float @recipe.round(float %wide) ret float %result }\ndefine internal float @recipe.from.u32(i32 %value) #1 { entry: %wide = uitofp i32 %value to float %result = call float @recipe.round(float %wide) ret float %result }\ndefine internal float @recipe.from.s32(i32 %value) #1 { entry: %wide = sitofp i32 %value to float %result = call float @recipe.round(float %wide) ret float %result }\ndefine internal i32 @recipe.to.u32(float %value) #1 { entry: %format = call float @recipe.round(float %value) %result = fptoui float %format to i32 ret i32 %result }\ndefine internal i32 @recipe.to.s32(float %value) #1 { entry: %format = call float @recipe.round(float %value) %result = fptosi float %format to i32 ret i32 %result }\n")
	} else {
		block.push_str(&format!("define internal {llvm} @recipe.from.u1(i1 %value) #1 {{ entry: %result = uitofp i1 %value to {llvm} ret {llvm} %result }}\ndefine internal {llvm} @recipe.from.u32(i32 %value) #1 {{ entry: %result = uitofp i32 %value to {llvm} ret {llvm} %result }}\ndefine internal {llvm} @recipe.from.s32(i32 %value) #1 {{ entry: %result = sitofp i32 %value to {llvm} ret {llvm} %result }}\ndefine internal i32 @recipe.to.u32({llvm} %value) #1 {{ entry: %result = fptoui {llvm} %value to i32 ret i32 %result }}\ndefine internal i32 @recipe.to.s32({llvm} %value) #1 {{ entry: %result = fptosi {llvm} %value to i32 ret i32 %result }}\n"))
	}
	let from_f32 = match llvm {
		"double" => "%result = fpext float %value to double",
		"float" => "%result = fadd float %value, 0.0",
		"half" => "%result = fptrunc float %value to half",
		_ => "%result = fptrunc float %value to bfloat",
	};
	let from_f16 = match llvm {
		"double" => "%result = fpext half %value to double",
		"float" => "%result = fpext half %value to float",
		"half" => "%result = fadd half %value, 0.0",
		_ => "%wide = fpext half %value to float %result = fptrunc float %wide to bfloat",
	};
	let to_f16 = match llvm {
		"double" => "%result = fptrunc double %value to half",
		"float" => "%result = fptrunc float %value to half",
		"half" => "%result = fadd half %value, 0.0",
		_ => "%wide = fpext bfloat %value to float %result = fptrunc float %wide to half",
	};
	if rounded {
		block.push_str("define internal float @recipe.from.f32(float %value) #1 { entry: %result = call float @recipe.round(float %value) ret float %result }\ndefine internal float @recipe.from.f16(half %value) #1 { entry: %wide = fpext half %value to float %result = call float @recipe.round(float %wide) ret float %result }\ndefine internal half @recipe.to.f16(float %value) #1 { entry: %format = call float @recipe.round(float %value) %result = fptrunc float %format to half ret half %result }\n")
	} else {
		block.push_str(&format!("define internal {llvm} @recipe.from.f32(float %value) #1 {{ entry: {from_f32} ret {llvm} %result }}\ndefine internal {llvm} @recipe.from.f16(half %value) #1 {{ entry: {from_f16} ret {llvm} %result }}\ndefine internal half @recipe.to.f16({llvm} %value) #1 {{ entry: {to_f16} ret half %result }}\n"))
	}
	for (name, symbol) in [("abs", format!("llvm.fabs.{intrinsic}")), ("floor", format!("llvm.floor.{intrinsic}")), ("sqrt", format!("llvm.sqrt.{intrinsic}")), ("exp", math[0].to_owned()), ("tanh", math[1].to_owned()), ("cos", math[2].to_owned()), ("sin", math[3].to_owned()), ("log", math[4].to_owned())] {
		if wide {
			block.push_str(&format!("define internal {llvm} @recipe.{name}({llvm} %value) #1 {{ entry: %wide = fpext {llvm} %value to float %computed = call float @{symbol}(float %wide) %result = fptrunc float %computed to {llvm} ret {llvm} %result }}\n"))
		} else if rounded {
			block.push_str(&format!("define internal float @recipe.{name}(float %value) #1 {{ entry: %format = call float @recipe.round(float %value) %wide = call float @{symbol}(float %format) %result = call float @recipe.round(float %wide) ret float %result }}\n"))
		} else {
			block.push_str(&format!("define internal {llvm} @recipe.{name}({llvm} %value) #1 {{ entry: %result = call {llvm} @{symbol}({llvm} %value) ret {llvm} %result }}\n"))
		}
	}
	if rounded {
		block.push_str("define internal float @recipe.atomic.add(ptr addrspace(1) %target, float %value) #1 { entry: %value.format = call float @recipe.round(float %value) %initial = load atomic i32, ptr addrspace(1) %target monotonic, align 4 br label %attempt attempt: %prior.bits = phi i32 [ %initial, %entry ], [ %observed, %attempt ] %prior = bitcast i32 %prior.bits to float %prior.format = call float @recipe.round(float %prior) %sum = fadd float %prior.format, %value.format %rounded = call float @recipe.round(float %sum) %next = bitcast float %rounded to i32 %exchange = cmpxchg ptr addrspace(1) %target, i32 %prior.bits, i32 %next monotonic monotonic, align 4 %observed = extractvalue { i32, i1 } %exchange, 0 %stored = extractvalue { i32, i1 } %exchange, 1 br i1 %stored, label %done, label %attempt done: ret float %prior.format }\ndefine internal void @recipe.set.format(i32 %exp, i32 %man) #1 { entry: ret void }\n; NUMERIC END")
	} else {
		block.push_str(&format!("define internal {llvm} @recipe.atomic.add(ptr addrspace(1) %target, {llvm} %value) #1 {{ entry: %prior = atomicrmw fadd ptr addrspace(1) %target, {llvm} %value monotonic, align {} ret {llvm} %prior }}\ndefine internal void @recipe.set.format(i32 %exp, i32 %man) #1 {{ entry: ret void }}\n; NUMERIC END", bits / 8))
	}
	block
}
fn native_ir(ir: String, suffix: &str, llvm: &str, format: FloatFormat) -> BuildResult<String> {
	let (start, end) = numeric_region(&ir)?;
	let bits = format.storage.bits();
	let numeric = native_numeric(&ir, llvm, format);
	let mut kernel = format!("{}@RECIPE_NUMERIC@{}", &ir[..start], &ir[end..]);
	kernel = word(kernel, "double", llvm).replace("@contraction_tile", &format!("@contraction_tile{suffix}")).replace("@forward_graph", &format!("@forward_graph{suffix}")).replace("@tape_epoch_graph", &format!("@tape_epoch_graph{suffix}")).replace("align 8", &format!("align {}", bits / 8));
	if bits < 64 {
		let literal = |value: f64| match llvm {
			"half" => format!("0xH{:04X}", format.pack(value)),
			"bfloat" => format!("0xR{:04X}", format.pack(value)),
			_ => format!("0x{:016X}", format.unpack(format.pack(value)).to_bits()),
		};
		kernel = kernel.replace(&format!("{llvm} 0.1"), &format!("{llvm} {}", literal(0.1))).replace("0x3CB0000000000000", &literal(f64::from_bits(0x3CB0000000000000))).replace("0x3FEFFFFFFFFFFFFE", &literal(f64::from_bits(0x3FEFFFFFFFFFFFFE)))
	}
	Ok(kernel.replace("@RECIPE_NUMERIC@", &numeric))
}
fn custom_numeric(ir: &str) -> String {
	let math = math_names(ir, 64);
	let mut block = format!(
		r#"; NUMERIC BEGIN
@recipe_f_exp = internal addrspace(3) global i32 undef, align 4
@recipe_f_man = internal addrspace(3) global i32 undef, align 4
declare double @llvm.sqrt.f64(double)
declare double @llvm.fabs.f64(double)
declare double @llvm.floor.f64(double)
declare i64 @llvm.ctlz.i64(i64, i1)
declare double @llvm.roundeven.f64(double)
declare double @{}(double)
declare double @{}(double)
declare double @{}(double)
declare double @{}(double)
declare double @{}(double)
define internal void @recipe.set.format(i32 %exp, i32 %man) #1 {{ entry: store atomic i32 %exp, ptr addrspace(3) @recipe_f_exp monotonic, align 4 store atomic i32 %man, ptr addrspace(3) @recipe_f_man monotonic, align 4 ret void }}
define internal double @recipe.f.power(i64 %exponent) #2 {{ entry: %high = icmp sgt i64 %exponent, 1023 br i1 %high, label %infinity, label %low.test infinity: ret double 0x7FF0000000000000 low.test: %low = icmp slt i64 %exponent, -1074 br i1 %low, label %zero, label %finite zero: ret double 0.0 finite: %normal = icmp sge i64 %exponent, -1022 br i1 %normal, label %power.normal, label %power.subnormal power.normal: %biased = add i64 %exponent, 1023 %normal.bits = shl i64 %biased, 52 %normal.result = bitcast i64 %normal.bits to double ret double %normal.result power.subnormal: %shift = add i64 %exponent, 1074 %subnormal.bits = shl i64 1, %shift %subnormal.result = bitcast i64 %subnormal.bits to double ret double %subnormal.result }}
define internal double @recipe.round(double %value) #2 {{ entry: %source = bitcast double %value to i64 %sign.source = lshr i64 %source, 63 %absolute.bits = and i64 %source, 9223372036854775807 %absolute = bitcast i64 %absolute.bits to double %exp.word = load atomic i32, ptr addrspace(3) @recipe_f_exp monotonic, align 4 %man.word = load atomic i32, ptr addrspace(3) @recipe_f_man monotonic, align 4 %exp = zext i32 %exp.word to i64 %man = zext i32 %man.word to i64 %total = add i64 %exp, %man %sign = shl i64 %sign.source, %total %exp.shift = sub i64 %exp, 1 %bias.one = shl i64 1, %exp.shift %bias = sub i64 %bias.one, 1 %exponent.one = shl i64 1, %exp %exponent.limit = sub i64 %exponent.one, 1 %mantissa.limit = shl i64 1, %man %nan = fcmp uno double %value, %value br i1 %nan, label %encode.nan, label %infinite.test encode.nan: %quiet.shift = sub i64 %man, 1 %quiet = shl i64 1, %quiet.shift %special.exponent = shl i64 %exponent.limit, %man %nan.base = or i64 %sign, %special.exponent %nan.bits = or i64 %nan.base, %quiet %nan.result = call double @recipe.f.decode(i64 %nan.bits) ret double %nan.result infinite.test: %infinite = fcmp oeq double %absolute, 0x7FF0000000000000 br i1 %infinite, label %encode.infinity, label %zero.test encode.infinity: %infinity.exponent = shl i64 %exponent.limit, %man %infinity.bits = or i64 %sign, %infinity.exponent %infinity.result = call double @recipe.f.decode(i64 %infinity.bits) ret double %infinity.result zero.test: %zero = fcmp oeq double %absolute, 0.0 br i1 %zero, label %encode.zero, label %finite encode.zero: %zero.bits = shl i64 %sign.source, 63 %zero.result = bitcast i64 %zero.bits to double ret double %zero.result finite: %minimum = sub i64 1, %bias %source.exponent.shifted = lshr i64 %absolute.bits, 52 %source.exponent = and i64 %source.exponent.shifted, 2047 %source.mantissa = and i64 %absolute.bits, 4503599627370495 %source.normal = icmp ne i64 %source.exponent, 0 br i1 %source.normal, label %source.normal.exponent, label %source.subnormal.exponent source.normal.exponent: %normal.unbiased = sub i64 %source.exponent, 1023 br label %source.exponent.ready source.subnormal.exponent: %leading.zeros = call i64 @llvm.ctlz.i64(i64 %source.mantissa, i1 false) %highest = sub i64 63, %leading.zeros %subnormal.unbiased = sub i64 %highest, 1074 br label %source.exponent.ready source.exponent.ready: %unbiased = phi i64 [ %normal.unbiased, %source.normal.exponent ], [ %subnormal.unbiased, %source.subnormal.exponent ] %subnormal = icmp slt i64 %unbiased, %minimum br i1 %subnormal, label %encode.subnormal, label %encode.normal encode.subnormal: %minimum.power = call double @recipe.f.power(i64 %minimum) %subnormal.ratio = fdiv double %absolute, %minimum.power %subnormal.scaled = uitofp i64 %mantissa.limit to double %subnormal.value = fmul double %subnormal.ratio, %subnormal.scaled %subnormal.rounded = call double @llvm.roundeven.f64(double %subnormal.value) %subnormal.mantissa = fptoui double %subnormal.rounded to i64 %subnormal.carry = icmp eq i64 %subnormal.mantissa, %mantissa.limit %subnormal.encoded = select i1 %subnormal.carry, i64 %mantissa.limit, i64 %subnormal.mantissa %subnormal.bits = or i64 %sign, %subnormal.encoded %subnormal.result = call double @recipe.f.decode(i64 %subnormal.bits) ret double %subnormal.result encode.normal: %power = call double @recipe.f.power(i64 %unbiased) %ratio = fdiv double %absolute, %power %fraction = fsub double %ratio, 1.0 %mantissa.scale = uitofp i64 %mantissa.limit to double %mantissa.value = fmul double %fraction, %mantissa.scale %mantissa.rounded = call double @llvm.roundeven.f64(double %mantissa.value) %mantissa.initial = fptoui double %mantissa.rounded to i64 %carry = icmp eq i64 %mantissa.initial, %mantissa.limit %carry.value = zext i1 %carry to i64 %final.unbiased = add i64 %unbiased, %carry.value %mantissa = select i1 %carry, i64 0, i64 %mantissa.initial %stored = add i64 %final.unbiased, %bias %overflow = icmp sge i64 %stored, %exponent.limit br i1 %overflow, label %encode.infinity, label %pack pack: %stored.bits = shl i64 %stored, %man %normal.base = or i64 %sign, %stored.bits %normal.bits = or i64 %normal.base, %mantissa %normal.result = call double @recipe.f.decode(i64 %normal.bits) ret double %normal.result }}
define internal double @recipe.f.decode(i64 %bits) #2 {{ entry: %exp.word = load atomic i32, ptr addrspace(3) @recipe_f_exp monotonic, align 4 %man.word = load atomic i32, ptr addrspace(3) @recipe_f_man monotonic, align 4 %exp = zext i32 %exp.word to i64 %man = zext i32 %man.word to i64 %total = add i64 %exp, %man %negative.bit = lshr i64 %bits, %total %negative = icmp ne i64 %negative.bit, 0 %exp.one = shl i64 1, %exp %exp.limit = sub i64 %exp.one, 1 %man.limit = shl i64 1, %man %shifted = lshr i64 %bits, %man %exponent = and i64 %shifted, %exp.limit %man.mask = sub i64 %man.limit, 1 %mantissa = and i64 %bits, %man.mask %special = icmp eq i64 %exponent, %exp.limit br i1 %special, label %decode.special, label %finite decode.special: %infinity = icmp eq i64 %mantissa, 0 %special.value = select i1 %infinity, double 0x7FF0000000000000, double 0x7FF8000000000000 br label %signed finite: %zero.exp = icmp eq i64 %exponent, 0 br i1 %zero.exp, label %decode.subnormal, label %decode.normal decode.subnormal: %zero.man = icmp eq i64 %mantissa, 0 br i1 %zero.man, label %decode.zero, label %subnormal decode.zero: br label %signed subnormal: %bias.shift = sub i64 %exp, 1 %bias.one = shl i64 1, %bias.shift %bias = sub i64 %bias.one, 1 %subnormal.exponent = sub i64 1, %bias %subnormal.power = call double @recipe.f.power(i64 %subnormal.exponent) %subnormal.man = uitofp i64 %mantissa to double %subnormal.limit = uitofp i64 %man.limit to double %subnormal.fraction = fdiv double %subnormal.man, %subnormal.limit %subnormal.value = fmul double %subnormal.power, %subnormal.fraction br label %signed decode.normal: %normal.bias.shift = sub i64 %exp, 1 %normal.bias.one = shl i64 1, %normal.bias.shift %normal.bias = sub i64 %normal.bias.one, 1 %normal.exponent = sub i64 %exponent, %normal.bias %normal.power = call double @recipe.f.power(i64 %normal.exponent) %normal.man = uitofp i64 %mantissa to double %normal.limit = uitofp i64 %man.limit to double %normal.fraction = fdiv double %normal.man, %normal.limit %normal.significand = fadd double 1.0, %normal.fraction %normal.value = fmul double %normal.power, %normal.significand br label %signed signed: %magnitude = phi double [ %special.value, %decode.special ], [ 0.0, %decode.zero ], [ %subnormal.value, %subnormal ], [ %normal.value, %decode.normal ] %negated = fneg double %magnitude %result = select i1 %negative, double %negated, double %magnitude ret double %result }}
"#,
		math[0], math[1], math[2], math[3], math[4]
	);
	for (name, opcode) in [("add", "fadd"), ("sub", "fsub"), ("mul", "fmul"), ("div", "fdiv")] {
		block.push_str(&format!("define internal double @recipe.{name}(double %left, double %right) #1 {{ entry: %left.format = call double @recipe.round(double %left) %right.format = call double @recipe.round(double %right) %wide = {opcode} double %left.format, %right.format %result = call double @recipe.round(double %wide) ret double %result }}\n"))
	}
	block.push_str("define internal double @recipe.neg(double %value) #1 { entry: %format = call double @recipe.round(double %value) %wide = fneg double %format %result = call double @recipe.round(double %wide) ret double %result }\n");
	for predicate in ["oeq", "oge", "ogt", "ole", "olt", "one", "ord"] {
		block.push_str(&format!("define internal i1 @recipe.{predicate}(double %left, double %right) #1 {{ entry: %left.format = call double @recipe.round(double %left) %right.format = call double @recipe.round(double %right) %result = fcmp {predicate} double %left.format, %right.format ret i1 %result }}\n"))
	}
	block.push_str("define internal double @recipe.from.u1(i1 %value) #1 { entry: %wide = uitofp i1 %value to double %result = call double @recipe.round(double %wide) ret double %result }\ndefine internal double @recipe.from.u32(i32 %value) #1 { entry: %wide = uitofp i32 %value to double %result = call double @recipe.round(double %wide) ret double %result }\ndefine internal double @recipe.from.s32(i32 %value) #1 { entry: %wide = sitofp i32 %value to double %result = call double @recipe.round(double %wide) ret double %result }\ndefine internal i32 @recipe.to.u32(double %value) #1 { entry: %format = call double @recipe.round(double %value) %result = fptoui double %format to i32 ret i32 %result }\ndefine internal i32 @recipe.to.s32(double %value) #1 { entry: %format = call double @recipe.round(double %value) %result = fptosi double %format to i32 ret i32 %result }\ndefine internal double @recipe.from.f32(float %value) #1 { entry: %wide = fpext float %value to double %result = call double @recipe.round(double %wide) ret double %result }\ndefine internal double @recipe.from.f16(half %value) #1 { entry: %wide = fpext half %value to double %result = call double @recipe.round(double %wide) ret double %result }\ndefine internal half @recipe.to.f16(double %value) #1 { entry: %format = call double @recipe.round(double %value) %result = fptrunc double %format to half ret half %result }\n");
	for (name, symbol) in [("abs", "llvm.fabs.f64"), ("floor", "llvm.floor.f64"), ("sqrt", "llvm.sqrt.f64"), ("exp", math[0]), ("tanh", math[1]), ("cos", math[2]), ("sin", math[3]), ("log", math[4])] {
		block.push_str(&format!("define internal double @recipe.{name}(double %value) #1 {{ entry: %format = call double @recipe.round(double %value) %wide = call double @{symbol}(double %format) %result = call double @recipe.round(double %wide) ret double %result }}\n"))
	}
	block.push_str("define internal double @recipe.atomic.add(ptr addrspace(1) %target, double %value) #1 { entry: %value.format = call double @recipe.round(double %value) %initial = load atomic i64, ptr addrspace(1) %target monotonic, align 8 br label %attempt attempt: %prior.bits = phi i64 [ %initial, %entry ], [ %observed, %attempt ] %prior = bitcast i64 %prior.bits to double %prior.format = call double @recipe.round(double %prior) %sum = fadd double %prior.format, %value.format %rounded = call double @recipe.round(double %sum) %next = bitcast double %rounded to i64 %exchange = cmpxchg ptr addrspace(1) %target, i64 %prior.bits, i64 %next monotonic monotonic, align 8 %observed = extractvalue { i64, i1 } %exchange, 0 %stored = extractvalue { i64, i1 } %exchange, 1 br i1 %stored, label %done, label %attempt done: ret double %prior.format }\nattributes #2 = { noinline nounwind }\n; NUMERIC END");
	block
}
fn custom_ir(ir: String, suffix: &str) -> BuildResult<String> {
	let (start, end) = numeric_region(&ir)?;
	Ok(format!("{}@RECIPE_NUMERIC@{}", &ir[..start], &ir[end..]).replace("@contraction_tile", &format!("@contraction_tile{suffix}")).replace("@forward_graph", &format!("@forward_graph{suffix}")).replace("@tape_epoch_graph", &format!("@tape_epoch_graph{suffix}")).replace("@RECIPE_NUMERIC@", &custom_numeric(&ir)))
}
fn encoded_numeric(ir: &str, llvm: &str, align: u8, codec: &str) -> String {
	let math = math_names(ir, 32);
	let mut block = format!("; NUMERIC BEGIN\ndeclare float @llvm.sqrt.f32(float) declare float @llvm.fabs.f32(float) declare float @llvm.floor.f32(float)\ndeclare float @{}(float) declare float @{}(float) declare float @{}(float) declare float @{}(float) declare float @{}(float)\n{codec}\n", math[0], math[1], math[2], math[3], math[4]);
	for (name, opcode) in [("add", "fadd"), ("sub", "fsub"), ("mul", "fmul"), ("div", "fdiv")] {
		block.push_str(&format!("define internal {llvm} @recipe.{name}({llvm} %left, {llvm} %right) #1 {{ entry: %left.wide = call float @recipe.decode({llvm} %left) %right.wide = call float @recipe.decode({llvm} %right) %wide = {opcode} float %left.wide, %right.wide %result = call {llvm} @recipe.encode(float %wide) ret {llvm} %result }}\n"))
	}
	block.push_str(&format!("define internal {llvm} @recipe.neg({llvm} %value) #1 {{ entry: %decoded = call float @recipe.decode({llvm} %value) %wide = fneg float %decoded %result = call {llvm} @recipe.encode(float %wide) ret {llvm} %result }}\n"));
	for predicate in ["oeq", "oge", "ogt", "ole", "olt", "one", "ord"] {
		block.push_str(&format!("define internal i1 @recipe.{predicate}({llvm} %left, {llvm} %right) #1 {{ entry: %left.wide = call float @recipe.decode({llvm} %left) %right.wide = call float @recipe.decode({llvm} %right) %result = fcmp {predicate} float %left.wide, %right.wide ret i1 %result }}\n"))
	}
	block.push_str(&format!("define internal {llvm} @recipe.from.u1(i1 %value) #1 {{ entry: %wide = uitofp i1 %value to float %result = call {llvm} @recipe.encode(float %wide) ret {llvm} %result }}\ndefine internal {llvm} @recipe.from.u32(i32 %value) #1 {{ entry: %wide = uitofp i32 %value to float %result = call {llvm} @recipe.encode(float %wide) ret {llvm} %result }}\ndefine internal {llvm} @recipe.from.s32(i32 %value) #1 {{ entry: %wide = sitofp i32 %value to float %result = call {llvm} @recipe.encode(float %wide) ret {llvm} %result }}\ndefine internal i32 @recipe.to.u32({llvm} %value) #1 {{ entry: %wide = call float @recipe.decode({llvm} %value) %result = fptoui float %wide to i32 ret i32 %result }}\ndefine internal i32 @recipe.to.s32({llvm} %value) #1 {{ entry: %wide = call float @recipe.decode({llvm} %value) %result = fptosi float %wide to i32 ret i32 %result }}\ndefine internal {llvm} @recipe.from.f32(float %value) #1 {{ entry: %result = call {llvm} @recipe.encode(float %value) ret {llvm} %result }}\ndefine internal {llvm} @recipe.from.f16(half %value) #1 {{ entry: %wide = fpext half %value to float %result = call {llvm} @recipe.encode(float %wide) ret {llvm} %result }}\ndefine internal half @recipe.to.f16({llvm} %value) #1 {{ entry: %wide = call float @recipe.decode({llvm} %value) %result = fptrunc float %wide to half ret half %result }}\n"));
	for (name, symbol) in [("abs", "llvm.fabs.f32"), ("floor", "llvm.floor.f32"), ("sqrt", "llvm.sqrt.f32"), ("exp", math[0]), ("tanh", math[1]), ("cos", math[2]), ("sin", math[3]), ("log", math[4])] {
		block.push_str(&format!("define internal {llvm} @recipe.{name}({llvm} %value) #1 {{ entry: %decoded = call float @recipe.decode({llvm} %value) %wide = call float @{symbol}(float %decoded) %result = call {llvm} @recipe.encode(float %wide) ret {llvm} %result }}\n"))
	}
	block.push_str(&format!("define internal {llvm} @recipe.atomic.add(ptr addrspace(1) %target, {llvm} %value) #1 {{ entry: %initial = load atomic {llvm}, ptr addrspace(1) %target monotonic, align {align} br label %attempt attempt: %prior = phi {llvm} [ %initial, %entry ], [ %observed, %attempt ] %prior.wide = call float @recipe.decode({llvm} %prior) %value.wide = call float @recipe.decode({llvm} %value) %sum = fadd float %prior.wide, %value.wide %next = call {llvm} @recipe.encode(float %sum) %exchange = cmpxchg ptr addrspace(1) %target, {llvm} %prior, {llvm} %next monotonic monotonic, align {align} %observed = extractvalue {{ {llvm}, i1 }} %exchange, 0 %stored = extractvalue {{ {llvm}, i1 }} %exchange, 1 br i1 %stored, label %done, label %attempt done: ret {llvm} %prior }}\ndefine internal void @recipe.set.format(i32 %exp, i32 %man) #1 {{ entry: ret void }}\n; NUMERIC END"));
	block
}
fn fp8_codec() -> &'static str {
	r#"define internal float @recipe.decode(i8 %value) #1 { entry: %wide = zext i8 %value to i32 %sign = and i32 %wide, 128 %exponent.shifted = lshr i32 %wide, 2 %exponent = and i32 %exponent.shifted, 31 %mantissa = and i32 %wide, 3 %zero.exponent = icmp eq i32 %exponent, 0 br i1 %zero.exponent, label %subnormal, label %nonzero subnormal: %mantissa.float = uitofp i32 %mantissa to float %negative = icmp ne i32 %sign, 0 %subnormal.value = select i1 %negative, float 0xBEF0000000000000, float 0x3EF0000000000000 %scaled = fmul float %mantissa.float, %subnormal.value ret float %scaled nonzero: %special = icmp eq i32 %exponent, 31 %biased.exponent = add i32 %exponent, 112 %float.exponent = select i1 %special, i32 255, i32 %biased.exponent %float.sign = shl i32 %sign, 24 %float.exponent.bits = shl i32 %float.exponent, 23 %float.mantissa = shl i32 %mantissa, 21 %signed = or i32 %float.sign, %float.exponent.bits %bits = or i32 %signed, %float.mantissa %result = bitcast i32 %bits to float ret float %result }
define internal i8 @recipe.encode(float %value) #1 { entry: %bits = bitcast float %value to i32 %sign.shifted = lshr i32 %bits, 24 %sign = and i32 %sign.shifted, 128 %absolute = and i32 %bits, 2147483647 %exponent.shifted = lshr i32 %absolute, 23 %exponent = and i32 %exponent.shifted, 255 %mantissa = and i32 %absolute, 8388607 %special = icmp eq i32 %exponent, 255 br i1 %special, label %encode.special, label %finite encode.special: %nan = icmp ne i32 %mantissa, 0 %special.mantissa = select i1 %nan, i32 2, i32 0 %special.base = or i32 %sign, 124 %special.bits = or i32 %special.base, %special.mantissa %special.result = trunc i32 %special.bits to i8 ret i8 %special.result finite: %zero = icmp eq i32 %exponent, 0 br i1 %zero, label %encode.zero, label %range encode.zero: %zero.result = trunc i32 %sign to i8 ret i8 %zero.result range: %unbiased = sub i32 %exponent, 127 %overflow = icmp sgt i32 %unbiased, 15 br i1 %overflow, label %encode.infinity, label %normal.test encode.infinity: %infinity.bits = or i32 %sign, 124 %infinity.result = trunc i32 %infinity.bits to i8 ret i8 %infinity.result normal.test: %normal = icmp sge i32 %unbiased, -14 br i1 %normal, label %encode.normal, label %subnormal.test encode.normal: %stored = add i32 %unbiased, 15 %top = lshr i32 %mantissa, 21 %remainder = and i32 %mantissa, 2097151 %above = icmp ugt i32 %remainder, 1048576 %tie = icmp eq i32 %remainder, 1048576 %odd.bit = and i32 %top, 1 %odd = icmp ne i32 %odd.bit, 0 %tie.odd = and i1 %tie, %odd %round = or i1 %above, %tie.odd %increment = zext i1 %round to i32 %rounded = add i32 %top, %increment %carry = lshr i32 %rounded, 2 %final.exponent = add i32 %stored, %carry %rounded.overflow = icmp uge i32 %final.exponent, 31 br i1 %rounded.overflow, label %encode.infinity, label %normal.pack normal.pack: %final.mantissa = and i32 %rounded, 3 %exponent.bits = shl i32 %final.exponent, 2 %normal.base = or i32 %sign, %exponent.bits %normal.bits = or i32 %normal.base, %final.mantissa %normal.result = trunc i32 %normal.bits to i8 ret i8 %normal.result subnormal.test: %too.small = icmp slt i32 %unbiased, -17 br i1 %too.small, label %encode.zero, label %encode.subnormal encode.subnormal: %significand = or i32 %mantissa, 8388608 %shift = sub i32 7, %unbiased %subnormal.top = lshr i32 %significand, %shift %one = shl i32 1, %shift %mask = sub i32 %one, 1 %subnormal.remainder = and i32 %significand, %mask %half.shift = sub i32 %shift, 1 %half = shl i32 1, %half.shift %subnormal.above = icmp ugt i32 %subnormal.remainder, %half %subnormal.tie = icmp eq i32 %subnormal.remainder, %half %subnormal.odd.bit = and i32 %subnormal.top, 1 %subnormal.odd = icmp ne i32 %subnormal.odd.bit, 0 %subnormal.tie.odd = and i1 %subnormal.tie, %subnormal.odd %subnormal.round = or i1 %subnormal.above, %subnormal.tie.odd %subnormal.increment = zext i1 %subnormal.round to i32 %subnormal.mantissa = add i32 %subnormal.top, %subnormal.increment %subnormal.bits = or i32 %sign, %subnormal.mantissa %subnormal.result = trunc i32 %subnormal.bits to i8 ret i8 %subnormal.result }"#
}
fn bf16_codec() -> &'static str {
	r#"define internal float @recipe.decode(i16 %value) #1 { entry: %wide = zext i16 %value to i32 %bits = shl i32 %wide, 16 %result = bitcast i32 %bits to float ret float %result }
define internal i16 @recipe.encode(float %value) #1 { entry: %bits = bitcast float %value to i32 %absolute = and i32 %bits, 2147483647 %special = icmp uge i32 %absolute, 2139095040 %upper = lshr i32 %bits, 16 %mantissa = and i32 %absolute, 8388607 %nan = icmp ne i32 %mantissa, 0 %quiet = or i32 %upper, 64 %special.bits = select i1 %nan, i32 %quiet, i32 %upper %lower = and i32 %bits, 65535 %above = icmp ugt i32 %lower, 32768 %tie = icmp eq i32 %lower, 32768 %odd.bit = and i32 %upper, 1 %odd = icmp ne i32 %odd.bit, 0 %tie.odd = and i1 %tie, %odd %round = or i1 %above, %tie.odd %increment = zext i1 %round to i32 %rounded = add i32 %upper, %increment %encoded = select i1 %special, i32 %special.bits, i32 %rounded %result = trunc i32 %encoded to i16 ret i16 %result }"#
}
fn float_codec(native: &str, llvm: &str) -> String { format!("define internal float @recipe.decode({llvm} %value) #1 {{ entry: %native = bitcast {llvm} %value to {native} %result = fpext {native} %native to float ret float %result }}\ndefine internal {llvm} @recipe.encode(float %value) #1 {{ entry: %native = fptrunc float %value to {native} %result = bitcast {native} %native to {llvm} ret {llvm} %result }}") }
fn encoded_ir(ir: String, suffix: &str, bytes: usize, codec: &str, pack: impl Fn(f64) -> u64) -> BuildResult<String> {
	let (start, end) = numeric_region(&ir)?;
	let llvm = match bytes {
		1 => "i8",
		2 => "i16",
		4 => "i32",
		_ => "i64",
	};
	let numeric = encoded_numeric(&ir, llvm, bytes as u8, codec);
	let mut kernel = word(format!("{}@RECIPE_NUMERIC@{}", &ir[..start], &ir[end..]), "double", llvm).replace("@contraction_tile", &format!("@contraction_tile{suffix}")).replace("@forward_graph", &format!("@forward_graph{suffix}")).replace("@tape_epoch_graph", &format!("@tape_epoch_graph{suffix}")).replace("align 8", &format!("align {bytes}"));
	for (source, value) in [("-2.0", -2.0), ("-1.0", -1.0), ("0.0", 0.0), ("0.1", 0.1), ("0.5", 0.5), ("1.0", 1.0), ("2.0", 2.0)] {
		kernel = word(kernel, source, &pack(value).to_string())
	}
	for bits in [0x3CB0000000000000, 0x3FEFFFFFFFFFFFFE, 0xFFF0000000000000, 0x7FF8000000000000] {
		kernel = kernel.replace(&format!("0x{bits:016X}"), &format!("{}", pack(f64::from_bits(bits))))
	}
	Ok(kernel.replace("@RECIPE_NUMERIC@", &numeric))
}
fn int_codec(format: IntFormat) -> String {
	let (bits, mask) = (format.bits, (1u16 << format.bits) - 1);
	let shift = 32 - bits;
	let minimum = -(1i16 << (bits - 1));
	let maximum = (1i16 << (bits - 1)) - 1;
	format!("declare float @llvm.roundeven.f32(float)\ndefine internal float @recipe.decode(i8 %value) #1 {{ entry: %wide = zext i8 %value to i32 %masked = and i32 %wide, {mask} %shifted = shl i32 %masked, {shift} %signed = ashr i32 %shifted, {shift} %result = sitofp i32 %signed to float ret float %result }}\ndefine internal i8 @recipe.encode(float %value) #1 {{ entry: %nan = fcmp uno float %value, %value %rounded = call float @llvm.roundeven.f32(float %value) %below = fcmp olt float %rounded, {minimum}.0 %above = fcmp ogt float %rounded, {maximum}.0 %lowered = select i1 %below, float {minimum}.0, float %rounded %clamped = select i1 %above, float {maximum}.0, float %lowered %finite = select i1 %nan, float 0.0, float %clamped %integer = fptosi float %finite to i32 %masked = and i32 %integer, {mask} %result = trunc i32 %masked to i8 ret i8 %result }}")
}
fn setting<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	let prefix = format!("{key} = ");
	manifest.lines().find_map(|line| line.trim().strip_prefix(&prefix)).ok_or_else(|| io::Error::other(format!("{key} must be configured")).into())
}
fn number<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	let value = setting(manifest, key)?;
	value.parse::<f64>().map_err(|error| io::Error::other(format!("{key} must be numeric: {error}")))?;
	Ok(value)
}
fn flag<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> {
	let value = setting(manifest, key)?;
	value.parse::<bool>().map_err(|error| io::Error::other(format!("{key} must be true or false: {error}")))?;
	Ok(value)
}
fn text<'a>(manifest: &'a str, key: &str) -> BuildResult<&'a str> { setting(manifest, key)?.strip_prefix('"').and_then(|value| value.strip_suffix('"')).ok_or_else(|| io::Error::other(format!("{key} must be quoted")).into()) }
fn architectures(manifest: &str) -> BuildResult<Vec<String>> {
	let directory = text(manifest, "hsa-device-library-directory")?;
	let mut values = Vec::new();
	for entry in fs::read_dir(directory)? {
		let name = entry?.file_name().into_string().map_err(|_| io::Error::other("ISA library name is not UTF-8"))?;
		if let Some(version) = name.strip_prefix("oclc_isa_version_").and_then(|value| value.strip_suffix(".bc"))
			&& !version.contains('-')
		{
			values.push(format!("gfx{version}"));
		}
	}
	values.sort();
	values.dedup();
	if values.is_empty() {
		return Err(io::Error::other("hsa-device-library-directory has no ISA libraries").into());
	}
	Ok(values)
}
fn run(command: &mut Command, role: &str) -> BuildResult<()> {
	let status = command.status()?;
	if !status.success() {
		return Err(io::Error::other(format!("{role} failed")).into());
	}
	Ok(())
}
fn render(command: &mut Command, role: &str, path: &Path) -> BuildResult<()> {
	let output = command.output()?;
	if !output.status.success() {
		let detail = String::from_utf8_lossy(&output.stderr);
		return Err(io::Error::other(format!("{role} failed: {}", detail.trim())).into());
	}
	fs::write(path, output.stdout)?;
	Ok(())
}
fn compile_amd_matrix(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let ir = parallel_ir(fs::read_to_string("amd-nv-cpu.ll")?, AMD_WIDTH);
	let sources = [("", native_ir(ir.clone(), "", "double", FloatFormat::FP64)?), ("-f32", native_ir(ir.clone(), "_f32", "float", FloatFormat::FP32)?), ("-f16", encoded_ir(ir.clone(), "_f16", FloatFormat::FP16.bytes(), &float_codec("half", "i16"), |value| FloatFormat::FP16.pack(value))?), ("-f8", encoded_ir(ir.clone(), "_f8", FloatFormat::FP8.bytes(), fp8_codec(), |value| FloatFormat::FP8.pack(value))?), ("-bf16", encoded_ir(ir.clone(), "_bf16", FloatFormat::BF16.bytes(), bf16_codec(), |value| FloatFormat::BF16.pack(value))?), ("-tf32", native_ir(ir.clone(), "_tf32", "float", FloatFormat::TF32)?), ("-int8", encoded_ir(ir.clone(), "_int8", IntFormat::INT8.bytes(), &int_codec(IntFormat::INT8), |value| IntFormat::INT8.pack(value))?), ("-int4", encoded_ir(ir.clone(), "_int4", IntFormat::INT4.bytes(), &int_codec(IntFormat::INT4), |value| IntFormat::INT4.pack(value))?), ("-int1", encoded_ir(ir.clone(), "_int1", IntFormat::INT1.bytes(), &int_codec(IntFormat::INT1), |value| IntFormat::INT1.pack(value))?), ("-f", custom_ir(ir, "_f")?)]
		.map(|(suffix, contents)| {
			let path = out.join(format!("recipe-amd{suffix}.ll"));
			fs::write(&path, contents).map(|_| (suffix, path))
		})
		.into_iter()
		.collect::<io::Result<Vec<_>>>()?;
	let compiler = text(manifest, "hsa-compiler")?;
	let libraries = [text(manifest, "hsa-device-library")?, text(manifest, "hsa-clock-library")?, text(manifest, "hsa-finite-library")?, text(manifest, "hsa-math-library")?];
	let library_directory = text(manifest, "hsa-device-library-directory")?;
	let disassembler = text(manifest, "hsa-disassembler")?;
	let architectures = architectures(manifest)?;
	let workers = env::var("NUM_JOBS")?.parse::<usize>()?.min(architectures.len());
	let next = AtomicUsize::new(0);
	let artifacts = Mutex::new(Vec::new());
	thread::scope(|scope| {
		for _ in 0..workers {
			scope.spawn(|| {
				loop {
					let index = next.fetch_add(1, Ordering::Relaxed);
					let Some(architecture) = architectures.get(index) else { break };
					let mut compiled = Vec::new();
					for (suffix, source) in &sources {
						let output = out.join(format!("recipe-{architecture}{suffix}.hsaco"));
						let mut command = Command::new(compiler);
						command.args(["-target", "amdgcn-amd-amdhsa", &format!("-mcpu={architecture}"), "-O2", "-nogpulib"]);
						for library in libraries {
							command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang", library]);
						}
						let isa = Path::new(library_directory).join(format!("oclc_isa_version_{}.bc", architecture.trim_start_matches("gfx")));
						command.args(["-Xclang", "-mlink-builtin-bitcode", "-Xclang"]).arg(isa).arg(source).arg("-o").arg(&output);
						if run(&mut command, "HSA LLVM IR compiler").is_err() {
							compiled.clear();
							break;
						}
						compiled.push(output)
					}
					if compiled.len() != sources.len() {
						continue;
					}
					let assembly = out.join(format!("recipe-{architecture}.amd.s"));
					if render(Command::new(disassembler).arg("--disassemble").arg(&compiled[0]), "HSA disassembler", &assembly).is_err() {
						continue;
					}
					artifacts.lock().unwrap().push((index, compiled, assembly))
				}
			});
		}
	});
	let mut artifacts = artifacts.into_inner().unwrap();
	artifacts.sort_by_key(|(index, _, _)| *index);
	let (mut objects, mut embeds): ([Vec<String>; 10], [Vec<String>; 10]) = (std::array::from_fn(|_| Vec::new()), std::array::from_fn(|_| Vec::new()));
	let mut assemblies = Vec::new();
	for (architecture_index, compiled, assembly) in artifacts {
		let architecture = &architectures[architecture_index];
		for (format_index, output) in compiled.iter().enumerate() {
			objects[format_index].push(format!("{architecture}={}", output.display()));
			embeds[format_index].push(format!("(\"{architecture}\", include_bytes!(\"{}\").as_slice()),", output.display()))
		}
		assemblies.push(format!("{architecture}={}", assembly.display()));
	}
	if objects[0].is_empty() {
		return Err(io::Error::other("no HSA architecture compiled").into());
	}
	let names = [("HSA_CODE_OBJECTS", "RECIPE_HSA_CODE_OBJECTS"), ("HSA_F32_CODE_OBJECTS", "RECIPE_HSA_F32_CODE_OBJECTS"), ("HSA_F16_CODE_OBJECTS", "RECIPE_HSA_F16_CODE_OBJECTS"), ("HSA_F8_CODE_OBJECTS", "RECIPE_HSA_F8_CODE_OBJECTS"), ("HSA_BF16_CODE_OBJECTS", "RECIPE_HSA_BF16_CODE_OBJECTS"), ("HSA_TF32_CODE_OBJECTS", "RECIPE_HSA_TF32_CODE_OBJECTS"), ("HSA_INT8_CODE_OBJECTS", "RECIPE_HSA_INT8_CODE_OBJECTS"), ("HSA_INT4_CODE_OBJECTS", "RECIPE_HSA_INT4_CODE_OBJECTS"), ("HSA_INT1_CODE_OBJECTS", "RECIPE_HSA_INT1_CODE_OBJECTS"), ("HSA_F_CODE_OBJECTS", "RECIPE_HSA_F_CODE_OBJECTS")];
	let table = names.iter().zip(&embeds).map(|(name, values)| format!("static {}: &[(&str, &[u8])] = &[{}]\x3b", name.0, values.join(" "))).collect::<String>();
	fs::write(out.join("hsa-embed.rs"), table)?;
	for ((_, environment), values) in names.iter().zip(&objects) {
		println!("cargo:rustc-env={environment}={}", values.join("\x3b"))
	}
	println!("cargo:rustc-env=RECIPE_HSA_ASSEMBLIES={}", assemblies.join("\x3b"));
	println!("cargo:rustc-link-search=native={}", text(manifest, "hsa-library")?);
	Ok(())
}
fn compile_nvidia_matrix(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let cuda = env::var_os("CUDA_PATH").map(PathBuf::from);
	let gpu_architecture = text(manifest, "nvidia-minimum-architecture")?;
	let architecture = format!("-march={gpu_architecture}");
	let ptx = format!("+{}", text(manifest, "nvidia-ptx")?);
	let ir = parallel_ir(fs::read_to_string("amd-nv-cpu.ll")?, "declare i32 @recipe.workgroup.size.x()").replace("amdgcn-amd-amdhsa", "nvptx64-nvidia-cuda").replace("__ockl_steadyctr_u64", "llvm.nvvm.read.ptx.sreg.globaltimer").replace("llvm.amdgcn.workitem.id.x", "llvm.nvvm.read.ptx.sreg.tid.x").replace("llvm.amdgcn.workgroup.id.x", "llvm.nvvm.read.ptx.sreg.ctaid.x").replace("recipe.workgroup.size.x", "llvm.nvvm.read.ptx.sreg.ntid.x").replace("llvm.amdgcn.s.barrier", "llvm.nvvm.barrier0").replace("__ocml_exp_f64", "__nv_exp").replace("__ocml_tanh_f64", "__nv_tanh").replace("__ocml_cos_f64", "__nv_cos").replace("__ocml_sin_f64", "__nv_sin").replace("__ocml_log_f64", "__nv_log").replace("define protected amdgpu_kernel", "define ptx_kernel").replace("attributes #0 = { nounwind \"amdgpu-flat-work-group-size\"=\"1,1024\" }", "attributes #0 = { nounwind }");
	let sources = [("", native_ir(ir.clone(), "", "double", FloatFormat::FP64)?), ("-f32", native_ir(ir.clone(), "_f32", "float", FloatFormat::FP32)?), ("-f16", encoded_ir(ir.clone(), "_f16", FloatFormat::FP16.bytes(), &float_codec("half", "i16"), |value| FloatFormat::FP16.pack(value))?), ("-f8", encoded_ir(ir.clone(), "_f8", FloatFormat::FP8.bytes(), fp8_codec(), |value| FloatFormat::FP8.pack(value))?), ("-bf16", encoded_ir(ir.clone(), "_bf16", FloatFormat::BF16.bytes(), bf16_codec(), |value| FloatFormat::BF16.pack(value))?), ("-tf32", native_ir(ir.clone(), "_tf32", "float", FloatFormat::TF32)?), ("-int8", encoded_ir(ir.clone(), "_int8", IntFormat::INT8.bytes(), &int_codec(IntFormat::INT8), |value| IntFormat::INT8.pack(value))?), ("-int4", encoded_ir(ir.clone(), "_int4", IntFormat::INT4.bytes(), &int_codec(IntFormat::INT4), |value| IntFormat::INT4.pack(value))?), ("-int1", encoded_ir(ir.clone(), "_int1", IntFormat::INT1.bytes(), &int_codec(IntFormat::INT1), |value| IntFormat::INT1.pack(value))?), ("-f", custom_ir(ir, "_f")?)]
		.map(|(suffix, contents)| {
			let path = out.join(format!("recipe-nvidia{suffix}.ll"));
			fs::write(&path, contents).map(|_| (suffix, path))
		})
		.into_iter()
		.collect::<io::Result<Vec<_>>>()?;
	let compiler = text(manifest, "nvidia-compiler")?;
	let compiler = if Path::new(compiler).exists() { compiler } else { "clang" };
	let device = text(manifest, "nvidia-device-library")?;
	let device = cuda.map(|path| path.join("nvvm/libdevice/libdevice.10.bc")).filter(|path| path.exists()).unwrap_or_else(|| device.into());
	for ((suffix, source), environment) in sources.iter().zip(["RECIPE_NV_PTX", "RECIPE_NV_F32_PTX", "RECIPE_NV_F16_PTX", "RECIPE_NV_F8_PTX", "RECIPE_NV_BF16_PTX", "RECIPE_NV_TF32_PTX", "RECIPE_NV_INT8_PTX", "RECIPE_NV_INT4_PTX", "RECIPE_NV_INT1_PTX", "RECIPE_NV_F_PTX"]) {
		let output = out.join(format!("recipe{suffix}.ptx"));
		let mut command = Command::new(compiler);
		command.args(["-target", "nvptx64-nvidia-cuda", &architecture, "-Xclang", "-target-feature", "-Xclang", &ptx, "-O2", "-S", "-x", "ir", source.to_str().ok_or_else(|| io::Error::other("NVIDIA IR path is not UTF-8"))?, "-Xclang", "-mlink-builtin-bitcode", "-Xclang", device.to_str().ok_or_else(|| io::Error::other("NVIDIA device library path is not UTF-8"))?, "-o"]);
		command.arg(&output);
		run(&mut command, "NVIDIA LLVM IR compiler")?;
		println!("cargo:rustc-env={environment}={}", output.display());
	}
	println!("cargo:rustc-link-search=native={}", text(manifest, "nvidia-library")?);
	Ok(())
}
const CPU_REPLACEMENTS: &[(&str, &str)] = &[("@contraction_tile = external addrspace(3) global [0 x double], align 8", "@contraction_tile = internal global [65536 x double] zeroinitializer, align 8"), (" addrspace(3)", ""), ("call i32 @llvm.amdgcn.workitem.id.x()", "add i32 0, 0"), ("call i32 @recipe.local.id.x()", "add i32 0, 0"), ("call i32 @recipe.group.id.x()", "add i32 0, 0"), ("call i32 @recipe.workgroup.size.x()", "add i32 1, 0"), ("call void @llvm.amdgcn.s.barrier()", ""), ("call void @recipe.local.barrier()", ""), ("call i64 @__ockl_steadyctr_u64()", "add i64 0, 0"), ("declare i32 @llvm.amdgcn.workitem.id.x()", ""), ("declare void @llvm.amdgcn.s.barrier()", ""), ("declare i64 @__ockl_steadyctr_u64()", ""), ("__ocml_exp_f64", "exp"), ("__ocml_tanh_f64", "tanh"), ("__ocml_cos_f64", "cos"), ("__ocml_sin_f64", "sin"), ("__ocml_log_f64", "log"), ("define protected amdgpu_kernel void @forward_graph(", "define void @recipe_forward_cpu("), ("define protected amdgpu_kernel void @tape_epoch_graph(", "define void @recipe_epoch_cpu("), ("attributes #0 = { nounwind \"amdgpu-flat-work-group-size\"=\"1,1024\" }", "attributes #0 = { nounwind }")];
fn compile_cpu_matrix(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let target = env::var("TARGET")?;
	let mut ir = fs::read_to_string("amd-nv-cpu.ll")?.replace("amdgcn-amd-amdhsa", &target);
	for (pattern, replacement) in CPU_REPLACEMENTS {
		ir = ir.replace(pattern, replacement);
	}
	let clang = ["nvidia-compiler", "hsa-compiler"].iter().filter_map(|key| text(manifest, key).ok()).find(|path| Path::new(path).exists()).unwrap_or("clang");
	let named = |suffix: &str, contents: String| (format!("recipe-cpu{suffix}"), contents.replace("@recipe_forward_cpu(", &format!("@recipe_forward_cpu{suffix}(")).replace("@recipe_epoch_cpu(", &format!("@recipe_epoch_cpu{suffix}(")));
	let sources = [named("", native_ir(ir.clone(), "", "double", FloatFormat::FP64)?), named("_f32", native_ir(ir.clone(), "_f32", "float", FloatFormat::FP32)?), named("_f16", encoded_ir(ir.clone(), "_f16", FloatFormat::FP16.bytes(), &float_codec("half", "i16"), |value| FloatFormat::FP16.pack(value))?), named("_f8", encoded_ir(ir.clone(), "_f8", FloatFormat::FP8.bytes(), fp8_codec(), |value| FloatFormat::FP8.pack(value))?), named("_bf16", encoded_ir(ir.clone(), "_bf16", FloatFormat::BF16.bytes(), bf16_codec(), |value| FloatFormat::BF16.pack(value))?), named("_tf32", native_ir(ir.clone(), "_tf32", "float", FloatFormat::TF32)?), named("_int8", encoded_ir(ir.clone(), "_int8", IntFormat::INT8.bytes(), &int_codec(IntFormat::INT8), |value| IntFormat::INT8.pack(value))?), named("_int4", encoded_ir(ir.clone(), "_int4", IntFormat::INT4.bytes(), &int_codec(IntFormat::INT4), |value| IntFormat::INT4.pack(value))?), named("_int1", encoded_ir(ir.clone(), "_int1", IntFormat::INT1.bytes(), &int_codec(IntFormat::INT1), |value| IntFormat::INT1.pack(value))?), named("_f", custom_ir(ir, "_f")?)];
	let mut objects = Vec::new();
	for (name, contents) in sources {
		let source = out.join(format!("{name}.ll"));
		let object = out.join(format!("{name}.o"));
		fs::write(&source, contents)?;
		let mut compile = Command::new(clang);
		compile.args(["-target", &target, "-Xclang", "-opaque-pointers", "-x", "ir", "-O2", "-fPIC", "-c", "-o"]).arg(&object).arg(&source);
		if run(&mut compile, "CPU LLVM IR compiler").is_err() {
			let mut backend = Command::new("llc");
			backend.args(["--mtriple", &target, "-O2", "--relocation-model=pic", "-filetype=obj", "-o"]).arg(&object).arg(&source);
			run(&mut backend, "CPU LLVM IR backend")?;
		}
		objects.push(object);
	}
	let mut archive = Command::new("llvm-ar");
	archive.arg("rcs").arg(out.join("librecipe_cpu.a")).args(objects);
	run(&mut archive, "CPU archive")?;
	println!("cargo:rustc-link-search=native={}", out.display());
	println!("cargo:rustc-link-lib=static=recipe_cpu");
	if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") && env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("musl") {
		println!("cargo:rustc-link-lib=m");
	}
	Ok(())
}
fn compile_amd(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let ir = parallel_ir(fs::read_to_string("amd-nv-cpu.ll")?, AMD_WIDTH);
	let sources = [
		("", native_ir(ir.clone(), "", "double", FloatFormat::FP64)?),
		("-f32", native_ir(ir.clone(), "_f32", "float", FloatFormat::FP32)?),
		("-f16", encoded_ir(ir.clone(), "_f16", FloatFormat::FP16.bytes(), &float_codec("half", "i16"), |value| FloatFormat::FP16.pack(value))?),
		("-f8", encoded_ir(ir.clone(), "_f8", FloatFormat::FP8.bytes(), fp8_codec(), |value| FloatFormat::FP8.pack(value))?),
		("-bf16", encoded_ir(ir.clone(), "_bf16", FloatFormat::BF16.bytes(), bf16_codec(), |value| FloatFormat::BF16.pack(value))?),
		("-tf32", native_ir(ir.clone(), "_tf32", "float", FloatFormat::TF32)?),
		("-int8", encoded_ir(ir.clone(), "_int8", IntFormat::INT8.bytes(), &int_codec(IntFormat::INT8), |value| IntFormat::INT8.pack(value))?),
		("-int4", encoded_ir(ir.clone(), "_int4", IntFormat::INT4.bytes(), &int_codec(IntFormat::INT4), |value| IntFormat::INT4.pack(value))?),
		("-int1", encoded_ir(ir.clone(), "_int1", IntFormat::INT1.bytes(), &int_codec(IntFormat::INT1), |value| IntFormat::INT1.pack(value))?),
		("-f", custom_ir(ir, "_f")?),
	];
	let mut values = Vec::new();
	for (suffix, contents) in sources {
		let path = out.join(format!("recipe-amd{suffix}.ll"));
		fs::write(&path, contents)?;
		values.push(format!("{}={}", if suffix.is_empty() { "default" } else { suffix }, path.display()));
	}
	println!("cargo:rustc-env=RECIPE_AMD_IR={}", values.join("\x3b"));
	println!("cargo:rustc-env=RECIPE_HSA_COMPILER={}", text(manifest, "hsa-compiler")?);
	for (key, environment) in [("hsa-device-library", "RECIPE_HSA_DEVICE_LIBRARY"), ("hsa-clock-library", "RECIPE_HSA_CLOCK_LIBRARY"), ("hsa-finite-library", "RECIPE_HSA_FINITE_LIBRARY"), ("hsa-math-library", "RECIPE_HSA_MATH_LIBRARY"), ("hsa-device-library-directory", "RECIPE_HSA_DEVICE_LIBRARY_DIRECTORY"), ("hsa-library", "RECIPE_HSA_LIBRARY")] {
		println!("cargo:rustc-env={environment}={}", text(manifest, key)?);
	}
	Ok(())
}
fn compile_nvidia(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let ir = parallel_ir(fs::read_to_string("amd-nv-cpu.ll")?, "declare i32 @recipe.workgroup.size.x()").replace("amdgcn-amd-amdhsa", "nvptx64-nvidia-cuda").replace("__ockl_steadyctr_u64", "llvm.nvvm.read.ptx.sreg.globaltimer").replace("llvm.amdgcn.workitem.id.x", "llvm.nvvm.read.ptx.sreg.tid.x").replace("llvm.amdgcn.workgroup.id.x", "llvm.nvvm.read.ptx.sreg.ctaid.x").replace("recipe.workgroup.size.x", "llvm.nvvm.read.ptx.sreg.ntid.x").replace("llvm.amdgcn.s.barrier", "llvm.nvvm.barrier0").replace("__ocml_exp_f64", "__nv_exp").replace("__ocml_tanh_f64", "__nv_tanh").replace("__ocml_cos_f64", "__nv_cos").replace("__ocml_sin_f64", "__nv_sin").replace("__ocml_log_f64", "__nv_log").replace("define protected amdgpu_kernel", "define ptx_kernel").replace("attributes #0 = { nounwind \"amdgpu-flat-work-group-size\"=\"1,1024\" }", "attributes #0 = { nounwind }");
	let sources = [
		("", native_ir(ir.clone(), "", "double", FloatFormat::FP64)?),
		("-f32", native_ir(ir.clone(), "_f32", "float", FloatFormat::FP32)?),
		("-f16", encoded_ir(ir.clone(), "_f16", FloatFormat::FP16.bytes(), &float_codec("half", "i16"), |value| FloatFormat::FP16.pack(value))?),
		("-f8", encoded_ir(ir.clone(), "_f8", FloatFormat::FP8.bytes(), fp8_codec(), |value| FloatFormat::FP8.pack(value))?),
		("-bf16", encoded_ir(ir.clone(), "_bf16", FloatFormat::BF16.bytes(), bf16_codec(), |value| FloatFormat::BF16.pack(value))?),
		("-tf32", native_ir(ir.clone(), "_tf32", "float", FloatFormat::TF32)?),
		("-int8", encoded_ir(ir.clone(), "_int8", IntFormat::INT8.bytes(), &int_codec(IntFormat::INT8), |value| IntFormat::INT8.pack(value))?),
		("-int4", encoded_ir(ir.clone(), "_int4", IntFormat::INT4.bytes(), &int_codec(IntFormat::INT4), |value| IntFormat::INT4.pack(value))?),
		("-int1", encoded_ir(ir.clone(), "_int1", IntFormat::INT1.bytes(), &int_codec(IntFormat::INT1), |value| IntFormat::INT1.pack(value))?),
		("-f", custom_ir(ir, "_f")?),
	];
	let mut values = Vec::new();
	for (suffix, contents) in sources {
		let path = out.join(format!("recipe-nvidia{suffix}.ll"));
		fs::write(&path, contents)?;
		values.push(format!("{}={}", if suffix.is_empty() { "default" } else { suffix }, path.display()));
	}
	let cuda = env::var_os("CUDA_PATH").map(PathBuf::from);
	let device = cuda.as_ref().map(|path| path.join("nvvm/libdevice/libdevice.10.bc")).filter(|path| path.exists()).unwrap_or_else(|| PathBuf::from(text(manifest, "nvidia-device-library").unwrap()));
	println!("cargo:rustc-env=RECIPE_NV_IR={}", values.join("\x3b"));
	println!("cargo:rustc-env=RECIPE_NV_COMPILER={}", text(manifest, "nvidia-compiler")?);
	println!("cargo:rustc-env=RECIPE_NV_DEVICE_LIBRARY={}", device.display());
	println!("cargo:rustc-env=RECIPE_NV_PTX_VERSION=+{}", text(manifest, "nvidia-ptx")?);
	println!("cargo:rustc-env=RECIPE_NV_PTX_ASSEMBLER={}", cuda.map_or_else(|| "ptxas".to_owned(), |path| path.join("bin/ptxas").display().to_string()));
	println!("cargo:rustc-env=RECIPE_NV_LIBRARY={}", text(manifest, "nvidia-library")?);
	Ok(())
}
fn compile_cpu(manifest: &str, out: &PathBuf) -> BuildResult<()> {
	let target = env::var("TARGET")?;
	let mut ir = fs::read_to_string("amd-nv-cpu.ll")?.replace("amdgcn-amd-amdhsa", &target);
	for (pattern, replacement) in CPU_REPLACEMENTS {
		ir = ir.replace(pattern, replacement);
	}
	let clang = ["nvidia-compiler", "hsa-compiler"].iter().filter_map(|key| text(manifest, key).ok()).find(|path| Path::new(path).exists()).unwrap_or("clang");
	let sources = [
		("", native_ir(ir.clone(), "", "double", FloatFormat::FP64)?),
		("-f32", native_ir(ir.clone(), "_f32", "float", FloatFormat::FP32)?),
		("-f16", encoded_ir(ir.clone(), "_f16", FloatFormat::FP16.bytes(), &float_codec("half", "i16"), |value| FloatFormat::FP16.pack(value))?),
		("-f8", encoded_ir(ir.clone(), "_f8", FloatFormat::FP8.bytes(), fp8_codec(), |value| FloatFormat::FP8.pack(value))?),
		("-bf16", encoded_ir(ir.clone(), "_bf16", FloatFormat::BF16.bytes(), bf16_codec(), |value| FloatFormat::BF16.pack(value))?),
		("-tf32", native_ir(ir.clone(), "_tf32", "float", FloatFormat::TF32)?),
		("-int8", encoded_ir(ir.clone(), "_int8", IntFormat::INT8.bytes(), &int_codec(IntFormat::INT8), |value| IntFormat::INT8.pack(value))?),
		("-int4", encoded_ir(ir.clone(), "_int4", IntFormat::INT4.bytes(), &int_codec(IntFormat::INT4), |value| IntFormat::INT4.pack(value))?),
		("-int1", encoded_ir(ir.clone(), "_int1", IntFormat::INT1.bytes(), &int_codec(IntFormat::INT1), |value| IntFormat::INT1.pack(value))?),
		("-f", custom_ir(ir, "_f")?),
	];
	let mut values = Vec::new();
	for (suffix, contents) in sources {
		let contents = contents.replace(" addrspace(1)", "");
		let path = out.join(format!("recipe-cpu{suffix}.ll"));
		fs::write(&path, contents)?;
		values.push(format!("{}={}", if suffix.is_empty() { "default" } else { suffix }, path.display()));
	}
	println!("cargo:rustc-env=RECIPE_CPU_IR={}", values.join("\x3b"));
	println!("cargo:rustc-env=RECIPE_CPU_COMPILER={clang}");
	println!("cargo:rustc-env=RECIPE_CPU_TARGET={target}");
	Ok(())
}
fn main() -> BuildResult<()> {
	let manifest = fs::read_to_string("Cargo.toml")?;
	for (key, environment) in [("epochs", "RECIPE_TRAIN_EPOCHS"), ("learning-rate", "RECIPE_TRAIN_LEARNING_RATE"), ("initial-weight", "RECIPE_TRAIN_INITIAL_WEIGHT"), ("adamw-beta1", "RECIPE_ADAMW_BETA1"), ("adamw-beta2", "RECIPE_ADAMW_BETA2"), ("adamw-epsilon", "RECIPE_ADAMW_EPSILON"), ("adamw-weight-decay", "RECIPE_ADAMW_WEIGHT_DECAY"), ("kmeans-iterations", "RECIPE_KMEANS_ITERATIONS"), ("svm-iterations", "RECIPE_SVM_ITERATIONS"), ("svm-learning-rate", "RECIPE_SVM_LEARNING_RATE"), ("svm-regularization", "RECIPE_SVM_REGULARIZATION"), ("svm-epsilon", "RECIPE_SVM_EPSILON"), ("tree-depth", "RECIPE_TREE_DEPTH"), ("tree-min-rows", "RECIPE_TREE_MIN_ROWS"), ("forest-feature-fraction", "RECIPE_FOREST_FEATURE_FRACTION"), ("bayes-prior-precision", "RECIPE_BAYES_PRIOR_PRECISION"), ("bayes-noise-variance", "RECIPE_BAYES_NOISE_VARIANCE"), ("bayes-variance-epsilon", "RECIPE_BAYES_VARIANCE_EPSILON"), ("boost-iterations", "RECIPE_BOOST_ITERATIONS"), ("boost-learning-rate", "RECIPE_BOOST_LEARNING_RATE"), ("catboost-ordered-prior", "RECIPE_CATBOOST_ORDERED_PRIOR"), ("xgboost-l2-regularization", "RECIPE_XGBOOST_L2_REGULARIZATION"), ("xgboost-minimum-gain", "RECIPE_XGBOOST_MINIMUM_GAIN"), ("lightgbm-histogram-bins", "RECIPE_LIGHTGBM_HISTOGRAM_BINS"), ("lightgbm-leaves", "RECIPE_LIGHTGBM_LEAVES"), ("quantization-block-weights", "RECIPE_QUANTIZATION_BLOCK_WEIGHTS"), ("surrogate-epochs", "RECIPE_SURROGATE_EPOCHS"), ("surrogate-rate", "RECIPE_SURROGATE_RATE"), ("surrogate-width", "RECIPE_SURROGATE_WIDTH"), ("rat-batch-rows", "RECIPE_RAT_BATCH_ROWS"), ("topology-latency-bytes", "RECIPE_TOPOLOGY_LATENCY_BYTES"), ("topology-bandwidth-bytes", "RECIPE_TOPOLOGY_BANDWIDTH_BYTES"), ("topology-probe-repetitions", "RECIPE_TOPOLOGY_REPETITIONS"), ("tile-vram-unit-bytes", "RECIPE_TILE_VRAM_UNIT_BYTES"), ("ssh-connect-timeout-seconds", "RECIPE_SSH_CONNECT_TIMEOUT"), ("random-seed", "RECIPE_RANDOM_SEED"), ("progress-refresh-hz", "RECIPE_PROGRESS_REFRESH_HZ"), ("normalization-epsilon", "RECIPE_NORMALIZATION_EPSILON"), ("categorical-ratio", "RECIPE_CATEGORICAL_RATIO"), ("leak-slope", "RECIPE_LEAK_SLOPE"), ("prelu-slope", "RECIPE_PRELU_SLOPE"), ("elu-alpha", "RECIPE_ELU_ALPHA"), ("selu-alpha", "RECIPE_SELU_ALPHA"), ("selu-scale", "RECIPE_SELU_SCALE"), ("gelu-scale", "RECIPE_GELU_SCALE"), ("gelu-cubic", "RECIPE_GELU_CUBIC"), ("huber-threshold", "RECIPE_HUBER_THRESHOLD"), ("output-tolerance", "RECIPE_OUTPUT_TOLERANCE"), ("gradient-tolerance", "RECIPE_GRADIENT_TOLERANCE"), ("backend-tolerance", "RECIPE_BACKEND_TOLERANCE")] {
		println!("cargo:rustc-env={environment}={}", number(&manifest, key)?);
	}
	for (key, environment) in [("hsa-runtime", "RECIPE_HSA_RUNTIME"), ("nvidia-runtime", "RECIPE_NV_RUNTIME"), ("ssh-config", "RECIPE_SSH_CONFIG")] {
		println!("cargo:rustc-env={environment}={}", text(&manifest, key)?);
	}
	println!("cargo:rustc-env=RECIPE_MULTI_DEVICE={}", flag(&manifest, "multi-device")?);
	let out = PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR must be configured"))?);
	println!("cargo::rustc-check-cfg=cfg(amd)");
	println!("cargo::rustc-check-cfg=cfg(nvidia)");
	let toolchain = |compiler: &str, library: &str| -> BuildResult<bool> { Ok(Path::new(text(&manifest, compiler)?).exists() && Path::new(text(&manifest, library)?).exists()) };
	compile_cpu(&manifest, &out)?;
	// GPU driver stubs and library search paths are host-arch: cross-compiled builds are CPU-only.
	let native = env::var("TARGET")? == env::var("HOST")?;
	let amd = native && toolchain("hsa-compiler", "hsa-device-library")?;
	let nvidia = native && (toolchain("nvidia-compiler", "nvidia-device-library")? || env::var_os("CUDA_PATH").is_some());
	if amd {
		println!("cargo:rustc-cfg=amd");
		compile_amd(&manifest, &out)?;
	}
	if nvidia {
		println!("cargo:rustc-cfg=nvidia");
		compile_nvidia(&manifest, &out)?;
	}
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=amd-nv-cpu.ll");
	Ok(())
}
