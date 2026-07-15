	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z25floor_scale_to_idx_kernelPKdPiii ; -- Begin function _Z25floor_scale_to_idx_kernelPKdPiii
	.globl	_Z25floor_scale_to_idx_kernelPKdPiii
	.p2align	8
	.type	_Z25floor_scale_to_idx_kernelPKdPiii,@function
_Z25floor_scale_to_idx_kernelPKdPiii:   ; @_Z25floor_scale_to_idx_kernelPKdPiii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB0_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	v_cvt_f64_i32_e32 v[5:6], s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[3:4], 3, v[1:2]
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s0, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s1, v4, vcc_lo
	s_add_i32 s0, s5, -1
	v_add_co_u32 v0, vcc_lo, s2, v0
	global_load_b64 v[3:4], v[3:4], off
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_mul_f64 v[3:4], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_i32_f64_e32 v3, v[3:4]
	v_min_i32_e32 v2, s0, v3
	global_store_b32 v[0:1], v2, off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z25floor_scale_to_idx_kernelPKdPiii
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
		.amdhsa_next_free_vgpr 7
		.amdhsa_next_free_sgpr 6
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
	.size	_Z25floor_scale_to_idx_kernelPKdPiii, .Lfunc_end0-_Z25floor_scale_to_idx_kernelPKdPiii
                                        ; -- End function
	.set _Z25floor_scale_to_idx_kernelPKdPiii.num_vgpr, 7
	.set _Z25floor_scale_to_idx_kernelPKdPiii.num_agpr, 0
	.set _Z25floor_scale_to_idx_kernelPKdPiii.numbered_sgpr, 6
	.set _Z25floor_scale_to_idx_kernelPKdPiii.num_named_barrier, 0
	.set _Z25floor_scale_to_idx_kernelPKdPiii.private_seg_size, 0
	.set _Z25floor_scale_to_idx_kernelPKdPiii.uses_vcc, 1
	.set _Z25floor_scale_to_idx_kernelPKdPiii.uses_flat_scratch, 0
	.set _Z25floor_scale_to_idx_kernelPKdPiii.has_dyn_sized_stack, 0
	.set _Z25floor_scale_to_idx_kernelPKdPiii.has_recursion, 0
	.set _Z25floor_scale_to_idx_kernelPKdPiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 184
; TotalNumSgprs: 8
; NumVgprs: 7
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
; NumVGPRsForWavesPerEU: 7
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
	.protected	_Z23forest_rand_keys_kernelPdij ; -- Begin function _Z23forest_rand_keys_kernelPdij
	.globl	_Z23forest_rand_keys_kernelPdij
	.p2align	8
	.type	_Z23forest_rand_keys_kernelPdij,@function
_Z23forest_rand_keys_kernelPdij:        ; @_Z23forest_rand_keys_kernelPdij
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x1c
	s_load_b64 s[4:5], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_2
