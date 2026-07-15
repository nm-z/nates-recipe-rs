	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	momentum_update_kernel  ; -- Begin function momentum_update_kernel
	.globl	momentum_update_kernel
	.p2align	8
	.type	momentum_update_kernel,@function
momentum_update_kernel:                 ; @momentum_update_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b32 s4, s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB0_2
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s9, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_u32 v0, vcc_lo, s4, v0
	global_load_b64 v[6:7], v[4:5], off
	s_load_b64 s[2:3], s[10:11], 0x0
	s_load_b64 s[0:1], s[0:1], 0x20
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[0:1], 0x0
	s_waitcnt vmcnt(1)
	v_mul_f64 v[2:3], s[2:3], v[2:3]
	s_waitcnt vmcnt(0) lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[2:3], s[0:1], v[6:7], -v[2:3]
	global_store_b64 v[4:5], v[2:3], off
	global_load_b64 v[4:5], v[0:1], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel momentum_update_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 304
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 8
		.amdhsa_next_free_sgpr 12
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 2
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end0:
	.size	momentum_update_kernel, .Lfunc_end0-momentum_update_kernel
                                        ; -- End function
	.set momentum_update_kernel.num_vgpr, 8
	.set momentum_update_kernel.num_agpr, 0
	.set momentum_update_kernel.numbered_sgpr, 12
	.set momentum_update_kernel.num_named_barrier, 0
	.set momentum_update_kernel.private_seg_size, 0
	.set momentum_update_kernel.uses_vcc, 1
	.set momentum_update_kernel.uses_flat_scratch, 0
	.set momentum_update_kernel.has_dyn_sized_stack, 0
	.set momentum_update_kernel.has_recursion, 0
	.set momentum_update_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 252
; TotalNumSgprs: 14
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 8
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	rmsprop_update_kernel   ; -- Begin function rmsprop_update_kernel
	.globl	rmsprop_update_kernel
	.p2align	8
	.type	rmsprop_update_kernel,@function
rmsprop_update_kernel:                  ; @rmsprop_update_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b32 s4, s[0:1], 0x30
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_2
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b128 s[0:3], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s9, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_load_b64 s[0:1], s[0:1], 0x0
	global_load_b64 v[6:7], v[4:5], off
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[8:9], -s[0:1], 1.0
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[8:9], v[2:3]
	v_mul_f64 v[8:9], v[2:3], v[8:9]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], s[0:1], v[6:7], v[8:9]
	s_load_b64 s[0:1], s[10:11], 0x0
	s_load_b64 s[2:3], s[2:3], 0x0
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[2:3], s[0:1], v[2:3]
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[6:7]
	global_store_b64 v[4:5], v[6:7], off
	v_cndmask_b32_e64 v8, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[6:7], v8
	v_rsq_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[12:13], v[8:9], v[10:11]
	v_mul_f64 v[10:11], v[10:11], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 0.5
	v_fma_f64 v[12:13], v[12:13], v[14:15], v[12:13]
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[12:13], v[12:13], v[8:9]
	v_fma_f64 v[12:13], v[14:15], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[12:13], v[12:13], v[8:9]
	v_fma_f64 v[10:11], v[14:15], v[10:11], v[12:13]
	v_cndmask_b32_e64 v12, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[8:9], 0x260
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[10:11], v[10:11], v12
	v_dual_cndmask_b32 v9, v11, v9 :: v_dual_cndmask_b32 v8, v10, v8
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	v_add_f64 v[8:9], s[2:3], v[8:9]
	global_load_b64 v[4:5], v[0:1], off
	v_div_scale_f64 v[10:11], null, v[8:9], v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[12:13], v[10:11]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[6:7], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[10:11], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[12:13], v[6:7]
	v_div_scale_f64 v[12:13], vcc_lo, v[2:3], v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[12:13], v[6:7]
	v_fma_f64 v[10:11], -v[10:11], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[6:7], v[10:11], v[6:7], v[14:15]
	v_div_fixup_f64 v[2:3], v[6:7], v[8:9], v[2:3]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[4:5], -v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel rmsprop_update_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 312
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 16
		.amdhsa_next_free_sgpr 12
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 5
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end1:
	.size	rmsprop_update_kernel, .Lfunc_end1-rmsprop_update_kernel
                                        ; -- End function
	.set rmsprop_update_kernel.num_vgpr, 16
	.set rmsprop_update_kernel.num_agpr, 0
	.set rmsprop_update_kernel.numbered_sgpr, 12
	.set rmsprop_update_kernel.num_named_barrier, 0
	.set rmsprop_update_kernel.private_seg_size, 0
	.set rmsprop_update_kernel.uses_vcc, 1
	.set rmsprop_update_kernel.uses_flat_scratch, 0
	.set rmsprop_update_kernel.has_dyn_sized_stack, 0
	.set rmsprop_update_kernel.has_recursion, 0
	.set rmsprop_update_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 580
; TotalNumSgprs: 14
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 16
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	adagrad_update_kernel   ; -- Begin function adagrad_update_kernel
	.globl	adagrad_update_kernel
	.p2align	8
	.type	adagrad_update_kernel,@function
adagrad_update_kernel:                  ; @adagrad_update_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b32 s4, s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s9, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[6:7], v[4:5], off
	s_load_b64 s[0:1], s[0:1], 0x20
	s_load_b64 s[2:3], s[10:11], 0x0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[0:1], 0x0
	s_waitcnt vmcnt(0)
	v_fma_f64 v[6:7], v[2:3], v[2:3], v[6:7]
	v_mul_f64 v[2:3], s[2:3], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[6:7]
	global_store_b64 v[4:5], v[6:7], off
	v_cndmask_b32_e64 v8, 0, 0x100, vcc_lo
	v_ldexp_f64 v[8:9], v[6:7], v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[12:13], v[8:9], v[10:11]
	v_mul_f64 v[10:11], v[10:11], 0.5
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[12:13], v[14:15], v[12:13]
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[10:11]
	v_fma_f64 v[14:15], -v[12:13], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[10:11], v[12:13]
	v_fma_f64 v[14:15], -v[12:13], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[14:15], v[10:11], v[12:13]
	v_cndmask_b32_e64 v12, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[8:9], 0x260
	v_ldexp_f64 v[10:11], v[10:11], v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v9, v11, v9 :: v_dual_cndmask_b32 v8, v10, v8
	v_add_co_u32 v0, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], s[0:1], v[8:9]
	global_load_b64 v[4:5], v[0:1], off
	v_div_scale_f64 v[10:11], null, v[8:9], v[8:9], v[2:3]
	v_rcp_f64_e32 v[12:13], v[10:11]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[6:7], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[12:13]
	v_fma_f64 v[12:13], -v[10:11], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[12:13], v[6:7]
	v_div_scale_f64 v[12:13], vcc_lo, v[2:3], v[8:9], v[2:3]
	v_mul_f64 v[14:15], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[10:11], v[14:15], v[12:13]
	v_div_fmas_f64 v[6:7], v[10:11], v[6:7], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[6:7], v[8:9], v[2:3]
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[4:5], -v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel adagrad_update_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 304
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 16
		.amdhsa_next_free_sgpr 12
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 5
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end2:
	.size	adagrad_update_kernel, .Lfunc_end2-adagrad_update_kernel
                                        ; -- End function
	.set adagrad_update_kernel.num_vgpr, 16
	.set adagrad_update_kernel.num_agpr, 0
	.set adagrad_update_kernel.numbered_sgpr, 12
	.set adagrad_update_kernel.num_named_barrier, 0
	.set adagrad_update_kernel.private_seg_size, 0
	.set adagrad_update_kernel.uses_vcc, 1
	.set adagrad_update_kernel.uses_flat_scratch, 0
	.set adagrad_update_kernel.has_dyn_sized_stack, 0
	.set adagrad_update_kernel.has_recursion, 0
	.set adagrad_update_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 536
; TotalNumSgprs: 14
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 16
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	lamb_phase1_kernel      ; -- Begin function lamb_phase1_kernel
	.globl	lamb_phase1_kernel
	.p2align	8
	.type	lamb_phase1_kernel,@function
