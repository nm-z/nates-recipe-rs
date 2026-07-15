	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z25discounted_returns_kernelPKdPdS0_i ; -- Begin function _Z25discounted_returns_kernelPKdPdS0_i
	.globl	_Z25discounted_returns_kernelPKdPdS0_i
	.p2align	8
	.type	_Z25discounted_returns_kernelPKdPdS0_i,@function
_Z25discounted_returns_kernelPKdPdS0_i: ; @_Z25discounted_returns_kernelPKdPdS0_i
; %bb.0:
	s_load_b32 s4, s[0:1], 0x18
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s3, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB0_3
; %bb.1:
	s_clause 0x1
	s_load_b64 s[6:7], s[0:1], 0x10
	s_load_b128 s[8:11], s[0:1], 0x0
	s_add_i32 s2, s4, -1
	v_mov_b32_e32 v0, 0
	s_lshl_b64 s[12:13], s[2:3], 3
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, 0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[6:7], 0x0
	s_add_i32 s6, s4, 1
	s_add_u32 s2, s8, s12
	s_addc_u32 s3, s9, s13
	s_add_u32 s4, s10, s12
	s_addc_u32 s5, s11, s13
.LBB0_2:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[3:4], v2, s[2:3]
	s_add_i32 s6, s6, -1
	s_add_u32 s2, s2, -8
	s_addc_u32 s3, s3, -1
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[0:1], s[0:1], v[0:1], v[3:4]
	global_store_b64 v2, v[0:1], s[4:5]
	s_add_u32 s4, s4, -8
	s_addc_u32 s5, s5, -1
	s_cmp_gt_u32 s6, 1
	s_cbranch_scc1 .LBB0_2
.LBB0_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z25discounted_returns_kernelPKdPdS0_i
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 28
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
.Lfunc_end0:
	.size	_Z25discounted_returns_kernelPKdPdS0_i, .Lfunc_end0-_Z25discounted_returns_kernelPKdPdS0_i
                                        ; -- End function
	.set _Z25discounted_returns_kernelPKdPdS0_i.num_vgpr, 5
	.set _Z25discounted_returns_kernelPKdPdS0_i.num_agpr, 0
	.set _Z25discounted_returns_kernelPKdPdS0_i.numbered_sgpr, 14
	.set _Z25discounted_returns_kernelPKdPdS0_i.num_named_barrier, 0
	.set _Z25discounted_returns_kernelPKdPdS0_i.private_seg_size, 0
	.set _Z25discounted_returns_kernelPKdPdS0_i.uses_vcc, 1
	.set _Z25discounted_returns_kernelPKdPdS0_i.uses_flat_scratch, 0
	.set _Z25discounted_returns_kernelPKdPdS0_i.has_dyn_sized_stack, 0
	.set _Z25discounted_returns_kernelPKdPdS0_i.has_recursion, 0
	.set _Z25discounted_returns_kernelPKdPdS0_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 184
; TotalNumSgprs: 16
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 16
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
	.protected	_Z10gae_kernelPKdS0_PdS0_S0_i ; -- Begin function _Z10gae_kernelPKdS0_PdS0_S0_i
	.globl	_Z10gae_kernelPKdS0_PdS0_S0_i
	.p2align	8
	.type	_Z10gae_kernelPKdS0_PdS0_S0_i,@function
_Z10gae_kernelPKdS0_PdS0_S0_i:          ; @_Z10gae_kernelPKdS0_PdS0_S0_i
; %bb.0:
	s_load_b32 s4, s[0:1], 0x28
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s5, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB1_5
; %bb.1:
	s_clause 0x1
	s_load_b256 s[8:15], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x20
	s_mov_b32 s7, s5
	v_mov_b32_e32 v2, 0
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v6, 0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[14:15], 0x0
	s_load_b64 s[2:3], s[2:3], 0x0
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[0:1], s[0:1], s[2:3]
	s_lshl_b64 s[2:3], s[4:5], 3
	s_mov_b32 s5, s4
	s_add_u32 s2, s10, s2
	s_addc_u32 s3, s11, s3
	s_add_i32 s6, s4, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[14:15], s[6:7], 3
	s_add_u32 s6, s8, s14
	s_addc_u32 s7, s9, s15
	s_add_u32 s8, s10, s14
	s_addc_u32 s9, s11, s15
	s_add_u32 s10, s12, s14
	s_addc_u32 s11, s13, s15
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB1_3
	.p2align	6
