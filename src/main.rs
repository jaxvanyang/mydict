// SPDX-License-Identifier: MIT

use std::env;

use mydict::{app, i18n};

fn main() -> cosmic::iced::Result {
	tracing_subscriber::fmt::init();

	// Get the system's preferred languages.
	let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

	// Enable localizations to be applied.
	i18n::init(&requested_languages);

	// Settings for configuring the application window and iced runtime.
	let settings = cosmic::app::Settings::default().size_limits(
		cosmic::iced::Limits::NONE
			.min_width(360.0)
			.min_height(180.0),
	);

	cosmic::app::run::<app::AppModel>(
		settings,
		env::args()
			.collect::<Vec<String>>()
			.get(1)
			.cloned()
			.unwrap_or_default(),
	)
}
