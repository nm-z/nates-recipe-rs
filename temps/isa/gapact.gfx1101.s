	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	gapact_elu_backward_kernel ; -- Begin function gapact_elu_backward_kernel
	.globl	gapact_elu_backward_kernel
	.p2align	8
	.type	gapact_elu_backward_kernel,@function
gapact_elu_backward_kernel:             ; @gapact_elu_backward_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB0_4
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0x3ff00000
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	global_load_b64 v[4:5], v[2:3], off
	v_add_co_u32 v2, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_mov_b32 s4, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(1)
	v_cmpx_nlt_f64_e32 0, v[4:5]
	s_cbranch_execz .LBB0_3
; %bb.2:
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0x3ff71547
	s_mov_b32 s8, 0x6a5dcb37
	v_mul_f64 v[6:7], v[4:5], s[6:7]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0xbfe62e42
	s_mov_b32 s9, 0x3e5ade15
	v_cmp_ngt_f64_e32 vcc_lo, 0xc090cc00, v[4:5]
	s_load_b64 s[0:1], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[6:7], v[4:5]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0xbc7abc9e
	v_cvt_i32_f64_e32 v12, v[6:7]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], s[6:7], v[8:9]
	s_mov_b32 s6, 0xfca7ab0c
	s_mov_b32 s7, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], s[8:9], s[6:7]
	s_mov_b32 s6, 0x623fde64
	s_mov_b32 s7, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x7c89e6b0
	s_mov_b32 s7, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x14761f6e
	s_mov_b32 s7, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x1852b7b0
	s_mov_b32 s7, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x11122322
	s_mov_b32 s7, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x555502a1
	s_mov_b32 s7, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x55555511
	s_mov_b32 s7, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 11
	s_mov_b32 s7, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], 1.0
	v_fma_f64 v[6:7], v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v12
	v_dual_cndmask_b32 v5, 0, v7 :: v_dual_cndmask_b32 v4, 0, v6
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[6:7], s[0:1], v[4:5]
.LBB0_3:
	s_or_b32 exec_lo, exec_lo, s4
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[2:3], v[2:3], v[6:7]
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB0_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_elu_backward_kernel
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
	.size	gapact_elu_backward_kernel, .Lfunc_end0-gapact_elu_backward_kernel
                                        ; -- End function
	.set gapact_elu_backward_kernel.num_vgpr, 13
	.set gapact_elu_backward_kernel.num_agpr, 0
	.set gapact_elu_backward_kernel.numbered_sgpr, 10
	.set gapact_elu_backward_kernel.num_named_barrier, 0
	.set gapact_elu_backward_kernel.private_seg_size, 0
	.set gapact_elu_backward_kernel.uses_vcc, 1
	.set gapact_elu_backward_kernel.uses_flat_scratch, 0
	.set gapact_elu_backward_kernel.has_dyn_sized_stack, 0
	.set gapact_elu_backward_kernel.has_recursion, 0
	.set gapact_elu_backward_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 664
; TotalNumSgprs: 12
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 12
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
	.protected	gapact_selu_backward_kernel ; -- Begin function gapact_selu_backward_kernel
	.globl	gapact_selu_backward_kernel
	.p2align	8
	.type	gapact_selu_backward_kernel,@function
gapact_selu_backward_kernel:            ; @gapact_selu_backward_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b32 s4, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_4
; %bb.1:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[4:5], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b128 s[0:3], s[0:1], 0x20
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0x3ff00000
	s_mov_b32 s6, exec_lo
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s10, v0
	v_add_co_ci_u32_e64 v3, null, s11, v1, vcc_lo
	s_load_b64 s[2:3], s[2:3], 0x0
	global_load_b64 v[4:5], v[2:3], off
	v_add_co_u32 v2, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s9, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(1)
	v_cmpx_nlt_f64_e32 0, v[4:5]
	s_cbranch_execz .LBB1_3
; %bb.2:
	s_mov_b32 s8, 0x652b82fe
	s_mov_b32 s9, 0x3ff71547
	s_mov_b32 s10, 0x6a5dcb37
	v_mul_f64 v[6:7], v[4:5], s[8:9]
	s_mov_b32 s8, 0xfefa39ef
	s_mov_b32 s9, 0xbfe62e42
	s_mov_b32 s11, 0x3e5ade15
	v_cmp_ngt_f64_e32 vcc_lo, 0xc090cc00, v[4:5]
	s_load_b64 s[0:1], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[8:9], v[4:5]
	s_mov_b32 s8, 0x3b39803f
	s_mov_b32 s9, 0xbc7abc9e
	v_cvt_i32_f64_e32 v12, v[6:7]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], s[8:9], v[8:9]
	s_mov_b32 s8, 0xfca7ab0c
	s_mov_b32 s9, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], s[10:11], s[8:9]
	s_mov_b32 s8, 0x623fde64
	s_mov_b32 s9, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0x7c89e6b0
	s_mov_b32 s9, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0x14761f6e
	s_mov_b32 s9, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0x1852b7b0
	s_mov_b32 s9, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0x11122322
	s_mov_b32 s9, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0x555502a1
	s_mov_b32 s9, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 0x55555511
	s_mov_b32 s9, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_mov_b32 s8, 11
	s_mov_b32 s9, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], 1.0
	v_fma_f64 v[6:7], v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v12
	v_dual_cndmask_b32 v5, 0, v7 :: v_dual_cndmask_b32 v4, 0, v6
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[6:7], s[0:1], v[4:5]
.LBB1_3:
	s_or_b32 exec_lo, exec_lo, s6
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_mul_f64 v[2:3], s[2:3], v[2:3]
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	v_mul_f64 v[2:3], v[2:3], v[6:7]
	global_store_b64 v[0:1], v[2:3], off
.LBB1_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_selu_backward_kernel
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
	.size	gapact_selu_backward_kernel, .Lfunc_end1-gapact_selu_backward_kernel
                                        ; -- End function
	.set gapact_selu_backward_kernel.num_vgpr, 13
	.set gapact_selu_backward_kernel.num_agpr, 0
	.set gapact_selu_backward_kernel.numbered_sgpr, 12
	.set gapact_selu_backward_kernel.num_named_barrier, 0
	.set gapact_selu_backward_kernel.private_seg_size, 0
	.set gapact_selu_backward_kernel.uses_vcc, 1
	.set gapact_selu_backward_kernel.uses_flat_scratch, 0
	.set gapact_selu_backward_kernel.has_dyn_sized_stack, 0
	.set gapact_selu_backward_kernel.has_recursion, 0
	.set gapact_selu_backward_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 676
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
	.protected	gapact_mish_kernel      ; -- Begin function gapact_mish_kernel
	.globl	gapact_mish_kernel
	.p2align	8
	.type	gapact_mish_kernel,@function
