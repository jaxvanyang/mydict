use tracing::info;

use super::AppModel;
use crate::{LazyDict, elapsed_secs, now};

/// Initialize imported & system dictionaries.
///
/// # Errors
///
/// Return `Err` if file system error.
pub fn init_app_dicts() -> anyhow::Result<Vec<LazyDict>> {
	let dicts: Vec<LazyDict> = AppModel::dict_paths()?
		.into_iter()
		.map(|p| {
			let t0 = now();
			let dict = LazyDict::new(p);
			info!("loaded ODict {:?} in {:.3}s", dict.path, elapsed_secs(&t0));
			dict
		})
		.collect();

	Ok(dicts)
}
