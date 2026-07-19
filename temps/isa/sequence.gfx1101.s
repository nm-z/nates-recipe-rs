	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z19forward_step_kernelPKdS0_Pdii ; -- Begin function _Z19forward_step_kernelPKdS0_Pdii
	.globl	_Z19forward_step_kernelPKdS0_Pdii
	.p2align	8
	.type	_Z19forward_step_kernelPKdS0_Pdii,@function
_Z19forward_step_kernelPKdS0_Pdii:      ; @_Z19forward_step_kernelPKdS0_Pdii
; %bb.0:
	s_load_b64 s[2:3], s[0:1], 0x18
	s_mov_b32 s4, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_gt_i32_e64 s2, v0
	s_cbranch_execz .LBB0_9
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x8
	v_mov_b32_e32 v1, 0
	v_lshlrev_b32_e32 v7, 3, v0
	s_cmp_eq_u32 s3, 0
	s_mov_b32 s9, 0
	s_cbranch_scc1 .LBB0_10
; %bb.2:
	s_load_b64 s[0:1], s[0:1], 0x0
	s_add_i32 s8, s3, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_i32 s10, s8, s2
	s_mov_b32 s8, s2
	s_ashr_i32 s11, s10, 31
	s_lshl_b64 s[8:9], s[8:9], 3
	s_lshl_b64 s[10:11], s[10:11], 3
	s_waitcnt lgkmcnt(0)
	s_add_u32 s10, s6, s10
	s_addc_u32 s11, s7, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_mov_b64 s[12:13], s[10:11]
	v_add_co_u32 v5, s0, s0, v7
	v_add_co_ci_u32_e64 v6, null, s1, 0, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_mov_b32 v2, 0xfe37e43c :: v_dual_mov_b32 v3, v5
	v_dual_mov_b32 v1, 0x8800759c :: v_dual_mov_b32 v4, v6
	s_mov_b32 s1, s2
	.p2align	6
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[3:4], off
	s_load_b64 s[14:15], s[12:13], 0x0
	v_add_co_u32 v3, s0, v3, s8
	s_add_i32 s1, s1, -1
	v_add_co_ci_u32_e64 v4, null, s9, v4, s0
	s_add_u32 s12, s12, 8
	s_addc_u32 s13, s13, 0
	s_cmp_eq_u32 s1, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[8:9], s[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, v[8:9], v[1:2]
	v_dual_cndmask_b32 v2, v2, v9 :: v_dual_cndmask_b32 v1, v1, v8
	s_cbranch_scc0 .LBB0_3
; %bb.4:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_mov_b32 s12, 0x652b82fe
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
	s_mov_b32 s13, 0x3ff71547
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
	s_mov_b32 s1, s2
.LBB0_5:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[5:6], off
	s_load_b64 s[40:41], s[10:11], 0x0
	s_add_i32 s1, s1, -1
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[8:9], s[40:41], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[1:2]
	v_mul_f64 v[10:11], v[8:9], s[12:13]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[8:9]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], s[14:15], v[8:9]
	v_cvt_i32_f64_e32 v16, v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], s[16:17], v[12:13]
	v_fma_f64 v[14:15], v[12:13], s[20:21], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[22:23]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[26:27]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[30:31]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[36:37]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], 1.0
	v_fma_f64 v[10:11], v[12:13], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[10:11], v[10:11], v16
	v_cndmask_b32_e32 v11, 0x7ff00000, v11, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_add_u32 s10, s10, 8
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v8, 0, v10, vcc_lo
	v_add_co_u32 v5, vcc_lo, v5, s8
	v_cndmask_b32_e64 v9, 0, v11, s0
	v_add_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_addc_u32 s11, s11, 0
	s_cmp_eq_u32 s1, 0
	v_add_f64 v[3:4], v[3:4], v[8:9]
	s_cbranch_scc0 .LBB0_5
