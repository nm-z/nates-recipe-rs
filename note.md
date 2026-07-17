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




FAIL
PASS

testing at temperature=0


llamacpp supports:
	134 architectures
	tokenizers









Architecture (134)
	Default:
		token embeddings
		repeated blocks
		final norm
		LM head -> logits
	Quant:
		weights
			ternary
		activations
			8-bit
	Block kind
		Decoder:
			dense
			MoE
			recurrent
			hybrid attention/scan
			multimodal decoder
			diffusion decoder
		Encoder:
			dense
			MoE
			multimodal encoder
		Encoder-decoder:
			text-to-text
			OCR / speech / audio
	Position:
		learned absolute positional embeddings
		ALiBi
		RoPE
		rotary dimension
		partial rotary
		2D positional encoding
		2D RoPE
		relative position bias
	Sequence mixer
		attention
			type:
				self-attn
					causal
					bidirectional
				sliding-window self-attn
				global self-attn
				linear-attn
			config:
				MHA
				GQA
				MQA
				MLA
				attention bias
				attention scaling
				Q/K norm
				attention sinks
		scan
			RWKV-style recurrent
			Mamba / SSM
			delta-net
			gated DeltaNet
			short convolution
			convolutional decoder path
	FFN / MLP
		SwiGLU
		GeGLU
		GELU sequential
		ReLU-squared sequential
		BitLinear
		shared expert MLP
		routed expert MLP
		channel-mixing MLP
		SSM gated projection
	Norm
		type
			RMSNorm
			LayerNorm
			non-parametric LayerNorm
		placement
			pre-attn norm
			post-attn norm
			pre-ffn norm
			post-ffn norm
			sandwich norm
			Q/K norm
			zero-centered norm
			reordered norm path
	Routing
		none
		top-k MoE
		shared experts
		routed experts
		routed dense/MoE hybrid layers
		fine-grained expert routing
		dropless token routing
		high-sparsity MoE
	Multimodal
		vision encoder
		visual expert path
		image encoder
		audio encoder
		OCR vision encoder
		vision transformer encoder
		SigLIP vision encoder
		language decoder path
		text decoder path
		image/text token stream
		discrete image tokens
		2D vision position path
		pan-and-scan image path
		early-fusion image/text path
		mobile-optimized multimodal stack
	Head
		embedding pooling head
		speculative decoding draft head
		instruction-tuned assistant path
	Audio
		codec-token decoder path
		speech/audio token generation
		waveform token decoder
		codec reconstruction path
	Training objective
		autoregressive next-token prediction
		fill-in-the-middle capable
		autoregressive blank infilling
		next-token prediction over mixed stream











afmoe
	MoE
	dense leading FFN fallback
	causal
	GQA
	sliding-window attention optional
	full attention layers optional
	sliding/full attention pattern optional
	RoPE optional
	NoPE layers optional
	rotary dimension
	per-layer RoPE frequency
	pre-RoPE Q/K norm
	attention scaling
	sigmoid attention gate
	post-attention output projection
	RMSNorm
	pre-attn norm
	post-attn norm
	pre-ffn norm
	post-ffn norm
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP
	shared expert MLP optional
	top-k MoE
	routed experts
	shared experts optional
	router logits
	router bias
	sigmoid expert gating default
	expert weight normalization optional
	expert weight scaling optional
	input embedding scaling
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
apertus
	dense
	decoder-only
	transformer
	causal
	RoPE
	LongRoPE optional
	GQA
	Q/K norm
	xIELU sequential FFN
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	sandwich norm
	optional attention output bias
	residual connections
	token embeddings
	final RMSNorm
	LM head -> logits
arcee
	dense
	causal decoder-only LM
	token embeddings
	RoPE
	GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	optional attention output bias
	ReLU-squared sequential FFN
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
arctic
	MoE
	dense FFN path
	causal
	GQA-capable
	RoPE
	rotary dimension
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	expert pre-ffn norm
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	softmax expert gating
	expert weight normalization
	expert weight scaling
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
arwkv7
	recurrent
	RWKV-style recurrent
	RWKV7 time-mixing
	linear-attn / WKV recurrence
	recurrent state
	token shift
	time-mix LoRA decay path
	time-mix LoRA in-context learning-rate path
	value residual mix
	optional time-mix gate
	receptance/key/value projections
	RMSNorm
	pre-time-mix RMSNorm
	pre-ffn RMSNorm
	SwiGLU-style gated FFN
	channel-mixing MLP
	residual connections
	token embeddings
	final RMSNorm
	LM head -> logits
baichuan
	dense
	causal decoder-only LM
	token embeddings
	RoPE for 7B
	ALiBi for 13B
	MHA / GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	residual connections
	final RMSNorm
	untied LM head
	LM head -> logits
bailingmoe
	MoE
	causal
	GQA-capable
	RoPE
	rotary dimension
	RoPE factors optional
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated expert FFN
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	softmax expert gating
	expert weight normalization optional
	expert weight scaling optional
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
bailingmoe2
	MoE
	dense leading FFN fallback
	causal
	GQA-capable
	fused QKV
	RoPE
	rotary dimension
	pre-RoPE Q/K norm
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	router bias optional
	expert gating function
	expert weight normalization optional
	expert weight scaling optional
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
	NextN / MTP tensors loaded but skipped
bert
	dense
	encoder-only
	transformer
	bidirectional
	learned absolute positional embeddings
	token embeddings
	token-type embeddings
	embedding LayerNorm
	MHA
	GELU sequential FFN
	LayerNorm
	post-attn LayerNorm
	post-ffn LayerNorm
	residual connections
	sequence embedding output
bitnet
	ternary / BitNet quantized weights
	dense
	decoder-only
	transformer
	causal
	RoPE
	GQA
	BitLinear-style scaled linear tensors
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	attention sub-layer RMSNorm
	pre-ffn RMSNorm
	FFN sub-layer RMSNorm
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings via token embeddings
	LM head -> logits
bloom
	dense
	causal
	GQA-capable
	ALiBi
	fused QKV
	QKV bias
	attention output bias
	attention scaling
	LayerNorm
	token embedding LayerNorm
	pre-attn norm
	pre-ffn norm
	GELU sequential FFN
	FFN bias
	residual connections
	token embeddings
	optional tied LM head
	final LayerNorm
	embedding output
	LM head -> logits
chameleon
	multimodal decoder
	decoder-only
	transformer
	causal
	RoPE
	GQA
	Q/K norm
	SwiGLU
	RMSNorm
	pre-attn RMSNorm or post-attn RMSNorm via swin_norm
	pre-ffn RMSNorm or post-ffn RMSNorm via swin_norm
	discrete image tokens
	image/text token stream
	next-token prediction over mixed stream architecturally
	image-token logits suppressed in llama.cpp
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
chatglm
	dense
	causal decoder-only LM
	token embeddings
	RoPE
	MQA / GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SwiGLU sequential FFN
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
codeshell
	dense
	causal
	GQA-capable
	RoPE
	rotary dimension
	attention output bias
	attention scaling
	LayerNorm
	pre-attn norm
	pre-ffn norm
	GELU sequential FFN
	FFN bias
	residual connections
	token embeddings optional
	output embeddings fallback
	final LayerNorm
	embedding output
	LM head -> logits
