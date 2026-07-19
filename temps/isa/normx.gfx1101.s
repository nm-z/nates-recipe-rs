	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	normx_groupnorm_kernel  ; -- Begin function normx_groupnorm_kernel
	.globl	normx_groupnorm_kernel
	.p2align	8
	.type	normx_groupnorm_kernel,@function
normx_groupnorm_kernel:                 ; @normx_groupnorm_kernel
; %bb.0:
	s_load_b128 s[4:7], s[0:1], 0x24
	s_abs_i32 s9, s2
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s3, s6
	s_ashr_i32 s11, s6, 31
	v_cvt_f32_u32_e32 v1, s3
	s_sub_i32 s8, 0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s7, v1
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mul_i32 s8, s8, s7
	s_mul_hi_u32 s8, s7, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s7, s7, s8
	s_ashr_i32 s8, s2, 31
	s_mul_hi_u32 s10, s9, s7
	s_xor_b32 s8, s8, s11
	s_mul_i32 s12, s10, s3
	s_sub_i32 s9, s9, s12
	s_add_i32 s12, s10, 1
	s_sub_i32 s13, s9, s3
	s_cmp_ge_u32 s9, s3
	s_cselect_b32 s10, s12, s10
	s_cselect_b32 s9, s13, s9
	s_add_i32 s12, s10, 1
	s_cmp_ge_u32 s9, s3
	s_cselect_b32 s9, s12, s10
	s_abs_i32 s12, s4
	s_xor_b32 s9, s9, s8
	s_mul_hi_u32 s7, s12, s7
	s_sub_i32 s17, s9, s8
	s_mul_i32 s8, s7, s3
	s_ashr_i32 s10, s4, 31
	s_mul_i32 s6, s17, s6
	s_sub_i32 s8, s12, s8
	s_xor_b32 s16, s10, s11
	s_add_i32 s9, s7, 1
	s_sub_i32 s2, s2, s6
	s_sub_i32 s6, s8, s3
	s_cmp_ge_u32 s8, s3
	s_cselect_b32 s7, s9, s7
	s_cselect_b32 s6, s6, s8
	s_load_b256 s[8:15], s[0:1], 0x0
	s_add_i32 s18, s7, 1
	s_cmp_ge_u32 s6, s3
	s_mul_hi_i32 s6, s17, s4
	s_cselect_b32 s3, s18, s7
	s_mul_i32 s4, s17, s4
	s_xor_b32 s3, s3, s16
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s3, s3, s16
	s_ashr_i32 s16, s5, 31
	s_mul_i32 s18, s3, s2
	s_mul_i32 s17, s3, s5
	s_mul_hi_i32 s3, s3, s2
	s_add_u32 s19, s18, s4
	s_addc_u32 s3, s3, s6
	s_mul_i32 s4, s19, s16
	s_mul_hi_u32 s6, s19, s5
	v_cmp_gt_i32_e64 s2, s17, v0
	s_mul_i32 s3, s3, s5
	s_add_i32 s6, s6, s4
	s_mov_b32 s4, 0
	s_add_i32 s7, s6, s3
	s_mul_i32 s6, s19, s5
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB0_4
; %bb.1:
	s_load_b32 s22, s[0:1], 0x44
	v_mov_b32_e32 v1, 0
	s_lshl_b64 s[20:21], s[6:7], 3
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_waitcnt lgkmcnt(0)
	s_add_u32 s19, s8, s20
	s_addc_u32 s20, s9, s21
	s_and_b32 s21, s22, 0xffff
.LBB0_2:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	v_add_nc_u32_e32 v3, s21, v3
	v_add_co_u32 v4, vcc_lo, s19, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v5, null, s20, v5, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s17, v3
	global_load_b64 v[4:5], v[4:5], off
	s_or_b32 s4, vcc_lo, s4
	s_waitcnt vmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[4:5]
	s_and_not1_b32 exec_lo, exec_lo, s4
	s_cbranch_execnz .LBB0_2
; %bb.3:
	s_or_b32 exec_lo, exec_lo, s4
.LBB0_4:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	s_or_b32 exec_lo, exec_lo, s3
	v_mbcnt_lo_u32_b32 v5, -1, 0
	v_and_b32_e32 v14, 31, v0
	v_lshrrev_b32_e32 v15, 2, v0
	v_lshl_or_b32 v9, v5, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v5
	s_delay_alu instid0(VALU_DEP_4)
	v_cmp_eq_u32_e64 s3, 0, v14
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v10, v3, v5, 2
	ds_bpermute_b32 v3, v10, v1
	ds_bpermute_b32 v4, v10, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v11, v3, v5, 2
	ds_bpermute_b32 v3, v11, v1
	ds_bpermute_b32 v4, v11, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v12, v3, v5, 2
	ds_bpermute_b32 v3, v12, v1
	ds_bpermute_b32 v4, v12, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_co_ci_u32_e64 v3, null, 0, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b32_e32 v13, 2, v3
	ds_bpermute_b32 v3, v13, v1
	ds_bpermute_b32 v4, v13, v2
	s_and_saveexec_b32 s4, s3
	s_cbranch_execz .LBB0_6
; %bb.5:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v15, v[1:2]
.LBB0_6:
	s_or_b32 exec_lo, exec_lo, s4
	v_cmp_gt_u32_e64 s4, 32, v0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s19, s4
	s_cbranch_execz .LBB0_11
; %bb.7:
	s_load_b32 s20, s[0:1], 0x44
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s20, s20, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s20, s20, 31
	s_lshr_b32 s20, s20, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_u32_e32 vcc_lo, s20, v14
	s_and_saveexec_b32 s20, vcc_lo
; %bb.8:
	v_lshlrev_b32_e32 v1, 3, v14
	ds_load_b64 v[1:2], v1
; %bb.9:
	s_or_b32 exec_lo, exec_lo, s20
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v10, v1
	ds_bpermute_b32 v4, v10, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v11, v1
	ds_bpermute_b32 v4, v11, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v12, v1
	ds_bpermute_b32 v4, v12, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v13, v1
	ds_bpermute_b32 v4, v13, v2
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB0_11
; %bb.10:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_mov_b32_e32 v3, 0
	ds_store_b64 v3, v[1:2]