gapact_mish_kernel:                     ; @gapact_mish_kernel
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
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s19, 0xbff71547
	s_mov_b32 s18, 0x652b82fe
	s_mov_b32 s23, 0xbfe62e42
	s_mov_b32 s22, 0xfefa39ef
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s4, 0xfca7ab0c
	s_mov_b32 s6, 0x6a5dcb37
	s_mov_b32 s5, 0x3e928af3
	s_mov_b32 s7, 0x3e5ade15
	s_mov_b32 s8, 0x623fde64
	s_mov_b32 s9, 0x3ec71dee
	s_mov_b32 s10, 0x7c89e6b0
	s_mov_b32 s11, 0x3efa0199
	s_mov_b32 s12, 0x14761f6e
	s_mov_b32 s13, 0x3f2a01a0
	s_mov_b32 s16, 0x1852b7b0
	s_mov_b32 s17, 0x3f56c16c
	s_mov_b32 s14, 0x11122322
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0xbc7abc9e
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s15, 0x3f811111
	global_load_b64 v[2:3], v[2:3], off
	s_mov_b32 s20, 0x555502a1
	s_mov_b32 s21, 0x3fa55555
	s_mov_b32 s24, 0x55555511
	s_mov_b32 s25, 0x3fc55555
	s_mov_b32 s26, 11
	s_mov_b32 s27, 0x3fe00000
	s_mov_b32 s29, 0x3fe55555
	s_mov_b32 s28, 0x55555555
	s_mov_b32 s30, 0x6b47b09a
	s_mov_b32 s34, 0xbf559e2b
	s_mov_b32 s31, 0x3fc38538
	s_mov_b32 s35, 0x3fc3ab76
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], |v[2:3]|, s[18:19]
	v_cmp_nlt_f64_e64 vcc_lo, 0x4090cc00, |v[2:3]|
	s_mov_b32 s19, 0x3ff71547
	v_rndne_f64_e32 v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[22:23], -|v[2:3]|
	v_cvt_i32_f64_e32 v10, v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[6:7]
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[6:7], s[4:5]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[16:17]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[20:21]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[26:27]
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	v_ldexp_f64 v[4:5], v[4:5], v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
	v_add_f64 v[6:7], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	v_add_f64 v[10:11], v[6:7], -1.0
	v_cmp_gt_f64_e32 vcc_lo, s[28:29], v[8:9]
	s_mov_b32 s28, 0x55555780
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[10:11], -v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[10:11]
	v_subrev_co_ci_u32_e64 v28, null, 0, v12, vcc_lo
	v_add_f64 v[8:9], v[8:9], 1.0
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v14, 0, v28
	v_ldexp_f64 v[6:7], v[6:7], v14
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], v[8:9]
	v_add_f64 v[12:13], v[6:7], 1.0
	v_add_f64 v[18:19], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[8:9], v[8:9], v14
	v_add_f64 v[10:11], v[12:13], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[18:19], 1.0
	v_add_f64 v[10:11], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	v_add_f64 v[10:11], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[14:15], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[18:19], v[6:7]
	v_rcp_f64_e32 v[16:17], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[22:23], -v[14:15], v[16:17], 1.0
	v_add_f64 v[6:7], v[6:7], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[22:23], v[16:17], v[16:17]
	v_fma_f64 v[8:9], -v[14:15], v[16:17], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[16:17]
	v_mul_f64 v[16:17], v[20:21], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[22:23], v[14:15], v[16:17]
	v_fma_f64 v[12:13], v[16:17], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[10:11], v[12:13]
	v_add_f64 v[24:25], v[22:23], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[20:21], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[22:23]
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_add_f64 v[20:21], v[20:21], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[20:21]
	v_add_f64 v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[26:27], v[6:7]
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	v_add_f64 v[24:25], v[26:27], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[20:21], v[14:15], v[18:19]
	v_add_f64 v[6:7], v[6:7], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[18:19], v[14:15], -v[20:21]
	v_fma_f64 v[10:11], v[18:19], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[20:21], v[10:11]
	v_add_f64 v[22:23], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	v_add_f64 v[10:11], v[20:21], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[10:11], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[22:23], v[6:7]
	v_add_f64 v[10:11], v[18:19], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], s[34:35], s[30:31]
	s_mov_b32 s30, 0xd7f4df2e
	s_mov_b32 s31, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[30:31]
	s_mov_b32 s30, 0x16291751
	s_mov_b32 s31, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[30:31]
	s_mov_b32 s30, 0x9b27acf1
	s_mov_b32 s31, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[30:31]
	s_mov_b32 s30, 0x998ef7b6
	s_mov_b32 s31, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[30:31]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[28:29]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s29, 0x3fe62e42
	s_mov_b32 s28, s22
	s_mov_b32 s22, 0xfefa3000
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v28
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[14:15], v[10:11]
	v_mul_f64 v[18:19], v[16:17], s[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[16:17], s[28:29], -v[18:19]
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[10:11], v[16:17], s[0:1], v[14:15]
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[4:5]
	v_cmp_ngt_f64_e64 s1, -1.0, v[4:5]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[18:19], v[10:11]
	s_and_b32 vcc_lo, vcc_lo, s0
	v_add_f64 v[14:15], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	v_add_f64 v[16:17], v[8:9], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	v_add_f64 v[20:21], v[16:17], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[22:23], v[16:17], -v[20:21]
	v_add_f64 v[12:13], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[10:11], v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], v[8:9]
	v_add_f64 v[12:13], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[14:15], v[8:9]
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[16:17], v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[18:19], -v[16:17]
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_max_f64 v[8:9], v[2:3], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_max_f64 v[8:9], v[8:9], 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[4:5]
	v_cndmask_b32_e64 v7, 0x7ff00000, v7, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0x7ff80000, v7, s1
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[8:9], v[6:7]
	v_mul_f64 v[6:7], |v[4:5]|, s[18:19]
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[4:5]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[22:23], |v[4:5]|
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[8:9], 0
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[8:9], v[8:9], 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[14:15], v[8:9]
	v_add_f64 v[16:17], v[12:13], v[10:11]
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], v[8:9]
	v_fma_f64 v[12:13], v[10:11], s[6:7], s[4:5]
	v_add_f64 v[14:15], v[16:17], -v[10:11]
	v_mul_f64 v[16:17], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[8:9]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[10:11], v[10:11], -v[16:17]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[8:9], v[8:9]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[18:19], v[14:15]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], v[14:15]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[26:27]
	v_mul_f64 v[20:21], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[18:19], v[12:13], -v[20:21]
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[20:21], v[12:13]
	v_add_f64 v[16:17], v[10:11], v[14:15]
	v_add_f64 v[18:19], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_cvt_i32_f64_e32 v18, v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	v_add_f64 v[8:9], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[14:15], v[10:11], -v[16:17]
	v_add_f64 v[16:17], v[12:13], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[6:7], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[6:7], v18
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_rcp_f64_e32 v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_ldexp_f64 v[6:7], v[6:7], v18
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[14:15], v[14:15]
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[10:11], -v[8:9]
	v_fma_f64 v[14:15], v[12:13], v[6:7], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[8:9], v[14:15]
	v_add_f64 v[18:19], -v[16:17], 1.0
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], -v[18:19], 1.0
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[20:21], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[18:19], v[8:9]
	v_mul_f64 v[16:17], v[12:13], v[14:15]
	v_add_f64 v[18:19], v[18:19], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[20:21], v[10:11], v[16:17]
	v_add_f64 v[8:9], v[8:9], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[16:17], v[10:11], -v[20:21]
	v_fma_f64 v[22:23], v[16:17], v[6:7], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[24:25], v[20:21], v[22:23]
	v_add_f64 v[26:27], v[14:15], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[26:27]
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[18:19], v[8:9]
	v_add_f64 v[18:19], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[26:27], v[8:9]
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	v_add_f64 v[8:9], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[14:15], v[8:9]
	v_add_f64 v[16:17], v[10:11], v[12:13]
	v_add_f64 v[14:15], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	v_add_f64 v[14:15], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[6:7], v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[18:19], v[14:15]
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], v[14:15]
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[22:23], v[18:19]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[18:19], v[22:23], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[12:13], v[22:23], v[22:23]
	v_add_f64 v[12:13], v[20:21], v[6:7]
	v_fma_f64 v[8:9], -v[18:19], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[10:11]
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[22:23], v[18:19], v[10:11]
	v_fma_f64 v[16:17], v[10:11], v[18:19], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], v[16:17]
	v_add_f64 v[16:17], v[22:23], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[12:13], -v[16:17]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	v_add_f64 v[24:25], v[12:13], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
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
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_bfi_b32 v5, 0x7fffffff, v6, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_mish_kernel
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
		.amdhsa_next_free_sgpr 36
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
		.amdhsa_inst_pref_size 26
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
	.size	gapact_mish_kernel, .Lfunc_end2-gapact_mish_kernel
                                        ; -- End function
	.set gapact_mish_kernel.num_vgpr, 29
	.set gapact_mish_kernel.num_agpr, 0
	.set gapact_mish_kernel.numbered_sgpr, 36
	.set gapact_mish_kernel.num_named_barrier, 0
	.set gapact_mish_kernel.private_seg_size, 0
	.set gapact_mish_kernel.uses_vcc, 1
	.set gapact_mish_kernel.uses_flat_scratch, 0
	.set gapact_mish_kernel.has_dyn_sized_stack, 0
	.set gapact_mish_kernel.has_recursion, 0
	.set gapact_mish_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3284
; TotalNumSgprs: 38
; NumVgprs: 29
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 38
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
	.text
	.protected	gapact_mish_backward_kernel ; -- Begin function gapact_mish_backward_kernel
	.globl	gapact_mish_backward_kernel
	.p2align	8
	.type	gapact_mish_backward_kernel,@function
