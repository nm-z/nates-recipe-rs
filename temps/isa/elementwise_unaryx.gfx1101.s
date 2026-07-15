	.amdgcn_target "amdgcn-amd-amdhsa--gfx1101"
	.amdhsa_code_object_version 6
	.text
	.protected	elementwise_unaryx_sinc_kernel ; -- Begin function elementwise_unaryx_sinc_kernel
	.globl	elementwise_unaryx_sinc_kernel
	.p2align	8
	.type	elementwise_unaryx_sinc_kernel,@function
elementwise_unaryx_sinc_kernel:         ; @elementwise_unaryx_sinc_kernel
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
	s_cbranch_execz .LBB0_8
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_mov_b32 s4, exec_lo
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[4:5], v[2:3], off
	v_mov_b32_e32 v2, 0
	v_mov_b32_e32 v3, 0x3ff00000
	s_waitcnt vmcnt(0)
	v_cmpx_neq_f64_e32 0, v[4:5]
	s_cbranch_execz .LBB0_7
; %bb.2:
	s_mov_b32 s1, 0x400921fb
	s_mov_b32 s0, 0x54442d18
	s_mov_b32 s5, exec_lo
	v_mul_f64 v[2:3], v[4:5], s[0:1]
                                        ; implicit-def: $vgpr8
                                        ; implicit-def: $vgpr4_vgpr5
                                        ; implicit-def: $vgpr6_vgpr7
	s_delay_alu instid0(VALU_DEP_1)
	v_cmpx_ngt_f64_e64 0x41d00000, |v[2:3]|
	s_xor_b32 s5, exec_lo, s5
	s_cbranch_execz .LBB0_4
; %bb.3:
	v_ldexp_f64 v[4:5], |v[2:3]|, 0xffffff80
	v_cmp_le_f64_e64 vcc_lo, 0x7b000000, |v[2:3]|
	v_trig_preop_f64 v[6:7], |v[2:3]|, 0
	v_and_b32_e32 v8, 0x7fffffff, v3
	v_trig_preop_f64 v[18:19], |v[2:3]|, 2
	v_mov_b32_e32 v26, 0
	s_mov_b32 s1, 0x3ff921fb
	s_mov_b32 s6, 0x33145c07
	s_mov_b32 s7, 0x3c91a626
	v_dual_cndmask_b32 v5, v8, v5 :: v_dual_cndmask_b32 v4, v2, v4
	v_trig_preop_f64 v[8:9], |v[2:3]|, 1
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_mul_f64 v[10:11], v[6:7], v[4:5]
	v_mul_f64 v[24:25], v[18:19], v[4:5]
	v_mul_f64 v[12:13], v[8:9], v[4:5]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[6:7], v[4:5], -v[10:11]
	v_fma_f64 v[8:9], v[8:9], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[4:5], v[18:19], v[4:5], -v[24:25]
	v_add_f64 v[14:15], v[12:13], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[16:17], v[14:15], -v[12:13]
	v_add_f64 v[22:23], v[10:11], v[14:15]
	v_add_f64 v[20:21], v[14:15], -v[16:17]
	v_add_f64 v[6:7], v[6:7], -v[16:17]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_ldexp_f64 v[16:17], v[22:23], -2
	v_add_f64 v[10:11], v[22:23], -v[10:11]
	v_add_f64 v[12:13], v[12:13], -v[20:21]
	v_add_f64 v[20:21], v[24:25], v[8:9]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_cmp_neq_f64_e64 vcc_lo, 0x7ff00000, |v[16:17]|
	v_add_f64 v[10:11], v[14:15], -v[10:11]
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[12:13]
	v_fract_f64_e32 v[12:13], v[16:17]
	v_add_f64 v[14:15], v[20:21], v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_ldexp_f64 v[12:13], v[12:13], 2
	v_add_f64 v[16:17], v[10:11], v[14:15]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_dual_cndmask_b32 v13, 0, v13 :: v_dual_cndmask_b32 v12, 0, v12
	v_add_f64 v[22:23], v[16:17], v[12:13]
	v_add_f64 v[10:11], v[16:17], -v[10:11]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_2) | instid1(VALU_DEP_2)
	v_cmp_gt_f64_e32 vcc_lo, 0, v[22:23]
	v_add_f64 v[22:23], v[20:21], -v[24:25]
	v_cndmask_b32_e64 v27, 0, 0x40100000, vcc_lo
	v_add_f64 v[31:32], v[20:21], -v[22:23]
	v_add_f64 v[8:9], v[8:9], -v[22:23]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_4)
	v_add_f64 v[12:13], v[12:13], v[26:27]
	v_add_f64 v[27:28], v[14:15], -v[20:21]
	v_add_f64 v[22:23], v[24:25], -v[31:32]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[29:30], v[16:17], v[12:13]
	v_add_f64 v[33:34], v[14:15], -v[27:28]
	v_add_f64 v[6:7], v[6:7], -v[27:28]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_add_f64 v[8:9], v[8:9], v[22:23]
	v_cvt_i32_f64_e32 v29, v[29:30]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[20:21], v[20:21], -v[33:34]
	v_cvt_f64_i32_e32 v[27:28], v29
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[6:7], v[6:7], v[20:21]
	v_add_f64 v[12:13], v[12:13], -v[27:28]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_add_f64 v[6:7], v[8:9], v[6:7]
	v_add_f64 v[8:9], v[14:15], -v[10:11]
	v_add_f64 v[18:19], v[16:17], v[12:13]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], -v[12:13]
	v_cmp_le_f64_e32 vcc_lo, 0.5, v[18:19]
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[8:9], v[4:5]
	v_add_f64 v[6:7], v[16:17], -v[6:7]
	v_cndmask_b32_e64 v27, 0, 0x3ff00000, vcc_lo
	v_add_co_ci_u32_e64 v8, null, 0, v29, vcc_lo
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_add_f64 v[6:7], v[18:19], -v[26:27]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[9:10], v[6:7], v[4:5]
	v_mul_f64 v[11:12], v[9:10], s[0:1]
	v_add_f64 v[6:7], v[9:10], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[13:14], v[9:10], s[0:1], -v[11:12]
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[6:7], v[9:10], s[6:7], v[13:14]
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[6:7]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[11:12], v[6:7]
	v_add_f64 v[9:10], v[4:5], -v[11:12]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[9:10]
