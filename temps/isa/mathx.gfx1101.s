	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	mx_square_kernel        ; -- Begin function mx_square_kernel
	.globl	mx_square_kernel
	.p2align	8
	.type	mx_square_kernel,@function
mx_square_kernel:                       ; @mx_square_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB0_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[2:3], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_square_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_sgpr 5
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
	.size	mx_square_kernel, .Lfunc_end0-mx_square_kernel
                                        ; -- End function
	.set mx_square_kernel.num_vgpr, 4
	.set mx_square_kernel.num_agpr, 0
	.set mx_square_kernel.numbered_sgpr, 5
	.set mx_square_kernel.num_named_barrier, 0
	.set mx_square_kernel.private_seg_size, 0
	.set mx_square_kernel.uses_vcc, 1
	.set mx_square_kernel.uses_flat_scratch, 0
	.set mx_square_kernel.has_dyn_sized_stack, 0
	.set mx_square_kernel.has_recursion, 0
	.set mx_square_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 156
; TotalNumSgprs: 7
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	mx_exp2_kernel          ; -- Begin function mx_exp2_kernel
	.globl	mx_exp2_kernel
	.p2align	8
	.type	mx_exp2_kernel,@function
mx_exp2_kernel:                         ; @mx_exp2_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_rndne_f64_e32 v[4:5], v[2:3]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[2:3], -v[4:5]
	v_cvt_i32_f64_e32 v10, v[4:5]
	v_mul_f64 v[8:9], v[6:7], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[6:7], s[0:1], v[8:9]
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
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_cndmask_b32_e64 v3, 0, v5, s0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_exp2_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_sgpr 6
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
	.size	mx_exp2_kernel, .Lfunc_end1-mx_exp2_kernel
                                        ; -- End function
	.set mx_exp2_kernel.num_vgpr, 11
	.set mx_exp2_kernel.num_agpr, 0
	.set mx_exp2_kernel.numbered_sgpr, 6
	.set mx_exp2_kernel.num_named_barrier, 0
	.set mx_exp2_kernel.private_seg_size, 0
	.set mx_exp2_kernel.uses_vcc, 1
	.set mx_exp2_kernel.uses_flat_scratch, 0
	.set mx_exp2_kernel.has_dyn_sized_stack, 0
	.set mx_exp2_kernel.has_recursion, 0
	.set mx_exp2_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 564
; TotalNumSgprs: 8
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 8
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
	.protected	mx_log2_kernel          ; -- Begin function mx_log2_kernel
	.globl	mx_log2_kernel
	.p2align	8
	.type	mx_log2_kernel,@function
mx_log2_kernel:                         ; @mx_log2_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s6, 0xbf559e2b
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s7, 0x3fc3ab76
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[4:5]
	s_mov_b32 s0, 0x55555780
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[14:15], v[4:5]
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[6:7], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[0:1]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_add_f64 v[8:9], v[6:7], -v[10:11]
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	v_fma_f64 v[8:9], v[6:7], s[0:1], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[4:5], v[4:5], s[0:1], v[8:9]
	v_frexp_exp_i32_f64_e32 v8, v[2:3]
	s_mov_b32 s0, 0xffda0d24
	s_mov_b32 s1, 0x3c7777d0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], s[0:1], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_co_ci_u32_e64 v6, null, 0, v8, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_cvt_f64_i32_e32 v[6:7], v6
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[10:11], v[4:5]
	v_add_f64 v[12:13], v[8:9], v[6:7]
	v_add_f64 v[10:11], v[8:9], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[12:13]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[6:7], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[12:13], v[4:5]
	v_dual_cndmask_b32 v4, v4, v2 :: v_dual_cndmask_b32 v5, v5, v3
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v5, 0x7ff80000, v5, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0xfff00000, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_log2_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
.Lfunc_end2:
	.size	mx_log2_kernel, .Lfunc_end2-mx_log2_kernel
                                        ; -- End function
	.set mx_log2_kernel.num_vgpr, 18
	.set mx_log2_kernel.num_agpr, 0
	.set mx_log2_kernel.numbered_sgpr, 8
	.set mx_log2_kernel.num_named_barrier, 0
	.set mx_log2_kernel.private_seg_size, 0
	.set mx_log2_kernel.uses_vcc, 1
	.set mx_log2_kernel.uses_flat_scratch, 0
	.set mx_log2_kernel.has_dyn_sized_stack, 0
	.set mx_log2_kernel.has_recursion, 0
	.set mx_log2_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 976
; TotalNumSgprs: 10
; NumVgprs: 18
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
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
	.protected	mx_log10_kernel         ; -- Begin function mx_log10_kernel
	.globl	mx_log10_kernel
	.p2align	8
	.type	mx_log10_kernel,@function
mx_log10_kernel:                        ; @mx_log10_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s6, 0xbf559e2b
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s7, 0x3fc3ab76
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[4:5]
	s_mov_b32 s0, 0x55555780
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[14:15], v[4:5]
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[6:7], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x509f79ff
	s_mov_b32 s5, 0x3fd34413
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[0:1]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s0, 0x1526e50e
	s_mov_b32 s1, 0x3fdbcb7b
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_frexp_exp_i32_f64_e32 v8, v[2:3]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_cvt_f64_i32_e32 v[8:9], v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_mul_f64 v[14:15], v[8:9], s[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[6:7], -v[10:11]
	v_mul_f64 v[12:13], v[6:7], s[0:1]
	v_fma_f64 v[16:17], v[8:9], s[4:5], -v[14:15]
	s_mov_b32 s4, 0xbaaafad3
	s_mov_b32 s5, 0x3c695355
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_fma_f64 v[10:11], v[6:7], s[0:1], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[4:5], v[4:5], s[0:1], v[10:11]
	s_mov_b32 s0, 0xa994fd21
	s_mov_b32 s1, 0xbc49dc1d
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[8:9], s[0:1], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[6:7], s[4:5], v[4:5]
	v_add_f64 v[6:7], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[12:13], v[4:5]
	v_add_f64 v[14:15], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[6:7], v[10:11]
	v_add_f64 v[12:13], v[10:11], -v[12:13]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[16:17], -v[6:7]
	v_add_f64 v[4:5], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[16:17], -v[18:19]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
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
	v_add_f64 v[14:15], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[10:11], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[4:5], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v4, v4, v2 :: v_dual_cndmask_b32 v5, v5, v3
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v5, 0x7ff80000, v5, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v5, 0xfff00000, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_log10_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_vgpr 22
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
		.amdhsa_inst_pref_size 10
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
	.size	mx_log10_kernel, .Lfunc_end3-mx_log10_kernel
                                        ; -- End function
	.set mx_log10_kernel.num_vgpr, 22
	.set mx_log10_kernel.num_agpr, 0
	.set mx_log10_kernel.numbered_sgpr, 8
	.set mx_log10_kernel.num_named_barrier, 0
	.set mx_log10_kernel.private_seg_size, 0
	.set mx_log10_kernel.uses_vcc, 1
	.set mx_log10_kernel.uses_flat_scratch, 0
	.set mx_log10_kernel.has_dyn_sized_stack, 0
	.set mx_log10_kernel.has_recursion, 0
	.set mx_log10_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1168
; TotalNumSgprs: 10
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 22
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
	.protected	mx_cbrt_kernel          ; -- Begin function mx_cbrt_kernel
	.globl	mx_cbrt_kernel
	.p2align	8
	.type	mx_cbrt_kernel,@function
mx_cbrt_kernel:                         ; @mx_cbrt_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB4_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_frexp_exp_i32_f64_e32 v4, v[2:3]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x198
	v_cvt_f32_i32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, 0x3eaaaaab, v4
	v_rndne_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_i32_f32_e32 v16, v4
	v_mul_lo_u32 v4, v16, -3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], |v[2:3]|, v4
	v_cvt_f32_f64_e32 v6, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_log_f32_e32 v6, v6
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v6, 0x3eaaaaab, v6
	v_exp_f32_e32 v6, v6
	s_waitcnt_depctr 0xfff
	v_cvt_f64_f32_e32 v[6:7], v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	v_add_f64 v[10:11], v[6:7], v[6:7]
	v_fma_f64 v[10:11], v[10:11], v[8:9], v[4:5]
	v_fma_f64 v[4:5], -v[6:7], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[12:13], v[10:11]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[8:9], v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[4:5], v[8:9]
	v_fma_f64 v[4:5], -v[10:11], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[4:5], v[8:9], v[12:13]
	v_fma_f64 v[4:5], v[6:7], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v16
	v_bfi_b32 v5, 0x7fffffff, v5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v5
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_cbrt_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_vgpr 17
		.amdhsa_next_free_sgpr 5
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
		.amdhsa_inst_pref_size 4
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
	.size	mx_cbrt_kernel, .Lfunc_end4-mx_cbrt_kernel
                                        ; -- End function
	.set mx_cbrt_kernel.num_vgpr, 17
	.set mx_cbrt_kernel.num_agpr, 0
	.set mx_cbrt_kernel.numbered_sgpr, 5
	.set mx_cbrt_kernel.num_named_barrier, 0
	.set mx_cbrt_kernel.private_seg_size, 0
	.set mx_cbrt_kernel.uses_vcc, 1
	.set mx_cbrt_kernel.uses_flat_scratch, 0
	.set mx_cbrt_kernel.has_dyn_sized_stack, 0
	.set mx_cbrt_kernel.has_recursion, 0
	.set mx_cbrt_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 408
; TotalNumSgprs: 7
; NumVgprs: 17
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 7
; NumVGPRsForWavesPerEU: 17
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
	.protected	mx_sinh_kernel          ; -- Begin function mx_sinh_kernel
	.globl	mx_sinh_kernel
	.p2align	8
	.type	mx_sinh_kernel,@function
mx_sinh_kernel:                         ; @mx_sinh_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB5_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s5, 0x3fe62e42
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0xbfe62e42
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s4, s0
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], |v[2:3]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[4:5], -|v[2:3]|
	v_add_f64 v[8:9], v[6:7], -v[4:5]
	v_add_f64 v[6:7], v[6:7], s[4:5]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], |v[2:3]|, v[8:9]
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], s[4:5]
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0x3ff71547
	v_add_f64 v[8:9], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[8:9], s[4:5]
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	v_rndne_f64_e32 v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[6:7], v[4:5]
	v_fma_f64 v[6:7], v[10:11], s[0:1], v[8:9]
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	v_add_f64 v[12:13], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_mul_f64 v[6:7], v[10:11], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[8:9], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[8:9], v[6:7]
	v_add_f64 v[14:15], v[14:15], -v[8:9]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[14:15]
	v_add_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[12:13], v[12:13], -v[6:7]
	v_mul_f64 v[14:15], v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[12:13]
	v_fma_f64 v[12:13], v[6:7], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	v_add_f64 v[16:17], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	v_fma_f64 v[12:13], v[6:7], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	v_add_f64 v[16:17], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x8fb9f87e
	s_mov_b32 s1, 0x408633ce
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_nge_f64_e64 vcc_lo, |v[2:3]|, s[0:1]
	v_mul_f64 v[18:19], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[8:9], -v[18:19]
	v_fma_f64 v[8:9], v[12:13], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], v[8:9]
	v_add_f64 v[14:15], v[6:7], v[12:13]
	v_add_f64 v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[14:15], -v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[16:17]
	v_cvt_i32_f64_e32 v16, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	v_add_f64 v[14:15], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[6:7], v16
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_rcp_f64_e32 v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_ldexp_f64 v[4:5], v[4:5], v16
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[12:13], v[12:13]
	v_mul_f64 v[6:7], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[8:9], v[10:11], -v[6:7]
	v_fma_f64 v[12:13], v[8:9], v[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[6:7], v[12:13]
	v_add_f64 v[16:17], -v[14:15], 1.0
	v_add_f64 v[6:7], v[14:15], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], -v[16:17], 1.0
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], -v[14:15]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], v[6:7]
	v_mul_f64 v[14:15], v[8:9], v[12:13]
	v_add_f64 v[16:17], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[10:11], v[14:15]
	v_add_f64 v[6:7], v[6:7], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[14:15], v[10:11], -v[18:19]
	v_fma_f64 v[20:21], v[14:15], v[4:5], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[18:19], v[20:21]
	v_add_f64 v[24:25], v[12:13], -v[22:23]
	v_add_f64 v[16:17], v[22:23], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[24:25]
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[16:17], v[6:7]
	v_add_f64 v[16:17], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[24:25], v[6:7]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[6:7], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_ldexp_f64 v[14:15], v[8:9], -2
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], -v[14:15]
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_ldexp_f64 v[6:7], v[6:7], -2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[10:11], -v[14:15]
	v_add_f64 v[4:5], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_and_b32_e32 v6, 0x7fffffff, v3
	v_add_f64 v[4:5], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v2, v4, v2, vcc_lo
	v_cndmask_b32_e32 v4, v5, v6, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_bfi_b32 v3, 0x7fffffff, v4, v3
	global_store_b64 v[0:1], v[2:3], off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_sinh_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_sgpr 6
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
		.amdhsa_inst_pref_size 13
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
	.size	mx_sinh_kernel, .Lfunc_end5-mx_sinh_kernel
                                        ; -- End function
	.set mx_sinh_kernel.num_vgpr, 26
	.set mx_sinh_kernel.num_agpr, 0
	.set mx_sinh_kernel.numbered_sgpr, 6
	.set mx_sinh_kernel.num_named_barrier, 0
	.set mx_sinh_kernel.private_seg_size, 0
	.set mx_sinh_kernel.uses_vcc, 1
	.set mx_sinh_kernel.uses_flat_scratch, 0
	.set mx_sinh_kernel.has_dyn_sized_stack, 0
	.set mx_sinh_kernel.has_recursion, 0
	.set mx_sinh_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1644
; TotalNumSgprs: 8
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 8
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
	.protected	mx_cosh_kernel          ; -- Begin function mx_cosh_kernel
	.globl	mx_cosh_kernel
	.p2align	8
	.type	mx_cosh_kernel,@function