cogvlm
	multimodal vision-language model
	causal language decoder
	token embeddings
	image embedding input path
	RoPE
	MHA / GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	visual expert attention path
	visual expert FFN path
	language attention path
	language FFN path
	vision encoder on multimodal side
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
cohere2
	dense
	causal
	GQA
	sliding-window attention
	full attention layers
	sliding/full attention pattern
	RoPE on sliding-window layers
	rotary dimension
	RoPE factors optional
	attention scaling
	LayerNorm-style norm
	pre-attn norm
	SiLU parallel gated FFN
	parallel attention + FFN block
	residual connections
	token embeddings
	tied input/output embeddings
	final norm
	embedding output
	LM head -> logits
	logit scaling
cohere2moe
	MoE
	decoder-only
	transformer
	causal
	RoPE conditional
	sliding-window self-attn
	global self-attn
	standard SWA
	SWA pattern
	dense-prefix full-attn layers
	GQA
	SwiGLU
	LayerNorm or RMSNorm
	pre-attn norm
	final norm
	leading dense FFN layers
	routed expert MLP
	shared expert MLP optional
	top-k MoE
	sigmoid expert gating default
	expert weight normalization optional
	expert weight scaling optional
	shared experts optional
	routed experts
	NextN/MTP block optional
	token embeddings
	tied input/output embeddings fallback
	logit scaling
	LM head -> logits
command-r
	dense
	causal decoder-only LM
	token embeddings
	RoPE
	GQA depending on KV-head metadata
	optional Q/K LayerNorm for larger variants
	LayerNorm
	pre-attn LayerNorm
	parallel attention/FFN residual path
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	tied LM head
	optional logit scaling
	LM head -> logits
dbrx
	MoE
	decoder-only
	transformer
	causal
	RoPE
	GQA
	SwiGLU
	LayerNorm
	pre-attn LayerNorm
	post-attn LayerNorm
	routed expert MLP
	top-k MoE
	softmax expert gating
	normalized expert weights
	routed experts
	residual connections
	token embeddings
	final LayerNorm
	LM head -> logits
deci
	dense
	causal decoder-only LM
	token embeddings
	RoPE
	optional LongRoPE factors
	GQA depending on KV-head metadata
	linear-attention layers optional
	attention-free layers optional
	FFN-free layers optional
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	attention output bias optional
	SiLU parallel gated FFN
	SwiGLU-style MLP
	FFN biases optional
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
deepseek
	MoE
	decoder-only
	transformer
	causal
	RoPE
	GQA
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	leading dense FFN layers
	routed expert MLP
	shared expert MLP
	top-k MoE
	softmax expert gating
	expert weight scaling optional
	shared experts
	routed experts
	dense/MoE hybrid FFN stack
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
deepseek2
	MoE
	causal decoder-only LM
	token embeddings
	MLA-style attention
	Q LoRA path optional
	KV LoRA path
	compressed KV path
	Q split into non-RoPE and RoPE parts
	K split into non-RoPE and RoPE parts
	RoPE
	YaRN-adjusted attention scaling
	attention scaling
	RMSNorm
	pre-attn RMSNorm
	Q-LoRA RMSNorm optional
	KV-LoRA RMSNorm
	pre-ffn RMSNorm
	leading dense FFN blocks
	MoE blocks after dense lead
	SiLU parallel gated dense FFN
	SwiGLU-style dense MLP
	SiLU routed expert FFN
	SwiGLU-style expert MLP
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	softmax expert gating by default
	expert weight scaling optional
	expert weight norm optional
	expert probability bias optional
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
deepseek2-ocr
	MoE
	dense leading FFN fallback
	causal
	MHA
	separate Q/K/V projections
	RoPE
	NeoX RoPE mode
	full-head rotary
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	router bias optional
	softmax expert gating default
	expert weight normalization optional
	expert weight scaling optional
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
deepseek32
	MoE
	decoder-only
	transformer
	causal
	MLA
	DSA
	lightning indexer
	RoPE
	YaRN RoPE scaling
	NoPE Q/K sub-dim
	RoPE Q/K sub-dim
	MLA absorption optimization
	MLA converts to MQA-style attention path
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	Q LoRA RMSNorm
	KV latent RMSNorm
	routed expert MLP
	shared expert MLP
	top-k MoE
	expert probability bias optional
	expert weight normalization optional
	expert weight scaling optional
	shared experts
	routed experts
	leading dense FFN layers
	MoE FFN layers
	dense/MoE hybrid FFN stack
	NextN/MTP tensors preserved but skipped
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
deepseek4
	MoE
	causal decoder-only LM
	token embeddings
	MLA-style attention
	Q LoRA path
	compressed KV path
	Q split into non-RoPE and RoPE parts
	KV split into non-RoPE and RoPE parts
	RoPE
	compressed-RoPE path
	sliding-window attention
	attention sinks
	DSV4 compressed attention
	CSA compressed attention path
	HCA compressed attention path
	LID top-k indexer
	output LoRA projection
	hyper-connection path
	RMSNorm
	pre-attn RMSNorm
	Q-LoRA RMSNorm
	Q RMSNorm
	KV RMSNorm
	pre-ffn RMSNorm
	SiLU routed expert FFN
	SwiGLU-style expert MLP
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	sqrt-softplus expert gating
	expert weight scaling
	expert weight norm
	expert probability bias
	hash-layer token-to-expert routing
	residual / hyper-connection mixing
	final RMSNorm
	untied LM head
	LM head -> logits
dflash
	diffusion draft model
	encoder feature-fusion graph
	target hidden-state feature input
	target layer fusion
	fusion projection
	h_nextn output
	decoder
	noise-block diffusion token path
	KV cache injection path
	cache-aware non-causal attention
	GQA-capable
	separate Q/K/V projections
	RoPE
	rotary dimension
	sliding-window attention optional
	sliding/full attention pattern optional
	Q/K norm
	attention scaling
	RMSNorm
	encoder output RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	target token embeddings fallback
	target LM head fallback
	final RMSNorm
	embedding output
	LM head -> logits
dots1
	MoE
	decoder-only
	transformer
	causal
	RoPE
	MHA
	Q/K norm
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	leading dense FFN layers
	routed expert MLP
	shared expert MLP
	top-k MoE
	expert probability bias optional
	expert weight normalization optional
	expert weight scaling optional
	residual connections
	token embeddings
	final RMSNorm
	LM head -> logits
dream
	masked diffusion language model
	non-causal decoder-style transformer
	token embeddings
	RoPE
	bidirectional / non-causal self-attention
	no KV cache
	GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	residual connections
	final RMSNorm
	optional tied LM head
	optional LM head bias
	LM head -> logits
