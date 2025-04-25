pub mod font {
	use cosmic::font;
	use cosmic::iced::font::{Style, Weight};

	#[must_use]
	pub fn font_builder() -> FontBuilder {
		FontBuilder(font::default())
	}

	pub struct FontBuilder(font::Font);

	impl FontBuilder {
		#[must_use]
		pub fn build(self) -> font::Font {
			self.0
		}

		#[must_use]
		pub fn normal_style(mut self) -> Self {
			self.0.style = Style::Normal;
			self
		}

		#[must_use]
		pub fn italic(mut self) -> Self {
			self.0.style = Style::Italic;
			self
		}

		#[must_use]
		pub fn oblique(mut self) -> Self {
			self.0.style = Style::Oblique;
			self
		}

		#[must_use]
		pub fn thin(mut self) -> Self {
			self.0.weight = Weight::Thin;
			self
		}

		#[must_use]
		pub fn extra_light(mut self) -> Self {
			self.0.weight = Weight::ExtraBold;
			self
		}

		#[must_use]
		pub fn light(mut self) -> Self {
			self.0.weight = Weight::Light;
			self
		}

		#[must_use]
		pub fn normal_weight(mut self) -> Self {
			self.0.weight = Weight::Normal;
			self
		}

		#[must_use]
		pub fn medium(mut self) -> Self {
			self.0.weight = Weight::Medium;
			self
		}

		#[must_use]
		pub fn semibold(mut self) -> Self {
			self.0.weight = Weight::Semibold;
			self
		}

		#[must_use]
		pub fn bold(mut self) -> Self {
			self.0.weight = Weight::Bold;
			self
		}

		#[must_use]
		pub fn extra_bold(mut self) -> Self {
			self.0.weight = Weight::ExtraBold;
			self
		}

		#[must_use]
		pub fn black(mut self) -> Self {
			self.0.weight = Weight::Black;
			self
		}
	}
}

pub mod dict;
pub mod utils;

pub use dict::*;
pub use utils::*;
