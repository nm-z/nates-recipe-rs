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
	.protected	widen_bf16_f32          ; -- Begin function widen_bf16_f32
	.globl	widen_bf16_f32
	.p2align	8
	.type	widen_bf16_f32,@function
widen_bf16_f32:                         ; @widen_bf16_f32
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
	v_lshlrev_b64 v[0:1], 1, v[2:3]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_load_u16 v4, v[0:1], off
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_lshlrev_b32_e32 v2, 16, v4
	global_store_b32 v[0:1], v2, off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel widen_bf16_f32
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
	.size	widen_bf16_f32, .Lfunc_end1-widen_bf16_f32
                                        ; -- End function
	.set widen_bf16_f32.num_vgpr, 5
	.set widen_bf16_f32.num_agpr, 0
	.set widen_bf16_f32.numbered_sgpr, 6
	.set widen_bf16_f32.num_named_barrier, 0
	.set widen_bf16_f32.private_seg_size, 0
	.set widen_bf16_f32.uses_vcc, 1
	.set widen_bf16_f32.uses_flat_scratch, 0
	.set widen_bf16_f32.has_dyn_sized_stack, 0
	.set widen_bf16_f32.has_recursion, 0
	.set widen_bf16_f32.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 160
; TotalNumSgprs: 8
; NumVgprs: 5
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
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
	.section	.text._Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid,"axG",@progbits,_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid,comdat
	.protected	_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid ; -- Begin function _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid
	.globl	_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid
	.p2align	8
	.type	_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid,@function
_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid: ; @_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid
; %bb.0:
	s_clause 0x1
	s_load_b128 s[16:19], s[0:1], 0x20
	s_load_b32 s44, s[0:1], 0x30
	v_mov_b32_e32 v3, 0
	s_abs_i32 s6, s2
	v_mov_b32_e32 v4, 0
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s3, s17
	s_ashr_i32 s7, s17, 31
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
	s_xor_b32 s9, s5, s7
	s_mul_i32 s8, s4, s3
	s_sub_i32 s5, s6, s8
	s_add_i32 s6, s4, 1
	s_sub_i32 s8, s5, s3
	s_cmp_ge_u32 s5, s3
	s_cselect_b32 s4, s6, s4
	s_cselect_b32 s5, s8, s5
	s_add_i32 s6, s4, 1
	s_cmp_ge_u32 s5, s3
	s_cselect_b32 s6, s6, s4
	s_abs_i32 s8, s18
	s_xor_b32 s6, s6, s9
	v_cvt_f32_u32_e32 v1, s8
	s_sub_i32 s11, 0, s8
	s_load_b64 s[4:5], s[0:1], 0x38
	s_sub_i32 s33, s6, s9
	s_ashr_i32 s12, s18, 31
	v_rcp_iflag_f32_e32 v1, v1
	s_mul_i32 s9, s33, s17
	s_xor_b32 s7, s7, s12
	s_sub_i32 s45, s2, s9
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s10, v1
	s_mul_i32 s11, s11, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s11, s10, s11
	s_add_i32 s10, s10, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s6, s3, s10
	s_mul_i32 s10, s6, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s2, s3, s10
	s_add_i32 s3, s6, 1
	s_sub_i32 s9, s2, s8
	s_cmp_ge_u32 s2, s8
	s_cselect_b32 s3, s3, s6
	s_cselect_b32 s2, s9, s2
	s_add_i32 s6, s3, 1
	s_cmp_ge_u32 s2, s8
	s_cselect_b32 s2, s6, s3
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f64_e64 s3, s[4:5], 0
	s_xor_b32 s2, s2, s7
	s_abs_i32 s48, s45
	s_sub_i32 s2, s2, s7
	s_ashr_i32 s47, s45, 31
	s_abs_i32 s46, s2
	s_ashr_i32 s50, s2, 31
	v_cvt_f32_u32_e32 v1, s46
	s_sub_i32 s7, 0, s46
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s6, v1
	s_mul_i32 s7, s7, s6
	s_and_b32 vcc_lo, exec_lo, s3
	s_mul_hi_u32 s7, s6, s7
	s_add_i32 s6, s6, s7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_hi_u32 s49, s48, s6
	s_cbranch_vccnz .LBB2_5
; %bb.1:
	v_cvt_f64_i32_e32 v[1:2], s17
	s_mov_b32 s9, 0x3fe55555
	s_mov_b32 s8, 0x55555555
	s_mov_b32 s2, 0x6b47b09a
	s_mov_b32 s6, 0xbf559e2b
	s_mov_b32 s3, 0x3fc38538
	s_mov_b32 s7, 0x3fc3ab76
	s_mov_b32 s41, 0x3fd99999
	s_mov_b32 s40, 0x998ef7b6
	s_mov_b32 s10, 0xffda0d24
	s_mov_b32 s11, 0x3c7777d0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[3:4], v[1:2]
	v_cmp_gt_f64_e32 vcc_lo, s[8:9], v[3:4]
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v5
	v_add_f64 v[5:6], v[3:4], 1.0
	v_add_f64 v[11:12], v[3:4], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[7:8], v[5:6]
	v_add_f64 v[13:14], v[5:6], -1.0
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[5:6], v[9:10]
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[9:10], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[11:12], -v[5:6]
	v_add_f64 v[15:16], v[5:6], -v[15:16]
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[15:16], -v[3:4]
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[3:4], v[13:14], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[3:4], v[7:8], v[3:4]
	v_add_f64 v[5:6], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[5:6], v[5:6]
	v_fma_f64 v[11:12], v[7:8], s[6:7], s[2:3]
	s_mov_b32 s2, 0xd7f4df2e
	s_mov_b32 s3, 0x3fc7474d
	s_mov_b32 s6, 0x55555780
	s_mov_b32 s7, s9
	v_mul_f64 v[13:14], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[2:3]
	s_mov_b32 s2, 0x16291751
	s_mov_b32 s3, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[2:3]
	s_mov_b32 s3, 0x3fd24924
	s_mov_b32 s2, 0x9b27acf1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[40:41]
	v_fma_f64 v[7:8], v[7:8], v[11:12], s[6:7]
	v_ldexp_f64 v[11:12], v[5:6], 1
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[7:8], v[13:14], v[7:8]
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[11:12], v[7:8]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], -v[11:12]
	v_add_f64 v[5:6], v[7:8], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], s[6:7]
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], s[6:7], -v[9:10]
	v_fma_f64 v[3:4], v[3:4], s[6:7], v[7:8]
	v_frexp_exp_i32_f64_e32 v7, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[1:2], v[5:6], s[10:11], v[3:4]
	v_subrev_co_ci_u32_e64 v3, null, 0, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[3:4], v3
	v_add_f64 v[5:6], v[9:10], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], v[3:4]
	v_add_f64 v[9:10], v[5:6], -v[9:10]
	v_add_f64 v[11:12], v[7:8], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[1:2], v[1:2], -v[9:10]
	v_add_f64 v[13:14], v[11:12], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[13:14], v[3:4]
	v_add_f64 v[3:4], v[5:6], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_f64 v[1:2], v[7:8], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_floor_f64_e32 v[1:2], v[1:2]
	v_cvt_i32_f64_e32 v1, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s2, v1
	s_lshl_b32 s22, 1, s2
	v_cvt_f64_i32_e32 v[1:2], s22
	s_cmp_ge_i32 s45, s22
	s_cbranch_scc0 .LBB2_3
; %bb.2:
	v_mul_f64 v[3:4], s[4:5], -0.5
	s_mov_b32 s11, 0x3c7abc9e
	s_mov_b32 s10, 0x3b39803f
	s_mov_b32 s15, 0x3fe62e42
	s_mov_b32 s14, 0xfefa39ef
	s_mov_b32 s12, 0xfca7ab0c
	s_mov_b32 s20, 0x6a5dcb37
	s_mov_b32 s13, 0x3e928af3
	s_mov_b32 s21, 0x3e5ade15
	s_mov_b32 s24, 0x623fde64
	s_mov_b32 s25, 0x3ec71dee
	s_mov_b32 s26, 0x7c89e6b0
	s_mov_b32 s27, 0x3efa0199
	s_mov_b32 s28, 0x14761f6e
	s_mov_b32 s29, 0x3f2a01a0
	s_mov_b32 s30, 0x1852b7b0
	s_mov_b32 s31, 0x3f56c16c
	s_mov_b32 s34, 0x11122322
	s_mov_b32 s35, 0x3f811111
	s_mov_b32 s36, 0x555502a1
	s_mov_b32 s37, 0x3fa55555
	s_mov_b32 s38, 0x55555511
	s_mov_b32 s39, 0x3fc55555
	s_mov_b32 s42, 11
	s_mov_b32 s43, 0x3fe00000
	s_mov_b32 s54, 0x4222de17
	s_mov_b32 s55, 0x3fbdee67
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[5:6], null, v[1:2], v[1:2], v[3:4]
	v_div_scale_f64 v[11:12], vcc_lo, v[3:4], v[1:2], v[3:4]
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[9:10], v[11:12]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[5:6], v[1:2], v[3:4]
	v_rndne_f64_e32 v[5:6], v[3:4]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[3:4]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[3:4], -v[5:6]
	v_cvt_i32_f64_e32 v11, v[5:6]
	s_and_b32 s40, vcc_lo, exec_lo
	v_mul_f64 v[9:10], v[7:8], s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], s[14:15], v[9:10]
	v_fma_f64 v[9:10], v[7:8], s[20:21], s[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[24:25]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[28:29]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[34:35]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[38:39]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], 1.0
	v_fma_f64 v[5:6], v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v11
	v_readfirstlane_b32 s23, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s40, v5
	s_cselect_b32 s23, s23, 0x7ff00000
	s_and_b32 s51, s2, vcc_lo
	s_and_b32 s51, s51, exec_lo
	s_cselect_b32 s52, s40, 0
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s53, s23, 0
	s_sub_i32 s2, s45, s22
	v_cmp_neq_f64_e64 vcc_lo, s[52:53], 1.0
	s_lshl_b32 s2, s2, 1
	s_mov_b32 s40, 0x9999999c
	s_or_b32 s2, s2, 1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_u32_e32 v[3:4], s2
	v_cndmask_b32_e32 v4, 0x3ff00000, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[3:4]
	s_and_b32 s2, vcc_lo, exec_lo
	s_cselect_b32 s23, s53, 0x3ff00000
	s_cselect_b32 s22, s52, 0
	s_mov_b32 s52, 0x968915a9
	v_frexp_mant_f64_e64 v[5:6], |s[22:23]|
	s_mov_b32 s53, 0x3fba6564
	s_mov_b32 s2, 0x924920da
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[8:9], v[5:6]
	v_cndmask_b32_e64 v7, 0, 1, vcc_lo
	v_ldexp_f64 v[5:6], v[5:6], v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], 1.0
	v_add_f64 v[13:14], v[5:6], -1.0
	v_rcp_f64_e32 v[9:10], v[7:8]
	v_add_f64 v[15:16], v[7:8], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	v_mul_f64 v[17:18], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[11:12], v[7:8], -v[17:18]
	v_fma_f64 v[5:6], v[11:12], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[17:18], v[5:6]
	v_add_f64 v[15:16], v[13:14], -v[7:8]
	v_add_f64 v[17:18], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_add_f64 v[5:6], v[17:18], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[5:6]
	v_mul_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_add_f64 v[9:10], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_fma_f64 v[9:10], v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[5:6], v[5:6]
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[54:55], s[52:53]
	s_mov_b32 s52, 0x3abe935a
	s_mov_b32 s53, 0x3fbe25e4
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_mul_f64 v[21:22], v[7:8], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x47e6c9c2
	s_mov_b32 s53, 0x3fc110ef
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0xcfa74449
	s_mov_b32 s53, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x71bf3c30
	s_mov_b32 s53, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x1c7792ce
	s_mov_b32 s53, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[2:3]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[11:12], v[13:14], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	v_add_f64 v[15:16], v[17:18], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[19:20], v[15:16], s[8:9]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_mov_b32 s9, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[23:24], v[19:20], s[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_fma_f64 v[17:18], v[13:14], v[7:8], -v[21:22]
	v_add_f64 v[15:16], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], s[2:3]
	v_fma_f64 v[13:14], v[13:14], v[5:6], v[17:18]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[7:8], v[13:14]
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[19:20], v[11:12]
	v_add_f64 v[15:16], v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[13:14]
	v_mul_f64 v[19:20], v[15:16], v[13:14]
	v_add_f64 v[21:22], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[17:18]
	v_fma_f64 v[17:18], v[15:16], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[21:22]
	v_fma_f64 v[11:12], v[15:16], v[11:12], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[9:10], v[13:14], v[11:12]
	v_frexp_exp_i32_f64_e32 v13, s[22:23]
	v_add_f64 v[11:12], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v13, null, 0, v13, vcc_lo
	v_cvt_f64_i32_e32 v[13:14], v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[7:8], v[11:12]
	v_add_f64 v[17:18], v[11:12], -v[19:20]
	v_mul_f64 v[19:20], v[13:14], s[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[15:16], -v[7:8]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], s[14:15], -v[19:20]
	s_mov_b32 s15, 0xbfe62e42
	v_add_f64 v[7:8], v[11:12], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_fma_f64 v[9:10], v[13:14], s[10:11], v[17:18]
	s_mov_b32 s11, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], v[5:6]
	v_add_f64 v[19:20], v[7:8], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[7:8], v[11:12]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[15:16], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[21:22]
	v_add_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[15:16], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[11:12]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	v_add_f64 v[17:18], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[11:12], v[17:18], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[17:18], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[7:8], -v[17:18]
	v_mul_f64 v[11:12], v[3:4], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[3:4], v[7:8], -v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	v_fma_f64 v[5:6], v[3:4], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_dual_cndmask_b32 v10, v8, v12 :: v_dual_cndmask_b32 v9, v7, v11
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[3:4], 0.5
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[13:14], v[9:10], s[6:7]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[9:10]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[9:10]
	v_cmp_neq_f64_e64 s3, 0x7ff00000, |v[9:10]|
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_trunc_f64_e32 v[7:8], v[3:4]
	v_rndne_f64_e32 v[13:14], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v6, 0, v6, s3
	v_cndmask_b32_e64 v5, 0, v5, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], s[14:15], v[9:10]
	v_cvt_i32_f64_e32 v19, v[13:14]
	v_trunc_f64_e32 v[9:10], v[11:12]
	v_fma_f64 v[15:16], v[13:14], s[10:11], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s3, v[9:10], v[11:12]
	v_fma_f64 v[17:18], v[15:16], s[20:21], s[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[24:25]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[28:29]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[34:35]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[38:39]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[13:14], v[13:14], v19
	v_cndmask_b32_e32 v14, 0x7ff00000, v14, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s6, v13
	v_cndmask_b32_e64 v14, 0, v14, s2
	s_and_b32 s2, s2, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s6, s6, 0
	v_cmp_eq_f64_e64 s2, v[7:8], v[3:4]
	v_mov_b32_e32 v13, s6
	v_fma_f64 v[5:6], v[13:14], v[5:6], v[13:14]
	v_cmp_class_f64_e64 vcc_lo, v[13:14], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v5
	s_and_b32 s8, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v6, v14, vcc_lo
	v_cmp_lt_f64_e64 vcc_lo, s[22:23], 0
	v_cmp_eq_f64_e64 s8, s[22:23], 0
	s_cselect_b32 s6, s6, s7
	s_and_b32 s3, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s7, s3, exec_lo
	s_cselect_b32 s7, s23, 0x3ff00000
	v_bfi_b32 v3, 0x7fffffff, v3, s7
	v_cmp_class_f64_e64 s7, s[22:23], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s2
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, s6, 0
	s_and_b32 s9, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	s_cselect_b32 s6, s2, s6
	s_or_b32 vcc_lo, s8, s7
	s_and_b32 s2, s8, exec_lo
	s_cselect_b32 s2, 0, 0x7ff00000
	s_and_b32 s3, s3, exec_lo
	s_cselect_b32 s3, s23, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v4, s3
	s_and_b32 s3, vcc_lo, exec_lo
	s_cselect_b32 s3, 0, s6
	v_bfi_b32 v4, 0x7fffffff, s2, v4
	v_cmp_o_f64_e64 s2, s[22:23], s[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s2
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, s3, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v3, s2
	s_cbranch_execz .LBB2_4
	s_branch .LBB2_5
.LBB2_3:
                                        ; implicit-def: $vgpr3_vgpr4
.LBB2_4:
	s_delay_alu instid0(VALU_DEP_1)
	v_div_scale_f64 v[3:4], null, v[1:2], v[1:2], -s[4:5]
	v_div_scale_f64 v[9:10], vcc_lo, -s[4:5], v[1:2], -s[4:5]
	s_mov_b32 s11, 0x3fe62e42
	s_mov_b32 s10, 0xfefa39ef
	s_mov_b32 s8, 0xfca7ab0c
	s_mov_b32 s12, 0x6a5dcb37
	s_mov_b32 s9, 0x3e928af3
	s_mov_b32 s13, 0x3e5ade15
	s_mov_b32 s14, 0x623fde64
	s_mov_b32 s15, 0x3ec71dee
	s_mov_b32 s20, 0x7c89e6b0
	s_mov_b32 s21, 0x3efa0199
	s_mov_b32 s22, 0x14761f6e
	s_mov_b32 s23, 0x3f2a01a0
	s_mov_b32 s24, 0x1852b7b0
	s_mov_b32 s25, 0x3f56c16c
	s_mov_b32 s26, 0x11122322
	s_mov_b32 s27, 0x3f811111
	s_mov_b32 s28, 0x555502a1
	s_mov_b32 s29, 0x3fa55555
	s_mov_b32 s30, 0x55555511
	s_mov_b32 s31, 0x3fc55555
	s_mov_b32 s34, 11
	s_mov_b32 s35, 0x3fe00000
	s_mov_b32 s36, 0x968915a9
	s_mov_b32 s38, 0x4222de17
	s_mov_b32 s37, 0x3fba6564
	s_mov_b32 s39, 0x3fbdee67
	v_rcp_f64_e32 v[5:6], v[3:4]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_mul_f64 v[7:8], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], -v[3:4], v[7:8], v[9:10]
	v_div_fmas_f64 v[3:4], v[3:4], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[1:2], v[3:4], v[1:2], -s[4:5]
	s_mov_b32 s5, 0x3c7abc9e
	s_mov_b32 s4, 0x3b39803f
	v_rndne_f64_e32 v[3:4], v[1:2]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[1:2]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[1:2], -v[3:4]
	v_cvt_i32_f64_e32 v9, v[3:4]
	s_and_b32 s6, vcc_lo, exec_lo
	v_mul_f64 v[7:8], v[5:6], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], s[10:11], v[7:8]
	v_fma_f64 v[7:8], v[5:6], s[12:13], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[14:15]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[22:23]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[26:27]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[30:31]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], 1.0
	v_fma_f64 v[3:4], v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v9
	v_readfirstlane_b32 s3, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s6, v3
	s_cselect_b32 s3, s3, 0x7ff00000
	s_and_b32 s7, s2, vcc_lo
	s_and_b32 s7, s7, exec_lo
	s_cselect_b32 s6, s6, 0
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s7, s3, 0
	s_add_i32 s2, s45, 1
	v_cmp_neq_f64_e64 vcc_lo, s[6:7], 1.0
	v_cvt_f64_i32_e32 v[1:2], s2
	s_mov_b32 s3, 0x3fe55555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, 0x3ff00000, v2, vcc_lo
	v_cndmask_b32_e32 v1, 0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[1:2]
	s_and_b32 s2, vcc_lo, exec_lo
	s_cselect_b32 s7, s7, 0x3ff00000
	s_cselect_b32 s6, s6, 0
	s_mov_b32 s2, 0x55555555
	v_frexp_mant_f64_e64 v[3:4], |s[6:7]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[3:4]
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	v_ldexp_f64 v[3:4], v[3:4], v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[3:4], 1.0
	v_add_f64 v[11:12], v[3:4], -1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	v_add_f64 v[13:14], v[5:6], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	v_mul_f64 v[15:16], v[5:6], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[15:16]
	v_fma_f64 v[3:4], v[9:10], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[3:4]
	v_add_f64 v[13:14], v[11:12], -v[5:6]
	v_add_f64 v[15:16], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	v_add_f64 v[3:4], v[15:16], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	v_add_f64 v[3:4], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[13:14], v[3:4]
	v_mul_f64 v[3:4], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[3:4]
	v_add_f64 v[7:8], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	v_fma_f64 v[7:8], v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[3:4], v[3:4]
	v_fma_f64 v[7:8], v[5:6], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[9:10], v[7:8]
	v_fma_f64 v[13:14], v[11:12], s[38:39], s[36:37]
	s_mov_b32 s36, 0x3abe935a
	s_mov_b32 s37, 0x3fbe25e4
	v_add_f64 v[9:10], v[11:12], -v[9:10]
	v_mul_f64 v[19:20], v[5:6], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x47e6c9c2
	s_mov_b32 s37, 0x3fc110ef
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0xcfa74449
	s_mov_b32 s37, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x71bf3c30
	s_mov_b32 s37, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x1c7792ce
	s_mov_b32 s37, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x924920da
	s_mov_b32 s37, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x9999999c
	s_mov_b32 s37, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[11:12], v[13:14]
	v_fma_f64 v[9:10], v[11:12], v[13:14], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	v_add_f64 v[13:14], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[17:18], v[13:14], s[2:3]
	v_add_f64 v[15:16], v[13:14], -v[15:16]
	s_mov_b32 s3, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[21:22], v[17:18], s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_fma_f64 v[15:16], v[11:12], v[5:6], -v[19:20]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_add_f64 v[13:14], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], s[2:3]
	v_fma_f64 v[11:12], v[11:12], v[3:4], v[15:16]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_mov_b32 s2, 0x652b82fe
	s_mov_b32 s3, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[13:14]
	v_fma_f64 v[7:8], v[7:8], v[5:6], v[11:12]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[17:18], v[9:10]
	v_add_f64 v[13:14], v[19:20], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[17:18], -v[11:12]
	v_mul_f64 v[17:18], v[13:14], v[11:12]
	v_add_f64 v[19:20], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[15:16]
	v_fma_f64 v[15:16], v[13:14], v[11:12], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[19:20]
	v_fma_f64 v[9:10], v[13:14], v[9:10], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[7:8], v[11:12], v[9:10]
	v_frexp_exp_i32_f64_e32 v11, s[6:7]
	v_add_f64 v[9:10], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	v_cvt_f64_i32_e32 v[11:12], v11
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[5:6], v[9:10]
	v_add_f64 v[15:16], v[9:10], -v[17:18]
	v_mul_f64 v[17:18], v[11:12], s[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[13:14], -v[5:6]
	v_add_f64 v[7:8], v[7:8], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[11:12], s[10:11], -v[17:18]
	s_mov_b32 s11, 0xbfe62e42
	v_add_f64 v[5:6], v[9:10], -v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_fma_f64 v[7:8], v[11:12], s[4:5], v[15:16]
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], v[3:4]
	v_add_f64 v[17:18], v[5:6], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[5:6], v[9:10]
	v_add_f64 v[13:14], v[9:10], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[11:12], -v[5:6]
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[13:14], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[19:20]
	v_add_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[13:14], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[9:10]
	v_add_f64 v[3:4], v[3:4], -v[9:10]
	v_add_f64 v[15:16], v[11:12], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[9:10], v[15:16], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], -v[15:16]
	v_mul_f64 v[9:10], v[1:2], v[5:6]
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[5:6], v[1:2], v[5:6], -v[9:10]
	v_cmp_class_f64_e64 vcc_lo, v[9:10], 0x204
	v_fma_f64 v[3:4], v[1:2], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[3:4]
	v_dual_cndmask_b32 v8, v6, v10 :: v_dual_cndmask_b32 v7, v5, v9
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[1:2], 0.5
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[11:12], v[7:8], s[2:3]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[7:8]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[7:8]
	v_cmp_neq_f64_e64 s3, 0x7ff00000, |v[7:8]|
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	v_trunc_f64_e32 v[5:6], v[1:2]
	v_rndne_f64_e32 v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v4, 0, v4, s3
	v_cndmask_b32_e64 v3, 0, v3, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[11:12], s[10:11], v[7:8]
	v_cvt_i32_f64_e32 v17, v[11:12]
	v_trunc_f64_e32 v[7:8], v[9:10]
	v_fma_f64 v[13:14], v[11:12], s[4:5], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s3, v[7:8], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[12:13], s[8:9]
	v_cmp_class_f64_e64 s9, s[6:7], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[14:15]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[22:23]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[26:27]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[30:31]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], 1.0
	v_fma_f64 v[11:12], v[13:14], v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[11:12], v[11:12], v17
	v_cndmask_b32_e32 v12, 0x7ff00000, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s4, v11
	v_cndmask_b32_e64 v12, 0, v12, s2
	s_and_b32 s2, s2, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s4, s4, 0
	v_cmp_eq_f64_e64 s2, v[5:6], v[1:2]
	v_mov_b32_e32 v11, s4
	v_fma_f64 v[3:4], v[11:12], v[3:4], v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s5, v3
	s_and_b32 s8, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v4, v12, vcc_lo
	v_cmp_lt_f64_e64 vcc_lo, s[6:7], 0
	v_cmp_eq_f64_e64 s8, s[6:7], 0
	s_cselect_b32 s4, s4, s5
	s_and_b32 s5, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s3, s5, exec_lo
	s_cselect_b32 s3, s7, 0x3ff00000
	v_bfi_b32 v3, 0x7fffffff, v3, s3
	v_cmp_gt_f64_e64 s3, 0, v[1:2]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s2
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, s4, 0
	s_and_b32 s10, vcc_lo, exec_lo
	s_cselect_b32 s4, s2, s4
	v_cndmask_b32_e32 v1, v3, v4, vcc_lo
	s_or_b32 vcc_lo, s8, s9
	s_xor_b32 s2, s3, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, 0, 0x7ff00000
	s_and_b32 s3, s5, exec_lo
	s_cselect_b32 s3, s7, 0
	v_mov_b32_e32 v2, s3
	s_and_b32 s3, vcc_lo, exec_lo
	s_cselect_b32 s3, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_bfi_b32 v2, 0x7fffffff, s2, v2
	v_cmp_o_f64_e64 s2, s[6:7], s[6:7]
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v4, 0x7ff80000, v1, s2
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, s3, 0
	v_mov_b32_e32 v3, s2
.LBB2_5:
	s_mul_i32 s2, s49, s46
	s_xor_b32 s3, s47, s50
	s_sub_i32 s2, s48, s2
	s_add_i32 s4, s49, 1
	s_sub_i32 s5, s2, s46
	s_cmp_ge_u32 s2, s46
	s_load_b256 s[8:15], s[0:1], 0x0
	s_cselect_b32 s4, s4, s49
	s_cselect_b32 s2, s5, s2
	s_add_i32 s5, s4, 1
	s_cmp_ge_u32 s2, s46
	s_mul_i32 s20, s19, s18
	s_cselect_b32 s2, s5, s4
	s_mul_i32 s4, s19, s17
	s_xor_b32 s2, s2, s3
	s_mul_hi_i32 s23, s45, s19
	s_sub_i32 s18, s2, s3
	s_cmp_lt_i32 s16, 1
	v_cmp_gt_i32_e64 s2, s19, v0
	s_cselect_b32 s21, -1, 0
	s_cmp_gt_i32 s16, 0
	v_cmp_eq_u32_e64 s3, 0, v0
	s_cselect_b32 s17, -1, 0
	s_mul_hi_i32 s25, s33, s4
	s_mul_i32 s24, s33, s4
	s_and_b32 vcc_lo, exec_lo, s17
	s_mul_i32 s22, s45, s19
	s_cbranch_vccz .LBB2_23
; %bb.6:
	v_mbcnt_lo_u32_b32 v1, -1, 0
	s_lshl_b64 s[4:5], s[24:25], 2
	s_mul_hi_i32 s7, s18, s19
	s_waitcnt lgkmcnt(0)
	s_add_u32 s6, s8, s4
	s_addc_u32 s9, s9, s5
	v_cmp_gt_u32_e32 vcc_lo, 24, v1
	s_lshl_b64 s[4:5], s[22:23], 2
	v_and_b32_e32 v5, 31, v0
	s_add_u32 s8, s6, s4
	s_mul_i32 s6, s18, s19
	v_cndmask_b32_e64 v2, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v1
	s_addc_u32 s9, s9, s5
	s_lshl_b64 s[4:5], s[6:7], 2
	s_ashr_i32 s26, s20, 31
	v_add_lshl_u32 v7, v2, v1, 2
	v_cndmask_b32_e64 v8, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v1
	v_lshrrev_b32_e32 v2, 3, v0
	s_add_u32 s10, s10, s4
	s_addc_u32 s11, s11, s5
	s_add_u32 s6, s0, 64
	v_cndmask_b32_e64 v9, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v1
	s_addc_u32 s7, s1, 0
	v_lshl_or_b32 v6, v1, 2, 64
	v_add_lshl_u32 v8, v8, v1, 2
	v_add_lshl_u32 v9, v9, v1, 2
	v_add_co_ci_u32_e64 v10, null, 0, v1, vcc_lo
	v_cmp_eq_u32_e64 s4, 0, v5
	v_and_b32_e32 v11, 0x7c, v2
	v_cmp_gt_u32_e64 s5, 32, v0
	s_delay_alu instid0(VALU_DEP_4)
	v_dual_mov_b32 v13, 0 :: v_dual_lshlrev_b32 v10, 2, v10
	v_lshlrev_b32_e32 v12, 2, v5
	s_cmp_lt_i32 s33, s44
	s_mov_b32 s27, 0
	s_cselect_b32 s28, -1, 0
	s_branch .LBB2_9
.LBB2_7:                                ;   in Loop: Header=BB2_9 Depth=1
	s_lshl_b32 s30, s27, 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_addk_i32 s30, 0x100
	v_mov_b32_e32 v1, s30
	ds_store_b32 v1, v2
.LBB2_8:                                ;   in Loop: Header=BB2_9 Depth=1
	s_or_b32 exec_lo, exec_lo, s29
	s_add_i32 s27, s27, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u32 s27, s16
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB2_23
.LBB2_9:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB2_11 Depth 2
	v_mov_b32_e32 v14, 0
	s_and_saveexec_b32 s29, s2
	s_cbranch_execz .LBB2_13
; %bb.10:                               ;   in Loop: Header=BB2_9 Depth=1
	s_load_b32 s34, s[6:7], 0xc
	s_mul_i32 s31, s27, s26
	s_mul_hi_u32 s35, s27, s20
	s_mul_i32 s30, s27, s20
	s_add_i32 s31, s35, s31
	v_dual_mov_b32 v14, 0 :: v_dual_mov_b32 v1, v0
	s_lshl_b64 s[30:31], s[30:31], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s30, s10, s30
	s_addc_u32 s31, s11, s31
	s_waitcnt lgkmcnt(0)
	s_and_b32 s35, s34, 0xffff
	s_mov_b32 s34, 0
	.p2align	6
.LBB2_11:                               ;   Parent Loop BB2_9 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[15:16], 2, v[1:2]
	v_add_co_u32 v17, vcc_lo, s8, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v18, null, s9, v16, vcc_lo
	v_add_co_u32 v15, vcc_lo, s30, v15
	v_add_co_ci_u32_e64 v16, null, s31, v16, vcc_lo
	global_load_b32 v2, v[17:18], off
	global_load_b32 v15, v[15:16], off
	s_waitcnt vmcnt(0)
	v_dual_fmac_f32 v14, v2, v15 :: v_dual_add_nc_u32 v1, s35, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s19, v1
	s_or_b32 s34, vcc_lo, s34
	s_and_not1_b32 exec_lo, exec_lo, s34
	s_cbranch_execnz .LBB2_11
; %bb.12:                               ;   in Loop: Header=BB2_9 Depth=1
	s_or_b32 exec_lo, exec_lo, s34
.LBB2_13:                               ;   in Loop: Header=BB2_9 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s29
	ds_bpermute_b32 v1, v6, v14
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v14, v1
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v9, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v10, v1
	s_and_saveexec_b32 s29, s4
	s_cbranch_execz .LBB2_15
; %bb.14:                               ;   in Loop: Header=BB2_9 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v11, v1
.LBB2_15:                               ;   in Loop: Header=BB2_9 Depth=1
	s_or_b32 exec_lo, exec_lo, s29
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s29, s5
	s_cbranch_execz .LBB2_20
