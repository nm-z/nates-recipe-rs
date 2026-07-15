	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	relu_f32_kernel         ; -- Begin function relu_f32_kernel
	.globl	relu_f32_kernel
	.p2align	8
	.type	relu_f32_kernel,@function
relu_f32_kernel:                        ; @relu_f32_kernel
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
	s_cbranch_execz .LBB0_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_max_f32_e32 v2, v2, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_max_f32_e32 v2, 0, v2
	global_store_b32 v[0:1], v2, off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel relu_f32_kernel
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
.Lfunc_end0:
	.size	relu_f32_kernel, .Lfunc_end0-relu_f32_kernel
                                        ; -- End function
	.set relu_f32_kernel.num_vgpr, 4
	.set relu_f32_kernel.num_agpr, 0
	.set relu_f32_kernel.numbered_sgpr, 5
	.set relu_f32_kernel.num_named_barrier, 0
	.set relu_f32_kernel.private_seg_size, 0
	.set relu_f32_kernel.uses_vcc, 1
	.set relu_f32_kernel.uses_flat_scratch, 0
	.set relu_f32_kernel.has_dyn_sized_stack, 0
	.set relu_f32_kernel.has_recursion, 0
	.set relu_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 160
; TotalNumSgprs: 7
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	relu_backward_f32_kernel ; -- Begin function relu_backward_f32_kernel
	.globl	relu_backward_f32_kernel
	.p2align	8
	.type	relu_backward_f32_kernel,@function
relu_backward_f32_kernel:               ; @relu_backward_f32_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_4
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	global_load_b32 v3, v[2:3], off
	v_mov_b32_e32 v2, 0
	s_waitcnt vmcnt(0)
	v_cmpx_lt_f32_e32 0, v3
	s_cbranch_execz .LBB1_3
; %bb.2:
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
.LBB1_3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b32 v[0:1], v2, off
.LBB1_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel relu_backward_f32_kernel
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
	.size	relu_backward_f32_kernel, .Lfunc_end1-relu_backward_f32_kernel
                                        ; -- End function
	.set relu_backward_f32_kernel.num_vgpr, 4
	.set relu_backward_f32_kernel.num_agpr, 0
	.set relu_backward_f32_kernel.numbered_sgpr, 8
	.set relu_backward_f32_kernel.num_named_barrier, 0
	.set relu_backward_f32_kernel.private_seg_size, 0
	.set relu_backward_f32_kernel.uses_vcc, 1
	.set relu_backward_f32_kernel.uses_flat_scratch, 0
	.set relu_backward_f32_kernel.has_dyn_sized_stack, 0
	.set relu_backward_f32_kernel.has_recursion, 0
	.set relu_backward_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 212
; TotalNumSgprs: 10
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
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
	.protected	gelu_f32_kernel         ; -- Begin function gelu_f32_kernel
	.globl	gelu_f32_kernel
	.p2align	8
	.type	gelu_f32_kernel,@function
gelu_f32_kernel:                        ; @gelu_f32_kernel
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
	s_cbranch_execz .LBB2_6
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v3, 0x3d372713, v2
	v_mul_f32_e32 v3, v2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v3, v2, v3, v2
	v_mul_f32_e32 v3, 0x3f4c422a, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_ngt_f32_e64 s0, 0x3f200000, |v3|
	s_and_saveexec_b32 s1, s0
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB2_3
; %bb.2:
	v_add_f32_e64 v4, |v3|, |v3|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v5, 0x3fb8aa3b, v4
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v4
	v_rndne_f32_e32 v6, v5
	v_fma_f32 v7, 0x3fb8aa3b, v4, -v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v5, v5, v6
	v_fmamk_f32 v7, v4, 0x32a5705f, v7
	v_cvt_i32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v5, v5, v7
	v_exp_f32_e32 v5, v5
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v5, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v4
	v_cndmask_b32_e32 v4, 0x7f800000, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v4, 1.0, v4
	v_rcp_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, v4, -2.0, 1.0
.LBB2_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB2_5
; %bb.4:
	v_mul_f32_e32 v4, v3, v3
	s_mov_b32 s1, 0xbbbac73d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fmaak_f32 v5, s1, v4, 0x3ca908c9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v5, v4, v5, 0xbd5c1c4e
	v_fmaak_f32 v5, v4, v5, 0x3e088382
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v5, v4, v5, 0xbeaaaa99
	v_mul_f32_e64 v5, |v3|, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f32 v4, v4, v5, |v3|
.LBB2_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_bfi_b32 v3, 0x7fffffff, v4, v3
	v_mul_f32_e32 v2, 0.5, v2
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, 1.0, v3
	v_mul_f32_e32 v2, v2, v3
	global_store_b32 v[0:1], v2, off
.LBB2_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gelu_f32_kernel
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
	.size	gelu_f32_kernel, .Lfunc_end2-gelu_f32_kernel
                                        ; -- End function
	.set gelu_f32_kernel.num_vgpr, 8
	.set gelu_f32_kernel.num_agpr, 0
	.set gelu_f32_kernel.numbered_sgpr, 5
	.set gelu_f32_kernel.num_named_barrier, 0
	.set gelu_f32_kernel.private_seg_size, 0
	.set gelu_f32_kernel.uses_vcc, 1
	.set gelu_f32_kernel.uses_flat_scratch, 0
	.set gelu_f32_kernel.has_dyn_sized_stack, 0
	.set gelu_f32_kernel.has_recursion, 0
	.set gelu_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 464
; TotalNumSgprs: 7
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	gelu_backward_f32_kernel ; -- Begin function gelu_backward_f32_kernel
	.globl	gelu_backward_f32_kernel
	.p2align	8
	.type	gelu_backward_f32_kernel,@function
gelu_backward_f32_kernel:               ; @gelu_backward_f32_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB3_6
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
                                        ; implicit-def: $vgpr4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v3, 0x3d372713, v2
	v_mul_f32_e32 v3, v2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v3, v2, v3, v2
	v_mul_f32_e32 v3, 0x3f4c422a, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_ngt_f32_e64 s2, 0x3f200000, |v3|
	s_and_saveexec_b32 s3, s2
	s_xor_b32 s2, exec_lo, s3
	s_cbranch_execz .LBB3_3
; %bb.2:
	v_add_f32_e64 v4, |v3|, |v3|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v5, 0x3fb8aa3b, v4
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v4
	v_rndne_f32_e32 v6, v5
	v_fma_f32 v7, 0x3fb8aa3b, v4, -v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v5, v5, v6
	v_fmamk_f32 v7, v4, 0x32a5705f, v7
	v_cvt_i32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v5, v5, v7
	v_exp_f32_e32 v5, v5
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v5, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v4
	v_cndmask_b32_e32 v4, 0x7f800000, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v4, 1.0, v4
	v_rcp_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, v4, -2.0, 1.0
.LBB3_3:
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execz .LBB3_5
; %bb.4:
	v_mul_f32_e32 v4, v3, v3
	s_mov_b32 s3, 0xbbbac73d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fmaak_f32 v5, s3, v4, 0x3ca908c9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v5, v4, v5, 0xbd5c1c4e
	v_fmaak_f32 v5, v4, v5, 0x3e088382
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v5, v4, v5, 0xbeaaaa99
	v_mul_f32_e64 v5, |v3|, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f32 v4, v4, v5, |v3|
.LBB3_5:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v5, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v6, null, s5, v1, vcc_lo
	v_bfi_b32 v3, 0x7fffffff, v4, v3
	v_mul_f32_e32 v4, 0.5, v2
	v_add_co_u32 v0, vcc_lo, s0, v0
	global_load_b32 v5, v[5:6], off
	v_mul_f32_e32 v6, 0x3e095d4e, v2
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v2, v2, v6, 1.0
	v_fma_f32 v6, -v3, v3, 1.0
	v_dual_add_f32 v3, 1.0, v3 :: v_dual_mul_f32 v2, 0x3f4c422a, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, v4, v6
	v_mul_f32_e32 v2, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v2, 0.5, v3
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v2, v5, v2
	global_store_b32 v[0:1], v2, off
.LBB3_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gelu_backward_f32_kernel
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
	.size	gelu_backward_f32_kernel, .Lfunc_end3-gelu_backward_f32_kernel
                                        ; -- End function
	.set gelu_backward_f32_kernel.num_vgpr, 8
	.set gelu_backward_f32_kernel.num_agpr, 0
	.set gelu_backward_f32_kernel.numbered_sgpr, 8
	.set gelu_backward_f32_kernel.num_named_barrier, 0
	.set gelu_backward_f32_kernel.private_seg_size, 0
	.set gelu_backward_f32_kernel.uses_vcc, 1
	.set gelu_backward_f32_kernel.uses_flat_scratch, 0
	.set gelu_backward_f32_kernel.has_dyn_sized_stack, 0
	.set gelu_backward_f32_kernel.has_recursion, 0
	.set gelu_backward_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 552
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
	.protected	bias_add_f32_kernel     ; -- Begin function bias_add_f32_kernel
	.globl	bias_add_f32_kernel
	.p2align	8
	.type	bias_add_f32_kernel,@function
bias_add_f32_kernel:                    ; @bias_add_f32_kernel
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
	s_cbranch_execz .LBB4_2
; %bb.1:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
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
	v_mul_lo_u32 v0, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v3, v0
	v_subrev_nc_u32_e32 v2, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
	v_ashrrev_i32_e32 v2, 31, v1
	v_subrev_nc_u32_e32 v3, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v0, v2
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[2:3], 2, v[3:4]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v4, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v5, null, s5, v1, vcc_lo
	v_add_co_u32 v2, vcc_lo, s6, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v3, vcc_lo
	global_load_b32 v4, v[4:5], off
	global_load_b32 v2, v[2:3], off
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v4, v2
	global_store_b32 v[0:1], v2, off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel bias_add_f32_kernel
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
.Lfunc_end4:
	.size	bias_add_f32_kernel, .Lfunc_end4-bias_add_f32_kernel
                                        ; -- End function
	.set bias_add_f32_kernel.num_vgpr, 6
	.set bias_add_f32_kernel.num_agpr, 0
	.set bias_add_f32_kernel.numbered_sgpr, 8
	.set bias_add_f32_kernel.num_named_barrier, 0
	.set bias_add_f32_kernel.private_seg_size, 0
	.set bias_add_f32_kernel.uses_vcc, 1
	.set bias_add_f32_kernel.uses_flat_scratch, 0
	.set bias_add_f32_kernel.has_dyn_sized_stack, 0
	.set bias_add_f32_kernel.has_recursion, 0
	.set bias_add_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 344
; TotalNumSgprs: 10
; NumVgprs: 6
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
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
	.protected	repeat_rows_f32_kernel  ; -- Begin function repeat_rows_f32_kernel
	.globl	repeat_rows_f32_kernel
	.p2align	8
	.type	repeat_rows_f32_kernel,@function
repeat_rows_f32_kernel:                 ; @repeat_rows_f32_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s5, v1
	s_cbranch_execz .LBB5_2