.LBB0_11:
	s_or_b32 exec_lo, exec_lo, s19
	s_waitcnt lgkmcnt(0)
	v_cvt_f64_i32_e32 v[3:4], s17
	v_mov_b32_e32 v1, 0
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[7:8], v[16:17], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[7:8], v[16:17], v[7:8]
	v_div_scale_f64 v[16:17], vcc_lo, v[1:2], v[3:4], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[16:17], v[7:8]
	v_fma_f64 v[5:6], -v[5:6], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[18:19]
	v_div_fixup_f64 v[1:2], v[5:6], v[3:4], v[1:2]
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_and_saveexec_b32 s19, s2
	s_cbranch_execz .LBB0_15
; %bb.12:
	s_load_b32 s22, s[0:1], 0x44
	v_mov_b32_e32 v5, 0
	s_lshl_b64 s[20:21], s[6:7], 3
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v7, v0
	s_add_u32 s20, s8, s20
	s_addc_u32 s21, s9, s21
	s_waitcnt lgkmcnt(0)
	s_and_b32 s23, s22, 0xffff
	s_mov_b32 s22, 0
	.p2align	6
.LBB0_13:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v8, 31, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[16:17], 3, v[7:8]
	v_add_nc_u32_e32 v7, s23, v7
	v_add_co_u32 v16, vcc_lo, s20, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v17, null, s21, v17, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s17, v7
	global_load_b64 v[16:17], v[16:17], off
	s_or_b32 s22, vcc_lo, s22
	s_waitcnt vmcnt(0)
	v_add_f64 v[16:17], v[16:17], -v[1:2]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[5:6], v[16:17], v[16:17], v[5:6]
	s_and_not1_b32 exec_lo, exec_lo, s22
	s_cbranch_execnz .LBB0_13
; %bb.14:
	s_or_b32 exec_lo, exec_lo, s22
.LBB0_15:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s19
	ds_bpermute_b32 v7, v9, v5
	ds_bpermute_b32 v8, v9, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v10, v5
	ds_bpermute_b32 v8, v10, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v11, v5
	ds_bpermute_b32 v8, v11, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v12, v5
	ds_bpermute_b32 v8, v12, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v13, v5
	ds_bpermute_b32 v8, v13, v6
	s_and_saveexec_b32 s19, s3
	s_cbranch_execz .LBB0_17
; %bb.16:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_store_b64 v15, v[5:6]
.LBB0_17:
	s_or_b32 exec_lo, exec_lo, s19
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s19, s4
	s_cbranch_execz .LBB0_22
; %bb.18:
	s_load_b32 s4, s[0:1], 0x44
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s4, s4, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s4, s4, 31
	s_lshr_b32 s4, s4, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_u32_e32 vcc_lo, s4, v14
	s_and_saveexec_b32 s4, vcc_lo
; %bb.19:
	v_lshlrev_b32_e32 v5, 3, v14
	ds_load_b64 v[5:6], v5
; %bb.20:
	s_or_b32 exec_lo, exec_lo, s4
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v7, v9, v5
	ds_bpermute_b32 v8, v9, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v10, v5
	ds_bpermute_b32 v8, v10, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v11, v5
	ds_bpermute_b32 v8, v11, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v12, v5
	ds_bpermute_b32 v8, v12, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v13, v5
	ds_bpermute_b32 v8, v13, v6
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB0_22
; %bb.21:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_mov_b32_e32 v7, 0
	ds_store_b64 v7, v[5:6]
.LBB0_22:
	s_or_b32 exec_lo, exec_lo, s19
	v_mov_b32_e32 v5, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[5:6], v5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB0_27
; %bb.23:
	v_div_scale_f64 v[7:8], null, v[3:4], v[3:4], v[5:6]
	v_div_scale_f64 v[13:14], vcc_lo, v[5:6], v[3:4], v[5:6]
	s_clause 0x1
	s_load_b64 s[2:3], s[0:1], 0x30
	s_load_b32 s0, s[0:1], 0x44
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[9:10], v[7:8]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	v_fma_f64 v[7:8], -v[7:8], v[11:12], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[7:8], v[7:8], v[9:10], v[11:12]
	v_div_fixup_f64 v[3:4], v[7:8], v[3:4], v[5:6]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], s[2:3], v[3:4]
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[3:4]
	s_and_b32 s1, vcc_lo, exec_lo
	s_cselect_b32 s1, 0x100, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], s1
	s_cselect_b32 s1, 0xffffff80, 0
	s_cmp_lg_u64 s[12:13], 0
	v_rsq_f64_e32 v[5:6], v[3:4]
	v_cmp_class_f64_e64 vcc_lo, v[3:4], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[7:8], v[3:4], v[5:6]
	v_mul_f64 v[5:6], v[5:6], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 0.5
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[5:6], v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[7:8], v[7:8], v[3:4]
	v_fma_f64 v[7:8], v[9:10], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[7:8], v[7:8], v[3:4]
	v_fma_f64 v[5:6], v[9:10], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], s1
	s_cselect_b32 s1, -1, 0
	s_abs_i32 s2, s5
	s_and_b32 s4, s0, 0xffff
	s_sub_i32 s3, 0, s2
	v_dual_cndmask_b32 v4, v6, v4 :: v_dual_cndmask_b32 v3, v5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], 1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_div_scale_f64 v[9:10], vcc_lo, 1.0, v[3:4], 1.0
	v_mul_f64 v[11:12], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[11:12], v[9:10]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[11:12]
	v_cvt_f32_u32_e32 v7, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rcp_iflag_f32_e32 v7, v7
	v_div_fixup_f64 v[3:4], v[5:6], v[3:4], 1.0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v5, 0x4f7ffffe, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v5, v5
	v_mul_lo_u32 v6, s3, v5
	s_mov_b32 s3, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v6, v5, v6
	v_add_nc_u32_e32 v9, v5, v6
	s_branch .LBB0_25
.LBB0_24:                               ;   in Loop: Header=BB0_25 Depth=1
	v_add_nc_u32_e32 v0, s4, v0
	v_add_co_u32 v5, s0, s10, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v6, null, s11, v6, s0
	v_cmp_le_i32_e32 vcc_lo, s17, v0
	global_store_b64 v[5:6], v[7:8], off
	s_or_b32 s3, vcc_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB0_27