; %bb.16:                               ;   in Loop: Header=BB2_9 Depth=1
	s_load_b32 s30, s[6:7], 0xc
	v_mov_b32_e32 v1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s30, s30, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s30, s30, 31
	s_lshr_b32 s30, s30, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_u32_e32 vcc_lo, s30, v5
	s_and_saveexec_b32 s30, vcc_lo
; %bb.17:                               ;   in Loop: Header=BB2_9 Depth=1
	ds_load_b32 v1, v12
; %bb.18:                               ;   in Loop: Header=BB2_9 Depth=1
	s_or_b32 exec_lo, exec_lo, s30
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v9, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v10, v1
	s_and_b32 exec_lo, exec_lo, s4
	s_cbranch_execz .LBB2_20
; %bb.19:                               ;   in Loop: Header=BB2_9 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v13, v1
.LBB2_20:                               ;   in Loop: Header=BB2_9 Depth=1
	s_or_b32 exec_lo, exec_lo, s29
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v1, v13
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s29, s3
	s_cbranch_execz .LBB2_8
; %bb.21:                               ;   in Loop: Header=BB2_9 Depth=1
	s_cmp_lt_i32 s33, s27
	v_mov_b32_e32 v2, 0xf149f2ca
	s_cselect_b32 s30, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s30, s28, s30
	s_and_b32 vcc_lo, exec_lo, s30
	s_cbranch_vccnz .LBB2_7
; %bb.22:                               ;   in Loop: Header=BB2_9 Depth=1
	s_sub_i32 s30, s33, s27
	v_cvt_f64_f32_e32 v[1:2], v1
	v_cvt_f64_i32_e32 v[14:15], s30
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[1:2], -v[3:4], v[14:15], v[1:2]
	v_cvt_f32_f64_e32 v2, v[1:2]
	s_branch .LBB2_7
.LBB2_23:
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_xor_b32 s3, s21, -1
	s_mov_b32 s2, 0
	s_and_b32 s3, vcc_lo, s3
	s_waitcnt lgkmcnt(0)
	s_and_saveexec_b32 s9, s3
	s_cbranch_execz .LBB2_43
; %bb.24:
	v_mov_b32_e32 v2, 0xff800000
	s_add_i32 s3, s16, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lt_u32 s3, 7
	s_cbranch_scc1 .LBB2_27
; %bb.25:
	s_and_b32 s2, s16, 0x7ffffff8
	s_mov_b32 s3, 0
	s_movk_i32 s4, 0x100
	.p2align	6
.LBB2_26:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v1, s4
	s_add_i32 s3, s3, 8
	s_add_i32 s4, s4, 32
	s_cmp_eq_u32 s2, s3
	ds_load_2addr_b32 v[3:4], v1 offset1:1
	ds_load_2addr_b32 v[5:6], v1 offset0:2 offset1:3
	ds_load_2addr_b32 v[7:8], v1 offset0:4 offset1:5
	ds_load_2addr_b32 v[9:10], v1 offset0:6 offset1:7
	s_waitcnt lgkmcnt(3)
	v_cmp_gt_f32_e32 vcc_lo, v3, v2
	v_cndmask_b32_e32 v1, v2, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, v4, v1
	v_cndmask_b32_e32 v1, v1, v4, vcc_lo
	s_waitcnt lgkmcnt(2)
	v_cmp_gt_f32_e32 vcc_lo, v5, v1
	v_cndmask_b32_e32 v1, v1, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, v6, v1
	v_cndmask_b32_e32 v1, v1, v6, vcc_lo
	s_waitcnt lgkmcnt(1)
	v_cmp_gt_f32_e32 vcc_lo, v7, v1
	v_cndmask_b32_e32 v1, v1, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, v8, v1
	v_cndmask_b32_e32 v1, v1, v8, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f32_e32 vcc_lo, v9, v1
	v_cndmask_b32_e32 v1, v1, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, v10, v1
	v_cndmask_b32_e32 v2, v1, v10, vcc_lo
	s_cbranch_scc0 .LBB2_26
.LBB2_27:
	s_and_b32 s10, s16, 7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lg_u32 s10, 0
	s_cselect_b32 s3, -1, 0
	s_cmp_eq_u32 s10, 0
	s_cbranch_scc1 .LBB2_30
; %bb.28:
	s_lshl_b32 s2, s2, 2
	s_mov_b32 s4, s10
	s_addk_i32 s2, 0x100
.LBB2_29:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v1, s2
	s_add_i32 s4, s4, -1
	s_add_i32 s2, s2, 4
	s_cmp_lg_u32 s4, 0
	ds_load_b32 v1, v1
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f32_e32 vcc_lo, v1, v2
	v_cndmask_b32_e32 v2, v2, v1, vcc_lo
	s_cbranch_scc1 .LBB2_29
.LBB2_30:
	v_mov_b32_e32 v1, 0
	s_cmp_gt_u32 s16, 7
	s_cselect_b32 s2, -1, 0
	s_cmp_lt_u32 s16, 8
	s_cbranch_scc1 .LBB2_34
; %bb.31:
	s_and_b32 s4, s16, 0x7ffffff8
	s_mov_b32 s5, 0
	s_movk_i32 s6, 0x100
.LBB2_32:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v11, s6
	s_add_i32 s5, s5, 8
	s_add_i32 s6, s6, 32
	s_cmp_lg_u32 s4, s5
	ds_load_2addr_b32 v[3:4], v11 offset1:1
	ds_load_2addr_b32 v[5:6], v11 offset0:2 offset1:3
	ds_load_2addr_b32 v[7:8], v11 offset0:4 offset1:5
	ds_load_2addr_b32 v[9:10], v11 offset0:6 offset1:7
	s_waitcnt lgkmcnt(3)
	v_sub_f32_e32 v4, v4, v2
	s_waitcnt lgkmcnt(2)
	v_sub_f32_e32 v5, v5, v2
	v_sub_f32_e32 v3, v3, v2
	v_sub_f32_e32 v6, v6, v2
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e32 v9, v9, v2
	v_dual_mul_f32 v13, 0x3fb8aa3b, v4 :: v_dual_mul_f32 v14, 0x3fb8aa3b, v5
	v_dual_mul_f32 v12, 0x3fb8aa3b, v3 :: v_dual_sub_f32 v7, v7, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v18, 0x3fb8aa3b, v9
	v_rndne_f32_e32 v23, v13
	v_sub_f32_e32 v8, v8, v2
	s_delay_alu instid0(VALU_DEP_4)
	v_rndne_f32_e32 v21, v12
	v_mul_f32_e32 v15, 0x3fb8aa3b, v6
	v_fma_f32 v22, 0x3fb8aa3b, v4, -v13
	v_dual_sub_f32 v13, v13, v23 :: v_dual_sub_f32 v10, v10, v2
	v_dual_mul_f32 v17, 0x3fb8aa3b, v8 :: v_dual_mul_f32 v16, 0x3fb8aa3b, v7
	v_fma_f32 v20, 0x3fb8aa3b, v3, -v12
	v_rndne_f32_e32 v25, v14
	s_delay_alu instid0(VALU_DEP_4)
	v_dual_mul_f32 v19, 0x3fb8aa3b, v10 :: v_dual_sub_f32 v12, v12, v21
	v_fma_f32 v26, 0x3fb8aa3b, v6, -v15
	v_rndne_f32_e32 v27, v15
	v_fma_f32 v24, 0x3fb8aa3b, v5, -v14
	v_fma_f32 v28, 0x3fb8aa3b, v7, -v16
	v_fmac_f32_e32 v22, 0x32a5705f, v4
	v_sub_f32_e32 v14, v14, v25
	v_rndne_f32_e32 v35, v19
	v_dual_fmac_f32 v26, 0x32a5705f, v6 :: v_dual_sub_f32 v15, v15, v27
	v_fma_f32 v34, 0x3fb8aa3b, v10, -v19
	v_dual_fmac_f32 v20, 0x32a5705f, v3 :: v_dual_add_f32 v13, v13, v22
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_sub_f32_e32 v19, v19, v35
	v_dual_fmac_f32 v28, 0x32a5705f, v7 :: v_dual_add_f32 v15, v15, v26
	v_cvt_i32_f32_e32 v27, v27
	v_cvt_i32_f32_e32 v21, v21
	v_exp_f32_e32 v13, v13
	v_cvt_i32_f32_e32 v23, v23
	v_exp_f32_e32 v15, v15
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v3
	v_fma_f32 v30, 0x3fb8aa3b, v8, -v17
	v_rndne_f32_e32 v31, v17
	v_fmac_f32_e32 v24, 0x32a5705f, v5
	v_rndne_f32_e32 v29, v16
	v_rndne_f32_e32 v33, v18
	v_cvt_i32_f32_e32 v25, v25
	v_ldexp_f32 v13, v13, v23
	v_sub_f32_e32 v17, v17, v31
	v_ldexp_f32 v15, v15, v27
	v_add_f32_e32 v12, v12, v20
	v_add_f32_e32 v14, v14, v24
	v_fma_f32 v32, 0x3fb8aa3b, v9, -v18
	v_cvt_i32_f32_e32 v31, v31
	v_cvt_i32_f32_e32 v35, v35
	v_exp_f32_e32 v12, v12
	v_exp_f32_e32 v14, v14
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v12, v12, v21
	v_ldexp_f32 v14, v14, v25
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v12, 0, v12, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v4
	v_dual_fmac_f32 v34, 0x32a5705f, v10 :: v_dual_cndmask_b32 v13, 0, v13
	v_dual_fmac_f32 v30, 0x32a5705f, v8 :: v_dual_add_f32 v19, v19, v34
	v_sub_f32_e32 v16, v16, v29
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v3
	s_delay_alu instid0(VALU_DEP_3)
	v_dual_sub_f32 v18, v18, v33 :: v_dual_add_f32 v17, v17, v30
	v_cvt_i32_f32_e32 v29, v29
	v_exp_f32_e32 v19, v19
	v_cndmask_b32_e32 v3, 0x7f800000, v12, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v5
	v_exp_f32_e32 v17, v17
	v_fmac_f32_e32 v32, 0x32a5705f, v9
	v_cvt_i32_f32_e32 v33, v33
	v_dual_add_f32 v1, v1, v3 :: v_dual_cndmask_b32 v12, 0, v14
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v4
	s_delay_alu instid0(TRANS32_DEP_2)
	v_ldexp_f32 v19, v19, v35
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v17, v17, v31
	v_add_f32_e32 v16, v16, v28
	v_cndmask_b32_e32 v4, 0x7f800000, v13, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_exp_f32_e32 v16, v16
	v_add_f32_e32 v1, v1, v4
	v_cndmask_b32_e32 v13, 0, v15, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v5
	v_cndmask_b32_e32 v5, 0x7f800000, v12, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v7
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v16, v16, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_dual_add_f32 v1, v1, v5 :: v_dual_cndmask_b32 v12, 0, v16
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v6
	v_cndmask_b32_e32 v6, 0x7f800000, v13, vcc_lo
	v_add_f32_e32 v18, v18, v32
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v8
	v_add_f32_e32 v1, v1, v6
	s_delay_alu instid0(VALU_DEP_3)
	v_exp_f32_e32 v18, v18
	v_cndmask_b32_e32 v13, 0, v17, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v7
	v_cndmask_b32_e32 v7, 0x7f800000, v12, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v9
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v18, v18, v33
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_dual_add_f32 v1, v1, v7 :: v_dual_cndmask_b32 v12, 0, v18
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v8
	v_cndmask_b32_e32 v8, 0x7f800000, v13, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v10
	v_add_f32_e32 v1, v1, v8
	v_cndmask_b32_e32 v13, 0, v19, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v9
	v_cndmask_b32_e32 v9, 0x7f800000, v12, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f32_e32 v1, v1, v9
	v_cndmask_b32_e32 v10, 0x7f800000, v13, vcc_lo
	v_add_f32_e32 v1, v1, v10
	ds_store_2addr_b32 v11, v3, v4 offset1:1
	ds_store_2addr_b32 v11, v5, v6 offset0:2 offset1:3
	ds_store_2addr_b32 v11, v7, v8 offset0:4 offset1:5
	ds_store_2addr_b32 v11, v9, v10 offset0:6 offset1:7
	s_cbranch_scc1 .LBB2_32
; %bb.33:
	v_cndmask_b32_e64 v3, 0, 1, s3
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccz .LBB2_35
	s_branch .LBB2_37
.LBB2_34:
	s_mov_b32 s4, 0
	v_cndmask_b32_e64 v3, 0, 1, s3
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccnz .LBB2_37
.LBB2_35:
	s_lshl_b32 s3, s4, 2
	s_mov_b32 s4, s10
	s_addk_i32 s3, 0x100
	.p2align	6
.LBB2_36:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v4, s3
	s_add_i32 s4, s4, -1
	s_add_i32 s3, s3, 4
	s_cmp_lg_u32 s4, 0
	ds_load_b32 v5, v4
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e32 v5, v5, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v6, 0x3fb8aa3b, v5
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v5
	v_fma_f32 v7, 0x3fb8aa3b, v5, -v6
	v_rndne_f32_e32 v8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v7, 0x32a5705f, v5 :: v_dual_sub_f32 v6, v6, v8
	v_add_f32_e32 v6, v6, v7
	v_cvt_i32_f32_e32 v7, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v6, v6
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v6, v6, v7
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0x7f800000, v6, vcc_lo
	v_add_f32_e32 v1, v1, v5
	ds_store_b32 v4, v5
	s_cbranch_scc1 .LBB2_36
.LBB2_37:
	s_and_not1_b32 vcc_lo, exec_lo, s2
	s_mov_b32 s11, 0
	s_cbranch_vccnz .LBB2_40
; %bb.38:
	s_and_b32 s11, s16, 0x7ffffff8
	s_mov_b32 s21, 0
	s_movk_i32 s26, 0x100
.LBB2_39:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v2, s26
	s_add_i32 s21, s21, 8
	s_add_i32 s26, s26, 32
	s_cmp_lg_u32 s11, s21
	ds_load_2addr_b32 v[4:5], v2 offset1:1
	ds_load_2addr_b32 v[6:7], v2 offset0:2 offset1:3
	ds_load_2addr_b32 v[8:9], v2 offset0:4 offset1:5
	ds_load_2addr_b32 v[10:11], v2 offset0:6 offset1:7
	s_waitcnt lgkmcnt(3)
	v_div_scale_f32 v12, null, v1, v1, v4
	v_div_scale_f32 v14, null, v1, v1, v5
	s_waitcnt lgkmcnt(2)
	v_div_scale_f32 v16, null, v1, v1, v6
	v_div_scale_f32 v18, null, v1, v1, v7
	v_rcp_f32_e32 v28, v12
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v24, null, v1, v1, v10
	v_rcp_f32_e32 v29, v14
	v_div_scale_f32 v20, null, v1, v1, v8
	v_div_scale_f32 v22, null, v1, v1, v9
	v_rcp_f32_e32 v30, v16
	v_rcp_f32_e32 v31, v18
	v_rcp_f32_e32 v34, v24
	v_div_scale_f32 v26, null, v1, v1, v11
	v_rcp_f32_e32 v32, v20
	v_rcp_f32_e32 v33, v22
	v_fma_f32 v36, -v12, v28, 1.0
	v_fma_f32 v37, -v14, v29, 1.0
	v_rcp_f32_e32 v35, v26
	v_fma_f32 v38, -v16, v30, 1.0
	v_div_scale_f32 v13, vcc_lo, v4, v1, v4
	v_fma_f32 v39, -v18, v31, 1.0
	v_fmac_f32_e32 v28, v36, v28
	v_div_scale_f32 v15, s2, v5, v1, v5
	v_fma_f32 v42, -v24, v34, 1.0
	v_fmac_f32_e32 v29, v37, v29
	v_fma_f32 v40, -v20, v32, 1.0
	v_div_scale_f32 v17, s3, v6, v1, v6
	v_fma_f32 v41, -v22, v33, 1.0
	v_fmac_f32_e32 v30, v38, v30
	v_div_scale_f32 v19, s4, v7, v1, v7
	v_dual_fmac_f32 v31, v39, v31 :: v_dual_mul_f32 v36, v13, v28
	v_dual_fmac_f32 v34, v42, v34 :: v_dual_mul_f32 v37, v15, v29
	v_div_scale_f32 v21, s5, v8, v1, v8
	v_fma_f32 v43, -v26, v35, 1.0
	v_fmac_f32_e32 v32, v40, v32
	v_div_scale_f32 v23, s6, v9, v1, v9
	v_fmac_f32_e32 v33, v41, v33
	v_dual_mul_f32 v38, v17, v30 :: v_dual_mul_f32 v39, v19, v31
	v_fma_f32 v44, -v12, v36, v13
	v_div_scale_f32 v25, s7, v10, v1, v10
	v_fma_f32 v45, -v14, v37, v15
	v_div_scale_f32 v27, s8, v11, v1, v11
	v_dual_fmac_f32 v35, v43, v35 :: v_dual_mul_f32 v40, v21, v32
	v_mul_f32_e32 v41, v23, v33
	v_fma_f32 v46, -v16, v38, v17
	v_dual_fmac_f32 v36, v44, v28 :: v_dual_fmac_f32 v37, v45, v29
	v_fma_f32 v47, -v18, v39, v19
	v_dual_mul_f32 v42, v25, v34 :: v_dual_mul_f32 v43, v27, v35
	v_fma_f32 v48, -v20, v40, v21
	v_fma_f32 v49, -v22, v41, v23
	v_fmac_f32_e32 v38, v46, v30
	v_fma_f32 v12, -v12, v36, v13
	v_fmac_f32_e32 v39, v47, v31
	v_fma_f32 v50, -v24, v42, v25
	v_fma_f32 v13, -v14, v37, v15
	v_fma_f32 v51, -v26, v43, v27
	v_dual_fmac_f32 v40, v48, v32 :: v_dual_fmac_f32 v41, v49, v33
	v_fma_f32 v14, -v16, v38, v17
	v_div_fmas_f32 v12, v12, v28, v36
	s_mov_b32 vcc_lo, s2
	v_fma_f32 v15, -v18, v39, v19
	v_fmac_f32_e32 v42, v50, v34
	v_div_fmas_f32 v13, v13, v29, v37
	s_mov_b32 vcc_lo, s3
	v_fmac_f32_e32 v43, v51, v35
	v_fma_f32 v16, -v20, v40, v21
	v_div_fmas_f32 v14, v14, v30, v38
	s_mov_b32 vcc_lo, s4
	v_fma_f32 v17, -v22, v41, v23
	v_div_fixup_f32 v4, v12, v1, v4
	v_div_fmas_f32 v12, v15, v31, v39
	s_mov_b32 vcc_lo, s5
	v_fma_f32 v18, -v24, v42, v25
	v_div_fixup_f32 v5, v13, v1, v5
	v_div_fmas_f32 v13, v16, v32, v40
	s_mov_b32 vcc_lo, s6
	v_fma_f32 v19, -v26, v43, v27
	v_div_fixup_f32 v6, v14, v1, v6
	v_div_fmas_f32 v14, v17, v33, v41
	s_mov_b32 vcc_lo, s7
	v_div_fixup_f32 v7, v12, v1, v7
	v_div_fmas_f32 v15, v18, v34, v42
	s_mov_b32 vcc_lo, s8
	v_div_fixup_f32 v8, v13, v1, v8
	v_div_fmas_f32 v16, v19, v35, v43
	v_div_fixup_f32 v9, v14, v1, v9
	v_div_fixup_f32 v10, v15, v1, v10
	s_delay_alu instid0(VALU_DEP_3)
	v_div_fixup_f32 v11, v16, v1, v11
	ds_store_2addr_b32 v2, v4, v5 offset1:1
	ds_store_2addr_b32 v2, v6, v7 offset0:2 offset1:3
	ds_store_2addr_b32 v2, v8, v9 offset0:4 offset1:5
	ds_store_2addr_b32 v2, v10, v11 offset0:6 offset1:7
	s_cbranch_scc1 .LBB2_39
.LBB2_40:
	v_cmp_ne_u32_e32 vcc_lo, 1, v3
	s_cbranch_vccnz .LBB2_43
; %bb.41:
	s_lshl_b32 s2, s11, 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s2, 0x100
	.p2align	6
.LBB2_42:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v2, s2
	s_add_i32 s10, s10, -1
	s_add_i32 s2, s2, 4
	s_cmp_lg_u32 s10, 0
	ds_load_b32 v3, v2
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v4, null, v1, v1, v3
	v_div_scale_f32 v7, vcc_lo, v3, v1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v5, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v6, -v4, v5, 1.0
	v_fmac_f32_e32 v5, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v6, v7, v5
	v_fma_f32 v8, -v4, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v8, v5
	v_fma_f32 v4, -v4, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v4, v4, v5, v6
	v_div_fixup_f32 v3, v4, v1, v3
	ds_store_b32 v2, v3
	s_cbranch_scc1 .LBB2_42
.LBB2_43:
	s_or_b32 exec_lo, exec_lo, s9
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s19, v0
	s_cbranch_execz .LBB2_54
; %bb.44:
	s_load_b32 s0, s[0:1], 0x4c
	s_lshl_b64 s[2:3], s[24:25], 2
	s_mul_hi_i32 s5, s19, s18
	s_add_u32 s6, s14, s2
	s_addc_u32 s7, s15, s3
	s_lshl_b64 s[2:3], s[22:23], 2
	s_mul_i32 s4, s19, s18
	s_add_u32 s10, s6, s2
	s_addc_u32 s11, s7, s3
	s_ashr_i32 s21, s20, 31
	s_and_b32 s14, s16, 3
	s_mov_b32 s1, 0
	s_mul_hi_i32 s8, s20, 12
	s_mul_i32 s9, s20, 12
	s_waitcnt lgkmcnt(0)
	s_and_b32 s15, s0, 0xffff
	s_cmp_gt_u32 s16, 3
	s_cselect_b32 s18, -1, 0
	s_and_b32 s16, s16, 0x7ffffffc
	s_cmp_lg_u32 s14, 0
	s_cselect_b32 s22, -1, 0
	s_lshl_b64 s[2:3], s[4:5], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s12, s12, s2
	s_addc_u32 s13, s13, s3
	s_lshl_b64 s[2:3], s[20:21], 4
	s_lshl_b64 s[4:5], s[20:21], 3
	s_lshl_b64 s[6:7], s[20:21], 2
	s_branch .LBB2_46
.LBB2_45:                               ;   in Loop: Header=BB2_46 Depth=1
	v_add_nc_u32_e32 v0, s15, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, s0, s10, v1
	v_add_co_ci_u32_e64 v2, null, s11, v2, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s19, v0
	global_store_b32 v[1:2], v5, off
	s_or_b32 s1, vcc_lo, s1
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB2_54
.LBB2_46:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB2_49 Depth 2
                                        ;     Child Loop BB2_53 Depth 2
	v_ashrrev_i32_e32 v1, 31, v0
	v_mov_b32_e32 v5, 0
	s_and_not1_b32 vcc_lo, exec_lo, s17
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b64 v[1:2], 2, v[0:1]
	s_cbranch_vccnz .LBB2_45
; %bb.47:                               ;   in Loop: Header=BB2_46 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s18
	s_cbranch_vccnz .LBB2_51
; %bb.48:                               ;   in Loop: Header=BB2_46 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s12, v1
	v_add_co_ci_u32_e64 v4, null, s13, v2, vcc_lo
	v_mov_b32_e32 v5, 0
	s_mov_b32 s0, 0
	s_movk_i32 s20, 0x100
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB2_49:                               ;   Parent Loop BB2_46 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_add_co_u32 v6, vcc_lo, v3, s6
	global_load_b32 v10, v[3:4], off
	v_add_co_ci_u32_e64 v7, null, s7, v4, vcc_lo
	v_add_co_u32 v8, vcc_lo, v3, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s5, v4, vcc_lo
	global_load_b32 v11, v[6:7], off
	v_add_co_u32 v6, vcc_lo, v3, s9
	v_add_co_ci_u32_e64 v7, null, s8, v4, vcc_lo
	s_clause 0x1
	global_load_b32 v12, v[8:9], off
	global_load_b32 v13, v[6:7], off
	v_mov_b32_e32 v8, s20
	ds_load_2addr_b32 v[6:7], v8 offset1:1
	ds_load_2addr_b32 v[8:9], v8 offset0:2 offset1:3
	v_add_co_u32 v3, vcc_lo, v3, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s3, v4, vcc_lo
	s_add_i32 s0, s0, 4
	s_add_i32 s20, s20, 16
	s_cmp_eq_u32 s16, s0
	s_waitcnt vmcnt(3) lgkmcnt(1)
	v_fmac_f32_e32 v5, v6, v10
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v5, v7, v11
	s_waitcnt vmcnt(1) lgkmcnt(0)
	v_fmac_f32_e32 v5, v8, v12
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v5, v9, v13
	s_cbranch_scc0 .LBB2_49
; %bb.50:                               ;   in Loop: Header=BB2_46 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s20, s16
	s_and_not1_b32 vcc_lo, exec_lo, s22
	s_cbranch_vccz .LBB2_52
	s_branch .LBB2_45
.LBB2_51:                               ;   in Loop: Header=BB2_46 Depth=1
	v_mov_b32_e32 v5, 0
	s_mov_b32 s20, 0
	s_and_not1_b32 vcc_lo, exec_lo, s22
	s_cbranch_vccnz .LBB2_45
.LBB2_52:                               ;   in Loop: Header=BB2_46 Depth=1
	s_lshl_b32 s0, s20, 2
	s_mul_i32 s21, s7, s20
	s_mul_hi_u32 s23, s6, s20
	s_mul_i32 s20, s6, s20
	s_addk_i32 s0, 0x100
	s_add_i32 s23, s23, s21
	s_add_u32 s20, s12, s20
	s_addc_u32 s21, s13, s23
	v_add_co_u32 v3, vcc_lo, s20, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s21, v2, vcc_lo
	s_mov_b32 s20, s14
.LBB2_53:                               ;   Parent Loop BB2_46 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b32 v6, v[3:4], off
	v_mov_b32_e32 v7, s0
	v_add_co_u32 v3, vcc_lo, v3, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s7, v4, vcc_lo
	ds_load_b32 v7, v7
	s_add_i32 s20, s20, -1
	s_add_i32 s0, s0, 4
	s_cmp_lg_u32 s20, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fmac_f32_e32 v5, v7, v6
	s_cbranch_scc1 .LBB2_53
	s_branch .LBB2_45
.LBB2_54:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid
		.amdhsa_group_segment_fixed_size 256
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
		.amdhsa_next_free_vgpr 52
		.amdhsa_next_free_sgpr 56
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
	.section	.text._Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid,"axG",@progbits,_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid,comdat
.Lfunc_end2:
	.size	_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid, .Lfunc_end2-_Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid
                                        ; -- End function
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.num_vgpr, 52
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.num_agpr, 0
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.numbered_sgpr, 56
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.num_named_barrier, 0
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.private_seg_size, 0
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.uses_vcc, 1
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.uses_flat_scratch, 0
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.has_dyn_sized_stack, 0
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.has_recursion, 0
	.set _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 10072
; TotalNumSgprs: 58
; NumVgprs: 52
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 6
; NumSGPRsForWavesPerEU: 58
; NumVGPRsForWavesPerEU: 52
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid,"axG",@progbits,_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid,comdat
	.protected	_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid ; -- Begin function _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid
	.globl	_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid
	.p2align	8
	.type	_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid,@function
_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid: ; @_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid
; %bb.0:
	s_clause 0x1
	s_load_b128 s[12:15], s[0:1], 0x20
	s_load_b32 s44, s[0:1], 0x30
	v_mov_b32_e32 v3, 0
	s_abs_i32 s6, s2
	v_mov_b32_e32 v4, 0
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
	s_xor_b32 s9, s5, s7
	s_mul_i32 s8, s4, s3
	s_sub_i32 s5, s6, s8
	s_add_i32 s6, s4, 1
	s_sub_i32 s8, s5, s3
	s_cmp_ge_u32 s5, s3
	s_cselect_b32 s4, s6, s4
	s_cselect_b32 s5, s8, s5
	s_add_i32 s6, s4, 1
	s_cmp_ge_u32 s5, s3
	s_cselect_b32 s6, s6, s4
	s_abs_i32 s8, s14
	s_xor_b32 s6, s6, s9
	v_cvt_f32_u32_e32 v1, s8
	s_sub_i32 s11, 0, s8
	s_load_b64 s[4:5], s[0:1], 0x38
	s_sub_i32 s33, s6, s9
	s_ashr_i32 s16, s14, 31
	v_rcp_iflag_f32_e32 v1, v1
	s_mul_i32 s9, s33, s13
	s_xor_b32 s7, s7, s16
	s_sub_i32 s45, s2, s9
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_u32_f32_e32 v1, v1
	v_readfirstlane_b32 s10, v1
	s_mul_i32 s11, s11, s10
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s11, s10, s11
	s_add_i32 s10, s10, s11
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_hi_u32 s6, s3, s10
	s_mul_i32 s10, s6, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	s_sub_i32 s2, s3, s10
	s_add_i32 s3, s6, 1
	s_sub_i32 s9, s2, s8
	s_cmp_ge_u32 s2, s8
	s_cselect_b32 s3, s3, s6
	s_cselect_b32 s2, s9, s2
	s_add_i32 s6, s3, 1
	s_cmp_ge_u32 s2, s8
	s_cselect_b32 s2, s6, s3
	s_waitcnt lgkmcnt(0)
	v_cmp_ngt_f64_e64 s3, s[4:5], 0
	s_xor_b32 s2, s2, s7
	s_abs_i32 s48, s45
	s_sub_i32 s2, s2, s7
	s_ashr_i32 s47, s45, 31
	s_abs_i32 s46, s2
	s_ashr_i32 s50, s2, 31
	v_cvt_f32_u32_e32 v1, s46
	s_sub_i32 s7, 0, s46
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v1, v1
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v1, 0x4f7ffffe, v1
	v_cvt_u32_f32_e32 v1, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s6, v1
	s_mul_i32 s7, s7, s6
	s_and_b32 vcc_lo, exec_lo, s3
	s_mul_hi_u32 s7, s6, s7
	s_add_i32 s6, s6, s7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_hi_u32 s49, s48, s6
	s_cbranch_vccnz .LBB3_5
; %bb.1:
	v_cvt_f64_i32_e32 v[1:2], s13
	s_mov_b32 s9, 0x3fe55555
	s_mov_b32 s8, 0x55555555
	s_mov_b32 s2, 0x6b47b09a
	s_mov_b32 s6, 0xbf559e2b
	s_mov_b32 s3, 0x3fc38538
	s_mov_b32 s7, 0x3fc3ab76
	s_mov_b32 s41, 0x3fd99999
	s_mov_b32 s40, 0x998ef7b6
	s_mov_b32 s10, 0xffda0d24
	s_mov_b32 s11, 0x3c7777d0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e32 v[3:4], v[1:2]
	v_cmp_gt_f64_e32 vcc_lo, s[8:9], v[3:4]
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v5
	v_add_f64 v[5:6], v[3:4], 1.0
	v_add_f64 v[11:12], v[3:4], -1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[7:8], v[5:6]
	v_add_f64 v[13:14], v[5:6], -1.0
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[5:6], v[9:10]
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], v[9:10], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[11:12], -v[5:6]
	v_add_f64 v[15:16], v[5:6], -v[15:16]
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[15:16], -v[3:4]
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[3:4], v[13:14], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[3:4], v[7:8], v[3:4]
	v_add_f64 v[5:6], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[5:6], v[5:6]
	v_fma_f64 v[11:12], v[7:8], s[6:7], s[2:3]
	s_mov_b32 s2, 0xd7f4df2e
	s_mov_b32 s3, 0x3fc7474d
	s_mov_b32 s6, 0x55555780
	s_mov_b32 s7, s9
	v_mul_f64 v[13:14], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[2:3]
	s_mov_b32 s2, 0x16291751
	s_mov_b32 s3, 0x3fcc71c0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[2:3]
	s_mov_b32 s3, 0x3fd24924
	s_mov_b32 s2, 0x9b27acf1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[7:8], v[11:12], s[40:41]
	v_fma_f64 v[7:8], v[7:8], v[11:12], s[6:7]
	v_ldexp_f64 v[11:12], v[5:6], 1
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[7:8], v[13:14], v[7:8]
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[11:12], v[7:8]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], -v[11:12]
	v_add_f64 v[5:6], v[7:8], -v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[9:10], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], s[6:7]
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], s[6:7], -v[9:10]
	v_fma_f64 v[3:4], v[3:4], s[6:7], v[7:8]
	v_frexp_exp_i32_f64_e32 v7, v[1:2]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[1:2], v[5:6], s[10:11], v[3:4]
	v_subrev_co_ci_u32_e64 v3, null, 0, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cvt_f64_i32_e32 v[3:4], v3
	v_add_f64 v[5:6], v[9:10], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], v[3:4]
	v_add_f64 v[9:10], v[5:6], -v[9:10]
	v_add_f64 v[11:12], v[7:8], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[1:2], v[1:2], -v[9:10]
	v_add_f64 v[13:14], v[11:12], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[13:14], v[3:4]
	v_add_f64 v[3:4], v[5:6], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_f64 v[1:2], v[7:8], v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_floor_f64_e32 v[1:2], v[1:2]
	v_cvt_i32_f64_e32 v1, v[1:2]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s2, v1
	s_lshl_b32 s22, 1, s2
	v_cvt_f64_i32_e32 v[1:2], s22
	s_cmp_ge_i32 s45, s22
	s_cbranch_scc0 .LBB3_3
