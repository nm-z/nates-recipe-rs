	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	activationx_relu_squared_kernel ; -- Begin function activationx_relu_squared_kernel
	.globl	activationx_relu_squared_kernel
	.p2align	8
	.type	activationx_relu_squared_kernel,@function
activationx_relu_squared_kernel:        ; @activationx_relu_squared_kernel
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_2)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[2:3], v[2:3]
	v_cmp_lt_f64_e32 vcc_lo, 0, v[2:3]
	v_dual_cndmask_b32 v3, 0, v5 :: v_dual_cndmask_b32 v2, 0, v4
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB0_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_relu_squared_kernel
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
	.size	activationx_relu_squared_kernel, .Lfunc_end0-activationx_relu_squared_kernel
                                        ; -- End function
	.set activationx_relu_squared_kernel.num_vgpr, 6
	.set activationx_relu_squared_kernel.num_agpr, 0
	.set activationx_relu_squared_kernel.numbered_sgpr, 5
	.set activationx_relu_squared_kernel.num_named_barrier, 0
	.set activationx_relu_squared_kernel.private_seg_size, 0
	.set activationx_relu_squared_kernel.uses_vcc, 1
	.set activationx_relu_squared_kernel.uses_flat_scratch, 0
	.set activationx_relu_squared_kernel.has_dyn_sized_stack, 0
	.set activationx_relu_squared_kernel.has_recursion, 0
	.set activationx_relu_squared_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 172
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
	.protected	activationx_shifted_softplus_kernel ; -- Begin function activationx_shifted_softplus_kernel
	.globl	activationx_shifted_softplus_kernel
	.p2align	8
	.type	activationx_shifted_softplus_kernel,@function