.LBB0_25:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v10, 31, v0
	v_add_co_u32 v5, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s7, v10, vcc_lo
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s8, v5
	v_add_co_ci_u32_e64 v8, null, s9, v6, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s1
	global_load_b64 v[7:8], v[7:8], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[7:8], v[7:8], -v[1:2]
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[7:8], v[3:4], v[7:8]
	s_cbranch_vccnz .LBB0_24
; %bb.26:                               ;   in Loop: Header=BB0_25 Depth=1
	v_sub_nc_u32_e32 v11, 0, v0
	v_xor_b32_e32 v10, s16, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v11, v0, v11
	v_mul_hi_u32 v12, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v13, v12, s2
	v_sub_nc_u32_e32 v11, v11, v13
	v_add_nc_u32_e32 v13, 1, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v14, s2, v11
	v_cmp_le_u32_e32 vcc_lo, s2, v11
	v_dual_cndmask_b32 v12, v12, v13 :: v_dual_cndmask_b32 v11, v11, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v13, 1, v12
	v_cmp_le_u32_e32 vcc_lo, s2, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v11, v12, v13, vcc_lo
	v_xor_b32_e32 v11, v11, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v10, v11, v10
	v_add_nc_u32_e32 v10, s18, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v11, 31, v10
	v_lshlrev_b64 v[10:11], 3, v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v12, vcc_lo, s12, v10
	v_add_co_ci_u32_e64 v13, null, s13, v11, vcc_lo
	v_add_co_u32 v10, vcc_lo, s14, v10
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v11, null, s15, v11, vcc_lo
	global_load_b64 v[12:13], v[12:13], off
	global_load_b64 v[10:11], v[10:11], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[7:8], v[7:8], v[12:13], v[10:11]
	s_branch .LBB0_24
.LBB0_27:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel normx_groupnorm_kernel
		.amdhsa_group_segment_fixed_size 256
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
		.amdhsa_next_free_vgpr 20
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
		.amdhsa_inst_pref_size 21
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
	.size	normx_groupnorm_kernel, .Lfunc_end0-normx_groupnorm_kernel
                                        ; -- End function
	.set normx_groupnorm_kernel.num_vgpr, 20
	.set normx_groupnorm_kernel.num_agpr, 0
	.set normx_groupnorm_kernel.numbered_sgpr, 24
	.set normx_groupnorm_kernel.num_named_barrier, 0
	.set normx_groupnorm_kernel.private_seg_size, 0
	.set normx_groupnorm_kernel.uses_vcc, 1
	.set normx_groupnorm_kernel.uses_flat_scratch, 0
	.set normx_groupnorm_kernel.has_dyn_sized_stack, 0
	.set normx_groupnorm_kernel.has_recursion, 0
	.set normx_groupnorm_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2616
; TotalNumSgprs: 26
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 26
; NumVGPRsForWavesPerEU: 20
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
	.protected	normx_instancenorm_kernel ; -- Begin function normx_instancenorm_kernel
	.globl	normx_instancenorm_kernel
	.p2align	8
	.type	normx_instancenorm_kernel,@function
normx_instancenorm_kernel:              ; @normx_instancenorm_kernel
; %bb.0:
	s_clause 0x1
	s_load_b64 s[6:7], s[0:1], 0x24
	s_load_b256 s[8:15], s[0:1], 0x0
	s_abs_i32 s18, s2
	s_mov_b32 s3, 0
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s6, s6
	s_mul_hi_i32 s17, s7, s2
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s5, 0, s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s4, v1
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mul_i32 s5, s5, s4
	s_mul_hi_u32 s16, s4, s5
	v_cmp_gt_i32_e64 s5, s7, v0
	s_add_i32 s4, s4, s16
	s_mul_i32 s16, s7, s2
	s_mul_hi_u32 s19, s18, s4
	s_and_saveexec_b32 s4, s5
	s_cbranch_execz .LBB1_4
; %bb.1:
	s_load_b32 s22, s[0:1], 0x44
	v_mov_b32_e32 v1, 0
	s_lshl_b64 s[20:21], s[16:17], 3
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_add_u32 s20, s8, s20
	s_addc_u32 s21, s9, s21
	s_waitcnt lgkmcnt(0)
	s_and_b32 s22, s22, 0xffff
.LBB1_2:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	v_add_nc_u32_e32 v3, s22, v3
	v_add_co_u32 v4, vcc_lo, s20, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v5, null, s21, v5, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s7, v3
	global_load_b64 v[4:5], v[4:5], off
	s_or_b32 s3, vcc_lo, s3
	s_waitcnt vmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[4:5]
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execnz .LBB1_2
; %bb.3:
	s_or_b32 exec_lo, exec_lo, s3
.LBB1_4:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	s_or_b32 exec_lo, exec_lo, s4
	v_mbcnt_lo_u32_b32 v5, -1, 0
	v_and_b32_e32 v14, 31, v0
	v_lshrrev_b32_e32 v15, 2, v0
	s_ashr_i32 s2, s2, 31
	v_lshl_or_b32 v9, v5, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v5
	v_cmp_eq_u32_e64 s3, 0, v14
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v10, v3, v5, 2
	ds_bpermute_b32 v3, v10, v1
	ds_bpermute_b32 v4, v10, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v11, v3, v5, 2
	ds_bpermute_b32 v3, v11, v1
	ds_bpermute_b32 v4, v11, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v12, v3, v5, 2
	ds_bpermute_b32 v3, v12, v1
	ds_bpermute_b32 v4, v12, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_co_ci_u32_e64 v3, null, 0, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b32_e32 v13, 2, v3
	ds_bpermute_b32 v3, v13, v1
	ds_bpermute_b32 v4, v13, v2
	s_and_saveexec_b32 s4, s3
	s_cbranch_execz .LBB1_6
; %bb.5:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v15, v[1:2]
.LBB1_6:
	s_or_b32 exec_lo, exec_lo, s4
	v_cmp_gt_u32_e64 s4, 32, v0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s20, s4
	s_cbranch_execz .LBB1_11
; %bb.7:
	s_load_b32 s21, s[0:1], 0x44
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s21, s21, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s21, s21, 31
	s_lshr_b32 s21, s21, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_u32_e32 vcc_lo, s21, v14
	s_and_saveexec_b32 s21, vcc_lo
; %bb.8:
	v_lshlrev_b32_e32 v1, 3, v14
	ds_load_b64 v[1:2], v1
