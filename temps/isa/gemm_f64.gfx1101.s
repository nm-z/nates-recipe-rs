	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.section	.text._Z11gemm_bt_f64IfEvPKT_S2_PS0_iii,"axG",@progbits,_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii,comdat
	.protected	_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii ; -- Begin function _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii
	.globl	_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii
	.p2align	8
	.type	_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii,@function
_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii:      ; @_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii
; %bb.0:
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB0_17
; %bb.1:
	v_lshrrev_b32_e32 v8, 5, v0
	s_clause 0x2
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b32 s13, s[0:1], 0x34
	s_load_b64 s[16:17], s[0:1], 0x10
	v_and_b32_e32 v7, 31, v0
	s_mul_hi_i32 s1, s6, s2
	s_mul_i32 s0, s6, s2
	v_mad_i64_i32 v[1:2], null, v8, s6, 0
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b32_e32 v3, 2, v7
	s_lshl_b64 s[14:15], s[0:1], 2
	s_ashr_i32 s3, s2, 31
	v_mbcnt_lo_u32_b32 v10, -1, 0
	v_lshl_add_u32 v9, v0, 2, 0
	v_cmp_gt_i32_e64 s0, s4, v8
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	v_cmp_eq_u32_e64 s1, 0, v7
	v_add_nc_u32_e32 v11, 0, v3
	v_lshl_or_b32 v12, v10, 2, 64
	s_mov_b32 s7, 0
	s_mov_b32 s12, s5
	v_add_co_u32 v1, vcc_lo, v1, v3
	s_waitcnt lgkmcnt(0)
	s_add_u32 s10, s10, s14
	s_addc_u32 s11, s11, s15
	s_and_b32 s13, s13, 0xffff
	v_add_co_ci_u32_e64 v2, null, 0, v2, vcc_lo
	s_add_i32 s14, s13, 31
	s_lshl_b64 s[2:3], s[2:3], 2
	v_add_co_u32 v1, vcc_lo, s8, v1
	s_lshr_b32 s14, s14, 5
	s_add_u32 s15, s16, s2
	v_add_co_ci_u32_e64 v2, null, s9, v2, vcc_lo
	s_addc_u32 s16, s17, s3
	s_mul_hi_i32 s3, s14, s6
	s_mul_i32 s2, s14, s6
	s_ashr_i32 s5, s5, 31
	s_lshl_b32 s17, s13, 2
	s_lshl_b64 s[8:9], s[2:3], 2
	s_branch .LBB0_3
.LBB0_2:                                ;   in Loop: Header=BB0_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s19
	v_add_co_u32 v1, vcc_lo, 0x4000, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, 0, v2, vcc_lo
	s_addk_i32 s7, 0x1000
	s_waitcnt lgkmcnt(0)
	s_waitcnt_vscnt null, 0x0
	s_cmp_ge_i32 s7, s6
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB0_17
.LBB0_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB0_5 Depth 2
                                        ;     Child Loop BB0_10 Depth 2
                                        ;       Child Loop BB0_12 Depth 3
	s_sub_i32 s2, s6, s7
	s_mov_b32 s3, exec_lo
	s_min_i32 s18, s2, 0x1000
	v_cmpx_gt_i32_e64 s2, v0
	s_cbranch_execz .LBB0_6
; %bb.4:                                ;   in Loop: Header=BB0_3 Depth=1
	v_dual_mov_b32 v3, v9 :: v_dual_mov_b32 v4, v0
	s_mov_b32 s19, 0
	.p2align	6
.LBB0_5:                                ;   Parent Loop BB0_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v5, s7, v4
	v_add_nc_u32_e32 v4, s13, v4
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 2, v[5:6]
	v_add_co_u32 v5, vcc_lo, s10, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s11, v6, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s18, v4
	global_load_b32 v5, v[5:6], off
	s_or_b32 s19, vcc_lo, s19
	s_waitcnt vmcnt(0)
	ds_store_b32 v3, v5
	v_add_nc_u32_e32 v3, s17, v3
	s_and_not1_b32 exec_lo, exec_lo, s19
	s_cbranch_execnz .LBB0_5
