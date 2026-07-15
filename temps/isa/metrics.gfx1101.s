	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z22pairwise_cosine_kernelPKdS0_Pdiii ; -- Begin function _Z22pairwise_cosine_kernelPKdS0_Pdiii
	.globl	_Z22pairwise_cosine_kernelPKdS0_Pdiii
	.p2align	8
	.type	_Z22pairwise_cosine_kernelPKdS0_Pdiii,@function
_Z22pairwise_cosine_kernelPKdS0_Pdiii:  ; @_Z22pairwise_cosine_kernelPKdS0_Pdiii
; %bb.0:
	s_clause 0x2
	s_load_b128 s[4:7], s[0:1], 0x18
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[12:13], s[0:1], 0x10
	v_bfe_u32 v12, v0, 10, 10
	v_and_b32_e32 v13, 0x3ff, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshl_add_u32 v11, s3, 4, v12
	v_lshl_add_u32 v0, s2, 4, v13
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e64 s0, s4, v11
	v_cmp_gt_i32_e64 s1, s5, v0
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB0_11
; %bb.1:
	v_lshlrev_b32_e32 v1, 3, v13
	v_dual_mov_b32 v3, 0 :: v_dual_lshlrev_b32 v14, 7, v12
	v_mov_b32_e32 v4, 0
	v_mul_lo_u32 v15, s6, v11
	s_delay_alu instid0(VALU_DEP_4)
	v_add_nc_u32_e32 v17, 0x800, v1
	v_mul_lo_u32 v16, s6, v0
	v_dual_mov_b32 v5, 0 :: v_dual_add_nc_u32 v18, v14, v1
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	v_dual_mov_b32 v6, 0 :: v_dual_add_nc_u32 v19, v17, v14
	s_mov_b32 s2, 0
	s_mov_b32 s3, s6
	s_branch .LBB0_3
.LBB0_2:                                ;   in Loop: Header=BB0_3 Depth=1
	s_add_i32 s2, s2, 16
	s_add_i32 s3, s3, -16
	s_cmp_ge_i32 s2, s6
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB0_10
.LBB0_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB0_9 Depth 2
	v_add_nc_u32_e32 v9, s2, v13
	v_mov_b32_e32 v7, 0
	v_mov_b32_e32 v8, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s6, v9
	s_and_b32 s14, s0, vcc_lo
	s_and_saveexec_b32 s7, s14
	s_cbranch_execz .LBB0_5
; %bb.4:                                ;   in Loop: Header=BB0_3 Depth=1
	v_add_nc_u32_e32 v7, v9, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s8, v7
	v_add_co_ci_u32_e64 v8, null, s9, v8, vcc_lo
	global_load_b64 v[7:8], v[7:8], off
.LBB0_5:                                ;   in Loop: Header=BB0_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s7
	v_dual_mov_b32 v9, 0 :: v_dual_add_nc_u32 v20, s2, v12
	v_mov_b32_e32 v10, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s6, v20
	s_and_b32 s14, s1, vcc_lo
	s_and_saveexec_b32 s7, s14
	s_cbranch_execz .LBB0_7
; %bb.6:                                ;   in Loop: Header=BB0_3 Depth=1
	v_add_nc_u32_e32 v9, v20, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v10, 31, v9
	v_lshlrev_b64 v[9:10], 3, v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v9, vcc_lo, s10, v9
	v_add_co_ci_u32_e64 v10, null, s11, v10, vcc_lo
	global_load_b64 v[9:10], v[9:10], off
.LBB0_7:                                ;   in Loop: Header=BB0_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s7
	s_cmp_le_i32 s6, s2
	s_waitcnt vmcnt(0)
	ds_store_b64 v18, v[7:8]
	ds_store_b64 v19, v[9:10]
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB0_2
; %bb.8:                                ;   in Loop: Header=BB0_3 Depth=1
	v_med3_i32 v7, s3, 1, 16
	v_dual_mov_b32 v8, v17 :: v_dual_mov_b32 v9, v14
	.p2align	6
.LBB0_9:                                ;   Parent Loop BB0_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ds_load_b64 v[20:21], v9
	ds_load_b64 v[22:23], v8
	v_add_nc_u32_e32 v7, -1, v7
	v_add_nc_u32_e32 v9, 8, v9
	v_add_nc_u32_e32 v8, 0x80, v8
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_eq_u32_e32 vcc_lo, 0, v7
	s_and_b32 vcc_lo, exec_lo, vcc_lo
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[3:4], v[20:21], v[20:21], v[3:4]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[1:2], v[20:21], v[22:23], v[1:2]
	v_fma_f64 v[5:6], v[22:23], v[22:23], v[5:6]
	s_cbranch_vccz .LBB0_9
	s_branch .LBB0_2
