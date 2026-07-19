	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	distancex_manhattan_kernel ; -- Begin function distancex_manhattan_kernel
	.globl	distancex_manhattan_kernel
	.p2align	8
	.type	distancex_manhattan_kernel,@function
distancex_manhattan_kernel:             ; @distancex_manhattan_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB0_4
; %bb.2:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v5, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s3, v0
	s_ashr_i32 s3, s5, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, v5, v0, 0
	v_mul_lo_u32 v0, v4, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v5, v0
	v_subrev_nc_u32_e32 v5, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v0, v0, v5 :: v_dual_add_nc_u32 v3, 1, v4
	v_cndmask_b32_e32 v3, v4, v3, vcc_lo
	v_xor_b32_e32 v5, s3, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	v_add_nc_u32_e32 v4, 1, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v3, v4, vcc_lo
	v_xor_b32_e32 v0, v0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v5
	v_mul_lo_u32 v3, v0, s5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v7, v1, v3
	v_mad_i64_i32 v[3:4], null, v0, s6, 0
	v_mad_i64_i32 v[5:6], null, v7, s6, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[7:8], 3, v[3:4]
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	v_lshlrev_b64 v[9:10], 3, v[5:6]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s8, v7
	v_add_co_ci_u32_e64 v6, null, s9, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s10, v9
	v_add_co_ci_u32_e64 v8, null, s11, v10, vcc_lo
	.p2align	6
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[9:10], v[5:6], off
	global_load_b64 v[11:12], v[7:8], off
	v_add_co_u32 v5, vcc_lo, v5, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	v_add_co_u32 v7, vcc_lo, v7, 8
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	s_add_i32 s6, s6, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s6, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	v_add_f64 v[3:4], v[3:4], |v[9:10]|
	s_cbranch_scc0 .LBB0_3
	s_branch .LBB0_5
.LBB0_4:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB0_5:
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB0_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel distancex_manhattan_kernel
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
		.amdhsa_next_free_vgpr 13
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
.Lfunc_end0:
	.size	distancex_manhattan_kernel, .Lfunc_end0-distancex_manhattan_kernel
                                        ; -- End function
	.set distancex_manhattan_kernel.num_vgpr, 13
	.set distancex_manhattan_kernel.num_agpr, 0
	.set distancex_manhattan_kernel.numbered_sgpr, 12
	.set distancex_manhattan_kernel.num_named_barrier, 0
	.set distancex_manhattan_kernel.private_seg_size, 0
	.set distancex_manhattan_kernel.uses_vcc, 1
	.set distancex_manhattan_kernel.uses_flat_scratch, 0
	.set distancex_manhattan_kernel.has_dyn_sized_stack, 0
	.set distancex_manhattan_kernel.has_recursion, 0
	.set distancex_manhattan_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 504
; TotalNumSgprs: 14
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
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
	.protected	distancex_chebyshev_kernel ; -- Begin function distancex_chebyshev_kernel
	.globl	distancex_chebyshev_kernel
	.p2align	8
	.type	distancex_chebyshev_kernel,@function
distancex_chebyshev_kernel:             ; @distancex_chebyshev_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB1_4
; %bb.2:
	s_abs_i32 s0, s5
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s0
	s_sub_i32 s1, 0, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v5, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s1, v0
	s_ashr_i32 s1, s5, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, v5, v0, 0
	v_mul_lo_u32 v0, v4, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v5, v0
	v_subrev_nc_u32_e32 v5, s0, v0
	v_cmp_le_u32_e32 vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v0, v0, v5 :: v_dual_add_nc_u32 v3, 1, v4
	v_cndmask_b32_e32 v3, v4, v3, vcc_lo
	v_xor_b32_e32 v5, s1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s0, v0
	v_add_nc_u32_e32 v4, 1, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v3, v4, vcc_lo
	v_xor_b32_e32 v0, v0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v5
	v_mul_lo_u32 v3, v0, s5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v7, v1, v3
	v_mad_i64_i32 v[3:4], null, v0, s6, 0
	v_mad_i64_i32 v[5:6], null, v7, s6, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[7:8], 3, v[3:4]
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	v_lshlrev_b64 v[9:10], 3, v[5:6]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s8, v7
	v_add_co_ci_u32_e64 v6, null, s9, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s10, v9
	v_add_co_ci_u32_e64 v8, null, s11, v10, vcc_lo
	.p2align	6
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[9:10], v[5:6], off
	global_load_b64 v[11:12], v[7:8], off
	v_add_co_u32 v5, s0, v5, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, 0, v6, s0
	v_add_co_u32 v7, s0, v7, 8
	v_add_co_ci_u32_e64 v8, null, 0, v8, s0
	s_add_i32 s6, s6, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s6, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	v_cmp_gt_f64_e64 vcc_lo, |v[9:10]|, v[3:4]
	v_dual_cndmask_b32 v3, v3, v9 :: v_dual_and_b32 v0, 0x7fffffff, v10
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v4, v4, v0, vcc_lo
	s_cbranch_scc0 .LBB1_3
	s_branch .LBB1_5
