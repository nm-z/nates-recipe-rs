	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	poolx_global_avg_pool_kernel ; -- Begin function poolx_global_avg_pool_kernel
	.globl	poolx_global_avg_pool_kernel
	.p2align	8
	.type	poolx_global_avg_pool_kernel,@function
poolx_global_avg_pool_kernel:           ; @poolx_global_avg_pool_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s6, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_6
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc1 .LBB0_4
; %bb.2:
	s_abs_i32 s4, s6
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s7, 0, s4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v5, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s7, v0
	s_ashr_i32 s7, s6, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, v5, v0, 0
	v_mul_lo_u32 v0, v4, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v5, v0
	v_subrev_nc_u32_e32 v5, s4, v0
	v_cmp_le_u32_e32 vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v0, v0, v5 :: v_dual_add_nc_u32 v3, 1, v4
	v_cndmask_b32_e32 v3, v4, v3, vcc_lo
	v_xor_b32_e32 v5, s7, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s4, v0
	v_add_nc_u32_e32 v4, 1, v3
	s_add_i32 s4, s5, -1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v0, v3, v4 :: v_dual_mov_b32 v3, 0
	v_mov_b32_e32 v4, 0
	v_xor_b32_e32 v0, v0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v5
	v_mul_lo_u32 v0, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_mad_u64_u32 v[5:6], null, v0, s4, v[1:2]
	s_mov_b32 s4, s5
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v6, 31, v5
	s_add_i32 s4, s4, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s4, 0
	v_lshlrev_b64 v[6:7], 3, v[5:6]
	v_add_nc_u32_e32 v5, s6, v5
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v6
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[3:4], v[3:4], v[6:7]
	s_cbranch_scc0 .LBB0_3
	s_branch .LBB0_5
.LBB0_4:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB0_5:
	v_cvt_f64_i32_e32 v[5:6], s5
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
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
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_div_fixup_f64 v[3:4], v[7:8], v[5:6], v[3:4]
	global_store_b64 v[0:1], v[3:4], off
.LBB0_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel poolx_global_avg_pool_kernel
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
.Lfunc_end0:
	.size	poolx_global_avg_pool_kernel, .Lfunc_end0-poolx_global_avg_pool_kernel
                                        ; -- End function
	.set poolx_global_avg_pool_kernel.num_vgpr, 15
	.set poolx_global_avg_pool_kernel.num_agpr, 0
	.set poolx_global_avg_pool_kernel.numbered_sgpr, 8
	.set poolx_global_avg_pool_kernel.num_named_barrier, 0
	.set poolx_global_avg_pool_kernel.private_seg_size, 0
	.set poolx_global_avg_pool_kernel.uses_vcc, 1
	.set poolx_global_avg_pool_kernel.uses_flat_scratch, 0
	.set poolx_global_avg_pool_kernel.has_dyn_sized_stack, 0
	.set poolx_global_avg_pool_kernel.has_recursion, 0
	.set poolx_global_avg_pool_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 520
; TotalNumSgprs: 10
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.text
	.protected	poolx_global_max_pool_kernel ; -- Begin function poolx_global_max_pool_kernel
	.globl	poolx_global_max_pool_kernel
	.p2align	8
	.type	poolx_global_max_pool_kernel,@function
poolx_global_max_pool_kernel:           ; @poolx_global_max_pool_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s6, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_6
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc1 .LBB1_4
; %bb.2:
	s_abs_i32 s4, s6
	v_sub_nc_u32_e32 v4, 0, v1
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s7, 0, s4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v5, v1, v4
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v3, s7, v0
	s_ashr_i32 s7, s6, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v3, v0, v3
	v_add_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[3:4], null, v5, v0, 0
	v_mul_lo_u32 v0, v4, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v5, v0
	v_subrev_nc_u32_e32 v5, s4, v0
	v_cmp_le_u32_e32 vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v0, v0, v5 :: v_dual_add_nc_u32 v3, 1, v4
	v_cndmask_b32_e32 v3, v4, v3, vcc_lo
	v_xor_b32_e32 v5, s7, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s4, v0
	v_add_nc_u32_e32 v4, 1, v3
	s_add_i32 s4, s5, -1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v0, v3, v4 :: v_dual_mov_b32 v3, 0x85ebc8a0
	v_mov_b32_e32 v4, 0xffe1ccf3
	v_xor_b32_e32 v0, v0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v5
	v_mul_lo_u32 v0, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_mad_u64_u32 v[5:6], null, v0, s4, v[1:2]
	.p2align	6
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ashrrev_i32_e32 v6, 31, v5
	s_add_i32 s5, s5, -1
	s_cmp_eq_u32 s5, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[6:7], 3, v[5:6]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v6, vcc_lo, s0, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[6:7], v[3:4]
	v_dual_cndmask_b32 v4, v4, v7 :: v_dual_add_nc_u32 v5, s6, v5
	v_cndmask_b32_e32 v3, v3, v6, vcc_lo
	s_cbranch_scc0 .LBB1_3
	s_branch .LBB1_5
.LBB1_4:
	v_mov_b32_e32 v3, 0x85ebc8a0
	v_mov_b32_e32 v4, 0xffe1ccf3
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
	.amdhsa_kernel poolx_global_max_pool_kernel
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
		.amdhsa_next_free_vgpr 8
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
	.size	poolx_global_max_pool_kernel, .Lfunc_end1-poolx_global_max_pool_kernel
                                        ; -- End function
	.set poolx_global_max_pool_kernel.num_vgpr, 8
	.set poolx_global_max_pool_kernel.num_agpr, 0
	.set poolx_global_max_pool_kernel.numbered_sgpr, 8
	.set poolx_global_max_pool_kernel.num_named_barrier, 0
	.set poolx_global_max_pool_kernel.private_seg_size, 0
	.set poolx_global_max_pool_kernel.uses_vcc, 1
	.set poolx_global_max_pool_kernel.uses_flat_scratch, 0
	.set poolx_global_max_pool_kernel.has_dyn_sized_stack, 0
	.set poolx_global_max_pool_kernel.has_recursion, 0
	.set poolx_global_max_pool_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 432
; TotalNumSgprs: 10
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
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
	.protected	poolx_lp_pool_kernel    ; -- Begin function poolx_lp_pool_kernel
	.globl	poolx_lp_pool_kernel
	.p2align	8
	.type	poolx_lp_pool_kernel,@function
