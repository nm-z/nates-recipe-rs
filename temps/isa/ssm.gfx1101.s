	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.section	.text._Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_,"axG",@progbits,_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_,comdat
	.protected	_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_ ; -- Begin function _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_
	.globl	_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_
	.p2align	8
	.type	_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_,@function
_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_: ; @_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b128 s[12:15], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_hi_i32 s3, s13, s12
	s_mul_i32 s2, s13, s12
	v_ashrrev_i32_e32 v2, 31, v1
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[1:2]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_18
; %bb.1:
	s_abs_i32 s2, s13
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b256 s[4:11], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v4, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v4, v0
	v_mul_lo_u32 v3, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v4, v3
	v_add_nc_u32_e32 v4, 1, v0
	v_subrev_nc_u32_e32 v5, s2, v3
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v0, v0, v4 :: v_dual_cndmask_b32 v3, v3, v5
	v_xor_b32_e32 v4, s13, v1
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_ashr_i32 s2, s13, 31
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[8:9], 0
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v4
	v_sub_nc_u32_e32 v13, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v13, s13
	v_sub_nc_u32_e32 v5, v1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v6, 31, v5
	v_lshlrev_b64 v[3:4], 2, v[5:6]
	s_cbranch_scc1 .LBB0_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s8, v3
	v_add_co_ci_u32_e64 v7, null, s9, v4, vcc_lo
	global_load_b32 v0, v[6:7], off
	s_cmp_lt_i32 s14, 1
	s_cbranch_scc0 .LBB0_4
	s_branch .LBB0_15
.LBB0_3:
	v_mov_b32_e32 v0, 0
	s_cmp_lt_i32 s14, 1
	s_cbranch_scc1 .LBB0_15
.LBB0_4:
	s_load_b64 s[0:1], s[0:1], 0x30
	v_mad_i64_i32 v[6:7], null, v5, s14, 0
	v_subrev_nc_u32_e32 v14, s14, v13
	s_mov_b32 s3, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[7:8], 2, v[6:7]
	v_mov_b32_e32 v6, 0
	v_add_co_u32 v7, vcc_lo, s6, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s7, v8, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_cmp_lg_u64 s[0:1], 0
	s_cselect_b32 s6, -1, 0
	s_sub_i32 s7, 0, s14
	s_branch .LBB0_6
.LBB0_5:                                ;   in Loop: Header=BB0_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s9
	v_add_co_u32 v7, vcc_lo, v7, 4
	s_add_i32 s3, s3, 1
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	s_add_i32 s8, s7, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s8, 0
	s_cbranch_scc1 .LBB0_15
.LBB0_6:                                ; =>This Inner Loop Header: Depth=1
	v_add3_u32 v5, v14, s3, 1
	v_mov_b32_e32 v10, v6
	s_mov_b32 s8, 0
	s_mov_b32 s9, exec_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_mov_b32_e32 v9, v5
	v_cmpx_gt_i32_e32 0, v5
	s_xor_b32 s9, exec_lo, s9
	s_cbranch_execz .LBB0_11
; %bb.7:                                ;   in Loop: Header=BB0_6 Depth=1
	s_and_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccz .LBB0_9
; %bb.8:                                ;   in Loop: Header=BB0_6 Depth=1
	v_add_nc_u32_e32 v9, s3, v13
	s_mov_b32 s8, -1
	s_delay_alu instid0(VALU_DEP_1)
	v_ashrrev_i32_e32 v10, 31, v9
	s_branch .LBB0_10
.LBB0_9:                                ;   in Loop: Header=BB0_6 Depth=1
                                        ; implicit-def: $vgpr9_vgpr10
.LBB0_10:                               ;   in Loop: Header=BB0_6 Depth=1
	s_and_b32 s8, s8, exec_lo
.LBB0_11:                               ;   in Loop: Header=BB0_6 Depth=1
	s_or_saveexec_b32 s9, s9
	v_dual_mov_b32 v12, s1 :: v_dual_mov_b32 v11, s0
	s_xor_b32 exec_lo, exec_lo, s9
; %bb.12:                               ;   in Loop: Header=BB0_6 Depth=1
	v_dual_mov_b32 v12, s5 :: v_dual_mov_b32 v11, s4
	s_or_b32 s8, s8, exec_lo
; %bb.13:                               ;   in Loop: Header=BB0_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s9
	s_and_saveexec_b32 s9, s8
	s_cbranch_execz .LBB0_5
; %bb.14:                               ;   in Loop: Header=BB0_6 Depth=1
	v_mul_lo_u32 v5, v10, s13
	v_mul_lo_u32 v10, v9, s2
	v_mad_u64_u32 v[15:16], null, v9, s13, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v16, v16, v10, v5
	v_add_co_u32 v5, vcc_lo, v11, v3
	v_add_co_ci_u32_e64 v11, null, v12, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[9:10], 2, v[15:16]
	v_add_co_u32 v9, vcc_lo, v5, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, v11, v10, vcc_lo
	global_load_b32 v5, v[7:8], off
	global_load_b32 v9, v[9:10], off
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v0, v5, v9
	s_branch .LBB0_5
.LBB0_15:
	s_cmp_lg_u32 s15, 0
	s_cbranch_scc0 .LBB0_17
; %bb.16:
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v3, 0xbfb8aa3b, v0
	v_cmp_nlt_f32_e32 vcc_lo, 0x42ce8ed0, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rndne_f32_e32 v4, v3
	v_fma_f32 v5, 0xbfb8aa3b, v0, -v3
	v_sub_f32_e32 v3, v3, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmamk_f32 v5, v0, 0xb2a5705f, v5
	v_cvt_i32_f32_e32 v4, v4
	v_add_f32_e32 v3, v3, v5
	v_cvt_f64_f32_e32 v[5:6], v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v3, v3
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v3, v3, v4
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2b17218, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, 0x7f800000, v3, vcc_lo
	v_cvt_f64_f32_e32 v[3:4], v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], 1.0
	v_div_scale_f64 v[7:8], null, v[3:4], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[9:10], v[7:8]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_div_scale_f64 v[11:12], vcc_lo, v[5:6], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[7:8], -v[7:8], v[13:14], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[7:8], v[7:8], v[9:10], v[13:14]
	v_div_fixup_f64 v[3:4], v[7:8], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_f32_f64_e32 v0, v[3:4]
.LBB0_17:
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, vcc_lo, s10, v1
	v_add_co_ci_u32_e64 v2, null, s11, v2, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b32 v[1:2], v0, off
.LBB0_18:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_
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
		.amdhsa_next_free_vgpr 17
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
	.section	.text._Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_,"axG",@progbits,_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_,comdat
.Lfunc_end0:
	.size	_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_, .Lfunc_end0-_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_
                                        ; -- End function
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.num_vgpr, 17
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.num_agpr, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.numbered_sgpr, 16
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.num_named_barrier, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.private_seg_size, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.uses_vcc, 1
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.uses_flat_scratch, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.has_dyn_sized_stack, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.has_recursion, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 972
; TotalNumSgprs: 18
; NumVgprs: 17
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 18
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
	.section	.text._Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_,"axG",@progbits,_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_,comdat
	.protected	_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_ ; -- Begin function _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_
	.globl	_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_
	.p2align	8
	.type	_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_,@function
_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_: ; @_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b128 s[12:15], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_hi_i32 s3, s13, s12
	s_mul_i32 s2, s13, s12
	v_ashrrev_i32_e32 v2, 31, v1
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[1:2]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_18
; %bb.1:
	s_abs_i32 s2, s13
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b256 s[4:11], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v4, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v4, v0
	v_mul_lo_u32 v3, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v4, v3
	v_add_nc_u32_e32 v4, 1, v0
	v_subrev_nc_u32_e32 v5, s2, v3
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v0, v0, v4 :: v_dual_cndmask_b32 v3, v3, v5
	v_xor_b32_e32 v4, s13, v1
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_ashr_i32 s2, s13, 31
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[8:9], 0
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v4
	v_sub_nc_u32_e32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v3, v0, s13
	v_sub_nc_u32_e32 v7, v1, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	v_lshlrev_b64 v[5:6], 3, v[7:8]
	s_cbranch_scc1 .LBB1_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s8, v5
	v_add_co_ci_u32_e64 v4, null, s9, v6, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_cmp_lt_i32 s14, 1
	s_cbranch_scc0 .LBB1_4
	s_branch .LBB1_15
.LBB1_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_lt_i32 s14, 1
	s_cbranch_scc1 .LBB1_15
.LBB1_4:
	s_load_b64 s[0:1], s[0:1], 0x30
	v_mad_i64_i32 v[8:9], null, v7, s14, 0
	v_subrev_nc_u32_e32 v15, s14, v0
	s_mov_b32 s3, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[9:10], 3, v[8:9]
	v_mov_b32_e32 v8, 0
	v_add_co_u32 v9, vcc_lo, s6, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s7, v10, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_cmp_lg_u64 s[0:1], 0
	s_cselect_b32 s6, -1, 0
	s_sub_i32 s7, 0, s14
	s_branch .LBB1_6
.LBB1_5:                                ;   in Loop: Header=BB1_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s9
	v_add_co_u32 v9, vcc_lo, v9, 8
	s_add_i32 s3, s3, 1
	v_add_co_ci_u32_e64 v10, null, 0, v10, vcc_lo
	s_add_i32 s8, s7, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s8, 0
	s_cbranch_scc1 .LBB1_15
.LBB1_6:                                ; =>This Inner Loop Header: Depth=1
	v_add3_u32 v7, v15, s3, 1
	v_mov_b32_e32 v12, v8
	s_mov_b32 s8, 0
	s_mov_b32 s9, exec_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_mov_b32_e32 v11, v7
	v_cmpx_gt_i32_e32 0, v7
	s_xor_b32 s9, exec_lo, s9
	s_cbranch_execz .LBB1_11
; %bb.7:                                ;   in Loop: Header=BB1_6 Depth=1
	s_and_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccz .LBB1_9
; %bb.8:                                ;   in Loop: Header=BB1_6 Depth=1
	v_add_nc_u32_e32 v11, s3, v0
	s_mov_b32 s8, -1
	s_delay_alu instid0(VALU_DEP_1)
	v_ashrrev_i32_e32 v12, 31, v11
	s_branch .LBB1_10
.LBB1_9:                                ;   in Loop: Header=BB1_6 Depth=1
                                        ; implicit-def: $vgpr11_vgpr12
.LBB1_10:                               ;   in Loop: Header=BB1_6 Depth=1
	s_and_b32 s8, s8, exec_lo
.LBB1_11:                               ;   in Loop: Header=BB1_6 Depth=1
	s_or_saveexec_b32 s9, s9
	v_dual_mov_b32 v14, s1 :: v_dual_mov_b32 v13, s0
	s_xor_b32 exec_lo, exec_lo, s9
; %bb.12:                               ;   in Loop: Header=BB1_6 Depth=1
	v_dual_mov_b32 v14, s5 :: v_dual_mov_b32 v13, s4
	s_or_b32 s8, s8, exec_lo
; %bb.13:                               ;   in Loop: Header=BB1_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s9
	s_and_saveexec_b32 s9, s8
	s_cbranch_execz .LBB1_5
; %bb.14:                               ;   in Loop: Header=BB1_6 Depth=1
	v_mul_lo_u32 v7, v12, s13
	v_mul_lo_u32 v12, v11, s2
	v_mad_u64_u32 v[16:17], null, v11, s13, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v17, v17, v12, v7
	v_add_co_u32 v7, vcc_lo, v13, v5
	v_add_co_ci_u32_e64 v13, null, v14, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[11:12], 3, v[16:17]
	v_add_co_u32 v11, vcc_lo, v7, v11
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v12, null, v13, v12, vcc_lo
	global_load_b64 v[13:14], v[9:10], off
	global_load_b64 v[11:12], v[11:12], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[13:14], v[11:12], v[3:4]
	s_branch .LBB1_5
.LBB1_15:
	s_cmp_lg_u32 s15, 0
	s_cbranch_scc0 .LBB1_17
; %bb.16:
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	s_mov_b32 s2, 0x6a5dcb37
	s_waitcnt vmcnt(0)
	v_mul_f64 v[5:6], v[3:4], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_mov_b32 s3, 0x3e5ade15
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[5:6], v[5:6]
	v_fma_f64 v[7:8], v[5:6], s[0:1], -v[3:4]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	v_cvt_i32_f64_e32 v0, v[5:6]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[7:8], v[5:6], s[0:1], v[7:8]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], 1.0
	v_fma_f64 v[5:6], v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v0
	v_add_f64 v[5:6], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, 0x7ff00000, v6, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v6, 0x3ff00000, v0, s0
	v_div_scale_f64 v[7:8], null, v[5:6], v[5:6], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[9:10], v[7:8]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_div_scale_f64 v[11:12], vcc_lo, v[3:4], v[5:6], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[7:8], -v[7:8], v[13:14], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[7:8], v[7:8], v[9:10], v[13:14]
	v_div_fixup_f64 v[3:4], v[7:8], v[5:6], v[3:4]
.LBB1_17:
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s10, v0
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB1_18:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_
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
		.amdhsa_next_free_vgpr 18
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
		.amdhsa_inst_pref_size 11
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_,"axG",@progbits,_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_,comdat
.Lfunc_end1:
	.size	_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_, .Lfunc_end1-_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_
                                        ; -- End function
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.num_vgpr, 18
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.num_agpr, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.numbered_sgpr, 16
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.num_named_barrier, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.private_seg_size, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.uses_vcc, 1
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.uses_flat_scratch, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.has_dyn_sized_stack, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.has_recursion, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1296
; TotalNumSgprs: 18
; NumVgprs: 18
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 18
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
	.section	.text._Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii,"axG",@progbits,_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii,comdat
	.protected	_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii ; -- Begin function _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii
	.globl	_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii
	.p2align	8
	.type	_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii,@function
_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii: ; @_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_add_i32 s3, s6, -1
	s_mov_b32 s2, exec_lo
	s_mul_hi_i32 s7, s3, s5
	s_mul_i32 s6, s3, s5
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i64_e64 s[6:7], v[1:2]
	s_cbranch_execz .LBB2_11
; %bb.1:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s6, 0, s2
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_max_i32_e32 v4, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s6, v0
	s_mov_b32 s6, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v4, v0
	v_mul_lo_u32 v3, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v4, v3
	v_add_nc_u32_e32 v4, 1, v0
	v_subrev_nc_u32_e32 v5, s2, v3
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v0, v0, v4 :: v_dual_cndmask_b32 v3, v3, v5
	v_xor_b32_e32 v4, s5, v1
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_ashr_i32 s2, s5, 31
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	v_xor_b32_e32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v4
	v_add_nc_u32_e32 v3, s4, v0
	s_mov_b32 s4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_le_i32_e64 s3, v3
	s_xor_b32 s6, exec_lo, s6
; %bb.2:
	v_subrev_nc_u32_e32 v3, s3, v3
	v_mov_b32_e32 v4, 0
	s_mov_b32 s4, exec_lo
; %bb.3:
	s_or_saveexec_b32 s3, s6
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v5, s10 :: v_dual_mov_b32 v6, s11
	s_xor_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB2_8
; %bb.4:
	s_cmp_lg_u64 s[8:9], 0
	s_cbranch_scc0 .LBB2_6
; %bb.5:
	v_ashrrev_i32_e32 v4, 31, v3
	s_or_b32 s6, s4, exec_lo
	s_branch .LBB2_7
.LBB2_6:
	s_mov_b32 s6, s4
                                        ; implicit-def: $vgpr3_vgpr4
.LBB2_7:
	v_dual_mov_b32 v5, s8 :: v_dual_mov_b32 v6, s9
	s_and_not1_b32 s4, s4, exec_lo
	s_and_b32 s6, s6, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 s4, s4, s6
.LBB2_8:
	s_or_b32 exec_lo, exec_lo, s3
	v_mov_b32_e32 v7, 0
	s_and_saveexec_b32 s3, s4
	s_cbranch_execz .LBB2_10
; %bb.9:
	v_mul_lo_u32 v0, v0, s5
	v_mul_lo_u32 v4, v4, s5
	v_mul_lo_u32 v9, v3, s2
	v_mad_u64_u32 v[7:8], null, v3, s5, 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v1, v0
	v_add3_u32 v8, v8, v9, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[7:8], 2, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	v_add_co_u32 v0, vcc_lo, v5, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, v6, v8, vcc_lo
	v_add_co_u32 v3, vcc_lo, v0, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, v5, v4, vcc_lo
	global_load_b32 v7, v[3:4], off
.LBB2_10:
	s_or_b32 exec_lo, exec_lo, s3
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b32 v[0:1], v7, off
.LBB2_11:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii
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
		.amdhsa_inst_pref_size 5
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii,"axG",@progbits,_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii,comdat
.Lfunc_end2:
	.size	_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii, .Lfunc_end2-_Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii
                                        ; -- End function
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.num_vgpr, 10
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.num_agpr, 0
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.numbered_sgpr, 12
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.num_named_barrier, 0
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.private_seg_size, 0
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.uses_vcc, 1
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.has_recursion, 0
	.set _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 564
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
	.section	.text._Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii,"axG",@progbits,_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii,comdat
	.protected	_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii ; -- Begin function _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii
	.globl	_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii
	.p2align	8
	.type	_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii,@function
_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii: ; @_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_add_i32 s3, s6, -1
	s_mov_b32 s2, exec_lo
	s_mul_hi_i32 s7, s3, s5
	s_mul_i32 s6, s3, s5
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i64_e64 s[6:7], v[1:2]
	s_cbranch_execz .LBB3_11
; %bb.1:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s6, 0, s2
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_max_i32_e32 v4, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s6, v0
	s_mov_b32 s6, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v4, v0
	v_mul_lo_u32 v3, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v4, v3
	v_add_nc_u32_e32 v4, 1, v0
	v_subrev_nc_u32_e32 v5, s2, v3
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v0, v0, v4 :: v_dual_cndmask_b32 v3, v3, v5
	v_xor_b32_e32 v4, s5, v1
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_ashr_i32 s2, s5, 31
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	v_xor_b32_e32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v4
	v_add_nc_u32_e32 v3, s4, v0
	s_mov_b32 s4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_le_i32_e64 s3, v3
	s_xor_b32 s6, exec_lo, s6
; %bb.2:
	v_subrev_nc_u32_e32 v3, s3, v3
	v_mov_b32_e32 v4, 0
	s_mov_b32 s4, exec_lo
; %bb.3:
	s_or_saveexec_b32 s3, s6
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v5, s10 :: v_dual_mov_b32 v6, s11
	s_xor_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB3_8
; %bb.4:
	s_cmp_lg_u64 s[8:9], 0
	s_cbranch_scc0 .LBB3_6
; %bb.5:
	v_ashrrev_i32_e32 v4, 31, v3
	s_or_b32 s6, s4, exec_lo
	s_branch .LBB3_7
.LBB3_6:
	s_mov_b32 s6, s4
                                        ; implicit-def: $vgpr3_vgpr4
.LBB3_7:
	v_dual_mov_b32 v5, s8 :: v_dual_mov_b32 v6, s9
	s_and_not1_b32 s4, s4, exec_lo
	s_and_b32 s6, s6, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 s4, s4, s6
.LBB3_8:
	s_or_b32 exec_lo, exec_lo, s3
	v_mov_b32_e32 v7, 0
	v_mov_b32_e32 v8, 0
	s_and_saveexec_b32 s3, s4
	s_cbranch_execz .LBB3_10
; %bb.9:
	v_mul_lo_u32 v0, v0, s5
	v_mul_lo_u32 v4, v4, s5
	v_mul_lo_u32 v9, v3, s2
	v_mad_u64_u32 v[7:8], null, v3, s5, 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v1, v0
	v_add3_u32 v8, v8, v9, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	v_add_co_u32 v0, vcc_lo, v5, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, v6, v8, vcc_lo
	v_add_co_u32 v3, vcc_lo, v0, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, v5, v4, vcc_lo
	global_load_b64 v[7:8], v[3:4], off
.LBB3_10:
	s_or_b32 exec_lo, exec_lo, s3
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[7:8], off
.LBB3_11:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii
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
		.amdhsa_inst_pref_size 5
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii,"axG",@progbits,_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii,comdat
.Lfunc_end3:
	.size	_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii, .Lfunc_end3-_Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii
                                        ; -- End function
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.num_vgpr, 10
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.num_agpr, 0
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.numbered_sgpr, 12
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.num_named_barrier, 0
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.private_seg_size, 0
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.uses_vcc, 1
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.has_recursion, 0
	.set _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 568
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
	.section	.text._Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,"axG",@progbits,_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,comdat
	.protected	_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_ ; -- Begin function _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
	.globl	_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
	.p2align	8
	.type	_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,@function