lamb_phase1_kernel:                     ; @lamb_phase1_kernel
; %bb.0:
	s_clause 0x3
	s_load_b32 s3, s[0:1], 0x6c
	s_load_b64 s[26:27], s[0:1], 0x40
	s_load_b64 s[24:25], s[0:1], 0x58
	s_load_b128 s[20:23], s[0:1], 0x48
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_mov_b32 s3, exec_lo
	v_cmpx_gt_i32_e64 s27, v1
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_load_b512 s[4:19], s[0:1], 0x0
	v_cvt_f64_i32_e32 v[2:3], s26
	s_mov_b32 s28, 0x55555555
	s_mov_b32 s29, 0x3fe55555
	s_mov_b32 s30, 0x968915a9
	s_mov_b32 s34, 0x4222de17
	s_mov_b32 s31, 0x3fba6564
	s_mov_b32 s35, 0x3fbdee67
	s_mov_b32 s36, 0x3abe935a
	s_mov_b32 s37, 0x3fbe25e4
	s_mov_b32 s38, 0x47e6c9c2
	s_mov_b32 s39, 0x3fc110ef
	s_mov_b32 s40, 0xcfa74449
	s_mov_b32 s41, 0x3fc3b13b
	s_mov_b32 s42, 0x71bf3c30
	s_mov_b32 s43, 0x3fc745d1
	s_mov_b32 s44, 0x1c7792ce
	s_mov_b32 s45, 0x3fcc71c7
	s_mov_b32 s46, 0x924920da
	s_mov_b32 s47, 0x3fd24924
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[12:13], s[12:13], 0x0
	s_mov_b32 s48, 0x9999999c
	s_mov_b32 s49, 0x3fd99999
	s_mov_b32 s51, 0xbfe55555
	s_mov_b32 s50, s28
	s_mov_b32 s52, 0xd5df274d
	s_mov_b32 s53, 0x3c8543b0
	s_mov_b32 s54, 0xfefa39ef
	s_mov_b32 s55, 0x3fe62e42
	s_mov_b32 s58, 0x3b39803f
	s_mov_b32 s59, 0x3c7abc9e
	s_mov_b32 s56, 0x652b82fe
	s_mov_b32 s57, 0x3ff71547
	s_mov_b32 s61, 0xbfe62e42
	s_mov_b32 s60, s54
	s_mov_b32 s63, 0xbc7abc9e
	s_mov_b32 s62, s58
	s_mov_b32 s64, 0xfca7ab0c
	s_mov_b32 s66, 0x6a5dcb37
	s_mov_b32 s65, 0x3e928af3
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f64_e64 vcc_lo, s[12:13], 1.0
	s_mov_b32 s67, 0x3e5ade15
	s_mov_b32 s68, 0x623fde64
	s_mov_b32 s69, 0x3ec71dee
	s_mov_b32 s70, 0x7c89e6b0
	s_mov_b32 s71, 0x3efa0199
	s_mov_b32 s72, 0x14761f6e
	s_mov_b32 s73, 0x3f2a01a0
	s_mov_b32 s74, 0x1852b7b0
	s_mov_b32 s75, 0x3f56c16c
	s_mov_b32 s76, 0x11122322
	s_mov_b32 s77, 0x3f811111
	s_mov_b32 s78, 0x555502a1
	s_mov_b32 s79, 0x3fa55555
	s_mov_b32 s80, 0x55555511
	s_mov_b32 s81, 0x3fc55555
	s_mov_b32 s82, 11
	s_mov_b32 s83, 0x3fe00000
	s_load_b64 s[14:15], s[14:15], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f64_e64 s2, s[14:15], 1.0
	v_cndmask_b32_e32 v5, 0x3ff00000, v3, vcc_lo
	v_cndmask_b32_e32 v4, 0, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[4:5]
	s_and_b32 s0, vcc_lo, exec_lo
	s_cselect_b32 s27, s13, 0x3ff00000
	s_cselect_b32 s26, s12, 0
	v_frexp_mant_f64_e64 v[6:7], |s[26:27]|
	v_cmp_class_f64_e64 s33, s[26:27], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[28:29], v[6:7]
	v_cndmask_b32_e64 v8, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_rcp_f64_e32 v[10:11], v[8:9]
	v_add_f64 v[16:17], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[12:13], v[8:9], -v[18:19]
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[18:19], v[6:7]
	v_add_f64 v[16:17], v[14:15], -v[8:9]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[16:17], v[6:7]
	v_mul_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_add_f64 v[10:11], v[8:9], -v[12:13]
	v_mul_f64 v[12:13], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	v_fma_f64 v[10:11], v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[6:7], v[6:7]
	v_fma_f64 v[10:11], v[8:9], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_fma_f64 v[16:17], v[14:15], s[34:35], s[30:31]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_mul_f64 v[22:23], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[36:37]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[38:39]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[42:43]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[44:45]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[46:47]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[48:49]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[14:15], v[16:17]
	v_fma_f64 v[12:13], v[14:15], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[16:17], v[12:13]
	v_add_f64 v[16:17], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[16:17], s[28:29]
	v_add_f64 v[18:19], v[16:17], -v[18:19]
	v_add_f64 v[24:25], v[20:21], s[50:51]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_fma_f64 v[18:19], v[14:15], v[8:9], -v[22:23]
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], s[52:53]
	v_fma_f64 v[14:15], v[14:15], v[6:7], v[18:19]
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], v[16:17]
	v_fma_f64 v[10:11], v[10:11], v[8:9], v[14:15]
	v_ldexp_f64 v[8:9], v[8:9], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[20:21], v[12:13]
	v_add_f64 v[16:17], v[22:23], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[20:21], -v[14:15]
	v_mul_f64 v[20:21], v[16:17], v[14:15]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], v[18:19]
	v_fma_f64 v[18:19], v[16:17], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[12:13], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[12:13]
	v_frexp_exp_i32_f64_e32 v14, s[26:27]
	v_add_f64 v[12:13], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v14, null, 0, v14, vcc_lo
	v_cvt_f64_i32_e32 v[14:15], v14
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[8:9], v[12:13]
	v_add_f64 v[18:19], v[12:13], -v[20:21]
	v_mul_f64 v[20:21], v[14:15], s[54:55]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[14:15], s[54:55], -v[20:21]
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_fma_f64 v[10:11], v[14:15], s[58:59], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[16:17], v[6:7]
	v_add_f64 v[20:21], v[8:9], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[8:9], v[12:13]
	v_add_f64 v[16:17], v[12:13], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[14:15], -v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[14:15], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_add_f64 v[16:17], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[16:17], -v[10:11]
	v_add_f64 v[8:9], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[16:17], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[18:19], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[12:13], v[18:19], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], -v[18:19]
	v_mul_f64 v[12:13], v[4:5], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[4:5], v[8:9], -v[12:13]
	v_cmp_class_f64_e64 vcc_lo, v[12:13], 0x204
	v_fma_f64 v[6:7], v[4:5], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_dual_cndmask_b32 v11, v9, v13 :: v_dual_cndmask_b32 v10, v8, v12
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[10:11]|
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_mul_f64 v[8:9], v[4:5], 0.5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v7, 0, v7, vcc_lo
	v_mul_f64 v[14:15], v[10:11], s[56:57]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[10:11]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[10:11]
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_rndne_f64_e32 v[14:15], v[14:15]
	s_and_b32 vcc_lo, s1, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], s[60:61], v[10:11]
	v_cvt_i32_f64_e32 v20, v[14:15]
	v_fma_f64 v[16:17], v[14:15], s[62:63], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], s[66:67], s[64:65]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[68:69]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[70:71]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[72:73]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[74:75]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[76:77]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[78:79]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[80:81]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[82:83]
	v_fma_f64 v[18:19], v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[18:19], 1.0
	v_ldexp_f64 v[12:13], v[14:15], v20
	v_trunc_f64_e32 v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v13, 0x7ff00000, v13, s0
	v_cndmask_b32_e32 v10, 0, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v11, 0, v13, s1
	v_trunc_f64_e32 v[12:13], v[8:9]
	v_cmp_eq_f64_e64 s1, v[14:15], v[4:5]
	v_fma_f64 v[6:7], v[10:11], v[6:7], v[10:11]
	v_cmp_class_f64_e64 s0, v[10:11], 0x204
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e32 vcc_lo, v[12:13], v[8:9]
	v_cndmask_b32_e64 v8, v6, v10, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v6, v7, v11, s0
	v_cndmask_b32_e64 v7, 0x3ff00000, v3, s2
	v_cndmask_b32_e64 v9, 0, v8, s1
	s_and_b32 s84, s1, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[4:5]
	s_and_b32 s0, s84, exec_lo
	s_cselect_b32 s0, s27, 0x3ff00000
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_bfi_b32 v10, 0x7fffffff, v6, s0
	v_cndmask_b32_e64 v6, 0, v2, s2
	v_cmp_eq_f64_e64 s2, s[26:27], 0
	v_cmp_lt_f64_e64 s0, s[26:27], 0
	v_cndmask_b32_e64 v2, 0x7ff80000, v10, s1
	s_delay_alu instid0(VALU_DEP_4)
	v_cmp_neq_f64_e64 s1, 0, v[6:7]
	s_xor_b32 s85, vcc_lo, s2
	s_or_b32 vcc_lo, s2, s33
	v_cndmask_b32_e64 v30, v10, v2, s0
	v_cndmask_b32_e64 v31, v8, v9, s0
	s_and_b32 s0, s85, exec_lo
	s_cselect_b32 s33, 0, 0x7ff00000
	s_and_b32 s0, s84, exec_lo
	s_cselect_b32 s86, s27, 0
	s_and_b32 s0, s1, exec_lo
	s_cselect_b32 s85, s15, 0x3ff00000
	s_cselect_b32 s84, s14, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[2:3], |s[84:85]|
	v_cmp_gt_f64_e64 s0, s[28:29], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0, 1, s0
	v_ldexp_f64 v[2:3], v[2:3], v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[2:3], 1.0
	v_add_f64 v[12:13], v[2:3], -1.0
	v_rcp_f64_e32 v[8:9], v[4:5]
	v_add_f64 v[14:15], v[4:5], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], -v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[4:5], v[8:9], 1.0
	v_fma_f64 v[8:9], v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[4:5], v[8:9], 1.0
	v_fma_f64 v[8:9], v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_mul_f64 v[16:17], v[4:5], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[10:11], v[4:5], -v[16:17]
	v_fma_f64 v[2:3], v[10:11], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[16:17], v[2:3]
	v_add_f64 v[14:15], v[12:13], -v[4:5]
	v_add_f64 v[16:17], v[4:5], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[2:3], v[16:17], -v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[12:13], -v[4:5]
	v_add_f64 v[2:3], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[14:15], v[2:3]
	v_mul_f64 v[2:3], v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[10:11], v[2:3]
	v_add_f64 v[8:9], v[4:5], -v[10:11]
	v_mul_f64 v[10:11], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], -v[8:9]
	v_fma_f64 v[8:9], v[4:5], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[2:3], v[2:3]
	v_fma_f64 v[8:9], v[4:5], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[10:11], v[8:9]
	v_fma_f64 v[14:15], v[12:13], s[34:35], s[30:31]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	v_mul_f64 v[20:21], v[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[36:37]
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[38:39]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[42:43]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[44:45]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[46:47]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[48:49]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[12:13], v[14:15]
	v_fma_f64 v[10:11], v[12:13], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[14:15], v[10:11]
	v_add_f64 v[14:15], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], s[28:29]
	v_add_f64 v[16:17], v[14:15], -v[16:17]
	v_add_f64 v[22:23], v[18:19], s[50:51]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_fma_f64 v[16:17], v[12:13], v[4:5], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], s[52:53]
	v_fma_f64 v[12:13], v[12:13], v[2:3], v[16:17]
	v_ldexp_f64 v[2:3], v[2:3], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_fma_f64 v[8:9], v[8:9], v[4:5], v[12:13]
	v_ldexp_f64 v[4:5], v[4:5], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[18:19], v[10:11]
	v_add_f64 v[14:15], v[20:21], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[18:19], -v[12:13]
	v_mul_f64 v[18:19], v[14:15], v[12:13]
	v_add_f64 v[20:21], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_fma_f64 v[16:17], v[14:15], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[20:21]
	v_fma_f64 v[10:11], v[14:15], v[10:11], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[8:9], v[12:13], v[10:11]
	v_frexp_exp_i32_f64_e32 v12, s[84:85]
	v_add_f64 v[10:11], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v12, null, 0, v12, s0
	v_cvt_f64_i32_e32 v[12:13], v12
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[4:5], v[10:11]
	v_add_f64 v[16:17], v[10:11], -v[18:19]
	v_mul_f64 v[18:19], v[12:13], s[54:55]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[14:15], -v[4:5]
	v_add_f64 v[8:9], v[8:9], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[12:13], s[54:55], -v[18:19]
	v_add_f64 v[4:5], v[10:11], -v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[2:3], v[2:3], v[8:9]
	v_fma_f64 v[8:9], v[12:13], s[58:59], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	v_add_f64 v[4:5], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[14:15], v[2:3]
	v_add_f64 v[18:19], v[4:5], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[4:5], v[10:11]
	v_add_f64 v[14:15], v[10:11], -v[14:15]
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[12:13], -v[4:5]
	v_add_f64 v[2:3], v[2:3], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[12:13], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[14:15], v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[20:21]
	v_add_f64 v[4:5], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[14:15], -v[8:9]
	v_add_f64 v[4:5], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[10:11]
	v_add_f64 v[2:3], v[2:3], -v[10:11]
	v_add_f64 v[16:17], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[10:11], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], v[8:9]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	v_add_f64 v[4:5], v[16:17], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[4:5], -v[16:17]
	v_mul_f64 v[10:11], v[6:7], v[4:5]
	v_add_f64 v[2:3], v[2:3], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[6:7], v[4:5], -v[10:11]
	v_cmp_class_f64_e64 s0, v[10:11], 0x204
	v_fma_f64 v[3:4], v[6:7], v[2:3], v[4:5]
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[1:2], 3, v[1:2]
	v_add_f64 v[8:9], v[10:11], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v13, v9, v11, s0
	v_cndmask_b32_e64 v12, v8, v10, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v20, s0, s10, v1
	v_add_co_ci_u32_e64 v21, null, s11, v2, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[12:13], s[56:57]
	v_add_co_u32 v22, s0, s6, v1
	global_load_b64 v[20:21], v[20:21], off
	v_add_co_ci_u32_e64 v23, null, s7, v2, s0
	v_add_co_u32 v26, s0, s8, v1
	v_add_co_ci_u32_e64 v27, null, s9, v2, s0
	global_load_b64 v[24:25], v[22:23], off
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_cmp_nlt_f64_e64 s1, 0x40900000, v[12:13]
	global_load_b64 v[28:29], v[26:27], off
	v_cmp_neq_f64_e64 s0, 0x7ff00000, |v[12:13]|
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[12:13]
	v_cmp_eq_f64_e64 s7, s[84:85], 0
	v_rndne_f64_e32 v[14:15], v[14:15]
	v_add_f64 v[3:4], v[3:4], -v[8:9]
	v_mul_f64 v[8:9], v[6:7], 0.5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[16:17], v[14:15], s[60:61], v[12:13]
	v_cvt_i32_f64_e32 v5, v[14:15]
	v_cndmask_b32_e64 v4, 0, v4, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_trunc_f64_e32 v[12:13], v[8:9]
	v_cndmask_b32_e64 v3, 0, v3, s0
	s_and_b32 s0, s2, s1
	v_fma_f64 v[16:17], v[14:15], s[62:63], v[16:17]
	v_fma_f64 v[18:19], v[16:17], s[66:67], s[64:65]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[68:69]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[70:71]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[72:73]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[74:75]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[76:77]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[78:79]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[80:81]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[82:83]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], 1.0
	v_fma_f64 v[14:15], v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[14:15], v5
	v_trunc_f64_e32 v[14:15], v[6:7]
	v_cndmask_b32_e64 v5, 0x7ff00000, v11, s1
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v10, 0, v10, s0
	v_cmp_neq_f64_e64 s0, v[12:13], v[8:9]
	v_add_f64 v[8:9], -s[14:15], 1.0
	v_mov_b32_e32 v13, s86
	v_cndmask_b32_e64 v11, 0, v5, s2
	v_cmp_eq_f64_e64 s2, v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[3:4], v[10:11], v[3:4], v[10:11]
	v_cmp_class_f64_e64 s1, v[10:11], 0x204
	s_and_b32 s6, s2, s0
	s_and_b32 s0, s6, exec_lo
	s_cselect_b32 s0, s85, 0x3ff00000
	v_cndmask_b32_e64 v10, v3, v10, s1
	v_cndmask_b32_e64 v5, v4, v11, s1
	s_waitcnt vmcnt(2)
	v_mul_f64 v[3:4], v[8:9], v[20:21]
	v_cmp_lt_f64_e64 s1, s[84:85], 0
	v_cndmask_b32_e64 v11, 0, v10, s2
	v_bfi_b32 v9, 0x7fffffff, v5, s0
	v_cmp_gt_f64_e64 s0, 0, v[6:7]
	v_add_f64 v[5:6], -s[12:13], 1.0
	s_waitcnt vmcnt(1)
	v_mul_f64 v[7:8], s[12:13], v[24:25]
	v_cndmask_b32_e64 v12, 0x7ff80000, v9, s2
	v_cmp_class_f64_e64 s2, s[84:85], 0x204
	v_mul_f64 v[3:4], v[20:21], v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v12, v9, v12, s1
	v_cndmask_b32_e64 v11, v10, v11, s1
	v_bfi_b32 v10, 0x7fffffff, s33, v13
	s_xor_b32 s8, s0, s7
	v_fma_f64 v[5:6], v[5:6], v[20:21], v[7:8]
	v_cndmask_b32_e32 v10, v30, v10, vcc_lo
	s_or_b32 s0, s7, s2
	s_and_b32 s1, s8, exec_lo
	s_cselect_b32 s1, 0, 0x7ff00000
	s_and_b32 s2, s6, exec_lo
	s_cselect_b32 s2, s85, 0
	v_cndmask_b32_e64 v11, v11, 0, s0
	v_mov_b32_e32 v9, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_bfi_b32 v13, 0x7fffffff, s1, v9
	v_cndmask_b32_e64 v9, v31, 0, vcc_lo
	v_cmp_o_f64_e64 vcc_lo, s[26:27], s[26:27]
	v_cndmask_b32_e64 v12, v12, v13, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], -v[9:10], 1.0
	v_cmp_o_f64_e64 s0, s[84:85], s[84:85]
	v_add_f64 v[11:12], -v[11:12], 1.0
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], s[14:15], v[28:29], v[3:4]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v8, 0x7ff80000, v10, vcc_lo
	v_cndmask_b32_e32 v7, 0, v9, vcc_lo
	v_cndmask_b32_e64 v10, 0x7ff80000, v12, s0
	v_cndmask_b32_e64 v9, 0, v11, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_div_scale_f64 v[11:12], null, v[7:8], v[7:8], v[5:6]
	v_div_scale_f64 v[28:29], vcc_lo, v[5:6], v[7:8], v[5:6]
	v_div_scale_f64 v[13:14], null, v[9:10], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[15:16], v[11:12]
	v_rcp_f64_e32 v[17:18], v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[19:20], -v[11:12], v[15:16], 1.0
	v_fma_f64 v[24:25], -v[13:14], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[15:16], v[19:20], v[15:16]
	v_fma_f64 v[17:18], v[17:18], v[24:25], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[19:20], -v[11:12], v[15:16], 1.0
	v_fma_f64 v[24:25], -v[13:14], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[15:16], v[19:20], v[15:16]
	v_div_scale_f64 v[19:20], s0, v[3:4], v[9:10], v[3:4]
	v_fma_f64 v[17:18], v[17:18], v[24:25], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[24:25], v[28:29], v[15:16]
	v_mul_f64 v[30:31], v[19:20], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], -v[11:12], v[24:25], v[28:29]
	v_fma_f64 v[13:14], -v[13:14], v[30:31], v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[11:12], v[11:12], v[15:16], v[24:25]
	s_mov_b32 vcc_lo, s0
	s_load_b64 s[0:1], s[16:17], 0x0
	v_div_fmas_f64 v[13:14], v[13:14], v[17:18], v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fixup_f64 v[7:8], v[11:12], v[7:8], v[5:6]
	v_div_fixup_f64 v[9:10], v[13:14], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[9:10]
	v_cndmask_b32_e64 v13, 0, 0x100, vcc_lo
	v_ldexp_f64 v[9:10], v[9:10], v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[13:14], v[9:10]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[15:16], v[9:10], v[13:14]
	v_mul_f64 v[13:14], v[13:14], 0.5
	v_fma_f64 v[17:18], -v[13:14], v[15:16], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[15:16], v[17:18], v[15:16]
	v_fma_f64 v[13:14], v[13:14], v[17:18], v[13:14]
	v_fma_f64 v[17:18], -v[15:16], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[17:18], -v[15:16], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[17:18], v[13:14], v[15:16]
	v_cndmask_b32_e64 v15, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[9:10], 0x260
	v_ldexp_f64 v[13:14], v[13:14], v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v10, v14, v10 :: v_dual_cndmask_b32 v9, v13, v9
	v_add_co_u32 v15, vcc_lo, s4, v1
	v_add_co_ci_u32_e64 v16, null, s5, v2, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3)
	v_add_f64 v[9:10], s[0:1], v[9:10]
	s_load_b64 s[0:1], s[18:19], 0x0
	global_store_b64 v[22:23], v[5:6], off
	global_store_b64 v[26:27], v[3:4], off
	global_load_b64 v[3:4], v[15:16], off
	v_div_scale_f64 v[11:12], null, v[9:10], v[9:10], v[7:8]
	v_div_scale_f64 v[17:18], vcc_lo, v[7:8], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[13:14], v[11:12]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[5:6], -v[11:12], v[13:14], 1.0
	v_fma_f64 v[5:6], v[13:14], v[5:6], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], -v[11:12], v[5:6], 1.0
	v_fma_f64 v[5:6], v[5:6], v[13:14], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[13:14], v[17:18], v[5:6]
	v_fma_f64 v[11:12], -v[11:12], v[13:14], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[5:6], v[11:12], v[5:6], v[13:14]
	v_add_co_u32 v1, vcc_lo, s20, v1
	v_add_co_ci_u32_e64 v2, null, s21, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[5:6], v[5:6], v[9:10], v[7:8]
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[6:7], s[0:1], v[3:4], v[5:6]
	global_store_b64 v[1:2], v[6:7], off
	global_load_b64 v[1:2], v[15:16], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[1:2], v[1:2]
	v_mul_f64 v[2:3], v[6:7], v[6:7]
