use std::{fmt::Write, fs};

use directories::ProjectDirs;
use iced::{
	Element,
	Length::Fill,
	Subscription, Task, Theme,
	widget::{
		Column, Row, column, container, horizontal_rule, markdown, row, scrollable, text,
		text_input, vertical_rule,
	},
	window,
};
use odict::{DefinitionType, Dictionary, DictionaryReader, Entry};

pub const NAME: &str = "mydict";
pub const TITLE: &str = "My Dictionary";

#[derive(Debug)]
struct State {
	pub dictionaries: Vec<Dictionary>,
	pub search_word: String,
	pub word_list: Vec<String>,
	pub word_description: Vec<markdown::Item>,
}

#[derive(Debug, Clone)]
pub enum Message {
	Created,
	SearchChanged(String),
	LinkClicked(markdown::Url),
}

impl Default for State {
	fn default() -> Self {
		let proj_dirs = ProjectDirs::from("", "", NAME).unwrap();
		let data_dir = proj_dirs.data_dir();
		if !data_dir.exists() {
			fs::create_dir(data_dir).expect("created directory");
		}
		let reader = DictionaryReader::new();
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
		let word_md = entry2md(&dictionaries[0].entries["do"]);
		let word_description = markdown::parse(&word_md).collect::<Vec<markdown::Item>>();

		eprintln!("Initialization done.");

		Self {
			dictionaries,
			search_word: String::default(),
			word_list: Vec::new(),
			word_description,
		}
	}
}

impl State {
	pub fn view(&self) -> Element<Message> {
		let ui = row![
			container(scrollable(Column::from_iter(
				self.word_list
					.iter()
					.map(|s| container(text!("{s}")).padding(5).into())
			)),)
			.width(200)
			.padding(5),
			vertical_rule(2),
			column![
				container(
					text_input("", &self.search_word)
						.id("search")
						.on_input(Message::SearchChanged),
				)
				.height(50)
				.padding(10),
				horizontal_rule(2),
				container(scrollable(Row::from_iter(self.dictionaries.iter().map(
					|d| {
						container(text!(
							"{}",
							d.name.as_ref().expect("dictionary should have name")
						))
						.padding(5)
						.into()
					}
				))))
				.padding(5),
				horizontal_rule(2),
				scrollable(
					container(
						markdown::view(&self.word_description, Theme::default())
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
			Message::Created => text_input::focus("search"),
			Message::SearchChanged(s) => {
				self.search_word = s.clone();
				let s = s.trim();
				let dict = &self.dictionaries[0];
				self.word_list = dict
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

				if !self.dictionaries[0].entries.contains_key(s) {
					self.word_description.clear();
					return Task::none();
				}

				let entry = &dict.entries[s];
				self.word_description = markdown::parse(&entry2md(entry)).collect();

				Task::none()
			}
			_ => Task::none(),
		}
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
		write!(
			md,
			"\n*{}*\n",
			ety.description
				.as_ref()
				.expect("Etymology should have description")
		)
		.unwrap();
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
