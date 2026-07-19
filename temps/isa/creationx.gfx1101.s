	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	creationx_arange_kernel ; -- Begin function creationx_arange_kernel
	.globl	creationx_arange_kernel
	.p2align	8
	.type	creationx_arange_kernel,@function
creationx_arange_kernel:                ; @creationx_arange_kernel
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
	s_cbranch_execz .LBB0_2
; %bb.1:
	v_cvt_f64_i32_e32 v[2:3], v1
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], s[0:1], v[2:3], s[6:7]
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel creationx_arange_kernel
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
.Lfunc_end0:
	.size	creationx_arange_kernel, .Lfunc_end0-creationx_arange_kernel
                                        ; -- End function
	.set creationx_arange_kernel.num_vgpr, 5
	.set creationx_arange_kernel.num_agpr, 0
	.set creationx_arange_kernel.numbered_sgpr, 8
	.set creationx_arange_kernel.num_named_barrier, 0
	.set creationx_arange_kernel.private_seg_size, 0
	.set creationx_arange_kernel.uses_vcc, 1
	.set creationx_arange_kernel.uses_flat_scratch, 0
	.set creationx_arange_kernel.has_dyn_sized_stack, 0
	.set creationx_arange_kernel.has_recursion, 0
	.set creationx_arange_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 144
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
	.protected	creationx_linspace_kernel ; -- Begin function creationx_linspace_kernel
	.globl	creationx_linspace_kernel
	.p2align	8
	.type	creationx_linspace_kernel,@function
creationx_linspace_kernel:              ; @creationx_linspace_kernel
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
	s_cbranch_execz .LBB1_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	s_cmp_eq_u32 s3, 1
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v3, s6 :: v_dual_mov_b32 v4, s7
	s_cbranch_scc1 .LBB1_5
; %bb.2:
	v_dual_mov_b32 v4, s1 :: v_dual_mov_b32 v3, s0
	s_add_i32 s3, s3, -1
	s_mov_b32 s2, exec_lo
	v_cmpx_ne_u32_e64 s3, v1
	s_cbranch_execz .LBB1_4
; %bb.3:
	v_add_f64 v[2:3], s[0:1], -s[6:7]
	v_cvt_f64_i32_e32 v[4:5], s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], v[2:3]
	v_div_scale_f64 v[12:13], vcc_lo, v[2:3], v[4:5], v[2:3]
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[12:13]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[6:7], v[4:5], v[2:3]
	v_cvt_f64_i32_e32 v[4:5], v1
	v_fma_f64 v[3:4], v[2:3], v[4:5], s[6:7]
.LBB1_4:
	s_or_b32 exec_lo, exec_lo, s2
.LBB1_5:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB1_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel creationx_linspace_kernel
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
.Lfunc_end1:
	.size	creationx_linspace_kernel, .Lfunc_end1-creationx_linspace_kernel
                                        ; -- End function
	.set creationx_linspace_kernel.num_vgpr, 14
	.set creationx_linspace_kernel.num_agpr, 0
	.set creationx_linspace_kernel.numbered_sgpr, 8
	.set creationx_linspace_kernel.num_named_barrier, 0
	.set creationx_linspace_kernel.private_seg_size, 0
	.set creationx_linspace_kernel.uses_vcc, 1
	.set creationx_linspace_kernel.uses_flat_scratch, 0
	.set creationx_linspace_kernel.has_dyn_sized_stack, 0
	.set creationx_linspace_kernel.has_recursion, 0
	.set creationx_linspace_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 312
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
	.protected	creationx_logspace_kernel ; -- Begin function creationx_logspace_kernel
	.globl	creationx_logspace_kernel
	.p2align	8
	.type	creationx_logspace_kernel,@function
creationx_logspace_kernel:              ; @creationx_logspace_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s4, s[0:1], 0x34
	s_load_b32 s3, s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s4, s4, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s4, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s3, v1
	s_cbranch_execz .LBB2_6
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	s_cmp_eq_u32 s3, 1
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v2, s6 :: v_dual_mov_b32 v3, s7
	s_cbranch_scc1 .LBB2_5
