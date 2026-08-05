target triple = "nvptx64-nvidia-cuda"
declare i32 @llvm.nvvm.read.ptx.sreg.tid.x()
declare i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
declare void @llvm.nvvm.barrier0()
declare double @llvm.sqrt.f64(double)
declare double @__nv_exp(double)
declare double @__nv_log(double)
declare double @__nv_sin(double)
declare double @__nv_cos(double)
declare double @__nv_tanh(double)
declare double @llvm.fabs.f64(double)

define internal void @dense_body(ptr addrspace(1) nocapture readonly %input, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture writeonly %output, i32 %rows, i32 %from, i32 %to, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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

define internal void @kmeans_forward_body(ptr addrspace(1) %input, ptr addrspace(1) %output, ptr addrspace(1) %context, i32 %rows, i32 %from, i32 %clusters, i32 %threads) #1 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %flag = load double, ptr addrspace(1) %context, align 8
  %initialized = fcmp oeq double %flag, 1.0
  br i1 %initialized, label %distance.loop, label %initialize.loop
initialize.loop:
  %ip = phi i32 [ %tid, %entry ], [ %ip.next, %initialize.step ]
  %centroid.count = mul i32 %clusters, %from
  %initialize.more = icmp ult i32 %ip, %centroid.count
  br i1 %initialize.more, label %initialize.step, label %initialize.done
initialize.step:
  %ic = udiv i32 %ip, %from
  %if = urem i32 %ip, %from
  %ir = urem i32 %ic, %rows
  %ir.base = mul i32 %ir, %from
  %ix = add i32 %ir.base, %if
  %ix.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %ix
  %iv = load double, ptr addrspace(1) %ix.ptr, align 8
  %icx = add i32 %ip, 1
  %icx.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %icx
  store double %iv, ptr addrspace(1) %icx.ptr, align 8
  %ip.next = add i32 %ip, %threads
  br label %initialize.loop
initialize.done:
  call void @llvm.nvvm.barrier0()
  %first = icmp eq i32 %tid, 0
  br i1 %first, label %flag.store, label %flag.done
flag.store:
  store double 1.0, ptr addrspace(1) %context, align 8
  br label %flag.done
flag.done:
  call void @llvm.nvvm.barrier0()
  br label %distance.loop
distance.loop:
  %p = phi i32 [ %tid, %entry ], [ %tid, %flag.done ], [ %p.next, %distance.done ]
  %distance.count = mul i32 %rows, %clusters
  %distance.more = icmp ult i32 %p, %distance.count
  br i1 %distance.more, label %distance.start, label %exit
distance.start:
  %row = udiv i32 %p, %clusters
  %cluster = urem i32 %p, %clusters
  %row.base = mul i32 %row, %from
  %centroid.base.0 = mul i32 %cluster, %from
  %centroid.base = add i32 %centroid.base.0, 1
  br label %feature.loop
feature.loop:
  %feature = phi i32 [ 0, %distance.start ], [ %feature.next, %feature.step ]
  %square.sum = phi double [ 0.0, %distance.start ], [ %square.next, %feature.step ]
  %feature.more = icmp ult i32 %feature, %from
  br i1 %feature.more, label %feature.step, label %distance.done
feature.step:
  %x.index = add i32 %row.base, %feature
  %c.index = add i32 %centroid.base, %feature
  %x.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %x.index
  %c.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %c.index
  %x = load double, ptr addrspace(1) %x.ptr, align 8
  %c = load double, ptr addrspace(1) %c.ptr, align 8
  %difference = fsub double %x, %c
  %square = fmul double %difference, %difference
  %square.next = fadd double %square.sum, %square
  %feature.next = add i32 %feature, 1
  br label %feature.loop
distance.done:
  %distance = call double @llvm.sqrt.f64(double %square.sum)
  %output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
  store double %distance, ptr addrspace(1) %output.ptr, align 8
  %p.next = add i32 %p, %threads
  br label %distance.loop
exit:
  ret void
}

define internal void @kmeans_update_body(ptr addrspace(1) %input, ptr addrspace(1) %distances, ptr addrspace(1) %context, i32 %rows, i32 %from, i32 %clusters, i32 %threads) #1 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %centroid.count = mul i32 %clusters, %from
  br label %centroid.loop
centroid.loop:
  %p = phi i32 [ %tid, %entry ], [ %p.next, %centroid.next ]
  %centroid.more = icmp ult i32 %p, %centroid.count
  br i1 %centroid.more, label %centroid.start, label %exit
centroid.start:
  %cluster = udiv i32 %p, %from
  %feature = urem i32 %p, %from
  br label %row.loop
row.loop:
  %row = phi i32 [ 0, %centroid.start ], [ %row.next, %row.done ]
  %sum = phi double [ 0.0, %centroid.start ], [ %sum.next, %row.done ]
  %members = phi i32 [ 0, %centroid.start ], [ %members.next, %row.done ]
  %row.more = icmp ult i32 %row, %rows
  br i1 %row.more, label %nearest.loop, label %centroid.done
nearest.loop:
  %candidate = phi i32 [ 0, %row.loop ], [ %candidate.next, %nearest.step ]
  %best.cluster = phi i32 [ 0, %row.loop ], [ %best.cluster.next, %nearest.step ]
  %best.distance = phi double [ 0x7FF0000000000000, %row.loop ], [ %best.distance.next, %nearest.step ]
  %candidate.more = icmp ult i32 %candidate, %clusters
  br i1 %candidate.more, label %nearest.step, label %row.done
nearest.step:
  %distance.row = mul i32 %row, %clusters
  %distance.index = add i32 %distance.row, %candidate
  %distance.ptr = getelementptr inbounds double, ptr addrspace(1) %distances, i32 %distance.index
  %distance = load double, ptr addrspace(1) %distance.ptr, align 8
  %closer = fcmp olt double %distance, %best.distance
  %best.cluster.next = select i1 %closer, i32 %candidate, i32 %best.cluster
  %best.distance.next = select i1 %closer, double %distance, double %best.distance
  %candidate.next = add i32 %candidate, 1
  br label %nearest.loop
row.done:
  %assigned = icmp eq i32 %best.cluster, %cluster
  %input.row = mul i32 %row, %from
  %input.index = add i32 %input.row, %feature
  %input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
  %value = load double, ptr addrspace(1) %input.ptr, align 8
  %term = select i1 %assigned, double %value, double 0.0
  %sum.next = fadd double %sum, %term
  %member = zext i1 %assigned to i32
  %members.next = add i32 %members, %member
  %row.next = add i32 %row, 1
  br label %row.loop
centroid.done:
  %nonempty = icmp ugt i32 %members, 0
  %members.double = uitofp i32 %members to double
  %mean = fdiv double %sum, %members.double
  %context.index = add i32 %p, 1
  %context.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %context.index
  br i1 %nonempty, label %centroid.store, label %centroid.next
