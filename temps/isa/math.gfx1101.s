	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	rsqrt_kernel            ; -- Begin function rsqrt_kernel
	.globl	rsqrt_kernel
	.p2align	8
	.type	rsqrt_kernel,@function
rsqrt_kernel:                           ; @rsqrt_kernel
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
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_rsq_f64_e32 v[4:5], v[2:3]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[2:3], v[4:5], -v[2:3]
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x180
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[2:3], v[4:5], 1.0
	v_mul_f64 v[6:7], v[4:5], v[2:3]
	v_fma_f64 v[2:3], 0x3fd80000, v[2:3], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[6:7], v[2:3], v[4:5]
	v_dual_cndmask_b32 v3, v5, v3 :: v_dual_cndmask_b32 v2, v4, v2
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel rsqrt_kernel
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
	.size	rsqrt_kernel, .Lfunc_end0-rsqrt_kernel
                                        ; -- End function
	.set rsqrt_kernel.num_vgpr, 8
	.set rsqrt_kernel.num_agpr, 0
	.set rsqrt_kernel.numbered_sgpr, 5
	.set rsqrt_kernel.num_named_barrier, 0
	.set rsqrt_kernel.private_seg_size, 0
	.set rsqrt_kernel.uses_vcc, 1
	.set rsqrt_kernel.uses_flat_scratch, 0
	.set rsqrt_kernel.has_dyn_sized_stack, 0
	.set rsqrt_kernel.has_recursion, 0
	.set rsqrt_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 232
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
	.protected	reciprocal_kernel       ; -- Begin function reciprocal_kernel
	.globl	reciprocal_kernel
	.p2align	8
	.type	reciprocal_kernel,@function
reciprocal_kernel:                      ; @reciprocal_kernel
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
	s_cbranch_execz .LBB1_2
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
	v_div_scale_f64 v[4:5], null, v[2:3], v[2:3], 1.0
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_div_scale_f64 v[8:9], vcc_lo, 1.0, v[2:3], 1.0
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[10:11], v[8:9]
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[10:11]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_div_fixup_f64 v[2:3], v[4:5], v[2:3], 1.0
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel reciprocal_kernel
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
	.size	reciprocal_kernel, .Lfunc_end1-reciprocal_kernel
                                        ; -- End function
	.set reciprocal_kernel.num_vgpr, 12
	.set reciprocal_kernel.num_agpr, 0
	.set reciprocal_kernel.numbered_sgpr, 5
	.set reciprocal_kernel.num_named_barrier, 0
	.set reciprocal_kernel.private_seg_size, 0
	.set reciprocal_kernel.uses_vcc, 1
	.set reciprocal_kernel.uses_flat_scratch, 0
	.set reciprocal_kernel.has_dyn_sized_stack, 0
	.set reciprocal_kernel.has_recursion, 0
	.set reciprocal_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 252
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
	.protected	sin_kernel              ; -- Begin function sin_kernel
	.globl	sin_kernel
	.p2align	8
	.type	sin_kernel,@function
sin_kernel:                             ; @sin_kernel
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
                                        ; implicit-def: $vgpr8
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr6_vgpr7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_ngt_f64_e64 s0, 0x41d00000, |v[2:3]|
	s_and_saveexec_b32 s1, s0
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB2_3
; %bb.2:
	v_ldexp_f64 v[4:5], |v[2:3]|, 0xffffff80
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_trig_preop_f64 v[6:7], |v[2:3]|, 0
	v_and_b32_e32 v8, 0x7fffffff, v3
	v_trig_preop_f64 v[18:19], |v[2:3]|, 2
	v_mov_b32_e32 v26, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_dual_cndmask_b32 v5, v8, v5 :: v_dual_cndmask_b32 v4, v2, v4
	v_trig_preop_f64 v[8:9], |v[2:3]|, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[6:7], v[4:5]
	v_mul_f64 v[24:25], v[18:19], v[4:5]
	v_mul_f64 v[12:13], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[4:5], -v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[4:5], v[18:19], v[4:5], -v[24:25]
	v_add_f64 v[14:15], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[12:13]
	v_add_f64 v[22:23], v[10:11], v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[16:17], v[22:23], -2
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[20:21], v[24:25], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[16:17]|
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_fract_f64_e32 v[12:13], v[16:17]
	v_add_f64 v[14:15], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[12:13], v[12:13], 2
	v_add_f64 v[16:17], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	v_add_f64 v[22:23], v[16:17], v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, 0, v[22:23]
	v_add_f64 v[22:23], v[20:21], -v[24:25]
	v_cndmask_b32_e64 v27, 0, 0x40100000, vcc_lo
	v_add_f64 v[31:32], v[20:21], -v[22:23]
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], v[26:27]
	v_add_f64 v[27:28], v[14:15], -v[20:21]
	v_add_f64 v[22:23], v[24:25], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[29:30], v[16:17], v[12:13]
	v_add_f64 v[33:34], v[14:15], -v[27:28]
	v_add_f64 v[6:7], v[6:7], -v[27:28]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[8:9], v[8:9], v[22:23]
	v_cvt_i32_f64_e32 v29, v[29:30]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[33:34]
	v_cvt_f64_i32_e32 v[27:28], v29
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[10:11]
	v_add_f64 v[18:19], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], -v[12:13]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_cndmask_b32_e64 v27, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v8, null, 0, v29, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[6:7], v[4:5]
	v_mul_f64 v[11:12], v[9:10], s[4:5]
	v_add_f64 v[6:7], v[9:10], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[9:10], s[4:5], -v[11:12]
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[9:10], s[6:7], v[13:14]
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[11:12], v[6:7]
	v_add_f64 v[9:10], v[4:5], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[9:10]
.LBB2_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB2_5
; %bb.4:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[4:5], |v[2:3]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[4:5]
	v_fma_f64 v[4:5], v[8:9], s[4:5], |v[2:3]|
	v_mul_f64 v[6:7], v[8:9], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[6:7], v[4:5]
	v_add_f64 v[10:11], v[4:5], v[6:7]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_fma_f64 v[6:7], v[8:9], s[6:7], v[6:7]
	v_add_f64 v[4:5], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_fma_f64 v[6:7], v[8:9], s[4:5], v[4:5]
	v_cvt_i32_f64_e32 v8, v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[12:13], v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
.LBB2_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_mul_f64 v[9:10], v[4:5], v[4:5]
	s_mov_b32 s0, 0xb42fdfa7
	s_mov_b32 s4, 0xf9a43bb8
	s_mov_b32 s1, 0xbe5ae600
	s_mov_b32 s5, 0x3de5e0b2
	s_mov_b32 s6, 0x796cde01
	s_mov_b32 s7, 0x3ec71de3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[19:20], v[6:7], 0.5
	v_fma_f64 v[11:12], v[9:10], s[4:5], s[0:1]
	s_mov_b32 s0, 0x9037ab78
	s_mov_b32 s4, 0x46cc5e42
	s_mov_b32 s1, 0x3e21eeb6
	s_mov_b32 s5, 0xbda907db
	v_mul_f64 v[15:16], v[9:10], 0.5
	v_fma_f64 v[13:14], v[9:10], s[4:5], s[0:1]
	s_mov_b32 s0, 0xa17f65f6
	s_mov_b32 s4, 0x19e83e5c
	s_mov_b32 s1, 0xbe927e4f
	s_mov_b32 s5, 0xbf2a01a0
	v_mul_f64 v[21:22], v[4:5], -v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[6:7]
	v_add_f64 v[17:18], -v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s0, 0x19f4ec90
	s_mov_b32 s1, 0x3efa01a0
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[4:5]
	s_mov_b32 s4, 0x11110bb3
	s_mov_b32 s5, 0x3f811111
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], -v[17:18], 1.0
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s0, 0x16c16967
	s_mov_b32 s1, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[4:5]
	v_add_f64 v[15:16], v[23:24], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s1, 0x3fa55555
	s_mov_b32 s0, 0x55555555
	v_fma_f64 v[11:12], v[21:22], v[11:12], v[19:20]
	v_mul_f64 v[19:20], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[15:16], v[4:5], -v[6:7], v[15:16]
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s1, 0xbfc55555
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[9:10], v[11:12], -v[6:7]
	v_fma_f64 v[9:10], v[19:20], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[6:7], v[21:22], s[0:1], v[6:7]
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	v_lshlrev_b32_e32 v2, 30, v8
	v_add_f64 v[9:10], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v2, v2, v3
	v_and_b32_e32 v2, 0x80000000, v2
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_and_b32_e32 v6, 1, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_eq_u32_e32 vcc_lo, 0, v6
	v_dual_cndmask_b32 v3, v10, v5 :: v_dual_cndmask_b32 v4, v9, v4
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_xor_b32_e32 v3, v3, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, 0, v4, s0
	v_cndmask_b32_e64 v3, 0x7ff80000, v3, s0
	global_store_b64 v[0:1], v[2:3], off
.LBB2_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel sin_kernel
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
		.amdhsa_next_free_vgpr 35
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
		.amdhsa_inst_pref_size 13
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
	.size	sin_kernel, .Lfunc_end2-sin_kernel
                                        ; -- End function
	.set sin_kernel.num_vgpr, 35
	.set sin_kernel.num_agpr, 0
	.set sin_kernel.numbered_sgpr, 8
	.set sin_kernel.num_named_barrier, 0
	.set sin_kernel.private_seg_size, 0
	.set sin_kernel.uses_vcc, 1
	.set sin_kernel.uses_flat_scratch, 0
	.set sin_kernel.has_dyn_sized_stack, 0
	.set sin_kernel.has_recursion, 0
	.set sin_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1608
; TotalNumSgprs: 10
; NumVgprs: 35
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 35
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
	.protected	cos_kernel              ; -- Begin function cos_kernel
	.globl	cos_kernel
	.p2align	8
	.type	cos_kernel,@function
cos_kernel:                             ; @cos_kernel
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
	s_cbranch_execz .LBB3_6
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr8
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr6_vgpr7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_ngt_f64_e64 s0, 0x41d00000, |v[2:3]|
	s_and_saveexec_b32 s1, s0
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB3_3
; %bb.2:
	v_ldexp_f64 v[4:5], |v[2:3]|, 0xffffff80
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_trig_preop_f64 v[6:7], |v[2:3]|, 0
	v_and_b32_e32 v8, 0x7fffffff, v3
	v_trig_preop_f64 v[18:19], |v[2:3]|, 2
	v_mov_b32_e32 v26, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_dual_cndmask_b32 v5, v8, v5 :: v_dual_cndmask_b32 v4, v2, v4
	v_trig_preop_f64 v[8:9], |v[2:3]|, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[6:7], v[4:5]
	v_mul_f64 v[24:25], v[18:19], v[4:5]
	v_mul_f64 v[12:13], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[4:5], -v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[4:5], v[18:19], v[4:5], -v[24:25]
	v_add_f64 v[14:15], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[12:13]
	v_add_f64 v[22:23], v[10:11], v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[16:17], v[22:23], -2
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[20:21], v[24:25], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[16:17]|
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_fract_f64_e32 v[12:13], v[16:17]
	v_add_f64 v[14:15], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[12:13], v[12:13], 2
	v_add_f64 v[16:17], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	v_add_f64 v[22:23], v[16:17], v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, 0, v[22:23]
	v_add_f64 v[22:23], v[20:21], -v[24:25]
	v_cndmask_b32_e64 v27, 0, 0x40100000, vcc_lo
	v_add_f64 v[31:32], v[20:21], -v[22:23]
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], v[26:27]
	v_add_f64 v[27:28], v[14:15], -v[20:21]
	v_add_f64 v[22:23], v[24:25], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[29:30], v[16:17], v[12:13]
	v_add_f64 v[33:34], v[14:15], -v[27:28]
	v_add_f64 v[6:7], v[6:7], -v[27:28]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[8:9], v[8:9], v[22:23]
	v_cvt_i32_f64_e32 v29, v[29:30]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[33:34]
	v_cvt_f64_i32_e32 v[27:28], v29
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[10:11]
	v_add_f64 v[18:19], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], -v[12:13]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_cndmask_b32_e64 v27, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v8, null, 0, v29, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[6:7], v[4:5]
	v_mul_f64 v[11:12], v[9:10], s[4:5]
	v_add_f64 v[6:7], v[9:10], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[9:10], s[4:5], -v[11:12]
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[9:10], s[6:7], v[13:14]
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[11:12], v[6:7]
	v_add_f64 v[9:10], v[4:5], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[9:10]
