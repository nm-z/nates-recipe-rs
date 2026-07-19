	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	optimizerx_nesterov_kernel ; -- Begin function optimizerx_nesterov_kernel
	.globl	optimizerx_nesterov_kernel
	.p2align	8
	.type	optimizerx_nesterov_kernel,@function
optimizerx_nesterov_kernel:             ; @optimizerx_nesterov_kernel
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
	s_load_b64 s[0:1], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s8, v0
	v_add_co_ci_u32_e64 v5, null, s9, v1, vcc_lo
	global_load_b64 v[6:7], v[2:3], off
	global_load_b64 v[8:9], v[4:5], off
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_fma_f64 v[6:7], s[0:1], v[6:7], v[8:9]
	global_store_b64 v[2:3], v[6:7], off
	global_load_b64 v[2:3], v[4:5], off
	global_load_b64 v[4:5], v[0:1], off
	s_waitcnt vmcnt(1)
	v_fma_f64 v[2:3], s[0:1], v[6:7], v[2:3]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[2:3], -s[10:11], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel optimizerx_nesterov_kernel
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
		.amdhsa_next_free_vgpr 10
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
	.size	optimizerx_nesterov_kernel, .Lfunc_end0-optimizerx_nesterov_kernel
                                        ; -- End function
	.set optimizerx_nesterov_kernel.num_vgpr, 10
	.set optimizerx_nesterov_kernel.num_agpr, 0
	.set optimizerx_nesterov_kernel.numbered_sgpr, 12
	.set optimizerx_nesterov_kernel.num_named_barrier, 0
	.set optimizerx_nesterov_kernel.private_seg_size, 0
	.set optimizerx_nesterov_kernel.uses_vcc, 1
	.set optimizerx_nesterov_kernel.uses_flat_scratch, 0
	.set optimizerx_nesterov_kernel.has_dyn_sized_stack, 0
	.set optimizerx_nesterov_kernel.has_recursion, 0
	.set optimizerx_nesterov_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 244
; TotalNumSgprs: 14
; NumVgprs: 10
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 10
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
	.protected	optimizerx_adadelta_kernel ; -- Begin function optimizerx_adadelta_kernel
	.globl	optimizerx_adadelta_kernel
	.p2align	8
	.type	optimizerx_adadelta_kernel,@function
optimizerx_adadelta_kernel:             ; @optimizerx_adadelta_kernel
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
	s_cbranch_execz .LBB1_2
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b256 s[12:19], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s10, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s11, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	v_add_f64 v[8:9], -s[14:15], 1.0
	global_load_b64 v[6:7], v[4:5], off
	s_waitcnt vmcnt(1)
	v_mul_f64 v[10:11], v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[2:3], v[10:11]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[6:7], s[14:15], v[6:7], v[10:11]
	v_add_co_u32 v10, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v11, null, s9, v1, vcc_lo
	global_store_b64 v[4:5], v[6:7], off
	global_load_b64 v[4:5], v[10:11], off
	v_add_f64 v[6:7], s[16:17], v[6:7]
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[6:7]
	v_cndmask_b32_e64 v14, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v14
	s_waitcnt vmcnt(0)
	v_add_f64 v[12:13], s[16:17], v[4:5]
	v_cmp_gt_f64_e64 s0, 0x10000000, v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v15, 0, 0x100, s0
	v_ldexp_f64 v[12:13], v[12:13], v15
	v_rsq_f64_e32 v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_rsq_f64_e32 v[16:17], v[12:13]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[18:19], v[6:7], v[14:15]
	v_mul_f64 v[14:15], v[14:15], 0.5
	v_mul_f64 v[20:21], v[12:13], v[16:17]
	v_mul_f64 v[16:17], v[16:17], 0.5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], -v[14:15], v[18:19], 0.5
	v_fma_f64 v[24:25], -v[16:17], v[20:21], 0.5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[18:19], v[22:23], v[18:19]
	v_fma_f64 v[14:15], v[14:15], v[22:23], v[14:15]
	v_fma_f64 v[20:21], v[20:21], v[24:25], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], -v[18:19], v[18:19], v[6:7]
	v_fma_f64 v[16:17], v[16:17], v[24:25], v[16:17]
	v_fma_f64 v[24:25], -v[20:21], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[22:23], v[14:15], v[18:19]
	v_fma_f64 v[20:21], v[24:25], v[16:17], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], -v[18:19], v[18:19], v[6:7]
	v_fma_f64 v[24:25], -v[20:21], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[22:23], v[14:15], v[18:19]
	v_cndmask_b32_e64 v18, 0, 0xffffff80, vcc_lo
	v_cndmask_b32_e64 v19, 0, 0xffffff80, s0
	v_cmp_class_f64_e64 vcc_lo, v[6:7], 0x260
	v_cmp_class_f64_e64 s0, v[12:13], 0x260
	v_fma_f64 v[16:17], v[24:25], v[16:17], v[20:21]
	v_ldexp_f64 v[14:15], v[14:15], v18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[16:17], v[16:17], v19
	v_dual_cndmask_b32 v7, v15, v7 :: v_dual_cndmask_b32 v6, v14, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v12, v16, v12, s0
	v_cndmask_b32_e64 v13, -v17, -v13, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[14:15], null, v[6:7], v[6:7], v[12:13]
	v_rcp_f64_e32 v[16:17], v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	v_div_scale_f64 v[18:19], vcc_lo, v[12:13], v[6:7], v[12:13]
	v_mul_f64 v[20:21], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[14:15], v[20:21], v[18:19]
	v_div_fmas_f64 v[14:15], v[14:15], v[16:17], v[20:21]
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	v_div_fixup_f64 v[6:7], v[14:15], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[2:3], v[2:3], v[6:7]
	v_mul_f64 v[6:7], v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[2:3], v[6:7]
	v_fma_f64 v[4:5], s[14:15], v[4:5], v[6:7]
	global_store_b64 v[10:11], v[4:5], off
	global_load_b64 v[4:5], v[0:1], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[2:3], s[12:13], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel optimizerx_adadelta_kernel
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
		.amdhsa_next_free_vgpr 26
		.amdhsa_next_free_sgpr 20
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
		.amdhsa_inst_pref_size 7
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
	.size	optimizerx_adadelta_kernel, .Lfunc_end1-optimizerx_adadelta_kernel
                                        ; -- End function
	.set optimizerx_adadelta_kernel.num_vgpr, 26
	.set optimizerx_adadelta_kernel.num_agpr, 0
	.set optimizerx_adadelta_kernel.numbered_sgpr, 20
	.set optimizerx_adadelta_kernel.num_named_barrier, 0
	.set optimizerx_adadelta_kernel.private_seg_size, 0
	.set optimizerx_adadelta_kernel.uses_vcc, 1
	.set optimizerx_adadelta_kernel.uses_flat_scratch, 0
	.set optimizerx_adadelta_kernel.has_dyn_sized_stack, 0
	.set optimizerx_adadelta_kernel.has_recursion, 0
	.set optimizerx_adadelta_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 800