_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_: ; @_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x5c
	s_load_b128 s[20:23], s[0:1], 0x38
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s21, v1
	s_cbranch_execz .LBB4_21
; %bb.1:
	s_load_b64 s[2:3], s[0:1], 0x48
	s_cmp_gt_i32 s22, 0
	v_ashrrev_i32_e32 v2, 31, v1
	s_cselect_b32 s24, -1, 0
	s_mov_b32 s4, 0
	s_and_b32 vcc_lo, exec_lo, s24
	s_cbranch_vccz .LBB4_3
; %bb.2:
	s_mov_b32 s4, -1
.LBB4_3:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccnz .LBB4_9
; %bb.4:
	v_mad_u64_u32 v[3:4], null, v1, s22, 0
	s_waitcnt lgkmcnt(0)
	s_cmp_lg_u64 s[2:3], 0
	s_mov_b32 s5, 0
	s_cselect_b32 s4, -1, 0
	s_mov_b32 s6, s22
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v0, v4
	v_mad_u64_u32 v[4:5], null, v2, s22, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	v_add_co_u32 v3, vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s3, v4, vcc_lo
	s_branch .LBB4_7
.LBB4_5:                                ;   in Loop: Header=BB4_7 Depth=1
	global_load_b32 v0, v[3:4], off
.LBB4_6:                                ;   in Loop: Header=BB4_7 Depth=1
	v_add_co_u32 v3, vcc_lo, v3, 4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	s_add_i32 s6, s6, -1
	s_waitcnt vmcnt(0)
	scratch_store_b32 off, v0, s5
	s_add_i32 s5, s5, 4
	s_cmp_eq_u32 s6, 0
	s_cbranch_scc1 .LBB4_9
.LBB4_7:                                ; =>This Inner Loop Header: Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccz .LBB4_5
; %bb.8:                                ;   in Loop: Header=BB4_7 Depth=1
	v_mov_b32_e32 v0, 0
	s_branch .LBB4_6
.LBB4_9:
	s_ashr_i32 s23, s22, 31
	v_mul_lo_u32 v0, v2, s22
	v_mul_lo_u32 v5, v1, s23
	v_mad_u64_u32 v[3:4], null, v1, s22, 0
	s_cmp_lt_i32 s20, 1
	v_add3_u32 v4, v4, v5, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	s_cbranch_scc1 .LBB4_18
; %bb.10:
	s_load_b256 s[4:11], s[0:1], 0x20
	v_lshlrev_b64 v[5:6], 2, v[1:2]
	s_load_b256 s[12:19], s[0:1], 0x0
	v_mov_b32_e32 v11, 0
	s_ashr_i32 s1, s21, 31
	s_mov_b32 s25, 0x3e9b6dac
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v5, vcc_lo, s6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s7, v6, vcc_lo
	s_lshl_b64 s[6:7], s[22:23], 2
	s_mov_b32 s23, 0
	global_load_b32 v0, v[5:6], off
	v_add_co_u32 v5, vcc_lo, s16, v3
	v_add_co_ci_u32_e64 v6, null, s17, v4, vcc_lo
	s_branch .LBB4_13
.LBB4_11:                               ;   in Loop: Header=BB4_13 Depth=1
	v_mov_b32_e32 v13, 0
.LBB4_12:                               ;   in Loop: Header=BB4_13 Depth=1
	v_add_co_u32 v7, vcc_lo, s8, v7
	s_add_i32 s23, s23, 1
	s_add_u32 s18, s18, s6
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v13, v0, v14
	v_add_co_ci_u32_e64 v8, null, s9, v8, vcc_lo
	s_addc_u32 s19, s19, s7
	s_add_u32 s4, s4, s6
	s_addc_u32 s5, s5, s7
	s_cmp_eq_u32 s23, s20
	global_store_b32 v[7:8], v13, off
	s_cbranch_scc1 .LBB4_18
.LBB4_13:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB4_17 Depth 2
	v_mad_u64_u32 v[7:8], null, s23, s21, v[1:2]
	s_mov_b32 s0, exec_lo
	v_mad_u64_u32 v[9:10], null, s23, s1, v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v8, v9
	v_lshlrev_b64 v[7:8], 2, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v9, vcc_lo, s14, v7
	v_add_co_ci_u32_e64 v10, null, s15, v8, vcc_lo
	global_load_b32 v12, v[9:10], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f32_e32 0x41a00000, v12
	s_cbranch_execz .LBB4_15
; %bb.14:                               ;   in Loop: Header=BB4_13 Depth=1
	v_mul_f32_e32 v9, 0x3fb8aa3b, v12
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rndne_f32_e32 v10, v9
	v_fma_f32 v13, 0x3fb8aa3b, v12, -v9
	v_sub_f32_e32 v9, v9, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v13, 0x32a5705f, v12
	v_cvt_i32_f32_e32 v10, v10
	v_add_f32_e32 v9, v9, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v9, v9
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v9, v9, v10
	v_cndmask_b32_e32 v9, 0, v9, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v12, 0x7f800000, v9, vcc_lo
	v_add_f32_e32 v13, 1.0, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[9:10], v13
	v_frexp_exp_i32_f64_e32 v9, v[9:10]
	v_frexp_mant_f32_e32 v10, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v10
	v_add_f32_e32 v10, -1.0, v13
	v_sub_f32_e32 v15, v10, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v15, 1.0, v15 :: v_dual_sub_f32 v10, v12, v10
	v_add_f32_e32 v10, v10, v15
	v_subrev_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	v_cmp_neq_f32_e32 vcc_lo, 0x7f800000, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v14, 0, v9
	v_cvt_f32_i32_e32 v9, v9
	v_ldexp_f32 v13, v13, v14
	v_ldexp_f32 v10, v10, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v16, 1.0, v13
	v_dual_add_f32 v14, -1.0, v13 :: v_dual_add_f32 v15, -1.0, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v17, 1.0, v14
	v_sub_f32_e32 v15, v13, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v13, v13, v17
	v_add_f32_e32 v15, v10, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v10, v10, v13
	v_dual_add_f32 v18, v14, v10 :: v_dual_add_f32 v17, v16, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v14, v14, v18
	v_rcp_f32_e32 v13, v17
	v_sub_f32_e32 v16, v16, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_dual_add_f32 v10, v10, v14 :: v_dual_add_f32 v15, v15, v16
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v19, v18, v13
	v_mul_f32_e32 v20, v17, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v16, v19, v17, -v20
	v_fmac_f32_e32 v16, v19, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v21, v20, v16
	v_sub_f32_e32 v22, v18, v21
	v_sub_f32_e32 v14, v21, v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v18, v18, v22
	v_sub_f32_e32 v14, v14, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v18, v18, v21
	v_add_f32_e32 v10, v10, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v10, v14, v10
	v_add_f32_e32 v14, v22, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v16, v13, v14
	v_dual_sub_f32 v21, v22, v14 :: v_dual_mul_f32 v18, v17, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v10, v10, v21
	v_fma_f32 v17, v16, v17, -v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v17, v16, v15
	v_add_f32_e32 v15, v18, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v20, v14, v15
	v_sub_f32_e32 v14, v14, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v14, v14, v15
	v_add_f32_e32 v10, v10, v14
	v_add_f32_e32 v14, v19, v16
	v_sub_f32_e32 v18, v15, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v15, v18, v17
	v_dual_add_f32 v10, v15, v10 :: v_dual_sub_f32 v15, v14, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v10, v20, v10
	v_dual_sub_f32 v15, v16, v15 :: v_dual_mul_f32 v10, v13, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v10, v15, v10
	v_add_f32_e32 v13, v14, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v15, v13, v13
	v_fmaak_f32 v16, s25, v15, 0x3ecc95a3
	v_mul_f32_e32 v17, v13, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fmaak_f32 v15, v15, v16, 0x3f2aaada
	v_ldexp_f32 v16, v13, 1
	v_sub_f32_e32 v13, v13, v14
	v_mul_f32_e32 v15, v17, v15
	v_mul_f32_e32 v17, 0x3f317218, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v10, v10, v13
	v_ldexp_f32 v10, v10, 1
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v14, v16, v15
	v_sub_f32_e32 v13, v14, v16
	v_fma_f32 v16, 0x3f317218, v9, -v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v13, v15, v13
	v_fmac_f32_e32 v16, 0xb102e308, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v9, v10, v13 :: v_dual_add_f32 v10, v17, v16
	v_add_f32_e32 v13, v14, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v17, v10, v17
	v_dual_add_f32 v15, v10, v13 :: v_dual_sub_f32 v14, v13, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v18, v15, v10
	v_sub_f32_e32 v9, v9, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v19, v15, v18
	v_dual_sub_f32 v13, v13, v18 :: v_dual_sub_f32 v16, v16, v17
	v_sub_f32_e32 v10, v10, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v14, v16, v9
	v_add_f32_e32 v10, v13, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v10, v14, v10
	v_add_f32_e32 v17, v15, v10
	v_sub_f32_e32 v13, v14, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v14, v14, v13
	v_sub_f32_e32 v9, v9, v13
	v_dual_sub_f32 v13, v17, v15 :: v_dual_sub_f32 v14, v16, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v10, v10, v13 :: v_dual_add_f32 v9, v9, v14
	v_add_f32_e32 v9, v9, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v17, v9
	v_cndmask_b32_e32 v9, 0x7f800000, v9, vcc_lo
	v_cmp_gt_f32_e32 vcc_lo, 0x33800000, v12
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v12, v9, v12, vcc_lo
.LBB4_15:                               ;   in Loop: Header=BB4_13 Depth=1
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v9, vcc_lo, s12, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s13, v8, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s24
	global_load_b32 v14, v[9:10], off
	s_cbranch_vccnz .LBB4_11
; %bb.16:                               ;   in Loop: Header=BB4_13 Depth=1
	s_waitcnt vmcnt(0)
	v_dual_mul_f32 v15, v12, v14 :: v_dual_mov_b32 v10, v6
	v_mov_b32_e32 v13, 0
	v_mov_b32_e32 v9, v5
	s_mov_b32 s26, 0
	s_mov_b64 s[10:11], s[4:5]
	s_mov_b64 s[16:17], s[18:19]
	s_mov_b32 s27, s22
.LBB4_17:                               ;   Parent Loop BB4_13 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b32 v16, v[9:10], off
	global_load_b32 v17, v11, s[16:17]
	scratch_load_b32 v18, off, s26
	global_load_b32 v19, v11, s[10:11]
	s_add_i32 s27, s27, -1
	s_add_u32 s16, s16, 4
	s_addc_u32 s17, s17, 0
	s_add_u32 s10, s10, 4
	s_addc_u32 s11, s11, 0
	s_waitcnt vmcnt(2)
	v_dual_mul_f32 v16, v12, v16 :: v_dual_mul_f32 v17, v15, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v20, 0x3fb8aa3b, v16
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v16
	v_cmp_nlt_f32_e64 s0, 0x42b17218, v16
	v_fma_f32 v21, 0x3fb8aa3b, v16, -v20
	v_rndne_f32_e32 v22, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v21, 0x32a5705f, v16 :: v_dual_sub_f32 v20, v20, v22
	v_add_f32_e32 v20, v20, v21
	v_cvt_i32_f32_e32 v21, v22
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v20, v20
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v20, v20, v21
	v_cndmask_b32_e32 v20, 0, v20, vcc_lo
	v_add_co_u32 v9, vcc_lo, v9, 4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v10, null, 0, v10, vcc_lo
	v_cndmask_b32_e64 v16, 0x7f800000, v20, s0
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v17, v18, v16
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v13, v17, v19
	scratch_store_b32 off, v17, s26
	s_add_i32 s26, s26, 4
	s_cmp_eq_u32 s27, 0
	s_cbranch_scc0 .LBB4_17
	s_branch .LBB4_12
.LBB4_18:
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[2:3], 0
	s_cselect_b32 s0, -1, 0
	s_xor_b32 s1, s24, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_or_b32 s0, s0, s1
	s_and_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccnz .LBB4_21
; %bb.19:
	v_add_co_u32 v0, vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v4, vcc_lo
	s_mov_b32 s0, 0
.LBB4_20:                               ; =>This Inner Loop Header: Depth=1
	scratch_load_b32 v2, off, s0
	s_add_i32 s22, s22, -1
	s_add_i32 s0, s0, 4
	s_cmp_lg_u32 s22, 0
	s_waitcnt vmcnt(0)
	global_store_b32 v[0:1], v2, off
	v_add_co_u32 v0, vcc_lo, v0, 4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_cbranch_scc1 .LBB4_20
.LBB4_21:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 1040
		.amdhsa_kernarg_size 336
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 1
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 23
		.amdhsa_next_free_sgpr 28
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
	.section	.text._Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,"axG",@progbits,_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,comdat
.Lfunc_end4:
	.size	_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_, .Lfunc_end4-_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
                                        ; -- End function
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.num_vgpr, 23
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.num_agpr, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.numbered_sgpr, 28
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.num_named_barrier, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.private_seg_size, 1040
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.uses_vcc, 1
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.uses_flat_scratch, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.has_dyn_sized_stack, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.has_recursion, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1748
; TotalNumSgprs: 30
; NumVgprs: 23
; ScratchSize: 1040
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 30
; NumVGPRsForWavesPerEU: 23
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,"axG",@progbits,_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,comdat
	.protected	_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_ ; -- Begin function _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
	.globl	_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
	.p2align	8
	.type	_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,@function
_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_: ; @_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x5c
	s_load_b128 s[20:23], s[0:1], 0x38
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s21, v1
	s_cbranch_execz .LBB5_21
; %bb.1:
	s_load_b64 s[2:3], s[0:1], 0x48
	s_cmp_gt_i32 s22, 0
	v_ashrrev_i32_e32 v2, 31, v1
	s_cselect_b32 s33, -1, 0
	s_mov_b32 s4, 0
	s_and_b32 vcc_lo, exec_lo, s33
	s_cbranch_vccz .LBB5_3
; %bb.2:
	s_mov_b32 s4, -1
.LBB5_3:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccnz .LBB5_9
; %bb.4:
	v_mad_u64_u32 v[3:4], null, v1, s22, 0
	s_waitcnt lgkmcnt(0)
	s_cmp_lg_u64 s[2:3], 0
	s_mov_b32 s5, 0
	s_cselect_b32 s4, -1, 0
	s_mov_b32 s6, s22
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v0, v4
	v_mad_u64_u32 v[4:5], null, v2, s22, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	v_add_co_u32 v3, vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s3, v4, vcc_lo
	s_branch .LBB5_7
	.p2align	6
.LBB5_5:                                ;   in Loop: Header=BB5_7 Depth=1
	global_load_b64 v[5:6], v[3:4], off
.LBB5_6:                                ;   in Loop: Header=BB5_7 Depth=1
	v_add_co_u32 v3, vcc_lo, v3, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	s_add_i32 s6, s6, -1
	s_waitcnt vmcnt(0)
	scratch_store_b64 off, v[5:6], s5
	s_add_i32 s5, s5, 8
	s_cmp_eq_u32 s6, 0
	s_cbranch_scc1 .LBB5_9
.LBB5_7:                                ; =>This Inner Loop Header: Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccz .LBB5_5
; %bb.8:                                ;   in Loop: Header=BB5_7 Depth=1
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_branch .LBB5_6
.LBB5_9:
	s_ashr_i32 s23, s22, 31
	v_mul_lo_u32 v0, v2, s22
	v_mul_lo_u32 v5, v1, s23
	v_mad_u64_u32 v[3:4], null, v1, s22, 0
	s_cmp_lt_i32 s20, 1
	v_add3_u32 v4, v4, v5, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_cbranch_scc1 .LBB5_18
; %bb.10:
	s_load_b256 s[4:11], s[0:1], 0x20
	v_lshlrev_b64 v[5:6], 3, v[1:2]
	s_load_b256 s[12:19], s[0:1], 0x0
	v_mov_b32_e32 v0, 0
	s_waitcnt lgkmcnt(0)
	s_mov_b32 s10, 0x652b82fe
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
	s_mov_b32 s48, 0x55555555
	s_mov_b32 s50, 0x6b47b09a
	s_mov_b32 s52, 0xbf559e2b
	s_mov_b32 s54, 0xd7f4df2e
	v_add_co_u32 v5, vcc_lo, s6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s7, v6, vcc_lo
	v_add_co_u32 v7, vcc_lo, s16, v3
	v_add_co_ci_u32_e64 v8, null, s17, v4, vcc_lo
	global_load_b64 v[5:6], v[5:6], off
	s_mov_b32 s16, 0xfefa39ef
	s_mov_b32 s56, 0x16291751
	s_mov_b32 s58, 0x9b27acf1
	s_mov_b32 s60, 0x998ef7b6
	s_ashr_i32 s72, s21, 31
	s_lshl_b64 s[6:7], s[22:23], 3
	s_mov_b32 s23, 0
	s_mov_b32 s11, 0x3ff71547
	s_mov_b32 s17, 0xbfe62e42
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
	s_mov_b32 s49, 0x3fe55555
	s_mov_b32 s51, 0x3fc38538
	s_mov_b32 s53, 0x3fc3ab76
	s_mov_b32 s55, 0x3fc7474d
	s_mov_b32 s57, 0x3fcc71c0
	s_mov_b32 s59, 0x3fd24924
	s_mov_b32 s61, 0x3fd99999
	s_mov_b32 s62, 0x55555780
	s_mov_b32 s65, 0x3fe62e42
	s_mov_b32 s67, 0x3c7abc9e
	s_branch .LBB5_13
.LBB5_11:                               ;   in Loop: Header=BB5_13 Depth=1
	v_mov_b32_e32 v15, 0
	v_mov_b32_e32 v16, 0
.LBB5_12:                               ;   in Loop: Header=BB5_13 Depth=1
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[11:12], v[5:6], v[13:14], v[15:16]
	v_add_co_u32 v9, vcc_lo, s8, v9
	s_add_i32 s23, s23, 1
	s_add_u32 s18, s18, s6
	v_add_co_ci_u32_e64 v10, null, s9, v10, vcc_lo
	s_addc_u32 s19, s19, s7
	s_add_u32 s4, s4, s6
	s_addc_u32 s5, s5, s7
	s_cmp_eq_u32 s23, s20
	global_store_b64 v[9:10], v[11:12], off
	s_cbranch_scc1 .LBB5_18
.LBB5_13:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB5_17 Depth 2
	v_mad_u64_u32 v[9:10], null, s23, s21, v[1:2]
	s_mov_b32 s68, exec_lo
	v_mad_u64_u32 v[11:12], null, s23, s72, v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v10, v11
	v_lshlrev_b64 v[9:10], 3, v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v11, vcc_lo, s14, v9
	v_add_co_ci_u32_e64 v12, null, s15, v10, vcc_lo
	global_load_b64 v[11:12], v[11:12], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f64_e32 0x40340000, v[11:12]
	s_cbranch_execz .LBB5_15
