target triple = "amdgcn-amd-amdhsa"
declare i32 @llvm.amdgcn.workitem.id.x()
declare i32 @llvm.amdgcn.workgroup.id.x()
declare void @llvm.amdgcn.s.barrier()
declare double @llvm.sqrt.f64(double)
declare double @__ocml_exp_f64(double)
declare double @__ocml_log_f64(double)
declare double @__ocml_sin_f64(double)
declare double @__ocml_cos_f64(double)
declare double @__ocml_tanh_f64(double)
declare double @llvm.fabs.f64(double)

define internal void @dense_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture writeonly %output, i32 %rows, i32 %from, i32 %to, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.p = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %p = select i1 %use.forced, i32 %forced, i32 %hardware.p
  %count = mul i32 %rows, %to
  %active = icmp ult i32 %p, %count
  br i1 %active, label %body, label %exit
body:
  %row = udiv i32 %p, %to
  %out = urem i32 %p, %to
  br label %loop
loop:
  %i = phi i32 [ 0, %body ], [ %next, %step ]
  %sum = phi double [ 0.0, %body ], [ %sum.next, %step ]
  %more = icmp ult i32 %i, %from
  br i1 %more, label %step, label %done
step:
  %row.base = mul i32 %row, %from
  %input.index = add i32 %row.base, %i
  %weight.base = mul i32 %i, %to
  %weight.index = add i32 %weight.base, %out
  %input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
  %weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.index
  %a = load double, ptr addrspace(1) %input.ptr, align 8
  %b = load double, ptr addrspace(1) %weight.ptr, align 8
  %product = fmul double %a, %b
  %sum.next = fadd double %sum, %product
  %next = add nuw i32 %i, 1
  br label %loop
done:
  %output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
  store double %sum, ptr addrspace(1) %output.ptr, align 8
  br label %exit
exit:
  ret void
}

define internal void @weight_gradient_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %delta, ptr addrspace(1) nocapture writeonly %gradient, i32 %rows, i32 %from, i32 %to, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.p = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %p = select i1 %use.forced, i32 %forced, i32 %hardware.p
  %count = mul i32 %from, %to
  %active = icmp ult i32 %p, %count
  br i1 %active, label %body, label %exit
body:
  %in = udiv i32 %p, %to
  %out = urem i32 %p, %to
  br label %loop
loop:
  %row = phi i32 [ 0, %body ], [ %next, %step ]
  %sum = phi double [ 0.0, %body ], [ %sum.next, %step ]
  %more = icmp ult i32 %row, %rows
  br i1 %more, label %step, label %done
step:
  %input.base = mul i32 %row, %from
  %input.index = add i32 %input.base, %in
  %delta.base = mul i32 %row, %to
  %delta.index = add i32 %delta.base, %out
  %input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %delta.index
  %a = load double, ptr addrspace(1) %input.ptr, align 8
  %b = load double, ptr addrspace(1) %delta.ptr, align 8
  %product = fmul double %a, %b
  %sum.next = fadd double %sum, %product
  %next = add nuw i32 %row, 1
  br label %loop
done:
  %gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %p
  store double %sum, ptr addrspace(1) %gradient.ptr, align 8
  br label %exit
exit:
  ret void
}

define internal void @previous_gradient_body(ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture readonly %delta, ptr addrspace(1) nocapture writeonly %previous, i32 %rows, i32 %from, i32 %to, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.p = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %p = select i1 %use.forced, i32 %forced, i32 %hardware.p
  %count = mul i32 %rows, %from
  %active = icmp ult i32 %p, %count
  br i1 %active, label %body, label %exit
body:
  %row = udiv i32 %p, %from
  %in = urem i32 %p, %from
  br label %loop
loop:
  %out = phi i32 [ 0, %body ], [ %next, %step ]
  %sum = phi double [ 0.0, %body ], [ %sum.next, %step ]
  %more = icmp ult i32 %out, %to
  br i1 %more, label %step, label %done
step:
  %weight.base = mul i32 %in, %to
  %weight.index = add i32 %weight.base, %out
  %delta.base = mul i32 %row, %to
  %delta.index = add i32 %delta.base, %out
  %weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.index
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %delta.index
  %a = load double, ptr addrspace(1) %weight.ptr, align 8
  %b = load double, ptr addrspace(1) %delta.ptr, align 8
  %product = fmul double %a, %b
  %sum.next = fadd double %sum, %product
  %next = add nuw i32 %out, 1
  br label %loop
done:
  %previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %p
  store double %sum, ptr addrspace(1) %previous.ptr, align 8
  br label %exit
exit:
  ret void
}

define internal void @conv_forward_body(ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output, i32 %p, i32 %from, i32 %to, i32 %kernel) #1 {
entry:
  %positions.0 = sub i32 %from, %kernel
  %positions = add i32 %positions.0, 1
  %row = udiv i32 %p, %to
  %out = urem i32 %p, %to
  %filter = udiv i32 %out, %positions
  %position = urem i32 %out, %positions
  %row.base = mul i32 %row, %from
  %input.base = add i32 %row.base, %position
  %weight.base = mul i32 %filter, %kernel
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %step ]
  %sum = phi double [ 0.0, %entry ], [ %sum.next, %step ]
  %more = icmp ult i32 %i, %kernel
  br i1 %more, label %step, label %done
step:
  %input.index = add i32 %input.base, %i
  %weight.index = add i32 %weight.base, %i
  %input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
  %weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.index
  %x = load double, ptr addrspace(1) %input.ptr, align 8
  %w = load double, ptr addrspace(1) %weight.ptr, align 8
  %product = fmul double %x, %w
  %sum.next = fadd double %sum, %product
  %next = add i32 %i, 1
  br label %loop
done:
  %output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
  store double %sum, ptr addrspace(1) %output.ptr, align 8
  ret void
}

define internal void @conv_weight_gradient_body(ptr addrspace(1) %input, ptr addrspace(1) %delta, ptr addrspace(1) %gradient, i32 %p, i32 %rows, i32 %from, i32 %to, i32 %kernel) #1 {
entry:
  %positions.0 = sub i32 %from, %kernel
  %positions = add i32 %positions.0, 1
  %filter = udiv i32 %p, %kernel
  %offset = urem i32 %p, %kernel
  %items = mul i32 %rows, %positions
  br label %loop
loop:
  %item = phi i32 [ 0, %entry ], [ %next, %step ]
  %sum = phi double [ 0.0, %entry ], [ %sum.next, %step ]
  %more = icmp ult i32 %item, %items
  br i1 %more, label %step, label %done
step:
  %row = udiv i32 %item, %positions
  %position = urem i32 %item, %positions
  %input.row = mul i32 %row, %from
  %input.window = add i32 %input.row, %position
  %input.index = add i32 %input.window, %offset
  %delta.row = mul i32 %row, %to
  %filter.base = mul i32 %filter, %positions
  %delta.local = add i32 %filter.base, %position
  %delta.index = add i32 %delta.row, %delta.local
  %input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %delta.index
  %x = load double, ptr addrspace(1) %input.ptr, align 8
  %d = load double, ptr addrspace(1) %delta.ptr, align 8
  %product = fmul double %x, %d
  %sum.next = fadd double %sum, %product
  %next = add i32 %item, 1
  br label %loop
done:
  %gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %p
  store double %sum, ptr addrspace(1) %gradient.ptr, align 8
  ret void
}

define internal void @conv_previous_gradient_body(ptr addrspace(1) %weights, ptr addrspace(1) %delta, ptr addrspace(1) %previous, i32 %p, i32 %from, i32 %to, i32 %kernel) #1 {
entry:
  %positions.0 = sub i32 %from, %kernel
  %positions = add i32 %positions.0, 1
  %filters = udiv i32 %to, %positions
  %row = udiv i32 %p, %from
  %column = urem i32 %p, %from
  %items = mul i32 %filters, %positions
  br label %loop
loop:
  %item = phi i32 [ 0, %entry ], [ %next, %step ]
  %sum = phi double [ 0.0, %entry ], [ %sum.next, %step ]
  %more = icmp ult i32 %item, %items
  br i1 %more, label %step, label %done
step:
  %filter = udiv i32 %item, %positions
  %position = urem i32 %item, %positions
  %end = add i32 %position, %kernel
  %after.start = icmp uge i32 %column, %position
  %before.end = icmp ult i32 %column, %end
  %inside = and i1 %after.start, %before.end
  %offset = sub i32 %column, %position
  %safe.offset = select i1 %inside, i32 %offset, i32 0
  %weight.base = mul i32 %filter, %kernel
  %weight.index = add i32 %weight.base, %safe.offset
  %delta.row = mul i32 %row, %to
  %delta.index = add i32 %delta.row, %item
  %weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.index
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %delta.index
  %w = load double, ptr addrspace(1) %weight.ptr, align 8
  %d = load double, ptr addrspace(1) %delta.ptr, align 8
  %product = fmul double %w, %d
  %selected = select i1 %inside, double %product, double 0.0
  %sum.next = fadd double %sum, %selected
  %next = add i32 %item, 1
  br label %loop
done:
  %previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %p
  store double %sum, ptr addrspace(1) %previous.ptr, align 8
  ret void
}

define internal void @pool_forward_body(ptr addrspace(1) %input, ptr addrspace(1) %output, i32 %p, i32 %from, i32 %to, i32 %size) #1 {
entry:
  %row = udiv i32 %p, %to
  %out = urem i32 %p, %to
  %start = mul i32 %out, %size
  %candidate.end = add i32 %start, %size
  %short = icmp ult i32 %candidate.end, %from
  %end = select i1 %short, i32 %candidate.end, i32 %from
  %row.base = mul i32 %row, %from
  br label %loop
loop:
  %i = phi i32 [ %start, %entry ], [ %next, %step ]
  %maximum = phi double [ 0xFFF0000000000000, %entry ], [ %maximum.next, %step ]
  %more = icmp ult i32 %i, %end
  br i1 %more, label %step, label %done
step:
  %index = add i32 %row.base, %i
  %input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %index
  %value = load double, ptr addrspace(1) %input.ptr, align 8
  %greater = fcmp ogt double %value, %maximum
  %maximum.next = select i1 %greater, double %value, double %maximum
  %next = add i32 %i, 1
  br label %loop
done:
  %output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
  store double %maximum, ptr addrspace(1) %output.ptr, align 8
  ret void
}

define internal void @pool_previous_gradient_body(ptr addrspace(1) %input, ptr addrspace(1) %delta, ptr addrspace(1) %previous, i32 %p, i32 %from, i32 %to, i32 %size) #1 {
entry:
  %row = udiv i32 %p, %from
  %column = urem i32 %p, %from
  %window = udiv i32 %column, %size
  %start = mul i32 %window, %size
  %candidate.end = add i32 %start, %size
  %short = icmp ult i32 %candidate.end, %from
  %end = select i1 %short, i32 %candidate.end, i32 %from
  %row.base = mul i32 %row, %from
  br label %loop
loop:
  %i = phi i32 [ %start, %entry ], [ %next, %step ]
  %maximum = phi double [ 0xFFF0000000000000, %entry ], [ %maximum.next, %step ]
  %winner = phi i32 [ %start, %entry ], [ %winner.next, %step ]
  %more = icmp ult i32 %i, %end
  br i1 %more, label %step, label %done
step:
  %index = add i32 %row.base, %i
  %input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %index
  %value = load double, ptr addrspace(1) %input.ptr, align 8
  %greater = fcmp ogt double %value, %maximum
  %maximum.next = select i1 %greater, double %value, double %maximum
  %winner.next = select i1 %greater, i32 %i, i32 %winner
  %next = add i32 %i, 1
  br label %loop
done:
  %delta.row = mul i32 %row, %to
  %delta.index = add i32 %delta.row, %window
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %delta.index
  %delta.value = load double, ptr addrspace(1) %delta.ptr, align 8
  %selected = icmp eq i32 %column, %winner
  %result = select i1 %selected, double %delta.value, double 0.0
  %previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %p
  store double %result, ptr addrspace(1) %previous.ptr, align 8
  ret void
}

define internal i32 @embedding_index(double %value, i32 %vocabulary) #1 {
entry:
  %unordered = fcmp uno double %value, %value
  %positive = fcmp ogt double %value, 0.0
  %not.positive = xor i1 %positive, true
  %zero = or i1 %unordered, %not.positive
  br i1 %zero, label %done, label %convert
convert:
  %last = sub i32 %vocabulary, 1
  %maximum = uitofp i32 %last to double
  %below = fcmp olt double %value, %maximum
  %bounded = select i1 %below, double %value, double %maximum
  %index = fptoui double %bounded to i32
  br label %done
done:
  %result = phi i32 [ 0, %entry ], [ %index, %convert ]
  ret i32 %result
}

define internal void @embedding_forward_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %table, ptr addrspace(1) nocapture writeonly %output, i32 %p, i32 %from, i32 %to, i32 %vocabulary) #1 {
entry:
  %dimensions = udiv i32 %to, %from
  %row = udiv i32 %p, %to
  %local = urem i32 %p, %to
  %slot = udiv i32 %local, %dimensions
  %component = urem i32 %local, %dimensions
  %row.base = mul i32 %row, %from
  %input.index = add i32 %row.base, %slot
  %input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
  %value = load double, ptr addrspace(1) %input.ptr, align 8
  %token = call i32 @embedding_index(double %value, i32 %vocabulary)
  %token.base = mul i32 %token, %dimensions
  %table.index = add i32 %token.base, %component
  %table.ptr = getelementptr inbounds double, ptr addrspace(1) %table, i32 %table.index
  %embedded = load double, ptr addrspace(1) %table.ptr, align 8
  %output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
  store double %embedded, ptr addrspace(1) %output.ptr, align 8
  ret void
}

