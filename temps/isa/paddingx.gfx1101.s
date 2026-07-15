	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	paddingx_constant1d_kernel ; -- Begin function paddingx_constant1d_kernel
	.globl	paddingx_constant1d_kernel
	.p2align	8
	.type	paddingx_constant1d_kernel,@function
paddingx_constant1d_kernel:             ; @paddingx_constant1d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[8:11], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s10, v1
	s_cbranch_execz .LBB0_4
; %bb.1:
	s_clause 0x1
	s_load_b64 s[2:3], s[0:1], 0x20
	s_load_b128 s[4:7], s[0:1], 0x0
	v_subrev_nc_u32_e32 v2, s9, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_lt_i32_e32 vcc_lo, -1, v2
	v_cmp_gt_i32_e64 s0, s8, v2
	s_and_b32 s1, vcc_lo, s0
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_and_saveexec_b32 s0, s1
	s_cbranch_execz .LBB0_3
; %bb.2:
	v_mov_b32_e32 v3, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	v_add_co_u32 v2, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
.LBB0_3:
	s_or_b32 exec_lo, exec_lo, s0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB0_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel paddingx_constant1d_kernel
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
.Lfunc_end0:
	.size	paddingx_constant1d_kernel, .Lfunc_end0-paddingx_constant1d_kernel
                                        ; -- End function
	.set paddingx_constant1d_kernel.num_vgpr, 5
	.set paddingx_constant1d_kernel.num_agpr, 0
	.set paddingx_constant1d_kernel.numbered_sgpr, 12
	.set paddingx_constant1d_kernel.num_named_barrier, 0
	.set paddingx_constant1d_kernel.private_seg_size, 0
	.set paddingx_constant1d_kernel.uses_vcc, 1
	.set paddingx_constant1d_kernel.uses_flat_scratch, 0
	.set paddingx_constant1d_kernel.has_dyn_sized_stack, 0
	.set paddingx_constant1d_kernel.has_recursion, 0
	.set paddingx_constant1d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 224
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
	.protected	paddingx_reflect1d_kernel ; -- Begin function paddingx_reflect1d_kernel
	.globl	paddingx_reflect1d_kernel
	.p2align	8
	.type	paddingx_reflect1d_kernel,@function
paddingx_reflect1d_kernel:              ; @paddingx_reflect1d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s6, v1
	s_cbranch_execz .LBB1_4
; %bb.1:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_cmp_eq_u32 s4, 1
	s_cbranch_scc1 .LBB1_3
; %bb.2:
	s_lshl_b32 s2, s4, 1
	v_subrev_nc_u32_e32 v3, s5, v1
	s_add_i32 s2, s2, -2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_abs_i32 s3, s2
	v_sub_nc_u32_e32 v4, 0, v3
	v_cvt_f32_u32_e32 v0, s3
	s_sub_i32 s6, 0, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v4, v3, v4
	v_rcp_iflag_f32_e32 v0, v0
	v_ashrrev_i32_e32 v3, 31, v3
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v4, v0
	v_mul_lo_u32 v0, v0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v4, v0
	v_subrev_nc_u32_e32 v2, s3, v0
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
	v_subrev_nc_u32_e32 v2, s3, v0
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
	v_xor_b32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v3
	v_ashrrev_i32_e32 v2, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b32_e32 v2, s2, v2
	v_add_nc_u32_e32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, s2, v0
	v_cmp_gt_i32_e32 vcc_lo, s4, v0
	v_cndmask_b32_e32 v2, v2, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
.LBB1_3:
	s_load_b128 s[0:3], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB1_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel paddingx_reflect1d_kernel
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
	.size	paddingx_reflect1d_kernel, .Lfunc_end1-paddingx_reflect1d_kernel
                                        ; -- End function
	.set paddingx_reflect1d_kernel.num_vgpr, 5
	.set paddingx_reflect1d_kernel.num_agpr, 0
	.set paddingx_reflect1d_kernel.numbered_sgpr, 8
	.set paddingx_reflect1d_kernel.num_named_barrier, 0
	.set paddingx_reflect1d_kernel.private_seg_size, 0
	.set paddingx_reflect1d_kernel.uses_vcc, 1
	.set paddingx_reflect1d_kernel.uses_flat_scratch, 0
	.set paddingx_reflect1d_kernel.has_dyn_sized_stack, 0
	.set paddingx_reflect1d_kernel.has_recursion, 0
	.set paddingx_reflect1d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 380
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
	.protected	paddingx_replicate1d_kernel ; -- Begin function paddingx_replicate1d_kernel
	.globl	paddingx_replicate1d_kernel
	.p2align	8
	.type	paddingx_replicate1d_kernel,@function