; TotalNumSgprs: 22
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 22
; NumVGPRsForWavesPerEU: 26
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
	.protected	optimizerx_radam_kernel ; -- Begin function optimizerx_radam_kernel
	.globl	optimizerx_radam_kernel
	.p2align	8
	.type	optimizerx_radam_kernel,@function
optimizerx_radam_kernel:                ; @optimizerx_radam_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x54
	s_load_b64 s[20:21], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s21, v2
	s_cbranch_execz .LBB2_6
; %bb.1:
	s_load_b512 s[4:19], s[0:1], 0x0
	v_cvt_f64_i32_e32 v[0:1], s20
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
	v_cmp_neq_f64_e64 vcc_lo, s[14:15], 1.0
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
	s_mov_b32 s61, 0x3e5ade15
	s_mov_b32 s62, 0x623fde64
	s_mov_b32 s63, 0x3ec71dee
	s_mov_b32 s64, 0x7c89e6b0
	s_mov_b32 s65, 0x3efa0199
	s_mov_b32 s66, 0x14761f6e
	s_mov_b32 s67, 0x3f2a01a0
	s_mov_b32 s68, 0x1852b7b0
	s_mov_b32 s69, 0x3f56c16c
	s_mov_b32 s72, 0x11122322
	s_mov_b32 s73, 0x3f811111
	s_mov_b32 s74, 0x555502a1
	s_mov_b32 s75, 0x3fa55555
	s_mov_b32 s76, 0x55555511
	s_mov_b32 s77, 0x3fc55555
	s_mov_b32 s78, 11
	s_mov_b32 s79, 0x3fe00000
	v_cmp_neq_f64_e64 s2, s[16:17], 1.0
	v_cndmask_b32_e32 v4, 0x3ff00000, v1, vcc_lo
	v_cndmask_b32_e32 v3, 0, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[3:4]
	s_and_b32 s0, vcc_lo, exec_lo
	s_cselect_b32 s21, s15, 0x3ff00000
	s_cselect_b32 s20, s14, 0
	v_frexp_mant_f64_e64 v[5:6], |s[20:21]|
	v_cmp_class_f64_e64 s3, s[20:21], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[22:23], v[5:6]
	v_cndmask_b32_e64 v7, 0, 1, vcc_lo
	v_ldexp_f64 v[5:6], v[5:6], v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], 1.0
	v_add_f64 v[13:14], v[5:6], -1.0
	v_rcp_f64_e32 v[9:10], v[7:8]
	v_add_f64 v[15:16], v[7:8], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	v_mul_f64 v[17:18], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[11:12], v[7:8], -v[17:18]
	v_fma_f64 v[5:6], v[11:12], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[17:18], v[5:6]
	v_add_f64 v[15:16], v[13:14], -v[7:8]
	v_add_f64 v[17:18], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_add_f64 v[5:6], v[17:18], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[5:6]
	v_mul_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_add_f64 v[9:10], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_fma_f64 v[9:10], v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[5:6], v[5:6]
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[26:27], s[24:25]
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_mul_f64 v[21:22], v[7:8], v[13:14]
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
	v_fma_f64 v[17:18], v[13:14], v[7:8], -v[21:22]
	v_add_f64 v[15:16], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], s[46:47]
	v_fma_f64 v[13:14], v[13:14], v[5:6], v[17:18]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[7:8], v[13:14]
	v_ldexp_f64 v[7:8], v[7:8], 1
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
	v_frexp_exp_i32_f64_e32 v13, s[20:21]
	v_add_f64 v[11:12], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v13, null, 0, v13, vcc_lo
	v_cvt_f64_i32_e32 v[13:14], v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[7:8], v[11:12]
	v_add_f64 v[17:18], v[11:12], -v[19:20]
	v_mul_f64 v[19:20], v[13:14], s[48:49]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[15:16], -v[7:8]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], s[48:49], -v[19:20]
	v_add_f64 v[7:8], v[11:12], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_fma_f64 v[9:10], v[13:14], s[52:53], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], v[5:6]
	v_add_f64 v[19:20], v[7:8], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[7:8], v[11:12]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[15:16], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[21:22]
	v_add_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[15:16], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[11:12]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	v_add_f64 v[17:18], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[11:12], v[17:18], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[17:18], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[7:8], -v[17:18]
	v_mul_f64 v[11:12], v[3:4], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[3:4], v[7:8], -v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	v_fma_f64 v[5:6], v[3:4], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_dual_cndmask_b32 v10, v8, v12 :: v_dual_cndmask_b32 v9, v7, v11
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[13:14], v[9:10], s[50:51]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[9:10]|
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[9:10]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[9:10]
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_rndne_f64_e32 v[13:14], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_mul_f64 v[7:8], v[3:4], 0.5
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	s_and_b32 vcc_lo, s1, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[13:14], s[54:55], v[9:10]
	v_cvt_i32_f64_e32 v19, v[13:14]
	v_fma_f64 v[15:16], v[13:14], s[56:57], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], s[60:61], s[58:59]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[62:63]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[64:65]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[66:67]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[68:69]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[72:73]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[74:75]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[76:77]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[78:79]
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	v_ldexp_f64 v[11:12], v[13:14], v19
	v_trunc_f64_e32 v[13:14], v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v12, 0x7ff00000, v12, s0
	v_cndmask_b32_e32 v9, 0, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v10, 0, v12, s1
	v_trunc_f64_e32 v[11:12], v[7:8]
	v_cmp_eq_f64_e64 s1, v[13:14], v[3:4]
	v_fma_f64 v[5:6], v[9:10], v[5:6], v[9:10]
	v_cmp_class_f64_e64 s0, v[9:10], 0x204
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e32 vcc_lo, v[11:12], v[7:8]
	v_cndmask_b32_e64 v7, v5, v9, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v5, v6, v10, s0
	v_cndmask_b32_e64 v6, 0x3ff00000, v1, s2
	v_cndmask_b32_e64 v8, 0, v7, s1
	s_and_b32 s33, s1, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[3:4]
	s_and_b32 s0, s33, exec_lo
	s_cselect_b32 s0, s21, 0x3ff00000
	s_delay_alu instid0(SALU_CYCLE_1)
	v_bfi_b32 v9, 0x7fffffff, v5, s0
	v_cndmask_b32_e64 v5, 0, v0, s2
	v_cmp_eq_f64_e64 s2, s[20:21], 0
	v_cmp_lt_f64_e64 s0, s[20:21], 0
	v_add_f64 v[0:1], v[0:1], v[0:1]
	v_cndmask_b32_e64 v3, 0x7ff80000, v9, s1
	v_cmp_neq_f64_e64 s1, 0, v[5:6]
	s_xor_b32 s70, vcc_lo, s2
	s_or_b32 vcc_lo, s2, s3
	v_cndmask_b32_e64 v27, v9, v3, s0
	v_cndmask_b32_e64 v28, v7, v8, s0
	s_and_b32 s0, s70, exec_lo
	s_cselect_b32 s3, 0, 0x7ff00000
	s_and_b32 s0, s33, exec_lo
	s_cselect_b32 s33, s21, 0
	s_and_b32 s0, s1, exec_lo
	s_cselect_b32 s71, s17, 0x3ff00000
	s_cselect_b32 s70, s16, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[3:4], |s[70:71]|
	v_cmp_gt_f64_e64 s0, s[22:23], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0, 1, s0
	v_ldexp_f64 v[3:4], v[3:4], v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[3:4], 1.0
	v_add_f64 v[13:14], v[3:4], -1.0
	v_rcp_f64_e32 v[9:10], v[7:8]
	v_add_f64 v[15:16], v[7:8], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	v_mul_f64 v[17:18], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[11:12], v[7:8], -v[17:18]
	v_fma_f64 v[3:4], v[11:12], v[3:4], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[17:18], v[3:4]
	v_add_f64 v[15:16], v[13:14], -v[7:8]
	v_add_f64 v[17:18], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_add_f64 v[3:4], v[17:18], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[13:14], -v[7:8]
	v_add_f64 v[3:4], v[3:4], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[15:16], v[3:4]
	v_mul_f64 v[3:4], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[3:4]
	v_add_f64 v[9:10], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], -v[9:10]
	v_fma_f64 v[9:10], v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[3:4], v[3:4]
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[26:27], s[24:25]
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_mul_f64 v[21:22], v[7:8], v[13:14]
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
	v_fma_f64 v[17:18], v[13:14], v[7:8], -v[21:22]
	v_add_f64 v[15:16], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], s[46:47]
	v_fma_f64 v[13:14], v[13:14], v[3:4], v[17:18]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[7:8], v[13:14]
	v_ldexp_f64 v[7:8], v[7:8], 1
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
	v_frexp_exp_i32_f64_e32 v13, s[70:71]
	v_add_f64 v[11:12], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v13, null, 0, v13, s0
	v_cvt_f64_i32_e32 v[13:14], v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[7:8], v[11:12]
	v_add_f64 v[17:18], v[11:12], -v[19:20]
	v_mul_f64 v[19:20], v[13:14], s[48:49]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[15:16], -v[7:8]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], s[48:49], -v[19:20]
	v_add_f64 v[7:8], v[11:12], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[3:4], v[3:4], v[9:10]
	v_fma_f64 v[9:10], v[13:14], s[52:53], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_add_f64 v[7:8], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], v[3:4]
	v_add_f64 v[19:20], v[7:8], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[7:8], v[11:12]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[13:14], -v[7:8]
	v_add_f64 v[3:4], v[3:4], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[15:16], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[21:22]
	v_add_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[15:16], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[11:12]
	v_add_f64 v[3:4], v[3:4], -v[11:12]
	v_add_f64 v[17:18], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[11:12], v[17:18], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_add_f64 v[7:8], v[17:18], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[7:8], -v[17:18]
	v_mul_f64 v[11:12], v[5:6], v[7:8]
	v_add_f64 v[3:4], v[3:4], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[5:6], v[7:8], -v[11:12]
	v_cmp_class_f64_e64 s0, v[11:12], 0x204
	v_fma_f64 v[7:8], v[5:6], v[3:4], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[11:12], v[7:8]
	v_cndmask_b32_e64 v14, v10, v12, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v13, v9, v11, s0
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	v_mul_f64 v[11:12], v[5:6], 0.5
	v_mul_f64 v[3:4], v[13:14], s[50:51]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[13:14]
	v_cmp_neq_f64_e64 s2, 0x7ff00000, |v[13:14]|
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_trunc_f64_e32 v[9:10], v[5:6]
	v_rndne_f64_e32 v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v8, 0, v8, s2
	v_cndmask_b32_e64 v7, 0, v7, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[15:16], s[54:55], v[13:14]
	v_fma_f64 v[17:18], v[15:16], s[56:57], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[17:18], s[60:61], s[58:59]
	v_fma_f64 v[3:4], v[17:18], v[3:4], s[62:63]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[17:18], v[3:4], s[64:65]
	v_fma_f64 v[3:4], v[17:18], v[3:4], s[66:67]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[17:18], v[3:4], s[68:69]
	v_fma_f64 v[3:4], v[17:18], v[3:4], s[72:73]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[19:20], v[17:18], v[3:4], s[74:75]
	v_ashrrev_i32_e32 v3, 31, v2
	v_cvt_i32_f64_e32 v4, v[15:16]
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v21, s0, s6, v2
	v_add_co_ci_u32_e64 v22, null, s7, v3, s0
	v_add_co_u32 v25, s0, s10, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v26, null, s11, v3, s0
	global_load_b64 v[23:24], v[21:22], off
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[13:14]
	v_trunc_f64_e32 v[13:14], v[11:12]
	global_load_b64 v[25:26], v[25:26], off
	v_cmp_eq_f64_e64 s10, s[70:71], 0
	v_cmp_class_f64_e64 s11, s[70:71], 0x204
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[76:77]
	v_cmp_neq_f64_e64 s2, v[13:14], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[78:79]
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	v_ldexp_f64 v[15:16], v[15:16], v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s6, v15
	v_cndmask_b32_e64 v4, 0x7ff00000, v16, s0
	s_and_b32 s0, s1, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_and_b32 s0, s0, exec_lo
	s_cselect_b32 s6, s6, 0
	v_cndmask_b32_e64 v16, 0, v4, s1
	v_mov_b32_e32 v15, s6
	v_cmp_eq_f64_e64 s1, v[9:10], v[5:6]
	v_mov_b32_e32 v10, s33
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[7:8], v[15:16], v[7:8], v[15:16]
	v_cmp_class_f64_e64 s0, v[15:16], 0x204
	v_bfi_b32 v10, 0x7fffffff, s3, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_readfirstlane_b32 s7, v7
	v_cndmask_b32_e64 v4, v8, v16, s0
	s_and_b32 s0, s0, exec_lo
	s_cselect_b32 s6, s6, s7
	s_and_b32 s7, s1, s2
	v_cmp_gt_f64_e64 s2, 0, v[5:6]
	s_and_b32 s0, s7, exec_lo
	s_cselect_b32 s0, s71, 0x3ff00000
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_bfi_b32 v8, 0x7fffffff, v4, s0
	v_cmp_lt_f64_e64 s0, s[70:71], 0
	v_add_f64 v[4:5], -s[14:15], 1.0
	v_cndmask_b32_e64 v9, 0x7ff80000, v8, s1
	s_and_b32 s1, s1, exec_lo
	s_cselect_b32 s1, s6, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v11, v8, v9, s0
	s_and_b32 s0, s0, exec_lo
	s_cselect_b32 s3, s1, s6
	s_xor_b32 s1, s2, s10
	s_or_b32 s0, s10, s11
	s_and_b32 s1, s1, exec_lo
	s_cselect_b32 s1, 0, 0x7ff00000
	s_and_b32 s2, s7, exec_lo
	s_cselect_b32 s2, s71, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_cndmask_b32 v9, v27, v10 :: v_dual_mov_b32 v12, s2
	v_cndmask_b32_e64 v8, v28, 0, vcc_lo
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], s[14:15], v[23:24]
	v_cmp_o_f64_e64 vcc_lo, s[20:21], s[20:21]
	v_bfi_b32 v10, 0x7fffffff, s1, v12
	v_cmp_o_f64_e64 s1, s[70:71], s[70:71]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v10, v11, v10, s0
	s_and_b32 s0, s0, exec_lo
	s_cselect_b32 s0, 0, s3
	s_waitcnt vmcnt(0)
	v_fma_f64 v[12:13], v[4:5], v[25:26], v[6:7]
	v_add_f64 v[4:5], -s[16:17], 1.0
	v_cndmask_b32_e64 v7, 0x7ff80000, v10, s1
	s_and_b32 s1, s1, exec_lo
	s_cselect_b32 s0, s0, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v6, s0
	v_add_f64 v[8:9], -v[8:9], 1.0
	v_mul_f64 v[16:17], v[0:1], v[6:7]
	v_add_f64 v[0:1], -v[6:7], 1.0
	v_div_scale_f64 v[10:11], null, v[4:5], v[4:5], 2.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v15, 0x7ff80000, v9, vcc_lo
	v_cndmask_b32_e32 v14, 0, v8, vcc_lo
	v_add_co_u32 v33, vcc_lo, s8, v2
	v_add_co_ci_u32_e64 v34, null, s9, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_div_scale_f64 v[8:9], null, v[14:15], v[14:15], v[12:13]
	v_div_scale_f64 v[23:24], null, v[0:1], v[0:1], v[16:17]
	global_load_b64 v[35:36], v[33:34], off
	v_rcp_f64_e32 v[18:19], v[10:11]
	v_rcp_f64_e32 v[6:7], v[8:9]
	v_rcp_f64_e32 v[29:30], v[23:24]
	s_delay_alu instid0(TRANS32_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], -v[10:11], v[18:19], 1.0
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[27:28], -v[8:9], v[6:7], 1.0
	v_fma_f64 v[18:19], v[18:19], v[31:32], v[18:19]
	v_fma_f64 v[6:7], v[6:7], v[27:28], v[6:7]
	v_fma_f64 v[27:28], -v[23:24], v[29:30], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[37:38], -v[10:11], v[18:19], 1.0
	v_fma_f64 v[31:32], -v[8:9], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[27:28], v[29:30], v[27:28], v[29:30]
	v_div_scale_f64 v[29:30], vcc_lo, v[12:13], v[14:15], v[12:13]
	v_fma_f64 v[18:19], v[18:19], v[37:38], v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[6:7], v[6:7], v[31:32], v[6:7]
	v_div_scale_f64 v[31:32], s0, 2.0, v[4:5], 2.0
	v_fma_f64 v[37:38], -v[23:24], v[27:28], 1.0
	v_mul_f64 v[39:40], v[29:30], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[41:42], v[31:32], v[18:19]
	v_fma_f64 v[27:28], v[27:28], v[37:38], v[27:28]
	v_div_scale_f64 v[37:38], s1, v[16:17], v[0:1], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[8:9], -v[8:9], v[39:40], v[29:30]
	v_fma_f64 v[10:11], -v[10:11], v[41:42], v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[29:30], v[37:38], v[27:28]
	v_div_fmas_f64 v[31:32], v[8:9], v[6:7], v[39:40]
	s_mov_b32 vcc_lo, s0
	s_mov_b32 s0, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_fmas_f64 v[6:7], v[10:11], v[18:19], v[41:42]
	v_fma_f64 v[8:9], -v[23:24], v[29:30], v[37:38]
	s_mov_b32 vcc_lo, s1
	s_waitcnt vmcnt(0)
	v_mul_f64 v[10:11], s[16:17], v[35:36]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_fixup_f64 v[6:7], v[6:7], v[4:5], 2.0
	v_div_fmas_f64 v[8:9], v[8:9], v[27:28], v[29:30]
	v_mul_f64 v[4:5], v[4:5], v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -1.0
	v_div_fixup_f64 v[8:9], v[8:9], v[0:1], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[25:26], v[4:5], v[10:11]
	v_div_fixup_f64 v[4:5], v[31:32], v[14:15], v[12:13]
	global_store_b64 v[21:22], v[12:13], off
	global_store_b64 v[33:34], v[10:11], off
	v_add_f64 v[8:9], v[6:7], -v[8:9]
	v_cmp_nlt_f64_e32 vcc_lo, 4.0, v[8:9]
	s_cbranch_vccz .LBB2_3
