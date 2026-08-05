target triple = "amdgcn-amd-amdhsa"
declare i32 @llvm.amdgcn.workitem.id.x()
declare i32 @llvm.amdgcn.workgroup.id.x()
declare void @llvm.amdgcn.s.barrier()
declare double @llvm.sqrt.f64(double)
declare double @__ocml_exp_f64(double)
declare double @__ocml_tanh_f64(double)
declare void @llvm.trap()

define internal void @dense_body(
	ptr addrspace(1) nocapture readonly %input,
	ptr addrspace(1) nocapture readonly %weights,
	ptr addrspace(1) nocapture writeonly %output,
	i32 %rows,
	i32 %from,
	i32 %to,
	i32 %threads,
	i32 %forced
) #1 {
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

define internal void @conv_forward_body(
	ptr addrspace(1) %input,
	ptr addrspace(1) %weights,
	ptr addrspace(1) %output,
	i32 %p,
	i32 %from,
	i32 %to,
	i32 %kernel,
	i32 %channels
) #1 {
entry:
	%length = udiv i32 %from, %channels
	%positions.0 = sub i32 %length, %kernel
	%positions = add i32 %positions.0, 1
	%row = udiv i32 %p, %to
	%out = urem i32 %p, %to
	%filter = udiv i32 %out, %positions
	%position = urem i32 %out, %positions
	%row.base = mul i32 %row, %from
	%weight.stride = mul i32 %channels, %kernel
	%weight.base = mul i32 %filter, %weight.stride
	br label %loop
loop:
	%i = phi i32 [ 0, %entry ], [ %next, %step ]
	%sum = phi double [ 0.0, %entry ], [ %sum.next, %step ]
	%more = icmp ult i32 %i, %weight.stride
	br i1 %more, label %step, label %done
step:
	%channel = udiv i32 %i, %kernel
	%offset = urem i32 %i, %kernel
	%channel.base = mul i32 %channel, %length
	%input.local.0 = add i32 %channel.base, %position
	%input.local = add i32 %input.local.0, %offset
	%input.index = add i32 %row.base, %input.local
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

define internal void @pool_forward_body(
	ptr addrspace(1) %input,
	ptr addrspace(1) %output,
	i32 %p,
	i32 %from,
	i32 %to,
	i32 %size,
	i32 %channels
) #1 {
entry:
	%length = udiv i32 %from, %channels
	%pooled.length = udiv i32 %to, %channels
	%row = udiv i32 %p, %to
	%out = urem i32 %p, %to
	%channel = udiv i32 %out, %pooled.length
	%spatial = urem i32 %out, %pooled.length
	%start = mul i32 %spatial, %size
	%candidate.end = add i32 %start, %size
	%short = icmp ult i32 %candidate.end, %length
	%end = select i1 %short, i32 %candidate.end, i32 %length
	%row.base = mul i32 %row, %from
	%channel.local = mul i32 %channel, %length
	%input.base = add i32 %row.base, %channel.local
	br label %loop
loop:
	%i = phi i32 [ %start, %entry ], [ %next, %step ]
	%maximum = phi double [ 0xFFF0000000000000, %entry ], [ %maximum.next, %step ]
	%more = icmp ult i32 %i, %end
	br i1 %more, label %step, label %done
step:
	%index = add i32 %input.base, %i
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

define internal i32 @embedding_index(double %value, i32 %vocabulary) #1 {
entry:
	%ordered = fcmp ord double %value, %value
	%nonnegative = fcmp oge double %value, 0.0
	%limit = uitofp i32 %vocabulary to double
	%below = fcmp olt double %value, %limit
	%lower.valid = and i1 %ordered, %nonnegative
	%range.valid = and i1 %lower.valid, %below
	br i1 %range.valid, label %convert, label %invalid
convert:
	%index = fptoui double %value to i32
	%roundtrip = uitofp i32 %index to double
	%integer = fcmp oeq double %value, %roundtrip
	%result = select i1 %integer, i32 %index, i32 %vocabulary
	ret i32 %result
invalid:
	ret i32 %vocabulary
}

