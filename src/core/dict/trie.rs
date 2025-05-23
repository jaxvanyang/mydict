use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Trie {
	map: BTreeMap<u8, Trie>,
	is_end: bool,
}

impl Trie {
	#[must_use]
	pub fn new() -> Self {
		Self {
			map: BTreeMap::new(),
			is_end: false,
		}
	}

	pub fn insert(&mut self, s: &str) {
		let mut current = self;
		for byte in s.as_bytes() {
			current = current.map.entry(*byte).or_default();
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

	#[must_use]
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

impl Default for Trie {
	fn default() -> Self {
		Self::new()
	}
}