.LBB0_4:
	s_and_not1_saveexec_b32 s0, s5
	s_cbranch_execz .LBB0_6
; %bb.5:
	s_mov_b32 s6, 0x6dc9c883
	s_mov_b32 s7, 0x3fe45f30
	s_mov_b32 s9, 0xbc91a626
	v_mul_f64 v[4:5], |v[2:3]|, s[6:7]
	s_mov_b32 s6, 0x54442d18
	s_mov_b32 s7, 0xbff921fb
	s_mov_b32 s8, 0x33145c00
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[8:9], v[4:5]
	v_fma_f64 v[4:5], v[8:9], s[6:7], |v[2:3]|
	v_mul_f64 v[6:7], v[8:9], s[8:9]
	s_mov_b32 s6, 0x252049c0
	s_mov_b32 s7, 0xb97b839a
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[12:13], v[8:9], s[8:9], v[4:5]
	v_add_f64 v[10:11], v[4:5], v[6:7]
	s_mov_b32 s9, 0x3c91a626
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[4:5], v[4:5], -v[10:11]
	v_add_f64 v[10:11], v[10:11], -v[12:13]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_2)
	v_add_f64 v[4:5], v[4:5], v[6:7]
	v_fma_f64 v[6:7], v[8:9], s[8:9], v[6:7]
	v_add_f64 v[4:5], v[10:11], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_fma_f64 v[6:7], v[8:9], s[6:7], v[4:5]
	v_cvt_i32_f64_e32 v8, v[8:9]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[12:13], v[6:7]
	v_add_f64 v[10:11], v[4:5], -v[12:13]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[6:7], v[6:7], -v[10:11]
