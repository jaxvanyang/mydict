use std::fmt::Write;

use iced::{
	Element, Theme,
	widget::{
		Column, Row, column, container, horizontal_rule, markdown, row, scrollable, text,
		text_input, vertical_rule,
	},
};
use odict::{DefinitionType, Dictionary, DictionaryReader, Entry};

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
		let dictionaries = vec![
			DictionaryReader::new()
				.read_from_path("enwn2odict-2024.2.odict")
				.expect("ODict file exists")
				.to_dictionary()
				.expect("ODict imported"),
		];
		let word_list = dictionaries[0].lexicon()[..1000]
			.iter()
			.map(|s| s.to_string())
			.collect::<Vec<String>>();
		let word_md = entry2md(&dictionaries[0].entries["do"]);
		let word_description = markdown::parse(&word_md).collect::<Vec<markdown::Item>>();

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
	iced::run("My Dictionary", State::update, State::view)
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
