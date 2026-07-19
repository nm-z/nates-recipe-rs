	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	searchx_searchsorted_kernel ; -- Begin function searchx_searchsorted_kernel
	.globl	searchx_searchsorted_kernel
	.p2align	8
	.type	searchx_searchsorted_kernel,@function
searchx_searchsorted_kernel:            ; @searchx_searchsorted_kernel
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
	s_cbranch_execz .LBB0_7
; %bb.1:
	s_clause 0x1
	s_load_b32 s2, s[0:1], 0x8
	s_load_b128 s[4:7], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s2, 1
	s_cbranch_scc1 .LBB0_5
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[3:4], 3, v[1:2]
	s_load_b64 s[0:1], s[0:1], 0x0
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v7, s2
	v_mov_b32_e32 v0, 0
	s_cmp_eq_u32 s9, 0
	v_add_co_u32 v3, vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s5, v4, vcc_lo
	s_mov_b32 s2, 0
	s_cselect_b32 s3, -1, 0
	global_load_b64 v[3:4], v[3:4], off
	.p2align	6
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	v_sub_nc_u32_e32 v5, v7, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshrrev_b32_e32 v5, 1, v5
	v_add_nc_u32_e32 v5, v5, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[8:9], 3, v[5:6]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v8, vcc_lo, s0, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s1, v9, vcc_lo
	global_load_b64 v[8:9], v[8:9], off
	s_waitcnt vmcnt(0)
	v_cmp_le_f64_e32 vcc_lo, v[8:9], v[3:4]
	v_cndmask_b32_e64 v10, 0, 1, vcc_lo
	v_cmp_lt_f64_e32 vcc_lo, v[8:9], v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mov_b16_e32 v8.l, v10.l
	v_cndmask_b32_e64 v9, 0, 1, vcc_lo
	v_cndmask_b16 v8.l, v8.l, v9.l, s3
	v_add_nc_u32_e32 v9, 1, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b16 v8.l, 1, v8.l
	v_cmp_eq_u16_e32 vcc_lo, 1, v8.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v0, v0, v9 :: v_dual_cndmask_b32 v7, v5, v7
	v_cmp_ge_i32_e32 vcc_lo, v0, v7
	s_or_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB0_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s2
	s_branch .LBB0_6
.LBB0_5:
	v_mov_b32_e32 v0, 0
.LBB0_6:
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, vcc_lo, s6, v1
	v_add_co_ci_u32_e64 v2, null, s7, v2, vcc_lo
	global_store_b32 v[1:2], v0, off
.LBB0_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_searchsorted_kernel
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
.Lfunc_end0:
	.size	searchx_searchsorted_kernel, .Lfunc_end0-searchx_searchsorted_kernel
                                        ; -- End function
	.set searchx_searchsorted_kernel.num_vgpr, 11
	.set searchx_searchsorted_kernel.num_agpr, 0
	.set searchx_searchsorted_kernel.numbered_sgpr, 10
	.set searchx_searchsorted_kernel.num_named_barrier, 0
	.set searchx_searchsorted_kernel.private_seg_size, 0
	.set searchx_searchsorted_kernel.uses_vcc, 1
	.set searchx_searchsorted_kernel.uses_flat_scratch, 0
	.set searchx_searchsorted_kernel.has_dyn_sized_stack, 0
	.set searchx_searchsorted_kernel.has_recursion, 0
	.set searchx_searchsorted_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 376
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
	.protected	searchx_isin_kernel     ; -- Begin function searchx_isin_kernel
	.globl	searchx_isin_kernel
	.p2align	8
	.type	searchx_isin_kernel,@function
searchx_isin_kernel:                    ; @searchx_isin_kernel
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
	s_cbranch_execz .LBB1_7
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s5, v4, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_clause 0x1
	s_load_b32 s2, s[0:1], 0x8
	s_load_b64 s[0:1], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s2, 1
	s_cbranch_scc1 .LBB1_8
