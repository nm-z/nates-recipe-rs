---
name: gfx1101-roofline-ceilings
description: RX 7700 XT (gfx1101) f64 + bandwidth peaks for roofline analysis of profiling DBs
metadata: 
  node_type: memory
  type: reference
  originSessionId: 348dc130-5104-43d2-aaaf-ea9177c58526
---

User's GPU is **AMD Radeon RX 7700 XT, gfx1101 (Navi 32, RDNA3), 54 CUs**, confirmed from rocpd_info_agent extdata (cu_count:54, 192-bit, 12GB GDDR6, f-compute clk 2226MHz, 48MB Infinity Cache, wavefront 32, max_waves_per_simd 16).

Roofline ceilings:
- **FP64 peak = 1.099 TFLOP/s** (= 1099 GFLOP/s). RDNA3 f64 = 1/16 the base ALU FMA rate, ≈3% of dual-issue f32 (~35 TFLOP). f64 is a HARD floor — never propose making an f64 GEMM "faster" past this; only lever is less/overlap f64. See [[project_f64_required]].
- **Memory BW = 432 GB/s.** Ridge point ≈ 2.54 FLOP/byte. f64 reductions (~0.125 FLOP/byte) are firmly bandwidth-bound → judge against 432 GB/s.
- Sibling part RX 7800 XT (also gfx1101, 60 CU) = 1.166 TFLOP/s f64, 624 GB/s — confirm which via cu_count before using.

Profiling DBs live in engi/<pid>_results.db (rocprofv3 schema: real data in rocpd_*_<uuid> tables, query via the unsuffixed views; top_kernels/top summary views exist). Get grid+workgroup from rocpd_kernel_dispatch: workgroups = (grid_size_x/workgroup_size_x)*grid_size_y.
