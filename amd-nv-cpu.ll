target triple = "amdgcn-amd-amdhsa"
; NUMERIC BEGIN
declare double @llvm.sqrt.f64(double) declare double @llvm.fabs.f64(double) declare double @llvm.floor.f64(double)
declare double @__ocml_exp_f64(double) declare double @__ocml_tanh_f64(double) declare double @__ocml_cos_f64(double) declare double @__ocml_sin_f64(double) declare double @__ocml_log_f64(double)
define internal double @recipe.add(double %left, double %right) #1 { entry: %result = fadd double %left, %right ret double %result }
define internal double @recipe.sub(double %left, double %right) #1 { entry: %result = fsub double %left, %right ret double %result }
define internal double @recipe.mul(double %left, double %right) #1 { entry: %result = fmul double %left, %right ret double %result }
define internal double @recipe.div(double %left, double %right) #1 { entry: %result = fdiv double %left, %right ret double %result }
define internal double @recipe.neg(double %value) #1 { entry: %result = fneg double %value ret double %result }
define internal i1 @recipe.oeq(double %left, double %right) #1 { entry: %result = fcmp oeq double %left, %right ret i1 %result }
define internal i1 @recipe.oge(double %left, double %right) #1 { entry: %result = fcmp oge double %left, %right ret i1 %result }
define internal i1 @recipe.ogt(double %left, double %right) #1 { entry: %result = fcmp ogt double %left, %right ret i1 %result }
define internal i1 @recipe.ole(double %left, double %right) #1 { entry: %result = fcmp ole double %left, %right ret i1 %result }
define internal i1 @recipe.olt(double %left, double %right) #1 { entry: %result = fcmp olt double %left, %right ret i1 %result }
define internal i1 @recipe.one(double %left, double %right) #1 { entry: %result = fcmp one double %left, %right ret i1 %result }
define internal i1 @recipe.ord(double %left, double %right) #1 { entry: %result = fcmp ord double %left, %right ret i1 %result }
define internal double @recipe.from.u1(i1 %value) #1 { entry: %result = uitofp i1 %value to double ret double %result }
define internal double @recipe.from.u32(i32 %value) #1 { entry: %result = uitofp i32 %value to double ret double %result }
define internal double @recipe.from.s32(i32 %value) #1 { entry: %result = sitofp i32 %value to double ret double %result }
define internal i32 @recipe.to.u32(double %value) #1 { entry: %result = fptoui double %value to i32 ret i32 %result }
define internal i32 @recipe.to.s32(double %value) #1 { entry: %result = fptosi double %value to i32 ret i32 %result }
define internal double @recipe.from.f32(float %value) #1 { entry: %result = fpext float %value to double ret double %result }
define internal double @recipe.from.f16(half %value) #1 { entry: %result = fpext half %value to double ret double %result }
define internal half @recipe.to.f16(double %value) #1 { entry: %result = fptrunc double %value to half ret half %result }
define internal double @recipe.abs(double %value) #1 { entry: %result = call double @llvm.fabs.f64(double %value) ret double %result }
define internal double @recipe.floor(double %value) #1 { entry: %result = call double @llvm.floor.f64(double %value) ret double %result }
define internal double @recipe.sqrt(double %value) #1 { entry: %result = call double @llvm.sqrt.f64(double %value) ret double %result }
define internal double @recipe.exp(double %value) #1 { entry: %result = call double @__ocml_exp_f64(double %value) ret double %result }
define internal double @recipe.tanh(double %value) #1 { entry: %result = call double @__ocml_tanh_f64(double %value) ret double %result }
define internal double @recipe.cos(double %value) #1 { entry: %result = call double @__ocml_cos_f64(double %value) ret double %result }
define internal double @recipe.sin(double %value) #1 { entry: %result = call double @__ocml_sin_f64(double %value) ret double %result }
define internal double @recipe.log(double %value) #1 { entry: %result = call double @__ocml_log_f64(double %value) ret double %result }
define internal double @recipe.atomic.add(ptr addrspace(1) %target, double %value) #1 { entry: %prior = atomicrmw fadd ptr addrspace(1) %target, double %value monotonic, align 8 ret double %prior }
define internal void @recipe.set.format(i32 %exp, i32 %man) #1 { entry: ret void }
; NUMERIC END
declare i32 @llvm.amdgcn.workitem.id.x()
declare void @llvm.amdgcn.s.barrier() declare i64 @__ockl_steadyctr_u64()
declare void @llvm.trap() @contraction_tile = external addrspace(3) global [0 x double], align 8
define internal double @contraction_input(
ptr addrspace(1) %input, i32 %row.base, i32 %position, i32 %term, i32 %span, i32 %length, i1 %conv ) #1 { entry:
%channel = udiv i32 %term, %span %window = urem i32 %term, %span
%offset = select i1 %conv, i32 %window, i32 0 %channel.base = mul i32 %channel, %length
%local.0 = add i32 %channel.base, %position %local = add i32 %local.0, %offset
%index = add i32 %row.base, %local %ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %index
%value = load double, ptr addrspace(1) %ptr, align 8 ret double %value } define internal void @contraction_forward_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output,
i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels, i32 %out.length, i32 %kernel,
i1 %has.bias, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry:
%lid = call i32 @recipe.local.id.x() %group = call i32 @recipe.group.id.x()
%block = call i32 @recipe.workgroup.size.x() %groups = udiv i32 %threads, %block
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length
%is.conv = icmp ne i32 %kernel, 0 %span = select i1 %is.conv, i32 %kernel, i32 1 %terms = mul i32 %in.channels, %span
%m.short = icmp ult i32 %tile.m, %out.length %m.tile = select i1 %m.short, i32 %tile.m, i32 %out.length
%n.short = icmp ult i32 %tile.n, %out.channels %n.tile = select i1 %n.short, i32 %tile.n, i32 %out.channels
%k.short = icmp ult i32 %tile.k, %terms %k.tile = select i1 %k.short, i32 %tile.k, i32 %terms
%positions.adjusted = add i32 %out.length, %m.tile %positions.numerator = sub i32 %positions.adjusted, 1
%position.tiles = udiv i32 %positions.numerator, %m.tile
%channels.adjusted = add i32 %out.channels, %n.tile %channels.numerator = sub i32 %channels.adjusted, 1
%channel.tiles = udiv i32 %channels.numerator, %n.tile
%jobs.per.row = mul i32 %position.tiles, %channel.tiles %jobs = mul i32 %rows, %jobs.per.row
br label %job.loop job.loop:
%job = phi i32 [ %group, %entry ], [ %job.next, %job.done ] %job.more = icmp ult i32 %job, %jobs
br i1 %job.more, label %job.step, label %exit job.step:
%row = udiv i32 %job, %jobs.per.row %within = urem i32 %job, %jobs.per.row
%position.tile = udiv i32 %within, %channel.tiles %channel.tile = urem i32 %within, %channel.tiles
%position.base = mul i32 %position.tile, %m.tile %channel.base = mul i32 %channel.tile, %n.tile
%m.remaining = sub i32 %out.length, %position.base %m.partial = icmp ult i32 %m.remaining, %m.tile
%m.count = select i1 %m.partial, i32 %m.remaining, i32 %m.tile
%n.remaining = sub i32 %out.channels, %channel.base %n.partial = icmp ult i32 %n.remaining, %n.tile
%n.count = select i1 %n.partial, i32 %n.remaining, i32 %n.tile
%row.base = mul i32 %row, %in.elements br label %channel.loop channel.loop:
%n.offset = phi i32 [ 0, %job.step ], [ %n.next, %channel.done ] %n.index = add i32 %n.offset, %lid
%active = icmp ult i32 %n.index, %n.count %channel.raw = add i32 %channel.base, %n.index
%channel = select i1 %active, i32 %channel.raw, i32 0 br label %position.loop position.loop:
%m = phi i32 [ 0, %channel.loop ], [ %m.next, %position.done ] %m.more = icmp ult i32 %m, %m.count
br i1 %m.more, label %position.step, label %channel.done position.step:
%position = add i32 %position.base, %m br label %tile.loop tile.loop:
%term.base = phi i32 [ 0, %position.step ], [ %term.next, %tile.done ]
%sum = phi double [ 0.0, %position.step ], [ %tile.sum, %tile.done ]
%k.remaining = sub i32 %terms, %term.base %k.partial = icmp ult i32 %k.remaining, %k.tile
%k.count = select i1 %k.partial, i32 %k.remaining, i32 %k.tile br label %load.loop load.loop:
%load = phi i32 [ %lid, %tile.loop ], [ %load.next, %load.step ] %load.more = icmp ult i32 %load, %k.count
br i1 %load.more, label %load.step, label %load.done load.step: %term = add i32 %term.base, %load
%value = call double @contraction_input( ptr addrspace(1) %input, i32 %row.base, i32 %position,
i32 %term, i32 %span, i32 %in.length, i1 %is.conv ) %tile.ptr = getelementptr [0 x double],
ptr addrspace(3) @contraction_tile, i32 0, i32 %load store double %value, ptr addrspace(3) %tile.ptr, align 8
%load.next = add i32 %load, %block br label %load.loop load.done:
call void @recipe.local.barrier() br label %compute.loop compute.loop:
%i = phi i32 [ 0, %load.done ], [ %i.next, %compute.step ]
%partial = phi double [ %sum, %load.done ], [ %partial.next, %compute.step ] %compute.more = icmp ult i32 %i, %k.count
br i1 %compute.more, label %compute.step, label %compute.done compute.step: %input.ptr = getelementptr [0 x double],
ptr addrspace(3) @contraction_tile, i32 0, i32 %i %x = load double, ptr addrspace(3) %input.ptr, align 8
%weight.channel.base = mul i32 %channel, %terms %weight.term = add i32 %term.base, %i
%weight.index = add i32 %weight.channel.base, %weight.term
%weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.index
%w = load double, ptr addrspace(1) %weight.ptr, align 8 %product = call double @recipe.mul(double %x, double %w)
%candidate = call double @recipe.add(double %partial, double %product) %partial.next = select i1 %active, double %candidate, double %partial
%i.next = add i32 %i, 1 br label %compute.loop compute.done:
%tile.sum = phi double [ %partial, %compute.loop ] call void @recipe.local.barrier()
%term.next = add i32 %term.base, %k.count %term.more = icmp ult i32 %term.next, %terms
br i1 %term.more, label %tile.done, label %store.test tile.done: br label %tile.loop store.test:
br i1 %active, label %store, label %position.done store:
%output.row.base = mul i32 %row, %out.elements %output.channel.base = mul i32 %channel.raw, %out.length
%output.local = add i32 %output.channel.base, %position %output.index = add i32 %output.row.base, %output.local
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %output.index
%bias.base = mul i32 %out.channels, %terms %bias.index = add i32 %bias.base, %channel.raw
%bias.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %bias.index
%bias = load double, ptr addrspace(1) %bias.ptr, align 8 %biased = call double @recipe.add(double %tile.sum, double %bias)
%result = select i1 %has.bias, double %biased, double %tile.sum
store double %result, ptr addrspace(1) %output.ptr, align 8 br label %position.done position.done:
%m.next = add i32 %m, 1 br label %position.loop channel.done:
%n.next = add i32 %n.offset, %block %n.more = icmp ult i32 %n.next, %n.count
br i1 %n.more, label %channel.loop, label %job.done job.done:
%job.next = add i32 %job, %groups br label %job.loop exit: ret void }
define internal double @quantized_value(
ptr addrspace(1) %matrix, i32 %kind, i32 %row, i32 %column, i32 %columns ) #1 { entry:
switch i32 %kind, label %invalid [ i32 0, label %f32 i32 12, label %q4 i32 14, label %q6 ]
f32:
%f32.row = mul i32 %row, %columns
%f32.index = add i32 %f32.row, %column
%f32.ptr = getelementptr inbounds float, ptr addrspace(1) %matrix, i32 %f32.index
%f32.value = load float, ptr addrspace(1) %f32.ptr, align 4
%f32.result = call double @recipe.from.f32(float %f32.value)
ret double %f32.result
q4:
%q4.blocks = udiv i32 %columns, 256
%q4.row.base = mul i32 %row, %q4.blocks
%q4.block.local = udiv i32 %column, 256
%q4.block.index = add i32 %q4.row.base, %q4.block.local
%q4.block.offset = mul i32 %q4.block.index, 144
%q4.block = getelementptr inbounds i8, ptr addrspace(1) %matrix, i32 %q4.block.offset
%q4.d.ptr = getelementptr inbounds i8, ptr addrspace(1) %q4.block, i32 0
%q4.min.ptr = getelementptr inbounds i8, ptr addrspace(1) %q4.block, i32 2
%q4.d.half = load half, ptr addrspace(1) %q4.d.ptr, align 2
%q4.min.half = load half, ptr addrspace(1) %q4.min.ptr, align 2
%q4.d = call double @recipe.from.f16(half %q4.d.half)
%q4.min = call double @recipe.from.f16(half %q4.min.half)
%q4.local = urem i32 %column, 256
%q4.sub = udiv i32 %q4.local, 32
%q4.within = urem i32 %q4.local, 32
%q4.pair = udiv i32 %q4.sub, 2
%q4.q.base = mul i32 %q4.pair, 32
%q4.q.local = add i32 %q4.q.base, %q4.within
%q4.q.offset = add i32 %q4.q.local, 16
%q4.q.ptr = getelementptr inbounds i8, ptr addrspace(1) %q4.block, i32 %q4.q.offset
%q4.q.byte = load i8, ptr addrspace(1) %q4.q.ptr, align 1
%q4.q.raw = zext i8 %q4.q.byte to i32
%q4.odd = urem i32 %q4.sub, 2
%q4.q.shift = mul i32 %q4.odd, 4
%q4.q.shifted = lshr i32 %q4.q.raw, %q4.q.shift
%q4.q = and i32 %q4.q.shifted, 15
%q4.low = icmp ult i32 %q4.sub, 4
br i1 %q4.low, label %q4.scale.low, label %q4.scale.high
q4.scale.low:
%q4.low.scale.offset = add i32 %q4.sub, 4
%q4.low.scale.ptr = getelementptr inbounds i8, ptr addrspace(1) %q4.block, i32 %q4.low.scale.offset
%q4.low.min.offset = add i32 %q4.sub, 8
%q4.low.min.ptr = getelementptr inbounds i8, ptr addrspace(1) %q4.block, i32 %q4.low.min.offset
%q4.low.scale.byte = load i8, ptr addrspace(1) %q4.low.scale.ptr, align 1
%q4.low.min.byte = load i8, ptr addrspace(1) %q4.low.min.ptr, align 1
%q4.low.scale.raw = zext i8 %q4.low.scale.byte to i32
%q4.low.min.raw = zext i8 %q4.low.min.byte to i32
%q4.low.scale = and i32 %q4.low.scale.raw, 63
%q4.low.min = and i32 %q4.low.min.raw, 63
br label %q4.calculate
q4.scale.high:
%q4.high.packed.offset = add i32 %q4.sub, 8
%q4.high.packed.ptr = getelementptr inbounds i8, ptr addrspace(1) %q4.block, i32 %q4.high.packed.offset
%q4.high.scale.ptr = getelementptr inbounds i8, ptr addrspace(1) %q4.block, i32 %q4.sub
%q4.high.min.offset = add i32 %q4.sub, 4
%q4.high.min.ptr = getelementptr inbounds i8, ptr addrspace(1) %q4.block, i32 %q4.high.min.offset
%q4.high.packed.byte = load i8, ptr addrspace(1) %q4.high.packed.ptr, align 1
%q4.high.scale.byte = load i8, ptr addrspace(1) %q4.high.scale.ptr, align 1
%q4.high.min.byte = load i8, ptr addrspace(1) %q4.high.min.ptr, align 1
%q4.high.packed = zext i8 %q4.high.packed.byte to i32
%q4.high.scale.bits = zext i8 %q4.high.scale.byte to i32
%q4.high.min.bits = zext i8 %q4.high.min.byte to i32
%q4.high.scale.low = and i32 %q4.high.packed, 15
%q4.high.scale.top = lshr i32 %q4.high.scale.bits, 6
%q4.high.scale.top.shifted = shl i32 %q4.high.scale.top, 4
%q4.high.scale = or i32 %q4.high.scale.low, %q4.high.scale.top.shifted
%q4.high.min.low = lshr i32 %q4.high.packed, 4
%q4.high.min.top = lshr i32 %q4.high.min.bits, 6
%q4.high.min.top.shifted = shl i32 %q4.high.min.top, 4
%q4.high.min = or i32 %q4.high.min.low, %q4.high.min.top.shifted
br label %q4.calculate
q4.calculate:
%q4.scale = phi i32 [ %q4.low.scale, %q4.scale.low ], [ %q4.high.scale, %q4.scale.high ]
%q4.minimum = phi i32 [ %q4.low.min, %q4.scale.low ], [ %q4.high.min, %q4.scale.high ]
%q4.scale.double = call double @recipe.from.u32(i32 %q4.scale)
%q4.minimum.double = call double @recipe.from.u32(i32 %q4.minimum)
%q4.q.double = call double @recipe.from.u32(i32 %q4.q)
%q4.step = call double @recipe.mul(double %q4.d, double %q4.scale.double)
%q4.base = call double @recipe.mul(double %q4.min, double %q4.minimum.double)
%q4.product = call double @recipe.mul(double %q4.step, double %q4.q.double)
%q4.result = call double @recipe.sub(double %q4.product, double %q4.base)
ret double %q4.result
q6:
%q6.blocks = udiv i32 %columns, 256
%q6.row.base = mul i32 %row, %q6.blocks
%q6.block.local = udiv i32 %column, 256
%q6.block.index = add i32 %q6.row.base, %q6.block.local
%q6.block.offset = mul i32 %q6.block.index, 210
%q6.block = getelementptr inbounds i8, ptr addrspace(1) %matrix, i32 %q6.block.offset
%q6.local = urem i32 %column, 256
%q6.chunk = udiv i32 %q6.local, 128
%q6.chunk.local = urem i32 %q6.local, 128
%q6.group = udiv i32 %q6.chunk.local, 32
%q6.within = urem i32 %q6.chunk.local, 32
%q6.low.group = and i32 %q6.group, 1
%q6.low.extra = mul i32 %q6.low.group, 32
%q6.low.local = add i32 %q6.within, %q6.low.extra
%q6.low.chunk = mul i32 %q6.chunk, 64
%q6.low.offset = add i32 %q6.low.chunk, %q6.low.local
%q6.low.ptr = getelementptr inbounds i8, ptr addrspace(1) %q6.block, i32 %q6.low.offset
%q6.high.chunk = mul i32 %q6.chunk, 32
%q6.high.local = add i32 %q6.high.chunk, %q6.within
%q6.high.offset = add i32 %q6.high.local, 128
%q6.high.ptr = getelementptr inbounds i8, ptr addrspace(1) %q6.block, i32 %q6.high.offset
%q6.low.byte = load i8, ptr addrspace(1) %q6.low.ptr, align 1
%q6.high.byte = load i8, ptr addrspace(1) %q6.high.ptr, align 1
%q6.low.raw = zext i8 %q6.low.byte to i32
%q6.high.raw = zext i8 %q6.high.byte to i32
%q6.low.half = udiv i32 %q6.group, 2
%q6.low.shift = mul i32 %q6.low.half, 4
%q6.high.shift = mul i32 %q6.group, 2
%q6.low.shifted = lshr i32 %q6.low.raw, %q6.low.shift
%q6.high.shifted = lshr i32 %q6.high.raw, %q6.high.shift
%q6.low.bits = and i32 %q6.low.shifted, 15
%q6.high.bits = and i32 %q6.high.shifted, 3
%q6.high.top = shl i32 %q6.high.bits, 4
%q6.quant.unsigned = or i32 %q6.low.bits, %q6.high.top
%q6.quant = sub i32 %q6.quant.unsigned, 32
%q6.scale.half = udiv i32 %q6.within, 16
%q6.scale.group = mul i32 %q6.group, 2
%q6.scale.local = add i32 %q6.scale.group, %q6.scale.half
%q6.scale.chunk = mul i32 %q6.chunk, 8
%q6.scale.index = add i32 %q6.scale.chunk, %q6.scale.local
%q6.scale.offset = add i32 %q6.scale.index, 192
%q6.scale.ptr = getelementptr inbounds i8, ptr addrspace(1) %q6.block, i32 %q6.scale.offset
%q6.scale.byte = load i8, ptr addrspace(1) %q6.scale.ptr, align 1
%q6.scale = sext i8 %q6.scale.byte to i32
%q6.d.ptr = getelementptr inbounds i8, ptr addrspace(1) %q6.block, i32 208
%q6.d.half = load half, ptr addrspace(1) %q6.d.ptr, align 2
%q6.d = call double @recipe.from.f16(half %q6.d.half)
%q6.scale.double = call double @recipe.from.s32(i32 %q6.scale)
%q6.quant.double = call double @recipe.from.s32(i32 %q6.quant)
%q6.scaled = call double @recipe.mul(double %q6.d, double %q6.scale.double)
%q6.result = call double @recipe.mul(double %q6.scaled, double %q6.quant.double)
ret double %q6.result
invalid: call void @llvm.trap() ret double 0.0 }
define internal void @pool_forward_body( ptr addrspace(1) %input, ptr addrspace(1) %output, ptr addrspace(1) %context,
i32 %p, i32 %from, i32 %to, i32 %size, i32 %channels ) #1 { entry: %length = udiv i32 %from, %channels
%pooled.length = udiv i32 %to, %channels %row = udiv i32 %p, %to %out = urem i32 %p, %to
%channel = udiv i32 %out, %pooled.length %spatial = urem i32 %out, %pooled.length %start = mul i32 %spatial, %size
%candidate.end = add i32 %start, %size %short = icmp ult i32 %candidate.end, %length
%end = select i1 %short, i32 %candidate.end, i32 %length %row.base = mul i32 %row, %from
%channel.local = mul i32 %channel, %length %input.base = add i32 %row.base, %channel.local br label %loop loop:
%i = phi i32 [ %start, %entry ], [ %next, %step ]
%maximum = phi double [ 0xFFF0000000000000, %entry ], [ %maximum.next, %step ]
%maximum.index = phi i32 [ %start, %entry ], [ %maximum.index.next, %step ] %more = icmp ult i32 %i, %end
br i1 %more, label %step, label %done step: %index = add i32 %input.base, %i
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %index
%value = load double, ptr addrspace(1) %input.ptr, align 8 %greater = call i1 @recipe.ogt(double %value, double %maximum)
%maximum.next = select i1 %greater, double %value, double %maximum
%maximum.index.next = select i1 %greater, i32 %index, i32 %maximum.index %next = add i32 %i, 1 br label %loop done:
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
%context.ptr = getelementptr inbounds i64, ptr addrspace(1) %context, i32 %p
%maximum.index.wide = zext i32 %maximum.index to i64
store double %maximum, ptr addrspace(1) %output.ptr, align 8
store i64 %maximum.index.wide, ptr addrspace(1) %context.ptr, align 8 ret void }
define internal i32 @embedding_index(double %value, i32 %vocabulary) #1 { entry:
%difference = call double @recipe.sub(double %value, double %value) %finite = call i1 @recipe.ord(double %difference, double %difference) br i1 %finite, label %convert, label %invalid convert:
%floored = call double @recipe.floor(double %value)
%max.index = sub i32 %vocabulary, 1 %max.double = call double @recipe.from.u32(i32 %max.index)
%low = call i1 @recipe.olt(double %floored, double 0.0) %clamped.low = select i1 %low, double 0.0, double %floored
%high = call i1 @recipe.ogt(double %clamped.low, double %max.double)
%clamped = select i1 %high, double %max.double, double %clamped.low
%index = call i32 @recipe.to.u32(double %clamped) ret i32 %index
invalid: ret i32 %vocabulary } define internal void @embedding_forward_body(
ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %table, ptr addrspace(1) nocapture writeonly %output, ptr addrspace(1) %context, i32 %p, i32 %from, i32 %to, i32 %vocabulary ) #1 { entry:
%dimensions = udiv i32 %to, %from %row = udiv i32 %p, %to %local = urem i32 %p, %to %component = udiv i32 %local, %from
%slot = urem i32 %local, %from %row.base = mul i32 %row, %from %input.index = add i32 %row.base, %slot
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
%value = load double, ptr addrspace(1) %input.ptr, align 8
%index = call i32 @embedding_index(double %value, i32 %vocabulary) %valid = icmp ult i32 %index, %vocabulary
br i1 %valid, label %lookup, label %invalid lookup: %table.base = mul i32 %index, %dimensions
%table.index = add i32 %table.base, %component
%table.ptr = getelementptr inbounds double, ptr addrspace(1) %table, i32 %table.index
%embedded = load double, ptr addrspace(1) %table.ptr, align 8 br label %store invalid:
store atomic i32 1, ptr addrspace(1) %context monotonic, align 4 br label %store store:
%result = phi double [ %embedded, %lookup ], [ 0x7FF8000000000000, %invalid ]
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %result, ptr addrspace(1) %output.ptr, align 8 ret void } define internal void @embedding_reverse_body(
ptr addrspace(1) %input, ptr addrspace(1) %delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.previous,
i32 %rows, i32 %tokens, i32 %dimensions, i32 %vocabulary, i32 %offset, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x() %parameters = mul i32 %dimensions, %vocabulary %output.row.width = mul i32 %tokens, %dimensions br label %parameter.loop
parameter.loop: %p = phi i32 [ %tid, %entry ], [ %next, %store ] %more = icmp ult i32 %p, %parameters
br i1 %more, label %row.loop, label %input.test row.loop:
%row = phi i32 [ 0, %parameter.loop ], [ %row.next, %token.loop.done ]
%sum = phi double [ 0.0, %parameter.loop ], [ %row.sum, %token.loop.done ] %row.more = icmp ult i32 %row, %rows
br i1 %row.more, label %token.loop, label %store token.loop:
%token = phi i32 [ 0, %row.loop ], [ %token.next, %token.step ]
%token.sum = phi double [ %sum, %row.loop ], [ %sum.next, %token.step ] %token.more = icmp ult i32 %token, %tokens
br i1 %token.more, label %token.step, label %token.loop.done token.step: %input.base = mul i32 %row, %tokens
%input.index = add i32 %input.base, %token
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
%input.value = load double, ptr addrspace(1) %input.ptr, align 8
%index = call i32 @embedding_index(double %input.value, i32 %vocabulary) %expected = udiv i32 %p, %dimensions
%matched = icmp eq i32 %index, %expected %component = urem i32 %p, %dimensions %output.row.base = mul i32 %row, %output.row.width
%output.channel.base = mul i32 %component, %tokens %output.local = add i32 %output.channel.base, %token
%output.index = add i32 %output.row.base, %output.local
%delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %output.index
%delta.value = load double, ptr addrspace(1) %delta.ptr, align 8
%contribution = select i1 %matched, double %delta.value, double 0.0 %sum.next = call double @recipe.add(double %token.sum, double %contribution)
%token.next = add nuw i32 %token, 1 br label %token.loop token.loop.done:
%row.sum = phi double [ %token.sum, %token.loop ] %row.next = add nuw i32 %row, 1 br label %row.loop store:
%gradient.index = add i32 %offset, %p
%gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.index
store double %sum, ptr addrspace(1) %gradient.ptr, align 8 %next = add i32 %p, %threads br label %parameter.loop input.test:
br i1 %write.previous, label %input.loop, label %exit input.loop: %input.p = phi i32 [ %tid, %input.test ], [ %input.next, %input.store ] %input.count = mul i32 %rows, %tokens
%input.more = icmp ult i32 %input.p, %input.count br i1 %input.more, label %input.component.loop, label %exit input.component.loop: %input.component = phi i32 [ 0, %input.loop ], [ %input.component.next, %input.component.step ]
%input.sum = phi double [ 0.0, %input.loop ], [ %input.sum.next, %input.component.step ] %input.component.more = icmp ult i32 %input.component, %dimensions br i1 %input.component.more, label %input.component.step, label %input.store input.component.step:
%input.row = udiv i32 %input.p, %tokens %input.token = urem i32 %input.p, %tokens %input.row.width = mul i32 %tokens, %dimensions %input.row.base = mul i32 %input.row, %input.row.width %input.component.base = mul i32 %input.component, %tokens %input.local = add i32 %input.component.base, %input.token
%input.delta.index = add i32 %input.row.base, %input.local %input.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %input.delta.index
%input.delta = load double, ptr addrspace(1) %input.delta.ptr, align 8 %input.sum.next = call double @recipe.add(double %input.sum, double %input.delta)
%input.component.next = add nuw i32 %input.component, 1 br label %input.component.loop input.store: %input.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %input.p %input.old = call double @recipe.atomic.add(ptr addrspace(1) %input.previous.ptr, double %input.sum) %input.next = add i32 %input.p, %threads br label %input.loop exit:
ret void } define internal double @sigmoid(double %x) #1 { entry: %negative = call double @recipe.neg(double %x)
%exponential = call double @recipe.exp(double %negative) %denominator = call double @recipe.add(double 1.0, double %exponential)
%value = call double @recipe.div(double 1.0, double %denominator) ret double %value }
define internal double @attention_score( ptr addrspace(1) nocapture readonly %context, i32 %plane, i32 %row, i32 %head,
i32 %query, i32 %key, i32 %from, i32 %length, i32 %head_width, double %scale ) #1 { entry:
%batch.stride = mul i32 %from, 3 %row.base = mul i32 %row, %batch.stride
%head.start = mul i32 %head, %head_width br label %channel.loop channel.loop:
%offset = phi i32 [ 0, %entry ], [ %offset.next, %channel.step ]
%sum = phi double [ 0.0, %entry ], [ %sum.next, %channel.step ] %more = icmp ult i32 %offset, %head_width
br i1 %more, label %channel.step, label %done channel.step: %channel = add i32 %head.start, %offset
%channel.base = mul i32 %channel, %length %query.local = add i32 %channel.base, %query
%key.local = add i32 %channel.base, %key %query.index = add i32 %row.base, %query.local
%key.plane = add i32 %row.base, %from %key.index = add i32 %key.plane, %key.local
%query.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %query.index
%key.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %key.index
%query.value = load double, ptr addrspace(1) %query.ptr, align 8
%key.value = load double, ptr addrspace(1) %key.ptr, align 8 %product = call double @recipe.mul(double %query.value, double %key.value)
%sum.next = call double @recipe.add(double %sum, double %product) %offset.next = add i32 %offset, 1 br label %channel.loop done:
%score = call double @recipe.div(double %sum, double %scale) ret double %score } define internal void @attention_forward_body(
ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %weights,
ptr addrspace(1) nocapture writeonly %output, ptr addrspace(1) %context,
i32 %rows, i32 %from, i32 %heads, i32 %channels, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%length = udiv i32 %from, %channels %head_width = udiv i32 %channels, %heads
%head_width.double = call double @recipe.from.u32(i32 %head_width) %scale = call double @recipe.sqrt(double %head_width.double)
%plane = mul i32 %rows, %from br label %output.loop output.loop:
%p = phi i32 [ %tid, %entry ], [ %p.next, %output.store ] %output.more = icmp ult i32 %p, %plane
br i1 %output.more, label %output.step, label %exit output.step: %output.row = udiv i32 %p, %from
%output.local = urem i32 %p, %from %output.channel.index = udiv i32 %output.local, %length
%query = urem i32 %output.local, %length %head = udiv i32 %output.channel.index, %head_width br label %online.loop
online.loop: %key = phi i32 [ 0, %output.step ], [ %key.next, %online.step ]
%maximum = phi double [ 0xFFF0000000000000, %output.step ], [ %maximum.next, %online.step ]
%denominator = phi double [ 0.0, %output.step ], [ %denominator.next, %online.step ]
%numerator = phi double [ 0.0, %output.step ], [ %numerator.next, %online.step ]
%key.more = icmp ult i32 %key, %length br i1 %key.more, label %online.step, label %output.store online.step:
%score = call double @attention_score( ptr addrspace(1) %input, i32 %plane, i32 %output.row, i32 %head, i32 %query,
i32 %key, i32 %from, i32 %length, i32 %head_width, double %scale )
%larger = call i1 @recipe.ogt(double %score, double %maximum) %maximum.next = select i1 %larger, double %score, double %maximum
%old.centered = call double @recipe.sub(double %maximum, double %maximum.next) %old.scale = call double @recipe.exp(double %old.centered)
%new.centered = call double @recipe.sub(double %score, double %maximum.next) %new.scale = call double @recipe.exp(double %new.centered)
%old.denominator = call double @recipe.mul(double %denominator, double %old.scale)
%denominator.next = call double @recipe.add(double %old.denominator, double %new.scale) %value.batch.stride = mul i32 %from, 3
%value.row = mul i32 %output.row, %value.batch.stride %value.plane = mul i32 %from, 2
%value.row.plane = add i32 %value.row, %value.plane %value.channel.base = mul i32 %output.channel.index, %length
%value.local = add i32 %value.channel.base, %key %value.index = add i32 %value.row.plane, %value.local
%value.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %value.index
%value = load double, ptr addrspace(1) %value.ptr, align 8 %weighted = call double @recipe.mul(double %new.scale, double %value)
%old.numerator = call double @recipe.mul(double %numerator, double %old.scale) %numerator.next = call double @recipe.add(double %old.numerator, double %weighted)
%key.next = add i32 %key, 1 br label %online.loop output.store: %attention = call double @recipe.div(double %numerator, double %denominator)
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %attention, ptr addrspace(1) %output.ptr, align 8 %p.next = add i32 %p, %threads br label %output.loop
exit: ret void }
define internal double @rope_value(
ptr addrspace(1) %vector, i32 %index, i32 %width, i32 %position,
double %base, double %factor, i32 %range ) #1 { entry:
%offset = urem i32 %index, %width
%half = udiv i32 %width, 2
%second = icmp uge i32 %offset, %half
%pair.first = add i32 %offset, %half
%pair.second = sub i32 %offset, %half
%pair.offset = select i1 %second, i32 %pair.second, i32 %pair.first
%pair.index = sub i32 %index, %offset
%pair = add i32 %pair.index, %pair.offset
%value.ptr = getelementptr inbounds double, ptr addrspace(1) %vector, i32 %index
%pair.ptr = getelementptr inbounds double, ptr addrspace(1) %vector, i32 %pair
%value = load double, ptr addrspace(1) %value.ptr, align 8
%paired = load double, ptr addrspace(1) %pair.ptr, align 8
%pair.number = urem i32 %offset, %half
%pair.double = call double @recipe.from.u32(i32 %pair.number)
%width.double = call double @recipe.from.u32(i32 %width)
%exponent.top = call double @recipe.mul(double %pair.double, double -2.0)
%exponent = call double @recipe.div(double %exponent.top, double %width.double)
%base.log = call double @recipe.log(double %base)
%power = call double @recipe.mul(double %base.log, double %exponent)
%frequency = call double @recipe.exp(double %power)
%position.double = call double @recipe.from.u32(i32 %position)
%theta.extrap = call double @recipe.mul(double %position.double, double %frequency)
%theta.interp = call double @recipe.mul(double %theta.extrap, double %factor)
%range.low = and i32 %range, 65535
%range.high = lshr i32 %range, 16
%ramp.top.integer = sub i32 %pair.number, %range.low
%ramp.bottom.integer = sub i32 %range.high, %range.low
%ramp.top = call double @recipe.from.s32(i32 %ramp.top.integer)
%ramp.bottom = call double @recipe.from.u32(i32 %ramp.bottom.integer)
%ramp.raw = call double @recipe.div(double %ramp.top, double %ramp.bottom)
%ramp.low = call i1 @recipe.olt(double %ramp.raw, double 0.0)
%ramp.nonnegative = select i1 %ramp.low, double 0.0, double %ramp.raw
%ramp.high = call i1 @recipe.ogt(double %ramp.nonnegative, double 1.0)
%ramp = select i1 %ramp.high, double 1.0, double %ramp.nonnegative
%one.minus.ramp = call double @recipe.sub(double 1.0, double %ramp)
%interp.part = call double @recipe.mul(double %theta.interp, double %ramp)
%extrap.part = call double @recipe.mul(double %theta.extrap, double %one.minus.ramp)
%theta = call double @recipe.add(double %interp.part, double %extrap.part)
%inverse.factor = call double @recipe.div(double 1.0, double %factor)
%factor.log = call double @recipe.log(double %inverse.factor)
%magnitude.extra = call double @recipe.mul(double %factor.log, double 0.1)
%magnitude = call double @recipe.add(double 1.0, double %magnitude.extra)
%cosine.raw = call double @recipe.cos(double %theta)
%sine.raw = call double @recipe.sin(double %theta)
%cosine = call double @recipe.mul(double %cosine.raw, double %magnitude)
%sine = call double @recipe.mul(double %sine.raw, double %magnitude)
%primary = call double @recipe.mul(double %value, double %cosine)
%secondary = call double @recipe.mul(double %paired, double %sine)
%first.value = call double @recipe.sub(double %primary, double %secondary)
%second.value = call double @recipe.add(double %primary, double %secondary)
%result = select i1 %second, double %second.value, double %first.value
ret double %result }
define internal void @cached_attention_body(
ptr addrspace(1) %input, ptr addrspace(1) %output, ptr addrspace(1) %context,
i32 %heads, i32 %kv.heads, i32 %width, i32 %maximum,
double %scale, double %rope.base, double %rope.factor, i32 %rope.range, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%q.width = mul i32 %heads, %width
%kv.width = mul i32 %kv.heads, %width
%position.double = load double, ptr addrspace(1) %context, align 8
%position = call i32 @recipe.to.u32(double %position.double)
%valid = icmp ult i32 %position, %maximum
br i1 %valid, label %cache.loop, label %invalid
cache.loop:
%cache.p = phi i32 [ %tid, %entry ], [ %cache.next, %cache.step ]
%cache.more = icmp ult i32 %cache.p, %kv.width
br i1 %cache.more, label %cache.step, label %cache.done
cache.step:
%key.input.index = add i32 %q.width, %cache.p
%key.input = getelementptr inbounds double, ptr addrspace(1) %input, i32 %q.width
%key = call double @rope_value(
ptr addrspace(1) %key.input, i32 %cache.p, i32 %width, i32 %position,
double %rope.base, double %rope.factor, i32 %rope.range )
%cache.row = mul i32 %position, %kv.width
%key.cache.index.raw = add i32 %cache.row, %cache.p
%key.cache.index = add i32 %key.cache.index.raw, 4
%key.cache.ptr = getelementptr inbounds half, ptr addrspace(1) %context, i32 %key.cache.index
%key.half = call half @recipe.to.f16(double %key)
store half %key.half, ptr addrspace(1) %key.cache.ptr, align 2
%value.input.base = add i32 %q.width, %kv.width
%value.input.index = add i32 %value.input.base, %cache.p
%value.input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %value.input.index
%value = load double, ptr addrspace(1) %value.input.ptr, align 8
%value.cache.base = mul i32 %maximum, %kv.width
%value.cache.row = add i32 %value.cache.base, %cache.row
%value.cache.local = add i32 %value.cache.row, %cache.p
%value.cache.index = add i32 %value.cache.local, 4
%value.cache.ptr = getelementptr inbounds half, ptr addrspace(1) %context, i32 %value.cache.index
%value.half = call half @recipe.to.f16(double %value)
store half %value.half, ptr addrspace(1) %value.cache.ptr, align 2
%cache.next = add i32 %cache.p, %threads
br label %cache.loop
cache.done:
call void @llvm.amdgcn.s.barrier()
br label %output.loop
output.loop:
%p = phi i32 [ %tid, %cache.done ], [ %p.next, %output.store ]
%p.more = icmp ult i32 %p, %q.width
br i1 %p.more, label %output.step, label %increment
output.step:
%head = udiv i32 %p, %width
%head.offset = urem i32 %p, %width
%heads.per.kv = udiv i32 %heads, %kv.heads
%kv.head = udiv i32 %head, %heads.per.kv
%kv.head.base = mul i32 %kv.head, %width
%value.channel = add i32 %kv.head.base, %head.offset
br label %online.loop
online.loop:
%key.position = phi i32 [ 0, %output.step ], [ %key.next, %online.step ]
%maximum.score = phi double [ 0xFFF0000000000000, %output.step ], [ %maximum.next, %online.step ]
%denominator = phi double [ 0.0, %output.step ], [ %denominator.next, %online.step ]
%numerator = phi double [ 0.0, %output.step ], [ %numerator.next, %online.step ]
%key.more = icmp ule i32 %key.position, %position
br i1 %key.more, label %score.loop, label %output.store
score.loop:
%d = phi i32 [ 0, %online.loop ], [ %d.next, %score.step ]
%score = phi double [ 0.0, %online.loop ], [ %score.next, %score.step ]
%d.more = icmp ult i32 %d, %width
br i1 %d.more, label %score.step, label %online.step
score.step:
%query.channel = mul i32 %head, %width
%query.index = add i32 %query.channel, %d
%query = call double @rope_value(
ptr addrspace(1) %input, i32 %query.index, i32 %width, i32 %position,
double %rope.base, double %rope.factor, i32 %rope.range )
%key.channel = add i32 %kv.head.base, %d
%key.row = mul i32 %key.position, %kv.width
%key.local = add i32 %key.row, %key.channel
%key.index = add i32 %key.local, 4
%key.ptr = getelementptr inbounds half, ptr addrspace(1) %context, i32 %key.index
%key.half.value = load half, ptr addrspace(1) %key.ptr, align 2
%key.value = call double @recipe.from.f16(half %key.half.value)
%product = call double @recipe.mul(double %query, double %key.value)
%score.next = call double @recipe.add(double %score, double %product)
%d.next = add nuw i32 %d, 1
br label %score.loop
online.step:
%scaled.score = call double @recipe.mul(double %score, double %scale)
%larger = call i1 @recipe.ogt(double %scaled.score, double %maximum.score)
%maximum.next = select i1 %larger, double %scaled.score, double %maximum.score
%old.centered = call double @recipe.sub(double %maximum.score, double %maximum.next)
%old.scale = call double @recipe.exp(double %old.centered)
%new.centered = call double @recipe.sub(double %scaled.score, double %maximum.next)
%new.scale = call double @recipe.exp(double %new.centered)
%old.denominator = call double @recipe.mul(double %denominator, double %old.scale)
%denominator.next = call double @recipe.add(double %old.denominator, double %new.scale)
%lookup.cache.base = mul i32 %maximum, %kv.width
%lookup.row = mul i32 %key.position, %kv.width
%lookup.local = add i32 %lookup.row, %value.channel
%lookup.raw = add i32 %lookup.cache.base, %lookup.local
%lookup.index = add i32 %lookup.raw, 4
%lookup.ptr = getelementptr inbounds half, ptr addrspace(1) %context, i32 %lookup.index
%lookup.half = load half, ptr addrspace(1) %lookup.ptr, align 2
%lookup.value = call double @recipe.from.f16(half %lookup.half)
%weighted = call double @recipe.mul(double %new.scale, double %lookup.value)
%old.numerator = call double @recipe.mul(double %numerator, double %old.scale)
%numerator.next = call double @recipe.add(double %old.numerator, double %weighted)
%key.next = add nuw i32 %key.position, 1
br label %online.loop
output.store:
%attention = call double @recipe.div(double %numerator, double %denominator)
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %attention, ptr addrspace(1) %output.ptr, align 8
%p.next = add i32 %p, %threads
br label %output.loop
increment:
call void @llvm.amdgcn.s.barrier()
%leader = icmp eq i32 %tid, 0
br i1 %leader, label %position.store, label %exit
position.store:
%position.next = add i32 %position, 1
%position.next.double = call double @recipe.from.u32(i32 %position.next)
store double %position.next.double, ptr addrspace(1) %context, align 8
br label %exit
invalid: call void @llvm.trap() br label %exit
exit: ret void }
define internal void @attention_reverse_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %context,
ptr addrspace(1) %delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient,
i1 %write.previous, i32 %rows, i32 %from, i32 %heads, i32 %channels, i32 %offset, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x() %length = udiv i32 %from, %channels
%head.width = udiv i32 %channels, %heads %head.width.double = call double @recipe.from.u32(i32 %head.width)
%scale = call double @recipe.sqrt(double %head.width.double) %plane = mul i32 %rows, %from br label %row.loop row.loop:
%row = phi i32 [ %tid, %entry ], [ %row.next, %row.done ] %row.more = icmp ult i32 %row, %rows
br i1 %row.more, label %head.loop, label %rows.done head.loop:
%head = phi i32 [ 0, %row.loop ], [ %head.next, %head.done ] %head.more = icmp ult i32 %head, %heads
br i1 %head.more, label %query.loop, label %row.done query.loop:
%query = phi i32 [ 0, %head.loop ], [ %query.next, %query.done ] %query.more = icmp ult i32 %query, %length
br i1 %query.more, label %online.statistics.loop, label %head.done online.statistics.loop:
%online.key = phi i32 [ 0, %query.loop ], [ %online.next, %online.statistics.update ]
%online.maximum = phi double [ 0xFFF0000000000000, %query.loop ], [ %online.maximum.next, %online.statistics.update ]
%online.denominator = phi double [ 0.0, %query.loop ], [ %online.denominator.next, %online.statistics.update ]
%online.weighted = phi double [ 0.0, %query.loop ], [ %online.weighted.next, %online.statistics.update ]
%online.more = icmp ult i32 %online.key, %length
br i1 %online.more, label %online.channel.loop, label %online.statistics.done online.channel.loop:
%online.channel.offset = phi i32 [ 0, %online.statistics.loop ], [ %online.channel.next, %online.channel.step ]
%online.dp = phi double [ 0.0, %online.statistics.loop ], [ %online.dp.next, %online.channel.step ]
%online.channel.more = icmp ult i32 %online.channel.offset, %head.width
br i1 %online.channel.more, label %online.channel.step, label %online.statistics.update online.channel.step:
%online.head.start = mul i32 %head, %head.width %online.channel = add i32 %online.head.start, %online.channel.offset
%online.row.base = mul i32 %row, %from %online.channel.base = mul i32 %online.channel, %length
%online.delta.local = add i32 %online.channel.base, %query
%online.value.local = add i32 %online.channel.base, %online.key
%online.delta.index = add i32 %online.row.base, %online.delta.local
%online.input.stride = mul i32 %from, 3 %online.input.row = mul i32 %row, %online.input.stride
%online.value.plane = mul i32 %from, 2 %online.value.row = add i32 %online.input.row, %online.value.plane
%online.value.index = add i32 %online.value.row, %online.value.local
%online.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %online.delta.index
%online.value.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %online.value.index
%online.delta = load double, ptr addrspace(1) %online.delta.ptr, align 8
%online.value = load double, ptr addrspace(1) %online.value.ptr, align 8
%online.dp.product = call double @recipe.mul(double %online.delta, double %online.value)
%online.dp.next = call double @recipe.add(double %online.dp, double %online.dp.product)
%online.channel.next = add nuw i32 %online.channel.offset, 1 br label %online.channel.loop online.statistics.update:
%online.score = call double @attention_score( ptr addrspace(1) %input, i32 %plane, i32 %row, i32 %head, i32 %query,
i32 %online.key, i32 %from, i32 %length, i32 %head.width, double %scale )
%online.larger = call i1 @recipe.ogt(double %online.score, double %online.maximum)
%online.maximum.next = select i1 %online.larger, double %online.score, double %online.maximum
%online.old.centered = call double @recipe.sub(double %online.maximum, double %online.maximum.next)
%online.old.scale = call double @recipe.exp(double %online.old.centered)
%online.new.centered = call double @recipe.sub(double %online.score, double %online.maximum.next)
%online.new.scale = call double @recipe.exp(double %online.new.centered)
%online.old.denominator = call double @recipe.mul(double %online.denominator, double %online.old.scale)
%online.denominator.next = call double @recipe.add(double %online.old.denominator, double %online.new.scale)
%online.old.weighted = call double @recipe.mul(double %online.weighted, double %online.old.scale)
%online.new.weighted = call double @recipe.mul(double %online.dp, double %online.new.scale)
%online.weighted.next = call double @recipe.add(double %online.old.weighted, double %online.new.weighted)
%online.next = add nuw i32 %online.key, 1 br label %online.statistics.loop online.statistics.done:
%online.mean = call double @recipe.div(double %online.weighted, double %online.denominator) br label %key.loop key.loop:
%key = phi i32 [ 0, %online.statistics.done ], [ %key.next, %key.channel.done ] %key.more = icmp ult i32 %key, %length
%key.head.start = mul i32 %head, %head.width %key.row.base = mul i32 %row, %from
br i1 %key.more, label %key.prepare, label %query.done key.prepare:
%key.score = call double @attention_score( ptr addrspace(1) %input, i32 %plane, i32 %row, i32 %head, i32 %query,
i32 %key, i32 %from, i32 %length, i32 %head.width, double %scale )
%key.centered = call double @recipe.sub(double %key.score, double %online.maximum)
%key.exponential = call double @recipe.exp(double %key.centered)
%key.probability = call double @recipe.div(double %key.exponential, double %online.denominator) br label %key.dp.loop key.dp.loop:
%key.dp.channel = phi i32 [ 0, %key.prepare ], [ %key.dp.next, %key.dp.step ]
%key.dp = phi double [ 0.0, %key.prepare ], [ %key.dp.sum, %key.dp.step ]
%key.dp.more = icmp ult i32 %key.dp.channel, %head.width
br i1 %key.dp.more, label %key.dp.step, label %key.channel.entry
key.dp.step: %key.channel = add i32 %key.head.start, %key.dp.channel %key.channel.base = mul i32 %key.channel, %length
%key.dp.delta.local = add i32 %key.channel.base, %query %key.value.local = add i32 %key.channel.base, %key
%key.dp.delta.index = add i32 %key.row.base, %key.dp.delta.local
%key.input.stride = mul i32 %from, 3 %key.input.row = mul i32 %row, %key.input.stride
%key.value.plane = mul i32 %from, 2 %key.value.row = add i32 %key.input.row, %key.value.plane
%key.value.index = add i32 %key.value.row, %key.value.local
%key.dp.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %key.dp.delta.index
%key.value.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %key.value.index
%key.dp.delta = load double, ptr addrspace(1) %key.dp.delta.ptr, align 8
%key.value = load double, ptr addrspace(1) %key.value.ptr, align 8
%key.dp.product = call double @recipe.mul(double %key.dp.delta, double %key.value)
%key.dp.sum = call double @recipe.add(double %key.dp, double %key.dp.product) %key.dp.next = add nuw i32 %key.dp.channel, 1 br label %key.dp.loop
key.channel.entry: %key.dp.centered = call double @recipe.sub(double %key.dp, double %online.mean)
%ds = call double @recipe.mul(double %key.probability, double %key.dp.centered) br label %key.channel.loop key.channel.loop:
%key.channel.offset = phi i32 [ 0, %key.channel.entry ], [ %key.channel.next, %key.channel.step ]
%key.channel.more = icmp ult i32 %key.channel.offset, %head.width
br i1 %key.channel.more, label %key.channel.step, label %key.channel.done key.channel.step:
%update.channel = add i32 %key.head.start, %key.channel.offset %update.channel.base = mul i32 %update.channel, %length
%query.local = add i32 %update.channel.base, %query %key.local = add i32 %update.channel.base, %key
%update.input.stride = mul i32 %from, 3 %update.input.row = mul i32 %row, %update.input.stride
%query.index = add i32 %update.input.row, %query.local %key.plane = add i32 %update.input.row, %from
%key.index = add i32 %key.plane, %key.local %value.plane = mul i32 %from, 2
%value.row = add i32 %update.input.row, %value.plane %value.index = add i32 %value.row, %key.local
%query.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %query.index
%key.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %key.index
%update.delta.index = add i32 %key.row.base, %query.local
%update.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %update.delta.index
%query.value = load double, ptr addrspace(1) %query.ptr, align 8
%key.value.current = load double, ptr addrspace(1) %key.ptr, align 8
%update.delta = load double, ptr addrspace(1) %update.delta.ptr, align 8 %dq.raw = call double @recipe.mul(double %ds, double %key.value.current)
%dq = call double @recipe.div(double %dq.raw, double %scale) %dk.raw = call double @recipe.mul(double %ds, double %query.value) %dk = call double @recipe.div(double %dk.raw, double %scale)
%dv = call double @recipe.mul(double %key.probability, double %update.delta)
%dq.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %query.index
%dk.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %key.index
%dv.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %value.index
%dq.old = call double @recipe.atomic.add(ptr addrspace(1) %dq.ptr, double %dq)
%dk.old = call double @recipe.atomic.add(ptr addrspace(1) %dk.ptr, double %dk)
%dv.old = call double @recipe.atomic.add(ptr addrspace(1) %dv.ptr, double %dv)
%key.channel.next = add nuw i32 %key.channel.offset, 1
br label %key.channel.loop key.channel.done: %key.next = add nuw i32 %key, 1 br label %key.loop query.done:
%query.next = add nuw i32 %query, 1 br label %query.loop head.done: %head.next = add nuw i32 %head, 1
br label %head.loop row.done: %row.next = add i32 %row, %threads br label %row.loop rows.done: ret void }
define internal void @scan_forward_body( ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output,
ptr addrspace(1) %context, i32 %rows, i32 %in.channels, i32 %length, i32 %out.channels, i32 %gates,
i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry: %tid = call i32 @llvm.amdgcn.workitem.id.x()
%in.elements = mul i32 %in.channels, %length
%out.elements = mul i32 %out.channels, %length %input.matrix = mul i32 %in.channels, %out.channels
%state.matrix = mul i32 %out.channels, %out.channels %matrix.span = add i32 %input.matrix, %state.matrix
%gate.stride = add i32 %matrix.span, %out.channels %gate.batch = mul i32 %rows, %out.elements
br label %precompute.loop precompute.loop:
%precompute.gate = phi i32 [ 0, %entry ], [ %precompute.next, %precompute.step ]
%precompute.more = icmp ult i32 %precompute.gate, %gates
br i1 %precompute.more, label %precompute.step, label %precompute.done precompute.step:
%precompute.weight.offset = mul i32 %precompute.gate, %gate.stride
%precompute.weights = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %precompute.weight.offset
%precompute.context.offset = mul i32 %precompute.gate, %gate.batch
%precompute.context = getelementptr inbounds double, ptr addrspace(1) %context, i32 %precompute.context.offset
call void @contraction_forward_body( ptr addrspace(1) %input, ptr addrspace(1) %precompute.weights,
ptr addrspace(1) %precompute.context, i32 %rows, i32 %in.channels, i32 %length, i32 %out.channels,
i32 %length, i32 0, i1 false, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )
%precompute.next = add i32 %precompute.gate, 1 br label %precompute.loop precompute.done:
call void @llvm.amdgcn.s.barrier() br label %row.loop row.loop:
%row = phi i32 [ %tid, %precompute.done ], [ %row.next, %time.done ] %row.more = icmp ult i32 %row, %rows
br i1 %row.more, label %time.loop, label %exit time.loop: %time = phi i32 [ 0, %row.loop ], [ %time.next, %output.done ]
%previous.exists = icmp ne i32 %time, 0 %output.row.base = mul i32 %row, %out.elements
%time.more = icmp ult i32 %time, %length br i1 %time.more, label %gate.loop, label %time.done gate.loop:
%gate = phi i32 [ 0, %time.loop ], [ %gate.next, %hidden.done ] %gate.more = icmp ult i32 %gate, %gates
br i1 %gate.more, label %hidden.loop, label %output.loop hidden.loop:
%hidden = phi i32 [ 0, %gate.loop ], [ %hidden.next, %gate.store ] %gate.weight.base = mul i32 %gate, %gate.stride
%hidden.more = icmp ult i32 %hidden, %out.channels br i1 %hidden.more, label %input.load, label %hidden.done
input.load: %input.gate.base = mul i32 %gate, %gate.batch %input.hidden.base = mul i32 %hidden, %length
%input.local = add i32 %input.hidden.base, %time %input.row.local = add i32 %output.row.base, %input.local
%input.index = add i32 %input.gate.base, %input.row.local
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %input.index
%input.sum = load double, ptr addrspace(1) %input.ptr, align 8 br label %state.sum.loop state.sum.loop:
%state.channel = phi i32 [ 0, %input.load ], [ %state.next, %state.sum.step ]
%state.sum = phi double [ %input.sum, %input.load ], [ %state.sum.next, %state.sum.step ]
%state.more = icmp ult i32 %state.channel, %out.channels br i1 %state.more, label %state.sum.step, label %gate.activate
state.sum.step: %previous.time = sub i32 %time, 1 %previous.safe = select i1 %previous.exists, i32 %previous.time, i32 0
%state.channel.base = mul i32 %state.channel, %length %previous.local = add i32 %state.channel.base, %previous.safe
%previous.index = add i32 %output.row.base, %previous.local
%previous.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %previous.index
%previous.loaded = load double, ptr addrspace(1) %previous.ptr, align 8
%previous = select i1 %previous.exists, double %previous.loaded, double 0.0 %candidate.gate = icmp eq i32 %gate, 2
%gru = icmp eq i32 %gates, 3 %reset.candidate = and i1 %gru, %candidate.gate
%reset.channel.base = mul i32 %state.channel, %length %reset.local = add i32 %reset.channel.base, %time
%reset.row.index = add i32 %output.row.base, %reset.local %reset.base = add i32 %gate.batch, %reset.row.index
%reset.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %reset.base
%reset = load double, ptr addrspace(1) %reset.ptr, align 8 %reset.state = call double @recipe.mul(double %reset, double %previous)
%state.value = select i1 %reset.candidate, double %reset.state, double %previous
%state.weight.base = add i32 %gate.weight.base, %input.matrix %state.weight.row = mul i32 %state.channel, %out.channels
%state.weight.local = add i32 %state.weight.row, %hidden
%state.weight.index = add i32 %state.weight.base, %state.weight.local
%state.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %state.weight.index
%state.weight = load double, ptr addrspace(1) %state.weight.ptr, align 8
%state.product = call double @recipe.mul(double %state.value, double %state.weight) %state.sum.next = call double @recipe.add(double %state.sum, double %state.product)
%state.next = add nuw i32 %state.channel, 1 br label %state.sum.loop gate.activate:
%bias.base = add i32 %gate.weight.base, %matrix.span %bias.index = add i32 %bias.base, %hidden
%bias.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %bias.index
%bias = load double, ptr addrspace(1) %bias.ptr, align 8 %linear = call double @recipe.add(double %state.sum, double %bias)
%rnn = icmp eq i32 %gates, 1 %last.gate = sub i32 %gates, 1 %candidate = icmp eq i32 %gate, %last.gate
%use.tanh = or i1 %rnn, %candidate %tanh.value = call double @recipe.tanh(double %linear)
%sigmoid.value = call double @sigmoid(double %linear)
%gate.value = select i1 %use.tanh, double %tanh.value, double %sigmoid.value br label %gate.store gate.store:
%gate.context.base = mul i32 %gate, %gate.batch %gate.hidden.base = mul i32 %hidden, %length
%gate.local = add i32 %gate.hidden.base, %time %gate.row.local = add i32 %output.row.base, %gate.local
%gate.index = add i32 %gate.context.base, %gate.row.local
%gate.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate.index
store double %gate.value, ptr addrspace(1) %gate.ptr, align 8 %hidden.next = add nuw i32 %hidden, 1
br label %hidden.loop hidden.done: %gate.next = add nuw i32 %gate, 1 br label %gate.loop output.loop:
%output.hidden = phi i32 [ 0, %gate.loop ], [ %output.next, %output.store ]
%output.more = icmp ult i32 %output.hidden, %out.channels br i1 %output.more, label %output.step, label %output.done
output.step: %output.hidden.base = mul i32 %output.hidden, %length %output.local = add i32 %output.hidden.base, %time
%output.index = add i32 %output.row.base, %output.local
%gate0.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %output.index
%gate0 = load double, ptr addrspace(1) %gate0.ptr, align 8
%is.gru = icmp eq i32 %gates, 3 %is.lstm = icmp eq i32 %gates, 4
%gate1.raw = add i32 %gate.batch, %output.index %gate1.index = select i1 %is.lstm, i32 %gate1.raw, i32 %output.index
%gate1.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate1.index
%gate1 = load double, ptr addrspace(1) %gate1.ptr, align 8 %gate2.base = mul i32 %gate.batch, 2
%gate2.raw = add i32 %gate2.base, %output.index %gate2.valid = or i1 %is.gru, %is.lstm
%gate2.index = select i1 %gate2.valid, i32 %gate2.raw, i32 %output.index
%gate2.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate2.index
%gate2 = load double, ptr addrspace(1) %gate2.ptr, align 8 %gate3.base = mul i32 %gate.batch, 3
%gate3.raw = add i32 %gate3.base, %output.index %gate3.index = select i1 %is.lstm, i32 %gate3.raw, i32 %output.index
%gate3.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate3.index
%gate3 = load double, ptr addrspace(1) %gate3.ptr, align 8 %output.previous.time = sub i32 %time, 1
%output.previous.safe = select i1 %previous.exists, i32 %output.previous.time, i32 0
%output.previous.local = add i32 %output.hidden.base, %output.previous.safe
%output.previous.index = add i32 %output.row.base, %output.previous.local
%output.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %output.previous.index
%output.previous.loaded = load double, ptr addrspace(1) %output.previous.ptr, align 8
%output.previous = select i1 %previous.exists, double %output.previous.loaded, double 0.0
%one.update = call double @recipe.sub(double 1.0, double %gate0) %gru.old = call double @recipe.mul(double %gate0, double %output.previous)
%gru.new = call double @recipe.mul(double %one.update, double %gate2) %gru.value = call double @recipe.add(double %gru.old, double %gru.new)
%cell.base = mul i32 %gate.batch, %gates %cell.index = add i32 %cell.base, %output.index
%cell.previous.index = add i32 %cell.base, %output.previous.index
%cell.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %cell.previous.index
%cell.previous.loaded = load double, ptr addrspace(1) %cell.previous.ptr, align 8
%cell.previous = select i1 %previous.exists, double %cell.previous.loaded, double 0.0
%cell.old = call double @recipe.mul(double %gate1, double %cell.previous) %cell.new = call double @recipe.mul(double %gate0, double %gate3)
%cell = call double @recipe.add(double %cell.old, double %cell.new)
%cell.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %cell.index
store double %cell, ptr addrspace(1) %cell.ptr, align 8 %cell.tanh = call double @recipe.tanh(double %cell)
%lstm.value = call double @recipe.mul(double %gate2, double %cell.tanh)
%rnn.or.gru = select i1 %is.gru, double %gru.value, double %gate0
%output.value = select i1 %is.lstm, double %lstm.value, double %rnn.or.gru br label %output.store output.store:
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %output.index
store double %output.value, ptr addrspace(1) %output.ptr, align 8 %output.next = add nuw i32 %output.hidden, 1
br label %output.loop output.done: %time.next = add nuw i32 %time, 1 br label %time.loop time.done:
%row.next = add i32 %row, %threads br label %row.loop exit: ret void } define internal void @contraction_reverse_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %delta,
ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.input,
i1 %has.bias,
i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels, i32 %out.length, i32 %kernel, i32 %offset,
i32 %threads ) #1 { entry: %tid = call i32 @llvm.amdgcn.workitem.id.x()
%lid = call i32 @recipe.local.id.x() %group = call i32 @recipe.group.id.x()
%block = call i32 @recipe.workgroup.size.x() %groups = udiv i32 %threads, %block
%in.elements = mul i32 %in.channels, %in.length
%out.elements = mul i32 %out.channels, %out.length %is.conv = icmp ne i32 %kernel, 0
%span = select i1 %is.conv, i32 %kernel, i32 1 %window = mul i32 %in.channels, %span
%terms.adjusted = add i32 %window, %block %terms.numerator = sub i32 %terms.adjusted, 1
%term.tiles = udiv i32 %terms.numerator, %block %jobs = mul i32 %out.channels, %term.tiles
br label %gradient.job.loop gradient.job.loop:
%job = phi i32 [ %group, %entry ], [ %job.next, %gradient.job.done ] %job.more = icmp ult i32 %job, %jobs
br i1 %job.more, label %gradient.job.step, label %bias.test gradient.job.step:
%filter = udiv i32 %job, %term.tiles %term.tile = urem i32 %job, %term.tiles
%term.base = mul i32 %term.tile, %block %term.raw = add i32 %term.base, %lid
%active = icmp ult i32 %term.raw, %window %gradient.term = select i1 %active, i32 %term.raw, i32 0
%channel = udiv i32 %gradient.term, %span %kernel.position = urem i32 %gradient.term, %span
%loader = icmp eq i32 %lid, 0 br i1 %loader, label %gradient.initial.load, label %gradient.initial.done
gradient.initial.load: %initial.filter.base = mul i32 %filter, %out.length
%initial.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %initial.filter.base
%initial.delta = load double, ptr addrspace(1) %initial.delta.ptr, align 8
%initial.tile.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 0
store double %initial.delta, ptr addrspace(3) %initial.tile.ptr, align 8 br label %gradient.initial.done
gradient.initial.done: call void @recipe.local.barrier() br label %gradient.item.loop gradient.item.loop:
%row = phi i32 [ 0, %gradient.initial.done ], [ %next.row, %gradient.tile.done ]
%position = phi i32 [ 0, %gradient.initial.done ], [ %next.position, %gradient.tile.done ]
%buffer = phi i32 [ 0, %gradient.initial.done ], [ %next.buffer, %gradient.tile.done ]
%sum = phi double [ 0.0, %gradient.initial.done ], [ %sum.next, %gradient.tile.done ]
%row.more = icmp ult i32 %row, %rows br i1 %row.more, label %gradient.preload, label %gradient.store
gradient.preload: %next.position.raw = add i32 %position, 1 %next.wrap = icmp eq i32 %next.position.raw, %out.length
%next.position = select i1 %next.wrap, i32 0, i32 %next.position.raw
%next.row.raw = add i32 %row, 1 %next.row = select i1 %next.wrap, i32 %next.row.raw, i32 %row
%next.more = icmp ult i32 %next.row, %rows %next.buffer = xor i32 %buffer, 1
%next.loader = and i1 %loader, %next.more br i1 %next.loader, label %gradient.next.load, label %gradient.compute
gradient.next.load: %next.delta.row.base = mul i32 %next.row, %out.elements
%next.delta.filter.base = mul i32 %filter, %out.length
%next.delta.local = add i32 %next.delta.filter.base, %next.position
%next.delta.index = add i32 %next.delta.row.base, %next.delta.local
%next.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %next.delta.index
%next.delta = load double, ptr addrspace(1) %next.delta.ptr, align 8 %next.tile.ptr = getelementptr [0 x double],
ptr addrspace(3) @contraction_tile, i32 0, i32 %next.buffer
store double %next.delta, ptr addrspace(3) %next.tile.ptr, align 8 br label %gradient.compute gradient.compute:
%input.row.base = mul i32 %row, %in.elements
%input.value = call double @contraction_input( ptr addrspace(1) %input, i32 %input.row.base, i32 %position,
i32 %gradient.term, i32 %span, i32 %in.length, i1 %is.conv ) %tile.ptr = getelementptr [0 x double],
ptr addrspace(3) @contraction_tile, i32 0, i32 %buffer
%delta.value = load double, ptr addrspace(3) %tile.ptr, align 8 %product = call double @recipe.mul(double %input.value, double %delta.value)
%candidate = call double @recipe.add(double %sum, double %product) %sum.next = select i1 %active, double %candidate, double %sum
br label %gradient.tile.done gradient.tile.done: call void @recipe.local.barrier()
br label %gradient.item.loop gradient.store: br i1 %active, label %gradient.write, label %gradient.job.done
gradient.write: %p.0 = mul i32 %filter, %window %p = add i32 %p.0, %gradient.term %gradient.index = add i32 %offset, %p
%gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.index
store double %sum, ptr addrspace(1) %gradient.ptr, align 8 br label %gradient.job.done gradient.job.done:
%job.next = add i32 %job, %groups br label %gradient.job.loop
bias.test: br i1 %has.bias, label %bias.loop, label %previous.loop bias.loop:
%bias.filter = phi i32 [ %tid, %bias.test ], [ %bias.next, %bias.store ]
%bias.more = icmp ult i32 %bias.filter, %out.channels
br i1 %bias.more, label %bias.sum.loop, label %previous.loop bias.sum.loop:
%bias.p = phi i32 [ 0, %bias.loop ], [ %bias.p.next, %bias.sum.step ]
%bias.sum = phi double [ 0.0, %bias.loop ], [ %bias.sum.next, %bias.sum.step ]
%bias.count = mul i32 %rows, %out.length %bias.p.more = icmp ult i32 %bias.p, %bias.count
br i1 %bias.p.more, label %bias.sum.step, label %bias.store bias.sum.step:
%bias.row = udiv i32 %bias.p, %out.length %bias.position = urem i32 %bias.p, %out.length
%bias.row.base = mul i32 %bias.row, %out.elements %bias.filter.base = mul i32 %bias.filter, %out.length
%bias.local = add i32 %bias.filter.base, %bias.position %bias.delta.index = add i32 %bias.row.base, %bias.local
%bias.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %bias.delta.index
%bias.delta = load double, ptr addrspace(1) %bias.delta.ptr, align 8
%bias.sum.next = call double @recipe.add(double %bias.sum, double %bias.delta) %bias.p.next = add nuw i32 %bias.p, 1 br label %bias.sum.loop
bias.store: %bias.weight.base = mul i32 %out.channels, %window %bias.weight = add i32 %bias.weight.base, %bias.filter
%bias.gradient.index = add i32 %offset, %bias.weight
%bias.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %bias.gradient.index
store double %bias.sum, ptr addrspace(1) %bias.gradient.ptr, align 8
%bias.next = add i32 %bias.filter, %threads br label %bias.loop
previous.loop: %previous.p = phi i32 [ %tid, %bias.test ], [ %tid, %bias.loop ], [ %previous.next, %previous.done ]
%previous.count = mul i32 %rows, %in.elements %previous.more = icmp ult i32 %previous.p, %previous.count
br i1 %previous.more, label %previous.entry, label %exit previous.entry:
%previous.row = udiv i32 %previous.p, %in.elements %previous.local = urem i32 %previous.p, %in.elements
%previous.channel = udiv i32 %previous.local, %in.length %previous.position = urem i32 %previous.local, %in.length
br label %previous.sum.loop previous.sum.loop:
%term = phi i32 [ 0, %previous.entry ], [ %term.next, %previous.sum.step ]
%previous.sum = phi double [ 0.0, %previous.entry ], [ %previous.sum.next, %previous.sum.step ]
%terms = mul i32 %out.channels, %span %term.more = icmp ult i32 %term, %terms
br i1 %term.more, label %previous.sum.step, label %previous.store previous.sum.step:
%term.filter = udiv i32 %term, %span %term.kernel = urem i32 %term, %span
%position.low = icmp uge i32 %previous.position, %term.kernel
%output.position.raw = sub i32 %previous.position, %term.kernel
%position.high = icmp ult i32 %output.position.raw, %out.length %valid = and i1 %position.low, %position.high
%output.position = select i1 %valid, i32 %output.position.raw, i32 0 %weight.filter.base = mul i32 %term.filter, %window
%weight.channel.base = mul i32 %previous.channel, %span %weight.local.0 = add i32 %weight.channel.base, %term.kernel
%weight.local = add i32 %weight.filter.base, %weight.local.0 %delta.row.base.1 = mul i32 %previous.row, %out.elements
%delta.filter.base.1 = mul i32 %term.filter, %out.length %delta.local = add i32 %delta.filter.base.1, %output.position
%delta.index.1 = add i32 %delta.row.base.1, %delta.local
%weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.local
%delta.ptr.1 = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %delta.index.1
%weight.value = load double, ptr addrspace(1) %weight.ptr, align 8
%delta.value.1 = load double, ptr addrspace(1) %delta.ptr.1, align 8
%term.product = call double @recipe.mul(double %weight.value, double %delta.value.1)
%contribution = select i1 %valid, double %term.product, double 0.0
%previous.sum.next = call double @recipe.add(double %previous.sum, double %contribution) %term.next = add nuw i32 %term, 1
br label %previous.sum.loop previous.store: br i1 %write.input, label %previous.add, label %previous.done previous.add:
%previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %previous.p
%previous.old = load double, ptr addrspace(1) %previous.ptr, align 8
%previous.value = call double @recipe.add(double %previous.old, double %previous.sum)
store double %previous.value, ptr addrspace(1) %previous.ptr, align 8 br label %previous.done previous.done:
%previous.next = add i32 %previous.p, %threads br label %previous.loop exit: ret void }
define internal void @scan_reverse_body( ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output,
ptr addrspace(1) %context, ptr addrspace(1) %delta, ptr addrspace(1) %previous,
ptr addrspace(1) %gradient, i1 %write.input, i32 %rows, i32 %in.channels,
i32 %length, i32 %out.channels, i32 %gates, i32 %parameters, i32 %offset, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x() %in.elements = mul i32 %in.channels, %length
%out.elements = mul i32 %out.channels, %length %batch = mul i32 %rows, %out.elements
%gate.stride.0 = mul i32 %in.channels, %out.channels %state.matrix = mul i32 %out.channels, %out.channels
%gate.stride.1 = add i32 %gate.stride.0, %state.matrix %gate.stride = add i32 %gate.stride.1, %out.channels
%delta.base.factor = add i32 %gates, 1 %delta.base = mul i32 %delta.base.factor, %batch %gate2.batch = mul i32 %batch, 2
%row.gradient.factor = mul i32 %gates, 2 %row.gradient.factor.1 = add i32 %row.gradient.factor, 1
%row.gradient.base = mul i32 %row.gradient.factor.1, %batch %rnn = icmp eq i32 %gates, 1
%gru = icmp eq i32 %gates, 3 %lstm = icmp eq i32 %gates, 4 %simple = or i1 %rnn, %gru
%supported = or i1 %simple, %lstm br i1 %supported, label %row.loop, label %invalid row.loop:
%row = phi i32 [ %tid, %entry ], [ %row.next, %row.done ]
%row.more = icmp ult i32 %row, %rows br i1 %row.more, label %clear.gradient.loop, label %reduce.entry
clear.gradient.loop: %clear.p = phi i32 [ 0, %row.loop ], [ %clear.next, %clear.gradient.step ]
%row.gradient.offset = mul i32 %row, %parameters %row.gradient.start = add i32 %row.gradient.base, %row.gradient.offset
%clear.more = icmp ult i32 %clear.p, %parameters br i1 %clear.more, label %clear.gradient.step, label %clear.state.loop
clear.gradient.step: %clear.index = add i32 %row.gradient.start, %clear.p
%clear.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %clear.index
store double 0.0, ptr addrspace(1) %clear.ptr, align 8 %clear.next = add nuw i32 %clear.p, 1
br label %clear.gradient.loop clear.state.loop:
%clear.h = phi i32 [ 0, %clear.gradient.loop ], [ %clear.h.next, %clear.state.step ]
%scratch.base.0 = mul i32 %rows, %parameters %scratch.base = add i32 %row.gradient.base, %scratch.base.0
%scratch.row = mul i32 %row, %out.channels %dh.start = add i32 %scratch.base, %scratch.row
%dc.base.0 = mul i32 %rows, %out.channels %dc.base = add i32 %scratch.base, %dc.base.0
%dc.start = add i32 %dc.base, %scratch.row %clear.h.more = icmp ult i32 %clear.h, %out.channels
br i1 %clear.h.more, label %clear.state.step, label %time.loop clear.state.step:
%clear.dh.index = add i32 %dh.start, %clear.h %clear.dc.index = add i32 %dc.start, %clear.h
%clear.dh.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %clear.dh.index
%clear.dc.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %clear.dc.index
store double 0.0, ptr addrspace(1) %clear.dh.ptr, align 8 store double 0.0, ptr addrspace(1) %clear.dc.ptr, align 8
%clear.h.next = add nuw i32 %clear.h, 1 br label %clear.state.loop time.loop:
%time = phi i32 [ %length, %clear.state.loop ], [ %time.current, %time.done ] %time.current = sub i32 %time, 1
%row.output.base = mul i32 %row, %out.elements %input.row.base = mul i32 %row, %in.elements
%previous.time = sub i32 %time.current, 1 %previous.exists = icmp sge i32 %previous.time, 0
%previous.safe = select i1 %previous.exists, i32 %previous.time, i32 0 %time.more = icmp sge i32 %time.current, 0
br i1 %time.more, label %scan.mode, label %row.done scan.mode:
br i1 %lstm, label %gate.delta.loop, label %rnn.test rnn.test:
br i1 %rnn, label %rnn.delta.loop, label %gru.delta.loop rnn.delta.loop:
%rnn.hidden = phi i32 [ 0, %rnn.test ], [ %rnn.next, %rnn.delta.step ]
%rnn.more = icmp ult i32 %rnn.hidden, %out.channels
br i1 %rnn.more, label %rnn.delta.step, label %delta.done rnn.delta.step:
%rnn.hidden.base = mul i32 %rnn.hidden, %length %rnn.local = add i32 %rnn.hidden.base, %time.current
%rnn.index = add i32 %row.output.base, %rnn.local %rnn.dy.ptr = getelementptr inbounds double,
ptr addrspace(1) %delta, i32 %rnn.index %rnn.future.index = add i32 %dh.start, %rnn.hidden
%rnn.future.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %rnn.future.index
%rnn.gate.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %rnn.index
%rnn.dy = load double, ptr addrspace(1) %rnn.dy.ptr, align 8
%rnn.future = load double, ptr addrspace(1) %rnn.future.ptr, align 8
%rnn.gate = load double, ptr addrspace(1) %rnn.gate.ptr, align 8 %rnn.dh = call double @recipe.add(double %rnn.dy, double %rnn.future)
%rnn.square = call double @recipe.mul(double %rnn.gate, double %rnn.gate) %rnn.derivative = call double @recipe.sub(double 1.0, double %rnn.square)
%rnn.delta = call double @recipe.mul(double %rnn.dh, double %rnn.derivative) %rnn.delta.index = add i32 %delta.base, %rnn.index
%rnn.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %rnn.delta.index
store double %rnn.delta, ptr addrspace(1) %rnn.delta.ptr, align 8 %rnn.next = add i32 %rnn.hidden, 1
br label %rnn.delta.loop gru.delta.loop: %gru.hidden = phi i32 [ 0, %rnn.test ], [ %gru.next, %gru.delta.step ]
%gru.more = icmp ult i32 %gru.hidden, %out.channels
br i1 %gru.more, label %gru.delta.step, label %gru.reset.loop gru.delta.step:
%gru.hidden.base = mul i32 %gru.hidden, %length %gru.local = add i32 %gru.hidden.base, %time.current
%gru.index = add i32 %row.output.base, %gru.local %gru.previous.local = add i32 %gru.hidden.base, %previous.safe
%gru.previous.index = add i32 %row.output.base, %gru.previous.local
%gru.dy.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %gru.index
%gru.future.index = add i32 %dh.start, %gru.hidden
%gru.future.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.future.index
%gru.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %gru.previous.index
%gru.z.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.index
%gru.n.index = add i32 %gru.index, %gate2.batch
%gru.n.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.n.index
%gru.dy = load double, ptr addrspace(1) %gru.dy.ptr, align 8
%gru.future = load double, ptr addrspace(1) %gru.future.ptr, align 8
%gru.previous.loaded = load double, ptr addrspace(1) %gru.previous.ptr, align 8
%gru.previous = select i1 %previous.exists, double %gru.previous.loaded, double 0.0
%gru.z = load double, ptr addrspace(1) %gru.z.ptr, align 8
%gru.n = load double, ptr addrspace(1) %gru.n.ptr, align 8 %gru.dh = call double @recipe.add(double %gru.dy, double %gru.future)
%gru.one.z = call double @recipe.sub(double 1.0, double %gru.z) %gru.z.difference = call double @recipe.sub(double %gru.previous, double %gru.n)
%gru.dz.0 = call double @recipe.mul(double %gru.dh, double %gru.z.difference) %gru.dz.1 = call double @recipe.mul(double %gru.dz.0, double %gru.z)
%gru.dz = call double @recipe.mul(double %gru.dz.1, double %gru.one.z) %gru.n.square = call double @recipe.mul(double %gru.n, double %gru.n)
%gru.n.derivative = call double @recipe.sub(double 1.0, double %gru.n.square) %gru.dn.0 = call double @recipe.mul(double %gru.dh, double %gru.one.z)
%gru.dn = call double @recipe.mul(double %gru.dn.0, double %gru.n.derivative) %gru.dz.index = add i32 %delta.base, %gru.index
%gru.dn.index.0 = add i32 %delta.base, %gate2.batch %gru.dn.index = add i32 %gru.dn.index.0, %gru.index
%gru.dz.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.dz.index
%gru.dn.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.dn.index
store double %gru.dz, ptr addrspace(1) %gru.dz.ptr, align 8
store double %gru.dn, ptr addrspace(1) %gru.dn.ptr, align 8 %gru.next = add i32 %gru.hidden, 1
br label %gru.delta.loop gru.reset.loop:
%gru.source = phi i32 [ 0, %gru.delta.loop ], [ %gru.source.next, %gru.reset.store ]
%gru.source.more = icmp ult i32 %gru.source, %out.channels
br i1 %gru.source.more, label %gru.reset.sum.loop, label %delta.done gru.reset.sum.loop:
%gru.target = phi i32 [ 0, %gru.reset.loop ], [ %gru.target.next, %gru.reset.sum.step ]
%gru.reset.sum = phi double [ 0.0, %gru.reset.loop ], [ %gru.reset.sum.next, %gru.reset.sum.step ]
%gru.target.more = icmp ult i32 %gru.target, %out.channels
br i1 %gru.target.more, label %gru.reset.sum.step, label %gru.reset.store gru.reset.sum.step:
%gru.candidate.base = mul i32 %gate.stride, 2 %gru.candidate.state = add i32 %gru.candidate.base, %gate.stride.0
%gru.weight.row = mul i32 %gru.source, %out.channels %gru.weight.local = add i32 %gru.weight.row, %gru.target
%gru.weight.index = add i32 %gru.candidate.state, %gru.weight.local
%gru.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %gru.weight.index
%gru.target.base = mul i32 %gru.target, %length %gru.target.local = add i32 %gru.target.base, %time.current
%gru.target.index = add i32 %row.output.base, %gru.target.local %gru.target.delta.0 = add i32 %delta.base, %gate2.batch
%gru.target.delta.index = add i32 %gru.target.delta.0, %gru.target.index
%gru.target.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.target.delta.index
%gru.weight = load double, ptr addrspace(1) %gru.weight.ptr, align 8
%gru.target.delta = load double, ptr addrspace(1) %gru.target.delta.ptr, align 8
%gru.reset.product = call double @recipe.mul(double %gru.weight, double %gru.target.delta)
%gru.reset.sum.next = call double @recipe.add(double %gru.reset.sum, double %gru.reset.product)
%gru.target.next = add i32 %gru.target, 1 br label %gru.reset.sum.loop gru.reset.store:
%gru.source.base = mul i32 %gru.source, %length %gru.source.local = add i32 %gru.source.base, %time.current
%gru.source.index = add i32 %row.output.base, %gru.source.local
%gru.source.previous.local = add i32 %gru.source.base, %previous.safe
%gru.source.previous.index = add i32 %row.output.base, %gru.source.previous.local
%gru.source.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %gru.source.previous.index
%gru.r.index = add i32 %batch, %gru.source.index
%gru.r.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.r.index
%gru.source.previous.loaded = load double, ptr addrspace(1) %gru.source.previous.ptr, align 8
%gru.source.previous = select i1 %previous.exists, double %gru.source.previous.loaded, double 0.0
%gru.r = load double, ptr addrspace(1) %gru.r.ptr, align 8
%gru.dr = call double @recipe.mul(double %gru.reset.sum, double %gru.source.previous) %gru.one.r = call double @recipe.sub(double 1.0, double %gru.r)
%gru.dr.0 = call double @recipe.mul(double %gru.dr, double %gru.r) %gru.dr.1 = call double @recipe.mul(double %gru.dr.0, double %gru.one.r)
%gru.dr.base = add i32 %delta.base, %batch %gru.dr.index = add i32 %gru.dr.base, %gru.source.index
%gru.dr.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.dr.index
store double %gru.dr.1, ptr addrspace(1) %gru.dr.ptr, align 8 %gru.source.next = add i32 %gru.source, 1
br label %gru.reset.loop gate.delta.loop: %hidden = phi i32 [ 0, %scan.mode ], [ %hidden.next, %gate.delta.step ]
%hidden.more = icmp ult i32 %hidden, %out.channels br i1 %hidden.more, label %gate.delta.step, label %delta.done
gate.delta.step: %hidden.base = mul i32 %hidden, %length %local = add i32 %hidden.base, %time.current
%index = add i32 %row.output.base, %local %previous.local = add i32 %hidden.base, %previous.safe
%previous.index = add i32 %row.output.base, %previous.local %cell.base = mul i32 %gates, %batch
%cell.index = add i32 %cell.base, %index %cell.previous.index = add i32 %cell.base, %previous.index
%dy.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %index %dh.index = add i32 %dh.start, %hidden
%dc.index = add i32 %dc.start, %hidden %dh.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dh.index
%dc.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dc.index
%cell.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %cell.index
%cell.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %cell.previous.index
%dy = load double, ptr addrspace(1) %dy.ptr, align 8 %dh.future = load double, ptr addrspace(1) %dh.ptr, align 8
%dc.future = load double, ptr addrspace(1) %dc.ptr, align 8 %cell = load double, ptr addrspace(1) %cell.ptr, align 8
%cell.previous.loaded = load double, ptr addrspace(1) %cell.previous.ptr, align 8
%cell.previous = select i1 %previous.exists, double %cell.previous.loaded, double 0.0
%i.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %index %f.index = add i32 %batch, %index
%o.index = add i32 %f.index, %batch %g.index = add i32 %o.index, %batch
%f.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %f.index
%o.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %o.index
%g.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %g.index
%i = load double, ptr addrspace(1) %i.ptr, align 8 %f = load double, ptr addrspace(1) %f.ptr, align 8
%o = load double, ptr addrspace(1) %o.ptr, align 8 %g = load double, ptr addrspace(1) %g.ptr, align 8
%dh = call double @recipe.add(double %dy, double %dh.future) %cell.tanh = call double @recipe.tanh(double %cell)
%cell.tanh.square = call double @recipe.mul(double %cell.tanh, double %cell.tanh) %cell.tanh.derivative = call double @recipe.sub(double 1.0, double %cell.tanh.square)
%cell.chain.0 = call double @recipe.mul(double %dh, double %o) %cell.chain = call double @recipe.mul(double %cell.chain.0, double %cell.tanh.derivative)
%dc = call double @recipe.add(double %dc.future, double %cell.chain) %one.o = call double @recipe.sub(double 1.0, double %o) %do.0 = call double @recipe.mul(double %dh, double %cell.tanh)
%do.1 = call double @recipe.mul(double %do.0, double %o) %do = call double @recipe.mul(double %do.1, double %one.o) %one.i = call double @recipe.sub(double 1.0, double %i) %di.0 = call double @recipe.mul(double %dc, double %g)
%di.1 = call double @recipe.mul(double %di.0, double %i) %di = call double @recipe.mul(double %di.1, double %one.i) %one.f = call double @recipe.sub(double 1.0, double %f)
%df.0 = call double @recipe.mul(double %dc, double %cell.previous) %df.1 = call double @recipe.mul(double %df.0, double %f) %df = call double @recipe.mul(double %df.1, double %one.f)
%g.square = call double @recipe.mul(double %g, double %g) %one.g.square = call double @recipe.sub(double 1.0, double %g.square) %dg.0 = call double @recipe.mul(double %dc, double %i)
%dg = call double @recipe.mul(double %dg.0, double %one.g.square) %dc.previous = call double @recipe.mul(double %dc, double %f)
store double %dc.previous, ptr addrspace(1) %dc.ptr, align 8 %delta0.index = add i32 %delta.base, %index
%delta1.index = add i32 %delta0.index, %batch %delta2.index = add i32 %delta1.index, %batch
%delta3.index = add i32 %delta2.index, %batch
%delta0.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %delta0.index
%delta1.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %delta1.index
%delta2.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %delta2.index
%delta3.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %delta3.index
store double %di, ptr addrspace(1) %delta0.ptr, align 8 store double %df, ptr addrspace(1) %delta1.ptr, align 8
store double %do, ptr addrspace(1) %delta2.ptr, align 8 store double %dg, ptr addrspace(1) %delta3.ptr, align 8
%hidden.next = add nuw i32 %hidden, 1 br label %gate.delta.loop delta.done: br label %parameter.loop parameter.loop:
%p = phi i32 [ 0, %delta.done ], [ %p.next, %parameter.advance ] %p.more = icmp ult i32 %p, %parameters
br i1 %p.more, label %parameter.step, label %hidden.gradient.loop parameter.step:
%gate = udiv i32 %p, %gate.stride %within = urem i32 %p, %gate.stride
%input.weight = icmp ult i32 %within, %gate.stride.0
br i1 %input.weight, label %parameter.advance, label %parameter.value parameter.value:
%state.end = add i32 %gate.stride.0, %state.matrix %state.weight = icmp ult i32 %within, %state.end
%state.index = sub i32 %within, %gate.stride.0 %selected.index = select i1 %state.weight, i32 %state.index, i32 0
%source.channel = udiv i32 %selected.index, %out.channels %target.hidden = urem i32 %selected.index, %out.channels
%bias.hidden = sub i32 %within, %state.end %delta.hidden = select i1 %state.weight, i32 %target.hidden, i32 %bias.hidden
%delta.hidden.base = mul i32 %delta.hidden, %length %delta.local = add i32 %delta.hidden.base, %time.current
%delta.row.local = add i32 %row.output.base, %delta.local %delta.gate.base = mul i32 %gate, %batch
%delta.gate.local = add i32 %delta.base, %delta.gate.base %delta.index = add i32 %delta.gate.local, %delta.row.local
%gate.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %delta.index
%gate.delta = load double, ptr addrspace(1) %gate.delta.ptr, align 8
%state.hidden.base = mul i32 %source.channel, %length
%state.local = add i32 %state.hidden.base, %previous.safe %state.index.value = add i32 %row.output.base, %state.local
%state.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %state.index.value
%state.loaded = load double, ptr addrspace(1) %state.ptr, align 8
%state.value = select i1 %previous.exists, double %state.loaded, double 0.0
%candidate.gate = icmp eq i32 %gate, 2 %gru.candidate = and i1 %gru, %candidate.gate
%parameter.reset.local = add i32 %state.hidden.base, %time.current
%parameter.reset.row = add i32 %row.output.base, %parameter.reset.local
%parameter.reset.raw = add i32 %batch, %parameter.reset.row
%parameter.reset.index = select i1 %gru.candidate, i32 %parameter.reset.raw, i32 0
%parameter.reset.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %parameter.reset.index
%parameter.reset = load double, ptr addrspace(1) %parameter.reset.ptr, align 8
%parameter.reset.state = call double @recipe.mul(double %parameter.reset, double %state.value)
%parameter.state = select i1 %gru.candidate, double %parameter.reset.state, double %state.value
%source.value = select i1 %state.weight, double %parameter.state, double 1.0
%contribution = call double @recipe.mul(double %source.value, double %gate.delta) %row.gradient.index = add i32 %row.gradient.start, %p
%row.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %row.gradient.index
%row.gradient.old = load double, ptr addrspace(1) %row.gradient.ptr, align 8
%row.gradient.new = call double @recipe.add(double %row.gradient.old, double %contribution)
store double %row.gradient.new, ptr addrspace(1) %row.gradient.ptr, align 8
br label %parameter.advance parameter.advance:
%p.next = add nuw i32 %p, 1 br label %parameter.loop hidden.gradient.loop:
%state.channel = phi i32 [ 0, %parameter.loop ], [ %state.channel.next, %hidden.gradient.store ]
%state.channel.more = icmp ult i32 %state.channel, %out.channels
br i1 %state.channel.more, label %hidden.gradient.sum.loop, label %time.done hidden.gradient.sum.loop:
%state.term = phi i32 [ 0, %hidden.gradient.loop ], [ %state.term.next, %hidden.gradient.sum.step ]
%state.sum = phi double [ 0.0, %hidden.gradient.loop ], [ %state.sum.next, %hidden.gradient.sum.step ]
%state.terms = mul i32 %gates, %out.channels %state.term.more = icmp ult i32 %state.term, %state.terms
br i1 %state.term.more, label %hidden.gradient.sum.step, label %hidden.gradient.store hidden.gradient.sum.step:
%state.gate = udiv i32 %state.term, %out.channels %state.hidden = urem i32 %state.term, %out.channels
%state.gate.base = mul i32 %state.gate, %gate.stride %state.matrix.base = add i32 %state.gate.base, %gate.stride.0
%state.weight.row = mul i32 %state.channel, %out.channels %state.weight.local = add i32 %state.weight.row, %state.hidden
%state.weight.index = add i32 %state.matrix.base, %state.weight.local
%state.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %state.weight.index
%state.delta.hidden.base = mul i32 %state.hidden, %length
%state.delta.local = add i32 %state.delta.hidden.base, %time.current
%state.delta.row = add i32 %row.output.base, %state.delta.local %state.delta.gate.base = mul i32 %state.gate, %batch
%state.delta.base = add i32 %delta.base, %state.delta.gate.base
%state.delta.index = add i32 %state.delta.base, %state.delta.row
%state.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %state.delta.index
%state.weight.value = load double, ptr addrspace(1) %state.weight.ptr, align 8
%state.delta.value = load double, ptr addrspace(1) %state.delta.ptr, align 8
%state.product = call double @recipe.mul(double %state.weight.value, double %state.delta.value) %state.candidate = icmp eq i32 %state.gate, 2
%state.gru.candidate = and i1 %gru, %state.candidate %state.reset.hidden.base = mul i32 %state.channel, %length
%state.reset.local = add i32 %state.reset.hidden.base, %time.current
%state.reset.row = add i32 %row.output.base, %state.reset.local %state.reset.raw = add i32 %batch, %state.reset.row
%state.reset.index = select i1 %state.gru.candidate, i32 %state.reset.raw, i32 0
%state.reset.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %state.reset.index
%state.reset = load double, ptr addrspace(1) %state.reset.ptr, align 8
%state.reset.product = call double @recipe.mul(double %state.product, double %state.reset)
%state.contribution = select i1 %state.gru.candidate, double %state.reset.product, double %state.product
%state.sum.next = call double @recipe.add(double %state.sum, double %state.contribution) %state.term.next = add nuw i32 %state.term, 1
br label %hidden.gradient.sum.loop hidden.gradient.store: %state.dh.index = add i32 %dh.start, %state.channel
%state.dh.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %state.dh.index
%state.direct.hidden.base = mul i32 %state.channel, %length
%state.direct.local = add i32 %state.direct.hidden.base, %time.current
%state.direct.index = add i32 %row.output.base, %state.direct.local
%state.direct.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %state.direct.index
%state.direct.z.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %state.direct.index
%state.direct.dy = load double, ptr addrspace(1) %state.direct.delta.ptr, align 8
%state.direct.future = load double, ptr addrspace(1) %state.dh.ptr, align 8
%state.direct.z = load double, ptr addrspace(1) %state.direct.z.ptr, align 8
%state.direct.dh = call double @recipe.add(double %state.direct.dy, double %state.direct.future)
%state.direct.raw = call double @recipe.mul(double %state.direct.z, double %state.direct.dh)
%state.direct = select i1 %gru, double %state.direct.raw, double 0.0
%state.total = call double @recipe.add(double %state.sum, double %state.direct)
store double %state.total, ptr addrspace(1) %state.dh.ptr, align 8 %state.channel.next = add nuw i32 %state.channel, 1
br label %hidden.gradient.loop time.done: br label %time.loop row.done: %row.next = add i32 %row, %threads
br label %row.loop reduce.entry: call void @llvm.amdgcn.s.barrier() br label %reduce.loop reduce.loop:
%reduce.p = phi i32 [ %tid, %reduce.entry ], [ %reduce.next, %reduce.store ]
%reduce.more = icmp ult i32 %reduce.p, %parameters br i1 %reduce.more, label %reduce.row.loop, label %projection.entry
reduce.row.loop: %reduce.row = phi i32 [ 0, %reduce.loop ], [ %reduce.row.next, %reduce.row.step ]
%reduce.sum = phi double [ 0.0, %reduce.loop ], [ %reduce.sum.next, %reduce.row.step ]
%reduce.row.more = icmp ult i32 %reduce.row, %rows br i1 %reduce.row.more, label %reduce.row.step, label %reduce.store
reduce.row.step: %reduce.row.offset = mul i32 %reduce.row, %parameters
%reduce.local = add i32 %reduce.row.offset, %reduce.p %reduce.index = add i32 %row.gradient.base, %reduce.local
%reduce.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %reduce.index
%reduce.value = load double, ptr addrspace(1) %reduce.ptr, align 8
%reduce.sum.next = call double @recipe.add(double %reduce.sum, double %reduce.value) %reduce.row.next = add nuw i32 %reduce.row, 1
br label %reduce.row.loop reduce.store: %reduce.gradient.index = add i32 %offset, %reduce.p
%reduce.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %reduce.gradient.index
store double %reduce.sum, ptr addrspace(1) %reduce.gradient.ptr, align 8 %reduce.next = add i32 %reduce.p, %threads
br label %reduce.loop projection.entry: call void @llvm.amdgcn.s.barrier() br label %projection.loop projection.loop:
%projection.gate = phi i32 [ 0, %projection.entry ], [ %projection.next, %projection.step ]
%projection.more = icmp ult i32 %projection.gate, %gates
br i1 %projection.more, label %projection.step, label %exit projection.step:
%projection.weight.offset = mul i32 %projection.gate, %gate.stride
%projection.weights = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %projection.weight.offset
%projection.delta.gate = mul i32 %projection.gate, %batch
%projection.delta.offset = add i32 %delta.base, %projection.delta.gate
%projection.delta = getelementptr inbounds double, ptr addrspace(1) %context, i32 %projection.delta.offset
%projection.gradient.offset = add i32 %offset, %projection.weight.offset
call void @contraction_reverse_body( ptr addrspace(1) %input, ptr addrspace(1) %projection.weights,
ptr addrspace(1) %projection.delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.input, i1 false,
i32 %rows, i32 %in.channels, i32 %length, i32 %out.channels, i32 %length, i32 0,
i32 %projection.gradient.offset, i32 %threads ) %projection.next = add i32 %projection.gate, 1 br label %projection.loop
invalid: call void @llvm.trap() br label %exit exit: ret void } attributes #0 = { nounwind "amdgpu-flat-work-group-size"="1,1024" } attributes #1 = { alwaysinline nounwind }