; %bb.14:                               ;   in Loop: Header=BB5_13 Depth=1
	v_mul_f64 v[13:14], v[11:12], s[10:11]
	v_cmp_ngt_f64_e32 vcc_lo, 0xc090cc00, v[11:12]
	s_mov_b32 s63, s49
	s_mov_b32 s64, s16
	s_mov_b32 s66, s24
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[13:14], v[13:14]
	v_fma_f64 v[15:16], v[13:14], s[16:17], v[11:12]
	v_cvt_i32_f64_e32 v19, v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], s[24:25], v[15:16]
	v_fma_f64 v[17:18], v[15:16], s[28:29], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[30:31]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[36:37]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[40:41]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[44:45]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[46:47]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[13:14], v[13:14], v19
	v_dual_cndmask_b32 v14, 0, v14 :: v_dual_cndmask_b32 v13, 0, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[13:14], 1.0
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[13:14]
	v_cmp_ngt_f64_e64 s1, -1.0, v[13:14]
	v_frexp_mant_f64_e32 v[15:16], v[11:12]
	v_frexp_exp_i32_f64_e32 v19, v[11:12]
	v_add_f64 v[17:18], v[11:12], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, s[48:49], v[15:16]
	v_add_f64 v[15:16], v[17:18], -v[11:12]
	v_add_f64 v[17:18], v[13:14], -v[17:18]
	v_subrev_co_ci_u32_e64 v35, null, 0, v19, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], 1.0
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[13:14]
	v_sub_nc_u32_e32 v21, 0, v35
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[11:12], v[11:12], v21
	v_add_f64 v[15:16], v[17:18], v[15:16]
	s_and_b32 vcc_lo, vcc_lo, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[11:12], 1.0
	v_add_f64 v[25:26], v[11:12], -1.0
	v_ldexp_f64 v[15:16], v[15:16], v21
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[19:20], -1.0
	v_add_f64 v[27:28], v[25:26], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[11:12], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[15:16], v[17:18]
	v_add_f64 v[11:12], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[19:20], v[17:18]
	v_add_f64 v[27:28], v[25:26], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[23:24], v[21:22]
	v_add_f64 v[19:20], v[21:22], -v[19:20]
	v_add_f64 v[25:26], v[27:28], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[17:18], -v[19:20]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[29:30], -v[21:22], v[23:24], 1.0
	v_add_f64 v[11:12], v[11:12], -v[25:26]
	v_fma_f64 v[23:24], v[29:30], v[23:24], v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], -v[21:22], v[23:24], 1.0
	v_fma_f64 v[15:16], v[15:16], v[23:24], v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[23:24], v[27:28], v[15:16]
	v_mul_f64 v[29:30], v[21:22], v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[23:24], v[21:22], -v[29:30]
	v_fma_f64 v[19:20], v[23:24], v[17:18], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[31:32], v[29:30], v[19:20]
	v_add_f64 v[33:34], v[27:28], -v[31:32]
	v_add_f64 v[25:26], v[31:32], -v[29:30]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[27:28], v[27:28], -v[33:34]
	v_add_f64 v[19:20], v[25:26], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[27:28], v[27:28], -v[31:32]
	v_add_f64 v[11:12], v[11:12], v[27:28]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[19:20], v[11:12]
	v_add_f64 v[19:20], v[33:34], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[25:26], v[15:16], v[19:20]
	v_add_f64 v[31:32], v[33:34], -v[19:20]
	v_mul_f64 v[27:28], v[21:22], v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], v[31:32]
	v_fma_f64 v[21:22], v[25:26], v[21:22], -v[27:28]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[25:26], v[17:18], v[21:22]
	v_add_f64 v[21:22], v[27:28], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[29:30], v[19:20], -v[21:22]
	v_add_f64 v[27:28], v[21:22], -v[27:28]
	v_add_f64 v[19:20], v[19:20], -v[29:30]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[27:28], -v[17:18]
	v_add_f64 v[19:20], v[19:20], -v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], v[19:20]
	v_add_f64 v[19:20], v[23:24], v[25:26]
	v_add_f64 v[11:12], v[17:18], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[23:24]
	v_add_f64 v[11:12], v[29:30], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[25:26], -v[17:18]
	v_mul_f64 v[11:12], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[17:18], v[11:12]
	v_add_f64 v[15:16], v[19:20], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[15:16], v[15:16]
	v_fma_f64 v[21:22], v[17:18], s[52:53], s[50:51]
	v_mul_f64 v[23:24], v[15:16], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[54:55]
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[56:57]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[58:59]
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[60:61]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[17:18], v[21:22], s[62:63]
	v_ldexp_f64 v[21:22], v[15:16], 1
	v_add_f64 v[15:16], v[15:16], -v[19:20]
	v_mul_f64 v[17:18], v[23:24], v[17:18]
	v_cvt_f64_i32_e32 v[23:24], v35
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[15:16]
	v_add_f64 v[19:20], v[21:22], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[25:26], v[23:24], s[64:65]
	v_ldexp_f64 v[11:12], v[11:12], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[19:20], -v[21:22]
	v_fma_f64 v[21:22], v[23:24], s[64:65], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[17:18], -v[15:16]
	v_fma_f64 v[17:18], v[23:24], s[66:67], v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_add_f64 v[15:16], v[25:26], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[19:20], v[11:12]
	v_add_f64 v[25:26], v[15:16], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[15:16], v[21:22]
	v_add_f64 v[19:20], v[21:22], -v[19:20]
	v_add_f64 v[17:18], v[17:18], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[27:28], v[23:24], -v[15:16]
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[29:30], v[23:24], -v[27:28]
	v_add_f64 v[19:20], v[21:22], -v[27:28]
	v_add_f64 v[21:22], v[17:18], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[15:16], -v[29:30]
	v_add_f64 v[15:16], v[19:20], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[21:22], -v[17:18]
	v_add_f64 v[15:16], v[21:22], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[21:22], -v[19:20]
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	v_add_f64 v[25:26], v[23:24], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[17:18], -v[21:22]
	v_add_f64 v[19:20], v[25:26], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], v[17:18]
	v_add_f64 v[15:16], v[15:16], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_add_f64 v[11:12], v[25:26], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v11, 0, v11, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[13:14]
	v_cndmask_b32_e64 v12, 0x7ff00000, v12, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v12, 0x7ff80000, v12, s1
	v_cndmask_b32_e32 v12, 0xfff00000, v12, vcc_lo
.LBB5_15:                               ;   in Loop: Header=BB5_13 Depth=1
	s_or_b32 exec_lo, exec_lo, s68
	v_add_co_u32 v13, vcc_lo, s12, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v14, null, s13, v10, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s33
	global_load_b64 v[13:14], v[13:14], off
	s_cbranch_vccnz .LBB5_11
; %bb.16:                               ;   in Loop: Header=BB5_13 Depth=1
	s_waitcnt vmcnt(0)
	v_mul_f64 v[17:18], v[11:12], v[13:14]
	v_dual_mov_b32 v15, 0 :: v_dual_mov_b32 v20, v8
	v_dual_mov_b32 v16, 0 :: v_dual_mov_b32 v19, v7
	s_mov_b32 s1, 0
	s_mov_b64 s[68:69], s[4:5]
	s_mov_b64 s[70:71], s[18:19]
	s_mov_b32 s63, s22
.LBB5_17:                               ;   Parent Loop BB5_13 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[21:22], v[19:20], off
	scratch_load_b64 v[29:30], off, s1
	s_clause 0x1
	global_load_b64 v[31:32], v0, s[70:71]
	global_load_b64 v[33:34], v0, s[68:69]
	s_add_i32 s63, s63, -1
	s_waitcnt vmcnt(3)
	v_mul_f64 v[21:22], v[11:12], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[23:24], v[21:22], s[10:11]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[21:22]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[21:22]
	v_rndne_f64_e32 v[23:24], v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[25:26], v[23:24], s[16:17], v[21:22]
	v_cvt_i32_f64_e32 v35, v[23:24]
	v_fma_f64 v[25:26], v[23:24], s[24:25], v[25:26]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[27:28], v[25:26], s[28:29], s[26:27]
	v_fma_f64 v[27:28], v[25:26], v[27:28], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[27:28], v[25:26], v[27:28], s[34:35]
	v_fma_f64 v[27:28], v[25:26], v[27:28], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[27:28], v[25:26], v[27:28], s[38:39]
	v_fma_f64 v[27:28], v[25:26], v[27:28], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[27:28], v[25:26], v[27:28], s[42:43]
	v_fma_f64 v[27:28], v[25:26], v[27:28], s[44:45]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[27:28], v[25:26], v[27:28], s[46:47]
	v_fma_f64 v[27:28], v[25:26], v[27:28], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[25:26], v[27:28], 1.0
	v_ldexp_f64 v[23:24], v[23:24], v35
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v24, 0x7ff00000, v24, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_add_u32 s70, s70, 8
	v_cndmask_b32_e32 v21, 0, v23, vcc_lo
	v_add_co_u32 v19, vcc_lo, v19, 8
	v_cndmask_b32_e64 v22, 0, v24, s0
	v_add_co_ci_u32_e64 v20, null, 0, v20, vcc_lo
	s_addc_u32 s71, s71, 0
	s_add_u32 s68, s68, 8
	s_waitcnt vmcnt(2)
	v_mul_f64 v[21:22], v[29:30], v[21:22]
	s_addc_u32 s69, s69, 0
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[21:22], v[17:18], v[31:32], v[21:22]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[15:16], v[33:34], v[21:22], v[15:16]
	scratch_store_b64 off, v[21:22], s1
	s_add_i32 s1, s1, 8
	s_cmp_eq_u32 s63, 0
	s_cbranch_scc0 .LBB5_17
	s_branch .LBB5_12
.LBB5_18:
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[2:3], 0
	s_cselect_b32 s0, -1, 0
	s_xor_b32 s1, s33, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_or_b32 s0, s0, s1
	s_and_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccnz .LBB5_21
; %bb.19:
	v_add_co_u32 v0, vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v4, vcc_lo
	s_mov_b32 s0, 0
.LBB5_20:                               ; =>This Inner Loop Header: Depth=1
	scratch_load_b64 v[2:3], off, s0
	s_add_i32 s22, s22, -1
	s_add_i32 s0, s0, 8
	s_cmp_lg_u32 s22, 0
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[2:3], off
	v_add_co_u32 v0, vcc_lo, v0, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_cbranch_scc1 .LBB5_20
.LBB5_21:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 2064
		.amdhsa_kernarg_size 336
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 1
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 36
		.amdhsa_next_free_sgpr 73
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
		.amdhsa_inst_pref_size 22
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,"axG",@progbits,_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_,comdat
.Lfunc_end5:
	.size	_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_, .Lfunc_end5-_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
                                        ; -- End function
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.num_vgpr, 36
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.num_agpr, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.numbered_sgpr, 73
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.num_named_barrier, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.private_seg_size, 2064
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.uses_vcc, 1
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.uses_flat_scratch, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.has_dyn_sized_stack, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.has_recursion, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2780
; TotalNumSgprs: 75
; NumVgprs: 36
; ScratchSize: 2064
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 75
; NumVGPRsForWavesPerEU: 36
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_,"axG",@progbits,_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_,comdat
	.protected	_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_ ; -- Begin function _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_
	.globl	_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_
	.p2align	8
	.type	_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_,@function
_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_: ; @_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x54
	s_load_b128 s[12:15], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s13, v1
	s_cbranch_execz .LBB6_19
; %bb.1:
	s_abs_i32 s2, s15
	s_abs_i32 s5, s13
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s4, 0, s2
	s_ashr_i32 s21, s13, 31
	s_ashr_i32 s22, s15, 31
	s_load_b128 s[16:19], s[0:1], 0x38
	v_rcp_iflag_f32_e32 v0, v0
	s_xor_b32 s6, s21, s22
	v_sub_nc_u32_e32 v4, 0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_max_i32_e32 v4, v1, v4
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v0
	s_mul_i32 s4, s4, s3
	s_mul_hi_u32 s4, s3, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s3, s3, s4
	s_mul_hi_u32 s3, s5, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s4, s3, s2
	s_sub_i32 s4, s5, s4
	s_add_i32 s5, s3, 1
	s_sub_i32 s7, s4, s2
	s_cmp_ge_u32 s4, s2
	s_cselect_b32 s3, s5, s3
	s_cselect_b32 s4, s7, s4
	s_add_i32 s5, s3, 1
	s_cmp_ge_u32 s4, s2
	s_cselect_b32 s3, s5, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b32 s3, s3, s6
	s_sub_i32 s4, s3, s6
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s6, s16
	s_abs_i32 s5, s4
	v_cvt_f32_u32_e32 v2, s6
	v_cvt_f32_u32_e32 v0, s5
	s_sub_i32 s3, 0, s5
	s_sub_i32 s7, 0, s6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v2, v2
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v2, v2
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v3, s3, v0
	v_readfirstlane_b32 s3, v2
	s_mul_i32 s7, s7, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s7, s3, s7
	v_mul_hi_u32 v3, v0, v3
	s_add_i32 s3, s3, s7
	s_mul_hi_u32 s3, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s7, s3, s6
	s_sub_i32 s2, s2, s7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v3
	s_add_i32 s7, s3, 1
	s_sub_i32 s8, s2, s6
	s_cmp_ge_u32 s2, s6
	v_mul_hi_u32 v0, v4, v0
	s_cselect_b32 s7, s7, s3
	s_cselect_b32 s2, s8, s2
	s_add_i32 s8, s7, 1
	s_cmp_ge_u32 s2, s6
	s_cselect_b32 s20, s8, s7
	s_cmp_gt_i32 s14, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s5
	s_cselect_b32 s23, -1, 0
	s_cmp_lt_i32 s14, 1
	v_sub_nc_u32_e32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v3, s5, v2
	v_cmp_le_u32_e64 s3, s5, v2
	v_cndmask_b32_e64 v2, v2, v3, s3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_u32_e64 s2, s5, v2
	v_ashrrev_i32_e32 v2, 31, v1
	s_cbranch_scc1 .LBB6_7
; %bb.2:
	v_mad_u64_u32 v[3:4], null, v1, s14, 0
	s_cmp_lg_u64 s[18:19], 0
	s_mov_b32 s6, 0
	s_cselect_b32 s5, -1, 0
	s_mov_b32 s7, s14
	v_mad_u64_u32 v[5:6], null, v2, s14, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v4, v5
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s18, v3
	v_add_co_ci_u32_e64 v4, null, s19, v4, vcc_lo
	s_branch .LBB6_5
.LBB6_3:                                ;   in Loop: Header=BB6_5 Depth=1
	global_load_b32 v5, v[3:4], off
.LBB6_4:                                ;   in Loop: Header=BB6_5 Depth=1
	v_add_co_u32 v3, vcc_lo, v3, 4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	s_add_i32 s7, s7, -1
	s_waitcnt vmcnt(0)
	scratch_store_b32 off, v5, s6
	s_add_i32 s6, s6, 4
	s_cmp_eq_u32 s7, 0
	s_cbranch_scc1 .LBB6_7
.LBB6_5:                                ; =>This Inner Loop Header: Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s5
	s_cbranch_vccz .LBB6_3
; %bb.6:                                ;   in Loop: Header=BB6_5 Depth=1
	v_mov_b32_e32 v5, 0
	s_branch .LBB6_4
.LBB6_7:
	s_ashr_i32 s24, s14, 31
	s_cmp_lt_i32 s12, 1
	s_cbranch_scc1 .LBB6_16
; %bb.8:
	v_add_nc_u32_e32 v3, 1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v0, v3, s3
	v_xor_b32_e32 v3, s4, v1
	s_load_b256 s[4:11], s[0:1], 0x0
	s_ashr_i32 s3, s16, 31
	v_add_nc_u32_e32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e64 v0, v0, v4, s2
	s_xor_b32 s2, s22, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v3, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[5:6], 2, v[3:4]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s8, v5
	v_add_co_ci_u32_e64 v8, null, s9, v6, vcc_lo
	v_add_co_u32 v9, vcc_lo, s10, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s11, v6, vcc_lo
	global_load_b32 v0, v[7:8], off
	global_load_b32 v12, v[9:10], off
	s_xor_b32 s8, s20, s2
	v_sub_nc_u32_e32 v9, 0, v3
	s_sub_i32 s2, s8, s2
	s_mov_b32 s20, s13
	s_abs_i32 s8, s2
	s_ashr_i32 s2, s2, 31
	v_cvt_f32_u32_e32 v7, s8
	s_sub_i32 s9, 0, s8
	v_max_i32_e32 v3, v3, v9
	v_xor_b32_e32 v4, s2, v4
	s_lshl_b64 s[10:11], s[20:21], 2
	v_rcp_iflag_f32_e32 v7, v7
	s_mov_b32 s2, s16
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v7, 0x4f7ffffe, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v7, v7
	v_mul_lo_u32 v8, s9, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v8, v7, v8
	v_add_nc_u32_e32 v9, v7, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[7:8], null, v3, v9, 0
	v_mul_lo_u32 v7, v8, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v7
	v_add_nc_u32_e32 v7, 1, v8
	v_subrev_nc_u32_e32 v9, s8, v3
	v_cmp_le_u32_e32 vcc_lo, s8, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v7, v8, v7, vcc_lo
	v_dual_cndmask_b32 v3, v3, v9 :: v_dual_add_nc_u32 v8, 1, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_u32_e32 vcc_lo, s8, v3
	s_load_b64 s[8:9], s[0:1], 0x20
	s_ashr_i32 s1, s17, 31
	s_add_u32 s10, s4, s10
	s_addc_u32 s11, s5, s11
	v_cndmask_b32_e32 v3, v7, v8, vcc_lo
	v_add_co_u32 v13, vcc_lo, s6, v5
	s_lshl_b64 s[2:3], s[2:3], 2
	v_add_co_ci_u32_e64 v14, null, s7, v6, vcc_lo
	v_xor_b32_e32 v3, v3, v4
	s_mov_b32 s0, s17
	s_mov_b32 s6, s15
	s_mov_b32 s7, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v3, v3, v4
	v_ashrrev_i32_e32 v4, 31, v3
	v_mad_i64_i32 v[6:7], null, s14, v3, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[4:5], 2, v[3:4]
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	v_lshlrev_b64 v[6:7], 2, v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, s2, v4
	v_add_co_ci_u32_e64 v4, null, s3, v5, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_u32 v15, vcc_lo, s8, v2
	v_mul_lo_u32 v9, v8, s24
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v10, v4, s14
	v_mad_u64_u32 v[4:5], null, v8, s14, s[10:11]
	v_add_co_ci_u32_e64 v16, null, s9, v3, vcc_lo
	v_add_co_u32 v6, vcc_lo, s10, v6
	v_add_co_ci_u32_e64 v7, null, s11, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_4)
	v_add3_u32 v5, v10, v5, v9
	s_lshl_b64 s[2:3], s[0:1], 2
	s_mov_b32 s8, 0x3e9b6dac
	s_branch .LBB6_11
.LBB6_9:                                ;   in Loop: Header=BB6_11 Depth=1
	v_mov_b32_e32 v18, 0
.LBB6_10:                               ;   in Loop: Header=BB6_11 Depth=1
	s_mul_i32 s9, s7, s21
	s_mul_hi_u32 s10, s7, s20
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v18, v12, v17
	s_add_i32 s11, s10, s9
	s_mul_i32 s10, s7, s20
	s_add_i32 s7, s7, 1
	s_lshl_b64 s[10:11], s[10:11], 2
	s_cmp_eq_u32 s7, s12
	v_add_co_u32 v8, vcc_lo, v15, s10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s11, v16, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, s2
	v_add_co_ci_u32_e64 v7, null, s3, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v4, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s3, v5, vcc_lo
	global_store_b32 v[8:9], v18, off
	s_cbranch_scc1 .LBB6_16
.LBB6_11:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB6_15 Depth 2
	s_mul_i32 s9, s7, s22
	s_mul_hi_u32 s11, s7, s6
	s_mul_i32 s10, s7, s6
	s_add_i32 s11, s11, s9
	s_mov_b32 s9, exec_lo
	s_lshl_b64 s[10:11], s[10:11], 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, v13, s10
	v_add_co_ci_u32_e64 v9, null, s11, v14, vcc_lo
	global_load_b32 v8, v[8:9], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f32_e32 0x41a00000, v8
	s_cbranch_execz .LBB6_13
