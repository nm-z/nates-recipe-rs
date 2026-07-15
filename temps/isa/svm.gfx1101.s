	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_ ; -- Begin function _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_
	.globl	_Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_
	.p2align	8
	.type	_Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_,@function
_Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_: ; @_Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_
; %bb.0:
	s_clause 0x2
	s_load_b128 s[12:15], s[0:1], 0x20
	s_load_b64 s[16:17], s[0:1], 0x30
	s_load_b256 s[4:11], s[0:1], 0x0
	v_bfe_u32 v12, v0, 10, 10
	v_and_b32_e32 v13, 0x3ff, v0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[12:13], s[12:13], 0x0
	s_load_b64 s[14:15], s[14:15], 0x0
	s_load_b64 s[16:17], s[16:17], 0x0
	v_lshl_add_u32 v11, s3, 4, v12
	v_lshl_add_u32 v0, s2, 4, v13
	s_cmp_lt_i32 s9, 1
	s_cbranch_scc1 .LBB0_12
; %bb.1:
	v_lshlrev_b32_e32 v1, 3, v13
	v_lshlrev_b32_e32 v15, 7, v12
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	v_mul_lo_u32 v14, s9, v11
	v_add_nc_u32_e32 v16, 0x800, v1
	v_mul_lo_u32 v17, s9, v0
	s_add_i32 s2, s9, 15
	v_dual_mov_b32 v5, 0 :: v_dual_add_nc_u32 v18, v15, v1
	v_mov_b32_e32 v1, 0
	v_cmp_gt_i32_e64 s0, s8, v11
	v_cmp_gt_i32_e64 s1, s8, v0
	s_lshr_b32 s2, s2, 4
	v_dual_mov_b32 v2, 0 :: v_dual_add_nc_u32 v19, v16, v15
	v_mov_b32_e32 v6, 0
	s_cmp_eq_u32 s10, 1
	s_mov_b32 s11, 0
	s_cselect_b32 s3, -1, 0
	s_mov_b32 s18, s9
	s_branch .LBB0_3
.LBB0_2:                                ;   in Loop: Header=BB0_3 Depth=1
	s_add_i32 s11, s11, 1
	s_add_i32 s18, s18, -16
	s_cmp_eq_u32 s11, s2
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB0_13
.LBB0_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB0_10 Depth 2
	s_lshl_b32 s19, s11, 4
	v_mov_b32_e32 v7, 0
	v_dual_mov_b32 v8, 0 :: v_dual_add_nc_u32 v9, s19, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s9, v9
	s_and_b32 s21, s0, vcc_lo
	s_and_saveexec_b32 s20, s21
	s_cbranch_execz .LBB0_5
; %bb.4:                                ;   in Loop: Header=BB0_3 Depth=1
	v_add_nc_u32_e32 v7, v9, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s4, v7
	v_add_co_ci_u32_e64 v8, null, s5, v8, vcc_lo
	global_load_b64 v[7:8], v[7:8], off
.LBB0_5:                                ;   in Loop: Header=BB0_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s20
	v_dual_mov_b32 v9, 0 :: v_dual_add_nc_u32 v20, s19, v12
	v_mov_b32_e32 v10, 0
	s_waitcnt vmcnt(0)
	ds_store_b64 v18, v[7:8]
	v_cmp_gt_i32_e32 vcc_lo, s9, v20
	s_and_b32 s21, s1, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s20, s21
	s_cbranch_execz .LBB0_7
; %bb.6:                                ;   in Loop: Header=BB0_3 Depth=1
	v_add_nc_u32_e32 v7, v20, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s4, v7
	v_add_co_ci_u32_e64 v8, null, s5, v8, vcc_lo
	global_load_b64 v[9:10], v[7:8], off
.LBB0_7:                                ;   in Loop: Header=BB0_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s20
	s_cmp_le_i32 s9, s19
	s_waitcnt vmcnt(0)
	ds_store_b64 v19, v[9:10]
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB0_2
; %bb.8:                                ;   in Loop: Header=BB0_3 Depth=1
	v_med3_i32 v20, s18, 1, 16
	v_dual_mov_b32 v21, v16 :: v_dual_mov_b32 v22, v15
	s_branch .LBB0_10
	.p2align	6
.LBB0_9:                                ;   in Loop: Header=BB0_10 Depth=2
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[1:2], v[7:8], v[9:10], v[1:2]
	v_add_nc_u32_e32 v20, -1, v20
	v_add_nc_u32_e32 v22, 8, v22
	v_add_nc_u32_e32 v21, 0x80, v21
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_eq_u32_e32 vcc_lo, 0, v20
	s_cbranch_vccnz .LBB0_2
.LBB0_10:                               ;   Parent Loop BB0_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ds_load_b64 v[7:8], v22
	ds_load_b64 v[9:10], v21
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccnz .LBB0_9
; %bb.11:                               ;   in Loop: Header=BB0_10 Depth=2
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[3:4], v[7:8], v[7:8], v[3:4]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[5:6], v[9:10], v[9:10], v[5:6]
	s_branch .LBB0_9
.LBB0_12:
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v4, 0
	v_mov_b32_e32 v6, 0
.LBB0_13:
	v_max_i32_e32 v7, v11, v0
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s8, v7
	s_cbranch_execz .LBB0_28
; %bb.14:
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc1 .LBB0_18
; %bb.15:
	s_cmp_gt_i32 s10, 1
	s_cbranch_scc0 .LBB0_19
; %bb.16:
	s_cmp_eq_u32 s10, 2
	s_cbranch_scc0 .LBB0_20
