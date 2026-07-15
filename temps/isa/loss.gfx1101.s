	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	mae_grad_kernel         ; -- Begin function mae_grad_kernel
	.globl	mae_grad_kernel
	.p2align	8
	.type	mae_grad_kernel,@function
mae_grad_kernel:                        ; @mae_grad_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s4, s[0:1], 0x2c
	s_load_b32 s3, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s4, s4, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s4, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s3, v1
	s_cbranch_execz .LBB0_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], -v[4:5]
	v_cvt_f64_i32_e32 v[4:5], s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e64 v7, 0, 0xbff00000, vcc_lo
	v_cmp_nlt_f64_e32 vcc_lo, 0, v[2:3]
	v_dual_mov_b32 v6, 0 :: v_dual_cndmask_b32 v7, 0x3ff00000, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[2:3], null, v[4:5], v[4:5], v[6:7]
	v_rcp_f64_e32 v[8:9], v[2:3]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[2:3], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[2:3], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_div_scale_f64 v[10:11], vcc_lo, v[6:7], v[4:5], v[6:7]
	v_mul_f64 v[12:13], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], -v[2:3], v[12:13], v[10:11]
	v_div_fmas_f64 v[2:3], v[2:3], v[8:9], v[12:13]
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	v_div_fixup_f64 v[2:3], v[2:3], v[4:5], v[6:7]
	global_store_b64 v[0:1], v[2:3], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel mae_grad_kernel
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
.Lfunc_end0:
	.size	mae_grad_kernel, .Lfunc_end0-mae_grad_kernel
                                        ; -- End function
	.set mae_grad_kernel.num_vgpr, 14
	.set mae_grad_kernel.num_agpr, 0
	.set mae_grad_kernel.numbered_sgpr, 8
	.set mae_grad_kernel.num_named_barrier, 0
	.set mae_grad_kernel.private_seg_size, 0
	.set mae_grad_kernel.uses_vcc, 1
	.set mae_grad_kernel.uses_flat_scratch, 0
	.set mae_grad_kernel.has_dyn_sized_stack, 0
	.set mae_grad_kernel.has_recursion, 0
	.set mae_grad_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 336
; TotalNumSgprs: 10
; NumVgprs: 14
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.protected	huber_grad_kernel       ; -- Begin function huber_grad_kernel
	.globl	huber_grad_kernel
	.p2align	8
	.type	huber_grad_kernel,@function
huber_grad_kernel:                      ; @huber_grad_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s4, s[0:1], 0x34
	s_load_b32 s3, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s4, s4, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s4, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s3, v1
	s_cbranch_execz .LBB1_9
; %bb.1:
	v_cvt_f64_i32_e32 v[6:7], s3
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s5, v3, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v2
	v_add_co_ci_u32_e64 v5, null, s7, v3, vcc_lo
	global_load_b64 v[0:1], v[0:1], off
	global_load_b64 v[4:5], v[4:5], off
	s_load_b64 s[0:1], s[0:1], 0x0
	s_mov_b32 s4, exec_lo
	v_div_scale_f64 v[8:9], null, v[6:7], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[12:13], v[10:11]
	v_fma_f64 v[8:9], -v[8:9], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[14:15]
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], v[0:1], -v[4:5]
	v_div_fixup_f64 v[0:1], v[8:9], v[6:7], 1.0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2)
	v_cmpx_nlt_f64_e32 s[0:1], v[4:5]
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB1_7
; %bb.2:
	v_cmp_nlt_f64_e64 s5, v[4:5], -s[0:1]
	v_add_co_u32 v2, vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_and_saveexec_b32 s6, s5
	s_xor_b32 s5, exec_lo, s6
	s_cbranch_execz .LBB1_4
; %bb.3:
	v_mul_f64 v[0:1], v[0:1], v[4:5]
	global_store_b64 v[2:3], v[0:1], off
                                        ; implicit-def: $vgpr0_vgpr1
                                        ; implicit-def: $vgpr2_vgpr3
.LBB1_4:
	s_and_not1_saveexec_b32 s5, s5
	s_cbranch_execz .LBB1_6
; %bb.5:
	v_mul_f64 v[0:1], v[0:1], -s[0:1]
	global_store_b64 v[2:3], v[0:1], off
.LBB1_6:
	s_or_b32 exec_lo, exec_lo, s5
                                        ; implicit-def: $vgpr0_vgpr1
                                        ; implicit-def: $vgpr2_vgpr3
.LBB1_7:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB1_9
; %bb.8:
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[0:1], v[0:1], s[0:1]
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	global_store_b64 v[2:3], v[0:1], off
.LBB1_9:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel huber_grad_kernel
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
	.size	huber_grad_kernel, .Lfunc_end1-huber_grad_kernel
                                        ; -- End function
	.set huber_grad_kernel.num_vgpr, 16
	.set huber_grad_kernel.num_agpr, 0
	.set huber_grad_kernel.numbered_sgpr, 8
	.set huber_grad_kernel.num_named_barrier, 0
	.set huber_grad_kernel.private_seg_size, 0
	.set huber_grad_kernel.uses_vcc, 1
	.set huber_grad_kernel.uses_flat_scratch, 0
	.set huber_grad_kernel.has_dyn_sized_stack, 0
	.set huber_grad_kernel.has_recursion, 0
	.set huber_grad_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 444
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
	.protected	bce_logits_kernel       ; -- Begin function bce_logits_kernel
	.globl	bce_logits_kernel
	.p2align	8
	.type	bce_logits_kernel,@function
bce_logits_kernel:                      ; @bce_logits_kernel
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
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s13, 0x3ff71547
	s_mov_b32 s12, 0x652b82fe
	s_mov_b32 s10, 0xfefa39ef
	s_mov_b32 s11, 0xbfe62e42
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s8, 0x3b39803f
	s_mov_b32 s9, 0xbc7abc9e
	s_mov_b32 s14, 0xfca7ab0c
	s_mov_b32 s16, 0x6a5dcb37
	s_mov_b32 s15, 0x3e928af3
	s_mov_b32 s17, 0x3e5ade15
	s_mov_b32 s18, 0x623fde64
	s_mov_b32 s19, 0x3ec71dee
	s_mov_b32 s20, 0x7c89e6b0
	s_mov_b32 s21, 0x3efa0199
	s_mov_b32 s22, 0x14761f6e
	s_mov_b32 s23, 0x3f2a01a0
	s_mov_b32 s24, 0x1852b7b0
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s25, 0x3f56c16c
	s_mov_b32 s26, 0x11122322
	s_mov_b32 s27, 0x3f811111
	global_load_b64 v[2:3], v[2:3], off
	s_mov_b32 s28, 0x555502a1
	s_mov_b32 s29, 0x3fa55555
	s_mov_b32 s30, 0x55555511
	s_mov_b32 s31, 0x3fc55555
	s_mov_b32 s34, 11
	s_mov_b32 s35, 0x3fe00000
	s_mov_b32 s1, 0x3fe55555
	s_waitcnt vmcnt(0)
	v_cmp_le_f64_e32 vcc_lo, 0, v[2:3]
	v_xor_b32_e32 v5, 0x80000000, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mov_b32 v4, v2 :: v_dual_cndmask_b32 v5, v3, v5
	v_mul_f64 v[6:7], v[4:5], s[12:13]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[4:5]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[4:5]
	s_mov_b32 s13, 0xbff71547
	v_rndne_f64_e32 v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], s[10:11], v[4:5]
	v_cvt_i32_f64_e32 v12, v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], s[16:17], s[14:15]
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[20:21]
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[24:25]
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[28:29]
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[34:35]
	v_fma_f64 v[10:11], v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[8:9], v[10:11], 1.0
	v_ldexp_f64 v[6:7], v[6:7], v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v4, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0, v7, s0
	s_mov_b32 s0, 0x55555555
	v_add_f64 v[6:7], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	v_add_f64 v[10:11], v[6:7], -1.0
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[8:9]
	s_mov_b32 s0, 0x55555780
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[10:11], -v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[10:11]
	v_subrev_co_ci_u32_e64 v28, null, 0, v12, vcc_lo
	v_add_f64 v[8:9], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v14, 0, v28
	v_ldexp_f64 v[6:7], v[6:7], v14
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_mul_f64 v[14:15], v[2:3], s[12:13]
	s_mov_b32 s12, 0x6b47b09a
	s_mov_b32 s13, 0x3fc38538
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[10:11], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[22:23], v[6:7]
	v_add_f64 v[10:11], v[18:19], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_rndne_f64_e32 v[8:9], v[14:15]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], s[10:11], -v[2:3]
	s_mov_b32 s11, 0x3fe62e42
	v_add_f64 v[14:15], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], s[8:9], v[10:11]
	s_mov_b32 s9, 0x3c7abc9e
	v_mul_f64 v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[10:11], s[16:17], s[14:15]
	s_mov_b32 s14, 0xbf559e2b
	s_mov_b32 s15, 0x3fc3ab76
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_fma_f64 v[20:21], v[16:17], s[14:15], s[12:13]
	s_mov_b32 s12, 0xd7f4df2e
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[18:19]
	s_mov_b32 s13, 0x3fc7474d
	v_mul_f64 v[22:23], v[14:15], v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[12:13]
	s_mov_b32 s12, 0x16291751
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[20:21]
	s_mov_b32 s13, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[12:13]
	s_mov_b32 s12, 0x9b27acf1
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[22:23]
	s_mov_b32 s13, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[12:13]
	s_mov_b32 s12, 0x998ef7b6
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[24:25]
	s_mov_b32 s13, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[26:27]
	v_fma_f64 v[16:17], v[16:17], v[20:21], s[0:1]
	v_ldexp_f64 v[20:21], v[14:15], 1
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[28:29]
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[4:5]
	v_cmp_ngt_f64_e64 s1, 0xc0900000, v[2:3]
	v_mul_f64 v[16:17], v[22:23], v[16:17]
	v_cvt_f64_i32_e32 v[22:23], v28
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[30:31]
	v_add_co_u32 v28, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v29, null, s3, v1, vcc_lo
	v_cmp_nlt_f64_e64 s2, 0x4090cc00, v[2:3]
	v_cmp_ngt_f64_e64 s3, -1.0, v[4:5]
	global_load_b64 v[28:29], v[28:29], off
	v_add_f64 v[14:15], v[20:21], v[16:17]
	v_mul_f64 v[24:25], v[22:23], s[10:11]
	v_fma_f64 v[18:19], v[10:11], v[18:19], s[34:35]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[14:15], -v[20:21]
	v_fma_f64 v[20:21], v[22:23], s[10:11], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[10:11], v[18:19], 1.0
	v_add_f64 v[12:13], v[16:17], -v[12:13]
	v_cvt_i32_f64_e32 v16, v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[8:9], v[10:11], v[18:19], 1.0
	v_fma_f64 v[10:11], v[22:23], s[8:9], v[20:21]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[8:9], v[8:9], v16
	v_add_f64 v[12:13], v[24:25], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[14:15], v[6:7]
	v_add_f64 v[8:9], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[12:13], -v[24:25]
	v_add_f64 v[18:19], v[12:13], v[16:17]
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_div_scale_f64 v[20:21], null, v[8:9], v[8:9], 1.0
	v_add_f64 v[10:11], v[10:11], -v[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[22:23], v[18:19], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[26:27], v[20:21]
	v_add_f64 v[30:31], v[18:19], -v[22:23]
	v_add_f64 v[14:15], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[10:11], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[32:33], -v[20:21], v[26:27], 1.0
	v_add_f64 v[12:13], v[12:13], -v[30:31]
	v_add_f64 v[24:25], v[22:23], -v[10:11]
	v_fma_f64 v[16:17], v[26:27], v[32:33], v[26:27]
	v_div_scale_f64 v[26:27], vcc_lo, 1.0, v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[14:15], v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], -v[20:21], v[16:17], 1.0
	v_add_f64 v[12:13], v[22:23], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[16:17]
	v_add_f64 v[16:17], v[22:23], -v[24:25]
	v_add_f64 v[22:23], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[30:31], v[26:27], v[14:15]
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[22:23], -v[18:19]
	v_fma_f64 v[18:19], -v[20:21], v[30:31], v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[10:11], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_div_fmas_f64 v[12:13], v[18:19], v[14:15], v[30:31]
	v_max_f64 v[14:15], v[2:3], v[2:3]
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[4:5]
	v_add_f64 v[6:7], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_div_fixup_f64 v[8:9], v[12:13], v[8:9], 1.0
	v_max_f64 v[10:11], v[14:15], 0
	s_and_b32 vcc_lo, vcc_lo, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[22:23], v[6:7]
	v_cndmask_b32_e64 v9, 0, v9, s1
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[2:3], -v[2:3], v[28:29], v[10:11]
	v_cndmask_b32_e64 v7, 0x7ff00000, v7, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v10, 0x7ff80000, v7, s3
	v_cndmask_b32_e64 v7, 0x3ff00000, v9, s2
	v_cndmask_b32_e32 v9, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[4:5]
	v_cndmask_b32_e32 v10, 0xfff00000, v10, vcc_lo
	s_and_b32 vcc_lo, s2, s1
	v_cndmask_b32_e32 v6, 0, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[2:3], v[2:3], v[9:10]
	v_add_f64 v[4:5], v[6:7], -v[28:29]
	v_add_co_u32 v6, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s5, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[6:7], v[2:3], off
	global_store_b64 v[0:1], v[4:5], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel bce_logits_kernel
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
		.amdhsa_next_free_vgpr 34
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
	.size	bce_logits_kernel, .Lfunc_end2-bce_logits_kernel
                                        ; -- End function
	.set bce_logits_kernel.num_vgpr, 34
	.set bce_logits_kernel.num_agpr, 0
	.set bce_logits_kernel.numbered_sgpr, 36
	.set bce_logits_kernel.num_named_barrier, 0
	.set bce_logits_kernel.private_seg_size, 0
	.set bce_logits_kernel.uses_vcc, 1
	.set bce_logits_kernel.uses_flat_scratch, 0
	.set bce_logits_kernel.has_dyn_sized_stack, 0
	.set bce_logits_kernel.has_recursion, 0
	.set bce_logits_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2276