; %bb.2:
	v_mul_f64 v[3:4], s[4:5], -0.5
	s_mov_b32 s11, 0x3c7abc9e
	s_mov_b32 s10, 0x3b39803f
	s_mov_b32 s19, 0x3fe62e42
	s_mov_b32 s18, 0xfefa39ef
	s_mov_b32 s16, 0xfca7ab0c
	s_mov_b32 s20, 0x6a5dcb37
	s_mov_b32 s17, 0x3e928af3
	s_mov_b32 s21, 0x3e5ade15
	s_mov_b32 s24, 0x623fde64
	s_mov_b32 s25, 0x3ec71dee
	s_mov_b32 s26, 0x7c89e6b0
	s_mov_b32 s27, 0x3efa0199
	s_mov_b32 s28, 0x14761f6e
	s_mov_b32 s29, 0x3f2a01a0
	s_mov_b32 s30, 0x1852b7b0
	s_mov_b32 s31, 0x3f56c16c
	s_mov_b32 s34, 0x11122322
	s_mov_b32 s35, 0x3f811111
	s_mov_b32 s36, 0x555502a1
	s_mov_b32 s37, 0x3fa55555
	s_mov_b32 s38, 0x55555511
	s_mov_b32 s39, 0x3fc55555
	s_mov_b32 s42, 11
	s_mov_b32 s43, 0x3fe00000
	s_mov_b32 s54, 0x4222de17
	s_mov_b32 s55, 0x3fbdee67
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_div_scale_f64 v[5:6], null, v[1:2], v[1:2], v[3:4]
	v_div_scale_f64 v[11:12], vcc_lo, v[3:4], v[1:2], v[3:4]
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[9:10], v[11:12]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[5:6], v[1:2], v[3:4]
	v_rndne_f64_e32 v[5:6], v[3:4]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[3:4]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[3:4], -v[5:6]
	v_cvt_i32_f64_e32 v11, v[5:6]
	s_and_b32 s40, vcc_lo, exec_lo
	v_mul_f64 v[9:10], v[7:8], s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], s[18:19], v[9:10]
	v_fma_f64 v[9:10], v[7:8], s[20:21], s[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[24:25]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[28:29]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[34:35]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[38:39]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], 1.0
	v_fma_f64 v[5:6], v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[5:6], v[5:6], v11
	v_readfirstlane_b32 s23, v6
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s40, v5
	s_cselect_b32 s23, s23, 0x7ff00000
	s_and_b32 s51, s2, vcc_lo
	s_and_b32 s51, s51, exec_lo
	s_cselect_b32 s52, s40, 0
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s53, s23, 0
	s_sub_i32 s2, s45, s22
	v_cmp_neq_f64_e64 vcc_lo, s[52:53], 1.0
	s_lshl_b32 s2, s2, 1
	s_mov_b32 s40, 0x9999999c
	s_or_b32 s2, s2, 1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f64_u32_e32 v[3:4], s2
	v_cndmask_b32_e32 v4, 0x3ff00000, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, 0, v3, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[3:4]
	s_and_b32 s2, vcc_lo, exec_lo
	s_cselect_b32 s23, s53, 0x3ff00000
	s_cselect_b32 s22, s52, 0
	s_mov_b32 s52, 0x968915a9
	v_frexp_mant_f64_e64 v[5:6], |s[22:23]|
	s_mov_b32 s53, 0x3fba6564
	s_mov_b32 s2, 0x924920da
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[8:9], v[5:6]
	v_cndmask_b32_e64 v7, 0, 1, vcc_lo
	v_ldexp_f64 v[5:6], v[5:6], v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], 1.0
	v_add_f64 v[13:14], v[5:6], -1.0
	v_rcp_f64_e32 v[9:10], v[7:8]
	v_add_f64 v[15:16], v[7:8], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], -v[7:8], v[9:10], 1.0
	v_fma_f64 v[9:10], v[11:12], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[11:12], v[13:14], v[9:10]
	v_mul_f64 v[17:18], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[11:12], v[7:8], -v[17:18]
	v_fma_f64 v[5:6], v[11:12], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[17:18], v[5:6]
	v_add_f64 v[15:16], v[13:14], -v[7:8]
	v_add_f64 v[17:18], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[13:14], v[13:14], -v[15:16]
	v_add_f64 v[5:6], v[17:18], -v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[5:6]
	v_mul_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_add_f64 v[9:10], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_fma_f64 v[9:10], v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[5:6], v[5:6]
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[13:14], v[11:12], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[54:55], s[52:53]
	s_mov_b32 s52, 0x3abe935a
	s_mov_b32 s53, 0x3fbe25e4
	v_add_f64 v[11:12], v[13:14], -v[11:12]
	v_mul_f64 v[21:22], v[7:8], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x47e6c9c2
	s_mov_b32 s53, 0x3fc110ef
	v_add_f64 v[9:10], v[9:10], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0xcfa74449
	s_mov_b32 s53, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x71bf3c30
	s_mov_b32 s53, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_mov_b32 s52, 0x1c7792ce
	s_mov_b32 s53, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[52:53]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[2:3]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[40:41]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[13:14], v[15:16]
	v_fma_f64 v[11:12], v[13:14], v[15:16], -v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[11:12], v[9:10], v[15:16], v[11:12]
	v_add_f64 v[15:16], v[17:18], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[19:20], v[15:16], s[8:9]
	v_add_f64 v[17:18], v[15:16], -v[17:18]
	s_mov_b32 s9, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[23:24], v[19:20], s[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_fma_f64 v[17:18], v[13:14], v[7:8], -v[21:22]
	v_add_f64 v[15:16], v[15:16], -v[23:24]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], s[2:3]
	v_fma_f64 v[13:14], v[13:14], v[5:6], v[17:18]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[15:16]
	v_fma_f64 v[9:10], v[9:10], v[7:8], v[13:14]
	v_ldexp_f64 v[7:8], v[7:8], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[19:20], v[11:12]
	v_add_f64 v[15:16], v[21:22], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[17:18], v[19:20], -v[13:14]
	v_mul_f64 v[19:20], v[15:16], v[13:14]
	v_add_f64 v[21:22], v[15:16], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[11:12], v[17:18]
	v_fma_f64 v[17:18], v[15:16], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[21:22]
	v_fma_f64 v[11:12], v[15:16], v[11:12], v[17:18]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[9:10], v[9:10], v[13:14], v[11:12]
	v_frexp_exp_i32_f64_e32 v13, s[22:23]
	v_add_f64 v[11:12], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v13, null, 0, v13, vcc_lo
	v_cvt_f64_i32_e32 v[13:14], v13
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[7:8], v[11:12]
	v_add_f64 v[17:18], v[11:12], -v[19:20]
	v_mul_f64 v[19:20], v[13:14], s[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[7:8], v[15:16], -v[7:8]
	v_add_f64 v[9:10], v[9:10], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[17:18], v[13:14], s[18:19], -v[19:20]
	s_mov_b32 s19, 0xbfe62e42
	v_add_f64 v[7:8], v[11:12], -v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_fma_f64 v[9:10], v[13:14], s[10:11], v[17:18]
	s_mov_b32 s11, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[19:20], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], v[5:6]
	v_add_f64 v[19:20], v[7:8], -v[19:20]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[7:8], v[11:12]
	v_add_f64 v[15:16], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[17:18], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[21:22], v[13:14], -v[17:18]
	v_add_f64 v[11:12], v[11:12], -v[17:18]
	v_add_f64 v[15:16], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[7:8], -v[21:22]
	v_add_f64 v[7:8], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[15:16], -v[9:10]
	v_add_f64 v[7:8], v[15:16], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[15:16], -v[11:12]
	v_add_f64 v[5:6], v[5:6], -v[11:12]
	v_add_f64 v[17:18], v[13:14], v[7:8]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[11:12], v[17:18], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[5:6], v[9:10]
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	v_add_f64 v[7:8], v[17:18], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[7:8], -v[17:18]
	v_mul_f64 v[11:12], v[3:4], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[3:4], v[7:8], -v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	v_fma_f64 v[5:6], v[3:4], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[7:8], v[11:12], v[5:6]
	v_dual_cndmask_b32 v10, v8, v12 :: v_dual_cndmask_b32 v9, v7, v11
	v_add_f64 v[7:8], v[7:8], -v[11:12]
	v_mul_f64 v[11:12], v[3:4], 0.5
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[13:14], v[9:10], s[6:7]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[9:10]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[9:10]
	v_cmp_neq_f64_e64 s3, 0x7ff00000, |v[9:10]|
	v_add_f64 v[5:6], v[5:6], -v[7:8]
	v_trunc_f64_e32 v[7:8], v[3:4]
	v_rndne_f64_e32 v[13:14], v[13:14]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v6, 0, v6, s3
	v_cndmask_b32_e64 v5, 0, v5, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[13:14], s[18:19], v[9:10]
	v_cvt_i32_f64_e32 v19, v[13:14]
	v_trunc_f64_e32 v[9:10], v[11:12]
	v_fma_f64 v[15:16], v[13:14], s[10:11], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s3, v[9:10], v[11:12]
	v_fma_f64 v[17:18], v[15:16], s[20:21], s[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[24:25]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[28:29]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[30:31]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[34:35]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[38:39]
	v_fma_f64 v[17:18], v[15:16], v[17:18], s[42:43]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], v[15:16], v[17:18], 1.0
	v_fma_f64 v[13:14], v[15:16], v[17:18], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[13:14], v[13:14], v19
	v_cndmask_b32_e32 v14, 0x7ff00000, v14, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s6, v13
	v_cndmask_b32_e64 v14, 0, v14, s2
	s_and_b32 s2, s2, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s6, s6, 0
	v_cmp_eq_f64_e64 s2, v[7:8], v[3:4]
	v_mov_b32_e32 v13, s6
	v_fma_f64 v[5:6], v[13:14], v[5:6], v[13:14]
	v_cmp_class_f64_e64 vcc_lo, v[13:14], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s7, v5
	s_and_b32 s8, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v6, v14, vcc_lo
	v_cmp_lt_f64_e64 vcc_lo, s[22:23], 0
	v_cmp_eq_f64_e64 s8, s[22:23], 0
	s_cselect_b32 s6, s6, s7
	s_and_b32 s3, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s7, s3, exec_lo
	s_cselect_b32 s7, s23, 0x3ff00000
	v_bfi_b32 v3, 0x7fffffff, v3, s7
	v_cmp_class_f64_e64 s7, s[22:23], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s2
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, s6, 0
	s_and_b32 s9, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	s_cselect_b32 s6, s2, s6
	s_or_b32 vcc_lo, s8, s7
	s_and_b32 s2, s8, exec_lo
	s_cselect_b32 s2, 0, 0x7ff00000
	s_and_b32 s3, s3, exec_lo
	s_cselect_b32 s3, s23, 0
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v4, s3
	s_and_b32 s3, vcc_lo, exec_lo
	s_cselect_b32 s3, 0, s6
	v_bfi_b32 v4, 0x7fffffff, s2, v4
	v_cmp_o_f64_e64 s2, s[22:23], s[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v3, v3, v4, vcc_lo
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s2
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, s3, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v3, s2
	s_cbranch_execz .LBB3_4
	s_branch .LBB3_5
.LBB3_3:
                                        ; implicit-def: $vgpr3_vgpr4
.LBB3_4:
	s_delay_alu instid0(VALU_DEP_1)
	v_div_scale_f64 v[3:4], null, v[1:2], v[1:2], -s[4:5]
	v_div_scale_f64 v[9:10], vcc_lo, -s[4:5], v[1:2], -s[4:5]
	s_mov_b32 s11, 0x3fe62e42
	s_mov_b32 s10, 0xfefa39ef
	s_mov_b32 s8, 0xfca7ab0c
	s_mov_b32 s16, 0x6a5dcb37
	s_mov_b32 s9, 0x3e928af3
	s_mov_b32 s17, 0x3e5ade15
	s_mov_b32 s18, 0x623fde64
	s_mov_b32 s19, 0x3ec71dee
	s_mov_b32 s20, 0x7c89e6b0
	s_mov_b32 s21, 0x3efa0199
	s_mov_b32 s22, 0x14761f6e
	s_mov_b32 s23, 0x3f2a01a0
	s_mov_b32 s24, 0x1852b7b0
	s_mov_b32 s25, 0x3f56c16c
	s_mov_b32 s26, 0x11122322
	s_mov_b32 s27, 0x3f811111
	s_mov_b32 s28, 0x555502a1
	s_mov_b32 s29, 0x3fa55555
	s_mov_b32 s30, 0x55555511
	s_mov_b32 s31, 0x3fc55555
	s_mov_b32 s34, 11
	s_mov_b32 s35, 0x3fe00000
	s_mov_b32 s36, 0x968915a9
	s_mov_b32 s38, 0x4222de17
	s_mov_b32 s37, 0x3fba6564
	s_mov_b32 s39, 0x3fbdee67
	v_rcp_f64_e32 v[5:6], v[3:4]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_fma_f64 v[7:8], -v[3:4], v[5:6], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], v[7:8], v[5:6]
	v_mul_f64 v[7:8], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[3:4], -v[3:4], v[7:8], v[9:10]
	v_div_fmas_f64 v[3:4], v[3:4], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[1:2], v[3:4], v[1:2], -s[4:5]
	s_mov_b32 s5, 0x3c7abc9e
	s_mov_b32 s4, 0x3b39803f
	v_rndne_f64_e32 v[3:4], v[1:2]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[1:2]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[1:2]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[1:2], -v[3:4]
	v_cvt_i32_f64_e32 v9, v[3:4]
	s_and_b32 s6, vcc_lo, exec_lo
	v_mul_f64 v[7:8], v[5:6], s[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[5:6], s[10:11], v[7:8]
	v_fma_f64 v[7:8], v[5:6], s[16:17], s[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[18:19]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[22:23]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[26:27]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[30:31]
	v_fma_f64 v[7:8], v[5:6], v[7:8], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[5:6], v[7:8], 1.0
	v_fma_f64 v[3:4], v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[3:4], v[3:4], v9
	v_readfirstlane_b32 s3, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_readfirstlane_b32 s6, v3
	s_cselect_b32 s3, s3, 0x7ff00000
	s_and_b32 s7, s2, vcc_lo
	s_and_b32 s7, s7, exec_lo
	s_cselect_b32 s6, s6, 0
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s7, s3, 0
	s_add_i32 s2, s45, 1
	v_cmp_neq_f64_e64 vcc_lo, s[6:7], 1.0
	v_cvt_f64_i32_e32 v[1:2], s2
	s_mov_b32 s3, 0x3fe55555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v2, 0x3ff00000, v2, vcc_lo
	v_cndmask_b32_e32 v1, 0, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_neq_f64_e32 vcc_lo, 0, v[1:2]
	s_and_b32 s2, vcc_lo, exec_lo
	s_cselect_b32 s7, s7, 0x3ff00000
	s_cselect_b32 s6, s6, 0
	s_mov_b32 s2, 0x55555555
	v_frexp_mant_f64_e64 v[3:4], |s[6:7]|
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[3:4]
	v_cndmask_b32_e64 v5, 0, 1, vcc_lo
	v_ldexp_f64 v[3:4], v[3:4], v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[5:6], v[3:4], 1.0
	v_add_f64 v[11:12], v[3:4], -1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	v_add_f64 v[13:14], v[5:6], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	v_fma_f64 v[7:8], v[9:10], v[7:8], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[11:12], v[7:8]
	v_mul_f64 v[15:16], v[5:6], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[9:10], v[5:6], -v[15:16]
	v_fma_f64 v[3:4], v[9:10], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[15:16], v[3:4]
	v_add_f64 v[13:14], v[11:12], -v[5:6]
	v_add_f64 v[15:16], v[5:6], -v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[11:12], v[11:12], -v[13:14]
	v_add_f64 v[3:4], v[15:16], -v[3:4]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[11:12], -v[5:6]
	v_add_f64 v[3:4], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[13:14], v[3:4]
	v_mul_f64 v[3:4], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[3:4]
	v_add_f64 v[7:8], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	v_fma_f64 v[7:8], v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[3:4], v[3:4]
	v_fma_f64 v[7:8], v[5:6], v[11:12], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[11:12], v[9:10], v[7:8]
	v_fma_f64 v[13:14], v[11:12], s[38:39], s[36:37]
	s_mov_b32 s36, 0x3abe935a
	s_mov_b32 s37, 0x3fbe25e4
	v_add_f64 v[9:10], v[11:12], -v[9:10]
	v_mul_f64 v[19:20], v[5:6], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x47e6c9c2
	s_mov_b32 s37, 0x3fc110ef
	v_add_f64 v[7:8], v[7:8], -v[9:10]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0xcfa74449
	s_mov_b32 s37, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x71bf3c30
	s_mov_b32 s37, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x1c7792ce
	s_mov_b32 s37, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x924920da
	s_mov_b32 s37, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_mov_b32 s36, 0x9999999c
	s_mov_b32 s37, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[13:14], v[11:12], v[13:14], s[36:37]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[15:16], v[11:12], v[13:14]
	v_fma_f64 v[9:10], v[11:12], v[13:14], -v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[13:14], v[9:10]
	v_add_f64 v[13:14], v[15:16], v[9:10]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[17:18], v[13:14], s[2:3]
	v_add_f64 v[15:16], v[13:14], -v[15:16]
	s_mov_b32 s3, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[21:22], v[17:18], s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_fma_f64 v[15:16], v[11:12], v[5:6], -v[19:20]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_add_f64 v[13:14], v[13:14], -v[21:22]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], s[2:3]
	v_fma_f64 v[11:12], v[11:12], v[3:4], v[15:16]
	v_ldexp_f64 v[3:4], v[3:4], 1
	s_mov_b32 s2, 0x652b82fe
	s_mov_b32 s3, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[13:14]
	v_fma_f64 v[7:8], v[7:8], v[5:6], v[11:12]
	v_ldexp_f64 v[5:6], v[5:6], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[17:18], v[9:10]
	v_add_f64 v[13:14], v[19:20], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[15:16], v[17:18], -v[11:12]
	v_mul_f64 v[17:18], v[13:14], v[11:12]
	v_add_f64 v[19:20], v[13:14], -v[19:20]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[9:10], v[9:10], v[15:16]
	v_fma_f64 v[15:16], v[13:14], v[11:12], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[19:20]
	v_fma_f64 v[9:10], v[13:14], v[9:10], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[7:8], v[11:12], v[9:10]
	v_frexp_exp_i32_f64_e32 v11, s[6:7]
	v_add_f64 v[9:10], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_subrev_co_ci_u32_e64 v11, null, 0, v11, vcc_lo
	v_cvt_f64_i32_e32 v[11:12], v11
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[5:6], v[9:10]
	v_add_f64 v[15:16], v[9:10], -v[17:18]
	v_mul_f64 v[17:18], v[11:12], s[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[5:6], v[13:14], -v[5:6]
	v_add_f64 v[7:8], v[7:8], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[15:16], v[11:12], s[10:11], -v[17:18]
	s_mov_b32 s11, 0xbfe62e42
	v_add_f64 v[5:6], v[9:10], -v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_fma_f64 v[7:8], v[11:12], s[4:5], v[15:16]
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[17:18], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], v[3:4]
	v_add_f64 v[17:18], v[5:6], -v[17:18]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[11:12], v[5:6], v[9:10]
	v_add_f64 v[13:14], v[9:10], -v[13:14]
	v_add_f64 v[7:8], v[7:8], -v[17:18]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[15:16], v[11:12], -v[5:6]
	v_add_f64 v[3:4], v[3:4], -v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[19:20], v[11:12], -v[15:16]
	v_add_f64 v[9:10], v[9:10], -v[15:16]
	v_add_f64 v[13:14], v[7:8], v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], -v[19:20]
	v_add_f64 v[5:6], v[9:10], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[9:10], v[13:14], -v[7:8]
	v_add_f64 v[5:6], v[13:14], v[5:6]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[13:14], v[13:14], -v[9:10]
	v_add_f64 v[3:4], v[3:4], -v[9:10]
	v_add_f64 v[15:16], v[11:12], v[5:6]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[7:8], -v[13:14]
	v_add_f64 v[9:10], v[15:16], -v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[3:4], v[3:4], v[7:8]
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[3:4], v[3:4], v[5:6]
	v_add_f64 v[5:6], v[15:16], v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[7:8], v[5:6], -v[15:16]
	v_mul_f64 v[9:10], v[1:2], v[5:6]
	v_add_f64 v[3:4], v[3:4], -v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[5:6], v[1:2], v[5:6], -v[9:10]
	v_cmp_class_f64_e64 vcc_lo, v[9:10], 0x204
	v_fma_f64 v[3:4], v[1:2], v[3:4], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[5:6], v[9:10], v[3:4]
	v_dual_cndmask_b32 v8, v6, v10 :: v_dual_cndmask_b32 v7, v5, v9
	v_add_f64 v[5:6], v[5:6], -v[9:10]
	v_mul_f64 v[9:10], v[1:2], 0.5
	s_delay_alu instid0(VALU_DEP_3)
	v_mul_f64 v[11:12], v[7:8], s[2:3]
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[7:8]
	v_cmp_ngt_f64_e64 s2, 0xc090cc00, v[7:8]
	v_cmp_neq_f64_e64 s3, 0x7ff00000, |v[7:8]|
	v_add_f64 v[3:4], v[3:4], -v[5:6]
	v_trunc_f64_e32 v[5:6], v[1:2]
	v_rndne_f64_e32 v[11:12], v[11:12]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v4, 0, v4, s3
	v_cndmask_b32_e64 v3, 0, v3, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[11:12], s[10:11], v[7:8]
	v_cvt_i32_f64_e32 v17, v[11:12]
	v_trunc_f64_e32 v[7:8], v[9:10]
	v_fma_f64 v[13:14], v[11:12], s[4:5], v[13:14]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_neq_f64_e64 s3, v[7:8], v[9:10]
	v_fma_f64 v[15:16], v[13:14], s[16:17], s[8:9]
	v_cmp_class_f64_e64 s9, s[6:7], 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[18:19]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[22:23]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[26:27]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[30:31]
	v_fma_f64 v[15:16], v[13:14], v[15:16], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[15:16], v[13:14], v[15:16], 1.0
	v_fma_f64 v[11:12], v[13:14], v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[11:12], v[11:12], v17
	v_cndmask_b32_e32 v12, 0x7ff00000, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_readfirstlane_b32 s4, v11
	v_cndmask_b32_e64 v12, 0, v12, s2
	s_and_b32 s2, s2, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s4, s4, 0
	v_cmp_eq_f64_e64 s2, v[5:6], v[1:2]
	v_mov_b32_e32 v11, s4
	v_fma_f64 v[3:4], v[11:12], v[3:4], v[11:12]
	v_cmp_class_f64_e64 vcc_lo, v[11:12], 0x204
	s_delay_alu instid0(VALU_DEP_2)
	v_readfirstlane_b32 s5, v3
	s_and_b32 s8, vcc_lo, exec_lo
	v_cndmask_b32_e32 v3, v4, v12, vcc_lo
	v_cmp_lt_f64_e64 vcc_lo, s[6:7], 0
	v_cmp_eq_f64_e64 s8, s[6:7], 0
	s_cselect_b32 s4, s4, s5
	s_and_b32 s5, s2, s3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	s_and_b32 s3, s5, exec_lo
	s_cselect_b32 s3, s7, 0x3ff00000
	v_bfi_b32 v3, 0x7fffffff, v3, s3
	v_cmp_gt_f64_e64 s3, 0, v[1:2]
	s_delay_alu instid0(VALU_DEP_2)
	v_cndmask_b32_e64 v4, 0x7ff80000, v3, s2
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, s4, 0
	s_and_b32 s10, vcc_lo, exec_lo
	s_cselect_b32 s4, s2, s4
	v_cndmask_b32_e32 v1, v3, v4, vcc_lo
	s_or_b32 vcc_lo, s8, s9
	s_xor_b32 s2, s3, s8
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, 0, 0x7ff00000
	s_and_b32 s3, s5, exec_lo
	s_cselect_b32 s3, s7, 0
	v_mov_b32_e32 v2, s3
	s_and_b32 s3, vcc_lo, exec_lo
	s_cselect_b32 s3, 0, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_bfi_b32 v2, 0x7fffffff, s2, v2
	v_cmp_o_f64_e64 s2, s[6:7], s[6:7]
	v_cndmask_b32_e32 v1, v1, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v4, 0x7ff80000, v1, s2
	s_and_b32 s2, s2, exec_lo
	s_cselect_b32 s2, s3, 0
	v_mov_b32_e32 v3, s2
.LBB3_5:
	s_mul_i32 s2, s49, s46
	s_xor_b32 s3, s47, s50
	s_sub_i32 s2, s48, s2
	s_add_i32 s4, s49, 1
	s_sub_i32 s5, s2, s46
	s_cmp_ge_u32 s2, s46
	s_mul_i32 s18, s15, s13
	s_cselect_b32 s16, s4, s49
	s_cselect_b32 s2, s5, s2
	s_load_b256 s[4:11], s[0:1], 0x0
	s_add_i32 s17, s16, 1
	s_cmp_ge_u32 s2, s46
	s_mul_hi_i32 s19, s45, s15
	s_cselect_b32 s2, s17, s16
	s_mul_i32 s16, s15, s14
	s_xor_b32 s2, s2, s3
	s_mul_hi_i32 s21, s33, s18
	s_sub_i32 s14, s2, s3
	s_cmp_lt_i32 s12, 1
	v_cmp_gt_i32_e64 s2, s15, v0
	s_cselect_b32 s17, -1, 0
	s_cmp_gt_i32 s12, 0
	v_cmp_eq_u32_e64 s3, 0, v0
	s_cselect_b32 s13, -1, 0
	s_mul_i32 s20, s33, s18
	s_and_b32 vcc_lo, exec_lo, s13
	s_mul_i32 s18, s45, s15
	s_cbranch_vccz .LBB3_21
; %bb.6:
	v_mbcnt_lo_u32_b32 v1, -1, 0
	s_lshl_b64 s[22:23], s[20:21], 3
	s_mul_hi_i32 s25, s14, s15
	s_waitcnt lgkmcnt(0)
	s_add_u32 s22, s4, s22
	s_addc_u32 s23, s5, s23
	v_cmp_gt_u32_e32 vcc_lo, 24, v1
	s_lshl_b64 s[4:5], s[18:19], 3
	s_mul_i32 s24, s14, s15
	s_add_u32 s22, s22, s4
	s_addc_u32 s23, s23, s5
	v_cndmask_b32_e64 v2, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v1
	s_lshl_b64 s[4:5], s[24:25], 3
	v_and_b32_e32 v7, 31, v0
	s_ashr_i32 s24, s16, 31
	s_add_u32 s25, s6, s4
	v_cndmask_b32_e64 v5, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v1
	s_addc_u32 s26, s7, s5
	s_add_u32 s6, s0, 64
	s_addc_u32 s7, s1, 0
	v_lshl_or_b32 v8, v1, 2, 64
	v_cndmask_b32_e64 v6, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v1
	v_add_lshl_u32 v9, v2, v1, 2
	v_add_lshl_u32 v10, v5, v1, 2
	v_lshrrev_b32_e32 v13, 2, v0
	v_add_lshl_u32 v11, v6, v1, 2
	v_add_co_ci_u32_e64 v12, null, 0, v1, vcc_lo
	v_cmp_gt_u32_e64 s4, 32, v0
	v_dual_mov_b32 v15, 0 :: v_dual_lshlrev_b32 v14, 3, v7
	s_delay_alu instid0(VALU_DEP_3)
	v_lshlrev_b32_e32 v12, 2, v12
	s_cmp_lt_i32 s33, s44
	s_mov_b32 s27, 0
	s_cselect_b32 s28, -1, 0
	v_cmp_eq_u32_e32 vcc_lo, 0, v7
	s_branch .LBB3_8
.LBB3_7:                                ;   in Loop: Header=BB3_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_add_i32 s27, s27, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u32 s27, s12
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB3_21
.LBB3_8:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB3_10 Depth 2
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_and_saveexec_b32 s29, s2
	s_cbranch_execz .LBB3_12
; %bb.9:                                ;   in Loop: Header=BB3_8 Depth=1
	s_load_b32 s5, s[6:7], 0xc
	s_mul_i32 s31, s27, s24
	s_mul_hi_u32 s34, s27, s16
	s_mul_i32 s30, s27, s16
	s_add_i32 s31, s34, s31
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v5, v0
	s_lshl_b64 s[30:31], s[30:31], 3
	s_mov_b32 s34, 0
	s_add_u32 s30, s25, s30
	s_addc_u32 s31, s26, s31
	s_waitcnt lgkmcnt(0)
	s_and_b32 s35, s5, 0xffff
	.p2align	6
.LBB3_10:                               ;   Parent Loop BB3_8 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[16:17], 3, v[5:6]
	v_add_nc_u32_e32 v5, s35, v5
	v_add_co_u32 v18, s5, s22, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v19, null, s23, v17, s5
	v_add_co_u32 v16, s5, s30, v16
	v_add_co_ci_u32_e64 v17, null, s31, v17, s5
	global_load_b64 v[18:19], v[18:19], off
	global_load_b64 v[16:17], v[16:17], off
	v_cmp_le_i32_e64 s5, s15, v5
	s_or_b32 s34, s5, s34
	s_waitcnt vmcnt(0)
	v_fma_f64 v[1:2], v[18:19], v[16:17], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s34
	s_cbranch_execnz .LBB3_10
; %bb.11:                               ;   in Loop: Header=BB3_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s34
.LBB3_12:                               ;   in Loop: Header=BB3_8 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s29
	ds_bpermute_b32 v5, v8, v1
	ds_bpermute_b32 v6, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_bpermute_b32 v5, v9, v1
	ds_bpermute_b32 v6, v9, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_bpermute_b32 v5, v10, v1
	ds_bpermute_b32 v6, v10, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_bpermute_b32 v5, v11, v1
	ds_bpermute_b32 v6, v11, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_bpermute_b32 v5, v12, v1
	ds_bpermute_b32 v6, v12, v2
	s_and_saveexec_b32 s5, vcc_lo
	s_cbranch_execz .LBB3_14
; %bb.13:                               ;   in Loop: Header=BB3_8 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_store_b64 v13, v[1:2]
.LBB3_14:                               ;   in Loop: Header=BB3_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s29, s4
	s_cbranch_execz .LBB3_19
; %bb.15:                               ;   in Loop: Header=BB3_8 Depth=1
	s_load_b32 s5, s[6:7], 0xc
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s30, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s5, s5, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s5, 31
	s_lshr_b32 s5, s5, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s5, v7
; %bb.16:                               ;   in Loop: Header=BB3_8 Depth=1
	ds_load_b64 v[1:2], v14
; %bb.17:                               ;   in Loop: Header=BB3_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s30
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v5, v8, v1
	ds_bpermute_b32 v6, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_bpermute_b32 v5, v9, v1
	ds_bpermute_b32 v6, v9, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_bpermute_b32 v5, v10, v1
	ds_bpermute_b32 v6, v10, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_bpermute_b32 v5, v11, v1
	ds_bpermute_b32 v6, v11, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_bpermute_b32 v5, v12, v1
	ds_bpermute_b32 v6, v12, v2
	s_and_b32 exec_lo, exec_lo, vcc_lo
	s_cbranch_execz .LBB3_19
; %bb.18:                               ;   in Loop: Header=BB3_8 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_store_b64 v15, v[1:2]
.LBB3_19:                               ;   in Loop: Header=BB3_8 Depth=1
	s_or_b32 exec_lo, exec_lo, s29
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v15
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s5, s3
	s_cbranch_execz .LBB3_7