.LBB0_6:                                ;   in Loop: Header=BB0_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s3
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s19, s0
	s_cbranch_execz .LBB0_2
; %bb.7:                                ;   in Loop: Header=BB0_3 Depth=1
	v_cmp_gt_u32_e32 vcc_lo, 24, v10
	v_cmp_gt_i32_e64 s2, s2, v7
	s_cmp_lg_u32 s7, 0
	s_mov_b32 s21, 0
	s_cselect_b32 s20, -1, 0
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v10
	v_mov_b32_e32 v17, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_lshl_u32 v13, v3, v10, 2
	v_cndmask_b32_e64 v4, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v10
	v_add_lshl_u32 v14, v4, v10, 2
	v_cndmask_b32_e64 v5, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v10
	v_dual_mov_b32 v4, v2 :: v_dual_mov_b32 v3, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_lshl_u32 v15, v5, v10, 2
	v_add_co_ci_u32_e64 v6, null, 0, v10, vcc_lo
	v_lshlrev_b32_e32 v16, 2, v6
	s_branch .LBB0_10
.LBB0_8:                                ;   in Loop: Header=BB0_10 Depth=2
	global_store_b32 v[5:6], v18, off
.LBB0_9:                                ;   in Loop: Header=BB0_10 Depth=2
	s_or_b32 exec_lo, exec_lo, s3
	v_add_nc_u32_e32 v17, s14, v17
	v_add_co_u32 v3, s3, v3, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v4, null, s9, v4, s3
	v_cmp_le_i32_e32 vcc_lo, s4, v17
	s_or_b32 s21, vcc_lo, s21
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s21
	s_cbranch_execz .LBB0_2
.LBB0_10:                               ;   Parent Loop BB0_3 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB0_12 Depth 3
	v_mov_b32_e32 v18, 0
	s_and_saveexec_b32 s22, s2
	s_cbranch_execz .LBB0_14
; %bb.11:                               ;   in Loop: Header=BB0_10 Depth=2
	v_dual_mov_b32 v18, 0 :: v_dual_mov_b32 v19, v11
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v6, v4 :: v_dual_mov_b32 v5, v3
	v_mov_b32_e32 v20, v7
	s_mov_b32 s23, 0
	.p2align	6
.LBB0_12:                               ;   Parent Loop BB0_3 Depth=1
                                        ;     Parent Loop BB0_10 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	global_load_b32 v21, v[5:6], off
	ds_load_b32 v22, v19
	v_add_nc_u32_e32 v20, 32, v20
	v_add_co_u32 v5, vcc_lo, 0x80, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	v_cmp_le_i32_e64 s3, s18, v20
	v_add_nc_u32_e32 v19, 0x80, v19
	s_or_b32 s23, s3, s23
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fmac_f32_e32 v18, v21, v22
	s_and_not1_b32 exec_lo, exec_lo, s23
	s_cbranch_execnz .LBB0_12
; %bb.13:                               ;   in Loop: Header=BB0_10 Depth=2
	s_or_b32 exec_lo, exec_lo, s23
.LBB0_14:                               ;   in Loop: Header=BB0_10 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s22
	ds_bpermute_b32 v5, v12, v18
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v5, v18, v5
	ds_bpermute_b32 v6, v13, v5
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v5, v5, v6
	ds_bpermute_b32 v6, v14, v5
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v5, v5, v6
	ds_bpermute_b32 v6, v15, v5
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v5, v5, v6
	ds_bpermute_b32 v6, v16, v5
	s_and_saveexec_b32 s3, s1
	s_cbranch_execz .LBB0_9
; %bb.15:                               ;   in Loop: Header=BB0_10 Depth=2
	v_mad_u64_u32 v[18:19], null, v17, s12, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[20:21], null, v17, s5, v[19:20]
	v_mov_b32_e32 v19, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[19:20], 2, v[18:19]
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v18, v5, v6
	v_add_co_u32 v5, vcc_lo, s15, v19
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s16, v20, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s20
	s_cbranch_vccnz .LBB0_8