eagle3
	speculative decoding draft model
	encoder feature-fusion graph
	target hidden-state feature input
	3 target layer fusion
	fusion projection
	h_nextn output
	single-layer draft decoder
	causal
	GQA-capable
	separate Q/K/V projections
	RoPE
	rotary dimension
	RoPE factors optional
	attention scaling
	RMSNorm
	token embedding norm
	target-feature norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings optional
	target token embeddings fallback
	output projection optional
	target LM head fallback
	draft-to-target vocab mapping optional
	LM head -> logits
ernie4_5
	dense
	decoder-only
	transformer
	causal
	RoPE
	GQA
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	optional attention output bias
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
ernie4_5-moe
	MoE
	causal decoder-only LM
	token embeddings
	RoPE
	GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	interleaved dense FFN and MoE layers
	leading dense blocks optional
	SiLU parallel gated dense FFN
	SwiGLU-style dense MLP
	SiLU routed expert FFN
	SwiGLU-style expert MLP
	routed expert MLP
	optional shared expert MLP
	top-k MoE
	routed experts
	optional shared experts
	router logits
	softmax expert gating
	expert weight scaling
	expert probability bias optional
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
eurobert
	encoder
	dense
	bidirectional
	no KV cache
	GQA-capable
	RoPE
	rotary dimension
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	no LM logits path
exaone
	dense
	decoder-only
	transformer
	causal
	RoPE
	GQA
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
exaone4
	dense
	causal decoder-only LM
	token embeddings
	RoPE
	RoPE only on SWA layers for 32B
	sliding-window self-attention
	global/full self-attention layers
	GQA depending on KV-head metadata
	pre-RoPE Q/K RMSNorm
	RMSNorm
	post-attn RMSNorm
	post-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	reordered norm path
	no pre-attn residual-stream norm
	no pre-ffn residual-stream norm
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
	NextN / MTP tensors preserved but skipped
exaone-moe
	MoE
	dense leading FFN fallback
	causal
	GQA
	sliding-window attention
	full attention layers
	sliding/full attention pattern
	RoPE on sliding-window layers
	rotary dimension
	RoPE factors optional
	pre-RoPE Q/K norm
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	router bias optional
	expert gating function
	expert weight normalization optional
	expert weight scaling optional
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
	NextN / MTP tensors loaded but skipped
falcon
	dense
	decoder-only
	transformer
	causal
	RoPE
	NeoX-style RoPE
	MQA / GQA depending on n_head_kv
	GELU sequential FFN
	LayerNorm
	LayerNorm bias
	parallel attention + FFN residual pattern
	optional second attention LayerNorm for Falcon-40B
	token embeddings
	final LayerNorm
	tied input/output embeddings fallback
	LM head -> logits
falcon-h1
	hybrid attention/SSM
	causal decoder-only LM
	token embeddings
	parallel self-attention and Mamba2 layer
	RoPE
	GQA depending on KV-head metadata
	attention scaling
	Mamba2 / SSM
	SSM input projection
	SSM conv1d
	SSM recurrent state
	SSM A/D state parameters
	SSM time-step bias
	SSM grouped state
	SSM RMSNorm optional
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	SiLU parallel gated FFN
	SwiGLU-style MLP
	FFN biases optional
	attention output bias optional
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
gemma
	dense
	causal
	GQA-capable
	RoPE
	rotary dimension
	Q-scaled attention
	RMSNorm
	pre-attn norm
	pre-ffn norm
	GELU parallel gated FFN
	residual connections
	token embeddings
	input embedding scaling
	tied input/output embeddings
	final RMSNorm
	embedding output
	LM head -> logits
gemma2
	dense
	RoPE
	causal
	sliding-window self-attn
	global self-attn
	GQA
	attention scaling
	GeGLU
	RMSNorm
	pre-attn norm
	post-attn norm
	pre-ffn norm
	post-ffn norm
gemma3
	dense
	multimodal conditional generation
	decoder-only
	transformer
	causal
	RoPE
	sliding-window self-attn optional
	global self-attn
	standard SWA
	SWA RoPE freq base optional
	GQA
	Q/K norm
	GELU gated FFN
	RMSNorm
	pre-attn RMSNorm
	post-attn RMSNorm
	pre-ffn RMSNorm
	post-ffn RMSNorm
	residual connections
	token embeddings
	encoded image embeddings accepted in decoder input
	SigLIP vision encoder
	Gemma3 vision projector
	final RMSNorm
	tied input/output embeddings fallback
	final logit softcapping optional
	LM head -> logits
gemma3n
	mobile-optimized multimodal language model
	causal decoder-only LM
	token embeddings
	input embedding scaling
	per-layer token embeddings
	AltUp projections
	LAUREL residual path
	RoPE
	sliding-window attention pattern
	global/full attention layers via SWA pattern
	GQA depending on KV-head metadata
	KV sharing / KV reuse after early layers
	pre-RoPE Q/K RMSNorm
	V RMSNorm
	RMSNorm
	pre-attn RMSNorm
	post-attn RMSNorm
	pre-ffn RMSNorm
	post-ffn RMSNorm
	attention scaling
	parallel GELU gated FFN
	GeGLU-like MLP
	activation sparsity in early layers
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
	final logit soft-capping
	multimodal embedding input path
	image encoder on multimodal side
	audio encoder on multimodal side
gemma4
	dense / MoE-capable
	multimodal embedding path
	causal
	GQA
	sliding-window attention
	full attention layers
	sliding/full attention pattern
	shared KV layers optional
	RoPE
	per-layer RoPE frequency
	rotary dimension
	Q/K norm
	V norm
	unit attention scaling
	RMSNorm
	pre-attn norm
	post-attn norm
	pre-ffn norm
	post-ffn norm
	GELU parallel gated FFN
	GELU parallel gated expert FFN optional
	shared expert MLP optional
	routed expert MLP optional
	top-k MoE optional
	routed experts optional
	router logits optional
	softmax expert gating optional
	per-layer token embeddings optional
	per-layer embedding projection optional
	layer output scale optional
	final logit softcapping optional
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	h_nextn output
	embedding output
	LM head -> logits
	logits bias for suppressed tokens optional
gemma4-assistant
	dense
	assistant / NextN draft model
	decoder-side assistant path
	causal
	sliding-window self-attn pattern
	global self-attn pattern
	standard SWA
	RoPE
	SWA RoPE freq base
	Q-only attention projection
	Q norm
	shared KV from other context
	GQA-like attention memory path
	GELU gated FFN
	RMSNorm
	pre-attn RMSNorm
	post-attn RMSNorm
	pre-ffn RMSNorm
	post-ffn RMSNorm
	layer output scaling
	token embeddings
	masked embedding tensors optional
	pre-projection from token embedding + backbone hidden state
	post-projection to next hidden state
	final RMSNorm
	tied input/output embeddings
	LM head -> logits
gemma-embedding
	dense
	embedding model
	Gemma-style transformer
	token embeddings
	input embedding scaling
	RoPE
	symmetric sliding-window attention pattern
	bidirectional / non-causal self-attention
	no KV cache
	GQA depending on KV-head metadata
	pre-RoPE Q/K RMSNorm
	RMSNorm
	pre-attn RMSNorm
	post-attn RMSNorm
	pre-ffn RMSNorm
	post-ffn RMSNorm
	attention scaling
	parallel GELU gated FFN
	GeGLU-like MLP
	residual connections
	final RMSNorm
	embedding output
	optional sentence-transformers dense modules
