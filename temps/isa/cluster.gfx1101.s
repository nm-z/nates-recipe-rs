	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	fixed_radius_kernel     ; -- Begin function fixed_radius_kernel
	.globl	fixed_radius_kernel
	.p2align	8
	.type	fixed_radius_kernel,@function
fixed_radius_kernel:                    ; @fixed_radius_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s8, s8
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_7
; %bb.1:
	s_abs_i32 s10, s8
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s10
	s_sub_i32 s2, 0, s10
	s_cmp_lt_i32 s9, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s2, v0
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v3, v0
	v_mul_lo_u32 v2, v0, s10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v3, v2
	v_add_nc_u32_e32 v3, 1, v0
	v_subrev_nc_u32_e32 v4, s10, v2
	v_cmp_le_u32_e32 vcc_lo, s10, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_xor_b32_e32 v3, s8, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_cmp_le_u32_e32 vcc_lo, s10, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v0, v0, v3
	s_cbranch_scc1 .LBB0_4
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v2, v0, s8
	v_mul_lo_u32 v4, v0, s9
	v_sub_nc_u32_e32 v2, v1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v5, 31, v4
	v_mul_lo_u32 v2, v2, s9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[8:9], 3, v[4:5]
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[6:7], 3, v[2:3]
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v4, vcc_lo, s4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s5, v7, vcc_lo
	v_add_co_u32 v6, vcc_lo, s4, v8
	v_add_co_ci_u32_e64 v7, null, s5, v9, vcc_lo
	.p2align	6
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[6:7], off
	global_load_b64 v[10:11], v[4:5], off
	v_add_co_u32 v4, vcc_lo, v4, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, 8
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	s_add_i32 s9, s9, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s9, 0
	s_waitcnt vmcnt(0)
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_fma_f64 v[2:3], v[8:9], v[8:9], v[2:3]
	s_cbranch_scc0 .LBB0_3
	s_branch .LBB0_5
.LBB0_4:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
.LBB0_5:
	s_load_b64 s[0:1], s[0:1], 0x20
	v_ashrrev_i32_e32 v4, 31, v1
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_ge_f64_e32 vcc_lo, s[0:1], v[2:3]
	v_add_co_u32 v2, s0, s6, v1
	v_add_co_ci_u32_e64 v3, null, s7, v4, s0
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b16_e32 v1.l, v5.l
	global_store_b8 v[2:3], v1, off
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB0_7
; %bb.6:
	v_ashrrev_i32_e32 v1, 31, v0
	v_mov_b32_e32 v2, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_atomic_add_u32 v[0:1], v2, off
.LBB0_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel fixed_radius_kernel
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
		.amdhsa_next_free_sgpr 11
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
	.size	fixed_radius_kernel, .Lfunc_end0-fixed_radius_kernel
                                        ; -- End function
	.set fixed_radius_kernel.num_vgpr, 12
	.set fixed_radius_kernel.num_agpr, 0
	.set fixed_radius_kernel.numbered_sgpr, 11
	.set fixed_radius_kernel.num_named_barrier, 0
	.set fixed_radius_kernel.private_seg_size, 0
	.set fixed_radius_kernel.uses_vcc, 1
	.set fixed_radius_kernel.uses_flat_scratch, 0
	.set fixed_radius_kernel.has_dyn_sized_stack, 0
	.set fixed_radius_kernel.has_recursion, 0
	.set fixed_radius_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 592
; TotalNumSgprs: 13
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 13
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
	.protected	uf_init_kernel          ; -- Begin function uf_init_kernel
	.globl	uf_init_kernel
	.p2align	8
	.type	uf_init_kernel,@function
uf_init_kernel:                         ; @uf_init_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x1c
	s_load_b32 s4, s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_2
; %bb.1:
	s_load_b64 s[0:1], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	global_store_b32 v[2:3], v1, off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel uf_init_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 272
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
.Lfunc_end1:
	.size	uf_init_kernel, .Lfunc_end1-uf_init_kernel
                                        ; -- End function
	.set uf_init_kernel.num_vgpr, 4
	.set uf_init_kernel.num_agpr, 0
	.set uf_init_kernel.numbered_sgpr, 5
	.set uf_init_kernel.num_named_barrier, 0
	.set uf_init_kernel.private_seg_size, 0
	.set uf_init_kernel.uses_vcc, 1
	.set uf_init_kernel.uses_flat_scratch, 0
	.set uf_init_kernel.has_dyn_sized_stack, 0
	.set uf_init_kernel.has_recursion, 0
	.set uf_init_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 120
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
	.protected	uf_hook_kernel          ; -- Begin function uf_hook_kernel
	.globl	uf_hook_kernel
	.p2align	8
	.type	uf_hook_kernel,@function
uf_hook_kernel:                         ; @uf_hook_kernel
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
	s_cbranch_execz .LBB2_9
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v4, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v5, null, s3, v1, vcc_lo
	global_load_b32 v1, v[2:3], off
	global_load_b32 v2, v[4:5], off
	s_mov_b32 s0, 0
.LBB2_2:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(1)
	v_mov_b32_e32 v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v1, 31, v0
	v_lshlrev_b64 v[3:4], 2, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s4, v3
	v_add_co_ci_u32_e64 v4, null, s5, v4, vcc_lo
	global_load_b32 v1, v[3:4], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v1, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB2_2
; %bb.3:
	s_or_b32 exec_lo, exec_lo, s0
	s_mov_b32 s0, 0