define internal void @embedding_forward_body(
	ptr addrspace(1) nocapture readonly %input,
	ptr addrspace(1) nocapture readonly %table,
	ptr addrspace(1) nocapture writeonly %output,
	i32 %p,
	i32 %from,
	i32 %to,
	i32 %vocabulary
) #1 {
entry:
	%dimensions = udiv i32 %to, %from
	%row = udiv i32 %p, %to
	%local = urem i32 %p, %to
	%component = udiv i32 %local, %from
	%slot = urem i32 %local, %from
	%row.base = mul i32 %row, %from
	%input.index = add i32 %row.base, %slot
	%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
	%value = load double, ptr addrspace(1) %input.ptr, align 8
	%index = call i32 @embedding_index(double %value, i32 %vocabulary)
	%valid = icmp ult i32 %index, %vocabulary
	br i1 %valid, label %lookup, label %invalid
lookup:
	%table.base = mul i32 %index, %dimensions
	%table.index = add i32 %table.base, %component
	%table.ptr = getelementptr inbounds double, ptr addrspace(1) %table, i32 %table.index
	%embedded = load double, ptr addrspace(1) %table.ptr, align 8
	br label %store
invalid:
	br label %store
store:
	%result = phi double [ %embedded, %lookup ], [ 0x7FF8000000000000, %invalid ]
	%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
	store double %result, ptr addrspace(1) %output.ptr, align 8
	ret void
}

define internal double @sigmoid(double %x) #1 {
entry:
	%negative = fneg double %x
	%exponential = call double @__ocml_exp_f64(double %negative)
	%denominator = fadd double 1.0, %exponential
	%value = fdiv double 1.0, %denominator
	ret double %value
}

define internal double @recurrent_linear(
	ptr addrspace(1) %input,
	ptr addrspace(1) %weights,
	ptr addrspace(1) %context,
	i32 %time,
	i32 %out,
	i32 %from,
	i32 %to,
	i32 %gate,
	i32 %operation,
	i32 %count
) #1 {
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

define internal void @recurrent_forward_body(
	ptr addrspace(1) %input,
	ptr addrspace(1) %weights,
	ptr addrspace(1) %output,
	ptr addrspace(1) %context,
	i32 %rows,
	i32 %from,
	i32 %to,
	i32 %operation,
	i32 %threads
) #1 {
entry:
	%tid = call i32 @llvm.amdgcn.workitem.id.x()
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
	%linear = call double @recurrent_linear(
		ptr addrspace(1) %input,
		ptr addrspace(1) %weights,
		ptr addrspace(1) %context,
		i32 %time,
		i32 %p,
		i32 %from,
		i32 %to,
		i32 %gate,
		i32 %operation,
		i32 %count
	)
	%tanh = call double @__ocml_tanh_f64(double %linear)
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
	%cell.state = call double @__ocml_tanh_f64(double %cell)
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
	call void @llvm.amdgcn.s.barrier()
	br i1 %is.gru, label %gru.loop, label %time.done
gru.loop:
	%gru.p = phi i32 [ %tid, %gate.barrier ], [ %gru.next, %gru.step ]
	%gru.more = icmp ult i32 %gru.p, %to
	br i1 %gru.more, label %gru.step, label %time.done
gru.step:
	%gru.linear = call double @recurrent_linear(
		ptr addrspace(1) %input,
		ptr addrspace(1) %weights,
		ptr addrspace(1) %context,
		i32 %time,
		i32 %gru.p,
		i32 %from,
		i32 %to,
		i32 2,
		i32 %operation,
		i32 %count
	)
	%candidate = call double @__ocml_tanh_f64(double %gru.linear)
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
	call void @llvm.amdgcn.s.barrier()
	%time.next = add i32 %time, 1
	br label %time.loop
exit:
	ret void
}

define internal double @attention_score(
	ptr addrspace(1) nocapture readonly %context,
	i32 %plane,
	i32 %row,
	i32 %head,
	i32 %query,
	i32 %key,
	i32 %from,
	i32 %length,
	i32 %head_width,
	double %scale
) #1 {
entry:
	%row.base = mul i32 %row, %from
	%head.start = mul i32 %head, %head_width
	br label %channel.loop
channel.loop:
	%offset = phi i32 [ 0, %entry ], [ %offset.next, %channel.step ]
	%sum = phi double [ 0.0, %entry ], [ %sum.next, %channel.step ]
	%more = icmp ult i32 %offset, %head_width
	br i1 %more, label %channel.step, label %done