poolx_lp_pool_kernel:                   ; @poolx_lp_pool_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[8:11], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s10, s8
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB2_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x20
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s9, 1
	s_cbranch_scc1 .LBB2_4
; %bb.2:
	s_abs_i32 s0, s10
	v_sub_nc_u32_e32 v6, 0, v1
	v_cvt_f32_u32_e32 v0, s0
	s_sub_i32 s1, 0, s0
	s_add_i32 s8, s9, -1
	s_mov_b32 s12, 0x55555555
	v_max_i32_e32 v8, v1, v6
	v_rcp_iflag_f32_e32 v0, v0
	s_mov_b32 s14, 0x968915a9
	s_mov_b32 s16, 0x4222de17
	s_mov_b32 s18, 0x3abe935a
	s_mov_b32 s20, 0x47e6c9c2
	s_mov_b32 s22, 0xcfa74449
	s_mov_b32 s24, 0x71bf3c30
	s_mov_b32 s26, 0x1c7792ce
	s_mov_b32 s28, 0x924920da
	s_mov_b32 s30, 0x9999999c
	s_waitcnt_depctr 0xfff
	v_dual_mul_f32 v0, 0x4f7ffffe, v0 :: v_dual_mov_b32 v3, 0
	s_mov_b32 s34, 0xfefa39ef
	s_mov_b32 s36, 0x3b39803f
	s_mov_b32 s40, 0xd5df274d
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_u32_f32_e32 v5, v0
	s_mov_b32 s42, 0x652b82fe
	s_mov_b32 s48, 0xfca7ab0c
	s_mov_b32 s50, 0x6a5dcb37
	s_mov_b32 s52, 0x623fde64
	v_mul_lo_u32 v0, s1, v5
	s_ashr_i32 s1, s10, 31
	s_mov_b32 s54, 0x7c89e6b0
	s_mov_b32 s56, 0x14761f6e
	s_mov_b32 s58, 0x1852b7b0
	s_mov_b32 s60, 0x11122322
	s_mov_b32 s62, 0x555502a1
	s_mov_b32 s64, 0x55555511
	v_mul_hi_u32 v7, v5, v0
	v_mov_b32_e32 v4, 0
	s_mov_b32 s66, 11
	s_mov_b32 s13, 0x3fe55555
	s_mov_b32 s15, 0x3fba6564
	s_mov_b32 s17, 0x3fbdee67
	s_mov_b32 s19, 0x3fbe25e4
	s_mov_b32 s21, 0x3fc110ef
	v_add_nc_u32_e32 v7, v5, v7
	s_mov_b32 s23, 0x3fc3b13b
	s_mov_b32 s25, 0x3fc745d1
	s_mov_b32 s27, 0x3fcc71c7
	s_mov_b32 s29, 0x3fd24924
	v_mad_u64_u32 v[5:6], null, v8, v7, 0
	s_waitcnt lgkmcnt(0)
	v_mov_b32_e32 v0, s3
	s_mov_b32 s31, 0x3fd99999
	s_mov_b32 s35, 0x3fe62e42
	s_mov_b32 s37, 0x3c7abc9e
	s_mov_b32 s39, 0xbfe55555
	s_mov_b32 s41, 0x3c8543b0
	v_mul_lo_u32 v5, v6, s0
	v_add_nc_u32_e32 v7, 1, v6
	s_mov_b32 s43, 0x3ff71547
	s_mov_b32 s45, 0xbfe62e42
	s_mov_b32 s47, 0xbc7abc9e
	s_mov_b32 s49, 0x3e928af3
	s_mov_b32 s51, 0x3e5ade15
	s_mov_b32 s53, 0x3ec71dee
	v_sub_nc_u32_e32 v5, v8, v5
	s_mov_b32 s55, 0x3efa0199
	s_mov_b32 s57, 0x3f2a01a0
	s_mov_b32 s59, 0x3f56c16c
	s_mov_b32 s61, 0x3f811111
	v_subrev_nc_u32_e32 v8, s0, v5
	v_cmp_le_u32_e32 vcc_lo, s0, v5
	s_mov_b32 s38, s12
	s_mov_b32 s44, s34
	s_mov_b32 s46, s36
	s_mov_b32 s63, 0x3fa55555
	v_dual_cndmask_b32 v6, v6, v7 :: v_dual_cndmask_b32 v5, v5, v8
	v_xor_b32_e32 v8, s1, v2
	s_mov_b32 s65, 0x3fc55555
	s_mov_b32 s67, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v7, 1, v6
	v_cmp_le_u32_e32 vcc_lo, s0, v5
	v_cndmask_b32_e32 v5, v6, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v5, v5, v8
	v_sub_nc_u32_e32 v5, v5, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v7, s10, v5
	v_mad_u64_u32 v[5:6], null, v7, s8, v[1:2]