.LBB2_4:                                ; =>This Inner Loop Header: Depth=1
	v_mov_b32_e32 v1, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v2, 31, v1
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v2
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v2, v1
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB2_4
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s0
	v_cmp_ne_u32_e32 vcc_lo, v0, v1
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB2_9
; %bb.6:
	v_min_i32_e32 v2, v0, v1
	v_max_i32_e32 v0, v0, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v3, 31, v2
	v_mov_b32_e32 v1, v2
	v_lshlrev_b64 v[3:4], 2, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s4, v3
	v_add_co_ci_u32_e64 v4, null, s5, v4, vcc_lo
	global_atomic_cmpswap_b32 v0, v[3:4], v[0:1], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v0, v2
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB2_9
; %bb.7:
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mbcnt_lo_u32_b32 v0, s0, 0
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_and_b32 s1, exec_lo, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 exec_lo, s1
	s_cbranch_execz .LBB2_9
; %bb.8:
	s_bcnt1_i32_b32 s0, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v0, 0 :: v_dual_mov_b32 v1, s0
	global_atomic_add_u32 v0, v1, s[6:7]
.LBB2_9:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel uf_hook_kernel
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
	.size	uf_hook_kernel, .Lfunc_end2-uf_hook_kernel
                                        ; -- End function
	.set uf_hook_kernel.num_vgpr, 6
	.set uf_hook_kernel.num_agpr, 0
	.set uf_hook_kernel.numbered_sgpr, 8
	.set uf_hook_kernel.num_named_barrier, 0
	.set uf_hook_kernel.private_seg_size, 0
	.set uf_hook_kernel.uses_vcc, 1
	.set uf_hook_kernel.uses_flat_scratch, 0
	.set uf_hook_kernel.has_dyn_sized_stack, 0
	.set uf_hook_kernel.has_recursion, 0
	.set uf_hook_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 452
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
	.protected	uf_compress_kernel      ; -- Begin function uf_compress_kernel
	.globl	uf_compress_kernel
	.p2align	8
	.type	uf_compress_kernel,@function
uf_compress_kernel:                     ; @uf_compress_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x1c
	s_load_b32 s4, s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB3_4
; %bb.1:
	s_load_b64 s[0:1], s[0:1], 0x0
	v_mov_b32_e32 v0, v1
	s_mov_b32 s2, 0
.LBB3_2:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v3, v0
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 2, v[3:4]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v4, vcc_lo, s0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v5, null, s1, v5, vcc_lo
	global_load_b32 v0, v[4:5], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, v0, v3
	s_or_b32 s2, vcc_lo, s2
	s_and_not1_b32 exec_lo, exec_lo, s2
	s_cbranch_execnz .LBB3_2
; %bb.3:
	s_or_b32 exec_lo, exec_lo, s2
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b32 v[0:1], v3, off
.LBB3_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel uf_compress_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 272
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
	.size	uf_compress_kernel, .Lfunc_end3-uf_compress_kernel
                                        ; -- End function
	.set uf_compress_kernel.num_vgpr, 6
	.set uf_compress_kernel.num_agpr, 0
	.set uf_compress_kernel.numbered_sgpr, 5
	.set uf_compress_kernel.num_named_barrier, 0
	.set uf_compress_kernel.private_seg_size, 0
	.set uf_compress_kernel.uses_vcc, 1
	.set uf_compress_kernel.uses_flat_scratch, 0
	.set uf_compress_kernel.has_dyn_sized_stack, 0
	.set uf_compress_kernel.has_recursion, 0
	.set uf_compress_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 204
; TotalNumSgprs: 7
; NumVgprs: 6
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 7
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
	.protected	boruvka_init_kernel     ; -- Begin function boruvka_init_kernel
	.globl	boruvka_init_kernel
	.p2align	8
	.type	boruvka_init_kernel,@function
boruvka_init_kernel:                    ; @boruvka_init_kernel
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
	s_cbranch_execz .LBB4_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	v_mov_b32_e32 v0, -1
	v_bfrev_b32_e32 v7, -2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	v_lshlrev_b64 v[5:6], 3, v[1:2]
	v_mov_b32_e32 v1, v0
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v3
	v_add_co_ci_u32_e64 v3, null, s1, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s2, v5
	v_add_co_ci_u32_e64 v5, null, s3, v6, vcc_lo
	global_store_b32 v[2:3], v7, off
	global_store_b64 v[4:5], v[0:1], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel boruvka_init_kernel
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
.Lfunc_end4:
	.size	boruvka_init_kernel, .Lfunc_end4-boruvka_init_kernel
                                        ; -- End function
	.set boruvka_init_kernel.num_vgpr, 8
	.set boruvka_init_kernel.num_agpr, 0
	.set boruvka_init_kernel.numbered_sgpr, 5
	.set boruvka_init_kernel.num_named_barrier, 0
	.set boruvka_init_kernel.private_seg_size, 0
	.set boruvka_init_kernel.uses_vcc, 1
	.set boruvka_init_kernel.uses_flat_scratch, 0
	.set boruvka_init_kernel.has_dyn_sized_stack, 0
	.set boruvka_init_kernel.has_recursion, 0
	.set boruvka_init_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 168
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
	.protected	boruvka_min_w_kernel    ; -- Begin function boruvka_min_w_kernel
	.globl	boruvka_min_w_kernel
	.p2align	8
	.type	boruvka_min_w_kernel,@function