.LBB3_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB3_5
; %bb.4:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[4:5], |v[2:3]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[4:5]
	v_fma_f64 v[4:5], v[8:9], s[4:5], |v[2:3]|
	v_mul_f64 v[6:7], v[8:9], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[6:7], v[4:5]
	v_add_f64 v[10:11], v[4:5], v[6:7]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_fma_f64 v[6:7], v[8:9], s[6:7], v[6:7]
	v_add_f64 v[4:5], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_fma_f64 v[6:7], v[8:9], s[4:5], v[4:5]
	v_cvt_i32_f64_e32 v8, v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[12:13], v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
.LBB3_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_mul_f64 v[9:10], v[4:5], v[4:5]
	s_mov_b32 s0, 0xb42fdfa7
	s_mov_b32 s4, 0xf9a43bb8
	s_mov_b32 s1, 0xbe5ae600
	s_mov_b32 s5, 0x3de5e0b2
	s_mov_b32 s6, 0x796cde01
	s_mov_b32 s7, 0x3ec71de3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[19:20], v[6:7], 0.5
	v_fma_f64 v[11:12], v[9:10], s[4:5], s[0:1]
	v_mul_f64 v[15:16], v[9:10], 0.5
	s_mov_b32 s0, 0x9037ab78
	s_mov_b32 s4, 0x46cc5e42
	s_mov_b32 s1, 0x3e21eeb6
	s_mov_b32 s5, 0xbda907db
	v_mul_f64 v[21:22], v[4:5], -v[9:10]
	v_fma_f64 v[13:14], v[9:10], s[4:5], s[0:1]
	s_mov_b32 s4, 0x19e83e5c
	s_mov_b32 s5, 0xbf2a01a0
	s_mov_b32 s0, 0xa17f65f6
	s_mov_b32 s1, 0xbe927e4f
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[6:7]
	v_add_f64 v[17:18], -v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s0, 0x19f4ec90
	s_mov_b32 s1, 0x3efa01a0
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[4:5]
	s_mov_b32 s4, 0x11110bb3
	s_mov_b32 s5, 0x3f811111
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], -v[17:18], 1.0
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s0, 0x16c16967
	s_mov_b32 s1, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[4:5]
	v_add_f64 v[15:16], v[23:24], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s1, 0x3fa55555
	s_mov_b32 s0, 0x55555555
	v_fma_f64 v[11:12], v[21:22], v[11:12], v[19:20]
	v_mul_f64 v[19:20], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[15:16], v[4:5], -v[6:7], v[15:16]
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s1, 0xbfc55555
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[9:10], v[11:12], -v[6:7]
	v_fma_f64 v[9:10], v[19:20], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[6:7], v[21:22], s[0:1], v[6:7]
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	v_lshlrev_b32_e32 v2, 30, v8
	v_add_f64 v[9:10], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_and_b32_e32 v2, 0x80000000, v2
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_xor_b32_e32 v3, 0x80000000, v5
	v_and_b32_e32 v6, 1, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_eq_u32_e32 vcc_lo, 0, v6
	v_dual_cndmask_b32 v3, v3, v10 :: v_dual_cndmask_b32 v4, v4, v9
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_xor_b32_e32 v3, v3, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, 0, v4, s0
	v_cndmask_b32_e64 v3, 0x7ff80000, v3, s0
	global_store_b64 v[0:1], v[2:3], off
.LBB3_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel cos_kernel
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
		.amdhsa_next_free_vgpr 35
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
		.amdhsa_inst_pref_size 13
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
	.size	cos_kernel, .Lfunc_end3-cos_kernel
                                        ; -- End function
	.set cos_kernel.num_vgpr, 35
	.set cos_kernel.num_agpr, 0
	.set cos_kernel.numbered_sgpr, 8
	.set cos_kernel.num_named_barrier, 0
	.set cos_kernel.private_seg_size, 0
	.set cos_kernel.uses_vcc, 1
	.set cos_kernel.uses_flat_scratch, 0
	.set cos_kernel.has_dyn_sized_stack, 0
	.set cos_kernel.has_recursion, 0
	.set cos_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1612
; TotalNumSgprs: 10
; NumVgprs: 35
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 35
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
	.protected	tan_kernel              ; -- Begin function tan_kernel
	.globl	tan_kernel
	.p2align	8
	.type	tan_kernel,@function
tan_kernel:                             ; @tan_kernel
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
	s_cbranch_execz .LBB4_6
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr8
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr6_vgpr7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_ngt_f64_e64 s0, 0x41d00000, |v[2:3]|
	s_and_saveexec_b32 s1, s0
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB4_3
; %bb.2:
	v_ldexp_f64 v[4:5], |v[2:3]|, 0xffffff80
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_trig_preop_f64 v[6:7], |v[2:3]|, 0
	v_and_b32_e32 v8, 0x7fffffff, v3
	v_trig_preop_f64 v[18:19], |v[2:3]|, 2
	v_mov_b32_e32 v26, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_dual_cndmask_b32 v5, v8, v5 :: v_dual_cndmask_b32 v4, v2, v4
	v_trig_preop_f64 v[8:9], |v[2:3]|, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[6:7], v[4:5]
	v_mul_f64 v[24:25], v[18:19], v[4:5]
	v_mul_f64 v[12:13], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[4:5], -v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[4:5], v[18:19], v[4:5], -v[24:25]
	v_add_f64 v[14:15], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[12:13]
	v_add_f64 v[22:23], v[10:11], v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[16:17], v[22:23], -2
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[20:21], v[24:25], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[16:17]|
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_fract_f64_e32 v[12:13], v[16:17]
	v_add_f64 v[14:15], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[12:13], v[12:13], 2
	v_add_f64 v[16:17], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	v_add_f64 v[22:23], v[16:17], v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, 0, v[22:23]
	v_add_f64 v[22:23], v[20:21], -v[24:25]
	v_cndmask_b32_e64 v27, 0, 0x40100000, vcc_lo
	v_add_f64 v[31:32], v[20:21], -v[22:23]
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], v[26:27]
	v_add_f64 v[27:28], v[14:15], -v[20:21]
	v_add_f64 v[22:23], v[24:25], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[29:30], v[16:17], v[12:13]
	v_add_f64 v[33:34], v[14:15], -v[27:28]
	v_add_f64 v[6:7], v[6:7], -v[27:28]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[8:9], v[8:9], v[22:23]
	v_cvt_i32_f64_e32 v29, v[29:30]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[33:34]
	v_cvt_f64_i32_e32 v[27:28], v29
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[10:11]
	v_add_f64 v[18:19], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], -v[12:13]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_cndmask_b32_e64 v27, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v8, null, 0, v29, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[6:7], v[4:5]
	v_mul_f64 v[11:12], v[9:10], s[4:5]
	v_add_f64 v[6:7], v[9:10], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[9:10], s[4:5], -v[11:12]
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[9:10], s[6:7], v[13:14]
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[11:12], v[6:7]
	v_add_f64 v[9:10], v[4:5], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[9:10]
.LBB4_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB4_5
; %bb.4:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[4:5], |v[2:3]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[4:5]
	v_fma_f64 v[4:5], v[8:9], s[4:5], |v[2:3]|
	v_mul_f64 v[6:7], v[8:9], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[6:7], v[4:5]
	v_add_f64 v[10:11], v[4:5], v[6:7]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_fma_f64 v[6:7], v[8:9], s[6:7], v[6:7]
	v_add_f64 v[4:5], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_fma_f64 v[6:7], v[8:9], s[4:5], v[4:5]
	v_cvt_i32_f64_e32 v8, v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[12:13], v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
.LBB4_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_mul_f64 v[9:10], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[11:12], v[6:7], v[6:7]
	s_mov_b32 s0, 0xa9a29f71
	s_mov_b32 s4, 0xc751c08c
	s_mov_b32 s1, 0xbf078809
	s_mov_b32 s5, 0x3ef5e089
	v_and_b32_e32 v8, 1, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_eq_u32_e32 vcc_lo, 0, v8
	v_fma_f64 v[13:14], v[4:5], v[4:5], -v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[4:5], v[11:12], v[13:14]
	v_add_f64 v[9:10], v[9:10], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], s[4:5], s[0:1]
	s_mov_b32 s0, 0x90a8aae0
	s_mov_b32 s1, 0x3f17746f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0xa6fbf144
	s_mov_b32 s1, 0xbefbb44d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0xa7943acf
	s_mov_b32 s1, 0x3f21e634
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0xdeb68feb
	s_mov_b32 s1, 0x3f2d250f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0xb58c4d95
	s_mov_b32 s1, 0x3f437fd9
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x15120e2c
	s_mov_b32 s1, 0x3f57d5af
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0xe09491df
	s_mov_b32 s1, 0x3f6d6d93
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x2033784d
	s_mov_b32 s1, 0x3f8226e1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x9ac36ae2
	s_mov_b32 s1, 0x3f9664f4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x1b451c21
	s_mov_b32 s1, 0x3faba1ba
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x111185b7
	s_mov_b32 s1, 0x3fc11111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	s_mov_b32 s0, 0x555554ee
	s_mov_b32 s1, 0x3fd55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[0:1]
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	v_and_b32_e32 v3, 0x80000000, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[9:10], v[11:12]
	v_mul_f64 v[11:12], v[4:5], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[4:5], v[11:12]
	v_fma_f64 v[9:10], v[4:5], v[9:10], -v[11:12]
	v_add_f64 v[4:5], v[13:14], -v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[9:10]
	v_add_f64 v[4:5], v[11:12], -v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[6:7], v[4:5]
	v_add_f64 v[6:7], v[13:14], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[9:10], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[6:7], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[6:7], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	v_add_f64 v[11:12], v[6:7], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[6:7], v[9:10]
	v_add_f64 v[4:5], v[4:5], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[6:7], -v[13:14]
	v_fma_f64 v[4:5], v[9:10], v[4:5], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[13:14], v[4:5]
	v_add_f64 v[15:16], -v[11:12], 1.0
	v_add_f64 v[13:14], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], -v[15:16], 1.0
	v_add_f64 v[4:5], v[13:14], -v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[17:18], -v[11:12]
	v_add_f64 v[4:5], v[4:5], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[15:16], v[4:5]
	v_mul_f64 v[4:5], v[9:10], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[9:10], v[4:5]
	v_xor_b32_e32 v2, 0x80000000, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, v4, v6, vcc_lo
	v_cndmask_b32_e32 v2, v2, v7, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_xor_b32_e32 v3, v2, v3
	v_cndmask_b32_e64 v2, 0, v4, s0
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v3, 0x7ff80000, v3, s0
	global_store_b64 v[0:1], v[2:3], off
.LBB4_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel tan_kernel
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
		.amdhsa_next_free_vgpr 35
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
		.amdhsa_inst_pref_size 15
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
	.size	tan_kernel, .Lfunc_end4-tan_kernel
                                        ; -- End function
	.set tan_kernel.num_vgpr, 35
	.set tan_kernel.num_agpr, 0
	.set tan_kernel.numbered_sgpr, 8
	.set tan_kernel.num_named_barrier, 0
	.set tan_kernel.private_seg_size, 0
	.set tan_kernel.uses_vcc, 1
	.set tan_kernel.uses_flat_scratch, 0
	.set tan_kernel.has_dyn_sized_stack, 0
	.set tan_kernel.has_recursion, 0
	.set tan_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1908
; TotalNumSgprs: 10
; NumVgprs: 35
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 35
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
	.protected	log1p_kernel            ; -- Begin function log1p_kernel
	.globl	log1p_kernel
	.p2align	8
	.type	log1p_kernel,@function