gapact_mish_backward_kernel:            ; @gapact_mish_backward_kernel
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
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s8, 0xfefa39ef
	s_mov_b32 s9, 0xbfe62e42
	s_mov_b32 s10, 0x3b39803f
	s_mov_b32 s11, 0xbc7abc9e
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s12, 0xfca7ab0c
	s_mov_b32 s14, 0x6a5dcb37
	s_mov_b32 s13, 0x3e928af3
	s_mov_b32 s15, 0x3e5ade15
	s_mov_b32 s16, 0x623fde64
	s_mov_b32 s17, 0x3ec71dee
	s_mov_b32 s18, 0x7c89e6b0
	s_mov_b32 s19, 0x3efa0199
	s_mov_b32 s20, 0x14761f6e
	s_mov_b32 s21, 0x3f2a01a0
	s_mov_b32 s22, 0x1852b7b0
	s_mov_b32 s23, 0x3f56c16c
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0xbff71547
	s_mov_b32 s24, 0x11122322
	global_load_b64 v[2:3], v[2:3], off
	s_mov_b32 s25, 0x3f811111
	s_mov_b32 s26, 0x555502a1
	s_mov_b32 s27, 0x3fa55555
	s_mov_b32 s28, 0x55555511
	s_mov_b32 s29, 0x3fc55555
	s_mov_b32 s30, 11
	s_mov_b32 s31, 0x3fe00000
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s34, 0x6b47b09a
	s_mov_b32 s36, 0xbf559e2b
	s_mov_b32 s35, 0x3fc38538
	s_mov_b32 s37, 0x3fc3ab76
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], |v[2:3]|, s[6:7]
	v_cmp_nlt_f64_e64 vcc_lo, 0x4090cc00, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[8:9], -|v[2:3]|
	v_cvt_i32_f64_e32 v10, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[4:5], s[10:11], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[14:15], s[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[16:17]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[20:21]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[24:25]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[28:29]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[4:5], 1.0
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	v_add_f64 v[10:11], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[8:9]
	s_mov_b32 s0, 0x55555780
	v_add_f64 v[8:9], v[10:11], -v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[10:11]
	v_subrev_co_ci_u32_e64 v28, null, 0, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], 1.0
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[4:5]
	v_sub_nc_u32_e32 v14, 0, v28
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[6:7], v[6:7], v14
	v_add_f64 v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[6:7], 1.0
	v_add_f64 v[18:19], v[6:7], -1.0
	v_ldexp_f64 v[8:9], v[8:9], v14
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[12:13], -1.0
	v_add_f64 v[20:21], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[6:7], -v[10:11]
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], v[10:11]
	v_add_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[20:21], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[16:17], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[22:23], -v[14:15], v[16:17], 1.0
	v_add_f64 v[6:7], v[6:7], -v[18:19]
	v_fma_f64 v[16:17], v[22:23], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[20:21], v[8:9]
	v_mul_f64 v[22:23], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[14:15], -v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[24:25], v[22:23], v[12:13]
	v_add_f64 v[26:27], v[20:21], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[20:21], v[20:21], -v[24:25]
	v_add_f64 v[6:7], v[6:7], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], v[6:7]
	v_add_f64 v[12:13], v[26:27], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	v_add_f64 v[24:25], v[26:27], -v[12:13]
	v_mul_f64 v[20:21], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[24:25]
	v_fma_f64 v[14:15], v[18:19], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[18:19], v[10:11], v[14:15]
	v_add_f64 v[14:15], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[20:21], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[16:17], v[18:19]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[12:13], -v[16:17]
	v_add_f64 v[6:7], v[22:23], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[18:19], -v[10:11]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[8:9], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	v_fma_f64 v[14:15], v[10:11], s[36:37], s[34:35]
	s_mov_b32 s34, 0xd7f4df2e
	s_mov_b32 s35, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[34:35]
	s_mov_b32 s34, 0x16291751
	s_mov_b32 s35, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[34:35]
	s_mov_b32 s34, 0x9b27acf1
	s_mov_b32 s35, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[34:35]
	s_mov_b32 s34, 0x998ef7b6
	s_mov_b32 s35, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[0:1]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s1, 0x3fe62e42
	s_mov_b32 s0, s8
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v28
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[16:17], s[0:1]
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[0:1], -v[18:19]
	s_mov_b32 s1, 0x3c7abc9e
	s_mov_b32 s0, s10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[0:1], v[14:15]
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[4:5]
	v_cmp_ngt_f64_e64 s1, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[10:11]
	s_and_b32 vcc_lo, vcc_lo, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[6:7]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[8:9], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[16:17], -v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], -v[20:21]
	v_add_f64 v[12:13], v[14:15], -v[20:21]
	v_add_f64 v[14:15], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[14:15], -v[10:11]
	v_add_f64 v[8:9], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[18:19], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[12:13], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_max_f64 v[8:9], v[2:3], v[2:3]
	v_add_f64 v[6:7], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_f64 v[8:9], v[8:9], 0
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0x7ff00000, v7, s0
	s_mov_b32 s0, s6
	v_cndmask_b32_e64 v7, 0x7ff80000, v7, s1
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	v_add_f64 v[4:5], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], |v[4:5]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, s9
	v_rndne_f64_e32 v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[0:1], |v[4:5]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[8:9], 0
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[8:9], v[8:9], 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x40331000, |v[4:5]|
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[14:15], v[8:9]
	v_add_f64 v[16:17], v[12:13], v[10:11]
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], v[8:9]
	v_fma_f64 v[12:13], v[10:11], s[14:15], s[12:13]
	v_add_f64 v[14:15], v[16:17], -v[10:11]
	v_mul_f64 v[16:17], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[16:17]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[10:11], v[10:11], -v[16:17]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[8:9], v[8:9]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[18:19], v[14:15]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], v[14:15]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[30:31]
	v_mul_f64 v[20:21], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[18:19], v[12:13], -v[20:21]
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[20:21], v[12:13]
	v_add_f64 v[16:17], v[10:11], v[14:15]
	v_add_f64 v[18:19], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_cvt_i32_f64_e32 v18, v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	v_add_f64 v[8:9], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[14:15], v[10:11], -v[16:17]
	v_add_f64 v[16:17], v[12:13], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[6:7], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[6:7], v18
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_rcp_f64_e32 v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_ldexp_f64 v[6:7], v[6:7], v18
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[14:15], v[14:15]
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[10:11], -v[8:9]
	v_fma_f64 v[14:15], v[12:13], v[6:7], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[8:9], v[14:15]
	v_add_f64 v[18:19], -v[16:17], 1.0
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], -v[18:19], 1.0
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[20:21], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[18:19], v[8:9]
	v_mul_f64 v[16:17], v[12:13], v[14:15]
	v_add_f64 v[18:19], v[18:19], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[20:21], v[10:11], v[16:17]
	v_add_f64 v[8:9], v[8:9], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[16:17], v[10:11], -v[20:21]
	v_fma_f64 v[22:23], v[16:17], v[6:7], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[24:25], v[20:21], v[22:23]
	v_add_f64 v[26:27], v[14:15], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[26:27]
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[18:19], v[8:9]
	v_add_f64 v[18:19], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[26:27], v[8:9]
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	v_mul_f64 v[12:13], v[2:3], s[6:7]
	v_add_f64 v[8:9], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rndne_f64_e32 v[12:13], v[12:13]
	v_add_f64 v[16:17], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[12:13], s[8:9], -v[2:3]
	v_add_f64 v[18:19], v[10:11], v[16:17]
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	v_add_f64 v[26:27], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[18:19], -v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_fma_f64 v[14:15], v[12:13], s[10:11], v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[10:11], -v[26:27]
	v_add_f64 v[20:21], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[22:23], v[6:7], v[8:9]
	v_fma_f64 v[24:25], v[14:15], s[14:15], s[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[20:21], v[22:23], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], v[14:15], v[24:25], s[16:17]
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[18:19], v[20:21]
	v_fma_f64 v[22:23], v[14:15], v[22:23], s[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_rcp_f64_e32 v[28:29], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[14:15], v[22:23], s[20:21]
	v_add_f64 v[18:19], v[24:25], -v[18:19]
	v_fma_f64 v[22:23], v[14:15], v[22:23], s[22:23]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[24:25], v[28:29], 1.0
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[16:17], v[28:29], v[28:29]
	v_fma_f64 v[16:17], v[14:15], v[22:23], s[24:25]
	v_add_f64 v[22:23], v[26:27], v[6:7]
	v_fma_f64 v[8:9], -v[24:25], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[26:27]
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[14:15], v[16:17], s[28:29]
	v_mul_f64 v[16:17], v[22:23], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[14:15], v[10:11], s[30:31]
	v_mul_f64 v[28:29], v[24:25], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[14:15], v[10:11], 1.0
	v_fma_f64 v[20:21], v[16:17], v[24:25], -v[28:29]
	v_cvt_i32_f64_e32 v24, v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[14:15], v[10:11], 1.0
	v_fma_f64 v[12:13], v[16:17], v[18:19], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[10:11], v[10:11], v24
	v_add_f64 v[14:15], v[28:29], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], 1.0
	v_add_f64 v[18:19], v[22:23], -v[14:15]
	v_add_f64 v[24:25], v[14:15], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_scale_f64 v[20:21], null, v[10:11], v[10:11], 1.0
	v_add_f64 v[28:29], v[22:23], -v[18:19]
	v_add_f64 v[22:23], v[22:23], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_rcp_f64_e32 v[30:31], v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[28:29], -v[14:15]
	v_add_f64 v[6:7], v[6:7], -v[22:23]
	v_add_co_u32 v22, vcc_lo, s4, v0
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[24:25], -v[20:21], v[30:31], 1.0
	v_add_co_ci_u32_e64 v23, null, s5, v1, vcc_lo
	global_load_b64 v[22:23], v[22:23], off
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_fma_f64 v[14:15], v[30:31], v[24:25], v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_fma_f64 v[12:13], -v[20:21], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_div_scale_f64 v[18:19], vcc_lo, 1.0, v[10:11], 1.0
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_mul_f64 v[8:9], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[16:17], v[6:7]
	v_fma_f64 v[14:15], -v[20:21], v[8:9], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v16, 0x3ff00000, v7, s0
	v_cndmask_b32_e64 v17, 0, v6, s0
	v_cmp_gt_f64_e64 s0, 0x3e400000, |v[4:5]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[6:7], v[14:15], v[12:13], v[8:9]
	v_and_b32_e32 v8, 0x7fffffff, v5
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[2:3]
	v_cndmask_b32_e64 v8, v16, v8, s0
	v_cndmask_b32_e64 v4, v17, v4, s0
	v_div_fixup_f64 v[6:7], v[6:7], v[10:11], 1.0
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_bfi_b32 v5, 0x7fffffff, v8, v5
	v_fma_f64 v[8:9], -v[4:5], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v7, 0, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_cndmask_b32_e64 v7, 0x3ff00000, v7, s0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_mul_f64 v[2:3], v[2:3], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[6:7], v[2:3], v[4:5]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[22:23], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_mish_backward_kernel
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
		.amdhsa_next_free_vgpr 32
		.amdhsa_next_free_sgpr 38
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
	.text