; %bb.1:
	s_abs_i32 s2, s4
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
	v_mul_lo_u32 v0, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v3, v0
	v_subrev_nc_u32_e32 v2, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
	v_ashrrev_i32_e32 v2, 31, v1
	v_subrev_nc_u32_e32 v3, s2, v0
	v_cmp_le_u32_e32 vcc_lo, s2, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v0, v2
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s1, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b32 v3, v[3:4], off
	s_waitcnt vmcnt(0)
	global_store_b32 v[0:1], v3, off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel repeat_rows_f32_kernel
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
.Lfunc_end5:
	.size	repeat_rows_f32_kernel, .Lfunc_end5-repeat_rows_f32_kernel
                                        ; -- End function
	.set repeat_rows_f32_kernel.num_vgpr, 5
	.set repeat_rows_f32_kernel.num_agpr, 0
	.set repeat_rows_f32_kernel.numbered_sgpr, 6
	.set repeat_rows_f32_kernel.num_named_barrier, 0
	.set repeat_rows_f32_kernel.private_seg_size, 0
	.set repeat_rows_f32_kernel.uses_vcc, 1
	.set repeat_rows_f32_kernel.uses_flat_scratch, 0
	.set repeat_rows_f32_kernel.has_dyn_sized_stack, 0
	.set repeat_rows_f32_kernel.has_recursion, 0
	.set repeat_rows_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 304
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
	.protected	layernorm_f32_kernel    ; -- Begin function layernorm_f32_kernel
	.globl	layernorm_f32_kernel
	.p2align	8
	.type	layernorm_f32_kernel,@function
layernorm_f32_kernel:                   ; @layernorm_f32_kernel
; %bb.0:
	s_load_b64 s[12:13], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s12
	s_cbranch_scc1 .LBB6_26
; %bb.1:
	s_clause 0x1
	s_load_b64 s[14:15], s[0:1], 0x28
	s_load_b256 s[4:11], s[0:1], 0x0
	s_mul_i32 s2, s13, s2
	v_mov_b32_e32 v3, 0
	s_ashr_i32 s3, s2, 31
	s_waitcnt lgkmcnt(0)
	s_load_b32 s12, s[14:15], 0x0
	s_lshl_b64 s[14:15], s[2:3], 2
	v_cmp_gt_i32_e64 s2, s13, v0
	s_add_u32 s4, s4, s14
	s_addc_u32 s5, s5, s15
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB6_5
; %bb.2:
	s_load_b32 s16, s[0:1], 0x3c
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s17, s16, 0xffff
	s_mov_b32 s16, 0
.LBB6_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	v_add_nc_u32_e32 v1, s17, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s4, v4
	v_add_co_ci_u32_e64 v5, null, s5, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e32 vcc_lo, s13, v1
	global_load_b32 v2, v[4:5], off
	s_or_b32 s16, vcc_lo, s16
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v3, v3, v2
	s_and_not1_b32 exec_lo, exec_lo, s16
	s_cbranch_execnz .LBB6_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s16
.LBB6_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s3
	v_mbcnt_lo_u32_b32 v1, -1, 0
	v_and_b32_e32 v9, 31, v0
	v_lshl_or_b32 v4, v1, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cmp_eq_u32_e64 s3, 0, v9
	ds_bpermute_b32 v2, v4, v3
	v_cndmask_b32_e64 v5, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v1
	v_add_lshl_u32 v5, v5, v1, 2
	v_cndmask_b32_e64 v6, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_lshl_u32 v6, v6, v1, 2
	v_cndmask_b32_e64 v7, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v1
	v_add_lshl_u32 v7, v7, v1, 2
	v_add_co_ci_u32_e64 v8, null, 0, v1, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b32_e32 v8, 2, v8
	ds_bpermute_b32 v3, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v2, v2, v3
	ds_bpermute_b32 v3, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v2, v2, v3
	ds_bpermute_b32 v3, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v2, v3
	v_lshrrev_b32_e32 v3, 5, v0
	ds_bpermute_b32 v2, v8, v1
	v_lshlrev_b32_e32 v11, 2, v3
	s_and_saveexec_b32 s16, s3
	s_cbranch_execz .LBB6_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v11, v1
.LBB6_7:
	s_or_b32 exec_lo, exec_lo, s16
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_load_b32 s0, s[0:1], 0x3c
	s_waitcnt lgkmcnt(0)
	s_and_b32 s1, s0, 0xffff
	v_cmp_gt_u32_e64 s0, 32, v0
	s_add_i32 s16, s1, 31
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshr_b32 s16, s16, 5
	s_and_saveexec_b32 s17, s0
	s_cbranch_execz .LBB6_12
; %bb.8:
	v_mov_b32_e32 v1, 0
	s_mov_b32 s18, exec_lo
	v_cmpx_gt_u32_e64 s16, v9
; %bb.9:
	v_lshlrev_b32_e32 v1, 2, v9
	ds_load_b32 v1, v1
; %bb.10:
	s_or_b32 exec_lo, exec_lo, s18
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v5, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB6_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB6_12:
	s_or_b32 exec_lo, exec_lo, s17
	v_mov_b32_e32 v12, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cvt_f32_i32_e32 v10, s13
	ds_load_b32 v1, v12
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v2, null, v10, v10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v3, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v13, -v2, v3, 1.0
	v_fmac_f32_e32 v3, v13, v3
	v_div_scale_f32 v13, vcc_lo, v1, v10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v14, v13, v3
	v_fma_f32 v15, -v2, v14, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v14, v15, v3
	v_fma_f32 v2, -v2, v14, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v3, v14
	v_div_fixup_f32 v3, v2, v10, v1
	s_and_saveexec_b32 s17, s2
	s_cbranch_execz .LBB6_16
; %bb.13:
	v_dual_mov_b32 v12, 0 :: v_dual_mov_b32 v1, v0
	s_mov_b32 s18, 0
.LBB6_14:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[13:14], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v13, vcc_lo, s4, v13
	v_add_co_ci_u32_e64 v14, null, s5, v14, vcc_lo
	global_load_b32 v2, v[13:14], off
	s_waitcnt vmcnt(0)
	v_dual_sub_f32 v2, v2, v3 :: v_dual_add_nc_u32 v1, s1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_i32_e32 vcc_lo, s13, v1
	v_fmac_f32_e32 v12, v2, v2
	s_or_b32 s18, vcc_lo, s18
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s18
	s_cbranch_execnz .LBB6_14
; %bb.15:
	s_or_b32 exec_lo, exec_lo, s18
.LBB6_16:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s17
	ds_bpermute_b32 v1, v4, v12
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v12, v1
	ds_bpermute_b32 v2, v5, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_saveexec_b32 s17, s3
	s_cbranch_execz .LBB6_18
; %bb.17:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v11, v1
.LBB6_18:
	s_or_b32 exec_lo, exec_lo, s17
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s17, s0
	s_cbranch_execz .LBB6_23
; %bb.19:
	v_mov_b32_e32 v1, 0
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_u32_e64 s16, v9
; %bb.20:
	v_lshlrev_b32_e32 v1, 2, v9
	ds_load_b32 v1, v1
; %bb.21:
	s_or_b32 exec_lo, exec_lo, s0
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v5, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB6_23
; %bb.22:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1
.LBB6_23:
	s_or_b32 exec_lo, exec_lo, s17
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, s2
	s_cbranch_execz .LBB6_26
; %bb.24:
	v_mov_b32_e32 v1, 0
	s_add_u32 s3, s6, s14
	s_mov_b32 s2, 0
	s_addc_u32 s6, s7, s15
	ds_load_b32 v1, v1
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v2, null, v10, v10, v1
	v_div_scale_f32 v6, vcc_lo, v1, v10, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v4, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v5, -v2, v4, 1.0
	v_fmac_f32_e32 v4, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v5, v6, v4
	v_fma_f32 v7, -v2, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v5, v7, v4
	v_fma_f32 v2, -v2, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v4, v5
	v_div_fixup_f32 v1, v2, v10, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v1, s12, v1
	v_mul_f32_e32 v2, 0x4f800000, v1
	v_cmp_gt_f32_e32 vcc_lo, 0xf800000, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	v_sqrt_f32_e32 v2, v1
	s_waitcnt_depctr 0xfff
	v_add_nc_u32_e32 v4, -1, v2
	v_add_nc_u32_e32 v5, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v6, -v4, v2, v1
	v_fma_f32 v7, -v5, v2, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_ge_f32_e64 s0, 0, v6
	v_cndmask_b32_e64 v2, v2, v4, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_lt_f32_e64 s0, 0, v7
	v_cndmask_b32_e64 v2, v2, v5, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, 0x37800000, v2
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x260
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v2, v1, vcc_lo
	v_div_scale_f32 v2, null, v1, v1, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v4, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v5, -v2, v4, 1.0
	v_fmac_f32_e32 v4, v5, v4
	v_div_scale_f32 v5, vcc_lo, 1.0, v1, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v6, v5, v4
	v_fma_f32 v7, -v2, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v7, v4
	v_fma_f32 v2, -v2, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v4, v6
	v_div_fixup_f32 v2, v2, v1, 1.0
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB6_25:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 2, v[0:1]
	v_add_nc_u32_e32 v0, s1, v0
	v_add_co_u32 v6, vcc_lo, s4, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s5, v5, vcc_lo
	global_load_b32 v1, v[6:7], off
	v_add_co_u32 v6, vcc_lo, s8, v4
	v_add_co_ci_u32_e64 v7, null, s9, v5, vcc_lo
	v_add_co_u32 v8, vcc_lo, s10, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s11, v5, vcc_lo
	global_load_b32 v6, v[6:7], off
	global_load_b32 v7, v[8:9], off
	v_add_co_u32 v4, s0, s3, v4
	v_add_co_ci_u32_e64 v5, null, s6, v5, s0
	v_cmp_le_i32_e32 vcc_lo, s13, v0
	s_or_b32 s2, vcc_lo, s2
	s_waitcnt vmcnt(2)
	v_sub_f32_e32 v1, v1, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v1, v2, v1
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v7, v6, v1
	global_store_b32 v[4:5], v7, off
	s_and_not1_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB6_25
.LBB6_26:
	s_set_inst_prefetch_distance 0x2
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel layernorm_f32_kernel
		.amdhsa_group_segment_fixed_size 128
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
	.size	layernorm_f32_kernel, .Lfunc_end6-layernorm_f32_kernel
                                        ; -- End function
	.set layernorm_f32_kernel.num_vgpr, 16
	.set layernorm_f32_kernel.num_agpr, 0
	.set layernorm_f32_kernel.numbered_sgpr, 19
	.set layernorm_f32_kernel.num_named_barrier, 0
	.set layernorm_f32_kernel.private_seg_size, 0
	.set layernorm_f32_kernel.uses_vcc, 1
	.set layernorm_f32_kernel.uses_flat_scratch, 0
	.set layernorm_f32_kernel.has_dyn_sized_stack, 0
	.set layernorm_f32_kernel.has_recursion, 0
	.set layernorm_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1704
; TotalNumSgprs: 21
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 128 bytes/workgroup (compile time only)
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
	.protected	layernorm_backward_f32_kernel ; -- Begin function layernorm_backward_f32_kernel
	.globl	layernorm_backward_f32_kernel
	.p2align	8
	.type	layernorm_backward_f32_kernel,@function
layernorm_backward_f32_kernel:          ; @layernorm_backward_f32_kernel
; %bb.0:
	s_load_b64 s[16:17], s[0:1], 0x30
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s16
	s_cbranch_scc1 .LBB7_43
