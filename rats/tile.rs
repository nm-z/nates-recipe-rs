use super::*;

#[cfg(feature = "amd")]
const RDNA3_VGPRS_PER_SIMD: u32 = 1536;
#[cfg(feature = "amd")]
const RDNA3_VGPR_GRANULE: u32 = 24;
const DOUBLE_BUFFER_VALUES: u32 = 2;

#[derive(Clone, Copy)]
pub(super) struct Resources {
	pub registers: u32, 	pub shared: u32, 	pub max_block: u32,
}
#[derive(Clone, Copy)]
pub(super) struct Geometry {
	pub groups: u32, 	pub block: u32,
}
impl Geometry {
	pub fn threads(self) -> Result<u32> {
		self.groups.checked_mul(self.block).filter(|value| *value != 0)
			.ok_or_else(|| RecipeError::new("GPU launch size overflows"))
	}
}
fn geometry(
	cus: u32, wave: u32, workgroup: u32, lds: u32, groups_per_cu: u32, resources: Resources,
) -> Result<Geometry> {
	require(wave != 0 && wave <= workgroup && wave <= resources.max_block, "GPU wave exceeds kernel workgroup")?;
	let waves = groups_per_cu.min(workgroup / wave).min(resources.max_block / wave);
	require(waves != 0, "GPU has no resident wave")?;
	let block = waves.checked_mul(wave).ok_or_else(|| RecipeError::new("GPU workgroup size overflows"))?;
	let tile = block.checked_mul(DOUBLE_BUFFER_VALUES * size_of::<f64>() as u32)
		.ok_or_else(|| RecipeError::new("GPU tile size overflows"))?;
	require(resources.shared.max(tile) <= lds, "GPU tile exceeds local memory")?;
	Ok(Geometry { groups: cus, block })
}
#[cfg(feature = "amd")]
pub(super) fn amd(
	cus: u32, wave: u32, workgroup: u32, lds: u32, waves_per_cu: u32, simds_per_cu: u32, resources: Resources,
) -> Result<Geometry> {
	let registers = resources.registers.div_ceil(RDNA3_VGPR_GRANULE) * RDNA3_VGPR_GRANULE;
	require(registers != 0, "AMD kernel register count is absent")?;
	let register_waves = RDNA3_VGPRS_PER_SIMD / registers * simds_per_cu;
	geometry(cus, wave, workgroup, lds, waves_per_cu.min(register_waves), resources)
}
#[cfg(feature = "nvidia")]
pub(super) fn nvidia(
	cus: u32, wave: u32, workgroup: u32, block_lds: u32, sm_lds: u32, waves_per_cu: u32, resources: Resources,
) -> Result<Geometry> {
	require(resources.registers != 0, "Nvidia kernel register count is absent")?;
	let tile = wave.checked_mul(DOUBLE_BUFFER_VALUES * size_of::<f64>() as u32)
		.ok_or_else(|| RecipeError::new("Nvidia tile size overflows"))?;
	require(resources.shared.max(tile) <= block_lds, "Nvidia tile exceeds workgroup shared memory")?;
	geometry(cus, wave, workgroup, sm_lds, waves_per_cu, resources)
}