; %bb.9:
	s_or_b32 exec_lo, exec_lo, s21
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v10, v1
	ds_bpermute_b32 v4, v10, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v11, v1
	ds_bpermute_b32 v4, v11, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v12, v1
	ds_bpermute_b32 v4, v12, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v13, v1
	ds_bpermute_b32 v4, v13, v2
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB1_11
; %bb.10:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_mov_b32_e32 v3, 0
	ds_store_b64 v3, v[1:2]
.LBB1_11:
	s_or_b32 exec_lo, exec_lo, s20
	s_waitcnt lgkmcnt(0)
	v_cvt_f64_i32_e32 v[3:4], s7
	v_mov_b32_e32 v1, 0
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[7:8], v[16:17], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[7:8], v[16:17], v[7:8]
	v_div_scale_f64 v[16:17], vcc_lo, v[1:2], v[3:4], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[16:17], v[7:8]
	v_fma_f64 v[5:6], -v[5:6], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[18:19]
	v_div_fixup_f64 v[1:2], v[5:6], v[3:4], v[1:2]
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_and_saveexec_b32 s20, s5
	s_cbranch_execz .LBB1_15
; %bb.12:
	s_load_b32 s24, s[0:1], 0x44
	v_mov_b32_e32 v5, 0
	s_lshl_b64 s[22:23], s[16:17], 3
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v7, v0
	s_add_u32 s21, s8, s22
	s_addc_u32 s22, s9, s23
	s_mov_b32 s23, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s24, s24, 0xffff
	.p2align	6
.LBB1_13:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v8, 31, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[16:17], 3, v[7:8]
	v_add_nc_u32_e32 v7, s24, v7
	v_add_co_u32 v16, vcc_lo, s21, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v17, null, s22, v17, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s7, v7
	global_load_b64 v[16:17], v[16:17], off
	s_or_b32 s23, vcc_lo, s23
	s_waitcnt vmcnt(0)
	v_add_f64 v[16:17], v[16:17], -v[1:2]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[5:6], v[16:17], v[16:17], v[5:6]
	s_and_not1_b32 exec_lo, exec_lo, s23
	s_cbranch_execnz .LBB1_13
; %bb.14:
	s_or_b32 exec_lo, exec_lo, s23
.LBB1_15:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s20
	ds_bpermute_b32 v7, v9, v5
	ds_bpermute_b32 v8, v9, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v10, v5
	ds_bpermute_b32 v8, v10, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v11, v5
	ds_bpermute_b32 v8, v11, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v12, v5
	ds_bpermute_b32 v8, v12, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v13, v5
	ds_bpermute_b32 v8, v13, v6
	s_and_saveexec_b32 s20, s3
	s_cbranch_execz .LBB1_17
; %bb.16:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_store_b64 v15, v[5:6]
.LBB1_17:
	s_or_b32 exec_lo, exec_lo, s20
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s20, s4
	s_cbranch_execz .LBB1_22
; %bb.18:
	s_load_b32 s4, s[0:1], 0x44
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s4, s4, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s4, s4, 31
	s_lshr_b32 s4, s4, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_u32_e32 vcc_lo, s4, v14
	s_and_saveexec_b32 s4, vcc_lo
; %bb.19:
	v_lshlrev_b32_e32 v5, 3, v14
	ds_load_b64 v[5:6], v5
; %bb.20:
	s_or_b32 exec_lo, exec_lo, s4
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v7, v9, v5
	ds_bpermute_b32 v8, v9, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v10, v5
	ds_bpermute_b32 v8, v10, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v11, v5
	ds_bpermute_b32 v8, v11, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v12, v5
	ds_bpermute_b32 v8, v12, v6
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	ds_bpermute_b32 v7, v13, v5
	ds_bpermute_b32 v8, v13, v6
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB1_22
; %bb.21:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_mov_b32_e32 v7, 0
	ds_store_b64 v7, v[5:6]
.LBB1_22:
	s_or_b32 exec_lo, exec_lo, s20
	v_mov_b32_e32 v9, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[5:6], v9
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s3, s5
	s_cbranch_execz .LBB1_27
; %bb.23:
	v_div_scale_f64 v[7:8], null, v[3:4], v[3:4], v[5:6]
	v_div_scale_f64 v[14:15], vcc_lo, v[5:6], v[3:4], v[5:6]
	s_clause 0x1
	s_load_b64 s[4:5], s[0:1], 0x30
	s_load_b32 s0, s[0:1], 0x44
	s_mul_i32 s19, s19, s6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[7:8]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[7:8], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[7:8], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_fma_f64 v[7:8], -v[7:8], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[7:8], v[7:8], v[10:11], v[12:13]
	v_div_fixup_f64 v[3:4], v[7:8], v[3:4], v[5:6]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], s[4:5], v[3:4]
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[3:4]
	s_and_b32 s1, vcc_lo, exec_lo
	s_cselect_b32 s1, 0x100, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], s1
	s_cselect_b32 s1, 0xffffff80, 0
	v_rsq_f64_e32 v[5:6], v[3:4]
	v_cmp_class_f64_e64 vcc_lo, v[3:4], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[7:8], v[3:4], v[5:6]
	v_mul_f64 v[5:6], v[5:6], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[5:6], v[7:8], 0.5
	v_fma_f64 v[7:8], v[7:8], v[10:11], v[7:8]
	v_fma_f64 v[5:6], v[5:6], v[10:11], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[7:8], v[7:8], v[3:4]
	v_fma_f64 v[7:8], v[10:11], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[7:8], v[7:8], v[3:4]
	v_fma_f64 v[5:6], v[10:11], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ldexp_f64 v[5:6], v[5:6], s1
	s_sub_i32 s1, s18, s19
	s_sub_i32 s3, s1, s6
	s_cmp_ge_u32 s1, s6
	s_cselect_b32 s1, s3, s1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_sub_i32 s3, s1, s6
	s_cmp_ge_u32 s1, s6
	s_mov_b32 s6, 0
	s_cselect_b32 s1, s3, s1
	s_xor_b32 s1, s1, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_sub_i32 s2, s1, s2
	s_cmp_lg_u64 s[12:13], 0
	s_cselect_b32 s1, -1, 0
	s_ashr_i32 s3, s2, 31
	s_lshl_b64 s[4:5], s[2:3], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	s_add_u32 s2, s14, s4
	s_addc_u32 s3, s15, s5
	s_add_u32 s4, s12, s4
	s_addc_u32 s5, s13, s5
	s_and_b32 s12, s0, 0xffff
	v_dual_cndmask_b32 v4, v6, v4 :: v_dual_cndmask_b32 v3, v5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], 1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[10:11], v[7:8]
	v_fma_f64 v[10:11], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[10:11], v[7:8]
	v_div_scale_f64 v[10:11], vcc_lo, 1.0, v[3:4], 1.0
	v_mul_f64 v[12:13], v[10:11], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[12:13], v[10:11]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[5:6], v[3:4], 1.0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB1_25
	.p2align	6