channel.step:
	%channel = add i32 %head.start, %offset
	%channel.base = mul i32 %channel, %length
	%query.local = add i32 %channel.base, %query
	%key.local = add i32 %channel.base, %key
	%query.index = add i32 %row.base, %query.local
	%key.row.index = add i32 %row.base, %key.local
	%key.index = add i32 %plane, %key.row.index
	%query.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %query.index
	%key.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %key.index
	%query.value = load double, ptr addrspace(1) %query.ptr, align 8
	%key.value = load double, ptr addrspace(1) %key.ptr, align 8
	%product = fmul double %query.value, %key.value
	%sum.next = fadd double %sum, %product
	%offset.next = add i32 %offset, 1
	br label %channel.loop
done:
	%score = fdiv double %sum, %scale
	ret double %score
}

define internal void @attention_forward_body(
	ptr addrspace(1) nocapture readonly %input,
	ptr addrspace(1) nocapture readonly %weights,
	ptr addrspace(1) nocapture writeonly %output,
	ptr addrspace(1) %context,
	i32 %rows,
	i32 %from,
	i32 %heads,
	i32 %channels,
	i32 %threads
) #1 {
entry:
	%tid = call i32 @llvm.amdgcn.workitem.id.x()
	%length = udiv i32 %from, %channels
	%head_width = udiv i32 %channels, %heads
	%head_width.double = uitofp i32 %head_width to double
	%scale = call double @llvm.sqrt.f64(double %head_width.double)
	%plane = mul i32 %rows, %from
	%projection.count = mul i32 %plane, 3
	%matrix = mul i32 %channels, %channels
	br label %projection.loop
projection.loop:
	%projection.p = phi i32 [ %tid, %entry ], [ %projection.next, %projection.store ]
	%projection.more = icmp ult i32 %projection.p, %projection.count
	br i1 %projection.more, label %projection.step, label %projection.done
projection.step:
	%projection = udiv i32 %projection.p, %plane
	%within = urem i32 %projection.p, %plane
	%row = udiv i32 %within, %from
	%local = urem i32 %within, %from
	%output.channel = udiv i32 %local, %length
	%time = urem i32 %local, %length
	%row.base = mul i32 %row, %from
	%projection.weight.base = mul i32 %projection, %matrix
	%output.weight.base = mul i32 %output.channel, %channels
	%weight.base = add i32 %projection.weight.base, %output.weight.base
	br label %projection.channel.loop
projection.channel.loop:
	%input.channel = phi i32 [ 0, %projection.step ], [ %input.channel.next, %projection.channel.step ]
	%projection.sum = phi double [ 0.0, %projection.step ], [ %projection.sum.next, %projection.channel.step ]
	%channel.more = icmp ult i32 %input.channel, %channels
	br i1 %channel.more, label %projection.channel.step, label %projection.store
projection.channel.step:
	%input.channel.base = mul i32 %input.channel, %length
	%input.local = add i32 %input.channel.base, %time
	%input.index = add i32 %row.base, %input.local
	%weight.index = add i32 %weight.base, %input.channel
	%input.ptr = getelementptr inbounds double, ptr addrspace(1) %input, i32 %input.index
	%weight.ptr = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.index
	%input.value = load double, ptr addrspace(1) %input.ptr, align 8
	%weight.value = load double, ptr addrspace(1) %weight.ptr, align 8
	%projection.product = fmul double %input.value, %weight.value
	%projection.sum.next = fadd double %projection.sum, %projection.product
	%input.channel.next = add i32 %input.channel, 1
	br label %projection.channel.loop
projection.store:
	%context.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %projection.p
	store double %projection.sum, ptr addrspace(1) %context.ptr, align 8
	%projection.next = add i32 %projection.p, %threads
	br label %projection.loop
projection.done:
	call void @llvm.amdgcn.s.barrier()
	br label %output.loop
output.loop:
	%p = phi i32 [ %tid, %projection.done ], [ %p.next, %output.store ]
	%output.more = icmp ult i32 %p, %plane
	br i1 %output.more, label %output.step, label %exit
