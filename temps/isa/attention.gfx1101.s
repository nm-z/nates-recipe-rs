	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z15attn_sdp_kernelPKfS0_S0_Pfiii ; -- Begin function _Z15attn_sdp_kernelPKfS0_S0_Pfiii
	.globl	_Z15attn_sdp_kernelPKfS0_S0_Pfiii
	.p2align	8
	.type	_Z15attn_sdp_kernelPKfS0_S0_Pfiii,@function
_Z15attn_sdp_kernelPKfS0_S0_Pfiii:      ; @_Z15attn_sdp_kernelPKfS0_S0_Pfiii
; %bb.0:
	s_load_b128 s[16:19], s[0:1], 0x20
	v_lshl_add_u32 v3, v0, 2, 0x80
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s3, s16
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v1, s3
	s_sub_i32 s4, 0, s3
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s6, v1
	s_mul_i32 s5, s4, s6
	s_abs_i32 s4, s2
	s_mul_hi_u32 s7, s6, s5
	v_cmp_gt_i32_e64 s5, s17, v0
	s_add_i32 s6, s6, s7
	s_mov_b32 s7, 0
	s_mul_hi_u32 s6, s4, s6
	s_and_saveexec_b32 s8, s5
	s_cbranch_execz .LBB0_3
; %bb.1:
	s_load_b32 s9, s[0:1], 0x3c
	v_lshl_add_u32 v1, v0, 2, 0x80
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v4, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s9, s9, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s10, s9, 2
.LBB0_2:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_nc_u32_e32 v4, s9, v4
	ds_store_b32 v1, v2
	v_add_nc_u32_e32 v1, s10, v1
	v_cmp_le_i32_e32 vcc_lo, s17, v4
	s_or_b32 s7, vcc_lo, s7
	s_and_not1_b32 exec_lo, exec_lo, s7
	s_cbranch_execnz .LBB0_2
.LBB0_3:
	s_or_b32 exec_lo, exec_lo, s8
	s_mul_i32 s6, s6, s3
	s_ashr_i32 s7, s2, 31
	s_sub_i32 s4, s4, s6
	s_load_b256 s[8:15], s[0:1], 0x0
	s_sub_i32 s6, s4, s3
	s_cmp_ge_u32 s4, s3
	s_waitcnt lgkmcnt(0)
	s_cselect_b32 s4, s6, s4
	s_barrier
	s_sub_i32 s6, s4, s3
	s_cmp_ge_u32 s4, s3
	buffer_gl0_inv
	s_cselect_b32 s3, s6, s4
	s_mul_i32 s6, s17, s2
	s_xor_b32 s3, s3, s7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s4, s3, s7
	s_ashr_i32 s7, s6, 31
	s_add_i32 s3, s4, 1
	s_cmp_eq_u32 s18, 0
	s_mov_b32 s18, 0
	s_cselect_b32 s16, s16, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lt_i32 s16, 1
	s_cbranch_scc1 .LBB0_21
; %bb.4:
	v_cvt_f32_i32_e32 v1, s17
	s_lshl_b64 s[20:21], s[6:7], 2
	v_mbcnt_lo_u32_b32 v11, -1, 0
	s_add_u32 s19, s8, s20
	s_addc_u32 s20, s9, s21
	v_dual_mul_f32 v2, 0x4f800000, v1 :: v_dual_and_b32 v13, 31, v0
	v_cmp_gt_f32_e32 vcc_lo, 0xf800000, v1
	s_sub_i32 s2, s2, s4
	s_load_b32 s22, s[0:1], 0x3c
	s_mul_i32 s8, s2, s17
	v_cmp_gt_u32_e64 s2, 24, v11
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	s_ashr_i32 s9, s8, 31
	v_cmp_eq_u32_e64 s4, 0, v13
	s_lshl_b64 s[8:9], s[8:9], 2
	v_cndmask_b32_e64 v9, 0, 8, s2
	v_sqrt_f32_e32 v2, v1
	v_cmp_gt_u32_e64 s2, 28, v11
	s_add_u32 s10, s10, s8
	s_addc_u32 s11, s11, s9
	s_add_u32 s12, s12, s8
	s_addc_u32 s13, s13, s9
	v_cndmask_b32_e64 v16, 0, 4, s2
	v_cmp_gt_u32_e64 s2, 30, v11
	v_lshl_or_b32 v8, v11, 2, 64
	v_add_lshl_u32 v9, v9, v11, 2
	s_waitcnt_depctr 0xfff
	v_add_nc_u32_e32 v4, -1, v2
	v_add_nc_u32_e32 v5, 1, v2
	v_cndmask_b32_e64 v17, 0, 2, s2
	v_cmp_ne_u32_e64 s2, 31, v11
	s_waitcnt lgkmcnt(0)
	s_and_b32 s21, s22, 0xffff
	v_fma_f32 v6, -v4, v2, v1
	s_lshl_b32 s22, s21, 2
	v_add_co_ci_u32_e64 v19, null, 0, v11, s2
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_ge_f32_e64 s3, 0, v6
	v_mov_b32_e32 v6, 0
	v_fma_f32 v7, -v5, v2, v1
	s_add_i32 s2, s21, 31
	v_cndmask_b32_e64 v2, v2, v4, s3
	s_lshr_b32 s2, s2, 5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_lt_f32_e64 s3, 0, v7
	v_cndmask_b32_e64 v2, v2, v5, s3
	v_cmp_gt_u32_e64 s3, 32, v0
	v_lshl_add_u32 v5, v0, 2, 0x80
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, 0x37800000, v2
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x260
	v_lshrrev_b32_e32 v4, 3, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v2, v1, vcc_lo
	v_div_scale_f32 v2, null, v1, v1, 1.0
	v_div_scale_f32 v10, vcc_lo, 1.0, v1, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v12, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v7, -v2, v12, 1.0
	v_dual_mov_b32 v15, 0xf149f2ca :: v_dual_fmac_f32 v12, v7, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mul_f32 v14, v10, v12 :: v_dual_lshlrev_b32 v7, 2, v13
	v_fma_f32 v18, -v2, v14, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v14, v18, v12
	v_fma_f32 v2, -v2, v14, v10
	v_add_lshl_u32 v10, v16, v11, 2
	v_add_lshl_u32 v11, v17, v11, 2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_div_fmas_f32 v2, v2, v12, v14
	v_lshlrev_b32_e32 v12, 2, v19
	v_cmp_gt_u32_e32 vcc_lo, s2, v13
	v_mov_b32_e32 v14, 0
	v_div_fixup_f32 v13, v2, v1, 1.0
.LBB0_5:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB0_7 Depth 2
                                        ;     Child Loop BB0_18 Depth 2
	v_mov_b32_e32 v16, 0
	s_mul_i32 s8, s18, s17
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ashr_i32 s9, s8, 31
	s_and_saveexec_b32 s23, s5
	s_cbranch_execz .LBB0_9
; %bb.6:                                ;   in Loop: Header=BB0_5 Depth=1
	s_lshl_b64 s[24:25], s[8:9], 2
	v_dual_mov_b32 v16, 0 :: v_dual_mov_b32 v1, v0
	s_add_u32 s24, s10, s24
	s_addc_u32 s26, s11, s25
	s_mov_b32 s25, 0
	.p2align	6
.LBB0_7:                                ;   Parent Loop BB0_5 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[17:18], 2, v[1:2]
	v_add_nc_u32_e32 v1, s21, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v19, s2, s19, v17
	v_add_co_ci_u32_e64 v20, null, s20, v18, s2
	v_add_co_u32 v17, s2, s24, v17
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v18, null, s26, v18, s2
	global_load_b32 v2, v[19:20], off
	global_load_b32 v17, v[17:18], off
	v_cmp_le_i32_e64 s2, s17, v1
	s_or_b32 s25, s2, s25
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v16, v2, v17
	s_and_not1_b32 exec_lo, exec_lo, s25
	s_cbranch_execnz .LBB0_7
; %bb.8:                                ;   in Loop: Header=BB0_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s25
.LBB0_9:                                ;   in Loop: Header=BB0_5 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s23
	ds_bpermute_b32 v1, v8, v16
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v16, v1
	ds_bpermute_b32 v2, v9, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v10, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v11, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v12, v1
	s_and_saveexec_b32 s2, s4
	s_cbranch_execz .LBB0_11
; %bb.10:                               ;   in Loop: Header=BB0_5 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v4, v1
.LBB0_11:                               ;   in Loop: Header=BB0_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s2, s3
	s_cbranch_execz .LBB0_16
; %bb.12:                               ;   in Loop: Header=BB0_5 Depth=1
	v_mov_b32_e32 v1, 0
	s_and_saveexec_b32 s23, vcc_lo
; %bb.13:                               ;   in Loop: Header=BB0_5 Depth=1
	ds_load_b32 v1, v7
; %bb.14:                               ;   in Loop: Header=BB0_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s23
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v8, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v9, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v10, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v11, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v12, v1
	s_and_b32 exec_lo, exec_lo, s4
	s_cbranch_execz .LBB0_16
; %bb.15:                               ;   in Loop: Header=BB0_5 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v6, v1
.LBB0_16:                               ;   in Loop: Header=BB0_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v1, v6
	s_waitcnt lgkmcnt(0)
	v_mul_f32_e32 v2, v13, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e64 s2, v2, v15
	v_cndmask_b32_e64 v17, v15, v2, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v2, v15, v17
	v_fma_f32 v1, v13, v1, -v17
	v_dual_mul_f32 v15, 0x3fb8aa3b, v2 :: v_dual_mul_f32 v16, 0x3fb8aa3b, v1
	v_cmp_ngt_f32_e64 s2, 0xc2ce8ed0, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f32 v18, 0x3fb8aa3b, v2, -v15
	v_rndne_f32_e32 v19, v15
	v_fma_f32 v20, 0x3fb8aa3b, v1, -v16
	v_rndne_f32_e32 v21, v16
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_fmac_f32 v18, 0x32a5705f, v2 :: v_dual_sub_f32 v15, v15, v19
	v_fmac_f32_e32 v20, 0x32a5705f, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_sub_f32 v16, v16, v21 :: v_dual_add_f32 v15, v15, v18
	v_cvt_i32_f32_e32 v18, v19
	v_add_f32_e32 v16, v16, v20
	v_cvt_i32_f32_e32 v19, v21
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_exp_f32_e32 v15, v15
	v_exp_f32_e32 v16, v16
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v15, v15, v18
	v_ldexp_f32 v16, v16, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v15, 0, v15, s2
	v_cmp_ngt_f32_e64 s2, 0xc2ce8ed0, v1
	v_cndmask_b32_e64 v16, 0, v16, s2
	v_cmp_nlt_f32_e64 s2, 0x42b17218, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v15, 0x7f800000, v15, s2
	v_cmp_nlt_f32_e64 s2, 0x42b17218, v1
	v_cndmask_b32_e64 v16, 0x7f800000, v16, s2
	s_and_saveexec_b32 s23, s5
	s_cbranch_execz .LBB0_19
; %bb.17:                               ;   in Loop: Header=BB0_5 Depth=1
	s_lshl_b64 s[8:9], s[8:9], 2
	v_dual_mov_b32 v18, v5 :: v_dual_mov_b32 v1, v0
	s_add_u32 s8, s12, s8
	s_addc_u32 s9, s13, s9
	s_mov_b32 s24, 0
	.p2align	6
.LBB0_18:                               ;   Parent Loop BB0_5 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[19:20], 2, v[1:2]
	v_add_nc_u32_e32 v1, s21, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v19, s2, s8, v19
	v_add_co_ci_u32_e64 v20, null, s9, v20, s2
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s2, s17, v1
	global_load_b32 v2, v[19:20], off
	ds_load_b32 v19, v18
	s_or_b32 s24, s2, s24
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v2, v16, v2
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v2, v15, v19
	ds_store_b32 v18, v2
	v_add_nc_u32_e32 v18, s22, v18
	s_and_not1_b32 exec_lo, exec_lo, s24
	s_cbranch_execnz .LBB0_18
.LBB0_19:                               ;   in Loop: Header=BB0_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s23
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v16, v14, v15
	s_add_i32 s18, s18, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u32 s18, s16
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB0_22
; %bb.20:                               ;   in Loop: Header=BB0_5 Depth=1
	v_dual_mov_b32 v15, v17 :: v_dual_mov_b32 v14, v16
	s_branch .LBB0_5
.LBB0_21:
	v_mov_b32_e32 v16, 0
.LBB0_22:
	s_and_saveexec_b32 s2, s5
	s_cbranch_execz .LBB0_25
; %bb.23:
	s_delay_alu instid0(VALU_DEP_1)
	v_div_scale_f32 v1, null, v16, v16, 1.0
	v_div_scale_f32 v5, vcc_lo, 1.0, v16, 1.0
	s_load_b32 s0, s[0:1], 0x3c
	v_rcp_f32_e32 v2, v1
	s_lshl_b64 s[2:3], s[6:7], 2
	s_mov_b32 s4, 0
	s_add_u32 s1, s14, s2
	s_addc_u32 s2, s15, s3
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, -v1, v2, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v2, v4, v2
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s0, 0xffff
	v_mul_f32_e32 v4, v5, v2
	s_lshl_b32 s5, s3, 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v6, -v1, v4, v5
	v_fmac_f32_e32 v4, v6, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v1, -v1, v4, v5
	v_div_fmas_f32 v1, v1, v2, v4
	v_cmp_lt_f32_e32 vcc_lo, 0, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v1, v1, v16, 1.0
	v_cndmask_b32_e32 v2, 0, v1, vcc_lo
	.p2align	6
.LBB0_24:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b32 v6, v3
	v_ashrrev_i32_e32 v1, 31, v0
	v_add_nc_u32_e32 v3, s5, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 2, v[0:1]
	v_add_nc_u32_e32 v0, s3, v0
	v_add_co_u32 v4, s0, s1, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s2, v5, s0
	s_waitcnt lgkmcnt(0)
	v_mul_f32_e32 v1, v2, v6
	v_cmp_le_i32_e32 vcc_lo, s17, v0
	global_store_b32 v[4:5], v1, off
	s_or_b32 s4, vcc_lo, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s4
	s_cbranch_execnz .LBB0_24
.LBB0_25:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15attn_sdp_kernelPKfS0_S0_Pfiii
		.amdhsa_group_segment_fixed_size 128
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
		.amdhsa_next_free_sgpr 27
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
.Lfunc_end0:
	.size	_Z15attn_sdp_kernelPKfS0_S0_Pfiii, .Lfunc_end0-_Z15attn_sdp_kernelPKfS0_S0_Pfiii
                                        ; -- End function
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.num_vgpr, 22
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.num_agpr, 0
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.numbered_sgpr, 27
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.num_named_barrier, 0
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.private_seg_size, 0
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.uses_vcc, 1
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.uses_flat_scratch, 0
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.has_dyn_sized_stack, 0
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.has_recursion, 0
	.set _Z15attn_sdp_kernelPKfS0_S0_Pfiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1920
; TotalNumSgprs: 29
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 128 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 29
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
	.protected	_Z26attn_causal_softmax_kernelPfii ; -- Begin function _Z26attn_causal_softmax_kernelPfii
	.globl	_Z26attn_causal_softmax_kernelPfii
	.p2align	8
	.type	_Z26attn_causal_softmax_kernelPfii,@function
_Z26attn_causal_softmax_kernelPfii:     ; @_Z26attn_causal_softmax_kernelPfii
; %bb.0:
	s_load_b64 s[4:5], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s4
	s_cbranch_scc1 .LBB1_28
; %bb.1:
	s_load_b64 s[6:7], s[0:1], 0x0
	s_mul_i32 s8, s5, s2
	v_cmp_ge_i32_e64 s3, s2, v0
	s_ashr_i32 s9, s8, 31
	v_cmp_gt_i32_e32 vcc_lo, s5, v0
	s_lshl_b64 s[8:9], s[8:9], 2
	v_mov_b32_e32 v3, 0xf149f2ca
	s_waitcnt lgkmcnt(0)
	s_add_u32 s6, s6, s8
	s_addc_u32 s7, s7, s9
	s_and_b32 s3, s3, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s8, s3
	s_cbranch_execz .LBB1_5
; %bb.2:
	s_load_b32 s3, s[0:1], 0x1c
	v_mov_b32_e32 v3, 0xf149f2ca
	v_mov_b32_e32 v1, v0
	s_mov_b32 s9, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s10, s3, 0xffff
	.p2align	6
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	v_add_nc_u32_e32 v1, s10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_i32_e64 s4, s5, v1
	v_add_co_u32 v4, s3, s6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v5, null, s7, v5, s3
	v_cmp_lt_i32_e64 s3, s2, v1
	global_load_b32 v2, v[4:5], off
	s_or_b32 s3, s3, s4
	s_and_b32 s4, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_or_b32 s9, s4, s9
	s_waitcnt vmcnt(0)
	v_cmp_gt_f32_e64 s3, v2, v3
	v_cndmask_b32_e64 v3, v3, v2, s3
	s_and_not1_b32 exec_lo, exec_lo, s9
	s_cbranch_execnz .LBB1_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s9
.LBB1_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s8
	v_mbcnt_lo_u32_b32 v1, -1, 0
	v_and_b32_e32 v9, 31, v0
	v_lshl_or_b32 v4, v1, 2, 64
	v_cmp_gt_u32_e64 s3, 24, v1
	ds_bpermute_b32 v2, v4, v3
	v_cndmask_b32_e64 v5, 0, 8, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_lshl_u32 v5, v5, v1, 2
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s3, v3, v2
	v_cndmask_b32_e64 v2, v3, v2, s3
	v_cmp_gt_u32_e64 s3, 28, v1
	ds_bpermute_b32 v3, v5, v2
	v_cndmask_b32_e64 v6, 0, 4, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_lshl_u32 v6, v6, v1, 2
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s3, v2, v3
	v_cndmask_b32_e64 v2, v2, v3, s3
	v_cmp_gt_u32_e64 s3, 30, v1
	ds_bpermute_b32 v3, v6, v2
	v_cndmask_b32_e64 v7, 0, 2, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_lshl_u32 v7, v7, v1, 2
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s3, v2, v3
	v_cndmask_b32_e64 v2, v2, v3, s3
	v_cmp_ne_u32_e64 s3, 31, v1
	ds_bpermute_b32 v3, v7, v2
	v_add_co_ci_u32_e64 v8, null, 0, v1, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_lshlrev_b32_e32 v8, 2, v8
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s3, v2, v3
	v_cndmask_b32_e64 v1, v2, v3, s3
	v_lshrrev_b32_e32 v3, 5, v0
	v_cmp_eq_u32_e64 s3, 0, v9
	ds_bpermute_b32 v2, v8, v1
	v_lshlrev_b32_e32 v10, 2, v3
	s_and_saveexec_b32 s8, s3
	s_cbranch_execz .LBB1_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s4, v1, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v1, v1, v2, s4
	ds_store_b32 v10, v1
.LBB1_7:
	s_or_b32 exec_lo, exec_lo, s8
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_load_b32 s1, s[0:1], 0x1c
	v_cmp_gt_u32_e64 s0, 32, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s8, s1, 0xffff
	s_and_saveexec_b32 s4, s0
	s_cbranch_execz .LBB1_12
; %bb.8:
	v_mov_b32_e32 v1, 0xf149f2ca
	s_add_i32 s1, s8, 31
	s_mov_b32 s9, exec_lo
	s_lshr_b32 s1, s1, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s1, v9
; %bb.9:
	v_lshlrev_b32_e32 v1, 2, v9
	ds_load_b32 v1, v1
; %bb.10:
	s_or_b32 exec_lo, exec_lo, s9
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s1, v1, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v1, v1, v2, s1
	ds_bpermute_b32 v2, v5, v1
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s1, v1, v2
	v_cndmask_b32_e64 v1, v1, v2, s1
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s1, v1, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v1, v1, v2, s1
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s1, v1, v2
	v_cndmask_b32_e64 v1, v1, v2, s1
	ds_bpermute_b32 v2, v8, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB1_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f32_e64 s1, v1, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v1, v1, v2, s1
	v_mov_b32_e32 v2, 0
	ds_store_b32 v2, v1
.LBB1_12:
	s_or_b32 exec_lo, exec_lo, s4
	v_mov_b32_e32 v11, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s9, vcc_lo
	s_cbranch_execz .LBB1_18
; %bb.13:
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v1, v0
	s_mov_b32 s10, 0
	ds_load_b32 v12, v11
	s_branch .LBB1_15
.LBB1_14:                               ;   in Loop: Header=BB1_15 Depth=1
	s_or_b32 exec_lo, exec_lo, s4
	v_add_nc_u32_e32 v1, s8, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, s4, s6, v2
	v_add_co_ci_u32_e64 v3, null, s7, v3, s4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e64 s1, s5, v1
	v_add_f32_e32 v11, v11, v13
	global_store_b32 v[2:3], v13, off
	s_or_b32 s10, s1, s10
	s_and_not1_b32 exec_lo, exec_lo, s10
	s_cbranch_execz .LBB1_17
.LBB1_15:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	v_mov_b32_e32 v13, 0
	s_mov_b32 s4, exec_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	v_cmpx_ge_i32_e64 s2, v1
	s_cbranch_execz .LBB1_14
; %bb.16:                               ;   in Loop: Header=BB1_15 Depth=1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v13, s1, s6, v2
	v_add_co_ci_u32_e64 v14, null, s7, v3, s1
	global_load_b32 v13, v[13:14], off
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_sub_f32_e32 v13, v13, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v14, 0x3fb8aa3b, v13
	v_fma_f32 v15, 0x3fb8aa3b, v13, -v14
	v_rndne_f32_e32 v16, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v15, 0x32a5705f, v13 :: v_dual_sub_f32 v14, v14, v16
	v_add_f32_e32 v14, v14, v15
	v_cvt_i32_f32_e32 v15, v16
	v_cmp_ngt_f32_e64 s1, 0xc2ce8ed0, v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v14, v14
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v14, v14, v15
	v_cndmask_b32_e64 v14, 0, v14, s1
	v_cmp_nlt_f32_e64 s1, 0x42b17218, v13
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v13, 0x7f800000, v14, s1
	s_branch .LBB1_14
.LBB1_17:
	s_or_b32 exec_lo, exec_lo, s10
.LBB1_18:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s9
	ds_bpermute_b32 v1, v4, v11
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v11, v1
	ds_bpermute_b32 v2, v5, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_saveexec_b32 s1, s3
	s_cbranch_execz .LBB1_20
; %bb.19:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v10, v1 offset:128
.LBB1_20:
	s_or_b32 exec_lo, exec_lo, s1
	s_waitcnt lgkmcnt(0)
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB1_25
; %bb.21:
	v_mov_b32_e32 v1, 0
	s_add_i32 s0, s8, 31
	s_mov_b32 s2, exec_lo
	s_lshr_b32 s0, s0, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s0, v9
; %bb.22:
	v_lshlrev_b32_e32 v1, 2, v9
	ds_load_b32 v1, v1 offset:128
; %bb.23:
	s_or_b32 exec_lo, exec_lo, s2
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v5, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB1_25
; %bb.24:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1 offset:128
.LBB1_25:
	s_or_b32 exec_lo, exec_lo, s1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, vcc_lo
	s_cbranch_execz .LBB1_28
; %bb.26:
	v_mov_b32_e32 v1, 0
	s_mov_b32 s0, 0
	ds_load_b32 v1, v1 offset:128
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v2, null, v1, v1, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v3, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, -v2, v3, 1.0
	v_fmac_f32_e32 v3, v4, v3
	v_div_scale_f32 v4, vcc_lo, 1.0, v1, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v5, v4, v3
	v_fma_f32 v6, -v2, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v5, v6, v3
	v_fma_f32 v2, -v2, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f32 v2, v2, v3, v5
	v_cmp_lt_f32_e32 vcc_lo, 0, v1
	v_div_fixup_f32 v2, v2, v1, 1.0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	.p2align	6
.LBB1_27:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[0:1]
	v_add_co_u32 v3, vcc_lo, s6, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s7, v4, vcc_lo
	global_load_b32 v1, v[3:4], off
	s_waitcnt vmcnt(0)
	v_dual_mul_f32 v1, v2, v1 :: v_dual_add_nc_u32 v0, s8, v0
	v_cmp_le_i32_e32 vcc_lo, s5, v0
	global_store_b32 v[3:4], v1, off
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB1_27
.LBB1_28:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z26attn_causal_softmax_kernelPfii
		.amdhsa_group_segment_fixed_size 256
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 272
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
.Lfunc_end1:
	.size	_Z26attn_causal_softmax_kernelPfii, .Lfunc_end1-_Z26attn_causal_softmax_kernelPfii
                                        ; -- End function
	.set _Z26attn_causal_softmax_kernelPfii.num_vgpr, 17
	.set _Z26attn_causal_softmax_kernelPfii.num_agpr, 0
	.set _Z26attn_causal_softmax_kernelPfii.numbered_sgpr, 11
	.set _Z26attn_causal_softmax_kernelPfii.num_named_barrier, 0
	.set _Z26attn_causal_softmax_kernelPfii.private_seg_size, 0
	.set _Z26attn_causal_softmax_kernelPfii.uses_vcc, 1
	.set _Z26attn_causal_softmax_kernelPfii.uses_flat_scratch, 0
	.set _Z26attn_causal_softmax_kernelPfii.has_dyn_sized_stack, 0
	.set _Z26attn_causal_softmax_kernelPfii.has_recursion, 0
	.set _Z26attn_causal_softmax_kernelPfii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1676
; TotalNumSgprs: 13
; NumVgprs: 17
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 13
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
	.protected	_Z21attn_mha_split_kernelPKfPfiii ; -- Begin function _Z21attn_mha_split_kernelPKfPfiii
	.globl	_Z21attn_mha_split_kernelPKfPfiii
	.p2align	8
	.type	_Z21attn_mha_split_kernelPKfPfiii,@function
_Z21attn_mha_split_kernelPKfPfiii:      ; @_Z21attn_mha_split_kernelPKfPfiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s6, s5
	s_mul_i32 s3, s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s3, v1
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_abs_i32 s3, s2
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s3
	s_sub_i32 s7, 0, s3
	v_xor_b32_e32 v5, s2, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v5
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, s7, v0
	s_abs_i32 s7, s6
	v_mul_hi_u32 v2, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v2
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s3
	v_sub_nc_u32_e32 v2, v3, v2
	v_add_nc_u32_e32 v3, 1, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_subrev_nc_u32_e32 v4, s3, v2
	v_cmp_le_u32_e32 vcc_lo, s3, v2
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_cvt_f32_u32_e32 v3, s7
	v_add_nc_u32_e32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s3, v2
	v_rcp_iflag_f32_e32 v3, v3
	s_sub_i32 s3, 0, s7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_xor_b32_e32 v0, v0, v5
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v3
	v_sub_nc_u32_e32 v0, v0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v2, v2
	v_mul_lo_u32 v3, v0, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v4, s3, v2
	s_load_b128 s[0:3], s[0:1], 0x0
	v_sub_nc_u32_e32 v5, v1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v1, v2, v4
	v_sub_nc_u32_e32 v3, 0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v1, v2, v1
	v_max_i32_e32 v2, v5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v1, v2, v1
	v_mul_lo_u32 v3, v1, s7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v2, v3
	v_add_nc_u32_e32 v3, 1, v1
	v_subrev_nc_u32_e32 v4, s7, v2
	v_cmp_le_u32_e32 vcc_lo, s7, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v1, v1, v3 :: v_dual_cndmask_b32 v2, v2, v4
	v_xor_b32_e32 v3, s6, v5
	v_add_nc_u32_e32 v4, 1, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s7, v2
	v_ashrrev_i32_e32 v3, 31, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v1, v4, vcc_lo
	v_xor_b32_e32 v1, v1, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v1, v1, v3
	v_mul_lo_u32 v4, v1, s6
	v_mad_u64_u32 v[2:3], null, v0, s5, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v3, v5, v4
	v_mad_u64_u32 v[4:5], null, v2, s6, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v4
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s0, v4
	v_add_co_ci_u32_e64 v5, null, s1, v5, vcc_lo
	global_load_b32 v2, v[4:5], off
	v_mad_u64_u32 v[4:5], null, v1, s4, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[0:1], null, v4, s6, v[3:4]
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b32 v[0:1], v2, off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21attn_mha_split_kernelPKfPfiii
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
.Lfunc_end2:
	.size	_Z21attn_mha_split_kernelPKfPfiii, .Lfunc_end2-_Z21attn_mha_split_kernelPKfPfiii
                                        ; -- End function
	.set _Z21attn_mha_split_kernelPKfPfiii.num_vgpr, 6
	.set _Z21attn_mha_split_kernelPKfPfiii.num_agpr, 0
	.set _Z21attn_mha_split_kernelPKfPfiii.numbered_sgpr, 8
	.set _Z21attn_mha_split_kernelPKfPfiii.num_named_barrier, 0
	.set _Z21attn_mha_split_kernelPKfPfiii.private_seg_size, 0
	.set _Z21attn_mha_split_kernelPKfPfiii.uses_vcc, 1
	.set _Z21attn_mha_split_kernelPKfPfiii.uses_flat_scratch, 0
	.set _Z21attn_mha_split_kernelPKfPfiii.has_dyn_sized_stack, 0
	.set _Z21attn_mha_split_kernelPKfPfiii.has_recursion, 0
	.set _Z21attn_mha_split_kernelPKfPfiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 572
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
	.protected	_Z21attn_mha_merge_kernelPKfPfiii ; -- Begin function _Z21attn_mha_merge_kernelPKfPfiii
	.globl	_Z21attn_mha_merge_kernelPKfPfiii
	.p2align	8
	.type	_Z21attn_mha_merge_kernelPKfPfiii,@function
