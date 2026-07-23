declare i32 @rocsolver_create_handle(ptr)

define void @recipe_owned_kernel() {
  %status = call i32 @cudnnCreate(ptr null)
  ret void
}