glm4
	dense
	causal
	GQA
	RoPE
	MRoPE optional
	rope dimension sections
	rotary dimension
	attention scaling
	RMSNorm
	pre-attn norm
	post-attn norm
	pre-ffn norm
	post-ffn norm
	SwiGLU sequential FFN
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	NextN / MTP tensors loaded but skipped
glm4moe
	MoE
	decoder-only
	transformer
	causal
	RoPE
	MRoPE optional
	rope dimension sections
	GQA
	Q/K norm optional
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	post-attn RMSNorm
	shared expert MLP
	routed expert MLP
	top-k MoE
	expert probability bias
	expert weight normalization optional
	expert weight scaling optional
	sigmoid expert gating default
	shared experts
	routed experts
	leading dense FFN layers
	MoE FFN layers
	dense/MoE hybrid FFN stack
	NextN/MTP tensors preserved but skipped
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
glm-dsa
	MoE
	causal decoder-only LM
	token embeddings
	MLA-style attention
	Q LoRA path
	KV LoRA path
	compressed KV path
	Q split into non-RoPE and RoPE parts
	K split into non-RoPE and RoPE parts
	RoPE
	rope dimension sections
	DSA indexer
	indexer top-k selection
	Q-LoRA RMSNorm
	KV-LoRA RMSNorm
	DSA indexer K norm
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU gated dense FFN in leading dense blocks
	SwiGLU-style dense FFN
	SiLU routed expert FFN
	SwiGLU-style expert MLP
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	sigmoid expert gating
	expert weight scaling optional
	expert weight norm optional
	expert probability bias optional
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
	NextN / MTP tensors preserved but skipped
gpt2
	dense
	causal
	MHA
	learned absolute positional embeddings
	token + position embedding sum
	fused QKV
	QKV bias
	attention output bias
	attention scaling
	LayerNorm
	pre-attn norm
	pre-ffn norm
	GELU sequential FFN
	FFN bias
	residual connections
	token embeddings
	optional tied LM head
	final LayerNorm
	embedding output
	LM head -> logits
gptj
	dense
	decoder-only
	transformer
	causal
	RoPE
	rotary dimension
	MHA
	separate Q/K/V projections
	attention output projection
	GELU sequential FFN
	LayerNorm
	pre-block LayerNorm
	final LayerNorm
	token embeddings
	LM head -> logits
gptneox
	dense
	causal
	GQA-capable
	fused QKV
	QKV bias
	attention output bias
	RoPE
	rotary dimension
	partial rotary
	attention scaling
	LayerNorm
	pre-attn norm
	pre-ffn norm
	GELU sequential FFN
	FFN bias
	residual connections
	parallel residual optional
	sequential residual optional
	token embeddings
	final LayerNorm
	embedding output
	LM head -> logits
gpt-oss
	MoE
	causal decoder-only LM
	token embeddings
	RoPE
	layer-dependent RoPE frequency base
	sliding-window self-attention pattern
	GQA
	attention sinks
	attention output bias
	RMSNorm
	pre-attn RMSNorm
	post-attn RMSNorm
	attention scaling
	OpenAI MoE SwiGLU
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	softmax-weight expert gating
	expert weight scaling
	expert FFN biases
	residual connections
	final RMSNorm
	untied LM head
	LM head -> logits
	MXFP4 expert weights
granite
	dense or MoE
	decoder-only
	transformer
	causal
	RoPE conditional
	LongRoPE factors optional
	GQA
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	optional attention output bias
	optional MLP bias
	optional embedding scale
	optional residual scale
	logit scaling
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
granitehybrid
	hybrid attention/scan
	MoE optional
	dense FFN fallback
	causal
	GQA-capable attention layers
	RoPE optional
	rotary dimension
	rope factors optional
	Mamba2 / SSM scan
	recurrent layers
	SSM conv1d
	SSM conv1d bias optional
	SSM A parameter
	SSM D skip parameter
	SSM dt bias
	SSM norm
	attention output bias optional
	attention scaling
	RMSNorm
	pre-mixer norm
	pre-ffn norm
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP optional
	top-k MoE optional
	routed experts optional
	router logits optional
	softmax expert gating optional
	expert weight normalization
	expert weight scaling
	shared expert MLP optional
	shared experts optional
	residual connections
	residual scaling optional
	embedding scaling optional
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	logit scaling optional
granitemoe
	MoE
	causal decoder-only LM
	token embeddings
	RoPE
	optional LongRoPE factors
	GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	optional attention output bias
	SiLU routed expert FFN
	SwiGLU-style expert MLP
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	softmax expert gating
	expert weight scaling
	optional shared expert MLP
	optional shared experts
	embedding scaling optional
	residual scaling optional
	logit scaling
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
grok
	MoE
	dense FFN optional
	causal
	GQA-capable
	RoPE
	rotary dimension
	YaRN RoPE scaling parameters
	RMSNorm
	pre-attn norm
	post-attn norm
	pre-ffn norm
	post-ffn norm
	GELU parallel expert FFN
	GELU parallel dense FFN optional
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	softmax expert gating
	expert weight normalization
	expert weight scaling
	attention output scaling
	embedding scaling
	logit scaling
	final logit softcapping optional
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
grovemoe
	MoE
	causal decoder-only LM
	token embeddings
	RoPE
	full-head rotary
	GQA depending on KV-head metadata
	pre-RoPE Q/K RMSNorm
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU routed expert FFN
	SwiGLU-style expert MLP
	routed expert MLP
	chunk expert MoE branch
	top-k MoE
	routed experts
	chunk/group experts
	router logits
	softmax expert gating
	expert weight scaling
	expert group scaling
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
hunyuan-dense
	dense
	causal
	GQA
	RoPE
	MRoPE optional
	rope dimension sections
	XDRoPE / NTK-aware RoPE scaling
	post-RoPE Q/K norm
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
hunyuan-moe
	MoE
	decoder-only
	transformer
	causal
	RoPE
	GQA
	Q/K norm
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	shared expert MLP
	routed expert MLP
	top-k MoE
	normalized top-k expert weights
	softmax expert gating
	shared experts
	routed experts
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
hunyuan-vl
	multimodal vision-language model
	vision encoder
	ViT-style vision tower
	vision position embeddings
	perceiver / PatchMerger projector
	projected vision tokens
	image begin/end tokens
	causal language decoder
	token embeddings
	RoPE
	M-RoPE / XD-RoPE
	rope dimension sections
	GQA depending on KV-head metadata
	post-RoPE Q/K RMSNorm
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
hy_v3
	MoE
	dense leading FFN fallback
	causal
	GQA
	RoPE
	rotary dimension
	RoPE factors optional
	pre-RoPE Q/K norm
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	router bias
	sigmoid expert gating default
	expert weight normalization optional
	expert weight scaling optional
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	NextN / MTP draft block
	MTP h norm
	MTP e norm
	MTP token embedding
	MTP hidden/input concat
	MTP eh projection
	MTP shared head norm
	MTP shared LM head
	h_nextn output
