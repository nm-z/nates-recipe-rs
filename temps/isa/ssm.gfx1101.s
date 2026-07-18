	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.section	.text._Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii,"axG",@progbits,_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii,comdat
	.protected	_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii ; -- Begin function _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii
	.globl	_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii
	.p2align	8
	.type	_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii,@function
_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii: ; @_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[8:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_hi_i32 s3, s9, s8
	s_mul_i32 s2, s9, s8
	v_ashrrev_i32_e32 v2, 31, v1
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[1:2]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_11
; %bb.1:
	s_abs_i32 s2, s9
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_ashr_i32 s8, s9, 31
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
	v_xor_b32_e32 v4, s9, v1
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_load_b256 s[0:7], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	v_xor_b32_e32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v7, v0, v4
	v_mul_lo_u32 v0, v7, s9
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[4:5], 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v3, v1, v0
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 2, v[3:4]
	s_cbranch_scc1 .LBB0_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, s4, v4
	v_add_co_ci_u32_e64 v9, null, s5, v5, vcc_lo
	global_load_b32 v0, v[8:9], off
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc0 .LBB0_4
	s_branch .LBB0_8
.LBB0_3:
	v_mov_b32_e32 v0, 0
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc1 .LBB0_8
.LBB0_4:
	v_mad_i64_i32 v[8:9], null, v3, s10, 0
	v_add_co_u32 v6, vcc_lo, s0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s1, v5, vcc_lo
	s_sub_i32 s0, 0, s10
	v_add_nc_u32_e32 v7, 1, v7
	v_lshlrev_b64 v[8:9], 2, v[8:9]
	v_add_co_u32 v3, vcc_lo, s2, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s3, v9, vcc_lo
	v_mov_b32_e32 v8, s0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB0_6
	.p2align	6
.LBB0_5:                                ;   in Loop: Header=BB0_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v3, vcc_lo, v3, 4
	v_add_co_u32 v8, s0, v8, 1
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	s_and_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccnz .LBB0_8
.LBB0_6:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v9, v7, v8
	s_mov_b32 s0, exec_lo
	v_cmpx_lt_i32_e32 -1, v9
	s_cbranch_execz .LBB0_5
; %bb.7:                                ;   in Loop: Header=BB0_6 Depth=1
	v_mad_u64_u32 v[10:11], null, v9, s9, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[12:13], null, v9, s8, v[11:12]
	v_mov_b32_e32 v11, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[9:10], 2, v[10:11]
	v_add_co_u32 v9, vcc_lo, v6, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, v5, v10, vcc_lo
	global_load_b32 v11, v[3:4], off
	global_load_b32 v9, v[9:10], off
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v0, v11, v9
	s_branch .LBB0_5
.LBB0_8:
	s_set_inst_prefetch_distance 0x2
	s_cmp_lg_u32 s11, 0
	s_cbranch_scc0 .LBB0_10
; %bb.9:
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
.LBB0_10:
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, vcc_lo, s6, v1
	v_add_co_ci_u32_e64 v2, null, s7, v2, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b32 v[1:2], v0, off
.LBB0_11:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii
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
		.amdhsa_next_free_vgpr 15
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
		.amdhsa_inst_pref_size 7
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii,"axG",@progbits,_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii,comdat
.Lfunc_end0:
	.size	_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii, .Lfunc_end0-_Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii
                                        ; -- End function
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.num_vgpr, 15
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.num_agpr, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.numbered_sgpr, 12
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.num_named_barrier, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.private_seg_size, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.uses_vcc, 1
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.uses_flat_scratch, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.has_dyn_sized_stack, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.has_recursion, 0
	.set _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 852
; TotalNumSgprs: 14
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
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
	.section	.text._Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii,"axG",@progbits,_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii,comdat
	.protected	_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii ; -- Begin function _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii
	.globl	_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii
	.p2align	8
	.type	_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii,@function
_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii: ; @_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[8:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_hi_i32 s3, s9, s8
	s_mul_i32 s2, s9, s8
	v_ashrrev_i32_e32 v2, 31, v1
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[1:2]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_11
; %bb.1:
	s_abs_i32 s2, s9
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_ashr_i32 s8, s9, 31
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
	v_xor_b32_e32 v4, s9, v1
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_load_b256 s[0:7], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	v_xor_b32_e32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v8, v0, v4
	v_mul_lo_u32 v0, v8, s9
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[4:5], 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v5, v1, v0
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b64 v[6:7], 3, v[5:6]
	s_cbranch_scc1 .LBB1_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s4, v6
	v_add_co_ci_u32_e64 v4, null, s5, v7, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc0 .LBB1_4
	s_branch .LBB1_8
.LBB1_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc1 .LBB1_8
.LBB1_4:
	v_mad_i64_i32 v[9:10], null, v5, s10, 0
	v_add_co_u32 v0, vcc_lo, s0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
	s_sub_i32 s0, 0, s10
	v_add_nc_u32_e32 v8, 1, v8
	v_lshlrev_b64 v[9:10], 3, v[9:10]
	v_add_co_u32 v5, vcc_lo, s2, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s3, v10, vcc_lo
	v_mov_b32_e32 v9, s0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB1_6
	.p2align	6
.LBB1_5:                                ;   in Loop: Header=BB1_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v5, vcc_lo, v5, 8
	v_add_co_u32 v9, s0, v9, 1
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	s_and_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccnz .LBB1_8
.LBB1_6:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v10, v8, v9
	s_mov_b32 s0, exec_lo
	v_cmpx_lt_i32_e32 -1, v10
	s_cbranch_execz .LBB1_5
; %bb.7:                                ;   in Loop: Header=BB1_6 Depth=1
	v_mad_u64_u32 v[11:12], null, v10, s9, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[13:14], null, v10, s8, v[12:13]
	v_mov_b32_e32 v12, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[10:11], 3, v[11:12]
	v_add_co_u32 v10, vcc_lo, v0, v10
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v11, null, v7, v11, vcc_lo
	global_load_b64 v[12:13], v[5:6], off
	global_load_b64 v[10:11], v[10:11], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[12:13], v[10:11], v[3:4]
	s_branch .LBB1_5
.LBB1_8:
	s_set_inst_prefetch_distance 0x2
	s_cmp_lg_u32 s11, 0
	s_cbranch_scc0 .LBB1_10
; %bb.9:
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
.LBB1_10:
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB1_11:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii
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
		.amdhsa_next_free_vgpr 15
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
		.amdhsa_inst_pref_size 10
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii,"axG",@progbits,_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii,comdat
.Lfunc_end1:
	.size	_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii, .Lfunc_end1-_Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii
                                        ; -- End function
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.num_vgpr, 15
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.num_agpr, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.numbered_sgpr, 12
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.num_named_barrier, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.private_seg_size, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.uses_vcc, 1
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.uses_flat_scratch, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.has_dyn_sized_stack, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.has_recursion, 0
	.set _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1176
; TotalNumSgprs: 14
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
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
	.section	.text._Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii,"axG",@progbits,_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii,comdat
	.protected	_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii ; -- Begin function _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii
	.globl	_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii
	.p2align	8
	.type	_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii,@function
_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii: ; @_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x54
	s_load_b128 s[20:23], s[0:1], 0x38
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s21, v1
	s_cbranch_execz .LBB2_13
; %bb.1:
	s_cmp_gt_i32 s22, 0
	s_cselect_b32 s24, -1, 0
	s_cmp_lt_i32 s22, 1
	s_cbranch_scc1 .LBB2_4
; %bb.2:
	v_mov_b32_e32 v0, 0
	s_mov_b32 s2, 0
	s_mov_b32 s3, s22
.LBB2_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s3, s3, -1
	scratch_store_b32 off, v0, s2
	s_add_i32 s2, s2, 4
	s_cmp_eq_u32 s3, 0
	s_cbranch_scc0 .LBB2_3
.LBB2_4:
	s_cmp_lt_i32 s20, 1
	s_cbranch_scc1 .LBB2_13
; %bb.5:
	s_load_b256 s[4:11], s[0:1], 0x20
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b256 s[12:19], s[0:1], 0x0
	v_mov_b32_e32 v9, 0
	s_ashr_i32 s23, s22, 31
	s_ashr_i32 s1, s21, 31
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	s_lshl_b64 s[2:3], s[22:23], 2
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s6, v3
	v_add_co_ci_u32_e64 v4, null, s7, v4, vcc_lo
	global_load_b32 v0, v[3:4], off
	v_mad_i64_i32 v[3:4], null, v1, s22, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	v_add_co_u32 v3, vcc_lo, s16, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s17, v4, vcc_lo
	s_mov_b32 s16, s21
	s_mov_b32 s17, 0
	s_mov_b32 s21, 0x3e9b6dac
	s_branch .LBB2_8
.LBB2_6:                                ;   in Loop: Header=BB2_8 Depth=1
	v_mov_b32_e32 v11, 0
.LBB2_7:                                ;   in Loop: Header=BB2_8 Depth=1
	v_add_co_u32 v5, vcc_lo, s8, v5
	s_add_i32 s17, s17, 1
	s_add_u32 s18, s18, s2
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v11, v0, v12
	v_add_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_addc_u32 s19, s19, s3
	s_add_u32 s4, s4, s2
	s_addc_u32 s5, s5, s3
	s_cmp_eq_u32 s17, s20
	global_store_b32 v[5:6], v11, off
	s_cbranch_scc1 .LBB2_13
.LBB2_8:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB2_12 Depth 2
	v_mad_u64_u32 v[5:6], null, s17, s16, v[1:2]
	s_mov_b32 s0, exec_lo
	v_mad_u64_u32 v[7:8], null, s17, s1, v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v6, v7
	v_lshlrev_b64 v[5:6], 2, v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s14, v5
	v_add_co_ci_u32_e64 v8, null, s15, v6, vcc_lo
	global_load_b32 v10, v[7:8], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f32_e32 0x41a00000, v10
	s_cbranch_execz .LBB2_10
; %bb.9:                                ;   in Loop: Header=BB2_8 Depth=1
	v_mul_f32_e32 v7, 0x3fb8aa3b, v10
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rndne_f32_e32 v8, v7
	v_fma_f32 v11, 0x3fb8aa3b, v10, -v7
	v_sub_f32_e32 v7, v7, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v11, 0x32a5705f, v10
	v_cvt_i32_f32_e32 v8, v8
	v_add_f32_e32 v7, v7, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v7, v7
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v7, v7, v8
	v_cndmask_b32_e32 v7, 0, v7, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v10, 0x7f800000, v7, vcc_lo
	v_add_f32_e32 v11, 1.0, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[7:8], v11
	v_frexp_exp_i32_f64_e32 v7, v[7:8]
	v_frexp_mant_f32_e32 v8, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v8
	v_add_f32_e32 v8, -1.0, v11
	v_sub_f32_e32 v13, v8, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v13, 1.0, v13 :: v_dual_sub_f32 v8, v10, v8
	v_add_f32_e32 v8, v8, v13
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	v_cmp_neq_f32_e32 vcc_lo, 0x7f800000, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v12, 0, v7
	v_cvt_f32_i32_e32 v7, v7
	v_ldexp_f32 v11, v11, v12
	v_ldexp_f32 v8, v8, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v14, 1.0, v11
	v_dual_add_f32 v12, -1.0, v11 :: v_dual_add_f32 v13, -1.0, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v15, 1.0, v12
	v_sub_f32_e32 v13, v11, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v11, v11, v15
	v_add_f32_e32 v13, v8, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v8, v8, v11
	v_dual_add_f32 v16, v12, v8 :: v_dual_add_f32 v15, v14, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v12, v12, v16
	v_rcp_f32_e32 v11, v15
	v_sub_f32_e32 v14, v14, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_dual_add_f32 v8, v8, v12 :: v_dual_add_f32 v13, v13, v14
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v17, v16, v11
	v_mul_f32_e32 v18, v15, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v14, v17, v15, -v18
	v_fmac_f32_e32 v14, v17, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v19, v18, v14
	v_sub_f32_e32 v20, v16, v19
	v_sub_f32_e32 v12, v19, v18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v16, v16, v20
	v_sub_f32_e32 v12, v12, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v16, v16, v19
	v_add_f32_e32 v8, v8, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v8, v12, v8
	v_add_f32_e32 v12, v20, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v14, v11, v12
	v_dual_sub_f32 v19, v20, v12 :: v_dual_mul_f32 v16, v15, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v8, v8, v19
	v_fma_f32 v15, v14, v15, -v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v15, v14, v13
	v_add_f32_e32 v13, v16, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v18, v12, v13
	v_sub_f32_e32 v12, v12, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v12, v12, v13
	v_add_f32_e32 v8, v8, v12
	v_add_f32_e32 v12, v17, v14
	v_sub_f32_e32 v16, v13, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v13, v16, v15
	v_dual_add_f32 v8, v13, v8 :: v_dual_sub_f32 v13, v12, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v8, v18, v8
	v_dual_sub_f32 v13, v14, v13 :: v_dual_mul_f32 v8, v11, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v8, v13, v8
	v_add_f32_e32 v11, v12, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v13, v11, v11
	v_fmaak_f32 v14, s21, v13, 0x3ecc95a3
	v_mul_f32_e32 v15, v11, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fmaak_f32 v13, v13, v14, 0x3f2aaada
	v_ldexp_f32 v14, v11, 1
	v_sub_f32_e32 v11, v11, v12
	v_mul_f32_e32 v13, v15, v13
	v_mul_f32_e32 v15, 0x3f317218, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v8, v8, v11
	v_ldexp_f32 v8, v8, 1
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v12, v14, v13
	v_sub_f32_e32 v11, v12, v14
	v_fma_f32 v14, 0x3f317218, v7, -v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v11, v13, v11
	v_fmac_f32_e32 v14, 0xb102e308, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v7, v8, v11 :: v_dual_add_f32 v8, v15, v14
	v_add_f32_e32 v11, v12, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v15, v8, v15
	v_dual_add_f32 v13, v8, v11 :: v_dual_sub_f32 v12, v11, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v16, v13, v8
	v_sub_f32_e32 v7, v7, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v17, v13, v16
	v_dual_sub_f32 v11, v11, v16 :: v_dual_sub_f32 v14, v14, v15
	v_sub_f32_e32 v8, v8, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v12, v14, v7
	v_add_f32_e32 v8, v11, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v8, v12, v8
	v_add_f32_e32 v15, v13, v8
	v_sub_f32_e32 v11, v12, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v12, v12, v11
	v_sub_f32_e32 v7, v7, v11
	v_dual_sub_f32 v11, v15, v13 :: v_dual_sub_f32 v12, v14, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v8, v8, v11 :: v_dual_add_f32 v7, v7, v12
	v_add_f32_e32 v7, v7, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v7, v15, v7
	v_cndmask_b32_e32 v7, 0x7f800000, v7, vcc_lo
	v_cmp_gt_f32_e32 vcc_lo, 0x33800000, v10
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v10, v7, v10, vcc_lo
.LBB2_10:                               ;   in Loop: Header=BB2_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v7, vcc_lo, s12, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s13, v6, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s24
	global_load_b32 v12, v[7:8], off
	s_cbranch_vccnz .LBB2_6
; %bb.11:                               ;   in Loop: Header=BB2_8 Depth=1
	s_waitcnt vmcnt(0)
	v_dual_mul_f32 v13, v10, v12 :: v_dual_mov_b32 v8, v4
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v7, v3
	s_mov_b32 s23, 0
	s_mov_b64 s[6:7], s[4:5]
	s_mov_b64 s[10:11], s[18:19]
	s_mov_b32 s25, s22
.LBB2_12:                               ;   Parent Loop BB2_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b32 v14, v[7:8], off
	global_load_b32 v15, v9, s[10:11]
	scratch_load_b32 v16, off, s23
	global_load_b32 v17, v9, s[6:7]
	s_add_i32 s25, s25, -1
	s_add_u32 s10, s10, 4
	s_addc_u32 s11, s11, 0
	s_add_u32 s6, s6, 4
	s_addc_u32 s7, s7, 0
	s_waitcnt vmcnt(2)
	v_dual_mul_f32 v14, v10, v14 :: v_dual_mul_f32 v15, v13, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v18, 0x3fb8aa3b, v14
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v14
	v_cmp_nlt_f32_e64 s0, 0x42b17218, v14
	v_fma_f32 v19, 0x3fb8aa3b, v14, -v18
	v_rndne_f32_e32 v20, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v19, 0x32a5705f, v14 :: v_dual_sub_f32 v18, v18, v20
	v_add_f32_e32 v18, v18, v19
	v_cvt_i32_f32_e32 v19, v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v18, v18
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v18, v18, v19
	v_cndmask_b32_e32 v18, 0, v18, vcc_lo
	v_add_co_u32 v7, vcc_lo, v7, 4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_cndmask_b32_e64 v14, 0x7f800000, v18, s0
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v15, v16, v14
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v11, v15, v17
	scratch_store_b32 off, v15, s23
	s_add_i32 s23, s23, 4
	s_cmp_eq_u32 s25, 0
	s_cbranch_scc0 .LBB2_12
	s_branch .LBB2_7
.LBB2_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii
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
		.amdhsa_next_free_vgpr 21
		.amdhsa_next_free_sgpr 26
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
	.section	.text._Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii,"axG",@progbits,_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii,comdat
.Lfunc_end2:
	.size	_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii, .Lfunc_end2-_Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii
                                        ; -- End function
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.num_vgpr, 21
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.num_agpr, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.numbered_sgpr, 26
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.num_named_barrier, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.private_seg_size, 1040
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.uses_vcc, 1
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.has_recursion, 0
	.set _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1480
; TotalNumSgprs: 28
; NumVgprs: 21
; ScratchSize: 1040
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 28
; NumVGPRsForWavesPerEU: 21
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii,"axG",@progbits,_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii,comdat
	.protected	_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii ; -- Begin function _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii
	.globl	_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii
	.p2align	8
	.type	_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii,@function
_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii: ; @_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x54
	s_load_b128 s[20:23], s[0:1], 0x38
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s21, v1
	s_cbranch_execz .LBB3_13
; %bb.1:
	s_cmp_gt_i32 s22, 0
	s_cselect_b32 s33, -1, 0
	s_cmp_lt_i32 s22, 1
	s_cbranch_scc1 .LBB3_4
; %bb.2:
	v_mov_b32_e32 v2, 0
	s_mov_b32 s2, 0
	s_mov_b32 s3, s22
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v3, v2
.LBB3_3:                                ; =>This Inner Loop Header: Depth=1
	s_add_i32 s3, s3, -1
	scratch_store_b64 off, v[2:3], s2
	s_add_i32 s2, s2, 8
	s_cmp_eq_u32 s3, 0
	s_cbranch_scc0 .LBB3_3
.LBB3_4:
	s_cmp_lt_i32 s20, 1
	s_cbranch_scc1 .LBB3_13
; %bb.5:
	s_load_b256 s[4:11], s[0:1], 0x20
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b256 s[12:19], s[0:1], 0x0
	v_mad_i64_i32 v[5:6], null, v1, s22, 0
	v_mov_b32_e32 v0, 0
	s_delay_alu instid0(VALU_DEP_3)
	v_lshlrev_b64 v[3:4], 3, v[1:2]
	s_ashr_i32 s23, s22, 31
	s_waitcnt lgkmcnt(0)
	s_mov_b32 s10, 0xfefa39ef
	s_mov_b32 s24, 0xfca7ab0c
	s_mov_b32 s26, 0x6a5dcb37
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	s_mov_b32 s28, 0x623fde64
	s_mov_b32 s30, 0x7c89e6b0
	s_mov_b32 s34, 0x14761f6e
	s_mov_b32 s36, 0x1852b7b0
	s_mov_b32 s38, 0x11122322
	s_mov_b32 s40, 0x555502a1
	s_mov_b32 s42, 0x55555511
	s_mov_b32 s44, 11
	v_add_co_u32 v3, vcc_lo, s6, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s7, v4, vcc_lo
	v_add_co_u32 v5, vcc_lo, s16, v5
	v_add_co_ci_u32_e64 v6, null, s17, v6, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s16, 0x3b39803f
	s_mov_b32 s46, 0x55555555
	s_mov_b32 s48, 0x6b47b09a
	s_mov_b32 s50, 0xbf559e2b
	s_mov_b32 s52, 0xd7f4df2e
	s_mov_b32 s54, 0x16291751
	s_mov_b32 s56, 0x9b27acf1
	s_mov_b32 s58, 0x998ef7b6
	s_ashr_i32 s70, s21, 31
	s_lshl_b64 s[2:3], s[22:23], 3
	s_mov_b32 s23, 0
	s_mov_b32 s7, 0x3ff71547
	s_mov_b32 s11, 0xbfe62e42
	s_mov_b32 s17, 0xbc7abc9e
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
	s_mov_b32 s47, 0x3fe55555
	s_mov_b32 s49, 0x3fc38538
	s_mov_b32 s51, 0x3fc3ab76
	s_mov_b32 s53, 0x3fc7474d
	s_mov_b32 s55, 0x3fcc71c0
	s_mov_b32 s57, 0x3fd24924
	s_mov_b32 s59, 0x3fd99999
	s_mov_b32 s60, 0x55555780
	s_mov_b32 s63, 0x3fe62e42
	s_mov_b32 s65, 0x3c7abc9e
	s_branch .LBB3_8
.LBB3_6:                                ;   in Loop: Header=BB3_8 Depth=1
	v_mov_b32_e32 v13, 0
	v_mov_b32_e32 v14, 0
.LBB3_7:                                ;   in Loop: Header=BB3_8 Depth=1
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[9:10], v[3:4], v[11:12], v[13:14]
	v_add_co_u32 v7, vcc_lo, s8, v7
	s_add_i32 s23, s23, 1
	s_add_u32 s18, s18, s2
	v_add_co_ci_u32_e64 v8, null, s9, v8, vcc_lo
	s_addc_u32 s19, s19, s3
	s_add_u32 s4, s4, s2
	s_addc_u32 s5, s5, s3
	s_cmp_eq_u32 s23, s20
	global_store_b64 v[7:8], v[9:10], off
	s_cbranch_scc1 .LBB3_13
.LBB3_8:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB3_12 Depth 2
	v_mad_u64_u32 v[7:8], null, s23, s21, v[1:2]
	s_mov_b32 s66, exec_lo
	v_mad_u64_u32 v[9:10], null, s23, s70, v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v8, v9
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v9, vcc_lo, s14, v7
	v_add_co_ci_u32_e64 v10, null, s15, v8, vcc_lo
	global_load_b64 v[9:10], v[9:10], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f64_e32 0x40340000, v[9:10]
	s_cbranch_execz .LBB3_10
; %bb.9:                                ;   in Loop: Header=BB3_8 Depth=1
	v_mul_f64 v[11:12], v[9:10], s[6:7]
	v_cmp_ngt_f64_e32 vcc_lo, 0xc090cc00, v[9:10]
	s_mov_b32 s61, s47
	s_mov_b32 s62, s10
	s_mov_b32 s64, s16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[11:12], v[11:12]
	v_fma_f64 v[13:14], v[11:12], s[10:11], v[9:10]
	v_cvt_i32_f64_e32 v17, v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], s[16:17], v[13:14]
	v_fma_f64 v[15:16], v[13:14], s[26:27], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[28:29]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[34:35]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[38:39]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[42:43]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[44:45]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], 1.0
	v_fma_f64 v[11:12], v[13:14], v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[11:12], v[11:12], v17
	v_dual_cndmask_b32 v12, 0, v12 :: v_dual_cndmask_b32 v11, 0, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[11:12], 1.0
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[11:12]
	v_cmp_ngt_f64_e64 s1, -1.0, v[11:12]
	v_frexp_mant_f64_e32 v[13:14], v[9:10]
	v_frexp_exp_i32_f64_e32 v17, v[9:10]
	v_add_f64 v[15:16], v[9:10], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, s[46:47], v[13:14]
	v_add_f64 v[13:14], v[15:16], -v[9:10]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_subrev_co_ci_u32_e64 v33, null, 0, v17, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], 1.0
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[11:12]
	v_sub_nc_u32_e32 v19, 0, v33
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[9:10], v[9:10], v19
	v_add_f64 v[13:14], v[15:16], v[13:14]
	s_and_b32 vcc_lo, vcc_lo, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[9:10], 1.0
	v_add_f64 v[23:24], v[9:10], -1.0
	v_ldexp_f64 v[13:14], v[13:14], v19
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[17:18], -1.0
	v_add_f64 v[25:26], v[23:24], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[9:10], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[13:14], v[15:16]
	v_add_f64 v[9:10], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[17:18], v[15:16]
	v_add_f64 v[25:26], v[23:24], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[21:22], v[19:20]
	v_add_f64 v[17:18], v[19:20], -v[17:18]
	v_add_f64 v[23:24], v[25:26], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[27:28], -v[19:20], v[21:22], 1.0
	v_add_f64 v[9:10], v[9:10], -v[23:24]
	v_fma_f64 v[21:22], v[27:28], v[21:22], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], -v[19:20], v[21:22], 1.0
	v_fma_f64 v[13:14], v[13:14], v[21:22], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[21:22], v[25:26], v[13:14]
	v_mul_f64 v[27:28], v[19:20], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[21:22], v[19:20], -v[27:28]
	v_fma_f64 v[17:18], v[21:22], v[15:16], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[29:30], v[27:28], v[17:18]
	v_add_f64 v[31:32], v[25:26], -v[29:30]
	v_add_f64 v[23:24], v[29:30], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[25:26], -v[31:32]
	v_add_f64 v[17:18], v[23:24], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[25:26], v[25:26], -v[29:30]
	v_add_f64 v[9:10], v[9:10], v[25:26]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[17:18], v[9:10]
	v_add_f64 v[17:18], v[31:32], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[23:24], v[13:14], v[17:18]
	v_add_f64 v[29:30], v[31:32], -v[17:18]
	v_mul_f64 v[25:26], v[19:20], v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], v[29:30]
	v_fma_f64 v[19:20], v[23:24], v[19:20], -v[25:26]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[23:24], v[15:16], v[19:20]
	v_add_f64 v[19:20], v[25:26], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[27:28], v[17:18], -v[19:20]
	v_add_f64 v[25:26], v[19:20], -v[25:26]
	v_add_f64 v[17:18], v[17:18], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[25:26], -v[15:16]
	v_add_f64 v[17:18], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], v[17:18]
	v_add_f64 v[17:18], v[21:22], v[23:24]
	v_add_f64 v[9:10], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[17:18], -v[21:22]
	v_add_f64 v[9:10], v[27:28], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[23:24], -v[15:16]
	v_mul_f64 v[9:10], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[15:16], v[9:10]
	v_add_f64 v[13:14], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[13:14], v[13:14]
	v_fma_f64 v[19:20], v[15:16], s[50:51], s[48:49]
	v_mul_f64 v[21:22], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[52:53]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[54:55]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[56:57]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[58:59]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[15:16], v[19:20], s[60:61]
	v_ldexp_f64 v[19:20], v[13:14], 1
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	v_mul_f64 v[15:16], v[21:22], v[15:16]
	v_cvt_f64_i32_e32 v[21:22], v33
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	v_add_f64 v[17:18], v[19:20], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[23:24], v[21:22], s[62:63]
	v_ldexp_f64 v[9:10], v[9:10], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[17:18], -v[19:20]
	v_fma_f64 v[19:20], v[21:22], s[62:63], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_fma_f64 v[15:16], v[21:22], s[64:65], v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], v[13:14]
	v_add_f64 v[13:14], v[23:24], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[17:18], v[9:10]
	v_add_f64 v[23:24], v[13:14], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], v[19:20]
	v_add_f64 v[17:18], v[19:20], -v[17:18]
	v_add_f64 v[15:16], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[25:26], v[21:22], -v[13:14]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[27:28], v[21:22], -v[25:26]
	v_add_f64 v[17:18], v[19:20], -v[25:26]
	v_add_f64 v[19:20], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[13:14], -v[27:28]
	v_add_f64 v[13:14], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[15:16]
	v_add_f64 v[13:14], v[19:20], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[19:20], -v[17:18]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	v_add_f64 v[23:24], v[21:22], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[15:16], -v[19:20]
	v_add_f64 v[17:18], v[23:24], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], v[15:16]
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[9:10], v[13:14]
	v_add_f64 v[9:10], v[23:24], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v9, 0, v9, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[11:12]
	v_cndmask_b32_e64 v10, 0x7ff00000, v10, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v10, 0x7ff80000, v10, s1
	v_cndmask_b32_e32 v10, 0xfff00000, v10, vcc_lo