mx_cosh_kernel:                         ; @mx_cosh_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB6_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s5, 0x3fe62e42
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0xbfe62e42
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s4, s0
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], |v[2:3]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[4:5], -|v[2:3]|
	v_add_f64 v[8:9], v[6:7], -v[4:5]
	v_add_f64 v[6:7], v[6:7], s[4:5]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], |v[2:3]|, v[8:9]
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], s[4:5]
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0x3ff71547
	v_add_f64 v[8:9], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[8:9], s[4:5]
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	v_rndne_f64_e32 v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[6:7], v[4:5]
	v_fma_f64 v[6:7], v[10:11], s[0:1], v[8:9]
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	v_add_f64 v[12:13], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_mul_f64 v[6:7], v[10:11], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[8:9], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[8:9], v[6:7]
	v_add_f64 v[14:15], v[14:15], -v[8:9]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[14:15]
	v_add_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[12:13], v[12:13], -v[6:7]
	v_mul_f64 v[14:15], v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[12:13]
	v_fma_f64 v[12:13], v[6:7], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	v_add_f64 v[16:17], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	v_fma_f64 v[12:13], v[6:7], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	v_add_f64 v[16:17], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x8fb9f87e
	s_mov_b32 s1, 0x408633ce
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_nge_f64_e64 vcc_lo, |v[2:3]|, s[0:1]
	v_mul_f64 v[18:19], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[8:9], -v[18:19]
	v_fma_f64 v[8:9], v[12:13], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], v[8:9]
	v_add_f64 v[14:15], v[6:7], v[12:13]
	v_add_f64 v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[14:15], -v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[16:17]
	v_cvt_i32_f64_e32 v16, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	v_add_f64 v[14:15], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[6:7], v16
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_rcp_f64_e32 v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_ldexp_f64 v[4:5], v[4:5], v16
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[12:13], v[12:13]
	v_mul_f64 v[6:7], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[8:9], v[10:11], -v[6:7]
	v_fma_f64 v[12:13], v[8:9], v[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[6:7], v[12:13]
	v_add_f64 v[16:17], -v[14:15], 1.0
	v_add_f64 v[6:7], v[14:15], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], -v[16:17], 1.0
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], -v[14:15]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], v[6:7]
	v_mul_f64 v[14:15], v[8:9], v[12:13]
	v_add_f64 v[16:17], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[10:11], v[14:15]
	v_add_f64 v[6:7], v[6:7], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[14:15], v[10:11], -v[18:19]
	v_fma_f64 v[20:21], v[14:15], v[4:5], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[18:19], v[20:21]
	v_add_f64 v[24:25], v[12:13], -v[22:23]
	v_add_f64 v[16:17], v[22:23], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[24:25]
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[16:17], v[6:7]
	v_add_f64 v[16:17], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[24:25], v[6:7]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[6:7], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_ldexp_f64 v[14:15], v[8:9], -2
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[10:11], v[14:15]
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[12:13], -v[10:11]
	v_ldexp_f64 v[6:7], v[6:7], -2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[4:5], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, 0x7ff00000, v5, vcc_lo
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_cosh_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_sgpr 6
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
		.amdhsa_inst_pref_size 13
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
	.size	mx_cosh_kernel, .Lfunc_end6-mx_cosh_kernel
                                        ; -- End function
	.set mx_cosh_kernel.num_vgpr, 26
	.set mx_cosh_kernel.num_agpr, 0
	.set mx_cosh_kernel.numbered_sgpr, 6
	.set mx_cosh_kernel.num_named_barrier, 0
	.set mx_cosh_kernel.private_seg_size, 0
	.set mx_cosh_kernel.uses_vcc, 1
	.set mx_cosh_kernel.uses_flat_scratch, 0
	.set mx_cosh_kernel.has_dyn_sized_stack, 0
	.set mx_cosh_kernel.has_recursion, 0
	.set mx_cosh_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1600
; TotalNumSgprs: 8
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 8
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
	.protected	mx_asin_kernel          ; -- Begin function mx_asin_kernel
	.globl	mx_asin_kernel
	.p2align	8
	.type	mx_asin_kernel,@function
mx_asin_kernel:                         ; @mx_asin_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB7_4
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0x9fea6a70
	s_mov_b32 s5, 0x3fa05985
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x78a05eaf
	s_mov_b32 s1, 0xbf90a5a3
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[4:5], |v[2:3]|, -0.5, 0.5
	v_mul_f64 v[6:7], v[2:3], v[2:3]
	v_cmp_ge_f64_e64 vcc_lo, |v[2:3]|, 0.5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v7, v7, v5 :: v_dual_cndmask_b32 v6, v6, v4
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0x37024d6a
	s_mov_b32 s1, 0x3f940521
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x98a70509
	s_mov_b32 s1, 0x3f7ab3a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xa300c8d2
	s_mov_b32 s1, 0x3f88ed60
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x4b77012b
	s_mov_b32 s1, 0x3f8c6fa8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11dccb70
	s_mov_b32 s1, 0x3f91c6c1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xa0adacf
	s_mov_b32 s1, 0x3f96e89f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xc668963f
	s_mov_b32 s1, 0x3f9f1c72
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xb41ce4bd
	s_mov_b32 s1, 0x3fa6db6d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3336fd5b
	s_mov_b32 s1, 0x3fb33333
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555380
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], |v[2:3]|, v[6:7], |v[2:3]|
	s_and_saveexec_b32 s0, vcc_lo
	s_cbranch_execz .LBB7_3
; %bb.2:
	v_rsq_f64_e32 v[8:9], v[4:5]
	v_cmp_eq_f64_e32 vcc_lo, 0, v[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3fe921fb
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[10:11], v[4:5], v[8:9]
	v_mul_f64 v[8:9], v[8:9], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 0.5
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[10:11], v[10:11], v[4:5]
	v_fma_f64 v[8:9], v[12:13], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v9, v9, v5 :: v_dual_cndmask_b32 v8, v8, v4
	v_add_f64 v[10:11], v[8:9], v[8:9]
	v_mul_f64 v[12:13], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[10:11]
	v_add_f64 v[16:17], v[4:5], -v[12:13]
	v_fma_f64 v[20:21], v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[10:11], v[14:15], 1.0
	v_add_f64 v[4:5], v[4:5], -v[12:13]
	v_fma_f64 v[12:13], v[18:19], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[20:21]
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[16:17], v[4:5]
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[4:5], v[12:13]
	v_fma_f64 v[4:5], -v[10:11], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[4:5], v[12:13], v[14:15]
	v_cndmask_b32_e64 v5, v5, 0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v4, v4, 0, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, |v[2:3]|, 1.0
	v_add_f64 v[10:11], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_mul_f64 v[12:13], v[6:7], v[10:11]
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[10:11], v[6:7], -v[12:13]
	v_fma_f64 v[6:7], v[4:5], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_add_f64 v[14:15], v[10:11], v[8:9]
	v_add_f64 v[12:13], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[6:7], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], -v[6:7], s[4:5]
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	v_add_f64 v[10:11], -v[8:9], s[4:5]
	s_mov_b32 s4, 0x33145c07
	s_mov_b32 s5, 0x3c81a626
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[12:13], -v[4:5]
	v_add_f64 v[6:7], v[10:11], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], s[4:5]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[4:5], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v9, 0x3ff921fb, v5, vcc_lo
	v_cndmask_b32_e32 v8, 0x54442d18, v4, vcc_lo
.LBB7_3:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_2)
	v_bfi_b32 v9, 0x7fffffff, v9, v3
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[8:9], off
.LBB7_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_asin_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_vgpr 22
		.amdhsa_next_free_sgpr 6
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
.Lfunc_end7:
	.size	mx_asin_kernel, .Lfunc_end7-mx_asin_kernel
                                        ; -- End function
	.set mx_asin_kernel.num_vgpr, 22
	.set mx_asin_kernel.num_agpr, 0
	.set mx_asin_kernel.numbered_sgpr, 6
	.set mx_asin_kernel.num_named_barrier, 0
	.set mx_asin_kernel.private_seg_size, 0
	.set mx_asin_kernel.uses_vcc, 1
	.set mx_asin_kernel.uses_flat_scratch, 0
	.set mx_asin_kernel.has_dyn_sized_stack, 0
	.set mx_asin_kernel.has_recursion, 0
	.set mx_asin_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1104
; TotalNumSgprs: 8
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 8
; NumVGPRsForWavesPerEU: 22
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
	.protected	mx_acos_kernel          ; -- Begin function mx_acos_kernel
	.globl	mx_acos_kernel
	.p2align	8
	.type	mx_acos_kernel,@function
mx_acos_kernel:                         ; @mx_acos_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB8_4
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0x9fea6a70
	s_mov_b32 s5, 0x3fa05985
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x78a05eaf
	s_mov_b32 s1, 0xbf90a5a3
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[4:5], |v[2:3]|, -0.5, 0.5
	v_mul_f64 v[6:7], v[2:3], v[2:3]
	v_cmp_ge_f64_e64 vcc_lo, |v[2:3]|, 0.5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v7, v7, v5 :: v_dual_cndmask_b32 v6, v6, v4
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0x37024d6a
	s_mov_b32 s1, 0x3f940521
	s_mov_b32 s5, 0x3fedd9ad
	s_mov_b32 s4, 0x336a0500
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x98a70509
	s_mov_b32 s1, 0x3f7ab3a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xa300c8d2
	s_mov_b32 s1, 0x3f88ed60
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x4b77012b
	s_mov_b32 s1, 0x3f8c6fa8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11dccb70
	s_mov_b32 s1, 0x3f91c6c1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xa0adacf
	s_mov_b32 s1, 0x3f96e89f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xc668963f
	s_mov_b32 s1, 0x3f9f1c72
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xb41ce4bd
	s_mov_b32 s1, 0x3fa6db6d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3336fd5b
	s_mov_b32 s1, 0x3fb33333
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555380
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xeeb562d6
	s_mov_b32 s1, 0x3ffaf154
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], v[2:3], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], s[4:5], s[0:1], -v[8:9]
	s_and_saveexec_b32 s6, vcc_lo
	s_cbranch_execz .LBB8_3
; %bb.2:
	v_rsq_f64_e32 v[8:9], v[4:5]
	v_cmp_eq_f64_e32 vcc_lo, 0, v[4:5]
	s_mov_b32 s5, 0x3ffdd9ad
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[10:11], v[4:5], v[8:9]
	v_mul_f64 v[8:9], v[8:9], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 0.5
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[10:11], v[10:11], v[4:5]
	v_fma_f64 v[8:9], v[12:13], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v9, v9, v5 :: v_dual_cndmask_b32 v8, v8, v4
	v_add_f64 v[10:11], v[8:9], v[8:9]
	v_mul_f64 v[12:13], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[10:11]
	v_add_f64 v[16:17], v[4:5], -v[12:13]
	v_fma_f64 v[20:21], v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[10:11], v[14:15], 1.0
	v_add_f64 v[4:5], v[4:5], -v[12:13]
	v_fma_f64 v[12:13], v[18:19], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[20:21]
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[16:17], v[4:5]
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[4:5], v[12:13]
	v_fma_f64 v[4:5], -v[10:11], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[4:5], v[12:13], v[14:15]
	v_cndmask_b32_e64 v5, v5, 0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v4, v4, 0, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[2:3]
	v_add_f64 v[10:11], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	v_fma_f64 v[8:9], v[10:11], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[10:11], v[6:7], v[4:5]
	v_mul_f64 v[6:7], v[8:9], -2.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[10:11], v[4:5]
	v_fma_f64 v[6:7], s[4:5], s[0:1], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[4:5]
	v_dual_cndmask_b32 v5, v5, v7 :: v_dual_cndmask_b32 v4, v4, v6
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v4, 0x54442d18, v4, vcc_lo
	v_cndmask_b32_e32 v5, 0x400921fb, v5, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 1.0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2)
	v_dual_cndmask_b32 v9, 0, v5 :: v_dual_cndmask_b32 v8, 0, v4
.LBB8_3:
	s_or_b32 exec_lo, exec_lo, s6
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[8:9], off
.LBB8_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_acos_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_vgpr 22
		.amdhsa_next_free_sgpr 7
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
.Lfunc_end8:
	.size	mx_acos_kernel, .Lfunc_end8-mx_acos_kernel
                                        ; -- End function
	.set mx_acos_kernel.num_vgpr, 22
	.set mx_acos_kernel.num_agpr, 0
	.set mx_acos_kernel.numbered_sgpr, 7
	.set mx_acos_kernel.num_named_barrier, 0
	.set mx_acos_kernel.private_seg_size, 0
	.set mx_acos_kernel.uses_vcc, 1
	.set mx_acos_kernel.uses_flat_scratch, 0
	.set mx_acos_kernel.has_dyn_sized_stack, 0
	.set mx_acos_kernel.has_recursion, 0
	.set mx_acos_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 992
; TotalNumSgprs: 9
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 9
; NumVGPRsForWavesPerEU: 22
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
	.protected	mx_atan_kernel          ; -- Begin function mx_atan_kernel
	.globl	mx_atan_kernel
	.p2align	8
	.type	mx_atan_kernel,@function
mx_atan_kernel:                         ; @mx_atan_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB9_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0xb5e68a13
	s_mov_b32 s5, 0x3eeba404
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0xbd3237f4
	s_mov_b32 s1, 0xbf23e260
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_dual_mov_b32 v4, v2 :: v_dual_and_b32 v5, 0x7fffffff, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[4:5], 1.0
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[12:13]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	v_cmp_gt_f64_e64 vcc_lo, |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[6:7], v[6:7], |v[2:3]|, 1.0
	v_dual_cndmask_b32 v5, v5, v7 :: v_dual_cndmask_b32 v4, v2, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[4:5], v[4:5]
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0x69efb384
	s_mov_b32 s1, 0x3f4b2bb0
	s_mov_b32 s4, 0x336a0500
	s_mov_b32 s5, 0x3fedd9ad
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xaf56de9b
	s_mov_b32 s1, 0xbf67952d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xa595c56f
	s_mov_b32 s1, 0x3f7d6d43
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xa57d9582
	s_mov_b32 s1, 0xbf8c6ea4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x5f08b19f
	s_mov_b32 s1, 0x3f967e29
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfc27006a
	s_mov_b32 s1, 0xbf9e9ae6
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x5711927a
	s_mov_b32 s1, 0x3fa2c15b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xe82d3ff0
	s_mov_b32 s1, 0xbfa59976
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x6ef28734
	s_mov_b32 s1, 0x3fa82d5d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x6a214619
	s_mov_b32 s1, 0xbfaae5ce
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x8427b883
	s_mov_b32 s1, 0x3fae1bb4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x8b207f05
	s_mov_b32 s1, 0xbfb110e4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x57b87036
	s_mov_b32 s1, 0x3fb3b136
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x19378e4f
	s_mov_b32 s1, 0xbfb745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x17e1913c
	s_mov_b32 s1, 0x3fbc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x92376b7d
	s_mov_b32 s1, 0xbfc24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x999952cc
	s_mov_b32 s1, 0x3fc99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555523
	s_mov_b32 s1, 0xbfd55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xeeb562d6
	s_mov_b32 s1, 0x3ffaf154
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[4:5], v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], s[4:5], s[0:1], -v[4:5]
	v_dual_cndmask_b32 v5, v5, v7 :: v_dual_cndmask_b32 v2, v4, v6
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_bfi_b32 v3, 0x7fffffff, v5, v3
	global_store_b64 v[0:1], v[2:3], off
.LBB9_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_atan_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_sgpr 6
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
.Lfunc_end9:
	.size	mx_atan_kernel, .Lfunc_end9-mx_atan_kernel
                                        ; -- End function
	.set mx_atan_kernel.num_vgpr, 14
	.set mx_atan_kernel.num_agpr, 0
	.set mx_atan_kernel.numbered_sgpr, 6
	.set mx_atan_kernel.num_named_barrier, 0
	.set mx_atan_kernel.private_seg_size, 0
	.set mx_atan_kernel.uses_vcc, 1
	.set mx_atan_kernel.uses_flat_scratch, 0
	.set mx_atan_kernel.has_dyn_sized_stack, 0
	.set mx_atan_kernel.has_recursion, 0
	.set mx_atan_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 928
; TotalNumSgprs: 8
; NumVgprs: 14
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 8
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
	.protected	mx_asinh_kernel         ; -- Begin function mx_asinh_kernel
	.globl	mx_asinh_kernel
	.p2align	8
	.type	mx_asinh_kernel,@function