.LBB0_6:
	s_or_b32 exec_lo, exec_lo, s0
	v_mul_f64 v[9:10], v[4:5], v[4:5]
	s_mov_b32 s0, 0xb42fdfa7
	s_mov_b32 s6, 0xf9a43bb8
	s_mov_b32 s1, 0xbe5ae600
	s_mov_b32 s7, 0x3de5e0b2
	s_mov_b32 s8, 0x796cde01
	s_mov_b32 s9, 0x3ec71de3
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_mul_f64 v[19:20], v[6:7], 0.5
	v_fma_f64 v[11:12], v[9:10], s[6:7], s[0:1]
	s_mov_b32 s0, 0x9037ab78
	s_mov_b32 s6, 0x46cc5e42
	s_mov_b32 s1, 0x3e21eeb6
	s_mov_b32 s7, 0xbda907db
	v_mul_f64 v[15:16], v[9:10], 0.5
	v_fma_f64 v[13:14], v[9:10], s[6:7], s[0:1]
	s_mov_b32 s0, 0xa17f65f6
	s_mov_b32 s6, 0x19e83e5c
	s_mov_b32 s1, 0xbe927e4f
	s_mov_b32 s7, 0xbf2a01a0
	v_mul_f64 v[21:22], v[4:5], -v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[8:9]
	v_add_f64 v[17:18], -v[15:16], 1.0
	s_delay_alu instid0(VALU_DEP_4) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s0, 0x19f4ec90
	s_mov_b32 s1, 0x3efa01a0
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[6:7]
	s_mov_b32 s6, 0x11110bb3
	s_mov_b32 s7, 0x3f811111
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_add_f64 v[23:24], -v[17:18], 1.0
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s0, 0x16c16967
	s_mov_b32 s1, 0xbf56c16c
	s_delay_alu instid0(VALU_DEP_3) | instskip(NEXT) | instid1(VALU_DEP_3)
	v_fma_f64 v[11:12], v[9:10], v[11:12], s[6:7]
	v_add_f64 v[15:16], v[23:24], -v[15:16]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_2) | instid1(VALU_DEP_3)
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s1, 0x3fa55555
	s_mov_b32 s0, 0x55555555
	v_fma_f64 v[11:12], v[21:22], v[11:12], v[19:20]
	v_mul_f64 v[19:20], v[9:10], v[9:10]
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_4)
	v_fma_f64 v[15:16], v[4:5], -v[6:7], v[15:16]
	v_fma_f64 v[13:14], v[9:10], v[13:14], s[0:1]
	s_mov_b32 s1, 0xbfc55555
	s_delay_alu instid0(VALU_DEP_4) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_fma_f64 v[6:7], v[9:10], v[11:12], -v[6:7]
	v_fma_f64 v[9:10], v[19:20], v[13:14], v[15:16]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_fma_f64 v[6:7], v[21:22], s[0:1], v[6:7]
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	v_add_f64 v[9:10], v[17:18], v[9:10]
	s_delay_alu instid0(VALU_DEP_3) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_f64 v[4:5], v[4:5], -v[6:7]
	v_and_b32_e32 v6, 1, v8
	v_cmp_eq_u32_e32 vcc_lo, 0, v6
	v_lshlrev_b32_e32 v6, 30, v8
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_xor_b32_e32 v6, v6, v3
	v_dual_cndmask_b32 v5, v10, v5 :: v_dual_and_b32 v6, 0x80000000, v6
	v_cndmask_b32_e32 v4, v9, v4, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_2)
	v_xor_b32_e32 v5, v5, v6
	v_cndmask_b32_e64 v4, 0, v4, s0
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0x7ff80000, v5, s0
	v_div_scale_f64 v[6:7], null, v[2:3], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_2) | instid1(VALU_DEP_1)
	v_rcp_f64_e32 v[8:9], v[6:7]
	s_waitcnt_depctr 0xfff
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[10:11], -v[6:7], v[8:9], 1.0
	v_fma_f64 v[8:9], v[8:9], v[10:11], v[8:9]
	v_div_scale_f64 v[10:11], vcc_lo, v[4:5], v[2:3], v[4:5]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_mul_f64 v[12:13], v[10:11], v[8:9]
	v_fma_f64 v[6:7], -v[6:7], v[12:13], v[10:11]
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_div_fmas_f64 v[6:7], v[6:7], v[8:9], v[12:13]
	v_div_fixup_f64 v[2:3], v[6:7], v[2:3], v[4:5]