; %bb.17:
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[9:10], s[12:13], v[1:2], s[14:15]
	v_mov_b32_e32 v7, s17
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s2, 0x968915a9
	s_mov_b32 s4, 0x4222de17
	s_mov_b32 s3, 0x3fba6564
	s_mov_b32 s5, 0x3fbdee67
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 1.0, v[9:10]
	v_cndmask_b32_e32 v8, 0x3ff00000, v7, vcc_lo
	v_cndmask_b32_e64 v7, 0, s16, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[7:8]
	v_cndmask_b32_e32 v10, 0x3ff00000, v10, vcc_lo
	v_cndmask_b32_e32 v9, 0, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[12:13], |v[9:10]|
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[12:13]
	v_cndmask_b32_e64 v14, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[12:13], v[12:13], v14
	v_add_f64 v[14:15], v[12:13], 1.0
	v_add_f64 v[20:21], v[12:13], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[16:17], v[14:15]
	v_add_f64 v[22:23], v[14:15], -1.0
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[18:19], v[16:17], v[16:17]
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[18:19], v[16:17], v[16:17]
	v_mul_f64 v[18:19], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[24:25], v[14:15], v[18:19]
	v_fma_f64 v[14:15], v[18:19], v[14:15], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[18:19], v[12:13], v[14:15]
	v_add_f64 v[14:15], v[24:25], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[20:21], -v[14:15]
	v_add_f64 v[24:25], v[14:15], -v[24:25]
	v_add_f64 v[20:21], v[20:21], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[24:25], -v[12:13]
	v_add_f64 v[14:15], v[20:21], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], v[14:15]
	v_add_f64 v[12:13], v[22:23], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[16:17], v[12:13]
	v_add_f64 v[14:15], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[18:19]
	v_mul_f64 v[18:19], v[14:15], v[14:15]
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], v[14:15], -v[18:19]
	v_add_f64 v[20:21], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[20:21], v[16:17]
	v_add_f64 v[20:21], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], s[4:5], s[2:3]
	s_mov_b32 s2, 0x3abe935a
	s_mov_b32 s3, 0x3fbe25e4
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	v_mul_f64 v[28:29], v[14:15], v[20:21]
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[2:3]
	s_mov_b32 s2, 0x47e6c9c2
	s_mov_b32 s3, 0x3fc110ef
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[2:3]
	s_mov_b32 s2, 0xcfa74449
	s_mov_b32 s3, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[2:3]
	s_mov_b32 s2, 0x71bf3c30
	s_mov_b32 s3, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[2:3]
	s_mov_b32 s2, 0x1c7792ce
	s_mov_b32 s3, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[2:3]
	s_mov_b32 s2, 0x924920da
	s_mov_b32 s3, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[2:3]
	s_mov_b32 s2, 0x9999999c
	s_mov_b32 s3, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[2:3]
	s_mov_b32 s3, 0x3c7abc9e
	s_mov_b32 s2, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[24:25], v[20:21], v[22:23]
	v_fma_f64 v[18:19], v[20:21], v[22:23], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[22:23], v[18:19]
	v_add_f64 v[22:23], v[24:25], v[18:19]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[26:27], v[22:23], s[0:1]
	v_add_f64 v[24:25], v[22:23], -v[24:25]
	s_mov_b32 s1, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[30:31], v[26:27], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_fma_f64 v[24:25], v[20:21], v[14:15], -v[28:29]
	s_mov_b32 s0, 0xd5df274d
	s_mov_b32 s1, 0x3c8543b0
	v_add_f64 v[22:23], v[22:23], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[18:19], s[0:1]
	v_fma_f64 v[20:21], v[20:21], v[12:13], v[24:25]
	s_mov_b32 s1, 0x3fe62e42
	s_mov_b32 s0, 0xfefa39ef
	v_ldexp_f64 v[12:13], v[12:13], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[18:19], v[22:23]
	v_fma_f64 v[16:17], v[16:17], v[14:15], v[20:21]
	v_ldexp_f64 v[14:15], v[14:15], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[26:27], v[18:19]
	v_add_f64 v[22:23], v[28:29], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[26:27], -v[20:21]
	v_mul_f64 v[26:27], v[22:23], v[20:21]
	v_add_f64 v[28:29], v[22:23], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[18:19], v[24:25]
	v_fma_f64 v[24:25], v[22:23], v[20:21], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[28:29]
	v_fma_f64 v[18:19], v[22:23], v[18:19], v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[16:17], v[20:21], v[18:19]
	v_frexp_exp_i32_f64_e32 v20, v[9:10]
	v_add_f64 v[18:19], v[26:27], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v20, null, 0, v20, vcc_lo
	v_cvt_f64_i32_e32 v[20:21], v20
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[14:15], v[18:19]
	v_add_f64 v[24:25], v[18:19], -v[26:27]
	v_mul_f64 v[26:27], v[20:21], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[22:23], -v[14:15]
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[24:25], v[20:21], s[0:1], -v[26:27]
	s_mov_b32 s1, 0xbfe62e42
	v_add_f64 v[14:15], v[18:19], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], v[16:17]
	v_fma_f64 v[16:17], v[20:21], s[2:3], v[24:25]
	s_mov_b32 s3, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[14:15]
	v_add_f64 v[14:15], v[26:27], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[22:23], v[12:13]
	v_add_f64 v[26:27], v[14:15], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[14:15], v[18:19]
	v_add_f64 v[22:23], v[18:19], -v[22:23]
	v_add_f64 v[16:17], v[16:17], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[20:21], -v[14:15]
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[28:29], v[20:21], -v[24:25]
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_add_f64 v[22:23], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[28:29]
	v_add_f64 v[14:15], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[22:23], -v[16:17]
	v_add_f64 v[14:15], v[22:23], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[22:23], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_add_f64 v[24:25], v[20:21], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[22:23]
	v_add_f64 v[18:19], v[24:25], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], v[14:15]
	v_add_f64 v[14:15], v[24:25], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[24:25]
	v_mul_f64 v[18:19], v[7:8], v[14:15]
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[7:8], v[14:15], -v[18:19]
	v_cmp_class_f64_e64 vcc_lo, v[18:19], 0x204
	v_fma_f64 v[12:13], v[7:8], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[18:19], v[12:13]
	v_dual_cndmask_b32 v17, v15, v19 :: v_dual_cndmask_b32 v16, v14, v18
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[20:21], v[16:17], s[4:5]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[16:17]|
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_trunc_f64_e32 v[14:15], v[7:8]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rndne_f64_e32 v[20:21], v[20:21]
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	v_cmp_lt_f64_e64 s4, |v[9:10]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[20:21], s[0:1], v[16:17]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_cvt_i32_f64_e32 v26, v[20:21]
	v_fma_f64 v[22:23], v[20:21], s[2:3], v[22:23]
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_cmp_neq_f64_e64 s3, v[7:8], |v[7:8]|
	v_cmp_eq_f64_e64 s2, 0, v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_xor_b32 s3, s3, s4
	v_cmp_class_f64_e64 s4, v[9:10], 0x204
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[16:17]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], 1.0
	s_and_b32 vcc_lo, s1, s0
	v_fma_f64 v[20:21], v[22:23], v[24:25], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[18:19], v[20:21], v26
	v_mul_f64 v[20:21], v[7:8], 0.5
	v_cndmask_b32_e64 v19, 0x7ff00000, v19, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[16:17], v[20:21]
	v_cndmask_b32_e32 v18, 0, v18, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[14:15], v[7:8]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v19, 0, v19, s1
	v_fma_f64 v[12:13], v[18:19], v[12:13], v[18:19]
	v_cmp_class_f64_e64 s1, v[18:19], 0x204
	v_cmp_neq_f64_e64 s0, v[16:17], v[20:21]
	v_cndmask_b32_e64 v16, 0x7ff00000, 0, s3
	v_cmp_neq_f64_e64 s3, |v[9:10]|, 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v13, v13, v19, s1
	v_cndmask_b32_e64 v12, v12, v18, s1
	v_cmp_gt_f64_e64 s1, 0, v[7:8]
	v_cndmask_b32_e32 v15, 0, v12, vcc_lo
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v14, 0x3ff00000, v10, s0
	v_cndmask_b32_e64 v16, 0x3ff00000, v16, s3
	v_bfi_b32 v13, 0x7fffffff, v13, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v14, 0x7ff80000, v13, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[9:10]
	s_xor_b32 s1, s1, s2
	v_dual_cndmask_b32 v12, v12, v15 :: v_dual_cndmask_b32 v13, v13, v14
	v_cmp_class_f64_e64 vcc_lo, v[7:8], 0x204
	v_cndmask_b32_e64 v14, 0x7ff00000, 0, s1
	v_cndmask_b32_e64 v15, 0, v10, s0
	s_or_b32 s0, s2, s4
	s_mov_b32 s1, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_bfi_b32 v14, 0x7fffffff, v14, v15
	v_cndmask_b32_e32 v13, v13, v16, vcc_lo
	v_cndmask_b32_e64 v13, v13, v14, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[9:10], v[7:8]
	v_cndmask_b32_e64 v12, v12, 0, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v7, 0, v12, vcc_lo
	v_cndmask_b32_e32 v8, 0x7ff80000, v13, vcc_lo
	s_branch .LBB0_21