.Lfunc_end3:
	.size	gapact_mish_backward_kernel, .Lfunc_end3-gapact_mish_backward_kernel
                                        ; -- End function
	.set gapact_mish_backward_kernel.num_vgpr, 32
	.set gapact_mish_backward_kernel.num_agpr, 0
	.set gapact_mish_backward_kernel.numbered_sgpr, 38
	.set gapact_mish_backward_kernel.num_named_barrier, 0
	.set gapact_mish_backward_kernel.private_seg_size, 0
	.set gapact_mish_backward_kernel.uses_vcc, 1
	.set gapact_mish_backward_kernel.uses_flat_scratch, 0
	.set gapact_mish_backward_kernel.has_dyn_sized_stack, 0
	.set gapact_mish_backward_kernel.has_recursion, 0
	.set gapact_mish_backward_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3688
; TotalNumSgprs: 40
; NumVgprs: 32
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 40
; NumVGPRsForWavesPerEU: 32
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
	.protected	gapact_softplus_backward_kernel ; -- Begin function gapact_softplus_backward_kernel
	.globl	gapact_softplus_backward_kernel
	.p2align	8
	.type	gapact_softplus_backward_kernel,@function
gapact_softplus_backward_kernel:        ; @gapact_softplus_backward_kernel
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
	s_cbranch_execz .LBB4_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	s_mov_b32 s6, 0x6a5dcb37
	s_mov_b32 s7, 0x3e5ade15
	v_add_co_u32 v12, vcc_lo, s4, v0
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_ci_u32_e64 v13, null, s5, v1, vcc_lo
	global_load_b64 v[12:13], v[12:13], off
	s_waitcnt vmcnt(1)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[0:1], -v[2:3]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	v_cvt_i32_f64_e32 v10, v[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[6:7]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[6:7], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_add_f64 v[4:5], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[14:15], vcc_lo, 1.0, v[4:5], 1.0
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_mul_f64 v[10:11], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[14:15]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[6:7], v[4:5], 1.0
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_cndmask_b32_e64 v3, 0x3ff00000, v5, s0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[2:3], v[12:13], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_softplus_backward_kernel
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
		.amdhsa_next_free_vgpr 16
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
	.size	gapact_softplus_backward_kernel, .Lfunc_end4-gapact_softplus_backward_kernel
                                        ; -- End function
	.set gapact_softplus_backward_kernel.num_vgpr, 16
	.set gapact_softplus_backward_kernel.num_agpr, 0
	.set gapact_softplus_backward_kernel.numbered_sgpr, 8
	.set gapact_softplus_backward_kernel.num_named_barrier, 0
	.set gapact_softplus_backward_kernel.private_seg_size, 0
	.set gapact_softplus_backward_kernel.uses_vcc, 1
	.set gapact_softplus_backward_kernel.uses_flat_scratch, 0
	.set gapact_softplus_backward_kernel.has_dyn_sized_stack, 0
	.set gapact_softplus_backward_kernel.has_recursion, 0
	.set gapact_softplus_backward_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 748
; TotalNumSgprs: 10
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.protected	gapact_hardswish_kernel ; -- Begin function gapact_hardswish_kernel
	.globl	gapact_hardswish_kernel
	.p2align	8
	.type	gapact_hardswish_kernel,@function
gapact_hardswish_kernel:                ; @gapact_hardswish_kernel
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
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], 0x40080000, v[2:3]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40180000, v[4:5]
	v_cmp_ngt_f64_e64 s0, 0, v[4:5]
	v_cndmask_b32_e32 v6, 0x40180000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0, v6, s0
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[4:5], null, 0x40180000, 0x40180000, v[2:3]
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_div_scale_f64 v[8:9], vcc_lo, v[2:3], 0x40180000, v[2:3]
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[10:11], v[8:9]
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[10:11]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_div_fixup_f64 v[2:3], v[4:5], 0x40180000, v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_hardswish_kernel
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
		.amdhsa_next_free_vgpr 12
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
	.size	gapact_hardswish_kernel, .Lfunc_end5-gapact_hardswish_kernel
                                        ; -- End function
	.set gapact_hardswish_kernel.num_vgpr, 12
	.set gapact_hardswish_kernel.num_agpr, 0
	.set gapact_hardswish_kernel.numbered_sgpr, 5
	.set gapact_hardswish_kernel.num_named_barrier, 0
	.set gapact_hardswish_kernel.private_seg_size, 0
	.set gapact_hardswish_kernel.uses_vcc, 1
	.set gapact_hardswish_kernel.uses_flat_scratch, 0
	.set gapact_hardswish_kernel.has_dyn_sized_stack, 0
	.set gapact_hardswish_kernel.has_recursion, 0
	.set gapact_hardswish_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 332
; TotalNumSgprs: 7
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 7
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
	.protected	gapact_hardswish_backward_kernel ; -- Begin function gapact_hardswish_backward_kernel
	.globl	gapact_hardswish_backward_kernel
	.p2align	8
	.type	gapact_hardswish_backward_kernel,@function
gapact_hardswish_backward_kernel:       ; @gapact_hardswish_backward_kernel
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
	s_cbranch_execz .LBB6_6
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	global_load_b64 v[4:5], v[2:3], off
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_waitcnt vmcnt(0)
	v_cmpx_nge_f64_e32 0xc0080000, v[4:5]
	s_cbranch_execz .LBB6_5
; %bb.2:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0x3ff00000
	s_mov_b32 s3, exec_lo
	v_cmpx_nle_f64_e32 0x40080000, v[4:5]
	s_cbranch_execz .LBB6_4
; %bb.3:
	v_fma_f64 v[2:3], v[4:5], 2.0, 0x40080000
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[4:5], null, 0x40180000, 0x40180000, v[2:3]
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_div_scale_f64 v[8:9], vcc_lo, v[2:3], 0x40180000, v[2:3]
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[10:11], v[8:9]
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[4:5], 0x40180000, v[2:3]
.LBB6_4:
	s_or_b32 exec_lo, exec_lo, s3
