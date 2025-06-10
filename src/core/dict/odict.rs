use std::path::{Path, PathBuf};

use tracing::{info, info_span};
use url::Url;

use crate::{app::AppModel, now};

/// # Errors
///
/// Will return `Err` if `path` or the format not valid
pub fn read_odict_file_from_path(path: &Path) -> anyhow::Result<odict::DictionaryFile> {
	odict::DictionaryReader::new()
		.read_from_path(
			path.to_str()
				.ok_or(anyhow::anyhow!("path is not valid unicode: {path:?}"))?,
		)
		.map_err(|err| anyhow::anyhow!(err))
}

/// # Errors
///
/// Will return `Err` if file format not valid or version not compatible
pub fn read_odict_from_path(path: &Path) -> anyhow::Result<odict::Dictionary> {
	let odict_file = read_odict_file_from_path(path)?;
	if (odict_file.version.major, odict_file.version.minor) != (2, 5) {
		anyhow::bail!(
			"require ODict version ~2.5.0, but found {}",
			odict_file.version
		)
	}

	odict_file
		.to_dictionary()
		.map_err(|err| anyhow::anyhow!(err))
}

/// # Errors
///
/// Will return `Err` if write failed
pub fn write_odict_to_path(dictionary: &odict::Dictionary, path: &Path) -> anyhow::Result<()> {
	odict::DictionaryWriter::new()
		.write_to_path(dictionary, path)
		.map_err(|err| anyhow::anyhow!(err))
}

/// # Return
///
/// The `ODict` and target path
///
/// # Errors
///
/// Error message should explain it
pub async fn import_odict(url: &Url) -> anyhow::Result<(odict::Dictionary, PathBuf)> {
	let _span = info_span!("import").entered();
	let t0 = now();

	let path = match url.scheme() {
		"file" => url
			.to_file_path()
			.map_err(|()| anyhow::anyhow!("url not valid: {url}"))?,
		other => {
			anyhow::bail!("{url} has unknown schema: {other}");
		}
	};
	info!("reading ODict from {path:?}...");
	let mut odict = read_odict_from_path(&path)?;
	let target_path = if let Some(name) = &odict.name {
		AppModel::data_dir().join(format!("{}.odict", name.replace(['/', '\\'], "|")))
	} else {
		let name = path
			.file_name()
			.ok_or(anyhow::anyhow!("path not valid: {}", path.display()))?
			.to_string_lossy()
			.to_string();
		odict.name = Some(name.clone());
		AppModel::data_dir().join(name)
	};

	if target_path.exists() {
		anyhow::bail!("target path exists: {}", target_path.display());
	}

	info!("writing ODict to {target_path:?}...");
	write_odict_to_path(&odict, &target_path)?;

	info!("import used {:.3}s", t0.elapsed().as_secs_f32());

	Ok((odict, target_path))
}