; %bb.2:
	v_dual_mov_b32 v2, s8 :: v_dual_mov_b32 v3, s9
	s_add_i32 s3, s3, -1
	s_mov_b32 s0, exec_lo
	v_cmpx_ne_u32_e64 s3, v1
	s_cbranch_execz .LBB2_4
; %bb.3:
	v_add_f64 v[2:3], s[8:9], -s[6:7]
	v_cvt_f64_i32_e32 v[4:5], s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], v[2:3]
	v_div_scale_f64 v[12:13], vcc_lo, v[2:3], v[4:5], v[2:3]
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[12:13]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[6:7], v[4:5], v[2:3]
	v_cvt_f64_i32_e32 v[4:5], v1
	v_fma_f64 v[2:3], v[2:3], v[4:5], s[6:7]
.LBB2_4:
	s_or_b32 exec_lo, exec_lo, s0
.LBB2_5:
	v_cmp_neq_f64_e64 vcc_lo, s[10:11], 1.0
	v_mov_b32_e32 v0, s11
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s2, 0x968915a9
	s_mov_b32 s6, 0x4222de17
	s_mov_b32 s3, 0x3fba6564
	s_mov_b32 s7, 0x3fbdee67
	v_cndmask_b32_e32 v3, 0x3ff00000, v3, vcc_lo
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v5, 0x3ff00000, v0, vcc_lo
	v_cndmask_b32_e64 v4, 0, s10, vcc_lo
	v_frexp_mant_f64_e64 v[6:7], |v[4:5]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[6:7]
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v0
	v_frexp_exp_i32_f64_e32 v0, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	v_add_f64 v[16:17], v[8:9], -1.0
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	v_fma_f64 v[8:9], v[12:13], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[8:9]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[6:7], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[8:9], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], -v[12:13]
	v_mul_f64 v[12:13], v[8:9], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[14:15], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[14:15], v[10:11]
	v_add_f64 v[14:15], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], s[6:7], s[2:3]
	s_mov_b32 s2, 0x3abe935a
	s_mov_b32 s3, 0x3fbe25e4
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_mul_f64 v[22:23], v[8:9], v[14:15]
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x47e6c9c2
	s_mov_b32 s3, 0x3fc110ef
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0xcfa74449
	s_mov_b32 s3, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x71bf3c30
	s_mov_b32 s3, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x1c7792ce
	s_mov_b32 s3, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x924920da
	s_mov_b32 s3, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x9999999c
	s_mov_b32 s3, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s3, 0x3c7abc9e
	s_mov_b32 s2, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[14:15], v[16:17]
	v_fma_f64 v[12:13], v[14:15], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[16:17], v[12:13]
	v_add_f64 v[16:17], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[20:21], v[16:17], s[0:1]
	v_add_f64 v[18:19], v[16:17], -v[18:19]
	s_mov_b32 s1, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[24:25], v[20:21], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_fma_f64 v[18:19], v[14:15], v[8:9], -v[22:23]
	s_mov_b32 s0, 0xd5df274d
	s_mov_b32 s1, 0x3c8543b0
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], s[0:1]
	v_fma_f64 v[14:15], v[14:15], v[6:7], v[18:19]
	s_mov_b32 s1, 0x3fe62e42
	s_mov_b32 s0, 0xfefa39ef
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], v[16:17]
	v_fma_f64 v[10:11], v[10:11], v[8:9], v[14:15]
	v_ldexp_f64 v[8:9], v[8:9], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[20:21], v[12:13]
	v_add_f64 v[16:17], v[22:23], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[20:21], -v[14:15]
	v_mul_f64 v[20:21], v[16:17], v[14:15]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], v[18:19]
	v_fma_f64 v[18:19], v[16:17], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[12:13], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[12:13]
	v_cvt_f64_i32_e32 v[14:15], v0
	v_add_f64 v[12:13], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[8:9], v[12:13]
	v_add_f64 v[18:19], v[12:13], -v[20:21]
	v_mul_f64 v[20:21], v[14:15], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[14:15], s[0:1], -v[20:21]
	s_mov_b32 s1, 0xbfe62e42
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_fma_f64 v[10:11], v[14:15], s[2:3], v[18:19]
	s_mov_b32 s3, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[16:17], v[6:7]
	v_add_f64 v[20:21], v[8:9], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[8:9], v[12:13]
	v_add_f64 v[16:17], v[12:13], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[14:15], -v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[14:15], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_add_f64 v[16:17], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[16:17], -v[10:11]
	v_add_f64 v[8:9], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[16:17], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[18:19], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[12:13], v[18:19], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], -v[18:19]
	v_mul_f64 v[12:13], v[2:3], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[2:3], v[8:9], -v[12:13]
	v_cmp_class_f64_e64 vcc_lo, v[12:13], 0x204
	v_fma_f64 v[6:7], v[2:3], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_dual_cndmask_b32 v11, v9, v13 :: v_dual_cndmask_b32 v10, v8, v12
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_mul_f64 v[14:15], v[10:11], s[6:7]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[10:11]|
	v_cmp_lt_f64_e64 s6, |v[4:5]|, 1.0
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_trunc_f64_e32 v[8:9], v[2:3]
	v_rndne_f64_e32 v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v7, 0, v7 :: v_dual_cndmask_b32 v6, 0, v6
	v_fma_f64 v[16:17], v[14:15], s[0:1], v[10:11]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_cvt_i32_f64_e32 v0, v[14:15]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], s[2:3], v[16:17]
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_cmp_neq_f64_e64 s3, v[2:3], |v[2:3]|
	v_cmp_eq_f64_e64 s2, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_xor_b32 s3, s3, s6
	v_cmp_class_f64_e64 s6, v[4:5], 0x204
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[10:11]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], 1.0
	s_and_b32 vcc_lo, s1, s0
	v_fma_f64 v[14:15], v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[12:13], v[14:15], v0
	v_mul_f64 v[14:15], v[2:3], 0.5
	v_cndmask_b32_e64 v0, 0x7ff00000, v13, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[10:11], v[14:15]
	v_cndmask_b32_e32 v12, 0, v12, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[8:9], v[2:3]
	v_cndmask_b32_e64 v9, 0x7ff00000, 0, s3
	v_cndmask_b32_e64 v13, 0, v0, s1
	v_cmp_neq_f64_e64 s3, |v[4:5]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[12:13]
	v_cmp_class_f64_e64 s1, v[12:13], 0x204
	v_cmp_neq_f64_e64 s0, v[10:11], v[14:15]
	v_cndmask_b32_e64 v9, 0x3ff00000, v9, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, v13, s1
	v_cndmask_b32_e64 v6, v6, v12, s1
	v_cmp_gt_f64_e64 s1, 0, v[2:3]
	v_cndmask_b32_e32 v8, 0, v6, vcc_lo
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v0, 0x3ff00000, v5, s0
	v_bfi_b32 v0, 0x7fffffff, v7, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v7, 0x7ff80000, v0, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[4:5]
	s_xor_b32 s1, s1, s2
	v_cndmask_b32_e32 v6, v6, v8, vcc_lo
	v_cndmask_b32_e32 v0, v0, v7, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_cndmask_b32_e64 v7, 0x7ff00000, 0, s1
	v_cndmask_b32_e64 v8, 0, v5, s0
	s_or_b32 s0, s2, s6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_bfi_b32 v7, 0x7fffffff, v7, v8
	v_cndmask_b32_e32 v0, v0, v9, vcc_lo
	v_cndmask_b32_e64 v7, v0, v7, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[4:5], v[2:3]
	v_ashrrev_i32_e32 v2, 31, v1
	v_cndmask_b32_e64 v6, v6, 0, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_cndmask_b32_e32 v2, 0, v6, vcc_lo
	v_cndmask_b32_e32 v3, 0x7ff80000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB2_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel creationx_logspace_kernel
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
		.amdhsa_next_free_vgpr 26
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
		.amdhsa_inst_pref_size 20
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
	.size	creationx_logspace_kernel, .Lfunc_end2-creationx_logspace_kernel
                                        ; -- End function
	.set creationx_logspace_kernel.num_vgpr, 26
	.set creationx_logspace_kernel.num_agpr, 0
	.set creationx_logspace_kernel.numbered_sgpr, 12
	.set creationx_logspace_kernel.num_named_barrier, 0
	.set creationx_logspace_kernel.private_seg_size, 0
	.set creationx_logspace_kernel.uses_vcc, 1
	.set creationx_logspace_kernel.uses_flat_scratch, 0
	.set creationx_logspace_kernel.has_dyn_sized_stack, 0
	.set creationx_logspace_kernel.has_recursion, 0
	.set creationx_logspace_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2468
