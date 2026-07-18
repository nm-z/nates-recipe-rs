#pragma once
#include <cstddef>

/// Dtype ABI shared with Rust: 0 = double, 1 = float.
#define DT_DISPATCH(dtype, ...)                                                \
      do {                                                                     \
            if ((dtype) == 1) {                                                \
                  using T = float;                                             \
                  __VA_ARGS__;                                                 \
            } else {                                                           \
                  using T = double;                                            \
                  __VA_ARGS__;                                                 \
            }                                                                  \
      } while (0)

/// Host-side element size for shared-memory byte math.
static inline size_t dt_elem_size(int dtype) { return dtype == 1 ? 4 : 8; }