.LBB3_2:
	s_or_b32 exec_lo, exec_lo, s3
	v_mbcnt_lo_u32_b32 v10, -1, 0
	v_and_b32_e32 v0, 31, v0
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_lshl_or_b32 v1, v10, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v10
	ds_bpermute_b32 v6, v1, v4
	ds_bpermute_b32 v7, v1, v5
	ds_bpermute_b32 v8, v1, v2
	ds_bpermute_b32 v9, v1, v3
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[2:3], v[8:9]
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v10
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v3, v3, v10, 2
	ds_bpermute_b32 v6, v3, v4
	ds_bpermute_b32 v7, v3, v5
	ds_bpermute_b32 v8, v3, v1
	ds_bpermute_b32 v9, v3, v2
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[3:4], v[4:5], v[6:7]
	v_cndmask_b32_e64 v5, 0, 4, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[8:9]
	v_cmp_gt_u32_e32 vcc_lo, 30, v10
	s_delay_alu instid0(VALU_DEP_3)
	v_add_lshl_u32 v8, v5, v10, 2
	ds_bpermute_b32 v5, v8, v3
	ds_bpermute_b32 v6, v8, v4
	ds_bpermute_b32 v7, v8, v1
	ds_bpermute_b32 v8, v8, v2
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[1:2], v[7:8]
	v_cndmask_b32_e64 v1, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v10
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v8, v1, v10, 2
	ds_bpermute_b32 v1, v8, v3
	ds_bpermute_b32 v2, v8, v4
	ds_bpermute_b32 v7, v8, v5
	ds_bpermute_b32 v8, v8, v6
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[1:2], v[3:4], v[1:2]
	v_add_co_ci_u32_e64 v3, null, 0, v10, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[4:5], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b32_e32 v3, 2, v3
	ds_bpermute_b32 v8, v3, v1
	ds_bpermute_b32 v9, v3, v2
	ds_bpermute_b32 v6, v3, v4
	ds_bpermute_b32 v7, v3, v5
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB3_14
; %bb.3:
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[0:1], v[1:2], v[8:9]
	v_mov_b32_e32 v8, 0
	v_bfrev_b32_e32 v9, 1
	s_mov_b32 s0, exec_lo