; %bb.20:                               ;   in Loop: Header=BB3_8 Depth=1
	s_cmp_lt_i32 s33, s27
	s_cselect_b32 s29, -1, 0
	s_sub_i32 s30, s33, s27
	s_and_b32 s29, s28, s29
	v_cvt_f64_i32_e32 v[5:6], s30
	s_lshl_b32 s30, s27, 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	s_addk_i32 s30, 0x200
	v_fma_f64 v[1:2], -v[3:4], v[5:6], v[1:2]
	v_mov_b32_e32 v5, s30
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v2, v2, 0xc6293e59, s29
	v_cndmask_b32_e64 v1, v1, 0x39a08cea, s29
	ds_store_b64 v5, v[1:2]
	s_branch .LBB3_7
.LBB3_21:
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_xor_b32 s2, s17, -1
	s_waitcnt lgkmcnt(0)
	s_mov_b32 s6, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB3_39
; %bb.22:
	s_add_i32 s4, s12, -1
	s_mov_b32 s3, 0xfe37e43c
	s_cmp_lt_u32 s4, 7
	s_mov_b32 s2, 0x8800759c
	s_cbranch_scc1 .LBB3_25
; %bb.23:
	s_and_b32 s6, s12, 0x7ffffff8
	s_mov_b32 s7, 0
	s_movk_i32 s17, 0x200
.LBB3_24:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_scc0 .LBB3_24
.LBB3_25:
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_and_b32 s7, s12, 7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s7, 0
	s_cbranch_scc1 .LBB3_28
; %bb.26:
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_lshl_b32 s2, s6, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s2, 0x200
.LBB3_27:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v1, s2
	s_add_i32 s7, s7, -1
	s_add_i32 s2, s2, 8
	s_cmp_lg_u32 s7, 0
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[1:2], v[3:4]
	v_dual_cndmask_b32 v4, v4, v2 :: v_dual_cndmask_b32 v3, v3, v1
	s_cbranch_scc1 .LBB3_27
.LBB3_28:
	s_cmp_eq_u32 s4, 0
	s_mov_b32 s17, 0
	s_cbranch_scc1 .LBB3_51
; %bb.29:
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
.LBB3_30:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_scc1 .LBB3_30
; %bb.31:
	s_bitcmp0_b32 s12, 0
	s_mov_b32 s6, 0
	s_cbranch_scc1 .LBB3_33
.LBB3_32:
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
.LBB3_33:
	s_cmp_lt_u32 s12, 4
	s_cbranch_scc1 .LBB3_36
; %bb.34:
	s_and_b32 s6, s12, 0x7ffffffc
	s_mov_b32 s7, 0
	s_movk_i32 s17, 0x200
.LBB3_35:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_scc1 .LBB3_35
.LBB3_36:
	s_and_b32 s2, s12, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s2, 0
	s_cbranch_scc1 .LBB3_39
; %bb.37:
	s_lshl_b32 s3, s6, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s3, 0x200
	.p2align	6
.LBB3_38:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_scc1 .LBB3_38
.LBB3_39:
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s15, v0
	s_cbranch_execz .LBB3_50
; %bb.40:
	s_load_b32 s0, s[0:1], 0x4c
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
	s_branch .LBB3_42
.LBB3_41:                               ;   in Loop: Header=BB3_42 Depth=1
	v_add_nc_u32_e32 v0, s18, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, s0, s10, v1
	v_add_co_ci_u32_e64 v2, null, s11, v2, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s15, v0
	global_store_b64 v[1:2], v[3:4], off
	s_or_b32 s1, vcc_lo, s1
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB3_50
.LBB3_42:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB3_45 Depth 2
                                        ;     Child Loop BB3_49 Depth 2
	v_ashrrev_i32_e32 v1, 31, v0
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_and_not1_b32 vcc_lo, exec_lo, s13
	s_delay_alu instid0(VALU_DEP_3)
	v_lshlrev_b64 v[1:2], 3, v[0:1]
	s_cbranch_vccnz .LBB3_41
; %bb.43:                               ;   in Loop: Header=BB3_42 Depth=1
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_and_not1_b32 vcc_lo, exec_lo, s19
	s_cbranch_vccnz .LBB3_47
; %bb.44:                               ;   in Loop: Header=BB3_42 Depth=1
	v_add_co_u32 v5, vcc_lo, s8, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s9, v2, vcc_lo
	s_mov_b32 s0, 0
	s_movk_i32 s16, 0x200
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB3_45:                               ;   Parent Loop BB3_42 Depth=1
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
	s_cbranch_scc0 .LBB3_45
; %bb.46:                               ;   in Loop: Header=BB3_42 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s16, s12
	s_and_not1_b32 vcc_lo, exec_lo, s22
	s_cbranch_vccz .LBB3_48
	s_branch .LBB3_41
.LBB3_47:                               ;   in Loop: Header=BB3_42 Depth=1
	s_mov_b32 s16, 0
	s_and_not1_b32 vcc_lo, exec_lo, s22
	s_cbranch_vccnz .LBB3_41
.LBB3_48:                               ;   in Loop: Header=BB3_42 Depth=1
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
.LBB3_49:                               ;   Parent Loop BB3_42 Depth=1
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
	s_cbranch_scc1 .LBB3_49
	s_branch .LBB3_41
.LBB3_50:
	s_endpgm
.LBB3_51:
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_bitcmp0_b32 s12, 0
	s_mov_b32 s6, 0
	s_cbranch_scc0 .LBB3_32
	s_branch .LBB3_33
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid
		.amdhsa_group_segment_fixed_size 512
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
		.amdhsa_next_free_vgpr 44
		.amdhsa_next_free_sgpr 56
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
	.section	.text._Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid,"axG",@progbits,_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid,comdat
.Lfunc_end3:
	.size	_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid, .Lfunc_end3-_Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid
                                        ; -- End function
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.num_vgpr, 44
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.num_agpr, 0
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.numbered_sgpr, 56
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.num_named_barrier, 0
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.private_seg_size, 0
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.uses_vcc, 1
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.uses_flat_scratch, 0
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.has_dyn_sized_stack, 0
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.has_recursion, 0
	.set _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 10284
; TotalNumSgprs: 58
; NumVgprs: 44
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 512 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 58
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
	.section	.text._Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii,"axG",@progbits,_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii,comdat
	.protected	_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii ; -- Begin function _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii
	.globl	_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii
	.p2align	8
	.type	_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii,@function
_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii: ; @_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii
; %bb.0:
	s_clause 0x1
	s_load_b128 s[16:19], s[0:1], 0x20
	s_load_b64 s[20:21], s[0:1], 0x30
	s_abs_i32 s6, s2
	s_mov_b32 s27, 0
	s_waitcnt lgkmcnt(0)
	s_abs_i32 s3, s17
	s_ashr_i32 s7, s17, 31
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
	s_abs_i32 s6, s18
	s_xor_b32 s4, s4, s5
	v_cvt_f32_u32_e32 v1, s6
	s_sub_i32 s9, 0, s6
	s_sub_i32 s23, s4, s5
	s_ashr_i32 s10, s18, 31
	s_mul_i32 s5, s23, s17
	v_rcp_iflag_f32_e32 v1, v1
	s_sub_i32 s24, s2, s5
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
	s_load_b256 s[8:15], s[0:1], 0x0
	s_cselect_b32 s3, s3, s4
	s_cselect_b32 s2, s5, s2
	s_add_i32 s4, s3, 1
	s_cmp_ge_u32 s2, s6
	s_cselect_b32 s2, s4, s3
	s_abs_i32 s6, s24
	s_xor_b32 s2, s2, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_sub_i32 s2, s2, s7
	s_abs_i32 s3, s2
	s_xor_b32 s2, s24, s2
	v_cvt_f32_u32_e32 v1, s3
	s_sub_i32 s5, 0, s3
	s_ashr_i32 s2, s2, 31
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
	s_mul_hi_u32 s4, s6, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_mul_i32 s5, s4, s3
	s_sub_i32 s5, s6, s5
	s_add_i32 s6, s4, 1
	s_sub_i32 s7, s5, s3
	s_cmp_ge_u32 s5, s3
	s_cselect_b32 s4, s6, s4
	s_cselect_b32 s5, s7, s5
	s_add_i32 s6, s4, 1
	s_cmp_ge_u32 s5, s3
	s_cselect_b32 s3, s6, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b32 s3, s3, s2
	s_sub_i32 s25, s3, s2
	s_cmp_lt_i32 s16, 1
	v_cmp_eq_u32_e64 s2, 0, v0
	s_cselect_b32 s26, -1, 0
	s_cmp_gt_i32 s16, 0
	s_cselect_b32 s22, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 vcc_lo, exec_lo, s22
	s_cbranch_vccz .LBB4_16
; %bb.1:
	s_mul_i32 s3, s19, s17
	v_mbcnt_lo_u32_b32 v1, -1, 0
	s_mul_hi_i32 s5, s23, s3
	s_mul_i32 s4, s23, s3
	s_mul_hi_i32 s7, s24, s19
	s_lshl_b64 s[4:5], s[4:5], 2
	s_mul_i32 s6, s24, s19
	s_waitcnt lgkmcnt(0)
	s_add_u32 s3, s8, s4
	s_addc_u32 s9, s9, s5
	s_lshl_b64 s[4:5], s[6:7], 2
	s_mul_i32 s28, s19, s18
	s_add_u32 s8, s3, s4
	v_cmp_gt_u32_e64 s3, 24, v1
	s_addc_u32 s9, s9, s5
	s_mul_hi_i32 s5, s25, s19
	s_mul_i32 s4, s25, s19
	v_and_b32_e32 v3, 31, v0
	v_cndmask_b32_e64 v2, 0, 8, s3
	v_cmp_gt_u32_e64 s3, 28, v1
	s_lshl_b64 s[4:5], s[4:5], 2
	s_ashr_i32 s29, s28, 31
	s_add_u32 s10, s10, s4
	v_add_lshl_u32 v5, v2, v1, 2
	v_cndmask_b32_e64 v6, 0, 4, s3
	v_cmp_gt_u32_e64 s3, 30, v1
	v_lshrrev_b32_e32 v2, 3, v0
	s_addc_u32 s11, s11, s5
	s_add_u32 s6, s0, 56
	s_addc_u32 s7, s1, 0
	v_cndmask_b32_e64 v7, 0, 2, s3
	v_cmp_ne_u32_e64 s3, 31, v1
	v_lshl_or_b32 v4, v1, 2, 64
	v_add_lshl_u32 v6, v6, v1, 2
	v_and_b32_e32 v9, 0x7c, v2
	v_add_lshl_u32 v7, v7, v1, 2
	v_add_co_ci_u32_e64 v8, null, 0, v1, s3
	v_cmp_eq_u32_e64 s3, 0, v3
	v_cmp_gt_u32_e64 s4, 32, v0
	v_lshlrev_b32_e32 v10, 2, v3
	s_delay_alu instid0(VALU_DEP_4)
	v_dual_mov_b32 v11, 0 :: v_dual_lshlrev_b32 v8, 2, v8
	s_cmp_lt_i32 s23, s21
	v_cmp_gt_i32_e32 vcc_lo, s19, v0
	s_cselect_b32 s21, -1, 0
	s_branch .LBB4_3
.LBB4_2:                                ;   in Loop: Header=BB4_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_add_i32 s27, s27, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u32 s27, s16
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB4_16
.LBB4_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB4_5 Depth 2
	v_mov_b32_e32 v12, 0
	s_and_saveexec_b32 s30, vcc_lo
	s_cbranch_execz .LBB4_7
; %bb.4:                                ;   in Loop: Header=BB4_3 Depth=1
	s_load_b32 s5, s[6:7], 0xc
	s_mul_i32 s31, s27, s29
	s_mul_hi_u32 s33, s27, s28
	s_mul_i32 s34, s27, s28
	s_add_i32 s35, s33, s31
	v_dual_mov_b32 v12, 0 :: v_dual_mov_b32 v1, v0
	s_lshl_b64 s[34:35], s[34:35], 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s31, s10, s34
	s_addc_u32 s33, s11, s35
	s_mov_b32 s34, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s35, s5, 0xffff
	.p2align	6
.LBB4_5:                                ;   Parent Loop BB4_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[13:14], 2, v[1:2]
	v_add_nc_u32_e32 v1, s35, v1
	v_add_co_u32 v15, s5, s8, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v16, null, s9, v14, s5
	v_add_co_u32 v13, s5, s31, v13
	v_add_co_ci_u32_e64 v14, null, s33, v14, s5
	global_load_b32 v2, v[15:16], off
	global_load_b32 v13, v[13:14], off
	v_cmp_le_i32_e64 s5, s19, v1
	s_or_b32 s34, s5, s34
	s_waitcnt vmcnt(0)
	v_fmac_f32_e32 v12, v2, v13
	s_and_not1_b32 exec_lo, exec_lo, s34
	s_cbranch_execnz .LBB4_5
; %bb.6:                                ;   in Loop: Header=BB4_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s34
.LBB4_7:                                ;   in Loop: Header=BB4_3 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s30
	ds_bpermute_b32 v1, v4, v12
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v12, v1
	ds_bpermute_b32 v2, v5, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_saveexec_b32 s5, s3
	s_cbranch_execz .LBB4_9
; %bb.8:                                ;   in Loop: Header=BB4_3 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v9, v1
.LBB4_9:                                ;   in Loop: Header=BB4_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s30, s4
	s_cbranch_execz .LBB4_14
; %bb.10:                               ;   in Loop: Header=BB4_3 Depth=1
	s_load_b32 s5, s[6:7], 0xc
	v_mov_b32_e32 v1, 0
	s_mov_b32 s31, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s5, s5, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s5, 31
	s_lshr_b32 s5, s5, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s5, v3
; %bb.11:                               ;   in Loop: Header=BB4_3 Depth=1
	ds_load_b32 v1, v10
; %bb.12:                               ;   in Loop: Header=BB4_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s31
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v2, v4, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v5, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v6, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v7, v1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_bpermute_b32 v2, v8, v1
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB4_14
; %bb.13:                               ;   in Loop: Header=BB4_3 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f32_e32 v1, v1, v2
	ds_store_b32 v11, v1
.LBB4_14:                               ;   in Loop: Header=BB4_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s30
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b32 v1, v11
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB4_2
; %bb.15:                               ;   in Loop: Header=BB4_3 Depth=1
	s_lshl_b32 s30, s27, 2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_addk_i32 s30, 0x100
	s_cmp_gt_i32 s27, s23
	v_mov_b32_e32 v2, s30
	s_cselect_b32 s31, -1, 0
	s_and_b32 s31, s21, s31
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e64 v1, v1, 0xf149f2ca, s31
	ds_store_b32 v2, v1
	s_branch .LBB4_2
.LBB4_16:
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_xor_b32 s3, s26, -1
	s_mov_b32 s2, 0
	s_and_b32 s3, vcc_lo, s3
	s_waitcnt lgkmcnt(0)
	s_and_saveexec_b32 s9, s3
	s_cbranch_execz .LBB4_36
; %bb.17:
	v_mov_b32_e32 v2, 0xff800000
	s_add_i32 s3, s16, -1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lt_u32 s3, 7
	s_cbranch_scc1 .LBB4_20
; %bb.18:
	s_and_b32 s2, s16, 0x7ffffff8
	s_mov_b32 s3, 0
	s_movk_i32 s4, 0x100
	.p2align	6
.LBB4_19:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v1, s4
	s_add_i32 s3, s3, 8
	s_add_i32 s4, s4, 32
	s_cmp_eq_u32 s2, s3
	ds_load_2addr_b32 v[3:4], v1 offset1:1
	ds_load_2addr_b32 v[5:6], v1 offset0:2 offset1:3
	ds_load_2addr_b32 v[7:8], v1 offset0:4 offset1:5
	ds_load_2addr_b32 v[9:10], v1 offset0:6 offset1:7
	s_waitcnt lgkmcnt(3)
	v_cmp_gt_f32_e32 vcc_lo, v3, v2
	v_cndmask_b32_e32 v1, v2, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, v4, v1
	v_cndmask_b32_e32 v1, v1, v4, vcc_lo
	s_waitcnt lgkmcnt(2)
	v_cmp_gt_f32_e32 vcc_lo, v5, v1
	v_cndmask_b32_e32 v1, v1, v5, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, v6, v1
	v_cndmask_b32_e32 v1, v1, v6, vcc_lo
	s_waitcnt lgkmcnt(1)
	v_cmp_gt_f32_e32 vcc_lo, v7, v1
	v_cndmask_b32_e32 v1, v1, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, v8, v1
	v_cndmask_b32_e32 v1, v1, v8, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f32_e32 vcc_lo, v9, v1
	v_cndmask_b32_e32 v1, v1, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, v10, v1
	v_cndmask_b32_e32 v2, v1, v10, vcc_lo
	s_cbranch_scc0 .LBB4_19
.LBB4_20:
	s_and_b32 s10, s16, 7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_lg_u32 s10, 0
	s_cselect_b32 s3, -1, 0
	s_cmp_eq_u32 s10, 0
	s_cbranch_scc1 .LBB4_23
; %bb.21:
	s_lshl_b32 s2, s2, 2
	s_mov_b32 s4, s10
	s_addk_i32 s2, 0x100
.LBB4_22:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v1, s2
	s_add_i32 s4, s4, -1
	s_add_i32 s2, s2, 4
	s_cmp_lg_u32 s4, 0
	ds_load_b32 v1, v1
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f32_e32 vcc_lo, v1, v2
	v_cndmask_b32_e32 v2, v2, v1, vcc_lo
	s_cbranch_scc1 .LBB4_22
.LBB4_23:
	v_mov_b32_e32 v1, 0
	s_cmp_gt_u32 s16, 7
	s_cselect_b32 s2, -1, 0
	s_cmp_lt_u32 s16, 8
	s_cbranch_scc1 .LBB4_27
; %bb.24:
	s_and_b32 s4, s16, 0x7ffffff8
	s_mov_b32 s5, 0
	s_movk_i32 s6, 0x100
.LBB4_25:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v11, s6
	s_add_i32 s5, s5, 8
	s_add_i32 s6, s6, 32
	s_cmp_lg_u32 s4, s5
	ds_load_2addr_b32 v[3:4], v11 offset1:1
	ds_load_2addr_b32 v[5:6], v11 offset0:2 offset1:3
	ds_load_2addr_b32 v[7:8], v11 offset0:4 offset1:5
	ds_load_2addr_b32 v[9:10], v11 offset0:6 offset1:7
	s_waitcnt lgkmcnt(3)
	v_sub_f32_e32 v4, v4, v2
	s_waitcnt lgkmcnt(2)
	v_sub_f32_e32 v5, v5, v2
	v_sub_f32_e32 v3, v3, v2
	v_sub_f32_e32 v6, v6, v2
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e32 v9, v9, v2
	v_dual_mul_f32 v13, 0x3fb8aa3b, v4 :: v_dual_mul_f32 v14, 0x3fb8aa3b, v5
	v_dual_mul_f32 v12, 0x3fb8aa3b, v3 :: v_dual_sub_f32 v7, v7, v2
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v18, 0x3fb8aa3b, v9
	v_rndne_f32_e32 v23, v13
	v_sub_f32_e32 v8, v8, v2
	s_delay_alu instid0(VALU_DEP_4)
	v_rndne_f32_e32 v21, v12
	v_mul_f32_e32 v15, 0x3fb8aa3b, v6
	v_fma_f32 v22, 0x3fb8aa3b, v4, -v13
	v_dual_sub_f32 v13, v13, v23 :: v_dual_sub_f32 v10, v10, v2
	v_dual_mul_f32 v17, 0x3fb8aa3b, v8 :: v_dual_mul_f32 v16, 0x3fb8aa3b, v7
	v_fma_f32 v20, 0x3fb8aa3b, v3, -v12
	v_rndne_f32_e32 v25, v14
	s_delay_alu instid0(VALU_DEP_4)
	v_dual_mul_f32 v19, 0x3fb8aa3b, v10 :: v_dual_sub_f32 v12, v12, v21
	v_fma_f32 v26, 0x3fb8aa3b, v6, -v15
	v_rndne_f32_e32 v27, v15
	v_fma_f32 v24, 0x3fb8aa3b, v5, -v14
	v_fma_f32 v28, 0x3fb8aa3b, v7, -v16
	v_fmac_f32_e32 v22, 0x32a5705f, v4
	v_sub_f32_e32 v14, v14, v25
	v_rndne_f32_e32 v35, v19
	v_dual_fmac_f32 v26, 0x32a5705f, v6 :: v_dual_sub_f32 v15, v15, v27
	v_fma_f32 v34, 0x3fb8aa3b, v10, -v19
	v_dual_fmac_f32 v20, 0x32a5705f, v3 :: v_dual_add_f32 v13, v13, v22
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_sub_f32_e32 v19, v19, v35
	v_dual_fmac_f32 v28, 0x32a5705f, v7 :: v_dual_add_f32 v15, v15, v26
	v_cvt_i32_f32_e32 v27, v27
	v_cvt_i32_f32_e32 v21, v21
	v_exp_f32_e32 v13, v13
	v_cvt_i32_f32_e32 v23, v23
	v_exp_f32_e32 v15, v15
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v3
	v_fma_f32 v30, 0x3fb8aa3b, v8, -v17
	v_rndne_f32_e32 v31, v17
	v_fmac_f32_e32 v24, 0x32a5705f, v5
	v_rndne_f32_e32 v29, v16
	v_rndne_f32_e32 v33, v18
	v_cvt_i32_f32_e32 v25, v25
	v_ldexp_f32 v13, v13, v23
	v_sub_f32_e32 v17, v17, v31
	v_ldexp_f32 v15, v15, v27
	v_add_f32_e32 v12, v12, v20
	v_add_f32_e32 v14, v14, v24
	v_fma_f32 v32, 0x3fb8aa3b, v9, -v18
	v_cvt_i32_f32_e32 v31, v31
	v_cvt_i32_f32_e32 v35, v35
	v_exp_f32_e32 v12, v12
	v_exp_f32_e32 v14, v14
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v12, v12, v21
	v_ldexp_f32 v14, v14, v25
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v12, 0, v12, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v4
	v_dual_fmac_f32 v34, 0x32a5705f, v10 :: v_dual_cndmask_b32 v13, 0, v13
	v_dual_fmac_f32 v30, 0x32a5705f, v8 :: v_dual_add_f32 v19, v19, v34
	v_sub_f32_e32 v16, v16, v29
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v3
	s_delay_alu instid0(VALU_DEP_3)
	v_dual_sub_f32 v18, v18, v33 :: v_dual_add_f32 v17, v17, v30
	v_cvt_i32_f32_e32 v29, v29
	v_exp_f32_e32 v19, v19
	v_cndmask_b32_e32 v3, 0x7f800000, v12, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v5
	v_exp_f32_e32 v17, v17
	v_fmac_f32_e32 v32, 0x32a5705f, v9
	v_cvt_i32_f32_e32 v33, v33
	v_dual_add_f32 v1, v1, v3 :: v_dual_cndmask_b32 v12, 0, v14
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v4
	s_delay_alu instid0(TRANS32_DEP_2)
	v_ldexp_f32 v19, v19, v35
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v17, v17, v31
	v_add_f32_e32 v16, v16, v28
	v_cndmask_b32_e32 v4, 0x7f800000, v13, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_exp_f32_e32 v16, v16
	v_add_f32_e32 v1, v1, v4
	v_cndmask_b32_e32 v13, 0, v15, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v5
	v_cndmask_b32_e32 v5, 0x7f800000, v12, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v7
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v16, v16, v29
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_dual_add_f32 v1, v1, v5 :: v_dual_cndmask_b32 v12, 0, v16
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v6
	v_cndmask_b32_e32 v6, 0x7f800000, v13, vcc_lo
	v_add_f32_e32 v18, v18, v32
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v8
	v_add_f32_e32 v1, v1, v6
	s_delay_alu instid0(VALU_DEP_3)
	v_exp_f32_e32 v18, v18
	v_cndmask_b32_e32 v13, 0, v17, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v7
	v_cndmask_b32_e32 v7, 0x7f800000, v12, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v9
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v18, v18, v33
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_dual_add_f32 v1, v1, v7 :: v_dual_cndmask_b32 v12, 0, v18
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v8
	v_cndmask_b32_e32 v8, 0x7f800000, v13, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v10
	v_add_f32_e32 v1, v1, v8
	v_cndmask_b32_e32 v13, 0, v19, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v9
	v_cndmask_b32_e32 v9, 0x7f800000, v12, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f32_e32 v1, v1, v9
	v_cndmask_b32_e32 v10, 0x7f800000, v13, vcc_lo
	v_add_f32_e32 v1, v1, v10
	ds_store_2addr_b32 v11, v3, v4 offset1:1
	ds_store_2addr_b32 v11, v5, v6 offset0:2 offset1:3
	ds_store_2addr_b32 v11, v7, v8 offset0:4 offset1:5
	ds_store_2addr_b32 v11, v9, v10 offset0:6 offset1:7
	s_cbranch_scc1 .LBB4_25
; %bb.26:
	v_cndmask_b32_e64 v3, 0, 1, s3
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccz .LBB4_28
	s_branch .LBB4_30
.LBB4_27:
	s_mov_b32 s4, 0
	v_cndmask_b32_e64 v3, 0, 1, s3
	s_and_not1_b32 vcc_lo, exec_lo, s3
	s_cbranch_vccnz .LBB4_30
.LBB4_28:
	s_lshl_b32 s3, s4, 2
	s_mov_b32 s4, s10
	s_addk_i32 s3, 0x100
	.p2align	6
.LBB4_29:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v4, s3
	s_add_i32 s4, s4, -1
	s_add_i32 s3, s3, 4
	s_cmp_lg_u32 s4, 0
	ds_load_b32 v5, v4
	s_waitcnt lgkmcnt(0)
	v_sub_f32_e32 v5, v5, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v6, 0x3fb8aa3b, v5
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v5
	v_fma_f32 v7, 0x3fb8aa3b, v5, -v6
	v_rndne_f32_e32 v8, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v7, 0x32a5705f, v5 :: v_dual_sub_f32 v6, v6, v8
	v_add_f32_e32 v6, v6, v7
	v_cvt_i32_f32_e32 v7, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v6, v6
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v6, v6, v7
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0x7f800000, v6, vcc_lo
	v_add_f32_e32 v1, v1, v5
	ds_store_b32 v4, v5
	s_cbranch_scc1 .LBB4_29
.LBB4_30:
	s_and_not1_b32 vcc_lo, exec_lo, s2
	s_mov_b32 s11, 0
	s_cbranch_vccnz .LBB4_33
; %bb.31:
	s_and_b32 s11, s16, 0x7ffffff8
	s_mov_b32 s19, 0
	s_movk_i32 s21, 0x100
.LBB4_32:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v2, s21
	s_add_i32 s19, s19, 8
	s_add_i32 s21, s21, 32
	s_cmp_lg_u32 s11, s19
	ds_load_2addr_b32 v[4:5], v2 offset1:1
	ds_load_2addr_b32 v[6:7], v2 offset0:2 offset1:3
	ds_load_2addr_b32 v[8:9], v2 offset0:4 offset1:5
	ds_load_2addr_b32 v[10:11], v2 offset0:6 offset1:7
	s_waitcnt lgkmcnt(3)
	v_div_scale_f32 v12, null, v1, v1, v4
	v_div_scale_f32 v14, null, v1, v1, v5
	s_waitcnt lgkmcnt(2)
	v_div_scale_f32 v16, null, v1, v1, v6
	v_div_scale_f32 v18, null, v1, v1, v7
	v_rcp_f32_e32 v28, v12
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v24, null, v1, v1, v10
	v_rcp_f32_e32 v29, v14
	v_div_scale_f32 v20, null, v1, v1, v8
	v_div_scale_f32 v22, null, v1, v1, v9
	v_rcp_f32_e32 v30, v16
	v_rcp_f32_e32 v31, v18
	v_rcp_f32_e32 v34, v24
	v_div_scale_f32 v26, null, v1, v1, v11
	v_rcp_f32_e32 v32, v20
	v_rcp_f32_e32 v33, v22
	v_fma_f32 v36, -v12, v28, 1.0
	v_fma_f32 v37, -v14, v29, 1.0
	v_rcp_f32_e32 v35, v26
	v_fma_f32 v38, -v16, v30, 1.0
	v_div_scale_f32 v13, vcc_lo, v4, v1, v4
	v_fma_f32 v39, -v18, v31, 1.0
	v_fmac_f32_e32 v28, v36, v28
	v_div_scale_f32 v15, s2, v5, v1, v5
	v_fma_f32 v42, -v24, v34, 1.0
	v_fmac_f32_e32 v29, v37, v29
	v_fma_f32 v40, -v20, v32, 1.0
	v_div_scale_f32 v17, s3, v6, v1, v6
	v_fma_f32 v41, -v22, v33, 1.0
	v_fmac_f32_e32 v30, v38, v30
	v_div_scale_f32 v19, s4, v7, v1, v7
	v_dual_fmac_f32 v31, v39, v31 :: v_dual_mul_f32 v36, v13, v28
	v_dual_fmac_f32 v34, v42, v34 :: v_dual_mul_f32 v37, v15, v29
	v_div_scale_f32 v21, s5, v8, v1, v8
	v_fma_f32 v43, -v26, v35, 1.0
	v_fmac_f32_e32 v32, v40, v32
	v_div_scale_f32 v23, s6, v9, v1, v9
	v_fmac_f32_e32 v33, v41, v33
	v_dual_mul_f32 v38, v17, v30 :: v_dual_mul_f32 v39, v19, v31
	v_fma_f32 v44, -v12, v36, v13
	v_div_scale_f32 v25, s7, v10, v1, v10
	v_fma_f32 v45, -v14, v37, v15
	v_div_scale_f32 v27, s8, v11, v1, v11
	v_dual_fmac_f32 v35, v43, v35 :: v_dual_mul_f32 v40, v21, v32
	v_mul_f32_e32 v41, v23, v33
	v_fma_f32 v46, -v16, v38, v17
	v_dual_fmac_f32 v36, v44, v28 :: v_dual_fmac_f32 v37, v45, v29
	v_fma_f32 v47, -v18, v39, v19
	v_dual_mul_f32 v42, v25, v34 :: v_dual_mul_f32 v43, v27, v35
	v_fma_f32 v48, -v20, v40, v21
	v_fma_f32 v49, -v22, v41, v23
	v_fmac_f32_e32 v38, v46, v30
	v_fma_f32 v12, -v12, v36, v13
	v_fmac_f32_e32 v39, v47, v31
	v_fma_f32 v50, -v24, v42, v25
	v_fma_f32 v13, -v14, v37, v15
	v_fma_f32 v51, -v26, v43, v27
	v_dual_fmac_f32 v40, v48, v32 :: v_dual_fmac_f32 v41, v49, v33
	v_fma_f32 v14, -v16, v38, v17
	v_div_fmas_f32 v12, v12, v28, v36
	s_mov_b32 vcc_lo, s2
	v_fma_f32 v15, -v18, v39, v19
	v_fmac_f32_e32 v42, v50, v34
	v_div_fmas_f32 v13, v13, v29, v37
	s_mov_b32 vcc_lo, s3
	v_fmac_f32_e32 v43, v51, v35
	v_fma_f32 v16, -v20, v40, v21
	v_div_fmas_f32 v14, v14, v30, v38
	s_mov_b32 vcc_lo, s4
	v_fma_f32 v17, -v22, v41, v23
	v_div_fixup_f32 v4, v12, v1, v4
	v_div_fmas_f32 v12, v15, v31, v39
	s_mov_b32 vcc_lo, s5
	v_fma_f32 v18, -v24, v42, v25
	v_div_fixup_f32 v5, v13, v1, v5
	v_div_fmas_f32 v13, v16, v32, v40
	s_mov_b32 vcc_lo, s6
	v_fma_f32 v19, -v26, v43, v27
	v_div_fixup_f32 v6, v14, v1, v6
	v_div_fmas_f32 v14, v17, v33, v41
	s_mov_b32 vcc_lo, s7
	v_div_fixup_f32 v7, v12, v1, v7
	v_div_fmas_f32 v15, v18, v34, v42
	s_mov_b32 vcc_lo, s8
	v_div_fixup_f32 v8, v13, v1, v8
	v_div_fmas_f32 v16, v19, v35, v43
	v_div_fixup_f32 v9, v14, v1, v9
	v_div_fixup_f32 v10, v15, v1, v10
	s_delay_alu instid0(VALU_DEP_3)
	v_div_fixup_f32 v11, v16, v1, v11
	ds_store_2addr_b32 v2, v4, v5 offset1:1
	ds_store_2addr_b32 v2, v6, v7 offset0:2 offset1:3
	ds_store_2addr_b32 v2, v8, v9 offset0:4 offset1:5
	ds_store_2addr_b32 v2, v10, v11 offset0:6 offset1:7
	s_cbranch_scc1 .LBB4_32
