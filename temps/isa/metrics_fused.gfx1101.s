	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z13ss_res_kernelPKdS0_Pdi ; -- Begin function _Z13ss_res_kernelPKdS0_Pdi
	.globl	_Z13ss_res_kernelPKdS0_Pdi
	.p2align	8
	.type	_Z13ss_res_kernelPKdS0_Pdi,@function
_Z13ss_res_kernelPKdS0_Pdi:             ; @_Z13ss_res_kernelPKdS0_Pdi
; %bb.0:
	s_clause 0x3
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b32 s10, s[0:1], 0x18
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[8:9], s[0:1], 0x10
	s_add_u32 s0, s0, 32
	s_addc_u32 s1, s1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_mad_u64_u32 v[3:4], null, s2, s3, v[0:1]
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s10, v3
	s_cbranch_execz .LBB0_4
; %bb.1:
	s_load_b32 s1, s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s0, 0
	s_waitcnt lgkmcnt(0)
	s_mul_i32 s1, s1, s3
	.p2align	6
.LBB0_2:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v4, 31, v3
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[4:5], 3, v[3:4]
	v_add_nc_u32_e32 v3, s1, v3
	v_add_co_u32 v6, vcc_lo, s6, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v7, null, s7, v5, vcc_lo
	v_add_co_u32 v4, vcc_lo, s4, v4
	v_add_co_ci_u32_e64 v5, null, s5, v5, vcc_lo
	global_load_b64 v[6:7], v[6:7], off
	global_load_b64 v[4:5], v[4:5], off
	v_cmp_le_i32_e32 vcc_lo, s10, v3
	s_or_b32 s0, vcc_lo, s0
	s_waitcnt vmcnt(0)
	v_add_f64 v[4:5], v[6:7], -v[4:5]
	s_delay_alu instid0(VALU_DEP_1)
	v_fma_f64 v[1:2], v[4:5], v[4:5], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB0_2
; %bb.3:
	s_or_b32 exec_lo, exec_lo, s0
.LBB0_4:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s2
	v_mbcnt_lo_u32_b32 v9, -1, 0
	v_and_b32_e32 v10, 31, v0
	s_mov_b32 s0, exec_lo
	v_lshl_or_b32 v5, v9, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v9
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v6, v3, v9, 2
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v7, v3, v9, 2
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v8, v3, v9, 2
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_co_ci_u32_e64 v3, null, 0, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b32_e32 v9, 2, v3
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	v_cmpx_eq_u32_e32 0, v10
	s_cbranch_execz .LBB0_6
; %bb.5:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB0_6:
	s_or_b32 exec_lo, exec_lo, s0
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB0_10
; %bb.7:
	s_add_i32 s3, s3, 31
	v_mov_b32_e32 v1, 0
	s_lshr_b32 s1, s3, 5
	v_mov_b32_e32 v2, 0
	v_cmp_gt_u32_e32 vcc_lo, s1, v10
	s_and_saveexec_b32 s1, vcc_lo
; %bb.8:
	v_lshlrev_b32_e32 v1, 3, v10
	ds_load_b64 v[1:2], v1
; %bb.9:
	s_or_b32 exec_lo, exec_lo, s1
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
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
.LBB0_10:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB0_16
; %bb.11:
	v_mov_b32_e32 v4, 0
	v_bfrev_b32_e32 v5, 1
	s_mov_b32 s0, exec_lo
.LBB0_12:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ctz_i32_b32 s1, s0
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s3, v2, s1
	v_readlane_b32 s2, v1, s1
	s_lshl_b32 s1, 1, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 s0, s0, s1
	v_add_f64 v[4:5], v[4:5], s[2:3]
	s_cmp_lg_u32 s0, 0
	s_cbranch_scc1 .LBB0_12
; %bb.13:
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_mov_b32 s0, 0
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB0_16
; %bb.14:
	s_load_b64 s[2:3], s[8:9], 0x0
	v_mov_b32_e32 v6, 0
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v2, s2 :: v_dual_mov_b32 v3, s3
.LBB0_15:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[0:1], v[2:3], v[4:5]
	global_atomic_cmpswap_b64 v[0:1], v6, v[0:3], s[8:9] glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB0_15
