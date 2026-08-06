use super::*;
#[cfg(feature = "amd")]
const RDNA3: (u32, u32, u32) = (110001, 1536, 24);
const DOUBLE_BUFFER_VALUES: u32 = 2;
#[derive(Clone, Copy)]
pub(super) struct Resources {
	pub registers: u32, pub shared: u32, pub max_block: u32,
}
#[derive(Clone, Copy)]
pub(super) struct Geometry {
	pub groups: u32, pub block: u32,
}
impl Geometry {
	pub fn threads(self) -> Result<u32> {
		self.groups.checked_mul(self.block).filter(|n| *n != 0).ok_or_else(|| RecipeError::new("GPU launch size overflows"))
	}
}
#[cfg(feature = "amd")]
fn property(text: &str, name: &str) -> Result<u32> {
	text.lines().find_map(|line| line.split_once(' ').filter(|v| v.0 == name))
		.ok_or_else(|| RecipeError::new(format!("KFD property {name:?} is absent")))?.1.parse()
		.map_err(|error| RecipeError::new(format!("KFD property {name:?} is invalid: {error}")))
}
fn geometry(cus: u32, wave: u32, workgroup: u32, lds: u32, groups: u32, r: Resources) -> Result<Geometry> {
	require(wave != 0 && wave <= workgroup && wave <= r.max_block, "GPU wave exceeds the kernel workgroup")?;
	let waves = groups.min(workgroup / wave).min(r.max_block / wave);
	require(waves != 0, "GPU has no resident wave")?;
	let block = waves.checked_mul(wave).ok_or_else(|| RecipeError::new("GPU workgroup size overflows"))?;
	let tile = block.checked_mul(DOUBLE_BUFFER_VALUES * size_of::<f64>() as u32)
		.ok_or_else(|| RecipeError::new("GPU tile size overflows"))?;
	require(r.shared.max(tile) != 0 && r.shared.max(tile) <= lds, "GPU tile exceeds local memory")?;
	Ok(Geometry { groups: cus, block })
}
#[cfg(feature = "amd")]
pub(super) fn amd(cus: u32, wave: u32, workgroup: u32, groups: u32, simds: u32, node: u32, r: Resources) -> Result<Geometry> {
	let path = format!("/sys/class/kfd/kfd/topology/nodes/{node}/properties");
	let text = fs::read_to_string(&path).map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
	let gfx = property(&text, "gfx_target_version")?;
	require(gfx == RDNA3.0, format!("GPU target {gfx} does not match the compiled gfx1101 kernel"))?;
	let lds = property(&text, "lds_size_in_kb")?.checked_mul(1024).ok_or_else(|| RecipeError::new("AMD LDS size overflows"))?;
	let registers = r.registers.div_ceil(RDNA3.2) * RDNA3.2;
	require(registers != 0, "AMD kernel register count is absent")?;
	geometry(cus, wave, workgroup, lds, groups.min(RDNA3.1 / registers * simds), r)
}
#[cfg(feature = "nvidia")]
pub(super) fn nvidia(cus: u32, wave: u32, workgroup: u32, block_lds: u32, sm_lds: u32, groups: u32, r: Resources) -> Result<Geometry> {
	require(r.registers != 0, "Nvidia kernel register count is absent")?;
	let tile = wave.checked_mul(DOUBLE_BUFFER_VALUES * size_of::<f64>() as u32).ok_or_else(|| RecipeError::new("Nvidia tile size overflows"))?;
	require(r.shared.max(tile) <= block_lds, "Nvidia tile exceeds workgroup shared memory")?;
	geometry(cus, wave, workgroup, sm_lds, groups, r) }