.LBB2_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v6, 31, v5
	s_add_i32 s9, s9, -1
	v_lshlrev_b64 v[6:7], 3, v[5:6]
	v_add_nc_u32_e32 v5, s10, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s4, v6
	v_add_co_ci_u32_e64 v7, null, s5, v7, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(0)
	v_cmp_neq_f64_e64 vcc_lo, |v[6:7]|, 1.0
	v_cndmask_b32_e32 v9, 0x3ff00000, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v8, 0, s2, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[8:9]
	v_cndmask_b32_e32 v7, 0x3ff00000, v7, vcc_lo
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[10:11], |v[6:7]|
	v_cmp_gt_f64_e32 vcc_lo, s[12:13], v[10:11]
	v_cndmask_b32_e64 v12, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[10:11], v[10:11], v12
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[18:19], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[12:13]
	v_add_f64 v[20:21], v[12:13], -1.0
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	v_mul_f64 v[16:17], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[22:23], v[12:13], v[16:17]
	v_fma_f64 v[12:13], v[16:17], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[16:17], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[22:23], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[18:19], -v[12:13]
	v_add_f64 v[22:23], v[12:13], -v[22:23]
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[10:11], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	v_add_f64 v[12:13], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], -v[16:17]
	v_mul_f64 v[16:17], v[12:13], v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[12:13], -v[16:17]
	v_add_f64 v[18:19], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[18:19], v[14:15]
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[20:21], v[18:19], s[16:17], s[14:15]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	v_mul_f64 v[26:27], v[12:13], v[18:19]
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[22:23]
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[26:27]
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[30:31]
	v_mul_f64 v[22:23], v[18:19], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[18:19], v[20:21], -v[22:23]
	v_fma_f64 v[16:17], v[14:15], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[20:21], v[22:23], v[16:17]
	v_add_f64 v[24:25], v[20:21], s[12:13]
	v_add_f64 v[22:23], v[20:21], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[28:29], v[24:25], s[38:39]
	v_add_f64 v[16:17], v[16:17], -v[22:23]
	v_fma_f64 v[22:23], v[18:19], v[12:13], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], -v[28:29]
	v_add_f64 v[16:17], v[16:17], s[40:41]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[18:19], v[10:11], v[22:23]
	v_ldexp_f64 v[10:11], v[10:11], 1
	v_add_f64 v[16:17], v[16:17], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[14:15], v[12:13], v[18:19]
	v_ldexp_f64 v[12:13], v[12:13], 1
	v_add_f64 v[18:19], v[24:25], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[26:27], v[14:15]
	v_add_f64 v[22:23], v[24:25], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[24:25], v[20:21], v[18:19]
	v_add_f64 v[26:27], v[20:21], -v[26:27]
	v_add_f64 v[16:17], v[16:17], v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], v[20:21], v[18:19], -v[24:25]
	v_add_f64 v[14:15], v[14:15], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[20:21], v[16:17], v[22:23]
	v_fma_f64 v[14:15], v[14:15], v[18:19], v[16:17]
	v_frexp_exp_i32_f64_e32 v18, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[24:25], v[14:15]
	v_subrev_co_ci_u32_e64 v18, null, 0, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[18:19], v18
	v_add_f64 v[20:21], v[12:13], v[16:17]
	v_add_f64 v[22:23], v[16:17], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[24:25], v[18:19], s[34:35]
	v_add_f64 v[12:13], v[20:21], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_fma_f64 v[22:23], v[18:19], s[34:35], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[16:17], -v[12:13]
	v_add_f64 v[10:11], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[18:19], s[36:37], v[22:23]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[24:25], v[14:15]
	v_add_f64 v[16:17], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[12:13], -v[24:25]
	v_add_f64 v[18:19], v[12:13], v[16:17]
	v_add_f64 v[20:21], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	v_add_f64 v[22:23], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	v_add_f64 v[26:27], v[18:19], -v[22:23]
	v_add_f64 v[16:17], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[14:15], v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[16:17], v[12:13]
	v_add_f64 v[16:17], v[20:21], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[20:21], v[12:13]
	v_add_f64 v[20:21], v[20:21], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[18:19], v[12:13]
	v_add_f64 v[14:15], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[22:23], -v[18:19]
	v_add_f64 v[10:11], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[22:23], v[10:11]
	v_add_f64 v[14:15], v[12:13], -v[22:23]
	v_mul_f64 v[16:17], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_fma_f64 v[12:13], v[8:9], v[12:13], -v[16:17]
	v_cmp_class_f64_e64 vcc_lo, v[16:17], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_dual_cndmask_b32 v15, v13, v17 :: v_dual_cndmask_b32 v14, v12, v16
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	v_cmp_eq_f64_e64 s8, |v[6:7]|, 0
	v_cmp_class_f64_e64 s11, v[6:7], 0x204
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[14:15]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_cndmask_b32_e32 v11, 0, v11, vcc_lo
	v_mul_f64 v[18:19], v[14:15], s[42:43]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[14:15]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[14:15]
	v_cndmask_b32_e32 v10, 0, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[18:19], v[18:19]
	s_and_b32 vcc_lo, s1, s0
	v_fma_f64 v[20:21], v[18:19], s[44:45], v[14:15]
	v_cvt_i32_f64_e32 v24, v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], s[46:47], v[20:21]
	v_fma_f64 v[22:23], v[20:21], s[50:51], s[48:49]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[52:53]
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[54:55]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[56:57]
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[58:59]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[60:61]
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[62:63]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[64:65]
	v_fma_f64 v[22:23], v[20:21], v[22:23], s[66:67]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[20:21], v[22:23], 1.0
	v_fma_f64 v[18:19], v[20:21], v[22:23], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[16:17], v[18:19], v24
	v_cndmask_b32_e64 v12, 0x7ff00000, v17, s0
	v_cmp_neq_f64_e64 s0, v[8:9], |v[8:9]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v13, 0, v12, s1
	v_cmp_lt_f64_e64 s1, |v[6:7]|, 1.0
	v_cndmask_b32_e32 v12, 0, v16, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[8:9]
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[12:13]
	s_xor_b32 s0, s0, s1
	v_cmp_neq_f64_e64 s1, |v[6:7]|, 1.0
	v_cndmask_b32_e64 v14, 0x7ff00000, 0, s0
	v_cmp_class_f64_e64 s0, v[12:13], 0x204
	s_xor_b32 s33, vcc_lo, s8
	s_or_b32 vcc_lo, s8, s11
	v_cndmask_b32_e64 v11, v11, v13, s0
	v_cndmask_b32_e64 v13, 0x3ff00000, v14, s1
	v_cmp_class_f64_e64 s1, v[8:9], 0x204
	v_cndmask_b32_e64 v14, 0x7ff00000, 0, s33
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b32_e32 v11, 0x7fffffff, v11
	v_cndmask_b32_e64 v11, v11, v13, s1
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v11, v11, v14, vcc_lo
	v_cndmask_b32_e64 v10, v10, v12, s0
	s_or_b32 s0, vcc_lo, s1
	v_cmp_o_f64_e64 vcc_lo, |v[6:7]|, v[8:9]
	s_cmp_eq_u32 s9, 0
	v_cndmask_b32_e64 v10, v10, 0, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, 0, v10, vcc_lo
	v_cndmask_b32_e32 v7, 0x7ff80000, v11, vcc_lo
	v_add_f64 v[3:4], v[3:4], v[6:7]
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
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB2_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel poolx_lp_pool_kernel
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
		.amdhsa_next_free_vgpr 30
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
		.amdhsa_inst_pref_size 37
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
	.size	poolx_lp_pool_kernel, .Lfunc_end2-poolx_lp_pool_kernel
                                        ; -- End function
	.set poolx_lp_pool_kernel.num_vgpr, 30
	.set poolx_lp_pool_kernel.num_agpr, 0
	.set poolx_lp_pool_kernel.numbered_sgpr, 68
	.set poolx_lp_pool_kernel.num_named_barrier, 0
	.set poolx_lp_pool_kernel.private_seg_size, 0
	.set poolx_lp_pool_kernel.uses_vcc, 1
	.set poolx_lp_pool_kernel.uses_flat_scratch, 0
	.set poolx_lp_pool_kernel.has_dyn_sized_stack, 0
	.set poolx_lp_pool_kernel.has_recursion, 0
	.set poolx_lp_pool_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4712