paddingx_replicate1d_kernel:            ; @paddingx_replicate1d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s6, v1
	s_cbranch_execz .LBB2_2
; %bb.1:
	v_subrev_nc_u32_e32 v0, s5, v1
	s_add_i32 s2, s4, -1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_min_i32_e32 v2, s2, v0
	v_cmp_lt_i32_e32 vcc_lo, -1, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel paddingx_replicate1d_kernel
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
.Lfunc_end2:
	.size	paddingx_replicate1d_kernel, .Lfunc_end2-paddingx_replicate1d_kernel
                                        ; -- End function
	.set paddingx_replicate1d_kernel.num_vgpr, 5
	.set paddingx_replicate1d_kernel.num_agpr, 0
	.set paddingx_replicate1d_kernel.numbered_sgpr, 8
	.set paddingx_replicate1d_kernel.num_named_barrier, 0
	.set paddingx_replicate1d_kernel.private_seg_size, 0
	.set paddingx_replicate1d_kernel.uses_vcc, 1
	.set paddingx_replicate1d_kernel.uses_flat_scratch, 0
	.set paddingx_replicate1d_kernel.has_dyn_sized_stack, 0
	.set paddingx_replicate1d_kernel.has_recursion, 0
	.set paddingx_replicate1d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 192
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
	.protected	paddingx_circular1d_kernel ; -- Begin function paddingx_circular1d_kernel
	.globl	paddingx_circular1d_kernel
	.p2align	8
	.type	paddingx_circular1d_kernel,@function
paddingx_circular1d_kernel:             ; @paddingx_circular1d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s6, v1
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_abs_i32 s2, s4
	v_subrev_nc_u32_e32 v3, s5, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v4, 0, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_max_i32_e32 v4, v3, v4
	v_ashrrev_i32_e32 v3, 31, v3
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, s3, v0
	v_mul_hi_u32 v2, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v2
	v_mul_hi_u32 v0, v4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v0, s2
	v_sub_nc_u32_e32 v0, v4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v2, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v2, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v0
	v_and_b32_e32 v2, s4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v2, v2, v0
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel paddingx_circular1d_kernel
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
	.size	paddingx_circular1d_kernel, .Lfunc_end3-paddingx_circular1d_kernel
                                        ; -- End function
	.set paddingx_circular1d_kernel.num_vgpr, 5
	.set paddingx_circular1d_kernel.num_agpr, 0
	.set paddingx_circular1d_kernel.numbered_sgpr, 8
	.set paddingx_circular1d_kernel.num_named_barrier, 0
	.set paddingx_circular1d_kernel.private_seg_size, 0
	.set paddingx_circular1d_kernel.uses_vcc, 1
	.set paddingx_circular1d_kernel.uses_flat_scratch, 0
	.set paddingx_circular1d_kernel.has_dyn_sized_stack, 0
	.set paddingx_circular1d_kernel.has_recursion, 0
	.set paddingx_circular1d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 336
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
	.protected	paddingx_constant2d_kernel ; -- Begin function paddingx_constant2d_kernel
	.globl	paddingx_constant2d_kernel
	.p2align	8
	.type	paddingx_constant2d_kernel,@function
paddingx_constant2d_kernel:             ; @paddingx_constant2d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b64 s[12:13], s[0:1], 0x24
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s13, v1
	s_cbranch_execz .LBB4_6
; %bb.1:
	s_abs_i32 s2, s12
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s12, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_load_b64 s[2:3], s[0:1], 0x30
	v_add_nc_u32_e32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_waitcnt lgkmcnt(0)
	v_mov_b32_e32 v4, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v2, s10, v0
	v_mov_b32_e32 v3, s2
	s_mov_b32 s10, exec_lo
	v_cmpx_lt_i32_e32 -1, v2
	s_cbranch_execz .LBB4_5
; %bb.2:
	v_mul_lo_u32 v0, v0, s12
	v_cmp_gt_i32_e64 s1, s8, v2
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v1, v0
	v_subrev_nc_u32_e32 v0, s11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_lt_i32_e32 vcc_lo, -1, v0
	v_cmp_gt_i32_e64 s0, s9, v0
	s_and_b32 s0, vcc_lo, s0
	s_and_b32 s1, s1, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s0, s1
	s_cbranch_execz .LBB4_4