; %bb.16:                               ;   in Loop: Header=BB0_10 Depth=2
	global_load_b32 v19, v[5:6], off
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v18, v18, v19
	s_branch .LBB0_8
.LBB0_17:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii
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
		.amdhsa_next_free_vgpr 23
		.amdhsa_next_free_sgpr 24
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
	.section	.text._Z11gemm_bt_f64IfEvPKT_S2_PS0_iii,"axG",@progbits,_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii,comdat
.Lfunc_end0:
	.size	_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii, .Lfunc_end0-_Z11gemm_bt_f64IfEvPKT_S2_PS0_iii
                                        ; -- End function
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.num_vgpr, 23
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.num_agpr, 0
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.numbered_sgpr, 24
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.num_named_barrier, 0
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.private_seg_size, 0
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.uses_vcc, 1
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.has_recursion, 0
	.set _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 944
; TotalNumSgprs: 26
; NumVgprs: 23
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 26
; NumVGPRsForWavesPerEU: 23
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z11gemm_bt_f64IdEvPKT_S2_PS0_iii,"axG",@progbits,_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii,comdat
	.protected	_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii ; -- Begin function _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii
	.globl	_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii
	.p2align	8
	.type	_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii,@function
_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii:      ; @_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii
; %bb.0:
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB1_17
; %bb.1:
	v_lshrrev_b32_e32 v10, 5, v0
	s_clause 0x2
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b32 s13, s[0:1], 0x34
	s_load_b64 s[16:17], s[0:1], 0x10
	v_and_b32_e32 v9, 31, v0
	s_mul_hi_i32 s1, s6, s2
	s_mul_i32 s0, s6, s2
	v_mad_i64_i32 v[1:2], null, v10, s6, 0
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b32_e32 v3, 3, v9
	s_lshl_b64 s[14:15], s[0:1], 3
	s_ashr_i32 s3, s2, 31
	v_mbcnt_lo_u32_b32 v12, -1, 0
	v_lshl_add_u32 v11, v0, 3, 0
	v_cmp_gt_i32_e64 s0, s4, v10
	v_lshlrev_b64 v[1:2], 3, v[1:2]
	v_cmp_eq_u32_e64 s1, 0, v9
	v_add_nc_u32_e32 v13, 0, v3
	v_lshl_or_b32 v14, v12, 2, 64
	s_mov_b32 s7, 0
	s_mov_b32 s12, s5
	v_add_co_u32 v1, vcc_lo, v1, v3
	s_waitcnt lgkmcnt(0)
	s_add_u32 s10, s10, s14
	s_addc_u32 s11, s11, s15
	s_and_b32 s13, s13, 0xffff
	v_add_co_ci_u32_e64 v2, null, 0, v2, vcc_lo
	s_add_i32 s14, s13, 31
	s_lshl_b64 s[2:3], s[2:3], 3
	v_add_co_u32 v1, vcc_lo, s8, v1
	s_lshr_b32 s14, s14, 5
	s_add_u32 s15, s16, s2
	v_add_co_ci_u32_e64 v2, null, s9, v2, vcc_lo
	s_addc_u32 s16, s17, s3
	s_mul_hi_i32 s3, s14, s6
	s_mul_i32 s2, s14, s6
	s_ashr_i32 s5, s5, 31
	s_lshl_b32 s17, s13, 3
	s_lshl_b64 s[8:9], s[2:3], 3
	s_branch .LBB1_3