.LBB1_2:                                ;   in Loop: Header=BB1_3 Depth=1
	s_clause 0x1
	global_load_b64 v[7:8], v6, s[6:7]
	global_load_b64 v[9:10], v6, s[8:9]
	s_add_i32 s12, s5, -1
	s_add_u32 s2, s2, -8
	s_addc_u32 s3, s3, -1
	s_add_u32 s6, s6, -8
	s_addc_u32 s7, s7, -1
	s_add_u32 s8, s8, -8
	s_addc_u32 s9, s9, -1
	s_waitcnt vmcnt(1)
	v_fma_f64 v[4:5], s[0:1], v[4:5], v[7:8]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[9:10]
	v_fma_f64 v[2:3], v[0:1], v[2:3], v[4:5]
	global_store_b64 v6, v[2:3], s[10:11]
	s_add_u32 s10, s10, -8
	s_addc_u32 s11, s11, -1
	s_cmp_gt_u32 s5, 1
	s_mov_b32 s5, s12
	s_cbranch_scc0 .LBB1_5
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0
	s_cmp_ge_i32 s5, s4
	s_cbranch_scc1 .LBB1_2
; %bb.4:                                ;   in Loop: Header=BB1_3 Depth=1
	global_load_b64 v[4:5], v6, s[2:3]
	s_branch .LBB1_2
.LBB1_5:
	s_set_inst_prefetch_distance 0x2
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z10gae_kernelPKdS0_PdS0_S0_i
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 44
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
	.size	_Z10gae_kernelPKdS0_PdS0_S0_i, .Lfunc_end1-_Z10gae_kernelPKdS0_PdS0_S0_i
                                        ; -- End function
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.num_vgpr, 11
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.num_agpr, 0
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.numbered_sgpr, 16
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.num_named_barrier, 0
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.private_seg_size, 0
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.uses_vcc, 1
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.uses_flat_scratch, 0
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.has_dyn_sized_stack, 0
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.has_recursion, 0
	.set _Z10gae_kernelPKdS0_PdS0_S0_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 328
; TotalNumSgprs: 18
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
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
	.text
	.protected	_Z17td_targets_kernelPKdS0_S0_PdS0_i ; -- Begin function _Z17td_targets_kernelPKdS0_S0_PdS0_i
	.globl	_Z17td_targets_kernelPKdS0_S0_PdS0_i
	.p2align	8
	.type	_Z17td_targets_kernelPKdS0_S0_PdS0_i,@function
_Z17td_targets_kernelPKdS0_S0_PdS0_i:   ; @_Z17td_targets_kernelPKdS0_S0_PdS0_i
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
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s8, v0
	v_add_co_ci_u32_e64 v5, null, s9, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_u32 v6, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s5, v1, vcc_lo
	s_load_b64 s[0:1], s[0:1], 0x0
	v_add_co_u32 v0, vcc_lo, s10, v0
	global_load_b64 v[6:7], v[6:7], off
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	s_waitcnt vmcnt(2) lgkmcnt(0)
	v_mul_f64 v[2:3], s[0:1], v[2:3]
	s_waitcnt vmcnt(1)
	v_add_f64 v[4:5], -v[4:5], 1.0
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[2:3], v[2:3], v[4:5], v[6:7]
	global_store_b64 v[0:1], v[2:3], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z17td_targets_kernelPKdS0_S0_PdS0_i
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
.Lfunc_end2:
	.size	_Z17td_targets_kernelPKdS0_S0_PdS0_i, .Lfunc_end2-_Z17td_targets_kernelPKdS0_S0_PdS0_i
                                        ; -- End function
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.num_vgpr, 8
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.num_agpr, 0
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.numbered_sgpr, 12
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.num_named_barrier, 0
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.private_seg_size, 0
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.uses_vcc, 1
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.uses_flat_scratch, 0
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.has_dyn_sized_stack, 0
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.has_recursion, 0
	.set _Z17td_targets_kernelPKdS0_S0_PdS0_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 252
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
	.protected	_Z26categorical_logprob_kernelPKdPKiPdii ; -- Begin function _Z26categorical_logprob_kernelPKdPKiPdii
	.globl	_Z26categorical_logprob_kernelPKdPKiPdii
	.p2align	8
	.type	_Z26categorical_logprob_kernelPKdPKiPdii,@function