; %bb.6:
	s_delay_alu instid0(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[5:6], v[3:4]
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[5:6]
	s_mov_b32 s0, 0x55555780
	v_cndmask_b32_e64 v8, 0, 1, vcc_lo
	v_ldexp_f64 v[5:6], v[5:6], v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[5:6], 1.0
	v_add_f64 v[14:15], v[5:6], -1.0
	v_rcp_f64_e32 v[10:11], v[8:9]
	v_add_f64 v[16:17], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[16:17]
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
	v_fma_f64 v[5:6], v[12:13], v[5:6], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[18:19], v[5:6]
	v_add_f64 v[16:17], v[14:15], -v[8:9]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_add_f64 v[5:6], v[18:19], -v[5:6]
	v_frexp_exp_i32_f64_e32 v18, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	v_add_f64 v[5:6], v[5:6], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[16:17], v[5:6]
	v_mul_f64 v[5:6], v[10:11], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[5:6]
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
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_subrev_co_ci_u32_e64 v16, null, 0, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[8:9]
	v_cvt_f64_i32_e32 v[16:17], v16
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[14:15], v[10:11]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[16:17], s[0:1]
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[16:17], s[0:1], -v[18:19]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[16:17], s[0:1], v[14:15]
	v_add_f64 v[5:6], v[5:6], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[18:19], v[10:11]
	v_add_f64 v[14:15], v[12:13], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	v_add_f64 v[16:17], v[8:9], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	v_add_f64 v[20:21], v[16:17], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[12:13]
	v_add_f64 v[22:23], v[16:17], -v[20:21]
	v_add_f64 v[12:13], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[20:21], v[10:11], v[5:6]
	v_mad_u64_u32 v[14:15], null, s3, s2, v[0:1]
	v_ashrrev_i32_e32 v15, 31, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[18:19], 3, v[14:15]
	v_add_co_u32 v18, vcc_lo, s4, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v19, null, s5, v19, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[3:4], 0x204
	global_load_b64 v[18:19], v[18:19], off
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[8:9], v[12:13], v[8:9]
	v_add_f64 v[12:13], v[20:21], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[20:21], v[8:9]
	v_add_f64 v[20:21], v[20:21], -v[12:13]
	v_add_f64 v[5:6], v[5:6], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[22:23], -v[16:17]
	v_add_f64 v[5:6], v[5:6], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[5:6], v[5:6], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[22:23], v[5:6]
	v_dual_cndmask_b32 v0, v5, v3 :: v_dual_cndmask_b32 v5, v6, v4
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v6, 0x7ff80000, v5, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[3:4]
	v_cndmask_b32_e32 v5, 0, v0, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[3:4]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, 0xfff00000, v6, vcc_lo
	v_add_f64 v[0:1], v[1:2], v[5:6]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[0:1], v[18:19]
	v_dual_mov_b32 v0, v14 :: v_dual_mov_b32 v1, v15
	s_branch .LBB0_8
.LBB0_7:
	v_cvt_f64_u32_e32 v[2:3], s2
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s2, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s3, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[4:5], null, v[2:3], v[2:3], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[2:3], 1.0
	v_rcp_f64_e32 v[8:9], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[4:5], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[4:5], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[10:11], v[12:13]
	v_div_fmas_f64 v[4:5], v[4:5], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[4:5], v[2:3], 1.0
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[4:5]
	s_mov_b32 s0, 0x55555780
	v_cndmask_b32_e64 v6, 0, 1, vcc_lo
	v_ldexp_f64 v[4:5], v[4:5], v6
	v_frexp_exp_i32_f64_e32 v6, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[4:5], 1.0
	v_add_f64 v[14:15], v[4:5], -1.0
	v_subrev_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	v_add_f64 v[16:17], v[8:9], -1.0
	v_add_f64 v[4:5], v[4:5], -v[16:17]
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
	v_fma_f64 v[4:5], v[12:13], v[4:5], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[8:9]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[18:19], -v[4:5]
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[4:5], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[10:11], v[4:5]
	v_add_f64 v[8:9], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	v_fma_f64 v[14:15], v[10:11], s[8:9], s[2:3]
	s_mov_b32 s2, 0xd7f4df2e
	s_mov_b32 s3, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[2:3]
	s_mov_b32 s2, 0x16291751
	s_mov_b32 s3, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[2:3]
	s_mov_b32 s2, 0x9b27acf1
	s_mov_b32 s3, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[2:3]
	s_mov_b32 s2, 0x998ef7b6
	s_mov_b32 s3, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[0:1]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v6
	s_delay_alu instid0(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[8:9]
	s_waitcnt lgkmcnt(0)
	global_load_b64 v[6:7], v7, s[4:5]
	v_add_f64 v[12:13], v[14:15], v[10:11]
	v_mul_f64 v[18:19], v[16:17], s[0:1]
	v_ldexp_f64 v[4:5], v[4:5], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[0:1], -v[18:19]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[0:1], v[14:15]
	v_cmp_neq_f64_e64 s0, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[4:5]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[8:9], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[16:17], -v[8:9]
	v_add_f64 v[4:5], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], -v[20:21]
	v_add_f64 v[12:13], v[14:15], -v[20:21]
	v_add_f64 v[14:15], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[14:15], -v[10:11]
	v_add_f64 v[8:9], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[4:5], v[4:5], -v[12:13]
	v_add_f64 v[18:19], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[12:13], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[4:5], v[18:19], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v3, 0xfff00000, v5, s0
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[6:7]
.LBB0_8:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB0_9:
	s_endpgm
.LBB0_10:
                                        ; implicit-def: $vgpr2_vgpr3
	s_cbranch_execnz .LBB0_7
	s_branch .LBB0_8
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z19forward_step_kernelPKdS0_Pdii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 32
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
		.amdhsa_next_free_sgpr 42
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
		.amdhsa_inst_pref_size 23
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
	.size	_Z19forward_step_kernelPKdS0_Pdii, .Lfunc_end0-_Z19forward_step_kernelPKdS0_Pdii
                                        ; -- End function
	.set _Z19forward_step_kernelPKdS0_Pdii.num_vgpr, 24
	.set _Z19forward_step_kernelPKdS0_Pdii.num_agpr, 0
	.set _Z19forward_step_kernelPKdS0_Pdii.numbered_sgpr, 42
	.set _Z19forward_step_kernelPKdS0_Pdii.num_named_barrier, 0
	.set _Z19forward_step_kernelPKdS0_Pdii.private_seg_size, 0
	.set _Z19forward_step_kernelPKdS0_Pdii.uses_vcc, 1
	.set _Z19forward_step_kernelPKdS0_Pdii.uses_flat_scratch, 0
	.set _Z19forward_step_kernelPKdS0_Pdii.has_dyn_sized_stack, 0
	.set _Z19forward_step_kernelPKdS0_Pdii.has_recursion, 0
	.set _Z19forward_step_kernelPKdS0_Pdii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2848
; TotalNumSgprs: 44
; NumVgprs: 24
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 44
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
	.protected	_Z20backward_step_kernelPKdS0_Pdiii ; -- Begin function _Z20backward_step_kernelPKdS0_Pdiii
	.globl	_Z20backward_step_kernelPKdS0_Pdiii
	.p2align	8
	.type	_Z20backward_step_kernelPKdS0_Pdiii,@function
_Z20backward_step_kernelPKdS0_Pdiii:    ; @_Z20backward_step_kernelPKdS0_Pdiii
; %bb.0:
	s_load_b128 s[4:7], s[0:1], 0x18
	s_mov_b32 s2, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_gt_i32_e64 s4, v0
	s_cbranch_execz .LBB1_8
; %bb.1:
	s_load_b64 s[2:3], s[0:1], 0x10
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_add_i32 s5, s5, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s6, s5
	s_cbranch_scc1 .LBB1_7
; %bb.2:
	s_load_b128 s[12:15], s[0:1], 0x0
	v_mul_lo_u32 v1, s4, v0
	v_mov_b32_e32 v2, 0
	s_add_i32 s0, s6, 1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s0, s0, s4
	s_ashr_i32 s1, s0, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 3, v[1:2]
	s_lshl_b64 s[0:1], s[0:1], 3
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v5, vcc_lo, s12, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s13, v2, vcc_lo
	s_add_u32 s8, s14, s0
	s_addc_u32 s9, s15, s1
	v_dual_mov_b32 v2, 0xfe37e43c :: v_dual_mov_b32 v3, v5
	s_add_u32 s10, s2, s0
	v_dual_mov_b32 v1, 0x8800759c :: v_dual_mov_b32 v4, v6
	s_addc_u32 s11, s3, s1
	s_mov_b64 s[14:15], s[8:9]
	s_mov_b64 s[12:13], s[10:11]
	s_mov_b32 s1, s4
	.p2align	6
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[7:8], v[3:4], off
	s_load_b64 s[16:17], s[14:15], 0x0
	s_add_i32 s1, s1, -1
	v_add_co_u32 v3, s0, v3, 8
	s_add_u32 s14, s14, 8
	v_add_co_ci_u32_e64 v4, null, 0, v4, s0
	s_addc_u32 s15, s15, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[7:8], v[7:8], s[16:17]
	s_load_b64 s[16:17], s[12:13], 0x0
	s_add_u32 s12, s12, 8
	s_addc_u32 s13, s13, 0
	s_cmp_eq_u32 s1, 0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], s[16:17]
	v_cmp_gt_f64_e32 vcc_lo, v[7:8], v[1:2]
	v_dual_cndmask_b32 v2, v2, v8 :: v_dual_cndmask_b32 v1, v1, v7
	s_cbranch_scc0 .LBB1_3
