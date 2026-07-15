	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	embeddingx_lookup_kernel ; -- Begin function embeddingx_lookup_kernel
	.globl	embeddingx_lookup_kernel
	.p2align	8
	.type	embeddingx_lookup_kernel,@function
embeddingx_lookup_kernel:               ; @embeddingx_lookup_kernel
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
	s_cbranch_execz .LBB0_2
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[3:4], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s10, v3
	v_add_co_ci_u32_e64 v4, null, s11, v4, vcc_lo
	global_load_b32 v0, v[3:4], off
	v_mul_lo_u32 v4, v2, s5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v4, v1, v4
	v_ashrrev_i32_e32 v5, 31, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_waitcnt vmcnt(0)
	v_mad_i64_i32 v[2:3], null, v0, s5, 0
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s8, v2
	v_add_co_ci_u32_e64 v3, null, s9, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, v0, v4
	v_add_co_ci_u32_e64 v3, null, v3, v5, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel embeddingx_lookup_kernel
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
.Lfunc_end0:
	.size	embeddingx_lookup_kernel, .Lfunc_end0-embeddingx_lookup_kernel
                                        ; -- End function
	.set embeddingx_lookup_kernel.num_vgpr, 6
	.set embeddingx_lookup_kernel.num_agpr, 0
	.set embeddingx_lookup_kernel.numbered_sgpr, 12
	.set embeddingx_lookup_kernel.num_named_barrier, 0
	.set embeddingx_lookup_kernel.private_seg_size, 0
	.set embeddingx_lookup_kernel.uses_vcc, 1
	.set embeddingx_lookup_kernel.uses_flat_scratch, 0
	.set embeddingx_lookup_kernel.has_dyn_sized_stack, 0
	.set embeddingx_lookup_kernel.has_recursion, 0
	.set embeddingx_lookup_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 440
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
	.protected	embeddingx_bag_kernel   ; -- Begin function embeddingx_bag_kernel
	.globl	embeddingx_bag_kernel
	.p2align	8
	.type	embeddingx_bag_kernel,@function