_Z26categorical_logprob_kernelPKdPKiPdii: ; @_Z26categorical_logprob_kernelPKdPKiPdii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB3_8
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_mul_lo_u32 v2, v1, s9
	s_cmp_lt_i32 s9, 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[7:8], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s4, v7
	v_add_co_ci_u32_e64 v6, null, s5, v8, vcc_lo
	global_load_b64 v[3:4], v[5:6], off
	s_cbranch_scc1 .LBB3_4
; %bb.2:
	v_add_co_u32 v0, vcc_lo, s4, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s5, v8, vcc_lo
	s_add_i32 s1, s9, -1
	v_add_co_u32 v7, vcc_lo, v0, 8
	v_add_co_ci_u32_e64 v8, null, 0, v2, vcc_lo
.LBB3_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[9:10], v[7:8], off
	v_add_co_u32 v7, s0, v7, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v8, null, 0, v8, s0
	s_add_i32 s1, s1, -1
	s_cmp_eq_u32 s1, 0
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[9:10], v[3:4]
	v_dual_cndmask_b32 v4, v4, v10 :: v_dual_cndmask_b32 v3, v3, v9
	s_cbranch_scc0 .LBB3_3
.LBB3_4:
	v_mov_b32_e32 v7, 0
	v_mov_b32_e32 v8, 0
	s_cmp_lt_i32 s9, 1
	s_cbranch_scc1 .LBB3_7
; %bb.5:
	v_dual_mov_b32 v10, v6 :: v_dual_mov_b32 v9, v5
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s10, 0xfefa39ef
	s_mov_b32 s12, 0x3b39803f
	s_mov_b32 s14, 0xfca7ab0c
	s_mov_b32 s16, 0x6a5dcb37
	s_mov_b32 s18, 0x623fde64
	s_mov_b32 s20, 0x7c89e6b0
	s_mov_b32 s22, 0x14761f6e
	s_mov_b32 s24, 0x1852b7b0
	s_mov_b32 s26, 0x11122322
	s_mov_b32 s28, 0x555502a1
	s_mov_b32 s30, 0x55555511
	s_mov_b32 s34, 11
	s_mov_b32 s5, 0x3ff71547
	s_mov_b32 s11, 0xbfe62e42
	s_mov_b32 s13, 0xbc7abc9e
	s_mov_b32 s15, 0x3e928af3
	s_mov_b32 s17, 0x3e5ade15
	s_mov_b32 s19, 0x3ec71dee
	s_mov_b32 s21, 0x3efa0199
	s_mov_b32 s23, 0x3f2a01a0
	s_mov_b32 s25, 0x3f56c16c
	s_mov_b32 s27, 0x3f811111
	s_mov_b32 s29, 0x3fa55555
	s_mov_b32 s31, 0x3fc55555
	s_mov_b32 s35, 0x3fe00000
.LBB3_6:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[11:12], v[9:10], off
	s_add_i32 s9, s9, -1
	s_waitcnt vmcnt(0)
	v_add_f64 v[11:12], v[11:12], -v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[13:14], v[11:12], s[4:5]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[11:12]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[11:12]
	v_rndne_f64_e32 v[13:14], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[13:14], s[10:11], v[11:12]
	v_cvt_i32_f64_e32 v0, v[13:14]
	v_fma_f64 v[15:16], v[13:14], s[12:13], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], s[16:17], s[14:15]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[18:19]
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
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	v_ldexp_f64 v[13:14], v[13:14], v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, 0x7ff00000, v14, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_cmp_eq_u32 s9, 0
	v_cndmask_b32_e32 v11, 0, v13, vcc_lo
	v_add_co_u32 v9, vcc_lo, v9, 8
	v_cndmask_b32_e64 v12, 0, v0, s0
	v_add_co_ci_u32_e64 v10, null, 0, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[11:12]
	s_cbranch_scc0 .LBB3_6
