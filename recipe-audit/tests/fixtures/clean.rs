fn architecture_relationship() {
    hsa_queue_create(agent, 64, queue_type, callback, data, 0, 0, queue);
    cuLaunchKernel(function, 1, 1, 1, 1, 1, 1, 0, stream, args, extra);
}
