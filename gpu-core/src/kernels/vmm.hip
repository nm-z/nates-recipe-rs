// VRAM tier of the tiered buffer — HIP virtual-memory-management wrappers.
// Each physical page is a fixed-size hipMemCreate handle; a contiguous VA range
// is reserved once and handles are mapped/unmapped into P-sized windows of it on
// demand. These extern "C" shims hide the prop/access structs so the Rust side
// deals only in raw pointers and sizes. hipMemGenericAllocationHandle_t is an
// opaque pointer type, carried across the FFI boundary as void*.
#include <hip/hip_runtime.h>

static hipMemAllocationProp vmm_prop(int dev) {
      hipMemAllocationProp prop = {};
      prop.type = hipMemAllocationTypePinned;
      prop.location.type = hipMemLocationTypeDevice;
      prop.location.id = dev;
      return prop;
}

// Minimum mappable granularity for device 0 — the true page floor; P must be a
// multiple of this.
extern "C" int vmm_granularity(size_t* out) {
      hipMemAllocationProp prop = vmm_prop(0);
      return (int)hipMemGetAllocationGranularity(out, &prop, hipMemAllocationGranularityMinimum);
}

// One physical page of device memory. This is the only call that consumes VRAM.
extern "C" int vmm_create(void** handle_out, size_t size) {
      hipMemAllocationProp prop = vmm_prop(0);
      hipMemGenericAllocationHandle_t h;
      int rc = (int)hipMemCreate(&h, size, &prop, 0);
      if (rc == 0) *handle_out = (void*)h;
      return rc;
}

// Reserve contiguous virtual address space (no physical backing) — the ring.
extern "C" int vmm_reserve(void** va_out, size_t size) {
      return (int)hipMemAddressReserve(va_out, size, 0, 0, 0);
}

// Back one VA window with a physical handle and grant the device R/W access.
extern "C" int vmm_map_at(void* va, size_t size, void* handle) {
      hipMemGenericAllocationHandle_t h = (hipMemGenericAllocationHandle_t)handle;
      int rc = (int)hipMemMap(va, size, 0, h, 0);
      if (rc != 0) return rc;
      hipMemAccessDesc desc = {};
      desc.location.type = hipMemLocationTypeDevice;
      desc.location.id = 0;
      desc.flags = hipMemAccessFlagsProtReadWrite;
      return (int)hipMemSetAccess(va, size, &desc, 1);
}

extern "C" int vmm_unmap(void* va, size_t size) {
      return (int)hipMemUnmap(va, size);
}

extern "C" int vmm_release(void* handle) {
      return (int)hipMemRelease((hipMemGenericAllocationHandle_t)handle);
}

extern "C" int vmm_addr_free(void* va, size_t size) {
      return (int)hipMemAddressFree(va, size);
}