; %bb.2:
	v_dual_mov_b32 v0, s2 :: v_dual_mov_b32 v7, 0
	v_mov_b32_e32 v5, 0
	s_mov_b32 s3, 0
	.p2align	6
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v6, v0, v5
	v_lshrrev_b32_e32 v6, 1, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v6, v6, v5
	v_lshlrev_b64 v[8:9], 3, v[6:7]
	v_add_nc_u32_e32 v10, 1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, s0, v8
	v_add_co_ci_u32_e64 v9, null, s1, v9, vcc_lo
	global_load_b64 v[8:9], v[8:9], off
	s_waitcnt vmcnt(0)
	v_cmp_lt_f64_e32 vcc_lo, v[8:9], v[3:4]
	v_dual_cndmask_b32 v5, v5, v10 :: v_dual_cndmask_b32 v0, v6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_ge_i32_e32 vcc_lo, v5, v0
	s_or_b32 s3, vcc_lo, s3
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execnz .LBB1_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s3
	v_cmp_gt_i32_e32 vcc_lo, s2, v5
	v_mov_b32_e32 v0, 0
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_6
.LBB1_5:
	v_mov_b32_e32 v6, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	v_add_co_u32 v5, vcc_lo, s0, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s1, v6, vcc_lo
	global_load_b64 v[5:6], v[5:6], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_f64_e32 vcc_lo, v[5:6], v[3:4]
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
.LBB1_6:
	s_or_b32 exec_lo, exec_lo, s2
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, vcc_lo, s6, v1
	v_add_co_ci_u32_e64 v2, null, s7, v2, vcc_lo
	global_store_b32 v[1:2], v0, off
.LBB1_7:
	s_endpgm
.LBB1_8:
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v5
	v_mov_b32_e32 v0, 0
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execnz .LBB1_5
	s_branch .LBB1_6
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_isin_kernel
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
		.amdhsa_next_free_vgpr 11
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
	.size	searchx_isin_kernel, .Lfunc_end1-searchx_isin_kernel
                                        ; -- End function
	.set searchx_isin_kernel.num_vgpr, 11
	.set searchx_isin_kernel.num_agpr, 0
	.set searchx_isin_kernel.numbered_sgpr, 8
	.set searchx_isin_kernel.num_named_barrier, 0
	.set searchx_isin_kernel.private_seg_size, 0
	.set searchx_isin_kernel.uses_vcc, 1
	.set searchx_isin_kernel.uses_flat_scratch, 0
	.set searchx_isin_kernel.has_dyn_sized_stack, 0
	.set searchx_isin_kernel.has_recursion, 0
	.set searchx_isin_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 412
; TotalNumSgprs: 10
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.protected	searchx_nonzero_kernel  ; -- Begin function searchx_nonzero_kernel
	.globl	searchx_nonzero_kernel
	.p2align	8
	.type	searchx_nonzero_kernel,@function
searchx_nonzero_kernel:                 ; @searchx_nonzero_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s2, 0
	s_mov_b32 s3, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB2_7
; %bb.1:
	s_clause 0x1
	s_load_b32 s8, s[0:1], 0x8
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s8, 1
	s_cbranch_scc1 .LBB2_6
; %bb.2:
	s_load_b64 s[0:1], s[0:1], 0x0
	v_mov_b32_e32 v0, 0
	s_mov_b32 s9, 0
	s_branch .LBB2_4
	.p2align	6
.LBB2_3:                                ;   in Loop: Header=BB2_4 Depth=1
	s_add_i32 s9, s9, 1
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_cmp_eq_u32 s8, s9
	s_cbranch_scc1 .LBB2_6
.LBB2_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[10:11], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_eq_f64_e64 s3, s[10:11], 0
	s_and_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccnz .LBB2_3
; %bb.5:                                ;   in Loop: Header=BB2_4 Depth=1
	s_ashr_i32 s3, s2, 31
	v_mov_b32_e32 v1, s9
	s_lshl_b64 s[10:11], s[2:3], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s10, s4, s10
	s_addc_u32 s11, s5, s11
	s_add_i32 s2, s2, 1
	global_store_b32 v0, v1, s[10:11]
	s_branch .LBB2_3
.LBB2_6:
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s2
	global_store_b32 v0, v1, s[6:7]
.LBB2_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_nonzero_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 32
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
		.amdhsa_next_free_vgpr 2
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
	.size	searchx_nonzero_kernel, .Lfunc_end2-searchx_nonzero_kernel
                                        ; -- End function
	.set searchx_nonzero_kernel.num_vgpr, 2
	.set searchx_nonzero_kernel.num_agpr, 0
	.set searchx_nonzero_kernel.numbered_sgpr, 12
	.set searchx_nonzero_kernel.num_named_barrier, 0
	.set searchx_nonzero_kernel.private_seg_size, 0
	.set searchx_nonzero_kernel.uses_vcc, 1
	.set searchx_nonzero_kernel.uses_flat_scratch, 0
	.set searchx_nonzero_kernel.has_dyn_sized_stack, 0
	.set searchx_nonzero_kernel.has_recursion, 0
	.set searchx_nonzero_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 188
; TotalNumSgprs: 14
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 2
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
	.protected	searchx_find_kernel     ; -- Begin function searchx_find_kernel
	.globl	searchx_find_kernel
	.p2align	8
	.type	searchx_find_kernel,@function
