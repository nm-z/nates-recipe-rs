	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	ropex_qk_kernel         ; -- Begin function ropex_qk_kernel
	.globl	ropex_qk_kernel
	.p2align	8
	.type	ropex_qk_kernel,@function
ropex_qk_kernel:                        ; @ropex_qk_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b64 s[16:17], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_lshr_b32 s2, s17, 31
	s_add_i32 s2, s17, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s6, s2, 1
	s_mul_i32 s2, s6, s16
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_10
; %bb.1:
	s_abs_i32 s2, s6
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	v_cvt_f64_i32_e32 v[5:6], s17
	s_mov_b32 s4, 0x968915a9
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_mov_b32 s8, 0x4222de17
	s_mov_b32 s5, 0x3fba6564
	s_mov_b32 s9, 0x3fbdee67
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
	v_xor_b32_e32 v3, s6, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_load_b64 s[2:3], s[0:1], 0x30
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s6
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[2:3], 0x0
	v_sub_nc_u32_e32 v2, v1, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cvt_f64_i32_e32 v[3:4], v2
	s_waitcnt lgkmcnt(0)
	v_mov_b32_e32 v1, s3
	v_add_f64 v[3:4], v[3:4], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[7:8], null, v[5:6], v[5:6], v[3:4]
	v_div_scale_f64 v[13:14], vcc_lo, v[3:4], v[5:6], v[3:4]
	v_rcp_f64_e32 v[9:10], v[7:8]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[7:8], v[11:12], v[13:14]
	v_div_fmas_f64 v[7:8], v[7:8], v[9:10], v[11:12]
	v_cmp_neq_f64_e64 vcc_lo, s[2:3], 1.0
	s_mov_b32 s3, 0x3fe55555
	v_div_fixup_f64 v[3:4], v[7:8], v[5:6], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0x3ff00000, v4, vcc_lo
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[3:4]
	v_cndmask_b32_e32 v6, 0x3ff00000, v1, vcc_lo
	v_cndmask_b32_e64 v5, 0, s2, vcc_lo
	s_mov_b32 s2, 0x55555555
	v_frexp_mant_f64_e64 v[7:8], |v[5:6]|
	v_cmp_lt_f64_e64 s7, |v[5:6]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[7:8]
	v_cndmask_b32_e64 v1, 0, 1, vcc_lo
	v_ldexp_f64 v[7:8], v[7:8], v1
	v_frexp_exp_i32_f64_e32 v1, v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[7:8], 1.0
	v_add_f64 v[15:16], v[7:8], -1.0
	v_subrev_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
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
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], s[8:9], s[4:5]
	s_mov_b32 s4, 0x3abe935a
	s_mov_b32 s5, 0x3fbe25e4
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_mul_f64 v[23:24], v[9:10], v[15:16]
	s_mov_b32 s8, 0x652b82fe
	s_mov_b32 s9, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x47e6c9c2
	s_mov_b32 s5, 0x3fc110ef
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0xcfa74449
	s_mov_b32 s5, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x71bf3c30
	s_mov_b32 s5, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x1c7792ce
	s_mov_b32 s5, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x924920da
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x9999999c
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s5, 0x3c7abc9e
	s_mov_b32 s4, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[15:16], v[17:18]
	v_fma_f64 v[13:14], v[15:16], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[17:18], v[13:14]
	v_add_f64 v[17:18], v[19:20], v[13:14]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[21:22], v[17:18], s[2:3]
	v_add_f64 v[19:20], v[17:18], -v[19:20]
	s_mov_b32 s3, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[25:26], v[21:22], s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	v_fma_f64 v[19:20], v[15:16], v[9:10], -v[23:24]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_add_f64 v[17:18], v[17:18], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], s[2:3]
	v_fma_f64 v[15:16], v[15:16], v[7:8], v[19:20]
	s_mov_b32 s3, 0x3fe62e42
	s_mov_b32 s2, 0xfefa39ef
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], v[17:18]
	v_fma_f64 v[11:12], v[11:12], v[9:10], v[15:16]
	v_ldexp_f64 v[9:10], v[9:10], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[21:22], v[13:14]
	v_add_f64 v[17:18], v[23:24], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[21:22], -v[15:16]
	v_mul_f64 v[21:22], v[17:18], v[15:16]
	v_add_f64 v[23:24], v[17:18], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], v[19:20]
	v_fma_f64 v[19:20], v[17:18], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[23:24]
	v_fma_f64 v[13:14], v[17:18], v[13:14], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[11:12], v[11:12], v[15:16], v[13:14]
	v_cvt_f64_i32_e32 v[15:16], v1
	v_ashrrev_i32_e32 v1, 31, v0
	v_add_f64 v[13:14], v[21:22], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[9:10], v[13:14]
	v_add_f64 v[19:20], v[13:14], -v[21:22]
	v_mul_f64 v[21:22], v[15:16], s[2:3]
	v_add_f64 v[9:10], v[17:18], -v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	v_fma_f64 v[19:20], v[15:16], s[2:3], -v[21:22]
	s_mov_b32 s3, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[13:14], -v[9:10]
	v_add_f64 v[7:8], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[15:16], s[4:5], v[19:20]
	s_mov_b32 s5, 0xbc7abc9e
	v_add_f64 v[7:8], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[21:22], v[11:12]
	v_add_f64 v[13:14], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[9:10], -v[21:22]
	v_add_f64 v[15:16], v[9:10], v[13:14]
	v_add_f64 v[17:18], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[11:12], v[11:12], -v[21:22]
	v_lshlrev_b64 v[21:22], 3, v[0:1]
	v_add_f64 v[19:20], v[15:16], -v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
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
	v_mul_f64 v[13:14], v[3:4], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	v_fma_f64 v[9:10], v[3:4], v[9:10], -v[13:14]
	v_cmp_class_f64_e64 vcc_lo, v[13:14], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[3:4], v[7:8], v[9:10]
	v_add_f64 v[9:10], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v12, v10, v14 :: v_dual_cndmask_b32 v11, v9, v13
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	v_mul_f64 v[15:16], v[11:12], s[8:9]
	s_load_b256 s[8:15], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v21, vcc_lo, s12, v21
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v22, null, s13, v22, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[11:12]|
	global_load_b64 v[21:22], v[21:22], off
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_trunc_f64_e32 v[9:10], v[3:4]
	v_rndne_f64_e32 v[15:16], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v8, 0, v8 :: v_dual_cndmask_b32 v7, 0, v7
	v_fma_f64 v[17:18], v[15:16], s[2:3], v[11:12]
	s_mov_b32 s2, 0xfca7ab0c
	s_mov_b32 s3, 0x3e928af3
	v_cvt_i32_f64_e32 v1, v[15:16]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], s[4:5], v[17:18]
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], s[4:5], s[2:3]
	s_mov_b32 s2, 0x623fde64
	s_mov_b32 s3, 0x3ec71dee
	v_cmp_neq_f64_e64 s5, v[3:4], |v[3:4]|
	v_cmp_eq_f64_e64 s4, 0, v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x7c89e6b0
	s_mov_b32 s3, 0x3efa0199
	s_xor_b32 s5, s5, s7
	v_cmp_class_f64_e64 s7, v[5:6], 0x204
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x14761f6e
	s_mov_b32 s3, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x11122322
	s_mov_b32 s3, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x555502a1
	s_mov_b32 s3, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x55555511
	s_mov_b32 s3, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 11
	s_mov_b32 s3, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	v_cmp_nlt_f64_e64 s2, 0x40900000, v[11:12]
	v_cmp_ngt_f64_e64 s3, 0xc090cc00, v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	s_and_b32 vcc_lo, s3, s2
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[13:14], v[15:16], v1
	v_mul_f64 v[15:16], v[3:4], 0.5
	v_cndmask_b32_e64 v1, 0x7ff00000, v14, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[11:12], v[15:16]
	v_cndmask_b32_e32 v13, 0, v13, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[9:10], v[3:4]
	v_cndmask_b32_e64 v10, 0x7ff00000, 0, s5
	v_cndmask_b32_e64 v14, 0, v1, s3
	v_cmp_neq_f64_e64 s5, |v[5:6]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[7:8], v[13:14], v[7:8], v[13:14]
	v_cmp_class_f64_e64 s3, v[13:14], 0x204
	v_cmp_neq_f64_e64 s2, v[11:12], v[15:16]
	v_cndmask_b32_e64 v10, 0x3ff00000, v10, s5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v8, v8, v14, s3
	v_cndmask_b32_e64 v7, v7, v13, s3
	v_cmp_gt_f64_e64 s3, 0, v[3:4]
	v_cndmask_b32_e32 v9, 0, v7, vcc_lo
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v1, 0x3ff00000, v6, s2
	v_cndmask_b32_e64 v11, 0, v6, s2
	s_or_b32 s2, s4, s7
	v_bfi_b32 v1, 0x7fffffff, v8, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v8, 0x7ff80000, v1, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[5:6]
	s_xor_b32 s3, s3, s4
	v_cndmask_b32_e32 v7, v7, v9, vcc_lo
	v_cndmask_b32_e32 v1, v1, v8, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[3:4], 0x204
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v8, v[21:22]
	v_cndmask_b32_e64 v9, 0x7ff00000, 0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_bfi_b32 v9, 0x7fffffff, v9, v11
	v_cndmask_b32_e32 v1, v1, v10, vcc_lo
	v_cndmask_b32_e64 v1, v1, v9, s2
	s_or_b32 s2, s2, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[5:6], v[3:4]
	v_cvt_f64_i32_e32 v[3:4], v8
	v_cndmask_b32_e64 v7, v7, 0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0, v7, vcc_lo
	v_cndmask_b32_e32 v6, 0x7ff80000, v1, vcc_lo
                                        ; implicit-def: $vgpr1
	v_div_scale_f64 v[7:8], null, v[5:6], v[5:6], v[3:4]
	v_div_scale_f64 v[13:14], vcc_lo, v[3:4], v[5:6], v[3:4]
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
	v_div_fixup_f64 v[4:5], v[7:8], v[5:6], v[3:4]
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_ngt_f64_e64 s4, 0x41d00000, |v[4:5]|
	v_trig_preop_f64 v[18:19], |v[4:5]|, 0
	v_trig_preop_f64 v[16:17], |v[4:5]|, 1
	v_ldexp_f64 v[20:21], |v[4:5]|, 0xffffff80
	v_trig_preop_f64 v[10:11], |v[4:5]|, 2
	v_and_b32_e32 v3, 0x7fffffff, v5
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s2, exec_lo, s2
	s_cbranch_execz .LBB0_3