; %bb.1:
	v_mul_hi_u32 v0, 0xd2511f53, v1
	v_mul_lo_u32 v2, 0xd2511f53, v1
	s_lshl_b32 s2, s5, 1
	s_load_b64 s[0:1], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, 0xd2511f53, v0
	v_mul_lo_u32 v0, 0xd2511f53, v0
	v_xor3_b32 v2, v2, s2, v3
	s_mul_i32 s2, s5, 3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, 0xd2511f53, v2
	v_mul_lo_u32 v2, 0xd2511f53, v2
	v_xor3_b32 v0, v0, s2, v3
	s_lshl_b32 s2, s5, 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, 0xd2511f53, v0
	v_mul_lo_u32 v0, 0xd2511f53, v0
	v_xor3_b32 v2, v2, s2, v3
	s_mul_i32 s2, s5, 5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, 0xd2511f53, v2
	v_mul_lo_u32 v2, 0xd2511f53, v2
	v_xor3_b32 v0, v0, s2, v3
	s_mul_i32 s2, s5, 6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, 0xd2511f53, v0
	v_mul_lo_u32 v0, 0xd2511f53, v0
	v_xor3_b32 v2, v2, s2, v3
	s_mul_i32 s2, s5, 7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, 0xd2511f53, v2
	v_mul_lo_u32 v2, 0xd2511f53, v2
	v_xor3_b32 v0, v0, s2, v3
	s_lshl_b32 s2, s5, 3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, 0xd2511f53, v0
	v_mul_lo_u32 v0, 0xd2511f53, v0
	v_xor3_b32 v2, v2, s2, v3
	s_mul_i32 s2, s5, 9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v3, 0xd2511f53, v2
	v_mul_lo_u32 v2, 0xd2511f53, v2
	v_xor3_b32 v0, v0, s2, v3
	s_mul_i32 s2, s5, 10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, 0xd2511f53, v0
	v_xor3_b32 v0, v2, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_u32_e32 v[2:3], v0
	v_ldexp_f64 v[3:4], v[2:3], 0xffffffe0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z23forest_rand_keys_kernelPdij
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
		.amdhsa_next_free_vgpr 5
		.amdhsa_next_free_sgpr 6
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
.Lfunc_end1:
	.size	_Z23forest_rand_keys_kernelPdij, .Lfunc_end1-_Z23forest_rand_keys_kernelPdij
                                        ; -- End function
	.set _Z23forest_rand_keys_kernelPdij.num_vgpr, 5
	.set _Z23forest_rand_keys_kernelPdij.num_agpr, 0
	.set _Z23forest_rand_keys_kernelPdij.numbered_sgpr, 6
	.set _Z23forest_rand_keys_kernelPdij.num_named_barrier, 0
	.set _Z23forest_rand_keys_kernelPdij.private_seg_size, 0
	.set _Z23forest_rand_keys_kernelPdij.uses_vcc, 1
	.set _Z23forest_rand_keys_kernelPdij.uses_flat_scratch, 0
	.set _Z23forest_rand_keys_kernelPdij.has_dyn_sized_stack, 0
	.set _Z23forest_rand_keys_kernelPdij.has_recursion, 0
	.set _Z23forest_rand_keys_kernelPdij.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 512
; TotalNumSgprs: 8
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
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
	.protected	_Z22bitonic_argsort_kernelPKdPii ; -- Begin function _Z22bitonic_argsort_kernelPKdPii
	.globl	_Z22bitonic_argsort_kernelPKdPii
	.p2align	8
	.type	_Z22bitonic_argsort_kernelPKdPii,@function
_Z22bitonic_argsort_kernelPKdPii:       ; @_Z22bitonic_argsort_kernelPKdPii
; %bb.0:
	s_clause 0x2
	s_load_b32 s2, s[0:1], 0x10
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b32 s0, s[0:1], 0x24
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 2.0
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_i32_e32 vcc_lo, s2, v0
	s_and_saveexec_b32 s1, vcc_lo
	s_cbranch_execz .LBB2_2
; %bb.1:
	v_lshlrev_b32_e32 v1, 3, v0
	global_load_b64 v[1:2], v1, s[4:5]
.LBB2_2:
	s_or_b32 exec_lo, exec_lo, s1
	s_and_b32 s2, 0xffff, s0
	v_lshl_add_u32 v6, v0, 3, 0
	s_lshl_b32 s0, s2, 3
	s_mov_b32 s4, 2
	s_add_i32 s3, s0, 0
	s_cmp_lt_u32 s2, 2
	v_lshl_add_u32 v5, v0, 2, s3
	s_waitcnt vmcnt(0)
	ds_store_b64 v6, v[1:2]
	ds_store_b32 v5, v0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc0 .LBB2_7
.LBB2_3:
	s_and_saveexec_b32 s0, vcc_lo
	s_cbranch_execz .LBB2_5
; %bb.4:
	ds_load_b32 v1, v5
	v_lshlrev_b32_e32 v0, 2, v0
	s_waitcnt lgkmcnt(0)
	global_store_b32 v0, v1, s[6:7]
.LBB2_5:
	s_endpgm
.LBB2_6:                                ;   in Loop: Header=BB2_7 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_lshl_b32 s4, s4, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_gt_u32 s4, s2
	s_cbranch_scc1 .LBB2_3