.LBB3_4:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ctz_i32_b32 s1, s0
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s3, v1, s1
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s2, v0, s1
	s_lshl_b32 s1, 1, s1
	s_and_not1_b32 s0, s0, s1
	v_add_f64 v[8:9], v[8:9], s[2:3]
	s_cmp_lg_u32 s0, 0
	s_cbranch_scc1 .LBB3_4
; %bb.5:
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_mov_b32 s1, 0
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_xor_b32 s0, exec_lo, s0
	s_cbranch_execz .LBB3_9
; %bb.6:
	v_mov_b32_e32 v10, 0
	global_load_b64 v[2:3], v10, s[22:23]
.LBB3_7:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[8:9]
	global_atomic_cmpswap_b64 v[0:1], v10, v[0:3], s[22:23] glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s1, vcc_lo, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execnz .LBB3_7
; %bb.8:
	s_or_b32 exec_lo, exec_lo, s1
.LBB3_9:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s0
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[0:1], v[4:5], v[6:7]
	v_mov_b32_e32 v4, 0
	v_bfrev_b32_e32 v5, 1
	s_mov_b32 s0, exec_lo
.LBB3_10:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ctz_i32_b32 s1, s0
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s3, v1, s1
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s2, v0, s1
	s_lshl_b32 s1, 1, s1
	s_and_not1_b32 s0, s0, s1
	v_add_f64 v[4:5], v[4:5], s[2:3]
	s_cmp_lg_u32 s0, 0
	s_cbranch_scc1 .LBB3_10
; %bb.11:
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_mov_b32 s0, 0
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB3_14
; %bb.12:
	v_mov_b32_e32 v6, 0
	global_load_b64 v[2:3], v6, s[24:25]
.LBB3_13:                               ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[4:5]
	global_atomic_cmpswap_b64 v[0:1], v6, v[0:3], s[24:25] glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB3_13
.LBB3_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lamb_phase1_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 352
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 32
		.amdhsa_next_free_sgpr 87
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 44
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end3:
	.size	lamb_phase1_kernel, .Lfunc_end3-lamb_phase1_kernel
                                        ; -- End function
	.set lamb_phase1_kernel.num_vgpr, 32
	.set lamb_phase1_kernel.num_agpr, 0
	.set lamb_phase1_kernel.numbered_sgpr, 87
	.set lamb_phase1_kernel.num_named_barrier, 0
	.set lamb_phase1_kernel.private_seg_size, 0
	.set lamb_phase1_kernel.uses_vcc, 1
	.set lamb_phase1_kernel.uses_flat_scratch, 0
	.set lamb_phase1_kernel.has_dyn_sized_stack, 0
	.set lamb_phase1_kernel.has_recursion, 0
	.set lamb_phase1_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 5516
; TotalNumSgprs: 89
; NumVgprs: 32
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 89
; NumVGPRsForWavesPerEU: 32
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	lamb_phase2_kernel      ; -- Begin function lamb_phase2_kernel
	.globl	lamb_phase2_kernel
	.p2align	8
	.type	lamb_phase2_kernel,@function
lamb_phase2_kernel:                     ; @lamb_phase2_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b32 s4, s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB4_2
; %bb.1:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[10:11], 0x0
	s_load_b64 s[10:11], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e64 s0, 0x10000000, s[2:3]
	v_cmp_gt_f64_e64 s1, 0x10000000, s[10:11]
	s_and_b32 s0, s0, exec_lo
	s_cselect_b32 s0, 0x100, 0
	s_cselect_b32 s12, 0xffffff80, 0
	s_and_b32 s1, s1, exec_lo
	s_cselect_b32 s1, 0x100, 0
	v_ldexp_f64 v[2:3], s[2:3], s0
	v_ldexp_f64 v[4:5], s[10:11], s1
	s_cselect_b32 s0, 0xffffff80, 0
	v_cmp_eq_f64_e64 s2, s[2:3], 0
	v_cmp_eq_f64_e64 s3, s[10:11], 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rsq_f64_e32 v[6:7], v[2:3]
	v_rsq_f64_e32 v[8:9], v[4:5]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x260
	s_or_b32 s2, s2, s3
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[10:11], v[2:3], v[6:7]
	v_mul_f64 v[6:7], v[6:7], 0.5
	v_mul_f64 v[12:13], v[4:5], v[8:9]
	v_mul_f64 v[8:9], v[8:9], 0.5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], -v[6:7], v[10:11], 0.5
	v_fma_f64 v[16:17], -v[8:9], v[12:13], 0.5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[10:11]
	v_fma_f64 v[6:7], v[6:7], v[14:15], v[6:7]
	v_fma_f64 v[12:13], v[12:13], v[16:17], v[12:13]
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], -v[10:11], v[10:11], v[2:3]
	v_fma_f64 v[16:17], -v[12:13], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[14:15], v[6:7], v[10:11]
	v_fma_f64 v[12:13], v[16:17], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], -v[10:11], v[10:11], v[2:3]
	v_fma_f64 v[16:17], -v[12:13], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[14:15], v[6:7], v[10:11]
	v_fma_f64 v[8:9], v[16:17], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[6:7], v[6:7], s12
	v_ldexp_f64 v[8:9], v[8:9], s0
	v_cmp_class_f64_e64 s0, v[4:5], 0x260
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v7, v7, v3 :: v_dual_cndmask_b32 v6, v6, v2
	v_ashrrev_i32_e32 v2, 31, v1
	v_cndmask_b32_e64 v5, v9, v5, s0
	v_cndmask_b32_e64 v4, v8, v4, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_div_scale_f64 v[8:9], null, v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[14:15], v[0:1], off
	v_div_scale_f64 v[16:17], vcc_lo, v[6:7], v[4:5], v[6:7]
	s_load_b64 s[0:1], s[8:9], 0x0
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_mul_f64 v[12:13], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[8:9], v[12:13], v[16:17]
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[8:9], v[4:5], v[6:7]
	v_cndmask_b32_e64 v5, v5, 0x3ff00000, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, v4, 0, s2
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[4:5], s[0:1], v[4:5]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[2:3], -v[2:3], v[4:5], v[14:15]
	global_store_b64 v[0:1], v[2:3], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lamb_phase2_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 304
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 18
		.amdhsa_next_free_sgpr 13
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 6
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end4:
	.size	lamb_phase2_kernel, .Lfunc_end4-lamb_phase2_kernel
                                        ; -- End function
	.set lamb_phase2_kernel.num_vgpr, 18
	.set lamb_phase2_kernel.num_agpr, 0
	.set lamb_phase2_kernel.numbered_sgpr, 13
	.set lamb_phase2_kernel.num_named_barrier, 0
	.set lamb_phase2_kernel.private_seg_size, 0
	.set lamb_phase2_kernel.uses_vcc, 1
	.set lamb_phase2_kernel.uses_flat_scratch, 0
	.set lamb_phase2_kernel.has_dyn_sized_stack, 0
	.set lamb_phase2_kernel.has_recursion, 0
	.set lamb_phase2_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 708
; TotalNumSgprs: 15
; NumVgprs: 18
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 15
; NumVGPRsForWavesPerEU: 18
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	lion_update_kernel      ; -- Begin function lion_update_kernel
	.globl	lion_update_kernel
	.p2align	8
	.type	lion_update_kernel,@function