boruvka_min_w_kernel:                   ; @boruvka_min_w_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x3c
	s_load_b32 s4, s[0:1], 0x28
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB5_3
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v5, vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s5, v4, vcc_lo
	v_add_co_u32 v3, vcc_lo, s6, v3
	v_add_co_ci_u32_e64 v4, null, s7, v4, vcc_lo
	global_load_b32 v5, v[5:6], off
	global_load_b32 v3, v[3:4], off
	s_waitcnt vmcnt(1)
	v_ashrrev_i32_e32 v6, 31, v5
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[5:6], 2, v[5:6]
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s10, v5
	v_add_co_ci_u32_e64 v6, null, s11, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s10, v3
	v_add_co_ci_u32_e64 v4, null, s11, v4, vcc_lo
	s_clause 0x1
	global_load_b32 v0, v[5:6], off
	global_load_b32 v3, v[3:4], off
	s_waitcnt vmcnt(0)
	v_cmp_ne_u32_e32 vcc_lo, v0, v3
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB5_3
; %bb.2:
	v_lshlrev_b64 v[1:2], 3, v[1:2]
	s_load_b64 s[2:3], s[0:1], 0x20
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, vcc_lo, s8, v1
	v_add_co_ci_u32_e64 v2, null, s9, v2, vcc_lo
	global_load_b64 v[5:6], v[1:2], off
	v_ashrrev_i32_e32 v1, 31, v0
	v_lshlrev_b64 v[2:3], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, s0, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, s0
	s_waitcnt vmcnt(0)
	v_cmp_gt_i64_e32 vcc_lo, 0, v[5:6]
	v_not_b32_e32 v4, v6
	v_not_b32_e32 v7, v5
	v_or_b32_e32 v8, 0x80000000, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v5, v5, v7 :: v_dual_cndmask_b32 v6, v8, v4
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_clause 0x1
	global_atomic_min_u64 v[0:1], v[5:6], off
	global_atomic_min_u64 v[2:3], v[5:6], off
.LBB5_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel boruvka_min_w_kernel
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
.Lfunc_end5:
	.size	boruvka_min_w_kernel, .Lfunc_end5-boruvka_min_w_kernel
                                        ; -- End function
	.set boruvka_min_w_kernel.num_vgpr, 9
	.set boruvka_min_w_kernel.num_agpr, 0
	.set boruvka_min_w_kernel.numbered_sgpr, 12
	.set boruvka_min_w_kernel.num_named_barrier, 0
	.set boruvka_min_w_kernel.private_seg_size, 0
	.set boruvka_min_w_kernel.uses_vcc, 1
	.set boruvka_min_w_kernel.uses_flat_scratch, 0
	.set boruvka_min_w_kernel.has_dyn_sized_stack, 0
	.set boruvka_min_w_kernel.has_recursion, 0
	.set boruvka_min_w_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 424
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
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	boruvka_min_e_kernel    ; -- Begin function boruvka_min_e_kernel
	.globl	boruvka_min_e_kernel
	.p2align	8
	.type	boruvka_min_e_kernel,@function
boruvka_min_e_kernel:                   ; @boruvka_min_e_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b32 s4, s[0:1], 0x30
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB6_6
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v5, vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s5, v4, vcc_lo
	v_add_co_u32 v3, vcc_lo, s6, v3
	v_add_co_ci_u32_e64 v4, null, s7, v4, vcc_lo
	global_load_b32 v5, v[5:6], off
	global_load_b32 v3, v[3:4], off
	s_waitcnt vmcnt(1)
	v_ashrrev_i32_e32 v6, 31, v5
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[5:6], 2, v[5:6]
	v_lshlrev_b64 v[3:4], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s10, v5
	v_add_co_ci_u32_e64 v6, null, s11, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v7, vcc_lo, s10, v3
	v_add_co_ci_u32_e64 v8, null, s11, v4, vcc_lo
	s_clause 0x1
	global_load_b32 v4, v[5:6], off
	global_load_b32 v3, v[7:8], off
	s_waitcnt vmcnt(0)
	v_cmp_ne_u32_e32 vcc_lo, v4, v3
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB6_6
; %bb.2:
	s_load_b128 s[0:3], s[0:1], 0x20
	v_lshlrev_b64 v[6:7], 3, v[1:2]
	v_ashrrev_i32_e32 v5, 31, v4
	s_mov_b32 s4, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[8:9], 3, v[4:5]
	v_add_co_u32 v6, vcc_lo, s8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s9, v7, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v8, vcc_lo, s0, v8
	v_add_co_ci_u32_e64 v9, null, s1, v9, vcc_lo
	global_load_b64 v[8:9], v[8:9], off
	s_waitcnt vmcnt(1)
	v_cmp_gt_i64_e32 vcc_lo, 0, v[6:7]
	v_not_b32_e32 v0, v7
	v_or_b32_e32 v2, 0x80000000, v7
	v_not_b32_e32 v10, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, v2, v0, vcc_lo
	v_cndmask_b32_e32 v6, v6, v10, vcc_lo
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u64_e64 v[6:7], v[8:9]
	s_cbranch_execz .LBB6_4
; %bb.3:
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s2, v4
	v_add_co_ci_u32_e64 v5, null, s3, v5, vcc_lo
	global_atomic_min_i32 v[4:5], v1, off
.LBB6_4:
	s_or_b32 exec_lo, exec_lo, s4
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[8:9], 3, v[3:4]
	v_add_co_u32 v8, vcc_lo, s0, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s1, v9, vcc_lo
	global_load_b64 v[8:9], v[8:9], off
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[6:7], v[8:9]
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB6_6
; %bb.5:
	v_lshlrev_b64 v[2:3], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	global_atomic_min_i32 v[2:3], v1, off