; %bb.3:
	v_mad_u64_u32 v[3:4], null, v2, s9, v[0:1]
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[3:4]
	v_add_co_u32 v2, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
.LBB4_4:
	s_or_b32 exec_lo, exec_lo, s0
.LBB4_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s10
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB4_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel paddingx_constant2d_kernel
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
.Lfunc_end4:
	.size	paddingx_constant2d_kernel, .Lfunc_end4-paddingx_constant2d_kernel
                                        ; -- End function
	.set paddingx_constant2d_kernel.num_vgpr, 5
	.set paddingx_constant2d_kernel.num_agpr, 0
	.set paddingx_constant2d_kernel.numbered_sgpr, 14
	.set paddingx_constant2d_kernel.num_named_barrier, 0
	.set paddingx_constant2d_kernel.private_seg_size, 0
	.set paddingx_constant2d_kernel.uses_vcc, 1
	.set paddingx_constant2d_kernel.uses_flat_scratch, 0
	.set paddingx_constant2d_kernel.has_dyn_sized_stack, 0
	.set paddingx_constant2d_kernel.has_recursion, 0
	.set paddingx_constant2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 456
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
	.protected	paddingx_reflect2d_kernel ; -- Begin function paddingx_reflect2d_kernel
	.globl	paddingx_reflect2d_kernel
	.p2align	8
	.type	paddingx_reflect2d_kernel,@function
paddingx_reflect2d_kernel:              ; @paddingx_reflect2d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[8:9], s[0:1], 0x24
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s9, v1
	s_cbranch_execz .LBB5_6
; %bb.1:
	s_abs_i32 s2, s8
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b128 s[4:7], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u32 s4, 1
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v2, v0, v3
	v_mov_b32_e32 v0, 0
	v_sub_nc_u32_e32 v2, v2, v3
	v_mov_b32_e32 v3, 0
	s_cbranch_scc1 .LBB5_3
; %bb.2:
	s_lshl_b32 s2, s4, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_subrev_nc_u32_e32 v5, s6, v2
	s_add_i32 s2, s2, -2
	s_abs_i32 s3, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v6, 0, v5
	v_cvt_f32_u32_e32 v3, s3
	s_sub_i32 s9, 0, s3
	v_max_i32_e32 v6, v5, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v3, v3
	v_ashrrev_i32_e32 v5, 31, v5
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v3, 0x4f7ffffe, v3
	v_cvt_u32_f32_e32 v3, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, s9, v3
	v_mul_hi_u32 v4, v3, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v3, v3, v4
	v_mul_hi_u32 v3, v6, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v3, v3, s3
	v_sub_nc_u32_e32 v3, v6, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v4, s3, v3
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v4, s3, v3
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v3, v3, v5
	v_sub_nc_u32_e32 v3, v3, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_and_b32_e32 v4, s2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v3, v4, v3
	v_sub_nc_u32_e32 v4, s2, v3
	v_cmp_gt_i32_e32 vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v3, v4, v3, vcc_lo
.LBB5_3:
	s_cmp_eq_u32 s5, 1
	s_cbranch_scc1 .LBB5_5
; %bb.4:
	s_lshl_b32 s2, s5, 1
	v_mul_lo_u32 v2, v2, s8
	s_add_i32 s2, s2, -2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_abs_i32 s3, s2
	v_cvt_f32_u32_e32 v0, s3
	s_sub_i32 s4, 0, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v1, v2
	v_rcp_iflag_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v2, s7, v2
	v_sub_nc_u32_e32 v5, 0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_max_i32_e32 v5, v2, v5
	v_ashrrev_i32_e32 v2, 31, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v4, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v4, v0, v4
	v_add_nc_u32_e32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v5, v0
	v_mul_lo_u32 v0, v0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v5, v0
	v_subrev_nc_u32_e32 v4, s3, v0
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_subrev_nc_u32_e32 v4, s3, v0
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_xor_b32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v2
	v_ashrrev_i32_e32 v2, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b32_e32 v2, s2, v2
	v_add_nc_u32_e32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, s2, v0
	v_cmp_gt_i32_e32 vcc_lo, s5, v0
	v_cndmask_b32_e32 v0, v2, v0, vcc_lo