lion_update_kernel:                     ; @lion_update_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x4c
	s_load_b32 s4, s[0:1], 0x38
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB5_2
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	v_add_co_u32 v6, vcc_lo, s8, v0
	v_add_co_ci_u32_e64 v7, null, s9, v1, vcc_lo
	global_load_b64 v[4:5], v[2:3], off
	v_add_co_u32 v0, vcc_lo, s4, v0
	global_load_b64 v[6:7], v[6:7], off
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	s_load_b256 s[0:7], s[0:1], 0x20
	global_load_b64 v[8:9], v[0:1], off
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[10:11], -s[0:1], 1.0
	s_waitcnt vmcnt(2)
	v_mul_f64 v[12:13], s[0:1], v[4:5]
	s_load_b64 s[0:1], s[2:3], 0x0
	s_load_b64 s[2:3], s[4:5], 0x0
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[4:5], s[0:1], v[4:5]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[6:7], v[12:13]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[10:11]
	v_cndmask_b32_e64 v13, 0, 0xbff00000, vcc_lo
	v_cmp_nlt_f64_e32 vcc_lo, 0, v[10:11]
	v_mov_b32_e32 v12, 0
	v_add_f64 v[10:11], -s[0:1], 1.0
	s_load_b64 s[0:1], s[10:11], 0x0
	v_cndmask_b32_e32 v13, 0x3ff00000, v13, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[10:11], v[6:7], v[4:5]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[12:13], s[2:3], v[8:9], v[12:13]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], -s[0:1], v[12:13], v[8:9]
	global_store_b64 v[0:1], v[8:9], off
	global_store_b64 v[2:3], v[4:5], off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lion_update_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 320
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 14
		.amdhsa_next_free_sgpr 12
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 3
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end5:
	.size	lion_update_kernel, .Lfunc_end5-lion_update_kernel
                                        ; -- End function
	.set lion_update_kernel.num_vgpr, 14
	.set lion_update_kernel.num_agpr, 0
	.set lion_update_kernel.numbered_sgpr, 12
	.set lion_update_kernel.num_named_barrier, 0
	.set lion_update_kernel.private_seg_size, 0
	.set lion_update_kernel.uses_vcc, 1
	.set lion_update_kernel.uses_flat_scratch, 0
	.set lion_update_kernel.has_dyn_sized_stack, 0
	.set lion_update_kernel.has_recursion, 0
	.set lion_update_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 360
; TotalNumSgprs: 14
; NumVgprs: 14
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 14
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	nadam_update_kernel     ; -- Begin function nadam_update_kernel
	.globl	nadam_update_kernel
	.p2align	8
	.type	nadam_update_kernel,@function
nadam_update_kernel:                    ; @nadam_update_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x54
	s_load_b64 s[78:79], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s79, v1
	s_cbranch_execz .LBB6_2