mx_asinh_kernel:                        ; @mx_asinh_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB10_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	v_dual_mov_b32 v9, 0x40000 :: v_dual_mov_b32 v8, 0
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s6, 0xbf559e2b
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s7, 0x3fc3ab76
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0x3fe55555
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_le_f64_e64 vcc_lo, 0x5ff00000, |v[2:3]|
	v_cndmask_b32_e64 v4, 0, 0xfffffe00, vcc_lo
	v_cndmask_b32_e32 v9, 0x3ff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], |v[2:3]|, v4
	v_mul_f64 v[6:7], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[8:9], v[6:7]
	v_add_f64 v[12:13], v[10:11], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], -v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[12:13], v[6:7], v[14:15]
	v_fma_f64 v[6:7], v[4:5], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[10:11], v[6:7]
	v_rsq_f64_e32 v[12:13], v[8:9]
	v_cmp_eq_f64_e64 s0, 0, v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[14:15], v[8:9], v[12:13]
	v_mul_f64 v[12:13], v[12:13], 0.5
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[14:15], v[16:17], v[14:15]
	v_fma_f64 v[12:13], v[12:13], v[16:17], v[12:13]
	v_fma_f64 v[16:17], -v[14:15], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[12:13], v[14:15]
	v_cndmask_b32_e64 v13, v13, v9, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v12, v12, v8, s0
	v_mul_f64 v[14:15], v[12:13], v[12:13]
	v_add_f64 v[16:17], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[8:9], -v[14:15]
	v_rcp_f64_e32 v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[16:17], v[20:21], 1.0
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_fma_f64 v[14:15], v[12:13], v[12:13], -v[14:15]
	v_fma_f64 v[10:11], v[10:11], v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], -v[16:17], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[16:17], v[10:11], v[6:7]
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, 0, s0
	v_cndmask_b32_e64 v6, v6, 0, s0
	s_mov_b32 s0, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_add_f64 v[10:11], v[4:5], v[8:9]
	v_add_f64 v[12:13], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[10:11], -v[4:5]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], -v[14:15]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[4:5], v[4:5], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e64 s0, s[0:1], v[8:9]
	v_subrev_co_ci_u32_e64 v26, null, 0, v12, s0
	s_mov_b32 s0, 0x55555780
	v_sub_nc_u32_e32 v12, 0, v26
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[8:9], v[6:7], v12
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	v_add_f64 v[10:11], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_f64 v[16:17], v[8:9], -1.0
	v_add_f64 v[6:7], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[4:5], v[4:5], v12
	v_add_f64 v[18:19], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[4:5], v[6:7]
	v_add_f64 v[4:5], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[10:11], v[6:7]
	v_add_f64 v[18:19], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[14:15], v[12:13]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[12:13], v[14:15], 1.0
	v_add_f64 v[4:5], v[4:5], -v[16:17]
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[8:9], v[8:9], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[18:19], v[8:9]
	v_mul_f64 v[20:21], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[14:15], v[12:13], -v[20:21]
	v_fma_f64 v[10:11], v[14:15], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[20:21], v[10:11]
	v_add_f64 v[24:25], v[18:19], -v[22:23]
	v_add_f64 v[16:17], v[22:23], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	v_add_f64 v[4:5], v[4:5], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[10:11], v[4:5]
	v_add_f64 v[10:11], v[24:25], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	v_add_f64 v[22:23], v[24:25], -v[10:11]
	v_mul_f64 v[18:19], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[16:17], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[10:11], -v[12:13]
	v_add_f64 v[18:19], v[12:13], -v[18:19]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[10:11]
	v_add_f64 v[10:11], v[14:15], v[16:17]
	v_add_f64 v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[14:15]
	v_add_f64 v[4:5], v[20:21], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[6:7], v[4:5]
	v_add_f64 v[6:7], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	v_fma_f64 v[12:13], v[8:9], s[6:7], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[0:1]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_cndmask_b32_e64 v14, 0, 0x200, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[2:3]|
	v_add_nc_u32_e32 v14, v26, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_f64 v[10:11], v[12:13], v[8:9]
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_mul_f64 v[16:17], v[14:15], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[0:1], -v[16:17]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_fma_f64 v[8:9], v[14:15], s[0:1], v[12:13]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_and_b32_e32 v6, 0x7fffffff, v3
	v_add_f64 v[4:5], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, v4, v2, vcc_lo
	v_cndmask_b32_e32 v5, v5, v6, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_cndmask_b32_e32 v4, 0x7ff00000, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_bfi_b32 v3, 0x7fffffff, v4, v3
	global_store_b64 v[0:1], v[2:3], off
.LBB10_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_asinh_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_vgpr 27
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
.Lfunc_end10:
	.size	mx_asinh_kernel, .Lfunc_end10-mx_asinh_kernel
                                        ; -- End function
	.set mx_asinh_kernel.num_vgpr, 27
	.set mx_asinh_kernel.num_agpr, 0
	.set mx_asinh_kernel.numbered_sgpr, 8
	.set mx_asinh_kernel.num_named_barrier, 0
	.set mx_asinh_kernel.private_seg_size, 0
	.set mx_asinh_kernel.uses_vcc, 1
	.set mx_asinh_kernel.uses_flat_scratch, 0
	.set mx_asinh_kernel.has_dyn_sized_stack, 0
	.set mx_asinh_kernel.has_recursion, 0
	.set mx_asinh_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1976
; TotalNumSgprs: 10
; NumVgprs: 27
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 27
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
	.protected	mx_acosh_kernel         ; -- Begin function mx_acosh_kernel
	.globl	mx_acosh_kernel
	.p2align	8
	.type	mx_acosh_kernel,@function
mx_acosh_kernel:                        ; @mx_acosh_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB11_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	v_dual_mov_b32 v9, 0x40000 :: v_dual_mov_b32 v8, 0
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s6, 0xbf559e2b
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s7, 0x3fc3ab76
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0x3fe55555
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_le_f64_e32 vcc_lo, 0x5ff00000, v[2:3]
	v_cndmask_b32_e64 v4, 0, 0xfffffe00, vcc_lo
	v_cndmask_b32_e32 v9, 0x3ff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[2:3], v4
	v_mul_f64 v[6:7], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[6:7], -v[8:9]
	v_add_f64 v[12:13], v[10:11], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], -v[10:11]
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[12:13], v[6:7], v[14:15]
	v_fma_f64 v[6:7], v[4:5], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[10:11], v[6:7]
	v_rsq_f64_e32 v[12:13], v[8:9]
	v_cmp_eq_f64_e64 s0, 0, v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[14:15], v[8:9], v[12:13]
	v_mul_f64 v[12:13], v[12:13], 0.5
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[14:15], v[16:17], v[14:15]
	v_fma_f64 v[12:13], v[12:13], v[16:17], v[12:13]
	v_fma_f64 v[16:17], -v[14:15], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[12:13], v[14:15]
	v_cndmask_b32_e64 v13, v13, v9, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v12, v12, v8, s0
	v_mul_f64 v[14:15], v[12:13], v[12:13]
	v_add_f64 v[16:17], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[8:9], -v[14:15]
	v_rcp_f64_e32 v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[16:17], v[20:21], 1.0
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_fma_f64 v[14:15], v[12:13], v[12:13], -v[14:15]
	v_fma_f64 v[10:11], v[10:11], v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], -v[16:17], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[16:17], v[10:11], v[6:7]
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, 0, s0
	v_cndmask_b32_e64 v6, v6, 0, s0
	s_mov_b32 s0, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_add_f64 v[10:11], v[4:5], v[8:9]
	v_add_f64 v[12:13], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[10:11], -v[4:5]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], -v[14:15]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[4:5], v[4:5], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e64 s0, s[0:1], v[8:9]
	v_subrev_co_ci_u32_e64 v26, null, 0, v12, s0
	s_mov_b32 s0, 0x55555780
	v_sub_nc_u32_e32 v12, 0, v26
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[8:9], v[6:7], v12
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	v_add_f64 v[10:11], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_f64 v[16:17], v[8:9], -1.0
	v_add_f64 v[6:7], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[4:5], v[4:5], v12
	v_add_f64 v[18:19], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[4:5], v[6:7]
	v_add_f64 v[4:5], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[10:11], v[6:7]
	v_add_f64 v[18:19], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[14:15], v[12:13]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[12:13], v[14:15], 1.0
	v_add_f64 v[4:5], v[4:5], -v[16:17]
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[8:9], v[8:9], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[18:19], v[8:9]
	v_mul_f64 v[20:21], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[14:15], v[12:13], -v[20:21]
	v_fma_f64 v[10:11], v[14:15], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[20:21], v[10:11]
	v_add_f64 v[24:25], v[18:19], -v[22:23]
	v_add_f64 v[16:17], v[22:23], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	v_add_f64 v[4:5], v[4:5], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[10:11], v[4:5]
	v_add_f64 v[10:11], v[24:25], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	v_add_f64 v[22:23], v[24:25], -v[10:11]
	v_mul_f64 v[18:19], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[16:17], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[10:11], -v[12:13]
	v_add_f64 v[18:19], v[12:13], -v[18:19]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[10:11]
	v_add_f64 v[10:11], v[14:15], v[16:17]
	v_add_f64 v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[14:15]
	v_add_f64 v[4:5], v[20:21], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[6:7], v[4:5]
	v_add_f64 v[6:7], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	v_fma_f64 v[12:13], v[8:9], s[6:7], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[0:1]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_cndmask_b32_e64 v14, 0, 0x200, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[2:3]
	v_add_nc_u32_e32 v14, v26, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_f64 v[10:11], v[12:13], v[8:9]
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_mul_f64 v[16:17], v[14:15], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[0:1], -v[16:17]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_fma_f64 v[8:9], v[14:15], s[0:1], v[12:13]
	v_cmp_ngt_f64_e64 s0, 1.0, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v3, 0x7ff80000, v5, s0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB11_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_acosh_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_vgpr 27
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
.Lfunc_end11:
	.size	mx_acosh_kernel, .Lfunc_end11-mx_acosh_kernel
                                        ; -- End function
	.set mx_acosh_kernel.num_vgpr, 27
	.set mx_acosh_kernel.num_agpr, 0
	.set mx_acosh_kernel.numbered_sgpr, 8
	.set mx_acosh_kernel.num_named_barrier, 0
	.set mx_acosh_kernel.private_seg_size, 0
	.set mx_acosh_kernel.uses_vcc, 1
	.set mx_acosh_kernel.uses_flat_scratch, 0
	.set mx_acosh_kernel.has_dyn_sized_stack, 0
	.set mx_acosh_kernel.has_recursion, 0
	.set mx_acosh_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1948
; TotalNumSgprs: 10
; NumVgprs: 27
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 27
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
	.protected	mx_atanh_kernel         ; -- Begin function mx_atanh_kernel
	.globl	mx_atanh_kernel
	.p2align	8
	.type	mx_atanh_kernel,@function
mx_atanh_kernel:                        ; @mx_atanh_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB12_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s6, 0xbf559e2b
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s7, 0x3fc3ab76
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], -|v[2:3]|, 1.0
	v_add_f64 v[10:11], |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[6:7], v[4:5]
	v_add_f64 v[12:13], -v[4:5], 1.0
	v_add_f64 v[12:13], v[12:13], -|v[2:3]|
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[8:9], v[6:7], v[6:7]
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[8:9], v[6:7], v[6:7]
	v_mul_f64 v[8:9], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[4:5], v[8:9]
	v_fma_f64 v[4:5], v[8:9], v[4:5], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[8:9], v[12:13], v[4:5]
	v_add_f64 v[12:13], v[14:15], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], -v[12:13]
	v_add_f64 v[14:15], v[12:13], -v[14:15]
	v_add_f64 v[18:19], v[10:11], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[14:15], -v[4:5]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], |v[2:3]|, -v[10:11]
	v_add_f64 v[4:5], v[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[10:11], v[4:5]
	v_add_f64 v[4:5], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[6:7], v[4:5]
	v_add_f64 v[6:7], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_frexp_mant_f64_e32 v[10:11], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[10:11]
	s_mov_b32 s0, 0x55555780
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_co_ci_u32_e64 v26, null, 0, v12, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[2:3]|
	v_sub_nc_u32_e32 v12, 0, v26
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[6:7], v12
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_f64 v[16:17], v[10:11], -1.0
	v_add_f64 v[6:7], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[4:5], v[4:5], v12
	v_add_f64 v[18:19], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[6:7]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[4:5], v[6:7]
	v_add_f64 v[4:5], v[4:5], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[8:9], v[6:7]
	v_add_f64 v[18:19], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[14:15], v[12:13]
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[12:13], v[14:15], 1.0
	v_add_f64 v[4:5], v[4:5], -v[16:17]
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[18:19], v[10:11]
	v_mul_f64 v[20:21], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[12:13], -v[20:21]
	v_fma_f64 v[8:9], v[14:15], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[20:21], v[8:9]
	v_add_f64 v[24:25], v[18:19], -v[22:23]
	v_add_f64 v[16:17], v[22:23], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	v_add_f64 v[4:5], v[4:5], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[8:9], v[24:25], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[16:17], v[10:11], v[8:9]
	v_add_f64 v[22:23], v[24:25], -v[8:9]
	v_mul_f64 v[18:19], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[16:17], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[8:9], -v[12:13]
	v_add_f64 v[18:19], v[12:13], -v[18:19]
	v_add_f64 v[8:9], v[8:9], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[8:9], v[14:15], v[16:17]
	v_add_f64 v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[14:15]
	v_add_f64 v[4:5], v[20:21], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_mul_f64 v[4:5], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[6:7], v[4:5]
	v_add_f64 v[6:7], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[6:7], v[6:7]
	v_fma_f64 v[12:13], v[10:11], s[6:7], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[12:13], s[0:1]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	v_cvt_f64_i32_e32 v[14:15], v26
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_f64 v[8:9], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[16:17], v[14:15], s[0:1]
	v_ldexp_f64 v[4:5], v[4:5], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[0:1], -v[16:17]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[6:7]
	v_fma_f64 v[10:11], v[14:15], s[0:1], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[8:9], v[4:5]
	v_add_f64 v[16:17], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[6:7], v[12:13]
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[14:15], -v[6:7]
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[14:15], -v[18:19]
	v_add_f64 v[8:9], v[12:13], -v[18:19]
	v_add_f64 v[12:13], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	v_add_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[12:13], -v[10:11]
	v_add_f64 v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[8:9]
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	v_add_f64 v[16:17], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_and_b32_e32 v6, 0x7fffffff, v3
	v_add_f64 v[4:5], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], 0.5
	v_cndmask_b32_e32 v4, v4, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, v5, v6, vcc_lo
	v_cmp_ngt_f64_e64 vcc_lo, |v[2:3]|, 1.0
	v_cndmask_b32_e32 v5, 0x7ff80000, v5, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	v_cmp_nge_f64_e64 vcc_lo, |v[2:3]|, 1.0
	v_bfi_b32 v3, 0x7fffffff, v5, v3
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB12_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_atanh_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_vgpr 27
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
		.amdhsa_inst_pref_size 14
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end12:
	.size	mx_atanh_kernel, .Lfunc_end12-mx_atanh_kernel
                                        ; -- End function
	.set mx_atanh_kernel.num_vgpr, 27
	.set mx_atanh_kernel.num_agpr, 0
	.set mx_atanh_kernel.numbered_sgpr, 8
	.set mx_atanh_kernel.num_named_barrier, 0
	.set mx_atanh_kernel.private_seg_size, 0
	.set mx_atanh_kernel.uses_vcc, 1
	.set mx_atanh_kernel.uses_flat_scratch, 0
	.set mx_atanh_kernel.has_dyn_sized_stack, 0
	.set mx_atanh_kernel.has_recursion, 0
	.set mx_atanh_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1676