_Z21attn_mha_merge_kernelPKfPfiii:      ; @_Z21attn_mha_merge_kernelPKfPfiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s6, s4
	s_mul_i32 s3, s2, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s3, v1
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_abs_i32 s3, s2
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s3
	s_sub_i32 s7, 0, s3
	v_xor_b32_e32 v5, s2, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v5
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, s7, v0
	s_abs_i32 s7, s6
	v_mul_hi_u32 v2, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v2
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s3
	v_sub_nc_u32_e32 v2, v3, v2
	v_add_nc_u32_e32 v3, 1, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_subrev_nc_u32_e32 v4, s3, v2
	v_cmp_le_u32_e32 vcc_lo, s3, v2
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_cvt_f32_u32_e32 v3, s7
	v_add_nc_u32_e32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s3, v2
	v_rcp_iflag_f32_e32 v3, v3
	s_sub_i32 s3, 0, s7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_xor_b32_e32 v0, v0, v5
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v3
	v_sub_nc_u32_e32 v0, v0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v2, v2
	v_mul_lo_u32 v3, v0, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v4, s3, v2
	s_load_b128 s[0:3], s[0:1], 0x0
	v_sub_nc_u32_e32 v5, v1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v1, v2, v4
	v_sub_nc_u32_e32 v3, 0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v1, v2, v1
	v_max_i32_e32 v2, v5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v1, v2, v1
	v_mul_lo_u32 v3, v1, s7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v2, v3
	v_add_nc_u32_e32 v3, 1, v1
	v_subrev_nc_u32_e32 v4, s7, v2
	v_cmp_le_u32_e32 vcc_lo, s7, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v1, v1, v3 :: v_dual_cndmask_b32 v2, v2, v4
	v_xor_b32_e32 v3, s6, v5
	v_add_nc_u32_e32 v4, 1, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s7, v2
	v_ashrrev_i32_e32 v3, 31, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v1, v4, vcc_lo
	v_xor_b32_e32 v1, v1, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v1, v1, v3
	v_mul_lo_u32 v4, v1, s6
	v_mad_u64_u32 v[2:3], null, v0, s4, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v3, v5, v4
	v_mad_u64_u32 v[4:5], null, v2, s6, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v4
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s0, v4
	v_add_co_ci_u32_e64 v5, null, s1, v5, vcc_lo
	global_load_b32 v2, v[4:5], off
	v_mad_u64_u32 v[4:5], null, v1, s5, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[0:1], null, v4, s6, v[3:4]
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b32 v[0:1], v2, off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21attn_mha_merge_kernelPKfPfiii
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
.Lfunc_end3:
	.size	_Z21attn_mha_merge_kernelPKfPfiii, .Lfunc_end3-_Z21attn_mha_merge_kernelPKfPfiii
                                        ; -- End function
	.set _Z21attn_mha_merge_kernelPKfPfiii.num_vgpr, 6
	.set _Z21attn_mha_merge_kernelPKfPfiii.num_agpr, 0
	.set _Z21attn_mha_merge_kernelPKfPfiii.numbered_sgpr, 8
	.set _Z21attn_mha_merge_kernelPKfPfiii.num_named_barrier, 0
	.set _Z21attn_mha_merge_kernelPKfPfiii.private_seg_size, 0
	.set _Z21attn_mha_merge_kernelPKfPfiii.uses_vcc, 1
	.set _Z21attn_mha_merge_kernelPKfPfiii.uses_flat_scratch, 0
	.set _Z21attn_mha_merge_kernelPKfPfiii.has_dyn_sized_stack, 0
	.set _Z21attn_mha_merge_kernelPKfPfiii.has_recursion, 0
	.set _Z21attn_mha_merge_kernelPKfPfiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 572
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
	.protected	_Z16attn_rope_kernelPKfPfiiS0_ ; -- Begin function _Z16attn_rope_kernelPKfPfiiS0_
	.globl	_Z16attn_rope_kernelPKfPfiiS0_
	.p2align	8
	.type	_Z16attn_rope_kernelPKfPfiiS0_,@function
_Z16attn_rope_kernelPKfPfiiS0_:         ; @_Z16attn_rope_kernelPKfPfiiS0_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[4:5], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_lshr_b32 s2, s5, 31
	s_add_i32 s2, s5, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s6, s2, 1
	s_mul_i32 s2, s6, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB4_10
; %bb.1:
	s_abs_i32 s2, s6
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
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
	v_xor_b32_e32 v3, s6, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_load_b64 s[2:3], s[0:1], 0x18
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v2, v0, v3
	v_cvt_f32_i32_e32 v3, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v2, s6
	s_waitcnt lgkmcnt(0)
	s_load_b32 s2, s[2:3], 0x0
	v_sub_nc_u32_e32 v0, v1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b32_e32 v1, 1, v0
	v_cvt_f32_i32_e32 v1, v1
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
	v_cmp_neq_f32_e64 s7, v1, |v1|
	v_cndmask_b32_e64 v5, 1.0, s2, vcc_lo
	s_mov_b32 s2, 0x3e76c4e1
	v_frexp_mant_f32_e64 v3, |v5|
	v_cmp_lt_f32_e64 s8, |v5|, 1.0
	v_cmp_eq_f32_e64 s4, 0, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v3
	s_xor_b32 s7, s7, s8
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fmaak_f32 v11, s2, v10, 0x3e91f4c4
	v_sub_f32_e32 v7, v10, v7
	v_mul_f32_e32 v14, v6, v10
	v_fmaak_f32 v11, v10, v11, 0x3ecccdef
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v7, v9, v7
	v_mul_f32_e32 v12, v10, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
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
	v_dual_fmac_f32 v12, v10, v8 :: v_dual_add_f32 v13, 0x3f2aaaaa, v11
	v_ldexp_f32 v8, v8, 1
	v_dual_fmac_f32 v12, v7, v6 :: v_dual_add_f32 v9, 0xbf2aaaaa, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v11, v9
	v_dual_add_f32 v4, v4, v9 :: v_dual_add_f32 v9, v14, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v7, v13, v4
	v_sub_f32_e32 v10, v13, v7
	v_subrev_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f32_e32 v4, v4, v10
	v_sub_f32_e32 v13, v9, v14
	v_cvt_f32_i32_e32 v3, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v12, v12, v13 :: v_dual_mul_f32 v11, v9, v7
	v_fma_f32 v10, v9, v7, -v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v10, v9, v4
	v_ldexp_f32 v4, v6, 1
	v_fmac_f32_e32 v10, v12, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v6, v11, v10
	v_add_f32_e32 v7, v4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_sub_f32 v4, v7, v4 :: v_dual_sub_f32 v9, v6, v11
	v_mul_f32_e32 v11, 0x3f317218, v3
	v_sub_f32_e32 v9, v10, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v10, 0x3f317218, v3, -v11
	v_dual_sub_f32 v4, v6, v4 :: v_dual_fmamk_f32 v3, v3, 0xb102e308, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v6, v8, v9
	v_add_f32_e32 v4, v6, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v6, v11, v3
	v_dual_add_f32 v8, v7, v4 :: v_dual_sub_f32 v11, v6, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v7, v8, v7
	v_add_f32_e32 v9, v6, v8
	v_sub_f32_e32 v3, v3, v11
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v4, v4, v7
	v_sub_f32_e32 v10, v9, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v7, v8, v10 :: v_dual_add_f32 v8, v3, v4
	v_sub_f32_e32 v12, v9, v10
	v_sub_f32_e32 v6, v6, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v6, v7, v6 :: v_dual_sub_f32 v7, v8, v3
	v_add_f32_e32 v6, v8, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v8, v8, v7
	v_dual_sub_f32 v4, v4, v7 :: v_dual_sub_f32 v3, v3, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v10, v9, v6 :: v_dual_add_f32 v3, v4, v3
	v_sub_f32_e32 v7, v10, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v4, v6, v7
	v_add_f32_e32 v3, v3, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v4, v10, v3
	v_dual_sub_f32 v6, v4, v10 :: v_dual_mul_f32 v7, v1, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v3, v3, v6
	v_fma_f32 v4, v1, v4, -v7
	v_cmp_class_f32_e64 vcc_lo, v7, 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v4, v1, v3
	v_add_f32_e32 v3, v7, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v6, v3, v7, vcc_lo
	v_sub_f32_e32 v3, v3, v7
	v_cmp_eq_f32_e32 vcc_lo, 0x42b17218, v6
	v_cndmask_b32_e64 v8, 0, 0x37000000, vcc_lo
	v_cmp_neq_f32_e64 vcc_lo, 0x7f800000, |v6|
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v9, v6, v8
	v_trunc_f32_e32 v6, v1
	v_dual_sub_f32 v3, v4, v3 :: v_dual_mul_f32 v10, 0x3fb8aa3b, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f32 v11, 0x3fb8aa3b, v9, -v10
	v_rndne_f32_e32 v12, v10
	v_dual_fmamk_f32 v11, v9, 0x32a5705f, v11 :: v_dual_sub_f32 v10, v10, v12
	v_cvt_i32_f32_e32 v7, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v10, v10, v11
	v_exp_f32_e32 v10, v10
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v4, v10, v7
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_mul_f32 v7, 0.5, v1 :: v_dual_cndmask_b32 v4, 0, v4
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v9
	v_trunc_f32_e32 v10, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_add_f32 v3, v8, v3 :: v_dual_cndmask_b32 v4, 0x7f800000, v4
	v_cmp_eq_f32_e32 vcc_lo, v6, v1
	v_cmp_neq_f32_e64 s2, v10, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f32 v3, v4, v3, v4
	v_cmp_class_f32_e64 s3, v4, 0x204
	s_and_b32 s2, vcc_lo, s2
	v_cndmask_b32_e64 v6, 1.0, v5, s2
	v_cndmask_b32_e64 v8, 0, v5, s2
	v_cndmask_b32_e64 v3, v3, v4, s3
	v_cndmask_b32_e64 v4, 0x7f800000, 0, s7
	v_cmp_gt_f32_e64 s3, 0, v1
	v_cmp_class_f32_e64 s2, v5, 0x204
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_bfi_b32 v3, 0x7fffffff, v3, v6
	s_xor_b32 s3, s3, s4
	v_cndmask_b32_e64 v6, 0x7f800000, 0, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v7, 0x7fc00000, v3, vcc_lo
	v_cmp_neq_f32_e64 vcc_lo, |v5|, 1.0
	v_bfi_b32 v6, 0x7fffffff, v6, v8
	v_cndmask_b32_e32 v4, 1.0, v4, vcc_lo
	v_cmp_gt_f32_e32 vcc_lo, 0, v5
	v_cndmask_b32_e32 v3, v3, v7, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	s_or_b32 vcc_lo, s4, s2
	v_cvt_f32_i32_e32 v4, v2
	v_cndmask_b32_e32 v3, v3, v6, vcc_lo
	v_cmp_o_f32_e32 vcc_lo, v5, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, 0x7fc00000, v3, vcc_lo
	v_div_scale_f32 v3, null, v1, v1, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v5, v3
	s_waitcnt_depctr 0xfff
	v_fma_f32 v6, -v3, v5, 1.0
	v_fmac_f32_e32 v5, v6, v5
	v_div_scale_f32 v6, vcc_lo, v4, v1, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v7, v6, v5
	v_fma_f32 v8, -v3, v7, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v7, v8, v5
	v_fma_f32 v3, -v3, v7, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v3, v3, v5, v7
                                        ; implicit-def: $vgpr5
	v_div_fixup_f32 v1, v3, v1, v4
                                        ; implicit-def: $vgpr4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_and_b32_e32 v3, 0x7fffffff, v1
	v_cmp_ngt_f32_e64 s4, 0x48000000, |v1|
	v_lshrrev_b32_e32 v6, 23, v3
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s7, exec_lo, s2
	s_cbranch_execz .LBB4_3
; %bb.2:
	s_mov_b32 s2, 0x7fffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_or_b32 v16, v3, s2, 0x800000
	v_mad_u64_u32 v[4:5], null, 0xfe5163ab, v16, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_mov_b32 v8, 0 :: v_dual_mov_b32 v7, v5
	v_add_nc_u32_e32 v5, 0xffffff88, v6
	v_mad_u64_u32 v[9:10], null, 0x3c439041, v16, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v5
	v_cndmask_b32_e64 v14, 0, 0xffffffc0, vcc_lo
	v_mov_b32_e32 v7, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v5, v14, v5
	v_mad_u64_u32 v[10:11], null, 0xdb629599, v16, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s2, 31, v5
	v_mov_b32_e32 v7, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v15, 0, 0xffffffe0, s2
	v_cndmask_b32_e32 v4, v10, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[11:12], null, 0xf534ddc0, v16, v[7:8]
	v_add_nc_u32_e32 v5, v15, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e64 s3, 31, v5
	v_mov_b32_e32 v7, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[12:13], null, 0xfc2757d1, v16, v[7:8]
	v_mov_b32_e32 v7, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[13:14], null, 0x4e441529, v16, v[7:8]
	v_mov_b32_e32 v7, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[14:15], null, 0xa2f9836e, v16, v[7:8]
	v_cndmask_b32_e64 v7, 0, 0xffffffe0, s3
	v_dual_cndmask_b32 v8, v13, v11 :: v_dual_add_nc_u32 v5, v7, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v14, v14, v12 :: v_dual_cndmask_b32 v13, v15, v13
	v_dual_cndmask_b32 v12, v12, v10 :: v_dual_cndmask_b32 v7, v11, v9
	v_cmp_eq_u32_e32 vcc_lo, 0, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v9, v14, v8, s2
	v_cndmask_b32_e64 v11, v13, v14, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v8, v8, v12, s2
	v_sub_nc_u32_e32 v13, 32, v5
	v_cndmask_b32_e64 v12, v12, v7, s2
	v_cndmask_b32_e64 v4, v7, v4, s2
	v_cndmask_b32_e64 v11, v11, v9, s3
	v_cndmask_b32_e64 v9, v9, v8, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v8, v8, v12, s3
	v_cndmask_b32_e64 v4, v12, v4, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v14, v11, v9, v13.l
	v_alignbit_b32 v10, v9, v8, v13.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v12, v8, v4, v13.l
	v_cndmask_b32_e32 v5, v14, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v7, v10, v9 :: v_dual_cndmask_b32 v8, v12, v8
	v_bfe_u32 v9, v5, 29, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v10, v5, v7, 30
	v_alignbit_b32 v7, v7, v8, 30
	v_alignbit_b32 v4, v8, v4, 30
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v11, 0, v9
	v_xor_b32_e32 v10, v10, v11
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v7, v7, v11
	v_xor_b32_e32 v4, v4, v11
	v_lshrrev_b32_e32 v11, 29, v5
	v_lshrrev_b32_e32 v5, 30, v5
	v_clz_i32_u32_e32 v12, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v5, v9, v5
	v_min_u32_e32 v12, 32, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v8, 31, v12
	v_lshlrev_b32_e32 v13, 23, v12
	v_alignbit_b32 v10, v10, v7, v8.l
	v_alignbit_b32 v4, v7, v4, v8.l
	v_lshlrev_b32_e32 v7, 31, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_alignbit_b32 v8, v10, v4, 9
	v_or_b32_e32 v11, 0.5, v7
	v_lshrrev_b32_e32 v10, 9, v10
	v_or_b32_e32 v7, 0x33000000, v7
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_clz_i32_u32_e32 v14, v8
	v_sub_nc_u32_e32 v11, v11, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_min_u32_e32 v13, 32, v14
	v_or_b32_e32 v10, v10, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_not_b32_e32 v11, v13
	v_mul_f32_e32 v14, 0x3fc90fda, v10
	v_add_lshl_u32 v12, v13, v12, 23
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v4, v8, v4, v11.l
	v_fma_f32 v8, 0x3fc90fda, v10, -v14
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v7, v12
	v_lshrrev_b32_e32 v4, 9, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmamk_f32 v8, v10, 0x33a22168, v8
	v_or_b32_e32 v4, v7, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v8, 0x3fc90fda, v4
	v_add_f32_e32 v4, v14, v8
	s_or_saveexec_b32 s2, s7
	v_mul_f32_e64 v9, 0x3f22f983, |v1|
	s_xor_b32 exec_lo, exec_lo, s2
	s_branch .LBB4_4
.LBB4_3:
	s_or_saveexec_b32 s2, s7
	v_mul_f32_e64 v9, 0x3f22f983, |v1|
	s_xor_b32 exec_lo, exec_lo, s2
.LBB4_4:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f32_e32 v5, v9
	v_fma_f32 v4, 0xbfc90fda, v5, |v1|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmamk_f32 v4, v5, 0xb3a22168, v4
	v_fmamk_f32 v4, v5, 0xa7c234c4, v4
	v_cvt_i32_f32_e32 v5, v5
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s2
                                        ; implicit-def: $vgpr8
                                        ; implicit-def: $vgpr7
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB4_7
; %bb.6:
	s_mov_b32 s2, 0x7fffff
	v_mov_b32_e32 v9, 0
	v_and_or_b32 v17, v3, s2, 0x800000
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[7:8], null, 0xfe5163ab, v17, 0
	v_mad_u64_u32 v[10:11], null, 0x3c439041, v17, v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v8, v11
	v_mad_u64_u32 v[11:12], null, 0xdb629599, v17, v[8:9]
	v_add_nc_u32_e32 v6, 0xffffff88, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v6
	v_mov_b32_e32 v8, v12
	v_cndmask_b32_e64 v15, 0, 0xffffffc0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[12:13], null, 0xf534ddc0, v17, v[8:9]
	v_cndmask_b32_e32 v7, v11, v7, vcc_lo
	v_add_nc_u32_e32 v6, v15, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v8, v13
	v_cmp_lt_u32_e64 s2, 31, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[13:14], null, 0xfc2757d1, v17, v[8:9]
	v_cndmask_b32_e64 v16, 0, 0xffffffe0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v6, v16, v6
	v_mov_b32_e32 v8, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s3, 31, v6
	v_mad_u64_u32 v[14:15], null, 0x4e441529, v17, v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v8, v15
	v_mad_u64_u32 v[15:16], null, 0xa2f9836e, v17, v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v8, 0, 0xffffffe0, s3
	v_dual_cndmask_b32 v9, v14, v12 :: v_dual_add_nc_u32 v6, v8, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v15, v15, v13 :: v_dual_cndmask_b32 v14, v16, v14
	v_dual_cndmask_b32 v13, v13, v11 :: v_dual_cndmask_b32 v8, v12, v10
	v_cmp_eq_u32_e32 vcc_lo, 0, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v10, v15, v9, s2
	v_cndmask_b32_e64 v12, v14, v15, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v9, v9, v13, s2
	v_sub_nc_u32_e32 v14, 32, v6
	v_cndmask_b32_e64 v13, v13, v8, s2
	v_cndmask_b32_e64 v7, v8, v7, s2
	v_cndmask_b32_e64 v12, v12, v10, s3
	v_cndmask_b32_e64 v10, v10, v9, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v9, v9, v13, s3
	v_cndmask_b32_e64 v7, v13, v7, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v15, v12, v10, v14.l
	v_alignbit_b32 v11, v10, v9, v14.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v13, v9, v7, v14.l
	v_cndmask_b32_e32 v6, v15, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v8, v11, v10 :: v_dual_cndmask_b32 v9, v13, v9
	v_bfe_u32 v10, v6, 29, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v11, v6, v8, 30
	v_alignbit_b32 v8, v8, v9, 30
	v_alignbit_b32 v7, v9, v7, 30
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v12, 0, v10
	v_xor_b32_e32 v11, v11, v12
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v8, v8, v12
	v_xor_b32_e32 v7, v7, v12
	v_lshrrev_b32_e32 v12, 29, v6
	v_lshrrev_b32_e32 v6, 30, v6
	v_clz_i32_u32_e32 v13, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_min_u32_e32 v13, 32, v13
	v_sub_nc_u32_e32 v9, 31, v13
	v_lshlrev_b32_e32 v14, 23, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_alignbit_b32 v11, v11, v8, v9.l
	v_alignbit_b32 v7, v8, v7, v9.l
	v_lshlrev_b32_e32 v8, 31, v12
	v_alignbit_b32 v9, v11, v7, 9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_or_b32_e32 v12, 0.5, v8
	v_lshrrev_b32_e32 v11, 9, v11
	v_or_b32_e32 v8, 0x33000000, v8
	v_clz_i32_u32_e32 v15, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v12, v12, v14
	v_min_u32_e32 v14, 32, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_or_b32_e32 v11, v11, v12
	v_not_b32_e32 v12, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v15, 0x3fc90fda, v11
	v_add_lshl_u32 v13, v14, v13, 23
	v_alignbit_b32 v7, v9, v7, v12.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f32 v9, 0x3fc90fda, v11, -v15
	v_sub_nc_u32_e32 v8, v8, v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshrrev_b32_e32 v7, 9, v7
	v_fmamk_f32 v9, v11, 0x33a22168, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_or_b32_e32 v7, v8, v7
	v_dual_fmac_f32 v9, 0x3fc90fda, v7 :: v_dual_add_nc_u32 v8, v10, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v7, v15, v9
                                        ; implicit-def: $vgpr9
	s_and_not1_saveexec_b32 s2, s4
	s_cbranch_execnz .LBB4_8
	s_branch .LBB4_9
.LBB4_7:
	s_and_not1_saveexec_b32 s2, s4
.LBB4_8:
	v_rndne_f32_e32 v6, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v7, 0xbfc90fda, v6, |v1|
	v_cvt_i32_f32_e32 v8, v6
	v_fmamk_f32 v7, v6, 0xb3a22168, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_fmamk_f32 v7, v6, 0xa7c234c4, v7
.LBB4_9:
	s_or_b32 exec_lo, exec_lo, s2
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mad_u64_u32 v[9:10], null, v2, s5, v[0:1]
	s_ashr_i32 s7, s6, 31
	s_mov_b32 s4, 0xb94c1982
	v_mul_f32_e32 v0, v4, v4
	s_mov_b32 s5, 0x37d75334
	v_and_b32_e32 v2, 1, v5
	v_lshlrev_b32_e32 v5, 30, v5
	v_ashrrev_i32_e32 v10, 31, v9
	v_fmaak_f32 v15, s5, v0, 0xbab64f3b
	v_xor_b32_e32 v3, v3, v1
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_and_b32_e32 v5, 0x80000000, v5
	v_lshlrev_b64 v[9:10], 2, v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmaak_f32 v15, v0, v15, 0x3d2aabf7
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v11, vcc_lo, s0, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v12, null, s1, v10, vcc_lo
	s_lshl_b64 s[0:1], s[6:7], 2
	v_fmaak_f32 v15, v0, v15, 0xbf000004
	v_add_co_u32 v13, vcc_lo, v11, s0
	v_add_co_ci_u32_e64 v14, null, s1, v12, vcc_lo
	s_clause 0x1
	global_load_b32 v6, v[11:12], off
	global_load_b32 v11, v[13:14], off
	v_dual_mul_f32 v12, v7, v7 :: v_dual_and_b32 v13, 1, v8
	v_fmaak_f32 v14, s4, v0, 0x3c0881c4
	v_lshlrev_b32_e32 v8, 30, v8
	v_cmp_eq_u32_e32 vcc_lo, 0, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v16, s4, v12, 0x3c0881c4
	v_fmaak_f32 v16, v12, v16, 0xbe2aaa9d
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v16, v12, v16
	v_dual_fmaak_f32 v14, v0, v14, 0xbe2aaa9d :: v_dual_fmac_f32 v7, v7, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_fmaak_f32 v17, s5, v12, 0xbab64f3b :: v_dual_mul_f32 v14, v0, v14
	v_fma_f32 v0, v0, v15, 1.0
	v_fmaak_f32 v17, v12, v17, 0x3d2aabf7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v4, v4, v14
	v_fmaak_f32 v17, v12, v17, 0xbf000004
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v0, -v4, v0, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, 0, v13
	v_fma_f32 v12, v12, v17, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v5, v0
	v_cndmask_b32_e32 v2, v12, v7, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x1f8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v4, 0x7fc00000, v0, vcc_lo
	v_and_b32_e32 v8, 0x80000000, v8
	v_xor3_b32 v2, v3, v8, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, 0x7fc00000, v2, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v9
	v_add_co_ci_u32_e64 v1, null, s3, v10, vcc_lo
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v5, v6, v2
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v7, v11, v2
	v_add_co_u32 v2, vcc_lo, v0, s0
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f32 v6, v4, v6, -v7
	v_fmac_f32_e32 v5, v4, v11
	s_clause 0x1
	global_store_b32 v[0:1], v6, off
	global_store_b32 v[2:3], v5, off
.LBB4_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z16attn_rope_kernelPKfPfiiS0_
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
		.amdhsa_next_free_vgpr 18
		.amdhsa_next_free_sgpr 9
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
		.amdhsa_inst_pref_size 29
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
	.size	_Z16attn_rope_kernelPKfPfiiS0_, .Lfunc_end4-_Z16attn_rope_kernelPKfPfiiS0_
                                        ; -- End function
	.set _Z16attn_rope_kernelPKfPfiiS0_.num_vgpr, 18
	.set _Z16attn_rope_kernelPKfPfiiS0_.num_agpr, 0
	.set _Z16attn_rope_kernelPKfPfiiS0_.numbered_sgpr, 9
	.set _Z16attn_rope_kernelPKfPfiiS0_.num_named_barrier, 0
	.set _Z16attn_rope_kernelPKfPfiiS0_.private_seg_size, 0
	.set _Z16attn_rope_kernelPKfPfiiS0_.uses_vcc, 1
	.set _Z16attn_rope_kernelPKfPfiiS0_.uses_flat_scratch, 0
	.set _Z16attn_rope_kernelPKfPfiiS0_.has_dyn_sized_stack, 0
	.set _Z16attn_rope_kernelPKfPfiiS0_.has_recursion, 0
	.set _Z16attn_rope_kernelPKfPfiiS0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3656
; TotalNumSgprs: 11
; NumVgprs: 18
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 11
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
	.protected	_Z19attn_pos_enc_kernelPfii ; -- Begin function _Z19attn_pos_enc_kernelPfii
	.globl	_Z19attn_pos_enc_kernelPfii
	.p2align	8
	.type	_Z19attn_pos_enc_kernelPfii,@function
_Z19attn_pos_enc_kernelPfii:            ; @_Z19attn_pos_enc_kernelPfii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x1c
	s_load_b64 s[4:5], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB5_14