; %bb.1:
	s_clause 0x2
	s_load_b64 s[18:19], s[0:1], 0x38
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b128 s[12:15], s[0:1], 0x20
	s_mul_i32 s2, s17, s2
	v_mov_b32_e32 v3, 0
	s_ashr_i32 s3, s2, 31
	s_waitcnt lgkmcnt(0)
	s_load_b32 s21, s[18:19], 0x0
	s_lshl_b64 s[18:19], s[2:3], 2
	v_cmp_gt_i32_e64 s2, s17, v0
	s_add_u32 s6, s6, s18
	s_addc_u32 s7, s7, s19
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB7_5
; %bb.2:
	s_load_b32 s16, s[0:1], 0x4c
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s20, s16, 0xffff
	s_mov_b32 s16, 0
.LBB7_3:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	v_add_nc_u32_e32 v1, s20, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s6, v4
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_i32_e32 vcc_lo, s17, v1
	global_load_b32 v2, v[4:5], off
	s_or_b32 s16, vcc_lo, s16
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v3, v3, v2
	s_and_not1_b32 exec_lo, exec_lo, s16
	s_cbranch_execnz .LBB7_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s16
.LBB7_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s3
	v_mbcnt_lo_u32_b32 v1, -1, 0
	v_and_b32_e32 v15, 31, v0
	v_lshl_or_b32 v9, v1, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cmp_eq_u32_e64 s3, 0, v15
	ds_bpermute_b32 v2, v9, v3
	v_cndmask_b32_e64 v4, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v1
	v_add_lshl_u32 v10, v4, v1, 2
	v_cndmask_b32_e64 v4, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_lshl_u32 v11, v4, v1, 2
	v_cndmask_b32_e64 v4, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v1
	v_add_lshl_u32 v12, v4, v1, 2
	v_add_co_ci_u32_e64 v4, null, 0, v1, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_add_f32 v2, v3, v2 :: v_dual_lshlrev_b32 v13, 2, v4
	ds_bpermute_b32 v3, v10, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v2, v2, v3
	ds_bpermute_b32 v3, v11, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v2, v2, v3
	ds_bpermute_b32 v3, v12, v2
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v2, v3
	v_lshrrev_b32_e32 v3, 5, v0
	ds_bpermute_b32 v2, v13, v1
	v_lshlrev_b32_e32 v16, 2, v3
	s_and_saveexec_b32 s16, s3
	s_cbranch_execz .LBB7_7
; %bb.6:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v16, v1 offset:256
.LBB7_7:
	s_or_b32 exec_lo, exec_lo, s16
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_load_b32 s0, s[0:1], 0x4c
	s_waitcnt lgkmcnt(0)
	s_and_b32 s16, s0, 0xffff
	v_cmp_gt_u32_e64 s0, 32, v0
	s_add_i32 s1, s16, 31
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshr_b32 s20, s1, 5
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB7_12
; %bb.8:
	v_mov_b32_e32 v1, 0
	s_mov_b32 s22, exec_lo
	v_cmpx_gt_u32_e64 s20, v15
; %bb.9:
	v_lshlrev_b32_e32 v1, 2, v15
	ds_load_b32 v1, v1 offset:256
; %bb.10:
	s_or_b32 exec_lo, exec_lo, s22
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v9, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v10, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v11, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v12, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v13, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB7_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1 offset:256
.LBB7_12:
	s_or_b32 exec_lo, exec_lo, s1
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cvt_f32_i32_e32 v14, s17
	ds_load_b32 v1, v3 offset:256
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v2, null, v14, v14, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v4, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v5, -v2, v4, 1.0
	v_fmac_f32_e32 v4, v5, v4
	v_div_scale_f32 v5, vcc_lo, v1, v14, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v6, v5, v4
	v_fma_f32 v7, -v2, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v7, v4
	v_fma_f32 v2, -v2, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v4, v6
	v_div_fixup_f32 v8, v2, v14, v1
	s_and_saveexec_b32 s1, s2
	s_cbranch_execz .LBB7_16
; %bb.13:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, v0
	s_mov_b32 s22, 0
.LBB7_14:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[4:5], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s6, v4
	v_add_co_ci_u32_e64 v5, null, s7, v5, vcc_lo
	global_load_b32 v2, v[4:5], off
	s_waitcnt vmcnt(0)
	v_dual_sub_f32 v2, v2, v8 :: v_dual_add_nc_u32 v1, s16, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_i32_e32 vcc_lo, s17, v1
	v_fmac_f32_e32 v3, v2, v2
	s_or_b32 s22, vcc_lo, s22
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s22
	s_cbranch_execnz .LBB7_14
; %bb.15:
	s_or_b32 exec_lo, exec_lo, s22
.LBB7_16:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s1
	ds_bpermute_b32 v1, v9, v3
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v3, v1
	ds_bpermute_b32 v2, v10, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v11, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v12, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v13, v1
	s_and_saveexec_b32 s1, s3
	s_cbranch_execz .LBB7_18
; %bb.17:
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v16, v1 offset:256
.LBB7_18:
	s_or_b32 exec_lo, exec_lo, s1
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB7_23
; %bb.19:
	v_mov_b32_e32 v1, 0
	s_mov_b32 s22, exec_lo
	v_cmpx_gt_u32_e64 s20, v15
; %bb.20:
	v_lshlrev_b32_e32 v1, 2, v15
	ds_load_b32 v1, v1 offset:256
; %bb.21:
	s_or_b32 exec_lo, exec_lo, s22
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v9, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v10, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v11, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v12, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v13, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB7_23
; %bb.22:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v2 :: v_dual_mov_b32 v2, 0
	ds_store_b32 v2, v1 offset:256
.LBB7_23:
	s_or_b32 exec_lo, exec_lo, s1
	v_mov_b32_e32 v18, 0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_mov_b32_e32 v19, 0
	ds_load_b32 v1, v18 offset:256
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v2, null, v14, v14, v1
	v_div_scale_f32 v5, vcc_lo, v1, v14, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v3, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, -v2, v3, 1.0
	v_fmac_f32_e32 v3, v4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, v5, v3
	v_fma_f32 v6, -v2, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v4, v6, v3
	v_fma_f32 v2, -v2, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v3, v4
	v_div_fixup_f32 v1, v2, v14, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v1, s21, v1
	v_mul_f32_e32 v2, 0x4f800000, v1
	v_cmp_gt_f32_e32 vcc_lo, 0xf800000, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	v_sqrt_f32_e32 v2, v1
	s_waitcnt_depctr 0xfff
	v_add_nc_u32_e32 v3, -1, v2
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v5, -v3, v2, v1
	v_fma_f32 v6, -v4, v2, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_ge_f32_e64 s1, 0, v5
	v_cndmask_b32_e64 v2, v2, v3, s1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_lt_f32_e64 s1, 0, v6
	v_cndmask_b32_e64 v2, v2, v4, s1
	s_add_u32 s1, s4, s18
	s_addc_u32 s4, s5, s19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v3, 0x37800000, v2
	v_cndmask_b32_e32 v2, v2, v3, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v1, 0x260
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v1, v2, v1, vcc_lo
	v_div_scale_f32 v2, null, v1, v1, 1.0
	v_div_scale_f32 v5, vcc_lo, 1.0, v1, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v3, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, -v2, v3, 1.0
	v_fmac_f32_e32 v3, v4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, v5, v3
	v_fma_f32 v6, -v2, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v4, v6, v3
	v_fma_f32 v2, -v2, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v2, v2, v3, v4
	v_div_fixup_f32 v17, v2, v1, 1.0
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB7_31
; %bb.24:
	v_dual_mov_b32 v18, 0 :: v_dual_mov_b32 v19, 0
	v_mov_b32_e32 v1, v0
	s_mov_b32 s21, 0
.LBB7_25:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB7_26 Depth 2
                                        ;     Child Loop BB7_28 Depth 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s22, 0
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s6, v2
	v_add_co_ci_u32_e64 v5, null, s7, v3, vcc_lo
	v_add_co_u32 v6, vcc_lo, s1, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s4, v3, vcc_lo
	global_load_b32 v22, v[4:5], off
	global_load_b32 v20, v[6:7], off
	v_add_co_u32 v6, vcc_lo, s8, v2
	v_add_co_ci_u32_e64 v7, null, s9, v3, vcc_lo
	v_add_co_u32 v4, vcc_lo, s12, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s13, v3, vcc_lo
	global_load_b32 v21, v[6:7], off
	global_load_b32 v7, v[4:5], off
	s_waitcnt vmcnt(3)
	v_sub_f32_e32 v6, v22, v8
	v_mul_f32_e32 v22, v17, v6
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f32_e32 v23, v20, v22
.LBB7_26:                               ;   Parent Loop BB7_25 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v6, v7, v23
	global_atomic_cmpswap_b32 v6, v[4:5], v[6:7], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v6, v7
	v_mov_b32_e32 v7, v6
	s_or_b32 s22, vcc_lo, s22
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s22
	s_cbranch_execnz .LBB7_26
; %bb.27:                               ;   in Loop: Header=BB7_25 Depth=1
	s_or_b32 exec_lo, exec_lo, s22
	v_add_co_u32 v2, vcc_lo, s14, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s15, v3, vcc_lo
	s_mov_b32 s22, 0
	global_load_b32 v5, v[2:3], off
.LBB7_28:                               ;   Parent Loop BB7_25 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v4, v5, v20
	global_atomic_cmpswap_b32 v4, v[2:3], v[4:5], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v4, v5
	v_mov_b32_e32 v5, v4
	s_or_b32 s22, vcc_lo, s22
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s22
	s_cbranch_execnz .LBB7_28
; %bb.29:                               ;   in Loop: Header=BB7_25 Depth=1
	s_or_b32 exec_lo, exec_lo, s22
	v_add_nc_u32_e32 v1, s16, v1
	v_mul_f32_e32 v2, v20, v21
	v_fmac_f32_e32 v18, v20, v21
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_i32_e32 vcc_lo, s17, v1
	v_fmac_f32_e32 v19, v2, v22
	s_or_b32 s21, vcc_lo, s21
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s21
	s_cbranch_execnz .LBB7_25
; %bb.30:
	s_or_b32 exec_lo, exec_lo, s21
.LBB7_31:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
	ds_bpermute_b32 v1, v9, v18
	ds_bpermute_b32 v2, v9, v19
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v18, v1 :: v_dual_add_f32 v2, v19, v2
	ds_bpermute_b32 v3, v10, v1
	ds_bpermute_b32 v4, v10, v2
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v3 :: v_dual_add_f32 v2, v2, v4
	ds_bpermute_b32 v3, v11, v1
	ds_bpermute_b32 v4, v11, v2
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v3 :: v_dual_add_f32 v2, v2, v4
	ds_bpermute_b32 v3, v12, v1
	ds_bpermute_b32 v4, v12, v2
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v1, v3 :: v_dual_add_f32 v2, v2, v4
	ds_bpermute_b32 v3, v13, v1
	ds_bpermute_b32 v4, v13, v2
	s_and_saveexec_b32 s5, s3
	s_cbranch_execz .LBB7_33
