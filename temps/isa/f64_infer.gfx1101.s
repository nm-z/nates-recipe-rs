	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	widen_bf16_f64          ; -- Begin function widen_bf16_f64
	.globl	widen_bf16_f64
	.p2align	8
	.type	widen_bf16_f64,@function
widen_bf16_f64:                         ; @widen_bf16_f64
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i64_e64 s[4:5], v[2:3]
	s_cbranch_execz .LBB0_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_lshlrev_b64 v[0:1], 1, v[2:3]
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	global_load_u16 v0, v[0:1], off
	s_waitcnt vmcnt(0)
	v_lshlrev_b32_e32 v0, 16, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[0:1], v0
	global_store_b64 v[2:3], v[0:1], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel widen_bf16_f64
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
		.amdhsa_next_free_vgpr 4
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
	.size	widen_bf16_f64, .Lfunc_end0-widen_bf16_f64
                                        ; -- End function
	.set widen_bf16_f64.num_vgpr, 4
	.set widen_bf16_f64.num_agpr, 0
	.set widen_bf16_f64.numbered_sgpr, 6
	.set widen_bf16_f64.num_named_barrier, 0
	.set widen_bf16_f64.private_seg_size, 0
	.set widen_bf16_f64.uses_vcc, 1
	.set widen_bf16_f64.uses_flat_scratch, 0
	.set widen_bf16_f64.has_dyn_sized_stack, 0
	.set widen_bf16_f64.has_recursion, 0
	.set widen_bf16_f64.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 168
; TotalNumSgprs: 8
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
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
	.protected	widen_bf16_f32          ; -- Begin function widen_bf16_f32
	.globl	widen_bf16_f32
	.p2align	8
	.type	widen_bf16_f32,@function
widen_bf16_f32:                         ; @widen_bf16_f32
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i64_e64 s[4:5], v[2:3]
	s_cbranch_execz .LBB1_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_lshlrev_b64 v[0:1], 1, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_u16 v4, v[0:1], off
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_lshlrev_b32_e32 v2, 16, v4
	global_store_b32 v[0:1], v2, off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel widen_bf16_f32
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
	.size	widen_bf16_f32, .Lfunc_end1-widen_bf16_f32
                                        ; -- End function
	.set widen_bf16_f32.num_vgpr, 5
	.set widen_bf16_f32.num_agpr, 0
	.set widen_bf16_f32.numbered_sgpr, 6
	.set widen_bf16_f32.num_named_barrier, 0
	.set widen_bf16_f32.private_seg_size, 0
	.set widen_bf16_f32.uses_vcc, 1
	.set widen_bf16_f32.uses_flat_scratch, 0
	.set widen_bf16_f32.has_dyn_sized_stack, 0
	.set widen_bf16_f32.has_recursion, 0
	.set widen_bf16_f32.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 160
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
	.protected	widen_bf16_f64_scaled   ; -- Begin function widen_bf16_f64_scaled
	.globl	widen_bf16_f64_scaled
	.p2align	8
	.type	widen_bf16_f64_scaled,@function
widen_bf16_f64_scaled:                  ; @widen_bf16_f64_scaled
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b256 s[4:11], s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s0, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s0, s2, v[0:1]
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_i64_e64 s[8:9], v[2:3]
	s_cbranch_execz .LBB2_2
; %bb.1:
	v_lshlrev_b64 v[0:1], 1, v[2:3]
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s6, v2
	v_add_co_ci_u32_e64 v3, null, s7, v3, vcc_lo
	global_load_u16 v0, v[0:1], off
	s_waitcnt vmcnt(0)
	v_lshlrev_b32_e32 v0, 16, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[0:1], v0
	v_mul_f64 v[0:1], s[10:11], v[0:1]
	global_store_b64 v[2:3], v[0:1], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel widen_bf16_f64_scaled
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
		.amdhsa_next_free_vgpr 4
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
	.size	widen_bf16_f64_scaled, .Lfunc_end2-widen_bf16_f64_scaled
                                        ; -- End function
	.set widen_bf16_f64_scaled.num_vgpr, 4
	.set widen_bf16_f64_scaled.num_agpr, 0
	.set widen_bf16_f64_scaled.numbered_sgpr, 12
	.set widen_bf16_f64_scaled.num_named_barrier, 0
	.set widen_bf16_f64_scaled.private_seg_size, 0
	.set widen_bf16_f64_scaled.uses_vcc, 1
	.set widen_bf16_f64_scaled.uses_flat_scratch, 0
	.set widen_bf16_f64_scaled.has_dyn_sized_stack, 0
	.set widen_bf16_f64_scaled.has_recursion, 0
	.set widen_bf16_f64_scaled.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 164
; TotalNumSgprs: 14
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 14
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
	.protected	widen_bf16_f32_scaled   ; -- Begin function widen_bf16_f32_scaled
	.globl	widen_bf16_f32_scaled
	.p2align	8
	.type	widen_bf16_f32_scaled,@function
widen_bf16_f32_scaled:                  ; @widen_bf16_f32_scaled
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b256 s[4:11], s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s0, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s0, s2, v[0:1]
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_i64_e64 s[8:9], v[2:3]
	s_cbranch_execz .LBB3_2
; %bb.1:
	v_lshlrev_b64 v[0:1], 1, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	global_load_u16 v0, v[0:1], off
	s_waitcnt vmcnt(0)
	v_lshlrev_b32_e32 v0, 16, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[0:1], v0
	v_mul_f64 v[0:1], s[10:11], v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_f32_f64_e32 v4, v[0:1]
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b32 v[0:1], v4, off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel widen_bf16_f32_scaled
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
.Lfunc_end3:
	.size	widen_bf16_f32_scaled, .Lfunc_end3-widen_bf16_f32_scaled
                                        ; -- End function
	.set widen_bf16_f32_scaled.num_vgpr, 5
	.set widen_bf16_f32_scaled.num_agpr, 0
	.set widen_bf16_f32_scaled.numbered_sgpr, 12
	.set widen_bf16_f32_scaled.num_named_barrier, 0
	.set widen_bf16_f32_scaled.private_seg_size, 0
	.set widen_bf16_f32_scaled.uses_vcc, 1
	.set widen_bf16_f32_scaled.uses_flat_scratch, 0
	.set widen_bf16_f32_scaled.has_dyn_sized_stack, 0
	.set widen_bf16_f32_scaled.has_recursion, 0
	.set widen_bf16_f32_scaled.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 172
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
	.section	.text._Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,"axG",@progbits,_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,comdat
	.protected	_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii ; -- Begin function _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
	.globl	_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
	.p2align	8
	.type	_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,@function
_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii: ; @_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
; %bb.0:
	s_clause 0x1
	s_load_b128 s[24:27], s[0:1], 0x24
	s_load_b64 s[6:7], s[0:1], 0x38
	s_abs_i32 s8, s2
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s4, s25
	s_ashr_i32 s9, s25, 31
	v_cvt_f32_u32_e32 v1, s4
	s_sub_i32 s5, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v1
	s_mul_i32 s5, s5, s3
	s_mul_hi_u32 s5, s3, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s3, s5
	s_ashr_i32 s3, s2, 31
	s_mul_hi_u32 s5, s8, s5
	s_xor_b32 s11, s3, s9
	s_mul_i32 s10, s5, s4
	s_sub_i32 s8, s8, s10
	s_add_i32 s10, s5, 1
	s_sub_i32 s12, s8, s4
	s_cmp_ge_u32 s8, s4
	s_cselect_b32 s5, s10, s5
	s_cselect_b32 s8, s12, s8
	s_add_i32 s10, s5, 1
	s_cmp_ge_u32 s8, s4
	s_cselect_b32 s5, s10, s5
	s_abs_i32 s8, s26
	s_xor_b32 s5, s5, s11
	v_cvt_f32_u32_e32 v1, s8
	s_sub_i32 s12, 0, s8
	s_sub_i32 s33, s5, s11
	s_ashr_i32 s13, s26, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_xor_b32 s9, s9, s13
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s10, v1
	s_mul_i32 s12, s12, s10
	s_mul_hi_u32 s12, s10, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s10, s10, s12
	s_mul_hi_u32 s5, s4, s10
	s_mul_i32 s10, s33, s25
	s_mul_i32 s11, s5, s8
	s_sub_i32 s48, s2, s10
	s_sub_i32 s4, s4, s11
	s_add_i32 s10, s5, 1
	s_sub_i32 s11, s4, s8
	s_cmp_ge_u32 s4, s8
	s_cselect_b32 s5, s10, s5
	s_cselect_b32 s4, s11, s4
	s_add_i32 s10, s5, 1
	s_cmp_ge_u32 s4, s8
	s_cselect_b32 s4, s10, s5
	v_cmp_ngt_f64_e64 s5, s[6:7], 0
	s_xor_b32 s4, s4, s9
	s_abs_i32 s47, s48
	s_sub_i32 s49, s4, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_abs_i32 s46, s49
	v_cvt_f32_u32_e32 v1, s46
	s_sub_i32 s4, 0, s46
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_readfirstlane_b32 s50, v1
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mul_i32 s4, s4, s50
	s_and_b32 vcc_lo, exec_lo, s5
	s_mul_hi_u32 s4, s50, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s50, s50, s4
	s_cbranch_vccnz .LBB4_5
; %bb.1:
	v_cvt_f64_i32_e32 v[1:2], s25
	s_mov_b32 s11, 0x3fe55555
	s_mov_b32 s10, 0x55555555
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_mov_b32 s43, 0x3fd99999
	s_mov_b32 s42, 0x998ef7b6
	s_mov_b32 s12, 0xffda0d24
	s_mov_b32 s13, 0x3c7777d0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[3:4], v[1:2]
	v_cmp_gt_f64_e32 vcc_lo, s[10:11], v[3:4]
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v5
	v_add_f64 v[5:6], v[3:4], 1.0
	v_add_f64 v[11:12], v[3:4], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[7:8], v[5:6]
	v_add_f64 v[13:14], v[5:6], -1.0
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[5:6], v[9:10]
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[9:10], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[11:12], -v[5:6]
	v_add_f64 v[15:16], v[5:6], -v[15:16]
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[15:16], -v[3:4]
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[3:4], v[13:14], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[3:4], v[7:8], v[3:4]
	v_add_f64 v[5:6], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[5:6], v[5:6]
	v_fma_f64 v[11:12], v[7:8], s[8:9], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	s_mov_b32 s8, 0x55555780
	s_mov_b32 s9, s11
	v_mul_f64 v[13:14], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[4:5]
	s_mov_b32 s5, 0x3fd24924
	s_mov_b32 s4, 0x9b27acf1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[42:43]
	v_fma_f64 v[7:8], v[7:8], v[11:12], s[8:9]
	v_ldexp_f64 v[11:12], v[5:6], 1
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_mov_b32 s8, 0x652b82fe
	s_mov_b32 s9, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[7:8], v[13:14], v[7:8]
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[11:12], v[7:8]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], -v[11:12]
	v_add_f64 v[5:6], v[7:8], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], s[8:9]
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], s[8:9], -v[9:10]
	v_fma_f64 v[3:4], v[3:4], s[8:9], v[7:8]
	v_frexp_exp_i32_f64_e32 v7, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[1:2], v[5:6], s[12:13], v[3:4]
	v_subrev_co_ci_u32_e64 v3, null, 0, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[3:4], v3
	v_add_f64 v[5:6], v[9:10], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], v[3:4]
	v_add_f64 v[9:10], v[5:6], -v[9:10]
	v_add_f64 v[11:12], v[7:8], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[1:2], v[1:2], -v[9:10]
	v_add_f64 v[13:14], v[11:12], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[13:14], v[3:4]
	v_add_f64 v[3:4], v[5:6], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_f64 v[1:2], v[7:8], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_floor_f64_e32 v[1:2], v[1:2]
	v_cvt_i32_f64_e32 v1, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s4, v1
	s_lshl_b32 s20, 1, s4
	v_cvt_f64_i32_e32 v[3:4], s20
	s_cmp_ge_i32 s48, s20
	s_cbranch_scc0 .LBB4_3
; %bb.2:
	v_mul_f64 v[1:2], s[6:7], -0.5
	s_mov_b32 s13, 0x3c7abc9e
	s_mov_b32 s12, 0x3b39803f
	s_mov_b32 s17, 0x3fe62e42
	s_mov_b32 s16, 0xfefa39ef
	s_mov_b32 s14, 0xfca7ab0c
	s_mov_b32 s18, 0x6a5dcb37
	s_mov_b32 s15, 0x3e928af3
	s_mov_b32 s19, 0x3e5ade15
	s_mov_b32 s22, 0x623fde64
	s_mov_b32 s23, 0x3ec71dee
	s_mov_b32 s28, 0x7c89e6b0
	s_mov_b32 s29, 0x3efa0199
	s_mov_b32 s30, 0x14761f6e
	s_mov_b32 s31, 0x3f2a01a0
	s_mov_b32 s34, 0x1852b7b0
	s_mov_b32 s35, 0x3f56c16c
	s_mov_b32 s36, 0x11122322
	s_mov_b32 s37, 0x3f811111
	s_mov_b32 s38, 0x555502a1
	s_mov_b32 s39, 0x3fa55555
	s_mov_b32 s40, 0x55555511
	s_mov_b32 s41, 0x3fc55555
	s_mov_b32 s44, 11
	s_mov_b32 s45, 0x3fe00000
	s_mov_b32 s54, 0x4222de17
	s_mov_b32 s55, 0x3fbdee67
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], v[1:2]
	v_div_scale_f64 v[11:12], vcc_lo, v[1:2], v[3:4], v[1:2]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[1:2], v[5:6], v[3:4], v[1:2]
	v_rndne_f64_e32 v[5:6], v[1:2]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[1:2]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[1:2], -v[5:6]
	v_cvt_i32_f64_e32 v11, v[5:6]
	s_and_b32 s42, vcc_lo, exec_lo
	v_mul_f64 v[9:10], v[7:8], s[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], s[16:17], v[9:10]
	v_fma_f64 v[9:10], v[7:8], s[18:19], s[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[22:23]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[30:31]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[36:37]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[40:41]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[44:45]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], 1.0
	v_fma_f64 v[5:6], v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v11
	v_readfirstlane_b32 s21, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s42, v5
	s_cselect_b32 s21, s21, 0x7ff00000
	s_and_b32 s51, s4, vcc_lo
	s_and_b32 s51, s51, exec_lo
	s_cselect_b32 s52, s42, 0
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s53, s21, 0
	s_sub_i32 s4, s48, s20
	v_cmp_neq_f64_e64 vcc_lo, s[52:53], 1.0
	s_lshl_b32 s4, s4, 1
	s_mov_b32 s42, 0x9999999c
	s_or_b32 s4, s4, 1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_u32_e32 v[1:2], s4
	v_cndmask_b32_e32 v2, 0x3ff00000, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, 0, v1, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[1:2]
	s_and_b32 s4, vcc_lo, exec_lo
	s_cselect_b32 s21, s53, 0x3ff00000
	s_cselect_b32 s20, s52, 0
	s_mov_b32 s52, 0x968915a9
	v_frexp_mant_f64_e64 v[5:6], |s[20:21]|
	s_mov_b32 s53, 0x3fba6564
	s_mov_b32 s4, 0x924920da
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[10:11], v[5:6]
	v_cndmask_b32_e64 v7, 0, 1, vcc_lo
	v_ldexp_f64 v[5:6], v[5:6], v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], 1.0
	v_add_f64 v[13:14], v[5:6], -1.0
	v_rcp_f64_e32 v[9:10], v[7:8]
	v_add_f64 v[15:16], v[7:8], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	v_mul_f64 v[17:18], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[11:12], v[7:8], -v[17:18]
	v_fma_f64 v[5:6], v[11:12], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[17:18], v[5:6]
	v_add_f64 v[15:16], v[13:14], -v[7:8]
	v_add_f64 v[17:18], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_add_f64 v[5:6], v[17:18], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[5:6]
	v_mul_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_add_f64 v[9:10], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_fma_f64 v[9:10], v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[5:6], v[5:6]
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[54:55], s[52:53]
	s_mov_b32 s52, 0x3abe935a
	s_mov_b32 s53, 0x3fbe25e4
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_mul_f64 v[21:22], v[7:8], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x47e6c9c2
	s_mov_b32 s53, 0x3fc110ef
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0xcfa74449
	s_mov_b32 s53, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x71bf3c30
	s_mov_b32 s53, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x1c7792ce
	s_mov_b32 s53, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s4, 0xd5df274d
	s_mov_b32 s5, 0x3c8543b0
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[11:12], v[13:14], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	v_add_f64 v[15:16], v[17:18], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[19:20], v[15:16], s[10:11]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_mov_b32 s11, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[23:24], v[19:20], s[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_fma_f64 v[17:18], v[13:14], v[7:8], -v[21:22]
	v_add_f64 v[15:16], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], s[4:5]
	v_fma_f64 v[13:14], v[13:14], v[5:6], v[17:18]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[7:8], v[13:14]
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[19:20], v[11:12]
	v_add_f64 v[15:16], v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[13:14]
	v_mul_f64 v[19:20], v[15:16], v[13:14]
	v_add_f64 v[21:22], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[17:18]
	v_fma_f64 v[17:18], v[15:16], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[21:22]
	v_fma_f64 v[11:12], v[15:16], v[11:12], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[9:10], v[13:14], v[11:12]
	v_frexp_exp_i32_f64_e32 v13, s[20:21]
	v_add_f64 v[11:12], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v13, null, 0, v13, vcc_lo
	v_cvt_f64_i32_e32 v[13:14], v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[7:8], v[11:12]
	v_add_f64 v[17:18], v[11:12], -v[19:20]
	v_mul_f64 v[19:20], v[13:14], s[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[15:16], -v[7:8]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], s[16:17], -v[19:20]
	s_mov_b32 s17, 0xbfe62e42
	v_add_f64 v[7:8], v[11:12], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_fma_f64 v[9:10], v[13:14], s[12:13], v[17:18]
	s_mov_b32 s13, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], v[5:6]
	v_add_f64 v[19:20], v[7:8], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[7:8], v[11:12]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[15:16], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[21:22]
	v_add_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[15:16], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[11:12]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	v_add_f64 v[17:18], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[11:12], v[17:18], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[17:18], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[7:8], -v[17:18]
	v_mul_f64 v[11:12], v[1:2], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[1:2], v[7:8], -v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	v_fma_f64 v[5:6], v[1:2], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_dual_cndmask_b32 v10, v8, v12 :: v_dual_cndmask_b32 v9, v7, v11
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[1:2], 0.5
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[13:14], v[9:10], s[8:9]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[9:10]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[9:10]
	v_cmp_neq_f64_e64 s5, 0x7ff00000, |v[9:10]|
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_trunc_f64_e32 v[7:8], v[1:2]
	v_rndne_f64_e32 v[13:14], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v6, 0, v6, s5
	v_cndmask_b32_e64 v5, 0, v5, s5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], s[16:17], v[9:10]
	v_cvt_i32_f64_e32 v19, v[13:14]
	v_trunc_f64_e32 v[9:10], v[11:12]
	v_fma_f64 v[15:16], v[13:14], s[12:13], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s5, v[9:10], v[11:12]
	v_fma_f64 v[17:18], v[15:16], s[18:19], s[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[22:23]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[30:31]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[36:37]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[40:41]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[44:45]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[13:14], v[13:14], v19
	v_cndmask_b32_e32 v14, 0x7ff00000, v14, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s8, v13
	v_cndmask_b32_e64 v14, 0, v14, s4
	s_and_b32 s4, s4, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s8, s8, 0
	v_cmp_eq_f64_e64 s4, v[7:8], v[1:2]
	v_mov_b32_e32 v13, s8
	v_fma_f64 v[5:6], v[13:14], v[5:6], v[13:14]
	v_cmp_class_f64_e64 vcc_lo, v[13:14], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s9, v5
	s_and_b32 s10, vcc_lo, exec_lo
	v_cndmask_b32_e32 v1, v6, v14, vcc_lo
	v_cmp_lt_f64_e64 vcc_lo, s[20:21], 0
	v_cmp_eq_f64_e64 s10, s[20:21], 0
	s_cselect_b32 s8, s8, s9
	s_and_b32 s5, s4, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s9, s5, exec_lo
	s_cselect_b32 s9, s21, 0x3ff00000
	v_bfi_b32 v1, 0x7fffffff, v1, s9
	v_cmp_class_f64_e64 s9, s[20:21], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, 0x7ff80000, v1, s4
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, s8, 0
	s_and_b32 s11, vcc_lo, exec_lo
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	s_cselect_b32 s8, s4, s8
	s_or_b32 vcc_lo, s10, s9
	s_and_b32 s4, s10, exec_lo
	s_cselect_b32 s4, 0, 0x7ff00000
	s_and_b32 s5, s5, exec_lo
	s_cselect_b32 s5, s21, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v2, s5
	s_and_b32 s5, vcc_lo, exec_lo
	s_cselect_b32 s5, 0, s8
	v_bfi_b32 v2, 0x7fffffff, s4, v2
	v_cmp_o_f64_e64 s4, s[20:21], s[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	v_cndmask_b32_e64 v2, 0x7ff80000, v1, s4
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, s5, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v1, s4
	s_cbranch_execz .LBB4_4
	s_branch .LBB4_5
.LBB4_3:
                                        ; implicit-def: $vgpr1_vgpr2
.LBB4_4:
	s_delay_alu instid0(VALU_DEP_1)
	v_div_scale_f64 v[1:2], null, v[3:4], v[3:4], -s[6:7]
	v_div_scale_f64 v[9:10], vcc_lo, -s[6:7], v[3:4], -s[6:7]
	s_mov_b32 s13, 0x3fe62e42
	s_mov_b32 s12, 0xfefa39ef
	s_mov_b32 s10, 0xfca7ab0c
	s_mov_b32 s14, 0x6a5dcb37
	s_mov_b32 s11, 0x3e928af3
	s_mov_b32 s15, 0x3e5ade15
	s_mov_b32 s16, 0x623fde64
	s_mov_b32 s17, 0x3ec71dee
	s_mov_b32 s18, 0x7c89e6b0
	s_mov_b32 s19, 0x3efa0199
	s_mov_b32 s20, 0x14761f6e
	s_mov_b32 s21, 0x3f2a01a0
	s_mov_b32 s22, 0x1852b7b0
	s_mov_b32 s23, 0x3f56c16c
	s_mov_b32 s28, 0x11122322
	s_mov_b32 s29, 0x3f811111
	s_mov_b32 s30, 0x555502a1
	s_mov_b32 s31, 0x3fa55555
	s_mov_b32 s34, 0x55555511
	s_mov_b32 s35, 0x3fc55555
	s_mov_b32 s36, 11
	s_mov_b32 s37, 0x3fe00000
	s_mov_b32 s38, 0x968915a9
	s_mov_b32 s40, 0x4222de17
	s_mov_b32 s39, 0x3fba6564
	s_mov_b32 s41, 0x3fbdee67
	v_rcp_f64_e32 v[5:6], v[1:2]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[7:8], -v[1:2], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_fma_f64 v[7:8], -v[1:2], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_mul_f64 v[7:8], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[1:2], -v[1:2], v[7:8], v[9:10]
	v_div_fmas_f64 v[1:2], v[1:2], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[1:2], v[1:2], v[3:4], -s[6:7]
	s_mov_b32 s7, 0x3c7abc9e
	s_mov_b32 s6, 0x3b39803f
	v_rndne_f64_e32 v[3:4], v[1:2]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[1:2]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[1:2], -v[3:4]
	v_cvt_i32_f64_e32 v9, v[3:4]
	s_and_b32 s8, vcc_lo, exec_lo
	v_mul_f64 v[7:8], v[5:6], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], s[12:13], v[7:8]
	v_fma_f64 v[7:8], v[5:6], s[14:15], s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[16:17]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[20:21]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[28:29]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[34:35]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], 1.0
	v_fma_f64 v[3:4], v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v9
	v_readfirstlane_b32 s5, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s8, v3
	s_cselect_b32 s5, s5, 0x7ff00000
	s_and_b32 s9, s4, vcc_lo
	s_and_b32 s9, s9, exec_lo
	s_cselect_b32 s8, s8, 0
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s9, s5, 0
	s_add_i32 s4, s48, 1
	v_cmp_neq_f64_e64 vcc_lo, s[8:9], 1.0
	v_cvt_f64_i32_e32 v[1:2], s4
	s_mov_b32 s5, 0x3fe55555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, 0x3ff00000, v2, vcc_lo
	v_cndmask_b32_e32 v1, 0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[1:2]
	s_and_b32 s4, vcc_lo, exec_lo
	s_cselect_b32 s9, s9, 0x3ff00000
	s_cselect_b32 s8, s8, 0
	s_mov_b32 s4, 0x55555555
	v_frexp_mant_f64_e64 v[3:4], |s[8:9]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[3:4]
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	v_ldexp_f64 v[3:4], v[3:4], v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[3:4], 1.0
	v_add_f64 v[11:12], v[3:4], -1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	v_add_f64 v[13:14], v[5:6], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	v_mul_f64 v[15:16], v[5:6], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[15:16]
	v_fma_f64 v[3:4], v[9:10], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[3:4]
	v_add_f64 v[13:14], v[11:12], -v[5:6]
	v_add_f64 v[15:16], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	v_add_f64 v[3:4], v[15:16], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	v_add_f64 v[3:4], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[13:14], v[3:4]
	v_mul_f64 v[3:4], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[3:4]
	v_add_f64 v[7:8], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	v_fma_f64 v[7:8], v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[3:4], v[3:4]
	v_fma_f64 v[7:8], v[5:6], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[9:10], v[7:8]
	v_fma_f64 v[13:14], v[11:12], s[40:41], s[38:39]
	s_mov_b32 s38, 0x3abe935a
	s_mov_b32 s39, 0x3fbe25e4
	v_add_f64 v[9:10], v[11:12], -v[9:10]
	v_mul_f64 v[19:20], v[5:6], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x47e6c9c2
	s_mov_b32 s39, 0x3fc110ef
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0xcfa74449
	s_mov_b32 s39, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x71bf3c30
	s_mov_b32 s39, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x1c7792ce
	s_mov_b32 s39, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x924920da
	s_mov_b32 s39, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x9999999c
	s_mov_b32 s39, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[11:12], v[13:14]
	v_fma_f64 v[9:10], v[11:12], v[13:14], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	v_add_f64 v[13:14], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[17:18], v[13:14], s[4:5]
	v_add_f64 v[15:16], v[13:14], -v[15:16]
	s_mov_b32 s5, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[21:22], v[17:18], s[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_fma_f64 v[15:16], v[11:12], v[5:6], -v[19:20]
	s_mov_b32 s4, 0xd5df274d
	s_mov_b32 s5, 0x3c8543b0
	v_add_f64 v[13:14], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], s[4:5]
	v_fma_f64 v[11:12], v[11:12], v[3:4], v[15:16]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[13:14]
	v_fma_f64 v[7:8], v[7:8], v[5:6], v[11:12]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[17:18], v[9:10]
	v_add_f64 v[13:14], v[19:20], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[17:18], -v[11:12]
	v_mul_f64 v[17:18], v[13:14], v[11:12]
	v_add_f64 v[19:20], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[15:16]
	v_fma_f64 v[15:16], v[13:14], v[11:12], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[19:20]
	v_fma_f64 v[9:10], v[13:14], v[9:10], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[7:8], v[11:12], v[9:10]
	v_frexp_exp_i32_f64_e32 v11, s[8:9]
	v_add_f64 v[9:10], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	v_cvt_f64_i32_e32 v[11:12], v11
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[5:6], v[9:10]
	v_add_f64 v[15:16], v[9:10], -v[17:18]
	v_mul_f64 v[17:18], v[11:12], s[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[13:14], -v[5:6]
	v_add_f64 v[7:8], v[7:8], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[11:12], s[12:13], -v[17:18]
	s_mov_b32 s13, 0xbfe62e42
	v_add_f64 v[5:6], v[9:10], -v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_fma_f64 v[7:8], v[11:12], s[6:7], v[15:16]
	s_mov_b32 s7, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], v[3:4]
	v_add_f64 v[17:18], v[5:6], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[5:6], v[9:10]
	v_add_f64 v[13:14], v[9:10], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[11:12], -v[5:6]
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[13:14], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[19:20]
	v_add_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[13:14], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[9:10]
	v_add_f64 v[3:4], v[3:4], -v[9:10]
	v_add_f64 v[15:16], v[11:12], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[9:10], v[15:16], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], -v[15:16]
	v_mul_f64 v[9:10], v[1:2], v[5:6]
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[5:6], v[1:2], v[5:6], -v[9:10]
	v_cmp_class_f64_e64 vcc_lo, v[9:10], 0x204
	v_fma_f64 v[3:4], v[1:2], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[3:4]
	v_dual_cndmask_b32 v8, v6, v10 :: v_dual_cndmask_b32 v7, v5, v9
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[1:2], 0.5
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[11:12], v[7:8], s[4:5]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[7:8]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[7:8]
	v_cmp_neq_f64_e64 s5, 0x7ff00000, |v[7:8]|
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	v_trunc_f64_e32 v[5:6], v[1:2]
	v_rndne_f64_e32 v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v4, 0, v4, s5
	v_cndmask_b32_e64 v3, 0, v3, s5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[11:12], s[12:13], v[7:8]
	v_cvt_i32_f64_e32 v17, v[11:12]
	v_trunc_f64_e32 v[7:8], v[9:10]
	v_fma_f64 v[13:14], v[11:12], s[6:7], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s5, v[7:8], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[14:15], s[10:11]
	v_cmp_class_f64_e64 s11, s[8:9], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[16:17]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[20:21]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[28:29]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[34:35]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], 1.0
	v_fma_f64 v[11:12], v[13:14], v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[11:12], v[11:12], v17
	v_cndmask_b32_e32 v12, 0x7ff00000, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s6, v11
	v_cndmask_b32_e64 v12, 0, v12, s4
	s_and_b32 s4, s4, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s6, s6, 0
	v_cmp_eq_f64_e64 s4, v[5:6], v[1:2]
	v_mov_b32_e32 v11, s6
	v_fma_f64 v[3:4], v[11:12], v[3:4], v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v3
	s_and_b32 s10, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v4, v12, vcc_lo
	v_cmp_lt_f64_e64 vcc_lo, s[8:9], 0
	v_cmp_eq_f64_e64 s10, s[8:9], 0
	s_cselect_b32 s6, s6, s7
	s_and_b32 s7, s4, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s5, s7, exec_lo
	s_cselect_b32 s5, s9, 0x3ff00000
	v_bfi_b32 v3, 0x7fffffff, v3, s5
	v_cmp_gt_f64_e64 s5, 0, v[1:2]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s4
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, s6, 0
	s_and_b32 s12, vcc_lo, exec_lo
	s_cselect_b32 s6, s4, s6
	v_cndmask_b32_e32 v1, v3, v4, vcc_lo
	s_or_b32 vcc_lo, s10, s11
	s_xor_b32 s4, s5, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, 0, 0x7ff00000
	s_and_b32 s5, s7, exec_lo
	s_cselect_b32 s5, s9, 0
	v_mov_b32_e32 v2, s5
	s_and_b32 s5, vcc_lo, exec_lo
	s_cselect_b32 s5, 0, s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_bfi_b32 v2, 0x7fffffff, s4, v2
	v_cmp_o_f64_e64 s4, s[8:9], s[8:9]
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v2, 0x7ff80000, v1, s4
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, s5, 0
	v_mov_b32_e32 v1, s4
.LBB4_5:
	s_clause 0x1
	s_load_b256 s[8:15], s[0:1], 0x48
	s_load_b256 s[16:23], s[0:1], 0x0
	s_mul_i32 s4, s27, s25
	s_ashr_i32 s29, s27, 31
	s_mul_hi_i32 s31, s33, s4
	s_mul_i32 s30, s33, s4
	v_cmp_gt_i32_e64 s4, s27, v0
	s_mul_hi_u32 s42, s47, s50
	s_mov_b32 s28, s27
	s_mul_hi_i32 s35, s48, s27
	s_mul_i32 s34, s48, s27
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s14, 0
	s_cselect_b32 s6, -1, 0
	s_and_saveexec_b32 s5, s4
	s_cbranch_execz .LBB4_11
; %bb.6:
	s_load_b32 s40, s[0:1], 0x74
	s_lshl_b64 s[36:37], s[30:31], 2
	s_mul_hi_i32 s39, s28, s2
	s_add_u32 s7, s16, s36
	s_addc_u32 s25, s17, s37
	s_lshl_b64 s[16:17], s[34:35], 2
	s_mul_i32 s38, s28, s2
	s_add_u32 s7, s7, s16
	s_addc_u32 s16, s25, s17
	s_lshl_b64 s[36:37], s[38:39], 2
	v_lshl_add_u32 v6, v0, 2, 0x110
	v_mov_b32_e32 v3, v0
	s_add_u32 s17, s12, s36
	s_addc_u32 s25, s13, s37
	s_lshl_b32 s37, s28, 2
	s_mov_b32 s39, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s36, s40, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s38, s36, 2
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB4_9
	.p2align	6
.LBB4_7:                                ;   in Loop: Header=BB4_9 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s17, v4
	v_add_co_ci_u32_e64 v8, null, s25, v5, vcc_lo
	global_load_b32 v7, v[7:8], off
.LBB4_8:                                ;   in Loop: Header=BB4_9 Depth=1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s7, v4
	v_add_co_ci_u32_e64 v5, null, s16, v5, vcc_lo
	v_add_nc_u32_e32 v3, s36, v3
	s_waitcnt vmcnt(0)
	ds_store_b32 v6, v7
	global_load_b32 v4, v[4:5], off
	v_add_nc_u32_e32 v5, s37, v6
	v_cmp_le_i32_e32 vcc_lo, s28, v3
	v_add_nc_u32_e32 v6, s38, v6
	s_or_b32 s39, vcc_lo, s39
	s_waitcnt vmcnt(0)
	ds_store_b32 v5, v4
	s_and_not1_b32 exec_lo, exec_lo, s39
	s_cbranch_execz .LBB4_11
.LBB4_9:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v4, 31, v3
	s_and_not1_b32 vcc_lo, exec_lo, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 2, v[3:4]
	s_cbranch_vccz .LBB4_7
; %bb.10:                               ;   in Loop: Header=BB4_9 Depth=1
	v_mov_b32_e32 v7, 0
	s_branch .LBB4_8
.LBB4_11:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s5
	v_cmp_eq_u32_e64 s5, 0, v0
	s_ashr_i32 s43, s48, 31
	s_ashr_i32 s44, s49, 31
	s_mov_b32 s16, 0
	s_and_saveexec_b32 s7, s5
	s_cbranch_execz .LBB4_16
; %bb.12:
	s_and_not1_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccnz .LBB4_14
; %bb.13:
	s_lshl_b64 s[16:17], s[2:3], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s36, s8, s16
	s_addc_u32 s37, s9, s17
	s_add_u32 s16, s10, s16
	s_addc_u32 s17, s11, s17
	s_load_b32 s6, s[36:37], 0x0
	s_load_b32 s16, s[16:17], 0x0
	s_branch .LBB4_15
.LBB4_14:
	s_mov_b32 s6, 0xff800000
.LBB4_15:
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v4, s6
	v_mov_b32_e32 v5, s16
	ds_store_2addr_b32 v3, v4, v5 offset0:65 offset1:66
.LBB4_16:
	s_or_b32 exec_lo, exec_lo, s7
	s_lshl_b64 s[6:7], s[28:29], 3
	s_mul_hi_i32 s37, s26, s28
	s_mul_i32 s36, s26, s28
	s_sub_u32 s16, 0x400000, s6
	s_subb_u32 s17, 0, s7
	s_lshl_b64 s[6:7], s[36:37], 3
	s_waitcnt lgkmcnt(0)
	s_or_b64 s[36:37], s[16:17], s[6:7]
	s_mov_b32 s36, 0
	s_barrier
	s_cmp_lg_u64 s[36:37], 0
	buffer_gl0_inv
	s_cbranch_scc0 .LBB4_49