.LBB0_10:
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[3:4]
	v_cmp_gt_f64_e64 s0, 0x10000000, v[5:6]
	v_cndmask_b32_e64 v7, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v8, 0, 0x100, s0
	v_ldexp_f64 v[3:4], v[3:4], v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[5:6], v[5:6], v8
	v_rsq_f64_e32 v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_1)
	v_rsq_f64_e32 v[9:10], v[5:6]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[12:13], v[3:4], v[7:8]
	v_mul_f64 v[7:8], v[7:8], 0.5
	v_mul_f64 v[14:15], v[5:6], v[9:10]
	v_mul_f64 v[9:10], v[9:10], 0.5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], -v[7:8], v[12:13], 0.5
	v_fma_f64 v[18:19], -v[9:10], v[14:15], 0.5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[12:13], v[16:17], v[12:13]
	v_fma_f64 v[7:8], v[7:8], v[16:17], v[7:8]
	v_fma_f64 v[14:15], v[14:15], v[18:19], v[14:15]
	v_fma_f64 v[9:10], v[9:10], v[18:19], v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], -v[12:13], v[12:13], v[3:4]
	v_fma_f64 v[18:19], -v[14:15], v[14:15], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[16:17], v[7:8], v[12:13]
	v_fma_f64 v[14:15], v[18:19], v[9:10], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], -v[12:13], v[12:13], v[3:4]
	v_fma_f64 v[18:19], -v[14:15], v[14:15], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[7:8], v[16:17], v[7:8], v[12:13]
	v_cndmask_b32_e64 v12, 0, 0xffffff80, vcc_lo
	v_fma_f64 v[9:10], v[18:19], v[9:10], v[14:15]
	v_cndmask_b32_e64 v13, 0, 0xffffff80, s0
	v_cmp_class_f64_e64 vcc_lo, v[3:4], 0x260
	v_cmp_class_f64_e64 s0, v[5:6], 0x260
	v_ldexp_f64 v[7:8], v[7:8], v12
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[9:10], v[9:10], v13
	v_dual_cndmask_b32 v4, v8, v4 :: v_dual_cndmask_b32 v3, v7, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v6, v10, v6, s0
	v_cndmask_b32_e64 v5, v9, v5, s0
	s_branch .LBB0_12
.LBB0_11:
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v4, 0
	v_mov_b32_e32 v6, 0