output.step:
	%output.row = udiv i32 %p, %from
	%output.local = urem i32 %p, %from
	%output.channel.index = udiv i32 %output.local, %length
	%query = urem i32 %output.local, %length
	%head = udiv i32 %output.channel.index, %head_width
	br label %maximum.loop
maximum.loop:
	%maximum.key = phi i32 [ 0, %output.step ], [ %maximum.next, %maximum.step ]
	%maximum = phi double [ 0xFFF0000000000000, %output.step ], [ %maximum.value, %maximum.step ]
	%maximum.more = icmp ult i32 %maximum.key, %length
	br i1 %maximum.more, label %maximum.step, label %softmax.loop
maximum.step:
	%score = call double @attention_score(
		ptr addrspace(1) %context,
		i32 %plane,
		i32 %output.row,
		i32 %head,
		i32 %query,
		i32 %maximum.key,
		i32 %from,
		i32 %length,
		i32 %head_width,
		double %scale
	)
	%larger = fcmp ogt double %score, %maximum
	%maximum.value = select i1 %larger, double %score, double %maximum
	%maximum.next = add i32 %maximum.key, 1
	br label %maximum.loop
softmax.loop:
	%key = phi i32 [ 0, %maximum.loop ], [ %key.next, %softmax.step ]
	%denominator = phi double [ 0.0, %maximum.loop ], [ %denominator.next, %softmax.step ]
	%numerator = phi double [ 0.0, %maximum.loop ], [ %numerator.next, %softmax.step ]
	%key.more = icmp ult i32 %key, %length
	br i1 %key.more, label %softmax.step, label %output.store
softmax.step:
	%softmax.score = call double @attention_score(
		ptr addrspace(1) %context,
		i32 %plane,
		i32 %output.row,
		i32 %head,
		i32 %query,
		i32 %key,
		i32 %from,
		i32 %length,
		i32 %head_width,
		double %scale
	)
	%centered = fsub double %softmax.score, %maximum
	%exponential = call double @__ocml_exp_f64(double %centered)
	%denominator.next = fadd double %denominator, %exponential
	%value.plane = mul i32 %plane, 2
	%value.row = mul i32 %output.row, %from
	%value.channel.base = mul i32 %output.channel.index, %length
	%value.local = add i32 %value.channel.base, %key
	%value.row.index = add i32 %value.row, %value.local
	%value.index = add i32 %value.plane, %value.row.index
	%value.ptr = getelementptr inbounds double, ptr addrspace(1) %context, i32 %value.index
	%value = load double, ptr addrspace(1) %value.ptr, align 8
	%weighted = fmul double %exponential, %value
	%numerator.next = fadd double %numerator, %weighted
	%key.next = add i32 %key, 1
	br label %softmax.loop
output.store:
	%attention = fdiv double %numerator, %denominator
	%output.ptr = getelementptr inbounds double, ptr addrspace(1) %output, i32 %p
	store double %attention, ptr addrspace(1) %output.ptr, align 8
	%p.next = add i32 %p, %threads
	br label %output.loop
exit:
	ret void
}

define internal void @forward_body(
	ptr addrspace(1) nocapture readonly %samples,
	ptr addrspace(1) nocapture readonly %weights,
	ptr addrspace(1) nocapture readonly %value_pointers,
	ptr addrspace(1) nocapture readonly %context_pointers,
	ptr addrspace(1) nocapture readonly %descriptors,
	ptr addrspace(1) nocapture readonly %parameters,
	i32 %rows,
	i32 %stages,
	i32 %threads
) #1 {
entry:
	%tid = call i32 @llvm.amdgcn.workitem.id.x()
	br label %stage.loop
stage.loop:
	%stage = phi i32 [ 0, %entry ], [ %stage.next, %stage.done ]
	%stage.more = icmp ult i32 %stage, %stages
	br i1 %stage.more, label %stage.step, label %exit