.LBB0_16:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z13ss_res_kernelPKdS0_Pdi
		.amdhsa_group_segment_fixed_size 512
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
		.amdhsa_next_free_vgpr 11
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
.Lfunc_end0:
	.size	_Z13ss_res_kernelPKdS0_Pdi, .Lfunc_end0-_Z13ss_res_kernelPKdS0_Pdi
                                        ; -- End function
	.set _Z13ss_res_kernelPKdS0_Pdi.num_vgpr, 11
	.set _Z13ss_res_kernelPKdS0_Pdi.num_agpr, 0
	.set _Z13ss_res_kernelPKdS0_Pdi.numbered_sgpr, 11
	.set _Z13ss_res_kernelPKdS0_Pdi.num_named_barrier, 0
	.set _Z13ss_res_kernelPKdS0_Pdi.private_seg_size, 0
	.set _Z13ss_res_kernelPKdS0_Pdi.uses_vcc, 1
	.set _Z13ss_res_kernelPKdS0_Pdi.uses_flat_scratch, 0
	.set _Z13ss_res_kernelPKdS0_Pdi.has_dyn_sized_stack, 0
	.set _Z13ss_res_kernelPKdS0_Pdi.has_recursion, 0
	.set _Z13ss_res_kernelPKdS0_Pdi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 944
; TotalNumSgprs: 13
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 512 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 13
; NumVGPRsForWavesPerEU: 11
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
	.protected	_Z10mse_kernelPKdS0_Pdi ; -- Begin function _Z10mse_kernelPKdS0_Pdi
	.globl	_Z10mse_kernelPKdS0_Pdi
	.p2align	8
	.type	_Z10mse_kernelPKdS0_Pdi,@function
_Z10mse_kernelPKdS0_Pdi:                ; @_Z10mse_kernelPKdS0_Pdi
; %bb.0:
	s_load_b32 s10, s[0:1], 0x18
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, 0
	v_mov_b32_e32 v4, 0
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc1 .LBB1_2
; %bb.1:
	v_cvt_f64_u32_e32 v[3:4], s10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], 1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_div_scale_f64 v[9:10], vcc_lo, 1.0, v[3:4], 1.0
	v_mul_f64 v[11:12], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[11:12], v[9:10]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[5:6], v[3:4], 1.0
.LBB1_2:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[8:9], s[0:1], 0x10
	s_add_u32 s0, s0, 32
	s_addc_u32 s1, s1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[5:6], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s10, v5
	s_cbranch_execz .LBB1_6
; %bb.3:
	s_load_b32 s1, s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s0, 0
	s_waitcnt lgkmcnt(0)
	s_mul_i32 s1, s1, s3
	.p2align	6
.LBB1_4:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[6:7], 3, v[5:6]
	v_add_nc_u32_e32 v5, s1, v5
	v_add_co_u32 v8, vcc_lo, s4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s5, v7, vcc_lo
	v_add_co_u32 v6, vcc_lo, s6, v6
	v_add_co_ci_u32_e64 v7, null, s7, v7, vcc_lo
	global_load_b64 v[8:9], v[8:9], off
	global_load_b64 v[6:7], v[6:7], off
	v_cmp_le_i32_e32 vcc_lo, s10, v5
	s_or_b32 s0, vcc_lo, s0
	s_waitcnt vmcnt(0)
	v_add_f64 v[6:7], v[8:9], -v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[6:7], v[6:7], v[6:7]
	v_fma_f64 v[1:2], v[3:4], v[6:7], v[1:2]
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB1_4
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s0
.LBB1_6:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s2
	v_mbcnt_lo_u32_b32 v9, -1, 0
	v_and_b32_e32 v10, 31, v0
	s_mov_b32 s0, exec_lo
	v_lshl_or_b32 v5, v9, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v9
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v6, v3, v9, 2
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v7, v3, v9, 2
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v8, v3, v9, 2
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_co_ci_u32_e64 v3, null, 0, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b32_e32 v9, 2, v3
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	v_cmpx_eq_u32_e32 0, v10
	s_cbranch_execz .LBB1_8
; %bb.7:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB1_8:
	s_or_b32 exec_lo, exec_lo, s0
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB1_12
; %bb.9:
	s_add_i32 s3, s3, 31
	v_mov_b32_e32 v1, 0
	s_lshr_b32 s1, s3, 5
	v_mov_b32_e32 v2, 0
	v_cmp_gt_u32_e32 vcc_lo, s1, v10
	s_and_saveexec_b32 s1, vcc_lo
; %bb.10:
	v_lshlrev_b32_e32 v1, 3, v10
	ds_load_b64 v[1:2], v1
; %bb.11:
	s_or_b32 exec_lo, exec_lo, s1
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
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
.LBB1_12:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB1_18
; %bb.13:
	v_mov_b32_e32 v4, 0
	v_bfrev_b32_e32 v5, 1
	s_mov_b32 s0, exec_lo
