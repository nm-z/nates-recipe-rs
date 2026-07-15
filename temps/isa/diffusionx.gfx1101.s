	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	diffusionx_entropy_gated_step_kernel ; -- Begin function diffusionx_entropy_gated_step_kernel
	.globl	diffusionx_entropy_gated_step_kernel
	.p2align	8
	.type	diffusionx_entropy_gated_step_kernel,@function
diffusionx_entropy_gated_step_kernel:   ; @diffusionx_entropy_gated_step_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[12:13], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s12, v1
	s_cbranch_execz .LBB0_16
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_mad_i64_i32 v[2:3], null, v1, s13, 0
	s_load_b64 s[2:3], s[0:1], 0x20
	v_mov_b32_e32 v7, 0
	v_mov_b32_e32 v8, 0
	s_cmp_lt_i32 s13, 2
	v_lshlrev_b64 v[9:10], 3, v[2:3]
	v_ashrrev_i32_e32 v2, 31, v1
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s4, v9
	v_add_co_ci_u32_e64 v4, null, s5, v10, vcc_lo
	global_load_b64 v[5:6], v[3:4], off
	s_cbranch_scc1 .LBB0_5
; %bb.2:
	v_add_co_u32 v0, vcc_lo, s4, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s5, v10, vcc_lo
	s_mov_b32 s1, 1
	v_add_co_u32 v7, vcc_lo, v0, 8
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_mov_b32_e32 v0, 0
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[9:10], v[7:8], off
	v_add_co_u32 v7, s0, v7, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, 0, v8, s0
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[9:10], v[5:6]
	v_cndmask_b32_e64 v0, v0, s1, vcc_lo
	v_dual_cndmask_b32 v6, v6, v10 :: v_dual_cndmask_b32 v5, v5, v9
	s_add_i32 s1, s1, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s13, s1
	s_cbranch_scc0 .LBB0_3
; %bb.4:
	v_cvt_f64_u32_e32 v[7:8], v0
.LBB0_5:
	s_load_b64 s[2:3], s[2:3], 0x0
	s_cmp_lt_i32 s13, 1
	s_cbranch_scc1 .LBB0_12
; %bb.6:
	v_dual_mov_b32 v9, 0 :: v_dual_mov_b32 v12, v4
	v_dual_mov_b32 v10, 0 :: v_dual_mov_b32 v11, v3
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s14, 0xfefa39ef
	s_mov_b32 s16, 0x3b39803f
	s_mov_b32 s18, 0xfca7ab0c
	s_mov_b32 s20, 0x6a5dcb37
	s_mov_b32 s22, 0x623fde64
	s_mov_b32 s24, 0x7c89e6b0
	s_mov_b32 s26, 0x14761f6e
	s_mov_b32 s28, 0x1852b7b0
	s_mov_b32 s30, 0x11122322
	s_mov_b32 s34, 0x555502a1
	s_mov_b32 s36, 0x55555511
	s_mov_b32 s38, 11
	s_mov_b32 s5, 0x3ff71547
	s_mov_b32 s15, 0xbfe62e42
	s_mov_b32 s17, 0xbc7abc9e
	s_mov_b32 s19, 0x3e928af3
	s_mov_b32 s21, 0x3e5ade15
	s_mov_b32 s23, 0x3ec71dee
	s_mov_b32 s25, 0x3efa0199
	s_mov_b32 s27, 0x3f2a01a0
	s_mov_b32 s29, 0x3f56c16c
	s_mov_b32 s31, 0x3f811111
	s_mov_b32 s35, 0x3fa55555
	s_mov_b32 s37, 0x3fc55555
	s_mov_b32 s39, 0x3fe00000
	s_mov_b32 s1, s13
.LBB0_7:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[13:14], v[11:12], off
	s_add_i32 s1, s1, -1
	s_waitcnt vmcnt(0)
	v_add_f64 v[13:14], v[13:14], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[15:16], v[13:14], s[4:5]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[13:14]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[13:14]
	v_rndne_f64_e32 v[15:16], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], s[14:15], v[13:14]
	v_cvt_i32_f64_e32 v0, v[15:16]
	v_fma_f64 v[17:18], v[15:16], s[16:17], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], s[20:21], s[18:19]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[24:25]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[28:29]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[34:35]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[38:39]
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	v_ldexp_f64 v[15:16], v[15:16], v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, 0x7ff00000, v16, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_cmp_lg_u32 s1, 0
	v_cndmask_b32_e32 v13, 0, v15, vcc_lo
	v_add_co_u32 v11, vcc_lo, v11, 8
	v_cndmask_b32_e64 v14, 0, v0, s0
	v_add_co_ci_u32_e64 v12, null, 0, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], v[13:14]
	s_cbranch_scc1 .LBB0_7
