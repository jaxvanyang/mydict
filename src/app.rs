// SPDX-License-Identifier: MIT

use crate::config::Config;
use crate::fl;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::Length::{self};
use cosmic::iced::{Alignment, Subscription};
use cosmic::iced_widget::{column, horizontal_rule};
use cosmic::prelude::*;
use cosmic::widget::{self, button, menu, nav_bar, scrollable, search_input, text};
use cosmic::{cosmic_theme, theme};
use directories::ProjectDirs;
use futures_util::SinkExt;
use odict::{DefinitionType, Dictionary, DictionaryReader, Entry};
use std::collections::HashMap;
use std::fs;

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

	dicts: Vec<Dictionary>,
	selected_dict: usize,
	search_term: String,
	// term_list: Vec<String>,
	dict_entry: Option<Entry>,
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
	SelectDict(usize),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
	/// The async executor that will be used to run your application's commands.
	type Executor = cosmic::executor::Default;

	/// Data that your application receives to its init method.
	type Flags = ();

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

	/// Initializes the application with any given flags and startup commands.
	fn init(
		core: cosmic::Core,
		_flags: Self::Flags,
	) -> (Self, Task<cosmic::Action<Self::Message>>) {
		// Construct the app model with the runtime's core.
		let mut app = AppModel {
			core,
			context_page: ContextPage::default(),
			nav: nav_bar::Model::default(),
			key_binds: HashMap::new(),
			// Optional configuration file for an application.
			config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
				.map(|context| match Config::get_entry(&context) {
					Ok(config) => config,
					Err((_errors, config)) => {
						// for why in errors {
						//     tracing::error!(%why, "error loading app config");
						// }

						config
					}
				})
				.unwrap_or_default(),
			dicts: Self::load_dicts(),
			selected_dict: 0,
			search_term: String::new(),
			dict_entry: None,
		};

		// Create a startup command that sets the window title.
		let command = app.update_title();

		(app, command)
	}

	/// Elements to pack at the start of the header bar.
	fn header_start(&self) -> Vec<Element<Self::Message>> {
		let menu_bar = menu::bar(vec![menu::Tree::with_children(
			menu::root(fl!("view")),
			menu::items(
				&self.key_binds,
				vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
			),
		)]);

		vec![menu_bar.into()]
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

	/// Describes the interface based on the current state of the application model.
	///
	/// Application events will be processed through the view. Any messages emitted by
	/// events received by widgets will be passed to the update method.
	fn view(&self) -> Element<Self::Message> {
		// TODO: move to title bar
		let search = search_input("", &self.search_term)
			.on_input(Message::Search)
			.always_active();

		let dicts = scrollable::horizontal(widget::Row::from_iter(
			self.dicts.iter().enumerate().map(|(i, d)| {
				let name = d.name.as_ref().expect("dictionary should have name");
				button::text(name).on_press(Message::SelectDict(i)).into()
			}),
		));

		let term_page = scrollable(self.build_term_page());

		column![
			search,
			horizontal_rule(2),
			dicts,
			horizontal_rule(2),
			term_page
		]
		.spacing(5)
		.into()
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
				cosmic::iced::stream::channel(4, move |mut channel| async move {
					_ = channel.send(Message::SubscriptionChannel).await;

					futures_util::future::pending().await
				}),
			),
			// Watch for application configuration changes.
			self.core()
				.watch_config::<Config>(Self::APP_ID)
				.map(|update| {
					// for why in update.errors {
					//     tracing::error!(?why, "app config error");
					// }

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
					eprintln!("failed to open {url:?}: {err}");
				}
			},

			Message::Search(s) => {
				self.search_term = s;
				self.refresh();
			}

			Message::SelectDict(i) => {
				self.selected_dict = i;
				self.refresh();
			}
		}
		Task::none()
	}

	/// Called when a nav item is selected.
	fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
		// Activate the page in the model.
		self.nav.activate(id);

		if let Some(dict) = self.dicts.get(self.selected_dict) {
			if let Some(s) = self.nav.text(id) {
				self.dict_entry = Some(dict.entries[s].clone());
			}
		}

		self.update_title()
	}
}

