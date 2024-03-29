#![deny(unsafe_code)]
#![no_main]
#![no_std]
#![allow(unused)] // TODO: Remove when done prototyping (never :)
#![feature(async_closure)]

pub mod led_state;
pub mod pzb;
pub mod pzb_state;

use panic_probe as _;
use rtic_monotonics::systick::ExtU32;

pub mod pins {
	use stm32f4xx_hal::gpio::{Output, PushPull, PA8, PA9, PB12, PB13, PB14, PB15};

	pub type Blue55 = PB12<Output<PushPull>>;
	pub type Blue70 = PB13<Output<PushPull>>;
	pub type Blue85 = PB14<Output<PushPull>>;

	pub type Hz1000 = PB15<Output<PushPull>>;
	pub type Hz500 = PA8<Output<PushPull>>;
	pub type Command = PA9<Output<PushPull>>;
}

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [SDIO])]
mod app {
	use cortex_m::asm::delay;
	use defmt::{error, export::panic, info, warn};
	use defmt_rtt as _;
	use rtic::{mutex_prelude::TupleExt02, Mutex};
	use stm32f4xx_hal::{
		gpio::{self, Edge, Input, Output, PushPull},
		pac::TIM1,
		prelude::*,
		rtc::{Event, Rtc},
		timer,
	};
	use rtic_monotonics::systick::Systick;
	use core::ops::Deref;
	use stm32f4xx_hal::i2c::I2c;
	use embedded_aht20::Aht20;
	use embedded_aht20::DEFAULT_I2C_ADDRESS;
	use stm32f4xx_hal::pac::{I2C1, TIM2};
	use stm32f4xx_hal::timer::Delay;


	use crate::{led_state::PzbLedState, pzb::PzbCategory, pzb_state::PzbState, shared};

	// Resources shared between tasks
	#[shared]
	struct Shared {
		delayval: u32,
		rtc:      Rtc,
		leds:     Leds,
		delay:    timer::DelayMs<TIM1>,

		pzb_state: PzbState,
	}

	pub struct Leds {
		pub b55:     crate::pins::Blue55,
		pub b70:     crate::pins::Blue70,
		pub b85:     crate::pins::Blue85,
		pub hz1000:  crate::pins::Hz1000,
		pub hz500:   crate::pins::Hz500,
		pub command: crate::pins::Command,
	}

	// Local resources to specific tasks (cannot be shared)
	#[local]
	struct Local {
		button: (gpio::PA0<Input>, u16),
		led:    gpio::PC13<Output<PushPull>>,
		aht20: Aht20<I2c<I2C1>, Delay<TIM2, 1000>>,
	}

	#[init]
	fn init(ctx: init::Context) -> (Shared, Local) {
		let mut dp = ctx.device;

		// Configure and obtain handle for delay abstraction
		// 1) Promote RCC structure to HAL to be able to configure clocks
		let rcc = dp.RCC.constrain();

		// 2) Configure the system clocks
		// 25 MHz must be used for HSE on the Blackpill-STM32F411CE board according to manual
		let clocks = rcc.cfgr.use_hse(25.MHz()).freeze();


		let systick_token = rtic_monotonics::create_systick_token!();
		Systick::start(ctx.core.SYST, clocks.sysclk().to_Hz(), systick_token);

		// Configure RTC
		let mut rtc = Rtc::new(dp.RTC, &mut dp.PWR);

		// Set date and time
		let _ = rtc.set_year(2024);
		let _ = rtc.set_month(3);
		let _ = rtc.set_day(22);
		let _ = rtc.set_hours(22);
		let _ = rtc.set_minutes(57);
		let _ = rtc.set_seconds(00);

		// 3) Create delay handle
		let delay = dp.TIM1.delay_ms(&clocks);

		let gpiob = dp.GPIOB.split();

		// Configure the LED pin as a push pull ouput and obtain handle
		// On the Blackpill STM32F411CEU6 there is an on-board LED connected to pin PC13
		// 1) Promote the GPIOC PAC struct
		let gpioc = dp.GPIOC.split();

		// 2) Configure PORTC OUTPUT Pins and Obtain Handle
		let led = gpioc.pc13.into_push_pull_output();

		// Configure the button pin as input and obtain handle
		// On the Blackpill STM32F411CEU6 there is a button connected to pin PA0
		// 1) Promote the GPIOA PAC struct
		let gpioa: gpio::gpioa::Parts = dp.GPIOA.split();
		// 2) Configure Pin and Obtain Handle
		let mut button = gpioa.pa0.into_pull_up_input();

		// Configure Button Pin for Interrupts
		// 1) Promote SYSCFG structure to HAL to be able to configure interrupts
		let mut syscfg = dp.SYSCFG.constrain();
		// 2) Make button an interrupt source
		button.make_interrupt_source(&mut syscfg);
		// 3) Configure the interruption to be triggered on a rising edge
		button.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
		// 4) Enable gpio interrupt for button
		button.enable_interrupt(&mut dp.EXTI);

		let b55 = gpiob.pb12.into_push_pull_output();
		let b70 = gpiob.pb13.into_push_pull_output();
		let b85 = gpiob.pb14.into_push_pull_output();
		let hz1000 = gpiob.pb15.into_push_pull_output();
		let hz500 = gpioa.pa8.into_push_pull_output();
		let command = gpioa.pa9.into_push_pull_output();

		let i2c_temp = I2c::new(
			dp.I2C1,
			(gpiob.pb6,
			 gpiob.pb7),
			100_u32.kHz(),
			&clocks
		);

		let aht20 = Aht20::new(i2c_temp, DEFAULT_I2C_ADDRESS, dp.TIM2.delay_ms(&clocks)).unwrap();


		pzb_lights::spawn().unwrap();
		aht20::spawn().unwrap();
		blinky::spawn().unwrap();
		(
			// Initialization of shared resources
			Shared {
				delayval: 500_u32,
				rtc,
				leds: Leds {
					b55,
					b70,
					b85,
					hz1000,
					hz500,
					command,
				},
				pzb_state: PzbState::Free,
				delay,
			},
			// Initialization of task local resources
			Local { button: (button, 0), led, aht20 },
		)
	}