activationx_shifted_softplus_kernel:    ; @activationx_shifted_softplus_kernel
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
	s_mov_b32 s4, 0xfefa39ef
	s_mov_b32 s5, 0xbfe62e42
	s_mov_b32 s6, 0xfca7ab0c
	s_mov_b32 s8, 0x6a5dcb37
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_mov_b32 s7, 0x3e928af3
	s_mov_b32 s9, 0x3e5ade15
	s_mov_b32 s10, 0xbf559e2b
	s_mov_b32 s11, 0x3fc3ab76
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], |v[2:3]|, s[0:1]
	s_mov_b32 s1, 0xbc7abc9e
	s_mov_b32 s0, 0x3b39803f
	v_cmp_nlt_f64_e64 vcc_lo, 0x4090cc00, |v[2:3]|
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[4:5], -|v[2:3]|
	v_cvt_i32_f64_e32 v10, v[4:5]
	v_max_f64 v[2:3], v[2:3], v[2:3]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[6:7]
	s_mov_b32 s1, 0x3c7abc9e
	v_max_f64 v[2:3], v[2:3], 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], s[8:9], s[6:7]
	s_mov_b32 s6, 0x623fde64
	s_mov_b32 s7, 0x3ec71dee
	s_mov_b32 s8, 0x6b47b09a
	s_mov_b32 s9, 0x3fc38538
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x7c89e6b0
	s_mov_b32 s7, 0x3efa0199
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x14761f6e
	s_mov_b32 s7, 0x3f2a01a0
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x1852b7b0
	s_mov_b32 s7, 0x3f56c16c
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x11122322
	s_mov_b32 s7, 0x3f811111
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x555502a1
	s_mov_b32 s7, 0x3fa55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 0x55555511
	s_mov_b32 s7, 0x3fc55555
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s6, 11
	s_mov_b32 s7, 0x3fe00000
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], s[6:7]
	s_mov_b32 s7, 0x3fe55555
	s_mov_b32 s6, 0x55555555
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_dual_cndmask_b32 v5, 0, v5 :: v_dual_cndmask_b32 v4, 0, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[4:5], 1.0
	v_frexp_mant_f64_e32 v[8:9], v[6:7]
	v_frexp_exp_i32_f64_e32 v12, v[6:7]
	v_add_f64 v[10:11], v[6:7], -1.0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, s[6:7], v[8:9]
	s_mov_b32 s6, 0x55555780
	v_add_f64 v[8:9], v[10:11], -v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[10:11]
	v_subrev_co_ci_u32_e64 v28, null, 0, v12, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[8:9], 1.0
	v_cmp_neq_f64_e32 vcc_lo, 0x7ff00000, v[4:5]
	v_sub_nc_u32_e32 v14, 0, v28
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[6:7], v[6:7], v14
	v_add_f64 v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[12:13], v[6:7], 1.0
	v_add_f64 v[18:19], v[6:7], -1.0
	v_ldexp_f64 v[8:9], v[8:9], v14
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[10:11], v[12:13], -1.0
	v_add_f64 v[20:21], v[18:19], 1.0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[6:7], -v[10:11]
	v_add_f64 v[6:7], v[6:7], -v[20:21]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[8:9], v[10:11]
	v_add_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[14:15], v[12:13], v[10:11]
	v_add_f64 v[20:21], v[18:19], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_rcp_f64_e32 v[16:17], v[14:15]
	v_add_f64 v[12:13], v[14:15], -v[12:13]
	v_add_f64 v[18:19], v[20:21], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[22:23], -v[14:15], v[16:17], 1.0
	v_add_f64 v[6:7], v[6:7], -v[18:19]
	v_fma_f64 v[16:17], v[22:23], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[14:15], v[16:17], 1.0
	v_fma_f64 v[8:9], v[8:9], v[16:17], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[16:17], v[20:21], v[8:9]
	v_mul_f64 v[22:23], v[14:15], v[16:17]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[12:13], v[16:17], v[14:15], -v[22:23]
	v_fma_f64 v[12:13], v[16:17], v[10:11], v[12:13]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[24:25], v[22:23], v[12:13]
	v_add_f64 v[26:27], v[20:21], -v[24:25]
	v_add_f64 v[18:19], v[24:25], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[26:27]
	v_add_f64 v[12:13], v[18:19], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[20:21], v[20:21], -v[24:25]
	v_add_f64 v[6:7], v[6:7], v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[12:13], v[6:7]
	v_add_f64 v[12:13], v[26:27], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_mul_f64 v[18:19], v[8:9], v[12:13]
	v_add_f64 v[24:25], v[26:27], -v[12:13]
	v_mul_f64 v[20:21], v[14:15], v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[24:25]
	v_fma_f64 v[14:15], v[18:19], v[14:15], -v[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[18:19], v[10:11], v[14:15]
	v_add_f64 v[14:15], v[20:21], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[22:23], v[12:13], -v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[22:23]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[20:21], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[14:15]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_add_f64 v[12:13], v[16:17], v[18:19]
	v_add_f64 v[6:7], v[10:11], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[12:13], -v[16:17]
	v_add_f64 v[6:7], v[22:23], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[10:11], v[18:19], -v[10:11]
	v_mul_f64 v[6:7], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[6:7], v[10:11], v[6:7]
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
	s_mov_b32 s7, 0x3fe62e42
	s_mov_b32 s6, s4
	v_mul_f64 v[10:11], v[16:17], v[10:11]
	v_cvt_f64_i32_e32 v[16:17], v28
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[6:7], -v[8:9]
	v_add_f64 v[12:13], v[14:15], v[10:11]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_mul_f64 v[18:19], v[16:17], s[6:7]
	v_ldexp_f64 v[6:7], v[6:7], 1
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[8:9], v[12:13], -v[14:15]
	v_fma_f64 v[14:15], v[16:17], s[6:7], -v[18:19]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], v[10:11], -v[8:9]
	v_fma_f64 v[10:11], v[16:17], s[0:1], v[14:15]
	v_cmp_nge_f64_e64 s0, -1.0, v[4:5]
	v_cmp_ngt_f64_e64 s1, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
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
	v_cndmask_b32_e32 v7, 0x7ff00000, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	v_cndmask_b32_e32 v6, 0, v6, vcc_lo
	v_cmp_neq_f64_e32 vcc_lo, -1.0, v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v7, 0x7ff80000, v7, s1
	v_cndmask_b32_e32 v7, 0xfff00000, v7, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	v_add_f64 v[2:3], v[2:3], v[6:7]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], s[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_shifted_softplus_kernel
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
.Lfunc_end1:
	.size	activationx_shifted_softplus_kernel, .Lfunc_end1-activationx_shifted_softplus_kernel
                                        ; -- End function
	.set activationx_shifted_softplus_kernel.num_vgpr, 29
	.set activationx_shifted_softplus_kernel.num_agpr, 0
	.set activationx_shifted_softplus_kernel.numbered_sgpr, 12
	.set activationx_shifted_softplus_kernel.num_named_barrier, 0
	.set activationx_shifted_softplus_kernel.private_seg_size, 0
	.set activationx_shifted_softplus_kernel.uses_vcc, 1
	.set activationx_shifted_softplus_kernel.uses_flat_scratch, 0
	.set activationx_shifted_softplus_kernel.has_dyn_sized_stack, 0
	.set activationx_shifted_softplus_kernel.has_recursion, 0
	.set activationx_shifted_softplus_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1856
; TotalNumSgprs: 14
; NumVgprs: 29
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 3
; NumSGPRsForWavesPerEU: 14
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
	.text
	.protected	activationx_squareplus_kernel ; -- Begin function activationx_squareplus_kernel
	.globl	activationx_squareplus_kernel
	.p2align	8
	.type	activationx_squareplus_kernel,@function
activationx_squareplus_kernel:          ; @activationx_squareplus_kernel
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
	s_cbranch_execz .LBB2_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_fma_f64 v[4:5], v[2:3], v[2:3], s[0:1]
	v_cmp_gt_f64_e32 vcc_lo, 0x10000000, v[4:5]
	v_cndmask_b32_e64 v6, 0, 0x100, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v6
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
	v_add_co_u32 v0, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	v_add_f64 v[2:3], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_mul_f64 v[2:3], v[2:3], 0.5
	global_store_b64 v[0:1], v[2:3], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_squareplus_kernel
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
.Lfunc_end2:
	.size	activationx_squareplus_kernel, .Lfunc_end2-activationx_squareplus_kernel
                                        ; -- End function
	.set activationx_squareplus_kernel.num_vgpr, 12
	.set activationx_squareplus_kernel.num_agpr, 0
	.set activationx_squareplus_kernel.numbered_sgpr, 8
	.set activationx_squareplus_kernel.num_named_barrier, 0
	.set activationx_squareplus_kernel.private_seg_size, 0
	.set activationx_squareplus_kernel.uses_vcc, 1
	.set activationx_squareplus_kernel.uses_flat_scratch, 0
	.set activationx_squareplus_kernel.has_dyn_sized_stack, 0
	.set activationx_squareplus_kernel.has_recursion, 0
	.set activationx_squareplus_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 356
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
	.protected	activationx_star_relu_kernel ; -- Begin function activationx_star_relu_kernel
	.globl	activationx_star_relu_kernel
	.p2align	8
	.type	activationx_star_relu_kernel,@function
activationx_star_relu_kernel:           ; @activationx_star_relu_kernel
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s4, s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s4, v1
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b128 s[0:3], s[0:1], 0x18
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s6, v0
	v_add_co_ci_u32_e64 v1, null, s7, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_max_f64 v[2:3], v[2:3], v[2:3]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_max_f64 v[2:3], v[2:3], 0
	v_mul_f64 v[4:5], s[0:1], v[2:3]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[2:3], v[2:3], v[4:5], s[2:3]
	global_store_b64 v[0:1], v[2:3], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_star_relu_kernel
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
	.size	activationx_star_relu_kernel, .Lfunc_end3-activationx_star_relu_kernel
                                        ; -- End function
	.set activationx_star_relu_kernel.num_vgpr, 6
	.set activationx_star_relu_kernel.num_agpr, 0
	.set activationx_star_relu_kernel.numbered_sgpr, 8
	.set activationx_star_relu_kernel.num_named_barrier, 0
	.set activationx_star_relu_kernel.private_seg_size, 0
	.set activationx_star_relu_kernel.uses_vcc, 1
	.set activationx_star_relu_kernel.uses_flat_scratch, 0
	.set activationx_star_relu_kernel.has_dyn_sized_stack, 0
	.set activationx_star_relu_kernel.has_recursion, 0
	.set activationx_star_relu_kernel.has_indirect_call, 0
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
	.protected	activationx_prelu_kernel ; -- Begin function activationx_prelu_kernel
	.globl	activationx_prelu_kernel
	.p2align	8
	.type	activationx_prelu_kernel,@function
activationx_prelu_kernel:               ; @activationx_prelu_kernel
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
	s_cbranch_execz .LBB4_4
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_load_b64 s[0:1], s[0:1], 0x10
	s_mov_b32 s2, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s4, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s5, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmpx_nlt_f64_e32 0, v[2:3]
	s_cbranch_execz .LBB4_3
; %bb.2:
	v_add_co_u32 v4, vcc_lo, s6, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s7, v1, vcc_lo
	global_load_b64 v[4:5], v[4:5], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
.LBB4_3:
	s_or_b32 exec_lo, exec_lo, s2
	v_add_co_u32 v0, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s1, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB4_4:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_prelu_kernel
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
.Lfunc_end4:
	.size	activationx_prelu_kernel, .Lfunc_end4-activationx_prelu_kernel
                                        ; -- End function
	.set activationx_prelu_kernel.num_vgpr, 6
	.set activationx_prelu_kernel.num_agpr, 0
	.set activationx_prelu_kernel.numbered_sgpr, 8
	.set activationx_prelu_kernel.num_named_barrier, 0
	.set activationx_prelu_kernel.private_seg_size, 0
	.set activationx_prelu_kernel.uses_vcc, 1
	.set activationx_prelu_kernel.uses_flat_scratch, 0
	.set activationx_prelu_kernel.has_dyn_sized_stack, 0
	.set activationx_prelu_kernel.has_recursion, 0
	.set activationx_prelu_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 216
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
	.protected	activationx_glu_kernel  ; -- Begin function activationx_glu_kernel
	.globl	activationx_glu_kernel
	.p2align	8
	.type	activationx_glu_kernel,@function
activationx_glu_kernel:                 ; @activationx_glu_kernel
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
	s_ashr_i32 s5, s4, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 3
	s_mov_b32 s4, 0x6a5dcb37
	v_add_co_u32 v4, vcc_lo, v2, s0
	v_add_co_ci_u32_e64 v5, null, s1, v3, vcc_lo
	s_mov_b32 s0, 0x652b82fe
	s_mov_b32 s1, 0xbff71547
	s_clause 0x1
	global_load_b64 v[2:3], v[2:3], off
	global_load_b64 v[4:5], v[4:5], off
	s_mov_b32 s5, 0x3e5ade15
	s_waitcnt vmcnt(0)
	v_mul_f64 v[6:7], v[4:5], s[0:1]
	s_mov_b32 s0, 0xfefa39ef
	s_mov_b32 s1, 0xbfe62e42
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[6:7], v[6:7]
	v_fma_f64 v[8:9], v[6:7], s[0:1], -v[4:5]
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
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[8:9], v[10:11], 1.0
	v_fma_f64 v[6:7], v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[6:7], v[6:7], v12
	v_add_f64 v[6:7], v[6:7], 1.0
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
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[6:7], v[8:9], v[6:7], 1.0
	v_cndmask_b32_e32 v7, 0, v7, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v4, 0, v6, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_cndmask_b32_e64 v5, 0x3ff00000, v7, s0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_delay_alu instid0(VALU_DEP_2)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_glu_kernel
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
.Lfunc_end5:
	.size	activationx_glu_kernel, .Lfunc_end5-activationx_glu_kernel
                                        ; -- End function
	.set activationx_glu_kernel.num_vgpr, 16
	.set activationx_glu_kernel.num_agpr, 0
	.set activationx_glu_kernel.numbered_sgpr, 6
	.set activationx_glu_kernel.num_named_barrier, 0
	.set activationx_glu_kernel.private_seg_size, 0
	.set activationx_glu_kernel.uses_vcc, 1
	.set activationx_glu_kernel.uses_flat_scratch, 0
	.set activationx_glu_kernel.has_dyn_sized_stack, 0
	.set activationx_glu_kernel.has_recursion, 0
	.set activationx_glu_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 744
; TotalNumSgprs: 8
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 8
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
	.protected	activationx_reglu_kernel ; -- Begin function activationx_reglu_kernel
	.globl	activationx_reglu_kernel
	.p2align	8
	.type	activationx_reglu_kernel,@function
activationx_reglu_kernel:               ; @activationx_reglu_kernel
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
	s_ashr_i32 s5, s4, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	s_lshl_b64 s[0:1], s[4:5], 3
	v_add_co_u32 v4, vcc_lo, v2, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s1, v3, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_clause 0x1
	global_load_b64 v[4:5], v[4:5], off
	global_load_b64 v[2:3], v[2:3], off
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(1)
	v_max_f64 v[4:5], v[4:5], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_max_f64 v[4:5], v[4:5], 0
	s_waitcnt vmcnt(0)
	v_mul_f64 v[2:3], v[2:3], v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_reglu_kernel
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
.Lfunc_end6:
	.size	activationx_reglu_kernel, .Lfunc_end6-activationx_reglu_kernel
                                        ; -- End function
	.set activationx_reglu_kernel.num_vgpr, 6
	.set activationx_reglu_kernel.num_agpr, 0
	.set activationx_reglu_kernel.numbered_sgpr, 6
	.set activationx_reglu_kernel.num_named_barrier, 0
	.set activationx_reglu_kernel.private_seg_size, 0
	.set activationx_reglu_kernel.uses_vcc, 1
	.set activationx_reglu_kernel.uses_flat_scratch, 0
	.set activationx_reglu_kernel.has_dyn_sized_stack, 0
	.set activationx_reglu_kernel.has_recursion, 0
	.set activationx_reglu_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 220
; TotalNumSgprs: 8
; NumVgprs: 6
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
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
	.protected	activationx_crelu_kernel ; -- Begin function activationx_crelu_kernel
	.globl	activationx_crelu_kernel
	.p2align	8
	.type	activationx_crelu_kernel,@function
activationx_crelu_kernel:               ; @activationx_crelu_kernel
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
	s_ashr_i32 s5, s4, 31
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_lshl_b64 s[0:1], s[4:5], 3
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v6, vcc_lo, v0, s0
	v_add_co_ci_u32_e64 v7, null, s1, v1, vcc_lo
	s_waitcnt vmcnt(0)
	v_max_f64 v[4:5], v[2:3], v[2:3]
	v_max_f64 v[2:3], -v[2:3], -v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_max_f64 v[4:5], v[4:5], 0
	v_max_f64 v[2:3], v[2:3], 0
	s_clause 0x1
	global_store_b64 v[0:1], v[4:5], off
	global_store_b64 v[6:7], v[2:3], off
.LBB7_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_crelu_kernel
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
	.size	activationx_crelu_kernel, .Lfunc_end7-activationx_crelu_kernel
                                        ; -- End function
	.set activationx_crelu_kernel.num_vgpr, 8
	.set activationx_crelu_kernel.num_agpr, 0
	.set activationx_crelu_kernel.numbered_sgpr, 6
	.set activationx_crelu_kernel.num_named_barrier, 0
	.set activationx_crelu_kernel.private_seg_size, 0
	.set activationx_crelu_kernel.uses_vcc, 1
	.set activationx_crelu_kernel.uses_flat_scratch, 0
	.set activationx_crelu_kernel.has_dyn_sized_stack, 0
	.set activationx_crelu_kernel.has_recursion, 0
	.set activationx_crelu_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 224
; TotalNumSgprs: 8
; NumVgprs: 8
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 0
; NumSGPRsForWavesPerEU: 8
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
	.protected	activationx_softmin_kernel ; -- Begin function activationx_softmin_kernel
	.globl	activationx_softmin_kernel
	.p2align	8
	.type	activationx_softmin_kernel,@function
activationx_softmin_kernel:             ; @activationx_softmin_kernel
; %bb.0:
	s_load_b64 s[8:9], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_cmp_ge_i32 s2, s8
	s_cbranch_scc1 .LBB8_9
; %bb.1:
	s_load_b128 s[4:7], s[0:1], 0x0
	s_mul_hi_i32 s1, s9, s2
	s_mul_i32 s0, s9, s2
	s_delay_alu instid0(SALU_CYCLE_1)
	s_lshl_b64 s[0:1], s[0:1], 3
	s_waitcnt lgkmcnt(0)
	s_add_u32 s2, s4, s0
	s_addc_u32 s3, s5, s1
	s_cmp_lt_i32 s9, 2
	s_load_b64 s[10:11], s[2:3], 0x0
	s_cbranch_scc1 .LBB8_4
; %bb.2:
	s_add_i32 s8, s9, -1
	s_add_u32 s4, s4, s0
	s_addc_u32 s5, s5, s1
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
.LBB8_3:                                ; =>This Inner Loop Header: Depth=1
	s_load_b64 s[12:13], s[4:5], 0x0
	s_waitcnt lgkmcnt(0)
	v_cmp_lt_f64_e64 s14, s[12:13], s[10:11]
	s_and_b32 s14, s14, exec_lo
	s_cselect_b32 s11, s13, s11
	s_cselect_b32 s10, s12, s10
	s_add_i32 s8, s8, -1
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
	s_cmp_eq_u32 s8, 0
	s_cbranch_scc0 .LBB8_3
.LBB8_4:
	s_cmp_lt_i32 s9, 1
	s_cbranch_scc1 .LBB8_9
; %bb.5:
	v_mov_b32_e32 v0, 0
	s_add_u32 s4, s6, s0
	v_dual_mov_b32 v1, 0 :: v_dual_mov_b32 v2, 0
	s_addc_u32 s5, s7, s1
	s_mov_b32 s6, 0x652b82fe
	s_mov_b32 s12, 0xfefa39ef
	s_mov_b32 s14, 0x3b39803f
	s_mov_b32 s16, 0xfca7ab0c
	s_mov_b32 s18, 0x6a5dcb37
	s_mov_b32 s20, 0x623fde64
	s_mov_b32 s22, 0x7c89e6b0
	s_mov_b32 s24, 0x14761f6e
	s_mov_b32 s26, 0x1852b7b0
	s_mov_b32 s28, 0x11122322
	s_mov_b32 s30, 0x555502a1
	s_mov_b32 s34, 0x55555511
	s_mov_b32 s36, 11
	s_mov_b32 s7, 0xbff71547
	s_mov_b32 s13, 0xbfe62e42
	s_mov_b32 s15, 0xbc7abc9e
	s_mov_b32 s17, 0x3e928af3
	s_mov_b32 s19, 0x3e5ade15
	s_mov_b32 s21, 0x3ec71dee
	s_mov_b32 s23, 0x3efa0199
	s_mov_b32 s25, 0x3f2a01a0
	s_mov_b32 s27, 0x3f56c16c
	s_mov_b32 s29, 0x3f811111
	s_mov_b32 s31, 0x3fa55555
	s_mov_b32 s35, 0x3fc55555
	s_mov_b32 s37, 0x3fe00000
	s_mov_b64 s[38:39], s[4:5]
	s_mov_b32 s1, s9
.LBB8_6:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[3:4], v2, s[2:3]
	s_add_i32 s1, s1, -1
	s_waitcnt vmcnt(0) lgkmcnt(0)
	v_add_f64 v[3:4], v[3:4], -s[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_mul_f64 v[5:6], v[3:4], s[6:7]
	v_cmp_ngt_f64_e32 vcc_lo, 0xc0900000, v[3:4]
	v_cmp_nlt_f64_e64 s0, 0x4090cc00, v[3:4]
	v_rndne_f64_e32 v[5:6], v[5:6]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_fma_f64 v[7:8], v[5:6], s[12:13], -v[3:4]
	v_cvt_i32_f64_e32 v11, v[5:6]
	v_fma_f64 v[7:8], v[5:6], s[14:15], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], s[18:19], s[16:17]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[20:21]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[22:23]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[24:25]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[26:27]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[28:29]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[30:31]
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[34:35]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[9:10], v[7:8], v[9:10], s[36:37]
	v_fma_f64 v[9:10], v[7:8], v[9:10], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], v[7:8], v[9:10], 1.0
	v_ldexp_f64 v[5:6], v[5:6], v11
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cndmask_b32_e32 v6, 0x7ff00000, v6, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_add_u32 s2, s2, 8
	v_cndmask_b32_e32 v3, 0, v5, vcc_lo
	s_addc_u32 s3, s3, 0
	v_cndmask_b32_e64 v4, 0, v6, s0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[0:1], v[0:1], v[3:4]
	global_store_b64 v2, v[3:4], s[38:39]
	s_add_u32 s38, s38, 8
	s_addc_u32 s39, s39, 0
	s_cmp_eq_u32 s1, 0
	s_cbranch_scc0 .LBB8_6