.LBB3_7:
	s_delay_alu instid0(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[9:10], v[7:8]
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[9:10]
	s_mov_b32 s0, 0x55555780
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_ldexp_f64 v[9:10], v[9:10], v0
	v_frexp_exp_i32_f64_e32 v0, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[9:10], 1.0
	v_add_f64 v[17:18], v[9:10], -1.0
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[13:14], v[11:12]
	v_add_f64 v[19:20], v[11:12], -1.0
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[15:16], -v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[15:16], v[13:14], v[13:14]
	v_fma_f64 v[15:16], -v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[15:16], v[13:14], v[13:14]
	v_mul_f64 v[15:16], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[21:22], v[11:12], v[15:16]
	v_fma_f64 v[11:12], v[15:16], v[11:12], -v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[15:16], v[9:10], v[11:12]
	v_add_f64 v[11:12], v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[17:18], -v[11:12]
	v_add_f64 v[21:22], v[11:12], -v[21:22]
	v_add_f64 v[17:18], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[21:22], -v[9:10]
	v_cvt_f64_i32_e32 v[21:22], v0
	v_add_f64 v[11:12], v[17:18], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[9:10], v[11:12]
	v_add_f64 v[9:10], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[13:14], v[9:10]
	v_add_f64 v[11:12], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[13:14], v[11:12], v[11:12]
	v_fma_f64 v[17:18], v[13:14], s[8:9], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[19:20], v[11:12], v[13:14]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[13:14], v[17:18], s[0:1]
	v_ldexp_f64 v[17:18], v[11:12], 1
	v_add_f64 v[11:12], v[11:12], -v[15:16]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[23:24], v[21:22], s[0:1]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_mul_f64 v[13:14], v[19:20], v[13:14]
	v_lshlrev_b64 v[19:20], 2, v[1:2]
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v19, vcc_lo, s6, v19
	v_add_co_ci_u32_e64 v20, null, s7, v20, vcc_lo
	global_load_b32 v19, v[19:20], off
	v_add_f64 v[15:16], v[17:18], v[13:14]
	v_ldexp_f64 v[9:10], v[9:10], 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], -v[17:18]
	v_fma_f64 v[17:18], v[21:22], s[0:1], -v[23:24]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[21:22], s[0:1], v[17:18]
	v_add_f64 v[9:10], v[9:10], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[23:24], v[13:14]
	v_add_f64 v[17:18], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[23:24], v[11:12], -v[23:24]
	v_add_f64 v[21:22], v[11:12], v[17:18]
	v_add_f64 v[15:16], v[17:18], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[23:24]
	v_add_f64 v[25:26], v[21:22], -v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[27:28], v[21:22], -v[25:26]
	v_add_f64 v[15:16], v[17:18], -v[25:26]
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v20, 31, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[17:18], 3, v[19:20]
	v_add_f64 v[11:12], v[11:12], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, v5, v17
	v_add_co_ci_u32_e64 v6, null, v6, v18, vcc_lo
	v_add_f64 v[17:18], v[13:14], v[9:10]
	v_cmp_class_f64_e64 vcc_lo, v[7:8], 0x204
	global_load_b64 v[5:6], v[5:6], off
	v_add_f64 v[11:12], v[15:16], v[11:12]
	v_add_f64 v[15:16], v[17:18], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[17:18], v[11:12]
	v_add_f64 v[17:18], v[17:18], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[21:22], v[11:12]
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[19:20], -v[21:22]
	v_add_f64 v[9:10], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[19:20], v[9:10]
	v_dual_cndmask_b32 v0, v9, v7 :: v_dual_cndmask_b32 v9, v10, v8
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v10, 0x7ff80000, v9, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[7:8]
	v_cndmask_b32_e32 v9, 0, v0, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[7:8]
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_cndmask_b32_e32 v10, 0xfff00000, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[9:10]
	s_waitcnt vmcnt(0)
	v_add_f64 v[3:4], v[5:6], -v[3:4]
	global_store_b64 v[0:1], v[3:4], off