impl AppModel {
	/// The about page for this app.
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

		if let Some(page) = self.nav.text(self.nav.active()) {
			window_title.push_str(" — ");
			window_title.push_str(page);
		}

		if let Some(id) = self.core.main_window_id() {
			self.set_window_title(window_title, id)
		} else {
			Task::none()
		}
	}

	/// Load ODict dictionaries.
	pub fn load_dicts() -> Vec<Dictionary> {
		// TODO: read name from metadata
		let proj_dirs = ProjectDirs::from("", "", "mydict").unwrap();
		let data_dir = proj_dirs.data_dir();
		if !data_dir.exists() {
			fs::create_dir(data_dir).expect("created directory");
		}
		let reader = DictionaryReader::new();
		// TODO: read by alphabetic order
		let dicts = data_dir
			.read_dir()
			.expect("read data directory")
			.filter_map(|e| {
				let path = e.expect("read data directory entry").path();
				let path = path.to_str().expect("path should be unicode valid");
				if path.ends_with(".odict") {
					eprintln!("Loading {path}...");
					let file = reader.read_from_path(path).expect("ODict file exists");
					if !(file.version.major == 2 && file.version.minor >= 5) {
						eprintln!(
							"File version not compatible: {}, expect 2.5.0",
							file.version
						);
						return None;
					}
					let dict = file.to_dictionary().expect("ODict file valid");
					eprintln!("Loaded {path}...");
					Some(dict)
				} else {
					None
				}
			})
			.collect::<Vec<Dictionary>>();

		dicts
	}

	// TODO: log execution time of this function
	/// Update state according to selected dictionary and term
	pub fn refresh(&mut self) {
		self.nav.clear();

		let s = self.search_term.trim();
		if s.is_empty() {
			self.dict_entry = None;
			return;
		}

		if let Some(dict) = self.dicts.get(self.selected_dict) {
			for (i, term) in dict
				.lexicon()
				.into_iter()
				.filter(|t| t.starts_with(s))
				.take(1000)
				.enumerate()
			{
				let item = self.nav.insert().text(term.to_string());
				if i == 0 {
					item.activate();
				}
			}

			self.dict_entry = dict.entries.get(s).cloned();
		}
	}

	/// Build term page from ODict entry
	fn build_term_page(&self) -> widget::Column<Message> {
		let mut page = widget::column();

		if let Some(entry) = &self.dict_entry {
			page = page.push(text::title2(&entry.term));

			for (i, ety) in entry.etymologies.iter().enumerate() {
				page = page.push(text::title3(format!("Etymology #{}", i + 1)));
				if let Some(desc) = &ety.description {
					// TODO: bold
					page = page.push(text::body(desc));
				}
				for (pos, sense) in &ety.senses {
					// TODO: ilatic
					page = page.push(text::body(pos.description()));
					for (j, def) in sense.definitions.iter().enumerate() {
						match def {
							DefinitionType::Definition(def) => {
								page = page.push(text::body(format!("{}. {}", j + 1, def.value)));
								for example in &def.examples {
									page = page.push(text::body(format!("\t- {}", example.value)));
								}

								if !def.notes.is_empty() {
									page = page.push(text::body("\tNotes"));
								}

								for (k, note) in def.notes.iter().enumerate() {
									page = page.push(text::body(format!(
										"\t{}. {}",
										k + 1,
										note.value
									)));
								}
							}
							DefinitionType::Group(_) => {
								todo!();
							}
						}
					}
				}
			}
		} else {
			page = page.push(
				text::title1(if self.search_term.is_empty() {
					"Type to search"
				} else {
					"Search not found"
				})
				.width(Length::Fill)
				.align_x(Alignment::Center),
			);
		}

		page.width(Length::Fill)
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
	About,
}

impl menu::action::MenuAction for MenuAction {
	type Message = Message;

	fn message(&self) -> Self::Message {
		match self {
			MenuAction::About => Message::ToggleContextPage(ContextPage::About),
		}
	}
}