define internal void @embedding_gradient_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %delta, ptr addrspace(1) nocapture writeonly %gradient, i32 %p, i32 %rows, i32 %from, i32 %to, i32 %vocabulary) #1 {
entry:
  %dimensions = udiv i32 %to, %from
  %token = udiv i32 %p, %dimensions
  %component = urem i32 %p, %dimensions
  %observations = mul i32 %rows, %from
  br label %loop
loop:
  %observation = phi i32 [ 0, %entry ], [ %next, %step ]
  %sum = phi double [ 0.0, %entry ], [ %sum.next, %step ]
  %more = icmp ult i32 %observation, %observations
  br i1 %more, label %step, label %done
step:
  %value.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %observation
  %value = load double, ptr addrspace(1) %value.ptr, align 8
  %index = call i32 @embedding_index(double %value, i32 %vocabulary)
  %matches = icmp eq i32 %index, %token
  %row = udiv i32 %observation, %from
  %slot = urem i32 %observation, %from
  %row.base = mul i32 %row, %to
  %slot.base = mul i32 %slot, %dimensions
  %local = add i32 %slot.base, %component
  %delta.index = add i32 %row.base, %local
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %delta.index
  %delta.value = load double, ptr addrspace(1) %delta.ptr, align 8
  %selected = select i1 %matches, double %delta.value, double 0.0
  %sum.next = fadd double %sum, %selected
  %next = add nuw i32 %observation, 1
  br label %loop
done:
  %gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %p
  store double %sum, ptr addrspace(1) %gradient.ptr, align 8
  ret void
}

define internal void @attention_forward_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture writeonly %output, ptr addrspace(1) nocapture writeonly %context, i32 %p, i32 %width, i32 %heads) #1 {
entry:
  %row = udiv i32 %p, %width
  %query = urem i32 %p, %width
  %row.base = mul i32 %row, %width
  %query.index = add i32 %row.base, %query
  %query.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %query.index
  %x.query = load double, ptr addrspace(1) %query.ptr, align 8
  br label %head.loop
head.loop:
  %head = phi i32 [ 0, %entry ], [ %head.next, %head.finish ]
  %result = phi double [ 0.0, %entry ], [ %result.next, %head.finish ]
  %head.more = icmp ult i32 %head, %heads
  br i1 %head.more, label %head.load, label %done
head.load:
  %weight.base = mul i32 %head, 4
  %row.head = mul i32 %row, %heads
  %row.head.index = add i32 %row.head, %head
  %context.width = mul i32 %row.head.index, %width
  %context.item = add i32 %context.width, %query
  %context.stride = add i32 %width, 2
  %context.base = mul i32 %context.item, %context.stride
  %q.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.base
  %k.index = add i32 %weight.base, 1
  %v.index = add i32 %weight.base, 2
  %o.index = add i32 %weight.base, 3
  %k.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %k.index
  %v.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %v.index
  %o.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %o.index
  %wq = load double, ptr addrspace(1) %q.ptr, align 8
  %wk = load double, ptr addrspace(1) %k.ptr, align 8
  %wv = load double, ptr addrspace(1) %v.ptr, align 8
  %wo = load double, ptr addrspace(1) %o.ptr, align 8
  %qk = fmul double %wq, %wk
  %beta = fmul double %qk, %x.query
  br label %max.loop
max.loop:
  %j.max = phi i32 [ 0, %head.load ], [ %j.max.next, %max.step ]
  %maximum = phi double [ 0xFFF0000000000000, %head.load ], [ %maximum.next, %max.step ]
  %max.more = icmp ult i32 %j.max, %width
  br i1 %max.more, label %max.step, label %sum.loop
max.step:
  %max.index = add i32 %row.base, %j.max
  %max.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %max.index
  %x.max = load double, ptr addrspace(1) %max.ptr, align 8
  %score.max = fmul double %beta, %x.max
  %greater = fcmp ogt double %score.max, %maximum
  %maximum.next = select i1 %greater, double %score.max, double %maximum
  %j.max.next = add nuw i32 %j.max, 1
  br label %max.loop
sum.loop:
  %j.sum = phi i32 [ 0, %max.loop ], [ %j.sum.next, %sum.step ]
  %denominator = phi double [ 0.0, %max.loop ], [ %denominator.next, %sum.step ]
  %numerator = phi double [ 0.0, %max.loop ], [ %numerator.next, %sum.step ]
  %sum.more = icmp ult i32 %j.sum, %width
  br i1 %sum.more, label %sum.step, label %head.done
sum.step:
  %sum.index = add i32 %row.base, %j.sum
  %sum.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %sum.index
  %x.sum = load double, ptr addrspace(1) %sum.ptr, align 8
  %score = fmul double %beta, %x.sum
  %centered = fsub double %score, %maximum
  %exponential = call double @__ocml_exp_f64(double %centered)
  %alpha.index = add i32 %context.base, %j.sum
  %alpha.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %alpha.index
  store double %exponential, ptr addrspace(1) %alpha.ptr, align 8
  %denominator.next = fadd double %denominator, %exponential
  %weighted = fmul double %exponential, %x.sum
  %numerator.next = fadd double %numerator, %weighted
  %j.sum.next = add nuw i32 %j.sum, 1
  br label %sum.loop
head.done:
  %mean = fdiv double %numerator, %denominator
  br label %normalize.loop
normalize.loop:
  %j.normalize = phi i32 [ 0, %head.done ], [ %j.normalize.next, %normalize.step ]
  %variance = phi double [ 0.0, %head.done ], [ %variance.next, %normalize.step ]
  %normalize.more = icmp ult i32 %j.normalize, %width
  br i1 %normalize.more, label %normalize.step, label %head.finish
normalize.step:
  %normalize.alpha.index = add i32 %context.base, %j.normalize
  %normalize.alpha.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %normalize.alpha.index
  %unnormalized = load double, ptr addrspace(1) %normalize.alpha.ptr, align 8
  %alpha = fdiv double %unnormalized, %denominator
  store double %alpha, ptr addrspace(1) %normalize.alpha.ptr, align 8
  %normalize.input.index = add i32 %row.base, %j.normalize
  %normalize.input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %normalize.input.index
  %normalize.input = load double, ptr addrspace(1) %normalize.input.ptr, align 8
  %normalize.difference = fsub double %normalize.input, %mean
  %normalize.square = fmul double %normalize.difference, %normalize.difference
  %normalize.weighted = fmul double %alpha, %normalize.square
  %variance.next = fadd double %variance, %normalize.weighted
  %j.normalize.next = add nuw i32 %j.normalize, 1
  br label %normalize.loop
head.finish:
  %value.scale = fmul double %wv, %wo
  %head.value = fmul double %mean, %value.scale
  %result.next = fadd double %result, %head.value
  %context.mean.index = add i32 %context.base, %width
  %context.variance.index = add i32 %context.mean.index, 1
  %context.mean = getelementptr inbounds double, ptr addrspace(1) %context, i32 %context.mean.index
  %context.variance = getelementptr inbounds double, ptr addrspace(1) %context, i32 %context.variance.index
  store double %mean, ptr addrspace(1) %context.mean, align 8
  store double %variance, ptr addrspace(1) %context.variance, align 8
  %head.next = add nuw i32 %head, 1
  br label %head.loop
done:
  %output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
  store double %result, ptr addrspace(1) %output.ptr, align 8
  ret void
}

define internal void @attention_weight_gradient_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture readonly %delta, ptr addrspace(1) nocapture readonly %context, ptr addrspace(1) nocapture writeonly %gradient, i32 %p, i32 %rows, i32 %width, i32 %heads) #1 {
entry:
  %head = udiv i32 %p, 4
  %kind = urem i32 %p, 4
  %weight.base = mul i32 %head, 4
  %q.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.base
  %k.index = add i32 %weight.base, 1
  %v.index = add i32 %weight.base, 2
  %o.index = add i32 %weight.base, 3
  %k.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %k.index
  %v.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %v.index
  %o.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %o.index
  %wq = load double, ptr addrspace(1) %q.ptr, align 8
  %wk = load double, ptr addrspace(1) %k.ptr, align 8
  %wv = load double, ptr addrspace(1) %v.ptr, align 8
  %wo = load double, ptr addrspace(1) %o.ptr, align 8
  %queries = mul i32 %rows, %width
  br label %query.loop
query.loop:
  %query.p = phi i32 [ 0, %entry ], [ %query.next, %accumulate ]
  %sum = phi double [ 0.0, %entry ], [ %sum.next, %accumulate ]
  %query.more = icmp ult i32 %query.p, %queries
  br i1 %query.more, label %query.load, label %done
query.load:
  %row = udiv i32 %query.p, %width
  %query = urem i32 %query.p, %width
  %x.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %query.p
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %query.p
  %x = load double, ptr addrspace(1) %x.ptr, align 8
  %d = load double, ptr addrspace(1) %delta.ptr, align 8
  %context.row = mul i32 %row, %heads
  %context.head = add i32 %context.row, %head
  %context.width = mul i32 %context.head, %width
  %context.item = add i32 %context.width, %query
  %context.stride = add i32 %width, 2
  %context.base = mul i32 %context.item, %context.stride
  %mean.index = add i32 %context.base, %width
  %variance.index = add i32 %mean.index, 1
  %mean.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %mean.index
  %variance.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %variance.index
  %mean = load double, ptr addrspace(1) %mean.ptr, align 8
  %variance = load double, ptr addrspace(1) %variance.ptr, align 8
  %is.q = icmp eq i32 %kind, 0
  %is.k = icmp eq i32 %kind, 1
  %needs.variance = or i1 %is.q, %is.k
  br i1 %needs.variance, label %variance.direct, label %direct
direct:
  %is.v = icmp eq i32 %kind, 2
  %other = select i1 %is.v, double %wo, double %wv
  %direct.first = fmul double %d, %other
  %direct.value = fmul double %direct.first, %mean
  br label %accumulate
variance.direct:
  %qk.other = select i1 %is.q, double %wk, double %wq
  %variance.first = fmul double %d, %wo
  %variance.second = fmul double %variance.first, %wv
  %variance.third = fmul double %variance.second, %x
  %variance.fourth = fmul double %variance.third, %variance
  %variance.result = fmul double %variance.fourth, %qk.other
  br label %accumulate
accumulate:
  %contribution = phi double [ %direct.value, %direct ], [ %variance.result, %variance.direct ]
  %sum.next = fadd double %sum, %contribution
  %query.next = add nuw i32 %query.p, 1
  br label %query.loop
done:
  %gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %p
  store double %sum, ptr addrspace(1) %gradient.ptr, align 8
  ret void
}

define internal void @attention_previous_gradient_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture readonly %delta, ptr addrspace(1) nocapture readonly %context, ptr addrspace(1) nocapture writeonly %previous, i32 %p, i32 %width, i32 %heads) #1 {
entry:
  %row = udiv i32 %p, %width
  %token = urem i32 %p, %width
  %row.base = mul i32 %row, %width
  %token.index = add i32 %row.base, %token
  %token.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %token.index
  %x.token = load double, ptr addrspace(1) %token.ptr, align 8
  br label %head.loop
head.loop:
  %head = phi i32 [ 0, %entry ], [ %head.next, %key.done ]
  %total = phi double [ 0.0, %entry ], [ %total.next, %key.done ]
  %head.more = icmp ult i32 %head, %heads
  br i1 %head.more, label %head.load, label %done
head.load:
  %weight.base = mul i32 %head, 4
  %q.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.base
  %k.index = add i32 %weight.base, 1
  %v.index = add i32 %weight.base, 2
  %o.index = add i32 %weight.base, 3
  %k.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %k.index
  %v.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %v.index
  %o.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %o.index
  %wq = load double, ptr addrspace(1) %q.ptr, align 8
  %wk = load double, ptr addrspace(1) %k.ptr, align 8
  %wv = load double, ptr addrspace(1) %v.ptr, align 8
  %wo = load double, ptr addrspace(1) %o.ptr, align 8
  %a = fmul double %wq, %wk
  %c = fmul double %wv, %wo
  %context.row = mul i32 %row, %heads
  %context.head = add i32 %context.row, %head
  %context.width = mul i32 %context.head, %width
  %query.item = add i32 %context.width, %token
  %context.stride = add i32 %width, 2
  %query.base = mul i32 %query.item, %context.stride
  %query.mean.index = add i32 %query.base, %width
  %query.variance.index = add i32 %query.mean.index, 1
  %query.mean.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %query.mean.index
  %query.variance.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %query.variance.index
  %query.mean = load double, ptr addrspace(1) %query.mean.ptr, align 8
  %variance = load double, ptr addrspace(1) %query.variance.ptr, align 8
  br label %variance.done
variance.done:
  %query.delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %token.index
  %query.delta = load double, ptr addrspace(1) %query.delta.ptr, align 8
  %query.first = fmul double %query.delta, %c
  %query.second = fmul double %query.first, %a
  %query.contribution = fmul double %query.second, %variance
  br label %key.loop