; TotalNumSgprs: 14
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 26
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
	.protected	creationx_geomspace_kernel ; -- Begin function creationx_geomspace_kernel
	.globl	creationx_geomspace_kernel
	.p2align	8
	.type	creationx_geomspace_kernel,@function
creationx_geomspace_kernel:             ; @creationx_geomspace_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s8, s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB3_6
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_cmp_eq_u32 s8, 1
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v3, s6 :: v_dual_mov_b32 v4, s7
	s_cbranch_scc1 .LBB3_5
; %bb.2:
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_add_i32 s0, s8, -1
	s_mov_b32 s8, exec_lo
	v_cmpx_ne_u32_e64 s0, v1
	s_cbranch_execz .LBB3_4
; %bb.3:
	v_cvt_f64_i32_e32 v[2:3], v1
	v_cvt_f64_i32_e32 v[4:5], s0
	v_div_scale_f64 v[8:9], null, s[6:7], s[6:7], s[2:3]
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s10, 0x4222de17
	s_mov_b32 s11, 0x3fbdee67
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], v[2:3]
	v_rcp_f64_e32 v[12:13], v[8:9]
	v_div_scale_f64 v[18:19], vcc_lo, v[2:3], v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[10:11], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[8:9], v[12:13], 1.0
	v_fma_f64 v[14:15], -v[6:7], v[10:11], 1.0
	v_fma_f64 v[12:13], v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[10:11]
	v_fma_f64 v[16:17], -v[8:9], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], -v[6:7], v[10:11], 1.0
	v_fma_f64 v[12:13], v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[10:11]
	v_div_scale_f64 v[14:15], s0, s[2:3], s[6:7], s[2:3]
	v_mul_f64 v[16:17], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[20:21], v[14:15], v[12:13]
	v_fma_f64 v[6:7], -v[6:7], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], -v[8:9], v[20:21], v[14:15]
	v_div_fmas_f64 v[6:7], v[6:7], v[10:11], v[16:17]
	s_mov_b32 vcc_lo, s0
	s_mov_b32 s0, 0x55555555
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[8:9], v[8:9], v[12:13], v[20:21]
	v_div_fixup_f64 v[2:3], v[6:7], v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[8:9], v[8:9], s[6:7], s[2:3]
	s_mov_b32 s2, 0x968915a9
	s_mov_b32 s3, 0x3fba6564
	v_cmp_neq_f64_e32 vcc_lo, 1.0, v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v3, 0x3ff00000, v3, vcc_lo
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v5, 0x3ff00000, v9, vcc_lo
	v_cndmask_b32_e32 v4, 0, v8, vcc_lo
	v_frexp_mant_f64_e64 v[6:7], |v[4:5]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[6:7]
	v_cndmask_b32_e64 v0, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v0
	v_frexp_exp_i32_f64_e32 v0, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_subrev_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	v_add_f64 v[16:17], v[8:9], -1.0
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	v_fma_f64 v[8:9], v[12:13], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[8:9]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[6:7], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[8:9], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], -v[12:13]
	v_mul_f64 v[12:13], v[8:9], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[14:15], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[14:15], v[10:11]
	v_add_f64 v[14:15], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], s[10:11], s[2:3]
	s_mov_b32 s2, 0x3abe935a
	s_mov_b32 s3, 0x3fbe25e4
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_mul_f64 v[22:23], v[8:9], v[14:15]
	s_mov_b32 s10, 0x652b82fe
	s_mov_b32 s11, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x47e6c9c2
	s_mov_b32 s3, 0x3fc110ef
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0xcfa74449
	s_mov_b32 s3, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x71bf3c30
	s_mov_b32 s3, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x1c7792ce
	s_mov_b32 s3, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x924920da
	s_mov_b32 s3, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s2, 0x9999999c
	s_mov_b32 s3, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[2:3]
	s_mov_b32 s3, 0x3c7abc9e
	s_mov_b32 s2, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[14:15], v[16:17]
	v_fma_f64 v[12:13], v[14:15], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[16:17], v[12:13]
	v_add_f64 v[16:17], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[20:21], v[16:17], s[0:1]
	v_add_f64 v[18:19], v[16:17], -v[18:19]
	s_mov_b32 s1, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[24:25], v[20:21], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_fma_f64 v[18:19], v[14:15], v[8:9], -v[22:23]
	s_mov_b32 s0, 0xd5df274d
	s_mov_b32 s1, 0x3c8543b0
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], s[0:1]
	v_fma_f64 v[14:15], v[14:15], v[6:7], v[18:19]
	s_mov_b32 s1, 0x3fe62e42
	s_mov_b32 s0, 0xfefa39ef
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], v[16:17]
	v_fma_f64 v[10:11], v[10:11], v[8:9], v[14:15]
	v_ldexp_f64 v[8:9], v[8:9], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[20:21], v[12:13]
	v_add_f64 v[16:17], v[22:23], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[20:21], -v[14:15]
	v_mul_f64 v[20:21], v[16:17], v[14:15]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], v[18:19]
	v_fma_f64 v[18:19], v[16:17], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[12:13], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[12:13]
	v_cvt_f64_i32_e32 v[14:15], v0
	v_add_f64 v[12:13], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[8:9], v[12:13]
	v_add_f64 v[18:19], v[12:13], -v[20:21]
	v_mul_f64 v[20:21], v[14:15], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[14:15], s[0:1], -v[20:21]
	s_mov_b32 s1, 0xbfe62e42
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_fma_f64 v[10:11], v[14:15], s[2:3], v[18:19]
	s_mov_b32 s3, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[16:17], v[6:7]
	v_add_f64 v[20:21], v[8:9], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[8:9], v[12:13]
	v_add_f64 v[16:17], v[12:13], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[14:15], -v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[14:15], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_add_f64 v[16:17], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[16:17], -v[10:11]
	v_add_f64 v[8:9], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[16:17], -v[12:13]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[18:19], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[12:13], v[18:19], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], -v[18:19]
	v_mul_f64 v[12:13], v[2:3], v[8:9]
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[2:3], v[8:9], -v[12:13]
	v_cmp_class_f64_e64 vcc_lo, v[12:13], 0x204
	v_fma_f64 v[6:7], v[2:3], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_dual_cndmask_b32 v11, v9, v13 :: v_dual_cndmask_b32 v10, v8, v12
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[14:15], v[10:11], s[10:11]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[10:11]|
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_trunc_f64_e32 v[8:9], v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_rndne_f64_e32 v[14:15], v[14:15]
	v_dual_cndmask_b32 v7, 0, v7 :: v_dual_cndmask_b32 v6, 0, v6
	v_cmp_lt_f64_e64 s9, |v[4:5]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], s[0:1], v[10:11]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_cvt_i32_f64_e32 v0, v[14:15]
	v_fma_f64 v[16:17], v[14:15], s[2:3], v[16:17]
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_cmp_neq_f64_e64 s3, v[2:3], |v[2:3]|
	v_cmp_eq_f64_e64 s2, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_xor_b32 s3, s3, s9
	v_cmp_class_f64_e64 s9, v[4:5], 0x204
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x40900000, v[10:11]
	v_cmp_ngt_f64_e64 s1, 0xc090cc00, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[16:17], v[18:19], 1.0
	s_and_b32 vcc_lo, s1, s0
	v_fma_f64 v[14:15], v[16:17], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[12:13], v[14:15], v0
	v_mul_f64 v[14:15], v[2:3], 0.5
	v_cndmask_b32_e64 v0, 0x7ff00000, v13, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[10:11], v[14:15]
	v_cndmask_b32_e32 v12, 0, v12, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[8:9], v[2:3]
	v_cndmask_b32_e64 v9, 0x7ff00000, 0, s3
	v_cndmask_b32_e64 v13, 0, v0, s1
	v_cmp_neq_f64_e64 s3, |v[4:5]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[12:13]
	v_cmp_class_f64_e64 s1, v[12:13], 0x204
	v_cmp_neq_f64_e64 s0, v[10:11], v[14:15]
	v_cndmask_b32_e64 v9, 0x3ff00000, v9, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, v13, s1
	v_cndmask_b32_e64 v6, v6, v12, s1
	v_cmp_gt_f64_e64 s1, 0, v[2:3]
	v_cndmask_b32_e32 v8, 0, v6, vcc_lo
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v0, 0x3ff00000, v5, s0
	v_bfi_b32 v0, 0x7fffffff, v7, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v7, 0x7ff80000, v0, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[4:5]
	s_xor_b32 s1, s1, s2
	v_cndmask_b32_e32 v6, v6, v8, vcc_lo
	v_cndmask_b32_e32 v0, v0, v7, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_cndmask_b32_e64 v7, 0x7ff00000, 0, s1
	v_cndmask_b32_e64 v8, 0, v5, s0
	s_or_b32 s0, s2, s9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_bfi_b32 v7, 0x7fffffff, v7, v8
	v_cndmask_b32_e32 v0, v0, v9, vcc_lo
	v_cndmask_b32_e64 v0, v0, v7, s0
	s_or_b32 s0, s0, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[4:5], v[2:3]
	v_cndmask_b32_e64 v6, v6, 0, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, 0, v6, vcc_lo
	v_cndmask_b32_e32 v3, 0x7ff80000, v0, vcc_lo
	v_mul_f64 v[3:4], s[6:7], v[2:3]
