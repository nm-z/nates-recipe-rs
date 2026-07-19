	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z22itemset_support_kernelPKdS0_Pdiiii ; -- Begin function _Z22itemset_support_kernelPKdS0_Pdiiii
	.globl	_Z22itemset_support_kernelPKdS0_Pdiiii
	.p2align	8
	.type	_Z22itemset_support_kernelPKdS0_Pdiiii,@function
_Z22itemset_support_kernelPKdS0_Pdiiii: ; @_Z22itemset_support_kernelPKdS0_Pdiiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s6, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_12
; %bb.1:
	s_abs_i32 s2, s6
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_cmp_lt_i32 s7, 1
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
	s_delay_alu instid0(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_mov_b32 s1, 0
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v2, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v2, s6
	v_sub_nc_u32_e32 v0, v1, v0
	s_cbranch_scc1 .LBB0_9
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v3, v0, s7
	s_add_i32 s0, s7, -1
	v_mov_b32_e32 v5, 1
                                        ; implicit-def: $sgpr4
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[6:7], 3, v[3:4]
	v_mul_lo_u32 v3, v2, s5
	v_mov_b32_e32 v4, s0
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v1, vcc_lo, s10, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s11, v7, vcc_lo
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB0_5
	.p2align	6
.LBB0_3:                                ;   in Loop: Header=BB0_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_not1_b32 s4, s4, exec_lo
	s_and_b32 s6, s6, exec_lo
	s_or_b32 s4, s4, s6
.LBB0_4:                                ;   in Loop: Header=BB0_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s0
	v_sub_co_u32 v4, s0, v4, 1
	v_add_co_u32 v1, vcc_lo, v1, 8
	s_or_b32 s0, s4, s0
	v_add_co_ci_u32_e64 v2, null, 0, v2, vcc_lo
	v_mov_b32_e32 v5, v6
	s_and_b32 s0, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_or_b32 s1, s0, s1
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB0_8
.LBB0_5:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[6:7], v[1:2], off
	s_or_b32 s4, s4, exec_lo
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v7, v[6:7]
	v_mov_b32_e32 v6, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_lt_i32_e32 vcc_lo, -1, v7
	v_cmp_gt_i32_e64 s0, s5, v7
	s_and_b32 s6, vcc_lo, s0
	s_and_saveexec_b32 s0, s6
	s_cbranch_execz .LBB0_4
; %bb.6:                                ;   in Loop: Header=BB0_5 Depth=1
	v_add_nc_u32_e32 v6, v3, v7
	s_mov_b32 s6, -1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v7, 31, v6
	v_lshlrev_b64 v[6:7], 3, v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s8, v6
	v_add_co_ci_u32_e64 v7, null, s9, v7, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[6:7]
	v_mov_b32_e32 v6, 0
	s_and_saveexec_b32 s7, vcc_lo
	s_cbranch_execz .LBB0_3
; %bb.7:                                ;   in Loop: Header=BB0_5 Depth=1
	v_mov_b32_e32 v6, v5
	s_xor_b32 s6, exec_lo, -1
	s_branch .LBB0_3
.LBB0_8:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s1
	v_cmp_ne_u32_e64 s0, 0, v6
	s_and_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB0_10
	s_branch .LBB0_12
.LBB0_9:
	s_mov_b32 s0, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 exec_lo, exec_lo, s0
	s_cbranch_execz .LBB0_12
.LBB0_10:
	v_ashrrev_i32_e32 v1, 31, v0
	s_mov_b32 s0, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v4, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[4:5], off
.LBB0_11:                               ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], 1.0
	global_atomic_cmpswap_b64 v[0:1], v[4:5], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB0_11
.LBB0_12:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22itemset_support_kernelPKdS0_Pdiiii
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
	.size	_Z22itemset_support_kernelPKdS0_Pdiiii, .Lfunc_end0-_Z22itemset_support_kernelPKdS0_Pdiiii
                                        ; -- End function
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.num_vgpr, 8
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.num_agpr, 0
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.numbered_sgpr, 12
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.num_named_barrier, 0
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.private_seg_size, 0
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.uses_vcc, 1
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.uses_flat_scratch, 0
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.has_dyn_sized_stack, 0
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.has_recursion, 0
	.set _Z22itemset_support_kernelPKdS0_Pdiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 716
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
	.protected	_Z31candidate_generate_count_kernelPKdiiPi ; -- Begin function _Z31candidate_generate_count_kernelPKdiiPi
	.globl	_Z31candidate_generate_count_kernelPKdiiPi
	.p2align	8
	.type	_Z31candidate_generate_count_kernelPKdiiPi,@function