; TotalNumSgprs: 70
; NumVgprs: 30
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 70
; NumVGPRsForWavesPerEU: 30
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
	.protected	poolx_adaptive_avg_pool_kernel ; -- Begin function poolx_adaptive_avg_pool_kernel
	.globl	poolx_adaptive_avg_pool_kernel
	.p2align	8
	.type	poolx_adaptive_avg_pool_kernel,@function
poolx_adaptive_avg_pool_kernel:         ; @poolx_adaptive_avg_pool_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s6, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB3_14
; %bb.1:
	s_abs_i32 s2, s6
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
	s_ashr_i32 s3, s6, 31
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s6
	v_sub_nc_u32_e32 v6, v1, v2
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_i64_i32 v[4:5], null, v6, s5, 0
	v_or_b32_e32 v3, s3, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_ne_u64_e32 vcc_lo, 0, v[2:3]
                                        ; implicit-def: $vgpr2_vgpr3
	s_and_saveexec_b32 s2, vcc_lo
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB3_3
; %bb.2:
	s_ashr_i32 s8, s3, 31
	v_ashrrev_i32_e32 v9, 31, v5
	s_add_u32 s10, s6, s8
	s_mov_b32 s9, s8
	s_addc_u32 s11, s3, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[10:11], s[10:11], s[8:9]
	v_add_co_u32 v4, vcc_lo, v4, v9
	v_cvt_f32_u32_e32 v2, s10
	v_cvt_f32_u32_e32 v3, s11
	s_sub_u32 s9, 0, s10
	s_subb_u32 s12, 0, s11
	v_add_co_ci_u32_e64 v5, null, v5, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v2, v3, 0x4f800000, v2
	v_xor_b32_e32 v10, v4, v9
	v_xor_b32_e32 v11, v5, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x5f7ffffc, v2
	v_mul_f32_e32 v3, 0x2f800000, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v3, v3
	v_fmamk_f32 v2, v3, 0xcf800000, v2
	v_cvt_u32_f32_e32 v3, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v2, v2
	v_readfirstlane_b32 s2, v3
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v2
	s_mul_i32 s13, s9, s2
	s_mul_hi_u32 s15, s9, s7
	s_mul_i32 s14, s12, s7
	s_add_i32 s13, s15, s13
	s_mul_i32 s16, s9, s7
	s_add_i32 s13, s13, s14
	s_mul_hi_u32 s15, s7, s16
	s_mul_i32 s18, s7, s13
	s_mul_hi_u32 s17, s2, s16
	s_mul_i32 s14, s2, s16
	s_mul_hi_u32 s16, s7, s13
	s_add_u32 s15, s15, s18
	s_addc_u32 s16, 0, s16
	s_mul_hi_u32 s19, s2, s13
	s_add_u32 s14, s15, s14
	s_mul_i32 s13, s2, s13
	s_addc_u32 s14, s16, s17
	s_addc_u32 s15, s19, 0
	s_add_u32 s13, s14, s13
	s_addc_u32 s14, 0, s15
	s_add_u32 s7, s7, s13
	s_cselect_b32 s13, -1, 0
	s_mul_hi_u32 s15, s9, s7
	s_cmp_lg_u32 s13, 0
	s_mul_i32 s13, s9, s7
	s_addc_u32 s2, s2, s14
	s_mul_i32 s12, s12, s7
	s_mul_i32 s9, s9, s2
	s_mul_hi_u32 s14, s7, s13
	s_add_i32 s9, s15, s9
	s_mul_hi_u32 s15, s2, s13
	s_add_i32 s9, s9, s12
	s_mul_i32 s12, s2, s13
	s_mul_i32 s17, s7, s9
	s_mul_hi_u32 s16, s7, s9
	s_add_u32 s14, s14, s17
	s_addc_u32 s16, 0, s16
	s_mul_hi_u32 s13, s2, s9
	s_add_u32 s12, s14, s12
	s_mul_i32 s9, s2, s9
	s_addc_u32 s12, s16, s15
	s_addc_u32 s13, s13, 0
	s_add_u32 s9, s12, s9
	s_addc_u32 s12, 0, s13
	s_add_u32 s7, s7, s9
	s_cselect_b32 s9, -1, 0
	v_mul_hi_u32 v12, v10, s7
	s_cmp_lg_u32 s9, 0
	v_mad_u64_u32 v[4:5], null, v11, s7, 0
	s_addc_u32 s2, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[2:3], null, v10, s2, 0
	v_mad_u64_u32 v[7:8], null, v11, s2, 0
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
	v_add_co_u32 v8, s2, v4, 2
	v_add_co_ci_u32_e64 v10, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v11, v3
	v_sub_co_u32 v12, s2, v2, s10
	v_sub_co_ci_u32_e64 v3, null, v11, v3, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s11, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s10, v12
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s2
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
	v_xor_b32_e32 v7, s8, v9
	v_cmp_ne_u32_e64 s2, 0, v2
	v_dual_cndmask_b32 v2, v11, v8 :: v_dual_cndmask_b32 v3, v12, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, v4, v2, s2
	v_cndmask_b32_e64 v3, v5, v3, s2
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v2, v2, v7
	v_xor_b32_e32 v3, v3, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v2, vcc_lo, v2, v7
	v_sub_co_ci_u32_e64 v3, null, v3, v7, vcc_lo
.LBB3_3:
	s_or_saveexec_b32 s2, s4
	v_cvt_f32_u32_e32 v7, s6
	s_xor_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB3_5
