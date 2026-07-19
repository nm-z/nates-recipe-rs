	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	convx_conv1d_kernel     ; -- Begin function convx_conv1d_kernel
	.globl	convx_conv1d_kernel
	.p2align	8
	.type	convx_conv1d_kernel,@function
convx_conv1d_kernel:                    ; @convx_conv1d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x4c
	s_load_b256 s[4:11], s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s7, s4
	s_mul_i32 s2, s2, s9
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_10
; %bb.1:
	s_abs_i32 s2, s9
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b256 s[12:19], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s3, v0
	s_abs_i32 s3, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s3
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s2, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s9, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s2, 0, s3
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[16:17], 0
	v_mul_lo_u32 v2, s2, v4
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_xor_b32_e32 v0, v0, v3
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v0, v3
	v_add_nc_u32_e32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v3, 0, v0
	v_max_i32_e32 v3, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v3, v2
	v_mul_lo_u32 v4, v2, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	v_subrev_nc_u32_e32 v5, s3, v3
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v5
	v_xor_b32_e32 v4, s7, v0
	v_add_nc_u32_e32 v5, 1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v2, v5, vcc_lo
	v_xor_b32_e32 v2, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v5, v2, v4
	v_mul_lo_u32 v2, v5, s7
	s_delay_alu instid0(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v0, v2
	s_cbranch_scc1 .LBB0_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[3:4], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s16, v3
	v_add_co_ci_u32_e64 v4, null, s17, v4, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc0 .LBB0_4
	s_branch .LBB0_9
.LBB0_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc1 .LBB0_9
.LBB0_4:
	v_mul_lo_u32 v0, v0, s9
	s_mul_i32 s0, s6, s5
	s_mul_i32 s1, s8, s5
	v_mul_lo_u32 v5, s0, v5
	s_cmp_gt_i32 s8, 0
	s_cselect_b32 s0, -1, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v1, v0
	v_mul_lo_u32 v6, v0, s10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v7, 31, v6
	v_lshlrev_b64 v[8:9], 3, v[6:7]
	v_mul_lo_u32 v7, s1, v2
	s_mov_b32 s1, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s12, v8
	v_add_co_ci_u32_e64 v2, null, s13, v9, vcc_lo
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB0_6
	.p2align	6
.LBB0_5:                                ;   in Loop: Header=BB0_6 Depth=1
	v_add_nc_u32_e32 v5, s6, v5
	v_add_nc_u32_e32 v7, s8, v7
	s_add_i32 s1, s1, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s1, s5
	s_cbranch_scc1 .LBB0_9
.LBB0_6:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB0_8 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s0
	s_cbranch_vccnz .LBB0_5
; %bb.7:                                ;   in Loop: Header=BB0_6 Depth=1
	v_ashrrev_i32_e32 v6, 31, v5
	v_ashrrev_i32_e32 v8, 31, v7
	s_mov_b32 s2, s8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[9:10], 3, v[5:6]
	v_lshlrev_b64 v[11:12], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v8, vcc_lo, v0, v9
	v_add_co_ci_u32_e64 v9, null, v2, v10, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v10, vcc_lo, s14, v11
	v_add_co_ci_u32_e64 v11, null, s15, v12, vcc_lo
	.p2align	6
.LBB0_8:                                ;   Parent Loop BB0_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[12:13], v[8:9], off
	global_load_b64 v[14:15], v[10:11], off
	v_add_co_u32 v8, vcc_lo, v8, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	v_add_co_u32 v10, vcc_lo, v10, 8
	v_add_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	s_add_i32 s2, s2, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s2, 0
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[12:13], v[14:15], v[3:4]
	s_cbranch_scc0 .LBB0_8
	s_branch .LBB0_5
.LBB0_9:
	s_set_inst_prefetch_distance 0x2
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s18, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB0_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_conv1d_kernel
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
		.amdhsa_next_free_vgpr 16
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
.Lfunc_end0:
	.size	convx_conv1d_kernel, .Lfunc_end0-convx_conv1d_kernel
                                        ; -- End function
	.set convx_conv1d_kernel.num_vgpr, 16
	.set convx_conv1d_kernel.num_agpr, 0
	.set convx_conv1d_kernel.numbered_sgpr, 20
	.set convx_conv1d_kernel.num_named_barrier, 0
	.set convx_conv1d_kernel.private_seg_size, 0
	.set convx_conv1d_kernel.uses_vcc, 1
	.set convx_conv1d_kernel.uses_flat_scratch, 0
	.set convx_conv1d_kernel.has_dyn_sized_stack, 0
	.set convx_conv1d_kernel.has_recursion, 0
	.set convx_conv1d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 852
; TotalNumSgprs: 22
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 22
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
	.protected	convx_conv1d_backward_data_kernel ; -- Begin function convx_conv1d_backward_data_kernel
	.globl	convx_conv1d_backward_data_kernel
	.p2align	8
	.type	convx_conv1d_backward_data_kernel,@function
convx_conv1d_backward_data_kernel:      ; @convx_conv1d_backward_data_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x44
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	s_mul_i32 s2, s2, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB1_13
; %bb.1:
	s_abs_i32 s4, s6
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s2, 0, s4
	s_cmp_lt_i32 s7, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s2, v0
	s_clause 0x2
	s_load_b128 s[8:11], s[0:1], 0x28
	s_load_b128 s[12:15], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_mov_b32 s1, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v3, v0
	v_mul_lo_u32 v2, v0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v3, s4, v2
	v_cmp_le_u32_e64 s0, s4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v2, v2, v3, s0
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	s_cbranch_scc1 .LBB1_11
; %bb.2:
	s_abs_i32 s16, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v2, s16
	v_rcp_iflag_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_dual_mul_f32 v2, 0x4f7ffffe, v2 :: v_dual_add_nc_u32 v3, 1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v0, v0, v3, s0
	v_xor_b32_e32 v3, s6, v1
	s_sub_i32 s0, 0, s16
	v_cvt_u32_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v4, 1, v0
	v_ashrrev_i32_e32 v3, 31, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	v_mul_lo_u32 v4, s0, v2
	s_ashr_i32 s0, s5, 31
	s_waitcnt lgkmcnt(0)
	s_cmp_gt_i32 s8, 0
	v_xor_b32_e32 v0, v0, v3
	s_cselect_b32 s4, -1, 0
	s_abs_i32 s11, s10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v0, v3
	v_mul_hi_u32 v3, v2, v4
	v_sub_nc_u32_e32 v4, 0, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v5, v2, v3
	v_max_i32_e32 v4, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[2:3], null, v4, v5, 0
	v_cvt_f32_u32_e32 v5, s11
	v_mul_lo_u32 v2, v3, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v4, v2
	v_add_nc_u32_e32 v4, 1, v3
	v_subrev_nc_u32_e32 v6, s16, v2
	v_cmp_le_u32_e32 vcc_lo, s16, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v3, v3, v4 :: v_dual_cndmask_b32 v2, v2, v6
	v_rcp_iflag_f32_e32 v4, v5
	v_ashrrev_i32_e32 v5, 31, v0
	v_add_nc_u32_e32 v6, 1, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s16, v2
	v_xor_b32_e32 v5, s0, v5
	s_sub_i32 s0, 0, s11
	s_waitcnt_depctr 0xfff
	v_dual_cndmask_b32 v2, v3, v6 :: v_dual_mul_f32 v3, 0x4f7ffffe, v4
	v_mul_lo_u32 v6, v0, s6
	s_ashr_i32 s6, s10, 31
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v3, v3
	v_mul_lo_u32 v4, s0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v4, v3, v4
	v_dual_mov_b32 v3, 0 :: v_dual_add_nc_u32 v10, v3, v4
	v_xor_b32_e32 v2, v2, v5
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v2, v5
	v_mul_lo_u32 v5, v2, s5
	v_mul_lo_u32 v2, v2, s7
	s_mul_i32 s5, s8, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v5, v0, v5
	v_sub_nc_u32_e32 v0, v1, v6
	v_mul_lo_u32 v5, s8, v5
	s_branch .LBB1_4
.LBB1_3:                                ;   in Loop: Header=BB1_4 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_nc_u32_e32 v5, s5, v5
	s_add_i32 s1, s1, 1
	s_cmp_eq_u32 s1, s7
	s_cbranch_scc1 .LBB1_12
.LBB1_4:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB1_8 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccnz .LBB1_3
; %bb.5:                                ;   in Loop: Header=BB1_4 Depth=1
	v_dual_mov_b32 v13, v0 :: v_dual_add_nc_u32 v6, s1, v2
	s_mov_b32 s16, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v7, v6, s9
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[11:12], 3, v[5:6]
	v_ashrrev_i32_e32 v8, 31, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_u32 v6, vcc_lo, s14, v11
	v_lshlrev_b64 v[8:9], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v7, null, s15, v12, vcc_lo
	v_add_co_u32 v11, vcc_lo, s12, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v12, null, s13, v9, vcc_lo
	s_branch .LBB1_8
.LBB1_6:                                ;   in Loop: Header=BB1_8 Depth=2
	s_or_b32 exec_lo, exec_lo, s0
.LBB1_7:                                ;   in Loop: Header=BB1_8 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s17
	v_add_co_u32 v6, vcc_lo, v6, 8
	v_add_nc_u32_e32 v13, -1, v13
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	s_add_i32 s16, s16, -1
	s_cmp_eq_u32 s16, 0
	s_cbranch_scc1 .LBB1_3
.LBB1_8:                                ;   Parent Loop BB1_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	s_mov_b32 s17, exec_lo
	v_cmpx_lt_i32_e32 -1, v13
	s_cbranch_execz .LBB1_7
; %bb.9:                                ;   in Loop: Header=BB1_8 Depth=2
	v_sub_nc_u32_e32 v8, 0, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v8, v13, v8
	v_mul_hi_u32 v9, v8, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v14, v9, s11
	v_sub_nc_u32_e32 v8, v8, v14
	v_add_nc_u32_e32 v14, 1, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v15, s11, v8
	v_cmp_le_u32_e32 vcc_lo, s11, v8
	v_dual_cndmask_b32 v9, v9, v14 :: v_dual_cndmask_b32 v8, v8, v15
	v_ashrrev_i32_e32 v14, 31, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v15, 1, v9
	v_cmp_le_u32_e32 vcc_lo, s11, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_xor_b32_e32 v14, s6, v14
	v_cndmask_b32_e32 v8, v9, v15, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v8, v8, v14
	v_sub_nc_u32_e32 v8, v8, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v9, v8, s10
	v_cmp_gt_i32_e64 s0, s9, v8
	v_sub_nc_u32_e32 v9, v13, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v9
	s_and_b32 s18, vcc_lo, s0
	s_and_saveexec_b32 s0, s18
	s_cbranch_execz .LBB1_6
; %bb.10:                               ;   in Loop: Header=BB1_8 Depth=2
	v_ashrrev_i32_e32 v9, 31, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[8:9], 3, v[8:9]
	v_add_co_u32 v8, vcc_lo, v11, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, v12, v9, vcc_lo
	global_load_b64 v[14:15], v[6:7], off
	global_load_b64 v[8:9], v[8:9], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[14:15], v[8:9], v[3:4]
	s_branch .LBB1_6
.LBB1_11:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB1_12:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB1_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_conv1d_backward_data_kernel
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
.Lfunc_end1:
	.size	convx_conv1d_backward_data_kernel, .Lfunc_end1-convx_conv1d_backward_data_kernel
                                        ; -- End function
	.set convx_conv1d_backward_data_kernel.num_vgpr, 16
	.set convx_conv1d_backward_data_kernel.num_agpr, 0
	.set convx_conv1d_backward_data_kernel.numbered_sgpr, 19
	.set convx_conv1d_backward_data_kernel.num_named_barrier, 0
	.set convx_conv1d_backward_data_kernel.private_seg_size, 0
	.set convx_conv1d_backward_data_kernel.uses_vcc, 1
	.set convx_conv1d_backward_data_kernel.uses_flat_scratch, 0
	.set convx_conv1d_backward_data_kernel.has_dyn_sized_stack, 0
	.set convx_conv1d_backward_data_kernel.has_recursion, 0
	.set convx_conv1d_backward_data_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1040
; TotalNumSgprs: 21
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 1
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
	.protected	convx_conv1d_backward_filter_kernel ; -- Begin function convx_conv1d_backward_filter_kernel
	.globl	convx_conv1d_backward_filter_kernel
	.p2align	8
	.type	convx_conv1d_backward_filter_kernel,@function
convx_conv1d_backward_filter_kernel:    ; @convx_conv1d_backward_filter_kernel
; %bb.0:
	s_load_b256 s[4:11], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	v_cvt_f32_u32_e32 v1, s11
	s_sub_i32 s12, 0, s11
	s_mul_i32 s22, s8, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_mul_i32 s16, s22, s7
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s3, v1
	s_mul_i32 s12, s12, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s12, s3, s12
	s_add_i32 s3, s3, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s3, s2, s3
	s_mul_i32 s12, s3, s11
	s_add_i32 s13, s3, 1
	s_sub_i32 s12, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s14, s12, s11
	s_cmp_ge_u32 s12, s11
	s_cselect_b32 s3, s13, s3
	s_cselect_b32 s12, s14, s12
	s_add_i32 s13, s3, 1
	s_cmp_ge_u32 s12, s11
	s_cselect_b32 s17, s13, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_ge_i32 s17, s16
	s_cbranch_scc1 .LBB2_23
; %bb.1:
	s_abs_i32 s3, s22
	s_abs_i32 s21, s17
	v_cvt_f32_u32_e32 v1, s3
	s_sub_i32 s13, 0, s3
	s_mul_i32 s24, s9, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s12, v1
	s_mul_i32 s13, s13, s12
	s_mul_hi_u32 s13, s12, s13
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_i32 s12, s12, s13
	s_mul_i32 s13, s17, s11
	s_mul_hi_u32 s12, s21, s12
	s_sub_i32 s18, s2, s13
	s_mul_i32 s14, s12, s3
	s_add_i32 s13, s12, 1
	s_sub_i32 s2, s21, s14
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s14, s2, s3
	s_cmp_ge_u32 s2, s3
	s_cselect_b32 s12, s13, s12
	s_cselect_b32 s2, s14, s2
	s_add_i32 s13, s12, 1
	s_cmp_ge_u32 s2, s3
	s_cselect_b32 s20, s13, s12
	s_abs_i32 s19, s11
	s_add_i32 s3, s24, s11
	v_cvt_f32_u32_e32 v1, s19
	s_sub_i32 s4, 0, s19
	s_abs_i32 s23, s8
	s_add_i32 s3, s3, -1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_abs_i32 s12, s3
	s_xor_b32 s3, s3, s11
	s_ashr_i32 s11, s3, 31
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s2, v1
	v_cvt_f32_u32_e32 v1, s23
	s_mul_i32 s4, s4, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_mul_hi_u32 s4, s2, s4
	s_add_i32 s2, s2, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s4, s12, s2
	s_mul_i32 s2, s4, s19
	s_add_i32 s26, s4, 1
	s_sub_i32 s25, s12, s2
	s_clause 0x1
	s_load_b128 s[12:15], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_sub_i32 s27, s25, s19
	s_cmp_ge_u32 s25, s19
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_cselect_b32 s4, s26, s4
	s_cselect_b32 s25, s27, s25
	s_add_i32 s26, s4, 1
	s_cmp_ge_u32 s25, s19
	v_cvt_u32_f32_e32 v2, v1
	s_cselect_b32 s4, s26, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b32 s4, s4, s11
	s_sub_i32 s4, s4, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s11, s4, s18
	v_add_nc_u32_e32 v1, s11, v0
	s_add_i32 s11, s11, s4
	v_readfirstlane_b32 s4, v2
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_min_i32 s19, s11, s24
	s_mov_b32 s11, exec_lo
	v_cmpx_gt_i32_e64 s19, v1
	s_cbranch_execz .LBB2_5
; %bb.2:
	s_ashr_i32 s24, s17, 31
	s_ashr_i32 s25, s22, 31
	s_sub_i32 s26, 0, s23
	s_xor_b32 s25, s24, s25
	s_mul_i32 s26, s26, s4
	s_xor_b32 s20, s20, s25
	s_ashr_i32 s8, s8, 31
	s_sub_i32 s20, s20, s25
	s_mul_hi_u32 s25, s4, s26
	s_mul_i32 s22, s20, s22
	s_add_i32 s4, s4, s25
	s_sub_i32 s22, s17, s22
	s_load_b32 s1, s[0:1], 0x44
	s_abs_i32 s25, s22
	s_ashr_i32 s22, s22, 31
	s_mul_hi_u32 s26, s25, s4
	s_xor_b32 s8, s22, s8
	s_mul_i32 s27, s26, s23
	s_mul_hi_u32 s4, s21, s4
	s_sub_i32 s22, s25, s27
	s_add_i32 s25, s26, 1
	s_sub_i32 s27, s22, s23
	s_cmp_ge_u32 s22, s23
	s_mul_i32 s4, s4, s23
	s_cselect_b32 s25, s25, s26
	s_cselect_b32 s22, s27, s22
	s_add_i32 s26, s25, 1
	s_cmp_ge_u32 s22, s23
	s_cselect_b32 s22, s26, s25
	s_sub_i32 s21, s21, s4
	s_xor_b32 s4, s22, s8
	s_sub_i32 s22, s21, s23
	s_sub_i32 s4, s4, s8
	s_cmp_ge_u32 s21, s23
	s_cselect_b32 s8, s22, s21
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s21, s8, s23
	s_cmp_ge_u32 s8, s23
	s_cselect_b32 s21, s21, s8
	s_abs_i32 s8, s9
	s_waitcnt lgkmcnt(0)
	s_and_b32 s1, s1, 0xffff
	v_cvt_f32_u32_e32 v2, s8
	s_sub_i32 s0, 0, s8
	s_ashr_i32 s22, s9, 31
	s_sub_i32 s23, 0, s9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	v_cvt_u32_f32_e32 v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, s0, v4
	s_xor_b32 s0, s21, s24
	s_mov_b32 s21, 0
	s_sub_i32 s0, s0, s24
	v_mul_hi_u32 v5, v4, v2
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_2)
	v_dual_mov_b32 v3, 0 :: v_dual_add_nc_u32 v4, v4, v5