; %bb.4:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_mov_b32 s12, 0x652b82fe
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
	s_mov_b32 s13, 0x3ff71547
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
	s_mov_b32 s1, s4
.LBB1_5:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[7:8], v[5:6], off
	s_load_b64 s[40:41], s[8:9], 0x0
	s_add_i32 s1, s1, -1
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[7:8], v[7:8], s[40:41]
	s_load_b64 s[40:41], s[10:11], 0x0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], s[40:41]
	v_add_f64 v[7:8], v[7:8], -v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[9:10], v[7:8], s[12:13]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[7:8]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[7:8]
	v_rndne_f64_e32 v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[9:10], s[14:15], v[7:8]
	v_cvt_i32_f64_e32 v15, v[9:10]
	v_fma_f64 v[11:12], v[9:10], s[16:17], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], s[20:21], s[18:19]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[24:25]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[28:29]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[34:35]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	v_fma_f64 v[13:14], v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[11:12], v[13:14], 1.0
	v_ldexp_f64 v[9:10], v[9:10], v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v10, 0x7ff00000, v10, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_add_u32 s8, s8, 8
	v_cndmask_b32_e32 v7, 0, v9, vcc_lo
	v_add_co_u32 v5, vcc_lo, v5, 8
	v_cndmask_b32_e64 v8, 0, v10, s0
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	s_addc_u32 s9, s9, 0
	s_add_u32 s10, s10, 8
	v_add_f64 v[3:4], v[3:4], v[7:8]
	s_addc_u32 s11, s11, 0
	s_cmp_eq_u32 s1, 0
	s_cbranch_scc0 .LBB1_5
