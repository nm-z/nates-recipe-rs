	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	widen_bf16_f64          ; -- Begin function widen_bf16_f64
	.globl	widen_bf16_f64
	.p2align	8
	.type	widen_bf16_f64,@function
widen_bf16_f64:                         ; @widen_bf16_f64
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
	s_cbranch_execz .LBB0_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_lshlrev_b64 v[0:1], 1, v[2:3]
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	global_load_u16 v0, v[0:1], off
	s_waitcnt vmcnt(0)
	v_lshlrev_b32_e32 v0, 16, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_f64_f32_e32 v[0:1], v0
	global_store_b64 v[2:3], v[0:1], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel widen_bf16_f64
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
.Lfunc_end0:
	.size	widen_bf16_f64, .Lfunc_end0-widen_bf16_f64
                                        ; -- End function
	.set widen_bf16_f64.num_vgpr, 4
	.set widen_bf16_f64.num_agpr, 0
	.set widen_bf16_f64.numbered_sgpr, 6
	.set widen_bf16_f64.num_named_barrier, 0
	.set widen_bf16_f64.private_seg_size, 0
	.set widen_bf16_f64.uses_vcc, 1
	.set widen_bf16_f64.uses_flat_scratch, 0
	.set widen_bf16_f64.has_dyn_sized_stack, 0
	.set widen_bf16_f64.has_recursion, 0
	.set widen_bf16_f64.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 168
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
	.protected	gqa_masked_attn         ; -- Begin function gqa_masked_attn
	.globl	gqa_masked_attn
	.p2align	8
	.type	gqa_masked_attn,@function
gqa_masked_attn:                        ; @gqa_masked_attn
; %bb.0:
	s_load_b128 s[12:15], s[0:1], 0x20
	s_abs_i32 s6, s2
	s_mov_b32 s22, 0
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s3, s13
	s_ashr_i32 s7, s13, 31
	v_cvt_f32_u32_e32 v1, s3
	s_sub_i32 s5, 0, s3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s4, v1
	s_mul_i32 s5, s5, s4
	s_mul_hi_u32 s5, s4, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	s_add_i32 s4, s4, s5
	s_ashr_i32 s5, s2, 31
	s_mul_hi_u32 s4, s6, s4
	s_xor_b32 s5, s5, s7
	s_mul_i32 s8, s4, s3
	s_sub_i32 s6, s6, s8
	s_add_i32 s8, s4, 1
	s_sub_i32 s9, s6, s3
	s_cmp_ge_u32 s6, s3
	s_cselect_b32 s4, s8, s4
	s_cselect_b32 s6, s9, s6
	s_add_i32 s8, s4, 1
	s_cmp_ge_u32 s6, s3
	s_cselect_b32 s4, s8, s4
	s_abs_i32 s6, s14
	s_xor_b32 s4, s4, s5
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s9, 0, s6
	s_sub_i32 s17, s4, s5
	s_ashr_i32 s10, s14, 31
	s_mul_i32 s5, s17, s13
	v_rcp_iflag_f32_e32 v1, v1
	s_sub_i32 s18, s2, s5
	s_xor_b32 s7, s7, s10
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s8, v1
	s_mul_i32 s9, s9, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s9, s8, s9
	s_add_i32 s8, s8, s9
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s4, s3, s8
	s_mul_i32 s8, s4, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s2, s3, s8
	s_add_i32 s3, s4, 1
	s_sub_i32 s5, s2, s6
	s_cmp_ge_u32 s2, s6
	s_cselect_b32 s3, s3, s4
	s_cselect_b32 s2, s5, s2
	s_add_i32 s4, s3, 1
	s_cmp_ge_u32 s2, s6
	s_cselect_b32 s2, s4, s3
	s_abs_i32 s6, s18
	s_xor_b32 s2, s2, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_sub_i32 s2, s2, s7
	s_abs_i32 s3, s2
	s_xor_b32 s2, s18, s2
	v_cvt_f32_u32_e32 v1, s3
	s_sub_i32 s5, 0, s3
	s_ashr_i32 s19, s2, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s4, v1
	s_mul_i32 s5, s5, s4
	s_mul_hi_u32 s5, s4, s5
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s4, s4, s5
	s_mul_hi_u32 s16, s6, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_i32 s4, s16, s3
	s_add_i32 s20, s16, 1
	s_sub_i32 s2, s6, s4
	s_load_b256 s[4:11], s[0:1], 0x0
	s_sub_i32 s21, s2, s3
	s_cmp_ge_u32 s2, s3
	s_cselect_b32 s16, s20, s16
	s_cselect_b32 s2, s21, s2
	s_add_i32 s20, s16, 1
	s_cmp_ge_u32 s2, s3
	s_mul_i32 s3, s15, s13
	s_cselect_b32 s2, s20, s16
	s_mul_i32 s16, s15, s14
	s_xor_b32 s13, s2, s19
	v_cmp_gt_i32_e64 s2, s15, v0
	s_sub_i32 s14, s13, s19
	s_cmp_lt_i32 s12, 1
	s_mul_hi_i32 s21, s17, s3
	s_cselect_b32 s23, -1, 0
	s_cmp_gt_i32 s12, 0
	s_mul_i32 s20, s17, s3
	v_cmp_eq_u32_e64 s3, 0, v0
	s_cselect_b32 s13, -1, 0
	s_mul_hi_i32 s19, s18, s15
	s_and_b32 vcc_lo, exec_lo, s13
	s_mul_i32 s18, s18, s15
	s_cbranch_vccz .LBB1_16
; %bb.1:
	v_mbcnt_lo_u32_b32 v1, -1, 0
	s_load_b32 s29, s[0:1], 0x30
	s_lshl_b64 s[24:25], s[20:21], 3
	s_mul_hi_i32 s27, s14, s15
	s_waitcnt lgkmcnt(0)
	s_add_u32 s24, s4, s24
	v_cmp_gt_u32_e32 vcc_lo, 24, v1
	s_addc_u32 s25, s5, s25
	s_lshl_b64 s[4:5], s[18:19], 3
	s_mul_i32 s26, s14, s15
	s_add_u32 s24, s24, s4
	v_cndmask_b32_e64 v2, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v1
	s_addc_u32 s25, s25, s5
	s_lshl_b64 s[4:5], s[26:27], 3
	v_and_b32_e32 v5, 31, v0
	s_ashr_i32 s26, s16, 31
	v_cndmask_b32_e64 v3, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v1
	s_add_u32 s27, s6, s4
	s_addc_u32 s28, s7, s5
	s_add_u32 s6, s0, 56
	s_addc_u32 s7, s1, 0
	v_cndmask_b32_e64 v4, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v1
	v_lshl_or_b32 v6, v1, 2, 64
	v_add_lshl_u32 v7, v2, v1, 2
	v_add_lshl_u32 v8, v3, v1, 2
	v_add_lshl_u32 v9, v4, v1, 2
	v_add_co_ci_u32_e64 v10, null, 0, v1, vcc_lo
	v_lshrrev_b32_e32 v11, 2, v0
	v_cmp_gt_u32_e64 s4, 32, v0
	v_lshlrev_b32_e32 v12, 3, v5
	s_delay_alu instid0(VALU_DEP_4)
	v_dual_mov_b32 v13, 0 :: v_dual_lshlrev_b32 v10, 2, v10
	s_cmp_lt_i32 s17, s29
	v_cmp_eq_u32_e32 vcc_lo, 0, v5
	s_cselect_b32 s29, -1, 0
	s_branch .LBB1_3
.LBB1_2:                                ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_add_i32 s22, s22, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u32 s22, s12
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB1_16
.LBB1_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB1_5 Depth 2
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_and_saveexec_b32 s30, s2
	s_cbranch_execz .LBB1_7
; %bb.4:                                ;   in Loop: Header=BB1_3 Depth=1
	s_load_b32 s5, s[6:7], 0xc
	s_mul_i32 s31, s22, s26
	s_mul_hi_u32 s33, s22, s16
	s_mul_i32 s34, s22, s16
	s_add_i32 s35, s33, s31
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_lshl_b64 s[34:35], s[34:35], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s31, s27, s34
	s_addc_u32 s33, s28, s35
	s_mov_b32 s34, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s35, s5, 0xffff
	.p2align	6
.LBB1_5:                                ;   Parent Loop BB1_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[14:15], 3, v[3:4]
	v_add_nc_u32_e32 v3, s35, v3
	v_add_co_u32 v16, s5, s24, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v17, null, s25, v15, s5
	v_add_co_u32 v14, s5, s31, v14
	v_add_co_ci_u32_e64 v15, null, s33, v15, s5
	global_load_b64 v[16:17], v[16:17], off
	global_load_b64 v[14:15], v[14:15], off
	v_cmp_le_i32_e64 s5, s15, v3
	s_or_b32 s34, s5, s34
	s_waitcnt vmcnt(0)
	v_fma_f64 v[1:2], v[16:17], v[14:15], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s34
	s_cbranch_execnz .LBB1_5
; %bb.6:                                ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s34
.LBB1_7:                                ;   in Loop: Header=BB1_3 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s30
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v10, v1
	ds_bpermute_b32 v4, v10, v2
	s_and_saveexec_b32 s5, vcc_lo
	s_cbranch_execz .LBB1_9
; %bb.8:                                ;   in Loop: Header=BB1_3 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v11, v[1:2]
.LBB1_9:                                ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s30, s4
	s_cbranch_execz .LBB1_14
; %bb.10:                               ;   in Loop: Header=BB1_3 Depth=1
	s_load_b32 s5, s[6:7], 0xc
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s31, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s5, s5, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s5, 31
	s_lshr_b32 s5, s5, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s5, v5
; %bb.11:                               ;   in Loop: Header=BB1_3 Depth=1
	ds_load_b64 v[1:2], v12
; %bb.12:                               ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s31
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_bpermute_b32 v3, v10, v1
	ds_bpermute_b32 v4, v10, v2
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB1_14
; %bb.13:                               ;   in Loop: Header=BB1_3 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v13, v[1:2]
.LBB1_14:                               ;   in Loop: Header=BB1_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s30
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v13
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s5, s3
	s_cbranch_execz .LBB1_2
; %bb.15:                               ;   in Loop: Header=BB1_3 Depth=1
	s_lshl_b32 s30, s22, 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_addk_i32 s30, 0x200
	s_cmp_gt_i32 s22, s17
	v_mov_b32_e32 v3, s30
	s_cselect_b32 s31, -1, 0
	s_and_b32 s31, s29, s31
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e64 v2, v2, 0xc6293e59, s31
	v_cndmask_b32_e64 v1, v1, 0x39a08cea, s31
	ds_store_b64 v3, v[1:2]
	s_branch .LBB1_2
.LBB1_16:
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_xor_b32 s2, s23, -1
	s_waitcnt lgkmcnt(0)
	s_mov_b32 s6, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB1_34
; %bb.17:
	s_add_i32 s4, s12, -1
	s_mov_b32 s3, 0xfe37e43c
	s_cmp_lt_u32 s4, 7
	s_mov_b32 s2, 0x8800759c
	s_cbranch_scc1 .LBB1_20
