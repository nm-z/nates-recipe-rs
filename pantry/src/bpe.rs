//! Byte-Pair Encoding — host-side encoder, no GPU. UTF-8 bytes in, token ids
//! out. The merge table is an ordered list of adjacent-pair merges (list order
//! = rank, lowest merges first); encode repeatedly collapses the lowest-ranked
//! applicable pair until no adjacent pair remains in the table. Pure CPU work,
//! knows nothing of models — the byte-id companion to `detect`'s token stream.

use anyhow::{Result, bail};
use std::collections::HashMap;

/// An ordered BPE merge table. Maps each adjacent symbol pair to its `rank`
/// (training order — lower merges first) and the `merged_id` it collapses to.
/// Base byte symbols hold ids `0..=255`; the `i`-th merge yields id `256 + i`.
pub struct MergeTable {
	ranks: HashMap<(u32, u32), (u32, u32)>,
}

impl MergeTable {
	/// Build from an ordered list of `(left_id, right_id)` merges. List index is
	/// the rank (0 merges first); merge `i` produces id `256 + i`. Fails if a
	/// pair is listed twice — its collapse target would be ambiguous.
	pub fn from_merges(merges: Vec<(u32, u32)>) -> Result<Self> {
		let mut ranks = HashMap::with_capacity(merges.len());
		for (i, pair) in merges.into_iter().enumerate() {
			let rank = i as u32;
			let merged_id = 256 + rank;
			if ranks.insert(pair, (rank, merged_id)).is_some() {
				bail!("duplicate merge pair {pair:?} at rank {rank}");
			}
		}
		Ok(Self { ranks })
	}

	/// `(rank, merged_id)` for an adjacent pair, or `None` if it is not a merge.
	pub fn lookup(&self, a: u32, b: u32) -> Option<(u32, u32)> {
		self.ranks.get(&(a, b)).copied()
	}

	/// Number of merges in the table.
	pub fn len(&self) -> usize {
		self.ranks.len()
	}

	pub fn is_empty(&self) -> bool {
		self.ranks.is_empty()
	}
}

/// Encode raw UTF-8 bytes to BPE token ids. Symbols start as the bytes
/// themselves (`0..=255`); the adjacent pair with the lowest rank present in
/// `table` is collapsed into its merged id, repeatedly, until no adjacent pair
/// is in the table. Ties go to the leftmost occurrence. Empty input → empty.
pub fn encode(bytes: &[u8], table: &MergeTable) -> Vec<u32> {
	let mut ids: Vec<u32> = bytes.iter().map(|&b| u32::from(b)).collect();
	loop {
		let mut best: Option<(u32, usize, u32)> = None;
		for i in 0..ids.len().saturating_sub(1) {
			if let Some((rank, merged_id)) = table.lookup(ids[i], ids[i + 1]) {
				if best.is_none_or(|(br, ..)| rank < br) {
					best = Some((rank, i, merged_id));
				}
			}
		}
		let Some((_, pos, merged_id)) = best else { break };
		ids[pos] = merged_id;
		ids.remove(pos + 1);
	}
	ids
}
