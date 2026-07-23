fn legacy_backend() {
    unsafe {
        hipMalloc(pointer, 4096);
        cudaMemcpyAsync(dst, src, 4096, kind, stream);
    }
    let library = Library::new("libcublas.so.12");
}