; %bb.18:
	s_and_b32 s6, s12, 0x7ffffff8
	s_mov_b32 s7, 0
	s_movk_i32 s17, 0x200
.LBB1_19:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v9, s17
	ds_load_2addr_b64 v[1:4], v9 offset1:1
	ds_load_2addr_b64 v[5:8], v9 offset0:2 offset1:3
	s_waitcnt lgkmcnt(1)
	v_readfirstlane_b32 s22, v1
	v_readfirstlane_b32 s23, v2
	v_readfirstlane_b32 s24, v3
	v_readfirstlane_b32 s25, v4
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_gt_f64_e64 s26, s[22:23], s[2:3]
	s_and_b32 s26, s26, exec_lo
	s_cselect_b32 s3, s23, s3
	s_cselect_b32 s2, s22, s2
	s_waitcnt lgkmcnt(0)
	v_readfirstlane_b32 s22, v5
	v_cmp_gt_f64_e64 s26, s[24:25], s[2:3]
	v_readfirstlane_b32 s23, v6
	s_and_b32 s26, s26, exec_lo
	s_cselect_b32 s3, s25, s3
	s_cselect_b32 s2, s24, s2
	v_readfirstlane_b32 s24, v7
	v_cmp_gt_f64_e64 s26, s[22:23], s[2:3]
	v_readfirstlane_b32 s25, v8
	ds_load_2addr_b64 v[1:4], v9 offset0:4 offset1:5
	ds_load_2addr_b64 v[5:8], v9 offset0:6 offset1:7
	s_and_b32 s26, s26, exec_lo
	s_cselect_b32 s3, s23, s3
	s_cselect_b32 s2, s22, s2
	s_waitcnt lgkmcnt(1)
	v_readfirstlane_b32 s22, v1
	v_cmp_gt_f64_e64 s26, s[24:25], s[2:3]
	v_readfirstlane_b32 s23, v2
	s_and_b32 s26, s26, exec_lo
	s_cselect_b32 s3, s25, s3
	s_cselect_b32 s2, s24, s2
	v_readfirstlane_b32 s24, v3
	v_cmp_gt_f64_e64 s26, s[22:23], s[2:3]
	v_readfirstlane_b32 s25, v4
	s_and_b32 s26, s26, exec_lo
	s_cselect_b32 s3, s23, s3
	s_cselect_b32 s2, s22, s2
	s_waitcnt lgkmcnt(0)
	v_readfirstlane_b32 s22, v5
	v_cmp_gt_f64_e64 s26, s[24:25], s[2:3]
	v_readfirstlane_b32 s23, v6
	s_and_b32 s26, s26, exec_lo
	s_cselect_b32 s3, s25, s3
	s_cselect_b32 s2, s24, s2
	v_readfirstlane_b32 s24, v7
	v_cmp_gt_f64_e64 s26, s[22:23], s[2:3]
	v_readfirstlane_b32 s25, v8
	s_and_b32 s26, s26, exec_lo
	s_cselect_b32 s3, s23, s3
	s_cselect_b32 s2, s22, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_f64_e64 s22, s[24:25], s[2:3]
	s_and_b32 s22, s22, exec_lo
	s_cselect_b32 s3, s25, s3
	s_cselect_b32 s2, s24, s2
	s_add_i32 s7, s7, 8
	s_add_i32 s17, s17, 64
	s_cmp_eq_u32 s6, s7
	s_cbranch_scc0 .LBB1_19
.LBB1_20:
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_and_b32 s7, s12, 7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s7, 0
	s_cbranch_scc1 .LBB1_23
; %bb.21:
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_lshl_b32 s2, s6, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s2, 0x200
.LBB1_22:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v1, s2
	s_add_i32 s7, s7, -1
	s_add_i32 s2, s2, 8
	s_cmp_lg_u32 s7, 0
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[1:2], v[3:4]
	v_dual_cndmask_b32 v4, v4, v2 :: v_dual_cndmask_b32 v3, v3, v1
	s_cbranch_scc1 .LBB1_22
.LBB1_23:
	s_cmp_eq_u32 s4, 0
	s_mov_b32 s17, 0
	s_cbranch_scc1 .LBB1_46
; %bb.24:
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s22, 0xfefa39ef
	s_mov_b32 s24, 0x3b39803f
	s_mov_b32 s26, 0xfca7ab0c
	s_mov_b32 s28, 0x6a5dcb37
	s_mov_b32 s30, 0x623fde64
	s_mov_b32 s34, 0x7c89e6b0
	s_mov_b32 s36, 0x14761f6e
	s_mov_b32 s38, 0x1852b7b0
	s_mov_b32 s40, 0x11122322
	s_mov_b32 s42, 0x555502a1
	s_mov_b32 s44, 0x55555511
	s_mov_b32 s46, 11
	s_and_b32 s17, s12, 0x7ffffffe
	s_mov_b32 s33, 0
	s_movk_i32 s48, 0x200
	s_mov_b32 s7, 0x3ff71547
	s_mov_b32 s23, 0xbfe62e42
	s_mov_b32 s25, 0xbc7abc9e
	s_mov_b32 s27, 0x3e928af3
	s_mov_b32 s29, 0x3e5ade15
	s_mov_b32 s31, 0x3ec71dee
	s_mov_b32 s35, 0x3efa0199
	s_mov_b32 s37, 0x3f2a01a0
	s_mov_b32 s39, 0x3f56c16c
	s_mov_b32 s41, 0x3f811111
	s_mov_b32 s43, 0x3fa55555
	s_mov_b32 s45, 0x3fc55555
	s_mov_b32 s47, 0x3fe00000
.LBB1_25:                               ; =>This Inner Loop Header: Depth=1
	v_mov_b32_e32 v21, s48
	s_add_i32 s33, s33, 2
	s_add_i32 s48, s48, 16
	ds_load_2addr_b64 v[5:8], v21 offset1:1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[5:6], v[5:6], -v[3:4]
	v_add_f64 v[7:8], v[7:8], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[9:10], v[5:6], s[6:7]
	v_mul_f64 v[11:12], v[7:8], s[6:7]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[5:6]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[5:6]
	v_cmp_nlt_f64_e64 s3, 0x40900000, v[7:8]
	v_cmp_ngt_f64_e64 s4, 0xc090cc00, v[7:8]
	v_rndne_f64_e32 v[9:10], v[9:10]
	v_rndne_f64_e32 v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[9:10], s[22:23], v[5:6]
	v_fma_f64 v[15:16], v[11:12], s[22:23], v[7:8]
	v_cvt_i32_f64_e32 v22, v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], s[24:25], v[13:14]
	v_fma_f64 v[15:16], v[11:12], s[24:25], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], s[28:29], s[26:27]
	v_fma_f64 v[19:20], v[15:16], s[28:29], s[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[30:31]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[34:35]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[36:37]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[38:39]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[38:39]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[40:41]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[40:41]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[42:43]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[42:43]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[44:45]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[44:45]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[46:47]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[46:47]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], 1.0
	v_fma_f64 v[9:10], v[15:16], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[13:14], v[17:18], 1.0
	v_cvt_i32_f64_e32 v17, v[11:12]
	v_fma_f64 v[9:10], v[15:16], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[11:12], v[13:14], v22
	v_ldexp_f64 v[9:10], v[9:10], v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v12, 0x7ff00000, v12, vcc_lo
	s_and_b32 vcc_lo, s2, vcc_lo
	v_cndmask_b32_e64 v10, 0x7ff00000, v10, s3
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e32 v5, 0, v11, vcc_lo
	s_and_b32 vcc_lo, s4, s3
	v_cndmask_b32_e64 v6, 0, v12, s2
	v_cndmask_b32_e32 v7, 0, v9, vcc_lo
	v_cndmask_b32_e64 v8, 0, v10, s4
	s_cmp_lg_u32 s17, s33
	s_delay_alu instid0(VALU_DEP_3)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_store_2addr_b64 v21, v[5:6], v[7:8] offset1:1
	v_add_f64 v[1:2], v[1:2], v[7:8]
	s_cbranch_scc1 .LBB1_25
; %bb.26:
	s_bitcmp0_b32 s12, 0
	s_mov_b32 s6, 0
	s_cbranch_scc1 .LBB1_28
.LBB1_27:
	s_lshl_b32 s2, s17, 3
	s_mov_b32 s22, 0x6a5dcb37
	s_addk_i32 s2, 0x200
	s_mov_b32 s23, 0x3e5ade15
	v_mov_b32_e32 v11, s2
	s_mov_b32 s2, 0x652b82fe
	s_mov_b32 s3, 0x3ff71547
	ds_load_b64 v[5:6], v11
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[3:4], v[5:6], -v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[5:6], v[3:4], s[2:3]
	s_mov_b32 s2, 0xfefa39ef
	s_mov_b32 s3, 0xbfe62e42
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[3:4]
	v_rndne_f64_e32 v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[5:6], s[2:3], v[3:4]
	s_mov_b32 s2, 0x3b39803f
	s_mov_b32 s3, 0xbc7abc9e
	v_cvt_i32_f64_e32 v12, v[5:6]
	v_fma_f64 v[7:8], v[5:6], s[2:3], v[7:8]
	s_mov_b32 s2, 0xfca7ab0c
	s_mov_b32 s3, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], s[22:23], s[2:3]
	s_mov_b32 s2, 0x623fde64
	s_mov_b32 s3, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x7c89e6b0
	s_mov_b32 s3, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x14761f6e
	s_mov_b32 s3, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x11122322
	s_mov_b32 s3, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x555502a1
	s_mov_b32 s3, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 0x55555511
	s_mov_b32 s3, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	s_mov_b32 s2, 11
	s_mov_b32 s3, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[2:3]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], 1.0
	v_fma_f64 v[5:6], v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v12
	v_cndmask_b32_e32 v6, 0x7ff00000, v6, vcc_lo
	s_and_b32 vcc_lo, s2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v3, 0, v5, vcc_lo
	v_cndmask_b32_e64 v4, 0, v6, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v11, v[3:4]
.LBB1_28:
	s_cmp_lt_u32 s12, 4
	s_cbranch_scc1 .LBB1_31
; %bb.29:
	s_and_b32 s6, s12, 0x7ffffffc
	s_mov_b32 s7, 0
	s_movk_i32 s17, 0x200