.LBB3_10:                               ;   in Loop: Header=BB3_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s66
	v_add_co_u32 v11, vcc_lo, s12, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v12, null, s13, v8, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s33
	global_load_b64 v[11:12], v[11:12], off
	s_cbranch_vccnz .LBB3_6
; %bb.11:                               ;   in Loop: Header=BB3_8 Depth=1
	s_waitcnt vmcnt(0)
	v_mul_f64 v[15:16], v[9:10], v[11:12]
	v_dual_mov_b32 v13, 0 :: v_dual_mov_b32 v18, v6
	v_dual_mov_b32 v14, 0 :: v_dual_mov_b32 v17, v5
	s_mov_b32 s1, 0
	s_mov_b64 s[66:67], s[4:5]
	s_mov_b64 s[68:69], s[18:19]
	s_mov_b32 s61, s22
.LBB3_12:                               ;   Parent Loop BB3_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[19:20], v[17:18], off
	scratch_load_b64 v[27:28], off, s1
	s_clause 0x1
	global_load_b64 v[29:30], v0, s[68:69]
	global_load_b64 v[31:32], v0, s[66:67]
	s_add_i32 s61, s61, -1
	s_waitcnt vmcnt(3)
	v_mul_f64 v[19:20], v[9:10], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[21:22], v[19:20], s[6:7]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[19:20]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[19:20]
	v_rndne_f64_e32 v[21:22], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[23:24], v[21:22], s[10:11], v[19:20]
	v_cvt_i32_f64_e32 v33, v[21:22]
	v_fma_f64 v[23:24], v[21:22], s[16:17], v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], s[26:27], s[24:25]
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[30:31]
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[36:37]
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[40:41]
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[44:45]
	v_fma_f64 v[25:26], v[23:24], v[25:26], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[21:22], v[23:24], v[25:26], 1.0
	v_ldexp_f64 v[21:22], v[21:22], v33
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v22, 0x7ff00000, v22, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_add_u32 s68, s68, 8
	v_cndmask_b32_e32 v19, 0, v21, vcc_lo
	v_add_co_u32 v17, vcc_lo, v17, 8
	v_cndmask_b32_e64 v20, 0, v22, s0
	v_add_co_ci_u32_e64 v18, null, 0, v18, vcc_lo
	s_addc_u32 s69, s69, 0
	s_add_u32 s66, s66, 8
	s_waitcnt vmcnt(2)
	v_mul_f64 v[19:20], v[27:28], v[19:20]
	s_addc_u32 s67, s67, 0
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[15:16], v[29:30], v[19:20]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[13:14], v[31:32], v[19:20], v[13:14]
	scratch_store_b64 off, v[19:20], s1
	s_add_i32 s1, s1, 8
	s_cmp_eq_u32 s61, 0
	s_cbranch_scc0 .LBB3_12
	s_branch .LBB3_7