; TotalNumSgprs: 10
; NumVgprs: 27
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 27
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
	.protected	mx_erf_kernel           ; -- Begin function mx_erf_kernel
	.globl	mx_erf_kernel
	.p2align	8
	.type	mx_erf_kernel,@function
mx_erf_kernel:                          ; @mx_erf_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB13_6
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_nlt_f64_e64 |v[2:3]|, 1.0
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB13_3
; %bb.2:
	s_mov_b32 s4, 0x502a41cd
	s_mov_b32 s6, 0xc14b24be
	s_mov_b32 s5, 0xbcc145a3
	s_mov_b32 s7, 0x3c598d37
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], |v[2:3]|, s[6:7], s[4:5]
	s_mov_b32 s4, 0xd735f9ec
	s_mov_b32 s5, 0x3d162dee
	s_mov_b32 s6, 0x6a5dcb37
	s_mov_b32 s7, 0x3e5ade15
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x5552ca22
	s_mov_b32 s5, 0xbd61ffe5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x7074b644
	s_mov_b32 s5, 0x3da4b9ba
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0xa78ce240
	s_mov_b32 s5, 0xbde20345
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0xcefddd8
	s_mov_b32 s5, 0x3e188b7a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x8c94b617
	s_mov_b32 s5, 0xbe4aded4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x312306d0
	s_mov_b32 s5, 0x3e7803aa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x6f4c5a9b
	s_mov_b32 s5, 0xbea1b010
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x7cfd79ae
	s_mov_b32 s5, 0x3ec58c0e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x6410fdf7
	s_mov_b32 s5, 0xbee59e38
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x1f9b1786
	s_mov_b32 s5, 0x3f0192fc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0xf4634b2e
	s_mov_b32 s5, 0xbf162cf3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0xb42f7e4b
	s_mov_b32 s5, 0x3f2314df
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0xc047288a
	s_mov_b32 s5, 0xbf12cb68
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x7bbcce25
	s_mov_b32 s5, 0xbf4038ff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0xae1babae
	s_mov_b32 s5, 0x3f5a9466
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0xe65a6063
	s_mov_b32 s5, 0xbf258be1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x6738ee3a
	s_mov_b32 s5, 0xbf939bc1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x28146b69
	s_mov_b32 s5, 0x3fba4fbc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0xa69750c4
	s_mov_b32 s5, 0x3fe45f2d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x919fcca8
	s_mov_b32 s5, 0x3fc06ebb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], s[4:5]
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0xbff71547
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], |v[2:3]|
	v_mul_f64 v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0xbfe62e42
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[4:5]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[4:5], -v[4:5]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0xbc7abc9e
	v_cvt_i32_f64_e32 v12, v[6:7]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], s[4:5], v[8:9]
	s_mov_b32 s4, 0xfca7ab0c
	s_mov_b32 s5, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], s[6:7], s[4:5]
	s_mov_b32 s4, 0x623fde64
	s_mov_b32 s5, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	s_mov_b32 s4, 0x7c89e6b0
	s_mov_b32 s5, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	s_mov_b32 s4, 0x14761f6e
	s_mov_b32 s5, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	s_mov_b32 s4, 0x1852b7b0
	s_mov_b32 s5, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	s_mov_b32 s4, 0x11122322
	s_mov_b32 s5, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	s_mov_b32 s4, 0x555502a1
	s_mov_b32 s5, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	s_mov_b32 s4, 0x55555511
	s_mov_b32 s5, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	s_mov_b32 s4, 11
	s_mov_b32 s5, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], 1.0
	v_fma_f64 v[6:7], v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v12
	v_add_f64 v[6:7], -v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v4, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v5, 0x3ff00000, v7, s0
.LBB13_3:
	s_and_not1_saveexec_b32 s0, s1
	s_cbranch_execz .LBB13_5
; %bb.4:
	v_mul_f64 v[4:5], v[2:3], v[2:3]
	s_mov_b32 s4, 0xdfeb1f49
	s_mov_b32 s6, 0x51d2ebeb
	s_mov_b32 s5, 0x3e4d6e3d
	s_mov_b32 s7, 0xbe0ab15c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], s[6:7], s[4:5]
	s_mov_b32 s4, 0x63844720
	s_mov_b32 s5, 0xbe85bfe7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x4280cfb9
	s_mov_b32 s5, 0x3ebb97e4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x4c771c5
	s_mov_b32 s5, 0xbeef4ca2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x75531772
	s_mov_b32 s5, 0x3f1f9a2b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x149d904
	s_mov_b32 s5, 0xbf4c02db
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xcf7e2856
	s_mov_b32 s5, 0x3f7565bc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x311ee09b
	s_mov_b32 s5, 0xbf9b82ce
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x1a0408d1
	s_mov_b32 s5, 0x3fbce2f2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x6b0379b2
	s_mov_b32 s5, 0xbfd81274
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x8214db68
	s_mov_b32 s5, 0x3fc06eba
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[4:5], v[6:7], s[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[4:5], |v[2:3]|, v[4:5], |v[2:3]|
.LBB13_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_2)
	v_bfi_b32 v5, 0x7fffffff, v5, v3
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB13_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_erf_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_inst_pref_size 13
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end13:
	.size	mx_erf_kernel, .Lfunc_end13-mx_erf_kernel
                                        ; -- End function
	.set mx_erf_kernel.num_vgpr, 13
	.set mx_erf_kernel.num_agpr, 0
	.set mx_erf_kernel.numbered_sgpr, 8
	.set mx_erf_kernel.num_named_barrier, 0
	.set mx_erf_kernel.private_seg_size, 0
	.set mx_erf_kernel.uses_vcc, 1
	.set mx_erf_kernel.uses_flat_scratch, 0
	.set mx_erf_kernel.has_dyn_sized_stack, 0
	.set mx_erf_kernel.has_recursion, 0
	.set mx_erf_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1628
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
	.protected	mx_erfc_kernel          ; -- Begin function mx_erfc_kernel
	.globl	mx_erfc_kernel
	.p2align	8
	.type	mx_erfc_kernel,@function
mx_erfc_kernel:                         ; @mx_erfc_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB14_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0x54df3c0e
	s_mov_b32 s5, 0xbe41f39d
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x37cfa789
	s_mov_b32 s1, 0xbe411663
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], |v[2:3]|, 4.0
	v_add_f64 v[16:17], |v[2:3]|, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[6:7], v[4:5]
	v_add_f64 v[18:19], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[20:21], v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[8:9], v[6:7], v[6:7]
	v_add_f64 v[8:9], |v[2:3]|, -4.0
	v_fma_f64 v[22:23], -v[18:19], v[20:21], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[20:21], v[22:23], v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[4:5], v[6:7], v[6:7]
	v_fma_f64 v[18:19], -v[18:19], v[20:21], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[6:7], v[8:9], v[4:5]
	v_fma_f64 v[18:19], v[18:19], v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_fma_f64 v[8:9], v[8:9], -4.0, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[6:7], |v[2:3]|, v[8:9]
	v_fma_f64 v[4:5], v[4:5], v[8:9], v[6:7]
	v_mul_f64 v[8:9], v[2:3], -v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[4:5], s[0:1]
	s_mov_b32 s0, 0xd9802b82
	s_mov_b32 s1, 0x3e7b45f1
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[8:9]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x8a03dcdb
	s_mov_b32 s1, 0x3e6d9048
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x2eba62d8
	s_mov_b32 s1, 0xbeab87b0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0xa56e15f1
	s_mov_b32 s1, 0x3e95104b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x71c907de
	s_mov_b32 s1, 0x3ed7f29f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x2cd770fb
	s_mov_b32 s1, 0xbee78f5c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mul_f64 v[10:11], v[8:9], s[0:1]
	s_mov_b32 s0, 0x76d0a51a
	s_mov_b32 s1, 0xbef995fb
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0xc022d0ed
	s_mov_b32 s1, 0x3f23be2e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rndne_f64_e32 v[10:11], v[10:11]
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], s[0:1], v[8:9]
	s_mov_b32 s0, 0x2fdbf62e
	s_mov_b32 s1, 0xbf2a1deb
	v_cvt_i32_f64_e32 v20, v[10:11]
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], s[0:1], v[12:13]
	s_mov_b32 s0, 0x3689fc43
	s_mov_b32 s1, 0xbf48d4ac
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[4:5], s[0:1]
	s_mov_b32 s0, 0x192d909b
	s_mov_b32 s1, 0x3f749c67
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x852ff070
	s_mov_b32 s1, 0xbf909623
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0xdfadea8f
	s_mov_b32 s1, 0x3fa3079e
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0xdff65910
	s_mov_b32 s1, 0xbfb0fb06
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x4de8f32
	s_mov_b32 s1, 0x3fb7fee0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x3c3dbeb3
	s_mov_b32 s1, 0xbfb9ddb2
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0xfcfa6930
	s_mov_b32 s1, 0x3fb16ece
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0xf66fb8a3
	s_mov_b32 s1, 0x3f8f7f5d
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0xd154a2a8
	s_mov_b32 s1, 0xbfc1df1a
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[0:1]
	s_mov_b32 s0, 0xb74febf8
	s_mov_b32 s1, 0x3fcdd2c8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], 1.0
	v_fma_f64 v[4:5], v[4:5], v[6:7], s[0:1]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[6:7], v[12:13], v[14:15], 1.0
	v_fma_f64 v[10:11], v[4:5], v[18:19], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[6:7], v[6:7], v20
	v_fma_f64 v[12:13], -v[10:11], v[16:17], 1.0
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v14, 0x7ff00000, v7, vcc_lo
	v_fma_f64 v[7:8], -v[2:3], v[2:3], -v[8:9]
	s_and_b32 vcc_lo, s0, vcc_lo
	v_add_f64 v[4:5], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v13, 0, v14, s0
	v_cndmask_b32_e32 v12, 0, v6, vcc_lo
	s_mov_b32 s0, 0x41e48bfc
	s_mov_b32 s1, 0x403b39dc
	v_cmp_ngt_f64_e64 vcc_lo, |v[2:3]|, s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[12:13], v[7:8], v[12:13]
	v_fma_f64 v[4:5], v[18:19], v[4:5], v[10:11]
	v_mul_f64 v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
	v_cmp_gt_f64_e32 vcc_lo, 0, v[2:3]
	v_add_f64 v[6:7], -v[4:5], 2.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v3, v5, v7 :: v_dual_cndmask_b32 v2, v4, v6
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB14_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_erfc_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_sgpr 6
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
		.amdhsa_inst_pref_size 12
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end14:
	.size	mx_erfc_kernel, .Lfunc_end14-mx_erfc_kernel
                                        ; -- End function
	.set mx_erfc_kernel.num_vgpr, 24
	.set mx_erfc_kernel.num_agpr, 0
	.set mx_erfc_kernel.numbered_sgpr, 6
	.set mx_erfc_kernel.num_named_barrier, 0
	.set mx_erfc_kernel.private_seg_size, 0
	.set mx_erfc_kernel.uses_vcc, 1
	.set mx_erfc_kernel.uses_flat_scratch, 0
	.set mx_erfc_kernel.has_dyn_sized_stack, 0
	.set mx_erfc_kernel.has_recursion, 0
	.set mx_erfc_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1504
; TotalNumSgprs: 8
; NumVgprs: 24
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 8
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
	.protected	mx_tgamma_kernel        ; -- Begin function mx_tgamma_kernel
	.globl	mx_tgamma_kernel
	.p2align	8
	.type	mx_tgamma_kernel,@function
mx_tgamma_kernel:                       ; @mx_tgamma_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB15_38
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr12_vgpr13
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_ngt_f64_e64 0x40300000, |v[2:3]|
	s_xor_b32 s12, exec_lo, s1
	s_cbranch_execz .LBB15_15
