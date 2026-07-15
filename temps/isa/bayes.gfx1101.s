	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	nb_count_table_kernel   ; -- Begin function nb_count_table_kernel
	.globl	nb_count_table_kernel
	.p2align	8
	.type	nb_count_table_kernel,@function
nb_count_table_kernel:                  ; @nb_count_table_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b128 s[4:7], s[0:1], 0x18
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mul_i32 s2, s5, s4
	v_cmp_gt_i32_e32 vcc_lo, s2, v1
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB0_4
; %bb.1:
	s_abs_i32 s2, s5
	v_sub_nc_u32_e32 v3, 0, v1
	v_cvt_f32_u32_e32 v0, s2
	s_sub_i32 s3, 0, s2
	s_load_b128 s[8:11], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_i32_e32 v3, v1, v3
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v0, v0
	v_mul_lo_u32 v2, s3, v0
	s_mov_b32 s3, 0
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v3, 31, v3
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v0, v0, v3
	v_sub_nc_u32_e32 v2, v0, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v3, 31, v2
	v_lshlrev_b64 v[3:4], 2, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s10, v3
	v_add_co_ci_u32_e64 v4, null, s11, v4, vcc_lo
	global_load_b32 v0, v[3:4], off
	s_waitcnt vmcnt(0)
	v_cmp_lt_i32_e32 vcc_lo, -1, v0
	v_cmp_gt_i32_e64 s2, s6, v0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB0_4
; %bb.2:
	v_mul_lo_u32 v2, v2, s5
	s_load_b64 s[0:1], s[0:1], 0x10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v2, v1, v2
	v_mad_u64_u32 v[3:4], null, v0, s5, v[2:3]
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_u32 v0, vcc_lo, s8, v0
	v_lshlrev_b64 v[2:3], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v4, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s1, v3, vcc_lo
	global_load_b64 v[6:7], v[0:1], off
	global_load_b64 v[2:3], v[4:5], off
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	s_waitcnt vmcnt(0)
	v_add_f64 v[0:1], v[2:3], v[6:7]
	global_atomic_cmpswap_b64 v[0:1], v[4:5], v[0:3], off glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s3, vcc_lo, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s3
	s_cbranch_execnz .LBB0_3
.LBB0_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel nb_count_table_kernel
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
.Lfunc_end0:
	.size	nb_count_table_kernel, .Lfunc_end0-nb_count_table_kernel
                                        ; -- End function
	.set nb_count_table_kernel.num_vgpr, 8
	.set nb_count_table_kernel.num_agpr, 0
	.set nb_count_table_kernel.numbered_sgpr, 12
	.set nb_count_table_kernel.num_named_barrier, 0
	.set nb_count_table_kernel.private_seg_size, 0
	.set nb_count_table_kernel.uses_vcc, 1
	.set nb_count_table_kernel.uses_flat_scratch, 0
	.set nb_count_table_kernel.has_dyn_sized_stack, 0
	.set nb_count_table_kernel.has_recursion, 0
	.set nb_count_table_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 496
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
; WaveLimiterHint : 1
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.text
	.protected	nb_feature_log_prob_kernel ; -- Begin function nb_feature_log_prob_kernel
	.globl	nb_feature_log_prob_kernel
	.p2align	8
	.type	nb_feature_log_prob_kernel,@function
nb_feature_log_prob_kernel:             ; @nb_feature_log_prob_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b64 s[4:5], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB1_8
; %bb.1:
	s_clause 0x1
	s_load_b64 s[6:7], s[0:1], 0x18
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mul_lo_u32 v0, v1, s5
	s_cmp_gt_i32 s5, 0
	s_cselect_b32 s4, -1, 0
	s_cmp_lt_i32 s5, 1
	s_delay_alu instid0(VALU_DEP_1)
	v_ashrrev_i32_e32 v1, 31, v0
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[6:7], s[6:7], 0x0
	s_cbranch_scc1 .LBB1_5
; %bb.2:
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[4:5], 3, v[0:1]
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_mov_b32 s8, s5
	v_add_co_u32 v4, vcc_lo, s0, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s1, v5, vcc_lo
.LBB1_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[6:7], v[4:5], off
	v_add_co_u32 v4, vcc_lo, v4, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	s_add_i32 s8, s8, -1
	s_cmp_eq_u32 s8, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[6:7], s[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], v[6:7]
	s_cbranch_scc0 .LBB1_3
; %bb.4:
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccz .LBB1_6
	s_branch .LBB1_8
.LBB1_5:
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0
	s_and_not1_b32 vcc_lo, exec_lo, s4
	s_cbranch_vccnz .LBB1_8