.LBB3_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii
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
		.amdhsa_next_free_vgpr 34
		.amdhsa_next_free_sgpr 71
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
	.section	.text._Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii,"axG",@progbits,_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii,comdat
.Lfunc_end3:
	.size	_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii, .Lfunc_end3-_Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii
                                        ; -- End function
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.num_vgpr, 34
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.num_agpr, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.numbered_sgpr, 71
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.num_named_barrier, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.private_seg_size, 2064
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.uses_vcc, 1
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.has_recursion, 0
	.set _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2504
; TotalNumSgprs: 73
; NumVgprs: 34
; ScratchSize: 2064
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 73
; NumVGPRsForWavesPerEU: 34
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii,"axG",@progbits,_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii,comdat
	.protected	_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii ; -- Begin function _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii
	.globl	_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii
	.p2align	8
	.type	_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii,@function
_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii: ; @_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x4c
	s_load_b128 s[12:15], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s13, v1
	s_cbranch_execz .LBB4_13
; %bb.1:
	s_abs_i32 s2, s15
	s_abs_i32 s5, s13
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s4, 0, s2
	s_ashr_i32 s18, s15, 31
	s_load_b64 s[16:17], s[0:1], 0x38
	v_sub_nc_u32_e32 v4, 0, v1
	v_rcp_iflag_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_max_i32_e32 v4, v1, v4
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v0
	s_mul_i32 s4, s4, s3
	s_mul_hi_u32 s4, s3, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s4, s3, s4
	s_ashr_i32 s3, s13, 31
	s_mul_hi_u32 s4, s5, s4
	s_xor_b32 s7, s3, s18
	s_mul_i32 s6, s4, s2
	s_sub_i32 s5, s5, s6
	s_add_i32 s6, s4, 1
	s_sub_i32 s8, s5, s2
	s_cmp_ge_u32 s5, s2
	s_cselect_b32 s4, s6, s4
	s_cselect_b32 s5, s8, s5
	s_add_i32 s6, s4, 1
	s_cmp_ge_u32 s5, s2
	s_cselect_b32 s4, s6, s4
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s6, s16
	s_xor_b32 s4, s4, s7
	v_cvt_f32_u32_e32 v2, s6
	s_sub_i32 s4, s4, s7
	s_sub_i32 s8, 0, s6
	s_abs_i32 s5, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v2, v2
	v_cvt_f32_u32_e32 v0, s5
	s_sub_i32 s7, 0, s5
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v2, v2
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_lo_u32 v3, s7, v0
	v_readfirstlane_b32 s7, v2
	s_mul_i32 s8, s8, s7
	v_mul_hi_u32 v3, v0, v3
	s_mul_hi_u32 s8, s7, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s7, s7, s8
	s_mul_hi_u32 s7, s2, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_mul_i32 s8, s7, s6
	v_add_nc_u32_e32 v0, v0, v3
	s_sub_i32 s2, s2, s8
	s_add_i32 s8, s7, 1
	s_sub_i32 s9, s2, s6
	s_cmp_ge_u32 s2, s6
	v_mul_hi_u32 v0, v4, v0
	s_cselect_b32 s7, s8, s7
	s_cselect_b32 s2, s9, s2
	s_add_i32 s8, s7, 1
	s_cmp_ge_u32 s2, s6
	s_cselect_b32 s20, s8, s7
	s_cmp_gt_i32 s14, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s5
	s_cselect_b32 s19, -1, 0
	s_cmp_lt_i32 s14, 1
	v_sub_nc_u32_e32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v3, s5, v2
	v_cmp_le_u32_e32 vcc_lo, s5, v2
	v_cndmask_b32_e32 v2, v2, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_u32_e64 s2, s5, v2
	s_cbranch_scc1 .LBB4_4
; %bb.2:
	v_mov_b32_e32 v2, 0
	s_mov_b32 s5, 0
	s_mov_b32 s6, s14
.LBB4_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s6, s6, -1
	scratch_store_b32 off, v2, s5
	s_add_i32 s5, s5, 4
	s_cmp_eq_u32 s6, 0
	s_cbranch_scc0 .LBB4_3
.LBB4_4:
	s_cmp_lt_i32 s12, 1
	s_cbranch_scc1 .LBB4_13
; %bb.5:
	v_add_nc_u32_e32 v2, 1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
	v_xor_b32_e32 v2, s4, v1
	s_load_b256 s[4:11], s[0:1], 0x0
	v_add_nc_u32_e32 v3, 1, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v2, 31, v2
	v_cndmask_b32_e64 v0, v0, v3, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v2
	v_sub_nc_u32_e32 v2, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[4:5], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s8, v4
	v_add_co_ci_u32_e64 v7, null, s9, v5, vcc_lo
	v_add_co_u32 v8, vcc_lo, s10, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s11, v5, vcc_lo
	global_load_b32 v10, v[6:7], off
	global_load_b32 v11, v[8:9], off
	s_ashr_i32 s11, s16, 31
	v_sub_nc_u32_e32 v7, 0, v2
	s_xor_b32 s2, s18, s11
	s_mov_b32 s10, s16
	s_xor_b32 s8, s20, s2
	s_load_b64 s[20:21], s[0:1], 0x20
	s_sub_i32 s2, s8, s2
	v_max_i32_e32 v2, v2, v7
	s_abs_i32 s8, s2
	s_ashr_i32 s2, s2, 31
	v_cvt_f32_u32_e32 v0, s8
	s_sub_i32 s9, 0, s8
	v_xor_b32_e32 v3, s2, v3
	s_mov_b32 s2, s13
	s_ashr_i32 s1, s17, 31
	v_rcp_iflag_f32_e32 v0, v0
	s_mov_b32 s0, s17
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v6, s9, v0
	s_ashr_i32 s9, s14, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v6, v0, v6
	v_add_nc_u32_e32 v0, v0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[6:7], null, v2, v0, 0
	v_mul_lo_u32 v0, v7, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v2, v0
	v_add_nc_u32_e32 v2, 1, v7
	v_subrev_nc_u32_e32 v6, s8, v0
	v_cmp_le_u32_e32 vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, v7, v2, vcc_lo
	v_cndmask_b32_e32 v0, v0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v6, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s8, v0
	s_mov_b32 s8, s15
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v2, v6, vcc_lo
	v_add_co_u32 v12, vcc_lo, s6, v4
	v_add_co_ci_u32_e64 v13, null, s7, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_xor_b32_e32 v0, v0, v3
	s_lshl_b64 s[6:7], s[2:3], 2
	v_ashrrev_i32_e32 v2, 31, v1
	s_add_u32 s6, s4, s6
	s_addc_u32 s7, s5, s7
	v_sub_nc_u32_e32 v6, v0, v3
	s_lshl_b64 s[10:11], s[10:11], 2
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v7, 31, v6
	v_lshlrev_b64 v[3:4], 2, v[6:7]
	v_mad_i64_i32 v[7:8], null, s14, v6, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s10, v3
	v_add_co_ci_u32_e64 v3, null, s11, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[4:5], 2, v[7:8]
	v_mul_lo_u32 v7, v6, s9
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v14, vcc_lo, s20, v0
	v_mul_lo_u32 v8, v3, s14
	v_mad_u64_u32 v[2:3], null, v6, s14, s[6:7]
	v_add_co_ci_u32_e64 v15, null, s21, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
	v_add3_u32 v3, v8, v3, v7
	s_lshl_b64 s[6:7], s[0:1], 2
	s_mov_b32 s9, 0
	s_mov_b32 s10, 0x3e9b6dac
	s_branch .LBB4_8