.LBB0_18:
	s_mov_b32 s1, 0
                                        ; implicit-def: $vgpr7_vgpr8
	s_cbranch_execnz .LBB0_24
	s_branch .LBB0_25
.LBB0_19:
	s_mov_b32 s1, 0
                                        ; implicit-def: $vgpr7_vgpr8
	s_cbranch_execnz .LBB0_22
	s_branch .LBB0_23
.LBB0_20:
	s_mov_b32 s1, -1
                                        ; implicit-def: $vgpr7_vgpr8
.LBB0_21:
	s_branch .LBB0_23
.LBB0_22:
	v_add_f64 v[3:4], v[5:6], v[3:4]
	s_mov_b32 s2, 0x652b82fe
	s_mov_b32 s3, 0x3ff71547
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[1:2], -2.0, v[3:4]
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[3:4]
	v_dual_cndmask_b32 v4, 0, v4 :: v_dual_cndmask_b32 v3, 0, v3
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[3:4], v[3:4], -s[12:13]
	v_mul_f64 v[5:6], v[3:4], s[2:3]
	s_mov_b32 s2, 0xfefa39ef
	s_mov_b32 s3, 0xbfe62e42
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[3:4]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[5:6], v[5:6]
	v_fma_f64 v[7:8], v[5:6], s[2:3], v[3:4]
	s_mov_b32 s2, 0x3b39803f
	s_mov_b32 s3, 0xbc7abc9e
	v_cvt_i32_f64_e32 v12, v[5:6]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[7:8], v[5:6], s[2:3], v[7:8]
	s_mov_b32 s2, 0xfca7ab0c
	s_mov_b32 s3, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], s[4:5], s[2:3]
	s_mov_b32 s2, 0x623fde64
	s_mov_b32 s3, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x7c89e6b0
	s_mov_b32 s3, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x14761f6e
	s_mov_b32 s3, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x11122322
	s_mov_b32 s3, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x555502a1
	s_mov_b32 s3, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x55555511
	s_mov_b32 s3, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 11
	s_mov_b32 s3, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], 1.0
	v_fma_f64 v[5:6], v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v12
	v_cndmask_b32_e32 v6, 0x7ff00000, v6, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0, v5, vcc_lo
	v_cndmask_b32_e64 v8, 0, v6, s0
.LBB0_23:
	s_branch .LBB0_25
.LBB0_24:
	v_dual_mov_b32 v8, v2 :: v_dual_mov_b32 v7, v1
	s_cmp_lg_u32 s10, 0
	s_cselect_b32 s1, -1, 0
.LBB0_25:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s1
	s_cbranch_vccnz .LBB0_27
; %bb.26:
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[1:2], s[12:13], v[1:2], s[14:15]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[3:4], |v[1:2]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[1:2]|
	v_rndne_f64_e32 v[3:4], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[5:6], v[3:4], s[0:1], |v[1:2]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	v_mul_f64 v[7:8], v[3:4], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[5:6], 0
	v_add_f64 v[12:13], v[9:10], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[12:13]
	v_add_f64 v[5:6], v[5:6], 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[9:10], v[7:8]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_mul_f64 v[7:8], v[3:4], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[12:13], v[5:6]
	v_add_f64 v[14:15], v[9:10], v[7:8]
	v_add_f64 v[12:13], v[12:13], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[14:15]
	v_add_f64 v[5:6], v[5:6], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[9:10], v[7:8]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[14:15], v[5:6]
	v_fma_f64 v[9:10], v[7:8], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[12:13], v[14:15], -v[7:8]
	v_mul_f64 v[14:15], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	v_add_f64 v[5:6], v[5:6], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[7:8], v[7:8], -v[14:15]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[5:6], v[5:6]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[7:8], v[16:17], v[12:13]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], v[12:13]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[16:17], v[9:10]
	v_fma_f64 v[14:15], v[16:17], v[9:10], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[12:13], v[9:10], v[14:15]
	v_add_f64 v[12:13], v[18:19], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[7:8], v[12:13]
	v_add_f64 v[16:17], v[12:13], -v[18:19]
	v_add_f64 v[7:8], v[14:15], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], -v[16:17]
	v_cvt_i32_f64_e32 v16, v[3:4]
	v_add_f64 v[7:8], v[12:13], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[14:15], v[5:6]
	v_add_f64 v[9:10], v[7:8], 1.0
	v_add_f64 v[12:13], v[7:8], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[9:10], -1.0
	v_add_f64 v[5:6], v[5:6], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[14:15]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[9:10], v[5:6]
	v_ldexp_f64 v[7:8], v[3:4], v16
	v_add_f64 v[3:4], v[3:4], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[12:13], v[7:8]
	v_add_f64 v[3:4], v[5:6], -v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v16
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[7:8], v[12:13], 1.0
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[7:8], v[12:13], 1.0
	v_fma_f64 v[9:10], v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[5:6], v[7:8], v[9:10]
	v_fma_f64 v[12:13], v[9:10], v[7:8], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[9:10], v[3:4], v[12:13]
	v_add_f64 v[14:15], v[5:6], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], -v[14:15], 1.0
	v_add_f64 v[5:6], v[14:15], -v[5:6]
	v_add_f64 v[18:19], -v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[12:13]
	v_add_f64 v[12:13], v[18:19], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[12:13]
	v_add_f64 v[12:13], v[16:17], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[14:15], v[9:10], v[12:13]
	v_add_f64 v[16:17], v[16:17], -v[12:13]
	v_mul_f64 v[18:19], v[7:8], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[16:17]
	v_fma_f64 v[20:21], v[14:15], v[7:8], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[14:15], v[3:4], v[20:21]
	v_add_f64 v[22:23], v[18:19], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[12:13], -v[22:23]
	v_add_f64 v[16:17], v[22:23], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[12:13]
	v_add_f64 v[12:13], v[9:10], v[14:15]
	v_add_f64 v[5:6], v[16:17], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], -v[9:10]
	v_add_f64 v[5:6], v[24:25], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_mul_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[14:15], v[5:6]
	v_add_f64 v[9:10], v[12:13], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[7:8], v[9:10]
	v_add_f64 v[12:13], v[9:10], -v[12:13]
	v_add_f64 v[18:19], v[7:8], -v[9:10]
	v_add_f64 v[16:17], v[14:15], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], -v[12:13]
	v_add_f64 v[7:8], v[7:8], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[9:10], -v[16:17]
	v_add_f64 v[16:17], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_add_f64 v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_add_f64 v[16:17], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	v_rcp_f64_e32 v[20:21], v[16:17]
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[16:17], v[20:21], 1.0
	v_fma_f64 v[7:8], v[9:10], v[20:21], v[20:21]
	v_add_f64 v[9:10], v[18:19], v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[16:17], v[7:8], 1.0
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[9:10], v[5:6]
	v_mul_f64 v[20:21], v[16:17], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[7:8], v[16:17], -v[20:21]
	v_fma_f64 v[12:13], v[7:8], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[20:21], v[12:13]
	v_add_f64 v[16:17], v[9:10], -v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[9:10], -v[16:17]
	v_add_f64 v[9:10], v[9:10], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[22:23], -v[14:15]
	v_add_f64 v[3:4], v[3:4], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[14:15], -v[12:13]
	v_add_f64 v[3:4], v[3:4], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[16:17], v[3:4]
	v_mul_f64 v[3:4], v[5:6], v[3:4]
	v_and_b32_e32 v5, 0x7fffffff, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[7:8], v[3:4]
	v_cndmask_b32_e32 v4, 0x3ff00000, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[1:2]|
	v_cndmask_b32_e32 v7, v3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v4, v5, vcc_lo
	v_bfi_b32 v8, 0x7fffffff, v1, v2