internlm2
	dense
	decoder-only
	transformer
	causal
	RoPE
	GQA
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	residual connections
	token embeddings
	final RMSNorm
	LM head -> logits
jais
	dense
	causal decoder-only LM
	token embeddings
	ALiBi
	MHA / GQA depending on KV-head metadata
	QKV projection bias
	attention output bias
	LayerNorm
	pre-attn LayerNorm
	pre-ffn LayerNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	FFN biases
	residual connections
	final LayerNorm
	untied LM head
	LM head -> logits
jais2
	dense
	causal
	GQA-capable
	separate Q/K/V projections
	Q/K/V projection bias
	attention output bias
	RoPE
	rotary dimension
	attention scaling
	LayerNorm
	pre-attn LayerNorm
	pre-ffn LayerNorm
	ReLU-squared sequential FFN
	FFN bias
	residual connections
	token embeddings
	optional tied LM head
	final LayerNorm
	embedding output
	LM head -> logits
jamba
	hybrid attention/scan
	decoder-only
	transformer
	causal
	hybrid Mamba / attention layers
	Mamba recurrent layers
	attention layers
	no RoPE
	GQA in attention layers
	Mamba / SSM
	SSM conv state
	SSM state
	dt/B/C RMSNorm inside Mamba
	SwiGLU
	RMSNorm
	pre-mixer RMSNorm
	pre-ffn RMSNorm
	MoE FFN optional per layer
	routed expert MLP
	top-k MoE
	softmax expert gating
	routed experts
	dense FFN fallback
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
jina-bert-v2
	dense
	embedding model
	BERT-style encoder
	bidirectional self-attention
	non-causal attention
	token embeddings
	token-type embeddings
	ALiBi
	symmetric bidirectional ALiBi
	no learned absolute positional embeddings
	MHA
	attention scaling
	optional Q/K LayerNorm
	LayerNorm
	embedding LayerNorm
	post-attn LayerNorm
	post-ffn LayerNorm
	gated GELU FFN
	GLU-style FFN
	residual connections
	pooled / embedding output
	mean pooling
jina-bert-v3
	encoder
	MoE-capable
	dense FFN fallback
	bidirectional
	token embeddings
	token type embeddings
	token embedding LayerNorm
	RoPE
	GQA-capable
	attention output bias optional
	attention scaling
	LayerNorm
	post-attn norm
	post-ffn norm
	GELU sequential FFN
	routed expert MLP optional
	MoE every N layers optional
	router logits optional
	top-k MoE optional
	residual connections
	embedding output
	task-specific LoRA adapters
	Matryoshka embedding dimensions
kimi-linear
	MoE
	decoder-only
	transformer
	causal
	hybrid KDA / MLA layers
	KDA recurrent layers
	Kimi Delta Attention
	linear-attn / delta-net scan
	recurrent state
	Q/K/V causal conv1d
	Q/K L2 norm in KDA
	beta mixing coefficient
	A_log decay path
	output gate in KDA
	RMSNorm-gated KDA output
	MLA attention layers
	compressed KV latent in MLA
	KV latent RMSNorm
	NoPE MLA path
	no RoPE applied in llama.cpp
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	leading dense FFN layers
	routed expert MLP
	shared expert MLP
	top-k MoE
	expert probability bias
	renormalized expert weights
	residual connections
	token embeddings
	final RMSNorm
	LM head -> logits
lfm2
	hybrid short-conv/attention
	dense
	causal decoder-only LM
	token embeddings
	gated short-convolution layers
	recurrent shortconv state
	GQA attention layers
	RoPE in attention layers
	pre-RoPE Q/K RMSNorm
	RMSNorm
	pre-mixer RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
lfm2moe
	hybrid attention/shortconv layers
	MoE
	dense leading layers
	dense FFN fallback
	causal
	GQA
	RoPE
	Q/K norm
	RMSNorm
	pre-attn norm
	pre-ffn norm
	short convolution
	shortconv recurrent layers
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	router bias
	expert gating function
	expert weight scaling
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	optional dense projection output
llada
	diffusion decoder
	decoder-only
	transformer
	non-causal attention
	bidirectional
	RoPE
	GQA-capable Q/K/V shapes
	separate Q/K/V projections
	no Q/K/V projection bias
	optional attention output bias
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
llada-moe
	masked diffusion language model
	MoE
	bidirectional self-attention
	non-causal attention
	token embeddings
	MHA
	RoPE
	full rotary fraction
	Q/K RMSNorm
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	SiLU gated expert FFN
	SwiGLU-style expert MLP
	routed expert MLP
	top-8 MoE
	64 routed experts
	router logits
	softmax expert gating
	routed expert weight scaling
	aux load-balancing loss
	no shared experts
	untied LM head
	LM head -> logits
llama
	dense
	causal decoder-only LM
	token embeddings
	GQA-capable
	RoPE
	rotary dimension
	RoPE factors optional
	attention output bias optional
	attention scaling
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	SiLU parallel gated FFN
	residual connections
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
llama4
	MoE
	decoder-only
	transformer
	causal
	RoPE
	conditional NoPE layers
	chunked sliding-window attention optional
	3 chunked / 1 full attention pattern default
	GQA
	Q/K norm conditional
	Llama4TextL2Norm on Q/K when enabled
	SwiGLU
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	interleaved MoE layers
	dense FFN on non-MoE layers
	routed expert MLP
	shared expert MLP
	sigmoid expert gating
	top-k MoE
	16 or 128 routed experts by type
	shared expert branch
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
llama-embed
	dense
	embedding model
	Llama-style transformer
	token embeddings
	RoPE
	bidirectional / non-causal self-attention
	no KV cache in embedding graph
	GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	residual connections
	final RMSNorm
	embedding output
	pooling outside core graph
maincoder
	dense
	causal
	GQA
	separate Q/K/V projections
	RoPE
	rotary dimension
	post-RoPE Q/K norm
	attention output bias
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
mamba
	recurrent
	SSM
	Mamba selective scan
	convolution state
	SSM state
	input projection -> x/z split
	1D causal conv path
	SiLU conv activation
	dt/B/C projection
	dt projection + bias
	A state matrix
	D skip parameter
	SSM scan
	SwiGLU-style output gate
	output projection
	RMSNorm
	pre-SSM RMSNorm
	residual connections
	token embeddings
	final RMSNorm
	tied input/output embeddings fallback
	LM head -> logits
mamba2
	recurrent
	Mamba-2 / SSM
	attention-free
	token embeddings
	SSM input projection
	SSM conv1d
	SSM A/D state parameters
	SSM time-step projection
	SSM grouped state
	SSM gated projection
	SSM output projection
	RMSNorm
	pre-SSM RMSNorm
	SSM RMSNorm
	final RMSNorm
	optional tied LM head
	LM head -> logits
