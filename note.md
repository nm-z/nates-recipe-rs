




























4. loads per FMA ratio (current)
5. loads per FMA ratio (tiled)
6. math utilization % (yours vs rocBLAS)
7. total register file across GPU
8. effective register file usage across GPU


1 VGPR	= 1 value / thread
1 SGPR	= 1 value / wavefront
1 Thread	= max 256	VGPRs
1 Wavefront	= max 128	SGPRs
1 Wavefront	= 32		threads
1 f64		= 2		VGPRs
1 SIMD	= 1,536	VGPRs
1 CU		= SIMDs,ALUs,L0/L1,LDS,SQC/SQ,scheduler


GPR		physical slots
	VGPR	vector general purpose register	(48)
	SGPR	scalar general purpose register	(86)
wavefront	32 threads
VRAM		Virtual Random Access Memory		(GB)
LDS		Local Data Share
SSE		Streaming SIMD Extensions
SIMD		Single Instruction Multiple Data
ISA		instruction set architecture
FMA		fused multiply-add			(a*b+c)
ALU		arithmetic logic unit
AVX		Advanced Vector Extensions
SW:
	HIP	Het.compute Interface for Portability
	ROCr	ROCm Runtime
	HSA	Heterogeneous System Architecture
	KFD	Kernel Fusion Driver
	AQL	Architected Queuing Language
SQ		Sequencer
SQC		Sequencer Cache
CUs		Compute Units





rust
	HIP
		ROCr
			HSA
				KFD
					CUs
DISK
	.s
		RAM
			CPU (llvm)
				RAM
					.co
						DISK
DISK
	.co
		RAM
			VRAM
				SQC
					SQ
						ALU
DISK
	.csv
		RAM
			VRAM
				L2
					L1/L0
						GPR
							ALU




Vendor:
	nvidia
		tool		cuda
		compiler	nvcc
		runtime	cudart
		blas		cublas
		dialect	cuda
	amd
		tool		rocm
		compiler	hipcc
		runtime	hip
		blas		rocblas
		dialect	hip
	moore_threads
		tool		musa
		compiler	mcc
		runtime	musa_runtime
		blas		mublas
		dialect	musa
	khronos
		spec		sycl
	uxl_foundation
		spec		oneapi
		builds_on	sycl
		donated_by	intel
	intel
		tool		oneapi_toolkit
		implements	oneapi
		compiler	icpx
		runtime	level_zero
		blas		onemkl
		dialect	sycl





















qwen3_0_6b
	downloads Qwen3 0.6B safetensors
	converts
		f16->bf16
	quantizes bf16
		q8_0 .. q6_k (10 formats)
	all 12 variants
		real text generation
		wikitext-2 perplexity
			gate	check_ppl
			ppl blowout	fails the run
			proves	quantization didn't break the model
	imatrix generation
	kv-cache save/load-state
		flash-attention/offload configs	4
ctest_with_model debug/release
	reruns ctest
	label filter	-L model
	targets		the q4_0 just built
	your failure	missing time binary
hardware_matrix
	same script every runner
	env flags gate backends
		GG_BUILD_CUDA
		GG_BUILD_ROCM
		GG_BUILD_VULKAN
		GG_BUILD_SYCL
		GG_BUILD_METAL
		GG_BUILD_MUSA
		GG_BUILD_WEBGPU
		...
	one flag per self-hosted runner
outputs
	tmp/results
		per-stage logs
		README.md summary
	tmp/mnt	model download cache
philosophy
	unit tests	cheap gate up front
	real verdict
		clone real model from HF
		convert quantize generate perplexity
		hold numbers to thresholds
	your cookbook-plus-suite	same conviction
		e2e of real binary	the claim
		unit green	supplementary























































































.