; %bb.1:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s3, v0
	s_mov_b32 s3, 0x3e76c4e1
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
	v_xor_b32_e32 v3, s5, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	v_cvt_f32_i32_e32 v3, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v2, v0, s5
	v_cvt_f32_i32_e32 v0, v0
	v_sub_nc_u32_e32 v4, v1, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshrrev_b32_e32 v2, 31, v4
	v_add_nc_u32_e32 v2, v4, v2
	v_and_b32_e32 v4, 1, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b32_e32 v2, -2, v2
	v_cvt_f32_i32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f32 v5, null, v3, v3, v2
	v_div_scale_f32 v8, vcc_lo, v2, v3, v2
	v_rcp_f32_e32 v6, v5
	s_waitcnt_depctr 0xfff
	v_fma_f32 v7, -v5, v6, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v7, v6
	v_mul_f32_e32 v7, v8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v9, -v5, v7, v8
	v_fmac_f32_e32 v7, v9, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v5, -v5, v7, v8
	v_div_fmas_f32 v5, v5, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v5, v5, v3, v2
	v_cmp_eq_f32_e32 vcc_lo, 0, v5
	v_cndmask_b32_e64 v2, 0x461c4000, 1.0, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f32_e32 v3, v2
	v_cmp_gt_f32_e64 s2, 0x3f2aaaab, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v6, 1.0, 2.0, s2
	v_mul_f32_e32 v3, v3, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v6, 1.0, v3
	v_add_f32_e32 v8, -1.0, v3
	v_rcp_f32_e32 v7, v6
	v_add_f32_e32 v10, -1.0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v3, v3, v10
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v9, v8, v7
	v_mul_f32_e32 v11, v6, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v6, v9, v6, -v11
	v_fmac_f32_e32 v6, v9, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, v11, v6
	v_sub_f32_e32 v10, v8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v11, v3, v11 :: v_dual_sub_f32 v8, v8, v10
	v_dual_sub_f32 v6, v11, v6 :: v_dual_sub_f32 v3, v8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, v6, v3
	v_add_f32_e32 v3, v10, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v3, v7, v3
	v_add_f32_e32 v6, v9, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v7, v6, v9
	v_dual_mul_f32 v8, v6, v6 :: v_dual_sub_f32 v7, v3, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v9, v6, v6, -v8
	v_add_f32_e32 v3, v7, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v9, v6, v3
	v_cvt_f64_f32_e32 v[2:3], v2
	v_add_f32_e32 v10, v8, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmaak_f32 v11, s3, v10, 0x3e91f4c4 :: v_dual_sub_f32 v8, v10, v8
	v_dual_fmaak_f32 v11, v10, v11, 0x3ecccdef :: v_dual_sub_f32 v8, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v12, v10, v11
	v_fma_f32 v9, v10, v11, -v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v9, v8, v11
	v_add_f32_e32 v11, v12, v9
	v_frexp_exp_i32_f64_e32 v2, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v12, v11, v12 :: v_dual_add_f32 v13, 0x3f2aaaaa, v11
	v_sub_f32_e32 v3, v9, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v9, 0xbf2aaaaa, v13 :: v_dual_mul_f32 v14, v6, v10
	v_sub_f32_e32 v9, v11, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v12, v10, v6, -v14
	v_fmac_f32_e32 v12, v10, v7
	v_add_f32_e32 v3, 0x31739010, v3
	v_ldexp_f32 v7, v7, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v12, v8, v6 :: v_dual_add_f32 v3, v3, v9
	v_dual_add_f32 v9, v14, v12 :: v_dual_add_f32 v8, v13, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v10, v13, v8
	v_sub_f32_e32 v13, v9, v14
	v_mul_f32_e32 v11, v9, v8
	v_subrev_co_ci_u32_e64 v2, null, 0, v2, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v12, v12, v13
	v_cvt_f32_i32_e32 v2, v2
	v_add_f32_e32 v3, v3, v10
	v_fma_f32 v10, v9, v8, -v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v10, v9, v3
	v_ldexp_f32 v3, v6, 1
	v_fmac_f32_e32 v10, v12, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v6, v11, v10
	v_add_f32_e32 v8, v3, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v3, v8, v3
	v_sub_f32_e32 v9, v6, v11
	v_mul_f32_e32 v11, 0x3f317218, v2
	v_sub_f32_e32 v9, v10, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v10, 0x3f317218, v2, -v11
	v_dual_sub_f32 v3, v6, v3 :: v_dual_add_f32 v6, v7, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmamk_f32 v2, v2, 0xb102e308, v10
	v_dual_add_f32 v3, v6, v3 :: v_dual_add_f32 v6, v11, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v7, v8, v3
	v_add_f32_e32 v9, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v10, v9, v6
	v_sub_f32_e32 v12, v9, v10
	v_sub_f32_e32 v8, v7, v8
	v_sub_f32_e32 v7, v7, v10
	v_sub_f32_e32 v11, v6, v11
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_sub_f32_e32 v6, v6, v12
	v_sub_f32_e32 v3, v3, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f32_e32 v6, v7, v6
	v_sub_f32_e32 v2, v2, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v8, v2, v3
	v_sub_f32_e32 v7, v8, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v6, v8, v6
	v_sub_f32_e32 v8, v8, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_sub_f32 v2, v2, v8 :: v_dual_sub_f32 v3, v3, v7
	v_add_f32_e32 v10, v9, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v2, v3, v2 :: v_dual_sub_f32 v7, v10, v9
	v_sub_f32_e32 v3, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v2, v2, v3
	v_add_f32_e32 v3, v10, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v6, v3, v10 :: v_dual_mul_f32 v7, v5, v3
	v_sub_f32_e32 v2, v2, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v3, v5, v3, -v7
	v_cmp_class_f32_e64 s2, v7, 0x204
	v_fmac_f32_e32 v3, v5, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v2, v7, v3
	v_cndmask_b32_e64 v6, v2, v7, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_eq_f32_e64 s2, 0x42b17218, v6
	v_cndmask_b32_e64 v8, 0, 0x37000000, s2
	v_cmp_neq_f32_e64 s2, 0x7f800000, |v6|
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v9, v6, v8
	v_sub_f32_e32 v2, v2, v7
	v_mul_f32_e32 v10, 0x3fb8aa3b, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v2, v3, v2
	v_fma_f32 v11, 0x3fb8aa3b, v9, -v10
	v_rndne_f32_e32 v12, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v2, 0, v2, s2
	v_cmp_ngt_f32_e64 s2, 0xc2ce8ed0, v9
	v_dual_fmamk_f32 v11, v9, 0x32a5705f, v11 :: v_dual_sub_f32 v10, v10, v12
	v_cvt_i32_f32_e32 v7, v12
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v2, v8, v2
	v_add_f32_e32 v10, v10, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v10, v10
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v3, v10, v7
	v_cndmask_b32_e64 v3, 0, v3, s2
	v_cmp_nlt_f32_e64 s2, 0x42b17218, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v3, 0x7f800000, v3, s2
	v_cmp_neq_f32_e64 s2, v5, |v5|
	v_fma_f32 v2, v3, v2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v6, 0x7f800000, 0, s2
	v_cmp_class_f32_e64 s2, v5, 0x204
	v_cndmask_b32_e64 v6, v6, 1.0, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v3, 0x204
	v_cndmask_b32_e32 v2, v2, v3, vcc_lo
	v_cmp_o_f32_e32 vcc_lo, v5, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, |v2|, v6, s2
	s_mov_b32 s2, exec_lo
	v_cndmask_b32_e32 v2, 0x7fc00000, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f32 v3, null, v2, v2, v0
	v_div_scale_f32 v7, vcc_lo, v0, v2, v0
	v_rcp_f32_e32 v5, v3
	s_waitcnt_depctr 0xfff
	v_fma_f32 v6, -v3, v5, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v5, v6, v5
	v_mul_f32_e32 v6, v7, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v8, -v3, v6, v7
	v_fmac_f32_e32 v6, v8, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v3, -v3, v6, v7
	v_div_fmas_f32 v3, v3, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v0, v3, v2, v0
                                        ; implicit-def: $vgpr3
	v_and_b32_e32 v2, 0x7fffffff, v0
	v_cmp_ngt_f32_e64 s4, 0x48000000, |v0|
	v_cmpx_eq_u32_e32 1, v4
	s_xor_b32 s5, exec_lo, s2
	s_cbranch_execz .LBB5_7
; %bb.2:
                                        ; implicit-def: $vgpr4
                                        ; implicit-def: $vgpr3
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s6, exec_lo, s2
	s_cbranch_execz .LBB5_4
; %bb.3:
	s_mov_b32 s2, 0x7fffff
	v_mov_b32_e32 v5, 0
	v_and_or_b32 v13, v2, s2, 0x800000
	v_lshrrev_b32_e32 v2, 23, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, 0xfe5163ab, v13, 0
	v_mad_u64_u32 v[6:7], null, 0x3c439041, v13, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v4, v7
	v_mad_u64_u32 v[7:8], null, 0xdb629599, v13, v[4:5]
	v_add_nc_u32_e32 v2, 0xffffff88, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v2
	v_mov_b32_e32 v4, v8
	v_cndmask_b32_e64 v11, 0, 0xffffffc0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[8:9], null, 0xf534ddc0, v13, v[4:5]
	v_cndmask_b32_e32 v3, v7, v3, vcc_lo
	v_add_nc_u32_e32 v2, v11, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v4, v9
	v_cmp_lt_u32_e64 s2, 31, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[9:10], null, 0xfc2757d1, v13, v[4:5]
	v_cndmask_b32_e64 v12, 0, 0xffffffe0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v2, v12, v2
	v_mov_b32_e32 v4, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s3, 31, v2
	v_mad_u64_u32 v[10:11], null, 0x4e441529, v13, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v4, v11
	v_mad_u64_u32 v[11:12], null, 0xa2f9836e, v13, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0, 0xffffffe0, s3
	v_dual_cndmask_b32 v5, v10, v8 :: v_dual_add_nc_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v11, v11, v9 :: v_dual_cndmask_b32 v10, v12, v10
	v_dual_cndmask_b32 v9, v9, v7 :: v_dual_cndmask_b32 v4, v8, v6
	v_cmp_eq_u32_e32 vcc_lo, 0, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v6, v11, v5, s2
	v_cndmask_b32_e64 v8, v10, v11, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v5, v5, v9, s2
	v_sub_nc_u32_e32 v10, 32, v2
	v_cndmask_b32_e64 v9, v9, v4, s2
	v_cndmask_b32_e64 v3, v4, v3, s2
	v_cndmask_b32_e64 v8, v8, v6, s3
	v_cndmask_b32_e64 v6, v6, v5, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v5, v5, v9, s3
	v_cndmask_b32_e64 v3, v9, v3, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v11, v8, v6, v10.l
	v_alignbit_b32 v7, v6, v5, v10.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v9, v5, v3, v10.l
	v_cndmask_b32_e32 v2, v11, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v4, v7, v6 :: v_dual_cndmask_b32 v5, v9, v5
	v_bfe_u32 v6, v2, 29, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v7, v2, v4, 30
	v_alignbit_b32 v4, v4, v5, 30
	v_alignbit_b32 v3, v5, v3, 30
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v8, 0, v6
	v_xor_b32_e32 v7, v7, v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v4, v4, v8
	v_xor_b32_e32 v3, v3, v8
	v_lshrrev_b32_e32 v8, 29, v2
	v_lshrrev_b32_e32 v2, 30, v2
	v_clz_i32_u32_e32 v9, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_min_u32_e32 v9, 32, v9
	v_sub_nc_u32_e32 v5, 31, v9
	v_lshlrev_b32_e32 v10, 23, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_alignbit_b32 v7, v7, v4, v5.l
	v_alignbit_b32 v3, v4, v3, v5.l
	v_lshlrev_b32_e32 v4, 31, v8
	v_alignbit_b32 v5, v7, v3, 9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_or_b32_e32 v8, 0.5, v4
	v_lshrrev_b32_e32 v7, 9, v7
	v_or_b32_e32 v4, 0x33000000, v4
	v_clz_i32_u32_e32 v11, v5
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v8, v8, v10
	v_min_u32_e32 v10, 32, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_or_b32_e32 v7, v7, v8
	v_not_b32_e32 v8, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v11, 0x3fc90fda, v7
	v_add_lshl_u32 v9, v10, v9, 23
	v_alignbit_b32 v3, v5, v3, v8.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f32 v5, 0x3fc90fda, v7, -v11
	v_sub_nc_u32_e32 v4, v4, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshrrev_b32_e32 v3, 9, v3
	v_fmamk_f32 v5, v7, 0x33a22168, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_or_b32_e32 v3, v4, v3
	v_dual_fmac_f32 v5, 0x3fc90fda, v3 :: v_dual_add_nc_u32 v4, v6, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v3, v11, v5
.LBB5_4:
	s_and_not1_saveexec_b32 s2, s6
; %bb.5:
	v_mul_f32_e64 v2, 0x3f22f983, |v0|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f32_e32 v2, v2
	v_fma_f32 v3, 0xbfc90fda, v2, |v0|
	v_cvt_i32_f32_e32 v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmamk_f32 v3, v2, 0xb3a22168, v3
	v_fmamk_f32 v3, v2, 0xa7c234c4, v3
; %bb.6:
	s_or_b32 exec_lo, exec_lo, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_dual_mul_f32 v2, v3, v3 :: v_dual_and_b32 v7, 1, v4
	s_mov_b32 s2, 0xb94c1982
	s_mov_b32 s3, 0x37d75334
	v_dual_fmaak_f32 v5, s2, v2, 0x3c0881c4 :: v_dual_lshlrev_b32 v4, 30, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_eq_u32_e32 vcc_lo, 0, v7
	v_fmaak_f32 v5, v2, v5, 0xbe2aaa9d
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmaak_f32 v6, s3, v2, 0xbab64f3b :: v_dual_mul_f32 v5, v2, v5
	v_dual_fmaak_f32 v6, v2, v6, 0x3d2aabf7 :: v_dual_fmac_f32 v3, v3, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v6, v2, v6, 0xbf000004
	v_fma_f32 v2, v2, v6, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, -v3, v2, vcc_lo
	v_and_b32_e32 v3, 0x80000000, v4
	v_xor_b32_e32 v3, v3, v2
                                        ; implicit-def: $vgpr2
.LBB5_7:
	s_and_not1_saveexec_b32 s5, s5
	s_cbranch_execz .LBB5_13
; %bb.8:
                                        ; implicit-def: $vgpr4
                                        ; implicit-def: $vgpr3
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB5_10
; %bb.9:
	s_mov_b32 s2, 0x7fffff
	v_mov_b32_e32 v5, 0
	v_and_or_b32 v13, v2, s2, 0x800000
	v_lshrrev_b32_e32 v10, 23, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[3:4], null, 0xfe5163ab, v13, 0
	v_add_nc_u32_e32 v11, 0xffffff88, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v11
	v_mad_u64_u32 v[6:7], null, 0x3c439041, v13, v[4:5]
	v_cndmask_b32_e64 v12, 0, 0xffffffc0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v4, v7
	v_add_nc_u32_e32 v12, v12, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[7:8], null, 0xdb629599, v13, v[4:5]
	v_cmp_lt_u32_e64 s2, 31, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v14, 0, 0xffffffe0, s2
	v_dual_mov_b32 v4, v8 :: v_dual_cndmask_b32 v3, v7, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v14, v14, v12
	v_mad_u64_u32 v[8:9], null, 0xf534ddc0, v13, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s3, 31, v14
	v_mov_b32_e32 v4, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v6, v8, v6, vcc_lo
	v_mad_u64_u32 v[9:10], null, 0xfc2757d1, v13, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v3, v6, v3, s2
	v_mov_b32_e32 v4, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[10:11], null, 0x4e441529, v13, v[4:5]
	v_mov_b32_e32 v4, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[11:12], null, 0xa2f9836e, v13, v[4:5]
	v_cndmask_b32_e64 v4, 0, 0xffffffe0, s3
	v_dual_cndmask_b32 v5, v10, v8 :: v_dual_add_nc_u32 v4, v4, v14
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v11, v11, v9 :: v_dual_cndmask_b32 v10, v12, v10
	v_cndmask_b32_e32 v9, v9, v7, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, 0, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v8, v11, v5, s2
	v_cndmask_b32_e64 v10, v10, v11, s2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v5, v5, v9, s2
	v_sub_nc_u32_e32 v11, 32, v4
	v_cndmask_b32_e64 v9, v9, v6, s2
	v_cndmask_b32_e64 v10, v10, v8, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v8, v8, v5, s3
	v_cndmask_b32_e64 v5, v5, v9, s3
	v_cndmask_b32_e64 v3, v9, v3, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v12, v10, v8, v11.l
	v_alignbit_b32 v7, v8, v5, v11.l
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v4, v12, v10, vcc_lo
	v_alignbit_b32 v10, v5, v3, v11.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v6, v7, v8, vcc_lo
	v_bfe_u32 v7, v4, 29, 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v5, v10, v5, vcc_lo
	v_alignbit_b32 v8, v4, v6, 30
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v9, 0, v7
	v_alignbit_b32 v3, v5, v3, 30
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v8, v8, v9
	v_xor_b32_e32 v3, v3, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_clz_i32_u32_e32 v10, v8
	v_min_u32_e32 v10, 32, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b32_e32 v11, 23, v10
	v_alignbit_b32 v6, v6, v5, 30
	v_xor_b32_e32 v5, v6, v9
	v_sub_nc_u32_e32 v6, 31, v10
	v_lshrrev_b32_e32 v9, 29, v4
	v_lshrrev_b32_e32 v4, 30, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_alignbit_b32 v8, v8, v5, v6.l
	v_alignbit_b32 v3, v5, v3, v6.l
	v_lshlrev_b32_e32 v5, 31, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, v7, v4
	v_alignbit_b32 v6, v8, v3, 9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_or_b32_e32 v9, 0.5, v5
	v_lshrrev_b32_e32 v8, 9, v8
	v_or_b32_e32 v5, 0x33000000, v5
	v_clz_i32_u32_e32 v12, v6
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v9, v9, v11
	v_min_u32_e32 v11, 32, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_or_b32_e32 v8, v8, v9
	v_not_b32_e32 v9, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v12, 0x3fc90fda, v8
	v_add_lshl_u32 v10, v11, v10, 23
	v_alignbit_b32 v3, v6, v3, v9.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f32 v6, 0x3fc90fda, v8, -v12
	v_sub_nc_u32_e32 v5, v5, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshrrev_b32_e32 v3, 9, v3
	v_fmamk_f32 v6, v8, 0x33a22168, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_or_b32_e32 v3, v5, v3
	v_fmac_f32_e32 v6, 0x3fc90fda, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v3, v12, v6
.LBB5_10:
	s_and_not1_saveexec_b32 s2, s4
; %bb.11:
	v_mul_f32_e64 v3, 0x3f22f983, |v0|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f32_e32 v4, v3
	v_fma_f32 v3, 0xbfc90fda, v4, |v0|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmamk_f32 v3, v4, 0xb3a22168, v3
	v_fmamk_f32 v3, v4, 0xa7c234c4, v3
	v_cvt_i32_f32_e32 v4, v4
; %bb.12:
	s_or_b32 exec_lo, exec_lo, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_dual_mul_f32 v5, v3, v3 :: v_dual_and_b32 v8, 1, v4
	s_mov_b32 s2, 0xb94c1982
	s_mov_b32 s3, 0x37d75334
	v_xor_b32_e32 v2, v2, v0
	v_fmaak_f32 v6, s2, v5, 0x3c0881c4
	v_cmp_eq_u32_e32 vcc_lo, 0, v8
	v_lshlrev_b32_e32 v4, 30, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmaak_f32 v6, v5, v6, 0xbe2aaa9d
	v_fmaak_f32 v7, s3, v5, 0xbab64f3b
	v_and_b32_e32 v4, 0x80000000, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v6, v5, v6
	v_fmaak_f32 v7, v5, v7, 0x3d2aabf7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v3, v3, v6
	v_fmaak_f32 v7, v5, v7, 0xbf000004
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v5, v5, v7, 1.0
	v_cndmask_b32_e32 v3, v5, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_xor3_b32 v3, v2, v4, v3
.LBB5_13:
	s_or_b32 exec_lo, exec_lo, s5
	s_load_b64 s[0:1], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	v_cmp_class_f32_e64 vcc_lo, v0, 0x1f8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	v_cndmask_b32_e32 v3, 0x7fc00000, v3, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v2, vcc_lo
	global_store_b32 v[0:1], v3, off
.LBB5_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z19attn_pos_enc_kernelPfii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 272
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
		.amdhsa_next_free_vgpr 15
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
		.amdhsa_inst_pref_size 27
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
	.size	_Z19attn_pos_enc_kernelPfii, .Lfunc_end5-_Z19attn_pos_enc_kernelPfii
                                        ; -- End function
	.set _Z19attn_pos_enc_kernelPfii.num_vgpr, 15
	.set _Z19attn_pos_enc_kernelPfii.num_agpr, 0
	.set _Z19attn_pos_enc_kernelPfii.numbered_sgpr, 7
	.set _Z19attn_pos_enc_kernelPfii.num_named_barrier, 0
	.set _Z19attn_pos_enc_kernelPfii.private_seg_size, 0
	.set _Z19attn_pos_enc_kernelPfii.uses_vcc, 1
	.set _Z19attn_pos_enc_kernelPfii.uses_flat_scratch, 0
	.set _Z19attn_pos_enc_kernelPfii.has_dyn_sized_stack, 0
	.set _Z19attn_pos_enc_kernelPfii.has_recursion, 0
	.set _Z19attn_pos_enc_kernelPfii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3376
; TotalNumSgprs: 9
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 9
; NumVGPRsForWavesPerEU: 15
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
	.protected	_Z19attn_rmsnorm_kernelPKfS0_PfiiS0_ ; -- Begin function _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_
	.globl	_Z19attn_rmsnorm_kernelPKfS0_PfiiS0_
	.p2align	8
	.type	_Z19attn_rmsnorm_kernelPKfS0_PfiiS0_,@function
_Z19attn_rmsnorm_kernelPKfS0_PfiiS0_:   ; @_Z19attn_rmsnorm_kernelPKfS0_PfiiS0_
; %bb.0:
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s8
	s_cbranch_scc1 .LBB6_15
; %bb.1:
	s_clause 0x2
	s_load_b64 s[12:13], s[0:1], 0x20
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[10:11], s[0:1], 0x10
	s_mul_i32 s2, s9, s2
	v_cmp_gt_i32_e32 vcc_lo, s9, v0
	s_ashr_i32 s3, s2, 31
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	s_load_b32 s8, s[12:13], 0x0
	s_lshl_b64 s[12:13], s[2:3], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s3, s4, s12
	s_addc_u32 s4, s5, s13
	s_and_saveexec_b32 s5, vcc_lo
	s_cbranch_execz .LBB6_5
; %bb.2:
	s_load_b32 s2, s[0:1], 0x34
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_mov_b32 s14, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s15, s2, 0xffff
.LBB6_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	v_add_nc_u32_e32 v1, s15, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, s2, s3, v4
	v_add_co_ci_u32_e64 v5, null, s4, v5, s2
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s2, s9, v1
	global_load_b32 v2, v[4:5], off
	s_or_b32 s14, s2, s14
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v3, v2, v2
	s_and_not1_b32 exec_lo, exec_lo, s14
	s_cbranch_execnz .LBB6_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s14
.LBB6_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s5
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
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB6_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v7, v7, v8
	v_lshrrev_b32_e32 v8, 3, v0
	ds_store_b32 v8, v7
.LBB6_7:
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_load_b32 s1, s[0:1], 0x34
	s_mov_b32 s5, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s1, s1, 0xffff
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB6_12
; %bb.8:
	v_mov_b32_e32 v7, 0
	s_add_i32 s0, s1, 31
	s_mov_b32 s14, exec_lo
	s_lshr_b32 s0, s0, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s0, v6
; %bb.9:
	v_lshlrev_b32_e32 v6, 2, v6
	ds_load_b32 v7, v6
; %bb.10:
	s_or_b32 exec_lo, exec_lo, s14
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
	s_cbranch_execz .LBB6_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB6_12:
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, vcc_lo
	s_cbranch_execz .LBB6_15
; %bb.13:
	v_mov_b32_e32 v1, 0
	v_cvt_f32_i32_e32 v2, s9
	s_add_u32 s2, s10, s12
	s_addc_u32 s5, s11, s13
	ds_load_b32 v1, v1
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v3, null, v2, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v4, v3
	s_waitcnt_depctr 0xfff
	v_fma_f32 v5, -v3, v4, 1.0
	v_fmac_f32_e32 v4, v5, v4
	v_div_scale_f32 v5, vcc_lo, v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v6, v5, v4
	v_fma_f32 v7, -v3, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v7, v4
	v_fma_f32 v3, -v3, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v3, v3, v4, v6
	v_div_fixup_f32 v1, v3, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f32_e32 v1, s8, v1
	s_mov_b32 s8, 0
	v_mul_f32_e32 v2, 0x4b800000, v1
	v_cmp_gt_f32_e32 vcc_lo, 0x800000, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	v_rsq_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x45800000, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v1, v2, vcc_lo
	.p2align	6
.LBB6_14:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[0:1]
	v_add_co_u32 v5, vcc_lo, s3, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s4, v4, vcc_lo
	v_add_co_u32 v7, vcc_lo, s6, v3
	v_add_co_ci_u32_e64 v8, null, s7, v4, vcc_lo
	global_load_b32 v1, v[5:6], off
	global_load_b32 v5, v[7:8], off
	v_add_nc_u32_e32 v0, s1, v0
	v_add_co_u32 v3, s0, s2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v4, null, s5, v4, s0
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v1, v2, v1
	v_cmp_le_i32_e32 vcc_lo, s9, v0
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v1, v1, v5
	s_or_b32 s8, vcc_lo, s8
	global_store_b32 v[3:4], v1, off
	s_and_not1_b32 exec_lo, exec_lo, s8
	s_cbranch_execnz .LBB6_14
.LBB6_15:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_
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
		.amdhsa_next_free_vgpr 9
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
.Lfunc_end6:
	.size	_Z19attn_rmsnorm_kernelPKfS0_PfiiS0_, .Lfunc_end6-_Z19attn_rmsnorm_kernelPKfS0_PfiiS0_
                                        ; -- End function
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.num_vgpr, 9
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.num_agpr, 0
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.numbered_sgpr, 16
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.num_named_barrier, 0
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.private_seg_size, 0
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.uses_vcc, 1
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.uses_flat_scratch, 0
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.has_dyn_sized_stack, 0
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.has_recursion, 0
	.set _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1000
; TotalNumSgprs: 18
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 128 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 18
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
	.protected	_Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_ ; -- Begin function _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_
	.globl	_Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_
	.p2align	8
	.type	_Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_,@function
_Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_: ; @_Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_
; %bb.0:
	s_load_b64 s[12:13], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s12
	s_cbranch_scc1 .LBB7_28
; %bb.1:
	s_clause 0x2
	s_load_b64 s[16:17], s[0:1], 0x30
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b64 s[14:15], s[0:1], 0x20
	s_mul_i32 s2, s13, s2
	v_mov_b32_e32 v5, 0
	s_ashr_i32 s3, s2, 31
	s_waitcnt lgkmcnt(0)
	s_load_b32 s12, s[16:17], 0x0
	s_lshl_b64 s[16:17], s[2:3], 2
	v_cmp_gt_i32_e64 s2, s13, v0
	s_add_u32 s6, s6, s16
	s_addc_u32 s7, s7, s17
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB7_5
; %bb.2:
	s_load_b32 s18, s[0:1], 0x44
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v1, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s19, s18, 0xffff
	s_mov_b32 s18, 0
.LBB7_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	v_add_nc_u32_e32 v1, s19, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s6, v2
	v_add_co_ci_u32_e64 v3, null, s7, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e32 vcc_lo, s13, v1
	global_load_b32 v2, v[2:3], off
	s_or_b32 s18, vcc_lo, s18
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v5, v2, v2
	s_and_not1_b32 exec_lo, exec_lo, s18
	s_cbranch_execnz .LBB7_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s18
.LBB7_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s3
	v_mbcnt_lo_u32_b32 v1, -1, 0
	v_and_b32_e32 v9, 31, v0
	v_lshl_or_b32 v3, v1, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cmp_eq_u32_e64 s3, 0, v9
	ds_bpermute_b32 v2, v3, v5
	v_cndmask_b32_e64 v4, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v1
	v_add_lshl_u32 v4, v4, v1, 2
	v_cndmask_b32_e64 v6, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_lshl_u32 v6, v6, v1, 2
	v_cndmask_b32_e64 v7, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v1
	v_add_lshl_u32 v7, v7, v1, 2
	v_add_co_ci_u32_e64 v8, null, 0, v1, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v2, v5, v2
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b32_e32 v8, 2, v8
	ds_bpermute_b32 v5, v4, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v2, v2, v5
	ds_bpermute_b32 v5, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v2, v2, v5
	ds_bpermute_b32 v5, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v2, v5
	v_lshrrev_b32_e32 v5, 5, v0
	ds_bpermute_b32 v2, v8, v1
	v_lshlrev_b32_e32 v11, 2, v5
	s_and_saveexec_b32 s18, s3
	s_cbranch_execz .LBB7_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v11, v1
.LBB7_7:
	s_or_b32 exec_lo, exec_lo, s18
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_load_b32 s1, s[0:1], 0x44
	v_cmp_gt_u32_e64 s0, 32, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s1, s1, 0xffff
	s_and_saveexec_b32 s18, s0
	s_cbranch_execz .LBB7_12
; %bb.8:
	s_add_i32 s19, s1, 31
	v_mov_b32_e32 v1, 0
	s_lshr_b32 s19, s19, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_u32_e32 vcc_lo, s19, v9
	s_and_saveexec_b32 s19, vcc_lo
; %bb.9:
	v_lshlrev_b32_e32 v1, 2, v9
	ds_load_b32 v1, v1
; %bb.10:
	s_or_b32 exec_lo, exec_lo, s19
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v3, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB7_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB7_12:
	s_or_b32 exec_lo, exec_lo, s18
	v_mov_b32_e32 v12, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cvt_f32_i32_e32 v10, s13
	ds_load_b32 v1, v12
	s_add_u32 s4, s4, s16
	s_addc_u32 s5, s5, s17
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v2, null, v10, v10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v5, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v13, -v2, v5, 1.0
	v_fmac_f32_e32 v5, v13, v5
	v_div_scale_f32 v13, vcc_lo, v1, v10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v14, v13, v5
	v_fma_f32 v15, -v2, v14, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v14, v15, v5
	v_fma_f32 v2, -v2, v14, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v5, v14
	v_div_fixup_f32 v1, v2, v10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v1, s12, v1
	v_mul_f32_e32 v2, 0x4b800000, v1
	v_cmp_gt_f32_e32 vcc_lo, 0x800000, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	v_rsq_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x45800000, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v5, v1, v2, vcc_lo
	s_and_saveexec_b32 s12, s2
	s_cbranch_execz .LBB7_16
; %bb.13:
	v_dual_mov_b32 v12, 0 :: v_dual_mov_b32 v1, v0
	s_mov_b32 s18, 0
	.p2align	6
.LBB7_14:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[13:14], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v15, vcc_lo, s4, v13
	v_add_co_ci_u32_e64 v16, null, s5, v14, vcc_lo
	v_add_co_u32 v13, vcc_lo, s6, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v14, null, s7, v14, vcc_lo
	global_load_b32 v2, v[15:16], off
	global_load_b32 v13, v[13:14], off
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v2, v2, v13
	v_dual_fmac_f32 v12, v5, v2 :: v_dual_add_nc_u32 v1, s1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s13, v1
	s_or_b32 s18, vcc_lo, s18
	s_and_not1_b32 exec_lo, exec_lo, s18
	s_cbranch_execnz .LBB7_14
; %bb.15:
	s_or_b32 exec_lo, exec_lo, s18
.LBB7_16:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s12
	ds_bpermute_b32 v1, v3, v12
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v12, v1
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_saveexec_b32 s12, s3
	s_cbranch_execz .LBB7_18
; %bb.17:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v11, v1 offset:128
.LBB7_18:
	s_or_b32 exec_lo, exec_lo, s12
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s12, s0
	s_cbranch_execz .LBB7_23
; %bb.19:
	s_add_i32 s0, s1, 31
	v_mov_b32_e32 v1, 0
	s_lshr_b32 s0, s0, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_u32_e32 vcc_lo, s0, v9
	s_and_saveexec_b32 s0, vcc_lo
; %bb.20:
	v_lshlrev_b32_e32 v1, 2, v9
	ds_load_b32 v1, v1 offset:128
; %bb.21:
	s_or_b32 exec_lo, exec_lo, s0
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v3, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB7_23
; %bb.22:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1 offset:128
.LBB7_23:
	s_or_b32 exec_lo, exec_lo, s12
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, s2
	s_cbranch_execz .LBB7_28
; %bb.24:
	v_mov_b32_e32 v1, 0
	s_add_u32 s0, s10, s16
	s_addc_u32 s2, s11, s17
	s_mov_b32 s3, 0
	ds_load_b32 v1, v1 offset:128
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v2, null, v10, v10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v3, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, -v2, v3, 1.0
	v_fmac_f32_e32 v3, v4, v3
	v_div_scale_f32 v4, vcc_lo, v1, v10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v6, v4, v3
	v_fma_f32 v7, -v2, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v7, v3
	v_fma_f32 v2, -v2, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v3, v6
	v_div_fixup_f32 v6, v2, v10, v1
