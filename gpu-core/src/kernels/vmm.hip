#include <hip/hip_runtime.h>

static hipMemAllocationProp vmm_prop(int dev) {
      hipMemAllocationProp prop = {};
      prop.type = hipMemAllocationTypePinned;
      prop.location.type = hipMemLocationTypeDevice;
      prop.location.id = dev;
      return prop;
}

extern "C" int vmm_granularity(size_t* out) {
      hipMemAllocationProp prop = vmm_prop(0);
      return (int)hipMemGetAllocationGranularity(out, &prop, hipMemAllocationGranularityMinimum);
}

extern "C" int vmm_create(void** handle_out, size_t size) {
      hipMemAllocationProp prop = vmm_prop(0);
      hipMemGenericAllocationHandle_t h;
      int rc = (int)hipMemCreate(&h, size, &prop, 0);
      if (rc == 0) *handle_out = (void*)h;
      return rc;
}

extern "C" int vmm_reserve(void** va_out, size_t size) {
      return (int)hipMemAddressReserve(va_out, size, 0, 0, 0);
}

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