; %bb.32:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v2, v2, v4 :: v_dual_add_f32 v1, v1, v3
	ds_store_2addr_b32 v16, v1, v2 offset1:32
.LBB7_33:
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s5, s0
	s_cbranch_execz .LBB7_40
; %bb.34:
	v_cmp_gt_u32_e32 vcc_lo, s20, v15
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, 0
	v_lshlrev_b32_e32 v3, 2, v15
	s_and_saveexec_b32 s0, vcc_lo
; %bb.35:
	ds_load_b32 v2, v3
; %bb.36:
	s_or_b32 exec_lo, exec_lo, s0
	s_and_saveexec_b32 s0, vcc_lo
; %bb.37:
	ds_load_b32 v1, v3 offset:128
; %bb.38:
	s_or_b32 exec_lo, exec_lo, s0
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v9, v2
	ds_bpermute_b32 v4, v9, v1
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v2, v2, v3 :: v_dual_add_f32 v1, v1, v4
	ds_bpermute_b32 v3, v10, v2
	ds_bpermute_b32 v4, v10, v1
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v2, v2, v3 :: v_dual_add_f32 v1, v1, v4
	ds_bpermute_b32 v3, v11, v2
	ds_bpermute_b32 v4, v11, v1
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v2, v2, v3 :: v_dual_add_f32 v3, v1, v4
	ds_bpermute_b32 v1, v12, v2
	ds_bpermute_b32 v4, v12, v3
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v1, v2, v1 :: v_dual_add_f32 v2, v3, v4
	ds_bpermute_b32 v3, v13, v1
	ds_bpermute_b32 v4, v13, v2
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB7_40
; %bb.39:
	s_waitcnt lgkmcnt(0)
	v_dual_add_f32 v2, v2, v4 :: v_dual_add_f32 v1, v1, v3
	v_mov_b32_e32 v3, 0
	ds_store_2addr_b32 v3, v1, v2 offset1:32
.LBB7_40:
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, s2
	s_cbranch_execz .LBB7_43
; %bb.41:
	v_div_scale_f32 v3, null, v14, v14, 1.0
	v_div_scale_f32 v5, vcc_lo, 1.0, v14, 1.0
	s_add_u32 s2, s10, s18
	v_rcp_f32_e32 v4, v3
	s_addc_u32 s3, s11, s19
	s_mov_b32 s5, 0
	s_waitcnt_depctr 0xfff
	v_fma_f32 v1, -v3, v4, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v4, v1, v4 :: v_dual_mov_b32 v1, 0
	v_mul_f32_e32 v6, v5, v4
	ds_load_2addr_b32 v[1:2], v1 offset1:32
	v_fma_f32 v7, -v3, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v7, v4
	v_fma_f32 v3, -v3, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v3, v3, v4, v6
	v_div_fixup_f32 v3, v3, v14, 1.0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f32_e32 v4, v3, v1
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB7_42:                               ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[5:6], 2, v[0:1]
	v_add_nc_u32_e32 v0, s16, v0
	v_add_co_u32 v9, vcc_lo, s6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s7, v6, vcc_lo
	v_add_co_u32 v11, vcc_lo, s1, v5
	v_add_co_ci_u32_e64 v12, null, s4, v6, vcc_lo
	global_load_b32 v1, v[9:10], off
	v_add_co_u32 v9, vcc_lo, s8, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s9, v6, vcc_lo
	global_load_b32 v7, v[11:12], off
	global_load_b32 v9, v[9:10], off
	v_add_co_u32 v5, s0, s2, v5
	v_add_co_ci_u32_e64 v6, null, s3, v6, s0
	v_cmp_le_i32_e32 vcc_lo, s17, v0
	s_or_b32 s5, vcc_lo, s5
	s_waitcnt vmcnt(2)
	v_sub_f32_e32 v1, v1, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v1, v17, v1
	s_waitcnt vmcnt(0)
	v_fma_f32 v7, v7, v9, -v4
	v_mul_f32_e32 v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v1, -v3, v1, v7
	v_mul_f32_e32 v1, v17, v1
	global_store_b32 v[5:6], v1, off
	s_and_not1_b32 exec_lo, exec_lo, s5
	s_cbranch_execnz .LBB7_42
.LBB7_43:
	s_set_inst_prefetch_distance 0x2
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel layernorm_backward_f32_kernel
		.amdhsa_group_segment_fixed_size 384
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 320
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
		.amdhsa_next_free_vgpr 24
		.amdhsa_next_free_sgpr 23
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
		.amdhsa_inst_pref_size 21
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
	.size	layernorm_backward_f32_kernel, .Lfunc_end7-layernorm_backward_f32_kernel
                                        ; -- End function
	.set layernorm_backward_f32_kernel.num_vgpr, 24
	.set layernorm_backward_f32_kernel.num_agpr, 0
	.set layernorm_backward_f32_kernel.numbered_sgpr, 23
	.set layernorm_backward_f32_kernel.num_named_barrier, 0
	.set layernorm_backward_f32_kernel.private_seg_size, 0
	.set layernorm_backward_f32_kernel.uses_vcc, 1
	.set layernorm_backward_f32_kernel.uses_flat_scratch, 0
	.set layernorm_backward_f32_kernel.has_dyn_sized_stack, 0
	.set layernorm_backward_f32_kernel.has_recursion, 0
	.set layernorm_backward_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2636
; TotalNumSgprs: 25
; NumVgprs: 24
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 384 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 25
; NumVGPRsForWavesPerEU: 24
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
	.protected	avg_pool_2d_f32_kernel  ; -- Begin function avg_pool_2d_f32_kernel
	.globl	avg_pool_2d_f32_kernel
	.p2align	8
	.type	avg_pool_2d_f32_kernel,@function
avg_pool_2d_f32_kernel:                 ; @avg_pool_2d_f32_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x44
	s_load_b256 s[4:11], s[0:1], 0x10
	s_load_b64 s[16:17], s[0:1], 0x30
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	s_mul_i32 s2, s2, s16
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s17
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB8_12
; %bb.1:
	s_load_b128 s[12:15], s[0:1], 0x0
	s_cmp_lt_i32 s8, 1
	s_cbranch_scc1 .LBB8_10
; %bb.2:
	s_abs_i32 s0, s17
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s0
	s_sub_i32 s1, 0, s0
	s_mov_b32 s2, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s1, v0
	s_abs_i32 s1, s16
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s1
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s0, v2
	v_cmp_le_u32_e32 vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s17, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s0, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s0, 0, s1
	s_cmp_gt_i32 s9, 0
	v_mul_lo_u32 v2, s0, v4
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	v_xor_b32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_sub_nc_u32_e32 v3, v0, v3
	v_mul_hi_u32 v0, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, 0, v3
	v_add_nc_u32_e32 v0, v4, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v2, v3, v2
	v_mul_hi_u32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, v0, s1
	v_sub_nc_u32_e32 v2, v2, v4
	v_add_nc_u32_e32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_subrev_nc_u32_e32 v5, s1, v2
	v_cmp_le_u32_e32 vcc_lo, s1, v2
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, v2, v5, vcc_lo
	v_xor_b32_e32 v4, s16, v3
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s1, v2
	v_ashrrev_i32_e32 v4, 31, v4
	s_cselect_b32 s1, -1, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	v_mul_lo_u32 v5, v3, s17
	v_xor_b32_e32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v4, v0, v4
	v_mul_lo_u32 v0, v4, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v3, v0
	v_mul_lo_u32 v0, v0, s10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[2:3], null, v4, s6, v[0:1]
	v_sub_nc_u32_e32 v3, v1, v5
	v_mov_b32_e32 v5, 0
	v_mul_lo_u32 v3, v3, s11
	s_delay_alu instid0(VALU_DEP_4)
	v_mul_lo_u32 v4, s7, v2
	v_mov_b32_e32 v2, 0
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB8_4
	.p2align	6
.LBB8_3:                                ;   in Loop: Header=BB8_4 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_nc_u32_e32 v4, s7, v4
	s_add_i32 s2, s2, 1
	s_cmp_eq_u32 s2, s8
	s_cbranch_scc1 .LBB8_9
.LBB8_4:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB8_7 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s1
	s_cbranch_vccnz .LBB8_3
; %bb.5:                                ;   in Loop: Header=BB8_4 Depth=1
	v_add_nc_u32_e32 v6, s2, v0
	s_mov_b32 s3, s9
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_i32_e32 vcc_lo, s6, v6
	v_mov_b32_e32 v6, v3
	s_branch .LBB8_7
	.p2align	6
.LBB8_6:                                ;   in Loop: Header=BB8_7 Depth=2
	s_or_b32 exec_lo, exec_lo, s4
	v_add_nc_u32_e32 v6, 1, v6
	s_add_i32 s3, s3, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s3, 0
	s_cbranch_scc1 .LBB8_3
.LBB8_7:                                ;   Parent Loop BB8_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e64 s0, s7, v6
	s_and_b32 s0, vcc_lo, s0
	s_and_saveexec_b32 s4, s0
	s_cbranch_execz .LBB8_6
; %bb.8:                                ;   in Loop: Header=BB8_7 Depth=2
	v_add_nc_u32_e32 v7, v4, v6
	v_add_nc_u32_e32 v5, 1, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	v_lshlrev_b64 v[7:8], 2, v[7:8]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, s0, s12, v7
	v_add_co_ci_u32_e64 v8, null, s13, v8, s0
	global_load_b32 v7, v[7:8], off
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v2, v7
	s_branch .LBB8_6
.LBB8_9:
	s_set_inst_prefetch_distance 0x2
	v_cvt_f32_i32_e32 v0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f32 v3, null, v0, v0, v2
	v_rcp_f32_e32 v4, v3
	s_waitcnt_depctr 0xfff
	v_fma_f32 v5, -v3, v4, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v4, v5, v4
	v_div_scale_f32 v5, vcc_lo, v2, v0, v2
	v_mul_f32_e32 v6, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v7, -v3, v6, v5
	v_fmac_f32_e32 v6, v7, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v3, -v3, v6, v5
	v_div_fmas_f32 v3, v3, v4, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f32 v0, v3, v0, v2
	s_branch .LBB8_11
.LBB8_10:
	v_mov_b32_e32 v0, 0x7fc00000
.LBB8_11:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v1, vcc_lo, s14, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s15, v2, vcc_lo
	global_store_b32 v[1:2], v0, off
.LBB8_12:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel avg_pool_2d_f32_kernel
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
		.amdhsa_next_free_vgpr 9
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
.Lfunc_end8:
	.size	avg_pool_2d_f32_kernel, .Lfunc_end8-avg_pool_2d_f32_kernel
                                        ; -- End function
	.set avg_pool_2d_f32_kernel.num_vgpr, 9
	.set avg_pool_2d_f32_kernel.num_agpr, 0
	.set avg_pool_2d_f32_kernel.numbered_sgpr, 18
	.set avg_pool_2d_f32_kernel.num_named_barrier, 0
	.set avg_pool_2d_f32_kernel.private_seg_size, 0
	.set avg_pool_2d_f32_kernel.uses_vcc, 1
	.set avg_pool_2d_f32_kernel.uses_flat_scratch, 0
	.set avg_pool_2d_f32_kernel.has_dyn_sized_stack, 0
	.set avg_pool_2d_f32_kernel.has_recursion, 0
	.set avg_pool_2d_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 856