.LBB2_3:                                ; =>This Inner Loop Header: Depth=1
	v_sub_nc_u32_e32 v5, 0, v1
	v_ashrrev_i32_e32 v6, 31, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v5, v1, v5
	v_xor_b32_e32 v8, s22, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v6, v5, v4
	v_mul_lo_u32 v7, v6, s8
	v_add_nc_u32_e32 v9, 1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v5, v5, v7
	v_subrev_nc_u32_e32 v7, s8, v5
	v_cmp_le_u32_e32 vcc_lo, s8, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v6, v6, v9 :: v_dual_cndmask_b32 v5, v5, v7
	v_add_nc_u32_e32 v7, 1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s8, v5
	v_cndmask_b32_e32 v5, v6, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v9, v5, v8
	v_sub_nc_u32_e32 v10, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v11, v10, s7
	v_mad_u64_u32 v[5:6], null, v10, s5, s[4:5]
	v_mad_u64_u32 v[6:7], null, s23, v10, v[1:2]
	v_add3_u32 v7, s20, v8, v11
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v8, v5, s6
	v_mul_lo_u32 v10, v6, s10
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v7, v7, v9
	v_mad_u64_u32 v[5:6], null, s9, v7, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add3_u32 v7, v10, s0, v8
	v_add_nc_u32_e32 v1, s1, v1
	v_ashrrev_i32_e32 v8, 31, v7
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v6, 31, v5
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	v_add_co_u32 v7, vcc_lo, s14, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v8, null, s15, v8, vcc_lo
	v_add_co_u32 v5, vcc_lo, s12, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s13, v6, vcc_lo
	global_load_b64 v[7:8], v[7:8], off
	global_load_b64 v[5:6], v[5:6], off
	v_cmp_le_i32_e32 vcc_lo, s19, v1
	s_or_b32 s21, vcc_lo, s21
	s_waitcnt vmcnt(0)
	v_fma_f64 v[2:3], v[5:6], v[7:8], v[2:3]
	s_and_not1_b32 exec_lo, exec_lo, s21
	s_cbranch_execnz .LBB2_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s21
.LBB2_5:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s11
	v_lshlrev_b32_e32 v1, 3, v0
	s_mov_b32 s0, exec_lo
	ds_store_b64 v1, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 0x80, v0
	s_cbranch_execz .LBB2_7
; %bb.6:
	ds_load_2addr_stride64_b64 v[2:5], v1 offset1:2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB2_7:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 64, v0
	s_cbranch_execz .LBB2_9
; %bb.8:
	ds_load_2addr_stride64_b64 v[2:5], v1 offset1:1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB2_9:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB2_11
; %bb.10:
	ds_load_2addr_b64 v[2:5], v1 offset1:32
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB2_11:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 16, v0
	s_cbranch_execz .LBB2_13
; %bb.12:
	ds_load_2addr_b64 v[2:5], v1 offset1:16
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB2_13:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 8, v0
	s_cbranch_execz .LBB2_15
; %bb.14:
	ds_load_2addr_b64 v[2:5], v1 offset1:8
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB2_15:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 4, v0
	s_cbranch_execz .LBB2_17
; %bb.16:
	ds_load_2addr_b64 v[2:5], v1 offset1:4
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB2_17:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 2, v0
	s_cbranch_execz .LBB2_19
; %bb.18:
	ds_load_2addr_b64 v[2:5], v1 offset1:2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB2_19:
	s_or_b32 exec_lo, exec_lo, s0
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, vcc_lo
	s_cbranch_execz .LBB2_21
; %bb.20:
	ds_load_2addr_b64 v[2:5], v1 offset1:1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB2_21:
	s_or_b32 exec_lo, exec_lo, s0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, vcc_lo
	s_cbranch_execz .LBB2_23
; %bb.22:
	v_mov_b32_e32 v2, 0
	s_mul_i32 s0, s18, s16
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s0, s0, s17
	s_ashr_i32 s1, s0, 31
	ds_load_b64 v[0:1], v2
	s_lshl_b64 s[0:1], s[0:1], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s0, s2, s0
	s_addc_u32 s1, s3, s1
	s_waitcnt lgkmcnt(0)
	global_store_b64 v2, v[0:1], s[0:1]
.LBB2_23:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_conv1d_backward_filter_kernel
		.amdhsa_group_segment_fixed_size 2048
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
		.amdhsa_next_free_vgpr 12
		.amdhsa_next_free_sgpr 28
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
.Lfunc_end2:
	.size	convx_conv1d_backward_filter_kernel, .Lfunc_end2-convx_conv1d_backward_filter_kernel
                                        ; -- End function
	.set convx_conv1d_backward_filter_kernel.num_vgpr, 12
	.set convx_conv1d_backward_filter_kernel.num_agpr, 0
	.set convx_conv1d_backward_filter_kernel.numbered_sgpr, 28
	.set convx_conv1d_backward_filter_kernel.num_named_barrier, 0
	.set convx_conv1d_backward_filter_kernel.private_seg_size, 0
	.set convx_conv1d_backward_filter_kernel.uses_vcc, 1
	.set convx_conv1d_backward_filter_kernel.uses_flat_scratch, 0
	.set convx_conv1d_backward_filter_kernel.has_dyn_sized_stack, 0
	.set convx_conv1d_backward_filter_kernel.has_recursion, 0
	.set convx_conv1d_backward_filter_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1684