.LBB5_5:
	s_load_b128 s[0:3], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[4:5], null, v3, s5, v[0:1]
	v_ashrrev_i32_e32 v5, 31, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB5_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel paddingx_reflect2d_kernel
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
		.amdhsa_next_free_vgpr 7
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
		.amdhsa_inst_pref_size 7
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
	.size	paddingx_reflect2d_kernel, .Lfunc_end5-paddingx_reflect2d_kernel
                                        ; -- End function
	.set paddingx_reflect2d_kernel.num_vgpr, 7
	.set paddingx_reflect2d_kernel.num_agpr, 0
	.set paddingx_reflect2d_kernel.numbered_sgpr, 10
	.set paddingx_reflect2d_kernel.num_named_barrier, 0
	.set paddingx_reflect2d_kernel.private_seg_size, 0
	.set paddingx_reflect2d_kernel.uses_vcc, 1
	.set paddingx_reflect2d_kernel.uses_flat_scratch, 0
	.set paddingx_reflect2d_kernel.has_dyn_sized_stack, 0
	.set paddingx_reflect2d_kernel.has_recursion, 0
	.set paddingx_reflect2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 788
; TotalNumSgprs: 12
; NumVgprs: 7
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 12
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
	.protected	paddingx_replicate2d_kernel ; -- Begin function paddingx_replicate2d_kernel
	.globl	paddingx_replicate2d_kernel
	.p2align	8
	.type	paddingx_replicate2d_kernel,@function
paddingx_replicate2d_kernel:            ; @paddingx_replicate2d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[4:5], s[0:1], 0x24
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s5, v1
	s_cbranch_execz .LBB6_2
; %bb.1:
	s_abs_i32 s2, s4
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b256 s[8:15], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	s_waitcnt lgkmcnt(0)
	s_add_i32 s0, s12, -1
	s_add_i32 s1, s13, -1
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
	v_xor_b32_e32 v3, s4, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s4
	v_subrev_nc_u32_e32 v0, s14, v0
	v_min_i32_e32 v3, s0, v0
	v_cmp_lt_i32_e32 vcc_lo, -1, v0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v2, v1, v2
	v_cndmask_b32_e32 v5, 0, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v2, s15, v2
	v_min_i32_e32 v4, s1, v2
	v_cmp_lt_i32_e32 vcc_lo, -1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, 0, v4, vcc_lo
	v_mad_u64_u32 v[2:3], null, v5, s13, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s8, v2
	v_add_co_ci_u32_e64 v3, null, s9, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s10, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel paddingx_replicate2d_kernel
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
		.amdhsa_next_free_vgpr 6
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
.Lfunc_end6:
	.size	paddingx_replicate2d_kernel, .Lfunc_end6-paddingx_replicate2d_kernel
                                        ; -- End function
	.set paddingx_replicate2d_kernel.num_vgpr, 6
	.set paddingx_replicate2d_kernel.num_agpr, 0
	.set paddingx_replicate2d_kernel.numbered_sgpr, 16
	.set paddingx_replicate2d_kernel.num_named_barrier, 0
	.set paddingx_replicate2d_kernel.private_seg_size, 0
	.set paddingx_replicate2d_kernel.uses_vcc, 1
	.set paddingx_replicate2d_kernel.uses_flat_scratch, 0
	.set paddingx_replicate2d_kernel.has_dyn_sized_stack, 0
	.set paddingx_replicate2d_kernel.has_recursion, 0
	.set paddingx_replicate2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 408
; TotalNumSgprs: 18
; NumVgprs: 6
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 18
; NumVGPRsForWavesPerEU: 6
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
	.protected	paddingx_circular2d_kernel ; -- Begin function paddingx_circular2d_kernel
	.globl	paddingx_circular2d_kernel
	.p2align	8
	.type	paddingx_circular2d_kernel,@function
paddingx_circular2d_kernel:             ; @paddingx_circular2d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[8:9], s[0:1], 0x24
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s9, v1
	s_cbranch_execz .LBB7_2
