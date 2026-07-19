	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	scanx_cummin_kernel     ; -- Begin function scanx_cummin_kernel
	.globl	scanx_cummin_kernel
	.p2align	8
	.type	scanx_cummin_kernel,@function
scanx_cummin_kernel:                    ; @scanx_cummin_kernel
; %bb.0:
	s_load_b32 s4, s[0:1], 0x10
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB0_3
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	s_mov_b32 s6, -1
	s_mov_b32 s7, 0x7fefffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v0, s6 :: v_dual_mov_b32 v1, s7
	v_mov_b32_e32 v2, 0
.LBB0_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	global_load_b64 v[3:4], v2, s[0:1]
	s_add_i32 s4, s4, -1
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_waitcnt vmcnt(0)
	v_cmp_lt_f64_e32 vcc_lo, v[3:4], v[0:1]
	v_dual_cndmask_b32 v1, v1, v4 :: v_dual_cndmask_b32 v0, v0, v3
	global_store_b64 v2, v[0:1], s[2:3]
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_lg_u32 s4, 0
	s_cbranch_scc1 .LBB0_2
.LBB0_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_cummin_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 20
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
.Lfunc_end0:
	.size	scanx_cummin_kernel, .Lfunc_end0-scanx_cummin_kernel
                                        ; -- End function
	.set scanx_cummin_kernel.num_vgpr, 5
	.set scanx_cummin_kernel.num_agpr, 0
	.set scanx_cummin_kernel.numbered_sgpr, 8
	.set scanx_cummin_kernel.num_named_barrier, 0
	.set scanx_cummin_kernel.private_seg_size, 0
	.set scanx_cummin_kernel.uses_vcc, 1
	.set scanx_cummin_kernel.uses_flat_scratch, 0
	.set scanx_cummin_kernel.has_dyn_sized_stack, 0
	.set scanx_cummin_kernel.has_recursion, 0
	.set scanx_cummin_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 152
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
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	scanx_cummax_kernel     ; -- Begin function scanx_cummax_kernel
	.globl	scanx_cummax_kernel
	.p2align	8
	.type	scanx_cummax_kernel,@function
scanx_cummax_kernel:                    ; @scanx_cummax_kernel
; %bb.0:
	s_load_b32 s4, s[0:1], 0x10
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB1_3
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	s_mov_b32 s6, -1
	s_mov_b32 s7, 0xffefffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v0, s6 :: v_dual_mov_b32 v1, s7
	v_mov_b32_e32 v2, 0
.LBB1_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	global_load_b64 v[3:4], v2, s[0:1]
	s_add_i32 s4, s4, -1
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[3:4], v[0:1]
	v_dual_cndmask_b32 v1, v1, v4 :: v_dual_cndmask_b32 v0, v0, v3
	global_store_b64 v2, v[0:1], s[2:3]
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_lg_u32 s4, 0
	s_cbranch_scc1 .LBB1_2
.LBB1_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_cummax_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 20
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
.Lfunc_end1:
	.size	scanx_cummax_kernel, .Lfunc_end1-scanx_cummax_kernel
                                        ; -- End function
	.set scanx_cummax_kernel.num_vgpr, 5
	.set scanx_cummax_kernel.num_agpr, 0
	.set scanx_cummax_kernel.numbered_sgpr, 8
	.set scanx_cummax_kernel.num_named_barrier, 0
	.set scanx_cummax_kernel.private_seg_size, 0
	.set scanx_cummax_kernel.uses_vcc, 1
	.set scanx_cummax_kernel.uses_flat_scratch, 0
	.set scanx_cummax_kernel.has_dyn_sized_stack, 0
	.set scanx_cummax_kernel.has_recursion, 0
	.set scanx_cummax_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 152
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
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	scanx_cumsum_kernel     ; -- Begin function scanx_cumsum_kernel
	.globl	scanx_cumsum_kernel
	.p2align	8
	.type	scanx_cumsum_kernel,@function
scanx_cumsum_kernel:                    ; @scanx_cumsum_kernel
; %bb.0:
	s_load_b32 s4, s[0:1], 0x10
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB2_3
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mov_b32_e32 v0, 0
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, 0
.LBB2_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	global_load_b64 v[3:4], v2, s[0:1]
	s_add_i32 s4, s4, -1
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[0:1], v[3:4]
	global_store_b64 v2, v[0:1], s[2:3]
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_lg_u32 s4, 0
	s_cbranch_scc1 .LBB2_2
