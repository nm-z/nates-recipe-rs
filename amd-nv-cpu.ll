target triple = "amdgcn-amd-amdhsa" declare i32 @llvm.amdgcn.workitem.id.x()
declare void @llvm.amdgcn.s.barrier() declare double @llvm.sqrt.f64(double) declare double @llvm.fabs.f64(double)
declare double @__ocml_exp_f64(double) declare double @__ocml_tanh_f64(double) declare double @__ocml_cos_f64(double)
declare double @__ocml_sin_f64(double) declare double @__ocml_log_f64(double) declare i64 @__ockl_steadyctr_u64()
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
i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry:
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
%w = load double, ptr addrspace(1) %weight.ptr, align 8 %product = fmul double %x, %w
%candidate = fadd double %partial, %product %partial.next = select i1 %active, double %candidate, double %partial
%i.next = add i32 %i, 1 br label %compute.loop compute.done:
%tile.sum = phi double [ %partial, %compute.loop ] call void @recipe.local.barrier()
%term.next = add i32 %term.base, %k.count %term.more = icmp ult i32 %term.next, %terms
br i1 %term.more, label %tile.done, label %store.test tile.done: br label %tile.loop store.test:
br i1 %active, label %store, label %position.done store:
%output.row.base = mul i32 %row, %out.elements %output.channel.base = mul i32 %channel.raw, %out.length
%output.local = add i32 %output.channel.base, %position %output.index = add i32 %output.row.base, %output.local
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %output.index
store double %tile.sum, ptr addrspace(1) %output.ptr, align 8 br label %position.done position.done:
%m.next = add i32 %m, 1 br label %position.loop channel.done:
%n.next = add i32 %n.offset, %block %n.more = icmp ult i32 %n.next, %n.count
br i1 %n.more, label %channel.loop, label %job.done job.done:
%job.next = add i32 %job, %groups br label %job.loop exit: ret void }
define internal double @scalar_operand( double %first, double %second, ptr addrspace(1) %context,
i32 %operand, i32 %p, i32 %elements ) #1 { entry: %register = icmp sge i32 %operand, 0
%safe = select i1 %register, i32 %operand, i32 0 %base = mul i32 %safe, %elements %index = add i32 %base, %p
%ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %index
%value = load double, ptr addrspace(1) %ptr, align 8 %is.first = icmp eq i32 %operand, -1
%not.first = select i1 %register, double %value, double %second
%result = select i1 %is.first, double %first, double %not.first ret double %result }
define internal void @scalar_forward_body( ptr addrspace(1) %first, ptr addrspace(1) %second, ptr addrspace(1) %output,
ptr addrspace(1) %context, ptr addrspace(1) %program, ptr addrspace(1) %weights, i32 %p, i32 %elements,
i32 %instructions ) #1 { entry: %first.ptr = getelementptr inbounds double, ptr addrspace(1) %first, i32 %p
%second.ptr = getelementptr inbounds double, ptr addrspace(1) %second, i32 %p
%first.value = load double, ptr addrspace(1) %first.ptr, align 8
%second.value = load double, ptr addrspace(1) %second.ptr, align 8 br label %loop loop:
%i = phi i32 [ 0, %entry ], [ %next, %operation.done ] %more = icmp ult i32 %i, %instructions
br i1 %more, label %step, label %done step: %instruction = mul i32 %i, 3 %left.index = add i32 %instruction, 1
%right.index = add i32 %instruction, 2
%opcode.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %instruction
%left.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %left.index
%right.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %right.index
%opcode.double = load double, ptr addrspace(1) %opcode.ptr, align 8
%left.double = load double, ptr addrspace(1) %left.ptr, align 8
%right.double = load double, ptr addrspace(1) %right.ptr, align 8 %opcode = fptosi double %opcode.double to i32
%left = fptosi double %left.double to i32 %right = fptosi double %right.double to i32
%left.value = call double @scalar_operand( double %first.value, double %second.value, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements )
%right.value = call double @scalar_operand( double %first.value, double %second.value, ptr addrspace(1) %context,
i32 %right, i32 %p, i32 %elements ) switch i32 %opcode, label %invalid [ i32 0, label %add i32 1, label %constant
i32 2, label %parameter i32 3, label %subtract i32 4, label %multiply i32 5, label %divide i32 6, label %absolute
i32 7, label %exponential i32 8, label %logarithm i32 9, label %square.root i32 10, label %sine i32 11, label %cosine
i32 12, label %hyperbolic i32 13, label %greater ] add: %add.result = fadd double %left.value, %right.value
br label %operation.done constant: br label %operation.done parameter:
%parameter.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %left
%parameter.result = load double, ptr addrspace(1) %parameter.ptr, align 8 br label %operation.done subtract:
%subtract.result = fsub double %left.value, %right.value br label %operation.done multiply:
%multiply.result = fmul double %left.value, %right.value br label %operation.done divide:
%divide.result = fdiv double %left.value, %right.value br label %operation.done absolute:
%absolute.result = call double @llvm.fabs.f64(double %left.value) br label %operation.done exponential:
%exponential.result = call double @__ocml_exp_f64(double %left.value) br label %operation.done logarithm:
%logarithm.result = call double @__ocml_log_f64(double %left.value) br label %operation.done square.root:
%square.root.result = call double @llvm.sqrt.f64(double %left.value) br label %operation.done sine:
%sine.result = call double @__ocml_sin_f64(double %left.value) br label %operation.done cosine:
%cosine.result = call double @__ocml_cos_f64(double %left.value) br label %operation.done hyperbolic:
%hyperbolic.result = call double @__ocml_tanh_f64(double %left.value) br label %operation.done greater:
%greater.condition = fcmp ogt double %left.value, %right.value %greater.result = uitofp i1 %greater.condition to double
br label %operation.done operation.done: %result = phi double [ %add.result, %add ], [ %left.double, %constant ],
[ %parameter.result, %parameter ], [ %subtract.result, %subtract ],
[ %multiply.result, %multiply ], [ %divide.result, %divide ],
[ %absolute.result, %absolute ], [ %exponential.result, %exponential ],
[ %logarithm.result, %logarithm ], [ %square.root.result, %square.root ],
[ %sine.result, %sine ], [ %cosine.result, %cosine ], [ %hyperbolic.result, %hyperbolic ], [ %greater.result, %greater ]
%result.base = mul i32 %i, %elements %result.index = add i32 %result.base, %p
%result.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %result.index
store double %result, ptr addrspace(1) %result.ptr, align 8 %next = add nuw i32 %i, 1 br label %loop done:
%last = sub i32 %instructions, 1 %last.base = mul i32 %last, %elements %last.index = add i32 %last.base, %p
%last.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %last.index
%value = load double, ptr addrspace(1) %last.ptr, align 8
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %value, ptr addrspace(1) %output.ptr, align 8 ret void invalid: call void @llvm.trap() ret void }
define internal void @scalar_add_adjoint( ptr addrspace(1) %first, ptr addrspace(1) %second, ptr addrspace(1) %context,
i32 %operand, i32 %p, i32 %elements, i32 %instructions, double %value, i1 %write.first, i1 %write.second ) #1 { entry:
%is.first = icmp eq i32 %operand, -1 %is.second = icmp eq i32 %operand, -2 %first.valid = and i1 %is.first, %write.first
br i1 %first.valid, label %first.add, label %second.test first.add:
%first.ptr = getelementptr inbounds double, ptr addrspace(1) %first, i32 %p
%first.old = load double, ptr addrspace(1) %first.ptr, align 8 %first.next = fadd double %first.old, %value
store double %first.next, ptr addrspace(1) %first.ptr, align 8 ret void second.test:
%second.valid = and i1 %is.second, %write.second br i1 %second.valid, label %second.add, label %register.test
second.add: %second.ptr = getelementptr inbounds double, ptr addrspace(1) %second, i32 %p
%second.old = load double, ptr addrspace(1) %second.ptr, align 8 %second.next = fadd double %second.old, %value
store double %second.next, ptr addrspace(1) %second.ptr, align 8 ret void register.test:
%is.register = icmp sge i32 %operand, 0 br i1 %is.register, label %register.add, label %done register.add:
%adjoint.plane = mul i32 %instructions, %elements %register.base = mul i32 %operand, %elements
%register.local = add i32 %register.base, %p %register.index = add i32 %adjoint.plane, %register.local
%register.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %register.index
%register.old = load double, ptr addrspace(1) %register.ptr, align 8 %register.next = fadd double %register.old, %value
store double %register.next, ptr addrspace(1) %register.ptr, align 8 ret void done: ret void }
define internal void @scalar_reverse_body( ptr addrspace(1) %first.values, ptr addrspace(1) %second.values,
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint,
ptr addrspace(1) %context, ptr addrspace(1) %program, ptr addrspace(1) %delta,
ptr addrspace(1) %gradient, i32 %weight.offset, i32 %elements,
i32 %instructions, i1 %write.first, i1 %write.second, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x() %adjoint.plane = mul i32 %instructions, %elements br label %element.loop
element.loop: %p = phi i32 [ %tid, %entry ], [ %p.next, %element.done ] %p.more = icmp ult i32 %p, %elements
br i1 %p.more, label %clear.loop, label %reduce.entry clear.loop:
%clear.i = phi i32 [ 0, %element.loop ], [ %clear.next, %clear.step ] %clear.more = icmp ult i32 %clear.i, %instructions
br i1 %clear.more, label %clear.step, label %seed clear.step: %clear.base = mul i32 %clear.i, %elements
%clear.local = add i32 %clear.base, %p %clear.index = add i32 %adjoint.plane, %clear.local
%clear.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %clear.index
store double 0.0, ptr addrspace(1) %clear.ptr, align 8 %clear.next = add nuw i32 %clear.i, 1 br label %clear.loop seed:
%last = sub i32 %instructions, 1 %last.base = mul i32 %last, %elements %last.local = add i32 %last.base, %p
%last.index = add i32 %adjoint.plane, %last.local
%last.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %last.index
%delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %p
%delta.value = load double, ptr addrspace(1) %delta.ptr, align 8
store double %delta.value, ptr addrspace(1) %last.ptr, align 8 br label %reverse.loop reverse.loop:
%i = phi i32 [ %instructions, %seed ], [ %i.previous, %reverse.done ] %more = icmp ne i32 %i, 0
br i1 %more, label %reverse.step, label %element.done reverse.step: %i.previous = sub i32 %i, 1
%instruction = mul i32 %i.previous, 3 %left.index = add i32 %instruction, 1 %right.index = add i32 %instruction, 2
%opcode.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %instruction
%left.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %left.index
%right.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %right.index
%opcode.double = load double, ptr addrspace(1) %opcode.ptr, align 8
%left.double = load double, ptr addrspace(1) %left.ptr, align 8
%right.double = load double, ptr addrspace(1) %right.ptr, align 8 %opcode = fptosi double %opcode.double to i32
%left = fptosi double %left.double to i32 %right = fptosi double %right.double to i32
%first.value.ptr = getelementptr inbounds double, ptr addrspace(1) %first.values, i32 %p
%second.value.ptr = getelementptr inbounds double, ptr addrspace(1) %second.values, i32 %p
%first.value = load double, ptr addrspace(1) %first.value.ptr, align 8
%second.value = load double, ptr addrspace(1) %second.value.ptr, align 8
%left.value = call double @scalar_operand( double %first.value, double %second.value, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements )
%right.value = call double @scalar_operand( double %first.value, double %second.value, ptr addrspace(1) %context,
i32 %right, i32 %p, i32 %elements ) %adjoint.base = mul i32 %i.previous, %elements
%adjoint.local = add i32 %adjoint.base, %p %adjoint.index = add i32 %adjoint.plane, %adjoint.local
%adjoint.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %adjoint.index
%adjoint = load double, ptr addrspace(1) %adjoint.ptr, align 8
%result.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %adjoint.local
%result.value = load double, ptr addrspace(1) %result.ptr, align 8 switch i32 %opcode, label %invalid [
i32 0, label %add.reverse i32 1, label %reverse.done i32 2, label %reverse.done i32 3, label %subtract.reverse
i32 4, label %multiply.reverse i32 5, label %divide.reverse i32 6, label %absolute.reverse
i32 7, label %exponential.reverse i32 8, label %logarithm.reverse i32 9, label %square.root.reverse
i32 10, label %sine.reverse i32 11, label %cosine.reverse i32 12, label %hyperbolic.reverse i32 13, label %reverse.done
] add.reverse: call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements, i32 %instructions, double %adjoint, i1 %write.first, i1 %write.second )
call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %right, i32 %p, i32 %elements, i32 %instructions, double %adjoint, i1 %write.first, i1 %write.second )
br label %reverse.done subtract.reverse: %subtract.right = fneg double %adjoint call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements, i32 %instructions, double %adjoint, i1 %write.first, i1 %write.second )
call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %right, i32 %p, i32 %elements, i32 %instructions, double %subtract.right, i1 %write.first, i1 %write.second )
br label %reverse.done multiply.reverse: %left.contribution = fmul double %adjoint, %right.value
%right.contribution = fmul double %adjoint, %left.value call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements, i32 %instructions, double %left.contribution, i1 %write.first, i1 %write.second )
call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %right, i32 %p, i32 %elements, i32 %instructions, double %right.contribution, i1 %write.first, i1 %write.second )
br label %reverse.done divide.reverse: %divide.left = fdiv double %adjoint, %right.value
%divide.square = fmul double %right.value, %right.value %divide.numerator = fmul double %adjoint, %left.value
%divide.right.raw = fdiv double %divide.numerator, %divide.square %divide.right = fneg double %divide.right.raw
br label %unary.pair absolute.reverse: %absolute.negative = fcmp olt double %left.value, 0.0
%absolute.positive = fcmp ogt double %left.value, 0.0
%absolute.upper = select i1 %absolute.positive, double %adjoint, double 0.0 %absolute.negated = fneg double %adjoint
%absolute.left = select i1 %absolute.negative, double %absolute.negated, double %absolute.upper br label %unary.single
exponential.reverse: %exponential.left = fmul double %adjoint, %result.value br label %unary.single logarithm.reverse:
%logarithm.left = fdiv double %adjoint, %left.value br label %unary.single square.root.reverse:
%square.root.denominator = fadd double %result.value, %result.value
%square.root.left = fdiv double %adjoint, %square.root.denominator br label %unary.single sine.reverse:
%sine.cosine = call double @__ocml_cos_f64(double %left.value) %sine.left = fmul double %adjoint, %sine.cosine
br label %unary.single cosine.reverse: %cosine.sine = call double @__ocml_sin_f64(double %left.value)
%cosine.raw = fmul double %adjoint, %cosine.sine %cosine.left = fneg double %cosine.raw br label %unary.single
hyperbolic.reverse: %hyperbolic.square = fmul double %result.value, %result.value
%hyperbolic.base = fsub double 1.0, %hyperbolic.square %hyperbolic.left = fmul double %adjoint, %hyperbolic.base
br label %unary.single unary.single: %unary.left = phi double [ %absolute.left, %absolute.reverse ],
[ %exponential.left, %exponential.reverse ], [ %logarithm.left, %logarithm.reverse ],
[ %square.root.left, %square.root.reverse ], [ %sine.left, %sine.reverse ],
[ %cosine.left, %cosine.reverse ], [ %hyperbolic.left, %hyperbolic.reverse ] call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements, i32 %instructions, double %unary.left, i1 %write.first, i1 %write.second )
br label %reverse.done unary.pair: call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements, i32 %instructions, double %divide.left, i1 %write.first, i1 %write.second )
call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %right, i32 %p, i32 %elements, i32 %instructions, double %divide.right, i1 %write.first, i1 %write.second )
br label %reverse.done reverse.done: br label %reverse.loop element.done: %p.next = add i32 %p, %threads
br label %element.loop reduce.entry: call void @llvm.amdgcn.s.barrier() %leader = icmp eq i32 %tid, 0
br i1 %leader, label %parameter.loop, label %exit parameter.loop:
%parameter.i = phi i32 [ 0, %reduce.entry ], [ %parameter.next, %parameter.done ]
%parameter.more = icmp ult i32 %parameter.i, %instructions br i1 %parameter.more, label %parameter.load, label %exit
parameter.load: %parameter.instruction = mul i32 %parameter.i, 3
%parameter.operand.index = add i32 %parameter.instruction, 1
%parameter.opcode.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %parameter.instruction
%parameter.operand.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %parameter.operand.index
%parameter.opcode.double = load double, ptr addrspace(1) %parameter.opcode.ptr, align 8
%parameter.operand.double = load double, ptr addrspace(1) %parameter.operand.ptr, align 8
%parameter.opcode = fptosi double %parameter.opcode.double to i32 %parameter.is = icmp eq i32 %parameter.opcode, 2
br i1 %parameter.is, label %parameter.sum.loop, label %parameter.done parameter.sum.loop:
%parameter.p = phi i32 [ 0, %parameter.load ], [ %parameter.p.next, %parameter.sum.step ]
%parameter.sum = phi double [ 0.0, %parameter.load ], [ %parameter.sum.next, %parameter.sum.step ]
%parameter.p.more = icmp ult i32 %parameter.p, %elements
br i1 %parameter.p.more, label %parameter.sum.step, label %parameter.store parameter.sum.step:
%parameter.base = mul i32 %parameter.i, %elements %parameter.local = add i32 %parameter.base, %parameter.p
%parameter.index = add i32 %adjoint.plane, %parameter.local
%parameter.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %parameter.index
%parameter.value = load double, ptr addrspace(1) %parameter.ptr, align 8
%parameter.sum.next = fadd double %parameter.sum, %parameter.value %parameter.p.next = add nuw i32 %parameter.p, 1
br label %parameter.sum.loop parameter.store: %parameter.operand = fptosi double %parameter.operand.double to i32
%parameter.gradient.index = add i32 %weight.offset, %parameter.operand
%parameter.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %parameter.gradient.index
%parameter.old = load double, ptr addrspace(1) %parameter.gradient.ptr, align 8
%parameter.updated = fadd double %parameter.old, %parameter.sum
store double %parameter.updated, ptr addrspace(1) %parameter.gradient.ptr, align 8 br label %parameter.done
parameter.done: %parameter.next = add nuw i32 %parameter.i, 1 br label %parameter.loop invalid: call void @llvm.trap()
br label %exit exit: ret void }
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
%value = load double, ptr addrspace(1) %input.ptr, align 8 %greater = fcmp ogt double %value, %maximum
%maximum.next = select i1 %greater, double %value, double %maximum
%maximum.index.next = select i1 %greater, i32 %index, i32 %maximum.index %next = add i32 %i, 1 br label %loop done:
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
%context.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %p
%maximum.index.double = uitofp i32 %maximum.index to double store double %maximum, ptr addrspace(1) %output.ptr, align 8
store double %maximum.index.double, ptr addrspace(1) %context.ptr, align 8 ret void }
declare double @llvm.floor.f64(double)
define internal i32 @embedding_index(double %value, i32 %vocabulary) #1 { entry:
%ordered = fcmp ord double %value, %value br i1 %ordered, label %convert, label %invalid convert:
%floored = call double @llvm.floor.f64(double %value)
%max.index = sub i32 %vocabulary, 1 %max.double = uitofp i32 %max.index to double
%low = fcmp olt double %floored, 0.0 %clamped.low = select i1 %low, double 0.0, double %floored
%high = fcmp ogt double %clamped.low, %max.double
%clamped = select i1 %high, double %max.double, double %clamped.low
%index = fptoui double %clamped to i32 ret i32 %index
invalid: ret i32 0 } define internal void @embedding_forward_body(
ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %table,
ptr addrspace(1) nocapture writeonly %output, i32 %p, i32 %from, i32 %to, i32 %vocabulary ) #1 { entry:
%dimensions = udiv i32 %to, %from %row = udiv i32 %p, %to %local = urem i32 %p, %to %component = udiv i32 %local, %from
%slot = urem i32 %local, %from %row.base = mul i32 %row, %from %input.index = add i32 %row.base, %slot
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
%value = load double, ptr addrspace(1) %input.ptr, align 8
%index = call i32 @embedding_index(double %value, i32 %vocabulary) %valid = icmp ult i32 %index, %vocabulary
br i1 %valid, label %lookup, label %invalid lookup: %table.base = mul i32 %index, %dimensions
%table.index = add i32 %table.base, %component
%table.ptr = getelementptr inbounds double, ptr addrspace(1) %table, i32 %table.index
%embedded = load double, ptr addrspace(1) %table.ptr, align 8 br label %store invalid: br label %store store:
%result = phi double [ %embedded, %lookup ], [ 0x7FF8000000000000, %invalid ]
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %result, ptr addrspace(1) %output.ptr, align 8 ret void } define internal void @embedding_reverse_body(
ptr addrspace(1) %input, ptr addrspace(1) %delta, ptr addrspace(1) %gradient,
i32 %rows, i32 %tokens, i32 %dimensions, i32 %vocabulary, i32 %offset, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x() %parameters = mul i32 %dimensions, %vocabulary br label %parameter.loop
parameter.loop: %p = phi i32 [ %tid, %entry ], [ %next, %store ] %more = icmp ult i32 %p, %parameters
br i1 %more, label %row.loop, label %exit row.loop:
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
%matched = icmp eq i32 %index, %expected %component = urem i32 %p, %dimensions %output.row.base = mul i32 %row, %tokens
%output.channel.base = mul i32 %component, %tokens %output.local = add i32 %output.channel.base, %token
%output.index = add i32 %output.row.base, %output.local
%delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %output.index
%delta.value = load double, ptr addrspace(1) %delta.ptr, align 8
%contribution = select i1 %matched, double %delta.value, double 0.0 %sum.next = fadd double %token.sum, %contribution
%token.next = add nuw i32 %token, 1 br label %token.loop token.loop.done:
%row.sum = phi double [ %token.sum, %token.loop ] %row.next = add nuw i32 %row, 1 br label %row.loop store:
%gradient.index = add i32 %offset, %p
%gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.index
store double %sum, ptr addrspace(1) %gradient.ptr, align 8 %next = add i32 %p, %threads br label %parameter.loop exit:
ret void } define internal double @sigmoid(double %x) #1 { entry: %negative = fneg double %x
%exponential = call double @__ocml_exp_f64(double %negative) %denominator = fadd double 1.0, %exponential
%value = fdiv double 1.0, %denominator ret double %value }
define internal double @attention_score( ptr addrspace(1) nocapture readonly %context, i32 %plane, i32 %row, i32 %head,
i32 %query, i32 %key, i32 %from, i32 %length, i32 %head_width, double %scale ) #1 { entry:
%row.base = mul i32 %row, %from %head.start = mul i32 %head, %head_width br label %channel.loop channel.loop:
%offset = phi i32 [ 0, %entry ], [ %offset.next, %channel.step ]
%sum = phi double [ 0.0, %entry ], [ %sum.next, %channel.step ] %more = icmp ult i32 %offset, %head_width
br i1 %more, label %channel.step, label %done channel.step: %channel = add i32 %head.start, %offset
%channel.base = mul i32 %channel, %length %query.local = add i32 %channel.base, %query
%key.local = add i32 %channel.base, %key %query.index = add i32 %row.base, %query.local
%key.row.index = add i32 %row.base, %key.local %key.index = add i32 %plane, %key.row.index
%query.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %query.index
%key.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %key.index
%query.value = load double, ptr addrspace(1) %query.ptr, align 8
%key.value = load double, ptr addrspace(1) %key.ptr, align 8 %product = fmul double %query.value, %key.value
%sum.next = fadd double %sum, %product %offset.next = add i32 %offset, 1 br label %channel.loop done:
%score = fdiv double %sum, %scale ret double %score } define internal void @attention_forward_body(
ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %weights,
ptr addrspace(1) nocapture writeonly %output, ptr addrspace(1) %context,
i32 %rows, i32 %from, i32 %heads, i32 %channels, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%length = udiv i32 %from, %channels %head_width = udiv i32 %channels, %heads
%head_width.double = uitofp i32 %head_width to double %scale = call double @llvm.sqrt.f64(double %head_width.double)
%plane = mul i32 %rows, %from %matrix = mul i32 %channels, %channels
%key.weights = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %matrix
%value.weight.offset = mul i32 %matrix, 2
%value.weights = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %value.weight.offset
%key.context = getelementptr inbounds double, ptr addrspace(1) %context, i32 %plane
%value.context.offset = mul i32 %plane, 2
%value.context = getelementptr inbounds double, ptr addrspace(1) %context, i32 %value.context.offset
call void @contraction_forward_body( ptr addrspace(1) %input, ptr addrspace(1) %weights,
ptr addrspace(1) %context, i32 %rows, i32 %channels, i32 %length, i32 %channels, i32 %length, i32 0,
i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )
call void @contraction_forward_body( ptr addrspace(1) %input, ptr addrspace(1) %key.weights,
ptr addrspace(1) %key.context, i32 %rows, i32 %channels, i32 %length, i32 %channels, i32 %length,
i32 0, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )
call void @contraction_forward_body( ptr addrspace(1) %input, ptr addrspace(1) %value.weights,
ptr addrspace(1) %value.context, i32 %rows, i32 %channels, i32 %length, i32 %channels, i32 %length,
i32 0, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )
%bias.base = mul i32 %matrix, 3
br label %attn.bias.loop attn.bias.loop:
%bp = phi i32 [ %tid, %entry ], [ %bp.next, %attn.bias.step ]
%bp.more = icmp ult i32 %bp, %plane
br i1 %bp.more, label %attn.bias.step, label %projection.done attn.bias.step:
%bp.local = urem i32 %bp, %from %bp.channel = udiv i32 %bp.local, %length
%qb.offset = add i32 %bias.base, %bp.channel
%qb.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %qb.offset
%qb.val = load double, ptr addrspace(1) %qb.ptr, align 8
%q.bias.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %bp
%q.bias.old = load double, ptr addrspace(1) %q.bias.ptr, align 8
%q.biased = fadd double %q.bias.old, %qb.val
store double %q.biased, ptr addrspace(1) %q.bias.ptr, align 8
%kb.base = add i32 %bias.base, %channels %kb.offset = add i32 %kb.base, %bp.channel
%kb.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %kb.offset
%kb.val = load double, ptr addrspace(1) %kb.ptr, align 8
%k.bias.idx = add i32 %plane, %bp
%k.bias.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %k.bias.idx
%k.bias.old = load double, ptr addrspace(1) %k.bias.ptr, align 8
%k.biased = fadd double %k.bias.old, %kb.val
store double %k.biased, ptr addrspace(1) %k.bias.ptr, align 8
%vb.base = add i32 %kb.base, %channels %vb.offset = add i32 %vb.base, %bp.channel
%vb.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %vb.offset
%vb.val = load double, ptr addrspace(1) %vb.ptr, align 8
%v.bias.idx = add i32 %value.context.offset, %bp
%v.bias.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %v.bias.idx
%v.bias.old = load double, ptr addrspace(1) %v.bias.ptr, align 8
%v.biased = fadd double %v.bias.old, %vb.val
store double %v.biased, ptr addrspace(1) %v.bias.ptr, align 8
%bp.next = add i32 %bp, %threads br label %attn.bias.loop
projection.done: call void @llvm.amdgcn.s.barrier() br label %output.loop output.loop:
%p = phi i32 [ %tid, %projection.done ], [ %p.next, %output.store ] %output.more = icmp ult i32 %p, %plane
br i1 %output.more, label %output.step, label %exit output.step: %output.row = udiv i32 %p, %from
%output.local = urem i32 %p, %from %output.channel.index = udiv i32 %output.local, %length
%query = urem i32 %output.local, %length %head = udiv i32 %output.channel.index, %head_width br label %online.loop
online.loop: %key = phi i32 [ 0, %output.step ], [ %key.next, %online.step ]
%maximum = phi double [ 0xFFF0000000000000, %output.step ], [ %maximum.next, %online.step ]
%denominator = phi double [ 0.0, %output.step ], [ %denominator.next, %online.step ]
%numerator = phi double [ 0.0, %output.step ], [ %numerator.next, %online.step ]
%key.more = icmp ult i32 %key, %length br i1 %key.more, label %online.step, label %output.store online.step:
%score = call double @attention_score( ptr addrspace(1) %context, i32 %plane, i32 %output.row, i32 %head, i32 %query,
i32 %key, i32 %from, i32 %length, i32 %head_width, double %scale )
%larger = fcmp ogt double %score, %maximum %maximum.next = select i1 %larger, double %score, double %maximum
%old.centered = fsub double %maximum, %maximum.next %old.scale = call double @__ocml_exp_f64(double %old.centered)
%new.centered = fsub double %score, %maximum.next %new.scale = call double @__ocml_exp_f64(double %new.centered)
%old.denominator = fmul double %denominator, %old.scale
%denominator.next = fadd double %old.denominator, %new.scale %value.plane = mul i32 %plane, 2
%value.row = mul i32 %output.row, %from %value.channel.base = mul i32 %output.channel.index, %length
%value.local = add i32 %value.channel.base, %key %value.row.index = add i32 %value.row, %value.local
%value.index = add i32 %value.plane, %value.row.index
%value.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %value.index
%value = load double, ptr addrspace(1) %value.ptr, align 8 %weighted = fmul double %new.scale, %value
%old.numerator = fmul double %numerator, %old.scale %numerator.next = fadd double %old.numerator, %weighted
%key.next = add i32 %key, 1 br label %online.loop output.store: %attention = fdiv double %numerator, %denominator
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %attention, ptr addrspace(1) %output.ptr, align 8 %p.next = add i32 %p, %threads br label %output.loop
exit: ret void } define internal void @attention_reverse_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %context,
ptr addrspace(1) %delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient,
i1 %write.previous, i32 %rows, i32 %from, i32 %heads, i32 %channels, i32 %offset, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x() %length = udiv i32 %from, %channels
%head.width = udiv i32 %channels, %heads %head.width.double = uitofp i32 %head.width to double
%scale = call double @llvm.sqrt.f64(double %head.width.double) %plane = mul i32 %rows, %from
%value.plane = mul i32 %plane, 2
%adjoint.base = mul i32 %plane, 3 %adjoint.count = mul i32 %plane, 3 br label %clear.loop clear.loop:
%clear.p = phi i32 [ %tid, %entry ], [ %clear.next, %clear.step ] %clear.more = icmp ult i32 %clear.p, %adjoint.count
br i1 %clear.more, label %clear.step, label %row.entry clear.step: %clear.index = add i32 %adjoint.base, %clear.p
%clear.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %clear.index
store double 0.0, ptr addrspace(1) %clear.ptr, align 8 %clear.next = add i32 %clear.p, %threads br label %clear.loop
row.entry: call void @llvm.amdgcn.s.barrier() br label %row.loop row.loop:
%row = phi i32 [ %tid, %row.entry ], [ %row.next, %row.done ] %row.more = icmp ult i32 %row, %rows
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
%online.value.row.index = add i32 %online.row.base, %online.value.local
%online.value.index = add i32 %value.plane, %online.value.row.index
%online.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %online.delta.index
%online.value.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %online.value.index
%online.delta = load double, ptr addrspace(1) %online.delta.ptr, align 8
%online.value = load double, ptr addrspace(1) %online.value.ptr, align 8
%online.dp.product = fmul double %online.delta, %online.value
%online.dp.next = fadd double %online.dp, %online.dp.product
%online.channel.next = add nuw i32 %online.channel.offset, 1 br label %online.channel.loop online.statistics.update:
%online.score = call double @attention_score( ptr addrspace(1) %context, i32 %plane, i32 %row, i32 %head, i32 %query,
i32 %online.key, i32 %from, i32 %length, i32 %head.width, double %scale )
%online.larger = fcmp ogt double %online.score, %online.maximum
%online.maximum.next = select i1 %online.larger, double %online.score, double %online.maximum
%online.old.centered = fsub double %online.maximum, %online.maximum.next
%online.old.scale = call double @__ocml_exp_f64(double %online.old.centered)
%online.new.centered = fsub double %online.score, %online.maximum.next
%online.new.scale = call double @__ocml_exp_f64(double %online.new.centered)
%online.old.denominator = fmul double %online.denominator, %online.old.scale
%online.denominator.next = fadd double %online.old.denominator, %online.new.scale
%online.old.weighted = fmul double %online.weighted, %online.old.scale
%online.new.weighted = fmul double %online.dp, %online.new.scale
%online.weighted.next = fadd double %online.old.weighted, %online.new.weighted
%online.next = add nuw i32 %online.key, 1 br label %online.statistics.loop online.statistics.done:
%online.mean = fdiv double %online.weighted, %online.denominator br label %key.loop key.loop:
%key = phi i32 [ 0, %online.statistics.done ], [ %key.next, %key.channel.done ] %key.more = icmp ult i32 %key, %length
%key.head.start = mul i32 %head, %head.width %key.row.base = mul i32 %row, %from
br i1 %key.more, label %key.prepare, label %query.done key.prepare:
%key.score = call double @attention_score( ptr addrspace(1) %context, i32 %plane, i32 %row, i32 %head, i32 %query,
i32 %key, i32 %from, i32 %length, i32 %head.width, double %scale )
%key.centered = fsub double %key.score, %online.maximum
%key.exponential = call double @__ocml_exp_f64(double %key.centered)
%key.probability = fdiv double %key.exponential, %online.denominator br label %key.dp.loop key.dp.loop:
%key.dp.channel = phi i32 [ 0, %key.prepare ], [ %key.dp.next, %key.dp.step ]
%key.dp = phi double [ 0.0, %key.prepare ], [ %key.dp.sum, %key.dp.step ]
%key.dp.more = icmp ult i32 %key.dp.channel, %head.width
br i1 %key.dp.more, label %key.dp.step, label %key.channel.entry
key.dp.step: %key.channel = add i32 %key.head.start, %key.dp.channel %key.channel.base = mul i32 %key.channel, %length
%key.dp.delta.local = add i32 %key.channel.base, %query %key.value.local = add i32 %key.channel.base, %key
%key.dp.delta.index = add i32 %key.row.base, %key.dp.delta.local
%key.value.row.index = add i32 %key.row.base, %key.value.local
%key.value.index = add i32 %value.plane, %key.value.row.index
%key.dp.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %key.dp.delta.index
%key.value.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %key.value.index
%key.dp.delta = load double, ptr addrspace(1) %key.dp.delta.ptr, align 8
%key.value = load double, ptr addrspace(1) %key.value.ptr, align 8
%key.dp.product = fmul double %key.dp.delta, %key.value
%key.dp.sum = fadd double %key.dp, %key.dp.product %key.dp.next = add nuw i32 %key.dp.channel, 1 br label %key.dp.loop
key.channel.entry: %key.dp.centered = fsub double %key.dp, %online.mean
%ds = fmul double %key.probability, %key.dp.centered br label %key.channel.loop key.channel.loop:
%key.channel.offset = phi i32 [ 0, %key.channel.entry ], [ %key.channel.next, %key.channel.step ]
%key.channel.more = icmp ult i32 %key.channel.offset, %head.width
br i1 %key.channel.more, label %key.channel.step, label %key.channel.done key.channel.step:
%update.channel = add i32 %key.head.start, %key.channel.offset %update.channel.base = mul i32 %update.channel, %length
%query.local = add i32 %update.channel.base, %query %key.local = add i32 %update.channel.base, %key
%query.row.index = add i32 %key.row.base, %query.local %key.row.index = add i32 %key.row.base, %key.local
%key.plane = add i32 %plane, %key.row.index %value.index = add i32 %value.plane, %key.row.index
%query.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %query.row.index
%key.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %key.plane
%value.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %value.index
%update.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %query.row.index
%query.value = load double, ptr addrspace(1) %query.ptr, align 8
%key.value.current = load double, ptr addrspace(1) %key.ptr, align 8
%update.delta = load double, ptr addrspace(1) %update.delta.ptr, align 8 %dq.raw = fmul double %ds, %key.value.current
%dq = fdiv double %dq.raw, %scale %dk.raw = fmul double %ds, %query.value %dk = fdiv double %dk.raw, %scale
%dv = fmul double %key.probability, %update.delta %dq.base = mul i32 %plane, 3 %dk.base = mul i32 %plane, 4
%dv.base = mul i32 %plane, 5 %dq.index = add i32 %dq.base, %query.row.index %dk.index = add i32 %dk.base, %key.row.index
%dv.index = add i32 %dv.base, %key.row.index
%dq.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dq.index
%dk.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dk.index
%dv.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dv.index
%dq.old = load double, ptr addrspace(1) %dq.ptr, align 8 %dk.old = load double, ptr addrspace(1) %dk.ptr, align 8
%dv.old = load double, ptr addrspace(1) %dv.ptr, align 8 %dq.next = fadd double %dq.old, %dq
%dk.next = fadd double %dk.old, %dk %dv.next = fadd double %dv.old, %dv
store double %dq.next, ptr addrspace(1) %dq.ptr, align 8 store double %dk.next, ptr addrspace(1) %dk.ptr, align 8
store double %dv.next, ptr addrspace(1) %dv.ptr, align 8 %key.channel.next = add nuw i32 %key.channel.offset, 1
br label %key.channel.loop key.channel.done: %key.next = add nuw i32 %key, 1 br label %key.loop query.done:
%query.next = add nuw i32 %query, 1 br label %query.loop head.done: %head.next = add nuw i32 %head, 1
br label %head.loop row.done: %row.next = add i32 %row, %threads br label %row.loop rows.done:
call void @llvm.amdgcn.s.barrier() %matrix = mul i32 %channels, %channels
%key.weights = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %matrix
%value.weight.offset = mul i32 %matrix, 2
%value.weights = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %value.weight.offset
%query.adjoint.offset = mul i32 %plane, 3
%query.adjoint = getelementptr inbounds double, ptr addrspace(1) %context, i32 %query.adjoint.offset
%key.adjoint.offset = mul i32 %plane, 4
%key.adjoint = getelementptr inbounds double, ptr addrspace(1) %context, i32 %key.adjoint.offset
%value.adjoint.offset = mul i32 %plane, 5
%value.adjoint = getelementptr inbounds double, ptr addrspace(1) %context, i32 %value.adjoint.offset
%key.gradient.offset = add i32 %offset, %matrix %value.gradient.offset = add i32 %offset, %value.weight.offset
call void @contraction_reverse_body( ptr addrspace(1) %input, ptr addrspace(1) %weights,
ptr addrspace(1) %query.adjoint, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.previous,
i32 %rows, i32 %channels, i32 %length, i32 %channels, i32 %length, i32 0, i32 %offset, i32 %threads )
call void @contraction_reverse_body( ptr addrspace(1) %input, ptr addrspace(1) %key.weights,
ptr addrspace(1) %key.adjoint, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.previous,
i32 %rows, i32 %channels, i32 %length, i32 %channels, i32 %length, i32 0, i32 %key.gradient.offset, i32 %threads )
call void @contraction_reverse_body( ptr addrspace(1) %input, ptr addrspace(1) %value.weights,
ptr addrspace(1) %value.adjoint, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.previous,
i32 %rows, i32 %channels, i32 %length, i32 %channels, i32 %length, i32 0, i32 %value.gradient.offset, i32 %threads )
%bg.matrix3 = mul i32 %matrix, 3
br label %attn.bg.loop attn.bg.loop:
%bg.p = phi i32 [ %tid, %rows.done ], [ %bg.next, %attn.bg.step ]
%bg.more = icmp ult i32 %bg.p, %plane
br i1 %bg.more, label %attn.bg.step, label %attn.bg.done attn.bg.step:
%bg.local = urem i32 %bg.p, %from %bg.channel = udiv i32 %bg.local, %length
%bg.q.adj.idx = add i32 %query.adjoint.offset, %bg.p
%bg.q.adj.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %bg.q.adj.idx
%bg.q.adj = load double, ptr addrspace(1) %bg.q.adj.ptr, align 8
%bg.q.grad.off = add i32 %bg.matrix3, %bg.channel %bg.q.grad.idx = add i32 %offset, %bg.q.grad.off
%bg.q.grad.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %bg.q.grad.idx
%bg.q.old = atomicrmw fadd ptr addrspace(1) %bg.q.grad.ptr, double %bg.q.adj monotonic, align 8
%bg.kb = add i32 %bg.matrix3, %channels %bg.k.adj.idx = add i32 %key.adjoint.offset, %bg.p
%bg.k.adj.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %bg.k.adj.idx
%bg.k.adj = load double, ptr addrspace(1) %bg.k.adj.ptr, align 8
%bg.k.grad.off = add i32 %bg.kb, %bg.channel %bg.k.grad.idx = add i32 %offset, %bg.k.grad.off
%bg.k.grad.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %bg.k.grad.idx
%bg.k.old = atomicrmw fadd ptr addrspace(1) %bg.k.grad.ptr, double %bg.k.adj monotonic, align 8
%bg.vb = add i32 %bg.kb, %channels %bg.v.adj.idx = add i32 %value.adjoint.offset, %bg.p
%bg.v.adj.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %bg.v.adj.idx
%bg.v.adj = load double, ptr addrspace(1) %bg.v.adj.ptr, align 8
%bg.v.grad.off = add i32 %bg.vb, %bg.channel %bg.v.grad.idx = add i32 %offset, %bg.v.grad.off
%bg.v.grad.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %bg.v.grad.idx
%bg.v.old = atomicrmw fadd ptr addrspace(1) %bg.v.grad.ptr, double %bg.v.adj monotonic, align 8
%bg.next = add i32 %bg.p, %threads br label %attn.bg.loop
attn.bg.done: ret void }
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
i32 %length, i32 0, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )
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
%reset = load double, ptr addrspace(1) %reset.ptr, align 8 %reset.state = fmul double %reset, %previous
%state.value = select i1 %reset.candidate, double %reset.state, double %previous
%state.weight.base = add i32 %gate.weight.base, %input.matrix %state.weight.row = mul i32 %state.channel, %out.channels
%state.weight.local = add i32 %state.weight.row, %hidden
%state.weight.index = add i32 %state.weight.base, %state.weight.local
%state.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %state.weight.index
%state.weight = load double, ptr addrspace(1) %state.weight.ptr, align 8
%state.product = fmul double %state.value, %state.weight %state.sum.next = fadd double %state.sum, %state.product
%state.next = add nuw i32 %state.channel, 1 br label %state.sum.loop gate.activate:
%bias.base = add i32 %gate.weight.base, %matrix.span %bias.index = add i32 %bias.base, %hidden
%bias.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %bias.index
%bias = load double, ptr addrspace(1) %bias.ptr, align 8 %linear = fadd double %state.sum, %bias
%rnn = icmp eq i32 %gates, 1 %last.gate = sub i32 %gates, 1 %candidate = icmp eq i32 %gate, %last.gate
%use.tanh = or i1 %rnn, %candidate %tanh.value = call double @__ocml_tanh_f64(double %linear)
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
%gate0 = load double, ptr addrspace(1) %gate0.ptr, align 8 %gate1.index = add i32 %gate.batch, %output.index
%gate1.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate1.index
%gate1 = load double, ptr addrspace(1) %gate1.ptr, align 8 %gate2.base = mul i32 %gate.batch, 2
%gate2.index = add i32 %gate2.base, %output.index
%gate2.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate2.index
%gate2 = load double, ptr addrspace(1) %gate2.ptr, align 8 %gate3.base = mul i32 %gate.batch, 3
%gate3.index = add i32 %gate3.base, %output.index
%gate3.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate3.index
%gate3 = load double, ptr addrspace(1) %gate3.ptr, align 8 %output.previous.time = sub i32 %time, 1
%output.previous.safe = select i1 %previous.exists, i32 %output.previous.time, i32 0
%output.previous.local = add i32 %output.hidden.base, %output.previous.safe
%output.previous.index = add i32 %output.row.base, %output.previous.local
%output.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %output.previous.index
%output.previous.loaded = load double, ptr addrspace(1) %output.previous.ptr, align 8
%output.previous = select i1 %previous.exists, double %output.previous.loaded, double 0.0
%one.update = fsub double 1.0, %gate0 %gru.old = fmul double %gate0, %output.previous
%gru.new = fmul double %one.update, %gate2 %gru.value = fadd double %gru.old, %gru.new
%cell.base = mul i32 %gate.batch, %gates %cell.index = add i32 %cell.base, %output.index
%cell.previous.index = add i32 %cell.base, %output.previous.index
%cell.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %cell.previous.index
%cell.previous.loaded = load double, ptr addrspace(1) %cell.previous.ptr, align 8
%cell.previous = select i1 %previous.exists, double %cell.previous.loaded, double 0.0
%cell.old = fmul double %gate1, %cell.previous %cell.new = fmul double %gate0, %gate3
%cell = fadd double %cell.old, %cell.new
%cell.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %cell.index
store double %cell, ptr addrspace(1) %cell.ptr, align 8 %cell.tanh = call double @__ocml_tanh_f64(double %cell)
%lstm.value = fmul double %gate2, %cell.tanh %is.gru = icmp eq i32 %gates, 3 %is.lstm = icmp eq i32 %gates, 4
%rnn.or.gru = select i1 %is.gru, double %gru.value, double %gate0
%output.value = select i1 %is.lstm, double %lstm.value, double %rnn.or.gru br label %output.store output.store:
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %output.index
store double %output.value, ptr addrspace(1) %output.ptr, align 8 %output.next = add nuw i32 %output.hidden, 1
br label %output.loop output.done: %time.next = add nuw i32 %time, 1 br label %time.loop time.done:
%row.next = add i32 %row, %threads br label %row.loop exit: ret void } define internal void @contraction_reverse_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %delta,
ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.input,
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
br i1 %job.more, label %gradient.job.step, label %previous.loop gradient.job.step:
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
%delta.value = load double, ptr addrspace(3) %tile.ptr, align 8 %product = fmul double %input.value, %delta.value
%candidate = fadd double %sum, %product %sum.next = select i1 %active, double %candidate, double %sum
br label %gradient.tile.done gradient.tile.done: call void @recipe.local.barrier()
br label %gradient.item.loop gradient.store: br i1 %active, label %gradient.write, label %gradient.job.done
gradient.write: %p.0 = mul i32 %filter, %window %p = add i32 %p.0, %gradient.term %gradient.index = add i32 %offset, %p
%gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.index
store double %sum, ptr addrspace(1) %gradient.ptr, align 8 br label %gradient.job.done gradient.job.done:
%job.next = add i32 %job, %groups br label %gradient.job.loop
previous.loop: %previous.p = phi i32 [ %tid, %gradient.job.loop ], [ %previous.next, %previous.done ]
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
%term.product = fmul double %weight.value, %delta.value.1
%contribution = select i1 %valid, double %term.product, double 0.0
%previous.sum.next = fadd double %previous.sum, %contribution %term.next = add nuw i32 %term, 1
br label %previous.sum.loop previous.store: br i1 %write.input, label %previous.add, label %previous.done previous.add:
%previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %previous.p
%previous.old = load double, ptr addrspace(1) %previous.ptr, align 8
%previous.value = fadd double %previous.old, %previous.sum
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
%rnn.gate = load double, ptr addrspace(1) %rnn.gate.ptr, align 8 %rnn.dh = fadd double %rnn.dy, %rnn.future
%rnn.square = fmul double %rnn.gate, %rnn.gate %rnn.derivative = fsub double 1.0, %rnn.square
%rnn.delta = fmul double %rnn.dh, %rnn.derivative %rnn.delta.index = add i32 %delta.base, %rnn.index
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
%gru.n = load double, ptr addrspace(1) %gru.n.ptr, align 8 %gru.dh = fadd double %gru.dy, %gru.future
%gru.one.z = fsub double 1.0, %gru.z %gru.z.difference = fsub double %gru.previous, %gru.n
%gru.dz.0 = fmul double %gru.dh, %gru.z.difference %gru.dz.1 = fmul double %gru.dz.0, %gru.z
%gru.dz = fmul double %gru.dz.1, %gru.one.z %gru.n.square = fmul double %gru.n, %gru.n
%gru.n.derivative = fsub double 1.0, %gru.n.square %gru.dn.0 = fmul double %gru.dh, %gru.one.z
%gru.dn = fmul double %gru.dn.0, %gru.n.derivative %gru.dz.index = add i32 %delta.base, %gru.index
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
%gru.reset.product = fmul double %gru.weight, %gru.target.delta
%gru.reset.sum.next = fadd double %gru.reset.sum, %gru.reset.product
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
%gru.dr = fmul double %gru.reset.sum, %gru.source.previous %gru.one.r = fsub double 1.0, %gru.r
%gru.dr.0 = fmul double %gru.dr, %gru.r %gru.dr.1 = fmul double %gru.dr.0, %gru.one.r
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
%dh = fadd double %dy, %dh.future %cell.tanh = call double @__ocml_tanh_f64(double %cell)
%cell.tanh.square = fmul double %cell.tanh, %cell.tanh %cell.tanh.derivative = fsub double 1.0, %cell.tanh.square
%cell.chain.0 = fmul double %dh, %o %cell.chain = fmul double %cell.chain.0, %cell.tanh.derivative
%dc = fadd double %dc.future, %cell.chain %one.o = fsub double 1.0, %o %do.0 = fmul double %dh, %cell.tanh
%do.1 = fmul double %do.0, %o %do = fmul double %do.1, %one.o %one.i = fsub double 1.0, %i %di.0 = fmul double %dc, %g
%di.1 = fmul double %di.0, %i %di = fmul double %di.1, %one.i %one.f = fsub double 1.0, %f
%df.0 = fmul double %dc, %cell.previous %df.1 = fmul double %df.0, %f %df = fmul double %df.1, %one.f
%g.square = fmul double %g, %g %one.g.square = fsub double 1.0, %g.square %dg.0 = fmul double %dc, %i
%dg = fmul double %dg.0, %one.g.square %dc.previous = fmul double %dc, %f
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
%parameter.reset.state = fmul double %parameter.reset, %state.value
%parameter.state = select i1 %gru.candidate, double %parameter.reset.state, double %state.value
%source.value = select i1 %state.weight, double %parameter.state, double 1.0
%contribution = fmul double %source.value, %gate.delta %row.gradient.index = add i32 %row.gradient.start, %p
%row.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %row.gradient.index
%row.gradient.old = load double, ptr addrspace(1) %row.gradient.ptr, align 8
%row.gradient.new = fadd double %row.gradient.old, %contribution
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
%state.product = fmul double %state.weight.value, %state.delta.value %state.candidate = icmp eq i32 %state.gate, 2
%state.gru.candidate = and i1 %gru, %state.candidate %state.reset.hidden.base = mul i32 %state.channel, %length
%state.reset.local = add i32 %state.reset.hidden.base, %time.current
%state.reset.row = add i32 %row.output.base, %state.reset.local %state.reset.raw = add i32 %batch, %state.reset.row
%state.reset.index = select i1 %state.gru.candidate, i32 %state.reset.raw, i32 0
%state.reset.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %state.reset.index
%state.reset = load double, ptr addrspace(1) %state.reset.ptr, align 8
%state.reset.product = fmul double %state.product, %state.reset
%state.contribution = select i1 %state.gru.candidate, double %state.reset.product, double %state.product
%state.sum.next = fadd double %state.sum, %state.contribution %state.term.next = add nuw i32 %state.term, 1
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
%state.direct.dh = fadd double %state.direct.dy, %state.direct.future
%state.direct.raw = fmul double %state.direct.z, %state.direct.dh
%state.direct = select i1 %gru, double %state.direct.raw, double 0.0
%state.total = fadd double %state.sum, %state.direct
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
%reduce.sum.next = fadd double %reduce.sum, %reduce.value %reduce.row.next = add nuw i32 %reduce.row, 1
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
ptr addrspace(1) %projection.delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, i1 %write.input,
i32 %rows, i32 %in.channels, i32 %length, i32 %out.channels, i32 %length, i32 0,
i32 %projection.gradient.offset, i32 %threads ) %projection.next = add i32 %projection.gate, 1 br label %projection.loop
invalid: call void @llvm.trap() br label %exit exit: ret void } define internal void @tape_forward_body(
ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %value.pointers,
ptr addrspace(1) %context.pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %arguments,
i32 %rows, i32 %nodes, i32 %threads, i32 %launch.tile.m, i32 %launch.tile.n, i32 %launch.tile.k,
ptr addrspace(1) %timings, ptr addrspace(1) %tiles ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x() call void @llvm.amdgcn.s.barrier()
%first = call i64 @__ockl_steadyctr_u64() br label %node.loop
node.loop: %node = phi i32 [ 0, %entry ], [ %node.next, %timing.done ]
%started = phi i64 [ %first, %entry ], [ %ended, %timing.done ] %node.more = icmp ult i32 %node, %nodes
br i1 %node.more, label %node.load, label %exit node.load: %base = mul i32 %node, 11 %tile.base = mul i32 %node, 3
%tile.n.index = add i32 %tile.base, 1 %tile.k.index = add i32 %tile.base, 2
%tile.m.ptr = getelementptr inbounds i32, ptr addrspace(1) %tiles, i32 %tile.base
%tile.n.ptr = getelementptr inbounds i32, ptr addrspace(1) %tiles, i32 %tile.n.index
%tile.k.ptr = getelementptr inbounds i32, ptr addrspace(1) %tiles, i32 %tile.k.index
%tile.m = load i32, ptr addrspace(1) %tile.m.ptr, align 4 %tile.n = load i32, ptr addrspace(1) %tile.n.ptr, align 4
%tile.k = load i32, ptr addrspace(1) %tile.k.ptr, align 4
%descriptor.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %base
%descriptor.second.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptor.ptr, i32 4
%descriptor.first = load <4 x i32>, ptr addrspace(1) %descriptor.ptr, align 4
%descriptor.second = load <4 x i32>, ptr addrspace(1) %descriptor.second.ptr, align 4
%op = extractelement <4 x i32> %descriptor.first, i32 0 %source.node = extractelement <4 x i32> %descriptor.first, i32 1
%second.node = extractelement <4 x i32> %descriptor.first, i32 2
%in.channels = extractelement <4 x i32> %descriptor.first, i32 3
%in.length = extractelement <4 x i32> %descriptor.second, i32 0
%out.channels = extractelement <4 x i32> %descriptor.second, i32 1
%out.length = extractelement <4 x i32> %descriptor.second, i32 2
%offset = extractelement <4 x i32> %descriptor.second, i32 3 %node.parameters.index = add i32 %base, 8
%program.offset.index = add i32 %base, 9 %program.count.index = add i32 %base, 10
%node.parameters.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %node.parameters.index
%program.offset.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %program.offset.index
%program.count.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %program.count.index
%node.parameters = load i32, ptr addrspace(1) %node.parameters.ptr, align 4
%program.offset = load i32, ptr addrspace(1) %program.offset.ptr, align 4
%program.count = load i32, ptr addrspace(1) %program.count.ptr, align 4
%program = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %program.offset
%source.external = icmp slt i32 %source.node, 0 %source.safe = select i1 %source.external, i32 0, i32 %source.node
%source.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %source.safe
%source.address = load i64, ptr addrspace(1) %source.slot, align 8
%source.values = inttoptr i64 %source.address to ptr addrspace(1)
%source = select i1 %source.external, ptr addrspace(1) %samples, ptr addrspace(1) %source.values
%second.external = icmp slt i32 %second.node, 0 %second.safe = select i1 %second.external, i32 0, i32 %second.node
%second.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %second.safe
%second.address = load i64, ptr addrspace(1) %second.slot, align 8
%second.values = inttoptr i64 %second.address to ptr addrspace(1)
%second = select i1 %second.external, ptr addrspace(1) %source, ptr addrspace(1) %second.values
%value.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %node
%value.address = load i64, ptr addrspace(1) %value.slot, align 8
%values = inttoptr i64 %value.address to ptr addrspace(1)
%context.slot = getelementptr inbounds i64, ptr addrspace(1) %context.pointers, i32 %node
%context.address = load i64, ptr addrspace(1) %context.slot, align 8
%context = inttoptr i64 %context.address to ptr addrspace(1) %argument.base = mul i32 %node, 9
%argument.second.index = add i32 %argument.base, 1 %argument.third.index = add i32 %argument.base, 2
%argument.ptr = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %argument.base
%argument.second.ptr = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %argument.second.index
%argument.third.ptr = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %argument.third.index
%argument = load double, ptr addrspace(1) %argument.ptr, align 8
%argument.second = load double, ptr addrspace(1) %argument.second.ptr, align 8
%argument.third = load double, ptr addrspace(1) %argument.third.ptr, align 8
%matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %offset
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length
%count = mul i32 %rows, %out.elements %is.attention = icmp eq i32 %op, 4 %is.scan = icmp eq i32 %op, 5
%is.contraction = icmp eq i32 %op, 0 %is.normalize = icmp eq i32 %op, 8
br i1 %is.attention, label %attention.node, label %scan.test attention.node: %heads = fptoui double %argument to i32
call void @attention_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values,
ptr addrspace(1) %context, i32 %rows, i32 %in.elements, i32 %heads, i32 %in.channels,
i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) br label %node.done
scan.test: br i1 %is.scan, label %scan.node, label %contraction.test scan.node:
%scan.gates = fptoui double %argument to i32
call void @scan_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values,
ptr addrspace(1) %context, i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels,
i32 %scan.gates, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads ) br label %node.done contraction.test:
br i1 %is.contraction, label %contraction.node, label %normalize.stats.entry contraction.node:
%contraction.kernel = fptoui double %argument to i32
call void @contraction_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix,
ptr addrspace(1) %values, i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels,
i32 %out.length, i32 %contraction.kernel, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )
br label %node.done normalize.stats.entry: br label %normalize.test normalize.test:
br i1 %is.normalize, label %normalize.stats.loop, label %element.loop
normalize.stats.loop: %stats.group = phi i32 [ %tid, %normalize.test ], [ %stats.next, %stats.store ]
%stats.mode = fptoui double %argument to i32 %stats.batch = icmp eq i32 %stats.mode, 0
%stats.layer.groups = mul i32 %rows, %out.length
%stats.groups = select i1 %stats.batch, i32 %out.channels, i32 %stats.layer.groups
%stats.more = icmp ult i32 %stats.group, %stats.groups br i1 %stats.more, label %stats.mean.loop, label %stats.done
stats.mean.loop: %stats.p = phi i32 [ 0, %normalize.stats.loop ], [ %stats.p.next, %stats.mean.step ]
%stats.sum = phi double [ 0.0, %normalize.stats.loop ], [ %stats.sum.next, %stats.mean.step ]
%stats.batch.count = mul i32 %rows, %out.length
%stats.items = select i1 %stats.batch, i32 %stats.batch.count, i32 %out.channels
%stats.p.more = icmp ult i32 %stats.p, %stats.items
br i1 %stats.p.more, label %stats.mean.step, label %stats.variance.loop stats.mean.step:
%stats.batch.row = udiv i32 %stats.p, %out.length %stats.batch.position = urem i32 %stats.p, %out.length
%stats.batch.row.base = mul i32 %stats.batch.row, %out.elements
%stats.batch.channel.base = mul i32 %stats.group, %out.length
%stats.batch.local = add i32 %stats.batch.channel.base, %stats.batch.position
%stats.batch.index = add i32 %stats.batch.row.base, %stats.batch.local
%stats.layer.row = udiv i32 %stats.group, %out.length %stats.layer.position = urem i32 %stats.group, %out.length
%stats.layer.row.base = mul i32 %stats.layer.row, %out.elements
%stats.layer.channel.base = mul i32 %stats.p, %out.length
%stats.layer.local = add i32 %stats.layer.channel.base, %stats.layer.position
%stats.layer.index = add i32 %stats.layer.row.base, %stats.layer.local
%stats.index = select i1 %stats.batch, i32 %stats.batch.index, i32 %stats.layer.index
%stats.input.ptr = getelementptr inbounds double, ptr addrspace(1) %source, i32 %stats.index
%stats.input = load double, ptr addrspace(1) %stats.input.ptr, align 8
%stats.sum.next = fadd double %stats.sum, %stats.input %stats.p.next = add nuw i32 %stats.p, 1 br label %stats.mean.loop
stats.variance.loop: %stats.variance.p = phi i32 [ 0, %stats.mean.loop ], [ %stats.variance.next, %stats.variance.step ]
%stats.variance.sum = phi double [ 0.0, %stats.mean.loop ], [ %stats.variance.sum.next, %stats.variance.step ]
%stats.items.double = uitofp i32 %stats.items to double %stats.mean = fdiv double %stats.sum, %stats.items.double
%stats.variance.more = icmp ult i32 %stats.variance.p, %stats.items
br i1 %stats.variance.more, label %stats.variance.step, label %stats.store stats.variance.step:
%stats.variance.batch.row = udiv i32 %stats.variance.p, %out.length
%stats.variance.batch.position = urem i32 %stats.variance.p, %out.length
%stats.variance.batch.row.base = mul i32 %stats.variance.batch.row, %out.elements
%stats.variance.batch.channel.base = mul i32 %stats.group, %out.length
%stats.variance.batch.local = add i32 %stats.variance.batch.channel.base, %stats.variance.batch.position
%stats.variance.batch.index = add i32 %stats.variance.batch.row.base, %stats.variance.batch.local
%stats.variance.layer.row = udiv i32 %stats.group, %out.length
%stats.variance.layer.position = urem i32 %stats.group, %out.length
%stats.variance.layer.row.base = mul i32 %stats.variance.layer.row, %out.elements
%stats.variance.layer.channel.base = mul i32 %stats.variance.p, %out.length
%stats.variance.layer.local = add i32 %stats.variance.layer.channel.base, %stats.variance.layer.position
%stats.variance.layer.index = add i32 %stats.variance.layer.row.base, %stats.variance.layer.local
%stats.variance.index = select i1 %stats.batch, i32 %stats.variance.batch.index, i32 %stats.variance.layer.index
%stats.variance.input.ptr = getelementptr inbounds double, ptr addrspace(1) %source, i32 %stats.variance.index
%stats.variance.input = load double, ptr addrspace(1) %stats.variance.input.ptr, align 8
%stats.difference = fsub double %stats.variance.input, %stats.mean
%stats.square = fmul double %stats.difference, %stats.difference
%stats.variance.sum.next = fadd double %stats.variance.sum, %stats.square
%stats.variance.next = add nuw i32 %stats.variance.p, 1 br label %stats.variance.loop stats.store:
%stats.variance = fdiv double %stats.variance.sum, %stats.items.double
%stats.adjusted = fadd double %stats.variance, %argument.second
%stats.deviation = call double @llvm.sqrt.f64(double %stats.adjusted) %stats.inverse = fdiv double 1.0, %stats.deviation
%stats.scale.index = add i32 %stats.groups, %stats.group
%stats.mean.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %stats.group
%stats.scale.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %stats.scale.index
store double %stats.mean, ptr addrspace(1) %stats.mean.ptr, align 8
store double %stats.inverse, ptr addrspace(1) %stats.scale.ptr, align 8 %stats.next = add i32 %stats.group, %threads
br label %normalize.stats.loop stats.done: call void @llvm.amdgcn.s.barrier() br label %element.loop element.loop:
%p = phi i32 [ %tid, %normalize.test ], [ %tid, %stats.done ], [ %p.next, %element.done ]
%p.more = icmp ult i32 %p, %count br i1 %p.more, label %element.step, label %node.done element.step:
switch i32 %op, label %invalid [ i32 2, label %pool i32 3, label %gather
i32 6, label %elementwise i32 7, label %route i32 8, label %normalize ] pool: %size = fptoui double %argument to i32
call void @pool_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %values, ptr addrspace(1) %context, i32 %p,
i32 %in.elements, i32 %out.elements, i32 %size, i32 %in.channels ) br label %element.done gather:
%vocabulary = fptoui double %argument to i32
call void @embedding_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values, i32 %p,
i32 %in.elements, i32 %out.elements, i32 %vocabulary ) br label %element.done elementwise:
call void @scalar_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %second, ptr addrspace(1) %values,
ptr addrspace(1) %context, ptr addrspace(1) %program, ptr addrspace(1) %matrix, i32 %p, i32 %count, i32 %program.count )
br label %element.done route: %route.row = udiv i32 %p, %out.elements %route.column = urem i32 %p, %out.elements
%route.base = mul i32 %route.column, 3 %route.stride.index = add i32 %route.base, 1
%route.field.index = add i32 %route.base, 2
%route.source.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %route.base
%route.stride.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %route.stride.index
%route.field.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %route.field.index
%route.source.double = load double, ptr addrspace(1) %route.source.ptr, align 8
%route.stride.double = load double, ptr addrspace(1) %route.stride.ptr, align 8
%route.field.double = load double, ptr addrspace(1) %route.field.ptr, align 8
%route.source = fptosi double %route.source.double to i32 %route.stride = fptoui double %route.stride.double to i32
%route.field = fptoui double %route.field.double to i32
%route.external = icmp slt i32 %route.source, 0 %route.safe = select i1 %route.external, i32 0, i32 %route.source
%route.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %route.safe
%route.address = load i64, ptr addrspace(1) %route.slot, align 8
%route.node.values = inttoptr i64 %route.address to ptr addrspace(1)
%route.source.values = select i1 %route.external, ptr addrspace(1) %samples, ptr addrspace(1) %route.node.values
%route.row.base = mul i32 %route.row, %route.stride %route.index = add i32 %route.row.base, %route.field
%route.input.ptr = getelementptr inbounds double, ptr addrspace(1) %route.source.values, i32 %route.index
%route.value = load double, ptr addrspace(1) %route.input.ptr, align 8
%route.output.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %p
store double %route.value, ptr addrspace(1) %route.output.ptr, align 8 br label %element.done
normalize: %normalize.row = udiv i32 %p, %out.elements
%normalize.local = urem i32 %p, %out.elements %normalize.channel = udiv i32 %normalize.local, %out.length
%normalize.position = urem i32 %normalize.local, %out.length %mode = fptoui double %argument to i32
%is.batch = icmp eq i32 %mode, 0 %normalize.layer.group.base = mul i32 %normalize.row, %out.length
%normalize.layer.group = add i32 %normalize.layer.group.base, %normalize.position
%normalize.group = select i1 %is.batch, i32 %normalize.channel, i32 %normalize.layer.group
%normalize.batch.groups = add i32 %out.channels, 0 %normalize.layer.groups = mul i32 %rows, %out.length
%normalize.groups = select i1 %is.batch, i32 %normalize.batch.groups, i32 %normalize.layer.groups
%normalize.scale.index = add i32 %normalize.groups, %normalize.group
%normalize.mean.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %normalize.group
%normalize.scale.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %normalize.scale.index
%normalize.mean = load double, ptr addrspace(1) %normalize.mean.ptr, align 8
%normalize.scale = load double, ptr addrspace(1) %normalize.scale.ptr, align 8
%normalize.input.ptr = getelementptr inbounds double, ptr addrspace(1) %source, i32 %p
%normalize.input = load double, ptr addrspace(1) %normalize.input.ptr, align 8
%centered = fsub double %normalize.input, %normalize.mean %normalized = fmul double %centered, %normalize.scale
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %p
store double %normalized, ptr addrspace(1) %output.ptr, align 8 br label %element.done element.done:
%p.next = add i32 %p, %threads br label %element.loop node.done: call void @llvm.amdgcn.s.barrier()
%ended = call i64 @__ockl_steadyctr_u64() %elapsed = sub i64 %ended, %started %leader = icmp eq i32 %tid, 0
br i1 %leader, label %timing.store, label %timing.done timing.store:
%timing.ptr = getelementptr inbounds i64, ptr addrspace(1) %timings, i32 %node
store i64 %elapsed, ptr addrspace(1) %timing.ptr, align 8 br label %timing.done timing.done:
%node.next = add nuw i32 %node, 1 br label %node.loop invalid: call void @llvm.trap() br label %exit exit: ret void }
define protected amdgpu_kernel void @forward_graph(
ptr addrspace(1) nocapture readonly %samples, ptr addrspace(1) nocapture readonly %weights,
ptr addrspace(1) nocapture readonly %value_pointers, ptr addrspace(1) nocapture readonly %context_pointers,
ptr addrspace(1) nocapture readonly %descriptors, ptr addrspace(1) nocapture readonly %parameters,
i32 %rows, i32 %stages, i32 %threads, i32 %tile.m, i32 %tile.n, i32 %tile.k,
ptr addrspace(1) %timings, ptr addrspace(1) %tiles ) #0 { entry:
call void @tape_forward_body( ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %value_pointers,
ptr addrspace(1) %context_pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %parameters,
i32 %rows, i32 %stages, i32 %threads, i32 %tile.m, i32 %tile.n, i32 %tile.k,
ptr addrspace(1) %timings, ptr addrspace(1) %tiles ) ret void }
define internal double @loss_item(double %prediction, double %target, i32 %code, double %threshold) #1 { entry:
%difference = fsub double %prediction, %target %square = fmul double %difference, %difference
switch i32 %code, label %focal [ i32 0, label %mse i32 1, label %mse i32 2, label %huber i32 3, label %mae
i32 4, label %cross i32 5, label %cross ] mse: br label %done huber:
%absolute = call double @llvm.fabs.f64(double %difference) %small = fcmp ole double %absolute, %threshold
%half.square = fmul double %square, 0.5 %half.threshold = fmul double %threshold, 0.5
%large.base = fsub double %absolute, %half.threshold %large = fmul double %threshold, %large.base
%huber.value = select i1 %small, double %half.square, double %large br label %done mae:
%mae.value = call double @llvm.fabs.f64(double %difference) br label %done cross:
%probability.raw = call double @sigmoid(double %prediction)
%probability.low = fcmp olt double %probability.raw, 0x3CB0000000000000
%probability.lowered = select i1 %probability.low, double 0x3CB0000000000000, double %probability.raw
%probability.high = fcmp ogt double %probability.lowered, 0x3FEFFFFFFFFFFFFE
%probability = select i1 %probability.high, double 0x3FEFFFFFFFFFFFFE, double %probability.lowered
%target.low = fcmp olt double %target, 0.0 %target.lowered = select i1 %target.low, double 0.0, double %target
%target.high = fcmp ogt double %target.lowered, 1.0
%target.clamped = select i1 %target.high, double 1.0, double %target.lowered
%log.probability = call double @__ocml_log_f64(double %probability) %one.probability = fsub double 1.0, %probability
%log.one.probability = call double @__ocml_log_f64(double %one.probability)
%cross.first = fmul double %target.clamped, %log.probability %one.target = fsub double 1.0, %target.clamped
%cross.second = fmul double %one.target, %log.one.probability %cross.sum = fadd double %cross.first, %cross.second
%cross.value = fneg double %cross.sum br label %done focal:
%focal.probability = call double @sigmoid(double %prediction) %focal.target = fcmp oge double %target, 0.5
%focal.one = fsub double 1.0, %focal.probability
%focal.correct.raw = select i1 %focal.target, double %focal.probability, double %focal.one
%focal.low = fcmp olt double %focal.correct.raw, 0x3CB0000000000000
%focal.correct = select i1 %focal.low, double 0x3CB0000000000000, double %focal.correct.raw
%focal.incorrect = fsub double 1.0, %focal.correct %focal.square = fmul double %focal.incorrect, %focal.incorrect
%focal.log = call double @__ocml_log_f64(double %focal.correct) %focal.product = fmul double %focal.square, %focal.log
%focal.value = fneg double %focal.product br label %done done:
%result = phi double [ %square, %mse ], [ %huber.value, %huber ], [ %mae.value, %mae ],
[ %cross.value, %cross ], [ %focal.value, %focal ] ret double %result } define internal double @loss_gradient(
double %prediction, double %target, i32 %code, double %threshold, double %loss, i32 %rows ) #1 { entry:
%difference = fsub double %prediction, %target %rows.double = uitofp i32 %rows to double
switch i32 %code, label %focal [ i32 0, label %mse i32 1, label %rmse i32 2, label %huber i32 3, label %mae
i32 4, label %cross i32 5, label %cross ] mse: %twice = fadd double %difference, %difference
%mse.value = fdiv double %twice, %rows.double br label %done rmse: %rmse.denominator = fmul double %rows.double, %loss
%rmse.zero = fcmp oeq double %loss, 0.0 %rmse.divided = fdiv double %difference, %rmse.denominator
%rmse.value = select i1 %rmse.zero, double 0.0, double %rmse.divided br label %done huber:
%negative.threshold = fneg double %threshold %huber.low = fcmp olt double %difference, %negative.threshold
%huber.high = fcmp ogt double %difference, %threshold
%huber.lower = select i1 %huber.low, double %negative.threshold, double %difference
%huber.clamped = select i1 %huber.high, double %threshold, double %huber.lower
%huber.value = fdiv double %huber.clamped, %rows.double br label %done mae:
%mae.negative = fcmp olt double %difference, 0.0 %mae.positive = fcmp ogt double %difference, 0.0
%mae.upper = select i1 %mae.positive, double 1.0, double 0.0
%mae.sign = select i1 %mae.negative, double -1.0, double %mae.upper %mae.value = fdiv double %mae.sign, %rows.double
br label %done cross: %cross.probability = call double @sigmoid(double %prediction)
%cross.difference = fsub double %cross.probability, %target %cross.value = fdiv double %cross.difference, %rows.double
br label %done focal: %focal.probability = call double @sigmoid(double %prediction)
%focal.target = fcmp oge double %target, 0.5 %focal.one = fsub double 1.0, %focal.probability
%focal.correct.raw = select i1 %focal.target, double %focal.probability, double %focal.one
%focal.low = fcmp olt double %focal.correct.raw, 0x3CB0000000000000
%focal.correct = select i1 %focal.low, double 0x3CB0000000000000, double %focal.correct.raw
%focal.incorrect = fsub double 1.0, %focal.correct %focal.log = call double @__ocml_log_f64(double %focal.correct)
%focal.first = fmul double 2.0, %focal.incorrect %focal.first.value = fmul double %focal.first, %focal.log
%focal.square = fmul double %focal.incorrect, %focal.incorrect %focal.second = fdiv double %focal.square, %focal.correct
%focal.by.correct = fsub double %focal.first.value, %focal.second
%focal.sigmoid.derivative = fmul double %focal.probability, %focal.one
%focal.negative.direction = fneg double %focal.sigmoid.derivative
%focal.direction = select i1 %focal.target, double %focal.sigmoid.derivative, double %focal.negative.direction
%focal.chain = fmul double %focal.by.correct, %focal.direction %focal.value = fdiv double %focal.chain, %rows.double
br label %done done: %result = phi double [ %mse.value, %mse ], [ %rmse.value, %rmse ], [ %huber.value, %huber ],
[ %mae.value, %mae ], [ %cross.value, %cross ], [ %focal.value, %focal ] ret double %result }
define protected amdgpu_kernel void @tape_epoch_graph(
ptr addrspace(1) %samples, ptr addrspace(1) %input.adjoint, ptr addrspace(1) %targets, ptr addrspace(1) %weights,
ptr addrspace(1) %frozen, ptr addrspace(1) %best, ptr addrspace(1) %value.pointers, ptr addrspace(1) %context.pointers,
ptr addrspace(1) %adjoint.pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %arguments,
ptr addrspace(1) %metrics, ptr addrspace(1) %gradient, ptr addrspace(1) %moments,
ptr addrspace(1) %variances, ptr addrspace(1) %best.loss, i32 %rows, i32 %nodes, i32 %parameter.count, i32 %loss.code,
double %huber.threshold, double %rate, double %beta1, double %beta2,
double %beta1.power, double %beta2.power, double %epsilon, double %decay, double %tolerance, i32 %step,
i32 %threads, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %phase,
ptr addrspace(1) %timings, ptr addrspace(1) %tiles ) #0 { entry: %tid = call i32 @llvm.amdgcn.workitem.id.x()
%optimizer.only = icmp eq i32 %phase, 2 %gradient.only = icmp eq i32 %phase, 1
br i1 %optimizer.only, label %optimizer.entry, label %forward.entry forward.entry:
call void @tape_forward_body( ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %value.pointers,
ptr addrspace(1) %context.pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %arguments,
i32 %rows, i32 %nodes, i32 %threads, i32 %tile.m, i32 %tile.n, i32 %tile.k,
ptr addrspace(1) %timings, ptr addrspace(1) %tiles ) call void @llvm.amdgcn.s.barrier() %last = sub i32 %nodes, 1
%last.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %last
%last.address = load i64, ptr addrspace(1) %last.slot, align 8
%predictions = inttoptr i64 %last.address to ptr addrspace(1) %last.base = mul i32 %last, 11
%last.channels.index = add i32 %last.base, 5 %last.length.index = add i32 %last.base, 6
%last.channels.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %last.channels.index
%last.length.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %last.length.index
%last.channels = load i32, ptr addrspace(1) %last.channels.ptr, align 4
%last.length = load i32, ptr addrspace(1) %last.length.ptr, align 4
%last.elements = mul i32 %last.channels, %last.length %items = mul i32 %rows, %last.elements
%input.channels.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 3
%input.length.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 4
%input.channels = load i32, ptr addrspace(1) %input.channels.ptr, align 4
%input.length = load i32, ptr addrspace(1) %input.length.ptr, align 4
%input.elements = mul i32 %input.channels, %input.length %input.items = mul i32 %rows, %input.elements
%direct = icmp eq i32 %loss.code, 7 %leader = icmp eq i32 %tid, 0
br i1 %leader, label %objective.test, label %loss.done objective.test:
br i1 %direct, label %direct.loss, label %loss.loop direct.loss: store double 0.0, ptr addrspace(1) %metrics, align 8
%direct.trigger.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 1
%direct.better.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 2
store double 0.0, ptr addrspace(1) %direct.trigger.ptr, align 8
store double 0.0, ptr addrspace(1) %direct.better.ptr, align 8 br label %loss.done loss.loop:
%loss.p = phi i32 [ 0, %objective.test ], [ %loss.next, %loss.step ]
%loss.sum = phi double [ 0.0, %objective.test ], [ %loss.sum.next, %loss.step ]
%loss.more = icmp ult i32 %loss.p, %items br i1 %loss.more, label %loss.step, label %loss.store loss.step:
%loss.prediction.ptr = getelementptr inbounds double, ptr addrspace(1) %predictions, i32 %loss.p
%loss.target.ptr = getelementptr inbounds double, ptr addrspace(1) %targets, i32 %loss.p
%loss.prediction = load double, ptr addrspace(1) %loss.prediction.ptr, align 8
%loss.target = load double, ptr addrspace(1) %loss.target.ptr, align 8
%loss.item = call double @loss_item( double %loss.prediction, double %loss.target, i32 %loss.code,
double %huber.threshold ) %loss.sum.next = fadd double %loss.sum, %loss.item %loss.next = add nuw i32 %loss.p, 1
br label %loss.loop loss.store: %items.double = uitofp i32 %items to double
%loss.mean = fdiv double %loss.sum, %items.double %loss.root = call double @llvm.sqrt.f64(double %loss.mean)
%loss.is.rmse = icmp eq i32 %loss.code, 1 %loss.value = select i1 %loss.is.rmse, double %loss.root, double %loss.mean
%old.best = load double, ptr addrspace(1) %best.loss, align 8
%last.loss.ptr = getelementptr inbounds double, ptr addrspace(1) %best.loss, i32 1
%trail.ptr = getelementptr inbounds double, ptr addrspace(1) %best.loss, i32 2
%saved.ptr = getelementptr inbounds double, ptr addrspace(1) %best.loss, i32 3
%last.loss = load double, ptr addrspace(1) %last.loss.ptr, align 8
%trail = load double, ptr addrspace(1) %trail.ptr, align 8
%better = fcmp olt double %loss.value, %old.best %best.value = select i1 %better, double %loss.value, double %old.best
%last.exists = fcmp ord double %last.loss, %last.loss %trail.exists = fcmp ord double %trail, %trail
%not.trail = xor i1 %trail.exists, true %increased = fcmp ogt double %loss.value, %last.loss
%start.base = and i1 %last.exists, %not.trail %start = and i1 %start.base, %increased
%trail.value = select i1 %start, double %last.loss, double %trail %one.tolerance = fadd double 1.0, %tolerance
%rise.floor = fmul double %last.loss, %one.tolerance %above.floor = fcmp ogt double %loss.value, %rise.floor
%below.trail = fcmp olt double %loss.value, %trail.value %tolerance.active = fcmp ogt double %tolerance, 0.0
%trigger.base = and i1 %trail.exists, %above.floor %trigger.below = and i1 %trigger.base, %below.trail
%trigger = and i1 %trigger.below, %tolerance.active %trigger.value = select i1 %trigger, double 1.0, double 0.0
%better.value = select i1 %better, double 1.0, double 0.0 store double %loss.value, ptr addrspace(1) %metrics, align 8
%trigger.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 1
%better.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 2
store double %trigger.value, ptr addrspace(1) %trigger.ptr, align 8
store double %better.value, ptr addrspace(1) %better.ptr, align 8
store double %best.value, ptr addrspace(1) %best.loss, align 8
store double %loss.value, ptr addrspace(1) %last.loss.ptr, align 8
store double %trail.value, ptr addrspace(1) %trail.ptr, align 8 br i1 %trigger, label %save.state, label %loss.done
save.state: store double %best.value, ptr addrspace(1) %saved.ptr, align 8 br label %loss.done loss.done:
call void @llvm.amdgcn.s.barrier()
%checkpoint.flag.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 2
%checkpoint.flag = load double, ptr addrspace(1) %checkpoint.flag.ptr, align 8
%checkpoint.active = fcmp one double %checkpoint.flag, 0.0
br i1 %checkpoint.active, label %checkpoint.loop, label %clear.gradient.loop checkpoint.loop:
%checkpoint.p = phi i32 [ %tid, %loss.done ], [ %checkpoint.next, %checkpoint.step ]
%checkpoint.more = icmp ult i32 %checkpoint.p, %parameter.count
br i1 %checkpoint.more, label %checkpoint.step, label %clear.gradient.loop checkpoint.step:
%checkpoint.source.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %checkpoint.p
%checkpoint.target.ptr = getelementptr inbounds double, ptr addrspace(1) %best, i32 %checkpoint.p
%checkpoint.weight = load double, ptr addrspace(1) %checkpoint.source.ptr, align 8
store double %checkpoint.weight, ptr addrspace(1) %checkpoint.target.ptr, align 8
%checkpoint.next = add i32 %checkpoint.p, %threads br label %checkpoint.loop clear.gradient.loop:
%gradient.p = phi i32 [ %tid, %loss.done ], [ %tid, %checkpoint.loop ], [ %gradient.next, %gradient.step ]
%gradient.more = icmp ult i32 %gradient.p, %parameter.count
br i1 %gradient.more, label %gradient.step, label %clear.input.loop gradient.step:
%gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.p
store double 0.0, ptr addrspace(1) %gradient.ptr, align 8 %gradient.next = add i32 %gradient.p, %threads
br label %clear.gradient.loop clear.input.loop:
%clear.input.p = phi i32 [ %tid, %clear.gradient.loop ], [ %clear.input.next, %clear.input.step ]
%clear.input.more = icmp ult i32 %clear.input.p, %input.items
br i1 %clear.input.more, label %clear.input.step, label %clear.node.entry clear.input.step:
%clear.input.ptr = getelementptr inbounds double, ptr addrspace(1) %input.adjoint, i32 %clear.input.p
store double 0.0, ptr addrspace(1) %clear.input.ptr, align 8
%clear.input.next = add i32 %clear.input.p, %threads br label %clear.input.loop clear.node.entry:
call void @llvm.amdgcn.s.barrier() br label %clear.node.loop
clear.node.loop: %clear.node = phi i32 [ 0, %clear.node.entry ], [ %clear.node.next, %clear.node.done ]
%clear.node.more = icmp ult i32 %clear.node, %nodes br i1 %clear.node.more, label %clear.node.load, label %seed.loop
clear.node.load: %clear.base = mul i32 %clear.node, 11 %clear.channels.index = add i32 %clear.base, 5
%clear.length.index = add i32 %clear.base, 6
%clear.channels.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %clear.channels.index
%clear.length.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %clear.length.index
%clear.channels = load i32, ptr addrspace(1) %clear.channels.ptr, align 4
%clear.length = load i32, ptr addrspace(1) %clear.length.ptr, align 4
%clear.elements.0 = mul i32 %clear.channels, %clear.length %clear.elements = mul i32 %rows, %clear.elements.0
%clear.slot = getelementptr inbounds i64, ptr addrspace(1) %adjoint.pointers, i32 %clear.node
%clear.address = load i64, ptr addrspace(1) %clear.slot, align 8
%clear.values = inttoptr i64 %clear.address to ptr addrspace(1) br label %clear.item.loop clear.item.loop:
%clear.p = phi i32 [ %tid, %clear.node.load ], [ %clear.next, %clear.step ]
%clear.more = icmp ult i32 %clear.p, %clear.elements br i1 %clear.more, label %clear.step, label %clear.node.done
clear.step: %clear.ptr = getelementptr inbounds double, ptr addrspace(1) %clear.values, i32 %clear.p
store double 0.0, ptr addrspace(1) %clear.ptr, align 8 %clear.next = add i32 %clear.p, %threads
br label %clear.item.loop clear.node.done: call void @llvm.amdgcn.s.barrier()
%clear.node.next = add nuw i32 %clear.node, 1 br label %clear.node.loop seed.loop:
%seed.p = phi i32 [ %tid, %clear.node.loop ], [ %seed.next, %seed.step ] %seed.more = icmp ult i32 %seed.p, %items
br i1 %seed.more, label %seed.step, label %reverse.entry seed.step:
%seed.prediction.ptr = getelementptr inbounds double, ptr addrspace(1) %predictions, i32 %seed.p
%seed.target.ptr = getelementptr inbounds double, ptr addrspace(1) %targets, i32 %seed.p
%seed.prediction = load double, ptr addrspace(1) %seed.prediction.ptr, align 8
%seed.target = load double, ptr addrspace(1) %seed.target.ptr, align 8
%seed.loss = load double, ptr addrspace(1) %metrics, align 8
%seed.loss.value = call double @loss_gradient( double %seed.prediction, double %seed.target, i32 %loss.code,
double %huber.threshold, double %seed.loss, i32 %items )
%seed.value = select i1 %direct, double %seed.target, double %seed.loss.value
%last.adjoint.slot = getelementptr inbounds i64, ptr addrspace(1) %adjoint.pointers, i32 %last
%last.adjoint.address = load i64, ptr addrspace(1) %last.adjoint.slot, align 8
%last.adjoint = inttoptr i64 %last.adjoint.address to ptr addrspace(1)
%seed.ptr = getelementptr inbounds double, ptr addrspace(1) %last.adjoint, i32 %seed.p
store double %seed.value, ptr addrspace(1) %seed.ptr, align 8 %seed.next = add i32 %seed.p, %threads br label %seed.loop
reverse.entry: call void @llvm.amdgcn.s.barrier() %reverse.first = sub i32 %nodes, 1 br label %reverse.loop
reverse.loop: %node = phi i32 [ %reverse.first, %reverse.entry ], [ %node.next, %node.done ]
%node.more = icmp sge i32 %node, 0 br i1 %node.more, label %node.load, label %backward.done backward.done:
br i1 %gradient.only, label %exit, label %optimizer.entry node.load:
%base = mul i32 %node, 11 %descriptor.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %base
%descriptor.second.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptor.ptr, i32 4
%descriptor.first = load <4 x i32>, ptr addrspace(1) %descriptor.ptr, align 4
%descriptor.second = load <4 x i32>, ptr addrspace(1) %descriptor.second.ptr, align 4
%op = extractelement <4 x i32> %descriptor.first, i32 0 %source.node = extractelement <4 x i32> %descriptor.first, i32 1
%second.node = extractelement <4 x i32> %descriptor.first, i32 2
%in.channels = extractelement <4 x i32> %descriptor.first, i32 3
%in.length = extractelement <4 x i32> %descriptor.second, i32 0
%out.channels = extractelement <4 x i32> %descriptor.second, i32 1
%out.length = extractelement <4 x i32> %descriptor.second, i32 2
%offset = extractelement <4 x i32> %descriptor.second, i32 3 %node.parameters.index = add i32 %base, 8
%program.offset.index = add i32 %base, 9 %program.count.index = add i32 %base, 10
%node.parameters.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %node.parameters.index
%program.offset.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %program.offset.index
%program.count.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %program.count.index
%node.parameters = load i32, ptr addrspace(1) %node.parameters.ptr, align 4
%program.offset = load i32, ptr addrspace(1) %program.offset.ptr, align 4
%program.count = load i32, ptr addrspace(1) %program.count.ptr, align 4
%program = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %program.offset
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length
%source.exists = icmp sge i32 %source.node, 0 %source.safe = select i1 %source.exists, i32 %source.node, i32 0
%source.value.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %source.safe
%source.value.address = load i64, ptr addrspace(1) %source.value.slot, align 8
%source.node.values = inttoptr i64 %source.value.address to ptr addrspace(1)
%source.values = select i1 %source.exists, ptr addrspace(1) %source.node.values, ptr addrspace(1) %samples
%second.exists = icmp sge i32 %second.node, 0 %second.safe = select i1 %second.exists, i32 %second.node, i32 0
%second.value.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %second.safe
%second.value.address = load i64, ptr addrspace(1) %second.value.slot, align 8
%second.values = inttoptr i64 %second.value.address to ptr addrspace(1)
%source.adjoint.slot = getelementptr inbounds i64, ptr addrspace(1) %adjoint.pointers, i32 %source.safe
%source.adjoint.address = load i64, ptr addrspace(1) %source.adjoint.slot, align 8
%source.node.adjoint = inttoptr i64 %source.adjoint.address to ptr addrspace(1)
%source.adjoint = select i1 %source.exists, ptr addrspace(1) %source.node.adjoint, ptr addrspace(1) %input.adjoint
%second.adjoint.slot = getelementptr inbounds i64, ptr addrspace(1) %adjoint.pointers, i32 %second.safe
%second.adjoint.address = load i64, ptr addrspace(1) %second.adjoint.slot, align 8
%second.adjoint = inttoptr i64 %second.adjoint.address to ptr addrspace(1)
%second.values.safe = select i1 %second.exists, ptr addrspace(1) %second.values, ptr addrspace(1) %source.values
%second.adjoint.safe = select i1 %second.exists, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %source.adjoint
%delta.slot = getelementptr inbounds i64, ptr addrspace(1) %adjoint.pointers, i32 %node
%delta.address = load i64, ptr addrspace(1) %delta.slot, align 8
%delta = inttoptr i64 %delta.address to ptr addrspace(1)
%node.value.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %node
%node.value.address = load i64, ptr addrspace(1) %node.value.slot, align 8
%node.values = inttoptr i64 %node.value.address to ptr addrspace(1) %argument.base = mul i32 %node, 9
%argument.ptr = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %argument.base
%argument = load double, ptr addrspace(1) %argument.ptr, align 8 switch i32 %op, label %invalid [
i32 0, label %contraction.gradient i32 2, label %pool.gradient.loop i32 3, label %gather.gradient
i32 4, label %attention.gradient i32 5, label %scan.gradient i32 6, label %elementwise.gradient
i32 7, label %route.gradient.loop i32 8, label %normalization.stats.loop ] contraction.gradient:
%contraction.matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %offset
%contraction.kernel = fptoui double %argument to i32 call void @contraction_reverse_body(
ptr addrspace(1) %source.values, ptr addrspace(1) %contraction.matrix, ptr addrspace(1) %delta,
ptr addrspace(1) %source.adjoint, ptr addrspace(1) %gradient, i1 true,
i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels,
i32 %out.length, i32 %contraction.kernel, i32 %offset, i32 %threads ) br label %node.done gather.gradient:
%gather.vocabulary = fptoui double %argument to i32
call void @embedding_reverse_body( ptr addrspace(1) %source.values, ptr addrspace(1) %delta, ptr addrspace(1) %gradient,
i32 %rows, i32 %in.elements, i32 %out.channels, i32 %gather.vocabulary, i32 %offset, i32 %threads ) br label %node.done
attention.gradient: %attention.context.slot = getelementptr inbounds i64, ptr addrspace(1) %context.pointers, i32 %node
%attention.context.address = load i64, ptr addrspace(1) %attention.context.slot, align 8
%attention.context = inttoptr i64 %attention.context.address to ptr addrspace(1)
%attention.matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %offset
%attention.heads = fptoui double %argument to i32
call void @attention_reverse_body( ptr addrspace(1) %source.values, ptr addrspace(1) %attention.matrix,
ptr addrspace(1) %attention.context, ptr addrspace(1) %delta,
ptr addrspace(1) %source.adjoint, ptr addrspace(1) %gradient, i1 true,
i32 %rows, i32 %in.elements, i32 %attention.heads, i32 %in.channels, i32 %offset, i32 %threads ) br label %node.done
scan.gradient: %scan.context.slot = getelementptr inbounds i64, ptr addrspace(1) %context.pointers, i32 %node
%scan.context.address = load i64, ptr addrspace(1) %scan.context.slot, align 8
%scan.context = inttoptr i64 %scan.context.address to ptr addrspace(1)
%scan.matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %offset
%scan.gates = fptoui double %argument to i32 call void @scan_reverse_body(
ptr addrspace(1) %source.values, ptr addrspace(1) %scan.matrix, ptr addrspace(1) %node.values,
ptr addrspace(1) %scan.context, ptr addrspace(1) %delta, ptr addrspace(1) %source.adjoint,
ptr addrspace(1) %gradient, i1 true, i32 %rows, i32 %in.channels,
i32 %in.length, i32 %out.channels, i32 %scan.gates, i32 %node.parameters, i32 %offset, i32 %threads )
br label %node.done pool.gradient.loop: %pool.p = phi i32 [ %tid, %node.load ], [ %pool.next, %pool.gradient.next ]
%pool.count = mul i32 %rows, %out.elements %pool.more = icmp ult i32 %pool.p, %pool.count
br i1 %pool.more, label %pool.gradient.step, label %node.done pool.gradient.step:
%pool.context.slot = getelementptr inbounds i64, ptr addrspace(1) %context.pointers, i32 %node
%pool.context.address = load i64, ptr addrspace(1) %pool.context.slot, align 8
%pool.context = inttoptr i64 %pool.context.address to ptr addrspace(1)
%pool.index.ptr = getelementptr inbounds double, ptr addrspace(1) %pool.context, i32 %pool.p
%pool.index.double = load double, ptr addrspace(1) %pool.index.ptr, align 8
%pool.index = fptoui double %pool.index.double to i32
%pool.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %pool.p
%pool.delta = load double, ptr addrspace(1) %pool.delta.ptr, align 8 br label %pool.gradient.add pool.gradient.add:
%pool.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %source.adjoint, i32 %pool.index
%pool.previous = load double, ptr addrspace(1) %pool.previous.ptr, align 8
%pool.value = fadd double %pool.previous, %pool.delta
store double %pool.value, ptr addrspace(1) %pool.previous.ptr, align 8 br label %pool.gradient.next pool.gradient.next:
%pool.next = add i32 %pool.p, %threads br label %pool.gradient.loop elementwise.gradient:
%elementwise.context.slot = getelementptr inbounds i64, ptr addrspace(1) %context.pointers, i32 %node
%elementwise.context.address = load i64, ptr addrspace(1) %elementwise.context.slot, align 8
%elementwise.context = inttoptr i64 %elementwise.context.address to ptr addrspace(1)
%elementwise.elements = mul i32 %rows, %out.elements
call void @scalar_reverse_body( ptr addrspace(1) %source.values, ptr addrspace(1) %second.values.safe,
ptr addrspace(1) %source.adjoint, ptr addrspace(1) %second.adjoint.safe,
ptr addrspace(1) %elementwise.context, ptr addrspace(1) %program,
ptr addrspace(1) %delta, ptr addrspace(1) %gradient, i32 %offset, i32 %elementwise.elements, i32 %program.count,
i1 true, i1 %second.exists, i32 %threads ) br label %node.done route.gradient.loop:
%route.p = phi i32 [ %tid, %node.load ], [ %route.next, %route.gradient.step ]
%route.count = mul i32 %rows, %out.elements %route.more = icmp ult i32 %route.p, %route.count
br i1 %route.more, label %route.gradient.step, label %node.done route.gradient.step:
%route.row = udiv i32 %route.p, %out.elements %route.column = urem i32 %route.p, %out.elements
%route.base = mul i32 %route.column, 3 %route.stride.index = add i32 %route.base, 1
%route.field.index = add i32 %route.base, 2
%route.source.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %route.base
%route.stride.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %route.stride.index
%route.field.ptr = getelementptr inbounds double, ptr addrspace(1) %program, i32 %route.field.index
%route.source.double = load double, ptr addrspace(1) %route.source.ptr, align 8
%route.stride.double = load double, ptr addrspace(1) %route.stride.ptr, align 8
%route.field.double = load double, ptr addrspace(1) %route.field.ptr, align 8
%route.source = fptosi double %route.source.double to i32 %route.stride = fptoui double %route.stride.double to i32
%route.field = fptoui double %route.field.double to i32
%route.exists = icmp sge i32 %route.source, 0 %route.safe = select i1 %route.exists, i32 %route.source, i32 0
%route.slot = getelementptr inbounds i64, ptr addrspace(1) %adjoint.pointers, i32 %route.safe
%route.address = load i64, ptr addrspace(1) %route.slot, align 8
%route.node.adjoint = inttoptr i64 %route.address to ptr addrspace(1)
%route.target = select i1 %route.exists, ptr addrspace(1) %route.node.adjoint, ptr addrspace(1) %input.adjoint
%route.row.base = mul i32 %route.row, %route.stride %route.index = add i32 %route.row.base, %route.field
%route.target.ptr = getelementptr inbounds double, ptr addrspace(1) %route.target, i32 %route.index
%route.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %route.p
%route.delta = load double, ptr addrspace(1) %route.delta.ptr, align 8
%route.old = atomicrmw fadd ptr addrspace(1) %route.target.ptr, double %route.delta monotonic, align 8
%route.next = add i32 %route.p, %threads br label %route.gradient.loop normalization.stats.loop:
%normalization.stats.group = phi i32 [ %tid, %node.load ], [ %normalization.stats.next, %normalization.stats.store ]
%normalization.mode = fptoui double %argument to i32 %normalization.batch = icmp eq i32 %normalization.mode, 0
%normalization.layer.groups = mul i32 %rows, %out.length
%normalization.groups = select i1 %normalization.batch, i32 %out.channels, i32 %normalization.layer.groups
%normalization.items.batch = mul i32 %rows, %out.length
%normalization.items = select i1 %normalization.batch, i32 %normalization.items.batch, i32 %out.channels
%normalization.context.slot = getelementptr inbounds i64, ptr addrspace(1) %context.pointers, i32 %node
%normalization.context.address = load i64, ptr addrspace(1) %normalization.context.slot, align 8
%normalization.context = inttoptr i64 %normalization.context.address to ptr addrspace(1)
%normalization.sum.base = mul i32 %normalization.groups, 2
%normalization.projected.base = mul i32 %normalization.groups, 3
%normalization.stats.more = icmp ult i32 %normalization.stats.group, %normalization.groups
br i1 %normalization.stats.more, label %normalization.stats.sum.loop, label %normalization.stats.done
normalization.stats.sum.loop: %normalization.stats.item = phi i32 [ 0, %normalization.stats.loop ],
[ %normalization.stats.item.next, %normalization.stats.sum.step ]
%normalization.stats.sum = phi double [ 0.0, %normalization.stats.loop ],
[ %normalization.stats.sum.next, %normalization.stats.sum.step ]
%normalization.stats.projected = phi double [ 0.0, %normalization.stats.loop ],
[ %normalization.stats.projected.next, %normalization.stats.sum.step ]
%normalization.stats.item.more = icmp ult i32 %normalization.stats.item, %normalization.items
br i1 %normalization.stats.item.more, label %normalization.stats.sum.step, label %normalization.stats.store
normalization.stats.sum.step: %normalization.stats.batch.row = udiv i32 %normalization.stats.item, %out.length
%normalization.stats.batch.position = urem i32 %normalization.stats.item, %out.length
%normalization.stats.batch.row.base = mul i32 %normalization.stats.batch.row, %out.elements
%normalization.stats.batch.channel.base = mul i32 %normalization.stats.group, %out.length
%normalization.stats.batch.local = add i32 %normalization.stats.batch.channel.base, %normalization.stats.batch.position
%normalization.stats.batch.index = add i32 %normalization.stats.batch.row.base, %normalization.stats.batch.local
%normalization.stats.layer.row = udiv i32 %normalization.stats.group, %out.length
%normalization.stats.layer.position = urem i32 %normalization.stats.group, %out.length
%normalization.stats.layer.row.base = mul i32 %normalization.stats.layer.row, %out.elements
%normalization.stats.layer.channel.base = mul i32 %normalization.stats.item, %out.length
%normalization.stats.layer.local = add i32 %normalization.stats.layer.channel.base, %normalization.stats.layer.position
%normalization.stats.layer.index = add i32 %normalization.stats.layer.row.base, %normalization.stats.layer.local
%normalization.stats.index = select i1 %normalization.batch, i32 %normalization.stats.batch.index,
i32 %normalization.stats.layer.index
%normalization.stats.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %normalization.stats.index
%normalization.stats.output.ptr = getelementptr inbounds double, ptr addrspace(1) %node.values,
i32 %normalization.stats.index
%normalization.stats.delta = load double, ptr addrspace(1) %normalization.stats.delta.ptr, align 8
%normalization.stats.output = load double, ptr addrspace(1) %normalization.stats.output.ptr, align 8
%normalization.stats.product = fmul double %normalization.stats.delta, %normalization.stats.output
%normalization.stats.sum.next = fadd double %normalization.stats.sum, %normalization.stats.delta
%normalization.stats.projected.next = fadd double %normalization.stats.projected, %normalization.stats.product
%normalization.stats.item.next = add nuw i32 %normalization.stats.item, 1 br label %normalization.stats.sum.loop
normalization.stats.store: %normalization.sum.index = add i32 %normalization.sum.base, %normalization.stats.group
%normalization.projected.index = add i32 %normalization.projected.base, %normalization.stats.group
%normalization.sum.ptr = getelementptr inbounds double, ptr addrspace(1) %normalization.context,
i32 %normalization.sum.index
%normalization.projected.ptr = getelementptr inbounds double, ptr addrspace(1) %normalization.context,
i32 %normalization.projected.index
store double %normalization.stats.sum, ptr addrspace(1) %normalization.sum.ptr, align 8
store double %normalization.stats.projected, ptr addrspace(1) %normalization.projected.ptr, align 8
%normalization.stats.next = add i32 %normalization.stats.group, %threads br label %normalization.stats.loop
normalization.stats.done: call void @llvm.amdgcn.s.barrier() br label %normalization.gradient.loop
normalization.gradient.loop:
%normalization.p = phi i32 [ %tid, %normalization.stats.done ], [ %normalization.next, %normalization.store ]
%normalization.count = mul i32 %rows, %out.elements
%normalization.more = icmp ult i32 %normalization.p, %normalization.count
br i1 %normalization.more, label %normalization.step, label %node.done normalization.step:
%normalization.row = udiv i32 %normalization.p, %out.elements
%normalization.local = urem i32 %normalization.p, %out.elements
%normalization.channel = udiv i32 %normalization.local, %out.length
%normalization.position = urem i32 %normalization.local, %out.length
%normalization.layer.base = mul i32 %normalization.row, %out.length
%normalization.layer.group = add i32 %normalization.layer.base, %normalization.position
%normalization.group = select i1 %normalization.batch, i32 %normalization.channel, i32 %normalization.layer.group
%normalization.scale.index = add i32 %normalization.groups, %normalization.group
%normalization.element.sum.index = add i32 %normalization.sum.base, %normalization.group
%normalization.element.projected.index = add i32 %normalization.projected.base, %normalization.group
%normalization.scale.ptr = getelementptr inbounds double, ptr addrspace(1) %normalization.context,
i32 %normalization.scale.index
%normalization.element.sum.ptr = getelementptr inbounds double, ptr addrspace(1) %normalization.context,
i32 %normalization.element.sum.index
%normalization.element.projected.ptr = getelementptr inbounds double, ptr addrspace(1) %normalization.context,
i32 %normalization.element.projected.index
%normalization.scale = load double, ptr addrspace(1) %normalization.scale.ptr, align 8
%normalization.sum = load double, ptr addrspace(1) %normalization.element.sum.ptr, align 8
%normalization.projected = load double, ptr addrspace(1) %normalization.element.projected.ptr, align 8
br label %normalization.store normalization.store:
%normalization.current.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %normalization.p
%normalization.current.output.ptr = getelementptr inbounds double, ptr addrspace(1) %node.values, i32 %normalization.p
%normalization.current.delta = load double, ptr addrspace(1) %normalization.current.delta.ptr, align 8
%normalization.current.output = load double, ptr addrspace(1) %normalization.current.output.ptr, align 8
%normalization.items.double = uitofp i32 %normalization.items to double
%normalization.scaled.delta = fmul double %normalization.items.double, %normalization.current.delta
%normalization.output.projection = fmul double %normalization.current.output, %normalization.projected
%normalization.centered = fsub double %normalization.scaled.delta, %normalization.sum
%normalization.numerator = fsub double %normalization.centered, %normalization.output.projection
%normalization.scaled = fmul double %normalization.scale, %normalization.numerator
%normalization.contribution = fdiv double %normalization.scaled, %normalization.items.double
%normalization.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %source.adjoint, i32 %normalization.p
%normalization.previous = load double, ptr addrspace(1) %normalization.previous.ptr, align 8
%normalization.value = fadd double %normalization.previous, %normalization.contribution
store double %normalization.value, ptr addrspace(1) %normalization.previous.ptr, align 8
%normalization.next = add i32 %normalization.p, %threads br label %normalization.gradient.loop node.done:
call void @llvm.amdgcn.s.barrier() %node.next = sub i32 %node, 1 br label %reverse.loop optimizer.entry:
%optimizer.initial.more = icmp ult i32 %tid, %parameter.count
br i1 %optimizer.initial.more, label %optimizer.initial.load, label %exit optimizer.initial.load:
%one.beta1 = fsub double 1.0, %beta1 %one.beta2 = fsub double 1.0, %beta2
%m.correction = fsub double 1.0, %beta1.power %v.correction = fsub double 1.0, %beta2.power
%m.inverse = fdiv double 1.0, %m.correction %v.inverse = fdiv double 1.0, %v.correction
%optimizer.initial.frozen.ptr = getelementptr inbounds i8, ptr addrspace(1) %frozen, i32 %tid
%optimizer.initial.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %tid
%optimizer.initial.moment.ptr = getelementptr inbounds double, ptr addrspace(1) %moments, i32 %tid
%optimizer.initial.variance.ptr = getelementptr inbounds double, ptr addrspace(1) %variances, i32 %tid
%optimizer.initial.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %tid
%optimizer.initial.frozen = load i8, ptr addrspace(1) %optimizer.initial.frozen.ptr, align 1
%optimizer.initial.g = load double, ptr addrspace(1) %optimizer.initial.gradient.ptr, align 8
%optimizer.initial.m = load double, ptr addrspace(1) %optimizer.initial.moment.ptr, align 8
%optimizer.initial.v = load double, ptr addrspace(1) %optimizer.initial.variance.ptr, align 8
%optimizer.initial.weight = load double, ptr addrspace(1) %optimizer.initial.weight.ptr, align 8
br label %optimizer.loop optimizer.loop:
%optimizer.p = phi i32 [ %tid, %optimizer.initial.load ], [ %optimizer.next, %optimizer.advance ]
%optimizer.frozen = phi i8 [ %optimizer.initial.frozen, %optimizer.initial.load ], [ %optimizer.next.frozen,
%optimizer.advance ]
%g = phi double [ %optimizer.initial.g, %optimizer.initial.load ], [ %optimizer.next.g, %optimizer.advance ]
%m.old = phi double [ %optimizer.initial.m, %optimizer.initial.load ], [ %optimizer.next.m, %optimizer.advance ]
%v.old = phi double [ %optimizer.initial.v, %optimizer.initial.load ], [ %optimizer.next.v, %optimizer.advance ]
%weight = phi double [ %optimizer.initial.weight, %optimizer.initial.load ], [ %optimizer.next.weight,
%optimizer.advance ]
%optimizer.next = add i32 %optimizer.p, %threads %optimizer.next.more = icmp ult i32 %optimizer.next, %parameter.count
%optimizer.next.safe = select i1 %optimizer.next.more, i32 %optimizer.next, i32 0
%optimizer.next.frozen.ptr = getelementptr inbounds i8, ptr addrspace(1) %frozen, i32 %optimizer.next.safe
%optimizer.next.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %optimizer.next.safe
%optimizer.next.moment.ptr = getelementptr inbounds double, ptr addrspace(1) %moments, i32 %optimizer.next.safe
%optimizer.next.variance.ptr = getelementptr inbounds double, ptr addrspace(1) %variances, i32 %optimizer.next.safe
%optimizer.next.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %optimizer.next.safe
%optimizer.next.frozen = load i8, ptr addrspace(1) %optimizer.next.frozen.ptr, align 1
%optimizer.next.g = load double, ptr addrspace(1) %optimizer.next.gradient.ptr, align 8
%optimizer.next.m = load double, ptr addrspace(1) %optimizer.next.moment.ptr, align 8
%optimizer.next.v = load double, ptr addrspace(1) %optimizer.next.variance.ptr, align 8
%optimizer.next.weight = load double, ptr addrspace(1) %optimizer.next.weight.ptr, align 8
%optimizer.is.frozen = icmp ne i8 %optimizer.frozen, 0
br i1 %optimizer.is.frozen, label %optimizer.advance, label %optimizer.update optimizer.update:
%optimizer.moment.ptr = getelementptr inbounds double, ptr addrspace(1) %moments, i32 %optimizer.p
%optimizer.variance.ptr = getelementptr inbounds double, ptr addrspace(1) %variances, i32 %optimizer.p
%optimizer.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %optimizer.p
%m.old.part = fmul double %beta1, %m.old %m.new.part = fmul double %one.beta1, %g
%m = fadd double %m.old.part, %m.new.part %g.square = fmul double %g, %g %v.old.part = fmul double %beta2, %v.old
%v.new.part = fmul double %one.beta2, %g.square %v = fadd double %v.old.part, %v.new.part
store double %m, ptr addrspace(1) %optimizer.moment.ptr, align 8
store double %v, ptr addrspace(1) %optimizer.variance.ptr, align 8 %m.hat = fmul double %m, %m.inverse
%v.hat = fmul double %v, %v.inverse %root = call double @llvm.sqrt.f64(double %v.hat)
%denominator = fadd double %root, %epsilon %direction = fdiv double %m.hat, %denominator
%decay.value = fmul double %decay, %weight %update.direction = fadd double %direction, %decay.value
%update = fmul double %rate, %update.direction %updated = fsub double %weight, %update
store double %updated, ptr addrspace(1) %optimizer.weight.ptr, align 8 br label %optimizer.advance optimizer.advance:
br i1 %optimizer.next.more, label %optimizer.loop, label %exit
invalid: call void @llvm.trap() br label %exit exit: ret void }
attributes #0 = { nounwind "amdgpu-flat-work-group-size"="1,1024" } attributes #1 = { alwaysinline nounwind }