.LBB0_27:
	v_mad_u64_u32 v[1:2], null, s8, v11, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[7:8], off
.LBB0_28:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_
		.amdhsa_group_segment_fixed_size 4096
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 56
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
		.amdhsa_next_free_vgpr 32
		.amdhsa_next_free_sgpr 22
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
		.amdhsa_inst_pref_size 40
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
	.size	_Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_, .Lfunc_end0-_Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_
                                        ; -- End function
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.num_vgpr, 32
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.num_agpr, 0
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.numbered_sgpr, 22
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.num_named_barrier, 0
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.private_seg_size, 0
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.uses_vcc, 1
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.uses_flat_scratch, 0
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.has_dyn_sized_stack, 0
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.has_recursion, 0
	.set _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 5056
; TotalNumSgprs: 24
; NumVgprs: 32
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 4096 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 24
; NumVGPRsForWavesPerEU: 32
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
	.protected	_Z26smo_update_gradient_kernelPdPKdiiidd ; -- Begin function _Z26smo_update_gradient_kernelPdPKdiiidd
	.globl	_Z26smo_update_gradient_kernelPdPKdiiidd
	.p2align	8
	.type	_Z26smo_update_gradient_kernelPdPKdiiidd,@function
_Z26smo_update_gradient_kernelPdPKdiiidd: ; @_Z26smo_update_gradient_kernelPdPKdiiidd
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_2
; %bb.1:
	s_load_b128 s[8:11], s[0:1], 0x0
	v_mad_u64_u32 v[2:3], null, s6, s4, v[1:2]
	s_load_b128 s[0:3], s[0:1], 0x20
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s10, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v3, null, s11, v3, vcc_lo
	v_mad_u64_u32 v[5:6], null, s5, s4, v[1:2]
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	v_add_co_u32 v5, vcc_lo, s10, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s11, v6, vcc_lo
	global_load_b64 v[5:6], v[5:6], off
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s8, v0
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	global_load_b64 v[7:8], v[0:1], off
	s_waitcnt vmcnt(2)
	v_mul_f64 v[2:3], s[2:3], v[3:4]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], s[0:1], v[5:6], v[2:3]
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[7:8], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z26smo_update_gradient_kernelPdPKdiiidd
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
		.amdhsa_next_free_vgpr 9
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
.Lfunc_end1:
	.size	_Z26smo_update_gradient_kernelPdPKdiiidd, .Lfunc_end1-_Z26smo_update_gradient_kernelPdPKdiiidd
                                        ; -- End function
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.num_vgpr, 9
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.num_agpr, 0
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.numbered_sgpr, 12
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.num_named_barrier, 0
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.private_seg_size, 0
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.uses_vcc, 1
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.uses_flat_scratch, 0
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.has_dyn_sized_stack, 0
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.has_recursion, 0
	.set _Z26smo_update_gradient_kernelPdPKdiiidd.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 276
; TotalNumSgprs: 14
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 9
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
	.protected	_Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_ ; -- Begin function _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_
	.globl	_Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_
	.p2align	8
	.type	_Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_,@function
_Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_: ; @_Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
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
	s_load_b64 s[2:3], s[0:1], 0x30
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s8, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s9, v3, vcc_lo
	v_add_co_u32 v4, vcc_lo, s4, v2
	v_add_co_ci_u32_e64 v5, null, s5, v3, vcc_lo
	v_add_co_u32 v6, vcc_lo, s6, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s7, v3, vcc_lo
	global_load_b64 v[0:1], v[0:1], off
	global_load_b64 v[4:5], v[4:5], off
	s_load_b64 s[4:5], s[2:3], 0x0
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(2)
	v_cmp_lt_f64_e32 vcc_lo, 0, v[0:1]
	s_waitcnt vmcnt(1)
	v_mul_f64 v[4:5], v[0:1], -v[4:5]
	s_waitcnt vmcnt(0)
	v_cmp_lt_f64_e64 s2, 0, v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v0, 0, 1, s2
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e64 s2, s[4:5], v[6:7]
	v_cndmask_b32_e64 v1, 0, 1, s2
	s_load_b64 s[2:3], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b16 v0.h, v0.l, v1.l, vcc_lo
	v_cndmask_b16 v0.l, v1.l, v0.l, vcc_lo
	v_add_co_u32 v6, vcc_lo, s10, v2
	v_add_co_ci_u32_e64 v7, null, s11, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_and_b16 v0.h, 1, v0.h
	v_and_b16 v0.l, 1, v0.l
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_eq_u16_e32 vcc_lo, 1, v0.h
	v_cmp_eq_u16_e64 s0, 1, v0.l
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, s1, s2, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v3, s1
	v_cndmask_b32_e32 v3, 0xffe1ccf3, v5, vcc_lo
	v_cndmask_b32_e32 v2, 0x85ebc8a0, v4, vcc_lo
	v_cndmask_b32_e64 v5, 0xffe1ccf3, v5, s0
	v_cndmask_b32_e64 v4, 0x85ebc8a0, v4, s0
	global_store_b64 v[6:7], v[2:3], off
	global_store_b64 v[0:1], v[4:5], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_
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
.Lfunc_end2:
	.size	_Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_, .Lfunc_end2-_Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_
                                        ; -- End function
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.num_vgpr, 8
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.num_agpr, 0
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.numbered_sgpr, 12
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.num_named_barrier, 0
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.private_seg_size, 0
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.uses_vcc, 1
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.uses_flat_scratch, 0
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.has_dyn_sized_stack, 0
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.has_recursion, 0
	.set _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 412
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
	.protected	_Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_ ; -- Begin function _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_
	.globl	_Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_
	.p2align	8
	.type	_Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_,@function
_Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_: ; @_Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b128 s[8:11], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB3_21
; %bb.1:
	s_clause 0x2
	s_load_b64 s[12:13], s[0:1], 0x30
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b128 s[0:3], s[0:1], 0x20
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s9, 1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[12:13], s[12:13], 0x0
	s_cbranch_scc1 .LBB3_6
; %bb.2:
	v_mad_i64_i32 v[5:6], null, v1, s9, 0
	s_mul_hi_i32 s15, s10, s9
	s_mul_i32 s14, s10, s9
	v_mov_b32_e32 v3, 0
	s_lshl_b64 s[14:15], s[14:15], 3
	v_mov_b32_e32 v4, 0
	s_add_u32 s14, s4, s14
	v_lshlrev_b64 v[7:8], 3, v[5:6]
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_addc_u32 s15, s5, s15
	s_cmp_eq_u32 s11, 1
	s_cselect_b32 s8, -1, 0
	v_add_co_u32 v9, vcc_lo, s4, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s5, v8, vcc_lo
	v_mov_b32_e32 v7, 0
	v_mov_b32_e32 v8, 0
	s_branch .LBB3_4
	.p2align	6