.LBB4_6:                                ;   in Loop: Header=BB4_8 Depth=1
	v_mov_b32_e32 v17, 0
.LBB4_7:                                ;   in Loop: Header=BB4_8 Depth=1
	s_mul_i32 s11, s9, s3
	s_mul_hi_u32 s13, s9, s2
	s_mul_i32 s16, s9, s2
	s_add_i32 s17, s13, s11
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v17, v11, v16
	s_lshl_b64 s[16:17], s[16:17], 2
	s_add_i32 s9, s9, 1
	v_add_co_u32 v6, vcc_lo, v14, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s17, v15, vcc_lo
	v_add_co_u32 v4, vcc_lo, v4, s6
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
	v_add_co_u32 v2, vcc_lo, v2, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v3, vcc_lo
	s_cmp_eq_u32 s9, s12
	global_store_b32 v[6:7], v17, off
	s_cbranch_scc1 .LBB4_13
.LBB4_8:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB4_12 Depth 2
	s_mul_i32 s11, s9, s18
	s_mul_hi_u32 s13, s9, s8
	s_mul_i32 s16, s9, s8
	s_add_i32 s17, s13, s11
	s_mov_b32 s11, exec_lo
	s_lshl_b64 s[16:17], s[16:17], 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, v12, s16
	v_add_co_ci_u32_e64 v7, null, s17, v13, vcc_lo
	global_load_b32 v6, v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f32_e32 0x41a00000, v6
	s_cbranch_execz .LBB4_10
; %bb.9:                                ;   in Loop: Header=BB4_8 Depth=1
	v_mul_f32_e32 v7, 0x3fb8aa3b, v6
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rndne_f32_e32 v8, v7
	v_fma_f32 v9, 0x3fb8aa3b, v6, -v7
	v_sub_f32_e32 v7, v7, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v9, 0x32a5705f, v6
	v_cvt_i32_f32_e32 v8, v8
	v_add_f32_e32 v7, v7, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v7, v7
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v7, v7, v8
	v_cndmask_b32_e32 v7, 0, v7, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v8, 0x7f800000, v7, vcc_lo
	v_add_f32_e32 v9, 1.0, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[6:7], v9
	v_frexp_exp_i32_f64_e32 v6, v[6:7]
	v_frexp_mant_f32_e32 v7, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v7
	v_add_f32_e32 v7, -1.0, v9
	v_sub_f32_e32 v17, v7, v9
	v_sub_f32_e32 v7, v8, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v17, 1.0, v17
	v_add_f32_e32 v7, v7, v17
	v_subrev_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	v_cmp_neq_f32_e32 vcc_lo, 0x7f800000, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v16, 0, v6
	v_cvt_f32_i32_e32 v6, v6
	v_ldexp_f32 v9, v9, v16
	v_ldexp_f32 v7, v7, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v18, 1.0, v9
	v_dual_add_f32 v16, -1.0, v9 :: v_dual_add_f32 v17, -1.0, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v19, 1.0, v16
	v_sub_f32_e32 v17, v9, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v9, v9, v19
	v_add_f32_e32 v17, v7, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v7, v7, v9
	v_dual_add_f32 v19, v18, v17 :: v_dual_add_f32 v20, v16, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v9, v19
	v_sub_f32_e32 v18, v18, v19
	v_dual_sub_f32 v16, v16, v20 :: v_dual_add_f32 v17, v17, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f32_e32 v7, v7, v16
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v21, v20, v9
	v_mul_f32_e32 v22, v19, v21
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v18, v21, v19, -v22
	v_fmac_f32_e32 v18, v21, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v23, v22, v18
	v_sub_f32_e32 v24, v20, v23
	v_sub_f32_e32 v16, v23, v22
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v20, v20, v24
	v_sub_f32_e32 v16, v16, v18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v20, v20, v23
	v_add_f32_e32 v7, v7, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v7, v16, v7
	v_add_f32_e32 v16, v24, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v18, v9, v16
	v_dual_sub_f32 v23, v24, v16 :: v_dual_mul_f32 v20, v19, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v7, v7, v23
	v_fma_f32 v19, v18, v19, -v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v19, v18, v17
	v_add_f32_e32 v17, v20, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v22, v16, v17
	v_sub_f32_e32 v20, v17, v20
	v_sub_f32_e32 v16, v16, v22
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v16, v16, v17
	v_sub_f32_e32 v17, v20, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v7, v7, v16 :: v_dual_add_f32 v16, v21, v18
	v_add_f32_e32 v7, v17, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v17, v16, v21
	v_add_f32_e32 v7, v22, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v17, v18, v17
	v_mul_f32_e32 v7, v9, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v7, v17, v7
	v_add_f32_e32 v9, v16, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v17, v9, v9
	v_fmaak_f32 v18, s10, v17, 0x3ecc95a3
	v_mul_f32_e32 v19, v9, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fmaak_f32 v17, v17, v18, 0x3f2aaada
	v_ldexp_f32 v18, v9, 1
	v_sub_f32_e32 v9, v9, v16
	v_mul_f32_e32 v17, v19, v17
	v_mul_f32_e32 v19, 0x3f317218, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v7, v7, v9
	v_add_f32_e32 v16, v18, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f32 v7, v7, 1
	v_sub_f32_e32 v9, v16, v18
	v_fma_f32 v18, 0x3f317218, v6, -v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v9, v17, v9 :: v_dual_fmac_f32 v18, 0xb102e308, v6
	v_add_f32_e32 v6, v7, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v7, v19, v18
	v_add_f32_e32 v9, v16, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v19, v7, v19
	v_dual_add_f32 v17, v7, v9 :: v_dual_sub_f32 v16, v9, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v18, v18, v19
	v_sub_f32_e32 v20, v17, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v6, v6, v16
	v_sub_f32_e32 v21, v17, v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_sub_f32 v9, v9, v20 :: v_dual_add_f32 v16, v18, v6
	v_sub_f32_e32 v7, v7, v21
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v7, v9, v7
	v_sub_f32_e32 v9, v16, v18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v7, v16, v7
	v_sub_f32_e32 v16, v16, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v6, v6, v9 :: v_dual_add_f32 v19, v17, v7
	v_dual_sub_f32 v16, v18, v16 :: v_dual_sub_f32 v9, v19, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v6, v6, v16 :: v_dual_sub_f32 v7, v7, v9
	v_add_f32_e32 v6, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v6, v19, v6
	v_cndmask_b32_e32 v6, 0x7f800000, v6, vcc_lo
	v_cmp_gt_f32_e32 vcc_lo, 0x33800000, v8
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v6, v6, v8, vcc_lo
.LBB4_10:                               ;   in Loop: Header=BB4_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s11
	s_mul_i32 s11, s9, s1
	s_mul_hi_u32 s13, s9, s0
	s_mul_i32 s16, s9, s0
	s_add_i32 s17, s13, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[16:17], s[16:17], 2
	s_add_u32 s11, s4, s16
	s_addc_u32 s13, s5, s17
	v_add_co_u32 v7, vcc_lo, s11, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s13, v1, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s19
	global_load_b32 v16, v[7:8], off
	s_cbranch_vccnz .LBB4_6
; %bb.11:                               ;   in Loop: Header=BB4_8 Depth=1
	v_mul_f32_e32 v8, v10, v6
	s_mov_b32 s11, 0
	s_mov_b32 s13, s14
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v18, v6, v16
	v_mul_f32_e32 v7, 0x3fb8aa3b, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f32 v9, 0x3fb8aa3b, v8, -v7
	v_rndne_f32_e32 v17, v7
	v_sub_f32_e32 v7, v7, v17
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v9, 0x32a5705f, v8
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v8
	v_add_f32_e32 v7, v7, v9
	v_cvt_i32_f32_e32 v9, v17
	v_mov_b32_e32 v17, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v7, v7
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v7, v7, v9
	v_cndmask_b32_e32 v9, 0, v7, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v8
	v_dual_mov_b32 v7, v3 :: v_dual_mov_b32 v6, v2
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e32 v19, 0x7f800000, v9, vcc_lo
	v_dual_mov_b32 v9, v5 :: v_dual_mov_b32 v8, v4
	.p2align	6
.LBB4_12:                               ;   Parent Loop BB4_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b32 v20, v[8:9], off
	scratch_load_b32 v21, off, s11
	global_load_b32 v22, v[6:7], off
	v_add_co_u32 v8, vcc_lo, v8, 4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, 4
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	s_add_i32 s13, s13, -1
	s_waitcnt vmcnt(2)
	v_mul_f32_e32 v20, v18, v20
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v20, v19, v21
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v17, v20, v22
	scratch_store_b32 off, v20, s11
	s_add_i32 s11, s11, 4
	s_cmp_eq_u32 s13, 0
	s_cbranch_scc0 .LBB4_12
	s_branch .LBB4_7
.LBB4_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 1040
		.amdhsa_kernarg_size 320
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
		.amdhsa_next_free_vgpr 25
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
		.amdhsa_inst_pref_size 18
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii,"axG",@progbits,_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii,comdat
.Lfunc_end4:
	.size	_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii, .Lfunc_end4-_Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii
                                        ; -- End function
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.num_vgpr, 25
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.num_agpr, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.numbered_sgpr, 22
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.num_named_barrier, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.private_seg_size, 1040
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.uses_vcc, 1
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.uses_flat_scratch, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.has_dyn_sized_stack, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.has_recursion, 0
	.set _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2296
; TotalNumSgprs: 24
; NumVgprs: 25
; ScratchSize: 1040
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 24
; NumVGPRsForWavesPerEU: 25
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii,"axG",@progbits,_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii,comdat
	.protected	_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii ; -- Begin function _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii
	.globl	_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii
	.p2align	8
	.type	_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii,@function
_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii: ; @_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x4c
	s_load_b128 s[12:15], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[4:5], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s13, v4
	s_cbranch_execz .LBB5_13
; %bb.1:
	s_abs_i32 s2, s15
	s_abs_i32 s5, s13
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s4, 0, s2
	s_ashr_i32 s33, s15, 31
	s_load_b64 s[16:17], s[0:1], 0x38
	v_sub_nc_u32_e32 v3, 0, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_max_i32_e32 v3, v4, v3
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v0
	s_mul_i32 s4, s4, s3
	s_mul_hi_u32 s4, s3, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s4, s3, s4
	s_ashr_i32 s3, s13, 31
	s_mul_hi_u32 s4, s5, s4
	s_xor_b32 s7, s3, s33
	s_mul_i32 s6, s4, s2
	s_sub_i32 s5, s5, s6
	s_add_i32 s6, s4, 1
	s_sub_i32 s8, s5, s2
	s_cmp_ge_u32 s5, s2
	s_cselect_b32 s4, s6, s4
	s_cselect_b32 s5, s8, s5
	s_add_i32 s6, s4, 1
	s_cmp_ge_u32 s5, s2
	s_cselect_b32 s4, s6, s4
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s6, s16
	s_xor_b32 s4, s4, s7
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s4, s4, s7
	s_sub_i32 s8, 0, s6
	s_abs_i32 s5, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	v_cvt_f32_u32_e32 v0, s5
	s_sub_i32 s7, 0, s5
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_lo_u32 v2, s7, v0
	v_readfirstlane_b32 s7, v1
	s_mul_i32 s8, s8, s7
	v_mul_hi_u32 v2, v0, v2
	s_mul_hi_u32 s8, s7, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s7, s7, s8
	s_mul_hi_u32 s7, s2, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_mul_i32 s8, s7, s6
	v_add_nc_u32_e32 v0, v0, v2
	s_sub_i32 s2, s2, s8
	s_add_i32 s8, s7, 1
	s_sub_i32 s9, s2, s6
	s_cmp_ge_u32 s2, s6
	v_mul_hi_u32 v2, v3, v0
	s_cselect_b32 s7, s8, s7
	s_cselect_b32 s2, s9, s2
	s_add_i32 s8, s7, 1
	s_cmp_ge_u32 s2, s6
	s_cselect_b32 s18, s8, s7
	s_cmp_gt_i32 s14, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v2, s5
	s_cselect_b32 s62, -1, 0
	s_cmp_lt_i32 s14, 1
	v_sub_nc_u32_e32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v1, s5, v0
	v_cmp_le_u32_e32 vcc_lo, s5, v0
	v_cndmask_b32_e32 v0, v0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_u32_e64 s2, s5, v0
	s_cbranch_scc1 .LBB5_4
