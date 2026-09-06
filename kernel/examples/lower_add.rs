use std::{env, fs, path::PathBuf};

use recipe_core::{AliasPermission, AliasRule, DType, ElementCount, IndexSpace, KernelInput, KernelInputId, KernelOutput, KernelOutputId, KernelTemplate, KernelTemplateId, ScalarInput, ScalarInstruction, ScalarOpcode, ScalarProgram, ScalarValueId, StaticBufferAccess};
use recipe_kernel::{AmdTarget, KernelTarget, LoweringOptions, NvidiaTarget, lower_elementwise};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut arguments = env::args_os().skip(1);
	let target = arguments
		.next()
		.ok_or("usage: lower_add <gfx1101|sm_86> <output.ll>")?;
	let output = PathBuf::from(
		arguments
			.next()
			.ok_or("usage: lower_add <gfx1101|sm_86> <output.ll>")?,
	);
	if arguments.next().is_some() {
		return Err("usage: lower_add <gfx1101|sm_86> <output.ll>".into());
	}
	let target = target.to_string_lossy();
	let target = if let Some(architecture) = target.strip_prefix("gfx") {
		if architecture.is_empty() {
			return Err("AMD target must include its numeric target ID".into());
		}
		KernelTarget::Amd(AmdTarget {
			target_id: format!("gfx{architecture}"),
			code_object_version: 6,
		})
	} else if let Some(architecture) = target.strip_prefix("sm_") {
		let digits = architecture.as_bytes();
		if digits.len() != 2 || !digits.iter().all(u8::is_ascii_digit) {
			return Err("NVIDIA target must have the form sm_86".into());
		}
		KernelTarget::Nvidia(NvidiaTarget {
			sm_major: digits[0] - b'0',
			sm_minor: digits[1] - b'0',
			ptx_isa: 75,
		})
	} else {
		return Err("target must have the form gfx1101 or sm_86".into());
	};
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
	let lowered = lower_elementwise(&template, &target, &LoweringOptions {
		entry_symbol: "recipe_add_f32".to_owned(),
		workgroup_lanes: 256,
	})?;
	fs::write(output, lowered.llvm_ir)?;
	Ok(())
}