; TotalNumSgprs: 20
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 20
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
	.protected	avg_pool_2d_backward_f32_kernel ; -- Begin function avg_pool_2d_backward_f32_kernel
	.globl	avg_pool_2d_backward_f32_kernel
	.p2align	8
	.type	avg_pool_2d_backward_f32_kernel,@function
avg_pool_2d_backward_f32_kernel:        ; @avg_pool_2d_backward_f32_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x44
	s_load_b256 s[4:11], s[0:1], 0x10
	s_load_b64 s[12:13], s[0:1], 0x30
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	s_mul_i32 s2, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s13
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB9_15
; %bb.1:
	s_abs_i32 s3, s13
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s3
	s_sub_i32 s2, 0, s3
	s_cmp_lt_i32 s8, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v3, v0
	v_mul_lo_u32 v2, v0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v3, s3, v2
	v_cmp_le_u32_e64 s2, s3, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, v2, v3, s2
	v_cmp_le_u32_e32 vcc_lo, s3, v2
	s_cbranch_scc1 .LBB9_15
; %bb.2:
	s_abs_i32 s3, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v2, s3
	v_rcp_iflag_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_dual_mul_f32 v2, 0x4f7ffffe, v2 :: v_dual_add_nc_u32 v3, 1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v0, v0, v3, s2
	v_xor_b32_e32 v3, s13, v1
	s_sub_i32 s2, 0, s3
	v_cvt_u32_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_ashrrev_i32_e32 v3, 31, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_mul_lo_u32 v4, s2, v2
	s_ashr_i32 s2, s12, 31
	s_cmp_gt_i32 s9, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v5, v0, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v0, v2, v4
	v_sub_nc_u32_e32 v3, 0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v0, v2, v0
	v_max_i32_e32 v4, v5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[2:3], null, v4, v0, 0
	v_mul_lo_u32 v0, v3, s3
	v_add_nc_u32_e32 v2, 1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v4, v0
	v_subrev_nc_u32_e32 v4, s3, v0
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, v3, v2, vcc_lo
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_ashrrev_i32_e32 v3, 31, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s3, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_xor_b32_e32 v3, s2, v3
	s_cselect_b32 s3, -1, 0
	v_cndmask_b32_e64 v6, 0, 1, s3
	v_cndmask_b32_e32 v0, v2, v4, vcc_lo
	v_mul_lo_u32 v2, v5, s13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v2, v1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v0, v3
	v_mul_lo_u32 v4, v2, s11
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v3, v0, s12
	v_mov_b32_e32 v2, 0
	v_sub_nc_u32_e32 v3, v5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_lo_u32 v5, v3, s10
	v_mov_b32_e32 v3, 0
	s_branch .LBB9_4
	.p2align	6
.LBB9_3:                                ;   in Loop: Header=BB9_4 Depth=1
	v_add_nc_u32_e32 v2, 1, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, s8, v2
	s_cbranch_vccnz .LBB9_7
.LBB9_4:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB9_6 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccnz .LBB9_3
; %bb.5:                                ;   in Loop: Header=BB9_4 Depth=1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v7, v2, v5
	s_mov_b32 s4, s9
	v_cmp_gt_i32_e32 vcc_lo, s6, v7
	v_mov_b32_e32 v7, v4
.LBB9_6:                                ;   Parent Loop BB9_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_i32_e64 s2, s7, v7
	v_add_nc_u32_e32 v7, 1, v7
	s_add_i32 s4, s4, -1
	s_and_b32 s2, vcc_lo, s2
	s_cmp_eq_u32 s4, 0
	v_cndmask_b32_e64 v8, 0, 1, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_nc_u32_e32 v3, v3, v8
	s_cbranch_scc0 .LBB9_6
	s_branch .LBB9_3
.LBB9_7:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v1, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s1, v2, vcc_lo
	s_mov_b32 s1, 0
	global_load_b32 v1, v[1:2], off
	v_cvt_f32_i32_e32 v2, v3
	s_waitcnt vmcnt(0)
	v_div_scale_f32 v3, null, v2, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v7, v3
	s_waitcnt_depctr 0xfff
	v_fma_f32 v8, -v3, v7, 1.0
	v_fmac_f32_e32 v7, v8, v7
	v_div_scale_f32 v8, vcc_lo, v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v9, v8, v7
	v_fma_f32 v10, -v3, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v9, v10, v7
	v_fma_f32 v3, -v3, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f32 v3, v3, v7, v9
	v_mul_lo_u32 v7, v0, s6
	v_div_fixup_f32 v8, v3, v2, v1
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB9_9
	.p2align	6
.LBB9_8:                                ;   in Loop: Header=BB9_9 Depth=1
	s_add_i32 s1, s1, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s1, s8
	s_cbranch_scc1 .LBB9_15
.LBB9_9:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB9_12 Depth 2
                                        ;       Child Loop BB9_14 Depth 3
	v_cmp_ne_u32_e32 vcc_lo, 1, v6
	s_cbranch_vccnz .LBB9_8
; %bb.10:                               ;   in Loop: Header=BB9_9 Depth=1
	v_add_nc_u32_e32 v0, s1, v5
	s_mov_b32 s4, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v1, v0, v7
	v_cmp_gt_i32_e32 vcc_lo, s6, v0
	v_mul_lo_u32 v9, v1, s7
	s_branch .LBB9_12
	.p2align	6
.LBB9_11:                               ;   in Loop: Header=BB9_12 Depth=2
	s_or_b32 exec_lo, exec_lo, s5
	s_add_i32 s4, s4, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s4, s9
	s_cbranch_scc1 .LBB9_8
.LBB9_12:                               ;   Parent Loop BB9_9 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB9_14 Depth 3
	v_add_nc_u32_e32 v0, s4, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e64 s0, s7, v0
	s_and_b32 s0, vcc_lo, s0
	s_and_saveexec_b32 s5, s0
	s_cbranch_execz .LBB9_11
; %bb.13:                               ;   in Loop: Header=BB9_12 Depth=2
	v_add_nc_u32_e32 v0, v0, v9
	s_mov_b32 s10, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v1, 31, v0
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, s0, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, s0
	global_load_b32 v3, v[0:1], off
