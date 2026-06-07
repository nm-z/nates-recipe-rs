use anyhow::Result;

fn main() -> Result<()> {
      let args: Vec<String> = std::env::args().collect();
      if args.len() < 2 {
            eprintln!("usage: recipe <train.csv> [--target <col>]");
            std::process::exit(1);
      }

      gpu_core::hip::set_device(0)?;

      let mut target = None;
      let mut path = &args[1];
      let mut i = 2;
      while i < args.len() {
            match args[i].as_str() {
                  "--target" => {
                        target = Some(args[i + 1].as_str());
                        i += 2;
                  }
                  _ => {
                        path = &args[i];
                        i += 1;
                  }
            }
      }

      let mut loader = nates_recipe::Data::load().set(path);
      if let Some(t) = target {
            loader = loader.target(t);
      }
      let (train, _test) = loader.prepare();
      eprintln!("loaded {} samples × {} features", train.x.nrows(), train.x.ncols());

      gpu_core::kernels::gpu_shutdown();
      Ok(())
}