.LBB1_14:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ctz_i32_b32 s1, s0
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s3, v2, s1
	v_readlane_b32 s2, v1, s1
	s_lshl_b32 s1, 1, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 s0, s0, s1
	v_add_f64 v[4:5], v[4:5], s[2:3]
	s_cmp_lg_u32 s0, 0
	s_cbranch_scc1 .LBB1_14
; %bb.15:
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_mov_b32 s0, 0
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB1_18
; %bb.16:
	s_load_b64 s[2:3], s[8:9], 0x0
	v_mov_b32_e32 v6, 0
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v2, s2 :: v_dual_mov_b32 v3, s3
.LBB1_17:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[0:1], v[2:3], v[4:5]
	global_atomic_cmpswap_b64 v[0:1], v6, v[0:3], s[8:9] glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB1_17
.LBB1_18:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z10mse_kernelPKdS0_Pdi
		.amdhsa_group_segment_fixed_size 512
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
		.amdhsa_next_free_vgpr 13
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
	.size	_Z10mse_kernelPKdS0_Pdi, .Lfunc_end1-_Z10mse_kernelPKdS0_Pdi
                                        ; -- End function
	.set _Z10mse_kernelPKdS0_Pdi.num_vgpr, 13
	.set _Z10mse_kernelPKdS0_Pdi.num_agpr, 0
	.set _Z10mse_kernelPKdS0_Pdi.numbered_sgpr, 11
	.set _Z10mse_kernelPKdS0_Pdi.num_named_barrier, 0
	.set _Z10mse_kernelPKdS0_Pdi.private_seg_size, 0
	.set _Z10mse_kernelPKdS0_Pdi.uses_vcc, 1
	.set _Z10mse_kernelPKdS0_Pdi.uses_flat_scratch, 0
	.set _Z10mse_kernelPKdS0_Pdi.has_dyn_sized_stack, 0
	.set _Z10mse_kernelPKdS0_Pdi.has_recursion, 0
	.set _Z10mse_kernelPKdS0_Pdi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1084
; TotalNumSgprs: 13
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 512 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 13
; NumVGPRsForWavesPerEU: 13
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
	.protected	_Z15accuracy_kernelPKdS0_Pdi ; -- Begin function _Z15accuracy_kernelPKdS0_Pdi
	.globl	_Z15accuracy_kernelPKdS0_Pdi
	.p2align	8
	.type	_Z15accuracy_kernelPKdS0_Pdi,@function
_Z15accuracy_kernelPKdS0_Pdi:           ; @_Z15accuracy_kernelPKdS0_Pdi
; %bb.0:
	s_load_b32 s10, s[0:1], 0x18
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, 0
	v_mov_b32_e32 v4, 0
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc1 .LBB2_2
; %bb.1:
	v_cvt_f64_u32_e32 v[3:4], s10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], 1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_div_scale_f64 v[9:10], vcc_lo, 1.0, v[3:4], 1.0
	v_mul_f64 v[11:12], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[11:12], v[9:10]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[5:6], v[3:4], 1.0
.LBB2_2:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[8:9], s[0:1], 0x10
	s_add_u32 s0, s0, 32
	s_addc_u32 s1, s1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[5:6], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s10, v5
	s_cbranch_execz .LBB2_6
; %bb.3:
	s_load_b32 s11, s[0:1], 0x0
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s1, 0
	s_waitcnt lgkmcnt(0)
	s_mul_i32 s11, s11, s3
	.p2align	6
.LBB2_4:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v6, 31, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[6:7], 3, v[5:6]
	v_add_nc_u32_e32 v5, s11, v5
	v_add_co_u32 v8, vcc_lo, s4, v6
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v9, null, s5, v7, vcc_lo
	v_add_co_u32 v6, vcc_lo, s6, v6
	v_add_co_ci_u32_e64 v7, null, s7, v7, vcc_lo
	global_load_b64 v[8:9], v[8:9], off
	global_load_b64 v[6:7], v[6:7], off
	s_waitcnt vmcnt(1)
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[8:9]
	s_waitcnt vmcnt(0)
	v_cmp_nle_f64_e64 s0, 0.5, v[6:7]
	s_xor_b32 vcc_lo, vcc_lo, s0
	v_dual_cndmask_b32 v7, 0, v4 :: v_dual_cndmask_b32 v6, 0, v3
	v_cmp_le_i32_e32 vcc_lo, s10, v5
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_add_f64 v[1:2], v[1:2], v[6:7]
	s_or_b32 s1, vcc_lo, s1
	s_and_not1_b32 exec_lo, exec_lo, s1
	s_cbranch_execnz .LBB2_4
