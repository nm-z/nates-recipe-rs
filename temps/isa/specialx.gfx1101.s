	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	specialx_digamma_kernel ; -- Begin function specialx_digamma_kernel
	.globl	specialx_digamma_kernel
	.p2align	8
	.type	specialx_digamma_kernel,@function
specialx_digamma_kernel:                ; @specialx_digamma_kernel
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
	s_cbranch_execz .LBB0_6
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_gt_f64_e32 0x40180000, v[2:3]
	s_cbranch_execz .LBB0_5
; %bb.2:
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0
	s_mov_b32 s1, 0
	.p2align	6
.LBB0_3:                                ; =>This Inner Loop Header: Depth=1
	v_div_scale_f64 v[6:7], null, v[2:3], v[2:3], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[2:3], 1.0
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
	v_div_fixup_f64 v[6:7], v[6:7], v[2:3], 1.0
	v_add_f64 v[2:3], v[2:3], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_cmp_ngt_f64_e32 vcc_lo, 0x40180000, v[2:3]
	s_or_b32 s1, vcc_lo, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execnz .LBB0_3
; %bb.4:
	s_or_b32 exec_lo, exec_lo, s1
.LBB0_5:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s0
	v_mul_f64 v[6:7], v[2:3], v[2:3]
	s_mov_b32 s4, 0xf07c1f08
	s_mov_b32 s1, 0x3f711111
	s_mov_b32 s0, 0x11111111
	s_mov_b32 s5, 0xbf7f07c1
	s_mov_b32 s6, 0xbf559e2b
	s_mov_b32 s7, 0x3fc3ab76
	v_div_scale_f64 v[28:29], null, v[2:3], v[2:3], -0.5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[8:9], null, v[6:7], v[6:7], 1.0
	v_div_scale_f64 v[14:15], vcc_lo, 1.0, v[6:7], 1.0
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[8:9], v[12:13], v[14:15]
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[12:13]
	v_frexp_mant_f64_e32 v[10:11], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[6:7], v[8:9], v[6:7], 1.0
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s4, 0x10410410
	s_mov_b32 s5, 0x3f704104
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x6b47b09a
	s_mov_b32 s5, 0x3fc38538
	v_fma_f64 v[8:9], -v[6:7], v[8:9], s[0:1]
	s_mov_b32 s1, 0x3fb55555
	s_mov_b32 s0, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], -v[6:7], v[8:9], s[0:1]
	s_mov_b32 s1, 0x3fe55555
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[10:11]
	s_mov_b32 s0, 0x55555780
	v_cndmask_b32_e64 v12, 0, 1, vcc_lo
	v_ldexp_f64 v[10:11], v[10:11], v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[18:19], v[10:11], -1.0
	v_rcp_f64_e32 v[14:15], v[12:13]
	v_add_f64 v[20:21], v[12:13], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[12:13], v[14:15], 1.0
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[18:19], v[14:15]
	v_mul_f64 v[22:23], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[12:13], -v[22:23]
	v_fma_f64 v[10:11], v[16:17], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[22:23], v[10:11]
	v_add_f64 v[20:21], v[18:19], -v[12:13]
	v_add_f64 v[22:23], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	v_frexp_exp_i32_f64_e32 v22, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[20:21], v[10:11]
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], v[10:11]
	v_mul_f64 v[14:15], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], s[6:7], s[4:5]
	s_mov_b32 s4, 0xd7f4df2e
	s_mov_b32 s5, 0x3fc7474d
	v_mul_f64 v[20:21], v[12:13], v[14:15]
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[4:5]
	s_mov_b32 s4, 0x16291751
	s_mov_b32 s5, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[4:5]
	s_mov_b32 s4, 0x9b27acf1
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[4:5]
	s_mov_b32 s4, 0x998ef7b6
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[14:15], v[18:19], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[14:15], v[18:19], s[0:1]
	v_ldexp_f64 v[18:19], v[12:13], 1
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0x3fe62e42
	v_mul_f64 v[14:15], v[20:21], v[14:15]
	v_subrev_co_ci_u32_e64 v20, null, 0, v22, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_cvt_f64_i32_e32 v[20:21], v20
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[18:19], v[14:15]
	v_ldexp_f64 v[10:11], v[10:11], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[22:23], v[20:21], s[0:1]
	v_add_f64 v[12:13], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[20:21], s[0:1], -v[22:23]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[20:21], s[0:1], v[18:19]
	v_cmp_class_f64_e64 s0, v[2:3], 0x204
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[22:23], v[14:15]
	v_add_f64 v[18:19], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[12:13], -v[22:23]
	v_add_f64 v[20:21], v[12:13], v[18:19]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_f64 v[24:25], v[20:21], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[26:27], v[20:21], -v[24:25]
	v_add_f64 v[16:17], v[18:19], -v[24:25]
	v_rcp_f64_e32 v[18:19], v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[14:15], v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[22:23], -v[14:15]
	v_add_f64 v[12:13], v[16:17], v[12:13]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[28:29], v[18:19], 1.0
	v_add_f64 v[10:11], v[10:11], -v[24:25]
	v_add_f64 v[12:13], v[22:23], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[18:19], v[16:17], v[18:19]
	v_add_f64 v[18:19], v[22:23], -v[24:25]
	v_add_f64 v[22:23], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[26:27], -v[28:29], v[16:17], 1.0
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[22:23], -v[20:21]
	v_div_scale_f64 v[20:21], vcc_lo, -0.5, v[2:3], -0.5
	v_fma_f64 v[16:17], v[16:17], v[26:27], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[14:15], v[20:21], v[16:17]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], -v[28:29], v[14:15], v[20:21]
	v_add_f64 v[10:11], v[22:23], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[12:13], v[12:13], v[16:17], v[14:15]
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v11, v11, v3, s0
	v_cndmask_b32_e64 v10, v10, v2, s0
	v_div_fixup_f64 v[2:3], v[12:13], v[2:3], -0.5
	v_add_f64 v[4:5], v[4:5], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[2:3], v[4:5], v[2:3]
	v_fma_f64 v[2:3], -v[6:7], v[8:9], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB0_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_digamma_kernel
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
.Lfunc_end0:
	.size	specialx_digamma_kernel, .Lfunc_end0-specialx_digamma_kernel
                                        ; -- End function
	.set specialx_digamma_kernel.num_vgpr, 30
	.set specialx_digamma_kernel.num_agpr, 0
	.set specialx_digamma_kernel.numbered_sgpr, 8
	.set specialx_digamma_kernel.num_named_barrier, 0
	.set specialx_digamma_kernel.private_seg_size, 0
	.set specialx_digamma_kernel.uses_vcc, 1
	.set specialx_digamma_kernel.uses_flat_scratch, 0
	.set specialx_digamma_kernel.has_dyn_sized_stack, 0
	.set specialx_digamma_kernel.has_recursion, 0
	.set specialx_digamma_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1576
; TotalNumSgprs: 10
; NumVgprs: 30
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 10
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
	.text
	.protected	specialx_expit_kernel   ; -- Begin function specialx_expit_kernel
	.globl	specialx_expit_kernel
	.p2align	8
	.type	specialx_expit_kernel,@function
specialx_expit_kernel:                  ; @specialx_expit_kernel
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
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
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
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
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
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[4:5], 1.0
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
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_expit_kernel
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
		.amdhsa_next_free_vgpr 14
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
	.size	specialx_expit_kernel, .Lfunc_end1-specialx_expit_kernel
                                        ; -- End function
	.set specialx_expit_kernel.num_vgpr, 14
	.set specialx_expit_kernel.num_agpr, 0
	.set specialx_expit_kernel.numbered_sgpr, 6
	.set specialx_expit_kernel.num_named_barrier, 0
	.set specialx_expit_kernel.private_seg_size, 0
	.set specialx_expit_kernel.uses_vcc, 1
	.set specialx_expit_kernel.uses_flat_scratch, 0
	.set specialx_expit_kernel.has_dyn_sized_stack, 0
	.set specialx_expit_kernel.has_recursion, 0
	.set specialx_expit_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 696
; TotalNumSgprs: 8
; NumVgprs: 14
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 8
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
	.protected	specialx_logit_kernel   ; -- Begin function specialx_logit_kernel
	.globl	specialx_logit_kernel
	.p2align	8
	.type	specialx_logit_kernel,@function
specialx_logit_kernel:                  ; @specialx_logit_kernel
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
	v_add_f64 v[4:5], -v[2:3], 1.0
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[2:3], v[6:7], v[4:5], v[2:3]
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[0:1], v[4:5]
	s_mov_b32 s0, 0x55555780
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
	v_subrev_co_ci_u32_e64 v14, null, 0, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_mul_f64 v[16:17], v[14:15], s[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[0:1], -v[16:17]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_fma_f64 v[8:9], v[14:15], s[0:1], v[12:13]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v4, v4, v2 :: v_dual_cndmask_b32 v5, v5, v3
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v5, 0x7ff80000, v5, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v5, 0xfff00000, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_logit_kernel
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
		.amdhsa_next_free_vgpr 22
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
.Lfunc_end2:
	.size	specialx_logit_kernel, .Lfunc_end2-specialx_logit_kernel
                                        ; -- End function
	.set specialx_logit_kernel.num_vgpr, 22
	.set specialx_logit_kernel.num_agpr, 0
	.set specialx_logit_kernel.numbered_sgpr, 8
	.set specialx_logit_kernel.num_named_barrier, 0
	.set specialx_logit_kernel.private_seg_size, 0
	.set specialx_logit_kernel.uses_vcc, 1
	.set specialx_logit_kernel.uses_flat_scratch, 0
	.set specialx_logit_kernel.has_dyn_sized_stack, 0
	.set specialx_logit_kernel.has_recursion, 0
	.set specialx_logit_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1176
; TotalNumSgprs: 10
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 22
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
	.protected	specialx_sinc_kernel    ; -- Begin function specialx_sinc_kernel
	.globl	specialx_sinc_kernel
	.p2align	8
	.type	specialx_sinc_kernel,@function
specialx_sinc_kernel:                   ; @specialx_sinc_kernel
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
	s_cbranch_execz .LBB3_4
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0x3ff00000
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_neq_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB3_3
; %bb.2:
	v_mul_f64 v[4:5], |v[2:3]|, 0.5
	s_mov_b32 s4, 0x6fdffd2b
	s_mov_b32 s6, 0xf99eb0bb
	s_mov_b32 s8, 0xca1d4f33
	s_mov_b32 s10, 0x2e21c33
	s_mov_b32 s5, 0xbf7e2fe7
	s_mov_b32 s7, 0x3f3e357e
	s_mov_b32 s9, 0x3f5f9c89
	s_mov_b32 s11, 0xbf1b1673
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fract_f64_e32 v[6:7], v[4:5]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[4:5]|
	v_and_b32_e32 v4, 0x7fffffff, v3
	v_add_f64 v[6:7], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v6, 0, v6 :: v_dual_cndmask_b32 v5, 0, v7
	v_cmp_gt_f64_e64 vcc_lo, |v[2:3]|, 1.0
	v_dual_cndmask_b32 v5, v4, v5 :: v_dual_cndmask_b32 v4, v2, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[4:5], v[4:5]
	v_rndne_f64_e32 v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[6:7], -0.5, v[4:5]
	v_mul_f64 v[8:9], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], s[6:7], s[4:5]
	v_fma_f64 v[12:13], v[8:9], s[10:11], s[8:9]
	s_mov_b32 s4, 0xd5f14825
	s_mov_b32 s6, 0x7294bff9
	s_mov_b32 s5, 0x3fb50782
	s_mov_b32 s7, 0xbf9a6d1e
	v_mul_f64 v[14:15], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s4, 0xcdfe9424
	s_mov_b32 s6, 0x67b90b37
	s_mov_b32 s5, 0xbfe32d2c
	s_mov_b32 s7, 0x3fce1f50
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s4, 0x67754fff
	s_mov_b32 s6, 0x7e3c325b
	s_mov_b32 s5, 0x400466bc
	s_mov_b32 s7, 0xbff55d3c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s4, 0xe625be09
	s_mov_b32 s6, 0x81b5a67
	s_mov_b32 s5, 0xc014abbc
	s_mov_b32 s7, 0x40103c1f
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[4:5]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s4, 0xc9be45de
	s_mov_b32 s5, 0xc013bd3c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	v_cvt_i32_f64_e32 v14, v[6:7]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x400921fb
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[4:5], s[4:5], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[6:7], v[8:9], v[12:13], 1.0
	v_and_b32_e32 v8, 1, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v8
	v_mul_f64 v[8:9], v[2:3], s[4:5]
	v_lshlrev_b32_e32 v2, 30, v14
	v_xor_b32_e32 v2, v2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v3, v7, v5 :: v_dual_and_b32 v2, 0x80000000, v2
	v_cndmask_b32_e32 v4, v6, v4, vcc_lo
	v_xor_b32_e32 v3, v3, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v2, 0, v4, s0
	v_cndmask_b32_e64 v3, 0x7ff80000, v3, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[4:5], null, v[8:9], v[8:9], v[2:3]
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[10:11], v[6:7]
	v_fma_f64 v[10:11], -v[4:5], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[10:11], v[6:7]
	v_div_scale_f64 v[10:11], vcc_lo, v[2:3], v[8:9], v[2:3]
	v_mul_f64 v[12:13], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[12:13], v[10:11]
	v_div_fmas_f64 v[4:5], v[4:5], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[4:5], v[8:9], v[2:3]
.LBB3_3:
	s_or_b32 exec_lo, exec_lo, s1
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB3_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_sinc_kernel
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
.Lfunc_end3:
	.size	specialx_sinc_kernel, .Lfunc_end3-specialx_sinc_kernel
                                        ; -- End function
	.set specialx_sinc_kernel.num_vgpr, 16
	.set specialx_sinc_kernel.num_agpr, 0
	.set specialx_sinc_kernel.numbered_sgpr, 12
	.set specialx_sinc_kernel.num_named_barrier, 0
	.set specialx_sinc_kernel.private_seg_size, 0
	.set specialx_sinc_kernel.uses_vcc, 1
	.set specialx_sinc_kernel.uses_flat_scratch, 0
	.set specialx_sinc_kernel.has_dyn_sized_stack, 0
	.set specialx_sinc_kernel.has_recursion, 0
	.set specialx_sinc_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 864
; TotalNumSgprs: 14
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
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
	.protected	specialx_entr_kernel    ; -- Begin function specialx_entr_kernel
	.globl	specialx_entr_kernel
	.p2align	8
	.type	specialx_entr_kernel,@function
specialx_entr_kernel:                   ; @specialx_entr_kernel
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
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_nlt_f64_e32 0, v[2:3]
	s_xor_b32 s0, exec_lo, s0
	s_cbranch_execz .LBB4_3
; %bb.2:
	v_cmp_eq_f64_e32 vcc_lo, 0, v[2:3]
	v_mov_b32_e32 v4, 0
                                        ; implicit-def: $vgpr2_vgpr3
	v_cndmask_b32_e64 v5, 0xfff00000, 0, vcc_lo
.LBB4_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB4_5
; %bb.4:
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_mov_b32 s5, 0x3fe55555
	s_mov_b32 s4, 0x55555555
	s_mov_b32 s6, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s7, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[4:5]
	s_mov_b32 s4, 0x55555780
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
	v_fma_f64 v[12:13], v[8:9], s[8:9], s[6:7]
	s_mov_b32 s6, 0xd7f4df2e
	s_mov_b32 s7, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s6, 0x16291751
	s_mov_b32 s7, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s6, 0x9b27acf1
	s_mov_b32 s7, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s6, 0x998ef7b6
	s_mov_b32 s7, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[4:5]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0x3fe62e42
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_subrev_co_ci_u32_e64 v14, null, 0, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[2:3]
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_mul_f64 v[16:17], v[14:15], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[4:5], -v[16:17]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_fma_f64 v[8:9], v[14:15], s[4:5], v[12:13]
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
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], -v[2:3]
.LBB4_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB4_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_entr_kernel
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
		.amdhsa_next_free_vgpr 22
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
	.size	specialx_entr_kernel, .Lfunc_end4-specialx_entr_kernel
                                        ; -- End function
	.set specialx_entr_kernel.num_vgpr, 22
	.set specialx_entr_kernel.num_agpr, 0
	.set specialx_entr_kernel.numbered_sgpr, 10
	.set specialx_entr_kernel.num_named_barrier, 0
	.set specialx_entr_kernel.private_seg_size, 0
	.set specialx_entr_kernel.uses_vcc, 1
	.set specialx_entr_kernel.uses_flat_scratch, 0
	.set specialx_entr_kernel.has_dyn_sized_stack, 0
	.set specialx_entr_kernel.has_recursion, 0
	.set specialx_entr_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1088
; TotalNumSgprs: 12
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 22
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
	.protected	specialx_erfinv_kernel  ; -- Begin function specialx_erfinv_kernel
	.globl	specialx_erfinv_kernel
	.p2align	8
	.type	specialx_erfinv_kernel,@function
specialx_erfinv_kernel:                 ; @specialx_erfinv_kernel
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
	s_cbranch_execz .LBB5_21
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s1, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_ngt_f64_e64 0x3fd80000, |v[2:3]|
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB5_18
; %bb.2:
	v_cmp_ngt_f64_e64 s0, 0x3fefffe0, |v[2:3]|
                                        ; implicit-def: $vgpr4_vgpr5
	s_and_saveexec_b32 s4, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s4
	s_cbranch_execz .LBB5_8
; %bb.3:
	v_add_f64 v[4:5], -|v[2:3]|, 1.0
	s_mov_b32 s5, 0x3fe55555
	s_mov_b32 s4, 0x55555555
	s_mov_b32 s6, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s7, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[6:7], v[4:5]
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[6:7]
	s_mov_b32 s4, 0x55555780
	v_cndmask_b32_e64 v8, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v8
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_frexp_exp_i32_f64_e32 v18, v[4:5]
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[6:7], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[10:11], v[6:7]
	v_add_f64 v[8:9], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	v_fma_f64 v[14:15], v[10:11], s[8:9], s[6:7]
	s_mov_b32 s6, 0xd7f4df2e
	s_mov_b32 s7, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x16291751
	s_mov_b32 s7, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x9b27acf1
	s_mov_b32 s7, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x998ef7b6
	s_mov_b32 s7, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[4:5]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_subrev_co_ci_u32_e64 v16, null, 0, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x204
	v_cvt_f64_i32_e32 v[16:17], v16
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_mul_f64 v[18:19], v[16:17], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[4:5], -v[18:19]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[4:5], v[14:15]
	s_mov_b32 s4, 0xffe00000
	s_mov_b32 s5, 0x3fefffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_nlt_f64_e64 s4, |v[2:3]|, s[4:5]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[18:19], v[10:11]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_cndmask_b32_e32 v6, v6, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, -v7, -v5, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v7, 0xfff80000, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[6:7]
	v_cndmask_b32_e64 v4, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[6:7], v4
	v_rsq_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[8:9], v[4:5], v[6:7]
	v_mul_f64 v[6:7], v[6:7], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 0.5
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[6:7], v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[8:9], v[8:9], v[4:5]
	v_fma_f64 v[8:9], v[10:11], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[8:9], v[8:9], v[4:5]
	v_fma_f64 v[6:7], v[10:11], v[6:7], v[8:9]
	v_cndmask_b32_e64 v8, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x260
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v8
	v_dual_cndmask_b32 v5, v7, v5 :: v_dual_cndmask_b32 v4, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[4:5], 1.0
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
                                        ; implicit-def: $vgpr8_vgpr9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_div_fixup_f64 v[6:7], v[6:7], v[4:5], 1.0
	s_and_saveexec_b32 s5, s4
	s_xor_b32 s4, exec_lo, s5
	s_cbranch_execz .LBB5_5
; %bb.4:
	s_mov_b32 s6, 0xd25bee8d
	s_mov_b32 s8, 0x2cc8e58a
	s_mov_b32 s7, 0xc07dd260
	s_mov_b32 s9, 0x406e1f46
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[6:7]
	s_mov_b32 s6, 0xb6c206e6
	s_mov_b32 s7, 0x407af7da
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x5a0f5809
	s_mov_b32 s7, 0xc06d97c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xbf45d30
	s_mov_b32 s7, 0x405632c2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x8179a727
	s_mov_b32 s7, 0xc038e490
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xa73a2c3c
	s_mov_b32 s7, 0x40189538
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x69b3607d
	s_mov_b32 s7, 0xbffaad85
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xec4b54cb
	s_mov_b32 s7, 0xbf980d1b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x6f90ea2c
	s_mov_b32 s7, 0x3ff00100
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
                                        ; implicit-def: $vgpr6_vgpr7
.LBB5_5:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB5_7
; %bb.6:
	s_mov_b32 s6, 0x5b757c26
	s_mov_b32 s8, 0x31a51669
	s_mov_b32 s7, 0xc0866af4
	s_mov_b32 s9, 0x406c4bd8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[6:7]
	s_mov_b32 s6, 0x93ee1671
	s_mov_b32 s7, 0x409061b2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xfd7248e9
	s_mov_b32 s7, 0xc08d4aa0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x88748d
	s_mov_b32 s7, 0x4081eebb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x6c165efe
	s_mov_b32 s7, 0xc06ff4cb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x9a609255
	s_mov_b32 s7, 0x40559c37
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x677680c6
	s_mov_b32 s7, 0xc03762b2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x32cf7c5a
	s_mov_b32 s7, 0x40176261
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xc231a949
	s_mov_b32 s7, 0xbffa298c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x429b22ca
	s_mov_b32 s7, 0xbf99fa2d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xc4b15d15
	s_mov_b32 s7, 0x3ff00131
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
.LBB5_7:
	s_or_b32 exec_lo, exec_lo, s4
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], v[8:9]
.LBB5_8:
	s_and_not1_saveexec_b32 s4, s0
	s_cbranch_execz .LBB5_22
; %bb.9:
	v_fma_f64 v[4:5], -|v[2:3]|, |v[2:3]|, 1.0
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[6:7], v[4:5]
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[6:7]
	s_mov_b32 s6, 0x55555780
	v_cndmask_b32_e64 v8, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v8
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_frexp_exp_i32_f64_e32 v18, v[4:5]
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[6:7], v[16:17], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[10:11], v[6:7]
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[6:7]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_subrev_co_ci_u32_e64 v16, null, 0, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x204
	v_cvt_f64_i32_e32 v[16:17], v16
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_mul_f64 v[18:19], v[16:17], s[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[6:7], -v[18:19]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[6:7], v[14:15]
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
	v_dual_cndmask_b32 v6, v6, v4 :: v_dual_cndmask_b32 v7, v7, v5
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v7, 0x7ff80000, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[4:5]
                                        ; implicit-def: $vgpr4_vgpr5
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_nlt_f64_e32 0xc0190000, v[6:7]
	s_xor_b32 s5, exec_lo, s0
	s_cbranch_execz .LBB5_15
; %bb.10:
	v_cmp_lt_f64_e32 vcc_lo, 0x90000000, v[6:7]
	v_cmp_nlt_f64_e64 s0, 0xc0300000, v[6:7]
	v_cndmask_b32_e64 v4, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], -v[6:7], v4
	v_rsq_f64_e32 v[8:9], v[4:5]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[10:11], v[4:5], v[8:9]
	v_mul_f64 v[8:9], v[8:9], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 0.5
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[10:11], v[10:11], v[4:5]
	v_fma_f64 v[10:11], v[12:13], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[10:11], v[10:11], v[4:5]
	v_fma_f64 v[8:9], v[12:13], v[8:9], v[10:11]
	v_cndmask_b32_e64 v10, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x260
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v10
	v_dual_cndmask_b32 v7, v9, v5 :: v_dual_cndmask_b32 v6, v8, v4
                                        ; implicit-def: $vgpr4_vgpr5
	s_and_saveexec_b32 s6, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s6
	s_cbranch_execz .LBB5_12
; %bb.11:
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[4:5], 0xc0140000, v[6:7]
	s_mov_b32 s6, 0xc0e38727
	s_mov_b32 s8, 0xa7785389
	s_mov_b32 s7, 0xbdf18fee
	s_mov_b32 s9, 0xbdbdcec3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], s[8:9], s[6:7]
	s_mov_b32 s6, 0x2dda45e3
	s_mov_b32 s7, 0x3e19e6bf
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xb24e2f5f
	s_mov_b32 s7, 0xbe30468f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xa8fba182
	s_mov_b32 s7, 0x3e405ac6
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x495fb9c0
	s_mov_b32 s7, 0xbe50102e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xe1334af8
	s_mov_b32 s7, 0x3e5f4c20
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xfdf9c3e
	s_mov_b32 s7, 0xbe722d22
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xb824cb54
	s_mov_b32 s7, 0x3e8ebc8b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xea372cc
	s_mov_b32 s7, 0xbeb0a8d4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x9d093d2b
	s_mov_b32 s7, 0x3ed2fbd2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x7e1e0fac
	s_mov_b32 s7, 0xbef4a349
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xeb00938f
	s_mov_b32 s7, 0x3f13ebf4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xa8fc5d53
	s_mov_b32 s7, 0xbf2c2f36
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xdf04047c
	s_mov_b32 s7, 0xbf222ea5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xd1fba0dc
	s_mov_b32 s7, 0x3ff02a30
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xdd1ad7fb
	s_mov_b32 s7, 0x4013664d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[4:5], v[6:7], s[6:7]
                                        ; implicit-def: $vgpr6_vgpr7
.LBB5_12:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB5_14
; %bb.13:
	v_add_f64 v[4:5], 0xc00a0000, v[6:7]
	s_mov_b32 s6, 0x52878635
	s_mov_b32 s8, 0x87dbd932
	s_mov_b32 s7, 0x3e785cbe
	s_mov_b32 s9, 0x3e23040f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], s[8:9], s[6:7]
	s_mov_b32 s6, 0x53dd3955
	s_mov_b32 s7, 0xbe927774
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xcd554c6c
	s_mov_b32 s7, 0x3e5395ab
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x8a3790ad
	s_mov_b32 s7, 0x3eb93638
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x812b5083
	s_mov_b32 s7, 0xbed0d5db
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xd5d652f6
	s_mov_b32 s7, 0x3ec8860c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xcacdfb23
	s_mov_b32 s7, 0x3eea29a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xf80281f2
	s_mov_b32 s7, 0xbf08cef1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xd0b9188a
	s_mov_b32 s7, 0x3f11e684
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x54c8a222
	s_mov_b32 s7, 0x3ef932cd
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x89ef8aa3
	s_mov_b32 s7, 0xbf37448a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x5ad40c25
	s_mov_b32 s7, 0x3f4f3cc5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x132f38b1
	s_mov_b32 s7, 0xbf5ba924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xca533cf8
	s_mov_b32 s7, 0x3f6468ee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xbb891bbd
	s_mov_b32 s7, 0xbf6ebada
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xe5b76afc
	s_mov_b32 s7, 0x3f75ffcf
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x6d641d39
	s_mov_b32 s7, 0x3ff0158a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x380d5a48
	s_mov_b32 s7, 0x4008abcc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[4:5], v[6:7], s[6:7]
.LBB5_14:
	s_or_b32 exec_lo, exec_lo, s0
                                        ; implicit-def: $vgpr6_vgpr7
.LBB5_15:
	s_and_not1_saveexec_b32 s0, s5
	s_cbranch_execz .LBB5_17
; %bb.16:
	v_add_f64 v[4:5], 0xc0090000, -v[6:7]
	s_mov_b32 s6, 0x3324d327
	s_mov_b32 s8, 0xe746e627
	s_mov_b32 s7, 0xbc08ddf9
	s_mov_b32 s9, 0xbbb135d2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], s[8:9], s[6:7]
	s_mov_b32 s6, 0xef0b7c9f
	s_mov_b32 s7, 0x3c37b83e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xcd589b91
	s_mov_b32 s7, 0x3c69ba72
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x90a6b96
	s_mov_b32 s7, 0xbca33689
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x898132e0
	s_mov_b32 s7, 0x3c782e11
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xfd9e26ba
	s_mov_b32 s7, 0x3cfde4ac
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xed66c487
	s_mov_b32 s7, 0xbd26d33e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x7040d8e2
	s_mov_b32 s7, 0xbd36f216
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xc2d77e20
	s_mov_b32 s7, 0x3d872a22
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xc4e5c0af
	s_mov_b32 s7, 0xbdac8859
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xd118a561
	s_mov_b32 s7, 0xbdcdc583
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xccf46b3c
	s_mov_b32 s7, 0x3e120f47
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x8dc84d60
	s_mov_b32 s7, 0xbe31a9e3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x6d3d46a9
	s_mov_b32 s7, 0xbe5f36cd
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x5d03b787
	s_mov_b32 s7, 0x3e9c6b4f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x434ae8a2
	s_mov_b32 s7, 0xbeb6e8a5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x7b8736f6
	s_mov_b32 s7, 0xbeed1d1f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xa212f024
	s_mov_b32 s7, 0x3f2879c2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x9484fca8
	s_mov_b32 s7, 0xbf484576
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x3114f909
	s_mov_b32 s7, 0xbf78b6c3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0xd9b13e28
	s_mov_b32 s7, 0x3fcebd80
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[6:7]
	s_mov_b32 s6, 0x7c99ae86
	s_mov_b32 s7, 0x3ffa755e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[4:5], v[6:7], s[6:7]
.LBB5_17:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[4:5], |v[2:3]|, v[4:5]
	s_or_b32 exec_lo, exec_lo, s4
.LBB5_18:
	s_and_not1_saveexec_b32 s0, s1
	s_cbranch_execz .LBB5_20
.LBB5_19:
	v_mul_f64 v[4:5], v[2:3], v[2:3]
	s_mov_b32 s4, 0x47aef0d6
	s_mov_b32 s6, 0x6cd8002b
	s_mov_b32 s5, 0xbfebb7dd
	s_mov_b32 s7, 0x3fdc5ec0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], s[6:7], s[4:5]
	s_mov_b32 s4, 0x92eccdb6
	s_mov_b32 s5, 0x3fed1899
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x80cde957
	s_mov_b32 s5, 0xbfe10ec1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x379dd66f
	s_mov_b32 s5, 0x3fd05cce
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x7e3dae74
	s_mov_b32 s5, 0xbfa6b906
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x487c11a3
	s_mov_b32 s5, 0x3fa5f7f0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x22b2350c
	s_mov_b32 s5, 0x3f9e0fbf
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x322b7f90
	s_mov_b32 s5, 0x3fa2ce26
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xee81dd31
	s_mov_b32 s5, 0x3fa5ebee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xb897f0d4
	s_mov_b32 s5, 0x3faa7cac
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xd62cba32
	s_mov_b32 s5, 0x3fb0a130
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xc8653359
	s_mov_b32 s5, 0x3fb62847
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xc0a5e083
	s_mov_b32 s5, 0x3fc053c2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xb2feec72
	s_mov_b32 s5, 0x3fcdb29f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x91b4ef6a
	s_mov_b32 s5, 0x3fec5bf8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[4:5], v[6:7], s[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[4:5], |v[2:3]|, v[4:5]
.LBB5_20:
	s_or_b32 exec_lo, exec_lo, s0
	v_cmp_ngt_f64_e64 vcc_lo, |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v5, 0x7ff80000, v5, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, |v[2:3]|, 1.0
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	v_cmp_nge_f64_e64 vcc_lo, |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_bfi_b32 v3, 0x7fffffff, v5, v3
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB5_21:
	s_endpgm
.LBB5_22:
	s_or_b32 exec_lo, exec_lo, s4
	s_and_not1_saveexec_b32 s0, s1
	s_cbranch_execnz .LBB5_19
	s_branch .LBB5_20
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_erfinv_kernel
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
		.amdhsa_inst_pref_size 43
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
	.size	specialx_erfinv_kernel, .Lfunc_end5-specialx_erfinv_kernel
                                        ; -- End function
	.set specialx_erfinv_kernel.num_vgpr, 24
	.set specialx_erfinv_kernel.num_agpr, 0
	.set specialx_erfinv_kernel.numbered_sgpr, 12
	.set specialx_erfinv_kernel.num_named_barrier, 0
	.set specialx_erfinv_kernel.private_seg_size, 0
	.set specialx_erfinv_kernel.uses_vcc, 1
	.set specialx_erfinv_kernel.uses_flat_scratch, 0
	.set specialx_erfinv_kernel.has_dyn_sized_stack, 0
	.set specialx_erfinv_kernel.has_recursion, 0
	.set specialx_erfinv_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 5456
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
	.protected	specialx_erfcx_kernel   ; -- Begin function specialx_erfcx_kernel
	.globl	specialx_erfcx_kernel
	.p2align	8
	.type	specialx_erfcx_kernel,@function
specialx_erfcx_kernel:                  ; @specialx_erfcx_kernel
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
	s_cbranch_execz .LBB6_8
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x41e48bfc
	s_mov_b32 s1, 0x403b39dc
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_nlt_f64_e64 s0, |v[2:3]|, s[0:1]
	s_and_saveexec_b32 s1, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB6_3
; %bb.2:
	v_dual_mov_b32 v4, v2 :: v_dual_and_b32 v5, 0x7fffffff, v3
	s_mov_b32 s4, 0
	s_mov_b32 s5, 0xc03d8800
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[4:5], vcc_lo, 1.0, v[4:5], 1.0
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_mul_f64 v[10:11], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[6:7], v[10:11], v[4:5]
	v_div_fmas_f64 v[4:5], v[4:5], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[4:5], |v[2:3]|, 1.0
	v_mul_f64 v[6:7], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[4:5], 0x401a4000
	s_mov_b32 s4, 0x50429b6d
	s_mov_b32 s5, 0x3fe20dd7
	v_mul_f64 v[4:5], v[4:5], s[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 0xbffe0000
	v_fma_f64 v[8:9], v[6:7], v[8:9], 0x3fe80000
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], -0.5
	v_fma_f64 v[6:7], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], v[6:7]
.LBB6_3:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB6_5
; %bb.4:
	v_add_f64 v[4:5], |v[2:3]|, 4.0
	s_mov_b32 s4, 0x37cfa789
	s_mov_b32 s6, 0x54df3c0e
	s_mov_b32 s5, 0xbe411663
	s_mov_b32 s7, 0xbe41f39d
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[8:9], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[6:7], v[8:9], v[6:7], v[6:7]
	v_add_f64 v[8:9], |v[2:3]|, -4.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[4:5], v[6:7], 1.0
	v_fma_f64 v[4:5], v[4:5], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[8:9], v[4:5]
	v_add_f64 v[8:9], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], -4.0, |v[2:3]|
	v_fma_f64 v[8:9], -v[6:7], |v[2:3]|, v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[4:5], v[8:9], v[6:7]
	v_add_f64 v[8:9], |v[2:3]|, |v[2:3]|
	v_fma_f64 v[6:7], v[4:5], s[6:7], s[4:5]
	s_mov_b32 s4, 0xd9802b82
	s_mov_b32 s5, 0x3e7b45f1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], 1.0
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x8a03dcdb
	s_mov_b32 s5, 0x3e6d9048
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[12:13], v[10:11]
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x2eba62d8
	s_mov_b32 s5, 0xbeab87b0
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xa56e15f1
	s_mov_b32 s5, 0x3e95104b
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x71c907de
	s_mov_b32 s5, 0x3ed7f29f
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x2cd770fb
	s_mov_b32 s5, 0xbee78f5c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[12:13]
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x76d0a51a
	s_mov_b32 s5, 0xbef995fb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xc022d0ed
	s_mov_b32 s5, 0x3f23be2e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x2fdbf62e
	s_mov_b32 s5, 0xbf2a1deb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x3689fc43
	s_mov_b32 s5, 0xbf48d4ac
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x192d909b
	s_mov_b32 s5, 0x3f749c67
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x852ff070
	s_mov_b32 s5, 0xbf909623
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xdfadea8f
	s_mov_b32 s5, 0x3fa3079e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xdff65910
	s_mov_b32 s5, 0xbfb0fb06
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x4de8f32
	s_mov_b32 s5, 0x3fb7fee0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0x3c3dbeb3
	s_mov_b32 s5, 0xbfb9ddb2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xfcfa6930
	s_mov_b32 s5, 0x3fb16ece
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xf66fb8a3
	s_mov_b32 s5, 0x3f8f7f5d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xd154a2a8
	s_mov_b32 s5, 0xbfc1df1a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[4:5], v[6:7], s[4:5]
	s_mov_b32 s4, 0xb74febf8
	s_mov_b32 s5, 0x3fcdd2c8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[4:5], v[6:7], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[4:5], v[10:11], v[10:11]
	v_fma_f64 v[8:9], -v[6:7], v[8:9], 1.0
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_fma_f64 v[4:5], v[10:11], v[4:5], v[6:7]
.LBB6_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB6_7
; %bb.6:
	v_mul_f64 v[6:7], v[2:3], v[2:3]
	s_mov_b32 s4, 0x652b82fe
	s_mov_b32 s5, 0x3ff71547
	s_mov_b32 s6, 0x6a5dcb37
	s_mov_b32 s7, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[6:7], s[4:5]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0xbfe62e42
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[6:7]
	v_rndne_f64_e32 v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[8:9], s[4:5], v[6:7]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0xbc7abc9e
	v_cvt_i32_f64_e32 v14, v[8:9]
	v_fma_f64 v[6:7], v[2:3], v[2:3], -v[6:7]
	v_fma_f64 v[10:11], v[8:9], s[4:5], v[10:11]
	s_mov_b32 s4, 0xfca7ab0c
	s_mov_b32 s5, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], s[6:7], s[4:5]
	s_mov_b32 s4, 0x623fde64
	s_mov_b32 s5, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x7c89e6b0
	s_mov_b32 s5, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x14761f6e
	s_mov_b32 s5, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x1852b7b0
	s_mov_b32 s5, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x11122322
	s_mov_b32 s5, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x555502a1
	s_mov_b32 s5, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0x55555511
	s_mov_b32 s5, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 11
	s_mov_b32 s5, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[4:5]
	s_mov_b32 s4, 0xd2e063ce
	s_mov_b32 s5, 0xc03aa0f4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], 1.0
	v_fma_f64 v[8:9], v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v14
	v_cndmask_b32_e32 v9, 0x7ff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, s[4:5], v[2:3]
	v_fma_f64 v[6:7], v[8:9], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[6:7], 2.0, -v[4:5]
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
.LBB6_7:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB6_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_erfcx_kernel
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
	.size	specialx_erfcx_kernel, .Lfunc_end6-specialx_erfcx_kernel
                                        ; -- End function
	.set specialx_erfcx_kernel.num_vgpr, 16
	.set specialx_erfcx_kernel.num_agpr, 0
	.set specialx_erfcx_kernel.numbered_sgpr, 8
	.set specialx_erfcx_kernel.num_named_barrier, 0
	.set specialx_erfcx_kernel.private_seg_size, 0
	.set specialx_erfcx_kernel.uses_vcc, 1
	.set specialx_erfcx_kernel.uses_flat_scratch, 0
	.set specialx_erfcx_kernel.has_dyn_sized_stack, 0
	.set specialx_erfcx_kernel.has_recursion, 0
	.set specialx_erfcx_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1760
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
	.protected	specialx_i0_kernel      ; -- Begin function specialx_i0_kernel
	.globl	specialx_i0_kernel
	.p2align	8
	.type	specialx_i0_kernel,@function
specialx_i0_kernel:                     ; @specialx_i0_kernel
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
	s_cbranch_execz .LBB7_6
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr6_vgpr7
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_and_b32_e32 v5, 0x7fffffff, v3
	v_cmpx_ngt_f64_e64 0x40200000, |v[2:3]|
	s_xor_b32 s3, exec_lo, s1
	s_cbranch_execz .LBB7_3
; %bb.2:
	v_mov_b32_e32 v4, v2
	s_mov_b32 s0, 0x22975981
	s_mov_b32 s4, 0xbacb549d
	s_mov_b32 s1, 0xc315ba77
	s_mov_b32 s5, 0x430cc967
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[4:5], 1.0
	v_rsq_f64_e64 v[18:19], |v[2:3]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[20:21], v[18:19], -|v[2:3]|
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[20:21], v[20:21], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[12:13]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	v_add_f64 v[10:11], 0xc0862800, |v[2:3]|
	v_cmp_lt_f64_e64 vcc_lo, 0x40862800, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_fixup_f64 v[6:7], v[6:7], |v[2:3]|, 1.0
	v_dual_cndmask_b32 v11, v5, v11 :: v_dual_cndmask_b32 v10, v2, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[10:11]
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0x36763276
	s_mov_b32 s1, 0x430df0f8
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x430f3f43
	s_mov_b32 s1, 0xc2f9042a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mul_f64 v[12:13], v[10:11], s[0:1]
	s_mov_b32 s0, 0x41c4f568
	s_mov_b32 s1, 0x42dc6305
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xe5a9784f
	s_mov_b32 s1, 0xc2b7366b
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rndne_f64_e32 v[12:13], v[12:13]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[0:1], v[10:11]
	s_mov_b32 s0, 0xa48f574e
	s_mov_b32 s1, 0x428c5669
	v_cvt_i32_f64_e32 v4, v[12:13]
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[0:1], v[14:15]
	s_mov_b32 s0, 0xac47f0ea
	s_mov_b32 s1, 0xc25a664c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], s[4:5], s[0:1]
	s_mov_b32 s0, 0x566988c
	s_mov_b32 s1, 0x42230825
	s_mov_b32 s4, 0x33daea3d
	s_mov_b32 s5, 0x3fa98845
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xc2ddb061
	s_mov_b32 s1, 0xc1e56874
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x968da2aa
	s_mov_b32 s1, 0x41a2da58
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x33f0d6bc
	s_mov_b32 s1, 0xc159faaa
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xf2bc76dd
	s_mov_b32 s1, 0x410be0a8
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x68c3cb02
	s_mov_b32 s1, 0xc0b7123c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x50cc72aa
	s_mov_b32 s1, 0x405d4021
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x85359520
	s_mov_b32 s1, 0xbff7a8ae
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xb6a753cd
	s_mov_b32 s1, 0x3fbbd7e0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3774506d
	s_mov_b32 s1, 0x3fa6d6ce
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], v[16:17], 1.0
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3d2f7cf9
	s_mov_b32 s1, 0x3f9debdd
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[12:13], v[14:15], v[16:17], 1.0
	v_mul_f64 v[14:15], v[18:19], v[20:21]
	v_fma_f64 v[16:17], 0x3fd80000, v[20:21], 0.5
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xb8d452d5
	s_mov_b32 s1, 0x3f9cb94d
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[12:13], v[12:13], v4
	v_fma_f64 v[14:15], v[14:15], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	v_cmp_nlt_f64_e64 s1, 0x40900000, v[10:11]
	v_cmp_class_f64_e64 s0, v[18:19], 0x180
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v4, 0x7ff00000, v13, s1
	v_cndmask_b32_e64 v11, v19, v15, s0
	v_cndmask_b32_e64 v10, v18, v14, s0
	s_and_b32 s0, s2, s1
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v13, 0, v4, s2
	v_cndmask_b32_e64 v12, 0, v12, s0
	s_mov_b32 s0, 0x33d4362f
	s_mov_b32 s1, 0x3fd98845
	v_mov_b32_e32 v4, 0x7fdd422d
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[0:1]
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_cndmask_b32_e32 v9, 0x3ff00000, v4, vcc_lo
	v_cndmask_b32_e64 v8, 0, 0x2be5dc9b, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
.LBB7_3:
	s_and_not1_saveexec_b32 s0, s3
	s_cbranch_execz .LBB7_5
; %bb.4:
	v_ldexp_f64 v[6:7], |v[2:3]|, -2
	s_mov_b32 s2, 0x59531e65
	s_mov_b32 s4, 0x50ff79b2
	s_mov_b32 s3, 0x3a643945
	s_mov_b32 s5, 0x39edd787
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], |v[2:3]|, v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[2:3]
	s_mov_b32 s2, 0x3f151c79
	s_mov_b32 s3, 0x3ae6f712
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xc5528048
	s_mov_b32 s3, 0x3b63d9e7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x323a0cab
	s_mov_b32 s3, 0x3bde736f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xe3b298c5
	s_mov_b32 s3, 0x3c54196c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xc7bf9255
	s_mov_b32 s3, 0x3cc69caa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x78c06ac8
	s_mov_b32 s3, 0x3d356018
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x291f5e48
	s_mov_b32 s3, 0x3da0b313
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x3f5dcb54
	s_mov_b32 s3, 0x3e0522a4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x3f659634
	s_mov_b32 s3, 0x3e6522a4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xc0898945
	s_mov_b32 s3, 0x3ec02e85
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x789abcf3
	s_mov_b32 s3, 0x3f123456
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s3, 0x3f5c71c7
	s_mov_b32 s2, 0x1c71c71c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s3, 0x3f9c71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 0x3fd00000
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], 1.0
.LBB7_5:
	s_or_b32 exec_lo, exec_lo, s0
	v_cmp_nlg_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v3, v7, v5 :: v_dual_cndmask_b32 v2, v6, v2
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB7_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_i0_kernel
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
		.amdhsa_next_free_vgpr 22
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
		.amdhsa_inst_pref_size 16
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
	.size	specialx_i0_kernel, .Lfunc_end7-specialx_i0_kernel
                                        ; -- End function
	.set specialx_i0_kernel.num_vgpr, 22
	.set specialx_i0_kernel.num_agpr, 0
	.set specialx_i0_kernel.numbered_sgpr, 8
	.set specialx_i0_kernel.num_named_barrier, 0
	.set specialx_i0_kernel.private_seg_size, 0
	.set specialx_i0_kernel.uses_vcc, 1
	.set specialx_i0_kernel.uses_flat_scratch, 0
	.set specialx_i0_kernel.has_dyn_sized_stack, 0
	.set specialx_i0_kernel.has_recursion, 0
	.set specialx_i0_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1996
; TotalNumSgprs: 10
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 22
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
	.protected	specialx_i1_kernel      ; -- Begin function specialx_i1_kernel
	.globl	specialx_i1_kernel
	.p2align	8
	.type	specialx_i1_kernel,@function
specialx_i1_kernel:                     ; @specialx_i1_kernel
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
	s_cbranch_execz .LBB8_6
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr6_vgpr7
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_and_b32_e32 v5, 0x7fffffff, v3
	v_cmpx_ngt_f64_e64 0x40200000, |v[2:3]|
	s_xor_b32 s3, exec_lo, s1
	s_cbranch_execz .LBB8_3
; %bb.2:
	v_mov_b32_e32 v4, v2
	s_mov_b32 s0, 0xe12fb4ba
	s_mov_b32 s4, 0x43214423
	s_mov_b32 s1, 0x4315c072
	s_mov_b32 s5, 0xc30c9d8d
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[4:5], 1.0
	v_rsq_f64_e32 v[18:19], v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[20:21], v[18:19], -v[2:3]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[20:21], v[20:21], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[12:13]
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	v_add_f64 v[10:11], 0xc0862800, v[2:3]
	v_cmp_lt_f64_e32 vcc_lo, 0x40862800, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_div_fixup_f64 v[6:7], v[6:7], |v[2:3]|, 1.0
	v_dual_cndmask_b32 v11, v3, v11 :: v_dual_cndmask_b32 v10, v2, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[10:11]
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0xf438b6f6
	s_mov_b32 s1, 0xc30e26cf
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x4c61a221
	s_mov_b32 s1, 0x42f95222
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mul_f64 v[12:13], v[10:11], s[0:1]
	s_mov_b32 s0, 0x873cf435
	s_mov_b32 s1, 0xc2dcdc7c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x2a15fb86
	s_mov_b32 s1, 0x42b7b1e3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rndne_f64_e32 v[12:13], v[12:13]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[0:1], v[10:11]
	s_mov_b32 s0, 0xd6696f1c
	s_mov_b32 s1, 0xc28d07db
	v_cvt_i32_f64_e32 v4, v[12:13]
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[0:1], v[14:15]
	s_mov_b32 s0, 0x34f2ced2
	s_mov_b32 s1, 0x425b2279
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], s[4:5], s[0:1]
	s_mov_b32 s0, 0xe6685444
	s_mov_b32 s1, 0xc2239f23
	s_mov_b32 s4, 0xe6e0f07a
	s_mov_b32 s5, 0xbfc32633
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x83f6f890
	s_mov_b32 s1, 0x41e62293
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xceeee865
	s_mov_b32 s1, 0xc1a38bf1
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x48b749b8
	s_mov_b32 s1, 0x415b01a3
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x3ef0916a
	s_mov_b32 s1, 0xc10d0e04
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xf82cfbac
	s_mov_b32 s1, 0x40b81b06
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xb2a6508b
	s_mov_b32 s1, 0xc05ea879
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xc8d54f52
	s_mov_b32 s1, 0x3ff85cff
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x7ee0f7e2
	s_mov_b32 s1, 0xbfc09f10
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1539fb0d
	s_mov_b32 s1, 0xbfad6163
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], v[16:17], 1.0
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1d904eba
	s_mov_b32 s1, 0xbfa4f1e0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[12:13], v[14:15], v[16:17], 1.0
	v_mul_f64 v[14:15], v[18:19], v[20:21]
	v_fma_f64 v[16:17], 0x3fd80000, v[20:21], 0.5
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xced79c58
	s_mov_b32 s1, 0xbfa7efc0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[12:13], v[12:13], v4
	v_fma_f64 v[14:15], v[14:15], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	v_cmp_nlt_f64_e64 s1, 0x40900000, v[10:11]
	v_cmp_class_f64_e64 s0, v[18:19], 0x180
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v4, 0x7ff00000, v13, s1
	v_cndmask_b32_e64 v11, v19, v15, s0
	v_cndmask_b32_e64 v10, v18, v14, s0
	s_and_b32 s0, s2, s1
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v13, 0, v4, s2
	v_cndmask_b32_e64 v12, 0, v12, s0
	s_mov_b32 s0, 0x33d43674
	s_mov_b32 s1, 0x3fd98845
	v_mov_b32_e32 v4, 0x7fdd422d
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[0:1]
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_cndmask_b32_e32 v9, 0x3ff00000, v4, vcc_lo
	v_cndmask_b32_e64 v8, 0, 0x2be5dc9b, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[6:7], v[8:9], v[6:7]
.LBB8_3:
	s_or_saveexec_b32 s0, s3
	v_mov_b32_e32 v4, v2
	s_xor_b32 exec_lo, exec_lo, s0
	s_cbranch_execz .LBB8_5
; %bb.4:
	v_mul_f64 v[4:5], |v[2:3]|, 0.5
	s_mov_b32 s2, 0x2d94a857
	s_mov_b32 s4, 0xc836e80a
	s_mov_b32 s3, 0x3aa43235
	s_mov_b32 s5, 0x3a2fc892
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[4:5], v[4:5]
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[2:3]
	s_mov_b32 s2, 0x4f7b7a4a
	s_mov_b32 s3, 0x3b2588ae
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xe9231b49
	s_mov_b32 s3, 0x3ba15e96
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x5f2184d1
	s_mov_b32 s3, 0x3c18bdcb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x7a1e02fe
	s_mov_b32 s3, 0x3c8e2623
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xca1a831f
	s_mov_b32 s3, 0x3cff176a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x97c83e75
	s_mov_b32 s3, 0x3d6ab81e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x8e3649ff
	s_mov_b32 s3, 0x3dd2c975
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x3f5ed306
	s_mov_b32 s3, 0x3e3522a4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xb778d591
	s_mov_b32 s3, 0x3e927e4f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xa0ce4eda
	s_mov_b32 s3, 0x3ee845c8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x16c16c26
	s_mov_b32 s3, 0x3f36c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x1c71c71c
	s_mov_b32 s3, 0x3f7c71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x55555555
	s_mov_b32 s3, 0x3fb55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 0.5
	v_mul_f64 v[8:9], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[4:5]
.LBB8_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_nlg_f64_e32 vcc_lo, 0x7ff00000, v[4:5]
	v_cndmask_b32_e32 v2, v6, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v4, v7, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_bfi_b32 v3, 0x7fffffff, v4, v3
	global_store_b64 v[0:1], v[2:3], off
.LBB8_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_i1_kernel
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
		.amdhsa_next_free_vgpr 22
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
		.amdhsa_inst_pref_size 16
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
	.size	specialx_i1_kernel, .Lfunc_end8-specialx_i1_kernel
                                        ; -- End function
	.set specialx_i1_kernel.num_vgpr, 22
	.set specialx_i1_kernel.num_agpr, 0
	.set specialx_i1_kernel.numbered_sgpr, 8
	.set specialx_i1_kernel.num_named_barrier, 0
	.set specialx_i1_kernel.private_seg_size, 0
	.set specialx_i1_kernel.uses_vcc, 1
	.set specialx_i1_kernel.uses_flat_scratch, 0
	.set specialx_i1_kernel.has_dyn_sized_stack, 0
	.set specialx_i1_kernel.has_recursion, 0
	.set specialx_i1_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2016
; TotalNumSgprs: 10
; NumVgprs: 22
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
; NumVGPRsForWavesPerEU: 22
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
	.protected	specialx_i0e_kernel     ; -- Begin function specialx_i0e_kernel
	.globl	specialx_i0e_kernel
	.p2align	8
	.type	specialx_i0e_kernel,@function
specialx_i0e_kernel:                    ; @specialx_i0e_kernel
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
	s_cbranch_execz .LBB9_6
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr6_vgpr7
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_dual_mov_b32 v4, v2 :: v_dual_and_b32 v5, 0x7fffffff, v3
	v_cmpx_ngt_f64_e64 0x40200000, |v[2:3]|
	s_xor_b32 s3, exec_lo, s1
	s_cbranch_execz .LBB9_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_2)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[4:5], 1.0
	s_mov_b32 s0, 0x22975981
	s_mov_b32 s4, 0xbacb549d
	s_mov_b32 s1, 0xc315ba77
	s_mov_b32 s5, 0x430cc967
	v_rsq_f64_e64 v[18:19], |v[2:3]|
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[20:21], v[18:19], -v[4:5]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[20:21], v[18:19], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	v_add_f64 v[10:11], 0xc0862800, |v[2:3]|
	v_cmp_lt_f64_e64 vcc_lo, 0x40862800, |v[2:3]|
	v_div_fixup_f64 v[6:7], v[6:7], |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v11, v5, v11 :: v_dual_cndmask_b32 v10, v2, v10
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0x36763276
	s_mov_b32 s1, 0x430df0f8
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x430f3f43
	s_mov_b32 s1, 0xc2f9042a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mul_f64 v[12:13], v[10:11], s[0:1]
	s_mov_b32 s0, 0x41c4f568
	s_mov_b32 s1, 0x42dc6305
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xe5a9784f
	s_mov_b32 s1, 0xc2b7366b
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rndne_f64_e32 v[12:13], v[12:13]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[0:1], v[10:11]
	s_mov_b32 s0, 0xa48f574e
	s_mov_b32 s1, 0x428c5669
	v_cvt_i32_f64_e32 v22, v[12:13]
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[0:1], v[14:15]
	s_mov_b32 s0, 0xac47f0ea
	s_mov_b32 s1, 0xc25a664c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], s[4:5], s[0:1]
	s_mov_b32 s0, 0x566988c
	s_mov_b32 s1, 0x42230825
	s_mov_b32 s4, 0x33daea3d
	s_mov_b32 s5, 0x3fa98845
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xc2ddb061
	s_mov_b32 s1, 0xc1e56874
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x968da2aa
	s_mov_b32 s1, 0x41a2da58
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x33f0d6bc
	s_mov_b32 s1, 0xc159faaa
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xf2bc76dd
	s_mov_b32 s1, 0x410be0a8
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x68c3cb02
	s_mov_b32 s1, 0xc0b7123c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x50cc72aa
	s_mov_b32 s1, 0x405d4021
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x85359520
	s_mov_b32 s1, 0xbff7a8ae
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xb6a753cd
	s_mov_b32 s1, 0x3fbbd7e0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3774506d
	s_mov_b32 s1, 0x3fa6d6ce
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], v[16:17], 1.0
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3d2f7cf9
	s_mov_b32 s1, 0x3f9debdd
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[12:13], v[14:15], v[16:17], 1.0
	v_mul_f64 v[14:15], v[18:19], v[20:21]
	v_fma_f64 v[16:17], 0x3fd80000, v[20:21], 0.5
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xb8d452d5
	s_mov_b32 s1, 0x3f9cb94d
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[12:13], v[12:13], v22
	v_fma_f64 v[14:15], v[14:15], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	v_cmp_nlt_f64_e64 s1, 0x40900000, v[10:11]
	v_cmp_class_f64_e64 s0, v[18:19], 0x180
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v13, 0x7ff00000, v13, s1
	v_cndmask_b32_e64 v11, v19, v15, s0
	v_cndmask_b32_e64 v10, v18, v14, s0
	s_and_b32 s0, s2, s1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v13, 0, v13, s2
	v_cndmask_b32_e64 v12, 0, v12, s0
	s_mov_b32 s0, 0x33d4362f
	s_mov_b32 s1, 0x3fd98845
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_mov_b32_e32 v8, 0x7fdd422d
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v9, 0x3ff00000, v8, vcc_lo
	v_cndmask_b32_e64 v8, 0, 0x2be5dc9b, vcc_lo
	v_mul_f64 v[6:7], v[8:9], v[6:7]
.LBB9_3:
	s_and_not1_saveexec_b32 s0, s3
	s_cbranch_execz .LBB9_5
; %bb.4:
	v_ldexp_f64 v[6:7], |v[2:3]|, -2
	s_mov_b32 s2, 0x59531e65
	s_mov_b32 s4, 0x50ff79b2
	s_mov_b32 s3, 0x3a643945
	s_mov_b32 s5, 0x39edd787
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], |v[2:3]|, v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[2:3]
	s_mov_b32 s2, 0x3f151c79
	s_mov_b32 s3, 0x3ae6f712
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xc5528048
	s_mov_b32 s3, 0x3b63d9e7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x323a0cab
	s_mov_b32 s3, 0x3bde736f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xe3b298c5
	s_mov_b32 s3, 0x3c54196c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xc7bf9255
	s_mov_b32 s3, 0x3cc69caa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x78c06ac8
	s_mov_b32 s3, 0x3d356018
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x291f5e48
	s_mov_b32 s3, 0x3da0b313
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x3f5dcb54
	s_mov_b32 s3, 0x3e0522a4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x3f659634
	s_mov_b32 s3, 0x3e6522a4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0xc0898945
	s_mov_b32 s3, 0x3ec02e85
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s2, 0x789abcf3
	s_mov_b32 s3, 0x3f123456
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s3, 0x3f5c71c7
	s_mov_b32 s2, 0x1c71c71c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_mov_b32 s3, 0x3f9c71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 0x3fd00000
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], 1.0
.LBB9_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	s_mov_b32 s2, 0x6a5dcb37
	v_mul_f64 v[8:9], v[4:5], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_mov_b32 s3, 0x3e5ade15
	v_cmp_nlt_f64_e32 vcc_lo, 0x4090cc00, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[8:9]
	v_fma_f64 v[10:11], v[8:9], s[0:1], -v[4:5]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	v_cvt_i32_f64_e32 v14, v[8:9]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], s[0:1], v[10:11]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	v_cmp_nlg_f64_e64 s0, 0x7ff00000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], 1.0
	v_cndmask_b32_e64 v3, v7, v5, s0
	v_cndmask_b32_e64 v2, v6, v2, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[10:11], v[12:13], 1.0
	v_ldexp_f64 v[8:9], v[8:9], v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v5, 0, v9 :: v_dual_cndmask_b32 v4, 0, v8
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[2:3], v[4:5], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB9_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_i0e_kernel
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
		.amdhsa_next_free_vgpr 23
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
		.amdhsa_inst_pref_size 19
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
	.size	specialx_i0e_kernel, .Lfunc_end9-specialx_i0e_kernel
                                        ; -- End function
	.set specialx_i0e_kernel.num_vgpr, 23
	.set specialx_i0e_kernel.num_agpr, 0
	.set specialx_i0e_kernel.numbered_sgpr, 8
	.set specialx_i0e_kernel.num_named_barrier, 0
	.set specialx_i0e_kernel.private_seg_size, 0
	.set specialx_i0e_kernel.uses_vcc, 1
	.set specialx_i0e_kernel.uses_flat_scratch, 0
	.set specialx_i0e_kernel.has_dyn_sized_stack, 0
	.set specialx_i0e_kernel.has_recursion, 0
	.set specialx_i0e_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2416
; TotalNumSgprs: 10
; NumVgprs: 23
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
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
	.text
	.protected	specialx_i1e_kernel     ; -- Begin function specialx_i1e_kernel
	.globl	specialx_i1e_kernel
	.p2align	8
	.type	specialx_i1e_kernel,@function
specialx_i1e_kernel:                    ; @specialx_i1e_kernel
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
	s_cbranch_execz .LBB10_6
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr8_vgpr9
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_dual_mov_b32 v4, v2 :: v_dual_and_b32 v5, 0x7fffffff, v3
	v_cmpx_ngt_f64_e64 0x40200000, |v[2:3]|
	s_xor_b32 s3, exec_lo, s1
	s_cbranch_execz .LBB10_3
; %bb.2:
	s_delay_alu instid0(VALU_DEP_2)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[4:5], 1.0
	s_mov_b32 s0, 0xe12fb4ba
	s_mov_b32 s4, 0x43214423
	s_mov_b32 s1, 0x4315c072
	s_mov_b32 s5, 0xc30c9d8d
	v_rsq_f64_e32 v[18:19], v[2:3]
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[20:21], v[18:19], -v[2:3]
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[20:21], v[18:19], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_fma_f64 v[6:7], -v[6:7], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	v_add_f64 v[10:11], 0xc0862800, v[2:3]
	v_cmp_lt_f64_e32 vcc_lo, 0x40862800, v[2:3]
	v_div_fixup_f64 v[6:7], v[6:7], |v[2:3]|, 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v11, v3, v11 :: v_dual_cndmask_b32 v10, v2, v10
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[4:5], s[0:1]
	s_mov_b32 s0, 0xf438b6f6
	s_mov_b32 s1, 0xc30e26cf
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x4c61a221
	s_mov_b32 s1, 0x42f95222
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mul_f64 v[12:13], v[10:11], s[0:1]
	s_mov_b32 s0, 0x873cf435
	s_mov_b32 s1, 0xc2dcdc7c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x2a15fb86
	s_mov_b32 s1, 0x42b7b1e3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rndne_f64_e32 v[12:13], v[12:13]
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[0:1], v[10:11]
	s_mov_b32 s0, 0xd6696f1c
	s_mov_b32 s1, 0xc28d07db
	v_cvt_i32_f64_e32 v22, v[12:13]
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[0:1], v[14:15]
	s_mov_b32 s0, 0x34f2ced2
	s_mov_b32 s1, 0x425b2279
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], s[4:5], s[0:1]
	s_mov_b32 s0, 0xe6685444
	s_mov_b32 s1, 0xc2239f23
	s_mov_b32 s4, 0xe6e0f07a
	s_mov_b32 s5, 0xbfc32633
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x83f6f890
	s_mov_b32 s1, 0x41e62293
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xceeee865
	s_mov_b32 s1, 0xc1a38bf1
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x48b749b8
	s_mov_b32 s1, 0x415b01a3
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x3ef0916a
	s_mov_b32 s1, 0xc10d0e04
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xf82cfbac
	s_mov_b32 s1, 0x40b81b06
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xb2a6508b
	s_mov_b32 s1, 0xc05ea879
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0xc8d54f52
	s_mov_b32 s1, 0x3ff85cff
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[14:15], v[16:17], s[0:1]
	s_mov_b32 s0, 0x7ee0f7e2
	s_mov_b32 s1, 0xbfc09f10
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1539fb0d
	s_mov_b32 s1, 0xbfad6163
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[14:15], v[16:17], 1.0
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0x1d904eba
	s_mov_b32 s1, 0xbfa4f1e0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_fma_f64 v[12:13], v[14:15], v[16:17], 1.0
	v_mul_f64 v[14:15], v[18:19], v[20:21]
	v_fma_f64 v[16:17], 0x3fd80000, v[20:21], 0.5
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	s_mov_b32 s0, 0xced79c58
	s_mov_b32 s1, 0xbfa7efc0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[12:13], v[12:13], v22
	v_fma_f64 v[14:15], v[14:15], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[0:1]
	v_cmp_nlt_f64_e64 s1, 0x40900000, v[10:11]
	v_cmp_class_f64_e64 s0, v[18:19], 0x180
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v13, 0x7ff00000, v13, s1
	v_cndmask_b32_e64 v11, v19, v15, s0
	v_cndmask_b32_e64 v10, v18, v14, s0
	s_and_b32 s0, s2, s1
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v13, 0, v13, s2
	v_cndmask_b32_e64 v12, 0, v12, s0
	s_mov_b32 s0, 0x33d43674
	s_mov_b32 s1, 0x3fd98845
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_mov_b32_e32 v8, 0x7fdd422d
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v9, 0x3ff00000, v8, vcc_lo
	v_cndmask_b32_e64 v8, 0, 0x2be5dc9b, vcc_lo
	v_mul_f64 v[8:9], v[8:9], v[6:7]
.LBB10_3:
	s_or_saveexec_b32 s0, s3
	v_dual_mov_b32 v7, v5 :: v_dual_mov_b32 v6, v4
	s_xor_b32 exec_lo, exec_lo, s0
	s_cbranch_execz .LBB10_5
; %bb.4:
	v_mul_f64 v[6:7], |v[2:3]|, 0.5
	s_mov_b32 s2, 0x2d94a857
	s_mov_b32 s4, 0xc836e80a
	s_mov_b32 s3, 0x3aa43235
	s_mov_b32 s5, 0x3a2fc892
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[6:7], v[6:7]
	v_fma_f64 v[10:11], v[8:9], s[4:5], s[2:3]
	s_mov_b32 s2, 0x4f7b7a4a
	s_mov_b32 s3, 0x3b2588ae
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0xe9231b49
	s_mov_b32 s3, 0x3ba15e96
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x5f2184d1
	s_mov_b32 s3, 0x3c18bdcb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x7a1e02fe
	s_mov_b32 s3, 0x3c8e2623
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0xca1a831f
	s_mov_b32 s3, 0x3cff176a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x97c83e75
	s_mov_b32 s3, 0x3d6ab81e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x8e3649ff
	s_mov_b32 s3, 0x3dd2c975
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x3f5ed306
	s_mov_b32 s3, 0x3e3522a4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0xb778d591
	s_mov_b32 s3, 0x3e927e4f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0xa0ce4eda
	s_mov_b32 s3, 0x3ee845c8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x16c16c26
	s_mov_b32 s3, 0x3f36c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x1c71c71c
	s_mov_b32 s3, 0x3f7c71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_mov_b32 s2, 0x55555555
	s_mov_b32 s3, 0x3fb55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], 0.5
	v_mul_f64 v[10:11], v[6:7], v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[6:7]
.LBB10_5:
	s_or_b32 exec_lo, exec_lo, s0
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	s_mov_b32 s2, 0x6a5dcb37
	v_mul_f64 v[10:11], v[4:5], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_mov_b32 s3, 0x3e5ade15
	v_cmp_nlg_f64_e32 vcc_lo, 0x7ff00000, v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], s[0:1], -v[4:5]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	v_cvt_i32_f64_e32 v2, v[10:11]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], s[0:1], v[12:13]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], 1.0
	v_fma_f64 v[10:11], v[12:13], v[14:15], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[10:11], v[10:11], v2
	v_cndmask_b32_e32 v2, v8, v6, vcc_lo
	v_cndmask_b32_e32 v6, v9, v7, vcc_lo
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_bfi_b32 v3, 0x7fffffff, v6, v3
	v_cndmask_b32_e64 v5, 0, v11, s0
	v_cndmask_b32_e64 v4, 0, v10, s0
	v_mul_f64 v[2:3], v[4:5], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB10_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_i1e_kernel
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
		.amdhsa_next_free_vgpr 23
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
		.amdhsa_inst_pref_size 19
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
	.size	specialx_i1e_kernel, .Lfunc_end10-specialx_i1e_kernel
                                        ; -- End function
	.set specialx_i1e_kernel.num_vgpr, 23
	.set specialx_i1e_kernel.num_agpr, 0
	.set specialx_i1e_kernel.numbered_sgpr, 8
	.set specialx_i1e_kernel.num_named_barrier, 0
	.set specialx_i1e_kernel.private_seg_size, 0
	.set specialx_i1e_kernel.uses_vcc, 1
	.set specialx_i1e_kernel.uses_flat_scratch, 0
	.set specialx_i1e_kernel.has_dyn_sized_stack, 0
	.set specialx_i1e_kernel.has_recursion, 0
	.set specialx_i1e_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2432
; TotalNumSgprs: 10
; NumVgprs: 23
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 10
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
	.text
	.protected	specialx_j0_kernel      ; -- Begin function specialx_j0_kernel
	.globl	specialx_j0_kernel
	.p2align	8
	.type	specialx_j0_kernel,@function
specialx_j0_kernel:                     ; @specialx_j0_kernel
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
	s_cbranch_execz .LBB11_22
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_ge_f64_e64 s0, 0x40292800, |v[2:3]|
	s_and_saveexec_b32 s1, s0
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB11_15
; %bb.2:
	v_cmp_ge_f64_e64 s1, 0x40191000, |v[2:3]|
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr4_vgpr5
	s_and_saveexec_b32 s4, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s1, exec_lo, s4
	s_cbranch_execz .LBB11_8
; %bb.3:
	s_getpc_b64 s[4:5]
	s_add_u32 s4, s4, __ocmltbl_M64_J0@rel32@lo+4
	s_addc_u32 s5, s5, __ocmltbl_M64_J0@rel32@hi+12
	v_mov_b32_e32 v8, 0
	v_dual_mov_b32 v9, 0 :: v_dual_mov_b32 v4, 0
	v_mov_b32_e32 v5, 0
	v_dual_mov_b32 v7, s5 :: v_dual_mov_b32 v6, s4
	s_mov_b32 s4, exec_lo
	v_cmpx_nge_f64_e64 0x3ffa8000, |v[2:3]|
	s_cbranch_execz .LBB11_7
; %bb.4:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J0@rel32@lo+124
	s_addc_u32 s7, s7, __ocmltbl_M64_J0@rel32@hi+132
	v_mov_b32_e32 v4, 0xd7da258e
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0xbca0f539
	v_mov_b32_e32 v8, 0x2e971b40
	v_mov_b32_e32 v9, 0x40033d15
	s_mov_b32 s5, exec_lo
	v_cmpx_nge_f64_e64 0x40090000, |v[2:3]|
	s_cbranch_execz .LBB11_6
; %bb.5:
	v_cmp_nge_f64_e64 vcc_lo, 0x4012c000, |v[2:3]|
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J0@rel32@lo+364
	s_addc_u32 s7, s7, __ocmltbl_M64_J0@rel32@hi+372
	v_dual_mov_b32 v4, 0x3c975054 :: v_dual_mov_b32 v11, s6
	v_mov_b32_e32 v6, 0xcd60a517
	v_dual_mov_b32 v7, 0x4016148f :: v_dual_mov_b32 v10, s7
	v_mov_b32_e32 v8, 0x5b2c2e45
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J0@rel32@lo+244
	s_addc_u32 s9, s9, __ocmltbl_M64_J0@rel32@hi+252
	v_cndmask_b32_e32 v5, 0xbca60155, v4, vcc_lo
	v_cndmask_b32_e32 v4, 0xa9d1b256, v6, vcc_lo
	v_cndmask_b32_e32 v9, 0x400ea755, v7, vcc_lo
	v_cndmask_b32_e32 v8, 0x75af6f09, v8, vcc_lo
	v_cndmask_b32_e32 v7, s9, v10, vcc_lo
	v_cndmask_b32_e32 v6, s8, v11, vcc_lo
.LBB11_6:
	s_or_b32 exec_lo, exec_lo, s5
.LBB11_7:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
.LBB11_8:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB11_14
; %bb.9:
	s_getpc_b64 s[4:5]
	s_add_u32 s4, s4, __ocmltbl_M64_J0@rel32@lo+484
	s_addc_u32 s5, s5, __ocmltbl_M64_J0@rel32@hi+492
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v4, 0x9d243827 :: v_dual_mov_b32 v7, s5
	v_dual_mov_b32 v6, s4 :: v_dual_mov_b32 v5, 0xbc9b226d
	v_mov_b32_e32 v8, 0xf3b47250
	v_mov_b32_e32 v9, 0x401c0ff5
	s_mov_b32 s4, exec_lo
	v_cmpx_nge_f64_e64 0x401f6000, |v[2:3]|
	s_cbranch_execz .LBB11_13
; %bb.10:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J0@rel32@lo+604
	s_addc_u32 s7, s7, __ocmltbl_M64_J0@rel32@hi+612
	v_mov_b32_e32 v4, 0x714c7c25
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0xbcb51970
	v_mov_b32_e32 v8, 0x6cccdeca
	v_mov_b32_e32 v9, 0x40214eb5
	s_mov_b32 s5, exec_lo
	v_cmpx_nge_f64_e64 0x4022d800, |v[2:3]|
	s_cbranch_execz .LBB11_12
; %bb.11:
	v_cmp_nge_f64_e64 vcc_lo, 0x4025f800, |v[2:3]|
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J0@rel32@lo+844
	s_addc_u32 s7, s7, __ocmltbl_M64_J0@rel32@hi+852
	v_dual_mov_b32 v4, 0x3cb444fd :: v_dual_mov_b32 v11, s6
	v_mov_b32_e32 v6, 0x5821d5b1
	v_dual_mov_b32 v7, 0x40279544 :: v_dual_mov_b32 v10, s7
	v_mov_b32_e32 v8, 0x8272b6
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J0@rel32@lo+724
	s_addc_u32 s9, s9, __ocmltbl_M64_J0@rel32@hi+732
	v_cndmask_b32_e32 v5, 0x3cc02610, v4, vcc_lo
	v_cndmask_b32_e32 v4, 0xa51562b6, v6, vcc_lo
	v_cndmask_b32_e32 v9, 0x402458d0, v7, vcc_lo
	v_cndmask_b32_e32 v8, 0xd0bdfc29, v8, vcc_lo
	v_cndmask_b32_e32 v7, s9, v10, vcc_lo
	v_cndmask_b32_e32 v6, s8, v11, vcc_lo
.LBB11_12:
	s_or_b32 exec_lo, exec_lo, s5
.LBB11_13:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
.LBB11_14:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s1
	s_clause 0x4
	global_load_b64 v[26:27], v[6:7], off offset:112
	global_load_b128 v[10:13], v[6:7], off offset:96
	global_load_b128 v[14:17], v[6:7], off offset:80
	global_load_b128 v[18:21], v[6:7], off offset:64
	global_load_b128 v[22:25], v[6:7], off offset:48
	v_add_f64 v[2:3], |v[2:3]|, -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_f64 v[28:29], v[2:3], -v[4:5]
	global_load_b128 v[2:5], v[6:7], off offset:32
	s_waitcnt vmcnt(4)
	v_fma_f64 v[8:9], v[28:29], v[26:27], v[12:13]
	v_fma_f64 v[12:13], v[28:29], v[8:9], v[10:11]
	global_load_b128 v[8:11], v[6:7], off offset:16
	s_waitcnt vmcnt(4)
	v_fma_f64 v[12:13], v[28:29], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[28:29], v[12:13], v[14:15]
	global_load_b128 v[12:15], v[6:7], off
	s_waitcnt vmcnt(4)
	v_fma_f64 v[6:7], v[28:29], v[16:17], v[20:21]
	v_fma_f64 v[6:7], v[28:29], v[6:7], v[18:19]
	s_waitcnt vmcnt(3)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[28:29], v[6:7], v[24:25]
	v_fma_f64 v[6:7], v[28:29], v[6:7], v[22:23]
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[28:29], v[6:7], v[4:5]
	v_fma_f64 v[2:3], v[28:29], v[4:5], v[2:3]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[28:29], v[2:3], v[10:11]
	v_fma_f64 v[2:3], v[28:29], v[2:3], v[8:9]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[28:29], v[2:3], v[14:15]
	v_fma_f64 v[4:5], v[28:29], v[2:3], v[12:13]
                                        ; implicit-def: $vgpr2_vgpr3
.LBB11_15:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB11_21
; %bb.16:
	v_cmp_ngt_f64_e64 s1, 0x41d00000, |v[2:3]|
	v_and_b32_e32 v5, 0x7fffffff, v3
                                        ; implicit-def: $vgpr10
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
	s_and_saveexec_b32 s4, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s1, exec_lo, s4
	s_cbranch_execz .LBB11_18
; %bb.17:
	v_ldexp_f64 v[6:7], |v[2:3]|, 0xffffff80
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_trig_preop_f64 v[8:9], |v[2:3]|, 0
	v_trig_preop_f64 v[10:11], |v[2:3]|, 1
	v_trig_preop_f64 v[20:21], |v[2:3]|, 2
	v_mov_b32_e32 v28, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_dual_cndmask_b32 v7, v5, v7 :: v_dual_cndmask_b32 v6, v2, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[12:13], v[8:9], v[6:7]
	v_mul_f64 v[14:15], v[10:11], v[6:7]
	v_mul_f64 v[26:27], v[20:21], v[6:7]
	v_fma_f64 v[8:9], v[8:9], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[6:7], -v[14:15]
	v_fma_f64 v[6:7], v[20:21], v[6:7], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[14:15], v[8:9]
	v_add_f64 v[18:19], v[16:17], -v[14:15]
	v_add_f64 v[24:25], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], -v[18:19]
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	v_ldexp_f64 v[18:19], v[24:25], -2
	v_add_f64 v[12:13], v[24:25], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_f64 v[22:23], v[26:27], v[10:11]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[18:19]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[16:17], -v[12:13]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_fract_f64_e32 v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[22:23], v[8:9]
	v_ldexp_f64 v[14:15], v[14:15], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[12:13], v[16:17]
	v_dual_cndmask_b32 v15, 0, v15 :: v_dual_cndmask_b32 v14, 0, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[18:19], v[14:15]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[24:25]
	v_add_f64 v[24:25], v[22:23], -v[26:27]
	v_cndmask_b32_e64 v29, 0, 0x40100000, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[33:34], v[22:23], -v[24:25]
	v_add_f64 v[10:11], v[10:11], -v[24:25]
	v_add_f64 v[14:15], v[14:15], v[28:29]
	v_add_f64 v[29:30], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[26:27], -v[33:34]
	v_add_f64 v[31:32], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[35:36], v[16:17], -v[29:30]
	v_add_f64 v[8:9], v[8:9], -v[29:30]
	v_add_f64 v[10:11], v[10:11], v[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_i32_f64_e32 v4, v[31:32]
	v_add_f64 v[22:23], v[22:23], -v[35:36]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_f64_i32_e32 v[29:30], v4
	v_add_f64 v[8:9], v[8:9], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[29:30]
	v_add_f64 v[8:9], v[10:11], v[8:9]
	v_add_f64 v[10:11], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[18:19], v[14:15]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[20:21], -v[14:15]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[20:21]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[18:19], -v[8:9]
	v_cndmask_b32_e64 v29, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v10, null, 0, v4, vcc_lo
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[20:21], -v[28:29]
	v_add_f64 v[11:12], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[11:12], s[4:5]
	v_add_f64 v[8:9], v[11:12], -v[8:9]
	v_fma_f64 v[15:16], v[11:12], s[4:5], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[11:12], s[6:7], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[4:5], v[8:9]
	v_add_f64 v[6:7], v[13:14], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[6:7], -v[13:14]
	v_add_f64 v[8:9], v[8:9], -v[11:12]
.LBB11_18:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB11_20
; %bb.19:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[6:7], |v[2:3]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[6:7]
	v_fma_f64 v[6:7], v[10:11], s[4:5], |v[2:3]|
	v_mul_f64 v[8:9], v[10:11], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], s[6:7], v[6:7]
	v_add_f64 v[12:13], v[6:7], v[8:9]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], v[10:11], s[6:7], v[8:9]
	v_add_f64 v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[10:11], s[4:5], v[6:7]
	v_cvt_i32_f64_e32 v10, v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[14:15], v[8:9]
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
.LBB11_20:
	s_or_b32 exec_lo, exec_lo, s1
	v_mov_b32_e32 v4, v2
	s_mov_b32 s4, 0x923b70a7
	s_mov_b32 s6, 0xa4a989b
	s_mov_b32 s5, 0x41752a41
	s_mov_b32 s7, 0xc1b91f78
	v_div_scale_f64 v[11:12], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[4:5], vcc_lo, 1.0, v[4:5], 1.0
	s_mov_b32 s8, 0x796cde01
	s_mov_b32 s9, 0x3ec71de3
	v_rcp_f64_e32 v[13:14], v[11:12]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[15:16], -v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[13:14], v[15:16], v[13:14]
	v_fma_f64 v[15:16], -v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[13:14], v[15:16], v[13:14]
	v_mul_f64 v[15:16], v[4:5], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[11:12], v[15:16], v[4:5]
	v_div_fmas_f64 v[4:5], v[4:5], v[13:14], v[15:16]
	v_mov_b32_e32 v15, 0x54442d18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[4:5], |v[2:3]|, 1.0
	v_mul_f64 v[11:12], v[4:5], v[4:5]
	v_rsq_f64_e32 v[21:22], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], s[6:7], s[4:5]
	s_mov_b32 s4, 0x31612a8d
	s_mov_b32 s5, 0xc1240a5e
	s_mov_b32 s6, 0xcd7ac32c
	s_mov_b32 s7, 0x41344395
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[27:28], v[4:5], v[21:22]
	v_mul_f64 v[21:22], v[21:22], 0.5
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xcbe3b3b8
	s_mov_b32 s5, 0x40d0c9a0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], -v[21:22], v[27:28], 0.5
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x167fe583
	s_mov_b32 s5, 0xc080af76
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[27:28], v[27:28], v[29:30], v[27:28]
	v_fma_f64 v[21:22], v[21:22], v[29:30], v[21:22]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x61b94139
	s_mov_b32 s5, 0x403778ea
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], -v[27:28], v[27:28], v[4:5]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xd1a82662
	s_mov_b32 s5, 0xbffa3581
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x30a1daf2
	s_mov_b32 s5, 0x3fcad333
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xaaaa7909
	s_mov_b32 s5, 0xbfb0aaaa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xabbee803
	s_mov_b32 s5, 0xc0f25bf3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], 0x3fc00000
	v_mul_f64 v[13:14], v[4:5], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cmp_lt_f64_e32 vcc_lo, v[6:7], v[13:14]
	v_subrev_co_ci_u32_e64 v37, null, 0, v10, vcc_lo
	v_cndmask_b32_e64 v10, 0, 0x80000000, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, 0, v[4:5]
	v_xor_b32_e32 v16, 0xbfe921fb, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[15:16], -v[13:14]
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_mov_b32_e32 v15, 0x33145c07
	v_xor_b32_e32 v16, 0xbc81a626, v10
	v_add_f64 v[13:14], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[17:18], v[13:14]
	v_add_f64 v[19:20], v[6:7], v[15:16]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[19:20], -v[15:16]
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[15:16]
	v_add_f64 v[8:9], v[8:9], v[13:14]
	v_fma_f64 v[15:16], v[11:12], s[6:7], s[4:5]
	s_mov_b32 s4, 0xb42fdfa7
	s_mov_b32 s6, 0xf9a43bb8
	s_mov_b32 s5, 0xbe5ae600
	s_mov_b32 s7, 0x3de5e0b2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[19:20], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[8:9], v[8:9]
	v_add_f64 v[19:20], v[8:9], -v[19:20]
	v_fma_f64 v[17:18], v[13:14], s[6:7], s[4:5]
	s_mov_b32 s4, 0x78625b0f
	s_mov_b32 s5, 0x40a55a4a
	s_mov_b32 s6, 0x46cc5e42
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x9037ab78
	s_mov_b32 s5, 0x3e21eeb6
	s_mov_b32 s7, 0xbda907db
	v_mul_f64 v[25:26], v[13:14], 0.5
	v_fma_f64 v[23:24], v[13:14], s[6:7], s[4:5]
	s_mov_b32 s4, 0x7ea56321
	s_mov_b32 s5, 0xc05a826c
	s_mov_b32 s6, 0x19e83e5c
	s_mov_b32 s7, 0xbf2a01a0
	v_add_f64 v[6:7], v[6:7], -v[19:20]
	v_mul_f64 v[31:32], v[8:9], -v[13:14]
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[8:9]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0xa17f65f6
	s_mov_b32 s5, 0xbe927e4f
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[23:24], s[4:5]
	v_add_f64 v[23:24], -v[25:26], 1.0
	s_mov_b32 s4, 0x3bbf53b6
	s_mov_b32 s5, 0x40176325
	v_mul_f64 v[33:34], v[6:7], 0.5
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0x11110bb3
	s_mov_b32 s7, 0x3f811111
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x19f4ec90
	s_mov_b32 s5, 0x3efa01a0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	v_add_f64 v[35:36], -v[23:24], 1.0
	s_mov_b32 s4, 0xff948953
	s_mov_b32 s5, 0xbfe15efa
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0xffff2868
	s_mov_b32 s7, 0xbfafffff
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x16c16967
	s_mov_b32 s5, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_add_f64 v[25:26], v[35:36], -v[25:26]
	s_mov_b32 s4, 0xf967a1d4
	s_mov_b32 s5, 0x3fba7fff
	v_fma_f64 v[17:18], v[31:32], v[17:18], v[33:34]
	v_mul_f64 v[33:34], v[13:14], v[13:14]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s5, 0x3fa55555
	s_mov_b32 s4, 0x55555555
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], v[17:18], -v[6:7]
	v_fma_f64 v[13:14], v[13:14], v[19:20], s[4:5]
	v_fma_f64 v[6:7], v[8:9], -v[6:7], v[25:26]
	v_fma_f64 v[19:20], v[29:30], v[21:22], v[27:28]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[6:7]
	s_mov_b32 s5, 0xbfc55555
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[17:18], v[31:32], s[4:5], v[17:18]
	s_mov_b32 s4, 0x33d43651
	v_fma_f64 v[6:7], v[33:34], v[13:14], v[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_dual_cndmask_b32 v5, v20, v5 :: v_dual_cndmask_b32 v4, v19, v4
	s_mov_b32 s5, 0x3fe98845
	v_fma_f64 v[10:11], v[11:12], v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[4:5], v[4:5], s[4:5]
	v_add_f64 v[8:9], v[8:9], -v[17:18]
	v_add_f64 v[6:7], v[23:24], v[6:7]
	v_mul_f64 v[4:5], v[4:5], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v9, 0x80000000, v9
	v_and_b32_e32 v10, 1, v37
	v_cmp_eq_u32_e32 vcc_lo, 0, v10
	v_dual_cndmask_b32 v6, v8, v6 :: v_dual_lshlrev_b32 v11, 30, v37
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_and_b32_e32 v10, 0x80000000, v11
	v_cndmask_b32_e32 v7, v9, v7, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	v_xor_b32_e32 v7, v7, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], v[6:7]
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
.LBB11_21:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB11_22:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_j0_kernel
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
		.amdhsa_next_free_vgpr 38
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
		.amdhsa_inst_pref_size 27
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
	.size	specialx_j0_kernel, .Lfunc_end11-specialx_j0_kernel
                                        ; -- End function
	.set specialx_j0_kernel.num_vgpr, 38
	.set specialx_j0_kernel.num_agpr, 0
	.set specialx_j0_kernel.numbered_sgpr, 10
	.set specialx_j0_kernel.num_named_barrier, 0
	.set specialx_j0_kernel.private_seg_size, 0
	.set specialx_j0_kernel.uses_vcc, 1
	.set specialx_j0_kernel.uses_flat_scratch, 0
	.set specialx_j0_kernel.has_dyn_sized_stack, 0
	.set specialx_j0_kernel.has_recursion, 0
	.set specialx_j0_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3416
; TotalNumSgprs: 12
; NumVgprs: 38
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 38
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
	.protected	specialx_j1_kernel      ; -- Begin function specialx_j1_kernel
	.globl	specialx_j1_kernel
	.p2align	8
	.type	specialx_j1_kernel,@function
specialx_j1_kernel:                     ; @specialx_j1_kernel
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
	s_cbranch_execz .LBB12_22
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_ge_f64_e64 s0, 0x40290800, |v[2:3]|
	s_and_saveexec_b32 s1, s0
	s_xor_b32 s0, exec_lo, s1
	s_cbranch_execz .LBB12_15
; %bb.2:
	v_cmp_ge_f64_e64 s1, 0x4018b000, |v[2:3]|
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr4_vgpr5
	s_and_saveexec_b32 s4, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s1, exec_lo, s4
	s_cbranch_execz .LBB12_8
; %bb.3:
	s_getpc_b64 s[4:5]
	s_add_u32 s4, s4, __ocmltbl_M64_J1@rel32@lo+4
	s_addc_u32 s5, s5, __ocmltbl_M64_J1@rel32@hi+12
	v_mov_b32_e32 v8, 0
	v_dual_mov_b32 v9, 0 :: v_dual_mov_b32 v4, 0
	v_mov_b32_e32 v5, 0
	v_dual_mov_b32 v7, s5 :: v_dual_mov_b32 v6, s4
	s_mov_b32 s4, exec_lo
	v_cmpx_nge_f64_e64 0x3ff18000, |v[2:3]|
	s_cbranch_execz .LBB12_7
; %bb.4:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J1@rel32@lo+124
	s_addc_u32 s7, s7, __ocmltbl_M64_J1@rel32@hi+132
	v_mov_b32_e32 v4, 0x20cfdaeb
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0x3c5616d8
	v_mov_b32_e32 v8, 0x1fec8a3a
	v_mov_b32_e32 v9, 0x3ffd757d
	s_mov_b32 s5, exec_lo
	v_cmpx_nge_f64_e64 0x4006c000, |v[2:3]|
	s_cbranch_execz .LBB12_6
; %bb.5:
	v_cmp_nge_f64_e64 vcc_lo, 0x40125000, |v[2:3]|
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J1@rel32@lo+364
	s_addc_u32 s7, s7, __ocmltbl_M64_J1@rel32@hi+372
	v_dual_mov_b32 v4, 0x3ca5c646 :: v_dual_mov_b32 v11, s6
	v_mov_b32_e32 v6, 0xa75d7539
	v_dual_mov_b32 v7, 0x40155365 :: v_dual_mov_b32 v10, s7
	v_mov_b32_e32 v8, 0xbc032467
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J1@rel32@lo+244
	s_addc_u32 s9, s9, __ocmltbl_M64_J1@rel32@hi+252
	v_cndmask_b32_e32 v5, 0xbca60155, v4, vcc_lo
	v_cndmask_b32_e32 v4, 0xa9d1b256, v6, vcc_lo
	v_cndmask_b32_e32 v9, 0x400ea755, v7, vcc_lo
	v_cndmask_b32_e32 v8, 0x75af6f09, v8, vcc_lo
	v_cndmask_b32_e32 v7, s9, v10, vcc_lo
	v_cndmask_b32_e32 v6, s8, v11, vcc_lo
.LBB12_6:
	s_or_b32 exec_lo, exec_lo, s5
.LBB12_7:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
.LBB12_8:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB12_14
; %bb.9:
	s_getpc_b64 s[4:5]
	s_add_u32 s4, s4, __ocmltbl_M64_J1@rel32@lo+484
	s_addc_u32 s5, s5, __ocmltbl_M64_J1@rel32@hi+492
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v4, 0x9d243827 :: v_dual_mov_b32 v7, s5
	v_dual_mov_b32 v6, s4 :: v_dual_mov_b32 v5, 0xbc9b226d
	v_mov_b32_e32 v8, 0xf3b47250
	v_mov_b32_e32 v9, 0x401c0ff5
	s_mov_b32 s4, exec_lo
	v_cmpx_nge_f64_e64 0x401f2000, |v[2:3]|
	s_cbranch_execz .LBB12_13
; %bb.10:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J1@rel32@lo+604
	s_addc_u32 s7, s7, __ocmltbl_M64_J1@rel32@hi+612
	v_mov_b32_e32 v4, 0xec20a31d
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0xbca63e17
	v_mov_b32_e32 v8, 0xf0b88a1
	v_mov_b32_e32 v9, 0x40211298
	s_mov_b32 s5, exec_lo
	v_cmpx_nge_f64_e64 0x4022b800, |v[2:3]|
	s_cbranch_execz .LBB12_12
; %bb.11:
	v_cmp_nge_f64_e64 vcc_lo, 0x4025e800, |v[2:3]|
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J1@rel32@lo+844
	s_addc_u32 s7, s7, __ocmltbl_M64_J1@rel32@hi+852
	v_dual_mov_b32 v4, 0x3cc9a84d :: v_dual_mov_b32 v11, s6
	v_mov_b32_e32 v6, 0x3a5fedc2
	v_dual_mov_b32 v7, 0x40276979 :: v_dual_mov_b32 v10, s7
	v_mov_b32_e32 v8, 0x797ee5ac
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J1@rel32@lo+724
	s_addc_u32 s9, s9, __ocmltbl_M64_J1@rel32@hi+732
	v_cndmask_b32_e32 v5, 0x3cc02610, v4, vcc_lo
	v_cndmask_b32_e32 v4, 0xa51562b6, v6, vcc_lo
	v_cndmask_b32_e32 v9, 0x402458d0, v7, vcc_lo
	v_cndmask_b32_e32 v8, 0xd0bdfc29, v8, vcc_lo
	v_cndmask_b32_e32 v7, s9, v10, vcc_lo
	v_cndmask_b32_e32 v6, s8, v11, vcc_lo
.LBB12_12:
	s_or_b32 exec_lo, exec_lo, s5
.LBB12_13:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
.LBB12_14:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s1
	s_clause 0x4
	global_load_b64 v[30:31], v[6:7], off offset:112
	global_load_b128 v[10:13], v[6:7], off offset:96
	global_load_b128 v[14:17], v[6:7], off offset:80
	global_load_b128 v[18:21], v[6:7], off offset:64
	global_load_b128 v[22:25], v[6:7], off offset:48
	v_add_f64 v[8:9], |v[2:3]|, -v[8:9]
	global_load_b128 v[26:29], v[6:7], off offset:32
	v_add_f64 v[32:33], v[8:9], -v[4:5]
	s_waitcnt vmcnt(4)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[32:33], v[30:31], v[12:13]
	v_fma_f64 v[4:5], v[32:33], v[4:5], v[10:11]
	global_load_b128 v[8:11], v[6:7], off offset:16
	s_waitcnt vmcnt(4)
	v_fma_f64 v[4:5], v[32:33], v[4:5], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[32:33], v[4:5], v[14:15]
	global_load_b128 v[4:7], v[6:7], off
	s_waitcnt vmcnt(4)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[20:21]
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[18:19]
	s_waitcnt vmcnt(3)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[24:25]
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[22:23]
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[28:29]
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[26:27]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[32:33], v[12:13], v[10:11]
	v_fma_f64 v[8:9], v[32:33], v[10:11], v[8:9]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[32:33], v[8:9], v[6:7]
	v_fma_f64 v[4:5], v[32:33], v[6:7], v[4:5]
.LBB12_15:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB12_21
; %bb.16:
	v_cmp_ngt_f64_e64 s1, 0x41d00000, |v[2:3]|
	v_and_b32_e32 v5, 0x7fffffff, v3
                                        ; implicit-def: $vgpr10
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
	s_and_saveexec_b32 s4, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s1, exec_lo, s4
	s_cbranch_execz .LBB12_18
; %bb.17:
	v_ldexp_f64 v[6:7], |v[2:3]|, 0xffffff80
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_trig_preop_f64 v[8:9], |v[2:3]|, 0
	v_trig_preop_f64 v[10:11], |v[2:3]|, 1
	v_trig_preop_f64 v[20:21], |v[2:3]|, 2
	v_mov_b32_e32 v28, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_dual_cndmask_b32 v7, v5, v7 :: v_dual_cndmask_b32 v6, v2, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[12:13], v[8:9], v[6:7]
	v_mul_f64 v[14:15], v[10:11], v[6:7]
	v_mul_f64 v[26:27], v[20:21], v[6:7]
	v_fma_f64 v[8:9], v[8:9], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[6:7], -v[14:15]
	v_fma_f64 v[6:7], v[20:21], v[6:7], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[14:15], v[8:9]
	v_add_f64 v[18:19], v[16:17], -v[14:15]
	v_add_f64 v[24:25], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], -v[18:19]
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	v_ldexp_f64 v[18:19], v[24:25], -2
	v_add_f64 v[12:13], v[24:25], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_f64 v[22:23], v[26:27], v[10:11]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[18:19]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[16:17], -v[12:13]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_fract_f64_e32 v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[22:23], v[8:9]
	v_ldexp_f64 v[14:15], v[14:15], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[12:13], v[16:17]
	v_dual_cndmask_b32 v15, 0, v15 :: v_dual_cndmask_b32 v14, 0, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[18:19], v[14:15]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[24:25]
	v_add_f64 v[24:25], v[22:23], -v[26:27]
	v_cndmask_b32_e64 v29, 0, 0x40100000, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[33:34], v[22:23], -v[24:25]
	v_add_f64 v[10:11], v[10:11], -v[24:25]
	v_add_f64 v[14:15], v[14:15], v[28:29]
	v_add_f64 v[29:30], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[26:27], -v[33:34]
	v_add_f64 v[31:32], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[35:36], v[16:17], -v[29:30]
	v_add_f64 v[8:9], v[8:9], -v[29:30]
	v_add_f64 v[10:11], v[10:11], v[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_i32_f64_e32 v4, v[31:32]
	v_add_f64 v[22:23], v[22:23], -v[35:36]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_f64_i32_e32 v[29:30], v4
	v_add_f64 v[8:9], v[8:9], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[29:30]
	v_add_f64 v[8:9], v[10:11], v[8:9]
	v_add_f64 v[10:11], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[18:19], v[14:15]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[20:21], -v[14:15]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[20:21]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[18:19], -v[8:9]
	v_cndmask_b32_e64 v29, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v10, null, 0, v4, vcc_lo
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[20:21], -v[28:29]
	v_add_f64 v[11:12], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[11:12], s[4:5]
	v_add_f64 v[8:9], v[11:12], -v[8:9]
	v_fma_f64 v[15:16], v[11:12], s[4:5], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[11:12], s[6:7], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[4:5], v[8:9]
	v_add_f64 v[6:7], v[13:14], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[6:7], -v[13:14]
	v_add_f64 v[8:9], v[8:9], -v[11:12]
.LBB12_18:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB12_20
; %bb.19:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[6:7], |v[2:3]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[6:7]
	v_fma_f64 v[6:7], v[10:11], s[4:5], |v[2:3]|
	v_mul_f64 v[8:9], v[10:11], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], s[6:7], v[6:7]
	v_add_f64 v[12:13], v[6:7], v[8:9]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], v[10:11], s[6:7], v[8:9]
	v_add_f64 v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[10:11], s[4:5], v[6:7]
	v_cvt_i32_f64_e32 v10, v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[14:15], v[8:9]
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
.LBB12_20:
	s_or_b32 exec_lo, exec_lo, s1
	v_mov_b32_e32 v4, v2
	s_mov_b32 s4, 0x95ed3e8e
	s_mov_b32 s6, 0x53d3a76e
	s_mov_b32 s5, 0xc1780a4d
	s_mov_b32 s7, 0x41bc22f6
	v_div_scale_f64 v[11:12], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[4:5], vcc_lo, 1.0, v[4:5], 1.0
	s_mov_b32 s8, 0x796cde01
	s_mov_b32 s9, 0x3ec71de3
	v_rcp_f64_e32 v[13:14], v[11:12]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[15:16], -v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[13:14], v[15:16], v[13:14]
	v_fma_f64 v[15:16], -v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[13:14], v[15:16], v[13:14]
	v_mul_f64 v[15:16], v[4:5], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[11:12], v[15:16], v[4:5]
	v_div_fmas_f64 v[4:5], v[4:5], v[13:14], v[15:16]
	v_mov_b32_e32 v15, 0x54442d18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[4:5], |v[2:3]|, 1.0
	v_mul_f64 v[11:12], v[4:5], v[4:5]
	v_rsq_f64_e32 v[21:22], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], s[6:7], s[4:5]
	s_mov_b32 s4, 0x1f8cdd76
	s_mov_b32 s5, 0x41272f1d
	s_mov_b32 s6, 0x6621145
	s_mov_b32 s7, 0xc137940a
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[27:28], v[4:5], v[21:22]
	v_mul_f64 v[21:22], v[21:22], 0.5
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x96460ad7
	s_mov_b32 s5, 0xc0d3ea4e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], -v[21:22], v[27:28], 0.5
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x98d9ab3a
	s_mov_b32 s5, 0x408488dd
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[27:28], v[27:28], v[29:30], v[27:28]
	v_fma_f64 v[21:22], v[21:22], v[29:30], v[21:22]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x12fa3b38
	s_mov_b32 s5, 0xc03e9ed6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], -v[27:28], v[27:28], v[4:5]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xfcab9dda
	s_mov_b32 s5, 0x4002f484
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xcad443c0
	s_mov_b32 s5, 0xbfd7bccc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_movk_i32 s4, 0xcbfa
	s_mov_b32 s5, 0x3fc4ffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x68428baf
	s_mov_b32 s5, 0x40f591fb
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], 0xbfd80000
	v_mul_f64 v[13:14], v[4:5], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cmp_lt_f64_e32 vcc_lo, v[6:7], v[13:14]
	v_subrev_co_ci_u32_e64 v37, null, 0, v10, vcc_lo
	v_cndmask_b32_e64 v10, 0, 0x80000000, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, 0, v[4:5]
	v_xor_b32_e32 v16, 0xbfe921fb, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[15:16], -v[13:14]
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_mov_b32_e32 v15, 0x33145c07
	v_xor_b32_e32 v16, 0xbc81a626, v10
	v_add_f64 v[13:14], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[17:18], v[13:14]
	v_add_f64 v[19:20], v[6:7], v[15:16]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[19:20], -v[15:16]
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[15:16]
	v_add_f64 v[8:9], v[8:9], v[13:14]
	v_fma_f64 v[15:16], v[11:12], s[6:7], s[4:5]
	s_mov_b32 s4, 0xb42fdfa7
	s_mov_b32 s6, 0xf9a43bb8
	s_mov_b32 s5, 0xbe5ae600
	s_mov_b32 s7, 0x3de5e0b2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[19:20], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[8:9], v[8:9]
	v_add_f64 v[19:20], v[8:9], -v[19:20]
	v_fma_f64 v[17:18], v[13:14], s[6:7], s[4:5]
	s_mov_b32 s4, 0x2a8bafb0
	s_mov_b32 s5, 0xc0a99655
	s_mov_b32 s6, 0x46cc5e42
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x9037ab78
	s_mov_b32 s5, 0x3e21eeb6
	s_mov_b32 s7, 0xbda907db
	v_mul_f64 v[25:26], v[13:14], 0.5
	v_fma_f64 v[23:24], v[13:14], s[6:7], s[4:5]
	s_mov_b32 s4, 0x78cd8c93
	s_mov_b32 s5, 0x40607955
	s_mov_b32 s6, 0x19e83e5c
	s_mov_b32 s7, 0xbf2a01a0
	v_add_f64 v[6:7], v[6:7], -v[19:20]
	v_mul_f64 v[31:32], v[8:9], -v[13:14]
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[8:9]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0xa17f65f6
	s_mov_b32 s5, 0xbe927e4f
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[23:24], s[4:5]
	v_add_f64 v[23:24], -v[25:26], 1.0
	s_mov_b32 s4, 0x64596b5a
	s_mov_b32 s5, 0xc01ef383
	v_mul_f64 v[33:34], v[6:7], 0.5
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0x11110bb3
	s_mov_b32 s7, 0x3f811111
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x19f4ec90
	s_mov_b32 s5, 0x3efa01a0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	v_add_f64 v[35:36], -v[23:24], 1.0
	s_mov_b32 s4, 0x465744c7
	s_mov_b32 s5, 0x3fe9c4fa
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_movk_i32 s6, 0xc240
	s_mov_b32 s7, 0x3fc7ffff
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x16c16967
	s_mov_b32 s5, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_add_f64 v[25:26], v[35:36], -v[25:26]
	s_mov_b32 s4, 0xfc3937c1
	s_mov_b32 s5, 0xbfc8bfff
	v_fma_f64 v[17:18], v[31:32], v[17:18], v[33:34]
	v_mul_f64 v[33:34], v[13:14], v[13:14]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s5, 0x3fa55555
	s_mov_b32 s4, 0x55555555
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], v[17:18], -v[6:7]
	v_fma_f64 v[13:14], v[13:14], v[19:20], s[4:5]
	v_fma_f64 v[6:7], v[8:9], -v[6:7], v[25:26]
	v_fma_f64 v[19:20], v[29:30], v[21:22], v[27:28]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[6:7]
	s_mov_b32 s5, 0xbfc55555
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[17:18], v[31:32], s[4:5], v[17:18]
	s_mov_b32 s4, 0x33d43651
	v_fma_f64 v[6:7], v[33:34], v[13:14], v[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_dual_cndmask_b32 v5, v20, v5 :: v_dual_cndmask_b32 v4, v19, v4
	s_mov_b32 s5, 0x3fe98845
	v_fma_f64 v[10:11], v[11:12], v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[4:5], v[4:5], s[4:5]
	v_add_f64 v[8:9], v[8:9], -v[17:18]
	v_add_f64 v[6:7], v[23:24], v[6:7]
	v_mul_f64 v[4:5], v[4:5], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v9, 0x80000000, v9
	v_add_nc_u32_e32 v10, -1, v37
	v_and_b32_e32 v11, 1, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_eq_u32_e32 vcc_lo, 0, v11
	v_dual_cndmask_b32 v7, v9, v7 :: v_dual_lshlrev_b32 v10, 30, v10
	v_cndmask_b32_e32 v6, v8, v6, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b32_e32 v10, 0x80000000, v10
	v_xor_b32_e32 v7, v7, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], v[6:7]
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
.LBB12_21:
	s_or_b32 exec_lo, exec_lo, s0
	v_cmp_gt_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v6, 0x80000000, v5
	v_cndmask_b32_e32 v5, v5, v6, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB12_22:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_j1_kernel
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
		.amdhsa_next_free_vgpr 38
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
		.amdhsa_inst_pref_size 27
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
	.size	specialx_j1_kernel, .Lfunc_end12-specialx_j1_kernel
                                        ; -- End function
	.set specialx_j1_kernel.num_vgpr, 38
	.set specialx_j1_kernel.num_agpr, 0
	.set specialx_j1_kernel.numbered_sgpr, 10
	.set specialx_j1_kernel.num_named_barrier, 0
	.set specialx_j1_kernel.private_seg_size, 0
	.set specialx_j1_kernel.uses_vcc, 1
	.set specialx_j1_kernel.uses_flat_scratch, 0
	.set specialx_j1_kernel.has_dyn_sized_stack, 0
	.set specialx_j1_kernel.has_recursion, 0
	.set specialx_j1_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 3436
; TotalNumSgprs: 12
; NumVgprs: 38
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 38
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
	.protected	specialx_y0_kernel      ; -- Begin function specialx_y0_kernel
	.globl	specialx_y0_kernel
	.p2align	8
	.type	specialx_y0_kernel,@function
specialx_y0_kernel:                     ; @specialx_y0_kernel
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
	s_cbranch_execz .LBB13_64
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f64_e32 0x40291800, v[2:3]
	s_xor_b32 s0, exec_lo, s0
	s_cbranch_execz .LBB13_57
; %bb.2:
	s_mov_b32 s1, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr4_vgpr5
	v_cmpx_ngt_f64_e32 2.0, v[2:3]
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB13_18
; %bb.3:
	s_mov_b32 s4, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr4_vgpr5
	v_cmpx_ngt_f64_e32 0x40191000, v[2:3]
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB13_9
; %bb.4:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y0@rel32@lo+1684
	s_addc_u32 s7, s7, __ocmltbl_M64_Y0@rel32@hi+1692
	v_mov_b32_e32 v4, 0x495f56cf
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0xbc99774a
	v_mov_b32_e32 v8, 0xc4e72103
	v_mov_b32_e32 v9, 0x401c581d
	s_mov_b32 s5, exec_lo
	v_cmpx_ngt_f64_e32 0x401f6000, v[2:3]
	s_cbranch_execz .LBB13_8
; %bb.5:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y0@rel32@lo+1804
	s_addc_u32 s7, s7, __ocmltbl_M64_Y0@rel32@hi+1812
	v_mov_b32_e32 v4, 0x68d9046
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0x3cb479cc
	v_mov_b32_e32 v8, 0xae6169b4
	v_mov_b32_e32 v9, 0x40213127
	s_mov_b32 s6, exec_lo
	v_cmpx_ngt_f64_e32 0x4022d800, v[2:3]
	s_cbranch_execz .LBB13_7
; %bb.6:
	v_cmp_gt_f64_e32 vcc_lo, 0x4025f800, v[2:3]
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y0@rel32@lo+1924
	s_addc_u32 s9, s9, __ocmltbl_M64_Y0@rel32@hi+1932
	v_dual_mov_b32 v4, 0xbcccb49f :: v_dual_mov_b32 v11, s8
	v_mov_b32_e32 v6, 0xf791c495
	v_dual_mov_b32 v7, 0x402471d7 :: v_dual_mov_b32 v10, s9
	v_mov_b32_e32 v8, 0x35a47d58
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_Y0@rel32@lo+2044
	s_addc_u32 s11, s11, __ocmltbl_M64_Y0@rel32@hi+2052
	v_cndmask_b32_e32 v5, 0x3c80fc78, v4, vcc_lo
	v_cndmask_b32_e32 v4, 0x6ce06080, v6, vcc_lo
	v_cndmask_b32_e32 v9, 0x40277f91, v7, vcc_lo
	v_cndmask_b32_e32 v8, 0x38d43206, v8, vcc_lo
	v_cndmask_b32_e32 v7, s11, v10, vcc_lo
	v_cndmask_b32_e32 v6, s10, v11, vcc_lo
.LBB13_7:
	s_or_b32 exec_lo, exec_lo, s6
.LBB13_8:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
.LBB13_9:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB13_17
; %bb.10:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y0@rel32@lo+1084
	s_addc_u32 s7, s7, __ocmltbl_M64_Y0@rel32@hi+1092
	v_mov_b32_e32 v4, 0xd219bfd
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0xbc8bd1e5
	v_mov_b32_e32 v8, 0xd4dff243
	v_mov_b32_e32 v9, 0x400193be
	s_mov_b32 s5, exec_lo
	v_cmpx_ngt_f64_e32 0x40044000, v[2:3]
	s_cbranch_execz .LBB13_16
; %bb.11:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y0@rel32@lo+1204
	s_addc_u32 s7, s7, __ocmltbl_M64_Y0@rel32@hi+1212
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0x40044000 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s6, exec_lo
	v_cmpx_ngt_f64_e32 0x40080000, v[2:3]
	s_cbranch_execz .LBB13_15
; %bb.12:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y0@rel32@lo+1324
	s_addc_u32 s9, s9, __ocmltbl_M64_Y0@rel32@hi+1332
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0x40080000 :: v_dual_mov_b32 v6, s8
	v_mov_b32_e32 v7, s9
	s_mov_b32 s7, exec_lo
	v_cmpx_ngt_f64_e32 0x400be000, v[2:3]
	s_cbranch_execz .LBB13_14
; %bb.13:
	v_cmp_gt_f64_e32 vcc_lo, 0x4012d000, v[2:3]
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y0@rel32@lo+1444
	s_addc_u32 s9, s9, __ocmltbl_M64_Y0@rel32@hi+1452
	v_dual_mov_b32 v4, 0xbc9f06ae :: v_dual_mov_b32 v11, s8
	v_mov_b32_e32 v6, 0x7804384e
	v_dual_mov_b32 v7, 0x400fa953 :: v_dual_mov_b32 v10, s9
	v_mov_b32_e32 v8, 0x4d98569c
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_Y0@rel32@lo+1564
	s_addc_u32 s11, s11, __ocmltbl_M64_Y0@rel32@hi+1572
	v_cndmask_b32_e32 v5, 0x3cbdfe7b, v4, vcc_lo
	v_cndmask_b32_e32 v4, 0xac228e8c, v6, vcc_lo
	v_cndmask_b32_e32 v9, 0x4015b7fe, v7, vcc_lo
	v_cndmask_b32_e32 v8, 0x4e87b02e, v8, vcc_lo
	v_cndmask_b32_e32 v7, s11, v10, vcc_lo
	v_cndmask_b32_e32 v6, s10, v11, vcc_lo
.LBB13_14:
	s_or_b32 exec_lo, exec_lo, s7
.LBB13_15:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s6
.LBB13_16:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
.LBB13_17:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
.LBB13_18:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB13_34
; %bb.19:
	s_mov_b32 s4, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr4_vgpr5
	v_cmpx_ngt_f64_e32 0x3fea0000, v[2:3]
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB13_25
; %bb.20:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y0@rel32@lo+604
	s_addc_u32 s7, s7, __ocmltbl_M64_Y0@rel32@hi+612
	v_mov_b32_e32 v4, 0x70347f83
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0x3c7ea9d2
	v_mov_b32_e32 v8, 0xb8d417ea
	v_mov_b32_e32 v9, 0x3fec982e
	s_mov_b32 s5, exec_lo
	v_cmpx_ngt_f64_e32 1.0, v[2:3]
	s_cbranch_execz .LBB13_24
; %bb.21:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y0@rel32@lo+724
	s_addc_u32 s7, s7, __ocmltbl_M64_Y0@rel32@hi+732
	v_mov_b32_e32 v8, 0
	v_dual_mov_b32 v9, 0x3ff00000 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s6, exec_lo
	v_cmpx_ngt_f64_e32 0x3ff40000, v[2:3]
	s_cbranch_execz .LBB13_23
; %bb.22:
	v_cmp_gt_f64_e32 vcc_lo, 0x3ffa0000, v[2:3]
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y0@rel32@lo+844
	s_addc_u32 s9, s9, __ocmltbl_M64_Y0@rel32@hi+852
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v4, 0x3ff40000 :: v_dual_mov_b32 v5, s9
	v_mov_b32_e32 v6, s8
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_Y0@rel32@lo+964
	s_addc_u32 s11, s11, __ocmltbl_M64_Y0@rel32@hi+972
	v_dual_mov_b32 v8, 0 :: v_dual_cndmask_b32 v9, 0x3ffa0000, v4
	v_cndmask_b32_e32 v7, s11, v5, vcc_lo
	v_cndmask_b32_e32 v6, s10, v6, vcc_lo
.LBB13_23:
	s_or_b32 exec_lo, exec_lo, s6
	v_mov_b32_e32 v4, 0
	v_mov_b32_e32 v5, 0
.LBB13_24:
	s_or_b32 exec_lo, exec_lo, s5
.LBB13_25:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB13_33
; %bb.26:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y0@rel32@lo+4
	s_addc_u32 s7, s7, __ocmltbl_M64_Y0@rel32@hi+12
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s5, exec_lo
	v_cmpx_ngt_f64_e32 0x3fd40000, v[2:3]
	s_cbranch_execz .LBB13_32
; %bb.27:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y0@rel32@lo+124
	s_addc_u32 s7, s7, __ocmltbl_M64_Y0@rel32@hi+132
	v_mov_b32_e32 v8, 0
	v_dual_mov_b32 v9, 0x3fd40000 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s6, exec_lo
	v_cmpx_ngt_f64_e32 0x3fdc0000, v[2:3]
	s_cbranch_execz .LBB13_31
; %bb.28:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y0@rel32@lo+244
	s_addc_u32 s9, s9, __ocmltbl_M64_Y0@rel32@hi+252
	v_mov_b32_e32 v8, 0
	v_dual_mov_b32 v9, 0x3fdc0000 :: v_dual_mov_b32 v6, s8
	v_mov_b32_e32 v7, s9
	s_mov_b32 s7, exec_lo
	v_cmpx_ngt_f64_e32 0x3fe20000, v[2:3]
	s_cbranch_execz .LBB13_30
; %bb.29:
	v_cmp_gt_f64_e32 vcc_lo, 0x3fe60000, v[2:3]
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y0@rel32@lo+364
	s_addc_u32 s9, s9, __ocmltbl_M64_Y0@rel32@hi+372
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v6, 0x3fe20000 :: v_dual_mov_b32 v7, s9
	v_mov_b32_e32 v10, s8
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_Y0@rel32@lo+484
	s_addc_u32 s11, s11, __ocmltbl_M64_Y0@rel32@hi+492
	v_dual_mov_b32 v8, 0 :: v_dual_cndmask_b32 v9, 0x3fe60000, v6
	v_cndmask_b32_e32 v7, s11, v7, vcc_lo
	v_cndmask_b32_e32 v6, s10, v10, vcc_lo
.LBB13_30:
	s_or_b32 exec_lo, exec_lo, s7
.LBB13_31:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s6
.LBB13_32:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
.LBB13_33:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
.LBB13_34:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s1
	v_mov_b32_e32 v10, 0
	v_mov_b32_e32 v11, 0
	s_mov_b32 s1, exec_lo
	v_cmpx_gt_f64_e32 0x3fd40000, v[2:3]
	s_cbranch_execz .LBB13_56
; %bb.35:
	v_cmp_ge_f64_e64 s4, 0x40292800, |v[2:3]|
                                        ; implicit-def: $vgpr10_vgpr11
	s_and_saveexec_b32 s5, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s4, exec_lo, s5
	s_cbranch_execz .LBB13_49
; %bb.36:
	v_cmp_ge_f64_e64 s5, 0x40191000, |v[2:3]|
                                        ; implicit-def: $vgpr10_vgpr11
                                        ; implicit-def: $vgpr14_vgpr15
                                        ; implicit-def: $vgpr12_vgpr13
	s_and_saveexec_b32 s6, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s5, exec_lo, s6
	s_cbranch_execz .LBB13_42
; %bb.37:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J0@rel32@lo+4
	s_addc_u32 s7, s7, __ocmltbl_M64_J0@rel32@hi+12
	v_mov_b32_e32 v14, 0
	v_dual_mov_b32 v15, 0 :: v_dual_mov_b32 v12, 0
	v_mov_b32_e32 v13, 0
	v_dual_mov_b32 v11, s7 :: v_dual_mov_b32 v10, s6
	s_mov_b32 s6, exec_lo
	v_cmpx_nge_f64_e64 0x3ffa8000, |v[2:3]|
	s_cbranch_execz .LBB13_41
; %bb.38:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J0@rel32@lo+124
	s_addc_u32 s9, s9, __ocmltbl_M64_J0@rel32@hi+132
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v12, 0xd7da258e :: v_dual_mov_b32 v11, s9
	v_dual_mov_b32 v10, s8 :: v_dual_mov_b32 v13, 0xbca0f539
	v_mov_b32_e32 v14, 0x2e971b40
	v_mov_b32_e32 v15, 0x40033d15
	s_mov_b32 s7, exec_lo
	v_cmpx_nge_f64_e64 0x40090000, |v[2:3]|
	s_cbranch_execz .LBB13_40
; %bb.39:
	v_cmp_nge_f64_e64 vcc_lo, 0x4012c000, |v[2:3]|
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J0@rel32@lo+364
	s_addc_u32 s9, s9, __ocmltbl_M64_J0@rel32@hi+372
	v_mov_b32_e32 v12, 0x4016148f
	v_dual_mov_b32 v10, s9 :: v_dual_mov_b32 v13, s8
	v_mov_b32_e32 v14, 0x5b2c2e45
	v_mov_b32_e32 v16, 0x3c975054
	v_mov_b32_e32 v17, 0xcd60a517
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_J0@rel32@lo+244
	s_addc_u32 s11, s11, __ocmltbl_M64_J0@rel32@hi+252
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e32 v11, s11, v10, vcc_lo
	v_cndmask_b32_e32 v10, s10, v13, vcc_lo
	v_cndmask_b32_e32 v15, 0x400ea755, v12, vcc_lo
	v_cndmask_b32_e32 v14, 0x75af6f09, v14, vcc_lo
	v_cndmask_b32_e32 v13, 0xbca60155, v16, vcc_lo
	v_cndmask_b32_e32 v12, 0xa9d1b256, v17, vcc_lo
.LBB13_40:
	s_or_b32 exec_lo, exec_lo, s7
.LBB13_41:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s6
.LBB13_42:
	s_and_not1_saveexec_b32 s5, s5
	s_cbranch_execz .LBB13_48
; %bb.43:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_J0@rel32@lo+484
	s_addc_u32 s7, s7, __ocmltbl_M64_J0@rel32@hi+492
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v12, 0x9d243827 :: v_dual_mov_b32 v11, s7
	v_dual_mov_b32 v10, s6 :: v_dual_mov_b32 v13, 0xbc9b226d
	v_mov_b32_e32 v14, 0xf3b47250
	v_mov_b32_e32 v15, 0x401c0ff5
	s_mov_b32 s6, exec_lo
	v_cmpx_nge_f64_e64 0x401f6000, |v[2:3]|
	s_cbranch_execz .LBB13_47
; %bb.44:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J0@rel32@lo+604
	s_addc_u32 s9, s9, __ocmltbl_M64_J0@rel32@hi+612
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v12, 0x714c7c25 :: v_dual_mov_b32 v11, s9
	v_dual_mov_b32 v10, s8 :: v_dual_mov_b32 v13, 0xbcb51970
	v_mov_b32_e32 v14, 0x6cccdeca
	v_mov_b32_e32 v15, 0x40214eb5
	s_mov_b32 s7, exec_lo
	v_cmpx_nge_f64_e64 0x4022d800, |v[2:3]|
	s_cbranch_execz .LBB13_46
; %bb.45:
	v_cmp_nge_f64_e64 vcc_lo, 0x4025f800, |v[2:3]|
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J0@rel32@lo+844
	s_addc_u32 s9, s9, __ocmltbl_M64_J0@rel32@hi+852
	v_mov_b32_e32 v12, 0x40279544
	v_dual_mov_b32 v10, s9 :: v_dual_mov_b32 v13, s8
	v_mov_b32_e32 v14, 0x8272b6
	v_mov_b32_e32 v16, 0x3cb444fd
	v_mov_b32_e32 v17, 0x5821d5b1
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_J0@rel32@lo+724
	s_addc_u32 s11, s11, __ocmltbl_M64_J0@rel32@hi+732
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e32 v11, s11, v10, vcc_lo
	v_cndmask_b32_e32 v10, s10, v13, vcc_lo
	v_cndmask_b32_e32 v15, 0x402458d0, v12, vcc_lo
	v_cndmask_b32_e32 v14, 0xd0bdfc29, v14, vcc_lo
	v_cndmask_b32_e32 v13, 0x3cc02610, v16, vcc_lo
	v_cndmask_b32_e32 v12, 0xa51562b6, v17, vcc_lo
.LBB13_46:
	s_or_b32 exec_lo, exec_lo, s7
.LBB13_47:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s6
.LBB13_48:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
	s_clause 0x4
	global_load_b64 v[32:33], v[10:11], off offset:112
	global_load_b128 v[16:19], v[10:11], off offset:96
	global_load_b128 v[20:23], v[10:11], off offset:80
	global_load_b128 v[24:27], v[10:11], off offset:64
	global_load_b128 v[28:31], v[10:11], off offset:48
	v_add_f64 v[14:15], |v[2:3]|, -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_f64 v[34:35], v[14:15], -v[12:13]
	global_load_b128 v[12:15], v[10:11], off offset:32
	s_waitcnt vmcnt(4)
	v_fma_f64 v[18:19], v[34:35], v[32:33], v[18:19]
	v_fma_f64 v[32:33], v[34:35], v[18:19], v[16:17]
	global_load_b128 v[16:19], v[10:11], off offset:16
	s_waitcnt vmcnt(4)
	v_fma_f64 v[22:23], v[34:35], v[32:33], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_fma_f64 v[32:33], v[34:35], v[22:23], v[20:21]
	global_load_b128 v[20:23], v[10:11], off
	s_waitcnt vmcnt(4)
	v_fma_f64 v[10:11], v[34:35], v[32:33], v[26:27]
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[24:25]
	s_waitcnt vmcnt(3)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[30:31]
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[28:29]
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[14:15]
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[12:13]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[18:19]
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[16:17]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[22:23]
	v_fma_f64 v[10:11], v[34:35], v[10:11], v[20:21]
.LBB13_49:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB13_55
; %bb.50:
	v_cmp_ngt_f64_e64 s5, 0x41d00000, |v[2:3]|
	v_and_b32_e32 v11, 0x7fffffff, v3
                                        ; implicit-def: $vgpr16
                                        ; implicit-def: $vgpr12_vgpr13
                                        ; implicit-def: $vgpr14_vgpr15
	s_and_saveexec_b32 s6, s5
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s5, exec_lo, s6
	s_cbranch_execz .LBB13_52
; %bb.51:
	v_ldexp_f64 v[12:13], |v[2:3]|, 0xffffff80
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_trig_preop_f64 v[14:15], |v[2:3]|, 0
	v_trig_preop_f64 v[16:17], |v[2:3]|, 1
	v_trig_preop_f64 v[26:27], |v[2:3]|, 2
	v_mov_b32_e32 v34, 0
	s_mov_b32 s6, 0x54442d18
	s_mov_b32 s7, 0x3ff921fb
	s_mov_b32 s8, 0x33145c07
	s_mov_b32 s9, 0x3c91a626
	v_dual_cndmask_b32 v13, v11, v13 :: v_dual_cndmask_b32 v12, v2, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[14:15], v[12:13]
	v_mul_f64 v[20:21], v[16:17], v[12:13]
	v_mul_f64 v[32:33], v[26:27], v[12:13]
	v_fma_f64 v[14:15], v[14:15], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[16:17], v[12:13], -v[20:21]
	v_fma_f64 v[12:13], v[26:27], v[12:13], -v[32:33]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[20:21], v[14:15]
	v_add_f64 v[24:25], v[22:23], -v[20:21]
	v_add_f64 v[30:31], v[18:19], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[28:29], v[22:23], -v[24:25]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	v_ldexp_f64 v[24:25], v[30:31], -2
	v_add_f64 v[18:19], v[30:31], -v[18:19]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], -v[28:29]
	v_add_f64 v[28:29], v[32:33], v[16:17]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[24:25]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[18:19], v[22:23], -v[18:19]
	v_add_f64 v[14:15], v[14:15], v[20:21]
	v_fract_f64_e32 v[20:21], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[28:29], v[14:15]
	v_ldexp_f64 v[20:21], v[20:21], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[18:19], v[22:23]
	v_dual_cndmask_b32 v21, 0, v21 :: v_dual_cndmask_b32 v20, 0, v20
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[30:31], v[24:25], v[20:21]
	v_add_f64 v[18:19], v[24:25], -v[18:19]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[30:31]
	v_add_f64 v[30:31], v[28:29], -v[32:33]
	v_cndmask_b32_e64 v35, 0, 0x40100000, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[39:40], v[28:29], -v[30:31]
	v_add_f64 v[16:17], v[16:17], -v[30:31]
	v_add_f64 v[20:21], v[20:21], v[34:35]
	v_add_f64 v[35:36], v[22:23], -v[28:29]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[30:31], v[32:33], -v[39:40]
	v_add_f64 v[37:38], v[24:25], v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[41:42], v[22:23], -v[35:36]
	v_add_f64 v[14:15], v[14:15], -v[35:36]
	v_add_f64 v[16:17], v[16:17], v[30:31]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_i32_f64_e32 v10, v[37:38]
	v_add_f64 v[28:29], v[28:29], -v[41:42]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_f64_i32_e32 v[35:36], v10
	v_add_f64 v[14:15], v[14:15], v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[35:36]
	v_add_f64 v[14:15], v[16:17], v[14:15]
	v_add_f64 v[16:17], v[22:23], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[24:25], v[20:21]
	v_add_f64 v[12:13], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[26:27], -v[20:21]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[26:27]
	v_add_f64 v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[24:25], -v[14:15]
	v_cndmask_b32_e64 v35, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v16, null, 0, v10, vcc_lo
	v_add_f64 v[12:13], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[26:27], -v[34:35]
	v_add_f64 v[17:18], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[19:20], v[17:18], s[6:7]
	v_add_f64 v[14:15], v[17:18], -v[14:15]
	v_fma_f64 v[21:22], v[17:18], s[6:7], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[17:18], s[8:9], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], s[6:7], v[14:15]
	v_add_f64 v[12:13], v[19:20], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[12:13], -v[19:20]
	v_add_f64 v[14:15], v[14:15], -v[17:18]
.LBB13_52:
	s_and_not1_saveexec_b32 s5, s5
	s_cbranch_execz .LBB13_54
; %bb.53:
	s_mov_b32 s6, 0x6dc9c883
	s_mov_b32 s7, 0x3fe45f30
	s_mov_b32 s9, 0xbc91a626
	v_mul_f64 v[12:13], |v[2:3]|, s[6:7]
	s_mov_b32 s6, 0x54442d18
	s_mov_b32 s7, 0xbff921fb
	s_mov_b32 s8, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[16:17], v[12:13]
	v_fma_f64 v[12:13], v[16:17], s[6:7], |v[2:3]|
	v_mul_f64 v[14:15], v[16:17], s[8:9]
	s_mov_b32 s6, 0x252049c0
	s_mov_b32 s7, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[16:17], s[8:9], v[12:13]
	v_add_f64 v[18:19], v[12:13], v[14:15]
	s_mov_b32 s9, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[8:9], v[14:15]
	v_add_f64 v[12:13], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[6:7], v[12:13]
	v_cvt_i32_f64_e32 v16, v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[20:21], v[14:15]
	v_add_f64 v[18:19], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[18:19]
.LBB13_54:
	s_or_b32 exec_lo, exec_lo, s5
	v_mov_b32_e32 v10, v2
	s_mov_b32 s6, 0x923b70a7
	s_mov_b32 s8, 0xa4a989b
	s_mov_b32 s7, 0x41752a41
	s_mov_b32 s9, 0xc1b91f78
	v_div_scale_f64 v[17:18], null, v[10:11], v[10:11], 1.0
	v_div_scale_f64 v[10:11], vcc_lo, 1.0, v[10:11], 1.0
	s_mov_b32 s10, 0x796cde01
	s_mov_b32 s11, 0x3ec71de3
	v_rcp_f64_e32 v[19:20], v[17:18]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[21:22], -v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[19:20], v[21:22], v[19:20]
	v_fma_f64 v[21:22], -v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[19:20], v[21:22], v[19:20]
	v_mul_f64 v[21:22], v[10:11], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[17:18], v[21:22], v[10:11]
	v_div_fmas_f64 v[10:11], v[10:11], v[19:20], v[21:22]
	v_mov_b32_e32 v21, 0x54442d18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[10:11], v[10:11], |v[2:3]|, 1.0
	v_mul_f64 v[17:18], v[10:11], v[10:11]
	v_rsq_f64_e32 v[27:28], v[10:11]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], s[8:9], s[6:7]
	s_mov_b32 s6, 0x31612a8d
	s_mov_b32 s7, 0xc1240a5e
	s_mov_b32 s8, 0xcd7ac32c
	s_mov_b32 s9, 0x41344395
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[33:34], v[10:11], v[27:28]
	v_mul_f64 v[27:28], v[27:28], 0.5
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[6:7]
	s_mov_b32 s6, 0xcbe3b3b8
	s_mov_b32 s7, 0x40d0c9a0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[35:36], -v[27:28], v[33:34], 0.5
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[6:7]
	s_mov_b32 s6, 0x167fe583
	s_mov_b32 s7, 0xc080af76
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[33:34], v[33:34], v[35:36], v[33:34]
	v_fma_f64 v[27:28], v[27:28], v[35:36], v[27:28]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[6:7]
	s_mov_b32 s6, 0x61b94139
	s_mov_b32 s7, 0x403778ea
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[35:36], -v[33:34], v[33:34], v[10:11]
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[6:7]
	s_mov_b32 s6, 0xd1a82662
	s_mov_b32 s7, 0xbffa3581
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[6:7]
	s_mov_b32 s6, 0x30a1daf2
	s_mov_b32 s7, 0x3fcad333
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[6:7]
	s_mov_b32 s6, 0xaaaa7909
	s_mov_b32 s7, 0xbfb0aaaa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[6:7]
	s_mov_b32 s6, 0xabbee803
	s_mov_b32 s7, 0xc0f25bf3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], 0x3fc00000
	v_mul_f64 v[19:20], v[10:11], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_cmp_lt_f64_e32 vcc_lo, v[12:13], v[19:20]
	v_subrev_co_ci_u32_e64 v43, null, 0, v16, vcc_lo
	v_cndmask_b32_e64 v16, 0, 0x80000000, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, 0, v[10:11]
	v_xor_b32_e32 v22, 0xbfe921fb, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[23:24], v[21:22], -v[19:20]
	v_add_f64 v[21:22], v[21:22], -v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[19:20], v[21:22], -v[19:20]
	v_mov_b32_e32 v21, 0x33145c07
	v_xor_b32_e32 v22, 0xbc81a626, v16
	v_add_f64 v[19:20], v[19:20], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[21:22], v[23:24], v[19:20]
	v_add_f64 v[25:26], v[12:13], v[21:22]
	v_add_f64 v[23:24], v[21:22], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[21:22], v[25:26], -v[21:22]
	v_add_f64 v[19:20], v[19:20], -v[23:24]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[21:22]
	v_add_f64 v[14:15], v[14:15], v[19:20]
	v_fma_f64 v[21:22], v[17:18], s[8:9], s[6:7]
	s_mov_b32 s6, 0xb42fdfa7
	s_mov_b32 s8, 0xf9a43bb8
	s_mov_b32 s7, 0xbe5ae600
	s_mov_b32 s9, 0x3de5e0b2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], v[14:15]
	v_add_f64 v[14:15], v[25:26], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[19:20], v[14:15], v[14:15]
	v_add_f64 v[25:26], v[14:15], -v[25:26]
	v_fma_f64 v[23:24], v[19:20], s[8:9], s[6:7]
	s_mov_b32 s6, 0x78625b0f
	s_mov_b32 s7, 0x40a55a4a
	s_mov_b32 s8, 0x46cc5e42
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[6:7]
	s_mov_b32 s6, 0x9037ab78
	s_mov_b32 s7, 0x3e21eeb6
	s_mov_b32 s9, 0xbda907db
	v_mul_f64 v[31:32], v[19:20], 0.5
	v_fma_f64 v[29:30], v[19:20], s[8:9], s[6:7]
	s_mov_b32 s6, 0x7ea56321
	s_mov_b32 s7, 0xc05a826c
	s_mov_b32 s8, 0x19e83e5c
	s_mov_b32 s9, 0xbf2a01a0
	v_add_f64 v[12:13], v[12:13], -v[25:26]
	v_mul_f64 v[37:38], v[14:15], -v[19:20]
	v_fma_f64 v[23:24], v[19:20], v[23:24], s[10:11]
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[6:7]
	s_mov_b32 s6, 0xa17f65f6
	s_mov_b32 s7, 0xbe927e4f
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[19:20], v[29:30], s[6:7]
	v_add_f64 v[29:30], -v[31:32], 1.0
	s_mov_b32 s6, 0x3bbf53b6
	s_mov_b32 s7, 0x40176325
	v_mul_f64 v[39:40], v[12:13], 0.5
	v_fma_f64 v[23:24], v[19:20], v[23:24], s[8:9]
	s_mov_b32 s8, 0x11110bb3
	s_mov_b32 s9, 0x3f811111
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[6:7]
	s_mov_b32 s6, 0x19f4ec90
	s_mov_b32 s7, 0x3efa01a0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_fma_f64 v[25:26], v[19:20], v[25:26], s[6:7]
	v_add_f64 v[41:42], -v[29:30], 1.0
	s_mov_b32 s6, 0xff948953
	s_mov_b32 s7, 0xbfe15efa
	v_fma_f64 v[23:24], v[19:20], v[23:24], s[8:9]
	s_mov_b32 s8, 0xffff2868
	s_mov_b32 s9, 0xbfafffff
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[6:7]
	s_mov_b32 s6, 0x16c16967
	s_mov_b32 s7, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[25:26], v[19:20], v[25:26], s[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_add_f64 v[31:32], v[41:42], -v[31:32]
	s_mov_b32 s6, 0xf967a1d4
	s_mov_b32 s7, 0x3fba7fff
	v_fma_f64 v[23:24], v[37:38], v[23:24], v[39:40]
	v_mul_f64 v[39:40], v[19:20], v[19:20]
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[6:7]
	s_mov_b32 s7, 0x3fa55555
	s_mov_b32 s6, 0x55555555
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[23:24], v[19:20], v[23:24], -v[12:13]
	v_fma_f64 v[19:20], v[19:20], v[25:26], s[6:7]
	v_fma_f64 v[12:13], v[14:15], -v[12:13], v[31:32]
	v_fma_f64 v[25:26], v[35:36], v[27:28], v[33:34]
	v_fma_f64 v[21:22], v[17:18], v[21:22], s[8:9]
	s_mov_b32 s7, 0xbfc55555
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[23:24], v[37:38], s[6:7], v[23:24]
	s_mov_b32 s6, 0x33d43651
	v_fma_f64 v[12:13], v[39:40], v[19:20], v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_dual_cndmask_b32 v11, v26, v11 :: v_dual_cndmask_b32 v10, v25, v10
	s_mov_b32 s7, 0x3fe98845
	v_fma_f64 v[16:17], v[17:18], v[21:22], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[10:11], s[6:7]
	v_add_f64 v[14:15], v[14:15], -v[23:24]
	v_add_f64 v[12:13], v[29:30], v[12:13]
	v_mul_f64 v[10:11], v[10:11], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v15, 0x80000000, v15
	v_and_b32_e32 v16, 1, v43
	v_cmp_eq_u32_e32 vcc_lo, 0, v16
	v_dual_cndmask_b32 v12, v14, v12 :: v_dual_lshlrev_b32 v17, 30, v43
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_and_b32_e32 v16, 0x80000000, v17
	v_cndmask_b32_e32 v13, v15, v13, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[2:3]|
	v_xor_b32_e32 v13, v13, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[10:11], v[12:13]
	v_dual_cndmask_b32 v11, 0, v11 :: v_dual_cndmask_b32 v10, 0, v10
.LBB13_55:
	s_or_b32 exec_lo, exec_lo, s4
	v_frexp_mant_f64_e32 v[12:13], v[2:3]
	s_mov_b32 s5, 0x3fe55555
	s_mov_b32 s4, 0x55555555
	s_mov_b32 s6, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s7, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[12:13]
	s_mov_b32 s4, 0x55555780
	v_cndmask_b32_e64 v14, 0, 1, vcc_lo
	v_ldexp_f64 v[12:13], v[12:13], v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], 1.0
	v_add_f64 v[20:21], v[12:13], -1.0
	v_rcp_f64_e32 v[16:17], v[14:15]
	v_add_f64 v[22:23], v[14:15], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[16:17], v[18:19], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[16:17], v[18:19], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[20:21], v[16:17]
	v_mul_f64 v[24:25], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[18:19], v[14:15], -v[24:25]
	v_fma_f64 v[12:13], v[18:19], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[24:25], v[12:13]
	v_add_f64 v[22:23], v[20:21], -v[14:15]
	v_add_f64 v[24:25], v[14:15], -v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[22:23]
	v_add_f64 v[12:13], v[24:25], -v[12:13]
	v_frexp_exp_i32_f64_e32 v24, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[20:21], -v[14:15]
	v_add_f64 v[12:13], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[22:23], v[12:13]
	v_mul_f64 v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[18:19], v[12:13]
	v_mul_f64 v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[16:17], s[8:9], s[6:7]
	s_mov_b32 s6, 0xd7f4df2e
	s_mov_b32 s7, 0x3fc7474d
	v_mul_f64 v[22:23], v[14:15], v[16:17]
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[6:7]
	s_mov_b32 s6, 0x16291751
	s_mov_b32 s7, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[6:7]
	s_mov_b32 s6, 0x9b27acf1
	s_mov_b32 s7, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[6:7]
	s_mov_b32 s6, 0x998ef7b6
	s_mov_b32 s7, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[16:17], v[20:21], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[16:17], v[16:17], v[20:21], s[4:5]
	v_ldexp_f64 v[20:21], v[14:15], 1
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0x3fe62e42
	v_mul_f64 v[16:17], v[22:23], v[16:17]
	v_subrev_co_ci_u32_e64 v22, null, 0, v24, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_cvt_f64_i32_e32 v[22:23], v22
	v_add_f64 v[18:19], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[12:13], v[12:13], 1
	v_mul_f64 v[24:25], v[22:23], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[18:19], -v[20:21]
	v_fma_f64 v[20:21], v[22:23], s[4:5], -v[24:25]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	v_fma_f64 v[16:17], v[22:23], s[4:5], v[20:21]
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[10:11], s[4:5]
	v_add_f64 v[12:13], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[24:25], v[16:17]
	v_add_f64 v[20:21], v[18:19], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[14:15], -v[24:25]
	v_add_f64 v[22:23], v[14:15], v[20:21]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	v_add_f64 v[26:27], v[22:23], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_add_f64 v[28:29], v[22:23], -v[26:27]
	v_add_f64 v[18:19], v[20:21], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[16:17], v[12:13]
	v_add_f64 v[14:15], v[14:15], -v[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[18:19], v[14:15]
	v_add_f64 v[18:19], v[20:21], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[20:21], v[14:15]
	v_add_f64 v[20:21], v[20:21], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[22:23], v[14:15]
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[24:25], -v[22:23]
	v_add_f64 v[12:13], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	v_add_f64 v[12:13], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[24:25], v[12:13]
	v_cndmask_b32_e32 v14, v12, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v12, v13, v3, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v15, 0x7ff80000, v12, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	v_mul_f64 v[12:13], v[2:3], v[2:3]
	v_cndmask_b32_e32 v14, 0, v14, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_mov_b32 v2, v12 :: v_dual_mov_b32 v3, v13
	v_cndmask_b32_e32 v15, 0xfff00000, v15, vcc_lo
	v_mul_f64 v[10:11], v[14:15], v[10:11]
.LBB13_56:
	s_or_b32 exec_lo, exec_lo, s1
	s_clause 0x4
	global_load_b64 v[28:29], v[6:7], off offset:112
	global_load_b128 v[12:15], v[6:7], off offset:96
	global_load_b128 v[16:19], v[6:7], off offset:80
	global_load_b128 v[20:23], v[6:7], off offset:64
	global_load_b128 v[24:27], v[6:7], off offset:48
	v_add_f64 v[2:3], v[2:3], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_f64 v[30:31], v[2:3], -v[4:5]
	global_load_b128 v[2:5], v[6:7], off offset:32
	s_waitcnt vmcnt(4)
	v_fma_f64 v[8:9], v[30:31], v[28:29], v[14:15]
	v_fma_f64 v[8:9], v[30:31], v[8:9], v[12:13]
	global_load_b128 v[12:15], v[6:7], off offset:16
	s_waitcnt vmcnt(4)
	v_fma_f64 v[8:9], v[30:31], v[8:9], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[30:31], v[8:9], v[16:17]
	global_load_b128 v[6:9], v[6:7], off
	s_waitcnt vmcnt(4)
	v_fma_f64 v[16:17], v[30:31], v[16:17], v[22:23]
	v_fma_f64 v[16:17], v[30:31], v[16:17], v[20:21]
	s_waitcnt vmcnt(3)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[30:31], v[16:17], v[26:27]
	v_fma_f64 v[16:17], v[30:31], v[16:17], v[24:25]
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[30:31], v[16:17], v[4:5]
	v_fma_f64 v[2:3], v[30:31], v[4:5], v[2:3]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[30:31], v[2:3], v[14:15]
	v_fma_f64 v[2:3], v[30:31], v[2:3], v[12:13]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[30:31], v[2:3], v[8:9]
	v_fma_f64 v[2:3], v[30:31], v[2:3], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[4:5], v[10:11], v[2:3]
                                        ; implicit-def: $vgpr2_vgpr3
.LBB13_57:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB13_63
; %bb.58:
	s_mov_b32 s1, exec_lo
                                        ; implicit-def: $vgpr8
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr6_vgpr7
	v_cmpx_ngt_f64_e32 0x41d00000, v[2:3]
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB13_60
; %bb.59:
	v_ldexp_f64 v[4:5], v[2:3], 0xffffff80
	v_cmp_le_f64_e32 vcc_lo, 0x7b000000, v[2:3]
	v_trig_preop_f64 v[6:7], v[2:3], 0
	v_trig_preop_f64 v[8:9], v[2:3], 1
	v_trig_preop_f64 v[18:19], v[2:3], 2
	v_mov_b32_e32 v26, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_dual_cndmask_b32 v5, v3, v5 :: v_dual_cndmask_b32 v4, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[6:7], v[4:5]
	v_mul_f64 v[12:13], v[8:9], v[4:5]
	v_mul_f64 v[24:25], v[18:19], v[4:5]
	v_fma_f64 v[6:7], v[6:7], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[4:5], -v[12:13]
	v_fma_f64 v[4:5], v[18:19], v[4:5], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[12:13], v[6:7]
	v_add_f64 v[16:17], v[14:15], -v[12:13]
	v_add_f64 v[22:23], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	v_ldexp_f64 v[16:17], v[22:23], -2
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[20:21], v[24:25], v[8:9]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[16:17]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_fract_f64_e32 v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[20:21], v[6:7]
	v_ldexp_f64 v[12:13], v[12:13], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], v[14:15]
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[16:17], v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[22:23]
	v_add_f64 v[22:23], v[20:21], -v[24:25]
	v_cndmask_b32_e64 v27, 0, 0x40100000, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[31:32], v[20:21], -v[22:23]
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[12:13], v[12:13], v[26:27]
	v_add_f64 v[27:28], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[24:25], -v[31:32]
	v_add_f64 v[29:30], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[33:34], v[14:15], -v[27:28]
	v_add_f64 v[6:7], v[6:7], -v[27:28]
	v_add_f64 v[8:9], v[8:9], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_i32_f64_e32 v29, v[29:30]
	v_add_f64 v[20:21], v[20:21], -v[33:34]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_f64_i32_e32 v[27:28], v29
	v_add_f64 v[6:7], v[6:7], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[27:28]
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[16:17], v[12:13]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[18:19], -v[12:13]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[18:19]
	v_add_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_cndmask_b32_e64 v27, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v8, null, 0, v29, vcc_lo
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], -v[26:27]
	v_add_f64 v[9:10], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[11:12], v[9:10], s[4:5]
	v_add_f64 v[6:7], v[9:10], -v[6:7]
	v_fma_f64 v[13:14], v[9:10], s[4:5], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_fma_f64 v[6:7], v[9:10], s[6:7], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	v_add_f64 v[4:5], v[11:12], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[4:5], -v[11:12]
	v_add_f64 v[6:7], v[6:7], -v[9:10]
.LBB13_60:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB13_62
; %bb.61:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[4:5], v[2:3], s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[4:5]
	v_fma_f64 v[4:5], v[8:9], s[4:5], v[2:3]
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
.LBB13_62:
	s_or_b32 exec_lo, exec_lo, s1
	v_div_scale_f64 v[9:10], null, v[2:3], v[2:3], 1.0
	v_div_scale_f64 v[15:16], vcc_lo, 1.0, v[2:3], 1.0
	s_mov_b32 s4, 0x923b70a7
	s_mov_b32 s6, 0xa4a989b
	s_mov_b32 s5, 0x41752a41
	s_mov_b32 s7, 0xc1b91f78
	s_mov_b32 s8, 0x796cde01
	s_mov_b32 s9, 0x3ec71de3
	v_rcp_f64_e32 v[11:12], v[9:10]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[11:12], v[13:14], v[11:12]
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[11:12], v[13:14], v[11:12]
	v_mul_f64 v[13:14], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[9:10], v[13:14], v[15:16]
	v_div_fmas_f64 v[9:10], v[9:10], v[11:12], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[9:10], v[9:10], v[2:3], 1.0
	v_mul_f64 v[11:12], v[9:10], v[9:10]
	v_rsq_f64_e32 v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], s[6:7], s[4:5]
	s_mov_b32 s4, 0x31612a8d
	s_mov_b32 s5, 0xc1240a5e
	s_mov_b32 s6, 0xcd7ac32c
	s_mov_b32 s7, 0x41344395
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[27:28], v[9:10], v[21:22]
	v_mul_f64 v[21:22], v[21:22], 0.5
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xcbe3b3b8
	s_mov_b32 s5, 0x40d0c9a0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], -v[21:22], v[27:28], 0.5
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x167fe583
	s_mov_b32 s5, 0xc080af76
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[27:28], v[27:28], v[29:30], v[27:28]
	v_fma_f64 v[21:22], v[21:22], v[29:30], v[21:22]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x61b94139
	s_mov_b32 s5, 0x403778ea
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], -v[27:28], v[27:28], v[9:10]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xd1a82662
	s_mov_b32 s5, 0xbffa3581
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x30a1daf2
	s_mov_b32 s5, 0x3fcad333
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xaaaa7909
	s_mov_b32 s5, 0xbfb0aaaa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xabbee803
	s_mov_b32 s5, 0xc0f25bf3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], 0x3fc00000
	v_mul_f64 v[13:14], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_cmp_lt_f64_e32 vcc_lo, v[4:5], v[13:14]
	v_subrev_co_ci_u32_e64 v37, null, 0, v8, vcc_lo
	v_cndmask_b32_e64 v8, 0, 0x80000000, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, 0, v[9:10]
	v_mov_b32_e32 v15, 0x54442d18
	v_xor_b32_e32 v16, 0xbfe921fb, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[15:16], -v[13:14]
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_mov_b32_e32 v15, 0x33145c07
	v_xor_b32_e32 v16, 0xbc81a626, v8
	v_add_f64 v[13:14], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[17:18], v[13:14]
	v_add_f64 v[19:20], v[4:5], v[15:16]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[19:20], -v[15:16]
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[15:16]
	v_add_f64 v[6:7], v[6:7], v[13:14]
	v_fma_f64 v[15:16], v[11:12], s[6:7], s[4:5]
	s_mov_b32 s4, 0xb42fdfa7
	s_mov_b32 s6, 0xf9a43bb8
	s_mov_b32 s5, 0xbe5ae600
	s_mov_b32 s7, 0x3de5e0b2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[19:20], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[6:7], v[6:7]
	v_add_f64 v[19:20], v[6:7], -v[19:20]
	v_fma_f64 v[17:18], v[13:14], s[6:7], s[4:5]
	s_mov_b32 s4, 0x78625b0f
	s_mov_b32 s5, 0x40a55a4a
	s_mov_b32 s6, 0x46cc5e42
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x9037ab78
	s_mov_b32 s5, 0x3e21eeb6
	s_mov_b32 s7, 0xbda907db
	v_mul_f64 v[25:26], v[13:14], 0.5
	v_fma_f64 v[23:24], v[13:14], s[6:7], s[4:5]
	s_mov_b32 s4, 0x7ea56321
	s_mov_b32 s5, 0xc05a826c
	s_mov_b32 s6, 0x19e83e5c
	s_mov_b32 s7, 0xbf2a01a0
	v_add_f64 v[4:5], v[4:5], -v[19:20]
	v_mul_f64 v[31:32], v[6:7], -v[13:14]
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[8:9]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0xa17f65f6
	s_mov_b32 s5, 0xbe927e4f
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[23:24], s[4:5]
	v_add_f64 v[23:24], -v[25:26], 1.0
	s_mov_b32 s4, 0x3bbf53b6
	s_mov_b32 s5, 0x40176325
	v_mul_f64 v[33:34], v[4:5], 0.5
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0x11110bb3
	s_mov_b32 s7, 0x3f811111
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x19f4ec90
	s_mov_b32 s5, 0x3efa01a0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	v_add_f64 v[35:36], -v[23:24], 1.0
	s_mov_b32 s4, 0xff948953
	s_mov_b32 s5, 0xbfe15efa
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0xffff2868
	s_mov_b32 s7, 0xbfafffff
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x16c16967
	s_mov_b32 s5, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_add_f64 v[25:26], v[35:36], -v[25:26]
	s_mov_b32 s4, 0xf967a1d4
	s_mov_b32 s5, 0x3fba7fff
	v_fma_f64 v[17:18], v[31:32], v[17:18], v[33:34]
	v_mul_f64 v[33:34], v[13:14], v[13:14]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s5, 0x3fa55555
	s_mov_b32 s4, 0x55555555
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	v_fma_f64 v[25:26], v[6:7], -v[4:5], v[25:26]
	s_mov_b32 s5, 0xbfc55555
	v_fma_f64 v[4:5], v[13:14], v[17:18], -v[4:5]
	v_fma_f64 v[13:14], v[29:30], v[21:22], v[27:28]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[17:18], v[33:34], v[19:20], v[25:26]
	v_fma_f64 v[4:5], v[31:32], s[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v10, v14, v10, vcc_lo
	s_mov_b32 s4, 0x33d43651
	s_mov_b32 s5, 0x3fe98845
	v_add_f64 v[4:5], v[6:7], -v[4:5]
	v_cndmask_b32_e32 v9, v13, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[9:10], s[4:5]
	v_fma_f64 v[10:11], v[11:12], v[15:16], 1.0
	v_add_f64 v[12:13], v[23:24], v[17:18]
	v_mul_f64 v[6:7], v[8:9], v[10:11]
	v_and_b32_e32 v8, 1, v37
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_eq_u32_e32 vcc_lo, 0, v8
	v_dual_cndmask_b32 v4, v12, v4 :: v_dual_cndmask_b32 v5, v13, v5
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[2:3]
	v_lshlrev_b32_e32 v9, 30, v37
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b32_e32 v8, 0x80000000, v9
	v_xor_b32_e32 v5, v5, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[6:7], v[4:5]
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
.LBB13_63:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB13_64:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_y0_kernel
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
		.amdhsa_next_free_vgpr 44
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
		.amdhsa_inst_pref_size 63
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
	.size	specialx_y0_kernel, .Lfunc_end13-specialx_y0_kernel
                                        ; -- End function
	.set specialx_y0_kernel.num_vgpr, 44
	.set specialx_y0_kernel.num_agpr, 0
	.set specialx_y0_kernel.numbered_sgpr, 12
	.set specialx_y0_kernel.num_named_barrier, 0
	.set specialx_y0_kernel.private_seg_size, 0
	.set specialx_y0_kernel.uses_vcc, 1
	.set specialx_y0_kernel.uses_flat_scratch, 0
	.set specialx_y0_kernel.has_dyn_sized_stack, 0
	.set specialx_y0_kernel.has_recursion, 0
	.set specialx_y0_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 8284
; TotalNumSgprs: 14
; NumVgprs: 44
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 44
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
	.protected	specialx_y1_kernel      ; -- Begin function specialx_y1_kernel
	.globl	specialx_y1_kernel
	.p2align	8
	.type	specialx_y1_kernel,@function
specialx_y1_kernel:                     ; @specialx_y1_kernel
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
	s_cbranch_execz .LBB14_68
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_ge_f64_e32 0x40292000, v[2:3]
	s_xor_b32 s0, exec_lo, s0
	s_cbranch_execz .LBB14_61
; %bb.2:
	s_mov_b32 s1, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr4_vgpr5
	v_cmpx_ngt_f64_e32 0x40028000, v[2:3]
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB14_18
; %bb.3:
	s_mov_b32 s4, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr4_vgpr5
	v_cmpx_ngt_f64_e32 0x4018c000, v[2:3]
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB14_9
; %bb.4:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y1@rel32@lo+1684
	s_addc_u32 s7, s7, __ocmltbl_M64_Y1@rel32@hi+1692
	v_mov_b32_e32 v4, 0x6b1c46ac
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0x3ca7960b
	v_mov_b32_e32 v8, 0x90588553
	v_mov_b32_e32 v9, 0x401bc418
	s_mov_b32 s5, exec_lo
	v_cmpx_ngt_f64_e32 0x401f1400, v[2:3]
	s_cbranch_execz .LBB14_8
; %bb.5:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y1@rel32@lo+1804
	s_addc_u32 s7, s7, __ocmltbl_M64_Y1@rel32@hi+1812
	v_mov_b32_e32 v4, 0x68d9046
	v_dual_mov_b32 v6, s6 :: v_dual_mov_b32 v7, s7
	v_mov_b32_e32 v5, 0x3cb479cc
	v_mov_b32_e32 v8, 0xae6169b4
	v_mov_b32_e32 v9, 0x40213127
	s_mov_b32 s6, exec_lo
	v_cmpx_ngt_f64_e32 0x4022b800, v[2:3]
	s_cbranch_execz .LBB14_7
; %bb.6:
	v_cmp_gt_f64_e32 vcc_lo, 0x4025e000, v[2:3]
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y1@rel32@lo+1924
	s_addc_u32 s9, s9, __ocmltbl_M64_Y1@rel32@hi+1932
	v_dual_mov_b32 v4, 0x3cc8f4ba :: v_dual_mov_b32 v11, s8
	v_mov_b32_e32 v6, 0x5d68e440
	v_dual_mov_b32 v7, 0x40243f2e :: v_dual_mov_b32 v10, s9
	v_mov_b32_e32 v8, 0xe51e8c7e
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_Y1@rel32@lo+2044
	s_addc_u32 s11, s11, __ocmltbl_M64_Y1@rel32@hi+2052
	v_cndmask_b32_e32 v5, 0x3c80fc78, v4, vcc_lo
	v_cndmask_b32_e32 v4, 0x6ce06080, v6, vcc_lo
	v_cndmask_b32_e32 v9, 0x40277f91, v7, vcc_lo
	v_cndmask_b32_e32 v8, 0x38d43206, v8, vcc_lo
	v_cndmask_b32_e32 v7, s11, v10, vcc_lo
	v_cndmask_b32_e32 v6, s10, v11, vcc_lo
.LBB14_7:
	s_or_b32 exec_lo, exec_lo, s6
.LBB14_8:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
.LBB14_9:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB14_17
; %bb.10:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y1@rel32@lo+1084
	s_addc_u32 s7, s7, __ocmltbl_M64_Y1@rel32@hi+1092
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0x40028000 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s5, exec_lo
	v_cmpx_ngt_f64_e32 0x4005e000, v[2:3]
	s_cbranch_execz .LBB14_16
; %bb.11:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y1@rel32@lo+1204
	s_addc_u32 s7, s7, __ocmltbl_M64_Y1@rel32@hi+1212
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0x4005e000 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s6, exec_lo
	v_cmpx_ngt_f64_e32 0x40094000, v[2:3]
	s_cbranch_execz .LBB14_15
; %bb.12:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y1@rel32@lo+1324
	s_addc_u32 s9, s9, __ocmltbl_M64_Y1@rel32@hi+1332
	v_mov_b32_e32 v4, 0x714e4129
	v_dual_mov_b32 v6, s8 :: v_dual_mov_b32 v7, s9
	v_mov_b32_e32 v5, 0x3c53bac0
	v_mov_b32_e32 v8, 0xaffba175
	v_mov_b32_e32 v9, 0x400d76d4
	s_mov_b32 s7, exec_lo
	v_cmpx_ngt_f64_e32 0x4010d000, v[2:3]
	s_cbranch_execz .LBB14_14
; %bb.13:
	v_cmp_gt_f64_e32 vcc_lo, 0x4012c000, v[2:3]
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y1@rel32@lo+1444
	s_addc_u32 s9, s9, __ocmltbl_M64_Y1@rel32@hi+1452
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v6, 0x4010d000 :: v_dual_mov_b32 v7, s9
	v_mov_b32_e32 v10, s8
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_Y1@rel32@lo+1564
	s_addc_u32 s11, s11, __ocmltbl_M64_Y1@rel32@hi+1572
	v_cndmask_b32_e64 v5, 0x3cbdfe7b, 0, vcc_lo
	v_cndmask_b32_e64 v4, 0xac228e8c, 0, vcc_lo
	v_cndmask_b32_e32 v9, 0x4015b7fe, v6, vcc_lo
	v_cndmask_b32_e64 v8, 0x4e87b02e, 0, vcc_lo
	v_cndmask_b32_e32 v7, s11, v7, vcc_lo
	v_cndmask_b32_e32 v6, s10, v10, vcc_lo
.LBB14_14:
	s_or_b32 exec_lo, exec_lo, s7
.LBB14_15:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s6
.LBB14_16:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
.LBB14_17:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
.LBB14_18:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB14_34
; %bb.19:
	s_mov_b32 s4, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
                                        ; implicit-def: $vgpr4_vgpr5
	v_cmpx_ngt_f64_e32 0x3ff38000, v[2:3]
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB14_25
; %bb.20:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y1@rel32@lo+604
	s_addc_u32 s7, s7, __ocmltbl_M64_Y1@rel32@hi+612
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0x3ff38000 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s5, exec_lo
	v_cmpx_ngt_f64_e32 0x3ff88000, v[2:3]
	s_cbranch_execz .LBB14_24
; %bb.21:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y1@rel32@lo+724
	s_addc_u32 s7, s7, __ocmltbl_M64_Y1@rel32@hi+732
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0x3ff88000 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s6, exec_lo
	v_cmpx_ngt_f64_e32 0x3ffd8000, v[2:3]
	s_cbranch_execz .LBB14_23
; %bb.22:
	v_cmp_gt_f64_e32 vcc_lo, 0x4000a000, v[2:3]
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y1@rel32@lo+844
	s_addc_u32 s9, s9, __ocmltbl_M64_Y1@rel32@hi+852
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v6, 0x3ffd8000 :: v_dual_mov_b32 v7, s9
	v_mov_b32_e32 v10, s8
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_Y1@rel32@lo+964
	s_addc_u32 s11, s11, __ocmltbl_M64_Y1@rel32@hi+972
	v_cndmask_b32_e64 v5, 0xbc8bd1e5, 0, vcc_lo
	v_cndmask_b32_e64 v4, 0xd219bfd, 0, vcc_lo
	v_cndmask_b32_e32 v9, 0x400193be, v6, vcc_lo
	v_cndmask_b32_e64 v8, 0xd4dff243, 0, vcc_lo
	v_cndmask_b32_e32 v7, s11, v7, vcc_lo
	v_cndmask_b32_e32 v6, s10, v10, vcc_lo
.LBB14_23:
	s_or_b32 exec_lo, exec_lo, s6
.LBB14_24:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
.LBB14_25:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB14_33
; %bb.26:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y1@rel32@lo+4
	s_addc_u32 s7, s7, __ocmltbl_M64_Y1@rel32@hi+12
	v_mov_b32_e32 v4, 0
	v_dual_mov_b32 v5, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s5, exec_lo
	v_cmpx_ngt_f64_e32 0.5, v[2:3]
	s_cbranch_execz .LBB14_32
; %bb.27:
	s_getpc_b64 s[6:7]
	s_add_u32 s6, s6, __ocmltbl_M64_Y1@rel32@lo+124
	s_addc_u32 s7, s7, __ocmltbl_M64_Y1@rel32@hi+132
	v_mov_b32_e32 v8, 0
	v_dual_mov_b32 v9, 0x3fe00000 :: v_dual_mov_b32 v6, s6
	v_mov_b32_e32 v7, s7
	s_mov_b32 s6, exec_lo
	v_cmpx_ngt_f64_e32 0x3fe40000, v[2:3]
	s_cbranch_execz .LBB14_31
; %bb.28:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y1@rel32@lo+244
	s_addc_u32 s9, s9, __ocmltbl_M64_Y1@rel32@hi+252
	v_mov_b32_e32 v8, 0
	v_dual_mov_b32 v9, 0x3fe40000 :: v_dual_mov_b32 v6, s8
	v_mov_b32_e32 v7, s9
	s_mov_b32 s7, exec_lo
	v_cmpx_ngt_f64_e32 0x3fe80000, v[2:3]
	s_cbranch_execz .LBB14_30
; %bb.29:
	v_cmp_gt_f64_e32 vcc_lo, 0x3fee0000, v[2:3]
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_Y1@rel32@lo+364
	s_addc_u32 s9, s9, __ocmltbl_M64_Y1@rel32@hi+372
	s_delay_alu instid0(SALU_CYCLE_1)
	v_dual_mov_b32 v6, 0x3fe80000 :: v_dual_mov_b32 v7, s9
	v_mov_b32_e32 v10, s8
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_Y1@rel32@lo+484
	s_addc_u32 s11, s11, __ocmltbl_M64_Y1@rel32@hi+492
	v_dual_mov_b32 v8, 0 :: v_dual_cndmask_b32 v9, 0x3fee0000, v6
	v_cndmask_b32_e32 v7, s11, v7, vcc_lo
	v_cndmask_b32_e32 v6, s10, v10, vcc_lo
.LBB14_30:
	s_or_b32 exec_lo, exec_lo, s7
.LBB14_31:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s6
.LBB14_32:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s5
.LBB14_33:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s4
.LBB14_34:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s1
	s_clause 0x4
	global_load_b64 v[30:31], v[6:7], off offset:112
	global_load_b128 v[10:13], v[6:7], off offset:96
	global_load_b128 v[14:17], v[6:7], off offset:80
	global_load_b128 v[18:21], v[6:7], off offset:64
	global_load_b128 v[22:25], v[6:7], off offset:48
	v_add_f64 v[8:9], v[2:3], -v[8:9]
	v_mul_f64 v[32:33], v[2:3], v[2:3]
	v_cmp_gt_f64_e32 vcc_lo, 0.5, v[2:3]
	global_load_b128 v[26:29], v[6:7], off offset:32
	v_add_f64 v[4:5], v[8:9], -v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v33, v5, v33 :: v_dual_cndmask_b32 v32, v4, v32
	s_waitcnt vmcnt(4)
	v_fma_f64 v[4:5], v[32:33], v[30:31], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[32:33], v[4:5], v[10:11]
	global_load_b128 v[8:11], v[6:7], off offset:16
	s_waitcnt vmcnt(4)
	v_fma_f64 v[4:5], v[32:33], v[4:5], v[16:17]
	v_fma_f64 v[12:13], v[32:33], v[4:5], v[14:15]
	global_load_b128 v[4:7], v[6:7], off
	s_waitcnt vmcnt(4)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[18:19]
	s_waitcnt vmcnt(3)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[22:23]
	s_waitcnt vmcnt(2)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[32:33], v[12:13], v[26:27]
	s_waitcnt vmcnt(1)
	v_fma_f64 v[10:11], v[32:33], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[32:33], v[10:11], v[8:9]
	s_waitcnt vmcnt(0)
	v_fma_f64 v[6:7], v[32:33], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[4:5], v[32:33], v[6:7], v[4:5]
	s_and_saveexec_b32 s1, vcc_lo
	s_cbranch_execz .LBB14_60
; %bb.35:
	s_mov_b32 s4, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
	v_cmpx_ngt_f64_e32 0x3de00000, v[2:3]
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB14_57
; %bb.36:
	s_mov_b32 s5, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
	v_cmpx_ge_f64_e32 0x40290800, v[2:3]
	s_xor_b32 s5, exec_lo, s5
	s_cbranch_execz .LBB14_50
; %bb.37:
	s_mov_b32 s6, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr10_vgpr11
                                        ; implicit-def: $vgpr8_vgpr9
	v_cmpx_ge_f64_e32 0x4018b000, v[2:3]
	s_xor_b32 s6, exec_lo, s6
	s_cbranch_execz .LBB14_43
; %bb.38:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J1@rel32@lo+4
	s_addc_u32 s9, s9, __ocmltbl_M64_J1@rel32@hi+12
	v_mov_b32_e32 v10, 0
	v_dual_mov_b32 v11, 0 :: v_dual_mov_b32 v8, 0
	v_dual_mov_b32 v9, 0 :: v_dual_mov_b32 v6, s8
	v_mov_b32_e32 v7, s9
	s_mov_b32 s7, exec_lo
	v_cmpx_nge_f64_e32 0x3ff18000, v[2:3]
	s_cbranch_execz .LBB14_42
; %bb.39:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J1@rel32@lo+124
	s_addc_u32 s9, s9, __ocmltbl_M64_J1@rel32@hi+132
	v_mov_b32_e32 v8, 0x20cfdaeb
	v_dual_mov_b32 v6, s8 :: v_dual_mov_b32 v7, s9
	v_mov_b32_e32 v9, 0x3c5616d8
	v_mov_b32_e32 v10, 0x1fec8a3a
	v_mov_b32_e32 v11, 0x3ffd757d
	s_mov_b32 s8, exec_lo
	v_cmpx_nge_f64_e32 0x4006c000, v[2:3]
	s_cbranch_execz .LBB14_41
; %bb.40:
	v_cmp_nge_f64_e32 vcc_lo, 0x40125000, v[2:3]
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_J1@rel32@lo+364
	s_addc_u32 s11, s11, __ocmltbl_M64_J1@rel32@hi+372
	v_mov_b32_e32 v8, 0x40155365
	v_dual_mov_b32 v6, s11 :: v_dual_mov_b32 v9, s10
	v_mov_b32_e32 v10, 0xbc032467
	v_mov_b32_e32 v12, 0x3ca5c646
	v_mov_b32_e32 v13, 0xa75d7539
	s_getpc_b64 s[12:13]
	s_add_u32 s12, s12, __ocmltbl_M64_J1@rel32@lo+244
	s_addc_u32 s13, s13, __ocmltbl_M64_J1@rel32@hi+252
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e32 v7, s13, v6, vcc_lo
	v_cndmask_b32_e32 v6, s12, v9, vcc_lo
	v_cndmask_b32_e32 v11, 0x400ea755, v8, vcc_lo
	v_cndmask_b32_e32 v10, 0x75af6f09, v10, vcc_lo
	v_cndmask_b32_e32 v9, 0xbca60155, v12, vcc_lo
	v_cndmask_b32_e32 v8, 0xa9d1b256, v13, vcc_lo
.LBB14_41:
	s_or_b32 exec_lo, exec_lo, s8
.LBB14_42:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s7
.LBB14_43:
	s_and_not1_saveexec_b32 s6, s6
	s_cbranch_execz .LBB14_49
; %bb.44:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J1@rel32@lo+484
	s_addc_u32 s9, s9, __ocmltbl_M64_J1@rel32@hi+492
	v_mov_b32_e32 v8, 0x9d243827
	v_dual_mov_b32 v6, s8 :: v_dual_mov_b32 v7, s9
	v_mov_b32_e32 v9, 0xbc9b226d
	v_mov_b32_e32 v10, 0xf3b47250
	v_mov_b32_e32 v11, 0x401c0ff5
	s_mov_b32 s7, exec_lo
	v_cmpx_nge_f64_e32 0x401f2000, v[2:3]
	s_cbranch_execz .LBB14_48
; %bb.45:
	s_getpc_b64 s[8:9]
	s_add_u32 s8, s8, __ocmltbl_M64_J1@rel32@lo+604
	s_addc_u32 s9, s9, __ocmltbl_M64_J1@rel32@hi+612
	v_mov_b32_e32 v8, 0xec20a31d
	v_dual_mov_b32 v6, s8 :: v_dual_mov_b32 v7, s9
	v_mov_b32_e32 v9, 0xbca63e17
	v_mov_b32_e32 v10, 0xf0b88a1
	v_mov_b32_e32 v11, 0x40211298
	s_mov_b32 s8, exec_lo
	v_cmpx_nge_f64_e32 0x4022b800, v[2:3]
	s_cbranch_execz .LBB14_47
; %bb.46:
	v_cmp_nge_f64_e32 vcc_lo, 0x4025e800, v[2:3]
	s_getpc_b64 s[10:11]
	s_add_u32 s10, s10, __ocmltbl_M64_J1@rel32@lo+844
	s_addc_u32 s11, s11, __ocmltbl_M64_J1@rel32@hi+852
	v_mov_b32_e32 v8, 0x40276979
	v_dual_mov_b32 v6, s11 :: v_dual_mov_b32 v9, s10
	v_mov_b32_e32 v10, 0x797ee5ac
	v_mov_b32_e32 v12, 0x3cc9a84d
	v_mov_b32_e32 v13, 0x3a5fedc2
	s_getpc_b64 s[12:13]
	s_add_u32 s12, s12, __ocmltbl_M64_J1@rel32@lo+724
	s_addc_u32 s13, s13, __ocmltbl_M64_J1@rel32@hi+732
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e32 v7, s13, v6, vcc_lo
	v_cndmask_b32_e32 v6, s12, v9, vcc_lo
	v_cndmask_b32_e32 v11, 0x402458d0, v8, vcc_lo
	v_cndmask_b32_e32 v10, 0xd0bdfc29, v10, vcc_lo
	v_cndmask_b32_e32 v9, 0x3cc02610, v12, vcc_lo
	v_cndmask_b32_e32 v8, 0xa51562b6, v13, vcc_lo
.LBB14_47:
	s_or_b32 exec_lo, exec_lo, s8
.LBB14_48:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s7
.LBB14_49:
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s6
	s_clause 0x4
	global_load_b64 v[28:29], v[6:7], off offset:112
	global_load_b128 v[12:15], v[6:7], off offset:96
	global_load_b128 v[16:19], v[6:7], off offset:80
	global_load_b128 v[20:23], v[6:7], off offset:64
	global_load_b128 v[24:27], v[6:7], off offset:48
	v_add_f64 v[10:11], v[2:3], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_f64 v[30:31], v[10:11], -v[8:9]
	global_load_b128 v[8:11], v[6:7], off offset:32
	s_waitcnt vmcnt(4)
	v_fma_f64 v[14:15], v[30:31], v[28:29], v[14:15]
	v_fma_f64 v[28:29], v[30:31], v[14:15], v[12:13]
	global_load_b128 v[12:15], v[6:7], off offset:16
	s_waitcnt vmcnt(4)
	v_fma_f64 v[18:19], v[30:31], v[28:29], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_fma_f64 v[28:29], v[30:31], v[18:19], v[16:17]
	global_load_b128 v[16:19], v[6:7], off
	s_waitcnt vmcnt(4)
	v_fma_f64 v[6:7], v[30:31], v[28:29], v[22:23]
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[20:21]
	s_waitcnt vmcnt(3)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[26:27]
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[24:25]
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[10:11]
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[8:9]
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[14:15]
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[12:13]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[18:19]
	v_fma_f64 v[6:7], v[30:31], v[6:7], v[16:17]
.LBB14_50:
	s_and_not1_saveexec_b32 s5, s5
	s_cbranch_execz .LBB14_56
; %bb.51:
	s_mov_b32 s6, exec_lo
                                        ; implicit-def: $vgpr10
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
	v_cmpx_ngt_f64_e32 0x41d00000, v[2:3]
	s_xor_b32 s6, exec_lo, s6
	s_cbranch_execz .LBB14_53
; %bb.52:
	v_ldexp_f64 v[6:7], v[2:3], 0xffffff80
	v_cmp_le_f64_e32 vcc_lo, 0x7b000000, v[2:3]
	v_trig_preop_f64 v[8:9], v[2:3], 0
	v_trig_preop_f64 v[10:11], v[2:3], 1
	v_trig_preop_f64 v[20:21], v[2:3], 2
	v_mov_b32_e32 v28, 0
	s_mov_b32 s8, 0x54442d18
	s_mov_b32 s9, 0x3ff921fb
	s_mov_b32 s10, 0x33145c07
	s_mov_b32 s11, 0x3c91a626
	v_dual_cndmask_b32 v7, v3, v7 :: v_dual_cndmask_b32 v6, v2, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[12:13], v[8:9], v[6:7]
	v_mul_f64 v[14:15], v[10:11], v[6:7]
	v_mul_f64 v[26:27], v[20:21], v[6:7]
	v_fma_f64 v[8:9], v[8:9], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[6:7], -v[14:15]
	v_fma_f64 v[6:7], v[20:21], v[6:7], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[14:15], v[8:9]
	v_add_f64 v[18:19], v[16:17], -v[14:15]
	v_add_f64 v[24:25], v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], -v[18:19]
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	v_ldexp_f64 v[18:19], v[24:25], -2
	v_add_f64 v[12:13], v[24:25], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_f64 v[22:23], v[26:27], v[10:11]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[18:19]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[16:17], -v[12:13]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_fract_f64_e32 v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[22:23], v[8:9]
	v_ldexp_f64 v[14:15], v[14:15], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[12:13], v[16:17]
	v_dual_cndmask_b32 v15, 0, v15 :: v_dual_cndmask_b32 v14, 0, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[18:19], v[14:15]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[24:25]
	v_add_f64 v[24:25], v[22:23], -v[26:27]
	v_cndmask_b32_e64 v29, 0, 0x40100000, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[33:34], v[22:23], -v[24:25]
	v_add_f64 v[10:11], v[10:11], -v[24:25]
	v_add_f64 v[14:15], v[14:15], v[28:29]
	v_add_f64 v[29:30], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[26:27], -v[33:34]
	v_add_f64 v[31:32], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[35:36], v[16:17], -v[29:30]
	v_add_f64 v[8:9], v[8:9], -v[29:30]
	v_add_f64 v[10:11], v[10:11], v[24:25]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_i32_f64_e32 v31, v[31:32]
	v_add_f64 v[22:23], v[22:23], -v[35:36]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_f64_i32_e32 v[29:30], v31
	v_add_f64 v[8:9], v[8:9], v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[29:30]
	v_add_f64 v[8:9], v[10:11], v[8:9]
	v_add_f64 v[10:11], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[18:19], v[14:15]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[20:21], -v[14:15]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[20:21]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[18:19], -v[8:9]
	v_cndmask_b32_e64 v29, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v10, null, 0, v31, vcc_lo
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[20:21], -v[28:29]
	v_add_f64 v[11:12], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[11:12], s[8:9]
	v_add_f64 v[8:9], v[11:12], -v[8:9]
	v_fma_f64 v[15:16], v[11:12], s[8:9], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[11:12], s[10:11], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], v[8:9]
	v_add_f64 v[6:7], v[13:14], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[6:7], -v[13:14]
	v_add_f64 v[8:9], v[8:9], -v[11:12]
.LBB14_53:
	s_and_not1_saveexec_b32 s6, s6
	s_cbranch_execz .LBB14_55
; %bb.54:
	s_mov_b32 s8, 0x6dc9c883
	s_mov_b32 s9, 0x3fe45f30
	s_mov_b32 s11, 0xbc91a626
	v_mul_f64 v[6:7], v[2:3], s[8:9]
	s_mov_b32 s8, 0x54442d18
	s_mov_b32 s9, 0xbff921fb
	s_mov_b32 s10, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[6:7]
	v_fma_f64 v[6:7], v[10:11], s[8:9], v[2:3]
	v_mul_f64 v[8:9], v[10:11], s[10:11]
	s_mov_b32 s8, 0x252049c0
	s_mov_b32 s9, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], s[10:11], v[6:7]
	v_add_f64 v[12:13], v[6:7], v[8:9]
	s_mov_b32 s11, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], v[10:11], s[10:11], v[8:9]
	v_add_f64 v[6:7], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[10:11], s[8:9], v[6:7]
	v_cvt_i32_f64_e32 v10, v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[14:15], v[8:9]
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
.LBB14_55:
	s_or_b32 exec_lo, exec_lo, s6
	v_div_scale_f64 v[11:12], null, v[2:3], v[2:3], 1.0
	v_div_scale_f64 v[17:18], vcc_lo, 1.0, v[2:3], 1.0
	s_mov_b32 s6, 0x95ed3e8e
	s_mov_b32 s8, 0x53d3a76e
	s_mov_b32 s7, 0xc1780a4d
	s_mov_b32 s9, 0x41bc22f6
	s_mov_b32 s10, 0x796cde01
	s_mov_b32 s11, 0x3ec71de3
	v_rcp_f64_e32 v[13:14], v[11:12]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[15:16], -v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[13:14], v[15:16], v[13:14]
	v_fma_f64 v[15:16], -v[11:12], v[13:14], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[13:14], v[15:16], v[13:14]
	v_mul_f64 v[15:16], v[17:18], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[11:12], v[15:16], v[17:18]
	v_div_fmas_f64 v[11:12], v[11:12], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[11:12], v[11:12], v[2:3], 1.0
	v_mul_f64 v[13:14], v[11:12], v[11:12]
	v_rsq_f64_e32 v[23:24], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], s[8:9], s[6:7]
	s_mov_b32 s6, 0x1f8cdd76
	s_mov_b32 s7, 0x41272f1d
	s_mov_b32 s8, 0x6621145
	s_mov_b32 s9, 0xc137940a
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[29:30], v[11:12], v[23:24]
	v_mul_f64 v[23:24], v[23:24], 0.5
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[6:7]
	s_mov_b32 s6, 0x96460ad7
	s_mov_b32 s7, 0xc0d3ea4e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], -v[23:24], v[29:30], 0.5
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[6:7]
	s_mov_b32 s6, 0x98d9ab3a
	s_mov_b32 s7, 0x408488dd
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[29:30], v[29:30], v[31:32], v[29:30]
	v_fma_f64 v[23:24], v[23:24], v[31:32], v[23:24]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[6:7]
	s_mov_b32 s6, 0x12fa3b38
	s_mov_b32 s7, 0xc03e9ed6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[31:32], -v[29:30], v[29:30], v[11:12]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[6:7]
	s_mov_b32 s6, 0xfcab9dda
	s_mov_b32 s7, 0x4002f484
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[6:7]
	s_mov_b32 s6, 0xcad443c0
	s_mov_b32 s7, 0xbfd7bccc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[6:7]
	s_movk_i32 s6, 0xcbfa
	s_mov_b32 s7, 0x3fc4ffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[6:7]
	s_mov_b32 s6, 0x68428baf
	s_mov_b32 s7, 0x40f591fb
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], 0xbfd80000
	v_mul_f64 v[15:16], v[11:12], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_cmp_lt_f64_e32 vcc_lo, v[6:7], v[15:16]
	v_subrev_co_ci_u32_e64 v39, null, 0, v10, vcc_lo
	v_cndmask_b32_e64 v10, 0, 0x80000000, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, 0, v[11:12]
	v_mov_b32_e32 v17, 0x54442d18
	v_xor_b32_e32 v18, 0xbfe921fb, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[19:20], v[17:18], -v[15:16]
	v_add_f64 v[17:18], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[17:18], -v[15:16]
	v_mov_b32_e32 v17, 0x33145c07
	v_xor_b32_e32 v18, 0xbc81a626, v10
	v_add_f64 v[15:16], v[15:16], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[19:20], v[15:16]
	v_add_f64 v[21:22], v[6:7], v[17:18]
	v_add_f64 v[19:20], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[21:22], -v[17:18]
	v_add_f64 v[15:16], v[15:16], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[17:18]
	v_add_f64 v[8:9], v[8:9], v[15:16]
	v_fma_f64 v[17:18], v[13:14], s[8:9], s[6:7]
	s_mov_b32 s6, 0xb42fdfa7
	s_mov_b32 s8, 0xf9a43bb8
	s_mov_b32 s7, 0xbe5ae600
	s_mov_b32 s9, 0x3de5e0b2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[21:22], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[15:16], v[8:9], v[8:9]
	v_add_f64 v[21:22], v[8:9], -v[21:22]
	v_fma_f64 v[19:20], v[15:16], s[8:9], s[6:7]
	s_mov_b32 s6, 0x2a8bafb0
	s_mov_b32 s7, 0xc0a99655
	s_mov_b32 s8, 0x46cc5e42
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0x9037ab78
	s_mov_b32 s7, 0x3e21eeb6
	s_mov_b32 s9, 0xbda907db
	v_mul_f64 v[27:28], v[15:16], 0.5
	v_fma_f64 v[25:26], v[15:16], s[8:9], s[6:7]
	s_mov_b32 s6, 0x78cd8c93
	s_mov_b32 s7, 0x40607955
	s_mov_b32 s8, 0x19e83e5c
	s_mov_b32 s9, 0xbf2a01a0
	v_add_f64 v[6:7], v[6:7], -v[21:22]
	v_mul_f64 v[33:34], v[8:9], -v[15:16]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[10:11]
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0xa17f65f6
	s_mov_b32 s7, 0xbe927e4f
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[21:22], v[15:16], v[25:26], s[6:7]
	v_add_f64 v[25:26], -v[27:28], 1.0
	s_mov_b32 s6, 0x64596b5a
	s_mov_b32 s7, 0xc01ef383
	v_mul_f64 v[35:36], v[6:7], 0.5
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[8:9]
	s_mov_b32 s8, 0x11110bb3
	s_mov_b32 s9, 0x3f811111
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0x19f4ec90
	s_mov_b32 s7, 0x3efa01a0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_fma_f64 v[21:22], v[15:16], v[21:22], s[6:7]
	v_add_f64 v[37:38], -v[25:26], 1.0
	s_mov_b32 s6, 0x465744c7
	s_mov_b32 s7, 0x3fe9c4fa
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[8:9]
	s_movk_i32 s8, 0xc240
	s_mov_b32 s9, 0x3fc7ffff
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0x16c16967
	s_mov_b32 s7, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[21:22], v[15:16], v[21:22], s[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_add_f64 v[27:28], v[37:38], -v[27:28]
	s_mov_b32 s6, 0xfc3937c1
	s_mov_b32 s7, 0xbfc8bfff
	v_fma_f64 v[19:20], v[33:34], v[19:20], v[35:36]
	v_mul_f64 v[35:36], v[15:16], v[15:16]
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s7, 0x3fa55555
	s_mov_b32 s6, 0x55555555
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[19:20], v[15:16], v[19:20], -v[6:7]
	v_fma_f64 v[15:16], v[15:16], v[21:22], s[6:7]
	v_fma_f64 v[6:7], v[8:9], -v[6:7], v[27:28]
	v_fma_f64 v[21:22], v[31:32], v[23:24], v[29:30]
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[8:9]
	s_mov_b32 s7, 0xbfc55555
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[19:20], v[33:34], s[6:7], v[19:20]
	s_mov_b32 s6, 0x33d43651
	v_fma_f64 v[6:7], v[35:36], v[15:16], v[6:7]
	s_delay_alu instid0(VALU_DEP_4)
	v_dual_cndmask_b32 v12, v22, v12 :: v_dual_cndmask_b32 v11, v21, v11
	s_mov_b32 s7, 0x3fe98845
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[10:11], v[11:12], s[6:7]
	v_fma_f64 v[12:13], v[13:14], v[17:18], 1.0
	v_add_f64 v[8:9], v[8:9], -v[19:20]
	v_add_f64 v[6:7], v[25:26], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[10:11], v[12:13]
	v_add_nc_u32_e32 v12, -1, v39
	v_and_b32_e32 v13, 1, v12
	v_lshlrev_b32_e32 v12, 30, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_eq_u32_e32 vcc_lo, 0, v13
	v_xor_b32_e32 v9, 0x80000000, v9
	v_cndmask_b32_e32 v6, v8, v6, vcc_lo
	v_dual_cndmask_b32 v7, v9, v7 :: v_dual_and_b32 v8, 0x80000000, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v7, v7, v8
	v_mul_f64 v[6:7], v[10:11], v[6:7]
.LBB14_56:
	s_or_b32 exec_lo, exec_lo, s5
	v_frexp_mant_f64_e32 v[8:9], v[2:3]
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	v_div_scale_f64 v[24:25], null, v[2:3], v[2:3], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[8:9]
	s_mov_b32 s6, 0x55555780
	v_rcp_f64_e32 v[28:29], v[24:25]
	v_cndmask_b32_e64 v10, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v10
	v_add_f64 v[10:11], v[8:9], 1.0
	v_add_f64 v[16:17], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[12:13], v[10:11]
	v_add_f64 v[18:19], v[10:11], -1.0
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_mul_f64 v[14:15], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[10:11], v[14:15]
	v_fma_f64 v[10:11], v[14:15], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[20:21], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	v_add_f64 v[20:21], v[10:11], -v[20:21]
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[20:21], -v[8:9]
	v_frexp_exp_i32_f64_e32 v20, v[2:3]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[8:9], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	v_add_f64 v[10:11], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[10:11], v[10:11]
	v_fma_f64 v[16:17], v[12:13], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[18:19], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[12:13], v[16:17], s[6:7]
	v_ldexp_f64 v[16:17], v[10:11], 1
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0x3fe62e42
	v_mul_f64 v[12:13], v[18:19], v[12:13]
	v_subrev_co_ci_u32_e64 v18, null, 0, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_cvt_f64_i32_e32 v[18:19], v18
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[16:17], v[12:13]
	v_ldexp_f64 v[8:9], v[8:9], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[20:21], v[18:19], s[6:7]
	v_add_f64 v[10:11], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[18:19], s[6:7], -v[20:21]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0x3c7abc9e
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[18:19], s[6:7], v[16:17]
	s_mov_b32 s6, 0x6dc9c883
	s_mov_b32 s7, 0x3fe45f30
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
	v_add_f64 v[26:27], v[18:19], -v[22:23]
	v_add_f64 v[14:15], v[16:17], -v[22:23]
	v_fma_f64 v[16:17], -v[24:25], v[28:29], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[12:13], v[8:9]
	v_add_f64 v[10:11], v[10:11], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[14:15], v[10:11]
	v_fma_f64 v[14:15], v[28:29], v[16:17], v[28:29]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[20:21], -v[12:13]
	v_div_scale_f64 v[28:29], vcc_lo, -1.0, v[2:3], -1.0
	v_add_f64 v[10:11], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[22:23], -v[24:25], v[14:15], 1.0
	v_add_f64 v[20:21], v[20:21], -v[16:17]
	v_add_f64 v[8:9], v[8:9], -v[16:17]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[26:27], v[18:19], v[10:11]
	v_fma_f64 v[14:15], v[14:15], v[22:23], v[14:15]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[16:17], v[26:27], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[28:29], v[14:15]
	v_add_f64 v[8:9], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_fma_f64 v[12:13], -v[24:25], v[18:19], v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_div_fmas_f64 v[10:11], v[12:13], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[26:27], v[8:9]
	v_div_fixup_f64 v[10:11], v[10:11], v[2:3], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], v[10:11]
	v_mul_f64 v[6:7], v[6:7], s[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[6:7], v[4:5], v[2:3], v[6:7]
.LBB14_57:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB14_59
; %bb.58:
	v_dual_mov_b32 v4, v2 :: v_dual_and_b32 v5, 0x7fffffff, v3
	s_mov_b32 s6, 0x6dc9c883
	s_mov_b32 s7, 0xbfe45f30
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], s[6:7]
	v_div_scale_f64 v[4:5], vcc_lo, s[6:7], v[4:5], s[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[4:5], v[8:9]
	v_fma_f64 v[4:5], -v[6:7], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[4:5], v[4:5], v[8:9], v[10:11]
	v_div_fixup_f64 v[6:7], v[4:5], |v[2:3]|, s[6:7]
.LBB14_59:
	s_or_b32 exec_lo, exec_lo, s4
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v5, 0x7ff80000, v7, vcc_lo
	v_cndmask_b32_e32 v4, 0, v6, vcc_lo
.LBB14_60:
	s_or_b32 exec_lo, exec_lo, s1
                                        ; implicit-def: $vgpr2_vgpr3
.LBB14_61:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB14_67
; %bb.62:
	s_mov_b32 s1, exec_lo
                                        ; implicit-def: $vgpr8
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr6_vgpr7
	v_cmpx_ngt_f64_e32 0x41d00000, v[2:3]
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB14_64
; %bb.63:
	v_ldexp_f64 v[4:5], v[2:3], 0xffffff80
	v_cmp_le_f64_e32 vcc_lo, 0x7b000000, v[2:3]
	v_trig_preop_f64 v[6:7], v[2:3], 0
	v_trig_preop_f64 v[8:9], v[2:3], 1
	v_trig_preop_f64 v[18:19], v[2:3], 2
	v_mov_b32_e32 v26, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_dual_cndmask_b32 v5, v3, v5 :: v_dual_cndmask_b32 v4, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[6:7], v[4:5]
	v_mul_f64 v[12:13], v[8:9], v[4:5]
	v_mul_f64 v[24:25], v[18:19], v[4:5]
	v_fma_f64 v[6:7], v[6:7], v[4:5], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[4:5], -v[12:13]
	v_fma_f64 v[4:5], v[18:19], v[4:5], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[12:13], v[6:7]
	v_add_f64 v[16:17], v[14:15], -v[12:13]
	v_add_f64 v[22:23], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	v_ldexp_f64 v[16:17], v[22:23], -2
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[20:21], v[24:25], v[8:9]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[16:17]|
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_fract_f64_e32 v[12:13], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[20:21], v[6:7]
	v_ldexp_f64 v[12:13], v[12:13], 2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], v[14:15]
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[16:17], v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[22:23]
	v_add_f64 v[22:23], v[20:21], -v[24:25]
	v_cndmask_b32_e64 v27, 0, 0x40100000, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[31:32], v[20:21], -v[22:23]
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	v_add_f64 v[12:13], v[12:13], v[26:27]
	v_add_f64 v[27:28], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[24:25], -v[31:32]
	v_add_f64 v[29:30], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[33:34], v[14:15], -v[27:28]
	v_add_f64 v[6:7], v[6:7], -v[27:28]
	v_add_f64 v[8:9], v[8:9], v[22:23]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cvt_i32_f64_e32 v29, v[29:30]
	v_add_f64 v[20:21], v[20:21], -v[33:34]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_f64_i32_e32 v[27:28], v29
	v_add_f64 v[6:7], v[6:7], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[27:28]
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[16:17], v[12:13]
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[18:19], -v[12:13]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[18:19]
	v_add_f64 v[4:5], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_cndmask_b32_e64 v27, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v8, null, 0, v29, vcc_lo
	v_add_f64 v[4:5], v[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], -v[26:27]
	v_add_f64 v[9:10], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[11:12], v[9:10], s[4:5]
	v_add_f64 v[6:7], v[9:10], -v[6:7]
	v_fma_f64 v[13:14], v[9:10], s[4:5], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_fma_f64 v[6:7], v[9:10], s[6:7], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[4:5], s[4:5], v[6:7]
	v_add_f64 v[4:5], v[11:12], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[4:5], -v[11:12]
	v_add_f64 v[6:7], v[6:7], -v[9:10]
.LBB14_64:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB14_66
; %bb.65:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[4:5], v[2:3], s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[4:5]
	v_fma_f64 v[4:5], v[8:9], s[4:5], v[2:3]
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
.LBB14_66:
	s_or_b32 exec_lo, exec_lo, s1
	v_div_scale_f64 v[9:10], null, v[2:3], v[2:3], 1.0
	v_div_scale_f64 v[15:16], vcc_lo, 1.0, v[2:3], 1.0
	s_mov_b32 s4, 0x95ed3e8e
	s_mov_b32 s6, 0x53d3a76e
	s_mov_b32 s5, 0xc1780a4d
	s_mov_b32 s7, 0x41bc22f6
	s_mov_b32 s8, 0x796cde01
	s_mov_b32 s9, 0x3ec71de3
	v_rcp_f64_e32 v[11:12], v[9:10]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[11:12], v[13:14], v[11:12]
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[11:12], v[13:14], v[11:12]
	v_mul_f64 v[13:14], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[9:10], v[13:14], v[15:16]
	v_div_fmas_f64 v[9:10], v[9:10], v[11:12], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[9:10], v[9:10], v[2:3], 1.0
	v_mul_f64 v[11:12], v[9:10], v[9:10]
	v_rsq_f64_e32 v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], s[6:7], s[4:5]
	s_mov_b32 s4, 0x1f8cdd76
	s_mov_b32 s5, 0x41272f1d
	s_mov_b32 s6, 0x6621145
	s_mov_b32 s7, 0xc137940a
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[27:28], v[9:10], v[21:22]
	v_mul_f64 v[21:22], v[21:22], 0.5
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x96460ad7
	s_mov_b32 s5, 0xc0d3ea4e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], -v[21:22], v[27:28], 0.5
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x98d9ab3a
	s_mov_b32 s5, 0x408488dd
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[27:28], v[27:28], v[29:30], v[27:28]
	v_fma_f64 v[21:22], v[21:22], v[29:30], v[21:22]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x12fa3b38
	s_mov_b32 s5, 0xc03e9ed6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[29:30], -v[27:28], v[27:28], v[9:10]
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xfcab9dda
	s_mov_b32 s5, 0x4002f484
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0xcad443c0
	s_mov_b32 s5, 0xbfd7bccc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_movk_i32 s4, 0xcbfa
	s_mov_b32 s5, 0x3fc4ffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[4:5]
	s_mov_b32 s4, 0x68428baf
	s_mov_b32 s5, 0x40f591fb
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], 0xbfd80000
	v_mul_f64 v[13:14], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_cmp_lt_f64_e32 vcc_lo, v[4:5], v[13:14]
	v_subrev_co_ci_u32_e64 v37, null, 0, v8, vcc_lo
	v_cndmask_b32_e64 v8, 0, 0x80000000, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, 0, v[9:10]
	v_mov_b32_e32 v15, 0x54442d18
	v_xor_b32_e32 v16, 0xbfe921fb, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[15:16], -v[13:14]
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_mov_b32_e32 v15, 0x33145c07
	v_xor_b32_e32 v16, 0xbc81a626, v8
	v_add_f64 v[13:14], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[17:18], v[13:14]
	v_add_f64 v[19:20], v[4:5], v[15:16]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[19:20], -v[15:16]
	v_add_f64 v[13:14], v[13:14], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], -v[15:16]
	v_add_f64 v[6:7], v[6:7], v[13:14]
	v_fma_f64 v[15:16], v[11:12], s[6:7], s[4:5]
	s_mov_b32 s4, 0xb42fdfa7
	s_mov_b32 s6, 0xf9a43bb8
	s_mov_b32 s5, 0xbe5ae600
	s_mov_b32 s7, 0x3de5e0b2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[19:20], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[6:7], v[6:7]
	v_add_f64 v[19:20], v[6:7], -v[19:20]
	v_fma_f64 v[17:18], v[13:14], s[6:7], s[4:5]
	s_mov_b32 s4, 0x2a8bafb0
	s_mov_b32 s5, 0xc0a99655
	s_mov_b32 s6, 0x46cc5e42
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x9037ab78
	s_mov_b32 s5, 0x3e21eeb6
	s_mov_b32 s7, 0xbda907db
	v_mul_f64 v[25:26], v[13:14], 0.5
	v_fma_f64 v[23:24], v[13:14], s[6:7], s[4:5]
	s_mov_b32 s4, 0x78cd8c93
	s_mov_b32 s5, 0x40607955
	s_mov_b32 s6, 0x19e83e5c
	s_mov_b32 s7, 0xbf2a01a0
	v_add_f64 v[4:5], v[4:5], -v[19:20]
	v_mul_f64 v[31:32], v[6:7], -v[13:14]
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[8:9]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0xa17f65f6
	s_mov_b32 s5, 0xbe927e4f
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[23:24], s[4:5]
	v_add_f64 v[23:24], -v[25:26], 1.0
	s_mov_b32 s4, 0x64596b5a
	s_mov_b32 s5, 0xc01ef383
	v_mul_f64 v[33:34], v[4:5], 0.5
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_mov_b32 s6, 0x11110bb3
	s_mov_b32 s7, 0x3f811111
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x19f4ec90
	s_mov_b32 s5, 0x3efa01a0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	v_add_f64 v[35:36], -v[23:24], 1.0
	s_mov_b32 s4, 0x465744c7
	s_mov_b32 s5, 0x3fe9c4fa
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[6:7]
	s_movk_i32 s6, 0xc240
	s_mov_b32 s7, 0x3fc7ffff
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s4, 0x16c16967
	s_mov_b32 s5, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_add_f64 v[25:26], v[35:36], -v[25:26]
	s_mov_b32 s4, 0xfc3937c1
	s_mov_b32 s5, 0xbfc8bfff
	v_fma_f64 v[17:18], v[31:32], v[17:18], v[33:34]
	v_mul_f64 v[33:34], v[13:14], v[13:14]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[4:5]
	s_mov_b32 s5, 0x3fa55555
	s_mov_b32 s4, 0x55555555
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[13:14], v[19:20], s[4:5]
	v_fma_f64 v[25:26], v[6:7], -v[4:5], v[25:26]
	s_mov_b32 s5, 0xbfc55555
	v_fma_f64 v[4:5], v[13:14], v[17:18], -v[4:5]
	v_fma_f64 v[13:14], v[29:30], v[21:22], v[27:28]
	v_fma_f64 v[15:16], v[11:12], v[15:16], s[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[17:18], v[33:34], v[19:20], v[25:26]
	v_fma_f64 v[4:5], v[31:32], s[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_4)
	v_dual_cndmask_b32 v10, v14, v10 :: v_dual_cndmask_b32 v9, v13, v9
	s_mov_b32 s4, 0x33d43651
	s_mov_b32 s5, 0x3fe98845
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[8:9], v[9:10], s[4:5]
	v_fma_f64 v[10:11], v[11:12], v[15:16], 1.0
	v_add_f64 v[12:13], v[23:24], v[17:18]
	v_add_f64 v[4:5], v[6:7], -v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[8:9], v[10:11]
	v_add_nc_u32_e32 v8, -1, v37
	v_and_b32_e32 v9, 1, v8
	v_lshlrev_b32_e32 v8, 30, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_eq_u32_e32 vcc_lo, 0, v9
	v_dual_cndmask_b32 v5, v13, v5 :: v_dual_and_b32 v8, 0x80000000, v8
	v_cndmask_b32_e32 v4, v12, v4, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v5, v5, v8
	v_mul_f64 v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
.LBB14_67:
	s_or_b32 exec_lo, exec_lo, s0
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB14_68:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_y1_kernel
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
		.amdhsa_next_free_vgpr 40
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
		.amdhsa_inst_pref_size 63
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
	.size	specialx_y1_kernel, .Lfunc_end14-specialx_y1_kernel
                                        ; -- End function
	.set specialx_y1_kernel.num_vgpr, 40
	.set specialx_y1_kernel.num_agpr, 0
	.set specialx_y1_kernel.numbered_sgpr, 14
	.set specialx_y1_kernel.num_named_barrier, 0
	.set specialx_y1_kernel.private_seg_size, 0
	.set specialx_y1_kernel.uses_vcc, 1
	.set specialx_y1_kernel.uses_flat_scratch, 0
	.set specialx_y1_kernel.has_dyn_sized_stack, 0
	.set specialx_y1_kernel.has_recursion, 0
	.set specialx_y1_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 8428
; TotalNumSgprs: 16
; NumVgprs: 40
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 16
; NumVGPRsForWavesPerEU: 40
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
	.protected	specialx_ndtr_kernel    ; -- Begin function specialx_ndtr_kernel
	.globl	specialx_ndtr_kernel
	.p2align	8
	.type	specialx_ndtr_kernel,@function
specialx_ndtr_kernel:                   ; @specialx_ndtr_kernel
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
	s_cbranch_execz .LBB15_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_brev_b32 s4, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0xdce2b7d6
	s_mov_b32 s1, 0x40434d4e
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e64 vcc_lo, |v[2:3]|, s[0:1]
	v_and_or_b32 v4, v3, s4, 0x40434d4e
	s_mov_b32 s0, 0x667f3bcd
	s_mov_b32 s1, 0xbfe6a09e
	s_mov_b32 s4, 0x54df3c0e
	s_mov_b32 s5, 0xbe41f39d
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	v_cndmask_b32_e64 v2, v2, 0xdce2b7d6, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	v_fma_f64 v[6:7], v[2:3], s[0:1], -v[4:5]
	s_mov_b32 s0, 0x13b26456
	s_mov_b32 s1, 0x3c8bdd34
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[2:3], s[0:1], v[6:7]
	s_mov_b32 s0, 0x37cfa789
	s_mov_b32 s1, 0xbe411663
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[4:5], v[6:7]
	v_add_f64 v[10:11], |v[8:9]|, 4.0
	v_add_f64 v[22:23], |v[8:9]|, |v[8:9]|
	v_add_f64 v[4:5], v[8:9], -v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[12:13], v[10:11]
	v_add_f64 v[24:25], v[22:23], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[26:27], v[24:25]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_add_f64 v[14:15], |v[8:9]|, -4.0
	v_fma_f64 v[28:29], -v[24:25], v[26:27], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], -v[10:11], v[12:13], 1.0
	v_fma_f64 v[26:27], v[28:29], v[26:27], v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[12:13]
	v_fma_f64 v[24:25], -v[24:25], v[26:27], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_fma_f64 v[24:25], v[24:25], v[26:27], v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[12:13], 1.0
	v_fma_f64 v[14:15], v[14:15], -4.0, |v[8:9]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[12:13], |v[8:9]|, v[14:15]
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[12:13]
	v_mul_f64 v[14:15], v[8:9], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], s[4:5], s[0:1]
	s_mov_b32 s0, 0xd9802b82
	s_mov_b32 s1, 0x3e7b45f1
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[14:15]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x8a03dcdb
	s_mov_b32 s1, 0x3e6d9048
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x2eba62d8
	s_mov_b32 s1, 0xbeab87b0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0xa56e15f1
	s_mov_b32 s1, 0x3e95104b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x71c907de
	s_mov_b32 s1, 0x3ed7f29f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x2cd770fb
	s_mov_b32 s1, 0xbee78f5c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mul_f64 v[16:17], v[14:15], s[0:1]
	s_mov_b32 s0, 0x76d0a51a
	s_mov_b32 s1, 0xbef995fb
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0xc022d0ed
	s_mov_b32 s1, 0x3f23be2e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_rndne_f64_e32 v[16:17], v[16:17]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], s[0:1], v[14:15]
	s_mov_b32 s0, 0x2fdbf62e
	s_mov_b32 s1, 0xbf2a1deb
	v_cvt_i32_f64_e32 v26, v[16:17]
	s_delay_alu instid0(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[18:19], v[16:17], s[0:1], v[18:19]
	s_mov_b32 s0, 0x3689fc43
	s_mov_b32 s1, 0xbf48d4ac
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], s[4:5], s[0:1]
	s_mov_b32 s0, 0x192d909b
	s_mov_b32 s1, 0x3f749c67
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[0:1]
	s_mov_b32 s0, 0x852ff070
	s_mov_b32 s1, 0xbf909623
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[0:1]
	s_mov_b32 s0, 0xdfadea8f
	s_mov_b32 s1, 0x3fa3079e
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[0:1]
	s_mov_b32 s0, 0xdff65910
	s_mov_b32 s1, 0xbfb0fb06
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[0:1]
	s_mov_b32 s0, 0x4de8f32
	s_mov_b32 s1, 0x3fb7fee0
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[0:1]
	s_mov_b32 s0, 0x3c3dbeb3
	s_mov_b32 s1, 0xbfb9ddb2
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[0:1]
	s_mov_b32 s0, 0xfcfa6930
	s_mov_b32 s1, 0x3fb16ece
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[0:1]
	s_mov_b32 s0, 0xf66fb8a3
	s_mov_b32 s1, 0x3f8f7f5d
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[20:21], v[18:19], v[20:21], s[0:1]
	s_mov_b32 s0, 0xd154a2a8
	s_mov_b32 s1, 0xbfc1df1a
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0xb74febf8
	s_mov_b32 s1, 0x3fcdd2c8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[20:21], v[18:19], v[20:21], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], s[0:1]
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[18:19], v[20:21], 1.0
	v_fma_f64 v[16:17], v[10:11], v[24:25], v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[12:13], v[12:13], v26
	v_fma_f64 v[18:19], -v[16:17], v[22:23], 1.0
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v20, 0x7ff00000, v13, vcc_lo
	v_fma_f64 v[13:14], -v[8:9], v[8:9], -v[14:15]
	s_and_b32 vcc_lo, s0, vcc_lo
	v_add_f64 v[10:11], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v19, 0, v20, s0
	v_cndmask_b32_e32 v18, 0, v12, vcc_lo
	s_mov_b32 s0, 0x41e48bfc
	s_mov_b32 s1, 0x403b39dc
	v_cmp_ngt_f64_e64 vcc_lo, |v[8:9]|, s[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[18:19], v[13:14], v[18:19]
	v_mul_f64 v[14:15], v[8:9], -2.0
	v_fma_f64 v[10:11], v[24:25], v[10:11], v[16:17]
	v_mul_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v11, 0, v11 :: v_dual_cndmask_b32 v10, 0, v10
	v_cmp_gt_f64_e32 vcc_lo, 0, v[8:9]
	v_add_f64 v[12:13], -v[10:11], 2.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v9, v11, v13 :: v_dual_cndmask_b32 v8, v10, v12
	v_cmp_nle_f64_e32 vcc_lo, -1.0, v[2:3]
	v_add_f64 v[2:3], v[6:7], -v[4:5]
	v_mul_f64 v[10:11], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v5, 0, v11 :: v_dual_cndmask_b32 v4, 0, v10
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[2:3], v[2:3], v[4:5], v[8:9]
	v_mul_f64 v[2:3], v[2:3], 0.5
	global_store_b64 v[0:1], v[2:3], off
.LBB15_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_ndtr_kernel
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
.Lfunc_end15:
	.size	specialx_ndtr_kernel, .Lfunc_end15-specialx_ndtr_kernel
                                        ; -- End function
	.set specialx_ndtr_kernel.num_vgpr, 30
	.set specialx_ndtr_kernel.num_agpr, 0
	.set specialx_ndtr_kernel.numbered_sgpr, 6
	.set specialx_ndtr_kernel.num_named_barrier, 0
	.set specialx_ndtr_kernel.private_seg_size, 0
	.set specialx_ndtr_kernel.uses_vcc, 1
	.set specialx_ndtr_kernel.uses_flat_scratch, 0
	.set specialx_ndtr_kernel.has_dyn_sized_stack, 0
	.set specialx_ndtr_kernel.has_recursion, 0
	.set specialx_ndtr_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1704
; TotalNumSgprs: 8
; NumVgprs: 30
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 8
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
	.text
	.protected	specialx_ndtri_kernel   ; -- Begin function specialx_ndtri_kernel
	.globl	specialx_ndtri_kernel
	.p2align	8
	.type	specialx_ndtri_kernel,@function
specialx_ndtri_kernel:                  ; @specialx_ndtri_kernel
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
	s_cbranch_execz .LBB16_53
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
                                        ; implicit-def: $vgpr4_vgpr5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, exec_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_add_f64 v[2:3], v[2:3], v[2:3]
	v_cmpx_nlt_f64_e32 0x3fe40000, v[2:3]
	s_xor_b32 s0, exec_lo, s0
	s_cbranch_execz .LBB16_31
; %bb.2:
	s_mov_b32 s1, exec_lo
                                        ; implicit-def: $vgpr4_vgpr5
	v_cmpx_nlt_f64_e32 0x3f500000, v[2:3]
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB16_28
; %bb.3:
	v_frexp_mant_f64_e32 v[4:5], v[2:3]
	s_mov_b32 s5, 0x3fe55555
	s_mov_b32 s4, 0x55555555
	s_mov_b32 s6, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s7, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[4:5]
	s_mov_b32 s4, 0x55555780
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
	v_fma_f64 v[12:13], v[8:9], s[8:9], s[6:7]
	s_mov_b32 s6, 0xd7f4df2e
	s_mov_b32 s7, 0x3fc7474d
	v_mul_f64 v[14:15], v[6:7], v[8:9]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s6, 0x16291751
	s_mov_b32 s7, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s6, 0x9b27acf1
	s_mov_b32 s7, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_mov_b32 s6, 0x998ef7b6
	s_mov_b32 s7, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[8:9], v[8:9], v[12:13], s[4:5]
	v_ldexp_f64 v[12:13], v[6:7], 1
	v_add_f64 v[6:7], v[6:7], -v[10:11]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0x3fe62e42
	v_mul_f64 v[8:9], v[14:15], v[8:9]
	v_subrev_co_ci_u32_e64 v14, null, 0, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_cmp_class_f64_e64 vcc_lo, v[2:3], 0x204
	v_cvt_f64_i32_e32 v[14:15], v14
	v_add_f64 v[10:11], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[4:5], v[4:5], 1
	v_mul_f64 v[16:17], v[14:15], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[10:11], -v[12:13]
	v_fma_f64 v[12:13], v[14:15], s[4:5], -v[16:17]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	v_fma_f64 v[8:9], v[14:15], s[4:5], v[12:13]
	s_mov_b32 s4, exec_lo
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
	v_cndmask_b32_e32 v4, v4, v2, vcc_lo
	v_cndmask_b32_e64 v5, -v5, -v3, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v5, 0xfff80000, v5, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v4, 0, v4, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[4:5]
	v_cndmask_b32_e64 v6, 0, 0x100, vcc_lo
	v_ldexp_f64 v[4:5], v[4:5], v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_rsq_f64_e32 v[6:7], v[4:5]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[8:9], v[4:5], v[6:7]
	v_mul_f64 v[6:7], v[6:7], 0.5
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_fma_f64 v[6:7], v[6:7], v[10:11], v[6:7]
	v_fma_f64 v[10:11], -v[8:9], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[10:11], v[6:7], v[8:9]
	v_fma_f64 v[10:11], -v[8:9], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[10:11], v[6:7], v[8:9]
	v_cndmask_b32_e64 v8, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x260
	v_ldexp_f64 v[6:7], v[6:7], v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v5, v7, v5 :: v_dual_cndmask_b32 v4, v6, v4
	v_div_scale_f64 v[6:7], null, v[4:5], v[4:5], 1.0
	v_div_scale_f64 v[12:13], vcc_lo, 1.0, v[4:5], 1.0
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
                                        ; implicit-def: $vgpr8_vgpr9
	v_div_fixup_f64 v[6:7], v[6:7], v[4:5], 1.0
	v_cmpx_nlt_f64_e32 0x3ec00000, v[2:3]
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB16_25
; %bb.4:
	s_mov_b32 s5, exec_lo
                                        ; implicit-def: $vgpr8_vgpr9
	v_cmpx_nlt_f64_e32 0x3d700000, v[2:3]
	s_xor_b32 s5, exec_lo, s5
	s_cbranch_execz .LBB16_22
; %bb.5:
	s_mov_b32 s6, exec_lo
                                        ; implicit-def: $vgpr8_vgpr9
	v_cmpx_nlt_f64_e32 0x3ad00000, v[2:3]
	s_xor_b32 s6, exec_lo, s6
	s_cbranch_execz .LBB16_19
; %bb.6:
	s_mov_b32 s7, exec_lo
                                        ; implicit-def: $vgpr8_vgpr9
	v_cmpx_nlt_f64_e32 0x33700000, v[2:3]
	s_xor_b32 s7, exec_lo, s7
	s_cbranch_execz .LBB16_16
; %bb.7:
	s_mov_b32 s8, exec_lo
                                        ; implicit-def: $vgpr8_vgpr9
	v_cmpx_nlt_f64_e32 0x26f00000, v[2:3]
	s_xor_b32 s8, exec_lo, s8
	s_cbranch_execz .LBB16_13
; %bb.8:
	s_mov_b32 s9, exec_lo
                                        ; implicit-def: $vgpr8_vgpr9
	v_cmpx_nlt_f64_e32 0x7b00000, v[2:3]
	s_xor_b32 s9, exec_lo, s9
	s_cbranch_execz .LBB16_10
; %bb.9:
	s_mov_b32 s10, 0xcd5b9596
	s_mov_b32 s12, 0xf1fdc7be
	s_mov_b32 s11, 0x40928d9a
	s_mov_b32 s13, 0xc0ae3d70
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[12:13], s[10:11]
	s_mov_b32 s10, 0xce591414
	s_mov_b32 s11, 0xc06554c1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x5a1fe7f5
	s_mov_b32 s11, 0x40315b1e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x6f616c69
	s_mov_b32 s11, 0xc001aa8e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xb3b4d6cc
	s_mov_b32 s11, 0xbf7f6803
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xac5bed2a
	s_mov_b32 s11, 0x3ff00019
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
                                        ; implicit-def: $vgpr6_vgpr7
.LBB16_10:
	s_and_not1_saveexec_b32 s9, s9
	s_cbranch_execz .LBB16_12
; %bb.11:
	s_mov_b32 s10, 0xba282b9b
	s_mov_b32 s12, 0x925f3a73
	s_mov_b32 s11, 0x4174aa2f
	s_mov_b32 s13, 0xc1821913
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[12:13], s[10:11]
	s_mov_b32 s10, 0xf9742896
	s_mov_b32 s11, 0xc155a2a3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x895772e8
	s_mov_b32 s11, 0x412b8ee3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xb036be4
	s_mov_b32 s11, 0xc0f7f2ce
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x1bcbb738
	s_mov_b32 s11, 0x40be62ab
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x965d2a06
	s_mov_b32 s11, 0xc07e0ed2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x705263e5
	s_mov_b32 s11, 0x403b0c16
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xa732ecc7
	s_mov_b32 s11, 0xc00334f9
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x412f9578
	s_mov_b32 s11, 0xbf765f60
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xbda43b5
	s_mov_b32 s11, 0x3ff0000e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
.LBB16_12:
	s_or_b32 exec_lo, exec_lo, s9
                                        ; implicit-def: $vgpr6_vgpr7
.LBB16_13:
	s_and_not1_saveexec_b32 s8, s8
	s_cbranch_execz .LBB16_15
; %bb.14:
	s_mov_b32 s10, 0x11ff3627
	s_mov_b32 s12, 0xbf9d81
	s_mov_b32 s11, 0x41384567
	s_mov_b32 s13, 0xc13d554f
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[12:13], s[10:11]
	s_mov_b32 s10, 0xacc5daaf
	s_mov_b32 s11, 0xc1226c90
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x1cdef815
	s_mov_b32 s11, 0x41010650
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x95601c04
	s_mov_b32 s11, 0xc0d57a4c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x7cbaede6
	s_mov_b32 s11, 0x40a3ca62
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0x91922fb
	s_mov_b32 s11, 0xc06c716e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xf6e8bc75
	s_mov_b32 s11, 0x403292f8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xc212bd5f
	s_mov_b32 s11, 0xc001b469
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xfb6d0462
	s_mov_b32 s11, 0xbf804977
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
	s_mov_b32 s10, 0xc9f52f8a
	s_mov_b32 s11, 0x3ff0001d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[10:11]
.LBB16_15:
	s_or_b32 exec_lo, exec_lo, s8
                                        ; implicit-def: $vgpr6_vgpr7
.LBB16_16:
	s_and_not1_saveexec_b32 s7, s7
	s_cbranch_execz .LBB16_18
; %bb.17:
	s_mov_b32 s8, 0xf98c6aa9
	s_mov_b32 s10, 0xaae00301
	s_mov_b32 s9, 0xc125781e
	s_mov_b32 s11, 0x411ff518
	s_delay_alu instid0(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[10:11], s[8:9]
	s_mov_b32 s8, 0xb21c7715
	s_mov_b32 s9, 0x411a9511
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x1455b21e
	s_mov_b32 s9, 0xc1041d8f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xd4025a4c
	s_mov_b32 s9, 0x40e4d4a3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xe7077996
	s_mov_b32 s9, 0xc0bf640f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x74f42181
	s_mov_b32 s9, 0x4091faf6
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xcd81d791
	s_mov_b32 s9, 0xc06080c5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x70098ef4
	s_mov_b32 s9, 0x402c0ae3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x67dc005a
	s_mov_b32 s9, 0xc0008ebd
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x29e72289
	s_mov_b32 s9, 0xbf85cf33
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xe75f27e2
	s_mov_b32 s9, 0x3ff00035
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
.LBB16_18:
	s_or_b32 exec_lo, exec_lo, s7
                                        ; implicit-def: $vgpr6_vgpr7
.LBB16_19:
	s_and_not1_saveexec_b32 s6, s6
	s_cbranch_execz .LBB16_21
; %bb.20:
	s_mov_b32 s8, 0x53b1bce6
	s_mov_b32 s10, 0x8e31c18e
	s_mov_b32 s9, 0xc0dc8661
	s_mov_b32 s11, 0x40cc9e5b
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[10:11], s[8:9]
	s_mov_b32 s8, 0x3b4fb25c
	s_mov_b32 s9, 0x40da386b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x78e7b5fb
	s_mov_b32 s9, 0xc0cd7bf3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xde0a7a75
	s_mov_b32 s9, 0x40b6b416
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x1cf44e90
	s_mov_b32 s9, 0xc099757c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xdedbaa8c
	s_mov_b32 s9, 0x4075b56e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x24b4d155
	s_mov_b32 s9, 0xc04da799
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x315d612b
	s_mov_b32 s9, 0x4022ba25
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x8fbd786d
	s_mov_b32 s9, 0xbffde580
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x4b9fc507
	s_mov_b32 s9, 0xbf904e01
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x8df1c89f
	s_mov_b32 s9, 0x3ff00078
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
.LBB16_21:
	s_or_b32 exec_lo, exec_lo, s6
                                        ; implicit-def: $vgpr6_vgpr7
.LBB16_22:
	s_and_not1_saveexec_b32 s5, s5
	s_cbranch_execz .LBB16_24
; %bb.23:
	s_mov_b32 s6, 0xeaa832db
	s_mov_b32 s8, 0x40bf066d
	s_mov_b32 s7, 0xc09870dd
	s_mov_b32 s9, 0x4080fdcb
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[6:7]
	s_mov_b32 s6, 0x9e0428c4
	s_mov_b32 s7, 0x40a035c3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x54a3ec14
	s_mov_b32 s7, 0xc09a4d3c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xee6efae8
	s_mov_b32 s7, 0x408d382a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x26565bc1
	s_mov_b32 s7, 0xc0779f9e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x58ce9aba
	s_mov_b32 s7, 0x405d00e0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x1821eb3
	s_mov_b32 s7, 0xc03c7d1e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xba7a3111
	s_mov_b32 s7, 0x4019d930
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x41dd2baa
	s_mov_b32 s7, 0xbffaf479
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xc823998b
	s_mov_b32 s7, 0xbf9787ec
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xe5fb73e3
	s_mov_b32 s7, 0x3ff000fa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
.LBB16_24:
	s_or_b32 exec_lo, exec_lo, s5
                                        ; implicit-def: $vgpr6_vgpr7
.LBB16_25:
	s_and_not1_saveexec_b32 s4, s4
	s_cbranch_execz .LBB16_27
; %bb.26:
	s_mov_b32 s6, 0xdab54a4e
	s_mov_b32 s8, 0xc98a5212
	s_mov_b32 s7, 0xc05907bc
	s_mov_b32 s9, 0x4038b3cf
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[6:7]
	s_mov_b32 s6, 0xf8216d7d
	s_mov_b32 s7, 0x4067659c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x777f664d
	s_mov_b32 s7, 0xc06ac222
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xe33151ac
	s_mov_b32 s7, 0x4064f2f8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xeb301c4c
	s_mov_b32 s7, 0xc057d7d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xc1c77e7
	s_mov_b32 s7, 0x40448e63
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xd0e327f6
	s_mov_b32 s7, 0xc02c63e7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x86aeb0df
	s_mov_b32 s7, 0x401225b2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xcc22b05d
	s_mov_b32 s7, 0xbff82a4a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x71680e57
	s_mov_b32 s7, 0xbfa0a882
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xacebb122
	s_mov_b32 s7, 0x3ff001f6
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
.LBB16_27:
	s_or_b32 exec_lo, exec_lo, s4
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], v[8:9]
.LBB16_28:
	s_and_not1_saveexec_b32 s1, s1
	s_cbranch_execz .LBB16_30
; %bb.29:
	v_add_f64 v[4:5], -v[2:3], 2.0
	s_mov_b32 s5, 0x3fe55555
	s_mov_b32 s4, 0x55555555
	s_mov_b32 s6, 0x6b47b09a
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s7, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[2:3], v[4:5]
	v_frexp_mant_f64_e32 v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[6:7]
	s_mov_b32 s4, 0x55555780
	v_cndmask_b32_e64 v8, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_rcp_f64_e32 v[10:11], v[8:9]
	v_add_f64 v[16:17], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[12:13], v[8:9], -v[18:19]
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[18:19], v[6:7]
	v_add_f64 v[16:17], v[14:15], -v[8:9]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_frexp_exp_i32_f64_e32 v18, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[16:17], v[6:7]
	v_mul_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], s[8:9], s[6:7]
	s_mov_b32 s6, 0xd7f4df2e
	s_mov_b32 s7, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x16291751
	s_mov_b32 s7, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x9b27acf1
	s_mov_b32 s7, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x998ef7b6
	s_mov_b32 s7, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x785a1166
	s_mov_b32 s7, 0x3ba1267a
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[4:5]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_subrev_co_ci_u32_e64 v16, null, 0, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x204
	v_cvt_f64_i32_e32 v[16:17], v16
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_mul_f64 v[18:19], v[16:17], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[4:5], -v[18:19]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[4:5], v[14:15]
	s_mov_b32 s4, 0x51dd484
	s_mov_b32 s5, 0xbc0a6581
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
	v_dual_cndmask_b32 v7, v7, v5 :: v_dual_cndmask_b32 v6, v6, v4
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[4:5]
	v_add_f64 v[6:7], 0xc0090000, -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v7, 0x7ff80000, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	v_fma_f64 v[4:5], v[6:7], s[6:7], s[4:5]
	s_mov_b32 s4, 0x6fc047a4
	s_mov_b32 s5, 0x3c32b295
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0xaed5cc07
	s_mov_b32 s5, 0x3c6ad835
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x12eae68f
	s_mov_b32 s5, 0xbca25e06
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x63f02a91
	s_mov_b32 s5, 0x3c6a0cab
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0xaf501adb
	s_mov_b32 s5, 0x3cfd9227
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x559a9b4e
	s_mov_b32 s5, 0xbd26c3ad
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x36036318
	s_mov_b32 s5, 0xbd36cafa
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x641e158f
	s_mov_b32 s5, 0x3d872879
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x55f7fff8
	s_mov_b32 s5, 0xbdac89d7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x71ddae3a
	s_mov_b32 s5, 0xbdcdc511
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x2744ae65
	s_mov_b32 s5, 0x3e120f51
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0xf4bcfcd8
	s_mov_b32 s5, 0xbe31a9e5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x926b83e8
	s_mov_b32 s5, 0xbe5f36ce
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x6c7cfa1e
	s_mov_b32 s5, 0x3e9c6b4f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x3e0c2026
	s_mov_b32 s5, 0xbeb6e8a5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x7bf4570b
	s_mov_b32 s5, 0xbeed1d1f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0xa20cc3e2
	s_mov_b32 s5, 0x3f2879c2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x94844d14
	s_mov_b32 s5, 0xbf484576
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x3114edad
	s_mov_b32 s5, 0xbf78b6c3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0xd9b13e14
	s_mov_b32 s5, 0x3fcebd80
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_mov_b32 s4, 0x7c99ae86
	s_mov_b32 s5, 0x3ffa755e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[6:7], v[4:5], s[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[4:5], -v[2:3], v[4:5], v[4:5]
.LBB16_30:
	s_or_b32 exec_lo, exec_lo, s1
.LBB16_31:
	s_and_not1_saveexec_b32 s1, s0
	s_cbranch_execz .LBB16_52
; %bb.32:
	v_add_f64 v[4:5], -v[2:3], 1.0
	s_mov_b32 s4, exec_lo
                                        ; implicit-def: $vgpr6_vgpr7
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ngt_f64_e64 0x3fd80000, |v[4:5]|
	s_xor_b32 s4, exec_lo, s4
	s_cbranch_execz .LBB16_49
; %bb.33:
	v_cmp_ngt_f64_e64 s0, 0x3fefffe0, |v[4:5]|
                                        ; implicit-def: $vgpr6_vgpr7
	s_and_saveexec_b32 s5, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s5
	s_cbranch_execz .LBB16_39
; %bb.34:
	v_add_f64 v[6:7], -|v[4:5]|, 1.0
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[8:9]
	s_mov_b32 s6, 0x55555780
	v_cndmask_b32_e64 v10, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v10
	v_add_f64 v[10:11], v[8:9], 1.0
	v_add_f64 v[16:17], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[12:13], v[10:11]
	v_add_f64 v[18:19], v[10:11], -1.0
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_mul_f64 v[14:15], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[10:11], v[14:15]
	v_fma_f64 v[10:11], v[14:15], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[20:21], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	v_add_f64 v[20:21], v[10:11], -v[20:21]
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[20:21], -v[8:9]
	v_frexp_exp_i32_f64_e32 v20, v[6:7]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[8:9], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	v_add_f64 v[10:11], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[10:11], v[10:11]
	v_fma_f64 v[16:17], v[12:13], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[18:19], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[12:13], v[16:17], s[6:7]
	v_ldexp_f64 v[16:17], v[10:11], 1
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0x3fe62e42
	v_mul_f64 v[12:13], v[18:19], v[12:13]
	v_subrev_co_ci_u32_e64 v18, null, 0, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_cmp_class_f64_e64 vcc_lo, v[6:7], 0x204
	v_cvt_f64_i32_e32 v[18:19], v18
	v_add_f64 v[14:15], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[8:9], v[8:9], 1
	v_mul_f64 v[20:21], v[18:19], s[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[14:15], -v[16:17]
	v_fma_f64 v[16:17], v[18:19], s[6:7], -v[20:21]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	v_fma_f64 v[12:13], v[18:19], s[6:7], v[16:17]
	s_mov_b32 s6, 0xffe00000
	s_mov_b32 s7, 0x3fefffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_nlt_f64_e64 s5, |v[4:5]|, s[6:7]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
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
	v_cndmask_b32_e32 v8, v8, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v9, -v9, -v7, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[6:7]
	v_cndmask_b32_e32 v9, 0xfff80000, v9, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[6:7]
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v9, 0x7ff00000, v9, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[8:9]
	v_cndmask_b32_e64 v6, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[8:9], v6
	v_rsq_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[10:11], v[6:7], v[8:9]
	v_mul_f64 v[8:9], v[8:9], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 0.5
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[10:11], v[10:11], v[6:7]
	v_fma_f64 v[10:11], v[12:13], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[10:11], v[10:11], v[6:7]
	v_fma_f64 v[8:9], v[12:13], v[8:9], v[10:11]
	v_cndmask_b32_e64 v10, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[6:7], 0x260
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v10
	v_dual_cndmask_b32 v7, v9, v7 :: v_dual_cndmask_b32 v6, v8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[8:9], null, v[6:7], v[6:7], 1.0
	v_div_scale_f64 v[14:15], vcc_lo, 1.0, v[6:7], 1.0
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[8:9], v[12:13], v[14:15]
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[12:13]
                                        ; implicit-def: $vgpr10_vgpr11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_div_fixup_f64 v[8:9], v[8:9], v[6:7], 1.0
	s_and_saveexec_b32 s6, s5
	s_xor_b32 s5, exec_lo, s6
	s_cbranch_execz .LBB16_36
; %bb.35:
	s_mov_b32 s6, 0xd25bee8d
	s_mov_b32 s8, 0x2cc8e58a
	s_mov_b32 s7, 0xc07dd260
	s_mov_b32 s9, 0x406e1f46
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], s[8:9], s[6:7]
	s_mov_b32 s6, 0xb6c206e6
	s_mov_b32 s7, 0x407af7da
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x5a0f5809
	s_mov_b32 s7, 0xc06d97c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0xbf45d30
	s_mov_b32 s7, 0x405632c2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x8179a727
	s_mov_b32 s7, 0xc038e490
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0xa73a2c3c
	s_mov_b32 s7, 0x40189538
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x69b3607d
	s_mov_b32 s7, 0xbffaad85
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0xec4b54cb
	s_mov_b32 s7, 0xbf980d1b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x6f90ea2c
	s_mov_b32 s7, 0x3ff00100
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
                                        ; implicit-def: $vgpr8_vgpr9
.LBB16_36:
	s_and_not1_saveexec_b32 s5, s5
	s_cbranch_execz .LBB16_38
; %bb.37:
	s_mov_b32 s6, 0x5b757c26
	s_mov_b32 s8, 0x31a51669
	s_mov_b32 s7, 0xc0866af4
	s_mov_b32 s9, 0x406c4bd8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], s[8:9], s[6:7]
	s_mov_b32 s6, 0x93ee1671
	s_mov_b32 s7, 0x409061b2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0xfd7248e9
	s_mov_b32 s7, 0xc08d4aa0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x88748d
	s_mov_b32 s7, 0x4081eebb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x6c165efe
	s_mov_b32 s7, 0xc06ff4cb
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x9a609255
	s_mov_b32 s7, 0x40559c37
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x677680c6
	s_mov_b32 s7, 0xc03762b2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x32cf7c5a
	s_mov_b32 s7, 0x40176261
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0xc231a949
	s_mov_b32 s7, 0xbffa298c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0x429b22ca
	s_mov_b32 s7, 0xbf99fa2d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
	s_mov_b32 s6, 0xc4b15d15
	s_mov_b32 s7, 0x3ff00131
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[6:7]
.LBB16_38:
	s_or_b32 exec_lo, exec_lo, s5
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[10:11]
.LBB16_39:
	s_and_not1_saveexec_b32 s5, s0
	s_cbranch_execz .LBB16_54
; %bb.40:
	v_fma_f64 v[6:7], -|v[4:5]|, |v[4:5]|, 1.0
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s9, 0x3fc38538
	s_mov_b32 s11, 0x3fc3ab76
	s_mov_b32 s0, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[8:9]
	s_mov_b32 s6, 0x55555780
	v_cndmask_b32_e64 v10, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v10
	v_add_f64 v[10:11], v[8:9], 1.0
	v_add_f64 v[16:17], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[12:13], v[10:11]
	v_add_f64 v[18:19], v[10:11], -1.0
	v_add_f64 v[8:9], v[8:9], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[12:13]
	v_mul_f64 v[14:15], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[10:11], v[14:15]
	v_fma_f64 v[10:11], v[14:15], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[14:15], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[20:21], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	v_add_f64 v[20:21], v[10:11], -v[20:21]
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[20:21], -v[8:9]
	v_frexp_exp_i32_f64_e32 v20, v[6:7]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[8:9], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	v_add_f64 v[10:11], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[10:11], v[10:11]
	v_fma_f64 v[16:17], v[12:13], s[10:11], s[8:9]
	s_mov_b32 s8, 0xd7f4df2e
	s_mov_b32 s9, 0x3fc7474d
	v_mul_f64 v[18:19], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x16291751
	s_mov_b32 s9, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x9b27acf1
	s_mov_b32 s9, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_mov_b32 s8, 0x998ef7b6
	s_mov_b32 s9, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[16:17], v[12:13], v[16:17], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[12:13], v[16:17], s[6:7]
	v_ldexp_f64 v[16:17], v[10:11], 1
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	s_mov_b32 s6, 0xfefa39ef
	s_mov_b32 s7, 0x3fe62e42
	v_mul_f64 v[12:13], v[18:19], v[12:13]
	v_subrev_co_ci_u32_e64 v18, null, 0, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_cmp_class_f64_e64 vcc_lo, v[6:7], 0x204
	v_cvt_f64_i32_e32 v[18:19], v18
	v_add_f64 v[14:15], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[8:9], v[8:9], 1
	v_mul_f64 v[20:21], v[18:19], s[6:7]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[14:15], -v[16:17]
	v_fma_f64 v[16:17], v[18:19], s[6:7], -v[20:21]
	s_mov_b32 s6, 0x3b39803f
	s_mov_b32 s7, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[12:13], -v[10:11]
	v_fma_f64 v[12:13], v[18:19], s[6:7], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], v[8:9]
	v_add_f64 v[20:21], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[10:11], v[16:17]
	v_add_f64 v[14:15], v[16:17], -v[14:15]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[18:19], -v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[18:19], -v[22:23]
	v_add_f64 v[14:15], v[16:17], -v[22:23]
	v_add_f64 v[16:17], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[24:25]
	v_add_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[16:17], -v[12:13]
	v_add_f64 v[10:11], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[16:17], -v[14:15]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[20:21], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	v_add_f64 v[14:15], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[8:9], v[20:21], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v8, v8, v6 :: v_dual_cndmask_b32 v9, v9, v7
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[6:7]
	v_cndmask_b32_e32 v9, 0x7ff80000, v9, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[6:7]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[6:7]
                                        ; implicit-def: $vgpr6_vgpr7
	v_cndmask_b32_e32 v9, 0xfff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_nlt_f64_e32 0xc0190000, v[8:9]
	s_xor_b32 s6, exec_lo, s0
	s_cbranch_execz .LBB16_46
; %bb.41:
	v_cmp_lt_f64_e32 vcc_lo, 0x90000000, v[8:9]
	v_cmp_nlt_f64_e64 s0, 0xc0300000, v[8:9]
	v_cndmask_b32_e64 v6, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], -v[8:9], v6
	v_rsq_f64_e32 v[10:11], v[6:7]
	s_waitcnt_depctr 0xfff
	v_mul_f64 v[12:13], v[6:7], v[10:11]
	v_mul_f64 v[10:11], v[10:11], 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[10:11], v[12:13], 0.5
	v_fma_f64 v[12:13], v[12:13], v[14:15], v[12:13]
	v_fma_f64 v[10:11], v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[12:13], v[12:13], v[6:7]
	v_fma_f64 v[12:13], v[14:15], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], -v[12:13], v[12:13], v[6:7]
	v_fma_f64 v[10:11], v[14:15], v[10:11], v[12:13]
	v_cndmask_b32_e64 v12, 0, 0xffffff80, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[6:7], 0x260
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[10:11], v[10:11], v12
	v_dual_cndmask_b32 v9, v11, v7 :: v_dual_cndmask_b32 v8, v10, v6
                                        ; implicit-def: $vgpr6_vgpr7
	s_and_saveexec_b32 s7, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s0, exec_lo, s7
	s_cbranch_execz .LBB16_43
; %bb.42:
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], 0xc0140000, v[8:9]
	s_mov_b32 s8, 0xc0e38727
	s_mov_b32 s10, 0xa7785389
	s_mov_b32 s9, 0xbdf18fee
	s_mov_b32 s11, 0xbdbdcec3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[10:11], s[8:9]
	s_mov_b32 s8, 0x2dda45e3
	s_mov_b32 s9, 0x3e19e6bf
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xb24e2f5f
	s_mov_b32 s9, 0xbe30468f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xa8fba182
	s_mov_b32 s9, 0x3e405ac6
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x495fb9c0
	s_mov_b32 s9, 0xbe50102e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xe1334af8
	s_mov_b32 s9, 0x3e5f4c20
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xfdf9c3e
	s_mov_b32 s9, 0xbe722d22
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xb824cb54
	s_mov_b32 s9, 0x3e8ebc8b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xea372cc
	s_mov_b32 s9, 0xbeb0a8d4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x9d093d2b
	s_mov_b32 s9, 0x3ed2fbd2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x7e1e0fac
	s_mov_b32 s9, 0xbef4a349
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xeb00938f
	s_mov_b32 s9, 0x3f13ebf4
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xa8fc5d53
	s_mov_b32 s9, 0xbf2c2f36
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xdf04047c
	s_mov_b32 s9, 0xbf222ea5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xd1fba0dc
	s_mov_b32 s9, 0x3ff02a30
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xdd1ad7fb
	s_mov_b32 s9, 0x4013664d
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[8:9]
                                        ; implicit-def: $vgpr8_vgpr9
.LBB16_43:
	s_and_not1_saveexec_b32 s0, s0
	s_cbranch_execz .LBB16_45
; %bb.44:
	v_add_f64 v[6:7], 0xc00a0000, v[8:9]
	s_mov_b32 s8, 0x52878635
	s_mov_b32 s10, 0x87dbd932
	s_mov_b32 s9, 0x3e785cbe
	s_mov_b32 s11, 0x3e23040f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[10:11], s[8:9]
	s_mov_b32 s8, 0x53dd3955
	s_mov_b32 s9, 0xbe927774
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xcd554c6c
	s_mov_b32 s9, 0x3e5395ab
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x8a3790ad
	s_mov_b32 s9, 0x3eb93638
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x812b5083
	s_mov_b32 s9, 0xbed0d5db
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xd5d652f6
	s_mov_b32 s9, 0x3ec8860c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xcacdfb23
	s_mov_b32 s9, 0x3eea29a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xf80281f2
	s_mov_b32 s9, 0xbf08cef1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xd0b9188a
	s_mov_b32 s9, 0x3f11e684
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x54c8a222
	s_mov_b32 s9, 0x3ef932cd
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x89ef8aa3
	s_mov_b32 s9, 0xbf37448a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x5ad40c25
	s_mov_b32 s9, 0x3f4f3cc5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x132f38b1
	s_mov_b32 s9, 0xbf5ba924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xca533cf8
	s_mov_b32 s9, 0x3f6468ee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xbb891bbd
	s_mov_b32 s9, 0xbf6ebada
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0xe5b76afc
	s_mov_b32 s9, 0x3f75ffcf
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x6d641d39
	s_mov_b32 s9, 0x3ff0158a
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[8:9]
	s_mov_b32 s8, 0x380d5a48
	s_mov_b32 s9, 0x4008abcc
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[8:9]
.LBB16_45:
	s_or_b32 exec_lo, exec_lo, s0
                                        ; implicit-def: $vgpr8_vgpr9
.LBB16_46:
	s_and_not1_saveexec_b32 s0, s6
	s_cbranch_execz .LBB16_48
; %bb.47:
	v_add_f64 v[6:7], 0xc0090000, -v[8:9]
	s_mov_b32 s6, 0x3324d327
	s_mov_b32 s8, 0xe746e627
	s_mov_b32 s7, 0xbc08ddf9
	s_mov_b32 s9, 0xbbb135d2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[6:7]
	s_mov_b32 s6, 0xef0b7c9f
	s_mov_b32 s7, 0x3c37b83e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xcd589b91
	s_mov_b32 s7, 0x3c69ba72
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x90a6b96
	s_mov_b32 s7, 0xbca33689
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x898132e0
	s_mov_b32 s7, 0x3c782e11
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xfd9e26ba
	s_mov_b32 s7, 0x3cfde4ac
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xed66c487
	s_mov_b32 s7, 0xbd26d33e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x7040d8e2
	s_mov_b32 s7, 0xbd36f216
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xc2d77e20
	s_mov_b32 s7, 0x3d872a22
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xc4e5c0af
	s_mov_b32 s7, 0xbdac8859
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xd118a561
	s_mov_b32 s7, 0xbdcdc583
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xccf46b3c
	s_mov_b32 s7, 0x3e120f47
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x8dc84d60
	s_mov_b32 s7, 0xbe31a9e3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x6d3d46a9
	s_mov_b32 s7, 0xbe5f36cd
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x5d03b787
	s_mov_b32 s7, 0x3e9c6b4f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x434ae8a2
	s_mov_b32 s7, 0xbeb6e8a5
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x7b8736f6
	s_mov_b32 s7, 0xbeed1d1f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xa212f024
	s_mov_b32 s7, 0x3f2879c2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x9484fca8
	s_mov_b32 s7, 0xbf484576
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x3114f909
	s_mov_b32 s7, 0xbf78b6c3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0xd9b13e28
	s_mov_b32 s7, 0x3fcebd80
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x7c99ae86
	s_mov_b32 s7, 0x3ffa755e
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[6:7]
.LBB16_48:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[6:7], |v[4:5]|, v[6:7]
	s_or_b32 exec_lo, exec_lo, s5
.LBB16_49:
	s_and_not1_saveexec_b32 s0, s4
	s_cbranch_execz .LBB16_51
.LBB16_50:
	v_mul_f64 v[6:7], v[4:5], v[4:5]
	s_mov_b32 s4, 0x47aef0d6
	s_mov_b32 s6, 0x6cd8002b
	s_mov_b32 s5, 0xbfebb7dd
	s_mov_b32 s7, 0x3fdc5ec0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], s[6:7], s[4:5]
	s_mov_b32 s4, 0x92eccdb6
	s_mov_b32 s5, 0x3fed1899
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x80cde957
	s_mov_b32 s5, 0xbfe10ec1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x379dd66f
	s_mov_b32 s5, 0x3fd05cce
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x7e3dae74
	s_mov_b32 s5, 0xbfa6b906
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x487c11a3
	s_mov_b32 s5, 0x3fa5f7f0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x22b2350c
	s_mov_b32 s5, 0x3f9e0fbf
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x322b7f90
	s_mov_b32 s5, 0x3fa2ce26
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0xee81dd31
	s_mov_b32 s5, 0x3fa5ebee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0xb897f0d4
	s_mov_b32 s5, 0x3faa7cac
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0xd62cba32
	s_mov_b32 s5, 0x3fb0a130
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0xc8653359
	s_mov_b32 s5, 0x3fb62847
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0xc0a5e083
	s_mov_b32 s5, 0x3fc053c2
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0xb2feec72
	s_mov_b32 s5, 0x3fcdb29f
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[4:5]
	s_mov_b32 s4, 0x91b4ef6a
	s_mov_b32 s5, 0x3fec5bf8
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[6:7], v[8:9], s[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[6:7], |v[4:5]|, v[6:7]
.LBB16_51:
	s_or_b32 exec_lo, exec_lo, s0
	v_cmp_ngt_f64_e64 vcc_lo, |v[4:5]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0x7ff80000, v7, vcc_lo
	v_cmp_neq_f64_e64 vcc_lo, |v[4:5]|, 1.0
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	v_cmp_nge_f64_e64 vcc_lo, |v[4:5]|, 1.0
	s_delay_alu instid0(VALU_DEP_2)
	v_bfi_b32 v5, 0x7fffffff, v7, v5
	v_cndmask_b32_e32 v4, 0, v6, vcc_lo
.LBB16_52:
	s_or_b32 exec_lo, exec_lo, s1
	s_mov_b32 s4, 0x667f3bcd
	v_cmp_gt_f64_e32 vcc_lo, 0, v[2:3]
	s_mov_b32 s5, 0xbff6a09e
	v_cmp_lt_f64_e64 s0, 2.0, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[4:5], v[4:5], s[4:5]
	s_or_b32 s0, vcc_lo, s0
	v_cmp_neq_f64_e32 vcc_lo, 0, v[2:3]
	v_cndmask_b32_e64 v4, v4, 0, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v5, v5, 0x7ff80000, s0
	v_cmp_neq_f64_e64 s0, 2.0, v[2:3]
	v_cndmask_b32_e32 v5, 0xfff00000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_3)
	v_cndmask_b32_e64 v3, 0x7ff00000, v5, s0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB16_53:
	s_endpgm
.LBB16_54:
	s_or_b32 exec_lo, exec_lo, s5
	s_and_not1_saveexec_b32 s0, s4
	s_cbranch_execnz .LBB16_50
	s_branch .LBB16_51
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_ndtri_kernel
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
		.amdhsa_next_free_vgpr 26
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
		.amdhsa_inst_pref_size 63
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
	.size	specialx_ndtri_kernel, .Lfunc_end16-specialx_ndtri_kernel
                                        ; -- End function
	.set specialx_ndtri_kernel.num_vgpr, 26
	.set specialx_ndtri_kernel.num_agpr, 0
	.set specialx_ndtri_kernel.numbered_sgpr, 14
	.set specialx_ndtri_kernel.num_named_barrier, 0
	.set specialx_ndtri_kernel.private_seg_size, 0
	.set specialx_ndtri_kernel.uses_vcc, 1
	.set specialx_ndtri_kernel.uses_flat_scratch, 0
	.set specialx_ndtri_kernel.has_dyn_sized_stack, 0
	.set specialx_ndtri_kernel.has_recursion, 0
	.set specialx_ndtri_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 10708
; TotalNumSgprs: 16
; NumVgprs: 26
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 16
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
	.protected	specialx_sinpi_kernel   ; -- Begin function specialx_sinpi_kernel
	.globl	specialx_sinpi_kernel
	.p2align	8
	.type	specialx_sinpi_kernel,@function
specialx_sinpi_kernel:                  ; @specialx_sinpi_kernel
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
	s_cbranch_execz .LBB17_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, 0xf99eb0bb
	s_mov_b32 s6, 0xca1d4f33
	s_mov_b32 s8, 0x2e21c33
	s_mov_b32 s5, 0x3f3e357e
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s7, 0x3f5f9c89
	s_mov_b32 s9, 0xbf1b1673
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x6fdffd2b
	s_mov_b32 s1, 0xbf7e2fe7
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], |v[2:3]|, 0.5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fract_f64_e32 v[6:7], v[4:5]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[4:5]|
	v_and_b32_e32 v4, 0x7fffffff, v3
	v_add_f64 v[6:7], v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v6, 0, v6 :: v_dual_cndmask_b32 v5, 0, v7
	v_cmp_gt_f64_e64 vcc_lo, |v[2:3]|, 1.0
	v_dual_cndmask_b32 v5, v4, v5 :: v_dual_cndmask_b32 v4, v2, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[4:5], v[4:5]
	v_rndne_f64_e32 v[6:7], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[4:5], v[6:7], -0.5, v[4:5]
	v_mul_f64 v[8:9], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], s[4:5], s[0:1]
	v_fma_f64 v[12:13], v[8:9], s[8:9], s[6:7]
	s_mov_b32 s0, 0xd5f14825
	s_mov_b32 s4, 0x7294bff9
	s_mov_b32 s1, 0x3fb50782
	s_mov_b32 s5, 0xbf9a6d1e
	v_mul_f64 v[14:15], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s0, 0xcdfe9424
	s_mov_b32 s4, 0x67b90b37
	s_mov_b32 s1, 0xbfe32d2c
	s_mov_b32 s5, 0x3fce1f50
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s0, 0x67754fff
	s_mov_b32 s4, 0x7e3c325b
	s_mov_b32 s1, 0x400466bc
	s_mov_b32 s5, 0xbff55d3c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s0, 0xe625be09
	s_mov_b32 s4, 0x81b5a67
	s_mov_b32 s1, 0xc014abbc
	s_mov_b32 s5, 0x40103c1f
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[4:5]
	s_mov_b32 s0, 0xc9be45de
	s_mov_b32 s1, 0xc013bd3c
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	v_fma_f64 v[12:13], v[8:9], v[12:13], s[0:1]
	v_cvt_i32_f64_e32 v14, v[6:7]
	s_mov_b32 s0, 0x54442d18
	s_mov_b32 s1, 0x400921fb
	s_delay_alu instid0(VALU_DEP_3) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[4:5], v[4:5], s[0:1], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_fma_f64 v[6:7], v[8:9], v[12:13], 1.0
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	v_and_b32_e32 v8, 1, v14
	v_lshlrev_b32_e32 v2, 30, v14
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_eq_u32_e32 vcc_lo, 0, v8
	v_xor_b32_e32 v2, v2, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v3, v7, v5 :: v_dual_and_b32 v2, 0x80000000, v2
	v_cndmask_b32_e32 v4, v6, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v3, v3, v2
	v_cndmask_b32_e64 v2, 0, v4, s0
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v3, 0x7ff80000, v3, s0
	global_store_b64 v[0:1], v[2:3], off
.LBB17_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_sinpi_kernel
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
.Lfunc_end17:
	.size	specialx_sinpi_kernel, .Lfunc_end17-specialx_sinpi_kernel
                                        ; -- End function
	.set specialx_sinpi_kernel.num_vgpr, 16
	.set specialx_sinpi_kernel.num_agpr, 0
	.set specialx_sinpi_kernel.numbered_sgpr, 10
	.set specialx_sinpi_kernel.num_named_barrier, 0
	.set specialx_sinpi_kernel.private_seg_size, 0
	.set specialx_sinpi_kernel.uses_vcc, 1
	.set specialx_sinpi_kernel.uses_flat_scratch, 0
	.set specialx_sinpi_kernel.has_dyn_sized_stack, 0
	.set specialx_sinpi_kernel.has_recursion, 0
	.set specialx_sinpi_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 716
; TotalNumSgprs: 12
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 12
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
	.protected	specialx_xlogy_kernel   ; -- Begin function specialx_xlogy_kernel
	.globl	specialx_xlogy_kernel
	.p2align	8
	.type	specialx_xlogy_kernel,@function
specialx_xlogy_kernel:                  ; @specialx_xlogy_kernel
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
	s_cbranch_execz .LBB18_4
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
	v_add_co_u32 v2, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_neq_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB18_3
; %bb.2:
	v_add_co_u32 v4, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	s_mov_b32 s5, 0x3fe55555
	s_mov_b32 s4, 0x55555555
	s_mov_b32 s6, 0x6b47b09a
	global_load_b64 v[4:5], v[4:5], off
	s_mov_b32 s8, 0xbf559e2b
	s_mov_b32 s7, 0x3fc38538
	s_mov_b32 s9, 0x3fc3ab76
	s_waitcnt vmcnt(0)
	v_frexp_mant_f64_e32 v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[4:5], v[6:7]
	s_mov_b32 s4, 0x55555780
	v_cndmask_b32_e64 v8, 0, 1, vcc_lo
	v_ldexp_f64 v[6:7], v[6:7], v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[6:7], 1.0
	v_add_f64 v[14:15], v[6:7], -1.0
	v_rcp_f64_e32 v[10:11], v[8:9]
	v_add_f64 v[16:17], v[8:9], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[12:13], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[12:13], v[8:9], -v[18:19]
	v_fma_f64 v[6:7], v[12:13], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[18:19], v[6:7]
	v_add_f64 v[16:17], v[14:15], -v[8:9]
	v_add_f64 v[18:19], v[8:9], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[18:19], -v[6:7]
	v_frexp_exp_i32_f64_e32 v18, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[14:15], -v[8:9]
	v_add_f64 v[6:7], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[16:17], v[6:7]
	v_mul_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[6:7]
	v_mul_f64 v[10:11], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[10:11], s[8:9], s[6:7]
	s_mov_b32 s6, 0xd7f4df2e
	s_mov_b32 s7, 0x3fc7474d
	v_mul_f64 v[16:17], v[8:9], v[10:11]
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x16291751
	s_mov_b32 s7, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x9b27acf1
	s_mov_b32 s7, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_mov_b32 s6, 0x998ef7b6
	s_mov_b32 s7, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[10:11], v[14:15], s[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[10:11], v[10:11], v[14:15], s[4:5]
	v_ldexp_f64 v[14:15], v[8:9], 1
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0x3fe62e42
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_subrev_co_ci_u32_e64 v16, null, 0, v18, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_cmp_class_f64_e64 vcc_lo, v[4:5], 0x204
	v_cvt_f64_i32_e32 v[16:17], v16
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_ldexp_f64 v[6:7], v[6:7], 1
	v_mul_f64 v[18:19], v[16:17], s[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[4:5], -v[18:19]
	s_mov_b32 s4, 0x3b39803f
	s_mov_b32 s5, 0x3c7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
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
	v_dual_cndmask_b32 v6, v6, v4 :: v_dual_cndmask_b32 v7, v7, v5
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v7, 0x7ff80000, v7, vcc_lo
	v_cmp_nge_f64_e32 vcc_lo, 0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[4:5]
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[4:5], v[2:3], v[6:7]
.LBB18_3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB18_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel specialx_xlogy_kernel
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
.Lfunc_end18:
	.size	specialx_xlogy_kernel, .Lfunc_end18-specialx_xlogy_kernel
                                        ; -- End function
	.set specialx_xlogy_kernel.num_vgpr, 24
	.set specialx_xlogy_kernel.num_agpr, 0
	.set specialx_xlogy_kernel.numbered_sgpr, 10
	.set specialx_xlogy_kernel.num_named_barrier, 0
	.set specialx_xlogy_kernel.private_seg_size, 0
	.set specialx_xlogy_kernel.uses_vcc, 1
	.set specialx_xlogy_kernel.uses_flat_scratch, 0
	.set specialx_xlogy_kernel.has_dyn_sized_stack, 0
	.set specialx_xlogy_kernel.has_recursion, 0
	.set specialx_xlogy_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1136
; TotalNumSgprs: 12
; NumVgprs: 24
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 12
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_daad206124ef2e23,@object ; @__hip_cuid_daad206124ef2e23
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_daad206124ef2e23
__hip_cuid_daad206124ef2e23:
	.byte	0                               ; 0x0
	.size	__hip_cuid_daad206124ef2e23, 1

	.type	__ocmltbl_M64_J0,@object        ; @__ocmltbl_M64_J0
	.section	.rodata,"a",@progbits
	.p2align	3, 0x0
__ocmltbl_M64_J0:
	.quad	0x3ff0000000000000              ; double 1
	.quad	0xbca4907401f4e73a              ; double -1.4269328868608038E-16
	.quad	0xbfcfffffffffff20              ; double -0.24999999999999378
	.quad	0xbd3e2aeb92c11928              ; double -1.0717704790389966E-13
	.quad	0x3f90000000044077              ; double 0.015625000000966751
	.quad	0xbd971846e585e8c0              ; double -5.2511567891715888E-12
	.quad	0xbf3c71c707fc64a5              ; double -4.3402775917084974E-4
	.quad	0xbdc8d2e5ca2e7507              ; double -4.5154263377571992E-11
	.quad	0x3edc71dc58841f48              ; double 6.7817612790023292E-6
	.quad	0xbdd9fb942a93af21              ; double -9.4524619593582297E-11
	.quad	0xbe722ea554940614              ; double -6.7734011417068306E-8
	.quad	0xbdcc309762886108              ; double -5.1276965587306845E-11
	.quad	0x3e00ece316eb4bd4              ; double 4.9259222901902217E-10
	.quad	0xbd99479286f572fd              ; double -5.7479109221671055E-12
	.quad	0xbd7cbb10470436c0              ; double -1.6331521876245402E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0xbfe09cdb36551280              ; double -0.51914749728946674
	.quad	0x3fbba1deea029494              ; double 0.1079387017549201
	.quad	0x3facfae864368d6b              ; double 0.056601774437946194
	.quad	0xbf81bb1cbe1a406d              ; double -0.0086576695933049067
	.quad	0xbf61f992590d0897              ; double -0.0021942003590150294
	.quad	0x3f315382ba06bf05              ; double 2.6437703675251417E-4
	.quad	0x3f06ed3b9eec933d              ; double 4.3729192716923726E-5
	.quad	0xbed232c77d035520              ; double -4.3388262868833416E-6
	.quad	0xbea1cce2df6157ca              ; double -5.3049137594784278E-7
	.quad	0x3e67ff98d2088a17              ; double 4.4700551042149104E-8
	.quad	0x3e3294ed7209404c              ; double 4.3264003773432389E-9
	.quad	0xbdf5c278f4188e72              ; double -3.1664470012675613E-10
	.quad	0xbdbb9f73114ac14b              ; double -2.5122835305798085E-11
	.quad	0x3d7c870190fb4ff7              ; double 1.6215931083463107E-12
	.quad	0xbfd9c6cf582cbf7f              ; double -0.40275939570255298
	.quad	0xbc2340630be882e1              ; double -5.2181326018778111E-19
	.quad	0x3fc9c6cf582cbf7e              ; double 0.20137969785127646
	.quad	0xbf91f06d14e11df9              ; double -0.017518715285659046
	.quad	0xbf8b589d1da136e9              ; double -0.013352611033180266
	.quad	0x3f50f9103cf5a452              ; double 0.0010359438491269924
	.quad	0x3f3864456219e47e              ; double 3.7218755651442076E-4
	.quad	0xbefa2a033caecdfc              ; double -2.4952041524263141E-5
	.quad	0xbed83a06df50149a              ; double -5.7760876091040011E-6
	.quad	0x3e96a4fd6f3e37bf              ; double 3.3742922699801003E-7
	.quad	0x3e6ec03769300bbd              ; double 5.7277913211048927E-8
	.quad	0xbe295d7532c9ae75              ; double -2.9528827354673038E-9
	.quad	0xbdfb1aa7f95eb2b7              ; double -3.9441693779923091E-10
	.quad	0x3db3d0e8d4f46c36              ; double 1.8022594969949102E-11
	.quad	0x3d809643d778859e              ; double 1.8857204715831148E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0x3fd5c6e60a097823              ; double 0.34026480655836816
	.quad	0xbf9f8f72e7a848e0              ; double -0.030820651425593648
	.quad	0xbfab2150cb41e89b              ; double -0.052988552867604365
	.quad	0x3f72f7ffe90256ab              ; double 0.0046310421459076307
	.quad	0x3f627e31fe9a6359              ; double 0.0022574402290271131
	.quad	0xbf26f641f41949df              ; double -1.7518572899406692E-4
	.quad	0xbf0863f48139d08a              ; double -4.6521090692503811E-5
	.quad	0x3ecad77d74a4eb89              ; double 3.1997869075739443E-6
	.quad	0x3ea32e6d3322f526              ; double 5.7164888846826253E-7
	.quad	0xbe62da3821a66401              ; double -3.5115366797673735E-8
	.quad	0xbe341d0e8e78f041              ; double -4.6830399346222682E-9
	.quad	0x3df1d089c7b66e54              ; double 2.592365833392453E-10
	.quad	0x3dbdd03e1bc02c21              ; double 2.7115172723816524E-11
	.quad	0xbd786cdf5a3f3b8f              ; double -1.3884165974276053E-12
	.quad	0x3fd33518b3874e8a              ; double 0.30011575252613254
	.quad	0x3c42f912abc5c301              ; double 2.0570504009629279E-18
	.quad	0xbfc33518b3874e8a              ; double -0.15005787626306627
	.quad	0x3f7d34125d59d874              ; double 0.0071297376031137401
	.quad	0x3f880c83bdeee45b              ; double 0.011742619737434781
	.quad	0xbf4483c20f1c66bb              ; double -6.2605834520753441E-4
	.quad	0xbf36ffa5fc8ae7ce              ; double -3.5093119053508377E-4
	.quad	0x3ef2ccf7b1d72132              ; double 1.7929701348313658E-5
	.quad	0x3ed796a74fb77cda              ; double 5.6239343808321796E-6
	.quad	0xbe91e8509b04b9cd              ; double -2.6684224520542096E-7
	.quad	0xbe6e6a46b48901dc              ; double -5.6652615547124155E-8
	.quad	0x3e254bf2843030ab              ; double 2.4792586052774416E-9
	.quad	0x3dfb064cf4b52f16              ; double 3.9325985931918325E-10
	.quad	0xbdb14a00318682f1              ; double -1.5724313427150256E-11
	.quad	0xbd81036310530753              ; double -1.9341803571391107E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0xbfd15f7977a772d4              ; double -0.27145229992838193
	.quad	0x3f900f7fcf183e0d              ; double 0.015684124960953883
	.quad	0x3fa68b984ec64925              ; double 0.044033774963411688
	.quad	0xbf648e63600d8406              ; double -0.0025093022272106883
	.quad	0xbf60e0d60385856b              ; double -0.002060335155122208
	.quad	0x3f1d796052772f54              ; double 1.1243486789352708E-4
	.quad	0x3f07800bc50775c0              ; double 4.4823035412848693E-5
	.quad	0xbec3324842d019a1              ; double -2.28839100780143E-6
	.quad	0xbea30e8c77c13527              ; double -5.6793781722802324E-7
	.quad	0x3e5ceda4325e2826              ; double 2.6941566442661997E-8
	.quad	0x3e3457dc477e660a              ; double 4.7365215013159891E-9
	.quad	0xbdecad9a4a509c92              ; double -2.0866089859212072E-10
	.quad	0xbdbe864d9617e1bc              ; double -2.7761981412381772E-11
	.quad	0x3d741351fe093d3d              ; double 1.1411583417182673E-12
	.quad	0xbfcff654544ebcd1              ; double -0.24970487705784319
	.quad	0xbc44353ed972a55a              ; double -2.1909546936929061E-18
	.quad	0x3fbff654544ebcd0              ; double 0.12485243852892158
	.quad	0xbf70c17ff72afa55              ; double -0.0040907858517003808
	.quad	0xbf84b0c5d5da66c1              ; double -0.010102792347697844
	.quad	0x3f394154be70516b              ; double 3.8536375944999449E-4
	.quad	0x3f34e12c3066b4a0              ; double 3.1859711489341282E-4
	.quad	0xbee9f32fc1c76819              ; double -1.2373899203877618E-5
	.quad	0xbed63c5473ef99e9              ; double -5.3013953324799308E-6
	.quad	0x3e8adbaf4eca4d0d              ; double 2.0010876457654012E-7
	.quad	0x3e6d601b6216d4a4              ; double 5.4715979534900831E-8
	.quad	0xbe20ee907fee672b              ; double -1.9711317018282612E-9
	.quad	0xbdfa83c5bbb08015              ; double -3.8584018939012558E-10
	.quad	0x3daca66d05214d85              ; double 1.3028557538648307E-11
	.quad	0x3d810d9ef3f98be2              ; double 1.9387251405422158E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0x3fcdc13e66ac2e77              ; double 0.23245983136472478
	.quad	0xbf842ff0cdc58463              ; double -0.0098570645138257913
	.quad	0xbfa38d1dd8992df2              ; double -0.038186009111622968
	.quad	0x3f5a55e9b346eda9              ; double 0.0016073972920896773
	.quad	0x3f5e2e16f97cd3db              ; double 0.0018420433388659427
	.quad	0xbf13dfc3782acfe1              ; double -7.5813584809846925E-5
	.quad	0xbf05ce7f4928aeae              ; double -4.1592845395702557E-5
	.quad	0x3ebbb178da527278              ; double 1.6506463478622605E-6
	.quad	0x3ea2346d3235a301              ; double 5.4254505636478444E-7
	.quad	0xbe5612f29b5796e6              ; double -2.0558027910130632E-8
	.quad	0xbe33d74f0f21f0ab              ; double -4.6196044646920419E-9
	.quad	0x3de6db6fedbdd2d3              ; double 1.6630784845680671E-10
	.quad	0x3dbe380534e5b583              ; double 2.7483865275708142E-11
	.quad	0xbd70827a2a754fb8              ; double -9.384664623993555E-13
	.size	__ocmltbl_M64_J0, 960

	.type	__ocmltbl_M64_J1,@object        ; @__ocmltbl_M64_J1
	.p2align	3, 0x0
__ocmltbl_M64_J1:
	.quad	0x0000000000000000              ; double 0
	.quad	0x3fe0000000000000              ; double 0.5
	.quad	0xbc37ed0e3b828b08              ; double -1.2970309732986904E-18
	.quad	0xbfaffffffffffff5              ; double -0.062499999999999924
	.quad	0xbce0293164fb7eb1              ; double -1.7942214325033242E-15
	.quad	0x3f65555555561a43              ; double 0.0026041666666885301
	.quad	0xbd4677d2ff1a71b8              ; double -1.5964519165155313E-13
	.quad	0xbf0c71c715cc6962              ; double -5.4253471466663884E-5
	.quad	0xbd855cabd8ebf8fc              ; double -2.4285779070936099E-12
	.quad	0x3ea6c1780f921557              ; double 6.7817384698301119E-7
	.quad	0xbda2ed5069aac6fe              ; double -8.6070068625189802E-12
	.quad	0xbe383b4470480845              ; double -5.6418387778447457E-9
	.quad	0xbda01864213fb0f9              ; double -7.3192849689297936E-12
	.quad	0x3dc4844c536d3a2c              ; double 3.7319822951004814E-11
	.quad	0xbd735a9c5cc3ab06              ; double -1.1001445955275011E-12
	.quad	0x3fe29ea3d19f035f              ; double 0.58186522428159637
	.quad	0xbc59e62cc35ec1be              ; double -5.6159765491837456E-18
	.quad	0xbfca41115c5df242              ; double -0.20511071214777316
	.quad	0x3f78d1448e6fee77              ; double 0.0060589483246037334
	.quad	0x3f8c441a2f9ddf5d              ; double 0.013801769807954828
	.quad	0xbf386671c18bfe53              ; double -3.7231709715965683E-4
	.quad	0xbf39e2504dd90dcf              ; double -3.9495907353545313E-4
	.quad	0x3ee34ccbcab07ec9              ; double 9.2029498173768214E-6
	.quad	0x3eda4973743766ee              ; double 6.2672896236849501E-6
	.quad	0xbe810453841038e8              ; double -1.2678578012497979E-7
	.quad	0xbe70fade42a46cb3              ; double -6.3255257619028975E-8
	.quad	0x3e135494d664aee3              ; double 1.1251771403253867E-9
	.quad	0x3dfe5b866c453f65              ; double 4.4176005585408685E-10
	.quad	0xbd9eb2970acb8068              ; double -6.9798300547918849E-12
	.quad	0xbd82faf08aafb901              ; double -2.1578026548615528E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0xbfd9c6cf582cbf7f              ; double -0.40275939570255298
	.quad	0x3faae8a39f51ad04              ; double 0.052556145856977238
	.quad	0x3fab589d1da13aa3              ; double 0.053410444132727684
	.quad	0xbf7537544c331cd2              ; double -0.0051797192456383857
	.quad	0xbf624b3409976ac2              ; double -0.0022331253392001435
	.quad	0x3f26e4c2d52dae44              ; double 1.7466429070665996E-4
	.quad	0x3f083a06e62d9f8f              ; double 4.6208701653337803E-5
	.quad	0xbec9799d369b7229              ; double -3.0368632238776932E-6
	.quad	0xbea338283463a4ee              ; double -5.7278166634453132E-7
	.quad	0x3e6170516f7c85ee              ; double 3.2482189325657563E-8
	.quad	0x3e34584933fddd86              ; double 4.7369084764612078E-9
	.quad	0xbdf026119d7f08af              ; double -2.3499460493506459E-10
	.quad	0xbdbf9000da3a4471              ; double -2.8705938354850319E-11
	.quad	0x3d5f7332c56d63d2              ; double 4.4693128781201312E-13
	.quad	0xbfd626ee83500bf2              ; double -0.3461262018537915
	.quad	0xbc40432466d6e3f3              ; double -1.7631593012980776E-18
	.quad	0x3fc55f6bec9ef961              ; double 0.16697453550109301
	.quad	0xbf83d23336fd10a9              ; double -0.0096782685428780813
	.quad	0xbf88c77a983a068d              ; double -0.012099225779141487
	.quad	0x3f45cdc98db18c8c              ; double 6.6540090064072651E-4
	.quad	0x3f373576ff44ef39              ; double 3.5413890079260022E-4
	.quad	0xbef24614479a1346              ; double -1.7427203124603727E-5
	.quad	0xbed7b85342ea7515              ; double -5.6552935762375829E-6
	.quad	0x3e90abfc294c82a1              ; double 2.4842942396474065E-7
	.quad	0x3e6ea79eab269916              ; double 5.7098949030140279E-8
	.quad	0xbe235bbe38f3529c              ; double -2.2536110266152492E-9
	.quad	0xbdfb5a33186e7193              ; double -3.9802896432910825E-10
	.quad	0x3daefc264aa83bf3              ; double 1.4090328151677641E-11
	.quad	0x3d8145cbb6e8a3a0              ; double 1.9636717850506286E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0x3fd33518b3874e8a              ; double 0.30011575252613254
	.quad	0xbf95e70dc60362bf              ; double -0.021389212809341581
	.quad	0xbfa80c83bdeee593              ; double -0.046970478949741289
	.quad	0x3f69a4b292e3de28              ; double 0.0031302917260480796
	.quad	0x3f613fbc7d695936              ; double 0.002105587143243738
	.quad	0xbf207358bbdbe512              ; double -1.2550790955127199E-4
	.quad	0xbf0796a751a29ac8              ; double -4.4991475264757159E-5
	.quad	0x3ec4255b013502cf              ; double 2.4015807952585114E-6
	.quad	0x3ea3026db6f0dbc4              ; double 5.665268484393475E-7
	.quad	0xbe5d48dca8c5fc90              ; double -2.7273424894801726E-8
	.quad	0xbe3445e1da91dbca              ; double -4.7201704013422047E-9
	.quad	0x3dec62a36e1968d3              ; double 2.0653028510455781E-10
	.quad	0x3dbe721272d8248e              ; double 2.7690106438474045E-11
	.quad	0xbd739f923d874246              ; double -1.1154568938183541E-12
	.quad	0x3fd17dbf09d40d25              ; double 0.27329994163319987
	.quad	0x3c44967f4f7fc629              ; double 2.232142433641675E-18
	.quad	0xbfc1404bf647c28f              ; double -0.13477468037992366
	.quad	0x3f74f4df2769f79d              ; double 0.0051163403464879161
	.quad	0x3f85c6285429b55e              ; double 0.010631861751984214
	.quad	0xbf3d68ab7227e79d              ; double -4.4874368373337155E-4
	.quad	0xbf356acb64517694              ; double -3.2680001851823873E-4
	.quad	0x3eec10b47c6794fc              ; double 1.3382555960237627E-5
	.quad	0x3ed67eaae7c19ec8              ; double 5.3631771344886525E-6
	.quad	0xbe8bb65280097fdb              ; double -2.0647195244065982E-7
	.quad	0xbe6d871ddeb2db00              ; double -5.499981255970334E-8
	.quad	0x3e20f432b5f8846e              ; double 1.9736935833650959E-9
	.quad	0x3dfa96b19cba8298              ; double 3.8691574660208311E-10
	.quad	0xbdac2077a86562a4              ; double -1.2790599536440081E-11
	.quad	0xbd810893dc905efb              ; double -1.9364854538966978E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0xbfcff654544ebcd0              ; double -0.24970487705784317
	.quad	0x3f89223ff2c0785a              ; double 0.012272357555101521
	.quad	0x3fa4b0c5d5da65d1              ; double 0.04041116939078971
	.quad	0xbf5f91a9ee0d218c              ; double -0.001926818797260396
	.quad	0xbf5f51c24898187f              ; double -0.0019115826893325858
	.quad	0x3f16b4c9ca04065c              ; double 8.6617294531543401E-5
	.quad	0x3f063c547294c80d              ; double 4.2411162505820531E-5
	.quad	0xbebe3725c3bed76a              ; double -1.8009793753942718E-6
	.quad	0xbea25c1053590f28              ; double -5.4715943659979785E-7
	.quad	0x3e57485bc4affd32              ; double 2.1683657796392875E-8
	.quad	0x3e33e27187db1b90              ; double 4.6297313740491136E-9
	.quad	0xbde77b93ff00a8d9              ; double -1.7085932625435941E-10
	.quad	0xbdbdb9d1890c1963              ; double -2.7035506268991825E-11
	.quad	0x3d69bc7332d23c98              ; double 7.3146488801751193E-13
	.quad	0xbfcddceb4ce1bf4a              ; double -0.23330441717143408
	.quad	0xbc44e6f0ff2be5da              ; double -2.2662118296062932E-18
	.quad	0x3fbda52116c0a63f              ; double 0.11580092244607786
	.quad	0xbf6a9da4603b66c1              ; double -0.0032489977328225843
	.quad	0xbf8331e74ea59a28              ; double -0.0093725272060512648
	.quad	0x3f33e5cb6eb9d4d2              ; double 3.0361382116634889E-4
	.quad	0x3f33885fe9aee88d              ; double 2.9804555532176523E-4
	.quad	0xbee494c0f422be24              ; double -9.8138185687649245E-6
	.quad	0xbed512b9d2882a32              ; double -5.0242299853933595E-6
	.quad	0x3e85a86081766e10              ; double 1.6136260748150418E-7
	.quad	0x3e6c323d60d5c85a              ; double 5.2519606534305693E-8
	.quad	0xbe1bcc0c4c6c296f              ; double -1.6180019977389105E-9
	.quad	0xbdf9bbb359d527c1              ; double -3.7446742393781687E-10
	.quad	0x3da7e38db953d46d              ; double 1.0863405480283854E-11
	.quad	0x3d80c831b59f5952              ; double 1.9078934776878299E-12
	.size	__ocmltbl_M64_J1, 960

	.type	__ocmltbl_M64_Y0,@object        ; @__ocmltbl_M64_Y0
	.p2align	3, 0x0
__ocmltbl_M64_Y0:
	.quad	0xbfb2e4d699cbd01f              ; double -0.073804295108687232
	.quad	0x3fc6bbcb41034286              ; double 0.17760601686906713
	.quad	0xbf9075b1bbf41364              ; double -0.016073968025938426
	.quad	0x3f41a6206b7b973d              ; double 5.3860266686165499E-4
	.quad	0xbee3e99794203bbd              ; double -9.4950052052215464E-6
	.quad	0x3e7bce4a600d3ea5              ; double 1.0358476033628097E-7
	.quad	0xbe0a6ee796b871b6              ; double -7.6930799009029322E-10
	.quad	0x3d92393d82c6b2e4              ; double 4.1435657365127101E-12
	.quad	0xbd131085da82054c              ; double -1.6932715179356949E-14
	.quad	0x3c8f4ed4b492ebcc              ; double 5.4310606578547998E-17
	.quad	0xbc04b7ac8a1b15c6              ; double -1.4038708139145727E-19
	.quad	0x3b769201941cc7b8              ; double 2.9871591749670352E-22
	.quad	0xbae4987e57338156              ; double -5.3238579320936111E-25
	.quad	0x3a4ff18d4705632d              ; double 8.0636887083404933E-28
	.quad	0xb9b5416acd087d02              ; double -1.0479788308161506E-30
	.quad	0xbfe8eea0ae99a033              ; double -0.77912935353834312
	.quad	0x4001b052cd42754e              ; double 2.2110954318911018
	.quad	0xc0092f7d329697cf              ; double -3.1481880142409646
	.quad	0x401b0d7849d94041              ; double 6.7631541766023142
	.quad	0xc0308f108854a13f              ; double -16.558846016561116
	.quad	0x4045473065287973              ; double 42.556164402735611
	.quad	0xc05c69a8813d10e8              ; double -113.65090971911889
	.quad	0x40737ec167e18fec              ; double 311.92221820936425
	.quad	0xc08b44127a0228e4              ; double -872.50902177512444
	.quad	0x40a33a1cf6a241c7              ; double 2461.0565691666884
	.quad	0xc0baad0c98bdb9b5              ; double -6829.0492056444537
	.quad	0x40d134629471039b              ; double 17617.540310147782
	.quad	0xc0e29c65ccf79ba0              ; double -38115.181270412402
	.quad	0x40ec922fbc085c32              ; double 58513.491703205174
	.quad	0xc0e655b619071060              ; double -45741.690555126173
	.quad	0xbfe15659a787357b              ; double -0.54179079742759428
	.quad	0x3ffa6174d29845e5              ; double 1.64879305137253
	.quad	0xbff9d0a5f4831145              ; double -1.6134395171403224
	.quad	0x40031f12941f635b              ; double 2.3901721546248331
	.quad	0xc0111bb0813976d0              ; double -4.2770404998133955
	.quad	0x401f8b042ca17533              ; double 7.8857581113382364
	.quad	0xc02e1eb9d27b3826              ; double -15.060011460820601
	.quad	0x403d8cb662fa6df2              ; double 29.549657999172219
	.quad	0xc04d9175a3310c79              ; double -59.136402510594912
	.quad	0x405dfcee0e45019e              ; double 119.95202976931475
	.quad	0xc06e7481fb9e5f61              ; double -243.64086705143112
	.quad	0x407deb3bb4dc60ce              ; double 478.70207677922451
	.quad	0xc08a25fab23792b8              ; double -836.74741023460865
	.quad	0x4091402bbfbaed82              ; double 1104.0427235801185
	.quad	0xc0885db459e00d9b              ; double -779.71306204835435
	.quad	0xbfd6da72f31dca44              ; double -0.35708307020027896
	.quad	0x3ff54dfd34c830f7              ; double 1.3315403043553127
	.quad	0xbff014af25dc721e              ; double -1.0050498465490203
	.quad	0x3ff13366c90bec01              ; double 1.0750491956121098
	.quad	0xbff8c024b43a4764              ; double -1.5469100036757135
	.quad	0x4001e294a410a304              ; double 2.2356350724770682
	.quad	0xc00a97f320b0ea96              ; double -3.324194198035296
	.quad	0x40144f870b1ac185              ; double 5.0776635871010329
	.quad	0xc01fa37c80ea46f7              ; double -7.9096546309462985
	.quad	0x402900da91537e82              ; double 12.50166753906456
	.quad	0xc033e7dbeab87589              ; double -19.905699415239301
	.quad	0x403f3ec6d4cf71aa              ; double 31.245221424718387
	.quad	0xc046a7aba5d5442a              ; double -45.309925774701995
	.quad	0x404a0c08542c3074              ; double 52.094004174782555
	.quad	0xc040c45498a8b8a5              ; double -33.533831674941474
	.quad	0xbfca2f2e18b92a4f              ; double -0.20456482131187889
	.quad	0x3ff1eedd9c1f2bd8              ; double 1.1208168123728139
	.quad	0xbfe6cfb9aba1d4a6              ; double -0.7128570892515611
	.quad	0x3fe1baba8b2960d8              ; double 0.55404402904516825
	.quad	0xbfe5c9a23cecfefa              ; double -0.68086349391521073
	.quad	0x3fea201bb93ebe9b              ; double 0.8164194696491508
	.quad	0xbfefccef9c6db67b              ; double -0.99376659920171961
	.quad	0x3ff3e3d322601093              ; double 1.2431212752135579
	.quad	0xbff95e86cad83531              ; double -1.585577766763276
	.quad	0x40006b019fb826fc              ; double 2.0522491911004845
	.quad	0xc005748824585339              ; double -2.6819002952055624
	.quad	0x400be6f54263d628              ; double 3.4877724825589844
	.quad	0xc0112ac8ac6925df              ; double -4.2917811335732656
	.quad	0x40116432c5740749              ; double 4.3478499271457816
	.quad	0xc0048433915014ef              ; double -2.5645514824451463
	.quad	0x0000000000000000              ; double 0
	.quad	0x3fec24371844b88a              ; double 0.87942080249719479
	.quad	0xbfdf7e38a46d7102              ; double -0.49207893426297755
	.quad	0x3fcc3b1338af433e              ; double 0.22055282848170948
	.quad	0xbfccf18e6a4b4233              ; double -0.22612171354423224
	.quad	0x3fcc068086ad19c8              ; double 0.21894842697129335
	.quad	0xbfca396a800cbb37              ; double -0.20487719776562027
	.quad	0x3fc9424bb759c9a8              ; double 0.1973356862323048
	.quad	0xbfc8d35c00975f04              ; double -0.1939501765143562
	.quad	0x3fc8c0719fb178f7              ; double 0.19337292001268455
	.quad	0xbfc8f72da405de4e              ; double -0.19504328259403042
	.quad	0x3fc976eb13434cf9              ; double 0.19894159737177811
	.quad	0xbfca693e0695b82e              ; double -0.20633673974538297
	.quad	0x3fca39aaeeee6dcf              ; double 0.20488487879343473
	.quad	0xbfc041222b5cf46e              ; double -0.12698771588648888
	.quad	0x3fb6980226f358df              ; double 0.088256964215676956
	.quad	0x3fe8ffb207d66b94              ; double 0.78121282130028868
	.quad	0xbfdbd2b24cb4d65a              ; double -0.43473489275797805
	.quad	0x3fc28c76ddcd8ebf              ; double 0.14491163091871859
	.quad	0xbfc19b76c6c2d753              ; double -0.13755688386089079
	.quad	0x3fbfe1a296ce781f              ; double 0.12453666860389533
	.quad	0xbfbaa16d38b246fe              ; double -0.10402567514600133
	.quad	0x3fb6e7c77660784e              ; double 0.089474169159502653
	.quad	0xbfb4223fd6f63950              ; double -0.078647603970442903
	.quad	0x3fb1ede638013f16              ; double 0.070036305115760505
	.quad	0xbfb00c129cf3fa8e              ; double -0.062684214895833729
	.quad	0x3fac25552887122d              ; double 0.054972325513095256
	.quad	0xbfa6828823344907              ; double -0.043964628503220075
	.quad	0x3f9c0732c7410916              ; double 0.027371209537030948
	.quad	0xbf8330c8d93428ff              ; double -0.0093703929219555153
	.quad	0x3fd0869ff937fa12              ; double 0.25821685159454077
	.quad	0x3fe2b31c35470a4a              ; double 0.58436403661500802
	.quad	0xbfd73900273b3659              ; double -0.36285404044324349
	.quad	0x3faf970acb313f75              ; double 0.0616992352521483
	.quad	0xbfa76b24758c6a51              ; double -0.045739306782895846
	.quad	0x3fa4d6f05a2f473b              ; double 0.040702353485939168
	.quad	0xbf9be8df6d1412d3              ; double -0.02725552657377046
	.quad	0x3f93098c63847dda              ; double 0.018591111730641298
	.quad	0xbf8ad67d917f84ef              ; double -0.013104420664549169
	.quad	0x3f8320b5b7ab98e4              ; double 0.0093397328068473631
	.quad	0xbf7b39dafcf12ccd              ; double -0.0066469721051120702
	.quad	0x3f72ac1299be4543              ; double 0.004558632524905906
	.quad	0xbf66a630582d99dd              ; double -0.0027647918918092111
	.quad	0x3f5497f4183f528f              ; double 0.0012569316613639003
	.quad	0xbf33eb6aa5da7d32              ; double -3.0394891460079892E-4
	.quad	0x3fdb7362a42dd8ff              ; double 0.42891756089319694
	.quad	0x3fd53a7b3f0dfb71              ; double 0.33169442327191861
	.quad	0xbfd441d73e1b39cb              ; double -0.31651860299180318
	.quad	0x3f9f505223be8c30              ; double 0.030579837257061537
	.quad	0xbf7371ca702291b7              ; double -0.0047471912131737326
	.quad	0x3f8599ba9529ff05              ; double 0.01054712074005649
	.quad	0xbf7813569be4ac1d              ; double -0.0058778174555227632
	.quad	0x3f67e92dac4148ad              ; double 0.0029188053177132329
	.quad	0xbf59ed66dc5a6048              ; double -0.0015824799060393403
	.quad	0x3f4ca8cc8b2b25b1              ; double 8.7461459619324864E-4
	.quad	0xbf3fb5d5d524368a              ; double -4.8386068841997003E-4
	.quad	0x3f313e17d93243dc              ; double 2.6310045468230596E-4
	.quad	0xbf214016505428d9              ; double -1.3160965333042818E-4
	.quad	0x3f0b3531e22732f6              ; double 5.189474565590005E-5
	.quad	0xbee7e3f31e3d1eff              ; double -1.1391844004684635E-5
	.quad	0x3fe0aa48442f014b              ; double 0.52078641240226753
	.quad	0xbc42fc44b41b87df              ; double -2.0584037223089672E-18
	.quad	0xbfd0aa48442f014c              ; double -0.26039320620113382
	.quad	0x3fa439fac165269b              ; double 0.039504848583033346
	.quad	0x3f80d2af4e933a41              ; double 0.0082143493513316973
	.quad	0x3f4f71646bcf7f6c              ; double 9.5956233382919537E-4
	.quad	0xbf5444bda8e8462d              ; double -0.001237092222826762
	.quad	0x3f384c22162349fd              ; double 3.7074882687906913E-4
	.quad	0xbf217ab499428eef              ; double -1.3335661481505371E-4
	.quad	0x3f0dafa7e064beaa              ; double 5.6621847806301765E-5
	.quad	0xbef8bb68be4d8127              ; double -2.3586337096205168E-5
	.quad	0x3ee490083e101288              ; double 9.8050240371430495E-6
	.quad	0xbed1512541c86fb3              ; double -4.1286885133182863E-6
	.quad	0x3ebc67c5be2b19cf              ; double 1.6930914560772783E-6
	.quad	0xbea0aef1edf4c84c              ; double -4.972034410076654E-7
	.quad	0x3fdf922e9b7fcff3              ; double 0.49329724488711618
	.quad	0xbfc46ae4b2d59fba              ; double -0.15951212627555639
	.quad	0xbfcb89b5949e4e6b              ; double -0.21514005429036173
	.quad	0x3fa9fe2b318dc766              ; double 0.05076727847962452
	.quad	0x3f80aa736e5f234e              ; double 0.0081376092965840495
	.quad	0xbf4c32fc82583918              ; double -8.6057023571742535E-4
	.quad	0xbf4582f115e796b3              ; double -6.5647861248115667E-4
	.quad	0x3f25ca419d5397d0              ; double 1.6624499281830832E-4
	.quad	0xbf04ccbf0f49a3cd              ; double -3.9672451667644924E-5
	.quad	0x3eefeb1f3c777328              ; double 1.521990078761635E-5
	.quad	0xbed7d81115e556b5              ; double -5.6848551522514057E-6
	.quad	0x3ec0dc1871a64faa              ; double 2.0098385792952419E-6
	.quad	0xbea690f94672b956              ; double -6.7252825610378242E-7
	.quad	0x3e88de4474fff121              ; double 1.8528276735086861E-7
	.quad	0xbe5fd1f6321a642b              ; double -2.96348360353022E-8
	.quad	0x3fd81e4f8120242a              ; double 0.37685001001279039
	.quad	0xbfd4c7773d150462              ; double -0.32467442479179998
	.quad	0xbfc13127c21922b4              ; double -0.13431260087442853
	.quad	0x3fb0224f7ebcb4e0              ; double 0.063023537103350957
	.quad	0x3f7240000575c220              ; double 0.0044555664857033606
	.quad	0xbf6135aa20d0a769              ; double -0.0021007845703210804
	.quad	0xbf3161ce7505eb62              ; double -2.6522913415021586E-4
	.quad	0x3f17b51bec1f5e5a              ; double 9.0436772580354373E-5
	.quad	0xbee3290b9a006192              ; double -9.1363588694971669E-6
	.quad	0x3ec677be1fc5d2ea              ; double 2.678363897052446E-6
	.quad	0xbeb15e4f8fa357f4              ; double -1.0352374020714479E-6
	.quad	0x3e9505e7de49ad7f              ; double 3.1326814412562562E-7
	.quad	0xbe77d76c65326b71              ; double -8.8816500198197073E-8
	.quad	0x3e572bd064db5810              ; double 2.1579813761319482E-8
	.quad	0xbe2aeeae21c65c13              ; double -3.1353375574613876E-9
	.quad	0x0000000000000000              ; double 0
	.quad	0xbfd9c34256a12a0c              ; double -0.40254267177502423
	.quad	0x3faa09c9290367ef              ; double 0.050855909592158237
	.quad	0x3fadf6d59bf50fe5              ; double 0.058523822105172298
	.quad	0xbf7c116fdc598542              ; double -0.0068525666771120392
	.quad	0xbf61e32bc4f26dbb              ; double -0.0021835188741314549
	.quad	0x3f299827653efc95              ; double 1.9526940252310014E-4
	.quad	0x3f0ab2c209548fe9              ; double 5.0922915003220725E-5
	.quad	0xbed4863787f98471              ; double -4.893370828180496E-6
	.quad	0xbe93b2382f029e6c              ; double -2.9349580100499912E-7
	.quad	0xbe57737c6a81e739              ; double -2.1840554837306538E-8
	.quad	0x3e545853fa20e785              ; double 1.8947787013197809E-8
	.quad	0xbe2fd2a529aab5e4              ; double -3.7046653083214055E-9
	.quad	0x3e0a42ddad39ad95              ; double 7.6430136737808286E-10
	.quad	0xbde112e48fb82cd5              ; double -1.2422824562419603E-10
	.quad	0xbfd5c7c556f0c19a              ; double -0.34031804552344058
	.quad	0x3c65b2c3f10bb869              ; double 9.4101386107437923E-18
	.quad	0x3fc5c7c556f0c19c              ; double 0.17015902276172035
	.quad	0xbf8564d4b1ed0eb7              ; double -0.010446225814696104
	.quad	0xbf8a15d92dfe4293              ; double -0.012736984935856987
	.quad	0x3f4b43843047ed3c              ; double 8.3202318688738825E-4
	.quad	0x3f37a8924cc88cc3              ; double 3.6099979186783262E-4
	.quad	0xbef5f69b4bc9edfd              ; double -2.094584191290708E-5
	.quad	0xbed85b94153d61e4              ; double -5.807334975426314E-6
	.quad	0x3e955ac235b60413              ; double 3.1820723275099964E-7
	.quad	0x3e6d56458c85b80d              ; double 5.4644418381581921E-8
	.quad	0xbe23ec1ea0457428              ; double -2.3192658923317209E-9
	.quad	0xbe000934b9fda092              ; double -4.6670788412863401E-10
	.quad	0x3dc0ae48dc572273              ; double 3.0342197107751323E-11
	.quad	0xbd45950b4ca3ef99              ; double -1.5335078035720072E-13
	.quad	0x0000000000000000              ; double 0
	.quad	0x3fd334cca0697a5a              ; double 0.30009761491047515
	.quad	0xbf95aef611fc4d57              ; double -0.02117523655676953
	.quad	0xbfa8969c64cbf437              ; double -0.048024070076259688
	.quad	0x3f6b2f14a95527b4              ; double 0.0033183482688956215
	.quad	0x3f61d35e85fdbc6f              ; double 0.0021759840164388626
	.quad	0xbf226dd71e3904d7              ; double -1.4060259774065803E-4
	.quad	0xbf08177e4f94ce0e              ; double -4.5951406671209627E-5
	.quad	0x3ec6a92273315ba3              ; double 2.7013637918060206E-6
	.quad	0x3ea34aa706e77dbb              ; double 5.7493481425343561E-7
	.quad	0xbe60a281377e8b1e              ; double -3.0984700082815648E-8
	.quad	0xbe344251968be3c8              ; double -4.7169293824539995E-9
	.quad	0x3defa6a0d37c6134              ; double 2.3029054509089804E-10
	.quad	0x3dbec1d47eab32bb              ; double 2.7973463750937909E-11
	.quad	0xbd76fb9a346b2386              ; double -1.3064221620824322E-12
	.quad	0x3fd15f993fceab5c              ; double 0.27145987731153354
	.quad	0x3c474335059e1f4e              ; double 2.5221283178979202E-18
	.quad	0xbfc15f993fceab5b              ; double -0.13572993865576674
	.quad	0x3f758ef6efbed6f1              ; double 0.0052632947880988249
	.quad	0x3f86395dfe49fba8              ; double 0.010851606676849659
	.quad	0xbf3fb15104a36e0f              ; double -4.8359134656347861E-4
	.quad	0xbf35f88a11d3d03a              ; double -3.3524866905954335E-4
	.quad	0x3eef37d226054dae              ; double 1.4885926419217314E-5
	.quad	0x3ed6f7baaf1eb952              ; double 5.4759245688276119E-6
	.quad	0xbe8f0c45054039d9              ; double -2.3132509119378261E-7
	.quad	0xbe6dfe0f689fe34d              ; double -5.5865240503001577E-8
	.quad	0x3e23115a93b5a609              ; double 2.2197827167333757E-9
	.quad	0x3dfad1aae15a8f0e              ; double 3.902680135255005E-10
	.quad	0xbdaf829cbaca6bc1              ; double -1.432918179702368E-11
	.quad	0xbd81191eaead7d7c              ; double -1.9438316968801126E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0xbfcff635cc72b9f1              ; double -0.24970123751468479
	.quad	0x3f89036451ff57c5              ; double 0.012213500740397518
	.quad	0x3fa4e667a7155698              ; double 0.040820349832455693
	.quad	0xbf60325ee41e90fc              ; double -0.0019771436063412678
	.quad	0xbf5fe23914fb4daa              ; double -0.0019460256043445181
	.quad	0x3f17f84d7c50bff4              ; double 9.1438035341395555E-5
	.quad	0x3f06afdd5774b982              ; double 4.3271963415458643E-5
	.quad	0xbec04053abd359b6              ; double -1.9373031522149207E-6
	.quad	0xbea2aea9a040b3be              ; double -5.5677520594475753E-7
	.quad	0x3e593eb9b7c33886              ; double 2.3511258260421399E-8
	.quad	0x3e342852d79837dc              ; double 4.6932869756461152E-9
	.quad	0xbde99d5155505d29              ; double -1.8637017854067416E-10
	.quad	0xbdbe747d553e2be7              ; double -2.7698695184429241E-11
	.quad	0x3d723dd8d96219f9              ; double 1.0369143470533368E-12
	.quad	0xbfcdc14ea14e89f9              ; double -0.23246176601703875
	.quad	0xbc42890a10af0448              ; double -2.0096023187886984E-18
	.quad	0x3fbdc14ea14e89f8              ; double 0.11623088300851936
	.quad	0xbf6b037fe9cf2945              ; double -0.0032975672060945615
	.quad	0xbf8367d7d608e3ff              ; double -0.0094754087632384891
	.quad	0x3f34abef563655fd              ; double 3.1542390044000929E-4
	.quad	0x3f33d8a66121994d              ; double 3.0283033368618402E-4
	.quad	0xbee5cfe92992edfe              ; double -1.0400844347883093E-5
	.quad	0xbed5718148dc24ef              ; double -5.1124999467324779E-6
	.quad	0x3e87414daf653481              ; double 1.7326393448661488E-7
	.quad	0x3e6ca704f47d3c94              ; double 5.3369289930627683E-8
	.quad	0xbe1e0aaea47f3944              ; double -1.748658677916985E-9
	.quad	0xbdfa14b5af307e06              ; double -3.7952700634084812E-10
	.quad	0x3da9e7e5ccbe0c5a              ; double 1.1780616758320276E-11
	.quad	0x3d80e377f8a6a708              ; double 1.9200057712000833E-12
	.size	__ocmltbl_M64_Y0, 2160

	.type	__ocmltbl_M64_Y1,@object        ; @__ocmltbl_M64_Y1
	.p2align	3, 0x0
__ocmltbl_M64_Y1:
	.quad	0xbfc91866143cbc8a              ; double -0.19605709064623894
	.quad	0x3fabd3975c75b4a7              ; double 0.054348688160510243
	.quad	0xbf6835b97894be5b              ; double -0.0029553053360798338
	.quad	0x3f12c7dbffcde97d              ; double 7.1642687499739622E-5
	.quad	0xbeb0a780ac776eac              ; double -9.926740619424822E-7
	.quad	0x3e432e5a4ddeea30              ; double 8.9318796212201323E-9
	.quad	0xbdcf0ce34d2066a6              ; double -5.6480245515956583E-11
	.quad	0x3d52a4e1aea45c18              ; double 2.6494815070087779E-13
	.quad	0xbcd1474ade9154ac              ; double -9.591486586335139E-16
	.quad	0x3c4978ba84f218c0              ; double 2.7616359783782749E-18
	.quad	0xbbbe9598c0163788              ; double -6.4764905786424365E-21
	.quad	0x3b2e7e5fcfc49d52              ; double 1.2611877823331126E-23
	.quad	0xba99a6c125cd4a4b              ; double -2.0721023543487956E-26
	.quad	0x3a0273872098881c              ; double 2.9110987879568908E-29
	.quad	0xb966e9d99d76d143              ; double -3.5303800868251435E-32
	.quad	0xbff78b26a2b7c4df              ; double -1.4714723926702431
	.quad	0x4003fcc6cc40cdc8              ; double 2.4984260518337784
	.quad	0xc012d291e3254d05              ; double -4.7056346408383023
	.quad	0x4023f3a228303640              ; double 9.9758465346195634
	.quad	0xc0342f25541834d5              ; double -20.18416333762146
	.quad	0x40443f9c12bf4ab6              ; double 40.496950477031916
	.quad	0xc05449bfbbf62991              ; double -81.152327528374613
	.quad	0x40644fb5451116bc              ; double 162.4908776601568
	.quad	0xc0745269ac3fa0db              ; double -325.15079903464147
	.quad	0x40844ba4401f2779              ; double 649.4552004274293
	.quad	0xc09414d76e7b7997              ; double -1285.2103823941195
	.quad	0x40a320f5fe16f70a              ; double 2448.4804541756212
	.quad	0xc0b03e4b57e41df5              ; double -4158.2943098614824
	.quad	0x40b4f69e69a4f4fb              ; double 5366.6187995050523
	.quad	0xc0ad2dbb0f5b271a              ; double -3734.8653515324813
	.quad	0xbff3797262d3470d              ; double -1.2171501026500124
	.quad	0x3ffab7e1edefafa8              ; double 1.6698931974778848
	.quad	0xc0024846fb79f39a              ; double -2.2852916380492845
	.quad	0x40101bf3f4fa0318              ; double 4.0272978093714968
	.quad	0xc01a594eab7356d8              ; double -6.5872141636989099
	.quad	0x402529f452075247              ; double 10.581942141908383
	.quad	0xc030fae25fdc4e3b              ; double -16.98001670006327
	.quad	0x403b37934724e423              ; double 27.217091032511359
	.quad	0xc045cc0de6c9e845              ; double -43.594174240672636
	.quad	0x4051708d1a40185c              ; double 69.75861221560757
	.quad	0xc05bc7f31a7d6e5c              ; double -111.12421285866861
	.quad	0x4065a379ba1401c8              ; double 173.10860923678979
	.quad	0xc06f0b0ad21c1a88              ; double -248.34507089127533
	.quad	0x4071ae75edd45deb              ; double 282.90379126506622
	.quad	0xc066a3bb6c3bc030              ; double -181.116628758145
	.quad	0xbff099fcbe60fd83              ; double -1.0375945507692854
	.quad	0x3ff3f0ca40455e64              ; double 1.2462866316399408
	.quad	0xbff3bff758706437              ; double -1.2343667463922097
	.quad	0x3ffe635f86eb952c              ; double 1.8992610235521381
	.quad	0xc00518fb90840d37              ; double -2.6371985712336499
	.quad	0x400c3f8901b6b53e              ; double 3.5310230382807779
	.quad	0xc012e70c7140d7a2              ; double -4.7256334014727219
	.quad	0x401944bba4ae4a2a              ; double 6.3171220523241036
	.quad	0xc020de9423c41cb2              ; double -8.4347239663023608
	.quad	0x40267a063c3e4e1f              ; double 11.238328821759806
	.quad	0xc02daa214568b0f7              ; double -14.832285088567444
	.quad	0x4032d7af68b5b7bb              ; double 18.842520279278443
	.quad	0xc03555c5991eb83a              ; double -21.335046358108436
	.quad	0x40325ad3bcd1c212              ; double 18.354793359003516
	.quad	0xc021074c12c09791              ; double -8.5142522678468442
	.quad	0xbfeacbf57f2ddca8              ; double -0.83739733543088324
	.quad	0x3fedca17107b904e              ; double 0.93091920108100523
	.quad	0xbfe1bbd2b0457cdf              ; double -0.55417761257185905
	.quad	0x3fe77a8f333ccbb2              ; double 0.73371086127587248
	.quad	0xbfeb89c1b814324b              ; double -0.86056600525768923
	.quad	0x3fed760af4ffb08b              ; double 0.92065952159238529
	.quad	0xbfef8cf4a5a36f58              ; double -0.98595650054219685
	.quad	0x3ff0e518d641e18f              ; double 1.0559318894794136
	.quad	0xbff20e81208fa7bc              ; double -1.1285411140365644
	.quad	0x3ff3376b177dad58              ; double 1.2010298650373752
	.quad	0xbff41c66d0e0c209              ; double -1.2569339904113142
	.quad	0x3ff3e3fed2ba82a1              ; double 1.2431629401764115
	.quad	0xbff1009bef1c1ed4              ; double -1.0626487102726303
	.quad	0x3fe551ad045b2b05              ; double 0.66622019625478457
	.quad	0xbfcbf968fb74c1f5              ; double -0.21854889181260231
	.quad	0xbfe36e6b6b7643f7              ; double -0.60722895611445338
	.quad	0x3fe79c5f275090c0              ; double 0.73783834150938077
	.quad	0xbfca0c195b672e36              ; double -0.20349423373260017
	.quad	0x3fcae3c79b655957              ; double 0.21007628524484787
	.quad	0xbfcd944bfbb59e94              ; double -0.23108815947056327
	.quad	0x3fc859ba5c97b008              ; double 0.19023828049773805
	.quad	0xbfc3e9c794c4910b              ; double -0.15557188762716864
	.quad	0x3fc073cbe8a621fb              ; double 0.12853382930576615
	.quad	0xbfbb1cf7a1d06a0b              ; double -0.10591075611629479
	.quad	0x3fb64331589b8d85              ; double 0.086962780125352593
	.quad	0xbfb214cbe1d92f45              ; double -0.070629828108562512
	.quad	0x3fac3017799ba518              ; double 0.055054410547947963
	.quad	0xbfa37c91f275a49c              ; double -0.038059769484626943
	.quad	0x3f945a0ee3e461f4              ; double 0.019874794635230189
	.quad	0xbf76ce6db619dfcb              ; double -0.0055679593657415689
	.quad	0xbfd9145d558c1484              ; double -0.39186795572488387
	.quad	0x3fe4d465c2cc8bb9              ; double 0.65092742964440398
	.quad	0xbfb9a53a6fc4f8d2              ; double -0.10017743328805587
	.quad	0x3fa5a04ef5b3be17              ; double 0.042238681309637531
	.quad	0xbfb2870dca6bbba8              ; double -0.07237325851359222
	.quad	0x3fa959dc1b5ca22f              ; double 0.049513700809545085
	.quad	0xbf9fd1701af4f6c8              ; double -0.031072379727666882
	.quad	0x3f94f4669c520714              ; double 0.020463565150300303
	.quad	0xbf8b9c51b6264199              ; double -0.013481748934993475
	.quad	0x3f821867b6956ed4              ; double 0.0088356115908746827
	.quad	0xbf7793000339b970              ; double -0.0057554245464487147
	.quad	0x3f6df65915265695              ; double 0.0036575069327209977
	.quad	0xbf6174d53e085260              ; double -0.0021309056176142399
	.quad	0x3f500d917dbc1489              ; double 9.7979744072177106E-4
	.quad	0xbf307f689e7c1ff8              ; double -2.5173477341455764E-4
	.quad	0xbfc9482110ce7907              ; double -0.19751370735770754
	.quad	0x3fe300298c4bc6db              ; double 0.59376981164515585
	.quad	0xbfb760867541f31a              ; double -0.091316608073566035
	.quad	0xbf8c1c01516c919f              ; double -0.013725290582052461
	.quad	0xbf99ce754717084f              ; double -0.025201637710559329
	.quad	0x3f92149f4f3e4a7d              ; double 0.017656792842510861
	.quad	0xbf8141d386a70a68              ; double -0.0084263349025423678
	.quad	0x3f7298e689470b8f              ; double 0.0045403485605132318
	.quad	0xbf6493304450d3e1              ; double -0.0025115912162854003
	.quad	0x3f5678e25a319117              ; double 0.0013715944740165292
	.quad	0xbf4872da1bce2dfb              ; double -7.461132987471303E-4
	.quad	0x3f3a677fb40dbf50              ; double 4.0289752728649587E-4
	.quad	0xbf2b726429d57741              ; double -2.0940277765196284E-4
	.quad	0x3f188b8a000ddeb3              ; double 9.3632028450469845E-5
	.quad	0xbefb11671433a545              ; double -2.5814036473647125E-5
	.quad	0x0000000000000000              ; double 0
	.quad	0x3fe0aa48442f014b              ; double 0.52078641240226753
	.quad	0xbfbe56f82217b8f1              ; double -0.1185145457490966
	.quad	0xbfa0d2af4e932386              ; double -0.032857397405286412
	.quad	0xbf73a6dec37290aa              ; double -0.0047978116701054372
	.quad	0x3f7e671c7d1198c2              ; double 0.0074225533327078614
	.quad	0xbf65429dc5a4571b              ; double -0.0025952416882643165
	.quad	0x3f517ab4afa1770f              ; double 0.0010668529999046693
	.quad	0xbf40b2d877cec32a              ; double -5.0960130430697148E-4
	.quad	0x3f2eea7bf7a3af87              ; double 2.3587001107416522E-4
	.quad	0xbf1c3fae660aeefb              ; double -1.0776044792753716E-4
	.quad	0x3f09d11d21c2ece2              ; double 4.9241735014382703E-5
	.quad	0xbef79526605f9903              ; double -2.2490135982788418E-5
	.quad	0x3ee5c5b6bee7147f              ; double 1.0381851066729737E-5
	.quad	0xbed3d818acf4319f              ; double -4.7312084483604928E-6
	.quad	0x3faded04eb2f8f23              ; double 0.058448938092423817
	.quad	0x3fdf7eb2f74619db              ; double 0.49210809848628195
	.quad	0xbfc0a92031647abd              ; double -0.13016130840056475
	.quad	0xbfa17d0aac12caf8              ; double -0.034157117371611478
	.quad	0xbf501b12268fc3be              ; double -9.8301670572829803E-4
	.quad	0x3f781b3a8783d65c              ; double 0.0058853422453829206
	.quad	0xbf5f13c3971e1240              ; double -0.0018968019544171183
	.quad	0x3f46af0f6bafeeb3              ; double 6.9225552522263754E-4
	.quad	0xbf34df6f71c51090              ; double -3.1849356470937343E-4
	.quad	0x3f227de227af4204              ; double 1.4108071977016201E-4
	.quad	0xbf0ffddd56e31177              ; double -6.1019246332646758E-5
	.quad	0x3efb3f437408af5d              ; double 2.5984881405857199E-5
	.quad	0xbee5aeef5a8d616e              ; double -1.0339422105751848E-5
	.quad	0x3ecc00d34be2e087              ; double 3.3382444533901784E-6
	.quad	0xbea4c7f0e4847db1              ; double -6.1932264209037928E-7
	.quad	0x3fcec444c4c077ca              ; double 0.24036464316389888
	.quad	0x3fd754d9f5ebee6e              ; double 0.36455391898900913
	.quad	0xbfc5dbc72a6fcd06              ; double -0.17076959201913428
	.quad	0xbf9c45318021591e              ; double -0.027607701726389704
	.quad	0x3f7f6232ad13a468              ; double 0.0076620082411206006
	.quad	0x3f6675fb15dd65cc              ; double 0.002741804505529832
	.quad	0xbf4b70dc1dfab822              ; double -8.3742854982005546E-4
	.quad	0x3f25178509c8dffd              ; double 1.6091822625852173E-4
	.quad	0xbf10fba65912dfad              ; double -6.4785030434387755E-5
	.quad	0x3efb97b9cab9dd89              ; double 2.631442900599476E-5
	.quad	0xbee42df3752f458b              ; double -9.6223335840663507E-6
	.quad	0x3ecd263c5a2a0cf3              ; double 3.4748743059101634E-6
	.quad	0xbeb419a6ed1f3133              ; double -1.1980654801456739E-6
	.quad	0x3e9722448d6c6b4d              ; double 3.4472135494879577E-7
	.quad	0xbe6f968c1760ea44              ; double -5.883737490315062E-8
	.quad	0x3fdaabb4011ed330              ; double 0.41672992810645137
	.quad	0x3c97623d98c40fbf              ; double 8.1128688460579778E-17
	.quad	0xbfc8b45babe797c1              ; double -0.19300409215719408
	.quad	0x3f8e147099a6d924              ; double 0.01468742340953761
	.quad	0x3f88c5af1eeb4695              ; double 0.012095802432131189
	.quad	0xbf4133fa47a23c24              ; double -5.249950475149129E-4
	.quad	0xbf3bf8af944ed4c6              ; double -4.2681013683971666E-4
	.quad	0x3f021d6483f67c2f              ; double 3.4551267613418576E-5
	.quad	0x3eb44d30d299b6f7              ; double 1.2100652590179382E-6
	.quad	0x3eb14c792dd315f1              ; double 1.0310843017597675E-6
	.quad	0xbe9b8f5a5b07796d              ; double -4.106755922254704E-7
	.quad	0x3e7a741606128773              ; double 9.8546821830054318E-8
	.quad	0xbe5bde8e4e6a28d8              ; double -2.5955363104051317E-8
	.quad	0x3e40cb2c5ba5e6b9              ; double 7.8201506283918031E-9
	.quad	0xbe22967744b15f9c              ; double -2.163899758634188E-9
	.quad	0x3fd7843613523e7f              ; double 0.36744453322260279
	.quad	0xbfc75654a46b95c5              ; double -0.18232210186321943
	.quad	0xbfc368bc54be4f06              ; double -0.15163377893315316
	.quad	0x3fa31bfbc9cb82a3              ; double 0.037322872527288518
	.quad	0x3f82cc37920f76a3              ; double 0.0091785756539438153
	.quad	0xbf5af2c8815e3fe3              ; double -0.0016447980937961341
	.quad	0xbf32a7121c69428c              ; double -2.8461639559388611E-4
	.quad	0x3f075298ecb8c751              ; double 4.4484416858016553E-5
	.quad	0x3ec63ded0744f6f9              ; double 2.6514408607837474E-6
	.quad	0xbe8754d788cb6020              ; double -1.738325789066566E-7
	.quad	0xbe8027347b18e32a              ; double -1.2035030532030089E-7
	.quad	0x3e597c5adcdac214              ; double 2.373546497427958E-8
	.quad	0xbe322680b0fbe186              ; double -4.2259695879330589E-9
	.quad	0x3e0e64b283a65579              ; double 8.8456287372942357E-10
	.quad	0xbde02608b566f8d5              ; double -1.1749631363438851E-10
	.quad	0x0000000000000000              ; double 0
	.quad	0xbfd5c7c556f0c199              ; double -0.34031804552344053
	.quad	0x3fa00b9f8571ca1f              ; double 0.031338677444086686
	.quad	0x3faa15d92dfe3dd1              ; double 0.050947939743419497
	.quad	0xbf710a329e2c23b2              ; double -0.0041601159343906281
	.quad	0xbf61be6db991a919              ; double -0.00216599875107194
	.quad	0x3f2337c7e137e72d              ; double 1.4662089289157448E-4
	.quad	0x3f085b940d416677              ; double 4.6458678895700102E-5
	.quad	0xbec806191189d631              ; double -2.8638625162956867E-6
	.quad	0xbea255e5098a01b5              ; double -5.4644125942198327E-7
	.quad	0x3e5b62c536f9e018              ; double 2.5505034027877052E-8
	.quad	0x3e3808e3d2ac8fca              ; double 5.5960207950021691E-9
	.quad	0xbdfa799f4759a3e7              ; double -3.8526321659827535E-10
	.quad	0x3d94ec0a14b5cdf4              ; double 4.7571185910585837E-12
	.quad	0xbd95412530847c52              ; double -4.8327078086606378E-12
	.quad	0xbfd36732d4b96094              ; double -0.30317374013748943
	.quad	0xbc3ceef52886c58e              ; double -1.5684842920394413E-18
	.quad	0x3fc3001c8002caf7              ; double 0.14844089746983233
	.quad	0xbf7bf5a03bab4931              ; double -0.00682604399726676
	.quad	0xbf8751ea028c1873              ; double -0.011386707499252169
	.quad	0x3f423874cd8ccda2              ; double 5.5604651706746646E-4
	.quad	0x3f364f6610d5226e              ; double 3.404258903470296E-4
	.quad	0xbef02978de052c61              ; double -1.5413284814952045E-5
	.quad	0xbed72f07655a8eb0              ; double -5.5274263865177849E-6
	.quad	0x3e8f208123bb6540              ; double 2.3191400254952197E-7
	.quad	0x3e6defd3e8ed0235              ; double 5.5761686038137684E-8
	.quad	0xbe2205926b336e7e              ; double -2.0980096215935157E-9
	.quad	0xbdfb62d4804fb244              ; double -3.9851955096283249E-10
	.quad	0x3db00c025fda5d77              ; double 1.4594580744289001E-11
	.quad	0x3d800419ac68aae1              ; double 1.8208102967600171E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0x3fd15f993fceab5c              ; double 0.27145987731153354
	.quad	0xbf902b3933cf21b1              ; double -0.015789884364296906
	.quad	0xbfa6395dfe49fcbd              ; double -0.04340642670740056
	.quad	0x3f63ced2a2e6916f              ; double 0.0024179567328294553
	.quad	0x3f607a678d5fdf7b              ; double 0.0020114920143860494
	.quad	0xbf1b50d7e1d2f596              ; double -1.042014850609257E-4
	.quad	0xbf06f7bab0bc8947              ; double -4.3807396734390487E-5
	.quad	0x3ec176e72bd30af3              ; double 2.0819264522088035E-6
	.quad	0x3ea2becae04d97fc              ; double 5.5865297153285871E-7
	.quad	0xbe5a384ea928e49a              ; double -2.4419231590119172E-8
	.quad	0xbe341e2aa12c9a44              ; double -4.6840491648468389E-9
	.quad	0x3de9e2e75967aac7              ; double 1.8834793161094203E-10
	.quad	0x3dbe6fcc09518560              ; double 2.7682023845401217E-11
	.quad	0xbd7243fbe2456b83              ; double -1.0382770024573064E-12
	.quad	0x3fd00ef3745e0e3c              ; double 0.25091253627781263
	.quad	0x3c4354a449398e41              ; double 2.0958312999524091E-18
	.quad	0xbfbfcdacdda138f1              ; double -0.12423210535891706
	.quad	0x3f706cc34cd82970              ; double 0.0040099743760130119
	.quad	0x3f84641bb10c15c9              ; double 0.0099565661817092743
	.quad	0xbf37fac943e210ba              ; double -3.6590017033001252E-4
	.quad	0xbf34769ed32cc451              ; double -3.1224610863761932E-4
	.quad	0x3ee80608ec528cbd              ; double 1.1455332592119589E-5
	.quad	0x3ed5cc824132db1a              ; double 5.1972538301279162E-6
	.quad	0xbe888c8ea16efd07              ; double -1.8290468581196801E-7
	.quad	0xbe6ce58e67a63c4a              ; double -5.382430586224423E-8
	.quad	0x3e1ed0d7b94d1e27              ; double 1.793715153149277E-9
	.quad	0x3dfa2f65ac967f11              ; double 3.8104401282521396E-10
	.quad	0xbdaa05857d61e344              ; double -1.1833239178630346E-11
	.quad	0xbd80ddb4c7a5b2c2              ; double -1.9174467220108449E-12
	.quad	0x0000000000000000              ; double 0
	.quad	0xbfcdc14ea14e89f9              ; double -0.23246176601703875
	.quad	0x3f84429fef5b5fbd              ; double 0.0098927016182840336
	.quad	0x3fa367d7d608e4a3              ; double 0.037901635052955095
	.quad	0xbf59d6eb2bc49e17              ; double -0.001577119502209961
	.quad	0xbf5dc4f991b39911              ; double -0.0018169820021341524
	.quad	0x3f1315ec04d6bd38              ; double 7.2805910540142747E-5
	.quad	0x3f05718149d2ac24              ; double 4.0899999683340315E-5
	.quad	0xbeba2977f9ed10d6              ; double -1.5593759383351303E-6
	.quad	0xbea1e863d8ac307d              ; double -5.3369324013028829E-7
	.quad	0x3e54a7b7d8af34a1              ; double 1.9236606567907089E-8
	.quad	0x3e339017071cb777              ; double 4.5548312775946849E-9
	.quad	0xbde549934363d75d              ; double -1.5488624419048933E-10
	.quad	0xbdbddf663c0d3f53              ; double -2.7169020291555583E-11
	.quad	0x3d6ea9d18acb267f              ; double 8.7150492645533501E-13
	.size	__ocmltbl_M64_Y1, 2160

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_daad206124ef2e23
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
    .name:           specialx_digamma_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         specialx_digamma_kernel.kd
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
    .name:           specialx_expit_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         specialx_expit_kernel.kd
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
    .name:           specialx_logit_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         specialx_logit_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     22
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
    .name:           specialx_sinc_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         specialx_sinc_kernel.kd
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
    .name:           specialx_entr_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         specialx_entr_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     22
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
    .name:           specialx_erfinv_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         specialx_erfinv_kernel.kd
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
    .name:           specialx_erfcx_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         specialx_erfcx_kernel.kd
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
    .name:           specialx_i0_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         specialx_i0_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     22
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
    .name:           specialx_i1_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         specialx_i1_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     22
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
    .name:           specialx_i0e_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         specialx_i0e_kernel.kd
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
    .name:           specialx_i1e_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         specialx_i1e_kernel.kd
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
    .name:           specialx_j0_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         specialx_j0_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     38
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
    .name:           specialx_j1_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         specialx_j1_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     38
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
    .name:           specialx_y0_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         specialx_y0_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     44
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
    .name:           specialx_y1_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         specialx_y1_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     40
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
    .name:           specialx_ndtr_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         specialx_ndtr_kernel.kd
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
    .name:           specialx_ndtri_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     16
    .sgpr_spill_count: 0
    .symbol:         specialx_ndtri_kernel.kd
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
    .name:           specialx_sinpi_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         specialx_sinpi_kernel.kd
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
    .name:           specialx_xlogy_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         specialx_xlogy_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     24
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