.LBB1_30:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v43, s17
	s_add_i32 s7, s7, 4
	s_add_i32 s17, s17, 32
	s_cmp_lg_u32 s6, s7
	ds_load_2addr_b64 v[3:6], v43 offset1:1
	ds_load_2addr_b64 v[7:10], v43 offset0:2 offset1:3
	s_waitcnt lgkmcnt(1)
	v_div_scale_f64 v[11:12], null, v[1:2], v[1:2], v[3:4]
	v_div_scale_f64 v[13:14], null, v[1:2], v[1:2], v[5:6]
	s_waitcnt lgkmcnt(0)
	v_div_scale_f64 v[15:16], null, v[1:2], v[1:2], v[7:8]
	v_div_scale_f64 v[17:18], null, v[1:2], v[1:2], v[9:10]
	v_div_scale_f64 v[35:36], vcc_lo, v[3:4], v[1:2], v[3:4]
	v_div_scale_f64 v[37:38], s2, v[5:6], v[1:2], v[5:6]
	v_div_scale_f64 v[39:40], s3, v[7:8], v[1:2], v[7:8]
	v_rcp_f64_e32 v[19:20], v[11:12]
	v_rcp_f64_e32 v[21:22], v[13:14]
	v_rcp_f64_e32 v[23:24], v[15:16]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(TRANS32_DEP_3)
	v_rcp_f64_e32 v[25:26], v[17:18]
	v_fma_f64 v[27:28], -v[11:12], v[19:20], 1.0
	v_fma_f64 v[29:30], -v[13:14], v[21:22], 1.0
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[31:32], -v[15:16], v[23:24], 1.0
	v_fma_f64 v[33:34], -v[17:18], v[25:26], 1.0
	v_fma_f64 v[19:20], v[19:20], v[27:28], v[19:20]
	v_fma_f64 v[21:22], v[21:22], v[29:30], v[21:22]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[23:24], v[23:24], v[31:32], v[23:24]
	v_fma_f64 v[25:26], v[25:26], v[33:34], v[25:26]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[27:28], -v[11:12], v[19:20], 1.0
	v_fma_f64 v[29:30], -v[13:14], v[21:22], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[31:32], -v[15:16], v[23:24], 1.0
	v_fma_f64 v[33:34], -v[17:18], v[25:26], 1.0
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f64 v[19:20], v[19:20], v[27:28], v[19:20]
	v_div_scale_f64 v[27:28], s4, v[9:10], v[1:2], v[9:10]
	v_fma_f64 v[21:22], v[21:22], v[29:30], v[21:22]
	v_fma_f64 v[23:24], v[23:24], v[31:32], v[23:24]
	v_fma_f64 v[25:26], v[25:26], v[33:34], v[25:26]
	v_mul_f64 v[29:30], v[35:36], v[19:20]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[31:32], v[37:38], v[21:22]
	v_mul_f64 v[33:34], v[39:40], v[23:24]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_mul_f64 v[41:42], v[27:28], v[25:26]
	v_fma_f64 v[11:12], -v[11:12], v[29:30], v[35:36]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[13:14], -v[13:14], v[31:32], v[37:38]
	v_fma_f64 v[15:16], -v[15:16], v[33:34], v[39:40]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[17:18], -v[17:18], v[41:42], v[27:28]
	v_div_fmas_f64 v[11:12], v[11:12], v[19:20], v[29:30]
	s_mov_b32 vcc_lo, s2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_div_fmas_f64 v[13:14], v[13:14], v[21:22], v[31:32]
	s_mov_b32 vcc_lo, s3
	v_div_fmas_f64 v[15:16], v[15:16], v[23:24], v[33:34]
	s_mov_b32 vcc_lo, s4
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_div_fmas_f64 v[17:18], v[17:18], v[25:26], v[41:42]
	v_div_fixup_f64 v[3:4], v[11:12], v[1:2], v[3:4]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_div_fixup_f64 v[5:6], v[13:14], v[1:2], v[5:6]
	v_div_fixup_f64 v[7:8], v[15:16], v[1:2], v[7:8]
	s_delay_alu instid0(VALU_DEP_4)
	v_div_fixup_f64 v[9:10], v[17:18], v[1:2], v[9:10]
	ds_store_2addr_b64 v43, v[3:4], v[5:6] offset1:1
	ds_store_2addr_b64 v43, v[7:8], v[9:10] offset0:2 offset1:3
	s_cbranch_scc1 .LBB1_30
.LBB1_31:
	s_and_b32 s2, s12, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s2, 0
	s_cbranch_scc1 .LBB1_34
; %bb.32:
	s_lshl_b32 s3, s6, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s3, 0x200
	.p2align	6
.LBB1_33:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v13, s3
	s_add_i32 s2, s2, -1
	s_add_i32 s3, s3, 8
	s_cmp_lg_u32 s2, 0
	ds_load_b64 v[3:4], v13
	s_waitcnt lgkmcnt(0)
	v_div_scale_f64 v[5:6], null, v[1:2], v[1:2], v[3:4]
	v_div_scale_f64 v[11:12], vcc_lo, v[3:4], v[1:2], v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	v_fma_f64 v[5:6], -v[5:6], v[9:10], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[9:10]
	v_div_fixup_f64 v[3:4], v[5:6], v[1:2], v[3:4]
	ds_store_b64 v13, v[3:4]
	s_cbranch_scc1 .LBB1_33
.LBB1_34:
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s15, v0
	s_cbranch_execz .LBB1_45
; %bb.35:
	s_load_b32 s0, s[0:1], 0x44
	s_lshl_b64 s[2:3], s[20:21], 3
	s_mul_hi_i32 s5, s15, s14
	s_add_u32 s6, s10, s2
	s_addc_u32 s7, s11, s3
	s_lshl_b64 s[2:3], s[18:19], 3
	s_mul_i32 s4, s15, s14
	s_add_u32 s10, s6, s2
	s_addc_u32 s11, s7, s3
	s_ashr_i32 s17, s16, 31
	s_and_b32 s14, s12, 3
	s_mov_b32 s1, 0
	s_mul_hi_i32 s20, s16, 24
	s_mul_i32 s21, s16, 24
	s_waitcnt lgkmcnt(0)
	s_and_b32 s18, s0, 0xffff
	s_cmp_gt_u32 s12, 3
	s_cselect_b32 s19, -1, 0
	s_and_b32 s12, s12, 0x7ffffffc
	s_cmp_lg_u32 s14, 0
	s_cselect_b32 s22, -1, 0
	s_lshl_b64 s[2:3], s[4:5], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s8, s8, s2
	s_addc_u32 s9, s9, s3
	s_lshl_b64 s[2:3], s[16:17], 5
	s_lshl_b64 s[4:5], s[16:17], 4
	s_lshl_b64 s[6:7], s[16:17], 3
	s_branch .LBB1_37
.LBB1_36:                               ;   in Loop: Header=BB1_37 Depth=1
	v_add_nc_u32_e32 v0, s18, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, s0, s10, v1
	v_add_co_ci_u32_e64 v2, null, s11, v2, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s15, v0
	global_store_b64 v[1:2], v[3:4], off
	s_or_b32 s1, vcc_lo, s1
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB1_45
.LBB1_37:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB1_40 Depth 2
                                        ;     Child Loop BB1_44 Depth 2
	v_ashrrev_i32_e32 v1, 31, v0
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_and_not1_b32 vcc_lo, exec_lo, s13
	s_delay_alu instid0(VALU_DEP_3)
	v_lshlrev_b64 v[1:2], 3, v[0:1]
	s_cbranch_vccnz .LBB1_36
; %bb.38:                               ;   in Loop: Header=BB1_37 Depth=1
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_and_not1_b32 vcc_lo, exec_lo, s19
	s_cbranch_vccnz .LBB1_42
; %bb.39:                               ;   in Loop: Header=BB1_37 Depth=1
	v_add_co_u32 v5, vcc_lo, s8, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s9, v2, vcc_lo
	s_mov_b32 s0, 0
	s_movk_i32 s16, 0x200
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB1_40:                               ;   Parent Loop BB1_37 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[15:16], v[5:6], off
	v_add_co_u32 v7, vcc_lo, v5, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s7, v6, vcc_lo
	v_mov_b32_e32 v11, s16
	s_add_i32 s0, s0, 4
	s_add_i32 s16, s16, 32
	global_load_b64 v[17:18], v[7:8], off
	v_add_co_u32 v7, vcc_lo, v5, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s5, v6, vcc_lo
	s_cmp_eq_u32 s12, s0
	global_load_b64 v[19:20], v[7:8], off
	v_add_co_u32 v7, vcc_lo, v5, s21
	v_add_co_ci_u32_e64 v8, null, s20, v6, vcc_lo
	v_add_co_u32 v5, vcc_lo, v5, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s3, v6, vcc_lo
	global_load_b64 v[21:22], v[7:8], off
	ds_load_2addr_b64 v[7:10], v11 offset1:1
	ds_load_2addr_b64 v[11:14], v11 offset0:2 offset1:3
	s_waitcnt vmcnt(3) lgkmcnt(1)
	v_fma_f64 v[3:4], v[7:8], v[15:16], v[3:4]
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[9:10], v[17:18], v[3:4]
	s_waitcnt vmcnt(1) lgkmcnt(0)
	v_fma_f64 v[3:4], v[11:12], v[19:20], v[3:4]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[3:4], v[13:14], v[21:22], v[3:4]
	s_cbranch_scc0 .LBB1_40
; %bb.41:                               ;   in Loop: Header=BB1_37 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s16, s12
	s_and_not1_b32 vcc_lo, exec_lo, s22
	s_cbranch_vccz .LBB1_43
	s_branch .LBB1_36
.LBB1_42:                               ;   in Loop: Header=BB1_37 Depth=1
	s_mov_b32 s16, 0
	s_and_not1_b32 vcc_lo, exec_lo, s22
	s_cbranch_vccnz .LBB1_36
.LBB1_43:                               ;   in Loop: Header=BB1_37 Depth=1
	s_lshl_b32 s0, s16, 3
	s_mul_i32 s17, s7, s16
	s_mul_hi_u32 s23, s6, s16
	s_mul_i32 s16, s6, s16
	s_addk_i32 s0, 0x200
	s_add_i32 s23, s23, s17
	s_add_u32 s16, s8, s16
	s_addc_u32 s17, s9, s23
	v_add_co_u32 v5, vcc_lo, s16, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s17, v2, vcc_lo
	s_mov_b32 s16, s14
.LBB1_44:                               ;   Parent Loop BB1_37 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[7:8], v[5:6], off
	v_mov_b32_e32 v9, s0
	v_add_co_u32 v5, vcc_lo, v5, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s7, v6, vcc_lo
	ds_load_b64 v[9:10], v9
	s_add_i32 s16, s16, -1
	s_add_i32 s0, s0, 8
	s_cmp_lg_u32 s16, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[3:4], v[9:10], v[7:8], v[3:4]
	s_cbranch_scc1 .LBB1_44
	s_branch .LBB1_36
.LBB1_45:
	s_endpgm
.LBB1_46:
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_bitcmp0_b32 s12, 0
	s_mov_b32 s6, 0
	s_cbranch_scc0 .LBB1_27
	s_branch .LBB1_28
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gqa_masked_attn
		.amdhsa_group_segment_fixed_size 512
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
		.amdhsa_next_free_vgpr 44
		.amdhsa_next_free_sgpr 49
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
		.amdhsa_inst_pref_size 36
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
	.size	gqa_masked_attn, .Lfunc_end1-gqa_masked_attn
                                        ; -- End function
	.set gqa_masked_attn.num_vgpr, 44
	.set gqa_masked_attn.num_agpr, 0
	.set gqa_masked_attn.numbered_sgpr, 49
	.set gqa_masked_attn.num_named_barrier, 0
	.set gqa_masked_attn.private_seg_size, 0
	.set gqa_masked_attn.uses_vcc, 1
	.set gqa_masked_attn.uses_flat_scratch, 0
	.set gqa_masked_attn.has_dyn_sized_stack, 0
	.set gqa_masked_attn.has_recursion, 0
	.set gqa_masked_attn.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4504