; %bb.5:
	s_or_b32 exec_lo, exec_lo, s1
.LBB2_6:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s2
	v_mbcnt_lo_u32_b32 v9, -1, 0
	v_and_b32_e32 v10, 31, v0
	s_mov_b32 s0, exec_lo
	v_lshl_or_b32 v5, v9, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v9
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v6, v3, v9, 2
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v7, v3, v9, 2
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v8, v3, v9, 2
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_co_ci_u32_e64 v3, null, 0, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b32_e32 v9, 2, v3
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	v_cmpx_eq_u32_e32 0, v10
	s_cbranch_execz .LBB2_8
; %bb.7:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB2_8:
	s_or_b32 exec_lo, exec_lo, s0
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB2_12
; %bb.9:
	s_add_i32 s3, s3, 31
	v_mov_b32_e32 v1, 0
	s_lshr_b32 s1, s3, 5
	v_mov_b32_e32 v2, 0
	v_cmp_gt_u32_e32 vcc_lo, s1, v10
	s_and_saveexec_b32 s1, vcc_lo
; %bb.10:
	v_lshlrev_b32_e32 v1, 3, v10
	ds_load_b64 v[1:2], v1
; %bb.11:
	s_or_b32 exec_lo, exec_lo, s1
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
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
.LBB2_12:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB2_18
; %bb.13:
	v_mov_b32_e32 v4, 0
	v_bfrev_b32_e32 v5, 1
	s_mov_b32 s0, exec_lo
.LBB2_14:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ctz_i32_b32 s1, s0
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s3, v2, s1
	v_readlane_b32 s2, v1, s1
	s_lshl_b32 s1, 1, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 s0, s0, s1
	v_add_f64 v[4:5], v[4:5], s[2:3]
	s_cmp_lg_u32 s0, 0
	s_cbranch_scc1 .LBB2_14
; %bb.15:
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_mov_b32 s0, 0
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB2_18
; %bb.16:
	s_load_b64 s[2:3], s[8:9], 0x0
	v_mov_b32_e32 v6, 0
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v2, s2 :: v_dual_mov_b32 v3, s3
.LBB2_17:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[0:1], v[2:3], v[4:5]
	global_atomic_cmpswap_b64 v[0:1], v6, v[0:3], s[8:9] glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB2_17
.LBB2_18:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15accuracy_kernelPKdS0_Pdi
		.amdhsa_group_segment_fixed_size 512
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
		.amdhsa_next_free_vgpr 13
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
.Lfunc_end2:
	.size	_Z15accuracy_kernelPKdS0_Pdi, .Lfunc_end2-_Z15accuracy_kernelPKdS0_Pdi
                                        ; -- End function
	.set _Z15accuracy_kernelPKdS0_Pdi.num_vgpr, 13
	.set _Z15accuracy_kernelPKdS0_Pdi.num_agpr, 0
	.set _Z15accuracy_kernelPKdS0_Pdi.numbered_sgpr, 12
	.set _Z15accuracy_kernelPKdS0_Pdi.num_named_barrier, 0
	.set _Z15accuracy_kernelPKdS0_Pdi.private_seg_size, 0
	.set _Z15accuracy_kernelPKdS0_Pdi.uses_vcc, 1
	.set _Z15accuracy_kernelPKdS0_Pdi.uses_flat_scratch, 0
	.set _Z15accuracy_kernelPKdS0_Pdi.has_dyn_sized_stack, 0
	.set _Z15accuracy_kernelPKdS0_Pdi.has_recursion, 0
	.set _Z15accuracy_kernelPKdS0_Pdi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1096
; TotalNumSgprs: 14
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 512 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 14
; NumVGPRsForWavesPerEU: 13
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
	.protected	_Z17argmax_acc_kernelPKdS0_Pdii ; -- Begin function _Z17argmax_acc_kernelPKdS0_Pdii
	.globl	_Z17argmax_acc_kernelPKdS0_Pdii
	.p2align	8
	.type	_Z17argmax_acc_kernelPKdS0_Pdii,@function
_Z17argmax_acc_kernelPKdS0_Pdii:        ; @_Z17argmax_acc_kernelPKdS0_Pdii
; %bb.0:
	s_load_b64 s[10:11], s[0:1], 0x18
	v_mov_b32_e32 v1, 0
	v_dual_mov_b32 v2, 0 :: v_dual_mov_b32 v3, 0
	v_mov_b32_e32 v4, 0
	s_waitcnt lgkmcnt(0)
	s_cmp_lt_i32 s10, 1
	s_cbranch_scc1 .LBB3_2