.LBB2_7:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB2_9 Depth 2
	v_and_b32_e32 v1, s4, v0
	s_mov_b32 s5, s4
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_eq_u32_e64 s0, 0, v1
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB2_9
	.p2align	6
.LBB2_8:                                ;   in Loop: Header=BB2_9 Depth=2
	s_or_b32 exec_lo, exec_lo, s9
	s_cmp_lt_u32 s8, 4
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB2_6
.LBB2_9:                                ;   Parent Loop BB2_7 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_mov_b32 s8, s5
	s_lshr_b32 s5, s5, 1
	s_mov_b32 s9, exec_lo
	v_xor_b32_e32 v7, s5, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_u32_e64 v7, v0
	s_cbranch_execz .LBB2_8
; %bb.10:                               ;   in Loop: Header=BB2_9 Depth=2
	v_lshl_add_u32 v8, v7, 3, 0
	ds_load_b64 v[1:2], v6
	ds_load_b64 v[3:4], v8
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f64_e64 s1, v[1:2], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v9, 0, 1, s1
	v_cmp_gt_f64_e64 s1, v[1:2], v[3:4]
	v_cndmask_b32_e64 v10, 0, 1, s1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b16 v9.l, v9.l, v10.l, s0
	v_and_b16 v9.l, 1, v9.l
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_eq_u16_e64 s1, 1, v9.l
	s_and_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB2_8
; %bb.11:                               ;   in Loop: Header=BB2_9 Depth=2
	v_lshl_add_u32 v7, v7, 2, s3
	ds_load_b32 v9, v7
	ds_load_b32 v10, v5
	ds_store_b64 v6, v[3:4]
	ds_store_b64 v8, v[1:2]
	s_waitcnt lgkmcnt(3)
	ds_store_b32 v5, v9
	s_waitcnt lgkmcnt(3)
	ds_store_b32 v7, v10
	s_branch .LBB2_8
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22bitonic_argsort_kernelPKdPii
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
		.amdhsa_next_free_vgpr 11
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
	.size	_Z22bitonic_argsort_kernelPKdPii, .Lfunc_end2-_Z22bitonic_argsort_kernelPKdPii
                                        ; -- End function
	.set _Z22bitonic_argsort_kernelPKdPii.num_vgpr, 11
	.set _Z22bitonic_argsort_kernelPKdPii.num_agpr, 0
	.set _Z22bitonic_argsort_kernelPKdPii.numbered_sgpr, 10
	.set _Z22bitonic_argsort_kernelPKdPii.num_named_barrier, 0
	.set _Z22bitonic_argsort_kernelPKdPii.private_seg_size, 0
	.set _Z22bitonic_argsort_kernelPKdPii.uses_vcc, 1
	.set _Z22bitonic_argsort_kernelPKdPii.uses_flat_scratch, 0
	.set _Z22bitonic_argsort_kernelPKdPii.has_dyn_sized_stack, 0
	.set _Z22bitonic_argsort_kernelPKdPii.has_recursion, 0
	.set _Z22bitonic_argsort_kernelPKdPii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 464
; TotalNumSgprs: 12
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 12
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
	.protected	_Z17col_minmax_kernelPKdPdS1_i ; -- Begin function _Z17col_minmax_kernelPKdPdS1_i
	.globl	_Z17col_minmax_kernelPKdPdS1_i
	.p2align	8
	.type	_Z17col_minmax_kernelPKdPdS1_i,@function
_Z17col_minmax_kernelPKdPdS1_i:         ; @_Z17col_minmax_kernelPKdPdS1_i
; %bb.0:
	s_clause 0x2
	s_load_b32 s8, s[0:1], 0x18
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_add_u32 s0, s0, 32
	s_addc_u32 s1, s1, 0
	s_mov_b32 s9, exec_lo
                                        ; implicit-def: $sgpr10
	s_waitcnt lgkmcnt(0)
	v_cmpx_le_i32_e64 s8, v0
	s_xor_b32 s9, exec_lo, s9
; %bb.1:
	s_load_b32 s10, s[0:1], 0xc
