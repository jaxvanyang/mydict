// SPDX-License-Identifier: MIT

pub mod tasks;
pub mod utils;

pub use tasks::*;
pub use utils::*;

use crate::config::Config;
use crate::font::font_builder;
use crate::{Dictionary, fl};
use crate::{LazyDict, elapsed_secs, now};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::Length::{self};
use cosmic::iced::{Alignment, Subscription, window};
use cosmic::iced_widget::{column, horizontal_rule};
use cosmic::prelude::*;
use cosmic::widget::{self, button, menu, nav_bar, scrollable, text};
use cosmic::{
	cosmic_theme::{self},
	theme,
};
use directories::ProjectDirs;
use futures_util::SinkExt;
use odict::{DefinitionType, Entry};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, debug_span, error, info, info_span};
use url::Url;

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
	selected_dict_url: Option<Url>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
	OpenRepositoryUrl,
	SubscriptionChannel,
	ToggleContextPage(ContextPage),
	UpdateConfig(Config),
	LaunchUrl(String),
	ChangeSearch(String),
	Search,
	SearchResult(Vec<String>),
	// messages for import
	OpenImportDialog,
	DictFileSelected(Url),
	ImportCancelled,
	ImportError(String),
	ODictCopied(odict::Dictionary, PathBuf),
	// messages for load
	SelectDict(usize),
	LoadDict((usize, Dictionary)),
	LoadError(String),
	DictNotCompatible((usize, (u64, u64, u64))),
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
		let _span = info_span!("init").entered();
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
						error!(%why, "error loading app config");
					}

					config
				}
			},
			config_manager,
			dicts: init_app_dicts().unwrap(),
			dict_entry: None,
			selected_dict_url: None,
		};

		if !flags.is_empty() {
			app.config
				.set_search_term(&app.config_manager, flags)
				.unwrap();
		}

		info!("initialized in {:.3}s", elapsed_secs(&t0));

		let command = app.load_selected_dict();

		(app, command)
	}

	/// Elements to pack at the start of the header bar.
	fn header_start(&self) -> Vec<Element<Self::Message>> {
		let file_menu = menu::Tree::with_children(
			menu::root(fl!("file")).apply(Element::from),
			menu::items(
				&self.key_binds,
				vec![menu::Item::Button(fl!("import"), None, MenuAction::Import)],
			),
		);
		let view_menu = menu::Tree::with_children(
			menu::root(fl!("view")).apply(Element::from),
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
			.on_input(Message::ChangeSearch)
			.on_clear(Message::ChangeSearch(String::new()))
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

	fn view_window(&self, _window_id: window::Id) -> Element<Self::Message> {
		widget::text("Unknown window ID").into()
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
		let content = column![dicts, term_page].spacing(5);
		let mut content = widget::popover(content).modal(true);

		if let Some(url) = &self.selected_dict_url {
			let dialog = widget::dialog().body(format!("Importing {url}, please wait."));
			content = content.popup(dialog);
		}

		content.into()
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
						error!(?why, "app config error");
					}

					Message::UpdateConfig(update.config)
				}),
		])
	}

	/// Handles messages emitted by the application and its widgets.
	///
	/// Tasks may be returned for asynchronous execution of code in the background
	/// on the application's async runtime.
	#[allow(clippy::too_many_lines)]
	fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
		match message {
			Message::LoadError(msg) => error!("load dictionary error: {msg}"),
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
			Message::UpdateConfig(config) => self.config = config,
			Message::LaunchUrl(url) => {
				if let Err(err) = open::that_detached(&url) {
					error!("failed to open {url:?}: {err}");
				}
			}
			Message::LoadDict((i, dict)) => {
				self.dicts[i].load(dict);
				self.dicts[i].is_loading = false;
				return Task::done(Message::Search).map(cosmic::Action::from);
			}
			Message::ChangeSearch(s) => {
				self.config
					.set_search_term(&self.config_manager, s)
					.unwrap();

				if let Some(dict) = self.selected_dict() {
					return if dict.is_loaded() {
						self.search()
					} else {
						self.load_selected_dict()
					};
				}
			}
			Message::Search => return self.search(),
			Message::SearchResult(terms) => {
				if terms.is_empty() {
					return Task::none();
				}
				let mut iter = terms.into_iter();
				self.nav.insert().text(iter.next().unwrap()).activate();
				for term in iter {
					self.nav.insert().text(term);
				}
				return self.update_title();
			}
			Message::SelectDict(i) => {
				if i == self.config.selected_index {
					return Task::none();
				}
				self.config
					.set_selected_index(&self.config_manager, i)
					.unwrap();

				return if self.selected_dict().unwrap().is_loaded() {
					self.search()
				} else {
					self.load_selected_dict()
				};
			}
			Message::OpenImportDialog => {
				return cosmic::task::future(async move {
					info!("opening new dialog");

					#[cfg(feature = "rfd")]
					let filter = FileFilter::new("ODict files").extension("odict");

					#[cfg(feature = "xdg-portal")]
					let filter = FileFilter::new("ODict files").glob("*.odict");

					let dialog = file_chooser::open::Dialog::new()
						.title("Choose a file")
						.filter(filter);

					match dialog.open_file().await {
						Ok(response) => Message::DictFileSelected(response.url().to_owned()),
						Err(file_chooser::Error::Cancelled) => Message::ImportCancelled,
						Err(err) => Message::ImportError(err.to_string()),
					}
				});
			}
			Message::DictFileSelected(url) => {
				info!("selected file: {url}");
				self.selected_dict_url = Some(url.clone());
				return create_import_task(url);
			}
			Message::ImportCancelled => info!("import cancelled"),
			Message::ImportError(err) => {
				error!("import failed: {err}");
				self.selected_dict_url = None;
			}
			Message::ODictCopied(odict, path) => {
				let mut dict = LazyDict::new(path);
				dict.load(odict.into());
				self.dicts.push(dict);
				self.selected_dict_url = None;
			}
			Message::DictNotCompatible((index, (major, minor, patch))) => {
				error!("dict {index} file version not compatible: {major}.{minor}.{patch}");
				self.dicts.remove(index);
			}
		}
		Task::none()
	}

	/// Called when a nav item is selected.
	fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
		// Activate the page in the model.
		self.nav.activate(id);

		if let Some(dict) = self.dicts.get_mut(self.config.selected_index) {
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

	/// Load selected dictionary.
	///
	/// # Panics
	///
	/// Will panic if load dictionary failed.
	pub fn load_selected_dict(&mut self) -> Task<cosmic::Action<Message>> {
		self.correct_selected_index();

		let index = self.config.selected_index;
		let Some(selected_dict) = self.dicts.get_mut(index) else {
			info!(
				"selected index ({}) out of range, dicts size: {}",
				index,
				self.dicts.len()
			);
			return Task::none();
		};

		if selected_dict.is_loading {
			info!("selected dictionary is loading, ignore load request");
			return Task::none();
		}
		selected_dict.is_loading = true;

		create_load_task(index, selected_dict.path.clone())
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

	#[must_use]
	pub fn selected_dict(&self) -> Option<&LazyDict> {
		self.dicts.get(self.config.selected_index)
	}

	/// # Panics
	///
	/// Will panic if config update failed.
	pub fn correct_selected_index(&mut self) {
		if self.dicts.len() <= self.config.selected_index {
			info!("reset selected dict index to 0, because there are not enough dicts");
			self.config
				.set_selected_index(&self.config_manager, 0)
				.unwrap();
		}
	}

	/// Search term in selected dictionary
	fn search(&mut self) -> Task<cosmic::Action<Message>> {
		let _span = debug_span!("search").entered();
		let t0 = now();

		self.nav.clear();

		let s = self.config.search_term.trim();
		if s.is_empty() {
			self.dict_entry = None;
			return Task::none();
		}

		if let Some(dict) = self.dicts.get_mut(self.config.selected_index) {
			let terms = dict.search(s).unwrap().into_iter().take(1000).collect();
			self.dict_entry = dict.get(s).unwrap().cloned();
			debug!(
				"search \"{}\" in dict {} finished in {:.3}s",
				s,
				self.config.selected_index,
				elapsed_secs(&t0)
			);

			return Task::done(Message::SearchResult(terms)).map(cosmic::Action::from);
		}

		error!("dict_index not valid: {}", self.config.selected_index);
		self.config.selected_index = 0;
		info!("reset dict_index to 0");

		self.update_title()
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
				for sense in &ety.senses {
					page = page.push(
						text::body(sense.pos.to_string())
							.font(font_builder().italic().bold().build()),
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
				// FIXME: change selected dictionary doesn't show loading
				text::title1(match self.selected_dict() {
					None => "no dictionary found, please import one",
					Some(dict) => {
						if dict.is_loading {
							"Loading..."
						} else if self.dict_entry.is_none() {
							"Type to search"
						} else {
							"Search not found"
						}
					}
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
			MenuAction::Import => Message::OpenImportDialog,
		}
	}
}
