use std::{collections::VecDeque, time::{Duration, Instant}};
use holani::{cartridge::lnx_header::LNXRotation, Lynx};
use log::trace;

use crate::runner_config::RunnerConfig;

const CRYSTAL_FREQUENCY: u32 = 16_000_000;
const SAMPLE_RATE: u32 = 16_000;
const SAMPLE_TICKS: u32 = CRYSTAL_FREQUENCY / SAMPLE_RATE;
const TICK_GROUP: u32 = 8;
const TICK_LENGTH: Duration = Duration::from_nanos((1_000_000_000f32 / CRYSTAL_FREQUENCY as f32 * TICK_GROUP as f32) as u64);

pub(crate) struct RunnerThread {
    lynx: Lynx,
    next_ticks_trigger: Instant,
    sound_tick: u32,
    sound_sample: VecDeque<(i16, i16)>,
    sample_ticks: u32,
    config: RunnerConfig,
    input_rx: kanal::Receiver<u8>,
    update_display_tx: kanal::Sender<Vec<u8>>,
    rotation_tx: kanal::Sender<LNXRotation>,
    sample_req_rx: kanal::Receiver<()>, 
    sample_rec_tx: kanal::Sender<(i16, i16)>,    
}

impl RunnerThread {
    pub(crate) fn new(
        config: RunnerConfig, 
        input_rx: kanal::Receiver<u8>, 
        update_display_tx: kanal::Sender<Vec<u8>>, 
        rotation_tx: kanal::Sender<LNXRotation>,
        sample_req_rx: kanal::Receiver<()>, 
        sample_rec_tx: kanal::Sender<(i16, i16)>,
    ) -> Self {
        Self {
            lynx: Lynx::new(),
            next_ticks_trigger: Instant::now(),
            config,
            input_rx,
            update_display_tx,
            rotation_tx,
            sample_rec_tx,
            sample_req_rx,
            sound_tick: 0,
            sound_sample: VecDeque::new(),
            sample_ticks: SAMPLE_TICKS,
        }
    }

    fn sound(&mut self) {
        if self.config.mute() {
            return;
        }

        self.sound_tick += 1;
        if self.sound_tick < self.sample_ticks {
            return;
        }

        if self.sound_sample.len() > 1100 {            
            self.sample_ticks += 2;
        }

        self.sound_tick = 0;
        self.sound_sample.push_back(self.lynx.audio_sample());
    }

    fn display(&mut self) {
        if !self.lynx.redraw_requested() {
            return;
        }
        trace!("Display updated.");
        let screen = self.lynx.screen_rgb().clone();
        let _ = self.update_display_tx.try_send(screen).is_ok();
    }

    fn inputs(&mut self) -> bool {
        if self.input_rx.is_disconnected() {
            return true;
        } else if let Ok(Some(joy)) = self.input_rx.try_recv() {
            self.lynx.set_joystick_u8(joy);
        }
        false
    }

    pub(crate) fn initialize(&mut self) {
        if let Some(rom) = self.config.rom() {
            if self.lynx.load_rom_from_slice(&std::fs::read(rom).unwrap()).is_err() {
                panic!("Couldn't not load ROM file.");
            }
            trace!("ROM loaded.");
        }

        match self.config.cartridge() {
            None => panic!("A cartridge is required."),
            Some(cart) => if self.lynx.load_cart_from_slice(&std::fs::read(cart).unwrap()).is_err() {
                panic!("Couldn't not load Cartridge file.");
            }
        }

        trace!("Cart loaded.");
        self.rotation_tx.send(self.lynx.rotation()).unwrap();
    }

    pub(crate) fn run(&mut self) {
        loop {
            while Instant::now() < self.next_ticks_trigger {
                if let Ok(Some(())) = self.sample_req_rx.try_recv() {
                    self.sample_rec_tx.send(match self.sound_sample.pop_front() {
                        None => {
                            self.sample_ticks -= 2;
                            (0, 0)
                        }
                        Some(v) => v
                    }).unwrap();
                }
            }
            self.next_ticks_trigger = Instant::now() + TICK_LENGTH;
            if self.inputs() {
                return;
            }
            for _ in 0..TICK_GROUP {
                self.lynx.tick();
                self.sound();
            }
            self.display();
        }
    }
}