.LBB7_25:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB7_26 Depth 2
	v_ashrrev_i32_e32 v1, 31, v0
	s_mov_b32 s10, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 2, v[0:1]
	v_add_co_u32 v3, vcc_lo, s6, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s7, v2, vcc_lo
	v_add_co_u32 v7, vcc_lo, s8, v1
	v_add_co_ci_u32_e64 v8, null, s9, v2, vcc_lo
	v_add_co_u32 v9, vcc_lo, s4, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s5, v2, vcc_lo
	global_load_b32 v3, v[3:4], off
	global_load_b32 v4, v[7:8], off
	global_load_b32 v7, v[9:10], off
	s_waitcnt vmcnt(2)
	v_mul_f32_e32 v8, v5, v3
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v11, v5, v4
	v_add_co_u32 v3, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s2, v2, vcc_lo
	s_waitcnt vmcnt(0)
	v_fma_f32 v7, -v6, v8, v7
	v_add_co_u32 v1, vcc_lo, s14, v1
	v_add_co_ci_u32_e64 v2, null, s15, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f32_e32 v7, v11, v7
	global_store_b32 v[3:4], v7, off
	global_load_b32 v3, v[9:10], off
	global_load_b32 v4, v[1:2], off
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v7, v8, v3
.LBB7_26:                               ;   Parent Loop BB7_25 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v3, v4, v7
	global_atomic_cmpswap_b32 v3, v[1:2], v[3:4], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v3, v4
	v_mov_b32_e32 v4, v3
	s_or_b32 s10, vcc_lo, s10
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s10
	s_cbranch_execnz .LBB7_26
; %bb.27:                               ;   in Loop: Header=BB7_25 Depth=1
	s_or_b32 exec_lo, exec_lo, s10
	v_add_nc_u32_e32 v0, s1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s13, v0
	s_or_b32 s3, vcc_lo, s3
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execnz .LBB7_25
.LBB7_28:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_
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
		.amdhsa_next_free_vgpr 17
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
.Lfunc_end7:
	.size	_Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_, .Lfunc_end7-_Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_
                                        ; -- End function
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.num_vgpr, 17
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.num_agpr, 0
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.numbered_sgpr, 20
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.num_named_barrier, 0
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.private_seg_size, 0
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.uses_vcc, 1
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.uses_flat_scratch, 0
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.has_dyn_sized_stack, 0
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.has_recursion, 0
	.set _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1676
; TotalNumSgprs: 22
; NumVgprs: 17
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 22
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
	.protected	_Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii ; -- Begin function _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii
	.globl	_Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii
	.p2align	8
	.type	_Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii,@function
_Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii: ; @_Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii
; %bb.0:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x54
	s_load_b256 s[4:11], s[0:1], 0x10
	s_load_b128 s[12:15], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s14, s3, 0xffff
	s_mul_i32 s3, s9, s8
	v_mad_u64_u32 v[1:2], null, s2, s14, v[0:1]
	s_mul_i32 s2, s3, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s5
	s_mul_i32 s2, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s13
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB8_6
; %bb.1:
	s_mul_i32 s2, s3, s5
	s_abs_i32 s14, s3
	s_abs_i32 s4, s2
	v_cvt_f32_u32_e32 v5, s14
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s8, 0, s4
	v_sub_nc_u32_e32 v3, 0, v1
	s_clause 0x1
	s_load_b128 s[20:23], s[0:1], 0x30
	s_load_b128 s[16:19], s[0:1], 0x0
	v_rcp_iflag_f32_e32 v0, v0
	v_rcp_iflag_f32_e32 v5, v5
	v_max_i32_e32 v3, v1, v3
	s_waitcnt_depctr 0xfff
	v_dual_mul_f32 v0, 0x4f7ffffe, v0 :: v_dual_mul_f32 v5, 0x4f7ffffe, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_cvt_u32_f32_e32 v5, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mul_lo_u32 v2, s8, v0
	s_mul_i32 s8, s13, s12
	s_abs_i32 s12, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_f32_u32_e32 v4, s12
	v_mul_hi_u32 v2, v0, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v4, v4
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v6, s4, v2
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v6 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s2, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	v_add_nc_u32_e32 v6, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s4, 0, s12
	v_mul_lo_u32 v2, s4, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v6, vcc_lo
	s_sub_i32 s4, 0, s14
	v_xor_b32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v2, v4, v2
	v_sub_nc_u32_e32 v0, v0, v3
	v_mul_lo_u32 v3, s4, v5
	s_abs_i32 s4, s9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_sub_nc_u32_e32 v6, 0, v0
	v_mul_lo_u32 v7, v0, s2
	v_add_nc_u32_e32 v2, v4, v2
	s_abs_i32 s2, s13
	v_mul_hi_u32 v3, v5, v3
	v_max_i32_e32 v4, v0, v6
	v_cvt_f32_u32_e32 v8, s2
	s_sub_i32 s15, 0, s2
	v_sub_nc_u32_e32 v6, v1, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_hi_u32 v2, v4, v2
	v_rcp_iflag_f32_e32 v8, v8
	v_add_nc_u32_e32 v3, v5, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, 0, v6
	v_mul_lo_u32 v5, v2, s12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_max_i32_e32 v7, v6, v7
	v_add_nc_u32_e32 v9, 1, v2
	v_mul_hi_u32 v3, v7, v3
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v4, v4, v5
	v_cvt_f32_u32_e32 v5, s4
	v_subrev_nc_u32_e32 v10, s12, v4
	v_cmp_le_u32_e32 vcc_lo, s12, v4
	v_mul_lo_u32 v11, v3, s14
	s_delay_alu instid0(VALU_DEP_4)
	v_rcp_iflag_f32_e32 v12, v5
	v_cndmask_b32_e32 v2, v2, v9, vcc_lo
	v_cndmask_b32_e32 v4, v4, v10, vcc_lo
	v_xor_b32_e32 v9, s8, v0
	v_add_nc_u32_e32 v10, 1, v3
	v_sub_nc_u32_e32 v7, v7, v11
	v_add_nc_u32_e32 v5, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s12, v4
	v_ashrrev_i32_e32 v9, 31, v9
	v_mul_f32_e32 v4, 0x4f7ffffe, v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v2, v2, v5, vcc_lo
	v_subrev_nc_u32_e32 v5, s14, v7
	v_cmp_le_u32_e32 vcc_lo, s14, v7
	v_cvt_u32_f32_e32 v8, v4
	v_xor_b32_e32 v4, s3, v6
	v_xor_b32_e32 v2, v2, v9
	v_cndmask_b32_e32 v3, v3, v10, vcc_lo
	v_cndmask_b32_e32 v7, v7, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_sub_nc_u32_e32 v5, v2, v9
	v_ashrrev_i32_e32 v9, 31, v4
	v_add_nc_u32_e32 v10, 1, v3
	s_delay_alu instid0(VALU_DEP_4)
	v_cmp_le_u32_e32 vcc_lo, s14, v7
	v_mul_lo_u32 v2, s15, v8
	v_mul_lo_u32 v4, v5, s8
	v_mul_f32_e32 v7, 0x4f7ffffe, v12
	s_sub_i32 s8, 0, s4
	v_cndmask_b32_e32 v3, v3, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cvt_u32_f32_e32 v7, v7
	v_mul_hi_u32 v2, v8, v2
	v_xor_b32_e32 v3, v3, v9
	v_sub_nc_u32_e32 v4, v0, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v3, v9
	v_sub_nc_u32_e32 v9, 0, v4
	v_mul_lo_u32 v3, s8, v7
	v_add_nc_u32_e32 v2, v8, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_lo_u32 v10, v0, s3
	v_max_i32_e32 v8, v4, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, v7, v3
	v_mul_hi_u32 v2, v8, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v6, v6, v10
	v_sub_nc_u32_e32 v9, 0, v6
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_nc_u32_e32 v3, v7, v3
	v_mul_lo_u32 v7, v2, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v9, v6, v9
	v_mul_hi_u32 v3, v9, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v7, v8, v7
	v_add_nc_u32_e32 v8, 1, v2
	v_subrev_nc_u32_e32 v10, s2, v7
	v_cmp_le_u32_e32 vcc_lo, s2, v7
	v_mul_lo_u32 v11, v3, s4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v2, v2, v8 :: v_dual_cndmask_b32 v7, v7, v10
	v_xor_b32_e32 v8, s13, v4
	v_sub_nc_u32_e32 v9, v9, v11
	v_add_nc_u32_e32 v11, 1, v3
	s_delay_alu instid0(VALU_DEP_4)
	v_add_nc_u32_e32 v10, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v7
	v_ashrrev_i32_e32 v8, 31, v8
	v_subrev_nc_u32_e32 v7, s4, v9
	s_mov_b32 s2, exec_lo
	v_cndmask_b32_e32 v2, v2, v10, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s4, v9
	v_xor_b32_e32 v10, s9, v6
	v_cndmask_b32_e32 v3, v3, v11, vcc_lo
	v_cndmask_b32_e32 v7, v9, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v10, 31, v10
	v_add_nc_u32_e32 v9, 1, v3
	v_xor_b32_e32 v2, v2, v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s4, v7
	v_sub_nc_u32_e32 v8, v2, v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, v3, v9, vcc_lo
	v_mul_lo_u32 v3, v8, s10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v2, v2, v10
	v_sub_nc_u32_e32 v9, v2, v10
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v7, s20, v3
	v_mad_u64_u32 v[2:3], null, v9, s22, v[7:8]
	v_mov_b32_e32 v7, 0
	s_delay_alu instid0(VALU_DEP_2)
	v_cmpx_lt_i32_e32 -1, v2
	s_cbranch_execz .LBB8_5
; %bb.2:
	v_mul_lo_u32 v3, v8, s13
	v_cmp_gt_i32_e64 s1, s6, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v4, v3
	v_mul_lo_u32 v4, v9, s9
	v_mul_lo_u32 v3, v3, s11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v7, v6, v4
	v_subrev_nc_u32_e32 v6, s21, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[3:4], null, v7, s23, v[6:7]
	v_mov_b32_e32 v7, 0
	v_cmp_lt_i32_e32 vcc_lo, -1, v3
	v_cmp_gt_i32_e64 s0, s7, v3
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s1, s1, s0
	s_and_saveexec_b32 s0, s1
	s_cbranch_execz .LBB8_4
; %bb.3:
	v_mad_u64_u32 v[6:7], null, v5, s5, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[4:5], null, v6, s6, v[2:3]
	v_mad_u64_u32 v[5:6], null, v4, s7, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v6, 31, v5
	v_lshlrev_b64 v[2:3], 2, v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s16, v2
	v_add_co_ci_u32_e64 v3, null, s17, v3, vcc_lo
	global_load_b32 v7, v[2:3], off
.LBB8_4:
	s_or_b32 exec_lo, exec_lo, s0
.LBB8_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s2
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s18, v0
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b32 v[0:1], v7, off
.LBB8_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii
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
		.amdhsa_next_free_vgpr 13
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
.Lfunc_end8:
	.size	_Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii, .Lfunc_end8-_Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii
                                        ; -- End function
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.num_vgpr, 13
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.num_agpr, 0
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.numbered_sgpr, 24
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.num_named_barrier, 0
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.private_seg_size, 0
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.uses_vcc, 1
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.uses_flat_scratch, 0
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.has_dyn_sized_stack, 0
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.has_recursion, 0
	.set _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1192
; TotalNumSgprs: 26
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 26
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
	.protected	_Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii ; -- Begin function _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii
	.globl	_Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii
	.p2align	8
	.type	_Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii,@function
_Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii: ; @_Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii
; %bb.0:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x54
	s_load_b256 s[4:11], s[0:1], 0x10
	s_load_b128 s[12:15], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s14, s3, 0xffff
	s_mul_i32 s3, s9, s8
	v_mad_u64_u32 v[1:2], null, s2, s14, v[0:1]
	s_mul_i32 s2, s3, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s5
	s_mul_i32 s2, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s13
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB9_5
; %bb.1:
	s_mul_i32 s2, s3, s5
	s_abs_i32 s14, s3
	s_abs_i32 s4, s2
	v_cvt_f32_u32_e32 v5, s14
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s8, 0, s4
	v_sub_nc_u32_e32 v3, 0, v1
	s_load_b128 s[16:19], s[0:1], 0x30
	v_rcp_iflag_f32_e32 v5, v5
	v_rcp_iflag_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_max_i32_e32 v3, v1, v3
	s_waitcnt_depctr 0xfff
	v_dual_mul_f32 v5, 0x4f7ffffe, v5 :: v_dual_mul_f32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v5, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s8, v0
	s_mul_i32 s8, s13, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_abs_i32 s12, s8
	v_cvt_f32_u32_e32 v4, s12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v2, v0, v2
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	v_cvt_u32_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s4
	v_sub_nc_u32_e32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v6, s4, v2
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	v_dual_cndmask_b32 v2, v2, v6 :: v_dual_add_nc_u32 v3, 1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s2, v1
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v6, 1, v0
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s4, 0, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v2, s4, v4
	v_cndmask_b32_e32 v0, v0, v6, vcc_lo
	s_sub_i32 s4, 0, s14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_xor_b32_e32 v0, v0, v3
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v0, v3
	v_mul_lo_u32 v3, s4, v5
	s_abs_i32 s4, s9
	v_sub_nc_u32_e32 v6, 0, v0
	v_mul_lo_u32 v7, v0, s2
	v_add_nc_u32_e32 v2, v4, v2
	s_abs_i32 s2, s13
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_mul_hi_u32 v3, v5, v3
	v_max_i32_e32 v4, v0, v6
	v_cvt_f32_u32_e32 v8, s2
	s_sub_i32 s15, 0, s2
	v_sub_nc_u32_e32 v6, v1, v7
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_rcp_iflag_f32_e32 v8, v8
	v_add_nc_u32_e32 v3, v5, v3
	v_sub_nc_u32_e32 v7, 0, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v5, v2, s12
	v_max_i32_e32 v7, v6, v7
	v_add_nc_u32_e32 v9, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_hi_u32 v3, v7, v3
	v_sub_nc_u32_e32 v4, v4, v5
	v_cvt_f32_u32_e32 v5, s4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_subrev_nc_u32_e32 v10, s12, v4
	v_cmp_le_u32_e32 vcc_lo, s12, v4
	v_mul_lo_u32 v11, v3, s14
	v_rcp_iflag_f32_e32 v12, v5
	v_cndmask_b32_e32 v2, v2, v9, vcc_lo
	v_cndmask_b32_e32 v4, v4, v10, vcc_lo
	v_xor_b32_e32 v9, s8, v0
	v_add_nc_u32_e32 v10, 1, v3
	v_sub_nc_u32_e32 v7, v7, v11
	v_add_nc_u32_e32 v5, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s12, v4
	v_ashrrev_i32_e32 v9, 31, v9
	v_mul_f32_e32 v4, 0x4f7ffffe, v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v2, v2, v5, vcc_lo
	v_subrev_nc_u32_e32 v5, s14, v7
	v_cmp_le_u32_e32 vcc_lo, s14, v7
	v_cvt_u32_f32_e32 v8, v4
	v_xor_b32_e32 v4, s3, v6
	v_xor_b32_e32 v2, v2, v9
	v_cndmask_b32_e32 v3, v3, v10, vcc_lo
	v_cndmask_b32_e32 v7, v7, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_sub_nc_u32_e32 v5, v2, v9
	v_ashrrev_i32_e32 v9, 31, v4
	v_add_nc_u32_e32 v10, 1, v3
	s_delay_alu instid0(VALU_DEP_4)
	v_cmp_le_u32_e32 vcc_lo, s14, v7
	v_mul_lo_u32 v2, s15, v8
	v_mul_lo_u32 v4, v5, s8
	v_mul_f32_e32 v7, 0x4f7ffffe, v12
	s_sub_i32 s8, 0, s4
	v_cndmask_b32_e32 v3, v3, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cvt_u32_f32_e32 v7, v7
	v_mul_hi_u32 v2, v8, v2
	v_xor_b32_e32 v3, v3, v9
	v_sub_nc_u32_e32 v4, v0, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v3, v9
	v_sub_nc_u32_e32 v9, 0, v4
	v_mul_lo_u32 v3, s8, v7
	v_add_nc_u32_e32 v2, v8, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_lo_u32 v10, v0, s3
	v_max_i32_e32 v8, v4, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, v7, v3
	v_mul_hi_u32 v2, v8, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v6, v6, v10
	v_sub_nc_u32_e32 v9, 0, v6
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_nc_u32_e32 v3, v7, v3
	v_mul_lo_u32 v7, v2, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v9, v6, v9
	v_mul_hi_u32 v3, v9, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v7, v8, v7
	v_add_nc_u32_e32 v8, 1, v2
	v_subrev_nc_u32_e32 v10, s2, v7
	v_cmp_le_u32_e32 vcc_lo, s2, v7
	v_mul_lo_u32 v11, v3, s4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v2, v2, v8 :: v_dual_cndmask_b32 v7, v7, v10
	v_xor_b32_e32 v8, s13, v4
	v_sub_nc_u32_e32 v9, v9, v11
	v_add_nc_u32_e32 v11, 1, v3
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_add_nc_u32_e32 v10, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v7
	v_ashrrev_i32_e32 v8, 31, v8
	v_subrev_nc_u32_e32 v7, s4, v9
	v_cndmask_b32_e32 v2, v2, v10, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s4, v9
	v_xor_b32_e32 v10, s9, v6
	v_cndmask_b32_e32 v3, v3, v11, vcc_lo
	v_cndmask_b32_e32 v9, v9, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v10, 31, v10
	v_add_nc_u32_e32 v11, 1, v3
	v_xor_b32_e32 v2, v2, v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s4, v9
	v_sub_nc_u32_e32 v7, v2, v8
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, v3, v11, vcc_lo
	v_mul_lo_u32 v3, v7, s10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v2, v2, v10
	v_sub_nc_u32_e32 v8, v2, v10
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v9, s16, v3
	v_mad_u64_u32 v[2:3], null, v8, s18, v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_lt_i32_e32 vcc_lo, -1, v2
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB9_5
; %bb.2:
	v_mul_lo_u32 v3, v7, s13
	v_cmp_gt_i32_e64 s3, s6, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v4, v3
	v_mul_lo_u32 v4, v8, s9
	v_mul_lo_u32 v3, v3, s11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v7, v6, v4
	v_subrev_nc_u32_e32 v6, s17, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, v7, s19, v[6:7]
	v_cmp_lt_i32_e32 vcc_lo, -1, v3
	v_cmp_gt_i32_e64 s2, s7, v3
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s2, s3, s2
	s_and_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB9_5
; %bb.3:
	v_mad_u64_u32 v[6:7], null, v5, s5, v[0:1]
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mad_u64_u32 v[4:5], null, v6, s6, v[2:3]
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	v_mad_u64_u32 v[5:6], null, v4, s7, v[3:4]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_u32 v4, vcc_lo, s0, v0
	v_ashrrev_i32_e32 v6, 31, v5
	s_mov_b32 s0, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[2:3], 2, v[5:6]
	v_add_co_ci_u32_e64 v5, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v3, vcc_lo
	global_load_b32 v4, v[4:5], off
	global_load_b32 v3, v[0:1], off