; %bb.17:
	s_ashr_i32 s38, s7, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_add_u32 s40, s6, s38
	s_mov_b32 s39, s38
	s_addc_u32 s41, s7, s38
	s_xor_b64 s[40:41], s[40:41], s[38:39]
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v3, s40
	v_cvt_f32_u32_e32 v4, s41
	s_sub_u32 s29, 0, s40
	s_subb_u32 s37, 0, s41
	v_fmamk_f32 v3, v4, 0x4f800000, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v3, v3
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v3, 0x5f7ffffc, v3
	v_mul_f32_e32 v4, 0x2f800000, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v4, v4
	v_fmamk_f32 v3, v4, 0xcf800000, v3
	v_cvt_u32_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v3, v3
	v_readfirstlane_b32 s7, v4
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s25, v3
	s_mul_i32 s45, s29, s7
	s_mul_hi_u32 s49, s29, s25
	s_mul_i32 s48, s37, s25
	s_add_i32 s45, s49, s45
	s_mul_i32 s50, s29, s25
	s_add_i32 s45, s45, s48
	s_mul_hi_u32 s49, s25, s50
	s_mul_i32 s52, s25, s45
	s_mul_hi_u32 s51, s7, s50
	s_mul_i32 s48, s7, s50
	s_mul_hi_u32 s50, s25, s45
	s_add_u32 s49, s49, s52
	s_addc_u32 s50, 0, s50
	s_mul_hi_u32 s53, s7, s45
	s_add_u32 s48, s49, s48
	s_mul_i32 s45, s7, s45
	s_addc_u32 s48, s50, s51
	s_addc_u32 s49, s53, 0
	s_add_u32 s45, s48, s45
	s_addc_u32 s48, 0, s49
	s_add_u32 s25, s25, s45
	s_cselect_b32 s45, -1, 0
	s_mul_hi_u32 s49, s29, s25
	s_cmp_lg_u32 s45, 0
	s_mul_i32 s45, s29, s25
	s_addc_u32 s7, s7, s48
	s_mul_i32 s37, s37, s25
	s_mul_i32 s29, s29, s7
	s_mul_hi_u32 s48, s25, s45
	s_add_i32 s29, s49, s29
	s_mul_hi_u32 s49, s7, s45
	s_add_i32 s29, s29, s37
	s_mul_i32 s37, s7, s45
	s_mul_i32 s51, s25, s29
	s_mul_hi_u32 s50, s25, s29
	s_add_u32 s48, s48, s51
	s_addc_u32 s50, 0, s50
	s_mul_hi_u32 s45, s7, s29
	s_add_u32 s37, s48, s37
	s_mul_i32 s29, s7, s29
	s_addc_u32 s37, s50, s49
	s_addc_u32 s45, s45, 0
	s_add_u32 s29, s37, s29
	s_addc_u32 s37, 0, s45
	s_add_u32 s25, s25, s29
	s_cselect_b32 s29, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s29, 0
	s_addc_u32 s7, s7, s37
	s_ashr_i32 s48, s17, 31
	s_add_u32 s50, s16, s48
	s_mov_b32 s49, s48
	s_addc_u32 s51, s17, s48
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b64 s[50:51], s[50:51], s[48:49]
	s_mul_i32 s29, s50, s7
	s_mul_hi_u32 s37, s50, s25
	s_mul_hi_u32 s17, s50, s7
	s_mul_hi_u32 s52, s51, s25
	s_mul_i32 s25, s51, s25
	s_add_u32 s29, s37, s29
	s_addc_u32 s17, 0, s17
	s_mul_hi_u32 s45, s51, s7
	s_add_u32 s25, s29, s25
	s_mul_i32 s7, s51, s7
	s_addc_u32 s17, s17, s52
	s_addc_u32 s25, s45, 0
	s_add_u32 s7, s17, s7
	s_addc_u32 s17, 0, s25
	s_mul_hi_u32 s25, s40, s7
	s_mul_i32 s29, s40, s17
	s_mul_i32 s37, s41, s7
	s_add_i32 s25, s25, s29
	s_mul_i32 s29, s40, s7
	s_add_i32 s25, s25, s37
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_sub_i32 s37, s51, s25
	s_sub_u32 s29, s50, s29
	s_cselect_b32 s45, -1, 0
	s_cmp_lg_u32 s45, 0
	s_subb_u32 s37, s37, s41
	s_sub_u32 s50, s29, s40
	s_cselect_b32 s52, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s52, 0
	s_subb_u32 s37, s37, 0
	s_cmp_ge_u32 s37, s41
	s_cselect_b32 s52, -1, 0
	s_cmp_ge_u32 s50, s40
	s_cselect_b32 s50, -1, 0
	s_cmp_eq_u32 s37, s41
	s_cselect_b32 s37, s50, s52
	s_add_u32 s50, s7, 1
	s_addc_u32 s52, s17, 0
	s_add_u32 s53, s7, 2
	s_addc_u32 s54, s17, 0
	s_cmp_lg_u32 s37, 0
	s_cselect_b32 s37, s53, s50
	s_cselect_b32 s50, s54, s52
	s_cmp_lg_u32 s45, 0
	s_subb_u32 s25, s51, s25
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_ge_u32 s25, s41
	s_cselect_b32 s45, -1, 0
	s_cmp_ge_u32 s29, s40
	s_cselect_b32 s29, -1, 0
	s_cmp_eq_u32 s25, s41
	s_cselect_b32 s25, s29, s45
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s25, 0
	s_cselect_b32 s41, s50, s17
	s_cselect_b32 s40, s37, s7
	s_xor_b64 s[38:39], s[48:49], s[38:39]
	s_xor_b64 s[40:41], s[40:41], s[38:39]
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_u32 s38, s40, s38
	s_subb_u32 s39, s41, s39
	s_and_not1_b32 vcc_lo, exec_lo, s36
	s_cbranch_vccnz .LBB4_19
.LBB4_18:
	v_cvt_f32_u32_e32 v3, s6
	s_sub_i32 s17, 0, s6
	s_mov_b32 s39, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v3, v3
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v3, 0x4f7ffffe, v3
	v_cvt_u32_f32_e32 v3, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s7, v3
	s_mul_i32 s17, s17, s7
	s_mul_hi_u32 s17, s7, s17
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s7, s7, s17
	s_mul_hi_u32 s7, s16, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s17, s7, s6
	s_sub_i32 s16, s16, s17
	s_add_i32 s17, s7, 1
	s_sub_i32 s25, s16, s6
	s_cmp_ge_u32 s16, s6
	s_cselect_b32 s7, s17, s7
	s_cselect_b32 s16, s25, s16
	s_add_i32 s17, s7, 1
	s_cmp_ge_u32 s16, s6
	s_cselect_b32 s38, s17, s7
.LBB4_19:
	s_cmp_lt_i32 s24, 1
	s_cbranch_scc1 .LBB4_43
; %bb.20:
	s_ashr_i32 s25, s24, 31
	v_cmp_gt_i64_e64 s7, s[38:39], 0
	v_cmp_lt_i64_e64 s6, s[38:39], s[24:25]
	s_mul_i32 s16, s42, s46
	s_load_b64 s[36:37], s[0:1], 0x40
	v_dual_mov_b32 v14, 0 :: v_dual_and_b32 v7, 31, v0
	v_lshrrev_b32_e32 v3, 3, v0
	s_and_b32 s6, s6, exec_lo
	s_cselect_b32 s6, s38, s24
	s_and_b32 s7, s7, exec_lo
	s_cselect_b32 s25, s6, 1
	s_sub_i32 s6, s47, s16
	s_xor_b32 s7, s43, s44
	s_add_i32 s16, s42, 1
	s_sub_i32 s17, s6, s46
	s_cmp_ge_u32 s6, s46
	v_lshlrev_b32_e32 v4, 2, v0
	s_cselect_b32 s16, s16, s42
	s_cselect_b32 s6, s17, s6
	s_add_i32 s17, s16, 1
	s_cmp_ge_u32 s6, s46
	v_mbcnt_lo_u32_b32 v9, -1, 0
	s_cselect_b32 s6, s17, s16
	s_waitcnt lgkmcnt(0)
	s_add_i32 s33, s33, s36
	s_xor_b32 s16, s6, s7
	v_cmp_gt_u32_e64 s6, 32, v0
	s_sub_i32 s7, s16, s7
	v_and_b32_e32 v8, 0x7c, v3
	s_mul_hi_i32 s17, s7, s28
	s_mul_i32 s16, s7, s28
	v_cmp_eq_u32_e64 s7, 0, v7
	s_lshl_b64 s[38:39], s[16:17], 2
	v_lshlrev_b32_e32 v10, 2, v7
	s_add_u32 s29, s18, s38
	s_addc_u32 s36, s19, s39
	s_add_u32 s16, s0, 0x68
	s_addc_u32 s17, s1, 0
	s_cmp_lt_i32 s33, s37
	v_add_nc_u32_e32 v12, 0x110, v4
	s_cselect_b32 s37, -1, 0
	s_add_u32 s20, s20, s38
	s_addc_u32 s21, s21, s39
	s_lshl_b32 s18, s28, 2
	v_lshl_or_b32 v13, v9, 2, 64
	v_add3_u32 v11, 0x110, s18, v4
	s_mul_i32 s26, s27, s26
	s_mov_b32 s27, 0
	s_branch .LBB4_22
.LBB4_21:                               ;   in Loop: Header=BB4_22 Depth=1
	s_cmp_lt_i32 s27, s24
	s_cbranch_scc0 .LBB4_43
.LBB4_22:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB4_25 Depth 2
                                        ;       Child Loop BB4_27 Depth 3
                                        ;       Child Loop BB4_42 Depth 3
	s_mov_b32 s38, s27
	s_add_i32 s27, s27, s25
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_min_i32 s39, s27, s24
	s_cmp_ge_i32 s38, s39
	s_cbranch_scc1 .LBB4_21
; %bb.23:                               ;   in Loop: Header=BB4_22 Depth=1
	v_cmp_gt_u32_e32 vcc_lo, 24, v9
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_lshl_u32 v15, v3, v9, 2
	v_cndmask_b32_e64 v4, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v9
	v_add_lshl_u32 v16, v4, v9, 2
	v_cndmask_b32_e64 v5, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_lshl_u32 v17, v5, v9, 2
	v_add_co_ci_u32_e64 v6, null, 0, v9, vcc_lo
	v_lshlrev_b32_e32 v18, 2, v6
	s_branch .LBB4_25
.LBB4_24:                               ;   in Loop: Header=BB4_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s40
	s_add_i32 s38, s38, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s38, s39
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB4_21
.LBB4_25:                               ;   Parent Loop BB4_22 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB4_27 Depth 3
                                        ;       Child Loop BB4_42 Depth 3
	v_mov_b32_e32 v5, 0
	s_mul_hi_i32 s19, s38, s26
	s_mul_i32 s18, s38, s26
	s_and_saveexec_b32 s40, s4
	s_cbranch_execz .LBB4_29
; %bb.26:                               ;   in Loop: Header=BB4_25 Depth=2
	s_load_b32 s44, s[16:17], 0xc
	s_lshl_b64 s[42:43], s[18:19], 2
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v6, v11
	v_mov_b32_e32 v3, v0
	s_add_u32 s41, s29, s42
	s_addc_u32 s42, s36, s43
	s_waitcnt lgkmcnt(0)
	s_and_b32 s43, s44, 0xffff
	s_mov_b32 s44, 0
	s_lshl_b32 s45, s43, 2
	.p2align	6
.LBB4_27:                               ;   Parent Loop BB4_22 Depth=1
                                        ;     Parent Loop BB4_25 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[19:20], 2, v[3:4]
	v_add_nc_u32_e32 v3, s43, v3
	v_add_co_u32 v19, vcc_lo, s41, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v20, null, s42, v20, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s28, v3
	global_load_b32 v4, v[19:20], off
	ds_load_b32 v19, v6
	v_add_nc_u32_e32 v6, s45, v6
	s_or_b32 s44, vcc_lo, s44
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fmac_f32_e32 v5, v19, v4
	s_and_not1_b32 exec_lo, exec_lo, s44
	s_cbranch_execnz .LBB4_27
; %bb.28:                               ;   in Loop: Header=BB4_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s44
.LBB4_29:                               ;   in Loop: Header=BB4_25 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s40
	ds_bpermute_b32 v3, v13, v5
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v5, v3
	ds_bpermute_b32 v4, v15, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_bpermute_b32 v4, v16, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_bpermute_b32 v4, v17, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_bpermute_b32 v4, v18, v3
	s_and_saveexec_b32 s40, s7
	s_cbranch_execz .LBB4_31
; %bb.30:                               ;   in Loop: Header=BB4_25 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_store_b32 v8, v3
.LBB4_31:                               ;   in Loop: Header=BB4_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s40
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s40, s6
	s_cbranch_execz .LBB4_36
; %bb.32:                               ;   in Loop: Header=BB4_25 Depth=2
	s_load_b32 s41, s[16:17], 0xc
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s41, s41, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s41, s41, 31
	s_lshr_b32 s41, s41, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_u32_e32 vcc_lo, s41, v7
	s_and_saveexec_b32 s41, vcc_lo
; %bb.33:                               ;   in Loop: Header=BB4_25 Depth=2
	ds_load_b32 v3, v10
; %bb.34:                               ;   in Loop: Header=BB4_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s41
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v4, v13, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_bpermute_b32 v4, v15, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_bpermute_b32 v4, v16, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_bpermute_b32 v4, v17, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_bpermute_b32 v4, v18, v3
	s_and_b32 exec_lo, exec_lo, s7
	s_cbranch_execz .LBB4_36
; %bb.35:                               ;   in Loop: Header=BB4_25 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v3, v3, v4
	ds_store_b32 v14, v3
.LBB4_36:                               ;   in Loop: Header=BB4_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s40
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v4, v14
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s40, s5
	s_cbranch_execz .LBB4_40
; %bb.37:                               ;   in Loop: Header=BB4_25 Depth=2
	s_add_i32 s41, s38, s14
	v_mov_b32_e32 v3, 0xf149f2ca
	s_cmp_lt_i32 s33, s41
	s_cselect_b32 s42, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s42, s37, s42
	s_and_b32 vcc_lo, exec_lo, s42
	s_cbranch_vccnz .LBB4_39
; %bb.38:                               ;   in Loop: Header=BB4_25 Depth=2
	s_sub_i32 s41, s33, s41
	v_cvt_f64_f32_e32 v[3:4], v4
	v_cvt_f64_i32_e32 v[5:6], s41
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], -v[1:2], v[5:6], v[3:4]
	v_cvt_f32_f64_e32 v3, v[3:4]
.LBB4_39:                               ;   in Loop: Header=BB4_25 Depth=2
	ds_load_2addr_b32 v[19:20], v14 offset0:65 offset1:66
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f32_e32 vcc_lo, v3, v19
	v_cndmask_b32_e32 v4, v19, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v5, v19, v4
	v_mul_f32_e32 v6, 0x3fb8aa3b, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f32 v21, 0x3fb8aa3b, v5, -v6
	v_rndne_f32_e32 v22, v6
	v_dual_sub_f32 v6, v6, v22 :: v_dual_fmac_f32 v21, 0x32a5705f, v5
	v_sub_f32_e32 v3, v3, v4
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_add_f32 v6, v6, v21 :: v_dual_mul_f32 v19, 0x3fb8aa3b, v3
	v_cvt_i32_f32_e32 v21, v22
	v_exp_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f32 v23, 0x3fb8aa3b, v3, -v19
	v_rndne_f32_e32 v24, v19
	v_cvt_i32_f32_e32 v22, v24
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v6, v6, v21
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_dual_fmac_f32 v23, 0x32a5705f, v3 :: v_dual_cndmask_b32 v6, 0, v6
	v_sub_f32_e32 v19, v19, v24
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v3
	v_add_f32_e32 v19, v19, v23
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v19, v19
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v19, v19, v22
	v_cndmask_b32_e32 v19, 0, v19, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v5
	v_cndmask_b32_e32 v6, 0x7f800000, v6, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, 0x7f800000, v19, vcc_lo
	v_fma_f32 v5, v20, v6, v3
	ds_store_b128 v14, v[3:6] offset:256
.LBB4_40:                               ;   in Loop: Header=BB4_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s40
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s40, s4
	s_cbranch_execz .LBB4_24
; %bb.41:                               ;   in Loop: Header=BB4_25 Depth=2
	s_load_b32 s41, s[16:17], 0xc
	ds_load_2addr_b32 v[3:4], v14 offset0:64 offset1:67
	s_lshl_b64 s[18:19], s[18:19], 2
	v_mov_b32_e32 v19, v12
	v_mov_b32_e32 v5, v0
	s_add_u32 s18, s20, s18
	s_addc_u32 s19, s21, s19
	s_mov_b32 s43, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s41, s41, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s42, s41, 2
	.p2align	6
.LBB4_42:                               ;   Parent Loop BB4_22 Depth=1
                                        ;     Parent Loop BB4_25 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[20:21], 2, v[5:6]
	v_add_nc_u32_e32 v5, s41, v5
	v_add_co_u32 v20, vcc_lo, s18, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v21, null, s19, v21, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s28, v5
	global_load_b32 v6, v[20:21], off
	ds_load_b32 v20, v19
	s_or_b32 s43, vcc_lo, s43
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v6, v3, v6
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v6, v20, v4
	ds_store_b32 v19, v6
	v_add_nc_u32_e32 v19, s42, v19
	s_and_not1_b32 exec_lo, exec_lo, s43
	s_cbranch_execnz .LBB4_42
	s_branch .LBB4_24
.LBB4_43:
	s_cmp_lg_u32 s15, 0
	s_cbranch_scc0 .LBB4_50
; %bb.44:
	s_and_saveexec_b32 s7, s4
	s_cbranch_execz .LBB4_47
; %bb.45:
	v_mov_b32_e32 v1, 0
	s_load_b32 s6, s[0:1], 0x74
	s_lshl_b64 s[14:15], s[30:31], 2
	v_lshl_add_u32 v4, v0, 2, 0x110
	s_add_u32 s16, s22, s14
	ds_load_b32 v3, v1 offset:264
	s_addc_u32 s17, s23, s15
	s_lshl_b64 s[14:15], s[34:35], 2
	v_mov_b32_e32 v1, v0
	s_add_u32 s14, s16, s14
	s_addc_u32 s15, s17, s15
	s_mov_b32 s18, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s16, s6, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s17, s16, 2
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB4_46:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b32 v7, v4
	v_ashrrev_i32_e32 v2, 31, v1
	v_add_nc_u32_e32 v4, s17, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 2, v[1:2]
	v_add_nc_u32_e32 v1, s16, v1
	v_cmp_le_i32_e64 s6, s28, v1
	s_or_b32 s18, s6, s18
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v8, null, v3, v3, v7
	v_div_scale_f32 v2, vcc_lo, v7, v3, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v9, v8
	s_waitcnt_depctr 0xfff
	v_fma_f32 v10, -v8, v9, 1.0
	v_fmac_f32_e32 v9, v10, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v10, v2, v9
	v_fma_f32 v11, -v8, v10, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v10, v11, v9
	v_fma_f32 v2, -v8, v10, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v9, v10
	v_add_co_u32 v5, vcc_lo, s14, v5
	v_add_co_ci_u32_e64 v6, null, s15, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_div_fixup_f32 v2, v2, v3, v7
	global_store_b32 v[5:6], v2, off
	s_and_not1_b32 exec_lo, exec_lo, s18
	s_cbranch_execnz .LBB4_46
.LBB4_47:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s7
	s_cbranch_execz .LBB4_51
.LBB4_48:
	s_endpgm
.LBB4_49:
                                        ; implicit-def: $sgpr38_sgpr39
	s_branch .LBB4_18
.LBB4_50:
.LBB4_51:
	s_and_saveexec_b32 s6, s4
	s_cbranch_execz .LBB4_54
; %bb.52:
	s_load_b32 s7, s[0:1], 0x74
	s_mul_hi_i32 s1, s28, s2
	s_mul_i32 s0, s28, s2
	v_lshl_add_u32 v2, v0, 2, 0x110
	s_lshl_b64 s[14:15], s[0:1], 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_u32 s1, s12, s14
	s_addc_u32 s4, s13, s15
	s_mov_b32 s13, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s7, s7, 0xffff
	s_lshl_b32 s12, s7, 2
	.p2align	6
.LBB4_53:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b32 v5, v2
	v_ashrrev_i32_e32 v1, 31, v0
	v_add_nc_u32_e32 v2, s12, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[0:1]
	v_add_nc_u32_e32 v0, s7, v0
	v_cmp_le_i32_e32 vcc_lo, s28, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, s0, s1, v3
	v_add_co_ci_u32_e64 v4, null, s4, v4, s0
	s_or_b32 s13, vcc_lo, s13
	s_waitcnt lgkmcnt(0)
	global_store_b32 v[3:4], v5, off
	s_and_not1_b32 exec_lo, exec_lo, s13
	s_cbranch_execnz .LBB4_53
.LBB4_54:
	s_or_b32 exec_lo, exec_lo, s6
	s_and_saveexec_b32 s0, s5
	s_cbranch_execz .LBB4_48
; %bb.55:
	v_mov_b32_e32 v2, 0
	s_lshl_b64 s[0:1], s[2:3], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s2, s10, s0
	s_addc_u32 s3, s11, s1
	ds_load_2addr_b32 v[0:1], v2 offset0:65 offset1:66
	s_add_u32 s0, s8, s0
	s_addc_u32 s1, s9, s1
	s_waitcnt lgkmcnt(0)
	s_clause 0x1
	global_store_b32 v2, v0, s[0:1]
	global_store_b32 v2, v1, s[2:3]
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
		.amdhsa_group_segment_fixed_size 272
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 360
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
		.amdhsa_next_free_vgpr 25
		.amdhsa_next_free_sgpr 56
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
		.amdhsa_inst_pref_size 63
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,"axG",@progbits,_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,comdat
.Lfunc_end4:
	.size	_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii, .Lfunc_end4-_Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
                                        ; -- End function
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.num_vgpr, 25
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.num_agpr, 0
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.numbered_sgpr, 56
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.num_named_barrier, 0
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.private_seg_size, 0
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.uses_vcc, 1
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.uses_flat_scratch, 0
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.has_dyn_sized_stack, 0
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.has_recursion, 0
	.set _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 9264
; TotalNumSgprs: 58
; NumVgprs: 25
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 272 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 58
; NumVGPRsForWavesPerEU: 25
; Occupancy: 16
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,"axG",@progbits,_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,comdat
	.protected	_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii ; -- Begin function _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
	.globl	_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
	.p2align	8
	.type	_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,@function
_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii: ; @_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
; %bb.0:
	s_clause 0x1
	s_load_b128 s[28:31], s[0:1], 0x24
	s_load_b64 s[6:7], s[0:1], 0x38
	v_mov_b32_e32 v5, 0
	s_abs_i32 s8, s2
	v_mov_b32_e32 v6, 0
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s4, s29
	s_ashr_i32 s9, s29, 31
	v_cvt_f32_u32_e32 v1, s4
	s_sub_i32 s5, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v1
	s_mul_i32 s5, s5, s3
	s_mul_hi_u32 s5, s3, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s3, s5
	s_ashr_i32 s3, s2, 31
	s_mul_hi_u32 s5, s8, s5
	s_xor_b32 s11, s3, s9
	s_mul_i32 s10, s5, s4
	s_sub_i32 s8, s8, s10
	s_add_i32 s10, s5, 1
	s_sub_i32 s12, s8, s4
	s_cmp_ge_u32 s8, s4
	s_cselect_b32 s5, s10, s5
	s_cselect_b32 s8, s12, s8
	s_add_i32 s10, s5, 1
	s_cmp_ge_u32 s8, s4
	s_cselect_b32 s5, s10, s5
	s_abs_i32 s8, s30
	s_xor_b32 s5, s5, s11
	v_cvt_f32_u32_e32 v1, s8
	s_sub_i32 s12, 0, s8
	s_sub_i32 s33, s5, s11
	s_ashr_i32 s13, s30, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_xor_b32 s9, s9, s13
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s10, v1
	s_mul_i32 s12, s12, s10
	s_mul_hi_u32 s12, s10, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s10, s10, s12
	s_mul_hi_u32 s5, s4, s10
	s_mul_i32 s10, s33, s29
	s_mul_i32 s11, s5, s8
	s_sub_i32 s48, s2, s10
	s_sub_i32 s4, s4, s11
	s_add_i32 s10, s5, 1
	s_sub_i32 s11, s4, s8
	s_cmp_ge_u32 s4, s8
	s_cselect_b32 s5, s10, s5
	s_cselect_b32 s4, s11, s4
	s_add_i32 s10, s5, 1
	s_cmp_ge_u32 s4, s8
	s_cselect_b32 s4, s10, s5
	v_cmp_ngt_f64_e64 s5, s[6:7], 0
	s_xor_b32 s4, s4, s9
	s_abs_i32 s47, s48
	s_sub_i32 s49, s4, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_abs_i32 s46, s49
	v_cvt_f32_u32_e32 v1, s46
	s_sub_i32 s4, 0, s46
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s50, v1
	s_mul_i32 s4, s4, s50
	s_and_b32 vcc_lo, exec_lo, s5
	s_mul_hi_u32 s4, s50, s4
	s_add_i32 s50, s50, s4
	s_cbranch_vccnz .LBB5_5
; %bb.1:
	v_cvt_f64_i32_e32 v[1:2], s29
	s_mov_b32 s11, 0x3fe55555
	s_mov_b32 s10, 0x55555555
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_mov_b32 s43, 0x3fd99999
	s_mov_b32 s42, 0x998ef7b6
	s_mov_b32 s12, 0xffda0d24
	s_mov_b32 s13, 0x3c7777d0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[3:4], v[1:2]
	v_cmp_gt_f64_e32 vcc_lo, s[10:11], v[3:4]
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v5
	v_add_f64 v[5:6], v[3:4], 1.0
	v_add_f64 v[11:12], v[3:4], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[7:8], v[5:6]
	v_add_f64 v[13:14], v[5:6], -1.0
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[5:6], v[9:10]
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[9:10], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[11:12], -v[5:6]
	v_add_f64 v[15:16], v[5:6], -v[15:16]
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[15:16], -v[3:4]
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[3:4], v[13:14], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[3:4], v[7:8], v[3:4]
	v_add_f64 v[5:6], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[5:6], v[5:6]
	v_fma_f64 v[11:12], v[7:8], s[8:9], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	s_mov_b32 s8, 0x55555780
	s_mov_b32 s9, s11
	v_mul_f64 v[13:14], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[4:5]
	s_mov_b32 s5, 0x3fd24924
	s_mov_b32 s4, 0x9b27acf1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[42:43]
	v_fma_f64 v[7:8], v[7:8], v[11:12], s[8:9]
	v_ldexp_f64 v[11:12], v[5:6], 1
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_mov_b32 s8, 0x652b82fe
	s_mov_b32 s9, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[7:8], v[13:14], v[7:8]
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[11:12], v[7:8]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], -v[11:12]
	v_add_f64 v[5:6], v[7:8], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], s[8:9]
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], s[8:9], -v[9:10]
	v_fma_f64 v[3:4], v[3:4], s[8:9], v[7:8]
	v_frexp_exp_i32_f64_e32 v7, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[1:2], v[5:6], s[12:13], v[3:4]
	v_subrev_co_ci_u32_e64 v3, null, 0, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[3:4], v3
	v_add_f64 v[5:6], v[9:10], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], v[3:4]
	v_add_f64 v[9:10], v[5:6], -v[9:10]
	v_add_f64 v[11:12], v[7:8], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[1:2], v[1:2], -v[9:10]
	v_add_f64 v[13:14], v[11:12], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[13:14], v[3:4]
	v_add_f64 v[3:4], v[5:6], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_f64 v[1:2], v[7:8], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_floor_f64_e32 v[1:2], v[1:2]
	v_cvt_i32_f64_e32 v1, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s4, v1
	s_lshl_b32 s20, 1, s4
	v_cvt_f64_i32_e32 v[1:2], s20
	s_cmp_ge_i32 s48, s20
	s_cbranch_scc0 .LBB5_3
; %bb.2:
	v_mul_f64 v[3:4], s[6:7], -0.5
	s_mov_b32 s13, 0x3c7abc9e
	s_mov_b32 s12, 0x3b39803f
	s_mov_b32 s17, 0x3fe62e42
	s_mov_b32 s16, 0xfefa39ef
	s_mov_b32 s14, 0xfca7ab0c
	s_mov_b32 s18, 0x6a5dcb37
	s_mov_b32 s15, 0x3e928af3
	s_mov_b32 s19, 0x3e5ade15
	s_mov_b32 s22, 0x623fde64
	s_mov_b32 s23, 0x3ec71dee
	s_mov_b32 s24, 0x7c89e6b0
	s_mov_b32 s25, 0x3efa0199
	s_mov_b32 s26, 0x14761f6e
	s_mov_b32 s27, 0x3f2a01a0
	s_mov_b32 s34, 0x1852b7b0
	s_mov_b32 s35, 0x3f56c16c
	s_mov_b32 s36, 0x11122322
	s_mov_b32 s37, 0x3f811111
	s_mov_b32 s38, 0x555502a1
	s_mov_b32 s39, 0x3fa55555
	s_mov_b32 s40, 0x55555511
	s_mov_b32 s41, 0x3fc55555
	s_mov_b32 s44, 11
	s_mov_b32 s45, 0x3fe00000
	s_mov_b32 s54, 0x4222de17
	s_mov_b32 s55, 0x3fbdee67
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[5:6], null, v[1:2], v[1:2], v[3:4]
	v_div_scale_f64 v[11:12], vcc_lo, v[3:4], v[1:2], v[3:4]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[5:6], v[1:2], v[3:4]
	v_rndne_f64_e32 v[5:6], v[3:4]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[3:4]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[3:4], -v[5:6]
	v_cvt_i32_f64_e32 v11, v[5:6]
	s_and_b32 s42, vcc_lo, exec_lo
	v_mul_f64 v[9:10], v[7:8], s[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], s[16:17], v[9:10]
	v_fma_f64 v[9:10], v[7:8], s[18:19], s[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[22:23]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[26:27]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[36:37]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[40:41]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[44:45]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], 1.0
	v_fma_f64 v[5:6], v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v11
	v_readfirstlane_b32 s21, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s42, v5
	s_cselect_b32 s21, s21, 0x7ff00000
	s_and_b32 s51, s4, vcc_lo
	s_and_b32 s51, s51, exec_lo
	s_cselect_b32 s52, s42, 0
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s53, s21, 0
	s_sub_i32 s4, s48, s20
	v_cmp_neq_f64_e64 vcc_lo, s[52:53], 1.0
	s_lshl_b32 s4, s4, 1
	s_mov_b32 s42, 0x9999999c
	s_or_b32 s4, s4, 1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_u32_e32 v[3:4], s4
	v_cndmask_b32_e32 v4, 0x3ff00000, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[3:4]
	s_and_b32 s4, vcc_lo, exec_lo
	s_cselect_b32 s21, s53, 0x3ff00000
	s_cselect_b32 s20, s52, 0
	s_mov_b32 s52, 0x968915a9
	v_frexp_mant_f64_e64 v[5:6], |s[20:21]|
	s_mov_b32 s53, 0x3fba6564
	s_mov_b32 s4, 0x924920da
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[10:11], v[5:6]
	v_cndmask_b32_e64 v7, 0, 1, vcc_lo
	v_ldexp_f64 v[5:6], v[5:6], v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], 1.0
	v_add_f64 v[13:14], v[5:6], -1.0
	v_rcp_f64_e32 v[9:10], v[7:8]
	v_add_f64 v[15:16], v[7:8], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	v_mul_f64 v[17:18], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[11:12], v[7:8], -v[17:18]
	v_fma_f64 v[5:6], v[11:12], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[17:18], v[5:6]
	v_add_f64 v[15:16], v[13:14], -v[7:8]
	v_add_f64 v[17:18], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_add_f64 v[5:6], v[17:18], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[5:6]
	v_mul_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_add_f64 v[9:10], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_fma_f64 v[9:10], v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[5:6], v[5:6]
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[54:55], s[52:53]
	s_mov_b32 s52, 0x3abe935a
	s_mov_b32 s53, 0x3fbe25e4
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_mul_f64 v[21:22], v[7:8], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x47e6c9c2
	s_mov_b32 s53, 0x3fc110ef
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0xcfa74449
	s_mov_b32 s53, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x71bf3c30
	s_mov_b32 s53, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x1c7792ce
	s_mov_b32 s53, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[4:5]
	s_mov_b32 s4, 0xd5df274d
	s_mov_b32 s5, 0x3c8543b0
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[11:12], v[13:14], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	v_add_f64 v[15:16], v[17:18], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[19:20], v[15:16], s[10:11]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_mov_b32 s11, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[23:24], v[19:20], s[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_fma_f64 v[17:18], v[13:14], v[7:8], -v[21:22]
	v_add_f64 v[15:16], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], s[4:5]
	v_fma_f64 v[13:14], v[13:14], v[5:6], v[17:18]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[7:8], v[13:14]
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[19:20], v[11:12]
	v_add_f64 v[15:16], v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[13:14]
	v_mul_f64 v[19:20], v[15:16], v[13:14]
	v_add_f64 v[21:22], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[17:18]
	v_fma_f64 v[17:18], v[15:16], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[21:22]
	v_fma_f64 v[11:12], v[15:16], v[11:12], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[9:10], v[13:14], v[11:12]
	v_frexp_exp_i32_f64_e32 v13, s[20:21]
	v_add_f64 v[11:12], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v13, null, 0, v13, vcc_lo
	v_cvt_f64_i32_e32 v[13:14], v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[7:8], v[11:12]
	v_add_f64 v[17:18], v[11:12], -v[19:20]
	v_mul_f64 v[19:20], v[13:14], s[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[15:16], -v[7:8]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], s[16:17], -v[19:20]
	s_mov_b32 s17, 0xbfe62e42
	v_add_f64 v[7:8], v[11:12], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_fma_f64 v[9:10], v[13:14], s[12:13], v[17:18]
	s_mov_b32 s13, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], v[5:6]
	v_add_f64 v[19:20], v[7:8], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[7:8], v[11:12]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[15:16], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[21:22]
	v_add_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[15:16], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[11:12]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	v_add_f64 v[17:18], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[11:12], v[17:18], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[17:18], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[7:8], -v[17:18]
	v_mul_f64 v[11:12], v[3:4], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[3:4], v[7:8], -v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	v_fma_f64 v[5:6], v[3:4], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_dual_cndmask_b32 v10, v8, v12 :: v_dual_cndmask_b32 v9, v7, v11
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[3:4], 0.5
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[13:14], v[9:10], s[8:9]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[9:10]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[9:10]
	v_cmp_neq_f64_e64 s5, 0x7ff00000, |v[9:10]|
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_trunc_f64_e32 v[7:8], v[3:4]
	v_rndne_f64_e32 v[13:14], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v6, 0, v6, s5
	v_cndmask_b32_e64 v5, 0, v5, s5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], s[16:17], v[9:10]
	v_cvt_i32_f64_e32 v19, v[13:14]
	v_trunc_f64_e32 v[9:10], v[11:12]
	v_fma_f64 v[15:16], v[13:14], s[12:13], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s5, v[9:10], v[11:12]
	v_fma_f64 v[17:18], v[15:16], s[18:19], s[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[22:23]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[26:27]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[36:37]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[40:41]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[44:45]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[13:14], v[13:14], v19
	v_cndmask_b32_e32 v14, 0x7ff00000, v14, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s8, v13
	v_cndmask_b32_e64 v14, 0, v14, s4
	s_and_b32 s4, s4, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s8, s8, 0
	v_cmp_eq_f64_e64 s4, v[7:8], v[3:4]
	v_mov_b32_e32 v13, s8
	v_fma_f64 v[5:6], v[13:14], v[5:6], v[13:14]
	v_cmp_class_f64_e64 vcc_lo, v[13:14], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s9, v5
	s_and_b32 s10, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v6, v14, vcc_lo
	v_cmp_lt_f64_e64 vcc_lo, s[20:21], 0
	v_cmp_eq_f64_e64 s10, s[20:21], 0
	s_cselect_b32 s8, s8, s9
	s_and_b32 s5, s4, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s9, s5, exec_lo
	s_cselect_b32 s9, s21, 0x3ff00000
	v_bfi_b32 v3, 0x7fffffff, v3, s9
	v_cmp_class_f64_e64 s9, s[20:21], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s4
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, s8, 0
	s_and_b32 s11, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	s_cselect_b32 s8, s4, s8
	s_or_b32 vcc_lo, s10, s9
	s_and_b32 s4, s10, exec_lo
	s_cselect_b32 s4, 0, 0x7ff00000
	s_and_b32 s5, s5, exec_lo
	s_cselect_b32 s5, s21, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v4, s5
	s_and_b32 s5, vcc_lo, exec_lo
	s_cselect_b32 s5, 0, s8
	v_bfi_b32 v4, 0x7fffffff, s4, v4
	v_cmp_o_f64_e64 s4, s[20:21], s[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	v_cndmask_b32_e64 v6, 0x7ff80000, v3, s4
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, s5, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v5, s4
	s_cbranch_execz .LBB5_4
	s_branch .LBB5_5
.LBB5_3:
                                        ; implicit-def: $vgpr5_vgpr6
.LBB5_4:
	s_delay_alu instid0(VALU_DEP_1)
	v_div_scale_f64 v[3:4], null, v[1:2], v[1:2], -s[6:7]
	v_div_scale_f64 v[9:10], vcc_lo, -s[6:7], v[1:2], -s[6:7]
	s_mov_b32 s13, 0x3fe62e42
	s_mov_b32 s12, 0xfefa39ef
	s_mov_b32 s10, 0xfca7ab0c
	s_mov_b32 s14, 0x6a5dcb37
	s_mov_b32 s11, 0x3e928af3
	s_mov_b32 s15, 0x3e5ade15
	s_mov_b32 s16, 0x623fde64
	s_mov_b32 s17, 0x3ec71dee
	s_mov_b32 s18, 0x7c89e6b0
	s_mov_b32 s19, 0x3efa0199
	s_mov_b32 s20, 0x14761f6e
	s_mov_b32 s21, 0x3f2a01a0
	s_mov_b32 s22, 0x1852b7b0
	s_mov_b32 s23, 0x3f56c16c
	s_mov_b32 s24, 0x11122322
	s_mov_b32 s25, 0x3f811111
	s_mov_b32 s26, 0x555502a1
	s_mov_b32 s27, 0x3fa55555
	s_mov_b32 s34, 0x55555511
	s_mov_b32 s35, 0x3fc55555
	s_mov_b32 s36, 11
	s_mov_b32 s37, 0x3fe00000
	s_mov_b32 s38, 0x968915a9
	s_mov_b32 s40, 0x4222de17
	s_mov_b32 s39, 0x3fba6564
	s_mov_b32 s41, 0x3fbdee67
	v_rcp_f64_e32 v[5:6], v[3:4]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_mul_f64 v[7:8], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], -v[3:4], v[7:8], v[9:10]
	v_div_fmas_f64 v[3:4], v[3:4], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[1:2], v[3:4], v[1:2], -s[6:7]
	s_mov_b32 s7, 0x3c7abc9e
	s_mov_b32 s6, 0x3b39803f
	v_rndne_f64_e32 v[3:4], v[1:2]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[1:2]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[1:2], -v[3:4]
	v_cvt_i32_f64_e32 v9, v[3:4]
	s_and_b32 s8, vcc_lo, exec_lo
	v_mul_f64 v[7:8], v[5:6], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], s[12:13], v[7:8]
	v_fma_f64 v[7:8], v[5:6], s[14:15], s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[16:17]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[20:21]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[24:25]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[34:35]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], 1.0
	v_fma_f64 v[3:4], v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v9
	v_readfirstlane_b32 s5, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s8, v3
	s_cselect_b32 s5, s5, 0x7ff00000
	s_and_b32 s9, s4, vcc_lo
	s_and_b32 s9, s9, exec_lo
	s_cselect_b32 s8, s8, 0
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s9, s5, 0
	s_add_i32 s4, s48, 1
	v_cmp_neq_f64_e64 vcc_lo, s[8:9], 1.0
	v_cvt_f64_i32_e32 v[1:2], s4
	s_mov_b32 s5, 0x3fe55555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, 0x3ff00000, v2, vcc_lo
	v_cndmask_b32_e32 v1, 0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[1:2]
	s_and_b32 s4, vcc_lo, exec_lo
	s_cselect_b32 s9, s9, 0x3ff00000
	s_cselect_b32 s8, s8, 0
	s_mov_b32 s4, 0x55555555
	v_frexp_mant_f64_e64 v[3:4], |s[8:9]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[3:4]
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	v_ldexp_f64 v[3:4], v[3:4], v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[3:4], 1.0
	v_add_f64 v[11:12], v[3:4], -1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	v_add_f64 v[13:14], v[5:6], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	v_mul_f64 v[15:16], v[5:6], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[15:16]
	v_fma_f64 v[3:4], v[9:10], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[3:4]
	v_add_f64 v[13:14], v[11:12], -v[5:6]
	v_add_f64 v[15:16], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	v_add_f64 v[3:4], v[15:16], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	v_add_f64 v[3:4], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[13:14], v[3:4]
	v_mul_f64 v[3:4], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[3:4]
	v_add_f64 v[7:8], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	v_fma_f64 v[7:8], v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[3:4], v[3:4]
	v_fma_f64 v[7:8], v[5:6], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[9:10], v[7:8]
	v_fma_f64 v[13:14], v[11:12], s[40:41], s[38:39]
	s_mov_b32 s38, 0x3abe935a
	s_mov_b32 s39, 0x3fbe25e4
	v_add_f64 v[9:10], v[11:12], -v[9:10]
	v_mul_f64 v[19:20], v[5:6], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x47e6c9c2
	s_mov_b32 s39, 0x3fc110ef
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0xcfa74449
	s_mov_b32 s39, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x71bf3c30
	s_mov_b32 s39, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x1c7792ce
	s_mov_b32 s39, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x924920da
	s_mov_b32 s39, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_mov_b32 s38, 0x9999999c
	s_mov_b32 s39, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[38:39]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[11:12], v[13:14]
	v_fma_f64 v[9:10], v[11:12], v[13:14], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	v_add_f64 v[13:14], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[17:18], v[13:14], s[4:5]
	v_add_f64 v[15:16], v[13:14], -v[15:16]
	s_mov_b32 s5, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[21:22], v[17:18], s[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_fma_f64 v[15:16], v[11:12], v[5:6], -v[19:20]
	s_mov_b32 s4, 0xd5df274d
	s_mov_b32 s5, 0x3c8543b0
	v_add_f64 v[13:14], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], s[4:5]
	v_fma_f64 v[11:12], v[11:12], v[3:4], v[15:16]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[13:14]
	v_fma_f64 v[7:8], v[7:8], v[5:6], v[11:12]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[17:18], v[9:10]
	v_add_f64 v[13:14], v[19:20], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[17:18], -v[11:12]
	v_mul_f64 v[17:18], v[13:14], v[11:12]
	v_add_f64 v[19:20], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[15:16]
	v_fma_f64 v[15:16], v[13:14], v[11:12], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[19:20]
	v_fma_f64 v[9:10], v[13:14], v[9:10], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[7:8], v[11:12], v[9:10]
	v_frexp_exp_i32_f64_e32 v11, s[8:9]
	v_add_f64 v[9:10], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	v_cvt_f64_i32_e32 v[11:12], v11
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[5:6], v[9:10]
	v_add_f64 v[15:16], v[9:10], -v[17:18]
	v_mul_f64 v[17:18], v[11:12], s[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[13:14], -v[5:6]
	v_add_f64 v[7:8], v[7:8], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[11:12], s[12:13], -v[17:18]
	s_mov_b32 s13, 0xbfe62e42
	v_add_f64 v[5:6], v[9:10], -v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_fma_f64 v[7:8], v[11:12], s[6:7], v[15:16]
	s_mov_b32 s7, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], v[3:4]
	v_add_f64 v[17:18], v[5:6], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[5:6], v[9:10]
	v_add_f64 v[13:14], v[9:10], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[11:12], -v[5:6]
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[13:14], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[19:20]
	v_add_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[13:14], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[9:10]
	v_add_f64 v[3:4], v[3:4], -v[9:10]
	v_add_f64 v[15:16], v[11:12], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[9:10], v[15:16], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], -v[15:16]
	v_mul_f64 v[9:10], v[1:2], v[5:6]
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[5:6], v[1:2], v[5:6], -v[9:10]
	v_cmp_class_f64_e64 vcc_lo, v[9:10], 0x204
	v_fma_f64 v[3:4], v[1:2], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[3:4]
	v_dual_cndmask_b32 v8, v6, v10 :: v_dual_cndmask_b32 v7, v5, v9
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[1:2], 0.5
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[11:12], v[7:8], s[4:5]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[7:8]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[7:8]
	v_cmp_neq_f64_e64 s5, 0x7ff00000, |v[7:8]|
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	v_trunc_f64_e32 v[5:6], v[1:2]
	v_rndne_f64_e32 v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v4, 0, v4, s5
	v_cndmask_b32_e64 v3, 0, v3, s5
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[11:12], s[12:13], v[7:8]
	v_cvt_i32_f64_e32 v17, v[11:12]
	v_trunc_f64_e32 v[7:8], v[9:10]
	v_fma_f64 v[13:14], v[11:12], s[6:7], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s5, v[7:8], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[14:15], s[10:11]
	v_cmp_class_f64_e64 s11, s[8:9], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[16:17]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[20:21]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[24:25]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[34:35]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], 1.0
	v_fma_f64 v[11:12], v[13:14], v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[11:12], v[11:12], v17
	v_cndmask_b32_e32 v12, 0x7ff00000, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s6, v11
	v_cndmask_b32_e64 v12, 0, v12, s4
	s_and_b32 s4, s4, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s6, s6, 0
	v_cmp_eq_f64_e64 s4, v[5:6], v[1:2]
	v_mov_b32_e32 v11, s6
	v_fma_f64 v[3:4], v[11:12], v[3:4], v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v3
	s_and_b32 s10, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v4, v12, vcc_lo
	v_cmp_lt_f64_e64 vcc_lo, s[8:9], 0
	v_cmp_eq_f64_e64 s10, s[8:9], 0
	s_cselect_b32 s6, s6, s7
	s_and_b32 s7, s4, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s5, s7, exec_lo
	s_cselect_b32 s5, s9, 0x3ff00000
	v_bfi_b32 v3, 0x7fffffff, v3, s5
	v_cmp_gt_f64_e64 s5, 0, v[1:2]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s4
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, s6, 0
	s_and_b32 s12, vcc_lo, exec_lo
	s_cselect_b32 s6, s4, s6
	v_cndmask_b32_e32 v1, v3, v4, vcc_lo
	s_or_b32 vcc_lo, s10, s11
	s_xor_b32 s4, s5, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, 0, 0x7ff00000
	s_and_b32 s5, s7, exec_lo
	s_cselect_b32 s5, s9, 0
	v_mov_b32_e32 v2, s5
	s_and_b32 s5, vcc_lo, exec_lo
	s_cselect_b32 s5, 0, s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_bfi_b32 v2, 0x7fffffff, s4, v2
	v_cmp_o_f64_e64 s4, s[8:9], s[8:9]
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v6, 0x7ff80000, v1, s4
	s_and_b32 s4, s4, exec_lo
	s_cselect_b32 s4, s5, 0
	v_mov_b32_e32 v5, s4
