use super::Trie;
use crate::{elapsed_secs, now};
use std::path::Path;

/// Not useful on its own, you should use the `LazyDict`.
pub struct Dictionary {
	pub(crate) odict: odict::Dictionary,
	pub(crate) trie: Trie,
}

impl From<odict::Dictionary> for Dictionary {
	fn from(dict: odict::Dictionary) -> Self {
		Self::new(dict)
	}
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
	pub fn read_from_path(path: &Path) -> anyhow::Result<Self> {
		let reader = odict::DictionaryReader::new();
		let dict_file = reader.read_from_path(
			path.to_str()
				.ok_or(anyhow::anyhow!("path is not valid unicode: {path:?}"))?,
		)?;
		Ok(Self::new(dict_file.to_dictionary()?))
	}
}
