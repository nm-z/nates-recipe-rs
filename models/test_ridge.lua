local X = upload({1,2, 3,4, 5,6, 7,8, 9,10}, 5, 2)
local y = upload({5, 11, 17, 23, 29}, 5, 1)
local l = 2

function Ridge(X,y,l)
      local XtX = gemm(X,X,"T","N")        -- rocblas_dgemm
      local R = diag_add(XtX,l)            -- hip_diag_add
      local Xty = gemm(X,y,"T","N")        -- rocblas_dgemm
      local W = solve(R,Xty)               -- rocsolver_dgesv
      local yh = gemm(X,W,"N","N")         -- rocblas_dgemm
      print(download(yh))
      return yh
end
Ridge(X, y, l)