.LBB3_4:
	s_or_b32 exec_lo, exec_lo, s8
.LBB3_5:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s5, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB3_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel creationx_geomspace_kernel
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
		.amdhsa_next_free_vgpr 26
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
.Lfunc_end3:
	.size	creationx_geomspace_kernel, .Lfunc_end3-creationx_geomspace_kernel
                                        ; -- End function
	.set creationx_geomspace_kernel.num_vgpr, 26
	.set creationx_geomspace_kernel.num_agpr, 0
	.set creationx_geomspace_kernel.numbered_sgpr, 12
	.set creationx_geomspace_kernel.num_named_barrier, 0
	.set creationx_geomspace_kernel.private_seg_size, 0
	.set creationx_geomspace_kernel.uses_vcc, 1
	.set creationx_geomspace_kernel.uses_flat_scratch, 0
	.set creationx_geomspace_kernel.has_dyn_sized_stack, 0
	.set creationx_geomspace_kernel.has_recursion, 0
	.set creationx_geomspace_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2568
; TotalNumSgprs: 14
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 26
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
	.protected	creationx_tri_kernel    ; -- Begin function creationx_tri_kernel
	.globl	creationx_tri_kernel
	.p2align	8
	.type	creationx_tri_kernel,@function
creationx_tri_kernel:                   ; @creationx_tri_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b128 s[4:7], s[0:1], 0x8
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
	s_load_b64 s[0:1], s[0:1], 0x0
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ashrrev_i32_e32 v3, 31, v3
	v_ashrrev_i32_e32 v2, 31, v1
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v3, v0, s5
	v_add_nc_u32_e32 v4, s6, v0
	v_sub_nc_u32_e32 v3, v1, v3
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_cmp_gt_i32_e32 vcc_lo, v3, v4
	v_cndmask_b32_e64 v3, 0x3ff00000, 0, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel creationx_tri_kernel
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
	.size	creationx_tri_kernel, .Lfunc_end4-creationx_tri_kernel
                                        ; -- End function
	.set creationx_tri_kernel.num_vgpr, 5
	.set creationx_tri_kernel.num_agpr, 0
	.set creationx_tri_kernel.numbered_sgpr, 8
	.set creationx_tri_kernel.num_named_barrier, 0
	.set creationx_tri_kernel.private_seg_size, 0
	.set creationx_tri_kernel.uses_vcc, 1
	.set creationx_tri_kernel.uses_flat_scratch, 0
	.set creationx_tri_kernel.has_dyn_sized_stack, 0
	.set creationx_tri_kernel.has_recursion, 0
	.set creationx_tri_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 324
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
	.protected	creationx_eye_rect_kernel ; -- Begin function creationx_eye_rect_kernel
	.globl	creationx_eye_rect_kernel
	.p2align	8
	.type	creationx_eye_rect_kernel,@function