.LBB1_4:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB1_5:
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB1_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel distancex_chebyshev_kernel
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
		.amdhsa_next_free_vgpr 13
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
	.text
.Lfunc_end1:
	.size	distancex_chebyshev_kernel, .Lfunc_end1-distancex_chebyshev_kernel
                                        ; -- End function
	.set distancex_chebyshev_kernel.num_vgpr, 13
	.set distancex_chebyshev_kernel.num_agpr, 0
	.set distancex_chebyshev_kernel.numbered_sgpr, 12
	.set distancex_chebyshev_kernel.num_named_barrier, 0
	.set distancex_chebyshev_kernel.private_seg_size, 0
	.set distancex_chebyshev_kernel.uses_vcc, 1
	.set distancex_chebyshev_kernel.uses_flat_scratch, 0
	.set distancex_chebyshev_kernel.has_dyn_sized_stack, 0
	.set distancex_chebyshev_kernel.has_recursion, 0
	.set distancex_chebyshev_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 524
; TotalNumSgprs: 14
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
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
	.protected	distancex_minkowski_kernel ; -- Begin function distancex_minkowski_kernel
	.globl	distancex_minkowski_kernel
	.p2align	8
	.type	distancex_minkowski_kernel,@function
distancex_minkowski_kernel:             ; @distancex_minkowski_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB2_6
; %bb.1:
	s_clause 0x2
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[12:13], s[0:1], 0x10
	s_load_b64 s[2:3], s[0:1], 0x28
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB2_4
; %bb.2:
	s_abs_i32 s0, s5
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s0
	s_sub_i32 s1, 0, s0
	s_mov_b32 s14, 0x55555555
	s_mov_b32 s16, 0x968915a9
	v_max_i32_e32 v7, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_mov_b32 s18, 0x4222de17
	s_mov_b32 s20, 0x3abe935a
	s_mov_b32 s22, 0x47e6c9c2
	s_mov_b32 s24, 0xcfa74449
	s_mov_b32 s26, 0x71bf3c30
	s_mov_b32 s28, 0x1c7792ce
	s_mov_b32 s30, 0x924920da
	s_mov_b32 s34, 0x9999999c
	s_mov_b32 s36, 0xfefa39ef
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_mov_b32 s38, 0x3b39803f
	s_mov_b32 s42, 0xd5df274d
	s_mov_b32 s44, 0x652b82fe
	s_mov_b32 s50, 0xfca7ab0c
	v_cvt_u32_f32_e32 v0, v0
	s_mov_b32 s52, 0x6a5dcb37
	s_mov_b32 s54, 0x7c89e6b0
	s_mov_b32 s56, 0x14761f6e
	s_mov_b32 s58, 0x1852b7b0
	v_mul_lo_u32 v3, s1, v0
	s_ashr_i32 s1, s5, 31
	s_mov_b32 s60, 0x11122322
	s_mov_b32 s62, 0x555502a1
	s_mov_b32 s15, 0x3fe55555
	s_mov_b32 s17, 0x3fba6564
	s_mov_b32 s19, 0x3fbdee67
	s_mov_b32 s21, 0x3fbe25e4
	v_mul_hi_u32 v3, v0, v3
	s_mov_b32 s23, 0x3fc110ef
	s_mov_b32 s25, 0x3fc3b13b
	s_mov_b32 s27, 0x3fc745d1
	s_mov_b32 s29, 0x3fcc71c7
	s_mov_b32 s31, 0x3fd24924
	s_mov_b32 s35, 0x3fd99999
	s_mov_b32 s37, 0x3fe62e42
	v_dual_mov_b32 v3, 0 :: v_dual_add_nc_u32 v8, v0, v3
	s_mov_b32 s39, 0x3c7abc9e
	s_mov_b32 s41, 0xbfe55555
	s_mov_b32 s43, 0x3c8543b0
	s_delay_alu instid0(VALU_DEP_1)
	v_mad_u64_u32 v[5:6], null, v7, v8, 0
	v_mov_b32_e32 v4, 0
	s_mov_b32 s45, 0x3ff71547
	s_mov_b32 s47, 0xbfe62e42
	s_mov_b32 s49, 0xbc7abc9e
	s_mov_b32 s51, 0x3e928af3
	s_mov_b32 s53, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_lo_u32 v5, v6, s0
	s_mov_b32 s55, 0x3efa0199
	s_mov_b32 s57, 0x3f2a01a0
	s_mov_b32 s59, 0x3f56c16c
	s_mov_b32 s61, 0x3f811111
	s_mov_b32 s63, 0x3fa55555
	s_mov_b32 s40, s14
	s_mov_b32 s46, s36
	v_sub_nc_u32_e32 v5, v7, v5
	v_add_nc_u32_e32 v7, 1, v6
	s_mov_b32 s48, s38
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v8, s0, v5
	v_cmp_le_u32_e32 vcc_lo, s0, v5
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v0, s3 :: v_dual_cndmask_b32 v5, v5, v8
	v_cndmask_b32_e32 v6, v6, v7, vcc_lo
	v_xor_b32_e32 v8, s1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s0, v5
	v_add_nc_u32_e32 v7, 1, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, v6, v7, vcc_lo
	v_xor_b32_e32 v5, v5, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v7, v5, v8
	v_mul_lo_u32 v5, v7, s5
	s_mov_b32 s4, 0x623fde64
	s_mov_b32 s5, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v9, v1, v5
	v_mad_i64_i32 v[5:6], null, v7, s6, 0
	v_mad_i64_i32 v[7:8], null, v9, s6, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s8, v5
	v_add_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s10, v7
	v_add_co_ci_u32_e64 v8, null, s11, v8, vcc_lo
	s_mov_b32 s8, 0x55555511
	s_mov_b32 s10, 11
	s_mov_b32 s9, 0x3fc55555
	s_mov_b32 s11, 0x3fe00000
