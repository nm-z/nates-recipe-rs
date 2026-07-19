	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii ; -- Begin function _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii
	.globl	_Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii
	.p2align	8
	.type	_Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii,@function
_Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii: ; @_Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x54
	s_load_b128 s[20:23], s[0:1], 0x38
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s21, s20
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_9
; %bb.1:
	s_abs_i32 s2, s21
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b256 s[4:11], s[0:1], 0x0
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
	v_xor_b32_e32 v3, s21, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v3, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s10, v4
	v_add_co_ci_u32_e64 v7, null, s11, v5, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[6:7]
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB0_9
; %bb.2:
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b256 s[12:19], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[6:7], 3, v[1:2]
	v_add_co_u32 v6, vcc_lo, s4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s5, v7, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v0, v[6:7]
	v_cmp_lt_i32_e32 vcc_lo, -1, v0
	v_cmp_gt_i32_e64 s0, s22, v0
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 exec_lo, exec_lo, s0
	s_cbranch_execz .LBB0_9
; %bb.3:
	v_mul_lo_u32 v2, v3, s21
	s_mov_b32 s0, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v3, v1, v2
	v_mad_u64_u32 v[1:2], null, v3, s22, v[0:1]
	v_add_co_u32 v0, vcc_lo, s6, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[6:7], 3, v[1:2]
	v_add_co_ci_u32_e64 v1, null, s7, v5, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, s12, v6
	v_add_co_ci_u32_e64 v9, null, s13, v7, vcc_lo
	global_load_b64 v[10:11], v[0:1], off
	global_load_b64 v[2:3], v[8:9], off
.LBB0_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[10:11]
	global_atomic_cmpswap_b64 v[0:1], v[8:9], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB0_4
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s8, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s9, v5, vcc_lo
	v_add_co_u32 v4, vcc_lo, s14, v6
	v_add_co_ci_u32_e64 v5, null, s15, v7, vcc_lo
	global_load_b64 v[8:9], v[0:1], off
	global_load_b64 v[2:3], v[4:5], off
	s_mov_b32 s0, 0
.LBB0_6:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[8:9]
	global_atomic_cmpswap_b64 v[0:1], v[4:5], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB0_6
; %bb.7:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v4, vcc_lo, s16, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s17, v7, vcc_lo
	s_mov_b32 s0, 0
	global_load_b64 v[2:3], v[4:5], off
.LBB0_8:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], 1.0
	global_atomic_cmpswap_b64 v[0:1], v[4:5], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB0_8
.LBB0_9:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii
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
		.amdhsa_next_free_vgpr 12
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
.Lfunc_end0:
	.size	_Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii, .Lfunc_end0-_Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii
                                        ; -- End function
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.num_vgpr, 12
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.num_agpr, 0
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.numbered_sgpr, 24
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.num_named_barrier, 0
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.private_seg_size, 0
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.uses_vcc, 1
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.uses_flat_scratch, 0
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.has_dyn_sized_stack, 0
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.has_recursion, 0
	.set _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 740
; TotalNumSgprs: 26
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 26
; NumVGPRsForWavesPerEU: 12
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
	.protected	_Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_ ; -- Begin function _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_
	.globl	_Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_
	.p2align	8
	.type	_Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_,@function
_Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_: ; @_Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b64 s[12:13], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s12, v1
	s_cbranch_execz .LBB1_13
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_mul_lo_u32 v2, v1, s13
	s_load_b128 s[0:3], s[0:1], 0x28
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0
	s_cmp_lt_i32 s13, 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[4:5], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v4
	v_add_co_ci_u32_e64 v3, null, s5, v5, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
	s_cbranch_scc1 .LBB1_10
; %bb.2:
	v_dual_mov_b32 v8, 0 :: v_dual_mov_b32 v13, v3
	v_dual_mov_b32 v11, v5 :: v_dual_mov_b32 v10, v4
	v_dual_mov_b32 v9, 0 :: v_dual_mov_b32 v12, v2
	s_mov_b32 s4, s13
	.p2align	6
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[14:15], v[12:13], off
	global_load_b64 v[16:17], v[10:11], off
	v_add_co_u32 v12, vcc_lo, v12, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v13, null, 0, v13, vcc_lo
	v_add_co_u32 v10, vcc_lo, v10, 8
	v_add_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	s_add_i32 s4, s4, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s4, 0
	s_waitcnt vmcnt(1)
	v_add_f64 v[6:7], v[6:7], v[14:15]
	s_waitcnt vmcnt(0)
	v_add_f64 v[8:9], v[8:9], v[16:17]
	s_cbranch_scc0 .LBB1_3
; %bb.4:
	s_load_b64 s[4:5], s[0:1], 0x0
	s_load_b64 s[2:3], s[2:3], 0x0
	s_cmp_lt_i32 s13, 2
	s_cbranch_scc1 .LBB1_11
.LBB1_5:
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[6:7], v[6:7]
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[12:13], s[4:5], v[8:9]
	s_add_i32 s1, s13, -1
	s_mov_b32 s6, 0
	v_mov_b32_e32 v0, -1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[14:15], null, v[12:13], v[12:13], v[10:11]
	v_rcp_f64_e32 v[16:17], v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	v_div_scale_f64 v[18:19], vcc_lo, v[10:11], v[12:13], v[10:11]
	v_mul_f64 v[20:21], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[14:15], v[20:21], v[18:19]
	v_div_fmas_f64 v[14:15], v[14:15], v[16:17], v[20:21]
	v_mov_b32_e32 v16, 0
	v_mov_b32_e32 v17, 0
	s_delay_alu instid0(VALU_DEP_3)
	v_div_fixup_f64 v[12:13], v[14:15], v[12:13], v[10:11]
	v_mov_b32_e32 v14, 0
	v_dual_mov_b32 v15, 0 :: v_dual_mov_b32 v10, 0x85ebc8a0
	v_mov_b32_e32 v11, 0xffe1ccf3
	s_branch .LBB1_7
.LBB1_6:                                ;   in Loop: Header=BB1_7 Depth=1
	s_or_b32 exec_lo, exec_lo, s7
	v_add_co_u32 v2, vcc_lo, v2, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	v_add_co_u32 v4, vcc_lo, v4, 8
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_add_i32 s6, s6, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s1, s6
	s_cbranch_scc1 .LBB1_9
.LBB1_7:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[18:19], v[4:5], off
	global_load_b64 v[20:21], v[2:3], off
	s_mov_b32 s7, exec_lo
	s_waitcnt vmcnt(1)
	v_add_f64 v[14:15], v[14:15], v[18:19]
	s_waitcnt vmcnt(0)
	v_add_f64 v[16:17], v[16:17], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[8:9], -v[14:15]
	v_min_f64 v[22:23], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ngt_f64_e32 s[2:3], v[22:23]
	s_cbranch_execz .LBB1_6