.LBB5_5:
	s_clause 0x1
	s_load_b256 s[12:19], s[0:1], 0x48
	s_load_b256 s[20:27], s[0:1], 0x0
	s_mul_i32 s4, s31, s29
	s_ashr_i32 s35, s31, 31
	s_mul_hi_i32 s37, s33, s4
	s_mul_i32 s36, s33, s4
	v_cmp_gt_i32_e64 s4, s31, v0
	s_mul_hi_u32 s42, s47, s50
	s_mov_b32 s34, s31
	s_mul_hi_i32 s39, s48, s31
	s_mul_i32 s38, s48, s31
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s18, 0
	s_cselect_b32 s6, -1, 0
	s_and_saveexec_b32 s5, s4
	s_cbranch_execz .LBB5_11
; %bb.6:
	s_load_b32 s29, s[0:1], 0x74
	s_lshl_b64 s[8:9], s[36:37], 3
	s_mul_hi_i32 s11, s34, s2
	s_add_u32 s7, s20, s8
	s_addc_u32 s20, s21, s9
	s_lshl_b64 s[8:9], s[38:39], 3
	s_mul_i32 s10, s34, s2
	s_add_u32 s7, s7, s8
	s_addc_u32 s8, s20, s9
	s_lshl_b64 s[10:11], s[10:11], 3
	v_lshl_add_u32 v4, v0, 3, 0x220
	v_mov_b32_e32 v1, v0
	s_add_u32 s9, s16, s10
	s_addc_u32 s10, s17, s11
	s_lshl_b32 s20, s34, 3
	s_waitcnt lgkmcnt(0)
	s_and_b32 s11, s29, 0xffff
	s_mov_b32 s29, 0
	s_lshl_b32 s21, s11, 3
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB5_9
	.p2align	6
.LBB5_7:                                ;   in Loop: Header=BB5_9 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s9, v2
	v_add_co_ci_u32_e64 v8, null, s10, v3, vcc_lo
	global_load_b64 v[7:8], v[7:8], off
.LBB5_8:                                ;   in Loop: Header=BB5_9 Depth=1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s7, v2
	v_add_co_ci_u32_e64 v3, null, s8, v3, vcc_lo
	v_add_nc_u32_e32 v1, s11, v1
	s_waitcnt vmcnt(0)
	ds_store_b64 v4, v[7:8]
	v_add_nc_u32_e32 v7, s20, v4
	global_load_b64 v[2:3], v[2:3], off
	v_add_nc_u32_e32 v4, s21, v4
	v_cmp_le_i32_e32 vcc_lo, s34, v1
	s_or_b32 s29, vcc_lo, s29
	s_waitcnt vmcnt(0)
	ds_store_b64 v7, v[2:3]
	s_and_not1_b32 exec_lo, exec_lo, s29
	s_cbranch_execz .LBB5_11
.LBB5_9:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	s_and_not1_b32 vcc_lo, exec_lo, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	s_cbranch_vccz .LBB5_7
; %bb.10:                               ;   in Loop: Header=BB5_9 Depth=1
	v_mov_b32_e32 v7, 0
	v_mov_b32_e32 v8, 0
	s_branch .LBB5_8
.LBB5_11:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s5
	v_cmp_eq_u32_e64 s5, 0, v0
	s_ashr_i32 s43, s48, 31
	s_ashr_i32 s44, s49, 31
	s_and_saveexec_b32 s10, s5
	s_cbranch_execz .LBB5_16
; %bb.12:
	s_and_not1_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccnz .LBB5_14
; %bb.13:
	s_lshl_b64 s[6:7], s[2:3], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s8, s12, s6
	s_addc_u32 s9, s13, s7
	s_add_u32 s20, s14, s6
	s_addc_u32 s21, s15, s7
	s_load_b64 s[6:7], s[8:9], 0x0
	s_load_b64 s[8:9], s[20:21], 0x0
	s_branch .LBB5_15
.LBB5_14:
	s_mov_b64 s[8:9], 0
	s_mov_b32 s7, 0xfe37e43c
	s_mov_b32 s6, 0x8800759c
.LBB5_15:
	s_waitcnt lgkmcnt(0)
	v_mov_b32_e32 v1, s6
	v_mov_b32_e32 v3, s8
	v_dual_mov_b32 v7, 0 :: v_dual_mov_b32 v2, s7
	v_mov_b32_e32 v4, s9
	ds_store_2addr_b64 v7, v[1:2], v[3:4] offset0:65 offset1:66
.LBB5_16:
	s_or_b32 exec_lo, exec_lo, s10
	s_lshl_b64 s[6:7], s[34:35], 4
	s_mul_hi_i32 s11, s30, s34
	s_mul_i32 s10, s30, s34
	s_sub_u32 s8, 0x400000, s6
	s_subb_u32 s9, 0, s7
	s_lshl_b64 s[6:7], s[10:11], 4
	s_waitcnt lgkmcnt(0)
	s_or_b64 s[10:11], s[8:9], s[6:7]
	s_mov_b32 s10, 0
	s_barrier
	s_cmp_lg_u64 s[10:11], 0
	buffer_gl0_inv
	s_cbranch_scc0 .LBB5_47
; %bb.17:
	s_ashr_i32 s20, s7, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_add_u32 s40, s6, s20
	s_mov_b32 s21, s20
	s_addc_u32 s41, s7, s20
	s_xor_b64 s[40:41], s[40:41], s[20:21]
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v1, s40
	v_cvt_f32_u32_e32 v2, s41
	s_sub_u32 s29, 0, s40
	s_subb_u32 s35, 0, s41
	v_fmamk_f32 v1, v2, 0x4f800000, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x5f7ffffc, v1
	v_mul_f32_e32 v2, 0x2f800000, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v2, v2
	v_fmamk_f32 v1, v2, 0xcf800000, v1
	v_cvt_u32_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s7, v2
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s11, v1
	s_mul_i32 s45, s29, s7
	s_mul_hi_u32 s49, s29, s11
	s_mul_i32 s48, s35, s11
	s_add_i32 s45, s49, s45
	s_mul_i32 s50, s29, s11
	s_add_i32 s45, s45, s48
	s_mul_hi_u32 s49, s11, s50
	s_mul_i32 s52, s11, s45
	s_mul_hi_u32 s51, s7, s50
	s_mul_i32 s48, s7, s50
	s_mul_hi_u32 s50, s11, s45
	s_add_u32 s49, s49, s52
	s_addc_u32 s50, 0, s50
	s_mul_hi_u32 s53, s7, s45
	s_add_u32 s48, s49, s48
	s_mul_i32 s45, s7, s45
	s_addc_u32 s48, s50, s51
	s_addc_u32 s49, s53, 0
	s_add_u32 s45, s48, s45
	s_addc_u32 s48, 0, s49
	s_add_u32 s11, s11, s45
	s_cselect_b32 s45, -1, 0
	s_mul_hi_u32 s49, s29, s11
	s_cmp_lg_u32 s45, 0
	s_mul_i32 s45, s29, s11
	s_addc_u32 s7, s7, s48
	s_mul_i32 s35, s35, s11
	s_mul_i32 s29, s29, s7
	s_mul_hi_u32 s48, s11, s45
	s_add_i32 s29, s49, s29
	s_mul_hi_u32 s49, s7, s45
	s_add_i32 s29, s29, s35
	s_mul_i32 s35, s7, s45
	s_mul_i32 s51, s11, s29
	s_mul_hi_u32 s50, s11, s29
	s_add_u32 s48, s48, s51
	s_addc_u32 s50, 0, s50
	s_mul_hi_u32 s45, s7, s29
	s_add_u32 s35, s48, s35
	s_mul_i32 s29, s7, s29
	s_addc_u32 s35, s50, s49
	s_addc_u32 s45, s45, 0
	s_add_u32 s29, s35, s29
	s_addc_u32 s35, 0, s45
	s_add_u32 s11, s11, s29
	s_cselect_b32 s29, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s29, 0
	s_addc_u32 s7, s7, s35
	s_ashr_i32 s48, s9, 31
	s_add_u32 s50, s8, s48
	s_mov_b32 s49, s48
	s_addc_u32 s51, s9, s48
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b64 s[50:51], s[50:51], s[48:49]
	s_mul_i32 s29, s50, s7
	s_mul_hi_u32 s35, s50, s11
	s_mul_hi_u32 s9, s50, s7
	s_mul_hi_u32 s52, s51, s11
	s_mul_i32 s11, s51, s11
	s_add_u32 s29, s35, s29
	s_addc_u32 s9, 0, s9
	s_mul_hi_u32 s45, s51, s7
	s_add_u32 s11, s29, s11
	s_mul_i32 s7, s51, s7
	s_addc_u32 s9, s9, s52
	s_addc_u32 s11, s45, 0
	s_add_u32 s7, s9, s7
	s_addc_u32 s9, 0, s11
	s_mul_hi_u32 s11, s40, s7
	s_mul_i32 s29, s40, s9
	s_mul_i32 s35, s41, s7
	s_add_i32 s11, s11, s29
	s_mul_i32 s29, s40, s7
	s_add_i32 s11, s11, s35
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_sub_i32 s35, s51, s11
	s_sub_u32 s29, s50, s29
	s_cselect_b32 s45, -1, 0
	s_cmp_lg_u32 s45, 0
	s_subb_u32 s35, s35, s41
	s_sub_u32 s50, s29, s40
	s_cselect_b32 s52, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s52, 0
	s_subb_u32 s35, s35, 0
	s_cmp_ge_u32 s35, s41
	s_cselect_b32 s52, -1, 0
	s_cmp_ge_u32 s50, s40
	s_cselect_b32 s50, -1, 0
	s_cmp_eq_u32 s35, s41
	s_cselect_b32 s35, s50, s52
	s_add_u32 s50, s7, 1
	s_addc_u32 s52, s9, 0
	s_add_u32 s53, s7, 2
	s_addc_u32 s54, s9, 0
	s_cmp_lg_u32 s35, 0
	s_cselect_b32 s35, s53, s50
	s_cselect_b32 s50, s54, s52
	s_cmp_lg_u32 s45, 0
	s_subb_u32 s11, s51, s11
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_ge_u32 s11, s41
	s_cselect_b32 s45, -1, 0
	s_cmp_ge_u32 s29, s40
	s_cselect_b32 s29, -1, 0
	s_cmp_eq_u32 s11, s41
	s_cselect_b32 s11, s29, s45
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s11, 0
	s_cselect_b32 s41, s50, s9
	s_cselect_b32 s40, s35, s7
	s_xor_b64 s[20:21], s[48:49], s[20:21]
	s_xor_b64 s[40:41], s[40:41], s[20:21]
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_u32 s20, s40, s20
	s_subb_u32 s21, s41, s21
	s_and_not1_b32 vcc_lo, exec_lo, s10
	s_cbranch_vccnz .LBB5_19
.LBB5_18:
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s9, 0, s6
	s_mov_b32 s21, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s7, v1
	s_mul_i32 s9, s9, s7
	s_mul_hi_u32 s9, s7, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s7, s7, s9
	s_mul_hi_u32 s7, s8, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s9, s7, s6
	s_sub_i32 s8, s8, s9
	s_add_i32 s9, s7, 1
	s_sub_i32 s10, s8, s6
	s_cmp_ge_u32 s8, s6
	s_cselect_b32 s7, s9, s7
	s_cselect_b32 s8, s10, s8
	s_add_i32 s9, s7, 1
	s_cmp_ge_u32 s8, s6
	s_cselect_b32 s20, s9, s7
.LBB5_19:
	s_cmp_lt_i32 s28, 1
	s_cbranch_scc1 .LBB5_41
; %bb.20:
	s_ashr_i32 s29, s28, 31
	v_cmp_gt_i64_e64 s9, s[20:21], 0
	v_cmp_lt_i64_e64 s8, s[20:21], s[28:29]
	s_mul_i32 s10, s42, s46
	s_load_b64 s[6:7], s[0:1], 0x40
	v_dual_mov_b32 v16, 0 :: v_dual_and_b32 v9, 31, v0
	v_lshlrev_b32_e32 v1, 3, v0
	s_and_b32 s8, s8, exec_lo
	s_cselect_b32 s8, s20, s28
	s_and_b32 s9, s9, exec_lo
	s_cselect_b32 s11, s8, 1
	s_sub_i32 s9, s47, s10
	s_xor_b32 s8, s43, s44
	s_add_i32 s10, s42, 1
	s_sub_i32 s20, s9, s46
	s_cmp_ge_u32 s9, s46
	v_mbcnt_lo_u32_b32 v11, -1, 0
	s_cselect_b32 s10, s10, s42
	s_cselect_b32 s9, s20, s9
	s_add_i32 s20, s10, 1
	s_cmp_ge_u32 s9, s46
	v_lshrrev_b32_e32 v10, 2, v0
	s_cselect_b32 s9, s20, s10
	s_waitcnt lgkmcnt(0)
	s_add_i32 s33, s33, s6
	s_xor_b32 s9, s9, s8
	v_cmp_eq_u32_e64 s6, 0, v9
	s_sub_i32 s8, s9, s8
	v_lshlrev_b32_e32 v12, 3, v9
	s_mul_hi_i32 s9, s8, s34
	s_mul_i32 s8, s8, s34
	v_add_nc_u32_e32 v14, 0x220, v1
	s_lshl_b64 s[8:9], s[8:9], 3
	v_lshl_or_b32 v15, v11, 2, 64
	s_add_u32 s62, s22, s8
	s_addc_u32 s63, s23, s9
	s_add_u32 s20, s0, 0x68
	s_addc_u32 s21, s1, 0
	s_cmp_lt_i32 s33, s7
	s_mul_i32 s29, s31, s30
	s_cselect_b32 s64, -1, 0
	s_add_u32 s65, s24, s8
	s_addc_u32 s66, s25, s9
	s_lshl_b32 s7, s34, 3
	s_mov_b32 s22, 0x652b82fe
	v_add3_u32 v13, 0x220, s7, v1
	s_mov_b32 s24, 0xfefa39ef
	s_mov_b32 s30, 0x3b39803f
	s_mov_b32 s40, 0xfca7ab0c
	s_mov_b32 s42, 0x6a5dcb37
	s_mov_b32 s44, 0x623fde64
	s_mov_b32 s46, 0x7c89e6b0
	s_mov_b32 s48, 0x14761f6e
	s_mov_b32 s50, 0x1852b7b0
	s_mov_b32 s52, 0x11122322
	s_mov_b32 s54, 0x555502a1
	s_mov_b32 s56, 0x55555511
	s_mov_b32 s58, 11
	s_mov_b32 s35, 0
	s_mov_b32 s23, 0x3ff71547
	s_mov_b32 s25, 0xbfe62e42
	s_mov_b32 s31, 0xbc7abc9e
	s_mov_b32 s41, 0x3e928af3
	s_mov_b32 s43, 0x3e5ade15
	s_mov_b32 s45, 0x3ec71dee
	s_mov_b32 s47, 0x3efa0199
	s_mov_b32 s49, 0x3f2a01a0
	s_mov_b32 s51, 0x3f56c16c
	s_mov_b32 s53, 0x3f811111
	s_mov_b32 s55, 0x3fa55555
	s_mov_b32 s57, 0x3fc55555
	s_mov_b32 s59, 0x3fe00000
	v_cmp_gt_u32_e32 vcc_lo, 32, v0
	s_branch .LBB5_22
.LBB5_21:                               ;   in Loop: Header=BB5_22 Depth=1
	s_cmp_lt_i32 s35, s28
	s_cbranch_scc0 .LBB5_41
.LBB5_22:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB5_25 Depth 2
                                        ;       Child Loop BB5_27 Depth 3
                                        ;       Child Loop BB5_40 Depth 3
	s_mov_b32 s67, s35
	s_add_i32 s35, s35, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_min_i32 s68, s35, s28
	s_cmp_ge_i32 s67, s68
	s_cbranch_scc1 .LBB5_21
; %bb.23:                               ;   in Loop: Header=BB5_22 Depth=1
	v_cmp_gt_u32_e64 s7, 24, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v1, 0, 8, s7
	v_cmp_gt_u32_e64 s7, 28, v11
	v_add_lshl_u32 v17, v1, v11, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, 0, 4, s7
	v_cmp_gt_u32_e64 s7, 30, v11
	v_add_lshl_u32 v18, v2, v11, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v3, 0, 2, s7
	v_cmp_ne_u32_e64 s7, 31, v11
	v_add_lshl_u32 v19, v3, v11, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v11, s7
	v_lshlrev_b32_e32 v20, 2, v4
	s_branch .LBB5_25
.LBB5_24:                               ;   in Loop: Header=BB5_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s8
	s_add_i32 s67, s67, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s67, s68
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB5_21
.LBB5_25:                               ;   Parent Loop BB5_22 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB5_27 Depth 3
                                        ;       Child Loop BB5_40 Depth 3
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mul_hi_i32 s61, s67, s29
	s_mul_i32 s60, s67, s29
	s_and_saveexec_b32 s8, s4
	s_cbranch_execz .LBB5_29
; %bb.26:                               ;   in Loop: Header=BB5_25 Depth=2
	s_load_b32 s7, s[20:21], 0xc
	s_lshl_b64 s[70:71], s[60:61], 3
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v7, v13
	v_mov_b32_e32 v3, v0
	s_add_u32 s9, s62, s70
	s_addc_u32 s10, s63, s71
	s_mov_b32 s70, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s69, s7, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s71, s69, 3
	.p2align	6
.LBB5_27:                               ;   Parent Loop BB5_22 Depth=1
                                        ;     Parent Loop BB5_25 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v4, 31, v3
	ds_load_b64 v[23:24], v7
	v_add_nc_u32_e32 v7, s71, v7
	v_lshlrev_b64 v[21:22], 3, v[3:4]
	v_add_nc_u32_e32 v3, s69, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v21, s7, s9, v21
	v_add_co_ci_u32_e64 v22, null, s10, v22, s7
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s7, s34, v3
	global_load_b64 v[21:22], v[21:22], off
	s_or_b32 s70, s7, s70
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[1:2], v[23:24], v[21:22], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s70
	s_cbranch_execnz .LBB5_27
; %bb.28:                               ;   in Loop: Header=BB5_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s70
.LBB5_29:                               ;   in Loop: Header=BB5_25 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s8
	ds_bpermute_b32 v3, v15, v1
	ds_bpermute_b32 v4, v15, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v17, v1
	ds_bpermute_b32 v4, v17, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v18, v1
	ds_bpermute_b32 v4, v18, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v19, v1
	ds_bpermute_b32 v4, v19, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v20, v1
	ds_bpermute_b32 v4, v20, v2
	s_and_saveexec_b32 s7, s6
	s_cbranch_execz .LBB5_31
; %bb.30:                               ;   in Loop: Header=BB5_25 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v10, v[1:2]
.LBB5_31:                               ;   in Loop: Header=BB5_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s7
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s8, vcc_lo
	s_cbranch_execz .LBB5_36
; %bb.32:                               ;   in Loop: Header=BB5_25 Depth=2
	s_load_b32 s7, s[20:21], 0xc
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s9, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s7, s7, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s7, s7, 31
	s_lshr_b32 s7, s7, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s7, v9
; %bb.33:                               ;   in Loop: Header=BB5_25 Depth=2
	ds_load_b64 v[1:2], v12
; %bb.34:                               ;   in Loop: Header=BB5_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s9
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v15, v1
	ds_bpermute_b32 v4, v15, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v17, v1
	ds_bpermute_b32 v4, v17, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v18, v1
	ds_bpermute_b32 v4, v18, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v19, v1
	ds_bpermute_b32 v4, v19, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v20, v1
	ds_bpermute_b32 v4, v20, v2
	s_and_b32 exec_lo, exec_lo, s6
	s_cbranch_execz .LBB5_36
; %bb.35:                               ;   in Loop: Header=BB5_25 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v16, v[1:2]
.LBB5_36:                               ;   in Loop: Header=BB5_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s8
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v16
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s69, s5
	s_cbranch_execz .LBB5_38
; %bb.37:                               ;   in Loop: Header=BB5_25 Depth=2
	s_add_i32 s7, s67, s18
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_cmp_lt_i32 s33, s7
	s_cselect_b32 s8, -1, 0
	s_sub_i32 s7, s33, s7
	v_cvt_f64_i32_e32 v[3:4], s7
	s_and_b32 s7, s64, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[5:6], v[3:4], v[1:2]
	ds_load_2addr_b64 v[1:4], v16 offset0:65 offset1:66
	v_cndmask_b32_e64 v8, v8, 0xc6293e59, s7
	v_cndmask_b32_e64 v7, v7, 0x39a08cea, s7
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e64 s7, v[7:8], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v24, v2, v8, s7
	v_cndmask_b32_e64 v23, v1, v7, s7
	v_add_f64 v[1:2], v[1:2], -v[23:24]
	v_add_f64 v[7:8], v[7:8], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[21:22], v[1:2], s[22:23]
	v_mul_f64 v[25:26], v[7:8], s[22:23]
	v_cmp_nlt_f64_e64 s7, 0x40900000, v[1:2]
	v_cmp_nlt_f64_e64 s9, 0x40900000, v[7:8]
	v_cmp_ngt_f64_e64 s8, 0xc090cc00, v[1:2]
	v_cmp_ngt_f64_e64 s10, 0xc090cc00, v[7:8]
	v_rndne_f64_e32 v[21:22], v[21:22]
	v_rndne_f64_e32 v[25:26], v[25:26]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[27:28], v[21:22], s[24:25], v[1:2]
	v_fma_f64 v[29:30], v[25:26], s[24:25], v[7:8]
	v_cvt_i32_f64_e32 v35, v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[27:28], v[21:22], s[30:31], v[27:28]
	v_fma_f64 v[29:30], v[25:26], s[30:31], v[29:30]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], s[42:43], s[40:41]
	v_fma_f64 v[33:34], v[29:30], s[42:43], s[40:41]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[44:45]
	v_fma_f64 v[33:34], v[29:30], v[33:34], s[44:45]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[46:47]
	v_fma_f64 v[33:34], v[29:30], v[33:34], s[46:47]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[48:49]
	v_fma_f64 v[33:34], v[29:30], v[33:34], s[48:49]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[50:51]
	v_fma_f64 v[33:34], v[29:30], v[33:34], s[50:51]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[52:53]
	v_fma_f64 v[33:34], v[29:30], v[33:34], s[52:53]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[54:55]
	v_fma_f64 v[33:34], v[29:30], v[33:34], s[54:55]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[56:57]
	v_fma_f64 v[33:34], v[29:30], v[33:34], s[56:57]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[58:59]
	v_fma_f64 v[33:34], v[29:30], v[33:34], s[58:59]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], v[27:28], v[31:32], 1.0
	v_fma_f64 v[33:34], v[29:30], v[33:34], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[21:22], v[27:28], v[31:32], 1.0
	v_cvt_i32_f64_e32 v27, v[25:26]
	v_fma_f64 v[25:26], v[29:30], v[33:34], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[21:22], v[21:22], v35
	v_ldexp_f64 v[25:26], v[25:26], v27
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v22, 0x7ff00000, v22, s7
	s_and_b32 s7, s8, s7
	v_cndmask_b32_e64 v1, 0x7ff00000, v26, s9
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v27, 0, v21, s7
	s_and_b32 s7, s10, s9
	v_cndmask_b32_e64 v28, 0, v22, s8
	v_cndmask_b32_e64 v21, 0, v25, s7
	v_cndmask_b32_e64 v22, 0, v1, s10
	v_fma_f64 v[25:26], v[3:4], v[27:28], v[21:22]
	ds_store_b128 v16, v[25:28] offset:528
	ds_store_b128 v16, v[21:24] offset:512
.LBB5_38:                               ;   in Loop: Header=BB5_25 Depth=2
	s_or_b32 exec_lo, exec_lo, s69
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s8, s4
	s_cbranch_execz .LBB5_24
; %bb.39:                               ;   in Loop: Header=BB5_25 Depth=2
	s_load_b32 s7, s[20:21], 0xc
	ds_load_2addr_b64 v[1:4], v16 offset0:64 offset1:67
	s_lshl_b64 s[60:61], s[60:61], 3
	v_mov_b32_e32 v21, v14
	v_mov_b32_e32 v7, v0
	s_add_u32 s9, s65, s60
	s_addc_u32 s10, s66, s61
	s_mov_b32 s69, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s60, s7, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s61, s60, 3
	.p2align	6
.LBB5_40:                               ;   Parent Loop BB5_22 Depth=1
                                        ;     Parent Loop BB5_25 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v8, 31, v7
	ds_load_b64 v[24:25], v21
	v_lshlrev_b64 v[22:23], 3, v[7:8]
	v_add_nc_u32_e32 v7, s60, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v22, s7, s9, v22
	v_add_co_ci_u32_e64 v23, null, s10, v23, s7
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s7, s34, v7
	global_load_b64 v[22:23], v[22:23], off
	s_or_b32 s69, s7, s69
	s_waitcnt vmcnt(0)
	v_mul_f64 v[22:23], v[1:2], v[22:23]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[22:23], v[24:25], v[3:4], v[22:23]
	ds_store_b64 v21, v[22:23]
	v_add_nc_u32_e32 v21, s61, v21
	s_and_not1_b32 exec_lo, exec_lo, s69
	s_cbranch_execnz .LBB5_40
	s_branch .LBB5_24
.LBB5_41:
	s_cmp_lg_u32 s19, 0
	s_cbranch_scc0 .LBB5_48
; %bb.42:
	s_and_saveexec_b32 s7, s4
	s_cbranch_execz .LBB5_45
; %bb.43:
	v_mov_b32_e32 v1, 0
	s_load_b32 s6, s[0:1], 0x74
	s_lshl_b64 s[8:9], s[36:37], 3
	v_lshl_add_u32 v5, v0, 3, 0x220
	s_add_u32 s10, s26, s8
	ds_load_b64 v[1:2], v1 offset:528
	s_addc_u32 s11, s27, s9
	s_lshl_b64 s[8:9], s[38:39], 3
	v_mov_b32_e32 v3, v0
	s_add_u32 s8, s10, s8
	s_addc_u32 s9, s11, s9
	s_mov_b32 s18, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s10, s6, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s11, s10, 3
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB5_44:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b64 v[6:7], v5
	v_ashrrev_i32_e32 v4, 31, v3
	v_add_nc_u32_e32 v5, s11, v5
	s_waitcnt lgkmcnt(0)
	v_div_scale_f64 v[8:9], null, v[1:2], v[1:2], v[6:7]
	v_div_scale_f64 v[14:15], vcc_lo, v[6:7], v[1:2], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_fma_f64 v[8:9], -v[8:9], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[12:13]
	v_div_fixup_f64 v[6:7], v[8:9], v[1:2], v[6:7]
	v_lshlrev_b64 v[8:9], 3, v[3:4]
	v_add_nc_u32_e32 v3, s10, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_i32_e32 vcc_lo, s34, v3
	v_add_co_u32 v8, s6, s8, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s9, v9, s6
	s_or_b32 s18, vcc_lo, s18
	global_store_b64 v[8:9], v[6:7], off
	s_and_not1_b32 exec_lo, exec_lo, s18
	s_cbranch_execnz .LBB5_44
.LBB5_45:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s7
	s_cbranch_execz .LBB5_49
.LBB5_46:
	s_endpgm
.LBB5_47:
                                        ; implicit-def: $sgpr20_sgpr21
	s_branch .LBB5_18
.LBB5_48:
.LBB5_49:
	s_and_saveexec_b32 s6, s4
	s_cbranch_execz .LBB5_52
