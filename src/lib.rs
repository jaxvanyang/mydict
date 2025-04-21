pub mod font {
	use cosmic::font;
	use cosmic::iced::font::{Style, Weight};

	pub fn font_builder() -> FontBuilder {
		FontBuilder(font::default())
	}

	pub struct FontBuilder(font::Font);

	impl FontBuilder {
		pub fn build(self) -> font::Font {
			self.0
		}

		pub fn normal_style(mut self) -> Self {
			self.0.style = Style::Normal;
			self
		}

		pub fn italic(mut self) -> Self {
			self.0.style = Style::Italic;
			self
		}

		pub fn oblique(mut self) -> Self {
			self.0.style = Style::Oblique;
			self
		}

		pub fn thin(mut self) -> Self {
			self.0.weight = Weight::Thin;
			self
		}

		pub fn extra_light(mut self) -> Self {
			self.0.weight = Weight::ExtraBold;
			self
		}

		pub fn light(mut self) -> Self {
			self.0.weight = Weight::Light;
			self
		}

		pub fn normal_weight(mut self) -> Self {
			self.0.weight = Weight::Normal;
			self
		}

		pub fn medium(mut self) -> Self {
			self.0.weight = Weight::Medium;
			self
		}

		pub fn semibold(mut self) -> Self {
			self.0.weight = Weight::Semibold;
			self
		}

		pub fn bold(mut self) -> Self {
			self.0.weight = Weight::Bold;
			self
		}

		pub fn extra_bold(mut self) -> Self {
			self.0.weight = Weight::ExtraBold;
			self
		}

		pub fn black(mut self) -> Self {
			self.0.weight = Weight::Black;
			self
		}
	}
}