searchx_find_kernel:                    ; @searchx_find_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB3_8
; %bb.1:
	s_clause 0x1
	s_load_b32 s2, s[0:1], 0x8
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s2, 1
	s_cbranch_scc1 .LBB3_6
; %bb.2:
	s_load_b64 s[0:1], s[0:1], 0x0
	s_mov_b32 s3, 0
	s_branch .LBB3_4
	.p2align	6
.LBB3_3:                                ;   in Loop: Header=BB3_4 Depth=1
	s_add_i32 s3, s3, 1
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_cmp_eq_u32 s2, s3
	s_mov_b32 s8, s2
	s_cselect_b32 s9, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s9
	s_cbranch_vccz .LBB3_7
.LBB3_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[8:9], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_eq_f64_e64 s8, s[8:9], s[4:5]
	s_and_b32 vcc_lo, exec_lo, s8
	s_cbranch_vccz .LBB3_3
; %bb.5:
	s_mov_b32 s8, s3
                                        ; implicit-def: $sgpr3
                                        ; implicit-def: $sgpr0_sgpr1
	s_branch .LBB3_7
.LBB3_6:
	s_mov_b32 s8, s2
.LBB3_7:
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s8
	global_store_b32 v0, v1, s[6:7]
.LBB3_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_find_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 32
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
		.amdhsa_next_free_vgpr 2
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
.Lfunc_end3:
	.size	searchx_find_kernel, .Lfunc_end3-searchx_find_kernel
                                        ; -- End function
	.set searchx_find_kernel.num_vgpr, 2
	.set searchx_find_kernel.num_agpr, 0
	.set searchx_find_kernel.numbered_sgpr, 10
	.set searchx_find_kernel.num_named_barrier, 0
	.set searchx_find_kernel.private_seg_size, 0
	.set searchx_find_kernel.uses_vcc, 1
	.set searchx_find_kernel.uses_flat_scratch, 0
	.set searchx_find_kernel.has_dyn_sized_stack, 0
	.set searchx_find_kernel.has_recursion, 0
	.set searchx_find_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 172
; TotalNumSgprs: 12
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 2
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
	.protected	searchx_find_n_kernel   ; -- Begin function searchx_find_n_kernel
	.globl	searchx_find_n_kernel
	.p2align	8
	.type	searchx_find_n_kernel,@function
searchx_find_n_kernel:                  ; @searchx_find_n_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB4_11
; %bb.1:
	s_load_b32 s6, s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB4_10
; %bb.2:
	s_clause 0x2
	s_load_b64 s[2:3], s[0:1], 0x0
	s_load_b64 s[4:5], s[0:1], 0x10
	s_load_b32 s7, s[0:1], 0x18
	s_mov_b32 s11, 0
	s_mov_b32 s12, 0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB4_4
	.p2align	6
.LBB4_3:                                ;   in Loop: Header=BB4_4 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s12
	s_mov_b32 s12, s9
	s_cbranch_vccz .LBB4_8
.LBB4_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[8:9], s[2:3], 0x0
	s_mov_b32 s10, -1
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f64_e64 s9, s[8:9], s[4:5]
	s_mov_b32 s8, s11
	s_mov_b32 s11, -1
	s_and_b32 vcc_lo, exec_lo, s9
	s_mov_b32 s9, 0
	s_cbranch_vccnz .LBB4_6
; %bb.5:                                ;   in Loop: Header=BB4_4 Depth=1
	s_add_i32 s9, s12, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lt_i32 s9, s7
	s_cselect_b32 s11, -1, 0
.LBB4_6:                                ;   in Loop: Header=BB4_4 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s11
	s_mov_b32 s12, -1
                                        ; implicit-def: $sgpr11
	s_cbranch_vccnz .LBB4_3
; %bb.7:                                ;   in Loop: Header=BB4_4 Depth=1
	s_add_i32 s11, s8, 1
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_eq_u32 s6, s11
	s_mov_b32 s10, 0
	s_cselect_b32 s12, -1, 0
	s_branch .LBB4_3
.LBB4_8:
	s_set_inst_prefetch_distance 0x2
	s_and_b32 vcc_lo, exec_lo, s10
	s_cbranch_vccz .LBB4_10
; %bb.9:
	s_sub_i32 s2, s8, s7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s6, s2, 1
.LBB4_10:
	s_load_b64 s[0:1], s[0:1], 0x20
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s6
	s_waitcnt lgkmcnt(0)
	global_store_b32 v0, v1, s[0:1]