; %bb.50:
	s_load_b32 s7, s[0:1], 0x74
	s_mul_hi_i32 s1, s34, s2
	s_mul_i32 s0, s34, s2
	v_lshl_add_u32 v2, v0, 3, 0x220
	s_lshl_b64 s[8:9], s[0:1], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_u32 s1, s16, s8
	s_addc_u32 s4, s17, s9
	s_mov_b32 s9, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s7, s7, 0xffff
	s_lshl_b32 s8, s7, 3
	.p2align	6
.LBB5_51:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b64 v[3:4], v2
	v_ashrrev_i32_e32 v1, 31, v0
	v_add_nc_u32_e32 v2, s8, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 3, v[0:1]
	v_add_nc_u32_e32 v0, s7, v0
	v_cmp_le_i32_e32 vcc_lo, s34, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, s0, s1, v5
	v_add_co_ci_u32_e64 v6, null, s4, v6, s0
	s_or_b32 s9, vcc_lo, s9
	s_waitcnt lgkmcnt(0)
	global_store_b64 v[5:6], v[3:4], off
	s_and_not1_b32 exec_lo, exec_lo, s9
	s_cbranch_execnz .LBB5_51
.LBB5_52:
	s_or_b32 exec_lo, exec_lo, s6
	s_and_saveexec_b32 s0, s5
	s_cbranch_execz .LBB5_46
; %bb.53:
	v_mov_b32_e32 v4, 0
	s_lshl_b64 s[0:1], s[2:3], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s2, s14, s0
	s_addc_u32 s3, s15, s1
	ds_load_2addr_b64 v[0:3], v4 offset0:65 offset1:66
	s_add_u32 s0, s12, s0
	s_addc_u32 s1, s13, s1
	s_waitcnt lgkmcnt(0)
	s_clause 0x1
	global_store_b64 v4, v[0:1], s[0:1]
	global_store_b64 v4, v[2:3], s[2:3]
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
		.amdhsa_group_segment_fixed_size 544
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 360
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
		.amdhsa_next_free_sgpr 72
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
		.amdhsa_inst_pref_size 63
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,"axG",@progbits,_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii,comdat
.Lfunc_end5:
	.size	_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii, .Lfunc_end5-_Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
                                        ; -- End function
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.num_vgpr, 36
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.num_agpr, 0
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.numbered_sgpr, 72
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.num_named_barrier, 0
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.private_seg_size, 0
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.uses_vcc, 1
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.uses_flat_scratch, 0
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.has_dyn_sized_stack, 0
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.has_recursion, 0
	.set _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 9928
; TotalNumSgprs: 74
; NumVgprs: 36
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 544 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 74
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
	.section	.text._Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,"axG",@progbits,_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,comdat
	.protected	_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii ; -- Begin function _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
	.globl	_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
	.p2align	8
	.type	_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,@function
_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii: ; @_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
; %bb.0:
	s_load_b256 s[12:19], s[0:1], 0x24
	s_abs_i32 s6, s2
	s_load_b256 s[20:27], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s4, s13
	s_ashr_i32 s7, s13, 31
	v_cvt_f32_u32_e32 v1, s4
	s_sub_i32 s5, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v1
	s_mul_i32 s5, s5, s3
	s_mul_hi_u32 s5, s3, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s3, s5
	s_ashr_i32 s3, s2, 31
	s_mul_hi_u32 s5, s6, s5
	s_xor_b32 s9, s3, s7
	s_mul_i32 s8, s5, s4
	s_sub_i32 s6, s6, s8
	s_add_i32 s8, s5, 1
	s_sub_i32 s10, s6, s4
	s_cmp_ge_u32 s6, s4
	s_cselect_b32 s5, s8, s5
	s_cselect_b32 s6, s10, s6
	s_add_i32 s8, s5, 1
	s_cmp_ge_u32 s6, s4
	s_cselect_b32 s5, s8, s5
	s_abs_i32 s6, s14
	s_xor_b32 s5, s5, s9
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s10, 0, s6
	s_sub_i32 s19, s5, s9
	s_ashr_i32 s11, s14, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_xor_b32 s7, s7, s11
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s8, v1
	s_mul_i32 s10, s10, s8
	s_mul_hi_u32 s10, s8, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s8, s8, s10
	s_mul_hi_u32 s5, s4, s8
	s_mul_i32 s8, s19, s13
	s_mul_i32 s9, s5, s6
	s_sub_i32 s33, s2, s8
	s_sub_i32 s4, s4, s9
	s_add_i32 s8, s5, 1
	s_sub_i32 s9, s4, s6
	s_cmp_ge_u32 s4, s6
	s_cselect_b32 s5, s8, s5
	s_cselect_b32 s4, s9, s4
	s_add_i32 s8, s5, 1
	s_cmp_ge_u32 s4, s6
	s_cselect_b32 s4, s8, s5
	s_abs_i32 s44, s33
	s_xor_b32 s4, s4, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_sub_i32 s5, s4, s7
	s_abs_i32 s7, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v1, s7
	s_sub_i32 s6, 0, s7
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s4, v1
	s_mul_i32 s8, s6, s4
	s_mov_b32 s6, 0
	s_mul_hi_u32 s8, s4, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s8, s4, s8
	v_cmp_gt_i32_e64 s4, s16, v0
	s_cmp_gt_i32 s26, 0
	s_mul_hi_u32 s45, s44, s8
	s_cselect_b32 s8, -1, 0
	s_and_saveexec_b32 s9, s4
	s_cbranch_execz .LBB6_6
; %bb.1:
	s_load_b32 s28, s[0:1], 0x6c
	s_mul_hi_i32 s11, s16, s2
	s_mul_i32 s10, s16, s2
	v_lshl_add_u32 v3, v0, 2, 0x110
	s_lshl_b64 s[10:11], s[10:11], 2
	v_mov_b32_e32 v1, v0
	s_add_u32 s10, s24, s10
	s_addc_u32 s11, s25, s11
	s_waitcnt lgkmcnt(0)
	s_and_b32 s28, s28, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s29, s28, 2
	s_branch .LBB6_4
	.p2align	6
.LBB6_2:                                ;   in Loop: Header=BB6_4 Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	v_add_co_u32 v4, vcc_lo, s10, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s11, v5, vcc_lo
	global_load_b32 v2, v[4:5], off
.LBB6_3:                                ;   in Loop: Header=BB6_4 Depth=1
	v_add_nc_u32_e32 v1, s28, v1
	s_waitcnt vmcnt(0)
	ds_store_b32 v3, v2
	v_add_nc_u32_e32 v3, s29, v3
	v_cmp_le_i32_e32 vcc_lo, s16, v1
	s_or_b32 s6, vcc_lo, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s6
	s_cbranch_execz .LBB6_6
.LBB6_4:                                ; =>This Inner Loop Header: Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s8
	s_cbranch_vccz .LBB6_2
; %bb.5:                                ;   in Loop: Header=BB6_4 Depth=1
	v_mov_b32_e32 v2, 0
	s_branch .LBB6_3
.LBB6_6:
	s_or_b32 exec_lo, exec_lo, s9
	s_ashr_i32 s46, s33, 31
	s_ashr_i32 s47, s5, 31
; %bb.7:
	s_load_b256 s[36:43], s[0:1], 0x0
	v_cmp_gt_i32_e64 s6, s15, v0
	v_lshlrev_b32_e32 v3, 2, v0
	s_and_saveexec_b32 s5, s6
	s_cbranch_execz .LBB6_10
; %bb.8:
	s_load_b32 s30, s[0:1], 0x6c
	s_mul_i32 s9, s15, s13
	s_mul_hi_i32 s29, s33, s15
	s_mul_hi_i32 s11, s19, s9
	s_mul_i32 s10, s19, s9
	s_mul_i32 s28, s33, s15
	s_lshl_b64 s[10:11], s[10:11], 2
	v_mov_b32_e32 v1, v0
	s_waitcnt lgkmcnt(0)
	s_add_u32 s9, s36, s10
	s_addc_u32 s31, s37, s11
	s_lshl_b64 s[10:11], s[28:29], 2
	s_mov_b32 s29, 0
	s_add_u32 s9, s9, s10
	s_addc_u32 s10, s31, s11
	s_lshl_b32 s11, s16, 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add3_u32 v4, 0x110, s11, v3
	s_and_b32 s11, s30, 0xffff
	s_lshl_b32 s28, s11, 2
	.p2align	6
.LBB6_9:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[5:6], 2, v[1:2]
	v_add_nc_u32_e32 v1, s11, v1
	v_add_co_u32 v5, vcc_lo, s9, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v6, null, s10, v6, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s15, v1
	global_load_b32 v2, v[5:6], off
	s_or_b32 s29, vcc_lo, s29
	s_waitcnt vmcnt(0)
	ds_store_b32 v4, v2
	v_add_nc_u32_e32 v4, s28, v4
	s_and_not1_b32 exec_lo, exec_lo, s29
	s_cbranch_execnz .LBB6_9
.LBB6_10:
	s_or_b32 exec_lo, exec_lo, s5
	v_cmp_eq_u32_e64 s5, 0, v0
	s_mov_b32 s10, 0
	s_and_saveexec_b32 s9, s5
	s_cbranch_execz .LBB6_15
; %bb.11:
	s_and_not1_b32 vcc_lo, exec_lo, s8
	s_cbranch_vccnz .LBB6_13
; %bb.12:
	s_lshl_b64 s[10:11], s[2:3], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s28, s20, s10
	s_addc_u32 s29, s21, s11
	s_add_u32 s10, s22, s10
	s_addc_u32 s11, s23, s11
	s_load_b32 s8, s[28:29], 0x0
	s_load_b32 s10, s[10:11], 0x0
	s_branch .LBB6_14
.LBB6_13:
	s_mov_b32 s8, 0xff800000
.LBB6_14:
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, s8
	v_mov_b32_e32 v4, s10
	ds_store_2addr_b32 v1, v2, v4 offset0:65 offset1:66
.LBB6_15:
	s_or_b32 exec_lo, exec_lo, s9
	s_add_i32 s8, s16, s15
	s_waitcnt lgkmcnt(0)
	s_ashr_i32 s9, s8, 31
	s_mul_hi_i32 s29, s14, s8
	s_lshl_b64 s[10:11], s[8:9], 2
	s_mul_i32 s28, s14, s8
	s_sub_u32 s10, 0x400000, s10
	s_subb_u32 s11, 0, s11
	s_lshl_b64 s[8:9], s[28:29], 2
	s_barrier
	s_or_b64 s[28:29], s[10:11], s[8:9]
	s_mov_b32 s28, 0
	buffer_gl0_inv
	s_cmp_lg_u64 s[28:29], 0
	s_cbranch_scc0 .LBB6_46
; %bb.16:
	s_ashr_i32 s30, s9, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_add_u32 s34, s8, s30
	s_mov_b32 s31, s30
	s_addc_u32 s35, s9, s30
	s_xor_b64 s[34:35], s[34:35], s[30:31]
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v1, s34
	v_cvt_f32_u32_e32 v2, s35
	s_sub_u32 s36, 0, s34
	s_subb_u32 s37, 0, s35
	v_fmamk_f32 v1, v2, 0x4f800000, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x5f7ffffc, v1
	v_mul_f32_e32 v2, 0x2f800000, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v2, v2
	v_fmamk_f32 v1, v2, 0xcf800000, v1
	v_cvt_u32_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s9, v2
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s29, v1
	s_mul_i32 s48, s36, s9
	s_mul_hi_u32 s50, s36, s29
	s_mul_i32 s49, s37, s29
	s_add_i32 s48, s50, s48
	s_mul_i32 s51, s36, s29
	s_add_i32 s48, s48, s49
	s_mul_hi_u32 s50, s29, s51
	s_mul_i32 s53, s29, s48
	s_mul_hi_u32 s52, s9, s51
	s_mul_i32 s49, s9, s51
	s_mul_hi_u32 s51, s29, s48
	s_add_u32 s50, s50, s53
	s_addc_u32 s51, 0, s51
	s_mul_hi_u32 s54, s9, s48
	s_add_u32 s49, s50, s49
	s_mul_i32 s48, s9, s48
	s_addc_u32 s49, s51, s52
	s_addc_u32 s50, s54, 0
	s_add_u32 s48, s49, s48
	s_addc_u32 s49, 0, s50
	s_add_u32 s29, s29, s48
	s_cselect_b32 s48, -1, 0
	s_mul_hi_u32 s50, s36, s29
	s_cmp_lg_u32 s48, 0
	s_mul_i32 s48, s36, s29
	s_addc_u32 s9, s9, s49
	s_mul_i32 s37, s37, s29
	s_mul_i32 s36, s36, s9
	s_mul_hi_u32 s49, s29, s48
	s_add_i32 s36, s50, s36
	s_mul_hi_u32 s50, s9, s48
	s_add_i32 s36, s36, s37
	s_mul_i32 s37, s9, s48
	s_mul_i32 s52, s29, s36
	s_mul_hi_u32 s51, s29, s36
	s_add_u32 s49, s49, s52
	s_addc_u32 s51, 0, s51
	s_mul_hi_u32 s48, s9, s36
	s_add_u32 s37, s49, s37
	s_mul_i32 s36, s9, s36
	s_addc_u32 s37, s51, s50
	s_addc_u32 s48, s48, 0
	s_add_u32 s36, s37, s36
	s_addc_u32 s37, 0, s48
	s_add_u32 s29, s29, s36
	s_cselect_b32 s36, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s36, 0
	s_addc_u32 s9, s9, s37
	s_ashr_i32 s36, s11, 31
	s_add_u32 s48, s10, s36
	s_mov_b32 s37, s36
	s_addc_u32 s49, s11, s36
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b64 s[48:49], s[48:49], s[36:37]
	s_mul_i32 s50, s48, s9
	s_mul_hi_u32 s51, s48, s29
	s_mul_hi_u32 s11, s48, s9
	s_mul_hi_u32 s53, s49, s29
	s_mul_i32 s29, s49, s29
	s_add_u32 s50, s51, s50
	s_addc_u32 s11, 0, s11
	s_mul_hi_u32 s52, s49, s9
	s_add_u32 s29, s50, s29
	s_mul_i32 s9, s49, s9
	s_addc_u32 s11, s11, s53
	s_addc_u32 s29, s52, 0
	s_add_u32 s9, s11, s9
	s_addc_u32 s11, 0, s29
	s_mul_hi_u32 s29, s34, s9
	s_mul_i32 s50, s34, s11
	s_mul_i32 s51, s35, s9
	s_add_i32 s29, s29, s50
	s_mul_i32 s50, s34, s9
	s_add_i32 s29, s29, s51
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_sub_i32 s51, s49, s29
	s_sub_u32 s48, s48, s50
	s_cselect_b32 s50, -1, 0
	s_cmp_lg_u32 s50, 0
	s_subb_u32 s51, s51, s35
	s_sub_u32 s52, s48, s34
	s_cselect_b32 s53, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s53, 0
	s_subb_u32 s51, s51, 0
	s_cmp_ge_u32 s51, s35
	s_cselect_b32 s53, -1, 0
	s_cmp_ge_u32 s52, s34
	s_cselect_b32 s52, -1, 0
	s_cmp_eq_u32 s51, s35
	s_cselect_b32 s51, s52, s53
	s_add_u32 s52, s9, 1
	s_addc_u32 s53, s11, 0
	s_add_u32 s54, s9, 2
	s_addc_u32 s55, s11, 0
	s_cmp_lg_u32 s51, 0
	s_cselect_b32 s51, s54, s52
	s_cselect_b32 s52, s55, s53
	s_cmp_lg_u32 s50, 0
	s_subb_u32 s29, s49, s29
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_ge_u32 s29, s35
	s_cselect_b32 s49, -1, 0
	s_cmp_ge_u32 s48, s34
	s_cselect_b32 s34, -1, 0
	s_cmp_eq_u32 s29, s35
	s_cselect_b32 s29, s34, s49
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s29, 0
	s_cselect_b32 s35, s52, s11
	s_cselect_b32 s34, s51, s9
	s_xor_b64 s[30:31], s[36:37], s[30:31]
	s_xor_b64 s[34:35], s[34:35], s[30:31]
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_u32 s30, s34, s30
	s_subb_u32 s31, s35, s31
	s_and_not1_b32 vcc_lo, exec_lo, s28
	s_cbranch_vccnz .LBB6_18
.LBB6_17:
	v_cvt_f32_u32_e32 v1, s8
	s_sub_i32 s11, 0, s8
	s_mov_b32 s31, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s9, v1
	s_mul_i32 s11, s11, s9
	s_mul_hi_u32 s11, s9, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s9, s9, s11
	s_mul_hi_u32 s9, s10, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s11, s9, s8
	s_sub_i32 s10, s10, s11
	s_add_i32 s11, s9, 1
	s_sub_i32 s28, s10, s8
	s_cmp_ge_u32 s10, s8
	s_cselect_b32 s9, s11, s9
	s_cselect_b32 s10, s28, s10
	s_add_i32 s11, s9, 1
	s_cmp_ge_u32 s10, s8
	s_cselect_b32 s30, s11, s9
.LBB6_18:
	s_cmp_lt_i32 s12, 1
	s_cbranch_scc1 .LBB6_40
; %bb.19:
	s_ashr_i32 s9, s12, 31
	s_mov_b32 s8, s12
	s_mul_i32 s10, s45, s7
	v_cmp_lt_i64_e64 s8, s[30:31], s[8:9]
	v_cmp_gt_i64_e64 s9, s[30:31], 0
	v_dual_mov_b32 v12, 0 :: v_dual_and_b32 v5, 31, v0
	v_lshrrev_b32_e32 v1, 3, v0
	v_mbcnt_lo_u32_b32 v7, -1, 0
	s_and_b32 s8, s8, exec_lo
	s_cselect_b32 s8, s30, s12
	s_and_b32 s9, s9, exec_lo
	s_cselect_b32 s9, s8, 1
	s_sub_i32 s10, s44, s10
	s_xor_b32 s8, s46, s47
	s_add_i32 s11, s45, 1
	s_sub_i32 s28, s10, s7
	s_cmp_ge_u32 s10, s7
	v_and_b32_e32 v6, 0x7c, v1
	s_cselect_b32 s11, s11, s45
	s_cselect_b32 s10, s28, s10
	s_add_i32 s28, s11, 1
	s_cmp_ge_u32 s10, s7
	v_lshlrev_b32_e32 v8, 2, v5
	s_cselect_b32 s7, s28, s11
	s_add_i32 s17, s19, s17
	s_xor_b32 s7, s7, s8
	v_add_nc_u32_e32 v10, 0x110, v3
	s_sub_i32 s8, s7, s8
	v_cmp_eq_u32_e64 s7, 0, v5
	s_mul_hi_i32 s11, s8, s15
	s_mul_i32 s10, s8, s15
	s_mul_hi_i32 s31, s8, s16
	s_lshl_b64 s[10:11], s[10:11], 2
	s_mul_i32 s30, s8, s16
	s_add_u32 s28, s38, s10
	s_addc_u32 s29, s39, s11
	s_add_u32 s10, s0, 0x60
	s_addc_u32 s11, s1, 0
	s_cmp_lt_i32 s17, s18
	v_lshl_or_b32 v11, v7, 2, 64
	s_cselect_b32 s18, -1, 0
	s_lshl_b64 s[30:31], s[30:31], 2
	s_mul_i32 s34, s15, s14
	s_add_u32 s30, s40, s30
	s_addc_u32 s31, s41, s31
	s_lshl_b32 s8, s16, 2
	s_mul_i32 s14, s16, s14
	v_add3_u32 v9, 0x110, s8, v3
	s_mov_b32 s35, 0
	v_cmp_gt_u32_e32 vcc_lo, 32, v0
	s_branch .LBB6_21
.LBB6_20:                               ;   in Loop: Header=BB6_21 Depth=1
	s_cmp_lt_i32 s35, s12
	s_cbranch_scc0 .LBB6_40
.LBB6_21:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB6_24 Depth 2
                                        ;       Child Loop BB6_26 Depth 3
                                        ;       Child Loop BB6_39 Depth 3
	s_mov_b32 s36, s35
	s_add_i32 s35, s35, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_min_i32 s37, s35, s12
	s_cmp_ge_i32 s36, s37
	s_cbranch_scc1 .LBB6_20
; %bb.22:                               ;   in Loop: Header=BB6_21 Depth=1
	v_cmp_gt_u32_e64 s8, 24, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v1, 0, 8, s8
	v_cmp_gt_u32_e64 s8, 28, v7
	v_add_lshl_u32 v13, v1, v7, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, 0, 4, s8
	v_cmp_gt_u32_e64 s8, 30, v7
	v_add_lshl_u32 v14, v2, v7, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v3, 0, 2, s8
	v_cmp_ne_u32_e64 s8, 31, v7
	v_add_lshl_u32 v15, v3, v7, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v7, s8
	v_lshlrev_b32_e32 v16, 2, v4
	s_branch .LBB6_24
.LBB6_23:                               ;   in Loop: Header=BB6_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s38
	s_add_i32 s36, s36, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s36, s37
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB6_20
.LBB6_24:                               ;   Parent Loop BB6_21 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB6_26 Depth 3
                                        ;       Child Loop BB6_39 Depth 3
	v_mov_b32_e32 v3, 0
	s_and_saveexec_b32 s38, s6
	s_cbranch_execz .LBB6_28
; %bb.25:                               ;   in Loop: Header=BB6_24 Depth=2
	s_load_b32 s8, s[10:11], 0xc
	s_mul_hi_i32 s41, s36, s34
	s_mul_i32 s40, s36, s34
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v4, v9
	s_lshl_b64 s[40:41], s[40:41], 2
	v_mov_b32_e32 v1, v0
	s_add_u32 s39, s28, s40
	s_addc_u32 s40, s29, s41
	s_mov_b32 s44, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s41, s8, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s45, s41, 2
	.p2align	6
.LBB6_26:                               ;   Parent Loop BB6_21 Depth=1
                                        ;     Parent Loop BB6_24 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[17:18], 2, v[1:2]
	v_add_nc_u32_e32 v1, s41, v1
	v_add_co_u32 v17, s8, s39, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v18, null, s40, v18, s8
	v_cmp_le_i32_e64 s8, s15, v1
	global_load_b32 v2, v[17:18], off
	ds_load_b32 v17, v4
	v_add_nc_u32_e32 v4, s45, v4
	s_or_b32 s44, s8, s44
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fmac_f32_e32 v3, v17, v2
	s_and_not1_b32 exec_lo, exec_lo, s44
	s_cbranch_execnz .LBB6_26
; %bb.27:                               ;   in Loop: Header=BB6_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s44
.LBB6_28:                               ;   in Loop: Header=BB6_24 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s38
	ds_bpermute_b32 v1, v11, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v3, v1
	ds_bpermute_b32 v2, v13, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v14, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v15, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v16, v1
	s_and_saveexec_b32 s8, s7
	s_cbranch_execz .LBB6_30
; %bb.29:                               ;   in Loop: Header=BB6_24 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v6, v1
.LBB6_30:                               ;   in Loop: Header=BB6_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s8
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s38, vcc_lo
	s_cbranch_execz .LBB6_35
; %bb.31:                               ;   in Loop: Header=BB6_24 Depth=2
	s_load_b32 s8, s[10:11], 0xc
	v_mov_b32_e32 v1, 0
	s_mov_b32 s39, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s8, s8, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s8, s8, 31
	s_lshr_b32 s8, s8, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s8, v5
; %bb.32:                               ;   in Loop: Header=BB6_24 Depth=2
	ds_load_b32 v1, v8
; %bb.33:                               ;   in Loop: Header=BB6_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s39
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v11, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v13, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v14, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v15, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v16, v1
	s_and_b32 exec_lo, exec_lo, s7
	s_cbranch_execz .LBB6_35
; %bb.34:                               ;   in Loop: Header=BB6_24 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v12, v1
.LBB6_35:                               ;   in Loop: Header=BB6_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s38
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v1, v12
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s38, s5
	s_cbranch_execz .LBB6_37
; %bb.36:                               ;   in Loop: Header=BB6_24 Depth=2
	ds_load_2addr_b32 v[17:18], v12 offset0:65 offset1:66
	s_add_i32 s8, s36, s26
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_cmp_gt_i32 s8, s17
	s_cselect_b32 s8, -1, 0
	s_and_b32 s8, s18, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v1, v1, 0xf149f2ca, s8
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f32_e64 s8, v1, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, v17, v1, s8
	v_sub_f32_e32 v3, v17, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v1, v1, v2 :: v_dual_mul_f32 v4, 0x3fb8aa3b, v3
	v_fma_f32 v19, 0x3fb8aa3b, v3, -v4
	v_rndne_f32_e32 v20, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_dual_sub_f32 v4, v4, v20 :: v_dual_fmac_f32 v19, 0x32a5705f, v3
	v_mul_f32_e32 v17, 0x3fb8aa3b, v1
	v_cmp_ngt_f32_e64 s8, 0xc2ce8ed0, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v4, v4, v19
	v_fma_f32 v21, 0x3fb8aa3b, v1, -v17
	v_rndne_f32_e32 v22, v17
	v_cvt_i32_f32_e32 v19, v20
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_exp_f32_e32 v4, v4
	v_fmac_f32_e32 v21, 0x32a5705f, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v17, v17, v22
	v_cvt_i32_f32_e32 v20, v22
	v_add_f32_e32 v17, v17, v21
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v4, v4, v19
	v_exp_f32_e32 v17, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0, v4, s8
	v_cmp_ngt_f32_e64 s8, 0xc2ce8ed0, v1
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v17, v17, v20
	v_cndmask_b32_e64 v17, 0, v17, s8
	v_cmp_nlt_f32_e64 s8, 0x42b17218, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0x7f800000, v4, s8
	v_cmp_nlt_f32_e64 s8, 0x42b17218, v1
	v_cndmask_b32_e64 v1, 0x7f800000, v17, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f32 v3, v18, v4, v1
	ds_store_b128 v12, v[1:4] offset:256
.LBB6_37:                               ;   in Loop: Header=BB6_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s38
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s38, s4
	s_cbranch_execz .LBB6_23
; %bb.38:                               ;   in Loop: Header=BB6_24 Depth=2
	s_load_b32 s8, s[10:11], 0xc
	ds_load_2addr_b32 v[1:2], v12 offset0:64 offset1:67
	s_mul_hi_i32 s41, s36, s14
	s_mul_i32 s40, s36, s14
	v_mov_b32_e32 v17, v10
	s_lshl_b64 s[40:41], s[40:41], 2
	v_mov_b32_e32 v3, v0
	s_add_u32 s39, s30, s40
	s_addc_u32 s40, s31, s41
	s_mov_b32 s45, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s41, s8, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s44, s41, 2
	.p2align	6
.LBB6_39:                               ;   Parent Loop BB6_21 Depth=1
                                        ;     Parent Loop BB6_24 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[18:19], 2, v[3:4]
	v_add_nc_u32_e32 v3, s41, v3
	v_add_co_u32 v18, s8, s39, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v19, null, s40, v19, s8
	v_cmp_le_i32_e64 s8, s16, v3
	global_load_b32 v4, v[18:19], off
	ds_load_b32 v18, v17
	s_or_b32 s45, s8, s45
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v4, v1, v4
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v4, v18, v2
	ds_store_b32 v17, v4
	v_add_nc_u32_e32 v17, s44, v17
	s_and_not1_b32 exec_lo, exec_lo, s45
	s_cbranch_execnz .LBB6_39
	s_branch .LBB6_23
.LBB6_40:
	s_cmp_lg_u32 s27, 0
	s_cbranch_scc0 .LBB6_47
; %bb.41:
	s_and_saveexec_b32 s7, s4
	s_cbranch_execz .LBB6_44
; %bb.42:
	v_mov_b32_e32 v1, 0
	s_load_b32 s6, s[0:1], 0x6c
	s_mul_i32 s8, s16, s13
	s_mul_hi_i32 s11, s33, s16
	s_mul_hi_i32 s9, s19, s8
	ds_load_b32 v3, v1 offset:264
	s_mul_i32 s8, s19, s8
	s_mul_i32 s10, s33, s16
	s_lshl_b64 s[8:9], s[8:9], 2
	v_lshl_add_u32 v4, v0, 2, 0x110
	s_add_u32 s12, s42, s8
	s_addc_u32 s13, s43, s9
	s_lshl_b64 s[8:9], s[10:11], 2
	v_mov_b32_e32 v1, v0
	s_add_u32 s8, s12, s8
	s_addc_u32 s9, s13, s9
	s_mov_b32 s12, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s10, s6, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s11, s10, 2
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB6_43:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b32 v7, v4
	v_ashrrev_i32_e32 v2, 31, v1
	v_add_nc_u32_e32 v4, s11, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 2, v[1:2]
	v_add_nc_u32_e32 v1, s10, v1
	v_cmp_le_i32_e64 s6, s16, v1
	s_or_b32 s12, s6, s12
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v8, null, v3, v3, v7
	v_div_scale_f32 v2, vcc_lo, v7, v3, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v9, v8
	s_waitcnt_depctr 0xfff
	v_fma_f32 v10, -v8, v9, 1.0
	v_fmac_f32_e32 v9, v10, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v10, v2, v9
	v_fma_f32 v11, -v8, v10, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v10, v11, v9
	v_fma_f32 v2, -v8, v10, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v9, v10
	v_add_co_u32 v5, vcc_lo, s8, v5
	v_add_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_div_fixup_f32 v2, v2, v3, v7
	global_store_b32 v[5:6], v2, off
	s_and_not1_b32 exec_lo, exec_lo, s12
	s_cbranch_execnz .LBB6_43
.LBB6_44:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s7
	s_cbranch_execz .LBB6_48
.LBB6_45:
	s_endpgm
.LBB6_46:
                                        ; implicit-def: $sgpr30_sgpr31
	s_branch .LBB6_17
.LBB6_47:
.LBB6_48:
	s_and_saveexec_b32 s6, s4
	s_cbranch_execz .LBB6_51
; %bb.49:
	s_load_b32 s7, s[0:1], 0x6c
	s_mul_hi_i32 s1, s16, s2
	s_mul_i32 s0, s16, s2
	v_lshl_add_u32 v2, v0, 2, 0x110
	s_lshl_b64 s[8:9], s[0:1], 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_u32 s1, s24, s8
	s_addc_u32 s4, s25, s9
	s_mov_b32 s9, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s7, s7, 0xffff
	s_lshl_b32 s8, s7, 2
	.p2align	6
.LBB6_50:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b32 v5, v2
	v_ashrrev_i32_e32 v1, 31, v0
	v_add_nc_u32_e32 v2, s8, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[0:1]
	v_add_nc_u32_e32 v0, s7, v0
	v_cmp_le_i32_e32 vcc_lo, s16, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, s0, s1, v3
	v_add_co_ci_u32_e64 v4, null, s4, v4, s0
	s_or_b32 s9, vcc_lo, s9
	s_waitcnt lgkmcnt(0)
	global_store_b32 v[3:4], v5, off
	s_and_not1_b32 exec_lo, exec_lo, s9
	s_cbranch_execnz .LBB6_50
.LBB6_51:
	s_or_b32 exec_lo, exec_lo, s6
	s_and_saveexec_b32 s0, s5
	s_cbranch_execz .LBB6_45
; %bb.52:
	v_mov_b32_e32 v2, 0
	s_lshl_b64 s[0:1], s[2:3], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s2, s22, s0
	s_addc_u32 s3, s23, s1
	ds_load_2addr_b32 v[0:1], v2 offset0:65 offset1:66
	s_add_u32 s0, s20, s0
	s_addc_u32 s1, s21, s1
	s_waitcnt lgkmcnt(0)
	s_clause 0x1
	global_store_b32 v2, v0, s[0:1]
	global_store_b32 v2, v1, s[2:3]
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
		.amdhsa_group_segment_fixed_size 272
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
		.amdhsa_next_free_vgpr 23
		.amdhsa_next_free_sgpr 56
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
		.amdhsa_inst_pref_size 29
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,"axG",@progbits,_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,comdat
.Lfunc_end6:
	.size	_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii, .Lfunc_end6-_Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
                                        ; -- End function
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.num_vgpr, 23
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.num_agpr, 0
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.numbered_sgpr, 56
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.num_named_barrier, 0
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.private_seg_size, 0
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.uses_vcc, 1
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.uses_flat_scratch, 0
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.has_dyn_sized_stack, 0
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.has_recursion, 0
	.set _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3680
; TotalNumSgprs: 58
; NumVgprs: 23
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 272 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 58
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
	.section	.text._Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,"axG",@progbits,_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,comdat
	.protected	_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii ; -- Begin function _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
	.globl	_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
	.p2align	8
	.type	_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,@function
_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii: ; @_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
; %bb.0:
	s_load_b256 s[12:19], s[0:1], 0x24
	s_abs_i32 s6, s2
	s_load_b256 s[20:27], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s4, s13
	s_ashr_i32 s7, s13, 31
	v_cvt_f32_u32_e32 v1, s4
	s_sub_i32 s5, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s3, v1
	s_mul_i32 s5, s5, s3
	s_mul_hi_u32 s5, s3, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s3, s5
	s_ashr_i32 s3, s2, 31
	s_mul_hi_u32 s5, s6, s5
	s_xor_b32 s9, s3, s7
	s_mul_i32 s8, s5, s4
	s_sub_i32 s6, s6, s8
	s_add_i32 s8, s5, 1
	s_sub_i32 s10, s6, s4
	s_cmp_ge_u32 s6, s4
	s_cselect_b32 s5, s8, s5
	s_cselect_b32 s6, s10, s6
	s_add_i32 s8, s5, 1
	s_cmp_ge_u32 s6, s4
	s_cselect_b32 s5, s8, s5
	s_abs_i32 s6, s14
	s_xor_b32 s5, s5, s9
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s10, 0, s6
	s_sub_i32 s33, s5, s9
	s_ashr_i32 s11, s14, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_xor_b32 s7, s7, s11
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s8, v1
	s_mul_i32 s10, s10, s8
	s_mul_hi_u32 s10, s8, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s8, s8, s10
	s_mul_hi_u32 s5, s4, s8
	s_mul_i32 s8, s33, s13
	s_mul_i32 s9, s5, s6
	s_sub_i32 s58, s2, s8
	s_sub_i32 s4, s4, s9
	s_add_i32 s8, s5, 1
	s_sub_i32 s9, s4, s6
	s_cmp_ge_u32 s4, s6
	s_cselect_b32 s5, s8, s5
	s_cselect_b32 s4, s9, s4
	s_add_i32 s8, s5, 1
	s_cmp_ge_u32 s4, s6
	s_cselect_b32 s4, s8, s5
	s_abs_i32 s44, s58
	s_xor_b32 s4, s4, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_sub_i32 s5, s4, s7
	s_abs_i32 s7, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v1, s7
	s_sub_i32 s6, 0, s7
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s4, v1
	s_mul_i32 s8, s6, s4
	s_mov_b32 s6, 0
	s_mul_hi_u32 s8, s4, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s8, s4, s8
	v_cmp_gt_i32_e64 s4, s16, v0
	s_cmp_gt_i32 s26, 0
	s_mul_hi_u32 s19, s44, s8
	s_cselect_b32 s8, -1, 0
	s_and_saveexec_b32 s9, s4
	s_cbranch_execz .LBB7_6
; %bb.1:
	s_load_b32 s28, s[0:1], 0x6c
	s_mul_hi_i32 s11, s16, s2
	s_mul_i32 s10, s16, s2
	v_lshl_add_u32 v4, v0, 3, 0x220
	s_lshl_b64 s[10:11], s[10:11], 3
	v_mov_b32_e32 v1, v0
	s_add_u32 s10, s24, s10
	s_addc_u32 s11, s25, s11
	s_waitcnt lgkmcnt(0)
	s_and_b32 s28, s28, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s29, s28, 3
	s_branch .LBB7_4
	.p2align	6
.LBB7_2:                                ;   in Loop: Header=BB7_4 Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	v_add_co_u32 v2, vcc_lo, s10, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s11, v3, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
.LBB7_3:                                ;   in Loop: Header=BB7_4 Depth=1
	v_add_nc_u32_e32 v1, s28, v1
	s_waitcnt vmcnt(0)
	ds_store_b64 v4, v[2:3]
	v_add_nc_u32_e32 v4, s29, v4
	v_cmp_le_i32_e32 vcc_lo, s16, v1
	s_or_b32 s6, vcc_lo, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s6
	s_cbranch_execz .LBB7_6
.LBB7_4:                                ; =>This Inner Loop Header: Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s8
	s_cbranch_vccz .LBB7_2
; %bb.5:                                ;   in Loop: Header=BB7_4 Depth=1
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_branch .LBB7_3
.LBB7_6:
	s_or_b32 exec_lo, exec_lo, s9
	s_ashr_i32 s45, s58, 31
	s_ashr_i32 s46, s5, 31
; %bb.7:
	s_load_b256 s[36:43], s[0:1], 0x0
	v_cmp_gt_i32_e64 s6, s15, v0
	v_lshlrev_b32_e32 v3, 3, v0
	s_and_saveexec_b32 s5, s6
	s_cbranch_execz .LBB7_10