; %bb.2:
	v_frexp_mant_f64_e64 v[4:5], |v[2:3]|
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s2, 0x968915a9
	s_mov_b32 s4, 0x4222de17
	s_mov_b32 s3, 0x3fba6564
	s_mov_b32 s5, 0x3fbdee67
	s_mov_b32 s11, 0x3ff71547
	s_mov_b32 s10, 0x652b82fe
	s_mov_b32 s8, 0x6a5dcb37
	s_mov_b32 s9, 0x3e5ade15
	s_mov_b32 s14, 0x7c89e6b0
	s_mov_b32 s15, 0x3efa0199
	v_rcp_f64_e64 v[26:27], |v[2:3]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[28:29], -|v[2:3]|, v[26:27], 1.0
	v_cndmask_b32_e64 v6, 0, 1, vcc_lo
	v_ldexp_f64 v[4:5], v[4:5], v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[28:29], v[26:27], v[26:27]
	v_add_f64 v[6:7], v[4:5], 1.0
	v_add_f64 v[12:13], v[4:5], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[28:29], -|v[2:3]|, v[26:27], 1.0
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[14:15], v[4:5]
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_add_f64 v[8:9], v[6:7], -v[10:11]
	v_mul_f64 v[10:11], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	v_fma_f64 v[8:9], v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[4:5], v[4:5]
	v_fma_f64 v[8:9], v[6:7], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[10:11], v[8:9]
	v_fma_f64 v[14:15], v[12:13], s[4:5], s[2:3]
	s_mov_b32 s2, 0x3abe935a
	s_mov_b32 s3, 0x3fbe25e4
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	v_mul_f64 v[20:21], v[6:7], v[12:13]
	s_mov_b32 s4, 0xfca7ab0c
	s_mov_b32 s5, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[2:3]
	s_mov_b32 s2, 0x47e6c9c2
	s_mov_b32 s3, 0x3fc110ef
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[2:3]
	s_mov_b32 s2, 0xcfa74449
	s_mov_b32 s3, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[2:3]
	s_mov_b32 s2, 0x71bf3c30
	s_mov_b32 s3, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[2:3]
	s_mov_b32 s2, 0x1c7792ce
	s_mov_b32 s3, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[2:3]
	s_mov_b32 s2, 0x924920da
	s_mov_b32 s3, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[2:3]
	s_mov_b32 s2, 0x9999999c
	s_mov_b32 s3, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[2:3]
	s_mov_b32 s3, 0x3c7abc9e
	s_mov_b32 s2, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[12:13], v[14:15]
	v_fma_f64 v[10:11], v[12:13], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[14:15], v[10:11]
	v_add_f64 v[14:15], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[18:19], v[14:15], s[0:1]
	v_add_f64 v[16:17], v[14:15], -v[16:17]
	s_mov_b32 s1, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[22:23], v[18:19], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_fma_f64 v[16:17], v[12:13], v[6:7], -v[20:21]
	s_mov_b32 s0, 0xd5df274d
	s_mov_b32 s1, 0x3c8543b0
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], s[0:1]
	v_fma_f64 v[12:13], v[12:13], v[4:5], v[16:17]
	s_mov_b32 s1, 0x3fe62e42
	s_mov_b32 s0, 0xfefa39ef
	v_ldexp_f64 v[4:5], v[4:5], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_fma_f64 v[8:9], v[8:9], v[6:7], v[12:13]
	v_ldexp_f64 v[6:7], v[6:7], 1
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
	v_frexp_exp_i32_f64_e32 v12, v[2:3]
	v_add_f64 v[10:11], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v12, null, 0, v12, vcc_lo
	v_cvt_f64_i32_e32 v[12:13], v12
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[6:7], v[10:11]
	v_add_f64 v[16:17], v[10:11], -v[18:19]
	v_mul_f64 v[18:19], v[12:13], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[14:15], -v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[12:13], s[0:1], -v[18:19]
	s_mov_b32 s1, 0xbfe62e42
	v_add_f64 v[6:7], v[10:11], -v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_fma_f64 v[8:9], v[12:13], s[2:3], v[16:17]
	s_mov_b32 s3, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[14:15], v[4:5]
	v_add_f64 v[18:19], v[6:7], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[6:7], v[10:11]
	v_add_f64 v[14:15], v[10:11], -v[14:15]
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[12:13], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[14:15], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[14:15], -v[8:9]
	v_add_f64 v[6:7], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[10:11]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_add_f64 v[16:17], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[10:11], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_fma_f64 v[8:9], |v[2:3]|, 0.5, 0xbfd00000
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[6:7], -v[16:17]
	v_mul_f64 v[12:13], v[8:9], v[6:7]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[8:9], v[6:7], -v[12:13]
	v_cmp_class_f64_e64 vcc_lo, v[12:13], 0x204
	v_fma_f64 v[6:7], v[8:9], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[12:13], v[6:7]
	v_dual_cndmask_b32 v15, v11, v13 :: v_dual_cndmask_b32 v14, v10, v12
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[4:5], v[14:15], s[10:11]
	s_mov_b32 s11, 0xbff71547
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[14:15]|
	v_mul_f64 v[20:21], |v[2:3]|, s[10:11]
	s_mov_b32 s10, 0x623fde64
	s_mov_b32 s11, 0x3ec71dee
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	v_rndne_f64_e32 v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rndne_f64_e32 v[20:21], v[20:21]
	v_dual_cndmask_b32 v7, 0, v7 :: v_dual_cndmask_b32 v6, 0, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[4:5], v[16:17], s[0:1], v[14:15]
	v_fma_f64 v[22:23], v[20:21], s[0:1], -|v[2:3]|
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[16:17], s[2:3], v[4:5]
	v_fma_f64 v[22:23], v[20:21], s[2:3], v[22:23]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[18:19], s[8:9], s[4:5]
	v_fma_f64 v[24:25], v[22:23], s[8:9], s[4:5]
	s_mov_b32 s4, 0x11122322
	s_mov_b32 s5, 0x3f811111
	s_mov_b32 s8, 0x555502a1
	s_mov_b32 s9, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[18:19], v[4:5], s[10:11]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[10:11]
	s_mov_b32 s10, 0x55555511
	s_mov_b32 s11, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[18:19], v[4:5], s[14:15]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[14:15]
	s_mov_b32 s14, 11
	s_mov_b32 s15, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[18:19], v[4:5], s[0:1]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	s_mov_b32 s0, 0x344f1d9b
	s_mov_b32 s1, 0x3f114869
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[18:19], v[4:5], s[2:3]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[2:3]
	s_mov_b32 s2, 0x5ea74bbf
	s_mov_b32 s3, 0xbf42b04c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[18:19], v[4:5], s[4:5]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	v_cmp_neq_f64_e64 s4, |v[2:3]|, 1.0
	v_cmp_class_f64_e64 s5, v[8:9], 0x204
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[18:19], v[4:5], s[8:9]
	v_fma_f64 v[4:5], v[18:19], v[4:5], s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[18:19], v[4:5], s[14:15]
	v_fma_f64 v[30:31], v[18:19], v[4:5], 1.0
	v_fma_f64 v[4:5], v[28:29], v[26:27], v[26:27]
	v_cvt_i32_f64_e32 v26, v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[16:17], v[18:19], v[30:31], 1.0
	v_fma_f64 v[18:19], v[22:23], v[24:25], s[8:9]
	v_fma_f64 v[24:25], v[4:5], s[2:3], s[0:1]
	s_mov_b32 s0, 0x7156ffef
	s_mov_b32 s1, 0x3f49b345
	s_mov_b32 s2, 0xe86ee097
	s_mov_b32 s3, 0xbf2e1427
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[12:13], v[16:17], v26
	v_fma_f64 v[16:17], v[22:23], v[18:19], s[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[18:19], v[4:5], v[24:25], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[14:15]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[14:15]
	v_fma_f64 v[10:11], v[22:23], v[16:17], s[14:15]
	v_mov_b32_e32 v17, 0x7ff80000
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v24, 0x7ff00000, v13, s0
	v_fma_f64 v[13:14], v[4:5], v[18:19], s[2:3]
	s_and_b32 vcc_lo, s1, s0
	s_mov_b32 s2, 0x6f67c4e0
	v_cndmask_b32_e32 v15, 0, v12, vcc_lo
	v_cndmask_b32_e64 v16, 0, v24, s1
	s_mov_b32 s3, 0xbf65f726
	v_cmp_class_f64_e64 s0, v[2:3], 0x204
	v_cmp_neq_f64_e32 vcc_lo, 0, v[8:9]
	v_cvt_i32_f64_e32 v18, v[20:21]
	v_fma_f64 v[6:7], v[15:16], v[6:7], v[15:16]
	v_cmp_class_f64_e64 s1, v[15:16], 0x204
	v_fma_f64 v[10:11], v[22:23], v[10:11], 1.0
	v_fma_f64 v[12:13], v[4:5], v[13:14], s[2:3]
	v_cmp_gt_f64_e64 s2, 0, v[8:9]
	v_cndmask_b32_e64 v14, 0x3ff00000, v17, s0
	v_cndmask_b32_e64 v15, v6, v15, s1
	v_cndmask_b32_e64 v16, v7, v16, s1
	s_and_b32 s1, s4, s5
	s_mov_b32 s4, 0x55555a28
	s_mov_b32 s5, 0x3fb55555
	v_fma_f64 v[6:7], v[22:23], v[10:11], 1.0
	v_cndmask_b32_e64 v17, 0x7ff00000, 0, s2
	s_and_b32 s2, s0, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 vcc_lo, s2, s1
	v_cmp_eq_f64_e64 s1, 0, v[8:9]
	s_mov_b32 s2, 0x1c0f96ad
	s_mov_b32 s3, 0x3f6c71c7
	v_cndmask_b32_e32 v16, v16, v17, vcc_lo
	v_fma_f64 v[10:11], v[4:5], v[12:13], s[2:3]
	v_cmp_nlt_f64_e64 s2, 0, v[2:3]
	v_ldexp_f64 v[12:13], v[6:7], v18
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v7, v16, v14, s1
	s_or_b32 s1, s1, vcc_lo
	v_cmp_o_f64_e64 vcc_lo, |v[2:3]|, v[8:9]
	v_cndmask_b32_e64 v6, v15, 0, s1
	v_cmp_nlt_f64_e64 s1, 0x4090cc00, |v[2:3]|
	v_fma_f64 v[10:11], v[4:5], v[10:11], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cndmask_b32_e32 v7, 0x7ff80000, v7, vcc_lo
	v_cndmask_b32_e64 v9, 0, v13, s1
	v_cndmask_b32_e64 v8, 0, v12, s1
                                        ; implicit-def: $vgpr12_vgpr13
	s_and_saveexec_b32 s1, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s2, exec_lo, s1
	s_cbranch_execz .LBB15_12
; %bb.3:
	v_mul_f64 v[12:13], |v[2:3]|, 0.5
	s_mov_b32 s4, 0x6fdffd2b
	s_mov_b32 s8, 0xf99eb0bb
	s_mov_b32 s10, 0xca1d4f33
	s_mov_b32 s14, 0x2e21c33
	s_mov_b32 s5, 0xbf7e2fe7
	s_mov_b32 s9, 0x3f3e357e
	s_mov_b32 s11, 0x3f5f9c89
	s_mov_b32 s15, 0xbf1b1673
	v_cmp_class_f64_e64 s1, v[2:3], 0x1f8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fract_f64_e32 v[14:15], v[12:13]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[12:13]|
	v_and_b32_e32 v12, 0x7fffffff, v3
	v_add_f64 v[14:15], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v14, 0, v14 :: v_dual_cndmask_b32 v13, 0, v15
	v_cmp_gt_f64_e64 vcc_lo, |v[2:3]|, 1.0
	v_dual_cndmask_b32 v13, v12, v13 :: v_dual_cndmask_b32 v12, v2, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[12:13], v[12:13]
	v_rndne_f64_e32 v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], -0.5, v[12:13]
	v_mul_f64 v[16:17], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], s[8:9], s[4:5]
	v_fma_f64 v[20:21], v[16:17], s[14:15], s[10:11]
	s_mov_b32 s4, 0xd5f14825
	s_mov_b32 s8, 0x7294bff9
	s_mov_b32 s5, 0x3fb50782
	s_mov_b32 s9, 0xbf9a6d1e
	v_mul_f64 v[22:23], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[4:5]
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[8:9]
	s_mov_b32 s4, 0xcdfe9424
	s_mov_b32 s8, 0x67b90b37
	s_mov_b32 s5, 0xbfe32d2c
	s_mov_b32 s9, 0x3fce1f50
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[4:5]
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[8:9]
	s_mov_b32 s4, 0x67754fff
	s_mov_b32 s8, 0x7e3c325b
	s_mov_b32 s5, 0x400466bc
	s_mov_b32 s9, 0xbff55d3c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[4:5]
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[8:9]
	s_mov_b32 s4, 0xe625be09
	s_mov_b32 s8, 0x81b5a67
	s_mov_b32 s5, 0xc014abbc
	s_mov_b32 s9, 0x40103c1f
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[4:5]
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[8:9]
	s_mov_b32 s4, 0xc9be45de
	s_mov_b32 s5, 0xc013bd3c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[22:23], v[18:19]
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[4:5]
	v_cvt_i32_f64_e32 v22, v[14:15]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x400921fb
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[12:13], s[4:5], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[16:17], v[20:21], 1.0
	v_and_b32_e32 v16, 1, v22
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_eq_u32_e32 vcc_lo, 0, v16
	v_cndmask_b32_e32 v12, v14, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v13, v15, v13 :: v_dual_lshlrev_b32 v14, 30, v22
	v_cndmask_b32_e64 v12, 0, v12, s1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v14, v14, v3
	v_and_b32_e32 v14, 0x80000000, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v13, v13, v14
	v_cndmask_b32_e64 v13, 0x7ff80000, v13, s1
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[14:15], -v[2:3], v[12:13]
                                        ; implicit-def: $vgpr12_vgpr13
	v_cmpx_nlt_f64_e32 0xc0655000, v[2:3]
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB15_9
; %bb.4:
	s_mov_b32 s3, exec_lo
                                        ; implicit-def: $vgpr12_vgpr13
	v_cmpx_nlt_f64_e32 0xc0670000, v[2:3]
	s_xor_b32 s3, exec_lo, s3
; %bb.5:
	s_delay_alu instid0(VALU_DEP_3)
	v_bfi_b32 v13, 0x7fffffff, 0, v15
	v_mov_b32_e32 v12, 0
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr10_vgpr11
                                        ; implicit-def: $vgpr14_vgpr15
; %bb.6:
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB15_8
; %bb.7:
	v_mul_f64 v[8:9], v[8:9], v[6:7]
	v_mul_f64 v[4:5], v[4:5], v[10:11]
	s_mov_b32 s4, 0x1ff62706
	s_mov_b32 s5, 0x3ff40d93
	v_mul_f64 v[6:7], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[8:9], v[4:5], v[8:9]
	v_div_scale_f64 v[8:9], null, v[4:5], v[4:5], s[4:5]
	v_div_scale_f64 v[16:17], vcc_lo, s[4:5], v[4:5], s[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[16:17], v[10:11]
	v_fma_f64 v[8:9], -v[8:9], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[12:13]
	v_div_fixup_f64 v[4:5], v[8:9], v[4:5], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[8:9], null, v[6:7], v[6:7], v[4:5]
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_div_scale_f64 v[12:13], vcc_lo, v[4:5], v[6:7], v[4:5]
	v_mul_f64 v[14:15], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[8:9], v[14:15], v[12:13]
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[12:13], v[8:9], v[6:7], v[4:5]
.LBB15_8:
	s_or_b32 exec_lo, exec_lo, s3
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr14_vgpr15
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr10_vgpr11
.LBB15_9:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB15_11
; %bb.10:
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[8:9], v[14:15]
	v_mul_f64 v[4:5], v[4:5], v[10:11]
	s_mov_b32 s4, 0x1ff62706
	s_mov_b32 s5, 0x3ff40d93
	v_mul_f64 v[8:9], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[4:5], v[6:7], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], s[4:5]
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_div_scale_f64 v[10:11], vcc_lo, s[4:5], v[4:5], s[4:5]
	v_mul_f64 v[12:13], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[12:13], v[10:11]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[12:13], v[6:7], v[4:5], s[4:5]
.LBB15_11:
	s_or_b32 exec_lo, exec_lo, s1
	v_fract_f64_e32 v[4:5], v[2:3]
	v_cmp_u_f64_e64 s1, v[2:3], v[2:3]
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr10_vgpr11
                                        ; implicit-def: $vgpr2_vgpr3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_eq_f64_e32 vcc_lo, 0, v[4:5]
                                        ; implicit-def: $vgpr4_vgpr5
	s_or_b32 s0, s0, vcc_lo
	s_or_b32 s0, s1, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e64 v13, v13, 0x7ff80000, s0
	v_cndmask_b32_e64 v12, v12, 0, s0
.LBB15_12:
	s_and_not1_saveexec_b32 s0, s2
	s_cbranch_execz .LBB15_14
; %bb.13:
	s_mov_b32 s2, 0x1ff62706
	s_mov_b32 s3, 0x40040d93
	v_mul_f64 v[4:5], v[4:5], v[10:11]
	v_mul_f64 v[8:9], v[8:9], s[2:3]
	s_mov_b32 s2, 0xe561f646
	s_mov_b32 s3, 0x406573fa
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_nlt_f64_e32 vcc_lo, s[2:3], v[2:3]
	v_mul_f64 v[8:9], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[4:5], v[6:7], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v13, 0x7ff00000, v5, vcc_lo
	v_cndmask_b32_e32 v12, 0, v4, vcc_lo
.LBB15_14:
	s_or_b32 exec_lo, exec_lo, s0
                                        ; implicit-def: $vgpr2_vgpr3
