




























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





Tokenizers (17)
	bert-bge
	command-r
	deepseek-coder
	deepseek-llm
	falcon
	gemma-4
	gpt-2
	llama-bpe
	llama-spm
	mpt
	phi-3
	qwen2
	qwen35
	refact
	starcoder
Inference (8)
	test-download-model
	test-eval-callback-download-model
	test-llama-archs:
		builds all 109 testable archs
		GPU-vs-CPU NMSE
		save/reload roundtrip
		seeding/saving: -s <seed> -a <arch> -o <dir>
	test-thread-safety:
		4 parallel inferences on model
	test-save-load-state:
		generate
		save KV
		restore KV
		verify identical
	test-state-restore-fragmented:
		fragmented KV cache
	test-recurrent-state-rollback
		state rollback recurrent (mamba-style) caches
	test-eval-callback
		runs llama-eval-callback on model
		verifies hooks fire
	test-model-load-cancel
		abort a model load mid-way
	test-autorelease
		load/free lifecycle
Kernels / GPU backend (6)
	test-backend-ops
		every op vs CPU reference
		16595 cases
	test-backend-sampler
		GPU-side =? CPU sampling
	test-rope
	test-col2im-1d
	test-alloc
		graph allocator
	test-barrier
		threadpool barrier
Quantization (2)
	test-quantize-fns
		quantize/dequantize error bounds
	test-quantize-perf
		throughput benchmark
Grammar / structured output
	gbnf parsing and constrained-decoding
		test-grammar-parser
		test-llama-grammar
		test-grammar-integration
	test-json-schema-to-grammar
		JSON schema to GBNF conversion
	test-peg-parser
		their PEG parser implementation
Chat plumbing (6)
	chat template rendering and model output parsing
		test-chat
		test-chat-template
		test-chat-peg-parser
		test-chat-auto-parser
	test-jinja
		minja engine vs reference Jinja2 behavior
Sampling + misc (9)
	test-sampling
		top-k/top-p/temp/mirostat correctness
	test-reasoning-budget
		thinking-token budget enforcement
	test-batch-alloc
		batch memory allocation
	test-gguf
		gguf read/write roundtrip
	test-mtmd-c-api
		multimodal C API surface
	test-log
		logging
	test-arg-parser
		CLI args
	test-double-float
		float conversion
	test-c
		C-linkage compilation



















































































.
