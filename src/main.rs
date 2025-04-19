use std::{fmt::Write, fs};

use directories::ProjectDirs;
use iced::{
	Element,
	Length::Fill,
	Subscription, Task, Theme,
	widget::{
		Column, Row, Scrollable, button, column, container, horizontal_rule, markdown, row,
		scrollable,
		scrollable::{Direction, Scrollbar},
		text_input, vertical_rule,
	},
	window,
};
use odict::{DefinitionType, Dictionary, DictionaryReader, Entry};

pub const NAME: &str = "mydict";
pub const TITLE: &str = "My Dictionary";

#[derive(Debug)]
struct State {
	pub dicts: Vec<Dictionary>,
	pub selected_dict: usize,
	pub search_term: String,
	pub term_list: Vec<String>,
	pub term_description: Vec<markdown::Item>,
}

#[derive(Debug, Clone)]
pub enum Message {
	Created,
	SearchChanged(String),
	LinkClicked(markdown::Url),
	TermSelected(usize),
	DictSelected(usize),
}

impl Default for State {
	fn default() -> Self {
		let proj_dirs = ProjectDirs::from("", "", NAME).unwrap();
		let data_dir = proj_dirs.data_dir();
		if !data_dir.exists() {
			fs::create_dir(data_dir).expect("created directory");
		}
		let reader = DictionaryReader::new();
		// TODO: read by alphabetic sorting
		let dictionaries = data_dir
			.read_dir()
			.expect("read data directory")
			.filter_map(|e| {
				let path = e.expect("read data directory entry").path();
				let path = path.to_str().expect("path should be unicode valid");
				if path.ends_with(".odict") {
					eprintln!("Loading {path}...");
					let dict = reader
						.read_from_path(path)
						.expect("ODict file exists")
						.to_dictionary()
						.expect("ODict file valid");
					eprintln!("Loaded {path}...");
					Some(dict)
				} else {
					None
				}
			})
			.collect::<Vec<Dictionary>>();

		eprintln!("Initialization done.");

		Self {
			dicts: dictionaries,
			selected_dict: 0,
			search_term: String::default(),
			term_list: Vec::new(),
			term_description: Self::default_term_description(),
		}
	}
}

impl State {
	fn default_term_description() -> Vec<markdown::Item> {
		markdown::parse("# Type something to search...").collect::<Vec<markdown::Item>>()
	}

	// TODO: log time for this function
	/// Update state according to selected dictionary and term
	pub fn refresh(&mut self) {
		let s = self.search_term.trim();
		if let Some(dict) = self.dicts.get(self.selected_dict) {
			self.term_list = dict
				.lexicon()
				.into_iter()
				.filter_map(|t| {
					if t.starts_with(s) {
						Some(t.to_string())
					} else {
						None
					}
				})
				.take(1000)
				.collect();

			if dict.entries.contains_key(s) {
				let entry = &dict.entries[s];
				self.term_description = markdown::parse(&entry2md(entry)).collect();
			} else {
				self.term_description = Self::default_term_description();
			}
		}
	}

	pub fn view(&self) -> Element<Message> {
		let ui = row![
			scrollable(container(
				Column::from_iter(self.term_list.iter().enumerate().map(|(i, s)| {
					container(
						button(s.as_str())
							.style(|theme: &Theme, _| {
								let palette = theme.extended_palette();

								button::Style {
									background: None,
									text_color: palette.secondary.base.text,
									border: iced::Border::default(),
									shadow: iced::Shadow::default(),
								}
							})
							.width(Fill)
							.on_press(Message::TermSelected(i)),
					)
					.into()
				}))
				.width(200)
				.padding(5),
			)),
			vertical_rule(2),
			column![
				container(
					text_input("", &self.search_term)
						.id("search")
						.on_input(Message::SearchChanged),
				)
				.height(50)
				.padding(10),
				horizontal_rule(2),
				container(Scrollable::with_direction(
					Row::from_iter(self.dicts.iter().enumerate().map(|(i, d)| {
						let name = d.name.as_ref().expect("dictionary should have name");
						container(
							button(name.as_str())
								.style(|theme: &Theme, _| {
									let palette = theme.extended_palette();

									button::Style {
										background: None,
										text_color: palette.secondary.base.text,
										border: iced::Border::default(),
										shadow: iced::Shadow::default(),
									}
								})
								.on_press(Message::DictSelected(i)),
						)
						.into()
					})),
					Direction::Horizontal(Scrollbar::new())
				))
				.padding(5),
				horizontal_rule(2),
				scrollable(
					container(
						markdown::view(&self.term_description, Theme::default())
							.map(Message::LinkClicked),
					)
					.width(Fill)
					.padding(10)
				),
			],
		];

		ui.into()
	}

	pub fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Created => {
				self.refresh();
				return text_input::focus("search");
			}
			Message::SearchChanged(s) => {
				self.search_term = s.clone();
				self.refresh();
			}
			Message::TermSelected(i) => {
				let s = &self.term_list[i];

				if let Some(dict) = self.dicts.get(self.selected_dict) {
					let entry = &dict.entries[s];
					self.term_description = markdown::parse(&entry2md(entry)).collect();
				}
			}
			Message::DictSelected(i) => {
				self.selected_dict = i;
				self.refresh();
			}
			Message::LinkClicked(_) => (),
		}

		Task::none()
	}
}

fn main() -> iced::Result {
	eprintln!("Initialization starting...");
	iced::application(TITLE, State::update, State::view)
		.subscription(subscription)
		.run()
}

fn subscription(_: &State) -> Subscription<Message> {
	window::open_events().map(|_| Message::Created)
}

/// Parse ODict Entry to markdown String
pub fn entry2md(entry: &Entry) -> String {
	let mut md = String::new();
	writeln!(md, "## {}", entry.term).unwrap();

	for (i, ety) in entry.etymologies.iter().enumerate() {
		write!(md, "\n### Etymology #{}\n", i + 1).unwrap();
		if let Some(desc) = &ety.description {
			write!(md, "\n*{}*\n", desc).unwrap();
		}
		for (pos, sense) in &ety.senses {
			write!(md, "\n**{}**\n", pos.description()).unwrap();
			for (j, def) in sense.definitions.iter().enumerate() {
				match def {
					DefinitionType::Definition(def) => {
						write!(md, "\n{}. {}\n", j + 1, def.value).unwrap();
						for example in &def.examples {
							writeln!(md, "\t- *{}*", example.value).unwrap();
						}

						if !def.notes.is_empty() {
							write!(md, "\n\t**Notes**\n").unwrap();
						}

						for (k, note) in def.notes.iter().enumerate() {
							write!(md, "\n\t{}. {}\n", k + 1, note.value).unwrap();
						}
					}
					DefinitionType::Group(_) => {
						todo!();
					}
				}
			}
		}
	}

	md
}