	// Not used
	// #[idle()]
	// fn idle(ctx: idle::Context) -> ! {
	// 	loop {
	//
	// 	}
	// }

	// Background task, runs whenever no other tasks are running
	#[task(shared = [delayval, leds, pzb_state, delay])]
	async fn pzb_lights(mut ctx: pzb_lights::Context) {
		let mut delayval = ctx.shared.delayval;
		loop {
			// Set non-alternating PZB state
			shared!(
				ctx,
				|leds, pzb_state| {
					let pzb_led = pzb_state.enabled(PzbCategory::O);
					pzb_led.set_leds(leds, true);
				},
				leds,
				pzb_state
			);

			// Sleep for full PZB cycle
			Systick::delay(delayval.lock(|val|*val).millis()).await;

			// Set alternating PZB state
			shared!(
				ctx,
				|leds, pzb_state| {
					let pzb_led = pzb_state.enabled(PzbCategory::O);
					pzb_led.set_leds(leds, false);
				},
				leds,
				pzb_state
			);

			// Sleep for full PZB cycle
			Systick::delay(delayval.lock(|val|*val).millis()).await;
		}
	}

	#[task(local = [led])]
	async fn blinky(mut ctx: blinky::Context) {
		let led = ctx.local.led;
		loop {
			let delay = 500_u32.millis();
			led.set_high();
			Systick::delay(delay).await;
			led.set_low();
			Systick::delay(delay).await;
		}
	}

	#[task(local = [aht20])]
	async fn aht20(mut ctx: aht20::Context) {
		let sensor = ctx.local.aht20;
		loop {
			let measurement = sensor.measure().unwrap();
			Systick::delay(100_u32.millis()).await;

			info!("temp: {=f32} hum: {}% abs-hum: {}g/m³", measurement.temperature.celcius(), measurement.relative_humidity, measurement.absolute_humidity());

			Systick::delay(400_u32.millis()).await;
		}
	}

	#[task(binds = EXTI0, local = [button], shared = [pzb_state, delay, rtc], priority = 1)]
	fn gpio_interrupt_handler(mut ctx: gpio_interrupt_handler::Context) {
		ctx.local.button.0.clear_interrupt_pending_bit();
		
		let now = ctx.shared.rtc.lock(|rtc| rtc.get_datetime());
		let relative_time = now.second() as u16 + 1000 * now.millisecond();

		// If this case is true, it means the button was "pressed" within the same millisecond
		// therefore we will reject its input and return early
		if ctx.local.button.1 == relative_time {
			return;
		}
		ctx.local.button.1 = relative_time;

		ctx.shared.pzb_state.lock(|state| {
			*state = state.next();
		});
		
	}
}

#[macro_export]
macro_rules! shared {
    ($ctx:ident, $lock:expr, $( $lockable:ident ),* ) => {
		($(
		&mut $ctx.shared.$lockable
		),*)
		.lock($lock)
	};
}