; %bb.2:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[4:5]|
	v_mov_b32_e32 v34, 0
	s_mov_b32 s12, 0x54442d18
	s_mov_b32 s13, 0x3ff921fb
	s_mov_b32 s18, 0x33145c07
	s_mov_b32 s19, 0x3c91a626
	v_dual_cndmask_b32 v7, v3, v21 :: v_dual_cndmask_b32 v6, v4, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[8:9], v[18:19], v[6:7]
	v_mul_f64 v[12:13], v[16:17], v[6:7]
	v_mul_f64 v[30:31], v[10:11], v[6:7]
	v_fma_f64 v[14:15], v[18:19], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[32:33], v[16:17], v[6:7], -v[12:13]
	v_fma_f64 v[6:7], v[10:11], v[6:7], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[12:13], v[14:15]
	v_add_f64 v[24:25], v[22:23], -v[12:13]
	v_add_f64 v[28:29], v[8:9], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[22:23], -v[24:25]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	v_ldexp_f64 v[24:25], v[28:29], -2
	v_add_f64 v[8:9], v[28:29], -v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], -v[26:27]
	v_add_f64 v[26:27], v[30:31], v[32:33]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[24:25]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[8:9], v[22:23], -v[8:9]
	v_add_f64 v[12:13], v[14:15], v[12:13]
	v_fract_f64_e32 v[14:15], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[26:27], v[12:13]
	v_ldexp_f64 v[14:15], v[14:15], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[8:9], v[22:23]
	v_dual_cndmask_b32 v15, 0, v15 :: v_dual_cndmask_b32 v14, 0, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[28:29], v[24:25], v[14:15]
	v_add_f64 v[8:9], v[24:25], -v[8:9]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[28:29]
	v_add_f64 v[28:29], v[26:27], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[22:23], -v[8:9]
	v_cndmask_b32_e64 v35, 0, 0x40100000, vcc_lo
	v_add_f64 v[39:40], v[26:27], -v[28:29]
	v_add_f64 v[28:29], v[32:33], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[14:15], v[34:35]
	v_add_f64 v[35:36], v[22:23], -v[26:27]
	v_add_f64 v[32:33], v[30:31], -v[39:40]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[37:38], v[24:25], v[14:15]
	v_add_f64 v[41:42], v[22:23], -v[35:36]
	v_add_f64 v[12:13], v[12:13], -v[35:36]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[28:29], v[28:29], v[32:33]
	v_cvt_i32_f64_e32 v1, v[37:38]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[26:27], -v[41:42]
	v_cvt_f64_i32_e32 v[35:36], v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[26:27]
	v_add_f64 v[14:15], v[14:15], -v[35:36]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[28:29], v[12:13]
	v_add_f64 v[26:27], v[24:25], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[26:27], -v[14:15]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[24:25], -v[12:13]
	v_cndmask_b32_e64 v35, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[26:27], -v[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[8:9], v[6:7]
	v_mul_f64 v[14:15], v[12:13], s[12:13]
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[12:13], s[12:13], -v[14:15]
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[12:13], s[18:19], v[22:23]
	v_fma_f64 v[8:9], v[6:7], s[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[14:15], v[8:9]
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_or_saveexec_b32 s5, s2
	s_load_b64 s[2:3], s[0:1], 0x20
	s_xor_b32 exec_lo, exec_lo, s5
	s_cbranch_execz .LBB0_5
	s_branch .LBB0_4
.LBB0_3:
	s_or_saveexec_b32 s5, s2
	s_load_b64 s[2:3], s[0:1], 0x20
	s_xor_b32 exec_lo, exec_lo, s5
	s_cbranch_execz .LBB0_5
.LBB0_4:
	s_mov_b32 s0, 0x6dc9c883
	s_mov_b32 s1, 0x3fe45f30
	s_mov_b32 s13, 0xbc91a626
	v_mul_f64 v[6:7], |v[4:5]|, s[0:1]
	s_mov_b32 s0, 0x54442d18
	s_mov_b32 s1, 0xbff921fb
	s_mov_b32 s12, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[12:13], v[6:7]
	v_fma_f64 v[6:7], v[12:13], s[0:1], |v[4:5]|
	v_mul_f64 v[8:9], v[12:13], s[12:13]
	s_mov_b32 s0, 0x252049c0
	s_mov_b32 s1, 0xb97b839a
	v_cvt_i32_f64_e32 v1, v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], v[12:13], s[12:13], v[6:7]
	v_add_f64 v[14:15], v[6:7], v[8:9]
	s_mov_b32 s13, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], v[12:13], s[12:13], v[8:9]
	v_add_f64 v[6:7], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[12:13], s[0:1], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[22:23], v[8:9]
	v_add_f64 v[14:15], v[6:7], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
.LBB0_5:
	s_or_b32 exec_lo, exec_lo, s5
                                        ; implicit-def: $vgpr22
                                        ; implicit-def: $vgpr12_vgpr13
                                        ; implicit-def: $vgpr14_vgpr15
	s_and_saveexec_b32 s0, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s0
	s_cbranch_execz .LBB0_7
; %bb.6:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[4:5]|
	v_mov_b32_e32 v32, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s12, 0x33145c07
	s_mov_b32 s13, 0x3c91a626
	v_dual_cndmask_b32 v13, v3, v21 :: v_dual_cndmask_b32 v12, v4, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[14:15], v[18:19], v[12:13]
	v_mul_f64 v[20:21], v[16:17], v[12:13]
	v_mul_f64 v[30:31], v[10:11], v[12:13]
	v_fma_f64 v[18:19], v[18:19], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[16:17], v[12:13], -v[20:21]
	v_fma_f64 v[10:11], v[10:11], v[12:13], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[20:21], v[18:19]
	v_add_f64 v[24:25], v[22:23], -v[20:21]
	v_add_f64 v[28:29], v[14:15], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[22:23], -v[24:25]
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_ldexp_f64 v[24:25], v[28:29], -2
	v_add_f64 v[14:15], v[28:29], -v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_add_f64 v[26:27], v[30:31], v[16:17]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[24:25]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[22:23], -v[14:15]
	v_add_f64 v[18:19], v[18:19], v[20:21]
	v_fract_f64_e32 v[20:21], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[26:27], v[18:19]
	v_ldexp_f64 v[20:21], v[20:21], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[14:15], v[22:23]
	v_dual_cndmask_b32 v21, 0, v21 :: v_dual_cndmask_b32 v20, 0, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[28:29], v[24:25], v[20:21]
	v_add_f64 v[12:13], v[24:25], -v[14:15]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[28:29]
	v_add_f64 v[28:29], v[26:27], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[22:23], -v[12:13]
	v_cndmask_b32_e64 v33, 0, 0x40100000, vcc_lo
	v_add_f64 v[37:38], v[26:27], -v[28:29]
	v_add_f64 v[16:17], v[16:17], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], v[32:33]
	v_add_f64 v[33:34], v[22:23], -v[26:27]
	v_add_f64 v[28:29], v[30:31], -v[37:38]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[35:36], v[24:25], v[20:21]
	v_add_f64 v[39:40], v[22:23], -v[33:34]
	v_add_f64 v[18:19], v[18:19], -v[33:34]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[16:17], v[28:29]
	v_cvt_i32_f64_e32 v3, v[35:36]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[26:27], -v[39:40]
	v_cvt_f64_i32_e32 v[33:34], v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], v[26:27]
	v_add_f64 v[20:21], v[20:21], -v[33:34]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[16:17], v[18:19]
	v_add_f64 v[16:17], v[24:25], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[14:15], v[16:17], -v[20:21]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[12:13], v[10:11]
	v_add_f64 v[12:13], v[24:25], -v[14:15]
	v_cndmask_b32_e64 v33, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v22, null, 0, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[16:17], -v[32:33]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_mul_f64 v[16:17], v[14:15], s[4:5]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], s[4:5], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], s[12:13], v[18:19]
	v_fma_f64 v[10:11], v[10:11], s[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], v[10:11]
	v_add_f64 v[14:15], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[14:15], v[10:11], -v[14:15]
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execnz .LBB0_8
	s_branch .LBB0_9
.LBB0_7:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB0_9
.LBB0_8:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s13, 0xbc91a626
	v_mul_f64 v[10:11], |v[4:5]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s12, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], s[4:5], |v[4:5]|
	v_mul_f64 v[14:15], v[10:11], s[12:13]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	v_cvt_i32_f64_e32 v22, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[10:11], s[12:13], v[12:13]
	v_add_f64 v[16:17], v[12:13], v[14:15]
	s_mov_b32 s13, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[14:15]
	v_fma_f64 v[14:15], v[10:11], s[12:13], v[14:15]
	v_add_f64 v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[10:11], s[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], v[14:15]
	v_add_f64 v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