centroid.store:
  store double %mean, ptr addrspace(1) %context.ptr, align 8
  br label %centroid.next
centroid.next:
  %p.next = add i32 %p, %threads
  br label %centroid.loop
exit:
  call void @llvm.nvvm.barrier0()
  ret void
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

define internal double @sigmoid(double %x) #1 {
entry:
  %negative = fneg double %x
  %exponential = call double @__nv_exp(double %negative)
  %denominator = fadd double 1.0, %exponential
  %value = fdiv double 1.0, %denominator
  ret double %value
}

define internal double @recurrent_linear(ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %context, i32 %time, i32 %out, i32 %from, i32 %to, i32 %gate, i32 %operation, i32 %count) #1 {
entry:
  %input.count = mul i32 %from, %to
  %state.count = mul i32 %to, %to
  %gate.stride.0 = add i32 %input.count, %state.count
  %gate.stride = add i32 %gate.stride.0, %to
  %gate.base = mul i32 %gate, %gate.stride
  %bias.base.0 = add i32 %gate.base, %input.count
  %bias.base = add i32 %bias.base.0, %state.count
  %bias.index = add i32 %bias.base, %out
  %bias.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %bias.index
  %bias = load double, ptr addrspace(1) %bias.ptr, align 8
  %input.row = mul i32 %time, %from
  br label %input.loop
input.loop:
  %i = phi i32 [ 0, %entry ], [ %i.next, %input.step ]
  %input.sum = phi double [ %bias, %entry ], [ %input.sum.next, %input.step ]
  %input.more = icmp ult i32 %i, %from
  br i1 %input.more, label %input.step, label %state.loop
input.step:
  %x.index = add i32 %input.row, %i
  %w.row = mul i32 %i, %to
  %w.local = add i32 %w.row, %out
  %w.index = add i32 %gate.base, %w.local
  %x.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %x.index
  %w.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %w.index
  %x = load double, ptr addrspace(1) %x.ptr, align 8
  %w = load double, ptr addrspace(1) %w.ptr, align 8
  %input.product = fmul double %x, %w
  %input.sum.next = fadd double %input.sum, %input.product
  %i.next = add i32 %i, 1
  br label %input.loop
state.loop:
  %j = phi i32 [ 0, %input.loop ], [ %j.next, %state.step ]
  %sum = phi double [ %input.sum, %input.loop ], [ %sum.next, %state.step ]
  %state.more = icmp ult i32 %j, %to
  br i1 %state.more, label %state.step, label %done
state.step:
  %has.previous = icmp ugt i32 %time, 0
  %previous.time = sub i32 %time, 1
  %previous.row = mul i32 %previous.time, %to
  %previous.index = add i32 %previous.row, %j
  %safe.previous = select i1 %has.previous, i32 %previous.index, i32 0
  %previous.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %safe.previous
  %loaded.previous = load double, ptr addrspace(1) %previous.ptr, align 8
  %previous = select i1 %has.previous, double %loaded.previous, double 0.0
  %is.gru = icmp eq i32 %operation, 7
  %is.candidate = icmp eq i32 %gate, 2
  %reset.candidate = and i1 %is.gru, %is.candidate
  %reset.base = mul i32 %count, 3
  %reset.row = mul i32 %time, %to
  %reset.local = add i32 %reset.row, %j
  %reset.index = add i32 %reset.base, %reset.local
  %reset.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %reset.index
  %reset = load double, ptr addrspace(1) %reset.ptr, align 8
  %reset.previous = fmul double %reset, %previous
  %state = select i1 %reset.candidate, double %reset.previous, double %previous
  %u.base = add i32 %gate.base, %input.count
  %u.row = mul i32 %j, %to
  %u.local = add i32 %u.row, %out
  %u.index = add i32 %u.base, %u.local
  %u.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %u.index
  %u = load double, ptr addrspace(1) %u.ptr, align 8
  %product = fmul double %state, %u
  %sum.next = fadd double %sum, %product
  %j.next = add i32 %j, 1
  br label %state.loop
done:
  ret double %sum
}

define internal void @recurrent_forward_body(ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %output, ptr addrspace(1) %context, i32 %rows, i32 %from, i32 %to, i32 %operation, i32 %threads) #1 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %count = mul i32 %rows, %to
  %is.rnn.gates = icmp eq i32 %operation, 6
  %is.gru.gates = icmp eq i32 %operation, 7
  %recurrent.gates = select i1 %is.gru.gates, i32 3, i32 4
  %gates = select i1 %is.rnn.gates, i32 1, i32 %recurrent.gates
  %is.gru = icmp eq i32 %operation, 7
  br label %time.loop
time.loop:
  %time = phi i32 [ 0, %entry ], [ %time.next, %time.done ]
  %time.more = icmp ult i32 %time, %rows
  br i1 %time.more, label %gate.loop, label %exit
gate.loop:
  %p = phi i32 [ %tid, %time.loop ], [ %p.next, %gate.done ]
  %p.more = icmp ult i32 %p, %to
  br i1 %p.more, label %gate.step, label %gate.barrier
gate.step:
  %gate.limit = select i1 %is.gru, i32 2, i32 %gates
  %gate.context = mul i32 %count, 2
  %time.row = mul i32 %time, %to
  %local = add i32 %time.row, %p
  br label %component.loop
component.loop:
  %gate = phi i32 [ 0, %gate.step ], [ %gate.next, %component.step ]
  %gate.more = icmp ult i32 %gate, %gate.limit
  br i1 %gate.more, label %component.step, label %gate.finish
component.step:
  %linear = call double @recurrent_linear(ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %context, i32 %time, i32 %p, i32 %from, i32 %to, i32 %gate, i32 %operation, i32 %count)
  %tanh = call double @__nv_tanh(double %linear)
  %sigmoid = call double @sigmoid(double %linear)
  %cell.gate = icmp eq i32 %gate, 3
  %is.rnn = icmp eq i32 %operation, 6
  %use.tanh = or i1 %is.rnn, %cell.gate
  %gate.value = select i1 %use.tanh, double %tanh, double %sigmoid
  %gate.base = mul i32 %gate, %count
  %gate.index.0 = add i32 %gate.context, %gate.base
  %gate.index = add i32 %gate.index.0, %local
  %gate.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate.index
  store double %gate.value, ptr addrspace(1) %gate.ptr, align 8
  %gate.next = add i32 %gate, 1
  br label %component.loop
gate.finish:
  br i1 %is.gru, label %gate.done, label %state.finish
state.finish:
  %is.lstm = icmp eq i32 %operation, 8
  br i1 %is.lstm, label %lstm.finish, label %rnn.finish
rnn.finish:
  %rnn.index = add i32 %gate.context, %local
  %rnn.gate.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %rnn.index
  %rnn.value = load double, ptr addrspace(1) %rnn.gate.ptr, align 8
  br label %state.store
