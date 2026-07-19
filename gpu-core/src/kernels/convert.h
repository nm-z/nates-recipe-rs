#pragma once

/// Dtype codes shared with Rust's `Dtype::conv()`. The float codes match the
/// `dtype_dispatch.h` ABI (0 = double, 1 = float) so a buffer's compute code
/// and its convert code never disagree.
#define CV_F64 0
#define CV_F32 1
#define CV_F16 2
#define CV_BF16 3
#define CV_Q4_0 10
#define CV_Q4_1 11
#define CV_Q5_0 12
#define CV_Q5_1 13
#define CV_Q8_0 14
#define CV_Q2_K 20
#define CV_Q3_K 21
#define CV_Q4_K 22
#define CV_Q5_K 23
#define CV_Q6_K 24

/// Block geometry lives with the dtype on the Rust side (`Dtype::block_elems`
/// and `Dtype::block_bytes`), which is what validates a conversion length and
/// sizes the destination. Device code derives each block's base address from
/// the literal layout inside its own `cv_load` case, so the two never drift
/// through a shared table that only one side reads.