.LBB0_9:
	s_or_b32 exec_lo, exec_lo, s0
	v_mul_f64 v[10:11], v[6:7], v[6:7]
	v_mul_f64 v[16:17], v[12:13], v[12:13]
	v_ashrrev_i32_e32 v3, 31, v2
	s_mov_b32 s0, 0xb42fdfa7
	s_mov_b32 s12, 0xf9a43bb8
	s_mov_b32 s1, 0xbe5ae600
	s_mov_b32 s13, 0x3de5e0b2
	v_mad_i64_i32 v[20:21], null, v0, s17, v[2:3]
	s_ashr_i32 s7, s6, 31
	v_mul_f64 v[39:40], v[8:9], 0.5
	s_lshl_b64 s[4:5], s[6:7], 3
	s_mov_b32 s6, 0x46cc5e42
	s_mov_b32 s7, 0xbda907db
	v_mul_f64 v[45:46], v[14:15], 0.5
	v_lshlrev_b64 v[2:3], 3, v[20:21]
	v_and_b32_e32 v0, 1, v1
	v_lshlrev_b32_e32 v1, 30, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v20, vcc_lo, s8, v2
	v_add_co_ci_u32_e64 v21, null, s9, v3, vcc_lo
	s_mov_b32 s8, 0x796cde01
	v_add_co_u32 v23, vcc_lo, v20, s4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v24, null, s5, v21, vcc_lo
	s_clause 0x1
	global_load_b64 v[20:21], v[20:21], off
	global_load_b64 v[23:24], v[23:24], off
	s_mov_b32 s9, 0x3ec71de3
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	v_and_b32_e32 v1, 0x80000000, v1
	v_fma_f64 v[18:19], v[10:11], s[12:13], s[0:1]
	v_fma_f64 v[25:26], v[16:17], s[12:13], s[0:1]
	s_mov_b32 s0, 0x9037ab78
	s_mov_b32 s1, 0x3e21eeb6
	v_mul_f64 v[29:30], v[10:11], 0.5
	v_fma_f64 v[27:28], v[10:11], s[6:7], s[0:1]
	v_fma_f64 v[31:32], v[16:17], s[6:7], s[0:1]
	v_mul_f64 v[33:34], v[16:17], 0.5
	s_mov_b32 s0, 0xa17f65f6
	s_mov_b32 s6, 0x19e83e5c
	s_mov_b32 s1, 0xbe927e4f
	s_mov_b32 s7, 0xbf2a01a0
	v_mul_f64 v[41:42], v[6:7], -v[10:11]
	v_mul_f64 v[47:48], v[12:13], -v[16:17]
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[8:9]
	v_fma_f64 v[25:26], v[16:17], v[25:26], s[8:9]
	v_add_f64 v[35:36], -v[29:30], 1.0
	v_fma_f64 v[27:28], v[10:11], v[27:28], s[0:1]
	v_fma_f64 v[31:32], v[16:17], v[31:32], s[0:1]
	v_add_f64 v[37:38], -v[33:34], 1.0
	s_mov_b32 s0, 0x19f4ec90
	s_mov_b32 s1, 0x3efa01a0
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[6:7]
	v_fma_f64 v[25:26], v[16:17], v[25:26], s[6:7]
	s_mov_b32 s6, 0x11110bb3
	s_mov_b32 s7, 0x3f811111
	v_add_f64 v[43:44], -v[35:36], 1.0
	v_fma_f64 v[27:28], v[10:11], v[27:28], s[0:1]
	v_fma_f64 v[31:32], v[16:17], v[31:32], s[0:1]
	v_add_f64 v[49:50], -v[37:38], 1.0
	s_mov_b32 s0, 0x16c16967
	s_mov_b32 s1, 0xbf56c16c
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[6:7]
	v_fma_f64 v[25:26], v[16:17], v[25:26], s[6:7]
	v_add_f64 v[29:30], v[43:44], -v[29:30]
	v_fma_f64 v[27:28], v[10:11], v[27:28], s[0:1]
	v_fma_f64 v[31:32], v[16:17], v[31:32], s[0:1]
	v_add_f64 v[33:34], v[49:50], -v[33:34]
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s1, 0x3fa55555
	v_fma_f64 v[18:19], v[41:42], v[18:19], v[39:40]
	v_fma_f64 v[25:26], v[47:48], v[25:26], v[45:46]
	v_mul_f64 v[39:40], v[10:11], v[10:11]
	v_fma_f64 v[29:30], v[6:7], -v[8:9], v[29:30]
	v_fma_f64 v[27:28], v[10:11], v[27:28], s[0:1]
	v_fma_f64 v[8:9], v[10:11], v[18:19], -v[8:9]
	v_mul_f64 v[10:11], v[16:17], v[16:17]
	v_fma_f64 v[18:19], v[16:17], v[31:32], s[0:1]
	v_fma_f64 v[31:32], v[12:13], -v[14:15], v[33:34]
	v_fma_f64 v[14:15], v[16:17], v[25:26], -v[14:15]
	s_mov_b32 s1, 0xbfc55555
	v_fma_f64 v[16:17], v[39:40], v[27:28], v[29:30]
	v_fma_f64 v[8:9], v[41:42], s[0:1], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[10:11], v[10:11], v[18:19], v[31:32]
	v_fma_f64 v[14:15], v[47:48], s[0:1], v[14:15]
	v_cmp_class_f64_e64 s0, v[4:5], 0x1f8
	v_add_f64 v[16:17], v[35:36], v[16:17]
	v_lshlrev_b32_e32 v4, 30, v22
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v4, v4, v5
	v_and_b32_e32 v4, 0x80000000, v4
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[8:9], v[37:38], v[10:11]
	v_add_f64 v[10:11], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, v6, v16, vcc_lo
	v_and_b32_e32 v6, 1, v22
	v_cndmask_b32_e64 v0, 0, v0, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_eq_u32_e64 s1, 0, v6
	v_cndmask_b32_e64 v6, v9, v11, s1
	v_cndmask_b32_e64 v5, v8, v10, s1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v6, v6, v4
	v_cndmask_b32_e64 v4, 0, v5, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v5, 0x7ff80000, v6, s0
	v_xor_b32_e32 v6, 0x80000000, v7
	s_waitcnt vmcnt(0)
	v_mul_f64 v[8:9], v[23:24], v[4:5]
	v_mul_f64 v[10:11], v[20:21], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, v6, v17, vcc_lo
	v_xor_b32_e32 v1, v6, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v1, 0x7ff80000, v1, s0
	v_fma_f64 v[6:7], v[20:21], v[0:1], -v[8:9]
	v_fma_f64 v[8:9], v[23:24], v[0:1], v[10:11]
	v_add_co_u32 v10, vcc_lo, s14, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v11, null, s15, v3, vcc_lo
	v_add_co_u32 v12, vcc_lo, s10, v2
	v_add_co_ci_u32_e64 v13, null, s11, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v14, vcc_lo, v10, s4
	v_add_co_ci_u32_e64 v15, null, s5, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v16, vcc_lo, v12, s4
	v_add_co_ci_u32_e64 v17, null, s5, v13, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_clause 0x1
	global_store_b64 v[10:11], v[6:7], off
	global_store_b64 v[14:15], v[8:9], off
	s_clause 0x1
	global_load_b64 v[6:7], v[16:17], off
	global_load_b64 v[8:9], v[12:13], off
	s_waitcnt vmcnt(1)
	v_mul_f64 v[10:11], v[6:7], v[4:5]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[8:9], v[0:1], -v[10:11]
	v_fma_f64 v[0:1], v[6:7], v[0:1], v[4:5]
	v_add_co_u32 v4, vcc_lo, v2, s4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s5, v3, vcc_lo
	s_clause 0x1
	global_store_b64 v[2:3], v[8:9], off
	global_store_b64 v[4:5], v[0:1], off
.LBB0_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel ropex_qk_kernel
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
		.amdhsa_next_free_vgpr 51
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
		.amdhsa_inst_pref_size 46
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
	.size	ropex_qk_kernel, .Lfunc_end0-ropex_qk_kernel
                                        ; -- End function
	.set ropex_qk_kernel.num_vgpr, 51
	.set ropex_qk_kernel.num_agpr, 0
	.set ropex_qk_kernel.numbered_sgpr, 20
	.set ropex_qk_kernel.num_named_barrier, 0
	.set ropex_qk_kernel.private_seg_size, 0
	.set ropex_qk_kernel.uses_vcc, 1
	.set ropex_qk_kernel.uses_flat_scratch, 0
	.set ropex_qk_kernel.has_dyn_sized_stack, 0
	.set ropex_qk_kernel.has_recursion, 0
	.set ropex_qk_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 5768
; TotalNumSgprs: 22
; NumVgprs: 51
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 6
; NumSGPRsForWavesPerEU: 22
; NumVGPRsForWavesPerEU: 51
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_,"axG",@progbits,_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_,comdat
	.protected	_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_ ; -- Begin function _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_
	.globl	_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_
	.p2align	8
	.type	_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_,@function
_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_: ; @_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_
; %bb.0:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x10
	s_load_b32 s4, s[0:1], 0x3c
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s3, s10
	s_abs_i32 s6, s9
	v_cvt_f32_u32_e32 v1, s3
	s_sub_i32 s5, 0, s3
	s_ashr_i32 s14, s10, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s15, v1
	s_mul_i32 s5, s5, s15
	s_mul_hi_u32 s5, s15, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s15, s15, s5
	s_ashr_i32 s5, s9, 31
	s_mul_hi_u32 s7, s6, s15
	s_xor_b32 s5, s5, s14
	s_mul_i32 s12, s7, s3
	s_sub_i32 s6, s6, s12
	s_add_i32 s12, s7, 1
	s_sub_i32 s13, s6, s3
	s_cmp_ge_u32 s6, s3
	s_cselect_b32 s7, s12, s7
	s_cselect_b32 s6, s13, s6
	s_add_i32 s12, s7, 1
	s_cmp_ge_u32 s6, s3
	s_cselect_b32 s6, s12, s7
	s_and_b32 s4, s4, 0xffff
	s_xor_b32 s6, s6, s5
	v_mad_u64_u32 v[1:2], null, s2, s4, v[0:1]
	s_sub_i32 s13, s6, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshr_b32 s5, s13, 31
	s_add_i32 s2, s13, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s12, s2, 1
	s_mul_i32 s2, s10, s8
	s_mul_i32 s2, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_10