lstm.finish:
  %gate0.index = add i32 %gate.context, %local
  %gate1.base = mul i32 %count, 3
  %gate2.base = mul i32 %count, 4
  %gate3.base = mul i32 %count, 5
  %gate1.index = add i32 %gate1.base, %local
  %gate2.index = add i32 %gate2.base, %local
  %gate3.index = add i32 %gate3.base, %local
  %gate0.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate0.index
  %gate1.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate1.index
  %gate2.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate2.index
  %gate3.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gate3.index
  %input.gate = load double, ptr addrspace(1) %gate0.ptr, align 8
  %forget.gate = load double, ptr addrspace(1) %gate1.ptr, align 8
  %output.gate = load double, ptr addrspace(1) %gate2.ptr, align 8
  %cell.candidate = load double, ptr addrspace(1) %gate3.ptr, align 8
  %has.previous = icmp ugt i32 %time, 0
  %previous.local = sub i32 %local, %to
  %safe.previous = select i1 %has.previous, i32 %previous.local, i32 0
  %previous.cell.index = add i32 %count, %safe.previous
  %previous.cell.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %previous.cell.index
  %loaded.previous.cell = load double, ptr addrspace(1) %previous.cell.ptr, align 8
  %previous.cell = select i1 %has.previous, double %loaded.previous.cell, double 0.0
  %forgotten = fmul double %forget.gate, %previous.cell
  %entered = fmul double %input.gate, %cell.candidate
  %cell = fadd double %forgotten, %entered
  %cell.index = add i32 %count, %local
  %cell.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %cell.index
  store double %cell, ptr addrspace(1) %cell.ptr, align 8
  %cell.state = call double @__nv_tanh(double %cell)
  %lstm.value = fmul double %output.gate, %cell.state
  br label %state.store
state.store:
  %state = phi double [ %rnn.value, %rnn.finish ], [ %lstm.value, %lstm.finish ]
  %state.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %local
  %output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %local
  store double %state, ptr addrspace(1) %state.ptr, align 8
  store double %state, ptr addrspace(1) %output.ptr, align 8
  br label %gate.done
gate.done:
  %p.next = add i32 %p, %threads
  br label %gate.loop
gate.barrier:
  call void @llvm.nvvm.barrier0()
  br i1 %is.gru, label %gru.loop, label %time.done
gru.loop:
  %gru.p = phi i32 [ %tid, %gate.barrier ], [ %gru.next, %gru.step ]
  %gru.more = icmp ult i32 %gru.p, %to
  br i1 %gru.more, label %gru.step, label %time.done
gru.step:
  %gru.linear = call double @recurrent_linear(ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %context, i32 %time, i32 %gru.p, i32 %from, i32 %to, i32 2, i32 %operation, i32 %count)
  %candidate = call double @__nv_tanh(double %gru.linear)
  %gru.row = mul i32 %time, %to
  %gru.local = add i32 %gru.row, %gru.p
  %candidate.base = mul i32 %count, 4
  %candidate.index = add i32 %candidate.base, %gru.local
  %candidate.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %candidate.index
  store double %candidate, ptr addrspace(1) %candidate.ptr, align 8
  %update.base = mul i32 %count, 2
  %update.index = add i32 %update.base, %gru.local
  %update.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %update.index
  %update = load double, ptr addrspace(1) %update.ptr, align 8
  %has.gru.previous = icmp ugt i32 %time, 0
  %gru.previous.index = sub i32 %gru.local, %to
  %safe.gru.previous = select i1 %has.gru.previous, i32 %gru.previous.index, i32 0
  %gru.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %safe.gru.previous
  %loaded.gru.previous = load double, ptr addrspace(1) %gru.previous.ptr, align 8
  %gru.previous = select i1 %has.gru.previous, double %loaded.gru.previous, double 0.0
  %one.update = fsub double 1.0, %update
  %new.part = fmul double %one.update, %candidate
  %old.part = fmul double %update, %gru.previous
  %gru.state = fadd double %new.part, %old.part
  %gru.state.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gru.local
  %gru.output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %gru.local
  store double %gru.state, ptr addrspace(1) %gru.state.ptr, align 8
  store double %gru.state, ptr addrspace(1) %gru.output.ptr, align 8
  %gru.next = add i32 %gru.p, %threads
  br label %gru.loop
time.done:
  call void @llvm.nvvm.barrier0()
  %time.next = add i32 %time, 1
  br label %time.loop
exit:
  ret void
}

define internal void @recurrent_backward_body(ptr addrspace(1) %input, ptr addrspace(1) %weights, ptr addrspace(1) %delta, ptr addrspace(1) %previous, ptr addrspace(1) %gradient, ptr addrspace(1) %context, i32 %rows, i32 %from, i32 %to, i32 %operation, i32 %threads) #1 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %count = mul i32 %rows, %to
  %is.rnn.gates = icmp eq i32 %operation, 6
  %is.gru.gates = icmp eq i32 %operation, 7
  %recurrent.gates = select i1 %is.gru.gates, i32 3, i32 4
  %gates = select i1 %is.rnn.gates, i32 1, i32 %recurrent.gates
  %input.count = mul i32 %from, %to
  %state.count = mul i32 %to, %to
  %stride.0 = add i32 %input.count, %state.count
  %stride = add i32 %stride.0, %to
  %gate.values.base = mul i32 %count, 2
  %gate.delta.factor = add i32 %gates, 2
  %gate.delta.base = mul i32 %count, %gate.delta.factor
  %hidden.delta.factor = add i32 %gate.delta.factor, %gates
  %hidden.delta.base = mul i32 %count, %hidden.delta.factor
  %cell.delta.base = add i32 %hidden.delta.base, %count
  %is.gru = icmp eq i32 %operation, 7
  %reset.values.base = mul i32 %count, 3
  %parameter.count = mul i32 %gates, %stride
  %previous.count = mul i32 %rows, %from
  %last = sub i32 %rows, 1
  br label %time.loop
time.loop:
  %time = phi i32 [ %last, %entry ], [ %time.next, %time.done ]
  %time.more = icmp sge i32 %time, 0
  br i1 %time.more, label %hidden.loop, label %weight.loop
hidden.loop:
  %p = phi i32 [ %tid, %time.loop ], [ %p.next, %hidden.done ]
  %p.more = icmp ult i32 %p, %to
  br i1 %p.more, label %hidden.step, label %hidden.barrier
hidden.step:
  %row = mul i32 %time, %to
  %local = add i32 %row, %p
  %delta.ptr = getelementptr inbounds double, ptr addrspace(1) %delta, i32 %local
  %direct = load double, ptr addrspace(1) %delta.ptr, align 8
  %has.future = icmp ult i32 %time, %last
  %future.time = add i32 %time, 1
  %future.row = mul i32 %future.time, %to
  %future.local = add i32 %future.row, %p
  %safe.future.row = select i1 %has.future, i32 %future.row, i32 0
  %safe.future.local = select i1 %has.future, i32 %future.local, i32 0
  %has.previous = icmp ugt i32 %time, 0
  %previous.local = sub i32 %local, %to
  %safe.previous = select i1 %has.previous, i32 %previous.local, i32 0
  br label %future.gate.loop