.LBB3_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z26categorical_logprob_kernelPKdPKiPdii
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
		.amdhsa_next_free_sgpr 36
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
.Lfunc_end3:
	.size	_Z26categorical_logprob_kernelPKdPKiPdii, .Lfunc_end3-_Z26categorical_logprob_kernelPKdPKiPdii
                                        ; -- End function
	.set _Z26categorical_logprob_kernelPKdPKiPdii.num_vgpr, 29
	.set _Z26categorical_logprob_kernelPKdPKiPdii.num_agpr, 0
	.set _Z26categorical_logprob_kernelPKdPKiPdii.numbered_sgpr, 36
	.set _Z26categorical_logprob_kernelPKdPKiPdii.num_named_barrier, 0
	.set _Z26categorical_logprob_kernelPKdPKiPdii.private_seg_size, 0
	.set _Z26categorical_logprob_kernelPKdPKiPdii.uses_vcc, 1
	.set _Z26categorical_logprob_kernelPKdPKiPdii.uses_flat_scratch, 0
	.set _Z26categorical_logprob_kernelPKdPKiPdii.has_dyn_sized_stack, 0
	.set _Z26categorical_logprob_kernelPKdPKiPdii.has_recursion, 0
	.set _Z26categorical_logprob_kernelPKdPKiPdii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1792
; TotalNumSgprs: 38
; NumVgprs: 29
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 38
; NumVGPRsForWavesPerEU: 29
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
	.protected	_Z23gaussian_logprob_kernelPKdS0_S0_Pdii ; -- Begin function _Z23gaussian_logprob_kernelPKdS0_S0_Pdii
	.globl	_Z23gaussian_logprob_kernelPKdS0_S0_Pdii
	.p2align	8
	.type	_Z23gaussian_logprob_kernelPKdS0_S0_Pdii,@function
_Z23gaussian_logprob_kernelPKdS0_S0_Pdii: ; @_Z23gaussian_logprob_kernelPKdS0_S0_Pdii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b64 s[8:9], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB4_6
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	s_cmp_lt_i32 s9, 1
	s_cbranch_scc1 .LBB4_4
; %bb.2:
	v_mul_lo_u32 v5, v1, s9
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_mov_b32 s10, 0x652b82fe
	s_mov_b32 s12, 0xfefa39ef
	s_mov_b32 s14, 0x6a5dcb37
	s_mov_b32 s16, 0x623fde64
	s_mov_b32 s18, 0x7c89e6b0
	v_ashrrev_i32_e32 v6, 31, v5
	s_mov_b32 s20, 0x14761f6e
	s_mov_b32 s22, 0x1852b7b0
	s_mov_b32 s24, 0x11122322
	s_mov_b32 s26, 0x555502a1
	v_lshlrev_b64 v[9:10], 3, v[5:6]
	s_mov_b32 s28, 0x55555511
	s_mov_b32 s30, 11
	s_mov_b32 s34, 0xc864beb4
	s_mov_b32 s11, 0x3ff71547
	s_mov_b32 s13, 0xbfe62e42
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v5, vcc_lo, s0, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s1, v10, vcc_lo
	v_add_co_u32 v7, vcc_lo, s2, v9
	v_add_co_ci_u32_e64 v8, null, s3, v10, vcc_lo
	v_add_co_u32 v9, vcc_lo, s4, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s5, v10, vcc_lo
	s_mov_b32 s2, 0x3b39803f
	s_mov_b32 s4, 0xfca7ab0c
	s_mov_b32 s3, 0xbc7abc9e
	s_mov_b32 s5, 0x3e928af3
	s_mov_b32 s15, 0x3e5ade15
	s_mov_b32 s17, 0x3ec71dee
	s_mov_b32 s19, 0x3efa0199
	s_mov_b32 s21, 0x3f2a01a0
	s_mov_b32 s23, 0x3f56c16c
	s_mov_b32 s25, 0x3f811111
	s_mov_b32 s27, 0x3fa55555
	s_mov_b32 s29, 0x3fc55555
	s_mov_b32 s31, 0x3fe00000
	s_mov_b32 s35, 0xbfed67f1
