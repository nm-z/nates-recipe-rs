#pragma once
#include <cstddef>
#include <cstdio>
#include <cstdlib>

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

#define DT_DISPATCH_F(dtype, ...)                                              \
      do {                                                                     \
            if ((dtype) == 1) {                                                \
                  using T = float;                                             \
                  __VA_ARGS__;                                                 \
            } else if ((dtype) == 0) {                                         \
                  using T = double;                                            \
                  __VA_ARGS__;                                                 \
            } else if ((dtype) == 2) {                                         \
                  using T = __half;                                            \
                  __VA_ARGS__;                                                 \
            } else if ((dtype) == 3) {                                         \
                  using T = __hip_bfloat16;                                    \
                  __VA_ARGS__;                                                 \
            } else {                                                           \
                  fprintf(stderr, "DT_DISPATCH_F: unsupported dtype %d\n",     \
                        (int)(dtype));                                         \
                  abort();                                                     \
            }                                                                  \
      } while (0)

static inline size_t dt_elem_size(int dtype) {
      if (dtype == 1) return 4;
      if (dtype == 2 || dtype == 3) return 2;
      return 8;
}