.LBB2_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_cumsum_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 20
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
.Lfunc_end2:
	.size	scanx_cumsum_kernel, .Lfunc_end2-scanx_cumsum_kernel
                                        ; -- End function
	.set scanx_cumsum_kernel.num_vgpr, 5
	.set scanx_cumsum_kernel.num_agpr, 0
	.set scanx_cumsum_kernel.numbered_sgpr, 5
	.set scanx_cumsum_kernel.num_named_barrier, 0
	.set scanx_cumsum_kernel.private_seg_size, 0
	.set scanx_cumsum_kernel.uses_vcc, 1
	.set scanx_cumsum_kernel.uses_flat_scratch, 0
	.set scanx_cumsum_kernel.has_dyn_sized_stack, 0
	.set scanx_cumsum_kernel.has_recursion, 0
	.set scanx_cumsum_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 132
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
	.protected	scanx_cumprod_kernel    ; -- Begin function scanx_cumprod_kernel
	.globl	scanx_cumprod_kernel
	.p2align	8
	.type	scanx_cumprod_kernel,@function
scanx_cumprod_kernel:                   ; @scanx_cumprod_kernel
; %bb.0:
	s_load_b32 s4, s[0:1], 0x10
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB3_3
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mov_b32_e32 v0, 0
	v_dual_mov_b32 v1, 0x3ff00000 :: v_dual_mov_b32 v2, 0
.LBB3_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	global_load_b64 v[3:4], v2, s[0:1]
	s_add_i32 s4, s4, -1
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_waitcnt vmcnt(0)
	v_mul_f64 v[0:1], v[0:1], v[3:4]
	global_store_b64 v2, v[0:1], s[2:3]
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_lg_u32 s4, 0
	s_cbranch_scc1 .LBB3_2
.LBB3_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_cumprod_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 20
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
.Lfunc_end3:
	.size	scanx_cumprod_kernel, .Lfunc_end3-scanx_cumprod_kernel
                                        ; -- End function
	.set scanx_cumprod_kernel.num_vgpr, 5
	.set scanx_cumprod_kernel.num_agpr, 0
	.set scanx_cumprod_kernel.numbered_sgpr, 5
	.set scanx_cumprod_kernel.num_named_barrier, 0
	.set scanx_cumprod_kernel.private_seg_size, 0
	.set scanx_cumprod_kernel.uses_vcc, 1
	.set scanx_cumprod_kernel.uses_flat_scratch, 0
	.set scanx_cumprod_kernel.has_dyn_sized_stack, 0
	.set scanx_cumprod_kernel.has_recursion, 0
	.set scanx_cumprod_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 136
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
	.protected	scanx_excl_sum_kernel   ; -- Begin function scanx_excl_sum_kernel
	.globl	scanx_excl_sum_kernel
	.p2align	8
	.type	scanx_excl_sum_kernel,@function
scanx_excl_sum_kernel:                  ; @scanx_excl_sum_kernel
; %bb.0:
	s_load_b32 s4, s[0:1], 0x10
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB4_3
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mov_b32_e32 v0, 0
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, 0
.LBB4_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	global_store_b64 v2, v[0:1], s[2:3]
	global_load_b64 v[3:4], v2, s[0:1]
	s_add_i32 s4, s4, -1
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_cmp_lg_u32 s4, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[0:1], v[3:4]
	s_cbranch_scc1 .LBB4_2
.LBB4_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_excl_sum_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 20
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
.Lfunc_end4:
	.size	scanx_excl_sum_kernel, .Lfunc_end4-scanx_excl_sum_kernel
                                        ; -- End function
	.set scanx_excl_sum_kernel.num_vgpr, 5
	.set scanx_excl_sum_kernel.num_agpr, 0
	.set scanx_excl_sum_kernel.numbered_sgpr, 5
	.set scanx_excl_sum_kernel.num_named_barrier, 0
	.set scanx_excl_sum_kernel.private_seg_size, 0
	.set scanx_excl_sum_kernel.uses_vcc, 1
	.set scanx_excl_sum_kernel.uses_flat_scratch, 0
	.set scanx_excl_sum_kernel.has_dyn_sized_stack, 0
	.set scanx_excl_sum_kernel.has_recursion, 0
	.set scanx_excl_sum_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 132
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
	.protected	scanx_excl_prod_kernel  ; -- Begin function scanx_excl_prod_kernel
	.globl	scanx_excl_prod_kernel
	.p2align	8
	.type	scanx_excl_prod_kernel,@function