future.gate.loop:
  %future.gate = phi i32 [ 0, %hidden.step ], [ %future.gate.next, %future.gate.done ]
  %future.sum = phi double [ 0.0, %hidden.step ], [ %future.sum.next, %future.gate.done ]
  %future.gate.more = icmp ult i32 %future.gate, %gates
  br i1 %future.gate.more, label %future.out.loop, label %future.done
future.out.loop:
  %future.out = phi i32 [ 0, %future.gate.loop ], [ %future.out.next, %future.out.step ]
  %future.gate.sum = phi double [ %future.sum, %future.gate.loop ], [ %future.gate.sum.next, %future.out.step ]
  %future.out.more = icmp ult i32 %future.out, %to
  br i1 %future.out.more, label %future.out.step, label %future.gate.done
future.out.step:
  %gd.gate = mul i32 %future.gate, %count
  %gd.base = add i32 %gate.delta.base, %gd.gate
  %gd.local = add i32 %safe.future.row, %future.out
  %gd.index = add i32 %gd.base, %gd.local
  %gd.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %gd.index
  %gd.loaded = load double, ptr addrspace(1) %gd.ptr, align 8
  %gd = select i1 %has.future, double %gd.loaded, double 0.0
  %u.gate = mul i32 %future.gate, %stride
  %u.base.0 = add i32 %u.gate, %input.count
  %u.row = mul i32 %p, %to
  %u.local = add i32 %u.row, %future.out
  %u.index = add i32 %u.base.0, %u.local
  %u.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %u.index
  %u = load double, ptr addrspace(1) %u.ptr, align 8
  %is.candidate = icmp eq i32 %future.gate, 2
  %gru.candidate = and i1 %is.gru, %is.candidate
  %reset.index = add i32 %reset.values.base, %safe.future.local
  %reset.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %reset.index
  %reset = load double, ptr addrspace(1) %reset.ptr, align 8
  %scaled.u = fmul double %u, %reset
  %effective.u = select i1 %gru.candidate, double %scaled.u, double %u
  %product = fmul double %gd, %effective.u
  %future.gate.sum.next = fadd double %future.gate.sum, %product
  %future.out.next = add i32 %future.out, 1
  br label %future.out.loop
future.gate.done:
  %future.sum.next = fadd double %future.gate.sum, 0.0
  %future.gate.next = add i32 %future.gate, 1
  br label %future.gate.loop
future.done:
  %future.hidden.index = add i32 %hidden.delta.base, %safe.future.local
  %future.hidden.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %future.hidden.index
  %future.hidden.loaded = load double, ptr addrspace(1) %future.hidden.ptr, align 8
  %future.hidden = select i1 %has.future, double %future.hidden.loaded, double 0.0
  %future.update.index = add i32 %gate.values.base, %safe.future.local
  %future.update.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %future.update.index
  %future.update = load double, ptr addrspace(1) %future.update.ptr, align 8
  %gru.direct = fmul double %future.hidden, %future.update
  %gru.extra = select i1 %is.gru, double %gru.direct, double 0.0
  %with.future = fadd double %direct, %future.sum
  %dh = fadd double %with.future, %gru.extra
  %dh.index = add i32 %hidden.delta.base, %local
  %dh.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dh.index
  store double %dh, ptr addrspace(1) %dh.ptr, align 8
  %is.rnn = icmp eq i32 %operation, 6
  br i1 %is.rnn, label %rnn.delta, label %gru.test
rnn.delta:
  %hidden.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %local
  %hidden = load double, ptr addrspace(1) %hidden.ptr, align 8
  %hidden.square = fmul double %hidden, %hidden
  %hidden.derivative = fsub double 1.0, %hidden.square
  %rnn.gd = fmul double %dh, %hidden.derivative
  %rnn.gd.index = add i32 %gate.delta.base, %local
  %rnn.gd.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %rnn.gd.index
  store double %rnn.gd, ptr addrspace(1) %rnn.gd.ptr, align 8
  br label %hidden.done
gru.test:
  br i1 %is.gru, label %gru.delta, label %lstm.delta
gru.delta:
  %z.index = add i32 %gate.values.base, %local
  %n.base = mul i32 %count, 4
  %n.index = add i32 %n.base, %local
  %z.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %z.index
  %n.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %n.index
  %z = load double, ptr addrspace(1) %z.ptr, align 8
  %n = load double, ptr addrspace(1) %n.ptr, align 8
  %previous.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %safe.previous
  %previous.loaded = load double, ptr addrspace(1) %previous.ptr, align 8
  %previous.hidden = select i1 %has.previous, double %previous.loaded, double 0.0
  %one.z = fsub double 1.0, %z
  %n.square = fmul double %n, %n
  %one.n = fsub double 1.0, %n.square
  %dn.0 = fmul double %dh, %one.z
  %dn = fmul double %dn.0, %one.n
  %z.diff = fsub double %previous.hidden, %n
  %dz.0 = fmul double %dh, %z.diff
  %dz.1 = fmul double %dz.0, %z
  %dz = fmul double %dz.1, %one.z
  %dz.index = add i32 %gate.delta.base, %local
  %dn.base = mul i32 %count, 2
  %dn.index.0 = add i32 %gate.delta.base, %dn.base
  %dn.index = add i32 %dn.index.0, %local
  %dz.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dz.index
  %dn.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dn.index
  store double %dz, ptr addrspace(1) %dz.ptr, align 8
  store double %dn, ptr addrspace(1) %dn.ptr, align 8
  br label %hidden.done