mellum
	MoE
	causal
	GQA
	RoPE
	rotary dimension
	Q/K norm
	sliding-window attention optional
	sliding/full attention pattern
	SWA-specific RoPE frequency base
	attention output bias
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated expert FFN
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	softmax expert gating
	expert weight scaling
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
mimo2
	MoE-capable
	dense FFN fallback
	causal
	GQA
	fused QKV optional
	separate Q/K/V optional
	RoPE
	per-layer RoPE frequency
	rotary dimension
	sliding-window attention
	sliding-window pattern
	attention sinks optional
	attention scaling
	attention value scaling optional
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	SiLU parallel gated expert FFN
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	router bias
	sigmoid expert gating
	expert weight scaling
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
	NextN / MTP tensors loaded but skipped
minicpm
	dense
	causal decoder-only LM
	token embeddings
	RoPE by default
	optional LongRoPE factors
	MHA / GQA depending on KV-head metadata
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention output bias optional
	SiLU gated FFN
	SwiGLU-style MLP
	FFN bias optional
	embedding scaling
	residual scaling
	logit scaling
	residual connections
	final RMSNorm
	optional tied LM head
	LM head -> logits
minicpm3
	dense
	causal
	MLA-style attention
	Q LoRA rank projection
	KV LoRA rank compression
	latent Q RMSNorm
	latent KV RMSNorm
	partial RoPE
	NoPE Q/K channels
	shared RoPE key
	RoPE
	rotary dimension
	LongRoPE factors optional
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	residual depth scaling
	token embeddings
	input embedding scaling
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	LM head scaling
minimax-m2
	MoE
	causal
	GQA
	separate Q/K/V projections
	RoPE
	partial RoPE
	rotary dimension
	Q/K norm
	RMSNorm
	pre-attn norm
	pre-ffn norm
	attention scaling
	SiLU parallel gated expert FFN
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	router bias
	expert gating function
	expert weight scaling
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
mistral3
	dense
	multimodal conditional generation model
	vision encoder
	Pixtral vision tower
	multimodal projector
	causal language decoder
	token embeddings
	RoPE
	causal
	full self-attention
	GQA
	attention scaling
	SiLU gated MLP
	SwiGLU-style MLP
	RMSNorm
	residual connections
	final RMSNorm
	LM head -> logits
mistral4
	MoE
	decoder-only
	transformer
	multimodal conditional generation
	text + image input
	text output
	Pixtral vision tower
	multimodal projector
	RoPE
	YaRN-scaled RoPE
	partial RoPE attention heads
	NoPE attention sub-dim
	causal
	MHA
	low-rank Q projection
	low-rank KV projection
	SwiGLU
	RMSNorm
	routed expert MLP
	shared expert MLP
	128 routed experts
	4 active routed experts per token
	top-4 MoE
	normalized top-k expert weights
	no attention bias
	no MLP bias
	token embeddings
	untied input/output embeddings
	LM head -> logits
modern-bert
	encoder
	dense
	bidirectional
	token embeddings
	token embedding LayerNorm
	RoPE
	per-layer RoPE frequency
	sliding-window attention optional
	symmetric sliding-window attention
	global attention layers
	sliding/global attention pattern
	GQA-capable
	fused QKV
	attention scaling
	LayerNorm
	pre-attn norm
	pre-ffn norm
	GeGLU sequential FFN
	SiLU/SwiGLU FFN optional
	residual connections
	final LayerNorm
	embedding output
	optional classification head
mpt
	dense
	causal decoder-only LM
	token embeddings
	MHA
	ALiBi
	ALiBi attention bias
	no learned positional embeddings
	LayerNorm
	pre-attn LayerNorm
	pre-ffn LayerNorm
	attention scaling
	GELU sequential FFN
	residual connections
	final LayerNorm
	LM head -> logits
nemotron
	dense
	decoder-only
	transformer
	RoPE
	partial RoPE
	causal
	GQA
	query-key layer scaling
	normalized attention scores
	squared-ReLU FFN
	LayerNorm1p
	pre-LN transformer block
	residual connections
	no projection biases
	token embeddings
	final norm
	untied input/output embeddings
	LM head -> logits
nemotron_h
	hybrid attention/scan/FFN layers
	causal
	GQA-capable
	attention output bias optional
	attention scaling
	Mamba2 / SSM scan
	recurrent state
	SSM conv1d
	SSM conv1d bias optional
	SSM A parameter
	SSM D skip parameter
	SSM dt bias
	SSM group norm
	RMSNorm
	pre-block RMSNorm
	ReLU-squared parallel FFN
	MoE optional
	routed expert MLP optional
	shared expert MLP optional
	router logits
	router bias
	sigmoid expert gating
	expert weight normalization optional
	expert weight scaling optional
	MoE latent down projection optional
	MoE latent up projection optional
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
nemotron_h_moe
	hybrid
	decoder-only
	causal
	hybrid Mamba2 / attention layers
	SSM scan layers
	Mamba2 selective SSM
	Mamba convolution state
	Mamba SSM state
	RoPE in attention layers
	GQA in attention layers
	Q/K norm in attention layers
	RMSNorm
	pre-norm residual path
	MoE FFN layers
	routed expert MLP
	shared expert MLP
	top-2 MoE
	8 routed experts default
	1 shared expert default
	normalized top-k expert weights
	squared-ReLU FFN activation
	Mamba SiLU activation
	residual connections
	no attention bias
	no MLP bias
	no Mamba projection bias
	token embeddings
	final norm
	untied input/output embeddings
	LM head -> logits
neo-bert
	dense
	bidirectional encoder-only transformer
	non-causal self-attention
	token embeddings
	MHA
	RoPE
	full-head rotary
	attention scaling
	SwiGLU FFN
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	residual connections
	final RMSNorm
	MLM head -> logits
nomic-bert
	encoder
	dense
	bidirectional
	token embeddings
	token type embeddings
	token embedding LayerNorm
	RoPE
	rotary dimension
	GQA-capable
	attention scaling
	LayerNorm
	pre-attn norm
	pre-ffn norm
	GELU sequential FFN
	residual connections
	final embedding output
	embedding model
nomic-bert-moe
	MoE encoder
	bidirectional encoder-only transformer
	non-causal attention
	token embeddings
	RoPE
	full rotary fraction
	MHA
	QKV projection bias
	attention scaling
	LayerNorm
	embedding LayerNorm
	post-attn LayerNorm
	post-ffn LayerNorm
	GELU sequential FFN
	dense FFN on alternating layers
	routed expert MLP on alternating layers
	top-2 MoE
	8 routed experts
	router logits
	softmax expert gating
	aux load-balancing loss
	no shared experts
	pooled / embedding output
olmo
	dense
	causal
	GQA-capable
	RoPE
	rotary dimension
	attention scaling
	LayerNorm without learned norm weights
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings
	optional tied LM head
	final norm
	embedding output
	LM head -> logits
olmo2
	dense 
	decoder-only
	transformer
	causal
	RoPE
	MHA by default
	Q/K norm
	SwiGLU
	RMSNorm
	post-attn RMSNorm
	post-ffn RMSNorm
	residual connections
	no attention projection bias
	attention dropout 0.0
	token embeddings
	final RMSNorm
	untied input/output embeddings
	LM head -> logits