; %bb.8:
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v12, 0
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s14, 0xfefa39ef
	s_mov_b32 s16, 0xfefa39ef
	s_mov_b32 s18, 0x3b39803f
	s_mov_b32 s20, 0x3b39803f
	s_mov_b32 s22, 0xfca7ab0c
	s_mov_b32 s24, 0x6a5dcb37
	s_mov_b32 s26, 0x623fde64
	s_mov_b32 s28, 0x7c89e6b0
	s_mov_b32 s30, 0x14761f6e
	s_mov_b32 s34, 0x1852b7b0
	s_mov_b32 s36, 0x11122322
	s_mov_b32 s38, 0x555502a1
	s_mov_b32 s40, 0x55555511
	s_mov_b32 s42, 11
	s_mov_b32 s44, 0x55555555
	s_mov_b32 s46, 0x6b47b09a
	s_mov_b32 s48, 0xbf559e2b
	s_mov_b32 s50, 0xd7f4df2e
	s_mov_b32 s52, 0x16291751
	s_mov_b32 s54, 0x9b27acf1
	s_mov_b32 s56, 0x998ef7b6
	s_mov_b32 s5, 0x3ff71547
	s_mov_b32 s15, 0xbfe62e42
	s_mov_b32 s17, 0x3fe62e42
	s_mov_b32 s19, 0xbc7abc9e
	s_mov_b32 s21, 0x3c7abc9e
	s_mov_b32 s23, 0x3e928af3
	s_mov_b32 s25, 0x3e5ade15
	s_mov_b32 s27, 0x3ec71dee
	s_mov_b32 s29, 0x3efa0199
	s_mov_b32 s31, 0x3f2a01a0
	s_mov_b32 s35, 0x3f56c16c
	s_mov_b32 s37, 0x3f811111
	s_mov_b32 s39, 0x3fa55555
	s_mov_b32 s41, 0x3fc55555
	s_mov_b32 s43, 0x3fe00000
	s_mov_b32 s45, 0x3fe55555
	s_mov_b32 s47, 0x3fc38538
	s_mov_b32 s49, 0x3fc3ab76
	s_mov_b32 s51, 0x3fc7474d
	s_mov_b32 s53, 0x3fcc71c0
	s_mov_b32 s55, 0x3fd24924
	s_mov_b32 s57, 0x3fd99999
	s_mov_b32 s58, 0x55555780
	s_branch .LBB0_10
.LBB0_9:                                ;   in Loop: Header=BB0_10 Depth=1
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v3, vcc_lo, v3, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	s_add_i32 s13, s13, -1
	s_cmp_eq_u32 s13, 0
	s_cbranch_scc1 .LBB0_13
.LBB0_10:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[13:14], v[3:4], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[13:14], v[13:14], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[15:16], v[13:14], s[4:5]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[13:14]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[13:14]
	v_rndne_f64_e32 v[15:16], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], s[14:15], v[13:14]
	v_cvt_i32_f64_e32 v0, v[15:16]
	v_fma_f64 v[17:18], v[15:16], s[18:19], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], s[24:25], s[22:23]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[28:29]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[34:35]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[38:39]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[42:43]
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	v_ldexp_f64 v[15:16], v[15:16], v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, 0x7ff00000, v16, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v13, 0, v15, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v14, 0, v0, s0
	s_mov_b32 s0, exec_lo
	v_div_scale_f64 v[15:16], null, v[9:10], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[17:18], v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[19:20], -v[15:16], v[17:18], 1.0
	v_fma_f64 v[17:18], v[17:18], v[19:20], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], -v[15:16], v[17:18], 1.0
	v_fma_f64 v[17:18], v[17:18], v[19:20], v[17:18]
	v_div_scale_f64 v[19:20], vcc_lo, v[13:14], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[21:22], v[19:20], v[17:18]
	v_fma_f64 v[15:16], -v[15:16], v[21:22], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[15:16], v[15:16], v[17:18], v[21:22]
	v_div_fixup_f64 v[13:14], v[15:16], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_lt_f64_e32 0, v[13:14]
	s_cbranch_execz .LBB0_9