lstm.delta:
  %i.index = add i32 %gate.values.base, %local
  %f.base = mul i32 %count, 3
  %o.base = mul i32 %count, 4
  %g.base = mul i32 %count, 5
  %f.index = add i32 %f.base, %local
  %o.index = add i32 %o.base, %local
  %g.index = add i32 %g.base, %local
  %i.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %i.index
  %f.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %f.index
  %o.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %o.index
  %g.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %g.index
  %iv = load double, ptr addrspace(1) %i.ptr, align 8
  %fv = load double, ptr addrspace(1) %f.ptr, align 8
  %ov = load double, ptr addrspace(1) %o.ptr, align 8
  %gv = load double, ptr addrspace(1) %g.ptr, align 8
  %cell.index = add i32 %count, %local
  %cell.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %cell.index
  %cell = load double, ptr addrspace(1) %cell.ptr, align 8
  %tanh.cell = call double @__nv_tanh(double %cell)
  %tanh.cell.square = fmul double %tanh.cell, %tanh.cell
  %cell.derivative = fsub double 1.0, %tanh.cell.square
  %future.dc.index = add i32 %cell.delta.base, %safe.future.local
  %future.dc.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %future.dc.index
  %future.dc.loaded = load double, ptr addrspace(1) %future.dc.ptr, align 8
  %future.f.index = add i32 %f.base, %safe.future.local
  %future.f.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %future.f.index
  %future.f = load double, ptr addrspace(1) %future.f.ptr, align 8
  %future.cell.0 = fmul double %future.dc.loaded, %future.f
  %future.cell = select i1 %has.future, double %future.cell.0, double 0.0
  %dc.0 = fmul double %dh, %ov
  %dc.1 = fmul double %dc.0, %cell.derivative
  %dc = fadd double %dc.1, %future.cell
  %dc.index = add i32 %cell.delta.base, %local
  %dc.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dc.index
  store double %dc, ptr addrspace(1) %dc.ptr, align 8
  %previous.cell.index = add i32 %count, %safe.previous
  %previous.cell.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %previous.cell.index
  %previous.cell.loaded = load double, ptr addrspace(1) %previous.cell.ptr, align 8
  %previous.cell = select i1 %has.previous, double %previous.cell.loaded, double 0.0
  %one.i = fsub double 1.0, %iv
  %one.f = fsub double 1.0, %fv
  %one.o = fsub double 1.0, %ov
  %g.square = fmul double %gv, %gv
  %one.g = fsub double 1.0, %g.square
  %di.0 = fmul double %dc, %gv
  %di.1 = fmul double %di.0, %iv
  %di = fmul double %di.1, %one.i
  %df.0 = fmul double %dc, %previous.cell
  %df.1 = fmul double %df.0, %fv
  %df = fmul double %df.1, %one.f
  %do.0 = fmul double %dh, %tanh.cell
  %do.1 = fmul double %do.0, %ov
  %do = fmul double %do.1, %one.o
  %dg.0 = fmul double %dc, %iv
  %dg = fmul double %dg.0, %one.g
  br label %lstm.store.loop
lstm.store.loop:
  %store.gate = phi i32 [ 0, %lstm.delta ], [ %store.next, %lstm.store ]
  %store.more = icmp ult i32 %store.gate, 4
  br i1 %store.more, label %lstm.store, label %hidden.done
lstm.store:
  %store.value.0 = select i1 true, double %di, double %df
  %store.is.f = icmp eq i32 %store.gate, 1
  %store.value.1 = select i1 %store.is.f, double %df, double %store.value.0
  %store.is.o = icmp eq i32 %store.gate, 2
  %store.value.2 = select i1 %store.is.o, double %do, double %store.value.1
  %store.is.g = icmp eq i32 %store.gate, 3
  %store.value = select i1 %store.is.g, double %dg, double %store.value.2
  %store.gate.base = mul i32 %store.gate, %count
  %store.index.0 = add i32 %gate.delta.base, %store.gate.base
  %store.index = add i32 %store.index.0, %local
  %store.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %store.index
  store double %store.value, ptr addrspace(1) %store.ptr, align 8
  %store.next = add i32 %store.gate, 1
  br label %lstm.store.loop
hidden.done:
  %p.next = add i32 %p, %threads
  br label %hidden.loop
hidden.barrier:
  call void @llvm.nvvm.barrier0()
  br i1 %is.gru, label %reset.loop, label %time.done
reset.loop:
  %reset.p = phi i32 [ %tid, %hidden.barrier ], [ %reset.next, %reset.done ]
  %reset.more = icmp ult i32 %reset.p, %to
  br i1 %reset.more, label %reset.step, label %time.done
reset.step:
  %reset.row.current = mul i32 %time, %to
  %reset.local.current = add i32 %reset.row.current, %reset.p
  %reset.gate.index = add i32 %reset.values.base, %reset.local.current
  %reset.gate.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %reset.gate.index
  %reset.gate = load double, ptr addrspace(1) %reset.gate.ptr, align 8
  %has.reset.previous = icmp ugt i32 %time, 0
  %reset.previous.index = sub i32 %reset.local.current, %to
  %safe.reset.previous = select i1 %has.reset.previous, i32 %reset.previous.index, i32 0
  %reset.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %safe.reset.previous
  %reset.previous.loaded = load double, ptr addrspace(1) %reset.previous.ptr, align 8
  %reset.previous = select i1 %has.reset.previous, double %reset.previous.loaded, double 0.0
  br label %reset.out.loop
reset.out.loop:
  %reset.out = phi i32 [ 0, %reset.step ], [ %reset.out.next, %reset.out.step ]
  %reset.sum = phi double [ 0.0, %reset.step ], [ %reset.sum.next, %reset.out.step ]
  %reset.out.more = icmp ult i32 %reset.out, %to
  br i1 %reset.out.more, label %reset.out.step, label %reset.done
reset.out.step:
  %reset.dn.local = add i32 %reset.row.current, %reset.out
  %reset.dn.offset = mul i32 %count, 2
  %reset.dn.base = add i32 %gate.delta.base, %reset.dn.offset
  %reset.dn.index = add i32 %reset.dn.base, %reset.dn.local
  %reset.dn.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %reset.dn.index
  %reset.dn = load double, ptr addrspace(1) %reset.dn.ptr, align 8
  %candidate.gate.base = mul i32 2, %stride
  %candidate.u.base = add i32 %candidate.gate.base, %input.count
  %candidate.u.row = mul i32 %reset.p, %to
  %candidate.u.local = add i32 %candidate.u.row, %reset.out
  %candidate.u.index = add i32 %candidate.u.base, %candidate.u.local
  %candidate.u.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %candidate.u.index
  %candidate.u = load double, ptr addrspace(1) %candidate.u.ptr, align 8
  %reset.product.0 = fmul double %reset.dn, %candidate.u
  %reset.sum.next = fadd double %reset.sum, %reset.product.0
  %reset.out.next = add i32 %reset.out, 1
  br label %reset.out.loop
reset.done:
  %one.reset = fsub double 1.0, %reset.gate
  %dr.0 = fmul double %reset.sum, %reset.previous
  %dr.1 = fmul double %dr.0, %reset.gate
  %dr = fmul double %dr.1, %one.reset
  %dr.base = add i32 %gate.delta.base, %count
  %dr.index = add i32 %dr.base, %reset.local.current
  %dr.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %dr.index
  store double %dr, ptr addrspace(1) %dr.ptr, align 8
  %reset.next = add i32 %reset.p, %threads
  br label %reset.loop
time.done:
  call void @llvm.nvvm.barrier0()
  %time.next = sub i32 %time, 1
  br label %time.loop
weight.loop:
  %wp = phi i32 [ %tid, %time.loop ], [ %wp.next, %weight.done ]
  %wp.more = icmp ult i32 %wp, %parameter.count
  br i1 %wp.more, label %weight.step, label %previous.loop