.LBB3_3:                                ;   in Loop: Header=BB3_4 Depth=1
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[3:4], s[4:5], v[11:12], v[3:4]
	v_add_co_u32 v9, vcc_lo, v9, 8
	s_add_i32 s9, s9, -1
	v_add_co_ci_u32_e64 v10, null, 0, v10, vcc_lo
	s_add_u32 s14, s14, 8
	s_addc_u32 s15, s15, 0
	s_cmp_eq_u32 s9, 0
	s_cbranch_scc1 .LBB3_7
.LBB3_4:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[11:12], v[9:10], off
	s_load_b64 s[4:5], s[14:15], 0x0
	s_and_not1_b32 vcc_lo, exec_lo, s8
	s_cbranch_vccnz .LBB3_3
; %bb.5:                                ;   in Loop: Header=BB3_4 Depth=1
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[5:6], s[4:5], s[4:5], v[5:6]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[7:8], v[11:12], v[11:12], v[7:8]
	s_branch .LBB3_3
.LBB3_6:
	v_mov_b32_e32 v3, 0
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, 0
	v_dual_mov_b32 v7, 0 :: v_dual_mov_b32 v6, 0
	v_mov_b32_e32 v8, 0
.LBB3_7:
	s_load_b64 s[4:5], s[0:1], 0x0
	s_load_b64 s[8:9], s[2:3], 0x0
	s_cmp_lt_i32 s11, 1
	s_cbranch_scc1 .LBB3_11
; %bb.8:
	s_cmp_gt_i32 s11, 1
	s_cbranch_scc0 .LBB3_12
; %bb.9:
	s_cmp_eq_u32 s11, 2
	s_cbranch_scc0 .LBB3_13
; %bb.10:
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[11:12], s[4:5], v[3:4], s[8:9]
	v_mov_b32_e32 v0, s13
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s2, 0x968915a9
	s_mov_b32 s3, 0x3fba6564
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 1.0, v[11:12]
	v_cndmask_b32_e32 v10, 0x3ff00000, v0, vcc_lo
	v_cndmask_b32_e64 v9, 0, s12, vcc_lo
	s_mov_b32 s12, 0x4222de17
	s_mov_b32 s13, 0x3fbdee67
	v_cmp_neq_f64_e32 vcc_lo, 0, v[9:10]
	v_cndmask_b32_e32 v12, 0x3ff00000, v12, vcc_lo
	v_cndmask_b32_e32 v11, 0, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_frexp_mant_f64_e64 v[13:14], |v[11:12]|
	v_cmp_lt_f64_e64 s10, |v[11:12]|, 1.0
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[13:14]
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[13:14], v[13:14], v0
	v_frexp_exp_i32_f64_e32 v0, v[11:12]
	v_add_f64 v[15:16], v[13:14], 1.0
	v_add_f64 v[21:22], v[13:14], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	v_rcp_f64_e32 v[17:18], v[15:16]
	v_add_f64 v[23:24], v[15:16], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[13:14], -v[23:24]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[19:20], -v[15:16], v[17:18], 1.0
	v_fma_f64 v[17:18], v[19:20], v[17:18], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], -v[15:16], v[17:18], 1.0
	v_fma_f64 v[17:18], v[19:20], v[17:18], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[21:22], v[17:18]
	v_mul_f64 v[25:26], v[15:16], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[19:20], v[15:16], -v[25:26]
	v_fma_f64 v[13:14], v[19:20], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[25:26], v[13:14]
	v_add_f64 v[23:24], v[21:22], -v[15:16]
	v_add_f64 v[25:26], v[15:16], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[21:22], -v[23:24]
	v_add_f64 v[13:14], v[25:26], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[21:22], -v[15:16]
	v_add_f64 v[13:14], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[23:24], v[13:14]
	v_mul_f64 v[13:14], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[19:20], v[13:14]
	v_add_f64 v[17:18], v[15:16], -v[19:20]
	v_mul_f64 v[19:20], v[15:16], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	v_fma_f64 v[17:18], v[15:16], v[15:16], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[21:22], v[13:14], v[13:14]
	v_fma_f64 v[17:18], v[15:16], v[21:22], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[21:22], v[19:20], v[17:18]
	v_fma_f64 v[23:24], v[21:22], s[12:13], s[2:3]
	s_mov_b32 s2, 0x3abe935a
	s_mov_b32 s3, 0x3fbe25e4
	v_add_f64 v[19:20], v[21:22], -v[19:20]
	v_mul_f64 v[29:30], v[15:16], v[21:22]
	s_mov_b32 s12, 0x652b82fe
	s_mov_b32 s13, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[2:3]
	s_mov_b32 s2, 0x47e6c9c2
	s_mov_b32 s3, 0x3fc110ef
	v_add_f64 v[17:18], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[2:3]
	s_mov_b32 s2, 0xcfa74449
	s_mov_b32 s3, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[2:3]
	s_mov_b32 s2, 0x71bf3c30
	s_mov_b32 s3, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[2:3]
	s_mov_b32 s2, 0x1c7792ce
	s_mov_b32 s3, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[2:3]
	s_mov_b32 s2, 0x924920da
	s_mov_b32 s3, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[2:3]
	s_mov_b32 s2, 0x9999999c
	s_mov_b32 s3, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[2:3]
	s_mov_b32 s3, 0x3c7abc9e
	s_mov_b32 s2, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[25:26], v[21:22], v[23:24]
	v_fma_f64 v[19:20], v[21:22], v[23:24], -v[25:26]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[23:24], v[19:20]
	v_add_f64 v[23:24], v[25:26], v[19:20]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[27:28], v[23:24], s[0:1]
	v_add_f64 v[25:26], v[23:24], -v[25:26]
	s_mov_b32 s1, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[31:32], v[27:28], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[19:20], -v[25:26]
	v_fma_f64 v[25:26], v[21:22], v[15:16], -v[29:30]
	s_mov_b32 s0, 0xd5df274d
	s_mov_b32 s1, 0x3c8543b0
	v_add_f64 v[23:24], v[23:24], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[19:20], s[0:1]
	v_fma_f64 v[21:22], v[21:22], v[13:14], v[25:26]
	s_mov_b32 s1, 0x3fe62e42
	s_mov_b32 s0, 0xfefa39ef
	v_ldexp_f64 v[13:14], v[13:14], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[19:20], v[23:24]
	v_fma_f64 v[17:18], v[17:18], v[15:16], v[21:22]
	v_ldexp_f64 v[15:16], v[15:16], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[27:28], v[19:20]
	v_add_f64 v[23:24], v[29:30], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[27:28], -v[21:22]
	v_mul_f64 v[27:28], v[23:24], v[21:22]
	v_add_f64 v[29:30], v[23:24], -v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[19:20], v[25:26]
	v_fma_f64 v[25:26], v[23:24], v[21:22], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[17:18], -v[29:30]
	v_fma_f64 v[19:20], v[23:24], v[19:20], v[25:26]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[17:18], v[21:22], v[19:20]
	v_cvt_f64_i32_e32 v[21:22], v0
	v_add_f64 v[19:20], v[27:28], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[23:24], v[15:16], v[19:20]
	v_add_f64 v[25:26], v[19:20], -v[27:28]
	v_mul_f64 v[27:28], v[21:22], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[23:24], -v[15:16]
	v_add_f64 v[17:18], v[17:18], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[25:26], v[21:22], s[0:1], -v[27:28]
	s_mov_b32 s1, 0xbfe62e42
	v_add_f64 v[15:16], v[19:20], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], v[17:18]
	v_fma_f64 v[17:18], v[21:22], s[2:3], v[25:26]
	s_mov_b32 s3, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], v[15:16]
	v_add_f64 v[15:16], v[27:28], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[23:24], v[13:14]
	v_add_f64 v[27:28], v[15:16], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[15:16], v[19:20]
	v_add_f64 v[23:24], v[19:20], -v[23:24]
	v_add_f64 v[17:18], v[17:18], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[25:26], v[21:22], -v[15:16]
	v_add_f64 v[13:14], v[13:14], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[29:30], v[21:22], -v[25:26]
	v_add_f64 v[19:20], v[19:20], -v[25:26]
	v_add_f64 v[23:24], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[15:16], -v[29:30]
	v_add_f64 v[15:16], v[19:20], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[23:24], -v[17:18]
	v_add_f64 v[15:16], v[23:24], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[23:24], -v[19:20]
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	v_add_f64 v[25:26], v[21:22], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[17:18], -v[23:24]
	v_add_f64 v[19:20], v[25:26], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], v[17:18]
	v_add_f64 v[15:16], v[15:16], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[13:14], v[15:16]
	v_add_f64 v[15:16], v[25:26], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[15:16], -v[25:26]
	v_mul_f64 v[19:20], v[9:10], v[15:16]
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[9:10], v[15:16], -v[19:20]
	v_cmp_class_f64_e64 vcc_lo, v[19:20], 0x204
	v_fma_f64 v[13:14], v[9:10], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[19:20], v[13:14]
	v_dual_cndmask_b32 v18, v16, v20 :: v_dual_cndmask_b32 v17, v15, v19
	v_add_f64 v[15:16], v[15:16], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[21:22], v[17:18], s[12:13]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[17:18]|
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_trunc_f64_e32 v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rndne_f64_e32 v[21:22], v[21:22]
	v_dual_cndmask_b32 v14, 0, v14 :: v_dual_cndmask_b32 v13, 0, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[23:24], v[21:22], s[0:1], v[17:18]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_cvt_i32_f64_e32 v0, v[21:22]
	v_fma_f64 v[23:24], v[21:22], s[2:3], v[23:24]
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[23:24], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_cmp_neq_f64_e64 s3, v[9:10], |v[9:10]|
	v_cmp_eq_f64_e64 s2, 0, v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_xor_b32 s3, s3, s10
	v_cmp_class_f64_e64 s10, v[11:12], 0x204
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[17:18]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], 1.0
	s_and_b32 vcc_lo, s1, s0
	v_fma_f64 v[21:22], v[23:24], v[25:26], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[19:20], v[21:22], v0
	v_mul_f64 v[21:22], v[9:10], 0.5
	v_cndmask_b32_e64 v0, 0x7ff00000, v20, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[17:18], v[21:22]
	v_cndmask_b32_e32 v19, 0, v19, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[15:16], v[9:10]
	v_cndmask_b32_e64 v16, 0x7ff00000, 0, s3
	v_cndmask_b32_e64 v20, 0, v0, s1
	v_cmp_neq_f64_e64 s3, |v[11:12]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[13:14], v[19:20], v[13:14], v[19:20]
	v_cmp_class_f64_e64 s1, v[19:20], 0x204
	v_cmp_neq_f64_e64 s0, v[17:18], v[21:22]
	v_cndmask_b32_e64 v16, 0x3ff00000, v16, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v14, v14, v20, s1
	v_cndmask_b32_e64 v13, v13, v19, s1
	v_cmp_gt_f64_e64 s1, 0, v[9:10]
	v_cndmask_b32_e32 v15, 0, v13, vcc_lo
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v0, 0x3ff00000, v12, s0
	v_bfi_b32 v0, 0x7fffffff, v14, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v14, 0x7ff80000, v0, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[11:12]
	s_xor_b32 s1, s1, s2
	v_dual_cndmask_b32 v13, v13, v15 :: v_dual_cndmask_b32 v0, v0, v14
	v_cmp_class_f64_e64 vcc_lo, v[9:10], 0x204
	v_cndmask_b32_e64 v14, 0x7ff00000, 0, s1
	v_cndmask_b32_e64 v15, 0, v12, s0
	s_or_b32 s0, s2, s10
	s_mov_b32 s1, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_bfi_b32 v14, 0x7fffffff, v14, v15
	v_cndmask_b32_e32 v0, v0, v16, vcc_lo
	v_cndmask_b32_e64 v0, v0, v14, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[11:12], v[9:10]
	v_cndmask_b32_e64 v13, v13, 0, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v9, 0, v13, vcc_lo
	v_cndmask_b32_e32 v10, 0x7ff80000, v0, vcc_lo
	s_branch .LBB3_14
