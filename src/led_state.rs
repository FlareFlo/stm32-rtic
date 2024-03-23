use crate::{impl_led_blinking, impl_led_state_builder, impl_led_static};

pub enum LedState {
	Off,
	Static,
	Blinking,
}

pub struct PzbLedState {
	b85: LedState,
	b70: LedState,
	b55: LedState,
	hz1000: LedState,
	hz500: LedState,
	command: LedState,
}

impl PzbLedState {
	pub const fn off() -> Self {
		Self {
			b85: LedState::Off,
			b70: LedState::Off,
			b55: LedState::Off,
			hz1000: LedState::Off,
			hz500: LedState::Off,
			command: LedState::Off,
		}
	}
}

impl_led_state_builder!((b85 b85_blink), (b70 b70_blink), (b55 b55_blink), (hz1000 hz1000_blink), (hz500 hz500_blink), (command command_blink));
mod macros {
	#[macro_export]
	macro_rules! impl_led_state_builder {
    ( $( ( $field:ident $fn_name:ident )),* ) => {
		impl PzbLedState {
			$(
				impl_led_static!($field, $field);
				impl_led_blinking!($fn_name, $field);
			)*
		}
	};
}

	#[macro_export]
	macro_rules! impl_led_static {
		($fn_name:ident, $field:ident) => {
			pub const fn $fn_name(mut self) -> Self {
				self.$field = LedState::Static;
				self
			}
		};
	}
	#[macro_export]
	macro_rules! impl_led_blinking {
		($fn_name:ident, $field:ident) => {
			pub const fn $fn_name(mut self) -> Self {
				self.$field = LedState::Blinking;
				self
			}
		};
	}
}