; %bb.2:
	s_or_saveexec_b32 s9, s9
	v_mov_b32_e32 v2, 0x85ebc8a0
	v_mov_b32_e32 v3, 0xffe1ccf3
	v_mov_b32_e32 v5, 0x7fe1ccf3
	s_waitcnt lgkmcnt(0)
	v_mov_b16_e32 v1.l, s10
	v_mov_b32_e32 v4, v2
	s_xor_b32 exec_lo, exec_lo, s9
	s_cbranch_execz .LBB3_6
; %bb.3:
	s_load_b32 s10, s[0:1], 0xc
	v_mov_b32_e32 v4, v2
	v_mov_b32_e32 v6, v0
	s_mov_b32 s11, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s12, s10, 0xffff
	.p2align	6
.LBB3_4:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v7, 31, v6
	v_lshlrev_b64 v[7:8], 3, v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s4, v7
	v_add_co_ci_u32_e64 v8, null, s5, v8, vcc_lo
	global_load_b64 v[7:8], v[7:8], off
	s_waitcnt vmcnt(0)
	v_cmp_lt_f64_e32 vcc_lo, v[7:8], v[4:5]
	v_cmp_gt_f64_e64 s0, v[7:8], v[2:3]
	v_dual_cndmask_b32 v5, v5, v8 :: v_dual_add_nc_u32 v6, s12, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_i32_e64 s1, s8, v6
	v_cndmask_b32_e64 v3, v3, v8, s0
	v_cndmask_b32_e32 v4, v4, v7, vcc_lo
	v_cndmask_b32_e64 v2, v2, v7, s0
	s_or_b32 s11, s1, s11
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s11
	s_cbranch_execnz .LBB3_4
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s11
	v_mov_b16_e32 v1.l, s10
.LBB3_6:
	s_or_b32 exec_lo, exec_lo, s9
	v_lshlrev_b32_e32 v6, 3, v0
	s_mov_b32 s0, exec_lo
	ds_store_2addr_stride64_b64 v6, v[2:3], v[4:5] offset1:4
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_lt_u16_e32 1, v1.l
	s_cbranch_execz .LBB3_14
; %bb.7:
	v_add_nc_u32_e32 v3, 0x800, v6
	v_lshrrev_b16 v4.l, 1, v1.l
	v_mov_b16_e32 v4.h, 0
	s_mov_b32 s1, 0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB3_9
	.p2align	6
.LBB3_8:                                ;   in Loop: Header=BB3_9 Depth=1
	s_or_b32 exec_lo, exec_lo, s4
	v_lshrrev_b32_e32 v1, 1, v4
	v_cmp_gt_u32_e32 vcc_lo, 2, v4
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_mov_b32_e32 v4, v1
	s_or_b32 s1, vcc_lo, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB3_14
.LBB3_9:                                ; =>This Inner Loop Header: Depth=1
	s_mov_b32 s4, exec_lo
	v_cmpx_lt_u32_e64 v0, v4
	s_cbranch_execz .LBB3_8
; %bb.10:                               ;   in Loop: Header=BB3_9 Depth=1
	v_lshl_add_u32 v1, v4, 3, v3
	s_mov_b32 s5, exec_lo
	ds_load_b64 v[1:2], v1
	ds_load_b64 v[7:8], v3
	s_waitcnt lgkmcnt(0)
	v_cmpx_lt_f64_e32 v[1:2], v[7:8]
; %bb.11:                               ;   in Loop: Header=BB3_9 Depth=1
	ds_store_b64 v3, v[1:2]
; %bb.12:                               ;   in Loop: Header=BB3_9 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	v_lshl_add_u32 v1, v4, 3, v6
	ds_load_b64 v[1:2], v1
	ds_load_b64 v[7:8], v6
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[1:2], v[7:8]
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB3_8
; %bb.13:                               ;   in Loop: Header=BB3_9 Depth=1
	ds_store_b64 v6, v[1:2]
	s_branch .LBB3_8
.LBB3_14:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB3_16
; %bb.15:
	v_mov_b32_e32 v4, 0
	ds_load_2addr_stride64_b64 v[0:3], v4 offset1:4
	s_waitcnt lgkmcnt(0)
	s_clause 0x1
	global_store_b64 v4, v[2:3], s[6:7]
	global_store_b64 v4, v[0:1], s[2:3]