; %bb.1:
	s_load_b512 s[4:19], s[0:1], 0x0
	v_cvt_f64_i32_e32 v[2:3], s78
	s_mov_b32 s22, 0x55555555
	s_mov_b32 s23, 0x3fe55555
	s_mov_b32 s24, 0x968915a9
	s_mov_b32 s26, 0x4222de17
	s_mov_b32 s25, 0x3fba6564
	s_mov_b32 s27, 0x3fbdee67
	s_mov_b32 s28, 0x3abe935a
	s_mov_b32 s29, 0x3fbe25e4
	s_mov_b32 s30, 0x47e6c9c2
	s_mov_b32 s31, 0x3fc110ef
	s_mov_b32 s34, 0xcfa74449
	s_mov_b32 s35, 0x3fc3b13b
	s_mov_b32 s36, 0x71bf3c30
	s_mov_b32 s37, 0x3fc745d1
	s_mov_b32 s38, 0x1c7792ce
	s_mov_b32 s39, 0x3fcc71c7
	s_mov_b32 s40, 0x924920da
	s_mov_b32 s41, 0x3fd24924
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[14:15], s[14:15], 0x0
	s_mov_b32 s42, 0x9999999c
	s_mov_b32 s43, 0x3fd99999
	s_mov_b32 s45, 0xbfe55555
	s_mov_b32 s44, s22
	s_mov_b32 s46, 0xd5df274d
	s_mov_b32 s47, 0x3c8543b0
	s_mov_b32 s48, 0xfefa39ef
	s_mov_b32 s49, 0x3fe62e42
	s_mov_b32 s52, 0x3b39803f
	s_mov_b32 s53, 0x3c7abc9e
	s_mov_b32 s50, 0x652b82fe
	s_mov_b32 s51, 0x3ff71547
	s_mov_b32 s55, 0xbfe62e42
	s_mov_b32 s54, s48
	s_mov_b32 s57, 0xbc7abc9e
	s_mov_b32 s56, s52
	s_mov_b32 s58, 0xfca7ab0c
	s_mov_b32 s60, 0x6a5dcb37
	s_mov_b32 s59, 0x3e928af3
	s_waitcnt lgkmcnt(0)
	v_cmp_eq_f64_e64 s2, s[14:15], 1.0
	s_mov_b32 s61, 0x3e5ade15
	s_mov_b32 s62, 0x623fde64
	s_mov_b32 s63, 0x3ec71dee
	s_mov_b32 s64, 0x7c89e6b0
	s_mov_b32 s65, 0x3efa0199
	s_mov_b32 s66, 0x14761f6e
	s_mov_b32 s67, 0x3f2a01a0
	s_mov_b32 s68, 0x1852b7b0
	s_mov_b32 s69, 0x3f56c16c
	s_mov_b32 s70, 0x11122322
	s_mov_b32 s71, 0x3f811111
	s_mov_b32 s74, 0x555502a1
	s_mov_b32 s75, 0x3fa55555
	s_mov_b32 s76, 0x55555511
	s_mov_b32 s77, 0x3fc55555
	s_mov_b32 s72, 11
	s_mov_b32 s73, 0x3fe00000
	s_load_b64 s[16:17], s[16:17], 0x0
	v_cndmask_b32_e64 v5, v3, 0x3ff00000, s2
	v_cndmask_b32_e64 v4, v2, 0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[4:5]
	s_and_b32 s0, vcc_lo, exec_lo
	s_cselect_b32 s21, s15, 0x3ff00000
	s_cselect_b32 s20, s14, 0
	v_frexp_mant_f64_e64 v[6:7], |s[20:21]|
	v_cmp_eq_f64_e64 s33, s[20:21], 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[22:23], v[6:7]
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v0
	v_frexp_exp_i32_f64_e32 v0, s[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	v_add_f64 v[16:17], v[8:9], -1.0
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	v_fma_f64 v[8:9], v[12:13], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[8:9]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[6:7], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[8:9], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], -v[12:13]
	v_mul_f64 v[12:13], v[8:9], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[14:15], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[14:15], v[10:11]
	v_add_f64 v[14:15], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[14:15], s[26:27], s[24:25]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_mul_f64 v[22:23], v[8:9], v[14:15]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[34:35]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[38:39]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[42:43]
	v_mul_f64 v[18:19], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[16:17], -v[18:19]
	v_fma_f64 v[12:13], v[10:11], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[18:19], v[12:13]
	v_add_f64 v[20:21], v[16:17], s[22:23]
	v_add_f64 v[18:19], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[20:21], s[44:45]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_fma_f64 v[18:19], v[14:15], v[8:9], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	v_add_f64 v[12:13], v[12:13], s[46:47]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[14:15], v[6:7], v[18:19]
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_add_f64 v[12:13], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[8:9], v[14:15]
	v_ldexp_f64 v[8:9], v[8:9], 1
	v_add_f64 v[14:15], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[22:23], v[10:11]
	v_add_f64 v[18:19], v[20:21], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[20:21], v[16:17], v[14:15]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	v_add_f64 v[12:13], v[12:13], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[16:17], v[14:15], -v[20:21]
	v_add_f64 v[10:11], v[10:11], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[12:13], v[18:19]
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[12:13]
	v_cvt_f64_i32_e32 v[14:15], v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[20:21], v[10:11]
	v_add_f64 v[16:17], v[8:9], v[12:13]
	v_add_f64 v[18:19], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[20:21], v[14:15], s[48:49]
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	v_fma_f64 v[18:19], v[14:15], s[48:49], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[14:15], s[52:53], v[18:19]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[20:21], v[10:11]
	v_add_f64 v[12:13], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[8:9], -v[20:21]
	v_add_f64 v[14:15], v[8:9], v[12:13]
	v_add_f64 v[16:17], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	v_add_f64 v[18:19], v[14:15], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	v_add_f64 v[22:23], v[14:15], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[10:11], v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], v[8:9]
	v_add_f64 v[12:13], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[16:17], v[8:9]
	v_add_f64 v[16:17], v[16:17], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[14:15], v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[18:19], -v[14:15]
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[18:19], v[6:7]
	v_add_f64 v[10:11], v[8:9], -v[18:19]
	v_mul_f64 v[12:13], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	v_fma_f64 v[8:9], v[4:5], v[8:9], -v[12:13]
	v_cmp_class_f64_e64 vcc_lo, v[12:13], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v11, v9, v13 :: v_dual_cndmask_b32 v10, v8, v12
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_mul_f64 v[14:15], v[10:11], s[50:51]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[10:11]|
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[10:11]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[10:11]
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_mul_f64 v[8:9], v[4:5], 0.5
	v_rndne_f64_e32 v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v7, 0, v7 :: v_dual_cndmask_b32 v6, 0, v6
	s_and_b32 vcc_lo, s1, s0
	v_fma_f64 v[16:17], v[14:15], s[54:55], v[10:11]
	v_cvt_i32_f64_e32 v0, v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], s[56:57], v[16:17]
	v_fma_f64 v[18:19], v[16:17], s[60:61], s[58:59]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[62:63]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[64:65]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[66:67]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[68:69]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[70:71]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[74:75]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[76:77]
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[72:73]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], 1.0
	v_fma_f64 v[14:15], v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[12:13], v[14:15], v0
	v_trunc_f64_e32 v[14:15], v[4:5]
	v_cndmask_b32_e64 v0, 0x7ff00000, v13, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v10, 0, v12, vcc_lo
	v_trunc_f64_e32 v[12:13], v[8:9]
	v_cndmask_b32_e64 v11, 0, v0, s1
	v_cmp_eq_f64_e64 s1, v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[10:11], v[6:7], v[10:11]
	v_cmp_class_f64_e64 s0, v[10:11], 0x204
	v_cmp_neq_f64_e32 vcc_lo, v[12:13], v[8:9]
	v_cndmask_b32_e64 v0, v7, v11, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v6, v6, v10, s0
	v_cndmask_b32_e64 v7, 0, v6, s1
	s_and_b32 s3, s1, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[4:5]
	s_and_b32 s0, s3, exec_lo
	s_cselect_b32 s0, s21, 0x3ff00000
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_bfi_b32 v0, 0x7fffffff, v0, s0
	v_cmp_lt_f64_e64 s0, s[20:21], 0
	v_cndmask_b32_e64 v4, 0x7ff80000, v0, s1
	v_cmp_class_f64_e64 s1, s[20:21], 0x204
	s_xor_b32 s79, vcc_lo, s33
	v_cndmask_b32_e64 v0, v0, v4, s0
	v_cndmask_b32_e64 v4, v6, v7, s0
	s_or_b32 vcc_lo, s33, s1
	s_and_b32 s0, s79, exec_lo
	s_cselect_b32 s33, 0, 0x7ff00000
	s_and_b32 s0, s3, exec_lo
	s_cselect_b32 s82, s21, 0
	s_add_i32 s0, s78, 1
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f64_e64 s3, s[16:17], 1.0
	v_cvt_f64_i32_e32 v[5:6], s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v6, v6, 0x3ff00000, s2
	v_cndmask_b32_e64 v5, v5, 0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_neq_f64_e64 s0, 0, v[5:6]
	s_and_b32 s0, s0, exec_lo
	s_cselect_b32 s79, s15, 0x3ff00000
	s_cselect_b32 s78, s14, 0
	v_frexp_mant_f64_e64 v[7:8], |s[78:79]|
	v_cmp_class_f64_e64 s81, s[78:79], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e64 s0, s[22:23], v[7:8]
	v_cndmask_b32_e64 v9, 0, 1, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[7:8], v[7:8], v9
	v_add_f64 v[9:10], v[7:8], 1.0
	v_add_f64 v[15:16], v[7:8], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[11:12], v[9:10]
	v_add_f64 v[17:18], v[9:10], -1.0
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[13:14], v[11:12], v[11:12]
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[13:14], v[11:12], v[11:12]
	v_mul_f64 v[13:14], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[9:10], v[13:14]
	v_fma_f64 v[9:10], v[13:14], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[13:14], v[7:8], v[9:10]
	v_add_f64 v[9:10], v[19:20], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[15:16], -v[9:10]
	v_add_f64 v[19:20], v[9:10], -v[19:20]
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[19:20], -v[7:8]
	v_add_f64 v[9:10], v[15:16], -v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_add_f64 v[7:8], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[11:12], v[7:8]
	v_add_f64 v[9:10], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[9:10], -v[13:14]
	v_mul_f64 v[13:14], v[9:10], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[9:10], v[9:10], -v[13:14]
	v_add_f64 v[15:16], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	v_add_f64 v[15:16], v[13:14], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[15:16], s[26:27], s[24:25]
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_mul_f64 v[23:24], v[9:10], v[15:16]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[34:35]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[38:39]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[42:43]
	v_mul_f64 v[19:20], v[15:16], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[15:16], v[17:18], -v[19:20]
	v_fma_f64 v[13:14], v[11:12], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[19:20], v[13:14]
	v_add_f64 v[21:22], v[17:18], s[22:23]
	v_add_f64 v[19:20], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[21:22], s[44:45]
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	v_fma_f64 v[19:20], v[15:16], v[9:10], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[17:18], -v[25:26]
	v_add_f64 v[13:14], v[13:14], s[46:47]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[15:16], v[7:8], v[19:20]
	v_ldexp_f64 v[7:8], v[7:8], 1
	v_add_f64 v[13:14], v[13:14], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[11:12], v[11:12], v[9:10], v[15:16]
	v_ldexp_f64 v[9:10], v[9:10], 1
	v_add_f64 v[15:16], v[21:22], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[23:24], v[11:12]
	v_add_f64 v[19:20], v[21:22], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[21:22], v[17:18], v[15:16]
	v_add_f64 v[23:24], v[17:18], -v[23:24]
	v_add_f64 v[13:14], v[13:14], v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[19:20], v[17:18], v[15:16], -v[21:22]
	v_add_f64 v[11:12], v[11:12], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[17:18], v[13:14], v[19:20]
	v_fma_f64 v[11:12], v[11:12], v[15:16], v[13:14]
	v_frexp_exp_i32_f64_e32 v15, s[78:79]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[21:22], v[11:12]
	v_subrev_co_ci_u32_e64 v15, null, 0, v15, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[15:16], v15
	v_add_f64 v[17:18], v[9:10], v[13:14]
	v_add_f64 v[19:20], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[21:22], v[15:16], s[48:49]
	v_add_f64 v[9:10], v[17:18], -v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	v_fma_f64 v[19:20], v[15:16], s[48:49], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[13:14], -v[9:10]
	v_add_f64 v[7:8], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[15:16], s[52:53], v[19:20]
	v_add_f64 v[7:8], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[21:22], v[11:12]
	v_add_f64 v[13:14], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[9:10], -v[21:22]
	v_add_f64 v[15:16], v[9:10], v[13:14]
	v_add_f64 v[17:18], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[21:22]
	v_add_f64 v[19:20], v[15:16], -v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	v_add_f64 v[23:24], v[15:16], -v[19:20]
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[11:12], v[7:8]
	v_add_f64 v[9:10], v[9:10], -v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[13:14], v[9:10]
	v_add_f64 v[13:14], v[17:18], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[17:18], v[9:10]
	v_add_f64 v[17:18], v[17:18], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[15:16], v[9:10]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[19:20], -v[15:16]
	v_add_f64 v[7:8], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	v_add_f64 v[7:8], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[19:20], v[7:8]
	v_add_f64 v[11:12], v[9:10], -v[19:20]
	v_mul_f64 v[13:14], v[5:6], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	v_fma_f64 v[9:10], v[5:6], v[9:10], -v[13:14]
	v_cmp_class_f64_e64 s0, v[13:14], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], v[9:10]
	v_add_f64 v[9:10], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v12, v10, v14, s0
	v_cndmask_b32_e64 v11, v9, v13, s0
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[15:16], v[11:12], s[50:51]
	v_cmp_nlt_f64_e64 s1, 0x40900000, v[11:12]
	v_cmp_neq_f64_e64 s0, 0x7ff00000, |v[11:12]|
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[11:12]
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], 0.5
	v_rndne_f64_e32 v[15:16], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v8, 0, v8, s0
	v_cndmask_b32_e64 v7, 0, v7, s0
	s_and_b32 s0, s2, s1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], s[54:55], v[11:12]
	v_cvt_i32_f64_e32 v21, v[15:16]
	v_fma_f64 v[17:18], v[15:16], s[56:57], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], s[60:61], s[58:59]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[62:63]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[64:65]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[66:67]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[68:69]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[70:71]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[74:75]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[76:77]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[72:73]
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	v_ldexp_f64 v[13:14], v[15:16], v21
	v_trunc_f64_e32 v[15:16], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v14, 0x7ff00000, v14, s1
	v_cndmask_b32_e64 v11, 0, v13, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v12, 0, v14, s2
	v_trunc_f64_e32 v[13:14], v[9:10]
	v_cmp_eq_f64_e64 s2, v[15:16], v[5:6]
	v_fma_f64 v[7:8], v[11:12], v[7:8], v[11:12]
	v_cmp_class_f64_e64 s1, v[11:12], 0x204
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s0, v[13:14], v[9:10]
	v_cndmask_b32_e64 v9, v7, v11, s1
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v7, v8, v12, s1
	v_cndmask_b32_e64 v8, 0x3ff00000, v3, s3
	v_cmp_lt_f64_e64 s1, s[78:79], 0
	v_cndmask_b32_e64 v10, 0, v9, s2
	s_and_b32 s80, s2, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s0, s80, exec_lo
	s_cselect_b32 s0, s79, 0x3ff00000
	v_bfi_b32 v11, 0x7fffffff, v7, s0
	v_cndmask_b32_e64 v7, 0, v2, s3
	v_cmp_gt_f64_e64 s0, 0, v[5:6]
	v_cmp_eq_f64_e64 s3, s[78:79], 0
	v_cndmask_b32_e64 v32, v9, v10, s1
	v_cndmask_b32_e64 v2, 0x7ff80000, v11, s2
	v_cmp_neq_f64_e64 s2, 0, v[7:8]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v31, v11, v2, s1
	s_xor_b32 s83, s0, s3
	s_or_b32 s0, s3, s81
	s_and_b32 s1, s83, exec_lo
	s_cselect_b32 s83, 0, 0x7ff00000
	s_and_b32 s1, s80, exec_lo
	s_cselect_b32 s84, s79, 0
	s_and_b32 s1, s2, exec_lo
	s_cselect_b32 s81, s17, 0x3ff00000
	s_cselect_b32 s80, s16, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[2:3], |s[80:81]|
	v_cmp_gt_f64_e64 s1, s[22:23], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0, 1, s1
	v_ldexp_f64 v[2:3], v[2:3], v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[2:3], 1.0
	v_add_f64 v[13:14], v[2:3], -1.0
	v_rcp_f64_e32 v[9:10], v[5:6]
	v_add_f64 v[15:16], v[5:6], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[5:6], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[5:6], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	v_mul_f64 v[17:18], v[5:6], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[11:12], v[5:6], -v[17:18]
	v_fma_f64 v[2:3], v[11:12], v[2:3], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[17:18], v[2:3]
	v_add_f64 v[15:16], v[13:14], -v[5:6]
	v_add_f64 v[17:18], v[5:6], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_add_f64 v[2:3], v[17:18], -v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[13:14], -v[5:6]
	v_add_f64 v[2:3], v[2:3], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[15:16], v[2:3]
	v_mul_f64 v[2:3], v[9:10], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[11:12], v[2:3]
	v_add_f64 v[9:10], v[5:6], -v[11:12]
	v_mul_f64 v[11:12], v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], -v[9:10]
	v_fma_f64 v[9:10], v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[2:3], v[2:3]
	v_fma_f64 v[9:10], v[5:6], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[26:27], s[24:25]
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_mul_f64 v[21:22], v[5:6], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[28:29]
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[30:31]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[36:37]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[40:41]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[11:12], v[13:14], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	v_add_f64 v[15:16], v[17:18], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[15:16], s[22:23]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	v_add_f64 v[23:24], v[19:20], s[44:45]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_fma_f64 v[17:18], v[13:14], v[5:6], -v[21:22]
	v_add_f64 v[15:16], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], s[46:47]
	v_fma_f64 v[13:14], v[13:14], v[2:3], v[17:18]
	v_ldexp_f64 v[2:3], v[2:3], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[5:6], v[13:14]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[19:20], v[11:12]
	v_add_f64 v[15:16], v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[13:14]
	v_mul_f64 v[19:20], v[15:16], v[13:14]
	v_add_f64 v[21:22], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[17:18]
	v_fma_f64 v[17:18], v[15:16], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[21:22]
	v_fma_f64 v[11:12], v[15:16], v[11:12], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[9:10], v[13:14], v[11:12]
	v_frexp_exp_i32_f64_e32 v13, s[80:81]
	v_add_f64 v[11:12], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v13, null, 0, v13, s1
	v_cvt_f64_i32_e32 v[13:14], v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[5:6], v[11:12]
	v_add_f64 v[17:18], v[11:12], -v[19:20]
	v_mul_f64 v[19:20], v[13:14], s[48:49]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[15:16], -v[5:6]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], s[48:49], -v[19:20]
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[2:3], v[2:3], v[9:10]
	v_fma_f64 v[9:10], v[13:14], s[52:53], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], v[5:6]
	v_add_f64 v[5:6], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], v[2:3]
	v_add_f64 v[19:20], v[5:6], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[5:6], v[11:12]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[13:14], -v[5:6]
	v_add_f64 v[2:3], v[2:3], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[15:16], v[9:10], v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[21:22]
	v_add_f64 v[5:6], v[11:12], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], -v[9:10]
	v_add_f64 v[5:6], v[15:16], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[11:12]
	v_add_f64 v[2:3], v[2:3], -v[11:12]
	v_add_f64 v[17:18], v[13:14], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[11:12], v[17:18], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], v[9:10]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], v[5:6]
	v_add_f64 v[5:6], v[17:18], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[5:6], -v[17:18]
	v_mul_f64 v[11:12], v[7:8], v[5:6]
	v_add_f64 v[2:3], v[2:3], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[5:6], v[7:8], v[5:6], -v[11:12]
	v_cmp_class_f64_e64 s1, v[11:12], 0x204
	v_fma_f64 v[5:6], v[7:8], v[2:3], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[11:12], v[5:6]
	v_cndmask_b32_e64 v14, v10, v12, s1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v13, v9, v11, s1
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	v_mul_f64 v[2:3], v[13:14], s[50:51]
	v_cmp_nlt_f64_e64 s2, 0x40900000, v[13:14]
	v_cmp_ngt_f64_e64 s3, 0xc090cc00, v[13:14]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[7:8], 0.5
	v_rndne_f64_e32 v[15:16], v[2:3]
	v_fma_f64 v[2:3], v[15:16], s[54:55], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], s[56:57], v[2:3]
	v_fma_f64 v[2:3], v[17:18], s[60:61], s[58:59]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[17:18], v[2:3], s[62:63]
	v_fma_f64 v[2:3], v[17:18], v[2:3], s[64:65]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[17:18], v[2:3], s[66:67]
	v_fma_f64 v[2:3], v[17:18], v[2:3], s[68:69]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[17:18], v[2:3], s[70:71]
	v_fma_f64 v[19:20], v[17:18], v[2:3], s[74:75]
	v_ashrrev_i32_e32 v2, 31, v1
	v_cvt_i32_f64_e32 v3, v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 3, v[1:2]
	v_add_co_u32 v21, s1, s10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v22, null, s11, v2, s1
	v_add_co_u32 v23, s1, s6, v1
	v_add_co_ci_u32_e64 v24, null, s7, v2, s1
	global_load_b64 v[21:22], v[21:22], off
	v_add_co_u32 v27, s1, s8, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v28, null, s9, v2, s1
	global_load_b64 v[25:26], v[23:24], off
	v_cmp_neq_f64_e64 s1, 0x7ff00000, |v[13:14]|
	v_trunc_f64_e32 v[13:14], v[9:10]
	global_load_b64 v[29:30], v[27:28], off
	v_cmp_eq_f64_e64 s7, s[80:81], 0
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[76:77]
	v_cndmask_b32_e64 v6, 0, v6, s1
	v_cndmask_b32_e64 v5, 0, v5, s1
	s_and_b32 s1, s3, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[72:73]
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	v_add_f64 v[17:18], -s[14:15], 1.0
	v_ldexp_f64 v[11:12], v[15:16], v3
	v_trunc_f64_e32 v[15:16], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v3, 0x7ff00000, v12, s2
	v_cndmask_b32_e64 v11, 0, v11, s1
	v_cmp_neq_f64_e64 s1, v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v12, 0, v3, s3
	v_cmp_eq_f64_e64 s3, v[15:16], v[7:8]
	v_fma_f64 v[5:6], v[11:12], v[5:6], v[11:12]
	v_cmp_class_f64_e64 s2, v[11:12], 0x204
	s_and_b32 s6, s3, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 s1, s6, exec_lo
	s_cselect_b32 s1, s81, 0x3ff00000
	v_cndmask_b32_e64 v3, v5, v11, s2
	v_cndmask_b32_e64 v11, v6, v12, s2
	v_add_f64 v[5:6], -s[16:17], 1.0
	v_cmp_lt_f64_e64 s2, s[80:81], 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v13, 0, v3, s3
	v_bfi_b32 v14, 0x7fffffff, v11, s1
	v_cmp_gt_f64_e64 s1, 0, v[7:8]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v15, 0x7ff80000, v14, s3
	v_cmp_class_f64_e64 s3, s[80:81], 0x204
	s_xor_b32 s8, s1, s7
	s_or_b32 s1, s7, s3
	s_waitcnt vmcnt(2)
	v_mul_f64 v[9:10], v[17:18], v[21:22]
	v_mul_f64 v[5:6], v[5:6], v[21:22]
	v_cndmask_b32_e64 v18, v14, v15, s2
	v_cndmask_b32_e64 v15, v3, v13, s2
	s_and_b32 s2, s8, exec_lo
	s_cselect_b32 s2, 0, 0x7ff00000
	s_and_b32 s3, s6, exec_lo
	s_cselect_b32 s3, s81, 0
	s_waitcnt vmcnt(0)
	v_mul_f64 v[11:12], s[16:17], v[29:30]
	v_dual_mov_b32 v16, s84 :: v_dual_mov_b32 v19, s3
	v_mov_b32_e32 v17, s82
	v_cndmask_b32_e64 v13, v32, 0, s0
	v_cndmask_b32_e64 v3, v4, 0, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_bfi_b32 v16, 0x7fffffff, s83, v16
	v_cndmask_b32_e64 v15, v15, 0, s1
	v_bfi_b32 v17, 0x7fffffff, s33, v17
	v_cndmask_b32_e64 v14, v31, v16, s0
	v_bfi_b32 v16, 0x7fffffff, s2, v19
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e32 v4, v0, v17, vcc_lo
	v_cmp_o_f64_e64 vcc_lo, s[78:79], s[78:79]
	v_cmp_o_f64_e64 s0, s[20:21], s[20:21]
	v_add_f64 v[13:14], -v[13:14], 1.0
	v_cndmask_b32_e64 v16, v18, v16, s1
	v_add_f64 v[3:4], -v[3:4], 1.0
	v_cmp_o_f64_e64 s1, s[80:81], s[80:81]
	s_delay_alu instid0(VALU_DEP_3)
	v_add_f64 v[15:16], -v[15:16], 1.0
	v_fma_f64 v[7:8], s[14:15], v[25:26], v[9:10]
	v_fma_f64 v[5:6], v[21:22], v[5:6], v[11:12]
	v_cndmask_b32_e32 v12, 0x7ff80000, v14, vcc_lo
	v_cndmask_b32_e32 v11, 0, v13, vcc_lo
	v_cndmask_b32_e64 v4, 0x7ff80000, v4, s0
	v_cndmask_b32_e64 v3, 0, v3, s0
	v_cndmask_b32_e64 v16, 0x7ff80000, v16, s1
	v_cndmask_b32_e64 v15, 0, v15, s1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_div_scale_f64 v[19:20], null, v[3:4], v[3:4], v[9:10]
	v_mul_f64 v[17:18], s[14:15], v[7:8]
	v_div_scale_f64 v[21:22], null, v[15:16], v[15:16], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[29:30], v[19:20]
	v_div_scale_f64 v[13:14], null, v[11:12], v[11:12], v[17:18]
	v_div_scale_f64 v[39:40], vcc_lo, v[17:18], v[11:12], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[31:32], v[21:22]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[35:36], -v[19:20], v[29:30], 1.0
	v_rcp_f64_e32 v[25:26], v[13:14]
	v_fma_f64 v[37:38], -v[21:22], v[31:32], 1.0
	v_fma_f64 v[29:30], v[29:30], v[35:36], v[29:30]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[33:34], -v[13:14], v[25:26], 1.0
	v_fma_f64 v[31:32], v[31:32], v[37:38], v[31:32]
	v_fma_f64 v[35:36], -v[19:20], v[29:30], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[25:26], v[25:26], v[33:34], v[25:26]
	v_fma_f64 v[37:38], -v[21:22], v[31:32], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[29:30], v[29:30], v[35:36], v[29:30]
	v_div_scale_f64 v[35:36], s1, v[5:6], v[15:16], v[5:6]
	v_fma_f64 v[33:34], -v[13:14], v[25:26], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[31:32], v[37:38], v[31:32]
	v_fma_f64 v[25:26], v[25:26], v[33:34], v[25:26]
	v_div_scale_f64 v[33:34], s0, v[9:10], v[3:4], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[43:44], v[35:36], v[31:32]
	v_mul_f64 v[37:38], v[39:40], v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[41:42], v[33:34], v[29:30]
	v_fma_f64 v[21:22], -v[21:22], v[43:44], v[35:36]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], -v[13:14], v[37:38], v[39:40]
	v_fma_f64 v[19:20], -v[19:20], v[41:42], v[33:34]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[13:14], v[13:14], v[25:26], v[37:38]
	s_mov_b32 vcc_lo, s0
	v_div_fmas_f64 v[19:20], v[19:20], v[29:30], v[41:42]
	s_mov_b32 vcc_lo, s1
	s_load_b64 s[0:1], s[12:13], 0x0
	s_load_b64 s[2:3], s[18:19], 0x0
	v_div_fmas_f64 v[21:22], v[21:22], v[31:32], v[43:44]
	global_store_b64 v[23:24], v[7:8], off
	global_store_b64 v[27:28], v[5:6], off
	v_div_fixup_f64 v[3:4], v[19:20], v[3:4], v[9:10]
	v_div_fixup_f64 v[15:16], v[21:22], v[15:16], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[15:16]
	v_cndmask_b32_e64 v0, 0, 0x100, vcc_lo
	v_ldexp_f64 v[15:16], v[15:16], v0
	v_cndmask_b32_e64 v0, 0, 0xffffff80, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[21:22], v[15:16]
	v_cmp_class_f64_e64 vcc_lo, v[15:16], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[25:26], v[15:16], v[21:22]
	v_mul_f64 v[21:22], v[21:22], 0.5
	v_fma_f64 v[29:30], -v[21:22], v[25:26], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[25:26], v[25:26], v[29:30], v[25:26]
	v_fma_f64 v[21:22], v[21:22], v[29:30], v[21:22]
	v_fma_f64 v[29:30], -v[25:26], v[25:26], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[29:30], v[21:22], v[25:26]
	v_fma_f64 v[29:30], -v[25:26], v[25:26], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[21:22], v[29:30], v[21:22], v[25:26]
	v_ldexp_f64 v[9:10], v[21:22], v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v10, v10, v16, vcc_lo
	v_div_fixup_f64 v[11:12], v[13:14], v[11:12], v[17:18]
	v_cndmask_b32_e32 v9, v9, v15, vcc_lo
	v_add_co_u32 v0, vcc_lo, s4, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s5, v2, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[9:10], s[2:3], v[9:10]
	global_load_b64 v[5:6], v[0:1], off
	v_add_f64 v[3:4], v[11:12], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[3:4], s[0:1], v[3:4]
	v_div_scale_f64 v[11:12], null, v[9:10], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[13:14], v[11:12]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[7:8], -v[11:12], v[13:14], 1.0
	v_fma_f64 v[7:8], v[13:14], v[7:8], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], -v[11:12], v[7:8], 1.0
	v_fma_f64 v[7:8], v[7:8], v[13:14], v[7:8]
	v_div_scale_f64 v[13:14], vcc_lo, v[3:4], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[13:14], v[7:8]
	v_fma_f64 v[11:12], -v[11:12], v[15:16], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[7:8], v[11:12], v[7:8], v[15:16]
	v_div_fixup_f64 v[2:3], v[7:8], v[9:10], v[3:4]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[5:6], -v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel nadam_update_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 328
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 45
		.amdhsa_next_free_sgpr 85
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 52
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end6:
	.size	nadam_update_kernel, .Lfunc_end6-nadam_update_kernel
                                        ; -- End function
	.set nadam_update_kernel.num_vgpr, 45
	.set nadam_update_kernel.num_agpr, 0
	.set nadam_update_kernel.numbered_sgpr, 85
	.set nadam_update_kernel.num_named_barrier, 0
	.set nadam_update_kernel.private_seg_size, 0
	.set nadam_update_kernel.uses_vcc, 1
	.set nadam_update_kernel.uses_flat_scratch, 0
	.set nadam_update_kernel.has_dyn_sized_stack, 0
	.set nadam_update_kernel.has_recursion, 0
	.set nadam_update_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 6560