embeddingx_bag_kernel:                  ; @embeddingx_bag_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[8:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s9, s8
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_8
; %bb.1:
	s_abs_i32 s2, s9
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
	v_xor_b32_e32 v3, s9, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_load_b256 s[0:7], s[0:1], 0x0
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v2, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[3:4], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s4, v3
	v_add_co_ci_u32_e64 v4, null, s5, v4, vcc_lo
	s_mov_b32 s5, 0
	s_mov_b32 s4, exec_lo
	global_load_b64 v[5:6], v[3:4], off
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_waitcnt vmcnt(0)
	v_cmpx_gt_i32_e64 v6, v5
	s_cbranch_execz .LBB1_5
; %bb.2:
	v_mul_lo_u32 v0, v2, s9
	v_ashrrev_i32_e32 v3, 31, v5
	v_mov_b32_e32 v2, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[9:10], 2, v[2:3]
	v_sub_nc_u32_e32 v7, v1, v0
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	v_lshlrev_b64 v[11:12], 3, v[7:8]
	v_add_co_u32 v7, vcc_lo, s2, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v8, null, s3, v10, vcc_lo
	v_mov_b32_e32 v9, v5
	v_add_co_u32 v0, vcc_lo, s0, v11
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s1, v12, vcc_lo
	s_mov_b32 s1, s9
	.p2align	6
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b32 v12, v[7:8], off
	v_add_nc_u32_e32 v9, 1, v9
	v_add_co_u32 v7, s0, v7, 4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, 0, v8, s0
	s_waitcnt vmcnt(0)
	v_mad_i64_i32 v[10:11], null, v12, s1, 0
	v_lshlrev_b64 v[10:11], 3, v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v10, vcc_lo, v0, v10
	v_add_co_ci_u32_e64 v11, null, v2, v11, vcc_lo
	v_cmp_ge_i32_e32 vcc_lo, v9, v6
	global_load_b64 v[10:11], v[10:11], off
	s_or_b32 s5, vcc_lo, s5
	s_waitcnt vmcnt(0)
	v_add_f64 v[3:4], v[3:4], v[10:11]
	s_and_not1_b32 exec_lo, exec_lo, s5
	s_cbranch_execnz .LBB1_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s5
.LBB1_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s4
	v_sub_nc_u32_e32 v0, v6, v5
	s_cmp_eq_u32 s10, 1
	s_cselect_b32 s0, -1, 0
	v_cmp_lt_i32_e32 vcc_lo, 0, v0
	s_and_b32 s1, s0, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s0, s1
	s_cbranch_execz .LBB1_7
; %bb.6:
	v_cvt_f64_u32_e32 v[5:6], v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[7:8], null, v[5:6], v[5:6], v[3:4]
	v_rcp_f64_e32 v[9:10], v[7:8]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_div_scale_f64 v[11:12], vcc_lo, v[3:4], v[5:6], v[3:4]
	v_mul_f64 v[13:14], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[7:8], v[13:14], v[11:12]
	v_div_fmas_f64 v[7:8], v[7:8], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[7:8], v[5:6], v[3:4]
.LBB1_7:
	s_or_b32 exec_lo, exec_lo, s0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB1_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel embeddingx_bag_kernel
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
	.size	embeddingx_bag_kernel, .Lfunc_end1-embeddingx_bag_kernel
                                        ; -- End function
	.set embeddingx_bag_kernel.num_vgpr, 15
	.set embeddingx_bag_kernel.num_agpr, 0
	.set embeddingx_bag_kernel.numbered_sgpr, 12
	.set embeddingx_bag_kernel.num_named_barrier, 0
	.set embeddingx_bag_kernel.private_seg_size, 0
	.set embeddingx_bag_kernel.uses_vcc, 1
	.set embeddingx_bag_kernel.uses_flat_scratch, 0
	.set embeddingx_bag_kernel.has_dyn_sized_stack, 0
	.set embeddingx_bag_kernel.has_recursion, 0
	.set embeddingx_bag_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 732
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
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	embeddingx_one_hot_kernel ; -- Begin function embeddingx_one_hot_kernel
	.globl	embeddingx_one_hot_kernel
	.p2align	8
	.type	embeddingx_one_hot_kernel,@function
embeddingx_one_hot_kernel:              ; @embeddingx_one_hot_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB2_2
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_load_b128 s[0:3], s[0:1], 0x0
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v2, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v3, 31, v2
	v_mul_lo_u32 v0, v2, s5
	v_lshlrev_b64 v[3:4], 2, v[2:3]
	v_ashrrev_i32_e32 v2, 31, v1
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s0, v3
	v_add_co_ci_u32_e64 v4, null, s1, v4, vcc_lo
	global_load_b32 v3, v[3:4], off
	v_sub_nc_u32_e32 v4, v1, v0
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_mov_b32_e32 v2, 0
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cmp_eq_u32_e32 vcc_lo, v3, v4
	v_cndmask_b32_e64 v3, 0, 0x3ff00000, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel embeddingx_one_hot_kernel
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
	.size	embeddingx_one_hot_kernel, .Lfunc_end2-embeddingx_one_hot_kernel
                                        ; -- End function
	.set embeddingx_one_hot_kernel.num_vgpr, 5
	.set embeddingx_one_hot_kernel.num_agpr, 0
	.set embeddingx_one_hot_kernel.numbered_sgpr, 6
	.set embeddingx_one_hot_kernel.num_named_barrier, 0
	.set embeddingx_one_hot_kernel.private_seg_size, 0
	.set embeddingx_one_hot_kernel.uses_vcc, 1
	.set embeddingx_one_hot_kernel.uses_flat_scratch, 0
	.set embeddingx_one_hot_kernel.has_dyn_sized_stack, 0
	.set embeddingx_one_hot_kernel.has_recursion, 0
	.set embeddingx_one_hot_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 364
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
	.protected	embeddingx_rope_kernel  ; -- Begin function embeddingx_rope_kernel
	.globl	embeddingx_rope_kernel
	.p2align	8
	.type	embeddingx_rope_kernel,@function
embeddingx_rope_kernel:                 ; @embeddingx_rope_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b64 s[10:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_lshr_b32 s2, s11, 31
	s_add_i32 s2, s11, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s8, s2, 1
	s_mul_i32 s2, s8, s10
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_abs_i32 s2, s8
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_ashr_i32 s9, s8, 31
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
	v_xor_b32_e32 v3, s8, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_load_b256 s[0:7], s[0:1], 0x0
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v6, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v6, s8
	v_sub_nc_u32_e32 v0, v1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v1, 31, v0
	v_mad_i64_i32 v[2:3], null, v6, s11, v[0:1]
	v_mad_i64_i32 v[4:5], null, v6, s8, v[0:1]
	s_lshl_b64 s[8:9], s[8:9], 3
	v_lshlrev_b64 v[0:1], 3, v[2:3]
	v_lshlrev_b64 v[2:3], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v5, null, s1, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s4, v2
	v_add_co_ci_u32_e64 v7, null, s5, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, v4, s8
	v_add_co_ci_u32_e64 v9, null, s9, v5, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_clause 0x1
	global_load_b64 v[8:9], v[8:9], off
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_u32 v2, vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(2)
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], v[4:5], v[6:7]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[4:5], v[2:3], -v[10:11]
	v_fma_f64 v[2:3], v[8:9], v[2:3], v[6:7]
	v_add_co_u32 v6, vcc_lo, v0, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s9, v1, vcc_lo
	s_clause 0x1
	global_store_b64 v[0:1], v[4:5], off
	global_store_b64 v[6:7], v[2:3], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel embeddingx_rope_kernel
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
.Lfunc_end3:
	.size	embeddingx_rope_kernel, .Lfunc_end3-embeddingx_rope_kernel
                                        ; -- End function
	.set embeddingx_rope_kernel.num_vgpr, 12
	.set embeddingx_rope_kernel.num_agpr, 0
	.set embeddingx_rope_kernel.numbered_sgpr, 12
	.set embeddingx_rope_kernel.num_named_barrier, 0
	.set embeddingx_rope_kernel.private_seg_size, 0
	.set embeddingx_rope_kernel.uses_vcc, 1
	.set embeddingx_rope_kernel.uses_flat_scratch, 0
	.set embeddingx_rope_kernel.has_dyn_sized_stack, 0
	.set embeddingx_rope_kernel.has_recursion, 0
	.set embeddingx_rope_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 544
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
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	embeddingx_rope_bwd_kernel ; -- Begin function embeddingx_rope_bwd_kernel
	.globl	embeddingx_rope_bwd_kernel
	.p2align	8
	.type	embeddingx_rope_bwd_kernel,@function
embeddingx_rope_bwd_kernel:             ; @embeddingx_rope_bwd_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b64 s[10:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_lshr_b32 s2, s11, 31
	s_add_i32 s2, s11, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s8, s2, 1
	s_mul_i32 s2, s8, s10
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB4_2
; %bb.1:
	s_abs_i32 s2, s8
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_ashr_i32 s9, s8, 31
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
	v_xor_b32_e32 v3, s8, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_load_b256 s[0:7], s[0:1], 0x0
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v6, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v6, s8
	v_sub_nc_u32_e32 v0, v1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v1, 31, v0
	v_mad_i64_i32 v[2:3], null, v6, s11, v[0:1]
	v_mad_i64_i32 v[4:5], null, v6, s8, v[0:1]
	s_lshl_b64 s[8:9], s[8:9], 3
	v_lshlrev_b64 v[0:1], 3, v[2:3]
	v_lshlrev_b64 v[2:3], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v5, null, s1, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s4, v2
	v_add_co_ci_u32_e64 v7, null, s5, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, v4, s8
	v_add_co_ci_u32_e64 v9, null, s9, v5, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_clause 0x1
	global_load_b64 v[8:9], v[8:9], off
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_u32 v2, vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(2)
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], v[4:5], v[6:7]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[4:5], v[2:3], v[10:11]
	v_fma_f64 v[2:3], v[8:9], v[2:3], -v[6:7]
	v_add_co_u32 v6, vcc_lo, v0, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s9, v1, vcc_lo
	s_clause 0x1
	global_store_b64 v[0:1], v[4:5], off
	global_store_b64 v[6:7], v[2:3], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel embeddingx_rope_bwd_kernel
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
.Lfunc_end4:
	.size	embeddingx_rope_bwd_kernel, .Lfunc_end4-embeddingx_rope_bwd_kernel
                                        ; -- End function
	.set embeddingx_rope_bwd_kernel.num_vgpr, 12
	.set embeddingx_rope_bwd_kernel.num_agpr, 0
	.set embeddingx_rope_bwd_kernel.numbered_sgpr, 12
	.set embeddingx_rope_bwd_kernel.num_named_barrier, 0
	.set embeddingx_rope_bwd_kernel.private_seg_size, 0
	.set embeddingx_rope_bwd_kernel.uses_vcc, 1
	.set embeddingx_rope_bwd_kernel.uses_flat_scratch, 0
	.set embeddingx_rope_bwd_kernel.has_dyn_sized_stack, 0
	.set embeddingx_rope_bwd_kernel.has_recursion, 0
	.set embeddingx_rope_bwd_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 544
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
	.type	__hip_cuid_1fafd5fe2e03713a,@object ; @__hip_cuid_1fafd5fe2e03713a
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_1fafd5fe2e03713a
__hip_cuid_1fafd5fe2e03713a:
	.byte	0                               ; 0x0
	.size	__hip_cuid_1fafd5fe2e03713a, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_1fafd5fe2e03713a
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
    .name:           embeddingx_lookup_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         embeddingx_lookup_kernel.kd
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
    .name:           embeddingx_bag_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         embeddingx_bag_kernel.kd
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
    .name:           embeddingx_one_hot_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         embeddingx_one_hot_kernel.kd
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
    .name:           embeddingx_rope_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         embeddingx_rope_kernel.kd
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
    .name:           embeddingx_rope_bwd_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         embeddingx_rope_bwd_kernel.kd
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