; TotalNumSgprs: 30
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 2048 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 30
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
	.protected	convx_conv1d_backward_bias_kernel ; -- Begin function convx_conv1d_backward_bias_kernel
	.globl	convx_conv1d_backward_bias_kernel
	.p2align	8
	.type	convx_conv1d_backward_bias_kernel,@function
convx_conv1d_backward_bias_kernel:      ; @convx_conv1d_backward_bias_kernel
; %bb.0:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_waitcnt lgkmcnt(0)
	v_cvt_f32_u32_e32 v1, s9
	s_sub_i32 s3, 0, s9
	s_mul_i32 s8, s10, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s13, v1
	s_mul_i32 s3, s3, s13
	s_mul_hi_u32 s3, s13, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s13, s13, s3
	s_mul_hi_u32 s3, s2, s13
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	s_mul_i32 s11, s3, s9
	s_add_i32 s12, s3, 1
	s_sub_i32 s11, s2, s11
	s_sub_i32 s14, s11, s9
	s_cmp_ge_u32 s11, s9
	s_cselect_b32 s3, s12, s3
	s_cselect_b32 s11, s14, s11
	s_add_i32 s12, s3, 1
	s_cmp_ge_u32 s11, s9
	s_mov_b32 s11, exec_lo
	s_cselect_b32 s3, s12, s3
	s_mov_b32 s12, 0
	v_lshl_add_u32 v1, s3, 8, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_gt_i32_e64 s8, v1
	s_cbranch_execz .LBB3_4
; %bb.1:
	s_load_b32 s0, s[0:1], 0x20
	s_waitcnt lgkmcnt(0)
	s_mul_hi_u32 s1, s0, s13
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s13, s1, s9
	s_sub_i32 s0, s0, s13
	s_add_i32 s13, s1, 1
	s_sub_i32 s14, s0, s9
	s_cmp_ge_u32 s0, s9
	s_cselect_b32 s1, s13, s1
	s_cselect_b32 s0, s14, s0
	s_add_i32 s13, s1, 1
	s_cmp_ge_u32 s0, s9
	s_cselect_b32 s1, s13, s1
	s_abs_i32 s0, s10
	s_lshl_b32 s1, s1, 8
	v_cvt_f32_u32_e32 v2, s0
	s_sub_i32 s13, 0, s0
	s_sub_i32 s14, 0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v2, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	v_cvt_u32_f32_e32 v4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, s13, v4
	s_ashr_i32 s13, s10, 31
	v_mul_hi_u32 v5, v4, v2
	v_mov_b32_e32 v2, 0
	s_delay_alu instid0(VALU_DEP_2)
	v_dual_mov_b32 v3, 0 :: v_dual_add_nc_u32 v4, v4, v5
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB3_2:                                ; =>This Inner Loop Header: Depth=1
	v_sub_nc_u32_e32 v5, 0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v5, v1, v5
	v_mul_hi_u32 v6, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v7, v6, s0
	v_sub_nc_u32_e32 v5, v5, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v7, s0, v5
	v_cmp_le_u32_e32 vcc_lo, s0, v5
	v_dual_cndmask_b32 v5, v5, v7 :: v_dual_add_nc_u32 v8, 1, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v6, v6, v8, vcc_lo
	v_ashrrev_i32_e32 v8, 31, v1
	v_cmp_le_u32_e32 vcc_lo, s0, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v7, 1, v6
	v_xor_b32_e32 v8, s13, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, v6, v7, vcc_lo
	v_xor_b32_e32 v5, v5, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v6, s14, v5
	v_sub_nc_u32_e32 v6, v6, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v6, s9, v6
	v_add3_u32 v6, s2, v8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v7, v6, v5
	v_mad_u64_u32 v[5:6], null, s10, v7, v[1:2]
	v_add_nc_u32_e32 v1, s1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v6, 31, v5
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v5, vcc_lo, s4, v5
	v_add_co_ci_u32_e64 v6, null, s5, v6, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s8, v1
	global_load_b64 v[5:6], v[5:6], off
	s_or_b32 s12, vcc_lo, s12
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[5:6]
	s_and_not1_b32 exec_lo, exec_lo, s12
	s_cbranch_execnz .LBB3_2
; %bb.3:
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s12
.LBB3_4:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s11
	v_lshlrev_b32_e32 v1, 3, v0
	s_mov_b32 s0, exec_lo
	ds_store_b64 v1, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 0x80, v0
	s_cbranch_execz .LBB3_6
; %bb.5:
	ds_load_2addr_stride64_b64 v[2:5], v1 offset1:2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB3_6:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 64, v0
	s_cbranch_execz .LBB3_8
; %bb.7:
	ds_load_2addr_stride64_b64 v[2:5], v1 offset1:1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB3_8:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB3_10
; %bb.9:
	ds_load_2addr_b64 v[2:5], v1 offset1:32
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB3_10:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 16, v0
	s_cbranch_execz .LBB3_12
; %bb.11:
	ds_load_2addr_b64 v[2:5], v1 offset1:16
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB3_12:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 8, v0
	s_cbranch_execz .LBB3_14
; %bb.13:
	ds_load_2addr_b64 v[2:5], v1 offset1:8
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB3_14:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 4, v0
	s_cbranch_execz .LBB3_16
; %bb.15:
	ds_load_2addr_b64 v[2:5], v1 offset1:4
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB3_16:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 2, v0
	s_cbranch_execz .LBB3_18
; %bb.17:
	ds_load_2addr_b64 v[2:5], v1 offset1:2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB3_18:
	s_or_b32 exec_lo, exec_lo, s0
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, vcc_lo
	s_cbranch_execz .LBB3_20
; %bb.19:
	ds_load_2addr_b64 v[2:5], v1 offset1:1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	ds_store_b64 v1, v[2:3]
.LBB3_20:
	s_or_b32 exec_lo, exec_lo, s0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s0, vcc_lo
	s_cbranch_execz .LBB3_24
; %bb.21:
	s_mov_b32 s5, exec_lo
	s_mov_b32 s4, 0
	v_mbcnt_lo_u32_b32 v0, s5, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_and_b32 s0, exec_lo, vcc_lo
	s_mov_b32 exec_lo, s0
	s_cbranch_execz .LBB3_24
; %bb.22:
	s_mul_i32 s3, s3, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_sub_i32 s0, s2, s3
	s_ashr_i32 s1, s0, 31
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_lshl_b64 s[0:1], s[0:1], 3
	s_add_u32 s0, s6, s0
	s_addc_u32 s1, s7, s1
	s_bcnt1_i32_b32 s2, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cvt_f64_u32_e32 v[0:1], s2
	s_load_b64 s[2:3], s[0:1], 0x0
	v_mov_b32_e32 v6, 0
	ds_load_b64 v[2:3], v6
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[4:5], v[2:3], v[0:1]
	v_dual_mov_b32 v2, s2 :: v_dual_mov_b32 v3, s3
.LBB3_23:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[0:1], v[2:3], v[4:5]
	global_atomic_cmpswap_b64 v[0:1], v6, v[0:3], s[0:1] glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s4, vcc_lo, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s4
	s_cbranch_execnz .LBB3_23
.LBB3_24:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_conv1d_backward_bias_kernel
		.amdhsa_group_segment_fixed_size 2048
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
		.amdhsa_next_free_vgpr 9
		.amdhsa_next_free_sgpr 15
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
.Lfunc_end3:
	.size	convx_conv1d_backward_bias_kernel, .Lfunc_end3-convx_conv1d_backward_bias_kernel
                                        ; -- End function
	.set convx_conv1d_backward_bias_kernel.num_vgpr, 9
	.set convx_conv1d_backward_bias_kernel.num_agpr, 0
	.set convx_conv1d_backward_bias_kernel.numbered_sgpr, 15
	.set convx_conv1d_backward_bias_kernel.num_named_barrier, 0
	.set convx_conv1d_backward_bias_kernel.private_seg_size, 0
	.set convx_conv1d_backward_bias_kernel.uses_vcc, 1
	.set convx_conv1d_backward_bias_kernel.uses_flat_scratch, 0
	.set convx_conv1d_backward_bias_kernel.has_dyn_sized_stack, 0
	.set convx_conv1d_backward_bias_kernel.has_recursion, 0
	.set convx_conv1d_backward_bias_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1264
; TotalNumSgprs: 17
; NumVgprs: 9
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 2048 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 17
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
	.protected	convx_conv2d_kernel     ; -- Begin function convx_conv2d_kernel
	.globl	convx_conv2d_kernel
	.p2align	8
	.type	convx_conv2d_kernel,@function
convx_conv2d_kernel:                    ; @convx_conv2d_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s12, s[0:1], 0x54
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b32 s3, s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s12, s12, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s12, v[0:1]
	s_mul_i32 s2, s8, s4
	s_mul_i32 s2, s2, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s3
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB4_13
; %bb.1:
	s_abs_i32 s2, s3
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s4, 0, s2
	s_load_b256 s[12:19], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s4, v0
	s_abs_i32 s4, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s4
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s2, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s3, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s2, 0, s4
	v_mul_lo_u32 v2, s2, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_abs_i32 s2, s8
	v_cvt_f32_u32_e32 v5, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v0, v0, v3
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rcp_iflag_f32_e32 v5, v5
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v2, v4, v2
	v_sub_nc_u32_e32 v3, 0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v3, v0, v3
	v_mul_hi_u32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, v2, s4
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v6, s4, v3
	v_cmp_le_u32_e32 vcc_lo, s4, v3
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v6
	v_mul_f32_e32 v4, 0x4f7ffffe, v5
	v_xor_b32_e32 v5, s11, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_nc_u32_e32 v6, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_u32_f32_e32 v4, v4
	v_ashrrev_i32_e32 v5, 31, v5
	s_sub_i32 s4, 0, s2
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[16:17], 0
	v_cndmask_b32_e32 v2, v2, v6, vcc_lo
	v_mul_lo_u32 v3, s4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v2, v2, v5
	v_sub_nc_u32_e32 v5, v2, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v2, v4, v3
	v_sub_nc_u32_e32 v3, 0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v2, v4, v2
	v_max_i32_e32 v3, v5, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v3, v2
	v_mul_lo_u32 v4, v2, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	v_subrev_nc_u32_e32 v6, s2, v3
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v6
	v_xor_b32_e32 v4, s8, v5
	v_add_nc_u32_e32 v6, 1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v2, v6, vcc_lo
	v_xor_b32_e32 v2, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v6, v2, v4
	v_mul_lo_u32 v2, v6, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v5, v2
	s_cbranch_scc1 .LBB4_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[3:4], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s16, v3
	v_add_co_ci_u32_e64 v4, null, s17, v4, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc0 .LBB4_4
	s_branch .LBB4_12