; %bb.2:
	v_mul_f64 v[12:13], s[12:13], v[4:5]
	s_and_not1_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccz .LBB2_4
	s_branch .LBB2_5
.LBB2_3:
                                        ; implicit-def: $vgpr12_vgpr13
.LBB2_4:
	v_add_f64 v[12:13], v[8:9], -4.0
	v_add_f64 v[14:15], v[8:9], -2.0
	v_add_f64 v[16:17], v[6:7], -4.0
	v_add_f64 v[18:19], v[6:7], -2.0
	v_add_f64 v[10:11], s[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[12:13], v[12:13], v[14:15]
	v_mul_f64 v[14:15], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_scale_f64 v[24:25], vcc_lo, v[0:1], v[10:11], v[0:1]
	v_mul_f64 v[6:7], v[6:7], v[12:13]
	v_div_scale_f64 v[12:13], null, v[10:11], v[10:11], v[0:1]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_rcp_f64_e32 v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[14:15], null, v[8:9], v[8:9], v[6:7]
	v_rcp_f64_e32 v[18:19], v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[12:13], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[16:17], v[20:21], v[16:17]
	v_fma_f64 v[22:23], -v[14:15], v[18:19], 1.0
	v_fma_f64 v[20:21], -v[12:13], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[18:19], v[22:23], v[18:19]
	v_fma_f64 v[16:17], v[16:17], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], -v[14:15], v[18:19], 1.0
	v_div_scale_f64 v[20:21], s0, v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[18:19], v[18:19], v[22:23], v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[22:23], v[24:25], v[16:17]
	v_mul_f64 v[26:27], v[20:21], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], -v[12:13], v[22:23], v[24:25]
	v_fma_f64 v[14:15], -v[14:15], v[26:27], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[12:13], v[12:13], v[16:17], v[22:23]
	s_mov_b32 vcc_lo, s0
	v_div_fmas_f64 v[14:15], v[14:15], v[18:19], v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fixup_f64 v[0:1], v[12:13], v[10:11], v[0:1]
	v_div_fixup_f64 v[6:7], v[14:15], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[6:7]
	s_and_b32 s0, vcc_lo, exec_lo
	s_cselect_b32 s0, 0x100, 0
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[0:1]
	v_ldexp_f64 v[6:7], v[6:7], s0
	s_cselect_b32 s0, 0xffffff80, 0
	v_cndmask_b32_e64 v10, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[8:9], v[6:7]
	v_ldexp_f64 v[0:1], v[0:1], v10
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[10:11], v[6:7], v[8:9]
	v_mul_f64 v[8:9], v[8:9], 0.5
	v_rsq_f64_e32 v[12:13], v[0:1]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[8:9], v[10:11], 0.5
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[16:17], v[0:1], v[12:13]
	v_mul_f64 v[12:13], v[12:13], 0.5
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], -v[12:13], v[16:17], 0.5
	v_fma_f64 v[14:15], -v[10:11], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	v_fma_f64 v[12:13], v[12:13], v[18:19], v[12:13]
	v_fma_f64 v[10:11], v[14:15], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], -v[16:17], v[16:17], v[0:1]
	v_fma_f64 v[18:19], -v[10:11], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[14:15], v[12:13], v[16:17]
	v_fma_f64 v[8:9], v[18:19], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], -v[14:15], v[14:15], v[0:1]
	v_ldexp_f64 v[8:9], v[8:9], s0
	v_cmp_class_f64_e64 s0, v[6:7], 0x260
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[14:15]
	v_cndmask_b32_e64 v7, v9, v7, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v6, v8, v6, s0
	v_cndmask_b32_e64 v8, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[0:1], 0x260
	v_mul_f64 v[6:7], s[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[8:9], v[10:11], v8
	v_mul_f64 v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v1, v9, v1 :: v_dual_cndmask_b32 v0, v8, v0
	v_mul_f64 v[12:13], v[4:5], v[0:1]
.LBB2_5:
	v_add_co_u32 v0, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s5, v3, vcc_lo
	global_load_b64 v[2:3], v[0:1], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], -v[12:13]
	global_store_b64 v[0:1], v[2:3], off