log1p_kernel:                           ; @log1p_kernel
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
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s6, 0xbf559e2b
	s_mov_b32 s5, 0x3fc38538
	s_mov_b32 s7, 0x3fc3ab76
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, 0x3fe55555
	s_mov_b32 s0, 0x55555555
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], v[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_frexp_mant_f64_e32 v[6:7], v[4:5]
	v_frexp_exp_i32_f64_e32 v10, v[4:5]
	v_add_f64 v[8:9], v[4:5], -1.0
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[6:7]
	s_mov_b32 s0, 0x55555780
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], -v[4:5]
	v_add_f64 v[8:9], v[2:3], -v[8:9]
	v_subrev_co_ci_u32_e64 v26, null, 0, v10, vcc_lo
	v_add_f64 v[6:7], v[6:7], 1.0
	v_cmp_nge_f64_e32 vcc_lo, -1.0, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v12, 0, v26
	v_ldexp_f64 v[4:5], v[4:5], v12
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[10:11], v[4:5], 1.0
	v_add_f64 v[16:17], v[4:5], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], v12
	v_add_f64 v[8:9], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], 1.0
	v_add_f64 v[8:9], v[4:5], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[18:19]
	v_add_f64 v[8:9], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[6:7], v[4:5]
	v_add_f64 v[12:13], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], v[4:5]
	v_rcp_f64_e32 v[14:15], v[12:13]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[12:13], v[14:15], 1.0
	v_add_f64 v[4:5], v[4:5], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[14:15]
	v_fma_f64 v[6:7], -v[12:13], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[14:15], v[14:15]
	v_mul_f64 v[14:15], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[12:13], v[14:15]
	v_fma_f64 v[10:11], v[14:15], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[14:15], v[8:9], v[10:11]
	v_add_f64 v[22:23], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[18:19], -v[22:23]
	v_add_f64 v[16:17], v[22:23], -v[20:21]
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[18:19]
	v_add_f64 v[4:5], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[24:25], v[4:5]
	v_mul_f64 v[16:17], v[6:7], v[10:11]
	v_add_f64 v[22:23], v[24:25], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[12:13], v[16:17]
	v_add_f64 v[4:5], v[4:5], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[12:13], -v[18:19]
	v_fma_f64 v[8:9], v[16:17], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], v[8:9]
	v_add_f64 v[20:21], v[10:11], -v[12:13]
	v_add_f64 v[18:19], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	v_add_f64 v[8:9], v[18:19], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_add_f64 v[4:5], v[4:5], v[10:11]
	v_add_f64 v[10:11], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[8:9], v[10:11], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[20:21], v[4:5]
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[6:7], v[4:5]
	v_add_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[4:5]
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[6:7], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[0:1]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_cvt_f64_i32_e32 v[14:15], v26
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_add_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[16:17], v[14:15], s[0:1]
	v_ldexp_f64 v[4:5], v[4:5], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[0:1], -v[16:17]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_fma_f64 v[8:9], v[14:15], s[0:1], v[12:13]
	v_cmp_neq_f64_e64 s0, 0x7ff00000, v[2:3]
	v_cmp_ngt_f64_e64 s1, -1.0, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[16:17], v[8:9]
	s_and_b32 vcc_lo, vcc_lo, s0
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[2:3]
	v_cndmask_b32_e64 v5, 0x7ff00000, v5, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0x7ff80000, v5, s1
	v_cndmask_b32_e32 v5, 0xfff00000, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel log1p_kernel
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
		.amdhsa_next_free_vgpr 27
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
		.amdhsa_inst_pref_size 12
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
	.size	log1p_kernel, .Lfunc_end5-log1p_kernel
                                        ; -- End function
	.set log1p_kernel.num_vgpr, 27
	.set log1p_kernel.num_agpr, 0
	.set log1p_kernel.numbered_sgpr, 8
	.set log1p_kernel.num_named_barrier, 0
	.set log1p_kernel.private_seg_size, 0
	.set log1p_kernel.uses_vcc, 1
	.set log1p_kernel.uses_flat_scratch, 0
	.set log1p_kernel.has_dyn_sized_stack, 0
	.set log1p_kernel.has_recursion, 0
	.set log1p_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1432
; TotalNumSgprs: 10
; NumVgprs: 27
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 27
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
	.protected	expm1_kernel            ; -- Begin function expm1_kernel
	.globl	expm1_kernel
	.p2align	8
	.type	expm1_kernel,@function
expm1_kernel:                           ; @expm1_kernel
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
	s_cbranch_execz .LBB6_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0xbc7abc9e
	s_mov_b32 s6, 0xa9d67f34
	s_mov_b32 s7, 0x3e21f32e
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s1, 0xbfe62e42
	s_mov_b32 s0, 0xfefa39ef
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[2:3]
	v_cvt_i32_f64_e32 v10, v[4:5]
	v_cmp_eq_f64_e32 vcc_lo, 0x40900000, v[4:5]
	s_mov_b32 s1, 0x40862e42
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_nlt_f64_e64 s0, s[0:1], v[2:3]
	v_cmp_ngt_f64_e64 s1, 0xc0428000, v[2:3]
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	s_mov_b32 s4, 0x2a1b768b
	s_mov_b32 s5, 0x3e5af4eb
	v_ldexp_f64 v[10:11], 1.0, v10
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], s[6:7], s[4:5]
	s_mov_b32 s4, 0xe0ac05b
	s_mov_b32 s5, 0x3e927e50
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x1b889c29
	s_mov_b32 s5, 0x3ec71de0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x197bcfd8
	s_mov_b32 s5, 0x3efa01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x1ac1a723
	s_mov_b32 s5, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x16c18931
	s_mov_b32 s5, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x11110056
	s_mov_b32 s5, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x55555552
	s_mov_b32 s5, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x55555557
	s_mov_b32 s5, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 0.5
	v_mul_f64 v[4:5], v[6:7], v[8:9]
	v_cndmask_b32_e64 v9, v11, 0x7fe00000, vcc_lo
	v_cndmask_b32_e64 v8, v10, 0, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[8:9], -1.0
	v_fma_f64 v[4:5], v[6:7], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[8:9], v[4:5], v[10:11]
	v_add_f64 v[6:7], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v5, v5, v7 :: v_dual_cndmask_b32 v2, v4, v6
	s_and_b32 vcc_lo, s1, s0
	v_cndmask_b32_e64 v5, 0x7ff00000, v5, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, 0, v2, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v3, 0xbff00000, v5, s1
	global_store_b64 v[0:1], v[2:3], off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel expm1_kernel
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
.Lfunc_end6:
	.size	expm1_kernel, .Lfunc_end6-expm1_kernel
                                        ; -- End function
	.set expm1_kernel.num_vgpr, 12
	.set expm1_kernel.num_agpr, 0
	.set expm1_kernel.numbered_sgpr, 8
	.set expm1_kernel.num_named_barrier, 0
	.set expm1_kernel.private_seg_size, 0
	.set expm1_kernel.uses_vcc, 1
	.set expm1_kernel.uses_flat_scratch, 0
	.set expm1_kernel.has_dyn_sized_stack, 0
	.set expm1_kernel.has_recursion, 0
	.set expm1_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 676
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
	.protected	floor_kernel            ; -- Begin function floor_kernel
	.globl	floor_kernel
	.p2align	8
	.type	floor_kernel,@function
floor_kernel:                           ; @floor_kernel
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
	s_cbranch_execz .LBB7_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_floor_f64_e32 v[2:3], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB7_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel floor_kernel
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
.Lfunc_end7:
	.size	floor_kernel, .Lfunc_end7-floor_kernel
                                        ; -- End function
	.set floor_kernel.num_vgpr, 4
	.set floor_kernel.num_agpr, 0
	.set floor_kernel.numbered_sgpr, 5
	.set floor_kernel.num_named_barrier, 0
	.set floor_kernel.private_seg_size, 0
	.set floor_kernel.uses_vcc, 1
	.set floor_kernel.uses_flat_scratch, 0
	.set floor_kernel.has_dyn_sized_stack, 0
	.set floor_kernel.has_recursion, 0
	.set floor_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 152
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
	.protected	ceil_kernel             ; -- Begin function ceil_kernel
	.globl	ceil_kernel
	.p2align	8
	.type	ceil_kernel,@function
ceil_kernel:                            ; @ceil_kernel
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
	s_cbranch_execz .LBB8_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_ceil_f64_e32 v[2:3], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB8_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel ceil_kernel
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
.Lfunc_end8:
	.size	ceil_kernel, .Lfunc_end8-ceil_kernel
                                        ; -- End function
	.set ceil_kernel.num_vgpr, 4
	.set ceil_kernel.num_agpr, 0
	.set ceil_kernel.numbered_sgpr, 5
	.set ceil_kernel.num_named_barrier, 0
	.set ceil_kernel.private_seg_size, 0
	.set ceil_kernel.uses_vcc, 1
	.set ceil_kernel.uses_flat_scratch, 0
	.set ceil_kernel.has_dyn_sized_stack, 0
	.set ceil_kernel.has_recursion, 0
	.set ceil_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 152
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
	.protected	round_kernel            ; -- Begin function round_kernel
	.globl	round_kernel
	.p2align	8
	.type	round_kernel,@function
round_kernel:                           ; @round_kernel
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
	s_cbranch_execz .LBB9_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_trunc_f64_e32 v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[2:3], -v[4:5]
	v_cmp_ge_f64_e64 s0, |v[6:7]|, 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, 0, 0x3ff00000, s0
	v_bfi_b32 v3, 0x7fffffff, v2, v3
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB9_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel round_kernel
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
.Lfunc_end9:
	.size	round_kernel, .Lfunc_end9-round_kernel
                                        ; -- End function
	.set round_kernel.num_vgpr, 8
	.set round_kernel.num_agpr, 0
	.set round_kernel.numbered_sgpr, 5
	.set round_kernel.num_named_barrier, 0
	.set round_kernel.private_seg_size, 0
	.set round_kernel.uses_vcc, 1
	.set round_kernel.uses_flat_scratch, 0
	.set round_kernel.has_dyn_sized_stack, 0
	.set round_kernel.has_recursion, 0
	.set round_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 216
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
	.protected	trunc_kernel            ; -- Begin function trunc_kernel
	.globl	trunc_kernel
	.p2align	8
	.type	trunc_kernel,@function
trunc_kernel:                           ; @trunc_kernel
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
	s_cbranch_execz .LBB10_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_trunc_f64_e32 v[2:3], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB10_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel trunc_kernel
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
.Lfunc_end10:
	.size	trunc_kernel, .Lfunc_end10-trunc_kernel
                                        ; -- End function
	.set trunc_kernel.num_vgpr, 4
	.set trunc_kernel.num_agpr, 0
	.set trunc_kernel.numbered_sgpr, 5
	.set trunc_kernel.num_named_barrier, 0
	.set trunc_kernel.private_seg_size, 0
	.set trunc_kernel.uses_vcc, 1
	.set trunc_kernel.uses_flat_scratch, 0
	.set trunc_kernel.has_dyn_sized_stack, 0
	.set trunc_kernel.has_recursion, 0
	.set trunc_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 152
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
	.protected	emax_kernel             ; -- Begin function emax_kernel
	.globl	emax_kernel
	.p2align	8
	.type	emax_kernel,@function
emax_kernel:                            ; @emax_kernel
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
	s_cbranch_execz .LBB11_2
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
	v_cmp_gt_f64_e32 vcc_lo, v[2:3], v[4:5]
	v_dual_cndmask_b32 v3, v5, v3 :: v_dual_cndmask_b32 v2, v4, v2
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB11_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel emax_kernel
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
.Lfunc_end11:
	.size	emax_kernel, .Lfunc_end11-emax_kernel
                                        ; -- End function
	.set emax_kernel.num_vgpr, 6
	.set emax_kernel.num_agpr, 0
	.set emax_kernel.numbered_sgpr, 8
	.set emax_kernel.num_named_barrier, 0
	.set emax_kernel.private_seg_size, 0
	.set emax_kernel.uses_vcc, 1
	.set emax_kernel.uses_flat_scratch, 0
	.set emax_kernel.has_dyn_sized_stack, 0
	.set emax_kernel.has_recursion, 0
	.set emax_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 196
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
	.protected	emin_kernel             ; -- Begin function emin_kernel
	.globl	emin_kernel
	.p2align	8
	.type	emin_kernel,@function
