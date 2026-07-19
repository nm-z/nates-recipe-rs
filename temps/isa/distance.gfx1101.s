	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	pairwise_l2_kernel      ; -- Begin function pairwise_l2_kernel
	.globl	pairwise_l2_kernel
	.p2align	8
	.type	pairwise_l2_kernel,@function
pairwise_l2_kernel:                     ; @pairwise_l2_kernel
; %bb.0:
	s_clause 0x2
	s_load_b128 s[4:7], s[0:1], 0x18
	s_load_b128 s[8:11], s[0:1], 0x0
	s_load_b64 s[12:13], s[0:1], 0x10
	v_bfe_u32 v8, v0, 10, 10
	v_and_b32_e32 v9, 0x3ff, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_lshl_add_u32 v7, s3, 4, v8
	v_lshl_add_u32 v0, s2, 4, v9
	s_waitcnt lgkmcnt(0)
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_cmp_gt_i32_e64 s0, s4, v7
	v_cmp_gt_i32_e64 s1, s5, v0
	s_cmp_lt_i32 s6, 1
	s_cbranch_scc1 .LBB0_10
; %bb.1:
	v_lshlrev_b32_e32 v1, 3, v9
	v_lshlrev_b32_e32 v10, 7, v8
	v_mul_lo_u32 v12, s6, v7
	v_mul_lo_u32 v13, s6, v0
	s_mov_b32 s2, 0
	v_add_nc_u32_e32 v11, 0x800, v1
	v_dual_mov_b32 v1, 0 :: v_dual_add_nc_u32 v14, v10, v1
	v_mov_b32_e32 v2, 0
	s_mov_b32 s3, s6
	s_delay_alu instid0(VALU_DEP_3)
	v_add_nc_u32_e32 v15, v11, v10
	s_branch .LBB0_3
.LBB0_2:                                ;   in Loop: Header=BB0_3 Depth=1
	s_add_i32 s2, s2, 16
	s_add_i32 s3, s3, -16
	s_cmp_ge_i32 s2, s6
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB0_11
.LBB0_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB0_9 Depth 2
	v_add_nc_u32_e32 v5, s2, v9
	v_mov_b32_e32 v3, 0
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, s6, v5
	s_and_b32 s14, s0, vcc_lo
	s_and_saveexec_b32 s7, s14
	s_cbranch_execz .LBB0_5
; %bb.4:                                ;   in Loop: Header=BB0_3 Depth=1
	v_add_nc_u32_e32 v3, v5, v12
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s8, v3
	v_add_co_ci_u32_e64 v4, null, s9, v4, vcc_lo
	global_load_b64 v[3:4], v[3:4], off
.LBB0_5:                                ;   in Loop: Header=BB0_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s7
	v_dual_mov_b32 v5, 0 :: v_dual_add_nc_u32 v16, s2, v8
	v_mov_b32_e32 v6, 0
	s_waitcnt vmcnt(0)
	ds_store_b64 v14, v[3:4]
	v_cmp_gt_i32_e32 vcc_lo, s6, v16
	s_and_b32 s14, s1, vcc_lo
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s7, s14
	s_cbranch_execz .LBB0_7
; %bb.6:                                ;   in Loop: Header=BB0_3 Depth=1
	v_add_nc_u32_e32 v3, v16, v13
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s10, v3
	v_add_co_ci_u32_e64 v4, null, s11, v4, vcc_lo
	global_load_b64 v[5:6], v[3:4], off
.LBB0_7:                                ;   in Loop: Header=BB0_3 Depth=1
	s_or_b32 exec_lo, exec_lo, s7
	s_cmp_le_i32 s6, s2
	s_waitcnt vmcnt(0)
	ds_store_b64 v15, v[5:6]
	s_waitcnt lgkmcnt(0)
	s_barrier
	buffer_gl0_inv
	s_cbranch_scc1 .LBB0_2
; %bb.8:                                ;   in Loop: Header=BB0_3 Depth=1
	v_med3_i32 v3, s3, 1, 16
	v_dual_mov_b32 v4, v11 :: v_dual_mov_b32 v5, v10
.LBB0_9:                                ;   Parent Loop BB0_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ds_load_b64 v[16:17], v5
	ds_load_b64 v[18:19], v4
	v_add_nc_u32_e32 v3, -1, v3
	v_add_nc_u32_e32 v5, 8, v5
	v_add_nc_u32_e32 v4, 0x80, v4
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_cmp_eq_u32_e32 vcc_lo, 0, v3
	s_waitcnt lgkmcnt(0)
	v_add_f64 v[16:17], v[16:17], -v[18:19]
	v_fma_f64 v[1:2], v[16:17], v[16:17], v[1:2]
	s_cbranch_vccz .LBB0_9
	s_branch .LBB0_2
.LBB0_10:
	v_mov_b32_e32 v1, 0
	v_mov_b32_e32 v2, 0
.LBB0_11:
	v_cmp_gt_i32_e32 vcc_lo, s4, v7
	v_cmp_gt_i32_e64 s0, s5, v0
	s_and_b32 s0, vcc_lo, s0
	s_delay_alu instid0(SALU_CYCLE_1)
	s_and_saveexec_b32 s1, s0
	s_cbranch_execz .LBB0_13
; %bb.12:
	v_mad_u64_u32 v[3:4], null, s5, v7, v[0:1]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ashrrev_i32_e32 v4, 31, v3
	v_lshlrev_b64 v[3:4], 3, v[3:4]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v3, vcc_lo, s12, v3
	v_add_co_ci_u32_e64 v4, null, s13, v4, vcc_lo
	global_store_b64 v[3:4], v[1:2], off
.LBB0_13:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel pairwise_l2_kernel
		.amdhsa_group_segment_fixed_size 4096
		.amdhsa_private_segment_fixed_size 0
		.amdhsa_kernarg_size 36
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
		.amdhsa_next_free_vgpr 20
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
	.size	pairwise_l2_kernel, .Lfunc_end0-pairwise_l2_kernel
                                        ; -- End function
	.set pairwise_l2_kernel.num_vgpr, 20
	.set pairwise_l2_kernel.num_agpr, 0
	.set pairwise_l2_kernel.numbered_sgpr, 15
	.set pairwise_l2_kernel.num_named_barrier, 0
	.set pairwise_l2_kernel.private_seg_size, 0
	.set pairwise_l2_kernel.uses_vcc, 1
	.set pairwise_l2_kernel.uses_flat_scratch, 0
	.set pairwise_l2_kernel.has_dyn_sized_stack, 0
	.set pairwise_l2_kernel.has_recursion, 0
	.set pairwise_l2_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 580
; TotalNumSgprs: 17
; NumVgprs: 20
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 4096 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 2
; NumSGPRsForWavesPerEU: 17
; NumVGPRsForWavesPerEU: 20
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
	.type	__hip_cuid_17a5d4dcc061b649,@object ; @__hip_cuid_17a5d4dcc061b649
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_17a5d4dcc061b649
__hip_cuid_17a5d4dcc061b649:
	.byte	0                               ; 0x0
	.size	__hip_cuid_17a5d4dcc061b649, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_17a5d4dcc061b649
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
    .group_segment_fixed_size: 4096
    .kernarg_segment_align: 8
    .kernarg_segment_size: 36
    .language:       OpenCL C
    .language_version:
      - 2
      - 0
    .max_flat_workgroup_size: 1024
    .name:           pairwise_l2_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     17
    .sgpr_spill_count: 0
    .symbol:         pairwise_l2_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     20
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