.LBB1_24:                               ;   in Loop: Header=BB1_25 Depth=1
	v_add_nc_u32_e32 v0, s12, v0
	v_add_co_u32 v5, s0, s10, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v6, null, s11, v6, s0
	v_cmp_le_i32_e32 vcc_lo, s7, v0
	global_store_b64 v[5:6], v[7:8], off
	s_or_b32 s6, vcc_lo, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s6
	s_cbranch_execz .LBB1_27
.LBB1_25:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v6, 31, v0
	v_add_co_u32 v5, vcc_lo, s16, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s17, v6, vcc_lo
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s8, v5
	v_add_co_ci_u32_e64 v8, null, s9, v6, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s1
	global_load_b64 v[7:8], v[7:8], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[7:8], v[7:8], -v[1:2]
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[7:8], v[3:4], v[7:8]
	s_cbranch_vccnz .LBB1_24
; %bb.26:                               ;   in Loop: Header=BB1_25 Depth=1
	s_clause 0x1
	global_load_b64 v[10:11], v9, s[4:5]
	global_load_b64 v[12:13], v9, s[2:3]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[7:8], v[7:8], v[10:11], v[12:13]
	s_branch .LBB1_24
.LBB1_27:
	s_set_inst_prefetch_distance 0x2
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel normx_instancenorm_kernel
		.amdhsa_group_segment_fixed_size 256
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
		.amdhsa_next_free_vgpr 20
		.amdhsa_next_free_sgpr 25
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
		.amdhsa_inst_pref_size 18
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
	.size	normx_instancenorm_kernel, .Lfunc_end1-normx_instancenorm_kernel
                                        ; -- End function
	.set normx_instancenorm_kernel.num_vgpr, 20
	.set normx_instancenorm_kernel.num_agpr, 0
	.set normx_instancenorm_kernel.numbered_sgpr, 25
	.set normx_instancenorm_kernel.num_named_barrier, 0
	.set normx_instancenorm_kernel.private_seg_size, 0
	.set normx_instancenorm_kernel.uses_vcc, 1
	.set normx_instancenorm_kernel.uses_flat_scratch, 0
	.set normx_instancenorm_kernel.has_dyn_sized_stack, 0
	.set normx_instancenorm_kernel.has_recursion, 0
	.set normx_instancenorm_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2304
; TotalNumSgprs: 27
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 1
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 27
; NumVGPRsForWavesPerEU: 20
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
	.protected	normx_l2_normalize_kernel ; -- Begin function normx_l2_normalize_kernel
	.globl	normx_l2_normalize_kernel
	.p2align	8
	.type	normx_l2_normalize_kernel,@function
normx_l2_normalize_kernel:              ; @normx_l2_normalize_kernel
; %bb.0:
	s_load_b64 s[8:9], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s8
	s_cbranch_scc1 .LBB2_15
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	s_mul_hi_i32 s3, s9, s2
	s_mul_i32 s2, s9, s2
	v_mov_b32_e32 v1, 0
	s_lshl_b64 s[10:11], s[2:3], 3
	v_mov_b32_e32 v2, 0
	v_cmp_gt_i32_e32 vcc_lo, s9, v0
	s_waitcnt lgkmcnt(0)
	s_add_u32 s4, s4, s10
	s_addc_u32 s5, s5, s11
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB2_5
; %bb.2:
	s_load_b32 s2, s[0:1], 0x2c
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_mov_b32 s8, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s12, s2, 0xffff
	.p2align	6
.LBB2_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	v_add_nc_u32_e32 v3, s12, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, s2, s4, v4
	v_add_co_ci_u32_e64 v5, null, s5, v5, s2
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s2, s9, v3
	global_load_b64 v[4:5], v[4:5], off
	s_or_b32 s8, s2, s8
	s_waitcnt vmcnt(0)
	v_fma_f64 v[1:2], v[4:5], v[4:5], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s8
	s_cbranch_execnz .LBB2_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s8
.LBB2_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s3
	v_mbcnt_lo_u32_b32 v9, -1, 0
	v_and_b32_e32 v10, 31, v0
	v_lshl_or_b32 v5, v9, 2, 64
	v_cmp_gt_u32_e64 s2, 24, v9
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 8, s2
	v_cmp_gt_u32_e64 s2, 28, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v6, v3, v9, 2
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 4, s2
	v_cmp_gt_u32_e64 s2, 30, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v7, v3, v9, 2
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 2, s2
	v_cmp_ne_u32_e64 s2, 31, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v8, v3, v9, 2
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_co_ci_u32_e64 v3, null, 0, v9, s2
	v_cmp_eq_u32_e64 s2, 0, v10
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b32_e32 v9, 2, v3
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB2_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB2_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s8, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB2_12
; %bb.8:
	s_load_b32 s3, s[0:1], 0x2c
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s12, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s3, s3, 31
	s_lshr_b32 s3, s3, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s3, v10
; %bb.9:
	v_lshlrev_b32_e32 v1, 3, v10
	ds_load_b64 v[1:2], v1
; %bb.10:
	s_or_b32 exec_lo, exec_lo, s12
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_and_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB2_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_mov_b32_e32 v3, 0
	ds_store_b64 v3, v[1:2]