; %bb.7:
	v_div_scale_f64 v[2:3], null, v[0:1], v[0:1], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[4:5], v[2:3]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[6:7], -v[2:3], v[4:5], 1.0
	v_fma_f64 v[4:5], v[4:5], v[6:7], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], -v[2:3], v[4:5], 1.0
	v_fma_f64 v[4:5], v[4:5], v[6:7], v[4:5]
	v_div_scale_f64 v[6:7], vcc_lo, 1.0, v[0:1], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[8:9], v[6:7], v[4:5]
	v_fma_f64 v[2:3], -v[2:3], v[8:9], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[2:3], v[2:3], v[4:5], v[8:9]
	v_div_fixup_f64 v[0:1], v[2:3], v[0:1], 1.0
	v_mov_b32_e32 v2, 0
.LBB8_8:                                ; =>This Inner Loop Header: Depth=1
	global_load_b64 v[3:4], v2, s[4:5]
	s_add_i32 s9, s9, -1
	s_waitcnt vmcnt(0)
	v_mul_f64 v[3:4], v[0:1], v[3:4]
	global_store_b64 v2, v[3:4], s[4:5]
	s_add_u32 s4, s4, 8
	s_addc_u32 s5, s5, 0
	s_cmp_lg_u32 s9, 0
	s_cbranch_scc1 .LBB8_8
