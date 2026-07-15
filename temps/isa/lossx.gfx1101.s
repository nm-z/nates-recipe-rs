	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	lossx_mse_kernel        ; -- Begin function lossx_mse_kernel
	.globl	lossx_mse_kernel
	.p2align	8
	.type	lossx_mse_kernel,@function
lossx_mse_kernel:                       ; @lossx_mse_kernel
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
	s_cbranch_execz .LBB0_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], -v[4:5]
	v_mul_f64 v[2:3], v[2:3], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_mse_kernel
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
.Lfunc_end0:
	.size	lossx_mse_kernel, .Lfunc_end0-lossx_mse_kernel
                                        ; -- End function
	.set lossx_mse_kernel.num_vgpr, 6
	.set lossx_mse_kernel.num_agpr, 0
	.set lossx_mse_kernel.numbered_sgpr, 8
	.set lossx_mse_kernel.num_named_barrier, 0
	.set lossx_mse_kernel.private_seg_size, 0
	.set lossx_mse_kernel.uses_vcc, 1
	.set lossx_mse_kernel.uses_flat_scratch, 0
	.set lossx_mse_kernel.has_dyn_sized_stack, 0
	.set lossx_mse_kernel.has_recursion, 0
	.set lossx_mse_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 200
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
	.protected	lossx_mae_kernel        ; -- Begin function lossx_mae_kernel
	.globl	lossx_mae_kernel
	.p2align	8
	.type	lossx_mae_kernel,@function
lossx_mae_kernel:                       ; @lossx_mae_kernel
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
	s_cbranch_execz .LBB1_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], -v[4:5]
	v_and_b32_e32 v3, 0x7fffffff, v3
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_mae_kernel
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
.Lfunc_end1:
	.size	lossx_mae_kernel, .Lfunc_end1-lossx_mae_kernel
                                        ; -- End function
	.set lossx_mae_kernel.num_vgpr, 6
	.set lossx_mae_kernel.num_agpr, 0
	.set lossx_mae_kernel.numbered_sgpr, 8
	.set lossx_mae_kernel.num_named_barrier, 0
	.set lossx_mae_kernel.private_seg_size, 0
	.set lossx_mae_kernel.uses_vcc, 1
	.set lossx_mae_kernel.uses_flat_scratch, 0
	.set lossx_mae_kernel.has_dyn_sized_stack, 0
	.set lossx_mae_kernel.has_recursion, 0
	.set lossx_mae_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 200
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
	.protected	lossx_log_cosh_kernel   ; -- Begin function lossx_log_cosh_kernel
	.globl	lossx_log_cosh_kernel
	.p2align	8
	.type	lossx_log_cosh_kernel,@function