; %bb.4:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v2, v7
	s_sub_i32 s4, 0, s6
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	v_cvt_u32_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v3, s4, v2
	v_mul_hi_u32 v3, v2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v2, v2, v3
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v3, v2, s6
	v_sub_nc_u32_e32 v3, v4, v3
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s6, v3
	v_cmp_le_u32_e32 vcc_lo, s6, v3
	v_dual_cndmask_b32 v3, v3, v5 :: v_dual_cndmask_b32 v2, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s6, v3
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
.LBB3_5:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_nc_u32_e32 v5, 1, v6
	s_add_u32 s8, s6, -1
	s_addc_u32 s9, s3, -1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_i64_i32 v[3:4], null, v5, s5, s[8:9]
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_or_b32_e32 v6, s3, v4
	v_cmp_ne_u64_e32 vcc_lo, 0, v[5:6]
                                        ; implicit-def: $vgpr5_vgpr6
	s_and_saveexec_b32 s2, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB3_7
; %bb.6:
	s_ashr_i32 s8, s3, 31
	v_ashrrev_i32_e32 v9, 31, v4
	s_add_u32 s2, s6, s8
	s_mov_b32 s9, s8
	s_addc_u32 s3, s3, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[10:11], s[2:3], s[8:9]
	v_add_co_u32 v3, vcc_lo, v3, v9
	v_cvt_f32_u32_e32 v5, s10
	v_cvt_f32_u32_e32 v6, s11
	s_sub_u32 s7, 0, s10
	s_subb_u32 s9, 0, s11
	v_add_co_ci_u32_e64 v4, null, v4, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v5, v6, 0x4f800000, v5
	v_xor_b32_e32 v10, v3, v9
	v_xor_b32_e32 v11, v4, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v5, v5
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v5, 0x5f7ffffc, v5
	v_mul_f32_e32 v6, 0x2f800000, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v6, v6
	v_fmamk_f32 v5, v6, 0xcf800000, v5
	v_cvt_u32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v5, v5
	v_readfirstlane_b32 s2, v6
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s3, v5
	s_mul_i32 s12, s7, s2
	s_mul_hi_u32 s14, s7, s3
	s_mul_i32 s13, s9, s3
	s_add_i32 s12, s14, s12
	s_mul_i32 s15, s7, s3
	s_add_i32 s12, s12, s13
	s_mul_hi_u32 s14, s3, s15
	s_mul_i32 s17, s3, s12
	s_mul_hi_u32 s16, s2, s15
	s_mul_i32 s13, s2, s15
	s_mul_hi_u32 s15, s3, s12
	s_add_u32 s14, s14, s17
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s18, s2, s12
	s_add_u32 s13, s14, s13
	s_mul_i32 s12, s2, s12
	s_addc_u32 s13, s15, s16
	s_addc_u32 s14, s18, 0
	s_add_u32 s12, s13, s12
	s_addc_u32 s13, 0, s14
	s_add_u32 s3, s3, s12
	s_cselect_b32 s12, -1, 0
	s_mul_hi_u32 s14, s7, s3
	s_cmp_lg_u32 s12, 0
	s_mul_i32 s12, s7, s3
	s_addc_u32 s2, s2, s13
	s_mul_i32 s9, s9, s3
	s_mul_i32 s7, s7, s2
	s_mul_hi_u32 s13, s3, s12
	s_add_i32 s7, s14, s7
	s_mul_hi_u32 s14, s2, s12
	s_add_i32 s7, s7, s9
	s_mul_i32 s9, s2, s12
	s_mul_i32 s16, s3, s7
	s_mul_hi_u32 s15, s3, s7
	s_add_u32 s13, s13, s16
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s12, s2, s7
	s_add_u32 s9, s13, s9
	s_mul_i32 s7, s2, s7
	s_addc_u32 s9, s15, s14
	s_addc_u32 s12, s12, 0
	s_add_u32 s7, s9, s7
	s_addc_u32 s9, 0, s12
	s_add_u32 s3, s3, s7
	s_cselect_b32 s7, -1, 0
	v_mul_hi_u32 v12, v10, s3
	s_cmp_lg_u32 s7, 0
	v_mad_u64_u32 v[5:6], null, v11, s3, 0
	s_addc_u32 s2, s2, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[3:4], null, v10, s2, 0
	v_mad_u64_u32 v[7:8], null, v11, s2, 0
	v_add_co_u32 v3, vcc_lo, v12, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_add_co_u32 v3, vcc_lo, v3, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v3, vcc_lo, v4, v6, vcc_lo
	v_add_co_ci_u32_e32 v4, vcc_lo, 0, v8, vcc_lo
	v_add_co_u32 v5, vcc_lo, v3, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v6, null, 0, v4, vcc_lo
	v_mul_lo_u32 v7, s11, v5
	v_mad_u64_u32 v[3:4], null, s10, v5, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v8, s10, v6
	v_sub_co_u32 v3, vcc_lo, v10, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v4, v4, v8, v7
	v_add_co_u32 v8, s2, v5, 2
	v_add_co_ci_u32_e64 v10, null, 0, v6, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v11, v4
	v_sub_co_u32 v12, s2, v3, s10
	v_sub_co_ci_u32_e64 v4, null, v11, v4, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s11, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s10, v12
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s2
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s11, v7
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s10, v3
	v_cndmask_b32_e64 v3, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s11, v4
	v_cndmask_b32_e64 v13, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s11, v7
	v_cndmask_b32_e32 v7, v12, v11, vcc_lo
	v_add_co_u32 v11, vcc_lo, v5, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v12, null, 0, v6, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s11, v4
	v_cndmask_b32_e32 v3, v13, v3, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v7
	v_xor_b32_e32 v7, s8, v9
	v_cmp_ne_u32_e64 s2, 0, v3
	v_dual_cndmask_b32 v3, v11, v8 :: v_dual_cndmask_b32 v4, v12, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v3, v5, v3, s2
	v_cndmask_b32_e64 v4, v6, v4, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v3, v3, v7
	v_xor_b32_e32 v4, v4, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v5, vcc_lo, v3, v7
	v_sub_co_ci_u32_e64 v3, null, v4, v7, vcc_lo
                                        ; implicit-def: $vgpr7
                                        ; implicit-def: $vgpr3_vgpr4
.LBB3_7:
	s_and_not1_saveexec_b32 s2, s4
	s_cbranch_execz .LBB3_9