; %bb.2:
	v_mov_b32_e32 v0, 0
	s_mov_b32 s5, 0
	s_mov_b32 s6, s14
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v0
.LBB5_3:                                ; =>This Inner Loop Header: Depth=1
	s_add_i32 s6, s6, -1
	scratch_store_b64 off, v[0:1], s5
	s_add_i32 s5, s5, 8
	s_cmp_eq_u32 s6, 0
	s_cbranch_scc0 .LBB5_3
.LBB5_4:
	s_cmp_lt_i32 s12, 1
	s_cbranch_scc1 .LBB5_13
; %bb.5:
	v_add_nc_u32_e32 v0, 1, v2
	v_xor_b32_e32 v1, s4, v4
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x20
	s_mov_b32 s20, 0xfca7ab0c
	v_cndmask_b32_e32 v0, v2, v0, vcc_lo
	v_ashrrev_i32_e32 v1, 31, v1
	s_mov_b32 s22, 0x6a5dcb37
	s_mov_b32 s24, 0x623fde64
	s_mov_b32 s26, 0x7c89e6b0
	v_add_nc_u32_e32 v2, 1, v0
	s_mov_b32 s28, 0x14761f6e
	s_mov_b32 s30, 0x1852b7b0
	s_mov_b32 s34, 0x11122322
	s_mov_b32 s36, 0x555502a1
	v_cndmask_b32_e64 v0, v0, v2, s2
	s_mov_b32 s38, 0x55555511
	s_mov_b32 s40, 11
	s_mov_b32 s42, 0x55555555
	s_mov_b32 s44, 0x6b47b09a
	v_xor_b32_e32 v0, v0, v1
	s_mov_b32 s46, 0xbf559e2b
	s_mov_b32 s48, 0xd7f4df2e
	s_mov_b32 s50, 0x16291751
	s_mov_b32 s52, 0x9b27acf1
	v_sub_nc_u32_e32 v5, v0, v1
	s_mov_b32 s54, 0x998ef7b6
	s_mov_b32 s21, 0x3e928af3
	s_mov_b32 s23, 0x3e5ade15
	s_mov_b32 s25, 0x3ec71dee
	v_ashrrev_i32_e32 v6, 31, v5
	v_sub_nc_u32_e32 v11, 0, v5
	s_mov_b32 s27, 0x3efa0199
	s_mov_b32 s29, 0x3f2a01a0
	s_mov_b32 s31, 0x3f56c16c
	v_lshlrev_b64 v[7:8], 3, v[5:6]
	v_max_i32_e32 v5, v5, v11
	s_mov_b32 s35, 0x3f811111
	s_mov_b32 s37, 0x3fa55555
	s_mov_b32 s39, 0x3fc55555
	s_mov_b32 s41, 0x3fe00000
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s8, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s9, v8, vcc_lo
	v_add_co_u32 v2, vcc_lo, s10, v7
	v_add_co_ci_u32_e64 v3, null, s11, v8, vcc_lo
	global_load_b64 v[0:1], v[0:1], off
	global_load_b64 v[2:3], v[2:3], off
	s_ashr_i32 s9, s16, 31
	s_mov_b32 s43, 0x3fe55555
	s_xor_b32 s2, s33, s9
	s_mov_b32 s45, 0x3fc38538
	s_xor_b32 s8, s18, s2
	s_ashr_i32 s18, s14, 31
	s_sub_i32 s2, s8, s2
	s_mov_b32 s47, 0x3fc3ab76
	s_abs_i32 s8, s2
	s_ashr_i32 s2, s2, 31
	v_cvt_f32_u32_e32 v9, s8
	s_sub_i32 s10, 0, s8
	v_xor_b32_e32 v6, s2, v6
	s_mov_b32 s2, s13
	s_mov_b32 s13, s15
	v_rcp_iflag_f32_e32 v9, v9
	s_mov_b32 s15, 0
	s_mov_b32 s49, 0x3fc7474d
	s_mov_b32 s51, 0x3fcc71c0
	s_mov_b32 s53, 0x3fd24924
	s_mov_b32 s55, 0x3fd99999
	s_mov_b32 s56, 0x55555780
	s_mov_b32 s59, 0x3fe62e42
	s_mov_b32 s61, 0x3c7abc9e
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v9, 0x4f7ffffe, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v9, v9
	v_mul_lo_u32 v10, s10, v9
	s_lshl_b64 s[10:11], s[2:3], 3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v10, v9, v10
	v_add_nc_u32_e32 v11, v9, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[9:10], null, v5, v11, 0
	v_mul_lo_u32 v9, v10, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v5, v5, v9
	v_add_nc_u32_e32 v9, 1, v10
	v_subrev_nc_u32_e32 v11, s8, v5
	v_cmp_le_u32_e32 vcc_lo, s8, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v9, v10, v9, vcc_lo
	v_dual_cndmask_b32 v5, v5, v11 :: v_dual_add_nc_u32 v10, 1, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s8, v5
	s_mov_b32 s8, s16
	v_cndmask_b32_e32 v5, v9, v10, vcc_lo
	v_add_co_u32 v22, vcc_lo, s6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v23, null, s7, v8, vcc_lo
	v_xor_b32_e32 v5, v5, v6
	s_ashr_i32 s7, s17, 31
	s_add_u32 s10, s4, s10
	s_addc_u32 s11, s5, s11
	s_lshl_b64 s[8:9], s[8:9], 3
	v_sub_nc_u32_e32 v9, v5, v6
	v_ashrrev_i32_e32 v5, 31, v4
	s_mov_b32 s6, s17
	s_mov_b32 s16, 0xfefa39ef
	s_mov_b32 s17, 0xbfe62e42
	v_ashrrev_i32_e32 v10, 31, v9
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[6:7], 3, v[9:10]
	v_mad_i64_i32 v[10:11], null, s14, v9, 0
	v_add_co_u32 v12, vcc_lo, s8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v6, null, s9, v7, vcc_lo
	v_lshlrev_b64 v[8:9], 3, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_mul_lo_u32 v10, v12, s18
	v_add_co_u32 v24, vcc_lo, s0, v4
	v_mul_lo_u32 v11, v6, s14
	v_mad_u64_u32 v[6:7], null, v12, s14, s[10:11]
	v_add_co_ci_u32_e64 v25, null, s1, v5, vcc_lo
	v_add_co_u32 v8, vcc_lo, s10, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v9, null, s11, v9, vcc_lo
	v_add3_u32 v7, v11, v7, v10
	s_mov_b32 s10, 0x652b82fe
	s_mov_b32 s18, 0x3b39803f
	s_lshl_b64 s[8:9], s[6:7], 3
	s_mov_b32 s11, 0x3ff71547
	s_mov_b32 s19, 0xbc7abc9e
	s_branch .LBB5_8
.LBB5_6:                                ;   in Loop: Header=BB5_8 Depth=1
	v_mov_b32_e32 v10, 0
	v_mov_b32_e32 v11, 0
.LBB5_7:                                ;   in Loop: Header=BB5_8 Depth=1
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[2:3], v[12:13], v[10:11]
	s_mul_i32 s0, s15, s3
	s_mul_hi_u32 s1, s15, s2
	s_add_i32 s1, s1, s0
	s_mul_i32 s0, s15, s2
	s_add_i32 s15, s15, 1
	s_lshl_b64 s[0:1], s[0:1], 3
	s_cmp_eq_u32 s15, s12
	v_add_co_u32 v12, vcc_lo, v24, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v13, null, s1, v25, vcc_lo
	v_add_co_u32 v8, vcc_lo, v8, s8
	v_add_co_ci_u32_e64 v9, null, s9, v9, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s9, v7, vcc_lo
	global_store_b64 v[12:13], v[10:11], off
	s_cbranch_scc1 .LBB5_13
.LBB5_8:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB5_12 Depth 2
	s_mul_i32 s1, s15, s33
	s_mul_hi_u32 s57, s15, s13
	s_mul_i32 s0, s15, s13
	s_add_i32 s1, s57, s1
	s_mov_b32 s63, exec_lo
	s_lshl_b64 s[0:1], s[0:1], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v10, vcc_lo, v22, s0
	v_add_co_ci_u32_e64 v11, null, s1, v23, vcc_lo
	global_load_b64 v[10:11], v[10:11], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f64_e32 0x40340000, v[10:11]
	s_cbranch_execz .LBB5_10
; %bb.9:                                ;   in Loop: Header=BB5_8 Depth=1
	v_mul_f64 v[12:13], v[10:11], s[10:11]
	v_cmp_ngt_f64_e32 vcc_lo, 0xc090cc00, v[10:11]
	s_mov_b32 s57, s43
	s_mov_b32 s58, s16
	s_mov_b32 s60, s18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[12:13], v[12:13]
	v_fma_f64 v[14:15], v[12:13], s[16:17], v[10:11]
	v_cvt_i32_f64_e32 v18, v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], s[18:19], v[14:15]
	v_fma_f64 v[16:17], v[14:15], s[22:23], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[24:25]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[28:29]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[34:35]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[38:39]
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], 1.0
	v_fma_f64 v[12:13], v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[12:13], v[12:13], v18
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[12:13], 1.0
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[12:13]
	v_cmp_ngt_f64_e64 s1, -1.0, v[12:13]
	v_frexp_mant_f64_e32 v[14:15], v[10:11]
	v_frexp_exp_i32_f64_e32 v18, v[10:11]
	v_add_f64 v[16:17], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, s[42:43], v[14:15]
	v_add_f64 v[14:15], v[16:17], -v[10:11]
	v_add_f64 v[16:17], v[12:13], -v[16:17]
	v_subrev_co_ci_u32_e64 v38, null, 0, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], 1.0
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[12:13]
	v_sub_nc_u32_e32 v20, 0, v38
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[10:11], v[10:11], v20
	v_add_f64 v[14:15], v[16:17], v[14:15]
	s_and_b32 vcc_lo, vcc_lo, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[10:11], 1.0
	v_add_f64 v[28:29], v[10:11], -1.0
	v_ldexp_f64 v[14:15], v[14:15], v20
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[18:19], -1.0
	v_add_f64 v[30:31], v[28:29], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], v[16:17]
	v_add_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[18:19], v[16:17]
	v_add_f64 v[30:31], v[28:29], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[26:27], v[20:21]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	v_add_f64 v[28:29], v[30:31], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[32:33], -v[20:21], v[26:27], 1.0
	v_add_f64 v[10:11], v[10:11], -v[28:29]
	v_fma_f64 v[26:27], v[32:33], v[26:27], v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[20:21], v[26:27], 1.0
	v_fma_f64 v[14:15], v[14:15], v[26:27], v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[26:27], v[30:31], v[14:15]
	v_mul_f64 v[32:33], v[20:21], v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[26:27], v[20:21], -v[32:33]
	v_fma_f64 v[18:19], v[26:27], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[34:35], v[32:33], v[18:19]
	v_add_f64 v[36:37], v[30:31], -v[34:35]
	v_add_f64 v[28:29], v[34:35], -v[32:33]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[30:31], v[30:31], -v[36:37]
	v_add_f64 v[18:19], v[28:29], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[30:31], v[30:31], -v[34:35]
	v_add_f64 v[10:11], v[10:11], v[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[18:19], v[10:11]
	v_add_f64 v[18:19], v[36:37], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[28:29], v[14:15], v[18:19]
	v_add_f64 v[34:35], v[36:37], -v[18:19]
	v_mul_f64 v[30:31], v[20:21], v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[34:35]
	v_fma_f64 v[20:21], v[28:29], v[20:21], -v[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[28:29], v[16:17], v[20:21]
	v_add_f64 v[20:21], v[30:31], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[32:33], v[18:19], -v[20:21]
	v_add_f64 v[30:31], v[20:21], -v[30:31]
	v_add_f64 v[18:19], v[18:19], -v[32:33]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[30:31], -v[16:17]
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[18:19]
	v_add_f64 v[18:19], v[26:27], v[28:29]
	v_add_f64 v[10:11], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[18:19], -v[26:27]
	v_add_f64 v[10:11], v[32:33], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[28:29], -v[16:17]
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], v[10:11]
	v_add_f64 v[14:15], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[14:15], v[14:15]
	v_fma_f64 v[20:21], v[16:17], s[46:47], s[44:45]
	v_mul_f64 v[26:27], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[48:49]
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[50:51]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[52:53]
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[54:55]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[16:17], v[20:21], s[56:57]
	v_ldexp_f64 v[20:21], v[14:15], 1
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	v_mul_f64 v[16:17], v[26:27], v[16:17]
	v_cvt_f64_i32_e32 v[26:27], v38
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[18:19], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[28:29], v[26:27], s[58:59]
	v_ldexp_f64 v[10:11], v[10:11], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[18:19], -v[20:21]
	v_fma_f64 v[20:21], v[26:27], s[58:59], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	v_fma_f64 v[16:17], v[26:27], s[60:61], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[14:15], v[28:29], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[18:19], v[10:11]
	v_add_f64 v[28:29], v[14:15], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[14:15], v[20:21]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	v_add_f64 v[16:17], v[16:17], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[30:31], v[26:27], -v[14:15]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[32:33], v[26:27], -v[30:31]
	v_add_f64 v[18:19], v[20:21], -v[30:31]
	v_add_f64 v[20:21], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[32:33]
	v_add_f64 v[14:15], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[20:21], -v[16:17]
	v_add_f64 v[14:15], v[20:21], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], -v[18:19]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	v_add_f64 v[28:29], v[26:27], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	v_add_f64 v[18:19], v[28:29], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[10:11], v[28:29], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v10, 0, v10, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[12:13]
	v_cndmask_b32_e64 v11, 0x7ff00000, v11, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v11, 0x7ff80000, v11, s1
	v_cndmask_b32_e32 v11, 0xfff00000, v11, vcc_lo