; %bb.1:
	v_cvt_f64_u32_e32 v[3:4], s10
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[5:6], null, v[3:4], v[3:4], 1.0
	v_rcp_f64_e32 v[7:8], v[5:6]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_fma_f64 v[9:10], -v[5:6], v[7:8], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[7:8], v[7:8], v[9:10], v[7:8]
	v_div_scale_f64 v[9:10], vcc_lo, 1.0, v[3:4], 1.0
	v_mul_f64 v[11:12], v[9:10], v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[5:6], -v[5:6], v[11:12], v[9:10]
	v_div_fmas_f64 v[5:6], v[5:6], v[7:8], v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_div_fixup_f64 v[3:4], v[5:6], v[3:4], 1.0
.LBB3_2:
	s_clause 0x2
	s_load_b32 s3, s[0:1], 0x2c
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[8:9], s[0:1], 0x10
	s_add_u32 s0, s0, 32
	s_addc_u32 s1, s1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s3, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[5:6], null, s2, s3, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s10, v5
	s_cbranch_execz .LBB3_10
; %bb.3:
	s_load_b32 s16, s[0:1], 0x0
	v_mul_lo_u32 v6, s11, v5
	s_cmp_gt_i32 s11, 1
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_cselect_b32 s12, -1, 0
	s_add_u32 s13, s6, 8
	s_addc_u32 s14, s7, 0
	s_add_u32 s15, s4, 8
	s_addc_u32 s17, s5, 0
	s_mov_b32 s18, 0
	s_waitcnt lgkmcnt(0)
	s_mul_i32 s16, s16, s3
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mul_i32 s19, s16, s11
	s_branch .LBB3_5
.LBB3_4:                                ;   in Loop: Header=BB3_5 Depth=1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[1:2], v[1:2], v[7:8]
	v_add_nc_u32_e32 v5, s16, v5
	v_add_nc_u32_e32 v6, s19, v6
	v_cmp_le_i32_e32 vcc_lo, s10, v5
	s_or_b32 s18, vcc_lo, s18
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s18
	s_cbranch_execz .LBB3_9
.LBB3_5:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB3_7 Depth 2
	v_dual_mov_b32 v8, v4 :: v_dual_mov_b32 v7, v3
	s_and_not1_b32 vcc_lo, exec_lo, s12
	s_cbranch_vccnz .LBB3_4
; %bb.6:                                ;   in Loop: Header=BB3_5 Depth=1
	v_mul_lo_u32 v7, v5, s11
	s_mov_b32 s20, 1
	v_mov_b32_e32 v16, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v8, 31, v7
	v_lshlrev_b64 v[7:8], 3, v[7:8]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v9, vcc_lo, s6, v7
	v_add_co_ci_u32_e64 v10, null, s7, v8, vcc_lo
	v_add_co_u32 v11, vcc_lo, s4, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_3) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v12, null, s5, v8, vcc_lo
	global_load_b64 v[8:9], v[9:10], off
	global_load_b64 v[10:11], v[11:12], off
	v_ashrrev_i32_e32 v7, 31, v6
	v_lshlrev_b64 v[14:15], 3, v[6:7]
	v_mov_b32_e32 v7, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v12, vcc_lo, s13, v14
	v_add_co_ci_u32_e64 v13, null, s14, v15, vcc_lo
	v_add_co_u32 v14, vcc_lo, s15, v14
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v15, null, s17, v15, vcc_lo
	.p2align	6
.LBB3_7:                                ;   Parent Loop BB3_5 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	global_load_b64 v[17:18], v[14:15], off
	global_load_b64 v[19:20], v[12:13], off
	v_add_co_u32 v12, s1, v12, 8
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v13, null, 0, v13, s1
	v_add_co_u32 v14, s1, v14, 8
	v_add_co_ci_u32_e64 v15, null, 0, v15, s1
	s_waitcnt vmcnt(1)
	v_cmp_gt_f64_e32 vcc_lo, v[17:18], v[10:11]
	s_waitcnt vmcnt(0)
	v_cmp_gt_f64_e64 s0, v[19:20], v[8:9]
	v_cndmask_b32_e64 v16, v16, s20, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_4) | instid1(SALU_CYCLE_1)
	v_cndmask_b32_e64 v7, v7, s20, s0
	v_dual_cndmask_b32 v11, v11, v18 :: v_dual_cndmask_b32 v10, v10, v17
	v_cndmask_b32_e64 v9, v9, v20, s0
	v_cndmask_b32_e64 v8, v8, v19, s0
	s_add_i32 s20, s20, 1
	s_cmp_eq_u32 s11, s20
	s_cbranch_scc0 .LBB3_7