_Z31candidate_generate_count_kernelPKdiiPi: ; @_Z31candidate_generate_count_kernelPKdiiPi
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_add_i32 s6, s4, -1
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s6, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshr_b32 s3, s2, 31
	s_add_i32 s2, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s2, s2, 1
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_18
; %bb.1:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v4, s4
	s_mov_b32 s3, 0
	s_max_i32 s2, s6, 0
                                        ; implicit-def: $sgpr4
                                        ; implicit-def: $sgpr6
	s_branch .LBB1_4
	.p2align	6
.LBB1_2:                                ;   in Loop: Header=BB1_4 Depth=1
	v_add_nc_u32_e32 v4, -1, v4
	v_add_nc_u32_e32 v2, 1, v0
	s_and_not1_b32 s6, s6, exec_lo
	s_mov_b32 s7, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_lt_i32_e32 vcc_lo, v3, v4
	v_sub_nc_u32_e32 v1, v3, v4
	s_and_b32 s8, vcc_lo, exec_lo
	s_or_b32 s6, s6, s8
.LBB1_3:                                ;   in Loop: Header=BB1_4 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s8, exec_lo, s6
	s_or_b32 s3, s8, s3
	s_and_not1_b32 s4, s4, exec_lo
	s_and_b32 s7, s7, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 s4, s4, s7
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB1_6
.LBB1_4:                                ; =>This Inner Loop Header: Depth=1
	v_dual_mov_b32 v0, v2 :: v_dual_mov_b32 v3, v1
	s_mov_b32 s7, -1
	s_or_b32 s6, s6, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, s2, v0
	s_cbranch_vccz .LBB1_2
; %bb.5:                                ;   in Loop: Header=BB1_4 Depth=1
                                        ; implicit-def: $vgpr1
                                        ; implicit-def: $vgpr4
                                        ; implicit-def: $vgpr2
	s_branch .LBB1_3
.LBB1_6:
	s_or_b32 exec_lo, exec_lo, s3
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v4, s2
	s_mov_b32 s3, -1
	s_xor_b32 s2, s4, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_saveexec_b32 s4, s2
	s_xor_b32 s2, exec_lo, s4
; %bb.7:
	v_dual_mov_b32 v4, v0 :: v_dual_add_nc_u32 v1, v2, v3
; %bb.8:
	s_or_b32 exec_lo, exec_lo, s2
	s_load_b64 s[6:7], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v0, v4, s5
	v_mul_lo_u32 v2, v1, s5
	s_add_i32 s2, s5, -1
	s_cmp_lt_i32 s5, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v1, 31, v0
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	v_lshlrev_b64 v[4:5], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s6, v4
	v_add_co_ci_u32_e64 v1, null, s7, v5, vcc_lo
	s_cbranch_scc1 .LBB1_14
; %bb.9:
	v_dual_mov_b32 v5, v1 :: v_dual_mov_b32 v4, v0
	v_dual_mov_b32 v7, v3 :: v_dual_mov_b32 v6, v2
	s_mov_b32 s4, 0
	s_mov_b32 s5, s2
                                        ; implicit-def: $sgpr3
                                        ; implicit-def: $sgpr7
                                        ; implicit-def: $sgpr6
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB1_11
	.p2align	6
.LBB1_10:                               ;   in Loop: Header=BB1_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s8
	s_xor_b32 s8, s6, -1
	s_and_b32 s9, exec_lo, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_or_b32 s4, s9, s4
	s_and_not1_b32 s3, s3, exec_lo
	s_and_b32 s8, s8, exec_lo
	s_or_b32 s3, s3, s8
	s_and_not1_b32 exec_lo, exec_lo, s4
	s_cbranch_execz .LBB1_13