.LBB3_11:
	s_mov_b32 s1, 0
                                        ; implicit-def: $vgpr9_vgpr10
	s_cbranch_execnz .LBB3_17
	s_branch .LBB3_18
.LBB3_12:
	s_mov_b32 s1, 0
                                        ; implicit-def: $vgpr9_vgpr10
	s_cbranch_execnz .LBB3_15
	s_branch .LBB3_16
.LBB3_13:
	s_mov_b32 s1, -1
                                        ; implicit-def: $vgpr9_vgpr10
.LBB3_14:
	s_branch .LBB3_16
.LBB3_15:
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_mov_b32 s2, 0x652b82fe
	s_mov_b32 s3, 0x3ff71547
	s_waitcnt lgkmcnt(0)
	s_mov_b32 s12, 0x6a5dcb37
	s_mov_b32 s13, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[3:4], -2.0, v[5:6]
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[5:6]
	v_dual_cndmask_b32 v6, 0, v6 :: v_dual_cndmask_b32 v5, 0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[5:6], v[5:6], -s[4:5]
	v_mul_f64 v[7:8], v[5:6], s[2:3]
	s_mov_b32 s2, 0xfefa39ef
	s_mov_b32 s3, 0xbfe62e42
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[5:6]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[7:8], v[7:8]
	v_fma_f64 v[9:10], v[7:8], s[2:3], v[5:6]
	s_mov_b32 s2, 0x3b39803f
	s_mov_b32 s3, 0xbc7abc9e
	v_cvt_i32_f64_e32 v0, v[7:8]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[9:10], v[7:8], s[2:3], v[9:10]
	s_mov_b32 s2, 0xfca7ab0c
	s_mov_b32 s3, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], s[12:13], s[2:3]
	s_mov_b32 s2, 0x623fde64
	s_mov_b32 s3, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[2:3]
	s_mov_b32 s2, 0x7c89e6b0
	s_mov_b32 s3, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[2:3]
	s_mov_b32 s2, 0x14761f6e
	s_mov_b32 s3, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[2:3]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[2:3]
	s_mov_b32 s2, 0x11122322
	s_mov_b32 s3, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[2:3]
	s_mov_b32 s2, 0x555502a1
	s_mov_b32 s3, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[2:3]
	s_mov_b32 s2, 0x55555511
	s_mov_b32 s3, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[2:3]
	s_mov_b32 s2, 11
	s_mov_b32 s3, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], 1.0
	v_fma_f64 v[7:8], v[9:10], v[11:12], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[7:8], v[7:8], v0
	v_cndmask_b32_e32 v0, 0x7ff00000, v8, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v9, 0, v7, vcc_lo
	v_cndmask_b32_e64 v10, 0, v0, s0
.LBB3_16:
	s_branch .LBB3_18