; %bb.1:
	s_abs_i32 s2, s12
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s4, 0, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s4, v0
	s_load_b128 s[4:7], s[0:1], 0x20
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
	v_xor_b32_e32 v3, s12, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v3
	s_waitcnt lgkmcnt(0)
	s_load_b32 s2, s[4:5], 0x0
	s_abs_i32 s5, s11
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_xor_b32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v0, v3
	v_mul_lo_u32 v0, v2, s12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v1, v0
	v_cvt_f64_i32_e32 v[3:4], v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[3:4]
	v_cvt_f32_f64_e32 v1, v[3:4]
	v_cvt_f32_i32_e32 v3, s13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f32 v4, null, v3, v3, v1
	v_div_scale_f32 v7, vcc_lo, v1, v3, v1
	v_rcp_f32_e32 v5, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v6, -v4, v5, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v5, v6, v5
	v_mul_f32_e32 v6, v7, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v8, -v4, v6, v7
	v_fmac_f32_e32 v6, v8, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v4, -v4, v6, v7
	v_div_fmas_f32 v4, v4, v5, v6
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f32_e64 vcc_lo, s2, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v1, v4, v3, v1
	v_cndmask_b32_e32 v1, 1.0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cmp_neq_f32_e32 vcc_lo, 0, v1
	v_cmp_neq_f32_e64 s8, v1, |v1|
	v_cndmask_b32_e64 v5, 1.0, s2, vcc_lo
	s_mov_b32 s2, 0x3e76c4e1
	v_frexp_mant_f32_e64 v3, |v5|
	v_cmp_lt_f32_e64 s11, |v5|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v3
	v_cndmask_b32_e64 v4, 1.0, 2.0, vcc_lo
	v_mul_f32_e32 v3, v3, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f32_e32 v7, -1.0, v3
	v_add_f32_e32 v4, 1.0, v3
	v_rcp_f32_e32 v6, v4
	v_add_f32_e32 v9, -1.0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v3, v3, v9
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v8, v7, v6
	v_mul_f32_e32 v10, v4, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v4, v8, v4, -v10
	v_fmac_f32_e32 v4, v8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, v10, v4
	v_sub_f32_e32 v9, v7, v3
	v_sub_f32_e32 v10, v3, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v7, v7, v9 :: v_dual_sub_f32 v4, v10, v4
	v_sub_f32_e32 v3, v7, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, v4, v3
	v_add_f32_e32 v3, v9, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v3, v6, v3
	v_add_f32_e32 v6, v8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v4, v6, v8
	v_mul_f32_e32 v7, v6, v6
	v_fma_f32 v9, v6, v6, -v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v8, v3, v4
	v_add_f32_e32 v3, v8, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v9, v6, v3
	v_cvt_f64_f32_e64 v[3:4], |v5|
	v_add_f32_e32 v10, v7, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fmaak_f32 v11, s2, v10, 0x3e91f4c4
	s_sub_i32 s2, 0, s5
	v_mul_f32_e32 v14, v6, v10
	v_fmaak_f32 v11, v10, v11, 0x3ecccdef
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v12, v10, v11
	v_sub_f32_e32 v7, v10, v7
	v_sub_f32_e32 v7, v9, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v9, v10, v11, -v12
	v_fmac_f32_e32 v9, v7, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v11, v12, v9
	v_frexp_exp_i32_f64_e32 v3, v[3:4]
	v_sub_f32_e32 v12, v11, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v4, v9, v12
	v_fma_f32 v12, v10, v6, -v14
	v_add_f32_e32 v4, 0x31739010, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_add_f32 v13, 0x3f2aaaaa, v11 :: v_dual_fmac_f32 v12, v10, v8
	v_ldexp_f32 v8, v8, 1
	v_dual_add_f32 v9, 0xbf2aaaaa, v13 :: v_dual_fmac_f32 v12, v7, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v11, v9
	v_dual_add_f32 v4, v4, v9 :: v_dual_add_f32 v9, v14, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v7, v13, v4
	v_subrev_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	v_sub_f32_e32 v10, v13, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_f32_i32_e32 v3, v3
	v_add_f32_e32 v4, v4, v10
	v_sub_f32_e32 v13, v9, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mul_f32 v11, v9, v7 :: v_dual_sub_f32 v12, v12, v13
	v_fma_f32 v10, v9, v7, -v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v10, v9, v4
	v_ldexp_f32 v4, v6, 1
	v_fmac_f32_e32 v10, v12, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v6, v11, v10
	v_sub_f32_e32 v9, v6, v11
	v_mul_f32_e32 v11, 0x3f317218, v3
	v_add_f32_e32 v7, v4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_sub_f32 v4, v7, v4 :: v_dual_sub_f32 v9, v10, v9
	v_fma_f32 v10, 0x3f317218, v3, -v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_sub_f32 v4, v6, v4 :: v_dual_fmamk_f32 v3, v3, 0xb102e308, v10
	v_add_f32_e32 v6, v8, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v4, v6, v4
	v_add_f32_e32 v6, v11, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v8, v7, v4 :: v_dual_sub_f32 v11, v6, v11
	v_sub_f32_e32 v7, v8, v7
	v_add_f32_e32 v9, v6, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v3, v3, v11
	v_sub_f32_e32 v4, v4, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v10, v9, v6
	v_sub_f32_e32 v12, v9, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_sub_f32 v7, v8, v10 :: v_dual_add_f32 v8, v3, v4
	v_sub_nc_u32_e32 v10, 0, v2
	v_sub_f32_e32 v6, v6, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v10, v2, v10
	v_add_f32_e32 v6, v7, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v6, v8, v6
	v_add_f32_e32 v11, v9, v6
	v_sub_f32_e32 v7, v8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v8, v8, v7
	v_sub_f32_e32 v3, v3, v8
	v_mul_hi_u32 v8, v10, s15
	v_dual_sub_f32 v4, v4, v7 :: v_dual_sub_f32 v7, v11, v9
	v_cvt_f32_u32_e32 v9, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v3, v4, v3
	v_sub_f32_e32 v4, v6, v7
	v_mul_lo_u32 v7, v8, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_rcp_iflag_f32_e32 v6, v9
	v_add_nc_u32_e32 v9, 1, v8
	v_add_f32_e32 v3, v3, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v7, v10, v7
	v_subrev_nc_u32_e32 v12, s3, v7
	v_cmp_le_u32_e32 vcc_lo, s3, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v8, v8, v9 :: v_dual_cndmask_b32 v7, v7, v12
	v_ashrrev_i32_e32 v9, 31, v2
	v_cmp_le_u32_e32 vcc_lo, s3, v7
	v_add_f32_e32 v4, v11, v3
	v_cmp_gt_f32_e64 s3, 0, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v10, v4, v11
	v_dual_sub_f32 v3, v3, v10 :: v_dual_add_nc_u32 v10, 1, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v7, v8, v10, vcc_lo
	v_dual_mul_f32 v6, 0x4f7ffffe, v6 :: v_dual_mul_f32 v11, v1, v4
	v_cvt_u32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f32 v4, v1, v4, -v11
	v_cmp_class_f32_e64 vcc_lo, v11, 0x204
	v_mul_lo_u32 v8, s2, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v4, v1, v3
	v_xor_b32_e32 v3, s14, v9
	v_add_f32_e32 v9, v11, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_xor_b32_e32 v7, v7, v3
	v_mul_hi_u32 v8, v6, v8
	v_cndmask_b32_e32 v10, v9, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v3, v7, v3
	v_sub_f32_e32 v9, v9, v11
	v_cmp_eq_f32_e32 vcc_lo, 0x42b17218, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_sub_nc_u32_e32 v12, 0, v3
	v_add_nc_u32_e32 v6, v6, v8
	v_sub_f32_e32 v4, v4, v9
	v_mul_f32_e32 v9, 0.5, v1
	v_cndmask_b32_e64 v7, 0, 0x37000000, vcc_lo
	v_max_i32_e32 v8, v3, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v13, v10, v7
	v_mul_hi_u32 v6, v8, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v12, 0x3fb8aa3b, v13
	v_mul_lo_u32 v6, v6, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v14, 0x3fb8aa3b, v13, -v12
	v_rndne_f32_e32 v15, v12
	v_fmamk_f32 v14, v13, 0x32a5705f, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v12, v12, v15
	v_sub_nc_u32_e32 v6, v8, v6
	v_add_f32_e32 v12, v12, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_subrev_nc_u32_e32 v11, s5, v6
	v_cmp_le_u32_e32 vcc_lo, s5, v6
	v_ashrrev_i32_e32 v14, 31, v3
	v_exp_f32_e32 v8, v12
	v_cvt_i32_f32_e32 v12, v15
	v_cndmask_b32_e32 v6, v6, v11, vcc_lo
	v_cmp_neq_f32_e64 vcc_lo, 0x7f800000, |v10|
	v_trunc_f32_e32 v10, v1
	v_trunc_f32_e32 v11, v9
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v13
	s_delay_alu instid0(TRANS32_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f32 v8, v8, v12
	v_cmp_neq_f32_e64 s2, v11, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v4, v7, v4
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v7, 0x7f800000, v8, vcc_lo
	v_cmp_eq_f32_e32 vcc_lo, v10, v1
	v_subrev_nc_u32_e32 v8, s5, v6
	v_cmp_le_u32_e64 s5, s5, v6
	v_fma_f32 v4, v7, v4, v7
	v_cmp_class_f32_e64 s4, v7, 0x204
	s_and_b32 s2, vcc_lo, s2
	v_cndmask_b32_e64 v6, v6, v8, s5
	v_cndmask_b32_e64 v9, 1.0, v5, s2
	s_xor_b32 s5, s8, s11
	v_cndmask_b32_e64 v4, v4, v7, s4
	v_cndmask_b32_e64 v7, 0x7f800000, 0, s5
	v_cmp_eq_f32_e64 s4, 0, v5
	v_cndmask_b32_e64 v10, 0, v5, s2
	v_cmp_class_f32_e64 s2, v5, 0x204
	v_bfi_b32 v4, 0x7fffffff, v4, v9
	v_xor_b32_e32 v6, v6, v14
	s_xor_b32 s3, s3, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v8, 0x7f800000, 0, s3
	v_cndmask_b32_e32 v9, 0x7fc00000, v4, vcc_lo
	v_cmp_neq_f32_e64 vcc_lo, |v5|, 1.0
	v_sub_nc_u32_e32 v6, v6, v14
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_bfi_b32 v8, 0x7fffffff, v8, v10
	v_cndmask_b32_e32 v7, 1.0, v7, vcc_lo
	v_cmp_gt_f32_e32 vcc_lo, 0, v5
	v_cvt_f32_i32_e32 v6, v6
	v_cndmask_b32_e32 v4, v4, v9, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v4, v4, v7, vcc_lo
	s_or_b32 vcc_lo, s4, s2
	v_cndmask_b32_e32 v4, v4, v8, vcc_lo
	v_cmp_o_f32_e32 vcc_lo, v5, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, 0x7fc00000, v4, vcc_lo
	v_div_scale_f32 v4, null, v1, v1, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v5, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v7, -v4, v5, 1.0
	v_fmac_f32_e32 v5, v7, v5
	v_div_scale_f32 v7, vcc_lo, v6, v1, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v8, v7, v5
	v_fma_f32 v9, -v4, v8, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v8, v9, v5
	v_fma_f32 v4, -v4, v8, v7
                                        ; implicit-def: $vgpr7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v4, v4, v5, v8
	v_div_fixup_f32 v4, v4, v1, v6
                                        ; implicit-def: $vgpr6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_and_b32_e32 v5, 0x7fffffff, v4
	v_cmp_ngt_f32_e64 s4, 0x48000000, |v4|
	v_lshrrev_b32_e32 v1, 23, v5
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s5, exec_lo, s2
	s_cbranch_execz .LBB1_3
; %bb.2:
	s_mov_b32 s2, 0x7fffff
	v_mov_b32_e32 v8, 0
	v_and_or_b32 v16, v5, s2, 0x800000
	v_add_nc_u32_e32 v14, 0xffffff88, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[6:7], null, 0xfe5163ab, v16, 0
	v_cmp_lt_u32_e32 vcc_lo, 63, v14
	v_cndmask_b32_e64 v15, 0, 0xffffffc0, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[9:10], null, 0x3c439041, v16, v[7:8]
	v_add_nc_u32_e32 v15, v15, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v7, v10
	v_cmp_lt_u32_e64 s2, 31, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[10:11], null, 0xdb629599, v16, v[7:8]
	v_cndmask_b32_e64 v17, 0, 0xffffffe0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v6, v10, v6 :: v_dual_add_nc_u32 v17, v17, v15
	v_mov_b32_e32 v7, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s3, 31, v17
	v_mad_u64_u32 v[11:12], null, 0xf534ddc0, v16, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v7, v12
	v_cndmask_b32_e32 v9, v11, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[12:13], null, 0xfc2757d1, v16, v[7:8]
	v_cndmask_b32_e64 v6, v9, v6, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v7, v13
	v_mad_u64_u32 v[13:14], null, 0x4e441529, v16, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v7, v14
	v_mad_u64_u32 v[14:15], null, 0xa2f9836e, v16, v[7:8]
	v_cndmask_b32_e64 v7, 0, 0xffffffe0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v8, v13, v11 :: v_dual_add_nc_u32 v7, v7, v17
	v_dual_cndmask_b32 v14, v14, v12 :: v_dual_cndmask_b32 v13, v15, v13
	v_cndmask_b32_e32 v12, v12, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_eq_u32_e32 vcc_lo, 0, v7
	v_cndmask_b32_e64 v11, v14, v8, s2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v13, v13, v14, s2
	v_cndmask_b32_e64 v8, v8, v12, s2
	v_sub_nc_u32_e32 v14, 32, v7
	v_cndmask_b32_e64 v12, v12, v9, s2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v13, v13, v11, s3
	v_cndmask_b32_e64 v11, v11, v8, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v8, v8, v12, s3
	v_cndmask_b32_e64 v6, v12, v6, s3
	v_alignbit_b32 v15, v13, v11, v14.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_alignbit_b32 v10, v11, v8, v14.l
	v_cndmask_b32_e32 v7, v15, v13, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v13, v8, v6, v14.l
	v_cndmask_b32_e32 v9, v10, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_bfe_u32 v10, v7, 29, 1
	v_cndmask_b32_e32 v8, v13, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v11, v7, v9, 30
	v_sub_nc_u32_e32 v12, 0, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_alignbit_b32 v6, v8, v6, 30
	v_xor_b32_e32 v11, v11, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v6, v6, v12
	v_clz_i32_u32_e32 v13, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_min_u32_e32 v13, 32, v13
	v_lshlrev_b32_e32 v14, 23, v13
	v_alignbit_b32 v9, v9, v8, 30
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_xor_b32_e32 v8, v9, v12
	v_sub_nc_u32_e32 v9, 31, v13
	v_lshrrev_b32_e32 v12, 29, v7
	v_lshrrev_b32_e32 v7, 30, v7
	v_alignbit_b32 v11, v11, v8, v9.l
	v_alignbit_b32 v6, v8, v6, v9.l
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_lshlrev_b32_e32 v8, 31, v12
	v_add_nc_u32_e32 v7, v10, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v9, v11, v6, 9
	v_or_b32_e32 v12, 0.5, v8
	v_lshrrev_b32_e32 v11, 9, v11
	v_or_b32_e32 v8, 0x33000000, v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_clz_i32_u32_e32 v15, v9
	v_sub_nc_u32_e32 v12, v12, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_min_u32_e32 v14, 32, v15
	v_or_b32_e32 v11, v11, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_not_b32_e32 v12, v14
	v_mul_f32_e32 v15, 0x3fc90fda, v11
	v_add_lshl_u32 v13, v14, v13, 23
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v6, v9, v6, v12.l
	v_fma_f32 v9, 0x3fc90fda, v11, -v15
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v8, v8, v13
	v_lshrrev_b32_e32 v6, 9, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmamk_f32 v9, v11, 0x33a22168, v9
	v_or_b32_e32 v6, v8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v9, 0x3fc90fda, v6
	v_add_f32_e32 v6, v15, v9
	s_or_saveexec_b32 s2, s5
	v_mul_f32_e64 v10, 0x3f22f983, |v4|
	s_xor_b32 exec_lo, exec_lo, s2
	s_branch .LBB1_4
.LBB1_3:
	s_or_saveexec_b32 s2, s5
	v_mul_f32_e64 v10, 0x3f22f983, |v4|
	s_xor_b32 exec_lo, exec_lo, s2
.LBB1_4:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f32_e32 v7, v10
	v_fma_f32 v6, 0xbfc90fda, v7, |v4|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmamk_f32 v6, v7, 0xb3a22168, v6
	v_fmamk_f32 v6, v7, 0xa7c234c4, v6
	v_cvt_i32_f32_e32 v7, v7
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s2
                                        ; implicit-def: $vgpr9
                                        ; implicit-def: $vgpr8
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB1_7
; %bb.6:
	s_mov_b32 s2, 0x7fffff
	v_mov_b32_e32 v10, 0
	v_and_or_b32 v18, v5, s2, 0x800000
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[8:9], null, 0xfe5163ab, v18, 0
	v_mad_u64_u32 v[11:12], null, 0x3c439041, v18, v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v9, v12
	v_mad_u64_u32 v[12:13], null, 0xdb629599, v18, v[9:10]
	v_add_nc_u32_e32 v1, 0xffffff88, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v1
	v_mov_b32_e32 v9, v13
	v_cndmask_b32_e64 v16, 0, 0xffffffc0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[13:14], null, 0xf534ddc0, v18, v[9:10]
	v_cndmask_b32_e32 v8, v12, v8, vcc_lo
	v_add_nc_u32_e32 v1, v16, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v9, v14
	v_cmp_lt_u32_e64 s2, 31, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[14:15], null, 0xfc2757d1, v18, v[9:10]
	v_cndmask_b32_e64 v17, 0, 0xffffffe0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v1, v17, v1
	v_mov_b32_e32 v9, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s3, 31, v1
	v_mad_u64_u32 v[15:16], null, 0x4e441529, v18, v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v9, v16
	v_mad_u64_u32 v[16:17], null, 0xa2f9836e, v18, v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v9, 0, 0xffffffe0, s3
	v_cndmask_b32_e32 v10, v15, v13, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_dual_cndmask_b32 v16, v16, v14 :: v_dual_add_nc_u32 v1, v9, v1
	v_dual_cndmask_b32 v15, v17, v15 :: v_dual_cndmask_b32 v14, v14, v12
	v_cndmask_b32_e32 v9, v13, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_eq_u32_e32 vcc_lo, 0, v1
	v_cndmask_b32_e64 v11, v16, v10, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v13, v15, v16, s2
	v_cndmask_b32_e64 v10, v10, v14, s2
	v_sub_nc_u32_e32 v15, 32, v1
	v_cndmask_b32_e64 v14, v14, v9, s2
	v_cndmask_b32_e64 v8, v9, v8, s2
	v_cndmask_b32_e64 v13, v13, v11, s3
	v_cndmask_b32_e64 v11, v11, v10, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v10, v10, v14, s3
	v_cndmask_b32_e64 v8, v14, v8, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v16, v13, v11, v15.l
	v_alignbit_b32 v12, v11, v10, v15.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v14, v10, v8, v15.l
	v_cndmask_b32_e32 v1, v16, v13, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v9, v12, v11 :: v_dual_cndmask_b32 v10, v14, v10
	v_bfe_u32 v11, v1, 29, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v12, v1, v9, 30
	v_alignbit_b32 v9, v9, v10, 30
	v_alignbit_b32 v8, v10, v8, 30
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v13, 0, v11
	v_xor_b32_e32 v12, v12, v13
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v9, v9, v13
	v_xor_b32_e32 v8, v8, v13
	v_lshrrev_b32_e32 v13, 29, v1
	v_lshrrev_b32_e32 v1, 30, v1
	v_clz_i32_u32_e32 v14, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_min_u32_e32 v14, 32, v14
	v_sub_nc_u32_e32 v10, 31, v14
	v_lshlrev_b32_e32 v15, 23, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_alignbit_b32 v12, v12, v9, v10.l
	v_alignbit_b32 v8, v9, v8, v10.l
	v_lshlrev_b32_e32 v9, 31, v13
	v_alignbit_b32 v10, v12, v8, 9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_or_b32_e32 v13, 0.5, v9
	v_lshrrev_b32_e32 v12, 9, v12
	v_or_b32_e32 v9, 0x33000000, v9
	v_clz_i32_u32_e32 v16, v10
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v13, v13, v15
	v_min_u32_e32 v15, 32, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_or_b32_e32 v12, v12, v13
	v_not_b32_e32 v13, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v16, 0x3fc90fda, v12
	v_add_lshl_u32 v14, v15, v14, 23
	v_alignbit_b32 v8, v10, v8, v13.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f32 v10, 0x3fc90fda, v12, -v16
	v_sub_nc_u32_e32 v9, v9, v14
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshrrev_b32_e32 v8, 9, v8
	v_fmamk_f32 v10, v12, 0x33a22168, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_or_b32_e32 v8, v9, v8
	v_dual_fmac_f32 v10, 0x3fc90fda, v8 :: v_dual_add_nc_u32 v9, v11, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v8, v16, v10
                                        ; implicit-def: $vgpr10
	s_or_saveexec_b32 s2, s4
	s_load_b32 s4, s[6:7], 0x0
	s_xor_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB1_8
	s_branch .LBB1_9
.LBB1_7:
	s_or_saveexec_b32 s2, s4
	s_load_b32 s4, s[6:7], 0x0
	s_xor_b32 exec_lo, exec_lo, s2
.LBB1_8:
	v_rndne_f32_e32 v1, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v8, 0xbfc90fda, v1, |v4|
	v_cvt_i32_f32_e32 v9, v1
	v_fmamk_f32 v8, v1, 0xb3a22168, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_fmamk_f32 v8, v1, 0xa7c234c4, v8
.LBB1_9:
	s_or_b32 exec_lo, exec_lo, s2
	v_mul_lo_u32 v10, v3, s10
	v_ashrrev_i32_e32 v1, 31, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	v_dual_mul_f32 v14, v6, v6 :: v_dual_and_b32 v15, 1, v7
	s_mov_b32 s5, 0xb94c1982
	v_dual_mul_f32 v16, v8, v8 :: v_dual_lshlrev_b32 v7, 30, v7
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v2, v2, v10
	v_dual_fmaak_f32 v18, s5, v14, 0x3c0881c4 :: v_dual_and_b32 v17, 1, v9
	s_mov_b32 s6, 0x37d75334
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_fmaak_f32 v20, s5, v16, 0x3c0881c4 :: v_dual_lshlrev_b32 v9, 30, v9
	v_mad_i64_i32 v[10:11], null, v2, s13, v[0:1]
	s_ashr_i32 s13, s12, 31
	v_fmaak_f32 v18, v14, v18, 0xbe2aaa9d
	v_fmaak_f32 v21, s6, v16, 0xbab64f3b
	v_fmaak_f32 v20, v16, v20, 0xbe2aaa9d
	v_fmaak_f32 v19, s6, v14, 0xbab64f3b
	v_xor_b32_e32 v5, v5, v4
	v_mad_i64_i32 v[0:1], null, v3, s9, v[10:11]
	v_dual_mul_f32 v18, v14, v18 :: v_dual_fmaak_f32 v21, v16, v21, 0x3d2aabf7
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_mul_f32 v20, v16, v20 :: v_dual_fmaak_f32 v19, v14, v19, 0x3d2aabf7
	v_fmac_f32_e32 v6, v6, v18
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	v_fmac_f32_e32 v8, v8, v20
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[12:13], 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v10, vcc_lo, v2, s0
	v_add_co_ci_u32_e64 v11, null, s1, v3, vcc_lo
	s_clause 0x1
	global_load_b32 v12, v[2:3], off
	global_load_b32 v13, v[10:11], off
	v_fmaak_f32 v21, v16, v21, 0xbf000004
	v_cmp_eq_u32_e32 vcc_lo, 0, v15
	v_and_b32_e32 v7, 0x80000000, v7
	v_and_b32_e32 v9, 0x80000000, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f32 v16, v16, v21, 1.0
	v_fmaak_f32 v19, v14, v19, 0xbf000004
	v_fma_f32 v14, v14, v19, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v6, -v6, v14, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, 0, v17
	v_xor_b32_e32 v6, v7, v6
	v_cndmask_b32_e32 v8, v16, v8, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v4, 0x1f8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor3_b32 v5, v5, v9, v8
	v_cndmask_b32_e32 v6, 0x7fc00000, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v4, 0x7fc00000, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v7, s4, v4
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v8, v12, v7
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v4, v13, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v8, v6, v13
	v_fma_f32 v9, v6, v12, -v4
	v_add_co_u32 v4, vcc_lo, v0, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s1, v1, vcc_lo
	s_clause 0x1
	global_store_b32 v[2:3], v9, off
	global_store_b32 v[10:11], v8, off
	s_clause 0x1
	global_load_b32 v2, v[4:5], off
	global_load_b32 v3, v[0:1], off
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v8, v2, v7
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v7, v3, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v3, v6, v3, -v8
	v_fmac_f32_e32 v7, v6, v2
	s_clause 0x1
	global_store_b32 v[0:1], v3, off
	global_store_b32 v[4:5], v7, off
.LBB1_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_
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
		.amdhsa_inst_pref_size 33
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_,"axG",@progbits,_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_,comdat
.Lfunc_end1:
	.size	_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_, .Lfunc_end1-_Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_
                                        ; -- End function
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.num_vgpr, 22
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.num_agpr, 0
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.numbered_sgpr, 16
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.num_named_barrier, 0
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.private_seg_size, 0
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.uses_vcc, 1
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.uses_flat_scratch, 0
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.has_dyn_sized_stack, 0
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.has_recursion, 0
	.set _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4140
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
	.section	.text._Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_,"axG",@progbits,_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_,comdat
	.protected	_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_ ; -- Begin function _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_
	.globl	_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_
	.p2align	8
	.type	_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_,@function
_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_: ; @_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_
; %bb.0:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x10
	s_load_b32 s3, s[0:1], 0x3c
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s14, s10
	s_abs_i32 s5, s9
	v_cvt_f32_u32_e32 v1, s14
	s_sub_i32 s4, 0, s14
	s_ashr_i32 s15, s10, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s16, v1
	s_mul_i32 s4, s4, s16
	s_mul_hi_u32 s4, s16, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s16, s16, s4
	s_ashr_i32 s4, s9, 31
	s_mul_hi_u32 s6, s5, s16
	s_xor_b32 s4, s4, s15
	s_mul_i32 s7, s6, s14
	s_sub_i32 s5, s5, s7
	s_add_i32 s7, s6, 1
	s_sub_i32 s12, s5, s14
	s_cmp_ge_u32 s5, s14
	s_cselect_b32 s6, s7, s6
	s_cselect_b32 s5, s12, s5
	s_add_i32 s7, s6, 1
	s_cmp_ge_u32 s5, s14
	s_cselect_b32 s5, s7, s6
	s_and_b32 s3, s3, 0xffff
	s_xor_b32 s5, s5, s4
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_sub_i32 s13, s5, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshr_b32 s4, s13, 31
	s_add_i32 s2, s13, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s12, s2, 1
	s_mul_i32 s2, s10, s8
	s_mul_i32 s2, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB2_10
; %bb.1:
	s_abs_i32 s2, s12
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b128 s[4:7], s[0:1], 0x20
	s_mov_b32 s18, 0x4222de17
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_mov_b32 s19, 0x3fbdee67
	s_abs_i32 s8, s11
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
	v_xor_b32_e32 v3, s12, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[4:5], 0x0
	s_mov_b32 s4, 0x968915a9
	s_mov_b32 s5, 0x3fba6564
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v20, v0, v3
	v_cvt_f64_i32_e32 v[3:4], s13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v20, s12
	v_sub_nc_u32_e32 v0, v1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_i32_e32 v[1:2], v0
	v_add_f64 v[1:2], v[1:2], v[1:2]
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
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f64_e64 vcc_lo, s[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_div_fixup_f64 v[1:2], v[5:6], v[3:4], v[1:2]
	v_mov_b32_e32 v3, s3
	s_mov_b32 s3, 0x3fe55555
	v_cndmask_b32_e32 v2, 0x3ff00000, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, 0, v1, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[1:2]
	v_cmp_neq_f64_e64 s11, v[1:2], |v[1:2]|
	v_cndmask_b32_e32 v4, 0x3ff00000, v3, vcc_lo
	v_cndmask_b32_e64 v3, 0, s2, vcc_lo
	s_mov_b32 s2, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[5:6], |v[3:4]|
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[5:6]
	v_cndmask_b32_e64 v7, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v7
	v_add_f64 v[7:8], v[5:6], 1.0
	v_add_f64 v[13:14], v[5:6], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[9:10], v[7:8]
	v_add_f64 v[15:16], v[7:8], -1.0
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[7:8], v[11:12]
	v_fma_f64 v[7:8], v[11:12], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[11:12], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[17:18], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[13:14], -v[7:8]
	v_add_f64 v[17:18], v[7:8], -v[17:18]
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[17:18], -v[5:6]
	v_add_f64 v[7:8], v[13:14], -v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[5:6], v[15:16], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[5:6], v[9:10], v[5:6]
	v_add_f64 v[7:8], v[11:12], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[7:8], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[7:8], v[7:8], -v[11:12]
	v_add_f64 v[13:14], v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	v_add_f64 v[13:14], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], s[18:19], s[4:5]
	s_mov_b32 s4, 0x3abe935a
	s_mov_b32 s5, 0x3fbe25e4
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_mul_f64 v[23:24], v[7:8], v[13:14]
	s_mov_b32 s18, 0x652b82fe
	s_mov_b32 s19, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s4, 0x47e6c9c2
	s_mov_b32 s5, 0x3fc110ef
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s4, 0xcfa74449
	s_mov_b32 s5, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s4, 0x71bf3c30
	s_mov_b32 s5, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s4, 0x1c7792ce
	s_mov_b32 s5, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s4, 0x924920da
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s4, 0x9999999c
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s5, 0x3c7abc9e
	s_mov_b32 s4, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[11:12], v[13:14], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	v_add_f64 v[15:16], v[17:18], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[21:22], v[15:16], s[2:3]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_mov_b32 s3, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[25:26], v[21:22], s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_fma_f64 v[17:18], v[13:14], v[7:8], -v[23:24]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_add_f64 v[15:16], v[15:16], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], s[2:3]
	v_fma_f64 v[13:14], v[13:14], v[5:6], v[17:18]
	s_mov_b32 s3, 0x3fe62e42
	s_mov_b32 s2, 0xfefa39ef
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[7:8], v[13:14]
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[21:22], v[11:12]
	v_add_f64 v[15:16], v[23:24], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[21:22], -v[13:14]
	v_mul_f64 v[21:22], v[15:16], v[13:14]
	v_add_f64 v[23:24], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[17:18]
	v_fma_f64 v[17:18], v[15:16], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[23:24]
	v_fma_f64 v[11:12], v[15:16], v[11:12], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[9:10], v[13:14], v[11:12]
	v_frexp_exp_i32_f64_e32 v13, v[3:4]
	v_add_f64 v[11:12], v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v13, null, 0, v13, vcc_lo
	v_cvt_f64_i32_e32 v[13:14], v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[7:8], v[11:12]
	v_add_f64 v[17:18], v[11:12], -v[21:22]
	v_mul_f64 v[21:22], v[13:14], s[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[15:16], -v[7:8]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], s[2:3], -v[21:22]
	s_mov_b32 s3, 0xbfe62e42
	v_add_f64 v[7:8], v[11:12], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_fma_f64 v[9:10], v[13:14], s[4:5], v[17:18]
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], v[5:6]
	v_add_f64 v[21:22], v[7:8], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[7:8], v[11:12]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[21:22]
                                        ; implicit-def: $vgpr22
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[15:16], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[23:24]
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
	v_mul_f64 v[11:12], v[1:2], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[1:2], v[7:8], -v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	v_fma_f64 v[5:6], v[1:2], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_dual_cndmask_b32 v10, v8, v12 :: v_dual_cndmask_b32 v9, v7, v11
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[13:14], v[9:10], s[18:19]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[9:10]|
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_trunc_f64_e32 v[7:8], v[1:2]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rndne_f64_e32 v[13:14], v[13:14]
	v_dual_cndmask_b32 v6, 0, v6 :: v_dual_cndmask_b32 v5, 0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[13:14], s[2:3], v[9:10]
	s_mov_b32 s2, 0xfca7ab0c
	s_mov_b32 s3, 0x3e928af3
	v_cvt_i32_f64_e32 v19, v[13:14]
	v_fma_f64 v[15:16], v[13:14], s[4:5], v[15:16]
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], s[4:5], s[2:3]
	s_mov_b32 s2, 0x623fde64
	s_mov_b32 s3, 0x3ec71dee
	s_sub_i32 s5, 0, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x7c89e6b0
	s_mov_b32 s3, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x14761f6e
	s_mov_b32 s3, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x11122322
	s_mov_b32 s3, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x555502a1
	s_mov_b32 s3, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x55555511
	s_mov_b32 s3, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 11
	s_mov_b32 s3, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	v_cmp_nlt_f64_e64 s2, 0x40900000, v[9:10]
	v_cmp_ngt_f64_e64 s3, 0xc090cc00, v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	s_and_b32 vcc_lo, s3, s2
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[11:12], v[13:14], v19
	v_mul_f64 v[13:14], v[1:2], 0.5
	v_cndmask_b32_e64 v12, 0x7ff00000, v12, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[9:10], v[13:14]
	v_cndmask_b32_e32 v11, 0, v11, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[7:8], v[1:2]
	v_sub_nc_u32_e32 v7, 0, v20
	v_cndmask_b32_e64 v12, 0, v12, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v7, v20, v7
	v_fma_f64 v[5:6], v[11:12], v[5:6], v[11:12]
	v_cmp_class_f64_e64 s3, v[11:12], 0x204
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_hi_u32 v8, v7, s16
	v_cmp_lt_f64_e64 s16, |v[3:4]|, 1.0
	v_cmp_neq_f64_e64 s2, v[9:10], v[13:14]
	v_mul_lo_u32 v10, v8, s14
	v_cvt_f32_u32_e32 v9, s8
	v_ashrrev_i32_e32 v13, 31, v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rcp_iflag_f32_e32 v9, v9
	v_sub_nc_u32_e32 v7, v7, v10
	v_add_nc_u32_e32 v10, 1, v8
	v_cndmask_b32_e64 v6, v6, v12, s3
	v_cndmask_b32_e64 v5, v5, v11, s3
	v_xor_b32_e32 v11, s15, v13
	v_cmp_le_u32_e64 s4, s14, v7
	s_delay_alu instid0(TRANS32_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v9, 0x4f7ffffe, v9
	v_cndmask_b32_e64 v8, v8, v10, s4
	v_subrev_nc_u32_e32 v10, s14, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v9, v9
	v_cndmask_b32_e64 v7, v7, v10, s4
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v10, 1, v8
	v_mul_lo_u32 v12, s5, v9
	s_xor_b32 s5, s11, s16
	v_cmp_eq_f64_e64 s4, 0, v[3:4]
	v_cmp_le_u32_e64 s3, s14, v7
	v_cmp_class_f64_e64 s11, v[3:4], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v7, v8, v10, s3
	v_cndmask_b32_e32 v10, 0, v5, vcc_lo
	v_cmp_gt_f64_e64 s3, 0, v[1:2]
	s_and_b32 s2, vcc_lo, s2
	v_cndmask_b32_e64 v14, 0x3ff00000, v4, s2
	v_xor_b32_e32 v7, v7, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_bfi_b32 v6, 0x7fffffff, v6, v14
	v_sub_nc_u32_e32 v21, v7, v11
	v_mul_hi_u32 v7, v9, v12
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v8, 0x7ff80000, v6, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[3:4]
	v_sub_nc_u32_e32 v11, 0, v21
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v7, v9, v7
	v_max_i32_e32 v9, v21, v11
	v_cndmask_b32_e64 v11, 0x7ff00000, 0, s5
	v_cmp_neq_f64_e64 s5, |v[3:4]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v7, v9, v7
	v_mul_lo_u32 v7, v7, s8
	s_xor_b32 s3, s3, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_sub_nc_u32_e32 v7, v9, v7
	v_ashrrev_i32_e32 v9, 31, v21
	v_dual_cndmask_b32 v5, v5, v10 :: v_dual_cndmask_b32 v6, v6, v8
	v_cmp_class_f64_e64 vcc_lo, v[1:2], 0x204
	v_subrev_nc_u32_e32 v8, s8, v7
	v_cndmask_b32_e64 v10, 0, v4, s2
	s_or_b32 s2, s4, s11
	v_cndmask_b32_e64 v11, 0x3ff00000, v11, s5
	v_cmp_le_u32_e64 s5, s8, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, v7, v8, s5
	v_subrev_nc_u32_e32 v8, s8, v7
	v_cmp_le_u32_e64 s5, s8, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, v8, s5
	v_cndmask_b32_e64 v8, 0x7ff00000, 0, s3
	v_xor_b32_e32 v7, v7, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_bfi_b32 v8, 0x7fffffff, v8, v10
	v_sub_nc_u32_e32 v7, v7, v9
	v_cndmask_b32_e32 v6, v6, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v6, v6, v8, s2
	s_or_b32 s2, s2, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[3:4], v[1:2]
	v_cvt_f64_i32_e32 v[1:2], v7
	v_cndmask_b32_e64 v5, v5, 0, s2
	v_cndmask_b32_e32 v3, 0, v5, vcc_lo
	v_cndmask_b32_e32 v4, 0x7ff80000, v6, vcc_lo
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
	v_div_fixup_f64 v[2:3], v[5:6], v[3:4], v[1:2]
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr6_vgpr7
	v_cmp_ngt_f64_e64 s2, 0x41d00000, |v[2:3]|
	v_trig_preop_f64 v[16:17], |v[2:3]|, 0
	v_trig_preop_f64 v[14:15], |v[2:3]|, 1
	v_ldexp_f64 v[18:19], |v[2:3]|, 0xffffff80
	v_trig_preop_f64 v[8:9], |v[2:3]|, 2
	v_and_b32_e32 v1, 0x7fffffff, v3
	s_and_saveexec_b32 s3, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s3, exec_lo, s3
	s_cbranch_execz .LBB2_3
; %bb.2:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_mov_b32_e32 v34, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s14, 0x33145c07
	s_mov_b32 s15, 0x3c91a626
	v_dual_cndmask_b32 v5, v1, v19 :: v_dual_cndmask_b32 v4, v2, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[6:7], v[16:17], v[4:5]
	v_mul_f64 v[10:11], v[14:15], v[4:5]
	v_mul_f64 v[30:31], v[8:9], v[4:5]
	v_fma_f64 v[12:13], v[16:17], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[32:33], v[14:15], v[4:5], -v[10:11]
	v_fma_f64 v[4:5], v[8:9], v[4:5], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[10:11], v[12:13]
	v_add_f64 v[24:25], v[22:23], -v[10:11]
	v_add_f64 v[28:29], v[6:7], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[22:23], -v[24:25]
	v_add_f64 v[12:13], v[12:13], -v[24:25]
	v_ldexp_f64 v[24:25], v[28:29], -2
	v_add_f64 v[6:7], v[28:29], -v[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[10:11], -v[26:27]
	v_add_f64 v[26:27], v[30:31], v[32:33]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[24:25]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[6:7], v[22:23], -v[6:7]
	v_add_f64 v[10:11], v[12:13], v[10:11]
	v_fract_f64_e32 v[12:13], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[26:27], v[10:11]
	v_ldexp_f64 v[12:13], v[12:13], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[6:7], v[22:23]
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[28:29], v[24:25], v[12:13]
	v_add_f64 v[6:7], v[24:25], -v[6:7]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[28:29]
	v_add_f64 v[28:29], v[26:27], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[22:23], -v[6:7]
	v_cndmask_b32_e64 v35, 0, 0x40100000, vcc_lo
	v_add_f64 v[39:40], v[26:27], -v[28:29]
	v_add_f64 v[28:29], v[32:33], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], v[34:35]
	v_add_f64 v[35:36], v[22:23], -v[26:27]
	v_add_f64 v[32:33], v[30:31], -v[39:40]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[37:38], v[24:25], v[12:13]
	v_add_f64 v[41:42], v[22:23], -v[35:36]
	v_add_f64 v[10:11], v[10:11], -v[35:36]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[28:29], v[28:29], v[32:33]
	v_cvt_i32_f64_e32 v37, v[37:38]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[26:27], -v[41:42]
	v_cvt_f64_i32_e32 v[35:36], v37
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[26:27]
	v_add_f64 v[12:13], v[12:13], -v[35:36]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[28:29], v[10:11]
	v_add_f64 v[26:27], v[24:25], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[10:11]
	v_add_f64 v[10:11], v[26:27], -v[12:13]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[6:7], v[4:5]
	v_add_f64 v[6:7], v[24:25], -v[10:11]
	v_cndmask_b32_e64 v35, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v22, null, 0, v37, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[26:27], -v[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[6:7], v[4:5]
	v_mul_f64 v[12:13], v[10:11], s[4:5]
	v_add_f64 v[6:7], v[10:11], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[23:24], v[10:11], s[4:5], -v[12:13]
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[10:11], s[14:15], v[23:24]
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[12:13], v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB2_5
	s_branch .LBB2_4
.LBB2_3:
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB2_5
.LBB2_4:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s15, 0xbc91a626
	v_mul_f64 v[4:5], |v[2:3]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s14, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[4:5]
	v_fma_f64 v[4:5], v[10:11], s[4:5], |v[2:3]|
	v_mul_f64 v[6:7], v[10:11], s[14:15]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[10:11], s[14:15], v[4:5]
	v_add_f64 v[12:13], v[4:5], v[6:7]
	s_mov_b32 s15, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_fma_f64 v[6:7], v[10:11], s[14:15], v[6:7]
	v_add_f64 v[4:5], v[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_fma_f64 v[6:7], v[10:11], s[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[22:23], v[6:7]
	v_add_f64 v[12:13], v[4:5], -v[22:23]
	v_cvt_i32_f64_e32 v22, v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[12:13]
.LBB2_5:
	s_or_b32 exec_lo, exec_lo, s3
                                        ; implicit-def: $vgpr23
                                        ; implicit-def: $vgpr10_vgpr11
                                        ; implicit-def: $vgpr12_vgpr13
	s_and_saveexec_b32 s3, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s2, exec_lo, s3
	s_cbranch_execz .LBB2_7
; %bb.6:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_mov_b32_e32 v33, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s14, 0x33145c07
	s_mov_b32 s15, 0x3c91a626
	v_dual_cndmask_b32 v11, v1, v19 :: v_dual_cndmask_b32 v10, v2, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[12:13], v[16:17], v[10:11]
	v_mul_f64 v[18:19], v[14:15], v[10:11]
	v_mul_f64 v[31:32], v[8:9], v[10:11]
	v_fma_f64 v[16:17], v[16:17], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[14:15], v[10:11], -v[18:19]
	v_fma_f64 v[8:9], v[8:9], v[10:11], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[23:24], v[18:19], v[16:17]
	v_add_f64 v[25:26], v[23:24], -v[18:19]
	v_add_f64 v[29:30], v[12:13], v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[27:28], v[23:24], -v[25:26]
	v_add_f64 v[16:17], v[16:17], -v[25:26]
	v_ldexp_f64 v[25:26], v[29:30], -2
	v_add_f64 v[12:13], v[29:30], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[18:19], -v[27:28]
	v_add_f64 v[27:28], v[31:32], v[14:15]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[25:26]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[23:24], -v[12:13]
	v_add_f64 v[16:17], v[16:17], v[18:19]
	v_fract_f64_e32 v[18:19], v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[23:24], v[27:28], v[16:17]
	v_ldexp_f64 v[18:19], v[18:19], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[12:13], v[23:24]
	v_dual_cndmask_b32 v19, 0, v19 :: v_dual_cndmask_b32 v18, 0, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[29:30], v[25:26], v[18:19]
	v_add_f64 v[10:11], v[25:26], -v[12:13]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[29:30]
	v_add_f64 v[29:30], v[27:28], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[23:24], -v[10:11]
	v_cndmask_b32_e64 v34, 0, 0x40100000, vcc_lo
	v_add_f64 v[38:39], v[27:28], -v[29:30]
	v_add_f64 v[14:15], v[14:15], -v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[18:19], v[33:34]
	v_add_f64 v[34:35], v[23:24], -v[27:28]
	v_add_f64 v[29:30], v[31:32], -v[38:39]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[36:37], v[25:26], v[18:19]
	v_add_f64 v[40:41], v[23:24], -v[34:35]
	v_add_f64 v[16:17], v[16:17], -v[34:35]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[14:15], v[29:30]
	v_cvt_i32_f64_e32 v1, v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[27:28], v[27:28], -v[40:41]
	v_cvt_f64_i32_e32 v[34:35], v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], v[27:28]
	v_add_f64 v[18:19], v[18:19], -v[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[14:15], v[16:17]
	v_add_f64 v[14:15], v[25:26], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[12:13], v[14:15], -v[18:19]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[10:11], v[8:9]
	v_add_f64 v[10:11], v[25:26], -v[12:13]
	v_cndmask_b32_e64 v34, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v23, null, 0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[14:15], -v[33:34]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[10:11], v[8:9]
	v_mul_f64 v[14:15], v[12:13], s[4:5]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[12:13], s[4:5], -v[14:15]
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[12:13], s[14:15], v[16:17]
	v_fma_f64 v[8:9], v[8:9], s[4:5], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[14:15], v[8:9]
	v_add_f64 v[12:13], v[10:11], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[12:13], v[8:9], -v[12:13]
	s_or_saveexec_b32 s2, s2
	s_load_b64 s[4:5], s[6:7], 0x0
	s_xor_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB2_8
	s_branch .LBB2_9
.LBB2_7:
	s_or_saveexec_b32 s2, s2
	s_load_b64 s[4:5], s[6:7], 0x0
	s_xor_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB2_9
.LBB2_8:
	s_mov_b32 s6, 0x6dc9c883
	s_mov_b32 s7, 0x3fe45f30
	s_mov_b32 s15, 0xbc91a626
	v_mul_f64 v[8:9], |v[2:3]|, s[6:7]
	s_mov_b32 s6, 0x54442d18
	s_mov_b32 s7, 0xbff921fb
	s_mov_b32 s14, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[8:9]
	v_fma_f64 v[10:11], v[8:9], s[6:7], |v[2:3]|
	v_mul_f64 v[12:13], v[8:9], s[14:15]
	s_mov_b32 s6, 0x252049c0
	s_mov_b32 s7, 0xb97b839a
	v_cvt_i32_f64_e32 v23, v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[8:9], s[14:15], v[10:11]
	v_add_f64 v[14:15], v[10:11], v[12:13]
	s_mov_b32 s15, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_fma_f64 v[12:13], v[8:9], s[14:15], v[12:13]
	v_add_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[8:9], s[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], v[12:13]
	v_add_f64 v[14:15], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
.LBB2_9:
	s_or_b32 exec_lo, exec_lo, s2
	v_mul_f64 v[8:9], v[4:5], v[4:5]
	v_mul_f64 v[14:15], v[10:11], v[10:11]
	s_mov_b32 s2, 0xb42fdfa7
	s_mov_b32 s6, 0xf9a43bb8
	s_mov_b32 s3, 0xbe5ae600
	s_mov_b32 s7, 0x3de5e0b2
	v_mul_lo_u32 v18, v21, s10
	v_ashrrev_i32_e32 v1, 31, v0
	s_mov_b32 s14, 0x796cde01
	s_mov_b32 s15, 0x3ec71de3
	s_mov_b32 s10, 0x9037ab78
	s_mov_b32 s11, 0x3e21eeb6
	v_mul_f64 v[42:43], v[6:7], 0.5
	v_mul_f64 v[48:49], v[12:13], 0.5
	v_sub_nc_u32_e32 v20, v20, v18
	s_delay_alu instid0(VALU_DEP_1)
	v_mad_i64_i32 v[24:25], null, v20, s13, v[0:1]
	s_ashr_i32 s13, s12, 31
	v_mad_i64_i32 v[28:29], null, v21, s9, v[24:25]
	s_lshl_b64 s[8:9], s[12:13], 3
	v_lshlrev_b64 v[20:21], 3, v[28:29]
	v_fma_f64 v[16:17], v[8:9], s[6:7], s[2:3]
	v_fma_f64 v[18:19], v[14:15], s[6:7], s[2:3]
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mul_f64 v[26:27], v[8:9], 0.5
	s_mov_b32 s6, 0x46cc5e42
	s_mov_b32 s7, 0xbda907db
	v_mul_f64 v[30:31], v[14:15], 0.5
	v_fma_f64 v[0:1], v[8:9], s[6:7], s[10:11]
	v_fma_f64 v[32:33], v[14:15], s[6:7], s[10:11]
	s_mov_b32 s6, 0x19e83e5c
	s_mov_b32 s7, 0xbf2a01a0
	v_mul_f64 v[44:45], v[4:5], -v[8:9]
	v_mul_f64 v[50:51], v[10:11], -v[14:15]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v24, vcc_lo, s0, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v25, null, s1, v21, vcc_lo
	s_mov_b32 s0, 0xa17f65f6
	v_add_co_u32 v28, vcc_lo, v24, s8
	v_add_co_ci_u32_e64 v29, null, s9, v25, vcc_lo
	s_mov_b32 s1, 0xbe927e4f
	s_clause 0x1
	global_load_b64 v[34:35], v[28:29], off
	global_load_b64 v[36:37], v[24:25], off
	v_fma_f64 v[16:17], v[8:9], v[16:17], s[14:15]
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[14:15]
	v_add_f64 v[38:39], -v[26:27], 1.0
	v_add_f64 v[40:41], -v[30:31], 1.0
	v_fma_f64 v[0:1], v[8:9], v[0:1], s[0:1]
	v_fma_f64 v[32:33], v[14:15], v[32:33], s[0:1]
	s_mov_b32 s0, 0x19f4ec90
	s_mov_b32 s1, 0x3efa01a0
	v_fma_f64 v[16:17], v[8:9], v[16:17], s[6:7]
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[6:7]
	s_mov_b32 s6, 0x11110bb3
	s_mov_b32 s7, 0x3f811111
	v_add_f64 v[46:47], -v[38:39], 1.0
	v_add_f64 v[52:53], -v[40:41], 1.0
	v_fma_f64 v[0:1], v[8:9], v[0:1], s[0:1]
	v_fma_f64 v[32:33], v[14:15], v[32:33], s[0:1]
	s_mov_b32 s0, 0x16c16967
	s_mov_b32 s1, 0xbf56c16c
	v_fma_f64 v[16:17], v[8:9], v[16:17], s[6:7]
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[6:7]
	v_add_f64 v[26:27], v[46:47], -v[26:27]
	v_add_f64 v[30:31], v[52:53], -v[30:31]
	v_fma_f64 v[0:1], v[8:9], v[0:1], s[0:1]
	v_fma_f64 v[32:33], v[14:15], v[32:33], s[0:1]
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s1, 0x3fa55555
	v_fma_f64 v[16:17], v[44:45], v[16:17], v[42:43]
	v_fma_f64 v[18:19], v[50:51], v[18:19], v[48:49]
	v_mul_f64 v[42:43], v[8:9], v[8:9]
	v_fma_f64 v[26:27], v[4:5], -v[6:7], v[26:27]
	v_fma_f64 v[30:31], v[10:11], -v[12:13], v[30:31]
	v_fma_f64 v[0:1], v[8:9], v[0:1], s[0:1]
	v_fma_f64 v[6:7], v[8:9], v[16:17], -v[6:7]
	v_mul_f64 v[8:9], v[14:15], v[14:15]
	v_fma_f64 v[16:17], v[14:15], v[32:33], s[0:1]
	v_fma_f64 v[12:13], v[14:15], v[18:19], -v[12:13]
	s_mov_b32 s1, 0xbfc55555
	v_fma_f64 v[0:1], v[42:43], v[0:1], v[26:27]
	v_fma_f64 v[6:7], v[44:45], s[0:1], v[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[30:31]
	v_fma_f64 v[12:13], v[50:51], s[0:1], v[12:13]
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	v_lshlrev_b32_e32 v2, 30, v23
	v_add_f64 v[0:1], v[38:39], v[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v2, v2, v3
	v_and_b32_e32 v2, 0x80000000, v2
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_f64 v[6:7], v[40:41], v[8:9]
	v_add_f64 v[8:9], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v5, 0x80000000, v5
	v_and_b32_e32 v10, 1, v22
	v_cmp_eq_u32_e32 vcc_lo, 0, v10
	v_cndmask_b32_e32 v0, v4, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v1, v5, v1 :: v_dual_and_b32 v4, 1, v23
	v_cndmask_b32_e64 v0, 0, v0, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_eq_u32_e64 s1, 0, v4
	v_cndmask_b32_e64 v4, v7, v9, s1
	v_cndmask_b32_e64 v3, v6, v8, s1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v4, v4, v2
	v_cndmask_b32_e64 v2, 0, v3, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v3, 0x7ff80000, v4, s0
	v_lshlrev_b32_e32 v4, 30, v22
	v_mul_f64 v[2:3], s[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b32_e32 v4, 0x80000000, v4
	v_xor_b32_e32 v1, v1, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v1, 0x7ff80000, v1, s0
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], v[34:35], v[2:3]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[8:9], v[36:37], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[36:37], v[0:1], -v[6:7]
	v_fma_f64 v[6:7], v[34:35], v[0:1], v[8:9]
	v_add_co_u32 v8, vcc_lo, s2, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s3, v21, vcc_lo
	s_clause 0x1
	global_store_b64 v[24:25], v[4:5], off
	global_store_b64 v[28:29], v[6:7], off
	v_add_co_u32 v10, vcc_lo, v8, s8
	v_add_co_ci_u32_e64 v11, null, s9, v9, vcc_lo
	s_clause 0x1
	global_load_b64 v[4:5], v[10:11], off
	global_load_b64 v[6:7], v[8:9], off
	s_waitcnt vmcnt(1)
	v_mul_f64 v[12:13], v[4:5], v[2:3]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[0:1], -v[12:13]
	v_fma_f64 v[0:1], v[4:5], v[0:1], v[2:3]
	s_clause 0x1
	global_store_b64 v[8:9], v[6:7], off
	global_store_b64 v[10:11], v[0:1], off
.LBB2_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_
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
		.amdhsa_next_free_vgpr 54
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
		.amdhsa_inst_pref_size 48
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_,"axG",@progbits,_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_,comdat
.Lfunc_end2:
	.size	_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_, .Lfunc_end2-_Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_
                                        ; -- End function
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.num_vgpr, 54
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.num_agpr, 0
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.numbered_sgpr, 20
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.num_named_barrier, 0
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.private_seg_size, 0
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.uses_vcc, 1
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.uses_flat_scratch, 0
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.has_dyn_sized_stack, 0
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.has_recursion, 0
	.set _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 6084
; TotalNumSgprs: 22
; NumVgprs: 54
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 6
; NumSGPRsForWavesPerEU: 22
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_efd4eefeef10034b,@object ; @__hip_cuid_efd4eefeef10034b
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_efd4eefeef10034b
__hip_cuid_efd4eefeef10034b:
	.byte	0                               ; 0x0
	.size	__hip_cuid_efd4eefeef10034b, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_efd4eefeef10034b
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
    .name:           ropex_qk_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         ropex_qk_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     51
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
    .name:           _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z21ropex_qk_heads_kernelIfEvPT_S1_iiiiPKS0_S3_.kd
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
    .name:           _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         _Z21ropex_qk_heads_kernelIdEvPT_S1_iiiiPKS0_S3_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     54
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