; %bb.8:                                ;   in Loop: Header=BB3_5 Depth=1
	v_cmp_eq_u32_e32 vcc_lo, v16, v7
	v_dual_cndmask_b32 v8, 0, v4 :: v_dual_cndmask_b32 v7, 0, v3
	s_branch .LBB3_4
.LBB3_9:
	s_or_b32 exec_lo, exec_lo, s18
.LBB3_10:
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_3) | instid1(VALU_DEP_2)
	s_or_b32 exec_lo, exec_lo, s2
	v_mbcnt_lo_u32_b32 v9, -1, 0
	v_and_b32_e32 v10, 31, v0
	s_mov_b32 s0, exec_lo
	v_lshl_or_b32 v5, v9, 2, 64
	v_cmp_gt_u32_e32 vcc_lo, 24, v9
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 8, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 28, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v6, v3, v9, 2
	ds_bpermute_b32 v3, v6, v1
	ds_bpermute_b32 v4, v6, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 4, vcc_lo
	v_cmp_gt_u32_e32 vcc_lo, 30, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v7, v3, v9, 2
	ds_bpermute_b32 v3, v7, v1
	ds_bpermute_b32 v4, v7, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_cndmask_b32_e64 v3, 0, 2, vcc_lo
	v_cmp_ne_u32_e32 vcc_lo, 31, v9
	s_delay_alu instid0(VALU_DEP_2)
	v_add_lshl_u32 v8, v3, v9, 2
	ds_bpermute_b32 v3, v8, v1
	ds_bpermute_b32 v4, v8, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_add_co_ci_u32_e64 v3, null, 0, v9, vcc_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_lshlrev_b32_e32 v9, 2, v3
	ds_bpermute_b32 v3, v9, v1
	ds_bpermute_b32 v4, v9, v2
	v_cmpx_eq_u32_e32 0, v10
	s_cbranch_execz .LBB3_12
; %bb.11:
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
	v_lshrrev_b32_e32 v3, 2, v0
	ds_store_b64 v3, v[1:2]
.LBB3_12:
	s_or_b32 exec_lo, exec_lo, s0
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
	s_mov_b32 s0, exec_lo
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	v_cmpx_gt_u32_e32 32, v0
	s_cbranch_execz .LBB3_16
; %bb.13:
	s_add_i32 s3, s3, 31
	v_mov_b32_e32 v1, 0
	s_lshr_b32 s1, s3, 5
	v_mov_b32_e32 v2, 0
	v_cmp_gt_u32_e32 vcc_lo, s1, v10
	s_and_saveexec_b32 s1, vcc_lo
; %bb.14:
	v_lshlrev_b32_e32 v1, 3, v10
	ds_load_b64 v[1:2], v1
; %bb.15:
	s_or_b32 exec_lo, exec_lo, s1
	s_waitcnt lgkmcnt(0)
	ds_bpermute_b32 v3, v5, v1
	ds_bpermute_b32 v4, v5, v2
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[1:2], v[1:2], v[3:4]
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
.LBB3_16:
	s_or_b32 exec_lo, exec_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_mov_b32 s0, exec_lo
	v_cmpx_eq_u32_e32 0, v0
	s_cbranch_execz .LBB3_22
; %bb.17:
	v_mov_b32_e32 v4, 0
	v_bfrev_b32_e32 v5, 1
	s_mov_b32 s0, exec_lo
.LBB3_18:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_ctz_i32_b32 s1, s0
	s_delay_alu instid0(VALU_DEP_4) | instid1(SALU_CYCLE_1)
	v_readlane_b32 s3, v2, s1
	v_readlane_b32 s2, v1, s1
	s_lshl_b32 s1, 1, s1
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 s0, s0, s1
	v_add_f64 v[4:5], v[4:5], s[2:3]
	s_cmp_lg_u32 s0, 0
	s_cbranch_scc1 .LBB3_18
; %bb.19:
	v_mbcnt_lo_u32_b32 v0, exec_lo, 0
	s_mov_b32 s0, 0
	s_mov_b32 s1, exec_lo
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_eq_u32_e32 0, v0
	s_xor_b32 s1, exec_lo, s1
	s_cbranch_execz .LBB3_22
; %bb.20:
	s_load_b64 s[2:3], s[8:9], 0x0
	v_mov_b32_e32 v6, 0
	s_waitcnt lgkmcnt(0)
	v_dual_mov_b32 v2, s2 :: v_dual_mov_b32 v3, s3