.LBB3_17:
	v_dual_mov_b32 v10, v4 :: v_dual_mov_b32 v9, v3
	s_cmp_lg_u32 s11, 0
	s_cselect_b32 s1, -1, 0
.LBB3_18:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s1
	s_cbranch_vccnz .LBB3_20
; %bb.19:
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[3:4], s[4:5], v[3:4], s[8:9]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[5:6], |v[3:4]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[3:4]|
	v_rndne_f64_e32 v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], s[0:1], |v[3:4]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	v_cvt_i32_f64_e32 v0, v[5:6]
	v_mul_f64 v[9:10], v[5:6], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[7:8], 0
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	v_add_f64 v[7:8], v[7:8], 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[11:12], v[9:10]
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_mul_f64 v[9:10], v[5:6], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[13:14], v[7:8]
	v_add_f64 v[15:16], v[11:12], v[9:10]
	v_add_f64 v[13:14], v[13:14], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[15:16]
	v_add_f64 v[7:8], v[7:8], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[11:12], v[9:10]
	v_add_f64 v[7:8], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[15:16], v[7:8]
	v_fma_f64 v[11:12], v[9:10], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[13:14], v[15:16], -v[9:10]
	v_mul_f64 v[15:16], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	v_add_f64 v[7:8], v[7:8], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], v[9:10], -v[15:16]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[7:8], v[7:8]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[9:10], v[17:18], v[13:14]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[15:16], v[13:14]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[17:18], -v[15:16]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[17:18], v[11:12]
	v_fma_f64 v[15:16], v[17:18], v[11:12], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[13:14], v[11:12], v[15:16]
	v_add_f64 v[13:14], v[19:20], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[9:10], v[13:14]
	v_add_f64 v[17:18], v[13:14], -v[19:20]
	v_add_f64 v[9:10], v[15:16], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[9:10], v[13:14], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], v[11:12]
	v_add_f64 v[7:8], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[15:16], v[7:8]
	v_add_f64 v[11:12], v[9:10], 1.0
	v_add_f64 v[13:14], v[9:10], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[11:12], -1.0
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[7:8], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[11:12], v[7:8]
	v_ldexp_f64 v[9:10], v[5:6], v0
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[13:14], v[9:10]
	v_add_f64 v[5:6], v[7:8], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v0
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[15:16], -v[9:10], v[13:14], 1.0
	v_fma_f64 v[13:14], v[15:16], v[13:14], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], -v[9:10], v[13:14], 1.0
	v_fma_f64 v[11:12], v[15:16], v[13:14], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[9:10], v[11:12]
	v_fma_f64 v[13:14], v[11:12], v[9:10], -v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[5:6], v[13:14]
	v_add_f64 v[15:16], v[7:8], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], -v[15:16], 1.0
	v_add_f64 v[7:8], v[15:16], -v[7:8]
	v_add_f64 v[19:20], -v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[13:14], v[19:20], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], v[13:14]
	v_add_f64 v[13:14], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[15:16], v[11:12], v[13:14]
	v_add_f64 v[17:18], v[17:18], -v[13:14]
	v_mul_f64 v[19:20], v[9:10], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[17:18]
	v_fma_f64 v[21:22], v[15:16], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[21:22], v[15:16], v[5:6], v[21:22]
	v_add_f64 v[23:24], v[19:20], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[13:14], -v[23:24]
	v_add_f64 v[17:18], v[23:24], -v[19:20]
	v_add_f64 v[13:14], v[13:14], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[17:18], -v[21:22]
	v_add_f64 v[13:14], v[13:14], -v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[13:14]
	v_add_f64 v[13:14], v[11:12], v[15:16]
	v_add_f64 v[7:8], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[13:14], -v[11:12]
	v_add_f64 v[7:8], v[25:26], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	v_mul_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[15:16], v[7:8]
	v_add_f64 v[11:12], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[9:10], v[11:12]
	v_add_f64 v[13:14], v[11:12], -v[13:14]
	v_add_f64 v[19:20], v[9:10], -v[11:12]
	v_add_f64 v[17:18], v[15:16], -v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[11:12], -v[17:18]
	v_add_f64 v[17:18], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	v_add_f64 v[13:14], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_add_f64 v[17:18], v[15:16], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_rcp_f64_e32 v[21:22], v[17:18]
	v_add_f64 v[15:16], v[17:18], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[17:18], v[21:22], 1.0
	v_fma_f64 v[9:10], v[11:12], v[21:22], v[21:22]
	v_add_f64 v[11:12], v[19:20], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[17:18], v[9:10], 1.0
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	v_mul_f64 v[21:22], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[9:10], v[17:18], -v[21:22]
	v_fma_f64 v[13:14], v[9:10], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[21:22], v[13:14]
	v_add_f64 v[17:18], v[11:12], -v[15:16]
	v_add_f64 v[21:22], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[11:12], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	v_add_f64 v[13:14], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[23:24], -v[15:16]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[15:16], -v[13:14]
	v_add_f64 v[5:6], v[5:6], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[17:18], v[5:6]
	v_mul_f64 v[5:6], v[7:8], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[5:6]
	v_cndmask_b32_e32 v0, 0x3ff00000, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_and_b32 v6, 0x7fffffff, v4
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[3:4]|
	v_dual_cndmask_b32 v9, v5, v3 :: v_dual_cndmask_b32 v0, v0, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_bfi_b32 v10, 0x7fffffff, v0, v4
.LBB3_20:
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[9:10], off
.LBB3_21:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_
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
		.amdhsa_next_free_vgpr 33
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
		.amdhsa_inst_pref_size 38
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
	.size	_Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_, .Lfunc_end3-_Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_
                                        ; -- End function
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.num_vgpr, 33
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.num_agpr, 0
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.numbered_sgpr, 16
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.num_named_barrier, 0
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.private_seg_size, 0
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.uses_vcc, 1
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.uses_flat_scratch, 0
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.has_dyn_sized_stack, 0
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.has_recursion, 0
	.set _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4768
; TotalNumSgprs: 18
; NumVgprs: 33
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 18
; NumVGPRsForWavesPerEU: 33
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
	.protected	_Z17smo_argmax_kernelPKdPdi ; -- Begin function _Z17smo_argmax_kernelPKdPdi
	.globl	_Z17smo_argmax_kernelPKdPdi
	.p2align	8
	.type	_Z17smo_argmax_kernelPKdPdi,@function
_Z17smo_argmax_kernelPKdPdi:            ; @_Z17smo_argmax_kernelPKdPdi
; %bb.0:
	s_load_b32 s8, s[0:1], 0x10
	s_add_u32 s2, s0, 24
	s_addc_u32 s3, s1, 0
	s_mov_b32 s4, exec_lo
                                        ; implicit-def: $sgpr10
	s_waitcnt lgkmcnt(0)
	v_cmpx_le_i32_e64 s8, v0
	s_xor_b32 s4, exec_lo, s4
; %bb.1:
	s_load_b32 s10, s[2:3], 0xc
; %bb.2:
	s_or_saveexec_b32 s9, s4
	s_load_b128 s[4:7], s[0:1], 0x0
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v3, 0xffe1ccf3
	v_mov_b32_e32 v2, 0x85ebc8a0
	s_waitcnt lgkmcnt(0)
	v_mov_b16_e32 v1.l, s10
	s_xor_b32 exec_lo, exec_lo, s9
	s_cbranch_execz .LBB4_6