.LBB8_9:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel activationx_softmin_kernel
		.amdhsa_group_segment_fixed_size 0
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 24
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
.Lfunc_end8:
	.size	activationx_softmin_kernel, .Lfunc_end8-activationx_softmin_kernel
                                        ; -- End function
	.set activationx_softmin_kernel.num_vgpr, 12
	.set activationx_softmin_kernel.num_agpr, 0
	.set activationx_softmin_kernel.numbered_sgpr, 40
	.set activationx_softmin_kernel.num_named_barrier, 0
	.set activationx_softmin_kernel.private_seg_size, 0
	.set activationx_softmin_kernel.uses_vcc, 1
	.set activationx_softmin_kernel.uses_flat_scratch, 0
	.set activationx_softmin_kernel.has_dyn_sized_stack, 0
	.set activationx_softmin_kernel.has_recursion, 0
	.set activationx_softmin_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 820
; TotalNumSgprs: 42
; NumVgprs: 12
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 42
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
	.type	__hip_cuid_2124a5eb2e20ae57,@object ; @__hip_cuid_2124a5eb2e20ae57
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_2124a5eb2e20ae57
__hip_cuid_2124a5eb2e20ae57:
	.byte	0                               ; 0x0
	.size	__hip_cuid_2124a5eb2e20ae57, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_2124a5eb2e20ae57
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
    .name:           activationx_relu_squared_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         activationx_relu_squared_kernel.kd
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
    .name:           activationx_shifted_softplus_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         activationx_shifted_softplus_kernel.kd
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
    .name:           activationx_squareplus_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         activationx_squareplus_kernel.kd
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
        .size:           8
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
    .name:           activationx_star_relu_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         activationx_star_relu_kernel.kd
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
    .name:           activationx_prelu_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         activationx_prelu_kernel.kd
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
    .name:           activationx_glu_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         activationx_glu_kernel.kd
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
    .name:           activationx_reglu_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         activationx_reglu_kernel.kd
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
    .name:           activationx_crelu_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         activationx_crelu_kernel.kd
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
    .group_segment_fixed_size: 0
    .kernarg_segment_align: 8
    .kernarg_segment_size: 24
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           activationx_softmin_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     42
    .sgpr_spill_count: 0
    .symbol:         activationx_softmin_kernel.kd
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