.LBB2_12:
	s_or_b32 exec_lo, exec_lo, s8
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB2_15
; %bb.13:
	s_clause 0x1
	s_load_b64 s[2:3], s[0:1], 0x18
	s_load_b32 s0, s[0:1], 0x2c
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f64_e32 vcc_lo, s[2:3], v[1:2]
	v_cndmask_b32_e32 v2, s3, v2, vcc_lo
	v_cndmask_b32_e32 v1, s2, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[1:2]
	s_and_b32 s1, vcc_lo, exec_lo
	s_cselect_b32 s1, 0x100, 0
	v_ldexp_f64 v[1:2], v[1:2], s1
	s_cselect_b32 s1, 0xffffff80, 0
	s_add_u32 s2, s6, s10
	s_addc_u32 s3, s7, s11
	s_and_b32 s6, s0, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[3:4], v[1:2]
	v_cmp_class_f64_e64 vcc_lo, v[1:2], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[5:6], v[1:2], v[3:4]
	v_mul_f64 v[3:4], v[3:4], 0.5
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_fma_f64 v[3:4], v[3:4], v[7:8], v[3:4]
	v_fma_f64 v[7:8], -v[5:6], v[5:6], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[7:8], v[3:4], v[5:6]
	v_fma_f64 v[7:8], -v[5:6], v[5:6], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[7:8], v[3:4], v[5:6]
	v_ldexp_f64 v[3:4], v[3:4], s1
	s_mov_b32 s1, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v4, v2 :: v_dual_cndmask_b32 v1, v3, v1
	v_div_scale_f64 v[3:4], null, v[1:2], v[1:2], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[5:6], v[3:4]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_div_scale_f64 v[7:8], vcc_lo, 1.0, v[1:2], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[7:8], v[5:6]
	v_fma_f64 v[3:4], -v[3:4], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[3:4], v[3:4], v[5:6], v[9:10]
	v_div_fixup_f64 v[2:3], v[3:4], v[1:2], 1.0
	.p2align	6
.LBB2_14:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 3, v[0:1]
	v_add_nc_u32_e32 v0, s6, v0
	v_add_co_u32 v6, vcc_lo, s4, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v7, null, s5, v5, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s9, v0
	v_add_co_u32 v4, s0, s2, v4
	global_load_b64 v[6:7], v[6:7], off
	v_add_co_ci_u32_e64 v5, null, s3, v5, s0
	s_or_b32 s1, vcc_lo, s1
	s_waitcnt vmcnt(0)
	v_mul_f64 v[6:7], v[2:3], v[6:7]
	global_store_b64 v[4:5], v[6:7], off
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execnz .LBB2_14
.LBB2_15:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel normx_l2_normalize_kernel
		.amdhsa_group_segment_fixed_size 256
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
		.amdhsa_next_free_vgpr 11
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
.Lfunc_end2:
	.size	normx_l2_normalize_kernel, .Lfunc_end2-normx_l2_normalize_kernel
                                        ; -- End function
	.set normx_l2_normalize_kernel.num_vgpr, 11
	.set normx_l2_normalize_kernel.num_agpr, 0
	.set normx_l2_normalize_kernel.numbered_sgpr, 13
	.set normx_l2_normalize_kernel.num_named_barrier, 0
	.set normx_l2_normalize_kernel.private_seg_size, 0
	.set normx_l2_normalize_kernel.uses_vcc, 1
	.set normx_l2_normalize_kernel.uses_flat_scratch, 0
	.set normx_l2_normalize_kernel.has_dyn_sized_stack, 0
	.set normx_l2_normalize_kernel.has_recursion, 0
	.set normx_l2_normalize_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1260
; TotalNumSgprs: 15
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 15
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
	.section	.text._Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_,"axG",@progbits,_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_,comdat
	.protected	_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_ ; -- Begin function _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_
	.globl	_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_
	.p2align	8
	.type	_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_,@function
_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_: ; @_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_
; %bb.0:
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s8
	s_cbranch_scc1 .LBB3_17
; %bb.1:
	s_clause 0x2
	s_load_b64 s[12:13], s[0:1], 0x20
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[10:11], s[0:1], 0x10
	s_mul_hi_i32 s3, s9, s2
	s_mul_i32 s2, s9, s2
	v_cmp_gt_i32_e32 vcc_lo, s9, v0
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	s_load_b32 s8, s[12:13], 0x0
	s_lshl_b64 s[12:13], s[2:3], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s4, s4, s12
	s_addc_u32 s5, s5, s13
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB3_5
; %bb.2:
	s_load_b32 s2, s[0:1], 0x34
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_mov_b32 s14, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s15, s2, 0xffff
.LBB3_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	v_add_nc_u32_e32 v1, s15, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, s2, s4, v4
	v_add_co_ci_u32_e64 v5, null, s5, v5, s2
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s2, s9, v1
	global_load_b32 v2, v[4:5], off
	s_or_b32 s14, s2, s14
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v3, v2, v2
	s_and_not1_b32 exec_lo, exec_lo, s14
	s_cbranch_execnz .LBB3_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s14
.LBB3_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s3
	v_mbcnt_lo_u32_b32 v5, -1, 0
	v_lshl_or_b32 v1, v5, 2, 64
	v_cmp_gt_u32_e64 s2, 24, v5
	ds_bpermute_b32 v4, v1, v3
	v_cndmask_b32_e64 v2, 0, 8, s2
	v_cmp_gt_u32_e64 s2, 28, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_lshl_u32 v2, v2, v5, 2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v4, v3, v4
	v_cndmask_b32_e64 v3, 0, 4, s2
	v_cmp_gt_u32_e64 s2, 30, v5
	ds_bpermute_b32 v6, v2, v4
	v_add_lshl_u32 v3, v3, v5, 2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v6, v4, v6
	v_cndmask_b32_e64 v4, 0, 2, s2
	v_cmp_ne_u32_e64 s2, 31, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_lshl_u32 v4, v4, v5, 2
	v_add_co_ci_u32_e64 v5, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b32_e32 v5, 2, v5
	ds_bpermute_b32 v7, v3, v6
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v6, v6, v7
	ds_bpermute_b32 v7, v4, v6
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v7, v6, v7 :: v_dual_and_b32 v6, 31, v0
	ds_bpermute_b32 v8, v5, v7
	v_cmp_eq_u32_e64 s2, 0, v6
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB3_7
; %bb.6:
	v_lshrrev_b32_e32 v9, 3, v0
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v7, v7, v8
	ds_store_b32 v9, v7
.LBB3_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s14, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB3_12
; %bb.8:
	s_load_b32 s3, s[0:1], 0x34
	v_mov_b32_e32 v7, 0
	s_mov_b32 s15, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s3, s3, 31
	s_lshr_b32 s3, s3, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s3, v6
; %bb.9:
	v_lshlrev_b32_e32 v6, 2, v6
	ds_load_b32 v7, v6