; %bb.6:
	s_delay_alu instid0(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[5:6], v[3:4]
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[5:6]
	s_mov_b32 s0, 0x55555780
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
	v_frexp_exp_i32_f64_e32 v17, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[5:6]
	v_mul_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_mul_f64 v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[9:10], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[15:16], v[7:8], v[9:10]
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[9:10], v[9:10], v[13:14], s[0:1]
	v_ldexp_f64 v[13:14], v[7:8], 1
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[9:10], v[15:16], v[9:10]
	v_subrev_co_ci_u32_e64 v15, null, 0, v17, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_cmp_class_f64_e64 vcc_lo, v[3:4], 0x204
	v_cvt_f64_i32_e32 v[15:16], v15
	v_add_f64 v[11:12], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[5:6], v[5:6], 1
	v_mul_f64 v[17:18], v[15:16], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[11:12], -v[13:14]
	v_fma_f64 v[13:14], v[15:16], s[0:1], -v[17:18]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[9:10], -v[7:8]
	v_fma_f64 v[9:10], v[15:16], s[0:1], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[11:12], v[5:6]
	v_add_f64 v[17:18], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[7:8], v[13:14]
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[15:16], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[15:16], -v[19:20]
	v_add_f64 v[11:12], v[13:14], -v[19:20]
	v_add_f64 v[13:14], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[21:22]
	v_add_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[13:14], -v[9:10]
	v_add_f64 v[7:8], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[11:12]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	v_add_f64 v[17:18], v[15:16], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	v_add_f64 v[11:12], v[17:18], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[5:6], v[17:18], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v5, v5, v3 :: v_dual_cndmask_b32 v6, v6, v4
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[3:4]
	v_cndmask_b32_e32 v6, 0x7ff80000, v6, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[3:4]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[3:4]
	v_cndmask_b32_e32 v6, 0xfff00000, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[1:2], v[1:2], v[5:6]
.LBB1_7:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, s6, s4, v[0:1]
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s3, v4, vcc_lo
	global_store_b64 v[3:4], v[1:2], off
.LBB1_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z20backward_step_kernelPKdS0_Pdiii
		.amdhsa_group_segment_fixed_size 0
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
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 23
		.amdhsa_next_free_sgpr 42
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
.Lfunc_end1:
	.size	_Z20backward_step_kernelPKdS0_Pdiii, .Lfunc_end1-_Z20backward_step_kernelPKdS0_Pdiii
                                        ; -- End function
	.set _Z20backward_step_kernelPKdS0_Pdiii.num_vgpr, 23
	.set _Z20backward_step_kernelPKdS0_Pdiii.num_agpr, 0
	.set _Z20backward_step_kernelPKdS0_Pdiii.numbered_sgpr, 42
	.set _Z20backward_step_kernelPKdS0_Pdiii.num_named_barrier, 0
	.set _Z20backward_step_kernelPKdS0_Pdiii.private_seg_size, 0
	.set _Z20backward_step_kernelPKdS0_Pdiii.uses_vcc, 1
	.set _Z20backward_step_kernelPKdS0_Pdiii.uses_flat_scratch, 0
	.set _Z20backward_step_kernelPKdS0_Pdiii.has_dyn_sized_stack, 0
	.set _Z20backward_step_kernelPKdS0_Pdiii.has_recursion, 0
	.set _Z20backward_step_kernelPKdS0_Pdiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1824
; TotalNumSgprs: 44
; NumVgprs: 23
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 44
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
	.text
	.protected	_Z12gamma_kernelPKdS0_Pdii ; -- Begin function _Z12gamma_kernelPKdS0_Pdii
	.globl	_Z12gamma_kernelPKdS0_Pdii
	.p2align	8
	.type	_Z12gamma_kernelPKdS0_Pdii,@function
_Z12gamma_kernelPKdS0_Pdii:             ; @_Z12gamma_kernelPKdS0_Pdii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s9, s8
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB2_10
; %bb.1:
	s_abs_i32 s2, s8
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_cmp_gt_i32 s8, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v3, v0
	v_mul_lo_u32 v2, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v3, v2
	v_add_nc_u32_e32 v3, 1, v0
	v_subrev_nc_u32_e32 v4, s2, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_xor_b32_e32 v3, s8, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_cselect_b32 s1, -1, 0
	s_cmp_lt_i32 s8, 1
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v5, v0, s8
	v_ashrrev_i32_e32 v6, 31, v5
	s_cbranch_scc1 .LBB2_7
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[2:3], 3, v[5:6]
	v_mov_b32_e32 v4, 0xfe37e43c
	s_mov_b32 s9, s8
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v7, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s5, v3, vcc_lo
	v_add_co_u32 v9, vcc_lo, s6, v2
	v_add_co_ci_u32_e64 v10, null, s7, v3, vcc_lo
	v_mov_b32_e32 v3, 0x8800759c
	.p2align	6
.LBB2_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[11:12], v[7:8], off
	global_load_b64 v[13:14], v[9:10], off
	v_add_co_u32 v7, s0, v7, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, 0, v8, s0
	v_add_co_u32 v9, s0, v9, 8
	v_add_co_ci_u32_e64 v10, null, 0, v10, s0
	s_add_i32 s9, s9, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s9, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[11:12], v[11:12], v[13:14]
	v_cmp_gt_f64_e32 vcc_lo, v[11:12], v[3:4]
	v_dual_cndmask_b32 v4, v4, v12 :: v_dual_cndmask_b32 v3, v3, v11
	s_cbranch_scc0 .LBB2_3
; %bb.4:
	s_and_not1_b32 vcc_lo, exec_lo, s1
	s_cbranch_vccnz .LBB2_8
.LBB2_5:
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	s_mov_b32 s10, 0x652b82fe
	s_mov_b32 s12, 0xfefa39ef
	s_mov_b32 s14, 0x3b39803f
	s_mov_b32 s16, 0xfca7ab0c
	s_mov_b32 s18, 0x6a5dcb37
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v7, vcc_lo, s4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s5, v6, vcc_lo
	v_add_co_u32 v9, vcc_lo, s6, v5
	v_add_co_ci_u32_e64 v10, null, s7, v6, vcc_lo
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_mov_b32 s20, 0x623fde64
	s_mov_b32 s22, 0x7c89e6b0
	s_mov_b32 s24, 0x14761f6e
	s_mov_b32 s26, 0x1852b7b0
	s_mov_b32 s28, 0x11122322
	s_mov_b32 s30, 0x555502a1
	s_mov_b32 s34, 0x55555511
	s_mov_b32 s36, 11
	s_mov_b32 s11, 0x3ff71547
	s_mov_b32 s13, 0xbfe62e42
	s_mov_b32 s15, 0xbc7abc9e
	s_mov_b32 s17, 0x3e928af3
	s_mov_b32 s19, 0x3e5ade15
	s_mov_b32 s21, 0x3ec71dee
	s_mov_b32 s23, 0x3efa0199
	s_mov_b32 s25, 0x3f2a01a0
	s_mov_b32 s27, 0x3f56c16c
	s_mov_b32 s29, 0x3f811111
	s_mov_b32 s31, 0x3fa55555
	s_mov_b32 s35, 0x3fc55555
	s_mov_b32 s37, 0x3fe00000
.LBB2_6:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[11:12], v[7:8], off
	global_load_b64 v[13:14], v[9:10], off
	s_add_i32 s8, s8, -1
	s_waitcnt vmcnt(0)
	v_add_f64 v[11:12], v[11:12], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[11:12], -v[3:4]
	v_mul_f64 v[13:14], v[11:12], s[10:11]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[11:12]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[13:14], v[13:14]
	v_fma_f64 v[15:16], v[13:14], s[12:13], v[11:12]
	v_cvt_i32_f64_e32 v0, v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], s[14:15], v[15:16]
	v_fma_f64 v[17:18], v[15:16], s[18:19], s[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[20:21]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[24:25]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[28:29]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[34:35]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[13:14], v[13:14], v0
	v_cndmask_b32_e32 v0, 0x7ff00000, v14, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_cmp_eq_u32 s8, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v11, 0, v13, vcc_lo
	v_add_co_u32 v7, vcc_lo, v7, 8
	v_cndmask_b32_e64 v12, 0, v0, s0
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_add_co_u32 v9, vcc_lo, v9, 8
	v_add_f64 v[5:6], v[5:6], v[11:12]
	v_add_co_ci_u32_e64 v10, null, 0, v10, vcc_lo
	s_cbranch_scc0 .LBB2_6
	s_branch .LBB2_9
.LBB2_7:
	v_mov_b32_e32 v3, 0x8800759c
	v_mov_b32_e32 v4, 0xfe37e43c
	s_and_not1_b32 vcc_lo, exec_lo, s1
	s_cbranch_vccz .LBB2_5
.LBB2_8:
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
.LBB2_9:
	s_delay_alu instid0(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[7:8], v[5:6]
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[7:8]
	s_mov_b32 s0, 0x55555780
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_ldexp_f64 v[7:8], v[7:8], v0
	v_frexp_exp_i32_f64_e32 v0, v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[7:8], 1.0
	v_add_f64 v[15:16], v[7:8], -1.0
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[9:10], v[9:10]
	v_fma_f64 v[15:16], v[11:12], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[17:18], v[9:10], v[11:12]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[11:12], v[11:12], v[15:16], s[0:1]
	v_ldexp_f64 v[15:16], v[9:10], 1
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[11:12], v[17:18], v[11:12]
	v_cvt_f64_i32_e32 v[17:18], v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v25, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v26, null, s5, v1, vcc_lo
	v_add_co_u32 v27, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v28, null, s7, v1, vcc_lo
	global_load_b64 v[25:26], v[25:26], off
	v_cmp_class_f64_e64 vcc_lo, v[5:6], 0x204
	v_add_f64 v[13:14], v[15:16], v[11:12]
	v_mul_f64 v[19:20], v[17:18], s[0:1]
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[13:14], -v[15:16]
	v_fma_f64 v[15:16], v[17:18], s[0:1], -v[19:20]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[11:12], -v[9:10]
	v_fma_f64 v[11:12], v[17:18], s[0:1], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_add_f64 v[9:10], v[19:20], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[13:14], v[7:8]
	v_add_f64 v[19:20], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[9:10], v[15:16]
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[17:18], -v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[17:18], -v[21:22]
	v_add_f64 v[13:14], v[15:16], -v[21:22]
	v_add_f64 v[19:20], v[11:12], v[7:8]
	global_load_b64 v[15:16], v[27:28], off
	v_add_f64 v[9:10], v[9:10], -v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], v[9:10]
	v_add_f64 v[13:14], v[19:20], -v[11:12]
	v_add_f64 v[9:10], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[19:20], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[21:22], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	v_add_f64 v[13:14], v[21:22], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[11:12]
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_add_f64 v[7:8], v[21:22], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, v7, v5 :: v_dual_cndmask_b32 v7, v8, v6
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[5:6]
	v_cndmask_b32_e32 v8, 0x7ff80000, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[5:6]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v7, 0, v2, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[5:6]
	v_cndmask_b32_e32 v8, 0xfff00000, v8, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_add_f64 v[2:3], v[3:4], v[7:8]
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], v[25:26], v[15:16]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[4:5], -v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB2_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z12gamma_kernelPKdS0_Pdii
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
		.amdhsa_next_free_sgpr 38
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
		.amdhsa_inst_pref_size 17
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
	.size	_Z12gamma_kernelPKdS0_Pdii, .Lfunc_end2-_Z12gamma_kernelPKdS0_Pdii
                                        ; -- End function
	.set _Z12gamma_kernelPKdS0_Pdii.num_vgpr, 29
	.set _Z12gamma_kernelPKdS0_Pdii.num_agpr, 0
	.set _Z12gamma_kernelPKdS0_Pdii.numbered_sgpr, 38
	.set _Z12gamma_kernelPKdS0_Pdii.num_named_barrier, 0
	.set _Z12gamma_kernelPKdS0_Pdii.private_seg_size, 0
	.set _Z12gamma_kernelPKdS0_Pdii.uses_vcc, 1
	.set _Z12gamma_kernelPKdS0_Pdii.uses_flat_scratch, 0
	.set _Z12gamma_kernelPKdS0_Pdii.has_dyn_sized_stack, 0
	.set _Z12gamma_kernelPKdS0_Pdii.has_recursion, 0
	.set _Z12gamma_kernelPKdS0_Pdii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2088