.LBB0_7:
	s_or_b32 exec_lo, exec_lo, s4
	v_add_co_u32 v0, vcc_lo, s2, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB0_8:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_sinc_kernel
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
		.amdhsa_next_free_vgpr 35
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
.Lfunc_end0:
	.size	elementwise_unaryx_sinc_kernel, .Lfunc_end0-elementwise_unaryx_sinc_kernel
                                        ; -- End function
	.set elementwise_unaryx_sinc_kernel.num_vgpr, 35
	.set elementwise_unaryx_sinc_kernel.num_agpr, 0
	.set elementwise_unaryx_sinc_kernel.numbered_sgpr, 10
	.set elementwise_unaryx_sinc_kernel.num_named_barrier, 0
	.set elementwise_unaryx_sinc_kernel.private_seg_size, 0
	.set elementwise_unaryx_sinc_kernel.uses_vcc, 1
	.set elementwise_unaryx_sinc_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_sinc_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_sinc_kernel.has_recursion, 0
	.set elementwise_unaryx_sinc_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 1764
; TotalNumSgprs: 12
; NumVgprs: 35
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 4
; NumSGPRsForWavesPerEU: 12
; NumVGPRsForWavesPerEU: 35
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
	.protected	elementwise_unaryx_frac_kernel ; -- Begin function elementwise_unaryx_frac_kernel
	.globl	elementwise_unaryx_frac_kernel
	.p2align	8
	.type	elementwise_unaryx_frac_kernel,@function
elementwise_unaryx_frac_kernel:         ; @elementwise_unaryx_frac_kernel
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
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_trunc_f64_e32 v[4:5], v[2:3]
	s_delay_alu instid0(VALU_DEP_1)
	v_add_f64 v[2:3], v[2:3], -v[4:5]
	global_store_b64 v[0:1], v[2:3], off
.LBB1_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_frac_kernel
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
.Lfunc_end1:
	.size	elementwise_unaryx_frac_kernel, .Lfunc_end1-elementwise_unaryx_frac_kernel
                                        ; -- End function
	.set elementwise_unaryx_frac_kernel.num_vgpr, 6
	.set elementwise_unaryx_frac_kernel.num_agpr, 0
	.set elementwise_unaryx_frac_kernel.numbered_sgpr, 5
	.set elementwise_unaryx_frac_kernel.num_named_barrier, 0
	.set elementwise_unaryx_frac_kernel.private_seg_size, 0
	.set elementwise_unaryx_frac_kernel.uses_vcc, 1
	.set elementwise_unaryx_frac_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_frac_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_frac_kernel.has_recursion, 0
	.set elementwise_unaryx_frac_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 164
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
	.protected	elementwise_unaryx_heaviside_kernel ; -- Begin function elementwise_unaryx_heaviside_kernel
	.globl	elementwise_unaryx_heaviside_kernel
	.p2align	8
	.type	elementwise_unaryx_heaviside_kernel,@function
elementwise_unaryx_heaviside_kernel:    ; @elementwise_unaryx_heaviside_kernel
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
	v_mov_b32_e32 v4, 0x3ff00000
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_lt_f64_e32 vcc_lo, 0, v[2:3]
	v_dual_cndmask_b32 v5, 0x3fe00000, v4 :: v_dual_mov_b32 v4, 0
	v_cmp_ngt_f64_e32 vcc_lo, 0, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_cndmask_b32_e32 v5, 0, v5, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[4:5], off
.LBB2_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_heaviside_kernel
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
.Lfunc_end2:
	.size	elementwise_unaryx_heaviside_kernel, .Lfunc_end2-elementwise_unaryx_heaviside_kernel
                                        ; -- End function
	.set elementwise_unaryx_heaviside_kernel.num_vgpr, 6
	.set elementwise_unaryx_heaviside_kernel.num_agpr, 0
	.set elementwise_unaryx_heaviside_kernel.numbered_sgpr, 5
	.set elementwise_unaryx_heaviside_kernel.num_named_barrier, 0
	.set elementwise_unaryx_heaviside_kernel.private_seg_size, 0
	.set elementwise_unaryx_heaviside_kernel.uses_vcc, 1
	.set elementwise_unaryx_heaviside_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_heaviside_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_heaviside_kernel.has_recursion, 0
	.set elementwise_unaryx_heaviside_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 184
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
	.protected	elementwise_unaryx_signbit_kernel ; -- Begin function elementwise_unaryx_signbit_kernel
	.globl	elementwise_unaryx_signbit_kernel
	.p2align	8
	.type	elementwise_unaryx_signbit_kernel,@function