weight.step:
  %weight.gate = udiv i32 %wp, %stride
  %weight.local = urem i32 %wp, %stride
  %is.input.weight = icmp ult i32 %weight.local, %input.count
  %state.end = add i32 %input.count, %state.count
  %is.before.bias = icmp ult i32 %weight.local, %state.end
  %not.input.weight = xor i1 %is.input.weight, true
  %is.state.weight = and i1 %is.before.bias, %not.input.weight
  %input.out = urem i32 %weight.local, %to
  %input.feature = udiv i32 %weight.local, %to
  %safe.input.feature = select i1 %is.input.weight, i32 %input.feature, i32 0
  %state.local.0 = sub i32 %weight.local, %input.count
  %state.local = select i1 %is.state.weight, i32 %state.local.0, i32 0
  %state.out = urem i32 %state.local, %to
  %state.feature = udiv i32 %state.local, %to
  %bias.out = sub i32 %weight.local, %state.end
  %selected.out.0 = select i1 %is.input.weight, i32 %input.out, i32 %state.out
  %selected.out = select i1 %is.before.bias, i32 %selected.out.0, i32 %bias.out
  br label %weight.time.loop
weight.time.loop:
  %wt = phi i32 [ 0, %weight.step ], [ %wt.next, %weight.time.step ]
  %weight.sum = phi double [ 0.0, %weight.step ], [ %weight.sum.next, %weight.time.step ]
  %wt.more = icmp ult i32 %wt, %rows
  br i1 %wt.more, label %weight.time.step, label %weight.done
weight.time.step:
  %wt.row = mul i32 %wt, %to
  %wt.gd.local = add i32 %wt.row, %selected.out
  %wt.gd.gate = mul i32 %weight.gate, %count
  %wt.gd.base = add i32 %gate.delta.base, %wt.gd.gate
  %wt.gd.index = add i32 %wt.gd.base, %wt.gd.local
  %wt.gd.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %wt.gd.index
  %wt.gd = load double, ptr addrspace(1) %wt.gd.ptr, align 8
  %wt.input.row = mul i32 %wt, %from
  %wt.input.index = add i32 %wt.input.row, %safe.input.feature
  %wt.input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %wt.input.index
  %wt.input = load double, ptr addrspace(1) %wt.input.ptr, align 8
  %wt.has.previous = icmp ugt i32 %wt, 0
  %wt.previous.row = sub i32 %wt.row, %to
  %wt.previous.index = add i32 %wt.previous.row, %state.feature
  %wt.safe.previous = select i1 %wt.has.previous, i32 %wt.previous.index, i32 0
  %wt.previous.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %wt.safe.previous
  %wt.previous.loaded = load double, ptr addrspace(1) %wt.previous.ptr, align 8
  %wt.previous = select i1 %wt.has.previous, double %wt.previous.loaded, double 0.0
  %wt.gru = icmp eq i32 %operation, 7
  %wt.candidate = icmp eq i32 %weight.gate, 2
  %wt.reset.candidate = and i1 %wt.gru, %wt.candidate
  %wt.reset.local = add i32 %wt.row, %state.feature
  %wt.reset.index = add i32 %reset.values.base, %wt.reset.local
  %wt.reset.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %wt.reset.index
  %wt.reset = load double, ptr addrspace(1) %wt.reset.ptr, align 8
  %wt.reset.previous = fmul double %wt.reset, %wt.previous
  %wt.state = select i1 %wt.reset.candidate, double %wt.reset.previous, double %wt.previous
  %wt.nonbias = select i1 %is.input.weight, double %wt.input, double %wt.state
  %wt.factor = select i1 %is.before.bias, double %wt.nonbias, double 1.0
  %wt.product = fmul double %wt.gd, %wt.factor
  %weight.sum.next = fadd double %weight.sum, %wt.product
  %wt.next = add i32 %wt, 1
  br label %weight.time.loop
weight.done:
  %gradient.ptr = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %wp
  store double %weight.sum, ptr addrspace(1) %gradient.ptr, align 8
  %wp.next = add i32 %wp, %threads
  br label %weight.loop
previous.loop:
  %xp = phi i32 [ %tid, %weight.loop ], [ %xp.next, %previous.done ]
  %xp.more = icmp ult i32 %xp, %previous.count
  br i1 %xp.more, label %previous.step, label %exit
previous.step:
  %x.time = udiv i32 %xp, %from
  %x.feature = urem i32 %xp, %from
  br label %x.gate.loop
x.gate.loop:
  %x.gate = phi i32 [ 0, %previous.step ], [ %x.gate.next, %x.gate.done ]
  %x.sum = phi double [ 0.0, %previous.step ], [ %x.sum.next, %x.gate.done ]
  %x.gate.more = icmp ult i32 %x.gate, %gates
  br i1 %x.gate.more, label %x.out.loop, label %previous.done
x.out.loop:
  %x.out = phi i32 [ 0, %x.gate.loop ], [ %x.out.next, %x.out.step ]
  %x.gate.sum = phi double [ %x.sum, %x.gate.loop ], [ %x.gate.sum.next, %x.out.step ]
  %x.out.more = icmp ult i32 %x.out, %to
  br i1 %x.out.more, label %x.out.step, label %x.gate.done
x.out.step:
  %x.w.gate = mul i32 %x.gate, %stride
  %x.w.row = mul i32 %x.feature, %to
  %x.w.local = add i32 %x.w.row, %x.out
  %x.w.index = add i32 %x.w.gate, %x.w.local
  %x.w.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %x.w.index
  %x.w = load double, ptr addrspace(1) %x.w.ptr, align 8
  %x.time.row = mul i32 %x.time, %to
  %x.gd.local = add i32 %x.time.row, %x.out
  %x.gd.gate = mul i32 %x.gate, %count
  %x.gd.base = add i32 %gate.delta.base, %x.gd.gate
  %x.gd.index = add i32 %x.gd.base, %x.gd.local
  %x.gd.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %x.gd.index
  %x.gd = load double, ptr addrspace(1) %x.gd.ptr, align 8
  %x.product = fmul double %x.w, %x.gd
  %x.gate.sum.next = fadd double %x.gate.sum, %x.product
  %x.out.next = add i32 %x.out, 1
  br label %x.out.loop
x.gate.done:
  %x.sum.next = fadd double %x.sum, %x.gate.sum
  %x.gate.next = add i32 %x.gate, 1
  br label %x.gate.loop
previous.done:
  %previous.output.ptr = getelementptr inbounds double, ptr addrspace(1) %previous, i32 %xp
  store double %x.sum, ptr addrspace(1) %previous.output.ptr, align 8
  %xp.next = add i32 %xp, %threads
  br label %previous.loop
exit:
  call void @llvm.nvvm.barrier0()
  ret void
}

define internal void @adamw_body(ptr addrspace(1) %weights, ptr addrspace(1) nocapture readonly %gradient, ptr addrspace(1) %moments, ptr addrspace(1) %variances, i32 %count, i32 %threads, double %rate, double %beta1, double %beta2, double %epsilon, double %decay, i32 %step, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
    i32 11, label %block.residual
    i32 12, label %block.perceptron
  ]