; %bb.8:
	v_rcp_iflag_f32_e32 v4, v7
	s_sub_i32 s3, 0, s6
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v5, s3, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v5, v4, v5
	v_add_nc_u32_e32 v4, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v4, v3, v4
	v_mul_lo_u32 v5, v4, s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v5
	v_add_nc_u32_e32 v5, 1, v4
	v_subrev_nc_u32_e32 v6, s6, v3
	v_cmp_le_u32_e32 vcc_lo, s6, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v3, v3, v6 :: v_dual_cndmask_b32 v4, v4, v5
	v_cmp_le_u32_e32 vcc_lo, s6, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v5, 1, v4
	v_cndmask_b32_e32 v5, v4, v5, vcc_lo
.LBB3_9:
	s_or_b32 exec_lo, exec_lo, s2
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_mov_b32 s4, exec_lo
	v_cmpx_gt_i32_e64 v5, v2
	s_cbranch_execz .LBB3_13
; %bb.10:
	v_mad_u64_u32 v[3:4], null, v0, s5, v[2:3]
	v_mov_b32_e32 v0, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[6:7], 3, v[3:4]
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v6
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
	s_mov_b32 s1, 0
.LBB3_11:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[6:7], off
	v_add_nc_u32_e32 v0, 1, v0
	v_add_co_u32 v6, s0, v6, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v7, null, 0, v7, s0
	v_cmp_ge_i32_e32 vcc_lo, v0, v5
	s_or_b32 s1, vcc_lo, s1
	s_waitcnt vmcnt(0)
	v_add_f64 v[3:4], v[3:4], v[8:9]
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execnz .LBB3_11
; %bb.12:
	s_or_b32 exec_lo, exec_lo, s1
.LBB3_13:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s4
	v_sub_nc_u32_e32 v0, v5, v2
	v_ashrrev_i32_e32 v2, 31, v1
	v_cvt_f64_i32_e32 v[5:6], v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[7:8], v[7:8], v[9:10], v[13:14]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_div_fixup_f64 v[3:4], v[7:8], v[5:6], v[3:4]
	global_store_b64 v[0:1], v[3:4], off
.LBB3_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel poolx_adaptive_avg_pool_kernel
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
		.amdhsa_inst_pref_size 19
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
	.size	poolx_adaptive_avg_pool_kernel, .Lfunc_end3-poolx_adaptive_avg_pool_kernel
                                        ; -- End function
	.set poolx_adaptive_avg_pool_kernel.num_vgpr, 15
	.set poolx_adaptive_avg_pool_kernel.num_agpr, 0
	.set poolx_adaptive_avg_pool_kernel.numbered_sgpr, 20
	.set poolx_adaptive_avg_pool_kernel.num_named_barrier, 0
	.set poolx_adaptive_avg_pool_kernel.private_seg_size, 0
	.set poolx_adaptive_avg_pool_kernel.uses_vcc, 1
	.set poolx_adaptive_avg_pool_kernel.uses_flat_scratch, 0
	.set poolx_adaptive_avg_pool_kernel.has_dyn_sized_stack, 0
	.set poolx_adaptive_avg_pool_kernel.has_recursion, 0
	.set poolx_adaptive_avg_pool_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2364
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
	.text
	.protected	poolx_adaptive_max_pool_kernel ; -- Begin function poolx_adaptive_max_pool_kernel
	.globl	poolx_adaptive_max_pool_kernel
	.p2align	8
	.type	poolx_adaptive_max_pool_kernel,@function
poolx_adaptive_max_pool_kernel:         ; @poolx_adaptive_max_pool_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s6, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB4_14
; %bb.1:
	s_abs_i32 s2, s6
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
	s_ashr_i32 s3, s6, 31
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s6
	v_sub_nc_u32_e32 v6, v1, v2
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_i64_i32 v[4:5], null, v6, s5, 0
	v_or_b32_e32 v3, s3, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_ne_u64_e32 vcc_lo, 0, v[2:3]
                                        ; implicit-def: $vgpr2_vgpr3
	s_and_saveexec_b32 s2, vcc_lo
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB4_3
; %bb.2:
	s_ashr_i32 s8, s3, 31
	v_ashrrev_i32_e32 v9, 31, v5
	s_add_u32 s10, s6, s8
	s_mov_b32 s9, s8
	s_addc_u32 s11, s3, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[10:11], s[10:11], s[8:9]
	v_add_co_u32 v4, vcc_lo, v4, v9
	v_cvt_f32_u32_e32 v2, s10
	v_cvt_f32_u32_e32 v3, s11
	s_sub_u32 s9, 0, s10
	s_subb_u32 s12, 0, s11
	v_add_co_ci_u32_e64 v5, null, v5, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v2, v3, 0x4f800000, v2
	v_xor_b32_e32 v10, v4, v9
	v_xor_b32_e32 v11, v5, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x5f7ffffc, v2
	v_mul_f32_e32 v3, 0x2f800000, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v3, v3
	v_fmamk_f32 v2, v3, 0xcf800000, v2
	v_cvt_u32_f32_e32 v3, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v2, v2
	v_readfirstlane_b32 s2, v3
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v2
	s_mul_i32 s13, s9, s2
	s_mul_hi_u32 s15, s9, s7
	s_mul_i32 s14, s12, s7
	s_add_i32 s13, s15, s13
	s_mul_i32 s16, s9, s7
	s_add_i32 s13, s13, s14
	s_mul_hi_u32 s15, s7, s16
	s_mul_i32 s18, s7, s13
	s_mul_hi_u32 s17, s2, s16
	s_mul_i32 s14, s2, s16
	s_mul_hi_u32 s16, s7, s13
	s_add_u32 s15, s15, s18
	s_addc_u32 s16, 0, s16
	s_mul_hi_u32 s19, s2, s13
	s_add_u32 s14, s15, s14
	s_mul_i32 s13, s2, s13
	s_addc_u32 s14, s16, s17
	s_addc_u32 s15, s19, 0
	s_add_u32 s13, s14, s13
	s_addc_u32 s14, 0, s15
	s_add_u32 s7, s7, s13
	s_cselect_b32 s13, -1, 0
	s_mul_hi_u32 s15, s9, s7
	s_cmp_lg_u32 s13, 0
	s_mul_i32 s13, s9, s7
	s_addc_u32 s2, s2, s14
	s_mul_i32 s12, s12, s7
	s_mul_i32 s9, s9, s2
	s_mul_hi_u32 s14, s7, s13
	s_add_i32 s9, s15, s9
	s_mul_hi_u32 s15, s2, s13
	s_add_i32 s9, s9, s12
	s_mul_i32 s12, s2, s13
	s_mul_i32 s17, s7, s9
	s_mul_hi_u32 s16, s7, s9
	s_add_u32 s14, s14, s17
	s_addc_u32 s16, 0, s16
	s_mul_hi_u32 s13, s2, s9
	s_add_u32 s12, s14, s12
	s_mul_i32 s9, s2, s9
	s_addc_u32 s12, s16, s15
	s_addc_u32 s13, s13, 0
	s_add_u32 s9, s12, s9
	s_addc_u32 s12, 0, s13
	s_add_u32 s7, s7, s9
	s_cselect_b32 s9, -1, 0
	v_mul_hi_u32 v12, v10, s7
	s_cmp_lg_u32 s9, 0
	v_mad_u64_u32 v[4:5], null, v11, s7, 0
	s_addc_u32 s2, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[2:3], null, v10, s2, 0
	v_mad_u64_u32 v[7:8], null, v11, s2, 0
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
	v_add_co_u32 v8, s2, v4, 2
	v_add_co_ci_u32_e64 v10, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v11, v3
	v_sub_co_u32 v12, s2, v2, s10
	v_sub_co_ci_u32_e64 v3, null, v11, v3, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s11, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s10, v12
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s2
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
	v_xor_b32_e32 v7, s8, v9
	v_cmp_ne_u32_e64 s2, 0, v2
	v_dual_cndmask_b32 v2, v11, v8 :: v_dual_cndmask_b32 v3, v12, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, v4, v2, s2
	v_cndmask_b32_e64 v3, v5, v3, s2
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v2, v2, v7
	v_xor_b32_e32 v3, v3, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v2, vcc_lo, v2, v7
	v_sub_co_ci_u32_e64 v3, null, v3, v7, vcc_lo