; TotalNumSgprs: 38
; NumVgprs: 34
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 38
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
	.text
	.protected	focal_loss_kernel       ; -- Begin function focal_loss_kernel
	.globl	focal_loss_kernel
	.p2align	8
	.type	focal_loss_kernel,@function
focal_loss_kernel:                      ; @focal_loss_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b32 s4, s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_movk_i32 s2, 0xdcd1
	s_mov_b32 s3, 0x3fefffff
	s_mov_b32 s12, 0x968915a9
	s_mov_b32 s14, 0x4222de17
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s13, 0x3fba6564
	s_mov_b32 s15, 0x3fbdee67
	s_mov_b32 s16, 0x3abe935a
	s_mov_b32 s17, 0x3fbe25e4
	s_mov_b32 s18, 0x47e6c9c2
	s_mov_b32 s19, 0x3fc110ef
	s_mov_b32 s20, 0x6b47b09a
	s_mov_b32 s22, 0xbf559e2b
	s_mov_b32 s21, 0x3fc38538
	s_mov_b32 s23, 0x3fc3ab76
	s_mov_b32 s24, 0x16291751
	s_mov_b32 s25, 0x3fcc71c0
	s_mov_b32 s26, 0x1c7792ce
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_load_b128 s[4:7], s[0:1], 0x28
	s_mov_b32 s27, 0x3fcc71c7
	global_load_b64 v[4:5], v[4:5], off
	s_mov_b32 s29, 0x3fd99999
	s_mov_b32 s28, 0x998ef7b6
	s_mov_b32 s30, 0x14761f6e
	s_mov_b32 s31, 0x3f2a01a0
	s_mov_b32 s34, 0x1852b7b0
	s_mov_b32 s35, 0x3f56c16c
	s_mov_b32 s36, 0x11122322
	s_mov_b32 s37, 0x3f811111
	s_mov_b32 s38, 0x55555511
	s_mov_b32 s39, 0x3fc55555
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[4:5], s[4:5], 0x0
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[20:21], s[4:5], -1.0
	s_waitcnt vmcnt(1)
	v_cmp_nlt_f64_e32 vcc_lo, s[2:3], v[2:3]
	s_mov_b32 s2, 0x812dea11
	s_mov_b32 s3, 0x3d719799
	v_cndmask_b32_e32 v6, 0xffffdcd1, v2, vcc_lo
	v_cndmask_b32_e32 v7, 0x3fefffff, v3, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, s[2:3], v[2:3]
	s_mov_b32 s2, 0x55555555
	s_mov_b32 s3, 0x3fe55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, 0x3d719799, v7, vcc_lo
	v_cndmask_b32_e32 v2, 0x812dea11, v6, vcc_lo
	s_waitcnt vmcnt(0)
	v_cmp_lt_f64_e32 vcc_lo, 0.5, v[4:5]
	v_add_f64 v[6:7], -v[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_mov_b32 v8, 0x3ff00000 :: v_dual_cndmask_b32 v3, v7, v3
	v_cndmask_b32_e32 v2, v6, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0xbff00000, v8, vcc_lo
	v_add_f64 v[12:13], -v[2:3], 1.0
	v_frexp_mant_f64_e32 v[22:23], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_eq_f64_e64 s0, 1.0, v[12:13]
	v_cndmask_b32_e64 v7, s5, v8, s0
	v_cndmask_b32_e64 v6, s4, 0, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[6:7]
	v_cndmask_b32_e32 v9, 0x3ff00000, v13, vcc_lo
	v_cndmask_b32_e32 v8, 0, v12, vcc_lo
	v_frexp_mant_f64_e64 v[10:11], |v[8:9]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[10:11]
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	v_ldexp_f64 v[14:15], v[10:11], v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], 1.0
	v_add_f64 v[24:25], v[14:15], -1.0
	v_rcp_f64_e32 v[10:11], v[16:17]
	v_add_f64 v[30:31], v[16:17], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[30:31]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[16:17], v[10:11], 1.0
	v_fma_f64 v[10:11], v[18:19], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], -v[16:17], v[10:11], 1.0
	v_fma_f64 v[18:19], v[18:19], v[10:11], v[10:11]
	v_cndmask_b32_e64 v11, v21, 0x3ff00000, s0
	v_cndmask_b32_e64 v10, v20, 0, s0
	v_cmp_gt_f64_e64 s0, s[2:3], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s1, 0, v[10:11]
	v_cndmask_b32_e64 v4, 0, 1, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v13, 0x3ff00000, v13, s1
	v_cndmask_b32_e64 v12, 0, v12, s1
	v_ldexp_f64 v[20:21], v[22:23], v4
	v_mul_f64 v[22:23], v[24:25], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_frexp_mant_f64_e64 v[26:27], |v[12:13]|
	v_add_f64 v[28:29], v[20:21], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_mul_f64 v[32:33], v[16:17], v[22:23]
	v_add_f64 v[44:45], v[20:21], -1.0
	v_cmp_gt_f64_e64 s1, s[2:3], v[26:27]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rcp_f64_e32 v[34:35], v[28:29]
	v_fma_f64 v[16:17], v[22:23], v[16:17], -v[32:33]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0, 1, s1
	v_ldexp_f64 v[26:27], v[26:27], v4
	v_frexp_exp_i32_f64_e32 v4, v[8:9]
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[16:17], v[22:23], v[14:15], v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[30:31], -v[28:29], v[34:35], 1.0
	v_add_f64 v[36:37], v[26:27], 1.0
	v_add_f64 v[46:47], v[26:27], -1.0
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[30:31], v[34:35], v[34:35]
	v_add_f64 v[34:35], v[32:33], v[16:17]
	v_rcp_f64_e32 v[30:31], v[36:37]
	v_fma_f64 v[38:39], -v[28:29], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[42:43], v[24:25], -v[34:35]
	v_add_f64 v[32:33], v[34:35], -v[32:33]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[40:41], -v[36:37], v[30:31], 1.0
	v_fma_f64 v[38:39], v[38:39], v[14:15], v[14:15]
	v_add_f64 v[24:25], v[24:25], -v[42:43]
	v_add_f64 v[16:17], v[32:33], -v[16:17]
	v_add_f64 v[32:33], v[28:29], -1.0
	v_fma_f64 v[30:31], v[40:41], v[30:31], v[30:31]
	v_mul_f64 v[14:15], v[44:45], v[38:39]
	v_add_f64 v[24:25], v[24:25], -v[34:35]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[32:33]
	v_fma_f64 v[40:41], -v[36:37], v[30:31], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[34:35], v[28:29], v[14:15]
	v_add_f64 v[16:17], v[16:17], v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[30:31], v[40:41], v[30:31], v[30:31]
	v_fma_f64 v[24:25], v[14:15], v[28:29], -v[34:35]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[42:43], v[16:17]
	v_mul_f64 v[28:29], v[46:47], v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[20:21], v[14:15], v[20:21], v[24:25]
	v_add_f64 v[24:25], v[36:37], -1.0
	v_mul_f64 v[16:17], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[32:33], v[36:37], v[28:29]
	v_add_f64 v[18:19], v[34:35], v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[26:27], -v[24:25]
	v_fma_f64 v[26:27], v[28:29], v[36:37], -v[32:33]
	v_add_f64 v[36:37], v[22:23], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[40:41], v[44:45], -v[18:19]
	v_fma_f64 v[24:25], v[28:29], v[24:25], v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[22:23], v[36:37], -v[22:23]
	v_add_f64 v[26:27], v[18:19], -v[34:35]
	v_add_f64 v[34:35], v[44:45], -v[40:41]
	v_mul_f64 v[44:45], v[36:37], v[36:37]
	v_add_f64 v[42:43], v[32:33], v[24:25]
	v_add_f64 v[16:17], v[16:17], -v[22:23]
	v_add_f64 v[20:21], v[26:27], -v[20:21]
	v_add_f64 v[18:19], v[34:35], -v[18:19]
	v_fma_f64 v[26:27], v[36:37], v[36:37], -v[44:45]
	v_add_f64 v[22:23], v[46:47], -v[42:43]
	v_add_f64 v[34:35], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[20:21], v[18:19]
	v_add_f64 v[20:21], v[42:43], -v[32:33]
	v_add_f64 v[32:33], v[46:47], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[26:27], v[36:37], v[34:35], v[26:27]
	v_add_f64 v[18:19], v[40:41], v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[24:25]
	v_add_f64 v[24:25], v[32:33], -v[42:43]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[32:33], v[44:45], v[26:27]
	v_mul_f64 v[18:19], v[38:39], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], v[24:25]
	v_fma_f64 v[24:25], v[32:33], s[14:15], s[12:13]
	v_add_f64 v[44:45], v[32:33], -v[44:45]
	v_mul_f64 v[52:53], v[36:37], v[32:33]
	v_add_f64 v[34:35], v[14:15], v[18:19]
	v_add_f64 v[20:21], v[22:23], v[20:21]
	v_fma_f64 v[22:23], v[32:33], v[24:25], s[16:17]
	v_add_f64 v[26:27], v[26:27], -v[44:45]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_mul_f64 v[24:25], v[34:35], v[34:35]
	v_add_f64 v[14:15], v[34:35], -v[14:15]
	v_mul_f64 v[20:21], v[30:31], v[20:21]
	v_fma_f64 v[22:23], v[32:33], v[22:23], s[18:19]
	v_fma_f64 v[30:31], v[24:25], s[22:23], s[20:21]
	s_mov_b32 s20, 0xcfa74449
	s_mov_b32 s21, 0x3fc3b13b
	s_mov_b32 s22, 0xd7f4df2e
	s_mov_b32 s23, 0x3fc7474d
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[18:19], -v[14:15]
	v_add_f64 v[38:39], v[28:29], v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[22:23], v[32:33], v[22:23], s[20:21]
	v_fma_f64 v[30:31], v[24:25], v[30:31], s[22:23]
	s_mov_b32 s22, 0x71bf3c30
	s_mov_b32 s23, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[14:15], v[14:15], 1
	v_add_f64 v[28:29], v[38:39], -v[28:29]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_4)
	v_fma_f64 v[22:23], v[32:33], v[22:23], s[22:23]
	v_mul_f64 v[40:41], v[38:39], v[38:39]
	v_fma_f64 v[30:31], v[24:25], v[30:31], s[24:25]
	s_mov_b32 s25, 0x3fd24924
	s_mov_b32 s24, 0x9b27acf1
	v_add_f64 v[20:21], v[20:21], -v[28:29]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], v[32:33], v[22:23], s[26:27]
	v_fma_f64 v[28:29], v[24:25], v[30:31], s[24:25]
	s_mov_b32 s24, 0x924920da
	v_fma_f64 v[30:31], v[38:39], v[38:39], -v[40:41]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[42:43], v[20:21], v[20:21]
	v_fma_f64 v[22:23], v[32:33], v[22:23], s[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[28:29]
	s_mov_b32 s28, 0x9999999c
	v_fma_f64 v[30:31], v[38:39], v[42:43], v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[32:33], v[22:23], s[28:29]
	v_add_f64 v[42:43], v[40:41], v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[46:47], v[32:33], v[22:23]
	v_fma_f64 v[48:49], v[42:43], s[14:15], s[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[44:45], v[32:33], v[22:23], -v[46:47]
	s_mov_b32 s13, 0xbfe55555
	s_mov_b32 s12, s2
	s_mov_b32 s14, 0xd5df274d
	s_mov_b32 s15, 0x3c8543b0
	v_add_f64 v[40:41], v[42:43], -v[40:41]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[48:49], v[42:43], v[48:49], s[16:17]
	v_fma_f64 v[22:23], v[26:27], v[22:23], v[44:45]
	s_mov_b32 s16, 0x652b82fe
	s_mov_b32 s17, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[30:31], v[30:31], -v[40:41]
	v_fma_f64 v[44:45], v[42:43], v[48:49], s[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[48:49], v[46:47], v[22:23]
	s_mov_b32 s19, 0xbfe62e42
	v_fma_f64 v[44:45], v[42:43], v[44:45], s[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[50:51], v[48:49], s[2:3]
	v_add_f64 v[46:47], v[48:49], -v[46:47]
	s_mov_b32 s21, 0xbc7abc9e
	v_fma_f64 v[44:45], v[42:43], v[44:45], s[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[54:55], v[50:51], s[12:13]
	v_add_f64 v[22:23], v[22:23], -v[46:47]
	v_fma_f64 v[46:47], v[32:33], v[36:37], -v[52:53]
	s_mov_b32 s22, 0xfca7ab0c
	s_mov_b32 s23, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[44:45], v[42:43], v[44:45], s[26:27]
	v_add_f64 v[48:49], v[48:49], -v[54:55]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[22:23], v[22:23], s[14:15]
	v_fma_f64 v[32:33], v[32:33], v[16:17], v[46:47]
	v_ldexp_f64 v[16:17], v[16:17], 1
	s_mov_b32 s26, 0x623fde64
	s_mov_b32 s27, 0x3ec71dee
	v_fma_f64 v[44:45], v[42:43], v[44:45], s[24:25]
	s_mov_b32 s24, 0x6a5dcb37
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[22:23], v[22:23], v[48:49]
	v_fma_f64 v[26:27], v[26:27], v[36:37], v[32:33]
	v_ldexp_f64 v[36:37], v[36:37], 1
	s_mov_b32 s25, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[32:33], v[42:43], v[44:45], s[28:29]
	s_mov_b32 s28, 0x7c89e6b0
	v_add_f64 v[44:45], v[50:51], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[46:47], v[52:53], v[26:27]
	s_mov_b32 s29, 0x3efa0199
	v_mul_f64 v[48:49], v[42:43], v[32:33]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[50:51], v[50:51], -v[44:45]
	v_mul_f64 v[54:55], v[46:47], v[44:45]
	v_add_f64 v[52:53], v[46:47], -v[52:53]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[40:41], v[42:43], v[32:33], -v[48:49]
	v_add_f64 v[22:23], v[22:23], v[50:51]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[50:51], v[46:47], v[44:45], -v[54:55]
	v_add_f64 v[26:27], v[26:27], -v[52:53]
	v_cvt_f64_i32_e32 v[52:53], v4
	v_frexp_exp_i32_f64_e32 v4, v[12:13]
	v_fma_f64 v[32:33], v[30:31], v[32:33], v[40:41]
	v_fma_f64 v[22:23], v[46:47], v[22:23], v[50:51]
	v_mul_f64 v[46:47], v[38:39], v[42:43]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, s1
	v_add_f64 v[40:41], v[48:49], v[32:33]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[26:27], v[44:45], v[22:23]
	v_add_f64 v[26:27], v[40:41], s[2:3]
	v_add_f64 v[44:45], v[40:41], -v[48:49]
	s_mov_b32 s2, 0x55555780
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[48:49], v[54:55], v[22:23]
	v_add_f64 v[50:51], v[26:27], s[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[32:33], v[32:33], -v[44:45]
	v_fma_f64 v[44:45], v[42:43], v[38:39], -v[46:47]
	v_add_f64 v[56:57], v[36:37], v[48:49]
	v_add_f64 v[54:55], v[48:49], -v[54:55]
	s_mov_b32 s12, 0xfefa39ef
	s_mov_b32 s13, 0x3fe62e42
	s_mov_b32 s18, s12
	v_add_f64 v[40:41], v[40:41], -v[50:51]
	v_add_f64 v[32:33], v[32:33], s[14:15]
	v_fma_f64 v[42:43], v[42:43], v[20:21], v[44:45]
	v_mul_f64 v[44:45], v[52:53], s[12:13]
	v_add_f64 v[36:37], v[56:57], -v[36:37]
	v_add_f64 v[22:23], v[22:23], -v[54:55]
	s_mov_b32 s14, 0x3b39803f
	s_mov_b32 s15, 0x3c7abc9e
	v_ldexp_f64 v[20:21], v[20:21], 1
	s_mov_b32 s20, s14
	v_add_f64 v[32:33], v[32:33], v[40:41]
	v_fma_f64 v[30:31], v[30:31], v[38:39], v[42:43]
	v_fma_f64 v[40:41], v[52:53], s[12:13], -v[44:45]
	v_add_f64 v[36:37], v[48:49], -v[36:37]
	v_add_f64 v[16:17], v[16:17], v[22:23]
	v_ldexp_f64 v[38:39], v[38:39], 1
	v_add_f64 v[22:23], v[26:27], v[32:33]
	v_add_f64 v[42:43], v[46:47], v[30:31]
	v_fma_f64 v[40:41], v[52:53], s[14:15], v[40:41]
	v_add_f64 v[16:17], v[16:17], v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[26:27], -v[22:23]
	v_mul_f64 v[36:37], v[42:43], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_4)
	v_add_f64 v[48:49], v[44:45], v[40:41]
	v_add_f64 v[46:47], v[42:43], -v[46:47]
	v_add_f64 v[50:51], v[56:57], v[16:17]
	v_add_f64 v[26:27], v[32:33], v[26:27]
	v_fma_f64 v[32:33], v[42:43], v[22:23], -v[36:37]
	v_add_f64 v[30:31], v[30:31], -v[46:47]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[52:53], v[48:49], v[50:51]
	v_fma_f64 v[26:27], v[42:43], v[26:27], v[32:33]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[32:33], v[52:53], -v[48:49]
	v_fma_f64 v[22:23], v[30:31], v[22:23], v[26:27]
	v_add_f64 v[26:27], v[48:49], -v[44:45]
	v_add_f64 v[30:31], v[50:51], -v[56:57]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[42:43], v[52:53], -v[32:33]
	v_add_f64 v[44:45], v[36:37], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[40:41], -v[26:27]
	v_add_f64 v[16:17], v[16:17], -v[30:31]
	v_add_f64 v[30:31], v[50:51], -v[32:33]
	v_add_f64 v[32:33], v[48:49], -v[42:43]
	v_cvt_f64_i32_e32 v[40:41], v4
	v_add_f64 v[42:43], v[38:39], v[44:45]
	v_add_f64 v[36:37], v[44:45], -v[36:37]
	v_add_f64 v[46:47], v[26:27], v[16:17]
	v_add_f64 v[30:31], v[30:31], v[32:33]
	v_mul_f64 v[32:33], v[40:41], s[12:13]
	v_add_f64 v[38:39], v[42:43], -v[38:39]
	v_add_f64 v[22:23], v[22:23], -v[36:37]
	v_add_f64 v[36:37], v[46:47], -v[26:27]
	v_add_f64 v[30:31], v[46:47], v[30:31]
	v_fma_f64 v[48:49], v[40:41], s[12:13], -v[32:33]
	v_add_f64 v[38:39], v[44:45], -v[38:39]
	v_add_f64 v[20:21], v[20:21], v[22:23]
	v_add_f64 v[22:23], v[46:47], -v[36:37]
	v_add_f64 v[16:17], v[16:17], -v[36:37]
	v_add_f64 v[44:45], v[52:53], v[30:31]
	v_fma_f64 v[40:41], v[40:41], s[14:15], v[48:49]
	v_add_f64 v[20:21], v[20:21], v[38:39]
	v_add_f64 v[22:23], v[26:27], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[44:45], -v[52:53]
	v_add_f64 v[36:37], v[32:33], v[40:41]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[38:39], v[42:43], v[20:21]
	v_add_f64 v[16:17], v[16:17], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[22:23], v[30:31], -v[26:27]
	v_add_f64 v[32:33], v[36:37], -v[32:33]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[36:37], v[38:39]
	v_add_f64 v[42:43], v[38:39], -v[42:43]
	v_add_f64 v[16:17], v[16:17], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[32:33], v[40:41], -v[32:33]
	v_add_f64 v[22:23], v[26:27], -v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[42:43]
	v_add_f64 v[30:31], v[44:45], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[46:47], v[26:27], -v[22:23]
	v_add_f64 v[22:23], v[38:39], -v[22:23]
	v_add_f64 v[38:39], v[32:33], v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[44:45], v[30:31], -v[44:45]
	v_mul_f64 v[48:49], v[6:7], v[30:31]
	v_add_f64 v[36:37], v[36:37], -v[46:47]
	v_add_f64 v[16:17], v[16:17], -v[44:45]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[30:31], v[6:7], v[30:31], -v[48:49]
	v_cmp_class_f64_e64 vcc_lo, v[48:49], 0x204
	v_add_f64 v[22:23], v[22:23], v[36:37]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[6:7], v[16:17], v[30:31]
	v_add_f64 v[30:31], v[38:39], -v[32:33]
	v_add_f64 v[22:23], v[38:39], v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[36:37], v[48:49], v[16:17]
	v_add_f64 v[38:39], v[38:39], -v[30:31]
	v_add_f64 v[20:21], v[20:21], -v[30:31]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[40:41], v[26:27], v[22:23]
	v_dual_cndmask_b32 v43, v37, v49 :: v_dual_cndmask_b32 v42, v36, v48
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[30:31], v[32:33], -v[38:39]
	v_mul_f64 v[44:45], v[42:43], s[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[40:41], -v[26:27]
	v_add_f64 v[20:21], v[20:21], v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rndne_f64_e32 v[32:33], v[44:45]
	v_add_f64 v[22:23], v[22:23], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[26:27], v[32:33], s[18:19], v[42:43]
	v_cvt_i32_f64_e32 v4, v[32:33]
	v_add_f64 v[20:21], v[20:21], v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[32:33], s[20:21], v[26:27]
	v_add_f64 v[26:27], v[40:41], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[30:31], v[22:23], s[24:25], s[22:23]
	v_add_f64 v[38:39], v[26:27], -v[40:41]
	v_mul_f64 v[40:41], v[10:11], v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[30:31], v[22:23], v[30:31], s[26:27]
	v_add_f64 v[20:21], v[20:21], -v[38:39]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[26:27], v[10:11], v[26:27], -v[40:41]
	v_cmp_class_f64_e64 vcc_lo, v[40:41], 0x204
	v_fma_f64 v[30:31], v[22:23], v[30:31], s[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[10:11], v[20:21], v[26:27]
	v_fma_f64 v[26:27], v[22:23], v[30:31], s[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[30:31], v[40:41], v[20:21]
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_dual_cndmask_b32 v39, v31, v41 :: v_dual_cndmask_b32 v38, v30, v40
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[42:43]|
	v_add_f64 v[30:31], v[30:31], -v[40:41]
	v_mul_f64 v[40:41], v[10:11], 0.5
	v_mul_f64 v[44:45], v[38:39], s[16:17]
	s_mov_b32 s16, 0x555502a1
	s_mov_b32 s17, 0x3fa55555
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[30:31]
	v_trunc_f64_e32 v[30:31], v[10:11]
	v_rndne_f64_e32 v[44:45], v[44:45]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[16:17]
	v_fma_f64 v[46:47], v[44:45], s[18:19], v[38:39]
	s_mov_b32 s18, 11
	s_mov_b32 s19, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[38:39]
	v_fma_f64 v[46:47], v[44:45], s[20:21], v[46:47]
	v_cmp_lt_f64_e64 s20, |v[8:9]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[18:19]
	v_fma_f64 v[50:51], v[46:47], s[24:25], s[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], 1.0
	v_fma_f64 v[50:51], v[46:47], v[50:51], s[26:27]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[22:23], v[22:23], v[26:27], 1.0
	v_mul_f64 v[26:27], v[34:35], v[24:25]
	v_fma_f64 v[24:25], v[24:25], v[28:29], s[2:3]
	v_add_f64 v[28:29], v[36:37], -v[48:49]
	v_ldexp_f64 v[48:49], v[34:35], 1
	v_mul_f64 v[36:37], v[6:7], 0.5
	v_cmp_neq_f64_e64 s3, v[6:7], |v[6:7]|
	v_fma_f64 v[32:33], v[46:47], v[50:51], s[28:29]
	v_ldexp_f64 v[22:23], v[22:23], v4
	v_frexp_exp_i32_f64_e32 v4, v[2:3]
	v_mul_f64 v[24:25], v[26:27], v[24:25]
	v_add_f64 v[16:17], v[16:17], -v[28:29]
	v_trunc_f64_e32 v[28:29], v[6:7]
	s_xor_b32 s3, s3, s20
	v_fma_f64 v[26:27], v[46:47], v[32:33], s[30:31]
	v_trunc_f64_e32 v[32:33], v[36:37]
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, s0
	v_cndmask_b32_e32 v17, 0, v17, vcc_lo
	v_cmp_nlt_f64_e64 s1, 0x40900000, v[42:43]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[42:43]
	s_delay_alu instid0(VALU_DEP_4)
	v_cvt_f64_i32_e32 v[42:43], v4
	v_add_f64 v[34:35], v[48:49], v[24:25]
	v_cndmask_b32_e32 v16, 0, v16, vcc_lo
	v_fma_f64 v[26:27], v[46:47], v[26:27], s[34:35]
	v_cmp_neq_f64_e64 s0, v[32:33], v[36:37]
	v_cndmask_b32_e64 v23, 0x7ff00000, v23, s1
	s_and_b32 vcc_lo, s2, s1
	v_cndmask_b32_e32 v22, 0, v22, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[28:29], v[6:7]
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v23, 0, v23, s2
	v_mul_f64 v[28:29], v[42:43], s[12:13]
	v_add_f64 v[18:19], v[34:35], -v[48:49]
	v_cmp_eq_f64_e64 s2, 0, v[8:9]
	v_cvt_i32_f64_e32 v48, v[44:45]
	v_fma_f64 v[16:17], v[22:23], v[16:17], v[22:23]
	v_cmp_class_f64_e64 s1, v[22:23], 0x204
	v_fma_f64 v[26:27], v[46:47], v[26:27], s[36:37]
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e64 v4, 0x3ff00000, v9, s0
	v_add_f64 v[18:19], v[24:25], -v[18:19]
	v_cndmask_b32_e64 v32, v16, v22, s1
	v_cndmask_b32_e64 v22, v17, v23, s1
	v_fma_f64 v[16:17], v[42:43], s[12:13], -v[28:29]
	v_cmp_gt_f64_e64 s1, 0, v[6:7]
	v_cmp_class_f64_e64 s12, v[8:9], 0x204
	v_cndmask_b32_e32 v25, 0, v32, vcc_lo
	v_bfi_b32 v4, 0x7fffffff, v22, v4
	v_cmp_lt_f64_e64 s13, |v[12:13]|, 1.0
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v24, 0x7ff80000, v4, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[8:9]
	v_fma_f64 v[22:23], v[46:47], v[26:27], s[16:17]
	v_cndmask_b32_e64 v26, 0x7ff00000, 0, s3
	v_cmp_neq_f64_e64 s3, |v[8:9]|, 1.0
	v_add_f64 v[14:15], v[14:15], v[18:19]
	v_fma_f64 v[16:17], v[42:43], s[14:15], v[16:17]
	s_xor_b32 s1, s1, s2
	v_cndmask_b32_e32 v27, v32, v25, vcc_lo
	v_cndmask_b32_e32 v4, v4, v24, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[6:7], 0x204
	v_fma_f64 v[18:19], v[46:47], v[22:23], s[38:39]
	v_cndmask_b32_e64 v22, 0x7ff00000, 0, s1
	v_cndmask_b32_e64 v23, 0, v9, s0
	v_cndmask_b32_e64 v26, 0x3ff00000, v26, s3
	s_or_b32 s0, s2, s12
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[38:39]
	v_cmp_neq_f64_e64 s12, v[10:11], |v[10:11]|
	v_bfi_b32 v32, 0x7fffffff, v22, v23
	v_cmp_eq_f64_e64 s2, 0, v[12:13]
	v_cmp_gt_f64_e64 s3, 0, v[10:11]
	v_add_f64 v[24:25], v[34:35], v[14:15]
	v_add_f64 v[22:23], v[28:29], v[16:17]
	v_cndmask_b32_e32 v4, v4, v26, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v26, v4, v32, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[8:9], v[6:7]
	v_fma_f64 v[6:7], v[46:47], v[18:19], s[18:19]
	v_cndmask_b32_e64 v27, v27, 0, s0
	v_mov_b32_e32 v4, 0
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[38:39]
	s_xor_b32 s12, s12, s13
	s_xor_b32 s3, s3, s2
	v_add_f64 v[34:35], v[24:25], -v[34:35]
	v_add_f64 v[8:9], v[22:23], v[24:25]
	v_add_f64 v[28:29], v[22:23], -v[28:29]
	v_cndmask_b32_e32 v18, 0, v27, vcc_lo
	v_cndmask_b32_e32 v19, 0x7ff80000, v26, vcc_lo
	v_fma_f64 v[6:7], v[46:47], v[6:7], 1.0
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[38:39]|
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[26:27], v[4:5], v[18:19]
	v_add_f64 v[14:15], v[14:15], -v[34:35]
	v_add_f64 v[32:33], v[8:9], -v[22:23]
	v_add_f64 v[16:17], v[16:17], -v[28:29]
	v_fma_f64 v[6:7], v[46:47], v[6:7], 1.0
	v_dual_cndmask_b32 v21, 0, v21 :: v_dual_cndmask_b32 v20, 0, v20
	s_and_b32 vcc_lo, s1, s0
	v_div_scale_f64 v[36:37], null, v[2:3], v[2:3], v[26:27]
	v_add_f64 v[42:43], v[8:9], -v[32:33]
	v_add_f64 v[24:25], v[24:25], -v[32:33]
	v_trunc_f64_e32 v[32:33], v[40:41]
	v_add_f64 v[34:35], v[16:17], v[14:15]
	v_ldexp_f64 v[6:7], v[6:7], v48
	v_rcp_f64_e32 v[44:45], v[36:37]
	v_add_f64 v[22:23], v[22:23], -v[42:43]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v7, 0x7ff00000, v7, s0
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[30:31], v[10:11]
	v_cmp_neq_f64_e64 s0, v[32:33], v[40:41]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(TRANS32_DEP_1)
	v_cndmask_b32_e64 v7, 0, v7, s1
	v_fma_f64 v[28:29], -v[36:37], v[44:45], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[6:7], v[20:21], v[6:7]
	v_cmp_class_f64_e64 s1, v[6:7], 0x204
	v_add_f64 v[22:23], v[24:25], v[22:23]
	s_and_b32 s0, vcc_lo, s0
	v_cndmask_b32_e64 v32, 0x3ff00000, v13, s0
	v_fma_f64 v[24:25], v[44:45], v[28:29], v[44:45]
	v_add_f64 v[28:29], v[34:35], -v[16:17]
	v_cndmask_b32_e64 v33, v20, v6, s1
	v_cndmask_b32_e64 v6, v21, v7, s1
	v_add_f64 v[22:23], v[34:35], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_bfi_b32 v32, 0x7fffffff, v6, v32
	v_fma_f64 v[30:31], -v[36:37], v[24:25], 1.0
	v_add_f64 v[6:7], v[34:35], -v[28:29]
	v_cndmask_b32_e32 v34, 0x7ff80000, v32, vcc_lo
	v_cndmask_b32_e32 v35, 0, v33, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[12:13]
	v_add_f64 v[14:15], v[14:15], -v[28:29]
	v_add_f64 v[20:21], v[8:9], v[22:23]
	v_fma_f64 v[24:25], v[24:25], v[30:31], v[24:25]
	v_div_scale_f64 v[30:31], s1, v[26:27], v[2:3], v[26:27]
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_dual_cndmask_b32 v28, v33, v35 :: v_dual_cndmask_b32 v29, v32, v34
	v_cmp_neq_f64_e64 vcc_lo, |v[12:13]|, 1.0
	v_cndmask_b32_e64 v16, 0x7ff00000, 0, s12
	v_cmp_class_f64_e64 s12, v[12:13], 0x204
	v_cndmask_b32_e64 v33, 0x7ff00000, 0, s3
	v_cndmask_b32_e64 v34, 0, v13, s0
	v_add_f64 v[8:9], v[20:21], -v[8:9]
	v_add_f64 v[6:7], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_3)
	v_bfi_b32 v15, 0x7fffffff, v33, v34
	v_cndmask_b32_e32 v32, 0x3ff00000, v16, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[10:11], 0x204
	v_mul_f64 v[16:17], v[30:31], v[24:25]
	s_or_b32 s0, s2, s12
	v_add_f64 v[8:9], v[22:23], -v[8:9]
	v_cndmask_b32_e32 v14, v29, v32, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_cndmask_b32_e64 v14, v14, v15, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[12:13], v[10:11]
	v_fma_f64 v[10:11], -v[36:37], v[16:17], v[30:31]
	v_cndmask_b32_e64 v15, v28, 0, s0
	v_add_f64 v[6:7], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v8, 0, v15, vcc_lo
	v_cndmask_b32_e32 v9, 0x7ff80000, v14, vcc_lo
	s_mov_b32 vcc_lo, s1
	s_load_b64 s[0:1], s[6:7], 0x0
	v_div_fmas_f64 v[10:11], v[10:11], v[24:25], v[16:17]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_mul_f64 v[8:9], s[4:5], v[8:9]
	v_dual_cndmask_b32 v6, v6, v2 :: v_dual_cndmask_b32 v7, v7, v3
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	v_div_fixup_f64 v[8:9], v[10:11], v[2:3], v[26:27]
	v_cndmask_b32_e32 v7, 0x7ff80000, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[2:3], s[0:1], -v[18:19]
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[4:5], -v[6:7], v[4:5], v[8:9]
	v_mul_f64 v[2:3], v[6:7], v[2:3]
	v_add_co_u32 v6, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s9, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s10, v0
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	v_mul_f64 v[4:5], v[4:5], -s[0:1]
	global_store_b64 v[6:7], v[2:3], off
	global_store_b64 v[0:1], v[4:5], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel focal_loss_kernel
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
		.amdhsa_next_free_vgpr 58
		.amdhsa_next_free_sgpr 40
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
		.amdhsa_inst_pref_size 40
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
	.size	focal_loss_kernel, .Lfunc_end3-focal_loss_kernel
                                        ; -- End function
	.set focal_loss_kernel.num_vgpr, 58
	.set focal_loss_kernel.num_agpr, 0
	.set focal_loss_kernel.numbered_sgpr, 40
	.set focal_loss_kernel.num_named_barrier, 0
	.set focal_loss_kernel.private_seg_size, 0
	.set focal_loss_kernel.uses_vcc, 1
	.set focal_loss_kernel.uses_flat_scratch, 0
	.set focal_loss_kernel.has_dyn_sized_stack, 0
	.set focal_loss_kernel.has_recursion, 0
	.set focal_loss_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 5032
; TotalNumSgprs: 42
; NumVgprs: 58
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 7
; NumSGPRsForWavesPerEU: 42
; NumVGPRsForWavesPerEU: 58
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
	.protected	focal_grad_kernel       ; -- Begin function focal_grad_kernel
	.globl	focal_grad_kernel
	.p2align	8
	.type	focal_grad_kernel,@function
focal_grad_kernel:                      ; @focal_grad_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
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
	s_load_b64 s[8:9], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_movk_i32 s2, 0xdcd1
	s_mov_b32 s3, 0x3fefffff
	s_mov_b32 s12, 0x968915a9
	s_mov_b32 s14, 0x4222de17
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s13, 0x3fba6564
	s_mov_b32 s15, 0x3fbdee67
	s_mov_b32 s16, 0x3abe935a
	s_mov_b32 s17, 0x3fbe25e4
	s_mov_b32 s18, 0x47e6c9c2
	s_mov_b32 s19, 0x3fc110ef
	s_mov_b32 s20, 0x6b47b09a
	s_mov_b32 s22, 0xbf559e2b
	s_mov_b32 s21, 0x3fc38538
	s_mov_b32 s23, 0x3fc3ab76
	s_mov_b32 s24, 0x16291751
	s_mov_b32 s25, 0x3fcc71c0
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_load_b128 s[4:7], s[0:1], 0x20
	s_mov_b32 s26, 0x1c7792ce
	global_load_b64 v[5:6], v[4:5], off
	s_mov_b32 s27, 0x3fcc71c7
	s_mov_b32 s29, 0x3fd99999
	s_mov_b32 s28, 0x998ef7b6
	s_mov_b32 s30, 0x7c89e6b0
	s_mov_b32 s31, 0x3efa0199
	s_mov_b32 s34, 0x14761f6e
	s_mov_b32 s35, 0x3f2a01a0
	s_mov_b32 s36, 0x1852b7b0
	s_mov_b32 s37, 0x3f56c16c
	s_mov_b32 s38, 0x11122322
	s_mov_b32 s39, 0x3f811111
	s_load_b64 s[10:11], s[0:1], 0x30
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[4:5], s[4:5], 0x0
	s_mov_b32 s40, 0x555502a1
	s_mov_b32 s41, 0x3fa55555
	s_waitcnt vmcnt(1)
	v_cmp_nlt_f64_e32 vcc_lo, s[2:3], v[2:3]
	s_mov_b32 s2, 0x812dea11
	s_mov_b32 s3, 0x3d719799
	v_cndmask_b32_e32 v4, 0xffffdcd1, v2, vcc_lo
	v_cndmask_b32_e32 v7, 0x3fefffff, v3, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, s[2:3], v[2:3]
	s_mov_b32 s2, 0x55555555
	s_mov_b32 s3, 0x3fe55555
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, 0x3d719799, v7, vcc_lo
	v_cndmask_b32_e32 v2, 0x812dea11, v4, vcc_lo
	s_waitcnt vmcnt(0)
	v_cmp_lt_f64_e32 vcc_lo, 0.5, v[5:6]
	v_add_f64 v[7:8], -v[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_mov_b32 v4, 0x3ff00000 :: v_dual_cndmask_b32 v3, v8, v3
	v_cndmask_b32_e32 v2, v7, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v5, 0xbff00000, v4, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[6:7], s[4:5], -1.0
	v_add_f64 v[8:9], -v[2:3], 1.0
	v_frexp_mant_f64_e32 v[22:23], v[2:3]
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_eq_f64_e32 vcc_lo, 1.0, v[8:9]
	v_cndmask_b32_e32 v11, s5, v4, vcc_lo
	v_cndmask_b32_e64 v10, s4, 0, vcc_lo
	v_cndmask_b32_e64 v7, v7, 0x3ff00000, vcc_lo
	v_cndmask_b32_e64 v6, v6, 0, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_neq_f64_e64 s0, 0, v[10:11]
	v_cndmask_b32_e64 v13, 0x3ff00000, v9, s0
	v_cndmask_b32_e64 v12, 0, v8, s0
	v_cmp_neq_f64_e64 s0, 0, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_frexp_mant_f64_e64 v[14:15], |v[12:13]|
	v_cndmask_b32_e64 v9, 0x3ff00000, v9, s0
	v_cndmask_b32_e64 v8, 0, v8, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e64 s1, s[2:3], v[14:15]
	v_frexp_mant_f64_e64 v[26:27], |v[8:9]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v4, 0, 1, s1
	v_cmp_gt_f64_e64 s0, s[2:3], v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[14:15], v[14:15], v4
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	v_add_f64 v[16:17], v[14:15], 1.0
	v_add_f64 v[24:25], v[14:15], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[18:19], v[16:17]
	v_add_f64 v[30:31], v[16:17], -1.0
	v_add_f64 v[14:15], v[14:15], -v[30:31]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[20:21], v[18:19], v[18:19]
	v_fma_f64 v[20:21], -v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[20:21], v[18:19], v[18:19]
	v_ldexp_f64 v[20:21], v[22:23], v4
	v_cndmask_b32_e64 v4, 0, 1, s0
	v_ldexp_f64 v[26:27], v[26:27], v4
	v_frexp_exp_i32_f64_e32 v4, v[12:13]
	v_mul_f64 v[22:23], v[24:25], v[18:19]
	v_add_f64 v[28:29], v[20:21], 1.0
	v_add_f64 v[44:45], v[20:21], -1.0
	v_add_f64 v[36:37], v[26:27], 1.0
	v_add_f64 v[46:47], v[26:27], -1.0
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, s1
	v_mul_f64 v[32:33], v[16:17], v[22:23]
	v_rcp_f64_e32 v[34:35], v[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[22:23], v[16:17], -v[32:33]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[30:31], -v[28:29], v[34:35], 1.0
	v_fma_f64 v[16:17], v[22:23], v[14:15], v[16:17]
	v_fma_f64 v[14:15], v[30:31], v[34:35], v[34:35]
	v_rcp_f64_e32 v[30:31], v[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[34:35], v[32:33], v[16:17]
	v_fma_f64 v[38:39], -v[28:29], v[14:15], 1.0
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[40:41], -v[36:37], v[30:31], 1.0
	v_add_f64 v[42:43], v[24:25], -v[34:35]
	v_add_f64 v[32:33], v[34:35], -v[32:33]
	v_fma_f64 v[38:39], v[38:39], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[30:31], v[40:41], v[30:31], v[30:31]
	v_add_f64 v[24:25], v[24:25], -v[42:43]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[32:33], -v[16:17]
	v_add_f64 v[32:33], v[28:29], -1.0
	v_mul_f64 v[14:15], v[44:45], v[38:39]
	v_fma_f64 v[40:41], -v[36:37], v[30:31], 1.0
	v_add_f64 v[24:25], v[24:25], -v[34:35]
	v_add_f64 v[20:21], v[20:21], -v[32:33]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[34:35], v[28:29], v[14:15]
	v_fma_f64 v[30:31], v[40:41], v[30:31], v[30:31]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[16:17], v[24:25]
	v_fma_f64 v[24:25], v[14:15], v[28:29], -v[34:35]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[28:29], v[46:47], v[30:31]
	v_add_f64 v[16:17], v[42:43], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[20:21], v[14:15], v[20:21], v[24:25]
	v_add_f64 v[24:25], v[36:37], -1.0
	v_mul_f64 v[32:33], v[36:37], v[28:29]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[16:17], v[18:19], v[16:17]
	v_add_f64 v[18:19], v[34:35], v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[24:25], v[26:27], -v[24:25]
	v_fma_f64 v[26:27], v[28:29], v[36:37], -v[32:33]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[36:37], v[22:23], v[16:17]
	v_add_f64 v[40:41], v[44:45], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[24:25], v[28:29], v[24:25], v[26:27]
	v_add_f64 v[26:27], v[18:19], -v[34:35]
	v_add_f64 v[22:23], v[36:37], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_add_f64 v[34:35], v[44:45], -v[40:41]
	v_mul_f64 v[44:45], v[36:37], v[36:37]
	v_add_f64 v[42:43], v[32:33], v[24:25]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	v_add_f64 v[16:17], v[26:27], -v[20:21]
	v_add_f64 v[18:19], v[34:35], -v[18:19]
	v_fma_f64 v[26:27], v[36:37], v[36:37], -v[44:45]
	v_add_f64 v[20:21], v[46:47], -v[42:43]
	v_add_f64 v[34:35], v[22:23], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[16:17], v[18:19]
	v_add_f64 v[18:19], v[42:43], -v[32:33]
	v_add_f64 v[32:33], v[46:47], -v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[26:27], v[36:37], v[34:35], v[26:27]
	v_add_f64 v[16:17], v[40:41], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_add_f64 v[24:25], v[32:33], -v[42:43]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[32:33], v[44:45], v[26:27]
	v_mul_f64 v[16:17], v[38:39], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[18:19], v[24:25]
	v_fma_f64 v[24:25], v[32:33], s[14:15], s[12:13]
	v_add_f64 v[44:45], v[32:33], -v[44:45]
	v_mul_f64 v[52:53], v[36:37], v[32:33]
	v_add_f64 v[34:35], v[14:15], v[16:17]
	v_add_f64 v[18:19], v[20:21], v[18:19]
	v_fma_f64 v[20:21], v[32:33], v[24:25], s[16:17]
	v_add_f64 v[26:27], v[26:27], -v[44:45]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_mul_f64 v[24:25], v[34:35], v[34:35]
	v_add_f64 v[14:15], v[34:35], -v[14:15]
	v_mul_f64 v[18:19], v[30:31], v[18:19]
	v_fma_f64 v[20:21], v[32:33], v[20:21], s[18:19]
	v_fma_f64 v[30:31], v[24:25], s[22:23], s[20:21]
	s_mov_b32 s20, 0xcfa74449
	s_mov_b32 s21, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_add_f64 v[38:39], v[28:29], v[18:19]
	s_mov_b32 s22, 0xd7f4df2e
	s_mov_b32 s23, 0x3fc7474d
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	v_fma_f64 v[20:21], v[32:33], v[20:21], s[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[30:31], v[24:25], v[30:31], s[22:23]
	s_mov_b32 s22, 0x71bf3c30
	s_mov_b32 s23, 0x3fc745d1
	v_add_f64 v[28:29], v[38:39], -v[28:29]
	v_mul_f64 v[40:41], v[38:39], v[38:39]
	v_ldexp_f64 v[14:15], v[14:15], 1
	v_fma_f64 v[20:21], v[32:33], v[20:21], s[22:23]
	v_fma_f64 v[30:31], v[24:25], v[30:31], s[24:25]
	s_mov_b32 s25, 0x3fd24924
	s_mov_b32 s24, 0x9b27acf1
	v_add_f64 v[18:19], v[18:19], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[20:21], v[32:33], v[20:21], s[26:27]
	v_fma_f64 v[28:29], v[24:25], v[30:31], s[24:25]
	s_mov_b32 s24, 0x924920da
	v_fma_f64 v[30:31], v[38:39], v[38:39], -v[40:41]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[42:43], v[18:19], v[18:19]
	v_fma_f64 v[20:21], v[32:33], v[20:21], s[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[28:29], v[24:25], v[28:29], s[28:29]
	s_mov_b32 s28, 0x9999999c
	v_fma_f64 v[30:31], v[38:39], v[42:43], v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[32:33], v[20:21], s[28:29]
	v_add_f64 v[42:43], v[40:41], v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[46:47], v[32:33], v[20:21]
	v_fma_f64 v[48:49], v[42:43], s[14:15], s[12:13]
	s_mov_b32 s13, 0xbfe55555
	s_mov_b32 s12, s2
	v_add_f64 v[40:41], v[42:43], -v[40:41]
	s_mov_b32 s14, 0xfefa39ef
	s_mov_b32 s15, 0x3fe62e42
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[44:45], v[32:33], v[20:21], -v[46:47]
	v_fma_f64 v[48:49], v[42:43], v[48:49], s[16:17]
	s_mov_b32 s16, 0xd5df274d
	s_mov_b32 s17, 0x3c8543b0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[30:31], v[30:31], -v[40:41]
	v_fma_f64 v[20:21], v[26:27], v[20:21], v[44:45]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[44:45], v[42:43], v[48:49], s[18:19]
	s_mov_b32 s18, 0x55555511
	s_mov_b32 s19, 0x3fc55555
	v_add_f64 v[48:49], v[46:47], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[44:45], v[42:43], v[44:45], s[20:21]
	s_mov_b32 s21, 0xbfe62e42
	s_mov_b32 s20, s14
	v_add_f64 v[50:51], v[48:49], s[2:3]
	v_add_f64 v[46:47], v[48:49], -v[46:47]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[44:45], v[42:43], v[44:45], s[22:23]
	s_mov_b32 s23, 0xbc7abc9e
	v_add_f64 v[54:55], v[50:51], s[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[46:47]
	v_fma_f64 v[46:47], v[32:33], v[36:37], -v[52:53]
	v_fma_f64 v[44:45], v[42:43], v[44:45], s[26:27]
	s_mov_b32 s26, 0x6a5dcb37
	s_mov_b32 s27, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[48:49], v[48:49], -v[54:55]
	v_add_f64 v[20:21], v[20:21], s[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_4) | instid1(VALU_DEP_4)
	v_fma_f64 v[32:33], v[32:33], v[22:23], v[46:47]
	v_ldexp_f64 v[22:23], v[22:23], 1
	v_fma_f64 v[44:45], v[42:43], v[44:45], s[24:25]
	s_mov_b32 s24, 0xfca7ab0c
	s_mov_b32 s25, 0x3e928af3
	v_add_f64 v[20:21], v[20:21], v[48:49]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[26:27], v[26:27], v[36:37], v[32:33]
	v_ldexp_f64 v[36:37], v[36:37], 1
	v_fma_f64 v[32:33], v[42:43], v[44:45], s[28:29]
	s_mov_b32 s28, 0x623fde64
	s_mov_b32 s29, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[44:45], v[50:51], v[20:21]
	v_add_f64 v[46:47], v[52:53], v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[48:49], v[42:43], v[32:33]
	v_add_f64 v[50:51], v[50:51], -v[44:45]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_mul_f64 v[54:55], v[46:47], v[44:45]
	v_add_f64 v[52:53], v[46:47], -v[52:53]
	v_fma_f64 v[40:41], v[42:43], v[32:33], -v[48:49]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], v[50:51]
	v_fma_f64 v[50:51], v[46:47], v[44:45], -v[54:55]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[26:27], v[26:27], -v[52:53]
	v_cvt_f64_i32_e32 v[52:53], v4
	v_frexp_exp_i32_f64_e32 v4, v[8:9]
	v_fma_f64 v[32:33], v[30:31], v[32:33], v[40:41]
	v_fma_f64 v[20:21], v[46:47], v[20:21], v[50:51]
	v_mul_f64 v[46:47], v[38:39], v[42:43]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, s0
	v_add_f64 v[40:41], v[48:49], v[32:33]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[26:27], v[44:45], v[20:21]
	v_add_f64 v[26:27], v[40:41], s[2:3]
	v_add_f64 v[44:45], v[40:41], -v[48:49]
	s_mov_b32 s2, 0x55555780
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[48:49], v[54:55], v[20:21]
	v_add_f64 v[50:51], v[26:27], s[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_4)
	v_add_f64 v[32:33], v[32:33], -v[44:45]
	v_fma_f64 v[44:45], v[42:43], v[38:39], -v[46:47]
	s_mov_b32 s12, 0x3b39803f
	s_mov_b32 s13, 0x3c7abc9e
	s_mov_b32 s22, s12
	v_add_f64 v[56:57], v[36:37], v[48:49]
	v_add_f64 v[54:55], v[48:49], -v[54:55]
	v_add_f64 v[40:41], v[40:41], -v[50:51]
	v_add_f64 v[32:33], v[32:33], s[16:17]
	v_fma_f64 v[42:43], v[42:43], v[18:19], v[44:45]
	v_mul_f64 v[44:45], v[52:53], s[14:15]
	v_ldexp_f64 v[18:19], v[18:19], 1
	s_mov_b32 s16, 0x652b82fe
	s_mov_b32 s17, 0x3ff71547
	v_add_f64 v[36:37], v[56:57], -v[36:37]
	v_add_f64 v[20:21], v[20:21], -v[54:55]
	v_add_f64 v[32:33], v[32:33], v[40:41]
	v_fma_f64 v[30:31], v[30:31], v[38:39], v[42:43]
	v_fma_f64 v[40:41], v[52:53], s[14:15], -v[44:45]
	v_ldexp_f64 v[38:39], v[38:39], 1
	v_add_f64 v[36:37], v[48:49], -v[36:37]
	v_add_f64 v[20:21], v[22:23], v[20:21]
	v_add_f64 v[22:23], v[26:27], v[32:33]
	v_add_f64 v[42:43], v[46:47], v[30:31]
	v_fma_f64 v[40:41], v[52:53], s[12:13], v[40:41]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], v[36:37]
	v_add_f64 v[26:27], v[26:27], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[36:37], v[42:43], v[22:23]
	v_add_f64 v[48:49], v[44:45], v[40:41]
	v_add_f64 v[46:47], v[42:43], -v[46:47]
	v_add_f64 v[50:51], v[56:57], v[20:21]
	v_add_f64 v[26:27], v[32:33], v[26:27]
	v_fma_f64 v[32:33], v[42:43], v[22:23], -v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[30:31], v[30:31], -v[46:47]
	v_add_f64 v[52:53], v[48:49], v[50:51]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[42:43], v[26:27], v[32:33]
	v_add_f64 v[32:33], v[52:53], -v[48:49]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[22:23], v[30:31], v[22:23], v[26:27]
	v_add_f64 v[26:27], v[48:49], -v[44:45]
	v_add_f64 v[30:31], v[50:51], -v[56:57]
	v_add_f64 v[42:43], v[52:53], -v[32:33]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[44:45], v[36:37], v[22:23]
	v_add_f64 v[26:27], v[40:41], -v[26:27]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[30:31]
	v_add_f64 v[30:31], v[50:51], -v[32:33]
	v_cvt_f64_i32_e32 v[40:41], v4
	v_add_f64 v[32:33], v[48:49], -v[42:43]
	v_add_f64 v[42:43], v[38:39], v[44:45]
	v_add_f64 v[36:37], v[44:45], -v[36:37]
	v_add_f64 v[46:47], v[26:27], v[20:21]
	s_delay_alu instid0(VALU_DEP_4)
	v_add_f64 v[30:31], v[30:31], v[32:33]
	v_mul_f64 v[32:33], v[40:41], s[14:15]
	v_add_f64 v[38:39], v[42:43], -v[38:39]
	v_add_f64 v[22:23], v[22:23], -v[36:37]
	v_add_f64 v[36:37], v[46:47], -v[26:27]
	v_add_f64 v[30:31], v[46:47], v[30:31]
	v_fma_f64 v[48:49], v[40:41], s[14:15], -v[32:33]
	v_add_f64 v[38:39], v[44:45], -v[38:39]
	v_add_f64 v[18:19], v[18:19], v[22:23]
	v_add_f64 v[22:23], v[46:47], -v[36:37]
	v_add_f64 v[20:21], v[20:21], -v[36:37]
	v_add_f64 v[44:45], v[52:53], v[30:31]
	v_fma_f64 v[40:41], v[40:41], s[12:13], v[48:49]
	v_add_f64 v[18:19], v[18:19], v[38:39]
	v_add_f64 v[22:23], v[26:27], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[44:45], -v[52:53]
	v_add_f64 v[36:37], v[32:33], v[40:41]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[38:39], v[42:43], v[18:19]
	v_add_f64 v[20:21], v[20:21], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[22:23], v[30:31], -v[26:27]
	v_add_f64 v[32:33], v[36:37], -v[32:33]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[36:37], v[38:39]
	v_add_f64 v[42:43], v[38:39], -v[42:43]
	v_add_f64 v[20:21], v[20:21], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[32:33], v[40:41], -v[32:33]
	v_add_f64 v[22:23], v[26:27], -v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[18:19], -v[42:43]
	v_add_f64 v[30:31], v[44:45], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[46:47], v[26:27], -v[22:23]
	v_add_f64 v[22:23], v[38:39], -v[22:23]
	v_add_f64 v[38:39], v[32:33], v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[44:45], v[30:31], -v[44:45]
	v_mul_f64 v[48:49], v[10:11], v[30:31]
	v_add_f64 v[36:37], v[36:37], -v[46:47]
	v_add_f64 v[20:21], v[20:21], -v[44:45]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[30:31], v[10:11], v[30:31], -v[48:49]
	v_cmp_class_f64_e64 s0, v[48:49], 0x204
	v_add_f64 v[22:23], v[22:23], v[36:37]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[20:21], v[10:11], v[20:21], v[30:31]
	v_add_f64 v[30:31], v[38:39], -v[32:33]
	v_add_f64 v[22:23], v[38:39], v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[36:37], v[48:49], v[20:21]
	v_add_f64 v[38:39], v[38:39], -v[30:31]
	v_add_f64 v[18:19], v[18:19], -v[30:31]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[40:41], v[26:27], v[22:23]
	v_cndmask_b32_e64 v43, v37, v49, s0
	v_cndmask_b32_e64 v42, v36, v48, s0
	v_add_f64 v[30:31], v[32:33], -v[38:39]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_mul_f64 v[44:45], v[42:43], s[16:17]
	v_add_f64 v[26:27], v[40:41], -v[26:27]
	v_cmp_nlt_f64_e64 s1, 0x40900000, v[42:43]
	v_add_f64 v[18:19], v[18:19], v[30:31]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_rndne_f64_e32 v[32:33], v[44:45]
	v_add_f64 v[22:23], v[22:23], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[32:33], s[20:21], v[42:43]
	v_add_f64 v[18:19], v[18:19], v[22:23]
	v_cvt_i32_f64_e32 v4, v[32:33]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[22:23], v[32:33], s[22:23], v[26:27]
	v_add_f64 v[26:27], v[40:41], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[30:31], v[22:23], s[26:27], s[24:25]
	v_add_f64 v[38:39], v[26:27], -v[40:41]
	v_mul_f64 v[40:41], v[6:7], v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[30:31], v[22:23], v[30:31], s[28:29]
	v_add_f64 v[18:19], v[18:19], -v[38:39]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[26:27], v[6:7], v[26:27], -v[40:41]
	v_cmp_class_f64_e64 s0, v[40:41], 0x204
	v_fma_f64 v[30:31], v[22:23], v[30:31], s[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[6:7], v[18:19], v[26:27]
	v_fma_f64 v[26:27], v[22:23], v[30:31], s[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[30:31], v[40:41], v[18:19]
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v39, v31, v41, s0
	v_cndmask_b32_e64 v38, v30, v40, s0
	v_cmp_neq_f64_e64 s0, 0x7ff00000, |v[42:43]|
	v_add_f64 v[30:31], v[30:31], -v[40:41]
	v_mul_f64 v[40:41], v[6:7], 0.5
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_mul_f64 v[44:45], v[38:39], s[16:17]
	s_mov_b32 s16, 11
	s_mov_b32 s17, 0x3fe00000
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[38:39]
	v_add_f64 v[18:19], v[18:19], -v[30:31]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[30:31], v[40:41]
	v_rndne_f64_e32 v[44:45], v[44:45]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[40:41]
	v_fma_f64 v[46:47], v[44:45], s[20:21], v[38:39]
	v_cmp_lt_f64_e64 s20, |v[12:13]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[18:19]
	v_fma_f64 v[46:47], v[44:45], s[22:23], v[46:47]
	v_cvt_i32_f64_e32 v44, v[44:45]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[26:27], v[22:23], v[26:27], s[16:17]
	v_fma_f64 v[50:51], v[46:47], s[26:27], s[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[22:23], v[26:27], 1.0
	v_fma_f64 v[50:51], v[46:47], v[50:51], s[28:29]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[22:23], v[22:23], v[26:27], 1.0
	v_mul_f64 v[26:27], v[34:35], v[24:25]
	v_fma_f64 v[24:25], v[24:25], v[28:29], s[2:3]
	v_add_f64 v[28:29], v[36:37], -v[48:49]
	v_ldexp_f64 v[48:49], v[34:35], 1
	v_mul_f64 v[36:37], v[10:11], 0.5
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[42:43]
	v_cmp_neq_f64_e64 s3, v[10:11], |v[10:11]|
	v_fma_f64 v[32:33], v[46:47], v[50:51], s[30:31]
	v_ldexp_f64 v[22:23], v[22:23], v4
	v_frexp_exp_i32_f64_e32 v4, v[2:3]
	v_mul_f64 v[24:25], v[26:27], v[24:25]
	v_add_f64 v[20:21], v[20:21], -v[28:29]
	v_trunc_f64_e32 v[28:29], v[10:11]
	s_xor_b32 s3, s3, s20
	v_fma_f64 v[26:27], v[46:47], v[32:33], s[34:35]
	v_trunc_f64_e32 v[32:33], v[36:37]
	v_cndmask_b32_e64 v23, 0x7ff00000, v23, s1
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_add_f64 v[34:35], v[48:49], v[24:25]
	s_and_b32 vcc_lo, s2, s1
	v_cndmask_b32_e64 v21, 0, v21, s0
	s_delay_alu instid0(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[42:43], v4
	v_cndmask_b32_e64 v20, 0, v20, s0
	v_fma_f64 v[26:27], v[46:47], v[26:27], s[36:37]
	v_cndmask_b32_e64 v23, 0, v23, s2
	v_cndmask_b32_e32 v22, 0, v22, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[28:29], v[10:11]
	v_cmp_neq_f64_e64 s0, v[32:33], v[36:37]
	v_cmp_eq_f64_e64 s2, 0, v[12:13]
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[20:21], v[22:23], v[20:21], v[22:23]
	v_cmp_class_f64_e64 s1, v[22:23], 0x204
	v_add_f64 v[16:17], v[34:35], -v[48:49]
	v_mul_f64 v[28:29], v[42:43], s[14:15]
	v_fma_f64 v[26:27], v[46:47], v[26:27], s[38:39]
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v4, 0x3ff00000, v13, s0
	v_cndmask_b32_e64 v32, v20, v22, s1
	v_cndmask_b32_e64 v22, v21, v23, s1
	v_cmp_gt_f64_e64 s1, 0, v[10:11]
	v_bfi_b32 v4, 0x7fffffff, v22, v4
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v24, 0x7ff80000, v4, vcc_lo
	v_cndmask_b32_e32 v25, 0, v32, vcc_lo
	v_fma_f64 v[20:21], v[42:43], s[14:15], -v[28:29]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[12:13]
	v_fma_f64 v[22:23], v[46:47], v[26:27], s[40:41]
	v_cndmask_b32_e64 v26, 0x7ff00000, 0, s3
	v_cmp_neq_f64_e64 s3, |v[12:13]|, 1.0
	s_xor_b32 s1, s1, s2
	v_add_f64 v[14:15], v[14:15], v[16:17]
	v_fma_f64 v[20:21], v[42:43], s[12:13], v[20:21]
	v_cndmask_b32_e32 v27, v32, v25, vcc_lo
	v_cndmask_b32_e32 v4, v4, v24, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[10:11], 0x204
	v_cmp_class_f64_e64 s12, v[12:13], 0x204
	v_fma_f64 v[16:17], v[46:47], v[22:23], s[18:19]
	v_cndmask_b32_e64 v22, 0x7ff00000, 0, s1
	v_cndmask_b32_e64 v23, 0, v13, s0
	v_cndmask_b32_e64 v26, 0x3ff00000, v26, s3
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[38:39]
	v_cmp_lt_f64_e64 s13, |v[8:9]|, 1.0
	v_cmp_gt_f64_e64 s3, 0, v[6:7]
	v_bfi_b32 v32, 0x7fffffff, v22, v23
	v_add_f64 v[24:25], v[34:35], v[14:15]
	v_add_f64 v[22:23], v[28:29], v[20:21]
	v_cndmask_b32_e32 v4, v4, v26, vcc_lo
	s_or_b32 s0, s2, s12
	v_cmp_neq_f64_e64 s12, v[6:7], |v[6:7]|
	v_cmp_eq_f64_e64 s2, 0, v[8:9]
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v26, v4, v32, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[12:13], v[10:11]
	v_fma_f64 v[10:11], v[46:47], v[16:17], s[16:17]
	v_cndmask_b32_e64 v27, v27, 0, s0
	v_mov_b32_e32 v4, 0
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[38:39]
	v_add_f64 v[34:35], v[24:25], -v[34:35]
	v_add_f64 v[12:13], v[22:23], v[24:25]
	v_add_f64 v[28:29], v[22:23], -v[28:29]
	s_xor_b32 s12, s12, s13
	s_xor_b32 s3, s3, s2
	v_cndmask_b32_e32 v16, 0, v27, vcc_lo
	v_cndmask_b32_e32 v17, 0x7ff80000, v26, vcc_lo
	v_fma_f64 v[10:11], v[46:47], v[10:11], 1.0
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[38:39]|
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[16:17], v[4:5], v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[34:35]
	v_add_f64 v[26:27], v[12:13], -v[22:23]
	v_add_f64 v[20:21], v[20:21], -v[28:29]
	v_trunc_f64_e32 v[28:29], v[6:7]
	v_fma_f64 v[10:11], v[46:47], v[10:11], 1.0
	v_dual_cndmask_b32 v19, 0, v19 :: v_dual_cndmask_b32 v18, 0, v18
	s_and_b32 vcc_lo, s1, s0
	v_div_scale_f64 v[32:33], null, v[2:3], v[2:3], v[16:17]
	v_add_f64 v[36:37], v[12:13], -v[26:27]
	v_add_f64 v[24:25], v[24:25], -v[26:27]
	v_add_f64 v[34:35], v[20:21], v[14:15]
	v_ldexp_f64 v[10:11], v[10:11], v44
	v_rcp_f64_e32 v[42:43], v[32:33]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[22:23], -v[36:37]
	v_cndmask_b32_e64 v11, 0x7ff00000, v11, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v10, 0, v10, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[28:29], v[6:7]
	v_cmp_neq_f64_e64 s0, v[30:31], v[40:41]
	v_cndmask_b32_e64 v11, 0, v11, s1
	s_delay_alu instid0(TRANS32_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], -v[32:33], v[42:43], 1.0
	v_fma_f64 v[18:19], v[10:11], v[18:19], v[10:11]
	v_cmp_class_f64_e64 s1, v[10:11], 0x204
	v_add_f64 v[22:23], v[24:25], v[22:23]
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v30, 0x3ff00000, v9, s0
	v_fma_f64 v[24:25], v[42:43], v[26:27], v[42:43]
	v_add_f64 v[26:27], v[34:35], -v[20:21]
	v_cndmask_b32_e64 v31, v18, v10, s1
	v_cndmask_b32_e64 v10, v19, v11, s1
	v_bfi_b32 v30, 0x7fffffff, v10, v30
	v_add_f64 v[22:23], v[34:35], v[22:23]
	v_fma_f64 v[28:29], -v[32:33], v[24:25], 1.0
	v_add_f64 v[10:11], v[34:35], -v[26:27]
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e32 v34, 0x7ff80000, v30, vcc_lo
	v_cndmask_b32_e32 v35, 0, v31, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[8:9]
	v_add_f64 v[14:15], v[14:15], -v[26:27]
	v_add_f64 v[18:19], v[12:13], v[22:23]
	v_fma_f64 v[24:25], v[24:25], v[28:29], v[24:25]
	v_div_scale_f64 v[28:29], s1, v[16:17], v[2:3], v[16:17]
	v_add_f64 v[10:11], v[20:21], -v[10:11]
	v_dual_cndmask_b32 v26, v31, v35 :: v_dual_cndmask_b32 v27, v30, v34
	v_cmp_neq_f64_e64 vcc_lo, |v[8:9]|, 1.0
	v_cndmask_b32_e64 v20, 0x7ff00000, 0, s12
	v_cmp_class_f64_e64 s12, v[8:9], 0x204
	v_cndmask_b32_e64 v31, 0x7ff00000, 0, s3
	v_cndmask_b32_e64 v34, 0, v9, s0
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_add_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3)
	v_bfi_b32 v15, 0x7fffffff, v31, v34
	v_cndmask_b32_e32 v30, 0x3ff00000, v20, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[6:7], 0x204
	v_mul_f64 v[20:21], v[28:29], v[24:25]
	s_or_b32 s0, s2, s12
	v_add_f64 v[12:13], v[22:23], -v[12:13]
	v_cndmask_b32_e32 v14, v27, v30, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v14, v14, v15, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[8:9], v[6:7]
	v_fma_f64 v[6:7], -v[32:33], v[20:21], v[28:29]
	v_cndmask_b32_e64 v15, v26, 0, s0
	v_add_f64 v[8:9], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v10, 0, v15, vcc_lo
	v_cndmask_b32_e32 v11, 0x7ff80000, v14, vcc_lo
	s_mov_b32 vcc_lo, s1
	s_load_b64 s[0:1], s[6:7], 0x0
	v_div_fmas_f64 v[6:7], v[6:7], v[24:25], v[20:21]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_add_f64 v[8:9], v[18:19], v[8:9]
	v_mul_f64 v[10:11], s[4:5], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_fixup_f64 v[6:7], v[6:7], v[2:3], v[16:17]
	v_dual_cndmask_b32 v8, v8, v2 :: v_dual_cndmask_b32 v9, v9, v3
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[4:5], v[10:11], v[4:5]
	v_cndmask_b32_e32 v9, 0x7ff80000, v9, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v9, 0xfff00000, v9, vcc_lo
	v_add_co_u32 v0, vcc_lo, s8, v0
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], -v[8:9], v[4:5], v[6:7]
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[2:3], v[2:3], -s[0:1]
	s_load_b64 s[0:1], s[10:11], 0x0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], s[0:1], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel focal_grad_kernel
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
		.amdhsa_next_free_vgpr 58
		.amdhsa_next_free_sgpr 42
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
		.amdhsa_inst_pref_size 40
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
	.size	focal_grad_kernel, .Lfunc_end4-focal_grad_kernel
                                        ; -- End function
	.set focal_grad_kernel.num_vgpr, 58
	.set focal_grad_kernel.num_agpr, 0
	.set focal_grad_kernel.numbered_sgpr, 42
	.set focal_grad_kernel.num_named_barrier, 0
	.set focal_grad_kernel.private_seg_size, 0
	.set focal_grad_kernel.uses_vcc, 1
	.set focal_grad_kernel.uses_flat_scratch, 0
	.set focal_grad_kernel.has_dyn_sized_stack, 0
	.set focal_grad_kernel.has_recursion, 0
	.set focal_grad_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 5064
; TotalNumSgprs: 44
; NumVgprs: 58
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 7
; NumSGPRsForWavesPerEU: 44
; NumVGPRsForWavesPerEU: 58
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
	.protected	kl_div_loss_kernel      ; -- Begin function kl_div_loss_kernel
	.globl	kl_div_loss_kernel
	.p2align	8
	.type	kl_div_loss_kernel,@function
kl_div_loss_kernel:                     ; @kl_div_loss_kernel
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
	s_cbranch_execz .LBB5_4
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0
	s_mov_b32 s2, exec_lo
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v3, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_lt_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB5_3
; %bb.2:
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[4:5]
	s_mov_b32 s6, 0x55555780
	v_cndmask_b32_e64 v6, 0, 1, vcc_lo
	v_ldexp_f64 v[4:5], v[4:5], v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[4:5], 1.0
	v_add_f64 v[12:13], v[4:5], -1.0
	v_rcp_f64_e32 v[8:9], v[6:7]
	v_add_f64 v[14:15], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_mul_f64 v[16:17], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[10:11], v[6:7], -v[16:17]
	v_fma_f64 v[4:5], v[10:11], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[16:17], v[4:5]
	v_add_f64 v[14:15], v[12:13], -v[6:7]
	v_add_f64 v[16:17], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[4:5], v[16:17], -v[4:5]
	v_frexp_exp_i32_f64_e32 v16, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], -v[6:7]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[14:15], v[4:5]
	v_mul_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[6:7]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0x3fe62e42
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_subrev_co_ci_u32_e64 v14, null, 0, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_co_u32 v22, vcc_lo, s4, v0
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_co_ci_u32_e64 v23, null, s5, v1, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[2:3]
	global_load_b64 v[22:23], v[22:23], off
	v_add_f64 v[10:11], v[12:13], v[8:9]
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_mul_f64 v[16:17], v[14:15], s[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[6:7], -v[16:17]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_fma_f64 v[8:9], v[14:15], s[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[10:11], v[4:5]
	v_add_f64 v[16:17], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[6:7], v[12:13]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[14:15], -v[6:7]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[14:15], -v[18:19]
	v_add_f64 v[10:11], v[12:13], -v[18:19]
	v_add_f64 v[12:13], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[12:13], -v[8:9]
	v_add_f64 v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[10:11]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_add_f64 v[16:17], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[4:5], v[16:17], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[22:23]
	v_mul_f64 v[4:5], v[2:3], v[4:5]
.LBB5_3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB5_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel kl_div_loss_kernel
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
		.amdhsa_next_free_vgpr 24
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
		.amdhsa_inst_pref_size 9
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
	.size	kl_div_loss_kernel, .Lfunc_end5-kl_div_loss_kernel
                                        ; -- End function
	.set kl_div_loss_kernel.num_vgpr, 24
	.set kl_div_loss_kernel.num_agpr, 0
	.set kl_div_loss_kernel.numbered_sgpr, 12
	.set kl_div_loss_kernel.num_named_barrier, 0
	.set kl_div_loss_kernel.private_seg_size, 0
	.set kl_div_loss_kernel.uses_vcc, 1
	.set kl_div_loss_kernel.uses_flat_scratch, 0
	.set kl_div_loss_kernel.has_dyn_sized_stack, 0
	.set kl_div_loss_kernel.has_recursion, 0
	.set kl_div_loss_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1100
; TotalNumSgprs: 14
; NumVgprs: 24
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 14
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
	.protected	hinge_kernel            ; -- Begin function hinge_kernel
	.globl	hinge_kernel
	.p2align	8
	.type	hinge_kernel,@function
hinge_kernel:                           ; @hinge_kernel
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
	s_cbranch_execz .LBB6_2
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v5, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(1)
	v_xor_b32_e32 v7, 0x80000000, v3
	s_waitcnt vmcnt(0)
	v_fma_f64 v[4:5], -v[2:3], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_lt_f64_e32 vcc_lo, 0, v[4:5]
	v_max_f64 v[3:4], v[4:5], 0
	v_add_co_u32 v5, s0, s4, v0
	v_add_co_ci_u32_e64 v6, null, s5, v1, s0
	v_dual_cndmask_b32 v8, 0, v7 :: v_dual_cndmask_b32 v7, 0, v2
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[5:6], v[3:4], off
	global_store_b64 v[0:1], v[7:8], off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel hinge_kernel
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
		.amdhsa_next_free_vgpr 9
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
.Lfunc_end6:
	.size	hinge_kernel, .Lfunc_end6-hinge_kernel
                                        ; -- End function
	.set hinge_kernel.num_vgpr, 9
	.set hinge_kernel.num_agpr, 0
	.set hinge_kernel.numbered_sgpr, 8
	.set hinge_kernel.num_named_barrier, 0
	.set hinge_kernel.private_seg_size, 0
	.set hinge_kernel.uses_vcc, 1
	.set hinge_kernel.uses_flat_scratch, 0
	.set hinge_kernel.has_dyn_sized_stack, 0
	.set hinge_kernel.has_recursion, 0
	.set hinge_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 244
; TotalNumSgprs: 10
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.protected	cosine_emb_kernel       ; -- Begin function cosine_emb_kernel
	.globl	cosine_emb_kernel
	.p2align	8
	.type	cosine_emb_kernel,@function
cosine_emb_kernel:                      ; @cosine_emb_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[12:13], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s12, v1
	s_cbranch_execz .LBB7_11
; %bb.1:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x28
	s_cmp_lt_i32 s13, 1
	s_cbranch_scc1 .LBB7_5
; %bb.2:
	v_mul_lo_u32 v2, v1, s13
	v_mov_b32_e32 v6, 0
	v_dual_mov_b32 v7, 0 :: v_dual_mov_b32 v4, 0
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[10:11], 3, v[2:3]
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, s4, v10
	v_add_co_ci_u32_e64 v9, null, s5, v11, vcc_lo
	v_add_co_u32 v10, vcc_lo, s6, v10
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v11, null, s7, v11, vcc_lo
	.p2align	6
.LBB7_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[12:13], v[8:9], off
	global_load_b64 v[14:15], v[10:11], off
	v_add_co_u32 v8, vcc_lo, v8, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	v_add_co_u32 v10, vcc_lo, v10, 8
	v_add_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	s_add_i32 s13, s13, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s13, 0
	s_waitcnt vmcnt(1)
	v_fma_f64 v[4:5], v[12:13], v[12:13], v[4:5]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[2:3], v[12:13], v[14:15], v[2:3]
	v_fma_f64 v[6:7], v[14:15], v[14:15], v[6:7]
	s_cbranch_scc0 .LBB7_3
; %bb.4:
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[4:5]
	v_cmp_gt_f64_e64 s0, 0x10000000, v[6:7]
	v_cndmask_b32_e64 v0, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v8, 0, 0x100, s0
	v_ldexp_f64 v[4:5], v[4:5], v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], v8
	v_cndmask_b32_e64 v0, 0, 0xffffff80, vcc_lo
	v_rsq_f64_e32 v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_rsq_f64_e32 v[10:11], v[6:7]
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[12:13], v[4:5], v[8:9]
	v_mul_f64 v[8:9], v[8:9], 0.5
	v_mul_f64 v[14:15], v[6:7], v[10:11]
	v_mul_f64 v[10:11], v[10:11], 0.5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], -v[8:9], v[12:13], 0.5
	v_fma_f64 v[18:19], -v[10:11], v[14:15], 0.5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[12:13], v[16:17], v[12:13]
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[8:9]
	v_fma_f64 v[14:15], v[14:15], v[18:19], v[14:15]
	v_fma_f64 v[10:11], v[10:11], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], -v[12:13], v[12:13], v[4:5]
	v_fma_f64 v[18:19], -v[14:15], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[16:17], v[8:9], v[12:13]
	v_fma_f64 v[14:15], v[18:19], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], -v[12:13], v[12:13], v[4:5]
	v_fma_f64 v[18:19], -v[14:15], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[16:17], v[8:9], v[12:13]
	v_cndmask_b32_e64 v12, 0, 0xffffff80, s0
	v_fma_f64 v[10:11], v[18:19], v[10:11], v[14:15]
	v_cmp_class_f64_e64 s0, v[6:7], 0x260
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[8:9], v[8:9], v0
	v_ldexp_f64 v[10:11], v[10:11], v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v5, v9, v5 :: v_dual_cndmask_b32 v4, v8, v4
	v_cndmask_b32_e64 v7, v11, v7, s0
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v6, v10, v6, s0
	s_mov_b32 s0, 0x812dea11
	s_mov_b32 s1, 0x3d719799
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], v[2:3]
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_div_scale_f64 v[10:11], vcc_lo, v[2:3], v[4:5], v[2:3]
	v_mul_f64 v[12:13], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[12:13], v[10:11]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[6:7], v[4:5], v[2:3]
	s_branch .LBB7_6
.LBB7_5:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB7_6:
	v_ashrrev_i32_e32 v2, 31, v1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[2:3], 0x0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v5, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v6, null, s9, v1, vcc_lo
	global_load_b64 v[5:6], v[5:6], off
	s_waitcnt vmcnt(0)
	v_cmp_nlt_f64_e32 vcc_lo, 0, v[5:6]
                                        ; implicit-def: $vgpr5_vgpr6
	s_and_saveexec_b32 s2, vcc_lo
	s_xor_b32 s2, exec_lo, s2
	s_cbranch_execz .LBB7_8
; %bb.7:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[3:4], -s[0:1]
	s_delay_alu instid0(VALU_DEP_1)
	v_max_f64 v[5:6], v[2:3], 0
                                        ; implicit-def: $vgpr3_vgpr4
.LBB7_8:
	s_waitcnt lgkmcnt(0)
	s_and_not1_saveexec_b32 s0, s2
	s_cbranch_execz .LBB7_10
; %bb.9:
	v_add_f64 v[5:6], -v[3:4], 1.0
.LBB7_10:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s10, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	global_store_b64 v[0:1], v[5:6], off
.LBB7_11:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel cosine_emb_kernel
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
		.amdhsa_next_free_vgpr 20
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
.Lfunc_end7:
	.size	cosine_emb_kernel, .Lfunc_end7-cosine_emb_kernel
                                        ; -- End function
	.set cosine_emb_kernel.num_vgpr, 20
	.set cosine_emb_kernel.num_agpr, 0
	.set cosine_emb_kernel.numbered_sgpr, 14
	.set cosine_emb_kernel.num_named_barrier, 0
	.set cosine_emb_kernel.private_seg_size, 0
	.set cosine_emb_kernel.uses_vcc, 1
	.set cosine_emb_kernel.uses_flat_scratch, 0
	.set cosine_emb_kernel.has_dyn_sized_stack, 0
	.set cosine_emb_kernel.has_recursion, 0
	.set cosine_emb_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 940
; TotalNumSgprs: 16
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 16
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
	.protected	triplet_kernel          ; -- Begin function triplet_kernel
	.globl	triplet_kernel
	.p2align	8
	.type	triplet_kernel,@function
triplet_kernel:                         ; @triplet_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[12:13], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s12, v1
	s_cbranch_execz .LBB8_7
; %bb.1:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x28
	s_cmp_lt_i32 s13, 1
	s_cbranch_scc1 .LBB8_5
; %bb.2:
	v_mul_lo_u32 v2, v1, s13
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[10:11], 3, v[2:3]
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s4, v10
	v_add_co_ci_u32_e64 v7, null, s5, v11, vcc_lo
	v_add_co_u32 v8, vcc_lo, s6, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s7, v11, vcc_lo
	v_add_co_u32 v10, vcc_lo, s8, v10
	v_add_co_ci_u32_e64 v11, null, s9, v11, vcc_lo
	.p2align	6
.LBB8_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[12:13], v[6:7], off
	global_load_b64 v[14:15], v[8:9], off
	global_load_b64 v[16:17], v[10:11], off
	v_add_co_u32 v6, vcc_lo, v6, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	v_add_co_u32 v8, vcc_lo, v8, 8
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	v_add_co_u32 v10, vcc_lo, v10, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	s_add_i32 s13, s13, -1
	s_cmp_eq_u32 s13, 0
	s_waitcnt vmcnt(1)
	v_add_f64 v[14:15], v[12:13], -v[14:15]
	s_waitcnt vmcnt(0)
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[2:3], v[14:15], v[14:15], v[2:3]
	v_fma_f64 v[4:5], v[12:13], v[12:13], v[4:5]
	s_cbranch_scc0 .LBB8_3
; %bb.4:
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], -v[4:5]
	s_branch .LBB8_6
.LBB8_5:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
.LBB8_6:
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[0:1], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], s[0:1], v[2:3]
	v_max_f64 v[3:4], v[2:3], 0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s10, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB8_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel triplet_kernel
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
		.amdhsa_next_free_vgpr 18
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
.Lfunc_end8:
	.size	triplet_kernel, .Lfunc_end8-triplet_kernel
                                        ; -- End function
	.set triplet_kernel.num_vgpr, 18
	.set triplet_kernel.num_agpr, 0
	.set triplet_kernel.numbered_sgpr, 14
	.set triplet_kernel.num_named_barrier, 0
	.set triplet_kernel.private_seg_size, 0
	.set triplet_kernel.uses_vcc, 1
	.set triplet_kernel.uses_flat_scratch, 0
	.set triplet_kernel.has_dyn_sized_stack, 0
	.set triplet_kernel.has_recursion, 0
	.set triplet_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 432
; TotalNumSgprs: 16
; NumVgprs: 18
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 16
; NumVGPRsForWavesPerEU: 18
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
	.protected	contrastive_kernel      ; -- Begin function contrastive_kernel
	.globl	contrastive_kernel
	.p2align	8
	.type	contrastive_kernel,@function
contrastive_kernel:                     ; @contrastive_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b64 s[12:13], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s12, v2
	s_cbranch_execz .LBB9_6
; %bb.1:
	s_clause 0x1
	s_load_b256 s[4:11], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x28
	s_cmp_lt_i32 s13, 1
	s_cbranch_scc1 .LBB9_4
; %bb.2:
	v_mul_lo_u32 v0, v2, s13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v1, 31, v0
	v_lshlrev_b64 v[5:6], 3, v[0:1]
	v_mov_b32_e32 v0, 0
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s4, v5
	v_add_co_ci_u32_e64 v4, null, s5, v6, vcc_lo
	v_add_co_u32 v5, vcc_lo, s6, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s7, v6, vcc_lo
	.p2align	6
.LBB9_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[7:8], v[3:4], off
	global_load_b64 v[9:10], v[5:6], off
	v_add_co_u32 v3, vcc_lo, v3, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_add_co_u32 v5, vcc_lo, v5, 8
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	s_add_i32 s13, s13, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s13, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_fma_f64 v[0:1], v[7:8], v[7:8], v[0:1]
	s_cbranch_scc0 .LBB9_3
	s_branch .LBB9_5
.LBB9_4:
	v_mov_b32_e32 v0, 0
	v_mov_b32_e32 v1, 0
.LBB9_5:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[0:1]
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[0:1], 0x0
	v_cndmask_b32_e64 v3, 0, 0x100, vcc_lo
	v_ldexp_f64 v[4:5], v[0:1], v3
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	v_add_co_u32 v10, s0, s8, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v11, null, s9, v3, s0
	global_load_b64 v[10:11], v[10:11], off
	v_rsq_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[8:9], v[4:5], v[6:7]
	v_mul_f64 v[6:7], v[6:7], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[6:7], v[8:9], 0.5
	v_fma_f64 v[8:9], v[8:9], v[12:13], v[8:9]
	v_fma_f64 v[6:7], v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[8:9], v[4:5]
	v_fma_f64 v[8:9], v[12:13], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[8:9], v[4:5]
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[8:9]
	v_cndmask_b32_e64 v8, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x260
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v8
	v_dual_cndmask_b32 v5, v7, v5 :: v_dual_cndmask_b32 v4, v6, v4
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], s[2:3], -v[4:5]
	v_mul_f64 v[6:7], v[4:5], v[4:5]
	v_cmp_lt_f64_e32 vcc_lo, 0, v[4:5]
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], -v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v7, 0, v7 :: v_dual_cndmask_b32 v6, 0, v6
	v_add_co_u32 v2, vcc_lo, s10, v2
	v_add_co_ci_u32_e64 v3, null, s11, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[6:7], v[4:5]
	v_fma_f64 v[0:1], v[0:1], v[10:11], v[4:5]
	global_store_b64 v[2:3], v[0:1], off
.LBB9_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel contrastive_kernel
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
		.amdhsa_next_free_vgpr 14
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
.Lfunc_end9:
	.size	contrastive_kernel, .Lfunc_end9-contrastive_kernel
                                        ; -- End function
	.set contrastive_kernel.num_vgpr, 14
	.set contrastive_kernel.num_agpr, 0
	.set contrastive_kernel.numbered_sgpr, 14
	.set contrastive_kernel.num_named_barrier, 0
	.set contrastive_kernel.private_seg_size, 0
	.set contrastive_kernel.uses_vcc, 1
	.set contrastive_kernel.uses_flat_scratch, 0
	.set contrastive_kernel.has_dyn_sized_stack, 0
	.set contrastive_kernel.has_recursion, 0
	.set contrastive_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 588
; TotalNumSgprs: 16
; NumVgprs: 14
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 16
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
	.type	__hip_cuid_d63cc4416d33e236,@object ; @__hip_cuid_d63cc4416d33e236
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_d63cc4416d33e236
__hip_cuid_d63cc4416d33e236:
	.byte	0                               ; 0x0
	.size	__hip_cuid_d63cc4416d33e236, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_d63cc4416d33e236
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
    .name:           mae_grad_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         mae_grad_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     14
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
    .name:           huber_grad_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         huber_grad_kernel.kd
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
    .name:           bce_logits_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     38
    .sgpr_spill_count: 0
    .symbol:         bce_logits_kernel.kd
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
    .name:           focal_loss_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     42
    .sgpr_spill_count: 0
    .symbol:         focal_loss_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     58
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
      - .address_space:  global
        .offset:         48
        .size:           8
        .value_kind:     global_buffer
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
    .name:           focal_grad_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     44
    .sgpr_spill_count: 0
    .symbol:         focal_grad_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     58
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
    .name:           kl_div_loss_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         kl_div_loss_kernel.kd
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
    .name:           hinge_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         hinge_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           cosine_emb_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         cosine_emb_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           triplet_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         triplet_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     18
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 304
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           contrastive_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         contrastive_kernel.kd
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