elementwise_unaryx_signbit_kernel:      ; @elementwise_unaryx_signbit_kernel
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
	s_cbranch_execz .LBB3_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_ashrrev_i32_e32 v3, 31, v3
	s_delay_alu instid0(VALU_DEP_1)
	v_dual_mov_b32 v2, 0 :: v_dual_and_b32 v3, 0x3ff00000, v3
	global_store_b64 v[0:1], v[2:3], off
.LBB3_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_signbit_kernel
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
.Lfunc_end3:
	.size	elementwise_unaryx_signbit_kernel, .Lfunc_end3-elementwise_unaryx_signbit_kernel
                                        ; -- End function
	.set elementwise_unaryx_signbit_kernel.num_vgpr, 4
	.set elementwise_unaryx_signbit_kernel.num_agpr, 0
	.set elementwise_unaryx_signbit_kernel.numbered_sgpr, 5
	.set elementwise_unaryx_signbit_kernel.num_named_barrier, 0
	.set elementwise_unaryx_signbit_kernel.private_seg_size, 0
	.set elementwise_unaryx_signbit_kernel.uses_vcc, 1
	.set elementwise_unaryx_signbit_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_signbit_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_signbit_kernel.has_recursion, 0
	.set elementwise_unaryx_signbit_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 168
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
	.protected	elementwise_unaryx_isnan_kernel ; -- Begin function elementwise_unaryx_isnan_kernel
	.globl	elementwise_unaryx_isnan_kernel
	.p2align	8
	.type	elementwise_unaryx_isnan_kernel,@function
elementwise_unaryx_isnan_kernel:        ; @elementwise_unaryx_isnan_kernel
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
	s_cbranch_execz .LBB4_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_u_f64_e32 vcc_lo, v[2:3], v[2:3]
	v_mov_b32_e32 v2, 0
	v_cndmask_b32_e64 v3, 0, 0x3ff00000, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB4_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_isnan_kernel
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
.Lfunc_end4:
	.size	elementwise_unaryx_isnan_kernel, .Lfunc_end4-elementwise_unaryx_isnan_kernel
                                        ; -- End function
	.set elementwise_unaryx_isnan_kernel.num_vgpr, 4
	.set elementwise_unaryx_isnan_kernel.num_agpr, 0
	.set elementwise_unaryx_isnan_kernel.numbered_sgpr, 5
	.set elementwise_unaryx_isnan_kernel.num_named_barrier, 0
	.set elementwise_unaryx_isnan_kernel.private_seg_size, 0
	.set elementwise_unaryx_isnan_kernel.uses_vcc, 1
	.set elementwise_unaryx_isnan_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_isnan_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_isnan_kernel.has_recursion, 0
	.set elementwise_unaryx_isnan_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 168
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
	.protected	elementwise_unaryx_isinf_kernel ; -- Begin function elementwise_unaryx_isinf_kernel
	.globl	elementwise_unaryx_isinf_kernel
	.p2align	8
	.type	elementwise_unaryx_isinf_kernel,@function
elementwise_unaryx_isinf_kernel:        ; @elementwise_unaryx_isinf_kernel
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
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_class_f64_e64 s0, v[2:3], 0x204
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0, 0x3ff00000, s0
	global_store_b64 v[0:1], v[4:5], off
.LBB5_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_isinf_kernel
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
.Lfunc_end5:
	.size	elementwise_unaryx_isinf_kernel, .Lfunc_end5-elementwise_unaryx_isinf_kernel
                                        ; -- End function
	.set elementwise_unaryx_isinf_kernel.num_vgpr, 6
	.set elementwise_unaryx_isinf_kernel.num_agpr, 0
	.set elementwise_unaryx_isinf_kernel.numbered_sgpr, 5
	.set elementwise_unaryx_isinf_kernel.num_named_barrier, 0
	.set elementwise_unaryx_isinf_kernel.private_seg_size, 0
	.set elementwise_unaryx_isinf_kernel.uses_vcc, 1
	.set elementwise_unaryx_isinf_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_isinf_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_isinf_kernel.has_recursion, 0
	.set elementwise_unaryx_isinf_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 180
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
	.protected	elementwise_unaryx_isfinite_kernel ; -- Begin function elementwise_unaryx_isfinite_kernel
	.globl	elementwise_unaryx_isfinite_kernel
	.p2align	8
	.type	elementwise_unaryx_isfinite_kernel,@function