.LBB2_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[9:10], v[5:6], off
	global_load_b64 v[11:12], v[7:8], off
	s_add_i32 s6, s6, -1
	s_waitcnt vmcnt(0)
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 vcc_lo, |v[9:10]|, 1.0
	v_cndmask_b32_e32 v12, 0x3ff00000, v0, vcc_lo
	v_cndmask_b32_e64 v11, 0, s2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[11:12]
	v_cmp_neq_f64_e64 s7, v[11:12], |v[11:12]|
	v_cndmask_b32_e32 v10, 0x3ff00000, v10, vcc_lo
	v_cndmask_b32_e32 v9, 0, v9, vcc_lo
	v_frexp_mant_f64_e64 v[13:14], |v[9:10]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[14:15], v[13:14]
	v_cndmask_b32_e64 v15, 0, 1, vcc_lo
	v_ldexp_f64 v[13:14], v[13:14], v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[13:14], 1.0
	v_add_f64 v[21:22], v[13:14], -1.0
	v_rcp_f64_e32 v[17:18], v[15:16]
	v_add_f64 v[23:24], v[15:16], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[13:14], -v[23:24]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[19:20], -v[15:16], v[17:18], 1.0
	v_fma_f64 v[17:18], v[19:20], v[17:18], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], -v[15:16], v[17:18], 1.0
	v_fma_f64 v[17:18], v[19:20], v[17:18], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[21:22], v[17:18]
	v_mul_f64 v[25:26], v[15:16], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[19:20], v[15:16], -v[25:26]
	v_fma_f64 v[13:14], v[19:20], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[25:26], v[13:14]
	v_add_f64 v[23:24], v[21:22], -v[15:16]
	v_add_f64 v[25:26], v[15:16], -v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[21:22], -v[23:24]
	v_add_f64 v[13:14], v[25:26], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[21:22], -v[15:16]
	v_add_f64 v[13:14], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[23:24], v[13:14]
	v_mul_f64 v[13:14], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[19:20], v[13:14]
	v_add_f64 v[17:18], v[15:16], -v[19:20]
	v_mul_f64 v[19:20], v[15:16], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	v_fma_f64 v[17:18], v[15:16], v[15:16], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[21:22], v[13:14], v[13:14]
	v_fma_f64 v[17:18], v[15:16], v[21:22], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[21:22], v[19:20], v[17:18]
	v_fma_f64 v[23:24], v[21:22], s[18:19], s[16:17]
	v_add_f64 v[19:20], v[21:22], -v[19:20]
	v_mul_f64 v[27:28], v[15:16], v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[20:21]
	v_add_f64 v[17:18], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[31:32], v[21:22], v[15:16], -v[27:28]
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[24:25]
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[28:29]
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], v[23:24], s[34:35]
	v_mul_f64 v[25:26], v[21:22], v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[19:20], v[21:22], v[23:24], -v[25:26]
	v_fma_f64 v[21:22], v[21:22], v[13:14], v[31:32]
	v_ldexp_f64 v[13:14], v[13:14], 1
	v_fma_f64 v[19:20], v[17:18], v[23:24], v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[17:18], v[15:16], v[21:22]
	v_ldexp_f64 v[15:16], v[15:16], 1
	v_add_f64 v[23:24], v[25:26], v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[27:28], v[17:18]
	v_add_f64 v[29:30], v[23:24], s[14:15]
	v_add_f64 v[25:26], v[23:24], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[27:28], v[21:22], -v[27:28]
	v_add_f64 v[33:34], v[29:30], s[40:41]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[19:20], -v[25:26]
	v_add_f64 v[17:18], v[17:18], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[23:24], -v[33:34]
	v_add_f64 v[19:20], v[19:20], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[19:20], v[19:20], v[23:24]
	v_add_f64 v[23:24], v[29:30], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[29:30], -v[23:24]
	v_mul_f64 v[29:30], v[21:22], v[23:24]
	v_add_f64 v[19:20], v[19:20], v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[21:22], v[23:24], -v[29:30]
	v_fma_f64 v[19:20], v[21:22], v[19:20], v[25:26]
	v_frexp_exp_i32_f64_e32 v21, v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[17:18], v[23:24], v[19:20]
	v_subrev_co_ci_u32_e64 v21, null, 0, v21, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[21:22], v21
	v_add_f64 v[19:20], v[29:30], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[27:28], v[21:22], s[36:37]
	v_add_f64 v[23:24], v[15:16], v[19:20]
	v_add_f64 v[25:26], v[19:20], -v[29:30]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[23:24], -v[15:16]
	v_add_f64 v[17:18], v[17:18], -v[25:26]
	v_fma_f64 v[25:26], v[21:22], s[36:37], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[19:20], -v[15:16]
	v_add_f64 v[13:14], v[13:14], v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[21:22], s[38:39], v[25:26]
	v_add_f64 v[13:14], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[27:28], v[17:18]
	v_add_f64 v[19:20], v[23:24], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[27:28], v[15:16], -v[27:28]
	v_add_f64 v[21:22], v[15:16], v[19:20]
	v_add_f64 v[23:24], v[19:20], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[17:18], -v[27:28]
	v_add_f64 v[25:26], v[21:22], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[23:24]
	v_add_f64 v[29:30], v[21:22], -v[25:26]
	v_add_f64 v[19:20], v[19:20], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[17:18], v[13:14]
	v_add_f64 v[15:16], v[15:16], -v[29:30]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[19:20], v[15:16]
	v_add_f64 v[19:20], v[23:24], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[23:24], v[15:16]
	v_add_f64 v[23:24], v[23:24], -v[19:20]
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[25:26], v[21:22], v[15:16]
	v_add_f64 v[17:18], v[17:18], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[25:26], -v[21:22]
	v_add_f64 v[13:14], v[13:14], v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[15:16], -v[19:20]
	v_add_f64 v[13:14], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[25:26], v[13:14]
	v_add_f64 v[17:18], v[15:16], -v[25:26]
	v_mul_f64 v[19:20], v[11:12], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	v_fma_f64 v[15:16], v[11:12], v[15:16], -v[19:20]
	v_cmp_class_f64_e64 vcc_lo, v[19:20], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], v[15:16]
	v_add_f64 v[15:16], v[19:20], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_dual_cndmask_b32 v18, v16, v20 :: v_dual_cndmask_b32 v17, v15, v19
	v_add_f64 v[15:16], v[15:16], -v[19:20]
	v_cmp_eq_f64_e64 s64, |v[9:10]|, 0
	v_cmp_class_f64_e64 s33, v[9:10], 0x204
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[17:18]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_cndmask_b32_e32 v14, 0, v14, vcc_lo
	v_mul_f64 v[21:22], v[17:18], s[44:45]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[17:18]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[17:18]
	v_cndmask_b32_e32 v13, 0, v13, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[21:22], v[21:22]
	s_and_b32 vcc_lo, s1, s0
	v_fma_f64 v[23:24], v[21:22], s[46:47], v[17:18]
	v_cvt_i32_f64_e32 v27, v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[23:24], v[21:22], s[48:49], v[23:24]
	v_fma_f64 v[25:26], v[23:24], s[52:53], s[50:51]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[4:5]
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[54:55]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[56:57]
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[58:59]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[60:61]
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[62:63]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[8:9]
	v_fma_f64 v[25:26], v[23:24], v[25:26], s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[25:26], v[23:24], v[25:26], 1.0
	v_fma_f64 v[21:22], v[23:24], v[25:26], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[19:20], v[21:22], v27
	v_cndmask_b32_e64 v15, 0x7ff00000, v20, s0
	v_cmp_lt_f64_e64 s0, |v[9:10]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v16, 0, v15, s1
	v_cndmask_b32_e32 v15, 0, v19, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[15:16], v[13:14], v[15:16]
	v_cmp_class_f64_e64 s1, v[15:16], 0x204
	s_xor_b32 s0, s7, s0
	v_cndmask_b32_e64 v17, 0x7ff00000, 0, s0
	v_cmp_neq_f64_e64 s0, |v[9:10]|, 1.0
	s_xor_b32 s7, vcc_lo, s64
	s_or_b32 vcc_lo, s64, s33
	v_cndmask_b32_e64 v14, v14, v16, s1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_and_b32_e32 v14, 0x7fffffff, v14
	v_cndmask_b32_e64 v16, 0x3ff00000, v17, s0
	v_cmp_class_f64_e64 s0, v[11:12], 0x204
	v_cndmask_b32_e64 v17, 0x7ff00000, 0, s7
	v_cndmask_b32_e64 v14, v14, v16, s0
	s_or_b32 s0, vcc_lo, s0
	s_cmp_eq_u32 s6, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v14, v14, v17, vcc_lo
	v_cmp_o_f64_e64 vcc_lo, |v[9:10]|, v[11:12]
	v_cndmask_b32_e64 v13, v13, v15, s1
	v_cndmask_b32_e64 v13, v13, 0, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v9, 0, v13, vcc_lo
	v_cndmask_b32_e32 v10, 0x7ff80000, v14, vcc_lo
	v_add_co_u32 v5, vcc_lo, v5, 8
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[9:10]
	v_add_co_u32 v7, vcc_lo, v7, 8
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	s_cbranch_scc0 .LBB2_3
	s_branch .LBB2_5