.LBB3_16:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z17col_minmax_kernelPKdPdS1_i
		.amdhsa_group_segment_fixed_size 4096
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
		.amdhsa_next_free_vgpr 9
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
	.size	_Z17col_minmax_kernelPKdPdS1_i, .Lfunc_end3-_Z17col_minmax_kernelPKdPdS1_i
                                        ; -- End function
	.set _Z17col_minmax_kernelPKdPdS1_i.num_vgpr, 9
	.set _Z17col_minmax_kernelPKdPdS1_i.num_agpr, 0
	.set _Z17col_minmax_kernelPKdPdS1_i.numbered_sgpr, 13
	.set _Z17col_minmax_kernelPKdPdS1_i.num_named_barrier, 0
	.set _Z17col_minmax_kernelPKdPdS1_i.private_seg_size, 0
	.set _Z17col_minmax_kernelPKdPdS1_i.uses_vcc, 1
	.set _Z17col_minmax_kernelPKdPdS1_i.uses_flat_scratch, 0
	.set _Z17col_minmax_kernelPKdPdS1_i.has_dyn_sized_stack, 0
	.set _Z17col_minmax_kernelPKdPdS1_i.has_recursion, 0
	.set _Z17col_minmax_kernelPKdPdS1_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 572
; TotalNumSgprs: 15
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 4096 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 15
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
	.protected	_Z21draw_threshold_kernelPKdS0_Pdj ; -- Begin function _Z21draw_threshold_kernelPKdS0_Pdj
	.globl	_Z21draw_threshold_kernelPKdS0_Pdj
	.p2align	8
	.type	_Z21draw_threshold_kernelPKdS0_Pdj,@function
_Z21draw_threshold_kernelPKdS0_Pdj:     ; @_Z21draw_threshold_kernelPKdS0_Pdj
; %bb.0:
	s_load_b32 s2, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_lshl_b32 s3, s2, 1
	s_mul_i32 s5, s2, 3
	s_mul_hi_u32 s3, s3, 0xd2511f53
	s_mul_i32 s4, s2, 0xa4a23ea6
	s_lshl_b32 s6, s2, 2
	s_xor_b32 s3, s5, s3
	s_xor_b32 s4, s4, s6
	s_mul_hi_u32 s5, s3, 0xd2511f53
	s_mul_i32 s7, s2, 5
	s_mul_i32 s3, s3, 0xd2511f53
	s_xor_b32 s4, s4, s5
	s_xor_b32 s3, s3, s7
	s_mul_hi_u32 s5, s4, 0xd2511f53
	s_mul_i32 s8, s2, 6
	s_mul_i32 s4, s4, 0xd2511f53
	s_xor_b32 s3, s3, s5
	s_xor_b32 s4, s4, s8
	s_mul_hi_u32 s5, s3, 0xd2511f53
	s_mul_i32 s9, s2, 7
	s_mul_i32 s3, s3, 0xd2511f53
	s_xor_b32 s4, s4, s5
	s_xor_b32 s3, s3, s9
	s_mul_hi_u32 s5, s4, 0xd2511f53
	s_lshl_b32 s10, s2, 3
	s_mul_i32 s4, s4, 0xd2511f53
	s_xor_b32 s3, s3, s5
	s_xor_b32 s4, s4, s10
	s_mul_hi_u32 s5, s3, 0xd2511f53
	s_mul_i32 s11, s2, 9
	s_mul_i32 s3, s3, 0xd2511f53
	s_xor_b32 s4, s4, s5
	s_xor_b32 s3, s3, s11
	s_mul_hi_u32 s5, s4, 0xd2511f53
	s_mul_i32 s2, s2, 10
	s_mul_i32 s4, s4, 0xd2511f53
	s_xor_b32 s3, s3, s5
	s_xor_b32 s2, s4, s2
	s_mul_hi_u32 s3, s3, 0xd2511f53
	s_load_b128 s[4:7], s[0:1], 0x0
	s_xor_b32 s2, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cvt_f64_u32_e32 v[0:1], s2
	s_load_b64 s[0:1], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[4:5], 0x0
	s_load_b64 s[4:5], s[6:7], 0x0
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], s[4:5], -s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[0:1], v[0:1], 0xffffffe0
	v_fma_f64 v[0:1], v[0:1], v[2:3], s[2:3]
	v_mov_b32_e32 v2, 0
	global_store_b64 v2, v[0:1], s[0:1]
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z21draw_threshold_kernelPKdS0_Pdj
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
		.amdhsa_next_free_vgpr 4
		.amdhsa_next_free_sgpr 12
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
	.size	_Z21draw_threshold_kernelPKdS0_Pdj, .Lfunc_end4-_Z21draw_threshold_kernelPKdS0_Pdj
                                        ; -- End function
	.set _Z21draw_threshold_kernelPKdS0_Pdj.num_vgpr, 4
	.set _Z21draw_threshold_kernelPKdS0_Pdj.num_agpr, 0
	.set _Z21draw_threshold_kernelPKdS0_Pdj.numbered_sgpr, 12
	.set _Z21draw_threshold_kernelPKdS0_Pdj.num_named_barrier, 0
	.set _Z21draw_threshold_kernelPKdS0_Pdj.private_seg_size, 0
	.set _Z21draw_threshold_kernelPKdS0_Pdj.uses_vcc, 0
	.set _Z21draw_threshold_kernelPKdS0_Pdj.uses_flat_scratch, 0
	.set _Z21draw_threshold_kernelPKdS0_Pdj.has_dyn_sized_stack, 0
	.set _Z21draw_threshold_kernelPKdS0_Pdj.has_recursion, 0
	.set _Z21draw_threshold_kernelPKdS0_Pdj.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 324
