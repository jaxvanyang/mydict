use super::{Trie, read_odict_from_path};
use crate::{elapsed_secs, now};
use std::path::Path;
use tracing::info;

/// Not useful on its own, you should use the `LazyDict`.
#[derive(Debug, Clone)]
pub struct Dictionary {
	pub(crate) odict: odict::Dictionary,
	pub(crate) trie: Trie,
}

impl Dictionary {
	pub fn new(odict: odict::Dictionary) -> Self {
		let t0 = now();
		let mut trie = Trie::new();
		for term in odict.entries.keys() {
			trie.insert(term);
		}
		tracing::info!(
			"build trie for {} in {:.3}s",
			odict
				.name
				.as_ref()
				.map_or("unknown".to_string(), Clone::clone),
			elapsed_secs(&t0)
		);
		Self { odict, trie }
	}

	/// # Errors
	///
	/// Will return `Err` if `path` or the file is not valid
	pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
		let t0 = now();
		let dict = read_odict_from_path(path)?.into();
		info!("load {:?} in {:.3}s", path, elapsed_secs(&t0));

		Ok(dict)
	}
}

impl From<odict::Dictionary> for Dictionary {
	fn from(dict: odict::Dictionary) -> Self {
		Self::new(dict)
	}
}