; %bb.8:
	s_load_b32 s30, s[0:1], 0x6c
	s_mul_i32 s9, s15, s13
	s_mul_hi_i32 s29, s58, s15
	s_mul_hi_i32 s11, s33, s9
	s_mul_i32 s10, s33, s9
	s_mul_i32 s28, s58, s15
	s_lshl_b64 s[10:11], s[10:11], 3
	v_mov_b32_e32 v1, v0
	s_waitcnt lgkmcnt(0)
	s_add_u32 s9, s36, s10
	s_addc_u32 s31, s37, s11
	s_lshl_b64 s[10:11], s[28:29], 3
	s_mov_b32 s29, 0
	s_add_u32 s9, s9, s10
	s_addc_u32 s10, s31, s11
	s_lshl_b32 s11, s16, 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add3_u32 v4, 0x220, s11, v3
	s_and_b32 s11, s30, 0xffff
	s_lshl_b32 s28, s11, 3
	.p2align	6
.LBB7_9:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[5:6], 3, v[1:2]
	v_add_nc_u32_e32 v1, s11, v1
	v_add_co_u32 v5, vcc_lo, s9, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v6, null, s10, v6, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s15, v1
	global_load_b64 v[5:6], v[5:6], off
	s_or_b32 s29, vcc_lo, s29
	s_waitcnt vmcnt(0)
	ds_store_b64 v4, v[5:6]
	v_add_nc_u32_e32 v4, s28, v4
	s_and_not1_b32 exec_lo, exec_lo, s29
	s_cbranch_execnz .LBB7_9
.LBB7_10:
	s_or_b32 exec_lo, exec_lo, s5
	v_cmp_eq_u32_e64 s5, 0, v0
	s_and_saveexec_b32 s28, s5
	s_cbranch_execz .LBB7_15
; %bb.11:
	s_and_not1_b32 vcc_lo, exec_lo, s8
	s_cbranch_vccnz .LBB7_13
; %bb.12:
	s_lshl_b64 s[8:9], s[2:3], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s10, s20, s8
	s_addc_u32 s11, s21, s9
	s_add_u32 s30, s22, s8
	s_addc_u32 s31, s23, s9
	s_load_b64 s[8:9], s[10:11], 0x0
	s_load_b64 s[10:11], s[30:31], 0x0
	s_branch .LBB7_14
.LBB7_13:
	s_mov_b64 s[10:11], 0
	s_mov_b32 s9, 0xfe37e43c
	s_mov_b32 s8, 0x8800759c
.LBB7_14:
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v1, s8
	v_mov_b32_e32 v4, s10
	v_dual_mov_b32 v2, s9 :: v_dual_mov_b32 v5, s11
	ds_store_2addr_b64 v6, v[1:2], v[4:5] offset0:65 offset1:66
.LBB7_15:
	s_or_b32 exec_lo, exec_lo, s28
	s_add_i32 s8, s16, s15
	s_waitcnt lgkmcnt(0)
	s_ashr_i32 s9, s8, 31
	s_mul_hi_i32 s29, s14, s8
	s_lshl_b64 s[10:11], s[8:9], 3
	s_mul_i32 s28, s14, s8
	s_sub_u32 s10, 0x400000, s10
	s_subb_u32 s11, 0, s11
	s_lshl_b64 s[8:9], s[28:29], 3
	s_barrier
	s_or_b64 s[28:29], s[10:11], s[8:9]
	s_mov_b32 s28, 0
	buffer_gl0_inv
	s_cmp_lg_u64 s[28:29], 0
	s_cbranch_scc0 .LBB7_46
; %bb.16:
	s_ashr_i32 s30, s9, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_add_u32 s34, s8, s30
	s_mov_b32 s31, s30
	s_addc_u32 s35, s9, s30
	s_xor_b64 s[34:35], s[34:35], s[30:31]
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v1, s34
	v_cvt_f32_u32_e32 v2, s35
	s_sub_u32 s36, 0, s34
	s_subb_u32 s37, 0, s35
	v_fmamk_f32 v1, v2, 0x4f800000, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x5f7ffffc, v1
	v_mul_f32_e32 v2, 0x2f800000, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v2, v2
	v_fmamk_f32 v1, v2, 0xcf800000, v1
	v_cvt_u32_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s9, v2
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s29, v1
	s_mul_i32 s47, s36, s9
	s_mul_hi_u32 s49, s36, s29
	s_mul_i32 s48, s37, s29
	s_add_i32 s47, s49, s47
	s_mul_i32 s50, s36, s29
	s_add_i32 s47, s47, s48
	s_mul_hi_u32 s49, s29, s50
	s_mul_i32 s52, s29, s47
	s_mul_hi_u32 s51, s9, s50
	s_mul_i32 s48, s9, s50
	s_mul_hi_u32 s50, s29, s47
	s_add_u32 s49, s49, s52
	s_addc_u32 s50, 0, s50
	s_mul_hi_u32 s53, s9, s47
	s_add_u32 s48, s49, s48
	s_mul_i32 s47, s9, s47
	s_addc_u32 s48, s50, s51
	s_addc_u32 s49, s53, 0
	s_add_u32 s47, s48, s47
	s_addc_u32 s48, 0, s49
	s_add_u32 s29, s29, s47
	s_cselect_b32 s47, -1, 0
	s_mul_hi_u32 s49, s36, s29
	s_cmp_lg_u32 s47, 0
	s_mul_i32 s47, s36, s29
	s_addc_u32 s9, s9, s48
	s_mul_i32 s37, s37, s29
	s_mul_i32 s36, s36, s9
	s_mul_hi_u32 s48, s29, s47
	s_add_i32 s36, s49, s36
	s_mul_hi_u32 s49, s9, s47
	s_add_i32 s36, s36, s37
	s_mul_i32 s37, s9, s47
	s_mul_i32 s51, s29, s36
	s_mul_hi_u32 s50, s29, s36
	s_add_u32 s48, s48, s51
	s_addc_u32 s50, 0, s50
	s_mul_hi_u32 s47, s9, s36
	s_add_u32 s37, s48, s37
	s_mul_i32 s36, s9, s36
	s_addc_u32 s37, s50, s49
	s_addc_u32 s47, s47, 0
	s_add_u32 s36, s37, s36
	s_addc_u32 s37, 0, s47
	s_add_u32 s29, s29, s36
	s_cselect_b32 s36, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s36, 0
	s_addc_u32 s9, s9, s37
	s_ashr_i32 s36, s11, 31
	s_add_u32 s48, s10, s36
	s_mov_b32 s37, s36
	s_addc_u32 s49, s11, s36
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b64 s[48:49], s[48:49], s[36:37]
	s_mul_i32 s47, s48, s9
	s_mul_hi_u32 s50, s48, s29
	s_mul_hi_u32 s11, s48, s9
	s_mul_hi_u32 s52, s49, s29
	s_mul_i32 s29, s49, s29
	s_add_u32 s47, s50, s47
	s_addc_u32 s11, 0, s11
	s_mul_hi_u32 s51, s49, s9
	s_add_u32 s29, s47, s29
	s_mul_i32 s9, s49, s9
	s_addc_u32 s11, s11, s52
	s_addc_u32 s29, s51, 0
	s_add_u32 s9, s11, s9
	s_addc_u32 s11, 0, s29
	s_mul_hi_u32 s29, s34, s9
	s_mul_i32 s47, s34, s11
	s_mul_i32 s50, s35, s9
	s_add_i32 s29, s29, s47
	s_mul_i32 s47, s34, s9
	s_add_i32 s29, s29, s50
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_sub_i32 s50, s49, s29
	s_sub_u32 s47, s48, s47
	s_cselect_b32 s48, -1, 0
	s_cmp_lg_u32 s48, 0
	s_subb_u32 s50, s50, s35
	s_sub_u32 s51, s47, s34
	s_cselect_b32 s52, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s52, 0
	s_subb_u32 s50, s50, 0
	s_cmp_ge_u32 s50, s35
	s_cselect_b32 s52, -1, 0
	s_cmp_ge_u32 s51, s34
	s_cselect_b32 s51, -1, 0
	s_cmp_eq_u32 s50, s35
	s_cselect_b32 s50, s51, s52
	s_add_u32 s51, s9, 1
	s_addc_u32 s52, s11, 0
	s_add_u32 s53, s9, 2
	s_addc_u32 s54, s11, 0
	s_cmp_lg_u32 s50, 0
	s_cselect_b32 s50, s53, s51
	s_cselect_b32 s51, s54, s52
	s_cmp_lg_u32 s48, 0
	s_subb_u32 s29, s49, s29
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_ge_u32 s29, s35
	s_cselect_b32 s48, -1, 0
	s_cmp_ge_u32 s47, s34
	s_cselect_b32 s34, -1, 0
	s_cmp_eq_u32 s29, s35
	s_cselect_b32 s29, s34, s48
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_cmp_lg_u32 s29, 0
	s_cselect_b32 s35, s51, s11
	s_cselect_b32 s34, s50, s9
	s_xor_b64 s[30:31], s[36:37], s[30:31]
	s_xor_b64 s[34:35], s[34:35], s[30:31]
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_u32 s30, s34, s30
	s_subb_u32 s31, s35, s31
	s_and_not1_b32 vcc_lo, exec_lo, s28
	s_cbranch_vccnz .LBB7_18
.LBB7_17:
	v_cvt_f32_u32_e32 v1, s8
	s_sub_i32 s11, 0, s8
	s_mov_b32 s31, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s9, v1
	s_mul_i32 s11, s11, s9
	s_mul_hi_u32 s11, s9, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s9, s9, s11
	s_mul_hi_u32 s9, s10, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s11, s9, s8
	s_sub_i32 s10, s10, s11
	s_add_i32 s11, s9, 1
	s_sub_i32 s28, s10, s8
	s_cmp_ge_u32 s10, s8
	s_cselect_b32 s9, s11, s9
	s_cselect_b32 s10, s28, s10
	s_add_i32 s11, s9, 1
	s_cmp_ge_u32 s10, s8
	s_cselect_b32 s30, s11, s9
.LBB7_18:
	s_cmp_lt_i32 s12, 1
	s_cbranch_scc1 .LBB7_40
; %bb.19:
	s_ashr_i32 s9, s12, 31
	s_mov_b32 s8, s12
	s_mul_i32 s10, s19, s7
	v_cmp_lt_i64_e64 s8, s[30:31], s[8:9]
	v_cmp_gt_i64_e64 s9, s[30:31], 0
	v_dual_mov_b32 v14, 0 :: v_dual_and_b32 v7, 31, v0
	v_mbcnt_lo_u32_b32 v9, -1, 0
	v_lshrrev_b32_e32 v8, 2, v0
	s_and_b32 s8, s8, exec_lo
	s_cselect_b32 s8, s30, s12
	s_and_b32 s9, s9, exec_lo
	s_cselect_b32 s59, s8, 1
	s_sub_i32 s9, s44, s10
	s_xor_b32 s8, s45, s46
	s_add_i32 s10, s19, 1
	s_sub_i32 s11, s9, s7
	s_cmp_ge_u32 s9, s7
	v_lshlrev_b32_e32 v10, 3, v7
	s_cselect_b32 s10, s10, s19
	s_cselect_b32 s9, s11, s9
	s_add_i32 s11, s10, 1
	s_cmp_ge_u32 s9, s7
	v_add_nc_u32_e32 v12, 0x220, v3
	s_cselect_b32 s7, s11, s10
	s_add_i32 s17, s33, s17
	s_xor_b32 s7, s7, s8
	v_lshl_or_b32 v13, v9, 2, 64
	s_sub_i32 s7, s7, s8
	s_mov_b32 s30, 0xfefa39ef
	s_mul_hi_i32 s9, s7, s15
	s_mul_i32 s8, s7, s15
	s_mov_b32 s34, 0x3b39803f
	s_lshl_b64 s[8:9], s[8:9], 3
	s_mov_b32 s36, 0xfca7ab0c
	s_add_u32 s62, s38, s8
	s_addc_u32 s63, s39, s9
	s_add_u32 s28, s0, 0x60
	s_addc_u32 s29, s1, 0
	s_cmp_lt_i32 s17, s18
	s_mul_hi_i32 s9, s7, s16
	s_mul_i32 s8, s7, s16
	s_cselect_b32 s64, -1, 0
	s_lshl_b64 s[8:9], s[8:9], 3
	v_cmp_gt_u32_e64 s7, 32, v0
	s_add_u32 s65, s40, s8
	s_addc_u32 s66, s41, s9
	s_lshl_b32 s8, s16, 3
	s_mov_b32 s18, 0x652b82fe
	v_add3_u32 v11, 0x220, s8, v3
	s_mov_b32 s38, 0x6a5dcb37
	s_mov_b32 s40, 0x623fde64
	s_mov_b32 s44, 0x7c89e6b0
	s_mov_b32 s46, 0x14761f6e
	s_mov_b32 s48, 0x1852b7b0
	s_mov_b32 s50, 0x11122322
	s_mov_b32 s52, 0x555502a1
	s_mov_b32 s54, 0x55555511
	s_mov_b32 s56, 11
	s_mul_i32 s60, s15, s14
	s_mul_i32 s14, s16, s14
	s_mov_b32 s61, 0
	s_mov_b32 s19, 0x3ff71547
	s_mov_b32 s31, 0xbfe62e42
	s_mov_b32 s35, 0xbc7abc9e
	s_mov_b32 s37, 0x3e928af3
	s_mov_b32 s39, 0x3e5ade15
	s_mov_b32 s41, 0x3ec71dee
	s_mov_b32 s45, 0x3efa0199
	s_mov_b32 s47, 0x3f2a01a0
	s_mov_b32 s49, 0x3f56c16c
	s_mov_b32 s51, 0x3f811111
	s_mov_b32 s53, 0x3fa55555
	s_mov_b32 s55, 0x3fc55555
	s_mov_b32 s57, 0x3fe00000
	v_cmp_eq_u32_e32 vcc_lo, 0, v7
	s_branch .LBB7_21
.LBB7_20:                               ;   in Loop: Header=BB7_21 Depth=1
	s_cmp_lt_i32 s61, s12
	s_cbranch_scc0 .LBB7_40
.LBB7_21:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB7_24 Depth 2
                                        ;       Child Loop BB7_26 Depth 3
                                        ;       Child Loop BB7_39 Depth 3
	s_mov_b32 s67, s61
	s_add_i32 s61, s61, s59
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_min_i32 s68, s61, s12
	s_cmp_ge_i32 s67, s68
	s_cbranch_scc1 .LBB7_20
; %bb.22:                               ;   in Loop: Header=BB7_21 Depth=1
	v_cmp_gt_u32_e64 s8, 24, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v1, 0, 8, s8
	v_cmp_gt_u32_e64 s8, 28, v9
	v_add_lshl_u32 v15, v1, v9, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, 0, 4, s8
	v_cmp_gt_u32_e64 s8, 30, v9
	v_add_lshl_u32 v16, v2, v9, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v3, 0, 2, s8
	v_cmp_ne_u32_e64 s8, 31, v9
	v_add_lshl_u32 v17, v3, v9, 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v9, s8
	v_lshlrev_b32_e32 v18, 2, v4
	s_branch .LBB7_24
.LBB7_23:                               ;   in Loop: Header=BB7_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s9
	s_add_i32 s67, s67, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s67, s68
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB7_20
.LBB7_24:                               ;   Parent Loop BB7_21 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB7_26 Depth 3
                                        ;       Child Loop BB7_39 Depth 3
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_and_saveexec_b32 s9, s6
	s_cbranch_execz .LBB7_28
; %bb.25:                               ;   in Loop: Header=BB7_24 Depth=2
	s_load_b32 s8, s[28:29], 0xc
	s_mul_hi_i32 s11, s67, s60
	s_mul_i32 s10, s67, s60
	v_mov_b32_e32 v1, 0
	s_lshl_b64 s[10:11], s[10:11], 3
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v5, v11
	v_mov_b32_e32 v3, v0
	s_add_u32 s10, s62, s10
	s_addc_u32 s11, s63, s11
	s_mov_b32 s70, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s69, s8, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s71, s69, 3
	.p2align	6
.LBB7_26:                               ;   Parent Loop BB7_21 Depth=1
                                        ;     Parent Loop BB7_24 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v4, 31, v3
	ds_load_b64 v[21:22], v5
	v_add_nc_u32_e32 v5, s71, v5
	v_lshlrev_b64 v[19:20], 3, v[3:4]
	v_add_nc_u32_e32 v3, s69, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v19, s8, s10, v19
	v_add_co_ci_u32_e64 v20, null, s11, v20, s8
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s8, s15, v3
	global_load_b64 v[19:20], v[19:20], off
	s_or_b32 s70, s8, s70
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[1:2], v[21:22], v[19:20], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s70
	s_cbranch_execnz .LBB7_26
; %bb.27:                               ;   in Loop: Header=BB7_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s70
.LBB7_28:                               ;   in Loop: Header=BB7_24 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s9
	ds_bpermute_b32 v3, v13, v1
	ds_bpermute_b32 v4, v13, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v15, v1
	ds_bpermute_b32 v4, v15, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v16, v1
	ds_bpermute_b32 v4, v16, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v17, v1
	ds_bpermute_b32 v4, v17, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v18, v1
	ds_bpermute_b32 v4, v18, v2
	s_and_saveexec_b32 s8, vcc_lo
	s_cbranch_execz .LBB7_30
; %bb.29:                               ;   in Loop: Header=BB7_24 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v8, v[1:2]
.LBB7_30:                               ;   in Loop: Header=BB7_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s8
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s9, s7
	s_cbranch_execz .LBB7_35
; %bb.31:                               ;   in Loop: Header=BB7_24 Depth=2
	s_load_b32 s8, s[28:29], 0xc
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s10, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s8, s8, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s8, s8, 31
	s_lshr_b32 s8, s8, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s8, v7
; %bb.32:                               ;   in Loop: Header=BB7_24 Depth=2
	ds_load_b64 v[1:2], v10
; %bb.33:                               ;   in Loop: Header=BB7_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s10
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v13, v1
	ds_bpermute_b32 v4, v13, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v15, v1
	ds_bpermute_b32 v4, v15, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v16, v1
	ds_bpermute_b32 v4, v16, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v17, v1
	ds_bpermute_b32 v4, v17, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v18, v1
	ds_bpermute_b32 v4, v18, v2
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB7_35
; %bb.34:                               ;   in Loop: Header=BB7_24 Depth=2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v14, v[1:2]
.LBB7_35:                               ;   in Loop: Header=BB7_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s9
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v14
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s69, s5
	s_cbranch_execz .LBB7_37
; %bb.36:                               ;   in Loop: Header=BB7_24 Depth=2
	ds_load_2addr_b64 v[3:6], v14 offset0:65 offset1:66
	s_add_i32 s8, s67, s26
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_cmp_gt_i32 s8, s17
	s_cselect_b32 s8, -1, 0
	s_and_b32 s8, s64, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, v2, 0xc6293e59, s8
	v_cndmask_b32_e64 v1, v1, 0x39a08cea, s8
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e64 s8, v[1:2], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v22, v4, v2, s8
	v_cndmask_b32_e64 v21, v3, v1, s8
	v_add_f64 v[3:4], v[3:4], -v[21:22]
	v_add_f64 v[1:2], v[1:2], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[19:20], v[3:4], s[18:19]
	v_mul_f64 v[23:24], v[1:2], s[18:19]
	v_cmp_nlt_f64_e64 s8, 0x40900000, v[3:4]
	v_cmp_nlt_f64_e64 s10, 0x40900000, v[1:2]
	v_cmp_ngt_f64_e64 s9, 0xc090cc00, v[3:4]
	v_cmp_ngt_f64_e64 s11, 0xc090cc00, v[1:2]
	v_rndne_f64_e32 v[19:20], v[19:20]
	v_rndne_f64_e32 v[23:24], v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[25:26], v[19:20], s[30:31], v[3:4]
	v_fma_f64 v[27:28], v[23:24], s[30:31], v[1:2]
	v_cvt_i32_f64_e32 v33, v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[25:26], v[19:20], s[34:35], v[25:26]
	v_fma_f64 v[27:28], v[23:24], s[34:35], v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], s[38:39], s[36:37]
	v_fma_f64 v[31:32], v[27:28], s[38:39], s[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], s[40:41]
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[40:41]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], s[44:45]
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[44:45]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], s[46:47]
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[46:47]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], s[48:49]
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[48:49]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], s[50:51]
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[50:51]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], s[52:53]
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[52:53]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], s[54:55]
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[54:55]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], s[56:57]
	v_fma_f64 v[31:32], v[27:28], v[31:32], s[56:57]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], v[25:26], v[29:30], 1.0
	v_fma_f64 v[31:32], v[27:28], v[31:32], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[19:20], v[25:26], v[29:30], 1.0
	v_cvt_i32_f64_e32 v25, v[23:24]
	v_fma_f64 v[23:24], v[27:28], v[31:32], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[19:20], v[19:20], v33
	v_ldexp_f64 v[23:24], v[23:24], v25
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v20, 0x7ff00000, v20, s8
	s_and_b32 s8, s9, s8
	v_cndmask_b32_e64 v3, 0x7ff00000, v24, s10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v4, 0, v20, s9
	v_cndmask_b32_e64 v20, 0, v3, s11
	v_cndmask_b32_e64 v3, 0, v19, s8
	s_and_b32 s8, s11, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v19, 0, v23, s8
	v_fma_f64 v[1:2], v[5:6], v[3:4], v[19:20]
	ds_store_b128 v14, v[1:4] offset:528
	ds_store_b128 v14, v[19:22] offset:512
.LBB7_37:                               ;   in Loop: Header=BB7_24 Depth=2
	s_or_b32 exec_lo, exec_lo, s69
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s9, s4
	s_cbranch_execz .LBB7_23
; %bb.38:                               ;   in Loop: Header=BB7_24 Depth=2
	s_load_b32 s8, s[28:29], 0xc
	ds_load_2addr_b64 v[1:4], v14 offset0:64 offset1:67
	s_mul_hi_i32 s11, s67, s14
	s_mul_i32 s10, s67, s14
	v_mov_b32_e32 v19, v12
	s_lshl_b64 s[10:11], s[10:11], 3
	v_mov_b32_e32 v5, v0
	s_add_u32 s10, s65, s10
	s_addc_u32 s11, s66, s11
	s_mov_b32 s71, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s69, s8, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s70, s69, 3
	.p2align	6
.LBB7_39:                               ;   Parent Loop BB7_21 Depth=1
                                        ;     Parent Loop BB7_24 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v6, 31, v5
	ds_load_b64 v[22:23], v19
	v_lshlrev_b64 v[20:21], 3, v[5:6]
	v_add_nc_u32_e32 v5, s69, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v20, s8, s10, v20
	v_add_co_ci_u32_e64 v21, null, s11, v21, s8
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e64 s8, s16, v5
	global_load_b64 v[20:21], v[20:21], off
	s_or_b32 s71, s8, s71
	s_waitcnt vmcnt(0)
	v_mul_f64 v[20:21], v[1:2], v[20:21]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[20:21], v[22:23], v[3:4], v[20:21]
	ds_store_b64 v19, v[20:21]
	v_add_nc_u32_e32 v19, s70, v19
	s_and_not1_b32 exec_lo, exec_lo, s71
	s_cbranch_execnz .LBB7_39
	s_branch .LBB7_23
.LBB7_40:
	s_cmp_lg_u32 s27, 0
	s_cbranch_scc0 .LBB7_47
; %bb.41:
	s_and_saveexec_b32 s7, s4
	s_cbranch_execz .LBB7_44
; %bb.42:
	v_mov_b32_e32 v1, 0
	s_load_b32 s6, s[0:1], 0x6c
	s_mul_i32 s8, s16, s13
	s_mul_hi_i32 s11, s58, s16
	s_mul_hi_i32 s9, s33, s8
	ds_load_b64 v[1:2], v1 offset:528
	s_mul_i32 s8, s33, s8
	s_mul_i32 s10, s58, s16
	s_lshl_b64 s[8:9], s[8:9], 3
	v_lshl_add_u32 v5, v0, 3, 0x220
	s_add_u32 s12, s42, s8
	s_addc_u32 s13, s43, s9
	s_lshl_b64 s[8:9], s[10:11], 3
	v_mov_b32_e32 v3, v0
	s_add_u32 s8, s12, s8
	s_addc_u32 s9, s13, s9
	s_mov_b32 s12, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s10, s6, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b32 s11, s10, 3
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB7_43:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b64 v[6:7], v5
	v_ashrrev_i32_e32 v4, 31, v3
	v_add_nc_u32_e32 v5, s11, v5
	s_waitcnt lgkmcnt(0)
	v_div_scale_f64 v[8:9], null, v[1:2], v[1:2], v[6:7]
	v_div_scale_f64 v[14:15], vcc_lo, v[6:7], v[1:2], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_fma_f64 v[8:9], -v[8:9], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[12:13]
	v_div_fixup_f64 v[6:7], v[8:9], v[1:2], v[6:7]
	v_lshlrev_b64 v[8:9], 3, v[3:4]
	v_add_nc_u32_e32 v3, s10, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_i32_e32 vcc_lo, s16, v3
	v_add_co_u32 v8, s6, s8, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s9, v9, s6
	s_or_b32 s12, vcc_lo, s12
	global_store_b64 v[8:9], v[6:7], off
	s_and_not1_b32 exec_lo, exec_lo, s12
	s_cbranch_execnz .LBB7_43
.LBB7_44:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s7
	s_cbranch_execz .LBB7_48
.LBB7_45:
	s_endpgm
.LBB7_46:
                                        ; implicit-def: $sgpr30_sgpr31
	s_branch .LBB7_17
.LBB7_47:
.LBB7_48:
	s_and_saveexec_b32 s6, s4
	s_cbranch_execz .LBB7_51
; %bb.49:
	s_load_b32 s7, s[0:1], 0x6c
	s_mul_hi_i32 s1, s16, s2
	s_mul_i32 s0, s16, s2
	v_lshl_add_u32 v2, v0, 3, 0x220
	s_lshl_b64 s[8:9], s[0:1], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_u32 s1, s24, s8
	s_addc_u32 s4, s25, s9
	s_mov_b32 s9, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s7, s7, 0xffff
	s_lshl_b32 s8, s7, 3
	.p2align	6
.LBB7_50:                               ; =>This Inner Loop Header: Depth=1
	ds_load_b64 v[3:4], v2
	v_ashrrev_i32_e32 v1, 31, v0
	v_add_nc_u32_e32 v2, s8, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 3, v[0:1]
	v_add_nc_u32_e32 v0, s7, v0
	v_cmp_le_i32_e32 vcc_lo, s16, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, s0, s1, v5
	v_add_co_ci_u32_e64 v6, null, s4, v6, s0
	s_or_b32 s9, vcc_lo, s9
	s_waitcnt lgkmcnt(0)
	global_store_b64 v[5:6], v[3:4], off
	s_and_not1_b32 exec_lo, exec_lo, s9
	s_cbranch_execnz .LBB7_50
.LBB7_51:
	s_or_b32 exec_lo, exec_lo, s6
	s_and_saveexec_b32 s0, s5
	s_cbranch_execz .LBB7_45
; %bb.52:
	v_mov_b32_e32 v4, 0
	s_lshl_b64 s[0:1], s[2:3], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s2, s22, s0
	s_addc_u32 s3, s23, s1
	ds_load_2addr_b64 v[0:3], v4 offset0:65 offset1:66
	s_add_u32 s0, s20, s0
	s_addc_u32 s1, s21, s1
	s_waitcnt lgkmcnt(0)
	s_clause 0x1
	global_store_b64 v4, v[0:1], s[0:1]
	global_store_b64 v4, v[2:3], s[2:3]
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
		.amdhsa_group_segment_fixed_size 544
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
		.amdhsa_next_free_vgpr 34
		.amdhsa_next_free_sgpr 72
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
		.amdhsa_inst_pref_size 34
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,"axG",@progbits,_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii,comdat
.Lfunc_end7:
	.size	_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii, .Lfunc_end7-_Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
                                        ; -- End function
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.num_vgpr, 34
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.num_agpr, 0
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.numbered_sgpr, 72
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.num_named_barrier, 0
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.private_seg_size, 0
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.uses_vcc, 1
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.uses_flat_scratch, 0
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.has_dyn_sized_stack, 0
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.has_recursion, 0
	.set _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4284
; TotalNumSgprs: 74
; NumVgprs: 34
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 544 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 74
; NumVGPRsForWavesPerEU: 34
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z12rope_partialIfEvPT_iiiiPKS0_S3_i,"axG",@progbits,_Z12rope_partialIfEvPT_iiiiPKS0_S3_i,comdat
	.protected	_Z12rope_partialIfEvPT_iiiiPKS0_S3_i ; -- Begin function _Z12rope_partialIfEvPT_iiiiPKS0_S3_i
	.globl	_Z12rope_partialIfEvPT_iiiiPKS0_S3_i
	.p2align	8
	.type	_Z12rope_partialIfEvPT_iiiiPKS0_S3_i,@function
_Z12rope_partialIfEvPT_iiiiPKS0_S3_i:   ; @_Z12rope_partialIfEvPT_iiiiPKS0_S3_i
; %bb.0:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x8
	s_load_b32 s3, s[0:1], 0x3c
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v5
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s8, s6, 31
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[3:4], null, s3, s2, v[0:1]
	s_add_i32 s2, s6, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s16, s2, 1
	s_mul_hi_i32 s3, s16, s4
	s_mul_i32 s2, s16, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[3:4]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB8_17
; %bb.1:
	s_load_b128 s[8:11], s[0:1], 0x18
	s_ashr_i32 s4, s16, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v6, s4, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[5:6]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB8_3
; %bb.2:
	s_ashr_i32 s12, s4, 31
	v_ashrrev_i32_e32 v2, 31, v4
	s_add_u32 s14, s16, s12
	s_mov_b32 s13, s12
	s_addc_u32 s15, s4, s12
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[14:15], s[14:15], s[12:13]
	v_add_co_u32 v5, vcc_lo, v3, v2
	v_cvt_f32_u32_e32 v0, s14
	v_cvt_f32_u32_e32 v1, s15
	s_sub_u32 s17, 0, s14
	s_subb_u32 s18, 0, s15
	v_add_co_ci_u32_e64 v6, null, v4, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v5, v2
	v_xor_b32_e32 v10, v6, v2
	v_xor_b32_e32 v2, s12, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x5f7ffffc, v0
	v_mul_f32_e32 v1, 0x2f800000, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v1, v1
	v_fmamk_f32 v0, v1, 0xcf800000, v0
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s2, v1
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s13, v0
	s_mul_i32 s19, s17, s2
	s_mul_hi_u32 s21, s17, s13
	s_mul_i32 s20, s18, s13
	s_add_i32 s19, s21, s19
	s_mul_i32 s22, s17, s13
	s_add_i32 s19, s19, s20
	s_mul_hi_u32 s21, s13, s22
	s_mul_i32 s24, s13, s19
	s_mul_hi_u32 s23, s2, s22
	s_mul_i32 s20, s2, s22
	s_mul_hi_u32 s22, s13, s19
	s_add_u32 s21, s21, s24
	s_addc_u32 s22, 0, s22
	s_mul_hi_u32 s25, s2, s19
	s_add_u32 s20, s21, s20
	s_mul_i32 s19, s2, s19
	s_addc_u32 s20, s22, s23
	s_addc_u32 s21, s25, 0
	s_add_u32 s19, s20, s19
	s_addc_u32 s20, 0, s21
	s_add_u32 s13, s13, s19
	s_cselect_b32 s19, -1, 0
	s_mul_hi_u32 s21, s17, s13
	s_cmp_lg_u32 s19, 0
	s_mul_i32 s19, s17, s13
	s_addc_u32 s2, s2, s20
	s_mul_i32 s18, s18, s13
	s_mul_i32 s17, s17, s2
	s_mul_hi_u32 s20, s13, s19
	s_add_i32 s17, s21, s17
	s_mul_hi_u32 s21, s2, s19
	s_add_i32 s17, s17, s18
	s_mul_i32 s18, s2, s19
	s_mul_i32 s23, s13, s17
	s_mul_hi_u32 s22, s13, s17
	s_add_u32 s20, s20, s23
	s_addc_u32 s22, 0, s22
	s_mul_hi_u32 s19, s2, s17
	s_add_u32 s18, s20, s18
	s_mul_i32 s17, s2, s17
	s_addc_u32 s18, s22, s21
	s_addc_u32 s19, s19, 0
	s_add_u32 s17, s18, s17
	s_addc_u32 s18, 0, s19
	s_add_u32 s13, s13, s17
	s_cselect_b32 s17, -1, 0
	v_mul_hi_u32 v11, v9, s13
	s_cmp_lg_u32 s17, 0
	v_mad_u64_u32 v[5:6], null, v10, s13, 0
	s_addc_u32 s2, s2, s18
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[7:8], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v6, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v8, vcc_lo
	v_add_co_u32 v5, vcc_lo, v0, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v6, null, 0, v1, vcc_lo
	v_mul_lo_u32 v7, s15, v5
	v_mad_u64_u32 v[0:1], null, s14, v5, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v8, s14, v6
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v8, v7
	v_add_co_u32 v8, s2, v5, 2
	v_add_co_ci_u32_e64 v9, null, 0, v6, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v10, v1
	v_sub_co_u32 v11, s2, v0, s14
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s15, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s14, v11
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s15, v7
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s14, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s15, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s15, v7
	v_cndmask_b32_e32 v7, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v5, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v11, null, 0, v6, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s15, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v7
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v8 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v5, v0, s2
	v_cndmask_b32_e64 v1, v6, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v2
	v_xor_b32_e32 v1, v1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v2
	v_sub_co_ci_u32_e64 v1, null, v1, v2, vcc_lo
.LBB8_3:
	s_or_saveexec_b32 s2, s3
	s_waitcnt lgkmcnt(0)
	s_load_b32 s8, s[8:9], 0x0
	s_xor_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB8_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s16
	s_sub_i32 s3, 0, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s3, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s16
	v_add_nc_u32_e32 v2, 1, v0
	v_sub_nc_u32_e32 v1, v3, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s16, v1
	v_cmp_le_u32_e32 vcc_lo, s16, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s16, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v2, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
.LBB8_5:
	s_or_b32 exec_lo, exec_lo, s2
	s_abs_i32 s3, s7
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v7, v1, s16
	v_cvt_f32_u32_e32 v2, s3
	s_sub_i32 s2, 0, s3
	v_mul_lo_u32 v9, v0, s4
	v_sub_nc_u32_e32 v8, 0, v0
	v_rcp_iflag_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_max_i32_e32 v8, v0, v8
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	v_cvt_u32_f32_e32 v5, v2
	v_mad_u64_u32 v[1:2], null, v0, s16, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v6, s2, v5
	s_ashr_i32 s2, s7, 31
	s_cmp_eq_u64 s[10:11], 0
	v_add3_u32 v2, v2, v9, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_co_u32 v3, vcc_lo, v3, v1
	v_mul_hi_u32 v6, v5, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_ci_u32_e64 v4, null, v4, v2, vcc_lo
	v_lshlrev_b64 v[1:2], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v7, v5, v6
	v_mad_u64_u32 v[5:6], null, v8, v7, 0
	v_ashrrev_i32_e32 v7, 31, v0
	s_cbranch_scc1 .LBB8_7
; %bb.6:
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v9, vcc_lo, s10, v1
	v_add_co_ci_u32_e64 v10, null, s11, v2, vcc_lo
	global_load_b32 v5, v[9:10], off
	s_branch .LBB8_8
.LBB8_7:
	v_mov_b32_e32 v5, 1.0