.LBB2_4:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB2_5:
	s_waitcnt lgkmcnt(0)
	v_div_scale_f64 v[5:6], null, s[2:3], s[2:3], 1.0
	v_div_scale_f64 v[11:12], vcc_lo, 1.0, s[2:3], 1.0
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s4, 0x4222de17
	s_mov_b32 s5, 0x3fbdee67
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
	v_cmp_neq_f64_e32 vcc_lo, 1.0, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[5:6], v[5:6], s[2:3], 1.0
	s_mov_b32 s2, 0x968915a9
	s_mov_b32 s3, 0x3fba6564
	v_cndmask_b32_e32 v6, 0x3ff00000, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[5:6]
	v_cndmask_b32_e32 v4, 0x3ff00000, v4, vcc_lo
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[7:8], |v[3:4]|
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[7:8]
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[7:8], v[7:8], v0
	v_frexp_exp_i32_f64_e32 v0, v[3:4]
	v_add_f64 v[9:10], v[7:8], 1.0
	v_add_f64 v[15:16], v[7:8], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	v_rcp_f64_e32 v[11:12], v[9:10]
	v_add_f64 v[17:18], v[9:10], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[13:14], v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[13:14], v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[13:14], v[15:16], v[11:12]
	v_mul_f64 v[19:20], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[13:14], v[9:10], -v[19:20]
	v_fma_f64 v[7:8], v[13:14], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[19:20], v[7:8]
	v_add_f64 v[17:18], v[15:16], -v[9:10]
	v_add_f64 v[19:20], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	v_add_f64 v[7:8], v[19:20], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[17:18], v[7:8]
	v_mul_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[13:14], v[7:8]
	v_add_f64 v[11:12], v[9:10], -v[13:14]
	v_mul_f64 v[13:14], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	v_fma_f64 v[11:12], v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[7:8], v[7:8]
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[13:14], v[11:12]
	v_fma_f64 v[17:18], v[15:16], s[4:5], s[2:3]
	s_mov_b32 s2, 0x3abe935a
	s_mov_b32 s3, 0x3fbe25e4
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_mul_f64 v[23:24], v[9:10], v[15:16]
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x47e6c9c2
	s_mov_b32 s3, 0x3fc110ef
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0xcfa74449
	s_mov_b32 s3, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x71bf3c30
	s_mov_b32 s3, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x1c7792ce
	s_mov_b32 s3, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x924920da
	s_mov_b32 s3, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s2, 0x9999999c
	s_mov_b32 s3, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[2:3]
	s_mov_b32 s3, 0x3c7abc9e
	s_mov_b32 s2, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[15:16], v[17:18]
	v_fma_f64 v[13:14], v[15:16], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[17:18], v[13:14]
	v_add_f64 v[17:18], v[19:20], v[13:14]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[21:22], v[17:18], s[0:1]
	v_add_f64 v[19:20], v[17:18], -v[19:20]
	s_mov_b32 s1, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[25:26], v[21:22], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	v_fma_f64 v[19:20], v[15:16], v[9:10], -v[23:24]
	s_mov_b32 s0, 0xd5df274d
	s_mov_b32 s1, 0x3c8543b0
	v_add_f64 v[17:18], v[17:18], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], s[0:1]
	v_fma_f64 v[15:16], v[15:16], v[7:8], v[19:20]
	s_mov_b32 s1, 0x3fe62e42
	s_mov_b32 s0, 0xfefa39ef
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[11:12], v[15:16], v[13:14]
	v_cvt_f64_i32_e32 v[15:16], v0
	v_add_f64 v[13:14], v[21:22], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[17:18], v[9:10], v[13:14]
	v_add_f64 v[19:20], v[13:14], -v[21:22]
	v_mul_f64 v[21:22], v[15:16], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[17:18], -v[9:10]
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[19:20], v[15:16], s[0:1], -v[21:22]
	s_mov_b32 s1, 0xbfe62e42
	v_add_f64 v[9:10], v[13:14], -v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[7:8], v[11:12]
	v_fma_f64 v[11:12], v[15:16], s[2:3], v[19:20]
	s_mov_b32 s3, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_add_f64 v[9:10], v[21:22], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[17:18], v[7:8]
	v_add_f64 v[21:22], v[9:10], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[9:10], v[13:14]
	v_add_f64 v[17:18], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[15:16], -v[19:20]
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	v_add_f64 v[17:18], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[9:10], -v[23:24]
	v_add_f64 v[9:10], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[17:18], -v[11:12]
	v_add_f64 v[9:10], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[17:18], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[19:20], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[13:14], v[19:20], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[11:12]
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_add_f64 v[9:10], v[19:20], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[9:10], -v[19:20]
	v_mul_f64 v[13:14], v[5:6], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[5:6], v[9:10], -v[13:14]
	v_cmp_class_f64_e64 vcc_lo, v[13:14], 0x204
	v_fma_f64 v[7:8], v[5:6], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[13:14], v[7:8]
	v_dual_cndmask_b32 v12, v10, v14 :: v_dual_cndmask_b32 v11, v9, v13
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[15:16], v[11:12], s[4:5]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[11:12]|
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_trunc_f64_e32 v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rndne_f64_e32 v[15:16], v[15:16]
	v_dual_cndmask_b32 v8, 0, v8 :: v_dual_cndmask_b32 v7, 0, v7
	v_cmp_lt_f64_e64 s4, |v[3:4]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], s[0:1], v[11:12]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_cvt_i32_f64_e32 v0, v[15:16]
	v_fma_f64 v[17:18], v[15:16], s[2:3], v[17:18]
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_cmp_neq_f64_e64 s3, v[5:6], |v[5:6]|
	v_cmp_eq_f64_e64 s2, 0, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_xor_b32 s3, s3, s4
	v_cmp_class_f64_e64 s4, v[3:4], 0x204
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[11:12]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	s_and_b32 vcc_lo, s1, s0
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[13:14], v[15:16], v0
	v_mul_f64 v[15:16], v[5:6], 0.5
	v_cndmask_b32_e64 v0, 0x7ff00000, v14, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[11:12], v[15:16]
	v_cndmask_b32_e32 v13, 0, v13, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[9:10], v[5:6]
	v_cndmask_b32_e64 v10, 0x7ff00000, 0, s3
	v_cndmask_b32_e64 v14, 0, v0, s1
	v_cmp_neq_f64_e64 s3, |v[3:4]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[7:8], v[13:14], v[7:8], v[13:14]
	v_cmp_class_f64_e64 s1, v[13:14], 0x204
	v_cmp_neq_f64_e64 s0, v[11:12], v[15:16]
	v_cndmask_b32_e64 v10, 0x3ff00000, v10, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v8, v8, v14, s1
	v_cndmask_b32_e64 v7, v7, v13, s1
	v_cmp_gt_f64_e64 s1, 0, v[5:6]
	v_cndmask_b32_e32 v9, 0, v7, vcc_lo
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v0, 0x3ff00000, v4, s0
	v_bfi_b32 v0, 0x7fffffff, v8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v8, 0x7ff80000, v0, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[3:4]
	s_xor_b32 s1, s1, s2
	v_dual_cndmask_b32 v7, v7, v9 :: v_dual_cndmask_b32 v0, v0, v8
	v_cmp_class_f64_e64 vcc_lo, v[5:6], 0x204
	v_cndmask_b32_e64 v8, 0x7ff00000, 0, s1
	v_cndmask_b32_e64 v9, 0, v4, s0
	s_or_b32 s0, s2, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_bfi_b32 v8, 0x7fffffff, v8, v9
	v_cndmask_b32_e32 v0, v0, v10, vcc_lo
	v_cndmask_b32_e64 v8, v0, v8, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[3:4], v[5:6]
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_cndmask_b32_e64 v7, v7, 0, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v2, 0, v7, vcc_lo
	v_cndmask_b32_e32 v3, 0x7ff80000, v8, vcc_lo
	v_add_co_u32 v0, vcc_lo, s12, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s13, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB2_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel distancex_minkowski_kernel
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
		.amdhsa_next_free_vgpr 35
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
		.amdhsa_inst_pref_size 38
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
	.size	distancex_minkowski_kernel, .Lfunc_end2-distancex_minkowski_kernel
                                        ; -- End function
	.set distancex_minkowski_kernel.num_vgpr, 35
	.set distancex_minkowski_kernel.num_agpr, 0
	.set distancex_minkowski_kernel.numbered_sgpr, 65
	.set distancex_minkowski_kernel.num_named_barrier, 0
	.set distancex_minkowski_kernel.private_seg_size, 0
	.set distancex_minkowski_kernel.uses_vcc, 1
	.set distancex_minkowski_kernel.uses_flat_scratch, 0
	.set distancex_minkowski_kernel.has_dyn_sized_stack, 0
	.set distancex_minkowski_kernel.has_recursion, 0
	.set distancex_minkowski_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4816