.LBB1_11:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[6:7], off
	global_load_b64 v[10:11], v[4:5], off
	s_or_b32 s6, s6, exec_lo
	s_or_b32 s7, s7, exec_lo
	s_mov_b32 s8, exec_lo
	s_waitcnt vmcnt(1)
	v_cvt_i32_f64_e32 v8, v[8:9]
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v9, v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e64 v8, v9
	s_cbranch_execz .LBB1_10
; %bb.12:                               ;   in Loop: Header=BB1_11 Depth=1
	s_add_i32 s5, s5, -1
	v_add_co_u32 v6, vcc_lo, v6, 8
	s_cmp_eq_u32 s5, 0
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v4, 8
	s_cselect_b32 s9, -1, 0
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_and_not1_b32 s7, s7, exec_lo
	s_and_b32 s9, s9, exec_lo
	s_and_not1_b32 s6, s6, exec_lo
	s_or_b32 s7, s7, s9
	s_branch .LBB1_10
.LBB1_13:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s4
.LBB1_14:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB1_18
; %bb.15:
	s_ashr_i32 s3, s2, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[2:3], s[2:3], 3
	v_add_co_u32 v2, vcc_lo, v2, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, s2
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_clause 0x1
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[0:1], v[0:1], off
	s_waitcnt vmcnt(1)
	v_cvt_i32_f64_e32 v2, v[2:3]
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v0, v[0:1]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_lt_i32_e32 vcc_lo, v2, v0
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB1_18
; %bb.16:
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mbcnt_lo_u32_b32 v0, s2, 0
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_and_b32 s3, exec_lo, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 exec_lo, s3
	s_cbranch_execz .LBB1_18
; %bb.17:
	s_load_b64 s[0:1], s[0:1], 0x10
	s_bcnt1_i32_b32 s2, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s2
	s_waitcnt lgkmcnt(0)
	global_atomic_add_u32 v0, v1, s[0:1]
.LBB1_18:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z31candidate_generate_count_kernelPKdiiPi
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
		.amdhsa_next_free_vgpr 12
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
.Lfunc_end1:
	.size	_Z31candidate_generate_count_kernelPKdiiPi, .Lfunc_end1-_Z31candidate_generate_count_kernelPKdiiPi
                                        ; -- End function
	.set _Z31candidate_generate_count_kernelPKdiiPi.num_vgpr, 12
	.set _Z31candidate_generate_count_kernelPKdiiPi.num_agpr, 0
	.set _Z31candidate_generate_count_kernelPKdiiPi.numbered_sgpr, 10
	.set _Z31candidate_generate_count_kernelPKdiiPi.num_named_barrier, 0
	.set _Z31candidate_generate_count_kernelPKdiiPi.private_seg_size, 0
	.set _Z31candidate_generate_count_kernelPKdiiPi.uses_vcc, 1
	.set _Z31candidate_generate_count_kernelPKdiiPi.uses_flat_scratch, 0
	.set _Z31candidate_generate_count_kernelPKdiiPi.has_dyn_sized_stack, 0
	.set _Z31candidate_generate_count_kernelPKdiiPi.has_recursion, 0
	.set _Z31candidate_generate_count_kernelPKdiiPi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 756
; TotalNumSgprs: 12
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 12
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
	.protected	_Z31candidate_generate_write_kernelPKdPdiiPi ; -- Begin function _Z31candidate_generate_write_kernelPKdPdiiPi
	.globl	_Z31candidate_generate_write_kernelPKdPdiiPi
	.p2align	8
	.type	_Z31candidate_generate_write_kernelPKdPdiiPi,@function
_Z31candidate_generate_write_kernelPKdPdiiPi: ; @_Z31candidate_generate_write_kernelPKdPdiiPi
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[8:9], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_add_i32 s4, s8, -1
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s4, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshr_b32 s3, s2, 31
	s_add_i32 s2, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s2, s2, 1
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB2_22
; %bb.1:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v4, s8
	s_mov_b32 s3, 0
	s_max_i32 s2, s4, 0
                                        ; implicit-def: $sgpr4
                                        ; implicit-def: $sgpr5
	s_branch .LBB2_4
	.p2align	6
