	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	gemm_bt_f64             ; -- Begin function gemm_bt_f64
	.globl	gemm_bt_f64
	.p2align	8
	.type	gemm_bt_f64,@function
gemm_bt_f64:                            ; @gemm_bt_f64
; %bb.0:
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x18
	s_load_b64 s[8:9], s[0:1], 0x8
	s_waitcnt lgkmcnt(0)
	s_bitcmp1_b32 s6, 0
	s_mul_hi_i32 s11, s6, s2
	s_cselect_b32 s3, -1, 0
	s_mul_i32 s10, s6, s2
	s_and_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccz .LBB0_5
; %bb.1:
	s_mov_b32 s3, exec_lo
	v_cmpx_gt_i32_e64 s6, v0
	s_cbranch_execz .LBB0_4
; %bb.2:
	s_load_b32 s14, s[0:1], 0x34
	s_lshl_b64 s[12:13], s[10:11], 3
	v_lshl_add_u32 v3, v0, 3, 0
	s_add_u32 s7, s8, s12
	v_mov_b32_e32 v1, v0
	s_addc_u32 s12, s9, s13
	s_waitcnt lgkmcnt(0)
	s_and_b32 s13, s14, 0xffff
	s_mov_b32 s14, 0
	s_lshl_b32 s15, s13, 3
	.p2align	6
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 3, v[1:2]
	v_add_nc_u32_e32 v1, s13, v1
	v_add_co_u32 v4, vcc_lo, s7, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v5, null, s12, v5, vcc_lo
	v_cmp_le_i32_e32 vcc_lo, s6, v1
	global_load_b64 v[4:5], v[4:5], off
	s_or_b32 s14, vcc_lo, s14
	s_waitcnt vmcnt(0)
	ds_store_b64 v3, v[4:5]
	v_add_nc_u32_e32 v3, s15, v3
	s_and_not1_b32 exec_lo, exec_lo, s14
	s_cbranch_execnz .LBB0_3
.LBB0_4:
	s_or_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB0_6
	s_branch .LBB0_10
.LBB0_5:
.LBB0_6:
	s_ashr_i32 s7, s6, 1
	s_mov_b32 s12, exec_lo
	v_cmpx_gt_i32_e64 s7, v0
	s_cbranch_execz .LBB0_9
; %bb.7:
	s_load_b32 s3, s[0:1], 0x34
	v_dual_mov_b32 v4, v0 :: v_dual_lshlrev_b32 v1, 4, v0
	s_lshl_b64 s[14:15], s[10:11], 3
	s_delay_alu instid0(VALU_DEP_1)
	v_add_nc_u32_e32 v3, 0, v1
	s_waitcnt lgkmcnt(0)
	s_and_b32 s10, s3, 0xffff
	s_add_u32 s3, s8, s14
	s_addc_u32 s8, s9, s15
	v_add_co_u32 v1, s3, s3, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v2, null, s8, 0, s3
	s_mov_b32 s8, 0
	s_lshl_b32 s9, s10, 4
.LBB0_8:                                ; =>This Inner Loop Header: Depth=1
	global_load_b128 v[5:8], v[1:2], off
	v_add_nc_u32_e32 v4, s10, v4
	v_add_co_u32 v1, vcc_lo, v1, s9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v2, null, 0, v2, vcc_lo
	v_cmp_le_i32_e64 s3, s7, v4
	s_or_b32 s8, s3, s8
	s_waitcnt vmcnt(0)
	ds_store_b128 v3, v[5:8]
	v_add_nc_u32_e32 v3, s9, v3
	s_and_not1_b32 exec_lo, exec_lo, s8
	s_cbranch_execnz .LBB0_8
.LBB0_9:
	s_or_b32 exec_lo, exec_lo, s12
.LBB0_10:
	v_lshrrev_b32_e32 v6, 5, v0
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_mov_b32 s3, exec_lo
	v_cmpx_gt_i32_e64 s4, v6
	s_cbranch_execz .LBB0_19