; %bb.11:                               ;   in Loop: Header=BB0_10 Depth=1
	v_frexp_mant_f64_e32 v[15:16], v[13:14]
	s_mov_b32 s59, s45
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[44:45], v[15:16]
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_ldexp_f64 v[15:16], v[15:16], v0
	v_frexp_exp_i32_f64_e32 v0, v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[15:16], 1.0
	v_add_f64 v[23:24], v[15:16], -1.0
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[13:14]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[19:20], v[17:18]
	v_add_f64 v[25:26], v[17:18], -1.0
	v_add_f64 v[15:16], v[15:16], -v[25:26]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[21:22], -v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[21:22], v[19:20], v[19:20]
	v_fma_f64 v[21:22], -v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[21:22], v[19:20], v[19:20]
	v_mul_f64 v[21:22], v[23:24], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[27:28], v[17:18], v[21:22]
	v_fma_f64 v[17:18], v[21:22], v[17:18], -v[27:28]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[21:22], v[15:16], v[17:18]
	v_add_f64 v[17:18], v[27:28], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[23:24], -v[17:18]
	v_add_f64 v[27:28], v[17:18], -v[27:28]
	v_add_f64 v[23:24], v[23:24], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[27:28], -v[15:16]
	v_add_f64 v[17:18], v[23:24], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[15:16], v[17:18]
	v_add_f64 v[15:16], v[25:26], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[19:20], v[15:16]
	v_add_f64 v[17:18], v[21:22], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[17:18], v[17:18]
	v_fma_f64 v[23:24], v[19:20], s[48:49], s[46:47]
	v_mul_f64 v[25:26], v[17:18], v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[19:20], v[23:24], s[50:51]
	v_fma_f64 v[23:24], v[19:20], v[23:24], s[52:53]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[19:20], v[23:24], s[54:55]
	v_fma_f64 v[23:24], v[19:20], v[23:24], s[56:57]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[19:20], v[19:20], v[23:24], s[58:59]
	v_ldexp_f64 v[23:24], v[17:18], 1
	v_add_f64 v[17:18], v[17:18], -v[21:22]
	v_mul_f64 v[19:20], v[25:26], v[19:20]
	v_cvt_f64_i32_e32 v[25:26], v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	v_add_f64 v[21:22], v[23:24], v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[27:28], v[25:26], s[16:17]
	v_ldexp_f64 v[15:16], v[15:16], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[21:22], -v[23:24]
	v_fma_f64 v[23:24], v[25:26], s[16:17], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[17:18]
	v_fma_f64 v[19:20], v[25:26], s[20:21], v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[15:16], v[17:18]
	v_add_f64 v[17:18], v[27:28], v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[23:24], v[21:22], v[15:16]
	v_add_f64 v[27:28], v[17:18], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[25:26], v[17:18], v[23:24]
	v_add_f64 v[21:22], v[23:24], -v[21:22]
	v_add_f64 v[19:20], v[19:20], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[29:30], v[25:26], -v[17:18]
	v_add_f64 v[15:16], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[31:32], v[25:26], -v[29:30]
	v_add_f64 v[21:22], v[23:24], -v[29:30]
	v_add_f64 v[23:24], v[19:20], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[17:18], -v[31:32]
	v_add_f64 v[17:18], v[21:22], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[23:24], -v[19:20]
	v_add_f64 v[17:18], v[23:24], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[23:24], -v[21:22]
	v_add_f64 v[15:16], v[15:16], -v[21:22]
	v_add_f64 v[27:28], v[25:26], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[19:20], -v[23:24]
	v_add_f64 v[21:22], v[27:28], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[15:16], v[19:20]
	v_add_f64 v[17:18], v[17:18], -v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[15:16], v[17:18]
	v_add_f64 v[15:16], v[27:28], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v16, 0x7ff00000, v16, vcc_lo
	v_cndmask_b32_e32 v15, 0, v15, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[13:14], v[15:16], v[11:12]
	s_branch .LBB0_9