.LBB6_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel boruvka_min_e_kernel
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
.Lfunc_end6:
	.size	boruvka_min_e_kernel, .Lfunc_end6-boruvka_min_e_kernel
                                        ; -- End function
	.set boruvka_min_e_kernel.num_vgpr, 11
	.set boruvka_min_e_kernel.num_agpr, 0
	.set boruvka_min_e_kernel.numbered_sgpr, 12
	.set boruvka_min_e_kernel.num_named_barrier, 0
	.set boruvka_min_e_kernel.private_seg_size, 0
	.set boruvka_min_e_kernel.uses_vcc, 1
	.set boruvka_min_e_kernel.uses_flat_scratch, 0
	.set boruvka_min_e_kernel.has_dyn_sized_stack, 0
	.set boruvka_min_e_kernel.has_recursion, 0
	.set boruvka_min_e_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 540
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
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	boruvka_mark_kernel     ; -- Begin function boruvka_mark_kernel
	.globl	boruvka_mark_kernel
	.p2align	8
	.type	boruvka_mark_kernel,@function
boruvka_mark_kernel:                    ; @boruvka_mark_kernel
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
	s_cbranch_execz .LBB7_3
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_b32 v0, v[0:1], off
	s_waitcnt vmcnt(0)
	v_cmp_lt_i32_e32 vcc_lo, -1, v0
	v_cmp_gt_i32_e64 s0, s5, v0
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 exec_lo, exec_lo, s0
	s_cbranch_execz .LBB7_3
; %bb.2:
	v_mov_b16_e32 v1.l, 1
	global_store_b8 v0, v1, s[2:3]
.LBB7_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel boruvka_mark_kernel
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
.Lfunc_end7:
	.size	boruvka_mark_kernel, .Lfunc_end7-boruvka_mark_kernel
                                        ; -- End function
	.set boruvka_mark_kernel.num_vgpr, 3
	.set boruvka_mark_kernel.num_agpr, 0
	.set boruvka_mark_kernel.numbered_sgpr, 6
	.set boruvka_mark_kernel.num_named_barrier, 0
	.set boruvka_mark_kernel.private_seg_size, 0
	.set boruvka_mark_kernel.uses_vcc, 1
	.set boruvka_mark_kernel.uses_flat_scratch, 0
	.set boruvka_mark_kernel.has_dyn_sized_stack, 0
	.set boruvka_mark_kernel.has_recursion, 0
	.set boruvka_mark_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 164
; TotalNumSgprs: 8
; NumVgprs: 3
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
; NumVGPRsForWavesPerEU: 3
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
	.protected	core_distance_kernel    ; -- Begin function core_distance_kernel
	.globl	core_distance_kernel
	.p2align	8
	.type	core_distance_kernel,@function
core_distance_kernel:                   ; @core_distance_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[8:11], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB8_29
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_mul_lo_u32 v7, v1, s9
	s_cmp_gt_i32 s8, 0
	s_mov_b32 s3, 0
	s_cselect_b32 s11, -1, 0
	s_cmp_lt_i32 s8, 1
	s_delay_alu instid0(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	s_cbranch_scc1 .LBB8_11
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b64 v[5:6], 3, v[7:8]
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_gt_i32 s9, 0
	s_mov_b32 s2, s3
	s_cselect_b32 s12, -1, 0
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v9, vcc_lo, s4, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s5, v6, vcc_lo
	v_dual_mov_b32 v5, -1 :: v_dual_mov_b32 v6, 0x7fefffff
	s_mov_b32 s13, s3
	s_branch .LBB8_6
.LBB8_3:                                ;   in Loop: Header=BB8_6 Depth=1
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v12, 0
.LBB8_4:                                ;   in Loop: Header=BB8_6 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_lt_f64_e32 vcc_lo, v[11:12], v[5:6]
	v_cmp_gt_f64_e64 s0, v[11:12], v[3:4]
	v_dual_cndmask_b32 v6, v6, v12 :: v_dual_cndmask_b32 v5, v5, v11
	v_cndmask_b32_e64 v4, v4, v12, s0
	v_cndmask_b32_e64 v3, v3, v11, s0
.LBB8_5:                                ;   in Loop: Header=BB8_6 Depth=1
	s_or_b32 exec_lo, exec_lo, s14
	s_add_i32 s13, s13, 1
	s_add_i32 s2, s2, s9
	s_cmp_eq_u32 s13, s8
	s_cbranch_scc1 .LBB8_12
.LBB8_6:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB8_9 Depth 2
	s_mov_b32 s14, exec_lo
	v_cmpx_ne_u32_e64 s13, v1
	s_cbranch_execz .LBB8_5
; %bb.7:                                ;   in Loop: Header=BB8_6 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s12
	s_cbranch_vccnz .LBB8_3
; %bb.8:                                ;   in Loop: Header=BB8_6 Depth=1
	v_dual_mov_b32 v11, 0 :: v_dual_mov_b32 v14, v10
	s_lshl_b64 s[0:1], s[2:3], 3
	v_dual_mov_b32 v12, 0 :: v_dual_mov_b32 v13, v9
	s_add_u32 s0, s4, s0
	s_addc_u32 s1, s5, s1
	s_mov_b32 s15, s9
	.p2align	6
.LBB8_9:                                ;   Parent Loop BB8_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[15:16], v[13:14], off
	s_load_b64 s[16:17], s[0:1], 0x0
	v_add_co_u32 v13, vcc_lo, v13, 8
	s_add_i32 s15, s15, -1
	v_add_co_ci_u32_e64 v14, null, 0, v14, vcc_lo
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_cmp_eq_u32 s15, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[15:16], v[15:16], -s[16:17]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[11:12], v[15:16], v[15:16], v[11:12]
	s_cbranch_scc0 .LBB8_9
; %bb.10:                               ;   in Loop: Header=BB8_6 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[11:12]
	v_cndmask_b32_e64 v0, 0, 0x100, vcc_lo
	v_ldexp_f64 v[11:12], v[11:12], v0
	v_cndmask_b32_e64 v0, 0, 0xffffff80, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[13:14], v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[15:16], v[11:12], v[13:14]
	v_mul_f64 v[13:14], v[13:14], 0.5
	v_fma_f64 v[17:18], -v[13:14], v[15:16], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[15:16], v[17:18], v[15:16]
	v_fma_f64 v[13:14], v[13:14], v[17:18], v[13:14]
	v_fma_f64 v[17:18], -v[15:16], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[17:18], -v[15:16], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[17:18], v[13:14], v[15:16]
	v_ldexp_f64 v[13:14], v[13:14], v0
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_cndmask_b32 v12, v14, v12 :: v_dual_cndmask_b32 v11, v13, v11
	s_branch .LBB8_4
.LBB8_11:
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v6, 0x7fefffff
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, -1
.LBB8_12:
	s_cmp_lt_i32 s10, 2
	s_cbranch_scc1 .LBB8_26
; %bb.13:
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_cmp_gt_i32 s9, 0
	s_mov_b32 s2, 0x812dea11
	s_mov_b32 s1, 0
	s_cselect_b32 s14, -1, 0
	s_mov_b32 s3, 0x3d719799
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v7, vcc_lo, s4, v7
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s5, v8, vcc_lo
	s_mov_b32 s15, 0
	s_mov_b32 s16, 0
	s_branch .LBB8_16
.LBB8_14:                               ;   in Loop: Header=BB8_16 Depth=1
	v_cmp_gt_i32_e64 s0, s10, v0
.LBB8_15:                               ;   in Loop: Header=BB8_16 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v6, v6, v10, s0
	v_cndmask_b32_e64 v5, v5, v9, s0
	v_cndmask_b32_e64 v4, v10, v4, s0
	v_cndmask_b32_e64 v3, v9, v3, s0
	s_add_i32 s16, s16, 1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s16, 64
	v_add_f64 v[9:10], v[3:4], -v[5:6]
	s_cselect_b32 s0, -1, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[9:10]
	s_or_b32 s0, vcc_lo, s0
	s_and_b32 s0, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_or_b32 s15, s0, s15
	s_and_not1_b32 exec_lo, exec_lo, s15
	s_cbranch_execz .LBB8_27
.LBB8_16:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB8_21 Depth 2
                                        ;       Child Loop BB8_24 Depth 3
	v_add_f64 v[9:10], v[3:4], -v[5:6]
	s_and_not1_b32 vcc_lo, exec_lo, s11
	s_mov_b32 s0, -1
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], 0.5, v[5:6]
	s_cbranch_vccnz .LBB8_15
