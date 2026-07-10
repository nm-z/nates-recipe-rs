use anyhow::Result;
use recipe::wire::{Conn, FN_MOE_FFN, NodeInfo, Op, Server};
use std::thread;

#[test]
fn wire_loopback_roundtrip() -> Result<()> {
      let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
      let addr = listener.local_addr()?.to_string();
      let server = Server::new(NodeInfo::probe(), std::collections::HashMap::new());
      thread::spawn(move || {
            let _ = server.serve_on(listener);
      });

      let c = Conn::connect(&addr)?;
      let mb = 2usize;
      let mut blob = vec![0u8; mb << 20];
      let mut x = 0x9e37_79b9_7f4a_7c15u64;
      for w in blob.chunks_exact_mut(8) {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            w.copy_from_slice(&x.to_le_bytes());
      }
      c.store_from(42, &blob)?;
      let back = c.fetch(42, 0, blob.len() as u64)?;
      assert_eq!(back, blob, "full fetch must round-trip the stored blob");
      let off = 1usize << 20;
      let slice = c.fetch(42, off as u64, 4096)?;
      assert_eq!(&slice[..], &blob[off..off + 4096], "offset fetch must match source");
      let past = c.fetch(42, blob.len() as u64, 1);
      assert!(past.is_err(), "fetch past end must error");
      let no_runner = c.run(FN_MOE_FFN, 42, vec![0u8; 8])?.recv()?;
      assert_eq!(no_runner.op, Op::Err, "RUN with no handler must return Err frame");
      Ok(())
}