; TotalNumSgprs: 12
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 4
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
	.protected	_Z19scatter_used_kernelPKiPhi ; -- Begin function _Z19scatter_used_kernelPKiPhi
	.globl	_Z19scatter_used_kernelPKiPhi
	.p2align	8
	.type	_Z19scatter_used_kernelPKiPhi,@function
_Z19scatter_used_kernelPKiPhi:          ; @_Z19scatter_used_kernelPKiPhi
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB5_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_b32 v0, v[0:1], off
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v2, 31, v0
	v_add_co_u32 v1, vcc_lo, s2, v0
	v_mov_b16_e32 v0.l, 1
	s_delay_alu instid0(VALU_DEP_3)
	v_add_co_ci_u32_e64 v2, null, s3, v2, vcc_lo
	global_store_b8 v[1:2], v0, off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z19scatter_used_kernelPKiPhi
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
		.amdhsa_next_free_vgpr 3
		.amdhsa_next_free_sgpr 5
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
	.size	_Z19scatter_used_kernelPKiPhi, .Lfunc_end5-_Z19scatter_used_kernelPKiPhi
                                        ; -- End function
	.set _Z19scatter_used_kernelPKiPhi.num_vgpr, 3
	.set _Z19scatter_used_kernelPKiPhi.num_agpr, 0
	.set _Z19scatter_used_kernelPKiPhi.numbered_sgpr, 5
	.set _Z19scatter_used_kernelPKiPhi.num_named_barrier, 0
	.set _Z19scatter_used_kernelPKiPhi.private_seg_size, 0
	.set _Z19scatter_used_kernelPKiPhi.uses_vcc, 1
	.set _Z19scatter_used_kernelPKiPhi.uses_flat_scratch, 0
	.set _Z19scatter_used_kernelPKiPhi.has_dyn_sized_stack, 0
	.set _Z19scatter_used_kernelPKiPhi.has_recursion, 0
	.set _Z19scatter_used_kernelPKiPhi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 160
; TotalNumSgprs: 7
; NumVgprs: 3
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
; NumVGPRsForWavesPerEU: 3
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
	.protected	_Z18invert_mask_kernelPKhPhi ; -- Begin function _Z18invert_mask_kernelPKhPhi
	.globl	_Z18invert_mask_kernelPKhPhi
	.p2align	8
	.type	_Z18invert_mask_kernelPKhPhi,@function
