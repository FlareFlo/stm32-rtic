#![deny(unsafe_code)]
#![no_main]
#![no_std]
#![allow(unused)] // TODO: Remove when done prototyping (never :)
#![feature(async_closure)]
#![feature(generic_const_exprs)]

use panic_probe as _;
use rtic_monotonics::systick::ExtU32;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [SDIO])]
mod app {
	use core::ops::{Deref, Sub};

	use cortex_m::asm::delay;
	use cortex_m::interrupt::disable;
	use defmt::{error, export::panic, info, println, warn};
	use defmt::export::display;
	use defmt_rtt as _;
	use embedded_graphics::{
		image::Image,
		mono_font::{*, ascii::*},
		pixelcolor::BinaryColor,
		prelude::*,
		primitives::Rectangle,
		text::{Alignment, Text},
	};
	use embedded_hal::spi::{Mode, Phase, Polarity};
	use epd::display::DisplaySize128x296;
	use epd::{EPDInterface, TriColor, TriColorEPD};
	use profont::{PROFONT_18_POINT, PROFONT_24_POINT, PROFONT_7_POINT};
	use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
	use rtic::{Mutex, mutex_prelude::TupleExt02};
	use rtic_monotonics::systick::Systick;
	use ssd1306::{
		I2CDisplayInterface,
		mode::{BufferedGraphicsMode, DisplayConfig},
		prelude::I2CInterface,
		rotation::DisplayRotation,
		size::DisplaySize128x64,
		Ssd1306,
	};
	use stm32f4xx_hal::{
		gpio::{self, Edge, Input, Output, PushPull},
		i2c::I2c,
		pac::{I2C1, TIM1, TIM2},
		prelude::*,
		rtc::{Event, Rtc},
		timer,
		timer::Delay,
	};
	use stm32f4xx_hal::adc::Adc;
	use stm32f4xx_hal::adc::config::{AdcConfig, SampleTime};
	use stm32f4xx_hal::gpio::{Analog, OpenDrain, Speed};
	use stm32f4xx_hal::pac::ADC1;
	use stm32f4xx_hal::pac::SPI1;
	use stm32f4xx_hal::spi::{NoMiso, Spi, Spi1};
	use time::{Duration, PrimitiveDateTime};
	use to_arraystring::ToArrayString;

	use tachometer::{
		Tachometer,
		TireDimensions,
		units::{length::Length, time::Time},
	};

	use crate::shared;

	// Resources shared between tasks
	#[shared]
	struct Shared {
		delayval: u32,
		rtc: Rtc,
		delay: timer::DelayMs<TIM1>,
		// 75 samples should suffice for around 10 seconds of history at 60 km/h
		tacho: Tachometer<75>,
	}

	// Local resources to specific tasks (cannot be shared)
	#[local]
	struct Local {
		button: (gpio::PA0<Input>, PrimitiveDateTime),
		led: gpio::PC13<Output<PushPull>>,
		display: DisplayType,

		sensor_digital: (gpio::PB1<Input>, PrimitiveDateTime),

		adc1: Adc<ADC1>,

		rolling_speed_average: ConstGenericRingBuffer<f32, 60>,
	}

	type DisplayType = TriColorEPD<EPDInterface<Spi<SPI1>, gpio::Pin<'A', 4, Output>, gpio::Pin<'A', 2, Output>, gpio::Pin<'A', 1, Output>, gpio::Pin<'A', 3>>, DisplaySize128x296, epd::drivers::SSD1680>;