; %bb.17:                               ;   in Loop: Header=BB8_16 Depth=1
	v_mov_b32_e32 v0, 0
	s_mov_b32 s0, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s17, s0
	s_branch .LBB8_21
.LBB8_18:                               ;   in Loop: Header=BB8_21 Depth=2
	v_mov_b32_e32 v11, 0
	v_mov_b32_e32 v12, 0
.LBB8_19:                               ;   in Loop: Header=BB8_21 Depth=2
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_f64_e32 vcc_lo, v[11:12], v[9:10]
	v_add_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
.LBB8_20:                               ;   in Loop: Header=BB8_21 Depth=2
	s_or_b32 exec_lo, exec_lo, s18
	s_add_i32 s17, s17, 1
	s_add_i32 s0, s0, s9
	s_cmp_eq_u32 s17, s8
	s_cbranch_scc1 .LBB8_14
.LBB8_21:                               ;   Parent Loop BB8_16 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB8_24 Depth 3
	s_mov_b32 s18, exec_lo
	v_cmpx_ne_u32_e64 s17, v1
	s_cbranch_execz .LBB8_20
; %bb.22:                               ;   in Loop: Header=BB8_21 Depth=2
	s_and_not1_b32 vcc_lo, exec_lo, s14
	s_cbranch_vccnz .LBB8_18
; %bb.23:                               ;   in Loop: Header=BB8_21 Depth=2
	v_dual_mov_b32 v11, 0 :: v_dual_mov_b32 v14, v8
	s_lshl_b64 s[12:13], s[0:1], 3
	v_dual_mov_b32 v12, 0 :: v_dual_mov_b32 v13, v7
	s_add_u32 s12, s4, s12
	s_addc_u32 s13, s5, s13
	s_mov_b32 s19, s9
	.p2align	6
.LBB8_24:                               ;   Parent Loop BB8_16 Depth=1
                                        ;     Parent Loop BB8_21 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	global_load_b64 v[15:16], v[13:14], off
	s_load_b64 s[20:21], s[12:13], 0x0
	v_add_co_u32 v13, vcc_lo, v13, 8
	s_add_i32 s19, s19, -1
	v_add_co_ci_u32_e64 v14, null, 0, v14, vcc_lo
	s_add_u32 s12, s12, 8
	s_addc_u32 s13, s13, 0
	s_cmp_eq_u32 s19, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[15:16], v[15:16], -s[20:21]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[11:12], v[15:16], v[15:16], v[11:12]
	s_cbranch_scc0 .LBB8_24
; %bb.25:                               ;   in Loop: Header=BB8_21 Depth=2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[11:12]
	v_cndmask_b32_e64 v2, 0, 0x100, vcc_lo
	v_ldexp_f64 v[11:12], v[11:12], v2
	v_cndmask_b32_e64 v2, 0, 0xffffff80, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[13:14], v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x260
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[15:16], v[11:12], v[13:14]
	v_mul_f64 v[13:14], v[13:14], 0.5
	v_fma_f64 v[17:18], -v[13:14], v[15:16], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[15:16], v[17:18], v[15:16]
	v_fma_f64 v[13:14], v[13:14], v[17:18], v[13:14]
	v_fma_f64 v[17:18], -v[15:16], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[17:18], -v[15:16], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[17:18], v[13:14], v[15:16]
	v_ldexp_f64 v[13:14], v[13:14], v2
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_cndmask_b32 v12, v14, v12 :: v_dual_cndmask_b32 v11, v13, v11
	s_branch .LBB8_19