key.loop:
  %query = phi i32 [ 0, %variance.done ], [ %query.next, %key.step ]
  %head.total = phi double [ %query.contribution, %variance.done ], [ %head.total.next, %key.step ]
  %key.more = icmp ult i32 %query, %width
  br i1 %key.more, label %key.step, label %key.done
key.step:
  %query.index = add i32 %row.base, %query
  %query.x.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %query.index
  %query.delta.all.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %query.index
  %x.query = load double, ptr addrspace(1) %query.x.ptr, align 8
  %delta.query = load double, ptr addrspace(1) %query.delta.all.ptr, align 8
  %key.context.item = add i32 %context.width, %query
  %key.context.base = mul i32 %key.context.item, %context.stride
  %key.mean.index = add i32 %key.context.base, %width
  %key.alpha.index = add i32 %key.context.base, %token
  %key.mean.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %key.mean.index
  %key.alpha.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %key.alpha.index
  %key.mean = load double, ptr addrspace(1) %key.mean.ptr, align 8
  %key.alpha = load double, ptr addrspace(1) %key.alpha.ptr, align 8
  %token.difference = fsub double %x.token, %key.mean
  %softmax.first = fmul double %a, %x.query
  %softmax.second = fmul double %softmax.first, %token.difference
  %bracket = fadd double 1.0, %softmax.second
  %key.first = fmul double %delta.query, %c
  %key.second = fmul double %key.first, %key.alpha
  %key.contribution = fmul double %key.second, %bracket
  %head.total.next = fadd double %head.total, %key.contribution
  %query.next = add nuw i32 %query, 1
  br label %key.loop
key.done:
  %total.next = fadd double %total, %head.total
  %head.next = add nuw i32 %head, 1
  br label %head.loop
done:
  %previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %p
  store double %total, ptr addrspace(1) %previous.ptr, align 8
  ret void
}

define internal void @adamw_body(ptr addrspace(1) %weights, ptr addrspace(1) nocapture readonly %gradient, ptr addrspace(1) %moments, ptr addrspace(1) %variances, i32 %count, i32 %threads, double %rate, double %beta1, double %beta2, double %epsilon, double %decay, i32 %step, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.p = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %p = select i1 %use.forced, i32 %forced, i32 %hardware.p
  %active = icmp ult i32 %p, %count
  br i1 %active, label %power.loop, label %exit
power.loop:
  %power.i = phi i32 [ 0, %entry ], [ %power.next, %power.step ]
  %beta1.power = phi double [ 1.0, %entry ], [ %beta1.next, %power.step ]
  %beta2.power = phi double [ 1.0, %entry ], [ %beta2.next, %power.step ]
  %power.more = icmp slt i32 %power.i, %step
  br i1 %power.more, label %power.step, label %body
power.step:
  %beta1.next = fmul double %beta1.power, %beta1
  %beta2.next = fmul double %beta2.power, %beta2
  %power.next = add nuw i32 %power.i, 1
  br label %power.loop
body:
  %w.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %p
  %g.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %p
  %m.ptr = getelementptr inbounds double, ptr addrspace(1) %moments, i32 %p
  %v.ptr = getelementptr inbounds double, ptr addrspace(1) %variances, i32 %p
  %w = load double, ptr addrspace(1) %w.ptr, align 8
  %g = load double, ptr addrspace(1) %g.ptr, align 8
  %m = load double, ptr addrspace(1) %m.ptr, align 8
  %v = load double, ptr addrspace(1) %v.ptr, align 8
  %one.b1 = fsub double 1.0, %beta1
  %one.b2 = fsub double 1.0, %beta2
  %m.old = fmul double %beta1, %m
  %m.new.part = fmul double %one.b1, %g
  %m.new = fadd double %m.old, %m.new.part
  %g.square = fmul double %g, %g
  %v.old = fmul double %beta2, %v
  %v.new.part = fmul double %one.b2, %g.square
  %v.new = fadd double %v.old, %v.new.part
  %correction1 = fsub double 1.0, %beta1.power
  %correction2 = fsub double 1.0, %beta2.power
  %m.hat = fdiv double %m.new, %correction1
  %v.hat = fdiv double %v.new, %correction2
  %root = call double @llvm.sqrt.f64(double %v.hat)
  %denominator = fadd double %root, %epsilon
  %adaptive = fdiv double %m.hat, %denominator
  %decayed = fmul double %decay, %w
  %update = fadd double %adaptive, %decayed
  %scaled = fmul double %rate, %update
  %w.new = fsub double %w, %scaled
  store double %w.new, ptr addrspace(1) %w.ptr, align 8
  store double %m.new, ptr addrspace(1) %m.ptr, align 8
  store double %v.new, ptr addrspace(1) %v.ptr, align 8
  br label %exit
exit:
  ret void
}

define internal void @transform_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) %values, ptr addrspace(1) nocapture writeonly %raw, ptr addrspace(1) nocapture writeonly %operation_derivatives, ptr addrspace(1) nocapture writeonly %activation_derivatives, ptr addrspace(1) nocapture readonly %config, i32 %count, i32 %from, i32 %to, i32 %operation, i32 %activation, double %parameter, double %secondary, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.p = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %p = select i1 %use.forced, i32 %forced, i32 %hardware.p
  %active = icmp ult i32 %p, %count
  br i1 %active, label %block, label %exit
block:
  %value.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %p
  %value = load double, ptr addrspace(1) %value.ptr, align 8
  switch i32 %operation, label %block.linear [
    i32 6, label %block.rnn
    i32 7, label %block.gru
    i32 8, label %block.lstm
    i32 11, label %block.residual
    i32 12, label %block.perceptron
  ]
block.linear:
  br label %block.done
block.rnn:
  %rnn.result = call double @__ocml_tanh_f64(double %value)
  %rnn.square = fmul double %rnn.result, %rnn.result
  %rnn.derivative = fsub double 1.0, %rnn.square
  br label %block.done
block.gru:
  %gru.negative = fneg double %value
  %gru.exp = call double @__ocml_exp_f64(double %gru.negative)
  %gru.denominator = fadd double 1.0, %gru.exp
  %gru.gate = fdiv double 1.0, %gru.denominator
  %gru.state = call double @__ocml_tanh_f64(double %value)
  %gru.result = fmul double %gru.gate, %gru.state
  %gru.one.gate = fsub double 1.0, %gru.gate
  %gru.gate.derivative = fmul double %gru.gate, %gru.one.gate
  %gru.term1 = fmul double %gru.gate.derivative, %gru.state
  %gru.state.square = fmul double %gru.state, %gru.state
  %gru.one.state = fsub double 1.0, %gru.state.square
  %gru.term2 = fmul double %gru.gate, %gru.one.state
  %gru.derivative = fadd double %gru.term1, %gru.term2
  br label %block.done
block.lstm:
  %lstm.negative = fneg double %value
  %lstm.exp = call double @__ocml_exp_f64(double %lstm.negative)
  %lstm.denominator = fadd double 1.0, %lstm.exp
  %lstm.gate = fdiv double 1.0, %lstm.denominator
  %lstm.cell = call double @__ocml_tanh_f64(double %value)
  %lstm.state = call double @__ocml_tanh_f64(double %lstm.cell)
  %lstm.result = fmul double %lstm.gate, %lstm.state
  %lstm.one.gate = fsub double 1.0, %lstm.gate
  %lstm.gate.derivative = fmul double %lstm.gate, %lstm.one.gate
  %lstm.term1 = fmul double %lstm.gate.derivative, %lstm.state
  %lstm.state.square = fmul double %lstm.state, %lstm.state
  %lstm.cell.square = fmul double %lstm.cell, %lstm.cell
  %lstm.one.state = fsub double 1.0, %lstm.state.square
  %lstm.one.cell = fsub double 1.0, %lstm.cell.square
  %lstm.inner = fmul double %lstm.one.state, %lstm.one.cell
  %lstm.term2a = fmul double %lstm.gate, %lstm.inner
  %lstm.derivative = fadd double %lstm.term1, %lstm.term2a
  br label %block.done
block.residual:
  %row = udiv i32 %p, %to
  %column = urem i32 %p, %to
  %skip.column = urem i32 %column, %from
  %row.base = mul i32 %row, %from
  %skip.index = add i32 %row.base, %skip.column
  %skip.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %skip.index
  %skip = load double, ptr addrspace(1) %skip.ptr, align 8
  %residual.result = fadd double %value, %skip
  br label %block.done
block.perceptron:
  %perceptron.test = fcmp oge double %value, 0.0
  %perceptron.result = select i1 %perceptron.test, double 1.0, double 0.0
  br label %block.done
block.done:
  %block.result = phi double [ %value, %block.linear ], [ %rnn.result, %block.rnn ], [ %gru.result, %block.gru ], [ %lstm.result, %block.lstm ], [ %residual.result, %block.residual ], [ %perceptron.result, %block.perceptron ]
  %block.derivative = phi double [ 1.0, %block.linear ], [ %rnn.derivative, %block.rnn ], [ %gru.derivative, %block.gru ], [ %lstm.derivative, %block.lstm ], [ 1.0, %block.residual ], [ 1.0, %block.perceptron ]
  %raw.ptr = getelementptr inbounds double, ptr addrspace(1) %raw, i32 %p
  store double %block.result, ptr addrspace(1) %raw.ptr, align 8
  %operation.ptr = getelementptr inbounds double, ptr addrspace(1) %operation_derivatives, i32 %p
  store double %block.derivative, ptr addrspace(1) %operation.ptr, align 8
  switch i32 %activation, label %activation.linear [
    i32 1, label %activation.cos
    i32 2, label %activation.exp
    i32 3, label %activation.log10
    i32 4, label %activation.ln
    i32 5, label %activation.ln
    i32 6, label %activation.huber
    i32 7, label %activation.tan
    i32 8, label %activation.relu
    i32 9, label %activation.leak
    i32 10, label %activation.sigmoid
    i32 11, label %activation.tanh
    i32 12, label %activation.selu
    i32 13, label %activation.gelu
    i32 14, label %activation.silu
    i32 15, label %activation.elu
    i32 16, label %activation.prelu
  ]
activation.linear:
  br label %activation.done
activation.cos:
  %cos.result = call double @__ocml_cos_f64(double %block.result)
  %cos.sin = call double @__ocml_sin_f64(double %block.result)
  %cos.derivative = fneg double %cos.sin
  br label %activation.done
activation.exp:
  %exp.result = call double @__ocml_exp_f64(double %block.result)
  br label %activation.done
activation.log10:
  %log.abs = call double @llvm.fabs.f64(double %block.result)
  %log.shift = fadd double %log.abs, 1.0
  %log.value = call double @__ocml_log_f64(double %log.shift)
  %log.positive = fcmp ogt double %block.result, 0.0
  %log.negative = fcmp olt double %block.result, 0.0
  %log.signed = select i1 %log.positive, double %log.value, double 0.0
  %log.negated = fneg double %log.value
  %log.result = select i1 %log.negative, double %log.negated, double %log.signed
  %log10.result = fdiv double %log.result, 0x40026BB1BBB55516
  %log.derivative.base = fdiv double 1.0, %log.shift
  %log10.derivative = fdiv double %log.derivative.base, 0x40026BB1BBB55516
  br label %activation.done
activation.ln:
  %ln.abs = call double @llvm.fabs.f64(double %block.result)
  %ln.shift = fadd double %ln.abs, 1.0
  %ln.value = call double @__ocml_log_f64(double %ln.shift)
  %ln.positive = fcmp ogt double %block.result, 0.0
  %ln.negative = fcmp olt double %block.result, 0.0
  %ln.signed = select i1 %ln.positive, double %ln.value, double 0.0
  %ln.negated = fneg double %ln.value
  %ln.result = select i1 %ln.negative, double %ln.negated, double %ln.signed
  %ln.derivative = fdiv double 1.0, %ln.shift
  br label %activation.done
activation.huber:
  %huber.abs = call double @llvm.fabs.f64(double %block.result)
  %huber.small = fcmp ole double %huber.abs, 1.0
  %huber.square = fmul double %block.result, %block.result
  %huber.half = fmul double %huber.square, 0.5
  %huber.large = fsub double %huber.abs, 0.5
  %huber.result = select i1 %huber.small, double %huber.half, double %huber.large
  %huber.low = fcmp olt double %block.result, -1.0
  %huber.high = fcmp ogt double %block.result, 1.0
  %huber.lower = select i1 %huber.low, double -1.0, double %block.result
  %huber.derivative = select i1 %huber.high, double 1.0, double %huber.lower
  br label %activation.done
activation.tan:
  %tan.sin = call double @__ocml_sin_f64(double %block.result)
  %tan.cos = call double @__ocml_cos_f64(double %block.result)
  %tan.result = fdiv double %tan.sin, %tan.cos
  %tan.square = fmul double %tan.result, %tan.result
  %tan.derivative = fadd double 1.0, %tan.square
  br label %activation.done
activation.relu:
  %relu.test = fcmp ogt double %block.result, 0.0
  %relu.result = select i1 %relu.test, double %block.result, double 0.0
  %relu.derivative = select i1 %relu.test, double 1.0, double 0.0
  br label %activation.done
activation.leak:
  %leak.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 0
  %leak = load double, ptr addrspace(1) %leak.ptr, align 8
  %leak.test = fcmp ogt double %block.result, 0.0
  %leak.negative = fmul double %leak, %block.result
  %leak.result = select i1 %leak.test, double %block.result, double %leak.negative
  %leak.derivative = select i1 %leak.test, double 1.0, double %leak
  br label %activation.done