block.linear:
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
  %block.result = phi double [ %value, %block.linear ], [ %residual.result, %block.residual ], [ %perceptron.result, %block.perceptron ]
  %block.derivative = phi double [ 1.0, %block.linear ], [ 1.0, %block.residual ], [ 1.0, %block.perceptron ]
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
  %cos.result = call double @__nv_cos(double %block.result)
  %cos.sin = call double @__nv_sin(double %block.result)
  %cos.derivative = fneg double %cos.sin
  br label %activation.done
activation.exp:
  %exp.result = call double @__nv_exp(double %block.result)
  br label %activation.done
activation.log10:
  %log.abs = call double @llvm.fabs.f64(double %block.result)
  %log.shift = fadd double %log.abs, 1.0
  %log.value = call double @__nv_log(double %log.shift)
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
  %ln.value = call double @__nv_log(double %ln.shift)
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
  %tan.sin = call double @__nv_sin(double %block.result)
  %tan.cos = call double @__nv_cos(double %block.result)
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
  %sigmoid.exp = call double @__nv_exp(double %sigmoid.negative)
  %sigmoid.denominator = fadd double 1.0, %sigmoid.exp
  %sigmoid.result = fdiv double 1.0, %sigmoid.denominator
  %sigmoid.one = fsub double 1.0, %sigmoid.result
  %sigmoid.derivative = fmul double %sigmoid.result, %sigmoid.one
  br label %activation.done
activation.tanh:
  %tanh.result = call double @__nv_tanh(double %block.result)
  %tanh.square = fmul double %tanh.result, %tanh.result
  %tanh.derivative = fsub double 1.0, %tanh.square
  br label %activation.done
activation.selu:
  %selu.alpha.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 3
  %selu.scale.ptr = getelementptr inbounds double, ptr addrspace(1) %config, i32 4
  %selu.alpha = load double, ptr addrspace(1) %selu.alpha.ptr, align 8
  %selu.scale = load double, ptr addrspace(1) %selu.scale.ptr, align 8
  %selu.test = fcmp ogt double %block.result, 0.0
  %selu.exp = call double @__nv_exp(double %block.result)
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
  %gelu.curve = call double @__nv_tanh(double %gelu.argument)
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
  %silu.exp = call double @__nv_exp(double %silu.negative)
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
  %elu.exp = call double @__nv_exp(double %block.result)
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
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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

define ptx_kernel void @normalize(ptr addrspace(1) %values, ptr addrspace(1) nocapture readonly %reference, ptr addrspace(1) %scales, i32 %rows, i32 %width, i32 %mode, i32 %reverse, double %epsilon, i32 %threads) #0 {
entry:
  call void @normalize_body(ptr addrspace(1) %values, ptr addrspace(1) %reference, ptr addrspace(1) %scales, i32 %rows, i32 %width, i32 %mode, i32 %reverse, double %epsilon, i32 %threads, i32 -1)
  ret void
}

define internal void @forward_body(ptr addrspace(1) nocapture readonly %samples, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture readonly %config, ptr addrspace(1) nocapture readonly %value_pointers, ptr addrspace(1) nocapture readonly %raw_pointers, ptr addrspace(1) nocapture readonly %operation_pointers, ptr addrspace(1) nocapture readonly %activation_pointers, ptr addrspace(1) nocapture readonly %scale_pointers, ptr addrspace(1) nocapture readonly %context_pointers, ptr addrspace(1) nocapture readonly %descriptors, ptr addrspace(1) nocapture readonly %parameters, i32 %rows, i32 %stages, double %epsilon, i32 %threads) #1 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
  %is.kmeans = icmp eq i32 %operation, 3
  %recurrent.low = icmp uge i32 %operation, 6
  %recurrent.high = icmp ule i32 %operation, 8
  %is.recurrent = and i1 %recurrent.low, %recurrent.high
  br i1 %is.embedding, label %embedding.loop, label %kmeans.test
kmeans.test:
  br i1 %is.kmeans, label %kmeans.forward, label %recurrent.test
kmeans.forward:
  call void @kmeans_forward_body(ptr addrspace(1) %source, ptr addrspace(1) %values, ptr addrspace(1) %context, i32 %rows, i32 %from, i32 %to, i32 %threads)
  br label %value.loop
recurrent.test:
  br i1 %is.recurrent, label %recurrent.forward, label %value.loop
recurrent.forward:
  call void @recurrent_forward_body(ptr addrspace(1) %source, ptr addrspace(1) %matrix, ptr addrspace(1) %values, ptr addrspace(1) %context, i32 %rows, i32 %from, i32 %to, i32 %operation, i32 %threads)
  br label %value.loop
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
value.loop:
  %p = phi i32 [ %tid, %recurrent.test ], [ %tid, %recurrent.forward ], [ %tid, %kmeans.forward ], [ %p.next, %transform.step ]
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
  %matrix.bypass = or i1 %is.recurrent, %is.kmeans
  br i1 %matrix.bypass, label %transform.step, label %conv.test
conv.test:
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
  %convolution.or.pool = or i1 %is.conv, %is.pool
  %differentiable.special = or i1 %convolution.or.pool, %is.recurrent
  %special = or i1 %differentiable.special, %is.kmeans
  %transform.operation = select i1 %special, i32 0, i32 %operation
  call void @transform_body(ptr addrspace(1) %skip, ptr addrspace(1) %value.element, ptr addrspace(1) %raw.element, ptr addrspace(1) %operation.element, ptr addrspace(1) %activation.element, ptr addrspace(1) %config, i32 1, i32 %from, i32 %to, i32 %transform.operation, i32 %activation, double %parameter, double %secondary, i32 1, i32 0)
  %p.next = add nuw i32 %p, %threads
  br label %value.loop
normalize.test:
  call void @llvm.nvvm.barrier0()
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
  call void @llvm.nvvm.barrier0()
  %stage.next = add nuw i32 %stage, 1
  br label %stage.loop
exit:
  ret void
}

define ptx_kernel void @forward_graph(ptr addrspace(1) nocapture readonly %samples, ptr addrspace(1) nocapture readonly %weights, ptr addrspace(1) nocapture readonly %config, ptr addrspace(1) nocapture readonly %value_pointers, ptr addrspace(1) nocapture readonly %raw_pointers, ptr addrspace(1) nocapture readonly %operation_pointers, ptr addrspace(1) nocapture readonly %activation_pointers, ptr addrspace(1) nocapture readonly %scale_pointers, ptr addrspace(1) nocapture readonly %context_pointers, ptr addrspace(1) nocapture readonly %descriptors, ptr addrspace(1) nocapture readonly %parameters, i32 %rows, i32 %stages, double %epsilon, i32 %threads) #0 {
entry:
  call void @forward_body(ptr addrspace(1) %samples, ptr addrspace(1) %weights, ptr addrspace(1) %config, ptr addrspace(1) %value_pointers, ptr addrspace(1) %raw_pointers, ptr addrspace(1) %operation_pointers, ptr addrspace(1) %activation_pointers, ptr addrspace(1) %scale_pointers, ptr addrspace(1) %context_pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %parameters, i32 %rows, i32 %stages, double %epsilon, i32 %threads)
  ret void
}