; %bb.11:
	v_mbcnt_lo_u32_b32 v2, -1, 0
	s_clause 0x2
	s_load_b32 s14, s[0:1], 0x34
	s_load_b64 s[8:9], s[0:1], 0x0
	s_load_b64 s[10:11], s[0:1], 0x10
	v_lshrrev_b32_e32 v3, 5, v0
	v_and_b32_e32 v7, 31, v0
	s_ashr_i32 s3, s2, 31
	v_cmp_gt_u32_e64 s1, 24, v2
	s_lshl_b64 s[12:13], s[2:3], 3
	v_mad_i64_i32 v[0:1], null, v3, s6, 0
	v_lshlrev_b32_e32 v3, 3, v7
	v_cndmask_b32_e64 v4, 0, 8, s1
	v_cmp_gt_u32_e64 s1, 28, v2
	v_lshl_or_b32 v8, v2, 2, 64
	v_cmp_eq_u32_e64 s0, 0, v7
	v_add_nc_u32_e32 v13, 0, v3
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	v_cndmask_b32_e64 v5, 0, 4, s1
	v_cmp_gt_u32_e64 s1, 30, v2
	v_add_lshl_u32 v9, v4, v2, 2
	s_mov_b32 s7, 0
	v_cmp_gt_i32_e32 vcc_lo, s6, v7
	v_add_lshl_u32 v10, v5, v2, 2
	v_cndmask_b32_e64 v11, 0, 2, s1
	s_waitcnt lgkmcnt(0)
	s_and_b32 s1, s14, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s1, s1, 31
	s_lshr_b32 s3, s1, 5
	v_cmp_ne_u32_e64 s1, 31, v2
	v_add_lshl_u32 v11, v11, v2, 2
	s_add_u32 s10, s10, s12
	s_addc_u32 s11, s11, s13
	s_ashr_i32 s12, s5, 31
	v_add_co_ci_u32_e64 v2, null, 0, v2, s1
	v_add_co_u32 v0, s1, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, 0, v1, s1
	v_lshlrev_b32_e32 v12, 2, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, s1, s8, v0
	v_add_co_ci_u32_e64 v1, null, s9, v1, s1
	s_mul_hi_i32 s9, s3, s6
	s_mul_i32 s8, s3, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b64 s[8:9], s[8:9], 3
	s_branch .LBB0_13
.LBB0_12:                               ;   in Loop: Header=BB0_13 Depth=1
	s_or_b32 exec_lo, exec_lo, s2
	v_add_nc_u32_e32 v6, s3, v6
	v_add_co_u32 v0, s2, v0, s8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s9, v1, s2
	v_cmp_le_i32_e64 s1, s4, v6
	s_or_b32 s7, s1, s7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s7
	s_cbranch_execz .LBB0_19
.LBB0_13:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB0_15 Depth 2
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_and_saveexec_b32 s13, vcc_lo
	s_cbranch_execz .LBB0_17
; %bb.14:                               ;   in Loop: Header=BB0_13 Depth=1
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v5, v1
	v_dual_mov_b32 v3, 0 :: v_dual_mov_b32 v14, v13
	v_dual_mov_b32 v4, v0 :: v_dual_mov_b32 v15, v7
	s_mov_b32 s14, 0
	.p2align	6
.LBB0_15:                               ;   Parent Loop BB0_13 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[16:17], v[4:5], off
	ds_load_b64 v[18:19], v14
	v_add_nc_u32_e32 v15, 32, v15
	v_add_co_u32 v4, s1, 0x100, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v5, null, 0, v5, s1
	v_cmp_le_i32_e64 s2, s6, v15
	v_add_nc_u32_e32 v14, 0x100, v14
	s_or_b32 s14, s2, s14
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[2:3], v[16:17], v[18:19], v[2:3]
	s_and_not1_b32 exec_lo, exec_lo, s14
	s_cbranch_execnz .LBB0_15