activation.sigmoid:
  %sigmoid.negative = fneg double %block.result
  %sigmoid.exp = call double @__ocml_exp_f64(double %sigmoid.negative)
  %sigmoid.denominator = fadd double 1.0, %sigmoid.exp
  %sigmoid.result = fdiv double 1.0, %sigmoid.denominator
  %sigmoid.one = fsub double 1.0, %sigmoid.result
  %sigmoid.derivative = fmul double %sigmoid.result, %sigmoid.one
  br label %activation.done
activation.tanh:
  %tanh.result = call double @__ocml_tanh_f64(double %block.result)
  %tanh.square = fmul double %tanh.result, %tanh.result
  %tanh.derivative = fsub double 1.0, %tanh.square
  br label %activation.done
activation.selu:
  %selu.alpha.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 3
  %selu.scale.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 4
  %selu.alpha = load double, ptr addrspace(1) %selu.alpha.ptr, align 8
  %selu.scale = load double, ptr addrspace(1) %selu.scale.ptr, align 8
  %selu.test = fcmp ogt double %block.result, 0.0
  %selu.exp = call double @__ocml_exp_f64(double %block.result)
  %selu.expm1 = fsub double %selu.exp, 1.0
  %selu.alpha.exp = fmul double %selu.alpha, %selu.expm1
  %selu.inner = select i1 %selu.test, double %block.result, double %selu.alpha.exp
  %selu.result = fmul double %selu.scale, %selu.inner
  %selu.alpha.derivative = fmul double %selu.alpha, %selu.exp
  %selu.inner.derivative = select i1 %selu.test, double 1.0, double %selu.alpha.derivative
  %selu.derivative = fmul double %selu.scale, %selu.inner.derivative
  br label %activation.done
activation.gelu:
  %gelu.scale.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 5
  %gelu.cubic.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 6
  %gelu.scale = load double, ptr addrspace(1) %gelu.scale.ptr, align 8
  %gelu.cubic = load double, ptr addrspace(1) %gelu.cubic.ptr, align 8
  %gelu.square = fmul double %block.result, %block.result
  %gelu.cube = fmul double %gelu.square, %block.result
  %gelu.cubic.term = fmul double %gelu.cubic, %gelu.cube
  %gelu.sum = fadd double %block.result, %gelu.cubic.term
  %gelu.argument = fmul double %gelu.scale, %gelu.sum
  %gelu.curve = call double @__ocml_tanh_f64(double %gelu.argument)
  %gelu.one.curve = fadd double 1.0, %gelu.curve
  %gelu.half.input = fmul double 0.5, %block.result
  %gelu.result = fmul double %gelu.half.input, %gelu.one.curve
  %gelu.curve.square = fmul double %gelu.curve, %gelu.curve
  %gelu.one.square = fsub double 1.0, %gelu.curve.square
  %gelu.three.cubic = fmul double 3.0, %gelu.cubic
  %gelu.poly.term = fmul double %gelu.three.cubic, %gelu.square
  %gelu.poly = fadd double 1.0, %gelu.poly.term
  %gelu.derivative.a = fmul double 0.5, %gelu.one.curve
  %gelu.derivative.b1 = fmul double %gelu.half.input, %gelu.one.square
  %gelu.derivative.b2 = fmul double %gelu.derivative.b1, %gelu.scale
  %gelu.derivative.b = fmul double %gelu.derivative.b2, %gelu.poly
  %gelu.derivative = fadd double %gelu.derivative.a, %gelu.derivative.b
  br label %activation.done
activation.silu:
  %silu.negative = fneg double %block.result
  %silu.exp = call double @__ocml_exp_f64(double %silu.negative)
  %silu.denominator = fadd double 1.0, %silu.exp
  %silu.sigmoid = fdiv double 1.0, %silu.denominator
  %silu.result = fmul double %block.result, %silu.sigmoid
  %silu.one = fsub double 1.0, %silu.sigmoid
  %silu.product = fmul double %silu.sigmoid, %silu.one
  %silu.input.product = fmul double %block.result, %silu.product
  %silu.derivative = fadd double %silu.sigmoid, %silu.input.product
  br label %activation.done
activation.elu:
  %elu.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 2
  %elu = load double, ptr addrspace(1) %elu.ptr, align 8
  %elu.test = fcmp ogt double %block.result, 0.0
  %elu.exp = call double @__ocml_exp_f64(double %block.result)
  %elu.expm1 = fsub double %elu.exp, 1.0
  %elu.negative = fmul double %elu, %elu.expm1
  %elu.result = select i1 %elu.test, double %block.result, double %elu.negative
  %elu.negative.derivative = fmul double %elu, %elu.exp
  %elu.derivative = select i1 %elu.test, double 1.0, double %elu.negative.derivative
  br label %activation.done
activation.prelu:
  %prelu.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 1
  %prelu = load double, ptr addrspace(1) %prelu.ptr, align 8
  %prelu.test = fcmp ogt double %block.result, 0.0
  %prelu.negative = fmul double %prelu, %block.result
  %prelu.result = select i1 %prelu.test, double %block.result, double %prelu.negative
  %prelu.derivative = select i1 %prelu.test, double 1.0, double %prelu
  br label %activation.done
activation.done:
  %activation.result = phi double [ %block.result, %activation.linear ], [ %cos.result, %activation.cos ], [ %exp.result, %activation.exp ], [ %log10.result, %activation.log10 ], [ %ln.result, %activation.ln ], [ %huber.result, %activation.huber ], [ %tan.result, %activation.tan ], [ %relu.result, %activation.relu ], [ %leak.result, %activation.leak ], [ %sigmoid.result, %activation.sigmoid ], [ %tanh.result, %activation.tanh ], [ %selu.result, %activation.selu ], [ %gelu.result, %activation.gelu ], [ %silu.result, %activation.silu ], [ %elu.result, %activation.elu ], [ %prelu.result, %activation.prelu ]
  %activation.derivative = phi double [ 1.0, %activation.linear ], [ %cos.derivative, %activation.cos ], [ %exp.result, %activation.exp ], [ %log10.derivative, %activation.log10 ], [ %ln.derivative, %activation.ln ], [ %huber.derivative, %activation.huber ], [ %tan.derivative, %activation.tan ], [ %relu.derivative, %activation.relu ], [ %leak.derivative, %activation.leak ], [ %sigmoid.derivative, %activation.sigmoid ], [ %tanh.derivative, %activation.tanh ], [ %selu.derivative, %activation.selu ], [ %gelu.derivative, %activation.gelu ], [ %silu.derivative, %activation.silu ], [ %elu.derivative, %activation.elu ], [ %prelu.derivative, %activation.prelu ]
  store double %activation.result, ptr addrspace(1) %value.ptr, align 8
  %activation.ptr = getelementptr inbounds double, ptr addrspace(1) %activation_derivatives, i32 %p
  store double %activation.derivative, ptr addrspace(1) %activation.ptr, align 8
  br label %exit
exit:
  ret void
}

define internal void @normalize_body(ptr addrspace(1) %values, ptr addrspace(1) nocapture readonly %reference, ptr addrspace(1) %scales, i32 %rows, i32 %width, i32 %mode, i32 %reverse, double %epsilon, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.group = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %group = select i1 %use.forced, i32 %forced, i32 %hardware.group
  %batch = icmp eq i32 %mode, 1
  %groups = select i1 %batch, i32 %width, i32 %rows
  %length = select i1 %batch, i32 %rows, i32 %width
  %active = icmp ult i32 %group, %groups
  br i1 %active, label %sum.loop, label %exit
sum.loop:
  %sum.item = phi i32 [ 0, %entry ], [ %sum.next, %sum.step ]
  %sum = phi double [ 0.0, %entry ], [ %sum.value, %sum.step ]
  %sum.more = icmp ult i32 %sum.item, %length
  br i1 %sum.more, label %sum.step, label %sum.done
sum.step:
  %sum.row.base = mul i32 %sum.item, %width
  %sum.batch.index = add i32 %sum.row.base, %group
  %sum.layer.base = mul i32 %group, %width
  %sum.layer.index = add i32 %sum.layer.base, %sum.item
  %sum.index = select i1 %batch, i32 %sum.batch.index, i32 %sum.layer.index
  %sum.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %sum.index
  %sum.loaded = load double, ptr addrspace(1) %sum.ptr, align 8
  %sum.value = fadd double %sum, %sum.loaded
  %sum.next = add nuw i32 %sum.item, 1
  br label %sum.loop
sum.done:
  %length.double = uitofp i32 %length to double
  %mean = fdiv double %sum, %length.double
  %backward = icmp ne i32 %reverse, 0
  br i1 %backward, label %projection.loop, label %variance.loop
variance.loop:
  %variance.item = phi i32 [ 0, %sum.done ], [ %variance.next, %variance.step ]
  %variance = phi double [ 0.0, %sum.done ], [ %variance.value, %variance.step ]
  %variance.more = icmp ult i32 %variance.item, %length
  br i1 %variance.more, label %variance.step, label %variance.done
variance.step:
  %variance.row.base = mul i32 %variance.item, %width
  %variance.batch.index = add i32 %variance.row.base, %group
  %variance.layer.base = mul i32 %group, %width
  %variance.layer.index = add i32 %variance.layer.base, %variance.item
  %variance.index = select i1 %batch, i32 %variance.batch.index, i32 %variance.layer.index
  %variance.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %variance.index
  %variance.loaded = load double, ptr addrspace(1) %variance.ptr, align 8
  %centered = fsub double %variance.loaded, %mean
  %centered.square = fmul double %centered, %centered
  %variance.value = fadd double %variance, %centered.square
  %variance.next = add nuw i32 %variance.item, 1
  br label %variance.loop
variance.done:
  %variance.mean = fdiv double %variance, %length.double
  %variance.adjusted = fadd double %variance.mean, %epsilon
  %scale = call double @llvm.sqrt.f64(double %variance.adjusted)
  %mean.ptr = getelementptr inbounds double, ptr addrspace(1) %scales, i32 %group
  store double %mean, ptr addrspace(1) %mean.ptr, align 8
  %scale.index = add i32 %groups, %group
  %scale.ptr = getelementptr inbounds double, ptr addrspace(1) %scales, i32 %scale.index
  store double %scale, ptr addrspace(1) %scale.ptr, align 8
  br label %forward.loop
forward.loop:
  %forward.item = phi i32 [ 0, %variance.done ], [ %forward.next, %forward.step ]
  %forward.more = icmp ult i32 %forward.item, %length
  br i1 %forward.more, label %forward.step, label %exit
forward.step:
  %forward.row.base = mul i32 %forward.item, %width
  %forward.batch.index = add i32 %forward.row.base, %group
  %forward.layer.base = mul i32 %group, %width
  %forward.layer.index = add i32 %forward.layer.base, %forward.item
  %forward.index = select i1 %batch, i32 %forward.batch.index, i32 %forward.layer.index
  %forward.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %forward.index
  %forward.loaded = load double, ptr addrspace(1) %forward.ptr, align 8
  %forward.centered = fsub double %forward.loaded, %mean
  %forward.value = fdiv double %forward.centered, %scale
  store double %forward.value, ptr addrspace(1) %forward.ptr, align 8
  %forward.next = add nuw i32 %forward.item, 1
  br label %forward.loop
projection.loop:
  %projection.item = phi i32 [ 0, %sum.done ], [ %projection.next, %projection.step ]
  %projection.sum = phi double [ 0.0, %sum.done ], [ %projection.value, %projection.step ]
  %projection.more = icmp ult i32 %projection.item, %length
  br i1 %projection.more, label %projection.step, label %projection.done
projection.step:
  %projection.row.base = mul i32 %projection.item, %width
  %projection.batch.index = add i32 %projection.row.base, %group
  %projection.layer.base = mul i32 %group, %width
  %projection.layer.index = add i32 %projection.layer.base, %projection.item
  %projection.index = select i1 %batch, i32 %projection.batch.index, i32 %projection.layer.index
  %gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %projection.index
  %gradient = load double, ptr addrspace(1) %gradient.ptr, align 8
  %normalized.ptr = getelementptr inbounds double, ptr addrspace(1) %reference, i32 %projection.index
  %normalized = load double, ptr addrspace(1) %normalized.ptr, align 8
  %projection.product = fmul double %gradient, %normalized
  %projection.value = fadd double %projection.sum, %projection.product
  %projection.next = add nuw i32 %projection.item, 1
  br label %projection.loop
projection.done:
  %projection.mean = fdiv double %projection.sum, %length.double
  %backward.scale.index = add i32 %groups, %group
  %backward.scale.ptr = getelementptr inbounds double, ptr addrspace(1) %scales, i32 %backward.scale.index
  %backward.scale = load double, ptr addrspace(1) %backward.scale.ptr, align 8
  br label %backward.loop
backward.loop:
  %backward.item = phi i32 [ 0, %projection.done ], [ %backward.next, %backward.step ]
  %backward.more = icmp ult i32 %backward.item, %length
  br i1 %backward.more, label %backward.step, label %exit
backward.step:
  %backward.row.base = mul i32 %backward.item, %width
  %backward.batch.index = add i32 %backward.row.base, %group
  %backward.layer.base = mul i32 %group, %width
  %backward.layer.index = add i32 %backward.layer.base, %backward.item
  %backward.index = select i1 %batch, i32 %backward.batch.index, i32 %backward.layer.index
  %backward.gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %backward.index
  %backward.gradient = load double, ptr addrspace(1) %backward.gradient.ptr, align 8
  %backward.normalized.ptr = getelementptr inbounds double, ptr addrspace(1) %reference, i32 %backward.index
  %backward.normalized = load double, ptr addrspace(1) %backward.normalized.ptr, align 8
  %backward.projected = fmul double %backward.normalized, %projection.mean
  %backward.centered = fsub double %backward.gradient, %mean
  %backward.numerator = fsub double %backward.centered, %backward.projected
  %backward.value = fdiv double %backward.numerator, %backward.scale
  store double %backward.value, ptr addrspace(1) %backward.gradient.ptr, align 8
  %backward.next = add nuw i32 %backward.item, 1
  br label %backward.loop
exit:
  ret void
}