olmoe
	MoE
	RoPE
	causal
	SwiGLU
	routed expert MLP
	top-k MoE
	fine-grained expert routing
	dropless token routing
openelm
	dense
	RoPE
	causal
	GQA
	SwiGLU
	RMSNorm
openelm
	dense
	causal decoder-only LM
	token embeddings
	GQA
	RoPE
	head-dim rotary
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SwiGLU FFN
	layer-wise scaled transformer widths
	no linear biases
	residual connections
	final RMSNorm
	LM head -> logits
paddleocr
	OCR / speech / audio
	multimodal encoder
	multimodal decoder
	2D positional encoding
	2D RoPE
	bidirectional
	causal
	OCR vision encoder
	text decoder path
pangu-embedded
	dense
	causal
	GQA-capable
	RoPE
	rotary dimension
	LongRoPE optional
	attention output bias
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	optional output bias
phi2
	dense
	causal
	GQA-capable
	RoPE
	rotary dimension
	Q-scaled attention
	LayerNorm
	pre-attn LayerNorm
	parallel attention + FFN block
	GELU sequential FFN
	residual connections
	token embeddings
	final LayerNorm
	embedding output
	LM head -> logits
	output bias
phi3
	dense
	causal
	GQA
	RoPE
	rotary dimension
	LongRoPE factors
	sliding-window attention disabled in llama.cpp
	Q-scaled attention
	attention output bias
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	SwiGLU sequential FFN
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	optional output bias
phimoe
	MoE
	causal
	GQA
	RoPE
	rotary dimension
	sliding-window attention
	sliding/full hybrid attention
	attention scaling
	LayerNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated expert FFN
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	softmax expert gating
	expert weight scaling
	residual connections
	token embeddings
	final LayerNorm
	embedding output
	LM head -> logits
	output bias
plamo
	dense 
	decoder-only
	transformer
	causal
	RoPE
	shared-KV attention
	GQA-like KV sharing
	SDPA attention path
	SwiGLU
	RMSNorm
	single pre-block RMSNorm
	parallel attention + MLP block
	residual connections
	no projection biases
	token embeddings
	final RMSNorm
	untied input/output embeddings
	LM head -> logits
plamo2
	hybrid attention
	SSM layers
	causal
	GQA
	fused QKV
	RoPE
	rotary dimension
	Q/K norm
	attention scaling
	Mamba / SSM scan
	recurrent state
	conv1d state
	SSM state
	SSM conv1d
	SSM A parameter
	SSM D skip parameter
	SSM dt projection
	SSM B/C/dt RMSNorm
	SSM SiLU conv activation
	SSM SwiGLU z gate
	RMSNorm
	pre-mixer norm
	post-mixer norm
	pre-ffn norm
	post-ffn norm
	SwiGLU sequential FFN
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
plamo3
	dense decoder-only transformer
	causal
	hybrid SWA / full-attn layers
	RoPE
	GQA
	SwiGLU
	RMSNorm
	pre-mixer RMSNorm
	post-mixer RMSNorm
	pre-MLP RMSNorm
	post-MLP RMSNorm
	dense gated MLP
	residual connections
	token embeddings
	final norm
	LM head -> logits
plm
	dense
	causal decoder-only LM
	token embeddings
	tied LM head
	Multi-head Latent Attention
	MLA-style compressed KV path
	KV LoRA rank
	Q split into non-RoPE and RoPE parts
	K split into compressed non-RoPE and shared RoPE parts
	partial RoPE
	shared RoPE key
	RMSNorm
	pre-attn RMSNorm
	KV-compression RMSNorm
	pre-ffn RMSNorm
	attention scaling
	squared-ReLU sequential FFN
	residual connections
	final RMSNorm
	LM head -> logits
qwen
	dense
	causal decoder-only LM
	token embeddings
	GQA-capable
	fused QKV
	QKV projection bias
	RoPE
	rotary dimension
	NeoX RoPE mode
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	residual connections
	final RMSNorm
	embedding output
	LM head -> logits
qwen2
	dense
	RoPE
	causal
	GQA
	attention bias
	SwiGLU
	RMSNorm
qwen2moe
	MoE
	causal
	GQA
	RoPE
	rotary dimension
	attention output bias
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated expert FFN
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	softmax expert gating
	expert weight scaling
	shared expert MLP
	shared expert gate
	shared experts
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
qwen2vl
	multimodal causal decoder
	vision transformer encoder
	dynamic-resolution visual tokenization
	3D patch embedding
	2D vision RoPE
	M-RoPE
	text/image/video position IDs
	causal language decoder path
	GQA in language decoder
	SwiGLU in language decoder
	RMSNorm in language decoder
	LayerNorm in vision encoder
	vision patch merger
	GELU patch-merger MLP
	residual connections
	token embeddings
	visual tokens inserted into text stream
	LM head -> logits
qwen3
	dense
	causal decoder-only LM
	token embeddings
	GQA
	RoPE
	head-dim rotary
	pre-RoPE Q/K norm
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated FFN
	SwiGLU-style MLP
	residual connections
	final RMSNorm
	untied LM head
	LM head -> logits
qwen35
	dense
	causal
	hybrid attention/recurrent layers
	full attention layers
	linear-attn / gated delta net layers
	GQA
	IMRoPE / multi-section RoPE
	rotary dimension sections
	Q/K norm
	RMSNorm
	pre-attn norm
	post-attn norm
	attention scaling
	sigmoid attention gate
	SSM conv1d
	recurrent state
	delta-net state update
	SiLU parallel gated FFN
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	NextN / MTP draft block
	MTP token embedding
	MTP hidden/input concat projection
	MTP shared head norm
	MTP shared LM head
	h_nextn output
qwen35moe
	MoE
	causal
	hybrid attention/recurrent layers
	full attention layers
	linear-attn / gated delta net layers
	GQA
	IMRoPE / multi-section RoPE
	rotary dimension sections
	Q/K norm
	RMSNorm
	pre-attn norm
	post-attn norm
	attention scaling
	sigmoid attention gate
	SSM conv1d
	recurrent state
	delta-net state update
	SiLU parallel gated expert FFN
	routed expert MLP
	shared expert MLP
	top-k MoE
	routed experts
	shared experts
	router logits
	shared expert gate
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
	NextN / MTP draft blocks
	MTP token embedding
	MTP hidden/input concat projection
	MTP shared head norm
	MTP shared LM head
	h_nextn output
qwen3moe
	MoE
	causal decoder-only LM
	GQA
	RoPE
	full-head RoPE / head-dim rotary
	pre-RoPE Q/K norm
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	attention scaling
	SiLU parallel gated expert FFN
	SwiGLU-style expert MLP
	routed expert MLP
	top-k MoE
	routed experts
	router logits
	softmax expert gating
	expert weight scaling
	residual connections
	token embeddings
	final RMSNorm
	untied LM head
	LM head -> logits
	no shared experts
