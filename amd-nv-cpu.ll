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
define internal double @contraction_delta(ptr addrspace(1) %delta, ptr addrspace(1) %output, i32 %index, i1 %relu) #1 {
entry:
%delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %index
%delta.value = load double, ptr addrspace(1) %delta.ptr, align 8
br i1 %relu, label %activation, label %done
activation:
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %index
%output.value = load double, ptr addrspace(1) %output.ptr, align 8
%positive = call i1 @recipe.ogt(double %output.value, double 0.0)
%activated = select i1 %positive, double %delta.value, double 0.0
br label %done
done:
%value = phi double [ %delta.value, %entry ], [ %activated, %activation ]
ret double %value
}
define internal void @reduce_rows(ptr addrspace(1) %source, ptr addrspace(1) %target, i32 %rows, i32 %columns, i32 %source.offset, i32 %target.offset, i32 %threads) #1 {
entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x()
br label %parameter.loop
parameter.loop:
%parameter = phi i32 [ %tid, %entry ], [ %parameter.next, %store ]
%parameter.more = icmp ult i32 %parameter, %columns
br i1 %parameter.more, label %seed.load, label %exit
seed.load:
%target.index = add i32 %target.offset, %parameter
%target.ptr = getelementptr inbounds double, ptr addrspace(1) %target, i32 %target.index
%source.first.index = add i32 %source.offset, %parameter
%source.first.ptr = getelementptr inbounds double, ptr addrspace(1) %source, i32 %source.first.index
%source.first = load double, ptr addrspace(1) %source.first.ptr, align 8
br label %row.loop
row.loop:
%row = phi i32 [ 1, %seed.load ], [ %row.next, %row.step ]
%sum = phi double [ %source.first, %seed.load ], [ %sum.next, %row.step ]
%row.more = icmp ult i32 %row, %rows
br i1 %row.more, label %row.step, label %store
row.step:
%row.base = mul i32 %row, %columns
%source.local = add i32 %row.base, %parameter
%source.index = add i32 %source.offset, %source.local
%source.ptr = getelementptr inbounds double, ptr addrspace(1) %source, i32 %source.index
%source.value = load double, ptr addrspace(1) %source.ptr, align 8
%sum.next = call double @recipe.add(double %sum, double %source.value)
%row.next = add i32 %row, 1
br label %row.loop
store:
store double %sum, ptr addrspace(1) %target.ptr, align 8
%parameter.next = add i32 %parameter, %threads
br label %parameter.loop
exit:
ret void
}
define internal i32 @contraction_b_index(i32 %k, i32 %n, i32 %tile.n, i32 %tile.b.k) #1 {
entry:
%vector.row = mul i32 %k, %tile.n
%vector = add i32 %vector.row, %n
%matrix.row = mul i32 %n, %tile.b.k
%matrix = add i32 %matrix.row, %k
ret i32 RECIPE_CONTRACTION_B_INDEX
}
define internal void @contraction_zero_edges(i32 %m.count, i32 %n.count, i32 %k.count, i32 %lid, i32 %block, i32 %tile.m, i32 %tile.n, i32 %tile.k) #1 {
entry:
%tile.b.k = add i32 %tile.k, RECIPE_WMMA_B_PADDING
%a.missing = sub i32 %tile.m, %m.count
%b.missing = sub i32 %tile.n, %n.count
%k.adjusted = add i32 %k.count, RECIPE_WMMA_K
%k.numerator = sub i32 %k.adjusted, 1
%k.fragments = udiv i32 %k.numerator, RECIPE_WMMA_K
%k.padded = mul i32 %k.fragments, RECIPE_WMMA_K
%k.missing = sub i32 %k.padded, %k.count
%a.count = mul i32 %a.missing, %k.count
%b.count = mul i32 %b.missing, %k.count
%k.width = add i32 %tile.m, %tile.n
%k.count.zeros = mul i32 %k.missing, %k.width
%edge.count = add i32 %a.count, %b.count
%count = add i32 %edge.count, %k.count.zeros
br label %loop
loop:
%p = phi i32 [ %lid, %entry ], [ %next, %store ]
%more = icmp ult i32 %p, %count
br i1 %more, label %classify, label %exit
classify:
%a = icmp ult i32 %p, %a.count
br i1 %a, label %a.step, label %b.test
a.step:
%a.k = udiv i32 %p, %a.missing
%a.local = urem i32 %p, %a.missing
%a.m = add i32 %m.count, %a.local
%a.row = mul i32 %a.k, %tile.m
%a.index = add i32 %a.row, %a.m
br label %store
b.test:
%b = icmp ult i32 %p, %edge.count
br i1 %b, label %b.step, label %k.step
b.step:
%b.p = sub i32 %p, %a.count
%b.k = udiv i32 %b.p, %b.missing
%b.local = urem i32 %b.p, %b.missing
%b.n = add i32 %n.count, %b.local
%b.base = mul i32 %tile.m, %tile.k
%b.local.index = call i32 @contraction_b_index(i32 %b.k, i32 %b.n, i32 %tile.n, i32 %tile.b.k)
%b.index = add i32 %b.base, %b.local.index
br label %store
k.step:
%k.p = sub i32 %p, %edge.count
%k.row = udiv i32 %k.p, %k.width
%k.local = urem i32 %k.p, %k.width
%k = add i32 %k.count, %k.row
%k.b = icmp uge i32 %k.local, %tile.m
br i1 %k.b, label %k.b.step, label %k.a.step
k.a.step:
%k.a.row = mul i32 %k, %tile.m
%k.a.index = add i32 %k.a.row, %k.local
br label %store
k.b.step:
%k.b.local = sub i32 %k.local, %tile.m
%k.b.base = mul i32 %tile.m, %tile.k
%k.b.local.index = call i32 @contraction_b_index(i32 %k, i32 %k.b.local, i32 %tile.n, i32 %tile.b.k)
%k.b.index = add i32 %k.b.base, %k.b.local.index
br label %store
store:
%index = phi i32 [ %a.index, %a.step ], [ %b.index, %b.step ], [ %k.a.index, %k.a.step ], [ %k.b.index, %k.b.step ]
%ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %index
store double 0.0, ptr addrspace(3) %ptr, align 8
%next = add i32 %p, %block
br label %loop
exit:
ret void
}
define internal <RECIPE_REGISTER_M x double> @contraction_a_fragment(i32 %k, i32 %output.m.base, i32 %tile.m) #1 {
entry:
%row = mul i32 %k, %tile.m
%index = add i32 %row, %output.m.base
%ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %index
%fragment = load <RECIPE_REGISTER_M x double>, ptr addrspace(3) %ptr, align 8
ret <RECIPE_REGISTER_M x double> %fragment
}
define internal <RECIPE_REGISTER_N x double> @contraction_b_fragment(i32 %k, i32 %output.n.base, i32 %tile.m, i32 %tile.n, i32 %tile.k) #1 {
entry:
%tile.b.k = add i32 %tile.k, RECIPE_WMMA_B_PADDING
%base = mul i32 %tile.m, %tile.k
%local = call i32 @contraction_b_index(i32 %k, i32 %output.n.base, i32 %tile.n, i32 %tile.b.k)
%index = add i32 %base, %local
%ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %index
%fragment = load <RECIPE_REGISTER_N x double>, ptr addrspace(3) %ptr, align 8
ret <RECIPE_REGISTER_N x double> %fragment
}
define internal void @contraction_stage_a_fragment(<RECIPE_WMMA_K x double> %fragment, i32 %k, i32 %m, i32 %tile.m) #1 {
entry:
br label %loop
loop:
%element = phi i32 [ 0, %entry ], [ %next, %step ]
%more = icmp ult i32 %element, RECIPE_WMMA_K
br i1 %more, label %step, label %exit
step:
%local.k = add i32 %k, %element
%row = mul i32 %local.k, %tile.m
%index = add i32 %row, %m
%value = extractelement <RECIPE_WMMA_K x double> %fragment, i32 %element
%target = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %index
store double %value, ptr addrspace(3) %target, align 8
%next = add i32 %element, 1
br label %loop
exit:
ret void
}
define internal void @contraction_stage_b_fragment(<RECIPE_WMMA_K x double> %fragment, i32 %k, i32 %n, i32 %tile.m, i32 %tile.n, i32 %tile.k) #1 {
entry:
%base = mul i32 %tile.m, %tile.k
%tile.b.k = add i32 %tile.k, RECIPE_WMMA_B_PADDING
br label %loop
loop:
%element = phi i32 [ 0, %entry ], [ %next, %step ]
%more = icmp ult i32 %element, RECIPE_WMMA_K
br i1 %more, label %step, label %exit
step:
%local.n = add i32 %n, %element
%local = call i32 @contraction_b_index(i32 %k, i32 %local.n, i32 %tile.n, i32 %tile.b.k)
%index = add i32 %base, %local
%value = extractelement <RECIPE_WMMA_K x double> %fragment, i32 %element
%target = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %index
store double %value, ptr addrspace(3) %target, align 8
%next = add i32 %element, 1
br label %loop
exit:
ret void
}
define internal void @contraction_stage_delta_b_fragment(<RECIPE_WMMA_K x double> %delta, <RECIPE_WMMA_K x double> %output, i1 %relu, i32 %k, i32 %n, i32 %tile.m, i32 %tile.n, i32 %tile.k) #1 {
entry:
%base = mul i32 %tile.m, %tile.k
%tile.b.k = add i32 %tile.k, RECIPE_WMMA_B_PADDING
br label %loop
loop:
%element = phi i32 [ 0, %entry ], [ %next, %step ]
%more = icmp ult i32 %element, RECIPE_WMMA_K
br i1 %more, label %step, label %exit
step:
%delta.value = extractelement <RECIPE_WMMA_K x double> %delta, i32 %element
%output.value = extractelement <RECIPE_WMMA_K x double> %output, i32 %element
%positive = call i1 @recipe.ogt(double %output.value, double 0.0)
%active = select i1 %positive, double %delta.value, double 0.0
%value = select i1 %relu, double %active, double %delta.value
%local.n = add i32 %n, %element
%local = call i32 @contraction_b_index(i32 %k, i32 %local.n, i32 %tile.n, i32 %tile.b.k)
%index = add i32 %base, %local
%target = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %index
store double %value, ptr addrspace(3) %target, align 8
%next = add i32 %element, 1
br label %loop
exit:
ret void
}
define internal void @contraction_stage_delta_a_fragment(<RECIPE_WMMA_K x double> %delta, <RECIPE_WMMA_K x double> %output, i1 %relu, i32 %k, i32 %m, i32 %tile.m) #1 {
entry:
br label %loop
loop:
%element = phi i32 [ 0, %entry ], [ %next, %step ]
%more = icmp ult i32 %element, RECIPE_WMMA_K
br i1 %more, label %step, label %exit
step:
%delta.value = extractelement <RECIPE_WMMA_K x double> %delta, i32 %element
%output.value = extractelement <RECIPE_WMMA_K x double> %output, i32 %element
%positive = call i1 @recipe.ogt(double %output.value, double 0.0)
%active = select i1 %positive, double %delta.value, double 0.0
%value = select i1 %relu, double %active, double %delta.value
%local.k = add i32 %k, %element
%row = mul i32 %local.k, %tile.m
%index = add i32 %row, %m
%target = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %index
store double %value, ptr addrspace(3) %target, align 8
%next = add i32 %element, 1
br label %loop
exit:
ret void
}
define internal i32 @contraction_output_lanes_vector(i32 %m.lanes, i32 %n.lanes, i32 %block) #1 {
entry:
%lanes = mul i32 %m.lanes, %n.lanes
ret i32 %lanes
}
define internal i32 @contraction_output_m_vector(i32 %lid, i32 %register, i32 %m.lanes) #1 {
entry:
%lane = urem i32 %lid, %m.lanes
%base = mul i32 %lane, RECIPE_REGISTER_M
%local = urem i32 %register, RECIPE_REGISTER_M
%m = add i32 %base, %local
ret i32 %m
}
define internal i32 @contraction_output_n_vector(i32 %lid, i32 %register, i32 %m.lanes) #1 {
entry:
%lane = udiv i32 %lid, %m.lanes
%base = mul i32 %lane, RECIPE_REGISTER_N
%local = udiv i32 %register, RECIPE_REGISTER_M
%n = add i32 %base, %local
ret i32 %n
}
define internal i1 @contraction_output_register_valid_vector(i32 %register) #1 {
entry:
ret i1 true
}
define internal i1 @contraction_bias_enable_vector(i1 %has.bias, i32 %m.base, i32 %output.m.base, i32 %lid) #1 {
entry:
%local = add i32 %m.base, %output.m.base
%first = icmp eq i32 %local, 0
%enabled = and i1 %has.bias, %first
ret i1 %enabled
}
define internal i32 @contraction_output_lanes_wmma(i32 %m.lanes, i32 %n.lanes, i32 %block) #1 {
entry:
ret i32 %block
}
define internal i32 @contraction_output_m_wmma(i32 %lid, i32 %register, i32 %m.lanes) #1 {
entry:
%wave.lane = urem i32 %lid, 32
%half = udiv i32 %wave.lane, 16
%tile = udiv i32 %register, 8
%fragment = urem i32 %register, 8
%tile.base = mul i32 %tile, 16
%row = mul i32 %fragment, 2
%local = add i32 %tile.base, %row
%m = add i32 %local, %half
ret i32 %m
}
define internal i32 @contraction_output_n_wmma(i32 %lid, i32 %register, i32 %m.lanes) #1 {
entry:
%wave = udiv i32 %lid, 32
%wave.base = mul i32 %wave, 16
%lane = urem i32 %lid, 16
%n = add i32 %wave.base, %lane
ret i32 %n
}
define internal i1 @contraction_output_register_valid_wmma(i32 %register) #1 {
entry:
ret i1 true
}
define internal i1 @contraction_bias_enable_wmma(i1 %has.bias, i32 %m.base, i32 %output.m.base, i32 %lid) #1 {
entry:
%first.tile = icmp eq i32 %m.base, 0
%wave.lane = urem i32 %lid, 32
%first.half = icmp ult i32 %wave.lane, 16
%first = and i1 %first.tile, %first.half
%enabled = and i1 %has.bias, %first
ret i1 %enabled
}
define internal void @contraction_accumulate_wmma(ptr addrspace(5) %sums, ptr addrspace(5) %biases, i1 %bias.enable, i1 %lane.active, i32 %output.m.base, i32 %output.n.base, i32 %m.count, i32 %n.count, i32 %k.offset, i32 %k.stride, i32 %k.count, i32 %tile.m, i32 %tile.n, i32 %tile.k) #1 {
entry:
%wide = alloca [RECIPE_REGISTER_COUNT x float], align 4, addrspace(5)
%tile.b.k = add i32 %tile.k, RECIPE_WMMA_B_PADDING
%lid = call i32 @recipe.local.id.x()
%wave = udiv i32 %lid, 32
%wave.base = mul i32 %wave, 16
%matrix.lane = urem i32 %lid, 16
%n = add i32 %wave.base, %matrix.lane
%m.tiles = udiv i32 %tile.m, 16
%b.base = mul i32 %tile.m, %tile.k
br label %wide.init.loop
wide.init.loop:
%wide.init.tile = phi i32 [ 0, %entry ], [ %wide.init.next, %wide.init.step ]
%wide.init.more = icmp ult i32 %wide.init.tile, %m.tiles
br i1 %wide.init.more, label %wide.init.step, label %wide.init.done
wide.init.step:
%wide.init.base = mul i32 %wide.init.tile, 8
%wide.init.source = getelementptr [RECIPE_REGISTER_COUNT x half], ptr addrspace(5) %sums, i32 0, i32 %wide.init.base
%wide.init.packed = load <8 x half>, ptr addrspace(5) %wide.init.source, align 2
%wide.init.value = fpext <8 x half> %wide.init.packed to <8 x float>
%wide.init.target = getelementptr [RECIPE_REGISTER_COUNT x float], ptr addrspace(5) %wide, i32 0, i32 %wide.init.base
store <8 x float> %wide.init.value, ptr addrspace(5) %wide.init.target, align 4
%wide.init.next = add i32 %wide.init.tile, 1
br label %wide.init.loop
wide.init.done:
br label %k.loop
k.loop:
%k.base = phi i32 [ 0, %wide.init.done ], [ %k.next, %m.done ]
%k.more = icmp ult i32 %k.base, %k.count
br i1 %k.more, label %b.load, label %exit
b.load:
%b.vector.local = call i32 @contraction_b_index(i32 %k.base, i32 %n, i32 %tile.n, i32 %tile.b.k)
%b.vector.index = add i32 %b.base, %b.vector.local
%b.vector.ptr = getelementptr [0 x half], ptr addrspace(3) @contraction_tile, i32 0, i32 %b.vector.index
%b.fragment.ready = load <16 x half>, ptr addrspace(3) %b.vector.ptr, align 2
br i1 %bias.enable, label %bias.entry, label %m.entry
bias.entry:
%bias.ptr = getelementptr [RECIPE_REGISTER_N x half], ptr addrspace(5) %biases, i32 0, i32 0
%bias.initial = load half, ptr addrspace(5) %bias.ptr, align 2
br label %bias.loop
bias.loop:
%bias.element = phi i32 [ 0, %bias.entry ], [ %bias.element.next, %bias.step ]
%bias.sum = phi half [ %bias.initial, %bias.entry ], [ %bias.sum.next, %bias.step ]
%bias.more = icmp ult i32 %bias.element, 16
br i1 %bias.more, label %bias.step, label %bias.done
bias.step:
%bias.value = extractelement <16 x half> %b.fragment.ready, i32 %bias.element
%bias.sum.next = call half @recipe.add(half %bias.sum, half %bias.value)
%bias.element.next = add i32 %bias.element, 1
br label %bias.loop
bias.done:
store half %bias.sum, ptr addrspace(5) %bias.ptr, align 2
br label %m.entry
m.entry:
br label %m.loop
m.loop:
%m.tile = phi i32 [ 0, %m.entry ], [ %m.next, %m.step ]
%m.more = icmp ult i32 %m.tile, %m.tiles
br i1 %m.more, label %m.step, label %m.done
m.step:
%a.k = add i32 %k.base, %matrix.lane
%a.row = mul i32 %a.k, %tile.m
%a.local = mul i32 %m.tile, 16
%a.index = add i32 %a.row, %a.local
%a.ptr = getelementptr [0 x half], ptr addrspace(3) @contraction_tile, i32 0, i32 %a.index
%a.fragment = load <16 x half>, ptr addrspace(3) %a.ptr, align 2
%sum.base = mul i32 %m.tile, 8
%sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x float], ptr addrspace(5) %wide, i32 0, i32 %sum.base
%sum = load <8 x float>, ptr addrspace(5) %sum.ptr, align 4
%next = call <8 x float> @llvm.amdgcn.wmma.f32.16x16x16.f16(<16 x half> %a.fragment, <16 x half> %b.fragment.ready, <8 x float> %sum)
store <8 x float> %next, ptr addrspace(5) %sum.ptr, align 4
%m.next = add i32 %m.tile, 1
br label %m.loop
m.done:
%k.next = add i32 %k.base, 16
br label %k.loop
exit:
br label %wide.pack.loop
wide.pack.loop:
%wide.pack.tile = phi i32 [ 0, %exit ], [ %wide.pack.next, %wide.pack.step ]
%wide.pack.more = icmp ult i32 %wide.pack.tile, %m.tiles
br i1 %wide.pack.more, label %wide.pack.step, label %wide.pack.done
wide.pack.step:
%wide.pack.base = mul i32 %wide.pack.tile, 8
%wide.pack.source = getelementptr [RECIPE_REGISTER_COUNT x float], ptr addrspace(5) %wide, i32 0, i32 %wide.pack.base
%wide.pack.value = load <8 x float>, ptr addrspace(5) %wide.pack.source, align 4
%wide.pack.value.narrow = fptrunc <8 x float> %wide.pack.value to <8 x half>
%wide.pack.target = getelementptr [RECIPE_REGISTER_COUNT x half], ptr addrspace(5) %sums, i32 0, i32 %wide.pack.base
store <8 x half> %wide.pack.value.narrow, ptr addrspace(5) %wide.pack.target, align 2
%wide.pack.next = add i32 %wide.pack.tile, 1
br label %wide.pack.loop
wide.pack.done:
ret void
}
define internal <4 x i32> @contraction_pack_i8(<16 x i8> %values) #1 {
entry:
%packed = bitcast <16 x i8> %values to <4 x i32>
ret <4 x i32> %packed
}
define internal <2 x i32> @contraction_pack_i4(<16 x i8> %values) #1 {
entry:
br label %loop
loop:
%p = phi i32 [ 0, %entry ], [ %next, %step ]
%packed = phi <8 x i8> [ zeroinitializer, %entry ], [ %packed.next, %step ]
%more = icmp ult i32 %p, 8
br i1 %more, label %step, label %done
step:
%low.index = mul i32 %p, 2
%high.index = add i32 %low.index, 1
%low.raw = extractelement <16 x i8> %values, i32 %low.index
%high.raw = extractelement <16 x i8> %values, i32 %high.index
%low = and i8 %low.raw, 15
%high.masked = and i8 %high.raw, 15
%high = shl i8 %high.masked, 4
%value = or i8 %low, %high
%packed.next = insertelement <8 x i8> %packed, i8 %value, i32 %p
%next = add i32 %p, 1
br label %loop
done:
%result = bitcast <8 x i8> %packed to <2 x i32>
ret <2 x i32> %result
}
define internal void @contraction_accumulate_wmma_integer(ptr addrspace(5) %sums, ptr addrspace(5) %biases, i1 %bias.enable, i1 %lane.active, i32 %output.m.base, i32 %output.n.base, i32 %m.count, i32 %n.count, i32 %k.offset, i32 %k.stride, i32 %k.count, i32 %tile.m, i32 %tile.n, i32 %tile.k) #1 {
entry:
%wide = alloca [RECIPE_REGISTER_COUNT x i32], align 4, addrspace(5)
%tile.b.k = add i32 %tile.k, RECIPE_WMMA_B_PADDING
%lid = call i32 @recipe.local.id.x()
%wave = udiv i32 %lid, 32
%wave.base = mul i32 %wave, 16
%matrix.lane = urem i32 %lid, 16
%n = add i32 %wave.base, %matrix.lane
%m.tiles = udiv i32 %tile.m, 16
%b.base = mul i32 %tile.m, %tile.k
br label %wide.init.loop
wide.init.loop:
%wide.init.tile = phi i32 [ 0, %entry ], [ %wide.init.next, %wide.init.step ]
%wide.init.more = icmp ult i32 %wide.init.tile, %m.tiles
br i1 %wide.init.more, label %wide.init.step, label %wide.init.done
wide.init.step:
%wide.init.base = mul i32 %wide.init.tile, 8
%wide.init.source = getelementptr [RECIPE_REGISTER_COUNT x i8], ptr addrspace(5) %sums, i32 0, i32 %wide.init.base
%wide.init.loaded = load <8 x i8>, ptr addrspace(5) %wide.init.source, align 1
%wide.init.unsigned = zext <8 x i8> %wide.init.loaded to <8 x i32>
%wide.init.masked = and <8 x i32> %wide.init.unsigned, <i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK>
%wide.init.shifted = shl <8 x i32> %wide.init.masked, <i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT>
%wide.init.value = ashr <8 x i32> %wide.init.shifted, <i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT, i32 RECIPE_WMMA_INTEGER_SHIFT>
%wide.init.target = getelementptr [RECIPE_REGISTER_COUNT x i32], ptr addrspace(5) %wide, i32 0, i32 %wide.init.base
store <8 x i32> %wide.init.value, ptr addrspace(5) %wide.init.target, align 4
%wide.init.next = add i32 %wide.init.tile, 1
br label %wide.init.loop
wide.init.done:
br label %k.loop
k.loop:
%k.base = phi i32 [ 0, %wide.init.done ], [ %k.next, %m.done ]
%k.more = icmp ult i32 %k.base, %k.count
br i1 %k.more, label %b.load, label %exit
b.load:
%b.vector.local = call i32 @contraction_b_index(i32 %k.base, i32 %n, i32 %tile.n, i32 %tile.b.k)
%b.vector.index = add i32 %b.base, %b.vector.local
%b.vector.ptr = getelementptr [0 x i8], ptr addrspace(3) @contraction_tile, i32 0, i32 %b.vector.index
%b.fragment.ready = load <16 x i8>, ptr addrspace(3) %b.vector.ptr, align 1
%b.packed = call RECIPE_WMMA_INTEGER_VECTOR @RECIPE_WMMA_INTEGER_PACK(<16 x i8> %b.fragment.ready)
br i1 %bias.enable, label %bias.entry, label %m.entry
bias.entry:
%bias.ptr = getelementptr [RECIPE_REGISTER_N x i8], ptr addrspace(5) %biases, i32 0, i32 0
%bias.initial = load i8, ptr addrspace(5) %bias.ptr, align 1
br label %bias.loop
bias.loop:
%bias.element = phi i32 [ 0, %bias.entry ], [ %bias.element.next, %bias.step ]
%bias.sum = phi i8 [ %bias.initial, %bias.entry ], [ %bias.sum.next, %bias.step ]
%bias.more = icmp ult i32 %bias.element, 16
br i1 %bias.more, label %bias.step, label %bias.done
bias.step:
%bias.value = extractelement <16 x i8> %b.fragment.ready, i32 %bias.element
%bias.sum.next = call i8 @recipe.add(i8 %bias.sum, i8 %bias.value)
%bias.element.next = add i32 %bias.element, 1
br label %bias.loop
bias.done:
store i8 %bias.sum, ptr addrspace(5) %bias.ptr, align 1
br label %m.entry
m.entry:
br label %m.loop
m.loop:
%m.tile = phi i32 [ 0, %m.entry ], [ %m.next, %m.step ]
%m.more = icmp ult i32 %m.tile, %m.tiles
br i1 %m.more, label %m.step, label %m.done
m.step:
%a.k = add i32 %k.base, %matrix.lane
%a.row = mul i32 %a.k, %tile.m
%a.local = mul i32 %m.tile, 16
%a.index = add i32 %a.row, %a.local
%a.ptr = getelementptr [0 x i8], ptr addrspace(3) @contraction_tile, i32 0, i32 %a.index
%a.fragment = load <16 x i8>, ptr addrspace(3) %a.ptr, align 1
%a.packed = call RECIPE_WMMA_INTEGER_VECTOR @RECIPE_WMMA_INTEGER_PACK(<16 x i8> %a.fragment)
%sum.base = mul i32 %m.tile, 8
%sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x i32], ptr addrspace(5) %wide, i32 0, i32 %sum.base
%sum = load <8 x i32>, ptr addrspace(5) %sum.ptr, align 4
%next = call <8 x i32> @RECIPE_WMMA_INTEGER_INTRINSIC(i1 true, RECIPE_WMMA_INTEGER_VECTOR %a.packed, i1 true, RECIPE_WMMA_INTEGER_VECTOR %b.packed, <8 x i32> %sum, i1 false)
store <8 x i32> %next, ptr addrspace(5) %sum.ptr, align 4
%m.next = add i32 %m.tile, 1
br label %m.loop
m.done:
%k.next = add i32 %k.base, 16
br label %k.loop
exit:
br label %wide.pack.loop
wide.pack.loop:
%wide.pack.tile = phi i32 [ 0, %exit ], [ %wide.pack.next, %wide.pack.step ]
%wide.pack.more = icmp ult i32 %wide.pack.tile, %m.tiles
br i1 %wide.pack.more, label %wide.pack.step, label %wide.pack.done
wide.pack.step:
%wide.pack.base = mul i32 %wide.pack.tile, 8
%wide.pack.source = getelementptr [RECIPE_REGISTER_COUNT x i32], ptr addrspace(5) %wide, i32 0, i32 %wide.pack.base
%wide.pack.value = load <8 x i32>, ptr addrspace(5) %wide.pack.source, align 4
%wide.pack.below = icmp slt <8 x i32> %wide.pack.value, <i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN>
%wide.pack.lower = select <8 x i1> %wide.pack.below, <8 x i32> <i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN, i32 RECIPE_WMMA_INTEGER_MIN>, <8 x i32> %wide.pack.value
%wide.pack.above = icmp sgt <8 x i32> %wide.pack.lower, <i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX>
%wide.pack.clamped = select <8 x i1> %wide.pack.above, <8 x i32> <i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX, i32 RECIPE_WMMA_INTEGER_MAX>, <8 x i32> %wide.pack.lower
%wide.pack.masked = and <8 x i32> %wide.pack.clamped, <i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK, i32 RECIPE_WMMA_INTEGER_MASK>
%wide.pack.encoded = trunc <8 x i32> %wide.pack.masked to <8 x i8>
%wide.pack.target = getelementptr [RECIPE_REGISTER_COUNT x i8], ptr addrspace(5) %sums, i32 0, i32 %wide.pack.base
store <8 x i8> %wide.pack.encoded, ptr addrspace(5) %wide.pack.target, align 1
%wide.pack.next = add i32 %wide.pack.tile, 1
br label %wide.pack.loop
wide.pack.done:
ret void
}
define internal void @contraction_accumulate_vector(ptr addrspace(5) %sums, ptr addrspace(5) %biases, i1 %bias.enable, i1 %lane.active, i32 %output.m.base, i32 %output.n.base, i32 %m.count, i32 %n.count, i32 %k.offset, i32 %k.stride, i32 %k.count, i32 %tile.m, i32 %tile.n, i32 %tile.k) #1 {
entry:
%output.m.first = icmp ult i32 %output.m.base, %m.count
%k.initial.more = icmp ult i32 %k.offset, %k.count
br i1 %k.initial.more, label %k.initial, label %exit
k.initial:
%a.initial = call <RECIPE_REGISTER_M x double> @contraction_a_fragment(i32 %k.offset, i32 %output.m.base, i32 %tile.m)
%b.initial = call <RECIPE_REGISTER_N x double> @contraction_b_fragment(i32 %k.offset, i32 %output.n.base, i32 %tile.m, i32 %tile.n, i32 %tile.k)
br label %k.loop
k.loop:
%k = phi i32 [ %k.offset, %k.initial ], [ %k.next, %register.done ]
%a.fragment = phi <RECIPE_REGISTER_M x double> [ %a.initial, %k.initial ], [ %a.next, %register.done ]
%b.fragment = phi <RECIPE_REGISTER_N x double> [ %b.initial, %k.initial ], [ %b.next, %register.done ]
%k.next = add i32 %k, %k.stride
%k.more = icmp ult i32 %k.next, %k.count
%k.prefetch = select i1 %k.more, i32 %k.next, i32 %k
%a.next = call <RECIPE_REGISTER_M x double> @contraction_a_fragment(i32 %k.prefetch, i32 %output.m.base, i32 %tile.m)
%b.next = call <RECIPE_REGISTER_N x double> @contraction_b_fragment(i32 %k.prefetch, i32 %output.n.base, i32 %tile.m, i32 %tile.n, i32 %tile.k)
br label %register.loop
register.loop:
%register.n = phi i32 [ 0, %k.loop ], [ %register.n.next, %register.next ]
%register.more = icmp ult i32 %register.n, RECIPE_REGISTER_N
br i1 %register.more, label %register.step, label %register.done
register.step:
%output.n.raw = add i32 %output.n.base, %register.n
%output.n.valid = icmp ult i32 %output.n.raw, %n.count
%output.n.active = and i1 %lane.active, %output.n.valid
%b = extractelement <RECIPE_REGISTER_N x double> %b.fragment, i32 %register.n
%b.seed = insertelement <RECIPE_REGISTER_M x double> poison, double %b, i32 0
%b.vector = shufflevector <RECIPE_REGISTER_M x double> %b.seed, <RECIPE_REGISTER_M x double> poison, <RECIPE_REGISTER_M x i32> zeroinitializer
%register.base = mul i32 %register.n, RECIPE_REGISTER_M
%sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %register.base
%sum = load <RECIPE_REGISTER_M x double>, ptr addrspace(5) %sum.ptr, align 8
%candidate = call <RECIPE_REGISTER_M x double> @recipe.madd.vector(<RECIPE_REGISTER_M x double> %sum, <RECIPE_REGISTER_M x double> %a.fragment, <RECIPE_REGISTER_M x double> %b.vector)
store <RECIPE_REGISTER_M x double> %candidate, ptr addrspace(5) %sum.ptr, align 8
%bias.output = and i1 %output.n.active, %output.m.first
%bias.active = and i1 %bias.enable, %bias.output
br i1 %bias.active, label %bias.step, label %register.next
bias.step:
%bias.ptr = getelementptr [RECIPE_REGISTER_N x double], ptr addrspace(5) %biases, i32 0, i32 %register.n
%bias = load double, ptr addrspace(5) %bias.ptr, align 8
%bias.next = call double @recipe.add(double %bias, double %b)
store double %bias.next, ptr addrspace(5) %bias.ptr, align 8
br label %register.next
register.next:
%register.n.next = add i32 %register.n, 1
br label %register.loop
register.done:
br i1 %k.more, label %k.loop, label %exit
exit:
ret void
}
define internal void @contraction_reduce(ptr addrspace(5) %sums, ptr addrspace(5) %biases, i1 %lane.active, i32 %lid, i32 %output.lanes, i32 %split, i32 %splits, i32 %block) #1 {
entry:
%bias.base = mul i32 %block, RECIPE_REGISTER_COUNT
br label %write.loop
write.loop:
%write.register = phi i32 [ 0, %entry ], [ %write.next, %write.step ]
%write.more = icmp ult i32 %write.register, RECIPE_REGISTER_COUNT
br i1 %write.more, label %write.step, label %bias.write.loop
write.step:
%write.lane.base = mul i32 %lid, RECIPE_REGISTER_COUNT
%write.index = add i32 %write.lane.base, %write.register
%write.source = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %write.register
%write.value = load double, ptr addrspace(5) %write.source, align 8
%write.target = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %write.index
store double %write.value, ptr addrspace(3) %write.target, align 8
%write.next = add i32 %write.register, 1
br label %write.loop
bias.write.loop:
%bias.write.register = phi i32 [ 0, %write.loop ], [ %bias.write.next, %bias.write.step ]
%bias.write.more = icmp ult i32 %bias.write.register, RECIPE_REGISTER_N
br i1 %bias.write.more, label %bias.write.step, label %reduce.test
bias.write.step:
%bias.write.lane.base = mul i32 %lid, RECIPE_REGISTER_N
%bias.write.local = add i32 %bias.write.lane.base, %bias.write.register
%bias.write.index = add i32 %bias.base, %bias.write.local
%bias.write.source = getelementptr [RECIPE_REGISTER_N x double], ptr addrspace(5) %biases, i32 0, i32 %bias.write.register
%bias.write.value = load double, ptr addrspace(5) %bias.write.source, align 8
%bias.write.target = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %bias.write.index
store double %bias.write.value, ptr addrspace(3) %bias.write.target, align 8
%bias.write.next = add i32 %bias.write.register, 1
br label %bias.write.loop
reduce.test:
call void @recipe.local.barrier()
%owner = icmp eq i32 %split, 0
%reduce.active = and i1 %lane.active, %owner
br i1 %reduce.active, label %reduce.register.loop, label %reduce.done
reduce.register.loop:
%reduce.register = phi i32 [ 0, %reduce.test ], [ %reduce.register.next, %reduce.register.store ]
%reduce.register.more = icmp ult i32 %reduce.register, RECIPE_REGISTER_COUNT
br i1 %reduce.register.more, label %reduce.split.loop, label %bias.reduce.loop
reduce.split.loop:
%reduce.split = phi i32 [ 1, %reduce.register.loop ], [ %reduce.split.next, %reduce.split.step ]
%reduce.sum = phi double [ 0.0, %reduce.register.loop ], [ %reduce.sum.next, %reduce.split.step ]
%reduce.split.more = icmp ult i32 %reduce.split, %splits
br i1 %reduce.split.more, label %reduce.split.step, label %reduce.register.store
reduce.split.step:
%reduce.split.offset = mul i32 %reduce.split, %output.lanes
%reduce.lane = add i32 %lid, %reduce.split.offset
%reduce.lane.base = mul i32 %reduce.lane, RECIPE_REGISTER_COUNT
%reduce.index = add i32 %reduce.lane.base, %reduce.register
%reduce.source = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %reduce.index
%reduce.value = load double, ptr addrspace(3) %reduce.source, align 8
%reduce.sum.next = call double @recipe.add(double %reduce.sum, double %reduce.value)
%reduce.split.next = add i32 %reduce.split, 1
br label %reduce.split.loop
reduce.register.store:
%reduce.target = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %reduce.register
%reduce.current = load double, ptr addrspace(5) %reduce.target, align 8
%reduce.total = call double @recipe.add(double %reduce.current, double %reduce.sum)
store double %reduce.total, ptr addrspace(5) %reduce.target, align 8
%reduce.register.next = add i32 %reduce.register, 1
br label %reduce.register.loop
bias.reduce.loop:
%bias.reduce.register = phi i32 [ 0, %reduce.register.loop ], [ %bias.reduce.register.next, %bias.reduce.store ]
%bias.reduce.more = icmp ult i32 %bias.reduce.register, RECIPE_REGISTER_N
br i1 %bias.reduce.more, label %bias.reduce.split.loop, label %reduce.done
bias.reduce.split.loop:
%bias.reduce.split = phi i32 [ 1, %bias.reduce.loop ], [ %bias.reduce.split.next, %bias.reduce.split.step ]
%bias.reduce.sum = phi double [ 0.0, %bias.reduce.loop ], [ %bias.reduce.sum.next, %bias.reduce.split.step ]
%bias.reduce.split.more = icmp ult i32 %bias.reduce.split, %splits
br i1 %bias.reduce.split.more, label %bias.reduce.split.step, label %bias.reduce.store
bias.reduce.split.step:
%bias.reduce.split.offset = mul i32 %bias.reduce.split, %output.lanes
%bias.reduce.lane = add i32 %lid, %bias.reduce.split.offset
%bias.reduce.lane.base = mul i32 %bias.reduce.lane, RECIPE_REGISTER_N
%bias.reduce.local = add i32 %bias.reduce.lane.base, %bias.reduce.register
%bias.reduce.index = add i32 %bias.base, %bias.reduce.local
%bias.reduce.source = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %bias.reduce.index
%bias.reduce.value = load double, ptr addrspace(3) %bias.reduce.source, align 8
%bias.reduce.sum.next = call double @recipe.add(double %bias.reduce.sum, double %bias.reduce.value)
%bias.reduce.split.next = add i32 %bias.reduce.split, 1
br label %bias.reduce.split.loop
bias.reduce.store:
%bias.reduce.target = getelementptr [RECIPE_REGISTER_N x double], ptr addrspace(5) %biases, i32 0, i32 %bias.reduce.register
%bias.reduce.current = load double, ptr addrspace(5) %bias.reduce.target, align 8
%bias.reduce.total = call double @recipe.add(double %bias.reduce.current, double %bias.reduce.sum)
store double %bias.reduce.total, ptr addrspace(5) %bias.reduce.target, align 8
%bias.reduce.register.next = add i32 %bias.reduce.register, 1
br label %bias.reduce.loop
reduce.done:
call void @recipe.local.barrier()
ret void
}
define internal void @contraction_forward_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output, i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels, i32 %out.length, i32 %kernel,
i1 %has.bias, i1 %relu, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry:
%tile.b.k = add i32 %tile.k, RECIPE_WMMA_B_PADDING
%sums = alloca [RECIPE_REGISTER_COUNT x double], align 8, addrspace(5) %lid = call i32 @recipe.local.id.x() %group = call i32 @recipe.group.id.x() %block = call i32 @recipe.workgroup.size.x() %groups = udiv i32 %threads, %block
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length %is.conv = icmp ne i32 %kernel, 0 %span = select i1 %is.conv, i32 %kernel, i32 1 %terms = mul i32 %in.channels, %span %m.total = mul i32 %rows, %out.length
%m.short = icmp ult i32 %tile.m, %m.total %m.tile = select i1 %m.short, i32 %tile.m, i32 %m.total %n.short = icmp ult i32 %tile.n, %out.channels %n.tile = select i1 %n.short, i32 %tile.n, i32 %out.channels %k.short = icmp ult i32 %tile.k, %terms %k.tile = select i1 %k.short, i32 %tile.k, i32 %terms
%m.adjusted = add i32 %m.total, %m.tile %m.numerator = sub i32 %m.adjusted, 1 %m.tiles = udiv i32 %m.numerator, %m.tile %n.adjusted = add i32 %out.channels, %n.tile %n.numerator = sub i32 %n.adjusted, 1 %n.tiles = udiv i32 %n.numerator, %n.tile %jobs = mul i32 %m.tiles, %n.tiles br label %job.loop job.loop:
%job = phi i32 [ %group, %entry ], [ %job.next, %job.done ] %job.more = icmp ult i32 %job, %jobs br i1 %job.more, label %job.step, label %exit job.step:
%m.group.short = icmp ult i32 %m.tiles, RECIPE_CONTRACTION_SWIZZLE_M %m.group.limit = select i1 %m.group.short, i32 %m.tiles, i32 RECIPE_CONTRACTION_SWIZZLE_M %group.width = mul i32 %m.group.limit, %n.tiles %group.index = udiv i32 %job, %group.width %m.group.base = mul i32 %group.index, %m.group.limit %m.group.remaining = sub i32 %m.tiles, %m.group.base %m.group.tail = icmp ult i32 %m.group.remaining, %m.group.limit %m.group.count = select i1 %m.group.tail, i32 %m.group.remaining, i32 %m.group.limit %group.local = urem i32 %job, %group.width %m.group.local = urem i32 %group.local, %m.group.count %m.tile.index = add i32 %m.group.base, %m.group.local %n.tile.index = udiv i32 %group.local, %m.group.count %m.base = mul i32 %m.tile.index, %m.tile %n.base = mul i32 %n.tile.index, %n.tile
%m.remaining = sub i32 %m.total, %m.base %m.partial = icmp ult i32 %m.remaining, %m.tile %m.count = select i1 %m.partial, i32 %m.remaining, i32 %m.tile %n.remaining = sub i32 %out.channels, %n.base %n.partial = icmp ult i32 %n.remaining, %n.tile %n.count = select i1 %n.partial, i32 %n.remaining, i32 %n.tile
%m.lanes.adjusted = add i32 %m.count, RECIPE_REGISTER_M %m.lanes.numerator = sub i32 %m.lanes.adjusted, 1 %m.lanes = udiv i32 %m.lanes.numerator, RECIPE_REGISTER_M %n.lanes.adjusted = add i32 %n.count, RECIPE_REGISTER_N %n.lanes.numerator = sub i32 %n.lanes.adjusted, 1 %n.lanes = udiv i32 %n.lanes.numerator, RECIPE_REGISTER_N
%lanes = call i32 @contraction_output_lanes(i32 %m.lanes, i32 %n.lanes, i32 %block) %lane.active = icmp ult i32 %lid, %lanes %lane.n.raw = udiv i32 %lid, %m.lanes %lane.m.raw = urem i32 %lid, %m.lanes %lane.n = select i1 %lane.active, i32 %lane.n.raw, i32 0 %lane.m = select i1 %lane.active, i32 %lane.m.raw, i32 0
%output.m.base = mul i32 %lane.m, RECIPE_REGISTER_M %output.n.base = mul i32 %lane.n, RECIPE_REGISTER_N br label %sum.init.loop sum.init.loop:
%sum.init = phi i32 [ 0, %job.step ], [ %sum.init.next, %sum.init.step ] %sum.init.more = icmp ult i32 %sum.init, RECIPE_REGISTER_COUNT br i1 %sum.init.more, label %sum.init.step, label %sum.init.done
sum.init.step: %sum.init.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %sum.init store double 0.0, ptr addrspace(5) %sum.init.ptr, align 8 %sum.init.next = add i32 %sum.init, 1 br label %sum.init.loop sum.init.done: br label %tile.loop tile.loop:
%term.base = phi i32 [ 0, %sum.init.done ], [ %term.next, %tile.done ] %k.remaining = sub i32 %terms, %term.base %k.partial = icmp ult i32 %k.remaining, %k.tile %k.count = select i1 %k.partial, i32 %k.remaining, i32 %k.tile
%a.project = icmp eq i32 %span, 1
%a.unit = icmp eq i32 %in.length, 1
%a.contiguous = and i1 %a.project, %a.unit
%a.fragment.remainder = urem i32 %k.count, RECIPE_WMMA_K
%a.fragment.full = icmp eq i32 %a.fragment.remainder, 0
%a.vector = and i1 %a.contiguous, %a.fragment.full
%a.width = select i1 %a.vector, i32 RECIPE_WMMA_K, i32 1
%a.columns = udiv i32 %k.count, %a.width
%b.fragment.remainder = urem i32 %k.count, RECIPE_WMMA_K
%b.vector = icmp eq i32 %b.fragment.remainder, 0
%b.width = select i1 %b.vector, i32 RECIPE_WMMA_K, i32 1
%b.rows = udiv i32 %k.count, %b.width
%a.count = mul i32 %m.count, %a.columns %b.count = mul i32 %n.count, %b.rows %load.count = add i32 %a.count, %b.count br label %load.loop load.loop:
%load = phi i32 [ %lid, %tile.loop ], [ %load.next, %load.advance ] %load.more = icmp ult i32 %load, %load.count br i1 %load.more, label %load.classify, label %load.done load.classify: %load.a = icmp ult i32 %load, %a.count br i1 %load.a, label %load.a.step, label %load.b.step
load.a.step: %a.m = udiv i32 %load, %a.columns %a.column = urem i32 %load, %a.columns %a.k = mul i32 %a.column, %a.width %a.global = add i32 %m.base, %a.m %a.row = udiv i32 %a.global, %out.length %a.position = urem i32 %a.global, %out.length %a.row.base = mul i32 %a.row, %in.elements %a.term = add i32 %term.base, %a.k
%a.tile.row = mul i32 %a.k, %tile.m %a.tile.index = add i32 %a.tile.row, %a.m
br i1 %a.vector, label %load.a.vector, label %load.a.scalar
load.a.vector:
%a.vector.index = add i32 %a.row.base, %a.term
%a.vector.source = getelementptr inbounds double, ptr addrspace(1) %input, i32 %a.vector.index
%a.vector.value = load <RECIPE_WMMA_K x double>, ptr addrspace(1) %a.vector.source, align 8
call void @contraction_stage_a_fragment(<RECIPE_WMMA_K x double> %a.vector.value, i32 %a.k, i32 %a.m, i32 %tile.m)
br label %load.advance
load.a.scalar:
%a.value = call double @contraction_input( ptr addrspace(1) %input, i32 %a.row.base, i32 %a.position, i32 %a.term, i32 %span, i32 %in.length, i1 %is.conv )
br label %load.store
load.b.step: %b.local = sub i32 %load, %a.count %b.n = udiv i32 %b.local, %b.rows %b.row = urem i32 %b.local, %b.rows %b.k = mul i32 %b.row, %b.width %b.channel = add i32 %n.base, %b.n %b.channel.base = mul i32 %b.channel, %terms %b.term = add i32 %term.base, %b.k
%b.index = add i32 %b.channel.base, %b.term %b.tile.base = mul i32 %tile.m, %tile.k %b.tile.local = call i32 @contraction_b_index(i32 %b.k, i32 %b.n, i32 %tile.n, i32 %tile.b.k) %b.tile.index = add i32 %b.tile.base, %b.tile.local
br i1 %b.vector, label %load.b.vector, label %load.b.scalar
load.b.vector:
%b.vector.source = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %b.index
%b.vector.value = load <RECIPE_WMMA_K x double>, ptr addrspace(1) %b.vector.source, align 8
%b.vector.target = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %b.tile.index
store <RECIPE_WMMA_K x double> %b.vector.value, ptr addrspace(3) %b.vector.target, align 8
br label %load.advance
load.b.scalar:
%b.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %b.index
%b.value = load double, ptr addrspace(1) %b.ptr, align 8
br label %load.store
load.store: %load.value = phi double [ %a.value, %load.a.scalar ], [ %b.value, %load.b.scalar ] %load.tile.index = phi i32 [ %a.tile.index, %load.a.scalar ], [ %b.tile.index, %load.b.scalar ] %load.tile.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %load.tile.index store double %load.value, ptr addrspace(3) %load.tile.ptr, align 8
br label %load.advance
load.advance:
%load.next = add i32 %load, %block br label %load.loop load.done:
%load.logical.output.edge = or i1 %m.partial, %n.partial
%load.logical.edge = or i1 %load.logical.output.edge, %k.partial
%load.m.edge = icmp ult i32 %m.count, %tile.m
%load.n.edge = icmp ult i32 %n.count, %tile.n
%load.k.edge = icmp ult i32 %k.count, %tile.k
%load.schedule.output.edge = or i1 %load.m.edge, %load.n.edge
%load.schedule.edge = or i1 %load.schedule.output.edge, %load.k.edge
%load.vector.edge = select i1 RECIPE_CONTRACTION_MATRIX, i1 %load.schedule.edge, i1 %load.logical.edge
br i1 %load.vector.edge, label %load.zero, label %load.ready
load.zero:
call void @contraction_zero_edges(i32 %m.count, i32 %n.count, i32 %k.count, i32 %lid, i32 %block, i32 %tile.m, i32 %tile.n, i32 %tile.k)
br label %load.ready
load.ready:
call void @recipe.local.barrier() call void @contraction_accumulate(ptr addrspace(5) %sums, ptr addrspace(5) null, i1 false, i1 %lane.active, i32 %output.m.base, i32 %output.n.base, i32 %m.count, i32 %n.count, i32 0, i32 1, i32 %k.count, i32 %tile.m, i32 %tile.n, i32 %tile.k) call void @recipe.local.barrier()
%term.next = add i32 %term.base, %k.count %term.more = icmp ult i32 %term.next, %terms br i1 %term.more, label %tile.done, label %store.loop tile.done: br label %tile.loop store.loop:
%store.register = phi i32 [ 0, %load.ready ], [ %store.register.next, %store.next ] %store.more = icmp ult i32 %store.register, RECIPE_REGISTER_COUNT br i1 %store.more, label %store.test, label %job.done
store.test: %store.output.m.raw = call i32 @contraction_output_m(i32 %lid, i32 %store.register, i32 %m.lanes) %store.output.n.raw = call i32 @contraction_output_n(i32 %lid, i32 %store.register, i32 %m.lanes) %store.register.valid = call i1 @contraction_output_register_valid(i32 %store.register)
%store.output.m.valid = icmp ult i32 %store.output.m.raw, %m.count %store.output.n.valid = icmp ult i32 %store.output.n.raw, %n.count %store.output.valid = and i1 %store.output.m.valid, %store.output.n.valid %store.lane.active = and i1 %lane.active, %store.output.valid %store.active = and i1 %store.lane.active, %store.register.valid br i1 %store.active, label %store, label %store.next
store: %store.channel = add i32 %n.base, %store.output.n.raw %store.m.global = add i32 %m.base, %store.output.m.raw %store.position = urem i32 %store.m.global, %out.length %store.row = udiv i32 %store.m.global, %out.length %store.output.row.base = mul i32 %store.row, %out.elements
%store.output.channel.base = mul i32 %store.channel, %out.length %store.output.local = add i32 %store.output.channel.base, %store.position %store.output.index = add i32 %store.output.row.base, %store.output.local %store.output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %store.output.index
%store.bias.base = mul i32 %out.channels, %terms %store.bias.index = add i32 %store.bias.base, %store.channel %store.bias.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %store.bias.index %store.bias = load double, ptr addrspace(1) %store.bias.ptr, align 8 %store.sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %store.register %store.sum = load double, ptr addrspace(5) %store.sum.ptr, align 8
%store.biased = call double @recipe.add(double %store.sum, double %store.bias) %store.raw = select i1 %has.bias, double %store.biased, double %store.sum %store.positive = call i1 @recipe.ogt(double %store.raw, double 0.0) %store.activated = select i1 %store.positive, double %store.raw, double 0.0 %store.result = select i1 %relu, double %store.activated, double %store.raw store double %store.result, ptr addrspace(1) %store.output.ptr, align 8 br label %store.next
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
i32 %length, i32 0, i1 false, i1 false, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )
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
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output, ptr addrspace(1) %delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.input, i1 %has.bias, i1 %relu,
i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels, i32 %out.length, i32 %kernel, i32 %offset,
i32 %gradient.tile.m, i32 %gradient.tile.n, i32 %gradient.tile.k, i32 %previous.tile.m, i32 %previous.tile.n, i32 %previous.tile.k, i32 %threads ) #1 { entry:
%gradient.tile.b.k = add i32 %gradient.tile.k, RECIPE_WMMA_B_PADDING
%previous.tile.b.k = add i32 %previous.tile.k, RECIPE_WMMA_B_PADDING
%sums = alloca [RECIPE_REGISTER_COUNT x double], align 8, addrspace(5) %biases = alloca [RECIPE_REGISTER_N x double], align 8, addrspace(5) %lid = call i32 @recipe.local.id.x() %group = call i32 @recipe.group.id.x() %block = call i32 @recipe.workgroup.size.x() %groups = udiv i32 %threads, %block
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length %is.conv = icmp ne i32 %kernel, 0
%span = select i1 %is.conv, i32 %kernel, i32 1 %window = mul i32 %in.channels, %span
%gradient.r.total = mul i32 %rows, %out.length
%gradient.matrix.values = mul i32 %out.channels, %window
%gradient.bias.values = select i1 %has.bias, i32 %out.channels, i32 0
%gradient.values = add i32 %gradient.matrix.values, %gradient.bias.values
%gradient.scratch = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 RECIPE_GRADIENT_ELEMENTS
%gradient.m.short = icmp ult i32 %gradient.tile.m, %window %gradient.m.tile = select i1 %gradient.m.short, i32 %gradient.tile.m, i32 %window %gradient.n.short = icmp ult i32 %gradient.tile.n, %out.channels %gradient.n.tile = select i1 %gradient.n.short, i32 %gradient.tile.n, i32 %out.channels
%gradient.k.short = icmp ult i32 %gradient.tile.k, %gradient.r.total %gradient.k.tile = select i1 %gradient.k.short, i32 %gradient.tile.k, i32 %gradient.r.total
%gradient.m.adjusted = add i32 %window, %gradient.m.tile %gradient.m.numerator = sub i32 %gradient.m.adjusted, 1 %gradient.m.tiles = udiv i32 %gradient.m.numerator, %gradient.m.tile %gradient.n.adjusted = add i32 %out.channels, %gradient.n.tile %gradient.n.numerator = sub i32 %gradient.n.adjusted, 1 %gradient.n.tiles = udiv i32 %gradient.n.numerator, %gradient.n.tile
%gradient.jobs = mul i32 %gradient.m.tiles, %gradient.n.tiles
%gradient.r.quotient = udiv i32 %gradient.r.total, %gradient.k.tile
%gradient.r.remainder = urem i32 %gradient.r.total, %gradient.k.tile
%gradient.r.has.remainder = icmp ne i32 %gradient.r.remainder, 0
%gradient.r.extra = select i1 %gradient.r.has.remainder, i32 1, i32 0
%gradient.r.tiles = add i32 %gradient.r.quotient, %gradient.r.extra
%gradient.splits.raw = udiv i32 RECIPE_CONTRACTION_GRADIENT_TASKS, %gradient.jobs
%gradient.splits.empty = icmp eq i32 %gradient.splits.raw, 0
%gradient.splits.nonzero = select i1 %gradient.splits.empty, i32 1, i32 %gradient.splits.raw
%gradient.splits.large = icmp ugt i32 %gradient.splits.nonzero, %gradient.r.tiles
%gradient.splits = select i1 %gradient.splits.large, i32 %gradient.r.tiles, i32 %gradient.splits.nonzero
%gradient.direct = icmp eq i32 %gradient.splits, 1
%gradient.destination.base = select i1 %gradient.direct, i32 %offset, i32 0
%gradient.destination = select i1 %gradient.direct, ptr addrspace(1) %gradient, ptr addrspace(1) %gradient.scratch
%gradient.tasks = mul i32 %gradient.jobs, %gradient.splits
%gradient.r.stride = mul i32 %gradient.splits, %gradient.k.tile
br label %gradient.job.loop
gradient.job.loop:
%gradient.task = phi i32 [ %group, %entry ], [ %gradient.task.next, %gradient.job.done ]
%gradient.task.more = icmp ult i32 %gradient.task, %gradient.tasks
br i1 %gradient.task.more, label %gradient.job.step, label %gradient.finish
gradient.job.step:
%gradient.job = udiv i32 %gradient.task, %gradient.splits
%gradient.split = urem i32 %gradient.task, %gradient.splits
%gradient.store.row = mul i32 %gradient.split, %gradient.values
%gradient.store.offset = add i32 %gradient.destination.base, %gradient.store.row
%gradient.r.first = mul i32 %gradient.split, %gradient.k.tile
%gradient.m.group.short = icmp ult i32 %gradient.m.tiles, RECIPE_CONTRACTION_SWIZZLE_M %gradient.m.group.limit = select i1 %gradient.m.group.short, i32 %gradient.m.tiles, i32 RECIPE_CONTRACTION_SWIZZLE_M %gradient.group.width = mul i32 %gradient.m.group.limit, %gradient.n.tiles %gradient.group.index = udiv i32 %gradient.job, %gradient.group.width %gradient.m.group.base = mul i32 %gradient.group.index, %gradient.m.group.limit %gradient.m.group.remaining = sub i32 %gradient.m.tiles, %gradient.m.group.base %gradient.m.group.tail = icmp ult i32 %gradient.m.group.remaining, %gradient.m.group.limit %gradient.m.group.count = select i1 %gradient.m.group.tail, i32 %gradient.m.group.remaining, i32 %gradient.m.group.limit %gradient.group.local = urem i32 %gradient.job, %gradient.group.width %gradient.m.group.local = urem i32 %gradient.group.local, %gradient.m.group.count %gradient.m.index = add i32 %gradient.m.group.base, %gradient.m.group.local %gradient.n.index = udiv i32 %gradient.group.local, %gradient.m.group.count %gradient.m.base = mul i32 %gradient.m.index, %gradient.m.tile %gradient.n.base = mul i32 %gradient.n.index, %gradient.n.tile
%gradient.m.remaining = sub i32 %window, %gradient.m.base %gradient.m.partial = icmp ult i32 %gradient.m.remaining, %gradient.m.tile %gradient.m.count = select i1 %gradient.m.partial, i32 %gradient.m.remaining, i32 %gradient.m.tile
%gradient.n.remaining = sub i32 %out.channels, %gradient.n.base %gradient.n.partial = icmp ult i32 %gradient.n.remaining, %gradient.n.tile %gradient.n.count = select i1 %gradient.n.partial, i32 %gradient.n.remaining, i32 %gradient.n.tile
%gradient.m.lanes.adjusted = add i32 %gradient.m.count, RECIPE_REGISTER_M %gradient.m.lanes.numerator = sub i32 %gradient.m.lanes.adjusted, 1 %gradient.m.lanes = udiv i32 %gradient.m.lanes.numerator, RECIPE_REGISTER_M %gradient.n.lanes.adjusted = add i32 %gradient.n.count, RECIPE_REGISTER_N %gradient.n.lanes.numerator = sub i32 %gradient.n.lanes.adjusted, 1 %gradient.n.lanes = udiv i32 %gradient.n.lanes.numerator, RECIPE_REGISTER_N
%gradient.output.lanes = call i32 @contraction_output_lanes(i32 %gradient.m.lanes, i32 %gradient.n.lanes, i32 %block)
%gradient.k.lanes.raw = udiv i32 %block, %gradient.output.lanes
%gradient.k.lanes.more = icmp ugt i32 %gradient.k.lanes.raw, 0
%gradient.k.lanes = select i1 %gradient.k.lanes.more, i32 %gradient.k.lanes.raw, i32 1
%gradient.active.lanes = mul i32 %gradient.output.lanes, %gradient.k.lanes
%gradient.lane.active = icmp ult i32 %lid, %gradient.active.lanes
%gradient.lane.output.raw = urem i32 %lid, %gradient.output.lanes
%gradient.lane.output = select i1 %gradient.lane.active, i32 %gradient.lane.output.raw, i32 0
%gradient.lane.k.raw = udiv i32 %lid, %gradient.output.lanes
%gradient.lane.k = select i1 %gradient.lane.active, i32 %gradient.lane.k.raw, i32 0
%gradient.lane.n.raw = udiv i32 %gradient.lane.output, %gradient.m.lanes
%gradient.lane.m.raw = urem i32 %gradient.lane.output, %gradient.m.lanes
%gradient.lane.n = select i1 %gradient.lane.active, i32 %gradient.lane.n.raw, i32 0
%gradient.lane.m = select i1 %gradient.lane.active, i32 %gradient.lane.m.raw, i32 0
%gradient.lane.owner = icmp eq i32 %gradient.lane.k, 0
%gradient.lane.store = and i1 %gradient.lane.active, %gradient.lane.owner
%gradient.output.m.base = mul i32 %gradient.lane.m, RECIPE_REGISTER_M %gradient.output.n.base = mul i32 %gradient.lane.n, RECIPE_REGISTER_N br label %gradient.sum.init.loop gradient.sum.init.loop:
%gradient.sum.init = phi i32 [ 0, %gradient.job.step ], [ %gradient.sum.init.next, %gradient.sum.init.step ] %gradient.sum.init.more = icmp ult i32 %gradient.sum.init, RECIPE_REGISTER_COUNT br i1 %gradient.sum.init.more, label %gradient.sum.init.step, label %gradient.bias.init.loop
gradient.sum.init.step: %gradient.sum.init.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %gradient.sum.init store double 0.0, ptr addrspace(5) %gradient.sum.init.ptr, align 8 %gradient.sum.init.next = add i32 %gradient.sum.init, 1 br label %gradient.sum.init.loop gradient.bias.init.loop:
%gradient.bias.init = phi i32 [ 0, %gradient.sum.init.loop ], [ %gradient.bias.init.next, %gradient.bias.init.step ] %gradient.bias.init.more = icmp ult i32 %gradient.bias.init, RECIPE_REGISTER_N br i1 %gradient.bias.init.more, label %gradient.bias.init.step, label %gradient.tile.loop
gradient.bias.init.step: %gradient.bias.init.ptr = getelementptr [RECIPE_REGISTER_N x double], ptr addrspace(5) %biases, i32 0, i32 %gradient.bias.init store double 0.0, ptr addrspace(5) %gradient.bias.init.ptr, align 8 %gradient.bias.init.next = add i32 %gradient.bias.init, 1 br label %gradient.bias.init.loop gradient.tile.loop:
%gradient.r.base = phi i32 [ %gradient.r.first, %gradient.bias.init.loop ], [ %gradient.r.next, %gradient.tile.done ]
%gradient.r.remaining = sub i32 %gradient.r.total, %gradient.r.base %gradient.r.partial = icmp ult i32 %gradient.r.remaining, %gradient.k.tile %gradient.r.count = select i1 %gradient.r.partial, i32 %gradient.r.remaining, i32 %gradient.k.tile
%gradient.a.project = icmp eq i32 %span, 1
%gradient.a.unit = icmp eq i32 %in.length, 1
%gradient.a.contiguous = and i1 %gradient.a.project, %gradient.a.unit
%gradient.a.fragment.remainder = urem i32 %gradient.m.count, RECIPE_WMMA_K
%gradient.a.fragment.full = icmp eq i32 %gradient.a.fragment.remainder, 0
%gradient.a.vector = and i1 %gradient.a.contiguous, %gradient.a.fragment.full
%gradient.a.width = select i1 %gradient.a.vector, i32 RECIPE_WMMA_K, i32 1
%gradient.a.columns = udiv i32 %gradient.m.count, %gradient.a.width
%gradient.b.unit = icmp eq i32 %out.length, 1
%gradient.b.fragment.remainder = urem i32 %gradient.n.count, RECIPE_WMMA_K
%gradient.b.fragment.full = icmp eq i32 %gradient.b.fragment.remainder, 0
%gradient.b.vector = and i1 %gradient.b.unit, %gradient.b.fragment.full
%gradient.b.width = select i1 %gradient.b.vector, i32 RECIPE_WMMA_K, i32 1
%gradient.b.columns = udiv i32 %gradient.n.count, %gradient.b.width
%gradient.a.count = mul i32 %gradient.a.columns, %gradient.r.count %gradient.b.count = mul i32 %gradient.b.columns, %gradient.r.count %gradient.load.count = add i32 %gradient.a.count, %gradient.b.count br label %gradient.load.loop gradient.load.loop:
%gradient.load = phi i32 [ %lid, %gradient.tile.loop ], [ %gradient.load.next, %gradient.load.advance ] %gradient.load.more = icmp ult i32 %gradient.load, %gradient.load.count br i1 %gradient.load.more, label %gradient.load.classify, label %gradient.load.done
gradient.load.classify: %gradient.load.a = icmp ult i32 %gradient.load, %gradient.a.count br i1 %gradient.load.a, label %gradient.load.a.step, label %gradient.load.b.step
gradient.load.a.step: %gradient.a.r = udiv i32 %gradient.load, %gradient.a.columns %gradient.a.column = urem i32 %gradient.load, %gradient.a.columns %gradient.a.m = mul i32 %gradient.a.column, %gradient.a.width %gradient.a.global = add i32 %gradient.r.base, %gradient.a.r
%gradient.a.row = udiv i32 %gradient.a.global, %out.length %gradient.a.position = urem i32 %gradient.a.global, %out.length %gradient.a.row.base = mul i32 %gradient.a.row, %in.elements %gradient.a.term = add i32 %gradient.m.base, %gradient.a.m
%gradient.a.tile.row = mul i32 %gradient.a.r, %gradient.tile.m %gradient.a.tile.index = add i32 %gradient.a.tile.row, %gradient.a.m
br i1 %gradient.a.vector, label %gradient.load.a.vector, label %gradient.load.a.scalar
gradient.load.a.vector:
%gradient.a.vector.index = add i32 %gradient.a.row.base, %gradient.a.term
%gradient.a.vector.source = getelementptr inbounds double, ptr addrspace(1) %input, i32 %gradient.a.vector.index
%gradient.a.vector.value = load <RECIPE_WMMA_K x double>, ptr addrspace(1) %gradient.a.vector.source, align 8
%gradient.a.vector.target = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %gradient.a.tile.index
store <RECIPE_WMMA_K x double> %gradient.a.vector.value, ptr addrspace(3) %gradient.a.vector.target, align 8
br label %gradient.load.advance
gradient.load.a.scalar:
%gradient.a.value = call double @contraction_input( ptr addrspace(1) %input, i32 %gradient.a.row.base, i32 %gradient.a.position, i32 %gradient.a.term, i32 %span, i32 %in.length, i1 %is.conv )
br label %gradient.load.store
gradient.load.b.step: %gradient.b.local = sub i32 %gradient.load, %gradient.a.count %gradient.b.r = udiv i32 %gradient.b.local, %gradient.b.columns %gradient.b.column = urem i32 %gradient.b.local, %gradient.b.columns %gradient.b.n = mul i32 %gradient.b.column, %gradient.b.width %gradient.b.global = add i32 %gradient.r.base, %gradient.b.r
%gradient.b.row = udiv i32 %gradient.b.global, %out.length %gradient.b.position = urem i32 %gradient.b.global, %out.length %gradient.b.filter = add i32 %gradient.n.base, %gradient.b.n
%gradient.b.row.base = mul i32 %gradient.b.row, %out.elements %gradient.b.filter.base = mul i32 %gradient.b.filter, %out.length %gradient.b.local.index = add i32 %gradient.b.filter.base, %gradient.b.position %gradient.b.index = add i32 %gradient.b.row.base, %gradient.b.local.index
%gradient.b.tile.base = mul i32 %gradient.tile.m, %gradient.tile.k
%gradient.b.tile.local = call i32 @contraction_b_index(i32 %gradient.b.r, i32 %gradient.b.n, i32 %gradient.tile.n, i32 %gradient.tile.b.k) %gradient.b.tile.index = add i32 %gradient.b.tile.base, %gradient.b.tile.local
br i1 %gradient.b.vector, label %gradient.load.b.vector, label %gradient.load.b.scalar
gradient.load.b.vector:
%gradient.b.vector.delta = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %gradient.b.index
%gradient.b.vector.output = getelementptr inbounds double, ptr addrspace(1) %output, i32 %gradient.b.index
%gradient.b.vector.delta.value = load <RECIPE_WMMA_K x double>, ptr addrspace(1) %gradient.b.vector.delta, align 8
%gradient.b.vector.output.value = load <RECIPE_WMMA_K x double>, ptr addrspace(1) %gradient.b.vector.output, align 8
call void @contraction_stage_delta_b_fragment(<RECIPE_WMMA_K x double> %gradient.b.vector.delta.value, <RECIPE_WMMA_K x double> %gradient.b.vector.output.value, i1 %relu, i32 %gradient.b.r, i32 %gradient.b.n, i32 %gradient.tile.m, i32 %gradient.tile.n, i32 %gradient.tile.k)
br label %gradient.load.advance
gradient.load.b.scalar:
%gradient.b.value = call double @contraction_delta(ptr addrspace(1) %delta, ptr addrspace(1) %output, i32 %gradient.b.index, i1 %relu)
br label %gradient.load.store
gradient.load.store: %gradient.load.value = phi double [ %gradient.a.value, %gradient.load.a.scalar ], [ %gradient.b.value, %gradient.load.b.scalar ] %gradient.load.index = phi i32 [ %gradient.a.tile.index, %gradient.load.a.scalar ], [ %gradient.b.tile.index, %gradient.load.b.scalar ]
%gradient.load.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %gradient.load.index store double %gradient.load.value, ptr addrspace(3) %gradient.load.ptr, align 8
br label %gradient.load.advance
gradient.load.advance:
%gradient.load.next = add i32 %gradient.load, %block br label %gradient.load.loop gradient.load.done:
%gradient.load.logical.output.edge = or i1 %gradient.m.partial, %gradient.n.partial
%gradient.load.logical.edge = or i1 %gradient.load.logical.output.edge, %gradient.r.partial
%gradient.load.m.edge = icmp ult i32 %gradient.m.count, %gradient.tile.m
%gradient.load.n.edge = icmp ult i32 %gradient.n.count, %gradient.tile.n
%gradient.load.k.edge = icmp ult i32 %gradient.r.count, %gradient.tile.k
%gradient.load.schedule.output.edge = or i1 %gradient.load.m.edge, %gradient.load.n.edge
%gradient.load.schedule.edge = or i1 %gradient.load.schedule.output.edge, %gradient.load.k.edge
%gradient.load.vector.edge = select i1 RECIPE_CONTRACTION_MATRIX, i1 %gradient.load.schedule.edge, i1 %gradient.load.logical.edge
br i1 %gradient.load.vector.edge, label %gradient.load.zero, label %gradient.load.ready
gradient.load.zero:
call void @contraction_zero_edges(i32 %gradient.m.count, i32 %gradient.n.count, i32 %gradient.r.count, i32 %lid, i32 %block, i32 %gradient.tile.m, i32 %gradient.tile.n, i32 %gradient.tile.k)
br label %gradient.load.ready
gradient.load.ready:
call void @recipe.local.barrier()
%gradient.bias.enable = call i1 @contraction_bias_enable(i1 %has.bias, i32 %gradient.m.base, i32 %gradient.output.m.base, i32 %lid) call void @contraction_accumulate(ptr addrspace(5) %sums, ptr addrspace(5) %biases, i1 %gradient.bias.enable, i1 %gradient.lane.active, i32 %gradient.output.m.base, i32 %gradient.output.n.base, i32 %gradient.m.count, i32 %gradient.n.count, i32 %gradient.lane.k, i32 %gradient.k.lanes, i32 %gradient.r.count, i32 %gradient.tile.m, i32 %gradient.tile.n, i32 %gradient.tile.k) call void @recipe.local.barrier()
%gradient.r.next = add i32 %gradient.r.base, %gradient.r.stride %gradient.r.more = icmp ult i32 %gradient.r.next, %gradient.r.total br i1 %gradient.r.more, label %gradient.tile.done, label %gradient.reduce gradient.tile.done: br label %gradient.tile.loop gradient.reduce:
call void @contraction_reduce(ptr addrspace(5) %sums, ptr addrspace(5) %biases, i1 %gradient.lane.active, i32 %lid, i32 %gradient.output.lanes, i32 %gradient.lane.k, i32 %gradient.k.lanes, i32 %block)
br label %gradient.store.loop
gradient.store.loop:
%gradient.store.register = phi i32 [ 0, %gradient.reduce ], [ %gradient.store.register.next, %gradient.store.next ] %gradient.store.more = icmp ult i32 %gradient.store.register, RECIPE_REGISTER_COUNT br i1 %gradient.store.more, label %gradient.store.test, label %gradient.job.done
gradient.store.test: %gradient.store.register.m = urem i32 %gradient.store.register, RECIPE_REGISTER_M %gradient.store.register.n = udiv i32 %gradient.store.register, RECIPE_REGISTER_M %gradient.store.output.m.raw = call i32 @contraction_output_m(i32 %lid, i32 %gradient.store.register, i32 %gradient.m.lanes) %gradient.store.output.n.raw = call i32 @contraction_output_n(i32 %lid, i32 %gradient.store.register, i32 %gradient.m.lanes) %gradient.store.register.valid = call i1 @contraction_output_register_valid(i32 %gradient.store.register)
%gradient.store.output.m.valid = icmp ult i32 %gradient.store.output.m.raw, %gradient.m.count %gradient.store.output.n.valid = icmp ult i32 %gradient.store.output.n.raw, %gradient.n.count %gradient.store.output.valid = and i1 %gradient.store.output.m.valid, %gradient.store.output.n.valid %gradient.store.lane.active = and i1 %gradient.lane.store, %gradient.store.output.valid %gradient.store.active = and i1 %gradient.store.lane.active, %gradient.store.register.valid br i1 %gradient.store.active, label %gradient.store, label %gradient.store.next
gradient.store: %gradient.store.filter = add i32 %gradient.n.base, %gradient.store.output.n.raw %gradient.store.term = add i32 %gradient.m.base, %gradient.store.output.m.raw %gradient.store.filter.base = mul i32 %gradient.store.filter, %window %gradient.store.local = add i32 %gradient.store.filter.base, %gradient.store.term %gradient.store.index = add i32 %gradient.store.offset, %gradient.store.local
%gradient.store.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient.destination, i32 %gradient.store.index %gradient.store.sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %gradient.store.register %gradient.store.sum = load double, ptr addrspace(5) %gradient.store.sum.ptr, align 8 store double %gradient.store.sum, ptr addrspace(1) %gradient.store.ptr, align 8
%gradient.store.bias.term = icmp eq i32 %gradient.store.term, 0 %gradient.store.bias.active = and i1 %has.bias, %gradient.store.bias.term br i1 %gradient.store.bias.active, label %gradient.bias.store, label %gradient.store.next gradient.bias.store:
%gradient.store.bias.base = mul i32 %out.channels, %window %gradient.store.bias.local = add i32 %gradient.store.bias.base, %gradient.store.filter %gradient.store.bias.index = add i32 %gradient.store.offset, %gradient.store.bias.local %gradient.store.bias.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient.destination, i32 %gradient.store.bias.index
%gradient.store.bias.value.ptr = getelementptr [RECIPE_REGISTER_N x double], ptr addrspace(5) %biases, i32 0, i32 %gradient.store.register.n %gradient.store.bias.value = load double, ptr addrspace(5) %gradient.store.bias.value.ptr, align 8 store double %gradient.store.bias.value, ptr addrspace(1) %gradient.store.bias.ptr, align 8 br label %gradient.store.next
gradient.store.next: %gradient.store.register.next = add i32 %gradient.store.register, 1 br label %gradient.store.loop gradient.job.done: %gradient.task.next = add i32 %gradient.task, %groups br label %gradient.job.loop
gradient.finish:
br i1 %gradient.direct, label %previous.test, label %gradient.reduce.entry
gradient.reduce.entry:
call void @llvm.amdgcn.s.barrier()
call void @reduce_rows(ptr addrspace(1) %gradient.scratch, ptr addrspace(1) %gradient, i32 %gradient.splits, i32 %gradient.values, i32 0, i32 %offset, i32 %threads)
br label %previous.test
previous.test: br i1 %write.input, label %previous.entry, label %exit previous.entry:
%previous.m.total = mul i32 %rows, %in.length %previous.r.total = mul i32 %out.channels, %span
%previous.m.short = icmp ult i32 %previous.tile.m, %previous.m.total %previous.m.tile = select i1 %previous.m.short, i32 %previous.tile.m, i32 %previous.m.total %previous.n.short = icmp ult i32 %previous.tile.n, %in.channels %previous.n.tile = select i1 %previous.n.short, i32 %previous.tile.n, i32 %in.channels
%previous.k.short = icmp ult i32 %previous.tile.k, %previous.r.total %previous.k.tile = select i1 %previous.k.short, i32 %previous.tile.k, i32 %previous.r.total
%previous.m.adjusted = add i32 %previous.m.total, %previous.m.tile %previous.m.numerator = sub i32 %previous.m.adjusted, 1 %previous.m.tiles = udiv i32 %previous.m.numerator, %previous.m.tile %previous.n.adjusted = add i32 %in.channels, %previous.n.tile %previous.n.numerator = sub i32 %previous.n.adjusted, 1 %previous.n.tiles = udiv i32 %previous.n.numerator, %previous.n.tile
%previous.jobs = mul i32 %previous.m.tiles, %previous.n.tiles br label %previous.job.loop previous.job.loop:
%previous.job = phi i32 [ %group, %previous.entry ], [ %previous.job.next, %previous.job.done ] %previous.job.more = icmp ult i32 %previous.job, %previous.jobs br i1 %previous.job.more, label %previous.job.step, label %exit
previous.job.step: %previous.m.group.short = icmp ult i32 %previous.m.tiles, RECIPE_CONTRACTION_SWIZZLE_M %previous.m.group.limit = select i1 %previous.m.group.short, i32 %previous.m.tiles, i32 RECIPE_CONTRACTION_SWIZZLE_M %previous.group.width = mul i32 %previous.m.group.limit, %previous.n.tiles %previous.group.index = udiv i32 %previous.job, %previous.group.width %previous.m.group.base = mul i32 %previous.group.index, %previous.m.group.limit %previous.m.group.remaining = sub i32 %previous.m.tiles, %previous.m.group.base %previous.m.group.tail = icmp ult i32 %previous.m.group.remaining, %previous.m.group.limit %previous.m.group.count = select i1 %previous.m.group.tail, i32 %previous.m.group.remaining, i32 %previous.m.group.limit %previous.group.local = urem i32 %previous.job, %previous.group.width %previous.m.group.local = urem i32 %previous.group.local, %previous.m.group.count %previous.m.index = add i32 %previous.m.group.base, %previous.m.group.local %previous.n.index = udiv i32 %previous.group.local, %previous.m.group.count %previous.m.base = mul i32 %previous.m.index, %previous.m.tile %previous.n.base = mul i32 %previous.n.index, %previous.n.tile
%previous.m.remaining = sub i32 %previous.m.total, %previous.m.base %previous.m.partial = icmp ult i32 %previous.m.remaining, %previous.m.tile %previous.m.count = select i1 %previous.m.partial, i32 %previous.m.remaining, i32 %previous.m.tile
%previous.n.remaining = sub i32 %in.channels, %previous.n.base %previous.n.partial = icmp ult i32 %previous.n.remaining, %previous.n.tile %previous.n.count = select i1 %previous.n.partial, i32 %previous.n.remaining, i32 %previous.n.tile
%previous.m.lanes.adjusted = add i32 %previous.m.count, RECIPE_REGISTER_M %previous.m.lanes.numerator = sub i32 %previous.m.lanes.adjusted, 1 %previous.m.lanes = udiv i32 %previous.m.lanes.numerator, RECIPE_REGISTER_M %previous.n.lanes.adjusted = add i32 %previous.n.count, RECIPE_REGISTER_N %previous.n.lanes.numerator = sub i32 %previous.n.lanes.adjusted, 1 %previous.n.lanes = udiv i32 %previous.n.lanes.numerator, RECIPE_REGISTER_N
%previous.lanes = call i32 @contraction_output_lanes(i32 %previous.m.lanes, i32 %previous.n.lanes, i32 %block) %previous.lane.active = icmp ult i32 %lid, %previous.lanes %previous.lane.n.raw = udiv i32 %lid, %previous.m.lanes %previous.lane.m.raw = urem i32 %lid, %previous.m.lanes %previous.lane.n = select i1 %previous.lane.active, i32 %previous.lane.n.raw, i32 0 %previous.lane.m = select i1 %previous.lane.active, i32 %previous.lane.m.raw, i32 0
%previous.output.m.base = mul i32 %previous.lane.m, RECIPE_REGISTER_M %previous.output.n.base = mul i32 %previous.lane.n, RECIPE_REGISTER_N br label %previous.sum.init.loop previous.sum.init.loop:
%previous.sum.init = phi i32 [ 0, %previous.job.step ], [ %previous.sum.init.next, %previous.sum.init.step ] %previous.sum.init.more = icmp ult i32 %previous.sum.init, RECIPE_REGISTER_COUNT br i1 %previous.sum.init.more, label %previous.sum.init.step, label %previous.tile.loop
previous.sum.init.step: %previous.sum.init.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %previous.sum.init store double 0.0, ptr addrspace(5) %previous.sum.init.ptr, align 8 %previous.sum.init.next = add i32 %previous.sum.init, 1 br label %previous.sum.init.loop previous.tile.loop:
%previous.r.base = phi i32 [ 0, %previous.sum.init.loop ], [ %previous.r.next, %previous.tile.done ]
%previous.r.remaining = sub i32 %previous.r.total, %previous.r.base %previous.r.partial = icmp ult i32 %previous.r.remaining, %previous.k.tile %previous.r.count = select i1 %previous.r.partial, i32 %previous.r.remaining, i32 %previous.k.tile
%previous.a.project = icmp eq i32 %span, 1 %previous.a.unit = icmp eq i32 %out.length, 1 %previous.a.contiguous = and i1 %previous.a.project, %previous.a.unit
%previous.a.fragment.remainder = urem i32 %previous.r.count, RECIPE_WMMA_K %previous.a.fragment.full = icmp eq i32 %previous.a.fragment.remainder, 0 %previous.a.vector = and i1 %previous.a.contiguous, %previous.a.fragment.full
%previous.a.width = select i1 %previous.a.vector, i32 RECIPE_WMMA_K, i32 1 %previous.a.columns = udiv i32 %previous.r.count, %previous.a.width
%previous.b.fragment.remainder = urem i32 %previous.n.count, RECIPE_WMMA_K %previous.b.fragment.full = icmp eq i32 %previous.b.fragment.remainder, 0 %previous.b.vector = and i1 %previous.a.project, %previous.b.fragment.full
%previous.b.width = select i1 %previous.b.vector, i32 RECIPE_WMMA_K, i32 1 %previous.b.columns = udiv i32 %previous.n.count, %previous.b.width
%previous.a.count = mul i32 %previous.m.count, %previous.a.columns %previous.b.count = mul i32 %previous.r.count, %previous.b.columns %previous.load.count = add i32 %previous.a.count, %previous.b.count br label %previous.load.loop previous.load.loop:
%previous.load = phi i32 [ %lid, %previous.tile.loop ], [ %previous.load.next, %previous.load.advance ] %previous.load.more = icmp ult i32 %previous.load, %previous.load.count br i1 %previous.load.more, label %previous.load.classify, label %previous.load.done
previous.load.classify: %previous.load.a = icmp ult i32 %previous.load, %previous.a.count br i1 %previous.load.a, label %previous.load.a.step, label %previous.load.b.step
previous.load.a.step: %previous.a.m = udiv i32 %previous.load, %previous.a.columns %previous.a.column = urem i32 %previous.load, %previous.a.columns %previous.a.r = mul i32 %previous.a.column, %previous.a.width %previous.a.term = add i32 %previous.r.base, %previous.a.r
%previous.a.filter = udiv i32 %previous.a.term, %span %previous.a.kernel = urem i32 %previous.a.term, %span %previous.a.global = add i32 %previous.m.base, %previous.a.m %previous.a.row = udiv i32 %previous.a.global, %in.length %previous.a.position = urem i32 %previous.a.global, %in.length
%previous.a.low = icmp uge i32 %previous.a.position, %previous.a.kernel %previous.a.position.raw = sub i32 %previous.a.position, %previous.a.kernel %previous.a.high = icmp ult i32 %previous.a.position.raw, %out.length %previous.a.valid = and i1 %previous.a.low, %previous.a.high
%previous.a.position.safe = select i1 %previous.a.valid, i32 %previous.a.position.raw, i32 0 %previous.a.row.base = mul i32 %previous.a.row, %out.elements %previous.a.filter.base = mul i32 %previous.a.filter, %out.length
%previous.a.local = add i32 %previous.a.filter.base, %previous.a.position.safe %previous.a.index = add i32 %previous.a.row.base, %previous.a.local %previous.a.tile.row = mul i32 %previous.a.r, %previous.tile.m %previous.a.tile.index = add i32 %previous.a.tile.row, %previous.a.m
br i1 %previous.a.vector, label %previous.load.a.vector, label %previous.load.a.scalar
previous.load.a.vector:
%previous.a.vector.delta = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %previous.a.index
%previous.a.vector.output = getelementptr inbounds double, ptr addrspace(1) %output, i32 %previous.a.index
%previous.a.vector.delta.value = load <RECIPE_WMMA_K x double>, ptr addrspace(1) %previous.a.vector.delta, align 8
%previous.a.vector.output.value = load <RECIPE_WMMA_K x double>, ptr addrspace(1) %previous.a.vector.output, align 8
call void @contraction_stage_delta_a_fragment(<RECIPE_WMMA_K x double> %previous.a.vector.delta.value, <RECIPE_WMMA_K x double> %previous.a.vector.output.value, i1 %relu, i32 %previous.a.r, i32 %previous.a.m, i32 %previous.tile.m)
br label %previous.load.advance
previous.load.a.scalar:
%previous.a.raw = call double @contraction_delta(ptr addrspace(1) %delta, ptr addrspace(1) %output, i32 %previous.a.index, i1 %relu)
%previous.a.value = select i1 %previous.a.valid, double %previous.a.raw, double 0.0
br label %previous.load.store
previous.load.b.step: %previous.b.local = sub i32 %previous.load, %previous.a.count %previous.b.r = udiv i32 %previous.b.local, %previous.b.columns %previous.b.column = urem i32 %previous.b.local, %previous.b.columns %previous.b.n = mul i32 %previous.b.column, %previous.b.width %previous.b.term = add i32 %previous.r.base, %previous.b.r
%previous.b.filter = udiv i32 %previous.b.term, %span %previous.b.kernel = urem i32 %previous.b.term, %span %previous.b.channel = add i32 %previous.n.base, %previous.b.n %previous.b.filter.base = mul i32 %previous.b.filter, %window
%previous.b.channel.base = mul i32 %previous.b.channel, %span %previous.b.channel.local = add i32 %previous.b.channel.base, %previous.b.kernel %previous.b.index = add i32 %previous.b.filter.base, %previous.b.channel.local
%previous.b.tile.base = mul i32 %previous.tile.m, %previous.tile.k
%previous.b.tile.local = call i32 @contraction_b_index(i32 %previous.b.r, i32 %previous.b.n, i32 %previous.tile.n, i32 %previous.tile.b.k) %previous.b.tile.index = add i32 %previous.b.tile.base, %previous.b.tile.local
br i1 %previous.b.vector, label %previous.load.b.vector, label %previous.load.b.scalar
previous.load.b.vector:
%previous.b.vector.source = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %previous.b.index
%previous.b.vector.value = load <RECIPE_WMMA_K x double>, ptr addrspace(1) %previous.b.vector.source, align 8
call void @contraction_stage_b_fragment(<RECIPE_WMMA_K x double> %previous.b.vector.value, i32 %previous.b.r, i32 %previous.b.n, i32 %previous.tile.m, i32 %previous.tile.n, i32 %previous.tile.k)
br label %previous.load.advance
previous.load.b.scalar:
%previous.b.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %previous.b.index
%previous.b.value = load double, ptr addrspace(1) %previous.b.ptr, align 8
br label %previous.load.store
previous.load.store: %previous.load.value = phi double [ %previous.a.value, %previous.load.a.scalar ], [ %previous.b.value, %previous.load.b.scalar ] %previous.load.index = phi i32 [ %previous.a.tile.index, %previous.load.a.scalar ], [ %previous.b.tile.index, %previous.load.b.scalar ]
%previous.load.ptr = getelementptr [0 x double], ptr addrspace(3) @contraction_tile, i32 0, i32 %previous.load.index store double %previous.load.value, ptr addrspace(3) %previous.load.ptr, align 8
br label %previous.load.advance
previous.load.advance:
%previous.load.next = add i32 %previous.load, %block br label %previous.load.loop previous.load.done:
%previous.load.logical.output.edge = or i1 %previous.m.partial, %previous.n.partial
%previous.load.logical.edge = or i1 %previous.load.logical.output.edge, %previous.r.partial
%previous.load.m.edge = icmp ult i32 %previous.m.count, %previous.tile.m
%previous.load.n.edge = icmp ult i32 %previous.n.count, %previous.tile.n
%previous.load.k.edge = icmp ult i32 %previous.r.count, %previous.tile.k
%previous.load.schedule.output.edge = or i1 %previous.load.m.edge, %previous.load.n.edge
%previous.load.schedule.edge = or i1 %previous.load.schedule.output.edge, %previous.load.k.edge
%previous.load.vector.edge = select i1 RECIPE_CONTRACTION_MATRIX, i1 %previous.load.schedule.edge, i1 %previous.load.logical.edge
br i1 %previous.load.vector.edge, label %previous.load.zero, label %previous.load.ready
previous.load.zero:
call void @contraction_zero_edges(i32 %previous.m.count, i32 %previous.n.count, i32 %previous.r.count, i32 %lid, i32 %block, i32 %previous.tile.m, i32 %previous.tile.n, i32 %previous.tile.k)
br label %previous.load.ready
previous.load.ready:
call void @recipe.local.barrier()
call void @contraction_accumulate(ptr addrspace(5) %sums, ptr addrspace(5) null, i1 false, i1 %previous.lane.active, i32 %previous.output.m.base, i32 %previous.output.n.base, i32 %previous.m.count, i32 %previous.n.count, i32 0, i32 1, i32 %previous.r.count, i32 %previous.tile.m, i32 %previous.tile.n, i32 %previous.tile.k) call void @recipe.local.barrier()
%previous.r.next = add i32 %previous.r.base, %previous.r.count %previous.r.more = icmp ult i32 %previous.r.next, %previous.r.total br i1 %previous.r.more, label %previous.tile.done, label %previous.store.loop previous.tile.done: br label %previous.tile.loop previous.store.loop:
%previous.store.register = phi i32 [ 0, %previous.load.ready ], [ %previous.store.register.next, %previous.store.next ] %previous.store.more = icmp ult i32 %previous.store.register, RECIPE_REGISTER_COUNT br i1 %previous.store.more, label %previous.store.test, label %previous.job.done
previous.store.test: %previous.store.output.m.raw = call i32 @contraction_output_m(i32 %lid, i32 %previous.store.register, i32 %previous.m.lanes) %previous.store.output.n.raw = call i32 @contraction_output_n(i32 %lid, i32 %previous.store.register, i32 %previous.m.lanes) %previous.store.register.valid = call i1 @contraction_output_register_valid(i32 %previous.store.register)
%previous.store.output.m.valid = icmp ult i32 %previous.store.output.m.raw, %previous.m.count %previous.store.output.n.valid = icmp ult i32 %previous.store.output.n.raw, %previous.n.count %previous.store.output.valid = and i1 %previous.store.output.m.valid, %previous.store.output.n.valid %previous.lane.output.active = and i1 %previous.lane.active, %previous.store.output.valid %previous.store.active = and i1 %previous.lane.output.active, %previous.store.register.valid br i1 %previous.store.active, label %previous.store, label %previous.store.next
previous.store: %previous.store.m.global = add i32 %previous.m.base, %previous.store.output.m.raw %previous.store.channel = add i32 %previous.n.base, %previous.store.output.n.raw %previous.store.row = udiv i32 %previous.store.m.global, %in.length %previous.store.position = urem i32 %previous.store.m.global, %in.length
%previous.store.row.base = mul i32 %previous.store.row, %in.elements %previous.store.channel.base = mul i32 %previous.store.channel, %in.length %previous.store.local = add i32 %previous.store.channel.base, %previous.store.position %previous.store.index = add i32 %previous.store.row.base, %previous.store.local %previous.store.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %previous.store.index
%previous.store.old = load double, ptr addrspace(1) %previous.store.ptr, align 8 %previous.store.sum.ptr = getelementptr [RECIPE_REGISTER_COUNT x double], ptr addrspace(5) %sums, i32 0, i32 %previous.store.register %previous.store.sum = load double, ptr addrspace(5) %previous.store.sum.ptr, align 8 %previous.store.value = call double @recipe.add(double %previous.store.old, double %previous.store.sum) store double %previous.store.value, ptr addrspace(1) %previous.store.ptr, align 8 br label %previous.store.next
previous.store.next: %previous.store.register.next = add i32 %previous.store.register, 1 br label %previous.store.loop previous.job.done: %previous.job.next = add i32 %previous.job, %groups br label %previous.job.loop exit: ret void }
define internal void @scan_reverse_body( ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output,
ptr addrspace(1) %context, ptr addrspace(1) %delta, ptr addrspace(1) %previous,
ptr addrspace(1) %gradient, i1 %write.input, i32 %rows, i32 %in.channels,
i32 %length, i32 %out.channels, i32 %gates, i32 %parameters, i32 %offset,
i32 %gradient.tile.m, i32 %gradient.tile.n, i32 %gradient.tile.k, i32 %previous.tile.m, i32 %previous.tile.n, i32 %previous.tile.k, i32 %threads ) #1 { entry:
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
br label %row.loop reduce.entry: call void @llvm.amdgcn.s.barrier()
call void @reduce_rows(ptr addrspace(1) %context, ptr addrspace(1) %gradient, i32 %rows, i32 %parameters, i32 %row.gradient.base, i32 %offset, i32 %threads)
br label %projection.entry
projection.entry: call void @llvm.amdgcn.s.barrier() br label %projection.loop projection.loop:
%projection.gate = phi i32 [ 0, %projection.entry ], [ %projection.next, %projection.step ]
%projection.more = icmp ult i32 %projection.gate, %gates
br i1 %projection.more, label %projection.step, label %exit projection.step:
%projection.weight.offset = mul i32 %projection.gate, %gate.stride
%projection.weights = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %projection.weight.offset
%projection.delta.gate = mul i32 %projection.gate, %batch
%projection.delta.offset = add i32 %delta.base, %projection.delta.gate
%projection.delta = getelementptr inbounds double, ptr addrspace(1) %context, i32 %projection.delta.offset
%projection.gradient.offset = add i32 %offset, %projection.weight.offset
call void @contraction_reverse_body( ptr addrspace(1) %input, ptr addrspace(1) %projection.weights, ptr addrspace(1) %output,
ptr addrspace(1) %projection.delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.input, i1 false, i1 false,
i32 %rows, i32 %in.channels, i32 %length, i32 %out.channels, i32 %length, i32 0,
i32 %projection.gradient.offset, i32 %gradient.tile.m, i32 %gradient.tile.n, i32 %gradient.tile.k,
i32 %previous.tile.m, i32 %previous.tile.n, i32 %previous.tile.k, i32 %threads ) call void @llvm.amdgcn.s.barrier() %projection.next = add i32 %projection.gate, 1 br label %projection.loop
invalid: call void @llvm.trap() br label %exit exit: ret void } attributes #0 = { nounwind "amdgpu-flat-work-group-size"="1,1024" } attributes #1 = { alwaysinline nounwind } attributes #3 = { noinline nounwind }