; TotalNumSgprs: 51
; NumVgprs: 44
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 512 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 51
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
	.protected	rope_partial            ; -- Begin function rope_partial
	.globl	rope_partial
	.p2align	8
	.type	rope_partial,@function
rope_partial:                           ; @rope_partial
; %bb.0:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x8
	s_load_b32 s3, s[0:1], 0x2c
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v4
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s4, s10, 31
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_add_i32 s2, s10, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s14, s2, 1
	s_mul_hi_i32 s3, s14, s8
	s_mul_i32 s2, s14, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[2:3]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB2_14
; %bb.1:
	s_load_b64 s[2:3], s[0:1], 0x18
                                        ; implicit-def: $vgpr0_vgpr1
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[4:5], s[2:3], 0x0
	s_ashr_i32 s3, s14, 31
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s3, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s8, exec_lo, s2
	s_cbranch_execz .LBB2_3
; %bb.2:
	s_ashr_i32 s6, s3, 31
	v_ashrrev_i32_e32 v8, 31, v3
	s_add_u32 s12, s14, s6
	s_mov_b32 s7, s6
	s_addc_u32 s13, s3, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[12:13], s[12:13], s[6:7]
	v_add_co_u32 v4, vcc_lo, v2, v8
	v_cvt_f32_u32_e32 v0, s12
	v_cvt_f32_u32_e32 v1, s13
	s_sub_u32 s15, 0, s12
	s_subb_u32 s16, 0, s13
	v_add_co_ci_u32_e64 v5, null, v3, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v4, v8
	v_xor_b32_e32 v10, v5, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x5f7ffffc, v0
	v_mul_f32_e32 v1, 0x2f800000, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v1, v1
	v_fmamk_f32 v0, v1, 0xcf800000, v0
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s2, v1
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v0
	s_mul_i32 s17, s15, s2
	s_mul_hi_u32 s19, s15, s7
	s_mul_i32 s18, s16, s7
	s_add_i32 s17, s19, s17
	s_mul_i32 s20, s15, s7
	s_add_i32 s17, s17, s18
	s_mul_hi_u32 s19, s7, s20
	s_mul_i32 s22, s7, s17
	s_mul_hi_u32 s21, s2, s20
	s_mul_i32 s18, s2, s20
	s_mul_hi_u32 s20, s7, s17
	s_add_u32 s19, s19, s22
	s_addc_u32 s20, 0, s20
	s_mul_hi_u32 s23, s2, s17
	s_add_u32 s18, s19, s18
	s_mul_i32 s17, s2, s17
	s_addc_u32 s18, s20, s21
	s_addc_u32 s19, s23, 0
	s_add_u32 s17, s18, s17
	s_addc_u32 s18, 0, s19
	s_add_u32 s7, s7, s17
	s_cselect_b32 s17, -1, 0
	s_mul_hi_u32 s19, s15, s7
	s_cmp_lg_u32 s17, 0
	s_mul_i32 s17, s15, s7
	s_addc_u32 s2, s2, s18
	s_mul_i32 s16, s16, s7
	s_mul_i32 s15, s15, s2
	s_mul_hi_u32 s18, s7, s17
	s_add_i32 s15, s19, s15
	s_mul_hi_u32 s19, s2, s17
	s_add_i32 s15, s15, s16
	s_mul_i32 s16, s2, s17
	s_mul_i32 s21, s7, s15
	s_mul_hi_u32 s20, s7, s15
	s_add_u32 s18, s18, s21
	s_addc_u32 s20, 0, s20
	s_mul_hi_u32 s17, s2, s15
	s_add_u32 s16, s18, s16
	s_mul_i32 s15, s2, s15
	s_addc_u32 s16, s20, s19
	s_addc_u32 s17, s17, 0
	s_add_u32 s15, s16, s15
	s_addc_u32 s16, 0, s17
	s_add_u32 s7, s7, s15
	s_cselect_b32 s15, -1, 0
	v_mul_hi_u32 v11, v9, s7
	s_cmp_lg_u32 s15, 0
	v_mad_u64_u32 v[4:5], null, v10, s7, 0
	s_addc_u32 s2, s2, s16
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[6:7], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v5, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, 0, v1, vcc_lo
	v_mul_lo_u32 v6, s13, v4
	v_mad_u64_u32 v[0:1], null, s12, v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v7, s12, v5
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v7, v6
	v_add_co_u32 v7, s2, v4, 2
	v_add_co_ci_u32_e64 v9, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v6, v10, v1
	v_sub_co_u32 v11, s2, v0, s12
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v6, null, s13, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s12, v11
	v_subrev_co_ci_u32_e64 v6, null, 0, v6, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s13, v6
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s12, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s13, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s13, v6
	v_cndmask_b32_e32 v6, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v4, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v11, null, 0, v5, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s13, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v6
	v_xor_b32_e32 v6, s6, v8
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v7 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v4, v0, s2
	v_cndmask_b32_e64 v1, v5, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v6
	v_xor_b32_e32 v1, v1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v6
	v_sub_co_ci_u32_e64 v1, null, v1, v6, vcc_lo
.LBB2_3:
	s_and_not1_saveexec_b32 s2, s8
	s_cbranch_execz .LBB2_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s14
	s_sub_i32 s6, 0, s14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s6, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s14
	v_add_nc_u32_e32 v4, 1, v0
	v_sub_nc_u32_e32 v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s14, v1
	v_cmp_le_u32_e32 vcc_lo, s14, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s14, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