.LBB1_6:
	s_delay_alu instid0(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_mov_b32 s8, 0x55555555
	s_mov_b32 s9, 0x3fe55555
	s_mov_b32 s10, 0x6b47b09a
	s_mov_b32 s12, 0xbf559e2b
	s_mov_b32 s11, 0x3fc38538
	s_mov_b32 s13, 0x3fc3ab76
	s_mov_b32 s14, 0xd7f4df2e
	s_mov_b32 s15, 0x3fc7474d
	s_mov_b32 s16, 0x16291751
	s_mov_b32 s17, 0x3fcc71c0
	s_mov_b32 s18, 0x9b27acf1
	s_mov_b32 s19, 0x3fd24924
	s_mov_b32 s20, 0x998ef7b6
	s_mov_b32 s21, 0x3fd99999
	s_mov_b32 s22, 0x55555780
	s_mov_b32 s23, s9
	s_mov_b32 s24, 0xfefa39ef
	s_mov_b32 s25, 0x3fe62e42
	s_mov_b32 s26, 0x3b39803f
	s_mov_b32 s27, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[8:9], v[4:5]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[12:13], s[10:11]
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[16:17]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[20:21]
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[22:23]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_subrev_co_ci_u32_e64 v14, null, 0, v16, vcc_lo
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_f64 v[10:11], v[12:13], v[8:9]
	v_ldexp_f64 v[4:5], v[4:5], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[16:17], v[14:15], s[24:25]
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[14:15], s[24:25], -v[16:17]
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[14:15], s[26:27], v[12:13]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[6:7], -v[16:17]
	v_add_f64 v[14:15], v[6:7], v[12:13]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[16:17]
	v_add_f64 v[18:19], v[14:15], -v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_add_f64 v[20:21], v[14:15], -v[18:19]
	v_add_f64 v[10:11], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[8:9], v[4:5]
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[10:11], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[12:13], v[6:7]
	v_add_f64 v[12:13], v[12:13], -v[10:11]
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[14:15], v[6:7]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[16:17], -v[14:15]
	v_add_f64 v[4:5], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_lshlrev_b64 v[6:7], 3, v[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[16:17], v[4:5]
	v_dual_cndmask_b32 v4, v4, v2 :: v_dual_cndmask_b32 v5, v5, v3
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0x7ff80000, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s0, v6
	v_add_co_ci_u32_e64 v1, null, s1, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v5, 0xfff00000, v5, vcc_lo
	v_add_co_u32 v2, vcc_lo, s2, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s3, v7, vcc_lo
.LBB1_7:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[6:7], v[0:1], off
	s_add_i32 s5, s5, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	s_cmp_lg_u32 s5, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[6:7], s[6:7], v[6:7]
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_cmp_ngt_f64_e64 s0, 0, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[8:9], v[8:9]
	v_cndmask_b32_e64 v10, 0, 1, vcc_lo
	v_ldexp_f64 v[8:9], v[8:9], v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], 1.0
	v_add_f64 v[16:17], v[8:9], -1.0
	v_rcp_f64_e32 v[12:13], v[10:11]
	v_add_f64 v[18:19], v[10:11], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[16:17], v[12:13]
	v_mul_f64 v[20:21], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[14:15], v[10:11], -v[20:21]
	v_fma_f64 v[8:9], v[14:15], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[20:21], v[8:9]
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	v_add_f64 v[20:21], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	v_add_f64 v[8:9], v[20:21], -v[8:9]
	v_frexp_exp_i32_f64_e32 v20, v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[18:19], v[8:9]
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[14:15], v[8:9]
	v_mul_f64 v[12:13], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[12:13], s[12:13], s[10:11]
	v_mul_f64 v[18:19], v[10:11], v[12:13]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[16:17]
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[20:21]
	v_fma_f64 v[12:13], v[12:13], v[16:17], s[22:23]
	v_ldexp_f64 v[16:17], v[10:11], 1
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[12:13], v[18:19], v[12:13]
	v_subrev_co_ci_u32_e64 v18, null, 0, v20, vcc_lo
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_cmp_class_f64_e64 vcc_lo, v[6:7], 0x204
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cvt_f64_i32_e32 v[18:19], v18
	v_add_f64 v[14:15], v[16:17], v[12:13]
	v_ldexp_f64 v[8:9], v[8:9], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[20:21], v[18:19], s[24:25]
	v_add_f64 v[10:11], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[18:19], s[24:25], -v[20:21]
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[18:19], s[26:27], v[16:17]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[20:21], v[12:13]
	v_add_f64 v[16:17], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[10:11], -v[20:21]
	v_add_f64 v[18:19], v[10:11], v[16:17]
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[22:23], v[18:19], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[24:25], v[18:19], -v[22:23]
	v_add_f64 v[14:15], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[12:13], v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[14:15], v[10:11]
	v_add_f64 v[14:15], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[16:17], v[10:11]
	v_add_f64 v[16:17], v[16:17], -v[14:15]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[18:19], v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[20:21], -v[18:19]
	v_add_f64 v[8:9], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[20:21], v[8:9]
	v_dual_cndmask_b32 v8, v8, v6 :: v_dual_cndmask_b32 v9, v9, v7
	v_cmp_nge_f64_e32 vcc_lo, 0, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v9, 0x7ff80000, v9, s0
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v9, 0xfff00000, v9, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, 8
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[8:9], -v[4:5]
	global_store_b64 v[2:3], v[6:7], off
	v_add_co_u32 v2, vcc_lo, v2, 8
	v_add_co_ci_u32_e64 v3, null, 0, v3, vcc_lo
	s_cbranch_scc1 .LBB1_7
.LBB1_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel nb_feature_log_prob_kernel
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
		.amdhsa_inst_pref_size 17
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
	.size	nb_feature_log_prob_kernel, .Lfunc_end1-nb_feature_log_prob_kernel
                                        ; -- End function
	.set nb_feature_log_prob_kernel.num_vgpr, 26
	.set nb_feature_log_prob_kernel.num_agpr, 0
	.set nb_feature_log_prob_kernel.numbered_sgpr, 28
	.set nb_feature_log_prob_kernel.num_named_barrier, 0
	.set nb_feature_log_prob_kernel.private_seg_size, 0
	.set nb_feature_log_prob_kernel.uses_vcc, 1
	.set nb_feature_log_prob_kernel.uses_flat_scratch, 0
	.set nb_feature_log_prob_kernel.has_dyn_sized_stack, 0
	.set nb_feature_log_prob_kernel.has_recursion, 0
	.set nb_feature_log_prob_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2072
; TotalNumSgprs: 30
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 30
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
	.protected	multinomial_nb_logprob_kernel ; -- Begin function multinomial_nb_logprob_kernel
	.globl	multinomial_nb_logprob_kernel
	.p2align	8
	.type	multinomial_nb_logprob_kernel,@function
multinomial_nb_logprob_kernel:          ; @multinomial_nb_logprob_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s4, s[0:1], 0x3c
	s_load_b128 s[8:11], s[0:1], 0x20
	v_and_b32_e32 v2, 0x3ff, v0
	v_bfe_u32 v3, v0, 10, 10
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s5, s4, 16
	s_and_b32 s4, s4, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[0:1], null, s2, s4, v[2:3]
	v_mad_u64_u32 v[1:2], null, s3, s5, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e32 vcc_lo, s8, v0
	v_cmp_gt_i32_e64 s2, s10, v1
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB2_5
; %bb.1:
	s_load_b256 s[0:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_cmp_lt_i32 s9, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_cbranch_scc1 .LBB2_4
; %bb.2:
	v_mul_lo_u32 v4, v1, s9
	v_mul_lo_u32 v6, v0, s9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v5, 31, v4
	v_ashrrev_i32_e32 v7, 31, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	v_lshlrev_b64 v[6:7], 3, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s2, v4
	v_add_co_ci_u32_e64 v5, null, s3, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s4, v6
	v_add_co_ci_u32_e64 v7, null, s5, v7, vcc_lo
	.p2align	6
.LBB2_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[8:9], v[6:7], off
	global_load_b64 v[10:11], v[4:5], off
	v_add_co_u32 v4, vcc_lo, v4, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, 8
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	s_add_i32 s9, s9, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s9, 0
	s_waitcnt vmcnt(0)
	v_fma_f64 v[2:3], v[8:9], v[10:11], v[2:3]
	s_cbranch_scc0 .LBB2_3
.LBB2_4:
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[4:5], null, v0, s10, v[1:2]
	v_ashrrev_i32_e32 v5, 31, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[4:5]
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB2_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel multinomial_nb_logprob_kernel
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
		.amdhsa_system_sgpr_workgroup_id_y 1
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
	.size	multinomial_nb_logprob_kernel, .Lfunc_end2-multinomial_nb_logprob_kernel
                                        ; -- End function
	.set multinomial_nb_logprob_kernel.num_vgpr, 12
	.set multinomial_nb_logprob_kernel.num_agpr, 0
	.set multinomial_nb_logprob_kernel.numbered_sgpr, 12
	.set multinomial_nb_logprob_kernel.num_named_barrier, 0
	.set multinomial_nb_logprob_kernel.private_seg_size, 0
	.set multinomial_nb_logprob_kernel.uses_vcc, 1
	.set multinomial_nb_logprob_kernel.uses_flat_scratch, 0
	.set multinomial_nb_logprob_kernel.has_dyn_sized_stack, 0
	.set multinomial_nb_logprob_kernel.has_recursion, 0
	.set multinomial_nb_logprob_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 400
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
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 1
	.text
	.protected	bernoulli_nb_logprob_kernel ; -- Begin function bernoulli_nb_logprob_kernel
	.globl	bernoulli_nb_logprob_kernel
	.p2align	8
	.type	bernoulli_nb_logprob_kernel,@function
bernoulli_nb_logprob_kernel:            ; @bernoulli_nb_logprob_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s4, s[0:1], 0x44
	s_load_b128 s[12:15], s[0:1], 0x28
	v_and_b32_e32 v2, 0x3ff, v0
	v_bfe_u32 v3, v0, 10, 10
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s5, s4, 16
	s_and_b32 s4, s4, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[0:1], null, s2, s4, v[2:3]
	v_mad_u64_u32 v[1:2], null, s3, s5, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e32 vcc_lo, s12, v0
	v_cmp_gt_i32_e64 s2, s14, v1
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s3, s2
	s_cbranch_execz .LBB3_5
; %bb.1:
	s_load_b256 s[4:11], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x20
	s_cmp_lt_i32 s13, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v3, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_cbranch_scc1 .LBB3_4
; %bb.2:
	v_mul_lo_u32 v4, v1, s13
	v_mul_lo_u32 v6, v0, s13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ashrrev_i32_e32 v5, 31, v4
	v_ashrrev_i32_e32 v7, 31, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[8:9], 3, v[4:5]
	v_lshlrev_b64 v[10:11], 3, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v4, vcc_lo, s6, v8
	v_add_co_ci_u32_e64 v5, null, s7, v9, vcc_lo
	v_add_co_u32 v6, vcc_lo, s8, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s9, v9, vcc_lo
	v_add_co_u32 v8, vcc_lo, s10, v10
	v_add_co_ci_u32_e64 v9, null, s11, v11, vcc_lo
	.p2align	6
.LBB3_3:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[10:11], v[8:9], off
	global_load_b64 v[12:13], v[6:7], off
	global_load_b64 v[14:15], v[4:5], off
	v_add_co_u32 v4, vcc_lo, v4, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, 0, v5, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, 8
	v_add_co_ci_u32_e64 v7, null, 0, v7, vcc_lo
	v_add_co_u32 v8, vcc_lo, v8, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	s_add_i32 s13, s13, -1
	s_cmp_eq_u32 s13, 0
	s_waitcnt vmcnt(2)
	v_add_f64 v[16:17], -v[10:11], 1.0
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[16:17], v[12:13]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], v[10:11]
	s_cbranch_scc0 .LBB3_3
.LBB3_4:
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[4:5], null, v0, s14, v[1:2]
	v_ashrrev_i32_e32 v5, 31, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[4:5]
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB3_5:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel bernoulli_nb_logprob_kernel
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
		.amdhsa_system_sgpr_workgroup_id_y 1
		.amdhsa_system_sgpr_workgroup_id_z 0
		.amdhsa_system_sgpr_workgroup_info 0
		.amdhsa_system_vgpr_workitem_id 1
		.amdhsa_next_free_vgpr 18
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
.Lfunc_end3:
	.size	bernoulli_nb_logprob_kernel, .Lfunc_end3-bernoulli_nb_logprob_kernel
                                        ; -- End function
	.set bernoulli_nb_logprob_kernel.num_vgpr, 18
	.set bernoulli_nb_logprob_kernel.num_agpr, 0
	.set bernoulli_nb_logprob_kernel.numbered_sgpr, 16
	.set bernoulli_nb_logprob_kernel.num_named_barrier, 0
	.set bernoulli_nb_logprob_kernel.private_seg_size, 0
	.set bernoulli_nb_logprob_kernel.uses_vcc, 1
	.set bernoulli_nb_logprob_kernel.uses_flat_scratch, 0
	.set bernoulli_nb_logprob_kernel.has_dyn_sized_stack, 0
	.set bernoulli_nb_logprob_kernel.has_recursion, 0
	.set bernoulli_nb_logprob_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 488
; TotalNumSgprs: 18
; NumVgprs: 18
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 18
; NumVGPRsForWavesPerEU: 18
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 1
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
	.type	__hip_cuid_e18353f4e7e06ee,@object ; @__hip_cuid_e18353f4e7e06ee
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_e18353f4e7e06ee
__hip_cuid_e18353f4e7e06ee:
	.byte	0                               ; 0x0
	.size	__hip_cuid_e18353f4e7e06ee, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_e18353f4e7e06ee
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
    .name:           nb_count_table_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         nb_count_table_kernel.kd
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
    .name:           nb_feature_log_prob_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     30
    .sgpr_spill_count: 0
    .symbol:         nb_feature_log_prob_kernel.kd
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
    .name:           multinomial_nb_logprob_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         multinomial_nb_logprob_kernel.kd
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
      - .address_space:  global
        .offset:         32
        .size:           8
        .value_kind:     global_buffer
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
    .name:           bernoulli_nb_logprob_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         bernoulli_nb_logprob_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     18
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
