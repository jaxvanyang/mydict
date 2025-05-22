use super::Dictionary;
use crate::{elapsed_secs, now};
use std::path::PathBuf;
use tracing::{info, warn};

pub struct LazyDict {
	pub path: PathBuf,
	pub dictionary: Option<Dictionary>,
}

impl LazyDict {
	#[must_use]
	pub fn new(path: PathBuf) -> Self {
		Self {
			path,
			dictionary: None,
		}
	}

	#[must_use]
	pub fn is_loaded(&self) -> bool {
		self.dictionary.is_some()
	}

	/// # Errors
	///
	/// Will return `Err` if read dictionary from `self.path` failed
	pub fn load(&mut self) -> anyhow::Result<()> {
		if self.is_loaded() {
			warn!("{:?} is already loaded, load again now", self.path);
		}

		let t0 = now();
		self.dictionary = Some(Dictionary::read_from_path(&self.path)?);
		info!("load {:?} in {:.3}s", self.path, elapsed_secs(&t0));

		Ok(())
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