; TotalNumSgprs: 67
; NumVgprs: 35
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 67
; NumVGPRsForWavesPerEU: 35
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
	.protected	distancex_braycurtis_kernel ; -- Begin function distancex_braycurtis_kernel
	.globl	distancex_braycurtis_kernel
	.p2align	8
	.type	distancex_braycurtis_kernel,@function
distancex_braycurtis_kernel:            ; @distancex_braycurtis_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB3_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB3_4
; %bb.2:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v5, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s3, v0
	s_ashr_i32 s3, s5, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, v5, v0, 0
	v_mul_lo_u32 v0, v4, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v5, v0
	v_subrev_nc_u32_e32 v5, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v0, v0, v5 :: v_dual_add_nc_u32 v3, 1, v4
	v_cndmask_b32_e32 v3, v4, v3, vcc_lo
	v_xor_b32_e32 v5, s3, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	v_add_nc_u32_e32 v4, 1, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v3, v4, vcc_lo
	v_xor_b32_e32 v0, v0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v5
	v_mul_lo_u32 v3, v0, s5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v5, v1, v3
	v_mad_i64_i32 v[3:4], null, v0, s6, 0
	v_mad_i64_i32 v[7:8], null, v5, s6, 0
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[9:10], 3, v[3:4]
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	v_lshlrev_b64 v[11:12], 3, v[7:8]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v7, vcc_lo, s8, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v8, null, s9, v10, vcc_lo
	v_add_co_u32 v9, vcc_lo, s10, v11
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s11, v12, vcc_lo
	.p2align	6