emin_kernel:                            ; @emin_kernel
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
	s_cbranch_execz .LBB12_2
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
	v_cmp_lt_f64_e32 vcc_lo, v[2:3], v[4:5]
	v_dual_cndmask_b32 v3, v5, v3 :: v_dual_cndmask_b32 v2, v4, v2
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB12_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel emin_kernel
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
.Lfunc_end12:
	.size	emin_kernel, .Lfunc_end12-emin_kernel
                                        ; -- End function
	.set emin_kernel.num_vgpr, 6
	.set emin_kernel.num_agpr, 0
	.set emin_kernel.numbered_sgpr, 8
	.set emin_kernel.num_named_barrier, 0
	.set emin_kernel.private_seg_size, 0
	.set emin_kernel.uses_vcc, 1
	.set emin_kernel.uses_flat_scratch, 0
	.set emin_kernel.has_dyn_sized_stack, 0
	.set emin_kernel.has_recursion, 0
	.set emin_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 196
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
	.protected	atan2_kernel            ; -- Begin function atan2_kernel
	.globl	atan2_kernel
	.p2align	8
	.type	atan2_kernel,@function
atan2_kernel:                           ; @atan2_kernel
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
	s_cbranch_execz .LBB13_2
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s0, 0xbd3237f4
	s_mov_b32 s1, 0xbf23e260
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
	s_mov_b32 s4, 0xb5e68a13
	s_mov_b32 s5, 0x3eeba404
	s_waitcnt vmcnt(1)
	v_max_f64 v[6:7], |v[2:3]|, |v[2:3]|
	s_waitcnt vmcnt(0)
	v_max_f64 v[8:9], |v[4:5]|, |v[4:5]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_max_f64 v[10:11], v[8:9], v[6:7]
	v_min_f64 v[6:7], v[8:9], v[6:7]
	v_div_scale_f64 v[8:9], null, v[10:11], v[10:11], v[6:7]
	v_div_scale_f64 v[16:17], vcc_lo, v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[12:13], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[8:9], v[12:13], 1.0
	v_fma_f64 v[12:13], v[12:13], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[8:9], v[12:13], 1.0
	v_fma_f64 v[12:13], v[12:13], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[16:17], v[12:13]
	v_fma_f64 v[8:9], -v[8:9], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[8:9], v[8:9], v[12:13], v[14:15]
	v_cmp_lt_f64_e64 vcc_lo, |v[4:5]|, |v[2:3]|
	v_div_fixup_f64 v[6:7], v[8:9], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	v_fma_f64 v[10:11], v[8:9], s[4:5], s[0:1]
	s_mov_b32 s0, 0x69efb384
	s_mov_b32 s1, 0x3f4b2bb0
	v_cmp_class_f64_e64 s4, v[4:5], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0xaf56de9b
	s_mov_b32 s1, 0xbf67952d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0xa595c56f
	s_mov_b32 s1, 0x3f7d6d43
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0xa57d9582
	s_mov_b32 s1, 0xbf8c6ea4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x5f08b19f
	s_mov_b32 s1, 0x3f967e29
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0xfc27006a
	s_mov_b32 s1, 0xbf9e9ae6
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x5711927a
	s_mov_b32 s1, 0x3fa2c15b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0xe82d3ff0
	s_mov_b32 s1, 0xbfa59976
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x6ef28734
	s_mov_b32 s1, 0x3fa82d5d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x6a214619
	s_mov_b32 s1, 0xbfaae5ce
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x8427b883
	s_mov_b32 s1, 0x3fae1bb4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x8b207f05
	s_mov_b32 s1, 0xbfb110e4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x57b87036
	s_mov_b32 s1, 0x3fb3b136
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x19378e4f
	s_mov_b32 s1, 0xbfb745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x17e1913c
	s_mov_b32 s1, 0x3fbc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x92376b7d
	s_mov_b32 s1, 0xbfc24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x999952cc
	s_mov_b32 s1, 0x3fc99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x55555523
	s_mov_b32 s1, 0xbfd55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x54442d18
	s_mov_b32 s1, 0x3ff921fb
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[8:9], v[10:11]
	v_ashrrev_i32_e32 v11, 31, v5
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], -v[6:7], s[0:1]
	s_mov_b32 s1, 0x400921fb
	v_dual_cndmask_b32 v7, v7, v9 :: v_dual_cndmask_b32 v6, v6, v8
	v_cmp_gt_i32_e32 vcc_lo, 0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], -v[6:7], s[0:1]
	v_cmp_class_f64_e64 s1, v[2:3], 0x204
	v_cmp_eq_f64_e64 s0, 0, v[2:3]
	v_dual_mov_b32 v10, 0x7f3321d2 :: v_dual_cndmask_b32 v7, v7, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v6, v6, v8, vcc_lo
	v_mov_b32_e32 v8, 0x4002d97c
	v_cndmask_b32_e32 v10, 0x54442d18, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v8, 0x3fe921fb, v8, vcc_lo
	s_and_b32 vcc_lo, s1, s4
	v_and_b32_e32 v12, 0x54442d18, v11
	v_and_b32_e32 v11, 0x400921fb, v11
	v_cndmask_b32_e64 v7, v7, v11, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v7, v7, v8, vcc_lo
	v_cndmask_b32_e64 v6, v6, v12, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v6, v6, v10, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[4:5], v[2:3]
	v_cndmask_b32_e32 v4, 0x7ff80000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, 0, v6, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_4)
	v_bfi_b32 v3, 0x7fffffff, v4, v3
	global_store_b64 v[0:1], v[2:3], off
.LBB13_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel atan2_kernel
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
		.amdhsa_next_free_vgpr 18
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
.Lfunc_end13:
	.size	atan2_kernel, .Lfunc_end13-atan2_kernel
                                        ; -- End function
	.set atan2_kernel.num_vgpr, 18
	.set atan2_kernel.num_agpr, 0
	.set atan2_kernel.numbered_sgpr, 8
	.set atan2_kernel.num_named_barrier, 0
	.set atan2_kernel.private_seg_size, 0
	.set atan2_kernel.uses_vcc, 1
	.set atan2_kernel.uses_flat_scratch, 0
	.set atan2_kernel.has_dyn_sized_stack, 0
	.set atan2_kernel.has_recursion, 0
	.set atan2_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1144
; TotalNumSgprs: 10
; NumVgprs: 18
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
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
	.protected	fmod_kernel             ; -- Begin function fmod_kernel
	.globl	fmod_kernel
	.p2align	8
	.type	fmod_kernel,@function
fmod_kernel:                            ; @fmod_kernel
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
	s_cbranch_execz .LBB14_10
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr6_vgpr7
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
	s_waitcnt vmcnt(1)
	v_and_b32_e32 v12, 0x80000000, v3
	s_waitcnt vmcnt(0)
	v_cmp_ngt_f64_e64 s0, |v[2:3]|, |v[4:5]|
	s_and_saveexec_b32 s1, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB14_3
; %bb.2:
	v_cmp_eq_f64_e64 vcc_lo, |v[2:3]|, |v[4:5]|
	v_cndmask_b32_e32 v7, v3, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v6, v2, 0, vcc_lo
                                        ; implicit-def: $vgpr12
.LBB14_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB14_9
; %bb.4:
	v_frexp_mant_f64_e64 v[6:7], |v[4:5]|
	v_frexp_mant_f64_e64 v[17:18], |v[2:3]|
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_div_scale_f64 v[8:9], null, v[6:7], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[13:14], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[13:14], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[13:14], v[10:11]
	v_div_scale_f64 v[13:14], vcc_lo, 1.0, v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[13:14], v[10:11]
	v_fma_f64 v[8:9], -v[8:9], v[15:16], v[13:14]
	v_frexp_exp_i32_f64_e32 v14, v[2:3]
	v_frexp_exp_i32_f64_e32 v13, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[15:16]
	v_ldexp_f64 v[10:11], v[17:18], 26
	v_sub_nc_u32_e32 v14, v14, v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_div_fixup_f64 v[8:9], v[8:9], v[6:7], 1.0
	v_cmpx_lt_i32_e32 26, v14
	s_cbranch_execz .LBB14_8
; %bb.5:
	s_mov_b32 s4, 0
	.p2align	6
.LBB14_6:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[8:9], v[10:11]
	v_rndne_f64_e32 v[15:16], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[15:16], v[6:7], v[10:11]
	v_add_f64 v[15:16], v[6:7], v[10:11]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v11, v11, v16 :: v_dual_cndmask_b32 v10, v10, v15
	v_cmp_gt_u32_e32 vcc_lo, 53, v14
	v_subrev_nc_u32_e32 v14, 26, v14
	v_ldexp_f64 v[10:11], v[10:11], 26
	s_or_b32 s4, vcc_lo, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s4
	s_cbranch_execnz .LBB14_6
; %bb.7:
	s_or_b32 exec_lo, exec_lo, s4
.LBB14_8:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	s_or_b32 exec_lo, exec_lo, s1
	v_subrev_nc_u32_e32 v14, 25, v14
	v_ldexp_f64 v[10:11], v[10:11], v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[8:9], v[10:11]
	v_rndne_f64_e32 v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[8:9], v[6:7], v[10:11]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v7, v9, v7 :: v_dual_add_nc_u32 v10, -1, v13
	v_cndmask_b32_e32 v6, v8, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v10
	v_xor_b32_e32 v7, v12, v7
.LBB14_9:
	s_or_b32 exec_lo, exec_lo, s0
	v_cmp_o_f64_e32 vcc_lo, v[4:5], v[4:5]
	v_cmp_class_f64_e64 s1, v[2:3], 0x1f8
	v_cmp_neq_f64_e64 s0, 0, v[4:5]
	s_and_b32 s1, vcc_lo, s1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 vcc_lo, s1, s0
	v_cndmask_b32_e32 v3, 0x7ff80000, v7, vcc_lo
	v_cndmask_b32_e32 v2, 0, v6, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB14_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel fmod_kernel
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
		.amdhsa_next_free_vgpr 19
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
.Lfunc_end14:
	.size	fmod_kernel, .Lfunc_end14-fmod_kernel
                                        ; -- End function
	.set fmod_kernel.num_vgpr, 19
	.set fmod_kernel.num_agpr, 0
	.set fmod_kernel.numbered_sgpr, 8
	.set fmod_kernel.num_named_barrier, 0
	.set fmod_kernel.private_seg_size, 0
	.set fmod_kernel.uses_vcc, 1
	.set fmod_kernel.uses_flat_scratch, 0
	.set fmod_kernel.has_dyn_sized_stack, 0
	.set fmod_kernel.has_recursion, 0
	.set fmod_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 656
; TotalNumSgprs: 10
; NumVgprs: 19
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 19
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
	.protected	sub_scalar_kernel       ; -- Begin function sub_scalar_kernel
	.globl	sub_scalar_kernel
	.p2align	8
	.type	sub_scalar_kernel,@function
sub_scalar_kernel:                      ; @sub_scalar_kernel
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
	s_cbranch_execz .LBB15_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_load_b64 s[0:1], s[0:1], 0x0
	v_add_co_u32 v0, vcc_lo, s6, v0
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[2:3], v[2:3], -s[0:1]
	global_store_b64 v[0:1], v[2:3], off