define protected amdgpu_kernel void @normalize(ptr addrspace(1) %values, ptr addrspace(1) nocapture readonly %reference, ptr addrspace(1) %scales, i32 %rows, i32 %width, i32 %mode, i32 %reverse, double %epsilon, i32 %threads) #0 {
entry:
  call void @normalize_body(ptr addrspace(1) %values, ptr addrspace(1) %reference, ptr addrspace(1) %scales, i32 %rows, i32 %width, i32 %mode, i32 %reverse, double %epsilon, i32 %threads, i32 -1)
  ret void
}

define internal void @forward_body(ptr addrspace(1) nocapture readonly %samples, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture readonly %config, ptr addrspace(1) nocapture readonly %value_pointers, ptr addrspace(1) nocapture readonly %raw_pointers, ptr addrspace(1) nocapture readonly %operation_pointers, ptr addrspace(1) nocapture readonly %activation_pointers, ptr addrspace(1) nocapture readonly %scale_pointers, ptr addrspace(1) nocapture readonly %context_pointers, ptr addrspace(1) nocapture readonly %descriptors, ptr addrspace(1) nocapture readonly %parameters, i32 %rows, i32 %stages, double %epsilon, i32 %threads) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %first.group = icmp eq i32 %bid, 0
  br i1 %first.group, label %stage.loop, label %exit
stage.loop:
  %stage = phi i32 [ 0, %entry ], [ %stage.next, %stage.done ]
  %stage.more = icmp ult i32 %stage, %stages
  br i1 %stage.more, label %stage.load, label %exit
stage.load:
  %descriptor.base = mul i32 %stage, 7
  %from.index = add i32 %descriptor.base, 0
  %to.index = add i32 %descriptor.base, 1
  %weight.index = add i32 %descriptor.base, 2
  %operation.index = add i32 %descriptor.base, 3
  %activation.index = add i32 %descriptor.base, 4
  %normalization.index = add i32 %descriptor.base, 5
  %scale.index = add i32 %descriptor.base, 6
  %from.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %from.index
  %to.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %to.index
  %weight.offset.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %weight.index
  %operation.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %operation.index
  %activation.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %activation.index
  %normalization.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %normalization.index
  %from = load i32, ptr addrspace(1) %from.ptr, align 4
  %to = load i32, ptr addrspace(1) %to.ptr, align 4
  %weight.offset = load i32, ptr addrspace(1) %weight.offset.ptr, align 4
  %operation = load i32, ptr addrspace(1) %operation.ptr, align 4
  %activation = load i32, ptr addrspace(1) %activation.ptr, align 4
  %normalization = load i32, ptr addrspace(1) %normalization.ptr, align 4
  %parameter.base = mul i32 %stage, 2
  %secondary.index = add i32 %parameter.base, 1
  %parameter.ptr = getelementptr inbounds double, ptr addrspace(1) %parameters, i32 %parameter.base
  %secondary.ptr = getelementptr inbounds double, ptr addrspace(1) %parameters, i32 %secondary.index
  %parameter = load double, ptr addrspace(1) %parameter.ptr, align 8
  %secondary = load double, ptr addrspace(1) %secondary.ptr, align 8
  %value.slot = getelementptr inbounds i64, ptr addrspace(1) %value_pointers, i32 %stage
  %raw.slot = getelementptr inbounds i64, ptr addrspace(1) %raw_pointers, i32 %stage
  %operation.slot = getelementptr inbounds i64, ptr addrspace(1) %operation_pointers, i32 %stage
  %activation.slot = getelementptr inbounds i64, ptr addrspace(1) %activation_pointers, i32 %stage
  %scale.slot = getelementptr inbounds i64, ptr addrspace(1) %scale_pointers, i32 %stage
  %context.slot = getelementptr inbounds i64, ptr addrspace(1) %context_pointers, i32 %stage
  %value.address = load i64, ptr addrspace(1) %value.slot, align 8
  %raw.address = load i64, ptr addrspace(1) %raw.slot, align 8
  %operation.address = load i64, ptr addrspace(1) %operation.slot, align 8
  %activation.address = load i64, ptr addrspace(1) %activation.slot, align 8
  %scale.address = load i64, ptr addrspace(1) %scale.slot, align 8
  %context.address = load i64, ptr addrspace(1) %context.slot, align 8
  %values = inttoptr i64 %value.address to ptr addrspace(1)
  %raw = inttoptr i64 %raw.address to ptr addrspace(1)
  %operation.derivatives = inttoptr i64 %operation.address to ptr addrspace(1)
  %activation.derivatives = inttoptr i64 %activation.address to ptr addrspace(1)
  %scales = inttoptr i64 %scale.address to ptr addrspace(1)
  %context = inttoptr i64 %context.address to ptr addrspace(1)
  %first.stage = icmp eq i32 %stage, 0
  %previous.stage = sub i32 %stage, 1
  %safe.previous.stage = select i1 %first.stage, i32 0, i32 %previous.stage
  %previous.slot = getelementptr inbounds i64, ptr addrspace(1) %value_pointers, i32 %safe.previous.stage
  %previous.address = load i64, ptr addrspace(1) %previous.slot, align 8
  %previous.values = inttoptr i64 %previous.address to ptr addrspace(1)
  %source = select i1 %first.stage, ptr addrspace(1) %samples, ptr addrspace(1) %previous.values
  %matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.offset
  %count = mul i32 %rows, %to
  %is.embedding = icmp eq i32 %operation, 4
  %is.attention = icmp eq i32 %operation, 5
  br i1 %is.embedding, label %embedding.loop, label %attention.test
attention.test:
  br i1 %is.attention, label %attention.loop, label %value.loop
embedding.loop:
  %embedding.p = phi i32 [ %tid, %stage.load ], [ %embedding.next, %embedding.step ]
  %embedding.more = icmp ult i32 %embedding.p, %count
  br i1 %embedding.more, label %embedding.step, label %normalize.test
embedding.step:
  %vocabulary = fptoui double %parameter to i32
  call void @embedding_forward_body(ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values, i32 %embedding.p, i32 %from, i32 %to, i32 %vocabulary)
  %embedding.row = udiv i32 %embedding.p, %to
  %embedding.column = urem i32 %embedding.p, %to
  %embedding.row.base = mul i32 %embedding.row, %from
  %embedding.skip.column = urem i32 %embedding.column, %from
  %embedding.skip.index = add i32 %embedding.row.base, %embedding.skip.column
  %embedding.skip = getelementptr inbounds double, ptr addrspace(1) %source, i32 %embedding.skip.index
  %embedding.value = getelementptr inbounds double, ptr addrspace(1) %values, i32 %embedding.p
  %embedding.raw = getelementptr inbounds double, ptr addrspace(1) %raw, i32 %embedding.p
  %embedding.operation = getelementptr inbounds double, ptr addrspace(1) %operation.derivatives, i32 %embedding.p
  %embedding.activation = getelementptr inbounds double, ptr addrspace(1) %activation.derivatives, i32 %embedding.p
  call void @transform_body(ptr addrspace(1) %embedding.skip, ptr addrspace(1) %embedding.value, ptr addrspace(1) %embedding.raw, ptr addrspace(1) %embedding.operation, ptr addrspace(1) %embedding.activation, ptr addrspace(1) %config, i32 1, i32 %from, i32 %to, i32 0, i32 %activation, double %parameter, double %secondary, i32 1, i32 0)
  %embedding.next = add nuw i32 %embedding.p, %threads
  br label %embedding.loop
attention.loop:
  %attention.p = phi i32 [ %tid, %attention.test ], [ %attention.next, %attention.step ]
  %attention.more = icmp ult i32 %attention.p, %count
  br i1 %attention.more, label %attention.step, label %normalize.test
attention.step:
  %heads = fptoui double %parameter to i32
  call void @attention_forward_body(ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values, ptr addrspace(1) %context, i32 %attention.p, i32 %from, i32 %heads)
  %attention.value = getelementptr inbounds double, ptr addrspace(1) %values, i32 %attention.p
  %attention.raw = getelementptr inbounds double, ptr addrspace(1) %raw, i32 %attention.p
  %attention.operation = getelementptr inbounds double, ptr addrspace(1) %operation.derivatives, i32 %attention.p
  %attention.activation = getelementptr inbounds double, ptr addrspace(1) %activation.derivatives, i32 %attention.p
  call void @transform_body(ptr addrspace(1) %source, ptr addrspace(1) %attention.value, ptr addrspace(1) %attention.raw, ptr addrspace(1) %attention.operation, ptr addrspace(1) %attention.activation, ptr addrspace(1) %config, i32 1, i32 %from, i32 %to, i32 0, i32 %activation, double %parameter, double %secondary, i32 1, i32 0)
  %attention.next = add nuw i32 %attention.p, %threads
  br label %attention.loop
value.loop:
  %p = phi i32 [ %tid, %attention.test ], [ %p.next, %transform.step ]
  %value.more = icmp ult i32 %p, %count
  br i1 %value.more, label %value.step, label %normalize.test
value.step:
  %row = udiv i32 %p, %to
  %column = urem i32 %p, %to
  %row.base = mul i32 %row, %from
  %input.row = getelementptr inbounds double, ptr addrspace(1) %source, i32 %row.base
  %weight.column = getelementptr inbounds double, ptr addrspace(1) %matrix, i32 %column
  %value.element = getelementptr inbounds double, ptr addrspace(1) %values, i32 %p
  %skip.column = urem i32 %column, %from
  %skip.index = add i32 %row.base, %skip.column
  %skip = getelementptr inbounds double, ptr addrspace(1) %source, i32 %skip.index
  %raw.element = getelementptr inbounds double, ptr addrspace(1) %raw, i32 %p
  %operation.element = getelementptr inbounds double, ptr addrspace(1) %operation.derivatives, i32 %p
  %activation.element = getelementptr inbounds double, ptr addrspace(1) %activation.derivatives, i32 %p
  %is.conv = icmp eq i32 %operation, 1
  %is.pool = icmp eq i32 %operation, 2
  br i1 %is.conv, label %conv.step, label %pool.test
conv.step:
  %kernel = fptoui double %parameter to i32
  call void @conv_forward_body(ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values, i32 %p, i32 %from, i32 %to, i32 %kernel)
  br label %transform.step
pool.test:
  br i1 %is.pool, label %pool.step, label %dense.step
pool.step:
  %pool.size = fptoui double %parameter to i32
  call void @pool_forward_body(ptr addrspace(1) %source, ptr addrspace(1) %values, i32 %p, i32 %from, i32 %to, i32 %pool.size)
  br label %transform.step
dense.step:
  call void @dense_body(ptr addrspace(1) %input.row, ptr addrspace(1) %weight.column, ptr addrspace(1) %value.element, i32 1, i32 %from, i32 %to, i32 1, i32 0)
  br label %transform.step
transform.step:
  %special = or i1 %is.conv, %is.pool
  %transform.operation = select i1 %special, i32 0, i32 %operation
  call void @transform_body(ptr addrspace(1) %skip, ptr addrspace(1) %value.element, ptr addrspace(1) %raw.element, ptr addrspace(1) %operation.element, ptr addrspace(1) %activation.element, ptr addrspace(1) %config, i32 1, i32 %from, i32 %to, i32 %transform.operation, i32 %activation, double %parameter, double %secondary, i32 1, i32 0)
  %p.next = add nuw i32 %p, %threads
  br label %value.loop
normalize.test:
  call void @llvm.amdgcn.s.barrier()
  %normalized = icmp ne i32 %normalization, 0
  br i1 %normalized, label %normalize.loop, label %stage.done
normalize.loop:
  %group = phi i32 [ %tid, %normalize.test ], [ %group.next, %normalize.step ]
  %batch = icmp eq i32 %normalization, 1
  %groups = select i1 %batch, i32 %to, i32 %rows
  %group.more = icmp ult i32 %group, %groups
  br i1 %group.more, label %normalize.step, label %stage.done
normalize.step:
  call void @normalize_body(ptr addrspace(1) %values, ptr addrspace(1) %values, ptr addrspace(1) %scales, i32 %rows, i32 %to, i32 %normalization, i32 0, double %epsilon, i32 1, i32 %group)
  %group.next = add nuw i32 %group, %threads
  br label %normalize.loop
stage.done:
  call void @llvm.amdgcn.s.barrier()
  %stage.next = add nuw i32 %stage, 1
  br label %stage.loop
exit:
  ret void
}