_Z18invert_mask_kernelPKhPhi:           ; @_Z18invert_mask_kernelPKhPhi
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB6_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v4, 31, v1
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v4, vcc_lo
	global_load_d16_u8 v0, v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_u16_e32 vcc_lo, 0, v0.l
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_add_co_u32 v1, vcc_lo, s2, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s3, v4, vcc_lo
	global_store_b8 v[1:2], v0, off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z18invert_mask_kernelPKhPhi
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
		.amdhsa_next_free_vgpr 5
		.amdhsa_next_free_sgpr 5
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
.Lfunc_end6:
	.size	_Z18invert_mask_kernelPKhPhi, .Lfunc_end6-_Z18invert_mask_kernelPKhPhi
                                        ; -- End function
	.set _Z18invert_mask_kernelPKhPhi.num_vgpr, 5
	.set _Z18invert_mask_kernelPKhPhi.num_agpr, 0
	.set _Z18invert_mask_kernelPKhPhi.numbered_sgpr, 5
	.set _Z18invert_mask_kernelPKhPhi.num_named_barrier, 0
	.set _Z18invert_mask_kernelPKhPhi.private_seg_size, 0
	.set _Z18invert_mask_kernelPKhPhi.uses_vcc, 1
	.set _Z18invert_mask_kernelPKhPhi.uses_flat_scratch, 0
	.set _Z18invert_mask_kernelPKhPhi.has_dyn_sized_stack, 0
	.set _Z18invert_mask_kernelPKhPhi.has_recursion, 0
	.set _Z18invert_mask_kernelPKhPhi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 152
; TotalNumSgprs: 7
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	_Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_ ; -- Begin function _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_
	.globl	_Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_
	.p2align	8
	.type	_Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_,@function
_Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_: ; @_Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x64
	s_load_b64 s[20:21], s[0:1], 0x48
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s20, v1
	s_cbranch_execz .LBB7_10
; %bb.1:
	s_load_b64 s[2:3], s[0:1], 0x50
	s_cmp_lt_i32 s21, 1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[2:3], 0x0
	s_cbranch_scc1 .LBB7_8
; %bb.2:
	s_load_b512 s[4:19], s[0:1], 0x0
	v_mov_b32_e32 v2, 0
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v0, 0
	s_mov_b32 s23, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s22, s23
	s_branch .LBB7_4
.LBB7_3:                                ;   in Loop: Header=BB7_4 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_add_i32 s22, s22, 1
	s_cmp_eq_u32 s22, s21
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s16, v4
	v_add_co_ci_u32_e64 v5, null, s17, v5, vcc_lo
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	s_cbranch_scc1 .LBB7_9
.LBB7_4:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB7_6 Depth 2
	s_lshl_b64 s[24:25], s[22:23], 2
	s_waitcnt lgkmcnt(0)
	s_add_u32 s24, s18, s24
	s_addc_u32 s25, s19, s25
	s_load_b32 s24, s[24:25], 0x0
	s_waitcnt lgkmcnt(0)
	s_ashr_i32 s25, s24, 31
	s_add_u32 s26, s14, s24
	s_addc_u32 s27, s15, s25
	global_load_u8 v4, v0, s[26:27]
	s_waitcnt vmcnt(0)
	v_cmp_ne_u32_e32 vcc_lo, 0, v4
	v_dual_mov_b32 v4, s24 :: v_dual_mov_b32 v5, s25
	s_cbranch_vccnz .LBB7_3