	#[init]
	fn init(ctx: init::Context) -> (Shared, Local) {
		let mut dp = ctx.device;

		let mut adc1 = Adc::adc1(dp.ADC1, true, AdcConfig::default());

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
		let mut delay1 = dp.TIM1.delay_ms(&clocks);
		let mut delay2 = dp.TIM2.delay_ms(&clocks);

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

		let mut sensor_digital = gpiob.pb1.into_pull_down_input();

		// Configure Pins for Interrupts
		// 1) Promote SYSCFG structure to HAL to be able to configure interrupts
		let mut syscfg = dp.SYSCFG.constrain();
		// 2) Make button an interrupt source
		button.make_interrupt_source(&mut syscfg);
		// 3) Configure the interruption to be triggered on a rising edge
		button.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
		// 4) Enable gpio interrupt for button
		button.enable_interrupt(&mut dp.EXTI);

		sensor_digital.make_interrupt_source(&mut syscfg);
		sensor_digital.trigger_on_edge(&mut dp.EXTI, Edge::Falling);
		sensor_digital.enable_interrupt(&mut dp.EXTI);

		let scl = gpioa.pa5.into_alternate().speed(Speed::VeryHigh);
		let mosi = gpioa.pa7.into_alternate().speed(Speed::VeryHigh);
		let cs = gpioa.pa4.into_push_pull_output();
		let busy = gpioa.pa3.into_input();
		let dc = gpioa.pa2.into_push_pull_output();
		let rst = gpioa.pa1.into_push_pull_output();

		let i2c1 = I2c::new(dp.I2C1, (gpiob.pb6, gpiob.pb7), 600_u32.kHz(), &clocks);
		let interface = I2CDisplayInterface::new(i2c1);

		let mut spi1 = Spi::new(
			dp.SPI1,
			(
				scl, // SCL
				NoMiso::new(), // MISO - dead
				mosi, // MOSI
			),
			Mode {
				polarity: Polarity::IdleLow,
				phase: Phase::CaptureOnFirstTransition,
			},
			1.MHz(),
			&clocks,
		);

		let mut driver = EPDInterface::new(spi1, cs, dc, rst, busy);
		let mut display = TriColorEPD::new(driver);
		display.init(&mut delay1).unwrap();
		display.set_rotation(270);
		display.clear(TriColor::White).unwrap();

		#[cfg(feature = "startup-logo")]
		{
			use embedded_graphics::pixelcolor::BinaryColor;
			use tinytga::Tga;

			let data = include_bytes!("../out.tga");
			let tga: Tga<BinaryColor> = Tga::from_slice(data).unwrap();
			Image::new(&tga, Point::zero()).draw(&mut display.framebuf1).unwrap();
			display.display_frame().unwrap();
			delay1.delay_ms(50000);
		}

		let now = rtc.get_datetime();


		// Text::with_alignment(
		// 				"SEX",
		// 				Point::new(50, 60),
		// 				MonoTextStyle::new(&PROFONT_24_POINT,  TriColor::Black),
		// 				Alignment::Center,
		// 			)
		// 			.draw(&mut display)
		// 			.unwrap();
		//
		// display.fill_solid(&Rectangle::new(Point::zero(), Size::new(20,39)), TriColor::Red);
		// display.display_frame().unwrap();

		// pzb_lights::spawn().unwrap();
		// display::spawn().unwrap();
		blinky::spawn().unwrap();
		(
			// Initialization of shared resources
			Shared {
				delayval: 500_u32,
				rtc,
				delay: delay1,
				tacho: Tachometer::new(
					TireDimensions::Diameter(Length::from_centimeters(70.0)),
					1,
					46.0 / 16.0,
				),
			},
			// Initialization of task local resources
			Local {
				button: (button, now),
				led,
				display: display,
				sensor_digital: (sensor_digital, now),
				adc1,
				rolling_speed_average: Default::default(),
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

	#[task(local = [led], shared = [tacho, rtc])]
	async fn blinky(mut ctx: blinky::Context) {
		let led = ctx.local.led;
		loop {
			// ctx.shared.tacho.lock(|tacho| {
			// 	tacho.insert(
			// 		ctx.shared
			// 			.rtc
			// 			.lock(|rtc| rtc.get_datetime())
			// 			.assume_utc()
			// 			.unix_timestamp_nanos() / 1_000_000,
			// 	)
			// });
			let delay = 500_u32.millis();
			led.set_high();
			Systick::delay(delay).await;
			led.set_low();
			Systick::delay(delay).await;
		}
	}

	// #[task(local = [display, rolling_speed_average], shared = [rtc, tacho])]
	// async fn display(mut ctx: display::Context) {
	// 	let display = ctx.local.display;
	// 	let rtc = &mut ctx.shared.rtc;
	// 	let rolling_speed_average = ctx.local.rolling_speed_average;
	//
	// 	let timeframe = 3_000; // Milliseconds
	//
	// 	let mut fc: u32 = 0;
	// 	loop {
	// 		let now = ctx.shared.rtc.lock(|rtc| rtc.get_datetime());
	//
	// 		let sample = ctx.shared.tacho.lock(|tacho| {
	// 			tacho.last_samples(
	// 				timeframe,
	// 				now.assume_utc().unix_timestamp_nanos() / 1_000_000,
	// 			)
	// 		});
	// 		let speed = (sample
	// 			.distance
	// 			.to_speed(Time::milliseconds(timeframe as f32)));
	//
	// 		rolling_speed_average.push(speed.as_kilometers_per_hour());
	//
	// 		// Take average from last second of speeds
	// 		let avg_speed = rolling_speed_average.iter().sum::<f32>() / rolling_speed_average.len() as f32;
	//
	// 		let mut buf = [0u8; 30];
	// 		let formatted_speed = format_no_std::show(
	// 			&mut buf,
	// 			format_args!("{:.1}kmh", avg_speed),
	// 		)
	// 		.unwrap();
	//
	// 		let mut buf = [0u8; 30];
	// 		let formatted_cadence =
	// 			format_no_std::show(&mut buf, format_args!("{:.1}rpm", sample.cadence)).unwrap();
	//
	// 		let mut buf = [0u8; 30];
	// 		let formatted_distance = format_no_std::show(
	// 			&mut buf,
	// 			format_args!(
	// 				"{:.1}m",
	// 				ctx.shared
	// 					.tacho
	// 					.lock(|tacho| tacho.total_distance_moved().as_meter())
	// 			),
	// 		)
	// 		.unwrap();
	//
	// 		// Draw speed
	// 		Text::with_alignment(
	// 			formatted_speed,
	// 			display.bounding_box().center() + Point::new(0, 10),
	// 			MonoTextStyle::new(&PROFONT_24_POINT, BinaryColor::On),
	// 			Alignment::Center,
	// 		)
	// 		.draw(display)
	// 		.unwrap();
	//
	// 		/// Draw all-time distance
	// 		Text::with_alignment(
	// 			formatted_distance,
	// 			display.bounding_box().center() + Point::new(0, 21),
	// 			MonoTextStyle::new(&FONT_7X14, BinaryColor::On),
	// 			Alignment::Center,
	// 		)
	// 		.draw(display)
	// 		.unwrap();
	//
	// 		/// Draw cadence
	// 		Text::with_alignment(
	// 			formatted_cadence,
	// 			display.bounding_box().center() + Point::new(0, 28),
	// 			MonoTextStyle::new(&FONT_6X9, BinaryColor::On),
	// 			Alignment::Center,
	// 		)
	// 		.draw(display)
	// 		.unwrap();
	//
	// 		// Draw frame counter
	// 		Text::with_alignment(
	// 			fc.to_arraystring().as_str(),
	// 			Point::new(0, 6),
	// 			MonoTextStyle::new(&FONT_6X9, BinaryColor::On),
	// 			Alignment::Left,
	// 		)
	// 		.draw(display)
	// 		.unwrap();
	//
	// 		display.flush().unwrap();
	// 		display.clear_buffer();
	// 		Systick::delay(2000_u32.millis()).await;
	// 		fc += 1;
	// 	}
	// }

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

	#[task(binds = EXTI0, local = [button, adc1], shared = [rtc, tacho], priority = 1)]
	fn exti0_interrupt(mut ctx: exti0_interrupt::Context) {
		let button = ctx.local.button;
		let adc = ctx.local.adc1;

		button.0.clear_interrupt_pending_bit();

		let now = ctx.shared.rtc.lock(|rtc| rtc.get_datetime());

		if now - button.1 <= Duration::milliseconds(100) {
			return;
		}
		button.1 = now;

		ctx.shared
			.tacho
			.lock(|tacho| tacho.insert(now.assume_utc().unix_timestamp_nanos() / 1_000_000));
	}

	#[task(binds = EXTI1, local = [sensor_digital], shared = [rtc, tacho], priority = 1)]
	fn exti1_interrupt(mut ctx: exti1_interrupt::Context) {
		let dsensor = ctx.local.sensor_digital;
		dsensor.0.clear_interrupt_pending_bit();

		let now = ctx.shared.rtc.lock(|rtc| rtc.get_datetime());

		if now - dsensor.1 <= Duration::milliseconds(100) {
			return;
		}
		dsensor.1 = now;

		ctx.shared
			.tacho
			.lock(|tacho| tacho.insert(now.assume_utc().unix_timestamp_nanos() / 1_000_000));
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