.LBB9_14:                               ;   Parent Loop BB9_9 Depth=1
                                        ;     Parent Loop BB9_12 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v3, v8
	global_atomic_cmpswap_b32 v2, v[0:1], v[2:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e64 s0, v2, v3
	v_mov_b32_e32 v3, v2
	s_or_b32 s10, s0, s10
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s10
	s_cbranch_execnz .LBB9_14
	s_branch .LBB9_11
.LBB9_15:
	s_set_inst_prefetch_distance 0x2
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel avg_pool_2d_backward_f32_kernel
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
		.amdhsa_next_free_vgpr 11
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
.Lfunc_end9:
	.size	avg_pool_2d_backward_f32_kernel, .Lfunc_end9-avg_pool_2d_backward_f32_kernel
                                        ; -- End function
	.set avg_pool_2d_backward_f32_kernel.num_vgpr, 11
	.set avg_pool_2d_backward_f32_kernel.num_agpr, 0
	.set avg_pool_2d_backward_f32_kernel.numbered_sgpr, 14
	.set avg_pool_2d_backward_f32_kernel.num_named_barrier, 0
	.set avg_pool_2d_backward_f32_kernel.private_seg_size, 0
	.set avg_pool_2d_backward_f32_kernel.uses_vcc, 1
	.set avg_pool_2d_backward_f32_kernel.uses_flat_scratch, 0
	.set avg_pool_2d_backward_f32_kernel.has_dyn_sized_stack, 0
	.set avg_pool_2d_backward_f32_kernel.has_recursion, 0
	.set avg_pool_2d_backward_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1004
; TotalNumSgprs: 16
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 16
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
	.protected	max_pool_2d_f32_kernel  ; -- Begin function max_pool_2d_f32_kernel
	.globl	max_pool_2d_f32_kernel
	.p2align	8
	.type	max_pool_2d_f32_kernel,@function
max_pool_2d_f32_kernel:                 ; @max_pool_2d_f32_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x4c
	s_load_b256 s[4:11], s[0:1], 0x18
	s_load_b64 s[16:17], s[0:1], 0x38
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	s_mul_i32 s2, s2, s16
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s17
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB10_14
; %bb.1:
	s_clause 0x1
	s_load_b128 s[12:15], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_cmp_lt_i32 s8, 1
	s_cbranch_scc1 .LBB10_12
; %bb.2:
	s_abs_i32 s0, s17
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s0
	s_sub_i32 s1, 0, s0
	v_mov_b32_e32 v7, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s1, v0
	s_abs_i32 s1, s16
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s1
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s0, v2
	v_cmp_le_u32_e32 vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s17, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s0, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s0, 0, s1
	s_cmp_gt_i32 s9, 0
	v_mul_lo_u32 v2, s0, v4
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_mul_i32 s0, s10, s7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v3, v0, v3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v0, v4, v2
	v_sub_nc_u32_e32 v2, 0, v3
	v_mul_lo_u32 v6, v3, s17
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v0, v4, v0
	v_max_i32_e32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v2, v0
	v_mul_lo_u32 v4, v0, s1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v2, v4
	v_add_nc_u32_e32 v4, 1, v0
	v_subrev_nc_u32_e32 v5, s1, v2
	v_cmp_le_u32_e32 vcc_lo, s1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_cndmask_b32_e32 v2, v2, v5, vcc_lo
	v_xor_b32_e32 v4, s16, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v5, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v4, 31, v4
	s_cselect_b32 s1, -1, 0
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v4
	v_sub_nc_u32_e32 v2, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v2, s16
	v_sub_nc_u32_e32 v5, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v0, v5, s10
	v_mul_lo_u32 v5, s0, v5
	v_mad_u64_u32 v[3:4], null, v2, s6, v[0:1]
	v_sub_nc_u32_e32 v4, v1, v6
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_lo_u32 v4, v4, s11
	v_mul_lo_u32 v6, s7, v3
	v_mov_b32_e32 v3, 0xff7fffff
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB10_4
	.p2align	6
.LBB10_3:                               ;   in Loop: Header=BB10_4 Depth=1
	v_add_nc_u32_e32 v7, 1, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v6, s7, v6
	v_add_nc_u32_e32 v5, s7, v5
	v_cmp_eq_u32_e32 vcc_lo, s8, v7
	s_cbranch_vccnz .LBB10_11
.LBB10_4:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB10_8 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s1
	s_cbranch_vccnz .LBB10_3
; %bb.5:                                ;   in Loop: Header=BB10_4 Depth=1
	v_add_nc_u32_e32 v8, v7, v0
	s_mov_b32 s4, s9
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_i32_e32 vcc_lo, s6, v8
	v_mov_b32_e32 v8, v4
	s_branch .LBB10_8
	.p2align	6
.LBB10_6:                               ;   in Loop: Header=BB10_8 Depth=2
	s_or_b32 exec_lo, exec_lo, s10
.LBB10_7:                               ;   in Loop: Header=BB10_8 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
	v_add_nc_u32_e32 v8, 1, v8
	s_add_i32 s4, s4, -1
	s_cmp_eq_u32 s4, 0
	s_cbranch_scc1 .LBB10_3
.LBB10_8:                               ;   Parent Loop BB10_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e64 s0, s7, v8
	s_and_b32 s0, vcc_lo, s0
	s_and_saveexec_b32 s5, s0
	s_cbranch_execz .LBB10_7
; %bb.9:                                ;   in Loop: Header=BB10_8 Depth=2
	v_add_nc_u32_e32 v9, v6, v8
	s_mov_b32 s10, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v10, 31, v9
	v_lshlrev_b64 v[9:10], 2, v[9:10]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v9, s0, s12, v9
	v_add_co_ci_u32_e64 v10, null, s13, v10, s0
	global_load_b32 v9, v[9:10], off
	s_waitcnt vmcnt(0)
	v_cmpx_gt_f32_e32 v9, v3
	s_cbranch_execz .LBB10_6
; %bb.10:                               ;   in Loop: Header=BB10_8 Depth=2
	v_add_nc_u32_e32 v2, v5, v8
	v_mov_b32_e32 v3, v9
	s_branch .LBB10_6
.LBB10_11:
	s_set_inst_prefetch_distance 0x2
	v_cvt_f32_i32_e32 v0, v2
	s_branch .LBB10_13
.LBB10_12:
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v3, 0xff7fffff
.LBB10_13:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v4, vcc_lo, s14, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s15, v2, vcc_lo
	v_add_co_u32 v1, vcc_lo, s2, v1
	v_add_co_ci_u32_e64 v2, null, s3, v2, vcc_lo
	global_store_b32 v[4:5], v3, off
	global_store_b32 v[1:2], v0, off
.LBB10_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel max_pool_2d_f32_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 320
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
.Lfunc_end10:
	.size	max_pool_2d_f32_kernel, .Lfunc_end10-max_pool_2d_f32_kernel
                                        ; -- End function
	.set max_pool_2d_f32_kernel.num_vgpr, 11
	.set max_pool_2d_f32_kernel.num_agpr, 0
	.set max_pool_2d_f32_kernel.numbered_sgpr, 18
	.set max_pool_2d_f32_kernel.num_named_barrier, 0
	.set max_pool_2d_f32_kernel.private_seg_size, 0
	.set max_pool_2d_f32_kernel.uses_vcc, 1
	.set max_pool_2d_f32_kernel.uses_flat_scratch, 0
	.set max_pool_2d_f32_kernel.has_dyn_sized_stack, 0
	.set max_pool_2d_f32_kernel.has_recursion, 0
	.set max_pool_2d_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 836
; TotalNumSgprs: 20
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 20
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
	.protected	max_pool_2d_backward_f32_kernel ; -- Begin function max_pool_2d_backward_f32_kernel
	.globl	max_pool_2d_backward_f32_kernel
	.p2align	8
	.type	max_pool_2d_backward_f32_kernel,@function
max_pool_2d_backward_f32_kernel:        ; @max_pool_2d_backward_f32_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s7, s6
	s_mul_i32 s3, s2, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s3, s3, s5
	v_cmp_gt_i32_e32 vcc_lo, s3, v1
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB11_3
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[8:9], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_abs_i32 s3, s2
	v_sub_nc_u32_e32 v6, 0, v1
	s_load_b32 s0, s[0:1], 0x28
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	v_max_i32_e32 v6, v1, v6
	v_xor_b32_e32 v1, s2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ashrrev_i32_e32 v1, 31, v1
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v4, vcc_lo, s6, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s7, v3, vcc_lo
	s_sub_i32 s6, 0, s3
	global_load_b32 v0, v[4:5], off
	v_cvt_f32_u32_e32 v4, s3
	v_rcp_iflag_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v5, s6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v5, v4, v5
	v_add_nc_u32_e32 v4, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v4, v6, v4
	v_mul_lo_u32 v5, v4, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v5, v6, v5
	v_add_nc_u32_e32 v6, 1, v4
	v_subrev_nc_u32_e32 v7, s3, v5
	v_cmp_le_u32_e32 vcc_lo, s3, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v4, v4, v6 :: v_dual_cndmask_b32 v5, v5, v7
	v_add_nc_u32_e32 v6, 1, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s3, v5
	v_cndmask_b32_e32 v4, v4, v6, vcc_lo
	v_add_co_u32 v2, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	v_xor_b32_e32 v4, v4, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v1, v4, v1
	s_waitcnt vmcnt(0)
	v_cvt_i32_f32_e32 v0, v0
	v_mad_u64_u32 v[4:5], null, v1, s0, v[0:1]
	s_mov_b32 s0, 0
	v_ashrrev_i32_e32 v5, 31, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[4:5]
	v_add_co_u32 v0, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	global_load_b32 v4, v[2:3], off
	global_load_b32 v3, v[0:1], off
.LBB11_2:                               ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v3, v4
	global_atomic_cmpswap_b32 v2, v[0:1], v[2:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v2, v3
	v_mov_b32_e32 v3, v2
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB11_2
.LBB11_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel max_pool_2d_backward_f32_kernel
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
.Lfunc_end11:
	.size	max_pool_2d_backward_f32_kernel, .Lfunc_end11-max_pool_2d_backward_f32_kernel
                                        ; -- End function
	.set max_pool_2d_backward_f32_kernel.num_vgpr, 8
	.set max_pool_2d_backward_f32_kernel.num_agpr, 0
	.set max_pool_2d_backward_f32_kernel.numbered_sgpr, 10
	.set max_pool_2d_backward_f32_kernel.num_named_barrier, 0
	.set max_pool_2d_backward_f32_kernel.private_seg_size, 0
	.set max_pool_2d_backward_f32_kernel.uses_vcc, 1
	.set max_pool_2d_backward_f32_kernel.uses_flat_scratch, 0
	.set max_pool_2d_backward_f32_kernel.has_dyn_sized_stack, 0
	.set max_pool_2d_backward_f32_kernel.has_recursion, 0
	.set max_pool_2d_backward_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 448
; TotalNumSgprs: 12
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 8
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
	.protected	lstm_cell_f32_kernel    ; -- Begin function lstm_cell_f32_kernel
	.globl	lstm_cell_f32_kernel
	.p2align	8
	.type	lstm_cell_f32_kernel,@function
lstm_cell_f32_kernel:                   ; @lstm_cell_f32_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s9, s8
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB12_10
; %bb.1:
	s_abs_i32 s2, s9
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b128 s[4:7], s[0:1], 0x0
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
	s_load_b64 s[2:3], s[0:1], 0x10
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, v0, s9
	v_sub_nc_u32_e32 v2, v1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshl_add_u32 v5, v0, 2, v2
	v_lshl_add_u32 v3, s9, 1, v5
	v_add_nc_u32_e32 v7, s9, v5
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v4, 31, v3
	v_ashrrev_i32_e32 v8, 31, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[5:6], 2, v[5:6]
	v_lshlrev_b64 v[9:10], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[7:8], 2, v[7:8]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v5, vcc_lo, s4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v6, null, s5, v6, vcc_lo
	v_add_co_u32 v9, vcc_lo, s4, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s5, v10, vcc_lo
	v_add_co_u32 v11, vcc_lo, s4, v7
	v_add_co_ci_u32_e64 v12, null, s5, v8, vcc_lo
	s_clause 0x2
	global_load_b32 v4, v[9:10], off
	global_load_b32 v7, v[5:6], off
	global_load_b32 v5, v[11:12], off
                                        ; implicit-def: $vgpr6
	s_waitcnt vmcnt(2)
	v_cmp_ngt_f32_e64 s0, 0x3f200000, |v4|
	s_and_saveexec_b32 s1, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB12_3
; %bb.2:
	v_add_f32_e64 v0, |v4|, |v4|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v2, 0x3fb8aa3b, v0
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v0
	v_rndne_f32_e32 v6, v2
	v_fma_f32 v8, 0x3fb8aa3b, v0, -v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v2, v2, v6
	v_fmamk_f32 v8, v0, 0x32a5705f, v8
	v_cvt_i32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v2, v2, v8
	v_exp_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v2, v2, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v0
	v_cndmask_b32_e32 v0, 0x7f800000, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v0, 1.0, v0
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_fma_f32 v6, v0, -2.0, 1.0
.LBB12_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB12_5
; %bb.4:
	v_mul_f32_e32 v0, v4, v4
	s_mov_b32 s1, 0xbbbac73d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fmaak_f32 v2, s1, v0, 0x3ca908c9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v2, v0, v2, 0xbd5c1c4e
	v_fmaak_f32 v2, v0, v2, 0x3e088382
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v2, v0, v2, 0xbeaaaa99
	v_mul_f32_e64 v2, |v4|, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f32 v6, v0, v2, |v4|
.LBB12_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_bfi_b32 v4, 0x7fffffff, v6, v4
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v11, 0xbfb8aa3b, v5
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	v_add_nc_u32_e32 v2, s9, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_rndne_f32_e32 v14, v11
	v_fma_f32 v15, 0xbfb8aa3b, v5, -v11
	v_ashrrev_i32_e32 v3, 31, v2
	v_add_co_u32 v8, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v9, null, s7, v1, vcc_lo
	v_lshlrev_b64 v[2:3], 2, v[2:3]
	v_sub_f32_e32 v11, v11, v14
	v_fmac_f32_e32 v15, 0xb2a5705f, v5
	global_load_b32 v10, v[8:9], off
	v_add_co_u32 v2, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	v_add_f32_e32 v11, v11, v15
	v_cmp_nlt_f32_e32 vcc_lo, 0x42ce8ed0, v7
	global_load_b32 v2, v[2:3], off
	v_mul_f32_e32 v3, 0xbfb8aa3b, v7
	v_exp_f32_e32 v11, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rndne_f32_e32 v12, v3
	v_fma_f32 v13, 0xbfb8aa3b, v7, -v3
	v_sub_f32_e32 v3, v3, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v13, 0xb2a5705f, v7
	v_cvt_i32_f32_e32 v12, v12
	v_add_f32_e32 v3, v3, v13
	v_cvt_i32_f32_e32 v13, v14
	s_delay_alu instid0(VALU_DEP_2)
	v_exp_f32_e32 v3, v3
	s_delay_alu instid0(TRANS32_DEP_2) | instid1(VALU_DEP_1)
	v_ldexp_f32 v11, v11, v13
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v3, v3, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42ce8ed0, v5
	v_cndmask_b32_e32 v11, 0, v11, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2b17218, v7
	v_cndmask_b32_e32 v3, 0x7f800000, v3, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2b17218, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v3, 1.0, v3
	v_cndmask_b32_e32 v5, 0x7f800000, v11, vcc_lo
	v_div_scale_f32 v7, null, v3, v3, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v5, 1.0, v5
	v_rcp_f32_e32 v12, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f32 v11, null, v5, v5, 1.0
	v_rcp_f32_e32 v13, v11
	s_waitcnt_depctr 0xfff
	v_fma_f32 v14, -v7, v12, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v12, v14, v12
	v_div_scale_f32 v14, vcc_lo, 1.0, v3, 1.0
	v_fma_f32 v15, -v11, v13, 1.0
	v_dual_mul_f32 v16, v14, v12 :: v_dual_fmac_f32 v13, v15, v13
	v_div_scale_f32 v15, s0, 1.0, v5, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v18, -v7, v16, v14
	v_dual_mul_f32 v17, v15, v13 :: v_dual_fmac_f32 v16, v18, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v19, -v11, v17, v15
	v_fma_f32 v7, -v7, v16, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v17, v19, v13
	v_div_fmas_f32 v7, v7, v12, v16
	s_mov_b32 vcc_lo, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v11, -v11, v17, v15
	v_div_fixup_f32 v3, v7, v3, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v11, v11, v13, v17
	v_div_fixup_f32 v5, v11, v5, 1.0
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v3, v3, v10
	v_fmac_f32_e32 v3, v5, v4
                                        ; implicit-def: $vgpr4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_ngt_f32_e64 s0, 0x3f200000, |v3|
	global_store_b32 v[8:9], v3, off
	s_and_saveexec_b32 s1, s0
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB12_7
; %bb.6:
	v_add_f32_e64 v4, |v3|, |v3|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v5, 0x3fb8aa3b, v4
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v4
	v_rndne_f32_e32 v6, v5
	v_fma_f32 v7, 0x3fb8aa3b, v4, -v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v5, v5, v6
	v_fmamk_f32 v7, v4, 0x32a5705f, v7
	v_cvt_i32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v5, v5, v7
	v_exp_f32_e32 v5, v5
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v5, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v4
	v_cndmask_b32_e32 v4, 0x7f800000, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v4, 1.0, v4
	v_rcp_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, v4, -2.0, 1.0
.LBB12_7:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB12_9
; %bb.8:
	v_mul_f32_e32 v4, v3, v3
	s_mov_b32 s1, 0xbbbac73d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fmaak_f32 v5, s1, v4, 0x3ca908c9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v5, v4, v5, 0xbd5c1c4e
	v_fmaak_f32 v5, v4, v5, 0x3e088382
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v5, v4, v5, 0xbeaaaa99
	v_mul_f32_e64 v5, |v3|, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f32 v4, v4, v5, |v3|
.LBB12_9:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_bfi_b32 v3, 0x7fffffff, v4, v3
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v5, 0xbfb8aa3b, v2
	v_cmp_nlt_f32_e32 vcc_lo, 0x42ce8ed0, v2
	v_rndne_f32_e32 v6, v5
	v_fma_f32 v7, 0xbfb8aa3b, v2, -v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v5, v5, v6
	v_fmamk_f32 v7, v2, 0xb2a5705f, v7
	v_cvt_i32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v5, v5, v7
	v_exp_f32_e32 v5, v5
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v5, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2b17218, v2
	v_cndmask_b32_e32 v2, 0x7f800000, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v2, 1.0, v2
	v_div_scale_f32 v5, null, v2, v2, 1.0
	v_div_scale_f32 v8, vcc_lo, 1.0, v2, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v6, v5
	s_waitcnt_depctr 0xfff
	v_fma_f32 v7, -v5, v6, 1.0
	v_fmac_f32_e32 v6, v7, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v7, v8, v6
	v_fma_f32 v9, -v5, v7, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v7, v9, v6
	v_fma_f32 v5, -v5, v7, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v5, v5, v6, v7
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v2, v5, v2, 1.0
	v_mul_f32_e32 v2, v2, v3
	global_store_b32 v[0:1], v2, off
.LBB12_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel lstm_cell_f32_kernel
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
		.amdhsa_next_free_vgpr 20
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
.Lfunc_end12:
	.size	lstm_cell_f32_kernel, .Lfunc_end12-lstm_cell_f32_kernel
                                        ; -- End function
	.set lstm_cell_f32_kernel.num_vgpr, 20
	.set lstm_cell_f32_kernel.num_agpr, 0
	.set lstm_cell_f32_kernel.numbered_sgpr, 10
	.set lstm_cell_f32_kernel.num_named_barrier, 0
	.set lstm_cell_f32_kernel.private_seg_size, 0
	.set lstm_cell_f32_kernel.uses_vcc, 1
	.set lstm_cell_f32_kernel.uses_flat_scratch, 0
	.set lstm_cell_f32_kernel.has_dyn_sized_stack, 0
	.set lstm_cell_f32_kernel.has_recursion, 0
	.set lstm_cell_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1700
; TotalNumSgprs: 12
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 20
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
	.protected	gru_cell_f32_kernel     ; -- Begin function gru_cell_f32_kernel
	.globl	gru_cell_f32_kernel
	.p2align	8
	.type	gru_cell_f32_kernel,@function
gru_cell_f32_kernel:                    ; @gru_cell_f32_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s9, s8
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB13_6
; %bb.1:
	s_abs_i32 s2, s9
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
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
	v_xor_b32_e32 v3, s9, v1
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
	v_mul_lo_u32 v0, v0, s9
	v_sub_nc_u32_e32 v2, v1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshl_add_u32 v2, v0, 2, v2
	v_add_nc_u32_e32 v3, s9, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s4, v3
	v_add_co_ci_u32_e64 v4, null, s5, v4, vcc_lo
	global_load_b32 v9, v[3:4], off
	v_lshl_add_u32 v3, s9, 1, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v5, s9, v3
	v_ashrrev_i32_e32 v4, 31, v3
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[7:8], 2, v[3:4]
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[4:5], 2, v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s4, v7
	v_add_co_ci_u32_e64 v7, null, s5, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s4, v4
	v_add_co_ci_u32_e64 v5, null, s5, v5, vcc_lo
	v_lshlrev_b64 v[2:3], 2, v[2:3]
	s_clause 0x1
	global_load_b32 v0, v[6:7], off
	global_load_b32 v4, v[4:5], off
	v_add_co_u32 v2, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	global_load_b32 v3, v[2:3], off
	s_waitcnt vmcnt(3)
	v_mul_f32_e32 v2, 0xbfb8aa3b, v9
	v_cmp_nlt_f32_e32 vcc_lo, 0x42ce8ed0, v9
	v_fma_f32 v5, 0xbfb8aa3b, v9, -v2
	v_rndne_f32_e32 v6, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmamk_f32 v5, v9, 0xb2a5705f, v5 :: v_dual_sub_f32 v2, v2, v6
	v_add_f32_e32 v2, v2, v5
	v_cvt_i32_f32_e32 v5, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v2, v2, v5
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2b17218, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, 0x7f800000, v2, vcc_lo
	v_add_f32_e32 v2, 1.0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f32 v5, null, v2, v2, 1.0
	v_rcp_f32_e32 v6, v5
	s_waitcnt_depctr 0xfff
	v_fma_f32 v7, -v5, v6, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v7, v6
	v_div_scale_f32 v7, vcc_lo, 1.0, v2, 1.0
	v_mul_f32_e32 v8, v7, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v9, -v5, v8, v7
	v_fmac_f32_e32 v8, v9, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v5, -v5, v8, v7
	v_div_fmas_f32 v5, v5, v6, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v2, v5, v2, 1.0
	s_waitcnt vmcnt(1)
	v_fmac_f32_e32 v0, v4, v2
                                        ; implicit-def: $vgpr4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_ngt_f32_e64 s2, 0x3f200000, |v0|
	s_and_saveexec_b32 s3, s2
	s_xor_b32 s2, exec_lo, s3
	s_cbranch_execz .LBB13_3
; %bb.2:
	v_add_f32_e64 v2, |v0|, |v0|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v4, 0x3fb8aa3b, v2
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v2
	v_rndne_f32_e32 v5, v4
	v_fma_f32 v6, 0x3fb8aa3b, v2, -v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v4, v4, v5
	v_fmamk_f32 v6, v2, 0x32a5705f, v6
	v_cvt_i32_f32_e32 v5, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v4, v4, v6
	v_exp_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v4, v4, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v2
	v_cndmask_b32_e32 v2, 0x7f800000, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v2, 1.0, v2
	v_rcp_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, v2, -2.0, 1.0
.LBB13_3:
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execz .LBB13_5
; %bb.4:
	v_mul_f32_e32 v2, v0, v0
	s_mov_b32 s3, 0xbbbac73d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fmaak_f32 v4, s3, v2, 0x3ca908c9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v4, v2, v4, 0xbd5c1c4e
	v_fmaak_f32 v4, v2, v4, 0x3e088382
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v4, v2, v4, 0xbeaaaa99
	v_mul_f32_e64 v4, |v0|, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f32 v4, v2, v4, |v0|
.LBB13_5:
	s_or_b32 exec_lo, exec_lo, s2
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_bfi_b32 v0, 0x7fffffff, v4, v0
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s6, v1
	v_add_co_ci_u32_e64 v6, null, s7, v2, vcc_lo
	s_waitcnt vmcnt(0)
	v_cmp_nlt_f32_e32 vcc_lo, 0x42ce8ed0, v3
	global_load_b32 v5, v[5:6], off
	v_mul_f32_e32 v6, 0xbfb8aa3b, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rndne_f32_e32 v7, v6
	v_fma_f32 v8, 0xbfb8aa3b, v3, -v6
	v_sub_f32_e32 v6, v6, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmamk_f32 v8, v3, 0xb2a5705f, v8
	v_cvt_i32_f32_e32 v7, v7
	v_add_f32_e32 v6, v6, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v6, v6
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v6, v6, v7
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2b17218, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, 0x7f800000, v6, vcc_lo
	v_add_f32_e32 v3, 1.0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f32 v6, null, v3, v3, 1.0
	v_div_scale_f32 v9, vcc_lo, 1.0, v3, 1.0
	v_rcp_f32_e32 v7, v6
	s_waitcnt_depctr 0xfff
	v_fma_f32 v8, -v6, v7, 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v7, v8, v7
	v_mul_f32_e32 v8, v9, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v10, -v6, v8, v9
	v_fmac_f32_e32 v8, v10, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v6, -v6, v8, v9
	v_div_fmas_f32 v6, v6, v7, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f32 v3, v6, v3, 1.0
	s_waitcnt vmcnt(0)
	v_dual_sub_f32 v4, 1.0, v3 :: v_dual_mul_f32 v3, v3, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v3, v4, v0
	v_add_co_u32 v0, vcc_lo, s0, v1
	v_add_co_ci_u32_e64 v1, null, s1, v2, vcc_lo
	global_store_b32 v[0:1], v3, off
.LBB13_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gru_cell_f32_kernel
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
		.amdhsa_inst_pref_size 10
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end13:
	.size	gru_cell_f32_kernel, .Lfunc_end13-gru_cell_f32_kernel
                                        ; -- End function
	.set gru_cell_f32_kernel.num_vgpr, 11
	.set gru_cell_f32_kernel.num_agpr, 0
	.set gru_cell_f32_kernel.numbered_sgpr, 10
	.set gru_cell_f32_kernel.num_named_barrier, 0
	.set gru_cell_f32_kernel.private_seg_size, 0
	.set gru_cell_f32_kernel.uses_vcc, 1
	.set gru_cell_f32_kernel.uses_flat_scratch, 0
	.set gru_cell_f32_kernel.has_dyn_sized_stack, 0
	.set gru_cell_f32_kernel.has_recursion, 0
	.set gru_cell_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1240
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
	.protected	relu_f16_kernel         ; -- Begin function relu_f16_kernel
	.globl	relu_f16_kernel
	.p2align	8
	.type	relu_f16_kernel,@function
relu_f16_kernel:                        ; @relu_f16_kernel
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
	s_cbranch_execz .LBB14_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 1, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s1, v2, vcc_lo
	v_add_co_u32 v1, vcc_lo, s2, v1
	v_add_co_ci_u32_e64 v2, null, s3, v2, vcc_lo
	global_load_d16_b16 v0, v[3:4], off
	s_waitcnt vmcnt(0)
	v_max_f16_e32 v0.l, v0.l, v0.l
	s_delay_alu instid0(VALU_DEP_1)
	v_max_f16_e32 v0.l, 0, v0.l
	global_store_b16 v[1:2], v0, off
.LBB14_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel relu_f16_kernel
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
.Lfunc_end14:
	.size	relu_f16_kernel, .Lfunc_end14-relu_f16_kernel
                                        ; -- End function
	.set relu_f16_kernel.num_vgpr, 5
	.set relu_f16_kernel.num_agpr, 0
	.set relu_f16_kernel.numbered_sgpr, 5
	.set relu_f16_kernel.num_named_barrier, 0
	.set relu_f16_kernel.private_seg_size, 0
	.set relu_f16_kernel.uses_vcc, 1
	.set relu_f16_kernel.uses_flat_scratch, 0
	.set relu_f16_kernel.has_dyn_sized_stack, 0
	.set relu_f16_kernel.has_recursion, 0
	.set relu_f16_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 160
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
	.protected	gelu_f16_kernel         ; -- Begin function gelu_f16_kernel
	.globl	gelu_f16_kernel
	.p2align	8
	.type	gelu_f16_kernel,@function
gelu_f16_kernel:                        ; @gelu_f16_kernel
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
	s_cbranch_execz .LBB15_6
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 1, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_d16_b16 v3, v[2:3], off
	s_waitcnt vmcnt(0)
	v_cvt_f32_f16_e32 v2, v3.l
	v_mul_f32_e32 v4, 0x3d372713, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, v4, v2
	v_fma_mix_f32 v3, v4, v3, v3 op_sel_hi:[0,1,1]
                                        ; implicit-def: $vgpr4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v3, 0x3f4c422a, v3
	v_cmp_ngt_f32_e64 s0, 0x3f200000, |v3|
	s_and_saveexec_b32 s1, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB15_3
; %bb.2:
	v_add_f32_e64 v4, |v3|, |v3|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v5, 0x3fb8aa3b, v4
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v4
	v_rndne_f32_e32 v6, v5
	v_fma_f32 v7, 0x3fb8aa3b, v4, -v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v5, v5, v6
	v_fmamk_f32 v7, v4, 0x32a5705f, v7
	v_cvt_i32_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v5, v5, v7
	v_exp_f32_e32 v5, v5
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v5, v5, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v4
	v_cndmask_b32_e32 v4, 0x7f800000, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v4, 1.0, v4
	v_rcp_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v4, v4, -2.0, 1.0
.LBB15_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB15_5
; %bb.4:
	v_mul_f32_e32 v4, v3, v3
	s_mov_b32 s1, 0xbbbac73d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fmaak_f32 v5, s1, v4, 0x3ca908c9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v5, v4, v5, 0xbd5c1c4e
	v_fmaak_f32 v5, v4, v5, 0x3e088382
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmaak_f32 v5, v4, v5, 0xbeaaaa99
	v_mul_f32_e64 v5, |v3|, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f32 v4, v4, v5, |v3|
.LBB15_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_bfi_b32 v3, 0x7fffffff, v4, v3
	v_mul_f32_e32 v2, 0.5, v2
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, 1.0, v3
	v_fma_mixlo_f16 v2, v2, v3, 0
	global_store_b16 v[0:1], v2, off
.LBB15_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gelu_f16_kernel
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
.Lfunc_end15:
	.size	gelu_f16_kernel, .Lfunc_end15-gelu_f16_kernel
                                        ; -- End function
	.set gelu_f16_kernel.num_vgpr, 8
	.set gelu_f16_kernel.num_agpr, 0
	.set gelu_f16_kernel.numbered_sgpr, 5
	.set gelu_f16_kernel.num_named_barrier, 0
	.set gelu_f16_kernel.private_seg_size, 0
	.set gelu_f16_kernel.uses_vcc, 1
	.set gelu_f16_kernel.uses_flat_scratch, 0
	.set gelu_f16_kernel.has_dyn_sized_stack, 0
	.set gelu_f16_kernel.has_recursion, 0
	.set gelu_f16_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 476
; TotalNumSgprs: 7
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	add_f16_kernel          ; -- Begin function add_f16_kernel
	.globl	add_f16_kernel
	.p2align	8
	.type	add_f16_kernel,@function
add_f16_kernel:                         ; @add_f16_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB16_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 1, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s4, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s5, v2, vcc_lo
	v_add_co_u32 v5, vcc_lo, s6, v1
	v_add_co_ci_u32_e64 v6, null, s7, v2, vcc_lo
	global_load_d16_b16 v0, v[3:4], off
	global_load_d16_hi_b16 v0, v[5:6], off
	v_add_co_u32 v1, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s1, v2, vcc_lo
	s_waitcnt vmcnt(0)
	v_add_f16_e32 v0.l, v0.l, v0.h
	global_store_b16 v[1:2], v0, off
.LBB16_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel add_f16_kernel
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
.Lfunc_end16:
	.size	add_f16_kernel, .Lfunc_end16-add_f16_kernel
                                        ; -- End function
	.set add_f16_kernel.num_vgpr, 7
	.set add_f16_kernel.num_agpr, 0
	.set add_f16_kernel.numbered_sgpr, 8
	.set add_f16_kernel.num_named_barrier, 0
	.set add_f16_kernel.private_seg_size, 0
	.set add_f16_kernel.uses_vcc, 1
	.set add_f16_kernel.uses_flat_scratch, 0
	.set add_f16_kernel.has_dyn_sized_stack, 0
	.set add_f16_kernel.has_recursion, 0
	.set add_f16_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 188
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
	.protected	mul_f16_kernel          ; -- Begin function mul_f16_kernel
	.globl	mul_f16_kernel
	.p2align	8
	.type	mul_f16_kernel,@function
mul_f16_kernel:                         ; @mul_f16_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB17_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 1, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s4, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s5, v2, vcc_lo
	v_add_co_u32 v5, vcc_lo, s6, v1
	v_add_co_ci_u32_e64 v6, null, s7, v2, vcc_lo
	global_load_d16_b16 v0, v[3:4], off
	global_load_d16_hi_b16 v0, v[5:6], off
	v_add_co_u32 v1, vcc_lo, s0, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s1, v2, vcc_lo
	s_waitcnt vmcnt(0)
	v_mul_f16_e32 v0.l, v0.l, v0.h
	global_store_b16 v[1:2], v0, off
.LBB17_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mul_f16_kernel
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
.Lfunc_end17:
	.size	mul_f16_kernel, .Lfunc_end17-mul_f16_kernel
                                        ; -- End function
	.set mul_f16_kernel.num_vgpr, 7
	.set mul_f16_kernel.num_agpr, 0
	.set mul_f16_kernel.numbered_sgpr, 8
	.set mul_f16_kernel.num_named_barrier, 0
	.set mul_f16_kernel.private_seg_size, 0
	.set mul_f16_kernel.uses_vcc, 1
	.set mul_f16_kernel.uses_flat_scratch, 0
	.set mul_f16_kernel.has_dyn_sized_stack, 0
	.set mul_f16_kernel.has_recursion, 0
	.set mul_f16_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 188
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
	.protected	sgd_update_f32_kernel   ; -- Begin function sgd_update_f32_kernel
	.globl	sgd_update_f32_kernel
	.p2align	8
	.type	sgd_update_f32_kernel,@function
sgd_update_f32_kernel:                  ; @sgd_update_f32_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB18_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	global_load_b32 v3, v[0:1], off
	s_load_b32 s0, s[6:7], 0x0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f32 v2, -s0, v2, v3
	global_store_b32 v[0:1], v2, off
.LBB18_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel sgd_update_f32_kernel
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
.Lfunc_end18:
	.size	sgd_update_f32_kernel, .Lfunc_end18-sgd_update_f32_kernel
                                        ; -- End function
	.set sgd_update_f32_kernel.num_vgpr, 4
	.set sgd_update_f32_kernel.num_agpr, 0
	.set sgd_update_f32_kernel.numbered_sgpr, 8
	.set sgd_update_f32_kernel.num_named_barrier, 0
	.set sgd_update_f32_kernel.private_seg_size, 0
	.set sgd_update_f32_kernel.uses_vcc, 1
	.set sgd_update_f32_kernel.uses_flat_scratch, 0
	.set sgd_update_f32_kernel.has_dyn_sized_stack, 0
	.set sgd_update_f32_kernel.has_recursion, 0
	.set sgd_update_f32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 184
; TotalNumSgprs: 10
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_78f7236bb5fcf443,@object ; @__hip_cuid_78f7236bb5fcf443
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_78f7236bb5fcf443
__hip_cuid_78f7236bb5fcf443:
	.byte	0                               ; 0x0
	.size	__hip_cuid_78f7236bb5fcf443, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_78f7236bb5fcf443
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
    .name:           relu_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         relu_f32_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           relu_backward_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         relu_backward_f32_kernel.kd
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
    .name:           gelu_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         gelu_f32_kernel.kd
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
    .name:           gelu_backward_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         gelu_backward_f32_kernel.kd
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
    .name:           bias_add_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         bias_add_f32_kernel.kd
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
    .name:           repeat_rows_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         repeat_rows_f32_kernel.kd
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
    .group_segment_fixed_size: 128
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           layernorm_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         layernorm_f32_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     16
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
      - .offset:         48
        .size:           4
        .value_kind:     by_value
      - .offset:         52
        .size:           4
        .value_kind:     by_value
      - .address_space:  global
        .offset:         56
        .size:           8
        .value_kind:     global_buffer
      - .offset:         64
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         68
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         76
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         78
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         80
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         82
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         84
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         86
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         128
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 384
    .kernarg_segment_align: 8
    .kernarg_segment_size: 320
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           layernorm_backward_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     25
    .sgpr_spill_count: 0
    .symbol:         layernorm_backward_f32_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     24
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
    .name:           avg_pool_2d_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     20
    .sgpr_spill_count: 0
    .symbol:         avg_pool_2d_f32_kernel.kd
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
    .name:           avg_pool_2d_backward_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         avg_pool_2d_backward_f32_kernel.kd
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
      - .offset:         64
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         68
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         76
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         78
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         80
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         82
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         84
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         86
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         104
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         128
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 320
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           max_pool_2d_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     20
    .sgpr_spill_count: 0
    .symbol:         max_pool_2d_f32_kernel.kd
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
    .name:           max_pool_2d_backward_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         max_pool_2d_backward_f32_kernel.kd
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
    .name:           lstm_cell_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         lstm_cell_f32_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     20
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
    .name:           gru_cell_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         gru_cell_f32_kernel.kd
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
    .name:           relu_f16_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         relu_f16_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 280
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           gelu_f16_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         gelu_f16_kernel.kd
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
    .name:           add_f16_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         add_f16_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           mul_f16_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         mul_f16_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           sgd_update_f32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         sgd_update_f32_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     4
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
