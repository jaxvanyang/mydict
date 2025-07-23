use std::path::PathBuf;

use super::Message;
use crate::{Dictionary, import_odict, is_odict_file_compatible, read_odict_file_from_path};
use cosmic::task;
use url::Url;

type Task = cosmic::Task<cosmic::Action<Message>>;

pub fn create_import_task(url: Url) -> Task {
	task::future(async move {
		match import_odict(&url).await {
			Err(err) => Message::ImportError(err.to_string()),
			Ok((odict, path)) => Message::ODictCopied(odict, path),
		}
	})
}

pub fn create_load_task(index: usize, path: PathBuf) -> Task {
	task::future(async move {
		let odict_file = match read_odict_file_from_path(&path) {
			Ok(file) => file,
			Err(err) => return Message::LoadError(err.to_string()),
		};
		if !is_odict_file_compatible(&odict_file) {
			return Message::DictNotCompatible((index, odict_file.version));
		}
		let dict = Dictionary::load_from_path(&path);
		match dict {
			Ok(dict) => Message::LoadDict((index, dict)),
			Err(err) => Message::LoadError(err.to_string()),
		}
	})
}