.LBB15_15:
	s_and_not1_saveexec_b32 s1, s12
	s_cbranch_execz .LBB15_37
; %bb.16:
	s_mov_b32 s0, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
	v_cmpx_nlt_f64_e32 0, v[2:3]
	s_xor_b32 s0, exec_lo, s0
	s_cbranch_execz .LBB15_24
; %bb.17:
	v_dual_mov_b32 v7, v3 :: v_dual_mov_b32 v6, v2
	v_dual_mov_b32 v9, v3 :: v_dual_mov_b32 v8, v2
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_f64_e32 0xbff80000, v[2:3]
	s_cbranch_execz .LBB15_21
; %bb.18:
	v_dual_mov_b32 v7, v3 :: v_dual_mov_b32 v6, v2
	v_dual_mov_b32 v9, v3 :: v_dual_mov_b32 v8, v2
	s_mov_b32 s3, 0
.LBB15_19:                              ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[8:9], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[4:5], 1.0
	v_fma_f64 v[6:7], v[6:7], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_ngt_f64_e32 vcc_lo, 0xbff80000, v[8:9]
	s_or_b32 s3, vcc_lo, s3
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execnz .LBB15_19
; %bb.20:
	s_or_b32 exec_lo, exec_lo, s3
.LBB15_21:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s2
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_f64_e32 -0.5, v[8:9]
	s_cbranch_execz .LBB15_23
; %bb.22:
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[8:9], 1.0
.LBB15_23:
	s_or_b32 exec_lo, exec_lo, s2
.LBB15_24:
	s_or_saveexec_b32 s2, s0
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0x3ff00000
	s_xor_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB15_32
; %bb.25:
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v9, v3
	v_dual_mov_b32 v5, 0x3ff00000 :: v_dual_mov_b32 v8, v2
	s_mov_b32 s0, exec_lo
	v_cmpx_lt_f64_e32 0x40040000, v[2:3]
	s_cbranch_execz .LBB15_29
; %bb.26:
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v9, v3
	v_dual_mov_b32 v5, 0x3ff00000 :: v_dual_mov_b32 v8, v2
	s_mov_b32 s3, 0
.LBB15_27:                              ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -1.0
	v_fma_f64 v[4:5], v[4:5], v[8:9], -v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[6:7], -1.0
	v_fma_f64 v[4:5], v[4:5], v[6:7], -v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_nlt_f64_e32 vcc_lo, 0x40040000, v[8:9]
	s_or_b32 s3, vcc_lo, s3
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execnz .LBB15_27
; %bb.28:
	s_or_b32 exec_lo, exec_lo, s3
.LBB15_29:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s0
	s_mov_b32 s0, exec_lo
	v_cmpx_lt_f64_e32 0x3ff80000, v[8:9]
	s_cbranch_execz .LBB15_31
; %bb.30:
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[4:5], v[4:5], v[8:9], -v[4:5]
	v_add_f64 v[8:9], v[8:9], -1.0
.LBB15_31:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[8:9], -1.0
	v_cmp_nle_f64_e32 vcc_lo, 0.5, v[2:3]
	v_cmp_gt_f64_e64 s0, 0.5, v[2:3]
	v_dual_cndmask_b32 v9, v11, v9 :: v_dual_cndmask_b32 v8, v10, v8
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v7, 0x3ff00000, v3, s0
	v_cndmask_b32_e64 v6, 0, v2, s0
.LBB15_32:
	s_or_b32 exec_lo, exec_lo, s2
	v_cmp_ngt_f64_e64 s2, 0, v[2:3]
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB15_34
; %bb.33:
	v_floor_f64_e32 v[10:11], v[2:3]
	s_mov_b32 s4, -1
	s_mov_b32 s5, 0x3fefffff
	s_and_not1_b32 s2, s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[2:3], -v[10:11]
	v_min_f64 v[10:11], v[10:11], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[10:11]
	s_and_b32 s3, vcc_lo, exec_lo
	s_or_b32 s2, s2, s3
.LBB15_34:
	s_or_b32 exec_lo, exec_lo, s0
	v_mov_b32_e32 v12, 0
	v_mov_b32_e32 v13, 0x7ff80000
	s_and_saveexec_b32 s0, s2
	s_cbranch_execz .LBB15_36
; %bb.35:
	s_mov_b32 s2, 0xa0be3cd3
	s_mov_b32 s4, 0xfeec7b9a
	s_mov_b32 s3, 0x3eb31854
	s_mov_b32 s5, 0xbe8aed75
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], s[4:5], s[2:3]
	s_mov_b32 s2, 0x6a97a8b7
	s_mov_b32 s3, 0xbeb5037d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0xf2cdbcfb
	s_mov_b32 s3, 0xbef51d67
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x2ac5112d
	s_mov_b32 s3, 0x3f20c8ab
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0xe9b5e149
	s_mov_b32 s3, 0xbf2c364c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x3a39f929
	s_mov_b32 s3, 0xbf531711
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x501178a3
	s_mov_b32 s3, 0x3f7d919c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x282da690
	s_mov_b32 s3, 0xbf83b4af
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x3bf2cd0
	s_mov_b32 s3, 0xbfa59af1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x20b432cc
	s_mov_b32 s3, 0x3fc55123
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x8fa28886
	s_mov_b32 s3, 0xbfa5815e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x26afa24
	s_mov_b32 s3, 0xbfe4fcf4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0xfc6fb61c
	s_mov_b32 s3, 0x3fe2788c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_brev_b32 s2, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[8:9], v[10:11]
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[8:9], null, v[6:7], v[6:7], v[4:5]
	v_div_scale_f64 v[14:15], vcc_lo, v[4:5], v[6:7], v[4:5]
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[8:9], v[12:13], v[14:15]
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[12:13]
	v_cmp_eq_f64_e32 vcc_lo, 0, v[2:3]
	v_and_or_b32 v10, v3, s2, 0x7ff00000
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[8:9], v[6:7], v[4:5]
	v_cndmask_b32_e32 v13, v5, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v12, v4, 0, vcc_lo
.LBB15_36:
	s_or_b32 exec_lo, exec_lo, s0
.LBB15_37:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s1
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[12:13], off
.LBB15_38:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_tgamma_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_inst_pref_size 39
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end15:
	.size	mx_tgamma_kernel, .Lfunc_end15-mx_tgamma_kernel
                                        ; -- End function
	.set mx_tgamma_kernel.num_vgpr, 32
	.set mx_tgamma_kernel.num_agpr, 0
	.set mx_tgamma_kernel.numbered_sgpr, 16
	.set mx_tgamma_kernel.num_named_barrier, 0
	.set mx_tgamma_kernel.private_seg_size, 0
	.set mx_tgamma_kernel.uses_vcc, 1
	.set mx_tgamma_kernel.uses_flat_scratch, 0
	.set mx_tgamma_kernel.has_dyn_sized_stack, 0
	.set mx_tgamma_kernel.has_recursion, 0
	.set mx_tgamma_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4956
; TotalNumSgprs: 18
; NumVgprs: 32
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 18
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
	.protected	mx_lgamma_kernel        ; -- Begin function mx_lgamma_kernel
	.globl	mx_lgamma_kernel
	.p2align	8
	.type	mx_lgamma_kernel,@function
mx_lgamma_kernel:                       ; @mx_lgamma_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB16_36
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr6_vgpr7
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_dual_mov_b32 v4, v2 :: v_dual_and_b32 v5, 0x7fffffff, v3
	v_cmpx_lt_u32_e32 0x3f6fffff, v5
	s_xor_b32 s3, exec_lo, s0
	s_cbranch_execz .LBB16_27
; %bb.2:
                                        ; implicit-def: $vgpr6_vgpr7
	s_mov_b32 s0, exec_lo
	v_cmpx_lt_u32_e32 0x3fffffff, v5
	s_xor_b32 s1, exec_lo, s0
	s_cbranch_execz .LBB16_12
; %bb.3:
                                        ; implicit-def: $vgpr6_vgpr7
	s_mov_b32 s0, exec_lo
	v_cmpx_lt_u32_e32 0x401fffff, v5
	s_xor_b32 s2, exec_lo, s0
	s_cbranch_execz .LBB16_9
; %bb.4:
                                        ; implicit-def: $vgpr6_vgpr7
	s_mov_b32 s0, exec_lo
	v_cmpx_lt_u32_e32 0x438fffff, v5
	s_xor_b32 s4, exec_lo, s0
	s_cbranch_execz .LBB16_6
; %bb.5:
	v_frexp_mant_f64_e64 v[6:7], |v[2:3]|
	s_mov_b32 s9, 0x3fe55555
	s_mov_b32 s8, 0x55555555
	s_mov_b32 s10, 0x6b47b09a
	s_mov_b32 s12, 0xbf559e2b
	s_mov_b32 s11, 0x3fc38538
	s_mov_b32 s13, 0x3fc3ab76
	v_cmp_neq_f64_e64 s0, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[8:9], v[6:7]
	s_mov_b32 s8, 0x55555780
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v4
	v_frexp_exp_i32_f64_e32 v4, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	v_fma_f64 v[14:15], v[10:11], s[12:13], s[10:11]
	s_mov_b32 s10, 0xd7f4df2e
	s_mov_b32 s11, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[10:11]
	s_mov_b32 s10, 0x16291751
	s_mov_b32 s11, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[10:11]
	s_mov_b32 s10, 0x9b27acf1
	s_mov_b32 s11, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[10:11]
	s_mov_b32 s10, 0x998ef7b6
	s_mov_b32 s11, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[8:9]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s8, 0xfefa39ef
	s_mov_b32 s9, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[16:17], s[8:9]
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[8:9], -v[18:19]
	s_mov_b32 s8, 0x3b39803f
	s_mov_b32 s9, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[8:9], v[14:15]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0x7ff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0xfff00000, v4, s0
	v_fma_f64 v[6:7], |v[2:3]|, v[6:7], -|v[2:3]|
.LBB16_6:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB16_8
; %bb.7:
	v_frexp_mant_f64_e64 v[6:7], |v[2:3]|
	s_mov_b32 s9, 0x3fe55555
	s_mov_b32 s8, 0x55555555
	s_mov_b32 s10, 0x6b47b09a
	s_mov_b32 s12, 0xbf559e2b
	s_mov_b32 s11, 0x3fc38538
	s_mov_b32 s13, 0x3fc3ab76
	v_cmp_neq_f64_e64 s0, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[8:9], v[6:7]
	s_mov_b32 s8, 0x55555780
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v4
	v_frexp_exp_i32_f64_e32 v4, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	v_fma_f64 v[14:15], v[10:11], s[12:13], s[10:11]
	s_mov_b32 s10, 0xd7f4df2e
	s_mov_b32 s11, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[10:11]
	s_mov_b32 s10, 0x16291751
	s_mov_b32 s11, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[10:11]
	s_mov_b32 s10, 0x9b27acf1
	s_mov_b32 s11, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[10:11]
	s_mov_b32 s10, 0x998ef7b6
	s_mov_b32 s11, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[10:11]
	s_mov_b32 s10, 0xb9e43e4
	s_mov_b32 s11, 0xbf5ab89d
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[8:9]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s8, 0xfefa39ef
	s_mov_b32 s9, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v4
	v_mov_b32_e32 v4, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_div_scale_f64 v[28:29], vcc_lo, 1.0, v[4:5], 1.0
	v_add_f64 v[12:13], v[14:15], v[10:11]
	v_mul_f64 v[18:19], v[16:17], s[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_div_scale_f64 v[14:15], null, v[4:5], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[20:21], v[16:17], s[8:9], -v[18:19]
	s_mov_b32 s8, 0x3b39803f
	s_mov_b32 s9, 0x3c7abc9e
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[10:11], v[14:15]
	v_fma_f64 v[16:17], v[16:17], s[8:9], v[20:21]
	s_mov_b32 s8, 0x4cdad5d1
	s_mov_b32 s9, 0x3f4b67ba
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[20:21], v[18:19], v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[14:15], v[10:11], 1.0
	v_add_f64 v[22:23], v[12:13], v[6:7]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[10:11], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[20:21], v[22:23]
	v_add_f64 v[12:13], v[22:23], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	v_fma_f64 v[24:25], -v[14:15], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[10:11], -v[20:21]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[24:25], v[8:9]
	v_add_f64 v[24:25], v[10:11], -v[26:27]
	v_add_f64 v[12:13], v[22:23], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[30:31], v[28:29], v[8:9]
	v_add_f64 v[18:19], v[20:21], -v[24:25]
	v_add_f64 v[20:21], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], -v[14:15], v[30:31], v[28:29]
	v_add_f64 v[12:13], v[12:13], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_div_fmas_f64 v[8:9], v[14:15], v[8:9], v[30:31]
	v_add_f64 v[14:15], v[20:21], -v[16:17]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[20:21], v[12:13]
	v_div_fixup_f64 v[8:9], v[8:9], |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[20:21], -v[14:15]
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	v_add_f64 v[20:21], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[22:23], v[8:9], v[8:9]
	v_add_f64 v[14:15], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[20:21], -v[10:11]
	v_fma_f64 v[16:17], v[22:23], s[10:11], s[8:9]
	s_mov_b32 s8, 0x8c0fe741
	s_mov_b32 s9, 0xbf4380cb
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[14:15]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[22:23], v[16:17], s[8:9]
	s_mov_b32 s8, 0x98cf38b6
	s_mov_b32 s9, 0x3f4a019f
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[22:23], v[12:13], s[8:9]
	s_mov_b32 s8, 0x16b02e5c
	s_mov_b32 s9, 0xbf66c16c
	v_add_f64 v[12:13], |v[2:3]|, -0.5
	v_add_f64 v[6:7], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[22:23], v[10:11], s[8:9]
	s_mov_b32 s8, 0x5555553b
	s_mov_b32 s9, 0x3fb55555
	v_add_f64 v[6:7], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[22:23], v[10:11], s[8:9]
	s_mov_b32 s8, 0x90c97d69
	s_mov_b32 s9, 0x3fdacfe3
	v_cndmask_b32_e32 v4, 0x7ff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[8:9], v[10:11], s[8:9]
	v_cndmask_b32_e64 v10, 0xfff00000, v4, s0
	v_cndmask_b32_e32 v9, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[12:13], v[9:10], v[7:8]
.LBB16_8:
	s_or_b32 exec_lo, exec_lo, s4