lossx_log_cosh_kernel:                  ; @lossx_log_cosh_kernel
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
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_mov_b32 s8, 0x6a5dcb37
	s_mov_b32 s9, 0x3e5ade15
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s11, 0x3fc3ab76
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0xbfe62e42
	s_mov_b32 s7, 0xbc7abc9e
	s_mov_b32 s6, 0x3b39803f
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], -v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], |v[2:3]|, -2.0
	v_mul_f64 v[6:7], v[4:5], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[4:5], v[4:5]
	v_cvt_i32_f64_e32 v12, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[6:7], v[8:9]
	s_mov_b32 s7, 0x3c7abc9e
	v_fma_f64 v[10:11], v[8:9], s[8:9], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s9, 0x3fc38538
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[4:5]
	s_mov_b32 s1, 0x3fe55555
	v_fma_f64 v[10:11], v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[8:9], v[10:11], 1.0
	v_ldexp_f64 v[6:7], v[6:7], v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v4, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0, v7, s0
	s_mov_b32 s0, 0x55555555
	v_add_f64 v[6:7], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	v_add_f64 v[10:11], v[6:7], -1.0
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[8:9]
	s_mov_b32 s0, 0x55555780
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[10:11], -v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[10:11]
	v_subrev_co_ci_u32_e64 v28, null, 0, v12, vcc_lo
	v_add_f64 v[8:9], v[8:9], 1.0
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v14, 0, v28
	v_ldexp_f64 v[6:7], v[6:7], v14
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], v[8:9]
	v_add_f64 v[12:13], v[6:7], 1.0
	v_add_f64 v[18:19], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[8:9], v[8:9], v14
	v_add_f64 v[10:11], v[12:13], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[18:19], 1.0
	v_add_f64 v[10:11], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	v_add_f64 v[10:11], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[14:15], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[18:19], v[6:7]
	v_rcp_f64_e32 v[16:17], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[22:23], -v[14:15], v[16:17], 1.0
	v_add_f64 v[6:7], v[6:7], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[22:23], v[16:17], v[16:17]
	v_fma_f64 v[8:9], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[16:17]
	v_mul_f64 v[16:17], v[20:21], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[22:23], v[14:15], v[16:17]
	v_fma_f64 v[12:13], v[16:17], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[10:11], v[12:13]
	v_add_f64 v[24:25], v[22:23], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[20:21], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[22:23]
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_add_f64 v[20:21], v[20:21], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[20:21]
	v_add_f64 v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[26:27], v[6:7]
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	v_add_f64 v[24:25], v[26:27], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[20:21], v[14:15], v[18:19]
	v_add_f64 v[6:7], v[6:7], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[18:19], v[14:15], -v[20:21]
	v_fma_f64 v[10:11], v[18:19], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[20:21], v[10:11]
	v_add_f64 v[22:23], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	v_add_f64 v[10:11], v[20:21], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[10:11], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[22:23], v[6:7]
	v_add_f64 v[10:11], v[18:19], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[0:1]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s1, 0x3fe62e42
	s_mov_b32 s0, s4
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v28
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[16:17], s[0:1]
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[0:1], -v[18:19]
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[4:5]
	v_cmp_ngt_f64_e64 s1, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[6:7], v[14:15]
	s_and_b32 vcc_lo, vcc_lo, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[6:7]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[8:9], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[16:17], -v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], -v[20:21]
	v_add_f64 v[12:13], v[14:15], -v[20:21]
	v_add_f64 v[14:15], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[14:15], -v[10:11]
	v_add_f64 v[8:9], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[18:19], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[12:13], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[6:7], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[4:5]
	v_cndmask_b32_e64 v7, 0x7ff00000, v7, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0x7ff80000, v7, s1
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_add_f64 v[2:3], |v[2:3]|, v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], s[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_log_cosh_kernel
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
		.amdhsa_next_free_vgpr 29
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
		.amdhsa_inst_pref_size 16
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
	.size	lossx_log_cosh_kernel, .Lfunc_end2-lossx_log_cosh_kernel
                                        ; -- End function
	.set lossx_log_cosh_kernel.num_vgpr, 29
	.set lossx_log_cosh_kernel.num_agpr, 0
	.set lossx_log_cosh_kernel.numbered_sgpr, 12
	.set lossx_log_cosh_kernel.num_named_barrier, 0
	.set lossx_log_cosh_kernel.private_seg_size, 0
	.set lossx_log_cosh_kernel.uses_vcc, 1
	.set lossx_log_cosh_kernel.uses_flat_scratch, 0
	.set lossx_log_cosh_kernel.has_dyn_sized_stack, 0
	.set lossx_log_cosh_kernel.has_recursion, 0
	.set lossx_log_cosh_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1928
; TotalNumSgprs: 14
; NumVgprs: 29
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 29
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
	.protected	lossx_bce_kernel        ; -- Begin function lossx_bce_kernel
	.globl	lossx_bce_kernel
	.p2align	8
	.type	lossx_bce_kernel,@function
lossx_bce_kernel:                       ; @lossx_bce_kernel
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
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_movk_i32 s0, 0xdcd1
	s_mov_b32 s1, 0x3fefffff
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s9, 0x3fc3ab76
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s10, 0xd7f4df2e
	s_mov_b32 s11, 0x3fc7474d
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s5, 0x3fc38538
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_nlt_f64_e32 vcc_lo, s[0:1], v[2:3]
	s_mov_b32 s0, 0x812dea11
	s_mov_b32 s1, 0x3d719799
	v_cndmask_b32_e32 v4, 0xffffdcd1, v2, vcc_lo
	v_cndmask_b32_e32 v5, 0x3fefffff, v3, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, s[0:1], v[2:3]
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s1, 0x3fe55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, 0x3d719799, v5, vcc_lo
	v_cndmask_b32_e32 v2, 0x812dea11, v4, vcc_lo
	v_add_f64 v[4:5], -v[2:3], 1.0
	v_frexp_mant_f64_e32 v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[6:7], v[4:5]
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e64 s0, s[0:1], v[8:9]
	v_cndmask_b32_e64 v10, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v10, 0, 1, s0
	v_ldexp_f64 v[8:9], v[8:9], v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[6:7], 1.0
	v_add_f64 v[22:23], v[6:7], -1.0
	v_add_f64 v[12:13], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[14:15], v[10:11]
	v_add_f64 v[26:27], v[10:11], -1.0
	v_rcp_f64_e32 v[16:17], v[12:13]
	v_add_f64 v[30:31], v[12:13], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[26:27]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[20:21], -v[12:13], v[16:17], 1.0
	v_fma_f64 v[14:15], v[18:19], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[20:21], v[16:17], v[16:17]
	v_fma_f64 v[18:19], -v[10:11], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], -v[12:13], v[16:17], 1.0
	v_fma_f64 v[14:15], v[18:19], v[14:15], v[14:15]
	v_add_f64 v[18:19], v[8:9], -1.0
	v_add_f64 v[8:9], v[8:9], -v[30:31]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[16:17], v[20:21], v[16:17], v[16:17]
	v_mul_f64 v[20:21], v[22:23], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[24:25], v[18:19], v[16:17]
	v_mul_f64 v[28:29], v[10:11], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[32:33], v[12:13], v[24:25]
	v_fma_f64 v[10:11], v[20:21], v[10:11], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[24:25], v[12:13], -v[32:33]
	v_fma_f64 v[6:7], v[20:21], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[24:25], v[8:9], v[12:13]
	v_add_f64 v[10:11], v[28:29], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[32:33], v[8:9]
	v_add_f64 v[26:27], v[22:23], -v[10:11]
	v_add_f64 v[28:29], v[10:11], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[30:31], v[18:19], -v[12:13]
	v_add_f64 v[32:33], v[12:13], -v[32:33]
	v_add_f64 v[22:23], v[22:23], -v[26:27]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[6:7], v[28:29], -v[6:7]
	v_frexp_exp_i32_f64_e32 v28, v[4:5]
	v_frexp_exp_i32_f64_e32 v29, v[2:3]
	v_add_f64 v[18:19], v[18:19], -v[30:31]
	v_add_f64 v[8:9], v[32:33], -v[8:9]
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	v_subrev_co_ci_u32_e64 v28, null, 0, v28, vcc_lo
	v_add_co_u32 v36, vcc_lo, s6, v0
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_add_co_ci_u32_e64 v37, null, s7, v1, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x204
	global_load_b64 v[36:37], v[36:37], off
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[8:9], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[26:27], v[6:7]
	v_add_f64 v[8:9], v[30:31], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[6:7], v[14:15], v[6:7]
	v_mul_f64 v[8:9], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[20:21], v[6:7]
	v_add_f64 v[12:13], v[24:25], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[14:15], v[10:11], v[10:11]
	v_mul_f64 v[16:17], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[14:15], s[8:9], s[4:5]
	v_mul_f64 v[26:27], v[10:11], v[14:15]
	v_fma_f64 v[22:23], v[16:17], s[8:9], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[10:11]
	v_fma_f64 v[22:23], v[16:17], v[22:23], s[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[4:5]
	v_fma_f64 v[22:23], v[16:17], v[22:23], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	v_fma_f64 v[22:23], v[16:17], v[22:23], s[8:9]
	s_mov_b32 s8, 0x55555780
	s_mov_b32 s9, s1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[4:5]
	v_fma_f64 v[22:23], v[16:17], v[22:23], s[4:5]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[14:15], v[18:19], s[8:9]
	v_mul_f64 v[18:19], v[12:13], v[16:17]
	v_fma_f64 v[16:17], v[16:17], v[22:23], s[8:9]
	v_ldexp_f64 v[22:23], v[10:11], 1
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	v_mul_f64 v[14:15], v[26:27], v[14:15]
	v_ldexp_f64 v[26:27], v[12:13], 1
	v_add_f64 v[12:13], v[12:13], -v[24:25]
	v_mul_f64 v[16:17], v[18:19], v[16:17]
	v_cvt_f64_i32_e32 v[18:19], v28
	v_subrev_co_ci_u32_e64 v28, null, 0, v29, s0
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	v_cvt_f64_i32_e32 v[28:29], v28
	v_add_f64 v[20:21], v[22:23], v[14:15]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[24:25], v[26:27], v[16:17]
	v_mul_f64 v[30:31], v[18:19], s[0:1]
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_add_f64 v[10:11], v[20:21], -v[22:23]
	v_mul_f64 v[22:23], v[28:29], s[0:1]
	v_ldexp_f64 v[8:9], v[8:9], 1
	v_add_f64 v[12:13], v[24:25], -v[26:27]
	v_fma_f64 v[26:27], v[18:19], s[0:1], -v[30:31]
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	v_fma_f64 v[14:15], v[28:29], s[0:1], -v[22:23]
	v_cmp_class_f64_e64 s0, v[2:3], 0x204
	v_add_f64 v[12:13], v[16:17], -v[12:13]
	v_fma_f64 v[16:17], v[18:19], s[4:5], v[26:27]
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_fma_f64 v[10:11], v[28:29], s[4:5], v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[12:13], v[30:31], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[20:21], v[6:7]
	v_add_f64 v[18:19], v[22:23], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[24:25], v[8:9]
	v_add_f64 v[30:31], v[12:13], -v[30:31]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[28:29], v[12:13], v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[20:21]
	v_add_f64 v[22:23], v[18:19], -v[22:23]
	v_add_f64 v[32:33], v[18:19], v[26:27]
	v_add_f64 v[16:17], v[16:17], -v[30:31]
	v_add_f64 v[24:25], v[26:27], -v[24:25]
	v_add_f64 v[34:35], v[28:29], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	v_add_f64 v[10:11], v[10:11], -v[22:23]
	v_add_f64 v[38:39], v[32:33], -v[18:19]
	v_add_f64 v[8:9], v[8:9], -v[24:25]
	v_add_f64 v[40:41], v[28:29], -v[34:35]
	v_add_f64 v[14:15], v[14:15], -v[34:35]
	v_add_f64 v[22:23], v[16:17], v[6:7]
	v_add_f64 v[30:31], v[32:33], -v[38:39]
	v_add_f64 v[20:21], v[26:27], -v[38:39]
	v_add_f64 v[12:13], v[12:13], -v[40:41]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[30:31]
	v_add_f64 v[12:13], v[14:15], v[12:13]
	v_add_f64 v[14:15], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[20:21], v[18:19]
	v_add_f64 v[20:21], v[22:23], -v[16:17]
	v_add_f64 v[12:13], v[22:23], v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[24:25], v[14:15], -v[10:11]
	v_add_f64 v[18:19], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[22:23], v[22:23], -v[20:21]
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	v_add_f64 v[26:27], v[28:29], v[12:13]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	v_add_f64 v[8:9], v[8:9], -v[24:25]
	v_add_f64 v[20:21], v[32:33], v[18:19]
	v_add_f64 v[16:17], v[16:17], -v[22:23]
	v_add_f64 v[22:23], v[26:27], -v[28:29]
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[20:21], -v[32:33]
	v_add_f64 v[6:7], v[6:7], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[18:19], -v[14:15]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_waitcnt vmcnt(0)
	v_add_f64 v[10:11], -v[36:37], 1.0
	v_add_f64 v[6:7], v[26:27], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[20:21], v[8:9]
	v_dual_cndmask_b32 v6, v6, v4 :: v_dual_cndmask_b32 v7, v7, v5
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v7, 0x7ff80000, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	v_mul_f64 v[4:5], v[10:11], v[6:7]
	v_cndmask_b32_e64 v7, v9, v3, s0
	v_cndmask_b32_e64 v6, v8, v2, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v7, 0x7ff80000, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[36:37], v[6:7], v[4:5]
	v_xor_b32_e32 v3, 0x80000000, v3
	global_store_b64 v[0:1], v[2:3], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_bce_kernel
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
		.amdhsa_next_free_vgpr 42
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
		.amdhsa_inst_pref_size 15
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
	.size	lossx_bce_kernel, .Lfunc_end3-lossx_bce_kernel
                                        ; -- End function
	.set lossx_bce_kernel.num_vgpr, 42
	.set lossx_bce_kernel.num_agpr, 0
	.set lossx_bce_kernel.numbered_sgpr, 12
	.set lossx_bce_kernel.num_named_barrier, 0
	.set lossx_bce_kernel.private_seg_size, 0
	.set lossx_bce_kernel.uses_vcc, 1
	.set lossx_bce_kernel.uses_flat_scratch, 0
	.set lossx_bce_kernel.has_dyn_sized_stack, 0
	.set lossx_bce_kernel.has_recursion, 0
	.set lossx_bce_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1892
; TotalNumSgprs: 14
; NumVgprs: 42
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 42
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
	.protected	lossx_poisson_nll_kernel ; -- Begin function lossx_poisson_nll_kernel
	.globl	lossx_poisson_nll_kernel
	.p2align	8
	.type	lossx_poisson_nll_kernel,@function
lossx_poisson_nll_kernel:               ; @lossx_poisson_nll_kernel
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
	s_cbranch_execz .LBB4_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	v_add_co_u32 v10, vcc_lo, s6, v0
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_ci_u32_e64 v11, null, s7, v1, vcc_lo
	global_load_b64 v[10:11], v[10:11], off
	s_waitcnt vmcnt(1)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[2:3]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	v_cvt_i32_f64_e32 v12, v[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[6:7]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v12
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_cndmask_b32_e64 v5, 0, v5, s0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[2:3], -v[2:3], v[10:11], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_poisson_nll_kernel
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
		.amdhsa_next_free_vgpr 13
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
.Lfunc_end4:
	.size	lossx_poisson_nll_kernel, .Lfunc_end4-lossx_poisson_nll_kernel
                                        ; -- End function
	.set lossx_poisson_nll_kernel.num_vgpr, 13
	.set lossx_poisson_nll_kernel.num_agpr, 0
	.set lossx_poisson_nll_kernel.numbered_sgpr, 8
	.set lossx_poisson_nll_kernel.num_named_barrier, 0
	.set lossx_poisson_nll_kernel.private_seg_size, 0
	.set lossx_poisson_nll_kernel.uses_vcc, 1
	.set lossx_poisson_nll_kernel.uses_flat_scratch, 0
	.set lossx_poisson_nll_kernel.has_dyn_sized_stack, 0
	.set lossx_poisson_nll_kernel.has_recursion, 0
	.set lossx_poisson_nll_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 632
; TotalNumSgprs: 10
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 13
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
	.protected	lossx_kl_div_kernel     ; -- Begin function lossx_kl_div_kernel
	.globl	lossx_kl_div_kernel
	.p2align	8
	.type	lossx_kl_div_kernel,@function
lossx_kl_div_kernel:                    ; @lossx_kl_div_kernel
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
	s_cbranch_execz .LBB5_4
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0
	s_mov_b32 s2, exec_lo
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_lt_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB5_3
; %bb.2:
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[4:5]
	s_mov_b32 s6, 0x55555780
	v_cndmask_b32_e64 v6, 0, 1, vcc_lo
	v_ldexp_f64 v[4:5], v[4:5], v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[4:5], 1.0
	v_add_f64 v[12:13], v[4:5], -1.0
	v_rcp_f64_e32 v[8:9], v[6:7]
	v_add_f64 v[14:15], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_mul_f64 v[16:17], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[10:11], v[6:7], -v[16:17]
	v_fma_f64 v[4:5], v[10:11], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[16:17], v[4:5]
	v_add_f64 v[14:15], v[12:13], -v[6:7]
	v_add_f64 v[16:17], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[4:5], v[16:17], -v[4:5]
	v_frexp_exp_i32_f64_e32 v16, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[14:15], v[4:5]
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[6:7]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0x3fe62e42
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_subrev_co_ci_u32_e64 v14, null, 0, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_co_u32 v22, vcc_lo, s4, v0
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_co_ci_u32_e64 v23, null, s5, v1, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[2:3]
	global_load_b64 v[22:23], v[22:23], off
	v_add_f64 v[10:11], v[12:13], v[8:9]
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_mul_f64 v[16:17], v[14:15], s[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[6:7], -v[16:17]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_fma_f64 v[8:9], v[14:15], s[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[10:11], v[4:5]
	v_add_f64 v[16:17], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[6:7], v[12:13]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[14:15], -v[6:7]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[14:15], -v[18:19]
	v_add_f64 v[10:11], v[12:13], -v[18:19]
	v_add_f64 v[12:13], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[12:13], -v[8:9]
	v_add_f64 v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[10:11]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_add_f64 v[16:17], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[4:5], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[22:23]
	v_mul_f64 v[4:5], v[2:3], v[4:5]
.LBB5_3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB5_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_kl_div_kernel
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
		.amdhsa_next_free_vgpr 24
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
		.amdhsa_inst_pref_size 9
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
	.size	lossx_kl_div_kernel, .Lfunc_end5-lossx_kl_div_kernel
                                        ; -- End function
	.set lossx_kl_div_kernel.num_vgpr, 24
	.set lossx_kl_div_kernel.num_agpr, 0
	.set lossx_kl_div_kernel.numbered_sgpr, 12
	.set lossx_kl_div_kernel.num_named_barrier, 0
	.set lossx_kl_div_kernel.private_seg_size, 0
	.set lossx_kl_div_kernel.uses_vcc, 1
	.set lossx_kl_div_kernel.uses_flat_scratch, 0
	.set lossx_kl_div_kernel.has_dyn_sized_stack, 0
	.set lossx_kl_div_kernel.has_recursion, 0
	.set lossx_kl_div_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1100
; TotalNumSgprs: 14
; NumVgprs: 24
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 24
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
	.protected	lossx_smooth_l1_kernel  ; -- Begin function lossx_smooth_l1_kernel
	.globl	lossx_smooth_l1_kernel
	.p2align	8
	.type	lossx_smooth_l1_kernel,@function
lossx_smooth_l1_kernel:                 ; @lossx_smooth_l1_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB6_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], v[2:3], -v[4:5]
                                        ; implicit-def: $vgpr2_vgpr3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_nlt_f64_e64 s4, |v[4:5]|, s[0:1]
	s_and_saveexec_b32 s5, s4
	s_xor_b32 s4, exec_lo, s5
	s_cbranch_execz .LBB6_3
; %bb.2:
	v_fma_f64 v[2:3], s[0:1], -0.5, |v[4:5]|
                                        ; implicit-def: $vgpr4_vgpr5
.LBB6_3:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB6_5
; %bb.4:
	v_mul_f64 v[2:3], v[4:5], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[2:3], v[4:5], v[2:3]
	v_div_scale_f64 v[4:5], null, s[0:1], s[0:1], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_div_scale_f64 v[8:9], vcc_lo, v[2:3], s[0:1], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	v_fma_f64 v[4:5], -v[4:5], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[10:11]
	v_div_fixup_f64 v[2:3], v[4:5], s[0:1], v[2:3]
.LBB6_5:
	s_or_b32 exec_lo, exec_lo, s4
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB6_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_smooth_l1_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 296
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
		.amdhsa_next_free_vgpr 12
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
.Lfunc_end6:
	.size	lossx_smooth_l1_kernel, .Lfunc_end6-lossx_smooth_l1_kernel
                                        ; -- End function
	.set lossx_smooth_l1_kernel.num_vgpr, 12
	.set lossx_smooth_l1_kernel.num_agpr, 0
	.set lossx_smooth_l1_kernel.numbered_sgpr, 8
	.set lossx_smooth_l1_kernel.num_named_barrier, 0
	.set lossx_smooth_l1_kernel.private_seg_size, 0
	.set lossx_smooth_l1_kernel.uses_vcc, 1
	.set lossx_smooth_l1_kernel.uses_flat_scratch, 0
	.set lossx_smooth_l1_kernel.has_dyn_sized_stack, 0
	.set lossx_smooth_l1_kernel.has_recursion, 0
	.set lossx_smooth_l1_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 372
; TotalNumSgprs: 10
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 12
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
	.protected	lossx_huber_kernel      ; -- Begin function lossx_huber_kernel
	.globl	lossx_huber_kernel
	.p2align	8
	.type	lossx_huber_kernel,@function
lossx_huber_kernel:                     ; @lossx_huber_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB7_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], v[2:3], -v[4:5]
                                        ; implicit-def: $vgpr2_vgpr3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_f64_e64 s4, |v[4:5]|, s[0:1]
	s_and_saveexec_b32 s5, s4
	s_xor_b32 s4, exec_lo, s5
	s_cbranch_execz .LBB7_3
; %bb.2:
	v_mul_f64 v[2:3], v[4:5], 0.5
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], v[4:5], v[2:3]
                                        ; implicit-def: $vgpr4_vgpr5
.LBB7_3:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB7_5
; %bb.4:
	v_fma_f64 v[2:3], s[0:1], -0.5, |v[4:5]|
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], s[0:1], v[2:3]
.LBB7_5:
	s_or_b32 exec_lo, exec_lo, s4
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB7_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_huber_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 296
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
.Lfunc_end7:
	.size	lossx_huber_kernel, .Lfunc_end7-lossx_huber_kernel
                                        ; -- End function
	.set lossx_huber_kernel.num_vgpr, 6
	.set lossx_huber_kernel.num_agpr, 0
	.set lossx_huber_kernel.numbered_sgpr, 8
	.set lossx_huber_kernel.num_named_barrier, 0
	.set lossx_huber_kernel.private_seg_size, 0
	.set lossx_huber_kernel.uses_vcc, 1
	.set lossx_huber_kernel.uses_flat_scratch, 0
	.set lossx_huber_kernel.has_dyn_sized_stack, 0
	.set lossx_huber_kernel.has_recursion, 0
	.set lossx_huber_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 280
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
	.protected	lossx_tweedie_kernel    ; -- Begin function lossx_tweedie_kernel
	.globl	lossx_tweedie_kernel
	.p2align	8
	.type	lossx_tweedie_kernel,@function