.LBB4_33:
	v_cmp_ne_u32_e32 vcc_lo, 1, v3
	s_cbranch_vccnz .LBB4_36
; %bb.34:
	s_lshl_b32 s2, s11, 2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s2, 0x100
	.p2align	6
.LBB4_35:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v2, s2
	s_add_i32 s10, s10, -1
	s_add_i32 s2, s2, 4
	s_cmp_lg_u32 s10, 0
	ds_load_b32 v3, v2
	s_waitcnt lgkmcnt(0)
	v_div_scale_f32 v4, null, v1, v1, v3
	v_div_scale_f32 v7, vcc_lo, v3, v1, v3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v5, v4
	s_waitcnt_depctr 0xfff
	v_fma_f32 v6, -v4, v5, 1.0
	v_fmac_f32_e32 v5, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v6, v7, v5
	v_fma_f32 v8, -v4, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v6, v8, v5
	v_fma_f32 v4, -v4, v6, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v4, v4, v5, v6
	v_div_fixup_f32 v3, v4, v1, v3
	ds_store_b32 v2, v3
	s_cbranch_scc1 .LBB4_35
.LBB4_36:
	s_or_b32 exec_lo, exec_lo, s9
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s20, v0
	s_cbranch_execz .LBB4_47
; %bb.37:
	s_load_b32 s0, s[0:1], 0x44
	s_mul_i32 s1, s20, s17
	s_mul_hi_i32 s3, s24, s20
	s_mul_hi_i32 s5, s23, s1
	s_mul_i32 s4, s23, s1
	s_mul_i32 s2, s24, s20
	s_lshl_b64 s[4:5], s[4:5], 2
	s_mul_i32 s6, s20, s18
	s_add_u32 s4, s14, s4
	s_addc_u32 s5, s15, s5
	s_lshl_b64 s[2:3], s[2:3], 2
	s_mul_hi_i32 s19, s20, s25
	s_add_u32 s8, s4, s2
	s_addc_u32 s9, s5, s3
	s_ashr_i32 s7, s6, 31
	s_and_b32 s10, s16, 3
	s_mul_i32 s18, s20, s25
	s_mov_b32 s1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s11, s0, 0xffff
	s_cmp_gt_u32 s16, 3
	s_cselect_b32 s14, -1, 0
	s_and_b32 s15, s16, 0x7ffffffc
	s_cmp_lg_u32 s10, 0
	s_mul_hi_i32 s16, s6, 12
	s_cselect_b32 s17, -1, 0
	s_lshl_b64 s[2:3], s[18:19], 2
	s_mul_i32 s18, s6, 12
	s_add_u32 s12, s12, s2
	s_addc_u32 s13, s13, s3
	s_lshl_b64 s[2:3], s[6:7], 4
	s_lshl_b64 s[4:5], s[6:7], 3
	s_lshl_b64 s[6:7], s[6:7], 2
	s_branch .LBB4_39
.LBB4_38:                               ;   in Loop: Header=BB4_39 Depth=1
	v_add_nc_u32_e32 v0, s11, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, s0, s8, v1
	v_add_co_ci_u32_e64 v2, null, s9, v2, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s20, v0
	global_store_b32 v[1:2], v5, off
	s_or_b32 s1, vcc_lo, s1
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB4_47
.LBB4_39:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB4_42 Depth 2
                                        ;     Child Loop BB4_46 Depth 2
	v_ashrrev_i32_e32 v1, 31, v0
	v_mov_b32_e32 v5, 0
	s_and_not1_b32 vcc_lo, exec_lo, s22
	s_delay_alu instid0(VALU_DEP_2)
	v_lshlrev_b64 v[1:2], 2, v[0:1]
	s_cbranch_vccnz .LBB4_38
; %bb.40:                               ;   in Loop: Header=BB4_39 Depth=1
	s_and_not1_b32 vcc_lo, exec_lo, s14
	s_cbranch_vccnz .LBB4_44
; %bb.41:                               ;   in Loop: Header=BB4_39 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s12, v1
	v_add_co_ci_u32_e64 v4, null, s13, v2, vcc_lo
	v_mov_b32_e32 v5, 0
	s_mov_b32 s0, 0
	s_movk_i32 s19, 0x100
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB4_42:                               ;   Parent Loop BB4_39 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_add_co_u32 v6, vcc_lo, v3, s6
	global_load_b32 v10, v[3:4], off
	v_add_co_ci_u32_e64 v7, null, s7, v4, vcc_lo
	v_add_co_u32 v8, vcc_lo, v3, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s5, v4, vcc_lo
	global_load_b32 v11, v[6:7], off
	v_add_co_u32 v6, vcc_lo, v3, s18
	v_add_co_ci_u32_e64 v7, null, s16, v4, vcc_lo
	s_clause 0x1
	global_load_b32 v12, v[8:9], off
	global_load_b32 v13, v[6:7], off
	v_mov_b32_e32 v8, s19
	ds_load_2addr_b32 v[6:7], v8 offset1:1
	ds_load_2addr_b32 v[8:9], v8 offset0:2 offset1:3
	v_add_co_u32 v3, vcc_lo, v3, s2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s3, v4, vcc_lo
	s_add_i32 s0, s0, 4
	s_add_i32 s19, s19, 16
	s_cmp_eq_u32 s15, s0
	s_waitcnt vmcnt(3) lgkmcnt(1)
	v_fmac_f32_e32 v5, v6, v10
	s_waitcnt vmcnt(2)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v5, v7, v11
	s_waitcnt vmcnt(1) lgkmcnt(0)
	v_fmac_f32_e32 v5, v8, v12
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_fmac_f32_e32 v5, v9, v13
	s_cbranch_scc0 .LBB4_42
; %bb.43:                               ;   in Loop: Header=BB4_39 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s19, s15
	s_and_not1_b32 vcc_lo, exec_lo, s17
	s_cbranch_vccz .LBB4_45
	s_branch .LBB4_38
.LBB4_44:                               ;   in Loop: Header=BB4_39 Depth=1
	v_mov_b32_e32 v5, 0
	s_mov_b32 s19, 0
	s_and_not1_b32 vcc_lo, exec_lo, s17
	s_cbranch_vccnz .LBB4_38
.LBB4_45:                               ;   in Loop: Header=BB4_39 Depth=1
	s_lshl_b32 s0, s19, 2
	s_mul_i32 s21, s7, s19
	s_mul_hi_u32 s23, s6, s19
	s_mul_i32 s19, s6, s19
	s_addk_i32 s0, 0x100
	s_add_i32 s23, s23, s21
	s_add_u32 s19, s12, s19
	s_addc_u32 s21, s13, s23
	v_add_co_u32 v3, vcc_lo, s19, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s21, v2, vcc_lo
	s_mov_b32 s19, s10
.LBB4_46:                               ;   Parent Loop BB4_39 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b32 v6, v[3:4], off
	v_mov_b32_e32 v7, s0
	v_add_co_u32 v3, vcc_lo, v3, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v4, null, s7, v4, vcc_lo
	ds_load_b32 v7, v7
	s_add_i32 s19, s19, -1
	s_add_i32 s0, s0, 4
	s_cmp_lg_u32 s19, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fmac_f32_e32 v5, v7, v6
	s_cbranch_scc1 .LBB4_46
	s_branch .LBB4_38
.LBB4_47:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii
		.amdhsa_group_segment_fixed_size 256
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
		.amdhsa_next_free_vgpr 52
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
		.amdhsa_inst_pref_size 34
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii,"axG",@progbits,_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii,comdat
.Lfunc_end4:
	.size	_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii, .Lfunc_end4-_Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii
                                        ; -- End function
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.num_vgpr, 52
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.num_agpr, 0
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.numbered_sgpr, 36
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.num_named_barrier, 0
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.private_seg_size, 0
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.uses_vcc, 1
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.uses_flat_scratch, 0
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.has_dyn_sized_stack, 0
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.has_recursion, 0
	.set _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4320
; TotalNumSgprs: 38
; NumVgprs: 52
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 256 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 6
; NumSGPRsForWavesPerEU: 38
; NumVGPRsForWavesPerEU: 52
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii,"axG",@progbits,_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii,comdat
	.protected	_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii ; -- Begin function _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii
	.globl	_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii
	.p2align	8
	.type	_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii,@function
_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii: ; @_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii
; %bb.0:
	s_clause 0x1
	s_load_b128 s[12:15], s[0:1], 0x20
	s_load_b64 s[16:17], s[0:1], 0x30
	s_abs_i32 s6, s2
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
	s_sub_i32 s44, s4, s5
	s_ashr_i32 s10, s14, 31
	s_mul_i32 s5, s44, s13
	v_rcp_iflag_f32_e32 v1, v1
	s_sub_i32 s45, s2, s5
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
	s_abs_i32 s6, s45
	s_xor_b32 s2, s2, s7
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_sub_i32 s2, s2, s7
	s_abs_i32 s3, s2
	s_xor_b32 s2, s45, s2
	v_cvt_f32_u32_e32 v1, s3
	s_sub_i32 s5, 0, s3
	s_ashr_i32 s2, s2, 31
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
	s_mul_hi_u32 s18, s6, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_i32 s4, s18, s3
	s_add_i32 s20, s18, 1
	s_sub_i32 s19, s6, s4
	s_load_b256 s[4:11], s[0:1], 0x0
	s_sub_i32 s21, s19, s3
	s_cmp_ge_u32 s19, s3
	s_cselect_b32 s18, s20, s18
	s_cselect_b32 s19, s21, s19
	s_add_i32 s20, s18, 1
	s_cmp_ge_u32 s19, s3
	s_mov_b32 s19, 0
	s_cselect_b32 s3, s20, s18
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_xor_b32 s3, s3, s2
	s_sub_i32 s46, s3, s2
	s_cmp_lt_i32 s12, 1
	v_cmp_eq_u32_e64 s2, 0, v0
	s_cselect_b32 s18, -1, 0
	s_cmp_gt_i32 s12, 0
	s_cselect_b32 s33, -1, 0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_b32 vcc_lo, exec_lo, s33
	s_cbranch_vccz .LBB5_16
; %bb.1:
	s_mul_i32 s3, s15, s13
	v_mbcnt_lo_u32_b32 v1, -1, 0
	s_mul_hi_i32 s21, s44, s3
	s_mul_i32 s20, s44, s3
	s_mul_hi_i32 s23, s45, s15
	s_lshl_b64 s[20:21], s[20:21], 3
	s_mul_i32 s22, s45, s15
	s_waitcnt lgkmcnt(0)
	s_add_u32 s3, s4, s20
	s_addc_u32 s21, s5, s21
	s_lshl_b64 s[4:5], s[22:23], 3
	s_mul_i32 s22, s15, s14
	s_add_u32 s20, s3, s4
	v_cmp_gt_u32_e64 s3, 24, v1
	s_addc_u32 s21, s21, s5
	s_mul_hi_i32 s5, s46, s15
	s_mul_i32 s4, s46, s15
	v_and_b32_e32 v5, 31, v0
	v_cndmask_b32_e64 v2, 0, 8, s3
	v_cmp_gt_u32_e64 s3, 28, v1
	s_lshl_b64 s[4:5], s[4:5], 3
	s_ashr_i32 s23, s22, 31
	s_add_u32 s24, s6, s4
	s_addc_u32 s25, s7, s5
	v_cndmask_b32_e64 v3, 0, 4, s3
	v_cmp_gt_u32_e64 s3, 30, v1
	s_add_u32 s6, s0, 56
	s_addc_u32 s7, s1, 0
	v_lshl_or_b32 v6, v1, 2, 64
	v_add_lshl_u32 v7, v2, v1, 2
	v_cndmask_b32_e64 v4, 0, 2, s3
	v_cmp_ne_u32_e64 s3, 31, v1
	v_add_lshl_u32 v8, v3, v1, 2
	v_lshrrev_b32_e32 v11, 2, v0
	v_cmp_gt_u32_e64 s4, 32, v0
	v_add_lshl_u32 v9, v4, v1, 2
	v_add_co_ci_u32_e64 v10, null, 0, v1, s3
	v_cmp_eq_u32_e64 s3, 0, v5
	v_dual_mov_b32 v13, 0 :: v_dual_lshlrev_b32 v12, 3, v5
	s_delay_alu instid0(VALU_DEP_3)
	v_lshlrev_b32_e32 v10, 2, v10
	s_cmp_lt_i32 s44, s17
	v_cmp_gt_i32_e32 vcc_lo, s15, v0
	s_cselect_b32 s17, -1, 0
	s_branch .LBB5_3
.LBB5_2:                                ;   in Loop: Header=BB5_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_add_i32 s19, s19, 1
	s_waitcnt lgkmcnt(0)
	s_cmp_eq_u32 s19, s12
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB5_16
.LBB5_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB5_5 Depth 2
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_and_saveexec_b32 s26, vcc_lo
	s_cbranch_execz .LBB5_7
; %bb.4:                                ;   in Loop: Header=BB5_3 Depth=1
	s_load_b32 s5, s[6:7], 0xc
	s_mul_i32 s27, s19, s23
	s_mul_hi_u32 s29, s19, s22
	s_mul_i32 s28, s19, s22
	s_add_i32 s29, s29, s27
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, v0
	s_lshl_b64 s[28:29], s[28:29], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s27, s24, s28
	s_addc_u32 s28, s25, s29
	s_mov_b32 s29, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s30, s5, 0xffff
	.p2align	6
.LBB5_5:                                ;   Parent Loop BB5_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[14:15], 3, v[3:4]
	v_add_nc_u32_e32 v3, s30, v3
	v_add_co_u32 v16, s5, s20, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v17, null, s21, v15, s5
	v_add_co_u32 v14, s5, s27, v14
	v_add_co_ci_u32_e64 v15, null, s28, v15, s5
	global_load_b64 v[16:17], v[16:17], off
	global_load_b64 v[14:15], v[14:15], off
	v_cmp_le_i32_e64 s5, s15, v3
	s_or_b32 s29, s5, s29
	s_waitcnt vmcnt(0)
	v_fma_f64 v[1:2], v[16:17], v[14:15], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s29
	s_cbranch_execnz .LBB5_5
; %bb.6:                                ;   in Loop: Header=BB5_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s29
.LBB5_7:                                ;   in Loop: Header=BB5_3 Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_or_b32 exec_lo, exec_lo, s26
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
	s_and_saveexec_b32 s5, s3
	s_cbranch_execz .LBB5_9
; %bb.8:                                ;   in Loop: Header=BB5_3 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v11, v[1:2]
.LBB5_9:                                ;   in Loop: Header=BB5_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s26, s4
	s_cbranch_execz .LBB5_14
; %bb.10:                               ;   in Loop: Header=BB5_3 Depth=1
	s_load_b32 s5, s[6:7], 0xc
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s27, exec_lo
	s_waitcnt lgkmcnt(0)
	s_and_b32 s5, s5, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_add_i32 s5, s5, 31
	s_lshr_b32 s5, s5, 5
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmpx_gt_u32_e64 s5, v5
; %bb.11:                               ;   in Loop: Header=BB5_3 Depth=1
	ds_load_b64 v[1:2], v12
; %bb.12:                               ;   in Loop: Header=BB5_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s27
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
	s_and_b32 exec_lo, exec_lo, s3
	s_cbranch_execz .LBB5_14
; %bb.13:                               ;   in Loop: Header=BB5_3 Depth=1
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	ds_store_b64 v13, v[1:2]
.LBB5_14:                               ;   in Loop: Header=BB5_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s26
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	ds_load_b64 v[1:2], v13
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB5_2
; %bb.15:                               ;   in Loop: Header=BB5_3 Depth=1
	s_lshl_b32 s26, s19, 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(SALU_CYCLE_1)
	s_addk_i32 s26, 0x200
	s_cmp_gt_i32 s19, s44
	v_mov_b32_e32 v3, s26
	s_cselect_b32 s27, -1, 0
	s_and_b32 s27, s17, s27
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cndmask_b32_e64 v2, v2, 0xc6293e59, s27
	v_cndmask_b32_e64 v1, v1, 0x39a08cea, s27
	ds_store_b64 v3, v[1:2]
	s_branch .LBB5_2
.LBB5_16:
	v_cmp_eq_u32_e32 vcc_lo, 0, v0
	s_xor_b32 s2, s18, -1
	s_waitcnt lgkmcnt(0)
	s_mov_b32 s6, 0
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s5, s2
	s_cbranch_execz .LBB5_34
; %bb.17:
	s_add_i32 s4, s12, -1
	s_mov_b32 s3, 0xfe37e43c
	s_cmp_lt_u32 s4, 7
	s_mov_b32 s2, 0x8800759c
	s_cbranch_scc1 .LBB5_20
; %bb.18:
	s_and_b32 s6, s12, 0x7ffffff8
	s_mov_b32 s7, 0
	s_movk_i32 s15, 0x200
.LBB5_19:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v9, s15
	ds_load_2addr_b64 v[1:4], v9 offset1:1
	ds_load_2addr_b64 v[5:8], v9 offset0:2 offset1:3
	s_waitcnt lgkmcnt(1)
	v_readfirstlane_b32 s18, v1
	v_readfirstlane_b32 s19, v2
	v_readfirstlane_b32 s20, v3
	v_readfirstlane_b32 s21, v4
	s_delay_alu instid0(VALU_DEP_3)
	v_cmp_gt_f64_e64 s17, s[18:19], s[2:3]
	s_and_b32 s17, s17, exec_lo
	s_cselect_b32 s3, s19, s3
	s_cselect_b32 s2, s18, s2
	s_waitcnt lgkmcnt(0)
	v_readfirstlane_b32 s18, v5
	v_cmp_gt_f64_e64 s17, s[20:21], s[2:3]
	v_readfirstlane_b32 s19, v6
	s_and_b32 s17, s17, exec_lo
	s_cselect_b32 s3, s21, s3
	s_cselect_b32 s2, s20, s2
	v_readfirstlane_b32 s20, v7
	v_cmp_gt_f64_e64 s17, s[18:19], s[2:3]
	v_readfirstlane_b32 s21, v8
	ds_load_2addr_b64 v[1:4], v9 offset0:4 offset1:5
	ds_load_2addr_b64 v[5:8], v9 offset0:6 offset1:7
	s_and_b32 s17, s17, exec_lo
	s_cselect_b32 s3, s19, s3
	s_cselect_b32 s2, s18, s2
	s_waitcnt lgkmcnt(1)
	v_readfirstlane_b32 s18, v1
	v_cmp_gt_f64_e64 s17, s[20:21], s[2:3]
	v_readfirstlane_b32 s19, v2
	s_and_b32 s17, s17, exec_lo
	s_cselect_b32 s3, s21, s3
	s_cselect_b32 s2, s20, s2
	v_readfirstlane_b32 s20, v3
	v_cmp_gt_f64_e64 s17, s[18:19], s[2:3]
	v_readfirstlane_b32 s21, v4
	s_and_b32 s17, s17, exec_lo
	s_cselect_b32 s3, s19, s3
	s_cselect_b32 s2, s18, s2
	s_waitcnt lgkmcnt(0)
	v_readfirstlane_b32 s18, v5
	v_cmp_gt_f64_e64 s17, s[20:21], s[2:3]
	v_readfirstlane_b32 s19, v6
	s_and_b32 s17, s17, exec_lo
	s_cselect_b32 s3, s21, s3
	s_cselect_b32 s2, s20, s2
	v_readfirstlane_b32 s20, v7
	v_cmp_gt_f64_e64 s17, s[18:19], s[2:3]
	v_readfirstlane_b32 s21, v8
	s_and_b32 s17, s17, exec_lo
	s_cselect_b32 s3, s19, s3
	s_cselect_b32 s2, s18, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_f64_e64 s17, s[20:21], s[2:3]
	s_and_b32 s17, s17, exec_lo
	s_cselect_b32 s3, s21, s3
	s_cselect_b32 s2, s20, s2
	s_add_i32 s7, s7, 8
	s_add_i32 s15, s15, 64
	s_cmp_eq_u32 s6, s7
	s_cbranch_scc0 .LBB5_19
.LBB5_20:
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_and_b32 s7, s12, 7
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s7, 0
	s_cbranch_scc1 .LBB5_23
; %bb.21:
	v_dual_mov_b32 v4, s3 :: v_dual_mov_b32 v3, s2
	s_lshl_b32 s2, s6, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s2, 0x200
.LBB5_22:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v1, s2
	s_add_i32 s7, s7, -1
	s_add_i32 s2, s2, 8
	s_cmp_lg_u32 s7, 0
	ds_load_b64 v[1:2], v1
	s_waitcnt lgkmcnt(0)
	v_cmp_gt_f64_e32 vcc_lo, v[1:2], v[3:4]
	v_dual_cndmask_b32 v4, v4, v2 :: v_dual_cndmask_b32 v3, v3, v1
	s_cbranch_scc1 .LBB5_22
.LBB5_23:
	s_cmp_eq_u32 s4, 0
	s_mov_b32 s15, 0
	s_cbranch_scc1 .LBB5_46
; %bb.24:
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s18, 0xfefa39ef
	s_mov_b32 s20, 0x3b39803f
	s_mov_b32 s22, 0xfca7ab0c
	s_mov_b32 s24, 0x6a5dcb37
	s_mov_b32 s26, 0x623fde64
	s_mov_b32 s28, 0x7c89e6b0
	s_mov_b32 s30, 0x14761f6e
	s_mov_b32 s34, 0x1852b7b0
	s_mov_b32 s36, 0x11122322
	s_mov_b32 s38, 0x555502a1
	s_mov_b32 s40, 0x55555511
	s_mov_b32 s42, 11
	s_and_b32 s15, s12, 0x7ffffffe
	s_mov_b32 s17, 0
	s_movk_i32 s47, 0x200
	s_mov_b32 s7, 0x3ff71547
	s_mov_b32 s19, 0xbfe62e42
	s_mov_b32 s21, 0xbc7abc9e
	s_mov_b32 s23, 0x3e928af3
	s_mov_b32 s25, 0x3e5ade15
	s_mov_b32 s27, 0x3ec71dee
	s_mov_b32 s29, 0x3efa0199
	s_mov_b32 s31, 0x3f2a01a0
	s_mov_b32 s35, 0x3f56c16c
	s_mov_b32 s37, 0x3f811111
	s_mov_b32 s39, 0x3fa55555
	s_mov_b32 s41, 0x3fc55555
	s_mov_b32 s43, 0x3fe00000
.LBB5_25:                               ; =>This Inner Loop Header: Depth=1
	v_mov_b32_e32 v21, s47
	s_add_i32 s17, s17, 2
	s_add_i32 s47, s47, 16
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
	v_fma_f64 v[13:14], v[9:10], s[18:19], v[5:6]
	v_fma_f64 v[15:16], v[11:12], s[18:19], v[7:8]
	v_cvt_i32_f64_e32 v22, v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], s[20:21], v[13:14]
	v_fma_f64 v[15:16], v[11:12], s[20:21], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], s[24:25], s[22:23]
	v_fma_f64 v[19:20], v[15:16], s[24:25], s[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[26:27]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[17:18], v[13:14], v[17:18], s[28:29]
	v_fma_f64 v[19:20], v[15:16], v[19:20], s[28:29]
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
	s_cmp_lg_u32 s15, s17
	s_delay_alu instid0(VALU_DEP_3)
	v_add_f64 v[1:2], v[1:2], v[5:6]
	ds_store_2addr_b64 v21, v[5:6], v[7:8] offset1:1
	v_add_f64 v[1:2], v[1:2], v[7:8]
	s_cbranch_scc1 .LBB5_25
; %bb.26:
	s_bitcmp0_b32 s12, 0
	s_mov_b32 s6, 0
	s_cbranch_scc1 .LBB5_28
.LBB5_27:
	s_lshl_b32 s2, s15, 3
	s_mov_b32 s18, 0x6a5dcb37
	s_addk_i32 s2, 0x200
	s_mov_b32 s19, 0x3e5ade15
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
	v_fma_f64 v[9:10], v[7:8], s[18:19], s[2:3]
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
.LBB5_28:
	s_cmp_lt_u32 s12, 4
	s_cbranch_scc1 .LBB5_31
; %bb.29:
	s_and_b32 s6, s12, 0x7ffffffc
	s_mov_b32 s7, 0
	s_movk_i32 s15, 0x200
.LBB5_30:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mov_b32_e32 v43, s15
	s_add_i32 s7, s7, 4
	s_add_i32 s15, s15, 32
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
	s_cbranch_scc1 .LBB5_30
.LBB5_31:
	s_and_b32 s2, s12, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_cmp_eq_u32 s2, 0
	s_cbranch_scc1 .LBB5_34
; %bb.32:
	s_lshl_b32 s3, s6, 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_addk_i32 s3, 0x200
	.p2align	6
.LBB5_33:                               ; =>This Inner Loop Header: Depth=1
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
	s_cbranch_scc1 .LBB5_33
.LBB5_34:
	s_or_b32 exec_lo, exec_lo, s5
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s16, v0
	s_cbranch_execz .LBB5_45
; %bb.35:
	s_load_b32 s0, s[0:1], 0x44
	s_mul_i32 s1, s16, s13
	s_mul_hi_i32 s3, s45, s16
	s_mul_hi_i32 s5, s44, s1
	s_mul_i32 s4, s44, s1
	s_mul_i32 s2, s45, s16
	s_lshl_b64 s[4:5], s[4:5], 3
	s_mul_i32 s6, s16, s14
	s_add_u32 s4, s10, s4
	s_addc_u32 s5, s11, s5
	s_lshl_b64 s[2:3], s[2:3], 3
	s_mul_hi_i32 s21, s16, s46
	s_add_u32 s10, s4, s2
	s_addc_u32 s11, s5, s3
	s_ashr_i32 s7, s6, 31
	s_and_b32 s13, s12, 3
	s_mul_i32 s20, s16, s46
	s_mov_b32 s1, 0
	s_mul_hi_i32 s17, s6, 24
	s_mul_i32 s19, s6, 24
	s_waitcnt lgkmcnt(0)
	s_and_b32 s14, s0, 0xffff
	s_cmp_gt_u32 s12, 3
	s_cselect_b32 s15, -1, 0
	s_and_b32 s12, s12, 0x7ffffffc
	s_cmp_lg_u32 s13, 0
	s_cselect_b32 s18, -1, 0
	s_lshl_b64 s[2:3], s[20:21], 3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_add_u32 s8, s8, s2
	s_addc_u32 s9, s9, s3
	s_lshl_b64 s[2:3], s[6:7], 5
	s_lshl_b64 s[4:5], s[6:7], 4
	s_lshl_b64 s[6:7], s[6:7], 3
	s_branch .LBB5_37
.LBB5_36:                               ;   in Loop: Header=BB5_37 Depth=1
	v_add_nc_u32_e32 v0, s14, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v1, s0, s10, v1
	v_add_co_ci_u32_e64 v2, null, s11, v2, s0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_le_i32_e32 vcc_lo, s16, v0
	global_store_b64 v[1:2], v[3:4], off
	s_or_b32 s1, vcc_lo, s1
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execz .LBB5_45
.LBB5_37:                               ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB5_40 Depth 2
                                        ;     Child Loop BB5_44 Depth 2
	v_ashrrev_i32_e32 v1, 31, v0
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_and_not1_b32 vcc_lo, exec_lo, s33
	s_delay_alu instid0(VALU_DEP_3)
	v_lshlrev_b64 v[1:2], 3, v[0:1]
	s_cbranch_vccnz .LBB5_36
; %bb.38:                               ;   in Loop: Header=BB5_37 Depth=1
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_and_not1_b32 vcc_lo, exec_lo, s15
	s_cbranch_vccnz .LBB5_42
; %bb.39:                               ;   in Loop: Header=BB5_37 Depth=1
	v_add_co_u32 v5, vcc_lo, s8, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s9, v2, vcc_lo
	s_mov_b32 s0, 0
	s_movk_i32 s20, 0x200
	s_set_inst_prefetch_distance 0x1
	.p2align	6
.LBB5_40:                               ;   Parent Loop BB5_37 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[15:16], v[5:6], off
	v_add_co_u32 v7, vcc_lo, v5, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s7, v6, vcc_lo
	v_mov_b32_e32 v11, s20
	s_add_i32 s0, s0, 4
	s_add_i32 s20, s20, 32
	global_load_b64 v[17:18], v[7:8], off
	v_add_co_u32 v7, vcc_lo, v5, s4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v8, null, s5, v6, vcc_lo
	s_cmp_eq_u32 s12, s0
	global_load_b64 v[19:20], v[7:8], off
	v_add_co_u32 v7, vcc_lo, v5, s19
	v_add_co_ci_u32_e64 v8, null, s17, v6, vcc_lo
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
	s_cbranch_scc0 .LBB5_40
; %bb.41:                               ;   in Loop: Header=BB5_37 Depth=1
	s_set_inst_prefetch_distance 0x2
	s_mov_b32 s20, s12
	s_and_not1_b32 vcc_lo, exec_lo, s18
	s_cbranch_vccz .LBB5_43
	s_branch .LBB5_36
.LBB5_42:                               ;   in Loop: Header=BB5_37 Depth=1
	s_mov_b32 s20, 0
	s_and_not1_b32 vcc_lo, exec_lo, s18
	s_cbranch_vccnz .LBB5_36
.LBB5_43:                               ;   in Loop: Header=BB5_37 Depth=1
	s_lshl_b32 s0, s20, 3
	s_mul_i32 s21, s7, s20
	s_mul_hi_u32 s22, s6, s20
	s_mul_i32 s20, s6, s20
	s_addk_i32 s0, 0x200
	s_add_i32 s22, s22, s21
	s_add_u32 s20, s8, s20
	s_addc_u32 s21, s9, s22
	v_add_co_u32 v5, vcc_lo, s20, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s21, v2, vcc_lo
	s_mov_b32 s20, s13
.LBB5_44:                               ;   Parent Loop BB5_37 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[7:8], v[5:6], off
	v_mov_b32_e32 v9, s0
	v_add_co_u32 v5, vcc_lo, v5, s6
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s7, v6, vcc_lo
	ds_load_b64 v[9:10], v9
	s_add_i32 s20, s20, -1
	s_add_i32 s0, s0, 8
	s_cmp_lg_u32 s20, 0
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_fma_f64 v[3:4], v[9:10], v[7:8], v[3:4]
	s_cbranch_scc1 .LBB5_44
	s_branch .LBB5_36
.LBB5_45:
	s_endpgm
.LBB5_46:
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_bitcmp0_b32 s12, 0
	s_mov_b32 s6, 0
	s_cbranch_scc0 .LBB5_27
	s_branch .LBB5_28
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii
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
		.amdhsa_next_free_sgpr 48
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
	.section	.text._Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii,"axG",@progbits,_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii,comdat
.Lfunc_end5:
	.size	_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii, .Lfunc_end5-_Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii
                                        ; -- End function
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.num_vgpr, 44
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.num_agpr, 0
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.numbered_sgpr, 48
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.num_named_barrier, 0
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.private_seg_size, 0
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.uses_vcc, 1
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.uses_flat_scratch, 0
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.has_dyn_sized_stack, 0
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.has_recursion, 0
	.set _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4556
; TotalNumSgprs: 50
; NumVgprs: 44
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 512 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 5
; NumSGPRsForWavesPerEU: 50
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
	.section	.text._Z12rope_partialIfEvPT_iiiiPKS0_S3_,"axG",@progbits,_Z12rope_partialIfEvPT_iiiiPKS0_S3_,comdat
	.protected	_Z12rope_partialIfEvPT_iiiiPKS0_S3_ ; -- Begin function _Z12rope_partialIfEvPT_iiiiPKS0_S3_
	.globl	_Z12rope_partialIfEvPT_iiiiPKS0_S3_
	.p2align	8
	.type	_Z12rope_partialIfEvPT_iiiiPKS0_S3_,@function
_Z12rope_partialIfEvPT_iiiiPKS0_S3_:    ; @_Z12rope_partialIfEvPT_iiiiPKS0_S3_
; %bb.0:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x8
	s_load_b32 s3, s[0:1], 0x34
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v5
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s4, s10, 31
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[3:4], null, s3, s2, v[0:1]
	s_add_i32 s2, s10, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s16, s2, 1
	s_mul_hi_i32 s3, s16, s8
	s_mul_i32 s2, s16, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[3:4]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB6_17
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x18
	s_ashr_i32 s3, s16, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v6, s3, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[5:6]
	s_xor_b32 s8, exec_lo, s2
	s_cbranch_execz .LBB6_3
; %bb.2:
	s_ashr_i32 s12, s3, 31
	v_ashrrev_i32_e32 v2, 31, v4
	s_add_u32 s14, s16, s12
	s_mov_b32 s13, s12
	s_addc_u32 s15, s3, s12
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[14:15], s[14:15], s[12:13]
	v_add_co_u32 v5, vcc_lo, v3, v2
	v_cvt_f32_u32_e32 v0, s14
	v_cvt_f32_u32_e32 v1, s15
	s_sub_u32 s17, 0, s14
	s_subb_u32 s18, 0, s15
	v_add_co_ci_u32_e64 v6, null, v4, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v5, v2
	v_xor_b32_e32 v10, v6, v2
	v_xor_b32_e32 v2, s12, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_1)
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
	v_readfirstlane_b32 s13, v0
	s_mul_i32 s19, s17, s2
	s_mul_hi_u32 s21, s17, s13
	s_mul_i32 s20, s18, s13
	s_add_i32 s19, s21, s19
	s_mul_i32 s22, s17, s13
	s_add_i32 s19, s19, s20
	s_mul_hi_u32 s21, s13, s22
	s_mul_i32 s24, s13, s19
	s_mul_hi_u32 s23, s2, s22
	s_mul_i32 s20, s2, s22
	s_mul_hi_u32 s22, s13, s19
	s_add_u32 s21, s21, s24
	s_addc_u32 s22, 0, s22
	s_mul_hi_u32 s25, s2, s19
	s_add_u32 s20, s21, s20
	s_mul_i32 s19, s2, s19
	s_addc_u32 s20, s22, s23
	s_addc_u32 s21, s25, 0
	s_add_u32 s19, s20, s19
	s_addc_u32 s20, 0, s21
	s_add_u32 s13, s13, s19
	s_cselect_b32 s19, -1, 0
	s_mul_hi_u32 s21, s17, s13
	s_cmp_lg_u32 s19, 0
	s_mul_i32 s19, s17, s13
	s_addc_u32 s2, s2, s20
	s_mul_i32 s18, s18, s13
	s_mul_i32 s17, s17, s2
	s_mul_hi_u32 s20, s13, s19
	s_add_i32 s17, s21, s17
	s_mul_hi_u32 s21, s2, s19
	s_add_i32 s17, s17, s18
	s_mul_i32 s18, s2, s19
	s_mul_i32 s23, s13, s17
	s_mul_hi_u32 s22, s13, s17
	s_add_u32 s20, s20, s23
	s_addc_u32 s22, 0, s22
	s_mul_hi_u32 s19, s2, s17
	s_add_u32 s18, s20, s18
	s_mul_i32 s17, s2, s17
	s_addc_u32 s18, s22, s21
	s_addc_u32 s19, s19, 0
	s_add_u32 s17, s18, s17
	s_addc_u32 s18, 0, s19
	s_add_u32 s13, s13, s17
	s_cselect_b32 s17, -1, 0
	v_mul_hi_u32 v11, v9, s13
	s_cmp_lg_u32 s17, 0
	v_mad_u64_u32 v[5:6], null, v10, s13, 0
	s_addc_u32 s2, s2, s18
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[7:8], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v6, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v8, vcc_lo
	v_add_co_u32 v5, vcc_lo, v0, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v6, null, 0, v1, vcc_lo
	v_mul_lo_u32 v7, s15, v5
	v_mad_u64_u32 v[0:1], null, s14, v5, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v8, s14, v6
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v8, v7
	v_add_co_u32 v8, s2, v5, 2
	v_add_co_ci_u32_e64 v9, null, 0, v6, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v10, v1
	v_sub_co_u32 v11, s2, v0, s14
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s15, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s14, v11
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s15, v7
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s14, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s15, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s15, v7
	v_cndmask_b32_e32 v7, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v5, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v11, null, 0, v6, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s15, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v7
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v8 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v5, v0, s2
	v_cndmask_b32_e64 v1, v6, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v2
	v_xor_b32_e32 v1, v1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v2
	v_sub_co_ci_u32_e64 v1, null, v1, v2, vcc_lo