.LBB3_21:                               ; =>This Inner Loop Header: Depth=1
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[0:1], v[2:3], v[4:5]
	global_atomic_cmpswap_b64 v[0:1], v6, v[0:3], s[8:9] glc
	s_waitcnt vmcnt(0)
	v_cmp_eq_u64_e32 vcc_lo, v[0:1], v[2:3]
	v_dual_mov_b32 v3, v1 :: v_dual_mov_b32 v2, v0
	s_or_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_not1_b32 exec_lo, exec_lo, s0
	s_cbranch_execnz .LBB3_21
.LBB3_22:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z17argmax_acc_kernelPKdS0_Pdii
		.amdhsa_group_segment_fixed_size 512
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
		.amdhsa_next_free_vgpr 21
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
.Lfunc_end3:
	.size	_Z17argmax_acc_kernelPKdS0_Pdii, .Lfunc_end3-_Z17argmax_acc_kernelPKdS0_Pdii
                                        ; -- End function
	.set _Z17argmax_acc_kernelPKdS0_Pdii.num_vgpr, 21
	.set _Z17argmax_acc_kernelPKdS0_Pdii.num_agpr, 0
	.set _Z17argmax_acc_kernelPKdS0_Pdii.numbered_sgpr, 21
	.set _Z17argmax_acc_kernelPKdS0_Pdii.num_named_barrier, 0
	.set _Z17argmax_acc_kernelPKdS0_Pdii.private_seg_size, 0
	.set _Z17argmax_acc_kernelPKdS0_Pdii.uses_vcc, 1
	.set _Z17argmax_acc_kernelPKdS0_Pdii.uses_flat_scratch, 0
	.set _Z17argmax_acc_kernelPKdS0_Pdii.has_dyn_sized_stack, 0
	.set _Z17argmax_acc_kernelPKdS0_Pdii.has_recursion, 0
	.set _Z17argmax_acc_kernelPKdS0_Pdii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1352
; TotalNumSgprs: 23
; NumVgprs: 21
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 512 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 23
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
	.text
	.protected	_Z15bce_grad_kernelPKdS0_PdiS0_ ; -- Begin function _Z15bce_grad_kernelPKdS0_PdiS0_
	.globl	_Z15bce_grad_kernelPKdS0_PdiS0_
	.p2align	8
	.type	_Z15bce_grad_kernelPKdS0_PdiS0_,@function
_Z15bce_grad_kernelPKdS0_PdiS0_:        ; @_Z15bce_grad_kernelPKdS0_PdiS0_
; %bb.0:
	s_clause 0x1
	s_load_b32 s3, s[0:1], 0x34
	s_load_b32 s14, s[0:1], 0x18
	s_add_u32 s4, s0, 40
	s_addc_u32 s5, s1, 0
	s_waitcnt lgkmcnt(0)
	s_and_b32 s10, s3, 0xffff
	s_delay_alu instid0(SALU_CYCLE_1)
	v_mad_u64_u32 v[1:2], null, s2, s10, v[0:1]
	s_mov_b32 s2, exec_lo
	v_cmpx_gt_i32_e64 s14, v1
	s_cbranch_execz .LBB4_3
; %bb.1:
	s_load_b64 s[8:9], s[0:1], 0x20
	s_load_b32 s11, s[4:5], 0x0
	s_clause 0x1
	s_load_b128 s[4:7], s[0:1], 0x0
	s_load_b64 s[2:3], s[0:1], 0x10
	s_mov_b32 s12, 0x9abcaf48
	s_mov_b32 s15, 0
	s_mov_b32 s13, 0x3e7ad7f2
	s_waitcnt lgkmcnt(0)
	s_load_b64 s[8:9], s[8:9], 0x0
	s_mul_i32 s1, s11, s10
	s_mov_b32 s10, 0xca501acb
	s_mov_b32 s11, 0x3fefffff
