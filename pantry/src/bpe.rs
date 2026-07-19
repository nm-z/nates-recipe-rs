//! Byte-Pair Encoding — host-side encoder, no GPU. UTF-8 bytes in, token ids
//! out. The merge table is an ordered list of adjacent-pair merges (list order
//! = rank, lowest merges first); encode repeatedly collapses the lowest-ranked
//! applicable pair until no adjacent pair remains in the table. Pure CPU work,
//! knows nothing of models — the byte-id companion to `detect`'s token stream.

use anyhow::{Result, bail};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pair {
	pub left: u32,
	pub right: u32,
}

#[derive(Clone, Copy)]
pub struct Merge {
	pub rank: u32,
	pub merged_id: u32,
}

pub struct MergeTable {
	ranks: HashMap<Pair, Merge>,
}

impl MergeTable {
	pub fn from_merges(merges: Vec<Pair>) -> Result<Self> {
		let mut ranks = HashMap::with_capacity(merges.len());
		let mut rank: u32 = 0;
		for pair in merges {
			let merged_id = 256 + rank;
			let None = ranks.insert(pair, Merge { rank, merged_id }) else {
				bail!(
					"duplicate merge pair ({}, {}) at rank {rank}",
					pair.left,
					pair.right
				);
			};
			rank += 1;
		}
		Ok(Self { ranks })
	}

	pub fn lookup(&self, a: u32, b: u32) -> Option<Merge> {
		self.ranks.get(&Pair { left: a, right: b }).copied()
	}

	pub fn len(&self) -> usize {
		self.ranks.len()
	}

	pub fn is_empty(&self) -> bool {
		self.ranks.is_empty()
	}
}

#[derive(Clone, Copy)]
struct Best {
	rank: u32,
	pos: usize,
	merged_id: u32,
}

pub fn encode(bytes: &[u8], table: &MergeTable) -> Vec<u32> {
	let mut ids: Vec<u32> = bytes.iter().map(|&b| u32::from(b)).collect();
	loop {
		let mut best: Option<Best> = None;
		for i in 0..ids.len().saturating_sub(1) {
			let Some(m) = table.lookup(ids[i], ids[i + 1]) else {
				continue;
			};
			let cand = Best {
				rank: m.rank,
				pos: i,
				merged_id: m.merged_id,
			};
			let better = match best {
				None => Some(cand),
				Some(cur) => Some(cand).filter(|c| c.rank < cur.rank),
			};
			best = better.or(best);
		}
		let Some(chosen) = best else { break };
		ids[chosen.pos] = chosen.merged_id;
		ids.remove(chosen.pos + 1);
	}
	ids
}