.LBB1_2:                                ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s19
	v_add_co_u32 v1, vcc_lo, 0x8000, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, 0, v2, vcc_lo
	s_addk_i32 s7, 0x1000
	s_waitcnt lgkmcnt(0)
	s_waitcnt_vscnt null, 0x0
	s_cmp_ge_i32 s7, s6
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB1_17
.LBB1_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB1_5 Depth 2
                                        ;     Child Loop BB1_10 Depth 2
                                        ;       Child Loop BB1_12 Depth 3
	s_sub_i32 s2, s6, s7
	s_mov_b32 s3, exec_lo
	s_min_i32 s18, s2, 0x1000
	v_cmpx_gt_i32_e64 s2, v0
	s_cbranch_execz .LBB1_6
; %bb.4:                                ;   in Loop: Header=BB1_3 Depth=1
	v_dual_mov_b32 v3, v11 :: v_dual_mov_b32 v4, v0
	s_mov_b32 s19, 0
	.p2align	6
.LBB1_5:                                ;   Parent Loop BB1_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v5, s7, v4
	v_add_nc_u32_e32 v4, s13, v4
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	v_add_co_u32 v5, vcc_lo, s10, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s11, v6, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s18, v4
	global_load_b64 v[5:6], v[5:6], off
	s_or_b32 s19, vcc_lo, s19
	s_waitcnt vmcnt(0)
	ds_store_b64 v3, v[5:6]
	v_add_nc_u32_e32 v3, s17, v3
	s_and_not1_b32 exec_lo, exec_lo, s19
	s_cbranch_execnz .LBB1_5
.LBB1_6:                                ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s3
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s19, s0
	s_cbranch_execz .LBB1_2
; %bb.7:                                ;   in Loop: Header=BB1_3 Depth=1
	v_cmp_gt_u32_e32 vcc_lo, 24, v12
	v_cmp_gt_i32_e64 s2, s2, v9
	v_mov_b32_e32 v19, v10
	s_cmp_lg_u32 s7, 0
	s_mov_b32 s21, 0
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v12
	s_cselect_b32 s20, -1, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_lshl_u32 v15, v3, v12, 2
	v_cndmask_b32_e64 v4, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v12
	v_add_lshl_u32 v16, v4, v12, 2
	v_cndmask_b32_e64 v5, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v12
	v_dual_mov_b32 v4, v2 :: v_dual_mov_b32 v3, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_lshl_u32 v17, v5, v12, 2
	v_add_co_ci_u32_e64 v6, null, 0, v12, vcc_lo
	v_lshlrev_b32_e32 v18, 2, v6
	s_branch .LBB1_10
.LBB1_8:                                ;   in Loop: Header=BB1_10 Depth=2
	global_store_b64 v[7:8], v[5:6], off
.LBB1_9:                                ;   in Loop: Header=BB1_10 Depth=2
	s_or_b32 exec_lo, exec_lo, s3
	v_add_nc_u32_e32 v19, s14, v19
	v_add_co_u32 v3, s3, v3, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v4, null, s9, v4, s3
	v_cmp_le_i32_e32 vcc_lo, s4, v19
	s_or_b32 s21, vcc_lo, s21
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s21
	s_cbranch_execz .LBB1_2
.LBB1_10:                               ;   Parent Loop BB1_3 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB1_12 Depth 3
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_and_saveexec_b32 s22, s2
	s_cbranch_execz .LBB1_14
; %bb.11:                               ;   in Loop: Header=BB1_10 Depth=2
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v20, v13
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v8, v4 :: v_dual_mov_b32 v21, v9
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v7, v3
	s_mov_b32 s23, 0
	.p2align	6
.LBB1_12:                               ;   Parent Loop BB1_3 Depth=1
                                        ;     Parent Loop BB1_10 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	global_load_b64 v[22:23], v[7:8], off
	ds_load_b64 v[24:25], v20
	v_add_nc_u32_e32 v21, 32, v21
	v_add_co_u32 v7, vcc_lo, 0x100, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_cmp_le_i32_e64 s3, s18, v21
	v_add_nc_u32_e32 v20, 0x100, v20
	s_or_b32 s23, s3, s23
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[5:6], v[22:23], v[24:25], v[5:6]
	s_and_not1_b32 exec_lo, exec_lo, s23
	s_cbranch_execnz .LBB1_12