.LBB4_2:                                ; =>This Inner Loop Header: Depth=1
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	v_add_nc_u32_e32 v1, s1, v1
	v_add_co_u32 v4, vcc_lo, s4, v2
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v5, null, s5, v3, vcc_lo
	v_add_co_u32 v6, vcc_lo, s6, v2
	v_add_co_ci_u32_e64 v7, null, s7, v3, vcc_lo
	global_load_b64 v[4:5], v[4:5], off
	v_add_co_u32 v2, s0, s2, v2
	global_load_b64 v[6:7], v[6:7], off
	v_add_co_ci_u32_e64 v3, null, s3, v3, s0
	s_waitcnt vmcnt(1)
	v_cmp_nlt_f64_e32 vcc_lo, s[10:11], v[4:5]
	v_cndmask_b32_e32 v0, 0xca501acb, v4, vcc_lo
	v_cndmask_b32_e32 v8, 0x3fefffff, v5, vcc_lo
	v_cmp_ngt_f64_e32 vcc_lo, s[12:13], v[4:5]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cndmask_b32_e32 v5, 0x3e7ad7f2, v8, vcc_lo
	v_cndmask_b32_e32 v4, 0x9abcaf48, v0, vcc_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_add_f64 v[8:9], -v[4:5], 1.0
	s_waitcnt vmcnt(0)
	v_add_f64 v[6:7], v[4:5], -v[6:7]
	v_mul_f64 v[4:5], v[4:5], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_scale_f64 v[8:9], null, v[4:5], v[4:5], v[6:7]
	v_rcp_f64_e32 v[10:11], v[8:9]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_fma_f64 v[12:13], -v[8:9], v[10:11], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], v[10:11], v[12:13], v[10:11]
	v_div_scale_f64 v[12:13], vcc_lo, v[6:7], v[4:5], v[6:7]
	v_mul_f64 v[14:15], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], -v[8:9], v[14:15], v[12:13]
	v_div_fmas_f64 v[8:9], v[8:9], v[10:11], v[14:15]
	v_cmp_le_i32_e32 vcc_lo, s14, v1
	s_or_b32 s15, vcc_lo, s15
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_div_fixup_f64 v[4:5], v[8:9], v[4:5], v[6:7]
	s_waitcnt lgkmcnt(0)
	v_mul_f64 v[4:5], s[8:9], v[4:5]
	global_store_b64 v[2:3], v[4:5], off
	s_and_not1_b32 exec_lo, exec_lo, s15
	s_cbranch_execnz .LBB4_2
.LBB4_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15bce_grad_kernelPKdS0_PdiS0_
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
		.amdhsa_next_free_vgpr 16
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
.Lfunc_end4:
	.size	_Z15bce_grad_kernelPKdS0_PdiS0_, .Lfunc_end4-_Z15bce_grad_kernelPKdS0_PdiS0_
                                        ; -- End function
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.num_vgpr, 16
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.num_agpr, 0
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.numbered_sgpr, 16
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.num_named_barrier, 0
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.private_seg_size, 0
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.uses_vcc, 1
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.uses_flat_scratch, 0
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.has_dyn_sized_stack, 0
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.has_recursion, 0
	.set _Z15bce_grad_kernelPKdS0_PdiS0_.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 472
; TotalNumSgprs: 18
; NumVgprs: 16
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 18
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
	.type	__hip_cuid_9f9ffdcfda7b6bee,@object ; @__hip_cuid_9f9ffdcfda7b6bee
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_9f9ffdcfda7b6bee
__hip_cuid_9f9ffdcfda7b6bee:
	.byte	0                               ; 0x0
	.size	__hip_cuid_9f9ffdcfda7b6bee, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_9f9ffdcfda7b6bee
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
    .group_segment_fixed_size: 512
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z13ss_res_kernelPKdS0_Pdi
    .private_segment_fixed_size: 0
    .sgpr_count:     13
    .sgpr_spill_count: 0
    .symbol:         _Z13ss_res_kernelPKdS0_Pdi.kd
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
    .group_segment_fixed_size: 512
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z10mse_kernelPKdS0_Pdi
    .private_segment_fixed_size: 0
    .sgpr_count:     13
    .sgpr_spill_count: 0
    .symbol:         _Z10mse_kernelPKdS0_Pdi.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     13
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
    .group_segment_fixed_size: 512
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z15accuracy_kernelPKdS0_Pdi
    .private_segment_fixed_size: 0
    .sgpr_count:     14
    .sgpr_spill_count: 0
    .symbol:         _Z15accuracy_kernelPKdS0_Pdi.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     13
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
    .group_segment_fixed_size: 512
    .kernarg_segment_align: 8
    .kernarg_segment_size: 288
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           _Z17argmax_acc_kernelPKdS0_Pdii
    .private_segment_fixed_size: 0
    .sgpr_count:     23
    .sgpr_spill_count: 0
    .symbol:         _Z17argmax_acc_kernelPKdS0_Pdii.kd
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
    .name:           _Z15bce_grad_kernelPKdS0_PdiS0_
    .private_segment_fixed_size: 0
    .sgpr_count:     18
    .sgpr_spill_count: 0
    .symbol:         _Z15bce_grad_kernelPKdS0_PdiS0_.kd
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