; %bb.12:                               ;   in Loop: Header=BB6_11 Depth=1
	v_mul_f32_e32 v9, 0x3fb8aa3b, v8
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rndne_f32_e32 v10, v9
	v_fma_f32 v11, 0x3fb8aa3b, v8, -v9
	v_sub_f32_e32 v9, v9, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v11, 0x32a5705f, v8
	v_cvt_i32_f32_e32 v10, v10
	v_add_f32_e32 v9, v9, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v9, v9
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v9, v9, v10
	v_cndmask_b32_e32 v9, 0, v9, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v10, 0x7f800000, v9, vcc_lo
	v_add_f32_e32 v11, 1.0, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[8:9], v11
	v_frexp_exp_i32_f64_e32 v8, v[8:9]
	v_frexp_mant_f32_e32 v9, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v9
	v_add_f32_e32 v9, -1.0, v11
	v_dual_sub_f32 v18, v9, v11 :: v_dual_sub_f32 v9, v10, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v18, 1.0, v18
	v_add_f32_e32 v9, v9, v18
	v_subrev_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_cmp_neq_f32_e32 vcc_lo, 0x7f800000, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v17, 0, v8
	v_cvt_f32_i32_e32 v8, v8
	v_ldexp_f32 v11, v11, v17
	v_ldexp_f32 v9, v9, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v19, 1.0, v11
	v_add_f32_e32 v17, -1.0, v11
	v_add_f32_e32 v18, -1.0, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v20, 1.0, v17
	v_sub_f32_e32 v18, v11, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v11, v11, v20 :: v_dual_add_f32 v18, v9, v18
	v_dual_add_f32 v9, v9, v11 :: v_dual_add_f32 v20, v19, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v21, v17, v9
	v_rcp_f32_e32 v11, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v17, v17, v21
	v_sub_f32_e32 v19, v19, v20
	v_dual_add_f32 v9, v9, v17 :: v_dual_add_f32 v18, v18, v19
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v22, v21, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v23, v20, v22
	v_fma_f32 v19, v22, v20, -v23
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v19, v22, v18
	v_add_f32_e32 v24, v23, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v25, v21, v24
	v_sub_f32_e32 v17, v24, v23
	v_sub_f32_e32 v21, v21, v25
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v17, v17, v19
	v_sub_f32_e32 v21, v21, v24
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v9, v21
	v_add_f32_e32 v9, v17, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v17, v25, v9
	v_mul_f32_e32 v19, v11, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v24, v25, v17 :: v_dual_mul_f32 v21, v20, v19
	v_add_f32_e32 v9, v9, v24
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v20, v19, v20, -v21
	v_fmac_f32_e32 v20, v19, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v18, v21, v20
	v_sub_f32_e32 v23, v17, v18
	v_sub_f32_e32 v21, v18, v21
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v17, v17, v23
	v_sub_f32_e32 v17, v17, v18
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v18, v21, v20
	v_add_f32_e32 v9, v9, v17
	v_add_f32_e32 v17, v22, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v9, v18, v9 :: v_dual_sub_f32 v18, v17, v22
	v_add_f32_e32 v9, v23, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v18, v19, v18
	v_mul_f32_e32 v9, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v18, v9
	v_add_f32_e32 v11, v17, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v18, v11, v11
	v_fmaak_f32 v19, s8, v18, 0x3ecc95a3
	v_mul_f32_e32 v20, v11, v18
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmaak_f32 v18, v18, v19, 0x3f2aaada
	v_ldexp_f32 v19, v11, 1
	v_dual_sub_f32 v11, v11, v17 :: v_dual_mul_f32 v18, v20, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_mul_f32 v20, 0x3f317218, v8 :: v_dual_sub_f32 v9, v9, v11
	v_add_f32_e32 v17, v19, v18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f32 v9, v9, 1
	v_sub_f32_e32 v11, v17, v19
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v19, 0x3f317218, v8, -v20
	v_sub_f32_e32 v11, v18, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v19, 0xb102e308, v8 :: v_dual_add_f32 v8, v9, v11
	v_add_f32_e32 v9, v20, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v11, v17, v8
	v_sub_f32_e32 v20, v9, v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_add_f32 v18, v9, v11 :: v_dual_sub_f32 v17, v11, v17
	v_sub_f32_e32 v19, v19, v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v21, v18, v9
	v_sub_f32_e32 v8, v8, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v22, v18, v21
	v_sub_f32_e32 v11, v11, v21
	v_add_f32_e32 v17, v19, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v9, v22
	v_add_f32_e32 v9, v11, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v11, v17, v19
	v_add_f32_e32 v9, v17, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v17, v17, v11
	v_sub_f32_e32 v8, v8, v11
	v_add_f32_e32 v20, v18, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v17, v19, v17
	v_sub_f32_e32 v11, v20, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v8, v8, v17 :: v_dual_sub_f32 v9, v9, v11
	v_add_f32_e32 v8, v8, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v8, v20, v8
	v_cndmask_b32_e32 v8, 0x7f800000, v8, vcc_lo
	v_cmp_gt_f32_e32 vcc_lo, 0x33800000, v10
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v8, v8, v10, vcc_lo
.LBB6_13:                               ;   in Loop: Header=BB6_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s9
	s_mul_i32 s9, s7, s1
	s_mul_hi_u32 s11, s7, s0
	s_mul_i32 s10, s7, s0
	s_add_i32 s11, s11, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[10:11], s[10:11], 2
	s_add_u32 s9, s4, s10
	s_addc_u32 s10, s5, s11
	v_add_co_u32 v9, vcc_lo, s9, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s10, v3, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s23
	global_load_b32 v17, v[9:10], off
	s_cbranch_vccnz .LBB6_9
; %bb.14:                               ;   in Loop: Header=BB6_11 Depth=1
	v_mul_f32_e32 v10, v0, v8
	s_mov_b32 s9, 0
	s_mov_b32 s10, s14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v9, 0x3fb8aa3b, v10
	v_fma_f32 v11, 0x3fb8aa3b, v10, -v9
	v_rndne_f32_e32 v18, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v9, v9, v18
	v_fmac_f32_e32 v11, 0x32a5705f, v10
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f32_e32 v9, v9, v11
	v_cvt_i32_f32_e32 v11, v18
	v_mov_b32_e32 v18, 0
	v_exp_f32_e32 v9, v9
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v9, v9, v11
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v11, 0, v9, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v10
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v19, v8, v17
	v_dual_mov_b32 v9, v5 :: v_dual_mov_b32 v8, v4
	v_dual_cndmask_b32 v20, 0x7f800000, v11 :: v_dual_mov_b32 v11, v7
	v_mov_b32_e32 v10, v6
	.p2align	6
.LBB6_15:                               ;   Parent Loop BB6_11 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b32 v21, v[10:11], off
	scratch_load_b32 v22, off, s9
	global_load_b32 v23, v[8:9], off
	v_add_co_u32 v10, vcc_lo, v10, 4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	v_add_co_u32 v8, vcc_lo, v8, 4
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	s_add_i32 s10, s10, -1
	s_waitcnt vmcnt(2)
	v_mul_f32_e32 v21, v19, v21
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v21, v20, v22
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v18, v21, v23
	scratch_store_b32 off, v21, s9
	s_add_i32 s9, s9, 4
	s_cmp_eq_u32 s10, 0
	s_cbranch_scc0 .LBB6_15
	s_branch .LBB6_10
.LBB6_16:
	s_cmp_eq_u64 s[18:19], 0
	s_cselect_b32 s0, -1, 0
	s_xor_b32 s1, s23, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_or_b32 s0, s0, s1
	s_and_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccnz .LBB6_19
; %bb.17:
	v_mad_i64_i32 v[2:3], null, v1, s14, 0
	s_mov_b32 s0, 0
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s18, v0
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
.LBB6_18:                               ; =>This Inner Loop Header: Depth=1
	scratch_load_b32 v2, off, s0
	s_add_i32 s14, s14, -1
	s_add_i32 s0, s0, 4
	s_cmp_lg_u32 s14, 0
	s_waitcnt vmcnt(0)
	global_store_b32 v[0:1], v2, off
	v_add_co_u32 v0, vcc_lo, v0, 4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_cbranch_scc1 .LBB6_18
.LBB6_19:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 1040
		.amdhsa_kernarg_size 328
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 1
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 26
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
		.amdhsa_inst_pref_size 20
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_,"axG",@progbits,_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_,comdat
.Lfunc_end6:
	.size	_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_, .Lfunc_end6-_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_
                                        ; -- End function
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.num_vgpr, 26
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.num_agpr, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.numbered_sgpr, 25
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.num_named_barrier, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.private_seg_size, 1040
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.uses_vcc, 1
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.uses_flat_scratch, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.has_dyn_sized_stack, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.has_recursion, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2552
; TotalNumSgprs: 27
; NumVgprs: 26
; ScratchSize: 1040
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 27
; NumVGPRsForWavesPerEU: 26
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_,"axG",@progbits,_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_,comdat
	.protected	_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_ ; -- Begin function _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_
	.globl	_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_
	.p2align	8
	.type	_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_,@function
_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_: ; @_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x54
	s_load_b128 s[12:15], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s13, v1
	s_cbranch_execz .LBB7_19
; %bb.1:
	s_abs_i32 s2, s15
	s_abs_i32 s5, s13
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s4, 0, s2
	s_ashr_i32 s21, s13, 31
	s_ashr_i32 s33, s15, 31
	s_load_b128 s[16:19], s[0:1], 0x38
	v_rcp_iflag_f32_e32 v0, v0
	s_xor_b32 s6, s21, s33
	v_sub_nc_u32_e32 v4, 0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_max_i32_e32 v4, v1, v4
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v0
	s_mul_i32 s4, s4, s3
	s_mul_hi_u32 s4, s3, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s3, s3, s4
	s_mul_hi_u32 s3, s5, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s4, s3, s2
	s_sub_i32 s4, s5, s4
	s_add_i32 s5, s3, 1
	s_sub_i32 s7, s4, s2
	s_cmp_ge_u32 s4, s2
	s_cselect_b32 s3, s5, s3
	s_cselect_b32 s4, s7, s4
	s_add_i32 s5, s3, 1
	s_cmp_ge_u32 s4, s2
	s_cselect_b32 s3, s5, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b32 s3, s3, s6
	s_sub_i32 s4, s3, s6
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s6, s16
	s_abs_i32 s5, s4
	v_cvt_f32_u32_e32 v2, s6
	v_cvt_f32_u32_e32 v0, s5
	s_sub_i32 s3, 0, s5
	s_sub_i32 s7, 0, s6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v2, v2
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v2, v2
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v3, s3, v0
	v_readfirstlane_b32 s3, v2
	s_mul_i32 s7, s7, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s7, s3, s7
	v_mul_hi_u32 v3, v0, v3
	s_add_i32 s3, s3, s7
	s_mul_hi_u32 s3, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s7, s3, s6
	s_sub_i32 s2, s2, s7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v3
	s_add_i32 s7, s3, 1
	s_sub_i32 s8, s2, s6
	s_cmp_ge_u32 s2, s6
	v_mul_hi_u32 v0, v4, v0
	s_cselect_b32 s7, s7, s3
	s_cselect_b32 s2, s8, s2
	s_add_i32 s8, s7, 1
	s_cmp_ge_u32 s2, s6
	s_cselect_b32 s20, s8, s7
	s_cmp_gt_i32 s14, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s5
	s_cselect_b32 s64, -1, 0
	s_cmp_lt_i32 s14, 1
	v_sub_nc_u32_e32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v3, s5, v2
	v_cmp_le_u32_e64 s3, s5, v2
	v_cndmask_b32_e64 v2, v2, v3, s3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_u32_e64 s2, s5, v2
	v_ashrrev_i32_e32 v2, 31, v1
	s_cbranch_scc1 .LBB7_7
; %bb.2:
	v_mad_u64_u32 v[3:4], null, v1, s14, 0
	s_cmp_lg_u64 s[18:19], 0
	s_mov_b32 s6, 0
	s_cselect_b32 s5, -1, 0
	s_mov_b32 s7, s14
	v_mad_u64_u32 v[5:6], null, v2, s14, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v4, v5
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s18, v3
	v_add_co_ci_u32_e64 v4, null, s19, v4, vcc_lo
	s_branch .LBB7_5
	.p2align	6
.LBB7_3:                                ;   in Loop: Header=BB7_5 Depth=1
	global_load_b64 v[5:6], v[3:4], off
.LBB7_4:                                ;   in Loop: Header=BB7_5 Depth=1
	v_add_co_u32 v3, vcc_lo, v3, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	s_add_i32 s7, s7, -1
	s_waitcnt vmcnt(0)
	scratch_store_b64 off, v[5:6], s6
	s_add_i32 s6, s6, 8
	s_cmp_eq_u32 s7, 0
	s_cbranch_scc1 .LBB7_7
.LBB7_5:                                ; =>This Inner Loop Header: Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s5
	s_cbranch_vccz .LBB7_3
; %bb.6:                                ;   in Loop: Header=BB7_5 Depth=1
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_branch .LBB7_4
.LBB7_7:
	s_ashr_i32 s22, s14, 31
	s_cmp_lt_i32 s12, 1
	s_cbranch_scc1 .LBB7_16
; %bb.8:
	v_add_nc_u32_e32 v3, 1, v0
	s_mov_b32 s24, 0x6a5dcb37
	s_mov_b32 s26, 0x623fde64
	s_mov_b32 s28, 0x7c89e6b0
	s_mov_b32 s30, 0x14761f6e
	v_cndmask_b32_e64 v0, v0, v3, s3
	v_xor_b32_e32 v3, s4, v1
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x20
	s_mov_b32 s34, 0x1852b7b0
	v_add_nc_u32_e32 v4, 1, v0
	v_ashrrev_i32_e32 v3, 31, v3
	s_mov_b32 s36, 0x11122322
	s_mov_b32 s38, 0x555502a1
	s_mov_b32 s40, 0x55555511
	v_cndmask_b32_e64 v0, v0, v4, s2
	s_mov_b32 s42, 11
	s_mov_b32 s44, 0x55555555
	s_mov_b32 s46, 0x6b47b09a
	s_mov_b32 s48, 0xbf559e2b
	v_xor_b32_e32 v0, v0, v3
	s_mov_b32 s50, 0xd7f4df2e
	s_mov_b32 s52, 0x16291751
	s_mov_b32 s54, 0x9b27acf1
	s_mov_b32 s56, 0x998ef7b6
	v_sub_nc_u32_e32 v7, v0, v3
	s_mov_b32 s25, 0x3e5ade15
	s_mov_b32 s27, 0x3ec71dee
	s_mov_b32 s29, 0x3efa0199
	s_mov_b32 s31, 0x3f2a01a0
	v_ashrrev_i32_e32 v8, 31, v7
	v_sub_nc_u32_e32 v12, 0, v7
	s_mov_b32 s35, 0x3f56c16c
	s_mov_b32 s37, 0x3f811111
	s_mov_b32 s39, 0x3fa55555
	v_lshlrev_b64 v[9:10], 3, v[7:8]
	v_max_i32_e32 v7, v7, v12
	s_mov_b32 s41, 0x3fc55555
	s_mov_b32 s43, 0x3fe00000
	s_mov_b32 s45, 0x3fe55555
	s_mov_b32 s47, 0x3fc38538
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s8, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s9, v10, vcc_lo
	v_add_co_u32 v5, vcc_lo, s10, v9
	v_add_co_ci_u32_e64 v6, null, s11, v10, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	global_load_b64 v[5:6], v[5:6], off
	s_ashr_i32 s9, s16, 31
	s_mov_b32 s10, 0xfefa39ef
	s_xor_b32 s2, s33, s9
	s_mov_b32 s11, 0xbfe62e42
	s_xor_b32 s3, s20, s2
	s_mov_b32 s20, s13
	s_sub_i32 s2, s3, s2
	s_mov_b32 s13, s15
	s_abs_i32 s3, s2
	s_ashr_i32 s2, s2, 31
	v_cvt_f32_u32_e32 v0, s3
	s_sub_i32 s8, 0, s3
	v_xor_b32_e32 v8, s2, v8
	s_mov_b32 s2, s17
	s_mov_b32 s15, 0
	v_rcp_iflag_f32_e32 v0, v0
	s_mov_b32 s49, 0x3fc3ab76
	s_mov_b32 s51, 0x3fc7474d
	s_mov_b32 s53, 0x3fcc71c0
	s_mov_b32 s55, 0x3fd24924
	s_mov_b32 s57, 0x3fd99999
	s_mov_b32 s58, 0x55555780
	s_mov_b32 s61, 0x3fe62e42
	s_mov_b32 s63, 0x3c7abc9e
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v11, s8, v0
	s_mov_b32 s8, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v11, v0, v11
	v_add_nc_u32_e32 v0, v0, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[11:12], null, v7, v0, 0
	v_mul_lo_u32 v0, v12, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v7, v0
	v_add_nc_u32_e32 v7, 1, v12
	v_subrev_nc_u32_e32 v11, s3, v0
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v7, v12, v7, vcc_lo
	v_cndmask_b32_e32 v0, v0, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v11, 1, v7
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	s_ashr_i32 s3, s17, 31
	s_mov_b32 s16, 0x3b39803f
	s_mov_b32 s17, 0xbc7abc9e
	v_cndmask_b32_e32 v0, v7, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v8
	v_sub_nc_u32_e32 v7, v0, v8
	v_add_co_u32 v0, vcc_lo, s6, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v25, null, s7, v10, vcc_lo
	v_ashrrev_i32_e32 v8, 31, v7
	s_lshl_b64 s[6:7], s[20:21], 3
	v_mad_i64_i32 v[10:11], null, s14, v7, 0
	s_add_u32 s6, s4, s6
	v_lshlrev_b64 v[8:9], 3, v[7:8]
	s_addc_u32 s7, s5, s7
	s_lshl_b64 s[8:9], s[8:9], 3
	v_lshlrev_b64 v[11:12], 3, v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v13, vcc_lo, s8, v8
	v_add_co_ci_u32_e64 v9, null, s9, v9, vcc_lo
	v_lshlrev_b64 v[7:8], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v14, v13, s22
	s_mov_b32 s8, 0x652b82fe
	v_mul_lo_u32 v15, v9, s14
	v_mad_u64_u32 v[9:10], null, v13, s14, s[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v7
	v_add_co_ci_u32_e64 v26, null, s1, v8, vcc_lo
	v_add_co_u32 v11, vcc_lo, s6, v11
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v12, null, s7, v12, vcc_lo
	v_add3_u32 v10, v15, v10, v14
	s_mov_b32 s22, 0xfca7ab0c
	s_lshl_b64 s[6:7], s[2:3], 3
	s_mov_b32 s9, 0x3ff71547
	s_mov_b32 s23, 0x3e928af3
	s_branch .LBB7_11
.LBB7_9:                                ;   in Loop: Header=BB7_11 Depth=1
	v_mov_b32_e32 v13, 0
	v_mov_b32_e32 v14, 0
.LBB7_10:                               ;   in Loop: Header=BB7_11 Depth=1
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[5:6], v[15:16], v[13:14]
	s_mul_i32 s0, s15, s21
	s_mul_hi_u32 s1, s15, s20
	s_add_i32 s1, s1, s0
	s_mul_i32 s0, s15, s20
	s_add_i32 s15, s15, 1
	s_lshl_b64 s[0:1], s[0:1], 3
	s_cmp_eq_u32 s15, s12
	v_add_co_u32 v15, vcc_lo, v2, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v16, null, s1, v26, vcc_lo
	v_add_co_u32 v11, vcc_lo, v11, s6
	v_add_co_ci_u32_e64 v12, null, s7, v12, vcc_lo
	v_add_co_u32 v9, vcc_lo, v9, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s7, v10, vcc_lo
	global_store_b64 v[15:16], v[13:14], off
	s_cbranch_scc1 .LBB7_16
.LBB7_11:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB7_15 Depth 2
	s_mul_i32 s1, s15, s33
	s_mul_hi_u32 s59, s15, s13
	s_mul_i32 s0, s15, s13
	s_add_i32 s1, s59, s1
	s_mov_b32 s65, exec_lo
	s_lshl_b64 s[0:1], s[0:1], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v13, vcc_lo, v0, s0
	v_add_co_ci_u32_e64 v14, null, s1, v25, vcc_lo
	global_load_b64 v[13:14], v[13:14], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f64_e32 0x40340000, v[13:14]
	s_cbranch_execz .LBB7_13
; %bb.12:                               ;   in Loop: Header=BB7_11 Depth=1
	v_mul_f64 v[15:16], v[13:14], s[8:9]
	v_cmp_ngt_f64_e32 vcc_lo, 0xc090cc00, v[13:14]
	s_mov_b32 s59, s45
	s_mov_b32 s60, s10
	s_mov_b32 s62, s16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[15:16], v[15:16]
	v_fma_f64 v[17:18], v[15:16], s[10:11], v[13:14]
	v_cvt_i32_f64_e32 v21, v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], s[16:17], v[17:18]
	v_fma_f64 v[19:20], v[17:18], s[24:25], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[26:27]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[30:31]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[36:37]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[40:41]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[15:16], v[15:16], v21
	v_dual_cndmask_b32 v16, 0, v16 :: v_dual_cndmask_b32 v15, 0, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[15:16], 1.0
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[15:16]
	v_cmp_ngt_f64_e64 s1, -1.0, v[15:16]
	v_frexp_mant_f64_e32 v[17:18], v[13:14]
	v_frexp_exp_i32_f64_e32 v21, v[13:14]
	v_add_f64 v[19:20], v[13:14], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, s[44:45], v[17:18]
	v_add_f64 v[17:18], v[19:20], -v[13:14]
	v_add_f64 v[19:20], v[15:16], -v[19:20]
	v_subrev_co_ci_u32_e64 v39, null, 0, v21, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[17:18], 1.0
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[15:16]
	v_sub_nc_u32_e32 v23, 0, v39
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[13:14], v[13:14], v23
	v_add_f64 v[17:18], v[19:20], v[17:18]
	s_and_b32 vcc_lo, vcc_lo, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], 1.0
	v_add_f64 v[29:30], v[13:14], -1.0
	v_ldexp_f64 v[17:18], v[17:18], v23
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[21:22], -1.0
	v_add_f64 v[31:32], v[29:30], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[13:14], -v[19:20]
	v_add_f64 v[13:14], v[13:14], -v[31:32]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[17:18], v[19:20]
	v_add_f64 v[13:14], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[23:24], v[21:22], v[19:20]
	v_add_f64 v[31:32], v[29:30], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[27:28], v[23:24]
	v_add_f64 v[21:22], v[23:24], -v[21:22]
	v_add_f64 v[29:30], v[31:32], -v[29:30]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[19:20], -v[21:22]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[33:34], -v[23:24], v[27:28], 1.0
	v_add_f64 v[13:14], v[13:14], -v[29:30]
	v_fma_f64 v[27:28], v[33:34], v[27:28], v[27:28]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], -v[23:24], v[27:28], 1.0
	v_fma_f64 v[17:18], v[17:18], v[27:28], v[27:28]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[27:28], v[31:32], v[17:18]
	v_mul_f64 v[33:34], v[23:24], v[27:28]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[21:22], v[27:28], v[23:24], -v[33:34]
	v_fma_f64 v[21:22], v[27:28], v[19:20], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[35:36], v[33:34], v[21:22]
	v_add_f64 v[37:38], v[31:32], -v[35:36]
	v_add_f64 v[29:30], v[35:36], -v[33:34]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[31:32], v[31:32], -v[37:38]
	v_add_f64 v[21:22], v[29:30], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[31:32], v[31:32], -v[35:36]
	v_add_f64 v[13:14], v[13:14], v[31:32]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[21:22], v[13:14]
	v_add_f64 v[21:22], v[37:38], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[29:30], v[17:18], v[21:22]
	v_add_f64 v[35:36], v[37:38], -v[21:22]
	v_mul_f64 v[31:32], v[23:24], v[29:30]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], v[35:36]
	v_fma_f64 v[23:24], v[29:30], v[23:24], -v[31:32]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[29:30], v[19:20], v[23:24]
	v_add_f64 v[23:24], v[31:32], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[33:34], v[21:22], -v[23:24]
	v_add_f64 v[31:32], v[23:24], -v[31:32]
	v_add_f64 v[21:22], v[21:22], -v[33:34]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[31:32], -v[19:20]
	v_add_f64 v[21:22], v[21:22], -v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], v[21:22]
	v_add_f64 v[21:22], v[27:28], v[29:30]
	v_add_f64 v[13:14], v[19:20], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[21:22], -v[27:28]
	v_add_f64 v[13:14], v[33:34], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[29:30], -v[19:20]
	v_mul_f64 v[13:14], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[19:20], v[13:14]
	v_add_f64 v[17:18], v[21:22], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[17:18], v[17:18]
	v_fma_f64 v[23:24], v[19:20], s[48:49], s[46:47]
	v_mul_f64 v[27:28], v[17:18], v[19:20]
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
	v_mul_f64 v[19:20], v[27:28], v[19:20]
	v_cvt_f64_i32_e32 v[27:28], v39
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	v_add_f64 v[21:22], v[23:24], v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[29:30], v[27:28], s[60:61]
	v_ldexp_f64 v[13:14], v[13:14], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[21:22], -v[23:24]
	v_fma_f64 v[23:24], v[27:28], s[60:61], -v[29:30]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[17:18]
	v_fma_f64 v[19:20], v[27:28], s[62:63], v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], v[17:18]
	v_add_f64 v[17:18], v[29:30], v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[23:24], v[21:22], v[13:14]
	v_add_f64 v[29:30], v[17:18], -v[29:30]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[27:28], v[17:18], v[23:24]
	v_add_f64 v[21:22], v[23:24], -v[21:22]
	v_add_f64 v[19:20], v[19:20], -v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[31:32], v[27:28], -v[17:18]
	v_add_f64 v[13:14], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[33:34], v[27:28], -v[31:32]
	v_add_f64 v[21:22], v[23:24], -v[31:32]
	v_add_f64 v[23:24], v[19:20], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[17:18], -v[33:34]
	v_add_f64 v[17:18], v[21:22], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[23:24], -v[19:20]
	v_add_f64 v[17:18], v[23:24], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[23:24], -v[21:22]
	v_add_f64 v[13:14], v[13:14], -v[21:22]
	v_add_f64 v[29:30], v[27:28], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[19:20], -v[23:24]
	v_add_f64 v[21:22], v[29:30], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], v[19:20]
	v_add_f64 v[17:18], v[17:18], -v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[13:14], v[17:18]
	v_add_f64 v[13:14], v[29:30], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v13, 0, v13, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[15:16]
	v_cndmask_b32_e64 v14, 0x7ff00000, v14, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v14, 0x7ff80000, v14, s1
	v_cndmask_b32_e32 v14, 0xfff00000, v14, vcc_lo