.LBB3_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[11:12], v[7:8], off
	global_load_b64 v[13:14], v[9:10], off
	v_add_co_u32 v7, vcc_lo, v7, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	v_add_co_u32 v9, vcc_lo, v9, 8
	v_add_co_ci_u32_e64 v10, null, 0, v10, vcc_lo
	s_add_i32 s6, s6, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	s_cmp_eq_u32 s6, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[15:16], v[11:12], -v[13:14]
	v_add_f64 v[11:12], v[11:12], v[13:14]
	v_add_f64 v[5:6], v[5:6], |v[15:16]|
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], |v[11:12]|
	s_cbranch_scc0 .LBB3_3
	s_branch .LBB3_5
.LBB3_4:
	v_mov_b32_e32 v3, 0
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, 0
	v_mov_b32_e32 v6, 0
.LBB3_5:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_div_scale_f64 v[7:8], null, v[3:4], v[3:4], v[5:6]
	v_div_scale_f64 v[13:14], vcc_lo, v[5:6], v[3:4], v[5:6]
	v_lshlrev_b64 v[0:1], 3, v[1:2]
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
	v_cmp_lt_f64_e32 vcc_lo, 0, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[5:6], v[7:8], v[3:4], v[5:6]
	v_dual_cndmask_b32 v3, 0, v6 :: v_dual_cndmask_b32 v2, 0, v5
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB3_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel distancex_braycurtis_kernel
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
		.amdhsa_next_free_vgpr 17
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
.Lfunc_end3:
	.size	distancex_braycurtis_kernel, .Lfunc_end3-distancex_braycurtis_kernel
                                        ; -- End function
	.set distancex_braycurtis_kernel.num_vgpr, 17
	.set distancex_braycurtis_kernel.num_agpr, 0
	.set distancex_braycurtis_kernel.numbered_sgpr, 12
	.set distancex_braycurtis_kernel.num_named_barrier, 0
	.set distancex_braycurtis_kernel.private_seg_size, 0
	.set distancex_braycurtis_kernel.uses_vcc, 1
	.set distancex_braycurtis_kernel.uses_flat_scratch, 0
	.set distancex_braycurtis_kernel.has_dyn_sized_stack, 0
	.set distancex_braycurtis_kernel.has_recursion, 0
	.set distancex_braycurtis_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 660