.LBB2_5:
	s_or_b32 exec_lo, exec_lo, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v1, s14
	v_mul_lo_u32 v6, v0, s3
	v_mad_u64_u32 v[4:5], null, v0, s14, 0
	s_mov_b32 s3, 0x3fe55555
	s_mov_b32 s2, 0x55555555
	s_mov_b32 s6, 0x4222de17
	s_mov_b32 s7, 0x3fbdee67
	v_add3_u32 v5, v5, v6, v1
	v_sub_co_u32 v1, vcc_lo, v2, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_co_ci_u32_e64 v2, null, v3, v5, vcc_lo
	v_cvt_f64_u32_e32 v[5:6], v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_u32_e32 v[3:4], v2
	v_ldexp_f64 v[3:4], v[3:4], 32
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_cvt_f64_i32_e32 v[5:6], s10
	v_mul_f64 v[3:4], v[3:4], -2.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[7:8], null, v[5:6], v[5:6], v[3:4]
	v_div_scale_f64 v[13:14], vcc_lo, v[3:4], v[5:6], v[3:4]
	v_rcp_f64_e32 v[9:10], v[7:8]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[9:10], v[11:12], v[9:10]
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], -v[7:8], v[11:12], v[13:14]
	v_div_fmas_f64 v[7:8], v[7:8], v[9:10], v[11:12]
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f64_e64 vcc_lo, s[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_fixup_f64 v[3:4], v[7:8], v[5:6], v[3:4]
	v_mov_b32_e32 v5, s5
	v_cndmask_b32_e32 v4, 0x3ff00000, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[3:4]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, 0x3ff00000, v5, vcc_lo
	v_cndmask_b32_e64 v5, 0, s4, vcc_lo
	s_mov_b32 s4, 0x968915a9
	s_mov_b32 s5, 0x3fba6564
	v_frexp_mant_f64_e64 v[7:8], |v[5:6]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[7:8]
	v_cndmask_b32_e64 v9, 0, 1, vcc_lo
	v_ldexp_f64 v[7:8], v[7:8], v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[7:8], 1.0
	v_add_f64 v[15:16], v[7:8], -1.0
	v_rcp_f64_e32 v[11:12], v[9:10]
	v_add_f64 v[17:18], v[9:10], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[13:14], v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[13:14], v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[13:14], v[15:16], v[11:12]
	v_mul_f64 v[19:20], v[9:10], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[13:14], v[9:10], -v[19:20]
	v_fma_f64 v[7:8], v[13:14], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[19:20], v[7:8]
	v_add_f64 v[17:18], v[15:16], -v[9:10]
	v_add_f64 v[19:20], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	v_add_f64 v[7:8], v[19:20], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[17:18], v[7:8]
	v_mul_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[13:14], v[7:8]
	v_add_f64 v[11:12], v[9:10], -v[13:14]
	v_mul_f64 v[13:14], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	v_fma_f64 v[11:12], v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[7:8], v[7:8]
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[15:16], v[13:14], v[11:12]
	v_fma_f64 v[17:18], v[15:16], s[6:7], s[4:5]
	s_mov_b32 s4, 0x3abe935a
	s_mov_b32 s5, 0x3fbe25e4
	v_add_f64 v[13:14], v[15:16], -v[13:14]
	v_mul_f64 v[23:24], v[9:10], v[15:16]
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x47e6c9c2
	s_mov_b32 s5, 0x3fc110ef
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0xcfa74449
	s_mov_b32 s5, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x71bf3c30
	s_mov_b32 s5, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x1c7792ce
	s_mov_b32 s5, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x924920da
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s4, 0x9999999c
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[4:5]
	s_mov_b32 s5, 0x3c7abc9e
	s_mov_b32 s4, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[19:20], v[15:16], v[17:18]
	v_fma_f64 v[13:14], v[15:16], v[17:18], -v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], v[11:12], v[17:18], v[13:14]
	v_add_f64 v[17:18], v[19:20], v[13:14]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[21:22], v[17:18], s[2:3]
	v_add_f64 v[19:20], v[17:18], -v[19:20]
	s_mov_b32 s3, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[25:26], v[21:22], s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	v_fma_f64 v[19:20], v[15:16], v[9:10], -v[23:24]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_add_f64 v[17:18], v[17:18], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], s[2:3]
	v_fma_f64 v[15:16], v[15:16], v[7:8], v[19:20]
	s_mov_b32 s3, 0x3fe62e42
	s_mov_b32 s2, 0xfefa39ef
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], v[17:18]
	v_fma_f64 v[11:12], v[11:12], v[9:10], v[15:16]
	v_ldexp_f64 v[9:10], v[9:10], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[21:22], v[13:14]
	v_add_f64 v[17:18], v[23:24], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[19:20], v[21:22], -v[15:16]
	v_mul_f64 v[21:22], v[17:18], v[15:16]
	v_add_f64 v[23:24], v[17:18], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], v[19:20]
	v_fma_f64 v[19:20], v[17:18], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[23:24]
	v_fma_f64 v[13:14], v[17:18], v[13:14], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[11:12], v[11:12], v[15:16], v[13:14]
	v_frexp_exp_i32_f64_e32 v15, v[5:6]
	v_add_f64 v[13:14], v[21:22], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v15, null, 0, v15, vcc_lo
	v_cvt_f64_i32_e32 v[15:16], v15
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[9:10], v[13:14]
	v_add_f64 v[19:20], v[13:14], -v[21:22]
	v_mul_f64 v[21:22], v[15:16], s[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[17:18], -v[9:10]
	v_add_f64 v[11:12], v[11:12], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[19:20], v[15:16], s[2:3], -v[21:22]
	s_mov_b32 s3, 0xbfe62e42
	v_add_f64 v[9:10], v[13:14], -v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[7:8], v[11:12]
	v_fma_f64 v[11:12], v[15:16], s[4:5], v[19:20]
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_add_f64 v[9:10], v[21:22], v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[17:18], v[7:8]
	v_add_f64 v[21:22], v[9:10], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[9:10], v[13:14]
	v_add_f64 v[17:18], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], v[15:16], -v[19:20]
	v_add_f64 v[13:14], v[13:14], -v[19:20]
	v_add_f64 v[17:18], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[9:10], -v[23:24]
	v_add_f64 v[9:10], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[17:18], -v[11:12]
	v_add_f64 v[9:10], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[17:18], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[19:20], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[13:14], v[19:20], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], v[11:12]
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], v[9:10]
	v_add_f64 v[9:10], v[19:20], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[9:10], -v[19:20]
	v_mul_f64 v[13:14], v[3:4], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[3:4], v[9:10], -v[13:14]
	v_cmp_class_f64_e64 vcc_lo, v[13:14], 0x204
	v_fma_f64 v[7:8], v[3:4], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[13:14], v[7:8]
	v_dual_cndmask_b32 v12, v10, v14 :: v_dual_cndmask_b32 v11, v9, v13
	v_add_f64 v[9:10], v[9:10], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[15:16], v[11:12], s[6:7]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[11:12]|
	s_abs_i32 s6, s11
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	v_trunc_f64_e32 v[9:10], v[3:4]
	v_rndne_f64_e32 v[15:16], v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v8, 0, v8 :: v_dual_cndmask_b32 v7, 0, v7
	v_fma_f64 v[17:18], v[15:16], s[2:3], v[11:12]
	s_mov_b32 s2, 0xfca7ab0c
	s_mov_b32 s3, 0x3e928af3
	v_cvt_i32_f64_e32 v21, v[15:16]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[17:18], v[15:16], s[4:5], v[17:18]
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], s[4:5], s[2:3]
	s_mov_b32 s2, 0x623fde64
	s_mov_b32 s3, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x7c89e6b0
	s_mov_b32 s3, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x14761f6e
	s_mov_b32 s3, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x11122322
	s_mov_b32 s3, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x555502a1
	s_mov_b32 s3, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 0x55555511
	s_mov_b32 s3, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	s_mov_b32 s2, 11
	s_mov_b32 s3, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], s[2:3]
	v_cmp_nlt_f64_e64 s2, 0x40900000, v[11:12]
	v_cmp_ngt_f64_e64 s3, 0xc090cc00, v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[19:20], v[17:18], v[19:20], 1.0
	s_and_b32 vcc_lo, s3, s2
	v_fma_f64 v[15:16], v[17:18], v[19:20], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[13:14], v[15:16], v21
	v_mul_f64 v[15:16], v[3:4], 0.5
                                        ; implicit-def: $vgpr21
	v_cndmask_b32_e64 v14, 0x7ff00000, v14, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[11:12], v[15:16]
	v_cndmask_b32_e32 v13, 0, v13, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[9:10], v[3:4]
	v_cvt_f32_u32_e32 v9, s6
	v_cndmask_b32_e64 v14, 0, v14, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v9, v9
	v_fma_f64 v[7:8], v[13:14], v[7:8], v[13:14]
	v_cmp_class_f64_e64 s3, v[13:14], 0x204
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v9, 0x4f7ffffe, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_cvt_u32_f32_e32 v9, v9
	v_cmp_neq_f64_e64 s2, v[11:12], v[15:16]
	v_cndmask_b32_e64 v7, v7, v13, s3
	v_cndmask_b32_e64 v8, v8, v14, s3
	s_sub_i32 s3, 0, s6
	v_sub_nc_u32_e32 v13, 0, v0
	v_mul_lo_u32 v12, s3, v9
	v_cndmask_b32_e32 v11, 0, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_max_i32_e32 v13, v0, v13
	v_mul_hi_u32 v12, v9, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v9, v9, v12
	v_cmp_lt_f64_e64 s7, |v[5:6]|, 1.0
	v_cmp_eq_f64_e64 s4, 0, v[5:6]
	v_mul_hi_u32 v9, v13, v9
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v10, 0x3ff00000, v6, s2
	v_mul_lo_u32 v14, v9, s6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_bfi_b32 v8, 0x7fffffff, v8, v10
	v_cndmask_b32_e32 v10, 0x7ff80000, v8, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[5:6]
	v_cndmask_b32_e32 v7, v7, v11, vcc_lo
	v_add_nc_u32_e32 v11, 1, v9
	v_cmp_neq_f64_e64 s5, v[3:4], |v[3:4]|
	v_cmp_gt_f64_e64 s3, 0, v[3:4]
	v_cndmask_b32_e32 v8, v8, v10, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[3:4], 0x204
	v_sub_nc_u32_e32 v10, v13, v14
	s_delay_alu instid0(VALU_DEP_1)
	v_subrev_nc_u32_e32 v13, s6, v10
	s_xor_b32 s5, s5, s7
	v_cmp_class_f64_e64 s7, v[5:6], 0x204
	v_cndmask_b32_e64 v12, 0x7ff00000, 0, s5
	v_cmp_neq_f64_e64 s5, |v[5:6]|, 1.0
	s_xor_b32 s3, s3, s4
	v_cndmask_b32_e64 v12, 0x3ff00000, v12, s5
	v_cmp_le_u32_e64 s5, s6, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v8, v8, v12, vcc_lo
	v_cndmask_b32_e64 v9, v9, v11, s5
	v_cndmask_b32_e64 v10, v10, v13, s5
	v_xor_b32_e32 v11, s11, v0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v13, 1, v9
	v_cmp_le_u32_e64 s5, s6, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_ashrrev_i32_e32 v11, 31, v11
	v_cndmask_b32_e64 v10, 0x7ff00000, 0, s3
	v_cndmask_b32_e64 v9, v9, v13, s5
	v_cndmask_b32_e64 v13, 0, v6, s2
	s_or_b32 s2, s4, s7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v9, v9, v11
	v_bfi_b32 v10, 0x7fffffff, v10, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v9, v9, v11
	v_cndmask_b32_e64 v8, v8, v10, s2
	s_or_b32 s2, s2, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[5:6], v[3:4]
	v_cndmask_b32_e64 v7, v7, 0, s2
	v_cvt_f64_i32_e32 v[3:4], v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0, v7, vcc_lo
	v_cndmask_b32_e32 v6, 0x7ff80000, v8, vcc_lo
                                        ; implicit-def: $vgpr7_vgpr8
	v_mul_f64 v[3:4], v[5:6], v[3:4]
                                        ; implicit-def: $vgpr5_vgpr6
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_ngt_f64_e64 s2, 0x41d00000, |v[3:4]|
	v_trig_preop_f64 v[17:18], |v[3:4]|, 0
	v_trig_preop_f64 v[15:16], |v[3:4]|, 1
	v_ldexp_f64 v[19:20], |v[3:4]|, 0xffffff80
	v_trig_preop_f64 v[9:10], |v[3:4]|, 2
	v_and_b32_e32 v23, 0x7fffffff, v4
	s_and_saveexec_b32 s3, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s3, exec_lo, s3
	s_cbranch_execz .LBB2_7
; %bb.6:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[3:4]|
	v_mov_b32_e32 v34, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_cndmask_b32_e32 v6, v23, v20, vcc_lo
	v_cndmask_b32_e32 v5, v3, v19, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[7:8], v[17:18], v[5:6]
	v_mul_f64 v[11:12], v[15:16], v[5:6]
	v_fma_f64 v[13:14], v[17:18], v[5:6], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[32:33], v[15:16], v[5:6], -v[11:12]
	v_add_f64 v[21:22], v[11:12], v[13:14]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[21:22], -v[11:12]
	v_add_f64 v[28:29], v[7:8], v[21:22]
	v_add_f64 v[26:27], v[21:22], -v[24:25]
	v_add_f64 v[13:14], v[13:14], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[24:25], v[28:29], -2
	v_add_f64 v[7:8], v[28:29], -v[7:8]
	v_add_f64 v[11:12], v[11:12], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[24:25]|
	v_add_f64 v[7:8], v[21:22], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[13:14], v[11:12]
	v_fract_f64_e32 v[13:14], v[24:25]
	v_ldexp_f64 v[13:14], v[13:14], 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v14, 0, v14 :: v_dual_cndmask_b32 v13, 0, v13
	v_mul_f64 v[30:31], v[9:10], v[5:6]
	v_add_f64 v[26:27], v[30:31], v[32:33]
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[21:22], v[26:27], v[11:12]
	v_add_f64 v[24:25], v[7:8], v[21:22]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[28:29], v[24:25], v[13:14]
	v_add_f64 v[7:8], v[24:25], -v[7:8]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[28:29]
	v_add_f64 v[28:29], v[26:27], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[21:22], -v[7:8]
	v_cndmask_b32_e64 v35, 0, 0x40100000, vcc_lo
	v_add_f64 v[39:40], v[26:27], -v[28:29]
	v_add_f64 v[28:29], v[32:33], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[13:14], v[13:14], v[34:35]
	v_add_f64 v[35:36], v[21:22], -v[26:27]
	v_add_f64 v[32:33], v[30:31], -v[39:40]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[37:38], v[24:25], v[13:14]
	v_add_f64 v[41:42], v[21:22], -v[35:36]
	v_add_f64 v[11:12], v[11:12], -v[35:36]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[28:29], v[28:29], v[32:33]
	v_cvt_i32_f64_e32 v37, v[37:38]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[26:27], -v[41:42]
	v_cvt_f64_i32_e32 v[35:36], v37
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], v[26:27]
	v_add_f64 v[13:14], v[13:14], -v[35:36]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[28:29], v[11:12]
	v_add_f64 v[26:27], v[24:25], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[11:12]
	v_add_f64 v[11:12], v[26:27], -v[13:14]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[7:8], v[5:6]
	v_add_f64 v[7:8], v[24:25], -v[11:12]
	v_cndmask_b32_e64 v35, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v21, null, 0, v37, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[26:27], -v[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[7:8], v[5:6]
	v_mul_f64 v[13:14], v[11:12], s[4:5]
	v_add_f64 v[7:8], v[11:12], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[24:25], v[11:12], s[4:5], -v[13:14]
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[11:12], s[6:7], v[24:25]
	v_fma_f64 v[7:8], v[5:6], s[4:5], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[13:14], v[7:8]
	v_add_f64 v[11:12], v[5:6], -v[13:14]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB2_9
	s_branch .LBB2_8
.LBB2_7:
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB2_9
.LBB2_8:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[5:6], |v[3:4]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[11:12], v[5:6]
	v_fma_f64 v[5:6], v[11:12], s[4:5], |v[3:4]|
	v_mul_f64 v[7:8], v[11:12], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[21:22], v[11:12], s[6:7], v[5:6]
	v_add_f64 v[13:14], v[5:6], v[7:8]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], -v[13:14]
	v_add_f64 v[13:14], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_fma_f64 v[7:8], v[11:12], s[6:7], v[7:8]
	v_add_f64 v[5:6], v[13:14], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_fma_f64 v[7:8], v[11:12], s[4:5], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[21:22], v[7:8]
	v_add_f64 v[13:14], v[5:6], -v[21:22]
	v_cvt_i32_f64_e32 v21, v[11:12]
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[13:14]
.LBB2_9:
	s_or_b32 exec_lo, exec_lo, s3
                                        ; implicit-def: $vgpr22
                                        ; implicit-def: $vgpr11_vgpr12
                                        ; implicit-def: $vgpr13_vgpr14
	s_and_saveexec_b32 s3, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s2, exec_lo, s3
	s_cbranch_execz .LBB2_11
