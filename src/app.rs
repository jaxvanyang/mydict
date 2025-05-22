// SPDX-License-Identifier: MIT

use crate::config::Config;
use crate::fl;
use crate::font::font_builder;
use crate::{LazyDict, elapsed_secs, now};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::Length::{self};
use cosmic::iced::{Alignment, Subscription, window};
use cosmic::iced_widget::{column, horizontal_rule};
use cosmic::prelude::*;
use cosmic::widget::{self, button, menu, nav_bar, scrollable, text};
use cosmic::{
	cosmic_theme::{self},
	theme,
};
use cosmic_files::dialog::{
	Dialog, DialogFilter, DialogFilterPattern, DialogKind, DialogMessage, DialogResult,
};
use directories::ProjectDirs;
use futures_util::SinkExt;
use odict::{DefinitionType, Entry};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::info;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
	/// Application state which is managed by the COSMIC runtime.
	core: cosmic::Core,
	/// Display a context drawer with the designated page if defined.
	context_page: ContextPage,
	/// Contains items assigned to the nav bar panel.
	nav: nav_bar::Model,
	/// Key bindings for the application's menu bar.
	key_binds: HashMap<menu::KeyBind, MenuAction>,
	// Configuration data that persists between application runs.
	config: Config,
	config_manager: cosmic_config::Config,
	dicts: Vec<LazyDict>,
	dict_entry: Option<Entry>,
	dialog: Option<Dialog<Message>>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
	OpenRepositoryUrl,
	SubscriptionChannel,
	ToggleContextPage(ContextPage),
	UpdateConfig(Config),
	LaunchUrl(String),
	Search(String),
	SearchResult(Vec<String>),
	SearchClear,
	SelectDict(usize),
	UpdateDialog(DialogMessage),
	ImportDictDialog,
	ImportDictResult(DialogResult),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
	/// The async executor that will be used to run your application's commands.
	type Executor = cosmic::executor::Default;

	/// Command line search term
	type Flags = String;

	/// Messages which the application and its widgets will emit.
	type Message = Message;

	/// Unique identifier in RDNN (reverse domain name notation) format.
	const APP_ID: &'static str = "com.github.jaxvanyang.mydict";

	fn core(&self) -> &cosmic::Core {
		&self.core
	}

	fn core_mut(&mut self) -> &mut cosmic::Core {
		&mut self.core
	}

	fn init(core: cosmic::Core, flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
		let _span = tracing::info_span!("init").entered();
		let t0 = now();
		let config_manager = cosmic_config::Config::new(Self::APP_ID, Config::VERSION).unwrap();

		let mut app = AppModel {
			core,
			context_page: ContextPage::default(),
			nav: nav_bar::Model::default(),
			key_binds: HashMap::new(),
			config: match Config::get_entry(&config_manager) {
				Ok(config) => config,
				Err((errors, config)) => {
					for why in errors {
						tracing::error!(%why, "error loading app config");
					}

					config
				}
			},
			config_manager,
			dicts: Self::load_dicts(),
			dict_entry: None,
			dialog: None,
		};

		if !flags.is_empty() {
			app.config
				.set_search_term(&app.config_manager, flags)
				.unwrap();
		}

		tracing::info!("initialized in {:.3}s", t0.elapsed().as_secs_f32());

		let command = Task::batch(vec![app.search(), app.update_title()]);

		(app, command)
	}

	/// Elements to pack at the start of the header bar.
	fn header_start(&self) -> Vec<Element<Self::Message>> {
		let file_menu = menu::Tree::with_children(
			menu::root(fl!("file")),
			menu::items(
				&self.key_binds,
				vec![menu::Item::Button(fl!("import"), None, MenuAction::Import)],
			),
		);
		let view_menu = menu::Tree::with_children(
			menu::root(fl!("view")),
			menu::items(
				&self.key_binds,
				vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
			),
		);
		let menu_bar = menu::bar(vec![file_menu, view_menu]);

		vec![menu_bar.into()]
	}

	fn header_center(&self) -> Vec<Element<Self::Message>> {
		let search_input = widget::search_input("", &self.config.search_term)
			.on_input(Message::Search)
			.on_clear(Message::SearchClear)
			.always_active();

		vec![search_input.into()]
	}

	/// Enables the COSMIC application to create a nav bar with this model.
	fn nav_model(&self) -> Option<&nav_bar::Model> {
		Some(&self.nav)
	}

	/// Display a context drawer if the context page is requested.
	fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
		if !self.core.window.show_context {
			return None;
		}

		Some(match self.context_page {
			ContextPage::About => context_drawer::context_drawer(
				self.about(),
				Message::ToggleContextPage(ContextPage::About),
			)
			.title(fl!("about")),
		})
	}

	fn view_window(&self, window_id: window::Id) -> Element<Self::Message> {
		match &self.dialog {
			Some(dialog) => dialog.view(window_id),
			None => widget::text("Unknown window ID").into(),
		}
	}

	/// Describes the interface based on the current state of the application model.
	///
	/// Application events will be processed through the view. Any messages emitted by
	/// events received by widgets will be passed to the update method.
	fn view(&self) -> Element<Self::Message> {
		#[allow(clippy::from_iter_instead_of_collect)]
		let dicts = scrollable::horizontal(widget::Row::from_iter(self.dicts.iter().enumerate().map(
			|(i, d)| {
				let name = d.name();
				button::text(name).on_press(Message::SelectDict(i)).into()
			},
		)));

		// TODO: use custom widget
		let term_page = scrollable(self.build_term_page().padding(10));

		column![dicts, term_page].spacing(5).into()
	}

	/// Register subscriptions for this application.
	///
	/// Subscriptions are long-running async tasks running in the background which
	/// emit messages to the application through a channel. They are started at the
	/// beginning of the application, and persist through its lifetime.
	fn subscription(&self) -> Subscription<Self::Message> {
		struct MySubscription;

		Subscription::batch(vec![
			// Create a subscription which emits updates through a channel.
			Subscription::run_with_id(
				std::any::TypeId::of::<MySubscription>(),
				#[allow(clippy::semicolon_if_nothing_returned)]
				cosmic::iced::stream::channel(4, move |mut channel| async move {
					_ = channel.send(Message::SubscriptionChannel).await;

					futures_util::future::pending().await
				}),
			),
			// Watch for application configuration changes.
			self.core()
				.watch_config::<Config>(Self::APP_ID)
				.map(|update| {
					for why in update.errors {
						tracing::error!(?why, "app config error");
					}

					Message::UpdateConfig(update.config)
				}),
		])
	}

	/// Handles messages emitted by the application and its widgets.
	///
	/// Tasks may be returned for asynchronous execution of code in the background
	/// on the application's async runtime.
	fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
		match message {
			Message::OpenRepositoryUrl => {
				_ = open::that_detached(REPOSITORY);
			}

			Message::SubscriptionChannel => {
				// For example purposes only.
			}

			Message::ToggleContextPage(context_page) => {
				if self.context_page == context_page {
					// Close the context drawer if the toggled context page is the same.
					self.core.window.show_context = !self.core.window.show_context;
				} else {
					// Open the context drawer to display the requested context page.
					self.context_page = context_page;
					self.core.window.show_context = true;
				}
			}

			Message::UpdateConfig(config) => {
				self.config = config;
			}

			Message::LaunchUrl(url) => match open::that_detached(&url) {
				Ok(()) => {}
				Err(err) => {
					tracing::error!("failed to open {url:?}: {err}");
				}
			},

			Message::Search(s) => {
				self.config
					.set_search_term(&self.config_manager, s)
					.unwrap();
				return Task::batch(vec![self.search(), self.update_title()]);
			}

			Message::SearchResult(terms) => {
				if terms.is_empty() {
					return Task::none();
				}
				let mut iter = terms.into_iter();
				self.nav.insert().text(iter.next().unwrap()).activate();
				for term in iter {
					self.nav.insert().text(term);
				}
			}

			Message::SearchClear => {
				self.config
					.set_search_term(&self.config_manager, String::new())
					.unwrap();
				return Task::batch(vec![self.search(), self.update_title()]);
			}

			Message::SelectDict(i) => {
				if i == self.config.selected_dict {
					return Task::none();
				}
				self.config
					.set_selected_dict(&self.config_manager, i)
					.unwrap();
				return self.search();
			}

			Message::ImportDictDialog => {
				if self.dialog.is_none() {
					let (mut dialog, command) = Dialog::new(
						DialogKind::OpenFile,
						None,
						Message::UpdateDialog,
						Message::ImportDictResult,
					);
					let filter = DialogFilter {
						label: "ODict".to_string(),
						patterns: vec![DialogFilterPattern::Glob("*.odict".to_string())],
					};
					let set_filter = dialog.set_filters([filter], Some(0));

					self.dialog = Some(dialog);
					return Task::batch([set_filter, command]);
				}
			}

			Message::UpdateDialog(msg) => {
				if let Some(dialog) = &mut self.dialog {
					return dialog.update(msg);
				}
			}

			Message::ImportDictResult(result) => {
				self.dialog = None;
				match result {
					DialogResult::Cancel => (),
					DialogResult::Open(path_bufs) => {
						// TODO: import ODict file
						dbg!(path_bufs);
					}
				}
			}
		}
		Task::none()
	}

	/// Called when a nav item is selected.
	fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
		// Activate the page in the model.
		self.nav.activate(id);

		if let Some(dict) = self.dicts.get_mut(self.config.selected_dict) {
			if let Some(s) = self.nav.text(id) {
				self.dict_entry = dict.get(s).unwrap().cloned();
			}
		}

		self.update_title()
	}
}