; %bb.16:                               ;   in Loop: Header=BB0_13 Depth=1
	s_or_b32 exec_lo, exec_lo, s14
.LBB0_17:                               ;   in Loop: Header=BB0_13 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s13
	s_waitcnt lgkmcnt(1)
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(1)
	ds_bpermute_b32 v5, v8, v3
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	ds_bpermute_b32 v4, v9, v2
	ds_bpermute_b32 v5, v9, v3
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	ds_bpermute_b32 v4, v10, v2
	ds_bpermute_b32 v5, v10, v3
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	ds_bpermute_b32 v4, v11, v2
	ds_bpermute_b32 v5, v11, v3
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	ds_bpermute_b32 v4, v12, v2
	ds_bpermute_b32 v5, v12, v3
	s_and_saveexec_b32 s2, s0
	s_cbranch_execz .LBB0_12
; %bb.18:                               ;   in Loop: Header=BB0_13 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[4:5]
	v_mad_u64_u32 v[14:15], null, v6, s5, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v4, v15
	v_mad_u64_u32 v[15:16], null, v6, s12, v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[4:5], 3, v[14:15]
	v_add_co_u32 v4, s1, s10, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s11, v5, s1
	global_store_b64 v[4:5], v[2:3], off
	s_branch .LBB0_12
.LBB0_19:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gemm_bt_f64
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
		.amdhsa_next_free_vgpr 20
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
.Lfunc_end0:
	.size	gemm_bt_f64, .Lfunc_end0-gemm_bt_f64
                                        ; -- End function
	.set gemm_bt_f64.num_vgpr, 20
	.set gemm_bt_f64.num_agpr, 0
	.set gemm_bt_f64.numbered_sgpr, 16
	.set gemm_bt_f64.num_named_barrier, 0
	.set gemm_bt_f64.private_seg_size, 0
	.set gemm_bt_f64.uses_vcc, 1
	.set gemm_bt_f64.uses_flat_scratch, 0
	.set gemm_bt_f64.has_dyn_sized_stack, 0
	.set gemm_bt_f64.has_recursion, 0
	.set gemm_bt_f64.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1124
; TotalNumSgprs: 18
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 18
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
	.protected	scale_f64               ; -- Begin function scale_f64
	.globl	scale_f64
	.p2align	8
	.type	scale_f64,@function
scale_f64:                              ; @scale_f64
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
	v_lshlrev_b64 v[0:1], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_load_b64 s[0:1], s[2:3], 0x0
	global_load_b64 v[2:3], v[0:1], off
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_mul_f64 v[2:3], s[0:1], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel scale_f64
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
.Lfunc_end1:
	.size	scale_f64, .Lfunc_end1-scale_f64
                                        ; -- End function
	.set scale_f64.num_vgpr, 4
	.set scale_f64.num_agpr, 0
	.set scale_f64.numbered_sgpr, 6
	.set scale_f64.num_named_barrier, 0
	.set scale_f64.private_seg_size, 0
	.set scale_f64.uses_vcc, 1
	.set scale_f64.uses_flat_scratch, 0
	.set scale_f64.has_dyn_sized_stack, 0
	.set scale_f64.has_recursion, 0
	.set scale_f64.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 144
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_78a634b71272fcb7,@object ; @__hip_cuid_78a634b71272fcb7
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_78a634b71272fcb7
__hip_cuid_78a634b71272fcb7:
	.byte	0                               ; 0x0
	.size	__hip_cuid_78a634b71272fcb7, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_78a634b71272fcb7
	.amdgpu_metadata
---
amdhsa.kernels:
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
      - .offset:         160
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 296
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           gemm_bt_f64
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         gemm_bt_f64.kd
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
    .name:           scale_f64
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         scale_f64.kd
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