scanx_excl_prod_kernel:                 ; @scanx_excl_prod_kernel
; %bb.0:
	s_load_b32 s4, s[0:1], 0x10
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB5_3
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mov_b32_e32 v0, 0
	v_dual_mov_b32 v1, 0x3ff00000 :: v_dual_mov_b32 v2, 0
.LBB5_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	global_store_b64 v2, v[0:1], s[2:3]
	global_load_b64 v[3:4], v2, s[0:1]
	s_add_i32 s4, s4, -1
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_cmp_lg_u32 s4, 0
	s_waitcnt vmcnt(0)
	v_mul_f64 v[0:1], v[0:1], v[3:4]
	s_cbranch_scc1 .LBB5_2
.LBB5_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_excl_prod_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 20
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
.Lfunc_end5:
	.size	scanx_excl_prod_kernel, .Lfunc_end5-scanx_excl_prod_kernel
                                        ; -- End function
	.set scanx_excl_prod_kernel.num_vgpr, 5
	.set scanx_excl_prod_kernel.num_agpr, 0
	.set scanx_excl_prod_kernel.numbered_sgpr, 5
	.set scanx_excl_prod_kernel.num_named_barrier, 0
	.set scanx_excl_prod_kernel.private_seg_size, 0
	.set scanx_excl_prod_kernel.uses_vcc, 1
	.set scanx_excl_prod_kernel.uses_flat_scratch, 0
	.set scanx_excl_prod_kernel.has_dyn_sized_stack, 0
	.set scanx_excl_prod_kernel.has_recursion, 0
	.set scanx_excl_prod_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 136
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
	.protected	scanx_logcumsumexp_kernel ; -- Begin function scanx_logcumsumexp_kernel
	.globl	scanx_logcumsumexp_kernel
	.p2align	8
	.type	scanx_logcumsumexp_kernel,@function
scanx_logcumsumexp_kernel:              ; @scanx_logcumsumexp_kernel
; %bb.0:
	s_load_b32 s33, s[0:1], 0x10
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s33, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB6_7
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	s_mov_b32 s0, -1
	s_mov_b32 s1, 0xffefffff
	v_mov_b32_e32 v2, 0
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v0, s0
	v_dual_mov_b32 v1, s1 :: v_dual_mov_b32 v8, 0
	s_mov_b32 s34, 0x55555555
	s_mov_b32 s35, 0x3fe55555
	s_mov_b32 s2, 0x652b82fe
	s_mov_b32 s8, 0xfefa39ef
	s_mov_b32 s10, 0x3b39803f
	s_mov_b32 s12, 0xfca7ab0c
	s_mov_b32 s14, 0x6a5dcb37
	s_mov_b32 s16, 0x623fde64
	s_mov_b32 s18, 0x7c89e6b0
	s_mov_b32 s20, 0x14761f6e
	s_mov_b32 s22, 0x1852b7b0
	s_mov_b32 s24, 0x11122322
	s_mov_b32 s26, 0x555502a1
	s_mov_b32 s28, 0x55555511
	s_mov_b32 s30, 11
	s_mov_b32 s36, 0x6b47b09a
	s_mov_b32 s38, 0xbf559e2b
	s_mov_b32 s40, 0xd7f4df2e
	s_mov_b32 s42, 0x16291751
	s_mov_b32 s44, 0x9b27acf1
	s_mov_b32 s46, 0x998ef7b6
	s_mov_b32 s3, 0x3ff71547
	s_mov_b32 s9, 0xbfe62e42
	s_mov_b32 s11, 0xbc7abc9e
	s_mov_b32 s13, 0x3e928af3
	s_mov_b32 s15, 0x3e5ade15
	s_mov_b32 s17, 0x3ec71dee
	s_mov_b32 s19, 0x3efa0199
	s_mov_b32 s21, 0x3f2a01a0
	s_mov_b32 s23, 0x3f56c16c
	s_mov_b32 s25, 0x3f811111
	s_mov_b32 s27, 0x3fa55555
	s_mov_b32 s29, 0x3fc55555
	s_mov_b32 s31, 0x3fe00000
	s_mov_b32 s37, 0x3fc38538
	s_mov_b32 s39, 0x3fc3ab76
	s_mov_b32 s41, 0x3fc7474d
	s_mov_b32 s43, 0x3fcc71c0
	s_mov_b32 s45, 0x3fd24924
	s_mov_b32 s47, 0x3fd99999
	s_mov_b32 s48, 0x55555780
	s_mov_b32 s49, s35
	s_mov_b32 s51, 0x3fe62e42
	s_mov_b32 s50, s8
	s_mov_b32 s53, 0x3c7abc9e
	s_mov_b32 s52, s10
	s_branch .LBB6_3