; TotalNumSgprs: 40
; NumVgprs: 29
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 40
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
	.protected	_Z19viterbi_step_kernelPKdS0_PdPiii ; -- Begin function _Z19viterbi_step_kernelPKdS0_PdPiii
	.globl	_Z19viterbi_step_kernelPKdS0_PdPiii
	.p2align	8
	.type	_Z19viterbi_step_kernelPKdS0_PdPiii,@function
_Z19viterbi_step_kernelPKdS0_PdPiii:    ; @_Z19viterbi_step_kernelPKdS0_PdPiii
; %bb.0:
	s_load_b64 s[8:9], s[0:1], 0x20
	s_mov_b32 s2, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_gt_i32_e64 s8, v0
	s_cbranch_execz .LBB3_7
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x8
	s_load_b64 s[2:3], s[0:1], 0x18
	v_mov_b32_e32 v1, 0
	s_cmp_eq_u32 s9, 0
	s_mov_b32 s11, 0
	s_cbranch_scc1 .LBB3_8
; %bb.2:
	s_load_b64 s[0:1], s[0:1], 0x0
	v_dual_mov_b32 v5, 0xfe37e43c :: v_dual_lshlrev_b32 v2, 3, v0
	s_add_i32 s10, s9, -1
	v_mov_b32_e32 v4, 0x8800759c
	s_mul_i32 s12, s10, s8
	s_mov_b32 s10, s8
	s_ashr_i32 s13, s12, 31
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b64 s[14:15], s[12:13], 3
	s_lshl_b64 s[12:13], s[10:11], 3
	s_waitcnt lgkmcnt(0)
	s_add_u32 s14, s6, s14
	s_addc_u32 s15, s7, s15
	v_add_co_u32 v2, s0, s0, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, 0, s0
	.p2align	6