.LBB15_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel sub_scalar_kernel
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
.Lfunc_end15:
	.size	sub_scalar_kernel, .Lfunc_end15-sub_scalar_kernel
                                        ; -- End function
	.set sub_scalar_kernel.num_vgpr, 4
	.set sub_scalar_kernel.num_agpr, 0
	.set sub_scalar_kernel.numbered_sgpr, 8
	.set sub_scalar_kernel.num_named_barrier, 0
	.set sub_scalar_kernel.private_seg_size, 0
	.set sub_scalar_kernel.uses_vcc, 1
	.set sub_scalar_kernel.uses_flat_scratch, 0
	.set sub_scalar_kernel.has_dyn_sized_stack, 0
	.set sub_scalar_kernel.has_recursion, 0
	.set sub_scalar_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 172
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
	.protected	div_scalar_kernel       ; -- Begin function div_scalar_kernel
	.globl	div_scalar_kernel
	.p2align	8
	.type	div_scalar_kernel,@function
div_scalar_kernel:                      ; @div_scalar_kernel
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
	s_cbranch_execz .LBB16_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_load_b64 s[0:1], s[0:1], 0x0
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_div_scale_f64 v[4:5], null, s[0:1], s[0:1], v[2:3]
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_div_scale_f64 v[8:9], vcc_lo, v[2:3], s[0:1], v[2:3]
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[10:11], v[8:9]
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[10:11]
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	v_div_fixup_f64 v[2:3], v[4:5], s[0:1], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB16_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel div_scalar_kernel
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
.Lfunc_end16:
	.size	div_scalar_kernel, .Lfunc_end16-div_scalar_kernel
                                        ; -- End function
	.set div_scalar_kernel.num_vgpr, 12
	.set div_scalar_kernel.num_agpr, 0
	.set div_scalar_kernel.numbered_sgpr, 8
	.set div_scalar_kernel.num_named_barrier, 0
	.set div_scalar_kernel.private_seg_size, 0
	.set div_scalar_kernel.uses_vcc, 1
	.set div_scalar_kernel.uses_flat_scratch, 0
	.set div_scalar_kernel.has_dyn_sized_stack, 0
	.set div_scalar_kernel.has_recursion, 0
	.set div_scalar_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 268
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
	.protected	rsub_scalar_kernel      ; -- Begin function rsub_scalar_kernel
	.globl	rsub_scalar_kernel
	.p2align	8
	.type	rsub_scalar_kernel,@function
rsub_scalar_kernel:                     ; @rsub_scalar_kernel
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
	s_cbranch_execz .LBB17_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_load_b64 s[0:1], s[0:1], 0x0
	v_add_co_u32 v0, vcc_lo, s6, v0
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[2:3], s[0:1], -v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB17_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel rsub_scalar_kernel
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
.Lfunc_end17:
	.size	rsub_scalar_kernel, .Lfunc_end17-rsub_scalar_kernel
                                        ; -- End function
	.set rsub_scalar_kernel.num_vgpr, 4
	.set rsub_scalar_kernel.num_agpr, 0
	.set rsub_scalar_kernel.numbered_sgpr, 8
	.set rsub_scalar_kernel.num_named_barrier, 0
	.set rsub_scalar_kernel.private_seg_size, 0
	.set rsub_scalar_kernel.uses_vcc, 1
	.set rsub_scalar_kernel.uses_flat_scratch, 0
	.set rsub_scalar_kernel.has_dyn_sized_stack, 0
	.set rsub_scalar_kernel.has_recursion, 0
	.set rsub_scalar_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 172
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
	.protected	rdiv_scalar_kernel      ; -- Begin function rdiv_scalar_kernel
	.globl	rdiv_scalar_kernel
	.p2align	8
	.type	rdiv_scalar_kernel,@function
rdiv_scalar_kernel:                     ; @rdiv_scalar_kernel
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
	s_cbranch_execz .LBB18_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	s_load_b64 s[0:1], s[0:1], 0x0
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_div_scale_f64 v[4:5], null, v[2:3], v[2:3], s[0:1]
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[6:7]
	v_div_scale_f64 v[8:9], vcc_lo, s[0:1], v[2:3], s[0:1]
	v_mul_f64 v[10:11], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[10:11], v[8:9]
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[10:11]
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	v_div_fixup_f64 v[2:3], v[4:5], v[2:3], s[0:1]
	global_store_b64 v[0:1], v[2:3], off
.LBB18_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel rdiv_scalar_kernel
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
.Lfunc_end18:
	.size	rdiv_scalar_kernel, .Lfunc_end18-rdiv_scalar_kernel
                                        ; -- End function
	.set rdiv_scalar_kernel.num_vgpr, 12
	.set rdiv_scalar_kernel.num_agpr, 0
	.set rdiv_scalar_kernel.numbered_sgpr, 8
	.set rdiv_scalar_kernel.num_named_barrier, 0
	.set rdiv_scalar_kernel.private_seg_size, 0
	.set rdiv_scalar_kernel.uses_vcc, 1
	.set rdiv_scalar_kernel.uses_flat_scratch, 0
	.set rdiv_scalar_kernel.has_dyn_sized_stack, 0
	.set rdiv_scalar_kernel.has_recursion, 0
	.set rdiv_scalar_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 268
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
	.protected	has_nan_kernel          ; -- Begin function has_nan_kernel
	.globl	has_nan_kernel
	.p2align	8
	.type	has_nan_kernel,@function
has_nan_kernel:                         ; @has_nan_kernel
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
	s_cbranch_execz .LBB19_4
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_b64 v[0:1], v[0:1], off
	s_waitcnt vmcnt(0)
	v_cmp_u_f64_e32 vcc_lo, v[0:1], v[0:1]
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB19_4
; %bb.2:
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB19_4
; %bb.3:
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, 1
	global_atomic_or_b32 v0, v1, s[2:3]
.LBB19_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel has_nan_kernel
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
		.amdhsa_next_free_vgpr 3
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
.Lfunc_end19:
	.size	has_nan_kernel, .Lfunc_end19-has_nan_kernel
                                        ; -- End function
	.set has_nan_kernel.num_vgpr, 3
	.set has_nan_kernel.num_agpr, 0
	.set has_nan_kernel.numbered_sgpr, 5
	.set has_nan_kernel.num_named_barrier, 0
	.set has_nan_kernel.private_seg_size, 0
	.set has_nan_kernel.uses_vcc, 1
	.set has_nan_kernel.uses_flat_scratch, 0
	.set has_nan_kernel.has_dyn_sized_stack, 0
	.set has_nan_kernel.has_recursion, 0
	.set has_nan_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 176
; TotalNumSgprs: 7
; NumVgprs: 3
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	isfinite_all_kernel     ; -- Begin function isfinite_all_kernel
	.globl	isfinite_all_kernel
	.p2align	8
	.type	isfinite_all_kernel,@function
isfinite_all_kernel:                    ; @isfinite_all_kernel
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
	s_cbranch_execz .LBB20_4
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_b64 v[0:1], v[0:1], off
	s_waitcnt vmcnt(0)
	v_cmp_nlg_f64_e64 s0, 0x7ff00000, |v[0:1]|
	s_and_b32 exec_lo, exec_lo, s0
	s_cbranch_execz .LBB20_4
; %bb.2:
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB20_4
; %bb.3:
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, 1
	global_atomic_or_b32 v0, v1, s[2:3]
.LBB20_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel isfinite_all_kernel
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
		.amdhsa_next_free_vgpr 3
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
.Lfunc_end20:
	.size	isfinite_all_kernel, .Lfunc_end20-isfinite_all_kernel
                                        ; -- End function
	.set isfinite_all_kernel.num_vgpr, 3
	.set isfinite_all_kernel.num_agpr, 0
	.set isfinite_all_kernel.numbered_sgpr, 5
	.set isfinite_all_kernel.num_named_barrier, 0
	.set isfinite_all_kernel.private_seg_size, 0
	.set isfinite_all_kernel.uses_vcc, 1
	.set isfinite_all_kernel.uses_flat_scratch, 0
	.set isfinite_all_kernel.has_dyn_sized_stack, 0
	.set isfinite_all_kernel.has_recursion, 0
	.set isfinite_all_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 184
; TotalNumSgprs: 7
; NumVgprs: 3
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	_Z15splitk_dw_pass1PKdS0_Pdiiii ; -- Begin function _Z15splitk_dw_pass1PKdS0_Pdiiii
	.globl	_Z15splitk_dw_pass1PKdS0_Pdiiii
	.p2align	8
	.type	_Z15splitk_dw_pass1PKdS0_Pdiiii,@function
_Z15splitk_dw_pass1PKdS0_Pdiiii:        ; @_Z15splitk_dw_pass1PKdS0_Pdiiii
; %bb.0:
	s_load_b128 s[4:7], s[0:1], 0x18
	s_abs_i32 s12, s2
	s_waitcnt lgkmcnt(0)
	s_add_i32 s8, s6, 63
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s9, s8, 31
	s_lshr_b32 s9, s9, 26
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s8, s8, s9
	s_ashr_i32 s9, s8, 6
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_abs_i32 s8, s9
	v_cvt_f32_u32_e32 v1, s8
	s_sub_i32 s11, 0, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s10, v1
	s_mul_i32 s11, s11, s10
	s_mul_hi_u32 s11, s10, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s10, s10, s11
	s_xor_b32 s11, s2, s9
	s_mul_hi_u32 s10, s12, s10
	s_ashr_i32 s11, s11, 31
	s_mul_i32 s13, s10, s8
	s_sub_i32 s12, s12, s13
	s_add_i32 s13, s10, 1
	s_sub_i32 s14, s12, s8
	s_cmp_ge_u32 s12, s8
	s_cselect_b32 s10, s13, s10
	s_cselect_b32 s12, s14, s12
	s_add_i32 s13, s10, 1
	s_cmp_ge_u32 s12, s8
	s_cselect_b32 s8, s13, s10
	s_abs_i32 s10, s7
	s_add_i32 s12, s4, s7
	v_cvt_f32_u32_e32 v1, s10
	s_sub_i32 s15, 0, s10
	s_xor_b32 s8, s8, s11
	s_add_i32 s14, s12, -1
	s_sub_i32 s12, 1, s12
	v_rcp_iflag_f32_e32 v1, v1
	s_sub_i32 s8, s8, s11
	s_max_i32 s12, s14, s12
	s_xor_b32 s7, s14, s7
	s_mul_i32 s9, s8, s9
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s13, v1
	s_mul_i32 s15, s15, s13
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s11, s13, s15
	s_add_i32 s13, s13, s11
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_hi_u32 s11, s12, s13
	s_ashr_i32 s13, s7, 31
	s_mul_i32 s14, s11, s10
	s_sub_i32 s7, s2, s9
	s_sub_i32 s2, s12, s14
	s_add_i32 s9, s11, 1
	s_sub_i32 s12, s2, s10
	s_cmp_ge_u32 s2, s10
	s_cselect_b32 s9, s9, s11
	s_cselect_b32 s2, s12, s2
	s_add_i32 s11, s9, 1
	s_cmp_ge_u32 s2, s10
	s_mov_b32 s10, s4
	s_cselect_b32 s2, s11, s9
	s_ashr_i32 s11, s4, 31
	s_xor_b32 s9, s2, s13
	s_ashr_i32 s2, s3, 31
	s_sub_i32 s9, s9, s13
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_mul_i32 s16, s9, s3
	s_ashr_i32 s13, s9, 31
	s_mul_hi_i32 s17, s9, s3
	s_add_u32 s12, s16, s9
	s_addc_u32 s13, s17, s13
	v_cmp_lt_i64_e64 s9, s[12:13], s[10:11]
	s_and_b32 s9, s9, exec_lo
	s_cselect_b32 s19, s13, s11
	s_cselect_b32 s18, s12, s4
	s_mov_b32 s9, -1
	v_cmp_lt_i64_e64 s4, s[16:17], s[18:19]
                                        ; implicit-def: $sgpr12_sgpr13
	s_and_b32 vcc_lo, exec_lo, s4
                                        ; implicit-def: $sgpr4
	s_cbranch_vccnz .LBB21_2
; %bb.1:
	s_ashr_i32 s4, s6, 31
	s_ashr_i32 s13, s5, 31
	s_mov_b32 s12, s5
	s_mov_b32 s9, 0