.LBB9_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v3, v4
	global_atomic_cmpswap_b32 v2, v[0:1], v[2:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v2, v3
	v_mov_b32_e32 v3, v2
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB9_4
.LBB9_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii
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
		.amdhsa_next_free_vgpr 13
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
.Lfunc_end9:
	.size	_Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii, .Lfunc_end9-_Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii
                                        ; -- End function
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.num_vgpr, 13
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.num_agpr, 0
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.numbered_sgpr, 20
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.num_named_barrier, 0
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.private_seg_size, 0
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.uses_vcc, 1
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.uses_flat_scratch, 0
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.has_dyn_sized_stack, 0
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.has_recursion, 0
	.set _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1216
; TotalNumSgprs: 22
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 22
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
	.protected	_Z30attn_embedding_backward_kernelPKfPKiPfii ; -- Begin function _Z30attn_embedding_backward_kernelPKfPKiPfii
	.globl	_Z30attn_embedding_backward_kernelPKfPKiPfii
	.p2align	8
	.type	_Z30attn_embedding_backward_kernelPKfPKiPfii,@function
_Z30attn_embedding_backward_kernelPKfPKiPfii: ; @_Z30attn_embedding_backward_kernelPKfPKiPfii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[4:5], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB10_3
; %bb.1:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
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
	v_xor_b32_e32 v3, s5, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v2, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v3, 31, v2
	v_mul_lo_u32 v0, v2, s5
	v_lshlrev_b64 v[3:4], 2, v[2:3]
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v0, v1, v0
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s10, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v4, null, s11, v4, vcc_lo
	global_load_b32 v5, v[3:4], off
	s_waitcnt vmcnt(0)
	v_mad_u64_u32 v[3:4], null, v5, s5, v[0:1]
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[2:3], 2, v[3:4]
	v_add_co_u32 v4, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v5, null, s9, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v3, vcc_lo
	global_load_b32 v4, v[4:5], off
	global_load_b32 v3, v[0:1], off
	s_mov_b32 s0, 0
.LBB10_2:                               ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v3, v4
	global_atomic_cmpswap_b32 v2, v[0:1], v[2:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v2, v3
	v_mov_b32_e32 v3, v2
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB10_2
.LBB10_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z30attn_embedding_backward_kernelPKfPKiPfii
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
.Lfunc_end10:
	.size	_Z30attn_embedding_backward_kernelPKfPKiPfii, .Lfunc_end10-_Z30attn_embedding_backward_kernelPKfPKiPfii
                                        ; -- End function
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.num_vgpr, 6
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.num_agpr, 0
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.numbered_sgpr, 12
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.num_named_barrier, 0
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.private_seg_size, 0
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.uses_vcc, 1
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.uses_flat_scratch, 0
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.has_dyn_sized_stack, 0
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.has_recursion, 0
	.set _Z30attn_embedding_backward_kernelPKfPKiPfii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 452
; TotalNumSgprs: 14
; NumVgprs: 6
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 6
; Occupancy: 16
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	_Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i ; -- Begin function _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i
	.globl	_Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i
	.p2align	8
	.type	_Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i,@function
_Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i: ; @_Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i
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
	s_cbranch_execz .LBB11_2
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s9, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v5, null, s5, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	global_load_b32 v3, v[4:5], off
	s_load_b32 s0, s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e64 v6, 1.0, s0
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v7, s0, v2
	v_add_co_u32 v2, vcc_lo, s10, v0
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v7, v6, v3
	v_add_co_ci_u32_e64 v3, null, s11, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b32 v[4:5], v7, off
	global_load_b32 v2, v[2:3], off
	global_load_b32 v3, v[0:1], off
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v2, s0, v2
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v2, v6, v3
	global_store_b32 v[0:1], v2, off
.LBB11_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i
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
.Lfunc_end11:
	.size	_Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i, .Lfunc_end11-_Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i
                                        ; -- End function
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.num_vgpr, 8
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.num_agpr, 0
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.numbered_sgpr, 12
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.num_named_barrier, 0
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.private_seg_size, 0
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.uses_vcc, 1
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.uses_flat_scratch, 0
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.has_dyn_sized_stack, 0
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.has_recursion, 0
	.set _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 276
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
	.protected	_Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii ; -- Begin function _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii
	.globl	_Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii
	.p2align	8
	.type	_Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii,@function
_Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii: ; @_Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii
; %bb.0:
	s_clause 0x3
	s_load_b32 s33, s[0:1], 0x34
	s_load_b64 s[18:19], s[0:1], 0x2c
	s_load_b64 s[16:17], s[0:1], 0x20
	s_load_b256 s[8:15], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s6, s33
	s_abs_i32 s20, s19
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s7, 0, s6
	s_xor_b32 s1, s19, s33
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s5, v1
	v_lshl_add_u32 v1, s2, 6, v0
	s_mul_i32 s7, s7, s5
	s_mul_hi_u32 s7, s5, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s5, s7
	s_mul_hi_u32 s0, s20, s5
	s_ashr_i32 s5, s1, 31
	s_mul_i32 s7, s0, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s1, s20, s7
	s_add_i32 s7, s0, 1
	s_sub_i32 s20, s1, s6
	s_cmp_ge_u32 s1, s6
	s_cselect_b32 s0, s7, s0
	s_cselect_b32 s1, s20, s1
	s_add_i32 s2, s0, 1
	s_cmp_ge_u32 s1, s6
	s_cselect_b32 s1, s2, s0
	v_cmp_gt_i32_e64 s0, s18, v1
	s_xor_b32 s2, s1, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s48, s2, s5
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB12_8
; %bb.1:
	s_cmp_lt_i32 s48, 1
	s_cbranch_scc1 .LBB12_8
; %bb.2:
	v_ashrrev_i32_e32 v2, 31, v1
	s_ashr_i32 s7, s19, 31
	s_mul_i32 s6, s48, s3
	s_mov_b32 s20, 0
	s_mul_i32 s24, s2, 0x600
	v_mad_i64_i32 v[3:4], null, s18, s4, v[1:2]
	v_mul_lo_u32 v2, v0, s48
	s_mul_i32 s25, s5, 0x600
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v8, v4, s19
	v_mul_lo_u32 v9, v3, s7
	s_ashr_i32 s7, s6, 31
	s_cmp_lt_u32 s48, 8
	s_cbranch_scc1 .LBB12_5
; %bb.3:
	v_mad_u64_u32 v[4:5], null, v3, s19, 0
	s_lshl_b64 s[26:27], s[6:7], 3
	s_and_b32 s20, s48, 0x7ffffff8
	s_sub_i32 s21, s24, s25
	s_add_u32 s23, s8, s26
	s_addc_u32 s26, s9, s27
	s_mov_b32 s22, 0
	v_add3_u32 v5, v5, v9, v8
	v_lshl_add_u32 v10, v2, 3, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	v_add_co_u32 v4, vcc_lo, s23, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s26, v5, vcc_lo
	s_mov_b32 s23, s22
	v_add_co_u32 v4, vcc_lo, v4, 56
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	v_dual_mov_b32 v6, s22 :: v_dual_mov_b32 v7, s23
.LBB12_4:                               ; =>This Inner Loop Header: Depth=1
	s_clause 0x3
	global_load_b128 v[11:14], v[4:5], off offset:-56
	global_load_b128 v[15:18], v[4:5], off offset:-40
	global_load_b128 v[19:22], v[4:5], off offset:-24
	global_load_b128 v[23:26], v[4:5], off offset:-8
	v_add_nc_u32_e32 v27, s21, v10
	v_add_co_u32 v4, vcc_lo, v4, 64
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_add_i32 s22, s22, 8
	s_waitcnt vmcnt(3)
	ds_store_b64 v10, v[11:12]
	ds_store_b64 v27, v[6:7]
	ds_store_b64 v10, v[13:14] offset:8
	ds_store_b64 v27, v[6:7] offset:8
	s_waitcnt vmcnt(2)
	ds_store_b64 v10, v[15:16] offset:16
	ds_store_b64 v27, v[6:7] offset:16
	ds_store_b64 v10, v[17:18] offset:24
	ds_store_b64 v27, v[6:7] offset:24
	s_waitcnt vmcnt(1)
	ds_store_b64 v10, v[19:20] offset:32
	ds_store_b64 v27, v[6:7] offset:32
	ds_store_b64 v10, v[21:22] offset:40
	ds_store_b64 v27, v[6:7] offset:40
	s_waitcnt vmcnt(0)
	ds_store_b64 v10, v[23:24] offset:48
	ds_store_b64 v27, v[6:7] offset:48
	ds_store_b64 v10, v[25:26] offset:56
	v_add_nc_u32_e32 v10, 64, v10
	s_cmp_lg_u32 s20, s22
	ds_store_b64 v27, v[6:7] offset:56
	s_cbranch_scc1 .LBB12_4
.LBB12_5:
	s_and_b32 s22, s48, 7
	s_mov_b32 s21, 0
	s_cmp_eq_u32 s22, 0
	s_cbranch_scc1 .LBB12_8
; %bb.6:
	v_mad_u64_u32 v[4:5], null, v3, s19, 0
	s_sub_i32 s23, s24, s25
	s_lshl_b32 s24, s20, 3
	s_lshl_b64 s[20:21], s[20:21], 3
	s_lshl_b64 s[6:7], s[6:7], 3
	s_add_u32 s8, s8, s20
	v_dual_mov_b32 v2, 0 :: v_dual_lshlrev_b32 v3, 3, v2
	v_add3_u32 v5, v5, v9, v8
	s_addc_u32 s9, s9, s21
	s_add_u32 s6, s8, s6
	s_addc_u32 s7, s9, s7
	v_add3_u32 v6, 0, s24, v3
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	v_mov_b32_e32 v3, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s6, v4
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
.LBB12_7:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[7:8], v[4:5], off
	v_add_co_u32 v4, vcc_lo, v4, 8
	v_add_nc_u32_e32 v9, s23, v6
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_add_i32 s22, s22, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lg_u32 s22, 0
	s_waitcnt vmcnt(0)
	ds_store_b64 v6, v[7:8]
	v_add_nc_u32_e32 v6, 8, v6
	ds_store_b64 v9, v[2:3]
	s_cbranch_scc1 .LBB12_7
.LBB12_8:
	s_or_b32 exec_lo, exec_lo, s1
	s_lshl_b32 s1, s48, 9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s49, s1, 0
	s_add_i32 s50, s49, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s7, s50, s1
	s_cmp_lt_i32 s18, 1
	s_cbranch_scc1 .LBB12_24
; %bb.9:
	v_cvt_f64_i32_e32 v[2:3], s48
	s_lshl_b32 s51, s48, 6
	s_mul_i32 s8, s48, s3
	s_mov_b32 s22, 0xfefa39ef
	s_mov_b32 s24, 0x3b39803f
	s_mov_b32 s26, 0xfca7ab0c
	s_mov_b32 s28, 0x6a5dcb37
	s_mov_b32 s30, 0x623fde64
	s_mov_b32 s34, 0x7c89e6b0
	s_mov_b32 s36, 0x14761f6e
	s_mov_b32 s38, 0x1852b7b0
	s_mov_b32 s40, 0x11122322
	s_mov_b32 s42, 0x555502a1
	s_mov_b32 s44, 0x55555511
	s_mov_b32 s46, 11
	s_mov_b32 s56, 0
	s_mul_i32 s57, s18, s4
	s_mov_b32 s58, s19
	s_mov_b32 s23, 0xbfe62e42
	s_mov_b32 s25, 0xbc7abc9e
	s_mov_b32 s27, 0x3e928af3
	s_mov_b32 s29, 0x3e5ade15
	s_mov_b32 s31, 0x3ec71dee
	s_mov_b32 s35, 0x3efa0199
	s_mov_b32 s37, 0x3f2a01a0
	s_mov_b32 s39, 0x3f56c16c
	s_mov_b32 s41, 0x3f811111
	s_mov_b32 s43, 0x3fa55555
	s_mov_b32 s45, 0x3fc55555
	s_mov_b32 s47, 0x3fe00000
	s_mov_b32 s62, s18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[2:3]
	s_and_b32 s1, vcc_lo, exec_lo
	s_cselect_b32 s1, 0x100, 0
	v_ldexp_f64 v[2:3], v[2:3], s1
	s_cselect_b32 s1, 0xffffff80, 0
	s_ashr_i32 s6, s4, 31
	s_ashr_i32 s52, s19, 31
	s_mul_i32 s6, s18, s6
	s_ashr_i32 s9, s8, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[4:5], v[2:3]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[6:7], v[2:3], v[4:5]
	v_mul_f64 v[4:5], v[4:5], 0.5
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[4:5], v[4:5], v[8:9], v[4:5]
	v_fma_f64 v[8:9], -v[6:7], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[8:9], v[4:5], v[6:7]
	v_fma_f64 v[8:9], -v[6:7], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[8:9], v[4:5], v[6:7]
	v_ldexp_f64 v[4:5], v[4:5], s1
	s_mul_hi_u32 s1, s18, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s53, s1, s6
	s_cmp_gt_i32 s48, 0
	s_cselect_b32 s54, -1, 0
	s_abs_i32 s55, s48
	s_lshl_b32 s6, s5, 9
	s_sub_i32 s1, 0, s55
	s_lshl_b32 s20, s5, 10
	s_lshl_b32 s5, s5, 3
	s_ashr_i32 s59, s48, 31
	s_sub_i32 s60, 0, s48
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v3, v5, v3 :: v_dual_cndmask_b32 v2, v4, v2
	v_div_scale_f64 v[4:5], null, v[2:3], v[2:3], 1.0
	v_div_scale_f64 v[10:11], vcc_lo, 1.0, v[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[6:7]
	v_fma_f64 v[4:5], -v[4:5], v[8:9], v[10:11]
	v_cvt_f32_u32_e32 v10, s55
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[8:9]
	v_rcp_iflag_f32_e32 v6, v10
	v_mul_lo_u32 v7, s48, v0
	v_lshlrev_b32_e32 v9, 3, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_lshl_add_u32 v10, s2, 9, v9
	v_lshl_add_u32 v9, s2, 10, v9
	s_waitcnt_depctr 0xfff
	v_dual_mul_f32 v6, 0x4f7ffffe, v6 :: v_dual_lshlrev_b32 v7, 3, v7
	s_lshl_b32 s2, s2, 3
	v_subrev_nc_u32_e32 v10, s6, v10
	v_subrev_nc_u32_e32 v9, s20, v9
	s_delay_alu instid0(VALU_DEP_3)
	v_add_nc_u32_e32 v13, 0, v7
	v_cvt_u32_f32_e32 v6, v6
	v_add_nc_u32_e32 v14, s7, v7
	s_mov_b32 s20, 0x652b82fe
	s_sub_i32 s61, s2, s5
	s_mov_b32 s21, 0x3ff71547
	v_mul_lo_u32 v8, s1, v6
	v_cmp_gt_i32_e64 s1, s51, v0
	v_add_nc_u32_e32 v16, 0, v10
	v_add_nc_u32_e32 v17, 0, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v8, v6, v8
	v_add_nc_u32_e32 v15, v6, v8
	v_div_fixup_f64 v[7:8], v[4:5], v[2:3], 1.0
	v_mov_b32_e32 v3, 0
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, 0x85ebc8a0
	v_mov_b32_e32 v6, 0xffe1ccf3
	s_branch .LBB12_11
.LBB12_10:                              ;   in Loop: Header=BB12_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s63
	s_add_i32 s56, s56, 64
	s_sub_i32 s62, s62, 64
	s_cmp_ge_i32 s56, s18
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB12_25
.LBB12_11:                              ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB12_14 Depth 2
                                        ;     Child Loop BB12_19 Depth 2
                                        ;       Child Loop BB12_21 Depth 3
                                        ;       Child Loop BB12_23 Depth 3
	s_and_saveexec_b32 s2, s1
	s_cbranch_execz .LBB12_16
; %bb.12:                               ;   in Loop: Header=BB12_11 Depth=1
	v_dual_mov_b32 v9, v17 :: v_dual_mov_b32 v10, v16
	v_mov_b32_e32 v2, v0
	s_mov_b32 s5, 0
	s_branch .LBB12_14
.LBB12_13:                              ;   in Loop: Header=BB12_14 Depth=2
	s_or_b32 exec_lo, exec_lo, s6
	v_add_nc_u32_e32 v2, 64, v2
	v_add_nc_u32_e32 v10, 0x200, v10
	v_add_nc_u32_e32 v9, 0x200, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s51, v2
	s_or_b32 s5, vcc_lo, s5
	s_and_not1_b32 exec_lo, exec_lo, s5
	s_cbranch_execz .LBB12_16
.LBB12_14:                              ;   Parent Loop BB12_11 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v11, v2, v15
	s_mov_b32 s6, exec_lo
	v_mul_lo_u32 v12, v11, s55
	v_add_nc_u32_e32 v18, 1, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v12, v2, v12
	v_subrev_nc_u32_e32 v19, s55, v12
	v_cmp_le_u32_e32 vcc_lo, s55, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v11, v11, v18 :: v_dual_cndmask_b32 v12, v12, v19
	v_add_nc_u32_e32 v18, 1, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s55, v12
	v_cndmask_b32_e32 v11, v11, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v11, s59, v11
	v_subrev_nc_u32_e32 v11, s59, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v12, s56, v11
	v_cmpx_gt_i32_e64 s18, v12
	s_cbranch_execz .LBB12_13
; %bb.15:                               ;   in Loop: Header=BB12_14 Depth=2
	v_ashrrev_i32_e32 v18, 31, v12
	v_add_co_u32 v12, vcc_lo, s57, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v20, null, s53, v18, vcc_lo
	v_mul_lo_u32 v22, v12, s52
	v_mad_u64_u32 v[18:19], null, v12, s58, s[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v12, v20, s58
	v_mad_u64_u32 v[20:21], null, s60, v11, v[2:3]
	v_add3_u32 v12, v12, v19, v22
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v11, vcc_lo, v18, v20
	v_add_co_ci_u32_e64 v12, null, 0, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[11:12], 3, v[11:12]
	v_add_co_u32 v18, vcc_lo, s10, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v19, null, s11, v12, vcc_lo
	v_add_co_u32 v11, vcc_lo, s12, v11
	v_add_co_ci_u32_e64 v12, null, s13, v12, vcc_lo
	global_load_b64 v[18:19], v[18:19], off
	global_load_b64 v[11:12], v[11:12], off
	s_waitcnt vmcnt(1)
	ds_store_b64 v10, v[18:19]
	s_waitcnt vmcnt(0)
	ds_store_b64 v9, v[11:12]
	s_branch .LBB12_13
.LBB12_16:                              ;   in Loop: Header=BB12_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	s_cmp_gt_i32 s18, s56
	s_waitcnt lgkmcnt(0)
	s_cselect_b32 s2, -1, 0
	s_barrier
	s_and_b32 s2, s0, s2
	buffer_gl0_inv
	s_and_saveexec_b32 s63, s2
	s_cbranch_execz .LBB12_10
; %bb.17:                               ;   in Loop: Header=BB12_11 Depth=1
	v_med3_i32 v2, s62, 1, 64
	s_mov_b32 s64, 0
	s_mov_b32 s65, s50
	s_mov_b32 s66, s49
	s_branch .LBB12_19
.LBB12_18:                              ;   in Loop: Header=BB12_19 Depth=2
	v_fma_f64 v[3:4], v[3:4], v[5:6], v[11:12]
	s_add_i32 s64, s64, 1
	v_dual_mov_b32 v5, v9 :: v_dual_mov_b32 v6, v10
	v_cmp_eq_u32_e32 vcc_lo, s64, v2
	s_add_i32 s66, s66, s61
	s_add_i32 s65, s65, s61
	s_cbranch_vccnz .LBB12_10
.LBB12_19:                              ;   Parent Loop BB12_11 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB12_21 Depth 3
                                        ;       Child Loop BB12_23 Depth 3
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v12, 0
	s_and_not1_b32 vcc_lo, exec_lo, s54
	s_cbranch_vccnz .LBB12_22
; %bb.20:                               ;   in Loop: Header=BB12_19 Depth=2
	v_mov_b32_e32 v9, v13
	s_mov_b32 s2, s66
	s_mov_b32 s5, s48
.LBB12_21:                              ;   Parent Loop BB12_11 Depth=1
                                        ;     Parent Loop BB12_19 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_mov_b32_e32 v10, s2
	s_add_i32 s5, s5, -1
	s_add_i32 s2, s2, 8
	s_cmp_eq_u32 s5, 0
	ds_load_b64 v[18:19], v9
	ds_load_b64 v[20:21], v10
	v_add_nc_u32_e32 v9, 8, v9
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[11:12], v[18:19], v[20:21], v[11:12]
	s_cbranch_scc0 .LBB12_21
.LBB12_22:                              ;   in Loop: Header=BB12_19 Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[7:8], v[11:12]
	v_cmp_gt_f64_e32 vcc_lo, v[9:10], v[5:6]
	v_dual_cndmask_b32 v10, v6, v10 :: v_dual_cndmask_b32 v9, v5, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_fma_f64 v[11:12], v[7:8], v[11:12], -v[9:10]
	v_mul_f64 v[18:19], v[5:6], s[20:21]
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[20:21], v[11:12], s[20:21]
	v_cmp_nlt_f64_e64 s5, 0x40900000, v[11:12]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[5:6]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[5:6]
	v_cmp_ngt_f64_e64 s6, 0xc090cc00, v[11:12]
	v_rndne_f64_e32 v[18:19], v[18:19]
	v_rndne_f64_e32 v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[18:19], s[22:23], v[5:6]
	v_fma_f64 v[24:25], v[20:21], s[22:23], v[11:12]
	v_cvt_i32_f64_e32 v30, v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], v[18:19], s[24:25], v[22:23]
	v_fma_f64 v[24:25], v[20:21], s[24:25], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], s[28:29], s[26:27]
	v_fma_f64 v[28:29], v[24:25], s[28:29], s[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[30:31]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[34:35]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[36:37]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[38:39]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[38:39]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[40:41]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[40:41]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[42:43]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[42:43]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[44:45]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[44:45]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[46:47]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[46:47]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], 1.0
	v_fma_f64 v[28:29], v[24:25], v[28:29], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[22:23], v[26:27], 1.0
	v_cvt_i32_f64_e32 v22, v[20:21]
	v_fma_f64 v[20:21], v[24:25], v[28:29], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[18:19], v[18:19], v30
	v_ldexp_f64 v[20:21], v[20:21], v22
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v19, 0x7ff00000, v19, vcc_lo
	s_and_b32 vcc_lo, s2, vcc_lo
	v_cndmask_b32_e64 v5, 0x7ff00000, v21, s5
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v12, 0, v5, s6
	v_cndmask_b32_e32 v5, 0, v18, vcc_lo
	s_and_b32 vcc_lo, s6, s5
	v_mov_b32_e32 v18, v14
	v_cndmask_b32_e64 v6, 0, v19, s2
	v_cndmask_b32_e32 v11, 0, v20, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s54
	s_mov_b32 s2, s65
	s_mov_b32 s5, s48
	s_cbranch_vccnz .LBB12_18
	.p2align	6
.LBB12_23:                              ;   Parent Loop BB12_11 Depth=1
                                        ;     Parent Loop BB12_19 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_mov_b32_e32 v19, s2
	ds_load_b64 v[21:22], v18
	s_add_i32 s5, s5, -1
	s_add_i32 s2, s2, 8
	s_cmp_eq_u32 s5, 0
	ds_load_b64 v[19:20], v19
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[19:20], v[11:12], v[19:20]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[19:20], v[5:6], v[21:22], v[19:20]
	ds_store_b64 v18, v[19:20]
	v_add_nc_u32_e32 v18, 8, v18
	s_cbranch_scc0 .LBB12_23
	s_branch .LBB12_18
.LBB12_24:
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v6, 0xffe1ccf3
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, 0x85ebc8a0
.LBB12_25:
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB12_32
; %bb.26:
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_lt_f64_e64 s0, 0, v[3:4]
	v_ashrrev_i32_e32 v2, 31, v1
	s_ashr_i32 s1, s18, 31
	s_cmp_lt_i32 s48, 1
	s_cbranch_scc1 .LBB12_29
; %bb.27:
	v_div_scale_f64 v[7:8], null, v[3:4], v[3:4], 1.0
	v_div_scale_f64 v[13:14], vcc_lo, 1.0, v[3:4], 1.0
	s_ashr_i32 s2, s19, 31
	s_mul_i32 s8, s48, s3
	v_mul_lo_u32 v0, s48, v0
	s_ashr_i32 s9, s8, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_lshl_b64 s[8:9], s[8:9], 3
	v_lshl_add_u32 v0, v0, 3, s7
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[9:10], v[7:8], v[3:4], 1.0
	v_mad_i64_i32 v[7:8], null, s18, s4, v[1:2]
	v_mul_lo_u32 v8, v8, s19
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v13, v7, s2
	v_mad_u64_u32 v[11:12], null, v7, s19, 0
	v_add3_u32 v12, v12, v13, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[7:8], 3, v[11:12]
	v_add_co_u32 v7, vcc_lo, s14, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v8, null, s15, v8, vcc_lo
	v_add_co_u32 v7, vcc_lo, v7, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s9, v8, vcc_lo
	v_cndmask_b32_e64 v10, 0, v10, s0
	v_cndmask_b32_e64 v9, 0, v9, s0
.LBB12_28:                              ; =>This Inner Loop Header: Depth=1
	ds_load_b64 v[11:12], v0
	v_add_nc_u32_e32 v0, 8, v0
	s_add_i32 s48, s48, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s48, 0
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[11:12], v[9:10], v[11:12]
	global_store_b64 v[7:8], v[11:12], off
	v_add_co_u32 v7, vcc_lo, v7, 8
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	s_cbranch_scc0 .LBB12_28
.LBB12_29:
	v_mov_b32_e32 v7, 0x85ebc8a0
	v_mov_b32_e32 v8, 0xffe1ccf3
	s_and_saveexec_b32 s2, s0
	s_cbranch_execz .LBB12_31
; %bb.30:
	v_frexp_mant_f64_e32 v[7:8], v[3:4]
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[7:8]
	s_mov_b32 s6, 0x55555780
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_ldexp_f64 v[7:8], v[7:8], v0
	v_frexp_exp_i32_f64_e32 v0, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[7:8], 1.0
	v_add_f64 v[15:16], v[7:8], -1.0
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[3:4]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
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
	v_fma_f64 v[11:12], v[11:12], v[15:16], s[6:7]
	v_ldexp_f64 v[15:16], v[9:10], 1
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0x3fe62e42
	v_mul_f64 v[11:12], v[17:18], v[11:12]
	v_cvt_f64_i32_e32 v[17:18], v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_add_f64 v[13:14], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[19:20], v[17:18], s[6:7]
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[13:14], -v[15:16]
	v_fma_f64 v[15:16], v[17:18], s[6:7], -v[19:20]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[11:12], -v[9:10]
	v_fma_f64 v[11:12], v[17:18], s[6:7], v[15:16]
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
	v_add_f64 v[15:16], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[9:10], -v[23:24]
	v_add_f64 v[9:10], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[15:16], -v[11:12]
	v_add_f64 v[9:10], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[19:20], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[15:16]
	v_add_f64 v[13:14], v[19:20], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[11:12]
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_add_f64 v[7:8], v[19:20], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0x7ff00000, v8, vcc_lo
	v_cndmask_b32_e32 v3, 0, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[7:8], v[5:6], v[3:4]
.LBB12_31:
	s_or_b32 exec_lo, exec_lo, s2
	s_mul_i32 s0, s33, s4
	s_ashr_i32 s2, s3, 31
	s_add_u32 s0, s0, s3
	s_mul_hi_i32 s3, s33, s4
	s_mul_i32 s1, s0, s1
	s_mul_hi_u32 s4, s0, s18
	s_addc_u32 s2, s3, s2
	s_add_i32 s1, s4, s1
	s_mul_i32 s2, s2, s18
	s_mul_i32 s0, s0, s18
	s_add_i32 s1, s1, s2
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_lshl_b64 s[0:1], s[0:1], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_add_u32 s0, s16, s0
	s_addc_u32 s1, s17, s1
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[7:8], off
.LBB12_32:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii
		.amdhsa_group_segment_fixed_size 0
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
		.amdhsa_system_sgpr_workgroup_id_z 1
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 31
		.amdhsa_next_free_sgpr 67
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
		.amdhsa_inst_pref_size 32
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
	.size	_Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii, .Lfunc_end12-_Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii
                                        ; -- End function
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.num_vgpr, 31
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.num_agpr, 0
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.numbered_sgpr, 67
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.num_named_barrier, 0
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.private_seg_size, 0
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.uses_vcc, 1
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.uses_flat_scratch, 0
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.has_dyn_sized_stack, 0
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.has_recursion, 0
	.set _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4084
; TotalNumSgprs: 69
; NumVgprs: 31
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 69
; NumVGPRsForWavesPerEU: 31
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 1
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	_Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii ; -- Begin function _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii
	.globl	_Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii
	.p2align	8
	.type	_Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii,@function
_Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii: ; @_Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s5, s[0:1], 0x34
	s_load_b128 s[8:11], s[0:1], 0x1c
	s_waitcnt lgkmcnt(0)
	s_and_b32 s5, s5, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s5, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB13_6
; %bb.1:
	s_abs_i32 s2, s10
	s_abs_i32 s7, s9
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s6, 0, s2
	s_clause 0x1
	s_load_b128 s[12:15], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s5, v0
	s_mul_i32 s6, s6, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s6, s5, s6
	s_add_i32 s5, s5, s6
	s_xor_b32 s6, s9, s10
	s_mul_hi_u32 s5, s7, s5
	s_ashr_i32 s6, s6, 31
	s_mul_i32 s11, s5, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s7, s7, s11
	s_add_i32 s11, s5, 1
	s_sub_i32 s16, s7, s2
	s_cmp_ge_u32 s7, s2
	s_cselect_b32 s5, s11, s5
	s_cselect_b32 s7, s16, s7
	s_add_i32 s11, s5, 1
	s_cmp_ge_u32 s7, s2
	s_cselect_b32 s2, s11, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b32 s2, s2, s6
	s_sub_i32 s5, s2, s6
	s_ashr_i32 s2, s8, 31
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc1 .LBB13_4
; %bb.2:
	v_mad_i64_i32 v[3:4], null, s8, s4, v[1:2]
	s_mul_i32 s6, s5, s3
	s_ashr_i32 s11, s9, 31
	s_ashr_i32 s7, s6, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[5:6], null, v3, s9, s[6:7]
	v_mul_lo_u32 v0, v4, s9
	v_mul_lo_u32 v3, v3, s11
	v_add3_u32 v6, v0, v6, v3
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[7:8], 3, v[5:6]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v5, vcc_lo, s14, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s15, v8, vcc_lo
	v_add_co_u32 v7, vcc_lo, s12, v7
	v_add_co_ci_u32_e64 v8, null, s13, v8, vcc_lo
	.p2align	6
.LBB13_3:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[9:10], v[5:6], off
	global_load_b64 v[11:12], v[7:8], off
	v_add_co_u32 v5, vcc_lo, v5, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	v_add_co_u32 v7, vcc_lo, v7, 8
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	s_add_i32 s5, s5, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s5, 0
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[9:10], v[11:12], v[3:4]
	s_cbranch_scc0 .LBB13_3
	s_branch .LBB13_5
.LBB13_4:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB13_5:
	s_mul_i32 s5, s10, s4
	s_ashr_i32 s6, s3, 31
	s_add_u32 s5, s5, s3
	s_mul_hi_i32 s3, s10, s4
	s_mul_i32 s2, s5, s2
	s_mul_hi_u32 s4, s5, s8
	s_addc_u32 s3, s3, s6
	s_add_i32 s2, s4, s2
	s_mul_i32 s3, s3, s8
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_add_i32 s3, s2, s3
	s_mul_i32 s2, s5, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	s_lshl_b64 s[2:3], s[2:3], 3
	s_waitcnt lgkmcnt(0)
	s_add_u32 s0, s0, s2
	s_addc_u32 s1, s1, s3
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB13_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii
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
		.amdhsa_system_sgpr_workgroup_id_y 1
		.amdhsa_system_sgpr_workgroup_id_z 1
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
.Lfunc_end13:
	.size	_Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii, .Lfunc_end13-_Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii
                                        ; -- End function
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.num_vgpr, 13
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.num_agpr, 0
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.numbered_sgpr, 17
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.num_named_barrier, 0
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.private_seg_size, 0
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.uses_vcc, 1
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.uses_flat_scratch, 0
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.has_dyn_sized_stack, 0
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.has_recursion, 0
	.set _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 532
; TotalNumSgprs: 19
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
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
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 1
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	_Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii ; -- Begin function _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii
	.globl	_Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii
	.p2align	8
	.type	_Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii,@function
_Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii: ; @_Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s31, s[0:1], 0x44
	s_load_b64 s[6:7], s[0:1], 0x3c
	v_mov_b32_e32 v3, 0
	s_load_b256 s[16:23], s[0:1], 0x0
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v4, 0
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s5, s31
	s_abs_i32 s24, s7
	v_cvt_f32_u32_e32 v1, s5
	s_sub_i32 s9, 0, s5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_readfirstlane_b32 s8, v1
	v_lshl_add_u32 v1, s2, 6, v0
	s_mul_i32 s9, s9, s8
	v_ashrrev_i32_e32 v2, 31, v1
	s_mul_hi_u32 s9, s8, s9
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s25, s8, s9
	s_load_b256 s[8:15], s[0:1], 0x20
	s_mul_hi_u32 s0, s24, s25
	s_xor_b32 s1, s7, s31
	s_waitcnt lgkmcnt(0)
	s_mul_i32 s14, s0, s5
	s_ashr_i32 s30, s1, 31
	s_sub_i32 s1, s24, s14
	s_add_i32 s14, s0, 1
	s_sub_i32 s15, s1, s5
	s_cmp_ge_u32 s1, s5
	s_cselect_b32 s0, s14, s0
	s_cselect_b32 s1, s15, s1
	s_add_i32 s2, s0, 1
	s_cmp_ge_u32 s1, s5
	s_cselect_b32 s1, s2, s0
	v_cmp_gt_i32_e64 s0, s6, v1
	s_xor_b32 s2, s1, s30
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_sub_i32 s5, s2, s30
	s_mul_i32 s14, s5, s3
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB14_9
; %bb.1:
	s_ashr_i32 s33, s6, 31
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc1 .LBB14_8
; %bb.2:
	v_mad_i64_i32 v[5:6], null, s6, s4, v[1:2]
	s_ashr_i32 s24, s7, 31
	s_ashr_i32 s15, s14, 31
	v_mul_lo_u32 v9, v0, s5
	s_cmp_lt_u32 s5, 8
	v_mad_u64_u32 v[3:4], null, v5, s7, s[14:15]
	v_mul_lo_u32 v6, v6, s7
	v_mul_lo_u32 v5, v5, s24
	s_mov_b32 s15, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_add3_u32 v4, v6, v4, v5
	s_cbranch_scc1 .LBB14_5
; %bb.3:
	s_lshl_b32 s24, s2, 11
	s_lshl_b32 s25, s30, 11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_lshlrev_b64 v[5:6], 3, v[3:4]
	s_sub_i32 s34, s24, s25
	s_mov_b32 s24, 0
	v_lshl_add_u32 v10, v9, 3, 0
	s_mov_b32 s25, s24
	v_dual_mov_b32 v7, s24 :: v_dual_mov_b32 v8, s25
	s_lshl_b32 s26, s2, 9
	s_lshl_b32 s27, s30, 9
	s_and_b32 s15, s5, 0x7ffffff8
	s_sub_i32 s25, s26, s27
	s_mov_b64 s[26:27], s[16:17]
	s_mov_b64 s[28:29], s[22:23]
.LBB14_4:                               ; =>This Inner Loop Header: Depth=1
	v_add_co_u32 v35, vcc_lo, s26, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v36, null, s27, v6, vcc_lo
	v_add_co_u32 v39, vcc_lo, s28, v5
	v_add_co_ci_u32_e64 v40, null, s29, v6, vcc_lo
	global_load_b128 v[11:14], v[35:36], off
	global_load_b128 v[15:18], v[39:40], off
	global_load_b128 v[19:22], v[35:36], off offset:16
	global_load_b128 v[23:26], v[39:40], off offset:16
	global_load_b128 v[27:30], v[35:36], off offset:32
	global_load_b128 v[31:34], v[39:40], off offset:32
	global_load_b128 v[35:38], v[35:36], off offset:48
	global_load_b128 v[39:42], v[39:40], off offset:48
	v_add_nc_u32_e32 v43, s25, v10
	v_add_nc_u32_e32 v44, s34, v10
	s_add_i32 s24, s24, 8
	s_add_u32 s28, s28, 64
	s_addc_u32 s29, s29, 0
	s_add_u32 s26, s26, 64
	s_addc_u32 s27, s27, 0
	s_cmp_eq_u32 s15, s24
	s_waitcnt vmcnt(7)
	ds_store_b64 v10, v[11:12]
	s_waitcnt vmcnt(6)
	ds_store_b64 v43, v[15:16]
	ds_store_b64 v44, v[7:8]
	ds_store_b64 v10, v[13:14] offset:8
	ds_store_b64 v43, v[17:18] offset:8
	ds_store_b64 v44, v[7:8] offset:8
	s_waitcnt vmcnt(5)
	ds_store_b64 v10, v[19:20] offset:16
	s_waitcnt vmcnt(4)
	ds_store_b64 v43, v[23:24] offset:16
	ds_store_b64 v44, v[7:8] offset:16
	ds_store_b64 v10, v[21:22] offset:24
	ds_store_b64 v43, v[25:26] offset:24
	ds_store_b64 v44, v[7:8] offset:24
	s_waitcnt vmcnt(3)
	ds_store_b64 v10, v[27:28] offset:32
	s_waitcnt vmcnt(2)
	ds_store_b64 v43, v[31:32] offset:32
	ds_store_b64 v44, v[7:8] offset:32
	ds_store_b64 v10, v[29:30] offset:40
	ds_store_b64 v43, v[33:34] offset:40
	ds_store_b64 v44, v[7:8] offset:40
	s_waitcnt vmcnt(1)
	ds_store_b64 v10, v[35:36] offset:48
	s_waitcnt vmcnt(0)
	ds_store_b64 v43, v[39:40] offset:48
	ds_store_b64 v44, v[7:8] offset:48
	ds_store_b64 v10, v[37:38] offset:56
	v_add_nc_u32_e32 v10, 64, v10
	ds_store_b64 v43, v[41:42] offset:56
	ds_store_b64 v44, v[7:8] offset:56
	s_cbranch_scc0 .LBB14_4
.LBB14_5:
	s_and_b32 s25, s5, 7
	s_mov_b32 s24, 0
	s_cmp_eq_u32 s25, 0
	s_cbranch_scc1 .LBB14_8
; %bb.6:
	v_lshlrev_b32_e32 v5, 3, v9
	s_lshl_b32 s27, s15, 3
	s_lshl_b32 s29, s2, 9
	v_add_co_u32 v3, vcc_lo, v3, s15
	s_delay_alu instid0(VALU_DEP_2)
	v_add3_u32 v7, s27, s29, v5
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	s_lshl_b32 s26, s2, 11
	s_lshl_b32 s15, s30, 9
	v_add3_u32 v6, s27, s26, v5
	v_subrev_nc_u32_e32 v10, s15, v7
	v_lshlrev_b64 v[7:8], 3, v[3:4]
	s_lshl_b32 s28, s30, 11
	v_add3_u32 v11, 0, s27, v5
	v_subrev_nc_u32_e32 v6, s28, v6
	v_dual_mov_b32 v5, 0 :: v_dual_add_nc_u32 v10, 0, v10
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s22, v7
	v_add_co_ci_u32_e64 v4, null, s23, v8, vcc_lo
	v_add_co_u32 v7, vcc_lo, s16, v7
	s_delay_alu instid0(VALU_DEP_4)
	v_dual_mov_b32 v6, v5 :: v_dual_add_nc_u32 v9, 0, v6
	v_add_co_ci_u32_e64 v8, null, s17, v8, vcc_lo
	s_lshl_b32 s15, s25, 3
	.p2align	6
.LBB14_7:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[12:13], v[7:8], off
	global_load_b64 v[14:15], v[3:4], off
	v_add_co_u32 v3, vcc_lo, v3, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_add_co_u32 v7, vcc_lo, v7, 8
	v_add_nc_u32_e32 v16, s24, v11
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_add_nc_u32_e32 v17, s24, v10
	v_add_nc_u32_e32 v18, s24, v9
	s_add_i32 s24, s24, 8
	s_waitcnt vmcnt(1)
	ds_store_b64 v16, v[12:13]
	s_waitcnt vmcnt(0)
	ds_store_b64 v17, v[14:15]
	ds_store_b64 v18, v[5:6]
	s_cmp_lg_u32 s15, s24
	s_cbranch_scc1 .LBB14_7
.LBB14_8:
	s_mul_i32 s15, s31, s4
	s_ashr_i32 s16, s3, 31
	s_add_u32 s3, s15, s3
	s_mul_hi_i32 s15, s31, s4
	v_mad_u64_u32 v[3:4], null, s3, s6, v[1:2]
	s_addc_u32 s15, s15, s16
	s_mul_i32 s3, s3, s33
	s_mul_i32 s15, s15, s6
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add3_u32 v4, s3, s15, v4
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s8, v3
	v_add_co_ci_u32_e64 v6, null, s9, v4, vcc_lo
	v_add_co_u32 v7, vcc_lo, s10, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s11, v4, vcc_lo
	global_load_b64 v[3:4], v[5:6], off
	global_load_b64 v[5:6], v[7:8], off
.LBB14_9:
	s_or_b32 exec_lo, exec_lo, s1
	s_lshl_b32 s1, s5, 9
	v_mul_lo_u32 v13, s5, v0
	s_add_i32 s33, s1, 0
	s_mov_b32 s45, 0
	s_add_i32 s33, s33, s1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s44, s33, s1
	s_add_i32 s3, s44, s1
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB14_26
; %bb.10:
	v_cvt_f64_i32_e32 v[7:8], s5
	s_lshl_b32 s46, s5, 6
	s_mov_b32 s16, 0x3b39803f
	s_mov_b32 s22, 0xfca7ab0c
	s_mov_b32 s24, 0x6a5dcb37
	s_mov_b32 s26, 0x623fde64
	s_mov_b32 s28, 0x7c89e6b0
	s_mov_b32 s34, 0x1852b7b0
	s_mov_b32 s36, 0x11122322
	s_mov_b32 s38, 0x555502a1
	s_mov_b32 s40, 0x55555511
	s_mov_b32 s42, 11
	s_mul_i32 s51, s6, s4
	s_mov_b32 s52, s7
	s_mov_b32 s17, 0xbc7abc9e
	s_mov_b32 s23, 0x3e928af3
	s_mov_b32 s25, 0x3e5ade15
	s_mov_b32 s27, 0x3ec71dee
	s_mov_b32 s29, 0x3efa0199
	s_mov_b32 s35, 0x3f56c16c
	s_mov_b32 s37, 0x3f811111
	s_mov_b32 s39, 0x3fa55555
	s_mov_b32 s41, 0x3fc55555
	s_mov_b32 s43, 0x3fe00000
	s_mov_b32 s57, s6
	v_lshlrev_b32_e32 v18, 3, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[7:8]
	s_and_b32 s1, vcc_lo, exec_lo
	s_cselect_b32 s1, 0x100, 0
	v_ldexp_f64 v[7:8], v[7:8], s1
	s_cselect_b32 s1, 0xffffff80, 0
	s_ashr_i32 s8, s4, 31
	s_ashr_i32 s47, s7, 31
	s_mul_i32 s8, s6, s8
	s_ashr_i32 s15, s14, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[9:10], v[7:8]
	v_cmp_class_f64_e64 vcc_lo, v[7:8], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[11:12], v[7:8], v[9:10]
	v_mul_f64 v[9:10], v[9:10], 0.5
	v_fma_f64 v[14:15], -v[9:10], v[11:12], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[11:12], v[14:15], v[11:12]
	v_fma_f64 v[9:10], v[9:10], v[14:15], v[9:10]
	v_fma_f64 v[14:15], -v[11:12], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[14:15], v[9:10], v[11:12]
	v_fma_f64 v[14:15], -v[11:12], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[14:15], v[9:10], v[11:12]
	v_ldexp_f64 v[9:10], v[9:10], s1
	s_mul_hi_u32 s1, s6, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s48, s1, s8
	s_cmp_gt_i32 s5, 0
	s_mul_i32 s8, s30, 0x600
	s_cselect_b32 s49, -1, 0
	s_abs_i32 s50, s5
	s_lshl_b32 s9, s30, 10
	s_sub_i32 s1, 0, s50
	s_lshl_b32 s10, s30, 9
	s_ashr_i32 s53, s5, 31
	s_sub_i32 s54, 0, s5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v8, v10, v8 :: v_dual_cndmask_b32 v7, v9, v7
	v_div_scale_f64 v[9:10], null, v[7:8], v[7:8], 1.0
	v_div_scale_f64 v[16:17], vcc_lo, 1.0, v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[11:12], v[9:10]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[11:12], v[14:15], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[11:12], v[14:15], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[16:17], v[11:12]
	v_fma_f64 v[9:10], -v[9:10], v[14:15], v[16:17]
	v_cvt_f32_u32_e32 v16, s50
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[9:10], v[9:10], v[11:12], v[14:15]
	v_rcp_iflag_f32_e32 v11, v16
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v11, 0x4f7ffffe, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v12, v11
	v_lshlrev_b32_e32 v11, 3, v0
	v_mul_lo_u32 v14, s1, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_mad_u64_u32 v[15:16], null, 0x600, s2, v[11:12]
	v_lshl_add_u32 v11, s2, 10, v11
	v_cmp_gt_i32_e64 s1, s46, v0
	v_mul_hi_u32 v16, v12, v14
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_subrev_nc_u32_e32 v17, s8, v15
	v_subrev_nc_u32_e32 v11, s9, v11
	v_add_nc_u32_e32 v14, s3, v18
	s_lshl_b32 s8, s2, 3
	s_lshl_b32 s9, s30, 3
	v_add_nc_u32_e32 v17, 0, v17
	s_lshl_b32 s2, s2, 9
	v_add_nc_u32_e32 v15, v12, v16
	v_div_fixup_f64 v[7:8], v[9:10], v[7:8], 1.0
	v_add_nc_u32_e32 v16, 0, v11
	v_add_nc_u32_e32 v18, 0, v18
	s_sub_i32 s55, s8, s9
	s_sub_i32 s56, s2, s10
	s_mov_b32 s8, 0x652b82fe
	s_mov_b32 s10, 0xfefa39ef
	s_mov_b32 s30, 0x14761f6e
	s_mov_b32 s9, 0x3ff71547
	s_mov_b32 s11, 0xbfe62e42
	s_mov_b32 s31, 0x3f2a01a0
	s_branch .LBB14_12
.LBB14_11:                              ;   in Loop: Header=BB14_12 Depth=1
	s_or_b32 exec_lo, exec_lo, s58
	s_add_i32 s45, s45, 64
	s_sub_i32 s57, s57, 64
	s_cmp_ge_i32 s45, s6
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB14_26
.LBB14_12:                              ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB14_15 Depth 2
                                        ;     Child Loop BB14_20 Depth 2
                                        ;       Child Loop BB14_22 Depth 3
                                        ;       Child Loop BB14_25 Depth 3
	s_and_saveexec_b32 s2, s1
	s_cbranch_execz .LBB14_17
; %bb.13:                               ;   in Loop: Header=BB14_12 Depth=1
	v_dual_mov_b32 v10, v17 :: v_dual_mov_b32 v11, v16
	v_mov_b32_e32 v9, v0
	s_mov_b32 s58, 0
	s_branch .LBB14_15
.LBB14_14:                              ;   in Loop: Header=BB14_15 Depth=2
	s_or_b32 exec_lo, exec_lo, s59
	v_add_nc_u32_e32 v9, 64, v9
	v_add_nc_u32_e32 v11, 0x200, v11
	v_add_nc_u32_e32 v10, 0x200, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s46, v9
	s_or_b32 s58, vcc_lo, s58
	s_and_not1_b32 exec_lo, exec_lo, s58
	s_cbranch_execz .LBB14_17
.LBB14_15:                              ;   Parent Loop BB14_12 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v12, v9, v15
	s_mov_b32 s59, exec_lo
	v_mul_lo_u32 v19, v12, s50
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v19, v9, v19
	v_subrev_nc_u32_e32 v21, s50, v19
	v_cmp_le_u32_e32 vcc_lo, s50, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v19, v19, v21 :: v_dual_add_nc_u32 v20, 1, v12
	v_cndmask_b32_e32 v12, v12, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s50, v19
	v_add_nc_u32_e32 v20, 1, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v12, v12, v20, vcc_lo
	v_xor_b32_e32 v12, s53, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v12, s53, v12
	v_add_nc_u32_e32 v19, s45, v12
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s6, v19
	s_cbranch_execz .LBB14_14
; %bb.16:                               ;   in Loop: Header=BB14_15 Depth=2
	v_ashrrev_i32_e32 v20, 31, v19
	v_add_co_u32 v21, vcc_lo, s51, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v22, null, s48, v20, vcc_lo
	v_mul_lo_u32 v23, v21, s47
	v_mad_u64_u32 v[19:20], null, v21, s52, s[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v24, v22, s52
	v_mad_u64_u32 v[21:22], null, s54, v12, v[9:10]
	v_add3_u32 v12, v24, v20, v23
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v19, vcc_lo, v19, v21
	v_add_co_ci_u32_e64 v20, null, 0, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[19:20], 3, v[19:20]
	v_add_co_u32 v21, vcc_lo, s18, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v22, null, s19, v20, vcc_lo
	v_add_co_u32 v19, vcc_lo, s20, v19
	v_add_co_ci_u32_e64 v20, null, s21, v20, vcc_lo
	global_load_b64 v[21:22], v[21:22], off
	global_load_b64 v[19:20], v[19:20], off
	s_waitcnt vmcnt(1)
	ds_store_b64 v11, v[21:22]
	s_waitcnt vmcnt(0)
	ds_store_b64 v10, v[19:20]
	s_branch .LBB14_14
.LBB14_17:                              ;   in Loop: Header=BB14_12 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	s_cmp_gt_i32 s6, s45
	s_waitcnt vmcnt(0) lgkmcnt(0)
	s_cselect_b32 s2, -1, 0
	s_barrier
	s_and_b32 s2, s0, s2
	buffer_gl0_inv
	s_and_saveexec_b32 s58, s2
	s_cbranch_execz .LBB14_11
; %bb.18:                               ;   in Loop: Header=BB14_12 Depth=1
	v_med3_i32 v19, s57, 1, 64
	s_mov_b32 s59, 0
	s_mov_b32 s60, s33
	s_mov_b32 s61, s44
	s_branch .LBB14_20
.LBB14_19:                              ;   in Loop: Header=BB14_20 Depth=2
	s_add_i32 s59, s59, 1
	s_add_i32 s61, s61, s55
	v_cmp_eq_u32_e32 vcc_lo, s59, v19
	s_add_i32 s60, s60, s55
	s_cbranch_vccnz .LBB14_11
.LBB14_20:                              ;   Parent Loop BB14_12 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB14_22 Depth 3
                                        ;       Child Loop BB14_25 Depth 3
	v_mov_b32_e32 v9, 0
	v_dual_mov_b32 v10, 0 :: v_dual_mov_b32 v11, 0
	v_mov_b32_e32 v12, 0
	s_and_not1_b32 vcc_lo, exec_lo, s49
	s_cbranch_vccnz .LBB14_23
; %bb.21:                               ;   in Loop: Header=BB14_20 Depth=2
	v_mov_b32_e32 v20, v18
	s_mov_b32 s2, s60
	s_mov_b32 s62, s61
	s_mov_b32 s63, s5
	.p2align	6
.LBB14_22:                              ;   Parent Loop BB14_12 Depth=1
                                        ;     Parent Loop BB14_20 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_mov_b32_e32 v23, s2
	v_add_nc_u32_e32 v25, s56, v20
	v_mov_b32_e32 v27, s62
	s_add_i32 s63, s63, -1
	ds_load_b64 v[21:22], v20
	ds_load_b64 v[23:24], v23
	ds_load_b64 v[25:26], v25
	ds_load_b64 v[27:28], v27
	v_add_nc_u32_e32 v20, 8, v20
	s_add_i32 s62, s62, 8
	s_add_i32 s2, s2, 8
	s_cmp_eq_u32 s63, 0
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[11:12], v[21:22], v[23:24], v[11:12]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[9:10], v[25:26], v[27:28], v[9:10]
	s_cbranch_scc0 .LBB14_22
.LBB14_23:                              ;   in Loop: Header=BB14_20 Depth=2
	s_and_not1_b32 vcc_lo, exec_lo, s49
	s_cbranch_vccnz .LBB14_19
; %bb.24:                               ;   in Loop: Header=BB14_20 Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[7:8], v[11:12], -v[3:4]
	v_add_f64 v[9:10], v[9:10], -v[5:6]
	s_mov_b32 s62, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[20:21], v[11:12], s[8:9]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[11:12]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[11:12]
	v_rndne_f64_e32 v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[20:21], s[10:11], v[11:12]
	v_cvt_i32_f64_e32 v26, v[20:21]
	v_fma_f64 v[22:23], v[20:21], s[16:17], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[24:25], v[22:23], s[24:25], s[22:23]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[28:29]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[34:35]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[38:39]
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[42:43]
	v_fma_f64 v[24:25], v[22:23], v[24:25], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[22:23], v[24:25], 1.0
	v_ldexp_f64 v[20:21], v[20:21], v26
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v21, 0x7ff00000, v21, vcc_lo
	s_and_b32 vcc_lo, s2, vcc_lo
	v_cndmask_b32_e32 v11, 0, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v12, 0, v21, s2
	s_mov_b32 s2, 0
	v_mul_f64 v[9:10], v[9:10], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[9:10], v[7:8], v[9:10]
.LBB14_25:                              ;   Parent Loop BB14_12 Depth=1
                                        ;     Parent Loop BB14_20 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	s_add_i32 s63, s60, s2
	v_add_nc_u32_e32 v22, s2, v14
	v_mov_b32_e32 v20, s63
	s_add_i32 s62, s62, -1
	s_add_i32 s2, s2, 8
	s_cmp_eq_u32 s62, 0
	ds_load_b64 v[11:12], v22
	ds_load_b64 v[20:21], v20
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[11:12], v[9:10], v[20:21], v[11:12]
	ds_store_b64 v22, v[11:12]
	s_cbranch_scc0 .LBB14_25
	s_branch .LBB14_19
.LBB14_26:
	s_cmp_gt_i32 s5, 0
	s_cselect_b32 s1, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s0, s0, s1
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB14_29
; %bb.27:
	s_waitcnt vmcnt(1)
	v_mad_i64_i32 v[3:4], null, s6, s4, v[1:2]
	s_ashr_i32 s0, s7, 31
	s_ashr_i32 s15, s14, 31
	v_mul_lo_u32 v2, v3, s0
	v_mul_lo_u32 v4, v4, s7
	v_mad_u64_u32 v[0:1], null, v3, s7, 0
	s_lshl_b64 s[0:1], s[14:15], 3
	v_add3_u32 v1, v1, v2, v4
	v_lshl_add_u32 v2, v13, 3, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	v_add_co_u32 v0, vcc_lo, s12, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, s13, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
.LBB14_28:                              ; =>This Inner Loop Header: Depth=1
	ds_load_b64 v[3:4], v2
	v_add_nc_u32_e32 v2, 8, v2
	s_add_i32 s5, s5, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_cmp_lg_u32 s5, 0
	s_waitcnt lgkmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
	v_add_co_u32 v0, vcc_lo, v0, 8
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_cbranch_scc1 .LBB14_28
.LBB14_29:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 72
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
		.amdhsa_system_sgpr_workgroup_id_z 1
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 45
		.amdhsa_next_free_sgpr 64
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
		.amdhsa_inst_pref_size 24
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
	.size	_Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii, .Lfunc_end14-_Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii
                                        ; -- End function
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.num_vgpr, 45
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.num_agpr, 0
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.numbered_sgpr, 64
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.num_named_barrier, 0
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.private_seg_size, 0
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.uses_vcc, 1
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.uses_flat_scratch, 0
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.has_dyn_sized_stack, 0
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.has_recursion, 0
	.set _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3024
; TotalNumSgprs: 66
; NumVgprs: 45
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 66
; NumVGPRsForWavesPerEU: 45
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 1
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	_Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii ; -- Begin function _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii
	.globl	_Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii
	.p2align	8
	.type	_Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii,@function
_Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii: ; @_Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s5, s[0:1], 0x4c
	s_load_b64 s[6:7], s[0:1], 0x44
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s24, s5
	s_abs_i32 s25, s7
	v_cvt_f32_u32_e32 v1, s24
	s_sub_i32 s9, 0, s24
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s8, v1
	v_lshl_add_u32 v1, s2, 6, v0
	s_mul_i32 s9, s9, s8
	s_mul_hi_u32 s9, s8, s9
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s26, s8, s9
	s_load_b512 s[8:23], s[0:1], 0x0
	s_mul_hi_u32 s0, s25, s26
	s_xor_b32 s1, s7, s5
	s_mul_i32 s26, s0, s24
	s_ashr_i32 s50, s1, 31
	s_sub_i32 s1, s25, s26
	s_add_i32 s25, s0, 1
	s_sub_i32 s26, s1, s24
	s_cmp_ge_u32 s1, s24
	s_cselect_b32 s0, s25, s0
	s_cselect_b32 s1, s26, s1
	s_add_i32 s2, s0, 1
	s_cmp_ge_u32 s1, s24
	s_cselect_b32 s1, s2, s0
	v_cmp_gt_i32_e64 s0, s6, v1
	s_xor_b32 s51, s1, s50
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s33, s51, s50
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB15_8
; %bb.1:
	s_cmp_lt_i32 s33, 1
	s_cbranch_scc1 .LBB15_8
; %bb.2:
	v_ashrrev_i32_e32 v2, 31, v1
	s_mul_i32 s24, s33, s3
	s_ashr_i32 s2, s7, 31
	s_ashr_i32 s25, s24, 31
	s_cmp_lt_u32 s33, 8
	v_mad_i64_i32 v[4:5], null, s6, s4, v[1:2]
	s_mul_i32 s30, s51, 0x600
	v_mul_lo_u32 v6, v4, s2
	v_mad_u64_u32 v[2:3], null, v4, s7, s[24:25]
	v_mul_lo_u32 v4, v5, s7
	s_mov_b32 s2, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_add3_u32 v3, v4, v3, v6
	s_cbranch_scc1 .LBB15_5
; %bb.3:
	v_mul_lo_u32 v4, v0, s33
	s_mul_i32 s25, s50, 0x600
	s_mov_b32 s24, 0
	s_sub_i32 s31, s30, s25
	s_mov_b32 s25, s24
	s_lshl_b32 s26, s51, 10
	s_lshl_b32 s27, s50, 10
	v_dual_mov_b32 v6, s24 :: v_dual_mov_b32 v7, s25
	v_lshl_add_u32 v8, v4, 3, 0
	v_lshlrev_b64 v[4:5], 3, v[2:3]
	s_sub_i32 s34, s26, s27
	s_lshl_b32 s26, s51, 9
	s_lshl_b32 s27, s50, 9
	s_and_b32 s2, s33, 0x7ffffff8
	s_sub_i32 s25, s26, s27
	s_waitcnt lgkmcnt(0)
	s_mov_b64 s[26:27], s[10:11]
	s_mov_b64 s[28:29], s[12:13]
.LBB15_4:                               ; =>This Inner Loop Header: Depth=1
	v_add_co_u32 v33, vcc_lo, s26, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v34, null, s27, v5, vcc_lo
	v_add_co_u32 v37, vcc_lo, s28, v4
	v_add_co_ci_u32_e64 v38, null, s29, v5, vcc_lo
	global_load_b128 v[9:12], v[33:34], off
	global_load_b128 v[13:16], v[37:38], off
	global_load_b128 v[17:20], v[33:34], off offset:16
	global_load_b128 v[21:24], v[37:38], off offset:16
	global_load_b128 v[25:28], v[33:34], off offset:32
	global_load_b128 v[29:32], v[37:38], off offset:32
	global_load_b128 v[33:36], v[33:34], off offset:48
	global_load_b128 v[37:40], v[37:38], off offset:48
	v_add_nc_u32_e32 v41, s25, v8
	v_add_nc_u32_e32 v42, s34, v8
	v_add_nc_u32_e32 v43, s31, v8
	s_add_i32 s24, s24, 8
	s_add_u32 s28, s28, 64
	s_addc_u32 s29, s29, 0
	s_add_u32 s26, s26, 64
	s_addc_u32 s27, s27, 0
	s_cmp_lg_u32 s2, s24
	s_waitcnt vmcnt(7)
	ds_store_b64 v8, v[9:10]
	s_waitcnt vmcnt(6)
	ds_store_b64 v41, v[13:14]
	ds_store_b64 v42, v[6:7]
	ds_store_b64 v43, v[6:7]
	ds_store_b64 v8, v[11:12] offset:8
	ds_store_b64 v41, v[15:16] offset:8
	ds_store_b64 v42, v[6:7] offset:8
	ds_store_b64 v43, v[6:7] offset:8
	s_waitcnt vmcnt(5)
	ds_store_b64 v8, v[17:18] offset:16
	s_waitcnt vmcnt(4)
	ds_store_b64 v41, v[21:22] offset:16
	ds_store_b64 v42, v[6:7] offset:16
	ds_store_b64 v43, v[6:7] offset:16
	ds_store_b64 v8, v[19:20] offset:24
	ds_store_b64 v41, v[23:24] offset:24
	ds_store_b64 v42, v[6:7] offset:24
	ds_store_b64 v43, v[6:7] offset:24
	s_waitcnt vmcnt(3)
	ds_store_b64 v8, v[25:26] offset:32
	s_waitcnt vmcnt(2)
	ds_store_b64 v41, v[29:30] offset:32
	ds_store_b64 v42, v[6:7] offset:32
	ds_store_b64 v43, v[6:7] offset:32
	ds_store_b64 v8, v[27:28] offset:40
	ds_store_b64 v41, v[31:32] offset:40
	ds_store_b64 v42, v[6:7] offset:40
	ds_store_b64 v43, v[6:7] offset:40
	s_waitcnt vmcnt(1)
	ds_store_b64 v8, v[33:34] offset:48
	s_waitcnt vmcnt(0)
	ds_store_b64 v41, v[37:38] offset:48
	ds_store_b64 v42, v[6:7] offset:48
	ds_store_b64 v43, v[6:7] offset:48
	ds_store_b64 v8, v[35:36] offset:56
	v_add_nc_u32_e32 v8, 64, v8
	ds_store_b64 v41, v[39:40] offset:56
	ds_store_b64 v42, v[6:7] offset:56
	ds_store_b64 v43, v[6:7] offset:56
	s_cbranch_scc1 .LBB15_4
.LBB15_5:
	s_and_b32 s26, s33, 7
	s_mov_b32 s24, 0
	s_cmp_eq_u32 s26, 0
	s_cbranch_scc1 .LBB15_8
; %bb.6:
	v_mul_lo_u32 v4, v0, s33
	s_lshl_b32 s25, s2, 3
	s_lshl_b32 s28, s51, 10
	v_add_co_u32 v2, vcc_lo, v2, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	s_lshl_b32 s31, s51, 9
	v_lshlrev_b32_e32 v6, 3, v4
	s_mul_i32 s27, s50, 0x600
	s_lshl_b32 s29, s50, 10
	s_lshl_b32 s2, s50, 9
	s_delay_alu instid0(VALU_DEP_1)
	v_add3_u32 v4, s25, s30, v6
	v_add3_u32 v5, s25, s28, v6
	v_add3_u32 v7, s25, s31, v6
	v_add3_u32 v8, 0, s25, v6
	s_mov_b32 s25, s24
	v_subrev_nc_u32_e32 v9, s27, v4
	v_subrev_nc_u32_e32 v10, s29, v5
	v_lshlrev_b64 v[4:5], 3, v[2:3]
	v_subrev_nc_u32_e32 v7, s2, v7
	s_lshl_b32 s2, s26, 3
	v_add_nc_u32_e32 v9, 0, v9
	v_add_nc_u32_e32 v10, 0, v10
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s12, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s13, v5, vcc_lo
	v_add_co_u32 v4, vcc_lo, s10, v4
	v_dual_mov_b32 v6, s24 :: v_dual_add_nc_u32 v11, 0, v7
	v_add_co_ci_u32_e64 v5, null, s11, v5, vcc_lo
	v_mov_b32_e32 v7, s25
	.p2align	6
.LBB15_7:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[12:13], v[4:5], off
	global_load_b64 v[14:15], v[2:3], off
	v_add_co_u32 v2, vcc_lo, v2, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	v_add_co_u32 v4, vcc_lo, v4, 8
	v_add_nc_u32_e32 v16, s24, v8
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	v_add_nc_u32_e32 v17, s24, v11
	v_add_nc_u32_e32 v18, s24, v10
	v_add_nc_u32_e32 v19, s24, v9
	s_add_i32 s24, s24, 8
	s_waitcnt vmcnt(1)
	ds_store_b64 v16, v[12:13]
	s_waitcnt vmcnt(0)
	ds_store_b64 v17, v[14:15]
	ds_store_b64 v18, v[6:7]
	ds_store_b64 v19, v[6:7]
	s_cmp_lg_u32 s2, s24
	s_cbranch_scc1 .LBB15_7
.LBB15_8:
	s_or_b32 exec_lo, exec_lo, s1
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB15_27
; %bb.9:
	v_cvt_f64_i32_e32 v[2:3], s33
	s_waitcnt lgkmcnt(0)
	s_mul_i32 s10, s33, s3
	s_mul_hi_i32 s2, s5, s4
	s_mul_i32 s5, s5, s4
	s_mov_b32 s26, 0xfefa39ef
	s_mov_b32 s28, 0x3b39803f
	s_mov_b32 s30, 0xfca7ab0c
	s_mov_b32 s34, 0x6a5dcb37
	s_mov_b32 s36, 0x623fde64
	s_mov_b32 s38, 0x7c89e6b0
	s_mov_b32 s40, 0x14761f6e
	s_mov_b32 s42, 0x1852b7b0
	s_mov_b32 s44, 0x11122322
	s_mov_b32 s46, 0x555502a1
	s_mov_b32 s48, 0x55555511
	s_mov_b32 s61, 0
	s_mul_i32 s62, s6, s4
	s_mov_b32 s63, s7
	s_mov_b32 s27, 0xbfe62e42
	s_mov_b32 s29, 0xbc7abc9e
	s_mov_b32 s31, 0x3e928af3
	s_mov_b32 s35, 0x3e5ade15
	s_mov_b32 s37, 0x3ec71dee
	s_mov_b32 s39, 0x3efa0199
	s_mov_b32 s41, 0x3f2a01a0
	s_mov_b32 s43, 0x3f56c16c
	s_mov_b32 s45, 0x3f811111
	s_mov_b32 s47, 0x3fa55555
	s_mov_b32 s49, 0x3fc55555
	s_mov_b32 s69, s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[2:3]
	s_and_b32 s1, vcc_lo, exec_lo
	s_cselect_b32 s1, 0x100, 0
	v_ldexp_f64 v[2:3], v[2:3], s1
	s_cselect_b32 s1, 0xffffff80, 0
	s_lshl_b32 s12, s33, 9
	s_lshl_b32 s13, s33, 8
	s_add_i32 s11, s12, 0
	s_ashr_i32 s24, s4, 31
	s_add_i32 s25, s11, s12
	s_lshl_b32 s52, s33, 5
	s_add_i32 s54, s25, s12
	s_ashr_i32 s53, s7, 31
	s_add_i32 s54, s54, s12
	s_mul_i32 s12, s6, s24
	s_add_i32 s55, s54, s13
	s_ashr_i32 s11, s10, 31
	s_ashr_i32 s25, s3, 31
	s_add_i32 s57, s55, s13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[4:5], v[2:3]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[6:7], v[2:3], v[4:5]
	v_mul_f64 v[4:5], v[4:5], 0.5
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[4:5], v[4:5], v[8:9], v[4:5]
	v_fma_f64 v[8:9], -v[6:7], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[8:9], v[4:5], v[6:7]
	v_fma_f64 v[8:9], -v[6:7], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[8:9], v[4:5], v[6:7]
	v_ldexp_f64 v[4:5], v[4:5], s1
	s_mul_hi_u32 s1, s6, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s56, s1, s12
	s_add_u32 s5, s5, s3
	s_addc_u32 s1, s2, s25
	s_mul_hi_u32 s2, s5, s6
	s_mul_i32 s1, s1, s6
	s_mul_i32 s64, s5, s6
	s_add_i32 s58, s2, s1
	s_cmp_gt_i32 s33, 0
	s_mul_i32 s2, s50, 0x900
	s_cselect_b32 s59, -1, 0
	s_abs_i32 s60, s33
	s_lshl_b32 s5, s50, 11
	s_sub_i32 s24, 0, s60
	s_lshl_b32 s25, s50, 10
	v_cmp_gt_u32_e64 s1, 32, v0
	s_mov_b32 s12, 11
	s_mov_b32 s13, 0x3fe00000
	s_ashr_i32 s65, s33, 31
	s_sub_i32 s66, 0, s33
	v_dual_cndmask_b32 v3, v5, v3 :: v_dual_cndmask_b32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[4:5], null, v[2:3], v[2:3], 1.0
	v_div_scale_f64 v[10:11], vcc_lo, 1.0, v[2:3], 1.0
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_mul_f64 v[8:9], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[8:9], v[10:11]
	v_cvt_f32_u32_e32 v10, s60
	v_rcp_iflag_f32_e32 v10, v10
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v10, 0x4f7ffffe, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v11, v10
	v_mul_lo_u32 v13, s24, v11
	s_lshl_b32 s24, s50, 9
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[8:9]
	v_lshlrev_b32_e32 v6, 3, v0
	v_mul_lo_u32 v9, v0, s33
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[7:8], null, 0x900, s51, v[6:7]
	v_lshl_add_u32 v10, s51, 11, v6
	v_lshlrev_b32_e32 v12, 3, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_subrev_nc_u32_e32 v10, s5, v10
	v_subrev_nc_u32_e32 v9, s2, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[7:8], null, 0x600, s51, v[12:13]
	v_lshl_add_u32 v14, s51, 10, v12
	s_lshl_b32 s5, s51, 9
	v_add_nc_u32_e32 v8, 0, v9
	v_add_nc_u32_e32 v9, s57, v6
	v_mul_hi_u32 v6, v11, v13
	s_sub_i32 s67, s5, s24
	s_mul_i32 s24, s50, 0x600
	v_cmp_gt_i32_e64 s2, s52, v0
	v_add_nc_u32_e32 v10, 0, v10
	s_lshl_b32 s5, s51, 3
	v_add_nc_u32_e32 v12, 0, v12
	v_add_nc_u32_e32 v11, v11, v6
	v_div_fixup_f64 v[2:3], v[4:5], v[2:3], 1.0
	v_subrev_nc_u32_e32 v4, s24, v7
	v_subrev_nc_u32_e32 v5, s25, v14
	s_lshl_b32 s24, s50, 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	s_sub_i32 s68, s5, s24
	v_add_nc_u32_e32 v13, 0, v4
	s_delay_alu instid0(VALU_DEP_2)
	v_add_nc_u32_e32 v14, 0, v5
	s_mov_b32 s24, 0x652b82fe
	s_mov_b32 s25, 0x3ff71547
	s_branch .LBB15_11
.LBB15_10:                              ;   in Loop: Header=BB15_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s70
	s_add_i32 s61, s61, 32
	s_sub_i32 s69, s69, 32
	s_cmp_ge_i32 s61, s6
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB15_27
.LBB15_11:                              ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB15_14 Depth 2
                                        ;     Child Loop BB15_21 Depth 2
                                        ;       Child Loop BB15_23 Depth 3
                                        ;       Child Loop BB15_26 Depth 3
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB15_16
; %bb.12:                               ;   in Loop: Header=BB15_11 Depth=1
	v_dual_mov_b32 v5, v8 :: v_dual_mov_b32 v6, v10
	v_mov_b32_e32 v4, v0
	s_mov_b32 s70, 0
	s_branch .LBB15_14
.LBB15_13:                              ;   in Loop: Header=BB15_14 Depth=2
	s_or_b32 exec_lo, exec_lo, s71
	v_add_nc_u32_e32 v4, 64, v4
	v_add_nc_u32_e32 v6, 0x200, v6
	v_add_nc_u32_e32 v5, 0x200, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s52, v4
	s_or_b32 s70, vcc_lo, s70
	s_and_not1_b32 exec_lo, exec_lo, s70
	s_cbranch_execz .LBB15_16
.LBB15_14:                              ;   Parent Loop BB15_11 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v7, v4, v11
	s_mov_b32 s71, exec_lo
	v_mul_lo_u32 v15, v7, s60
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v15, v4, v15
	v_subrev_nc_u32_e32 v17, s60, v15
	v_cmp_le_u32_e32 vcc_lo, s60, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v15, v15, v17 :: v_dual_add_nc_u32 v16, 1, v7
	v_cndmask_b32_e32 v7, v7, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s60, v15
	v_add_nc_u32_e32 v16, 1, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v7, v7, v16, vcc_lo
	v_xor_b32_e32 v7, s65, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v7, s65, v7
	v_add_nc_u32_e32 v15, s61, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s6, v15
	s_cbranch_execz .LBB15_13
; %bb.15:                               ;   in Loop: Header=BB15_14 Depth=2
	v_ashrrev_i32_e32 v16, 31, v15
	v_add_co_u32 v17, vcc_lo, s62, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v18, null, s56, v16, vcc_lo
	v_mul_lo_u32 v19, v17, s53
	v_mad_u64_u32 v[15:16], null, v17, s63, s[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v20, v18, s63
	v_mad_u64_u32 v[17:18], null, s66, v7, v[4:5]
	v_add3_u32 v7, v20, v16, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v15, vcc_lo, v15, v17
	v_add_co_ci_u32_e64 v16, null, 0, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[15:16], 3, v[15:16]
	v_add_co_u32 v17, vcc_lo, s8, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v18, null, s9, v16, vcc_lo
	v_add_co_u32 v15, vcc_lo, s14, v15
	v_add_co_ci_u32_e64 v16, null, s15, v16, vcc_lo
	global_load_b64 v[17:18], v[17:18], off
	global_load_b64 v[15:16], v[15:16], off
	s_waitcnt vmcnt(1)
	ds_store_b64 v6, v[17:18]
	s_waitcnt vmcnt(0)
	ds_store_b64 v5, v[15:16]
	s_branch .LBB15_13
.LBB15_16:                              ;   in Loop: Header=BB15_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	v_or_b32_e32 v4, s61, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s6, v4
	s_and_b32 s70, s1, vcc_lo
	s_and_saveexec_b32 s5, s70
	s_cbranch_execz .LBB15_18
; %bb.17:                               ;   in Loop: Header=BB15_11 Depth=1
	v_add_co_u32 v4, s70, s64, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s58, 0, s70
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s16, v4
	v_add_co_ci_u32_e64 v7, null, s17, v5, vcc_lo
	v_add_co_u32 v4, vcc_lo, s18, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s19, v5, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	ds_store_2addr_b64 v9, v[6:7], v[4:5] offset1:32
.LBB15_18:                              ;   in Loop: Header=BB15_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_cmp_gt_i32 s6, s61
	s_waitcnt lgkmcnt(0)
	s_cselect_b32 s5, -1, 0
	s_barrier
	s_and_b32 s5, s0, s5
	buffer_gl0_inv
	s_and_saveexec_b32 s70, s5
	s_cbranch_execz .LBB15_10
; %bb.19:                               ;   in Loop: Header=BB15_11 Depth=1
	v_med3_i32 v15, s69, 1, 32
	s_mov_b32 s71, 0
	s_mov_b32 s72, s54
	s_mov_b32 s73, s55
	s_branch .LBB15_21
.LBB15_20:                              ;   in Loop: Header=BB15_21 Depth=2
	s_add_i32 s71, s71, 1
	s_add_i32 s73, s73, s68
	v_cmp_eq_u32_e32 vcc_lo, s71, v15
	s_add_i32 s72, s72, s68
	s_cbranch_vccnz .LBB15_10
.LBB15_21:                              ;   Parent Loop BB15_11 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB15_23 Depth 3
                                        ;       Child Loop BB15_26 Depth 3
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v6, 0
	v_mov_b32_e32 v7, 0
	s_and_not1_b32 vcc_lo, exec_lo, s59
	s_cbranch_vccnz .LBB15_24
; %bb.22:                               ;   in Loop: Header=BB15_21 Depth=2
	v_mov_b32_e32 v16, v12
	s_mov_b32 s5, s72
	s_mov_b32 s74, s73
	s_mov_b32 s75, s33
	.p2align	6
.LBB15_23:                              ;   Parent Loop BB15_11 Depth=1
                                        ;     Parent Loop BB15_21 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_mov_b32_e32 v19, s5
	v_mov_b32_e32 v21, s74
	v_add_nc_u32_e32 v23, s67, v16
	s_add_i32 s75, s75, -1
	ds_load_b64 v[17:18], v16
	ds_load_b64 v[19:20], v19
	ds_load_b64 v[21:22], v21
	ds_load_b64 v[23:24], v23
	v_add_nc_u32_e32 v16, 8, v16
	s_add_i32 s74, s74, 8
	s_add_i32 s5, s5, 8
	s_cmp_eq_u32 s75, 0
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[6:7], v[19:20], v[17:18], v[6:7]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[4:5], v[21:22], v[23:24], v[4:5]
	s_cbranch_scc0 .LBB15_23
.LBB15_24:                              ;   in Loop: Header=BB15_21 Depth=2
	s_and_not1_b32 vcc_lo, exec_lo, s59
	s_cbranch_vccnz .LBB15_20
; %bb.25:                               ;   in Loop: Header=BB15_21 Depth=2
	s_lshl_b32 s5, s71, 3
	s_mov_b32 s74, s33
	s_add_i32 s5, s57, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v16, s5
	ds_load_2addr_b64 v[16:19], v16 offset1:32
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[6:7], v[2:3], v[6:7], -v[16:17]
	v_mul_f64 v[16:17], v[6:7], s[24:25]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[6:7]
	v_cmp_ngt_f64_e64 s5, 0xc090cc00, v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[16:17], v[16:17]
	v_fma_f64 v[20:21], v[16:17], s[26:27], v[6:7]
	v_cvt_i32_f64_e32 v24, v[16:17]
	v_add_f64 v[6:7], v[4:5], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[16:17], s[28:29], v[20:21]
	v_fma_f64 v[22:23], v[20:21], s[34:35], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[36:37]
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[40:41]
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[44:45]
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[46:47]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[48:49]
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], 1.0
	v_fma_f64 v[16:17], v[20:21], v[22:23], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[16:17], v[16:17], v24
	v_cndmask_b32_e32 v17, 0x7ff00000, v17, vcc_lo
	s_and_b32 vcc_lo, s5, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0, v16, vcc_lo
	v_cndmask_b32_e64 v5, 0, v17, s5
	s_mov_b32 s5, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[4:5]
	v_mul_f64 v[6:7], v[2:3], v[6:7]
	.p2align	6
.LBB15_26:                              ;   Parent Loop BB15_11 Depth=1
                                        ;     Parent Loop BB15_21 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	s_add_i32 s75, s72, s5
	v_add_nc_u32_e32 v20, s5, v14
	v_dual_mov_b32 v18, s75 :: v_dual_add_nc_u32 v21, s5, v13
	s_add_i32 s75, s73, s5
	s_add_i32 s74, s74, -1
	ds_load_b64 v[16:17], v20
	ds_load_b64 v[18:19], v18
	s_add_i32 s5, s5, 8
	s_cmp_eq_u32 s74, 0
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[16:17], v[6:7], v[18:19], v[16:17]
	v_mov_b32_e32 v18, s75
	ds_store_b64 v20, v[16:17]
	ds_load_b64 v[16:17], v18
	ds_load_b64 v[18:19], v21
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[16:17], v[4:5], v[16:17], v[18:19]
	ds_store_b64 v21, v[16:17]
	s_cbranch_scc0 .LBB15_26
	s_branch .LBB15_20
.LBB15_27:
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB15_31
; %bb.28:
	s_cmp_lt_i32 s33, 1
	s_cbranch_scc1 .LBB15_31
; %bb.29:
	v_ashrrev_i32_e32 v2, 31, v1
	v_mul_lo_u32 v0, v0, s33
	s_mul_i32 s0, s33, s3
	s_ashr_i32 s2, s7, 31
	s_ashr_i32 s1, s0, 31
	v_mad_i64_i32 v[3:4], null, s6, s4, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b32_e32 v0, 3, v0
	v_mad_u64_u32 v[1:2], null, v3, s7, s[0:1]
	v_mul_lo_u32 v5, v3, s2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v6, v4, s7
	s_lshl_b32 s0, s50, 10
	s_mulk_i32 s50, 0x600
	v_mad_u64_u32 v[3:4], null, 0x600, s51, v[0:1]
	v_lshl_add_u32 v0, s51, 10, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add3_u32 v2, v6, v2, v5
	v_subrev_nc_u32_e32 v0, s0, v0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_subrev_nc_u32_e32 v5, s50, v3
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 0, v0
	v_add_nc_u32_e32 v5, 0, v5
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s20, v2
	v_add_co_ci_u32_e64 v1, null, s21, v3, vcc_lo
	v_add_co_u32 v2, vcc_lo, s22, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s23, v3, vcc_lo
	.p2align	6
.LBB15_30:                              ; =>This Inner Loop Header: Depth=1
	ds_load_b64 v[6:7], v4
	ds_load_b64 v[8:9], v5
	v_add_nc_u32_e32 v5, 8, v5
	v_add_nc_u32_e32 v4, 8, v4
	s_add_i32 s33, s33, -1
	s_waitcnt lgkmcnt(1)
	global_store_b64 v[0:1], v[6:7], off
	s_waitcnt lgkmcnt(0)
	global_store_b64 v[2:3], v[8:9], off
	v_add_co_u32 v0, vcc_lo, v0, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v2, vcc_lo, v2, 8
	v_add_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	s_cmp_lg_u32 s33, 0
	s_cbranch_scc1 .LBB15_30
.LBB15_31:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 80
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
		.amdhsa_system_sgpr_workgroup_id_z 1
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 44
		.amdhsa_next_free_sgpr 76
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
		.amdhsa_inst_pref_size 27
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
	.size	_Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii, .Lfunc_end15-_Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii
                                        ; -- End function
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.num_vgpr, 44
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.num_agpr, 0
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.numbered_sgpr, 76
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.num_named_barrier, 0
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.private_seg_size, 0
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.uses_vcc, 1
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.uses_flat_scratch, 0
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.has_dyn_sized_stack, 0
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.has_recursion, 0
	.set _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3428
; TotalNumSgprs: 78
; NumVgprs: 44
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 78
; NumVGPRsForWavesPerEU: 44
; Occupancy: 16
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 1
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii,"axG",@progbits,_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii,comdat
	.protected	_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii ; -- Begin function _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii
	.globl	_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii
	.p2align	8
	.type	_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii,@function
_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii: ; @_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s5, s[0:1], 0x2c
	s_load_b64 s[6:7], s[0:1], 0x24
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s16, s5
	s_abs_i32 s17, s7
	v_cvt_f32_u32_e32 v1, s16
	s_sub_i32 s9, 0, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s8, v1
	v_lshl_add_u32 v1, s2, 6, v0
	s_mul_i32 s9, s9, s8
	s_mul_hi_u32 s9, s8, s9
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s18, s8, s9
	s_load_b256 s[8:15], s[0:1], 0x0
	s_mul_hi_u32 s0, s17, s18
	s_xor_b32 s1, s7, s5
	s_mul_i32 s5, s0, s16
	s_ashr_i32 s26, s1, 31
	s_sub_i32 s1, s17, s5
	s_add_i32 s5, s0, 1
	s_sub_i32 s17, s1, s16
	s_cmp_ge_u32 s1, s16
	s_cselect_b32 s0, s5, s0
	s_cselect_b32 s1, s17, s1
	s_add_i32 s2, s0, 1
	s_cmp_ge_u32 s1, s16
	s_cselect_b32 s1, s2, s0
	v_cmp_gt_i32_e64 s0, s6, v1
	s_xor_b32 s28, s1, s26
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s2, s28, s26
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB16_8
; %bb.1:
	s_cmp_lt_i32 s2, 1
	s_cbranch_scc1 .LBB16_8
; %bb.2:
	v_ashrrev_i32_e32 v2, 31, v1
	s_ashr_i32 s5, s7, 31
	s_mul_i32 s16, s2, s3
	s_mov_b32 s18, 0
	s_ashr_i32 s17, s16, 31
	v_mad_i64_i32 v[3:4], null, s6, s4, v[1:2]
	v_mul_lo_u32 v2, v0, s2
	s_cmp_lt_u32 s2, 8
	s_mul_i32 s20, s26, 0x300
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v6, v4, s7
	v_mul_lo_u32 v7, v3, s5
	s_mul_i32 s5, s28, 0x300
	s_cbranch_scc1 .LBB16_5
; %bb.3:
	v_mad_u64_u32 v[4:5], null, v3, s7, 0
	s_lshl_b64 s[22:23], s[16:17], 2
	s_and_b32 s18, s2, 0x7ffffff8
	s_sub_i32 s19, s5, s20
	s_waitcnt lgkmcnt(0)
	s_add_u32 s21, s8, s22
	s_addc_u32 s22, s9, s23
	v_lshl_add_u32 v8, v2, 2, 0
	v_add3_u32 v5, v5, v7, v6
	v_mov_b32_e32 v9, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	v_add_co_u32 v4, vcc_lo, s21, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s22, v5, vcc_lo
	s_mov_b32 s21, 0
	v_add_co_u32 v4, vcc_lo, v4, 28
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB16_4:                               ; =>This Inner Loop Header: Depth=1
	s_clause 0x1
	global_load_b128 v[10:13], v[4:5], off offset:-28
	global_load_b128 v[14:17], v[4:5], off offset:-12
	v_add_nc_u32_e32 v18, s19, v8
	v_add_co_u32 v4, vcc_lo, v4, 32
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_add_i32 s21, s21, 8
	s_waitcnt vmcnt(1)
	ds_store_b32 v8, v10
	ds_store_b32 v18, v9
	ds_store_b32 v8, v11 offset:4
	ds_store_b32 v18, v9 offset:4
	ds_store_b32 v8, v12 offset:8
	ds_store_b32 v18, v9 offset:8
	ds_store_b32 v8, v13 offset:12
	ds_store_b32 v18, v9 offset:12
	s_waitcnt vmcnt(0)
	ds_store_b32 v8, v14 offset:16
	ds_store_b32 v18, v9 offset:16
	ds_store_b32 v8, v15 offset:20
	ds_store_b32 v18, v9 offset:20
	ds_store_b32 v8, v16 offset:24
	ds_store_b32 v18, v9 offset:24
	ds_store_b32 v8, v17 offset:28
	v_add_nc_u32_e32 v8, 32, v8
	s_cmp_lg_u32 s18, s21
	ds_store_b32 v18, v9 offset:28
	s_cbranch_scc1 .LBB16_4
.LBB16_5:
	s_set_inst_prefetch_distance 0x2
	s_and_b32 s21, s2, 7
	s_mov_b32 s19, 0
	s_cmp_eq_u32 s21, 0
	s_cbranch_scc1 .LBB16_8
; %bb.6:
	v_mad_u64_u32 v[4:5], null, v3, s7, 0
	s_sub_i32 s5, s5, s20
	v_lshlrev_b32_e32 v8, 2, v2
	s_lshl_b32 s20, s18, 2
	s_lshl_b64 s[18:19], s[18:19], 2
	s_lshl_b64 s[16:17], s[16:17], 2
	s_waitcnt lgkmcnt(0)
	s_add_u32 s8, s8, s18
	v_add3_u32 v5, v5, v7, v6
	s_addc_u32 s9, s9, s19
	s_add_u32 s8, s8, s16
	s_addc_u32 s9, s9, s17
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[2:3], 2, v[4:5]
	v_mov_b32_e32 v5, 0
	v_add3_u32 v4, 0, s20, v8
	v_add_co_u32 v2, vcc_lo, s8, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s9, v3, vcc_lo
.LBB16_7:                               ; =>This Inner Loop Header: Depth=1
	global_load_b32 v6, v[2:3], off
	v_add_co_u32 v2, vcc_lo, v2, 4
	v_add_nc_u32_e32 v7, s5, v4
	v_add_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	s_add_i32 s21, s21, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lg_u32 s21, 0
	s_waitcnt vmcnt(0)
	ds_store_b32 v4, v6
	v_add_nc_u32_e32 v4, 4, v4
	ds_store_b32 v7, v5
	s_cbranch_scc1 .LBB16_7
.LBB16_8:
	s_or_b32 exec_lo, exec_lo, s1
	s_lshl_b32 s1, s2, 8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s16, s1, 0
	s_add_i32 s17, s16, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s5, s17, s1
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB16_24
; %bb.9:
	v_cvt_f64_i32_e32 v[2:3], s2
	s_lshl_b32 s18, s2, 6
	s_waitcnt lgkmcnt(0)
	s_mul_hi_u32 s8, s6, s4
	s_mov_b32 s23, 0
	s_mul_i32 s24, s6, s4
	s_mov_b32 s25, s7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[2:3]
	s_and_b32 s1, vcc_lo, exec_lo
	s_cselect_b32 s1, 0x100, 0
	v_ldexp_f64 v[2:3], v[2:3], s1
	s_cselect_b32 s1, 0xffffff80, 0
	s_ashr_i32 s20, s7, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[4:5], v[2:3]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[6:7], v[2:3], v[4:5]
	v_mul_f64 v[4:5], v[4:5], 0.5
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[4:5], v[4:5], v[8:9], v[4:5]
	v_fma_f64 v[8:9], -v[6:7], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[8:9], v[4:5], v[6:7]
	v_fma_f64 v[8:9], -v[6:7], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[8:9], v[4:5], v[6:7]
	v_ldexp_f64 v[4:5], v[4:5], s1
	s_ashr_i32 s1, s4, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s1, s6, s1
	s_add_i32 s19, s8, s1
	s_mul_i32 s8, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ashr_i32 s9, s8, 31
	s_cmp_gt_i32 s2, 0
	s_cselect_b32 s21, -1, 0
	s_abs_i32 s22, s2
	s_lshl_b32 s27, s26, 8
	s_sub_i32 s1, 0, s22
	s_lshl_b32 s29, s26, 9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v3, v5, v3 :: v_dual_cndmask_b32 v2, v4, v2
	v_div_scale_f64 v[4:5], null, v[2:3], v[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_div_scale_f64 v[8:9], vcc_lo, 1.0, v[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	v_fma_f64 v[4:5], -v[4:5], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[10:11]
	v_dual_mov_b32 v10, 0xff800000 :: v_dual_lshlrev_b32 v7, 2, v0
	v_lshl_add_u32 v9, s28, 8, v7
	v_lshl_add_u32 v7, s28, 9, v7
	s_lshl_b32 s28, s28, 2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[4:5], v[2:3], 1.0
	v_cvt_f32_u32_e32 v4, s22
	v_rcp_iflag_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v5, 0x4f7ffffe, v4
	v_cvt_f32_f64_e32 v4, v[2:3]
	v_mul_lo_u32 v2, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_lshlrev_b32_e32 v2, 2, v2
	v_cvt_u32_f32_e32 v3, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v5, 0, v2
	v_mul_lo_u32 v6, s1, v3
	v_cmp_gt_i32_e64 s1, s18, v0
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_hi_u32 v8, v3, v6
	v_add_nc_u32_e32 v6, s5, v2
	v_subrev_nc_u32_e32 v2, s27, v9
	v_subrev_nc_u32_e32 v9, s29, v7
	s_lshl_b32 s29, s26, 2
	s_ashr_i32 s27, s2, 31
	s_sub_i32 s26, 0, s2
	s_sub_i32 s28, s28, s29
	v_add_nc_u32_e32 v7, v3, v8
	v_dual_mov_b32 v3, 0 :: v_dual_add_nc_u32 v8, 0, v2
	v_add_nc_u32_e32 v9, 0, v9
	s_mov_b32 s29, s6
	s_branch .LBB16_11
.LBB16_10:                              ;   in Loop: Header=BB16_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s30
	s_add_i32 s23, s23, 64
	s_sub_i32 s29, s29, 64
	s_cmp_ge_i32 s23, s6
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB16_25
.LBB16_11:                              ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB16_14 Depth 2
                                        ;     Child Loop BB16_19 Depth 2
                                        ;       Child Loop BB16_21 Depth 3
                                        ;       Child Loop BB16_23 Depth 3
	s_and_saveexec_b32 s30, s1
	s_cbranch_execz .LBB16_16
; %bb.12:                               ;   in Loop: Header=BB16_11 Depth=1
	v_dual_mov_b32 v11, v9 :: v_dual_mov_b32 v12, v8
	v_mov_b32_e32 v2, v0
	s_mov_b32 s31, 0
	s_branch .LBB16_14
.LBB16_13:                              ;   in Loop: Header=BB16_14 Depth=2
	s_or_b32 exec_lo, exec_lo, s33
	v_add_nc_u32_e32 v2, 64, v2
	v_add_nc_u32_e32 v12, 0x100, v12
	v_add_nc_u32_e32 v11, 0x100, v11
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s18, v2
	s_or_b32 s31, vcc_lo, s31
	s_and_not1_b32 exec_lo, exec_lo, s31
	s_cbranch_execz .LBB16_16
.LBB16_14:                              ;   Parent Loop BB16_11 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v13, v2, v7
	s_mov_b32 s33, exec_lo
	v_mul_lo_u32 v14, v13, s22
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v14, v2, v14
	v_subrev_nc_u32_e32 v16, s22, v14
	v_cmp_le_u32_e32 vcc_lo, s22, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v14, v14, v16 :: v_dual_add_nc_u32 v15, 1, v13
	v_cndmask_b32_e32 v13, v13, v15, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s22, v14
	v_add_nc_u32_e32 v15, 1, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v13, v13, v15, vcc_lo
	v_xor_b32_e32 v13, s27, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v13, s27, v13
	v_add_nc_u32_e32 v14, s23, v13
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s6, v14
	s_cbranch_execz .LBB16_13
; %bb.15:                               ;   in Loop: Header=BB16_14 Depth=2
	v_ashrrev_i32_e32 v15, 31, v14
	v_add_co_u32 v16, vcc_lo, s24, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v17, null, s19, v15, vcc_lo
	v_mul_lo_u32 v18, v16, s20
	v_mad_u64_u32 v[14:15], null, v16, s25, s[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v19, v17, s25
	v_mad_u64_u32 v[16:17], null, s26, v13, v[2:3]
	v_add3_u32 v15, v19, v15, v18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v13, vcc_lo, v14, v16
	v_add_co_ci_u32_e64 v14, null, 0, v15, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[13:14], 2, v[13:14]
	v_add_co_u32 v15, vcc_lo, s10, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v16, null, s11, v14, vcc_lo
	v_add_co_u32 v13, vcc_lo, s12, v13
	v_add_co_ci_u32_e64 v14, null, s13, v14, vcc_lo
	global_load_b32 v15, v[15:16], off
	global_load_b32 v13, v[13:14], off
	s_waitcnt vmcnt(1)
	ds_store_b32 v12, v15
	s_waitcnt vmcnt(0)
	ds_store_b32 v11, v13
	s_branch .LBB16_13
.LBB16_16:                              ;   in Loop: Header=BB16_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s30
	s_cmp_gt_i32 s6, s23
	s_waitcnt lgkmcnt(0)
	s_cselect_b32 s30, -1, 0
	s_barrier
	s_and_b32 s31, s0, s30
	buffer_gl0_inv
	s_and_saveexec_b32 s30, s31
	s_cbranch_execz .LBB16_10
; %bb.17:                               ;   in Loop: Header=BB16_11 Depth=1
	v_med3_i32 v2, s29, 1, 64
	s_mov_b32 s31, 0
	s_mov_b32 s33, s17
	s_mov_b32 s34, s16
	s_branch .LBB16_19
.LBB16_18:                              ;   in Loop: Header=BB16_19 Depth=2
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v12, v3, v10
	s_add_i32 s31, s31, 1
	v_mov_b32_e32 v10, v11
	v_cmp_eq_u32_e32 vcc_lo, s31, v2
	s_add_i32 s34, s34, s28
	v_mov_b32_e32 v3, v12
	s_add_i32 s33, s33, s28
	s_cbranch_vccnz .LBB16_10
.LBB16_19:                              ;   Parent Loop BB16_11 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB16_21 Depth 3
                                        ;       Child Loop BB16_23 Depth 3
	v_mov_b32_e32 v12, 0
	s_and_not1_b32 vcc_lo, exec_lo, s21
	s_cbranch_vccnz .LBB16_22
; %bb.20:                               ;   in Loop: Header=BB16_19 Depth=2
	v_mov_b32_e32 v11, v5
	s_mov_b32 s35, s34
	s_mov_b32 s36, s2
.LBB16_21:                              ;   Parent Loop BB16_11 Depth=1
                                        ;     Parent Loop BB16_19 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_mov_b32_e32 v13, s35
	s_add_i32 s36, s36, -1
	s_add_i32 s35, s35, 4
	s_cmp_eq_u32 s36, 0
	ds_load_b32 v14, v11
	ds_load_b32 v13, v13
	s_waitcnt lgkmcnt(0)
	v_dual_fmac_f32 v12, v14, v13 :: v_dual_add_nc_u32 v11, 4, v11
	s_cbranch_scc0 .LBB16_21
.LBB16_22:                              ;   in Loop: Header=BB16_19 Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v11, v12, v4
	s_mov_b32 s35, s33
	s_mov_b32 s36, s2
	v_cmp_gt_f32_e32 vcc_lo, v11, v10
	v_cndmask_b32_e32 v11, v10, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v12, v12, v4, -v11
	v_mul_f32_e32 v14, 0x3fb8aa3b, v12
	v_sub_f32_e32 v10, v10, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f32 v17, 0x3fb8aa3b, v12, -v14
	v_rndne_f32_e32 v18, v14
	v_dual_fmac_f32 v17, 0x32a5705f, v12 :: v_dual_sub_f32 v14, v14, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mul_f32 v13, 0x3fb8aa3b, v10 :: v_dual_add_f32 v14, v14, v17
	v_fma_f32 v15, 0x3fb8aa3b, v10, -v13
	v_rndne_f32_e32 v16, v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v14, v14
	v_sub_f32_e32 v13, v13, v16
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v15, 0x32a5705f, v10
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v10
	v_add_f32_e32 v13, v13, v15
	v_cvt_i32_f32_e32 v15, v16
	v_cvt_i32_f32_e32 v16, v18
	s_delay_alu instid0(VALU_DEP_3)
	v_exp_f32_e32 v13, v13
	s_delay_alu instid0(TRANS32_DEP_2) | instid1(VALU_DEP_1)
	v_ldexp_f32 v14, v14, v16
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v13, v13, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v13, 0, v13, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v12
	v_cndmask_b32_e32 v14, 0, v14, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v10
	v_dual_cndmask_b32 v10, 0x7f800000, v13 :: v_dual_mov_b32 v13, v6
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v12
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e32 v12, 0x7f800000, v14, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s21
	s_cbranch_vccnz .LBB16_18
.LBB16_23:                              ;   Parent Loop BB16_11 Depth=1
                                        ;     Parent Loop BB16_19 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_mov_b32_e32 v14, s35
	s_add_i32 s36, s36, -1
	s_add_i32 s35, s35, 4
	s_cmp_eq_u32 s36, 0
	ds_load_b32 v14, v14
	ds_load_b32 v15, v13
	s_waitcnt lgkmcnt(1)
	v_mul_f32_e32 v14, v12, v14
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v14, v10, v15
	ds_store_b32 v13, v14
	v_add_nc_u32_e32 v13, 4, v13
	s_cbranch_scc0 .LBB16_23
	s_branch .LBB16_18
.LBB16_24:
	v_mov_b32_e32 v3, 0
.LBB16_25:
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB16_29
; %bb.26:
	s_cmp_lt_i32 s2, 1
	s_cbranch_scc1 .LBB16_29
; %bb.27:
	v_div_scale_f32 v6, null, v3, v3, 1.0
	v_ashrrev_i32_e32 v2, 31, v1
	v_div_scale_f32 v9, vcc_lo, 1.0, v3, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f32_e32 v7, v6
	s_ashr_i32 s0, s7, 31
	v_mad_i64_i32 v[4:5], null, s6, s4, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v5, v5, s7
	s_waitcnt_depctr 0xfff
	v_fma_f32 v8, -v6, v7, 1.0
	v_mad_u64_u32 v[1:2], null, v4, s7, 0
	v_fmac_f32_e32 v7, v8, v7
	v_mul_lo_u32 v8, v4, s0
	s_mul_i32 s0, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	s_ashr_i32 s1, s0, 31
	v_mul_f32_e32 v10, v9, v7
	s_lshl_b64 s[0:1], s[0:1], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add3_u32 v2, v2, v8, v5
	v_fma_f32 v4, -v6, v10, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fmac_f32_e32 v10, v4, v7
	v_mul_lo_u32 v4, s2, v0
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v5, -v6, v10, v9
	v_div_fmas_f32 v2, v5, v7, v10
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s14, v0
	v_add_co_ci_u32_e64 v1, null, s15, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_fixup_f32 v2, v2, v3, 1.0
	v_add_co_u32 v0, vcc_lo, v0, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	v_cmp_lt_f32_e32 vcc_lo, 0, v3
	v_lshl_add_u32 v3, v4, 2, s5
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
.LBB16_28:                              ; =>This Inner Loop Header: Depth=1
	ds_load_b32 v4, v3
	v_add_nc_u32_e32 v3, 4, v3
	s_add_i32 s2, s2, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	s_cmp_lg_u32 s2, 0
	s_waitcnt lgkmcnt(0)
	v_mul_f32_e32 v4, v2, v4
	global_store_b32 v[0:1], v4, off
	v_add_co_u32 v0, vcc_lo, v0, 4
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_cbranch_scc1 .LBB16_28
.LBB16_29:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 48
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
		.amdhsa_system_sgpr_workgroup_id_z 1
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 20
		.amdhsa_next_free_sgpr 37
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
		.amdhsa_inst_pref_size 20
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii,"axG",@progbits,_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii,comdat
.Lfunc_end16:
	.size	_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii, .Lfunc_end16-_Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii
                                        ; -- End function
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.num_vgpr, 20
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.num_agpr, 0
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.numbered_sgpr, 37
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.num_named_barrier, 0
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.private_seg_size, 0
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.uses_vcc, 1
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.uses_flat_scratch, 0
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.has_dyn_sized_stack, 0
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.has_recursion, 0
	.set _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2548
; TotalNumSgprs: 39
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 39
; NumVGPRsForWavesPerEU: 20
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 1
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii,"axG",@progbits,_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii,comdat
	.protected	_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii ; -- Begin function _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii
	.globl	_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii
	.p2align	8
	.type	_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii,@function
_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii: ; @_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s5, s[0:1], 0x2c
	s_load_b64 s[16:17], s[0:1], 0x24
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s6, s5
	s_abs_i32 s18, s17
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s8, 0, s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s7, v1
	v_lshl_add_u32 v1, s2, 6, v0
	s_mul_i32 s8, s8, s7
	s_mul_hi_u32 s8, s7, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s7, s7, s8
	s_load_b256 s[8:15], s[0:1], 0x0
	s_mul_hi_u32 s0, s18, s7
	s_xor_b32 s1, s17, s5
	s_mul_i32 s7, s0, s6
	s_ashr_i32 s5, s1, 31
	s_sub_i32 s1, s18, s7
	s_add_i32 s7, s0, 1
	s_sub_i32 s18, s1, s6
	s_cmp_ge_u32 s1, s6
	s_cselect_b32 s0, s7, s0
	s_cselect_b32 s1, s18, s1
	s_add_i32 s2, s0, 1
	s_cmp_ge_u32 s1, s6
	s_cselect_b32 s1, s2, s0
	v_cmp_gt_i32_e64 s0, s16, v1
	s_xor_b32 s2, s1, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s33, s2, s5
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB17_8
; %bb.1:
	s_cmp_lt_i32 s33, 1
	s_cbranch_scc1 .LBB17_8
; %bb.2:
	v_ashrrev_i32_e32 v2, 31, v1
	s_ashr_i32 s7, s17, 31
	s_mul_i32 s6, s33, s3
	s_mov_b32 s18, 0
	s_mul_i32 s22, s2, 0x600
	v_mad_i64_i32 v[3:4], null, s16, s4, v[1:2]
	v_mul_lo_u32 v2, v0, s33
	s_mul_i32 s23, s5, 0x600
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v8, v4, s17
	v_mul_lo_u32 v9, v3, s7
	s_ashr_i32 s7, s6, 31
	s_cmp_lt_u32 s33, 8
	s_cbranch_scc1 .LBB17_5
; %bb.3:
	v_mad_u64_u32 v[4:5], null, v3, s17, 0
	s_lshl_b64 s[24:25], s[6:7], 3
	s_and_b32 s18, s33, 0x7ffffff8
	s_sub_i32 s19, s22, s23
	s_waitcnt lgkmcnt(0)
	s_add_u32 s21, s8, s24
	s_addc_u32 s24, s9, s25
	s_mov_b32 s20, 0
	v_add3_u32 v5, v5, v9, v8
	v_lshl_add_u32 v10, v2, 3, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	v_add_co_u32 v4, vcc_lo, s21, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s24, v5, vcc_lo
	s_mov_b32 s21, s20
	v_add_co_u32 v4, vcc_lo, v4, 56
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	v_dual_mov_b32 v6, s20 :: v_dual_mov_b32 v7, s21
.LBB17_4:                               ; =>This Inner Loop Header: Depth=1
	s_clause 0x3
	global_load_b128 v[11:14], v[4:5], off offset:-56
	global_load_b128 v[15:18], v[4:5], off offset:-40
	global_load_b128 v[19:22], v[4:5], off offset:-24
	global_load_b128 v[23:26], v[4:5], off offset:-8
	v_add_nc_u32_e32 v27, s19, v10
	v_add_co_u32 v4, vcc_lo, v4, 64
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_add_i32 s20, s20, 8
	s_waitcnt vmcnt(3)
	ds_store_b64 v10, v[11:12]
	ds_store_b64 v27, v[6:7]
	ds_store_b64 v10, v[13:14] offset:8
	ds_store_b64 v27, v[6:7] offset:8
	s_waitcnt vmcnt(2)
	ds_store_b64 v10, v[15:16] offset:16
	ds_store_b64 v27, v[6:7] offset:16
	ds_store_b64 v10, v[17:18] offset:24
	ds_store_b64 v27, v[6:7] offset:24
	s_waitcnt vmcnt(1)
	ds_store_b64 v10, v[19:20] offset:32
	ds_store_b64 v27, v[6:7] offset:32
	ds_store_b64 v10, v[21:22] offset:40
	ds_store_b64 v27, v[6:7] offset:40
	s_waitcnt vmcnt(0)
	ds_store_b64 v10, v[23:24] offset:48
	ds_store_b64 v27, v[6:7] offset:48
	ds_store_b64 v10, v[25:26] offset:56
	v_add_nc_u32_e32 v10, 64, v10
	s_cmp_lg_u32 s18, s20
	ds_store_b64 v27, v[6:7] offset:56
	s_cbranch_scc1 .LBB17_4
.LBB17_5:
	s_and_b32 s20, s33, 7
	s_mov_b32 s19, 0
	s_cmp_eq_u32 s20, 0
	s_cbranch_scc1 .LBB17_8
; %bb.6:
	v_mad_u64_u32 v[4:5], null, v3, s17, 0
	s_sub_i32 s21, s22, s23
	s_lshl_b32 s22, s18, 3
	s_lshl_b64 s[18:19], s[18:19], 3
	s_lshl_b64 s[6:7], s[6:7], 3
	s_waitcnt lgkmcnt(0)
	s_add_u32 s8, s8, s18
	v_dual_mov_b32 v2, 0 :: v_dual_lshlrev_b32 v3, 3, v2
	v_add3_u32 v5, v5, v9, v8
	s_addc_u32 s9, s9, s19
	s_add_u32 s6, s8, s6
	s_addc_u32 s7, s9, s7
	v_add3_u32 v6, 0, s22, v3
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	v_mov_b32_e32 v3, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s6, v4
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
.LBB17_7:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[7:8], v[4:5], off
	v_add_co_u32 v4, vcc_lo, v4, 8
	v_add_nc_u32_e32 v9, s21, v6
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_add_i32 s20, s20, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lg_u32 s20, 0
	s_waitcnt vmcnt(0)
	ds_store_b64 v6, v[7:8]
	v_add_nc_u32_e32 v6, 8, v6
	ds_store_b64 v9, v[2:3]
	s_cbranch_scc1 .LBB17_7
.LBB17_8:
	s_or_b32 exec_lo, exec_lo, s1
	s_lshl_b32 s1, s33, 9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s46, s1, 0
	s_add_i32 s47, s46, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s7, s47, s1
	s_cmp_lt_i32 s16, 1
	s_cbranch_scc1 .LBB17_24
; %bb.9:
	v_cvt_f64_i32_e32 v[2:3], s33
	s_lshl_b32 s48, s33, 6
	s_waitcnt lgkmcnt(0)
	s_mul_i32 s8, s33, s3
	s_mov_b32 s20, 0xfefa39ef
	s_mov_b32 s22, 0x3b39803f
	s_mov_b32 s24, 0xfca7ab0c
	s_mov_b32 s26, 0x6a5dcb37
	s_mov_b32 s28, 0x623fde64
	s_mov_b32 s30, 0x7c89e6b0
	s_mov_b32 s34, 0x14761f6e
	s_mov_b32 s36, 0x1852b7b0
	s_mov_b32 s38, 0x11122322
	s_mov_b32 s40, 0x555502a1
	s_mov_b32 s42, 0x55555511
	s_mov_b32 s44, 11
	s_mov_b32 s53, 0
	s_mul_i32 s54, s16, s4
	s_mov_b32 s55, s17
	s_mov_b32 s21, 0xbfe62e42
	s_mov_b32 s23, 0xbc7abc9e
	s_mov_b32 s25, 0x3e928af3
	s_mov_b32 s27, 0x3e5ade15
	s_mov_b32 s29, 0x3ec71dee
	s_mov_b32 s31, 0x3efa0199
	s_mov_b32 s35, 0x3f2a01a0
	s_mov_b32 s37, 0x3f56c16c
	s_mov_b32 s39, 0x3f811111
	s_mov_b32 s41, 0x3fa55555
	s_mov_b32 s43, 0x3fc55555
	s_mov_b32 s45, 0x3fe00000
	s_mov_b32 s59, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[2:3]
	s_and_b32 s1, vcc_lo, exec_lo
	s_cselect_b32 s1, 0x100, 0
	v_ldexp_f64 v[2:3], v[2:3], s1
	s_cselect_b32 s1, 0xffffff80, 0
	s_ashr_i32 s6, s4, 31
	s_ashr_i32 s49, s17, 31
	s_mul_i32 s6, s16, s6
	s_ashr_i32 s9, s8, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[4:5], v[2:3]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[6:7], v[2:3], v[4:5]
	v_mul_f64 v[4:5], v[4:5], 0.5
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[4:5], v[4:5], v[8:9], v[4:5]
	v_fma_f64 v[8:9], -v[6:7], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[8:9], v[4:5], v[6:7]
	v_fma_f64 v[8:9], -v[6:7], v[6:7], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[8:9], v[4:5], v[6:7]
	v_ldexp_f64 v[4:5], v[4:5], s1
	s_mul_hi_u32 s1, s16, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s50, s1, s6
	s_cmp_gt_i32 s33, 0
	s_cselect_b32 s51, -1, 0
	s_abs_i32 s52, s33
	s_lshl_b32 s6, s5, 9
	s_sub_i32 s1, 0, s52
	s_lshl_b32 s18, s5, 10
	s_lshl_b32 s5, s5, 3
	s_ashr_i32 s56, s33, 31
	s_sub_i32 s57, 0, s33
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v3, v5, v3 :: v_dual_cndmask_b32 v2, v4, v2
	v_div_scale_f64 v[4:5], null, v[2:3], v[2:3], 1.0
	v_div_scale_f64 v[10:11], vcc_lo, 1.0, v[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[6:7]
	v_fma_f64 v[4:5], -v[4:5], v[8:9], v[10:11]
	v_cvt_f32_u32_e32 v10, s52
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[8:9]
	v_rcp_iflag_f32_e32 v6, v10
	v_mul_lo_u32 v7, s33, v0
	v_lshlrev_b32_e32 v9, 3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_lshl_add_u32 v10, s2, 9, v9
	v_lshl_add_u32 v9, s2, 10, v9
	s_waitcnt_depctr 0xfff
	v_dual_mul_f32 v6, 0x4f7ffffe, v6 :: v_dual_lshlrev_b32 v7, 3, v7
	s_lshl_b32 s2, s2, 3
	s_sub_i32 s58, s2, s5
	v_subrev_nc_u32_e32 v9, s18, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v6, v6
	s_mov_b32 s18, 0x652b82fe
	s_mov_b32 s19, 0x3ff71547
	v_add_nc_u32_e32 v17, 0, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v8, s1, v6
	v_cmp_gt_i32_e64 s1, s48, v0
	v_mul_hi_u32 v8, v6, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_nc_u32_e32 v15, v6, v8
	v_div_fixup_f64 v[5:6], v[4:5], v[2:3], 1.0
	v_mov_b32_e32 v3, 0
	v_dual_mov_b32 v4, 0 :: v_dual_add_nc_u32 v13, 0, v7
	v_mov_b32_e32 v8, 0xffe1ccf3
	v_add_nc_u32_e32 v14, s7, v7
	v_subrev_nc_u32_e32 v7, s6, v10
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v7, 0x85ebc8a0 :: v_dual_add_nc_u32 v16, 0, v7
	s_branch .LBB17_11
.LBB17_10:                              ;   in Loop: Header=BB17_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s60
	s_add_i32 s53, s53, 64
	s_sub_i32 s59, s59, 64
	s_cmp_ge_i32 s53, s16
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB17_25
.LBB17_11:                              ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB17_14 Depth 2
                                        ;     Child Loop BB17_19 Depth 2
                                        ;       Child Loop BB17_21 Depth 3
                                        ;       Child Loop BB17_23 Depth 3
	s_and_saveexec_b32 s2, s1
	s_cbranch_execz .LBB17_16
; %bb.12:                               ;   in Loop: Header=BB17_11 Depth=1
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v9, v17 :: v_dual_mov_b32 v10, v16
	v_mov_b32_e32 v2, v0
	s_mov_b32 s5, 0
	s_branch .LBB17_14
.LBB17_13:                              ;   in Loop: Header=BB17_14 Depth=2
	s_or_b32 exec_lo, exec_lo, s6
	v_add_nc_u32_e32 v2, 64, v2
	v_add_nc_u32_e32 v10, 0x200, v10
	v_add_nc_u32_e32 v9, 0x200, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s48, v2
	s_or_b32 s5, vcc_lo, s5
	s_and_not1_b32 exec_lo, exec_lo, s5
	s_cbranch_execz .LBB17_16
.LBB17_14:                              ;   Parent Loop BB17_11 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v11, v2, v15
	s_mov_b32 s6, exec_lo
	v_mul_lo_u32 v12, v11, s52
	v_add_nc_u32_e32 v18, 1, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v12, v2, v12
	v_subrev_nc_u32_e32 v19, s52, v12
	v_cmp_le_u32_e32 vcc_lo, s52, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v11, v11, v18 :: v_dual_cndmask_b32 v12, v12, v19
	v_add_nc_u32_e32 v18, 1, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s52, v12
	v_cndmask_b32_e32 v11, v11, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v11, s56, v11
	v_subrev_nc_u32_e32 v11, s56, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v12, s53, v11
	v_cmpx_gt_i32_e64 s16, v12
	s_cbranch_execz .LBB17_13
; %bb.15:                               ;   in Loop: Header=BB17_14 Depth=2
	v_ashrrev_i32_e32 v18, 31, v12
	v_add_co_u32 v12, vcc_lo, s54, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v20, null, s50, v18, vcc_lo
	v_mul_lo_u32 v22, v12, s49
	v_mad_u64_u32 v[18:19], null, v12, s55, s[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v12, v20, s55
	v_mad_u64_u32 v[20:21], null, s57, v11, v[2:3]
	v_add3_u32 v12, v12, v19, v22
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v11, vcc_lo, v18, v20
	v_add_co_ci_u32_e64 v12, null, 0, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[11:12], 3, v[11:12]
	v_add_co_u32 v18, vcc_lo, s10, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v19, null, s11, v12, vcc_lo
	v_add_co_u32 v11, vcc_lo, s12, v11
	v_add_co_ci_u32_e64 v12, null, s13, v12, vcc_lo
	global_load_b64 v[18:19], v[18:19], off
	global_load_b64 v[11:12], v[11:12], off
	s_waitcnt vmcnt(1)
	ds_store_b64 v10, v[18:19]
	s_waitcnt vmcnt(0)
	ds_store_b64 v9, v[11:12]
	s_branch .LBB17_13
.LBB17_16:                              ;   in Loop: Header=BB17_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	s_cmp_gt_i32 s16, s53
	s_waitcnt lgkmcnt(0)
	s_cselect_b32 s2, -1, 0
	s_barrier
	s_and_b32 s2, s0, s2
	buffer_gl0_inv
	s_and_saveexec_b32 s60, s2
	s_cbranch_execz .LBB17_10
; %bb.17:                               ;   in Loop: Header=BB17_11 Depth=1
	v_med3_i32 v2, s59, 1, 64
	s_mov_b32 s61, 0
	s_mov_b32 s62, s47
	s_mov_b32 s63, s46
	s_branch .LBB17_19
.LBB17_18:                              ;   in Loop: Header=BB17_19 Depth=2
	v_fma_f64 v[3:4], v[3:4], v[7:8], v[11:12]
	s_add_i32 s61, s61, 1
	v_dual_mov_b32 v7, v9 :: v_dual_mov_b32 v8, v10
	v_cmp_eq_u32_e32 vcc_lo, s61, v2
	s_add_i32 s63, s63, s58
	s_add_i32 s62, s62, s58
	s_cbranch_vccnz .LBB17_10
.LBB17_19:                              ;   Parent Loop BB17_11 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB17_21 Depth 3
                                        ;       Child Loop BB17_23 Depth 3
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v12, 0
	s_and_not1_b32 vcc_lo, exec_lo, s51
	s_cbranch_vccnz .LBB17_22
; %bb.20:                               ;   in Loop: Header=BB17_19 Depth=2
	v_mov_b32_e32 v9, v13
	s_mov_b32 s2, s63
	s_mov_b32 s5, s33
.LBB17_21:                              ;   Parent Loop BB17_11 Depth=1
                                        ;     Parent Loop BB17_19 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_mov_b32_e32 v10, s2
	s_add_i32 s5, s5, -1
	s_add_i32 s2, s2, 8
	s_cmp_eq_u32 s5, 0
	ds_load_b64 v[18:19], v9
	ds_load_b64 v[20:21], v10
	v_add_nc_u32_e32 v9, 8, v9
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[11:12], v[18:19], v[20:21], v[11:12]
	s_cbranch_scc0 .LBB17_21
.LBB17_22:                              ;   in Loop: Header=BB17_19 Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[5:6], v[11:12]
	v_cmp_gt_f64_e32 vcc_lo, v[9:10], v[7:8]
	v_dual_cndmask_b32 v10, v8, v10 :: v_dual_cndmask_b32 v9, v7, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_fma_f64 v[11:12], v[5:6], v[11:12], -v[9:10]
	v_mul_f64 v[18:19], v[7:8], s[18:19]
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[20:21], v[11:12], s[18:19]
	v_cmp_nlt_f64_e64 s5, 0x40900000, v[11:12]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[7:8]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[7:8]
	v_cmp_ngt_f64_e64 s6, 0xc090cc00, v[11:12]
	v_rndne_f64_e32 v[18:19], v[18:19]
	v_rndne_f64_e32 v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[18:19], s[20:21], v[7:8]
	v_fma_f64 v[24:25], v[20:21], s[20:21], v[11:12]
	v_cvt_i32_f64_e32 v30, v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], v[18:19], s[22:23], v[22:23]
	v_fma_f64 v[24:25], v[20:21], s[22:23], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], s[26:27], s[24:25]
	v_fma_f64 v[28:29], v[24:25], s[26:27], s[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[28:29]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[30:31]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[34:35]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[36:37]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[38:39]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[38:39]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[40:41]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[40:41]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[42:43]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[42:43]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[44:45]
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[44:45]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], 1.0
	v_fma_f64 v[28:29], v[24:25], v[28:29], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[22:23], v[26:27], 1.0
	v_cvt_i32_f64_e32 v22, v[20:21]
	v_fma_f64 v[20:21], v[24:25], v[28:29], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[18:19], v[18:19], v30
	v_ldexp_f64 v[20:21], v[20:21], v22
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v19, 0x7ff00000, v19, vcc_lo
	s_and_b32 vcc_lo, s2, vcc_lo
	v_cndmask_b32_e64 v7, 0x7ff00000, v21, s5
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v12, 0, v7, s6
	v_cndmask_b32_e32 v7, 0, v18, vcc_lo
	s_and_b32 vcc_lo, s6, s5
	v_mov_b32_e32 v18, v14
	v_cndmask_b32_e64 v8, 0, v19, s2
	v_cndmask_b32_e32 v11, 0, v20, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s51
	s_mov_b32 s2, s62
	s_mov_b32 s5, s33
	s_cbranch_vccnz .LBB17_18
	.p2align	6
.LBB17_23:                              ;   Parent Loop BB17_11 Depth=1
                                        ;     Parent Loop BB17_19 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_mov_b32_e32 v19, s2
	ds_load_b64 v[21:22], v18
	s_add_i32 s5, s5, -1
	s_add_i32 s2, s2, 8
	s_cmp_eq_u32 s5, 0
	ds_load_b64 v[19:20], v19
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[19:20], v[11:12], v[19:20]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[19:20], v[7:8], v[21:22], v[19:20]
	ds_store_b64 v18, v[19:20]
	v_add_nc_u32_e32 v18, 8, v18
	s_cbranch_scc0 .LBB17_23
	s_branch .LBB17_18
.LBB17_24:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB17_25:
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB17_29
; %bb.26:
	s_cmp_lt_i32 s33, 1
	s_cbranch_scc1 .LBB17_29
; %bb.27:
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], 1.0
	v_div_scale_f64 v[11:12], vcc_lo, 1.0, v[3:4], 1.0
	v_ashrrev_i32_e32 v2, 31, v1
	s_ashr_i32 s0, s17, 31
	s_mul_i32 s2, s33, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	s_ashr_i32 s3, s2, 31
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
	v_cmp_lt_f64_e32 vcc_lo, 0, v[3:4]
	v_mad_i64_i32 v[7:8], null, s16, s4, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mad_u64_u32 v[1:2], null, v7, s17, 0
	v_div_fixup_f64 v[5:6], v[5:6], v[3:4], 1.0
	v_mul_lo_u32 v3, v7, s0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, v8, s17
	v_add3_u32 v2, v2, v3, v4
	v_mul_lo_u32 v4, s33, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[1:2], 3, v[1:2]
	v_lshl_add_u32 v4, v4, 3, s7
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, s0, s14, v1
	v_add_co_ci_u32_e64 v1, null, s15, v2, s0
	s_lshl_b64 s[0:1], s[2:3], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, s0, v0, s0
	v_add_co_ci_u32_e64 v1, null, s1, v1, s0
	v_dual_cndmask_b32 v3, 0, v6 :: v_dual_cndmask_b32 v2, 0, v5
.LBB17_28:                              ; =>This Inner Loop Header: Depth=1
	ds_load_b64 v[5:6], v4
	v_add_nc_u32_e32 v4, 8, v4
	s_add_i32 s33, s33, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	s_cmp_lg_u32 s33, 0
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[5:6], v[2:3], v[5:6]
	global_store_b64 v[0:1], v[5:6], off
	v_add_co_u32 v0, vcc_lo, v0, 8
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_cbranch_scc1 .LBB17_28
.LBB17_29:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 48
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
		.amdhsa_system_sgpr_workgroup_id_z 1
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 31
		.amdhsa_next_free_sgpr 64
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
		.amdhsa_inst_pref_size 24
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii,"axG",@progbits,_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii,comdat
.Lfunc_end17:
	.size	_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii, .Lfunc_end17-_Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii
                                        ; -- End function
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.num_vgpr, 31
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.num_agpr, 0
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.numbered_sgpr, 64
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.num_named_barrier, 0
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.private_seg_size, 0
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.uses_vcc, 1
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.uses_flat_scratch, 0
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.has_dyn_sized_stack, 0
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.has_recursion, 0
	.set _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3048
; TotalNumSgprs: 66
; NumVgprs: 31
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 66
; NumVGPRsForWavesPerEU: 31
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 1
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_a91fbdd93d1ec8c8,@object ; @__hip_cuid_a91fbdd93d1ec8c8
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_a91fbdd93d1ec8c8
__hip_cuid_a91fbdd93d1ec8c8:
	.byte	0                               ; 0x0
	.size	__hip_cuid_a91fbdd93d1ec8c8, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_a91fbdd93d1ec8c8
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
      - .offset:         168
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 128
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z15attn_sdp_kernelPKfS0_S0_Pfiii
    .private_segment_fixed_size: 0
    .sgpr_count:     29
    .sgpr_spill_count: 0
    .symbol:         _Z15attn_sdp_kernelPKfS0_S0_Pfiii.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .offset:         12
        .size:           4
        .value_kind:     by_value
      - .offset:         16
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         20
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         28
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         30
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         32
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         34
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         36
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         38
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         56
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         80
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 272
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z26attn_causal_softmax_kernelPfii
    .private_segment_fixed_size: 0
    .sgpr_count:     13
    .sgpr_spill_count: 0
    .symbol:         _Z26attn_causal_softmax_kernelPfii.kd
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
      - .offset:         20
        .size:           4
        .value_kind:     by_value
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
    .name:           _Z21attn_mha_split_kernelPKfPfiii
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         _Z21attn_mha_split_kernelPKfPfiii.kd
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
    .name:           _Z21attn_mha_merge_kernelPKfPfiii
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         _Z21attn_mha_merge_kernelPKfPfiii.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         20
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
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
    .name:           _Z16attn_rope_kernelPKfPfiiS0_
    .private_segment_fixed_size: 0
    .sgpr_count:     11
    .sgpr_spill_count: 0
    .symbol:         _Z16attn_rope_kernelPKfPfiiS0_.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .offset:         12
        .size:           4
        .value_kind:     by_value
      - .offset:         16
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         20
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         28
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         30
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         32
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         34
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         36
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         38
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         56
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         80
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 272
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z19attn_pos_enc_kernelPfii
    .private_segment_fixed_size: 0
    .sgpr_count:     9
    .sgpr_spill_count: 0
    .symbol:         _Z19attn_pos_enc_kernelPfii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     15
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
    .name:           _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z19attn_rmsnorm_kernelPKfS0_PfiiS0_.kd
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
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 312
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         _Z28attn_rmsnorm_backward_kernelPKfS0_S0_PfS1_iiS0_.kd
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
      - .offset:         20
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .offset:         28
        .size:           4
        .value_kind:     by_value
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
        .size:           4
        .value_kind:     by_value
      - .offset:         52
        .size:           4
        .value_kind:     by_value
      - .offset:         56
        .size:           4
        .value_kind:     by_value
      - .offset:         60
        .size:           4
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
    .name:           _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii
    .private_segment_fixed_size: 0
    .sgpr_count:     26
    .sgpr_spill_count: 0
    .symbol:         _Z25attn_im2col_2d_ext_kernelPKfPfiiiiiiiiiiiiii.kd
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
      - .offset:         20
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .offset:         28
        .size:           4
        .value_kind:     by_value
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
        .size:           4
        .value_kind:     by_value
      - .offset:         52
        .size:           4
        .value_kind:     by_value
      - .offset:         56
        .size:           4
        .value_kind:     by_value
      - .offset:         60
        .size:           4
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
    .name:           _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         _Z25attn_col2im_2d_ext_kernelPKfPfiiiiiiiiiiiiii.kd
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
    .name:           _Z30attn_embedding_backward_kernelPKfPKiPfii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z30attn_embedding_backward_kernelPKfPKiPfii.kd
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
    .name:           _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z29attn_bn_update_running_kernelPfS_PKfS1_S1_i.kd
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
        .value_kind:     by_value
      - .offset:         52
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 56
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii
    .private_segment_fixed_size: 0
    .sgpr_count:     69
    .sgpr_spill_count: 0
    .symbol:         _Z31flash_attn_f64_train_fwd_kernelPKdS0_S0_PdS1_iiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     31
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
      - .offset:         36
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
    .name:           _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii
    .private_segment_fixed_size: 0
    .sgpr_count:     19
    .sgpr_spill_count: 0
    .symbol:         _Z26flash_attn_f64_dsum_kernelPKdS0_Pdiiii.kd
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
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
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
        .value_kind:     by_value
      - .offset:         60
        .size:           4
        .value_kind:     by_value
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         68
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 72
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii
    .private_segment_fixed_size: 0
    .sgpr_count:     66
    .sgpr_spill_count: 0
    .symbol:         _Z28flash_attn_f64_bwd_dq_kernelPKdS0_S0_S0_S0_S0_Pdiiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     45
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
      - .address_space:  global
        .offset:         40
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         48
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         56
        .size:           8
        .value_kind:     global_buffer
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         68
        .size:           4
        .value_kind:     by_value
      - .offset:         72
        .size:           4
        .value_kind:     by_value
      - .offset:         76
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 80
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii
    .private_segment_fixed_size: 0
    .sgpr_count:     78
    .sgpr_spill_count: 0
    .symbol:         _Z29flash_attn_f64_bwd_dkv_kernelPKdS0_S0_S0_S0_S0_PdS1_iiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     44
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
      - .offset:         44
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 48
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii
    .private_segment_fixed_size: 0
    .sgpr_count:     39
    .sgpr_spill_count: 0
    .symbol:         _Z21flash_attn_f64_kernelIfEvPKT_S2_S2_PS0_iiii.kd
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
      - .offset:         44
        .size:           4
        .value_kind:     by_value
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 48
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii
    .private_segment_fixed_size: 0
    .sgpr_count:     66
    .sgpr_spill_count: 0
    .symbol:         _Z21flash_attn_f64_kernelIdEvPKT_S2_S2_PS0_iiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     31
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