elementwise_unaryx_isfinite_kernel:     ; @elementwise_unaryx_isfinite_kernel
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
	v_mov_b32_e32 v4, 0
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_cmp_class_f64_e64 s0, v[2:3], 0x1f8
	s_delay_alu instid0(VALU_DEP_1)
	v_cndmask_b32_e64 v5, 0, 0x3ff00000, s0
	global_store_b64 v[0:1], v[4:5], off
.LBB6_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_isfinite_kernel
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
.Lfunc_end6:
	.size	elementwise_unaryx_isfinite_kernel, .Lfunc_end6-elementwise_unaryx_isfinite_kernel
                                        ; -- End function
	.set elementwise_unaryx_isfinite_kernel.num_vgpr, 6
	.set elementwise_unaryx_isfinite_kernel.num_agpr, 0
	.set elementwise_unaryx_isfinite_kernel.numbered_sgpr, 5
	.set elementwise_unaryx_isfinite_kernel.num_named_barrier, 0
	.set elementwise_unaryx_isfinite_kernel.private_seg_size, 0
	.set elementwise_unaryx_isfinite_kernel.uses_vcc, 1
	.set elementwise_unaryx_isfinite_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_isfinite_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_isfinite_kernel.has_recursion, 0
	.set elementwise_unaryx_isfinite_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 180
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
	.protected	elementwise_unaryx_logit_kernel ; -- Begin function elementwise_unaryx_logit_kernel
	.globl	elementwise_unaryx_logit_kernel
	.p2align	8
	.type	elementwise_unaryx_logit_kernel,@function
elementwise_unaryx_logit_kernel:        ; @elementwise_unaryx_logit_kernel
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
.LBB7_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_logit_kernel
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
.Lfunc_end7:
	.size	elementwise_unaryx_logit_kernel, .Lfunc_end7-elementwise_unaryx_logit_kernel
                                        ; -- End function
	.set elementwise_unaryx_logit_kernel.num_vgpr, 22
	.set elementwise_unaryx_logit_kernel.num_agpr, 0
	.set elementwise_unaryx_logit_kernel.numbered_sgpr, 8
	.set elementwise_unaryx_logit_kernel.num_named_barrier, 0
	.set elementwise_unaryx_logit_kernel.private_seg_size, 0
	.set elementwise_unaryx_logit_kernel.uses_vcc, 1
	.set elementwise_unaryx_logit_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_logit_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_logit_kernel.has_recursion, 0
	.set elementwise_unaryx_logit_kernel.has_indirect_call, 0
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
	.protected	elementwise_unaryx_expit_kernel ; -- Begin function elementwise_unaryx_expit_kernel
	.globl	elementwise_unaryx_expit_kernel
	.p2align	8
	.type	elementwise_unaryx_expit_kernel,@function
elementwise_unaryx_expit_kernel:        ; @elementwise_unaryx_expit_kernel
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
	s_cbranch_execz .LBB8_2
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
.LBB8_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_expit_kernel
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
.Lfunc_end8:
	.size	elementwise_unaryx_expit_kernel, .Lfunc_end8-elementwise_unaryx_expit_kernel
                                        ; -- End function
	.set elementwise_unaryx_expit_kernel.num_vgpr, 14
	.set elementwise_unaryx_expit_kernel.num_agpr, 0
	.set elementwise_unaryx_expit_kernel.numbered_sgpr, 6
	.set elementwise_unaryx_expit_kernel.num_named_barrier, 0
	.set elementwise_unaryx_expit_kernel.private_seg_size, 0
	.set elementwise_unaryx_expit_kernel.uses_vcc, 1
	.set elementwise_unaryx_expit_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_expit_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_expit_kernel.has_recursion, 0
	.set elementwise_unaryx_expit_kernel.has_indirect_call, 0
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
	.protected	elementwise_unaryx_positive_kernel ; -- Begin function elementwise_unaryx_positive_kernel
	.globl	elementwise_unaryx_positive_kernel
	.p2align	8
	.type	elementwise_unaryx_positive_kernel,@function
elementwise_unaryx_positive_kernel:     ; @elementwise_unaryx_positive_kernel
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
	s_cbranch_execz .LBB9_2