.LBB2_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel optimizerx_radam_kernel
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
		.amdhsa_next_free_vgpr 43
		.amdhsa_next_free_sgpr 80
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
		.amdhsa_inst_pref_size 41
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
	.size	optimizerx_radam_kernel, .Lfunc_end2-optimizerx_radam_kernel
                                        ; -- End function
	.set optimizerx_radam_kernel.num_vgpr, 43
	.set optimizerx_radam_kernel.num_agpr, 0
	.set optimizerx_radam_kernel.numbered_sgpr, 80
	.set optimizerx_radam_kernel.num_named_barrier, 0
	.set optimizerx_radam_kernel.private_seg_size, 0
	.set optimizerx_radam_kernel.uses_vcc, 1
	.set optimizerx_radam_kernel.uses_flat_scratch, 0
	.set optimizerx_radam_kernel.has_dyn_sized_stack, 0
	.set optimizerx_radam_kernel.has_recursion, 0
	.set optimizerx_radam_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 5212
; TotalNumSgprs: 82
; NumVgprs: 43
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 82
; NumVGPRsForWavesPerEU: 43
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
	.protected	optimizerx_lars_phase1_kernel ; -- Begin function optimizerx_lars_phase1_kernel
	.globl	optimizerx_lars_phase1_kernel
	.p2align	8
	.type	optimizerx_lars_phase1_kernel,@function
