use std::time::{Duration, Instant};
use holani::{cartridge::lnx_header::LNXRotation, Lynx};
use log::trace;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};

use crate::runner_config::RunnerConfig;

const CRYSTAL_FREQUENCY: u32 = 16_000_000;
const SAMPLE_RATE: u32 = 16_000;
const SAMPLE_TICKS: u32 = CRYSTAL_FREQUENCY / SAMPLE_RATE;
const SAMPLE_BUFFER_SIZE: u32 = SAMPLE_RATE / 32;
const TICK_GROUP: u32 = 16;
const TICK_LENGTH: Duration = Duration::from_nanos((1_000_000_000f32 / CRYSTAL_FREQUENCY as f32 * TICK_GROUP as f32) as u64);

pub(crate) struct RunnerThread {
    lynx: Lynx,
    next_ticks_trigger: Instant,
    sound_tick: u32,
    sound_buffer: Vec<i16>,
    _stream: OutputStream,
    sink: Sink,
    config: RunnerConfig,
    input_rx: kanal::Receiver<u8>,
    update_display_tx: kanal::Sender<Vec<u8>>,
    rotation_tx: kanal::Sender<LNXRotation>,    
}

impl RunnerThread {
    pub(crate) fn new(config: RunnerConfig, input_rx: kanal::Receiver<u8>, update_display_tx: kanal::Sender<Vec<u8>>, rotation_tx: kanal::Sender<LNXRotation>) -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        Self {
            lynx: Lynx::new(),
            next_ticks_trigger: Instant::now(),
            sound_tick: 0,
            sound_buffer: vec![],
            _stream: stream,
            sink: Sink::try_new(&stream_handle).unwrap(),  
            config,
            input_rx,
            update_display_tx,
            rotation_tx,
        }
    }

    fn sound(&mut self) {
        if self.config.mute() {
            return;
        }

        self.sound_tick += 1;
        if self.sound_tick < SAMPLE_TICKS {
            return;
        }        

        self.sound_tick = 0;
        let (l, r) = self.lynx.audio_sample();
        self.sound_buffer.push(l);
        self.sound_buffer.push(r);

        if self.sound_buffer.len() < SAMPLE_BUFFER_SIZE as usize {
            return;
        }

        self.sink.append(SamplesBuffer::new(2, SAMPLE_RATE, self.sound_buffer.clone()));
        self.sound_buffer.clear();
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