; %bb.13:                               ;   in Loop: Header=BB1_10 Depth=2
	s_or_b32 exec_lo, exec_lo, s23
.LBB1_14:                               ;   in Loop: Header=BB1_10 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s22
	s_waitcnt lgkmcnt(1)
	ds_bpermute_b32 v7, v14, v5
	s_waitcnt lgkmcnt(1)
	ds_bpermute_b32 v8, v14, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v15, v5
	ds_bpermute_b32 v8, v15, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v16, v5
	ds_bpermute_b32 v8, v16, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v17, v5
	ds_bpermute_b32 v8, v17, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v18, v5
	ds_bpermute_b32 v8, v18, v6
	s_and_saveexec_b32 s3, s1
	s_cbranch_execz .LBB1_9
; %bb.15:                               ;   in Loop: Header=BB1_10 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_mad_u64_u32 v[20:21], null, v19, s12, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v7, v21
	v_mad_u64_u32 v[21:22], null, v19, s5, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[7:8], 3, v[20:21]
	v_add_co_u32 v7, vcc_lo, s15, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s16, v8, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s20
	s_cbranch_vccnz .LBB1_8
; %bb.16:                               ;   in Loop: Header=BB1_10 Depth=2
	global_load_b64 v[20:21], v[7:8], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[20:21]
	s_branch .LBB1_8
.LBB1_17:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii
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
		.amdhsa_next_free_vgpr 26
		.amdhsa_next_free_sgpr 24
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
	.section	.text._Z11gemm_bt_f64IdEvPKT_S2_PS0_iii,"axG",@progbits,_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii,comdat
.Lfunc_end1:
	.size	_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii, .Lfunc_end1-_Z11gemm_bt_f64IdEvPKT_S2_PS0_iii
                                        ; -- End function
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.num_vgpr, 26
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.num_agpr, 0
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.numbered_sgpr, 24
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.num_named_barrier, 0
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.private_seg_size, 0
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.uses_vcc, 1
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.has_recursion, 0
	.set _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1028
; TotalNumSgprs: 26
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 1
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 26
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
	.section	.text._Z9scale_f64IfEvPT_PKS0_l,"axG",@progbits,_Z9scale_f64IfEvPT_PKS0_l,comdat
	.protected	_Z9scale_f64IfEvPT_PKS0_l ; -- Begin function _Z9scale_f64IfEvPT_PKS0_l
	.globl	_Z9scale_f64IfEvPT_PKS0_l
	.p2align	8
	.type	_Z9scale_f64IfEvPT_PKS0_l,@function
_Z9scale_f64IfEvPT_PKS0_l:              ; @_Z9scale_f64IfEvPT_PKS0_l
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i64_e64 s[4:5], v[2:3]
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_load_b32 s0, s[2:3], 0x0
	global_load_b32 v2, v[0:1], off
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_mul_f32_e32 v2, s0, v2
	global_store_b32 v[0:1], v2, off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z9scale_f64IfEvPT_PKS0_l
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
		.amdhsa_inst_pref_size 2
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z9scale_f64IfEvPT_PKS0_l,"axG",@progbits,_Z9scale_f64IfEvPT_PKS0_l,comdat
.Lfunc_end2:
	.size	_Z9scale_f64IfEvPT_PKS0_l, .Lfunc_end2-_Z9scale_f64IfEvPT_PKS0_l
                                        ; -- End function
	.set _Z9scale_f64IfEvPT_PKS0_l.num_vgpr, 4
	.set _Z9scale_f64IfEvPT_PKS0_l.num_agpr, 0
	.set _Z9scale_f64IfEvPT_PKS0_l.numbered_sgpr, 6
	.set _Z9scale_f64IfEvPT_PKS0_l.num_named_barrier, 0
	.set _Z9scale_f64IfEvPT_PKS0_l.private_seg_size, 0
	.set _Z9scale_f64IfEvPT_PKS0_l.uses_vcc, 1
	.set _Z9scale_f64IfEvPT_PKS0_l.uses_flat_scratch, 0
	.set _Z9scale_f64IfEvPT_PKS0_l.has_dyn_sized_stack, 0
	.set _Z9scale_f64IfEvPT_PKS0_l.has_recursion, 0
	.set _Z9scale_f64IfEvPT_PKS0_l.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 140