; %bb.10:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[3:4]|
	v_mov_b32_e32 v32, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_cndmask_b32_e32 v12, v23, v20, vcc_lo
	v_cndmask_b32_e32 v11, v3, v19, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[13:14], v[17:18], v[11:12]
	v_mul_f64 v[19:20], v[15:16], v[11:12]
	v_fma_f64 v[17:18], v[17:18], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[15:16], v[15:16], v[11:12], -v[19:20]
	v_add_f64 v[22:23], v[19:20], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[24:25], v[22:23], -v[19:20]
	v_add_f64 v[28:29], v[13:14], v[22:23]
	v_add_f64 v[26:27], v[22:23], -v[24:25]
	v_add_f64 v[17:18], v[17:18], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[24:25], v[28:29], -2
	v_add_f64 v[13:14], v[28:29], -v[13:14]
	v_add_f64 v[19:20], v[19:20], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[24:25]|
	v_add_f64 v[13:14], v[22:23], -v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[17:18], v[17:18], v[19:20]
	v_fract_f64_e32 v[19:20], v[24:25]
	v_ldexp_f64 v[19:20], v[19:20], 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v20, 0, v20 :: v_dual_cndmask_b32 v19, 0, v19
	v_mul_f64 v[30:31], v[9:10], v[11:12]
	v_add_f64 v[26:27], v[30:31], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[11:12], -v[30:31]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[26:27], v[17:18]
	v_add_f64 v[24:25], v[13:14], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[28:29], v[24:25], v[19:20]
	v_add_f64 v[11:12], v[24:25], -v[13:14]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[28:29]
	v_add_f64 v[28:29], v[26:27], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[22:23], -v[11:12]
	v_cndmask_b32_e64 v33, 0, 0x40100000, vcc_lo
	v_add_f64 v[37:38], v[26:27], -v[28:29]
	v_add_f64 v[15:16], v[15:16], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[19:20], v[19:20], v[32:33]
	v_add_f64 v[33:34], v[22:23], -v[26:27]
	v_add_f64 v[28:29], v[30:31], -v[37:38]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[35:36], v[24:25], v[19:20]
	v_add_f64 v[39:40], v[22:23], -v[33:34]
	v_add_f64 v[17:18], v[17:18], -v[33:34]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[15:16], v[15:16], v[28:29]
	v_cvt_i32_f64_e32 v35, v[35:36]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[26:27], -v[39:40]
	v_cvt_f64_i32_e32 v[33:34], v35
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[17:18], v[26:27]
	v_add_f64 v[19:20], v[19:20], -v[33:34]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[15:16], v[17:18]
	v_add_f64 v[15:16], v[24:25], v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], v[13:14]
	v_add_f64 v[13:14], v[15:16], -v[19:20]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[11:12], v[9:10]
	v_add_f64 v[11:12], v[24:25], -v[13:14]
	v_cndmask_b32_e64 v33, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v22, null, 0, v35, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[11:12]
	v_add_f64 v[11:12], v[15:16], -v[32:33]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_mul_f64 v[15:16], v[13:14], s[4:5]
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], s[4:5], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[13:14], s[6:7], v[17:18]
	v_fma_f64 v[9:10], v[9:10], s[4:5], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[15:16], v[9:10]
	v_add_f64 v[13:14], v[11:12], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[13:14], v[9:10], -v[13:14]
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execnz .LBB2_12
	s_branch .LBB2_13
.LBB2_11:
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execz .LBB2_13
.LBB2_12:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[9:10], |v[3:4]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[9:10], v[9:10]
	v_fma_f64 v[11:12], v[9:10], s[4:5], |v[3:4]|
	v_mul_f64 v[13:14], v[9:10], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	v_cvt_i32_f64_e32 v22, v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[9:10], s[6:7], v[11:12]
	v_add_f64 v[15:16], v[11:12], v[13:14]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[15:16]
	v_add_f64 v[15:16], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], v[13:14]
	v_fma_f64 v[13:14], v[9:10], s[6:7], v[13:14]
	v_add_f64 v[11:12], v[15:16], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	v_fma_f64 v[13:14], v[9:10], s[4:5], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[17:18], v[13:14]
	v_add_f64 v[15:16], v[11:12], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
.LBB2_13:
	s_or_b32 exec_lo, exec_lo, s2
	v_mul_f64 v[9:10], v[5:6], v[5:6]
	v_mul_f64 v[15:16], v[11:12], v[11:12]
	s_load_b64 s[0:1], s[0:1], 0x0
	v_mad_i64_i32 v[17:18], null, v0, s9, 0
	v_add_nc_u32_e32 v0, s14, v1
	v_lshlrev_b64 v[23:24], 3, v[1:2]
	s_mov_b32 s2, 0xb42fdfa7
	s_mov_b32 s4, 0xf9a43bb8
	s_mov_b32 s3, 0xbe5ae600
	v_ashrrev_i32_e32 v1, 31, v0
	s_delay_alu instid0(VALU_DEP_4)
	v_lshlrev_b64 v[17:18], 3, v[17:18]
	s_mov_b32 s5, 0x3de5e0b2
	v_mul_f64 v[41:42], v[7:8], 0.5
	v_mul_f64 v[47:48], v[13:14], 0.5
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v17
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v25, null, s1, v18, vcc_lo
	s_mov_b32 s0, 0x9037ab78
	v_add_co_u32 v17, vcc_lo, v2, v23
	v_add_co_ci_u32_e64 v18, null, v25, v24, vcc_lo
	v_add_co_u32 v0, vcc_lo, v2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v25, v1, vcc_lo
	s_clause 0x1
	global_load_b64 v[23:24], v[17:18], off
	global_load_b64 v[25:26], v[0:1], off
	v_fma_f64 v[19:20], v[9:10], s[4:5], s[2:3]
	v_fma_f64 v[27:28], v[15:16], s[4:5], s[2:3]
	s_mov_b32 s2, 0x46cc5e42
	s_mov_b32 s4, 0x796cde01
	s_mov_b32 s1, 0x3e21eeb6
	s_mov_b32 s3, 0xbda907db
	s_mov_b32 s5, 0x3ec71de3
	v_fma_f64 v[29:30], v[9:10], s[2:3], s[0:1]
	v_mul_f64 v[31:32], v[9:10], 0.5
	v_fma_f64 v[33:34], v[15:16], s[2:3], s[0:1]
	v_mul_f64 v[35:36], v[15:16], 0.5
	s_mov_b32 s0, 0xa17f65f6
	s_mov_b32 s2, 0x19e83e5c
	s_mov_b32 s1, 0xbe927e4f
	s_mov_b32 s3, 0xbf2a01a0
	v_mul_f64 v[43:44], v[5:6], -v[9:10]
	v_mul_f64 v[49:50], v[11:12], -v[15:16]
	v_and_b32_e32 v2, 1, v21
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v2
	v_fma_f64 v[19:20], v[9:10], v[19:20], s[4:5]
	v_fma_f64 v[27:28], v[15:16], v[27:28], s[4:5]
	v_fma_f64 v[29:30], v[9:10], v[29:30], s[0:1]
	v_add_f64 v[37:38], -v[31:32], 1.0
	v_fma_f64 v[33:34], v[15:16], v[33:34], s[0:1]
	v_add_f64 v[39:40], -v[35:36], 1.0
	s_mov_b32 s0, 0x19f4ec90
	s_mov_b32 s1, 0x3efa01a0
	v_fma_f64 v[19:20], v[9:10], v[19:20], s[2:3]
	v_fma_f64 v[27:28], v[15:16], v[27:28], s[2:3]
	s_mov_b32 s2, 0x11110bb3
	s_mov_b32 s3, 0x3f811111
	v_fma_f64 v[29:30], v[9:10], v[29:30], s[0:1]
	v_add_f64 v[45:46], -v[37:38], 1.0
	v_fma_f64 v[33:34], v[15:16], v[33:34], s[0:1]
	v_add_f64 v[51:52], -v[39:40], 1.0
	s_mov_b32 s0, 0x16c16967
	s_mov_b32 s1, 0xbf56c16c
	v_fma_f64 v[19:20], v[9:10], v[19:20], s[2:3]
	v_fma_f64 v[27:28], v[15:16], v[27:28], s[2:3]
	v_fma_f64 v[29:30], v[9:10], v[29:30], s[0:1]
	v_add_f64 v[31:32], v[45:46], -v[31:32]
	v_fma_f64 v[33:34], v[15:16], v[33:34], s[0:1]
	v_add_f64 v[35:36], v[51:52], -v[35:36]
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s1, 0x3fa55555
	v_fma_f64 v[19:20], v[43:44], v[19:20], v[41:42]
	v_fma_f64 v[27:28], v[49:50], v[27:28], v[47:48]
	v_mul_f64 v[41:42], v[9:10], v[9:10]
	v_fma_f64 v[29:30], v[9:10], v[29:30], s[0:1]
	v_fma_f64 v[31:32], v[5:6], -v[7:8], v[31:32]
	v_fma_f64 v[7:8], v[9:10], v[19:20], -v[7:8]
	v_mul_f64 v[9:10], v[15:16], v[15:16]
	v_fma_f64 v[19:20], v[15:16], v[33:34], s[0:1]
	v_fma_f64 v[33:34], v[11:12], -v[13:14], v[35:36]
	v_fma_f64 v[13:14], v[15:16], v[27:28], -v[13:14]
	s_mov_b32 s1, 0xbfc55555
	v_fma_f64 v[15:16], v[41:42], v[29:30], v[31:32]
	v_fma_f64 v[7:8], v[43:44], s[0:1], v[7:8]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[9:10], v[9:10], v[19:20], v[33:34]
	v_fma_f64 v[13:14], v[49:50], s[0:1], v[13:14]
	v_cmp_class_f64_e64 s0, v[3:4], 0x1f8
	v_lshlrev_b32_e32 v3, 30, v22
	v_add_f64 v[15:16], v[37:38], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v3, v3, v4
	v_and_b32_e32 v3, 0x80000000, v3
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_add_f64 v[7:8], v[39:40], v[9:10]
	v_add_f64 v[9:10], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v2, v5, v15, vcc_lo
	v_xor_b32_e32 v6, 0x80000000, v6
	v_and_b32_e32 v5, 1, v22
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v2, 0, v2, s0
	v_cndmask_b32_e32 v6, v6, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cmp_eq_u32_e64 s1, 0, v5
	v_cndmask_b32_e64 v5, v8, v10, s1
	v_cndmask_b32_e64 v4, v7, v9, s1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v5, v5, v3
	v_cndmask_b32_e64 v3, 0, v4, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0x7ff80000, v5, s0
	s_waitcnt vmcnt(0)
	v_mul_f64 v[7:8], v[25:26], v[3:4]
	v_mul_f64 v[4:5], v[23:24], v[3:4]
	v_lshlrev_b32_e32 v3, 30, v21
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_b32_e32 v3, 0x80000000, v3
	v_xor_b32_e32 v3, v6, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v3, 0x7ff80000, v3, s0
	v_fma_f64 v[6:7], v[23:24], v[2:3], -v[7:8]
	v_fma_f64 v[2:3], v[25:26], v[2:3], v[4:5]
	s_clause 0x1
	global_store_b64 v[17:18], v[6:7], off
	global_store_b64 v[0:1], v[2:3], off