optimizerx_lars_phase1_kernel:          ; @optimizerx_lars_phase1_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x10
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[5:6], null, s2, s3, v[0:1]
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v5
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 3, v[5:6]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s4, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s5, v2, vcc_lo
	v_add_co_u32 v1, vcc_lo, s6, v1
	v_add_co_ci_u32_e64 v2, null, s7, v2, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	global_load_b64 v[5:6], v[1:2], off
	s_waitcnt vmcnt(1)
	v_mul_f64 v[1:2], v[3:4], v[3:4]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[3:4], v[5:6], v[5:6]
.LBB3_2:
	s_or_b32 exec_lo, exec_lo, s2
	v_mbcnt_lo_u32_b32 v9, -1, 0
	v_and_b32_e32 v0, 31, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_lshl_or_b32 v8, v9, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v9
	ds_bpermute_b32 v5, v8, v1
	ds_bpermute_b32 v6, v8, v2
	ds_bpermute_b32 v7, v8, v3
	ds_bpermute_b32 v8, v8, v4
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	v_cndmask_b32_e64 v5, 0, 8, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_cmp_gt_u32_e32 vcc_lo, 28, v9
	s_delay_alu instid0(VALU_DEP_3)
	v_add_lshl_u32 v8, v5, v9, 2
	ds_bpermute_b32 v5, v8, v1
	ds_bpermute_b32 v6, v8, v2
	ds_bpermute_b32 v7, v8, v3
	ds_bpermute_b32 v8, v8, v4
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	v_cndmask_b32_e64 v5, 0, 4, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_cmp_gt_u32_e32 vcc_lo, 30, v9
	s_delay_alu instid0(VALU_DEP_3)
	v_add_lshl_u32 v8, v5, v9, 2
	ds_bpermute_b32 v5, v8, v1
	ds_bpermute_b32 v6, v8, v2
	ds_bpermute_b32 v7, v8, v3
	ds_bpermute_b32 v8, v8, v4
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	v_cndmask_b32_e64 v5, 0, 2, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_cmp_ne_u32_e32 vcc_lo, 31, v9
	s_delay_alu instid0(VALU_DEP_3)
	v_add_lshl_u32 v8, v5, v9, 2
	ds_bpermute_b32 v5, v8, v1
	ds_bpermute_b32 v6, v8, v2
	ds_bpermute_b32 v7, v8, v3
	ds_bpermute_b32 v8, v8, v4
	s_waitcnt lgkmcnt(2)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[4:5], v[3:4], v[7:8]
	v_add_co_ci_u32_e64 v3, null, 0, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
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
	s_mov_b32 s2, exec_lo