; TotalNumSgprs: 14
; NumVgprs: 17
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 14
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
	.protected	distancex_canberra_kernel ; -- Begin function distancex_canberra_kernel
	.globl	distancex_canberra_kernel
	.p2align	8
	.type	distancex_canberra_kernel,@function
distancex_canberra_kernel:              ; @distancex_canberra_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB4_8
; %bb.1:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB4_6
; %bb.2:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v5, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s3, v0
	s_ashr_i32 s3, s5, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, v5, v0, 0
	v_mul_lo_u32 v0, v4, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v5, v0
	v_subrev_nc_u32_e32 v5, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v0, v0, v5 :: v_dual_add_nc_u32 v3, 1, v4
	v_cndmask_b32_e32 v3, v4, v3, vcc_lo
	v_xor_b32_e32 v5, s3, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	v_add_nc_u32_e32 v4, 1, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v3, v4, vcc_lo
	v_xor_b32_e32 v0, v0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v5
	v_mul_lo_u32 v3, v0, s5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v7, v1, v3
	v_mad_i64_i32 v[3:4], null, v0, s6, 0
	v_mad_i64_i32 v[5:6], null, v7, s6, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[7:8], 3, v[3:4]
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	v_lshlrev_b64 v[9:10], 3, v[5:6]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s8, v7
	v_add_co_ci_u32_e64 v6, null, s9, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s10, v9
	v_add_co_ci_u32_e64 v8, null, s11, v10, vcc_lo
	s_branch .LBB4_4