creationx_eye_rect_kernel:              ; @creationx_eye_rect_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b128 s[4:7], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB5_2
; %bb.1:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b64 s[0:1], s[0:1], 0x0
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ashrrev_i32_e32 v3, 31, v3
	v_ashrrev_i32_e32 v2, 31, v1
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v3, v0, s5
	v_add_nc_u32_e32 v4, s6, v0
	v_sub_nc_u32_e32 v3, v1, v3
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_cmp_eq_u32_e32 vcc_lo, v3, v4
	v_cndmask_b32_e64 v3, 0, 0x3ff00000, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel creationx_eye_rect_kernel
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
.Lfunc_end5:
	.size	creationx_eye_rect_kernel, .Lfunc_end5-creationx_eye_rect_kernel
                                        ; -- End function
	.set creationx_eye_rect_kernel.num_vgpr, 5
	.set creationx_eye_rect_kernel.num_agpr, 0
	.set creationx_eye_rect_kernel.numbered_sgpr, 8
	.set creationx_eye_rect_kernel.num_named_barrier, 0
	.set creationx_eye_rect_kernel.private_seg_size, 0
	.set creationx_eye_rect_kernel.uses_vcc, 1
	.set creationx_eye_rect_kernel.uses_flat_scratch, 0
	.set creationx_eye_rect_kernel.has_dyn_sized_stack, 0
	.set creationx_eye_rect_kernel.has_recursion, 0
	.set creationx_eye_rect_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 324
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
	.protected	creationx_arange_i32_kernel ; -- Begin function creationx_arange_i32_kernel
	.globl	creationx_arange_i32_kernel
	.p2align	8
	.type	creationx_arange_i32_kernel,@function