.LBB5_10:                               ;   in Loop: Header=BB5_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s63
	s_mul_i32 s1, s15, s7
	s_mul_hi_u32 s57, s15, s6
	s_mul_i32 s0, s15, s6
	s_add_i32 s1, s57, s1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[0:1], s[0:1], 3
	s_add_u32 s0, s4, s0
	s_addc_u32 s1, s5, s1
	v_add_co_u32 v12, vcc_lo, s0, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v13, null, s1, v5, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s62
	global_load_b64 v[12:13], v[12:13], off
	s_cbranch_vccnz .LBB5_6
; %bb.11:                               ;   in Loop: Header=BB5_8 Depth=1
	v_mul_f64 v[14:15], v[0:1], v[10:11]
	s_mov_b32 s1, s14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[16:17], v[14:15], s[10:11]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[14:15]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[14:15]
	v_rndne_f64_e32 v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], s[16:17], v[14:15]
	v_cvt_i32_f64_e32 v26, v[16:17]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[14:15], v[10:11], v[12:13]
	v_mov_b32_e32 v10, 0
	v_mov_b32_e32 v11, 0
	v_fma_f64 v[18:19], v[16:17], s[18:19], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], s[22:23], s[20:21]
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[26:27]
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[30:31]
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[36:37]
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[40:41]
	v_fma_f64 v[20:21], v[18:19], v[20:21], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[18:19], v[20:21], 1.0
	v_dual_mov_b32 v21, v9 :: v_dual_mov_b32 v20, v8
	v_ldexp_f64 v[18:19], v[16:17], v26
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v19, 0x7ff00000, v19, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_dual_mov_b32 v17, v7 :: v_dual_cndmask_b32 v18, 0, v18
	v_mov_b32_e32 v16, v6
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v19, 0, v19, s0
	s_mov_b32 s0, 0
	.p2align	6
.LBB5_12:                               ;   Parent Loop BB5_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[26:27], v[20:21], off
	scratch_load_b64 v[28:29], off, s0
	global_load_b64 v[30:31], v[16:17], off
	v_add_co_u32 v20, vcc_lo, v20, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v21, null, 0, v21, vcc_lo
	v_add_co_u32 v16, vcc_lo, v16, 8
	v_add_co_ci_u32_e64 v17, null, 0, v17, vcc_lo
	s_add_i32 s1, s1, -1
	s_waitcnt vmcnt(2)
	v_mul_f64 v[26:27], v[14:15], v[26:27]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[26:27], v[18:19], v[28:29], v[26:27]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[10:11], v[26:27], v[30:31], v[10:11]
	scratch_store_b64 off, v[26:27], s0
	s_add_i32 s0, s0, 8
	s_cmp_eq_u32 s1, 0
	s_cbranch_scc0 .LBB5_12
	s_branch .LBB5_7
.LBB5_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 2064
		.amdhsa_kernarg_size 320
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
		.amdhsa_next_free_vgpr 39
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
		.amdhsa_inst_pref_size 26
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii,"axG",@progbits,_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii,comdat
.Lfunc_end5:
	.size	_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii, .Lfunc_end5-_Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii
                                        ; -- End function
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.num_vgpr, 39
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.num_agpr, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.numbered_sgpr, 64
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.num_named_barrier, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.private_seg_size, 2064
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.uses_vcc, 1
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.uses_flat_scratch, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.has_dyn_sized_stack, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.has_recursion, 0
	.set _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3300
; TotalNumSgprs: 66
; NumVgprs: 39
; ScratchSize: 2064
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 66
; NumVGPRsForWavesPerEU: 39
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
	s_cbranch_scc1 .LBB6_15
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
	s_cbranch_execz .LBB6_5
; %bb.2:
	s_load_b32 s3, s[0:1], 0x3c
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s19, s3, 0xffff
.LBB6_3:                                ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_execnz .LBB6_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s17
.LBB6_5:
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
	s_cbranch_execz .LBB6_7
; %bb.6:
	v_lshrrev_b32_e32 v9, 3, v0
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v7, v7, v8
	ds_store_b32 v9, v7
.LBB6_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s18, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB6_12
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
	s_cbranch_execz .LBB6_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB6_12:
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
	s_cbranch_execz .LBB6_15
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
.LBB6_14:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_execnz .LBB6_14
.LBB6_15:
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
.Lfunc_end6:
	.size	_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii, .Lfunc_end6-_Z24ssm_group_rmsnorm_kernelIfEvPKT_S2_S2_PS0_iii
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
	s_cbranch_scc1 .LBB7_15
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
	s_cbranch_execz .LBB7_5
; %bb.2:
	s_load_b32 s3, s[0:1], 0x3c
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s20, s3, 0xffff
	.p2align	6
.LBB7_3:                                ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_execnz .LBB7_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s18
.LBB7_5:
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
	s_cbranch_execz .LBB7_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB7_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s19, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB7_12
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
	s_cbranch_execz .LBB7_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_mov_b32_e32 v3, 0
	ds_store_b64 v3, v[1:2]
.LBB7_12:
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
	s_cbranch_execz .LBB7_15
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
.LBB7_14:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_execnz .LBB7_14
.LBB7_15:
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
.Lfunc_end7:
	.size	_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii, .Lfunc_end7-_Z24ssm_group_rmsnorm_kernelIdEvPKT_S2_S2_PS0_iii
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
	s_cbranch_scc1 .LBB8_15
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
	s_cbranch_execz .LBB8_5
; %bb.2:
	s_load_b32 s2, s[0:1], 0x2c
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_mov_b32 s12, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s13, s2, 0xffff
.LBB8_3:                                ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_execnz .LBB8_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s12
.LBB8_5:
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
	s_cbranch_execz .LBB8_7
; %bb.6:
	v_lshrrev_b32_e32 v9, 3, v0
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v7, v7, v8
	ds_store_b32 v9, v7
.LBB8_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s12, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB8_12
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
	s_cbranch_execz .LBB8_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB8_12:
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
	s_cbranch_execz .LBB8_15
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
.LBB8_14:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_execnz .LBB8_14
.LBB8_15:
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
.Lfunc_end8:
	.size	_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii, .Lfunc_end8-_Z18l2norm_rows_kernelIfEvPKT_S2_PS0_ii
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
	s_cbranch_scc1 .LBB9_15
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
	s_cbranch_execz .LBB9_5
; %bb.2:
	s_load_b32 s2, s[0:1], 0x2c
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_mov_b32 s13, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s14, s2, 0xffff
	.p2align	6
.LBB9_3:                                ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_execnz .LBB9_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s13
.LBB9_5:
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
	s_cbranch_execz .LBB9_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB9_7:
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s13, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB9_12
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
	s_cbranch_execz .LBB9_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_mov_b32_e32 v3, 0
	ds_store_b64 v3, v[1:2]
.LBB9_12:
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
	s_cbranch_execz .LBB9_15
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
.LBB9_14:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_execnz .LBB9_14
.LBB9_15:
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
.Lfunc_end9:
	.size	_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii, .Lfunc_end9-_Z18l2norm_rows_kernelIdEvPKT_S2_PS0_ii
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
	.section	.text._Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,"axG",@progbits,_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,comdat
	.protected	_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_ ; -- Begin function _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
	.globl	_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
	.p2align	8
	.type	_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,@function
_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_: ; @_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
; %bb.0:
	s_load_b128 s[20:23], s[0:1], 0x38
	s_mov_b32 s3, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_gt_i32_e64 s22, v0
	s_cbranch_execz .LBB10_20
; %bb.1:
	s_load_b32 s24, s[0:1], 0x48
	v_mov_b32_e32 v1, 0
	s_mov_b32 s3, 0
	s_mov_b32 s4, s22
.LBB10_2:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s4, s4, -1
	scratch_store_b32 off, v1, s3
	s_add_i32 s3, s3, 4
	s_cmp_eq_u32 s4, 0
	s_cbranch_scc0 .LBB10_2
; %bb.3:
	s_cmp_lt_i32 s20, 1
	s_cbranch_scc1 .LBB10_20
; %bb.4:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b256 s[12:19], s[0:1], 0x0
	s_ashr_i32 s3, s2, 31
	v_dual_mov_b32 v1, 0 :: v_dual_lshlrev_b32 v4, 2, v0
	s_lshl_b64 s[0:1], s[2:3], 2
	s_mov_b32 s29, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_nc_u32_e32 v5, 0x400, v4
	s_waitcnt lgkmcnt(0)
	s_add_u32 s10, s6, s0
	s_addc_u32 s11, s7, s1
	s_ashr_i32 s6, s21, 31
	s_load_b32 s10, s[10:11], 0x0
	s_add_u32 s7, s18, s0
	s_addc_u32 s11, s19, s1
	s_add_u32 s4, s4, s0
	s_addc_u32 s5, s5, s1
	s_cmp_eq_u32 s23, 0
	s_cselect_b32 s23, -1, 0
	s_add_i32 s0, s22, -1
	s_and_b32 s25, s22, 7
	s_cmp_gt_u32 s0, 6
	s_cselect_b32 s26, -1, 0
	s_and_b32 s27, s22, -8
	s_cmp_lg_u32 s25, 0
	s_cselect_b32 s28, -1, 0
	s_branch .LBB10_6
.LBB10_5:                               ;   in Loop: Header=BB10_6 Depth=1
	v_add_co_u32 v2, vcc_lo, s8, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s9, v3, vcc_lo
	s_add_i32 s29, s29, 1
	s_cmp_eq_u32 s29, s20
	global_store_b32 v[2:3], v6, off
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB10_20
.LBB10_6:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB10_8 Depth 2
                                        ;     Child Loop BB10_12 Depth 2
                                        ;     Child Loop BB10_15 Depth 2
                                        ;     Child Loop BB10_19 Depth 2
	s_mul_i32 s1, s29, s6
	s_mul_hi_u32 s30, s29, s21
	s_mul_i32 s0, s29, s21
	s_add_i32 s1, s30, s1
	s_add_u32 s30, s0, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s30, s22, v[0:1]
	s_addc_u32 s30, s1, s3
	s_lshl_b64 s[0:1], s[0:1], 2
	v_mad_u64_u32 v[6:7], null, s30, s22, v[3:4]
	s_add_u32 s30, s7, s0
	s_addc_u32 s31, s11, s1
	v_mov_b32_e32 v3, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 2, v[2:3]
	v_add_co_u32 v6, vcc_lo, s12, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s13, v3, vcc_lo
	v_add_co_u32 v8, vcc_lo, s14, v2
	v_add_co_ci_u32_e64 v9, null, s15, v3, vcc_lo
	global_load_b32 v6, v[6:7], off
	global_load_b32 v8, v[8:9], off
	v_add_co_u32 v9, vcc_lo, s18, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s19, v3, vcc_lo
	v_cndmask_b32_e64 v7, v7, s31, s23
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v10, s24, v6
	v_cndmask_b32_e64 v6, v9, s30, s23
	s_waitcnt vmcnt(0)
	ds_store_b32 v4, v8
	ds_store_b32 v5, v10
	s_waitcnt lgkmcnt(0)
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
	global_load_b32 v6, v[6:7], off
	s_mov_b32 s30, 0
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v6, s10, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v7, 0x3fb8aa3b, v6
	v_fma_f32 v8, 0x3fb8aa3b, v6, -v7
	v_rndne_f32_e32 v9, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_sub_f32_e32 v7, v7, v9
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v6
	v_fmac_f32_e32 v8, 0x32a5705f, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v7, v7, v8
	v_cvt_i32_f32_e32 v8, v9
	v_exp_f32_e32 v7, v7
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v7, v7, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0, v7, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v6
	v_dual_mov_b32 v6, 0 :: v_dual_cndmask_b32 v7, 0x7f800000, v7
	s_and_not1_b32 vcc_lo, exec_lo, s26
	s_cbranch_vccnz .LBB10_10
