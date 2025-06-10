use tracing::info;

use super::AppModel;
use crate::{LazyDict, elapsed_secs, now};

/// Initialize imported & system dictionaries.
///
/// # Errors
///
/// Return `Err` if file system error.
pub fn init_app_dicts() -> anyhow::Result<Vec<LazyDict>> {
	let data_dir = AppModel::data_dir();
	if !data_dir.exists() {
		std::fs::create_dir(&data_dir)?;
	}

	// TODO: load by alphabetic order
	let dicts: Vec<LazyDict> = data_dir
		.read_dir()?
		.filter_map(|e| {
			let path = e.ok()?.path();
			if path.extension().is_some_and(|s| s == "odict") {
				let t0 = now();
				let dict = LazyDict::new(path);
				info!("loaded ODict {:?} in {:.3}s", dict.path, elapsed_secs(&t0));
				Some(dict)
			} else {
				None
			}
		})
		.collect();

	Ok(dicts)
}