.LBB8_26:
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v3, v5 :: v_dual_mov_b32 v4, v6
	s_branch .LBB8_28
.LBB8_27:
	s_or_b32 exec_lo, exec_lo, s15
.LBB8_28:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB8_29:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel core_distance_kernel
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
		.amdhsa_next_free_sgpr 22
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
.Lfunc_end8:
	.size	core_distance_kernel, .Lfunc_end8-core_distance_kernel
                                        ; -- End function
	.set core_distance_kernel.num_vgpr, 19
	.set core_distance_kernel.num_agpr, 0
	.set core_distance_kernel.numbered_sgpr, 22
	.set core_distance_kernel.num_named_barrier, 0
	.set core_distance_kernel.private_seg_size, 0
	.set core_distance_kernel.uses_vcc, 1
	.set core_distance_kernel.uses_flat_scratch, 0
	.set core_distance_kernel.has_dyn_sized_stack, 0
	.set core_distance_kernel.has_recursion, 0
	.set core_distance_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1232
; TotalNumSgprs: 24
; NumVgprs: 19
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 24
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
	.protected	fixed_radius_count_kernel ; -- Begin function fixed_radius_count_kernel
	.globl	fixed_radius_count_kernel
	.p2align	8
	.type	fixed_radius_count_kernel,@function
fixed_radius_count_kernel:              ; @fixed_radius_count_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[8:9], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB9_10
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	s_cmp_lt_i32 s8, 1
	s_cbranch_scc1 .LBB9_8
; %bb.2:
	s_load_b64 s[0:1], s[0:1], 0x18
	v_mul_lo_u32 v4, v1, s9
	v_mov_b32_e32 v0, 0
	s_cmp_gt_i32 s9, 0
	s_cselect_b32 s10, -1, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v4
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s4, v4
	s_load_b64 s[0:1], s[0:1], 0x0
	v_add_co_ci_u32_e64 v5, null, s5, v5, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[2:3], s[0:1], s[0:1]
	s_mov_b32 s1, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, s1
	s_mov_b32 s11, s1
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB9_5
	.p2align	6
.LBB9_3:                                ;   in Loop: Header=BB9_5 Depth=1
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0
.LBB9_4:                                ;   in Loop: Header=BB9_5 Depth=1
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_f64_e32 vcc_lo, v[6:7], v[2:3]
	s_add_i32 s11, s11, 1
	s_add_i32 s0, s0, s9
	s_cmp_eq_u32 s11, s8
	v_add_co_ci_u32_e64 v0, null, 0, v0, vcc_lo
	s_cbranch_scc1 .LBB9_9
.LBB9_5:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB9_7 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s10
	s_cbranch_vccnz .LBB9_3
; %bb.6:                                ;   in Loop: Header=BB9_5 Depth=1
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v9, v5
	s_lshl_b64 s[2:3], s[0:1], 3
	v_dual_mov_b32 v7, 0 :: v_dual_mov_b32 v8, v4
	s_add_u32 s2, s4, s2
	s_addc_u32 s3, s5, s3
	s_mov_b32 s12, s9
	.p2align	6