.LBB6_5:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v4, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v5, null, s5, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB6_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_hardswish_backward_kernel
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
		.amdhsa_next_free_vgpr 12
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
.Lfunc_end6:
	.size	gapact_hardswish_backward_kernel, .Lfunc_end6-gapact_hardswish_backward_kernel
                                        ; -- End function
	.set gapact_hardswish_backward_kernel.num_vgpr, 12
	.set gapact_hardswish_backward_kernel.num_agpr, 0
	.set gapact_hardswish_backward_kernel.numbered_sgpr, 8
	.set gapact_hardswish_backward_kernel.num_named_barrier, 0
	.set gapact_hardswish_backward_kernel.private_seg_size, 0
	.set gapact_hardswish_backward_kernel.uses_vcc, 1
	.set gapact_hardswish_backward_kernel.uses_flat_scratch, 0
	.set gapact_hardswish_backward_kernel.has_dyn_sized_stack, 0
	.set gapact_hardswish_backward_kernel.has_recursion, 0
	.set gapact_hardswish_backward_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 392
; TotalNumSgprs: 10
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.protected	gapact_swiglu_kernel    ; -- Begin function gapact_swiglu_kernel
	.globl	gapact_swiglu_kernel
	.p2align	8
	.type	gapact_swiglu_kernel,@function
gapact_swiglu_kernel:                   ; @gapact_swiglu_kernel
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
	s_cbranch_execz .LBB7_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	s_mov_b32 s6, 0x6a5dcb37
	s_mov_b32 s7, 0x3e5ade15
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[0:1], -v[2:3]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	v_cvt_i32_f64_e32 v10, v[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[6:7]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[6:7], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_add_f64 v[4:5], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_add_co_u32 v10, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v5, 0x3ff00000, v5, s0
	v_add_co_ci_u32_e64 v11, null, s5, v1, vcc_lo
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], v[2:3]
	global_load_b64 v[10:11], v[10:11], off
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[12:13], v[8:9]
	v_fma_f64 v[12:13], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[12:13], v[8:9]
	v_div_scale_f64 v[12:13], vcc_lo, v[2:3], v[4:5], v[2:3]
	v_mul_f64 v[14:15], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[14:15], v[12:13]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[14:15]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_div_fixup_f64 v[2:3], v[6:7], v[4:5], v[2:3]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], v[10:11], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB7_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_swiglu_kernel
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
		.amdhsa_next_free_vgpr 16
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
	.size	gapact_swiglu_kernel, .Lfunc_end7-gapact_swiglu_kernel
                                        ; -- End function
	.set gapact_swiglu_kernel.num_vgpr, 16
	.set gapact_swiglu_kernel.num_agpr, 0
	.set gapact_swiglu_kernel.numbered_sgpr, 8
	.set gapact_swiglu_kernel.num_named_barrier, 0
	.set gapact_swiglu_kernel.private_seg_size, 0
	.set gapact_swiglu_kernel.uses_vcc, 1
	.set gapact_swiglu_kernel.uses_flat_scratch, 0
	.set gapact_swiglu_kernel.has_dyn_sized_stack, 0
	.set gapact_swiglu_kernel.has_recursion, 0
	.set gapact_swiglu_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 752
; TotalNumSgprs: 10
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.protected	gapact_geglu_kernel     ; -- Begin function gapact_geglu_kernel
	.globl	gapact_geglu_kernel
	.p2align	8
	.type	gapact_geglu_kernel,@function
gapact_geglu_kernel:                    ; @gapact_geglu_kernel
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
	s_cbranch_execz .LBB8_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s0, 0x667f3bcd
	s_mov_b32 s1, 0x3fe6a09e
                                        ; implicit-def: $vgpr6_vgpr7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s1, exec_lo
	v_cmpx_nlt_f64_e64 |v[4:5]|, 1.0
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB8_3
; %bb.2:
	s_mov_b32 s6, 0x502a41cd
	s_mov_b32 s8, 0xc14b24be
	s_mov_b32 s7, 0xbcc145a3
	s_mov_b32 s9, 0x3c598d37
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], |v[4:5]|, s[8:9], s[6:7]
	s_mov_b32 s6, 0xd735f9ec
	s_mov_b32 s7, 0x3d162dee
	s_mov_b32 s8, 0x6a5dcb37
	s_mov_b32 s9, 0x3e5ade15
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x5552ca22
	s_mov_b32 s7, 0xbd61ffe5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x7074b644
	s_mov_b32 s7, 0x3da4b9ba
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0xa78ce240
	s_mov_b32 s7, 0xbde20345
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0xcefddd8
	s_mov_b32 s7, 0x3e188b7a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x8c94b617
	s_mov_b32 s7, 0xbe4aded4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x312306d0
	s_mov_b32 s7, 0x3e7803aa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x6f4c5a9b
	s_mov_b32 s7, 0xbea1b010
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x7cfd79ae
	s_mov_b32 s7, 0x3ec58c0e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x6410fdf7
	s_mov_b32 s7, 0xbee59e38
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x1f9b1786
	s_mov_b32 s7, 0x3f0192fc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0xf4634b2e
	s_mov_b32 s7, 0xbf162cf3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0xb42f7e4b
	s_mov_b32 s7, 0x3f2314df
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0xc047288a
	s_mov_b32 s7, 0xbf12cb68
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x7bbcce25
	s_mov_b32 s7, 0xbf4038ff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0xae1babae
	s_mov_b32 s7, 0x3f5a9466
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0xe65a6063
	s_mov_b32 s7, 0xbf258be1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x6738ee3a
	s_mov_b32 s7, 0xbf939bc1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x28146b69
	s_mov_b32 s7, 0x3fba4fbc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0xa69750c4
	s_mov_b32 s7, 0x3fe45f2d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x919fcca8
	s_mov_b32 s7, 0x3fc06ebb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], s[6:7]
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0xbff71547
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], |v[4:5]|
	v_mul_f64 v[8:9], v[6:7], s[6:7]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0xbfe62e42
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[6:7]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[8:9]
	v_fma_f64 v[10:11], v[8:9], s[6:7], -v[6:7]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0xbc7abc9e
	v_cvt_i32_f64_e32 v14, v[8:9]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], s[6:7], v[10:11]
	s_mov_b32 s6, 0xfca7ab0c
	s_mov_b32 s7, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], s[8:9], s[6:7]
	s_mov_b32 s6, 0x623fde64
	s_mov_b32 s7, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[6:7]
	s_mov_b32 s6, 0x7c89e6b0
	s_mov_b32 s7, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[6:7]
	s_mov_b32 s6, 0x14761f6e
	s_mov_b32 s7, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[6:7]
	s_mov_b32 s6, 0x1852b7b0
	s_mov_b32 s7, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[6:7]
	s_mov_b32 s6, 0x11122322
	s_mov_b32 s7, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[6:7]
	s_mov_b32 s6, 0x555502a1
	s_mov_b32 s7, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[6:7]
	s_mov_b32 s6, 0x55555511
	s_mov_b32 s7, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[6:7]
	s_mov_b32 s6, 11
	s_mov_b32 s7, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], 1.0
	v_fma_f64 v[8:9], v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v14
	v_add_f64 v[8:9], -v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v9, 0xfff00000, v9, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v6, 0, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v7, 0x3ff00000, v9, s0
.LBB8_3:
	s_and_not1_saveexec_b32 s0, s1
	s_cbranch_execz .LBB8_5