.LBB16_9:
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execz .LBB16_11
; %bb.10:
	v_cvt_i32_f64_e32 v4, v[4:5]
	s_mov_b32 s5, 0x3fe55555
	s_mov_b32 s4, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[6:7], v4
	v_cmp_lt_i32_e32 vcc_lo, 2, v4
	v_cmp_lt_i32_e64 s0, 3, v4
	v_add_f64 v[6:7], |v[2:3]|, -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[6:7], 2.0
	v_add_f64 v[10:11], 0x40080000, v[6:7]
	v_add_f64 v[12:13], v[6:7], 4.0
	v_cndmask_b32_e32 v9, 0x3ff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cndmask_b32_e64 v11, 0x3ff00000, v11, s0
	v_cndmask_b32_e64 v10, 0, v10, s0
	v_cmp_lt_i32_e32 vcc_lo, 4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], 0x40140000, v[6:7]
	v_cndmask_b32_e32 v13, 0x3ff00000, v13, vcc_lo
	v_cndmask_b32_e32 v12, 0, v12, vcc_lo
	v_cmp_lt_i32_e32 vcc_lo, 5, v4
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	v_add_f64 v[12:13], 0x40180000, v[6:7]
	v_cndmask_b32_e32 v11, 0x3ff00000, v11, vcc_lo
	v_cndmask_b32_e32 v10, 0, v10, vcc_lo
	v_cmp_lt_i32_e32 vcc_lo, 6, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[8:9]
	v_cndmask_b32_e32 v11, 0x3ff00000, v13, vcc_lo
	v_cndmask_b32_e32 v10, 0, v12, vcc_lo
	v_mul_f64 v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[10:11], v[8:9]
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[10:11]
	s_mov_b32 s4, 0x55555780
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[10:11], v4
	v_frexp_exp_i32_f64_e32 v4, v[8:9]
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[18:19], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_cvt_f64_i32_e32 v[24:25], v4
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[12:13]
	v_add_f64 v[20:21], v[12:13], -1.0
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	v_mul_f64 v[16:17], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[22:23], v[12:13], v[16:17]
	v_fma_f64 v[12:13], v[16:17], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[16:17], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[22:23], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[18:19], -v[12:13]
	v_add_f64 v[22:23], v[12:13], -v[22:23]
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[10:11], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	v_add_f64 v[12:13], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[14:15], v[12:13], v[12:13]
	v_ldexp_f64 v[22:23], v[12:13], 1
	v_fma_f64 v[18:19], v[14:15], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[20:21], v[12:13], v[14:15]
	s_mov_b32 s10, 0xa5b38140
	s_mov_b32 s11, 0x3edebaf7
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	v_ldexp_f64 v[10:11], v[10:11], 1
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	s_mov_b32 s8, 0xdd17e945
	s_mov_b32 s9, 0x3f00bfec
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[14:15], v[18:19], s[4:5]
	s_mov_b32 s4, 0x7368f239
	s_mov_b32 s5, 0x3f5e26b6
	v_fma_f64 v[18:19], v[6:7], s[8:9], s[4:5]
	s_mov_b32 s4, 0x7e939961
	s_mov_b32 s5, 0x3f9b481c
	s_mov_b32 s8, 0xca41a95b
	s_mov_b32 s9, 0x3f497dda
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_mul_f64 v[14:15], v[20:21], v[14:15]
	v_fma_f64 v[20:21], v[6:7], s[10:11], s[8:9]
	s_mov_b32 s8, 0x742ed475
	s_mov_b32 s9, 0x3f9317ea
	v_fma_f64 v[18:19], v[6:7], v[18:19], s[4:5]
	s_mov_b32 s4, 0xbee5f2f7
	s_mov_b32 s5, 0x3fc2bb9c
	s_mov_b32 s10, 0xccfbdf27
	s_mov_b32 s11, 0x3fc601ed
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[22:23], v[14:15]
	v_fma_f64 v[20:21], v[6:7], v[20:21], s[8:9]
	s_mov_b32 s8, 0x4f139f59
	s_mov_b32 s9, 0x3fd4d98f
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[6:7], v[18:19], s[4:5]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0x3fe62e42
	v_mul_f64 v[26:27], v[24:25], s[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[16:17], -v[22:23]
	v_fma_f64 v[20:21], v[6:7], v[20:21], s[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[18:19], v[6:7], v[18:19], s[8:9]
	s_mov_b32 s8, 0x93d3dcdc
	s_mov_b32 s9, 0x3fe71a18
	v_fma_f64 v[22:23], v[24:25], s[4:5], -v[26:27]
	s_mov_b32 s4, 0x36e20878
	s_mov_b32 s5, 0x3fcb848b
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_fma_f64 v[14:15], v[6:7], v[20:21], s[8:9]
	s_mov_b32 s8, 0x62c4ab74
	s_mov_b32 s9, 0x3ff645a7
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[18:19], v[6:7], v[18:19], s[4:5]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[24:25], s[4:5], v[22:23]
	s_mov_b32 s4, 0xe37db0c8
	s_mov_b32 s5, 0xbfb3c467
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_fma_f64 v[12:13], v[6:7], v[14:15], s[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[14:15], v[6:7], v[18:19], s[4:5]
	v_add_f64 v[18:19], v[26:27], v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[22:23], v[16:17], v[10:11]
	v_fma_f64 v[12:13], v[6:7], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[14:15], v[6:7], v[14:15]
	v_add_f64 v[26:27], v[18:19], -v[26:27]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[24:25], v[18:19], v[22:23]
	v_add_f64 v[16:17], v[22:23], -v[16:17]
	v_div_scale_f64 v[28:29], null, v[12:13], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_add_f64 v[30:31], v[24:25], -v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_rcp_f64_e32 v[32:33], v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[34:35], v[24:25], -v[30:31]
	v_add_f64 v[16:17], v[22:23], -v[30:31]
	v_add_f64 v[26:27], v[20:21], v[10:11]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[36:37], -v[28:29], v[32:33], 1.0
	v_add_f64 v[18:19], v[18:19], -v[34:35]
	v_add_f64 v[30:31], v[26:27], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[22:23], v[32:33], v[36:37], v[32:33]
	v_div_scale_f64 v[32:33], vcc_lo, v[14:15], v[12:13], v[14:15]
	v_add_f64 v[16:17], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[10:11], -v[30:31]
	v_fma_f64 v[18:19], -v[28:29], v[22:23], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[26:27], v[16:17]
	v_fma_f64 v[18:19], v[22:23], v[18:19], v[22:23]
	v_add_f64 v[22:23], v[26:27], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[24:25], v[16:17]
	v_mul_f64 v[34:35], v[32:33], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], -v[22:23]
	v_add_f64 v[22:23], v[26:27], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[24:25], -v[28:29], v[34:35], v[32:33]
	v_add_f64 v[10:11], v[10:11], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[16:17], -v[22:23]
	v_div_fmas_f64 v[18:19], v[24:25], v[18:19], v[34:35]
	v_cmp_class_f64_e64 vcc_lo, v[8:9], 0x204
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_div_fixup_f64 v[12:13], v[18:19], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[26:27], v[10:11]
	v_fma_f64 v[6:7], v[6:7], 0.5, v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v4, v10, v8, vcc_lo
	v_cndmask_b32_e32 v10, v11, v9, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v11, 0x7ff80000, v10, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[8:9]
	v_cndmask_b32_e32 v10, 0, v4, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[8:9]
	v_cndmask_b32_e32 v11, 0xfff00000, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[10:11]
.LBB16_11:
	s_or_b32 exec_lo, exec_lo, s2
.LBB16_12:
	s_and_not1_saveexec_b32 s4, s1
	s_cbranch_execz .LBB16_26
; %bb.13:
                                        ; implicit-def: $vgpr4
                                        ; implicit-def: $vgpr8_vgpr9
	s_mov_b32 s0, exec_lo
	v_cmpx_lt_u32_e32 0x3feccccc, v5
	s_xor_b32 s1, exec_lo, s0
	s_cbranch_execz .LBB16_15
; %bb.14:
	s_mov_b32 s8, 0x6356be3f
	s_mov_b32 s9, 0xbff762d8
	v_add_f64 v[6:7], -|v[2:3]|, 2.0
	v_add_f64 v[8:9], |v[2:3]|, s[8:9]
	v_add_f64 v[10:11], |v[2:3]|, -1.0
	v_cmp_gt_u32_e32 vcc_lo, 0x3ffbb4c3, v5
	v_cmp_gt_u32_e64 s0, 0x3ff3b4c4, v5
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v4, v6, v8, vcc_lo
	v_cndmask_b32_e32 v6, v7, v9, vcc_lo
	v_cndmask_b32_e64 v7, 0, 1, vcc_lo
	v_cndmask_b32_e64 v8, v4, v10, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v9, v6, v11, s0
	v_cndmask_b32_e64 v4, v7, 2, s0
.LBB16_15:
	s_or_saveexec_b32 s5, s1
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0
	s_xor_b32 exec_lo, exec_lo, s5
	s_cbranch_execz .LBB16_17
; %bb.16:
	v_frexp_mant_f64_e64 v[6:7], |v[2:3]|
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	v_cmp_gt_u32_e64 s2, 0x3fcda661, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[6:7]
	s_mov_b32 s0, 0x55555780
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v4
	v_frexp_exp_i32_f64_e32 v4, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	v_fma_f64 v[14:15], v[10:11], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
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
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[16:17], s[0:1]
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[0:1], -v[18:19]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[0:1], v[14:15]
	s_mov_b32 s0, 0x8d5af8fc
	s_mov_b32 s1, 0xbfdd8b61
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[10:11], |v[2:3]|, s[0:1]
	v_cmp_neq_f64_e64 s0, 0, v[2:3]
	v_cmp_gt_u32_e64 s1, 0x3fe76944, v5
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], -|v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_xor_b32_e32 v4, 0x80000000, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v7, v8, v10, s1
	v_cndmask_b32_e64 v8, v9, v11, s1
	v_cndmask_b32_e64 v10, 0, 1, s1
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v4, 0xfff00000, v4, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e64 v9, v8, v5, s2
	v_cndmask_b32_e64 v8, v7, v2, s2
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cndmask_b32_e64 v7, 0x7ff00000, v4, s0
	v_cndmask_b32_e64 v4, v10, 2, s2
.LBB16_17:
	s_or_b32 exec_lo, exec_lo, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_mov_b32 s0, exec_lo
                                        ; implicit-def: $vgpr12_vgpr13
	v_cmpx_lt_i32_e32 1, v4
	s_xor_b32 s0, exec_lo, s0
	s_cbranch_execz .LBB16_19
; %bb.18:
	s_mov_b32 s8, 0xf6010924
	s_mov_b32 s10, 0xbf2bab09
	s_mov_b32 s9, 0x3fcd4eae
	s_mov_b32 s11, 0x3f8b678b
	s_mov_b32 s12, 0x57d0cf61
	v_fma_f64 v[10:11], v[8:9], s[10:11], s[8:9]
	s_mov_b32 s8, 0x44ea8450
	s_mov_b32 s10, 0xd6537c88
	s_mov_b32 s9, 0x3fef4976
	s_mov_b32 s11, 0x3fbaae55
	s_mov_b32 s13, 0x3f6a5abb
                                        ; implicit-def: $vgpr4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[12:13], s[10:11]
	s_mov_b32 s10, 0xe45050af
	s_mov_b32 s11, 0x3fe89dfb
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0xd119bd6f
	s_mov_b32 s9, 0x3ff7475c
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[10:11]
	s_mov_b32 s10, 0xa42b18f5
	s_mov_b32 s11, 0x40010725
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0x8b005dff
	s_mov_b32 s9, 0x3fe4401e
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[10:11]
	s_mov_b32 s10, 0xc2bd619c
	s_mov_b32 s11, 0x4003a5d7
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0xe37db0c8
	s_mov_b32 s9, 0xbfb3c467
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[10:11]
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], v[12:13], 1.0
	v_mul_f64 v[10:11], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[14:15], null, v[12:13], v[12:13], v[10:11]
	v_rcp_f64_e32 v[16:17], v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	v_div_scale_f64 v[18:19], vcc_lo, v[10:11], v[12:13], v[10:11]
	v_mul_f64 v[20:21], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[14:15], v[20:21], v[18:19]
	v_div_fmas_f64 v[14:15], v[14:15], v[16:17], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[10:11], v[14:15], v[12:13], v[10:11]
	v_fma_f64 v[12:13], v[8:9], -0.5, v[10:11]
                                        ; implicit-def: $vgpr8_vgpr9
.LBB16_19:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB16_25
; %bb.20:
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	s_mov_b32 s1, exec_lo
                                        ; implicit-def: $vgpr12_vgpr13
	v_cmpx_ne_u32_e32 1, v4
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB16_22
; %bb.21:
	s_mov_b32 s8, 0x987dfb07
	s_mov_b32 s10, 0x90a45837
	s_mov_b32 s9, 0x3f1c5088
	s_mov_b32 s11, 0x3f07858e
	s_mov_b32 s12, 0x89b99c00
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], s[10:11], s[8:9]
	s_mov_b32 s8, 0xed10e54d
	s_mov_b32 s10, 0x428cfa52
	s_mov_b32 s9, 0x3f2cf2ec
	s_mov_b32 s11, 0x3efa7074
	s_mov_b32 s13, 0x3f40b6c6
	v_fma_f64 v[14:15], v[10:11], s[10:11], s[8:9]
	s_mov_b32 s8, 0x116f3f5d
	s_mov_b32 s10, 0xccb7926b
	s_mov_b32 s9, 0x3f538a94
	s_mov_b32 s11, 0x3f67add8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[12:13]
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0xb68fefe8
	s_mov_b32 s9, 0x3f7e404f
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[10:11]
	s_mov_b32 s10, 0xac92547b
	s_mov_b32 s11, 0x3f951322
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0x1a5562a7
	s_mov_b32 s9, 0x3fb13e00
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[10:11]
	s_mov_b32 s10, 0xc4a60fad
	s_mov_b32 s11, 0x3fd4a34c
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0xe37db0c8
	s_mov_b32 s9, 0x3fb3c467
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[10:11]
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[10:11], v[12:13]
	v_fma_f64 v[10:11], v[8:9], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[12:13], v[8:9], -0.5, v[10:11]
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr10_vgpr11
.LBB16_22:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB16_24
; %bb.23:
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[12:13], v[8:9], v[10:11]
	s_mov_b32 s8, 0xef61a8e9
	s_mov_b32 s10, 0xecc38c38
	s_mov_b32 s12, 0x9c73e0ec
	s_mov_b32 s14, 0xe8c2d3f4
	s_mov_b32 s9, 0x3f4cdf0c
	s_mov_b32 s11, 0xbf347f24
	s_mov_b32 s13, 0xbf41a610
	s_mov_b32 s15, 0x3f35fd3e
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], s[10:11], s[8:9]
	v_fma_f64 v[16:17], v[12:13], s[14:15], s[12:13]
	s_mov_b32 s8, 0xb3e914d7
	s_mov_b32 s10, 0x2e15c915
	s_mov_b32 s9, 0xbf6e2eff
	s_mov_b32 s11, 0x3f6282d3
	s_mov_b32 s12, 0x970af9ec
	s_mov_b32 s14, 0xba91ec6a
	s_mov_b32 s13, 0x3f9266e7
	s_mov_b32 s15, 0xbf851f9f
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[8:9]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[10:11]
	s_mov_b32 s8, 0xbf2d1af1
	s_mov_b32 s10, 0x6c0ebbf7
	s_mov_b32 s9, 0xbf56fe8e
	s_mov_b32 s11, 0x3f34af6d
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[12:13], s[10:11], s[8:9]
	s_mov_b32 s8, 0xe370e344
	s_mov_b32 s10, 0x8dc6c509
	s_mov_b32 s9, 0x3f78fce0
	s_mov_b32 s11, 0xbfc2e427
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[14:15]
	s_mov_b32 s12, 0x94d5419b
	s_mov_b32 s13, 0x3fb08b42
	v_fma_f64 v[18:19], v[12:13], v[18:19], s[8:9]
	s_mov_b32 s8, 0xdf35b713
	s_mov_b32 s9, 0xbfa0c9a8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[10:11]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[12:13]
	s_mov_b32 s10, 0xa48a971f
	s_mov_b32 s11, 0xbc50c7ca
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[12:13], v[18:19], s[8:9]
	s_mov_b32 s8, 0xc8ee38a2
	s_mov_b32 s9, 0x3fdef72b
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[18:19], s[8:9]
	s_mov_b32 s8, 0xbcc38a42
	s_mov_b32 s9, 0xbfbf19b9
	v_fma_f64 v[8:9], v[12:13], -v[8:9], s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[10:11], v[14:15], -v[8:9]
	v_add_f64 v[12:13], v[8:9], s[8:9]