.LBB6_3:
	s_or_saveexec_b32 s2, s8
	s_waitcnt lgkmcnt(0)
	s_load_b32 s4, s[4:5], 0x0
	s_xor_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB6_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s16
	s_sub_i32 s5, 0, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s5, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s16
	v_add_nc_u32_e32 v2, 1, v0
	v_sub_nc_u32_e32 v1, v3, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s16, v1
	v_cmp_le_u32_e32 vcc_lo, s16, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s16, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v2, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
.LBB6_5:
	s_or_b32 exec_lo, exec_lo, s2
	s_abs_i32 s5, s11
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_3)
	v_mul_lo_u32 v8, v1, s16
	v_cvt_f32_u32_e32 v2, s5
	s_sub_i32 s2, 0, s5
	v_mul_lo_u32 v9, v0, s3
	v_sub_nc_u32_e32 v7, 0, v0
	v_rcp_iflag_f32_e32 v2, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_max_i32_e32 v7, v0, v7
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	v_cvt_u32_f32_e32 v5, v2
	v_mad_u64_u32 v[1:2], null, v0, s16, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v6, s2, v5
	s_ashr_i32 s2, s11, 31
	s_cmp_eq_u64 s[6:7], 0
	v_add3_u32 v2, v2, v9, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_co_u32 v3, vcc_lo, v3, v1
	v_mul_hi_u32 v6, v5, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_ci_u32_e64 v4, null, v4, v2, vcc_lo
	v_lshlrev_b64 v[1:2], 2, v[3:4]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v8, v5, v6
	v_mad_u64_u32 v[5:6], null, v7, v8, 0
	v_ashrrev_i32_e32 v8, 31, v0
	s_cbranch_scc1 .LBB6_7
; %bb.6:
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v9, vcc_lo, s6, v1
	v_add_co_ci_u32_e64 v10, null, s7, v2, vcc_lo
	global_load_b32 v5, v[9:10], off
	s_branch .LBB6_8
.LBB6_7:
	v_mov_b32_e32 v5, 1.0
.LBB6_8:
	v_cvt_f64_u32_e32 v[9:10], v4
	v_cvt_f64_u32_e32 v[11:12], v3
	s_mov_b32 s3, 0x3e76c4e1
	v_xor_b32_e32 v8, s2, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[9:10], v[9:10], 32
	v_add_f64 v[9:10], v[9:10], v[11:12]
	v_cvt_f64_i32_e32 v[11:12], s10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[9:10], v[9:10], -2.0
	v_div_scale_f64 v[13:14], null, v[11:12], v[11:12], v[9:10]
	v_div_scale_f64 v[19:20], vcc_lo, v[9:10], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[15:16], v[13:14]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[17:18], -v[13:14], v[15:16], 1.0
	v_fma_f64 v[15:16], v[15:16], v[17:18], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[17:18], -v[13:14], v[15:16], 1.0
	v_fma_f64 v[15:16], v[15:16], v[17:18], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[17:18], v[19:20], v[15:16]
	v_fma_f64 v[13:14], -v[13:14], v[17:18], v[19:20]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_div_fmas_f64 v[13:14], v[13:14], v[15:16], v[17:18]
	s_waitcnt lgkmcnt(0)
	v_cmp_neq_f32_e64 vcc_lo, s4, 1.0
	v_div_fixup_f64 v[9:10], v[13:14], v[11:12], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cvt_f32_f64_e32 v4, v[9:10]
	v_cndmask_b32_e32 v4, 1.0, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_neq_f32_e32 vcc_lo, 0, v4
	v_cndmask_b32_e64 v11, 1.0, s4, vcc_lo
	v_frexp_mant_f32_e64 v9, |v11|
	v_cmp_lt_f32_e64 s7, |v11|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_gt_f32_e32 vcc_lo, 0x3f2aaaab, v9
	v_cndmask_b32_e64 v10, 1.0, 2.0, vcc_lo
	v_mul_f32_e32 v9, v9, v10
	v_cmp_neq_f32_e64 s6, v4, |v4|
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v10, 1.0, v9
	v_add_f32_e32 v13, -1.0, v9
	v_add_f32_e32 v15, -1.0, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v9, v15
	v_rcp_f32_e32 v12, v10
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v14, v13, v12
	v_mul_f32_e32 v16, v10, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v10, v14, v10, -v16
	v_fmac_f32_e32 v10, v14, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v16, v10
	v_sub_f32_e32 v16, v9, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v15, v13, v9 :: v_dual_sub_f32 v10, v16, v10
	v_sub_f32_e32 v13, v13, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v13, v9
	v_add_f32_e32 v9, v10, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v15, v9
	v_mul_f32_e32 v9, v12, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v12, v14, v9
	v_sub_f32_e32 v10, v12, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_mul_f32 v13, v12, v12 :: v_dual_sub_f32 v14, v9, v10
	v_fma_f32 v15, v12, v12, -v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v9, v14, v14
	v_fmac_f32_e32 v15, v12, v9
	v_cvt_f64_f32_e64 v[9:10], |v11|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v16, v13, v15
	v_fmaak_f32 v17, s3, v16, 0x3e91f4c4
	v_sub_f32_e32 v13, v16, v13
	v_cmp_gt_f32_e64 s3, 0, v4
	v_mul_f32_e32 v20, v12, v16
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fmaak_f32 v17, v16, v17, 0x3ecccdef
	v_sub_f32_e32 v13, v15, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v18, v16, v17
	v_fma_f32 v15, v16, v17, -v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v15, v13, v17
	v_add_f32_e32 v17, v18, v15
	v_frexp_exp_i32_f64_e32 v9, v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v18, v17, v18
	v_sub_f32_e32 v10, v15, v18
	v_fma_f32 v18, v16, v12, -v20
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v10, 0x31739010, v10
	v_fmac_f32_e32 v18, v16, v14
	v_ldexp_f32 v14, v14, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_fmac_f32 v18, v13, v12 :: v_dual_add_f32 v19, 0x3f2aaaaa, v17
	v_add_f32_e32 v15, 0xbf2aaaaa, v19
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v15, v17, v15
	v_dual_add_f32 v10, v10, v15 :: v_dual_add_f32 v15, v20, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f32_e32 v13, v19, v10
	v_subrev_co_ci_u32_e64 v9, null, 0, v9, vcc_lo
	v_sub_f32_e32 v16, v19, v13
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_f32_e32 v19, v15, v20
	v_cvt_f32_i32_e32 v9, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f32_e32 v10, v10, v16
	v_dual_sub_f32 v18, v18, v19 :: v_dual_mul_f32 v17, v15, v13
	v_mul_lo_u32 v19, v6, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f32 v16, v15, v13, -v17
	v_sub_nc_u32_e32 v7, v7, v19
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fmac_f32_e32 v16, v15, v10
	v_ldexp_f32 v10, v12, 1
	v_fmac_f32_e32 v16, v18, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v12, v17, v16
	v_add_f32_e32 v13, v10, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v10, v13, v10 :: v_dual_sub_f32 v15, v12, v17
	v_sub_f32_e32 v10, v12, v10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v15, v16, v15
	v_dual_mul_f32 v17, 0x3f317218, v9 :: v_dual_add_f32 v12, v14, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f32 v16, 0x3f317218, v9, -v17
	v_dual_add_f32 v10, v12, v10 :: v_dual_fmamk_f32 v9, v9, 0xb102e308, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f32_e32 v14, v13, v10
	v_add_f32_e32 v12, v17, v9
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v13, v14, v13
	v_dual_add_f32 v15, v12, v14 :: v_dual_sub_f32 v10, v10, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v16, v15, v12 :: v_dual_sub_f32 v17, v12, v17
	v_sub_f32_e32 v18, v15, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_sub_f32 v13, v14, v16 :: v_dual_sub_f32 v12, v12, v18
	v_add_f32_e32 v12, v13, v12
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v9, v17
	v_add_f32_e32 v14, v9, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v12, v14, v12
	v_dual_sub_f32 v13, v14, v9 :: v_dual_add_f32 v16, v15, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v14, v14, v13
	v_dual_sub_f32 v10, v10, v13 :: v_dual_sub_f32 v13, v16, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v9, v9, v14
	v_add_f32_e32 v9, v10, v9
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v10, v12, v13
	v_add_f32_e32 v9, v9, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f32_e32 v10, v16, v9
	v_dual_sub_f32 v12, v10, v16 :: v_dual_mul_f32 v13, v4, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_f32_e32 v9, v9, v12
	v_fma_f32 v10, v4, v10, -v13
	v_cmp_class_f32_e64 vcc_lo, v13, 0x204
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v10, v4, v9
	v_add_f32_e32 v9, v13, v10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v12, v9, v13, vcc_lo
	v_cmp_eq_f32_e32 vcc_lo, 0x42b17218, v12
	v_cndmask_b32_e64 v14, 0, 0x37000000, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s5, v7
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v15, v12, v14
	v_mul_f32_e32 v16, 0x3fb8aa3b, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f32 v17, 0x3fb8aa3b, v15, -v16
	v_rndne_f32_e32 v18, v16
	v_dual_fmamk_f32 v17, v15, 0x32a5705f, v17 :: v_dual_sub_f32 v16, v16, v18
	v_cvt_i32_f32_e32 v18, v18
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_add_f32 v16, v16, v17 :: v_dual_add_nc_u32 v17, 1, v6
	v_exp_f32_e32 v16, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, v6, v17, vcc_lo
	v_sub_f32_e32 v9, v9, v13
	v_subrev_nc_u32_e32 v13, s5, v7
	v_cndmask_b32_e32 v7, v7, v13, vcc_lo
	v_mul_f32_e32 v13, 0.5, v4
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(TRANS32_DEP_1)
	v_sub_f32_e32 v9, v10, v9
	v_cmp_neq_f32_e64 vcc_lo, 0x7f800000, |v12|
	v_ldexp_f32 v10, v16, v18
	v_trunc_f32_e32 v12, v4
	v_trunc_f32_e32 v16, v13
	v_cndmask_b32_e32 v9, 0, v9, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2ce8ed0, v15
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_neq_f32_e64 s2, v16, v13
	v_cndmask_b32_e32 v10, 0, v10, vcc_lo
	v_cmp_nlt_f32_e32 vcc_lo, 0x42b17218, v15
	v_dual_add_f32 v9, v14, v9 :: v_dual_cndmask_b32 v10, 0x7f800000, v10
	v_cmp_eq_f32_e32 vcc_lo, v12, v4
	v_add_nc_u32_e32 v12, 1, v6
	v_cmp_le_u32_e64 s5, s5, v7
	s_delay_alu instid0(VALU_DEP_4)
	v_fma_f32 v9, v10, v9, v10
	v_cmp_class_f32_e64 s4, v10, 0x204
	s_and_b32 s2, vcc_lo, s2
	v_cndmask_b32_e64 v6, v6, v12, s5
	v_cndmask_b32_e64 v13, 1.0, v11, s2
	s_xor_b32 s5, s6, s7
	v_cndmask_b32_e64 v9, v9, v10, s4
	v_cmp_eq_f32_e64 s4, 0, v11
	v_xor_b32_e32 v6, v6, v8
	s_delay_alu instid0(VALU_DEP_3)
	v_bfi_b32 v7, 0x7fffffff, v9, v13
	v_cndmask_b32_e64 v9, 0x7f800000, 0, s5
	s_xor_b32 s3, s3, s4
	v_cndmask_b32_e64 v13, 0, v11, s2
	v_cndmask_b32_e64 v10, 0x7f800000, 0, s3
	v_cndmask_b32_e32 v12, 0x7fc00000, v7, vcc_lo
	v_cmp_neq_f32_e64 vcc_lo, |v11|, 1.0
	v_cmp_class_f32_e64 s2, v11, 0x204
	v_sub_nc_u32_e32 v6, v6, v8
	v_bfi_b32 v10, 0x7fffffff, v10, v13
	v_cndmask_b32_e32 v9, 1.0, v9, vcc_lo
	v_cmp_gt_f32_e32 vcc_lo, 0, v11
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cvt_f32_i32_e32 v6, v6
	v_cndmask_b32_e32 v7, v7, v12, vcc_lo
	v_cmp_class_f32_e64 vcc_lo, v4, 0x204
	v_cndmask_b32_e32 v7, v7, v9, vcc_lo
	s_or_b32 vcc_lo, s4, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, v7, v10, vcc_lo
	v_cmp_o_f32_e32 vcc_lo, v11, v4
	v_cndmask_b32_e32 v4, 0x7fc00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v4, v4, v6
	s_waitcnt vmcnt(0)
	v_div_scale_f32 v6, null, v5, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f32_e32 v7, v6
	s_waitcnt_depctr 0xfff
	v_fma_f32 v8, -v6, v7, 1.0
	v_fmac_f32_e32 v7, v8, v7
	v_div_scale_f32 v8, vcc_lo, v4, v5, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f32_e32 v9, v8, v7
	v_fma_f32 v10, -v6, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v9, v10, v7
	v_fma_f32 v6, -v6, v9, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f32 v6, v6, v7, v9
                                        ; implicit-def: $vgpr7
	v_div_fixup_f32 v4, v6, v5, v4
                                        ; implicit-def: $vgpr6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_and_b32_e32 v5, 0x7fffffff, v4
	v_cmp_ngt_f32_e64 s4, 0x48000000, |v4|
	v_lshrrev_b32_e32 v8, 23, v5
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s5, exec_lo, s2
	s_cbranch_execz .LBB6_10
; %bb.9:
	s_mov_b32 s2, 0x7fffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_and_or_b32 v18, v5, s2, 0x800000
	v_mad_u64_u32 v[6:7], null, 0xfe5163ab, v18, 0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_dual_mov_b32 v10, 0 :: v_dual_mov_b32 v9, v7
	v_add_nc_u32_e32 v7, 0xffffff88, v8
	v_mad_u64_u32 v[11:12], null, 0x3c439041, v18, v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v7
	v_cndmask_b32_e64 v16, 0, 0xffffffc0, vcc_lo
	v_mov_b32_e32 v9, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v7, v16, v7
	v_mad_u64_u32 v[12:13], null, 0xdb629599, v18, v[9:10]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s2, 31, v7
	v_mov_b32_e32 v9, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v17, 0, 0xffffffe0, s2
	v_cndmask_b32_e32 v6, v12, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[13:14], null, 0xf534ddc0, v18, v[9:10]
	v_add_nc_u32_e32 v7, v17, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e64 s3, 31, v7
	v_mov_b32_e32 v9, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[14:15], null, 0xfc2757d1, v18, v[9:10]
	v_mov_b32_e32 v9, v15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[15:16], null, 0x4e441529, v18, v[9:10]
	v_mov_b32_e32 v9, v16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[16:17], null, 0xa2f9836e, v18, v[9:10]
	v_cndmask_b32_e64 v9, 0, 0xffffffe0, s3
	v_dual_cndmask_b32 v10, v15, v13 :: v_dual_add_nc_u32 v7, v9, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v16, v16, v14 :: v_dual_cndmask_b32 v15, v17, v15
	v_dual_cndmask_b32 v14, v14, v12 :: v_dual_cndmask_b32 v9, v13, v11
	v_cmp_eq_u32_e32 vcc_lo, 0, v7
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v11, v16, v10, s2
	v_cndmask_b32_e64 v13, v15, v16, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v10, v10, v14, s2
	v_sub_nc_u32_e32 v15, 32, v7
	v_cndmask_b32_e64 v14, v14, v9, s2
	v_cndmask_b32_e64 v6, v9, v6, s2
	v_cndmask_b32_e64 v13, v13, v11, s3
	v_cndmask_b32_e64 v11, v11, v10, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v10, v10, v14, s3
	v_cndmask_b32_e64 v6, v14, v6, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v16, v13, v11, v15.l
	v_alignbit_b32 v12, v11, v10, v15.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v14, v10, v6, v15.l
	v_cndmask_b32_e32 v7, v16, v13, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v9, v12, v11 :: v_dual_cndmask_b32 v10, v14, v10
	v_bfe_u32 v11, v7, 29, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v12, v7, v9, 30
	v_alignbit_b32 v9, v9, v10, 30
	v_alignbit_b32 v6, v10, v6, 30
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v13, 0, v11
	v_xor_b32_e32 v12, v12, v13
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v9, v9, v13
	v_xor_b32_e32 v6, v6, v13
	v_lshrrev_b32_e32 v13, 29, v7
	v_lshrrev_b32_e32 v7, 30, v7
	v_clz_i32_u32_e32 v14, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v7, v11, v7
	v_min_u32_e32 v14, 32, v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v10, 31, v14
	v_lshlrev_b32_e32 v15, 23, v14
	v_alignbit_b32 v12, v12, v9, v10.l
	v_alignbit_b32 v6, v9, v6, v10.l
	v_lshlrev_b32_e32 v9, 31, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_alignbit_b32 v10, v12, v6, 9
	v_or_b32_e32 v13, 0.5, v9
	v_lshrrev_b32_e32 v12, 9, v12
	v_or_b32_e32 v9, 0x33000000, v9
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_clz_i32_u32_e32 v16, v10
	v_sub_nc_u32_e32 v13, v13, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_min_u32_e32 v15, 32, v16
	v_or_b32_e32 v12, v12, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_not_b32_e32 v13, v15
	v_mul_f32_e32 v16, 0x3fc90fda, v12
	v_add_lshl_u32 v14, v15, v14, 23
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v6, v10, v6, v13.l
	v_fma_f32 v10, 0x3fc90fda, v12, -v16
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v9, v9, v14
	v_lshrrev_b32_e32 v6, 9, v6
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fmamk_f32 v10, v12, 0x33a22168, v10
	v_or_b32_e32 v6, v9, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmac_f32_e32 v10, 0x3fc90fda, v6
	v_add_f32_e32 v6, v16, v10
	s_or_saveexec_b32 s2, s5
	v_mul_f32_e64 v11, 0x3f22f983, |v4|
	s_xor_b32 exec_lo, exec_lo, s2
	s_branch .LBB6_11
.LBB6_10:
	s_or_saveexec_b32 s2, s5
	v_mul_f32_e64 v11, 0x3f22f983, |v4|
	s_xor_b32 exec_lo, exec_lo, s2
.LBB6_11:
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f32_e32 v7, v11
	v_fma_f32 v6, 0xbfc90fda, v7, |v4|
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fmamk_f32 v6, v7, 0xb3a22168, v6
	v_fmamk_f32 v6, v7, 0xa7c234c4, v6
	v_cvt_i32_f32_e32 v7, v7
; %bb.12:
	s_or_b32 exec_lo, exec_lo, s2
                                        ; implicit-def: $vgpr10
                                        ; implicit-def: $vgpr9
	s_and_saveexec_b32 s2, s4
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s4, exec_lo, s2
	s_cbranch_execz .LBB6_14
; %bb.13:
	s_mov_b32 s2, 0x7fffff
	v_mov_b32_e32 v11, 0
	v_and_or_b32 v19, v5, s2, 0x800000
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mad_u64_u32 v[9:10], null, 0xfe5163ab, v19, 0
	v_mad_u64_u32 v[12:13], null, 0x3c439041, v19, v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v10, v13
	v_mad_u64_u32 v[13:14], null, 0xdb629599, v19, v[10:11]
	v_add_nc_u32_e32 v8, 0xffffff88, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_lt_u32_e32 vcc_lo, 63, v8
	v_mov_b32_e32 v10, v14
	v_cndmask_b32_e64 v17, 0, 0xffffffc0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[14:15], null, 0xf534ddc0, v19, v[10:11]
	v_cndmask_b32_e32 v9, v13, v9, vcc_lo
	v_add_nc_u32_e32 v8, v17, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mov_b32_e32 v10, v15
	v_cmp_lt_u32_e64 s2, 31, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[15:16], null, 0xfc2757d1, v19, v[10:11]
	v_cndmask_b32_e64 v18, 0, 0xffffffe0, s2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_nc_u32_e32 v8, v18, v8
	v_mov_b32_e32 v10, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_lt_u32_e64 s3, 31, v8
	v_mad_u64_u32 v[16:17], null, 0x4e441529, v19, v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mov_b32_e32 v10, v17
	v_mad_u64_u32 v[17:18], null, 0xa2f9836e, v19, v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v10, 0, 0xffffffe0, s3
	v_dual_cndmask_b32 v11, v16, v14 :: v_dual_add_nc_u32 v8, v10, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_dual_cndmask_b32 v17, v17, v15 :: v_dual_cndmask_b32 v16, v18, v16
	v_dual_cndmask_b32 v15, v15, v13 :: v_dual_cndmask_b32 v10, v14, v12
	v_cmp_eq_u32_e32 vcc_lo, 0, v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v12, v17, v11, s2
	v_cndmask_b32_e64 v14, v16, v17, s2
	s_delay_alu instid0(VALU_DEP_4)
	v_cndmask_b32_e64 v11, v11, v15, s2
	v_sub_nc_u32_e32 v16, 32, v8
	v_cndmask_b32_e64 v15, v15, v10, s2
	v_cndmask_b32_e64 v9, v10, v9, s2
	v_cndmask_b32_e64 v14, v14, v12, s3
	v_cndmask_b32_e64 v12, v12, v11, s3
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e64 v11, v11, v15, s3
	v_cndmask_b32_e64 v9, v15, v9, s3
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v17, v14, v12, v16.l
	v_alignbit_b32 v13, v12, v11, v16.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v15, v11, v9, v16.l
	v_cndmask_b32_e32 v8, v17, v14, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v10, v13, v12 :: v_dual_cndmask_b32 v11, v15, v11
	v_bfe_u32 v12, v8, 29, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_alignbit_b32 v13, v8, v10, 30
	v_alignbit_b32 v10, v10, v11, 30
	v_alignbit_b32 v9, v11, v9, 30
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_nc_u32_e32 v14, 0, v12
	v_xor_b32_e32 v13, v13, v14
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_xor_b32_e32 v10, v10, v14
	v_xor_b32_e32 v9, v9, v14
	v_lshrrev_b32_e32 v14, 29, v8
	v_lshrrev_b32_e32 v8, 30, v8
	v_clz_i32_u32_e32 v15, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_min_u32_e32 v15, 32, v15
	v_sub_nc_u32_e32 v11, 31, v15
	v_lshlrev_b32_e32 v16, 23, v15
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_alignbit_b32 v13, v13, v10, v11.l
	v_alignbit_b32 v9, v10, v9, v11.l
	v_lshlrev_b32_e32 v10, 31, v14
	v_alignbit_b32 v11, v13, v9, 9
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_or_b32_e32 v14, 0.5, v10
	v_lshrrev_b32_e32 v13, 9, v13
	v_or_b32_e32 v10, 0x33000000, v10
	v_clz_i32_u32_e32 v17, v11
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_sub_nc_u32_e32 v14, v14, v16
	v_min_u32_e32 v16, 32, v17
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_or_b32_e32 v13, v13, v14
	v_not_b32_e32 v14, v16
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f32_e32 v17, 0x3fc90fda, v13
	v_add_lshl_u32 v15, v16, v15, 23
	v_alignbit_b32 v9, v11, v9, v14.l
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f32 v11, 0x3fc90fda, v13, -v17
	v_sub_nc_u32_e32 v10, v10, v15
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshrrev_b32_e32 v9, 9, v9
	v_fmamk_f32 v11, v13, 0x33a22168, v11
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_or_b32_e32 v9, v10, v9
	v_dual_fmac_f32 v11, 0x3fc90fda, v9 :: v_dual_add_nc_u32 v10, v12, v8
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f32_e32 v9, v17, v11
                                        ; implicit-def: $vgpr11
	s_and_not1_saveexec_b32 s2, s4
	s_cbranch_execnz .LBB6_15
	s_branch .LBB6_16
.LBB6_14:
	s_and_not1_saveexec_b32 s2, s4
.LBB6_15:
	v_rndne_f32_e32 v8, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v9, 0xbfc90fda, v8, |v4|
	v_cvt_i32_f32_e32 v10, v8
	v_fmamk_f32 v9, v8, 0xb3a22168, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_fmamk_f32 v9, v8, 0xa7c234c4, v9
.LBB6_16:
	s_or_b32 exec_lo, exec_lo, s2
	s_load_b64 s[0:1], s[0:1], 0x0
	v_mad_i64_i32 v[11:12], null, v0, s9, 0
	v_dual_mul_f32 v8, v6, v6 :: v_dual_add_nc_u32 v13, s16, v3
	v_xor_b32_e32 v5, v5, v4
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ashrrev_i32_e32 v14, 31, v13
	v_lshlrev_b64 v[11:12], 2, v[11:12]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[13:14], 2, v[13:14]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v0, vcc_lo, s0, v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v12, vcc_lo
	s_mov_b32 s0, 0xb94c1982
	v_add_co_u32 v11, vcc_lo, v0, v13
	v_add_co_ci_u32_e64 v12, null, v3, v14, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v3, v2, vcc_lo
	s_clause 0x1
	global_load_b32 v2, v[11:12], off
	global_load_b32 v3, v[0:1], off
	v_dual_mul_f32 v14, v9, v9 :: v_dual_and_b32 v13, 1, v7
	s_mov_b32 s1, 0x37d75334
	v_dual_fmaak_f32 v16, s0, v8, 0x3c0881c4 :: v_dual_lshlrev_b32 v7, 30, v7
	v_fmaak_f32 v17, s1, v8, 0xbab64f3b
	s_delay_alu instid0(VALU_DEP_3)
	v_fmaak_f32 v18, s0, v14, 0x3c0881c4
	v_and_b32_e32 v15, 1, v10
	v_lshlrev_b32_e32 v10, 30, v10
	v_fmaak_f32 v16, v8, v16, 0xbe2aaa9d
	v_fmaak_f32 v19, s1, v14, 0xbab64f3b
	v_fmaak_f32 v18, v14, v18, 0xbe2aaa9d
	v_fmaak_f32 v17, v8, v17, 0x3d2aabf7
	v_cmp_eq_u32_e32 vcc_lo, 0, v13
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_dual_mul_f32 v16, v8, v16 :: v_dual_fmaak_f32 v19, v14, v19, 0x3d2aabf7
	v_dual_mul_f32 v18, v14, v18 :: v_dual_fmaak_f32 v17, v8, v17, 0xbf000004
	v_and_b32_e32 v10, 0x80000000, v10
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fmac_f32_e32 v6, v6, v16
	v_fmaak_f32 v19, v14, v19, 0xbf000004
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmac_f32_e32 v9, v9, v18
	v_fma_f32 v8, v8, v17, 1.0
	v_fma_f32 v14, v14, v19, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e64 v6, -v6, v8, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, 0, v15
	v_dual_cndmask_b32 v8, v14, v9 :: v_dual_and_b32 v7, 0x80000000, v7
	v_cmp_class_f32_e64 vcc_lo, v4, 0x1f8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_xor_b32_e32 v6, v7, v6
	v_xor3_b32 v5, v5, v10, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v4, 0x7fc00000, v6, vcc_lo
	v_cndmask_b32_e32 v5, 0x7fc00000, v5, vcc_lo
	s_waitcnt vmcnt(1)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_mul_f32_e32 v6, v2, v5
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v5, v3, v5
	v_fma_f32 v3, v4, v3, -v6
	s_delay_alu instid0(VALU_DEP_2)
	v_fmac_f32_e32 v5, v4, v2
	s_clause 0x1
	global_store_b32 v[0:1], v3, off
	global_store_b32 v[11:12], v5, off
.LBB6_17:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z12rope_partialIfEvPT_iiiiPKS0_S3_
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
		.amdhsa_next_free_vgpr 21
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
		.amdhsa_inst_pref_size 37
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z12rope_partialIfEvPT_iiiiPKS0_S3_,"axG",@progbits,_Z12rope_partialIfEvPT_iiiiPKS0_S3_,comdat
.Lfunc_end6:
	.size	_Z12rope_partialIfEvPT_iiiiPKS0_S3_, .Lfunc_end6-_Z12rope_partialIfEvPT_iiiiPKS0_S3_
                                        ; -- End function
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.num_vgpr, 21
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.num_agpr, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.numbered_sgpr, 26
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.num_named_barrier, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.private_seg_size, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.uses_vcc, 1
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.uses_flat_scratch, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.has_dyn_sized_stack, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.has_recursion, 0
	.set _Z12rope_partialIfEvPT_iiiiPKS0_S3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 4692