.LBB7_13:                               ;   in Loop: Header=BB7_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s65
	s_mul_i32 s1, s15, s3
	s_mul_hi_u32 s59, s15, s2
	s_mul_i32 s0, s15, s2
	s_add_i32 s1, s59, s1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[0:1], s[0:1], 3
	s_add_u32 s0, s4, s0
	s_addc_u32 s1, s5, s1
	v_add_co_u32 v15, vcc_lo, s0, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v16, null, s1, v8, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s64
	global_load_b64 v[15:16], v[15:16], off
	s_cbranch_vccnz .LBB7_9
; %bb.14:                               ;   in Loop: Header=BB7_11 Depth=1
	v_mul_f64 v[17:18], v[3:4], v[13:14]
	s_mov_b32 s1, s14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[19:20], v[17:18], s[8:9]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[17:18]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[17:18]
	v_rndne_f64_e32 v[19:20], v[19:20]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[21:22], v[19:20], s[10:11], v[17:18]
	v_cvt_i32_f64_e32 v27, v[19:20]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[17:18], v[13:14], v[15:16]
	v_mov_b32_e32 v13, 0
	v_mov_b32_e32 v14, 0
	v_fma_f64 v[21:22], v[19:20], s[16:17], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], s[24:25], s[22:23]
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[28:29]
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[34:35]
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[38:39]
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[42:43]
	v_fma_f64 v[23:24], v[21:22], v[23:24], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[19:20], v[21:22], v[23:24], 1.0
	v_dual_mov_b32 v24, v12 :: v_dual_mov_b32 v23, v11
	v_ldexp_f64 v[21:22], v[19:20], v27
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v22, 0x7ff00000, v22, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_dual_mov_b32 v20, v10 :: v_dual_cndmask_b32 v21, 0, v21
	v_mov_b32_e32 v19, v9
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v22, 0, v22, s0
	s_mov_b32 s0, 0
	.p2align	6
.LBB7_15:                               ;   Parent Loop BB7_11 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[27:28], v[23:24], off
	scratch_load_b64 v[29:30], off, s0
	global_load_b64 v[31:32], v[19:20], off
	v_add_co_u32 v23, vcc_lo, v23, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v24, null, 0, v24, vcc_lo
	v_add_co_u32 v19, vcc_lo, v19, 8
	v_add_co_ci_u32_e64 v20, null, 0, v20, vcc_lo
	s_add_i32 s1, s1, -1
	s_waitcnt vmcnt(2)
	v_mul_f64 v[27:28], v[17:18], v[27:28]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[27:28], v[21:22], v[29:30], v[27:28]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[13:14], v[27:28], v[31:32], v[13:14]
	scratch_store_b64 off, v[27:28], s0
	s_add_i32 s0, s0, 8
	s_cmp_eq_u32 s1, 0
	s_cbranch_scc0 .LBB7_15
	s_branch .LBB7_10
.LBB7_16:
	s_cmp_eq_u64 s[18:19], 0
	s_cselect_b32 s0, -1, 0
	s_xor_b32 s1, s64, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_or_b32 s0, s0, s1
	s_and_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccnz .LBB7_19
; %bb.17:
	v_mad_i64_i32 v[2:3], null, v1, s14, 0
	s_mov_b32 s0, 0
	v_lshlrev_b64 v[0:1], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s18, v0
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
.LBB7_18:                               ; =>This Inner Loop Header: Depth=1
	scratch_load_b64 v[2:3], off, s0
	s_add_i32 s14, s14, -1
	s_add_i32 s0, s0, 8
	s_cmp_lg_u32 s14, 0
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[2:3], off
	v_add_co_u32 v0, vcc_lo, v0, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_cbranch_scc1 .LBB7_18
.LBB7_19:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 2064
		.amdhsa_kernarg_size 328
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 1
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 40
		.amdhsa_next_free_sgpr 66
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
		.amdhsa_inst_pref_size 28
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_,"axG",@progbits,_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_,comdat
.Lfunc_end7:
	.size	_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_, .Lfunc_end7-_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_
                                        ; -- End function
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.num_vgpr, 40
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.num_agpr, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.numbered_sgpr, 66
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.num_named_barrier, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.private_seg_size, 2064
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.uses_vcc, 1
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.uses_flat_scratch, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.has_dyn_sized_stack, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.has_recursion, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3556
; TotalNumSgprs: 68
; NumVgprs: 40
; ScratchSize: 2064
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 68
; NumVGPRsForWavesPerEU: 40
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii,"axG",@progbits,_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii,comdat
	.protected	_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii ; -- Begin function _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii
	.globl	_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii
	.p2align	8
	.type	_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii,@function
_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii: ; @_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii
; %bb.0:
	s_load_b128 s[12:15], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s12
	s_cbranch_scc1 .LBB8_15
; %bb.1:
	s_abs_i32 s12, s14
	s_load_b256 s[4:11], s[0:1], 0x0
	v_cvt_f32_u32_e32 v1, s12
	s_sub_i32 s15, 0, s12
	v_cmp_gt_i32_e32 vcc_lo, s13, v0
	v_mov_b32_e32 v3, 0
	s_abs_i32 s14, s2
	v_rcp_iflag_f32_e32 v1, v1
	s_mov_b32 s17, 0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_waitcnt lgkmcnt(0)
	s_load_b32 s16, s[8:9], 0x0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cvt_u32_f32_e32 v1, v1
	s_mul_hi_i32 s9, s13, s2
	s_mul_i32 s8, s13, s2
	s_lshl_b64 s[8:9], s[8:9], 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v1
	s_mul_i32 s15, s15, s3
	s_mul_hi_u32 s15, s3, s15
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s3, s3, s15
	s_add_u32 s4, s4, s8
	s_mul_hi_u32 s15, s14, s3
	s_addc_u32 s5, s5, s9
	s_and_saveexec_b32 s18, vcc_lo
	s_cbranch_execz .LBB8_5
; %bb.2:
	s_load_b32 s3, s[0:1], 0x3c
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s19, s3, 0xffff
.LBB8_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	v_add_nc_u32_e32 v1, s19, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, s3, s4, v4
	v_add_co_ci_u32_e64 v5, null, s5, v5, s3
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s3, s13, v1
	global_load_b32 v2, v[4:5], off
	s_or_b32 s17, s3, s17
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v3, v2, v2
	s_and_not1_b32 exec_lo, exec_lo, s17
	s_cbranch_execnz .LBB8_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s17
.LBB8_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s18
	v_mbcnt_lo_u32_b32 v5, -1, 0
	s_ashr_i32 s17, s2, 31
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
	s_cbranch_execz .LBB8_7
; %bb.6:
	v_lshrrev_b32_e32 v9, 3, v0
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v7, v7, v8
	ds_store_b32 v9, v7
.LBB8_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s18, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB8_12
; %bb.8:
	s_load_b32 s3, s[0:1], 0x3c
	v_mov_b32_e32 v7, 0
	s_mov_b32 s19, exec_lo
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
	s_or_b32 exec_lo, exec_lo, s19
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
	s_cbranch_execz .LBB8_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB8_12:
	s_or_b32 exec_lo, exec_lo, s18
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v1, v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB8_15
; %bb.13:
	v_cvt_f32_i32_e32 v2, s13
	s_mul_i32 s15, s15, s12
	s_load_b32 s0, s[0:1], 0x3c
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
	v_add_f32_e32 v1, s16, v1
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v2, v2, v4, s2
	s_sub_i32 s2, s14, s15
	s_sub_i32 s3, s2, s12
	s_cmp_ge_u32 s2, s12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mul_f32_e32 v3, 0x37800000, v2
	s_cselect_b32 s2, s3, s2
	s_sub_i32 s1, s2, s12
	s_cmp_ge_u32 s2, s12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e32 v2, v2, v3, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x260
	s_cselect_b32 s1, s1, s2
	s_xor_b32 s2, s1, s17
	s_mov_b32 s1, 0
	v_cndmask_b32_e32 v1, v2, v1, vcc_lo
	s_sub_i32 s3, s2, s17
	s_add_u32 s2, s10, s8
	s_mul_hi_i32 s15, s3, s13
	s_mul_i32 s14, s3, s13
	v_div_scale_f32 v2, null, v1, v1, 1.0
	v_div_scale_f32 v5, vcc_lo, 1.0, v1, 1.0
	s_addc_u32 s3, s11, s9
	v_rcp_f32_e32 v3, v2
	s_lshl_b64 s[8:9], s[14:15], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s6, s6, s8
	s_addc_u32 s7, s7, s9
	s_waitcnt lgkmcnt(0)
	s_and_b32 s8, s0, 0xffff
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
	v_div_fixup_f32 v2, v2, v1, 1.0
	.p2align	6
.LBB8_14:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[0:1]
	v_add_co_u32 v5, vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s5, v4, vcc_lo
	v_add_co_u32 v7, vcc_lo, s6, v3
	v_add_co_ci_u32_e64 v8, null, s7, v4, vcc_lo
	global_load_b32 v1, v[5:6], off
	global_load_b32 v5, v[7:8], off
	v_add_nc_u32_e32 v0, s8, v0
	v_add_co_u32 v3, s0, s2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v4, null, s3, v4, s0
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v1, v2, v1
	v_cmp_le_i32_e32 vcc_lo, s13, v0
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v1, v1, v5
	s_or_b32 s1, vcc_lo, s1
	global_store_b32 v[3:4], v1, off
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execnz .LBB8_14
.LBB8_15:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii
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
		.amdhsa_next_free_vgpr 10
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
		.amdhsa_inst_pref_size 11
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii,"axG",@progbits,_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii,comdat
.Lfunc_end8:
	.size	_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii, .Lfunc_end8-_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii
                                        ; -- End function
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.num_vgpr, 10
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.num_agpr, 0
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.numbered_sgpr, 20
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.num_named_barrier, 0
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.private_seg_size, 0
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.uses_vcc, 1
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.has_recursion, 0
	.set _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1336
; TotalNumSgprs: 22
; NumVgprs: 10
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 128 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 22
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
	.section	.text._Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii,"axG",@progbits,_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii,comdat
	.protected	_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii ; -- Begin function _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii
	.globl	_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii
	.p2align	8
	.type	_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii,@function
_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii: ; @_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii
; %bb.0:
	s_load_b128 s[12:15], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s12
	s_cbranch_scc1 .LBB9_15
; %bb.1:
	s_abs_i32 s12, s14
	s_load_b256 s[4:11], s[0:1], 0x0
	v_cvt_f32_u32_e32 v1, s12
	s_sub_i32 s17, 0, s12
	v_cmp_gt_i32_e32 vcc_lo, s13, v0
	s_abs_i32 s16, s2
	s_mov_b32 s18, 0
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[14:15], s[8:9], 0x0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cvt_u32_f32_e32 v1, v1
	s_mul_hi_i32 s9, s13, s2
	s_mul_i32 s8, s13, s2
	s_lshl_b64 s[8:9], s[8:9], 3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v1
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mul_i32 s17, s17, s3
	s_mul_hi_u32 s17, s3, s17
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s3, s3, s17
	s_add_u32 s4, s4, s8
	s_mul_hi_u32 s17, s16, s3
	s_addc_u32 s5, s5, s9
	s_and_saveexec_b32 s19, vcc_lo
	s_cbranch_execz .LBB9_5
; %bb.2:
	s_load_b32 s3, s[0:1], 0x3c
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s20, s3, 0xffff
	.p2align	6
.LBB9_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	v_add_nc_u32_e32 v3, s20, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, s3, s4, v4
	v_add_co_ci_u32_e64 v5, null, s5, v5, s3
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s3, s13, v3
	global_load_b64 v[4:5], v[4:5], off
	s_or_b32 s18, s3, s18
	s_waitcnt vmcnt(0)
	v_fma_f64 v[1:2], v[4:5], v[4:5], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s18
	s_cbranch_execnz .LBB9_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s18
.LBB9_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s19
	v_mbcnt_lo_u32_b32 v9, -1, 0
	s_ashr_i32 s18, s2, 31
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
	s_cbranch_execz .LBB9_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB9_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s19, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB9_12
; %bb.8:
	s_load_b32 s3, s[0:1], 0x3c
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s20, exec_lo
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
	s_or_b32 exec_lo, exec_lo, s20
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
	s_cbranch_execz .LBB9_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_mov_b32_e32 v3, 0
	ds_store_b64 v3, v[1:2]
.LBB9_12:
	s_or_b32 exec_lo, exec_lo, s19
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB9_15
; %bb.13:
	v_cvt_f64_i32_e32 v[3:4], s13
	s_mul_i32 s17, s17, s12
	s_load_b32 s0, s[0:1], 0x3c
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
	s_sub_i32 s2, s16, s17
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s3, s2, s12
	s_cmp_ge_u32 s2, s12
	s_cselect_b32 s1, s3, s2
	s_mov_b32 s3, 0
	s_sub_i32 s2, s1, s12
	s_cmp_ge_u32 s1, s12
	s_cselect_b32 s1, s2, s1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b32 s1, s1, s18
	s_sub_i32 s2, s1, s18
	s_add_u32 s1, s10, s8
	s_mul_hi_i32 s15, s2, s13
	s_mul_i32 s14, s2, s13
	s_addc_u32 s2, s11, s9
	s_lshl_b64 s[8:9], s[14:15], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_add_u32 s6, s6, s8
	s_addc_u32 s7, s7, s9
	s_waitcnt lgkmcnt(0)
	s_and_b32 s8, s0, 0xffff
	v_dual_cndmask_b32 v2, v4, v2 :: v_dual_cndmask_b32 v1, v3, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[3:4], null, v[1:2], v[1:2], 1.0
	v_rcp_f64_e32 v[5:6], v[3:4]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_div_scale_f64 v[7:8], vcc_lo, 1.0, v[1:2], 1.0
	v_mul_f64 v[9:10], v[7:8], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], -v[3:4], v[9:10], v[7:8]
	v_div_fmas_f64 v[3:4], v[3:4], v[5:6], v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[3:4], v[1:2], 1.0
	.p2align	6
.LBB9_14:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 3, v[0:1]
	v_add_nc_u32_e32 v0, s8, v0
	v_add_co_u32 v6, vcc_lo, s4, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s5, v5, vcc_lo
	v_add_co_u32 v8, vcc_lo, s6, v4
	v_add_co_ci_u32_e64 v9, null, s7, v5, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	v_cmp_le_i32_e32 vcc_lo, s13, v0
	v_add_co_u32 v4, s0, s1, v4
	global_load_b64 v[8:9], v[8:9], off
	v_add_co_ci_u32_e64 v5, null, s2, v5, s0
	s_or_b32 s3, vcc_lo, s3
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], v[2:3], v[6:7]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[8:9]
	global_store_b64 v[4:5], v[6:7], off
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execnz .LBB9_14
.LBB9_15:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii
		.amdhsa_group_segment_fixed_size 256
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
		.amdhsa_next_free_vgpr 13
		.amdhsa_next_free_sgpr 21
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
	.section	.text._Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii,"axG",@progbits,_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii,comdat
.Lfunc_end9:
	.size	_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii, .Lfunc_end9-_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii
                                        ; -- End function
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.num_vgpr, 13
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.num_agpr, 0
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.numbered_sgpr, 21
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.num_named_barrier, 0
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.private_seg_size, 0
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.uses_vcc, 1
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.has_recursion, 0
	.set _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1548
; TotalNumSgprs: 23
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 23
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
	.section	.text._Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii,"axG",@progbits,_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii,comdat
	.protected	_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii ; -- Begin function _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii
	.globl	_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii
	.p2align	8
	.type	_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii,@function
_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii: ; @_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii
; %bb.0:
	s_load_b64 s[4:5], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s4
	s_cbranch_scc1 .LBB10_15
; %bb.1:
	s_clause 0x1
	s_load_b128 s[12:15], s[0:1], 0x0
	s_load_b64 s[6:7], s[0:1], 0x10
	s_mul_hi_i32 s3, s5, s2
	s_mul_i32 s2, s5, s2
	v_cmp_gt_i32_e32 vcc_lo, s5, v0
	s_lshl_b64 s[8:9], s[2:3], 2
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	s_load_b32 s11, s[14:15], 0x0
	s_add_u32 s4, s12, s8
	s_addc_u32 s10, s13, s9
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB10_5
; %bb.2:
	s_load_b32 s2, s[0:1], 0x2c
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_mov_b32 s12, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s13, s2, 0xffff
.LBB10_3:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	v_add_nc_u32_e32 v1, s13, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, s2, s4, v4
	v_add_co_ci_u32_e64 v5, null, s10, v5, s2
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s2, s5, v1
	global_load_b32 v2, v[4:5], off
	s_or_b32 s12, s2, s12
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v3, v2, v2
	s_and_not1_b32 exec_lo, exec_lo, s12
	s_cbranch_execnz .LBB10_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s12