; %bb.1:
	s_load_b128 s[0:3], s[0:1], 0x0
	v_ashrrev_i32_e32 v2, 31, v1
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_lshlrev_b64 v[0:1], 3, v[1:2]
	s_waitcnt lgkmcnt(0)
	v_add_co_u32 v2, vcc_lo, s0, v0
	s_delay_alu instid0(VALU_DEP_1) | instskip(SKIP_1) | instid1(VALU_DEP_1)
	v_add_co_ci_u32_e64 v3, null, s1, v1, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	global_store_b64 v[0:1], v[2:3], off
.LBB9_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_positive_kernel
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
.Lfunc_end9:
	.size	elementwise_unaryx_positive_kernel, .Lfunc_end9-elementwise_unaryx_positive_kernel
                                        ; -- End function
	.set elementwise_unaryx_positive_kernel.num_vgpr, 4
	.set elementwise_unaryx_positive_kernel.num_agpr, 0
	.set elementwise_unaryx_positive_kernel.numbered_sgpr, 5
	.set elementwise_unaryx_positive_kernel.num_named_barrier, 0
	.set elementwise_unaryx_positive_kernel.private_seg_size, 0
	.set elementwise_unaryx_positive_kernel.uses_vcc, 1
	.set elementwise_unaryx_positive_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_positive_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_positive_kernel.has_recursion, 0
	.set elementwise_unaryx_positive_kernel.has_indirect_call, 0
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
	.protected	elementwise_unaryx_exp10_kernel ; -- Begin function elementwise_unaryx_exp10_kernel
	.globl	elementwise_unaryx_exp10_kernel
	.p2align	8
	.type	elementwise_unaryx_exp10_kernel,@function
elementwise_unaryx_exp10_kernel:        ; @elementwise_unaryx_exp10_kernel
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
	s_cbranch_execz .LBB10_2
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
	s_mov_b32 s0, 0x979a371
	s_mov_b32 s1, 0x400a934f
	global_load_b64 v[2:3], v[2:3], off
	s_waitcnt vmcnt(0)
	v_mul_f64 v[4:5], v[2:3], s[0:1]
	s_mov_b32 s0, 0x509f79ff
	s_mov_b32 s1, 0xbfd34413
	v_cmp_nlt_f64_e32 vcc_lo, 0x40900000, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_rndne_f64_e32 v[4:5], v[4:5]
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[2:3]
	s_mov_b32 s0, 0xa994fd21
	s_mov_b32 s1, 0x3c49dc1d
	v_cvt_i32_f64_e32 v10, v[4:5]
	s_delay_alu instid0(VALU_DEP_2)
	v_fma_f64 v[6:7], v[4:5], s[0:1], v[6:7]
	s_mov_b32 s0, 0x494ea3e9
	s_mov_b32 s1, 0xbcaf48ad
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_mul_f64 v[8:9], v[6:7], s[0:1]
	s_mov_b32 s0, 0xbbb55516
	s_mov_b32 s1, 0x40026bb1
	s_delay_alu instid0(VALU_DEP_1) | instid1(SALU_CYCLE_1)
	v_fma_f64 v[6:7], v[6:7], s[0:1], v[8:9]
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
	v_cmp_ngt_f64_e64 s0, 0xc090cc00, v[2:3]
	s_delay_alu instid0(VALU_DEP_2) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_fma_f64 v[8:9], v[6:7], v[8:9], 1.0
	v_fma_f64 v[4:5], v[6:7], v[8:9], 1.0
	s_delay_alu instid0(VALU_DEP_1) | instskip(NEXT) | instid1(VALU_DEP_1)
	v_ldexp_f64 v[4:5], v[4:5], v10
	v_cndmask_b32_e32 v5, 0x7ff00000, v5, vcc_lo
	s_and_b32 vcc_lo, s0, vcc_lo
	s_delay_alu instid0(VALU_DEP_2) | instskip(SKIP_1) | instid1(VALU_DEP_3)
	v_cndmask_b32_e32 v2, 0, v4, vcc_lo
	v_add_co_u32 v0, vcc_lo, s2, v0
	v_cndmask_b32_e64 v3, 0, v5, s0
	v_add_co_ci_u32_e64 v1, null, s3, v1, vcc_lo
	global_store_b64 v[0:1], v[2:3], off
