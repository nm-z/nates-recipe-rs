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

llamacpp:
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

YES
  tokenizer set (ggml-vocab + .inp/.out)
  test-llama-archs (seeded ggufs + CPU logits)
NO
  test-backend-ops
  test-backend-sampler
  test-rope
  test-col2im-1d
  test-alloc
  test-barrier
  test-gguf
  test-quantize-fns / test-quantize-perf
  save/load state, fragmented restore, recurrent rollback
  thread-safety, load-cancel, autorelease, eval-callback, downloads
  grammar / GBNF / JSON-schema / PEG
  chat / templates / jinja
  sampling, reasoning-budget, batch-alloc, mtmd, log, arg-parser, double-float, test-c




a polymath holds knowledge in one head
	29,000 lines of graph code
	each arch re-implements

encyclopedism holds it in one structure
	any arch is expressible
	anyone can compose references

bat recipe-infer/src/models/common.rs recipe-infer/src/models/mod.rs recipe-infer/src/llm.rs








llm.rs says supported_archs() returns architectures with a verified decode composition
mod.rs says SUPPORTED is wired into dispatch
But the stated audit status is 134 declared, 24 execute, 0 parity. That makes this table a declaration ledger, not a verified support list. Calling all 134 supported is misleading and will let Headless::open accept models that are known not to work.































































.