.LBB8_8:
	v_cvt_f64_u32_e32 v[9:10], v4
	v_cvt_f64_u32_e32 v[11:12], v3
	s_mov_b32 s4, 0x3e76c4e1
	v_xor_b32_e32 v7, s2, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[9:10], v[9:10], 32
	v_add_f64 v[9:10], v[9:10], v[11:12]
	v_cvt_f64_i32_e32 v[11:12], s6
	s_load_b32 s6, s[0:1], 0x28
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[9:10], -2.0
	v_div_scale_f64 v[13:14], null, v[11:12], v[11:12], v[9:10]
	v_div_scale_f64 v[19:20], vcc_lo, v[9:10], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[15:16], v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[17:18], -v[13:14], v[15:16], 1.0
	v_fma_f64 v[15:16], v[15:16], v[17:18], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], -v[13:14], v[15:16], 1.0
	v_fma_f64 v[15:16], v[15:16], v[17:18], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[19:20], v[15:16]
	v_fma_f64 v[13:14], -v[13:14], v[17:18], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[13:14], v[13:14], v[15:16], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f32_e64 vcc_lo, s8, 1.0
	v_div_fixup_f64 v[9:10], v[13:14], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_f64_e32 v4, v[9:10]
	v_cndmask_b32_e32 v4, 1.0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_neq_f32_e32 vcc_lo, 0, v4
	v_cndmask_b32_e64 v11, 1.0, s8, vcc_lo
	v_frexp_mant_f32_e64 v9, |v11|
	v_cmp_lt_f32_e64 s8, |v11|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v9
	v_cndmask_b32_e64 v10, 1.0, 2.0, vcc_lo
	v_mul_f32_e32 v9, v9, v10
	v_cmp_neq_f32_e64 s7, v4, |v4|
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f32_e32 v10, 1.0, v9
	v_add_f32_e32 v13, -1.0, v9
	s_xor_b32 s7, s7, s8
	v_add_f32_e32 v15, -1.0, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v9, v15
	v_rcp_f32_e32 v12, v10
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v14, v13, v12
	v_mul_f32_e32 v16, v10, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v10, v14, v10, -v16
	v_fmac_f32_e32 v10, v14, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v16, v10
	v_sub_f32_e32 v16, v9, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v15, v13, v9 :: v_dual_sub_f32 v10, v16, v10
	v_sub_f32_e32 v13, v13, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v13, v9
	v_add_f32_e32 v9, v10, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v15, v9
	v_mul_f32_e32 v9, v12, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v12, v14, v9
	v_sub_f32_e32 v10, v12, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mul_f32 v13, v12, v12 :: v_dual_sub_f32 v14, v9, v10
	v_fma_f32 v15, v12, v12, -v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v14, v14
	v_fmac_f32_e32 v15, v12, v9
	v_cvt_f64_f32_e64 v[9:10], |v11|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v16, v13, v15
	v_fmaak_f32 v17, s4, v16, 0x3e91f4c4
	v_sub_f32_e32 v13, v16, v13
	v_cmp_eq_f32_e64 s4, 0, v11
	v_mul_f32_e32 v20, v12, v16
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fmaak_f32 v17, v16, v17, 0x3ecccdef
	v_sub_f32_e32 v13, v15, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v18, v16, v17
	v_fma_f32 v15, v16, v17, -v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v15, v13, v17
	v_add_f32_e32 v17, v18, v15
	v_frexp_exp_i32_f64_e32 v9, v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v18, v17, v18
	v_sub_f32_e32 v10, v15, v18
	v_fma_f32 v18, v16, v12, -v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v10, 0x31739010, v10
	v_fmac_f32_e32 v18, v16, v14
	v_ldexp_f32 v14, v14, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v18, v13, v12 :: v_dual_add_f32 v19, 0x3f2aaaaa, v17
	v_add_f32_e32 v15, 0xbf2aaaaa, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v15, v17, v15
	v_dual_add_f32 v10, v10, v15 :: v_dual_add_f32 v15, v20, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v13, v19, v10
	v_subrev_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	v_sub_f32_e32 v16, v19, v13
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v19, v15, v20
	v_cvt_f32_i32_e32 v9, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v10, v10, v16
	v_dual_sub_f32 v18, v18, v19 :: v_dual_mul_f32 v17, v15, v13
	v_mul_lo_u32 v19, v6, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v16, v15, v13, -v17
	v_sub_nc_u32_e32 v8, v8, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmac_f32_e32 v16, v15, v10
	v_ldexp_f32 v10, v12, 1
	v_subrev_nc_u32_e32 v19, s3, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v16, v18, v13
	v_add_f32_e32 v12, v17, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v13, v10, v12
	v_dual_sub_f32 v10, v13, v10 :: v_dual_sub_f32 v15, v12, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v10, v12, v10
	v_sub_f32_e32 v15, v16, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mul_f32 v17, 0x3f317218, v9 :: v_dual_add_f32 v12, v14, v15
	v_fma_f32 v16, 0x3f317218, v9, -v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v10, v12, v10 :: v_dual_fmamk_f32 v9, v9, 0xb102e308, v16
	v_add_f32_e32 v14, v13, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v12, v17, v9
	v_sub_f32_e32 v13, v14, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v15, v12, v14 :: v_dual_sub_f32 v10, v10, v13
	v_dual_sub_f32 v16, v15, v12 :: v_dual_sub_f32 v17, v12, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v18, v15, v16
	v_dual_sub_f32 v13, v14, v16 :: v_dual_sub_f32 v12, v12, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f32_e32 v12, v13, v12
	v_sub_f32_e32 v9, v9, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v14, v9, v10
	v_add_f32_e32 v12, v14, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v13, v14, v9 :: v_dual_add_f32 v16, v15, v12
	v_sub_f32_e32 v14, v14, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_sub_f32 v10, v10, v13 :: v_dual_sub_f32 v13, v16, v15
	v_sub_f32_e32 v9, v9, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v9, v10, v9
	v_sub_f32_e32 v10, v12, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v9, v10
	v_add_f32_e32 v10, v16, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v12, v10, v16 :: v_dual_mul_f32 v13, v4, v10
	v_sub_f32_e32 v9, v9, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v10, v4, v10, -v13
	v_cmp_class_f32_e64 vcc_lo, v13, 0x204
	v_fmac_f32_e32 v10, v4, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v13, v10
	v_cndmask_b32_e32 v12, v9, v13, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_eq_f32_e32 vcc_lo, 0x42b17218, v12
	v_cndmask_b32_e64 v14, 0, 0x37000000, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s3, v8
	v_sub_f32_e32 v15, v12, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v16, 0x3fb8aa3b, v15
	v_fma_f32 v17, 0x3fb8aa3b, v15, -v16
	v_rndne_f32_e32 v18, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmamk_f32 v17, v15, 0x32a5705f, v17 :: v_dual_sub_f32 v16, v16, v18
	v_dual_add_f32 v16, v16, v17 :: v_dual_add_nc_u32 v17, 1, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v16, v16
	v_cndmask_b32_e32 v6, v6, v17, vcc_lo
	v_sub_f32_e32 v9, v9, v13
	v_cvt_i32_f32_e32 v13, v18
	v_cndmask_b32_e32 v8, v8, v19, vcc_lo
	v_cmp_neq_f32_e64 vcc_lo, 0x7f800000, |v12|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_sub_f32 v9, v10, v9 :: v_dual_add_nc_u32 v10, 1, v6
	v_cmp_le_u32_e64 s3, s3, v8
	s_delay_alu instid0(TRANS32_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f32 v13, v16, v13
	v_dual_mul_f32 v16, 0.5, v4 :: v_dual_cndmask_b32 v9, 0, v9
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v15
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v6, v6, v10, s3
	v_trunc_f32_e32 v17, v16
	v_cndmask_b32_e32 v12, 0, v13, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v15
	v_trunc_f32_e32 v13, v4
	v_add_f32_e32 v9, v14, v9
	v_cmp_neq_f32_e64 s2, v17, v16
	v_xor_b32_e32 v6, v6, v7
	v_cndmask_b32_e32 v12, 0x7f800000, v12, vcc_lo
	v_cmp_eq_f32_e32 vcc_lo, v13, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v6, v6, v7
	v_fma_f32 v8, v12, v9, v12
	v_cmp_class_f32_e64 s3, v12, 0x204
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e64 v9, 1.0, v11, s2
	v_cndmask_b32_e64 v13, 0, v11, s2
	v_cndmask_b32_e64 v8, v8, v12, s3
	v_cmp_gt_f32_e64 s3, 0, v4
	v_cmp_class_f32_e64 s2, v11, 0x204
	v_add_nc_u32_e32 v6, s6, v6
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_bfi_b32 v8, 0x7fffffff, v8, v9
	v_cndmask_b32_e64 v9, 0x7f800000, 0, s7
	s_xor_b32 s3, s3, s4
	v_cvt_f32_i32_e32 v6, v6
	v_cndmask_b32_e64 v10, 0x7f800000, 0, s3
	v_cndmask_b32_e32 v12, 0x7fc00000, v8, vcc_lo
	v_cmp_neq_f32_e64 vcc_lo, |v11|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_bfi_b32 v7, 0x7fffffff, v10, v13
	v_cndmask_b32_e32 v9, 1.0, v9, vcc_lo
	v_cmp_gt_f32_e32 vcc_lo, 0, v11
	v_cndmask_b32_e32 v8, v8, v12, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v4, 0x204
	v_cndmask_b32_e32 v8, v8, v9, vcc_lo
	s_or_b32 vcc_lo, s4, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, v8, v7, vcc_lo
	v_cmp_o_f32_e32 vcc_lo, v11, v4
	v_cndmask_b32_e32 v4, 0x7fc00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, v4, v6
	s_waitcnt vmcnt(0)
	v_div_scale_f32 v6, null, v5, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v7, v6
	s_waitcnt_depctr 0xfff
	v_fma_f32 v8, -v6, v7, 1.0
	v_fmac_f32_e32 v7, v8, v7
	v_div_scale_f32 v8, vcc_lo, v4, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v9, v8, v7
	v_fma_f32 v10, -v6, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v9, v10, v7
	v_fma_f32 v6, -v6, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v6, v6, v7, v9
                                        ; implicit-def: $vgpr7
	v_div_fixup_f32 v4, v6, v5, v4
                                        ; implicit-def: $vgpr6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_and_b32_e32 v5, 0x7fffffff, v4
	v_cmp_ngt_f32_e64 s4, 0x48000000, |v4|
	v_lshrrev_b32_e32 v8, 23, v5
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s6, exec_lo, s2
	s_cbranch_execz .LBB8_10
; %bb.9:
	s_mov_b32 s2, 0x7fffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_or_b32 v18, v5, s2, 0x800000
	v_mad_u64_u32 v[6:7], null, 0xfe5163ab, v18, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_mov_b32 v10, 0 :: v_dual_mov_b32 v9, v7
	v_add_nc_u32_e32 v7, 0xffffff88, v8
	v_mad_u64_u32 v[11:12], null, 0x3c439041, v18, v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v7
	v_cndmask_b32_e64 v16, 0, 0xffffffc0, vcc_lo
	v_mov_b32_e32 v9, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v7, v16, v7
	v_mad_u64_u32 v[12:13], null, 0xdb629599, v18, v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s2, 31, v7
	v_mov_b32_e32 v9, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v17, 0, 0xffffffe0, s2
	v_cndmask_b32_e32 v6, v12, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[13:14], null, 0xf534ddc0, v18, v[9:10]
	v_add_nc_u32_e32 v7, v17, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e64 s3, 31, v7
	v_mov_b32_e32 v9, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[14:15], null, 0xfc2757d1, v18, v[9:10]
	v_mov_b32_e32 v9, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[15:16], null, 0x4e441529, v18, v[9:10]
	v_mov_b32_e32 v9, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[16:17], null, 0xa2f9836e, v18, v[9:10]
	v_cndmask_b32_e64 v9, 0, 0xffffffe0, s3
	v_dual_cndmask_b32 v10, v15, v13 :: v_dual_add_nc_u32 v7, v9, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v16, v16, v14 :: v_dual_cndmask_b32 v15, v17, v15
	v_dual_cndmask_b32 v14, v14, v12 :: v_dual_cndmask_b32 v9, v13, v11
	v_cmp_eq_u32_e32 vcc_lo, 0, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v11, v16, v10, s2
	v_cndmask_b32_e64 v13, v15, v16, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v10, v10, v14, s2
	v_sub_nc_u32_e32 v15, 32, v7
	v_cndmask_b32_e64 v14, v14, v9, s2
	v_cndmask_b32_e64 v6, v9, v6, s2
	v_cndmask_b32_e64 v13, v13, v11, s3
	v_cndmask_b32_e64 v11, v11, v10, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v10, v10, v14, s3
	v_cndmask_b32_e64 v6, v14, v6, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v16, v13, v11, v15.l
	v_alignbit_b32 v12, v11, v10, v15.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v14, v10, v6, v15.l
	v_cndmask_b32_e32 v7, v16, v13, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v9, v12, v11 :: v_dual_cndmask_b32 v10, v14, v10
	v_bfe_u32 v11, v7, 29, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v12, v7, v9, 30
	v_alignbit_b32 v9, v9, v10, 30
	v_alignbit_b32 v6, v10, v6, 30
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v13, 0, v11
	v_xor_b32_e32 v12, v12, v13
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v9, v9, v13
	v_xor_b32_e32 v6, v6, v13
	v_lshrrev_b32_e32 v13, 29, v7
	v_lshrrev_b32_e32 v7, 30, v7
	v_clz_i32_u32_e32 v14, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v7, v11, v7
	v_min_u32_e32 v14, 32, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v10, 31, v14
	v_lshlrev_b32_e32 v15, 23, v14
	v_alignbit_b32 v12, v12, v9, v10.l
	v_alignbit_b32 v6, v9, v6, v10.l
	v_lshlrev_b32_e32 v9, 31, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_alignbit_b32 v10, v12, v6, 9
	v_or_b32_e32 v13, 0.5, v9
	v_lshrrev_b32_e32 v12, 9, v12
	v_or_b32_e32 v9, 0x33000000, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_clz_i32_u32_e32 v16, v10
	v_sub_nc_u32_e32 v13, v13, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_min_u32_e32 v15, 32, v16
	v_or_b32_e32 v12, v12, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_not_b32_e32 v13, v15
	v_mul_f32_e32 v16, 0x3fc90fda, v12
	v_add_lshl_u32 v14, v15, v14, 23
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v6, v10, v6, v13.l
	v_fma_f32 v10, 0x3fc90fda, v12, -v16
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v9, v9, v14
	v_lshrrev_b32_e32 v6, 9, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmamk_f32 v10, v12, 0x33a22168, v10
	v_or_b32_e32 v6, v9, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v10, 0x3fc90fda, v6
	v_add_f32_e32 v6, v16, v10
	s_or_saveexec_b32 s2, s6
	v_mul_f32_e64 v11, 0x3f22f983, |v4|
	s_xor_b32 exec_lo, exec_lo, s2
	s_branch .LBB8_11
.LBB8_10:
	s_or_saveexec_b32 s2, s6
	v_mul_f32_e64 v11, 0x3f22f983, |v4|
	s_xor_b32 exec_lo, exec_lo, s2
.LBB8_11:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f32_e32 v7, v11
	v_fma_f32 v6, 0xbfc90fda, v7, |v4|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmamk_f32 v6, v7, 0xb3a22168, v6
	v_fmamk_f32 v6, v7, 0xa7c234c4, v6
	v_cvt_i32_f32_e32 v7, v7
; %bb.12:
	s_or_b32 exec_lo, exec_lo, s2
                                        ; implicit-def: $vgpr10
                                        ; implicit-def: $vgpr9
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB8_14
; %bb.13:
	s_mov_b32 s2, 0x7fffff
	v_mov_b32_e32 v11, 0
	v_and_or_b32 v19, v5, s2, 0x800000
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[9:10], null, 0xfe5163ab, v19, 0
	v_mad_u64_u32 v[12:13], null, 0x3c439041, v19, v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v10, v13
	v_mad_u64_u32 v[13:14], null, 0xdb629599, v19, v[10:11]
	v_add_nc_u32_e32 v8, 0xffffff88, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v8
	v_mov_b32_e32 v10, v14
	v_cndmask_b32_e64 v17, 0, 0xffffffc0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[14:15], null, 0xf534ddc0, v19, v[10:11]
	v_cndmask_b32_e32 v9, v13, v9, vcc_lo
	v_add_nc_u32_e32 v8, v17, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v10, v15
	v_cmp_lt_u32_e64 s2, 31, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[15:16], null, 0xfc2757d1, v19, v[10:11]
	v_cndmask_b32_e64 v18, 0, 0xffffffe0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v8, v18, v8
	v_mov_b32_e32 v10, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s3, 31, v8
	v_mad_u64_u32 v[16:17], null, 0x4e441529, v19, v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v10, v17
	v_mad_u64_u32 v[17:18], null, 0xa2f9836e, v19, v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v10, 0, 0xffffffe0, s3
	v_dual_cndmask_b32 v11, v16, v14 :: v_dual_add_nc_u32 v8, v10, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v17, v17, v15 :: v_dual_cndmask_b32 v16, v18, v16
	v_dual_cndmask_b32 v15, v15, v13 :: v_dual_cndmask_b32 v10, v14, v12
	v_cmp_eq_u32_e32 vcc_lo, 0, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v12, v17, v11, s2
	v_cndmask_b32_e64 v14, v16, v17, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v11, v11, v15, s2
	v_sub_nc_u32_e32 v16, 32, v8
	v_cndmask_b32_e64 v15, v15, v10, s2
	v_cndmask_b32_e64 v9, v10, v9, s2
	v_cndmask_b32_e64 v14, v14, v12, s3
	v_cndmask_b32_e64 v12, v12, v11, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v11, v11, v15, s3
	v_cndmask_b32_e64 v9, v15, v9, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v17, v14, v12, v16.l
	v_alignbit_b32 v13, v12, v11, v16.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v15, v11, v9, v16.l
	v_cndmask_b32_e32 v8, v17, v14, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v10, v13, v12 :: v_dual_cndmask_b32 v11, v15, v11
	v_bfe_u32 v12, v8, 29, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v13, v8, v10, 30
	v_alignbit_b32 v10, v10, v11, 30
	v_alignbit_b32 v9, v11, v9, 30
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v14, 0, v12
	v_xor_b32_e32 v13, v13, v14
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v10, v10, v14
	v_xor_b32_e32 v9, v9, v14
	v_lshrrev_b32_e32 v14, 29, v8
	v_lshrrev_b32_e32 v8, 30, v8
	v_clz_i32_u32_e32 v15, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_min_u32_e32 v15, 32, v15
	v_sub_nc_u32_e32 v11, 31, v15
	v_lshlrev_b32_e32 v16, 23, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_alignbit_b32 v13, v13, v10, v11.l
	v_alignbit_b32 v9, v10, v9, v11.l
	v_lshlrev_b32_e32 v10, 31, v14
	v_alignbit_b32 v11, v13, v9, 9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_or_b32_e32 v14, 0.5, v10
	v_lshrrev_b32_e32 v13, 9, v13
	v_or_b32_e32 v10, 0x33000000, v10
	v_clz_i32_u32_e32 v17, v11
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v14, v14, v16
	v_min_u32_e32 v16, 32, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_or_b32_e32 v13, v13, v14
	v_not_b32_e32 v14, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v17, 0x3fc90fda, v13
	v_add_lshl_u32 v15, v16, v15, 23
	v_alignbit_b32 v9, v11, v9, v14.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f32 v11, 0x3fc90fda, v13, -v17
	v_sub_nc_u32_e32 v10, v10, v15
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshrrev_b32_e32 v9, 9, v9
	v_fmamk_f32 v11, v13, 0x33a22168, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_or_b32_e32 v9, v10, v9
	v_dual_fmac_f32 v11, 0x3fc90fda, v9 :: v_dual_add_nc_u32 v10, v12, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v9, v17, v11
                                        ; implicit-def: $vgpr11
	s_and_not1_saveexec_b32 s2, s4
	s_cbranch_execnz .LBB8_15
	s_branch .LBB8_16
.LBB8_14:
	s_and_not1_saveexec_b32 s2, s4
.LBB8_15:
	v_rndne_f32_e32 v8, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v9, 0xbfc90fda, v8, |v4|
	v_cvt_i32_f32_e32 v10, v8
	v_fmamk_f32 v9, v8, 0xb3a22168, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_fmamk_f32 v9, v8, 0xa7c234c4, v9
.LBB8_16:
	s_or_b32 exec_lo, exec_lo, s2
	s_load_b64 s[0:1], s[0:1], 0x0
	v_mad_i64_i32 v[11:12], null, v0, s5, 0
	v_dual_mul_f32 v8, v6, v6 :: v_dual_add_nc_u32 v13, s16, v3
	v_xor_b32_e32 v5, v5, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ashrrev_i32_e32 v14, 31, v13
	v_lshlrev_b64 v[11:12], 2, v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[13:14], 2, v[13:14]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v12, vcc_lo
	s_mov_b32 s0, 0xb94c1982
	v_add_co_u32 v11, vcc_lo, v0, v13
	v_add_co_ci_u32_e64 v12, null, v3, v14, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v3, v2, vcc_lo
	s_clause 0x1
	global_load_b32 v2, v[11:12], off
	global_load_b32 v3, v[0:1], off
	v_dual_mul_f32 v14, v9, v9 :: v_dual_and_b32 v13, 1, v7
	s_mov_b32 s1, 0x37d75334
	v_dual_fmaak_f32 v16, s0, v8, 0x3c0881c4 :: v_dual_lshlrev_b32 v7, 30, v7
	v_fmaak_f32 v17, s1, v8, 0xbab64f3b
	s_delay_alu instid0(VALU_DEP_3)
	v_fmaak_f32 v18, s0, v14, 0x3c0881c4
	v_and_b32_e32 v15, 1, v10
	v_lshlrev_b32_e32 v10, 30, v10
	v_fmaak_f32 v16, v8, v16, 0xbe2aaa9d
	v_fmaak_f32 v19, s1, v14, 0xbab64f3b
	v_fmaak_f32 v18, v14, v18, 0xbe2aaa9d
	v_fmaak_f32 v17, v8, v17, 0x3d2aabf7
	v_cmp_eq_u32_e32 vcc_lo, 0, v13
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_mul_f32 v16, v8, v16 :: v_dual_fmaak_f32 v19, v14, v19, 0x3d2aabf7
	v_dual_mul_f32 v18, v14, v18 :: v_dual_fmaak_f32 v17, v8, v17, 0xbf000004
	v_and_b32_e32 v10, 0x80000000, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fmac_f32_e32 v6, v6, v16
	v_fmaak_f32 v19, v14, v19, 0xbf000004
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmac_f32_e32 v9, v9, v18
	v_fma_f32 v8, v8, v17, 1.0
	v_fma_f32 v14, v14, v19, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v6, -v6, v8, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, 0, v15
	v_dual_cndmask_b32 v8, v14, v9 :: v_dual_and_b32 v7, 0x80000000, v7
	v_cmp_class_f32_e64 vcc_lo, v4, 0x1f8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_xor_b32_e32 v6, v7, v6
	v_xor3_b32 v5, v5, v10, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0x7fc00000, v6, vcc_lo
	v_cndmask_b32_e32 v5, 0x7fc00000, v5, vcc_lo
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v6, v2, v5
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v5, v3, v5
	v_fma_f32 v3, v4, v3, -v6
	s_delay_alu instid0(VALU_DEP_2)
	v_fmac_f32_e32 v5, v4, v2
	s_clause 0x1
	global_store_b32 v[0:1], v3, off
	global_store_b32 v[11:12], v5, off
.LBB8_17:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z12rope_partialIfEvPT_iiiiPKS0_S3_i
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
		.amdhsa_next_free_vgpr 21
		.amdhsa_next_free_sgpr 26
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
	.section	.text._Z12rope_partialIfEvPT_iiiiPKS0_S3_i,"axG",@progbits,_Z12rope_partialIfEvPT_iiiiPKS0_S3_i,comdat
.Lfunc_end8:
	.size	_Z12rope_partialIfEvPT_iiiiPKS0_S3_i, .Lfunc_end8-_Z12rope_partialIfEvPT_iiiiPKS0_S3_i
                                        ; -- End function
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.num_vgpr, 21
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.num_agpr, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.numbered_sgpr, 26
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.num_named_barrier, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.private_seg_size, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.uses_vcc, 1
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.uses_flat_scratch, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.has_dyn_sized_stack, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.has_recursion, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4712
; TotalNumSgprs: 28
; NumVgprs: 21
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 28
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
	.section	.text._Z12rope_partialIdEvPT_iiiiPKS0_S3_i,"axG",@progbits,_Z12rope_partialIdEvPT_iiiiPKS0_S3_i,comdat
	.protected	_Z12rope_partialIdEvPT_iiiiPKS0_S3_i ; -- Begin function _Z12rope_partialIdEvPT_iiiiPKS0_S3_i
	.globl	_Z12rope_partialIdEvPT_iiiiPKS0_S3_i
	.p2align	8
	.type	_Z12rope_partialIdEvPT_iiiiPKS0_S3_i,@function
_Z12rope_partialIdEvPT_iiiiPKS0_S3_i:   ; @_Z12rope_partialIdEvPT_iiiiPKS0_S3_i
; %bb.0:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x8
	s_load_b32 s3, s[0:1], 0x3c
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v5
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s4, s10, 31
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[3:4], null, s3, s2, v[0:1]
	s_add_i32 s2, s10, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s16, s2, 1
	s_mul_hi_i32 s3, s16, s8
	s_mul_i32 s2, s16, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[3:4]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB9_17
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x18
	s_ashr_i32 s17, s16, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v6, s17, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[5:6]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB9_3
; %bb.2:
	s_ashr_i32 s12, s17, 31
	v_ashrrev_i32_e32 v2, 31, v4
	s_add_u32 s14, s16, s12
	s_mov_b32 s13, s12
	s_addc_u32 s15, s17, s12
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[14:15], s[14:15], s[12:13]
	v_add_co_u32 v5, vcc_lo, v3, v2
	v_cvt_f32_u32_e32 v0, s14
	v_cvt_f32_u32_e32 v1, s15
	s_sub_u32 s13, 0, s14
	s_subb_u32 s18, 0, s15
	v_add_co_ci_u32_e64 v6, null, v4, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v5, v2
	v_xor_b32_e32 v10, v6, v2
	v_xor_b32_e32 v2, s12, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x5f7ffffc, v0
	v_mul_f32_e32 v1, 0x2f800000, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v1, v1
	v_fmamk_f32 v0, v1, 0xcf800000, v0
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s2, v1
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s8, v0
	s_mul_i32 s19, s13, s2
	s_mul_hi_u32 s21, s13, s8
	s_mul_i32 s20, s18, s8
	s_add_i32 s19, s21, s19
	s_mul_i32 s22, s13, s8
	s_add_i32 s19, s19, s20
	s_mul_hi_u32 s21, s8, s22
	s_mul_i32 s24, s8, s19
	s_mul_hi_u32 s23, s2, s22
	s_mul_i32 s20, s2, s22
	s_mul_hi_u32 s22, s8, s19
	s_add_u32 s21, s21, s24
	s_addc_u32 s22, 0, s22
	s_mul_hi_u32 s25, s2, s19
	s_add_u32 s20, s21, s20
	s_mul_i32 s19, s2, s19
	s_addc_u32 s20, s22, s23
	s_addc_u32 s21, s25, 0
	s_add_u32 s19, s20, s19
	s_addc_u32 s20, 0, s21
	s_add_u32 s8, s8, s19
	s_cselect_b32 s19, -1, 0
	s_mul_hi_u32 s21, s13, s8
	s_cmp_lg_u32 s19, 0
	s_mul_i32 s19, s13, s8
	s_addc_u32 s2, s2, s20
	s_mul_i32 s18, s18, s8
	s_mul_i32 s13, s13, s2
	s_mul_hi_u32 s20, s8, s19
	s_add_i32 s13, s21, s13
	s_mul_hi_u32 s21, s2, s19
	s_add_i32 s13, s13, s18
	s_mul_i32 s18, s2, s19
	s_mul_i32 s23, s8, s13
	s_mul_hi_u32 s22, s8, s13
	s_add_u32 s20, s20, s23
	s_addc_u32 s22, 0, s22
	s_mul_hi_u32 s19, s2, s13
	s_add_u32 s18, s20, s18
	s_mul_i32 s13, s2, s13
	s_addc_u32 s18, s22, s21
	s_addc_u32 s19, s19, 0
	s_add_u32 s13, s18, s13
	s_addc_u32 s18, 0, s19
	s_add_u32 s8, s8, s13
	s_cselect_b32 s13, -1, 0
	v_mul_hi_u32 v11, v9, s8
	s_cmp_lg_u32 s13, 0
	v_mad_u64_u32 v[5:6], null, v10, s8, 0
	s_addc_u32 s2, s2, s18
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[7:8], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v6, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v8, vcc_lo
	v_add_co_u32 v5, vcc_lo, v0, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v6, null, 0, v1, vcc_lo
	v_mul_lo_u32 v7, s15, v5
	v_mad_u64_u32 v[0:1], null, s14, v5, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v8, s14, v6
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v8, v7
	v_add_co_u32 v8, s2, v5, 2
	v_add_co_ci_u32_e64 v9, null, 0, v6, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v10, v1
	v_sub_co_u32 v11, s2, v0, s14
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s15, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s14, v11
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s15, v7
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s14, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s15, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s15, v7
	v_cndmask_b32_e32 v7, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v5, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v11, null, 0, v6, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s15, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v7
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v8 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v5, v0, s2
	v_cndmask_b32_e64 v1, v6, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v2
	v_xor_b32_e32 v1, v1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v2
	v_sub_co_ci_u32_e64 v1, null, v1, v2, vcc_lo
.LBB9_3:
	s_or_saveexec_b32 s8, s3
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[4:5], 0x0
	s_xor_b32 exec_lo, exec_lo, s8
	s_cbranch_execz .LBB9_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s16
	s_sub_i32 s4, 0, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s4, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s16
	v_add_nc_u32_e32 v2, 1, v0
	v_sub_nc_u32_e32 v1, v3, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s16, v1
	v_cmp_le_u32_e32 vcc_lo, s16, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s16, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v2, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
.LBB9_5:
	s_or_b32 exec_lo, exec_lo, s8
	s_abs_i32 s8, s11
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_lo_u32 v7, v1, s16
	v_cvt_f32_u32_e32 v2, s8
	s_sub_i32 s4, 0, s8
	v_mul_lo_u32 v8, v0, s17
	v_sub_nc_u32_e32 v9, 0, v0
	v_ashrrev_i32_e32 v12, 31, v0
	v_rcp_iflag_f32_e32 v2, v2
	s_ashr_i32 s11, s11, 31
	s_cmp_eq_u64 s[6:7], 0
	v_max_i32_e32 v13, v0, v9
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v5, v2
	v_mad_u64_u32 v[1:2], null, v0, s16, 0
	v_mul_lo_u32 v6, s4, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add3_u32 v2, v2, v8, v7
	v_sub_co_u32 v3, vcc_lo, v3, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_hi_u32 v6, v5, v6
	v_sub_co_ci_u32_e64 v4, null, v4, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[1:2], 3, v[3:4]
	v_add_nc_u32_e32 v5, v5, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_mad_u64_u32 v[6:7], null, v13, v5, 0
	s_cbranch_scc1 .LBB9_7
; %bb.6:
	v_add_co_u32 v5, vcc_lo, s6, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s7, v2, vcc_lo
	global_load_b64 v[5:6], v[5:6], off
	s_branch .LBB9_8
.LBB9_7:
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0x3ff00000
.LBB9_8:
	v_cvt_f64_u32_e32 v[8:9], v4
	v_cvt_f64_u32_e32 v[10:11], v3
	s_waitcnt lgkmcnt(0)
	v_mov_b32_e32 v4, s3
	s_mov_b32 s4, 0x968915a9
	s_mov_b32 s6, 0x4222de17
	s_mov_b32 s5, 0x3fba6564
	s_mov_b32 s7, 0x3fbdee67
	v_xor_b32_e32 v12, s11, v12
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], 32
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_cvt_f64_i32_e32 v[10:11], s10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[8:9], -2.0
	v_div_scale_f64 v[14:15], null, v[10:11], v[10:11], v[8:9]
	v_div_scale_f64 v[20:21], vcc_lo, v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[16:17], v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[20:21], v[16:17]
	v_fma_f64 v[14:15], -v[14:15], v[18:19], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[14:15], v[14:15], v[16:17], v[18:19]
	v_cmp_neq_f64_e64 vcc_lo, s[2:3], 1.0
	s_mov_b32 s3, 0x3fe55555
	v_div_fixup_f64 v[8:9], v[14:15], v[10:11], v[8:9]
	v_cndmask_b32_e32 v9, 0x3ff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[8:9]
	v_cndmask_b32_e32 v11, 0x3ff00000, v4, vcc_lo
	v_cndmask_b32_e64 v10, 0, s2, vcc_lo
	s_mov_b32 s2, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[14:15], |v[10:11]|
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[14:15]
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[14:15], v[14:15], v4
	v_frexp_exp_i32_f64_e32 v4, v[10:11]
	v_add_f64 v[16:17], v[14:15], 1.0
	v_add_f64 v[22:23], v[14:15], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_rcp_f64_e32 v[18:19], v[16:17]
	v_add_f64 v[24:25], v[16:17], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[16:17], v[18:19], 1.0
	v_fma_f64 v[18:19], v[20:21], v[18:19], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], -v[16:17], v[18:19], 1.0
	v_fma_f64 v[18:19], v[20:21], v[18:19], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[22:23], v[18:19]
	v_mul_f64 v[26:27], v[16:17], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[20:21], v[16:17], -v[26:27]
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[26:27], v[14:15]
	v_add_f64 v[24:25], v[22:23], -v[16:17]
	v_add_f64 v[26:27], v[16:17], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[22:23], -v[24:25]
	v_add_f64 v[14:15], v[26:27], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[22:23], -v[16:17]
	v_add_f64 v[14:15], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[24:25], v[14:15]
	v_mul_f64 v[14:15], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[20:21], v[14:15]
	v_add_f64 v[18:19], v[16:17], -v[20:21]
	v_mul_f64 v[20:21], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	v_fma_f64 v[18:19], v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[14:15], v[14:15]
	v_fma_f64 v[18:19], v[16:17], v[22:23], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[20:21], v[18:19]
	v_fma_f64 v[24:25], v[22:23], s[6:7], s[4:5]
	s_mov_b32 s4, 0x3abe935a
	s_mov_b32 s5, 0x3fbe25e4
	v_add_f64 v[20:21], v[22:23], -v[20:21]
	v_mul_f64 v[30:31], v[16:17], v[22:23]
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x47e6c9c2
	s_mov_b32 s5, 0x3fc110ef
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0xcfa74449
	s_mov_b32 s5, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x71bf3c30
	s_mov_b32 s5, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x1c7792ce
	s_mov_b32 s5, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x924920da
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x9999999c
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s5, 0x3c7abc9e
	s_mov_b32 s4, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[26:27], v[22:23], v[24:25]
	v_fma_f64 v[20:21], v[22:23], v[24:25], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[24:25], v[20:21]
	v_add_f64 v[24:25], v[26:27], v[20:21]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[28:29], v[24:25], s[2:3]
	v_add_f64 v[26:27], v[24:25], -v[26:27]
	s_mov_b32 s3, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[32:33], v[28:29], s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_fma_f64 v[26:27], v[22:23], v[16:17], -v[30:31]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_add_f64 v[24:25], v[24:25], -v[32:33]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], s[2:3]
	v_fma_f64 v[22:23], v[22:23], v[14:15], v[26:27]
	s_mov_b32 s3, 0x3fe62e42
	s_mov_b32 s2, 0xfefa39ef
	v_ldexp_f64 v[14:15], v[14:15], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], v[24:25]
	v_fma_f64 v[18:19], v[18:19], v[16:17], v[22:23]
	v_ldexp_f64 v[16:17], v[16:17], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[28:29], v[20:21]
	v_add_f64 v[24:25], v[30:31], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[28:29], -v[22:23]
	v_mul_f64 v[28:29], v[24:25], v[22:23]
	v_add_f64 v[30:31], v[24:25], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], v[26:27]
	v_fma_f64 v[26:27], v[24:25], v[22:23], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[30:31]
	v_fma_f64 v[20:21], v[24:25], v[20:21], v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[18:19], v[22:23], v[20:21]
	v_cvt_f64_i32_e32 v[22:23], v4
	v_add_f64 v[20:21], v[28:29], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[24:25], v[16:17], v[20:21]
	v_add_f64 v[26:27], v[20:21], -v[28:29]
	v_mul_f64 v[28:29], v[22:23], s[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	v_add_f64 v[18:19], v[18:19], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[26:27], v[22:23], s[2:3], -v[28:29]
	s_mov_b32 s3, 0xbfe62e42
	v_add_f64 v[16:17], v[20:21], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], v[18:19]
	v_fma_f64 v[18:19], v[22:23], s[4:5], v[26:27]
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], v[16:17]
	v_add_f64 v[16:17], v[28:29], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[24:25], v[14:15]
	v_add_f64 v[28:29], v[16:17], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], v[20:21]
	v_add_f64 v[24:25], v[20:21], -v[24:25]
	v_add_f64 v[18:19], v[18:19], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[22:23], -v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[30:31], v[22:23], -v[26:27]
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_add_f64 v[24:25], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[16:17], -v[30:31]
	v_add_f64 v[16:17], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[24:25], -v[18:19]
	v_add_f64 v[16:17], v[24:25], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[24:25], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[20:21]
	v_add_f64 v[26:27], v[22:23], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_add_f64 v[20:21], v[26:27], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], v[18:19]
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], v[16:17]
	v_add_f64 v[16:17], v[26:27], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], -v[26:27]
	v_mul_f64 v[20:21], v[8:9], v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[8:9], v[16:17], -v[20:21]
	v_cmp_class_f64_e64 vcc_lo, v[20:21], 0x204
	v_fma_f64 v[14:15], v[8:9], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[20:21], v[14:15]
	v_dual_cndmask_b32 v19, v17, v21 :: v_dual_cndmask_b32 v18, v16, v20
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[22:23], v[18:19], s[6:7]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[18:19]|
	s_load_b32 s7, s[0:1], 0x28
	v_cmp_lt_f64_e64 s6, |v[10:11]|, 1.0
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_trunc_f64_e32 v[16:17], v[8:9]
	v_rndne_f64_e32 v[22:23], v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v15, 0, v15 :: v_dual_cndmask_b32 v14, 0, v14
	v_fma_f64 v[24:25], v[22:23], s[2:3], v[18:19]
	s_mov_b32 s2, 0xfca7ab0c
	s_mov_b32 s3, 0x3e928af3
	v_cvt_i32_f64_e32 v4, v[22:23]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[24:25], v[22:23], s[4:5], v[24:25]
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], s[4:5], s[2:3]
	s_mov_b32 s2, 0x623fde64
	s_mov_b32 s3, 0x3ec71dee
	v_cmp_neq_f64_e64 s5, v[8:9], |v[8:9]|
	v_cmp_eq_f64_e64 s4, 0, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x7c89e6b0
	s_mov_b32 s3, 0x3efa0199
	s_xor_b32 s5, s5, s6
	v_cmp_class_f64_e64 s6, v[10:11], 0x204
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x14761f6e
	s_mov_b32 s3, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x11122322
	s_mov_b32 s3, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x555502a1
	s_mov_b32 s3, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x55555511
	s_mov_b32 s3, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 11
	s_mov_b32 s3, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	v_cmp_nlt_f64_e64 s2, 0x40900000, v[18:19]
	v_cmp_ngt_f64_e64 s3, 0xc090cc00, v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], 1.0
	s_and_b32 vcc_lo, s3, s2
	v_fma_f64 v[22:23], v[24:25], v[26:27], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[20:21], v[22:23], v4
	v_mul_f64 v[22:23], v[8:9], 0.5
	v_cndmask_b32_e64 v4, 0x7ff00000, v21, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[18:19], v[22:23]
	v_cndmask_b32_e32 v20, 0, v20, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[16:17], v[8:9]
	v_cndmask_b32_e64 v17, 0x7ff00000, 0, s5
	v_cndmask_b32_e64 v21, 0, v4, s3
	v_cmp_neq_f64_e64 s5, |v[10:11]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[20:21]
	v_cmp_class_f64_e64 s3, v[20:21], 0x204
	v_cmp_neq_f64_e64 s2, v[18:19], v[22:23]
	v_mul_lo_u32 v18, v7, s8
                                        ; implicit-def: $vgpr22
	v_cndmask_b32_e64 v17, 0x3ff00000, v17, s5
	v_sub_nc_u32_e32 v13, v13, v18
	v_cndmask_b32_e64 v15, v15, v21, s3
	v_cndmask_b32_e64 v14, v14, v20, s3
	v_cmp_gt_f64_e64 s3, 0, v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e64 s5, s8, v13
	v_cndmask_b32_e32 v16, 0, v14, vcc_lo
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0x3ff00000, v11, s2
	v_bfi_b32 v4, 0x7fffffff, v15, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v15, 0x7ff80000, v4, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[10:11]
	s_xor_b32 s3, s3, s4
	v_cndmask_b32_e32 v14, v14, v16, vcc_lo
	v_cndmask_b32_e32 v4, v4, v15, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[8:9], 0x204
	v_add_nc_u32_e32 v15, 1, v7
	v_subrev_nc_u32_e32 v16, s8, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, v15, s5
	v_cndmask_b32_e64 v13, v13, v16, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v15, 1, v7
	v_cmp_le_u32_e64 s5, s8, v13
	v_cndmask_b32_e64 v13, 0x7ff00000, 0, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, v15, s5
	v_cndmask_b32_e64 v15, 0, v11, s2
	s_or_b32 s2, s4, s6
	v_xor_b32_e32 v7, v7, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v7, v12
	v_bfi_b32 v12, 0x7fffffff, v13, v15
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v4, v4, v17 :: v_dual_add_nc_u32 v7, s7, v7
	v_cndmask_b32_e64 v4, v4, v12, s2
	s_or_b32 s2, s2, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_f64_i32_e32 v[7:8], v7
	v_cndmask_b32_e64 v12, v14, 0, s2
	v_cndmask_b32_e32 v9, 0, v12, vcc_lo
	v_cndmask_b32_e32 v10, 0x7ff80000, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[9:10], v[7:8]
	s_waitcnt vmcnt(0)
	v_div_scale_f64 v[9:10], null, v[5:6], v[5:6], v[7:8]
	v_div_scale_f64 v[15:16], vcc_lo, v[7:8], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[11:12], v[9:10]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[11:12], v[13:14], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[11:12], v[13:14], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[13:14], v[15:16], v[11:12]
	v_fma_f64 v[9:10], -v[9:10], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[9:10], v[9:10], v[11:12], v[13:14]
	v_div_fixup_f64 v[4:5], v[9:10], v[5:6], v[7:8]
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_ngt_f64_e64 s2, 0x41d00000, |v[4:5]|
	v_trig_preop_f64 v[18:19], |v[4:5]|, 0
	v_trig_preop_f64 v[16:17], |v[4:5]|, 1
	v_ldexp_f64 v[20:21], |v[4:5]|, 0xffffff80
	v_trig_preop_f64 v[10:11], |v[4:5]|, 2
	v_and_b32_e32 v24, 0x7fffffff, v5
	s_and_saveexec_b32 s3, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s3, exec_lo, s3
	s_cbranch_execz .LBB9_10
