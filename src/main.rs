#![deny(unsafe_code)]
#![no_main]
#![no_std]

pub mod led_state;
pub mod pzb;
pub mod pzb_state;

use panic_halt as _;

mod pins {
	use stm32f4xx_hal::gpio::{Output, PushPull, PA8, PA9, PB12, PB13, PB14, PB15};

	pub type Blue55 = PB12<Output<PushPull>>;
	pub type Blue70 = PB13<Output<PushPull>>;
	pub type Blue85 = PB14<Output<PushPull>>;

	pub type Hz1000 = PB15<Output<PushPull>>;
	pub type Hz500 = PA8<Output<PushPull>>;
	pub type Command = PA9<Output<PushPull>>;
}

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
	use defmt::warn;
	use defmt_rtt as _;
	use stm32f4xx_hal::{
		gpio::{self, Edge, Input, Output, PushPull},
		pac::TIM1,
		prelude::*,
		rtc::{Event, Rtc},
		timer,
	};

	// Resources shared between tasks
	#[shared]
	struct Shared {
		delayval: u32,
		rtc:      Rtc,
		leds:     Leds,
	}

	pub struct Leds {
		blue_55: crate::pins::Blue55,
		blue_70: crate::pins::Blue70,
		blue_85: crate::pins::Blue85,
		hz_1000: crate::pins::Hz1000,
		hz_500:  crate::pins::Hz500,
		command: crate::pins::Command,
	}

	// Local resources to specific tasks (cannot be shared)
	#[local]
	struct Local {
		button: gpio::PA0<Input>,
		led:    gpio::PC13<Output<PushPull>>,
		delay:  timer::DelayMs<TIM1>,
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

		// Configure RTC
		let mut rtc = Rtc::new(dp.RTC, &mut dp.PWR);

		// Set date and time
		let _ = rtc.set_year(2024);
		let _ = rtc.set_month(3);
		let _ = rtc.set_day(22);
		let _ = rtc.set_hours(22);
		let _ = rtc.set_minutes(57);
		let _ = rtc.set_seconds(00);

		// Start listening to WAKE UP INTERRUPTS
		rtc.enable_wakeup(10.secs());
		rtc.listen(&mut dp.EXTI, Event::Wakeup);

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

		let blue_55 = gpiob.pb12.into_push_pull_output();
		let blue_70 = gpiob.pb13.into_push_pull_output();
		let blue_85 = gpiob.pb14.into_push_pull_output();
		let hz_1000 = gpiob.pb15.into_push_pull_output();
		let hz_500 = gpioa.pa8.into_push_pull_output();
		let command = gpioa.pa9.into_push_pull_output();

		(
			// Initialization of shared resources
			Shared {
				delayval: 1000_u32,
				rtc,
				leds: Leds {
					blue_55,
					blue_70,
					blue_85,
					hz_1000,
					hz_500,
					command,
				},
			},
			// Initialization of task local resources
			Local { button, led, delay },
		)
	}

	// Background task, runs whenever no other tasks are running
	#[idle(local = [led, delay], shared = [delayval, leds])]
	fn idle(mut ctx: idle::Context) -> ! {
		let led = ctx.local.led;
		let delay = ctx.local.delay;
		let mut leds = ctx.shared.leds;
		loop {
			warn!("on cuh");
			// Turn On LED
			led.set_high();
			leds.lock(|v| {
				v.blue_55.set_high();
				v.blue_70.set_high();
				v.blue_85.set_high();
				v.hz_1000.set_high();
				v.hz_500.set_high();
				// v.command.set_high();
			});
			// Obtain shared delay variable and delay
			delay.delay_ms(ctx.shared.delayval.lock(|del| *del));
			// Turn off LED
			led.set_low();
			leds.lock(|v| {
				v.blue_55.set_low();
				v.blue_70.set_low();
				v.blue_85.set_low();
				v.hz_1000.set_low();
				v.hz_500.set_low();
				// v.command.set_low();
			});
			// Obtain shared delay variable and delay
			delay.delay_ms(ctx.shared.delayval.lock(|del| *del));
		}
	}

	#[task(binds = EXTI0, local = [button])]
	fn gpio_interrupt_handler(mut ctx: gpio_interrupt_handler::Context) {

		ctx.local.button.clear_interrupt_pending_bit();
	}

	#[task(binds = RTC_WKUP, shared = [rtc])]
	fn rtc_wakeup(mut ctx: rtc_wakeup::Context) {
		ctx.shared.rtc.lock(|rtc| {
			rtc.clear_interrupt(Event::Wakeup);
		});
		// Your RTC wakeup interrupt handling code here
	}
}
