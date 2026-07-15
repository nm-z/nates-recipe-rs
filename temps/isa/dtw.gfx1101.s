	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	_Z19dtw_antidiag_kernelPKdPdiii ; -- Begin function _Z19dtw_antidiag_kernelPKdPdiii
	.globl	_Z19dtw_antidiag_kernelPKdPdiii
	.p2align	8
	.type	_Z19dtw_antidiag_kernelPKdPdiii,@function
_Z19dtw_antidiag_kernelPKdPdiii:        ; @_Z19dtw_antidiag_kernelPKdPdiii
; %bb.0:
	s_load_b128 s[4:7], s[0:1], 0x10
	s_waitcnt lgkmcnt(0)
	s_load_b32 s7, s[0:1], 0x2c
	s_add_i32 s3, s6, 2
	s_waitcnt lgkmcnt(0)
	s_and_b32 s6, s7, 0xffff
	s_sub_i32 s7, s3, s5
	s_mul_i32 s2, s2, s6
	s_max_i32 s6, s7, 1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add3_u32 v0, s2, s6, v0
	s_mov_b32 s2, exec_lo
	v_cmpx_lt_i32_e32 0, v0
	s_cbranch_execz .LBB0_3
; %bb.1:
	v_sub_nc_u32_e32 v1, s3, v0
	v_cmp_ge_i32_e64 s3, s4, v0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(SALU_CYCLE_1)
	v_cmp_gt_i32_e32 vcc_lo, 1, v1
	v_cmp_lt_i32_e64 s2, s5, v1
	s_or_b32 s2, vcc_lo, s2
	s_xor_b32 s2, s2, -1
	s_delay_alu instid0(SALU_CYCLE_1) | instskip(NEXT) | instid1(SALU_CYCLE_1)
	s_and_b32 s2, s3, s2
	s_and_b32 exec_lo, exec_lo, s2
	s_cbranch_execz .LBB0_3
; %bb.2:
	v_dual_mov_b32 v3, 0 :: v_dual_add_nc_u32 v4, -1, v0
	v_add_nc_u32_e32 v11, -1, v1
	s_load_b128 s[0:3], s[0:1], 0x0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_lo_u32 v5, v4, s5
	v_add_nc_u32_e32 v2, v5, v11
	v_add_nc_u32_e32 v12, v5, v4
	v_mad_u64_u32 v[4:5], null, v0, s5, v[0:1]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[5:6], 3, v[2:3]
	v_add_nc_u32_e32 v2, v12, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_lshlrev_b64 v[7:8], 3, v[2:3]
	v_add_nc_u32_e32 v2, v11, v4
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[9:10], 3, v[2:3]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v7, vcc_lo, s2, v7
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_co_ci_u32_e64 v8, null, s3, v8, vcc_lo
	v_add_nc_u32_e32 v2, v12, v11
	v_add_co_u32 v9, vcc_lo, s2, v9
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v10, null, s3, v10, vcc_lo
	s_clause 0x1
	global_load_b64 v[7:8], v[7:8], off
	global_load_b64 v[9:10], v[9:10], off
	v_lshlrev_b64 v[11:12], 3, v[2:3]
	v_add_nc_u32_e32 v2, v4, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_co_u32 v11, vcc_lo, s2, v11
	v_add_co_ci_u32_e64 v12, null, s3, v12, vcc_lo
	v_add_co_u32 v5, vcc_lo, s0, v5
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_4) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v6, null, s1, v6, vcc_lo
	global_load_b64 v[11:12], v[11:12], off
	v_lshlrev_b64 v[0:1], 3, v[2:3]
	global_load_b64 v[5:6], v[5:6], off
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	s_waitcnt vmcnt(3)
	v_max_f64 v[7:8], v[7:8], v[7:8]
	s_waitcnt vmcnt(2)
	v_max_f64 v[9:10], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_min_f64 v[7:8], v[7:8], v[9:10]
	s_waitcnt vmcnt(1)
	v_max_f64 v[9:10], v[11:12], v[11:12]
	v_min_f64 v[7:8], v[7:8], v[9:10]
	s_waitcnt vmcnt(0)
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[5:6], v[5:6], v[7:8]
	global_store_b64 v[0:1], v[5:6], off
