#[cfg(test)]
mod lua_gpu_tests {
      #[test]
      fn test_lua_gpu_buffer_roundtrip() {
            gpu_core::hip::set_device(0).expect("GPU set_device failed");
            let lua = mlua::Lua::new();
            crate::lua_runtime::init(&lua).expect("init failed");

            let code = r#"
                  local buf = upload({1.0, 2.0, 3.0, 4.0, 5.0, 6.0}, 2, 3)
                  assert(buf:rows() == 2)
                  assert(buf:cols() == 3)
                  local data = download(buf)
                  assert(#data == 6)
                  assert(data[1] == 1.0)
                  assert(data[6] == 6.0)
                  return true
            "#;
            let result: bool = lua.load(code).eval().expect("Lua eval failed");
            assert!(result);
      }

      #[test]
      fn test_lua_gemm() {
            gpu_core::hip::set_device(0).expect("GPU set_device failed");
            let lua = mlua::Lua::new();
            crate::lua_runtime::init(&lua).expect("init failed");

            let code = r#"
                  local A = upload({1,2,3, 4,5,6}, 2, 3)
                  local B = upload({1,4, 2,5, 3,6}, 3, 2)
                  local C = gemm(A, B, "N", "N")
                  assert(C:rows() == 2)
                  assert(C:cols() == 2)
                  local d = download(C)
                  assert(math.abs(d[1] - 14) < 0.01, "expected 14 got " .. d[1])
                  assert(math.abs(d[2] - 32) < 0.01, "expected 32 got " .. d[2])
                  assert(math.abs(d[3] - 32) < 0.01, "expected 32 got " .. d[3])
                  assert(math.abs(d[4] - 77) < 0.01, "expected 77 got " .. d[4])
                  return true
            "#;
            let result: bool = lua.load(code).eval().expect("Lua eval failed");
            assert!(result);
      }

      #[test]
      fn test_lua_ridge_inline() {
            gpu_core::hip::set_device(0).expect("GPU set_device failed");
            let lua = mlua::Lua::new();
            crate::lua_runtime::init(&lua).expect("init failed");

            let code = r#"
                  local X = upload({1,2, 3,4, 5,6, 7,8, 9,10}, 5, 2)
                  local y = upload({5, 11, 17, 23, 29}, 5, 1)
                  local XtX = gemm(X, X, "T", "N")
                  local R = diag_add(XtX, 0.01)
                  local Xty = gemm(X, y, "T", "N")
                  local W = solve(R, Xty)
                  local yh = gemm(X, W, "N", "N")
                  local d = download(yh)
                  assert(math.abs(d[1] - 5) < 0.1, "expected ~5 got " .. d[1])
                  assert(math.abs(d[5] - 29) < 0.1, "expected ~29 got " .. d[5])
                  return true
            "#;
            let result: bool = lua.load(code).eval().expect("Lua eval failed");
            assert!(result);
      }
}