.LBB4_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc1 .LBB4_12
.LBB4_4:
	v_mul_lo_u32 v5, v5, s11
	v_mul_lo_u32 v8, v0, s3
	s_mul_i32 s2, s10, s9
	s_cmp_gt_i32 s9, 0
	s_mul_i32 s0, s2, s5
	s_cselect_b32 s3, -1, 0
	s_cmp_gt_i32 s10, 0
	s_mul_i32 s6, s7, s6
	v_sub_nc_u32_e32 v5, v0, v5
	v_mul_lo_u32 v0, v6, s5
	s_cselect_b32 s4, -1, 0
	s_mov_b32 s1, 0
	s_mov_b32 s8, 0
	v_mad_u64_u32 v[6:7], null, s7, v5, v[1:2]
	v_mul_lo_u32 v5, s0, v2
	s_delay_alu instid0(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v6, v8
	s_branch .LBB4_6
.LBB4_5:                                ;   in Loop: Header=BB4_6 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_nc_u32_e32 v5, s2, v5
	s_add_i32 s8, s8, 1
	s_cmp_eq_u32 s8, s5
	s_cbranch_scc1 .LBB4_12
.LBB4_6:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB4_9 Depth 2
                                        ;       Child Loop BB4_11 Depth 3
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccnz .LBB4_5
; %bb.7:                                ;   in Loop: Header=BB4_6 Depth=1
	v_add_nc_u32_e32 v6, s8, v0
	v_mov_b32_e32 v14, v2
	s_mov_b32 s0, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	s_mov_b32 s11, s0
	v_mul_lo_u32 v7, s6, v6
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[10:11], 3, v[5:6]
	v_ashrrev_i32_e32 v8, 31, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_u32 v10, vcc_lo, s14, v10
	v_lshlrev_b64 v[6:7], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v11, null, s15, v11, vcc_lo
	v_add_co_u32 v12, vcc_lo, s12, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v13, null, s13, v7, vcc_lo
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB4_9
	.p2align	6
.LBB4_8:                                ;   in Loop: Header=BB4_9 Depth=2
	v_add_nc_u32_e32 v14, s7, v14
	s_add_i32 s11, s11, 1
	s_add_i32 s0, s0, s10
	s_cmp_eq_u32 s11, s9
	s_cbranch_scc1 .LBB4_5
.LBB4_9:                                ;   Parent Loop BB4_6 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB4_11 Depth 3
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccnz .LBB4_8
; %bb.10:                               ;   in Loop: Header=BB4_9 Depth=2
	s_lshl_b64 s[16:17], s[0:1], 3
	v_mov_b32_e32 v8, v14
	v_add_co_u32 v6, vcc_lo, v10, s16
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s17, v11, vcc_lo
	s_mov_b32 s16, s10
	.p2align	6
.LBB4_11:                               ;   Parent Loop BB4_6 Depth=1
                                        ;     Parent Loop BB4_9 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v9, 31, v8
	s_add_i32 s16, s16, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s16, 0
	v_lshlrev_b64 v[15:16], 3, v[8:9]
	v_add_nc_u32_e32 v8, 1, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v15, vcc_lo, v12, v15
	v_add_co_ci_u32_e64 v16, null, v13, v16, vcc_lo
	global_load_b64 v[17:18], v[6:7], off
	global_load_b64 v[15:16], v[15:16], off
	v_add_co_u32 v6, vcc_lo, v6, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[15:16], v[17:18], v[3:4]
	s_cbranch_scc0 .LBB4_11
	s_branch .LBB4_8
.LBB4_12:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s18, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB4_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_conv2d_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 328
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
.Lfunc_end4:
	.size	convx_conv2d_kernel, .Lfunc_end4-convx_conv2d_kernel
                                        ; -- End function
	.set convx_conv2d_kernel.num_vgpr, 19
	.set convx_conv2d_kernel.num_agpr, 0
	.set convx_conv2d_kernel.numbered_sgpr, 20
	.set convx_conv2d_kernel.num_named_barrier, 0
	.set convx_conv2d_kernel.private_seg_size, 0
	.set convx_conv2d_kernel.uses_vcc, 1
	.set convx_conv2d_kernel.uses_flat_scratch, 0
	.set convx_conv2d_kernel.has_dyn_sized_stack, 0
	.set convx_conv2d_kernel.has_recursion, 0
	.set convx_conv2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1124
; TotalNumSgprs: 22
; NumVgprs: 19
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 22
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
	.protected	convx_conv3d_kernel     ; -- Begin function convx_conv3d_kernel
	.globl	convx_conv3d_kernel
	.p2align	8
	.type	convx_conv3d_kernel,@function
convx_conv3d_kernel:                    ; @convx_conv3d_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x5c
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b128 s[20:23], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s9, s4
	s_mul_i32 s2, s2, s21
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s22
	s_mul_i32 s2, s2, s23
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB5_16
; %bb.1:
	s_abs_i32 s2, s23
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b256 s[12:19], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s3, v0
	s_abs_i32 s3, s22
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s3
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v0, s2
	v_sub_nc_u32_e32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s2, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s23, v1
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v6, 1, v0
	v_ashrrev_i32_e32 v5, 31, v3
	v_cvt_u32_f32_e32 v3, v4
	s_sub_i32 s2, 0, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, v0, v6, vcc_lo
	v_mul_lo_u32 v2, s2, v3
	s_abs_i32 s2, s21
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v5
	v_cvt_f32_u32_e32 v7, s2
	v_sub_nc_u32_e32 v6, v0, v5
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_hi_u32 v2, v3, v2
	v_rcp_iflag_f32_e32 v7, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v4, 0, v6
	v_add_nc_u32_e32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v3, v6, v4
	v_mul_hi_u32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, v2, s3
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_subrev_nc_u32_e32 v8, s3, v3
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v3, v3, v8 :: v_dual_mul_f32 v4, 0x4f7ffffe, v7
	v_xor_b32_e32 v7, s22, v6
	v_add_nc_u32_e32 v8, 1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	v_cvt_u32_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v7, 31, v7
	s_sub_i32 s3, 0, s2
	v_cndmask_b32_e32 v2, v2, v8, vcc_lo
	v_mul_lo_u32 v3, s3, v4
	s_abs_i32 s3, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_f32_u32_e32 v8, s3
	v_xor_b32_e32 v2, v2, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v8, v8
	v_sub_nc_u32_e32 v7, v2, v7
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v2, v4, v3
	v_sub_nc_u32_e32 v3, 0, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v2, v4, v2
	v_max_i32_e32 v3, v7, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v3, v2
	v_mul_lo_u32 v4, v2, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	v_subrev_nc_u32_e32 v9, s2, v3
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v9
	v_mul_f32_e32 v4, 0x4f7ffffe, v8
	v_xor_b32_e32 v8, s21, v7
	v_add_nc_u32_e32 v9, 1, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_cvt_u32_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_4)
	v_ashrrev_i32_e32 v8, 31, v8
	s_sub_i32 s2, 0, s3
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[16:17], 0
	v_cndmask_b32_e32 v2, v2, v9, vcc_lo
	v_mul_lo_u32 v3, s2, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v2, v2, v8
	v_sub_nc_u32_e32 v9, v2, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v2, v4, v3
	v_sub_nc_u32_e32 v3, 0, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v2, v4, v2
	v_max_i32_e32 v3, v9, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v3, v2
	v_mul_lo_u32 v4, v2, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	v_subrev_nc_u32_e32 v8, s3, v3
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_cndmask_b32_e32 v3, v3, v8, vcc_lo
	v_xor_b32_e32 v4, s9, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v8, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v4, 31, v4
	v_cndmask_b32_e32 v2, v2, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v2, v2, v4
	v_sub_nc_u32_e32 v8, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v2, v8, s9
	v_sub_nc_u32_e32 v2, v9, v2
	s_cbranch_scc1 .LBB5_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[3:4], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s16, v3
	v_add_co_ci_u32_e64 v4, null, s17, v4, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc0 .LBB5_4
	s_branch .LBB5_15