.LBB21_2:
	s_load_b64 s[14:15], s[0:1], 0x10
	v_mov_b32_e32 v33, 0
	v_mov_b32_e32 v31, 0
	v_mov_b32_e32 v27, 0
	v_mov_b32_e32 v25, 0
	v_mov_b32_e32 v23, 0
	v_mov_b32_e32 v21, 0
	v_mov_b32_e32 v19, 0
	v_mov_b32_e32 v17, 0
	v_mov_b32_e32 v15, 0
	v_mov_b32_e32 v13, 0
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v9, 0
	v_mov_b32_e32 v7, 0
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v34, 0 :: v_dual_and_b32 v37, 15, v0
	v_mov_b32_e32 v32, 0
	v_mov_b32_e32 v28, 0
	v_mov_b32_e32 v26, 0
	v_mov_b32_e32 v24, 0
	v_mov_b32_e32 v22, 0
	v_mov_b32_e32 v20, 0
	v_mov_b32_e32 v18, 0
	v_mov_b32_e32 v16, 0
	v_mov_b32_e32 v14, 0
	v_mov_b32_e32 v12, 0
	v_mov_b32_e32 v10, 0
	v_mov_b32_e32 v8, 0
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v2, 0
	v_lshrrev_b32_e32 v38, 4, v0
	s_lshl_b32 s20, s7, 6
	s_and_not1_b32 vcc_lo, exec_lo, s9
	s_lshl_b32 s7, s8, 6
	s_cbranch_vccnz .LBB21_13
; %bb.3:
	s_clause 0x1
	s_load_b32 s4, s[0:1], 0x34
	s_load_b128 s[8:11], s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v7, 0
	v_mov_b32_e32 v9, 0
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v13, 0
	v_mov_b32_e32 v15, 0
	v_mov_b32_e32 v17, 0
	v_mov_b32_e32 v19, 0
	v_mov_b32_e32 v21, 0
	v_mov_b32_e32 v23, 0
	v_mov_b32_e32 v25, 0
	v_mov_b32_e32 v27, 0
	v_mov_b32_e32 v31, 0
	v_mov_b32_e32 v33, 0
	v_dual_mov_b32 v2, 0 :: v_dual_lshlrev_b32 v39, 5, v38
	v_lshl_or_b32 v40, v37, 5, 0x2000
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v8, 0
	v_mov_b32_e32 v10, 0
	v_mov_b32_e32 v12, 0
	v_mov_b32_e32 v14, 0
	v_mov_b32_e32 v16, 0
	v_mov_b32_e32 v18, 0
	v_mov_b32_e32 v20, 0
	v_mov_b32_e32 v22, 0
	v_mov_b32_e32 v24, 0
	v_mov_b32_e32 v26, 0
	v_mov_b32_e32 v28, 0
	v_mov_b32_e32 v32, 0
	v_mov_b32_e32 v34, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s1, s4, 0xffff
	s_ashr_i32 s4, s6, 31
	s_ashr_i32 s13, s5, 31
	s_mov_b32 s12, s5
	s_branch .LBB21_5
.LBB21_4:                               ;   in Loop: Header=BB21_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s22
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b128 v[41:44], v39
	ds_load_b128 v[45:48], v40
	ds_load_b128 v[49:52], v40 offset:16
	ds_load_b128 v[53:56], v39 offset:16
	s_add_u32 s16, s16, 16
	s_addc_u32 s17, s17, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_ge_i64_e64 s0, s[16:17], s[18:19]
	s_and_b32 vcc_lo, exec_lo, s0
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[41:42], v[45:46], v[33:34]
	v_fma_f64 v[31:32], v[41:42], v[47:48], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[41:42], v[49:50], v[27:28]
	v_fma_f64 v[25:26], v[41:42], v[51:52], v[25:26]
	v_fma_f64 v[23:24], v[43:44], v[45:46], v[23:24]
	v_fma_f64 v[21:22], v[43:44], v[47:48], v[21:22]
	v_fma_f64 v[19:20], v[43:44], v[49:50], v[19:20]
	v_fma_f64 v[17:18], v[43:44], v[51:52], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[53:54], v[45:46], v[15:16]
	v_fma_f64 v[35:36], v[53:54], v[47:48], v[13:14]
	v_fma_f64 v[41:42], v[53:54], v[49:50], v[11:12]
	v_fma_f64 v[43:44], v[53:54], v[51:52], v[9:10]
	v_fma_f64 v[45:46], v[55:56], v[45:46], v[7:8]
	v_fma_f64 v[47:48], v[55:56], v[47:48], v[5:6]
	v_fma_f64 v[49:50], v[55:56], v[49:50], v[3:4]
	v_fma_f64 v[51:52], v[55:56], v[51:52], v[1:2]
	ds_load_b128 v[1:4], v39 offset:512
	ds_load_b128 v[5:8], v40 offset:512
	ds_load_b128 v[9:12], v40 offset:528
	ds_load_b128 v[13:16], v39 offset:528
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:1024
	ds_load_b128 v[5:8], v40 offset:1024
	ds_load_b128 v[9:12], v40 offset:1040
	ds_load_b128 v[13:16], v39 offset:1040
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:1536
	ds_load_b128 v[5:8], v40 offset:1536
	ds_load_b128 v[9:12], v40 offset:1552
	ds_load_b128 v[13:16], v39 offset:1552
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:2048
	ds_load_b128 v[5:8], v40 offset:2048
	ds_load_b128 v[9:12], v40 offset:2064
	ds_load_b128 v[13:16], v39 offset:2064
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:2560
	ds_load_b128 v[5:8], v40 offset:2560
	ds_load_b128 v[9:12], v40 offset:2576
	ds_load_b128 v[13:16], v39 offset:2576
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:3072
	ds_load_b128 v[5:8], v40 offset:3072
	ds_load_b128 v[9:12], v40 offset:3088
	ds_load_b128 v[13:16], v39 offset:3088
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:3584
	ds_load_b128 v[5:8], v40 offset:3584
	ds_load_b128 v[9:12], v40 offset:3600
	ds_load_b128 v[13:16], v39 offset:3600
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:4096
	ds_load_b128 v[5:8], v40 offset:4096
	ds_load_b128 v[9:12], v40 offset:4112
	ds_load_b128 v[13:16], v39 offset:4112
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:4608
	ds_load_b128 v[5:8], v40 offset:4608
	ds_load_b128 v[9:12], v40 offset:4624
	ds_load_b128 v[13:16], v39 offset:4624
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:5120
	ds_load_b128 v[5:8], v40 offset:5120
	ds_load_b128 v[9:12], v40 offset:5136
	ds_load_b128 v[13:16], v39 offset:5136
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:5632
	ds_load_b128 v[5:8], v40 offset:5632
	ds_load_b128 v[9:12], v40 offset:5648
	ds_load_b128 v[13:16], v39 offset:5648
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:6144
	ds_load_b128 v[5:8], v40 offset:6144
	ds_load_b128 v[9:12], v40 offset:6160
	ds_load_b128 v[13:16], v39 offset:6160
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:6656
	ds_load_b128 v[5:8], v40 offset:6656
	ds_load_b128 v[9:12], v40 offset:6672
	ds_load_b128 v[13:16], v39 offset:6672
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[33:34], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[41:42], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[43:44], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[45:46], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[47:48], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[49:50], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[51:52], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:7168
	ds_load_b128 v[5:8], v40 offset:7168
	ds_load_b128 v[9:12], v40 offset:7184
	ds_load_b128 v[13:16], v39 offset:7184
	s_waitcnt lgkmcnt(2)
	v_fma_f64 v[29:30], v[1:2], v[5:6], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[7:8], v[31:32]
	s_waitcnt lgkmcnt(1)
	v_fma_f64 v[27:28], v[1:2], v[9:10], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[11:12], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[5:6], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[7:8], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[9:10], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[11:12], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_fma_f64 v[53:54], v[13:14], v[5:6], v[33:34]
	v_fma_f64 v[35:36], v[13:14], v[7:8], v[35:36]
	v_fma_f64 v[55:56], v[13:14], v[9:10], v[41:42]
	v_fma_f64 v[57:58], v[13:14], v[11:12], v[43:44]
	v_fma_f64 v[5:6], v[15:16], v[5:6], v[45:46]
	v_fma_f64 v[59:60], v[15:16], v[7:8], v[47:48]
	v_fma_f64 v[61:62], v[15:16], v[9:10], v[49:50]
	v_fma_f64 v[63:64], v[15:16], v[11:12], v[51:52]
	ds_load_b128 v[1:4], v39 offset:7680
	ds_load_b128 v[41:44], v40 offset:7680
	ds_load_b128 v[45:48], v40 offset:7696
	ds_load_b128 v[49:52], v39 offset:7696
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_fma_f64 v[33:34], v[1:2], v[41:42], v[29:30]
	v_fma_f64 v[31:32], v[1:2], v[43:44], v[31:32]
	v_fma_f64 v[27:28], v[1:2], v[45:46], v[27:28]
	v_fma_f64 v[25:26], v[1:2], v[47:48], v[25:26]
	v_fma_f64 v[23:24], v[3:4], v[41:42], v[23:24]
	v_fma_f64 v[21:22], v[3:4], v[43:44], v[21:22]
	v_fma_f64 v[19:20], v[3:4], v[45:46], v[19:20]
	v_fma_f64 v[17:18], v[3:4], v[47:48], v[17:18]
	v_fma_f64 v[15:16], v[49:50], v[41:42], v[53:54]
	v_fma_f64 v[13:14], v[49:50], v[43:44], v[35:36]
	v_fma_f64 v[11:12], v[49:50], v[45:46], v[55:56]
	v_fma_f64 v[9:10], v[49:50], v[47:48], v[57:58]
	v_fma_f64 v[7:8], v[51:52], v[41:42], v[5:6]
	v_fma_f64 v[5:6], v[51:52], v[43:44], v[59:60]
	v_fma_f64 v[3:4], v[51:52], v[45:46], v[61:62]
	v_fma_f64 v[1:2], v[51:52], v[47:48], v[63:64]
	s_cbranch_vccnz .LBB21_13
.LBB21_5:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB21_7 Depth 2
                                        ;     Child Loop BB21_11 Depth 2
	v_mov_b32_e32 v41, v0
	s_sub_i32 s0, s18, s16
	s_mov_b32 s22, 0
	s_min_i32 s21, s0, 16
	s_branch .LBB21_7
.LBB21_6:                               ;   in Loop: Header=BB21_7 Depth=2
	s_or_b32 exec_lo, exec_lo, s0
	v_add_nc_u32_e32 v41, s1, v41
	v_lshlrev_b32_e32 v29, 3, v43
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e32 vcc_lo, 0x3ff, v41
	v_lshl_or_b32 v29, v42, 9, v29
	s_or_b32 s22, vcc_lo, s22
	s_waitcnt vmcnt(0)
	ds_store_b64 v29, v[35:36]
	s_and_not1_b32 exec_lo, exec_lo, s22
	s_cbranch_execz .LBB21_9
.LBB21_7:                               ;   Parent Loop BB21_5 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_and_b32_e32 v43, 63, v41
	v_mov_b32_e32 v35, 0
	v_lshrrev_b32_e32 v42, 6, v41
	v_mov_b32_e32 v36, 0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_or_b32_e32 v29, s20, v43
	v_cmp_gt_i32_e32 vcc_lo, s21, v42
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e64 s0, s6, v29
	s_and_b32 s23, vcc_lo, s0
	s_and_saveexec_b32 s0, s23
	s_cbranch_execz .LBB21_6