; TotalNumSgprs: 87
; NumVgprs: 45
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 87
; NumVGPRsForWavesPerEU: 45
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	clip_value_kernel       ; -- Begin function clip_value_kernel
	.globl	clip_value_kernel
	.p2align	8
	.type	clip_value_kernel,@function
clip_value_kernel:                      ; @clip_value_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB7_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	s_load_b64 s[2:3], s[6:7], 0x0
	s_load_b64 s[0:1], s[0:1], 0x0
	global_load_b64 v[2:3], v[0:1], off
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[2:3]
	v_cndmask_b32_e64 v3, v3, s3, vcc_lo
	v_cndmask_b32_e64 v2, v2, s2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_lt_f64_e32 vcc_lo, s[0:1], v[2:3]
	v_cndmask_b32_e64 v3, v3, s1, vcc_lo
	v_cndmask_b32_e64 v2, v2, s0, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB7_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel clip_value_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 288
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 4
		.amdhsa_next_free_sgpr 8
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 2
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end7:
	.size	clip_value_kernel, .Lfunc_end7-clip_value_kernel
                                        ; -- End function
	.set clip_value_kernel.num_vgpr, 4
	.set clip_value_kernel.num_agpr, 0
	.set clip_value_kernel.numbered_sgpr, 8
	.set clip_value_kernel.num_named_barrier, 0
	.set clip_value_kernel.private_seg_size, 0
	.set clip_value_kernel.uses_vcc, 1
	.set clip_value_kernel.uses_flat_scratch, 0
	.set clip_value_kernel.has_dyn_sized_stack, 0
	.set clip_value_kernel.has_recursion, 0
	.set clip_value_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 200