.LBB5_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc1 .LBB5_15
.LBB5_4:
	v_mul_lo_u32 v9, v9, s21
	s_mul_i32 s3, s20, s11
	s_mul_i32 s2, s7, s8
	s_cmp_gt_i32 s10, 0
	s_mul_i32 s6, s2, s6
	s_cselect_b32 s4, -1, 0
	s_cmp_gt_i32 s11, 0
	s_mov_b32 s1, 0
	v_sub_nc_u32_e32 v11, v7, v9
	v_mul_lo_u32 v7, v7, s22
	s_cselect_b32 s9, -1, 0
	s_cmp_gt_i32 s20, 0
	s_mov_b32 s17, 0
	v_mad_u64_u32 v[9:10], null, s7, v11, v[0:1]
	s_mul_i32 s7, s3, s10
	v_mul_lo_u32 v0, v8, s5
	s_mul_i32 s0, s7, s5
	s_cselect_b32 s16, -1, 0
	v_sub_nc_u32_e32 v7, v9, v7
	v_mul_lo_u32 v9, v6, s23
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v5, v7, v5
	v_mad_u64_u32 v[6:7], null, s8, v5, v[1:2]
	v_mul_lo_u32 v5, s0, v2
	s_delay_alu instid0(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v6, v9
	s_branch .LBB5_6
.LBB5_5:                                ;   in Loop: Header=BB5_6 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_nc_u32_e32 v5, s7, v5
	s_add_i32 s17, s17, 1
	s_cmp_eq_u32 s17, s5
	s_cbranch_scc1 .LBB5_15
.LBB5_6:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB5_9 Depth 2
                                        ;       Child Loop BB5_12 Depth 3
                                        ;         Child Loop BB5_14 Depth 4
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccnz .LBB5_5
; %bb.7:                                ;   in Loop: Header=BB5_6 Depth=1
	v_add_nc_u32_e32 v6, s17, v0
	v_mov_b32_e32 v14, v2
	s_mov_b32 s21, 0
	s_mov_b32 s22, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v7, s6, v6
	v_ashrrev_i32_e32 v6, 31, v5
	v_lshlrev_b64 v[10:11], 3, v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v8, 31, v7
	v_add_co_u32 v10, vcc_lo, s14, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[6:7], 3, v[7:8]
	v_add_co_ci_u32_e64 v11, null, s15, v11, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v12, vcc_lo, s12, v6
	v_add_co_ci_u32_e64 v13, null, s13, v7, vcc_lo
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB5_9
	.p2align	6
.LBB5_8:                                ;   in Loop: Header=BB5_9 Depth=2
	v_add_nc_u32_e32 v14, s2, v14
	s_add_i32 s22, s22, 1
	s_add_i32 s21, s21, s3
	s_cmp_eq_u32 s22, s10
	s_cbranch_scc1 .LBB5_5
.LBB5_9:                                ;   Parent Loop BB5_6 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB5_12 Depth 3
                                        ;         Child Loop BB5_14 Depth 4
	s_and_not1_b32 vcc_lo, exec_lo, s9
	s_cbranch_vccnz .LBB5_8
; %bb.10:                               ;   in Loop: Header=BB5_9 Depth=2
	v_mov_b32_e32 v15, v14
	s_mov_b32 s23, 0
	s_mov_b32 s0, s21
	s_branch .LBB5_12
	.p2align	6
.LBB5_11:                               ;   in Loop: Header=BB5_12 Depth=3
	v_add_nc_u32_e32 v15, s8, v15
	s_add_i32 s23, s23, 1
	s_add_i32 s0, s0, s20
	s_cmp_eq_u32 s23, s11
	s_cbranch_scc1 .LBB5_8
.LBB5_12:                               ;   Parent Loop BB5_6 Depth=1
                                        ;     Parent Loop BB5_9 Depth=2
                                        ; =>    This Loop Header: Depth=3
                                        ;         Child Loop BB5_14 Depth 4
	s_and_not1_b32 vcc_lo, exec_lo, s16
	s_cbranch_vccnz .LBB5_11
; %bb.13:                               ;   in Loop: Header=BB5_12 Depth=3
	s_lshl_b64 s[24:25], s[0:1], 3
	v_mov_b32_e32 v8, v15
	v_add_co_u32 v6, vcc_lo, v10, s24
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s25, v11, vcc_lo
	s_mov_b32 s24, s20
	.p2align	6
.LBB5_14:                               ;   Parent Loop BB5_6 Depth=1
                                        ;     Parent Loop BB5_9 Depth=2
                                        ;       Parent Loop BB5_12 Depth=3
                                        ; =>      This Inner Loop Header: Depth=4
	v_ashrrev_i32_e32 v9, 31, v8
	s_add_i32 s24, s24, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s24, 0
	v_lshlrev_b64 v[16:17], 3, v[8:9]
	v_add_nc_u32_e32 v8, 1, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v16, vcc_lo, v12, v16
	v_add_co_ci_u32_e64 v17, null, v13, v17, vcc_lo
	global_load_b64 v[18:19], v[6:7], off
	global_load_b64 v[16:17], v[16:17], off
	v_add_co_u32 v6, vcc_lo, v6, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[16:17], v[18:19], v[3:4]
	s_cbranch_scc0 .LBB5_14
	s_branch .LBB5_11
.LBB5_15:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s18, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB5_16:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_conv3d_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 336
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
		.amdhsa_inst_pref_size 11
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
	.size	convx_conv3d_kernel, .Lfunc_end5-convx_conv3d_kernel
                                        ; -- End function
	.set convx_conv3d_kernel.num_vgpr, 20
	.set convx_conv3d_kernel.num_agpr, 0
	.set convx_conv3d_kernel.numbered_sgpr, 26
	.set convx_conv3d_kernel.num_named_barrier, 0
	.set convx_conv3d_kernel.private_seg_size, 0
	.set convx_conv3d_kernel.uses_vcc, 1
	.set convx_conv3d_kernel.uses_flat_scratch, 0
	.set convx_conv3d_kernel.has_dyn_sized_stack, 0
	.set convx_conv3d_kernel.has_recursion, 0
	.set convx_conv3d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1376
; TotalNumSgprs: 28
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 28
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
	.protected	convx_dwconv1d_kernel   ; -- Begin function convx_dwconv1d_kernel
	.globl	convx_dwconv1d_kernel
	.p2align	8
	.type	convx_dwconv1d_kernel,@function
convx_dwconv1d_kernel:                  ; @convx_dwconv1d_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x44
	s_load_b128 s[12:15], s[0:1], 0x20
	s_load_b64 s[16:17], s[0:1], 0x30
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s15, s13
	s_mul_i32 s3, s2, s12
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s3, s3, s17
	v_cmp_gt_i32_e32 vcc_lo, s3, v1
	s_and_saveexec_b32 s3, vcc_lo
	s_cbranch_execz .LBB6_7
; %bb.1:
	s_abs_i32 s3, s17
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s3
	s_sub_i32 s4, 0, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s4, v0
	s_abs_i32 s4, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s4
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s3, v2
	v_cmp_le_u32_e32 vcc_lo, s3, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s17, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s3, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s3, 0, s4
	v_mul_lo_u32 v2, s3, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_abs_i32 s3, s15
	v_xor_b32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_hi_u32 v2, v4, v2
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v2, v4, v2
	v_sub_nc_u32_e32 v3, 0, v0
	v_xor_b32_e32 v6, s2, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v0, v3
	v_ashrrev_i32_e32 v6, 31, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v3, v2
	v_mul_lo_u32 v4, v2, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	v_subrev_nc_u32_e32 v5, s4, v3
	v_cmp_le_u32_e32 vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v5
	v_cvt_f32_u32_e32 v4, s3
	v_add_nc_u32_e32 v5, 1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s4, v3
	v_rcp_iflag_f32_e32 v4, v4
	s_load_b256 s[4:11], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v2, v5, vcc_lo
	v_xor_b32_e32 v2, v2, v6
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v3, 0x4f7ffffe, v4
	v_sub_nc_u32_e32 v5, v2, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v3, v3
	v_mul_lo_u32 v2, v5, s2
	s_sub_i32 s2, 0, s3
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[8:9], 0
	v_mul_lo_u32 v4, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v0, v2
	v_mul_hi_u32 v4, v3, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v6, 0, v2
	v_max_i32_e32 v7, v2, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v3, v3, v4
	v_mul_hi_u32 v6, v7, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v3, v6, s3
	v_sub_nc_u32_e32 v3, v7, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v4, s3, v3
	v_cmp_le_u32_e64 s0, s3, v3
	v_cndmask_b32_e64 v3, v3, v4, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	s_cbranch_scc1 .LBB6_3
; %bb.2:
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 3, v[2:3]
	v_add_co_u32 v3, s1, s8, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s9, v4, s1
	global_load_b64 v[3:4], v[3:4], off
	s_cmp_lt_i32 s16, 1
	s_cbranch_scc0 .LBB6_4
	s_branch .LBB6_6
.LBB6_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_lt_i32 s16, 1
	s_cbranch_scc1 .LBB6_6
.LBB6_4:
	v_add_nc_u32_e32 v7, 1, v6
	v_mul_lo_u32 v9, v0, s17
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v6, v6, v7, s0
	v_xor_b32_e32 v7, s15, v2
	v_add_nc_u32_e32 v8, 1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v7, 31, v7
	v_cndmask_b32_e32 v6, v6, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v6, v6, v7
	v_sub_nc_u32_e32 v6, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[7:8], null, v5, s13, v[6:7]
	v_mul_lo_u32 v5, v2, s16
	v_mul_lo_u32 v7, v7, s14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v6, 31, v5
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v8, 31, v7
	v_add_co_u32 v5, vcc_lo, s6, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	v_add_co_ci_u32_e64 v6, null, s7, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s4, v7
	v_add_co_ci_u32_e64 v2, null, s5, v8, vcc_lo
	v_sub_nc_u32_e32 v7, v1, v9
	.p2align	6
.LBB6_5:                                ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ashrrev_i32_e32 v8, 31, v7
	s_add_i32 s16, s16, -1
	s_cmp_eq_u32 s16, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[8:9], 3, v[7:8]
	v_add_nc_u32_e32 v7, 1, v7
	v_add_co_u32 v8, vcc_lo, v0, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, v2, v9, vcc_lo
	global_load_b64 v[10:11], v[5:6], off
	global_load_b64 v[8:9], v[8:9], off
	v_add_co_u32 v5, vcc_lo, v5, 8
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[8:9], v[10:11], v[3:4]
	s_cbranch_scc0 .LBB6_5
.LBB6_6:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s10, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s11, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB6_7:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_dwconv1d_kernel
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
		.amdhsa_next_free_vgpr 12
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
.Lfunc_end6:
	.size	convx_dwconv1d_kernel, .Lfunc_end6-convx_dwconv1d_kernel
                                        ; -- End function
	.set convx_dwconv1d_kernel.num_vgpr, 12
	.set convx_dwconv1d_kernel.num_agpr, 0
	.set convx_dwconv1d_kernel.numbered_sgpr, 18
	.set convx_dwconv1d_kernel.num_named_barrier, 0
	.set convx_dwconv1d_kernel.private_seg_size, 0
	.set convx_dwconv1d_kernel.uses_vcc, 1
	.set convx_dwconv1d_kernel.uses_flat_scratch, 0
	.set convx_dwconv1d_kernel.has_dyn_sized_stack, 0
	.set convx_dwconv1d_kernel.has_recursion, 0
	.set convx_dwconv1d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 956
; TotalNumSgprs: 20
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 20
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
	.protected	convx_dwconv2d_kernel   ; -- Begin function convx_dwconv2d_kernel
	.globl	convx_dwconv2d_kernel
	.p2align	8
	.type	convx_dwconv2d_kernel,@function
convx_dwconv2d_kernel:                  ; @convx_dwconv2d_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s12, s[0:1], 0x54
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b32 s3, s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s12, s12, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s12, v[0:1]
	s_mul_i32 s2, s8, s5
	s_mul_i32 s4, s2, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s4, s4, s11
	s_mul_i32 s4, s4, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s4, v1
	s_and_saveexec_b32 s4, vcc_lo
	s_cbranch_execz .LBB7_10
; %bb.1:
	s_abs_i32 s4, s3
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s12, 0, s4
	s_abs_i32 s20, s8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s12, v0
	s_abs_i32 s12, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s12
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s4, v2
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s3, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s4, 0, s12
	v_mul_lo_u32 v2, s4, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_abs_i32 s4, s2
	v_cvt_f32_u32_e32 v5, s4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v0, v0, v3
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rcp_iflag_f32_e32 v5, v5
	v_sub_nc_u32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v2, v4, v2
	v_sub_nc_u32_e32 v3, 0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v3, v0, v3
	v_mul_hi_u32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, v2, s12
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v6, s12, v3
	v_cmp_le_u32_e32 vcc_lo, s12, v3
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v6
	v_mul_f32_e32 v4, 0x4f7ffffe, v5
	v_xor_b32_e32 v5, s11, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_nc_u32_e32 v6, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s12, v3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_u32_f32_e32 v4, v4
	v_ashrrev_i32_e32 v5, 31, v5
	s_sub_i32 s12, 0, s4
	v_cndmask_b32_e32 v2, v2, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v3, s12, v4
	s_load_b256 s[12:19], s[0:1], 0x0
	v_xor_b32_e32 v2, v2, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v5, v2, v5
	v_mul_hi_u32 v2, v4, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v3, 0, v5
	v_xor_b32_e32 v7, s2, v5
	v_add_nc_u32_e32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_max_i32_e32 v3, v5, v3
	v_ashrrev_i32_e32 v7, 31, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v3, v2
	v_mul_lo_u32 v4, v2, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	v_subrev_nc_u32_e32 v6, s4, v3
	v_cmp_le_u32_e32 vcc_lo, s4, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v6
	v_cvt_f32_u32_e32 v4, s20
	v_add_nc_u32_e32 v6, 1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s4, v3
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v2, v6, vcc_lo
	v_xor_b32_e32 v2, v2, v7
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v3, 0x4f7ffffe, v4
	v_sub_nc_u32_e32 v6, v2, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v3, v3
	v_mul_lo_u32 v2, v6, s2
	s_sub_i32 s2, 0, s20
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[16:17], 0
	v_mul_lo_u32 v4, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v5, v2
	v_mul_hi_u32 v4, v3, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v7, 0, v2
	v_max_i32_e32 v8, v2, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v3, v3, v4
	v_mul_hi_u32 v7, v8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v3, v7, s20
	v_sub_nc_u32_e32 v3, v8, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_subrev_nc_u32_e32 v4, s20, v3
	v_cmp_le_u32_e64 s0, s20, v3
	v_cndmask_b32_e64 v3, v3, v4, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_u32_e32 vcc_lo, s20, v3
	s_cbranch_scc1 .LBB7_3
; %bb.2:
	v_ashrrev_i32_e32 v3, 31, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[3:4], 3, v[2:3]
	v_add_co_u32 v3, s1, s16, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s17, v4, s1
	global_load_b64 v[3:4], v[3:4], off
	s_cmp_lt_i32 s9, 1
	s_cbranch_scc0 .LBB7_4
	s_branch .LBB7_9
.LBB7_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_lt_i32 s9, 1
	s_cbranch_scc1 .LBB7_9
.LBB7_4:
	v_add_nc_u32_e32 v8, 1, v7
	v_mul_lo_u32 v11, v0, s3
	s_mov_b32 s1, 0
	s_cmp_gt_i32 s10, 0
	s_mov_b32 s3, s1
	v_cndmask_b32_e64 v7, v7, v8, s0
	v_xor_b32_e32 v8, s8, v2
	s_mul_i32 s0, s10, s9
	s_cselect_b32 s2, -1, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v9, 1, v7
	v_ashrrev_i32_e32 v8, 31, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v7, v7, v9, vcc_lo
	v_xor_b32_e32 v7, v7, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v7, v7, v8
	v_mad_u64_u32 v[8:9], null, v6, s5, v[7:8]
	v_mul_lo_u32 v6, s0, v2
	s_mul_i32 s0, s7, s6
	v_mul_lo_u32 v2, v5, s11
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v5, s0, v8
	s_mov_b32 s0, s1
	v_ashrrev_i32_e32 v7, 31, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v0, v2
	v_lshlrev_b64 v[7:8], 3, v[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v6, 31, v5
	v_mad_u64_u32 v[9:10], null, s7, v2, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_u32 v0, vcc_lo, s14, v7
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	v_add_co_ci_u32_e64 v2, null, s15, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v9, v9, v11
	v_add_co_u32 v10, vcc_lo, s12, v5
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v11, null, s13, v6, vcc_lo
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB7_6
	.p2align	6
.LBB7_5:                                ;   in Loop: Header=BB7_6 Depth=1
	v_add_nc_u32_e32 v9, s7, v9
	s_add_i32 s3, s3, 1
	s_add_i32 s0, s0, s10
	s_cmp_eq_u32 s3, s9
	s_cbranch_scc1 .LBB7_9
.LBB7_6:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB7_8 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s2
	s_cbranch_vccnz .LBB7_5
; %bb.7:                                ;   in Loop: Header=BB7_6 Depth=1
	s_lshl_b64 s[4:5], s[0:1], 3
	v_mov_b32_e32 v7, v9
	v_add_co_u32 v5, vcc_lo, v0, s4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s5, v2, vcc_lo
	s_mov_b32 s4, s10
	.p2align	6
.LBB7_8:                                ;   Parent Loop BB7_6 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_ashrrev_i32_e32 v8, 31, v7
	s_add_i32 s4, s4, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s4, 0
	v_lshlrev_b64 v[12:13], 3, v[7:8]
	v_add_nc_u32_e32 v7, 1, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v12, vcc_lo, v10, v12
	v_add_co_ci_u32_e64 v13, null, v11, v13, vcc_lo
	global_load_b64 v[14:15], v[5:6], off
	global_load_b64 v[12:13], v[12:13], off
	v_add_co_u32 v5, vcc_lo, v5, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[12:13], v[14:15], v[3:4]
	s_cbranch_scc0 .LBB7_8
	s_branch .LBB7_5
.LBB7_9:
	s_set_inst_prefetch_distance 0x2
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s18, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB7_10:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_dwconv2d_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 328
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
		.amdhsa_next_free_sgpr 21
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
.Lfunc_end7:
	.size	convx_dwconv2d_kernel, .Lfunc_end7-convx_dwconv2d_kernel
                                        ; -- End function
	.set convx_dwconv2d_kernel.num_vgpr, 16
	.set convx_dwconv2d_kernel.num_agpr, 0
	.set convx_dwconv2d_kernel.numbered_sgpr, 21
	.set convx_dwconv2d_kernel.num_named_barrier, 0
	.set convx_dwconv2d_kernel.private_seg_size, 0
	.set convx_dwconv2d_kernel.uses_vcc, 1
	.set convx_dwconv2d_kernel.uses_flat_scratch, 0
	.set convx_dwconv2d_kernel.has_dyn_sized_stack, 0
	.set convx_dwconv2d_kernel.has_recursion, 0
	.set convx_dwconv2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1252
; TotalNumSgprs: 23
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 23
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
	.protected	convx_gconv2d_kernel    ; -- Begin function convx_gconv2d_kernel
	.globl	convx_gconv2d_kernel
	.p2align	8
	.type	convx_gconv2d_kernel,@function
convx_gconv2d_kernel:                   ; @convx_gconv2d_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x54
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b64 s[20:21], s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s8, s4
	s_mul_i32 s2, s2, s20
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s21
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB8_14
; %bb.1:
	s_abs_i32 s2, s21
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_abs_i32 s4, s8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s3, v0
	s_abs_i32 s3, s20
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s3
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s2, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s21, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s2, 0, s3
	v_mul_lo_u32 v2, s2, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	v_cvt_f32_u32_e32 v5, s4
	s_sub_i32 s2, 0, s4
	v_xor_b32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_rcp_iflag_f32_e32 v5, v5
	v_mul_hi_u32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v0, v0, v3
	v_add_nc_u32_e32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v3, 0, v0
	v_max_i32_e32 v3, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v3, v2
	v_mul_lo_u32 v4, v2, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	v_subrev_nc_u32_e32 v6, s3, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_dual_mul_f32 v4, 0x4f7ffffe, v5 :: v_dual_cndmask_b32 v3, v3, v6
	v_xor_b32_e32 v5, s20, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_u32_f32_e32 v4, v4
	v_cmp_le_u32_e32 vcc_lo, s3, v3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v5, 31, v5
	s_ashr_i32 s3, s8, 31
	v_mul_lo_u32 v3, s2, v4
	s_abs_i32 s2, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_sub_i32 s12, 0, s2
	v_mul_hi_u32 v3, v4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v3, v4, v3
	v_add_nc_u32_e32 v6, 1, v2
	v_cndmask_b32_e32 v2, v2, v6, vcc_lo
	v_cvt_f32_u32_e32 v6, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v2, v2, v5
	v_rcp_iflag_f32_e32 v6, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v2, v5
	v_sub_nc_u32_e32 v5, 0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v4, v2, v5
	v_mul_hi_u32 v3, v4, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v5, v3, s4
	v_sub_nc_u32_e32 v4, v4, v5
	v_add_nc_u32_e32 v5, 1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v7, s4, v4
	v_cmp_le_u32_e32 vcc_lo, s4, v4
	v_dual_cndmask_b32 v3, v3, v5 :: v_dual_cndmask_b32 v4, v4, v7
	v_mul_f32_e32 v5, 0x4f7ffffe, v6
	v_ashrrev_i32_e32 v6, 31, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_nc_u32_e32 v7, 1, v3
	v_cmp_le_u32_e32 vcc_lo, s4, v4
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_u32_f32_e32 v5, v5
	v_xor_b32_e32 v6, s3, v6
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v3, v3, v7, vcc_lo
	v_readfirstlane_b32 s22, v5
	s_delay_alu instid0(VALU_DEP_2)
	v_xor_b32_e32 v3, v3, v6
	s_mul_i32 s23, s12, s22
	s_load_b256 s[12:19], s[0:1], 0x0
	s_mul_hi_u32 s0, s22, s23
	s_ashr_i32 s1, s5, 31
	v_sub_nc_u32_e32 v7, v3, v6
	s_add_i32 s22, s22, s0
	s_ashr_i32 s0, s9, 31
	s_mul_hi_u32 s23, s4, s22
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_lo_u32 v3, v7, s8
	s_mul_i32 s9, s23, s2
	s_abs_i32 s8, s5
	s_sub_i32 s4, s4, s9
	s_add_i32 s9, s23, 1
	s_sub_i32 s24, s4, s2
	s_cmp_ge_u32 s4, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_sub_nc_u32_e32 v5, v2, v3
	s_cselect_b32 s9, s9, s23
	s_cselect_b32 s4, s24, s4
	s_add_i32 s23, s9, 1
	s_cmp_ge_u32 s4, s2
	v_ashrrev_i32_e32 v6, 31, v5
	s_cselect_b32 s4, s23, s9
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[16:17], 0
	s_mul_hi_u32 s9, s8, s22
	s_cbranch_scc1 .LBB8_3
; %bb.2:
	v_lshlrev_b64 v[3:4], 3, v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s16, v3
	v_add_co_ci_u32_e64 v4, null, s17, v4, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_branch .LBB8_4
.LBB8_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
.LBB8_4:
	s_mul_i32 s16, s9, s2
	s_xor_b32 s1, s1, s0
	s_sub_i32 s8, s8, s16
	s_add_i32 s16, s9, 1
	s_sub_i32 s17, s8, s2
	s_cmp_ge_u32 s8, s2
	s_cselect_b32 s9, s16, s9
	s_cselect_b32 s8, s17, s8
	s_add_i32 s16, s9, 1
	s_cmp_ge_u32 s8, s2
	s_cselect_b32 s2, s16, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b32 s2, s2, s1
	s_sub_i32 s2, s2, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lt_i32 s2, 1
	s_cbranch_scc1 .LBB8_13
; %bb.5:
	s_xor_b32 s0, s3, s0
	v_sub_nc_u32_e32 v10, 0, v5
	s_xor_b32 s1, s4, s0
	v_mul_lo_u32 v2, v2, s20
	s_sub_i32 s0, s1, s0
	s_mul_i32 s6, s7, s6
	s_abs_i32 s1, s0
	v_max_i32_e32 v10, v5, v10
	v_cvt_f32_u32_e32 v8, s1
	s_sub_i32 s3, 0, s1
	s_ashr_i32 s0, s0, 31
	v_sub_nc_u32_e32 v2, v0, v2
	s_cmp_gt_i32 s10, 0
	v_rcp_iflag_f32_e32 v8, v8
	s_cselect_b32 s4, -1, 0
	s_cmp_gt_i32 s11, 0
	s_mov_b32 s8, 0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v8, 0x4f7ffffe, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v8, v8
	v_mul_lo_u32 v9, s3, v8
	s_mul_i32 s3, s11, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v5, s3, v5
	v_mul_hi_u32 v9, v8, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v5, v5, s2
	v_add_nc_u32_e32 v11, v8, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[8:9], null, v10, v11, 0
	v_mul_lo_u32 v8, v9, s1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v8, v10, v8
	v_add_nc_u32_e32 v10, 1, v9
	v_subrev_nc_u32_e32 v11, s1, v8
	v_cmp_le_u32_e32 vcc_lo, s1, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v9, v9, v10 :: v_dual_cndmask_b32 v8, v8, v11
	v_xor_b32_e32 v11, s0, v6
	v_add_nc_u32_e32 v10, 1, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s1, v8
	s_mov_b32 s1, 0
	v_cndmask_b32_e32 v6, v9, v10, vcc_lo
	v_mul_lo_u32 v9, v0, s21
	v_mul_lo_u32 v0, v7, s5
	s_cselect_b32 s5, -1, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v8, v6, v11
	v_mad_u64_u32 v[6:7], null, s7, v2, v[1:2]
	v_sub_nc_u32_e32 v2, v8, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[7:8], null, v2, s2, v[0:1]
	v_sub_nc_u32_e32 v0, v6, v9
	s_branch .LBB8_7
.LBB8_6:                                ;   in Loop: Header=BB8_7 Depth=1
	s_set_inst_prefetch_distance 0x2
	v_add_nc_u32_e32 v5, s3, v5
	s_add_i32 s8, s8, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s8, s2
	s_cbranch_scc1 .LBB8_13
.LBB8_7:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB8_10 Depth 2
                                        ;       Child Loop BB8_12 Depth 3
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccnz .LBB8_6
; %bb.8:                                ;   in Loop: Header=BB8_7 Depth=1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_add_nc_u32_e32 v2, s8, v7
	v_ashrrev_i32_e32 v6, 31, v5
	v_mov_b32_e32 v14, v0
	s_mov_b32 s0, 0
	s_mov_b32 s9, s0
	v_mul_lo_u32 v8, s6, v2
	v_lshlrev_b64 v[10:11], 3, v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_u32 v2, vcc_lo, s14, v10
	v_ashrrev_i32_e32 v9, 31, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v6, null, s15, v11, vcc_lo
	v_lshlrev_b64 v[8:9], 3, v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v12, vcc_lo, s12, v8
	v_add_co_ci_u32_e64 v13, null, s13, v9, vcc_lo
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB8_10
	.p2align	6
.LBB8_9:                                ;   in Loop: Header=BB8_10 Depth=2
	v_add_nc_u32_e32 v14, s7, v14
	s_add_i32 s9, s9, 1
	s_add_i32 s0, s0, s11
	s_cmp_eq_u32 s9, s10
	s_cbranch_scc1 .LBB8_6
.LBB8_10:                               ;   Parent Loop BB8_7 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB8_12 Depth 3
	s_and_not1_b32 vcc_lo, exec_lo, s5
	s_cbranch_vccnz .LBB8_9
; %bb.11:                               ;   in Loop: Header=BB8_10 Depth=2
	s_lshl_b64 s[16:17], s[0:1], 3
	v_mov_b32_e32 v10, v14
	v_add_co_u32 v8, vcc_lo, v2, s16
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s17, v6, vcc_lo
	s_mov_b32 s16, s11
	.p2align	6
.LBB8_12:                               ;   Parent Loop BB8_7 Depth=1
                                        ;     Parent Loop BB8_10 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_ashrrev_i32_e32 v11, 31, v10
	s_add_i32 s16, s16, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_cmp_eq_u32 s16, 0
	v_lshlrev_b64 v[15:16], 3, v[10:11]
	v_add_nc_u32_e32 v10, 1, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v15, vcc_lo, v12, v15
	v_add_co_ci_u32_e64 v16, null, v13, v16, vcc_lo
	global_load_b64 v[17:18], v[8:9], off
	global_load_b64 v[15:16], v[15:16], off
	v_add_co_u32 v8, vcc_lo, v8, 8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[15:16], v[17:18], v[3:4]
	s_cbranch_scc0 .LBB8_12
	s_branch .LBB8_9
.LBB8_13:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s18, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB8_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_gconv2d_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 328
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
		.amdhsa_next_free_sgpr 25
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
.Lfunc_end8:
	.size	convx_gconv2d_kernel, .Lfunc_end8-convx_gconv2d_kernel
                                        ; -- End function
	.set convx_gconv2d_kernel.num_vgpr, 19
	.set convx_gconv2d_kernel.num_agpr, 0
	.set convx_gconv2d_kernel.numbered_sgpr, 25
	.set convx_gconv2d_kernel.num_named_barrier, 0
	.set convx_gconv2d_kernel.private_seg_size, 0
	.set convx_gconv2d_kernel.uses_vcc, 1
	.set convx_gconv2d_kernel.uses_flat_scratch, 0
	.set convx_gconv2d_kernel.has_dyn_sized_stack, 0
	.set convx_gconv2d_kernel.has_recursion, 0
	.set convx_gconv2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1472
; TotalNumSgprs: 27
; NumVgprs: 19
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 27
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
	.protected	convx_convtranspose2d_kernel ; -- Begin function convx_convtranspose2d_kernel
	.globl	convx_convtranspose2d_kernel
	.p2align	8
	.type	convx_convtranspose2d_kernel,@function
convx_convtranspose2d_kernel:           ; @convx_convtranspose2d_kernel
; %bb.0:
	s_clause 0x2
	s_load_b32 s12, s[0:1], 0x54
	s_load_b256 s[4:11], s[0:1], 0x20
	s_load_b32 s3, s[0:1], 0x40
	s_waitcnt lgkmcnt(0)
	s_and_b32 s12, s12, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s12, v[0:1]
	s_mul_i32 s2, s8, s4
	s_mul_i32 s2, s2, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s3
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB9_15
; %bb.1:
	s_abs_i32 s2, s3
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s4, 0, s2
	s_load_b256 s[12:19], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s4, v0
	s_abs_i32 s4, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_u32_e32 v4, s4
	v_rcp_iflag_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v4, v4
	v_mul_lo_u32 v2, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s2, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s3, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	v_add_nc_u32_e32 v5, 1, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_ashrrev_i32_e32 v3, 31, v3
	s_sub_i32 s2, 0, s4
	v_mul_lo_u32 v2, s2, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_abs_i32 s2, s8
	v_xor_b32_e32 v0, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_sub_nc_u32_e32 v5, v0, v3
	v_mul_hi_u32 v0, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, 0, v5
	v_add_nc_u32_e32 v0, v4, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_max_i32_e32 v2, v5, v2
	v_cvt_f32_u32_e32 v4, s2
	v_mul_hi_u32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v4, v4
	v_mul_lo_u32 v3, v0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v2, v3
	v_subrev_nc_u32_e32 v6, s4, v2
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v2, v2, v6 :: v_dual_add_nc_u32 v3, 1, v0
	s_waitcnt_depctr 0xfff
	v_dual_cndmask_b32 v0, v0, v3 :: v_dual_mul_f32 v3, 0x4f7ffffe, v4
	v_xor_b32_e32 v4, s11, v5
	v_cmp_le_u32_e32 vcc_lo, s4, v2
	v_add_nc_u32_e32 v6, 1, v0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_u32_f32_e32 v3, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_sub_i32 s4, 0, s2
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u64 s[16:17], 0
	v_cndmask_b32_e32 v0, v0, v6, vcc_lo
	v_mul_lo_u32 v2, s4, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v4
	v_mul_hi_u32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v4
	v_sub_nc_u32_e32 v4, 0, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v2, v3, v2
	v_max_i32_e32 v3, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v3, v2
	v_mul_lo_u32 v4, v2, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v3, v3, v4
	v_add_nc_u32_e32 v4, 1, v2
	v_subrev_nc_u32_e32 v6, s2, v3
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, v2, v4 :: v_dual_cndmask_b32 v3, v3, v6
	v_xor_b32_e32 v4, s8, v0
	v_add_nc_u32_e32 v6, 1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v3
	v_ashrrev_i32_e32 v4, 31, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v2, v6, vcc_lo
	v_xor_b32_e32 v2, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v6, v2, v4
	v_mul_lo_u32 v2, v6, s8
	s_delay_alu instid0(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v0, v2
	s_cbranch_scc1 .LBB9_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[3:4], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s16, v3
	v_add_co_ci_u32_e64 v4, null, s17, v4, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc0 .LBB9_4
	s_branch .LBB9_14
.LBB9_3:
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_cmp_lt_i32 s5, 1
	s_cbranch_scc1 .LBB9_14
.LBB9_4:
	v_mul_lo_u32 v7, v5, s3
	v_mul_lo_u32 v8, v0, s11
	s_cmp_gt_i32 s9, 0
	s_mul_i32 s1, s7, s6
	s_cselect_b32 s11, -1, 0
	s_cmp_lt_i32 s10, 1
	s_mul_i32 s4, s10, s9
	s_mov_b32 s3, 0
	v_sub_nc_u32_e32 v0, v1, v7
	v_sub_nc_u32_e32 v7, v5, v8
	v_mul_lo_u32 v8, v6, s5
	v_mov_b32_e32 v6, 0
	s_cselect_b32 s16, -1, 0
	s_mov_b32 s17, 0
	v_mul_lo_u32 v9, s7, v7
	s_branch .LBB9_6
.LBB9_5:                                ;   in Loop: Header=BB9_6 Depth=1
	s_add_i32 s17, s17, 1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s17, s5
	s_cbranch_scc1 .LBB9_14
.LBB9_6:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB9_9 Depth 2
                                        ;       Child Loop BB9_12 Depth 3
	s_and_not1_b32 vcc_lo, exec_lo, s11
	s_cbranch_vccnz .LBB9_5
; %bb.7:                                ;   in Loop: Header=BB9_6 Depth=1
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[10:11], null, s17, s8, v[2:3]
	v_add_nc_u32_e32 v5, s17, v8
	s_mov_b32 s20, 0
	s_mov_b32 s21, 0
	v_mul_lo_u32 v11, s1, v5
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v13, s4, v10
	v_ashrrev_i32_e32 v12, 31, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v14, 31, v13
	v_lshlrev_b64 v[10:11], 3, v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[12:13], 3, v[13:14]
	v_mov_b32_e32 v14, v9
	v_add_co_u32 v10, vcc_lo, s12, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v11, null, s13, v11, vcc_lo
	v_add_co_u32 v12, vcc_lo, s14, v12
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v13, null, s15, v13, vcc_lo
	s_branch .LBB9_9
.LBB9_8:                                ;   in Loop: Header=BB9_9 Depth=2
	s_set_inst_prefetch_distance 0x2
	s_or_b32 exec_lo, exec_lo, s22
	v_subrev_nc_u32_e32 v14, s7, v14
	s_add_i32 s21, s21, 1
	s_add_i32 s20, s20, s10
	s_cmp_eq_u32 s21, s9
	s_cbranch_scc1 .LBB9_5
.LBB9_9:                                ;   Parent Loop BB9_6 Depth=1
                                        ; =>  This Loop Header: Depth=2
                                        ;       Child Loop BB9_12 Depth 3
	v_subrev_nc_u32_e32 v5, s21, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, 0, v5
	v_cmp_le_i32_e64 s0, s6, v5
	s_or_b32 s0, vcc_lo, s0
	s_nor_b32 s0, s0, s16
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s22, s0
	s_cbranch_execz .LBB9_8
; %bb.10:                               ;   in Loop: Header=BB9_9 Depth=2
	v_mov_b32_e32 v15, v0
	s_mov_b32 s23, s10
	s_mov_b32 s2, s20
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB9_12
	.p2align	6
.LBB9_11:                               ;   in Loop: Header=BB9_12 Depth=3
	s_or_b32 exec_lo, exec_lo, s0
	v_add_nc_u32_e32 v15, -1, v15
	s_add_i32 s23, s23, -1
	s_add_i32 s2, s2, 1
	s_cmp_lg_u32 s23, 0
	s_cbranch_scc0 .LBB9_8
.LBB9_12:                               ;   Parent Loop BB9_6 Depth=1
                                        ;     Parent Loop BB9_9 Depth=2
                                        ; =>    This Inner Loop Header: Depth=3
	v_cmp_lt_i32_e32 vcc_lo, -1, v15
	v_cmp_gt_i32_e64 s0, s7, v15
	s_and_b32 s24, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s0, s24
	s_cbranch_execz .LBB9_11
; %bb.13:                               ;   in Loop: Header=BB9_12 Depth=3
	v_add_nc_u32_e32 v5, v14, v15
	s_lshl_b64 s[24:25], s[2:3], 3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[16:17], 3, v[5:6]
	v_add_co_u32 v16, vcc_lo, v10, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v17, null, v11, v17, vcc_lo
	v_add_co_u32 v18, vcc_lo, v12, s24
	v_add_co_ci_u32_e64 v19, null, s25, v13, vcc_lo
	global_load_b64 v[16:17], v[16:17], off
	global_load_b64 v[18:19], v[18:19], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[3:4], v[16:17], v[18:19], v[3:4]
	s_branch .LBB9_11
.LBB9_14:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_add_co_u32 v0, vcc_lo, s18, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s19, v1, vcc_lo
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[3:4], off
.LBB9_15:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_convtranspose2d_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 328
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
.Lfunc_end9:
	.size	convx_convtranspose2d_kernel, .Lfunc_end9-convx_convtranspose2d_kernel
                                        ; -- End function
	.set convx_convtranspose2d_kernel.num_vgpr, 20
	.set convx_convtranspose2d_kernel.num_agpr, 0
	.set convx_convtranspose2d_kernel.numbered_sgpr, 26
	.set convx_convtranspose2d_kernel.num_named_barrier, 0
	.set convx_convtranspose2d_kernel.private_seg_size, 0
	.set convx_convtranspose2d_kernel.uses_vcc, 1
	.set convx_convtranspose2d_kernel.uses_flat_scratch, 0
	.set convx_convtranspose2d_kernel.has_dyn_sized_stack, 0
	.set convx_convtranspose2d_kernel.has_recursion, 0
	.set convx_convtranspose2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1188
; TotalNumSgprs: 28
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 28
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
	.protected	convx_dilation2d_kernel ; -- Begin function convx_dilation2d_kernel
	.globl	convx_dilation2d_kernel
	.p2align	8
	.type	convx_dilation2d_kernel,@function
convx_dilation2d_kernel:                ; @convx_dilation2d_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x4c
	s_load_b256 s[4:11], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	s_mul_i32 s2, s2, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s2, s2, s11
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB10_12
; %bb.1:
	s_abs_i32 s2, s11
	s_mov_b32 s18, 0x85ebc8a0
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	v_sub_nc_u32_e32 v3, 0, v1
	s_abs_i32 s16, s10
	s_load_b32 s17, s[0:1], 0x38
	v_rcp_iflag_f32_e32 v0, v0
	v_cvt_f32_u32_e32 v4, s16
	v_max_i32_e32 v3, v1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v4, v4
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	v_mul_f32_e32 v4, 0x4f7ffffe, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v2, s3, v0
	v_cvt_u32_f32_e32 v4, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v2, v0, v2
	v_add_nc_u32_e32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_hi_u32 v0, v3, v0
	v_mul_lo_u32 v2, v0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v3, v2
	v_subrev_nc_u32_e32 v5, s2, v2
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v2, v2, v5 :: v_dual_add_nc_u32 v3, 1, v0
	v_cndmask_b32_e32 v0, v0, v3, vcc_lo
	v_xor_b32_e32 v3, s11, v1
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_le_u32_e32 vcc_lo, s2, v2
	s_sub_i32 s2, 0, s16
	s_waitcnt lgkmcnt(0)
	s_cmp_lg_u32 s17, 0
	v_mul_lo_u32 v2, s2, v4
	s_cselect_b32 s4, -1, 0
	s_cmp_eq_u32 s17, 0
	s_clause 0x1
	s_load_b128 s[12:15], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_cselect_b32 s0, -1, 0
	s_mov_b32 s1, 0xffe1ccf3
	s_and_b32 s17, s0, exec_lo
	s_cselect_b32 s19, s1, 0x7fe1ccf3
	v_add_nc_u32_e32 v5, 1, v0
	v_ashrrev_i32_e32 v3, 31, v3
	v_mul_hi_u32 v2, v4, v2
	s_cmp_lt_i32 s8, 1
	s_mov_b32 s17, 0
	v_cndmask_b32_e32 v0, v0, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_xor_b32_e32 v0, v0, v3
	v_add_nc_u32_e32 v2, v4, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v3, 0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_i32_e32 v3, v0, v3
	v_mul_hi_u32 v2, v3, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v4, v2, s16
	v_sub_nc_u32_e32 v3, v3, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v4, s16, v3
	v_cmp_le_u32_e32 vcc_lo, s16, v3
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_le_u32_e64 s1, s16, v3
	v_dual_mov_b32 v3, s18 :: v_dual_mov_b32 v4, s19
	s_cbranch_scc1 .LBB10_11
; %bb.2:
	s_abs_i32 s5, s5
	v_mul_lo_u32 v11, v0, s11
	v_cvt_f32_u32_e32 v3, s5
	s_mov_b32 s16, s17
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v3, v3
	s_waitcnt_depctr 0xfff
	v_dual_mul_f32 v3, 0x4f7ffffe, v3 :: v_dual_add_nc_u32 v4, 1, v2
	v_cndmask_b32_e32 v2, v2, v4, vcc_lo
	v_xor_b32_e32 v4, s10, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_u32_f32_e32 v3, v3
	v_add_nc_u32_e32 v5, 1, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v4, 31, v4
	v_cndmask_b32_e64 v2, v2, v5, s1
	s_sub_i32 s1, 0, s5
	s_cmp_gt_i32 s9, 0
	v_mul_lo_u32 v5, s1, v3
	s_mul_i32 s1, s7, s6
	v_xor_b32_e32 v2, v2, v4
	s_mov_b32 s6, s17
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v4, v2, v4
	v_mul_hi_u32 v2, v3, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v5, 0, v4
	v_mul_lo_u32 v8, v4, s10
	v_add_nc_u32_e32 v6, v3, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_max_i32_e32 v5, v4, v5
	v_sub_nc_u32_e32 v0, v0, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[2:3], null, v5, v6, 0
	v_ashrrev_i32_e32 v6, 31, v4
	v_mul_lo_u32 v2, v3, s5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v5, v2
	v_mul_lo_u32 v5, s1, v4
	s_mul_i32 s1, s9, s8
	v_subrev_nc_u32_e32 v3, s5, v2
	v_cmp_le_u32_e32 vcc_lo, s5, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v2, v2, v3, vcc_lo
	v_subrev_nc_u32_e32 v3, s5, v2
	v_cmp_le_u32_e32 vcc_lo, s5, v2
	s_cselect_b32 s5, -1, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v2, v2, v3 :: v_dual_mov_b32 v3, s18
	v_mov_b32_e32 v4, s19
	v_xor_b32_e32 v2, v2, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v2, v2, v6
	v_ashrrev_i32_e32 v6, 31, v5
	v_mul_lo_u32 v7, s1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[5:6], 3, v[5:6]
	v_mad_u64_u32 v[9:10], null, s7, v0, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s12, v5
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ashrrev_i32_e32 v8, 31, v7
	v_add_co_ci_u32_e64 v2, null, s13, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v14, v9, v11
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v15, vcc_lo, s14, v7
	v_add_co_ci_u32_e64 v16, null, s15, v8, vcc_lo
	s_branch .LBB10_4
.LBB10_3:                               ;   in Loop: Header=BB10_4 Depth=1
	s_set_inst_prefetch_distance 0x2
	v_add_nc_u32_e32 v14, s7, v14
	s_add_i32 s6, s6, 1
	s_add_i32 s16, s16, s9
	s_cmp_eq_u32 s6, s8
	s_cbranch_scc1 .LBB10_11
.LBB10_4:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB10_7 Depth 2
	s_and_not1_b32 vcc_lo, exec_lo, s5
	s_cbranch_vccnz .LBB10_3
; %bb.5:                                ;   in Loop: Header=BB10_4 Depth=1
	s_lshl_b64 s[10:11], s[16:17], 3
	v_mov_b32_e32 v7, v14
	v_add_co_u32 v5, vcc_lo, v15, s10
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s11, v16, vcc_lo
	s_mov_b32 s10, s9
	s_set_inst_prefetch_distance 0x1
	s_branch .LBB10_7
	.p2align	6
.LBB10_6:                               ;   in Loop: Header=BB10_7 Depth=2
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_lt_f64_e32 vcc_lo, v[8:9], v[3:4]
	v_cmp_gt_f64_e64 s1, v[8:9], v[3:4]
	s_add_i32 s10, s10, -1
	v_add_nc_u32_e32 v7, 1, v7
	s_cmp_eq_u32 s10, 0
	s_waitcnt vmcnt(1)
	v_cndmask_b32_e32 v10, v3, v8, vcc_lo
	v_cndmask_b32_e64 v3, v3, v8, s1
	v_cndmask_b32_e32 v8, v4, v9, vcc_lo
	v_cndmask_b32_e64 v4, v4, v9, s1
	v_add_co_u32 v5, vcc_lo, v5, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v6, null, 0, v6, vcc_lo
	v_cndmask_b32_e64 v3, v10, v3, s0
	v_cndmask_b32_e64 v4, v8, v4, s0
	s_cbranch_scc1 .LBB10_3
.LBB10_7:                               ;   Parent Loop BB10_4 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_ashrrev_i32_e32 v8, 31, v7
	s_mov_b32 s1, -1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[8:9], 3, v[7:8]
	v_add_co_u32 v8, vcc_lo, v0, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, v2, v9, vcc_lo
	s_and_not1_b32 vcc_lo, exec_lo, s4
	global_load_b64 v[10:11], v[8:9], off
	global_load_b64 v[12:13], v[5:6], off
                                        ; implicit-def: $vgpr8_vgpr9
	s_cbranch_vccnz .LBB10_9
; %bb.8:                                ;   in Loop: Header=BB10_7 Depth=2
	s_waitcnt vmcnt(0)
	v_add_f64 v[8:9], v[10:11], -v[12:13]
	s_mov_b32 s1, 0
.LBB10_9:                               ;   in Loop: Header=BB10_7 Depth=2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 vcc_lo, exec_lo, s1
	s_cbranch_vccnz .LBB10_6
; %bb.10:                               ;   in Loop: Header=BB10_7 Depth=2
	s_waitcnt vmcnt(0)
	v_add_f64 v[8:9], v[10:11], v[12:13]
	s_branch .LBB10_6
.LBB10_11:
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[3:4], off
.LBB10_12:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel convx_dilation2d_kernel
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
		.amdhsa_next_free_vgpr 17
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
.Lfunc_end10:
	.size	convx_dilation2d_kernel, .Lfunc_end10-convx_dilation2d_kernel
                                        ; -- End function
	.set convx_dilation2d_kernel.num_vgpr, 17
	.set convx_dilation2d_kernel.num_agpr, 0
	.set convx_dilation2d_kernel.numbered_sgpr, 20
	.set convx_dilation2d_kernel.num_named_barrier, 0
	.set convx_dilation2d_kernel.private_seg_size, 0
	.set convx_dilation2d_kernel.uses_vcc, 1
	.set convx_dilation2d_kernel.uses_flat_scratch, 0
	.set convx_dilation2d_kernel.has_dyn_sized_stack, 0
	.set convx_dilation2d_kernel.has_recursion, 0
	.set convx_dilation2d_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1128
; TotalNumSgprs: 22
; NumVgprs: 17
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 22
; NumVGPRsForWavesPerEU: 17
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
	.type	__hip_cuid_1f9972cdfad5d976,@object ; @__hip_cuid_1f9972cdfad5d976
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_1f9972cdfad5d976
__hip_cuid_1f9972cdfad5d976:
	.byte	0                               ; 0x0
	.size	__hip_cuid_1f9972cdfad5d976, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_1f9972cdfad5d976
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
    .name:           convx_conv1d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         convx_conv1d_kernel.kd
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
    .name:           convx_conv1d_backward_data_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         convx_conv1d_backward_data_kernel.kd
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
    .group_segment_fixed_size: 2048
    .kernarg_segment_align: 8
    .kernarg_segment_size: 312
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           convx_conv1d_backward_filter_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     30
    .sgpr_spill_count: 0
    .symbol:         convx_conv1d_backward_filter_kernel.kd
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
    .group_segment_fixed_size: 2048
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           convx_conv1d_backward_bias_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     17
    .sgpr_spill_count: 0
    .symbol:         convx_conv1d_backward_bias_kernel.kd
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
        .value_kind:     by_value
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         76
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         80
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         84
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         86
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         88
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         90
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         92
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         94
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         136
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 328
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           convx_conv2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         convx_conv2d_kernel.kd
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
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         68
        .size:           4
        .value_kind:     by_value
      - .offset:         72
        .size:           4
        .value_kind:     by_value
      - .offset:         76
        .size:           4
        .value_kind:     by_value
      - .offset:         80
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         84
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         88
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         92
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         94
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         96
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         98
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         100
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         102
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         136
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         144
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 336
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           convx_conv3d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     28
    .sgpr_spill_count: 0
    .symbol:         convx_conv3d_kernel.kd
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
    .name:           convx_dwconv1d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     20
    .sgpr_spill_count: 0
    .symbol:         convx_dwconv1d_kernel.kd
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
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         76
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         80
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         84
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         86
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         88
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         90
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         92
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         94
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         136
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 328
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           convx_dwconv2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     23
    .sgpr_spill_count: 0
    .symbol:         convx_dwconv2d_kernel.kd
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
        .value_kind:     by_value
      - .offset:         68
        .size:           4
        .value_kind:     by_value
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         76
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         80
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         84
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         86
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         88
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         90
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         92
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         94
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         136
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 328
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           convx_gconv2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     27
    .sgpr_spill_count: 0
    .symbol:         convx_gconv2d_kernel.kd
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
      - .offset:         64
        .size:           4
        .value_kind:     by_value
      - .offset:         72
        .size:           4
        .value_kind:     hidden_block_count_x
      - .offset:         76
        .size:           4
        .value_kind:     hidden_block_count_y
      - .offset:         80
        .size:           4
        .value_kind:     hidden_block_count_z
      - .offset:         84
        .size:           2
        .value_kind:     hidden_group_size_x
      - .offset:         86
        .size:           2
        .value_kind:     hidden_group_size_y
      - .offset:         88
        .size:           2
        .value_kind:     hidden_group_size_z
      - .offset:         90
        .size:           2
        .value_kind:     hidden_remainder_x
      - .offset:         92
        .size:           2
        .value_kind:     hidden_remainder_y
      - .offset:         94
        .size:           2
        .value_kind:     hidden_remainder_z
      - .offset:         112
        .size:           8
        .value_kind:     hidden_global_offset_x
      - .offset:         120
        .size:           8
        .value_kind:     hidden_global_offset_y
      - .offset:         128
        .size:           8
        .value_kind:     hidden_global_offset_z
      - .offset:         136
        .size:           2
        .value_kind:     hidden_grid_dims
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 328
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           convx_convtranspose2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     28
    .sgpr_spill_count: 0
    .symbol:         convx_convtranspose2d_kernel.kd
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
    .name:           convx_dilation2d_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     22
    .sgpr_spill_count: 0
    .symbol:         convx_dilation2d_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     17
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