.LBB4_11:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_find_n_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 40
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
		.amdhsa_next_free_vgpr 2
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
	.size	searchx_find_n_kernel, .Lfunc_end4-searchx_find_n_kernel
                                        ; -- End function
	.set searchx_find_n_kernel.num_vgpr, 2
	.set searchx_find_n_kernel.num_agpr, 0
	.set searchx_find_n_kernel.numbered_sgpr, 13
	.set searchx_find_n_kernel.num_named_barrier, 0
	.set searchx_find_n_kernel.private_seg_size, 0
	.set searchx_find_n_kernel.uses_vcc, 1
	.set searchx_find_n_kernel.uses_flat_scratch, 0
	.set searchx_find_n_kernel.has_dyn_sized_stack, 0
	.set searchx_find_n_kernel.has_recursion, 0
	.set searchx_find_n_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 260
; TotalNumSgprs: 15
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 15
; NumVGPRsForWavesPerEU: 2
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
	.protected	searchx_is_sorted_kernel ; -- Begin function searchx_is_sorted_kernel
	.globl	searchx_is_sorted_kernel
	.p2align	8
	.type	searchx_is_sorted_kernel,@function
searchx_is_sorted_kernel:               ; @searchx_is_sorted_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB5_7
; %bb.1:
	s_load_b32 s4, s[0:1], 0x8
	s_mov_b32 s5, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s4, 2
	s_cbranch_scc1 .LBB5_6
; %bb.2:
	s_load_b64 s[2:3], s[0:1], 0x0
	s_add_i32 s4, s4, -1
	s_branch .LBB5_4
	.p2align	6
.LBB5_3:                                ;   in Loop: Header=BB5_4 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccz .LBB5_6
.LBB5_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	s_load_b128 s[8:11], s[2:3], 0x0
	s_mov_b32 s6, -1
	s_waitcnt lgkmcnt(0)
	v_cmp_nlt_f64_e64 s5, s[10:11], s[8:9]
	s_and_b32 vcc_lo, exec_lo, s5
	s_mov_b32 s5, 0
	s_cbranch_vccz .LBB5_3
; %bb.5:                                ;   in Loop: Header=BB5_4 Depth=1
	s_add_i32 s4, s4, -1
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_eq_u32 s4, 0
	s_mov_b32 s5, 1
	s_cselect_b32 s6, -1, 0
	s_branch .LBB5_3
.LBB5_6:
	s_load_b64 s[0:1], s[0:1], 0x10
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s5
	s_waitcnt lgkmcnt(0)
	global_store_b32 v0, v1, s[0:1]
.LBB5_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_is_sorted_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 24
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
		.amdhsa_next_free_vgpr 2
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
.Lfunc_end5:
	.size	searchx_is_sorted_kernel, .Lfunc_end5-searchx_is_sorted_kernel
                                        ; -- End function
	.set searchx_is_sorted_kernel.num_vgpr, 2
	.set searchx_is_sorted_kernel.num_agpr, 0
	.set searchx_is_sorted_kernel.numbered_sgpr, 12
	.set searchx_is_sorted_kernel.num_named_barrier, 0
	.set searchx_is_sorted_kernel.private_seg_size, 0
	.set searchx_is_sorted_kernel.uses_vcc, 1
	.set searchx_is_sorted_kernel.uses_flat_scratch, 0
	.set searchx_is_sorted_kernel.has_dyn_sized_stack, 0
	.set searchx_is_sorted_kernel.has_recursion, 0
	.set searchx_is_sorted_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 168
; TotalNumSgprs: 14
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 1
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 2
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
	.protected	searchx_is_sorted_until_kernel ; -- Begin function searchx_is_sorted_until_kernel
	.globl	searchx_is_sorted_until_kernel
	.p2align	8
	.type	searchx_is_sorted_until_kernel,@function
searchx_is_sorted_until_kernel:         ; @searchx_is_sorted_until_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB6_8
; %bb.1:
	s_load_b32 s4, s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s4, 2
	s_cbranch_scc1 .LBB6_6
; %bb.2:
	s_load_b64 s[2:3], s[0:1], 0x0
	s_mov_b32 s5, 1
	s_branch .LBB6_4
	.p2align	6
.LBB6_3:                                ;   in Loop: Header=BB6_4 Depth=1
	s_add_i32 s5, s5, 1
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_eq_u32 s4, s5
	s_mov_b32 s6, s4
	s_cselect_b32 s7, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s7
	s_cbranch_vccz .LBB6_7
.LBB6_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	s_load_b128 s[8:11], s[2:3], 0x0
	s_mov_b32 s7, -1
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f64_e64 s6, s[10:11], s[8:9]
	s_and_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccz .LBB6_3