; %bb.4:
	v_mul_f64 v[6:7], v[4:5], v[4:5]
	s_mov_b32 s6, 0xdfeb1f49
	s_mov_b32 s8, 0x51d2ebeb
	s_mov_b32 s7, 0x3e4d6e3d
	s_mov_b32 s9, 0xbe0ab15c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[6:7]
	s_mov_b32 s6, 0x63844720
	s_mov_b32 s7, 0xbe85bfe7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x4280cfb9
	s_mov_b32 s7, 0x3ebb97e4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x4c771c5
	s_mov_b32 s7, 0xbeef4ca2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x75531772
	s_mov_b32 s7, 0x3f1f9a2b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x149d904
	s_mov_b32 s7, 0xbf4c02db
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xcf7e2856
	s_mov_b32 s7, 0x3f7565bc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x311ee09b
	s_mov_b32 s7, 0xbf9b82ce
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x1a0408d1
	s_mov_b32 s7, 0x3fbce2f2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x6b0379b2
	s_mov_b32 s7, 0xbfd81274
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x8214db68
	s_mov_b32 s7, 0x3fc06eba
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], |v[4:5]|, v[6:7], |v[4:5]|
.LBB8_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v8, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v9, null, s5, v1, vcc_lo
	v_bfi_b32 v7, 0x7fffffff, v7, v5
	v_mul_f64 v[2:3], v[2:3], 0.5
	v_add_co_u32 v0, vcc_lo, s2, v0
	global_load_b64 v[8:9], v[8:9], off
	v_add_f64 v[4:5], v[6:7], 1.0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[8:9], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB8_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gapact_geglu_kernel
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
.Lfunc_end8:
	.size	gapact_geglu_kernel, .Lfunc_end8-gapact_geglu_kernel
                                        ; -- End function
	.set gapact_geglu_kernel.num_vgpr, 15
	.set gapact_geglu_kernel.num_agpr, 0
	.set gapact_geglu_kernel.numbered_sgpr, 10
	.set gapact_geglu_kernel.num_named_barrier, 0
	.set gapact_geglu_kernel.private_seg_size, 0
	.set gapact_geglu_kernel.uses_vcc, 1
	.set gapact_geglu_kernel.uses_flat_scratch, 0
	.set gapact_geglu_kernel.has_dyn_sized_stack, 0
	.set gapact_geglu_kernel.has_recursion, 0
	.set gapact_geglu_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1728
; TotalNumSgprs: 12
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 12
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
	.section	.text._Z17gapact_elu_kernelIfEvPKT_PS0_iS2_,"axG",@progbits,_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_,comdat
	.protected	_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_ ; -- Begin function _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_
	.globl	_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_
	.p2align	8
	.type	_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_,@function
_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_:  ; @_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB9_4
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_nlt_f32_e32 0, v2
	s_cbranch_execz .LBB9_3
; %bb.2:
	v_mul_f32_e32 v3, 0x3fb8aa3b, v2
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v2
	s_load_b64 s[0:1], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rndne_f32_e32 v4, v3
	v_fma_f32 v5, 0x3fb8aa3b, v2, -v3
	v_sub_f32_e32 v3, v3, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmamk_f32 v5, v2, 0x32a5705f, v5
	v_cvt_i32_f32_e32 v4, v4
	v_add_f32_e32 v3, v3, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v3, v3
	s_waitcnt lgkmcnt(0)
	s_load_b32 s0, s[0:1], 0x0
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v3, v3, v4
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, 0x7f800000, v3, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_cvt_f64_f32_e32 v[4:5], s0
	v_cvt_f64_f32_e32 v[2:3], v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], -1.0
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_f32_f64_e32 v2, v[2:3]
.LBB9_3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b32 v[0:1], v2, off
.LBB9_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_
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
	.section	.text._Z17gapact_elu_kernelIfEvPKT_PS0_iS2_,"axG",@progbits,_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_,comdat
.Lfunc_end9:
	.size	_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_, .Lfunc_end9-_Z17gapact_elu_kernelIfEvPKT_PS0_iS2_
                                        ; -- End function
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.num_vgpr, 6
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.num_agpr, 0
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.numbered_sgpr, 8
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.num_named_barrier, 0
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.private_seg_size, 0
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.uses_vcc, 1
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.uses_flat_scratch, 0
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.has_dyn_sized_stack, 0
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.has_recursion, 0
	.set _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 332
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
	.section	.text._Z17gapact_elu_kernelIdEvPKT_PS0_iS2_,"axG",@progbits,_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_,comdat
	.protected	_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_ ; -- Begin function _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_
	.globl	_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_
	.p2align	8
	.type	_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_,@function
_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_:  ; @_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB10_4
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_nlt_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB10_3
; %bb.2:
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0x3ff71547
	s_mov_b32 s8, 0x6a5dcb37
	v_mul_f64 v[4:5], v[2:3], s[4:5]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0xbfe62e42
	s_mov_b32 s9, 0x3e5ade15
	v_cmp_ngt_f64_e32 vcc_lo, 0xc090cc00, v[2:3]
	s_load_b64 s[0:1], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[2:3]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0xbc7abc9e
	v_cvt_i32_f64_e32 v10, v[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	s_mov_b32 s4, 0xfca7ab0c
	s_mov_b32 s5, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[4:5]
	s_mov_b32 s4, 0x623fde64
	s_mov_b32 s5, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x7c89e6b0
	s_mov_b32 s5, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x14761f6e
	s_mov_b32 s5, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x1852b7b0
	s_mov_b32 s5, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x11122322
	s_mov_b32 s5, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x555502a1
	s_mov_b32 s5, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x55555511
	s_mov_b32 s5, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 11
	s_mov_b32 s5, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_add_f64 v[4:5], v[4:5], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, 0xbff00000, v5, vcc_lo
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], s[0:1], v[2:3]
.LBB10_3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB10_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_
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
		.amdhsa_inst_pref_size 5
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z17gapact_elu_kernelIdEvPKT_PS0_iS2_,"axG",@progbits,_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_,comdat
.Lfunc_end10:
	.size	_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_, .Lfunc_end10-_Z17gapact_elu_kernelIdEvPKT_PS0_iS2_
                                        ; -- End function
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.num_vgpr, 11
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.num_agpr, 0
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.numbered_sgpr, 10
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.num_named_barrier, 0
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.private_seg_size, 0
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.uses_vcc, 1
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.uses_flat_scratch, 0
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.has_dyn_sized_stack, 0
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.has_recursion, 0
	.set _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 620
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
	.section	.text._Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_,"axG",@progbits,_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_,comdat
	.protected	_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_ ; -- Begin function _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_
	.globl	_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_
	.p2align	8
	.type	_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_,@function
_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_: ; @_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB11_6
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b128 s[0:3], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_mov_b32 s4, exec_lo
	global_load_b32 v4, v[2:3], off
                                        ; implicit-def: $vgpr2_vgpr3
	s_waitcnt vmcnt(0)
	v_cmpx_nlt_f32_e32 0, v4
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB11_3
; %bb.2:
	v_mul_f32_e32 v2, 0x3fb8aa3b, v4
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v4
	s_load_b32 s0, s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rndne_f32_e32 v3, v2
	v_fma_f32 v5, 0x3fb8aa3b, v4, -v2
	v_dual_sub_f32 v2, v2, v3 :: v_dual_fmamk_f32 v5, v4, 0x32a5705f, v5
	v_cvt_i32_f32_e32 v3, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v2, v2, v5
	v_exp_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v2, v2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v4
	s_waitcnt lgkmcnt(0)
	v_cvt_f64_f32_e32 v[4:5], s0
	v_cndmask_b32_e32 v2, 0x7f800000, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[2:3], v2
	v_add_f64 v[2:3], v[2:3], -1.0
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
                                        ; implicit-def: $vgpr4
.LBB11_3:
	s_or_saveexec_b32 s0, s4
	s_load_b32 s1, s[2:3], 0x0
	s_xor_b32 exec_lo, exec_lo, s0
	s_cbranch_execz .LBB11_5
; %bb.4:
	v_cvt_f64_f32_e32 v[2:3], v4
.LBB11_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_waitcnt lgkmcnt(0)
	v_cvt_f64_f32_e32 v[4:5], s1
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_f32_f64_e32 v2, v[2:3]
	global_store_b32 v[0:1], v2, off
.LBB11_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_
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
	.section	.text._Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_,"axG",@progbits,_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_,comdat
.Lfunc_end11:
	.size	_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_, .Lfunc_end11-_Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_
                                        ; -- End function
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.num_vgpr, 6
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.num_agpr, 0
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.numbered_sgpr, 8
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.num_named_barrier, 0
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.private_seg_size, 0
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.uses_vcc, 1
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.uses_flat_scratch, 0
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.has_dyn_sized_stack, 0
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.has_recursion, 0
	.set _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 372
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
	.section	.text._Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_,"axG",@progbits,_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_,comdat
	.protected	_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_ ; -- Begin function _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_
	.globl	_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_
	.p2align	8
	.type	_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_,@function
_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_: ; @_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB12_4
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b128 s[0:3], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_load_b64 s[2:3], s[2:3], 0x0
	s_mov_b32 s4, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_nlt_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB12_3