.LBB10_5:
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
	s_cbranch_execz .LBB10_7
; %bb.6:
	v_lshrrev_b32_e32 v9, 3, v0
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v7, v7, v8
	ds_store_b32 v9, v7
.LBB10_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s12, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB10_12
; %bb.8:
	s_load_b32 s3, s[0:1], 0x2c
	v_mov_b32_e32 v7, 0
	s_mov_b32 s13, exec_lo
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
	s_or_b32 exec_lo, exec_lo, s13
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
	s_cbranch_execz .LBB10_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB10_12:
	s_or_b32 exec_lo, exec_lo, s12
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v1, v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB10_15
; %bb.13:
	v_mul_f32_e32 v2, 0x4f800000, v1
	v_cmp_gt_f32_e32 vcc_lo, 0xf800000, v1
	s_load_b32 s0, s[0:1], 0x2c
	s_mov_b32 s1, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	v_sqrt_f32_e32 v2, v1
	s_waitcnt_depctr 0xfff
	v_add_nc_u32_e32 v3, -1, v2
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v5, -v3, v2, v1
	v_fma_f32 v6, -v4, v2, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_ge_f32_e64 s2, 0, v5
	v_cndmask_b32_e64 v2, v2, v3, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_lt_f32_e64 s2, 0, v6
	v_cndmask_b32_e64 v2, v2, v4, s2
	s_add_u32 s2, s6, s8
	s_addc_u32 s3, s7, s9
	s_waitcnt lgkmcnt(0)
	s_and_b32 s6, s0, 0xffff
	v_mul_f32_e32 v3, 0x37800000, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, v2, v3, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x260
	v_max_f32_e64 v3, s11, s11
	v_cndmask_b32_e32 v1, v2, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_f32_e32 v1, v1, v3
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v3, v5
	v_div_fixup_f32 v2, v2, v1, 1.0
	.p2align	6
.LBB10_14:                              ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[0:1]
	v_add_co_u32 v5, vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s10, v4, vcc_lo
	v_add_co_u32 v3, s0, s2, v3
	v_add_co_ci_u32_e64 v4, null, s3, v4, s0
	global_load_b32 v1, v[5:6], off
	s_waitcnt vmcnt(0)
	v_dual_mul_f32 v1, v2, v1 :: v_dual_add_nc_u32 v0, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s5, v0
	global_store_b32 v[3:4], v1, off
	s_or_b32 s1, vcc_lo, s1
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execnz .LBB10_14
.LBB10_15:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii
		.amdhsa_group_segment_fixed_size 128
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
		.amdhsa_inst_pref_size 9
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii,"axG",@progbits,_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii,comdat
.Lfunc_end10:
	.size	_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii, .Lfunc_end10-_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii
                                        ; -- End function
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.num_vgpr, 10
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.num_agpr, 0
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.numbered_sgpr, 16
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.num_named_barrier, 0
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.private_seg_size, 0
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.uses_vcc, 1
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.uses_flat_scratch, 0
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.has_dyn_sized_stack, 0
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.has_recursion, 0
	.set _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1084
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
	.section	.text._Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii,"axG",@progbits,_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii,comdat
	.protected	_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii ; -- Begin function _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii
	.globl	_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii
	.p2align	8
	.type	_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii,@function
_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii: ; @_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii
; %bb.0:
	s_load_b64 s[4:5], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s4
	s_cbranch_scc1 .LBB11_15
; %bb.1:
	s_clause 0x1
	s_load_b128 s[12:15], s[0:1], 0x0
	s_load_b64 s[6:7], s[0:1], 0x10
	s_mul_hi_i32 s3, s5, s2
	s_mul_i32 s2, s5, s2
	v_mov_b32_e32 v1, 0
	s_lshl_b64 s[8:9], s[2:3], 3
	v_mov_b32_e32 v2, 0
	v_cmp_gt_i32_e32 vcc_lo, s5, v0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[10:11], s[14:15], 0x0
	s_add_u32 s4, s12, s8
	s_addc_u32 s12, s13, s9
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB11_5
; %bb.2:
	s_load_b32 s2, s[0:1], 0x2c
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_mov_b32 s13, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s14, s2, 0xffff
	.p2align	6
.LBB11_3:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	v_add_nc_u32_e32 v3, s14, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, s2, s4, v4
	v_add_co_ci_u32_e64 v5, null, s12, v5, s2
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s2, s5, v3
	global_load_b64 v[4:5], v[4:5], off
	s_or_b32 s13, s2, s13
	s_waitcnt vmcnt(0)
	v_fma_f64 v[1:2], v[4:5], v[4:5], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s13
	s_cbranch_execnz .LBB11_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s13
.LBB11_5:
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
	s_cbranch_execz .LBB11_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB11_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s13, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB11_12
; %bb.8:
	s_load_b32 s3, s[0:1], 0x2c
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s14, exec_lo
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
	s_or_b32 exec_lo, exec_lo, s14
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
	s_cbranch_execz .LBB11_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_mov_b32_e32 v3, 0
	ds_store_b64 v3, v[1:2]
.LBB11_12:
	s_or_b32 exec_lo, exec_lo, s13
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB11_15
; %bb.13:
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[1:2]
	s_load_b32 s0, s[0:1], 0x2c
	s_mov_b32 s1, 0
	s_and_b32 s2, vcc_lo, exec_lo
	s_cselect_b32 s2, 0x100, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[1:2], v[1:2], s2
	s_cselect_b32 s2, 0xffffff80, 0
	v_rsq_f64_e32 v[3:4], v[1:2]
	v_cmp_class_f64_e64 vcc_lo, v[1:2], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[5:6], v[1:2], v[3:4]
	v_mul_f64 v[3:4], v[3:4], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 0.5
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_fma_f64 v[3:4], v[3:4], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[5:6], v[5:6], v[1:2]
	v_fma_f64 v[5:6], v[7:8], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[5:6], v[5:6], v[1:2]
	v_fma_f64 v[3:4], v[7:8], v[3:4], v[5:6]
	v_max_f64 v[5:6], s[10:11], s[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], s2
	s_add_u32 s2, s6, s8
	s_addc_u32 s3, s7, s9
	s_waitcnt lgkmcnt(0)
	s_and_b32 s6, s0, 0xffff
	v_dual_cndmask_b32 v2, v4, v2 :: v_dual_cndmask_b32 v1, v3, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_f64 v[1:2], v[1:2], v[5:6]
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
.LBB11_14:                              ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 3, v[0:1]
	v_add_nc_u32_e32 v0, s6, v0
	v_add_co_u32 v6, vcc_lo, s4, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v7, null, s12, v5, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s5, v0
	v_add_co_u32 v4, s0, s2, v4
	global_load_b64 v[6:7], v[6:7], off
	v_add_co_ci_u32_e64 v5, null, s3, v5, s0
	s_or_b32 s1, vcc_lo, s1
	s_waitcnt vmcnt(0)
	v_mul_f64 v[6:7], v[2:3], v[6:7]
	global_store_b64 v[4:5], v[6:7], off
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execnz .LBB11_14
.LBB11_15:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii
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
	.section	.text._Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii,"axG",@progbits,_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii,comdat
.Lfunc_end11:
	.size	_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii, .Lfunc_end11-_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii
                                        ; -- End function
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.num_vgpr, 11
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.num_agpr, 0
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.numbered_sgpr, 16
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.num_named_barrier, 0
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.private_seg_size, 0
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.uses_vcc, 1
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.uses_flat_scratch, 0
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.has_dyn_sized_stack, 0
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.has_recursion, 0
	.set _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1272
; TotalNumSgprs: 18
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 18
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
	.section	.text._Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,"axG",@progbits,_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,comdat
	.protected	_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_ ; -- Begin function _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
	.globl	_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
	.p2align	8
	.type	_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,@function
_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_: ; @_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
; %bb.0:
	s_load_b128 s[20:23], s[0:1], 0x38
	s_mov_b32 s3, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_gt_i32_e64 s22, v0
	s_cbranch_execz .LBB12_29
; %bb.1:
	s_clause 0x1
	s_load_b64 s[6:7], s[0:1], 0x50
	s_load_b32 s28, s[0:1], 0x48
	s_mul_hi_u32 s3, s22, s22
	s_mul_i32 s5, s22, s22
	s_mul_i32 s8, s2, s3
	s_mul_hi_u32 s9, s2, s5
	v_dual_mov_b32 v1, 0 :: v_dual_lshlrev_b32 v8, 2, v0
	s_mov_b32 s27, 0
	s_mov_b32 s26, s22
	s_waitcnt lgkmcnt(0)
	s_cmp_lg_u64 s[6:7], 0
	s_cselect_b32 s4, -1, 0
	s_ashr_i32 s3, s2, 31
	s_add_i32 s9, s9, s8
	s_mul_i32 s10, s3, s5
	s_mul_i32 s8, s2, s5
	s_add_i32 s9, s9, s10
	v_cndmask_b32_e64 v9, 0, 1, s4
	s_lshl_b64 s[8:9], s[8:9], 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_add_u32 s5, s6, s8
	s_addc_u32 s6, s7, s9
	v_add_co_u32 v2, s5, s5, v8
	v_add_co_ci_u32_e64 v3, null, s6, 0, s5
	s_lshl_b64 s[24:25], s[26:27], 2
	s_mov_b32 s5, 0
	s_mov_b32 s6, s22
	v_dual_mov_b32 v5, v3 :: v_dual_mov_b32 v4, v2
	s_branch .LBB12_3
.LBB12_2:                               ;   in Loop: Header=BB12_3 Depth=1
	v_add_co_u32 v4, vcc_lo, v4, s24
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s25, v5, vcc_lo
	s_add_i32 s6, s6, -1
	s_waitcnt vmcnt(0)
	scratch_store_b32 off, v6, s5
	s_add_i32 s5, s5, 4
	s_cmp_eq_u32 s6, 0
	s_cbranch_scc1 .LBB12_5
.LBB12_3:                               ; =>This Inner Loop Header: Depth=1
	v_mov_b32_e32 v6, 0
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccnz .LBB12_2
; %bb.4:                                ;   in Loop: Header=BB12_3 Depth=1
	global_load_b32 v6, v[4:5], off
	s_branch .LBB12_2
.LBB12_5:
	s_cmp_lt_i32 s20, 1
	s_cbranch_scc1 .LBB12_26
; %bb.6:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b256 s[12:19], s[0:1], 0x0
	v_mad_u64_u32 v[4:5], null, s26, s2, v[0:1]
	s_lshl_b64 s[0:1], s[2:3], 2
	s_waitcnt lgkmcnt(0)
	s_mul_hi_u32 s10, s26, s21
	v_dual_mov_b32 v11, 0 :: v_dual_add_nc_u32 v10, 0x400, v8
	s_mov_b32 s31, s21
	v_mov_b32_e32 v0, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_mad_u64_u32 v[5:6], null, s26, s3, v[0:1]
	s_add_u32 s6, s6, s0
	s_addc_u32 s7, s7, s1
	s_ashr_i32 s2, s21, 31
	s_load_b32 s3, s[6:7], 0x0
	s_mul_i32 s11, s26, s2
	s_mul_i32 s21, s26, s21
	s_add_i32 s6, s10, s11
	s_add_u32 s7, s18, s0
	s_addc_u32 s10, s19, s1
	s_add_u32 s4, s4, s0
	s_addc_u32 s5, s5, s1
	s_cmp_lg_u32 s23, 0
	s_mov_b32 s26, 0
	s_cselect_b32 s11, -1, 0
	s_add_i32 s0, s22, -1
	s_and_b32 s23, s22, 7
	s_cmp_gt_u32 s0, 6
	s_cselect_b32 s27, -1, 0
	s_and_b32 s29, s22, -8
	s_cmp_lg_u32 s23, 0
	s_cselect_b32 s30, -1, 0
	s_branch .LBB12_8
.LBB12_7:                               ;   in Loop: Header=BB12_8 Depth=1
	v_add_co_u32 v0, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	s_add_i32 s26, s26, 1
	s_cmp_eq_u32 s26, s20
	global_store_b32 v[0:1], v6, off
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB12_26
.LBB12_8:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB12_14 Depth 2
                                        ;     Child Loop BB12_18 Depth 2
                                        ;     Child Loop BB12_21 Depth 2
                                        ;     Child Loop BB12_25 Depth 2
	v_mad_u64_u32 v[0:1], null, s21, s26, v[4:5]
	s_mov_b32 s0, -1
	v_mad_u64_u32 v[6:7], null, s6, s26, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v1, v6
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s12, v0
	v_add_co_ci_u32_e64 v7, null, s13, v1, vcc_lo
	v_add_co_u32 v12, vcc_lo, s14, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v13, null, s15, v1, vcc_lo
	global_load_b32 v6, v[6:7], off
	global_load_b32 v7, v[12:13], off
	s_and_not1_b32 vcc_lo, exec_lo, s11
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v6, s28, v6
	s_waitcnt vmcnt(0)
	ds_store_b32 v8, v7
	ds_store_b32 v10, v6
	s_waitcnt lgkmcnt(0)
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
                                        ; implicit-def: $vgpr6_vgpr7
	s_cbranch_vccnz .LBB12_10
; %bb.9:                                ;   in Loop: Header=BB12_8 Depth=1
	v_add_co_u32 v6, vcc_lo, s18, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s19, v1, vcc_lo
	s_mov_b32 s0, 0
.LBB12_10:                              ;   in Loop: Header=BB12_8 Depth=1
	s_mul_i32 s33, s26, s2
	s_and_not1_b32 vcc_lo, exec_lo, s0
	s_mul_hi_u32 s34, s26, s31
	s_mul_i32 s0, s26, s31
	s_cbranch_vccnz .LBB12_12
; %bb.11:                               ;   in Loop: Header=BB12_8 Depth=1
	s_add_i32 s1, s34, s33
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[36:37], s[0:1], 2
	s_add_u32 s36, s7, s36
	s_addc_u32 s37, s10, s37
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v6, s36 :: v_dual_mov_b32 v7, s37
.LBB12_12:                              ;   in Loop: Header=BB12_8 Depth=1
	global_load_b32 v6, v[6:7], off
	s_mov_b32 s1, 0
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v7, s3, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v6, 0x3fb8aa3b, v7
	v_fma_f32 v12, 0x3fb8aa3b, v7, -v6
	v_rndne_f32_e32 v13, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v6, v6, v13
	v_fmac_f32_e32 v12, 0x32a5705f, v7
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v6, v6, v12
	v_cvt_i32_f32_e32 v12, v13
	v_exp_f32_e32 v6, v6
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v6, v6, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v12, 0, v6, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v7
	v_dual_mov_b32 v6, 0 :: v_dual_cndmask_b32 v7, 0x7f800000, v12
	s_and_not1_b32 vcc_lo, exec_lo, s27
	s_cbranch_vccnz .LBB12_16
; %bb.13:                               ;   in Loop: Header=BB12_8 Depth=1
	v_mov_b32_e32 v6, 0
	s_mov_b32 s35, 0
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB12_14:                              ;   Parent Loop BB12_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b128 v[12:15], off, s1
	s_add_i32 s37, s1, 16
	s_add_i32 s35, s35, 8
	scratch_load_b128 v[16:19], off, s37
	v_mov_b32_e32 v24, s1
	s_mov_b32 s36, s1
	s_add_i32 s1, s1, 32
	s_cmp_eq_u32 s29, s35
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v13, v7, v13
	ds_load_b128 v[20:23], v24
	v_mul_f32_e32 v12, v7, v12
	ds_load_b128 v[24:27], v24 offset:16
	v_mul_f32_e32 v14, v7, v14
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v16, v7, v16
	v_mul_f32_e32 v18, v7, v18
	v_mul_f32_e32 v15, v7, v15
	v_mul_f32_e32 v19, v7, v19
	v_mul_f32_e32 v17, v7, v17
	s_clause 0x1
	scratch_store_b128 off, v[12:15], s36
	scratch_store_b128 off, v[16:19], s37
	s_waitcnt lgkmcnt(1)
	v_fmac_f32_e32 v6, v12, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v13, v21
	v_fmac_f32_e32 v6, v14, v22
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v15, v23
	s_waitcnt lgkmcnt(0)
	v_fmac_f32_e32 v6, v16, v24
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v17, v25
	v_fmac_f32_e32 v6, v18, v26
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v6, v19, v27
	s_cbranch_scc0 .LBB12_14
; %bb.15:                               ;   in Loop: Header=BB12_8 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s1, s29
.LBB12_16:                              ;   in Loop: Header=BB12_8 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s30
	s_cbranch_vccnz .LBB12_19
; %bb.17:                               ;   in Loop: Header=BB12_8 Depth=1
	s_lshl_b32 s1, s1, 2
	s_mov_b32 s36, s23
	s_mov_b32 s35, s1
.LBB12_18:                              ;   Parent Loop BB12_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b32 v12, off, s35
	v_mov_b32_e32 v13, s1
	s_add_i32 s36, s36, -1
	s_add_i32 s1, s1, 4
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v12, v7, v12
	ds_load_b32 v13, v13
	scratch_store_b32 off, v12, s35
	s_add_i32 s35, s35, 4
	s_cmp_lg_u32 s36, 0
	s_waitcnt lgkmcnt(0)
	v_fmac_f32_e32 v6, v12, v13
	s_cbranch_scc1 .LBB12_18
.LBB12_19:                              ;   in Loop: Header=BB12_8 Depth=1
	v_add_co_u32 v12, vcc_lo, s16, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v13, null, s17, v1, vcc_lo
	s_add_i32 s1, s34, s33
	s_lshl_b64 s[0:1], s[0:1], 2
	global_load_b32 v7, v[12:13], off
	s_add_u32 s0, s4, s0
	s_addc_u32 s1, s5, s1
	s_and_not1_b32 vcc_lo, exec_lo, s27
	global_load_b32 v12, v11, s[0:1]
	s_mov_b32 s0, 0
	s_waitcnt vmcnt(1)
	v_sub_f32_e32 v6, v7, v6
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mul_f32 v7, v12, v6 :: v_dual_mov_b32 v6, 0
	s_cbranch_vccnz .LBB12_23
; %bb.20:                               ;   in Loop: Header=BB12_8 Depth=1
	s_mov_b32 s1, 0
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB12_21:                              ;   Parent Loop BB12_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b128 v[12:15], off, s0
	s_add_i32 s34, s0, 16
	s_add_i32 s1, s1, 8
	scratch_load_b128 v[16:19], off, s34
	v_mov_b32_e32 v32, s0
	s_mov_b32 s33, s0
	s_add_i32 s0, s0, 32
	ds_load_b128 v[20:23], v32
	ds_load_b128 v[24:27], v32 offset:1024
	ds_load_b128 v[28:31], v32 offset:16
	s_cmp_eq_u32 s29, s1
	s_waitcnt vmcnt(1) lgkmcnt(2)
	v_fma_f32 v12, v7, v20, v12
	v_fma_f32 v13, v7, v21, v13
	v_fmac_f32_e32 v15, v7, v23
	ds_load_b128 v[32:35], v32 offset:1040
	v_fma_f32 v14, v7, v22, v14
	s_waitcnt vmcnt(0) lgkmcnt(1)
	v_dual_fmac_f32 v6, v12, v24 :: v_dual_fmac_f32 v19, v7, v31
	v_fma_f32 v16, v7, v28, v16
	v_fma_f32 v17, v7, v29, v17
	v_fma_f32 v18, v7, v30, v18
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v13, v25
	s_clause 0x1
	scratch_store_b128 off, v[12:15], s33
	scratch_store_b128 off, v[16:19], s34
	v_fmac_f32_e32 v6, v14, v26
	v_fmac_f32_e32 v6, v15, v27
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v16, v32
	v_fmac_f32_e32 v6, v17, v33
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v18, v34
	v_fmac_f32_e32 v6, v19, v35
	s_cbranch_scc0 .LBB12_21
; %bb.22:                               ;   in Loop: Header=BB12_8 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s0, s29
.LBB12_23:                              ;   in Loop: Header=BB12_8 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s30
	s_cbranch_vccnz .LBB12_7
; %bb.24:                               ;   in Loop: Header=BB12_8 Depth=1
	s_lshl_b32 s0, s0, 2
	s_mov_b32 s33, s23
	s_mov_b32 s1, s0