.LBB9_7:                                ;   Parent Loop BB9_5 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[10:11], v[8:9], off
	s_load_b64 s[14:15], s[2:3], 0x0
	v_add_co_u32 v8, vcc_lo, v8, 8
	s_add_i32 s12, s12, -1
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	s_add_u32 s2, s2, 8
	s_addc_u32 s3, s3, 0
	s_cmp_eq_u32 s12, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[10:11], v[10:11], -s[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[10:11], v[10:11], v[6:7]
	s_cbranch_scc0 .LBB9_7
	s_branch .LBB9_4
.LBB9_8:
	v_mov_b32_e32 v0, 0
.LBB9_9:
	s_set_inst_prefetch_distance 0x2
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[1:2], 2, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v1, vcc_lo, s6, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s7, v2, vcc_lo
	global_store_b32 v[1:2], v0, off
.LBB9_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel fixed_radius_count_kernel
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
.Lfunc_end9:
	.size	fixed_radius_count_kernel, .Lfunc_end9-fixed_radius_count_kernel
                                        ; -- End function
	.set fixed_radius_count_kernel.num_vgpr, 12
	.set fixed_radius_count_kernel.num_agpr, 0
	.set fixed_radius_count_kernel.numbered_sgpr, 16
	.set fixed_radius_count_kernel.num_named_barrier, 0
	.set fixed_radius_count_kernel.private_seg_size, 0
	.set fixed_radius_count_kernel.uses_vcc, 1
	.set fixed_radius_count_kernel.uses_flat_scratch, 0
	.set fixed_radius_count_kernel.has_dyn_sized_stack, 0
	.set fixed_radius_count_kernel.has_recursion, 0
	.set fixed_radius_count_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 408
; TotalNumSgprs: 18
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 18
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
	.protected	exclusive_scan_i32_kernel ; -- Begin function exclusive_scan_i32_kernel
	.globl	exclusive_scan_i32_kernel
	.p2align	8
	.type	exclusive_scan_i32_kernel,@function
exclusive_scan_i32_kernel:              ; @exclusive_scan_i32_kernel
; %bb.0:
	s_load_b32 s3, s[0:1], 0x24
	v_sub_nc_u32_e32 v0, 0, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, s2, v0
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB10_6
; %bb.1:
	s_clause 0x1
	s_load_b32 s4, s[0:1], 0x10
	s_load_b128 s[0:3], s[0:1], 0x0
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s4, 1
	s_cbranch_scc1 .LBB10_4
; %bb.2:
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v0, 0
	s_mov_b64 s[6:7], s[2:3]
	s_mov_b32 s5, s4
.LBB10_3:                               ; =>This Inner Loop Header: Depth=1
	global_store_b32 v1, v0, s[6:7]
	global_load_b32 v2, v1, s[0:1]
	s_add_i32 s5, s5, -1
	s_add_u32 s6, s6, 4
	s_addc_u32 s7, s7, 0
	s_add_u32 s0, s0, 4
	s_addc_u32 s1, s1, 0
	s_cmp_eq_u32 s5, 0
	s_waitcnt vmcnt(0)
	v_add_nc_u32_e32 v0, v2, v0
	s_cbranch_scc0 .LBB10_3
	s_branch .LBB10_5
.LBB10_4:
	v_mov_b32_e32 v0, 0
.LBB10_5:
	s_ashr_i32 s5, s4, 31
	v_mov_b32_e32 v1, 0
	s_lshl_b64 s[0:1], s[4:5], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s0, s2, s0
	s_addc_u32 s1, s3, s1
	global_store_b32 v1, v0, s[0:1]
.LBB10_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel exclusive_scan_i32_kernel
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
.Lfunc_end10:
	.size	exclusive_scan_i32_kernel, .Lfunc_end10-exclusive_scan_i32_kernel
                                        ; -- End function
	.set exclusive_scan_i32_kernel.num_vgpr, 3
	.set exclusive_scan_i32_kernel.num_agpr, 0
	.set exclusive_scan_i32_kernel.numbered_sgpr, 8
	.set exclusive_scan_i32_kernel.num_named_barrier, 0
	.set exclusive_scan_i32_kernel.private_seg_size, 0
	.set exclusive_scan_i32_kernel.uses_vcc, 1
	.set exclusive_scan_i32_kernel.uses_flat_scratch, 0
	.set exclusive_scan_i32_kernel.has_dyn_sized_stack, 0
	.set exclusive_scan_i32_kernel.has_recursion, 0
	.set exclusive_scan_i32_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 192
; TotalNumSgprs: 10
; NumVgprs: 3
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 10
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
	.protected	masked_weight_sum_kernel ; -- Begin function masked_weight_sum_kernel
	.globl	masked_weight_sum_kernel
	.p2align	8
	.type	masked_weight_sum_kernel,@function
masked_weight_sum_kernel:               ; @masked_weight_sum_kernel
; %bb.0:
	s_load_b32 s3, s[0:1], 0x2c
	v_sub_nc_u32_e32 v0, 0, v0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, s2, v0
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB11_7
; %bb.1:
	s_clause 0x2
	s_load_b32 s2, s[0:1], 0x18
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[0:1], s[0:1], 0x10
	v_mov_b32_e32 v0, 0
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s2, 1
	s_cbranch_scc1 .LBB11_6
; %bb.2:
	v_mov_b32_e32 v2, 0
	s_branch .LBB11_4
	.p2align	6
.LBB11_3:                               ;   in Loop: Header=BB11_4 Depth=1
	s_add_i32 s2, s2, -1
	s_add_u32 s6, s6, 1
	s_addc_u32 s7, s7, 0
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
	s_cmp_eq_u32 s2, 0
	s_cbranch_scc1 .LBB11_6
.LBB11_4:                               ; =>This Inner Loop Header: Depth=1
	global_load_u8 v3, v2, s[6:7]
	s_waitcnt vmcnt(0)
	v_cmp_eq_u32_e32 vcc_lo, 0, v3
	s_cbranch_vccnz .LBB11_3
; %bb.5:                                ;   in Loop: Header=BB11_4 Depth=1
	s_load_b64 s[8:9], s[4:5], 0x0
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[0:1], v[0:1], s[8:9]
	s_branch .LBB11_3
.LBB11_6:
	v_mov_b32_e32 v2, 0
	global_store_b64 v2, v[0:1], s[0:1]
.LBB11_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel masked_weight_sum_kernel
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
	.size	masked_weight_sum_kernel, .Lfunc_end11-masked_weight_sum_kernel
                                        ; -- End function
	.set masked_weight_sum_kernel.num_vgpr, 4
	.set masked_weight_sum_kernel.num_agpr, 0
	.set masked_weight_sum_kernel.numbered_sgpr, 10
	.set masked_weight_sum_kernel.num_named_barrier, 0
	.set masked_weight_sum_kernel.private_seg_size, 0
	.set masked_weight_sum_kernel.uses_vcc, 1
	.set masked_weight_sum_kernel.uses_flat_scratch, 0
	.set masked_weight_sum_kernel.has_dyn_sized_stack, 0
	.set masked_weight_sum_kernel.has_recursion, 0
	.set masked_weight_sum_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 192
; TotalNumSgprs: 12
; NumVgprs: 4
; ScratchSize: 0
; MemoryBound: 1
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 12
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
	.protected	fixed_radius_fill_csr_kernel ; -- Begin function fixed_radius_fill_csr_kernel
	.globl	fixed_radius_fill_csr_kernel
	.p2align	8
	.type	fixed_radius_fill_csr_kernel,@function
fixed_radius_fill_csr_kernel:           ; @fixed_radius_fill_csr_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b64 s[8:9], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s3, 0
	s_mov_b32 s2, exec_lo
	v_max_i32_e32 v0, 0, v1
	v_cmpx_gt_i32_e64 s8, v0
	s_cbranch_execz .LBB12_9
; %bb.1:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[10:11], s[0:1], 0x10
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	v_mul_lo_u32 v4, v1, s9
	s_cmp_gt_i32 s9, 0
	s_mov_b32 s2, s3
	v_lshlrev_b64 v[2:3], 2, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v5, 31, v4
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s6, v2
	v_add_co_ci_u32_e64 v3, null, s7, v3, vcc_lo
	s_load_b64 s[0:1], s[0:1], 0x0
	v_add_co_u32 v4, vcc_lo, s4, v4
	global_load_b32 v0, v[2:3], off
	v_add_co_ci_u32_e64 v5, null, s5, v5, vcc_lo
	s_cselect_b32 s6, -1, 0
	s_mov_b32 s7, s3
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[2:3], s[0:1], s[0:1]
	s_branch .LBB12_3
.LBB12_2:                               ;   in Loop: Header=BB12_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s0
	s_add_i32 s7, s7, 1
	s_add_i32 s2, s2, s9
	s_cmp_lg_u32 s7, s8
	s_cbranch_scc0 .LBB12_9
.LBB12_3:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB12_5 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s6
	s_cbranch_vccnz .LBB12_6
; %bb.4:                                ;   in Loop: Header=BB12_3 Depth=1
	v_dual_mov_b32 v6, 0 :: v_dual_mov_b32 v9, v5
	s_lshl_b64 s[0:1], s[2:3], 3
	v_dual_mov_b32 v7, 0 :: v_dual_mov_b32 v8, v4
	s_add_u32 s0, s4, s0
	s_addc_u32 s1, s5, s1
	s_mov_b32 s12, s9
	.p2align	6
.LBB12_5:                               ;   Parent Loop BB12_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[10:11], v[8:9], off
	s_load_b64 s[14:15], s[0:1], 0x0
	v_add_co_u32 v8, vcc_lo, v8, 8
	s_add_i32 s12, s12, -1
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	s_add_u32 s0, s0, 8
	s_addc_u32 s1, s1, 0
	s_cmp_eq_u32 s12, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[10:11], v[10:11], -s[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[10:11], v[10:11], v[6:7]
	s_cbranch_scc0 .LBB12_5
	s_branch .LBB12_7
.LBB12_6:                               ;   in Loop: Header=BB12_3 Depth=1
	v_mov_b32_e32 v6, 0
	v_mov_b32_e32 v7, 0
.LBB12_7:                               ;   in Loop: Header=BB12_3 Depth=1
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_le_f64_e32 v[6:7], v[2:3]
	s_cbranch_execz .LBB12_2
; %bb.8:                                ;   in Loop: Header=BB12_3 Depth=1
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[6:7], 2, v[0:1]
	v_dual_mov_b32 v1, s7 :: v_dual_add_nc_u32 v0, 1, v0
	v_add_co_u32 v6, vcc_lo, s10, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s11, v7, vcc_lo
	global_store_b32 v[6:7], v1, off
	s_branch .LBB12_2
.LBB12_9:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel fixed_radius_fill_csr_kernel
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
.Lfunc_end12:
	.size	fixed_radius_fill_csr_kernel, .Lfunc_end12-fixed_radius_fill_csr_kernel
                                        ; -- End function
	.set fixed_radius_fill_csr_kernel.num_vgpr, 12
	.set fixed_radius_fill_csr_kernel.num_agpr, 0
	.set fixed_radius_fill_csr_kernel.numbered_sgpr, 16
	.set fixed_radius_fill_csr_kernel.num_named_barrier, 0
	.set fixed_radius_fill_csr_kernel.private_seg_size, 0
	.set fixed_radius_fill_csr_kernel.uses_vcc, 1
	.set fixed_radius_fill_csr_kernel.uses_flat_scratch, 0
	.set fixed_radius_fill_csr_kernel.has_dyn_sized_stack, 0
	.set fixed_radius_fill_csr_kernel.has_recursion, 0
	.set fixed_radius_fill_csr_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 448
; TotalNumSgprs: 18
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 18
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
	.type	__hip_cuid_f9d118558e76dc3a,@object ; @__hip_cuid_f9d118558e76dc3a
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_f9d118558e76dc3a
__hip_cuid_f9d118558e76dc3a:
	.byte	0                               ; 0x0
	.size	__hip_cuid_f9d118558e76dc3a, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_f9d118558e76dc3a
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
    .name:           fixed_radius_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     13
    .sgpr_spill_count: 0
    .symbol:         fixed_radius_kernel.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .offset:         16
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         20
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         28
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         30
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         32
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         34
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         36
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         38
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         56
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         80
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 272
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           uf_init_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         uf_init_kernel.kd
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
    .name:           uf_hook_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         uf_hook_kernel.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .offset:         16
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         20
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         24
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         28
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         30
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         32
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         34
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         36
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         38
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         56
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         64
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         72
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         80
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 272
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           uf_compress_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         uf_compress_kernel.kd
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
    .name:           boruvka_init_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         boruvka_init_kernel.kd
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
    .name:           boruvka_min_w_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         boruvka_min_w_kernel.kd
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
    .name:           boruvka_min_e_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         boruvka_min_e_kernel.kd
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
    .name:           boruvka_mark_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         boruvka_mark_kernel.kd
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
    .name:           core_distance_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     24
    .sgpr_spill_count: 0
    .symbol:         core_distance_kernel.kd
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
      - .offset:         20
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
    .name:           fixed_radius_count_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         fixed_radius_count_kernel.kd
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
    .name:           exclusive_scan_i32_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         exclusive_scan_i32_kernel.kd
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
    .name:           masked_weight_sum_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         masked_weight_sum_kernel.kd
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
      - .offset:         28
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
    .name:           fixed_radius_fill_csr_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         fixed_radius_fill_csr_kernel.kd
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
