// The three import styles, each exercised from a real consumer crate. All read
// and write the same graph via itnl/file/add/del and check the value back.

const SRC: &str = "engi\n    GPU0\n        VRAM 12\n        FLOPs 380\n    CPU\n        RAM 31\n";

fn tmp(tag: &str) -> String {
      std::env::temp_dir().join(format!("nrs_ogdl_style_{tag}.ogdl")).to_str().expect("utf8").to_string()
}

// ── style 1: static (dot syntax), glob import brings Chain in for free ──
#[test]
fn style1_static() {
      use ogdl::*;
      let p = tmp("s1");
      std::fs::write(&p, SRC).expect("seed");
      // start of chain: reads the file into the internal graph, then bind it.
      ogdl.file(&p).itnl(());
      assert_eq!(format!("{}", ogdl.itnl("engi.GPU0.VRAM")), "12");
      // mutate, then write it back out at the end of a chain.
      let cpu = ogdl.itnl("engi.CPU");
      ogdl.add("HTT", &cpu);
      ogdl.itnl(()).file(&p);
      assert!(std::fs::read_to_string(&p).expect("read").contains("HTT"));
}

// ── style 2: struct import (associated fn) ──
#[test]
fn style2_struct() {
      use ogdl::{Chain, Ogdl};
      let p = tmp("s2");
      std::fs::write(&p, SRC).expect("seed");
      let graph = Ogdl::file(&p).itnl(());
      assert_eq!(format!("{}", graph.itnl("engi.GPU0.FLOPs")), "380");
      graph.itnl(()).file(&p); // round-trip write
      assert_eq!(std::fs::read_to_string(&p).expect("read"), SRC);
}

// ── style 3: crate path (free fn) ──
#[test]
fn style3_free() {
      use ogdl::Chain;
      let p = tmp("s3");
      std::fs::write(&p, SRC).expect("seed");
      let graph = ogdl::file(&p).itnl(());
      assert_eq!(format!("{}", graph.itnl("engi.CPU.RAM")), "31");
      graph.itnl(()).file(&p);
      assert_eq!(std::fs::read_to_string(&p).expect("read"), SRC);
}