stage.step:
	%descriptor.base = mul i32 %stage, 5
	%to.index = add i32 %descriptor.base, 1
	%weight.index = add i32 %descriptor.base, 2
	%operation.index = add i32 %descriptor.base, 3
	%activation.index = add i32 %descriptor.base, 4
	%from.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %descriptor.base
	%to.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %to.index
	%weight.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %weight.index
	%operation.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %operation.index
	%activation.ptr = getelementptr inbounds i32, ptr addrspace(1) %descriptors, i32 %activation.index
	%from = load i32, ptr addrspace(1) %from.ptr, align 4
	%to = load i32, ptr addrspace(1) %to.ptr, align 4
	%weight.offset = load i32, ptr addrspace(1) %weight.ptr, align 4
	%operation = load i32, ptr addrspace(1) %operation.ptr, align 4
	%activation = load i32, ptr addrspace(1) %activation.ptr, align 4
	%parameter.base = mul i32 %stage, 2
	%secondary.index = add i32 %parameter.base, 1
	%parameter.ptr = getelementptr inbounds double, ptr addrspace(1) %parameters, i32 %parameter.base
	%secondary.ptr = getelementptr inbounds double, ptr addrspace(1) %parameters, i32 %secondary.index
	%parameter = load double, ptr addrspace(1) %parameter.ptr, align 8
	%secondary = load double, ptr addrspace(1) %secondary.ptr, align 8
	%value.slot = getelementptr inbounds i64, ptr addrspace(1) %value_pointers, i32 %stage
	%context.slot = getelementptr inbounds i64, ptr addrspace(1) %context_pointers, i32 %stage
	%value.address = load i64, ptr addrspace(1) %value.slot, align 8
	%context.address = load i64, ptr addrspace(1) %context.slot, align 8
	%values = inttoptr i64 %value.address to ptr addrspace(1)
	%context = inttoptr i64 %context.address to ptr addrspace(1)
	%first = icmp eq i32 %stage, 0
	%previous.stage = sub i32 %stage, 1
	%safe.previous = select i1 %first, i32 0, i32 %previous.stage
	%previous.slot = getelementptr inbounds i64, ptr addrspace(1) %value_pointers, i32 %safe.previous
	%previous.address = load i64, ptr addrspace(1) %previous.slot, align 8
	%previous.values = inttoptr i64 %previous.address to ptr addrspace(1)
	%source = select i1 %first, ptr addrspace(1) %samples, ptr addrspace(1) %previous.values
	%matrix = getelementptr inbounds double, ptr addrspace(1) %weights, i32 %weight.offset
	%count = mul i32 %rows, %to
	switch i32 %operation, label %invalid [
		i32 0, label %normal
		i32 1, label %normal
		i32 2, label %normal
		i32 4, label %normal
		i32 5, label %attention
		i32 6, label %recurrent
		i32 7, label %recurrent
		i32 8, label %recurrent
		i32 11, label %normal
	]
attention:
	%heads = fptoui double %parameter to i32
	%channels = fptoui double %secondary to i32
	call void @attention_forward_body(
		ptr addrspace(1) %source,
		ptr addrspace(1) %matrix,
		ptr addrspace(1) %values,
		ptr addrspace(1) %context,
		i32 %rows,
		i32 %from,
		i32 %heads,
		i32 %channels,
		i32 %threads
	)
	br label %computed
recurrent:
	call void @recurrent_forward_body(
		ptr addrspace(1) %source,
		ptr addrspace(1) %matrix,
		ptr addrspace(1) %values,
		ptr addrspace(1) %context,
		i32 %rows,
		i32 %from,
		i32 %to,
		i32 %operation,
		i32 %threads
	)
	br label %computed
normal:
	br label %normal.loop
normal.loop:
	%p = phi i32 [ %tid, %normal ], [ %p.next, %normal.next ]
	%normal.more = icmp ult i32 %p, %count
	br i1 %normal.more, label %normal.step, label %computed
normal.step:
	switch i32 %operation, label %invalid [
		i32 0, label %dense
		i32 1, label %convolution
		i32 2, label %pool
		i32 4, label %embedding
		i32 11, label %dense
	]
dense:
	call void @dense_body(
		ptr addrspace(1) %source,
		ptr addrspace(1) %matrix,
		ptr addrspace(1) %values,
		i32 %rows,
		i32 %from,
		i32 %to,
		i32 1,
		i32 %p
	)
	br label %normal.next
convolution:
	%kernel = fptoui double %parameter to i32
	%input.channels = fptoui double %secondary to i32
	call void @conv_forward_body(
		ptr addrspace(1) %source,
		ptr addrspace(1) %matrix,
		ptr addrspace(1) %values,
		i32 %p,
		i32 %from,
		i32 %to,
		i32 %kernel,
		i32 %input.channels
	)
	br label %normal.next