; TotalNumSgprs: 8
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
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
	.section	.text._Z9scale_f64IdEvPT_PKS0_l,"axG",@progbits,_Z9scale_f64IdEvPT_PKS0_l,comdat
	.protected	_Z9scale_f64IdEvPT_PKS0_l ; -- Begin function _Z9scale_f64IdEvPT_PKS0_l
	.globl	_Z9scale_f64IdEvPT_PKS0_l
	.p2align	8
	.type	_Z9scale_f64IdEvPT_PKS0_l,@function
_Z9scale_f64IdEvPT_PKS0_l:              ; @_Z9scale_f64IdEvPT_PKS0_l
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i64_e64 s[4:5], v[2:3]
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_lshlrev_b64 v[0:1], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_load_b64 s[0:1], s[2:3], 0x0
	global_load_b64 v[2:3], v[0:1], off
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_mul_f64 v[2:3], s[0:1], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z9scale_f64IdEvPT_PKS0_l
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
		.amdhsa_inst_pref_size 2
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z9scale_f64IdEvPT_PKS0_l,"axG",@progbits,_Z9scale_f64IdEvPT_PKS0_l,comdat
.Lfunc_end3:
	.size	_Z9scale_f64IdEvPT_PKS0_l, .Lfunc_end3-_Z9scale_f64IdEvPT_PKS0_l
                                        ; -- End function
	.set _Z9scale_f64IdEvPT_PKS0_l.num_vgpr, 4
	.set _Z9scale_f64IdEvPT_PKS0_l.num_agpr, 0
	.set _Z9scale_f64IdEvPT_PKS0_l.numbered_sgpr, 6
	.set _Z9scale_f64IdEvPT_PKS0_l.num_named_barrier, 0
	.set _Z9scale_f64IdEvPT_PKS0_l.private_seg_size, 0
	.set _Z9scale_f64IdEvPT_PKS0_l.uses_vcc, 1
	.set _Z9scale_f64IdEvPT_PKS0_l.uses_flat_scratch, 0
	.set _Z9scale_f64IdEvPT_PKS0_l.has_dyn_sized_stack, 0
	.set _Z9scale_f64IdEvPT_PKS0_l.has_recursion, 0
	.set _Z9scale_f64IdEvPT_PKS0_l.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 144
; TotalNumSgprs: 8
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
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
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.section	.AMDGPU.csdata,"",@progbits
	.type	__hip_cuid_cac7378cc9b996ca,@object ; @__hip_cuid_cac7378cc9b996ca
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_cac7378cc9b996ca
__hip_cuid_cac7378cc9b996ca:
	.byte	0                               ; 0x0
	.size	__hip_cuid_cac7378cc9b996ca, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_cac7378cc9b996ca
	.amdgpu_metadata
---
amdhsa.kernels:
  - .args:
      - .actual_access:  read_only
        .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .actual_access:  read_only
        .address_space:  global
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
      - .offset:         160
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     26
    .sgpr_spill_count: 0
    .symbol:         _Z11gemm_bt_f64IfEvPKT_S2_PS0_iii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     23
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .actual_access:  read_only
        .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .actual_access:  read_only
        .address_space:  global
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
      - .offset:         160
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     26
    .sgpr_spill_count: 0
    .symbol:         _Z11gemm_bt_f64IdEvPKT_S2_PS0_iii.kd
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
        .size:           8
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
    .name:           _Z9scale_f64IfEvPT_PKS0_l
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         _Z9scale_f64IfEvPT_PKS0_l.kd
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
        .size:           8
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
    .name:           _Z9scale_f64IdEvPT_PKS0_l
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         _Z9scale_f64IdEvPT_PKS0_l.kd
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
