use std::{fmt::Write, fs};

use directories::ProjectDirs;
use iced::{
	Element, Theme,
	widget::{
		Column, Row, column, container, horizontal_rule, markdown, row, scrollable, text,
		text_input, vertical_rule,
	},
};
use odict::{DefinitionType, Dictionary, DictionaryReader, Entry};

pub const PROJECT_NAME: &str = "mydict";
pub const PROJECT_TITLE: &str = "My Dictionary";

#[derive(Debug)]
struct State {
	pub dictionaries: Vec<Dictionary>,
	pub search_word: String,
	pub word_list: Vec<String>,
	pub dict_list: Vec<String>,
	pub word_description: Vec<markdown::Item>,
}

#[derive(Debug, Clone)]
pub enum Message {
	SearchChanged(String),
	LinkClicked(markdown::Url),
}

impl State {
	pub fn view(&self) -> Element<Message> {
		row![
			container(scrollable(Column::from_iter(
				self.word_list
					.iter()
					.map(|s| container(text!("{s}")).padding(5).into())
			)),)
			.width(200)
			.padding(5),
			vertical_rule(2),
			column![
				container(text_input("", &self.search_word).on_input(Message::SearchChanged),)
					.height(50)
					.padding(10),
				horizontal_rule(2),
				container(scrollable(Row::from_iter(
					self.dict_list
						.iter()
						.map(|s| container(text!("{s}")).padding(5).into())
				)))
				.padding(5),
				horizontal_rule(2),
				container(scrollable(
					markdown::view(&self.word_description, Theme::default())
						.map(Message::LinkClicked),
				))
				.padding(10),
			],
		]
		.into()
	}

	pub fn update(&mut self, message: Message) {
		if let Message::SearchChanged(s) = message {
			self.search_word = s.clone();
			if !self.dictionaries[0].entries.contains_key(&s) {
				self.word_description.clear();
				return;
			}

			let entry = &self.dictionaries[0].entries[&s];
			self.word_description = markdown::parse(&entry2md(entry)).collect();
		}
	}
}

impl Default for State {
	fn default() -> Self {
		let proj_dirs = ProjectDirs::from("", "", PROJECT_NAME).unwrap();
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
		let word_list = dictionaries[0].lexicon()[..1000]
			.iter()
			.map(|s| s.to_string())
			.collect::<Vec<String>>();
		let word_md = entry2md(&dictionaries[0].entries["do"]);
		let word_description = markdown::parse(&word_md).collect::<Vec<markdown::Item>>();

		eprintln!("Initialization done.");

		Self {
			dictionaries,
			search_word: String::default(),
			word_list,
			dict_list: vec!["Open English WordNet".to_string(), "OPTED".to_string()],
			word_description,
		}
	}
}

fn main() -> iced::Result {
	eprintln!("Initialization starting...");
	iced::run(PROJECT_TITLE, State::update, State::view)
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