.LBB4_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[11:12], v[7:8], off
	global_load_b64 v[19:20], v[9:10], off
	global_load_b64 v[21:22], v[5:6], off
	s_add_i32 s9, s9, -1
	s_waitcnt vmcnt(2)
	v_mul_f64 v[13:14], v[11:12], s[10:11]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[11:12]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[13:14], v[13:14]
	v_fma_f64 v[15:16], v[13:14], s[12:13], v[11:12]
	v_cvt_i32_f64_e32 v0, v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], s[2:3], v[15:16]
	v_fma_f64 v[17:18], v[15:16], s[14:15], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[16:17]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[18:19]
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
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[13:14], v[13:14], v0
	v_cndmask_b32_e32 v0, 0x7ff00000, v14, vcc_lo
	s_waitcnt vmcnt(0)
	v_add_f64 v[14:15], v[19:20], -v[21:22]
	s_and_b32 vcc_lo, s0, vcc_lo
	s_cmp_eq_u32 s9, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v16, 0, v13, vcc_lo
	v_cndmask_b32_e64 v17, 0, v0, s0
	v_div_scale_f64 v[18:19], null, v[16:17], v[16:17], v[14:15]
	v_div_scale_f64 v[24:25], vcc_lo, v[14:15], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[20:21], v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[22:23], -v[18:19], v[20:21], 1.0
	v_fma_f64 v[20:21], v[20:21], v[22:23], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], -v[18:19], v[20:21], 1.0
	v_fma_f64 v[20:21], v[20:21], v[22:23], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[22:23], v[24:25], v[20:21]
	v_fma_f64 v[18:19], -v[18:19], v[22:23], v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[18:19], v[18:19], v[20:21], v[22:23]
	v_add_co_u32 v7, vcc_lo, v7, 8
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_add_co_u32 v9, vcc_lo, v9, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, 0, v10, vcc_lo
	v_add_co_u32 v5, vcc_lo, v5, 8
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	v_div_fixup_f64 v[13:14], v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[13:14], -0.5
	v_fma_f64 v[11:12], v[13:14], v[15:16], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[11:12], s[34:35]
	v_add_f64 v[3:4], v[3:4], v[11:12]
	s_cbranch_scc0 .LBB4_3
	s_branch .LBB4_5
.LBB4_4:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB4_5:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB4_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z23gaussian_logprob_kernelPKdS0_S0_Pdii
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
		.amdhsa_next_free_vgpr 26
		.amdhsa_next_free_sgpr 36
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
.Lfunc_end4:
	.size	_Z23gaussian_logprob_kernelPKdS0_S0_Pdii, .Lfunc_end4-_Z23gaussian_logprob_kernelPKdS0_S0_Pdii
                                        ; -- End function
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.num_vgpr, 26
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.num_agpr, 0
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.numbered_sgpr, 36
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.num_named_barrier, 0
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.private_seg_size, 0
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.uses_vcc, 1
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.uses_flat_scratch, 0
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.has_dyn_sized_stack, 0
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.has_recursion, 0
	.set _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 904
; TotalNumSgprs: 38
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 38
; NumVGPRsForWavesPerEU: 26
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
	.type	__hip_cuid_e0d81515a70b4672,@object ; @__hip_cuid_e0d81515a70b4672
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_e0d81515a70b4672
__hip_cuid_e0d81515a70b4672:
	.byte	0                               ; 0x0
	.size	__hip_cuid_e0d81515a70b4672, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_e0d81515a70b4672
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 28
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z25discounted_returns_kernelPKdPdS0_i
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         _Z25discounted_returns_kernelPKdPdS0_i.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 44
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z10gae_kernelPKdS0_PdS0_S0_i
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z10gae_kernelPKdS0_PdS0_S0_i.kd
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
    .name:           _Z17td_targets_kernelPKdS0_S0_PdS0_i
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z17td_targets_kernelPKdS0_S0_PdS0_i.kd
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
    .name:           _Z26categorical_logprob_kernelPKdPKiPdii
    .private_segment_fixed_size: 0
    .sgpr_count:     38
    .sgpr_spill_count: 0
    .symbol:         _Z26categorical_logprob_kernelPKdPKiPdii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     29
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
    .name:           _Z23gaussian_logprob_kernelPKdS0_S0_Pdii
    .private_segment_fixed_size: 0
    .sgpr_count:     38
    .sgpr_spill_count: 0
    .symbol:         _Z23gaussian_logprob_kernelPKdS0_S0_Pdii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     26
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
