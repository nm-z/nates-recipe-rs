	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	lgbm_histogram_kernel   ; -- Begin function lgbm_histogram_kernel
	.globl	lgbm_histogram_kernel
	.p2align	8
	.type	lgbm_histogram_kernel,@function
lgbm_histogram_kernel:                  ; @lgbm_histogram_kernel
; %bb.0:
	s_load_b128 s[20:23], s[0:1], 0x38
	v_lshlrev_b32_e32 v5, 2, v0
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_i32_e32 vcc_lo, s23, v0
	s_and_saveexec_b32 s4, vcc_lo
	s_cbranch_execz .LBB0_3
; %bb.1:
	s_load_b32 s3, s[0:1], 0x54
	v_dual_mov_b32 v2, 0 :: v_dual_lshlrev_b32 v1, 2, v0
	v_mov_b32_e32 v3, v0
	s_mov_b32 s7, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s5, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s6, s5, 2
.LBB0_2:                                ; =>This Inner Loop Header: Depth=1
	v_add_nc_u32_e32 v3, s5, v3
	ds_store_b32 v1, v2 offset:2048
	ds_store_2addr_stride64_b32 v1, v2, v2 offset1:4
	v_add_nc_u32_e32 v1, s6, v1
	v_cmp_le_i32_e64 s3, s23, v3
	s_or_b32 s7, s3, s7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s7
	s_cbranch_execnz .LBB0_2