; %bb.5:                                ;   in Loop: Header=BB6_4 Depth=1
	s_mov_b32 s6, s5
                                        ; implicit-def: $sgpr5
                                        ; implicit-def: $sgpr2_sgpr3
	s_and_not1_b32 vcc_lo, exec_lo, s7
	s_cbranch_vccnz .LBB6_4
	s_branch .LBB6_7
.LBB6_6:
	s_mov_b32 s6, s4
.LBB6_7:
	s_load_b64 s[0:1], s[0:1], 0x10
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s6
	s_waitcnt lgkmcnt(0)
	global_store_b32 v0, v1, s[0:1]
.LBB6_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_is_sorted_until_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 24
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
		.amdhsa_next_free_vgpr 2
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
.Lfunc_end6:
	.size	searchx_is_sorted_until_kernel, .Lfunc_end6-searchx_is_sorted_until_kernel
                                        ; -- End function
	.set searchx_is_sorted_until_kernel.num_vgpr, 2
	.set searchx_is_sorted_until_kernel.num_agpr, 0
	.set searchx_is_sorted_until_kernel.numbered_sgpr, 12
	.set searchx_is_sorted_until_kernel.num_named_barrier, 0
	.set searchx_is_sorted_until_kernel.private_seg_size, 0
	.set searchx_is_sorted_until_kernel.uses_vcc, 1
	.set searchx_is_sorted_until_kernel.uses_flat_scratch, 0
	.set searchx_is_sorted_until_kernel.has_dyn_sized_stack, 0
	.set searchx_is_sorted_until_kernel.has_recursion, 0
	.set searchx_is_sorted_until_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 180
; TotalNumSgprs: 14
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 1
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 2
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
	.protected	searchx_partition_point_kernel ; -- Begin function searchx_partition_point_kernel
	.globl	searchx_partition_point_kernel
	.p2align	8
	.type	searchx_partition_point_kernel,@function
searchx_partition_point_kernel:         ; @searchx_partition_point_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s8, 0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB7_5
; %bb.1:
	s_clause 0x1
	s_load_b32 s9, s[0:1], 0x8
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s9, 1
	s_cbranch_scc1 .LBB7_4
; %bb.2:
	s_load_b64 s[0:1], s[0:1], 0x0
	s_mov_b32 s3, 0
.LBB7_3:                                ; =>This Inner Loop Header: Depth=1
	s_sub_i32 s2, s9, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshr_b32 s2, s2, 1
	s_add_i32 s2, s2, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b64 s[10:11], s[2:3], 3
	s_waitcnt lgkmcnt(0)
	s_add_u32 s10, s0, s10
	s_addc_u32 s11, s1, s11
	s_load_b64 s[10:11], s[10:11], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f64_e64 s10, s[10:11], s[4:5]
	s_add_i32 s11, s2, 1
	s_and_b32 s10, s10, exec_lo
	s_cselect_b32 s9, s9, s2
	s_cselect_b32 s8, s11, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lt_i32 s8, s9
	s_cbranch_scc1 .LBB7_3
.LBB7_4:
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s8
	global_store_b32 v0, v1, s[6:7]
.LBB7_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_partition_point_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 32
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
		.amdhsa_next_free_vgpr 2
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
.Lfunc_end7:
	.size	searchx_partition_point_kernel, .Lfunc_end7-searchx_partition_point_kernel
                                        ; -- End function
	.set searchx_partition_point_kernel.num_vgpr, 2
	.set searchx_partition_point_kernel.num_agpr, 0
	.set searchx_partition_point_kernel.numbered_sgpr, 12
	.set searchx_partition_point_kernel.num_named_barrier, 0
	.set searchx_partition_point_kernel.private_seg_size, 0
	.set searchx_partition_point_kernel.uses_vcc, 0
	.set searchx_partition_point_kernel.uses_flat_scratch, 0
	.set searchx_partition_point_kernel.has_dyn_sized_stack, 0
	.set searchx_partition_point_kernel.has_recursion, 0
	.set searchx_partition_point_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 172
; TotalNumSgprs: 12
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 2
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
	.protected	searchx_mismatch_kernel ; -- Begin function searchx_mismatch_kernel
	.globl	searchx_mismatch_kernel
	.p2align	8
	.type	searchx_mismatch_kernel,@function
searchx_mismatch_kernel:                ; @searchx_mismatch_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB8_8
; %bb.1:
	s_load_b32 s2, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s2, 1
	s_cbranch_scc1 .LBB8_6
; %bb.2:
	s_load_b128 s[4:7], s[0:1], 0x0
	s_mov_b32 s3, 0
	s_branch .LBB8_4
	.p2align	6
