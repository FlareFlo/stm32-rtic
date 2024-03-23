use stm32f4xx_hal::gpio::PinState;

use crate::{app::Leds, impl_led_blinking, impl_led_state_builder, impl_led_static};

#[derive(Copy, Clone)]
pub enum LedState {
	Off,
	Static,
	Blinking,
}

impl LedState {
	pub const fn to_bool(self, cycle: bool) -> bool {
		match self {
			LedState::Off => false,
			LedState::Static => true,
			LedState::Blinking => cycle,
		}
	}
}

#[derive(Copy, Clone)]
pub struct PzbLedState {
	b85:     LedState,
	b70:     LedState,
	b55:     LedState,
	hz1000:  LedState,
	hz500:   LedState,
	command: LedState,
}

impl PzbLedState {
	pub const fn off() -> Self {
		Self {
			b85:     LedState::Off,
			b70:     LedState::Off,
			b55:     LedState::Off,
			hz1000:  LedState::Off,
			hz500:   LedState::Off,
			command: LedState::Off,
		}
	}

	pub fn set_leds(self, leds: &mut Leds, light_cycle: bool) {
		#[macro_export]
		macro_rules! impl_led_set {
			( $( $struct_field:ident ),* ) => {
				$(
				leds.$struct_field
					.set_state(PinState::from(self.$struct_field.to_bool(light_cycle)));
				)*
			};
		}

		impl_led_set!(b85, b70, b55, hz1000, hz500, command);
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