.LBB2_2:                                ;   in Loop: Header=BB2_4 Depth=1
	v_add_nc_u32_e32 v4, -1, v4
	v_add_nc_u32_e32 v2, 1, v0
	s_and_not1_b32 s5, s5, exec_lo
	s_mov_b32 s6, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_lt_i32_e32 vcc_lo, v3, v4
	v_sub_nc_u32_e32 v1, v3, v4
	s_and_b32 s7, vcc_lo, exec_lo
	s_or_b32 s5, s5, s7
.LBB2_3:                                ;   in Loop: Header=BB2_4 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s7, exec_lo, s5
	s_or_b32 s3, s7, s3
	s_and_not1_b32 s4, s4, exec_lo
	s_and_b32 s6, s6, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 s4, s4, s6
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB2_6
.LBB2_4:                                ; =>This Inner Loop Header: Depth=1
	v_dual_mov_b32 v0, v2 :: v_dual_mov_b32 v3, v1
	s_mov_b32 s6, -1
	s_or_b32 s5, s5, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, s2, v0
	s_cbranch_vccz .LBB2_2
; %bb.5:                                ;   in Loop: Header=BB2_4 Depth=1
                                        ; implicit-def: $vgpr1
                                        ; implicit-def: $vgpr4
                                        ; implicit-def: $vgpr2
	s_branch .LBB2_3
.LBB2_6:
	s_or_b32 exec_lo, exec_lo, s3
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v4, s2
	s_mov_b32 s3, -1
	s_xor_b32 s2, s4, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_saveexec_b32 s4, s2
	s_xor_b32 s2, exec_lo, s4
; %bb.7:
	v_dual_mov_b32 v4, v0 :: v_dual_add_nc_u32 v1, v2, v3
; %bb.8:
	s_or_b32 exec_lo, exec_lo, s2
	s_load_b128 s[4:7], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v0, v4, s9
	v_mul_lo_u32 v2, v1, s9
	s_add_i32 s2, s9, -1
	s_cmp_lt_i32 s9, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v1, 31, v0
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v2
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	s_cbranch_scc1 .LBB2_14
; %bb.9:
	v_dual_mov_b32 v5, v3 :: v_dual_mov_b32 v4, v2
	v_dual_mov_b32 v7, v1 :: v_dual_mov_b32 v6, v0
	s_mov_b32 s4, 0
	s_mov_b32 s5, s2
                                        ; implicit-def: $sgpr3
                                        ; implicit-def: $sgpr10
                                        ; implicit-def: $sgpr8
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB2_11
	.p2align	6
.LBB2_10:                               ;   in Loop: Header=BB2_11 Depth=1
	s_or_b32 exec_lo, exec_lo, s11
	s_xor_b32 s11, s8, -1
	s_and_b32 s12, exec_lo, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_or_b32 s4, s12, s4
	s_and_not1_b32 s3, s3, exec_lo
	s_and_b32 s11, s11, exec_lo
	s_or_b32 s3, s3, s11
	s_and_not1_b32 exec_lo, exec_lo, s4
	s_cbranch_execz .LBB2_13
.LBB2_11:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[6:7], off
	global_load_b64 v[10:11], v[4:5], off
	s_or_b32 s8, s8, exec_lo
	s_or_b32 s10, s10, exec_lo
	s_mov_b32 s11, exec_lo
	s_waitcnt vmcnt(1)
	v_cvt_i32_f64_e32 v8, v[8:9]
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v9, v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e64 v8, v9
	s_cbranch_execz .LBB2_10
; %bb.12:                               ;   in Loop: Header=BB2_11 Depth=1
	s_add_i32 s5, s5, -1
	v_add_co_u32 v6, vcc_lo, v6, 8
	s_cmp_eq_u32 s5, 0
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v4, 8
	s_cselect_b32 s12, -1, 0
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_and_not1_b32 s10, s10, exec_lo
	s_and_b32 s12, s12, exec_lo
	s_and_not1_b32 s8, s8, exec_lo
	s_or_b32 s10, s10, s12
	s_branch .LBB2_10