; %bb.2:
	s_mov_b32 s8, 0x652b82fe
	s_mov_b32 s9, 0x3ff71547
	s_mov_b32 s10, 0x6a5dcb37
	v_mul_f64 v[4:5], v[2:3], s[8:9]
	s_mov_b32 s8, 0xfefa39ef
	s_mov_b32 s9, 0xbfe62e42
	s_mov_b32 s11, 0x3e5ade15
	v_cmp_ngt_f64_e32 vcc_lo, 0xc090cc00, v[2:3]
	s_load_b64 s[0:1], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[8:9], v[2:3]
	s_mov_b32 s8, 0x3b39803f
	s_mov_b32 s9, 0xbc7abc9e
	v_cvt_i32_f64_e32 v10, v[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[8:9], v[6:7]
	s_mov_b32 s8, 0xfca7ab0c
	s_mov_b32 s9, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[10:11], s[8:9]
	s_mov_b32 s8, 0x623fde64
	s_mov_b32 s9, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x7c89e6b0
	s_mov_b32 s9, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x14761f6e
	s_mov_b32 s9, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x1852b7b0
	s_mov_b32 s9, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x11122322
	s_mov_b32 s9, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x555502a1
	s_mov_b32 s9, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x55555511
	s_mov_b32 s9, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 11
	s_mov_b32 s9, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_add_f64 v[4:5], v[4:5], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, 0xbff00000, v5, vcc_lo
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], s[0:1], v[2:3]
.LBB12_3:
	s_or_b32 exec_lo, exec_lo, s4
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[2:3], s[2:3], v[2:3]
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB12_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_
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
	.section	.text._Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_,"axG",@progbits,_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_,comdat
.Lfunc_end12:
	.size	_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_, .Lfunc_end12-_Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_
                                        ; -- End function
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.num_vgpr, 11
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.num_agpr, 0
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.numbered_sgpr, 12
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.num_named_barrier, 0
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.private_seg_size, 0
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.uses_vcc, 1
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.uses_flat_scratch, 0
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.has_dyn_sized_stack, 0
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.has_recursion, 0
	.set _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 636
; TotalNumSgprs: 14
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
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
	.section	.text._Z22gapact_softplus_kernelIfEvPKT_PS0_i,"axG",@progbits,_Z22gapact_softplus_kernelIfEvPKT_PS0_i,comdat
	.protected	_Z22gapact_softplus_kernelIfEvPKT_PS0_i ; -- Begin function _Z22gapact_softplus_kernelIfEvPKT_PS0_i
	.globl	_Z22gapact_softplus_kernelIfEvPKT_PS0_i
	.p2align	8
	.type	_Z22gapact_softplus_kernelIfEvPKT_PS0_i,@function
_Z22gapact_softplus_kernelIfEvPKT_PS0_i: ; @_Z22gapact_softplus_kernelIfEvPKT_PS0_i
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
	s_cbranch_execz .LBB13_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x3e9b6dac
	global_load_b32 v4, v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f32_e64 v2, 0xbfb8aa3b, |v4|
	v_cmp_nlt_f32_e64 vcc_lo, 0x42ce8ed0, |v4|
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v3, 0xbfb8aa3b, |v4|, -v2
	v_rndne_f32_e32 v5, v2
	v_fma_f32 v3, 0xb2a5705f, |v4|, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v2, v2, v5
	v_add_f32_e32 v2, v2, v3
	v_cvt_i32_f32_e32 v3, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v2, v2, v3
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	v_cmp_ngt_f32_e64 vcc_lo, 0xc2b17218, |v4|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0x7f800000, v2, vcc_lo
	v_add_f32_e32 v6, 1.0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[2:3], v6
	v_frexp_exp_i32_f64_e32 v2, v[2:3]
	v_frexp_mant_f32_e32 v3, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v3
	v_add_f32_e32 v3, -1.0, v6
	v_sub_f32_e32 v8, v3, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v8, 1.0, v8 :: v_dual_sub_f32 v3, v5, v3
	v_add_f32_e32 v3, v3, v8
	v_subrev_co_ci_u32_e64 v2, null, 0, v2, vcc_lo
	v_cmp_neq_f32_e32 vcc_lo, 0x7f800000, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v7, 0, v2
	v_cvt_f32_i32_e32 v2, v2
	v_ldexp_f32 v6, v6, v7
	v_ldexp_f32 v3, v3, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, 1.0, v6
	v_dual_add_f32 v7, -1.0, v6 :: v_dual_add_f32 v8, -1.0, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v10, 1.0, v7
	v_sub_f32_e32 v8, v6, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v6, v6, v10
	v_add_f32_e32 v8, v3, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, v3, v6
	v_dual_add_f32 v11, v7, v3 :: v_dual_add_f32 v10, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v7, v7, v11
	v_rcp_f32_e32 v6, v10
	v_sub_f32_e32 v9, v9, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_dual_add_f32 v8, v8, v9 :: v_dual_add_f32 v3, v3, v7
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v12, v11, v6
	v_mul_f32_e32 v13, v10, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v9, v12, v10, -v13
	v_fmac_f32_e32 v9, v12, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v14, v13, v9
	v_sub_f32_e32 v15, v11, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v11, v11, v15
	v_sub_f32_e32 v7, v14, v13
	v_sub_f32_e32 v11, v11, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v7, v7, v9
	v_add_f32_e32 v3, v3, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, v7, v3
	v_add_f32_e32 v7, v15, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v9, v6, v7
	v_dual_sub_f32 v14, v15, v7 :: v_dual_mul_f32 v11, v10, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v3, v3, v14
	v_fma_f32 v10, v9, v10, -v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v10, v9, v8
	v_add_f32_e32 v8, v11, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v13, v7, v8
	v_sub_f32_e32 v7, v7, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v7, v7, v8
	v_add_f32_e32 v3, v3, v7
	v_add_f32_e32 v7, v12, v9
	v_sub_f32_e32 v11, v8, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v8, v11, v10
	v_dual_add_f32 v3, v8, v3 :: v_dual_sub_f32 v8, v7, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, v13, v3
	v_dual_sub_f32 v8, v9, v8 :: v_dual_mul_f32 v3, v6, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v3, v8, v3
	v_add_f32_e32 v6, v7, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v8, v6, v6
	v_fmaak_f32 v9, s0, v8, 0x3ecc95a3
	v_mul_f32_e32 v10, v6, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fmaak_f32 v8, v8, v9, 0x3f2aaada
	v_ldexp_f32 v9, v6, 1
	v_sub_f32_e32 v6, v6, v7
	v_mul_f32_e32 v8, v10, v8
	v_mul_f32_e32 v10, 0x3f317218, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v3, v3, v6
	v_add_f32_e32 v7, v9, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f32 v3, v3, 1
	v_sub_f32_e32 v6, v7, v9
	v_fma_f32 v9, 0x3f317218, v2, -v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v6, v8, v6
	v_dual_fmamk_f32 v2, v2, 0xb102e308, v9 :: v_dual_add_f32 v3, v3, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v6, v10, v2
	v_add_f32_e32 v8, v7, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v10, v6, v10
	v_add_f32_e32 v9, v6, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v10, v2, v10
	v_sub_f32_e32 v11, v9, v6
	v_sub_f32_e32 v7, v8, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v12, v9, v11
	v_dual_sub_f32 v2, v6, v12 :: v_dual_sub_f32 v7, v3, v7
	v_sub_f32_e32 v3, v8, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v6, v10, v7
	v_add_f32_e32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v8, v6, v10
	v_add_f32_e32 v11, v6, v2
	v_cvt_f64_f32_e32 v[2:3], v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v4, v6, v8
	v_dual_add_f32 v6, v9, v11 :: v_dual_sub_f32 v7, v7, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v4, v10, v4
	v_sub_f32_e32 v8, v6, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v4, v7, v4
	v_sub_f32_e32 v7, v11, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v4, v4, v7
	v_add_f32_e32 v4, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0x7f800000, v4, vcc_lo
	v_cmp_gt_f32_e64 vcc_lo, 0x33800000, |v5|
	v_cndmask_b32_e32 v4, v4, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_max_f64 v[2:3], v[2:3], 0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[4:5], v4
	v_add_f64 v[2:3], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_f32_f64_e32 v2, v[2:3]
	global_store_b32 v[0:1], v2, off