.LBB6_2:                                ;   in Loop: Header=BB6_3 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[2:3], v[4:5]
	s_add_i32 s33, s33, -1
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
	v_cmp_gt_f64_e32 vcc_lo, s[34:35], v[2:3]
	v_cndmask_b32_e64 v6, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[2:3], v[2:3], v6
	v_add_f64 v[6:7], v[2:3], 1.0
	v_add_f64 v[13:14], v[2:3], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[9:10], v[6:7]
	v_add_f64 v[15:16], v[6:7], -1.0
	v_add_f64 v[2:3], v[2:3], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[6:7], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	v_fma_f64 v[11:12], -v[6:7], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[6:7], v[11:12]
	v_fma_f64 v[6:7], v[11:12], v[6:7], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[11:12], v[2:3], v[6:7]
	v_add_f64 v[6:7], v[17:18], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[13:14], -v[6:7]
	v_add_f64 v[17:18], v[6:7], -v[17:18]
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[2:3], v[17:18], -v[2:3]
	v_frexp_exp_i32_f64_e32 v17, v[4:5]
	v_add_f64 v[6:7], v[13:14], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], v[6:7]
	v_add_f64 v[2:3], v[15:16], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[2:3], v[9:10], v[2:3]
	v_add_f64 v[6:7], v[11:12], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[6:7], v[6:7]
	v_fma_f64 v[13:14], v[9:10], s[38:39], s[36:37]
	v_mul_f64 v[15:16], v[6:7], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[40:41]
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[44:45]
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[46:47]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[9:10], v[9:10], v[13:14], s[48:49]
	v_ldexp_f64 v[13:14], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[11:12]
	v_mul_f64 v[9:10], v[15:16], v[9:10]
	v_subrev_co_ci_u32_e64 v15, null, 0, v17, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[2:3], v[2:3], -v[6:7]
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x204
	v_cvt_f64_i32_e32 v[15:16], v15
	v_add_f64 v[11:12], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[2:3], v[2:3], 1
	v_mul_f64 v[17:18], v[15:16], s[50:51]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[11:12], -v[13:14]
	v_fma_f64 v[13:14], v[15:16], s[50:51], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[9:10], -v[6:7]
	v_fma_f64 v[9:10], v[15:16], s[52:53], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], v[6:7]
	v_add_f64 v[6:7], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[11:12], v[2:3]
	v_add_f64 v[17:18], v[6:7], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[6:7], v[13:14]
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[15:16], -v[6:7]
	v_add_f64 v[2:3], v[2:3], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[15:16], -v[19:20]
	v_add_f64 v[11:12], v[13:14], -v[19:20]
	v_add_f64 v[13:14], v[9:10], v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[21:22]
	v_add_f64 v[6:7], v[11:12], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[13:14], -v[9:10]
	v_add_f64 v[6:7], v[13:14], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[11:12]
	v_add_f64 v[2:3], v[2:3], -v[11:12]
	v_add_f64 v[17:18], v[15:16], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	v_add_f64 v[11:12], v[17:18], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], v[9:10]
	v_add_f64 v[6:7], v[6:7], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], v[6:7]
	v_add_f64 v[2:3], v[17:18], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v5
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v3, 0x7ff80000, v3, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v3, 0xfff00000, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[0:1], v[2:3]
	global_store_b64 v8, v[2:3], s[6:7]
	v_dual_mov_b32 v2, v4 :: v_dual_mov_b32 v3, v5
	s_add_u32 s6, s6, 8
	s_addc_u32 s7, s7, 0
	s_cmp_lg_u32 s33, 0
	s_cbranch_scc0 .LBB6_7