; %bb.8:                                ;   in Loop: Header=BB1_7 Depth=1
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[6:7], -v[16:17]
	v_mul_f64 v[22:23], v[16:17], v[16:17]
	v_add_f64 v[24:25], s[4:5], v[14:15]
	v_add_f64 v[18:19], s[4:5], v[18:19]
	v_mul_f64 v[20:21], v[20:21], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_div_scale_f64 v[26:27], null, v[24:25], v[24:25], v[22:23]
	v_div_scale_f64 v[38:39], vcc_lo, v[22:23], v[24:25], v[22:23]
	v_div_scale_f64 v[28:29], null, v[18:19], v[18:19], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[30:31], v[26:27]
	v_rcp_f64_e32 v[32:33], v[28:29]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[34:35], -v[26:27], v[30:31], 1.0
	v_fma_f64 v[36:37], -v[28:29], v[32:33], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[30:31], v[30:31], v[34:35], v[30:31]
	v_fma_f64 v[32:33], v[32:33], v[36:37], v[32:33]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[34:35], -v[26:27], v[30:31], 1.0
	v_fma_f64 v[36:37], -v[28:29], v[32:33], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[30:31], v[30:31], v[34:35], v[30:31]
	v_div_scale_f64 v[34:35], s0, v[20:21], v[18:19], v[20:21]
	v_fma_f64 v[32:33], v[32:33], v[36:37], v[32:33]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[36:37], v[38:39], v[30:31]
	v_mul_f64 v[40:41], v[34:35], v[32:33]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], -v[26:27], v[36:37], v[38:39]
	v_fma_f64 v[28:29], -v[28:29], v[40:41], v[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[26:27], v[26:27], v[30:31], v[36:37]
	s_mov_b32 vcc_lo, s0
	v_div_fmas_f64 v[28:29], v[28:29], v[32:33], v[40:41]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fixup_f64 v[22:23], v[26:27], v[24:25], v[22:23]
	v_div_fixup_f64 v[18:19], v[28:29], v[18:19], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[22:23], v[18:19]
	v_add_f64 v[18:19], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, v[18:19], v[10:11]
	v_dual_cndmask_b32 v11, v11, v19 :: v_dual_cndmask_b32 v10, v10, v18
	v_cndmask_b32_e64 v0, v0, s6, vcc_lo
	s_branch .LBB1_6
.LBB1_9:
	v_cvt_f64_i32_e32 v[3:4], v0
	s_branch .LBB1_12
.LBB1_10:
	v_mov_b32_e32 v8, 0
	v_mov_b32_e32 v9, 0
	s_load_b64 s[4:5], s[0:1], 0x0
	s_load_b64 s[2:3], s[2:3], 0x0
	s_cmp_lt_i32 s13, 2
	s_cbranch_scc0 .LBB1_5
.LBB1_11:
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v10, 0x85ebc8a0
	v_mov_b32_e32 v4, 0xbff00000
	v_mov_b32_e32 v11, 0xffe1ccf3
.LBB1_12:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v5, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s9, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s10, v0
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	global_store_b64 v[5:6], v[10:11], off
	global_store_b64 v[0:1], v[3:4], off
.LBB1_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_
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
		.amdhsa_next_free_vgpr 42
		.amdhsa_next_free_sgpr 14
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
.Lfunc_end1:
	.size	_Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_, .Lfunc_end1-_Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_
                                        ; -- End function
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.num_vgpr, 42
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.num_agpr, 0
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.numbered_sgpr, 14
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.num_named_barrier, 0
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.private_seg_size, 0
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.uses_vcc, 1
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.uses_flat_scratch, 0
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.has_dyn_sized_stack, 0
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.has_recursion, 0
	.set _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1060
; TotalNumSgprs: 16
; NumVgprs: 42
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 16
; NumVGPRsForWavesPerEU: 42
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
	.protected	_Z21data_partition_kernelPKdS0_PdS1_iiii ; -- Begin function _Z21data_partition_kernelPKdS0_PdS1_iiii
	.globl	_Z21data_partition_kernelPKdS0_PdS1_iiii
	.p2align	8
	.type	_Z21data_partition_kernelPKdS0_PdS1_iiii,@function
_Z21data_partition_kernelPKdS0_PdS1_iiii: ; @_Z21data_partition_kernelPKdS0_PdS1_iiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[8:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[4:5], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v4
	s_cbranch_execz .LBB2_6
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v5, 31, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
                                        ; implicit-def: $vgpr2_vgpr3
	s_and_saveexec_b32 s2, vcc_lo
	s_xor_b32 s2, exec_lo, s2
	s_cbranch_execz .LBB2_3
; %bb.2:
	v_mad_u64_u32 v[2:3], null, v4, s9, s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v2
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	v_add_co_u32 v4, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v3, v[2:3]
	v_mov_b32_e32 v2, 0
	v_cmp_lt_i32_e32 vcc_lo, s11, v3
	v_cndmask_b32_e64 v3, 0x3ff00000, 0, vcc_lo
	global_store_b64 v[4:5], v[2:3], off
	v_cndmask_b32_e64 v3, 0, 0x3ff00000, vcc_lo
.LBB2_3:
	s_and_not1_saveexec_b32 s0, s2
	s_cbranch_execz .LBB2_5
; %bb.4:
	v_mov_b32_e32 v4, 0
	v_add_co_u32 v6, vcc_lo, s4, v0
	v_mov_b32_e32 v2, 0
	v_add_co_ci_u32_e64 v7, null, s5, v1, vcc_lo
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v5, v4
	global_store_b64 v[6:7], v[4:5], off
.LBB2_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB2_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21data_partition_kernelPKdS0_PdS1_iiii
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
.Lfunc_end2:
	.size	_Z21data_partition_kernelPKdS0_PdS1_iiii, .Lfunc_end2-_Z21data_partition_kernelPKdS0_PdS1_iiii
                                        ; -- End function
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.num_vgpr, 8
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.num_agpr, 0
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.numbered_sgpr, 12
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.num_named_barrier, 0
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.private_seg_size, 0
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.uses_vcc, 1
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.uses_flat_scratch, 0
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.has_dyn_sized_stack, 0
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.has_recursion, 0
	.set _Z21data_partition_kernelPKdS0_PdS1_iiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 340
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
	.protected	_Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii ; -- Begin function _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii
	.globl	_Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii
	.p2align	8
	.type	_Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii,@function
_Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii: ; @_Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x4c
	s_load_b128 s[12:15], s[0:1], 0x30
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s13, s12
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB3_7
; %bb.1:
	s_abs_i32 s2, s13
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b256 s[4:11], s[0:1], 0x0
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
	v_xor_b32_e32 v3, s13, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v3, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[5:6], 2, v[3:4]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s10, v5
	v_add_co_ci_u32_e64 v6, null, s11, v6, vcc_lo
	global_load_b32 v5, v[5:6], off
	s_waitcnt vmcnt(0)
	v_cmp_le_i32_e32 vcc_lo, s15, v5
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB3_7
; %bb.2:
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b128 s[16:19], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[6:7], 3, v[1:2]
	v_add_co_u32 v6, vcc_lo, s4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s5, v7, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v0, v[6:7]
	v_cmp_lt_i32_e32 vcc_lo, -1, v0
	v_cmp_gt_i32_e64 s0, s14, v0
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 exec_lo, exec_lo, s0
	s_cbranch_execz .LBB3_7
; %bb.3:
	v_mul_lo_u32 v2, v3, s13
	v_subrev_nc_u32_e32 v7, s15, v5
	s_mov_b32 s0, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v1, v1, v2
	v_mad_u64_u32 v[5:6], null, v7, s13, v[1:2]
	v_lshlrev_b64 v[6:7], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[1:2], null, v5, s14, v[0:1]
	v_add_co_u32 v0, vcc_lo, s6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 3, v[1:2]
	v_add_co_ci_u32_e64 v1, null, s7, v7, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, s16, v4
	v_add_co_ci_u32_e64 v9, null, s17, v5, vcc_lo
	global_load_b64 v[10:11], v[0:1], off
	global_load_b64 v[2:3], v[8:9], off
.LBB3_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[10:11]
	global_atomic_cmpswap_b64 v[0:1], v[8:9], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB3_4
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s9, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, s18, v4
	v_add_co_ci_u32_e64 v5, null, s19, v5, vcc_lo
	global_load_b64 v[6:7], v[0:1], off
	global_load_b64 v[2:3], v[4:5], off
	s_mov_b32 s0, 0
.LBB3_6:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[6:7]
	global_atomic_cmpswap_b64 v[0:1], v[4:5], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB3_6
.LBB3_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 320
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
		.amdhsa_next_free_vgpr 12
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
.Lfunc_end3:
	.size	_Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii, .Lfunc_end3-_Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii
                                        ; -- End function
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.num_vgpr, 12
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.num_agpr, 0
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.numbered_sgpr, 20
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.num_named_barrier, 0
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.private_seg_size, 0
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.uses_vcc, 1
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.uses_flat_scratch, 0
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.has_dyn_sized_stack, 0
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.has_recursion, 0
	.set _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 676
; TotalNumSgprs: 22
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 22
; NumVGPRsForWavesPerEU: 12
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
	.protected	_Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i ; -- Begin function _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i
	.globl	_Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i
	.p2align	8
	.type	_Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i,@function
_Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i: ; @_Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x54
	s_load_b128 s[12:15], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s12, v1
	s_cbranch_execz .LBB4_14
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	s_cmp_lt_i32 s13, 1
	s_cbranch_scc1 .LBB4_12
; %bb.2:
	s_load_b128 s[16:19], s[0:1], 0x30
	v_mul_lo_u32 v0, s14, v1
	s_cmp_gt_i32 s14, 0
	v_mov_b32_e32 v20, -1
	s_cselect_b32 s3, -1, 0
	s_cmp_gt_i32 s14, 1
	s_mov_b32 s12, 0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[16:17], s[16:17], 0x0
	s_load_b64 s[18:19], s[18:19], 0x0
	v_mul_lo_u32 v2, v0, s13
	v_mov_b32_e32 v0, -1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v21, vcc_lo, s4, v2
	v_add_co_ci_u32_e64 v22, null, s5, v3, vcc_lo
	v_add_co_u32 v23, vcc_lo, s6, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v24, null, s7, v3, vcc_lo
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_cselect_b32 s6, -1, 0
	s_mov_b32 s4, 0
	s_add_i32 s7, s14, -1
	s_branch .LBB4_4
.LBB4_3:                                ;   in Loop: Header=BB4_4 Depth=1
	s_add_i32 s12, s12, 1
	s_add_i32 s4, s4, s14
	s_cmp_eq_u32 s12, s13
	s_cbranch_scc1 .LBB4_13
.LBB4_4:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB4_6 Depth 2
                                        ;     Child Loop BB4_10 Depth 2
	s_ashr_i32 s5, s4, 31
	v_mov_b32_e32 v6, 0
	s_lshl_b64 s[20:21], s[4:5], 3
	v_dual_mov_b32 v7, 0 :: v_dual_mov_b32 v10, 0
	v_add_co_u32 v4, vcc_lo, v21, s20
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s21, v22, vcc_lo
	v_add_co_u32 v8, vcc_lo, v23, s20
	v_mov_b32_e32 v11, 0
	v_add_co_ci_u32_e64 v9, null, s21, v24, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccnz .LBB4_7
; %bb.5:                                ;   in Loop: Header=BB4_4 Depth=1
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v13, v9
	v_dual_mov_b32 v10, 0 :: v_dual_mov_b32 v15, v5
	v_dual_mov_b32 v7, 0 :: v_dual_mov_b32 v12, v8
	v_dual_mov_b32 v11, 0 :: v_dual_mov_b32 v14, v4
	s_mov_b32 s2, s14
	.p2align	6
.LBB4_6:                                ;   Parent Loop BB4_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[16:17], v[14:15], off
	global_load_b64 v[18:19], v[12:13], off
	v_add_co_u32 v14, vcc_lo, v14, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v15, null, 0, v15, vcc_lo
	v_add_co_u32 v12, vcc_lo, v12, 8
	v_add_co_ci_u32_e64 v13, null, 0, v13, vcc_lo
	s_add_i32 s2, s2, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s2, 0
	s_waitcnt vmcnt(1)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	s_waitcnt vmcnt(0)
	v_add_f64 v[6:7], v[6:7], v[18:19]
	s_cbranch_scc0 .LBB4_6
.LBB4_7:                                ;   in Loop: Header=BB4_4 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccnz .LBB4_3
; %bb.8:                                ;   in Loop: Header=BB4_4 Depth=1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[12:13], v[10:11], v[10:11]
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[14:15], s[16:17], v[6:7]
	s_mov_b32 s5, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[16:17], null, v[14:15], v[14:15], v[12:13]
	v_rcp_f64_e32 v[18:19], v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[25:26], -v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[18:19], v[25:26], v[18:19]
	v_fma_f64 v[25:26], -v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[18:19], v[25:26], v[18:19]
	v_div_scale_f64 v[25:26], vcc_lo, v[12:13], v[14:15], v[12:13]
	v_mul_f64 v[27:28], v[25:26], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[16:17], v[27:28], v[25:26]
	v_div_fmas_f64 v[16:17], v[16:17], v[18:19], v[27:28]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[12:13], v[16:17], v[14:15], v[12:13]
	v_mov_b32_e32 v14, 0
	v_dual_mov_b32 v15, 0 :: v_dual_mov_b32 v16, 0
	v_mov_b32_e32 v17, 0
	s_branch .LBB4_10
.LBB4_9:                                ;   in Loop: Header=BB4_10 Depth=2
	s_or_b32 exec_lo, exec_lo, s15
	v_add_co_u32 v4, vcc_lo, v4, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	v_add_co_u32 v8, vcc_lo, v8, 8
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	s_add_i32 s5, s5, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s7, s5
	s_cbranch_scc1 .LBB4_3
.LBB4_10:                               ;   Parent Loop BB4_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[18:19], v[8:9], off
	global_load_b64 v[25:26], v[4:5], off
	s_mov_b32 s15, exec_lo
	s_waitcnt vmcnt(1)
	v_add_f64 v[14:15], v[14:15], v[18:19]
	s_waitcnt vmcnt(0)
	v_add_f64 v[16:17], v[16:17], v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[6:7], -v[14:15]
	v_min_f64 v[27:28], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ngt_f64_e32 s[18:19], v[27:28]
	s_cbranch_execz .LBB4_9
; %bb.11:                               ;   in Loop: Header=BB4_10 Depth=2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_add_f64 v[25:26], v[10:11], -v[16:17]
	v_mul_f64 v[27:28], v[16:17], v[16:17]
	v_add_f64 v[29:30], s[16:17], v[14:15]
	v_add_f64 v[18:19], s[16:17], v[18:19]
	v_mul_f64 v[25:26], v[25:26], v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_div_scale_f64 v[31:32], null, v[29:30], v[29:30], v[27:28]
	v_div_scale_f64 v[43:44], vcc_lo, v[27:28], v[29:30], v[27:28]
	v_div_scale_f64 v[33:34], null, v[18:19], v[18:19], v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[35:36], v[31:32]
	v_rcp_f64_e32 v[37:38], v[33:34]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[39:40], -v[31:32], v[35:36], 1.0
	v_fma_f64 v[41:42], -v[33:34], v[37:38], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[35:36], v[35:36], v[39:40], v[35:36]
	v_fma_f64 v[37:38], v[37:38], v[41:42], v[37:38]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[39:40], -v[31:32], v[35:36], 1.0
	v_fma_f64 v[41:42], -v[33:34], v[37:38], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[35:36], v[35:36], v[39:40], v[35:36]
	v_div_scale_f64 v[39:40], s2, v[25:26], v[18:19], v[25:26]
	v_fma_f64 v[37:38], v[37:38], v[41:42], v[37:38]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[41:42], v[43:44], v[35:36]
	v_mul_f64 v[45:46], v[39:40], v[37:38]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], -v[31:32], v[41:42], v[43:44]
	v_fma_f64 v[33:34], -v[33:34], v[45:46], v[39:40]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[31:32], v[31:32], v[35:36], v[41:42]
	s_mov_b32 vcc_lo, s2
	v_div_fmas_f64 v[33:34], v[33:34], v[37:38], v[45:46]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fixup_f64 v[27:28], v[31:32], v[29:30], v[27:28]
	v_div_fixup_f64 v[18:19], v[33:34], v[18:19], v[25:26]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[27:28], v[18:19]
	v_add_f64 v[18:19], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, v[18:19], v[2:3]
	v_cndmask_b32_e64 v20, v20, s5, vcc_lo
	v_cndmask_b32_e64 v0, v0, s12, vcc_lo
	v_dual_cndmask_b32 v3, v3, v19 :: v_dual_cndmask_b32 v2, v2, v18
	s_branch .LBB4_9
.LBB4_12:
	v_mov_b32_e32 v0, -1
	v_mov_b32_e32 v20, -1
.LBB4_13:
	s_load_b32 s0, s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	v_add_nc_u32_e32 v1, s0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s8, v1
	v_add_co_ci_u32_e64 v4, null, s9, v2, vcc_lo
	v_add_co_u32 v1, vcc_lo, s10, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s11, v2, vcc_lo
	global_store_b32 v[3:4], v0, off
	global_store_b32 v[1:2], v20, off
.LBB4_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i
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
		.amdhsa_next_free_vgpr 47
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
.Lfunc_end4:
	.size	_Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i, .Lfunc_end4-_Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i
                                        ; -- End function
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.num_vgpr, 47
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.num_agpr, 0
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.numbered_sgpr, 22
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.num_named_barrier, 0
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.private_seg_size, 0
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.uses_vcc, 1
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.uses_flat_scratch, 0
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.has_dyn_sized_stack, 0
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.has_recursion, 0
	.set _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1144
; TotalNumSgprs: 24
; NumVgprs: 47
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 24
; NumVGPRsForWavesPerEU: 47
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
	.protected	_Z21tb_repartition_kernelPKdPiPKiS3_ii ; -- Begin function _Z21tb_repartition_kernelPKdPiPKiS3_ii
	.globl	_Z21tb_repartition_kernelPKdPiPKiS3_ii
	.p2align	8
	.type	_Z21tb_repartition_kernelPKdPiPKiS3_ii,@function
_Z21tb_repartition_kernelPKdPiPKiS3_ii: ; @_Z21tb_repartition_kernelPKdPiPKiS3_ii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b64 s[8:9], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v2
	s_cbranch_execz .LBB5_3
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b32 v3, v[0:1], off
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[4:5], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s4, v4
	v_add_co_ci_u32_e64 v7, null, s5, v5, vcc_lo
	global_load_b32 v6, v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmp_lt_i32_e32 vcc_lo, -1, v6
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB5_3
; %bb.2:
	v_mad_u64_u32 v[7:8], null, v2, s9, v[6:7]
	v_lshlrev_b32_e32 v3, 1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	v_lshlrev_b64 v[6:7], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v6
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	global_load_b32 v2, v[4:5], off
	v_or_b32_e32 v5, 1, v3
	v_add_nc_u32_e32 v3, 2, v3
	s_waitcnt vmcnt(1)
	v_cvt_i32_f64_e32 v4, v[6:7]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_lt_i32_e32 vcc_lo, v2, v4
	v_cndmask_b32_e32 v2, v5, v3, vcc_lo
	global_store_b32 v[0:1], v2, off
.LBB5_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21tb_repartition_kernelPKdPiPKiS3_ii
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
		.amdhsa_next_free_vgpr 9
		.amdhsa_next_free_sgpr 10
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
.Lfunc_end5:
	.size	_Z21tb_repartition_kernelPKdPiPKiS3_ii, .Lfunc_end5-_Z21tb_repartition_kernelPKdPiPKiS3_ii
                                        ; -- End function
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.num_vgpr, 9
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.num_agpr, 0
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.numbered_sgpr, 10
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.num_named_barrier, 0
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.private_seg_size, 0
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.uses_vcc, 1
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.uses_flat_scratch, 0
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.has_dyn_sized_stack, 0
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.has_recursion, 0
	.set _Z21tb_repartition_kernelPKdPiPKiS3_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 304
; TotalNumSgprs: 12
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 9
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
	.protected	_Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i ; -- Begin function _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i
	.globl	_Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i
	.p2align	8
	.type	_Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i,@function
_Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i:  ; @_Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i
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
	s_cbranch_execz .LBB6_5
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_mov_b32 s2, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	v_lshlrev_b64 v[6:7], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v4, null, s9, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s5, v7, vcc_lo
	global_load_b32 v3, v[3:4], off
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, s10, v4
	v_add_co_ci_u32_e64 v9, null, s11, v5, vcc_lo
	global_load_b64 v[10:11], v[0:1], off
	global_load_b64 v[2:3], v[8:9], off
.LBB6_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[10:11]
	global_atomic_cmpswap_b64 v[0:1], v[8:9], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB6_2
; %bb.3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s6, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, s0, v4
	v_add_co_ci_u32_e64 v5, null, s1, v5, vcc_lo
	global_load_b64 v[6:7], v[0:1], off
	global_load_b64 v[2:3], v[4:5], off
	s_mov_b32 s0, 0
.LBB6_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[6:7]
	global_atomic_cmpswap_b64 v[0:1], v[4:5], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB6_4
.LBB6_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i
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
		.amdhsa_next_free_vgpr 12
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
.Lfunc_end6:
	.size	_Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i, .Lfunc_end6-_Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i
                                        ; -- End function
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.num_vgpr, 12
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.num_agpr, 0
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.numbered_sgpr, 12
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.num_named_barrier, 0
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.private_seg_size, 0
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.uses_vcc, 1
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.uses_flat_scratch, 0
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.has_dyn_sized_stack, 0
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.has_recursion, 0
	.set _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 376
; TotalNumSgprs: 14
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 12
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
	.protected	_Z18tb_leaf_val_kernelPKdS0_PdiiS0_ ; -- Begin function _Z18tb_leaf_val_kernelPKdS0_PdiiS0_
	.globl	_Z18tb_leaf_val_kernelPKdS0_PdiiS0_
	.p2align	8
	.type	_Z18tb_leaf_val_kernelPKdS0_PdiiS0_,@function
_Z18tb_leaf_val_kernelPKdS0_PdiiS0_:    ; @_Z18tb_leaf_val_kernelPKdS0_PdiiS0_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b64 s[4:5], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB7_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_add_nc_u32_e32 v0, s5, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v1, 31, v0
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s10, v0
	v_add_co_ci_u32_e64 v3, null, s11, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s9, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_load_b64 s[0:1], s[0:1], 0x0
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(1) lgkmcnt(0)
	v_add_f64 v[2:3], s[0:1], v[2:3]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[6:7], null, v[2:3], v[2:3], -v[4:5]
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_div_scale_f64 v[10:11], vcc_lo, -v[4:5], v[2:3], -v[4:5]
	v_mul_f64 v[12:13], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[12:13], v[10:11]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[12:13]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_div_fixup_f64 v[2:3], v[6:7], v[2:3], -v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB7_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z18tb_leaf_val_kernelPKdS0_PdiiS0_
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
		.amdhsa_next_free_vgpr 14
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
.Lfunc_end7:
	.size	_Z18tb_leaf_val_kernelPKdS0_PdiiS0_, .Lfunc_end7-_Z18tb_leaf_val_kernelPKdS0_PdiiS0_
                                        ; -- End function
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.num_vgpr, 14
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.num_agpr, 0
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.numbered_sgpr, 12
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.num_named_barrier, 0
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.private_seg_size, 0
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.uses_vcc, 1
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.uses_flat_scratch, 0
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.has_dyn_sized_stack, 0
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.has_recursion, 0
	.set _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 328
; TotalNumSgprs: 14
; NumVgprs: 14
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
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
	.text
	.protected	_Z17tb_scatter_kernelPKiPKdPdi ; -- Begin function _Z17tb_scatter_kernelPKiPKdPdi
	.globl	_Z17tb_scatter_kernelPKiPKdPdi
	.p2align	8
	.type	_Z17tb_scatter_kernelPKiPKdPdi,@function
_Z17tb_scatter_kernelPKiPKdPdi:         ; @_Z17tb_scatter_kernelPKiPKdPdi
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB8_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s5, v4, vcc_lo
	global_load_b32 v3, v[3:4], off
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s6, v3
	v_add_co_ci_u32_e64 v4, null, s7, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB8_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z17tb_scatter_kernelPKiPKdPdi
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
		.amdhsa_next_free_vgpr 5
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
.Lfunc_end8:
	.size	_Z17tb_scatter_kernelPKiPKdPdi, .Lfunc_end8-_Z17tb_scatter_kernelPKiPKdPdi
                                        ; -- End function
	.set _Z17tb_scatter_kernelPKiPKdPdi.num_vgpr, 5
	.set _Z17tb_scatter_kernelPKiPKdPdi.num_agpr, 0
	.set _Z17tb_scatter_kernelPKiPKdPdi.numbered_sgpr, 8
	.set _Z17tb_scatter_kernelPKiPKdPdi.num_named_barrier, 0
	.set _Z17tb_scatter_kernelPKiPKdPdi.private_seg_size, 0
	.set _Z17tb_scatter_kernelPKiPKdPdi.uses_vcc, 1
	.set _Z17tb_scatter_kernelPKiPKdPdi.uses_flat_scratch, 0
	.set _Z17tb_scatter_kernelPKiPKdPdi.has_dyn_sized_stack, 0
	.set _Z17tb_scatter_kernelPKiPKdPdi.has_recursion, 0
	.set _Z17tb_scatter_kernelPKiPKdPdi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 212
; TotalNumSgprs: 10
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 5
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
	.protected	_Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii ; -- Begin function _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii
	.globl	_Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii
	.p2align	8
	.type	_Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii,@function
_Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii: ; @_Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b128 s[12:15], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s12, v1
	s_cbranch_execz .LBB9_9
; %bb.1:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x20
	s_cmp_lt_i32 s14, 1
	s_cbranch_scc1 .LBB9_7
; %bb.2:
	v_mul_lo_u32 v0, v1, s13
	s_add_i32 s0, s14, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v5, s0
	s_mov_b32 s1, 0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB9_4
	.p2align	6
.LBB9_3:                                ;   in Loop: Header=BB9_4 Depth=1
	s_or_b32 exec_lo, exec_lo, s12
	v_sub_co_u32 v5, s0, v5, 1
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s0, exec_lo, s0
	s_or_b32 s1, s0, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB9_6
.LBB9_4:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	s_mov_b32 s12, exec_lo
	v_lshlrev_b64 v[3:4], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s6, v3
	v_add_co_ci_u32_e64 v7, null, s7, v4, vcc_lo
	global_load_b32 v6, v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmp_gt_i32_e32 vcc_lo, 0, v6
	v_cmpx_lt_i32_e32 -1, v6
	s_cbranch_execz .LBB9_3
; %bb.5:                                ;   in Loop: Header=BB9_4 Depth=1
	v_add_nc_u32_e32 v6, v6, v0
	v_lshlrev_b32_e32 v2, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v7, 31, v6
	v_lshlrev_b64 v[6:7], 3, v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, s0, s4, v6
	v_add_co_ci_u32_e64 v7, null, s5, v7, s0
	v_add_co_u32 v3, s0, s8, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s9, v4, s0
	global_load_b64 v[6:7], v[6:7], off
	global_load_b32 v3, v[3:4], off
	s_waitcnt vmcnt(1)
	v_cvt_i32_f64_e32 v4, v[6:7]
	v_or_b32_e32 v6, 1, v2
	v_add_nc_u32_e32 v2, 2, v2
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_lt_i32_e64 s0, v3, v4
	v_cndmask_b32_e64 v2, v6, v2, s0
	s_branch .LBB9_3
.LBB9_6:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s1
	v_ashrrev_i32_e32 v3, 31, v2
	s_branch .LBB9_8
.LBB9_7:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
.LBB9_8:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s10, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s11, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB9_9:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii
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
.Lfunc_end9:
	.size	_Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii, .Lfunc_end9-_Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii
                                        ; -- End function
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.num_vgpr, 8
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.num_agpr, 0
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.numbered_sgpr, 16
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.num_named_barrier, 0
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.private_seg_size, 0
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.uses_vcc, 1
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.uses_flat_scratch, 0
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.has_dyn_sized_stack, 0
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.has_recursion, 0
	.set _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 472
; TotalNumSgprs: 18
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 18
; NumVGPRsForWavesPerEU: 8
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
	.protected	oblivious_histogram_kernel ; -- Begin function oblivious_histogram_kernel
	.globl	oblivious_histogram_kernel
	.p2align	8
	.type	oblivious_histogram_kernel,@function
oblivious_histogram_kernel:             ; @oblivious_histogram_kernel
; %bb.0:
	s_load_b128 s[16:19], s[0:1], 0x30
	v_lshlrev_b32_e32 v3, 2, v0
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_i32_e32 vcc_lo, s18, v0
	s_and_saveexec_b32 s5, vcc_lo
	s_cbranch_execz .LBB10_3
; %bb.1:
	s_load_b32 s4, s[0:1], 0x4c
	v_dual_mov_b32 v2, 0 :: v_dual_lshlrev_b32 v1, 2, v0
	v_mov_b32_e32 v4, v0
	s_mov_b32 s8, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s6, s4, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s7, s6, 2
.LBB10_2:                               ; =>This Inner Loop Header: Depth=1
	v_add_nc_u32_e32 v4, s6, v4
	ds_store_2addr_stride64_b32 v1, v2, v2 offset1:4
	v_add_nc_u32_e32 v1, s7, v1
	v_cmp_le_i32_e64 s4, s18, v4
	s_or_b32 s8, s4, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s8
	s_cbranch_execnz .LBB10_2
.LBB10_3:
	s_or_b32 exec_lo, exec_lo, s5
	s_clause 0x1
	s_load_b256 s[8:15], s[0:1], 0x0
	s_load_b128 s[20:23], s[0:1], 0x20
	s_mov_b32 s5, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_i32_e64 s16, v0
	s_cbranch_execz .LBB10_8
; %bb.4:
	s_load_b32 s4, s[0:1], 0x4c
	s_mul_i32 s7, s16, s2
	v_mov_b32_e32 v1, v0
	s_ashr_i32 s19, s7, 31
	s_add_u32 s7, s8, s7
	s_mov_b32 s6, 0
	s_addc_u32 s8, s9, s19
	s_and_b32 s19, s3, 0xff
	s_waitcnt lgkmcnt(0)
	s_and_b32 s9, s4, 0xffff
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB10_6
	.p2align	6
.LBB10_5:                               ;   in Loop: Header=BB10_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s24
	v_add_nc_u32_e32 v1, s9, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e64 s4, s16, v1
	s_or_b32 s6, s4, s6
	s_and_not1_b32 exec_lo, exec_lo, s6
	s_cbranch_execz .LBB10_8
.LBB10_6:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	v_add_co_u32 v4, s4, s10, v1
	s_mov_b32 s24, exec_lo
	v_add_co_ci_u32_e64 v5, null, s11, v2, s4
	global_load_d16_u8 v4, v[4:5], off
	s_waitcnt vmcnt(0)
	v_cmpx_eq_u16_e32 s19, v4.l
	s_cbranch_execz .LBB10_5
; %bb.7:                                ;   in Loop: Header=BB10_6 Depth=1
	v_add_co_u32 v4, s4, s7, v1
	v_lshlrev_b64 v[6:7], 2, v[1:2]
	v_add_co_ci_u32_e64 v5, null, s8, v2, s4
	global_load_u8 v2, v[4:5], off
	v_add_co_u32 v4, s4, s12, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s13, v7, s4
	global_load_b32 v8, v[4:5], off
	v_add_co_u32 v4, s4, s14, v6
	v_add_co_ci_u32_e64 v5, null, s15, v7, s4
	s_waitcnt vmcnt(1)
	v_lshlrev_b32_e32 v2, 2, v2
	s_waitcnt vmcnt(0)
	ds_add_f32 v2, v8
	global_load_b32 v4, v[4:5], off
	s_waitcnt vmcnt(0)
	ds_add_f32 v2, v4 offset:1024
	s_branch .LBB10_5
.LBB10_8:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s4, vcc_lo
	s_cbranch_execz .LBB10_11
; %bb.9:
	s_load_b32 s0, s[0:1], 0x4c
	s_mul_i32 s1, s17, s3
	s_mov_b32 s4, 0
	s_add_i32 s2, s1, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s1, s0, 0xffff
	s_lshl_b32 s3, s1, 2
	.p2align	6
.LBB10_10:                              ; =>This Inner Loop Header: Depth=1
	v_add_nc_u32_e32 v1, s2, v0
	ds_load_2addr_stride64_b32 v[4:5], v3 offset1:4
	v_add_nc_u32_e32 v0, s1, v0
	v_add_nc_u32_e32 v3, s3, v3
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_i32_e32 vcc_lo, s18, v0
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_or_b32 s4, vcc_lo, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, s0, s20, v1
	v_add_co_ci_u32_e64 v7, null, s21, v2, s0
	v_add_co_u32 v1, s0, s22, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s23, v2, s0
	s_waitcnt lgkmcnt(0)
	global_store_b32 v[6:7], v4, off
	global_store_b32 v[1:2], v5, off
	s_and_not1_b32 exec_lo, exec_lo, s4
	s_cbranch_execnz .LBB10_10
.LBB10_11:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel oblivious_histogram_kernel
		.amdhsa_group_segment_fixed_size 2048
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 320
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
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 9
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
.Lfunc_end10:
	.size	oblivious_histogram_kernel, .Lfunc_end10-oblivious_histogram_kernel
                                        ; -- End function
	.set oblivious_histogram_kernel.num_vgpr, 9
	.set oblivious_histogram_kernel.num_agpr, 0
	.set oblivious_histogram_kernel.numbered_sgpr, 25
	.set oblivious_histogram_kernel.num_named_barrier, 0
	.set oblivious_histogram_kernel.private_seg_size, 0
	.set oblivious_histogram_kernel.uses_vcc, 1
	.set oblivious_histogram_kernel.uses_flat_scratch, 0
	.set oblivious_histogram_kernel.has_dyn_sized_stack, 0
	.set oblivious_histogram_kernel.has_recursion, 0
	.set oblivious_histogram_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 616
; TotalNumSgprs: 27
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 2048 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 27
; NumVGPRsForWavesPerEU: 9
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	oblivious_route_step_kernel ; -- Begin function oblivious_route_step_kernel
	.globl	oblivious_route_step_kernel
	.p2align	8
	.type	oblivious_route_step_kernel,@function
oblivious_route_step_kernel:            ; @oblivious_route_step_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s7, v1
	s_cbranch_execz .LBB11_2
; %bb.1:
	s_clause 0x1
	s_load_b32 s2, s[0:1], 0x28
	s_load_b128 s[8:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v4, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	v_mad_u64_u32 v[2:3], null, v1, s2, s[4:5]
	s_and_b32 s2, s5, 0xff
	v_ashrrev_i32_e32 v0, 31, v2
	v_add_co_u32 v2, vcc_lo, s8, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s9, v0, vcc_lo
	global_load_d16_u8 v0, v[2:3], off
	v_add_co_u32 v2, vcc_lo, s10, v1
	v_add_co_ci_u32_e64 v3, null, s11, v4, vcc_lo
	global_load_d16_hi_u8 v0, v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_lt_u16_e32 vcc_lo, s2, v0.l
	v_cndmask_b32_e64 v2, 0, 1, vcc_lo
	v_add_co_u32 v1, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b32_e32 v2, s6, v2
	v_or_b16 v0.l, v0.h, v2.l
	v_add_co_ci_u32_e64 v2, null, s1, v4, vcc_lo
	global_store_b8 v[1:2], v0, off
.LBB11_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel oblivious_route_step_kernel
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
		.amdhsa_next_free_vgpr 5
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
.Lfunc_end11:
	.size	oblivious_route_step_kernel, .Lfunc_end11-oblivious_route_step_kernel
                                        ; -- End function
	.set oblivious_route_step_kernel.num_vgpr, 5
	.set oblivious_route_step_kernel.num_agpr, 0
	.set oblivious_route_step_kernel.numbered_sgpr, 12
	.set oblivious_route_step_kernel.num_named_barrier, 0
	.set oblivious_route_step_kernel.private_seg_size, 0
	.set oblivious_route_step_kernel.uses_vcc, 1
	.set oblivious_route_step_kernel.uses_flat_scratch, 0
	.set oblivious_route_step_kernel.has_dyn_sized_stack, 0
	.set oblivious_route_step_kernel.has_recursion, 0
	.set oblivious_route_step_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 228
; TotalNumSgprs: 14
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 5
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
	.protected	oblivious_route_step_dev_kernel ; -- Begin function oblivious_route_step_dev_kernel
	.globl	oblivious_route_step_dev_kernel
	.p2align	8
	.type	oblivious_route_step_dev_kernel,@function
oblivious_route_step_dev_kernel:        ; @oblivious_route_step_dev_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b128 s[4:7], s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s5, v1
	s_cbranch_execz .LBB12_2
; %bb.1:
	s_load_b256 s[8:15], s[0:1], 0x0
	s_ashr_i32 s5, s4, 31
	v_mov_b32_e32 v4, 0
	s_lshl_b64 s[2:3], s[4:5], 2
	s_waitcnt lgkmcnt(0)
	s_add_u32 s2, s14, s2
	s_addc_u32 s3, s15, s3
	s_load_b32 s2, s[2:3], 0x0
	s_load_b64 s[0:1], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	v_mad_u64_u32 v[2:3], null, v1, s6, s[2:3]
	s_add_u32 s0, s0, s4
	s_addc_u32 s1, s1, s5
	v_ashrrev_i32_e32 v0, 31, v2
	v_add_co_u32 v2, vcc_lo, s8, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s9, v0, vcc_lo
	global_load_d16_u8 v0, v4, s[0:1]
	global_load_d16_hi_u8 v0, v[2:3], off
	v_ashrrev_i32_e32 v4, 31, v1
	v_add_co_u32 v2, vcc_lo, s10, v1
	v_add_co_ci_u32_e64 v3, null, s11, v4, vcc_lo
	global_load_d16_u8 v2, v[2:3], off
	s_waitcnt vmcnt(1)
	v_cmp_gt_u16_e32 vcc_lo, v0.h, v0.l
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_add_co_u32 v1, vcc_lo, s12, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b32_e32 v0, s4, v0
	s_waitcnt vmcnt(0)
	v_or_b16 v0.l, v2.l, v0.l
	v_add_co_ci_u32_e64 v2, null, s13, v4, vcc_lo
	global_store_b8 v[1:2], v0, off
.LBB12_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel oblivious_route_step_dev_kernel
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
		.amdhsa_next_free_vgpr 5
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
.Lfunc_end12:
	.size	oblivious_route_step_dev_kernel, .Lfunc_end12-oblivious_route_step_dev_kernel
                                        ; -- End function
	.set oblivious_route_step_dev_kernel.num_vgpr, 5
	.set oblivious_route_step_dev_kernel.num_agpr, 0
	.set oblivious_route_step_dev_kernel.numbered_sgpr, 16
	.set oblivious_route_step_dev_kernel.num_named_barrier, 0
	.set oblivious_route_step_dev_kernel.private_seg_size, 0
	.set oblivious_route_step_dev_kernel.uses_vcc, 1
	.set oblivious_route_step_dev_kernel.uses_flat_scratch, 0
	.set oblivious_route_step_dev_kernel.has_dyn_sized_stack, 0
	.set oblivious_route_step_dev_kernel.has_recursion, 0
	.set oblivious_route_step_dev_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 260
; TotalNumSgprs: 18
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 18
; NumVGPRsForWavesPerEU: 5
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
	.protected	oblivious_route_full_kernel ; -- Begin function oblivious_route_full_kernel
	.globl	oblivious_route_full_kernel
	.p2align	8
	.type	oblivious_route_full_kernel,@function
oblivious_route_full_kernel:            ; @oblivious_route_full_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[8:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB13_6
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc1 .LBB13_4
; %bb.2:
	v_mul_lo_u32 v0, v1, s9
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v4, 31, v0
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s0, v0
	v_mov_b32_e32 v0, 0
	v_add_co_ci_u32_e64 v4, null, s1, v4, vcc_lo
	s_mov_b64 s[0:1], 0
	.p2align	6
.LBB13_3:                               ; =>This Inner Loop Header: Depth=1
	s_load_b32 s8, s[2:3], 0x0
	s_waitcnt lgkmcnt(0)
	s_ashr_i32 s9, s8, 31
	v_add_co_u32 v6, vcc_lo, v3, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s9, v4, vcc_lo
	s_add_u32 s8, s4, s0
	s_addc_u32 s9, s5, s1
	global_load_d16_u8 v5, v2, s[8:9]
	global_load_d16_hi_u8 v5, v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmp_gt_u16_e32 vcc_lo, v5.h, v5.l
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_lshl_or_b32 v0, v5, s0, v0
	s_add_u32 s0, s0, 1
	s_addc_u32 s1, s1, 0
	s_add_u32 s2, s2, 4
	s_addc_u32 s3, s3, 0
	s_cmp_eq_u32 s10, s0
	s_cbranch_scc0 .LBB13_3
	s_branch .LBB13_5
.LBB13_4:
	v_mov_b16_e32 v0.l, 0
.LBB13_5:
	v_ashrrev_i32_e32 v2, 31, v1
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v1, vcc_lo, s6, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s7, v2, vcc_lo
	global_store_b8 v[1:2], v0, off
.LBB13_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel oblivious_route_full_kernel
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
.Lfunc_end13:
	.size	oblivious_route_full_kernel, .Lfunc_end13-oblivious_route_full_kernel
                                        ; -- End function
	.set oblivious_route_full_kernel.num_vgpr, 8
	.set oblivious_route_full_kernel.num_agpr, 0
	.set oblivious_route_full_kernel.numbered_sgpr, 12
	.set oblivious_route_full_kernel.num_named_barrier, 0
	.set oblivious_route_full_kernel.private_seg_size, 0
	.set oblivious_route_full_kernel.uses_vcc, 1
	.set oblivious_route_full_kernel.uses_flat_scratch, 0
	.set oblivious_route_full_kernel.has_dyn_sized_stack, 0
	.set oblivious_route_full_kernel.has_recursion, 0
	.set oblivious_route_full_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 284
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
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	scatter_add_by_leaf_kernel ; -- Begin function scatter_add_by_leaf_kernel
	.globl	scatter_add_by_leaf_kernel
	.p2align	8
	.type	scatter_add_by_leaf_kernel,@function
scatter_add_by_leaf_kernel:             ; @scatter_add_by_leaf_kernel
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
	s_cbranch_execz .LBB14_2
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s3, v2, vcc_lo
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	global_load_u8 v3, v[3:4], off
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_lshlrev_b32_e32 v2, 2, v3
	global_load_b32 v2, v2, s[4:5]
	global_load_b32 v3, v[0:1], off
	s_load_b32 s0, s[6:7], 0x0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fmac_f32_e32 v3, s0, v2
	global_store_b32 v[0:1], v3, off
.LBB14_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scatter_add_by_leaf_kernel
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
		.amdhsa_next_free_vgpr 5
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
.Lfunc_end14:
	.size	scatter_add_by_leaf_kernel, .Lfunc_end14-scatter_add_by_leaf_kernel
                                        ; -- End function
	.set scatter_add_by_leaf_kernel.num_vgpr, 5
	.set scatter_add_by_leaf_kernel.num_agpr, 0
	.set scatter_add_by_leaf_kernel.numbered_sgpr, 8
	.set scatter_add_by_leaf_kernel.num_named_barrier, 0
	.set scatter_add_by_leaf_kernel.private_seg_size, 0
	.set scatter_add_by_leaf_kernel.uses_vcc, 1
	.set scatter_add_by_leaf_kernel.uses_flat_scratch, 0
	.set scatter_add_by_leaf_kernel.has_dyn_sized_stack, 0
	.set scatter_add_by_leaf_kernel.has_recursion, 0
	.set scatter_add_by_leaf_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 180
; TotalNumSgprs: 10
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 5
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
	.protected	leaf_reduce_kernel      ; -- Begin function leaf_reduce_kernel
	.globl	leaf_reduce_kernel
	.p2align	8
	.type	leaf_reduce_kernel,@function
leaf_reduce_kernel:                     ; @leaf_reduce_kernel
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
	s_cbranch_execz .LBB15_5
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s4, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s5, v2, vcc_lo
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	global_load_u8 v6, v[3:4], off
	v_add_co_u32 v2, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_lshlrev_b32_e32 v4, 2, v6
	global_load_b32 v7, v[2:3], off
	global_load_b32 v5, v4, s[10:11]
	v_add_co_u32 v2, s2, s10, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s11, 0, s2
	s_mov_b32 s2, 0
.LBB15_2:                               ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v4, v5, v7
	global_atomic_cmpswap_b32 v4, v[2:3], v[4:5], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v4, v5
	v_mov_b32_e32 v5, v4
	s_or_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB15_2
; %bb.3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	v_lshlrev_b32_e32 v2, 2, v6
	global_load_b32 v4, v[0:1], off
	global_load_b32 v3, v2, s[0:1]
	v_add_co_u32 v0, s0, s0, v2
	v_add_co_ci_u32_e64 v1, null, s1, 0, s0
	s_mov_b32 s0, 0
.LBB15_4:                               ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v3, v4
	global_atomic_cmpswap_b32 v2, v[0:1], v[2:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v2, v3
	v_mov_b32_e32 v3, v2
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB15_4
.LBB15_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel leaf_reduce_kernel
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
.Lfunc_end15:
	.size	leaf_reduce_kernel, .Lfunc_end15-leaf_reduce_kernel
                                        ; -- End function
	.set leaf_reduce_kernel.num_vgpr, 8
	.set leaf_reduce_kernel.num_agpr, 0
	.set leaf_reduce_kernel.numbered_sgpr, 12
	.set leaf_reduce_kernel.num_named_barrier, 0
	.set leaf_reduce_kernel.private_seg_size, 0
	.set leaf_reduce_kernel.uses_vcc, 1
	.set leaf_reduce_kernel.uses_flat_scratch, 0
	.set leaf_reduce_kernel.has_dyn_sized_stack, 0
	.set leaf_reduce_kernel.has_recursion, 0
	.set leaf_reduce_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 340
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
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	leaf_finalize_kernel    ; -- Begin function leaf_finalize_kernel
	.globl	leaf_finalize_kernel
	.p2align	8
	.type	leaf_finalize_kernel,@function
leaf_finalize_kernel:                   ; @leaf_finalize_kernel
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
	s_cbranch_execz .LBB16_2
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v5, null, s1, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	global_load_b32 v3, v[4:5], off
	s_load_b32 s0, s[6:7], 0x0
	s_waitcnt vmcnt(1) lgkmcnt(0)
	v_add_f32_e32 v2, s0, v2
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f32 v4, null, v2, v2, -v3
	v_rcp_f32_e32 v5, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v6, -v4, v5, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v5, v6, v5
	v_div_scale_f32 v7, vcc_lo, -v3, v2, -v3
	v_mul_f32_e32 v6, v7, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v8, -v4, v6, v7
	v_fmac_f32_e32 v6, v8, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v4, -v4, v6, v7
	v_div_fmas_f32 v4, v4, v5, v6
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	v_div_fixup_f32 v2, v4, v2, -v3
	global_store_b32 v[0:1], v2, off
.LBB16_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel leaf_finalize_kernel
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
		.amdhsa_next_free_vgpr 9
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
.Lfunc_end16:
	.size	leaf_finalize_kernel, .Lfunc_end16-leaf_finalize_kernel
                                        ; -- End function
	.set leaf_finalize_kernel.num_vgpr, 9
	.set leaf_finalize_kernel.num_agpr, 0
	.set leaf_finalize_kernel.numbered_sgpr, 8
	.set leaf_finalize_kernel.num_named_barrier, 0
	.set leaf_finalize_kernel.private_seg_size, 0
	.set leaf_finalize_kernel.uses_vcc, 1
	.set leaf_finalize_kernel.uses_flat_scratch, 0
	.set leaf_finalize_kernel.has_dyn_sized_stack, 0
	.set leaf_finalize_kernel.has_recursion, 0
	.set leaf_finalize_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 284
; TotalNumSgprs: 10
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.protected	oblivious_split_eval_kernel ; -- Begin function oblivious_split_eval_kernel
	.globl	oblivious_split_eval_kernel
	.p2align	8
	.type	oblivious_split_eval_kernel,@function
oblivious_split_eval_kernel:            ; @oblivious_split_eval_kernel
; %bb.0:
	s_load_b128 s[8:11], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s9
	s_cbranch_scc1 .LBB17_25
; %bb.1:
	s_clause 0x2
	s_load_b128 s[16:19], s[0:1], 0x28
	s_load_b128 s[12:15], s[0:1], 0x0
	s_load_b64 s[4:5], s[0:1], 0x10
	s_mov_b32 s3, exec_lo
	s_waitcnt lgkmcnt(0)
	s_load_b32 s6, s[16:17], 0x0
	s_load_b32 s7, s[18:19], 0x0
	v_cmpx_gt_i32_e64 s10, v0
	s_cbranch_execz .LBB17_4
; %bb.2:
	s_load_b32 s0, s[0:1], 0x44
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, v0
	s_mul_i32 s1, s10, s2
	s_mov_b32 s16, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s11, s0, 0xffff
.LBB17_3:                               ; =>This Inner Loop Header: Depth=1
	v_add_nc_u32_e32 v3, s1, v2
	v_add_nc_u32_e32 v2, s11, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v4, 31, v3
	v_cmp_le_i32_e32 vcc_lo, s10, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	s_or_b32 s16, vcc_lo, s16
	v_add_co_u32 v3, s0, s4, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s5, v4, s0
	global_store_b32 v[3:4], v1, off
	s_and_not1_b32 exec_lo, exec_lo, s16
	s_cbranch_execnz .LBB17_3
.LBB17_4:
	s_or_b32 exec_lo, exec_lo, s3
	s_cmp_lt_i32 s8, 1
	s_waitcnt lgkmcnt(0)
	s_waitcnt_vscnt null, 0x0
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB17_25
; %bb.5:
	v_mad_u64_u32 v[1:2], null, s10, s2, v[0:1]
	v_dual_mov_b32 v2, 0 :: v_dual_lshlrev_b32 v5, 2, v0
	s_cmp_gt_i32 s10, 1
	v_cmp_gt_u32_e64 s0, s10, v0
	s_cselect_b32 s11, -1, 0
	s_add_i32 s1, s10, -1
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	s_lshl_b32 s16, s1, 2
	v_cmp_gt_u32_e64 s1, s1, v0
	s_add_i32 s3, s16, 0x400
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_mov_b32 v7, s3 :: v_dual_add_nc_u32 v6, 0x400, v5
	v_add_co_u32 v3, vcc_lo, s4, v3
	v_add_co_ci_u32_e64 v4, null, s5, v4, vcc_lo
	s_mov_b32 s5, 0
	s_branch .LBB17_7
.LBB17_6:                               ;   in Loop: Header=BB17_7 Depth=1
	s_add_i32 s5, s5, 1
	buffer_gl0_inv
	s_cmp_lg_u32 s5, s8
	s_cbranch_scc0 .LBB17_25
.LBB17_7:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB17_14 Depth 2
	s_mul_i32 s3, s5, s9
	v_mov_b32_e32 v8, 0
	s_add_i32 s3, s3, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s3, s3, s10
	v_add_nc_u32_e32 v1, s3, v0
	s_and_saveexec_b32 s3, s0
	s_cbranch_execz .LBB17_9
; %bb.8:                                ;   in Loop: Header=BB17_7 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[8:9], 2, v[1:2]
	v_add_co_u32 v8, vcc_lo, s12, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s13, v9, vcc_lo
	global_load_b32 v8, v[8:9], off
.LBB17_9:                               ;   in Loop: Header=BB17_7 Depth=1
	s_or_b32 exec_lo, exec_lo, s3
	v_mov_b32_e32 v9, 0
	s_and_saveexec_b32 s3, s0
	s_cbranch_execz .LBB17_11
; %bb.10:                               ;   in Loop: Header=BB17_7 Depth=1
	v_lshlrev_b64 v[9:10], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v9, vcc_lo, s14, v9
	v_add_co_ci_u32_e64 v10, null, s15, v10, vcc_lo
	global_load_b32 v9, v[9:10], off
.LBB17_11:                              ;   in Loop: Header=BB17_7 Depth=1
	s_or_b32 exec_lo, exec_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s11
	s_waitcnt vmcnt(0)
	ds_store_b32 v5, v8
	ds_store_b32 v6, v9
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_vccnz .LBB17_18
; %bb.12:                               ;   in Loop: Header=BB17_7 Depth=1
	s_mov_b32 s3, 1
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB17_14
	.p2align	6
.LBB17_13:                              ;   in Loop: Header=BB17_14 Depth=2
	s_or_b32 exec_lo, exec_lo, s4
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v9, v5
	ds_load_b32 v10, v6
	s_lshl_b32 s3, s3, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_ge_i32 s3, s10
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v8, v8, v9 :: v_dual_add_f32 v1, v1, v10
	ds_store_b32 v5, v8
	ds_store_b32 v6, v1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB17_18
.LBB17_14:                              ;   Parent Loop BB17_7 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_subrev_nc_u32_e32 v8, s3, v0
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	v_mov_b32_e32 v1, 0
	s_delay_alu instid0(VALU_DEP_3)
	v_dual_mov_b32 v8, 0 :: v_dual_lshlrev_b32 v9, 2, v8
	s_and_saveexec_b32 s4, vcc_lo
; %bb.15:                               ;   in Loop: Header=BB17_14 Depth=2
	ds_load_b32 v8, v9
; %bb.16:                               ;   in Loop: Header=BB17_14 Depth=2
	s_or_b32 exec_lo, exec_lo, s4
	s_and_saveexec_b32 s4, vcc_lo
	s_cbranch_execz .LBB17_13
; %bb.17:                               ;   in Loop: Header=BB17_14 Depth=2
	ds_load_b32 v1, v9 offset:1024
	s_branch .LBB17_13
.LBB17_18:                              ;   in Loop: Header=BB17_7 Depth=1
	s_set_inst_prefetch_distance 0x2
	ds_load_b32 v1, v7
	s_mov_b32 s3, -1
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, 0x2edbe6ff, v1
	s_cbranch_vccnz .LBB17_20
; %bb.19:                               ;   in Loop: Header=BB17_7 Depth=1
	s_and_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccz .LBB17_6
	s_branch .LBB17_24
.LBB17_20:                              ;   in Loop: Header=BB17_7 Depth=1
	s_and_saveexec_b32 s17, s1
	s_cbranch_execz .LBB17_23
; %bb.21:                               ;   in Loop: Header=BB17_7 Depth=1
	ds_load_b32 v9, v6
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e32 v8, v1, v9
	v_cmp_le_f32_e32 vcc_lo, s7, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_f32_e64 s3, s7, v8
	s_and_b32 s3, vcc_lo, s3
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB17_23
; %bb.22:                               ;   in Loop: Header=BB17_7 Depth=1
	global_load_b32 v10, v[3:4], off
	v_mov_b32_e32 v11, s16
	v_add_f32_e32 v1, s6, v1
	v_add_f32_e32 v9, s6, v9
	ds_load_b32 v11, v11
	ds_load_b32 v12, v5
	s_waitcnt lgkmcnt(1)
	v_dual_add_f32 v8, s6, v8 :: v_dual_mul_f32 v13, v11, v11
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e32 v11, v11, v12
	v_mul_f32_e32 v12, v12, v12
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_scale_f32 v14, null, v1, v1, v13
	v_div_scale_f32 v15, null, v9, v9, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v16, v14
	v_rcp_f32_e32 v18, v15
	s_waitcnt_depctr 0xfff
	v_fma_f32 v20, -v14, v16, 1.0
	v_fma_f32 v21, -v15, v18, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmac_f32_e32 v16, v20, v16
	v_div_scale_f32 v20, vcc_lo, v13, v1, v13
	v_fmac_f32_e32 v18, v21, v18
	v_div_scale_f32 v21, s3, v12, v9, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mul_f32 v23, v20, v16 :: v_dual_mul_f32 v24, v21, v18
	v_fma_f32 v25, -v14, v23, v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v27, -v15, v24, v21
	v_dual_fmac_f32 v23, v25, v16 :: v_dual_fmac_f32 v24, v27, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v14, -v14, v23, v20
	v_fma_f32 v15, -v15, v24, v21
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f32 v14, v14, v16, v23
	s_mov_b32 vcc_lo, s3
	v_div_fmas_f32 v15, v15, v18, v24
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fixup_f32 v1, v14, v1, v13
	v_div_fixup_f32 v9, v15, v9, v12
	v_mul_f32_e32 v11, v11, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f32 v17, null, v8, v8, v11
	v_rcp_f32_e32 v19, v17
	s_waitcnt_depctr 0xfff
	v_fma_f32 v22, -v17, v19, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v19, v22, v19
	v_div_scale_f32 v22, s4, v11, v8, v11
	s_mov_b32 vcc_lo, s4
	v_mul_f32_e32 v26, v22, v19
	v_fma_f32 v25, -v17, v26, v22
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v26, v25, v19
	v_fma_f32 v16, -v17, v26, v22
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v16, v16, v19, v26
	v_div_fixup_f32 v8, v16, v8, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v8, v9, v8
	v_sub_f32_e32 v1, v8, v1
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v1, v10, v1
	global_store_b32 v[3:4], v1, off
.LBB17_23:                              ;   in Loop: Header=BB17_7 Depth=1
	s_or_b32 exec_lo, exec_lo, s17
	s_waitcnt_vscnt null, 0x0
	s_barrier
	s_branch .LBB17_6
.LBB17_24:                              ;   in Loop: Header=BB17_7 Depth=1
	s_barrier
	s_branch .LBB17_6
.LBB17_25:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel oblivious_split_eval_kernel
		.amdhsa_group_segment_fixed_size 2048
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
		.amdhsa_next_free_vgpr 28
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
.Lfunc_end17:
	.size	oblivious_split_eval_kernel, .Lfunc_end17-oblivious_split_eval_kernel
                                        ; -- End function
	.set oblivious_split_eval_kernel.num_vgpr, 28
	.set oblivious_split_eval_kernel.num_agpr, 0
	.set oblivious_split_eval_kernel.numbered_sgpr, 20
	.set oblivious_split_eval_kernel.num_named_barrier, 0
	.set oblivious_split_eval_kernel.private_seg_size, 0
	.set oblivious_split_eval_kernel.uses_vcc, 1
	.set oblivious_split_eval_kernel.uses_flat_scratch, 0
	.set oblivious_split_eval_kernel.has_dyn_sized_stack, 0
	.set oblivious_split_eval_kernel.has_recursion, 0
	.set oblivious_split_eval_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1192
; TotalNumSgprs: 22
; NumVgprs: 28
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 2048 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 22
; NumVGPRsForWavesPerEU: 28
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
	.type	__hip_cuid_44dc31849326c6be,@object ; @__hip_cuid_44dc31849326c6be
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_44dc31849326c6be
__hip_cuid_44dc31849326c6be:
	.byte	0                               ; 0x0
	.size	__hip_cuid_44dc31849326c6be, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_44dc31849326c6be
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
    .name:           _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     26
    .sgpr_spill_count: 0
    .symbol:         _Z22histogram_build_kernelPKdS0_S0_S0_PdS1_S1_iii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     12
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
    .name:           _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         _Z17split_eval_kernelPKdS0_PdS1_iiS0_S0_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     42
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
    .name:           _Z21data_partition_kernelPKdS0_PdS1_iiii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z21data_partition_kernelPKdS0_PdS1_iiii.kd
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
      - .address_space:  global
        .offset:         40
        .size:           8
        .value_kind:     global_buffer
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
    .name:           _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         _Z19tb_histogram_kernelPKdS0_S0_PKiPdS3_iiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     12
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
    .name:           _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i
    .private_segment_fixed_size: 0
    .sgpr_count:     24
    .sgpr_spill_count: 0
    .symbol:         _Z20tb_split_eval_kernelPKdS0_PiS1_iiiS0_S0_i.kd
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
    .name:           _Z21tb_repartition_kernelPKdPiPKiS3_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         _Z21tb_repartition_kernelPKdPiPKiS3_ii.kd
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
    .name:           _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z18tb_leaf_sum_kernelPKdS0_PKiPdS3_i.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     12
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z18tb_leaf_val_kernelPKdS0_PdiiS0_
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z18tb_leaf_val_kernelPKdS0_PdiiS0_.kd
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
    .name:           _Z17tb_scatter_kernelPKiPKdPdi
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         _Z17tb_scatter_kernelPKiPKdPdi.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     5
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
    .name:           _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z20tb_apply_tree_kernelPKdPKiS2_S0_Pdiii.kd
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
      - .address_space:  global
        .offset:         40
        .size:           8
        .value_kind:     global_buffer
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
    .group_segment_fixed_size: 2048
    .kernarg_segment_align: 8
    .kernarg_segment_size: 320
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           oblivious_histogram_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     27
    .sgpr_spill_count: 0
    .symbol:         oblivious_histogram_kernel.kd
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
      - .offset:         28
        .size:           1
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
    .name:           oblivious_route_step_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         oblivious_route_step_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     5
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
    .name:           oblivious_route_step_dev_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         oblivious_route_step_dev_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     5
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           oblivious_route_full_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         oblivious_route_full_kernel.kd
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
    .name:           scatter_add_by_leaf_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         scatter_add_by_leaf_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     5
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
    .name:           leaf_reduce_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         leaf_reduce_kernel.kd
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
    .name:           leaf_finalize_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         leaf_finalize_kernel.kd
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
      - .offset:         28
        .size:           4
        .value_kind:     by_value
      - .offset:         32
        .size:           4
        .value_kind:     by_value
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
    .group_segment_fixed_size: 2048
    .kernarg_segment_align: 8
    .kernarg_segment_size: 312
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           oblivious_split_eval_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         oblivious_split_eval_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     28
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