define protected amdgpu_kernel void @forward_graph(ptr addrspace(1) nocapture readonly %samples, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture readonly %config, ptr addrspace(1) nocapture readonly %value_pointers, ptr addrspace(1) nocapture readonly %raw_pointers, ptr addrspace(1) nocapture readonly %operation_pointers, ptr addrspace(1) nocapture readonly %activation_pointers, ptr addrspace(1) nocapture readonly %scale_pointers, ptr addrspace(1) nocapture readonly %context_pointers, ptr addrspace(1) nocapture readonly %descriptors, ptr addrspace(1) nocapture readonly %parameters, i32 %rows, i32 %stages, double %epsilon, i32 %threads) #0 {
entry:
  call void @forward_body(ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %config, ptr addrspace(1) %value_pointers, ptr addrspace(1) %raw_pointers, ptr addrspace(1) %operation_pointers, ptr addrspace(1) %activation_pointers, ptr addrspace(1) %scale_pointers, ptr addrspace(1) %context_pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %parameters, i32 %rows, i32 %stages, double %epsilon, i32 %threads)
  ret void
}

define protected amdgpu_kernel void @epoch_graph(ptr addrspace(1) %samples, ptr addrspace(1) %targets, ptr addrspace(1) %weights, ptr addrspace(1) %config, ptr addrspace(1) %value_pointers, ptr addrspace(1) %raw_pointers, ptr addrspace(1) %operation_pointers, ptr addrspace(1) %activation_pointers, ptr addrspace(1) %scale_pointers, ptr addrspace(1) %context_pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %parameters, ptr addrspace(1) %metrics, ptr addrspace(1) %gradient, ptr addrspace(1) %delta_a, ptr addrspace(1) %delta_b, ptr addrspace(1) %moments, ptr addrspace(1) %variances, ptr addrspace(1) %checkpoint_weights, i32 %rows, i32 %stages, i32 %parameter_count, i32 %loss, double %previous_loss, double %tolerance, i32 %checkpoint_enabled, double %normalization_epsilon, double %rate, double %beta1, double %beta2, double %optimizer_epsilon, double %decay, i32 %step, i32 %threads) #0 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  call void @forward_body(ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %config, ptr addrspace(1) %value_pointers, ptr addrspace(1) %raw_pointers, ptr addrspace(1) %operation_pointers, ptr addrspace(1) %activation_pointers, ptr addrspace(1) %scale_pointers, ptr addrspace(1) %context_pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %parameters, i32 %rows, i32 %stages, double %normalization_epsilon, i32 %threads)
  br label %gradient.zero.loop
gradient.zero.loop:
  %zero.p = phi i32 [ %tid, %entry ], [ %zero.next, %gradient.zero.step ]
  %zero.more = icmp ult i32 %zero.p, %parameter_count
  br i1 %zero.more, label %gradient.zero.step, label %metrics.test
gradient.zero.step:
  %zero.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %zero.p
  store double 0.0, ptr addrspace(1) %zero.ptr, align 8
  %zero.next = add nuw i32 %zero.p, %threads
  br label %gradient.zero.loop
metrics.test:
  call void @llvm.amdgcn.s.barrier()
  %metric.thread = icmp eq i32 %tid, 0
  %last.stage = sub i32 %stages, 1
  br i1 %metric.thread, label %metrics.call, label %metrics.done
metrics.call:
  %prediction.slot = getelementptr inbounds i64, ptr addrspace(1) %value_pointers, i32 %last.stage
  %prediction.address = load i64, ptr addrspace(1) %prediction.slot, align 8
  %predictions = inttoptr i64 %prediction.address to ptr addrspace(1)
  call void @metrics_body(ptr addrspace(1) %predictions, ptr addrspace(1) %targets, ptr addrspace(1) %metrics, i32 %rows, i32 %loss)
  br label %metrics.done
metrics.done:
  call void @llvm.amdgcn.s.barrier()
  %prediction.slot.all = getelementptr inbounds i64, ptr addrspace(1) %value_pointers, i32 %last.stage
  %prediction.address.all = load i64, ptr addrspace(1) %prediction.slot.all, align 8
  %predictions.all = inttoptr i64 %prediction.address.all to ptr addrspace(1)
  %loss.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 0
  %loss.value = load double, ptr addrspace(1) %loss.ptr, align 8
  %checkpoint.factor = fsub double 2.0, %tolerance
  %checkpoint.threshold = fmul double %previous_loss, %checkpoint.factor
  %checkpoint.worse = fcmp ogt double %loss.value, %checkpoint.threshold
  %checkpoint.allowed = icmp ne i32 %checkpoint_enabled, 0
  %checkpoint = and i1 %checkpoint.allowed, %checkpoint.worse
  br i1 %metric.thread, label %checkpoint.flag.store, label %checkpoint.flag.done
checkpoint.flag.store:
  %checkpoint.flag.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 3
  %checkpoint.flag = select i1 %checkpoint, double 1.0, double 0.0
  store double %checkpoint.flag, ptr addrspace(1) %checkpoint.flag.ptr, align 8
  br label %checkpoint.flag.done
checkpoint.flag.done:
  call void @llvm.amdgcn.s.barrier()
  br i1 %checkpoint, label %checkpoint.loop, label %checkpoint.done
checkpoint.loop:
  %checkpoint.p = phi i32 [ %tid, %checkpoint.flag.done ], [ %checkpoint.next, %checkpoint.step ]
  %checkpoint.more = icmp ult i32 %checkpoint.p, %parameter_count
  br i1 %checkpoint.more, label %checkpoint.step, label %checkpoint.done
checkpoint.step:
  %checkpoint.source = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %checkpoint.p
  %checkpoint.destination = getelementptr inbounds double, ptr addrspace(1) %checkpoint_weights, i32 %checkpoint.p
  %checkpoint.value = load double, ptr addrspace(1) %checkpoint.source, align 8
  store double %checkpoint.value, ptr addrspace(1) %checkpoint.destination, align 8
  %checkpoint.next = add nuw i32 %checkpoint.p, %threads
  br label %checkpoint.loop
checkpoint.done:
  call void @llvm.amdgcn.s.barrier()
  %loss.absolute = call double @llvm.fabs.f64(double %loss.value)
  %loss.finite = fcmp olt double %loss.absolute, 0x7FF0000000000000
  br i1 %loss.finite, label %loss.gradient.loop, label %exit
loss.gradient.loop:
  %loss.p = phi i32 [ %tid, %checkpoint.done ], [ %loss.next, %loss.gradient.step ]
  %loss.more = icmp ult i32 %loss.p, %rows
  br i1 %loss.more, label %loss.gradient.step, label %backward.entry
loss.gradient.step:
  call void @loss_gradient_body(ptr addrspace(1) %predictions.all, ptr addrspace(1) %targets, ptr addrspace(1) %delta_a, i32 %rows, i32 %loss, double %loss.value, i32 1, i32 %loss.p)
  %loss.next = add nuw i32 %loss.p, %threads
  br label %loss.gradient.loop
backward.entry:
  call void @llvm.amdgcn.s.barrier()
  %backward.first = sub i32 %stages, 1
  br label %backward.loop
backward.loop:
  %backward.stage = phi i32 [ %backward.first, %backward.entry ], [ %backward.next, %backward.done ]
  %delta = phi ptr addrspace(1) [ %delta_a, %backward.entry ], [ %previous, %backward.done ]
  %previous = phi ptr addrspace(1) [ %delta_b, %backward.entry ], [ %delta, %backward.done ]
  %backward.more = icmp sge i32 %backward.stage, 0
  br i1 %backward.more, label %backward.load, label %optimizer.loop
backward.load:
  %backward.descriptor.base = mul i32 %backward.stage, 7
  %backward.to.index = add i32 %backward.descriptor.base, 1
  %backward.weight.index = add i32 %backward.descriptor.base, 2
  %backward.operation.index = add i32 %backward.descriptor.base, 3
  %backward.normalization.index = add i32 %backward.descriptor.base, 5
  %backward.prelu.index = add i32 %backward.descriptor.base, 6
  %backward.from.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %backward.descriptor.base
  %backward.to.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %backward.to.index
  %backward.weight.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %backward.weight.index
  %backward.operation.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %backward.operation.index
  %backward.normalization.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %backward.normalization.index
  %backward.prelu.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %backward.prelu.index
  %backward.from = load i32, ptr addrspace(1) %backward.from.ptr, align 4
  %backward.to = load i32, ptr addrspace(1) %backward.to.ptr, align 4
  %backward.weight.offset = load i32, ptr addrspace(1) %backward.weight.ptr, align 4
  %backward.operation = load i32, ptr addrspace(1) %backward.operation.ptr, align 4
  %backward.normalization = load i32, ptr addrspace(1) %backward.normalization.ptr, align 4
  %backward.prelu = load i32, ptr addrspace(1) %backward.prelu.ptr, align 4
  %backward.parameter.base = mul i32 %backward.stage, 2
  %backward.parameter.ptr = getelementptr inbounds double, ptr addrspace(1) %parameters, i32 %backward.parameter.base
  %backward.parameter = load double, ptr addrspace(1) %backward.parameter.ptr, align 8
  %backward.parameter.integer = fptoui double %backward.parameter to i32
  %backward.value.slot = getelementptr inbounds i64, ptr addrspace(1) %value_pointers, i32 %backward.stage
  %backward.raw.slot = getelementptr inbounds i64, ptr addrspace(1) %raw_pointers, i32 %backward.stage
  %backward.operation.slot = getelementptr inbounds i64, ptr addrspace(1) %operation_pointers, i32 %backward.stage
  %backward.activation.slot = getelementptr inbounds i64, ptr addrspace(1) %activation_pointers, i32 %backward.stage
  %backward.scale.slot = getelementptr inbounds i64, ptr addrspace(1) %scale_pointers, i32 %backward.stage
  %backward.context.slot = getelementptr inbounds i64, ptr addrspace(1) %context_pointers, i32 %backward.stage
  %backward.value.address = load i64, ptr addrspace(1) %backward.value.slot, align 8
  %backward.raw.address = load i64, ptr addrspace(1) %backward.raw.slot, align 8
  %backward.operation.address = load i64, ptr addrspace(1) %backward.operation.slot, align 8
  %backward.activation.address = load i64, ptr addrspace(1) %backward.activation.slot, align 8
  %backward.scale.address = load i64, ptr addrspace(1) %backward.scale.slot, align 8
  %backward.context.address = load i64, ptr addrspace(1) %backward.context.slot, align 8
  %backward.values = inttoptr i64 %backward.value.address to ptr addrspace(1)
  %backward.raw = inttoptr i64 %backward.raw.address to ptr addrspace(1)
  %backward.operations = inttoptr i64 %backward.operation.address to ptr addrspace(1)
  %backward.activations = inttoptr i64 %backward.activation.address to ptr addrspace(1)
  %backward.scales = inttoptr i64 %backward.scale.address to ptr addrspace(1)
  %backward.context = inttoptr i64 %backward.context.address to ptr addrspace(1)
  %backward.first.stage = icmp eq i32 %backward.stage, 0
  %backward.previous.stage = sub i32 %backward.stage, 1
  %backward.safe.stage = select i1 %backward.first.stage, i32 0, i32 %backward.previous.stage
  %backward.source.slot = getelementptr inbounds i64, ptr addrspace(1) %value_pointers, i32 %backward.safe.stage
  %backward.source.address = load i64, ptr addrspace(1) %backward.source.slot, align 8
  %backward.previous.values = inttoptr i64 %backward.source.address to ptr addrspace(1)
  %backward.source = select i1 %backward.first.stage, ptr addrspace(1) %samples, ptr addrspace(1) %backward.previous.values
  %backward.matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %backward.weight.offset
  %backward.matrix.gradient = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %backward.weight.offset
  %has.normalization = icmp ne i32 %backward.normalization, 0
  br i1 %has.normalization, label %normalization.loop, label %normalization.done
normalization.loop:
  %normalization.group = phi i32 [ %tid, %backward.load ], [ %normalization.next, %normalization.step ]
  %normalization.batch = icmp eq i32 %backward.normalization, 1
  %normalization.groups = select i1 %normalization.batch, i32 %backward.to, i32 %rows
  %normalization.more = icmp ult i32 %normalization.group, %normalization.groups
  br i1 %normalization.more, label %normalization.step, label %normalization.done
normalization.step:
  call void @normalize_body(ptr addrspace(1) %delta, ptr addrspace(1) %backward.values, ptr addrspace(1) %backward.scales, i32 %rows, i32 %backward.to, i32 %backward.normalization, i32 1, double %normalization_epsilon, i32 1, i32 %normalization.group)
  %normalization.next = add nuw i32 %normalization.group, %threads
  br label %normalization.loop
normalization.done:
  call void @llvm.amdgcn.s.barrier()
  %has.prelu = icmp sge i32 %backward.prelu, 0
  %prelu.thread = and i1 %has.prelu, %metric.thread
  br i1 %prelu.thread, label %prelu.call, label %prelu.done
prelu.call:
  %prelu.gradient = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %backward.prelu
  %backward.count = mul i32 %rows, %backward.to
  call void @prelu_gradient_body(ptr addrspace(1) %backward.raw, ptr addrspace(1) %delta, ptr addrspace(1) %prelu.gradient, i32 %backward.count)
  br label %prelu.done
prelu.done:
  call void @llvm.amdgcn.s.barrier()
  %chain.count = mul i32 %rows, %backward.to
  br label %chain.loop