; TotalNumSgprs: 10
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 4
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	sgd_update_f64_kernel   ; -- Begin function sgd_update_f64_kernel
	.globl	sgd_update_f64_kernel
	.p2align	8
	.type	sgd_update_f64_kernel,@function
sgd_update_f64_kernel:                  ; @sgd_update_f64_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB8_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[0:1], off
	s_load_b64 s[0:1], s[0:1], 0x0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[2:3], s[0:1], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB8_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel sgd_update_f64_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 288
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 0
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 6
		.amdhsa_next_free_sgpr 8
		.amdhsa_reserve_vcc 1
		.amdhsa_float_round_mode_32 0
		.amdhsa_float_round_mode_16_64 0
		.amdhsa_float_denorm_mode_32 3
		.amdhsa_float_denorm_mode_16_64 3
		.amdhsa_dx10_clamp 1
		.amdhsa_ieee_mode 1
		.amdhsa_fp16_overflow 0
		.amdhsa_workgroup_processor_mode 1
		.amdhsa_memory_ordered 1
		.amdhsa_forward_progress 1
		.amdhsa_shared_vgpr_count 0
		.amdhsa_inst_pref_size 2
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end8:
	.size	sgd_update_f64_kernel, .Lfunc_end8-sgd_update_f64_kernel
                                        ; -- End function
	.set sgd_update_f64_kernel.num_vgpr, 6
	.set sgd_update_f64_kernel.num_agpr, 0
	.set sgd_update_f64_kernel.numbered_sgpr, 8
	.set sgd_update_f64_kernel.num_named_barrier, 0
	.set sgd_update_f64_kernel.private_seg_size, 0
	.set sgd_update_f64_kernel.uses_vcc, 1
	.set sgd_update_f64_kernel.uses_flat_scratch, 0
	.set sgd_update_f64_kernel.has_dyn_sized_stack, 0
	.set sgd_update_f64_kernel.has_recursion, 0
	.set sgd_update_f64_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 180
; TotalNumSgprs: 10
; NumVgprs: 6
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 6
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_a4d0a144ee294820,@object ; @__hip_cuid_a4d0a144ee294820
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_a4d0a144ee294820
__hip_cuid_a4d0a144ee294820:
	.byte	0                               ; 0x0
	.size	__hip_cuid_a4d0a144ee294820, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_a4d0a144ee294820
	.amdgpu_metadata
---
amdhsa.kernels:
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
      - .offset:         40
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         52
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         56
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         60
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         62
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         64
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         66
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         68
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         70
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         96
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         112
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           momentum_update_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         momentum_update_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     8
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         40
        .size:           8
        .value_kind:     global_buffer
      - .offset:         48
        .size:           4
        .value_kind:     by_value
      - .offset:         56
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         60
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         64
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         68
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         70
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         72
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         74
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         76
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         78
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         96
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         120
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 312
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           rmsprop_update_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         rmsprop_update_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     16
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
      - .offset:         40
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         52
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         56
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         60
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         62
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         64
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         66
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         68
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         70
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         96
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         112
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           adagrad_update_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         adagrad_update_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     16
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         40
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         48
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         56
        .size:           8
        .value_kind:     global_buffer
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         68
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         72
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         80
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         88
        .size:           8
        .value_kind:     global_buffer
      - .offset:         96
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         100
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         104
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         108
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         110
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         112
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         114
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         116
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         118
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         136
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         144
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         152
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         160
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 352
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lamb_phase1_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     89
    .sgpr_spill_count: 0
    .symbol:         lamb_phase1_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     32
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
      - .offset:         40
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         52
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         56
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         60
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         62
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         64
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         66
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         68
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         70
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         96
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         112
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lamb_phase2_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     15
    .sgpr_spill_count: 0
    .symbol:         lamb_phase2_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     18
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         40
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         48
        .size:           8
        .value_kind:     global_buffer
      - .offset:         56
        .size:           4
        .value_kind:     by_value
      - .offset:         64
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         68
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         76
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         78
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         80
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         82
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         84
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         86
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         128
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 320
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lion_update_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         lion_update_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     14
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         40
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         48
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         56
        .size:           8
        .value_kind:     global_buffer
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         68
        .size:           4
        .value_kind:     by_value
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         76
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         80
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         84
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         86
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         88
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         90
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         92
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         94
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         136
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 328
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           nadam_update_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     87
    .sgpr_spill_count: 0
    .symbol:         nadam_update_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     45
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         36
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         40
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         44
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         46
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         48
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         50
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         52
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         54
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         96
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           clip_value_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         clip_value_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     4
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         36
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         40
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         44
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         46
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         48
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         50
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         52
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         54
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         96
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           sgd_update_f64_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         sgd_update_f64_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     6
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