.LBB3_4:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ctz_i32_b32 s3, s2
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s5, v1, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s4, v0, s3
	s_lshl_b32 s3, 1, s3
	s_and_not1_b32 s2, s2, s3
	v_add_f64 v[8:9], v[8:9], s[4:5]
	s_cmp_lg_u32 s2, 0
	s_cbranch_scc1 .LBB3_4
; %bb.5:
	s_load_b128 s[0:3], s[0:1], 0x18
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_mov_b32 s5, 0
	s_mov_b32 s4, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB3_9
; %bb.6:
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[6:7], s[0:1], 0x0
	v_mov_b32_e32 v10, 0
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v2, s6 :: v_dual_mov_b32 v3, s7
.LBB3_7:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[0:1], v[2:3], v[8:9]
	global_atomic_cmpswap_b64 v[0:1], v10, v[0:3], s[0:1] glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s5, vcc_lo, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s5
	s_cbranch_execnz .LBB3_7
; %bb.8:
	s_or_b32 exec_lo, exec_lo, s5
.LBB3_9:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[0:1], v[4:5], v[6:7]
	v_mov_b32_e32 v4, 0
	v_bfrev_b32_e32 v5, 1
	s_mov_b32 s0, exec_lo
.LBB3_10:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ctz_i32_b32 s1, s0
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s5, v1, s1
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s4, v0, s1
	s_lshl_b32 s1, 1, s1
	s_and_not1_b32 s0, s0, s1
	v_add_f64 v[4:5], v[4:5], s[4:5]
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
	global_load_b64 v[2:3], v6, s[2:3]