; %bb.10:
	s_or_b32 exec_lo, exec_lo, s15
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v1, v1, v7
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v7, v1
	ds_bpermute_b32 v2, v2, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v3, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v5, v1
	s_and_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB3_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB3_12:
	s_or_b32 exec_lo, exec_lo, s14
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v1, v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB3_17
; %bb.13:
	v_cvt_f32_i32_e32 v2, s9
	s_load_b32 s0, s[0:1], 0x34
	s_add_u32 s1, s6, s12
	s_mov_b32 s6, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f32 v3, null, v2, v2, v1
	v_div_scale_f32 v6, vcc_lo, v1, v2, v1
	v_rcp_f32_e32 v4, v3
	s_waitcnt_depctr 0xfff
	v_fma_f32 v5, -v3, v4, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v4, v5, v4
	v_mul_f32_e32 v5, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v7, -v3, v5, v6
	v_fmac_f32_e32 v5, v7, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v3, -v3, v5, v6
	v_div_fmas_f32 v3, v3, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v1, v3, v2, v1
	v_add_f32_e32 v1, s8, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v2, 0x4f800000, v1
	v_cmp_gt_f32_e32 vcc_lo, 0xf800000, v1
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_sqrt_f32_e32 v2, v1
	s_waitcnt_depctr 0xfff
	v_add_nc_u32_e32 v3, -1, v2
	v_add_nc_u32_e32 v4, 1, v2
	v_fma_f32 v5, -v3, v2, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v6, -v4, v2, v1
	v_cmp_ge_f32_e64 s2, 0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v2, v2, v3, s2
	v_cmp_lt_f32_e64 s2, 0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, v2, v4, s2
	s_addc_u32 s2, s7, s13
	s_cmp_lg_u64 s[10:11], 0
	s_cselect_b32 s3, -1, 0
	v_mul_f32_e32 v3, 0x37800000, v2
	s_waitcnt lgkmcnt(0)
	s_and_b32 s7, s0, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, v2, v3, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x260
	v_cndmask_b32_e32 v1, v2, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f32 v2, null, v1, v1, 1.0
	v_div_scale_f32 v5, vcc_lo, 1.0, v1, 1.0
	v_rcp_f32_e32 v3, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, -v2, v3, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v3, v4, v3
	v_mul_f32_e32 v4, v5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v6, -v2, v4, v5
	v_fmac_f32_e32 v4, v6, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v2, -v2, v4, v5
	v_div_fmas_f32 v2, v2, v3, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f32 v3, v2, v1, 1.0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB3_15
	.p2align	6
.LBB3_14:                               ;   in Loop: Header=BB3_15 Depth=1
	v_add_nc_u32_e32 v0, s7, v0
	v_add_co_u32 v1, s0, s1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v2, null, s2, v2, s0
	v_cmp_le_i32_e32 vcc_lo, s9, v0
	global_store_b32 v[1:2], v4, off
	s_or_b32 s6, vcc_lo, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s6
	s_cbranch_execz .LBB3_17
.LBB3_15:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 2, v[0:1]
	v_add_co_u32 v4, vcc_lo, s4, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s5, v2, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s3
	global_load_b32 v4, v[4:5], off
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v4, v3, v4
	s_cbranch_vccnz .LBB3_14
; %bb.16:                               ;   in Loop: Header=BB3_15 Depth=1
	v_add_co_u32 v5, vcc_lo, s10, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s11, v2, vcc_lo
	global_load_b32 v5, v[5:6], off
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v4, v4, v5
	s_branch .LBB3_14
.LBB3_17:
	s_set_inst_prefetch_distance 0x2
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_
		.amdhsa_group_segment_fixed_size 128
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
		.amdhsa_next_free_vgpr 10
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
		.amdhsa_inst_pref_size 10
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_,"axG",@progbits,_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_,comdat
.Lfunc_end3:
	.size	_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_, .Lfunc_end3-_Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_
                                        ; -- End function
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.num_vgpr, 10
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.num_agpr, 0
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.numbered_sgpr, 16
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.num_named_barrier, 0
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.private_seg_size, 0
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.uses_vcc, 1
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.uses_flat_scratch, 0
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.has_dyn_sized_stack, 0
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.has_recursion, 0
	.set _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1264
; TotalNumSgprs: 18
; NumVgprs: 10
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 128 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 18
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
	.section	.text._Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_,"axG",@progbits,_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_,comdat
	.protected	_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_ ; -- Begin function _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_
	.globl	_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_
	.p2align	8
	.type	_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_,@function
_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_: ; @_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_
; %bb.0:
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s8
	s_cbranch_scc1 .LBB4_17
; %bb.1:
	s_clause 0x2
	s_load_b64 s[12:13], s[0:1], 0x20
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[10:11], s[0:1], 0x10
	s_mul_hi_i32 s3, s9, s2
	s_mul_i32 s2, s9, s2
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	v_cmp_gt_i32_e32 vcc_lo, s9, v0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[14:15], s[12:13], 0x0
	s_lshl_b64 s[12:13], s[2:3], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s4, s4, s12
	s_addc_u32 s5, s5, s13
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB4_5
; %bb.2:
	s_load_b32 s2, s[0:1], 0x34
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_mov_b32 s8, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s16, s2, 0xffff
	.p2align	6
.LBB4_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	v_add_nc_u32_e32 v3, s16, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, s2, s4, v4
	v_add_co_ci_u32_e64 v5, null, s5, v5, s2
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s2, s9, v3
	global_load_b64 v[4:5], v[4:5], off
	s_or_b32 s8, s2, s8
	s_waitcnt vmcnt(0)
	v_fma_f64 v[1:2], v[4:5], v[4:5], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s8
	s_cbranch_execnz .LBB4_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s8
.LBB4_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s3
	v_mbcnt_lo_u32_b32 v9, -1, 0
	v_and_b32_e32 v10, 31, v0
	v_lshl_or_b32 v5, v9, 2, 64
	v_cmp_gt_u32_e64 s2, 24, v9
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 8, s2
	v_cmp_gt_u32_e64 s2, 28, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v6, v3, v9, 2
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 4, s2
	v_cmp_gt_u32_e64 s2, 30, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v7, v3, v9, 2
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 2, s2
	v_cmp_ne_u32_e64 s2, 31, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v8, v3, v9, 2
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_co_ci_u32_e64 v3, null, 0, v9, s2
	v_cmp_eq_u32_e64 s2, 0, v10
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b32_e32 v9, 2, v3
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB4_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB4_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s8, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB4_12
; %bb.8:
	s_load_b32 s3, s[0:1], 0x34
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s16, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s3, s3, 31
	s_lshr_b32 s3, s3, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s3, v10