; %bb.1:
	s_abs_i32 s9, s8
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s9
	s_sub_i32 s2, 0, s9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s2, v0
	s_load_b256 s[0:7], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s10, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_mul_hi_u32 v0, v3, v0
	v_cvt_f32_u32_e32 v4, s10
	s_abs_i32 s11, s5
	v_cvt_f32_u32_e32 v6, s11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rcp_iflag_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v6, v6
	v_sub_nc_u32_e32 v2, v3, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_subrev_nc_u32_e32 v5, s9, v2
	v_cmp_le_u32_e32 vcc_lo, s9, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s8, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s9, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cvt_u32_f32_e32 v2, v4
	s_sub_i32 s9, 0, s10
	v_dual_cndmask_b32 v0, v0, v5 :: v_dual_mul_f32 v5, 0x4f7ffffe, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v4, s9, v2
	v_xor_b32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v4, v2, v4
	v_sub_nc_u32_e32 v0, v0, v3
	v_cvt_u32_f32_e32 v3, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s6, v0
	v_mul_lo_u32 v0, v0, s8
	s_sub_i32 s6, 0, s11
	v_add_nc_u32_e32 v2, v2, v4
	v_mul_lo_u32 v6, s6, v3
	v_sub_nc_u32_e32 v7, 0, v5
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v1, v0
	v_max_i32_e32 v4, v5, v7
	v_ashrrev_i32_e32 v5, 31, v5
	v_mul_hi_u32 v6, v3, v6
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_subrev_nc_u32_e32 v0, s7, v0
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_sub_nc_u32_e32 v7, 0, v0
	v_add_nc_u32_e32 v3, v3, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v2, v2, s10
	v_max_i32_e32 v6, v0, v7
	v_ashrrev_i32_e32 v0, 31, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_hi_u32 v3, v6, v3
	v_sub_nc_u32_e32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_subrev_nc_u32_e32 v4, s10, v2
	v_cmp_le_u32_e32 vcc_lo, s10, v2
	v_mul_lo_u32 v3, v3, s11
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_sub_nc_u32_e32 v3, v6, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_subrev_nc_u32_e32 v4, s10, v2
	v_cmp_le_u32_e32 vcc_lo, s10, v2
	v_subrev_nc_u32_e32 v6, s11, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s11, v3
	v_xor_b32_e32 v2, v2, v5
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, v3, v6, vcc_lo
	v_sub_nc_u32_e32 v2, v2, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v4, s11, v3
	v_cmp_le_u32_e32 vcc_lo, s11, v3
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v4, 31, v2
	v_xor_b32_e32 v3, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_and_b32_e32 v4, s4, v4
	v_sub_nc_u32_e32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v2, v4, v2
	v_ashrrev_i32_e32 v3, 31, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v2, v2, s5
	v_and_b32_e32 v3, s5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add3_u32 v2, v3, v0, v2
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	v_add_co_u32 v2, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	global_load_b64 v[3:4], v[2:3], off
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB7_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel paddingx_circular2d_kernel
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
.Lfunc_end7:
	.size	paddingx_circular2d_kernel, .Lfunc_end7-paddingx_circular2d_kernel
                                        ; -- End function
	.set paddingx_circular2d_kernel.num_vgpr, 8
	.set paddingx_circular2d_kernel.num_agpr, 0
	.set paddingx_circular2d_kernel.numbered_sgpr, 12
	.set paddingx_circular2d_kernel.num_named_barrier, 0
	.set paddingx_circular2d_kernel.private_seg_size, 0
	.set paddingx_circular2d_kernel.uses_vcc, 1
	.set paddingx_circular2d_kernel.uses_flat_scratch, 0
	.set paddingx_circular2d_kernel.has_dyn_sized_stack, 0
	.set paddingx_circular2d_kernel.has_recursion, 0
	.set paddingx_circular2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 688
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_986494e33675af0b,@object ; @__hip_cuid_986494e33675af0b
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_986494e33675af0b
__hip_cuid_986494e33675af0b:
	.byte	0                               ; 0x0
	.size	__hip_cuid_986494e33675af0b, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_986494e33675af0b
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
    .name:           paddingx_constant1d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         paddingx_constant1d_kernel.kd
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
    .name:           paddingx_reflect1d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         paddingx_reflect1d_kernel.kd
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
    .name:           paddingx_replicate1d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         paddingx_replicate1d_kernel.kd
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
    .name:           paddingx_circular1d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         paddingx_circular1d_kernel.kd
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
        .size:           4
        .value_kind:     by_value
      - .offset:         36
        .size:           4
        .value_kind:     by_value
      - .offset:         40
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           8
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
    .name:           paddingx_constant2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         paddingx_constant2d_kernel.kd
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
    .name:           paddingx_reflect2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         paddingx_reflect2d_kernel.kd
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
    .name:           paddingx_replicate2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         paddingx_replicate2d_kernel.kd
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
    .name:           paddingx_circular2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         paddingx_circular2d_kernel.kd
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