.LBB4_3:
	s_or_saveexec_b32 s2, s4
	v_cvt_f32_u32_e32 v7, s6
	s_xor_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB4_5
; %bb.4:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v2, v7
	s_sub_i32 s4, 0, s6
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	v_cvt_u32_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v3, s4, v2
	v_mul_hi_u32 v3, v2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v2, v2, v3
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v3, v2, s6
	v_sub_nc_u32_e32 v3, v4, v3
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s6, v3
	v_cmp_le_u32_e32 vcc_lo, s6, v3
	v_dual_cndmask_b32 v3, v3, v5 :: v_dual_cndmask_b32 v2, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s6, v3
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
.LBB4_5:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_nc_u32_e32 v5, 1, v6
	s_add_u32 s8, s6, -1
	s_addc_u32 s9, s3, -1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_i64_i32 v[3:4], null, v5, s5, s[8:9]
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_or_b32_e32 v6, s3, v4
	v_cmp_ne_u64_e32 vcc_lo, 0, v[5:6]
                                        ; implicit-def: $vgpr5_vgpr6
	s_and_saveexec_b32 s2, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB4_7
; %bb.6:
	s_ashr_i32 s8, s3, 31
	v_ashrrev_i32_e32 v9, 31, v4
	s_add_u32 s2, s6, s8
	s_mov_b32 s9, s8
	s_addc_u32 s3, s3, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[10:11], s[2:3], s[8:9]
	v_add_co_u32 v3, vcc_lo, v3, v9
	v_cvt_f32_u32_e32 v5, s10
	v_cvt_f32_u32_e32 v6, s11
	s_sub_u32 s7, 0, s10
	s_subb_u32 s9, 0, s11
	v_add_co_ci_u32_e64 v4, null, v4, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v5, v6, 0x4f800000, v5
	v_xor_b32_e32 v10, v3, v9
	v_xor_b32_e32 v11, v4, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v5, v5
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v5, 0x5f7ffffc, v5
	v_mul_f32_e32 v6, 0x2f800000, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v6, v6
	v_fmamk_f32 v5, v6, 0xcf800000, v5
	v_cvt_u32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v5, v5
	v_readfirstlane_b32 s2, v6
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s3, v5
	s_mul_i32 s12, s7, s2
	s_mul_hi_u32 s14, s7, s3
	s_mul_i32 s13, s9, s3
	s_add_i32 s12, s14, s12
	s_mul_i32 s15, s7, s3
	s_add_i32 s12, s12, s13
	s_mul_hi_u32 s14, s3, s15
	s_mul_i32 s17, s3, s12
	s_mul_hi_u32 s16, s2, s15
	s_mul_i32 s13, s2, s15
	s_mul_hi_u32 s15, s3, s12
	s_add_u32 s14, s14, s17
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s18, s2, s12
	s_add_u32 s13, s14, s13
	s_mul_i32 s12, s2, s12
	s_addc_u32 s13, s15, s16
	s_addc_u32 s14, s18, 0
	s_add_u32 s12, s13, s12
	s_addc_u32 s13, 0, s14
	s_add_u32 s3, s3, s12
	s_cselect_b32 s12, -1, 0
	s_mul_hi_u32 s14, s7, s3
	s_cmp_lg_u32 s12, 0
	s_mul_i32 s12, s7, s3
	s_addc_u32 s2, s2, s13
	s_mul_i32 s9, s9, s3
	s_mul_i32 s7, s7, s2
	s_mul_hi_u32 s13, s3, s12
	s_add_i32 s7, s14, s7
	s_mul_hi_u32 s14, s2, s12
	s_add_i32 s7, s7, s9
	s_mul_i32 s9, s2, s12
	s_mul_i32 s16, s3, s7
	s_mul_hi_u32 s15, s3, s7
	s_add_u32 s13, s13, s16
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s12, s2, s7
	s_add_u32 s9, s13, s9
	s_mul_i32 s7, s2, s7
	s_addc_u32 s9, s15, s14
	s_addc_u32 s12, s12, 0
	s_add_u32 s7, s9, s7
	s_addc_u32 s9, 0, s12
	s_add_u32 s3, s3, s7
	s_cselect_b32 s7, -1, 0
	v_mul_hi_u32 v12, v10, s3
	s_cmp_lg_u32 s7, 0
	v_mad_u64_u32 v[5:6], null, v11, s3, 0
	s_addc_u32 s2, s2, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[3:4], null, v10, s2, 0
	v_mad_u64_u32 v[7:8], null, v11, s2, 0
	v_add_co_u32 v3, vcc_lo, v12, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_add_co_u32 v3, vcc_lo, v3, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v3, vcc_lo, v4, v6, vcc_lo
	v_add_co_ci_u32_e32 v4, vcc_lo, 0, v8, vcc_lo
	v_add_co_u32 v5, vcc_lo, v3, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v6, null, 0, v4, vcc_lo
	v_mul_lo_u32 v7, s11, v5
	v_mad_u64_u32 v[3:4], null, s10, v5, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v8, s10, v6
	v_sub_co_u32 v3, vcc_lo, v10, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v4, v4, v8, v7
	v_add_co_u32 v8, s2, v5, 2
	v_add_co_ci_u32_e64 v10, null, 0, v6, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v11, v4
	v_sub_co_u32 v12, s2, v3, s10
	v_sub_co_ci_u32_e64 v4, null, v11, v4, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s11, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s10, v12
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s2
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s11, v7
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s10, v3
	v_cndmask_b32_e64 v3, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s11, v4
	v_cndmask_b32_e64 v13, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s11, v7
	v_cndmask_b32_e32 v7, v12, v11, vcc_lo
	v_add_co_u32 v11, vcc_lo, v5, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v12, null, 0, v6, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s11, v4
	v_cndmask_b32_e32 v3, v13, v3, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v7
	v_xor_b32_e32 v7, s8, v9
	v_cmp_ne_u32_e64 s2, 0, v3
	v_dual_cndmask_b32 v3, v11, v8 :: v_dual_cndmask_b32 v4, v12, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v3, v5, v3, s2
	v_cndmask_b32_e64 v4, v6, v4, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v3, v3, v7
	v_xor_b32_e32 v4, v4, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v5, vcc_lo, v3, v7
	v_sub_co_ci_u32_e64 v3, null, v4, v7, vcc_lo
                                        ; implicit-def: $vgpr7
                                        ; implicit-def: $vgpr3_vgpr4