.LBB10_2:
	s_endpgm
	.section	.rodata,"a",@progbits
	.p2align	6, 0x0
	.amdhsa_kernel elementwise_unaryx_exp10_kernel
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
		.amdhsa_next_free_vgpr 11
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
.Lfunc_end10:
	.size	elementwise_unaryx_exp10_kernel, .Lfunc_end10-elementwise_unaryx_exp10_kernel
                                        ; -- End function
	.set elementwise_unaryx_exp10_kernel.num_vgpr, 11
	.set elementwise_unaryx_exp10_kernel.num_agpr, 0
	.set elementwise_unaryx_exp10_kernel.numbered_sgpr, 6
	.set elementwise_unaryx_exp10_kernel.num_named_barrier, 0
	.set elementwise_unaryx_exp10_kernel.private_seg_size, 0
	.set elementwise_unaryx_exp10_kernel.uses_vcc, 1
	.set elementwise_unaryx_exp10_kernel.uses_flat_scratch, 0
	.set elementwise_unaryx_exp10_kernel.has_dyn_sized_stack, 0
	.set elementwise_unaryx_exp10_kernel.has_recursion, 0
	.set elementwise_unaryx_exp10_kernel.has_indirect_call, 0
	.section	.AMDGPU.csdata,"",@progbits
; Kernel info:
; codeLenInByte = 636
; TotalNumSgprs: 8
; NumVgprs: 11
; ScratchSize: 0
; MemoryBound: 0
; FloatMode: 240
; IeeeMode: 1
; LDSByteSize: 0 bytes/workgroup (compile time only)
; SGPRBlocks: 0
; VGPRBlocks: 1
; NumSGPRsForWavesPerEU: 8
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
	.p2alignl 7, 3214868480
	.fill 96, 4, 3214868480
	.section	.AMDGPU.gpr_maximums,"",@progbits
	.set amdgpu.max_num_vgpr, 0
	.set amdgpu.max_num_agpr, 0
	.set amdgpu.max_num_sgpr, 0
	.text
	.type	__hip_cuid_29f2f3ccadb756b3,@object ; @__hip_cuid_29f2f3ccadb756b3
	.section	.bss,"aw",@nobits
	.globl	__hip_cuid_29f2f3ccadb756b3
__hip_cuid_29f2f3ccadb756b3:
	.byte	0                               ; 0x0
	.size	__hip_cuid_29f2f3ccadb756b3, 1

	.ident	"AMD clang version 22.0.0git (/srcdest/rocm-llvm f58b06dce1f9c15707c5f808fd002e18c2accf7e)"
	.section	".note.GNU-stack","",@progbits
	.addrsig
	.addrsig_sym __hip_cuid_29f2f3ccadb756b3
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
    .name:           elementwise_unaryx_sinc_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     12
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_sinc_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     35
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
    .name:           elementwise_unaryx_frac_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_frac_kernel.kd
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
    .name:           elementwise_unaryx_heaviside_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_heaviside_kernel.kd
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
    .name:           elementwise_unaryx_signbit_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_signbit_kernel.kd
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
    .name:           elementwise_unaryx_isnan_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_isnan_kernel.kd
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
    .name:           elementwise_unaryx_isinf_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_isinf_kernel.kd
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
    .name:           elementwise_unaryx_isfinite_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_isfinite_kernel.kd
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
    .name:           elementwise_unaryx_logit_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     10
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_logit_kernel.kd
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
    .name:           elementwise_unaryx_expit_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_expit_kernel.kd
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
    .name:           elementwise_unaryx_positive_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     7
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_positive_kernel.kd
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
    .name:           elementwise_unaryx_exp10_kernel
    .private_segment_fixed_size: 0
    .sgpr_count:     8
    .sgpr_spill_count: 0
    .symbol:         elementwise_unaryx_exp10_kernel.kd
    .uniform_work_group_size: 1
    .uses_dynamic_stack: false
    .vgpr_count:     11
    .vgpr_spill_count: 0
    .wavefront_size: 32
    .workgroup_processor_mode: 1
amdhsa.target:   amdgcn-amd-amdhsa--gfx1101
amdhsa.version:
  - 1
  - 2
...

	.end_amdgpu_metadata