; %bb.8:                                ;   in Loop: Header=BB21_7 Depth=2
	v_add_co_u32 v30, s23, s16, v42
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v35, null, s17, 0, s23
	v_mul_lo_u32 v45, v30, s4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v44, v35, s6
	v_mad_u64_u32 v[35:36], null, v30, s6, 0
	v_ashrrev_i32_e32 v30, 31, v29
	v_lshlrev_b64 v[29:30], 3, v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add3_u32 v36, v36, v45, v44
	v_lshlrev_b64 v[35:36], 3, v[35:36]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v35, vcc_lo, s8, v35
	v_add_co_ci_u32_e64 v36, null, s9, v36, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v29, vcc_lo, v35, v29
	v_add_co_ci_u32_e64 v30, null, v36, v30, vcc_lo
	global_load_b64 v[35:36], v[29:30], off
	s_branch .LBB21_6
.LBB21_9:                               ;   in Loop: Header=BB21_5 Depth=1
	s_or_b32 exec_lo, exec_lo, s22
	v_mov_b32_e32 v41, v0
	s_mov_b32 s22, 0
	s_branch .LBB21_11
.LBB21_10:                              ;   in Loop: Header=BB21_11 Depth=2
	s_or_b32 exec_lo, exec_lo, s0
	v_add_nc_u32_e32 v41, s1, v41
	v_lshlrev_b32_e32 v29, 3, v43
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e32 vcc_lo, 0x3ff, v41
	v_lshl_or_b32 v29, v42, 9, v29
	s_or_b32 s22, vcc_lo, s22
	s_waitcnt vmcnt(0)
	ds_store_b64 v29, v[35:36] offset:8192
	s_and_not1_b32 exec_lo, exec_lo, s22
	s_cbranch_execz .LBB21_4
.LBB21_11:                              ;   Parent Loop BB21_5 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_and_b32_e32 v43, 63, v41
	v_mov_b32_e32 v35, 0
	v_lshrrev_b32_e32 v42, 6, v41
	v_mov_b32_e32 v36, 0
	v_or_b32_e32 v29, s7, v43
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e32 vcc_lo, s21, v42
	v_cmp_gt_i32_e64 s0, s5, v29
	s_and_b32 s23, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s0, s23
	s_cbranch_execz .LBB21_10
; %bb.12:                               ;   in Loop: Header=BB21_11 Depth=2
	v_add_co_u32 v30, s23, s16, v42
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v35, null, s17, 0, s23
	v_mul_lo_u32 v45, v30, s13
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v44, v35, s12
	v_mad_u64_u32 v[35:36], null, v30, s12, 0
	v_ashrrev_i32_e32 v30, 31, v29
	v_lshlrev_b64 v[29:30], 3, v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add3_u32 v36, v36, v45, v44
	v_lshlrev_b64 v[35:36], 3, v[35:36]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v35, vcc_lo, s10, v35
	v_add_co_ci_u32_e64 v36, null, s11, v36, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v29, vcc_lo, v35, v29
	v_add_co_ci_u32_e64 v30, null, v36, v30, vcc_lo
	global_load_b64 v[35:36], v[29:30], off
	s_branch .LBB21_10
.LBB21_13:
	s_mul_i32 s0, s12, s2
	s_mul_hi_u32 s1, s12, s3
	s_mul_i32 s2, s12, s3
	s_add_i32 s0, s1, s0
	s_mul_i32 s1, s13, s3
	s_mul_i32 s3, s2, s4
	s_mul_hi_u32 s4, s2, s6
	s_add_i32 s0, s0, s1
	s_add_i32 s1, s4, s3
	s_mul_i32 s0, s0, s6
	v_lshl_add_u32 v0, v38, 2, s20
	s_add_i32 s1, s1, s0
	s_mul_i32 s0, s2, s6
	v_lshl_or_b32 v29, v37, 2, s7
	s_lshl_b64 s[0:1], s[0:1], 3
	s_mov_b32 s2, exec_lo
	s_waitcnt lgkmcnt(0)
	s_add_u32 s0, s14, s0
	s_addc_u32 s1, s15, s1
	v_cmpx_gt_i32_e64 s6, v0
	s_cbranch_execz .LBB21_22
; %bb.14:
	v_ashrrev_i32_e32 v30, 31, v0
	v_mul_lo_u32 v37, s13, v0
	v_mad_u64_u32 v[35:36], null, s12, v0, 0
	s_mov_b32 s3, exec_lo
	v_mul_lo_u32 v30, s12, v30
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add3_u32 v36, v36, v30, v37
	v_lshlrev_b64 v[35:36], 3, v[35:36]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v35, vcc_lo, s0, v35
	v_add_co_ci_u32_e64 v36, null, s1, v36, vcc_lo
	v_cmpx_gt_i32_e64 s5, v29
	s_cbranch_execz .LBB21_16
; %bb.15:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[37:38], 3, v[29:30]
	v_add_co_u32 v37, vcc_lo, v35, v37
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v38, null, v36, v38, vcc_lo
	global_store_b64 v[37:38], v[33:34], off
.LBB21_16:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v30, 1, v29
	s_mov_b32 s3, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s5, v30
	s_cbranch_execz .LBB21_18
; %bb.17:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[33:34], 3, v[29:30]
	v_add_co_u32 v33, vcc_lo, v35, v33
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v34, null, v36, v34, vcc_lo
	global_store_b64 v[33:34], v[31:32], off offset:8
.LBB21_18:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v30, 2, v29
	s_mov_b32 s3, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s5, v30
	s_cbranch_execz .LBB21_20
; %bb.19:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[30:31], 3, v[29:30]
	v_add_co_u32 v30, vcc_lo, v35, v30
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v31, null, v36, v31, vcc_lo
	global_store_b64 v[30:31], v[27:28], off offset:16
.LBB21_20:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v27, 3, v29
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_i32_e32 vcc_lo, s5, v27
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB21_22
; %bb.21:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[27:28], 3, v[29:30]
	v_add_co_u32 v27, vcc_lo, v35, v27
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v28, null, v36, v28, vcc_lo
	global_store_b64 v[27:28], v[25:26], off offset:24
.LBB21_22:
	s_or_b32 exec_lo, exec_lo, s2
	v_or_b32_e32 v25, 1, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s6, v25
	s_cbranch_execz .LBB21_31
; %bb.23:
	v_ashrrev_i32_e32 v28, 31, v25
	v_mul_lo_u32 v30, s13, v25
	v_mad_u64_u32 v[26:27], null, s12, v25, 0
	s_mov_b32 s3, exec_lo
	v_mul_lo_u32 v25, s12, v28
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add3_u32 v27, v27, v25, v30
	v_lshlrev_b64 v[25:26], 3, v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v25, vcc_lo, s0, v25
	v_add_co_ci_u32_e64 v26, null, s1, v26, vcc_lo
	v_cmpx_gt_i32_e64 s5, v29
	s_cbranch_execz .LBB21_25
; %bb.24:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[27:28], 3, v[29:30]
	v_add_co_u32 v27, vcc_lo, v25, v27
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v28, null, v26, v28, vcc_lo
	global_store_b64 v[27:28], v[23:24], off
.LBB21_25:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v23, 1, v29
	s_mov_b32 s3, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s5, v23
	s_cbranch_execz .LBB21_27
; %bb.26:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[23:24], 3, v[29:30]
	v_add_co_u32 v23, vcc_lo, v25, v23
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v24, null, v26, v24, vcc_lo
	global_store_b64 v[23:24], v[21:22], off offset:8
.LBB21_27:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v21, 2, v29
	s_mov_b32 s3, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s5, v21
	s_cbranch_execz .LBB21_29
; %bb.28:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[21:22], 3, v[29:30]
	v_add_co_u32 v21, vcc_lo, v25, v21
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v22, null, v26, v22, vcc_lo
	global_store_b64 v[21:22], v[19:20], off offset:16
.LBB21_29:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v19, 3, v29
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_i32_e32 vcc_lo, s5, v19
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB21_31
; %bb.30:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[19:20], 3, v[29:30]
	v_add_co_u32 v19, vcc_lo, v25, v19
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v20, null, v26, v20, vcc_lo
	global_store_b64 v[19:20], v[17:18], off offset:24
.LBB21_31:
	s_or_b32 exec_lo, exec_lo, s2
	v_or_b32_e32 v17, 2, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s6, v17
	s_cbranch_execz .LBB21_40
; %bb.32:
	v_ashrrev_i32_e32 v20, 31, v17
	v_mul_lo_u32 v21, s13, v17
	v_mad_u64_u32 v[18:19], null, s12, v17, 0
	s_mov_b32 s3, exec_lo
	v_mul_lo_u32 v17, s12, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add3_u32 v19, v19, v17, v21
	v_lshlrev_b64 v[17:18], 3, v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v17, vcc_lo, s0, v17
	v_add_co_ci_u32_e64 v18, null, s1, v18, vcc_lo
	v_cmpx_gt_i32_e64 s5, v29
	s_cbranch_execz .LBB21_34
; %bb.33:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[19:20], 3, v[29:30]
	v_add_co_u32 v19, vcc_lo, v17, v19
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v20, null, v18, v20, vcc_lo
	global_store_b64 v[19:20], v[15:16], off
.LBB21_34:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v15, 1, v29
	s_mov_b32 s3, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s5, v15
	s_cbranch_execz .LBB21_36
; %bb.35:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[15:16], 3, v[29:30]
	v_add_co_u32 v15, vcc_lo, v17, v15
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v16, null, v18, v16, vcc_lo
	global_store_b64 v[15:16], v[13:14], off offset:8
.LBB21_36:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v13, 2, v29
	s_mov_b32 s3, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s5, v13
	s_cbranch_execz .LBB21_38
; %bb.37:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[13:14], 3, v[29:30]
	v_add_co_u32 v13, vcc_lo, v17, v13
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v14, null, v18, v14, vcc_lo
	global_store_b64 v[13:14], v[11:12], off offset:16
.LBB21_38:
	s_or_b32 exec_lo, exec_lo, s3
	v_or_b32_e32 v11, 3, v29
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_i32_e32 vcc_lo, s5, v11
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB21_40
; %bb.39:
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[11:12], 3, v[29:30]
	v_add_co_u32 v11, vcc_lo, v17, v11
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v12, null, v18, v12, vcc_lo
	global_store_b64 v[11:12], v[9:10], off offset:24
.LBB21_40:
	s_or_b32 exec_lo, exec_lo, s2
	v_or_b32_e32 v0, 3, v0
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s6, v0
	s_cbranch_execz .LBB21_49
; %bb.41:
	v_ashrrev_i32_e32 v11, 31, v0
	v_mul_lo_u32 v12, s13, v0
	v_mad_u64_u32 v[9:10], null, s12, v0, 0
	v_ashrrev_i32_e32 v30, 31, v29
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v0, s12, v11
	v_add3_u32 v10, v10, v0, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[9:10], 3, v[9:10]
	v_add_co_u32 v0, s0, s0, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s1, v10, s0
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_i32_e64 s5, v29
	s_cbranch_execz .LBB21_43
; %bb.42:
	v_lshlrev_b64 v[10:11], 3, v[29:30]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v10, vcc_lo, v0, v10
	v_add_co_ci_u32_e64 v11, null, v9, v11, vcc_lo
	global_store_b64 v[10:11], v[7:8], off
.LBB21_43:
	s_or_b32 exec_lo, exec_lo, s0
	v_or_b32_e32 v7, 1, v29
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s5, v7
	s_cbranch_execz .LBB21_45
; %bb.44:
	v_lshlrev_b64 v[7:8], 3, v[29:30]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, v0, v7
	v_add_co_ci_u32_e64 v8, null, v9, v8, vcc_lo
	global_store_b64 v[7:8], v[5:6], off offset:8
.LBB21_45:
	s_or_b32 exec_lo, exec_lo, s0
	v_or_b32_e32 v5, 2, v29
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s5, v5
	s_cbranch_execz .LBB21_47
; %bb.46:
	v_lshlrev_b64 v[5:6], 3, v[29:30]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, v0, v5
	v_add_co_ci_u32_e64 v6, null, v9, v6, vcc_lo
	global_store_b64 v[5:6], v[3:4], off offset:16