define ptx_kernel void @epoch_graph(ptr addrspace(1) %samples, ptr addrspace(1) %targets, ptr addrspace(1) %weights, ptr addrspace(1) %config, ptr addrspace(1) %value_pointers, ptr addrspace(1) %raw_pointers, ptr addrspace(1) %operation_pointers, ptr addrspace(1) %activation_pointers, ptr addrspace(1) %scale_pointers, ptr addrspace(1) %context_pointers, ptr addrspace(1) %descriptors, ptr addrspace(1) %parameters, ptr addrspace(1) %metrics, ptr addrspace(1) %gradient, ptr addrspace(1) %delta_a, ptr addrspace(1) %delta_b, ptr addrspace(1) %moments, ptr addrspace(1) %variances, ptr addrspace(1) %checkpoint_weights, i32 %rows, i32 %stages, i32 %parameter_count, i32 %loss, double %previous_loss, double %tolerance, i32 %checkpoint_enabled, double %normalization_epsilon, double %rate, double %beta1, double %beta2, double %optimizer_epsilon, double %decay, i32 %step, i32 %threads) #0 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
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
  call void @llvm.nvvm.barrier0()
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
  call void @llvm.nvvm.barrier0()
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
  call void @llvm.nvvm.barrier0()
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
  call void @llvm.nvvm.barrier0()
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
  call void @llvm.nvvm.barrier0()
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
  call void @llvm.nvvm.barrier0()
  %has.prelu = icmp sge i32 %backward.prelu, 0
  %prelu.thread = and i1 %has.prelu, %metric.thread
  br i1 %prelu.thread, label %prelu.call, label %prelu.done
prelu.call:
  %prelu.gradient = getelementptr inbounds double, ptr addrspace(1) %gradient, i32 %backward.prelu
  %backward.count = mul i32 %rows, %backward.to
  call void @prelu_gradient_body(ptr addrspace(1) %backward.raw, ptr addrspace(1) %delta, ptr addrspace(1) %prelu.gradient, i32 %backward.count)
  br label %prelu.done
prelu.done:
  call void @llvm.nvvm.barrier0()
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
  call void @llvm.nvvm.barrier0()
  %previous.count = mul i32 %rows, %backward.from
  %matrix.recurrent.low = icmp uge i32 %backward.operation, 6
  %matrix.recurrent.high = icmp ule i32 %backward.operation, 8
  %matrix.recurrent = and i1 %matrix.recurrent.low, %matrix.recurrent.high
  %matrix.conv = icmp eq i32 %backward.operation, 1
  %matrix.pool = icmp eq i32 %backward.operation, 2
  %matrix.embedding = icmp eq i32 %backward.operation, 4
  %matrix.kmeans = icmp eq i32 %backward.operation, 3
  br i1 %matrix.recurrent, label %recurrent.backward, label %kmeans.matrix.test
recurrent.backward:
  call void @recurrent_backward_body(ptr addrspace(1) %backward.source, ptr addrspace(1) %backward.matrix, ptr addrspace(1) %delta, ptr addrspace(1) %previous, ptr addrspace(1) %backward.matrix.gradient, ptr addrspace(1) %backward.context, i32 %rows, i32 %backward.from, i32 %backward.to, i32 %backward.operation, i32 %threads)
  br label %residual.test
kmeans.matrix.test:
  br i1 %matrix.kmeans, label %kmeans.update, label %conv.matrix.test
kmeans.update:
  call void @kmeans_update_body(ptr addrspace(1) %backward.source, ptr addrspace(1) %backward.raw, ptr addrspace(1) %backward.context, i32 %rows, i32 %backward.from, i32 %backward.to, i32 %threads)
  br label %residual.test
conv.matrix.test:
  br i1 %matrix.conv, label %conv.matrix.loop, label %pool.matrix.test
pool.matrix.test:
  br i1 %matrix.pool, label %previous.dispatch, label %embedding.matrix.test
embedding.matrix.test:
  br i1 %matrix.embedding, label %embedding.matrix.loop, label %matrix.loop
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
  call void @llvm.nvvm.barrier0()
  br i1 %matrix.conv, label %conv.previous.loop, label %pool.previous.test
pool.previous.test:
  br i1 %matrix.pool, label %pool.previous.loop, label %embedding.previous.test
embedding.previous.test:
  br i1 %matrix.embedding, label %embedding.previous.loop, label %previous.loop
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
  call void @llvm.nvvm.barrier0()
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
  call void @llvm.nvvm.barrier0()
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
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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

define ptx_kernel void @affine(ptr addrspace(1) %values, i32 %count, double %scale, double %offset, i32 %threads) #0 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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

define ptx_kernel void @initialize(ptr addrspace(1) nocapture writeonly %values, i32 %count, i64 %seed, double %scale, i32 %threads) #0 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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

define ptx_kernel void @set_value(ptr addrspace(1) %values, i32 %index, double %value) #0 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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
  %exponential = call double @__nv_exp(double %negative)
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
  %log.probability = call double @__nv_log(double %probability)
  %one.probability = fsub double 1.0, %probability
  %log.one.probability = call double @__nv_log(double %one.probability)
  %first = fmul double %target.clamped, %log.probability
  %one.target = fsub double 1.0, %target.clamped
  %second = fmul double %one.target, %log.one.probability
  %cross.sum = fadd double %first, %second
  %cross = fneg double %cross.sum
  br label %metric.done
loss.focal:
  %focal.negative = fneg double %prediction
  %focal.exp = call double @__nv_exp(double %focal.negative)
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
  %correct.log = call double @__nv_log(double %correct)
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

define ptx_kernel void @metrics(ptr addrspace(1) nocapture readonly %predictions, ptr addrspace(1) nocapture readonly %targets, ptr addrspace(1) nocapture writeonly %metrics, i32 %rows, i32 %loss) #0 {
entry:
  call void @metrics_body(ptr addrspace(1) %predictions, ptr addrspace(1) %targets, ptr addrspace(1) %metrics, i32 %rows, i32 %loss)
  ret void
}

define internal void @loss_gradient_body(ptr addrspace(1) nocapture readonly %predictions, ptr addrspace(1) nocapture readonly %targets, ptr addrspace(1) nocapture writeonly %gradient, i32 %rows, i32 %loss, double %loss_value, i32 %threads, i32 %forced) #1 {
entry:
  %tid = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()
  %bid = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()
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

attributes #0 = { nounwind }
attributes #1 = { alwaysinline nounwind }