.LBB16_24:
	s_or_b32 exec_lo, exec_lo, s1
.LBB16_25:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s0
	v_add_f64 v[6:7], v[6:7], v[12:13]
.LBB16_26:
	s_or_b32 exec_lo, exec_lo, s4
.LBB16_27:
	s_and_not1_saveexec_b32 s1, s3
	s_cbranch_execz .LBB16_29
; %bb.28:
	v_frexp_mant_f64_e64 v[6:7], |v[2:3]|
	s_mov_b32 s3, 0x3fe55555
	s_mov_b32 s2, 0x55555555
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	v_cmp_neq_f64_e64 s0, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[6:7]
	s_mov_b32 s2, 0x55555780
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v4
	v_frexp_exp_i32_f64_e32 v4, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	v_fma_f64 v[14:15], v[10:11], s[8:9], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[4:5]
	s_mov_b32 s4, 0x17aa6149
	s_mov_b32 s5, 0xbfca8b9c
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[2:3]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s2, 0xfefa39ef
	s_mov_b32 s3, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[16:17], s[2:3]
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[2:3], -v[18:19]
	s_mov_b32 s2, 0x3b39803f
	s_mov_b32 s3, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[2:3], v[14:15]
	s_mov_b32 s2, 0x2ac7d848
	s_mov_b32 s3, 0x3fd15132
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_fma_f64 v[10:11], |v[2:3]|, s[4:5], s[2:3]
	s_mov_b32 s2, 0x5beab2d7
	s_mov_b32 s3, 0xbfd9a4d5
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], |v[2:3]|, v[10:11], s[2:3]
	s_mov_b32 s2, 0x625307d3
	s_mov_b32 s3, 0x3fea51a6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_fma_f64 v[8:9], |v[2:3]|, v[8:9], s[2:3]
	s_mov_b32 s2, 0xfc6fb619
	s_mov_b32 s3, 0xbfe2788c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v4, 0x80000000, v7
	v_fma_f64 v[7:8], |v[2:3]|, v[8:9], s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0xfff00000, v4, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v9, 0, v6, vcc_lo
	v_cndmask_b32_e64 v10, 0x7ff00000, v4, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], |v[2:3]|, v[7:8], v[9:10]
.LBB16_29:
	s_or_b32 exec_lo, exec_lo, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	v_cmpx_le_f64_e32 0, v[2:3]
	s_xor_b32 s1, exec_lo, s0
	s_cbranch_execz .LBB16_31
; %bb.30:
	v_cmp_eq_f64_e32 vcc_lo, 1.0, v[2:3]
	v_cmp_eq_f64_e64 s0, 2.0, v[2:3]
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e64 v7, v7, 0, s0
	v_cndmask_b32_e64 v6, v6, 0, s0
.LBB16_31:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB16_35
; %bb.32:
	v_add_nc_u32_e32 v4, 0xc32fffff, v5
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_u32_e32 0x65fffff, v4
	s_cbranch_execz .LBB16_34
; %bb.33:
	v_mul_f64 v[8:9], |v[2:3]|, 0.5
	s_mov_b32 s4, 0x6fdffd2b
	s_mov_b32 s8, 0xf99eb0bb
	s_mov_b32 s10, 0xca1d4f33
	s_mov_b32 s12, 0x2e21c33
	s_mov_b32 s5, 0xbf7e2fe7
	s_mov_b32 s9, 0x3f3e357e
	s_mov_b32 s11, 0x3f5f9c89
	s_mov_b32 s13, 0xbf1b1673
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fract_f64_e32 v[10:11], v[8:9]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[8:9]|
	v_add_f64 v[10:11], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v8, 0, v11, vcc_lo
	v_cndmask_b32_e32 v4, 0, v10, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v9, v5, v8, vcc_lo
	v_cndmask_b32_e32 v8, v2, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[8:9], v[8:9]
	v_rndne_f64_e32 v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[10:11], -0.5, v[8:9]
	v_cvt_i32_f64_e32 v4, v[10:11]
	v_mul_f64 v[12:13], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], s[8:9], s[4:5]
	v_fma_f64 v[16:17], v[12:13], s[12:13], s[10:11]
	s_mov_b32 s4, 0xd5f14825
	s_mov_b32 s8, 0x7294bff9
	s_mov_b32 s5, 0x3fb50782
	s_mov_b32 s9, 0xbf9a6d1e
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[4:5]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s4, 0xcdfe9424
	s_mov_b32 s8, 0x67b90b37
	s_mov_b32 s5, 0xbfe32d2c
	s_mov_b32 s9, 0x3fce1f50
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[4:5]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s4, 0x67754fff
	s_mov_b32 s8, 0x7e3c325b
	s_mov_b32 s5, 0x400466bc
	s_mov_b32 s9, 0xbff55d3c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[4:5]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s4, 0xe625be09
	s_mov_b32 s8, 0x81b5a67
	s_mov_b32 s5, 0xc014abbc
	s_mov_b32 s9, 0x40103c1f
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[4:5]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s4, 0xc9be45de
	s_mov_b32 s5, 0xc013bd3c
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s9, 0x3fc38538
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[14:15], v[18:19], v[14:15]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x400921fb
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[8:9], s[4:5], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[12:13], v[16:17], 1.0
	v_and_b32_e32 v12, 1, v4
	v_lshlrev_b32_e32 v4, 30, v4
	v_cmp_eq_u32_e32 vcc_lo, 0, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v4, v4, v3
	v_dual_cndmask_b32 v9, v11, v9 :: v_dual_and_b32 v4, 0x80000000, v4
	v_cndmask_b32_e32 v8, v10, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v4, v9, v4
	v_cndmask_b32_e64 v8, 0, v8, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v9, 0x7ff80000, v4, s0
	v_cmp_class_f64_e64 s0, v[2:3], 0x204
	v_mul_f64 v[8:9], v[2:3], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mov_b32 v10, v8 :: v_dual_and_b32 v11, 0x7fffffff, v9
	v_div_scale_f64 v[12:13], null, v[10:11], v[10:11], s[4:5]
	v_div_scale_f64 v[10:11], vcc_lo, s[4:5], v[10:11], s[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[12:13]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[14:15], v[14:15], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[14:15], v[14:15], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[10:11], v[14:15]
	v_fma_f64 v[10:11], -v[12:13], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[10:11], v[10:11], v[14:15], v[16:17]
	v_div_fixup_f64 v[8:9], v[10:11], |v[8:9]|, s[4:5]
	s_mov_b32 s5, 0x3fe55555
	s_mov_b32 s4, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[10:11], v[8:9]
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[10:11]
	s_mov_b32 s4, 0x55555780
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[10:11], v4
	v_frexp_exp_i32_f64_e32 v4, v[8:9]
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[18:19], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[8:9], 0x204
	v_rcp_f64_e32 v[14:15], v[12:13]
	v_add_f64 v[20:21], v[12:13], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[18:19], v[14:15]
	v_mul_f64 v[22:23], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[12:13], -v[22:23]
	v_fma_f64 v[10:11], v[16:17], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[22:23], v[10:11]
	v_add_f64 v[20:21], v[18:19], -v[12:13]
	v_add_f64 v[22:23], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[20:21], v[10:11]
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], v[10:11]
	v_mul_f64 v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[20:21], v[12:13], v[14:15]
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[14:15], v[18:19], s[4:5]
	v_ldexp_f64 v[18:19], v[12:13], 1
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0x3fe62e42
	v_mul_f64 v[14:15], v[20:21], v[14:15]
	v_cvt_f64_i32_e32 v[20:21], v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_add_f64 v[16:17], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[22:23], v[20:21], s[4:5]
	v_ldexp_f64 v[10:11], v[10:11], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[16:17], -v[18:19]
	v_fma_f64 v[18:19], v[20:21], s[4:5], -v[22:23]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_fma_f64 v[14:15], v[20:21], s[4:5], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[22:23], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], v[10:11]
	v_add_f64 v[22:23], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[12:13], v[18:19]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[20:21], -v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[20:21], -v[24:25]
	v_add_f64 v[16:17], v[18:19], -v[24:25]
	v_add_f64 v[18:19], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[26:27]
	v_add_f64 v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[18:19], -v[14:15]
	v_add_f64 v[12:13], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[18:19], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[22:23], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	v_add_f64 v[16:17], v[22:23], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[10:11], v[22:23], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, v10, v8, vcc_lo
	v_cndmask_b32_e32 v10, v11, v9, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[8:9]
	v_fract_f64_e32 v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v11, 0xfff00000, v10, vcc_lo
	v_cndmask_b32_e32 v10, 0, v4, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, 0, v[8:9]
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[6:7]
	s_or_b32 s0, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v7, v7, 0x7ff00000, s0
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v6, v6, 0, s0
.LBB16_34:
	s_or_b32 exec_lo, exec_lo, s2
.LBB16_35:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s1
	v_cmp_gt_f64_e32 vcc_lo, 0, v[2:3]
	v_cmp_class_f64_e64 s1, v[2:3], 0x264
	v_cmp_lt_u32_e64 s0, 0x432fffff, v5
	s_and_b32 s0, vcc_lo, s0
	v_cmp_u_f64_e32 vcc_lo, v[2:3], v[2:3]
	s_or_b32 s0, s1, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, v6, 0, s0
	v_cndmask_b32_e64 v5, v7, 0x7ff00000, s0
	v_dual_cndmask_b32 v3, v5, v3 :: v_dual_cndmask_b32 v2, v4, v2
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB16_36:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_lgamma_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_vgpr 38
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
		.amdhsa_inst_pref_size 63
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end16:
	.size	mx_lgamma_kernel, .Lfunc_end16-mx_lgamma_kernel
                                        ; -- End function
	.set mx_lgamma_kernel.num_vgpr, 38
	.set mx_lgamma_kernel.num_agpr, 0
	.set mx_lgamma_kernel.numbered_sgpr, 16
	.set mx_lgamma_kernel.num_named_barrier, 0
	.set mx_lgamma_kernel.private_seg_size, 0
	.set mx_lgamma_kernel.uses_vcc, 1
	.set mx_lgamma_kernel.uses_flat_scratch, 0
	.set mx_lgamma_kernel.has_dyn_sized_stack, 0
	.set mx_lgamma_kernel.has_recursion, 0
	.set mx_lgamma_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 9344
; TotalNumSgprs: 18
; NumVgprs: 38
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 18
; NumVGPRsForWavesPerEU: 38
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
	.protected	mx_deg2rad_kernel       ; -- Begin function mx_deg2rad_kernel
	.globl	mx_deg2rad_kernel
	.p2align	8
	.type	mx_deg2rad_kernel,@function
mx_deg2rad_kernel:                      ; @mx_deg2rad_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB17_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0xa2529d39
	s_mov_b32 s1, 0x3f91df46
	v_add_co_u32 v0, vcc_lo, s2, v0
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[2:3], s[0:1]
	global_store_b64 v[0:1], v[2:3], off
.LBB17_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_deg2rad_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_sgpr 5
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
.Lfunc_end17:
	.size	mx_deg2rad_kernel, .Lfunc_end17-mx_deg2rad_kernel
                                        ; -- End function
	.set mx_deg2rad_kernel.num_vgpr, 4
	.set mx_deg2rad_kernel.num_agpr, 0
	.set mx_deg2rad_kernel.numbered_sgpr, 5
	.set mx_deg2rad_kernel.num_named_barrier, 0
	.set mx_deg2rad_kernel.private_seg_size, 0
	.set mx_deg2rad_kernel.uses_vcc, 1
	.set mx_deg2rad_kernel.uses_flat_scratch, 0
	.set mx_deg2rad_kernel.has_dyn_sized_stack, 0
	.set mx_deg2rad_kernel.has_recursion, 0
	.set mx_deg2rad_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 172
; TotalNumSgprs: 7
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	mx_rad2deg_kernel       ; -- Begin function mx_rad2deg_kernel
	.globl	mx_rad2deg_kernel
	.p2align	8
	.type	mx_rad2deg_kernel,@function
mx_rad2deg_kernel:                      ; @mx_rad2deg_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB18_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x1a63c1f8
	s_mov_b32 s1, 0x404ca5dc
	v_add_co_u32 v0, vcc_lo, s2, v0
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[2:3], s[0:1]
	global_store_b64 v[0:1], v[2:3], off
.LBB18_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mx_rad2deg_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 280
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
		.amdhsa_next_free_sgpr 5
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
.Lfunc_end18:
	.size	mx_rad2deg_kernel, .Lfunc_end18-mx_rad2deg_kernel
                                        ; -- End function
	.set mx_rad2deg_kernel.num_vgpr, 4
	.set mx_rad2deg_kernel.num_agpr, 0
	.set mx_rad2deg_kernel.numbered_sgpr, 5
	.set mx_rad2deg_kernel.num_named_barrier, 0
	.set mx_rad2deg_kernel.private_seg_size, 0
	.set mx_rad2deg_kernel.uses_vcc, 1
	.set mx_rad2deg_kernel.uses_flat_scratch, 0
	.set mx_rad2deg_kernel.has_dyn_sized_stack, 0
	.set mx_rad2deg_kernel.has_recursion, 0
	.set mx_rad2deg_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 172
; TotalNumSgprs: 7
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_92810c59e977c741,@object ; @__hip_cuid_92810c59e977c741
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_92810c59e977c741
__hip_cuid_92810c59e977c741:
	.byte	0                               ; 0x0
	.size	__hip_cuid_92810c59e977c741, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_92810c59e977c741
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_square_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         mx_square_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_exp2_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         mx_exp2_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_log2_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         mx_log2_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_log10_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         mx_log10_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     22
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
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_cbrt_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         mx_cbrt_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     17
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
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_sinh_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         mx_sinh_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_cosh_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         mx_cosh_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_asin_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         mx_asin_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     22
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
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_acos_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     9
    .sgpr_spill_count: 0
    .symbol:         mx_acos_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     22
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
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_atan_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         mx_atan_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_asinh_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         mx_asinh_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     27
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
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_acosh_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         mx_acosh_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     27
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
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_atanh_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         mx_atanh_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     27
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
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_erf_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         mx_erf_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_erfc_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         mx_erfc_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_tgamma_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         mx_tgamma_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_lgamma_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         mx_lgamma_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     38
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
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_deg2rad_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         mx_deg2rad_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         28
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         32
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         36
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         38
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         40
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         42
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         44
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         46
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         80
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         88
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mx_rad2deg_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         mx_rad2deg_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     4
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
