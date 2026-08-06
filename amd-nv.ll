target triple = "amdgcn-amd-amdhsa" declare i32 @llvm.amdgcn.workitem.id.x()
declare void @llvm.amdgcn.s.barrier() declare double @llvm.sqrt.f64(double) declare double @llvm.fabs.f64(double)
declare double @__ocml_exp_f64(double) declare double @__ocml_tanh_f64(double) declare double @__ocml_cos_f64(double)
declare double @__ocml_sin_f64(double) declare double @__ocml_log_f64(double) declare void @llvm.trap()
define internal void @contraction_forward_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output,
i32 %p, i32 %in.channels, i32 %in.length, i32 %out.channels, i32 %out.length, i32 %kernel ) #1 { entry:
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length
%is.conv = icmp ne i32 %kernel, 0 %span = select i1 %is.conv, i32 %kernel, i32 1 %terms = mul i32 %in.channels, %span
%row = udiv i32 %p, %out.elements %local = urem i32 %p, %out.elements %out.channel = udiv i32 %local, %out.length
%position = urem i32 %local, %out.length %row.base = mul i32 %row, %in.elements br label %loop loop:
%i = phi i32 [ 0, %entry ], [ %next, %step ] %sum = phi double [ 0.0, %entry ], [ %sum.next, %step ]
%more = icmp ult i32 %i, %terms br i1 %more, label %step, label %done step: %channel = udiv i32 %i, %span
%window.offset = urem i32 %i, %span %offset = select i1 %is.conv, i32 %window.offset, i32 0
%channel.base = mul i32 %channel, %in.length %input.local.0 = add i32 %channel.base, %position
%input.local = add i32 %input.local.0, %offset %input.index = add i32 %row.base, %input.local
%conv.weight.base = mul i32 %out.channel, %terms %conv.weight.index = add i32 %conv.weight.base, %i
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
%weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %conv.weight.index
%x = load double, ptr addrspace(1) %input.ptr, align 8 %w = load double, ptr addrspace(1) %weight.ptr, align 8
%product = fmul double %x, %w %sum.next = fadd double %sum, %product %next = add i32 %i, 1 br label %loop done:
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %sum, ptr addrspace(1) %output.ptr, align 8 ret void }
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
i32 12, label %hyperbolic i32 13, label %greater i32 14, label %surrogate ]
add: %add.result = fadd double %left.value, %right.value
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
br label %operation.done surrogate: br label %operation.done operation.done:
%result = phi double [ %add.result, %add ], [ %left.double, %constant ],
[ %parameter.result, %parameter ], [ %subtract.result, %subtract ],
[ %multiply.result, %multiply ], [ %divide.result, %divide ],
[ %absolute.result, %absolute ], [ %exponential.result, %exponential ],
[ %logarithm.result, %logarithm ], [ %square.root.result, %square.root ],
[ %sine.result, %sine ], [ %cosine.result, %cosine ], [ %hyperbolic.result, %hyperbolic ],
[ %greater.result, %greater ],
[ %right.value, %surrogate ]
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
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%adjoint.plane = mul i32 %instructions, %elements br label %element.loop
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
i32 14, label %surrogate.reverse
] add.reverse: call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements, i32 %instructions, double %adjoint, i1 %write.first, i1 %write.second )
call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %right, i32 %p, i32 %elements, i32 %instructions, double %adjoint, i1 %write.first, i1 %write.second )
br label %reverse.done surrogate.reverse: call void @scalar_add_adjoint(
ptr addrspace(1) %first.adjoint, ptr addrspace(1) %second.adjoint, ptr addrspace(1) %context,
i32 %left, i32 %p, i32 %elements, i32 %instructions, double %adjoint, i1 %write.first, i1 %write.second )
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
define internal i32 @embedding_index(double %value, i32 %vocabulary) #1 { entry:
%ordered = fcmp ord double %value, %value br i1 %ordered, label %convert, label %invalid convert:
%limit = uitofp i32 %vocabulary to double %remainder = frem double %value, %limit
%absolute = call double @llvm.fabs.f64(double %remainder) %index = fptoui double %absolute to i32 ret i32 %index
invalid: ret i32 0 }
define internal void @embedding_forward_body(
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
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%parameters = mul i32 %dimensions, %vocabulary br label %parameter.loop
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
i32 %rows, i32 %from, i32 %heads, i32 %channels, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%length = udiv i32 %from, %channels %head_width = udiv i32 %channels, %heads
%head_width.double = uitofp i32 %head_width to double %scale = call double @llvm.sqrt.f64(double %head_width.double)
%plane = mul i32 %rows, %from %projection.count = mul i32 %plane, 3 %matrix = mul i32 %channels, %channels
br label %projection.loop projection.loop:
%projection.p = phi i32 [ %tid, %entry ], [ %projection.next, %projection.store ]
%projection.more = icmp ult i32 %projection.p, %projection.count
br i1 %projection.more, label %projection.step, label %projection.done projection.step:
%projection = udiv i32 %projection.p, %plane %within = urem i32 %projection.p, %plane %row = udiv i32 %within, %from
%local = urem i32 %within, %from %output.channel = udiv i32 %local, %length %time = urem i32 %local, %length
%row.base = mul i32 %row, %from %projection.weight.base = mul i32 %projection, %matrix
%output.weight.base = mul i32 %output.channel, %channels
%weight.base = add i32 %projection.weight.base, %output.weight.base br label %projection.channel.loop
projection.channel.loop:
%input.channel = phi i32 [ 0, %projection.step ], [ %input.channel.next, %projection.channel.step ]
%projection.sum = phi double [ 0.0, %projection.step ], [ %projection.sum.next, %projection.channel.step ]
%channel.more = icmp ult i32 %input.channel, %channels
br i1 %channel.more, label %projection.channel.step, label %projection.store projection.channel.step:
%input.channel.base = mul i32 %input.channel, %length %input.local = add i32 %input.channel.base, %time
%input.index = add i32 %row.base, %input.local %weight.index = add i32 %weight.base, %input.channel
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
%weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.index
%input.value = load double, ptr addrspace(1) %input.ptr, align 8
%weight.value = load double, ptr addrspace(1) %weight.ptr, align 8
%projection.product = fmul double %input.value, %weight.value
%projection.sum.next = fadd double %projection.sum, %projection.product %input.channel.next = add i32 %input.channel, 1
br label %projection.channel.loop projection.store:
%context.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %projection.p
store double %projection.sum, ptr addrspace(1) %context.ptr, align 8 %projection.next = add i32 %projection.p, %threads
br label %projection.loop projection.done: call void @llvm.amdgcn.s.barrier() br label %output.loop output.loop:
%p = phi i32 [ %tid, %projection.done ], [ %p.next, %output.store ] %output.more = icmp ult i32 %p, %plane
br i1 %output.more, label %output.step, label %exit output.step: %output.row = udiv i32 %p, %from
%output.local = urem i32 %p, %from %output.channel.index = udiv i32 %output.local, %length
%query = urem i32 %output.local, %length %head = udiv i32 %output.channel.index, %head_width br label %maximum.loop
maximum.loop: %maximum.key = phi i32 [ 0, %output.step ], [ %maximum.next, %maximum.step ]
%maximum = phi double [ 0xFFF0000000000000, %output.step ], [ %maximum.value, %maximum.step ]
%maximum.more = icmp ult i32 %maximum.key, %length br i1 %maximum.more, label %maximum.step, label %softmax.loop
maximum.step:
%score = call double @attention_score( ptr addrspace(1) %context, i32 %plane, i32 %output.row, i32 %head, i32 %query,
i32 %maximum.key, i32 %from, i32 %length, i32 %head_width, double %scale ) %larger = fcmp ogt double %score, %maximum
%maximum.value = select i1 %larger, double %score, double %maximum %maximum.next = add i32 %maximum.key, 1
br label %maximum.loop softmax.loop: %key = phi i32 [ 0, %maximum.loop ], [ %key.next, %softmax.step ]
%denominator = phi double [ 0.0, %maximum.loop ], [ %denominator.next, %softmax.step ]
%numerator = phi double [ 0.0, %maximum.loop ], [ %numerator.next, %softmax.step ]
%key.more = icmp ult i32 %key, %length br i1 %key.more, label %softmax.step, label %output.store softmax.step:
%softmax.score = call double @attention_score( ptr addrspace(1) %context, i32 %plane, i32 %output.row, i32 %head,
i32 %query, i32 %key, i32 %from, i32 %length, i32 %head_width, double %scale )
%centered = fsub double %softmax.score, %maximum %exponential = call double @__ocml_exp_f64(double %centered)
%denominator.next = fadd double %denominator, %exponential %value.plane = mul i32 %plane, 2
%value.row = mul i32 %output.row, %from %value.channel.base = mul i32 %output.channel.index, %length
%value.local = add i32 %value.channel.base, %key %value.row.index = add i32 %value.row, %value.local
%value.index = add i32 %value.plane, %value.row.index
%value.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %value.index
%value = load double, ptr addrspace(1) %value.ptr, align 8 %weighted = fmul double %exponential, %value
%numerator.next = fadd double %numerator, %weighted %key.next = add i32 %key, 1 br label %softmax.loop output.store:
%attention = fdiv double %numerator, %denominator
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %attention, ptr addrspace(1) %output.ptr, align 8 %p.next = add i32 %p, %threads br label %output.loop
exit: ret void } define internal void @attention_reverse_body(
ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %context,
ptr addrspace(1) %delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient,
i1 %write.previous, i32 %rows, i32 %from, i32 %heads, i32 %channels, i32 %offset, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%length = udiv i32 %from, %channels
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
br i1 %query.more, label %maximum.loop, label %head.done maximum.loop:
%maximum.key = phi i32 [ 0, %query.loop ], [ %maximum.next, %maximum.step ]
%maximum = phi double [ 0xFFF0000000000000, %query.loop ], [ %maximum.value, %maximum.step ]
%maximum.more = icmp ult i32 %maximum.key, %length br i1 %maximum.more, label %maximum.step, label %denominator.loop
maximum.step:
%maximum.score = call double @attention_score( ptr addrspace(1) %context, i32 %plane, i32 %row, i32 %head, i32 %query,
i32 %maximum.key, i32 %from, i32 %length, i32 %head.width, double %scale )
%maximum.larger = fcmp ogt double %maximum.score, %maximum
%maximum.value = select i1 %maximum.larger, double %maximum.score, double %maximum
%maximum.next = add nuw i32 %maximum.key, 1 br label %maximum.loop denominator.loop:
%denominator.key = phi i32 [ 0, %maximum.loop ], [ %denominator.next.key, %denominator.step ]
%denominator = phi double [ 0.0, %maximum.loop ], [ %denominator.next, %denominator.step ]
%denominator.more = icmp ult i32 %denominator.key, %length
br i1 %denominator.more, label %denominator.step, label %mean.loop denominator.step:
%denominator.score = call double @attention_score(
ptr addrspace(1) %context, i32 %plane, i32 %row, i32 %head, i32 %query,
i32 %denominator.key, i32 %from, i32 %length, i32 %head.width, double %scale )
%denominator.centered = fsub double %denominator.score, %maximum
%denominator.exponential = call double @__ocml_exp_f64(double %denominator.centered)
%denominator.next = fadd double %denominator, %denominator.exponential
%denominator.next.key = add nuw i32 %denominator.key, 1 br label %denominator.loop mean.loop:
%mean.key = phi i32 [ 0, %denominator.loop ], [ %mean.next.key, %mean.channel.done ]
%mean = phi double [ 0.0, %denominator.loop ], [ %mean.next, %mean.channel.done ]
%mean.more = icmp ult i32 %mean.key, %length br i1 %mean.more, label %mean.channel.loop, label %key.loop
mean.channel.loop: %mean.channel.offset = phi i32 [ 0, %mean.loop ], [ %mean.channel.next, %mean.channel.step ]
%dp = phi double [ 0.0, %mean.loop ], [ %dp.next, %mean.channel.step ]
%mean.channel.more = icmp ult i32 %mean.channel.offset, %head.width
br i1 %mean.channel.more, label %mean.channel.step, label %mean.channel.done mean.channel.step:
%head.start = mul i32 %head, %head.width %mean.channel = add i32 %head.start, %mean.channel.offset
%row.base = mul i32 %row, %from %mean.channel.base = mul i32 %mean.channel, %length
%mean.delta.local = add i32 %mean.channel.base, %query %mean.value.local = add i32 %mean.channel.base, %mean.key
%mean.delta.index = add i32 %row.base, %mean.delta.local %mean.value.row.index = add i32 %row.base, %mean.value.local
%mean.value.index = add i32 %value.plane, %mean.value.row.index
%mean.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %mean.delta.index
%mean.value.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %mean.value.index
%mean.delta = load double, ptr addrspace(1) %mean.delta.ptr, align 8
%mean.value = load double, ptr addrspace(1) %mean.value.ptr, align 8 %dp.product = fmul double %mean.delta, %mean.value
%dp.next = fadd double %dp, %dp.product %mean.channel.next = add nuw i32 %mean.channel.offset, 1
br label %mean.channel.loop mean.channel.done:
%mean.score = call double @attention_score( ptr addrspace(1) %context, i32 %plane, i32 %row, i32 %head, i32 %query,
i32 %mean.key, i32 %from, i32 %length, i32 %head.width, double %scale )
%mean.centered = fsub double %mean.score, %maximum
%mean.exponential = call double @__ocml_exp_f64(double %mean.centered)
%probability = fdiv double %mean.exponential, %denominator %mean.product = fmul double %probability, %dp
%mean.next = fadd double %mean, %mean.product %mean.next.key = add nuw i32 %mean.key, 1 br label %mean.loop key.loop:
%key = phi i32 [ 0, %mean.loop ], [ %key.next, %key.channel.done ] %key.more = icmp ult i32 %key, %length
%key.head.start = mul i32 %head, %head.width %key.row.base = mul i32 %row, %from
br i1 %key.more, label %key.dp.loop, label %query.done key.dp.loop:
%key.dp.channel = phi i32 [ 0, %key.loop ], [ %key.dp.next, %key.dp.step ]
%key.dp = phi double [ 0.0, %key.loop ], [ %key.dp.sum, %key.dp.step ]
%key.dp.more = icmp ult i32 %key.dp.channel, %head.width br i1 %key.dp.more, label %key.dp.step, label %key.channel.loop
key.dp.step: %key.channel = add i32 %key.head.start, %key.dp.channel
%key.channel.base = mul i32 %key.channel, %length
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
key.channel.loop: %key.channel.offset = phi i32 [ 0, %key.dp.loop ], [ %key.channel.next, %key.channel.step ]
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
%update.delta = load double, ptr addrspace(1) %update.delta.ptr, align 8
%key.score = call double @attention_score( ptr addrspace(1) %context, i32 %plane, i32 %row, i32 %head, i32 %query,
i32 %key, i32 %from, i32 %length, i32 %head.width, double %scale ) %key.centered = fsub double %key.score, %maximum
%key.exponential = call double @__ocml_exp_f64(double %key.centered)
%key.probability = fdiv double %key.exponential, %denominator %key.dp.centered = fsub double %key.dp, %mean
%ds = fmul double %key.probability, %key.dp.centered %dq.raw = fmul double %ds, %key.value.current
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
call void @llvm.amdgcn.s.barrier() %matrix = mul i32 %channels, %channels %parameter.count = mul i32 %matrix, 3
br label %gradient.loop gradient.loop: %gradient.p = phi i32 [ %tid, %rows.done ], [ %gradient.next, %gradient.store ]
%gradient.more = icmp ult i32 %gradient.p, %parameter.count
br i1 %gradient.more, label %gradient.sum.loop, label %previous.loop gradient.sum.loop:
%gradient.item = phi i32 [ 0, %gradient.loop ], [ %gradient.item.next, %gradient.sum.step ]
%gradient.sum = phi double [ 0.0, %gradient.loop ], [ %gradient.sum.next, %gradient.sum.step ]
%gradient.items = mul i32 %rows, %length %gradient.item.more = icmp ult i32 %gradient.item, %gradient.items
br i1 %gradient.item.more, label %gradient.sum.step, label %gradient.store gradient.sum.step:
%projection = udiv i32 %gradient.p, %matrix %matrix.local = urem i32 %gradient.p, %matrix
%output.channel = udiv i32 %matrix.local, %channels %input.channel = urem i32 %matrix.local, %channels
%gradient.row = udiv i32 %gradient.item, %length %gradient.time = urem i32 %gradient.item, %length
%gradient.row.base = mul i32 %gradient.row, %from %gradient.output.base = mul i32 %output.channel, %length
%gradient.output.local = add i32 %gradient.output.base, %gradient.time
%gradient.output.row.index = add i32 %gradient.row.base, %gradient.output.local
%gradient.projection.base.0 = add i32 %projection, 3
%gradient.projection.base = mul i32 %gradient.projection.base.0, %plane
%gradient.projection.index = add i32 %gradient.projection.base, %gradient.output.row.index
%gradient.input.base = mul i32 %input.channel, %length
%gradient.input.local = add i32 %gradient.input.base, %gradient.time
%gradient.input.index = add i32 %gradient.row.base, %gradient.input.local
%gradient.projection.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gradient.projection.index
%gradient.input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %gradient.input.index
%gradient.projection = load double, ptr addrspace(1) %gradient.projection.ptr, align 8
%gradient.input = load double, ptr addrspace(1) %gradient.input.ptr, align 8
%gradient.product = fmul double %gradient.projection, %gradient.input
%gradient.sum.next = fadd double %gradient.sum, %gradient.product %gradient.item.next = add nuw i32 %gradient.item, 1
br label %gradient.sum.loop gradient.store: %gradient.index = add i32 %offset, %gradient.p
%gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.index
store double %gradient.sum, ptr addrspace(1) %gradient.ptr, align 8 %gradient.next = add i32 %gradient.p, %threads
br label %gradient.loop previous.loop:
%previous.p = phi i32 [ %tid, %gradient.loop ], [ %previous.next, %previous.store ]
%previous.count = mul i32 %rows, %from %previous.more.0 = icmp ult i32 %previous.p, %previous.count
%previous.more = and i1 %previous.more.0, %write.previous br i1 %previous.more, label %previous.sum.loop, label %exit
previous.sum.loop: %previous.term = phi i32 [ 0, %previous.loop ], [ %previous.term.next, %previous.sum.step ]
%previous.sum = phi double [ 0.0, %previous.loop ], [ %previous.sum.next, %previous.sum.step ]
%previous.terms = mul i32 %channels, 3 %previous.term.more = icmp ult i32 %previous.term, %previous.terms
br i1 %previous.term.more, label %previous.sum.step, label %previous.store previous.sum.step:
%previous.projection = udiv i32 %previous.term, %channels %previous.output.channel = urem i32 %previous.term, %channels
%previous.row = udiv i32 %previous.p, %from %previous.local = urem i32 %previous.p, %from
%previous.time = urem i32 %previous.local, %length %previous.row.base = mul i32 %previous.row, %from
%previous.output.base = mul i32 %previous.output.channel, %length
%previous.output.local = add i32 %previous.output.base, %previous.time
%previous.output.index.0 = add i32 %previous.row.base, %previous.output.local
%previous.projection.base.0 = add i32 %previous.projection, 3
%previous.projection.base = mul i32 %previous.projection.base.0, %plane
%previous.projection.index = add i32 %previous.projection.base, %previous.output.index.0
%previous.input.channel = udiv i32 %previous.local, %length
%previous.weight.projection.base = mul i32 %previous.projection, %matrix
%previous.weight.output.base = mul i32 %previous.output.channel, %channels
%previous.weight.base = add i32 %previous.weight.projection.base, %previous.weight.output.base
%previous.weight.index = add i32 %previous.weight.base, %previous.input.channel
%previous.projection.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %previous.projection.index
%previous.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %previous.weight.index
%previous.projection.value = load double, ptr addrspace(1) %previous.projection.ptr, align 8
%previous.weight = load double, ptr addrspace(1) %previous.weight.ptr, align 8
%previous.product = fmul double %previous.projection.value, %previous.weight
%previous.sum.next = fadd double %previous.sum, %previous.product %previous.term.next = add nuw i32 %previous.term, 1
br label %previous.sum.loop previous.store:
%previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %previous.p
%previous.old = load double, ptr addrspace(1) %previous.ptr, align 8
%previous.value = fadd double %previous.old, %previous.sum
store double %previous.value, ptr addrspace(1) %previous.ptr, align 8 %previous.next = add i32 %previous.p, %threads
br label %previous.loop exit: ret void }
define internal void @scan_forward_body( ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output,
ptr addrspace(1) %context, i32 %rows, i32 %in.channels, i32 %length, i32 %out.channels, i32 %gates,
i32 %threads ) #1 { entry: %tid = call i32 @llvm.amdgcn.workitem.id.x()
%in.elements = mul i32 %in.channels, %length
%out.elements = mul i32 %out.channels, %length %input.matrix = mul i32 %in.channels, %out.channels
%state.matrix = mul i32 %out.channels, %out.channels %matrix.span = add i32 %input.matrix, %state.matrix
%gate.stride = add i32 %matrix.span, %out.channels %gate.batch = mul i32 %rows, %out.elements br label %row.loop
row.loop: %row = phi i32 [ %tid, %entry ], [ %row.next, %time.done ] %row.more = icmp ult i32 %row, %rows
br i1 %row.more, label %time.loop, label %exit time.loop: %time = phi i32 [ 0, %row.loop ], [ %time.next, %output.done ]
%previous.exists = icmp ne i32 %time, 0 %output.row.base = mul i32 %row, %out.elements
%time.more = icmp ult i32 %time, %length br i1 %time.more, label %gate.loop, label %time.done gate.loop:
%gate = phi i32 [ 0, %time.loop ], [ %gate.next, %hidden.done ] %gate.more = icmp ult i32 %gate, %gates
br i1 %gate.more, label %hidden.loop, label %output.loop hidden.loop:
%hidden = phi i32 [ 0, %gate.loop ], [ %hidden.next, %gate.store ] %gate.weight.base = mul i32 %gate, %gate.stride
%hidden.more = icmp ult i32 %hidden, %out.channels br i1 %hidden.more, label %input.sum.loop, label %hidden.done
input.sum.loop: %in.channel = phi i32 [ 0, %hidden.loop ], [ %in.next, %input.sum.step ]
%input.sum = phi double [ 0.0, %hidden.loop ], [ %input.sum.next, %input.sum.step ]
%input.more = icmp ult i32 %in.channel, %in.channels br i1 %input.more, label %input.sum.step, label %state.sum.loop
input.sum.step: %input.row.base = mul i32 %row, %in.elements %input.channel.base = mul i32 %in.channel, %length
%input.local = add i32 %input.channel.base, %time %input.index = add i32 %input.row.base, %input.local
%input.weight.base = mul i32 %in.channel, %out.channels %input.weight.local = add i32 %input.weight.base, %hidden
%input.weight.index = add i32 %gate.weight.base, %input.weight.local
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
%input.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %input.weight.index
%input.value = load double, ptr addrspace(1) %input.ptr, align 8
%input.weight = load double, ptr addrspace(1) %input.weight.ptr, align 8
%input.product = fmul double %input.value, %input.weight %input.sum.next = fadd double %input.sum, %input.product
%in.next = add nuw i32 %in.channel, 1 br label %input.sum.loop state.sum.loop:
%state.channel = phi i32 [ 0, %input.sum.loop ], [ %state.next, %state.sum.step ]
%state.sum = phi double [ %input.sum, %input.sum.loop ], [ %state.sum.next, %state.sum.step ]
%state.more = icmp ult i32 %state.channel, %out.channels br i1 %state.more, label %state.sum.step, label %gate.activate
state.sum.step: %previous.time = sub i32 %time, 1 %previous.safe = select i1 %previous.exists, i32 %previous.time, i32 0
%state.channel.base = mul i32 %state.channel, %length %previous.local = add i32 %state.channel.base, %previous.safe
%previous.index = add i32 %output.row.base, %previous.local
%previous.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %previous.index
%previous.loaded = load double, ptr addrspace(1) %previous.ptr, align 8
%previous = select i1 %previous.exists, double %previous.loaded, double 0.0 %candidate.gate = icmp eq i32 %gate, 2
%gru = icmp eq i32 %gates, 3 %reset.candidate = and i1 %gru, %candidate.gate
%reset.base = add i32 %gate.batch, %previous.index
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
%in.elements = mul i32 %in.channels, %in.length
%out.elements = mul i32 %out.channels, %out.length %is.conv = icmp ne i32 %kernel, 0
%span = select i1 %is.conv, i32 %kernel, i32 1 %window = mul i32 %in.channels, %span
%parameter.count = mul i32 %out.channels, %window br label %gradient.loop gradient.loop:
%p = phi i32 [ %tid, %entry ], [ %p.next, %gradient.store ] %p.more = icmp ult i32 %p, %parameter.count
br i1 %p.more, label %gradient.entry, label %previous.loop gradient.entry: %filter = udiv i32 %p, %window
%within = urem i32 %p, %window %channel = udiv i32 %within, %span %kernel.position = urem i32 %within, %span
br label %gradient.sum.loop gradient.sum.loop:
%item = phi i32 [ 0, %gradient.entry ], [ %item.next, %gradient.sum.step ]
%sum = phi double [ 0.0, %gradient.entry ], [ %sum.next, %gradient.sum.step ] %items = mul i32 %rows, %out.length
%item.more = icmp ult i32 %item, %items br i1 %item.more, label %gradient.sum.step, label %gradient.store
gradient.sum.step: %row = udiv i32 %item, %out.length %position = urem i32 %item, %out.length
%input.row.base = mul i32 %row, %in.elements %input.channel.base = mul i32 %channel, %in.length
%input.base = add i32 %input.row.base, %input.channel.base %input.position = add i32 %position, %kernel.position
%input.index = add i32 %input.base, %input.position %delta.row.base = mul i32 %row, %out.elements
%delta.filter.base = mul i32 %filter, %out.length %delta.base = add i32 %delta.row.base, %delta.filter.base
%delta.index = add i32 %delta.base, %position
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
%delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %delta.index
%input.value = load double, ptr addrspace(1) %input.ptr, align 8
%delta.value = load double, ptr addrspace(1) %delta.ptr, align 8 %product = fmul double %input.value, %delta.value
%sum.next = fadd double %sum, %product %item.next = add nuw i32 %item, 1 br label %gradient.sum.loop gradient.store:
%gradient.index = add i32 %offset, %p
%gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.index
store double %sum, ptr addrspace(1) %gradient.ptr, align 8 %p.next = add i32 %p, %threads br label %gradient.loop
previous.loop: %previous.p = phi i32 [ %tid, %gradient.loop ], [ %previous.next, %previous.done ]
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
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%in.elements = mul i32 %in.channels, %length
%out.elements = mul i32 %out.channels, %length %batch = mul i32 %rows, %out.elements
%gate.stride.0 = mul i32 %in.channels, %out.channels %state.matrix = mul i32 %out.channels, %out.channels
%gate.stride.1 = add i32 %gate.stride.0, %state.matrix %gate.stride = add i32 %gate.stride.1, %out.channels
%delta.base.factor = add i32 %gates, 1 %delta.base = mul i32 %delta.base.factor, %batch
%row.gradient.factor = mul i32 %gates, 2 %row.gradient.factor.1 = add i32 %row.gradient.factor, 1
%row.gradient.base = mul i32 %row.gradient.factor.1, %batch %lstm = icmp eq i32 %gates, 4
br i1 %lstm, label %row.loop, label %invalid row.loop: %row = phi i32 [ %tid, %entry ], [ %row.next, %row.done ]
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
br i1 %time.more, label %gate.delta.loop, label %row.done gate.delta.loop:
%hidden = phi i32 [ 0, %time.loop ], [ %hidden.next, %gate.delta.step ]
%hidden.more = icmp ult i32 %hidden, %out.channels br i1 %hidden.more, label %gate.delta.step, label %parameter.loop
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
%hidden.next = add nuw i32 %hidden, 1 br label %gate.delta.loop parameter.loop:
%p = phi i32 [ 0, %gate.delta.loop ], [ %p.next, %parameter.step ] %p.more = icmp ult i32 %p, %parameters
br i1 %p.more, label %parameter.step, label %input.gradient.loop parameter.step: %gate = udiv i32 %p, %gate.stride
%within = urem i32 %p, %gate.stride %input.weight = icmp ult i32 %within, %gate.stride.0
%state.end = add i32 %gate.stride.0, %state.matrix %state.weight = icmp ult i32 %within, %state.end
%matrix.weight = or i1 %input.weight, %state.weight %matrix.index = select i1 %input.weight, i32 %within, i32 0
%state.index = sub i32 %within, %gate.stride.0
%selected.raw = select i1 %input.weight, i32 %matrix.index, i32 %state.index
%selected.index = select i1 %matrix.weight, i32 %selected.raw, i32 0
%source.channel = udiv i32 %selected.index, %out.channels %target.hidden = urem i32 %selected.index, %out.channels
%bias.hidden = sub i32 %within, %state.end
%delta.hidden = select i1 %matrix.weight, i32 %target.hidden, i32 %bias.hidden
%delta.hidden.base = mul i32 %delta.hidden, %length %delta.local = add i32 %delta.hidden.base, %time.current
%delta.row.local = add i32 %row.output.base, %delta.local %delta.gate.base = mul i32 %gate, %batch
%delta.gate.local = add i32 %delta.base, %delta.gate.base %delta.index = add i32 %delta.gate.local, %delta.row.local
%gate.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %delta.index
%gate.delta = load double, ptr addrspace(1) %gate.delta.ptr, align 8
%input.channel.base = mul i32 %source.channel, %length %input.local = add i32 %input.channel.base, %time.current
%input.index = add i32 %input.row.base, %input.local
%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
%input.value = load double, ptr addrspace(1) %input.ptr, align 8 %state.hidden.base = mul i32 %source.channel, %length
%state.local = add i32 %state.hidden.base, %previous.safe %state.index.value = add i32 %row.output.base, %state.local
%state.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %state.index.value
%state.loaded = load double, ptr addrspace(1) %state.ptr, align 8
%state.value = select i1 %previous.exists, double %state.loaded, double 0.0
%matrix.source = select i1 %input.weight, double %input.value, double %state.value
%source.value = select i1 %matrix.weight, double %matrix.source, double 1.0
%contribution = fmul double %source.value, %gate.delta %row.gradient.index = add i32 %row.gradient.start, %p
%row.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %row.gradient.index
%row.gradient.old = load double, ptr addrspace(1) %row.gradient.ptr, align 8
%row.gradient.new = fadd double %row.gradient.old, %contribution
store double %row.gradient.new, ptr addrspace(1) %row.gradient.ptr, align 8 %p.next = add nuw i32 %p, 1
br label %parameter.loop input.gradient.loop:
%input.channel = phi i32 [ 0, %parameter.loop ], [ %input.channel.next, %input.gradient.next ]
%input.channel.more = icmp ult i32 %input.channel, %in.channels
br i1 %input.channel.more, label %input.gradient.sum.loop, label %hidden.gradient.loop input.gradient.sum.loop:
%input.term = phi i32 [ 0, %input.gradient.loop ], [ %input.term.next, %input.gradient.sum.step ]
%input.sum = phi double [ 0.0, %input.gradient.loop ], [ %input.sum.next, %input.gradient.sum.step ]
%input.terms = mul i32 %gates, %out.channels %input.term.more = icmp ult i32 %input.term, %input.terms
br i1 %input.term.more, label %input.gradient.sum.step, label %input.gradient.store input.gradient.sum.step:
%input.gate = udiv i32 %input.term, %out.channels %input.hidden = urem i32 %input.term, %out.channels
%input.gate.base = mul i32 %input.gate, %gate.stride %input.weight.row = mul i32 %input.channel, %out.channels
%input.weight.local = add i32 %input.weight.row, %input.hidden
%input.weight.index = add i32 %input.gate.base, %input.weight.local
%input.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %input.weight.index
%input.delta.hidden.base = mul i32 %input.hidden, %length
%input.delta.local = add i32 %input.delta.hidden.base, %time.current
%input.delta.row = add i32 %row.output.base, %input.delta.local %input.delta.gate.base = mul i32 %input.gate, %batch
%input.delta.base = add i32 %delta.base, %input.delta.gate.base
%input.delta.index = add i32 %input.delta.base, %input.delta.row
%input.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %input.delta.index
%input.weight.value = load double, ptr addrspace(1) %input.weight.ptr, align 8
%input.delta.value = load double, ptr addrspace(1) %input.delta.ptr, align 8
%input.product = fmul double %input.weight.value, %input.delta.value
%input.sum.next = fadd double %input.sum, %input.product %input.term.next = add nuw i32 %input.term, 1
br label %input.gradient.sum.loop input.gradient.store:
br i1 %write.input, label %input.gradient.add, label %input.gradient.next input.gradient.add:
%input.previous.channel.base = mul i32 %input.channel, %length
%input.previous.local = add i32 %input.previous.channel.base, %time.current
%input.previous.index = add i32 %input.row.base, %input.previous.local
%input.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %input.previous.index
%input.previous.old = load double, ptr addrspace(1) %input.previous.ptr, align 8
%input.previous.new = fadd double %input.previous.old, %input.sum
store double %input.previous.new, ptr addrspace(1) %input.previous.ptr, align 8 br label %input.gradient.next
input.gradient.next: %input.channel.next = add nuw i32 %input.channel, 1 br label %input.gradient.loop
hidden.gradient.loop:
%state.channel = phi i32 [ 0, %input.gradient.loop ], [ %state.channel.next, %hidden.gradient.store ]
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
%state.product = fmul double %state.weight.value, %state.delta.value
%state.sum.next = fadd double %state.sum, %state.product %state.term.next = add nuw i32 %state.term, 1
br label %hidden.gradient.sum.loop hidden.gradient.store: %state.dh.index = add i32 %dh.start, %state.channel
%state.dh.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %state.dh.index
store double %state.sum, ptr addrspace(1) %state.dh.ptr, align 8 %state.channel.next = add nuw i32 %state.channel, 1
br label %hidden.gradient.loop time.done: br label %time.loop row.done: %row.next = add i32 %row, %threads
br label %row.loop reduce.entry: call void @llvm.amdgcn.s.barrier() br label %reduce.loop reduce.loop:
%reduce.p = phi i32 [ %tid, %reduce.entry ], [ %reduce.next, %reduce.store ]
%reduce.more = icmp ult i32 %reduce.p, %parameters br i1 %reduce.more, label %reduce.row.loop, label %exit
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
br label %reduce.loop invalid: call void @llvm.trap() br label %exit exit: ret void }
define internal void @estimator_forward_body( ptr addrspace(1) %input, ptr addrspace(1) %state,
ptr addrspace(1) %output, ptr addrspace(1) %context, i32 %p, i32 %features, i32 %kind, i32 %argument,
i32 %training.rows ) #1 { entry: %row.base = mul i32 %p, %features
%query = getelementptr inbounds double, ptr addrspace(1) %input, i32 %row.base
%kmeans = icmp eq i32 %kind, 0 br i1 %kmeans, label %cluster.loop, label %knn.initialize.loop cluster.loop:
%cluster = phi i32 [ 0, %entry ], [ %cluster.next, %cluster.step ]
%best = phi i32 [ 0, %entry ], [ %best.next, %cluster.step ]
%best.distance = phi double [ 0x7FF0000000000000, %entry ], [ %best.distance.next, %cluster.step ]
%cluster.more = icmp ult i32 %cluster, %argument br i1 %cluster.more, label %cluster.step, label %cluster.done
cluster.step: %centroid.offset = mul i32 %cluster, %features
%centroid = getelementptr inbounds double, ptr addrspace(1) %state, i32 %centroid.offset
%candidate = call double @distance( ptr addrspace(1) %query, ptr addrspace(1) %centroid, i32 %features )
%better = fcmp olt double %candidate, %best.distance
%best.next = select i1 %better, i32 %cluster, i32 %best
%best.distance.next = select i1 %better, double %candidate, double %best.distance
%cluster.next = add nuw i32 %cluster, 1 br label %cluster.loop cluster.done:
%cluster.value = uitofp i32 %best to double br label %store knn.initialize.loop:
%slot = phi i32 [ 0, %entry ], [ %slot.next, %knn.initialize.step ]
%context.base = mul i32 %p, %argument %context.double.base = mul i32 %context.base, 2
%target.base = add i32 %context.double.base, %argument
%slot.more = icmp ult i32 %slot, %argument
br i1 %slot.more, label %knn.initialize.step, label %knn.training.loop knn.initialize.step:
%distance.index = add i32 %context.double.base, %slot
%distance.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %distance.index
store double 0x7FF0000000000000, ptr addrspace(1) %distance.ptr, align 8
%slot.next = add nuw i32 %slot, 1 br label %knn.initialize.loop knn.training.loop:
%training.row = phi i32 [ 0, %knn.initialize.loop ], [ %training.next, %knn.training.next ]
%training.more = icmp ult i32 %training.row, %training.rows
br i1 %training.more, label %knn.training.test, label %knn.average.loop knn.training.test:
%self = icmp eq i32 %training.row, %p br i1 %self, label %knn.training.next, label %knn.distance
knn.distance: %sample.offset = mul i32 %training.row, %features
%sample = getelementptr inbounds double, ptr addrspace(1) %state, i32 %sample.offset
%knn.candidate = call double @distance( ptr addrspace(1) %query, ptr addrspace(1) %sample, i32 %features )
br label %knn.worst.loop knn.worst.loop:
%worst.slot = phi i32 [ 0, %knn.distance ], [ %worst.next, %knn.worst.step ]
%worst.index = phi i32 [ 0, %knn.distance ], [ %worst.index.next, %knn.worst.step ]
%worst.distance = phi double [ -1.0, %knn.distance ], [ %worst.distance.next, %knn.worst.step ]
%worst.more = icmp ult i32 %worst.slot, %argument
br i1 %worst.more, label %knn.worst.step, label %knn.replace knn.worst.step:
%worst.local = add i32 %context.double.base, %worst.slot
%worst.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %worst.local
%worst.candidate = load double, ptr addrspace(1) %worst.ptr, align 8
%worse = fcmp ogt double %worst.candidate, %worst.distance
%worst.index.next = select i1 %worse, i32 %worst.slot, i32 %worst.index
%worst.distance.next = select i1 %worse, double %worst.candidate, double %worst.distance
%worst.next = add nuw i32 %worst.slot, 1 br label %knn.worst.loop knn.replace:
%replace = fcmp olt double %knn.candidate, %worst.distance
br i1 %replace, label %knn.replace.store, label %knn.training.next knn.replace.store:
%replace.distance.index = add i32 %context.double.base, %worst.index
%replace.target.index = add i32 %target.base, %worst.index
%replace.distance.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %replace.distance.index
%replace.target.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %replace.target.index
%state.targets.base = mul i32 %training.rows, %features
%state.target.index = add i32 %state.targets.base, %training.row
%state.target.ptr = getelementptr inbounds double, ptr addrspace(1) %state, i32 %state.target.index
%state.target = load double, ptr addrspace(1) %state.target.ptr, align 8
store double %knn.candidate, ptr addrspace(1) %replace.distance.ptr, align 8
store double %state.target, ptr addrspace(1) %replace.target.ptr, align 8 br label %knn.training.next
knn.training.next: %training.next = add nuw i32 %training.row, 1 br label %knn.training.loop knn.average.loop:
%average.slot = phi i32 [ 0, %knn.training.loop ], [ %average.next, %knn.average.step ]
%sum = phi double [ 0.0, %knn.training.loop ], [ %sum.next, %knn.average.step ]
%average.more = icmp ult i32 %average.slot, %argument
br i1 %average.more, label %knn.average.step, label %knn.average.done knn.average.step:
%average.index = add i32 %target.base, %average.slot
%average.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %average.index
%average.value = load double, ptr addrspace(1) %average.ptr, align 8
%sum.next = fadd double %sum, %average.value %average.next = add nuw i32 %average.slot, 1
br label %knn.average.loop knn.average.done: %argument.double = uitofp i32 %argument to double
%knn.value = fdiv double %sum, %argument.double br label %store store:
%value = phi double [ %cluster.value, %cluster.done ], [ %knn.value, %knn.average.done ]
%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
store double %value, ptr addrspace(1) %output.ptr, align 8 ret void }
define internal void @tape_forward_body(
ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %value.pointers,
ptr addrspace(1) %context.pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %arguments,
i32 %rows, i32 %nodes, i32 %threads ) #1 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x() br label %node.loop
node.loop: %node = phi i32 [ 0, %entry ], [ %node.next, %node.done ] %node.more = icmp ult i32 %node, %nodes
br i1 %node.more, label %node.load, label %exit node.load: %base = mul i32 %node, 11
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
%argument.second.index = add i32 %argument.base, 1
%argument.third.index = add i32 %argument.base, 2
%argument.ptr = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %argument.base
%argument.second.ptr = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %argument.second.index
%argument.third.ptr = getelementptr inbounds double, ptr addrspace(1) %arguments, i32 %argument.third.index
%argument = load double, ptr addrspace(1) %argument.ptr, align 8
%argument.second = load double, ptr addrspace(1) %argument.second.ptr, align 8
%argument.third = load double, ptr addrspace(1) %argument.third.ptr, align 8
%matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %offset
%in.elements = mul i32 %in.channels, %in.length %out.elements = mul i32 %out.channels, %out.length
%count = mul i32 %rows, %out.elements %is.attention = icmp eq i32 %op, 4 %is.scan = icmp eq i32 %op, 5
%is.normalize = icmp eq i32 %op, 8 %is.estimator = icmp eq i32 %op, 9
br i1 %is.attention, label %attention.node, label %scan.test attention.node:
%heads = fptoui double %argument to i32
call void @attention_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values,
ptr addrspace(1) %context, i32 %rows, i32 %in.elements, i32 %heads, i32 %in.channels, i32 %threads )
br label %node.done
scan.test: br i1 %is.scan, label %scan.node, label %normalize.stats.entry scan.node:
%scan.gates = fptoui double %argument to i32
call void @scan_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values,
ptr addrspace(1) %context, i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels,
i32 %scan.gates, i32 %threads ) br label %node.done normalize.stats.entry:
br i1 %is.estimator, label %estimator.loop, label %normalize.test estimator.loop:
%estimator.p = phi i32 [ %tid, %normalize.stats.entry ], [ %estimator.next, %estimator.step ]
%estimator.more = icmp ult i32 %estimator.p, %rows
br i1 %estimator.more, label %estimator.step, label %node.done estimator.step:
%estimator.kind = fptoui double %argument to i32
%estimator.argument = fptoui double %argument.second to i32
%estimator.training.rows = fptoui double %argument.third to i32
call void @estimator_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix,
ptr addrspace(1) %values, ptr addrspace(1) %context, i32 %estimator.p, i32 %in.elements,
i32 %estimator.kind, i32 %estimator.argument, i32 %estimator.training.rows )
%estimator.next = add i32 %estimator.p, %threads br label %estimator.loop normalize.test:
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
switch i32 %op, label %invalid [ i32 0, label %contraction i32 2, label %pool i32 3, label %gather
i32 6, label %elementwise i32 8, label %normalize ] contraction: %kernel = fptoui double %argument to i32
call void @contraction_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values,
i32 %p, i32 %in.channels, i32 %in.length, i32 %out.channels, i32 %out.length, i32 %kernel ) br label %element.done pool:
%size = fptoui double %argument to i32
call void @pool_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %values, ptr addrspace(1) %context, i32 %p,
i32 %in.elements, i32 %out.elements, i32 %size, i32 %in.channels ) br label %element.done gather:
%vocabulary = fptoui double %argument to i32
call void @embedding_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values, i32 %p,
i32 %in.elements, i32 %out.elements, i32 %vocabulary ) br label %element.done elementwise:
call void @scalar_forward_body( ptr addrspace(1) %source, ptr addrspace(1) %second, ptr addrspace(1) %values,
ptr addrspace(1) %context, ptr addrspace(1) %program, ptr addrspace(1) %matrix, i32 %p, i32 %count, i32 %program.count )
br label %element.done normalize: %normalize.row = udiv i32 %p, %out.elements
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
%node.next = add nuw i32 %node, 1 br label %node.loop invalid: call void @llvm.trap() br label %exit exit: ret void }
define protected amdgpu_kernel void @forward_graph(
ptr addrspace(1) nocapture readonly %samples, ptr addrspace(1) nocapture readonly %weights,
ptr addrspace(1) nocapture readonly %value_pointers, ptr addrspace(1) nocapture readonly %context_pointers,
ptr addrspace(1) nocapture readonly %descriptors, ptr addrspace(1) nocapture readonly %parameters,
i32 %rows, i32 %stages, i32 %threads ) #0 { entry:
call void @tape_forward_body( ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %value_pointers,
ptr addrspace(1) %context_pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %parameters,
i32 %rows, i32 %stages, i32 %threads ) ret void }
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
ptr addrspace(1) %samples, ptr addrspace(1) %targets, ptr addrspace(1) %weights,
ptr addrspace(1) %frozen, ptr addrspace(1) %best, ptr addrspace(1) %value.pointers,
ptr addrspace(1) %context.pointers,
ptr addrspace(1) %adjoint.pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %arguments,
ptr addrspace(1) %metrics, ptr addrspace(1) %gradient, ptr addrspace(1) %moments,
ptr addrspace(1) %variances, ptr addrspace(1) %best.loss, i32 %rows, i32 %nodes, i32 %parameter.count, i32 %loss.code,
double %huber.threshold, double %rate, double %beta1, double %beta2,
double %beta1.power, double %beta2.power, double %epsilon, double %decay, double %tolerance, i32 %step,
i32 %threads ) #0 { entry: %tid = call i32 @llvm.amdgcn.workitem.id.x()
call void @tape_forward_body( ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %value.pointers,
ptr addrspace(1) %context.pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %arguments,
i32 %rows, i32 %nodes, i32 %threads ) call void @llvm.amdgcn.s.barrier()
%last = sub i32 %nodes, 1
%last.slot = getelementptr inbounds i64, ptr addrspace(1) %value.pointers, i32 %last
%last.address = load i64, ptr addrspace(1) %last.slot, align 8
%predictions = inttoptr i64 %last.address to ptr addrspace(1) %leader = icmp eq i32 %tid, 0
br i1 %leader, label %loss.loop, label %loss.done loss.loop: %loss.p = phi i32 [ 0, %entry ], [ %loss.next, %loss.step ]
%loss.sum = phi double [ 0.0, %entry ], [ %loss.sum.next, %loss.step ] %loss.more = icmp ult i32 %loss.p, %rows
br i1 %loss.more, label %loss.step, label %loss.store loss.step:
%loss.prediction.ptr = getelementptr inbounds double, ptr addrspace(1) %predictions, i32 %loss.p
%loss.target.ptr = getelementptr inbounds double, ptr addrspace(1) %targets, i32 %loss.p
%loss.prediction = load double, ptr addrspace(1) %loss.prediction.ptr, align 8
%loss.target = load double, ptr addrspace(1) %loss.target.ptr, align 8
%loss.item = call double @loss_item( double %loss.prediction, double %loss.target, i32 %loss.code,
double %huber.threshold ) %loss.sum.next = fadd double %loss.sum, %loss.item %loss.next = add nuw i32 %loss.p, 1
br label %loss.loop loss.store: %rows.double = uitofp i32 %rows to double
%loss.mean = fdiv double %loss.sum, %rows.double %loss.root = call double @llvm.sqrt.f64(double %loss.mean)
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
br i1 %gradient.more, label %gradient.step, label %clear.node.entry gradient.step:
%gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %gradient.p
store double 0.0, ptr addrspace(1) %gradient.ptr, align 8 %gradient.next = add i32 %gradient.p, %threads
br label %clear.gradient.loop clear.node.entry: call void @llvm.amdgcn.s.barrier() br label %clear.node.loop
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
%seed.p = phi i32 [ %tid, %clear.node.loop ], [ %seed.next, %seed.step ] %seed.more = icmp ult i32 %seed.p, %rows
br i1 %seed.more, label %seed.step, label %reverse.entry seed.step:
%seed.prediction.ptr = getelementptr inbounds double, ptr addrspace(1) %predictions, i32 %seed.p
%seed.target.ptr = getelementptr inbounds double, ptr addrspace(1) %targets, i32 %seed.p
%seed.prediction = load double, ptr addrspace(1) %seed.prediction.ptr, align 8
%seed.target = load double, ptr addrspace(1) %seed.target.ptr, align 8
%seed.loss = load double, ptr addrspace(1) %metrics, align 8
%seed.value = call double @loss_gradient( double %seed.prediction, double %seed.target, i32 %loss.code,
double %huber.threshold, double %seed.loss, i32 %rows )
%last.adjoint.slot = getelementptr inbounds i64, ptr addrspace(1) %adjoint.pointers, i32 %last
%last.adjoint.address = load i64, ptr addrspace(1) %last.adjoint.slot, align 8
%last.adjoint = inttoptr i64 %last.adjoint.address to ptr addrspace(1)
%seed.ptr = getelementptr inbounds double, ptr addrspace(1) %last.adjoint, i32 %seed.p
store double %seed.value, ptr addrspace(1) %seed.ptr, align 8 %seed.next = add i32 %seed.p, %threads br label %seed.loop
reverse.entry: call void @llvm.amdgcn.s.barrier() %reverse.first = sub i32 %nodes, 1 br label %reverse.loop
reverse.loop: %node = phi i32 [ %reverse.first, %reverse.entry ], [ %node.next, %node.done ]
%node.more = icmp sge i32 %node, 0 br i1 %node.more, label %node.load, label %optimizer.loop node.load:
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
%source.adjoint = inttoptr i64 %source.adjoint.address to ptr addrspace(1)
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
i32 8, label %normalization.stats.loop i32 9, label %node.done ] contraction.gradient:
%contraction.matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %offset
%contraction.kernel = fptoui double %argument to i32 call void @contraction_reverse_body(
ptr addrspace(1) %source.values, ptr addrspace(1) %contraction.matrix, ptr addrspace(1) %delta,
ptr addrspace(1) %source.adjoint, ptr addrspace(1) %gradient, i1 %source.exists,
i32 %rows, i32 %in.channels, i32 %in.length, i32 %out.channels,
i32 %out.length, i32 %contraction.kernel, i32 %offset, i32 %threads ) br label %node.done gather.gradient:
%gather.vocabulary = fptoui double %argument to i32
call void @embedding_reverse_body( ptr addrspace(1) %source.values, ptr addrspace(1) %delta, ptr addrspace(1) %gradient,
i32 %rows, i32 %in.elements, i32 %out.channels, i32 %gather.vocabulary, i32 %offset, i32 %threads )
br label %node.done
attention.gradient: %attention.context.slot = getelementptr inbounds i64, ptr addrspace(1) %context.pointers, i32 %node
%attention.context.address = load i64, ptr addrspace(1) %attention.context.slot, align 8
%attention.context = inttoptr i64 %attention.context.address to ptr addrspace(1)
%attention.matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %offset
%attention.heads = fptoui double %argument to i32
call void @attention_reverse_body( ptr addrspace(1) %source.values, ptr addrspace(1) %attention.matrix,
ptr addrspace(1) %attention.context, ptr addrspace(1) %delta,
ptr addrspace(1) %source.adjoint, ptr addrspace(1) %gradient, i1 %source.exists,
i32 %rows, i32 %in.elements, i32 %attention.heads, i32 %in.channels, i32 %offset, i32 %threads )
br label %node.done
scan.gradient: %scan.context.slot = getelementptr inbounds i64, ptr addrspace(1) %context.pointers, i32 %node
%scan.context.address = load i64, ptr addrspace(1) %scan.context.slot, align 8
%scan.context = inttoptr i64 %scan.context.address to ptr addrspace(1)
%scan.matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %offset
%scan.gates = fptoui double %argument to i32 call void @scan_reverse_body(
ptr addrspace(1) %source.values, ptr addrspace(1) %scan.matrix, ptr addrspace(1) %node.values,
ptr addrspace(1) %scan.context, ptr addrspace(1) %delta, ptr addrspace(1) %source.adjoint,
ptr addrspace(1) %gradient, i1 %source.exists, i32 %rows, i32 %in.channels,
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
%pool.delta = load double, ptr addrspace(1) %pool.delta.ptr, align 8
br i1 %source.exists, label %pool.gradient.add, label %pool.gradient.next pool.gradient.add:
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
i1 %source.exists, i1 %second.exists, i32 %threads ) br label %node.done normalization.stats.loop:
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
normalization.stats.sum.loop:
%normalization.stats.item = phi i32 [ 0, %normalization.stats.loop ],
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
call void @llvm.amdgcn.s.barrier() %node.next = sub i32 %node, 1 br label %reverse.loop optimizer.loop:
%optimizer.p = phi i32 [ %tid, %reverse.loop ], [ %optimizer.next, %optimizer.advance ]
%optimizer.more = icmp ult i32 %optimizer.p, %parameter.count br i1 %optimizer.more, label %optimizer.step, label %exit
optimizer.step: %optimizer.frozen.ptr = getelementptr inbounds i8, ptr addrspace(1) %frozen, i32 %optimizer.p
%optimizer.frozen = load i8, ptr addrspace(1) %optimizer.frozen.ptr, align 1
%optimizer.is.frozen = icmp ne i8 %optimizer.frozen, 0
br i1 %optimizer.is.frozen, label %optimizer.advance, label %optimizer.update optimizer.update:
%optimizer.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %optimizer.p
%optimizer.moment.ptr = getelementptr inbounds double, ptr addrspace(1) %moments, i32 %optimizer.p
%optimizer.variance.ptr = getelementptr inbounds double, ptr addrspace(1) %variances, i32 %optimizer.p
%optimizer.weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %optimizer.p
%g = load double, ptr addrspace(1) %optimizer.gradient.ptr, align 8
%m.old = load double, ptr addrspace(1) %optimizer.moment.ptr, align 8
%v.old = load double, ptr addrspace(1) %optimizer.variance.ptr, align 8
%weight = load double, ptr addrspace(1) %optimizer.weight.ptr, align 8 %one.beta1 = fsub double 1.0, %beta1
%one.beta2 = fsub double 1.0, %beta2 %m.old.part = fmul double %beta1, %m.old %m.new.part = fmul double %one.beta1, %g
%m = fadd double %m.old.part, %m.new.part %g.square = fmul double %g, %g %v.old.part = fmul double %beta2, %v.old
%v.new.part = fmul double %one.beta2, %g.square %v = fadd double %v.old.part, %v.new.part
store double %m, ptr addrspace(1) %optimizer.moment.ptr, align 8
store double %v, ptr addrspace(1) %optimizer.variance.ptr, align 8 %m.correction = fsub double 1.0, %beta1.power
%v.correction = fsub double 1.0, %beta2.power %m.hat = fdiv double %m, %m.correction
%v.hat = fdiv double %v, %v.correction %root = call double @llvm.sqrt.f64(double %v.hat)
%denominator = fadd double %root, %epsilon %direction = fdiv double %m.hat, %denominator
%decay.value = fmul double %decay, %weight %update.direction = fadd double %direction, %decay.value
%update = fmul double %rate, %update.direction %updated = fsub double %weight, %update
store double %updated, ptr addrspace(1) %optimizer.weight.ptr, align 8 br label %optimizer.advance optimizer.advance:
%optimizer.next = add i32 %optimizer.p, %threads br label %optimizer.loop
invalid: call void @llvm.trap() br label %exit exit: ret void }
define internal double @distance( ptr addrspace(1) %left, ptr addrspace(1) %right, i32 %features ) #1 { entry:
br label %loop loop: %feature = phi i32 [ 0, %entry ], [ %next, %step ]
%sum = phi double [ 0.0, %entry ], [ %sum.next, %step ] %more = icmp ult i32 %feature, %features
br i1 %more, label %step, label %done step:
%left.ptr = getelementptr inbounds double, ptr addrspace(1) %left, i32 %feature
%right.ptr = getelementptr inbounds double, ptr addrspace(1) %right, i32 %feature
%left.value = load double, ptr addrspace(1) %left.ptr, align 8
%right.value = load double, ptr addrspace(1) %right.ptr, align 8 %difference = fsub double %left.value, %right.value
%square = fmul double %difference, %difference %sum.next = fadd double %sum, %square %next = add nuw i32 %feature, 1
br label %loop done: ret double %sum } define protected amdgpu_kernel void @estimate_graph(
ptr addrspace(1) %samples, ptr addrspace(1) %targets, ptr addrspace(1) %output,
ptr addrspace(1) %workspace, i32 %training.rows, i32 %test.rows, i32 %features,
i32 %operation, i32 %argument, i32 %iterations, i32 %threads ) #0 { entry:
%tid = call i32 @llvm.amdgcn.workitem.id.x()
%kind = and i32 %operation, 1 %fitted.bit = and i32 %operation, 2 %exclude.bit = and i32 %operation, 4
%kmeans = icmp eq i32 %kind, 0 %fitted = icmp ne i32 %fitted.bit, 0 %exclude.self = icmp ne i32 %exclude.bit, 0
br i1 %kmeans, label %kmeans.entry, label %knn.query.loop kmeans.entry:
br i1 %fitted, label %kmeans.predict.loop, label %kmeans.initialize.loop kmeans.initialize.loop:
%initialize.p = phi i32 [ %tid, %kmeans.entry ], [ %initialize.next, %kmeans.initialize.step ]
%centroid.count = mul i32 %argument, %features %initialize.more = icmp ult i32 %initialize.p, %centroid.count
br i1 %initialize.more, label %kmeans.initialize.step, label %kmeans.iteration.entry kmeans.initialize.step:
%initialize.source = getelementptr inbounds double, ptr addrspace(1) %samples, i32 %initialize.p
%initialize.target = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %initialize.p
%initialize.value = load double, ptr addrspace(1) %initialize.source, align 8
store double %initialize.value, ptr addrspace(1) %initialize.target, align 8
%initialize.next = add i32 %initialize.p, %threads br label %kmeans.initialize.loop kmeans.iteration.entry:
call void @llvm.amdgcn.s.barrier() br label %kmeans.iteration.loop kmeans.iteration.loop:
%iteration = phi i32 [ 0, %kmeans.iteration.entry ], [ %iteration.next, %kmeans.update.done ]
%iteration.more = icmp ult i32 %iteration, %iterations
br i1 %iteration.more, label %kmeans.assignment.loop, label %kmeans.predict.loop kmeans.assignment.loop:
%assignment.row = phi i32 [ %tid, %kmeans.iteration.loop ], [ %assignment.next, %kmeans.assignment.store ]
%assignment.more = icmp ult i32 %assignment.row, %training.rows
br i1 %assignment.more, label %kmeans.assignment.entry, label %kmeans.update.entry kmeans.assignment.entry:
%assignment.sample.offset = mul i32 %assignment.row, %features
%assignment.sample = getelementptr inbounds double, ptr addrspace(1) %samples, i32 %assignment.sample.offset
br label %kmeans.cluster.loop kmeans.cluster.loop:
%assignment.cluster = phi i32 [ 0, %kmeans.assignment.entry ], [ %cluster.next, %kmeans.cluster.step ]
%best.cluster = phi i32 [ 0, %kmeans.assignment.entry ], [ %best.cluster.next, %kmeans.cluster.step ]
%best.distance = phi double [ 0x7FF0000000000000, %kmeans.assignment.entry ],
[ %best.distance.next, %kmeans.cluster.step ]
%cluster.more = icmp ult i32 %assignment.cluster, %argument
br i1 %cluster.more, label %kmeans.cluster.step, label %kmeans.assignment.store kmeans.cluster.step:
%cluster.offset = mul i32 %assignment.cluster, %features
%centroid = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %cluster.offset
%cluster.distance = call double @distance( ptr addrspace(1) %assignment.sample, ptr addrspace(1) %centroid,
i32 %features ) %cluster.better = fcmp olt double %cluster.distance, %best.distance
%best.cluster.next = select i1 %cluster.better, i32 %assignment.cluster, i32 %best.cluster
%best.distance.next = select i1 %cluster.better, double %cluster.distance, double %best.distance
%cluster.next = add nuw i32 %assignment.cluster, 1 br label %kmeans.cluster.loop kmeans.assignment.store:
%assignment.index = add i32 %centroid.count, %assignment.row %distance.base = add i32 %centroid.count, %training.rows
%distance.index = add i32 %distance.base, %assignment.row
%assignment.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %assignment.index
%distance.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %distance.index
%best.cluster.double = uitofp i32 %best.cluster to double
store double %best.cluster.double, ptr addrspace(1) %assignment.ptr, align 8
store double %best.distance, ptr addrspace(1) %distance.ptr, align 8
%assignment.next = add i32 %assignment.row, %threads br label %kmeans.assignment.loop kmeans.update.entry:
call void @llvm.amdgcn.s.barrier() %distance.base.global = add i32 %centroid.count, %training.rows
%update.leader = icmp eq i32 %tid, 0 br i1 %update.leader, label %kmeans.update.cluster.loop, label %kmeans.update.done
kmeans.update.cluster.loop:
%update.cluster = phi i32 [ 0, %kmeans.update.entry ], [ %update.cluster.next, %kmeans.update.cluster.done ]
%update.cluster.more = icmp ult i32 %update.cluster, %argument
br i1 %update.cluster.more, label %kmeans.count.loop, label %kmeans.update.done kmeans.count.loop:
%count.row = phi i32 [ 0, %kmeans.update.cluster.loop ], [ %count.next, %kmeans.count.step ]
%count = phi i32 [ 0, %kmeans.update.cluster.loop ], [ %count.value, %kmeans.count.step ]
%count.more = icmp ult i32 %count.row, %training.rows
br i1 %count.more, label %kmeans.count.step, label %kmeans.count.done kmeans.count.step:
%count.index = add i32 %centroid.count, %count.row
%count.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %count.index
%count.assignment.double = load double, ptr addrspace(1) %count.ptr, align 8
%count.assignment = fptoui double %count.assignment.double to i32
%count.member = icmp eq i32 %count.assignment, %update.cluster %count.increment = zext i1 %count.member to i32
%count.value = add i32 %count, %count.increment %count.next = add nuw i32 %count.row, 1 br label %kmeans.count.loop
kmeans.count.done: %empty = icmp eq i32 %count, 0 br i1 %empty, label %kmeans.worst.loop, label %kmeans.feature.loop
kmeans.worst.loop: %worst.row = phi i32 [ 0, %kmeans.count.done ], [ %worst.next, %kmeans.worst.step ]
%worst.index = phi i32 [ 0, %kmeans.count.done ], [ %worst.index.next, %kmeans.worst.step ]
%worst.distance = phi double [ -1.0, %kmeans.count.done ], [ %worst.distance.next, %kmeans.worst.step ]
%worst.more = icmp ult i32 %worst.row, %training.rows
br i1 %worst.more, label %kmeans.worst.step, label %kmeans.reseed.loop kmeans.worst.step:
%worst.distance.index = add i32 %distance.base.global, %worst.row
%worst.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %worst.distance.index
%worst.candidate = load double, ptr addrspace(1) %worst.ptr, align 8
%worst.better = fcmp ogt double %worst.candidate, %worst.distance
%worst.index.next = select i1 %worst.better, i32 %worst.row, i32 %worst.index
%worst.distance.next = select i1 %worst.better, double %worst.candidate, double %worst.distance
%worst.next = add nuw i32 %worst.row, 1 br label %kmeans.worst.loop kmeans.reseed.loop:
%reseed.feature = phi i32 [ 0, %kmeans.worst.loop ], [ %reseed.next, %kmeans.reseed.step ]
%reseed.more = icmp ult i32 %reseed.feature, %features
br i1 %reseed.more, label %kmeans.reseed.step, label %kmeans.reseed.done kmeans.reseed.step:
%reseed.source.base = mul i32 %worst.index, %features
%reseed.source.index = add i32 %reseed.source.base, %reseed.feature
%reseed.target.base = mul i32 %update.cluster, %features
%reseed.target.index = add i32 %reseed.target.base, %reseed.feature
%reseed.source.ptr = getelementptr inbounds double, ptr addrspace(1) %samples, i32 %reseed.source.index
%reseed.target.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %reseed.target.index
%reseed.value = load double, ptr addrspace(1) %reseed.source.ptr, align 8
store double %reseed.value, ptr addrspace(1) %reseed.target.ptr, align 8 %reseed.next = add nuw i32 %reseed.feature, 1
br label %kmeans.reseed.loop kmeans.reseed.done: %reseed.distance.index = add i32 %distance.base.global, %worst.index
%reseed.distance.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %reseed.distance.index
store double -1.0, ptr addrspace(1) %reseed.distance.ptr, align 8 br label %kmeans.update.cluster.done
kmeans.feature.loop: %update.feature = phi i32 [ 0, %kmeans.count.done ], [ %feature.next, %kmeans.feature.store ]
%feature.more = icmp ult i32 %update.feature, %features
br i1 %feature.more, label %kmeans.sum.loop, label %kmeans.update.cluster.done kmeans.sum.loop:
%sum.row = phi i32 [ 0, %kmeans.feature.loop ], [ %sum.next, %kmeans.sum.step ]
%sum = phi double [ 0.0, %kmeans.feature.loop ], [ %sum.value, %kmeans.sum.step ]
%sum.more = icmp ult i32 %sum.row, %training.rows br i1 %sum.more, label %kmeans.sum.step, label %kmeans.feature.store
kmeans.sum.step: %sum.assignment.index = add i32 %centroid.count, %sum.row
%sum.assignment.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %sum.assignment.index
%sum.assignment.double = load double, ptr addrspace(1) %sum.assignment.ptr, align 8
%sum.assignment = fptoui double %sum.assignment.double to i32 %sum.member = icmp eq i32 %sum.assignment, %update.cluster
%sum.sample.base = mul i32 %sum.row, %features %sum.sample.index = add i32 %sum.sample.base, %update.feature
%sum.sample.ptr = getelementptr inbounds double, ptr addrspace(1) %samples, i32 %sum.sample.index
%sum.sample = load double, ptr addrspace(1) %sum.sample.ptr, align 8
%sum.contribution = select i1 %sum.member, double %sum.sample, double 0.0
%sum.value = fadd double %sum, %sum.contribution %sum.next = add nuw i32 %sum.row, 1 br label %kmeans.sum.loop
kmeans.feature.store: %count.double = uitofp i32 %count to double %mean = fdiv double %sum, %count.double
%mean.base = mul i32 %update.cluster, %features %mean.index = add i32 %mean.base, %update.feature
%mean.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %mean.index
store double %mean, ptr addrspace(1) %mean.ptr, align 8 %feature.next = add nuw i32 %update.feature, 1
br label %kmeans.feature.loop kmeans.update.cluster.done: %update.cluster.next = add nuw i32 %update.cluster, 1
br label %kmeans.update.cluster.loop kmeans.update.done: call void @llvm.amdgcn.s.barrier()
%iteration.next = add nuw i32 %iteration, 1 br label %kmeans.iteration.loop kmeans.predict.loop:
%predict.row = phi i32 [ %tid, %kmeans.entry ], [ %tid, %kmeans.iteration.loop ],
[ %predict.next, %kmeans.predict.store ] %predict.more = icmp ult i32 %predict.row, %test.rows
br i1 %predict.more, label %kmeans.predict.entry, label %exit kmeans.predict.entry:
%predict.global.row = add i32 %training.rows, %predict.row
%predict.sample.offset = mul i32 %predict.global.row, %features
%predict.sample = getelementptr inbounds double, ptr addrspace(1) %samples, i32 %predict.sample.offset
br label %kmeans.predict.cluster.loop kmeans.predict.cluster.loop:
%predict.cluster = phi i32 [ 0, %kmeans.predict.entry ], [ %predict.cluster.next, %kmeans.predict.cluster.step ]
%predict.best = phi i32 [ 0, %kmeans.predict.entry ], [ %predict.best.next, %kmeans.predict.cluster.step ]
%predict.distance = phi double [ 0x7FF0000000000000, %kmeans.predict.entry ],
[ %predict.distance.next, %kmeans.predict.cluster.step ]
%predict.cluster.more = icmp ult i32 %predict.cluster, %argument
br i1 %predict.cluster.more, label %kmeans.predict.cluster.step, label %kmeans.predict.store
kmeans.predict.cluster.step: %predict.centroid.offset = mul i32 %predict.cluster, %features
%predict.centroid = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %predict.centroid.offset
%predict.candidate = call double @distance( ptr addrspace(1) %predict.sample, ptr addrspace(1) %predict.centroid,
i32 %features ) %predict.better = fcmp olt double %predict.candidate, %predict.distance
%predict.best.next = select i1 %predict.better, i32 %predict.cluster, i32 %predict.best
%predict.distance.next = select i1 %predict.better, double %predict.candidate, double %predict.distance
%predict.cluster.next = add nuw i32 %predict.cluster, 1 br label %kmeans.predict.cluster.loop kmeans.predict.store:
%predict.output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %predict.row
%predict.value = uitofp i32 %predict.best to double
store double %predict.value, ptr addrspace(1) %predict.output.ptr, align 8
%predict.next = add i32 %predict.row, %threads br label %kmeans.predict.loop knn.query.loop:
%query = phi i32 [ %tid, %entry ], [ %query.next, %knn.store ] %query.more = icmp ult i32 %query, %test.rows
br i1 %query.more, label %knn.initialize.loop, label %exit knn.initialize.loop:
%knn.slot = phi i32 [ 0, %knn.query.loop ], [ %knn.initialize.next, %knn.initialize.step ]
%knn.base = mul i32 %query, %argument %knn.double.base = mul i32 %knn.base, 2
%knn.target.base = add i32 %knn.double.base, %argument %knn.initialize.more = icmp ult i32 %knn.slot, %argument
br i1 %knn.initialize.more, label %knn.initialize.step, label %knn.training.loop knn.initialize.step:
%knn.distance.index = add i32 %knn.double.base, %knn.slot
%knn.distance.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %knn.distance.index
store double 0x7FF0000000000000, ptr addrspace(1) %knn.distance.ptr, align 8
%knn.initialize.next = add nuw i32 %knn.slot, 1 br label %knn.initialize.loop knn.training.loop:
%knn.row = phi i32 [ 0, %knn.initialize.loop ], [ %knn.row.next, %knn.replace.done ]
%knn.training.more = icmp ult i32 %knn.row, %training.rows
br i1 %knn.training.more, label %knn.distance.test, label %knn.average.loop knn.distance.test:
%knn.same.row = icmp eq i32 %knn.row, %query %knn.skip.self = and i1 %exclude.self, %knn.same.row
br i1 %knn.skip.self, label %knn.replace.done, label %knn.distance.entry knn.distance.entry:
%knn.query.global = add i32 %training.rows, %query %knn.query.offset = mul i32 %knn.query.global, %features
%knn.sample.offset = mul i32 %knn.row, %features
%knn.query.ptr = getelementptr inbounds double, ptr addrspace(1) %samples, i32 %knn.query.offset
%knn.sample.ptr = getelementptr inbounds double, ptr addrspace(1) %samples, i32 %knn.sample.offset
%knn.candidate = call double @distance( ptr addrspace(1) %knn.query.ptr, ptr addrspace(1) %knn.sample.ptr, i32 %features
) br label %knn.worst.loop knn.worst.loop:
%knn.worst.slot = phi i32 [ 0, %knn.distance.entry ], [ %knn.worst.next, %knn.worst.step ]
%knn.worst.index = phi i32 [ 0, %knn.distance.entry ], [ %knn.worst.index.next, %knn.worst.step ]
%knn.worst.distance = phi double [ -1.0, %knn.distance.entry ], [ %knn.worst.distance.next, %knn.worst.step ]
%knn.worst.more = icmp ult i32 %knn.worst.slot, %argument
br i1 %knn.worst.more, label %knn.worst.step, label %knn.replace knn.worst.step:
%knn.worst.local = add i32 %knn.double.base, %knn.worst.slot
%knn.worst.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %knn.worst.local
%knn.worst.candidate = load double, ptr addrspace(1) %knn.worst.ptr, align 8
%knn.worse = fcmp ogt double %knn.worst.candidate, %knn.worst.distance
%knn.worst.index.next = select i1 %knn.worse, i32 %knn.worst.slot, i32 %knn.worst.index
%knn.worst.distance.next = select i1 %knn.worse, double %knn.worst.candidate, double %knn.worst.distance
%knn.worst.next = add nuw i32 %knn.worst.slot, 1 br label %knn.worst.loop knn.replace:
%knn.replace.test = fcmp olt double %knn.candidate, %knn.worst.distance
br i1 %knn.replace.test, label %knn.replace.store, label %knn.replace.done knn.replace.store:
%knn.replace.distance.index = add i32 %knn.double.base, %knn.worst.index
%knn.replace.target.index = add i32 %knn.target.base, %knn.worst.index
%knn.replace.distance.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %knn.replace.distance.index
%knn.replace.target.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %knn.replace.target.index
%knn.target.ptr = getelementptr inbounds double, ptr addrspace(1) %targets, i32 %knn.row
%knn.target = load double, ptr addrspace(1) %knn.target.ptr, align 8
store double %knn.candidate, ptr addrspace(1) %knn.replace.distance.ptr, align 8
store double %knn.target, ptr addrspace(1) %knn.replace.target.ptr, align 8 br label %knn.replace.done knn.replace.done:
%knn.row.next = add nuw i32 %knn.row, 1 br label %knn.training.loop knn.average.loop:
%knn.average.slot = phi i32 [ 0, %knn.training.loop ], [ %knn.average.next, %knn.average.step ]
%knn.sum = phi double [ 0.0, %knn.training.loop ], [ %knn.sum.next, %knn.average.step ]
%knn.average.more = icmp ult i32 %knn.average.slot, %argument
br i1 %knn.average.more, label %knn.average.step, label %knn.store knn.average.step:
%knn.average.index = add i32 %knn.target.base, %knn.average.slot
%knn.average.ptr = getelementptr inbounds double, ptr addrspace(1) %workspace, i32 %knn.average.index
%knn.average.value = load double, ptr addrspace(1) %knn.average.ptr, align 8
%knn.sum.next = fadd double %knn.sum, %knn.average.value %knn.average.next = add nuw i32 %knn.average.slot, 1
br label %knn.average.loop knn.store: %knn.argument.double = uitofp i32 %argument to double
%knn.mean = fdiv double %knn.sum, %knn.argument.double
%knn.output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %query
store double %knn.mean, ptr addrspace(1) %knn.output.ptr, align 8 %query.next = add i32 %query, %threads
br label %knn.query.loop exit: ret void } attributes #0 = { nounwind "amdgpu-flat-work-group-size"="1,1024" }
attributes #1 = { alwaysinline nounwind }
