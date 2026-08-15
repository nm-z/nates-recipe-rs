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
%value = load double, ptr addrspace(1) %ptr, align 8 ret double %value }
define internal void @contraction_accumulate(ptr addrspace(5) %sums, ptr addrspace(5) %biases, i1 %bias.enable, i1 %lane.active, i32 %output.m.base, i32 %output.n.base, i32 %m.count, i32 %n.count, i32 %k.count) #1 { entry: br label %k.loop k.loop:
%k = phi i32 [ 0, %entry ], [ %k.next, %register.done ] %k.more = icmp ult i32 %k, %k.count br i1 %k.more, label %register.loop, label %exit register.loop:
%register = phi i32 [ 0, %k.loop ], [ %register.next, %register.next.step ] %register.more = icmp ult i32 %register, RECIPE_REGISTER_COUNT br i1 %register.more, label %register.step, label %register.done
register.step: %register.local.m = urem i32 %register, RECIPE_REGISTER_M %register.local.n = udiv i32 %register, RECIPE_REGISTER_M %output.m.raw = add i32 %output.m.base, %register.local.m %output.n.raw = add i32 %output.n.base, %register.local.n
%output.m.valid = icmp ult i32 %output.m.raw, %m.count %output.n.valid = icmp ult i32 %output.n.raw, %n.count %output.valid = and i1 %output.m.valid, %output.n.valid %active = and i1 %lane.active, %output.valid %output.m = select i1 %active, i32 %output.m.raw, i32 0 %output.n = select i1 %active, i32 %output.n.raw, i32 0
%a.row = mul i32 %k, %tile.m %a.index = add i32 %a.row, %output.m %b.base = mul i32 %tile.m, %tile.k %b.row = mul i32 %k, %tile.n %b.local = add i32 %b.row, %output.n %b.index = add i32 %b.base, %b.local
%a.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %a.index %b.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %b.index %a = load double, ptr addrspace(3) %a.ptr, align 8 %b = load double, ptr addrspace(3) %b.ptr, align 8
%sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %register %sum = load double, ptr addrspace(5) %sum.ptr, align 8 %product = call double @recipe.mul(double %a, double %b) %candidate = call double @recipe.add(double %sum, double %product) %next = select i1 %active, double %candidate, double %sum store double %next, ptr addrspace(5) %sum.ptr, align 8
%bias.m = icmp eq i32 %register.local.m, 0 %bias.output = and i1 %active, %bias.m %bias.active = and i1 %bias.enable, %bias.output br i1 %bias.active, label %bias.step, label %register.next.step bias.step:
%bias.ptr = getelementptr [RECIPE_REGISTER_N x double], ptr addrspace(5) %biases, i32 0, i32 %register.local.n %bias = load double, ptr addrspace(5) %bias.ptr, align 8 %bias.next = call double @recipe.add(double %bias, double %b) store double %bias.next, ptr addrspace(5) %bias.ptr, align 8 br label %register.next.step
register.next.step: %register.next = add i32 %register, 1 br label %register.loop register.done: %k.next = add i32 %k, 1 br label %k.loop exit: ret void }
define internal void @contraction_forward_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output, i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels, i32 %out.length, i32 %kernel,
i1 %has.bias, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry:
%sums = alloca [RECIPE_REGISTER_COUNT x double], align 8, addrspace(5) %lid = call i32 @recipe.local.id.x() %group = call i32 @recipe.group.id.x() %block = call i32 @recipe.workgroup.size.x() %groups = udiv i32 %threads, %block
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length %is.conv = icmp ne i32 %kernel, 0 %span = select i1 %is.conv, i32 %kernel, i32 1 %terms = mul i32 %in.channels, %span %m.total = mul i32 %rows, %out.length
%m.short = icmp ult i32 %tile.m, %m.total %m.tile = select i1 %m.short, i32 %tile.m, i32 %m.total %n.short = icmp ult i32 %tile.n, %out.channels %n.tile = select i1 %n.short, i32 %tile.n, i32 %out.channels %k.short = icmp ult i32 %tile.k, %terms %k.tile = select i1 %k.short, i32 %tile.k, i32 %terms
%m.adjusted = add i32 %m.total, %m.tile %m.numerator = sub i32 %m.adjusted, 1 %m.tiles = udiv i32 %m.numerator, %m.tile %n.adjusted = add i32 %out.channels, %n.tile %n.numerator = sub i32 %n.adjusted, 1 %n.tiles = udiv i32 %n.numerator, %n.tile %jobs = mul i32 %m.tiles, %n.tiles br label %job.loop job.loop:
%job = phi i32 [ %group, %entry ], [ %job.next, %job.done ] %job.more = icmp ult i32 %job, %jobs br i1 %job.more, label %job.step, label %exit job.step:
%n.tile.index = udiv i32 %job, %m.tiles %m.tile.index = urem i32 %job, %m.tiles %m.base = mul i32 %m.tile.index, %m.tile %n.base = mul i32 %n.tile.index, %n.tile
%m.remaining = sub i32 %m.total, %m.base %m.partial = icmp ult i32 %m.remaining, %m.tile %m.count = select i1 %m.partial, i32 %m.remaining, i32 %m.tile %n.remaining = sub i32 %out.channels, %n.base %n.partial = icmp ult i32 %n.remaining, %n.tile %n.count = select i1 %n.partial, i32 %n.remaining, i32 %n.tile
%m.lanes.adjusted = add i32 %m.count, RECIPE_REGISTER_M %m.lanes.numerator = sub i32 %m.lanes.adjusted, 1 %m.lanes = udiv i32 %m.lanes.numerator, RECIPE_REGISTER_M %n.lanes.adjusted = add i32 %n.count, RECIPE_REGISTER_N %n.lanes.numerator = sub i32 %n.lanes.adjusted, 1 %n.lanes = udiv i32 %n.lanes.numerator, RECIPE_REGISTER_N
%lanes = mul i32 %m.lanes, %n.lanes %lane.active = icmp ult i32 %lid, %lanes %lane.n.raw = udiv i32 %lid, %m.lanes %lane.m.raw = urem i32 %lid, %m.lanes %lane.n = select i1 %lane.active, i32 %lane.n.raw, i32 0 %lane.m = select i1 %lane.active, i32 %lane.m.raw, i32 0
%output.m.base = mul i32 %lane.m, RECIPE_REGISTER_M %output.n.base = mul i32 %lane.n, RECIPE_REGISTER_N br label %sum.init.loop sum.init.loop:
%sum.init = phi i32 [ 0, %job.step ], [ %sum.init.next, %sum.init.step ] %sum.init.more = icmp ult i32 %sum.init, RECIPE_REGISTER_COUNT br i1 %sum.init.more, label %sum.init.step, label %sum.init.done
sum.init.step: %sum.init.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %sum.init store double 0.0, ptr addrspace(5) %sum.init.ptr, align 8 %sum.init.next = add i32 %sum.init, 1 br label %sum.init.loop sum.init.done: br label %tile.loop tile.loop:
%term.base = phi i32 [ 0, %sum.init.done ], [ %term.next, %tile.done ] %k.remaining = sub i32 %terms, %term.base %k.partial = icmp ult i32 %k.remaining, %k.tile %k.count = select i1 %k.partial, i32 %k.remaining, i32 %k.tile
%a.count = mul i32 %m.count, %k.count %b.count = mul i32 %n.count, %k.count %load.count = add i32 %a.count, %b.count br label %load.loop load.loop:
%load = phi i32 [ %lid, %tile.loop ], [ %load.next, %load.store ] %load.more = icmp ult i32 %load, %load.count br i1 %load.more, label %load.classify, label %load.done load.classify: %load.a = icmp ult i32 %load, %a.count br i1 %load.a, label %load.a.step, label %load.b.step
load.a.step: %a.k = udiv i32 %load, %m.count %a.m = urem i32 %load, %m.count %a.global = add i32 %m.base, %a.m %a.row = udiv i32 %a.global, %out.length %a.position = urem i32 %a.global, %out.length %a.row.base = mul i32 %a.row, %in.elements %a.term = add i32 %term.base, %a.k
%a.value = call double @contraction_input( ptr addrspace(1) %input, i32 %a.row.base, i32 %a.position, i32 %a.term, i32 %span, i32 %in.length, i1 %is.conv ) %a.tile.row = mul i32 %a.k, %tile.m %a.tile.index = add i32 %a.tile.row, %a.m br label %load.store
load.b.step: %b.local = sub i32 %load, %a.count %b.n = udiv i32 %b.local, %k.count %b.k = urem i32 %b.local, %k.count %b.channel = add i32 %n.base, %b.n %b.channel.base = mul i32 %b.channel, %terms %b.term = add i32 %term.base, %b.k
%b.index = add i32 %b.channel.base, %b.term %b.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %b.index %b.value = load double, ptr addrspace(1) %b.ptr, align 8 %b.tile.base = mul i32 %tile.m, %tile.k %b.tile.row = mul i32 %b.k, %tile.n %b.tile.local = add i32 %b.tile.row, %b.n %b.tile.index = add i32 %b.tile.base, %b.tile.local br label %load.store
load.store: %load.value = phi double [ %a.value, %load.a.step ], [ %b.value, %load.b.step ] %load.tile.index = phi i32 [ %a.tile.index, %load.a.step ], [ %b.tile.index, %load.b.step ] %load.tile.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %load.tile.index store double %load.value, ptr addrspace(3) %load.tile.ptr, align 8 %load.next = add i32 %load, %block br label %load.loop load.done:
call void @recipe.local.barrier() call void @contraction_accumulate(ptr addrspace(5) %sums, ptr addrspace(5) null, i1 false, i1 %lane.active, i32 %output.m.base, i32 %output.n.base, i32 %m.count, i32 %n.count, i32 %k.count) call void @recipe.local.barrier()
%term.next = add i32 %term.base, %k.count %term.more = icmp ult i32 %term.next, %terms br i1 %term.more, label %tile.done, label %store.loop tile.done: br label %tile.loop store.loop:
%store.register = phi i32 [ 0, %load.done ], [ %store.register.next, %store.next ] %store.more = icmp ult i32 %store.register, RECIPE_REGISTER_COUNT br i1 %store.more, label %store.test, label %job.done
store.test: %store.register.m = urem i32 %store.register, RECIPE_REGISTER_M %store.register.n = udiv i32 %store.register, RECIPE_REGISTER_M %store.output.m.raw = add i32 %output.m.base, %store.register.m %store.output.n.raw = add i32 %output.n.base, %store.register.n
%store.output.m.valid = icmp ult i32 %store.output.m.raw, %m.count %store.output.n.valid = icmp ult i32 %store.output.n.raw, %n.count %store.output.valid = and i1 %store.output.m.valid, %store.output.n.valid %store.active = and i1 %lane.active, %store.output.valid br i1 %store.active, label %store, label %store.next
store: %store.channel = add i32 %n.base, %store.output.n.raw %store.m.global = add i32 %m.base, %store.output.m.raw %store.position = urem i32 %store.m.global, %out.length %store.row = udiv i32 %store.m.global, %out.length %store.output.row.base = mul i32 %store.row, %out.elements
%store.output.channel.base = mul i32 %store.channel, %out.length %store.output.local = add i32 %store.output.channel.base, %store.position %store.output.index = add i32 %store.output.row.base, %store.output.local %store.output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %store.output.index
%store.bias.base = mul i32 %out.channels, %terms %store.bias.index = add i32 %store.bias.base, %store.channel %store.bias.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %store.bias.index %store.bias = load double, ptr addrspace(1) %store.bias.ptr, align 8 %store.sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %store.register %store.sum = load double, ptr addrspace(5) %store.sum.ptr, align 8
%store.biased = call double @recipe.add(double %store.sum, double %store.bias) %store.result = select i1 %has.bias, double %store.biased, double %store.sum store double %store.result, ptr addrspace(1) %store.output.ptr, align 8 br label %store.next
store.next: %store.register.next = add i32 %store.register, 1 br label %store.loop job.done: %job.next = add i32 %job, %groups br label %job.loop exit: ret void }
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
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.input, i1 %has.bias,
i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels, i32 %out.length, i32 %kernel, i32 %offset, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry:
%sums = alloca [RECIPE_REGISTER_COUNT x double], align 8, addrspace(5) %biases = alloca [RECIPE_REGISTER_N x double], align 8, addrspace(5) %lid = call i32 @recipe.local.id.x() %group = call i32 @recipe.group.id.x() %block = call i32 @recipe.workgroup.size.x() %groups = udiv i32 %threads, %block
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length %is.conv = icmp ne i32 %kernel, 0
%span = select i1 %is.conv, i32 %kernel, i32 1 %window = mul i32 %in.channels, %span
%gradient.r.total = mul i32 %rows, %out.length
%gradient.m.short = icmp ult i32 %tile.m, %window %gradient.m.tile = select i1 %gradient.m.short, i32 %tile.m, i32 %window %gradient.n.short = icmp ult i32 %tile.n, %out.channels %gradient.n.tile = select i1 %gradient.n.short, i32 %tile.n, i32 %out.channels
%gradient.k.short = icmp ult i32 %tile.k, %gradient.r.total %gradient.k.tile = select i1 %gradient.k.short, i32 %tile.k, i32 %gradient.r.total
%gradient.m.adjusted = add i32 %window, %gradient.m.tile %gradient.m.numerator = sub i32 %gradient.m.adjusted, 1 %gradient.m.tiles = udiv i32 %gradient.m.numerator, %gradient.m.tile %gradient.n.adjusted = add i32 %out.channels, %gradient.n.tile %gradient.n.numerator = sub i32 %gradient.n.adjusted, 1 %gradient.n.tiles = udiv i32 %gradient.n.numerator, %gradient.n.tile
%gradient.jobs = mul i32 %gradient.m.tiles, %gradient.n.tiles br label %gradient.job.loop gradient.job.loop:
%gradient.job = phi i32 [ %group, %entry ], [ %gradient.job.next, %gradient.job.done ] %gradient.job.more = icmp ult i32 %gradient.job, %gradient.jobs
br i1 %gradient.job.more, label %gradient.job.step, label %previous.test gradient.job.step:
%gradient.n.index = udiv i32 %gradient.job, %gradient.m.tiles %gradient.m.index = urem i32 %gradient.job, %gradient.m.tiles %gradient.m.base = mul i32 %gradient.m.index, %gradient.m.tile %gradient.n.base = mul i32 %gradient.n.index, %gradient.n.tile
%gradient.m.remaining = sub i32 %window, %gradient.m.base %gradient.m.partial = icmp ult i32 %gradient.m.remaining, %gradient.m.tile %gradient.m.count = select i1 %gradient.m.partial, i32 %gradient.m.remaining, i32 %gradient.m.tile
%gradient.n.remaining = sub i32 %out.channels, %gradient.n.base %gradient.n.partial = icmp ult i32 %gradient.n.remaining, %gradient.n.tile %gradient.n.count = select i1 %gradient.n.partial, i32 %gradient.n.remaining, i32 %gradient.n.tile
%gradient.m.lanes.adjusted = add i32 %gradient.m.count, RECIPE_REGISTER_M %gradient.m.lanes.numerator = sub i32 %gradient.m.lanes.adjusted, 1 %gradient.m.lanes = udiv i32 %gradient.m.lanes.numerator, RECIPE_REGISTER_M %gradient.n.lanes.adjusted = add i32 %gradient.n.count, RECIPE_REGISTER_N %gradient.n.lanes.numerator = sub i32 %gradient.n.lanes.adjusted, 1 %gradient.n.lanes = udiv i32 %gradient.n.lanes.numerator, RECIPE_REGISTER_N
%gradient.lanes = mul i32 %gradient.m.lanes, %gradient.n.lanes %gradient.lane.active = icmp ult i32 %lid, %gradient.lanes %gradient.lane.n.raw = udiv i32 %lid, %gradient.m.lanes %gradient.lane.m.raw = urem i32 %lid, %gradient.m.lanes %gradient.lane.n = select i1 %gradient.lane.active, i32 %gradient.lane.n.raw, i32 0 %gradient.lane.m = select i1 %gradient.lane.active, i32 %gradient.lane.m.raw, i32 0
%gradient.output.m.base = mul i32 %gradient.lane.m, RECIPE_REGISTER_M %gradient.output.n.base = mul i32 %gradient.lane.n, RECIPE_REGISTER_N br label %gradient.sum.init.loop gradient.sum.init.loop:
%gradient.sum.init = phi i32 [ 0, %gradient.job.step ], [ %gradient.sum.init.next, %gradient.sum.init.step ] %gradient.sum.init.more = icmp ult i32 %gradient.sum.init, RECIPE_REGISTER_COUNT br i1 %gradient.sum.init.more, label %gradient.sum.init.step, label %gradient.bias.init.loop
gradient.sum.init.step: %gradient.sum.init.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %gradient.sum.init store double 0.0, ptr addrspace(5) %gradient.sum.init.ptr, align 8 %gradient.sum.init.next = add i32 %gradient.sum.init, 1 br label %gradient.sum.init.loop gradient.bias.init.loop:
%gradient.bias.init = phi i32 [ 0, %gradient.sum.init.loop ], [ %gradient.bias.init.next, %gradient.bias.init.step ] %gradient.bias.init.more = icmp ult i32 %gradient.bias.init, RECIPE_REGISTER_N br i1 %gradient.bias.init.more, label %gradient.bias.init.step, label %gradient.tile.loop
gradient.bias.init.step: %gradient.bias.init.ptr = getelementptr [RECIPE_REGISTER_N x double], ptr addrspace(5) %biases, i32 0, i32 %gradient.bias.init store double 0.0, ptr addrspace(5) %gradient.bias.init.ptr, align 8 %gradient.bias.init.next = add i32 %gradient.bias.init, 1 br label %gradient.bias.init.loop gradient.tile.loop:
%gradient.r.base = phi i32 [ 0, %gradient.bias.init.loop ], [ %gradient.r.next, %gradient.tile.done ]
%gradient.r.remaining = sub i32 %gradient.r.total, %gradient.r.base %gradient.r.partial = icmp ult i32 %gradient.r.remaining, %gradient.k.tile %gradient.r.count = select i1 %gradient.r.partial, i32 %gradient.r.remaining, i32 %gradient.k.tile
%gradient.a.count = mul i32 %gradient.m.count, %gradient.r.count %gradient.b.count = mul i32 %gradient.n.count, %gradient.r.count %gradient.load.count = add i32 %gradient.a.count, %gradient.b.count br label %gradient.load.loop gradient.load.loop:
%gradient.load = phi i32 [ %lid, %gradient.tile.loop ], [ %gradient.load.next, %gradient.load.store ] %gradient.load.more = icmp ult i32 %gradient.load, %gradient.load.count br i1 %gradient.load.more, label %gradient.load.classify, label %gradient.load.done
gradient.load.classify: %gradient.load.a = icmp ult i32 %gradient.load, %gradient.a.count br i1 %gradient.load.a, label %gradient.load.a.step, label %gradient.load.b.step
gradient.load.a.step: %gradient.a.r = udiv i32 %gradient.load, %gradient.m.count %gradient.a.m = urem i32 %gradient.load, %gradient.m.count %gradient.a.global = add i32 %gradient.r.base, %gradient.a.r
%gradient.a.row = udiv i32 %gradient.a.global, %out.length %gradient.a.position = urem i32 %gradient.a.global, %out.length %gradient.a.row.base = mul i32 %gradient.a.row, %in.elements %gradient.a.term = add i32 %gradient.m.base, %gradient.a.m
%gradient.a.value = call double @contraction_input( ptr addrspace(1) %input, i32 %gradient.a.row.base, i32 %gradient.a.position, i32 %gradient.a.term, i32 %span, i32 %in.length, i1 %is.conv )
%gradient.a.tile.row = mul i32 %gradient.a.r, %tile.m %gradient.a.tile.index = add i32 %gradient.a.tile.row, %gradient.a.m br label %gradient.load.store
gradient.load.b.step: %gradient.b.local = sub i32 %gradient.load, %gradient.a.count %gradient.b.r = udiv i32 %gradient.b.local, %gradient.n.count %gradient.b.n = urem i32 %gradient.b.local, %gradient.n.count %gradient.b.global = add i32 %gradient.r.base, %gradient.b.r
%gradient.b.row = udiv i32 %gradient.b.global, %out.length %gradient.b.position = urem i32 %gradient.b.global, %out.length %gradient.b.filter = add i32 %gradient.n.base, %gradient.b.n
%gradient.b.row.base = mul i32 %gradient.b.row, %out.elements %gradient.b.filter.base = mul i32 %gradient.b.filter, %out.length %gradient.b.local.index = add i32 %gradient.b.filter.base, %gradient.b.position %gradient.b.index = add i32 %gradient.b.row.base, %gradient.b.local.index
%gradient.b.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %gradient.b.index %gradient.b.value = load double, ptr addrspace(1) %gradient.b.ptr, align 8 %gradient.b.tile.base = mul i32 %tile.m, %tile.k %gradient.b.tile.row = mul i32 %gradient.b.r, %tile.n
%gradient.b.tile.local = add i32 %gradient.b.tile.row, %gradient.b.n %gradient.b.tile.index = add i32 %gradient.b.tile.base, %gradient.b.tile.local br label %gradient.load.store
gradient.load.store: %gradient.load.value = phi double [ %gradient.a.value, %gradient.load.a.step ], [ %gradient.b.value, %gradient.load.b.step ] %gradient.load.index = phi i32 [ %gradient.a.tile.index, %gradient.load.a.step ], [ %gradient.b.tile.index, %gradient.load.b.step ]
%gradient.load.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %gradient.load.index store double %gradient.load.value, ptr addrspace(3) %gradient.load.ptr, align 8
%gradient.load.next = add i32 %gradient.load, %block br label %gradient.load.loop gradient.load.done: call void @recipe.local.barrier()
%gradient.bias.m.base = add i32 %gradient.m.base, %gradient.output.m.base %gradient.bias.first = icmp eq i32 %gradient.bias.m.base, 0 %gradient.bias.enable = and i1 %has.bias, %gradient.bias.first call void @contraction_accumulate(ptr addrspace(5) %sums, ptr addrspace(5) %biases, i1 %gradient.bias.enable, i1 %gradient.lane.active, i32 %gradient.output.m.base, i32 %gradient.output.n.base, i32 %gradient.m.count, i32 %gradient.n.count, i32 %gradient.r.count) call void @recipe.local.barrier()
%gradient.r.next = add i32 %gradient.r.base, %gradient.r.count %gradient.r.more = icmp ult i32 %gradient.r.next, %gradient.r.total br i1 %gradient.r.more, label %gradient.tile.done, label %gradient.store.loop gradient.tile.done: br label %gradient.tile.loop gradient.store.loop:
%gradient.store.register = phi i32 [ 0, %gradient.load.done ], [ %gradient.store.register.next, %gradient.store.next ] %gradient.store.more = icmp ult i32 %gradient.store.register, RECIPE_REGISTER_COUNT br i1 %gradient.store.more, label %gradient.store.test, label %gradient.job.done
gradient.store.test: %gradient.store.register.m = urem i32 %gradient.store.register, RECIPE_REGISTER_M %gradient.store.register.n = udiv i32 %gradient.store.register, RECIPE_REGISTER_M %gradient.store.output.m.raw = add i32 %gradient.output.m.base, %gradient.store.register.m %gradient.store.output.n.raw = add i32 %gradient.output.n.base, %gradient.store.register.n
%gradient.store.output.m.valid = icmp ult i32 %gradient.store.output.m.raw, %gradient.m.count %gradient.store.output.n.valid = icmp ult i32 %gradient.store.output.n.raw, %gradient.n.count %gradient.store.output.valid = and i1 %gradient.store.output.m.valid, %gradient.store.output.n.valid %gradient.store.active = and i1 %gradient.lane.active, %gradient.store.output.valid br i1 %gradient.store.active, label %gradient.store, label %gradient.store.next
gradient.store: %gradient.store.filter = add i32 %gradient.n.base, %gradient.store.output.n.raw %gradient.store.term = add i32 %gradient.m.base, %gradient.store.output.m.raw %gradient.store.filter.base = mul i32 %gradient.store.filter, %window %gradient.store.local = add i32 %gradient.store.filter.base, %gradient.store.term %gradient.store.index = add i32 %offset, %gradient.store.local
%gradient.store.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.store.index %gradient.store.sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %gradient.store.register %gradient.store.sum = load double, ptr addrspace(5) %gradient.store.sum.ptr, align 8 store double %gradient.store.sum, ptr addrspace(1) %gradient.store.ptr, align 8
%gradient.store.bias.term = icmp eq i32 %gradient.store.term, 0 %gradient.store.bias.active = and i1 %has.bias, %gradient.store.bias.term br i1 %gradient.store.bias.active, label %gradient.bias.store, label %gradient.store.next gradient.bias.store:
%gradient.store.bias.base = mul i32 %out.channels, %window %gradient.store.bias.local = add i32 %gradient.store.bias.base, %gradient.store.filter %gradient.store.bias.index = add i32 %offset, %gradient.store.bias.local %gradient.store.bias.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.store.bias.index
%gradient.store.bias.value.ptr = getelementptr [RECIPE_REGISTER_N x double], ptr addrspace(5) %biases, i32 0, i32 %gradient.store.register.n %gradient.store.bias.value = load double, ptr addrspace(5) %gradient.store.bias.value.ptr, align 8 store double %gradient.store.bias.value, ptr addrspace(1) %gradient.store.bias.ptr, align 8 br label %gradient.store.next
gradient.store.next: %gradient.store.register.next = add i32 %gradient.store.register, 1 br label %gradient.store.loop gradient.job.done: %gradient.job.next = add i32 %gradient.job, %groups br label %gradient.job.loop
previous.test: br i1 %write.input, label %previous.entry, label %exit previous.entry:
%previous.m.total = mul i32 %rows, %in.length %previous.r.total = mul i32 %out.channels, %span
%previous.m.short = icmp ult i32 %tile.m, %previous.m.total %previous.m.tile = select i1 %previous.m.short, i32 %tile.m, i32 %previous.m.total %previous.n.short = icmp ult i32 %tile.n, %in.channels %previous.n.tile = select i1 %previous.n.short, i32 %tile.n, i32 %in.channels
%previous.k.short = icmp ult i32 %tile.k, %previous.r.total %previous.k.tile = select i1 %previous.k.short, i32 %tile.k, i32 %previous.r.total
%previous.m.adjusted = add i32 %previous.m.total, %previous.m.tile %previous.m.numerator = sub i32 %previous.m.adjusted, 1 %previous.m.tiles = udiv i32 %previous.m.numerator, %previous.m.tile %previous.n.adjusted = add i32 %in.channels, %previous.n.tile %previous.n.numerator = sub i32 %previous.n.adjusted, 1 %previous.n.tiles = udiv i32 %previous.n.numerator, %previous.n.tile
%previous.jobs = mul i32 %previous.m.tiles, %previous.n.tiles br label %previous.job.loop previous.job.loop:
%previous.job = phi i32 [ %group, %previous.entry ], [ %previous.job.next, %previous.job.done ] %previous.job.more = icmp ult i32 %previous.job, %previous.jobs br i1 %previous.job.more, label %previous.job.step, label %exit
previous.job.step: %previous.n.index = udiv i32 %previous.job, %previous.m.tiles %previous.m.index = urem i32 %previous.job, %previous.m.tiles %previous.m.base = mul i32 %previous.m.index, %previous.m.tile %previous.n.base = mul i32 %previous.n.index, %previous.n.tile
%previous.m.remaining = sub i32 %previous.m.total, %previous.m.base %previous.m.partial = icmp ult i32 %previous.m.remaining, %previous.m.tile %previous.m.count = select i1 %previous.m.partial, i32 %previous.m.remaining, i32 %previous.m.tile
%previous.n.remaining = sub i32 %in.channels, %previous.n.base %previous.n.partial = icmp ult i32 %previous.n.remaining, %previous.n.tile %previous.n.count = select i1 %previous.n.partial, i32 %previous.n.remaining, i32 %previous.n.tile
%previous.m.lanes.adjusted = add i32 %previous.m.count, RECIPE_REGISTER_M %previous.m.lanes.numerator = sub i32 %previous.m.lanes.adjusted, 1 %previous.m.lanes = udiv i32 %previous.m.lanes.numerator, RECIPE_REGISTER_M %previous.n.lanes.adjusted = add i32 %previous.n.count, RECIPE_REGISTER_N %previous.n.lanes.numerator = sub i32 %previous.n.lanes.adjusted, 1 %previous.n.lanes = udiv i32 %previous.n.lanes.numerator, RECIPE_REGISTER_N
%previous.lanes = mul i32 %previous.m.lanes, %previous.n.lanes %previous.lane.active = icmp ult i32 %lid, %previous.lanes %previous.lane.n.raw = udiv i32 %lid, %previous.m.lanes %previous.lane.m.raw = urem i32 %lid, %previous.m.lanes %previous.lane.n = select i1 %previous.lane.active, i32 %previous.lane.n.raw, i32 0 %previous.lane.m = select i1 %previous.lane.active, i32 %previous.lane.m.raw, i32 0
%previous.output.m.base = mul i32 %previous.lane.m, RECIPE_REGISTER_M %previous.output.n.base = mul i32 %previous.lane.n, RECIPE_REGISTER_N br label %previous.sum.init.loop previous.sum.init.loop:
%previous.sum.init = phi i32 [ 0, %previous.job.step ], [ %previous.sum.init.next, %previous.sum.init.step ] %previous.sum.init.more = icmp ult i32 %previous.sum.init, RECIPE_REGISTER_COUNT br i1 %previous.sum.init.more, label %previous.sum.init.step, label %previous.tile.loop
previous.sum.init.step: %previous.sum.init.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %previous.sum.init store double 0.0, ptr addrspace(5) %previous.sum.init.ptr, align 8 %previous.sum.init.next = add i32 %previous.sum.init, 1 br label %previous.sum.init.loop previous.tile.loop:
%previous.r.base = phi i32 [ 0, %previous.sum.init.loop ], [ %previous.r.next, %previous.tile.done ]
%previous.r.remaining = sub i32 %previous.r.total, %previous.r.base %previous.r.partial = icmp ult i32 %previous.r.remaining, %previous.k.tile %previous.r.count = select i1 %previous.r.partial, i32 %previous.r.remaining, i32 %previous.k.tile
%previous.a.count = mul i32 %previous.m.count, %previous.r.count %previous.b.count = mul i32 %previous.n.count, %previous.r.count %previous.load.count = add i32 %previous.a.count, %previous.b.count br label %previous.load.loop previous.load.loop:
%previous.load = phi i32 [ %lid, %previous.tile.loop ], [ %previous.load.next, %previous.load.store ] %previous.load.more = icmp ult i32 %previous.load, %previous.load.count br i1 %previous.load.more, label %previous.load.classify, label %previous.load.done
previous.load.classify: %previous.load.a = icmp ult i32 %previous.load, %previous.a.count br i1 %previous.load.a, label %previous.load.a.step, label %previous.load.b.step
previous.load.a.step: %previous.a.r = udiv i32 %previous.load, %previous.m.count %previous.a.m = urem i32 %previous.load, %previous.m.count %previous.a.term = add i32 %previous.r.base, %previous.a.r
%previous.a.filter = udiv i32 %previous.a.term, %span %previous.a.kernel = urem i32 %previous.a.term, %span %previous.a.global = add i32 %previous.m.base, %previous.a.m %previous.a.row = udiv i32 %previous.a.global, %in.length %previous.a.position = urem i32 %previous.a.global, %in.length
%previous.a.low = icmp uge i32 %previous.a.position, %previous.a.kernel %previous.a.position.raw = sub i32 %previous.a.position, %previous.a.kernel %previous.a.high = icmp ult i32 %previous.a.position.raw, %out.length %previous.a.valid = and i1 %previous.a.low, %previous.a.high
%previous.a.position.safe = select i1 %previous.a.valid, i32 %previous.a.position.raw, i32 0 %previous.a.row.base = mul i32 %previous.a.row, %out.elements %previous.a.filter.base = mul i32 %previous.a.filter, %out.length
%previous.a.local = add i32 %previous.a.filter.base, %previous.a.position.safe %previous.a.index = add i32 %previous.a.row.base, %previous.a.local %previous.a.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %previous.a.index %previous.a.raw = load double, ptr addrspace(1) %previous.a.ptr, align 8
%previous.a.value = select i1 %previous.a.valid, double %previous.a.raw, double 0.0 %previous.a.tile.row = mul i32 %previous.a.r, %tile.m %previous.a.tile.index = add i32 %previous.a.tile.row, %previous.a.m br label %previous.load.store
previous.load.b.step: %previous.b.local = sub i32 %previous.load, %previous.a.count %previous.b.r = udiv i32 %previous.b.local, %previous.n.count %previous.b.n = urem i32 %previous.b.local, %previous.n.count %previous.b.term = add i32 %previous.r.base, %previous.b.r
%previous.b.filter = udiv i32 %previous.b.term, %span %previous.b.kernel = urem i32 %previous.b.term, %span %previous.b.channel = add i32 %previous.n.base, %previous.b.n %previous.b.filter.base = mul i32 %previous.b.filter, %window
%previous.b.channel.base = mul i32 %previous.b.channel, %span %previous.b.channel.local = add i32 %previous.b.channel.base, %previous.b.kernel %previous.b.index = add i32 %previous.b.filter.base, %previous.b.channel.local
%previous.b.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %previous.b.index %previous.b.value = load double, ptr addrspace(1) %previous.b.ptr, align 8 %previous.b.tile.base = mul i32 %tile.m, %tile.k %previous.b.tile.row = mul i32 %previous.b.r, %tile.n
%previous.b.tile.local = add i32 %previous.b.tile.row, %previous.b.n %previous.b.tile.index = add i32 %previous.b.tile.base, %previous.b.tile.local br label %previous.load.store
previous.load.store: %previous.load.value = phi double [ %previous.a.value, %previous.load.a.step ], [ %previous.b.value, %previous.load.b.step ] %previous.load.index = phi i32 [ %previous.a.tile.index, %previous.load.a.step ], [ %previous.b.tile.index, %previous.load.b.step ]
%previous.load.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %previous.load.index store double %previous.load.value, ptr addrspace(3) %previous.load.ptr, align 8
%previous.load.next = add i32 %previous.load, %block br label %previous.load.loop previous.load.done: call void @recipe.local.barrier()
call void @contraction_accumulate(ptr addrspace(5) %sums, ptr addrspace(5) null, i1 false, i1 %previous.lane.active, i32 %previous.output.m.base, i32 %previous.output.n.base, i32 %previous.m.count, i32 %previous.n.count, i32 %previous.r.count) call void @recipe.local.barrier()
%previous.r.next = add i32 %previous.r.base, %previous.r.count %previous.r.more = icmp ult i32 %previous.r.next, %previous.r.total br i1 %previous.r.more, label %previous.tile.done, label %previous.store.loop previous.tile.done: br label %previous.tile.loop previous.store.loop:
%previous.store.register = phi i32 [ 0, %previous.load.done ], [ %previous.store.register.next, %previous.store.next ] %previous.store.more = icmp ult i32 %previous.store.register, RECIPE_REGISTER_COUNT br i1 %previous.store.more, label %previous.store.test, label %previous.job.done
previous.store.test: %previous.store.register.m = urem i32 %previous.store.register, RECIPE_REGISTER_M %previous.store.register.n = udiv i32 %previous.store.register, RECIPE_REGISTER_M %previous.store.output.m.raw = add i32 %previous.output.m.base, %previous.store.register.m %previous.store.output.n.raw = add i32 %previous.output.n.base, %previous.store.register.n
%previous.store.output.m.valid = icmp ult i32 %previous.store.output.m.raw, %previous.m.count %previous.store.output.n.valid = icmp ult i32 %previous.store.output.n.raw, %previous.n.count %previous.store.output.valid = and i1 %previous.store.output.m.valid, %previous.store.output.n.valid %previous.store.active = and i1 %previous.lane.active, %previous.store.output.valid br i1 %previous.store.active, label %previous.store, label %previous.store.next
previous.store: %previous.store.m.global = add i32 %previous.m.base, %previous.store.output.m.raw %previous.store.channel = add i32 %previous.n.base, %previous.store.output.n.raw %previous.store.row = udiv i32 %previous.store.m.global, %in.length %previous.store.position = urem i32 %previous.store.m.global, %in.length
%previous.store.row.base = mul i32 %previous.store.row, %in.elements %previous.store.channel.base = mul i32 %previous.store.channel, %in.length %previous.store.local = add i32 %previous.store.channel.base, %previous.store.position %previous.store.index = add i32 %previous.store.row.base, %previous.store.local %previous.store.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %previous.store.index
%previous.store.old = load double, ptr addrspace(1) %previous.store.ptr, align 8 %previous.store.sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %previous.store.register %previous.store.sum = load double, ptr addrspace(5) %previous.store.sum.ptr, align 8 %previous.store.value = call double @recipe.add(double %previous.store.old, double %previous.store.sum) store double %previous.store.value, ptr addrspace(1) %previous.store.ptr, align 8 br label %previous.store.next
previous.store.next: %previous.store.register.next = add i32 %previous.store.register, 1 br label %previous.store.loop previous.job.done: %previous.job.next = add i32 %previous.job, %groups br label %previous.job.loop exit: ret void }
define internal void @scan_reverse_body( ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output,
ptr addrspace(1) %context, ptr addrspace(1) %delta, ptr addrspace(1) %previous,
ptr addrspace(1) %gradient, i1 %write.input, i32 %rows, i32 %in.channels,
i32 %length, i32 %out.channels, i32 %gates, i32 %parameters, i32 %offset, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry:
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
i32 %projection.gradient.offset, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) call void @llvm.amdgcn.s.barrier() %projection.next = add i32 %projection.gate, 1 br label %projection.loop
invalid: call void @llvm.trap() br label %exit exit: ret void } attributes #0 = { nounwind "amdgpu-flat-work-group-size"="1,1024" } attributes #1 = { alwaysinline nounwind }
