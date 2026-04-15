use anyhow::Result;

fn main() -> Result<()> {
      let args: Vec<String> = std::env::args().collect();
      if args.len() < 2 {
            eprintln!("usage: recipe <model.lua>");
            std::process::exit(1);
      }

      let lua = mlua::Lua::new();
      nates_recipe::lua_runtime::init(&lua)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

      let code = std::fs::read_to_string(&args[1])?;
      match lua.load(&code).set_name(&args[1]).eval::<mlua::Value>() {
            Ok(mlua::Value::Nil) => {}
            Ok(val) => println!("{}", val.to_string().unwrap_or_default()),
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
      }

      gpu_core::kernels::gpu_shutdown();
      Ok(())
}