.LBB3_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[6:7], v[2:3], off
	s_load_b64 s[0:1], s[14:15], 0x0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[6:7], s[0:1], v[6:7]
	v_add_co_u32 v2, s0, v2, s12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v3, null, s13, v3, s0
	v_cmp_gt_f64_e32 vcc_lo, v[6:7], v[4:5]
	v_dual_cndmask_b32 v5, v5, v7 :: v_dual_cndmask_b32 v4, v4, v6
	v_cndmask_b32_e64 v1, v1, s11, vcc_lo
	s_add_i32 s11, s11, 1
	s_add_u32 s14, s14, 8
	s_addc_u32 s15, s15, 0
	s_cmp_eq_u32 s8, s11
	s_cbranch_scc0 .LBB3_3
; %bb.4:
	v_mad_u64_u32 v[2:3], null, s9, s8, v[0:1]
	s_delay_alu instid0(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	s_branch .LBB3_6
.LBB3_5:
	v_cvt_f64_u32_e32 v[0:1], s8
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[4:5], null, v[0:1], v[0:1], 1.0
	v_div_scale_f64 v[10:11], vcc_lo, 1.0, v[0:1], 1.0
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_mul_f64 v[8:9], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[8:9], v[10:11]
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[0:1], v[4:5], v[0:1], 1.0
	v_frexp_mant_f64_e32 v[4:5], v[0:1]
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
	v_frexp_exp_i32_f64_e32 v16, v[0:1]
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
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[0:1]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_subrev_co_ci_u32_e64 v14, null, 0, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[0:1]
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
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
	v_cmp_neq_f64_e64 s0, 0, v[0:1]
	v_mov_b32_e32 v1, -1
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
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
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v5, 0xfff00000, v5, s0
.LBB3_6:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[6:7], 3, v[2:3]
	v_lshlrev_b64 v[2:3], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v8, vcc_lo, s4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s5, v7, vcc_lo
	v_add_co_u32 v6, vcc_lo, s6, v6
	v_add_co_ci_u32_e64 v7, null, s7, v7, vcc_lo
	global_load_b64 v[8:9], v[8:9], off
	v_add_co_u32 v2, vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	global_store_b64 v[6:7], v[4:5], off
	global_store_b32 v[2:3], v1, off
.LBB3_7:
	s_endpgm
.LBB3_8:
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
                                        ; implicit-def: $vgpr4_vgpr5
	s_cbranch_execnz .LBB3_5
	s_branch .LBB3_6
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z19viterbi_step_kernelPKdS0_PdPiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 40
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
.Lfunc_end3:
	.size	_Z19viterbi_step_kernelPKdS0_PdPiii, .Lfunc_end3-_Z19viterbi_step_kernelPKdS0_PdPiii
                                        ; -- End function
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.num_vgpr, 22
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.num_agpr, 0
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.numbered_sgpr, 16
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.num_named_barrier, 0
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.private_seg_size, 0
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.uses_vcc, 1
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.uses_flat_scratch, 0
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.has_dyn_sized_stack, 0
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.has_recursion, 0
	.set _Z19viterbi_step_kernelPKdS0_PdPiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1420
; TotalNumSgprs: 18
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 18
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
	.protected	_Z24viterbi_backtrace_kernelPKdPKiPiii ; -- Begin function _Z24viterbi_backtrace_kernelPKdPKiPiii
	.globl	_Z24viterbi_backtrace_kernelPKdPKiPiii
	.p2align	8
	.type	_Z24viterbi_backtrace_kernelPKdPKiPiii,@function
_Z24viterbi_backtrace_kernelPKdPKiPiii: ; @_Z24viterbi_backtrace_kernelPKdPKiPiii
; %bb.0:
	s_clause 0x1
	s_load_b64 s[2:3], s[0:1], 0x18
	s_load_b128 s[4:7], s[0:1], 0x8
	s_mov_b32 s9, 0
	s_waitcnt lgkmcnt(0)
	s_add_i32 s12, s3, -1
	s_cmp_lt_i32 s2, 2
	s_mul_i32 s8, s12, s2
	s_cbranch_scc1 .LBB4_3
; %bb.1:
	s_load_b64 s[0:1], s[0:1], 0x0
	s_ashr_i32 s9, s8, 31
	s_mov_b32 s13, 1
	s_lshl_b64 s[10:11], s[8:9], 3
	s_mov_b32 s9, 0
	s_waitcnt lgkmcnt(0)
	s_add_u32 s10, s0, s10
	s_addc_u32 s11, s1, s11
	s_load_b64 s[0:1], s[10:11], 0x0
	s_add_u32 s10, s10, 8
	s_addc_u32 s11, s11, 0
.LBB4_2:                                ; =>This Inner Loop Header: Depth=1
	s_load_b64 s[14:15], s[10:11], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e64 s16, s[14:15], s[0:1]
	s_and_b32 s16, s16, exec_lo
	s_cselect_b32 s1, s15, s1
	s_cselect_b32 s0, s14, s0
	s_cselect_b32 s9, s13, s9
	s_add_i32 s13, s13, 1
	s_add_u32 s10, s10, 8
	s_addc_u32 s11, s11, 0
	s_cmp_eq_u32 s2, s13
	s_cbranch_scc0 .LBB4_2
.LBB4_3:
	s_ashr_i32 s1, s3, 31
	s_mov_b32 s0, s3
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s9
	s_lshl_b64 s[0:1], s[0:1], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s0, s6, s0
	s_addc_u32 s1, s7, s1
	s_cmp_lt_i32 s3, 2
	global_store_b32 v0, v1, s[0:1] offset:-4
	s_cbranch_scc1 .LBB4_6
; %bb.4:
	s_add_i32 s0, s3, -2
	s_mov_b32 s1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[0:1], s[0:1], 2
	s_add_u32 s0, s6, s0
	s_addc_u32 s1, s7, s1
.LBB4_5:                                ; =>This Inner Loop Header: Depth=1
	s_add_i32 s6, s9, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s7, s6, 31
	s_lshl_b64 s[6:7], s[6:7], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s6, s4, s6
	s_addc_u32 s7, s5, s7
	s_add_i32 s12, s12, -1
	s_load_b32 s9, s[6:7], 0x0
	s_waitcnt lgkmcnt(0)
	v_mov_b32_e32 v1, s9
	global_store_b32 v0, v1, s[0:1]
	s_add_u32 s0, s0, -4
	s_addc_u32 s1, s1, -1
	s_sub_i32 s8, s8, s2
	s_cmp_eq_u32 s12, 0
	s_cbranch_scc0 .LBB4_5
.LBB4_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z24viterbi_backtrace_kernelPKdPKiPiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 32
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
		.amdhsa_next_free_vgpr 2
		.amdhsa_next_free_sgpr 17
		.amdhsa_reserve_vcc 0
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
.Lfunc_end4:
	.size	_Z24viterbi_backtrace_kernelPKdPKiPiii, .Lfunc_end4-_Z24viterbi_backtrace_kernelPKdPKiPiii
                                        ; -- End function
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.num_vgpr, 2
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.num_agpr, 0
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.numbered_sgpr, 17
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.num_named_barrier, 0
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.private_seg_size, 0
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.uses_vcc, 0
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.uses_flat_scratch, 0
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.has_dyn_sized_stack, 0
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.has_recursion, 0
	.set _Z24viterbi_backtrace_kernelPKdPKiPiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 304
; TotalNumSgprs: 17
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 17
; NumVGPRsForWavesPerEU: 2
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
	.type	__hip_cuid_422a2e29fa6152a3,@object ; @__hip_cuid_422a2e29fa6152a3
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_422a2e29fa6152a3
__hip_cuid_422a2e29fa6152a3:
	.byte	0                               ; 0x0
	.size	__hip_cuid_422a2e29fa6152a3, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_422a2e29fa6152a3
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 32
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z19forward_step_kernelPKdS0_Pdii
    .private_segment_fixed_size: 0
    .sgpr_count:     44
    .sgpr_spill_count: 0
    .symbol:         _Z19forward_step_kernelPKdS0_Pdii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     24
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 36
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z20backward_step_kernelPKdS0_Pdiii
    .private_segment_fixed_size: 0
    .sgpr_count:     44
    .sgpr_spill_count: 0
    .symbol:         _Z20backward_step_kernelPKdS0_Pdiii.kd
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
      - .actual_access:  write_only
        .address_space:  global
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
    .name:           _Z12gamma_kernelPKdS0_Pdii
    .private_segment_fixed_size: 0
    .sgpr_count:     40
    .sgpr_spill_count: 0
    .symbol:         _Z12gamma_kernelPKdS0_Pdii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     29
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
      - .actual_access:  write_only
        .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
      - .offset:         32
        .size:           4
        .value_kind:     by_value
      - .offset:         36
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 40
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z19viterbi_step_kernelPKdS0_PdPiii
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z19viterbi_step_kernelPKdS0_PdPiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     22
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
      - .actual_access:  write_only
        .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .offset:         28
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 32
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z24viterbi_backtrace_kernelPKdPKiPiii
    .private_segment_fixed_size: 0
    .sgpr_count:     17
    .sgpr_spill_count: 0
    .symbol:         _Z24viterbi_backtrace_kernelPKdPKiPiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
