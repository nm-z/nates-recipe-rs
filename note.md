measuring:
	engi
		ETH
			1GbE     0.125
		DISK
			SIZE     879998558208
			SATA     2.171
		GPU0
			VRAM     12868124672
			PCIe     16.728
			FLOPs    542.1
			Transfer 179.824
		CPU
			RAM      33233760256
			DDR5     22.741
			FLOPs    135.8
			Transfer 59.507
---
// skeleton
ogdl!(r"
measuring:
	engi
		ETH
			1GbE
		DISK
			SIZE
			SATA
		GPU0
			VRAM
			PCIe
			FLOPs
			Transfer
		CPU
			RAM
			DDR5
			FLOPs
			Transfer
")

Write::block(probe, ogdl!(r"measuring:".engi.ETH));
// measuring:
// 	engi
// 		ETH

let eth = measure_eth();
ogdl!(r"measuring:".engi.ETH.r"1GbE".eth)
Write::block(probe, ogdl!(r"measuring:".engi.DISK));
// 			1GbE	0.125
// 		DISK

let disk_size = measure_disk_size();
ogdl!(r"measuring:".engi.DISK.SIZE.disk_size)
Write::block(probe, ogdl!(r"measuring:".engi.DISK.SIZE));
// 			SIZE	879998558208

let disk_speed = measure_disk_speed();
ogdl!(r"measuring:".engi.DISK.SATA.disk_speed)
Write::block(probe, ogdl!(r"measuring:".engi.GPU0));
// 			SATA	2.171
// 		GPU0

let vram = measure_vram();
ogdl!(r"measuring:".engi.GPU0.VRAM.vram)
Write::block(probe, ogdl!(r"measuring:".engi.GPU0.VRAM));
// 			VRAM	12868124672

let pcie = measure_pcie();
ogdl!(r"measuring:".engi.GPU0.PCIe.pcie)
Write::block(probe, ogdl!(r"measuring:".engi.GPU0.PCIe));
// 			PCIe	16.728

let flops = measure_flops();
ogdl!(r"measuring:".engi.GPU0.FLOPs.flops)
Write::block(probe, ogdl!(r"measuring:".engi.GPU0.FLOPs));
// 			FLOPs	542.1

let transfer = measure_transfer();
ogdl!(r"measuring:".engi.GPU0.Transfer.transfer)
Write::block(probe, ogdl!(r"measuring:".engi.CPU));
// 			Transfer	179.824
// 		CPU

let ram = measure_ram();
ogdl!(r"measuring:".engi.CPU.RAM.ram)
Write::block(probe, ogdl!(r"measuring:".engi.CPU.RAM));
// 			RAM	33233760256

let ddr5 = measure_ddr();
ogdl!(r"measuring:".engi.CPU.DDR5.ddr5)
Write::block(probe, ogdl!(r"measuring:".engi.CPU.DDR5));
// 			DDR5	22.741

let cpu_flops = measure_cpu_flops();
ogdl!(r"measuring:".engi.CPU.FLOPs.cpu_flops)
Write::block(probe, ogdl!(r"measuring:".engi.CPU.FLOPs));
// 			FLOPs	135.8

let cpu_transfer = measure_cpu_transfer();
ogdl!(r"measuring:".engi.CPU.Transfer.cpu_transfer)
Write::block(probe, ogdl!(r"measuring:".engi.CPU.Transfer));
// 			Transfer	59.507


---
node
	one required:
		token    foo
		doc      r"child	parent"
	optional:
		index    [3]
		add      *
		del      {}
		selector {2}


add text node:
	ogdl!(a.b.r"hello world")
add multiple nodes:
	ogdl!(a.b.r"
		child
		child
	");
add string variable:
	ogdl!(a.b.var_name)

r"child	child"
node
	node

r"parent".r"child"
parent
	child

r"child
child"
child
child

r"parent
	child"
parent
	child

r"parent
	child	dog"
parent
	"child	dog"

r"parent
	child
	dog	parrot"
parent
	child
	"dog	parrot"

r"inline	tab
newline"
inline
	tab
newline

---


Current commit: a319c1d
Tests suite:    XXX/336

Confident about:
	1. uncached suite
	2. real detect run
	3. combined commit
Design questions:
	1. probe.rs 
	2. Old API
	3. Installed binary
---