.LBB8_3:                                ;   in Loop: Header=BB8_4 Depth=1
	s_add_i32 s3, s3, 1
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
	s_add_u32 s6, s6, 8
	s_addc_u32 s7, s7, 0
	s_cmp_eq_u32 s2, s3
	s_mov_b32 s8, s2
	s_cselect_b32 s9, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s9
	s_cbranch_vccz .LBB8_7
.LBB8_4:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[8:9], s[4:5], 0x0
	s_load_b64 s[10:11], s[6:7], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f64_e64 s8, s[8:9], s[10:11]
	s_and_b32 vcc_lo, exec_lo, s8
	s_cbranch_vccz .LBB8_3
; %bb.5:
	s_mov_b32 s8, s3
                                        ; implicit-def: $sgpr3
                                        ; implicit-def: $sgpr4_sgpr5
	s_branch .LBB8_7
.LBB8_6:
	s_mov_b32 s8, s2
.LBB8_7:
	s_load_b64 s[0:1], s[0:1], 0x18
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s8
	s_waitcnt lgkmcnt(0)
	global_store_b32 v0, v1, s[0:1]
.LBB8_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_mismatch_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 32
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
		.amdhsa_next_free_vgpr 2
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
.Lfunc_end8:
	.size	searchx_mismatch_kernel, .Lfunc_end8-searchx_mismatch_kernel
                                        ; -- End function
	.set searchx_mismatch_kernel.num_vgpr, 2
	.set searchx_mismatch_kernel.num_agpr, 0
	.set searchx_mismatch_kernel.numbered_sgpr, 12
	.set searchx_mismatch_kernel.num_named_barrier, 0
	.set searchx_mismatch_kernel.private_seg_size, 0
	.set searchx_mismatch_kernel.uses_vcc, 1
	.set searchx_mismatch_kernel.uses_flat_scratch, 0
	.set searchx_mismatch_kernel.has_dyn_sized_stack, 0
	.set searchx_mismatch_kernel.has_recursion, 0
	.set searchx_mismatch_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 184
; TotalNumSgprs: 14
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 1
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 2
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
	.protected	searchx_argextreme_kernel ; -- Begin function searchx_argextreme_kernel
	.globl	searchx_argextreme_kernel
	.p2align	8
	.type	searchx_argextreme_kernel,@function
searchx_argextreme_kernel:              ; @searchx_argextreme_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s8, 0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB9_5
; %bb.1:
	s_load_b64 s[2:3], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s2, 2
	s_cbranch_scc1 .LBB9_4
; %bb.2:
	s_load_b64 s[6:7], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[4:5], s[6:7], 0x0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s8, s5, 0x7fffffff
	s_cmp_gt_i32 s3, 1
	s_cselect_b32 s9, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 s10, s9, exec_lo
	s_cselect_b32 s5, s8, s5
	s_cselect_b32 s4, s4, s4
	s_and_b32 s8, s3, -3
	s_mov_b32 s3, 1
	s_cmp_eq_u32 s8, 0
	s_mov_b32 s8, 0
	s_cselect_b32 s10, -1, 0
	s_add_u32 s6, s6, 8
	s_addc_u32 s7, s7, 0
	.p2align	6
.LBB9_3:                                ; =>This Inner Loop Header: Depth=1
	s_load_b64 s[12:13], s[6:7], 0x0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s11, s13, 0x7fffffff
	s_and_b32 s14, s9, exec_lo
	s_cselect_b32 s13, s11, s13
	s_cselect_b32 s12, s12, s12
	s_and_b32 s15, s10, exec_lo
	v_cmp_gt_f64_e64 s11, s[12:13], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v0, 0, 1, s11
	v_cmp_lt_f64_e64 s11, s[12:13], s[4:5]
	v_cndmask_b32_e64 v1, 0, 1, s11
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s11, v0
	v_readfirstlane_b32 s14, v1
	s_cselect_b32 s11, s11, s14
	s_delay_alu instid0(SALU_CYCLE_1)
	s_bitcmp1_b32 s11, 0
	s_cselect_b32 s5, s13, s5
	s_cselect_b32 s4, s12, s4
	s_cselect_b32 s8, s3, s8
	s_add_i32 s3, s3, 1
	s_add_u32 s6, s6, 8
	s_addc_u32 s7, s7, 0
	s_cmp_eq_u32 s2, s3
	s_cbranch_scc0 .LBB9_3
.LBB9_4:
	s_load_b64 s[0:1], s[0:1], 0x10
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s8
	s_waitcnt lgkmcnt(0)
	global_store_b32 v0, v1, s[0:1]