; TotalNumSgprs: 28
; NumVgprs: 21
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 28
; NumVGPRsForWavesPerEU: 21
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z12rope_partialIdEvPT_iiiiPKS0_S3_,"axG",@progbits,_Z12rope_partialIdEvPT_iiiiPKS0_S3_,comdat
	.protected	_Z12rope_partialIdEvPT_iiiiPKS0_S3_ ; -- Begin function _Z12rope_partialIdEvPT_iiiiPKS0_S3_
	.globl	_Z12rope_partialIdEvPT_iiiiPKS0_S3_
	.p2align	8
	.type	_Z12rope_partialIdEvPT_iiiiPKS0_S3_,@function
_Z12rope_partialIdEvPT_iiiiPKS0_S3_:    ; @_Z12rope_partialIdEvPT_iiiiPKS0_S3_
; %bb.0:
	s_clause 0x1
	s_load_b128 s[8:11], s[0:1], 0x8
	s_load_b32 s3, s[0:1], 0x34
	v_mov_b32_e32 v5, 0
	s_delay_alu instid0(VALU_DEP_1)
	v_mov_b32_e32 v1, v5
	s_waitcnt lgkmcnt(0)
	s_lshr_b32 s4, s10, 31
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mad_u64_u32 v[3:4], null, s3, s2, v[0:1]
	s_add_i32 s2, s10, s4
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_ashr_i32 s16, s2, 1
	s_mul_hi_i32 s3, s16, s8
	s_mul_i32 s2, s16, s8
	s_delay_alu instid0(SALU_CYCLE_1)
	v_cmp_gt_i64_e32 vcc_lo, s[2:3], v[3:4]
	s_and_saveexec_b32 s2, vcc_lo
	s_cbranch_execz .LBB7_17
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x18
	s_ashr_i32 s17, s16, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v6, s17, v4
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[5:6]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB7_3
; %bb.2:
	s_ashr_i32 s12, s17, 31
	v_ashrrev_i32_e32 v2, 31, v4
	s_add_u32 s14, s16, s12
	s_mov_b32 s13, s12
	s_addc_u32 s15, s17, s12
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b64 s[14:15], s[14:15], s[12:13]
	v_add_co_u32 v5, vcc_lo, v3, v2
	v_cvt_f32_u32_e32 v0, s14
	v_cvt_f32_u32_e32 v1, s15
	s_sub_u32 s13, 0, s14
	s_subb_u32 s18, 0, s15
	v_add_co_ci_u32_e64 v6, null, v4, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fmamk_f32 v0, v1, 0x4f800000, v0
	v_xor_b32_e32 v9, v5, v2
	v_xor_b32_e32 v10, v6, v2
	v_xor_b32_e32 v2, s12, v2
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_1)
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
	v_readfirstlane_b32 s8, v0
	s_mul_i32 s19, s13, s2
	s_mul_hi_u32 s21, s13, s8
	s_mul_i32 s20, s18, s8
	s_add_i32 s19, s21, s19
	s_mul_i32 s22, s13, s8
	s_add_i32 s19, s19, s20
	s_mul_hi_u32 s21, s8, s22
	s_mul_i32 s24, s8, s19
	s_mul_hi_u32 s23, s2, s22
	s_mul_i32 s20, s2, s22
	s_mul_hi_u32 s22, s8, s19
	s_add_u32 s21, s21, s24
	s_addc_u32 s22, 0, s22
	s_mul_hi_u32 s25, s2, s19
	s_add_u32 s20, s21, s20
	s_mul_i32 s19, s2, s19
	s_addc_u32 s20, s22, s23
	s_addc_u32 s21, s25, 0
	s_add_u32 s19, s20, s19
	s_addc_u32 s20, 0, s21
	s_add_u32 s8, s8, s19
	s_cselect_b32 s19, -1, 0
	s_mul_hi_u32 s21, s13, s8
	s_cmp_lg_u32 s19, 0
	s_mul_i32 s19, s13, s8
	s_addc_u32 s2, s2, s20
	s_mul_i32 s18, s18, s8
	s_mul_i32 s13, s13, s2
	s_mul_hi_u32 s20, s8, s19
	s_add_i32 s13, s21, s13
	s_mul_hi_u32 s21, s2, s19
	s_add_i32 s13, s13, s18
	s_mul_i32 s18, s2, s19
	s_mul_i32 s23, s8, s13
	s_mul_hi_u32 s22, s8, s13
	s_add_u32 s20, s20, s23
	s_addc_u32 s22, 0, s22
	s_mul_hi_u32 s19, s2, s13
	s_add_u32 s18, s20, s18
	s_mul_i32 s13, s2, s13
	s_addc_u32 s18, s22, s21
	s_addc_u32 s19, s19, 0
	s_add_u32 s13, s18, s13
	s_addc_u32 s18, 0, s19
	s_add_u32 s8, s8, s13
	s_cselect_b32 s13, -1, 0
	v_mul_hi_u32 v11, v9, s8
	s_cmp_lg_u32 s13, 0
	v_mad_u64_u32 v[5:6], null, v10, s8, 0
	s_addc_u32 s2, s2, s18
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mad_u64_u32 v[0:1], null, v9, s2, 0
	v_mad_u64_u32 v[7:8], null, v10, s2, 0
	v_add_co_u32 v0, vcc_lo, v11, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v1, null, 0, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, v0, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e32 v0, vcc_lo, v1, v6, vcc_lo
	v_add_co_ci_u32_e32 v1, vcc_lo, 0, v8, vcc_lo
	v_add_co_u32 v5, vcc_lo, v0, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v6, null, 0, v1, vcc_lo
	v_mul_lo_u32 v7, s15, v5
	v_mad_u64_u32 v[0:1], null, s14, v5, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v8, s14, v6
	v_sub_co_u32 v0, vcc_lo, v9, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v1, v1, v8, v7
	v_add_co_u32 v8, s2, v5, 2
	v_add_co_ci_u32_e64 v9, null, 0, v6, s2
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_sub_nc_u32_e32 v7, v10, v1
	v_sub_co_u32 v11, s2, v0, s14
	v_sub_co_ci_u32_e64 v1, null, v10, v1, vcc_lo
	v_subrev_co_ci_u32_e64 v7, null, s15, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s14, v11
	v_subrev_co_ci_u32_e64 v7, null, 0, v7, s2
	v_cndmask_b32_e64 v10, 0, -1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s15, v7
	v_cndmask_b32_e64 v11, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s14, v0
	v_cndmask_b32_e64 v0, 0, -1, vcc_lo
	v_cmp_le_u32_e32 vcc_lo, s15, v1
	v_cndmask_b32_e64 v12, 0, -1, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s15, v7
	v_cndmask_b32_e32 v7, v11, v10, vcc_lo
	v_add_co_u32 v10, vcc_lo, v5, 1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v11, null, 0, v6, vcc_lo
	v_cmp_eq_u32_e32 vcc_lo, s15, v1
	v_cndmask_b32_e32 v0, v12, v0, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 0, v7
	v_cmp_ne_u32_e64 s2, 0, v0
	v_dual_cndmask_b32 v0, v10, v8 :: v_dual_cndmask_b32 v1, v11, v9
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v0, v5, v0, s2
	v_cndmask_b32_e64 v1, v6, v1, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v0, v0, v2
	v_xor_b32_e32 v1, v1, v2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_co_u32 v0, vcc_lo, v0, v2
	v_sub_co_ci_u32_e64 v1, null, v1, v2, vcc_lo
.LBB7_3:
	s_or_saveexec_b32 s8, s3
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[2:3], s[4:5], 0x0
	s_xor_b32 exec_lo, exec_lo, s8
	s_cbranch_execz .LBB7_5
; %bb.4:
	v_cvt_f32_u32_e32 v0, s16
	s_sub_i32 s4, 0, s16
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_iflag_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v0, 0x4f7ffffe, v0
	v_cvt_u32_f32_e32 v0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v1, s4, v0
	v_mul_hi_u32 v1, v0, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_nc_u32_e32 v0, v0, v1
	v_mul_hi_u32 v0, v3, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_lo_u32 v1, v0, s16
	v_add_nc_u32_e32 v2, 1, v0
	v_sub_nc_u32_e32 v1, v3, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_subrev_nc_u32_e32 v5, s16, v1
	v_cmp_le_u32_e32 vcc_lo, s16, v1
	v_dual_cndmask_b32 v1, v1, v5 :: v_dual_cndmask_b32 v0, v0, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_le_u32_e32 vcc_lo, s16, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v2, 1, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e32 v0, v0, v2, vcc_lo
.LBB7_5:
	s_or_b32 exec_lo, exec_lo, s8
	s_abs_i32 s8, s11
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_lo_u32 v7, v1, s16
	v_cvt_f32_u32_e32 v2, s8
	s_sub_i32 s4, 0, s8
	v_mul_lo_u32 v8, v0, s17
	v_sub_nc_u32_e32 v9, 0, v0
	v_ashrrev_i32_e32 v13, 31, v0
	v_rcp_iflag_f32_e32 v2, v2
	s_ashr_i32 s11, s11, 31
	s_cmp_eq_u64 s[6:7], 0
	v_max_i32_e32 v12, v0, v9
	s_waitcnt_depctr 0xfff
	v_mul_f32_e32 v2, 0x4f7ffffe, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cvt_u32_f32_e32 v5, v2
	v_mad_u64_u32 v[1:2], null, v0, s16, 0
	v_mul_lo_u32 v6, s4, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add3_u32 v2, v2, v8, v7
	v_sub_co_u32 v3, vcc_lo, v3, v1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_hi_u32 v6, v5, v6
	v_sub_co_ci_u32_e64 v4, null, v4, v2, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[1:2], 3, v[3:4]
	v_add_nc_u32_e32 v5, v5, v6
	s_delay_alu instid0(VALU_DEP_1)
	v_mad_u64_u32 v[6:7], null, v12, v5, 0
	s_cbranch_scc1 .LBB7_7
; %bb.6:
	v_add_co_u32 v5, vcc_lo, s6, v1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s7, v2, vcc_lo
	global_load_b64 v[5:6], v[5:6], off
	s_branch .LBB7_8
.LBB7_7:
	v_mov_b32_e32 v5, 0
	v_mov_b32_e32 v6, 0x3ff00000
.LBB7_8:
	v_cvt_f64_u32_e32 v[8:9], v4
	v_cvt_f64_u32_e32 v[10:11], v3
	s_waitcnt lgkmcnt(0)
	v_mov_b32_e32 v4, s3
	s_mov_b32 s4, 0x968915a9
	s_mov_b32 s6, 0x4222de17
	s_mov_b32 s5, 0x3fba6564
	s_mov_b32 s7, 0x3fbdee67
	v_xor_b32_e32 v13, s11, v13
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[8:9], v[8:9], 32
	v_add_f64 v[8:9], v[8:9], v[10:11]
	v_cvt_f64_i32_e32 v[10:11], s10
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[8:9], -2.0
	v_div_scale_f64 v[14:15], null, v[10:11], v[10:11], v[8:9]
	v_div_scale_f64 v[20:21], vcc_lo, v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[16:17], v[14:15]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[18:19], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[16:17], v[16:17], v[18:19], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[18:19], v[20:21], v[16:17]
	v_fma_f64 v[14:15], -v[14:15], v[18:19], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[14:15], v[14:15], v[16:17], v[18:19]
	v_cmp_neq_f64_e64 vcc_lo, s[2:3], 1.0
	s_mov_b32 s3, 0x3fe55555
	v_div_fixup_f64 v[8:9], v[14:15], v[10:11], v[8:9]
	v_cndmask_b32_e32 v9, 0x3ff00000, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v8, 0, v8, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, 0, v[8:9]
	v_cndmask_b32_e32 v11, 0x3ff00000, v4, vcc_lo
	v_cndmask_b32_e64 v10, 0, s2, vcc_lo
	s_mov_b32 s2, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_frexp_mant_f64_e64 v[14:15], |v[10:11]|
	v_cmp_gt_f64_e32 vcc_lo, s[2:3], v[14:15]
	v_cndmask_b32_e64 v4, 0, 1, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[14:15], v[14:15], v4
	v_frexp_exp_i32_f64_e32 v4, v[10:11]
	v_add_f64 v[16:17], v[14:15], 1.0
	v_add_f64 v[22:23], v[14:15], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_subrev_co_ci_u32_e64 v4, null, 0, v4, vcc_lo
	v_rcp_f64_e32 v[18:19], v[16:17]
	v_add_f64 v[24:25], v[16:17], -1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[20:21], -v[16:17], v[18:19], 1.0
	v_fma_f64 v[18:19], v[20:21], v[18:19], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], -v[16:17], v[18:19], 1.0
	v_fma_f64 v[18:19], v[20:21], v[18:19], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[20:21], v[22:23], v[18:19]
	v_mul_f64 v[26:27], v[16:17], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[16:17], v[20:21], v[16:17], -v[26:27]
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[26:27], v[14:15]
	v_add_f64 v[24:25], v[22:23], -v[16:17]
	v_add_f64 v[26:27], v[16:17], -v[26:27]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[22:23], -v[24:25]
	v_add_f64 v[14:15], v[26:27], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[22:23], -v[16:17]
	v_add_f64 v[14:15], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[24:25], v[14:15]
	v_mul_f64 v[14:15], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[20:21], v[14:15]
	v_add_f64 v[18:19], v[16:17], -v[20:21]
	v_mul_f64 v[20:21], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	v_fma_f64 v[18:19], v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[14:15], v[14:15]
	v_fma_f64 v[18:19], v[16:17], v[22:23], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[20:21], v[18:19]
	v_fma_f64 v[24:25], v[22:23], s[6:7], s[4:5]
	s_mov_b32 s4, 0x3abe935a
	s_mov_b32 s5, 0x3fbe25e4
	v_add_f64 v[20:21], v[22:23], -v[20:21]
	v_mul_f64 v[30:31], v[16:17], v[22:23]
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s7, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x47e6c9c2
	s_mov_b32 s5, 0x3fc110ef
	v_add_f64 v[18:19], v[18:19], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0xcfa74449
	s_mov_b32 s5, 0x3fc3b13b
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x71bf3c30
	s_mov_b32 s5, 0x3fc745d1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x1c7792ce
	s_mov_b32 s5, 0x3fcc71c7
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x924920da
	s_mov_b32 s5, 0x3fd24924
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s4, 0x9999999c
	s_mov_b32 s5, 0x3fd99999
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[24:25], v[22:23], v[24:25], s[4:5]
	s_mov_b32 s5, 0x3c7abc9e
	s_mov_b32 s4, 0x3b39803f
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[26:27], v[22:23], v[24:25]
	v_fma_f64 v[20:21], v[22:23], v[24:25], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[20:21], v[18:19], v[24:25], v[20:21]
	v_add_f64 v[24:25], v[26:27], v[20:21]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[28:29], v[24:25], s[2:3]
	v_add_f64 v[26:27], v[24:25], -v[26:27]
	s_mov_b32 s3, 0xbfe55555
	s_delay_alu instid0(VALU_DEP_2) | instid1(SALU_CYCLE_1)
	v_add_f64 v[32:33], v[28:29], s[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_fma_f64 v[26:27], v[22:23], v[16:17], -v[30:31]
	s_mov_b32 s2, 0xd5df274d
	s_mov_b32 s3, 0x3c8543b0
	v_add_f64 v[24:25], v[24:25], -v[32:33]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], s[2:3]
	v_fma_f64 v[22:23], v[22:23], v[14:15], v[26:27]
	s_mov_b32 s3, 0x3fe62e42
	s_mov_b32 s2, 0xfefa39ef
	v_ldexp_f64 v[14:15], v[14:15], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], v[24:25]
	v_fma_f64 v[18:19], v[18:19], v[16:17], v[22:23]
	v_ldexp_f64 v[16:17], v[16:17], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[28:29], v[20:21]
	v_add_f64 v[24:25], v[30:31], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[26:27], v[28:29], -v[22:23]
	v_mul_f64 v[28:29], v[24:25], v[22:23]
	v_add_f64 v[30:31], v[24:25], -v[30:31]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[20:21], v[20:21], v[26:27]
	v_fma_f64 v[26:27], v[24:25], v[22:23], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[30:31]
	v_fma_f64 v[20:21], v[24:25], v[20:21], v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[18:19], v[22:23], v[20:21]
	v_cvt_f64_i32_e32 v[22:23], v4
	v_add_f64 v[20:21], v[28:29], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[24:25], v[16:17], v[20:21]
	v_add_f64 v[26:27], v[20:21], -v[28:29]
	v_mul_f64 v[28:29], v[22:23], s[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	v_add_f64 v[18:19], v[18:19], -v[26:27]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[26:27], v[22:23], s[2:3], -v[28:29]
	s_mov_b32 s3, 0xbfe62e42
	v_add_f64 v[16:17], v[20:21], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[14:15], v[14:15], v[18:19]
	v_fma_f64 v[18:19], v[22:23], s[4:5], v[26:27]
	s_mov_b32 s5, 0xbc7abc9e
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], v[16:17]
	v_add_f64 v[16:17], v[28:29], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[24:25], v[14:15]
	v_add_f64 v[28:29], v[16:17], -v[28:29]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[22:23], v[16:17], v[20:21]
	v_add_f64 v[24:25], v[20:21], -v[24:25]
	v_add_f64 v[18:19], v[18:19], -v[28:29]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[26:27], v[22:23], -v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[24:25]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[30:31], v[22:23], -v[26:27]
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_add_f64 v[24:25], v[18:19], v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[16:17], -v[30:31]
	v_add_f64 v[16:17], v[20:21], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[24:25], -v[18:19]
	v_add_f64 v[16:17], v[24:25], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[24:25], v[24:25], -v[20:21]
	v_add_f64 v[14:15], v[14:15], -v[20:21]
	v_add_f64 v[26:27], v[22:23], v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], -v[24:25]
	v_add_f64 v[20:21], v[26:27], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[14:15], v[18:19]
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], v[16:17]
	v_add_f64 v[16:17], v[26:27], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[16:17], -v[26:27]
	v_mul_f64 v[20:21], v[8:9], v[16:17]
	v_add_f64 v[14:15], v[14:15], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[8:9], v[16:17], -v[20:21]
	v_cmp_class_f64_e64 vcc_lo, v[20:21], 0x204
	v_fma_f64 v[14:15], v[8:9], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[16:17], v[20:21], v[14:15]
	v_dual_cndmask_b32 v19, v17, v21 :: v_dual_cndmask_b32 v18, v16, v20
	v_add_f64 v[16:17], v[16:17], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_4)
	v_mul_f64 v[22:23], v[18:19], s[6:7]
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[18:19]|
	v_cmp_lt_f64_e64 s6, |v[10:11]|, 1.0
	v_add_f64 v[14:15], v[14:15], -v[16:17]
	v_trunc_f64_e32 v[16:17], v[8:9]
	v_rndne_f64_e32 v[22:23], v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_dual_cndmask_b32 v15, 0, v15 :: v_dual_cndmask_b32 v14, 0, v14
	v_fma_f64 v[24:25], v[22:23], s[2:3], v[18:19]
	s_mov_b32 s2, 0xfca7ab0c
	s_mov_b32 s3, 0x3e928af3
	v_cvt_i32_f64_e32 v4, v[22:23]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[24:25], v[22:23], s[4:5], v[24:25]
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], s[4:5], s[2:3]
	s_mov_b32 s2, 0x623fde64
	s_mov_b32 s3, 0x3ec71dee
	v_cmp_neq_f64_e64 s5, v[8:9], |v[8:9]|
	v_cmp_eq_f64_e64 s4, 0, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x7c89e6b0
	s_mov_b32 s3, 0x3efa0199
	s_xor_b32 s5, s5, s6
	v_cmp_class_f64_e64 s6, v[10:11], 0x204
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x14761f6e
	s_mov_b32 s3, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x1852b7b0
	s_mov_b32 s3, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x11122322
	s_mov_b32 s3, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x555502a1
	s_mov_b32 s3, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 0x55555511
	s_mov_b32 s3, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	s_mov_b32 s2, 11
	s_mov_b32 s3, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], s[2:3]
	v_cmp_nlt_f64_e64 s2, 0x40900000, v[18:19]
	v_cmp_ngt_f64_e64 s3, 0xc090cc00, v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[26:27], v[24:25], v[26:27], 1.0
	s_and_b32 vcc_lo, s3, s2
	v_fma_f64 v[22:23], v[24:25], v[26:27], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[20:21], v[22:23], v4
	v_mul_f64 v[22:23], v[8:9], 0.5
	v_cndmask_b32_e64 v4, 0x7ff00000, v21, s2
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_trunc_f64_e32 v[18:19], v[22:23]
	v_cndmask_b32_e32 v20, 0, v20, vcc_lo
	v_cmp_eq_f64_e32 vcc_lo, v[16:17], v[8:9]
	v_cndmask_b32_e64 v17, 0x7ff00000, 0, s5
	v_cndmask_b32_e64 v21, 0, v4, s3
	v_cmp_neq_f64_e64 s5, |v[10:11]|, 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_fma_f64 v[14:15], v[20:21], v[14:15], v[20:21]
	v_cmp_class_f64_e64 s3, v[20:21], 0x204
	v_cmp_neq_f64_e64 s2, v[18:19], v[22:23]
	v_mul_lo_u32 v18, v7, s8
                                        ; implicit-def: $vgpr22
	v_cndmask_b32_e64 v17, 0x3ff00000, v17, s5
	v_sub_nc_u32_e32 v12, v12, v18
	v_cndmask_b32_e64 v15, v15, v21, s3
	v_cndmask_b32_e64 v14, v14, v20, s3
	v_cmp_gt_f64_e64 s3, 0, v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_le_u32_e64 s5, s8, v12
	v_cndmask_b32_e32 v16, 0, v14, vcc_lo
	s_and_b32 s2, vcc_lo, s2
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, 0x3ff00000, v11, s2
	v_bfi_b32 v4, 0x7fffffff, v15, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v15, 0x7ff80000, v4, vcc_lo
	v_cmp_gt_f64_e32 vcc_lo, 0, v[10:11]
	s_xor_b32 s3, s3, s4
	v_cndmask_b32_e32 v14, v14, v16, vcc_lo
	v_cndmask_b32_e32 v4, v4, v15, vcc_lo
	v_cmp_class_f64_e64 vcc_lo, v[8:9], 0x204
	v_add_nc_u32_e32 v15, 1, v7
	v_subrev_nc_u32_e32 v16, s8, v12
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, v15, s5
	v_cndmask_b32_e64 v12, v12, v16, s5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_nc_u32_e32 v15, 1, v7
	v_cmp_le_u32_e64 s5, s8, v12
	v_cndmask_b32_e64 v12, 0x7ff00000, 0, s3
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v7, v7, v15, s5
	v_cndmask_b32_e64 v15, 0, v11, s2
	s_or_b32 s2, s4, s6
	v_xor_b32_e32 v7, v7, v13
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_bfi_b32 v12, 0x7fffffff, v12, v15
	v_sub_nc_u32_e32 v7, v7, v13
	v_cndmask_b32_e32 v4, v4, v17, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v4, v4, v12, s2
	s_or_b32 s2, s2, vcc_lo
	v_cmp_o_f64_e32 vcc_lo, v[10:11], v[8:9]
	v_cvt_f64_i32_e32 v[7:8], v7
	v_cndmask_b32_e64 v12, v14, 0, s2
	v_cndmask_b32_e32 v9, 0, v12, vcc_lo
	v_cndmask_b32_e32 v10, 0x7ff80000, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_mul_f64 v[7:8], v[9:10], v[7:8]
	s_waitcnt vmcnt(0)
	v_div_scale_f64 v[9:10], null, v[5:6], v[5:6], v[7:8]
	v_div_scale_f64 v[15:16], vcc_lo, v[7:8], v[5:6], v[7:8]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[11:12], v[9:10]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[11:12], v[13:14], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[13:14], -v[9:10], v[11:12], 1.0
	v_fma_f64 v[11:12], v[11:12], v[13:14], v[11:12]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[13:14], v[15:16], v[11:12]
	v_fma_f64 v[9:10], -v[9:10], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[9:10], v[9:10], v[11:12], v[13:14]
	v_div_fixup_f64 v[4:5], v[9:10], v[5:6], v[7:8]
                                        ; implicit-def: $vgpr6_vgpr7
                                        ; implicit-def: $vgpr8_vgpr9
	s_delay_alu instid0(VALU_DEP_1)
	v_cmp_ngt_f64_e64 s2, 0x41d00000, |v[4:5]|
	v_trig_preop_f64 v[18:19], |v[4:5]|, 0
	v_trig_preop_f64 v[16:17], |v[4:5]|, 1
	v_ldexp_f64 v[20:21], |v[4:5]|, 0xffffff80
	v_trig_preop_f64 v[10:11], |v[4:5]|, 2
	v_and_b32_e32 v24, 0x7fffffff, v5
	s_and_saveexec_b32 s3, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s3, exec_lo, s3
	s_cbranch_execz .LBB7_10
; %bb.9:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[4:5]|
	v_mov_b32_e32 v35, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_cndmask_b32_e32 v7, v24, v21, vcc_lo
	v_cndmask_b32_e32 v6, v4, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[18:19], v[6:7]
	v_mul_f64 v[12:13], v[16:17], v[6:7]
	v_fma_f64 v[14:15], v[18:19], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[33:34], v[16:17], v[6:7], -v[12:13]
	v_add_f64 v[22:23], v[12:13], v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[22:23], -v[12:13]
	v_add_f64 v[29:30], v[8:9], v[22:23]
	v_add_f64 v[27:28], v[22:23], -v[25:26]
	v_add_f64 v[14:15], v[14:15], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[25:26], v[29:30], -2
	v_add_f64 v[8:9], v[29:30], -v[8:9]
	v_add_f64 v[12:13], v[12:13], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[25:26]|
	v_add_f64 v[8:9], v[22:23], -v[8:9]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[14:15], v[12:13]
	v_fract_f64_e32 v[14:15], v[25:26]
	v_ldexp_f64 v[14:15], v[14:15], 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v15, 0, v15 :: v_dual_cndmask_b32 v14, 0, v14
	v_mul_f64 v[31:32], v[10:11], v[6:7]
	v_add_f64 v[27:28], v[31:32], v[33:34]
	v_fma_f64 v[6:7], v[10:11], v[6:7], -v[31:32]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[22:23], v[27:28], v[12:13]
	v_add_f64 v[25:26], v[8:9], v[22:23]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[29:30], v[25:26], v[14:15]
	v_add_f64 v[8:9], v[25:26], -v[8:9]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[29:30]
	v_add_f64 v[29:30], v[27:28], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[22:23], -v[8:9]
	v_cndmask_b32_e64 v36, 0, 0x40100000, vcc_lo
	v_add_f64 v[40:41], v[27:28], -v[29:30]
	v_add_f64 v[29:30], v[33:34], -v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[14:15], v[14:15], v[35:36]
	v_add_f64 v[36:37], v[22:23], -v[27:28]
	v_add_f64 v[33:34], v[31:32], -v[40:41]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[38:39], v[25:26], v[14:15]
	v_add_f64 v[42:43], v[22:23], -v[36:37]
	v_add_f64 v[12:13], v[12:13], -v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[29:30], v[29:30], v[33:34]
	v_cvt_i32_f64_e32 v38, v[38:39]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[27:28], v[27:28], -v[42:43]
	v_cvt_f64_i32_e32 v[36:37], v38
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[27:28]
	v_add_f64 v[14:15], v[14:15], -v[36:37]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[29:30], v[12:13]
	v_add_f64 v[27:28], v[25:26], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[27:28], -v[14:15]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[25:26], -v[12:13]
	v_cndmask_b32_e64 v36, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v22, null, 0, v38, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_add_f64 v[8:9], v[27:28], -v[35:36]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[8:9], v[6:7]
	v_mul_f64 v[14:15], v[12:13], s[4:5]
	v_add_f64 v[8:9], v[12:13], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[25:26], v[12:13], s[4:5], -v[14:15]
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[12:13], s[6:7], v[25:26]
	v_fma_f64 v[8:9], v[6:7], s[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[14:15], v[8:9]
	v_add_f64 v[12:13], v[6:7], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[8:9], v[8:9], -v[12:13]
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB7_12
	s_branch .LBB7_11
.LBB7_10:
	s_and_not1_saveexec_b32 s3, s3
	s_cbranch_execz .LBB7_12
.LBB7_11:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[6:7], |v[4:5]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[12:13], v[6:7]
	v_fma_f64 v[6:7], v[12:13], s[4:5], |v[4:5]|
	v_mul_f64 v[8:9], v[12:13], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[22:23], v[12:13], s[6:7], v[6:7]
	v_add_f64 v[14:15], v[6:7], v[8:9]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[14:15]
	v_add_f64 v[14:15], v[14:15], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[8:9]
	v_fma_f64 v[8:9], v[12:13], s[6:7], v[8:9]
	v_add_f64 v[6:7], v[14:15], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_fma_f64 v[8:9], v[12:13], s[4:5], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[22:23], v[8:9]
	v_add_f64 v[14:15], v[6:7], -v[22:23]
	v_cvt_i32_f64_e32 v22, v[12:13]
	s_delay_alu instid0(VALU_DEP_2)
	v_add_f64 v[8:9], v[8:9], -v[14:15]
.LBB7_12:
	s_or_b32 exec_lo, exec_lo, s3
                                        ; implicit-def: $vgpr23
                                        ; implicit-def: $vgpr12_vgpr13
                                        ; implicit-def: $vgpr14_vgpr15
	s_and_saveexec_b32 s3, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_xor_b32 s2, exec_lo, s3
	s_cbranch_execz .LBB7_14
; %bb.13:
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[4:5]|
	v_mov_b32_e32 v33, 0
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_cndmask_b32_e32 v13, v24, v21, vcc_lo
	v_cndmask_b32_e32 v12, v4, v20, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[14:15], v[18:19], v[12:13]
	v_mul_f64 v[20:21], v[16:17], v[12:13]
	v_fma_f64 v[18:19], v[18:19], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[16:17], v[16:17], v[12:13], -v[20:21]
	v_add_f64 v[23:24], v[20:21], v[18:19]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[25:26], v[23:24], -v[20:21]
	v_add_f64 v[29:30], v[14:15], v[23:24]
	v_add_f64 v[27:28], v[23:24], -v[25:26]
	v_add_f64 v[18:19], v[18:19], -v[25:26]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[25:26], v[29:30], -2
	v_add_f64 v[14:15], v[29:30], -v[14:15]
	v_add_f64 v[20:21], v[20:21], -v[27:28]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[25:26]|
	v_add_f64 v[14:15], v[23:24], -v[14:15]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[18:19], v[18:19], v[20:21]
	v_fract_f64_e32 v[20:21], v[25:26]
	v_ldexp_f64 v[20:21], v[20:21], 2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v21, 0, v21 :: v_dual_cndmask_b32 v20, 0, v20
	v_mul_f64 v[31:32], v[10:11], v[12:13]
	v_add_f64 v[27:28], v[31:32], v[16:17]
	v_fma_f64 v[10:11], v[10:11], v[12:13], -v[31:32]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[23:24], v[27:28], v[18:19]
	v_add_f64 v[25:26], v[14:15], v[23:24]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[29:30], v[25:26], v[20:21]
	v_add_f64 v[12:13], v[25:26], -v[14:15]
	v_cmp_gt_f64_e32 vcc_lo, 0, v[29:30]
	v_add_f64 v[29:30], v[27:28], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[23:24], -v[12:13]
	v_cndmask_b32_e64 v34, 0, 0x40100000, vcc_lo
	v_add_f64 v[38:39], v[27:28], -v[29:30]
	v_add_f64 v[16:17], v[16:17], -v[29:30]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[20:21], v[20:21], v[33:34]
	v_add_f64 v[34:35], v[23:24], -v[27:28]
	v_add_f64 v[29:30], v[31:32], -v[38:39]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[36:37], v[25:26], v[20:21]
	v_add_f64 v[40:41], v[23:24], -v[34:35]
	v_add_f64 v[18:19], v[18:19], -v[34:35]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[16:17], v[16:17], v[29:30]
	v_cvt_i32_f64_e32 v36, v[36:37]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[27:28], v[27:28], -v[40:41]
	v_cvt_f64_i32_e32 v[34:35], v36
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[18:19], v[18:19], v[27:28]
	v_add_f64 v[20:21], v[20:21], -v[34:35]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[16:17], v[18:19]
	v_add_f64 v[16:17], v[25:26], v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], v[14:15]
	v_add_f64 v[14:15], v[16:17], -v[20:21]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[12:13], v[10:11]
	v_add_f64 v[12:13], v[25:26], -v[14:15]
	v_cndmask_b32_e64 v34, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v23, null, 0, v36, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[10:11], v[12:13]
	v_add_f64 v[12:13], v[16:17], -v[33:34]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_mul_f64 v[16:17], v[14:15], s[4:5]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[18:19], v[14:15], s[4:5], -v[16:17]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[14:15], s[6:7], v[18:19]
	v_fma_f64 v[10:11], v[10:11], s[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], v[10:11]
	v_add_f64 v[14:15], v[12:13], -v[16:17]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[14:15], v[10:11], -v[14:15]
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execnz .LBB7_15
	s_branch .LBB7_16