qwen3next
	hybrid Gated DeltaNet/Gated Attention
	MoE
	RoPE
	partial RoPE
	causal
	Gated DeltaNet linear attention
	GQA in gated-attention layers
	Q/K norm
	gated DeltaNet
	delta-net
	SiLU gated MLP
	SwiGLU-style MLP
	shared expert MLP
	routed expert MLP
	RMSNorm
	zero-centered RMSNorm
	top-k MoE
	shared experts
	routed experts
	high-sparsity MoE
	multi-token prediction
qwen3vl
	multimodal
	dense
	vision encoder
	ViT vision encoder
	DeepStack vision feature fusion
	MLP vision-language projector
	visual token insertion
	language decoder path
	causal
	GQA
	RoPE
	Interleaved-MRoPE
	3D RoPE
	temporal RoPE
	height RoPE
	width RoPE
	Q/K norm
	RMSNorm
	SwiGLU
	token embeddings
	LM head -> logits
	text + image + video
	text-timestamp alignment
qwen3vlmoe
	multimodal causal generation model
	MoE
	vision encoder
	causal language decoder
	language decoder path
	RoPE
	Interleaved-MRoPE
	temporal-height-width RoPE sections
	causal
	GQA
	Q/K norm
	RMSNorm
	SiLU gated MLP
	SwiGLU-style MLP
	routed expert MLP
	top-k MoE
	routed experts
	DeepStack visual feature fusion
	vision-token integration
	LM head -> logits
refact
	dense
	causal
	GQA
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
rnd1
	MoE
	RoPE
	bidirectional
	diffusion LM
	GQA
	Q/K norm
	SwiGLU
	RMSNorm
rwkv6
	recurrent
	RWKV-style recurrent
	WKV time-mix
	linear-attn-style recurrence
	recurrent state
	token shift
	time-mix lerp
	time decay
	time first
	receptance gate
	time-mix gate
	channel-mixing MLP
	receptance-gated channel mix
	squared ReLU channel key
	LayerNorm
	token embedding norm
	pre-time-mix norm
	pre-channel-mix norm
	residual connections
	token embeddings
	final norm
	embedding output
	LM head -> logits
rwkv6qwen2
	recurrent
	RWKV6 time-mixing
	linear-attn
	Qwen2 channel-mixing MLP
	SwiGLU
	RMSNorm
rwkv7
	recurrent
	RWKV-style recurrent
	attention-free
	linear-time sequence mixing
	dynamic state evolution
	time-mixing
	channel-mixing MLP
seed_oss
	dense
	causal
	GQA
	RoPE
	rotary dimension
	attention scaling
	attention output bias
	RMSNorm
	pre-attn norm
	post-attn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
smallthinker
	MoE
	causal
	GQA
	RoPE
	NoPE layers
	sliding-window attention
	sliding/full hybrid attention
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	ReLU expert FFN
	top-k MoE
	router logits
	expert gating function
	expert weight scaling
	residual connections
	token embeddings
	optional tied LM head
	final RMSNorm
	embedding output
	LM head -> logits
smollm3
	dense
	causal decoder-only LM
	transformer decoder
	token embeddings
	tied input/output embeddings
	GQA
	RoPE/NoPE hybrid
	NoPE every 4th layer
	full attention
	attention scaling
	RMSNorm
	pre-attn RMSNorm
	pre-ffn RMSNorm
	SwiGLU / SiLU gated MLP
	residual connections
	final RMSNorm
	LM head -> logits
	autoregressive next-token prediction
	long-context capable
	YaRN extension capable
stablelm
	dense
	RoPE
	partial rotary
	causal
	SwiGLU
	LayerNorm
starcoder
	dense
	learned absolute positional embeddings
	fused QKV
	QKV bias
	attention output bias
	attention scaling
	GELU sequential
	LayerNorm
	pre-attn norm
	pre-ffn norm
	residual connections
	token embeddings
	final norm
	embedding output
	LM head -> logits
starcoder2
	dense
	causal decoder-only LM
	GQA
	RoPE
	full-head rotary dimension
	sliding-window attention
	attention scaling
	LayerNorm
	pre-attn norm
	pre-ffn norm
	GELU sequential MLP
	residual connections
	token embeddings
	final norm
	LM head -> logits
	fill-in-the-middle capable
step35
	MoE
	dense/MoE hybrid layers
	dense MLP fallback
	causal
	GQA
	sliding-window attention
	full attention
	sliding/full hybrid attention
	RoPE
	partial RoPE
	per-layer RoPE dimension
	per-layer RoPE factors
	LongRoPE optional
	Q/K norm
	RMSNorm
	pre-attn norm
	pre-ffn norm
	attention scaling
	head-wise attention gate
	sigmoid attention gate
	SiLU parallel gated FFN
	SwiGLU clamp
	routed expert MLP
	shared expert MLP
	top-k MoE
	router bias
	sigmoid expert gating
	expert weight normalization
	expert weight scaling
	shared experts
	routed experts
	residual connections
	token embeddings
	final RMSNorm
	embedding output
	LM head -> logits
	NextN / MTP draft head
	extra MTP decoder blocks
	MTP token embedding
	MTP hidden/input concat
	MTP eh projection
	MTP h norm
	MTP e norm
	MTP shared head norm
	MTP shared LM head
	h_nextn output
t5
	text-to-text
	encoder-decoder
	separate encoder stack
	separate decoder stack
	optional separate decoder layer count
	dense
	token embeddings
	optional tied LM head
	bidirectional encoder self-attn
	causal decoder self-attn
	decoder cross-attn
	relative position bucket bias
	encoder relative attention bias
	decoder relative attention bias
	GQA-capable
	attention bias
	unit attention scaling
	RMSNorm / T5LayerNorm-style
	pre-attn norm
	pre-cross-attn norm
	pre-ffn norm
	residual connections
	ReLU sequential FFN
	GELU gated parallel FFN
	final encoder norm
	final decoder norm
	encoder embedding output
	decoder embedding output
	LM head -> logits
t5encoder
	encoder
	dense
	bidirectional
	token embeddings
	relative position bias
	attention bias
	GQA-capable
	RMSNorm / T5LayerNorm-style
	pre-attn norm
	pre-ffn norm
	residual connections
	ReLU sequential FFN
	GELU gated parallel FFN
	final encoder norm
	embedding output
talkie
	dense
	causal
	token embeddings
	input RMSNorm
	RoPE
	rotary dimension
	GQA
	post-RoPE Q/K norm
	Q norm learned gain
	K norm no gain
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	embed-skip residual
	layer output scale
	final RMSNorm
	embedding output
	LM head -> logits
	logit scaling
wavtokenizer-dec
	audio codec tokens
	convolutional decoder path
	1D convolution
	depthwise 1D convolution
	PosNet
	ConvNeXt-style block
	self-attn
	attention scaling
	GroupNorm
	LayerNorm
	GELU sequential
	sigmoid gate
	residual connections
	layer scale / gamma
	output projection
	embeddings / features output
xverse
	dense
	causal
	GQA
	RoPE
	rotary dimension
	attention scaling
	RMSNorm
	pre-attn norm
	pre-ffn norm
	SiLU parallel gated FFN
	residual connections
	token embeddings
	final norm
	LM head -> logits










































.