.LBB21_47:
	s_or_b32 exec_lo, exec_lo, s0
	v_or_b32_e32 v3, 3, v29
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_i32_e32 vcc_lo, s5, v3
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB21_49
; %bb.48:
	v_lshlrev_b64 v[3:4], 3, v[29:30]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, v0, v3
	v_add_co_ci_u32_e64 v4, null, v9, v4, vcc_lo
	global_store_b64 v[3:4], v[1:2], off offset:24
.LBB21_49:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15splitk_dw_pass1PKdS0_Pdiiii
		.amdhsa_group_segment_fixed_size 16384
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
		.amdhsa_system_sgpr_workgroup_id_y 1
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 0
		.amdhsa_next_free_vgpr 65
		.amdhsa_next_free_sgpr 24
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
		.amdhsa_inst_pref_size 44
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.text
.Lfunc_end21:
	.size	_Z15splitk_dw_pass1PKdS0_Pdiiii, .Lfunc_end21-_Z15splitk_dw_pass1PKdS0_Pdiiii
                                        ; -- End function
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.num_vgpr, 65
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.num_agpr, 0
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.numbered_sgpr, 24
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.num_named_barrier, 0
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.private_seg_size, 0
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.uses_vcc, 1
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.uses_flat_scratch, 0
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.has_dyn_sized_stack, 0
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.has_recursion, 0
	.set _Z15splitk_dw_pass1PKdS0_Pdiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 5600
; TotalNumSgprs: 26
; NumVgprs: 65
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 16384 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 8
; NumSGPRsForWavesPerEU: 26
; NumVGPRsForWavesPerEU: 65
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	_Z15splitk_dw_pass2PKdPdii ; -- Begin function _Z15splitk_dw_pass2PKdPdii
	.globl	_Z15splitk_dw_pass2PKdPdii
	.p2align	8
	.type	_Z15splitk_dw_pass2PKdPdii,@function
_Z15splitk_dw_pass2PKdPdii:             ; @_Z15splitk_dw_pass2PKdPdii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB22_7
; %bb.1:
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_gt_i32 s5, 0
	s_mov_b32 s6, 0
	s_cbranch_scc0 .LBB22_3
; %bb.2:
	s_mov_b32 s6, -1
.LBB22_3:
	s_load_b64 s[2:3], s[0:1], 0x8
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_and_not1_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccnz .LBB22_6
; %bb.4:
	s_load_b64 s[0:1], s[0:1], 0x0
	v_lshlrev_b64 v[5:6], 3, v[1:2]
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_ashr_i32 s7, s4, 31
	s_mov_b32 s6, s4
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v5, vcc_lo, s0, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s1, v6, vcc_lo
	s_lshl_b64 s[0:1], s[6:7], 3
.LBB22_5:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[7:8], v[5:6], off
	v_add_co_u32 v5, vcc_lo, v5, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v6, null, s1, v6, vcc_lo
	s_add_i32 s5, s5, -1
	s_cmp_eq_u32 s5, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	s_cbranch_scc0 .LBB22_5
.LBB22_6:
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB22_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15splitk_dw_pass2PKdPdii
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
.Lfunc_end22:
	.size	_Z15splitk_dw_pass2PKdPdii, .Lfunc_end22-_Z15splitk_dw_pass2PKdPdii
                                        ; -- End function
	.set _Z15splitk_dw_pass2PKdPdii.num_vgpr, 9
	.set _Z15splitk_dw_pass2PKdPdii.num_agpr, 0
	.set _Z15splitk_dw_pass2PKdPdii.numbered_sgpr, 8
	.set _Z15splitk_dw_pass2PKdPdii.num_named_barrier, 0
	.set _Z15splitk_dw_pass2PKdPdii.private_seg_size, 0
	.set _Z15splitk_dw_pass2PKdPdii.uses_vcc, 1
	.set _Z15splitk_dw_pass2PKdPdii.uses_flat_scratch, 0
	.set _Z15splitk_dw_pass2PKdPdii.has_dyn_sized_stack, 0
	.set _Z15splitk_dw_pass2PKdPdii.has_recursion, 0
	.set _Z15splitk_dw_pass2PKdPdii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 260
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
	.section	.text._Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii,"axG",@progbits,_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii,comdat
	.protected	_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii ; -- Begin function _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii
	.globl	_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii
	.p2align	8
	.type	_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii,@function
_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii: ; @_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	v_bfe_u32 v3, v0, 10, 10
	v_and_b32_e32 v0, 0x3ff, v0
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s3, s3, 16
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e64 s2, s5, v0
	v_cmp_gt_i32_e32 vcc_lo, s4, v1
	s_and_b32 s2, s2, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB23_6
; %bb.1:
	s_load_b64 s[2:3], s[0:1], 0x10
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB23_4
; %bb.2:
	s_load_b128 s[8:11], s[0:1], 0x0
	v_mul_lo_u32 v2, v1, s6
	v_dual_mov_b32 v6, 0 :: v_dual_lshlrev_b32 v7, 2, v0
	s_ashr_i32 s1, s5, 31
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[4:5], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, s0, s10, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_co_u32 v4, vcc_lo, s8, v4
	v_add_co_ci_u32_e64 v3, null, s11, 0, s0
	v_add_co_ci_u32_e64 v5, null, s9, v5, vcc_lo
	s_mov_b32 s0, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b64 s[0:1], s[0:1], 2
	.p2align	6
.LBB23_3:                               ; =>This Inner Loop Header: Depth=1
	global_load_b32 v7, v[4:5], off
	global_load_b32 v8, v[2:3], off
	v_add_co_u32 v2, vcc_lo, v2, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	v_add_co_u32 v4, vcc_lo, v4, 4
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_add_i32 s6, s6, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s6, 0
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v6, v7, v8
	s_cbranch_scc0 .LBB23_3
	s_branch .LBB23_5
.LBB23_4:
	v_mov_b32_e32 v6, 0
.LBB23_5:
	v_mad_u64_u32 v[2:3], null, v1, s5, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b32 v2, v[0:1], off
	s_waitcnt vmcnt(0)
	v_add_f32_e32 v2, v6, v2
	global_store_b32 v[0:1], v2, off
.LBB23_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii
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
		.amdhsa_system_vgpr_workitem_id 1
		.amdhsa_next_free_vgpr 9
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
		.amdhsa_inst_pref_size 3
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii,"axG",@progbits,_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii,comdat
.Lfunc_end23:
	.size	_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii, .Lfunc_end23-_Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii
                                        ; -- End function
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.num_vgpr, 9
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.num_agpr, 0
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.numbered_sgpr, 12
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.num_named_barrier, 0
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.private_seg_size, 0
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.uses_vcc, 1
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.has_recursion, 0
	.set _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 360
; TotalNumSgprs: 14
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 9
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 1
	.section	.text._Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii,"axG",@progbits,_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii,comdat
	.protected	_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii ; -- Begin function _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii
	.globl	_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii
	.p2align	8
	.type	_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii,@function
_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii: ; @_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	v_bfe_u32 v3, v0, 10, 10
	v_and_b32_e32 v0, 0x3ff, v0
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s3, s3, 16
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e64 s2, s5, v0
	v_cmp_gt_i32_e32 vcc_lo, s4, v1
	s_and_b32 s2, s2, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB24_6
; %bb.1:
	s_load_b64 s[2:3], s[0:1], 0x10
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB24_4
; %bb.2:
	s_load_b128 s[8:11], s[0:1], 0x0
	v_mul_lo_u32 v4, v1, s6
	v_lshlrev_b32_e32 v8, 3, v0
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_ashr_i32 s1, s5, 31
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v4
	v_lshlrev_b64 v[6:7], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v4, s0, s10, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_co_u32 v6, vcc_lo, s8, v6
	v_add_co_ci_u32_e64 v5, null, s11, 0, s0
	v_add_co_ci_u32_e64 v7, null, s9, v7, vcc_lo
	s_mov_b32 s0, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b64 s[0:1], s[0:1], 3
	.p2align	6
.LBB24_3:                               ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[6:7], off
	global_load_b64 v[10:11], v[4:5], off
	v_add_co_u32 v4, vcc_lo, v4, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s1, v5, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, 8
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	s_add_i32 s6, s6, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s6, 0
	s_waitcnt vmcnt(0)
	v_fma_f64 v[2:3], v[8:9], v[10:11], v[2:3]
	s_cbranch_scc0 .LBB24_3
	s_branch .LBB24_5
.LBB24_4:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
.LBB24_5:
	v_mad_u64_u32 v[4:5], null, v1, s5, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v4
	v_lshlrev_b64 v[0:1], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[4:5], v[0:1], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB24_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii
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
		.amdhsa_system_vgpr_workitem_id 1
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
		.amdhsa_inst_pref_size 3
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii,"axG",@progbits,_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii,comdat
.Lfunc_end24:
	.size	_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii, .Lfunc_end24-_Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii
                                        ; -- End function
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.num_vgpr, 12
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.num_agpr, 0
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.numbered_sgpr, 12
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.num_named_barrier, 0
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.private_seg_size, 0
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.uses_vcc, 1
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.uses_flat_scratch, 0
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.has_dyn_sized_stack, 0
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.has_recursion, 0
	.set _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 376
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
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 1
	.text
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_4b6c6c44af1fe877,@object ; @__hip_cuid_4b6c6c44af1fe877
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_4b6c6c44af1fe877
__hip_cuid_4b6c6c44af1fe877:
	.byte	0                               ; 0x0
	.size	__hip_cuid_4b6c6c44af1fe877, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_4b6c6c44af1fe877
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
    .name:           rsqrt_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         rsqrt_kernel.kd
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
    .name:           reciprocal_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         reciprocal_kernel.kd
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
    .name:           sin_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         sin_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     35
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
    .name:           cos_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         cos_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     35
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
    .name:           tan_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         tan_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     35
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
    .name:           log1p_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         log1p_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     27
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
    .name:           expm1_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         expm1_kernel.kd
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
    .name:           floor_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         floor_kernel.kd
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
    .name:           ceil_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         ceil_kernel.kd
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
    .name:           round_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         round_kernel.kd
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
    .name:           trunc_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         trunc_kernel.kd
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
    .name:           emax_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         emax_kernel.kd
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
    .name:           emin_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         emin_kernel.kd
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
    .name:           atan2_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         atan2_kernel.kd
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
    .name:           fmod_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         fmod_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     19
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
    .name:           sub_scalar_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         sub_scalar_kernel.kd
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
    .name:           div_scalar_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         div_scalar_kernel.kd
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
    .name:           rsub_scalar_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         rsub_scalar_kernel.kd
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
    .name:           rdiv_scalar_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         rdiv_scalar_kernel.kd
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
    .name:           has_nan_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         has_nan_kernel.kd
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
    .name:           isfinite_all_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         isfinite_all_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     3
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .actual_access:  read_only
        .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .actual_access:  read_only
        .address_space:  global
        .offset:         8
        .size:           8
        .value_kind:     global_buffer
      - .actual_access:  write_only
        .address_space:  global
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
    .group_segment_fixed_size: 16384
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z15splitk_dw_pass1PKdS0_Pdiiii
    .private_segment_fixed_size: 0
    .sgpr_count:     26
    .sgpr_spill_count: 0
    .symbol:         _Z15splitk_dw_pass1PKdS0_Pdiiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     65
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .actual_access:  read_only
        .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .actual_access:  write_only
        .address_space:  global
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
    .name:           _Z15splitk_dw_pass2PKdPdii
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         _Z15splitk_dw_pass2PKdPdii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     9
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .actual_access:  read_only
        .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .actual_access:  read_only
        .address_space:  global
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
    .name:           _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z24tall_skinny_dgemm_kernelIfEvPKT_S2_PS0_iii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     9
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
  - .args:
      - .actual_access:  read_only
        .address_space:  global
        .offset:         0
        .size:           8
        .value_kind:     global_buffer
      - .actual_access:  read_only
        .address_space:  global
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
    .name:           _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z24tall_skinny_dgemm_kernelIdEvPKT_S2_PS0_iii.kd
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