pool:
	%size = fptoui double %parameter to i32
	%pool.channels = fptoui double %secondary to i32
	call void @pool_forward_body(
		ptr addrspace(1) %source,
		ptr addrspace(1) %values,
		i32 %p,
		i32 %from,
		i32 %to,
		i32 %size,
		i32 %pool.channels
	)
	br label %normal.next
embedding:
	%vocabulary = fptoui double %parameter to i32
	call void @embedding_forward_body(
		ptr addrspace(1) %source,
		ptr addrspace(1) %matrix,
		ptr addrspace(1) %values,
		i32 %p,
		i32 %from,
		i32 %to,
		i32 %vocabulary
	)
	br label %normal.next
normal.next:
	%p.next = add i32 %p, %threads
	br label %normal.loop
computed:
	call void @llvm.amdgcn.s.barrier()
	%is.residual = icmp eq i32 %operation, 11
	%branch.relu = fcmp one double %parameter, 0.0
	br label %activation.loop
activation.loop:
	%activation.p = phi i32 [ %tid, %computed ], [ %activation.next, %activation.step ]
	%activation.more = icmp ult i32 %activation.p, %count
	br i1 %activation.more, label %activation.step, label %stage.done
activation.step:
	%value.ptr = getelementptr inbounds double, ptr addrspace(1) %values, i32 %activation.p
	%value = load double, ptr addrspace(1) %value.ptr, align 8
	%branch.ordered = fcmp ord double %value, %value
	%branch.positive = fcmp ogt double %value, 0.0
	%branch.clipped = select i1 %branch.positive, double %value, double 0.0
	%branch.valid = select i1 %branch.ordered, double %branch.clipped, double %value
	%branch.value = select i1 %branch.relu, double %branch.valid, double %value
	%row = udiv i32 %activation.p, %to
	%column = urem i32 %activation.p, %to
	%safe.column = urem i32 %column, %from
	%row.base = mul i32 %row, %from
	%skip.index = add i32 %row.base, %safe.column
	%skip.ptr = getelementptr inbounds double, ptr addrspace(1) %source, i32 %skip.index
	%skip = load double, ptr addrspace(1) %skip.ptr, align 8
	%residual.value = fadd double %branch.value, %skip
	%operation.value = select i1 %is.residual, double %residual.value, double %value
	%is.relu = icmp eq i32 %activation, 8
	%activation.ordered = fcmp ord double %operation.value, %operation.value
	%activation.positive = fcmp ogt double %operation.value, 0.0
	%activation.clipped = select i1 %activation.positive, double %operation.value, double 0.0
	%activation.valid = select i1 %activation.ordered, double %activation.clipped, double %operation.value
	%result = select i1 %is.relu, double %activation.valid, double %operation.value
	store double %result, ptr addrspace(1) %value.ptr, align 8
	%activation.next = add i32 %activation.p, %threads
	br label %activation.loop
stage.done:
	call void @llvm.amdgcn.s.barrier()
	%stage.next = add i32 %stage, 1
	br label %stage.loop
invalid:
	call void @llvm.trap()
	br label %exit
exit:
	ret void
}

define protected amdgpu_kernel void @forward_graph(
	ptr addrspace(1) nocapture readonly %samples,
	ptr addrspace(1) nocapture readonly %weights,
	ptr addrspace(1) nocapture readonly %value_pointers,
	ptr addrspace(1) nocapture readonly %context_pointers,
	ptr addrspace(1) nocapture readonly %descriptors,
	ptr addrspace(1) nocapture readonly %parameters,
	i32 %rows,
	i32 %stages,
	i32 %threads
) #0 {
entry:
	call void @forward_body(
		ptr addrspace(1) %samples,
		ptr addrspace(1) %weights,
		ptr addrspace(1) %value_pointers,
		ptr addrspace(1) %context_pointers,
		ptr addrspace(1) %descriptors,
		ptr addrspace(1) %parameters,
		i32 %rows,
		i32 %stages,
		i32 %threads
	)
	ret void
}

attributes #0 = { nounwind "amdgpu-flat-work-group-size"="1,1024" }
attributes #1 = { alwaysinline nounwind }
