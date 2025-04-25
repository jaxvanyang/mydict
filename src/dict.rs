use std::collections::HashMap;

use crate::{elapsed_secs, now};

enum InnerDict {
	DictFile(odict::DictionaryFile),
	Dict(odict::Dictionary),
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
			inner: InnerDict::Dict(dict),
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
			self.inner = InnerDict::Dict(dict);
			tracing::info!("lazy load {} in {:.3}s", name, elapsed_secs(&t0));
		}
	}
	pub fn lexicon(&mut self) -> Vec<&str> {
		self.lazy_load();
		match &self.inner {
			InnerDict::Dict(dict) => dict.lexicon(),
			InnerDict::DictFile(_) => unreachable!(),
		}
	}

	pub fn entries(&mut self) -> &HashMap<String, odict::Entry> {
		self.lazy_load();
		match &self.inner {
			InnerDict::Dict(dict) => &dict.entries,
			InnerDict::DictFile(_) => unreachable!(),
		}
	}

	/// # Panics
	///
	/// Will panic if dictionary file not valid
	#[must_use]
	pub fn name(&self) -> Option<String> {
		match &self.inner {
			InnerDict::Dict(dict) => dict.name.clone(),
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