.LBB3_13:                               ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[4:5]
	global_atomic_cmpswap_b64 v[0:1], v6, v[0:3], s[2:3] glc
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
	.amdhsa_kernel optimizerx_lars_phase1_kernel
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
		.amdhsa_next_free_vgpr 11
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
		.amdhsa_inst_pref_size 8
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
	.size	optimizerx_lars_phase1_kernel, .Lfunc_end3-optimizerx_lars_phase1_kernel
                                        ; -- End function
	.set optimizerx_lars_phase1_kernel.num_vgpr, 11
	.set optimizerx_lars_phase1_kernel.num_agpr, 0
	.set optimizerx_lars_phase1_kernel.numbered_sgpr, 8
	.set optimizerx_lars_phase1_kernel.num_named_barrier, 0
	.set optimizerx_lars_phase1_kernel.private_seg_size, 0
	.set optimizerx_lars_phase1_kernel.uses_vcc, 1
	.set optimizerx_lars_phase1_kernel.uses_flat_scratch, 0
	.set optimizerx_lars_phase1_kernel.has_dyn_sized_stack, 0
	.set optimizerx_lars_phase1_kernel.has_recursion, 0
	.set optimizerx_lars_phase1_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 960
; TotalNumSgprs: 10
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 11
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
	.protected	optimizerx_lars_phase2_kernel ; -- Begin function optimizerx_lars_phase2_kernel
	.globl	optimizerx_lars_phase2_kernel
	.p2align	8
	.type	optimizerx_lars_phase2_kernel,@function
optimizerx_lars_phase2_kernel:          ; @optimizerx_lars_phase2_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x64
	s_load_b32 s4, s[0:1], 0x50
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB4_4
; %bb.1:
	s_clause 0x1
	s_load_b512 s[4:19], s[0:1], 0x0
	s_load_b128 s[0:3], s[0:1], 0x40
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0x3ff00000
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f64_e64 s20, s[18:19], 0
	v_cmp_ngt_f64_e64 s21, s[0:1], 0
	s_or_b32 s20, s20, s21
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 vcc_lo, exec_lo, s20
	s_cbranch_vccnz .LBB4_3