; %bb.9:
	v_lshlrev_b32_e32 v1, 3, v10
	ds_load_b64 v[1:2], v1
; %bb.10:
	s_or_b32 exec_lo, exec_lo, s16
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_and_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB4_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_mov_b32_e32 v3, 0
	ds_store_b64 v3, v[1:2]
.LBB4_12:
	s_or_b32 exec_lo, exec_lo, s8
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB4_17
; %bb.13:
	v_cvt_f64_i32_e32 v[3:4], s9
	s_load_b32 s0, s[0:1], 0x34
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], v[1:2]
	v_div_scale_f64 v[11:12], vcc_lo, v[1:2], v[3:4], v[1:2]
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[9:10], v[11:12]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[1:2], v[5:6], v[3:4], v[1:2]
	v_add_f64 v[1:2], s[14:15], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[1:2]
	s_and_b32 s2, vcc_lo, exec_lo
	s_cselect_b32 s2, 0x100, 0
	v_ldexp_f64 v[1:2], v[1:2], s2
	s_cselect_b32 s2, 0xffffff80, 0
	s_add_u32 s1, s6, s12
	s_mov_b32 s6, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[3:4], v[1:2]
	v_cmp_class_f64_e64 vcc_lo, v[1:2], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[5:6], v[1:2], v[3:4]
	v_mul_f64 v[3:4], v[3:4], 0.5
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_fma_f64 v[3:4], v[3:4], v[7:8], v[3:4]
	v_fma_f64 v[7:8], -v[5:6], v[5:6], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[7:8], v[3:4], v[5:6]
	v_fma_f64 v[7:8], -v[5:6], v[5:6], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[7:8], v[3:4], v[5:6]
	v_ldexp_f64 v[3:4], v[3:4], s2
	s_addc_u32 s2, s7, s13
	s_cmp_lg_u64 s[10:11], 0
	s_cselect_b32 s3, -1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s7, s0, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v4, v2 :: v_dual_cndmask_b32 v1, v3, v1
	v_div_scale_f64 v[3:4], null, v[1:2], v[1:2], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[5:6], v[3:4]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_div_scale_f64 v[7:8], vcc_lo, 1.0, v[1:2], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[7:8], v[5:6]
	v_fma_f64 v[3:4], -v[3:4], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[3:4], v[3:4], v[5:6], v[9:10]
	v_div_fixup_f64 v[2:3], v[3:4], v[1:2], 1.0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB4_15
	.p2align	6
.LBB4_14:                               ;   in Loop: Header=BB4_15 Depth=1
	v_add_nc_u32_e32 v0, s7, v0
	v_add_co_u32 v4, s0, s1, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v5, null, s2, v5, s0
	v_cmp_le_i32_e32 vcc_lo, s9, v0
	global_store_b64 v[4:5], v[6:7], off
	s_or_b32 s6, vcc_lo, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s6
	s_cbranch_execz .LBB4_17
.LBB4_15:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 3, v[0:1]
	v_add_co_u32 v6, vcc_lo, s4, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s5, v5, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s3
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[6:7], v[2:3], v[6:7]
	s_cbranch_vccnz .LBB4_14
; %bb.16:                               ;   in Loop: Header=BB4_15 Depth=1
	v_add_co_u32 v8, vcc_lo, s10, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s11, v5, vcc_lo
	global_load_b64 v[8:9], v[8:9], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[6:7], v[6:7], v[8:9]
	s_branch .LBB4_14
.LBB4_17:
	s_set_inst_prefetch_distance 0x2
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_
		.amdhsa_group_segment_fixed_size 256
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
		.amdhsa_next_free_vgpr 13
		.amdhsa_next_free_sgpr 17
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
	.section	.text._Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_,"axG",@progbits,_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_,comdat
.Lfunc_end4:
	.size	_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_, .Lfunc_end4-_Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_
                                        ; -- End function
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.num_vgpr, 13
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.num_agpr, 0
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.numbered_sgpr, 17
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.num_named_barrier, 0
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.private_seg_size, 0
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.uses_vcc, 1
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.uses_flat_scratch, 0
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.has_dyn_sized_stack, 0
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.has_recursion, 0
	.set _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1468
; TotalNumSgprs: 19
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 19
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_ee853abf7dfa3674,@object ; @__hip_cuid_ee853abf7dfa3674
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_ee853abf7dfa3674
__hip_cuid_ee853abf7dfa3674:
	.byte	0                               ; 0x0
	.size	__hip_cuid_ee853abf7dfa3674, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_ee853abf7dfa3674
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
      - .offset:         32
        .size:           4
        .value_kind:     by_value
      - .offset:         36
        .size:           4
        .value_kind:     by_value
      - .offset:         40
        .size:           4
        .value_kind:     by_value
      - .offset:         44
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           8
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
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 312
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           normx_groupnorm_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     26
    .sgpr_spill_count: 0
    .symbol:         normx_groupnorm_kernel.kd
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
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .offset:         32
        .size:           4
        .value_kind:     by_value
      - .offset:         36
        .size:           4
        .value_kind:     by_value
      - .offset:         40
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           8
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
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 312
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           normx_instancenorm_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     27
    .sgpr_spill_count: 0
    .symbol:         normx_instancenorm_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         20
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           8
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
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           normx_l2_normalize_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     15
    .sgpr_spill_count: 0
    .symbol:         normx_l2_normalize_kernel.kd
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
        .size:           4
        .value_kind:     by_value
      - .offset:         28
        .size:           4
        .value_kind:     by_value
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
    .group_segment_fixed_size: 128
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z20normx_rmsnorm_kernelIfEvPKT_PS0_S2_iiS2_.kd
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
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .offset:         28
        .size:           4
        .value_kind:     by_value
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
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_
    .private_segment_fixed_size: 0
    .sgpr_count:     19
    .sgpr_spill_count: 0
    .symbol:         _Z20normx_rmsnorm_kernelIdEvPKT_PS0_S2_iiS2_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     13
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
