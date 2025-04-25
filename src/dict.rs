use std::collections::{BTreeMap, HashMap};

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

struct Dictionary {
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
			odict.name.as_ref().unwrap(),
			elapsed_secs(&t0)
		);
		Self { odict, trie }
	}
}

enum InnerDict {
	DictFile(odict::DictionaryFile),
	Dict(Dictionary),
}

pub struct LazyDict {
	inner: InnerDict,
}

impl From<odict::DictionaryFile> for LazyDict {
	fn from(file: odict::DictionaryFile) -> Self {
		Self {
			inner: InnerDict::DictFile(file),
		}
	}
}

impl From<odict::Dictionary> for LazyDict {
	fn from(dict: odict::Dictionary) -> Self {
		Self {
			inner: InnerDict::Dict(dict.into()),
		}
	}
}

impl LazyDict {
	fn lazy_load(&mut self) {
		if let InnerDict::DictFile(file) = &self.inner {
			let t0 = now();
			let dict = file.to_dictionary().expect("ODict file valid");
			let name = dict
				.name
				.as_ref()
				.expect("dictionary should have a name")
				.clone();
			self.inner = InnerDict::Dict(dict.into());
			tracing::info!("lazy load {} in {:.3}s", name, elapsed_secs(&t0));
		}
	}

	pub fn search(&mut self, s: &str) -> Vec<String> {
		self.lazy_load();
		match &self.inner {
			InnerDict::Dict(dict) => dict.trie.search(s),
			InnerDict::DictFile(_) => unreachable!(),
		}
	}

	pub fn entries(&mut self) -> &HashMap<String, odict::Entry> {
		self.lazy_load();
		match &self.inner {
			InnerDict::Dict(dict) => &dict.odict.entries,
			InnerDict::DictFile(_) => unreachable!(),
		}
	}

	/// # Panics
	///
	/// Will panic if dictionary file not valid
	#[must_use]
	pub fn name(&self) -> Option<String> {
		match &self.inner {
			InnerDict::Dict(dict) => dict.odict.name.clone(),
			InnerDict::DictFile(file) => {
				let dict = file.to_archive().expect("ODict file to archive error");
				match &dict.name {
					odict::ArchivedOption::None => None,
					odict::ArchivedOption::Some(name) => Some(name.to_string()),
				}
			}
		}
	}
}