.LBB7_14:
	s_and_not1_saveexec_b32 s2, s2
	s_cbranch_execz .LBB7_16
.LBB7_15:
	s_mov_b32 s4, 0x6dc9c883
	s_mov_b32 s5, 0x3fe45f30
	s_mov_b32 s7, 0xbc91a626
	v_mul_f64 v[10:11], |v[4:5]|, s[4:5]
	s_mov_b32 s4, 0x54442d18
	s_mov_b32 s5, 0xbff921fb
	s_mov_b32 s6, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[10:11], v[10:11]
	v_fma_f64 v[12:13], v[10:11], s[4:5], |v[4:5]|
	v_mul_f64 v[14:15], v[10:11], s[6:7]
	s_mov_b32 s4, 0x252049c0
	s_mov_b32 s5, 0xb97b839a
	v_cvt_i32_f64_e32 v23, v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[18:19], v[10:11], s[6:7], v[12:13]
	v_add_f64 v[16:17], v[12:13], v[14:15]
	s_mov_b32 s7, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[12:13], -v[16:17]
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[12:13], v[12:13], v[14:15]
	v_fma_f64 v[14:15], v[10:11], s[6:7], v[14:15]
	v_add_f64 v[12:13], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[10:11], s[4:5], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[18:19], v[14:15]
	v_add_f64 v[16:17], v[12:13], -v[18:19]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[14:15], v[14:15], -v[16:17]
.LBB7_16:
	s_or_b32 exec_lo, exec_lo, s2
	v_mul_f64 v[10:11], v[6:7], v[6:7]
	v_mul_f64 v[16:17], v[12:13], v[12:13]
	s_load_b64 s[0:1], s[0:1], 0x0
	v_mad_i64_i32 v[18:19], null, v0, s9, 0
	v_add_nc_u32_e32 v24, s16, v3
	s_mov_b32 s2, 0xb42fdfa7
	s_mov_b32 s4, 0xf9a43bb8
	s_mov_b32 s3, 0xbe5ae600
	s_mov_b32 s5, 0x3de5e0b2
	v_ashrrev_i32_e32 v25, 31, v24
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[18:19], 3, v[18:19]
	v_mul_f64 v[40:41], v[8:9], 0.5
	v_mul_f64 v[46:47], v[14:15], 0.5
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v3, vcc_lo, s0, v18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v26, null, s1, v19, vcc_lo
	v_lshlrev_b64 v[18:19], 3, v[24:25]
	v_add_co_u32 v0, vcc_lo, v3, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v26, v2, vcc_lo
	s_mov_b32 s0, 0x9037ab78
	v_add_co_u32 v2, vcc_lo, v3, v18
	v_add_co_ci_u32_e64 v3, null, v26, v19, vcc_lo
	s_clause 0x1
	global_load_b64 v[18:19], v[0:1], off
	global_load_b64 v[24:25], v[2:3], off
	v_fma_f64 v[20:21], v[10:11], s[4:5], s[2:3]
	v_fma_f64 v[26:27], v[16:17], s[4:5], s[2:3]
	s_mov_b32 s2, 0x46cc5e42
	s_mov_b32 s4, 0x796cde01
	s_mov_b32 s1, 0x3e21eeb6
	s_mov_b32 s3, 0xbda907db
	s_mov_b32 s5, 0x3ec71de3
	v_fma_f64 v[28:29], v[10:11], s[2:3], s[0:1]
	v_mul_f64 v[30:31], v[10:11], 0.5
	v_fma_f64 v[32:33], v[16:17], s[2:3], s[0:1]
	v_mul_f64 v[34:35], v[16:17], 0.5
	s_mov_b32 s0, 0xa17f65f6
	s_mov_b32 s2, 0x19e83e5c
	s_mov_b32 s1, 0xbe927e4f
	s_mov_b32 s3, 0xbf2a01a0
	v_mul_f64 v[42:43], v[6:7], -v[10:11]
	v_mul_f64 v[48:49], v[12:13], -v[16:17]
	v_fma_f64 v[20:21], v[10:11], v[20:21], s[4:5]
	v_fma_f64 v[26:27], v[16:17], v[26:27], s[4:5]
	v_fma_f64 v[28:29], v[10:11], v[28:29], s[0:1]
	v_add_f64 v[36:37], -v[30:31], 1.0
	v_fma_f64 v[32:33], v[16:17], v[32:33], s[0:1]
	v_add_f64 v[38:39], -v[34:35], 1.0
	s_mov_b32 s0, 0x19f4ec90
	s_mov_b32 s1, 0x3efa01a0
	v_fma_f64 v[20:21], v[10:11], v[20:21], s[2:3]
	v_fma_f64 v[26:27], v[16:17], v[26:27], s[2:3]
	s_mov_b32 s2, 0x11110bb3
	s_mov_b32 s3, 0x3f811111
	v_fma_f64 v[28:29], v[10:11], v[28:29], s[0:1]
	v_add_f64 v[44:45], -v[36:37], 1.0
	v_fma_f64 v[32:33], v[16:17], v[32:33], s[0:1]
	v_add_f64 v[50:51], -v[38:39], 1.0
	s_mov_b32 s0, 0x16c16967
	s_mov_b32 s1, 0xbf56c16c
	v_fma_f64 v[20:21], v[10:11], v[20:21], s[2:3]
	v_fma_f64 v[26:27], v[16:17], v[26:27], s[2:3]
	v_fma_f64 v[28:29], v[10:11], v[28:29], s[0:1]
	v_add_f64 v[30:31], v[44:45], -v[30:31]
	v_fma_f64 v[32:33], v[16:17], v[32:33], s[0:1]
	v_add_f64 v[34:35], v[50:51], -v[34:35]
	s_mov_b32 s0, 0x55555555
	s_mov_b32 s1, 0x3fa55555
	v_fma_f64 v[20:21], v[42:43], v[20:21], v[40:41]
	v_fma_f64 v[26:27], v[48:49], v[26:27], v[46:47]
	v_mul_f64 v[40:41], v[10:11], v[10:11]
	v_fma_f64 v[28:29], v[10:11], v[28:29], s[0:1]
	v_fma_f64 v[30:31], v[6:7], -v[8:9], v[30:31]
	v_fma_f64 v[8:9], v[10:11], v[20:21], -v[8:9]
	v_mul_f64 v[10:11], v[16:17], v[16:17]
	v_fma_f64 v[20:21], v[16:17], v[32:33], s[0:1]
	v_fma_f64 v[32:33], v[12:13], -v[14:15], v[34:35]
	v_fma_f64 v[14:15], v[16:17], v[26:27], -v[14:15]
	s_mov_b32 s1, 0xbfc55555
	v_fma_f64 v[16:17], v[40:41], v[28:29], v[30:31]
	v_fma_f64 v[8:9], v[42:43], s[0:1], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[10:11], v[10:11], v[20:21], v[32:33]
	v_fma_f64 v[14:15], v[48:49], s[0:1], v[14:15]
	v_cmp_class_f64_e64 s0, v[4:5], 0x1f8
	v_lshlrev_b32_e32 v4, 30, v23
	v_add_f64 v[16:17], v[36:37], v[16:17]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v4, v4, v5
	v_and_b32_e32 v4, 0x80000000, v4
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[8:9], v[38:39], v[10:11]
	v_add_f64 v[10:11], v[12:13], -v[14:15]
	v_and_b32_e32 v12, 1, v22
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v12
	v_and_b32_e32 v12, 1, v23
	v_cmp_eq_u32_e64 s1, 0, v12
	v_cndmask_b32_e32 v6, v6, v16, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e64 v5, v8, v10, s1
	v_cndmask_b32_e64 v8, v9, v11, s1
	v_cndmask_b32_e64 v5, 0, v5, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v8, v8, v4
	v_cndmask_b32_e64 v4, 0, v6, s0
	v_cndmask_b32_e64 v6, 0x7ff80000, v8, s0
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_mul_f64 v[8:9], v[24:25], v[5:6]
	v_mul_f64 v[10:11], v[18:19], v[5:6]
	v_lshlrev_b32_e32 v5, 30, v22
	v_xor_b32_e32 v6, 0x80000000, v7
	v_and_b32_e32 v5, 0x80000000, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, v6, v17, vcc_lo
	v_xor_b32_e32 v5, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0x7ff80000, v5, s0
	v_fma_f64 v[6:7], v[18:19], v[4:5], -v[8:9]
	v_fma_f64 v[4:5], v[24:25], v[4:5], v[10:11]
	s_clause 0x1
	global_store_b64 v[0:1], v[6:7], off
	global_store_b64 v[2:3], v[4:5], off
.LBB7_17:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z12rope_partialIdEvPT_iiiiPKS0_S3_
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
		.amdhsa_next_free_vgpr 52
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
		.amdhsa_inst_pref_size 52
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z12rope_partialIdEvPT_iiiiPKS0_S3_,"axG",@progbits,_Z12rope_partialIdEvPT_iiiiPKS0_S3_,comdat
.Lfunc_end7:
	.size	_Z12rope_partialIdEvPT_iiiiPKS0_S3_, .Lfunc_end7-_Z12rope_partialIdEvPT_iiiiPKS0_S3_
                                        ; -- End function
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.num_vgpr, 52
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.num_agpr, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.numbered_sgpr, 26
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.num_named_barrier, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.private_seg_size, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.uses_vcc, 1
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.uses_flat_scratch, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.has_dyn_sized_stack, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.has_recursion, 0
	.set _Z12rope_partialIdEvPT_iiiiPKS0_S3_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 6576
; TotalNumSgprs: 28
; NumVgprs: 52
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 6
; NumSGPRsForWavesPerEU: 28
; NumVGPRsForWavesPerEU: 52
; Occupancy: 16
; WaveLimiterHint : 0
; COMPUTE_PGM_RSRC2:SCRATCH_EN: 0
; COMPUTE_PGM_RSRC2:USER_SGPR: 2
; COMPUTE_PGM_RSRC2:TRAP_HANDLER: 0
; COMPUTE_PGM_RSRC2:TGID_X_EN: 1
; COMPUTE_PGM_RSRC2:TGID_Y_EN: 0
; COMPUTE_PGM_RSRC2:TGID_Z_EN: 0
; COMPUTE_PGM_RSRC2:TIDIG_COMP_CNT: 0
	.section	.text._Z8gelu_mulIfEvPKT_S2_PS0_l,"axG",@progbits,_Z8gelu_mulIfEvPKT_S2_PS0_l,comdat
	.protected	_Z8gelu_mulIfEvPKT_S2_PS0_l ; -- Begin function _Z8gelu_mulIfEvPKT_S2_PS0_l
	.globl	_Z8gelu_mulIfEvPKT_S2_PS0_l
	.p2align	8
	.type	_Z8gelu_mulIfEvPKT_S2_PS0_l,@function
_Z8gelu_mulIfEvPKT_S2_PS0_l:            ; @_Z8gelu_mulIfEvPKT_S2_PS0_l
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
	s_cbranch_execz .LBB8_2
; %bb.1:
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_mov_b32 s0, 0x6d4801f7
	s_mov_b32 s1, 0x3fa6e4e2
	s_mov_b32 s2, 0x6a5dcb37
	s_mov_b32 s3, 0x3e5ade15
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v2, vcc_lo, s4, v0
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b32 v2, v[2:3], off
	s_waitcnt vmcnt(0)
	v_cvt_f64_f32_e32 v[2:3], v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s0, 0x33d43651
	s_mov_b32 s1, 0x3fe98845
	v_mul_f64 v[4:5], v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[4:5], v[4:5], v[2:3], v[2:3]
	v_mul_f64 v[2:3], v[2:3], 0.5
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], -v[14:15]
	v_add_co_u32 v14, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v15, null, s7, v1, vcc_lo
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[4:5]|
	global_load_b32 v14, v[14:15], off
	v_add_f64 v[6:7], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_and_b32_e32 v8, 0x7fffffff, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_cndmask_b32_e32 v7, 0x3ff00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[4:5]|
	v_cndmask_b32_e32 v4, v6, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, v7, v8, vcc_lo
	v_add_co_u32 v0, vcc_lo, s8, v0
	v_add_co_ci_u32_e64 v1, null, s9, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_bfi_b32 v5, 0x7fffffff, v6, v5
	v_add_f64 v[4:5], v[4:5], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	v_cvt_f32_f64_e32 v2, v[2:3]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f32_e32 v2, v14, v2
	global_store_b32 v[0:1], v2, off
.LBB8_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8gelu_mulIfEvPKT_S2_PS0_l
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
	.section	.text._Z8gelu_mulIfEvPKT_S2_PS0_l,"axG",@progbits,_Z8gelu_mulIfEvPKT_S2_PS0_l,comdat
.Lfunc_end8:
	.size	_Z8gelu_mulIfEvPKT_S2_PS0_l, .Lfunc_end8-_Z8gelu_mulIfEvPKT_S2_PS0_l
                                        ; -- End function
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.num_vgpr, 28
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.num_agpr, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.numbered_sgpr, 12
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.num_named_barrier, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.private_seg_size, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.uses_vcc, 1
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.uses_flat_scratch, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.has_dyn_sized_stack, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.has_recursion, 0
	.set _Z8gelu_mulIfEvPKT_S2_PS0_l.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1900
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
	.section	.text._Z8gelu_mulIdEvPKT_S2_PS0_l,"axG",@progbits,_Z8gelu_mulIdEvPKT_S2_PS0_l,comdat
	.protected	_Z8gelu_mulIdEvPKT_S2_PS0_l ; -- Begin function _Z8gelu_mulIdEvPKT_S2_PS0_l
	.globl	_Z8gelu_mulIdEvPKT_S2_PS0_l
	.p2align	8
	.type	_Z8gelu_mulIdEvPKT_S2_PS0_l,@function
_Z8gelu_mulIdEvPKT_S2_PS0_l:            ; @_Z8gelu_mulIdEvPKT_S2_PS0_l
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
	s_cbranch_execz .LBB9_2
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
.LBB9_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8gelu_mulIdEvPKT_S2_PS0_l
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
	.section	.text._Z8gelu_mulIdEvPKT_S2_PS0_l,"axG",@progbits,_Z8gelu_mulIdEvPKT_S2_PS0_l,comdat
.Lfunc_end9:
	.size	_Z8gelu_mulIdEvPKT_S2_PS0_l, .Lfunc_end9-_Z8gelu_mulIdEvPKT_S2_PS0_l
                                        ; -- End function
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.num_vgpr, 28
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.num_agpr, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.numbered_sgpr, 12
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.num_named_barrier, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.private_seg_size, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.uses_vcc, 1
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.uses_flat_scratch, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.has_dyn_sized_stack, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.has_recursion, 0
	.set _Z8gelu_mulIdEvPKT_S2_PS0_l.has_indirect_call, 0
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
	.section	.text._Z8glu_geluIfEvPKT_PS0_ii,"axG",@progbits,_Z8glu_geluIfEvPKT_PS0_ii,comdat
	.protected	_Z8glu_geluIfEvPKT_PS0_ii ; -- Begin function _Z8glu_geluIfEvPKT_PS0_ii
	.globl	_Z8glu_geluIfEvPKT_PS0_ii
	.p2align	8
	.type	_Z8glu_geluIfEvPKT_PS0_ii,@function
_Z8glu_geluIfEvPKT_PS0_ii:              ; @_Z8glu_geluIfEvPKT_PS0_ii
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
	s_cbranch_execz .LBB10_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB10_3
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
.LBB10_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB10_5
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
.LBB10_5:
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v7, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 2
	v_add_co_u32 v0, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v7, v5, vcc_lo
	s_mov_b32 s4, 0x6a5dcb37
	s_mov_b32 s5, 0x3e5ade15
	global_load_b32 v8, v[0:1], off
	v_add_co_u32 v0, vcc_lo, v6, s0
	v_add_co_ci_u32_e64 v1, null, s1, v7, vcc_lo
	s_mov_b32 s0, 0x6d4801f7
	v_add_co_u32 v0, vcc_lo, v0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v1, v5, vcc_lo
	s_mov_b32 s1, 0x3fa6e4e2
	global_load_b32 v28, v[0:1], off
	s_waitcnt vmcnt(1)
	v_cvt_f64_f32_e32 v[0:1], v8
	v_mul_f64 v[4:5], v[0:1], s[0:1]
	s_mov_b32 s0, 0x33d43651
	s_mov_b32 s1, 0x3fe98845
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[4:5], v[4:5], v[0:1]
	v_fma_f64 v[4:5], v[4:5], v[0:1], v[0:1]
	v_mul_f64 v[0:1], v[0:1], 0.5
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[4:5], v[4:5], s[0:1]
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0x3ff71547
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[6:7], |v[4:5]|, s[0:1]
	s_mov_b32 s0, 0xfefa3000
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_nlt_f64_e64 vcc_lo, 0x40331000, |v[4:5]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
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
	v_fma_f64 v[12:13], v[10:11], s[4:5], s[0:1]
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
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[16:17], v[24:25], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[12:13], v[16:17], -v[14:15]
	v_add_f64 v[6:7], v[6:7], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[18:19], v[6:7]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	v_and_b32_e32 v8, 0x7fffffff, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[6:7]
	v_cndmask_b32_e32 v7, 0x3ff00000, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_gt_f64_e64 vcc_lo, 0x3e400000, |v[4:5]|
	v_cndmask_b32_e32 v4, v6, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v6, v7, v8, vcc_lo
	v_bfi_b32 v5, 0x7fffffff, v6, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], 1.0
	v_mul_f64 v[0:1], v[0:1], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cvt_f32_f64_e32 v4, v[0:1]
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_mul_f32_e32 v2, v28, v4
	global_store_b32 v[0:1], v2, off
.LBB10_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8glu_geluIfEvPKT_PS0_ii
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
	.section	.text._Z8glu_geluIfEvPKT_PS0_ii,"axG",@progbits,_Z8glu_geluIfEvPKT_PS0_ii,comdat
.Lfunc_end10:
	.size	_Z8glu_geluIfEvPKT_PS0_ii, .Lfunc_end10-_Z8glu_geluIfEvPKT_PS0_ii
                                        ; -- End function
	.set _Z8glu_geluIfEvPKT_PS0_ii.num_vgpr, 29
	.set _Z8glu_geluIfEvPKT_PS0_ii.num_agpr, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.numbered_sgpr, 19
	.set _Z8glu_geluIfEvPKT_PS0_ii.num_named_barrier, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.private_seg_size, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.uses_vcc, 1
	.set _Z8glu_geluIfEvPKT_PS0_ii.uses_flat_scratch, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.has_dyn_sized_stack, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.has_recursion, 0
	.set _Z8glu_geluIfEvPKT_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 2984
; TotalNumSgprs: 21
; NumVgprs: 29
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 21
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
	.section	.text._Z8glu_geluIdEvPKT_PS0_ii,"axG",@progbits,_Z8glu_geluIdEvPKT_PS0_ii,comdat
	.protected	_Z8glu_geluIdEvPKT_PS0_ii ; -- Begin function _Z8glu_geluIdEvPKT_PS0_ii
	.globl	_Z8glu_geluIdEvPKT_PS0_ii
	.p2align	8
	.type	_Z8glu_geluIdEvPKT_PS0_ii,@function
_Z8glu_geluIdEvPKT_PS0_ii:              ; @_Z8glu_geluIdEvPKT_PS0_ii
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
	s_cbranch_execz .LBB11_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB11_3
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
.LBB11_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB11_5
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
.LBB11_5:
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
.LBB11_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8glu_geluIdEvPKT_PS0_ii
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
	.section	.text._Z8glu_geluIdEvPKT_PS0_ii,"axG",@progbits,_Z8glu_geluIdEvPKT_PS0_ii,comdat
.Lfunc_end11:
	.size	_Z8glu_geluIdEvPKT_PS0_ii, .Lfunc_end11-_Z8glu_geluIdEvPKT_PS0_ii
                                        ; -- End function
	.set _Z8glu_geluIdEvPKT_PS0_ii.num_vgpr, 30
	.set _Z8glu_geluIdEvPKT_PS0_ii.num_agpr, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.numbered_sgpr, 19
	.set _Z8glu_geluIdEvPKT_PS0_ii.num_named_barrier, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.private_seg_size, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.uses_vcc, 1
	.set _Z8glu_geluIdEvPKT_PS0_ii.uses_flat_scratch, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.has_dyn_sized_stack, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.has_recursion, 0
	.set _Z8glu_geluIdEvPKT_PS0_ii.has_indirect_call, 0
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
	.section	.text._Z8glu_siluIfEvPKT_PS0_ii,"axG",@progbits,_Z8glu_siluIfEvPKT_PS0_ii,comdat
	.protected	_Z8glu_siluIfEvPKT_PS0_ii ; -- Begin function _Z8glu_siluIfEvPKT_PS0_ii
	.globl	_Z8glu_siluIfEvPKT_PS0_ii
	.p2align	8
	.type	_Z8glu_siluIfEvPKT_PS0_ii,@function
_Z8glu_siluIfEvPKT_PS0_ii:              ; @_Z8glu_siluIfEvPKT_PS0_ii
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
	s_cbranch_execz .LBB12_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB12_3
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
.LBB12_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB12_5
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
.LBB12_5:
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
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[0:1], 2, v[0:1]
	v_lshlrev_b64 v[4:5], 2, v[4:5]
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, s0, v0
	v_add_co_ci_u32_e64 v7, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 2
	v_add_co_u32 v0, vcc_lo, v6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, v7, v5, vcc_lo
	global_load_b32 v8, v[0:1], off
	v_add_co_u32 v0, vcc_lo, v6, s0
	v_add_co_ci_u32_e64 v1, null, s1, v7, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, v0, v4
	v_add_co_ci_u32_e64 v1, null, v1, v5, vcc_lo
	global_load_b32 v14, v[0:1], off
	s_waitcnt vmcnt(1)
	v_mul_f32_e32 v0, 0xbfb8aa3b, v8
	v_cmp_nlt_f32_e32 vcc_lo, 0x42ce8ed0, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f32 v1, 0xbfb8aa3b, v8, -v0
	v_rndne_f32_e32 v4, v0
	v_fmamk_f32 v1, v8, 0xb2a5705f, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_sub_f32_e32 v0, v0, v4
	v_add_f32_e32 v0, v0, v1
	v_cvt_i32_f32_e32 v1, v4
	v_cvt_f64_f32_e32 v[4:5], v8
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_exp_f32_e32 v0, v0
	s_waitcnt_depctr 0xfff
	v_ldexp_f32 v0, v0, v1
	v_cndmask_b32_e32 v0, 0, v0, vcc_lo
	v_cmp_ngt_f32_e32 vcc_lo, 0xc2b17218, v8
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v0, 0x7f800000, v0, vcc_lo
	v_cvt_f64_f32_e32 v[0:1], v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[0:1], v[0:1], 1.0
	v_div_scale_f64 v[6:7], null, v[0:1], v[0:1], v[4:5]
	v_div_scale_f64 v[12:13], vcc_lo, v[4:5], v[0:1], v[4:5]
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
	v_div_fixup_f64 v[0:1], v[6:7], v[0:1], v[4:5]
	s_waitcnt vmcnt(0)
	v_cvt_f64_f32_e32 v[4:5], v14
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[0:1], v[0:1], v[4:5]
	v_cvt_f32_f64_e32 v4, v[0:1]
	v_lshlrev_b64 v[0:1], 2, v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b32 v[0:1], v4, off
.LBB12_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8glu_siluIfEvPKT_PS0_ii
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
		.amdhsa_next_free_vgpr 15
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
		.amdhsa_inst_pref_size 12
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z8glu_siluIfEvPKT_PS0_ii,"axG",@progbits,_Z8glu_siluIfEvPKT_PS0_ii,comdat
.Lfunc_end12:
	.size	_Z8glu_siluIfEvPKT_PS0_ii, .Lfunc_end12-_Z8glu_siluIfEvPKT_PS0_ii
                                        ; -- End function
	.set _Z8glu_siluIfEvPKT_PS0_ii.num_vgpr, 15
	.set _Z8glu_siluIfEvPKT_PS0_ii.num_agpr, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.numbered_sgpr, 19
	.set _Z8glu_siluIfEvPKT_PS0_ii.num_named_barrier, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.private_seg_size, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.uses_vcc, 1
	.set _Z8glu_siluIfEvPKT_PS0_ii.uses_flat_scratch, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.has_dyn_sized_stack, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.has_recursion, 0
	.set _Z8glu_siluIfEvPKT_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1496
; TotalNumSgprs: 21
; NumVgprs: 15
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 21
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
	.section	.text._Z8glu_siluIdEvPKT_PS0_ii,"axG",@progbits,_Z8glu_siluIdEvPKT_PS0_ii,comdat
	.protected	_Z8glu_siluIdEvPKT_PS0_ii ; -- Begin function _Z8glu_siluIdEvPKT_PS0_ii
	.globl	_Z8glu_siluIdEvPKT_PS0_ii
	.p2align	8
	.type	_Z8glu_siluIdEvPKT_PS0_ii,@function
_Z8glu_siluIdEvPKT_PS0_ii:              ; @_Z8glu_siluIdEvPKT_PS0_ii
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
	s_cbranch_execz .LBB13_6
; %bb.1:
	s_mov_b32 s4, s5
	s_ashr_i32 s5, s5, 31
                                        ; implicit-def: $vgpr0_vgpr1
	s_mov_b32 s2, exec_lo
	v_or_b32_e32 v5, s5, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ne_u64_e32 0, v[4:5]
	s_xor_b32 s3, exec_lo, s2
	s_cbranch_execz .LBB13_3
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
.LBB13_3:
	s_and_not1_saveexec_b32 s2, s3
	s_cbranch_execz .LBB13_5
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
.LBB13_5:
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
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	s_mov_b32 s4, 0x6a5dcb37
	global_load_b64 v[4:5], v[4:5], off
	s_mov_b32 s5, 0x3e5ade15
	s_waitcnt vmcnt(1)
	v_mul_f64 v[6:7], v[0:1], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[0:1], -v[0:1]
	s_mov_b32 s0, 0x3b39803f
	s_mov_b32 s1, 0xbc7abc9e
	v_cvt_i32_f64_e32 v12, v[6:7]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[8:9], v[6:7], s[0:1], v[8:9]
	s_mov_b32 s0, 0xfca7ab0c
	s_mov_b32 s1, 0x3e928af3
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], s[4:5], s[0:1]
	s_mov_b32 s0, 0x623fde64
	s_mov_b32 s1, 0x3ec71dee
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x7c89e6b0
	s_mov_b32 s1, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x14761f6e
	s_mov_b32 s1, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x1852b7b0
	s_mov_b32 s1, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x11122322
	s_mov_b32 s1, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x555502a1
	s_mov_b32 s1, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 0x55555511
	s_mov_b32 s1, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	s_mov_b32 s0, 11
	s_mov_b32 s1, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], s[0:1]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[0:1]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], 1.0
	v_fma_f64 v[6:7], v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v12
	v_add_f64 v[6:7], v[6:7], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0x3ff00000, v7, s0
	v_div_scale_f64 v[8:9], null, v[6:7], v[6:7], v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_div_scale_f64 v[12:13], vcc_lo, v[0:1], v[6:7], v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[14:15], v[12:13], v[10:11]
	v_fma_f64 v[8:9], -v[8:9], v[14:15], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[14:15]
	v_add_co_u32 v2, vcc_lo, s2, v2
	v_add_co_ci_u32_e64 v3, null, s3, v3, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[0:1], v[8:9], v[6:7], v[0:1]
	s_waitcnt vmcnt(0)
	v_mul_f64 v[0:1], v[4:5], v[0:1]
	global_store_b64 v[2:3], v[0:1], off
.LBB13_6:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z8glu_siluIdEvPKT_PS0_ii
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
		.amdhsa_inst_pref_size 15
		.amdhsa_exception_fp_ieee_invalid_op 0
		.amdhsa_exception_fp_denorm_src 0
		.amdhsa_exception_fp_ieee_div_zero 0
		.amdhsa_exception_fp_ieee_overflow 0
		.amdhsa_exception_fp_ieee_underflow 0
		.amdhsa_exception_fp_ieee_inexact 0
		.amdhsa_exception_int_div_zero 0
	.end_amdhsa_kernel
	.section	.text._Z8glu_siluIdEvPKT_PS0_ii,"axG",@progbits,_Z8glu_siluIdEvPKT_PS0_ii,comdat
.Lfunc_end13:
	.size	_Z8glu_siluIdEvPKT_PS0_ii, .Lfunc_end13-_Z8glu_siluIdEvPKT_PS0_ii
                                        ; -- End function
	.set _Z8glu_siluIdEvPKT_PS0_ii.num_vgpr, 16
	.set _Z8glu_siluIdEvPKT_PS0_ii.num_agpr, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.numbered_sgpr, 19
	.set _Z8glu_siluIdEvPKT_PS0_ii.num_named_barrier, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.private_seg_size, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.uses_vcc, 1
	.set _Z8glu_siluIdEvPKT_PS0_ii.uses_flat_scratch, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.has_dyn_sized_stack, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.has_recursion, 0
	.set _Z8glu_siluIdEvPKT_PS0_ii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1808
; TotalNumSgprs: 21
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
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
    .name:           widen_bf16_f32
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         widen_bf16_f32.kd
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
        .size:           8
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
      - .offset:         184
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 320
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid
    .private_segment_fixed_size: 0
    .sgpr_count:     58
    .sgpr_spill_count: 0
    .symbol:         _Z15gqa_masked_attnIfEvPKT_S2_S2_PS0_iiiiid.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     52
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
        .size:           8
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
      - .offset:         184
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 512
    .kernarg_segment_align: 8
    .kernarg_segment_size: 320
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid
    .private_segment_fixed_size: 0
    .sgpr_count:     58
    .sgpr_spill_count: 0
    .symbol:         _Z15gqa_masked_attnIdEvPKT_S2_S2_PS0_iiiiid.kd
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
      - .offset:         176
        .size:           4
        .value_kind:     hidden_dynamic_lds_size
    .group_segment_fixed_size: 256
    .kernarg_segment_align: 8
    .kernarg_segment_size: 312
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii
    .private_segment_fixed_size: 0
    .sgpr_count:     38
    .sgpr_spill_count: 0
    .symbol:         _Z15mla_masked_attnIfEvPKT_S2_S2_PS0_iiiiii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     52
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
    .name:           _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii
    .private_segment_fixed_size: 0
    .sgpr_count:     50
    .sgpr_spill_count: 0
    .symbol:         _Z15mla_masked_attnIdEvPKT_S2_S2_PS0_iiiiii.kd
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
    .name:           _Z12rope_partialIfEvPT_iiiiPKS0_S3_
    .private_segment_fixed_size: 0
    .sgpr_count:     28
    .sgpr_spill_count: 0
    .symbol:         _Z12rope_partialIfEvPT_iiiiPKS0_S3_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     21
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
    .name:           _Z12rope_partialIdEvPT_iiiiPKS0_S3_
    .private_segment_fixed_size: 0
    .sgpr_count:     28
    .sgpr_spill_count: 0
    .symbol:         _Z12rope_partialIdEvPT_iiiiPKS0_S3_.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     52
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
    .name:           _Z8gelu_mulIfEvPKT_S2_PS0_l
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z8gelu_mulIfEvPKT_S2_PS0_l.kd
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
    .name:           _Z8gelu_mulIdEvPKT_S2_PS0_l
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z8gelu_mulIdEvPKT_S2_PS0_l.kd
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
    .name:           _Z8glu_geluIfEvPKT_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         _Z8glu_geluIfEvPKT_PS0_ii.kd
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
    .name:           _Z8glu_geluIdEvPKT_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         _Z8glu_geluIdEvPKT_PS0_ii.kd
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
    .name:           _Z8glu_siluIfEvPKT_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         _Z8glu_siluIfEvPKT_PS0_ii.kd
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
    .name:           _Z8glu_siluIdEvPKT_PS0_ii
    .private_segment_fixed_size: 0
    .sgpr_count:     21
    .sgpr_spill_count: 0
    .symbol:         _Z8glu_siluIdEvPKT_PS0_ii.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     16
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