impl AppModel {
	const APP_NAME: &'static str = "mydict";

	/// The about page for this app.
	#[allow(clippy::unused_self)]
	pub fn about(&self) -> Element<Message> {
		let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

		let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

		let title = widget::text::title3(fl!("app-title"));

		let hash = env!("VERGEN_GIT_SHA");
		let short_hash: String = hash.chars().take(7).collect();
		let date = env!("VERGEN_GIT_COMMIT_DATE");

		let link = widget::button::link(REPOSITORY)
			.on_press(Message::OpenRepositoryUrl)
			.padding(0);

		widget::column()
			.push(icon)
			.push(title)
			.push(link)
			.push(
				widget::button::link(fl!(
					"git-description",
					hash = short_hash.as_str(),
					date = date
				))
				.on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
				.padding(0),
			)
			.align_x(Alignment::Center)
			.spacing(space_xxs)
			.into()
	}

	/// Updates the header and window titles.
	pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
		let mut window_title = fl!("app-title");

		if let Some(entry) = &self.dict_entry {
			window_title.push_str(" — ");
			window_title.push_str(&entry.term);
		}

		if let Some(id) = self.core.main_window_id() {
			self.set_window_title(window_title, id)
		} else {
			Task::none()
		}
	}

	/// # Panics
	///
	/// Will panic if no valid home directory path could be retrieved
	#[must_use]
	pub fn project_dirs() -> ProjectDirs {
		ProjectDirs::from("", "", Self::APP_NAME).unwrap()
	}

	#[must_use]
	pub fn data_dir() -> PathBuf {
		Self::project_dirs().data_dir().to_path_buf()
	}

	// TODO: move to lib
	/// Load dictionaries.
	///
	/// # Panics
	///
	/// Will panic if file system error
	#[must_use]
	pub fn load_dicts() -> Vec<LazyDict> {
		let data_dir = Self::data_dir();
		if !data_dir.exists() {
			fs::create_dir(&data_dir).expect("created directory");
		}
		// TODO: load by alphabetic order
		let dicts: Vec<LazyDict> = data_dir
			.read_dir()
			.expect("read data directory")
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

		dicts
	}

	/// Search term in selected dictionary
	fn search(&mut self) -> Task<cosmic::Action<Message>> {
		let _span = tracing::debug_span!("search").entered();
		let t0 = now();

		self.nav.clear();

		let s = self.config.search_term.trim();
		if s.is_empty() {
			self.dict_entry = None;
			return Task::none();
		}

		if let Some(dict) = self.dicts.get_mut(self.config.selected_dict) {
			if !dict.is_loaded() {
				dict.load().unwrap();
			}

			let terms = dict.search(s).unwrap().into_iter().take(1000).collect();
			self.dict_entry = dict.get(s).unwrap().cloned();
			tracing::debug!(
				"search \"{}\" in dict {} finished in {:.3}s",
				s,
				self.config.selected_dict,
				elapsed_secs(&t0)
			);

			return Task::done(Message::SearchResult(terms)).map(cosmic::Action::from);
		}

		tracing::error!("selected_dict not valid: {}", self.config.selected_dict);
		self.config.selected_dict = 0;
		tracing::info!("reset selected_dict to 0");

		Task::none()
	}

	/// Build term page from `ODict` entry
	fn build_term_page(&self) -> widget::Column<Message> {
		let mut page = widget::column().push(horizontal_rule(2));

		if let Some(entry) = &self.dict_entry {
			page = page.push(text::title1(&entry.term));

			for (i, ety) in entry.etymologies.iter().enumerate() {
				page = page.push(horizontal_rule(2));
				if entry.etymologies.len() > 1 {
					page = page.push(text::title2(format!("Etymology #{}", i + 1)));
				}
				if let Some(desc) = &ety.description {
					for p in desc.lines().map(text::body) {
						page = page.push(p);
					}
				}
				for (pos, sense) in &ety.senses {
					page = page.push(
						text::body(pos.description()).font(font_builder().italic().bold().build()),
					);
					for (j, def) in sense.definitions.iter().enumerate() {
						let alphabetic_numbering = |i| (b'a' + u8::try_from(i).unwrap()) as char;
						match def {
							DefinitionType::Definition(def) => {
								page =
									page.push(text::body(format!("{:>4}. {}", j + 1, def.value)));
								for example in &def.examples {
									page = page.push(
										text::body(format!("\t▸ {}", example.value))
											.font(font_builder().italic().build()),
									);
								}

								if !def.notes.is_empty() {
									page = page.push(text::heading("\tNotes"));
								}

								for (k, note) in def.notes.iter().enumerate() {
									page = page.push(text::body(format!(
										"\t{:>4}. {}",
										alphabetic_numbering(k),
										note.value
									)));
								}
							}
							DefinitionType::Group(group) => {
								page = page.push(text::body(format!(
									"{:>4}. {}",
									j + 1,
									group.description
								)));

								for (k, def) in group.definitions.iter().enumerate() {
									page = page.push(text::body(format!(
										"{:>8}. {}",
										alphabetic_numbering(k),
										def.value
									)));
									for example in &def.examples {
										page = page.push(
											text::body(format!("\t    ▸ {}", example.value))
												.font(font_builder().italic().build()),
										);
									}

									if !def.notes.is_empty() {
										page = page.push(text::heading("\t    Notes"));
									}

									for (l, note) in def.notes.iter().enumerate() {
										page = page.push(text::body(format!(
											"\t{:>8}. {}",
											l + 1,
											note.value
										)));
									}
								}
							}
						}
					}
				}
			}
		} else {
			page = page.push(
				text::title1(if self.config.search_term.is_empty() {
					"Type to search"
				} else {
					"Search not found"
				})
				.width(Length::Fill)
				.align_x(Alignment::Center),
			);
		}

		page.width(Length::Fill).spacing(5)
	}
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
	#[default]
	About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
	Import,
	About,
}

impl menu::action::MenuAction for MenuAction {
	type Message = Message;

	fn message(&self) -> Self::Message {
		match self {
			MenuAction::About => Message::ToggleContextPage(ContextPage::About),
			MenuAction::Import => Message::ImportDictDialog,
		}
	}
}