.LBB0_3:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z19dtw_antidiag_kernelPKdPdiii
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
		.amdhsa_next_free_vgpr 13
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
.Lfunc_end0:
	.size	_Z19dtw_antidiag_kernelPKdPdiii, .Lfunc_end0-_Z19dtw_antidiag_kernelPKdPdiii
                                        ; -- End function
	.set _Z19dtw_antidiag_kernelPKdPdiii.num_vgpr, 13
	.set _Z19dtw_antidiag_kernelPKdPdiii.num_agpr, 0
	.set _Z19dtw_antidiag_kernelPKdPdiii.numbered_sgpr, 8
	.set _Z19dtw_antidiag_kernelPKdPdiii.num_named_barrier, 0
	.set _Z19dtw_antidiag_kernelPKdPdiii.private_seg_size, 0
	.set _Z19dtw_antidiag_kernelPKdPdiii.uses_vcc, 1
	.set _Z19dtw_antidiag_kernelPKdPdiii.uses_flat_scratch, 0
	.set _Z19dtw_antidiag_kernelPKdPdiii.has_dyn_sized_stack, 0
	.set _Z19dtw_antidiag_kernelPKdPdiii.has_recursion, 0
	.set _Z19dtw_antidiag_kernelPKdPdiii.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 460
; TotalNumSgprs: 10
; NumVgprs: 13
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 10
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
	.protected	_Z15dtw_init_kernelPdi  ; -- Begin function _Z15dtw_init_kernelPdi
	.globl	_Z15dtw_init_kernelPdi
	.p2align	8
	.type	_Z15dtw_init_kernelPdi,@function
_Z15dtw_init_kernelPdi:                 ; @_Z15dtw_init_kernelPdi
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
	v_cmp_eq_u32_e32 vcc_lo, 0, v1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_3) | instid1(VALU_DEP_3)
	v_lshlrev_b64 v[2:3], 3, v[1:2]
	v_cndmask_b32_e64 v1, 0x7fe1ccf3, 0, vcc_lo
	v_cndmask_b32_e64 v0, 0x85ebc8a0, 0, vcc_lo
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, s0, s0, v2
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v3, s0
	global_store_b64 v[2:3], v[0:1], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel _Z15dtw_init_kernelPdi
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
	.size	_Z15dtw_init_kernelPdi, .Lfunc_end1-_Z15dtw_init_kernelPdi
                                        ; -- End function
	.set _Z15dtw_init_kernelPdi.num_vgpr, 4
	.set _Z15dtw_init_kernelPdi.num_agpr, 0
	.set _Z15dtw_init_kernelPdi.numbered_sgpr, 5
	.set _Z15dtw_init_kernelPdi.num_named_barrier, 0
	.set _Z15dtw_init_kernelPdi.private_seg_size, 0
	.set _Z15dtw_init_kernelPdi.uses_vcc, 1
	.set _Z15dtw_init_kernelPdi.uses_flat_scratch, 0
	.set _Z15dtw_init_kernelPdi.has_dyn_sized_stack, 0
	.set _Z15dtw_init_kernelPdi.has_recursion, 0
	.set _Z15dtw_init_kernelPdi.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 148
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_dc9beb25bb7d514,@object ; @__hip_cuid_dc9beb25bb7d514
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_dc9beb25bb7d514
__hip_cuid_dc9beb25bb7d514:
	.byte	0                               ; 0x0
	.size	__hip_cuid_dc9beb25bb7d514, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_dc9beb25bb7d514
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
    .name:           _Z19dtw_antidiag_kernelPKdPdiii
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         _Z19dtw_antidiag_kernelPKdPdiii.kd
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
    .name:           _Z15dtw_init_kernelPdi
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         _Z15dtw_init_kernelPdi.kd
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
