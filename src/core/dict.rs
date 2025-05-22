use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
};

use anyhow::Ok;
use odict::DictionaryReader;

use crate::{elapsed_secs, now};

#[derive(Debug)]
struct Trie {
	map: BTreeMap<u8, Trie>,
	is_end: bool,
}

impl Trie {
	pub fn new() -> Self {
		Self {
			map: BTreeMap::new(),
			is_end: false,
		}
	}

	pub fn insert(&mut self, s: &str) {
		let mut current = self;
		for byte in s.as_bytes() {
			current = current.map.entry(*byte).or_insert(Trie::new());
		}
		current.is_end = true;
	}

	fn lexicon_iter(&self, buffer: &mut Vec<u8>) -> Vec<String> {
		let mut result = Vec::new();
		if self.is_end {
			result.push(String::from_utf8(buffer.clone()).unwrap());
		}
		for (byte, next) in &self.map {
			buffer.push(*byte);
			result.extend(next.lexicon_iter(buffer));
			buffer.pop();
		}

		result
	}

	pub fn search(&self, s: &str) -> Vec<String> {
		let mut current = self;
		let mut buffer = Vec::new();
		for byte in s.as_bytes() {
			if !current.map.contains_key(byte) {
				return Vec::new();
			}

			buffer.push(*byte);
			current = &current.map[byte];
		}

		current.lexicon_iter(&mut buffer)
	}
}

pub struct Dictionary {
	odict: odict::Dictionary,
	trie: Trie,
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
		let reader = DictionaryReader::new();
		let dict_file = reader.read_from_path(
			path.to_str()
				.ok_or(anyhow::anyhow!("path is not valid unicode: {path:?}"))?,
		)?;
		Ok(Self::new(dict_file.to_dictionary()?))
	}
}

pub struct LazyDict {
	pub path: PathBuf,
	pub dict: Option<Dictionary>,
}

impl LazyDict {
	#[must_use]
	pub fn new(path: PathBuf) -> Self {
		Self { path, dict: None }
	}

	/// # Errors
	///
	/// Will return `Err` if read dictionary from `self.path` failed
	fn lazy_load(&mut self) -> anyhow::Result<()> {
		if self.dict.is_none() {
			let t0 = now();
			self.dict = Some(Dictionary::read_from_path(&self.path)?);
			tracing::info!("lazy load {:?} in {:.3}s", self.path, elapsed_secs(&t0));
		}

		Ok(())
	}

	/// # Errors
	///
	/// Will return `Err` if lazy load failed
	pub fn search(&mut self, s: &str) -> anyhow::Result<Vec<String>> {
		self.lazy_load()?;
		match &self.dict {
			Some(dict) => Ok(dict.trie.search(s)),
			None => unreachable!(),
		}
	}

	/// # Errors
	///
	/// Will return `Err` if lazy load failed
	pub fn entries(&mut self) -> anyhow::Result<&HashMap<String, odict::Entry>> {
		self.lazy_load()?;
		match &self.dict {
			Some(dict) => Ok(&dict.odict.entries),
			None => unreachable!(),
		}
	}

	/// # Panics
	///
	/// Will panic if `self.path` is not valid
	#[must_use]
	pub fn name(&self) -> String {
		let stem = self.path.file_stem().unwrap().to_str().unwrap().to_owned();

		if let Some(dict) = &self.dict {
			dict.odict.name.clone().unwrap_or(stem)
		} else {
			stem
		}
	}
}