.LBB4_7:
	s_and_not1_saveexec_b32 s2, s4
	s_cbranch_execz .LBB4_9
; %bb.8:
	v_rcp_iflag_f32_e32 v4, v7
	s_sub_i32 s3, 0, s6
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v5, s3, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v5, v4, v5
	v_add_nc_u32_e32 v4, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v4, v3, v4
	v_mul_lo_u32 v5, v4, s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v5
	v_add_nc_u32_e32 v5, 1, v4
	v_subrev_nc_u32_e32 v6, s6, v3
	v_cmp_le_u32_e32 vcc_lo, s6, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v3, v3, v6 :: v_dual_cndmask_b32 v4, v4, v5
	v_cmp_le_u32_e32 vcc_lo, s6, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v5, 1, v4
	v_cndmask_b32_e32 v5, v4, v5, vcc_lo
.LBB4_9:
	s_or_b32 exec_lo, exec_lo, s2
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mov_b32_e32 v3, 0x85ebc8a0
	v_mov_b32_e32 v4, 0xffe1ccf3
	s_mov_b32 s4, exec_lo
	v_cmpx_lt_i32_e64 v2, v5
	s_cbranch_execz .LBB4_13
; %bb.10:
	v_mad_u64_u32 v[6:7], null, v0, s5, v[2:3]
	s_mov_b32 s5, 0
	v_ashrrev_i32_e32 v7, 31, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[6:7], 3, v[6:7]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v6, vcc_lo, s0, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
.LBB4_11:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[6:7], off
	v_add_co_u32 v6, s0, v6, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, 0, v7, s0
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[8:9], v[3:4]
	v_dual_cndmask_b32 v3, v3, v8 :: v_dual_add_nc_u32 v2, 1, v2
	v_cmp_ge_i32_e64 s1, v2, v5
	v_cndmask_b32_e32 v4, v4, v9, vcc_lo
	s_or_b32 s5, s1, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s5
	s_cbranch_execnz .LBB4_11
; %bb.12:
	s_or_b32 exec_lo, exec_lo, s5
.LBB4_13:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s4
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB4_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel poolx_adaptive_max_pool_kernel
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
.Lfunc_end4:
	.size	poolx_adaptive_max_pool_kernel, .Lfunc_end4-poolx_adaptive_max_pool_kernel
                                        ; -- End function
	.set poolx_adaptive_max_pool_kernel.num_vgpr, 14
	.set poolx_adaptive_max_pool_kernel.num_agpr, 0
	.set poolx_adaptive_max_pool_kernel.numbered_sgpr, 20
	.set poolx_adaptive_max_pool_kernel.num_named_barrier, 0
	.set poolx_adaptive_max_pool_kernel.private_seg_size, 0
	.set poolx_adaptive_max_pool_kernel.uses_vcc, 1
	.set poolx_adaptive_max_pool_kernel.uses_flat_scratch, 0
	.set poolx_adaptive_max_pool_kernel.has_dyn_sized_stack, 0
	.set poolx_adaptive_max_pool_kernel.has_recursion, 0
	.set poolx_adaptive_max_pool_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2256
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
	.text
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_1534af8ea322cf28,@object ; @__hip_cuid_1534af8ea322cf28
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_1534af8ea322cf28
__hip_cuid_1534af8ea322cf28:
	.byte	0                               ; 0x0
	.size	__hip_cuid_1534af8ea322cf28, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_1534af8ea322cf28
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
    .name:           poolx_global_avg_pool_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         poolx_global_avg_pool_kernel.kd
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
    .name:           poolx_global_max_pool_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         poolx_global_max_pool_kernel.kd
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
      - .offset:         16
        .size:           4
        .value_kind:     by_value
      - .offset:         20
        .size:           4
        .value_kind:     by_value
      - .offset:         24
        .size:           4
        .value_kind:     by_value
      - .offset:         32
        .size:           8
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
    .name:           poolx_lp_pool_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     70
    .sgpr_spill_count: 0
    .symbol:         poolx_lp_pool_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     30
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
    .name:           poolx_adaptive_avg_pool_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         poolx_adaptive_avg_pool_kernel.kd
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
    .name:           poolx_adaptive_max_pool_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         poolx_adaptive_max_pool_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     14
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