.LBB6_3:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	global_load_b64 v[6:7], v8, s[4:5]
	s_mov_b32 s0, -1
                                        ; implicit-def: $vgpr4_vgpr5
	s_waitcnt vmcnt(0)
	v_cmp_ngt_f64_e32 vcc_lo, v[6:7], v[0:1]
	s_cbranch_vccz .LBB6_5
; %bb.4:                                ;   in Loop: Header=BB6_3 Depth=1
	v_add_f64 v[4:5], v[6:7], -v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[9:10], v[4:5], s[2:3]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[4:5]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[4:5]
	v_rndne_f64_e32 v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[9:10], s[8:9], v[4:5]
	v_cvt_i32_f64_e32 v15, v[9:10]
	v_fma_f64 v[11:12], v[9:10], s[10:11], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], s[14:15], s[12:13]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[18:19]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[22:23]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[26:27]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[30:31]
	v_fma_f64 v[13:14], v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[11:12], v[13:14], 1.0
	v_ldexp_f64 v[9:10], v[9:10], v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v10, 0x7ff00000, v10, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v4, 0, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0, v10, s0
	s_mov_b32 s0, 0
	v_add_f64 v[4:5], v[2:3], v[4:5]
.LBB6_5:                                ;   in Loop: Header=BB6_3 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccnz .LBB6_2
; %bb.6:                                ;   in Loop: Header=BB6_3 Depth=1
	v_add_f64 v[0:1], v[0:1], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[4:5], v[0:1], s[2:3]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[0:1]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[0:1]
	v_rndne_f64_e32 v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[4:5], s[8:9], v[0:1]
	v_cvt_i32_f64_e32 v13, v[4:5]
	v_fma_f64 v[9:10], v[4:5], s[10:11], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], s[14:15], s[12:13]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[18:19]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[22:23]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[26:27]
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[30:31]
	v_fma_f64 v[11:12], v[9:10], v[11:12], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[9:10], v[11:12], 1.0
	v_ldexp_f64 v[4:5], v[4:5], v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v0, 0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v1, 0, v5, s0
	v_fma_f64 v[4:5], v[2:3], v[0:1], 1.0
	v_dual_mov_b32 v0, v6 :: v_dual_mov_b32 v1, v7
	s_branch .LBB6_2
.LBB6_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_logcumsumexp_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 20
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
		.amdhsa_next_free_vgpr 23
		.amdhsa_next_free_sgpr 54
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
.Lfunc_end6:
	.size	scanx_logcumsumexp_kernel, .Lfunc_end6-scanx_logcumsumexp_kernel
                                        ; -- End function
	.set scanx_logcumsumexp_kernel.num_vgpr, 23
	.set scanx_logcumsumexp_kernel.num_agpr, 0
	.set scanx_logcumsumexp_kernel.numbered_sgpr, 54
	.set scanx_logcumsumexp_kernel.num_named_barrier, 0
	.set scanx_logcumsumexp_kernel.private_seg_size, 0
	.set scanx_logcumsumexp_kernel.uses_vcc, 1
	.set scanx_logcumsumexp_kernel.uses_flat_scratch, 0
	.set scanx_logcumsumexp_kernel.has_dyn_sized_stack, 0
	.set scanx_logcumsumexp_kernel.has_recursion, 0
	.set scanx_logcumsumexp_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1768
; TotalNumSgprs: 56
; NumVgprs: 23
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 56
; NumVGPRsForWavesPerEU: 23
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
	.protected	scanx_assoc_add_kernel  ; -- Begin function scanx_assoc_add_kernel
	.globl	scanx_assoc_add_kernel
	.p2align	8
	.type	scanx_assoc_add_kernel,@function
