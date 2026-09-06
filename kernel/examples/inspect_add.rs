use std::{env, fs};

use recipe_core::{AliasPermission, AliasRule, DType, ElementCount, IndexSpace, KernelInput, KernelInputId, KernelOutput, KernelOutputId, KernelTemplate, KernelTemplateId, ScalarInput, ScalarInstruction, ScalarOpcode, ScalarProgram, ScalarValueId, StaticBufferAccess};
use recipe_kernel::{AmdTarget, KernelTarget, LoweringOptions, NvidiaTarget, inspect_cubin, inspect_hsaco, lower_elementwise};
use write::{block, device};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut arguments = env::args_os().skip(1);
	let target = arguments
		.next()
		.ok_or("usage: inspect_add <gfx1101|sm_86> <artifact>")?;
	let artifact = arguments
		.next()
		.ok_or("usage: inspect_add <gfx1101|sm_86> <artifact>")?;
	if arguments.next().is_some() {
		return Err("usage: inspect_add <gfx1101|sm_86> <artifact>".into());
	}
	let left = ScalarValueId::new(1);
	let right = ScalarValueId::new(2);
	let result = ScalarValueId::new(3);
	let index_space = IndexSpace::new(vec![ElementCount::new(1_048_576)?])?;
	let access = StaticBufferAccess::contiguous(&index_space, DType::F32)?;
	let template = KernelTemplate {
		id: KernelTemplateId::new(1),
		index_space,
		inputs: vec![
			KernelInput {
				id: KernelInputId::new(1),
				dtype: DType::F32,
				access: access.clone(),
			},
			KernelInput {
				id: KernelInputId::new(2),
				dtype: DType::F32,
				access: access.clone(),
			},
		],
		outputs: vec![KernelOutput {
			id: KernelOutputId::new(1),
			dtype: DType::F32,
			access,
		}],
		program: ScalarProgram {
			inputs: vec![
				ScalarInput {
					id: left,
					dtype: DType::F32,
				},
				ScalarInput {
					id: right,
					dtype: DType::F32,
				},
			],
			constants: Vec::new(),
			instructions: vec![ScalarInstruction {
				result,
				dtype: DType::F32,
				opcode: ScalarOpcode::Add,
				operands: vec![left, right],
			}],
			outputs: vec![result],
		},
		alias_rules: vec![
			AliasRule {
				input: KernelInputId::new(1),
				output: KernelOutputId::new(1),
				permission: AliasPermission::MayAliasExact,
			},
			AliasRule {
				input: KernelInputId::new(2),
				output: KernelOutputId::new(1),
				permission: AliasPermission::Forbidden,
			},
		],
	};
	let target_text = target.to_string_lossy();
	let target = match target_text.as_ref() {
		"gfx1101" => {
			KernelTarget::Amd(AmdTarget {
				target_id: "gfx1101".to_owned(),
				code_object_version: 6,
			})
		}
		"sm_86" => {
			KernelTarget::Nvidia(NvidiaTarget {
				sm_major: 8,
				sm_minor: 6,
				ptx_isa: 75,
			})
		}
		_ => return Err("example accepts gfx1101 or sm_86".into()),
	};
	let lowered = lower_elementwise(&template, &target, &LoweringOptions {
		entry_symbol: "recipe_add_f32".to_owned(),
		workgroup_lanes: 256,
	})?;
	let artifact = fs::read(artifact)?;
	match target {
		KernelTarget::Amd(target) => {
			let inspected = inspect_hsaco(
				&artifact,
				&target.target_id,
				target.code_object_version,
				&lowered.abi,
			)?;
			block(device, &format!("{inspected:#?}"));
		}
		KernelTarget::Nvidia(target) => {
			let inspected = inspect_cubin(
				&artifact,
				target.sm_major * 10 + target.sm_minor,
				&lowered.abi.entry_symbol,
			)?;
			block(device, &format!("{inspected:#?}"));
		}
	}
	Ok(())
}