.LBB0_3:
	s_or_b32 exec_lo, exec_lo, s4
	s_clause 0x1
	s_load_b256 s[12:19], s[0:1], 0x0
	s_load_b256 s[4:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_ashr_i32 s10, s2, 31
	s_mov_b32 s11, exec_lo
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_i32_e64 s21, v0
	s_cbranch_execz .LBB0_9
; %bb.4:
	s_load_b32 s3, s[0:1], 0x54
	s_mul_i32 s25, s21, s2
	v_dual_mov_b32 v6, 1.0 :: v_dual_mov_b32 v1, v0
	s_mul_hi_i32 s24, s21, s2
	s_add_u32 s12, s12, s25
	s_addc_u32 s13, s13, s24
	s_mov_b32 s25, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s24, s3, 0xffff
	s_branch .LBB0_6
.LBB0_5:                                ;   in Loop: Header=BB0_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s26
	v_add_nc_u32_e32 v1, s24, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e64 s3, s21, v1
	s_or_b32 s25, s3, s25
	s_and_not1_b32 exec_lo, exec_lo, s25
	s_cbranch_execz .LBB0_9
.LBB0_6:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s26, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	v_add_co_u32 v7, s3, s14, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s15, v4, s3
	global_load_b32 v7, v[7:8], off
	s_waitcnt vmcnt(0)
	v_cmpx_eq_u32_e64 s20, v7
	s_cbranch_execz .LBB0_5
; %bb.7:                                ;   in Loop: Header=BB0_6 Depth=1
	v_add_co_u32 v7, s3, s12, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s13, v2, s3
	global_load_u8 v2, v[7:8], off
	s_waitcnt vmcnt(0)
	v_cmp_gt_i32_e64 s3, s23, v2
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB0_5
; %bb.8:                                ;   in Loop: Header=BB0_6 Depth=1
	v_add_co_u32 v7, s3, s16, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s17, v4, s3
	global_load_b32 v7, v[7:8], off
	v_lshlrev_b32_e32 v8, 2, v2
	v_add_co_u32 v2, s3, s18, v3
	v_add_co_ci_u32_e64 v3, null, s19, v4, s3
	s_waitcnt vmcnt(0)
	ds_add_f32 v8, v7 offset:1024
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	ds_add_f32 v8, v2 offset:2048
	ds_add_f32 v8, v6
	s_branch .LBB0_5
.LBB0_9:
	s_or_b32 exec_lo, exec_lo, s11
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB0_12
; %bb.10:
	s_load_b32 s0, s[0:1], 0x54
	s_mul_i32 s1, s22, s20
	s_mul_hi_i32 s3, s22, s20
	s_add_u32 s2, s1, s2
	s_addc_u32 s3, s3, s10
	s_ashr_i32 s10, s23, 31
	s_mul_hi_u32 s11, s2, s23
	s_mul_i32 s1, s2, s23
	s_mul_i32 s2, s2, s10
	s_mul_i32 s3, s3, s23
	s_add_i32 s10, s11, s2
	s_mov_b32 s11, 0
	s_add_i32 s3, s10, s3
	s_waitcnt lgkmcnt(0)
	s_and_b32 s2, s0, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s10, s2, 2
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB0_11:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v4, 31, v0
	v_add_co_u32 v3, vcc_lo, s1, v0
	ds_load_2addr_stride64_b32 v[1:2], v5 offset1:4
	ds_load_b32 v10, v5 offset:2048
	v_add_co_ci_u32_e64 v4, null, s3, v4, vcc_lo
	v_add_nc_u32_e32 v0, s2, v0
	v_add_nc_u32_e32 v5, s10, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	v_cmp_le_i32_e32 vcc_lo, s23, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, s0, s4, v3
	v_add_co_ci_u32_e64 v7, null, s5, v4, s0
	v_add_co_u32 v8, s0, s6, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s7, v4, s0
	v_add_co_u32 v3, s0, s8, v3
	v_add_co_ci_u32_e64 v4, null, s9, v4, s0
	s_or_b32 s11, vcc_lo, s11
	s_waitcnt lgkmcnt(1)
	global_store_b32 v[6:7], v2, off
	s_waitcnt lgkmcnt(0)
	global_store_b32 v[8:9], v10, off
	global_store_b32 v[3:4], v1, off
	s_and_not1_b32 exec_lo, exec_lo, s11
	s_cbranch_execnz .LBB0_11
.LBB0_12:
	s_set_inst_prefetch_distance 0x2
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lgbm_histogram_kernel
		.amdhsa_group_segment_fixed_size 3072
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
		.amdhsa_next_free_vgpr 11
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
	.size	lgbm_histogram_kernel, .Lfunc_end0-lgbm_histogram_kernel
                                        ; -- End function
	.set lgbm_histogram_kernel.num_vgpr, 11
	.set lgbm_histogram_kernel.num_agpr, 0
	.set lgbm_histogram_kernel.numbered_sgpr, 27
	.set lgbm_histogram_kernel.num_named_barrier, 0
	.set lgbm_histogram_kernel.private_seg_size, 0
	.set lgbm_histogram_kernel.uses_vcc, 1
	.set lgbm_histogram_kernel.uses_flat_scratch, 0
	.set lgbm_histogram_kernel.has_dyn_sized_stack, 0
	.set lgbm_histogram_kernel.has_recursion, 0
	.set lgbm_histogram_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 744
; TotalNumSgprs: 29
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 3072 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 29
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
	.protected	lgbm_hist_subtract_kernel ; -- Begin function lgbm_hist_subtract_kernel
	.globl	lgbm_hist_subtract_kernel
	.p2align	8
	.type	lgbm_hist_subtract_kernel,@function
lgbm_hist_subtract_kernel:              ; @lgbm_hist_subtract_kernel
; %bb.0:
	s_load_b128 s[4:7], s[0:1], 0x18
	s_mov_b32 s3, exec_lo
	s_waitcnt lgkmcnt(0)
	v_cmpx_gt_i32_e64 s7, v0
	s_cbranch_execz .LBB1_3
; %bb.1:
	s_clause 0x2
	s_load_b32 s16, s[0:1], 0x34
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[12:13], s[0:1], 0x10
	s_mul_hi_i32 s3, s6, s4
	s_mul_i32 s4, s6, s4
	s_ashr_i32 s14, s2, 31
	s_add_u32 s4, s4, s2
	s_addc_u32 s3, s3, s14
	s_ashr_i32 s15, s7, 31
	s_mul_hi_u32 s1, s4, s7
	s_mul_i32 s0, s4, s15
	s_mul_i32 s3, s3, s7
	s_add_i32 s0, s1, s0
	s_mul_i32 s17, s6, s5
	s_add_i32 s1, s0, s3
	s_add_u32 s0, s17, s2
	s_mul_hi_i32 s2, s6, s5
	s_mul_i32 s3, s0, s15
	s_mul_hi_u32 s5, s0, s7
	s_addc_u32 s2, s2, s14
	s_add_i32 s3, s5, s3
	s_mul_i32 s5, s2, s7
	s_mul_i32 s2, s4, s7
	s_add_i32 s3, s3, s5
	s_mul_i32 s4, s0, s7
	s_waitcnt lgkmcnt(0)
	s_and_b32 s5, s16, 0xffff
	s_mov_b32 s6, 0
.LBB1_2:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v0
	v_add_co_u32 v3, s0, s4, v0
	v_add_co_u32 v1, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v4, null, s3, v2, s0
	v_add_co_ci_u32_e64 v2, null, s1, v2, vcc_lo
	v_add_nc_u32_e32 v0, s5, v0
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	v_add_co_u32 v5, vcc_lo, s8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v6, null, s9, v4, vcc_lo
	v_add_co_u32 v7, vcc_lo, s8, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s9, v2, vcc_lo
	s_clause 0x1
	global_load_b32 v10, v[5:6], off
	global_load_b32 v11, v[7:8], off
	v_add_co_u32 v5, vcc_lo, s10, v3
	v_add_co_ci_u32_e64 v6, null, s11, v4, vcc_lo
	v_add_co_u32 v9, vcc_lo, s10, v1
	s_waitcnt vmcnt(0)
	v_sub_f32_e32 v11, v11, v10
	v_add_co_ci_u32_e64 v10, null, s11, v2, vcc_lo
	v_add_co_u32 v3, vcc_lo, s12, v3
	global_store_b32 v[7:8], v11, off
	s_clause 0x1
	global_load_b32 v5, v[5:6], off
	global_load_b32 v6, v[9:10], off
	v_add_co_ci_u32_e64 v4, null, s13, v4, vcc_lo
	v_add_co_u32 v1, vcc_lo, s12, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s13, v2, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s7, v0
	s_or_b32 s6, vcc_lo, s6
	s_waitcnt vmcnt(0)
	v_sub_f32_e32 v5, v6, v5
	global_store_b32 v[9:10], v5, off
	s_clause 0x1
	global_load_b32 v3, v[3:4], off
	global_load_b32 v4, v[1:2], off
	s_waitcnt vmcnt(0)
	v_sub_f32_e32 v3, v4, v3
	global_store_b32 v[1:2], v3, off
	s_and_not1_b32 exec_lo, exec_lo, s6
	s_cbranch_execnz .LBB1_2
.LBB1_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lgbm_hist_subtract_kernel
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
		.amdhsa_next_free_vgpr 12
		.amdhsa_next_free_sgpr 18
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
	.size	lgbm_hist_subtract_kernel, .Lfunc_end1-lgbm_hist_subtract_kernel
                                        ; -- End function
	.set lgbm_hist_subtract_kernel.num_vgpr, 12
	.set lgbm_hist_subtract_kernel.num_agpr, 0
	.set lgbm_hist_subtract_kernel.numbered_sgpr, 18
	.set lgbm_hist_subtract_kernel.num_named_barrier, 0
	.set lgbm_hist_subtract_kernel.private_seg_size, 0
	.set lgbm_hist_subtract_kernel.uses_vcc, 1
	.set lgbm_hist_subtract_kernel.uses_flat_scratch, 0
	.set lgbm_hist_subtract_kernel.has_dyn_sized_stack, 0
	.set lgbm_hist_subtract_kernel.has_recursion, 0
	.set lgbm_hist_subtract_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 460
; TotalNumSgprs: 20
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 20
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
	.protected	lgbm_best_split_kernel  ; -- Begin function lgbm_best_split_kernel
	.globl	lgbm_best_split_kernel
	.p2align	8
	.type	lgbm_best_split_kernel,@function
lgbm_best_split_kernel:                 ; @lgbm_best_split_kernel
; %bb.0:
	s_load_b128 s[24:27], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s24
	s_cbranch_scc1 .LBB2_57
; %bb.1:
	s_load_b512 s[8:23], s[0:1], 0x0
	v_lshlrev_b32_e32 v1, 2, v0
	s_ashr_i32 s3, s2, 31
	s_cmp_lt_i32 s25, 1
	s_cbranch_scc1 .LBB2_27
; %bb.2:
	s_lshl_b64 s[4:5], s[2:3], 2
	s_load_b128 s[28:31], s[0:1], 0x50
	s_waitcnt lgkmcnt(0)
	s_add_u32 s4, s14, s4
	s_addc_u32 s5, s15, s5
	v_dual_mov_b32 v2, 0 :: v_dual_add_nc_u32 v5, 0x400, v1
	s_load_b32 s6, s[4:5], 0x0
	v_add_co_u32 v3, s5, s8, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s9, 0, s5
	v_add_co_u32 v6, s5, s10, v1
	v_add_co_ci_u32_e64 v7, null, s11, 0, s5
	v_add_co_u32 v9, s5, s12, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s13, 0, s5
	s_ashr_i32 s10, s26, 31
	s_load_b32 s8, s[28:29], 0x0
	s_load_b32 s9, s[30:31], 0x0
	v_cmp_le_i32_e64 s4, s26, v0
	v_dual_mov_b32 v11, -1 :: v_dual_add_nc_u32 v8, 0x800, v1
	v_dual_mov_b32 v13, 0xff7fffff :: v_dual_mov_b32 v14, -1
	s_waitcnt lgkmcnt(0)
	s_ashr_i32 s5, s6, 31
	s_mul_hi_u32 s7, s6, s25
	s_mul_i32 s5, s5, s25
	s_mov_b32 s15, 0
	s_add_i32 s11, s7, s5
	s_cmp_gt_i32 s26, 1
	v_mov_b32_e32 v15, 0
	s_cselect_b32 s12, -1, 0
	s_add_i32 s5, s26, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s7, s5, 2
	v_cmp_gt_i32_e64 s5, s5, v0
	s_add_i32 s13, s7, 0x800
	s_add_i32 s14, s7, 0x400
	v_mov_b32_e32 v12, s13
	s_mul_i32 s13, s6, s25
	s_and_saveexec_b32 s6, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s6, exec_lo, s6
	s_cbranch_execz .LBB2_4
.LBB2_3:
	ds_store_b32 v5, v2
	ds_store_b32 v8, v2
.LBB2_4:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB2_9 Depth 2
	s_or_saveexec_b32 s6, s6
	v_mov_b32_e32 v16, 0
	s_xor_b32 exec_lo, exec_lo, s6
	s_cbranch_execz .LBB2_6
; %bb.5:                                ;   in Loop: Header=BB2_4 Depth=1
	s_add_u32 s7, s13, s15
	s_addc_u32 s28, s11, 0
	s_mul_i32 s24, s7, s10
	s_mul_hi_u32 s27, s7, s26
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_add_i32 s24, s27, s24
	s_mul_i32 s27, s28, s26
	s_mul_i32 s28, s7, s26
	s_add_i32 s29, s24, s27
	s_lshl_b64 s[28:29], s[28:29], 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v16, vcc_lo, v3, s28
	v_add_co_ci_u32_e64 v17, null, s29, v4, vcc_lo
	v_add_co_u32 v18, vcc_lo, v6, s28
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v19, null, s29, v7, vcc_lo
	v_add_co_u32 v20, vcc_lo, v9, s28
	v_add_co_ci_u32_e64 v21, null, s29, v10, vcc_lo
	global_load_b32 v17, v[16:17], off
	global_load_b32 v18, v[18:19], off
	global_load_b32 v16, v[20:21], off
	s_waitcnt vmcnt(2)
	ds_store_b32 v5, v17
	s_waitcnt vmcnt(1)
	ds_store_b32 v8, v18
.LBB2_6:                                ;   in Loop: Header=BB2_4 Depth=1
	s_or_b32 exec_lo, exec_lo, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s12
	s_waitcnt vmcnt(0)
	ds_store_b32 v1, v16
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_vccnz .LBB2_15
; %bb.7:                                ;   in Loop: Header=BB2_4 Depth=1
	s_mov_b32 s6, 1
	s_branch .LBB2_9
.LBB2_8:                                ;   in Loop: Header=BB2_9 Depth=2
	s_or_b32 exec_lo, exec_lo, s7
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v18, v5
	ds_load_b32 v20, v8
	ds_load_b32 v21, v1
	s_lshl_b32 s6, s6, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_ge_i32 s6, s26
	s_waitcnt lgkmcnt(1)
	v_dual_add_f32 v17, v17, v18 :: v_dual_add_f32 v16, v16, v20
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v18, v19, v21
	ds_store_b32 v5, v17
	ds_store_b32 v8, v16
	ds_store_b32 v1, v18
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB2_15
.LBB2_9:                                ;   Parent Loop BB2_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_cmp_le_i32_e32 vcc_lo, s6, v0
	v_dual_mov_b32 v16, 0 :: v_dual_mov_b32 v17, 0
	v_subrev_nc_u32_e32 v18, s6, v0
	s_and_saveexec_b32 s7, vcc_lo
	s_cbranch_execnz .LBB2_12
; %bb.10:                               ;   in Loop: Header=BB2_9 Depth=2
	s_or_b32 exec_lo, exec_lo, s7
	s_and_saveexec_b32 s7, vcc_lo
	s_cbranch_execnz .LBB2_13
.LBB2_11:                               ;   in Loop: Header=BB2_9 Depth=2
	s_or_b32 exec_lo, exec_lo, s7
	v_mov_b32_e32 v19, 0
	s_and_saveexec_b32 s7, vcc_lo
	s_cbranch_execz .LBB2_8
	s_branch .LBB2_14
.LBB2_12:                               ;   in Loop: Header=BB2_9 Depth=2
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b32_e32 v17, 2, v18
	ds_load_b32 v17, v17 offset:1024
	s_or_b32 exec_lo, exec_lo, s7
	s_and_saveexec_b32 s7, vcc_lo
	s_cbranch_execz .LBB2_11
.LBB2_13:                               ;   in Loop: Header=BB2_9 Depth=2
	v_lshlrev_b32_e32 v16, 2, v18
	ds_load_b32 v16, v16 offset:2048
	s_or_b32 exec_lo, exec_lo, s7
	v_mov_b32_e32 v19, 0
	s_and_saveexec_b32 s7, vcc_lo
	s_cbranch_execz .LBB2_8
.LBB2_14:                               ;   in Loop: Header=BB2_9 Depth=2
	v_lshlrev_b32_e32 v18, 2, v18
	ds_load_b32 v19, v18
	s_branch .LBB2_8
.LBB2_15:                               ;   in Loop: Header=BB2_4 Depth=1
	ds_load_b32 v20, v12
	s_mov_b32 s6, -1
                                        ; implicit-def: $vgpr17
                                        ; implicit-def: $vgpr18
                                        ; implicit-def: $vgpr19
                                        ; implicit-def: $vgpr16
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, 0x2edbe6ff, v20
	s_cbranch_vccnz .LBB2_18
; %bb.16:                               ;   in Loop: Header=BB2_4 Depth=1
	s_and_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccnz .LBB2_25
.LBB2_17:                               ;   in Loop: Header=BB2_4 Depth=1
	s_add_i32 s15, s15, 1
	buffer_gl0_inv
	s_cmp_eq_u32 s15, s25
	s_cbranch_scc0 .LBB2_26
	s_branch .LBB2_28
.LBB2_18:                               ;   in Loop: Header=BB2_4 Depth=1
	v_dual_mov_b32 v16, v15 :: v_dual_mov_b32 v19, v14
	v_dual_mov_b32 v18, v11 :: v_dual_mov_b32 v17, v13
	s_and_saveexec_b32 s24, s5
	s_cbranch_execz .LBB2_24
; %bb.19:                               ;   in Loop: Header=BB2_4 Depth=1
	ds_load_b32 v22, v8
	v_dual_mov_b32 v17, v13 :: v_dual_mov_b32 v16, v15
	v_dual_mov_b32 v19, v14 :: v_dual_mov_b32 v18, v11
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e32 v21, v20, v22
	v_cmp_le_f32_e32 vcc_lo, s9, v22
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_f32_e64 s6, s9, v21
	s_and_b32 s6, vcc_lo, s6
	s_and_saveexec_b32 s27, s6
	s_cbranch_execz .LBB2_23
; %bb.20:                               ;   in Loop: Header=BB2_4 Depth=1
	v_dual_mov_b32 v16, s14 :: v_dual_add_f32 v19, s8, v22
	v_dual_add_f32 v18, s8, v20 :: v_dual_add_f32 v21, s8, v21
	ds_load_b32 v16, v16
	ds_load_b32 v17, v5
	s_waitcnt lgkmcnt(1)
	v_mul_f32_e32 v20, v16, v16
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e32 v16, v16, v17
	v_mul_f32_e32 v17, v17, v17
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_scale_f32 v22, null, v18, v18, v20
	v_mul_f32_e32 v16, v16, v16
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_scale_f32 v23, null, v19, v19, v17
	v_rcp_f32_e32 v24, v22
	v_div_scale_f32 v29, vcc_lo, v20, v18, v20
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_scale_f32 v25, null, v21, v21, v16
	v_rcp_f32_e32 v26, v23
	v_div_scale_f32 v31, s6, v17, v19, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(TRANS32_DEP_3)
	v_rcp_f32_e32 v27, v25
	v_fma_f32 v28, -v22, v24, 1.0
	s_waitcnt_depctr 0xfff
	v_fma_f32 v30, -v23, v26, 1.0
	v_fmac_f32_e32 v24, v28, v24
	v_fma_f32 v28, -v25, v27, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v26, v30, v26
	v_dual_mul_f32 v32, v29, v24 :: v_dual_fmac_f32 v27, v28, v27
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v28, v31, v26
	v_fma_f32 v33, -v22, v32, v29
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v35, -v23, v28, v31
	v_fmac_f32_e32 v32, v33, v24
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v28, v35, v26
	v_fma_f32 v22, -v22, v32, v29
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v23, -v23, v28, v31
	v_div_fmas_f32 v22, v22, v24, v32
	s_mov_b32 vcc_lo, s6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fmas_f32 v23, v23, v26, v28
	v_div_fixup_f32 v18, v22, v18, v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v17, v23, v19, v17
	v_mov_b32_e32 v19, v14
	v_div_scale_f32 v30, s7, v16, v21, v16
	s_mov_b32 vcc_lo, s7
	v_mul_f32_e32 v34, v30, v27
	v_fma_f32 v33, -v25, v34, v30
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v34, v33, v27
	v_fma_f32 v24, -v25, v34, v30
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v24, v24, v27, v34
	v_div_fixup_f32 v16, v24, v21, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v16, v17, v16
	v_dual_mov_b32 v17, v13 :: v_dual_sub_f32 v20, v16, v18
	v_mov_b32_e32 v18, v11
	v_mov_b32_e32 v16, v15
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_class_f32_e64 s6, v20, 0x1f8
	v_cmp_gt_f32_e32 vcc_lo, v20, v13
	s_and_b32 s7, s6, vcc_lo
	s_and_saveexec_b32 s6, s7
; %bb.21:                               ;   in Loop: Header=BB2_4 Depth=1
	ds_load_b32 v16, v1
	v_dual_mov_b32 v18, s15 :: v_dual_mov_b32 v17, v20
	v_mov_b32_e32 v19, v0
; %bb.22:                               ;   in Loop: Header=BB2_4 Depth=1
	s_or_b32 exec_lo, exec_lo, s6
.LBB2_23:                               ;   in Loop: Header=BB2_4 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s27
.LBB2_24:                               ;   in Loop: Header=BB2_4 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s24
	s_waitcnt lgkmcnt(0)
	s_barrier
	s_branch .LBB2_17
.LBB2_25:                               ;   in Loop: Header=BB2_4 Depth=1
	v_dual_mov_b32 v16, v15 :: v_dual_mov_b32 v19, v14
	v_dual_mov_b32 v18, v11 :: v_dual_mov_b32 v17, v13
	s_barrier
	s_add_i32 s15, s15, 1
	buffer_gl0_inv
	s_cmp_eq_u32 s15, s25
	s_cbranch_scc1 .LBB2_28
.LBB2_26:                               ;   in Loop: Header=BB2_4 Depth=1
	v_dual_mov_b32 v13, v17 :: v_dual_mov_b32 v14, v19
	v_mov_b32_e32 v11, v18
	v_mov_b32_e32 v15, v16
	s_and_saveexec_b32 s6, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s6, exec_lo, s6
	s_cbranch_execnz .LBB2_3
	s_branch .LBB2_4
.LBB2_27:
	v_dual_mov_b32 v16, 0 :: v_dual_mov_b32 v17, 0xff7fffff
	v_dual_mov_b32 v18, -1 :: v_dual_mov_b32 v19, -1
.LBB2_28:
	ds_store_2addr_stride64_b32 v1, v18, v17 offset0:20 offset1:24
	ds_store_2addr_stride64_b32 v1, v16, v19 offset0:12 offset1:16
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_mov_b32 s4, exec_lo
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB2_57
; %bb.29:
	s_load_b32 s0, s[0:1], 0x6c
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, -1
	v_dual_mov_b32 v2, -1 :: v_dual_mov_b32 v3, 0xff7fffff
	s_waitcnt lgkmcnt(0)
	s_and_b32 s0, s0, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s0, 0
	s_cbranch_scc1 .LBB2_56
; %bb.30:
	s_cmp_lt_u32 s0, 8
	s_cbranch_scc1 .LBB2_49
; %bb.31:
	s_and_b32 s1, s0, 0xfff8
	s_mov_b32 s4, 0
	s_movk_i32 s5, 0xc00
	s_branch .LBB2_33
.LBB2_32:                               ;   in Loop: Header=BB2_33 Depth=1
	s_add_i32 s4, s4, 8
	s_add_i32 s5, s5, 32
	s_cmp_eq_u32 s1, s4
	s_cbranch_scc1 .LBB2_50
.LBB2_33:                               ; =>This Inner Loop Header: Depth=1
	v_mov_b32_e32 v4, s5
	ds_load_b32 v4, v4 offset:3072
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_35
; %bb.34:                               ;   in Loop: Header=BB2_33 Depth=1
	v_dual_mov_b32 v0, s5 :: v_dual_mov_b32 v3, v4
	ds_load_b32 v2, v0 offset:2048
	ds_load_2addr_stride64_b32 v[0:1], v0 offset1:4
.LBB2_35:                               ;   in Loop: Header=BB2_33 Depth=1
	v_mov_b32_e32 v4, s5
	ds_load_b32 v4, v4 offset:3076
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_37
; %bb.36:                               ;   in Loop: Header=BB2_33 Depth=1
	v_dual_mov_b32 v0, s5 :: v_dual_mov_b32 v3, v4
	v_add_nc_u32_e64 v1, 4, s5
	ds_load_b32 v2, v0 offset:2052
	ds_load_2addr_stride64_b32 v[0:1], v1 offset1:4
.LBB2_37:                               ;   in Loop: Header=BB2_33 Depth=1
	v_mov_b32_e32 v4, s5
	ds_load_b32 v4, v4 offset:3080
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_39
; %bb.38:                               ;   in Loop: Header=BB2_33 Depth=1
	v_dual_mov_b32 v0, s5 :: v_dual_mov_b32 v3, v4
	v_add_nc_u32_e64 v1, 8, s5
	ds_load_b32 v2, v0 offset:2056
	ds_load_2addr_stride64_b32 v[0:1], v1 offset1:4
.LBB2_39:                               ;   in Loop: Header=BB2_33 Depth=1
	v_mov_b32_e32 v4, s5
	ds_load_b32 v4, v4 offset:3084
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_41
; %bb.40:                               ;   in Loop: Header=BB2_33 Depth=1
	v_dual_mov_b32 v0, s5 :: v_dual_mov_b32 v3, v4
	v_add_nc_u32_e64 v1, 12, s5
	ds_load_b32 v2, v0 offset:2060
	ds_load_2addr_stride64_b32 v[0:1], v1 offset1:4
.LBB2_41:                               ;   in Loop: Header=BB2_33 Depth=1
	v_mov_b32_e32 v4, s5
	ds_load_b32 v4, v4 offset:3088
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_43
; %bb.42:                               ;   in Loop: Header=BB2_33 Depth=1
	v_dual_mov_b32 v0, s5 :: v_dual_mov_b32 v3, v4
	v_add_nc_u32_e64 v1, 16, s5
	ds_load_b32 v2, v0 offset:2064
	ds_load_2addr_stride64_b32 v[0:1], v1 offset1:4
.LBB2_43:                               ;   in Loop: Header=BB2_33 Depth=1
	v_mov_b32_e32 v4, s5
	ds_load_b32 v4, v4 offset:3092
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_45
; %bb.44:                               ;   in Loop: Header=BB2_33 Depth=1
	v_dual_mov_b32 v0, s5 :: v_dual_mov_b32 v3, v4
	v_add_nc_u32_e64 v1, 20, s5
	ds_load_b32 v2, v0 offset:2068
	ds_load_2addr_stride64_b32 v[0:1], v1 offset1:4
.LBB2_45:                               ;   in Loop: Header=BB2_33 Depth=1
	v_mov_b32_e32 v4, s5
	ds_load_b32 v4, v4 offset:3096
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_47
; %bb.46:                               ;   in Loop: Header=BB2_33 Depth=1
	v_dual_mov_b32 v0, s5 :: v_dual_mov_b32 v3, v4
	v_add_nc_u32_e64 v1, 24, s5
	ds_load_b32 v2, v0 offset:2072
	ds_load_2addr_stride64_b32 v[0:1], v1 offset1:4
.LBB2_47:                               ;   in Loop: Header=BB2_33 Depth=1
	v_mov_b32_e32 v4, s5
	ds_load_b32 v4, v4 offset:3100
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_32
; %bb.48:                               ;   in Loop: Header=BB2_33 Depth=1
	v_dual_mov_b32 v0, s5 :: v_dual_mov_b32 v3, v4
	v_add_nc_u32_e64 v1, 28, s5
	ds_load_b32 v2, v0 offset:2076
	ds_load_2addr_stride64_b32 v[0:1], v1 offset1:4
	s_branch .LBB2_32
.LBB2_49:
	s_mov_b32 s1, 0
.LBB2_50:
	s_and_b32 s0, s0, 7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s0, 0
	s_cbranch_scc1 .LBB2_55
; %bb.51:
	s_lshl_b32 s1, s1, 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s1, 0xc00
	s_branch .LBB2_53
	.p2align	6
.LBB2_52:                               ;   in Loop: Header=BB2_53 Depth=1
	s_add_i32 s0, s0, -1
	s_add_i32 s1, s1, 4
	s_cmp_lg_u32 s0, 0
	s_cbranch_scc0 .LBB2_55
.LBB2_53:                               ; =>This Inner Loop Header: Depth=1
	v_mov_b32_e32 v4, s1
	ds_load_b32 v4, v4 offset:3072
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f32_e32 vcc_lo, v4, v3
	s_cbranch_vccnz .LBB2_52
; %bb.54:                               ;   in Loop: Header=BB2_53 Depth=1
	v_dual_mov_b32 v0, s1 :: v_dual_mov_b32 v3, v4
	ds_load_b32 v2, v0 offset:2048
	ds_load_2addr_stride64_b32 v[0:1], v0 offset1:4
	s_branch .LBB2_52
.LBB2_55:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v0, 0.5, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_i32_f32_e32 v0, v0
.LBB2_56:
	s_lshl_b64 s[0:1], s[2:3], 2
	v_mov_b32_e32 v4, 0
	s_add_u32 s2, s16, s0
	s_addc_u32 s3, s17, s1
	s_add_u32 s4, s18, s0
	s_addc_u32 s5, s19, s1
	s_clause 0x1
	global_store_b32 v4, v3, s[2:3]
	global_store_b32 v4, v2, s[4:5]
	s_add_u32 s2, s20, s0
	s_addc_u32 s3, s21, s1
	s_add_u32 s0, s22, s0
	s_addc_u32 s1, s23, s1
	s_clause 0x1
	global_store_b32 v4, v1, s[2:3]
	global_store_b32 v4, v0, s[0:1]
.LBB2_57:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lgbm_best_split_kernel
		.amdhsa_group_segment_fixed_size 7168
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 352
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
		.amdhsa_next_free_vgpr 36
		.amdhsa_next_free_sgpr 32
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
	.text
.Lfunc_end2:
	.size	lgbm_best_split_kernel, .Lfunc_end2-lgbm_best_split_kernel
                                        ; -- End function
	.set lgbm_best_split_kernel.num_vgpr, 36
	.set lgbm_best_split_kernel.num_agpr, 0
	.set lgbm_best_split_kernel.numbered_sgpr, 32
	.set lgbm_best_split_kernel.num_named_barrier, 0
	.set lgbm_best_split_kernel.private_seg_size, 0
	.set lgbm_best_split_kernel.uses_vcc, 1
	.set lgbm_best_split_kernel.uses_flat_scratch, 0
	.set lgbm_best_split_kernel.has_dyn_sized_stack, 0
	.set lgbm_best_split_kernel.has_recursion, 0
	.set lgbm_best_split_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2240
; TotalNumSgprs: 34
; NumVgprs: 36
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 7168 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 34
; NumVGPRsForWavesPerEU: 36
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
	.protected	lgbm_leaf_reduce_kernel ; -- Begin function lgbm_leaf_reduce_kernel
	.globl	lgbm_leaf_reduce_kernel
	.p2align	8
	.type	lgbm_leaf_reduce_kernel,@function
lgbm_leaf_reduce_kernel:                ; @lgbm_leaf_reduce_kernel
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
	s_cbranch_execz .LBB3_5
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_mov_b32 s2, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v6, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v7, null, s7, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 2, v[2:3]
	v_add_co_u32 v4, vcc_lo, s10, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s11, v3, vcc_lo
	global_load_b32 v8, v[6:7], off
	global_load_b32 v7, v[4:5], off
.LBB3_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v6, v7, v8
	global_atomic_cmpswap_b32 v6, v[4:5], v[6:7], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v6, v7
	v_mov_b32_e32 v7, v6
	s_or_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB3_2
; %bb.3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v4, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s9, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s0, v2
	v_add_co_ci_u32_e64 v1, null, s1, v3, vcc_lo
	global_load_b32 v4, v[4:5], off
	global_load_b32 v3, v[0:1], off
	s_mov_b32 s0, 0
.LBB3_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v3, v4
	global_atomic_cmpswap_b32 v2, v[0:1], v[2:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v2, v3
	v_mov_b32_e32 v3, v2
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB3_4
.LBB3_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lgbm_leaf_reduce_kernel
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
		.amdhsa_next_free_vgpr 9
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
.Lfunc_end3:
	.size	lgbm_leaf_reduce_kernel, .Lfunc_end3-lgbm_leaf_reduce_kernel
                                        ; -- End function
	.set lgbm_leaf_reduce_kernel.num_vgpr, 9
	.set lgbm_leaf_reduce_kernel.num_agpr, 0
	.set lgbm_leaf_reduce_kernel.numbered_sgpr, 12
	.set lgbm_leaf_reduce_kernel.num_named_barrier, 0
	.set lgbm_leaf_reduce_kernel.private_seg_size, 0
	.set lgbm_leaf_reduce_kernel.uses_vcc, 1
	.set lgbm_leaf_reduce_kernel.uses_flat_scratch, 0
	.set lgbm_leaf_reduce_kernel.has_dyn_sized_stack, 0
	.set lgbm_leaf_reduce_kernel.has_recursion, 0
	.set lgbm_leaf_reduce_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 352
; TotalNumSgprs: 14
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
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
	.protected	goss_sample_kernel      ; -- Begin function goss_sample_kernel
	.globl	goss_sample_kernel
	.p2align	8
	.type	goss_sample_kernel,@function
goss_sample_kernel:                     ; @goss_sample_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB4_5
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	v_cmp_gt_i32_e64 s4, s9, v1
	v_cmp_le_i32_e32 vcc_lo, s9, v1
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	v_mov_b32_e32 v2, 1.0
	s_and_saveexec_b32 s5, vcc_lo
	s_cbranch_execz .LBB4_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v3, null, s3, v1, vcc_lo
	s_load_b128 s[0:3], s[0:1], 0x20
	global_load_b32 v2, v[2:3], off
	s_waitcnt lgkmcnt(0)
	s_load_b32 s0, s[0:1], 0x0
	s_load_b32 s1, s[2:3], 0x0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_cmp_gt_f32_e32 vcc_lo, s0, v2
	v_mov_b32_e32 v2, s1
	s_and_not1_b32 s0, s4, exec_lo
	s_and_b32 s1, vcc_lo, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 s4, s0, s1
.LBB4_3:
	s_or_b32 exec_lo, exec_lo, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 exec_lo, exec_lo, s4
	s_cbranch_execz .LBB4_5
; %bb.4:
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b32 v[0:1], v2, off
.LBB4_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel goss_sample_kernel
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
		.amdhsa_next_free_vgpr 4
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
.Lfunc_end4:
	.size	goss_sample_kernel, .Lfunc_end4-goss_sample_kernel
                                        ; -- End function
	.set goss_sample_kernel.num_vgpr, 4
	.set goss_sample_kernel.num_agpr, 0
	.set goss_sample_kernel.numbered_sgpr, 10
	.set goss_sample_kernel.num_named_barrier, 0
	.set goss_sample_kernel.private_seg_size, 0
	.set goss_sample_kernel.uses_vcc, 1
	.set goss_sample_kernel.uses_flat_scratch, 0
	.set goss_sample_kernel.has_dyn_sized_stack, 0
	.set goss_sample_kernel.has_recursion, 0
	.set goss_sample_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 304
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
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	leaf_split_apply_kernel ; -- Begin function leaf_split_apply_kernel
	.globl	leaf_split_apply_kernel
	.p2align	8
	.type	leaf_split_apply_kernel,@function
leaf_split_apply_kernel:                ; @leaf_split_apply_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[8:9], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s9, v1
	s_cbranch_execz .LBB5_3
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s3, v4, vcc_lo
	global_load_b32 v0, v[3:4], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, s4, v0
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB5_3
; %bb.2:
	s_mul_i32 s3, s9, s7
	s_and_b32 s2, s8, 0xff
	s_mul_hi_i32 s4, s9, s7
	s_add_u32 s0, s0, s3
	s_addc_u32 s1, s1, s4
	v_add_co_u32 v0, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, s1, v2, vcc_lo
	global_load_d16_u8 v0, v[0:1], off
	v_mov_b32_e32 v1, s6
	s_waitcnt vmcnt(0)
	v_cmp_lt_u16_e32 vcc_lo, s2, v0.l
	v_cndmask_b32_e32 v0, s5, v1, vcc_lo
	global_store_b32 v[3:4], v0, off
.LBB5_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel leaf_split_apply_kernel
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
	.size	leaf_split_apply_kernel, .Lfunc_end5-leaf_split_apply_kernel
                                        ; -- End function
	.set leaf_split_apply_kernel.num_vgpr, 5
	.set leaf_split_apply_kernel.num_agpr, 0
	.set leaf_split_apply_kernel.numbered_sgpr, 10
	.set leaf_split_apply_kernel.num_named_barrier, 0
	.set leaf_split_apply_kernel.private_seg_size, 0
	.set leaf_split_apply_kernel.uses_vcc, 1
	.set leaf_split_apply_kernel.uses_flat_scratch, 0
	.set leaf_split_apply_kernel.has_dyn_sized_stack, 0
	.set leaf_split_apply_kernel.has_recursion, 0
	.set leaf_split_apply_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 212
; TotalNumSgprs: 12
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 12
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_cab2fe6cf073248b,@object ; @__hip_cuid_cab2fe6cf073248b
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_cab2fe6cf073248b
__hip_cuid_cab2fe6cf073248b:
	.byte	0                               ; 0x0
	.size	__hip_cuid_cab2fe6cf073248b, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_cab2fe6cf073248b
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
    .group_segment_fixed_size: 3072
    .kernarg_segment_align: 8
    .kernarg_segment_size: 328
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lgbm_histogram_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     29
    .sgpr_spill_count: 0
    .symbol:         lgbm_histogram_kernel.kd
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
    .name:           lgbm_hist_subtract_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     20
    .sgpr_spill_count: 0
    .symbol:         lgbm_hist_subtract_kernel.kd
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
      - .address_space:  global
        .offset:         80
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         88
        .size:           8
        .value_kind:     global_buffer
      - .offset:         96
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         100
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         104
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         108
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         110
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         112
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         114
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         116
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         118
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         136
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         144
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         152
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         160
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 7168
    .kernarg_segment_align: 8
    .kernarg_segment_size: 352
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           lgbm_best_split_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     34
    .sgpr_spill_count: 0
    .symbol:         lgbm_best_split_kernel.kd
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
    .name:           lgbm_leaf_reduce_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         lgbm_leaf_reduce_kernel.kd
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
    .name:           goss_sample_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         goss_sample_kernel.kd
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
        .size:           1
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
    .name:           leaf_split_apply_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         leaf_split_apply_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     5
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