chain.loop:
  %chain.p = phi i32 [ %tid, %prelu.done ], [ %chain.next, %chain.step ]
  %chain.more = icmp ult i32 %chain.p, %chain.count
  br i1 %chain.more, label %chain.step, label %matrix.dispatch
chain.step:
  call void @chain_body(ptr addrspace(1) %delta, ptr addrspace(1) %backward.activations, ptr addrspace(1) %backward.operations, i32 %chain.count, i32 1, i32 %chain.p)
  %chain.next = add nuw i32 %chain.p, %threads
  br label %chain.loop
matrix.dispatch:
  call void @llvm.amdgcn.s.barrier()
  %matrix.conv = icmp eq i32 %backward.operation, 1
  %matrix.pool = icmp eq i32 %backward.operation, 2
  %matrix.embedding = icmp eq i32 %backward.operation, 4
  %matrix.attention = icmp eq i32 %backward.operation, 5
  br i1 %matrix.conv, label %conv.matrix.loop, label %pool.matrix.test
pool.matrix.test:
  br i1 %matrix.pool, label %previous.dispatch, label %embedding.matrix.test
embedding.matrix.test:
  br i1 %matrix.embedding, label %embedding.matrix.loop, label %attention.matrix.test
attention.matrix.test:
  br i1 %matrix.attention, label %attention.matrix.loop, label %matrix.loop
conv.matrix.loop:
  %conv.positions = sub i32 %backward.from, %backward.parameter.integer
  %conv.positions.one = add i32 %conv.positions, 1
  %conv.filters = udiv i32 %backward.to, %conv.positions.one
  %conv.matrix.count = mul i32 %conv.filters, %backward.parameter.integer
  br label %conv.matrix.iterate
conv.matrix.iterate:
  %conv.matrix.p = phi i32 [ %tid, %conv.matrix.loop ], [ %conv.matrix.next, %conv.matrix.step ]
  %conv.matrix.more = icmp ult i32 %conv.matrix.p, %conv.matrix.count
  br i1 %conv.matrix.more, label %conv.matrix.step, label %previous.dispatch
conv.matrix.step:
  call void @conv_weight_gradient_body(ptr addrspace(1) %backward.source, ptr addrspace(1) %delta, ptr addrspace(1) %backward.matrix.gradient, i32 %conv.matrix.p, i32 %rows, i32 %backward.from, i32 %backward.to, i32 %backward.parameter.integer)
  %conv.matrix.next = add nuw i32 %conv.matrix.p, %threads
  br label %conv.matrix.iterate
embedding.matrix.loop:
  %embedding.dimensions = udiv i32 %backward.to, %backward.from
  %embedding.vocabulary = add i32 %backward.parameter.integer, 0
  %embedding.matrix.count = mul i32 %embedding.dimensions, %embedding.vocabulary
  br label %embedding.matrix.iterate
embedding.matrix.iterate:
  %embedding.matrix.p = phi i32 [ %tid, %embedding.matrix.loop ], [ %embedding.matrix.next, %embedding.matrix.step ]
  %embedding.matrix.more = icmp ult i32 %embedding.matrix.p, %embedding.matrix.count
  br i1 %embedding.matrix.more, label %embedding.matrix.step, label %previous.dispatch
embedding.matrix.step:
  call void @embedding_gradient_body(ptr addrspace(1) %backward.source, ptr addrspace(1) %delta, ptr addrspace(1) %backward.matrix.gradient, i32 %embedding.matrix.p, i32 %rows, i32 %backward.from, i32 %backward.to, i32 %embedding.vocabulary)
  %embedding.matrix.next = add nuw i32 %embedding.matrix.p, %threads
  br label %embedding.matrix.iterate
attention.matrix.loop:
  %attention.matrix.count = mul i32 %backward.parameter.integer, 4
  br label %attention.matrix.iterate
attention.matrix.iterate:
  %attention.matrix.p = phi i32 [ %tid, %attention.matrix.loop ], [ %attention.matrix.next, %attention.matrix.step ]
  %attention.matrix.more = icmp ult i32 %attention.matrix.p, %attention.matrix.count
  br i1 %attention.matrix.more, label %attention.matrix.step, label %previous.dispatch
attention.matrix.step:
  call void @attention_weight_gradient_body(ptr addrspace(1) %backward.source, ptr addrspace(1) %backward.matrix, ptr addrspace(1) %delta, ptr addrspace(1) %backward.context, ptr addrspace(1) %backward.matrix.gradient, i32 %attention.matrix.p, i32 %rows, i32 %backward.from, i32 %backward.parameter.integer)
  %attention.matrix.next = add nuw i32 %attention.matrix.p, %threads
  br label %attention.matrix.iterate
matrix.loop:
  %matrix.count = mul i32 %backward.from, %backward.to
  br label %matrix.iterate
matrix.iterate:
  %matrix.p = phi i32 [ %tid, %matrix.loop ], [ %matrix.next, %matrix.step ]
  %matrix.more = icmp ult i32 %matrix.p, %matrix.count
  br i1 %matrix.more, label %matrix.step, label %previous.dispatch
matrix.step:
  call void @weight_gradient_body(ptr addrspace(1) %backward.source, ptr addrspace(1) %delta, ptr addrspace(1) %backward.matrix.gradient, i32 %rows, i32 %backward.from, i32 %backward.to, i32 1, i32 %matrix.p)
  %matrix.next = add nuw i32 %matrix.p, %threads
  br label %matrix.iterate
previous.dispatch:
  call void @llvm.amdgcn.s.barrier()
  %previous.count = mul i32 %rows, %backward.from
  br i1 %matrix.conv, label %conv.previous.loop, label %pool.previous.test
pool.previous.test:
  br i1 %matrix.pool, label %pool.previous.loop, label %embedding.previous.test
embedding.previous.test:
  br i1 %matrix.embedding, label %embedding.previous.loop, label %attention.previous.test
attention.previous.test:
  br i1 %matrix.attention, label %attention.previous.loop, label %previous.loop
conv.previous.loop:
  br label %conv.previous.iterate
conv.previous.iterate:
  %conv.previous.p = phi i32 [ %tid, %conv.previous.loop ], [ %conv.previous.next, %conv.previous.step ]
  %conv.previous.more = icmp ult i32 %conv.previous.p, %previous.count
  br i1 %conv.previous.more, label %conv.previous.step, label %residual.test
conv.previous.step:
  call void @conv_previous_gradient_body(ptr addrspace(1) %backward.matrix, ptr addrspace(1) %delta, ptr addrspace(1) %previous, i32 %conv.previous.p, i32 %backward.from, i32 %backward.to, i32 %backward.parameter.integer)
  %conv.previous.next = add nuw i32 %conv.previous.p, %threads
  br label %conv.previous.iterate
pool.previous.loop:
  br label %pool.previous.iterate
pool.previous.iterate:
  %pool.previous.p = phi i32 [ %tid, %pool.previous.loop ], [ %pool.previous.next, %pool.previous.step ]
  %pool.previous.more = icmp ult i32 %pool.previous.p, %previous.count
  br i1 %pool.previous.more, label %pool.previous.step, label %residual.test
pool.previous.step:
  call void @pool_previous_gradient_body(ptr addrspace(1) %backward.source, ptr addrspace(1) %delta, ptr addrspace(1) %previous, i32 %pool.previous.p, i32 %backward.from, i32 %backward.to, i32 %backward.parameter.integer)
  %pool.previous.next = add nuw i32 %pool.previous.p, %threads
  br label %pool.previous.iterate
embedding.previous.loop:
  br label %embedding.previous.iterate
embedding.previous.iterate:
  %embedding.previous.p = phi i32 [ %tid, %embedding.previous.loop ], [ %embedding.previous.next, %embedding.previous.step ]
  %embedding.previous.more = icmp ult i32 %embedding.previous.p, %previous.count
  br i1 %embedding.previous.more, label %embedding.previous.step, label %residual.test
embedding.previous.step:
  %embedding.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %embedding.previous.p
  store double 0.0, ptr addrspace(1) %embedding.previous.ptr, align 8
  %embedding.previous.next = add nuw i32 %embedding.previous.p, %threads
  br label %embedding.previous.iterate
attention.previous.loop:
  br label %attention.previous.iterate
attention.previous.iterate:
  %attention.previous.p = phi i32 [ %tid, %attention.previous.loop ], [ %attention.previous.next, %attention.previous.step ]
  %attention.previous.more = icmp ult i32 %attention.previous.p, %previous.count
  br i1 %attention.previous.more, label %attention.previous.step, label %residual.test
attention.previous.step:
  call void @attention_previous_gradient_body(ptr addrspace(1) %backward.source, ptr addrspace(1) %backward.matrix, ptr addrspace(1) %delta, ptr addrspace(1) %backward.context, ptr addrspace(1) %previous, i32 %attention.previous.p, i32 %backward.from, i32 %backward.parameter.integer)
  %attention.previous.next = add nuw i32 %attention.previous.p, %threads
  br label %attention.previous.iterate
previous.loop:
  br label %previous.iterate
previous.iterate:
  %previous.p = phi i32 [ %tid, %previous.loop ], [ %previous.next, %previous.step ]
  %previous.more = icmp ult i32 %previous.p, %previous.count
  br i1 %previous.more, label %previous.step, label %residual.test
previous.step:
  call void @previous_gradient_body(ptr addrspace(1) %backward.matrix, ptr addrspace(1) %delta, ptr addrspace(1) %previous, i32 %rows, i32 %backward.from, i32 %backward.to, i32 1, i32 %previous.p)
  %previous.next = add nuw i32 %previous.p, %threads
  br label %previous.iterate
residual.test:
  call void @llvm.amdgcn.s.barrier()
  %is.residual = icmp eq i32 %backward.operation, 11
  br i1 %is.residual, label %residual.loop, label %backward.done
residual.loop:
  %residual.p = phi i32 [ %tid, %residual.test ], [ %residual.next, %residual.step ]
  %residual.more = icmp ult i32 %residual.p, %previous.count
  br i1 %residual.more, label %residual.step, label %backward.done
residual.step:
  call void @residual_gradient_body(ptr addrspace(1) %previous, ptr addrspace(1) %delta, i32 %rows, i32 %backward.from, i32 %backward.to, i32 1, i32 %residual.p)
  %residual.next = add nuw i32 %residual.p, %threads
  br label %residual.loop
backward.done:
  call void @llvm.amdgcn.s.barrier()
  %backward.next = sub i32 %backward.stage, 1
  br label %backward.loop
optimizer.loop:
  %optimizer.p = phi i32 [ %tid, %backward.loop ], [ %optimizer.next, %optimizer.step ]
  %optimizer.more = icmp ult i32 %optimizer.p, %parameter_count
  br i1 %optimizer.more, label %optimizer.step, label %exit
optimizer.step:
  call void @adamw_body(ptr addrspace(1) %weights, ptr addrspace(1) %gradient, ptr addrspace(1) %moments, ptr addrspace(1) %variances, i32 %parameter_count, i32 1, double %rate, double %beta1, double %beta2, double %optimizer_epsilon, double %decay, i32 %step, i32 %optimizer.p)
  %optimizer.next = add nuw i32 %optimizer.p, %threads
  br label %optimizer.loop
exit:
  ret void
}

define internal void @chain_body(ptr addrspace(1) %delta, ptr addrspace(1) nocapture readonly %activation, ptr addrspace(1) nocapture readonly %operation, i32 %count, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.p = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %p = select i1 %use.forced, i32 %forced, i32 %hardware.p
  %active = icmp ult i32 %p, %count
  br i1 %active, label %body, label %exit
body:
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %p
  %activation.ptr = getelementptr inbounds double, ptr addrspace(1) %activation, i32 %p
  %operation.ptr = getelementptr inbounds double, ptr addrspace(1) %operation, i32 %p
  %delta.value = load double, ptr addrspace(1) %delta.ptr, align 8
  %activation.value = load double, ptr addrspace(1) %activation.ptr, align 8
  %operation.value = load double, ptr addrspace(1) %operation.ptr, align 8
  %first = fmul double %delta.value, %activation.value
  %result = fmul double %first, %operation.value
  store double %result, ptr addrspace(1) %delta.ptr, align 8
  br label %exit
exit:
  ret void
}

define protected amdgpu_kernel void @affine(ptr addrspace(1) %values, i32 %count, double %scale, double %offset, i32 %threads) #0 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %p = add i32 %base, %tid
  %active = icmp ult i32 %p, %count
  br i1 %active, label %body, label %exit
body:
  %ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %p
  %value = load double, ptr addrspace(1) %ptr, align 8
  %scaled = fmul double %value, %scale
  %result = fadd double %scaled, %offset
  store double %result, ptr addrspace(1) %ptr, align 8
  br label %exit
exit:
  ret void
}

define protected amdgpu_kernel void @initialize(ptr addrspace(1) nocapture writeonly %values, i32 %count, i64 %seed, double %scale, i32 %threads) #0 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %p = add i32 %base, %tid
  %active = icmp ult i32 %p, %count
  br i1 %active, label %body, label %exit