; %bb.2:
	v_cmp_gt_f64_e64 s20, 0x10000000, s[0:1]
	v_cmp_gt_f64_e64 s21, 0x10000000, s[18:19]
	s_and_b32 s20, s20, exec_lo
	s_cselect_b32 s20, 0x100, 0
	s_cselect_b32 s22, 0xffffff80, 0
	s_and_b32 s21, s21, exec_lo
	s_cselect_b32 s21, 0x100, 0
	v_ldexp_f64 v[2:3], s[0:1], s20
	v_ldexp_f64 v[4:5], s[18:19], s21
	s_cselect_b32 s0, 0xffffff80, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[6:7], v[2:3]
	v_rsq_f64_e32 v[8:9], v[4:5]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x260
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
	v_ldexp_f64 v[6:7], v[6:7], s22
	v_ldexp_f64 v[8:9], v[8:9], s0
	v_cmp_class_f64_e64 s0, v[4:5], 0x260
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v3, v7, v3 :: v_dual_cndmask_b32 v2, v6, v2
	v_cndmask_b32_e64 v5, v9, v5, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, v8, v4, s0
	v_fma_f64 v[2:3], s[14:15], v[4:5], v[2:3]
	v_mul_f64 v[4:5], s[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], s[2:3], v[2:3]
	v_div_scale_f64 v[6:7], null, v[2:3], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_div_scale_f64 v[10:11], vcc_lo, v[4:5], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[10:11], v[8:9]
	v_fma_f64 v[6:7], -v[6:7], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[12:13]
	v_div_fixup_f64 v[3:4], v[6:7], v[2:3], v[4:5]
.LBB4_3:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v5, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s9, v1, vcc_lo
	v_add_co_u32 v7, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v8, null, s5, v1, vcc_lo
	global_load_b64 v[5:6], v[5:6], off
	global_load_b64 v[9:10], v[7:8], off
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_load_b64 v[11:12], v[0:1], off
	s_waitcnt vmcnt(1)
	v_fma_f64 v[5:6], s[14:15], v[9:10], v[5:6]
	v_mul_f64 v[2:3], v[3:4], v[5:6]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[2:3], s[12:13], v[11:12], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
	global_load_b64 v[0:1], v[7:8], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[0:1], -s[10:11], v[2:3], v[0:1]
	global_store_b64 v[7:8], v[0:1], off
.LBB4_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel optimizerx_lars_phase2_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 344
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
		.amdhsa_next_free_sgpr 23
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
		.amdhsa_inst_pref_size 7
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
	.size	optimizerx_lars_phase2_kernel, .Lfunc_end4-optimizerx_lars_phase2_kernel
                                        ; -- End function
	.set optimizerx_lars_phase2_kernel.num_vgpr, 18
	.set optimizerx_lars_phase2_kernel.num_agpr, 0
	.set optimizerx_lars_phase2_kernel.numbered_sgpr, 23
	.set optimizerx_lars_phase2_kernel.num_named_barrier, 0
	.set optimizerx_lars_phase2_kernel.private_seg_size, 0
	.set optimizerx_lars_phase2_kernel.uses_vcc, 1
	.set optimizerx_lars_phase2_kernel.uses_flat_scratch, 0
	.set optimizerx_lars_phase2_kernel.has_dyn_sized_stack, 0
	.set optimizerx_lars_phase2_kernel.has_recursion, 0
	.set optimizerx_lars_phase2_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 772
; TotalNumSgprs: 25
; NumVgprs: 18
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 25
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_7d59ec952e2a2d26,@object ; @__hip_cuid_7d59ec952e2a2d26
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_7d59ec952e2a2d26
__hip_cuid_7d59ec952e2a2d26:
	.byte	0                               ; 0x0
	.size	__hip_cuid_7d59ec952e2a2d26, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_7d59ec952e2a2d26
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
        .size:           8
        .value_kind:     by_value
      - .offset:         32
        .size:           8
        .value_kind:     by_value
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
    .name:           optimizerx_nesterov_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         optimizerx_nesterov_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     10
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
      - .offset:         32
        .size:           8
        .value_kind:     by_value
      - .offset:         40
        .size:           8
        .value_kind:     by_value
      - .offset:         48
        .size:           8
        .value_kind:     by_value
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
    .name:           optimizerx_adadelta_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         optimizerx_adadelta_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     26
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
      - .offset:         32
        .size:           8
        .value_kind:     by_value
      - .offset:         40
        .size:           8
        .value_kind:     by_value
      - .offset:         48
        .size:           8
        .value_kind:     by_value
      - .offset:         56
        .size:           8
        .value_kind:     by_value
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
    .name:           optimizerx_radam_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     82
    .sgpr_spill_count: 0
    .symbol:         optimizerx_radam_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     43
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
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
    .name:           optimizerx_lars_phase1_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         optimizerx_lars_phase1_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     11
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
        .size:           8
        .value_kind:     by_value
      - .offset:         32
        .size:           8
        .value_kind:     by_value
      - .offset:         40
        .size:           8
        .value_kind:     by_value
      - .offset:         48
        .size:           8
        .value_kind:     by_value
      - .offset:         56
        .size:           8
        .value_kind:     by_value
      - .offset:         64
        .size:           8
        .value_kind:     by_value
      - .offset:         72
        .size:           8
        .value_kind:     by_value
      - .offset:         80
        .size:           4
        .value_kind:     by_value
      - .offset:         88
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         92
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         96
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         100
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         102
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         104
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         106
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         108
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         110
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         136
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         144
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         152
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 344
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           optimizerx_lars_phase2_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     25
    .sgpr_spill_count: 0
    .symbol:         optimizerx_lars_phase2_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     18
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