; %bb.9:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[4:5]|
	v_mov_b32_e32 v35, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_cndmask_b32_e32 v7, v24, v21, vcc_lo
	v_cndmask_b32_e32 v6, v4, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[18:19], v[6:7]
	v_mul_f64 v[12:13], v[16:17], v[6:7]
	v_fma_f64 v[14:15], v[18:19], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[33:34], v[16:17], v[6:7], -v[12:13]
	v_add_f64 v[22:23], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[22:23], -v[12:13]
	v_add_f64 v[29:30], v[8:9], v[22:23]
	v_add_f64 v[27:28], v[22:23], -v[25:26]
	v_add_f64 v[14:15], v[14:15], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[25:26], v[29:30], -2
	v_add_f64 v[8:9], v[29:30], -v[8:9]
	v_add_f64 v[12:13], v[12:13], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[25:26]|
	v_add_f64 v[8:9], v[22:23], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[14:15], v[12:13]
	v_fract_f64_e32 v[14:15], v[25:26]
	v_ldexp_f64 v[14:15], v[14:15], 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v15, 0, v15 :: v_dual_cndmask_b32 v14, 0, v14
	v_mul_f64 v[31:32], v[10:11], v[6:7]
	v_add_f64 v[27:28], v[31:32], v[33:34]
	v_fma_f64 v[6:7], v[10:11], v[6:7], -v[31:32]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[27:28], v[12:13]
	v_add_f64 v[25:26], v[8:9], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[29:30], v[25:26], v[14:15]
	v_add_f64 v[8:9], v[25:26], -v[8:9]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[29:30]
	v_add_f64 v[29:30], v[27:28], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[22:23], -v[8:9]
	v_cndmask_b32_e64 v36, 0, 0x40100000, vcc_lo
	v_add_f64 v[40:41], v[27:28], -v[29:30]
	v_add_f64 v[29:30], v[33:34], -v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[14:15], v[35:36]
	v_add_f64 v[36:37], v[22:23], -v[27:28]
	v_add_f64 v[33:34], v[31:32], -v[40:41]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[38:39], v[25:26], v[14:15]
	v_add_f64 v[42:43], v[22:23], -v[36:37]
	v_add_f64 v[12:13], v[12:13], -v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[29:30], v[29:30], v[33:34]
	v_cvt_i32_f64_e32 v38, v[38:39]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[27:28], v[27:28], -v[42:43]
	v_cvt_f64_i32_e32 v[36:37], v38
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[27:28]
	v_add_f64 v[14:15], v[14:15], -v[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[29:30], v[12:13]
	v_add_f64 v[27:28], v[25:26], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[27:28], -v[14:15]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[25:26], -v[12:13]
	v_cndmask_b32_e64 v36, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v22, null, 0, v38, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[27:28], -v[35:36]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[8:9], v[6:7]
	v_mul_f64 v[14:15], v[12:13], s[4:5]
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[25:26], v[12:13], s[4:5], -v[14:15]
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[12:13], s[6:7], v[25:26]
	v_fma_f64 v[8:9], v[6:7], s[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[14:15], v[8:9]
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB9_12
	s_branch .LBB9_11
.LBB9_10:
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB9_12
.LBB9_11:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[6:7], |v[4:5]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[12:13], v[6:7]
	v_fma_f64 v[6:7], v[12:13], s[4:5], |v[4:5]|
	v_mul_f64 v[8:9], v[12:13], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[12:13], s[6:7], v[6:7]
	v_add_f64 v[14:15], v[6:7], v[8:9]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], v[12:13], s[6:7], v[8:9]
	v_add_f64 v[6:7], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[12:13], s[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[22:23], v[8:9]
	v_add_f64 v[14:15], v[6:7], -v[22:23]
	v_cvt_i32_f64_e32 v22, v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
.LBB9_12:
	s_or_b32 exec_lo, exec_lo, s3
                                        ; implicit-def: $vgpr23
                                        ; implicit-def: $vgpr12_vgpr13
                                        ; implicit-def: $vgpr14_vgpr15
	s_and_saveexec_b32 s3, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s2, exec_lo, s3
	s_cbranch_execz .LBB9_14
; %bb.13:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[4:5]|
	v_mov_b32_e32 v33, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_cndmask_b32_e32 v13, v24, v21, vcc_lo
	v_cndmask_b32_e32 v12, v4, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[14:15], v[18:19], v[12:13]
	v_mul_f64 v[20:21], v[16:17], v[12:13]
	v_fma_f64 v[18:19], v[18:19], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[16:17], v[12:13], -v[20:21]
	v_add_f64 v[23:24], v[20:21], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[23:24], -v[20:21]
	v_add_f64 v[29:30], v[14:15], v[23:24]
	v_add_f64 v[27:28], v[23:24], -v[25:26]
	v_add_f64 v[18:19], v[18:19], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[25:26], v[29:30], -2
	v_add_f64 v[14:15], v[29:30], -v[14:15]
	v_add_f64 v[20:21], v[20:21], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[25:26]|
	v_add_f64 v[14:15], v[23:24], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[18:19], v[20:21]
	v_fract_f64_e32 v[20:21], v[25:26]
	v_ldexp_f64 v[20:21], v[20:21], 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v21, 0, v21 :: v_dual_cndmask_b32 v20, 0, v20
	v_mul_f64 v[31:32], v[10:11], v[12:13]
	v_add_f64 v[27:28], v[31:32], v[16:17]
	v_fma_f64 v[10:11], v[10:11], v[12:13], -v[31:32]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[23:24], v[27:28], v[18:19]
	v_add_f64 v[25:26], v[14:15], v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[29:30], v[25:26], v[20:21]
	v_add_f64 v[12:13], v[25:26], -v[14:15]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[29:30]
	v_add_f64 v[29:30], v[27:28], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[23:24], -v[12:13]
	v_cndmask_b32_e64 v34, 0, 0x40100000, vcc_lo
	v_add_f64 v[38:39], v[27:28], -v[29:30]
	v_add_f64 v[16:17], v[16:17], -v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], v[33:34]
	v_add_f64 v[34:35], v[23:24], -v[27:28]
	v_add_f64 v[29:30], v[31:32], -v[38:39]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[36:37], v[25:26], v[20:21]
	v_add_f64 v[40:41], v[23:24], -v[34:35]
	v_add_f64 v[18:19], v[18:19], -v[34:35]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[16:17], v[29:30]
	v_cvt_i32_f64_e32 v36, v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[27:28], v[27:28], -v[40:41]
	v_cvt_f64_i32_e32 v[34:35], v36
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], v[27:28]
	v_add_f64 v[20:21], v[20:21], -v[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[16:17], v[18:19]
	v_add_f64 v[16:17], v[25:26], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[14:15], v[16:17], -v[20:21]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[12:13], v[10:11]
	v_add_f64 v[12:13], v[25:26], -v[14:15]
	v_cndmask_b32_e64 v34, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v23, null, 0, v36, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[16:17], -v[33:34]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_mul_f64 v[16:17], v[14:15], s[4:5]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], s[4:5], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], s[6:7], v[18:19]
	v_fma_f64 v[10:11], v[10:11], s[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], v[10:11]
	v_add_f64 v[14:15], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[14:15], v[10:11], -v[14:15]
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execnz .LBB9_15
	s_branch .LBB9_16
.LBB9_14:
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execz .LBB9_16
.LBB9_15:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[10:11], |v[4:5]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], s[4:5], |v[4:5]|
	v_mul_f64 v[14:15], v[10:11], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	v_cvt_i32_f64_e32 v23, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[10:11], s[6:7], v[12:13]
	v_add_f64 v[16:17], v[12:13], v[14:15]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[14:15]
	v_fma_f64 v[14:15], v[10:11], s[6:7], v[14:15]
	v_add_f64 v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[10:11], s[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], v[14:15]
	v_add_f64 v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
.LBB9_16:
	s_or_b32 exec_lo, exec_lo, s2
	v_mul_f64 v[10:11], v[6:7], v[6:7]
	v_mul_f64 v[16:17], v[12:13], v[12:13]
	s_load_b64 s[0:1], s[0:1], 0x0
	v_mad_i64_i32 v[18:19], null, v0, s9, 0
	v_add_nc_u32_e32 v24, s16, v3
	s_mov_b32 s2, 0xb42fdfa7
	s_mov_b32 s4, 0xf9a43bb8
	s_mov_b32 s3, 0xbe5ae600
	s_mov_b32 s5, 0x3de5e0b2
	v_ashrrev_i32_e32 v25, 31, v24
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[18:19], 3, v[18:19]
	v_mul_f64 v[40:41], v[8:9], 0.5
	v_mul_f64 v[46:47], v[14:15], 0.5
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s0, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v26, null, s1, v19, vcc_lo
	v_lshlrev_b64 v[18:19], 3, v[24:25]
	v_add_co_u32 v0, vcc_lo, v3, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v26, v2, vcc_lo
	s_mov_b32 s0, 0x9037ab78
	v_add_co_u32 v2, vcc_lo, v3, v18
	v_add_co_ci_u32_e64 v3, null, v26, v19, vcc_lo
	s_clause 0x1
	global_load_b64 v[18:19], v[0:1], off
	global_load_b64 v[24:25], v[2:3], off
	v_fma_f64 v[20:21], v[10:11], s[4:5], s[2:3]
	v_fma_f64 v[26:27], v[16:17], s[4:5], s[2:3]
	s_mov_b32 s2, 0x46cc5e42
	s_mov_b32 s4, 0x796cde01
	s_mov_b32 s1, 0x3e21eeb6
	s_mov_b32 s3, 0xbda907db
	s_mov_b32 s5, 0x3ec71de3
	v_fma_f64 v[28:29], v[10:11], s[2:3], s[0:1]
	v_mul_f64 v[30:31], v[10:11], 0.5
	v_fma_f64 v[32:33], v[16:17], s[2:3], s[0:1]
	v_mul_f64 v[34:35], v[16:17], 0.5
	s_mov_b32 s0, 0xa17f65f6
	s_mov_b32 s2, 0x19e83e5c
	s_mov_b32 s1, 0xbe927e4f
	s_mov_b32 s3, 0xbf2a01a0
	v_mul_f64 v[42:43], v[6:7], -v[10:11]
	v_mul_f64 v[48:49], v[12:13], -v[16:17]
	v_fma_f64 v[20:21], v[10:11], v[20:21], s[4:5]
	v_fma_f64 v[26:27], v[16:17], v[26:27], s[4:5]
	v_fma_f64 v[28:29], v[10:11], v[28:29], s[0:1]
	v_add_f64 v[36:37], -v[30:31], 1.0
	v_fma_f64 v[32:33], v[16:17], v[32:33], s[0:1]
	v_add_f64 v[38:39], -v[34:35], 1.0
	s_mov_b32 s0, 0x19f4ec90
	s_mov_b32 s1, 0x3efa01a0
	v_fma_f64 v[20:21], v[10:11], v[20:21], s[2:3]
	v_fma_f64 v[26:27], v[16:17], v[26:27], s[2:3]
	s_mov_b32 s2, 0x11110bb3
	s_mov_b32 s3, 0x3f811111
	v_fma_f64 v[28:29], v[10:11], v[28:29], s[0:1]
	v_add_f64 v[44:45], -v[36:37], 1.0
	v_fma_f64 v[32:33], v[16:17], v[32:33], s[0:1]
	v_add_f64 v[50:51], -v[38:39], 1.0
	s_mov_b32 s0, 0x16c16967
	s_mov_b32 s1, 0xbf56c16c
	v_fma_f64 v[20:21], v[10:11], v[20:21], s[2:3]
	v_fma_f64 v[26:27], v[16:17], v[26:27], s[2:3]
	v_fma_f64 v[28:29], v[10:11], v[28:29], s[0:1]
	v_add_f64 v[30:31], v[44:45], -v[30:31]
	v_fma_f64 v[32:33], v[16:17], v[32:33], s[0:1]
	v_add_f64 v[34:35], v[50:51], -v[34:35]
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s1, 0x3fa55555
	v_fma_f64 v[20:21], v[42:43], v[20:21], v[40:41]
	v_fma_f64 v[26:27], v[48:49], v[26:27], v[46:47]
	v_mul_f64 v[40:41], v[10:11], v[10:11]
	v_fma_f64 v[28:29], v[10:11], v[28:29], s[0:1]
	v_fma_f64 v[30:31], v[6:7], -v[8:9], v[30:31]
	v_fma_f64 v[8:9], v[10:11], v[20:21], -v[8:9]
	v_mul_f64 v[10:11], v[16:17], v[16:17]
	v_fma_f64 v[20:21], v[16:17], v[32:33], s[0:1]
	v_fma_f64 v[32:33], v[12:13], -v[14:15], v[34:35]
	v_fma_f64 v[14:15], v[16:17], v[26:27], -v[14:15]
	s_mov_b32 s1, 0xbfc55555
	v_fma_f64 v[16:17], v[40:41], v[28:29], v[30:31]
	v_fma_f64 v[8:9], v[42:43], s[0:1], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[10:11], v[10:11], v[20:21], v[32:33]
	v_fma_f64 v[14:15], v[48:49], s[0:1], v[14:15]
	v_cmp_class_f64_e64 s0, v[4:5], 0x1f8
	v_lshlrev_b32_e32 v4, 30, v23
	v_add_f64 v[16:17], v[36:37], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v4, v4, v5
	v_and_b32_e32 v4, 0x80000000, v4
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[8:9], v[38:39], v[10:11]
	v_add_f64 v[10:11], v[12:13], -v[14:15]
	v_and_b32_e32 v12, 1, v22
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v12
	v_and_b32_e32 v12, 1, v23
	v_cmp_eq_u32_e64 s1, 0, v12
	v_cndmask_b32_e32 v6, v6, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v5, v8, v10, s1
	v_cndmask_b32_e64 v8, v9, v11, s1
	v_cndmask_b32_e64 v5, 0, v5, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v8, v8, v4
	v_cndmask_b32_e64 v4, 0, v6, s0
	v_cndmask_b32_e64 v6, 0x7ff80000, v8, s0
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[24:25], v[5:6]
	v_mul_f64 v[10:11], v[18:19], v[5:6]
	v_lshlrev_b32_e32 v5, 30, v22
	v_xor_b32_e32 v6, 0x80000000, v7
	v_and_b32_e32 v5, 0x80000000, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, v6, v17, vcc_lo
	v_xor_b32_e32 v5, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0x7ff80000, v5, s0
	v_fma_f64 v[6:7], v[18:19], v[4:5], -v[8:9]
	v_fma_f64 v[4:5], v[24:25], v[4:5], v[10:11]
	s_clause 0x1
	global_store_b64 v[0:1], v[6:7], off
	global_store_b64 v[2:3], v[4:5], off
.LBB9_17:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z12rope_partialIdEvPT_iiiiPKS0_S3_i
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
		.amdhsa_next_free_vgpr 52
		.amdhsa_next_free_sgpr 26
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
		.amdhsa_inst_pref_size 52
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z12rope_partialIdEvPT_iiiiPKS0_S3_i,"axG",@progbits,_Z12rope_partialIdEvPT_iiiiPKS0_S3_i,comdat
.Lfunc_end9:
	.size	_Z12rope_partialIdEvPT_iiiiPKS0_S3_i, .Lfunc_end9-_Z12rope_partialIdEvPT_iiiiPKS0_S3_i
                                        ; -- End function
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.num_vgpr, 52
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.num_agpr, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.numbered_sgpr, 26
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.num_named_barrier, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.private_seg_size, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.uses_vcc, 1
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.uses_flat_scratch, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.has_dyn_sized_stack, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.has_recursion, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 6596
; TotalNumSgprs: 28
; NumVgprs: 52
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 6
; NumSGPRsForWavesPerEU: 28
; NumVGPRsForWavesPerEU: 52
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z8gelu_mulIfEvPKT_S2_PS0_l,"axG",@progbits,_Z8gelu_mulIfEvPKT_S2_PS0_l,comdat
	.protected	_Z8gelu_mulIfEvPKT_S2_PS0_l ; -- Begin function _Z8gelu_mulIfEvPKT_S2_PS0_l
	.globl	_Z8gelu_mulIfEvPKT_S2_PS0_l
	.p2align	8
	.type	_Z8gelu_mulIfEvPKT_S2_PS0_l,@function
_Z8gelu_mulIfEvPKT_S2_PS0_l:            ; @_Z8gelu_mulIfEvPKT_S2_PS0_l
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b256 s[4:11], s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s0, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s0, s2, v[0:1]
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_i64_e64 s[10:11], v[2:3]
	s_cbranch_execz .LBB10_2
; %bb.1:
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_mov_b32 s0, 0x6d4801f7
	s_mov_b32 s1, 0x3fa6e4e2
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_cvt_f64_f32_e32 v[2:3], v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s0, 0x33d43651
	s_mov_b32 s1, 0x3fe98845
	v_mul_f64 v[4:5], v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[4:5], v[2:3], v[2:3]
	v_mul_f64 v[2:3], v[2:3], 0.5
	v_mul_f64 v[4:5], v[4:5], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[6:7], |v[4:5]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[0:1], |v[4:5]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	v_add_f64 v[12:13], v[8:9], 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], 0
	v_add_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[12:13], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], v[10:11]
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[14:15], v[16:17], -v[10:11]
	v_mul_f64 v[16:17], v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_fma_f64 v[14:15], v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	v_add_f64 v[18:19], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	v_fma_f64 v[14:15], v[10:11], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[18:19], v[12:13]
	v_fma_f64 v[16:17], v[18:19], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[16:17]
	v_add_f64 v[14:15], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], v[14:15]
	v_add_f64 v[18:19], v[14:15], -v[20:21]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_cvt_i32_f64_e32 v18, v[6:7]
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[14:15], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], -1.0
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], v[8:9]
	v_ldexp_f64 v[10:11], v[6:7], v18
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[10:11]
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v18
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[12:13], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	v_fma_f64 v[14:15], v[12:13], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[6:7], v[14:15]
	v_add_f64 v[16:17], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], -v[16:17], 1.0
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	v_add_f64 v[20:21], -v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[14:15], v[20:21], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[16:17], v[12:13], v[14:15]
	v_add_f64 v[18:19], v[18:19], -v[14:15]
	v_mul_f64 v[20:21], v[10:11], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[18:19]
	v_fma_f64 v[22:23], v[16:17], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[16:17], v[6:7], v[22:23]
	v_add_f64 v[24:25], v[20:21], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[14:15], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[12:13], v[16:17]
	v_add_f64 v[8:9], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], -v[12:13]
	v_add_f64 v[8:9], v[26:27], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[10:11], v[12:13]
	v_add_f64 v[14:15], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[10:11], -v[12:13]
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[12:13], -v[18:19]
	v_add_f64 v[18:19], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_add_f64 v[14:15], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_rcp_f64_e32 v[22:23], v[18:19]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[18:19], v[22:23], 1.0
	v_fma_f64 v[10:11], v[12:13], v[22:23], v[22:23]
	v_add_f64 v[12:13], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[18:19], v[10:11], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_mul_f64 v[22:23], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[10:11], v[18:19], -v[22:23]
	v_fma_f64 v[14:15], v[10:11], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[22:23], v[14:15]
	v_add_f64 v[18:19], v[12:13], -v[16:17]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[12:13], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], -v[14:15]
	v_add_co_u32 v14, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v15, null, s7, v1, vcc_lo
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[4:5]|
	global_load_b32 v14, v[14:15], off
	v_add_f64 v[6:7], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_and_b32_e32 v8, 0x7fffffff, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_cndmask_b32_e32 v7, 0x3ff00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[4:5]|
	v_cndmask_b32_e32 v4, v6, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, v7, v8, vcc_lo
	v_add_co_u32 v0, vcc_lo, s8, v0
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_bfi_b32 v5, 0x7fffffff, v6, v5
	v_add_f64 v[4:5], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	v_cvt_f32_f64_e32 v2, v[2:3]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f32_e32 v2, v14, v2
	global_store_b32 v[0:1], v2, off
.LBB10_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8gelu_mulIfEvPKT_S2_PS0_l
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
		.amdhsa_next_free_vgpr 28
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
		.amdhsa_inst_pref_size 15
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z8gelu_mulIfEvPKT_S2_PS0_l,"axG",@progbits,_Z8gelu_mulIfEvPKT_S2_PS0_l,comdat
.Lfunc_end10:
	.size	_Z8gelu_mulIfEvPKT_S2_PS0_l, .Lfunc_end10-_Z8gelu_mulIfEvPKT_S2_PS0_l
                                        ; -- End function
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.num_vgpr, 28
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.num_agpr, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.numbered_sgpr, 12
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.num_named_barrier, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.private_seg_size, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.uses_vcc, 1
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.uses_flat_scratch, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.has_dyn_sized_stack, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.has_recursion, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1900
; TotalNumSgprs: 14
; NumVgprs: 28
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 28
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z8gelu_mulIdEvPKT_S2_PS0_l,"axG",@progbits,_Z8gelu_mulIdEvPKT_S2_PS0_l,comdat
	.protected	_Z8gelu_mulIdEvPKT_S2_PS0_l ; -- Begin function _Z8gelu_mulIdEvPKT_S2_PS0_l
	.globl	_Z8gelu_mulIdEvPKT_S2_PS0_l
	.p2align	8
	.type	_Z8gelu_mulIdEvPKT_S2_PS0_l,@function
_Z8gelu_mulIdEvPKT_S2_PS0_l:            ; @_Z8gelu_mulIdEvPKT_S2_PS0_l
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b256 s[4:11], s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s0, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s0, s2, v[0:1]
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_i64_e64 s[10:11], v[2:3]
	s_cbranch_execz .LBB11_2
; %bb.1:
	v_lshlrev_b64 v[0:1], 3, v[2:3]
	s_mov_b32 s0, 0x6d4801f7
	s_mov_b32 s1, 0x3fa6e4e2
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s0, 0x33d43651
	s_mov_b32 s1, 0x3fe98845
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[2:3], v[4:5]
	v_fma_f64 v[4:5], v[2:3], v[4:5], v[2:3]
	v_mul_f64 v[2:3], v[2:3], 0.5
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[4:5], v[4:5], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[6:7], |v[4:5]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[0:1], |v[4:5]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	v_add_f64 v[12:13], v[8:9], 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], 0
	v_add_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[12:13], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], v[10:11]
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[14:15], v[16:17], -v[10:11]
	v_mul_f64 v[16:17], v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_fma_f64 v[14:15], v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	v_add_f64 v[18:19], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	v_fma_f64 v[14:15], v[10:11], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[18:19], v[12:13]
	v_fma_f64 v[16:17], v[18:19], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[16:17]
	v_add_f64 v[14:15], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], v[14:15]
	v_add_f64 v[18:19], v[14:15], -v[20:21]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_cvt_i32_f64_e32 v18, v[6:7]
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[14:15], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], -1.0
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], v[8:9]
	v_ldexp_f64 v[10:11], v[6:7], v18
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[10:11]
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v18
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[12:13], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	v_fma_f64 v[14:15], v[12:13], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[6:7], v[14:15]
	v_add_f64 v[16:17], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], -v[16:17], 1.0
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	v_add_f64 v[20:21], -v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[14:15], v[20:21], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[16:17], v[12:13], v[14:15]
	v_add_f64 v[18:19], v[18:19], -v[14:15]
	v_mul_f64 v[20:21], v[10:11], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[18:19]
	v_fma_f64 v[22:23], v[16:17], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[16:17], v[6:7], v[22:23]
	v_add_f64 v[24:25], v[20:21], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[14:15], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[12:13], v[16:17]
	v_add_f64 v[8:9], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], -v[12:13]
	v_add_f64 v[8:9], v[26:27], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[10:11], v[12:13]
	v_add_f64 v[14:15], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[10:11], -v[12:13]
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[12:13], -v[18:19]
	v_add_f64 v[18:19], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_add_f64 v[14:15], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_rcp_f64_e32 v[22:23], v[18:19]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[18:19], v[22:23], 1.0
	v_fma_f64 v[10:11], v[12:13], v[22:23], v[22:23]
	v_add_f64 v[12:13], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[18:19], v[10:11], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_mul_f64 v[22:23], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[10:11], v[18:19], -v[22:23]
	v_fma_f64 v[14:15], v[10:11], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[22:23], v[14:15]
	v_add_f64 v[18:19], v[12:13], -v[16:17]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[12:13], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_co_u32 v20, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v21, null, s7, v1, vcc_lo
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[4:5]|
	global_load_b64 v[20:21], v[20:21], off
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[12:13], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[6:7], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_and_b32_e32 v8, 0x7fffffff, v5
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0x3ff00000, v7, vcc_lo
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[4:5]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v4, v6, v4, vcc_lo
	v_cndmask_b32_e32 v6, v7, v8, vcc_lo
	v_add_co_u32 v0, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	v_bfi_b32 v5, 0x7fffffff, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], 1.0
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], v[20:21], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB11_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8gelu_mulIdEvPKT_S2_PS0_l
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
		.amdhsa_next_free_vgpr 28
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
		.amdhsa_inst_pref_size 15
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z8gelu_mulIdEvPKT_S2_PS0_l,"axG",@progbits,_Z8gelu_mulIdEvPKT_S2_PS0_l,comdat
.Lfunc_end11:
	.size	_Z8gelu_mulIdEvPKT_S2_PS0_l, .Lfunc_end11-_Z8gelu_mulIdEvPKT_S2_PS0_l
                                        ; -- End function
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.num_vgpr, 28
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.num_agpr, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.numbered_sgpr, 12
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.num_named_barrier, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.private_seg_size, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.uses_vcc, 1
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.uses_flat_scratch, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.has_dyn_sized_stack, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.has_recursion, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1892
; TotalNumSgprs: 14
; NumVgprs: 28
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 28
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z8glu_geluIfEvPKT_PS0_ii,"axG",@progbits,_Z8glu_geluIfEvPKT_PS0_ii,comdat
	.protected	_Z8glu_geluIfEvPKT_PS0_ii ; -- Begin function _Z8glu_geluIfEvPKT_PS0_ii
	.globl	_Z8glu_geluIfEvPKT_PS0_ii
	.p2align	8
	.type	_Z8glu_geluIfEvPKT_PS0_ii,@function
_Z8glu_geluIfEvPKT_PS0_ii:              ; @_Z8glu_geluIfEvPKT_PS0_ii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v4
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mul_hi_i32 s3, s5, s4
	s_mul_i32 s2, s5, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[2:3]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB12_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB12_3
; %bb.2:
	s_ashr_i32 s6, s5, 31
	v_ashrrev_i32_e32 v8, 31, v3
	s_add_u32 s8, s4, s6
	s_mov_b32 s7, s6
	s_addc_u32 s9, s5, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[8:9], s[8:9], s[6:7]
	v_add_co_u32 v4, vcc_lo, v2, v8
	v_cvt_f32_u32_e32 v0, s8
	v_cvt_f32_u32_e32 v1, s9
	s_sub_u32 s10, 0, s8
	s_subb_u32 s11, 0, s9
	v_add_co_ci_u32_e64 v5, null, v3, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v4, v8
	v_xor_b32_e32 v10, v5, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x5f7ffffc, v0
	v_mul_f32_e32 v1, 0x2f800000, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v1, v1
	v_fmamk_f32 v0, v1, 0xcf800000, v0
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s2, v1
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v0
	s_mul_i32 s12, s10, s2
	s_mul_hi_u32 s14, s10, s7
	s_mul_i32 s13, s11, s7
	s_add_i32 s12, s14, s12
	s_mul_i32 s15, s10, s7
	s_add_i32 s12, s12, s13
	s_mul_hi_u32 s14, s7, s15
	s_mul_i32 s17, s7, s12
	s_mul_hi_u32 s16, s2, s15
	s_mul_i32 s13, s2, s15
	s_mul_hi_u32 s15, s7, s12
	s_add_u32 s14, s14, s17
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s18, s2, s12
	s_add_u32 s13, s14, s13
	s_mul_i32 s12, s2, s12
	s_addc_u32 s13, s15, s16
	s_addc_u32 s14, s18, 0
	s_add_u32 s12, s13, s12
	s_addc_u32 s13, 0, s14
	s_add_u32 s7, s7, s12
	s_cselect_b32 s12, -1, 0
	s_mul_hi_u32 s14, s10, s7
	s_cmp_lg_u32 s12, 0
	s_mul_i32 s12, s10, s7
	s_addc_u32 s2, s2, s13
	s_mul_i32 s11, s11, s7
	s_mul_i32 s10, s10, s2
	s_mul_hi_u32 s13, s7, s12
	s_add_i32 s10, s14, s10
	s_mul_hi_u32 s14, s2, s12
	s_add_i32 s10, s10, s11
	s_mul_i32 s11, s2, s12
	s_mul_i32 s16, s7, s10
	s_mul_hi_u32 s15, s7, s10
	s_add_u32 s13, s13, s16
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s12, s2, s10
	s_add_u32 s11, s13, s11
	s_mul_i32 s10, s2, s10
	s_addc_u32 s11, s15, s14
	s_addc_u32 s12, s12, 0
	s_add_u32 s10, s11, s10
	s_addc_u32 s11, 0, s12
	s_add_u32 s7, s7, s10
	s_cselect_b32 s10, -1, 0
	v_mul_hi_u32 v11, v9, s7
	s_cmp_lg_u32 s10, 0
	v_mad_u64_u32 v[4:5], null, v10, s7, 0
	s_addc_u32 s2, s2, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[6:7], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v5, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, 0, v1, vcc_lo
	v_mul_lo_u32 v6, s9, v4
	v_mad_u64_u32 v[0:1], null, s8, v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v7, s8, v5
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v7, v6
	v_add_co_u32 v7, s2, v4, 2
	v_add_co_ci_u32_e64 v9, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v6, v10, v1
	v_sub_co_u32 v11, s2, v0, s8
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s8, v11
	v_subrev_co_ci_u32_e64 v6, null, 0, v6, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s8, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e32 v6, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v4, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v11, null, 0, v5, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v6
	v_xor_b32_e32 v6, s6, v8
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v7 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v4, v0, s2
	v_cndmask_b32_e64 v1, v5, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v6
	v_xor_b32_e32 v1, v1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v6
	v_sub_co_ci_u32_e64 v1, null, v1, v6, vcc_lo
