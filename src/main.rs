#![deny(unsafe_code)]
#![no_main]
#![no_std]
#![allow(unused)] // TODO: Remove when done prototyping (never :)
#![feature(async_closure)]


use panic_probe as _;
use rtic_monotonics::systick::ExtU32;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [SDIO])]
mod app {
	use core::ops::{Deref, Sub};

	use cortex_m::asm::delay;
	use defmt::{error, export::panic, info, println, warn};
	use defmt_rtt as _;
	use embedded_graphics::image::Image;

	use embedded_graphics::prelude::*;
	use embedded_graphics::mono_font::ascii::*;
	use embedded_graphics::mono_font::*;
	use embedded_graphics::pixelcolor::BinaryColor;
	use embedded_graphics::primitives::Rectangle;
	use embedded_graphics::text::Text;
	use embedded_graphics::text::Alignment;

	use rtic::{mutex_prelude::TupleExt02, Mutex};
	use rtic_monotonics::systick::Systick;

	use ssd1306::mode::BufferedGraphicsMode;
	use ssd1306::{I2CDisplayInterface, Ssd1306};
	use ssd1306::prelude::I2CInterface;
	use ssd1306::rotation::DisplayRotation;
	use ssd1306::size::DisplaySize128x64;
	use ssd1306::mode::DisplayConfig;

	use stm32f4xx_hal::{
		gpio::{self, Edge, Input, Output, PushPull},
		i2c::I2c,
		pac::{I2C1, TIM1, TIM2},
		prelude::*,
		rtc::{Event, Rtc},
		timer,
		timer::Delay,
	};
	use time::{Duration, PrimitiveDateTime};
	use tachometer::{Tachometer, TireDimensions};

	use crate::{shared};

	// Resources shared between tasks
	#[shared]
	struct Shared {
		delayval: u32,
		rtc:      Rtc,
		delay:    timer::DelayMs<TIM1>,
		// 75 samples should suffice for around 10 seconds of history at 60 km/h
		tacho: Tachometer<75>,
	}

	// Local resources to specific tasks (cannot be shared)
	#[local]
	struct Local {
		button: (gpio::PA0<Input>, PrimitiveDateTime),
		led:    gpio::PC13<Output<PushPull>>,
		display:  Ssd1306<I2CInterface<I2c<I2C1>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
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
		let mut delay = dp.TIM1.delay_ms(&clocks);

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

		let i2c1 = I2c::new(dp.I2C1, (gpiob.pb6, gpiob.pb7), 600_u32.kHz(), &clocks);
		let interface = I2CDisplayInterface::new(i2c1);

		let mut display = Ssd1306::new(
			interface,
			DisplaySize128x64,
			DisplayRotation::Rotate0,
		).into_buffered_graphics_mode();
		display.init().unwrap();

		#[cfg(feature = "startup-logo")]
		{
			use tinytga::Tga;
			use embedded_graphics::pixelcolor::BinaryColor;

			let data = include_bytes!("../images/feris/final.tga");
			let tga: Tga<BinaryColor> = Tga::from_slice(data).unwrap();
			Image::new(&tga, Point::zero()).draw(&mut display).unwrap();
			display.flush().unwrap();
			delay.delay_ms(5000);
		}

		display.flush().unwrap();


		let now = rtc.get_datetime();


		// pzb_lights::spawn().unwrap();
		display::spawn().unwrap();
		blinky::spawn().unwrap();
		(
			// Initialization of shared resources
			Shared {
				delayval: 500_u32,
				rtc,
				delay,
				tacho: Tachometer::new(TireDimensions::Diameter(70.0)),
			},
			// Initialization of task local resources
			Local {
				button: (button, now),
				led,
				display,
			},
		)
	}

	// Not used
	// #[idle()]
	// fn idle(ctx: idle::Context) -> ! {
	// 	loop {
	//
	// 	}
	// }


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

	#[task(local = [display], shared = [rtc, tacho])]
	async fn display(mut ctx: display::Context) {
		let display = ctx.local.display;
		let rtc = &mut ctx.shared.rtc;

		let character_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
		let timeframe = 3000; // Milliseconds

		let mut fc = 0;
		loop {
			let latest_distance = ctx.shared.tacho.lock(|tacho|tacho.last_distance_moved(timeframe)) / timeframe as f32;
			println!("{}", latest_distance);
			let mut buf = itoa::Buffer::new();
			display.clear_buffer();
			Text::with_alignment(
				buf.format(latest_distance as u32),
				display.bounding_box().center() + Point::new(0, 15),
				character_style,
				Alignment::Center,
			).draw(display).unwrap();
			display.flush().unwrap();
			Systick::delay(33_u32.millis()).await;
			fc += 1;
		}
	}

	// #[task(local = [display], shared = [rtc])]
	// async fn display(mut ctx: display::Context) {
	// 	let display = ctx.local.display;
	// 	let rtc = &mut ctx.shared.rtc;
	// 	loop {
	// 		let now = rtc.lock(|rtc|rtc.get_datetime());
	// 		display.fill_solid(&Rectangle::new(Point::zero(), Size::new(128, 64)), BinaryColor::On);
	// 		display.flush().unwrap();
	// 		let after = rtc.lock(|rtc|rtc.get_datetime());
	// 		println!("{}", 1.0 / (after - now).as_seconds_f64());
	//
	// 		let now = rtc.lock(|rtc|rtc.get_datetime());
	// 		display.fill_solid(&Rectangle::new(Point::zero(), Size::new(128, 64)), BinaryColor::Off);
	// 		display.flush().unwrap();
	// 		let after = rtc.lock(|rtc|rtc.get_datetime());
	// 		println!("{}", 1.0 / (after - now).as_seconds_f64());
	// 	}
	// }

	#[task(binds = EXTI0, local = [button], shared = [rtc, tacho], priority = 1)]
	fn gpio_interrupt_handler(mut ctx: gpio_interrupt_handler::Context) {
		ctx.local.button.0.clear_interrupt_pending_bit();

		let now = ctx.shared.rtc.lock(|rtc| rtc.get_datetime());

		// If this case is true, it means the button was "pressed" within the same millisecond
		// therefore we will reject its input and return early
		if now - ctx.local.button.1 <= Duration::milliseconds(100) {
			return;
		}
		ctx.local.button.1 = now;

		ctx.shared.tacho.lock(|tacho|tacho.insert(now.assume_utc().unix_timestamp()));
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