.LBB4_3:                                ;   in Loop: Header=BB4_4 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v5, vcc_lo, v5, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	v_add_co_u32 v7, vcc_lo, v7, 8
	v_add_co_ci_u32_e64 v8, null, 0, v8, vcc_lo
	s_add_i32 s6, s6, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s6, 0
	s_cbranch_scc1 .LBB4_7
.LBB4_4:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[11:12], v[5:6], off
	global_load_b64 v[13:14], v[7:8], off
	s_mov_b32 s2, exec_lo
	s_waitcnt vmcnt(0)
	v_add_f64 v[9:10], |v[11:12]|, |v[13:14]|
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_lt_f64_e32 0, v[9:10]
	s_cbranch_execz .LBB4_3
; %bb.5:                                ;   in Loop: Header=BB4_4 Depth=1
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mov_b32 v13, v11 :: v_dual_and_b32 v14, 0x7fffffff, v12
	v_div_scale_f64 v[15:16], null, v[9:10], v[9:10], v[13:14]
	v_div_scale_f64 v[13:14], vcc_lo, v[13:14], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[17:18], v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[19:20], -v[15:16], v[17:18], 1.0
	v_fma_f64 v[17:18], v[17:18], v[19:20], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], -v[15:16], v[17:18], 1.0
	v_fma_f64 v[17:18], v[17:18], v[19:20], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[13:14], v[17:18]
	v_fma_f64 v[13:14], -v[15:16], v[19:20], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[13:14], v[13:14], v[17:18], v[19:20]
	v_div_fixup_f64 v[9:10], v[13:14], v[9:10], |v[11:12]|
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[9:10]
	s_branch .LBB4_3
.LBB4_6:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB4_7:
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB4_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel distancex_canberra_kernel
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
		.amdhsa_next_free_vgpr 21
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
.Lfunc_end4:
	.size	distancex_canberra_kernel, .Lfunc_end4-distancex_canberra_kernel
                                        ; -- End function
	.set distancex_canberra_kernel.num_vgpr, 21
	.set distancex_canberra_kernel.num_agpr, 0
	.set distancex_canberra_kernel.numbered_sgpr, 12
	.set distancex_canberra_kernel.num_named_barrier, 0
	.set distancex_canberra_kernel.private_seg_size, 0
	.set distancex_canberra_kernel.uses_vcc, 1
	.set distancex_canberra_kernel.uses_flat_scratch, 0
	.set distancex_canberra_kernel.has_dyn_sized_stack, 0
	.set distancex_canberra_kernel.has_recursion, 0
	.set distancex_canberra_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 660
; TotalNumSgprs: 14
; NumVgprs: 21
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 21
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
	.type	__hip_cuid_d17a3cedc364a2da,@object ; @__hip_cuid_d17a3cedc364a2da
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_d17a3cedc364a2da
__hip_cuid_d17a3cedc364a2da:
	.byte	0                               ; 0x0
	.size	__hip_cuid_d17a3cedc364a2da, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_d17a3cedc364a2da
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
    .name:           distancex_manhattan_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         distancex_manhattan_kernel.kd
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
    .name:           distancex_chebyshev_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         distancex_chebyshev_kernel.kd
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
        .value_kind:     by_value
      - .offset:         40
        .size:           8
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
    .name:           distancex_minkowski_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     67
    .sgpr_spill_count: 0
    .symbol:         distancex_minkowski_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     35
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
    .name:           distancex_braycurtis_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         distancex_braycurtis_kernel.kd
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
    .name:           distancex_canberra_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         distancex_canberra_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     21
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
