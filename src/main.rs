use iced::{
	Element, Theme,
	widget::{
		Column, Container, Row, button, column, container, horizontal_rule, markdown, row,
		scrollable, text, text_input, vertical_rule,
	},
};

#[derive(Debug)]
struct State {
	pub search_word: String,
	pub word_list: Vec<String>,
	pub dict_list: Vec<String>,
	pub word_md: Vec<markdown::Item>,
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
				container(
					markdown::view(&self.word_md, Theme::default()).map(Message::LinkClicked),
				)
				.padding(10),
			],
		]
		.into()
	}

	pub fn update(&mut self, message: Message) {
		match message {
			Message::SearchChanged(s) => {
				self.search_word = s.clone();
				self.word_list = vec![format!("{s}1"), format!("{s}2"), format!("{s}3")]
			}
			_ => (),
		}
	}
}

impl Default for State {
	fn default() -> Self {
		Self {
			search_word: String::default(),
			word_list: Vec::default(),
			dict_list: vec!["Open English WordNet".to_string(), "OPTED".to_string()],
			word_md: markdown::parse("This is a *word*.").collect::<Vec<markdown::Item>>(),
		}
	}
}

fn main() -> iced::Result {
	iced::run("My Dictionary", State::update, State::view)
}