.LBB2_14:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel rope_partial
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
		.amdhsa_next_free_vgpr 53
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
		.amdhsa_inst_pref_size 51
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
	.size	rope_partial, .Lfunc_end2-rope_partial
                                        ; -- End function
	.set rope_partial.num_vgpr, 53
	.set rope_partial.num_agpr, 0
	.set rope_partial.numbered_sgpr, 24
	.set rope_partial.num_named_barrier, 0
	.set rope_partial.private_seg_size, 0
	.set rope_partial.uses_vcc, 1
	.set rope_partial.uses_flat_scratch, 0
	.set rope_partial.has_dyn_sized_stack, 0
	.set rope_partial.has_recursion, 0
	.set rope_partial.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 6424
; TotalNumSgprs: 26
; NumVgprs: 53
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 6
; NumSGPRsForWavesPerEU: 26
; NumVGPRsForWavesPerEU: 53
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
	.protected	gelu_mul                ; -- Begin function gelu_mul
	.globl	gelu_mul
	.p2align	8
	.type	gelu_mul,@function
gelu_mul:                               ; @gelu_mul
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b256 s[4:11], s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s0, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s0, s2, v[0:1]
	s_mov_b32 s0, exec_lo
	v_cmpx_gt_i64_e64 s[10:11], v[2:3]
	s_cbranch_execz .LBB3_2