; %bb.7:                                ;   in Loop: Header=BB10_6 Depth=1
	v_mov_b32_e32 v6, 0
	s_mov_b32 s31, 0
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB10_8:                               ;   Parent Loop BB10_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b128 v[8:11], off, s30
	s_add_i32 s34, s30, 16
	s_add_i32 s31, s31, 8
	scratch_load_b128 v[12:15], off, s34
	v_mov_b32_e32 v20, s30
	s_mov_b32 s33, s30
	s_add_i32 s30, s30, 32
	s_cmp_eq_u32 s27, s31
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v9, v7, v9
	ds_load_b128 v[16:19], v20
	v_mul_f32_e32 v8, v7, v8
	ds_load_b128 v[20:23], v20 offset:16
	v_mul_f32_e32 v10, v7, v10
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v12, v7, v12
	v_mul_f32_e32 v14, v7, v14
	v_mul_f32_e32 v11, v7, v11
	v_mul_f32_e32 v15, v7, v15
	v_mul_f32_e32 v13, v7, v13
	s_clause 0x1
	scratch_store_b128 off, v[8:11], s33
	scratch_store_b128 off, v[12:15], s34
	s_waitcnt lgkmcnt(1)
	v_fmac_f32_e32 v6, v8, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v9, v17
	v_fmac_f32_e32 v6, v10, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v11, v19
	s_waitcnt lgkmcnt(0)
	v_fmac_f32_e32 v6, v12, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v13, v21
	v_fmac_f32_e32 v6, v14, v22
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v6, v15, v23
	s_cbranch_scc0 .LBB10_8
; %bb.9:                                ;   in Loop: Header=BB10_6 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s30, s27
.LBB10_10:                              ;   in Loop: Header=BB10_6 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s28
	s_cbranch_vccnz .LBB10_13
; %bb.11:                               ;   in Loop: Header=BB10_6 Depth=1
	s_lshl_b32 s30, s30, 2
	s_mov_b32 s33, s25
	s_mov_b32 s31, s30
.LBB10_12:                              ;   Parent Loop BB10_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b32 v8, off, s31
	v_mov_b32_e32 v9, s30
	s_add_i32 s33, s33, -1
	s_add_i32 s30, s30, 4
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v8, v7, v8
	ds_load_b32 v9, v9
	scratch_store_b32 off, v8, s31
	s_add_i32 s31, s31, 4
	s_cmp_lg_u32 s33, 0
	s_waitcnt lgkmcnt(0)
	v_fmac_f32_e32 v6, v8, v9
	s_cbranch_scc1 .LBB10_12
.LBB10_13:                              ;   in Loop: Header=BB10_6 Depth=1
	v_add_co_u32 v7, vcc_lo, s16, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s17, v3, vcc_lo
	s_add_u32 s0, s4, s0
	s_addc_u32 s1, s5, s1
	s_and_not1_b32 vcc_lo, exec_lo, s26
	global_load_b32 v7, v[7:8], off
	global_load_b32 v8, v1, s[0:1]
	s_mov_b32 s0, 0
	s_waitcnt vmcnt(1)
	v_sub_f32_e32 v6, v7, v6
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mul_f32 v7, v8, v6 :: v_dual_mov_b32 v6, 0
	s_cbranch_vccnz .LBB10_17
; %bb.14:                               ;   in Loop: Header=BB10_6 Depth=1
	s_mov_b32 s1, 0
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB10_15:                              ;   Parent Loop BB10_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b128 v[8:11], off, s0
	s_add_i32 s31, s0, 16
	s_add_i32 s1, s1, 8
	scratch_load_b128 v[12:15], off, s31
	v_mov_b32_e32 v28, s0
	s_mov_b32 s30, s0
	s_add_i32 s0, s0, 32
	ds_load_b128 v[16:19], v28
	ds_load_b128 v[20:23], v28 offset:1024
	ds_load_b128 v[24:27], v28 offset:16
	s_cmp_eq_u32 s27, s1
	s_waitcnt vmcnt(1) lgkmcnt(2)
	v_fma_f32 v8, v7, v16, v8
	v_fma_f32 v9, v7, v17, v9
	v_fmac_f32_e32 v11, v7, v19
	ds_load_b128 v[28:31], v28 offset:1040
	v_fma_f32 v10, v7, v18, v10
	s_waitcnt vmcnt(0) lgkmcnt(1)
	v_dual_fmac_f32 v6, v8, v20 :: v_dual_fmac_f32 v15, v7, v27
	v_fma_f32 v12, v7, v24, v12
	v_fma_f32 v13, v7, v25, v13
	v_fma_f32 v14, v7, v26, v14
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v9, v21
	s_clause 0x1
	scratch_store_b128 off, v[8:11], s30
	scratch_store_b128 off, v[12:15], s31
	v_fmac_f32_e32 v6, v10, v22
	v_fmac_f32_e32 v6, v11, v23
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v12, v28
	v_fmac_f32_e32 v6, v13, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v14, v30
	v_fmac_f32_e32 v6, v15, v31
	s_cbranch_scc0 .LBB10_15
; %bb.16:                               ;   in Loop: Header=BB10_6 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s0, s27
.LBB10_17:                              ;   in Loop: Header=BB10_6 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s28
	s_cbranch_vccnz .LBB10_5
; %bb.18:                               ;   in Loop: Header=BB10_6 Depth=1
	s_lshl_b32 s0, s0, 2
	s_mov_b32 s30, s25
	s_mov_b32 s1, s0
.LBB10_19:                              ;   Parent Loop BB10_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b32 v10, off, s1
	v_mov_b32_e32 v8, s0
	s_add_i32 s30, s30, -1
	s_add_i32 s0, s0, 4
	ds_load_2addr_stride64_b32 v[8:9], v8 offset1:4
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fmac_f32_e32 v10, v7, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v6, v10, v9
	scratch_store_b32 off, v10, s1
	s_add_i32 s1, s1, 4
	s_cmp_lg_u32 s30, 0
	s_cbranch_scc1 .LBB10_19
	s_branch .LBB10_5
.LBB10_20:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
		.amdhsa_group_segment_fixed_size 2048
		.amdhsa_private_segment_fixed_size 1040
		.amdhsa_kernarg_size 76
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
		.amdhsa_next_free_vgpr 32
		.amdhsa_next_free_sgpr 35
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
	.section	.text._Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,"axG",@progbits,_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,comdat
.Lfunc_end10:
	.size	_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_, .Lfunc_end10-_Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
                                        ; -- End function
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.num_vgpr, 32
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.num_agpr, 0
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.numbered_sgpr, 35
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.num_named_barrier, 0
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.private_seg_size, 1040
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.uses_vcc, 1
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.uses_flat_scratch, 1
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.has_dyn_sized_stack, 0
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.has_recursion, 0
	.set _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1292
; TotalNumSgprs: 37
; NumVgprs: 32
; ScratchSize: 1040
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 2048 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 37
; NumVGPRsForWavesPerEU: 32
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 1
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,"axG",@progbits,_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,comdat
	.protected	_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_ ; -- Begin function _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
	.globl	_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
	.p2align	8
	.type	_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,@function
_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_: ; @_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
; %bb.0:
	s_load_b128 s[20:23], s[0:1], 0x38
	s_mov_b32 s3, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_gt_i32_e64 s22, v0
	s_cbranch_execz .LBB11_20
; %bb.1:
	v_mov_b32_e32 v1, 0
	s_mov_b32 s3, 0
	s_mov_b32 s4, s22
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v2, v1
.LBB11_2:                               ; =>This Inner Loop Header: Depth=1
	s_add_i32 s4, s4, -1
	scratch_store_b64 off, v[1:2], s3
	s_add_i32 s3, s3, 8
	s_cmp_eq_u32 s4, 0
	s_cbranch_scc0 .LBB11_2
; %bb.3:
	s_cmp_lt_i32 s20, 1
	s_cbranch_scc1 .LBB11_20
; %bb.4:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b256 s[12:19], s[0:1], 0x0
	s_ashr_i32 s3, s2, 31
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[10:11], s[0:1], 0x48
	s_lshl_b64 s[34:35], s[2:3], 3
	s_mov_b32 s33, s21
	v_dual_mov_b32 v1, 0 :: v_dual_lshlrev_b32 v8, 3, v0
	s_mov_b32 s24, 0x652b82fe
	s_mov_b32 s26, 0xfefa39ef
	s_mov_b32 s28, 0x3b39803f
	s_mov_b32 s30, 0xfca7ab0c
	v_add_nc_u32_e32 v9, 0x800, v8
	s_mov_b32 s36, 0x7c89e6b0
	s_mov_b32 s38, 0x14761f6e
	s_mov_b32 s40, 0x1852b7b0
	s_mov_b32 s42, 0x11122322
	s_mov_b32 s44, 0x555502a1
	s_mov_b32 s46, 0x55555511
	s_mov_b32 s48, 11
	s_add_u32 s6, s6, s34
	s_addc_u32 s7, s7, s35
	s_ashr_i32 s1, s21, 31
	s_add_u32 s21, s18, s34
	s_addc_u32 s53, s19, s35
	s_add_u32 s54, s4, s34
	s_addc_u32 s55, s5, s35
	s_load_b64 s[4:5], s[6:7], 0x0
	s_cmp_eq_u32 s23, 0
	s_mov_b32 s6, 0x6a5dcb37
	s_cselect_b32 s23, -1, 0
	s_add_i32 s0, s22, -1
	s_and_b32 s56, s22, 7
	s_cmp_gt_u32 s0, 6
	s_mov_b32 s34, 0x623fde64
	s_cselect_b32 s57, -1, 0
	s_and_b32 s58, s22, -8
	s_cmp_lg_u32 s56, 0
	s_mov_b32 s52, 0
	s_mov_b32 s25, 0x3ff71547
	s_mov_b32 s27, 0xbfe62e42
	s_mov_b32 s29, 0xbc7abc9e
	s_mov_b32 s31, 0x3e928af3
	s_mov_b32 s7, 0x3e5ade15
	s_mov_b32 s35, 0x3ec71dee
	s_mov_b32 s37, 0x3efa0199
	s_mov_b32 s39, 0x3f2a01a0
	s_mov_b32 s41, 0x3f56c16c
	s_mov_b32 s43, 0x3f811111
	s_mov_b32 s45, 0x3fa55555
	s_mov_b32 s47, 0x3fc55555
	s_cselect_b32 s59, -1, 0
	s_mov_b32 s49, 0x3fe00000
	s_branch .LBB11_6
.LBB11_5:                               ;   in Loop: Header=BB11_6 Depth=1
	v_add_co_u32 v2, vcc_lo, s8, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s9, v3, vcc_lo
	s_add_i32 s52, s52, 1
	s_cmp_eq_u32 s52, s20
	global_store_b64 v[2:3], v[6:7], off
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB11_20
.LBB11_6:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB11_8 Depth 2
                                        ;     Child Loop BB11_12 Depth 2
                                        ;     Child Loop BB11_15 Depth 2
                                        ;     Child Loop BB11_19 Depth 2
	s_mul_i32 s0, s52, s1
	s_mul_hi_u32 s51, s52, s33
	s_mul_i32 s50, s52, s33
	s_add_i32 s51, s51, s0
	s_add_u32 s0, s50, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s0, s22, v[0:1]
	s_addc_u32 s0, s51, s3
	s_lshl_b64 s[50:51], s[50:51], 3
	v_mad_u64_u32 v[4:5], null, s0, s22, v[3:4]
	s_add_u32 s0, s21, s50
	s_addc_u32 s60, s53, s51
	v_mov_b32_e32 v3, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	v_add_co_u32 v4, vcc_lo, s12, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s13, v3, vcc_lo
	v_add_co_u32 v6, vcc_lo, s14, v2
	v_add_co_ci_u32_e64 v7, null, s15, v3, vcc_lo
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_u32 v10, vcc_lo, s18, v2
	global_load_b64 v[6:7], v[6:7], off
	v_add_co_ci_u32_e64 v11, null, s19, v3, vcc_lo
	v_cndmask_b32_e64 v10, v10, s0, s23
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v11, v11, s60, s23
	s_waitcnt vmcnt(1) lgkmcnt(0)
	v_mul_f64 v[4:5], s[10:11], v[4:5]
	s_waitcnt vmcnt(0)
	ds_store_b64 v8, v[6:7]
	ds_store_b64 v9, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
	global_load_b64 v[4:5], v[10:11], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], s[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[6:7], v[4:5], s[24:25]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[4:5]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[4:5]
	v_rndne_f64_e32 v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[6:7], s[26:27], v[4:5]
	v_cvt_i32_f64_e32 v14, v[6:7]
	v_fma_f64 v[10:11], v[6:7], s[28:29], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], s[6:7], s[30:31]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[36:37]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[40:41]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[44:45]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[46:47]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[48:49]
	v_fma_f64 v[12:13], v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[10:11], v[12:13], 1.0
	v_ldexp_f64 v[10:11], v[6:7], v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_mov_b32 v6, 0 :: v_dual_cndmask_b32 v11, 0x7ff00000, v11
	s_and_b32 vcc_lo, s0, vcc_lo
	v_dual_mov_b32 v7, 0 :: v_dual_cndmask_b32 v4, 0, v10
	s_and_not1_b32 vcc_lo, exec_lo, s57
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v5, 0, v11, s0
	s_mov_b32 s0, 0
	s_cbranch_vccnz .LBB11_10