lossx_tweedie_kernel:                   ; @lossx_tweedie_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB8_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[12:13], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_mov_b32 s2, 0x55555555
	s_mov_b32 s3, 0x3fe55555
	s_mov_b32 s4, 0x968915a9
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s6, 0x4222de17
	s_mov_b32 s5, 0x3fba6564
	s_mov_b32 s7, 0x3fbdee67
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s8, v0
	v_add_co_ci_u32_e64 v3, null, s9, v1, vcc_lo
	v_add_f64 v[4:5], -s[0:1], 2.0
	global_load_b64 v[10:11], v[2:3], off
	v_add_f64 v[2:3], -s[0:1], 1.0
	s_waitcnt vmcnt(0)
	v_cmp_eq_f64_e32 vcc_lo, 1.0, v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v9, v3, 0x3ff00000, vcc_lo
	v_cndmask_b32_e64 v8, v2, 0, vcc_lo
	v_cndmask_b32_e64 v7, v5, 0x3ff00000, vcc_lo
	v_cndmask_b32_e64 v6, v4, 0, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[8:9]
	v_cmp_neq_f64_e64 s8, v[8:9], |v[8:9]|
	v_cmp_neq_f64_e64 s0, 0, v[6:7]
	v_cmp_neq_f64_e64 s9, v[6:7], |v[6:7]|
	v_cndmask_b32_e32 v13, 0x3ff00000, v11, vcc_lo
	v_cndmask_b32_e32 v12, 0, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v11, 0x3ff00000, v11, s0
	v_cndmask_b32_e64 v10, 0, v10, s0
	v_frexp_mant_f64_e64 v[14:15], |v[12:13]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_frexp_mant_f64_e64 v[16:17], |v[10:11]|
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e64 s0, s[2:3], v[16:17]
	v_cndmask_b32_e64 v18, 0, 1, vcc_lo
	v_cndmask_b32_e64 v19, 0, 1, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[14:15], v[14:15], v18
	v_ldexp_f64 v[16:17], v[16:17], v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[14:15], 1.0
	v_add_f64 v[30:31], v[14:15], -1.0
	v_add_f64 v[20:21], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[22:23], v[18:19]
	v_add_f64 v[34:35], v[18:19], -1.0
	v_rcp_f64_e32 v[24:25], v[20:21]
	v_add_f64 v[38:39], v[20:21], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[34:35]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[26:27], -v[18:19], v[22:23], 1.0
	v_fma_f64 v[28:29], -v[20:21], v[24:25], 1.0
	v_fma_f64 v[22:23], v[26:27], v[22:23], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[24:25], v[28:29], v[24:25], v[24:25]
	v_fma_f64 v[26:27], -v[18:19], v[22:23], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[28:29], -v[20:21], v[24:25], 1.0
	v_fma_f64 v[22:23], v[26:27], v[22:23], v[22:23]
	v_add_f64 v[26:27], v[16:17], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[24:25], v[28:29], v[24:25], v[24:25]
	v_add_f64 v[16:17], v[16:17], -v[38:39]
	v_mul_f64 v[28:29], v[30:31], v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[32:33], v[26:27], v[24:25]
	v_mul_f64 v[36:37], v[18:19], v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[40:41], v[20:21], v[32:33]
	v_fma_f64 v[18:19], v[28:29], v[18:19], -v[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[32:33], v[20:21], -v[40:41]
	v_fma_f64 v[14:15], v[28:29], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[32:33], v[16:17], v[20:21]
	v_add_f64 v[18:19], v[36:37], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[40:41], v[16:17]
	v_add_f64 v[34:35], v[30:31], -v[18:19]
	v_add_f64 v[36:37], v[18:19], -v[36:37]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[38:39], v[26:27], -v[20:21]
	v_add_f64 v[40:41], v[20:21], -v[40:41]
	v_add_f64 v[30:31], v[30:31], -v[34:35]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[36:37], -v[14:15]
	v_add_f64 v[26:27], v[26:27], -v[38:39]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[40:41], -v[16:17]
	v_add_f64 v[18:19], v[30:31], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[26:27], -v[20:21]
	v_add_f64 v[14:15], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], v[20:21]
	v_add_f64 v[14:15], v[34:35], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[38:39], v[16:17]
	v_mul_f64 v[14:15], v[22:23], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[16:17], v[24:25], v[16:17]
	v_add_f64 v[18:19], v[28:29], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[32:33], v[16:17]
	v_add_f64 v[22:23], v[18:19], -v[28:29]
	v_mul_f64 v[26:27], v[18:19], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[20:21], -v[32:33]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_mul_f64 v[22:23], v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	v_fma_f64 v[24:25], v[18:19], v[18:19], -v[26:27]
	v_add_f64 v[28:29], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[30:31], v[20:21], v[20:21], -v[22:23]
	v_add_f64 v[32:33], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[24:25], v[18:19], v[28:29], v[24:25]
	v_fma_f64 v[28:29], v[20:21], v[32:33], v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[30:31], v[26:27], v[24:25]
	v_add_f64 v[32:33], v[22:23], v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[34:35], v[30:31], s[6:7], s[4:5]
	v_add_f64 v[26:27], v[30:31], -v[26:27]
	v_fma_f64 v[36:37], v[32:33], s[6:7], s[4:5]
	s_mov_b32 s4, 0x3abe935a
	s_mov_b32 s5, 0x3fbe25e4
	v_add_f64 v[22:23], v[32:33], -v[22:23]
	v_mul_f64 v[44:45], v[18:19], v[30:31]
	v_mul_f64 v[48:49], v[20:21], v[32:33]
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0x3ff71547
	v_fma_f64 v[34:35], v[30:31], v[34:35], s[4:5]
	v_add_f64 v[24:25], v[24:25], -v[26:27]
	v_fma_f64 v[36:37], v[32:33], v[36:37], s[4:5]
	s_mov_b32 s4, 0x47e6c9c2
	s_mov_b32 s5, 0x3fc110ef
	v_add_f64 v[22:23], v[28:29], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[34:35], v[30:31], v[34:35], s[4:5]
	v_fma_f64 v[36:37], v[32:33], v[36:37], s[4:5]
	s_mov_b32 s4, 0xcfa74449
	s_mov_b32 s5, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[34:35], v[30:31], v[34:35], s[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[36:37], v[32:33], v[36:37], s[4:5]
	s_mov_b32 s4, 0x71bf3c30
	s_mov_b32 s5, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[34:35], v[30:31], v[34:35], s[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[36:37], v[32:33], v[36:37], s[4:5]
	s_mov_b32 s4, 0x1c7792ce
	s_mov_b32 s5, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[34:35], v[30:31], v[34:35], s[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[36:37], v[32:33], v[36:37], s[4:5]
	s_mov_b32 s4, 0x924920da
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[34:35], v[30:31], v[34:35], s[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[36:37], v[32:33], v[36:37], s[4:5]
	s_mov_b32 s4, 0x9999999c
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[34:35], v[30:31], v[34:35], s[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[36:37], v[32:33], v[36:37], s[4:5]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	v_mul_f64 v[38:39], v[30:31], v[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[40:41], v[32:33], v[36:37]
	v_fma_f64 v[26:27], v[30:31], v[34:35], -v[38:39]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[28:29], v[32:33], v[36:37], -v[40:41]
	v_fma_f64 v[26:27], v[24:25], v[34:35], v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[28:29], v[22:23], v[36:37], v[28:29]
	v_add_f64 v[34:35], v[38:39], v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[36:37], v[40:41], v[28:29]
	v_add_f64 v[42:43], v[34:35], s[2:3]
	v_add_f64 v[38:39], v[34:35], -v[38:39]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_add_f64 v[46:47], v[36:37], s[2:3]
	s_mov_b32 s3, 0xbfe55555
	v_add_f64 v[40:41], v[36:37], -v[40:41]
	v_add_f64 v[50:51], v[42:43], s[2:3]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[26:27], v[26:27], -v[38:39]
	v_fma_f64 v[38:39], v[30:31], v[18:19], -v[44:45]
	v_add_f64 v[52:53], v[46:47], s[2:3]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_add_f64 v[28:29], v[28:29], -v[40:41]
	v_fma_f64 v[40:41], v[32:33], v[20:21], -v[48:49]
	v_add_f64 v[34:35], v[34:35], -v[50:51]
	v_add_f64 v[26:27], v[26:27], s[2:3]
	v_fma_f64 v[30:31], v[30:31], v[14:15], v[38:39]
	v_add_f64 v[36:37], v[36:37], -v[52:53]
	v_ldexp_f64 v[14:15], v[14:15], 1
	v_add_f64 v[28:29], v[28:29], s[2:3]
	v_fma_f64 v[32:33], v[32:33], v[16:17], v[40:41]
	s_mov_b32 s2, 0xfefa39ef
	s_mov_b32 s3, 0x3fe62e42
	v_ldexp_f64 v[16:17], v[16:17], 1
	v_add_f64 v[26:27], v[26:27], v[34:35]
	v_fma_f64 v[24:25], v[24:25], v[18:19], v[30:31]
	v_ldexp_f64 v[18:19], v[18:19], 1
	v_add_f64 v[28:29], v[28:29], v[36:37]
	v_fma_f64 v[22:23], v[22:23], v[20:21], v[32:33]
	v_ldexp_f64 v[20:21], v[20:21], 1
	v_add_f64 v[30:31], v[42:43], v[26:27]
	v_add_f64 v[32:33], v[44:45], v[24:25]
	v_add_f64 v[34:35], v[46:47], v[28:29]
	v_add_f64 v[36:37], v[48:49], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[38:39], v[42:43], -v[30:31]
	v_mul_f64 v[40:41], v[32:33], v[30:31]
	v_add_f64 v[44:45], v[32:33], -v[44:45]
	v_add_f64 v[42:43], v[46:47], -v[34:35]
	v_mul_f64 v[46:47], v[36:37], v[34:35]
	v_add_f64 v[48:49], v[36:37], -v[48:49]
	v_add_f64 v[26:27], v[26:27], v[38:39]
	v_fma_f64 v[38:39], v[32:33], v[30:31], -v[40:41]
	v_add_f64 v[24:25], v[24:25], -v[44:45]
	v_add_f64 v[28:29], v[28:29], v[42:43]
	v_fma_f64 v[42:43], v[36:37], v[34:35], -v[46:47]
	v_add_f64 v[22:23], v[22:23], -v[48:49]
	v_fma_f64 v[26:27], v[32:33], v[26:27], v[38:39]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[28:29], v[36:37], v[28:29], v[42:43]
	v_fma_f64 v[24:25], v[24:25], v[30:31], v[26:27]
	v_frexp_exp_i32_f64_e32 v30, v[12:13]
	v_frexp_exp_i32_f64_e32 v31, v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[22:23], v[22:23], v[34:35], v[28:29]
	v_add_f64 v[26:27], v[40:41], v[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_subrev_co_ci_u32_e64 v30, null, 0, v30, vcc_lo
	v_subrev_co_ci_u32_e64 v36, null, 0, v31, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[28:29], v[46:47], v[22:23]
	v_cvt_f64_i32_e32 v[30:31], v30
	s_delay_alu instid0(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[36:37], v36
	v_add_f64 v[32:33], v[18:19], v[26:27]
	v_add_f64 v[34:35], v[26:27], -v[40:41]
	v_add_f64 v[38:39], v[20:21], v[28:29]
	v_add_f64 v[40:41], v[28:29], -v[46:47]
	v_mul_f64 v[42:43], v[30:31], s[2:3]
	v_add_f64 v[18:19], v[32:33], -v[18:19]
	v_add_f64 v[24:25], v[24:25], -v[34:35]
	v_mul_f64 v[34:35], v[36:37], s[2:3]
	v_add_f64 v[20:21], v[38:39], -v[20:21]
	v_add_f64 v[22:23], v[22:23], -v[40:41]
	v_fma_f64 v[40:41], v[30:31], s[2:3], -v[42:43]
	v_add_f64 v[18:19], v[26:27], -v[18:19]
	v_add_f64 v[14:15], v[14:15], v[24:25]
	v_fma_f64 v[24:25], v[36:37], s[2:3], -v[34:35]
	s_mov_b32 s3, 0xbfe62e42
	v_add_f64 v[20:21], v[28:29], -v[20:21]
	v_add_f64 v[16:17], v[16:17], v[22:23]
	v_fma_f64 v[22:23], v[30:31], s[4:5], v[40:41]
	v_add_f64 v[14:15], v[14:15], v[18:19]
	v_fma_f64 v[18:19], v[36:37], s[4:5], v[24:25]
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[16:17], v[20:21]
	v_add_f64 v[20:21], v[42:43], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[24:25], v[32:33], v[14:15]
	v_add_f64 v[26:27], v[34:35], v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[28:29], v[38:39], v[16:17]
	v_add_f64 v[42:43], v[20:21], -v[42:43]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[30:31], v[20:21], v[24:25]
	v_add_f64 v[32:33], v[24:25], -v[32:33]
	v_add_f64 v[34:35], v[26:27], -v[34:35]
	v_add_f64 v[36:37], v[26:27], v[28:29]
	v_add_f64 v[38:39], v[28:29], -v[38:39]
	v_add_f64 v[22:23], v[22:23], -v[42:43]
	v_add_f64 v[40:41], v[30:31], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[32:33]
	v_add_f64 v[18:19], v[18:19], -v[34:35]
	v_add_f64 v[44:45], v[36:37], -v[26:27]
	v_add_f64 v[16:17], v[16:17], -v[38:39]
	v_add_f64 v[46:47], v[30:31], -v[40:41]
	v_add_f64 v[24:25], v[24:25], -v[40:41]
	v_add_f64 v[32:33], v[22:23], v[14:15]
	v_add_f64 v[48:49], v[36:37], -v[44:45]
	v_add_f64 v[28:29], v[28:29], -v[44:45]
	v_add_f64 v[20:21], v[20:21], -v[46:47]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[26:27], -v[48:49]
	v_add_f64 v[20:21], v[24:25], v[20:21]
	v_add_f64 v[24:25], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[28:29], v[26:27]
	v_add_f64 v[28:29], v[32:33], -v[22:23]
	v_add_f64 v[20:21], v[32:33], v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[34:35], v[24:25], -v[18:19]
	v_add_f64 v[26:27], v[24:25], v[26:27]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[32:33], v[32:33], -v[28:29]
	v_add_f64 v[14:15], v[14:15], -v[28:29]
	v_add_f64 v[38:39], v[30:31], v[20:21]
	v_add_f64 v[24:25], v[24:25], -v[34:35]
	v_add_f64 v[16:17], v[16:17], -v[34:35]
	v_add_f64 v[40:41], v[36:37], v[26:27]
	v_add_f64 v[22:23], v[22:23], -v[32:33]
	v_add_f64 v[28:29], v[38:39], -v[30:31]
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[24:25], v[40:41], -v[36:37]
	v_add_f64 v[14:15], v[14:15], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[28:29]
	v_add_f64 v[16:17], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[26:27], -v[24:25]
	v_add_f64 v[14:15], v[14:15], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], v[18:19]
	v_add_f64 v[18:19], v[38:39], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[40:41], v[16:17]
	v_add_f64 v[22:23], v[18:19], -v[38:39]
	v_mul_f64 v[24:25], v[8:9], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[20:21], -v[40:41]
	v_mul_f64 v[28:29], v[6:7], v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[18:19], v[8:9], v[18:19], -v[24:25]
	v_cmp_class_f64_e64 vcc_lo, v[24:25], 0x204
	v_add_f64 v[16:17], v[16:17], -v[26:27]
	v_fma_f64 v[20:21], v[6:7], v[20:21], -v[28:29]
	v_cmp_class_f64_e64 s0, v[28:29], 0x204
	v_fma_f64 v[14:15], v[8:9], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[6:7], v[16:17], v[20:21]
	v_add_f64 v[18:19], v[24:25], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[28:29], v[16:17]
	v_dual_cndmask_b32 v23, v19, v25 :: v_dual_cndmask_b32 v22, v18, v24
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_cmp_lt_f64_e64 s14, |v[12:13]|, 1.0
	v_cmp_class_f64_e64 s15, v[12:13], 0x204
	v_cndmask_b32_e64 v27, v21, v29, s0
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[22:23]|
	v_cndmask_b32_e64 v26, v20, v28, s0
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[20:21], v[20:21], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[32:33], v[26:27], s[6:7]
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_xor_b32 s8, s8, s14
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rndne_f64_e32 v[32:33], v[32:33]
	v_cndmask_b32_e32 v15, 0, v15, vcc_lo
	v_mul_f64 v[30:31], v[22:23], s[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[36:37], v[32:33], s[2:3], v[26:27]
	v_rndne_f64_e32 v[30:31], v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[36:37], v[32:33], s[4:5], v[36:37]
	v_fma_f64 v[34:35], v[30:31], s[2:3], v[22:23]
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	v_cvt_i32_f64_e32 v42, v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[40:41], v[36:37], s[2:3], s[0:1]
	v_fma_f64 v[34:35], v[30:31], s[4:5], v[34:35]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[26:27]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[38:39], v[34:35], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_cmp_nlt_f64_e64 s3, 0x40900000, v[26:27]
	v_fma_f64 v[40:41], v[36:37], v[40:41], s[0:1]
	v_cmp_neq_f64_e64 s2, 0x7ff00000, |v[26:27]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[38:39], v[34:35], v[38:39], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	v_fma_f64 v[40:41], v[36:37], v[40:41], s[0:1]
	v_cndmask_b32_e64 v17, 0, v17, s2
	v_cndmask_b32_e64 v16, 0, v16, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[38:39], v[34:35], v[38:39], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[40:41], v[36:37], v[40:41], s[0:1]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[38:39], v[34:35], v[38:39], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[40:41], v[36:37], v[40:41], s[0:1]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[38:39], v[34:35], v[38:39], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[40:41], v[36:37], v[40:41], s[0:1]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[38:39], v[34:35], v[38:39], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[40:41], v[36:37], v[40:41], s[0:1]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[38:39], v[34:35], v[38:39], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[40:41], v[36:37], v[40:41], s[0:1]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[38:39], v[34:35], v[38:39], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[40:41], v[36:37], v[40:41], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[38:39], v[34:35], v[38:39], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[22:23]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[22:23]
	v_fma_f64 v[40:41], v[36:37], v[40:41], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[38:39], v[34:35], v[38:39], 1.0
	v_fma_f64 v[30:31], v[34:35], v[38:39], 1.0
	v_cvt_i32_f64_e32 v38, v[32:33]
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[32:33], v[36:37], v[40:41], 1.0
	v_mul_f64 v[34:35], v[8:9], 0.5
	v_trunc_f64_e32 v[36:37], v[6:7]
	v_ldexp_f64 v[24:25], v[30:31], v42
	v_mul_f64 v[30:31], v[6:7], 0.5
	v_ldexp_f64 v[28:29], v[32:33], v38
	v_trunc_f64_e32 v[32:33], v[8:9]
	v_trunc_f64_e32 v[38:39], v[34:35]
	v_cmp_eq_f64_e64 s6, v[36:37], v[6:7]
	v_cndmask_b32_e64 v20, 0x7ff00000, v25, s0
	v_trunc_f64_e32 v[18:19], v[30:31]
	v_cndmask_b32_e64 v22, 0x7ff00000, v29, s3
	v_cmp_eq_f64_e64 s5, v[32:33], v[8:9]
	v_cmp_neq_f64_e64 s7, v[38:39], v[34:35]
	v_cndmask_b32_e64 v21, 0, v20, s1
	v_cndmask_b32_e32 v14, 0, v14, vcc_lo
	s_and_b32 vcc_lo, s1, s0
	v_cndmask_b32_e64 v23, 0, v22, s4
	v_cndmask_b32_e32 v20, 0, v24, vcc_lo
	s_and_b32 vcc_lo, s4, s3
	v_cmp_gt_f64_e64 s3, 0, v[8:9]
	v_cndmask_b32_e32 v22, 0, v28, vcc_lo
	v_cmp_eq_f64_e64 s4, 0, v[12:13]
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[20:21]
	v_cmp_class_f64_e64 vcc_lo, v[20:21], 0x204
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[16:17], v[22:23], v[16:17], v[22:23]
	v_cmp_class_f64_e64 s1, v[22:23], 0x204
	v_cmp_neq_f64_e64 s0, v[18:19], v[30:31]
	s_and_b32 s2, s5, s7
	v_cmp_lt_f64_e64 s7, |v[10:11]|, 1.0
	v_cndmask_b32_e64 v18, 0x3ff00000, v13, s2
	s_xor_b32 s3, s3, s4
	v_dual_cndmask_b32 v14, v14, v20 :: v_dual_cndmask_b32 v15, v15, v21
	v_cndmask_b32_e64 v16, v16, v22, s1
	v_cndmask_b32_e64 v17, v17, v23, s1
	v_cmp_gt_f64_e64 s1, 0, v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_bfi_b32 v15, 0x7fffffff, v15, v18
	v_cndmask_b32_e64 v22, 0x7ff00000, 0, s8
	v_cndmask_b32_e64 v21, 0, v16, s6
	v_cmp_class_f64_e64 s8, v[10:11], 0x204
	v_cndmask_b32_e64 v18, 0x7ff80000, v15, s5
	s_and_b32 vcc_lo, s6, s0
	v_cmp_gt_f64_e64 s0, 0, v[12:13]
	v_cndmask_b32_e32 v19, 0x3ff00000, v11, vcc_lo
	s_xor_b32 s7, s9, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v23, 0x7ff00000, 0, s7
	v_cmp_neq_f64_e64 s7, |v[12:13]|, 1.0
	v_bfi_b32 v17, 0x7fffffff, v17, v19
	v_cndmask_b32_e64 v19, 0, v14, s5
	v_cmp_gt_f64_e64 s5, 0, v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v20, 0x7ff80000, v17, s6
	v_cmp_eq_f64_e64 s6, 0, v[10:11]
	v_cndmask_b32_e64 v16, v16, v21, s1
	v_cndmask_b32_e64 v17, v17, v20, s1
	v_cmp_class_f64_e64 s1, v[6:7], 0x204
	v_cndmask_b32_e32 v21, 0, v11, vcc_lo
	s_or_b32 vcc_lo, s4, s15
	v_cndmask_b32_e64 v14, v14, v19, s0
	v_cndmask_b32_e64 v15, v15, v18, s0
	v_cmp_class_f64_e64 s0, v[8:9], 0x204
	v_cndmask_b32_e64 v18, 0x7ff00000, 0, s3
	v_cndmask_b32_e64 v19, 0, v13, s2
	v_cndmask_b32_e64 v22, 0x3ff00000, v22, s7
	v_cmp_neq_f64_e64 s7, |v[10:11]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_bfi_b32 v18, 0x7fffffff, v18, v19
	s_xor_b32 s2, s5, s6
	v_cndmask_b32_e64 v20, 0x7ff00000, 0, s2
	s_or_b32 s2, s6, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_bfi_b32 v19, 0x7fffffff, v20, v21
	v_cndmask_b32_e64 v15, v15, v22, s0
	s_or_b32 s0, vcc_lo, s0
	v_cndmask_b32_e64 v14, v14, 0, s0
	s_or_b32 s0, s2, s1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v15, v15, v18, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[12:13], v[8:9]
	v_cndmask_b32_e64 v8, v16, 0, s0
	v_cmp_o_f64_e64 s0, v[10:11], v[6:7]
	v_cndmask_b32_e64 v23, 0x3ff00000, v23, s7
	v_cndmask_b32_e64 v17, v17, v23, s1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v17, v17, v19, s2
	v_cndmask_b32_e32 v6, 0, v14, vcc_lo
	v_cndmask_b32_e32 v7, 0x7ff80000, v15, vcc_lo
	v_cndmask_b32_e64 v8, 0, v8, s0
	v_cndmask_b32_e64 v9, 0x7ff80000, v17, s0
	v_add_co_u32 v18, vcc_lo, s10, v0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_scale_f64 v[10:11], null, v[2:3], v[2:3], v[6:7]
	v_div_scale_f64 v[12:13], null, v[4:5], v[4:5], v[8:9]
	v_add_co_ci_u32_e64 v19, null, s11, v1, vcc_lo
	v_div_scale_f64 v[24:25], vcc_lo, v[6:7], v[2:3], v[6:7]
	global_load_b64 v[18:19], v[18:19], off
	v_rcp_f64_e32 v[14:15], v[10:11]
	v_rcp_f64_e32 v[16:17], v[12:13]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[22:23], -v[12:13], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[14:15], v[20:21], v[14:15]
	v_fma_f64 v[16:17], v[16:17], v[22:23], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[22:23], -v[12:13], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[14:15], v[20:21], v[14:15]
	v_div_scale_f64 v[20:21], s0, v[8:9], v[4:5], v[8:9]
	v_fma_f64 v[16:17], v[16:17], v[22:23], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[22:23], v[24:25], v[14:15]
	v_mul_f64 v[26:27], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], -v[10:11], v[22:23], v[24:25]
	v_fma_f64 v[12:13], -v[12:13], v[26:27], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[10:11], v[10:11], v[14:15], v[22:23]
	s_mov_b32 vcc_lo, s0
	v_div_fmas_f64 v[12:13], v[12:13], v[16:17], v[26:27]
	v_add_co_u32 v0, vcc_lo, s12, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v1, null, s13, v1, vcc_lo
	v_div_fixup_f64 v[2:3], v[10:11], v[2:3], v[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[12:13], v[4:5], v[8:9]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[2:3], -v[18:19], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB8_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_tweedie_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 296
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
		.amdhsa_next_free_vgpr 54
		.amdhsa_next_free_sgpr 16
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
		.amdhsa_inst_pref_size 34
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
	.size	lossx_tweedie_kernel, .Lfunc_end8-lossx_tweedie_kernel
                                        ; -- End function
	.set lossx_tweedie_kernel.num_vgpr, 54
	.set lossx_tweedie_kernel.num_agpr, 0
	.set lossx_tweedie_kernel.numbered_sgpr, 16
	.set lossx_tweedie_kernel.num_named_barrier, 0
	.set lossx_tweedie_kernel.private_seg_size, 0
	.set lossx_tweedie_kernel.uses_vcc, 1
	.set lossx_tweedie_kernel.uses_flat_scratch, 0
	.set lossx_tweedie_kernel.has_dyn_sized_stack, 0
	.set lossx_tweedie_kernel.has_recursion, 0
	.set lossx_tweedie_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4252
; TotalNumSgprs: 18
; NumVgprs: 54
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 6
; NumSGPRsForWavesPerEU: 18
; NumVGPRsForWavesPerEU: 54
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
	.protected	lossx_quantile_kernel   ; -- Begin function lossx_quantile_kernel
	.globl	lossx_quantile_kernel
	.p2align	8
	.type	lossx_quantile_kernel,@function
lossx_quantile_kernel:                  ; @lossx_quantile_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB9_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v5, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], -v[4:5]
	v_add_f64 v[4:5], s[0:1], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[6:7], s[0:1], v[2:3]
	v_mul_f64 v[2:3], v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, v[6:7], v[2:3]
	v_dual_cndmask_b32 v3, v3, v7 :: v_dual_cndmask_b32 v2, v2, v6
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB9_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lossx_quantile_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 296
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
.Lfunc_end9:
	.size	lossx_quantile_kernel, .Lfunc_end9-lossx_quantile_kernel
                                        ; -- End function
	.set lossx_quantile_kernel.num_vgpr, 8
	.set lossx_quantile_kernel.num_agpr, 0
	.set lossx_quantile_kernel.numbered_sgpr, 8
	.set lossx_quantile_kernel.num_named_barrier, 0
	.set lossx_quantile_kernel.private_seg_size, 0
	.set lossx_quantile_kernel.uses_vcc, 1
	.set lossx_quantile_kernel.uses_flat_scratch, 0
	.set lossx_quantile_kernel.has_dyn_sized_stack, 0
	.set lossx_quantile_kernel.has_recursion, 0
	.set lossx_quantile_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 244
; TotalNumSgprs: 10
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_a1e89877511770a5,@object ; @__hip_cuid_a1e89877511770a5
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_a1e89877511770a5
__hip_cuid_a1e89877511770a5:
	.byte	0                               ; 0x0
	.size	__hip_cuid_a1e89877511770a5, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_a1e89877511770a5
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
    .name:           lossx_mse_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         lossx_mse_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     6
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
    .name:           lossx_mae_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         lossx_mae_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     6
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
    .name:           lossx_log_cosh_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         lossx_log_cosh_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     29
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
    .name:           lossx_bce_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         lossx_bce_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     42
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
    .name:           lossx_poisson_nll_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         lossx_poisson_nll_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     13
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
    .name:           lossx_kl_div_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         lossx_kl_div_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     24
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
        .size:           8
        .value_kind:     by_value
      - .offset:         40
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         44
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         48
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         52
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         54
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         56
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         58
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         60
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         62
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         96
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         104
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lossx_smooth_l1_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         lossx_smooth_l1_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     12
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
        .size:           8
        .value_kind:     by_value
      - .offset:         40
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         44
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         48
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         52
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         54
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         56
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         58
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         60
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         62
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         96
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         104
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lossx_huber_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         lossx_huber_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     6
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
        .size:           8
        .value_kind:     by_value
      - .offset:         40
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         44
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         48
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         52
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         54
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         56
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         58
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         60
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         62
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         96
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         104
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lossx_tweedie_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         lossx_tweedie_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     54
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
        .size:           8
        .value_kind:     by_value
      - .offset:         40
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         44
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         48
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         52
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         54
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         56
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         58
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         60
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         62
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         88
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         96
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         104
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lossx_quantile_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         lossx_quantile_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     8
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
