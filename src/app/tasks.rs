use super::Message;
use crate::import_odict;

use cosmic::Task;
use url::Url;

pub fn create_import_task(url: Url) -> Task<cosmic::Action<Message>> {
	cosmic::task::future(async move {
		match import_odict(&url).await {
			Err(err) => Message::ImportError(err.to_string()),
			Ok((odict, path)) => Message::ODictCopied(odict, path),
		}
	})
}