; %bb.7:                                ;   in Loop: Header=BB11_6 Depth=1
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0
	s_mov_b32 s60, 0
.LBB11_8:                               ;   Parent Loop BB11_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b128 v[10:13], off, s0
	s_add_i32 s62, s0, 16
	s_add_i32 s63, s0, 32
	s_clause 0x1
	scratch_load_b128 v[14:17], off, s62
	scratch_load_b128 v[18:21], off, s63
	s_add_i32 s64, s0, 48
	v_mov_b32_e32 v34, s0
	scratch_load_b128 v[22:25], off, s64
	ds_load_b128 v[26:29], v34
	ds_load_b128 v[30:33], v34 offset:16
	s_add_i32 s60, s60, 8
	s_mov_b32 s61, s0
	s_add_i32 s0, s0, 64
	s_cmp_eq_u32 s58, s60
	s_waitcnt vmcnt(3)
	v_mul_f64 v[10:11], v[4:5], v[10:11]
	v_mul_f64 v[12:13], v[4:5], v[12:13]
	s_waitcnt vmcnt(2)
	v_mul_f64 v[14:15], v[4:5], v[14:15]
	v_mul_f64 v[16:17], v[4:5], v[16:17]
	s_waitcnt vmcnt(1)
	v_mul_f64 v[18:19], v[4:5], v[18:19]
	v_mul_f64 v[20:21], v[4:5], v[20:21]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[22:23], v[4:5], v[22:23]
	v_mul_f64 v[24:25], v[4:5], v[24:25]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[6:7], v[10:11], v[26:27], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[12:13], v[28:29], v[6:7]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[6:7], v[14:15], v[30:31], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[16:17], v[32:33], v[6:7]
	ds_load_b128 v[26:29], v34 offset:32
	ds_load_b128 v[30:33], v34 offset:48
	s_clause 0x3
	scratch_store_b128 off, v[10:13], s61
	scratch_store_b128 off, v[14:17], s62
	scratch_store_b128 off, v[18:21], s63
	scratch_store_b128 off, v[22:25], s64
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[6:7], v[18:19], v[26:27], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[20:21], v[28:29], v[6:7]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[6:7], v[22:23], v[30:31], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[24:25], v[32:33], v[6:7]
	s_cbranch_scc0 .LBB11_8
; %bb.9:                                ;   in Loop: Header=BB11_6 Depth=1
	s_mov_b32 s0, s58
.LBB11_10:                              ;   in Loop: Header=BB11_6 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s59
	s_cbranch_vccnz .LBB11_13
; %bb.11:                               ;   in Loop: Header=BB11_6 Depth=1
	s_lshl_b32 s0, s0, 3
	s_mov_b32 s61, s56
	s_mov_b32 s60, s0
.LBB11_12:                              ;   Parent Loop BB11_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b64 v[10:11], off, s60
	v_mov_b32_e32 v12, s0
	s_add_i32 s61, s61, -1
	s_add_i32 s0, s0, 8
	ds_load_b64 v[12:13], v12
	s_waitcnt vmcnt(0)
	v_mul_f64 v[10:11], v[4:5], v[10:11]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[10:11], v[12:13], v[6:7]
	scratch_store_b64 off, v[10:11], s60
	s_add_i32 s60, s60, 8
	s_cmp_lg_u32 s61, 0
	s_cbranch_scc1 .LBB11_12
.LBB11_13:                              ;   in Loop: Header=BB11_6 Depth=1
	v_add_co_u32 v4, vcc_lo, s16, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s17, v3, vcc_lo
	s_add_u32 s50, s54, s50
	s_addc_u32 s51, s55, s51
	s_mov_b32 s0, 0
	global_load_b64 v[4:5], v[4:5], off
	global_load_b64 v[10:11], v1, s[50:51]
	s_and_not1_b32 vcc_lo, exec_lo, s57
	s_waitcnt vmcnt(1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[4:5], v[10:11], v[4:5]
	s_cbranch_vccnz .LBB11_17
; %bb.14:                               ;   in Loop: Header=BB11_6 Depth=1
	s_mov_b32 s50, 0
.LBB11_15:                              ;   Parent Loop BB11_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b128 v[10:13], off, s0
	s_add_i32 s60, s0, 16
	s_add_i32 s61, s0, 32
	s_clause 0x1
	scratch_load_b128 v[14:17], off, s60
	scratch_load_b128 v[18:21], off, s61
	s_add_i32 s62, s0, 48
	v_mov_b32_e32 v42, s0
	scratch_load_b128 v[22:25], off, s62
	ds_load_b128 v[26:29], v42
	ds_load_b128 v[30:33], v42 offset:16
	ds_load_b128 v[34:37], v42 offset:2048
	ds_load_b128 v[38:41], v42 offset:2064
	s_add_i32 s50, s50, 8
	s_mov_b32 s51, s0
	s_add_i32 s0, s0, 64
	s_cmp_eq_u32 s58, s50
	s_waitcnt vmcnt(3) lgkmcnt(3)
	v_fma_f64 v[10:11], v[4:5], v[26:27], v[10:11]
	v_fma_f64 v[12:13], v[4:5], v[28:29], v[12:13]
	s_waitcnt vmcnt(2) lgkmcnt(2)
	v_fma_f64 v[14:15], v[4:5], v[30:31], v[14:15]
	v_fma_f64 v[16:17], v[4:5], v[32:33], v[16:17]
	ds_load_b128 v[26:29], v42 offset:32
	ds_load_b128 v[30:33], v42 offset:48
	s_waitcnt vmcnt(1) lgkmcnt(1)
	v_fma_f64 v[18:19], v[4:5], v[26:27], v[18:19]
	v_fma_f64 v[20:21], v[4:5], v[28:29], v[20:21]
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[22:23], v[4:5], v[30:31], v[22:23]
	v_fma_f64 v[24:25], v[4:5], v[32:33], v[24:25]
	v_fma_f64 v[6:7], v[10:11], v[34:35], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[12:13], v[36:37], v[6:7]
	v_fma_f64 v[6:7], v[14:15], v[38:39], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[16:17], v[40:41], v[6:7]
	ds_load_b128 v[34:37], v42 offset:2080
	ds_load_b128 v[38:41], v42 offset:2096
	s_clause 0x3
	scratch_store_b128 off, v[10:13], s51
	scratch_store_b128 off, v[14:17], s60
	scratch_store_b128 off, v[18:21], s61
	scratch_store_b128 off, v[22:25], s62
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[6:7], v[18:19], v[34:35], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[20:21], v[36:37], v[6:7]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[6:7], v[22:23], v[38:39], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[24:25], v[40:41], v[6:7]
	s_cbranch_scc0 .LBB11_15
; %bb.16:                               ;   in Loop: Header=BB11_6 Depth=1
	s_mov_b32 s0, s58
.LBB11_17:                              ;   in Loop: Header=BB11_6 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s59
	s_cbranch_vccnz .LBB11_5
; %bb.18:                               ;   in Loop: Header=BB11_6 Depth=1
	s_lshl_b32 s0, s0, 3
	s_mov_b32 s51, s56
	s_mov_b32 s50, s0
	.p2align	6
.LBB11_19:                              ;   Parent Loop BB11_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	scratch_load_b64 v[14:15], off, s50
	v_mov_b32_e32 v10, s0
	s_add_i32 s51, s51, -1
	s_add_i32 s0, s0, 8
	ds_load_2addr_stride64_b64 v[10:13], v10 offset1:4
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[10:11], v[4:5], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[10:11], v[12:13], v[6:7]
	scratch_store_b64 off, v[10:11], s50
	s_add_i32 s50, s50, 8
	s_cmp_lg_u32 s51, 0
	s_cbranch_scc1 .LBB11_19
	s_branch .LBB11_5
.LBB11_20:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
		.amdhsa_group_segment_fixed_size 4096
		.amdhsa_private_segment_fixed_size 2064
		.amdhsa_kernarg_size 80
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
		.amdhsa_next_free_vgpr 43
		.amdhsa_next_free_sgpr 65
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
	.section	.text._Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,"axG",@progbits,_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_,comdat
.Lfunc_end11:
	.size	_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_, .Lfunc_end11-_Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
                                        ; -- End function
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.num_vgpr, 43
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.num_agpr, 0
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.numbered_sgpr, 65
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.num_named_barrier, 0
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.private_seg_size, 2064
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.uses_vcc, 1
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.uses_flat_scratch, 1
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.has_dyn_sized_stack, 0
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.has_recursion, 0
	.set _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1912
; TotalNumSgprs: 67
; NumVgprs: 43
; ScratchSize: 2064
; MemoryBound: 1
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 4096 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 67
; NumVGPRsForWavesPerEU: 43
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
	s_cbranch_execz .LBB12_6
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
	s_cbranch_execz .LBB12_3
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
.LBB12_3:
	s_and_not1_saveexec_b32 s0, s1
	s_cbranch_execz .LBB12_5
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
.LBB12_5:
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
.LBB12_6:
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
.Lfunc_end12:
	.size	_Z16row_scale_kernelIfEvPKT_S2_PS0_ii, .Lfunc_end12-_Z16row_scale_kernelIfEvPKT_S2_PS0_ii
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
	s_cbranch_execz .LBB13_6
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
	s_cbranch_execz .LBB13_3
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
.LBB13_3:
	s_and_not1_saveexec_b32 s0, s1
	s_cbranch_execz .LBB13_5
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
.LBB13_5:
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
.LBB13_6:
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
.Lfunc_end13:
	.size	_Z16row_scale_kernelIdEvPKT_S2_PS0_ii, .Lfunc_end13-_Z16row_scale_kernelIdEvPKT_S2_PS0_ii
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
	.type	__hip_cuid_f3d4056ec3248fb2,@object ; @__hip_cuid_f3d4056ec3248fb2
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_f3d4056ec3248fb2
__hip_cuid_f3d4056ec3248fb2:
	.byte	0                               ; 0x0
	.size	__hip_cuid_f3d4056ec3248fb2, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_f3d4056ec3248fb2
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
    .name:           _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z27ssm_conv_causal_silu_kernelIfEvPKT_S2_S2_PS0_iiii.kd
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
    .name:           _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z27ssm_conv_causal_silu_kernelIdEvPKT_S2_S2_PS0_iiii.kd
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
    .name:           _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii
    .private_segment_fixed_size: 1040
    .sgpr_count:     28
    .sgpr_spill_count: 0
    .symbol:         _Z22ssm_scan_mamba1_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     21
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
    .name:           _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii
    .private_segment_fixed_size: 2064
    .sgpr_count:     73
    .sgpr_spill_count: 0
    .symbol:         _Z22ssm_scan_mamba1_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     34
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
      - .offset:         64
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         68
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         76
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         78
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         80
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         82
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         84
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         86
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         128
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 320
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii
    .private_segment_fixed_size: 1040
    .sgpr_count:     24
    .sgpr_spill_count: 0
    .symbol:         _Z22ssm_scan_mamba2_kernelIfEvPKT_S2_S2_S2_PS0_iiiiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     25
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
      - .offset:         64
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         68
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         76
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         78
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         80
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         82
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         84
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         86
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         128
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 320
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii
    .private_segment_fixed_size: 2064
    .sgpr_count:     66
    .sgpr_spill_count: 0
    .symbol:         _Z22ssm_scan_mamba2_kernelIdEvPKT_S2_S2_S2_PS0_iiiiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     39
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
    .group_segment_fixed_size: 2048
    .kernarg_segment_align: 8
    .kernarg_segment_size: 76
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
    .private_segment_fixed_size: 1040
    .sgpr_count:     37
    .sgpr_spill_count: 0
    .symbol:         _Z23gated_delta_scan_kernelIfEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.kd
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
    .group_segment_fixed_size: 4096
    .kernarg_segment_align: 8
    .kernarg_segment_size: 80
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_
    .private_segment_fixed_size: 2064
    .sgpr_count:     67
    .sgpr_spill_count: 0
    .symbol:         _Z23gated_delta_scan_kernelIdEvPKT_S2_S2_S2_S2_S2_PS0_iiiiS0_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     43
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