.LBB12_25:                              ;   Parent Loop BB12_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b32 v14, off, s1
	v_mov_b32_e32 v12, s0
	s_add_i32 s33, s33, -1
	s_add_i32 s0, s0, 4
	ds_load_2addr_stride64_b32 v[12:13], v12 offset1:4
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fmac_f32_e32 v14, v7, v12
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v6, v14, v13
	scratch_store_b32 off, v14, s1
	s_add_i32 s1, s1, 4
	s_cmp_lg_u32 s33, 0
	s_cbranch_scc1 .LBB12_25
	s_branch .LBB12_7
.LBB12_26:
	v_cmp_ne_u32_e32 vcc_lo, 1, v9
	s_cbranch_vccnz .LBB12_29
; %bb.27:
	s_mov_b32 s0, 0
.LBB12_28:                              ; =>This Inner Loop Header: Depth=1
	scratch_load_b32 v0, off, s0
	s_add_i32 s22, s22, -1
	s_add_i32 s0, s0, 4
	s_cmp_lg_u32 s22, 0
	s_waitcnt vmcnt(0)
	global_store_b32 v[2:3], v0, off
	v_add_co_u32 v2, vcc_lo, v2, s24
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s25, v3, vcc_lo
	s_cbranch_scc1 .LBB12_28
.LBB12_29:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
		.amdhsa_group_segment_fixed_size 2048
		.amdhsa_private_segment_fixed_size 1040
		.amdhsa_kernarg_size 88
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 1
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 36
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
		.amdhsa_inst_pref_size 13
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,"axG",@progbits,_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,comdat
.Lfunc_end12:
	.size	_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_, .Lfunc_end12-_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
                                        ; -- End function
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.num_vgpr, 36
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.num_agpr, 0
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.numbered_sgpr, 38
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.num_named_barrier, 0
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.private_seg_size, 1040
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.uses_vcc, 1
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.uses_flat_scratch, 1
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.has_dyn_sized_stack, 0
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.has_recursion, 0
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1588
; TotalNumSgprs: 40
; NumVgprs: 36
; ScratchSize: 1040
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 2048 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 40
; NumVGPRsForWavesPerEU: 36
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,"axG",@progbits,_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,comdat
	.protected	_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_ ; -- Begin function _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
	.globl	_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
	.p2align	8
	.type	_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,@function
_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_: ; @_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
; %bb.0:
	s_load_b128 s[20:23], s[0:1], 0x38
	s_mov_b32 s3, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_gt_i32_e64 s22, v0
	s_cbranch_execz .LBB13_30
; %bb.1:
	s_load_b128 s[24:27], s[0:1], 0x48
	s_mul_hi_u32 s3, s22, s22
	s_mul_i32 s5, s22, s22
	s_mul_i32 s6, s2, s3
	s_mul_hi_u32 s7, s2, s5
	v_dual_mov_b32 v1, 0 :: v_dual_lshlrev_b32 v10, 3, v0
	s_mov_b32 s29, 0
	s_mov_b32 s28, s22
	s_waitcnt lgkmcnt(0)
	s_cmp_lg_u64 s[26:27], 0
	s_cselect_b32 s4, -1, 0
	s_ashr_i32 s3, s2, 31
	s_add_i32 s7, s7, s6
	s_mul_i32 s8, s3, s5
	s_mul_i32 s6, s2, s5
	s_add_i32 s7, s7, s8
	v_cndmask_b32_e64 v11, 0, 1, s4
	s_lshl_b64 s[6:7], s[6:7], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_add_u32 s5, s26, s6
	s_addc_u32 s6, s27, s7
	v_add_co_u32 v2, s5, s5, v10
	v_add_co_ci_u32_e64 v3, null, s6, 0, s5
	s_lshl_b64 s[26:27], s[28:29], 3
	s_mov_b32 s5, 0
	s_mov_b32 s6, s22
	v_dual_mov_b32 v5, v3 :: v_dual_mov_b32 v4, v2
	s_branch .LBB13_4
	.p2align	6
.LBB13_2:                               ;   in Loop: Header=BB13_4 Depth=1
	global_load_b64 v[6:7], v[4:5], off
.LBB13_3:                               ;   in Loop: Header=BB13_4 Depth=1
	v_add_co_u32 v4, vcc_lo, v4, s26
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s27, v5, vcc_lo
	s_add_i32 s6, s6, -1
	s_waitcnt vmcnt(0)
	scratch_store_b64 off, v[6:7], s5
	s_add_i32 s5, s5, 8
	s_cmp_eq_u32 s6, 0
	s_cbranch_scc1 .LBB13_6
.LBB13_4:                               ; =>This Inner Loop Header: Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccz .LBB13_2
; %bb.5:                                ;   in Loop: Header=BB13_4 Depth=1
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0
	s_branch .LBB13_3
.LBB13_6:
	s_cmp_lt_i32 s20, 1
	s_cbranch_scc1 .LBB13_27
; %bb.7:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b256 s[12:19], s[0:1], 0x0
	v_mad_u64_u32 v[4:5], null, s28, s2, v[0:1]
	s_lshl_b64 s[30:31], s[2:3], 3
	s_mov_b32 s1, s21
	s_mul_hi_u32 s0, s28, s21
	s_mul_i32 s33, s28, s21
	v_dual_mov_b32 v13, 0 :: v_dual_add_nc_u32 v12, 0x800, v10
	v_mov_b32_e32 v0, v5
	s_waitcnt lgkmcnt(0)
	s_mov_b32 s10, 0x652b82fe
	s_mov_b32 s36, 0x7c89e6b0
	s_mov_b32 s38, 0x14761f6e
	s_mov_b32 s40, 0x1852b7b0
	v_mad_u64_u32 v[5:6], null, s28, s3, v[0:1]
	s_mov_b32 s2, 0xfefa39ef
	s_mov_b32 s42, 0x11122322
	s_mov_b32 s44, 0x555502a1
	s_add_u32 s34, s6, s30
	s_addc_u32 s35, s7, s31
	s_ashr_i32 s21, s21, 31
	s_mov_b32 s6, 0x3b39803f
	s_mul_i32 s28, s28, s21
	s_mov_b32 s46, 0x55555511
	s_add_i32 s53, s0, s28
	s_add_u32 s54, s18, s30
	s_addc_u32 s55, s19, s31
	s_add_u32 s56, s4, s30
	s_addc_u32 s57, s5, s31
	s_load_b64 s[4:5], s[34:35], 0x0
	s_cmp_lg_u32 s23, 0
	s_mov_b32 s28, 0xfca7ab0c
	s_cselect_b32 s23, -1, 0
	s_add_i32 s0, s22, -1
	s_and_b32 s58, s22, 7
	s_cmp_gt_u32 s0, 6
	s_mov_b32 s30, 0x6a5dcb37
	s_cselect_b32 s59, -1, 0
	s_and_b32 s60, s22, -8
	s_cmp_lg_u32 s58, 0
	s_mov_b32 s34, 0x623fde64
	s_mov_b32 s48, 11
	s_mov_b32 s52, 0
	s_mov_b32 s11, 0x3ff71547
	s_mov_b32 s3, 0xbfe62e42
	s_mov_b32 s7, 0xbc7abc9e
	s_mov_b32 s29, 0x3e928af3
	s_mov_b32 s31, 0x3e5ade15
	s_mov_b32 s35, 0x3ec71dee
	s_mov_b32 s37, 0x3efa0199
	s_mov_b32 s39, 0x3f2a01a0
	s_mov_b32 s41, 0x3f56c16c
	s_mov_b32 s43, 0x3f811111
	s_mov_b32 s45, 0x3fa55555
	s_mov_b32 s47, 0x3fc55555
	s_cselect_b32 s61, -1, 0
	s_mov_b32 s49, 0x3fe00000
	s_branch .LBB13_9
.LBB13_8:                               ;   in Loop: Header=BB13_9 Depth=1
	v_add_co_u32 v0, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	s_add_i32 s52, s52, 1
	s_cmp_eq_u32 s52, s20
	global_store_b64 v[0:1], v[8:9], off
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB13_27
.LBB13_9:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB13_15 Depth 2
                                        ;     Child Loop BB13_19 Depth 2
                                        ;     Child Loop BB13_22 Depth 2
                                        ;     Child Loop BB13_26 Depth 2
	v_mad_u64_u32 v[0:1], null, s33, s52, v[4:5]
	s_mov_b32 s0, -1
	v_mad_u64_u32 v[6:7], null, s53, s52, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v1, v6
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s12, v0
	v_add_co_ci_u32_e64 v7, null, s13, v1, vcc_lo
	v_add_co_u32 v8, vcc_lo, s14, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s15, v1, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_and_not1_b32 vcc_lo, exec_lo, s23
	global_load_b64 v[8:9], v[8:9], off
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], s[24:25], v[6:7]
	s_waitcnt vmcnt(0)
	ds_store_b64 v10, v[8:9]
	ds_store_b64 v12, v[6:7]
	s_waitcnt lgkmcnt(0)
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
                                        ; implicit-def: $vgpr6_vgpr7
	s_cbranch_vccnz .LBB13_11
; %bb.10:                               ;   in Loop: Header=BB13_9 Depth=1
	v_add_co_u32 v6, vcc_lo, s18, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s19, v1, vcc_lo
	s_mov_b32 s0, 0
.LBB13_11:                              ;   in Loop: Header=BB13_9 Depth=1
	s_mul_i32 s62, s52, s21
	s_and_not1_b32 vcc_lo, exec_lo, s0
	s_mul_hi_u32 s63, s52, s1
	s_mul_i32 s50, s52, s1
	s_cbranch_vccnz .LBB13_13
; %bb.12:                               ;   in Loop: Header=BB13_9 Depth=1
	s_add_i32 s51, s63, s62
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[64:65], s[50:51], 3
	s_add_u32 s64, s54, s64
	s_addc_u32 s65, s55, s65
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v6, s64 :: v_dual_mov_b32 v7, s65
.LBB13_13:                              ;   in Loop: Header=BB13_9 Depth=1
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[6:7], s[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[8:9], v[6:7], s[10:11]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[6:7]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[6:7]
	v_rndne_f64_e32 v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[8:9], s[2:3], v[6:7]
	v_cvt_i32_f64_e32 v18, v[8:9]
	v_fma_f64 v[14:15], v[8:9], s[6:7], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], s[30:31], s[28:29]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[36:37]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[40:41]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[44:45]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[46:47]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[48:49]
	v_fma_f64 v[16:17], v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[16:17], 1.0
	v_ldexp_f64 v[14:15], v[8:9], v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_mov_b32 v8, 0 :: v_dual_cndmask_b32 v15, 0x7ff00000, v15
	s_and_b32 vcc_lo, s0, vcc_lo
	v_dual_mov_b32 v9, 0 :: v_dual_cndmask_b32 v6, 0, v14
	s_and_not1_b32 vcc_lo, exec_lo, s59
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v7, 0, v15, s0
	s_mov_b32 s0, 0
	s_cbranch_vccnz .LBB13_17
; %bb.14:                               ;   in Loop: Header=BB13_9 Depth=1
	v_mov_b32_e32 v8, 0
	v_mov_b32_e32 v9, 0
	s_mov_b32 s51, 0
.LBB13_15:                              ;   Parent Loop BB13_9 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b128 v[14:17], off, s0
	s_add_i32 s65, s0, 16
	s_add_i32 s66, s0, 32
	s_clause 0x1
	scratch_load_b128 v[18:21], off, s65
	scratch_load_b128 v[22:25], off, s66
	s_add_i32 s67, s0, 48
	v_mov_b32_e32 v38, s0
	scratch_load_b128 v[26:29], off, s67
	ds_load_b128 v[30:33], v38
	ds_load_b128 v[34:37], v38 offset:16
	s_add_i32 s51, s51, 8
	s_mov_b32 s64, s0
	s_add_i32 s0, s0, 64
	s_cmp_eq_u32 s60, s51
	s_waitcnt vmcnt(3)
	v_mul_f64 v[14:15], v[6:7], v[14:15]
	v_mul_f64 v[16:17], v[6:7], v[16:17]
	s_waitcnt vmcnt(2)
	v_mul_f64 v[18:19], v[6:7], v[18:19]
	v_mul_f64 v[20:21], v[6:7], v[20:21]
	s_waitcnt vmcnt(1)
	v_mul_f64 v[22:23], v[6:7], v[22:23]
	v_mul_f64 v[24:25], v[6:7], v[24:25]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[26:27], v[6:7], v[26:27]
	v_mul_f64 v[28:29], v[6:7], v[28:29]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[8:9], v[14:15], v[30:31], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[16:17], v[32:33], v[8:9]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[8:9], v[18:19], v[34:35], v[8:9]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[20:21], v[36:37], v[8:9]
	ds_load_b128 v[30:33], v38 offset:32
	ds_load_b128 v[34:37], v38 offset:48
	s_clause 0x3
	scratch_store_b128 off, v[14:17], s64
	scratch_store_b128 off, v[18:21], s65
	scratch_store_b128 off, v[22:25], s66
	scratch_store_b128 off, v[26:29], s67
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[8:9], v[22:23], v[30:31], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[24:25], v[32:33], v[8:9]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[8:9], v[26:27], v[34:35], v[8:9]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[28:29], v[36:37], v[8:9]
	s_cbranch_scc0 .LBB13_15
; %bb.16:                               ;   in Loop: Header=BB13_9 Depth=1
	s_mov_b32 s0, s60
.LBB13_17:                              ;   in Loop: Header=BB13_9 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s61
	s_cbranch_vccnz .LBB13_20
; %bb.18:                               ;   in Loop: Header=BB13_9 Depth=1
	s_lshl_b32 s0, s0, 3
	s_mov_b32 s64, s58
	s_mov_b32 s51, s0
.LBB13_19:                              ;   Parent Loop BB13_9 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b64 v[14:15], off, s51
	v_mov_b32_e32 v16, s0
	s_add_i32 s64, s64, -1
	s_add_i32 s0, s0, 8
	ds_load_b64 v[16:17], v16
	s_waitcnt vmcnt(0)
	v_mul_f64 v[14:15], v[6:7], v[14:15]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[16:17], v[8:9]
	scratch_store_b64 off, v[14:15], s51
	s_add_i32 s51, s51, 8
	s_cmp_lg_u32 s64, 0
	s_cbranch_scc1 .LBB13_19
.LBB13_20:                              ;   in Loop: Header=BB13_9 Depth=1
	v_add_co_u32 v6, vcc_lo, s16, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s17, v1, vcc_lo
	s_add_i32 s51, s63, s62
	s_mov_b32 s0, 0
	s_lshl_b64 s[50:51], s[50:51], 3
	global_load_b64 v[6:7], v[6:7], off
	s_add_u32 s50, s56, s50
	s_addc_u32 s51, s57, s51
	s_and_not1_b32 vcc_lo, exec_lo, s59
	global_load_b64 v[14:15], v13, s[50:51]
	s_waitcnt vmcnt(1)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_mov_b32_e32 v8, 0
	v_mov_b32_e32 v9, 0
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[6:7], v[14:15], v[6:7]
	s_cbranch_vccnz .LBB13_24
; %bb.21:                               ;   in Loop: Header=BB13_9 Depth=1
	s_mov_b32 s50, 0
.LBB13_22:                              ;   Parent Loop BB13_9 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b128 v[14:17], off, s0
	s_add_i32 s62, s0, 16
	s_add_i32 s63, s0, 32
	s_clause 0x1
	scratch_load_b128 v[18:21], off, s62
	scratch_load_b128 v[22:25], off, s63
	s_add_i32 s64, s0, 48
	v_mov_b32_e32 v46, s0
	scratch_load_b128 v[26:29], off, s64
	ds_load_b128 v[30:33], v46
	ds_load_b128 v[34:37], v46 offset:16
	ds_load_b128 v[38:41], v46 offset:2048
	ds_load_b128 v[42:45], v46 offset:2064
	s_add_i32 s50, s50, 8
	s_mov_b32 s51, s0
	s_add_i32 s0, s0, 64
	s_cmp_eq_u32 s60, s50
	s_waitcnt vmcnt(3) lgkmcnt(3)
	v_fma_f64 v[14:15], v[6:7], v[30:31], v[14:15]
	v_fma_f64 v[16:17], v[6:7], v[32:33], v[16:17]
	s_waitcnt vmcnt(2) lgkmcnt(2)
	v_fma_f64 v[18:19], v[6:7], v[34:35], v[18:19]
	v_fma_f64 v[20:21], v[6:7], v[36:37], v[20:21]
	ds_load_b128 v[30:33], v46 offset:32
	ds_load_b128 v[34:37], v46 offset:48
	s_waitcnt vmcnt(1) lgkmcnt(1)
	v_fma_f64 v[22:23], v[6:7], v[30:31], v[22:23]
	v_fma_f64 v[24:25], v[6:7], v[32:33], v[24:25]
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[26:27], v[6:7], v[34:35], v[26:27]
	v_fma_f64 v[28:29], v[6:7], v[36:37], v[28:29]
	v_fma_f64 v[8:9], v[14:15], v[38:39], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[16:17], v[40:41], v[8:9]
	v_fma_f64 v[8:9], v[18:19], v[42:43], v[8:9]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[20:21], v[44:45], v[8:9]
	ds_load_b128 v[38:41], v46 offset:2080
	ds_load_b128 v[42:45], v46 offset:2096
	s_clause 0x3
	scratch_store_b128 off, v[14:17], s51
	scratch_store_b128 off, v[18:21], s62
	scratch_store_b128 off, v[22:25], s63
	scratch_store_b128 off, v[26:29], s64
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[8:9], v[22:23], v[38:39], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[24:25], v[40:41], v[8:9]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[8:9], v[26:27], v[42:43], v[8:9]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[28:29], v[44:45], v[8:9]
	s_cbranch_scc0 .LBB13_22
; %bb.23:                               ;   in Loop: Header=BB13_9 Depth=1
	s_mov_b32 s0, s60
.LBB13_24:                              ;   in Loop: Header=BB13_9 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s61
	s_cbranch_vccnz .LBB13_8
; %bb.25:                               ;   in Loop: Header=BB13_9 Depth=1
	s_lshl_b32 s0, s0, 3
	s_mov_b32 s51, s58
	s_mov_b32 s50, s0
	.p2align	6
.LBB13_26:                              ;   Parent Loop BB13_9 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b64 v[18:19], off, s50
	v_mov_b32_e32 v14, s0
	s_add_i32 s51, s51, -1
	s_add_i32 s0, s0, 8
	ds_load_2addr_stride64_b64 v[14:17], v14 offset1:4
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[14:15], v[6:7], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[16:17], v[8:9]
	scratch_store_b64 off, v[14:15], s50
	s_add_i32 s50, s50, 8
	s_cmp_lg_u32 s51, 0
	s_cbranch_scc1 .LBB13_26
	s_branch .LBB13_8
.LBB13_27:
	v_cmp_ne_u32_e32 vcc_lo, 1, v11
	s_cbranch_vccnz .LBB13_30
; %bb.28:
	s_mov_b32 s0, 0
.LBB13_29:                              ; =>This Inner Loop Header: Depth=1
	scratch_load_b64 v[0:1], off, s0
	s_add_i32 s22, s22, -1
	s_add_i32 s0, s0, 8
	s_cmp_lg_u32 s22, 0
	s_waitcnt vmcnt(0)
	global_store_b64 v[2:3], v[0:1], off
	v_add_co_u32 v2, vcc_lo, v2, s26
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s27, v3, vcc_lo
	s_cbranch_scc1 .LBB13_29
.LBB13_30:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
		.amdhsa_group_segment_fixed_size 4096
		.amdhsa_private_segment_fixed_size 2064
		.amdhsa_kernarg_size 88
		.amdhsa_user_sgpr_count 2
		.amdhsa_user_sgpr_dispatch_ptr 0
		.amdhsa_user_sgpr_queue_ptr 0
		.amdhsa_user_sgpr_kernarg_segment_ptr 1
		.amdhsa_user_sgpr_dispatch_id 0
		.amdhsa_user_sgpr_private_segment_size 0
		.amdhsa_wavefront_size32 1
		.amdhsa_uses_dynamic_stack 0
		.amdhsa_enable_private_segment 1
		.amdhsa_system_sgpr_workgroup_id_x 1
		.amdhsa_system_sgpr_workgroup_id_y 0
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 47
		.amdhsa_next_free_sgpr 68
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
	.section	.text._Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,"axG",@progbits,_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_,comdat