creationx_arange_i32_kernel:            ; @creationx_arange_i32_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b128 s[4:7], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s6, v1
	s_cbranch_execz .LBB6_2
; %bb.1:
	s_load_b64 s[0:1], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	v_mad_u64_u32 v[4:5], null, v1, s5, s[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v3, vcc_lo
	global_store_b32 v[0:1], v4, off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel creationx_arange_i32_kernel
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
		.amdhsa_inst_pref_size 1
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
	.size	creationx_arange_i32_kernel, .Lfunc_end6-creationx_arange_i32_kernel
                                        ; -- End function
	.set creationx_arange_i32_kernel.num_vgpr, 6
	.set creationx_arange_i32_kernel.num_agpr, 0
	.set creationx_arange_i32_kernel.numbered_sgpr, 8
	.set creationx_arange_i32_kernel.num_named_barrier, 0
	.set creationx_arange_i32_kernel.private_seg_size, 0
	.set creationx_arange_i32_kernel.uses_vcc, 1
	.set creationx_arange_i32_kernel.uses_flat_scratch, 0
	.set creationx_arange_i32_kernel.has_dyn_sized_stack, 0
	.set creationx_arange_i32_kernel.has_recursion, 0
	.set creationx_arange_i32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 128
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_9ba2689e70a4216b,@object ; @__hip_cuid_9ba2689e70a4216b
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_9ba2689e70a4216b
__hip_cuid_9ba2689e70a4216b:
	.byte	0                               ; 0x0
	.size	__hip_cuid_9ba2689e70a4216b, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_9ba2689e70a4216b
	.amdgpu_metadata
---
amdhsa.kernels:
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .offset:         8
        .size:           8
        .value_kind:     by_value
      - .offset:         16
        .size:           8
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
    .name:           creationx_arange_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         creationx_arange_kernel.kd
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
      - .offset:         8
        .size:           8
        .value_kind:     by_value
      - .offset:         16
        .size:           8
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
    .name:           creationx_linspace_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         creationx_linspace_kernel.kd
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
      - .offset:         8
        .size:           8
        .value_kind:     by_value
      - .offset:         16
        .size:           8
        .value_kind:     by_value
      - .offset:         24
        .size:           8
        .value_kind:     by_value
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
    .name:           creationx_logspace_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         creationx_logspace_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     26
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .offset:         8
        .size:           8
        .value_kind:     by_value
      - .offset:         16
        .size:           8
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
    .name:           creationx_geomspace_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         creationx_geomspace_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     26
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
    .name:           creationx_tri_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         creationx_tri_kernel.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .offset:         12
        .size:           4
        .value_kind:     by_value
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
    .name:           creationx_eye_rect_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         creationx_eye_rect_kernel.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .offset:         12
        .size:           4
        .value_kind:     by_value
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
    .name:           creationx_arange_i32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         creationx_arange_i32_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     6
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
