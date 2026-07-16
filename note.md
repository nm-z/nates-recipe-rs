




























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















































































































.
