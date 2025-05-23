use tracing::warn;

use super::Dictionary;
use std::path::PathBuf;

pub struct LazyDict {
	pub path: PathBuf,
	dictionary: Option<Dictionary>,
	/// Used for sync
	pub is_loading: bool,
}

impl LazyDict {
	#[must_use]
	pub fn new(path: PathBuf) -> Self {
		Self {
			path,
			dictionary: None,
			is_loading: false,
		}
	}

	#[must_use]
	pub fn is_loaded(&self) -> bool {
		self.dictionary.is_some()
	}

	pub fn load(&mut self, dictionary: Dictionary) {
		if self.is_loaded() {
			warn!("dictionary {:?} is already loaded", self.path);
		}
		self.dictionary = Some(dictionary);
	}

	/// # Errors
	///
	/// Will return `Err` if dictionary is not loaded
	pub fn search(&self, s: &str) -> anyhow::Result<Vec<String>> {
		match &self.dictionary {
			Some(dict) => Ok(dict.trie.search(s)),
			None => Err(anyhow::anyhow!("dictionary {:?} is not loaded", self.path)),
		}
	}

	/// # Errors
	///
	/// Will return `Err` if dictionary is not loaded
	pub fn get(&self, s: &str) -> anyhow::Result<Option<&odict::Entry>> {
		match &self.dictionary {
			Some(dict) => Ok(dict.odict.entries.get(s)),
			None => Err(anyhow::anyhow!("dictionary {:?} is not loaded", self.path)),
		}
	}

	/// # Panics
	///
	/// Will panic if `self.path` is not valid
	#[must_use]
	pub fn name(&self) -> String {
		let stem = self.path.file_stem().unwrap().to_str().unwrap().to_owned();

		if let Some(dict) = &self.dictionary {
			dict.odict.name.clone().unwrap_or(stem)
		} else {
			stem
		}
	}
}