; %bb.3:
	s_load_b32 s1, s[2:3], 0xc
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v4, v0
	s_mov_b32 s2, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s1, 0xffff
	.p2align	6
.LBB4_4:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v4
	v_lshlrev_b64 v[7:8], 3, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s4, v7
	v_add_co_ci_u32_e64 v8, null, s5, v8, vcc_lo
	global_load_b64 v[7:8], v[7:8], off
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[7:8], v[2:3]
	v_cndmask_b32_e32 v6, v6, v4, vcc_lo
	v_add_nc_u32_e32 v4, s3, v4
	v_dual_cndmask_b32 v3, v3, v8 :: v_dual_cndmask_b32 v2, v2, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e64 s0, s8, v4
	s_or_b32 s2, s0, s2
	s_and_not1_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB4_4
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s2
	v_mov_b16_e32 v1.l, s1
.LBB4_6:
	s_or_b32 exec_lo, exec_lo, s9
	v_lshlrev_b32_e32 v4, 3, v0
	v_lshlrev_b32_e32 v5, 2, v0
	s_mov_b32 s0, exec_lo
	ds_store_b64 v4, v[2:3]
	ds_store_b32 v5, v6 offset:2048
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_lt_u16_e32 1, v1.l
	s_cbranch_execz .LBB4_12
; %bb.7:
	v_add_nc_u32_e32 v3, 0x800, v5
	v_lshrrev_b16 v5.l, 1, v1.l
	v_mov_b16_e32 v5.h, 0
	s_mov_b32 s1, 0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB4_9
	.p2align	6
.LBB4_8:                                ;   in Loop: Header=BB4_9 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	v_lshrrev_b32_e32 v1, 1, v5
	v_cmp_gt_u32_e32 vcc_lo, 2, v5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_mov_b32_e32 v5, v1
	s_or_b32 s1, vcc_lo, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB4_12
.LBB4_9:                                ; =>This Inner Loop Header: Depth=1
	s_mov_b32 s2, exec_lo
	v_cmpx_lt_u32_e64 v0, v5
	s_cbranch_execz .LBB4_8
; %bb.10:                               ;   in Loop: Header=BB4_9 Depth=1
	v_lshl_add_u32 v1, v5, 3, v4
	ds_load_b64 v[1:2], v1
	ds_load_b64 v[6:7], v4
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[1:2], v[6:7]
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB4_8
; %bb.11:                               ;   in Loop: Header=BB4_9 Depth=1
	v_lshl_add_u32 v6, v5, 2, v3
	ds_load_b32 v6, v6
	ds_store_b64 v4, v[1:2]
	s_waitcnt lgkmcnt(1)
	ds_store_b32 v3, v6
	s_branch .LBB4_8
.LBB4_12:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB4_14
; %bb.13:
	v_mov_b32_e32 v4, 0
	ds_load_b32 v0, v4 offset:2048
	s_waitcnt lgkmcnt(0)
	v_cvt_f64_i32_e32 v[2:3], v0
	ds_load_b64 v[0:1], v4
	s_waitcnt lgkmcnt(0)
	global_store_b128 v4, v[0:3], s[6:7]
.LBB4_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z17smo_argmax_kernelPKdPdi
		.amdhsa_group_segment_fixed_size 3072
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
		.amdhsa_next_free_vgpr 9
		.amdhsa_next_free_sgpr 11
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
	.size	_Z17smo_argmax_kernelPKdPdi, .Lfunc_end4-_Z17smo_argmax_kernelPKdPdi
                                        ; -- End function
	.set _Z17smo_argmax_kernelPKdPdi.num_vgpr, 9
	.set _Z17smo_argmax_kernelPKdPdi.num_agpr, 0
	.set _Z17smo_argmax_kernelPKdPdi.numbered_sgpr, 11
	.set _Z17smo_argmax_kernelPKdPdi.num_named_barrier, 0
	.set _Z17smo_argmax_kernelPKdPdi.private_seg_size, 0
	.set _Z17smo_argmax_kernelPKdPdi.uses_vcc, 1
	.set _Z17smo_argmax_kernelPKdPdi.uses_flat_scratch, 0
	.set _Z17smo_argmax_kernelPKdPdi.has_dyn_sized_stack, 0
	.set _Z17smo_argmax_kernelPKdPdi.has_recursion, 0
	.set _Z17smo_argmax_kernelPKdPdi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 524
; TotalNumSgprs: 13
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 3072 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 13
; NumVGPRsForWavesPerEU: 9
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
	.protected	_Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_ ; -- Begin function _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_
	.globl	_Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_
	.p2align	8
	.type	_Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_,@function
_Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_: ; @_Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB5_2
; %bb.1:
	s_clause 0x1
	s_load_b64 s[2:3], s[0:1], 0x10
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_u32 v0, vcc_lo, s4, v0
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	s_load_b128 s[0:3], s[0:1], 0x20
	global_load_b64 v[6:7], v[0:1], off
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[2:3], 0x0
	s_load_b64 s[0:1], s[0:1], 0x0
	s_waitcnt vmcnt(2) lgkmcnt(0)
	v_mul_f64 v[2:3], s[2:3], v[2:3]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], s[0:1], v[4:5], v[2:3]
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[6:7], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_
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
.Lfunc_end5:
	.size	_Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_, .Lfunc_end5-_Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_
                                        ; -- End function
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.num_vgpr, 8
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.num_agpr, 0
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.numbered_sgpr, 8
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.num_named_barrier, 0
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.private_seg_size, 0
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.uses_vcc, 1
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.uses_flat_scratch, 0
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.has_dyn_sized_stack, 0
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.has_recursion, 0
	.set _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 256
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
	.type	__hip_cuid_2f0281dc8ae90cfe,@object ; @__hip_cuid_2f0281dc8ae90cfe
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_2f0281dc8ae90cfe
__hip_cuid_2f0281dc8ae90cfe:
	.byte	0                               ; 0x0
	.size	__hip_cuid_2f0281dc8ae90cfe, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_2f0281dc8ae90cfe
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
      - .offset:         20
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     by_value
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
    .group_segment_fixed_size: 4096
    .kernarg_segment_align: 8
    .kernarg_segment_size: 56
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_
    .private_segment_fixed_size: 0
    .sgpr_count:     24
    .sgpr_spill_count: 0
    .symbol:         _Z20kernel_matrix_kernelPKdPdiiiS0_S0_S0_.kd
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
      - .offset:         20
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .offset:         32
        .size:           8
        .value_kind:     by_value
      - .offset:         40
        .size:           8
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
    .name:           _Z26smo_update_gradient_kernelPdPKdiiidd
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z26smo_update_gradient_kernelPdPKdiiidd.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     9
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
      - .address_space:  global
        .offset:         48
        .size:           8
        .value_kind:     global_buffer
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
    .name:           _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z20smo_kkt_score_kernelPKdS0_S0_PdS1_iS0_.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         20
        .size:           4
        .value_kind:     by_value
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
    .name:           _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z21smo_kernel_row_kernelPKdPdiiiiS0_S0_S0_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     33
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
    .group_segment_fixed_size: 3072
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z17smo_argmax_kernelPKdPdi
    .private_segment_fixed_size: 0
    .sgpr_count:     13
    .sgpr_spill_count: 0
    .symbol:         _Z17smo_argmax_kernelPKdPdi.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     9
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
    .name:           _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         _Z31smo_update_gradient_rows_kernelPdPKdS1_iS1_S1_.kd
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