scanx_assoc_add_kernel:                 ; @scanx_assoc_add_kernel
; %bb.0:
	s_load_b32 s4, s[0:1], 0x10
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s4, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB7_3
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mov_b32_e32 v0, 0
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, 0
.LBB7_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	global_load_b64 v[3:4], v2, s[0:1]
	s_add_i32 s4, s4, -1
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[0:1], v[3:4]
	global_store_b64 v2, v[0:1], s[2:3]
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_lg_u32 s4, 0
	s_cbranch_scc1 .LBB7_2
.LBB7_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_assoc_add_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 20
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
.Lfunc_end7:
	.size	scanx_assoc_add_kernel, .Lfunc_end7-scanx_assoc_add_kernel
                                        ; -- End function
	.set scanx_assoc_add_kernel.num_vgpr, 5
	.set scanx_assoc_add_kernel.num_agpr, 0
	.set scanx_assoc_add_kernel.numbered_sgpr, 5
	.set scanx_assoc_add_kernel.num_named_barrier, 0
	.set scanx_assoc_add_kernel.private_seg_size, 0
	.set scanx_assoc_add_kernel.uses_vcc, 1
	.set scanx_assoc_add_kernel.uses_flat_scratch, 0
	.set scanx_assoc_add_kernel.has_dyn_sized_stack, 0
	.set scanx_assoc_add_kernel.has_recursion, 0
	.set scanx_assoc_add_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 132
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
	.protected	scanx_recur_kernel      ; -- Begin function scanx_recur_kernel
	.globl	scanx_recur_kernel
	.p2align	8
	.type	scanx_recur_kernel,@function
scanx_recur_kernel:                     ; @scanx_recur_kernel
; %bb.0:
	s_load_b32 s3, s[0:1], 0x18
	v_or_b32_e32 v0, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s3, 0
	s_cselect_b32 s2, -1, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s4, s2
	s_cbranch_execz .LBB8_3
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_mov_b32_e32 v0, 0
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, 0
	.p2align	6
.LBB8_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt lgkmcnt(0)
	s_clause 0x1
	global_load_b64 v[3:4], v2, s[4:5]
	global_load_b64 v[5:6], v2, s[6:7]
	s_add_i32 s3, s3, -1
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
	s_add_u32 s6, s6, 8
	s_addc_u32 s7, s7, 0
	s_waitcnt vmcnt(0)
	v_fma_f64 v[0:1], v[0:1], v[3:4], v[5:6]
	global_store_b64 v2, v[0:1], s[0:1]
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_cmp_lg_u32 s3, 0
	s_cbranch_scc1 .LBB8_2
.LBB8_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scanx_recur_kernel
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
		.amdhsa_next_free_vgpr 7
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
	.size	scanx_recur_kernel, .Lfunc_end8-scanx_recur_kernel
                                        ; -- End function
	.set scanx_recur_kernel.num_vgpr, 7
	.set scanx_recur_kernel.num_agpr, 0
	.set scanx_recur_kernel.numbered_sgpr, 8
	.set scanx_recur_kernel.num_named_barrier, 0
	.set scanx_recur_kernel.private_seg_size, 0
	.set scanx_recur_kernel.uses_vcc, 1
	.set scanx_recur_kernel.uses_flat_scratch, 0
	.set scanx_recur_kernel.has_dyn_sized_stack, 0
	.set scanx_recur_kernel.has_recursion, 0
	.set scanx_recur_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 164
; TotalNumSgprs: 10
; NumVgprs: 7
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_91a00240aebb83a4,@object ; @__hip_cuid_91a00240aebb83a4
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_91a00240aebb83a4
__hip_cuid_91a00240aebb83a4:
	.byte	0                               ; 0x0
	.size	__hip_cuid_91a00240aebb83a4, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_91a00240aebb83a4
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 20
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           scanx_cummin_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         scanx_cummin_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 20
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           scanx_cummax_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         scanx_cummax_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 20
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           scanx_cumsum_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         scanx_cumsum_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 20
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           scanx_cumprod_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         scanx_cumprod_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 20
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           scanx_excl_sum_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         scanx_excl_sum_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 20
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           scanx_excl_prod_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         scanx_excl_prod_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 20
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           scanx_logcumsumexp_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     56
    .sgpr_spill_count: 0
    .symbol:         scanx_logcumsumexp_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     23
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 20
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           scanx_assoc_add_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         scanx_assoc_add_kernel.kd
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
    .name:           scanx_recur_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         scanx_recur_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     7
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