.LBB2_13:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s4
.LBB2_14:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB2_22
; %bb.15:
	s_ashr_i32 s3, s2, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[2:3], s[2:3], 3
	v_add_co_u32 v4, vcc_lo, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s3, v1, vcc_lo
	v_add_co_u32 v2, vcc_lo, v2, s2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_clause 0x1
	global_load_b64 v[4:5], v[4:5], off
	global_load_b64 v[6:7], v[2:3], off
	s_waitcnt vmcnt(1)
	v_cvt_i32_f64_e32 v4, v[4:5]
	s_waitcnt vmcnt(0)
	v_cvt_i32_f64_e32 v5, v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_lt_i32_e32 vcc_lo, v4, v5
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB2_22
; %bb.16:
	s_mov_b32 s3, exec_lo
	s_mov_b32 s2, exec_lo
	v_mbcnt_lo_u32_b32 v4, s3, 0
                                        ; implicit-def: $vgpr5
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v4
	s_cbranch_execz .LBB2_18
; %bb.17:
	s_load_b64 s[0:1], s[0:1], 0x18
	s_bcnt1_i32_b32 s3, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v6, s3
	s_waitcnt lgkmcnt(0)
	global_atomic_add_u32 v5, v5, v6, s[0:1] glc
.LBB2_18:
	s_or_b32 exec_lo, exec_lo, s2
	s_waitcnt vmcnt(0)
	v_readfirstlane_b32 s0, v5
	s_cmp_lt_i32 s9, 1
	v_add_nc_u32_e32 v4, s0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[5:6], null, v4, s9, v[4:5]
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 3, v[5:6]
	v_add_co_u32 v4, vcc_lo, s6, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
	s_cbranch_scc1 .LBB2_21
; %bb.19:
	v_dual_mov_b32 v7, v5 :: v_dual_mov_b32 v6, v4
	s_mov_b32 s0, s9
.LBB2_20:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[0:1], off
	v_add_co_u32 v0, vcc_lo, v0, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_add_i32 s0, s0, -1
	s_cmp_eq_u32 s0, 0
	s_waitcnt vmcnt(0)
	global_store_b64 v[6:7], v[8:9], off
	v_add_co_u32 v6, vcc_lo, v6, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	s_cbranch_scc0 .LBB2_20
.LBB2_21:
	global_load_b64 v[0:1], v[2:3], off
	s_ashr_i32 s1, s9, 31
	s_mov_b32 s0, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[0:1], s[0:1], 3
	v_add_co_u32 v2, vcc_lo, v4, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v5, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[2:3], v[0:1], off
.LBB2_22:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z31candidate_generate_write_kernelPKdPdiiPi
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
		.amdhsa_next_free_vgpr 12
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
.Lfunc_end2:
	.size	_Z31candidate_generate_write_kernelPKdPdiiPi, .Lfunc_end2-_Z31candidate_generate_write_kernelPKdPdiiPi
                                        ; -- End function
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.num_vgpr, 12
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.num_agpr, 0
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.numbered_sgpr, 13
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.num_named_barrier, 0
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.private_seg_size, 0
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.uses_vcc, 1
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.uses_flat_scratch, 0
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.has_dyn_sized_stack, 0
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.has_recursion, 0
	.set _Z31candidate_generate_write_kernelPKdPdiiPi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 960
; TotalNumSgprs: 15
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 15
; NumVGPRsForWavesPerEU: 12
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
	.type	__hip_cuid_dac963c36f46779,@object ; @__hip_cuid_dac963c36f46779
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_dac963c36f46779
__hip_cuid_dac963c36f46779:
	.byte	0                               ; 0x0
	.size	__hip_cuid_dac963c36f46779, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_dac963c36f46779
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
    .name:           _Z22itemset_support_kernelPKdS0_Pdiiii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z22itemset_support_kernelPKdS0_Pdiiii.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .offset:         12
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
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
    .name:           _Z31candidate_generate_count_kernelPKdiiPi
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         _Z31candidate_generate_count_kernelPKdiiPi.kd
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
    .name:           _Z31candidate_generate_write_kernelPKdPdiiPi
    .private_segment_fixed_size: 0
    .sgpr_count:     15
    .sgpr_spill_count: 0
    .symbol:         _Z31candidate_generate_write_kernelPKdPdiiPi.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     12
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