body:
  %p64 = zext i32 %p to i64
  %seeded = add i64 %seed, %p64
  %z0 = add i64 %seeded, -7046029254386353131
  %r0 = lshr i64 %z0, 30
  %x0 = xor i64 %z0, %r0
  %z1 = mul i64 %x0, -4658895280553007687
  %r1 = lshr i64 %z1, 27
  %x1 = xor i64 %z1, %r1
  %z2 = mul i64 %x1, -7723592293110705685
  %r2 = lshr i64 %z2, 31
  %random = xor i64 %z2, %r2
  %mantissa = lshr i64 %random, 11
  %mantissa.double = uitofp i64 %mantissa to double
  %unit = fdiv double %mantissa.double, 9.007199254740992e+15
  %twice = fmul double %unit, 2.0
  %signed = fsub double %twice, 1.0
  %value = fmul double %scale, %signed
  %ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %p
  store double %value, ptr addrspace(1) %ptr, align 8
  br label %exit
exit:
  ret void
}

define protected amdgpu_kernel void @set_value(ptr addrspace(1) %values, i32 %index, double %value) #0 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %only.thread = icmp eq i32 %tid, 0
  %only.block = icmp eq i32 %bid, 0
  %active = and i1 %only.thread, %only.block
  br i1 %active, label %body, label %exit
body:
  %ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %index
  store double %value, ptr addrspace(1) %ptr, align 8
  br label %exit
exit:
  ret void
}

define internal void @prelu_gradient_body(ptr addrspace(1) nocapture readonly %raw, ptr addrspace(1) nocapture readonly %delta, ptr addrspace(1) nocapture writeonly %gradient, i32 %count) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %only.thread = icmp eq i32 %tid, 0
  %only.block = icmp eq i32 %bid, 0
  %active = and i1 %only.thread, %only.block
  br i1 %active, label %loop, label %exit
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %step ]
  %sum = phi double [ 0.0, %entry ], [ %sum.next, %step ]
  %more = icmp ult i32 %i, %count
  br i1 %more, label %step, label %done
step:
  %raw.ptr = getelementptr inbounds double, ptr addrspace(1) %raw, i32 %i
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %i
  %raw.value = load double, ptr addrspace(1) %raw.ptr, align 8
  %delta.value = load double, ptr addrspace(1) %delta.ptr, align 8
  %negative = fcmp olt double %raw.value, 0.0
  %product = fmul double %raw.value, %delta.value
  %term = select i1 %negative, double %product, double 0.0
  %sum.next = fadd double %sum, %term
  %next = add nuw i32 %i, 1
  br label %loop
done:
  store double %sum, ptr addrspace(1) %gradient, align 8
  br label %exit
exit:
  ret void
}

define internal void @residual_gradient_body(ptr addrspace(1) %previous, ptr addrspace(1) nocapture readonly %delta, i32 %rows, i32 %from, i32 %to, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.p = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %p = select i1 %use.forced, i32 %forced, i32 %hardware.p
  %count = mul i32 %rows, %from
  %active = icmp ult i32 %p, %count
  br i1 %active, label %body, label %exit
body:
  %row = udiv i32 %p, %from
  %column = urem i32 %p, %from
  %previous.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %p
  %initial = load double, ptr addrspace(1) %previous.ptr, align 8
  br label %loop
loop:
  %output = phi i32 [ %column, %body ], [ %next, %step ]
  %sum = phi double [ %initial, %body ], [ %sum.next, %step ]
  %more = icmp ult i32 %output, %to
  br i1 %more, label %step, label %done
step:
  %row.base = mul i32 %row, %to
  %index = add i32 %row.base, %output
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %index
  %value = load double, ptr addrspace(1) %delta.ptr, align 8
  %sum.next = fadd double %sum, %value
  %next = add i32 %output, %from
  br label %loop
done:
  store double %sum, ptr addrspace(1) %previous.ptr, align 8
  br label %exit
exit:
  ret void
}

define internal void @metrics_body(ptr addrspace(1) nocapture readonly %predictions, ptr addrspace(1) nocapture readonly %targets, ptr addrspace(1) nocapture writeonly %metrics, i32 %rows, i32 %loss) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %only.thread = icmp eq i32 %tid, 0
  %only.block = icmp eq i32 %bid, 0
  %active = and i1 %only.thread, %only.block
  br i1 %active, label %mean.loop, label %exit
mean.loop:
  %mean.i = phi i32 [ 0, %entry ], [ %mean.next, %mean.step ]
  %mean.sum = phi double [ 0.0, %entry ], [ %mean.value, %mean.step ]
  %mean.more = icmp ult i32 %mean.i, %rows
  br i1 %mean.more, label %mean.step, label %mean.done
mean.step:
  %mean.ptr = getelementptr inbounds double, ptr addrspace(1) %targets, i32 %mean.i
  %mean.loaded = load double, ptr addrspace(1) %mean.ptr, align 8
  %mean.value = fadd double %mean.sum, %mean.loaded
  %mean.next = add nuw i32 %mean.i, 1
  br label %mean.loop
mean.done:
  %rows.double = uitofp i32 %rows to double
  %mean = fdiv double %mean.sum, %rows.double
  br label %metric.loop
metric.loop:
  %i = phi i32 [ 0, %mean.done ], [ %next, %metric.done ]
  %loss.sum = phi double [ 0.0, %mean.done ], [ %loss.next, %metric.done ]
  %sse = phi double [ 0.0, %mean.done ], [ %sse.next, %metric.done ]
  %total = phi double [ 0.0, %mean.done ], [ %total.next, %metric.done ]
  %more = icmp ult i32 %i, %rows
  br i1 %more, label %metric.step, label %finish
metric.step:
  %prediction.ptr = getelementptr inbounds double, ptr addrspace(1) %predictions, i32 %i
  %target.ptr = getelementptr inbounds double, ptr addrspace(1) %targets, i32 %i
  %prediction = load double, ptr addrspace(1) %prediction.ptr, align 8
  %target = load double, ptr addrspace(1) %target.ptr, align 8
  %difference = fsub double %prediction, %target
  %square = fmul double %difference, %difference
  %sse.next = fadd double %sse, %square
  %target.centered = fsub double %target, %mean
  %target.square = fmul double %target.centered, %target.centered
  %total.next = fadd double %total, %target.square
  switch i32 %loss, label %loss.focal [ i32 0, label %loss.mse i32 1, label %loss.mse i32 2, label %loss.huber i32 3, label %loss.mae i32 4, label %loss.bce i32 5, label %loss.bce ]
loss.mse:
  br label %metric.done
loss.huber:
  %absolute = call double @llvm.fabs.f64(double %difference)
  %small = fcmp ole double %absolute, 1.0
  %half.square = fmul double 0.5, %square
  %large = fsub double %absolute, 0.5
  %huber = select i1 %small, double %half.square, double %large
  br label %metric.done
loss.mae:
  %mae = call double @llvm.fabs.f64(double %difference)
  br label %metric.done
loss.bce:
  %negative = fneg double %prediction
  %exponential = call double @__ocml_exp_f64(double %negative)
  %denominator = fadd double 1.0, %exponential
  %probability.raw = fdiv double 1.0, %denominator
  %probability.low = fcmp olt double %probability.raw, 0x3CB0000000000000
  %probability.lowered = select i1 %probability.low, double 0x3CB0000000000000, double %probability.raw
  %probability.high = fcmp ogt double %probability.lowered, 0x3FEFFFFFFFFFFFFE
  %probability = select i1 %probability.high, double 0x3FEFFFFFFFFFFFFE, double %probability.lowered
  %target.low = fcmp olt double %target, 0.0
  %target.lowered = select i1 %target.low, double 0.0, double %target
  %target.high = fcmp ogt double %target.lowered, 1.0
  %target.clamped = select i1 %target.high, double 1.0, double %target.lowered
  %log.probability = call double @__ocml_log_f64(double %probability)
  %one.probability = fsub double 1.0, %probability
  %log.one.probability = call double @__ocml_log_f64(double %one.probability)
  %first = fmul double %target.clamped, %log.probability
  %one.target = fsub double 1.0, %target.clamped
  %second = fmul double %one.target, %log.one.probability
  %cross.sum = fadd double %first, %second
  %cross = fneg double %cross.sum
  br label %metric.done
loss.focal:
  %focal.negative = fneg double %prediction
  %focal.exp = call double @__ocml_exp_f64(double %focal.negative)
  %focal.denominator = fadd double 1.0, %focal.exp
  %focal.raw = fdiv double 1.0, %focal.denominator
  %focal.low = fcmp olt double %focal.raw, 0x3CB0000000000000
  %focal.lowered = select i1 %focal.low, double 0x3CB0000000000000, double %focal.raw
  %focal.high = fcmp ogt double %focal.lowered, 0x3FEFFFFFFFFFFFFE
  %focal.probability = select i1 %focal.high, double 0x3FEFFFFFFFFFFFFE, double %focal.lowered
  %focal.target = fcmp oge double %target, 0.5
  %focal.one = fsub double 1.0, %focal.probability
  %correct = select i1 %focal.target, double %focal.probability, double %focal.one
  %incorrect = fsub double 1.0, %correct
  %incorrect.square = fmul double %incorrect, %incorrect
  %correct.log = call double @__ocml_log_f64(double %correct)
  %focal.product = fmul double %incorrect.square, %correct.log
  %focal = fneg double %focal.product
  br label %metric.done
metric.done:
  %item.loss = phi double [ %square, %loss.mse ], [ %huber, %loss.huber ], [ %mae, %loss.mae ], [ %cross, %loss.bce ], [ %focal, %loss.focal ]
  %loss.next = fadd double %loss.sum, %item.loss
  %next = add nuw i32 %i, 1
  br label %metric.loop
finish:
  %mean.loss = fdiv double %loss.sum, %rows.double
  %is.rmse = icmp eq i32 %loss, 1
  %root.loss = call double @llvm.sqrt.f64(double %mean.loss)
  %final.loss = select i1 %is.rmse, double %root.loss, double %mean.loss
  %ratio = fdiv double %sse, %total
  %r2 = fsub double 1.0, %ratio
  %loss.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 0
  %sse.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 1
  %r2.ptr = getelementptr inbounds double, ptr addrspace(1) %metrics, i32 2
  store double %final.loss, ptr addrspace(1) %loss.ptr, align 8
  store double %sse, ptr addrspace(1) %sse.ptr, align 8
  store double %r2, ptr addrspace(1) %r2.ptr, align 8
  br label %exit
exit:
  ret void
}

define protected amdgpu_kernel void @metrics(ptr addrspace(1) nocapture readonly %predictions, ptr addrspace(1) nocapture readonly %targets, ptr addrspace(1) nocapture writeonly %metrics, i32 %rows, i32 %loss) #0 {
entry:
  call void @metrics_body(ptr addrspace(1) %predictions, ptr addrspace(1) %targets, ptr addrspace(1) %metrics, i32 %rows, i32 %loss)
  ret void
}

define internal void @loss_gradient_body(ptr addrspace(1) nocapture readonly %predictions, ptr addrspace(1) nocapture readonly %targets, ptr addrspace(1) nocapture writeonly %gradient, i32 %rows, i32 %loss, double %loss_value, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.amdgcn.workitem.id.x()
  %bid = call i32 @llvm.amdgcn.workgroup.id.x()
  %base = mul i32 %bid, %threads
  %hardware.p = add i32 %base, %tid
  %use.forced = icmp sge i32 %forced, 0
  %p = select i1 %use.forced, i32 %forced, i32 %hardware.p
  %active = icmp ult i32 %p, %rows
  br i1 %active, label %body, label %exit
body:
  %prediction.ptr = getelementptr inbounds double, ptr addrspace(1) %predictions, i32 %p
  %target.ptr = getelementptr inbounds double, ptr addrspace(1) %targets, i32 %p
  %prediction = load double, ptr addrspace(1) %prediction.ptr, align 8
  %target = load double, ptr addrspace(1) %target.ptr, align 8
  %difference = fsub double %prediction, %target
  %rows.double = uitofp i32 %rows to double
  %base.gradient = fdiv double %difference, %rows.double
  switch i32 %loss, label %done [ i32 0, label %mse i32 1, label %rmse i32 2, label %huber i32 3, label %mae ]
mse:
  %mse.gradient = fmul double %base.gradient, 2.0
  br label %done
rmse:
  %rmse.denominator = fmul double %rows.double, %loss_value
  %rmse.zero = fcmp oeq double %loss_value, 0.0
  %rmse.divided = fdiv double %difference, %rmse.denominator
  %rmse.gradient = select i1 %rmse.zero, double 0.0, double %rmse.divided
  br label %done
huber:
  %huber.low = fcmp olt double %difference, -1.0
  %huber.high = fcmp ogt double %difference, 1.0
  %huber.lowered = select i1 %huber.low, double -1.0, double %difference
  %huber.clamped = select i1 %huber.high, double 1.0, double %huber.lowered
  %huber.gradient = fdiv double %huber.clamped, %rows.double
  br label %done
mae:
  %mae.negative = fcmp olt double %difference, 0.0
  %mae.positive = fcmp ogt double %difference, 0.0
  %mae.upper = select i1 %mae.positive, double 1.0, double 0.0
  %mae.sign = select i1 %mae.negative, double -1.0, double %mae.upper
  %mae.gradient = fdiv double %mae.sign, %rows.double
  br label %done
done:
  %result = phi double [ %base.gradient, %body ], [ %mse.gradient, %mse ], [ %rmse.gradient, %rmse ], [ %huber.gradient, %huber ], [ %mae.gradient, %mae ]
  %gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %p
  store double %result, ptr addrspace(1) %gradient.ptr, align 8
  br label %exit
exit:
  ret void
}

attributes #0 = { nounwind "amdgpu-flat-work-group-size"="1,1024" }
attributes #1 = { alwaysinline nounwind }