.LBB0_12:
	v_cmp_gt_i32_e32 vcc_lo, s4, v11
	v_cmp_gt_i32_e64 s0, s5, v0
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB0_14
; %bb.13:
	v_mul_f64 v[3:4], v[5:6], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], v[1:2]
	v_div_scale_f64 v[12:13], vcc_lo, v[1:2], v[3:4], v[1:2]
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_mul_f64 v[9:10], v[12:13], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[9:10], v[12:13]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[9:10]
	v_cmp_lt_f64_e32 vcc_lo, 0, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[1:2], v[5:6], v[3:4], v[1:2]
	v_mad_u64_u32 v[3:4], null, s5, v11, v[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, 0, v2 :: v_dual_cndmask_b32 v1, 0, v1
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	v_add_co_u32 v3, vcc_lo, s12, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s13, v4, vcc_lo
	global_store_b64 v[3:4], v[1:2], off
.LBB0_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22pairwise_cosine_kernelPKdS0_Pdiii
		.amdhsa_group_segment_fixed_size 4096
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 36
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
		.amdhsa_system_sgpr_workgroup_id_y 1
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 1
		.amdhsa_next_free_vgpr 24
		.amdhsa_next_free_sgpr 15
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
.Lfunc_end0:
	.size	_Z22pairwise_cosine_kernelPKdS0_Pdiii, .Lfunc_end0-_Z22pairwise_cosine_kernelPKdS0_Pdiii
                                        ; -- End function
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.num_vgpr, 24
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.num_agpr, 0
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.numbered_sgpr, 15
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.num_named_barrier, 0
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.private_seg_size, 0
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.uses_vcc, 1
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.uses_flat_scratch, 0
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.has_dyn_sized_stack, 0
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.has_recursion, 0
	.set _Z22pairwise_cosine_kernelPKdS0_Pdiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1108
; TotalNumSgprs: 17
; NumVgprs: 24
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 4096 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 17
; NumVGPRsForWavesPerEU: 24
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 1
	.text
	.protected	_Z18pairwise_l1_kernelPKdS0_Pdiii ; -- Begin function _Z18pairwise_l1_kernelPKdS0_Pdiii
	.globl	_Z18pairwise_l1_kernelPKdS0_Pdiii
	.p2align	8
	.type	_Z18pairwise_l1_kernelPKdS0_Pdiii,@function
_Z18pairwise_l1_kernelPKdS0_Pdiii:      ; @_Z18pairwise_l1_kernelPKdS0_Pdiii
; %bb.0:
	s_clause 0x2
	s_load_b128 s[4:7], s[0:1], 0x18
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[12:13], s[0:1], 0x10
	v_bfe_u32 v8, v0, 10, 10
	v_and_b32_e32 v9, 0x3ff, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshl_add_u32 v7, s3, 4, v8
	v_lshl_add_u32 v0, s2, 4, v9
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e64 s0, s4, v7
	v_cmp_gt_i32_e64 s1, s5, v0
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB1_10
; %bb.1:
	v_lshlrev_b32_e32 v1, 3, v9
	v_lshlrev_b32_e32 v10, 7, v8
	v_mul_lo_u32 v12, s6, v7
	v_mul_lo_u32 v13, s6, v0
	s_mov_b32 s2, 0
	v_add_nc_u32_e32 v11, 0x800, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v14, v10, v1
	v_mov_b32_e32 v2, 0
	s_mov_b32 s3, s6
	s_delay_alu instid0(VALU_DEP_3)
	v_add_nc_u32_e32 v15, v11, v10
	s_branch .LBB1_3
.LBB1_2:                                ;   in Loop: Header=BB1_3 Depth=1
	s_add_i32 s2, s2, 16
	s_add_i32 s3, s3, -16
	s_cmp_ge_i32 s2, s6
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB1_11
.LBB1_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB1_9 Depth 2
	v_add_nc_u32_e32 v5, s2, v9
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s6, v5
	s_and_b32 s14, s0, vcc_lo
	s_and_saveexec_b32 s7, s14
	s_cbranch_execz .LBB1_5
; %bb.4:                                ;   in Loop: Header=BB1_3 Depth=1
	v_add_nc_u32_e32 v3, v5, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s8, v3
	v_add_co_ci_u32_e64 v4, null, s9, v4, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
.LBB1_5:                                ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s7
	v_dual_mov_b32 v5, 0 :: v_dual_add_nc_u32 v16, s2, v8
	v_mov_b32_e32 v6, 0
	s_waitcnt vmcnt(0)
	ds_store_b64 v14, v[3:4]
	v_cmp_gt_i32_e32 vcc_lo, s6, v16
	s_and_b32 s14, s1, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s7, s14
	s_cbranch_execz .LBB1_7
; %bb.6:                                ;   in Loop: Header=BB1_3 Depth=1
	v_add_nc_u32_e32 v3, v16, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s10, v3
	v_add_co_ci_u32_e64 v4, null, s11, v4, vcc_lo
	global_load_b64 v[5:6], v[3:4], off
.LBB1_7:                                ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s7
	s_cmp_le_i32 s6, s2
	s_waitcnt vmcnt(0)
	ds_store_b64 v15, v[5:6]
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB1_2
; %bb.8:                                ;   in Loop: Header=BB1_3 Depth=1
	v_med3_i32 v3, s3, 1, 16
	v_dual_mov_b32 v4, v11 :: v_dual_mov_b32 v5, v10
	.p2align	6
.LBB1_9:                                ;   Parent Loop BB1_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ds_load_b64 v[16:17], v5
	ds_load_b64 v[18:19], v4
	v_add_nc_u32_e32 v3, -1, v3
	v_add_nc_u32_e32 v5, 8, v5
	v_add_nc_u32_e32 v4, 0x80, v4
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, 0, v[16:17]
	v_xor_b32_e32 v6, 0x80000000, v17
	v_cndmask_b32_e32 v17, v17, v6, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, 0, v3
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[1:2], v[1:2], v[16:17]
	s_cbranch_vccz .LBB1_9
	s_branch .LBB1_2
.LBB1_10:
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
.LBB1_11:
	v_cmp_gt_i32_e32 vcc_lo, s4, v7
	v_cmp_gt_i32_e64 s0, s5, v0
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB1_13
; %bb.12:
	v_mad_u64_u32 v[3:4], null, s5, v7, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s12, v3
	v_add_co_ci_u32_e64 v4, null, s13, v4, vcc_lo
	global_store_b64 v[3:4], v[1:2], off
.LBB1_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z18pairwise_l1_kernelPKdS0_Pdiii
		.amdhsa_group_segment_fixed_size 4096
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 36
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
		.amdhsa_system_sgpr_workgroup_id_y 1
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 1
		.amdhsa_next_free_vgpr 20
		.amdhsa_next_free_sgpr 15
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
	.size	_Z18pairwise_l1_kernelPKdS0_Pdiii, .Lfunc_end1-_Z18pairwise_l1_kernelPKdS0_Pdiii
                                        ; -- End function
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.num_vgpr, 20
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.num_agpr, 0
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.numbered_sgpr, 15
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.num_named_barrier, 0
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.private_seg_size, 0
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.uses_vcc, 1
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.uses_flat_scratch, 0
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.has_dyn_sized_stack, 0
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.has_recursion, 0
	.set _Z18pairwise_l1_kernelPKdS0_Pdiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 600
; TotalNumSgprs: 17
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 4096 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 17
; NumVGPRsForWavesPerEU: 20
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 1
	.text
	.protected	_Z23pairwise_hamming_kernelPKhS0_Pdiii ; -- Begin function _Z23pairwise_hamming_kernelPKhS0_Pdiii
	.globl	_Z23pairwise_hamming_kernelPKhS0_Pdiii
	.p2align	8
	.type	_Z23pairwise_hamming_kernelPKhS0_Pdiii,@function
_Z23pairwise_hamming_kernelPKhS0_Pdiii: ; @_Z23pairwise_hamming_kernelPKhS0_Pdiii
; %bb.0:
	s_clause 0x2
	s_load_b128 s[4:7], s[0:1], 0x18
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[12:13], s[0:1], 0x10
	v_bfe_u32 v2, v0, 10, 10
	v_and_b32_e32 v3, 0x3ff, v0
	s_waitcnt lgkmcnt(0)
	s_mov_b32 s7, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshl_add_u32 v4, s3, 4, v2
	v_lshl_add_u32 v1, s2, 4, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e64 s0, s4, v4
	v_cmp_gt_i32_e64 s1, s5, v1
	s_cmp_gt_i32 s6, 0
	s_cselect_b32 s3, -1, 0
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB2_11
; %bb.1:
	v_dual_mov_b32 v8, 0 :: v_dual_lshlrev_b32 v5, 4, v2
	v_mul_lo_u32 v6, s6, v4
	v_mul_lo_u32 v7, s6, v1
	v_add_nc_u32_e32 v9, 0x100, v3
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v0, 0x100, v5
	v_add_nc_u32_e32 v10, v5, v3
	s_mov_b32 s14, s6
	v_add_nc_u32_e32 v11, v0, v3
	s_branch .LBB2_3
.LBB2_2:                                ;   in Loop: Header=BB2_3 Depth=1
	s_add_i32 s7, s7, 16
	s_add_i32 s14, s14, -16
	s_cmp_ge_i32 s7, s6
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB2_10
.LBB2_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB2_9 Depth 2
	v_add_nc_u32_e32 v12, s7, v3
	v_mov_b16_e32 v0.l, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e32 vcc_lo, s6, v12
	v_mov_b16_e32 v0.h, v0.l
	s_and_b32 s15, s0, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s2, s15
	s_cbranch_execz .LBB2_5
; %bb.4:                                ;   in Loop: Header=BB2_3 Depth=1
	v_add_nc_u32_e32 v12, v12, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v13, 31, v12
	v_add_co_u32 v12, vcc_lo, s8, v12
	v_add_co_ci_u32_e64 v13, null, s9, v13, vcc_lo
	global_load_d16_hi_u8 v0, v[12:13], off
.LBB2_5:                                ;   in Loop: Header=BB2_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	v_add_nc_u32_e32 v12, s7, v2
	s_waitcnt vmcnt(0)
	ds_store_b8_d16_hi v10, v0
	v_cmp_gt_i32_e32 vcc_lo, s6, v12
	s_and_b32 s15, s1, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s2, s15
	s_cbranch_execz .LBB2_7
; %bb.6:                                ;   in Loop: Header=BB2_3 Depth=1
	v_add_nc_u32_e32 v0, v12, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v13, 31, v0
	v_add_co_u32 v12, vcc_lo, s10, v0
	v_add_co_ci_u32_e64 v13, null, s11, v13, vcc_lo
	global_load_d16_u8 v0, v[12:13], off
.LBB2_7:                                ;   in Loop: Header=BB2_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	s_cmp_le_i32 s6, s7
	s_waitcnt vmcnt(0)
	ds_store_b8 v11, v0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB2_2
; %bb.8:                                ;   in Loop: Header=BB2_3 Depth=1
	v_med3_i32 v0, s14, 1, 16
	v_mov_b32_e32 v12, v9
	v_mov_b32_e32 v13, v5
.LBB2_9:                                ;   Parent Loop BB2_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ds_load_u8_d16 v14, v13
	s_waitcnt lgkmcnt(0)
	ds_load_u8_d16_hi v14, v12
	v_add_nc_u32_e32 v0, -1, v0
	v_add_nc_u32_e32 v13, 1, v13
	v_add_nc_u32_e32 v12, 16, v12
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	v_cmp_ne_u16_e64 s2, v14.l, v14.h
	v_add_co_ci_u32_e64 v8, null, 0, v8, s2
	s_cbranch_vccz .LBB2_9
	s_branch .LBB2_2
.LBB2_10:
	v_cvt_f64_i32_e32 v[2:3], v8
	s_branch .LBB2_12
.LBB2_11:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
.LBB2_12:
	v_cmp_gt_i32_e32 vcc_lo, s4, v4
	v_cmp_gt_i32_e64 s0, s5, v1
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB2_17
; %bb.13:
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccnz .LBB2_15
; %bb.14:
	v_cvt_f64_u32_e32 v[5:6], s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[7:8], null, v[5:6], v[5:6], v[2:3]
	v_rcp_f64_e32 v[9:10], v[7:8]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_div_scale_f64 v[11:12], vcc_lo, v[2:3], v[5:6], v[2:3]
	v_mul_f64 v[13:14], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[7:8], v[13:14], v[11:12]
	v_div_fmas_f64 v[7:8], v[7:8], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[7:8], v[5:6], v[2:3]
	s_branch .LBB2_16
.LBB2_15:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
.LBB2_16:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[5:6], null, s5, v4, v[1:2]
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[5:6]
	v_add_co_u32 v0, vcc_lo, s12, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s13, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB2_17:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z23pairwise_hamming_kernelPKhS0_Pdiii
		.amdhsa_group_segment_fixed_size 512
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 36
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
		.amdhsa_system_sgpr_workgroup_id_y 1
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 1
		.amdhsa_next_free_vgpr 15
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
.Lfunc_end2:
	.size	_Z23pairwise_hamming_kernelPKhS0_Pdiii, .Lfunc_end2-_Z23pairwise_hamming_kernelPKhS0_Pdiii
                                        ; -- End function
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.num_vgpr, 15
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.num_agpr, 0
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.numbered_sgpr, 16
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.num_named_barrier, 0
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.private_seg_size, 0
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.uses_vcc, 1
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.uses_flat_scratch, 0
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.has_dyn_sized_stack, 0
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.has_recursion, 0
	.set _Z23pairwise_hamming_kernelPKhS0_Pdiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 704
; TotalNumSgprs: 18
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 512 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 18
; NumVGPRsForWavesPerEU: 15
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 1
	.text
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_77c96545b3dbf162,@object ; @__hip_cuid_77c96545b3dbf162
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_77c96545b3dbf162
__hip_cuid_77c96545b3dbf162:
	.byte	0                               ; 0x0
	.size	__hip_cuid_77c96545b3dbf162, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_77c96545b3dbf162
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
      - .offset:         28
        .size:           4
        .value_kind:     by_value
      - .offset:         32
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 4096
    .kernarg_segment_align: 8
    .kernarg_segment_size: 36
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z22pairwise_cosine_kernelPKdS0_Pdiii
    .private_segment_fixed_size: 0
    .sgpr_count:     17
    .sgpr_spill_count: 0
    .symbol:         _Z22pairwise_cosine_kernelPKdS0_Pdiii.kd
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
      - .offset:         28
        .size:           4
        .value_kind:     by_value
      - .offset:         32
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 4096
    .kernarg_segment_align: 8
    .kernarg_segment_size: 36
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z18pairwise_l1_kernelPKdS0_Pdiii
    .private_segment_fixed_size: 0
    .sgpr_count:     17
    .sgpr_spill_count: 0
    .symbol:         _Z18pairwise_l1_kernelPKdS0_Pdiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     20
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
      - .offset:         28
        .size:           4
        .value_kind:     by_value
      - .offset:         32
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 512
    .kernarg_segment_align: 8
    .kernarg_segment_size: 36
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z23pairwise_hamming_kernelPKhS0_Pdiii
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z23pairwise_hamming_kernelPKhS0_Pdiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     15
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