.LBB12_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB12_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s3, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s3, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s4
	v_add_nc_u32_e32 v4, 1, v0
	v_sub_nc_u32_e32 v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s4, v1
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
.LBB12_5:
	s_or_b32 exec_lo, exec_lo, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mul_lo_u32 v8, v1, s4
	v_mul_lo_u32 v9, v0, s5
	v_mad_u64_u32 v[6:7], null, v0, s4, 0
	v_ashrrev_i64 v[4:5], 31, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add3_u32 v7, v7, v9, v8
	v_mul_lo_u32 v5, v5, s4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v10, v4, s5
	v_mad_u64_u32 v[0:1], null, v4, s4, 0
	v_sub_co_u32 v4, vcc_lo, v2, v6
	v_add3_u32 v1, v1, v10, v5
	v_sub_co_ci_u32_e64 v5, null, v3, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v7, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 2
	v_add_co_u32 v0, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v7, v5, vcc_lo
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	global_load_b32 v8, v[0:1], off
	v_add_co_u32 v0, vcc_lo, v6, s0
	v_add_co_ci_u32_e64 v1, null, s1, v7, vcc_lo
	s_mov_b32 s0, 0x6d4801f7
	v_add_co_u32 v0, vcc_lo, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v1, v5, vcc_lo
	s_mov_b32 s1, 0x3fa6e4e2
	global_load_b32 v28, v[0:1], off
	s_waitcnt vmcnt(1)
	v_cvt_f64_f32_e32 v[0:1], v8
	v_mul_f64 v[4:5], v[0:1], s[0:1]
	s_mov_b32 s0, 0x33d43651
	s_mov_b32 s1, 0x3fe98845
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], v[0:1]
	v_fma_f64 v[4:5], v[4:5], v[0:1], v[0:1]
	v_mul_f64 v[0:1], v[0:1], 0.5
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[4:5], v[4:5], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[6:7], |v[4:5]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[4:5]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[0:1], |v[4:5]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	v_add_f64 v[12:13], v[8:9], 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], 0
	v_add_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[12:13], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], v[10:11]
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], s[4:5], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[14:15], v[16:17], -v[10:11]
	v_mul_f64 v[16:17], v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_fma_f64 v[14:15], v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	v_add_f64 v[18:19], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	v_fma_f64 v[14:15], v[10:11], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[18:19], v[12:13]
	v_fma_f64 v[16:17], v[18:19], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[16:17]
	v_add_f64 v[14:15], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], v[14:15]
	v_add_f64 v[18:19], v[14:15], -v[20:21]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_cvt_i32_f64_e32 v18, v[6:7]
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[14:15], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], -1.0
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], v[8:9]
	v_ldexp_f64 v[10:11], v[6:7], v18
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[10:11]
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v18
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[12:13], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	v_fma_f64 v[14:15], v[12:13], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[6:7], v[14:15]
	v_add_f64 v[16:17], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], -v[16:17], 1.0
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	v_add_f64 v[20:21], -v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[14:15], v[20:21], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[16:17], v[12:13], v[14:15]
	v_add_f64 v[18:19], v[18:19], -v[14:15]
	v_mul_f64 v[20:21], v[10:11], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[18:19]
	v_fma_f64 v[22:23], v[16:17], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[16:17], v[6:7], v[22:23]
	v_add_f64 v[24:25], v[20:21], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[14:15], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[12:13], v[16:17]
	v_add_f64 v[8:9], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], -v[12:13]
	v_add_f64 v[8:9], v[26:27], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[10:11], v[12:13]
	v_add_f64 v[14:15], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[10:11], -v[12:13]
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[12:13], -v[18:19]
	v_add_f64 v[18:19], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_add_f64 v[14:15], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_rcp_f64_e32 v[22:23], v[18:19]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[18:19], v[22:23], 1.0
	v_fma_f64 v[10:11], v[12:13], v[22:23], v[22:23]
	v_add_f64 v[12:13], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[18:19], v[10:11], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_mul_f64 v[22:23], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[10:11], v[18:19], -v[22:23]
	v_fma_f64 v[14:15], v[10:11], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[22:23], v[14:15]
	v_add_f64 v[18:19], v[12:13], -v[16:17]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[12:13], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], -v[14:15]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_and_b32_e32 v8, 0x7fffffff, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_cndmask_b32_e32 v7, 0x3ff00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[4:5]|
	v_cndmask_b32_e32 v4, v6, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, v7, v8, vcc_lo
	v_bfi_b32 v5, 0x7fffffff, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], 1.0
	v_mul_f64 v[0:1], v[0:1], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_f32_f64_e32 v4, v[0:1]
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v2, v28, v4
	global_store_b32 v[0:1], v2, off
.LBB12_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8glu_geluIfEvPKT_PS0_ii
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
		.amdhsa_next_free_vgpr 29
		.amdhsa_next_free_sgpr 19
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
		.amdhsa_inst_pref_size 24
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z8glu_geluIfEvPKT_PS0_ii,"axG",@progbits,_Z8glu_geluIfEvPKT_PS0_ii,comdat
.Lfunc_end12:
	.size	_Z8glu_geluIfEvPKT_PS0_ii, .Lfunc_end12-_Z8glu_geluIfEvPKT_PS0_ii
                                        ; -- End function
	.set _Z8glu_geluIfEvPKT_PS0_ii.num_vgpr, 29
	.set _Z8glu_geluIfEvPKT_PS0_ii.num_agpr, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.numbered_sgpr, 19
	.set _Z8glu_geluIfEvPKT_PS0_ii.num_named_barrier, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.private_seg_size, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.uses_vcc, 1
	.set _Z8glu_geluIfEvPKT_PS0_ii.uses_flat_scratch, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.has_dyn_sized_stack, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.has_recursion, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2984
; TotalNumSgprs: 21
; NumVgprs: 29
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 21
; NumVGPRsForWavesPerEU: 29
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z8glu_geluIdEvPKT_PS0_ii,"axG",@progbits,_Z8glu_geluIdEvPKT_PS0_ii,comdat
	.protected	_Z8glu_geluIdEvPKT_PS0_ii ; -- Begin function _Z8glu_geluIdEvPKT_PS0_ii
	.globl	_Z8glu_geluIdEvPKT_PS0_ii
	.p2align	8
	.type	_Z8glu_geluIdEvPKT_PS0_ii,@function
_Z8glu_geluIdEvPKT_PS0_ii:              ; @_Z8glu_geluIdEvPKT_PS0_ii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v4
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mul_hi_i32 s3, s5, s4
	s_mul_i32 s2, s5, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[2:3]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB13_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB13_3
; %bb.2:
	s_ashr_i32 s6, s5, 31
	v_ashrrev_i32_e32 v8, 31, v3
	s_add_u32 s8, s4, s6
	s_mov_b32 s7, s6
	s_addc_u32 s9, s5, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[8:9], s[8:9], s[6:7]
	v_add_co_u32 v4, vcc_lo, v2, v8
	v_cvt_f32_u32_e32 v0, s8
	v_cvt_f32_u32_e32 v1, s9
	s_sub_u32 s10, 0, s8
	s_subb_u32 s11, 0, s9
	v_add_co_ci_u32_e64 v5, null, v3, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v4, v8
	v_xor_b32_e32 v10, v5, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x5f7ffffc, v0
	v_mul_f32_e32 v1, 0x2f800000, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v1, v1
	v_fmamk_f32 v0, v1, 0xcf800000, v0
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s2, v1
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v0
	s_mul_i32 s12, s10, s2
	s_mul_hi_u32 s14, s10, s7
	s_mul_i32 s13, s11, s7
	s_add_i32 s12, s14, s12
	s_mul_i32 s15, s10, s7
	s_add_i32 s12, s12, s13
	s_mul_hi_u32 s14, s7, s15
	s_mul_i32 s17, s7, s12
	s_mul_hi_u32 s16, s2, s15
	s_mul_i32 s13, s2, s15
	s_mul_hi_u32 s15, s7, s12
	s_add_u32 s14, s14, s17
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s18, s2, s12
	s_add_u32 s13, s14, s13
	s_mul_i32 s12, s2, s12
	s_addc_u32 s13, s15, s16
	s_addc_u32 s14, s18, 0
	s_add_u32 s12, s13, s12
	s_addc_u32 s13, 0, s14
	s_add_u32 s7, s7, s12
	s_cselect_b32 s12, -1, 0
	s_mul_hi_u32 s14, s10, s7
	s_cmp_lg_u32 s12, 0
	s_mul_i32 s12, s10, s7
	s_addc_u32 s2, s2, s13
	s_mul_i32 s11, s11, s7
	s_mul_i32 s10, s10, s2
	s_mul_hi_u32 s13, s7, s12
	s_add_i32 s10, s14, s10
	s_mul_hi_u32 s14, s2, s12
	s_add_i32 s10, s10, s11
	s_mul_i32 s11, s2, s12
	s_mul_i32 s16, s7, s10
	s_mul_hi_u32 s15, s7, s10
	s_add_u32 s13, s13, s16
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s12, s2, s10
	s_add_u32 s11, s13, s11
	s_mul_i32 s10, s2, s10
	s_addc_u32 s11, s15, s14
	s_addc_u32 s12, s12, 0
	s_add_u32 s10, s11, s10
	s_addc_u32 s11, 0, s12
	s_add_u32 s7, s7, s10
	s_cselect_b32 s10, -1, 0
	v_mul_hi_u32 v11, v9, s7
	s_cmp_lg_u32 s10, 0
	v_mad_u64_u32 v[4:5], null, v10, s7, 0
	s_addc_u32 s2, s2, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[6:7], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v5, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, 0, v1, vcc_lo
	v_mul_lo_u32 v6, s9, v4
	v_mad_u64_u32 v[0:1], null, s8, v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v7, s8, v5
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v7, v6
	v_add_co_u32 v7, s2, v4, 2
	v_add_co_ci_u32_e64 v9, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v6, v10, v1
	v_sub_co_u32 v11, s2, v0, s8
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s8, v11
	v_subrev_co_ci_u32_e64 v6, null, 0, v6, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s8, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e32 v6, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v4, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v11, null, 0, v5, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v6
	v_xor_b32_e32 v6, s6, v8
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v7 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v4, v0, s2
	v_cndmask_b32_e64 v1, v5, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v6
	v_xor_b32_e32 v1, v1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v6
	v_sub_co_ci_u32_e64 v1, null, v1, v6, vcc_lo
.LBB13_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB13_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s3, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s3, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s4
	v_add_nc_u32_e32 v4, 1, v0
	v_sub_nc_u32_e32 v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s4, v1
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
.LBB13_5:
	s_or_b32 exec_lo, exec_lo, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mul_lo_u32 v8, v1, s4
	v_mul_lo_u32 v9, v0, s5
	v_mad_u64_u32 v[6:7], null, v0, s4, 0
	v_ashrrev_i64 v[4:5], 31, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add3_u32 v7, v7, v9, v8
	v_mul_lo_u32 v5, v5, s4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v10, v4, s5
	v_mad_u64_u32 v[0:1], null, v4, s4, 0
	v_sub_co_u32 v4, vcc_lo, v2, v6
	v_add3_u32 v1, v1, v10, v5
	v_sub_co_ci_u32_e64 v5, null, v3, v7, vcc_lo
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v7, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 3
	v_add_co_u32 v0, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v7, v5, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, s0
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
	global_load_b64 v[0:1], v[0:1], off
	v_add_co_u32 v4, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, v7, v5, vcc_lo
	s_mov_b32 s0, 0x6d4801f7
	s_mov_b32 s1, 0x3fa6e4e2
	s_mov_b32 s4, 0x6a5dcb37
	global_load_b64 v[4:5], v[4:5], off
	s_mov_b32 s5, 0x3e5ade15
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], v[0:1], s[0:1]
	s_mov_b32 s0, 0x33d43651
	s_mov_b32 s1, 0x3fe98845
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[0:1], v[6:7]
	v_fma_f64 v[6:7], v[0:1], v[6:7], v[0:1]
	v_mul_f64 v[0:1], v[0:1], 0.5
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[6:7], v[6:7], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[8:9], |v[6:7]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[6:7]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[8:9]
	v_fma_f64 v[10:11], v[8:9], s[0:1], |v[6:7]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[12:13], v[8:9], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	v_add_f64 v[14:15], v[10:11], 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], 0
	v_add_f64 v[12:13], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_mul_f64 v[12:13], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[14:15], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], v[12:13]
	v_add_f64 v[16:17], v[16:17], -v[14:15]
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_add_f64 v[12:13], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[12:13], s[4:5], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[16:17], v[18:19], -v[12:13]
	v_mul_f64 v[18:19], v[12:13], v[12:13]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_fma_f64 v[16:17], v[12:13], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	v_add_f64 v[20:21], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	v_fma_f64 v[16:17], v[12:13], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	v_add_f64 v[20:21], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[22:23], v[20:21], v[14:15]
	v_fma_f64 v[18:19], v[20:21], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[18:19]
	v_add_f64 v[16:17], v[22:23], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[12:13], v[16:17]
	v_add_f64 v[20:21], v[16:17], -v[22:23]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[20:21]
	v_cvt_i32_f64_e32 v20, v[8:9]
	v_add_f64 v[12:13], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], v[10:11]
	v_add_f64 v[14:15], v[12:13], 1.0
	v_add_f64 v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], -1.0
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[14:15], v[10:11]
	v_ldexp_f64 v[12:13], v[8:9], v20
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[16:17], v[12:13]
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v20
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[12:13], v[16:17], 1.0
	v_fma_f64 v[16:17], v[18:19], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], -v[12:13], v[16:17], 1.0
	v_fma_f64 v[14:15], v[18:19], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[14:15]
	v_fma_f64 v[16:17], v[14:15], v[12:13], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[8:9], v[16:17]
	v_add_f64 v[18:19], v[10:11], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], -v[18:19], 1.0
	v_add_f64 v[10:11], v[18:19], -v[10:11]
	v_add_f64 v[22:23], -v[20:21], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[16:17], v[22:23], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_add_f64 v[16:17], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[14:15], v[16:17]
	v_add_f64 v[20:21], v[20:21], -v[16:17]
	v_mul_f64 v[22:23], v[12:13], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[20:21]
	v_fma_f64 v[24:25], v[18:19], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[24:25], v[18:19], v[8:9], v[24:25]
	v_add_f64 v[26:27], v[22:23], v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[28:29], v[16:17], -v[26:27]
	v_add_f64 v[20:21], v[26:27], -v[22:23]
	v_add_f64 v[16:17], v[16:17], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[24:25]
	v_add_f64 v[16:17], v[16:17], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_add_f64 v[16:17], v[14:15], v[18:19]
	v_add_f64 v[10:11], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[16:17], -v[14:15]
	v_add_f64 v[10:11], v[28:29], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[18:19], v[10:11]
	v_add_f64 v[14:15], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[12:13], v[14:15]
	v_add_f64 v[16:17], v[14:15], -v[16:17]
	v_add_f64 v[22:23], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[14:15], -v[20:21]
	v_add_f64 v[20:21], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[16:17], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[20:21], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_rcp_f64_e32 v[24:25], v[20:21]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[20:21], v[24:25], 1.0
	v_fma_f64 v[12:13], v[14:15], v[24:25], v[24:25]
	v_add_f64 v[14:15], v[22:23], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[20:21], v[12:13], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_mul_f64 v[24:25], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[12:13], v[20:21], -v[24:25]
	v_fma_f64 v[16:17], v[12:13], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[24:25], v[16:17]
	v_add_f64 v[20:21], v[14:15], -v[18:19]
	v_add_f64 v[24:25], v[18:19], -v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[14:15], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[26:27], -v[18:19]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[18:19], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[20:21], v[8:9]
	v_mul_f64 v[8:9], v[10:11], v[8:9]
	v_and_b32_e32 v10, 0x7fffffff, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[8:9]
	v_cndmask_b32_e32 v9, 0x3ff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[6:7]|
	v_cndmask_b32_e32 v6, v8, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v8, v9, v10, vcc_lo
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_bfi_b32 v7, 0x7fffffff, v8, v7
	v_add_f64 v[6:7], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[0:1], v[0:1], v[6:7]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[0:1], v[4:5], v[0:1]
	global_store_b64 v[2:3], v[0:1], off
.LBB13_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8glu_geluIdEvPKT_PS0_ii
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
		.amdhsa_next_free_vgpr 30
		.amdhsa_next_free_sgpr 19
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
		.amdhsa_inst_pref_size 24
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z8glu_geluIdEvPKT_PS0_ii,"axG",@progbits,_Z8glu_geluIdEvPKT_PS0_ii,comdat
.Lfunc_end13:
	.size	_Z8glu_geluIdEvPKT_PS0_ii, .Lfunc_end13-_Z8glu_geluIdEvPKT_PS0_ii
                                        ; -- End function
	.set _Z8glu_geluIdEvPKT_PS0_ii.num_vgpr, 30
	.set _Z8glu_geluIdEvPKT_PS0_ii.num_agpr, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.numbered_sgpr, 19
	.set _Z8glu_geluIdEvPKT_PS0_ii.num_named_barrier, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.private_seg_size, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.uses_vcc, 1
	.set _Z8glu_geluIdEvPKT_PS0_ii.uses_flat_scratch, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.has_dyn_sized_stack, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.has_recursion, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2976
; TotalNumSgprs: 21
; NumVgprs: 30
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 21
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
	.section	.text._Z8glu_siluIfEvPKT_PS0_ii,"axG",@progbits,_Z8glu_siluIfEvPKT_PS0_ii,comdat
	.protected	_Z8glu_siluIfEvPKT_PS0_ii ; -- Begin function _Z8glu_siluIfEvPKT_PS0_ii
	.globl	_Z8glu_siluIfEvPKT_PS0_ii
	.p2align	8
	.type	_Z8glu_siluIfEvPKT_PS0_ii,@function
_Z8glu_siluIfEvPKT_PS0_ii:              ; @_Z8glu_siluIfEvPKT_PS0_ii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v4
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mul_hi_i32 s3, s5, s4
	s_mul_i32 s2, s5, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[2:3]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB14_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB14_3
; %bb.2:
	s_ashr_i32 s6, s5, 31
	v_ashrrev_i32_e32 v8, 31, v3
	s_add_u32 s8, s4, s6
	s_mov_b32 s7, s6
	s_addc_u32 s9, s5, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[8:9], s[8:9], s[6:7]
	v_add_co_u32 v4, vcc_lo, v2, v8
	v_cvt_f32_u32_e32 v0, s8
	v_cvt_f32_u32_e32 v1, s9
	s_sub_u32 s10, 0, s8
	s_subb_u32 s11, 0, s9
	v_add_co_ci_u32_e64 v5, null, v3, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v4, v8
	v_xor_b32_e32 v10, v5, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x5f7ffffc, v0
	v_mul_f32_e32 v1, 0x2f800000, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v1, v1
	v_fmamk_f32 v0, v1, 0xcf800000, v0
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s2, v1
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v0
	s_mul_i32 s12, s10, s2
	s_mul_hi_u32 s14, s10, s7
	s_mul_i32 s13, s11, s7
	s_add_i32 s12, s14, s12
	s_mul_i32 s15, s10, s7
	s_add_i32 s12, s12, s13
	s_mul_hi_u32 s14, s7, s15
	s_mul_i32 s17, s7, s12
	s_mul_hi_u32 s16, s2, s15
	s_mul_i32 s13, s2, s15
	s_mul_hi_u32 s15, s7, s12
	s_add_u32 s14, s14, s17
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s18, s2, s12
	s_add_u32 s13, s14, s13
	s_mul_i32 s12, s2, s12
	s_addc_u32 s13, s15, s16
	s_addc_u32 s14, s18, 0
	s_add_u32 s12, s13, s12
	s_addc_u32 s13, 0, s14
	s_add_u32 s7, s7, s12
	s_cselect_b32 s12, -1, 0
	s_mul_hi_u32 s14, s10, s7
	s_cmp_lg_u32 s12, 0
	s_mul_i32 s12, s10, s7
	s_addc_u32 s2, s2, s13
	s_mul_i32 s11, s11, s7
	s_mul_i32 s10, s10, s2
	s_mul_hi_u32 s13, s7, s12
	s_add_i32 s10, s14, s10
	s_mul_hi_u32 s14, s2, s12
	s_add_i32 s10, s10, s11
	s_mul_i32 s11, s2, s12
	s_mul_i32 s16, s7, s10
	s_mul_hi_u32 s15, s7, s10
	s_add_u32 s13, s13, s16
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s12, s2, s10
	s_add_u32 s11, s13, s11
	s_mul_i32 s10, s2, s10
	s_addc_u32 s11, s15, s14
	s_addc_u32 s12, s12, 0
	s_add_u32 s10, s11, s10
	s_addc_u32 s11, 0, s12
	s_add_u32 s7, s7, s10
	s_cselect_b32 s10, -1, 0
	v_mul_hi_u32 v11, v9, s7
	s_cmp_lg_u32 s10, 0
	v_mad_u64_u32 v[4:5], null, v10, s7, 0
	s_addc_u32 s2, s2, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[6:7], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v5, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, 0, v1, vcc_lo
	v_mul_lo_u32 v6, s9, v4
	v_mad_u64_u32 v[0:1], null, s8, v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v7, s8, v5
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v7, v6
	v_add_co_u32 v7, s2, v4, 2
	v_add_co_ci_u32_e64 v9, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v6, v10, v1
	v_sub_co_u32 v11, s2, v0, s8
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s8, v11
	v_subrev_co_ci_u32_e64 v6, null, 0, v6, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s8, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e32 v6, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v4, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v11, null, 0, v5, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v6
	v_xor_b32_e32 v6, s6, v8
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v7 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v4, v0, s2
	v_cndmask_b32_e64 v1, v5, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v6
	v_xor_b32_e32 v1, v1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v6
	v_sub_co_ci_u32_e64 v1, null, v1, v6, vcc_lo
.LBB14_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB14_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s3, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s3, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s4
	v_add_nc_u32_e32 v4, 1, v0
	v_sub_nc_u32_e32 v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s4, v1
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
.LBB14_5:
	s_or_b32 exec_lo, exec_lo, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mul_lo_u32 v8, v1, s4
	v_mul_lo_u32 v9, v0, s5
	v_mad_u64_u32 v[6:7], null, v0, s4, 0
	v_ashrrev_i64 v[4:5], 31, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add3_u32 v7, v7, v9, v8
	v_mul_lo_u32 v5, v5, s4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v10, v4, s5
	v_mad_u64_u32 v[0:1], null, v4, s4, 0
	v_sub_co_u32 v4, vcc_lo, v2, v6
	v_add3_u32 v1, v1, v10, v5
	v_sub_co_ci_u32_e64 v5, null, v3, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v7, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 2
	v_add_co_u32 v0, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v7, v5, vcc_lo
	global_load_b32 v8, v[0:1], off
	v_add_co_u32 v0, vcc_lo, v6, s0
	v_add_co_ci_u32_e64 v1, null, s1, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, v0, v4
	v_add_co_ci_u32_e64 v1, null, v1, v5, vcc_lo
	global_load_b32 v14, v[0:1], off
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v0, 0xbfb8aa3b, v8
	v_cmp_nlt_f32_e32 vcc_lo, 0x42ce8ed0, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v1, 0xbfb8aa3b, v8, -v0
	v_rndne_f32_e32 v4, v0
	v_fmamk_f32 v1, v8, 0xb2a5705f, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v0, v0, v4
	v_add_f32_e32 v0, v0, v1
	v_cvt_i32_f32_e32 v1, v4
	v_cvt_f64_f32_e32 v[4:5], v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v0, v0, v1
	v_cndmask_b32_e32 v0, 0, v0, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2b17218, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, 0x7f800000, v0, vcc_lo
	v_cvt_f64_f32_e32 v[0:1], v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[0:1], v[0:1], 1.0
	v_div_scale_f64 v[6:7], null, v[0:1], v[0:1], v[4:5]
	v_div_scale_f64 v[12:13], vcc_lo, v[4:5], v[0:1], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	v_div_fixup_f64 v[0:1], v[6:7], v[0:1], v[4:5]
	s_waitcnt vmcnt(0)
	v_cvt_f64_f32_e32 v[4:5], v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[0:1], v[0:1], v[4:5]
	v_cvt_f32_f64_e32 v4, v[0:1]
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b32 v[0:1], v4, off
.LBB14_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8glu_siluIfEvPKT_PS0_ii
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
		.amdhsa_next_free_vgpr 15
		.amdhsa_next_free_sgpr 19
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
		.amdhsa_inst_pref_size 12
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z8glu_siluIfEvPKT_PS0_ii,"axG",@progbits,_Z8glu_siluIfEvPKT_PS0_ii,comdat
.Lfunc_end14:
	.size	_Z8glu_siluIfEvPKT_PS0_ii, .Lfunc_end14-_Z8glu_siluIfEvPKT_PS0_ii
                                        ; -- End function
	.set _Z8glu_siluIfEvPKT_PS0_ii.num_vgpr, 15
	.set _Z8glu_siluIfEvPKT_PS0_ii.num_agpr, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.numbered_sgpr, 19
	.set _Z8glu_siluIfEvPKT_PS0_ii.num_named_barrier, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.private_seg_size, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.uses_vcc, 1
	.set _Z8glu_siluIfEvPKT_PS0_ii.uses_flat_scratch, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.has_dyn_sized_stack, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.has_recursion, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1496
; TotalNumSgprs: 21
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 21
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
	.section	.text._Z8glu_siluIdEvPKT_PS0_ii,"axG",@progbits,_Z8glu_siluIdEvPKT_PS0_ii,comdat
	.protected	_Z8glu_siluIdEvPKT_PS0_ii ; -- Begin function _Z8glu_siluIdEvPKT_PS0_ii
	.globl	_Z8glu_siluIdEvPKT_PS0_ii
	.p2align	8
	.type	_Z8glu_siluIdEvPKT_PS0_ii,@function
_Z8glu_siluIdEvPKT_PS0_ii:              ; @_Z8glu_siluIdEvPKT_PS0_ii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v4
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mul_hi_i32 s3, s5, s4
	s_mul_i32 s2, s5, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[2:3]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB15_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB15_3
; %bb.2:
	s_ashr_i32 s6, s5, 31
	v_ashrrev_i32_e32 v8, 31, v3
	s_add_u32 s8, s4, s6
	s_mov_b32 s7, s6
	s_addc_u32 s9, s5, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[8:9], s[8:9], s[6:7]
	v_add_co_u32 v4, vcc_lo, v2, v8
	v_cvt_f32_u32_e32 v0, s8
	v_cvt_f32_u32_e32 v1, s9
	s_sub_u32 s10, 0, s8
	s_subb_u32 s11, 0, s9
	v_add_co_ci_u32_e64 v5, null, v3, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v4, v8
	v_xor_b32_e32 v10, v5, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x5f7ffffc, v0
	v_mul_f32_e32 v1, 0x2f800000, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v1, v1
	v_fmamk_f32 v0, v1, 0xcf800000, v0
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s2, v1
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v0
	s_mul_i32 s12, s10, s2
	s_mul_hi_u32 s14, s10, s7
	s_mul_i32 s13, s11, s7
	s_add_i32 s12, s14, s12
	s_mul_i32 s15, s10, s7
	s_add_i32 s12, s12, s13
	s_mul_hi_u32 s14, s7, s15
	s_mul_i32 s17, s7, s12
	s_mul_hi_u32 s16, s2, s15
	s_mul_i32 s13, s2, s15
	s_mul_hi_u32 s15, s7, s12
	s_add_u32 s14, s14, s17
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s18, s2, s12
	s_add_u32 s13, s14, s13
	s_mul_i32 s12, s2, s12
	s_addc_u32 s13, s15, s16
	s_addc_u32 s14, s18, 0
	s_add_u32 s12, s13, s12
	s_addc_u32 s13, 0, s14
	s_add_u32 s7, s7, s12
	s_cselect_b32 s12, -1, 0
	s_mul_hi_u32 s14, s10, s7
	s_cmp_lg_u32 s12, 0
	s_mul_i32 s12, s10, s7
	s_addc_u32 s2, s2, s13
	s_mul_i32 s11, s11, s7
	s_mul_i32 s10, s10, s2
	s_mul_hi_u32 s13, s7, s12
	s_add_i32 s10, s14, s10
	s_mul_hi_u32 s14, s2, s12
	s_add_i32 s10, s10, s11
	s_mul_i32 s11, s2, s12
	s_mul_i32 s16, s7, s10
	s_mul_hi_u32 s15, s7, s10
	s_add_u32 s13, s13, s16
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s12, s2, s10
	s_add_u32 s11, s13, s11
	s_mul_i32 s10, s2, s10
	s_addc_u32 s11, s15, s14
	s_addc_u32 s12, s12, 0
	s_add_u32 s10, s11, s10
	s_addc_u32 s11, 0, s12
	s_add_u32 s7, s7, s10
	s_cselect_b32 s10, -1, 0
	v_mul_hi_u32 v11, v9, s7
	s_cmp_lg_u32 s10, 0
	v_mad_u64_u32 v[4:5], null, v10, s7, 0
	s_addc_u32 s2, s2, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[6:7], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v5, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, 0, v1, vcc_lo
	v_mul_lo_u32 v6, s9, v4
	v_mad_u64_u32 v[0:1], null, s8, v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v7, s8, v5
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v7, v6
	v_add_co_u32 v7, s2, v4, 2
	v_add_co_ci_u32_e64 v9, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v6, v10, v1
	v_sub_co_u32 v11, s2, v0, s8
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s8, v11
	v_subrev_co_ci_u32_e64 v6, null, 0, v6, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s8, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e32 v6, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v4, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v11, null, 0, v5, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v6
	v_xor_b32_e32 v6, s6, v8
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v7 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v4, v0, s2
	v_cndmask_b32_e64 v1, v5, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v6
	v_xor_b32_e32 v1, v1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v6
	v_sub_co_ci_u32_e64 v1, null, v1, v6, vcc_lo
.LBB15_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB15_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s3, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s3, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s4
	v_add_nc_u32_e32 v4, 1, v0
	v_sub_nc_u32_e32 v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s4, v1
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
.LBB15_5:
	s_or_b32 exec_lo, exec_lo, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mul_lo_u32 v8, v1, s4
	v_mul_lo_u32 v9, v0, s5
	v_mad_u64_u32 v[6:7], null, v0, s4, 0
	v_ashrrev_i64 v[4:5], 31, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add3_u32 v7, v7, v9, v8
	v_mul_lo_u32 v5, v5, s4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v10, v4, s5
	v_mad_u64_u32 v[0:1], null, v4, s4, 0
	v_sub_co_u32 v4, vcc_lo, v2, v6
	v_add3_u32 v1, v1, v10, v5
	v_sub_co_ci_u32_e64 v5, null, v3, v7, vcc_lo
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v7, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 3
	v_add_co_u32 v0, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v7, v5, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, s0
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
	global_load_b64 v[0:1], v[0:1], off
	v_add_co_u32 v4, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, v7, v5, vcc_lo
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	s_mov_b32 s4, 0x6a5dcb37
	global_load_b64 v[4:5], v[4:5], off
	s_mov_b32 s5, 0x3e5ade15
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], v[0:1], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[0:1], -v[0:1]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	v_cvt_i32_f64_e32 v12, v[6:7]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], s[0:1], v[8:9]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], s[4:5], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], 1.0
	v_fma_f64 v[6:7], v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v12
	v_add_f64 v[6:7], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0x3ff00000, v7, s0
	v_div_scale_f64 v[8:9], null, v[6:7], v[6:7], v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_div_scale_f64 v[12:13], vcc_lo, v[0:1], v[6:7], v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[12:13], v[10:11]
	v_fma_f64 v[8:9], -v[8:9], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[14:15]
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[0:1], v[8:9], v[6:7], v[0:1]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[0:1], v[4:5], v[0:1]
	global_store_b64 v[2:3], v[0:1], off
.LBB15_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8glu_siluIdEvPKT_PS0_ii
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
		.amdhsa_next_free_vgpr 16
		.amdhsa_next_free_sgpr 19
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
		.amdhsa_inst_pref_size 15
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z8glu_siluIdEvPKT_PS0_ii,"axG",@progbits,_Z8glu_siluIdEvPKT_PS0_ii,comdat
.Lfunc_end15:
	.size	_Z8glu_siluIdEvPKT_PS0_ii, .Lfunc_end15-_Z8glu_siluIdEvPKT_PS0_ii
                                        ; -- End function
	.set _Z8glu_siluIdEvPKT_PS0_ii.num_vgpr, 16
	.set _Z8glu_siluIdEvPKT_PS0_ii.num_agpr, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.numbered_sgpr, 19
	.set _Z8glu_siluIdEvPKT_PS0_ii.num_named_barrier, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.private_seg_size, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.uses_vcc, 1
	.set _Z8glu_siluIdEvPKT_PS0_ii.uses_flat_scratch, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.has_dyn_sized_stack, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.has_recursion, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1808
; TotalNumSgprs: 21
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 21
; NumVGPRsForWavesPerEU: 16
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
	.type	__hip_cuid_8223f6af993cc274,@object ; @__hip_cuid_8223f6af993cc274
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_8223f6af993cc274
__hip_cuid_8223f6af993cc274:
	.byte	0                               ; 0x0
	.size	__hip_cuid_8223f6af993cc274, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_8223f6af993cc274
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
        .size:           8
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
    .name:           widen_bf16_f64
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         widen_bf16_f64.kd
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
        .size:           8
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
    .name:           widen_bf16_f32
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         widen_bf16_f32.kd
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
        .size:           8
        .value_kind:     by_value
      - .offset:         24
        .size:           8
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
    .name:           widen_bf16_f64_scaled
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         widen_bf16_f64_scaled.kd
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
        .size:           8
        .value_kind:     by_value
      - .offset:         24
        .size:           8
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
    .name:           widen_bf16_f32_scaled
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         widen_bf16_f32_scaled.kd
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
        .value_kind:     by_value
      - .offset:         44
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           4
        .value_kind:     by_value
      - .offset:         56
        .size:           8
        .value_kind:     by_value
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         68
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         72
        .size:           8
        .value_kind:     global_buffer
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
        .value_kind:     by_value
      - .offset:         100
        .size:           4
        .value_kind:     by_value
      - .offset:         104
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         108
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         112
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         116
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         118
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         120
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         122
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         124
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         126
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         144
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         152
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         160
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         168
        .size:           2
        .value_kind:     hidden_grid_dims
      - .offset:         224
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 272
    .kernarg_segment_align: 8
    .kernarg_segment_size: 360
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     58
    .sgpr_spill_count: 0
    .symbol:         _Z9flash_gqaIfEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     25
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
      - .offset:         44
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           4
        .value_kind:     by_value
      - .offset:         56
        .size:           8
        .value_kind:     by_value
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         68
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         72
        .size:           8
        .value_kind:     global_buffer
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
        .value_kind:     by_value
      - .offset:         100
        .size:           4
        .value_kind:     by_value
      - .offset:         104
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         108
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         112
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         116
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         118
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         120
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         122
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         124
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         126
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         144
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         152
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         160
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         168
        .size:           2
        .value_kind:     hidden_grid_dims
      - .offset:         224
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 544
    .kernarg_segment_align: 8
    .kernarg_segment_size: 360
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     74
    .sgpr_spill_count: 0
    .symbol:         _Z9flash_gqaIdEvPKT_S2_S2_PS0_iiiiidiiS3_S3_S3_ii.kd
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
      - .offset:         32
        .size:           4
        .value_kind:     by_value
      - .offset:         36
        .size:           4
        .value_kind:     by_value
      - .offset:         40
        .size:           4
        .value_kind:     by_value
      - .offset:         44
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           4
        .value_kind:     by_value
      - .offset:         52
        .size:           4
        .value_kind:     by_value
      - .offset:         56
        .size:           4
        .value_kind:     by_value
      - .offset:         60
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         64
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         72
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         80
        .size:           8
        .value_kind:     global_buffer
      - .offset:         88
        .size:           4
        .value_kind:     by_value
      - .offset:         92
        .size:           4
        .value_kind:     by_value
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
      - .offset:         216
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 272
    .kernarg_segment_align: 8
    .kernarg_segment_size: 352
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     58
    .sgpr_spill_count: 0
    .symbol:         _Z9flash_mlaIfEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.kd
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
      - .offset:         44
        .size:           4
        .value_kind:     by_value
      - .offset:         48
        .size:           4
        .value_kind:     by_value
      - .offset:         52
        .size:           4
        .value_kind:     by_value
      - .offset:         56
        .size:           4
        .value_kind:     by_value
      - .offset:         60
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         64
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         72
        .size:           8
        .value_kind:     global_buffer
      - .address_space:  global
        .offset:         80
        .size:           8
        .value_kind:     global_buffer
      - .offset:         88
        .size:           4
        .value_kind:     by_value
      - .offset:         92
        .size:           4
        .value_kind:     by_value
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
      - .offset:         216
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 544
    .kernarg_segment_align: 8
    .kernarg_segment_size: 352
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     74
    .sgpr_spill_count: 0
    .symbol:         _Z9flash_mlaIdEvPKT_S2_S2_PS0_iiiiiiiiS3_S3_S3_ii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     34
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
        .value_kind:     by_value
      - .offset:         20
        .size:           4
        .value_kind:     by_value
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
    .name:           _Z12rope_partialIfEvPT_iiiiPKS0_S3_i
    .private_segment_fixed_size: 0
    .sgpr_count:     28
    .sgpr_spill_count: 0
    .symbol:         _Z12rope_partialIfEvPT_iiiiPKS0_S3_i.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     21
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
        .value_kind:     by_value
      - .offset:         20
        .size:           4
        .value_kind:     by_value
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
    .name:           _Z12rope_partialIdEvPT_iiiiPKS0_S3_i
    .private_segment_fixed_size: 0
    .sgpr_count:     28
    .sgpr_spill_count: 0
    .symbol:         _Z12rope_partialIdEvPT_iiiiPKS0_S3_i.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     52
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
        .size:           8
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
    .name:           _Z8gelu_mulIfEvPKT_S2_PS0_l
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z8gelu_mulIfEvPKT_S2_PS0_l.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     28
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
        .size:           8
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
    .name:           _Z8gelu_mulIdEvPKT_S2_PS0_l
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z8gelu_mulIdEvPKT_S2_PS0_l.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     28
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
    .name:           _Z8glu_geluIfEvPKT_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         _Z8glu_geluIfEvPKT_PS0_ii.kd
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
    .name:           _Z8glu_geluIdEvPKT_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         _Z8glu_geluIdEvPKT_PS0_ii.kd
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
    .name:           _Z8glu_siluIfEvPKT_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         _Z8glu_siluIfEvPKT_PS0_ii.kd
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
    .name:           _Z8glu_siluIdEvPKT_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         _Z8glu_siluIdEvPKT_PS0_ii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     16
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