.LBB13_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22gapact_softplus_kernelIfEvPKT_PS0_i
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
		.amdhsa_inst_pref_size 8
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z22gapact_softplus_kernelIfEvPKT_PS0_i,"axG",@progbits,_Z22gapact_softplus_kernelIfEvPKT_PS0_i,comdat
.Lfunc_end13:
	.size	_Z22gapact_softplus_kernelIfEvPKT_PS0_i, .Lfunc_end13-_Z22gapact_softplus_kernelIfEvPKT_PS0_i
                                        ; -- End function
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.num_vgpr, 16
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.num_agpr, 0
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.numbered_sgpr, 5
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.num_named_barrier, 0
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.private_seg_size, 0
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.uses_vcc, 1
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.uses_flat_scratch, 0
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.has_dyn_sized_stack, 0
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.has_recursion, 0
	.set _Z22gapact_softplus_kernelIfEvPKT_PS0_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 976
; TotalNumSgprs: 7
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 7
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
	.section	.text._Z22gapact_softplus_kernelIdEvPKT_PS0_i,"axG",@progbits,_Z22gapact_softplus_kernelIdEvPKT_PS0_i,comdat
	.protected	_Z22gapact_softplus_kernelIdEvPKT_PS0_i ; -- Begin function _Z22gapact_softplus_kernelIdEvPKT_PS0_i
	.globl	_Z22gapact_softplus_kernelIdEvPKT_PS0_i
	.p2align	8
	.type	_Z22gapact_softplus_kernelIdEvPKT_PS0_i,@function
_Z22gapact_softplus_kernelIdEvPKT_PS0_i: ; @_Z22gapact_softplus_kernelIdEvPKT_PS0_i
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
	s_mov_b32 s5, 0xbc7abc9e
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s6, 0xfca7ab0c
	s_mov_b32 s8, 0x6a5dcb37
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s7, 0x3e928af3
	s_mov_b32 s9, 0x3e5ade15
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s11, 0x3fc3ab76
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], |v[2:3]|, s[0:1]
	s_mov_b32 s1, 0xbfe62e42
	s_mov_b32 s0, 0xfefa39ef
	v_cmp_nlt_f64_e64 vcc_lo, 0x4090cc00, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[0:1], -|v[2:3]|
	v_cvt_i32_f64_e32 v10, v[4:5]
	s_mov_b32 s1, 0x3fe62e42
	v_max_f64 v[2:3], v[2:3], v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	s_mov_b32 s5, 0x3c7abc9e
	v_max_f64 v[2:3], v[2:3], 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[6:7]
	s_mov_b32 s6, 0x623fde64
	s_mov_b32 s7, 0x3ec71dee
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s9, 0x3fc38538
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x7c89e6b0
	s_mov_b32 s7, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x14761f6e
	s_mov_b32 s7, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x1852b7b0
	s_mov_b32 s7, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x11122322
	s_mov_b32 s7, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x555502a1
	s_mov_b32 s7, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x55555511
	s_mov_b32 s7, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 11
	s_mov_b32 s7, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[4:5], 1.0
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	v_add_f64 v[10:11], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[8:9]
	s_mov_b32 s6, 0x55555780
	v_add_f64 v[8:9], v[10:11], -v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[10:11]
	v_subrev_co_ci_u32_e64 v28, null, 0, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], 1.0
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[4:5]
	v_sub_nc_u32_e32 v14, 0, v28
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[6:7], v[6:7], v14
	v_add_f64 v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[6:7], 1.0
	v_add_f64 v[18:19], v[6:7], -1.0
	v_ldexp_f64 v[8:9], v[8:9], v14
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[12:13], -1.0
	v_add_f64 v[20:21], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[6:7], -v[10:11]
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], v[10:11]
	v_add_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[20:21], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[16:17], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[22:23], -v[14:15], v[16:17], 1.0
	v_add_f64 v[6:7], v[6:7], -v[18:19]
	v_fma_f64 v[16:17], v[22:23], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[20:21], v[8:9]
	v_mul_f64 v[22:23], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[14:15], -v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[24:25], v[22:23], v[12:13]
	v_add_f64 v[26:27], v[20:21], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[20:21], v[20:21], -v[24:25]
	v_add_f64 v[6:7], v[6:7], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], v[6:7]
	v_add_f64 v[12:13], v[26:27], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	v_add_f64 v[24:25], v[26:27], -v[12:13]
	v_mul_f64 v[20:21], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[24:25]
	v_fma_f64 v[14:15], v[18:19], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[18:19], v[10:11], v[14:15]
	v_add_f64 v[14:15], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[20:21], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[16:17], v[18:19]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[12:13], -v[16:17]
	v_add_f64 v[6:7], v[22:23], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[18:19], -v[10:11]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[8:9], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	v_fma_f64 v[14:15], v[10:11], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[6:7]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v28
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[16:17], s[0:1]
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[0:1], -v[18:19]
	v_cmp_nge_f64_e64 s0, -1.0, v[4:5]
	v_cmp_ngt_f64_e64 s1, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[4:5], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[6:7]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[8:9], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[16:17], -v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], -v[20:21]
	v_add_f64 v[12:13], v[14:15], -v[20:21]
	v_add_f64 v[14:15], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[14:15], -v[10:11]
	v_add_f64 v[8:9], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[18:19], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[12:13], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[6:7], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0x7ff80000, v7, s1
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_add_f64 v[2:3], v[2:3], v[6:7]
	global_store_b64 v[0:1], v[2:3], off
.LBB14_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z22gapact_softplus_kernelIdEvPKT_PS0_i
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
	.section	.text._Z22gapact_softplus_kernelIdEvPKT_PS0_i,"axG",@progbits,_Z22gapact_softplus_kernelIdEvPKT_PS0_i,comdat
.Lfunc_end14:
	.size	_Z22gapact_softplus_kernelIdEvPKT_PS0_i, .Lfunc_end14-_Z22gapact_softplus_kernelIdEvPKT_PS0_i
                                        ; -- End function
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.num_vgpr, 29
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.num_agpr, 0
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.numbered_sgpr, 12
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.num_named_barrier, 0
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.private_seg_size, 0
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.uses_vcc, 1
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.uses_flat_scratch, 0
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.has_dyn_sized_stack, 0
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.has_recursion, 0
	.set _Z22gapact_softplus_kernelIdEvPKT_PS0_i.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1840
; TotalNumSgprs: 14
; NumVgprs: 29
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 14
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
	.text
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_dd745aa21e213ce1,@object ; @__hip_cuid_dd745aa21e213ce1
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_dd745aa21e213ce1
__hip_cuid_dd745aa21e213ce1:
	.byte	0                               ; 0x0
	.size	__hip_cuid_dd745aa21e213ce1, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_dd745aa21e213ce1
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
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
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
    .name:           gapact_elu_backward_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         gapact_elu_backward_kernel.kd
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
    .name:           gapact_selu_backward_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         gapact_selu_backward_kernel.kd
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
    .name:           gapact_mish_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     38
    .sgpr_spill_count: 0
    .symbol:         gapact_mish_kernel.kd
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
    .name:           gapact_mish_backward_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     40
    .sgpr_spill_count: 0
    .symbol:         gapact_mish_backward_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     32
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
    .name:           gapact_softplus_backward_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         gapact_softplus_backward_kernel.kd
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
    .name:           gapact_hardswish_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         gapact_hardswish_kernel.kd
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
    .name:           gapact_hardswish_backward_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         gapact_hardswish_backward_kernel.kd
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
    .name:           gapact_swiglu_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         gapact_swiglu_kernel.kd
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
    .name:           gapact_geglu_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         gapact_geglu_kernel.kd
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
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
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
    .name:           _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         _Z17gapact_elu_kernelIfEvPKT_PS0_iS2_.kd
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
      - .address_space:  global
        .offset:         24
        .size:           8
        .value_kind:     global_buffer
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
    .name:           _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         _Z17gapact_elu_kernelIdEvPKT_PS0_iS2_.kd
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
    .name:           _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         _Z18gapact_selu_kernelIfEvPKT_PS0_iS2_S2_.kd
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
    .name:           _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z18gapact_selu_kernelIdEvPKT_PS0_iS2_S2_.kd
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
    .name:           _Z22gapact_softplus_kernelIfEvPKT_PS0_i
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         _Z22gapact_softplus_kernelIfEvPKT_PS0_i.kd
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
    .name:           _Z22gapact_softplus_kernelIdEvPKT_PS0_i
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z22gapact_softplus_kernelIdEvPKT_PS0_i.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     29
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