.LBB9_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_argextreme_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 24
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
		.amdhsa_next_free_vgpr 2
		.amdhsa_next_free_sgpr 16
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
.Lfunc_end9:
	.size	searchx_argextreme_kernel, .Lfunc_end9-searchx_argextreme_kernel
                                        ; -- End function
	.set searchx_argextreme_kernel.num_vgpr, 2
	.set searchx_argextreme_kernel.num_agpr, 0
	.set searchx_argextreme_kernel.numbered_sgpr, 16
	.set searchx_argextreme_kernel.num_named_barrier, 0
	.set searchx_argextreme_kernel.private_seg_size, 0
	.set searchx_argextreme_kernel.uses_vcc, 0
	.set searchx_argextreme_kernel.uses_flat_scratch, 0
	.set searchx_argextreme_kernel.has_dyn_sized_stack, 0
	.set searchx_argextreme_kernel.has_recursion, 0
	.set searchx_argextreme_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 288
; TotalNumSgprs: 16
; NumVgprs: 2
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 16
; NumVGPRsForWavesPerEU: 2
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
	.protected	searchx_minmax_element_kernel ; -- Begin function searchx_minmax_element_kernel
	.globl	searchx_minmax_element_kernel
	.p2align	8
	.type	searchx_minmax_element_kernel,@function
searchx_minmax_element_kernel:          ; @searchx_minmax_element_kernel
; %bb.0:
	v_or_b32_e32 v0, s2, v0
	s_mov_b32 s8, 0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB10_5
; %bb.1:
	s_load_b32 s9, s[0:1], 0x8
	s_mov_b32 s10, 0
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s9, 2
	s_cbranch_scc1 .LBB10_4
; %bb.2:
	s_load_b64 s[4:5], s[0:1], 0x0
	s_mov_b32 s11, 1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[4:5], 0x0
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
	s_waitcnt lgkmcnt(0)
	s_mov_b64 s[6:7], s[2:3]
	.p2align	6
.LBB10_3:                               ; =>This Inner Loop Header: Depth=1
	s_load_b64 s[12:13], s[4:5], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f64_e64 s14, s[12:13], s[6:7]
	v_cmp_gt_f64_e64 s15, s[12:13], s[2:3]
	s_and_b32 s14, s14, exec_lo
	s_cselect_b32 s7, s13, s7
	s_cselect_b32 s6, s12, s6
	s_cselect_b32 s8, s11, s8
	s_and_b32 s14, s15, exec_lo
	s_cselect_b32 s10, s11, s10
	s_cselect_b32 s3, s13, s3
	s_cselect_b32 s2, s12, s2
	s_add_i32 s11, s11, 1
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
	s_cmp_eq_u32 s9, s11
	s_cbranch_scc0 .LBB10_3
.LBB10_4:
	s_load_b64 s[0:1], s[0:1], 0x10
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v1, s10
	v_mov_b32_e32 v0, s8
	s_waitcnt lgkmcnt(0)
	global_store_b64 v2, v[0:1], s[0:1]
.LBB10_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_minmax_element_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 24
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
		.amdhsa_next_free_sgpr 16
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
.Lfunc_end10:
	.size	searchx_minmax_element_kernel, .Lfunc_end10-searchx_minmax_element_kernel
                                        ; -- End function
	.set searchx_minmax_element_kernel.num_vgpr, 3
	.set searchx_minmax_element_kernel.num_agpr, 0
	.set searchx_minmax_element_kernel.numbered_sgpr, 16
	.set searchx_minmax_element_kernel.num_named_barrier, 0
	.set searchx_minmax_element_kernel.private_seg_size, 0
	.set searchx_minmax_element_kernel.uses_vcc, 0
	.set searchx_minmax_element_kernel.uses_flat_scratch, 0
	.set searchx_minmax_element_kernel.has_dyn_sized_stack, 0
	.set searchx_minmax_element_kernel.has_recursion, 0
	.set searchx_minmax_element_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 204
; TotalNumSgprs: 16
; NumVgprs: 3
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 16
; NumVGPRsForWavesPerEU: 3
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
	.protected	searchx_argrel_kernel   ; -- Begin function searchx_argrel_kernel
	.globl	searchx_argrel_kernel
	.p2align	8
	.type	searchx_argrel_kernel,@function
searchx_argrel_kernel:                  ; @searchx_argrel_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB11_13
; %bb.1:
	s_add_i32 s2, s4, -1
	v_cmp_lt_i32_e32 vcc_lo, 0, v1
	v_cmp_gt_i32_e64 s2, s2, v1
	v_mov_b32_e32 v0, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s6, s2
	s_cbranch_execz .LBB11_12