; %bb.1:
	v_lshlrev_b64 v[0:1], 3, v[2:3]
	s_mov_b32 s0, 0x6d4801f7
	s_mov_b32 s1, 0x3fa6e4e2
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s0, 0x33d43651
	s_mov_b32 s1, 0x3fe98845
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[2:3], v[4:5]
	v_fma_f64 v[4:5], v[2:3], v[4:5], v[2:3]
	v_mul_f64 v[2:3], v[2:3], 0.5
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[4:5], v[4:5], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[6:7], |v[4:5]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[0:1], |v[4:5]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	v_add_f64 v[12:13], v[8:9], 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], 0
	v_add_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_mul_f64 v[10:11], v[6:7], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[12:13], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], v[10:11]
	v_add_f64 v[14:15], v[14:15], -v[12:13]
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_add_f64 v[10:11], v[16:17], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], s[2:3], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[14:15], v[16:17], -v[10:11]
	v_mul_f64 v[16:17], v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_fma_f64 v[14:15], v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	v_add_f64 v[18:19], v[8:9], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	v_fma_f64 v[14:15], v[10:11], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[12:13], v[10:11], v[12:13], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[18:19], v[12:13]
	v_fma_f64 v[16:17], v[18:19], v[12:13], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], v[12:13], v[16:17]
	v_add_f64 v[14:15], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[10:11], v[14:15]
	v_add_f64 v[18:19], v[14:15], -v[20:21]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_cvt_i32_f64_e32 v18, v[6:7]
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[10:11], 1.0
	v_add_f64 v[14:15], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[12:13], -1.0
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], v[8:9]
	v_ldexp_f64 v[10:11], v[6:7], v18
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[14:15], v[10:11]
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v18
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], -v[10:11], v[14:15], 1.0
	v_fma_f64 v[12:13], v[16:17], v[14:15], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[10:11], v[12:13]
	v_fma_f64 v[14:15], v[12:13], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[12:13], v[6:7], v[14:15]
	v_add_f64 v[16:17], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], -v[16:17], 1.0
	v_add_f64 v[8:9], v[16:17], -v[8:9]
	v_add_f64 v[20:21], -v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[14:15], v[20:21], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[16:17], v[12:13], v[14:15]
	v_add_f64 v[18:19], v[18:19], -v[14:15]
	v_mul_f64 v[20:21], v[10:11], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[18:19]
	v_fma_f64 v[22:23], v[16:17], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[22:23], v[16:17], v[6:7], v[22:23]
	v_add_f64 v[24:25], v[20:21], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[14:15], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[22:23]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[14:15]
	v_add_f64 v[14:15], v[12:13], v[16:17]
	v_add_f64 v[8:9], v[18:19], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], -v[12:13]
	v_add_f64 v[8:9], v[26:27], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	v_mul_f64 v[8:9], v[12:13], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[16:17], v[8:9]
	v_add_f64 v[12:13], v[14:15], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[10:11], v[12:13]
	v_add_f64 v[14:15], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[10:11], -v[12:13]
	v_add_f64 v[18:19], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	v_add_f64 v[10:11], v[10:11], -v[20:21]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[12:13], -v[18:19]
	v_add_f64 v[18:19], v[6:7], v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	v_add_f64 v[14:15], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[10:11]
	v_add_f64 v[18:19], v[16:17], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_rcp_f64_e32 v[22:23], v[18:19]
	v_add_f64 v[16:17], v[18:19], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[18:19], v[22:23], 1.0
	v_fma_f64 v[10:11], v[12:13], v[22:23], v[22:23]
	v_add_f64 v[12:13], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[18:19], v[10:11], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[8:9]
	v_mul_f64 v[22:23], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[10:11], v[18:19], -v[22:23]
	v_fma_f64 v[14:15], v[10:11], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[22:23], v[14:15]
	v_add_f64 v[18:19], v[12:13], -v[16:17]
	v_add_f64 v[22:23], v[16:17], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[12:13], -v[18:19]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_co_u32 v20, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v21, null, s7, v1, vcc_lo
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[4:5]|
	global_load_b64 v[20:21], v[20:21], off
	v_add_f64 v[16:17], v[24:25], -v[16:17]
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
	v_add_co_u32 v0, vcc_lo, s8, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	v_bfi_b32 v5, 0x7fffffff, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], 1.0
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], v[20:21], v[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel gelu_mul
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
		.amdhsa_next_free_vgpr 28
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
	.text
.Lfunc_end3:
	.size	gelu_mul, .Lfunc_end3-gelu_mul
                                        ; -- End function
	.set gelu_mul.num_vgpr, 28
	.set gelu_mul.num_agpr, 0
	.set gelu_mul.numbered_sgpr, 12
	.set gelu_mul.num_named_barrier, 0
	.set gelu_mul.private_seg_size, 0
	.set gelu_mul.uses_vcc, 1
	.set gelu_mul.uses_flat_scratch, 0
	.set gelu_mul.has_dyn_sized_stack, 0
	.set gelu_mul.has_recursion, 0
	.set gelu_mul.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1892
; TotalNumSgprs: 14
; NumVgprs: 28
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 28
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
	.protected	glu_gelu                ; -- Begin function glu_gelu
	.globl	glu_gelu
	.p2align	8
	.type	glu_gelu,@function
glu_gelu:                               ; @glu_gelu
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x24
	s_load_b64 s[4:5], s[0:1], 0x10
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v4
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[2:3], null, s3, s2, v[0:1]
	s_mul_hi_i32 s3, s5, s4
	s_mul_i32 s2, s5, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[2:3]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB4_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB4_3
; %bb.2:
	s_ashr_i32 s6, s5, 31
	v_ashrrev_i32_e32 v8, 31, v3
	s_add_u32 s8, s4, s6
	s_mov_b32 s7, s6
	s_addc_u32 s9, s5, s6
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[8:9], s[8:9], s[6:7]
	v_add_co_u32 v4, vcc_lo, v2, v8
	v_cvt_f32_u32_e32 v0, s8
	v_cvt_f32_u32_e32 v1, s9
	s_sub_u32 s10, 0, s8
	s_subb_u32 s11, 0, s9
	v_add_co_ci_u32_e64 v5, null, v3, v8, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v4, v8
	v_xor_b32_e32 v10, v5, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x5f7ffffc, v0
	v_mul_f32_e32 v1, 0x2f800000, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_trunc_f32_e32 v1, v1
	v_fmamk_f32 v0, v1, 0xcf800000, v0
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v0, v0
	v_readfirstlane_b32 s2, v1
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v0
	s_mul_i32 s12, s10, s2
	s_mul_hi_u32 s14, s10, s7
	s_mul_i32 s13, s11, s7
	s_add_i32 s12, s14, s12
	s_mul_i32 s15, s10, s7
	s_add_i32 s12, s12, s13
	s_mul_hi_u32 s14, s7, s15
	s_mul_i32 s17, s7, s12
	s_mul_hi_u32 s16, s2, s15
	s_mul_i32 s13, s2, s15
	s_mul_hi_u32 s15, s7, s12
	s_add_u32 s14, s14, s17
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s18, s2, s12
	s_add_u32 s13, s14, s13
	s_mul_i32 s12, s2, s12
	s_addc_u32 s13, s15, s16
	s_addc_u32 s14, s18, 0
	s_add_u32 s12, s13, s12
	s_addc_u32 s13, 0, s14
	s_add_u32 s7, s7, s12
	s_cselect_b32 s12, -1, 0
	s_mul_hi_u32 s14, s10, s7
	s_cmp_lg_u32 s12, 0
	s_mul_i32 s12, s10, s7
	s_addc_u32 s2, s2, s13
	s_mul_i32 s11, s11, s7
	s_mul_i32 s10, s10, s2
	s_mul_hi_u32 s13, s7, s12
	s_add_i32 s10, s14, s10
	s_mul_hi_u32 s14, s2, s12
	s_add_i32 s10, s10, s11
	s_mul_i32 s11, s2, s12
	s_mul_i32 s16, s7, s10
	s_mul_hi_u32 s15, s7, s10
	s_add_u32 s13, s13, s16
	s_addc_u32 s15, 0, s15
	s_mul_hi_u32 s12, s2, s10
	s_add_u32 s11, s13, s11
	s_mul_i32 s10, s2, s10
	s_addc_u32 s11, s15, s14
	s_addc_u32 s12, s12, 0
	s_add_u32 s10, s11, s10
	s_addc_u32 s11, 0, s12
	s_add_u32 s7, s7, s10
	s_cselect_b32 s10, -1, 0
	v_mul_hi_u32 v11, v9, s7
	s_cmp_lg_u32 s10, 0
	v_mad_u64_u32 v[4:5], null, v10, s7, 0
	s_addc_u32 s2, s2, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[6:7], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v5, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v7, vcc_lo
	v_add_co_u32 v4, vcc_lo, v0, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v5, null, 0, v1, vcc_lo
	v_mul_lo_u32 v6, s9, v4
	v_mad_u64_u32 v[0:1], null, s8, v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v7, s8, v5
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v7, v6
	v_add_co_u32 v7, s2, v4, 2
	v_add_co_ci_u32_e64 v9, null, 0, v5, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v6, v10, v1
	v_sub_co_u32 v11, s2, v0, s8
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v6, null, s9, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s8, v11
	v_subrev_co_ci_u32_e64 v6, null, 0, v6, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s8, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v6
	v_cndmask_b32_e32 v6, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v4, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v11, null, 0, v5, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s9, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v6
	v_xor_b32_e32 v6, s6, v8
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v7 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v4, v0, s2
	v_cndmask_b32_e64 v1, v5, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v6
	v_xor_b32_e32 v1, v1, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v6
	v_sub_co_ci_u32_e64 v1, null, v1, v6, vcc_lo
.LBB4_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB4_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s4
	s_sub_i32 s3, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s3, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s4
	v_add_nc_u32_e32 v4, 1, v0
	v_sub_nc_u32_e32 v1, v2, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s4, v1
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s4, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v4, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v4, vcc_lo
.LBB4_5:
	s_or_b32 exec_lo, exec_lo, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v4, 0 :: v_dual_mov_b32 v5, v0
	s_load_b128 s[0:3], s[0:1], 0x0
	v_mul_lo_u32 v8, v1, s4
	v_mul_lo_u32 v9, v0, s5
	v_mad_u64_u32 v[6:7], null, v0, s4, 0
	v_ashrrev_i64 v[4:5], 31, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add3_u32 v7, v7, v9, v8
	v_mul_lo_u32 v5, v5, s4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v10, v4, s5
	v_mad_u64_u32 v[0:1], null, v4, s4, 0
	v_sub_co_u32 v4, vcc_lo, v2, v6
	v_add3_u32 v1, v1, v10, v5
	v_sub_co_ci_u32_e64 v5, null, v3, v7, vcc_lo
	v_lshlrev_b64 v[2:3], 3, v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[0:1], 3, v[0:1]
	v_lshlrev_b64 v[4:5], 3, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v7, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 3
	v_add_co_u32 v0, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v7, v5, vcc_lo
	v_add_co_u32 v6, vcc_lo, v6, s0
	v_add_co_ci_u32_e64 v7, null, s1, v7, vcc_lo
	global_load_b64 v[0:1], v[0:1], off
	v_add_co_u32 v4, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, v7, v5, vcc_lo
	s_mov_b32 s0, 0x6d4801f7
	s_mov_b32 s1, 0x3fa6e4e2
	s_mov_b32 s4, 0x6a5dcb37
	global_load_b64 v[4:5], v[4:5], off
	s_mov_b32 s5, 0x3e5ade15
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], v[0:1], s[0:1]
	s_mov_b32 s0, 0x33d43651
	s_mov_b32 s1, 0x3fe98845
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[0:1], v[6:7]
	v_fma_f64 v[6:7], v[0:1], v[6:7], v[0:1]
	v_mul_f64 v[0:1], v[0:1], 0.5
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[6:7], v[6:7], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[8:9], |v[6:7]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[6:7]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[8:9]
	v_fma_f64 v[10:11], v[8:9], s[0:1], |v[6:7]|
	s_mov_b32 s0, 0xf278e000
	s_mov_b32 s1, 0xbd53de6a
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f64 v[12:13], v[8:9], s[0:1]
	s_mov_b32 s0, 0xf97b57a0
	s_mov_b32 s1, 0xbac9cc01
	v_add_f64 v[14:15], v[10:11], 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], v[12:13]
	v_add_f64 v[10:11], v[10:11], -v[14:15]
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], 0
	v_add_f64 v[12:13], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_mul_f64 v[12:13], v[8:9], s[0:1]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	v_add_f64 v[14:15], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], v[12:13]
	v_add_f64 v[16:17], v[16:17], -v[14:15]
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_add_f64 v[12:13], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[18:19], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[12:13], s[4:5], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	v_add_f64 v[16:17], v[18:19], -v[12:13]
	v_mul_f64 v[18:19], v[12:13], v[12:13]
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_fma_f64 v[16:17], v[12:13], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	v_add_f64 v[20:21], v[10:11], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	v_fma_f64 v[16:17], v[12:13], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	v_add_f64 v[20:21], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[14:15], v[12:13], v[14:15], s[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[22:23], v[20:21], v[14:15]
	v_fma_f64 v[18:19], v[20:21], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[14:15], v[16:17], v[14:15], v[18:19]
	v_add_f64 v[16:17], v[22:23], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[12:13], v[16:17]
	v_add_f64 v[20:21], v[16:17], -v[22:23]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], -v[20:21]
	v_cvt_i32_f64_e32 v20, v[8:9]
	v_add_f64 v[12:13], v[16:17], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], v[10:11]
	v_add_f64 v[14:15], v[12:13], 1.0
	v_add_f64 v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[14:15], -1.0
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[18:19]
	v_add_f64 v[10:11], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[14:15], v[10:11]
	v_ldexp_f64 v[12:13], v[8:9], v20
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[16:17], v[12:13]
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], v20
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[12:13], v[16:17], 1.0
	v_fma_f64 v[16:17], v[18:19], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], -v[12:13], v[16:17], 1.0
	v_fma_f64 v[14:15], v[18:19], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[10:11], v[12:13], v[14:15]
	v_fma_f64 v[16:17], v[14:15], v[12:13], -v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[14:15], v[8:9], v[16:17]
	v_add_f64 v[18:19], v[10:11], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], -v[18:19], 1.0
	v_add_f64 v[10:11], v[18:19], -v[10:11]
	v_add_f64 v[22:23], -v[20:21], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[16:17], v[22:23], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_add_f64 v[16:17], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[14:15], v[16:17]
	v_add_f64 v[20:21], v[20:21], -v[16:17]
	v_mul_f64 v[22:23], v[12:13], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[20:21]
	v_fma_f64 v[24:25], v[18:19], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[24:25], v[18:19], v[8:9], v[24:25]
	v_add_f64 v[26:27], v[22:23], v[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[28:29], v[16:17], -v[26:27]
	v_add_f64 v[20:21], v[26:27], -v[22:23]
	v_add_f64 v[16:17], v[16:17], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[24:25]
	v_add_f64 v[16:17], v[16:17], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[16:17]
	v_add_f64 v[16:17], v[14:15], v[18:19]
	v_add_f64 v[10:11], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[16:17], -v[14:15]
	v_add_f64 v[10:11], v[28:29], v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	v_mul_f64 v[10:11], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[10:11], v[18:19], v[10:11]
	v_add_f64 v[14:15], v[16:17], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[12:13], v[14:15]
	v_add_f64 v[16:17], v[14:15], -v[16:17]
	v_add_f64 v[22:23], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], -v[16:17]
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[14:15], -v[20:21]
	v_add_f64 v[20:21], v[8:9], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_add_f64 v[16:17], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], v[12:13]
	v_add_f64 v[20:21], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[10:11]
	v_rcp_f64_e32 v[24:25], v[20:21]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[14:15], -v[20:21], v[24:25], 1.0
	v_fma_f64 v[12:13], v[14:15], v[24:25], v[24:25]
	v_add_f64 v[14:15], v[22:23], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[20:21], v[12:13], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[14:15], v[10:11]
	v_mul_f64 v[24:25], v[20:21], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], v[12:13], v[20:21], -v[24:25]
	v_fma_f64 v[16:17], v[12:13], v[16:17], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[24:25], v[16:17]
	v_add_f64 v[20:21], v[14:15], -v[18:19]
	v_add_f64 v[24:25], v[18:19], -v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[14:15], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	v_add_f64 v[16:17], v[16:17], -v[24:25]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[18:19], v[26:27], -v[18:19]
	v_add_f64 v[8:9], v[8:9], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[18:19], -v[16:17]
	v_add_f64 v[8:9], v[8:9], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[20:21], v[8:9]
	v_mul_f64 v[8:9], v[10:11], v[8:9]
	v_and_b32_e32 v10, 0x7fffffff, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[8:9], v[12:13], v[8:9]
	v_cndmask_b32_e32 v9, 0x3ff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[6:7]|
	v_cndmask_b32_e32 v6, v8, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v8, v9, v10, vcc_lo
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_bfi_b32 v7, 0x7fffffff, v8, v7
	v_add_f64 v[6:7], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[0:1], v[0:1], v[6:7]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[0:1], v[4:5], v[0:1]
	global_store_b64 v[2:3], v[0:1], off
.LBB4_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel glu_gelu
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
		.amdhsa_inst_pref_size 24
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
	.size	glu_gelu, .Lfunc_end4-glu_gelu
                                        ; -- End function
	.set glu_gelu.num_vgpr, 30
	.set glu_gelu.num_agpr, 0
	.set glu_gelu.numbered_sgpr, 19
	.set glu_gelu.num_named_barrier, 0
	.set glu_gelu.private_seg_size, 0
	.set glu_gelu.uses_vcc, 1
	.set glu_gelu.uses_flat_scratch, 0
	.set glu_gelu.has_dyn_sized_stack, 0
	.set glu_gelu.has_recursion, 0
	.set glu_gelu.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2976
; TotalNumSgprs: 21
; NumVgprs: 30
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 21
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_9347edca86c33db3,@object ; @__hip_cuid_9347edca86c33db3
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_9347edca86c33db3
__hip_cuid_9347edca86c33db3:
	.byte	0                               ; 0x0
	.size	__hip_cuid_9347edca86c33db3, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_9347edca86c33db3
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
    .name:           widen_bf16_f64
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         widen_bf16_f64.kd
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
      - .offset:         176
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 512
    .kernarg_segment_align: 8
    .kernarg_segment_size: 312
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           gqa_masked_attn
    .private_segment_fixed_size: 0
    .sgpr_count:     51
    .sgpr_spill_count: 0
    .symbol:         gqa_masked_attn.kd
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
      - .offset:         8
        .size:           4
        .value_kind:     by_value
      - .offset:         12
        .size:           4
        .value_kind:     by_value
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
    .name:           rope_partial
    .private_segment_fixed_size: 0
    .sgpr_count:     26
    .sgpr_spill_count: 0
    .symbol:         rope_partial.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     53
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
        .size:           8
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
    .name:           gelu_mul
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         gelu_mul.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     28
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
    .name:           glu_gelu
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         glu_gelu.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     30
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
