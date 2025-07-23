use crate::{app::AppModel, elapsed_secs, now};
use odict::semver::SemanticVersion;
use std::path::{Path, PathBuf};
use tracing::{info, info_span};
use url::Url;

pub const MINIMAL_ODICT_VERSION: SemanticVersion = SemanticVersion {
	major: 2,
	minor: 8,
	patch: 0,
	prerelease: None,
};

#[must_use]
pub fn is_odict_file_compatible(file: &odict::DictionaryFile) -> bool {
	file.version == MINIMAL_ODICT_VERSION || file.version > MINIMAL_ODICT_VERSION
}

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
	if !is_odict_file_compatible(&odict_file) {
		anyhow::bail!(
			"require ODict version ~{MINIMAL_ODICT_VERSION}, but found {}",
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
	let compress_options = odict::CompressOptions::default().quality(8).window_size(22);
	let writer_options =
		odict::io::DictionaryWriterOptions::default().compression(compress_options);
	odict::DictionaryWriter::new()
		.write_to_path_with_opts(dictionary, path, writer_options)
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

	info!("reading ODict from {}...", path.display());
	let mut odict = read_odict_from_path(&path)?;

	let local_data_dir = AppModel::local_data_dir();
	if !local_data_dir.exists() {
		std::fs::create_dir_all(&local_data_dir)?;
	}

	let target_path = if let Some(name) = &odict.name {
		local_data_dir.join(format!("{}.odict", name.replace(['/', '\\'], "|")))
	} else {
		let name = path
			.file_stem()
			.ok_or(anyhow::anyhow!("path not valid: {}", path.display()))?
			.to_string_lossy()
			.to_string();
		odict.name = Some(name.clone());
		local_data_dir.join(format!("{name}.odict"))
	};

	if target_path.exists() {
		anyhow::bail!("target path exists: {}", target_path.display());
	}

	info!("writing ODict to {target_path:?}...");
	write_odict_to_path(&odict, &target_path)?;

	info!("import used {:.3}s", elapsed_secs(&t0));

	Ok((odict, target_path))
}