; %bb.2:
	s_load_b64 s[2:3], s[0:1], 0x0
	v_mov_b32_e32 v2, 0
	s_cmp_lt_i32 s5, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v6, vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s3, v3, vcc_lo
	s_clause 0x1
	global_load_b128 v[2:5], v[6:7], off offset:-8
	global_load_b64 v[6:7], v[6:7], off offset:8
	s_waitcnt vmcnt(1)
	v_cmp_lt_f64_e64 s3, v[4:5], v[2:3]
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e64 s2, v[4:5], v[6:7]
	v_cmp_lt_f64_e64 s4, v[4:5], v[6:7]
	v_cmp_gt_f64_e32 vcc_lo, v[4:5], v[2:3]
	s_cbranch_scc1 .LBB11_6
; %bb.3:
	s_cmp_eq_u32 s5, 1
	s_mov_b32 s7, -1
	s_cbranch_scc0 .LBB11_5
; %bb.4:
	s_mov_b32 s7, 0
.LBB11_5:
	s_mov_b32 s8, 0
	s_branch .LBB11_7
.LBB11_6:
	s_mov_b32 s8, -1
	s_mov_b32 s7, 0
.LBB11_7:
	s_and_b32 s3, s3, s4
	s_and_b32 s2, vcc_lo, s2
	s_and_b32 vcc_lo, exec_lo, s8
	s_mov_b32 s4, s3
	s_cbranch_vccz .LBB11_9
; %bb.8:
	s_cmp_lg_u32 s5, 0
	s_cselect_b32 s7, -1, 0
	s_and_not1_b32 s4, s3, exec_lo
	s_and_b32 s5, s2, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 s4, s4, s5
.LBB11_9:
	s_and_not1_b32 vcc_lo, exec_lo, s7
	s_cbranch_vccnz .LBB11_11
; %bb.10:
	s_or_b32 s2, s2, s3
	s_and_not1_b32 s3, s4, exec_lo
	s_and_b32 s2, s2, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 s4, s3, s2
.LBB11_11:
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e64 v0, 0, 1, s4
.LBB11_12:
	s_or_b32 exec_lo, exec_lo, s6
	s_load_b64 s[0:1], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v1, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s1, v2, vcc_lo
	global_store_b32 v[1:2], v0, off
.LBB11_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel searchx_argrel_kernel
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
		.amdhsa_next_free_vgpr 8
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
.Lfunc_end11:
	.size	searchx_argrel_kernel, .Lfunc_end11-searchx_argrel_kernel
                                        ; -- End function
	.set searchx_argrel_kernel.num_vgpr, 8
	.set searchx_argrel_kernel.num_agpr, 0
	.set searchx_argrel_kernel.numbered_sgpr, 9
	.set searchx_argrel_kernel.num_named_barrier, 0
	.set searchx_argrel_kernel.private_seg_size, 0
	.set searchx_argrel_kernel.uses_vcc, 1
	.set searchx_argrel_kernel.uses_flat_scratch, 0
	.set searchx_argrel_kernel.has_dyn_sized_stack, 0
	.set searchx_argrel_kernel.has_recursion, 0
	.set searchx_argrel_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 388
; TotalNumSgprs: 11
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 11
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_55d4661872b462c1,@object ; @__hip_cuid_55d4661872b462c1
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_55d4661872b462c1
__hip_cuid_55d4661872b462c1:
	.byte	0                               ; 0x0
	.size	__hip_cuid_55d4661872b462c1, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_55d4661872b462c1
	.amdgpu_metadata
---
amdhsa.kernels:
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .offset:         8
        .size:           4
        .value_kind:     by_value
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
    .name:           searchx_searchsorted_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         searchx_searchsorted_kernel.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
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
    .name:           searchx_isin_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         searchx_isin_kernel.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 32
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_nonzero_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         searchx_nonzero_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
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
      - .offset:         16
        .size:           8
        .value_kind:     by_value
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 32
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_find_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         searchx_find_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
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
      - .offset:         16
        .size:           8
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 40
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_find_n_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     15
    .sgpr_spill_count: 0
    .symbol:         searchx_find_n_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
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
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 24
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_is_sorted_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         searchx_is_sorted_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
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
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 24
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_is_sorted_until_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         searchx_is_sorted_until_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
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
      - .offset:         16
        .size:           8
        .value_kind:     by_value
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 32
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_partition_point_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         searchx_partition_point_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
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
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 32
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_mismatch_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         searchx_mismatch_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 24
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_argextreme_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         searchx_argextreme_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     2
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
      - .address_space:  global
        .offset:         16
        .size:           8
        .value_kind:     global_buffer
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 24
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           searchx_minmax_element_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         searchx_minmax_element_kernel.kd
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
    .name:           searchx_argrel_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     11
    .sgpr_spill_count: 0
    .symbol:         searchx_argrel_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     8
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