.LBB0_12:
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v12, 0
.LBB0_13:
	v_mov_b32_e32 v3, 0
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_mov_b32_e32 v4, 0
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_ngt_f64_e32 s[2:3], v[11:12]
	s_cbranch_execz .LBB0_15
; %bb.14:
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	global_load_b64 v[7:8], v[2:3], off
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0x3ff00000
.LBB0_15:
	s_or_b32 exec_lo, exec_lo, s0
	s_waitcnt vmcnt(0)
	v_add_co_u32 v5, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s9, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s10, v0
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	global_store_b64 v[5:6], v[7:8], off
	global_store_b64 v[0:1], v[3:4], off
.LBB0_16:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel diffusionx_entropy_gated_step_kernel
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
		.amdhsa_next_free_vgpr 33
		.amdhsa_next_free_sgpr 60
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
		.amdhsa_inst_pref_size 19
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
	.size	diffusionx_entropy_gated_step_kernel, .Lfunc_end0-diffusionx_entropy_gated_step_kernel
                                        ; -- End function
	.set diffusionx_entropy_gated_step_kernel.num_vgpr, 33
	.set diffusionx_entropy_gated_step_kernel.num_agpr, 0
	.set diffusionx_entropy_gated_step_kernel.numbered_sgpr, 60
	.set diffusionx_entropy_gated_step_kernel.num_named_barrier, 0
	.set diffusionx_entropy_gated_step_kernel.private_seg_size, 0
	.set diffusionx_entropy_gated_step_kernel.uses_vcc, 1
	.set diffusionx_entropy_gated_step_kernel.uses_flat_scratch, 0
	.set diffusionx_entropy_gated_step_kernel.has_dyn_sized_stack, 0
	.set diffusionx_entropy_gated_step_kernel.has_recursion, 0
	.set diffusionx_entropy_gated_step_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2408
; TotalNumSgprs: 62
; NumVgprs: 33
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 62
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
	.protected	diffusionx_commit_kernel ; -- Begin function diffusionx_commit_kernel
	.globl	diffusionx_commit_kernel
	.p2align	8
	.type	diffusionx_commit_kernel,@function
diffusionx_commit_kernel:               ; @diffusionx_commit_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_4
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s6, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v3, vcc_lo
	global_load_b64 v[4:5], v[0:1], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_f64_e32 vcc_lo, 0, v[4:5]
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB1_4
; %bb.2:
	v_add_co_u32 v4, vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s3, v3, vcc_lo
	v_add_co_u32 v6, vcc_lo, s0, v2
	v_add_co_ci_u32_e64 v7, null, s1, v3, vcc_lo
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_u32 v2, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[6:7], v[4:5], off
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_f64_e32 vcc_lo, 0, v[2:3]
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB1_4
; %bb.3:
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, 0x3ff00000
	global_store_b64 v[0:1], v[2:3], off
.LBB1_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel diffusionx_commit_kernel
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
.Lfunc_end1:
	.size	diffusionx_commit_kernel, .Lfunc_end1-diffusionx_commit_kernel
                                        ; -- End function
	.set diffusionx_commit_kernel.num_vgpr, 8
	.set diffusionx_commit_kernel.num_agpr, 0
	.set diffusionx_commit_kernel.numbered_sgpr, 8
	.set diffusionx_commit_kernel.num_named_barrier, 0
	.set diffusionx_commit_kernel.private_seg_size, 0
	.set diffusionx_commit_kernel.uses_vcc, 1
	.set diffusionx_commit_kernel.uses_flat_scratch, 0
	.set diffusionx_commit_kernel.has_dyn_sized_stack, 0
	.set diffusionx_commit_kernel.has_recursion, 0
	.set diffusionx_commit_kernel.has_indirect_call, 0
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
	.type	__hip_cuid_bf122bead7c1e904,@object ; @__hip_cuid_bf122bead7c1e904
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_bf122bead7c1e904
__hip_cuid_bf122bead7c1e904:
	.byte	0                               ; 0x0
	.size	__hip_cuid_bf122bead7c1e904, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_bf122bead7c1e904
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
      - .offset:         44
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
    .name:           diffusionx_entropy_gated_step_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     62
    .sgpr_spill_count: 0
    .symbol:         diffusionx_entropy_gated_step_kernel.kd
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
    .name:           diffusionx_commit_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         diffusionx_commit_kernel.kd
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