; %bb.5:                                ;   in Loop: Header=BB7_4 Depth=1
	v_dual_mov_b32 v4, s24 :: v_dual_mov_b32 v5, s25
	s_mov_b32 s24, 0
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB7_6:                                ;   Parent Loop BB7_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	v_add_co_u32 v6, vcc_lo, s6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s7, v5, vcc_lo
	global_load_b32 v8, v[6:7], off
	s_waitcnt vmcnt(0)
	v_mad_u64_u32 v[6:7], null, v8, s20, v[1:2]
	v_add_co_u32 v7, vcc_lo, s8, v4
	v_add_co_ci_u32_e64 v8, null, s9, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v10, 31, v6
	v_add_co_u32 v9, vcc_lo, s4, v6
	v_add_co_ci_u32_e64 v10, null, s5, v10, vcc_lo
	global_load_b32 v6, v[7:8], off
	global_load_u8 v7, v[9:10], off
	v_dual_mov_b32 v8, s13 :: v_dual_mov_b32 v9, s12
	s_waitcnt vmcnt(0)
	v_cmp_lt_i32_e32 vcc_lo, v6, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v7, s10, v9, vcc_lo
	v_cndmask_b32_e32 v6, s11, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, v7, v4
	v_add_co_ci_u32_e64 v5, null, v6, v5, vcc_lo
	global_load_b32 v4, v[4:5], off
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v5, 31, v4
	v_add_co_u32 v6, vcc_lo, s14, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v7, null, s15, v5, vcc_lo
	global_load_d16_u8 v6, v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmp_ne_u16_e32 vcc_lo, 0, v6.l
	s_or_b32 s24, vcc_lo, s24
	s_and_not1_b32 exec_lo, exec_lo, s24
	s_cbranch_execnz .LBB7_6
; %bb.7:                                ;   in Loop: Header=BB7_4 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s24
	s_branch .LBB7_3
.LBB7_8:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
.LBB7_9:
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_f64 v[3:4], s[2:3], v[2:3]
	s_load_b64 s[0:1], s[0:1], 0x40
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB7_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 344
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
.Lfunc_end7:
	.size	_Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_, .Lfunc_end7-_Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_
                                        ; -- End function
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.num_vgpr, 11
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.num_agpr, 0
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.numbered_sgpr, 28
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.num_named_barrier, 0
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.private_seg_size, 0
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.uses_vcc, 1
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.uses_flat_scratch, 0
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.has_dyn_sized_stack, 0
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.has_recursion, 0
	.set _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 584
; TotalNumSgprs: 30
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 30
; NumVGPRsForWavesPerEU: 11
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_5014904dcbf4d3e0,@object ; @__hip_cuid_5014904dcbf4d3e0
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_5014904dcbf4d3e0
__hip_cuid_5014904dcbf4d3e0:
	.byte	0                               ; 0x0
	.size	__hip_cuid_5014904dcbf4d3e0, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_5014904dcbf4d3e0
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
    .name:           _Z25floor_scale_to_idx_kernelPKdPiii
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         _Z25floor_scale_to_idx_kernelPKdPiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     7
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
    .name:           _Z23forest_rand_keys_kernelPdij
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         _Z23forest_rand_keys_kernelPdij.kd
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
      - .offset:         144
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z22bitonic_argsort_kernelPKdPii
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         _Z22bitonic_argsort_kernelPKdPii.kd
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
    .group_segment_fixed_size: 4096
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z17col_minmax_kernelPKdPdS1_i
    .private_segment_fixed_size: 0
    .sgpr_count:     15
    .sgpr_spill_count: 0
    .symbol:         _Z17col_minmax_kernelPKdPdS1_i.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 28
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z21draw_threshold_kernelPKdS0_Pdj
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         _Z21draw_threshold_kernelPKdS0_Pdj.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     4
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z19scatter_used_kernelPKiPhi
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         _Z19scatter_used_kernelPKiPhi.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     3
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z18invert_mask_kernelPKhPhi
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         _Z18invert_mask_kernelPKhPhi.kd
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
      - .address_space:  global
        .offset:         64
        .size:           8
        .value_kind:     global_buffer
      - .offset:         72
        .size:           4
        .value_kind:     by_value
      - .offset:         76
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         80
        .size:           8
        .value_kind:     global_buffer
      - .offset:         88
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         92
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         96
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         100
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         102
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         104
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         106
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         108
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         110
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         136
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         144
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         152
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 344
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_
    .private_segment_fixed_size: 0
    .sgpr_count:     30
    .sgpr_spill_count: 0
    .symbol:         _Z28tree_ensemble_predict_kernelPKhPKiS2_S2_S2_S0_PKdS2_PdiiS4_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     11
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