.Lfunc_end13:
	.size	_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_, .Lfunc_end13-_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
                                        ; -- End function
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.num_vgpr, 47
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.num_agpr, 0
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.numbered_sgpr, 68
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.num_named_barrier, 0
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.private_seg_size, 2064
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.uses_vcc, 1
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.uses_flat_scratch, 1
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.has_dyn_sized_stack, 0
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.has_recursion, 0
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2192
; TotalNumSgprs: 70
; NumVgprs: 47
; ScratchSize: 2064
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 4096 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 70
; NumVGPRsForWavesPerEU: 47
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z16row_scale_kernelIfEvPKT_S2_PS0_ii,"axG",@progbits,_Z16row_scale_kernelIfEvPKT_S2_PS0_ii,comdat
	.protected	_Z16row_scale_kernelIfEvPKT_S2_PS0_ii ; -- Begin function _Z16row_scale_kernelIfEvPKT_S2_PS0_ii
	.globl	_Z16row_scale_kernelIfEvPKT_S2_PS0_ii
	.p2align	8
	.type	_Z16row_scale_kernelIfEvPKT_S2_PS0_ii,@function
_Z16row_scale_kernelIfEvPKT_S2_PS0_ii:  ; @_Z16row_scale_kernelIfEvPKT_S2_PS0_ii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[8:9], s[0:1], 0x18
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v4
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mul_hi_i32 s3, s9, s8
	s_mul_i32 s2, s9, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[2:3]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB14_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_ashr_i32 s0, s9, 31
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v6, null, s5, v1, vcc_lo
	global_load_b32 v6, v[5:6], off
	v_or_b32_e32 v5, s0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_ne_u64_e32 vcc_lo, 0, v[4:5]
                                        ; implicit-def: $vgpr4_vgpr5
	s_and_saveexec_b32 s1, vcc_lo
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB14_3
; %bb.2:
	s_ashr_i32 s4, s0, 31
	v_ashrrev_i32_e32 v9, 31, v3
	s_add_u32 s10, s9, s4
	s_mov_b32 s5, s4
	s_addc_u32 s11, s0, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[10:11], s[10:11], s[4:5]
	v_add_co_u32 v2, vcc_lo, v2, v9
	v_cvt_f32_u32_e32 v4, s10
	v_cvt_f32_u32_e32 v5, s11
	s_sub_u32 s8, 0, s10
	s_subb_u32 s12, 0, s11
	v_add_co_ci_u32_e64 v3, null, v3, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v4, v5, 0x4f800000, v4
	v_xor_b32_e32 v10, v2, v9
	v_xor_b32_e32 v11, v3, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x5f7ffffc, v4
	v_mul_f32_e32 v5, 0x2f800000, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v5, v5
	v_fmamk_f32 v4, v5, 0xcf800000, v4
	v_cvt_u32_f32_e32 v5, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_readfirstlane_b32 s0, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s5, v4
	s_mul_i32 s13, s8, s0
	s_mul_hi_u32 s15, s8, s5
	s_mul_i32 s14, s12, s5
	s_add_i32 s13, s15, s13
	s_mul_i32 s16, s8, s5
	s_add_i32 s13, s13, s14
	s_mul_hi_u32 s15, s5, s16
	s_mul_i32 s18, s5, s13
	s_mul_hi_u32 s17, s0, s16
	s_mul_i32 s14, s0, s16
	s_mul_hi_u32 s16, s5, s13
	s_add_u32 s15, s15, s18
	s_addc_u32 s16, 0, s16
	s_mul_hi_u32 s19, s0, s13
	s_add_u32 s14, s15, s14
	s_mul_i32 s13, s0, s13
	s_addc_u32 s14, s16, s17
	s_addc_u32 s15, s19, 0
	s_add_u32 s13, s14, s13
	s_addc_u32 s14, 0, s15
	s_add_u32 s5, s5, s13
	s_cselect_b32 s13, -1, 0
	s_mul_hi_u32 s15, s8, s5
	s_cmp_lg_u32 s13, 0
	s_mul_i32 s13, s8, s5
	s_addc_u32 s0, s0, s14
	s_mul_i32 s12, s12, s5
	s_mul_i32 s8, s8, s0
	s_mul_hi_u32 s14, s5, s13
	s_add_i32 s8, s15, s8
	s_mul_hi_u32 s15, s0, s13
	s_add_i32 s8, s8, s12
	s_mul_i32 s12, s0, s13
	s_mul_i32 s17, s5, s8
	s_mul_hi_u32 s16, s5, s8
	s_add_u32 s14, s14, s17
	s_addc_u32 s16, 0, s16
	s_mul_hi_u32 s13, s0, s8
	s_add_u32 s12, s14, s12
	s_mul_i32 s8, s0, s8
	s_addc_u32 s12, s16, s15
	s_addc_u32 s13, s13, 0
	s_add_u32 s8, s12, s8
	s_addc_u32 s12, 0, s13
	s_add_u32 s5, s5, s8
	s_cselect_b32 s8, -1, 0
	v_mul_hi_u32 v12, v10, s5
	s_cmp_lg_u32 s8, 0
	v_mad_u64_u32 v[4:5], null, v11, s5, 0
	s_addc_u32 s0, s0, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[2:3], null, v10, s0, 0
	v_mad_u64_u32 v[7:8], null, v11, s0, 0
	v_add_co_u32 v2, vcc_lo, v12, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	v_add_co_u32 v2, vcc_lo, v2, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v2, vcc_lo, v3, v5, vcc_lo
	v_add_co_ci_u32_e32 v3, vcc_lo, 0, v8, vcc_lo
	v_add_co_u32 v4, vcc_lo, v2, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, 0, v3, vcc_lo
	v_mul_lo_u32 v7, s11, v4
	v_mad_u64_u32 v[2:3], null, s10, v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v8, s10, v5
	v_sub_co_u32 v2, vcc_lo, v10, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v3, v3, v8, v7
	v_add_co_u32 v8, s0, v4, 2
	v_add_co_ci_u32_e64 v10, null, 0, v5, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v11, v3
	v_sub_co_u32 v12, s0, v2, s10
	v_sub_co_ci_u32_e64 v3, null, v11, v3, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s11, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s10, v12
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s0
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s11, v7
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s10, v2
	v_cndmask_b32_e64 v2, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s11, v3
	v_cndmask_b32_e64 v13, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s11, v7
	v_cndmask_b32_e32 v7, v12, v11, vcc_lo
	v_add_co_u32 v11, vcc_lo, v4, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v12, null, 0, v5, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s11, v3
	v_cndmask_b32_e32 v2, v13, v2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v7
	v_xor_b32_e32 v7, s4, v9
	v_cmp_ne_u32_e64 s0, 0, v2
	v_dual_cndmask_b32 v2, v11, v8 :: v_dual_cndmask_b32 v3, v12, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, v4, v2, s0
	v_cndmask_b32_e64 v3, v5, v3, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v2, v2, v7
	v_xor_b32_e32 v3, v3, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v4, vcc_lo, v2, v7
	v_sub_co_ci_u32_e64 v5, null, v3, v7, vcc_lo
                                        ; implicit-def: $vgpr2_vgpr3
.LBB14_3:
	s_and_not1_saveexec_b32 s0, s1
	s_cbranch_execz .LBB14_5
; %bb.4:
	v_cvt_f32_u32_e32 v3, s9
	s_sub_i32 s1, 0, s9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v3, v3
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v3, 0x4f7ffffe, v3
	v_cvt_u32_f32_e32 v3, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, s1, v3
	v_mul_hi_u32 v4, v3, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v3, v3, v4
	v_mul_hi_u32 v3, v2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, v3, s9
	v_sub_nc_u32_e32 v2, v2, v4
	v_add_nc_u32_e32 v4, 1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s9, v2
	v_cmp_le_u32_e32 vcc_lo, s9, v2
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_mov_b32 v5, 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s9, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v4, 1, v3
	v_cndmask_b32_e32 v4, v3, v4, vcc_lo
.LBB14_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 2, v[4:5]
	v_add_co_u32 v2, vcc_lo, s6, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v3, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v2, v6, v2
	global_store_b32 v[0:1], v2, off
.LBB14_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z16row_scale_kernelIfEvPKT_S2_PS0_ii
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
		.amdhsa_next_free_vgpr 14
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
		.amdhsa_inst_pref_size 9
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z16row_scale_kernelIfEvPKT_S2_PS0_ii,"axG",@progbits,_Z16row_scale_kernelIfEvPKT_S2_PS0_ii,comdat
.Lfunc_end14:
	.size	_Z16row_scale_kernelIfEvPKT_S2_PS0_ii, .Lfunc_end14-_Z16row_scale_kernelIfEvPKT_S2_PS0_ii
                                        ; -- End function
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.num_vgpr, 14
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.num_agpr, 0
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.numbered_sgpr, 20
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.num_named_barrier, 0
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.private_seg_size, 0
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.uses_vcc, 1
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.uses_flat_scratch, 0
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.has_dyn_sized_stack, 0
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.has_recursion, 0
	.set _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1096
; TotalNumSgprs: 22
; NumVgprs: 14
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 22
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
	.section	.text._Z16row_scale_kernelIdEvPKT_S2_PS0_ii,"axG",@progbits,_Z16row_scale_kernelIdEvPKT_S2_PS0_ii,comdat
	.protected	_Z16row_scale_kernelIdEvPKT_S2_PS0_ii ; -- Begin function _Z16row_scale_kernelIdEvPKT_S2_PS0_ii
	.globl	_Z16row_scale_kernelIdEvPKT_S2_PS0_ii
	.p2align	8
	.type	_Z16row_scale_kernelIdEvPKT_S2_PS0_ii,@function
_Z16row_scale_kernelIdEvPKT_S2_PS0_ii:  ; @_Z16row_scale_kernelIdEvPKT_S2_PS0_ii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[8:9], s[0:1], 0x18
	v_mov_b32_e32 v6, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v6
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[4:5], null, s3, s2, v[0:1]
	s_mul_hi_i32 s3, s9, s8
	s_mul_i32 s2, s9, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[4:5]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB15_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_lshlrev_b64 v[0:1], 3, v[4:5]
	s_ashr_i32 s0, s9, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_or_b32_e32 v7, s0, v5
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_cmp_ne_u64_e32 vcc_lo, 0, v[6:7]
                                        ; implicit-def: $vgpr6_vgpr7
	global_load_b64 v[2:3], v[2:3], off
	s_and_saveexec_b32 s1, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB15_3
; %bb.2:
	s_ashr_i32 s4, s0, 31
	v_ashrrev_i32_e32 v10, 31, v5
	s_add_u32 s10, s9, s4
	s_mov_b32 s5, s4
	s_addc_u32 s11, s0, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[10:11], s[10:11], s[4:5]
	v_add_co_u32 v4, vcc_lo, v4, v10
	v_cvt_f32_u32_e32 v6, s10
	v_cvt_f32_u32_e32 v7, s11
	s_sub_u32 s8, 0, s10
	s_subb_u32 s12, 0, s11
	v_add_co_ci_u32_e64 v5, null, v5, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v6, v7, 0x4f800000, v6
	v_xor_b32_e32 v11, v4, v10
	v_xor_b32_e32 v12, v5, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v6, v6
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v6, 0x5f7ffffc, v6
	v_mul_f32_e32 v7, 0x2f800000, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v7, v7
	v_fmamk_f32 v6, v7, 0xcf800000, v6
	v_cvt_u32_f32_e32 v7, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v6, v6
	v_readfirstlane_b32 s0, v7
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s5, v6
	s_mul_i32 s13, s8, s0
	s_mul_hi_u32 s15, s8, s5
	s_mul_i32 s14, s12, s5
	s_add_i32 s13, s15, s13
	s_mul_i32 s16, s8, s5
	s_add_i32 s13, s13, s14
	s_mul_hi_u32 s15, s5, s16
	s_mul_i32 s18, s5, s13
	s_mul_hi_u32 s17, s0, s16
	s_mul_i32 s14, s0, s16
	s_mul_hi_u32 s16, s5, s13
	s_add_u32 s15, s15, s18
	s_addc_u32 s16, 0, s16
	s_mul_hi_u32 s19, s0, s13
	s_add_u32 s14, s15, s14
	s_mul_i32 s13, s0, s13
	s_addc_u32 s14, s16, s17
	s_addc_u32 s15, s19, 0
	s_add_u32 s13, s14, s13
	s_addc_u32 s14, 0, s15
	s_add_u32 s5, s5, s13
	s_cselect_b32 s13, -1, 0
	s_mul_hi_u32 s15, s8, s5
	s_cmp_lg_u32 s13, 0
	s_mul_i32 s13, s8, s5
	s_addc_u32 s0, s0, s14
	s_mul_i32 s12, s12, s5
	s_mul_i32 s8, s8, s0
	s_mul_hi_u32 s14, s5, s13
	s_add_i32 s8, s15, s8
	s_mul_hi_u32 s15, s0, s13
	s_add_i32 s8, s8, s12
	s_mul_i32 s12, s0, s13
	s_mul_i32 s17, s5, s8
	s_mul_hi_u32 s16, s5, s8
	s_add_u32 s14, s14, s17
	s_addc_u32 s16, 0, s16
	s_mul_hi_u32 s13, s0, s8
	s_add_u32 s12, s14, s12
	s_mul_i32 s8, s0, s8
	s_addc_u32 s12, s16, s15
	s_addc_u32 s13, s13, 0
	s_add_u32 s8, s12, s8
	s_addc_u32 s12, 0, s13
	s_add_u32 s5, s5, s8
	s_cselect_b32 s8, -1, 0
	v_mul_hi_u32 v13, v11, s5
	s_cmp_lg_u32 s8, 0
	v_mad_u64_u32 v[6:7], null, v12, s5, 0
	s_addc_u32 s0, s0, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[4:5], null, v11, s0, 0
	v_mad_u64_u32 v[8:9], null, v12, s0, 0
	v_add_co_u32 v4, vcc_lo, v13, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	v_add_co_u32 v4, vcc_lo, v4, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v4, vcc_lo, v5, v7, vcc_lo
	v_add_co_ci_u32_e32 v5, vcc_lo, 0, v9, vcc_lo
	v_add_co_u32 v6, vcc_lo, v4, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v7, null, 0, v5, vcc_lo
	v_mul_lo_u32 v8, s11, v6
	v_mad_u64_u32 v[4:5], null, s10, v6, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v9, s10, v7
	v_sub_co_u32 v4, vcc_lo, v11, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v5, v5, v9, v8
	v_add_co_u32 v9, s0, v6, 2
	v_add_co_ci_u32_e64 v11, null, 0, v7, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v8, v12, v5
	v_sub_co_u32 v13, s0, v4, s10
	v_sub_co_ci_u32_e64 v5, null, v12, v5, vcc_lo
	v_subrev_co_ci_u32_e64 v8, null, s11, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s10, v13
	v_subrev_co_ci_u32_e64 v8, null, 0, v8, s0
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s11, v8
	v_cndmask_b32_e64 v13, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s10, v4
	v_cndmask_b32_e64 v4, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s11, v5
	v_cndmask_b32_e64 v14, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s11, v8
	v_cndmask_b32_e32 v8, v13, v12, vcc_lo
	v_add_co_u32 v12, vcc_lo, v6, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v13, null, 0, v7, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s11, v5
	v_cndmask_b32_e32 v4, v14, v4, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v8
	v_xor_b32_e32 v8, s4, v10
	v_cmp_ne_u32_e64 s0, 0, v4
	v_dual_cndmask_b32 v4, v12, v9 :: v_dual_cndmask_b32 v5, v13, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v4, v6, v4, s0
	v_cndmask_b32_e64 v5, v7, v5, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v4, v4, v8
	v_xor_b32_e32 v5, v5, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v6, vcc_lo, v4, v8
	v_sub_co_ci_u32_e64 v7, null, v5, v8, vcc_lo
                                        ; implicit-def: $vgpr4_vgpr5
.LBB15_3:
	s_and_not1_saveexec_b32 s0, s1
	s_cbranch_execz .LBB15_5
; %bb.4:
	v_cvt_f32_u32_e32 v5, s9
	s_sub_i32 s1, 0, s9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v5, v5
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v5, 0x4f7ffffe, v5
	v_cvt_u32_f32_e32 v5, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v6, s1, v5
	v_mul_hi_u32 v6, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v5, v5, v6
	v_mul_hi_u32 v5, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v6, v5, s9
	v_sub_nc_u32_e32 v4, v4, v6
	v_add_nc_u32_e32 v6, 1, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v7, s9, v4
	v_cmp_le_u32_e32 vcc_lo, s9, v4
	v_dual_cndmask_b32 v4, v4, v7 :: v_dual_mov_b32 v7, 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, v5, v6, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s9, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v6, 1, v5
	v_cndmask_b32_e32 v6, v5, v6, vcc_lo
.LBB15_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 3, v[6:7]
	v_add_co_u32 v4, vcc_lo, s6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB15_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z16row_scale_kernelIdEvPKT_S2_PS0_ii
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
		.amdhsa_next_free_vgpr 15
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
		.amdhsa_inst_pref_size 9
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z16row_scale_kernelIdEvPKT_S2_PS0_ii,"axG",@progbits,_Z16row_scale_kernelIdEvPKT_S2_PS0_ii,comdat
.Lfunc_end15:
	.size	_Z16row_scale_kernelIdEvPKT_S2_PS0_ii, .Lfunc_end15-_Z16row_scale_kernelIdEvPKT_S2_PS0_ii
                                        ; -- End function
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.num_vgpr, 15
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.num_agpr, 0
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.numbered_sgpr, 20
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.num_named_barrier, 0
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.private_seg_size, 0
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.uses_vcc, 1
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.uses_flat_scratch, 0
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.has_dyn_sized_stack, 0
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.has_recursion, 0
	.set _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1104
; TotalNumSgprs: 22
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 22
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
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.section	.AMDGPU.csdata,"",@progbits
	.type	__hip_cuid_d3a2918ef1c13ce7,@object ; @__hip_cuid_d3a2918ef1c13ce7
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_d3a2918ef1c13ce7
__hip_cuid_d3a2918ef1c13ce7:
	.byte	0                               ; 0x0
	.size	__hip_cuid_d3a2918ef1c13ce7, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_d3a2918ef1c13ce7
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
    .name:           _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiiiS2_.kd
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
    .name:           _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiiiS2_.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z26ssm_conv_state_save_kernelIfEvPKT_S2_PS0_iii.kd
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
    .name:           _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z26ssm_conv_state_save_kernelIdEvPKT_S2_PS0_iii.kd
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
      - .address_space:  global
        .offset:         72
        .size:           8
        .value_kind:     global_buffer
      - .offset:         80
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         84
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         88
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         92
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         94
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         96
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         98
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         100
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         102
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         136
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         144
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 336
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
    .private_segment_fixed_size: 1040
    .sgpr_count:     30
    .sgpr_spill_count: 0
    .symbol:         _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     23
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
      - .address_space:  global
        .offset:         72
        .size:           8
        .value_kind:     global_buffer
      - .offset:         80
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         84
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         88
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         92
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         94
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         96
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         98
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         100
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         102
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         136
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         144
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 336
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_
    .private_segment_fixed_size: 2064
    .sgpr_count:     75
    .sgpr_spill_count: 0
    .symbol:         _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiS3_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     36
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
      - .offset:         56
        .size:           4
        .value_kind:     by_value
      - .offset:         60
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         64
        .size:           8
        .value_kind:     global_buffer
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
    .name:           _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_
    .private_segment_fixed_size: 1040
    .sgpr_count:     27
    .sgpr_spill_count: 0
    .symbol:         _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiiiS3_.kd
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
      - .offset:         56
        .size:           4
        .value_kind:     by_value
      - .offset:         60
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         64
        .size:           8
        .value_kind:     global_buffer
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
    .name:           _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_
    .private_segment_fixed_size: 2064
    .sgpr_count:     68
    .sgpr_spill_count: 0
    .symbol:         _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiiiS3_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     40
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
    .group_segment_fixed_size: 128
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         _Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii.kd
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
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     23
    .sgpr_spill_count: 0
    .symbol:         _Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii.kd
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
    .group_segment_fixed_size: 128
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii.kd
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
    .name:           _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii.kd
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
      - .offset:         72
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         80
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 2048
    .kernarg_segment_align: 8
    .kernarg_segment_size: 88
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
    .private_segment_fixed_size: 1040
    .sgpr_count:     40
    .sgpr_spill_count: 0
    .symbol:         _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     36
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
      - .offset:         72
        .size:           8
        .value_kind:     by_value
      - .address_space:  global
        .offset:         80
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 4096
    .kernarg_segment_align: 8
    .kernarg_segment_size: 88
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_
    .private_segment_fixed_size: 2064
    .sgpr_count:     70
    .sgpr_spill_count: 0
    .symbol:         _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_S3_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     47
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
    .name:           _Z16row_scale_kernelIfEvPKT_S2_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         _Z16row_scale_kernelIfEvPKT_S2_PS0_ii.kd
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
    .name:           _Z16row_scale_kernelIdEvPKT_S2_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         _Z16row_scale_kernelIdEvPKT_S2_PS0_ii.kd
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
