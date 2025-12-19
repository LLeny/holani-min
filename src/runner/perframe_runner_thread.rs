use holani::{cartridge::lnx_header::LNXRotation, lynx::Lynx};
use log::trace;
use ringbuf::{
    traits::{Producer, Split as _},
    HeapProd, HeapRb,
};
use rodio::OutputStream;
use std::time::{Duration, Instant};

use crate::sound_source::SoundSource;

use super::{RunnerConfig, RunnerThread, CRYSTAL_FREQUENCY, SAMPLE_RATE};
const TICKS_PER_AUDIO_SAMPLE: u64 = CRYSTAL_FREQUENCY as u64 / SAMPLE_RATE as u64;
const SAMPLE_BUFFER_SIZE: usize = 2048;

pub(crate) struct PerFrameRunnerThread {
    lynx: Lynx,
    sound_tick: u64,
    config: RunnerConfig,
    input_rx: kanal::Receiver<(u8, u8)>,
    update_display_tx: kanal::Sender<Vec<u8>>,
    rotation_tx: kanal::Sender<LNXRotation>,
    frame_time: Duration,
    next_lcd_refresh: Instant,
    last_refresh_rate: f64,
    stream: Option<OutputStream>,
}

impl PerFrameRunnerThread {
    pub(crate) fn new(
        config: RunnerConfig,
        input_rx: kanal::Receiver<(u8, u8)>,
        update_display_tx: kanal::Sender<Vec<u8>>,
        rotation_tx: kanal::Sender<LNXRotation>,
    ) -> Self {
        Self {
            lynx: Lynx::new(),
            config,
            input_rx,
            update_display_tx,
            rotation_tx,
            sound_tick: 0,
            frame_time: Duration::from_millis(16),
            last_refresh_rate: 0f64,
            next_lcd_refresh: Instant::now(),
            stream: None,
        }
    }

    fn sound(&mut self, sound_buffer: &mut HeapProd<i16>) {
        if self.config.mute() {
            return;
        }

        self.sound_tick += 1;

        if self.sound_tick != TICKS_PER_AUDIO_SAMPLE {
            return;
        }

        self.sound_tick = 0;
        let (l, r) = self.lynx.audio_sample();
        sound_buffer.push_slice(&[l, r]);
    }

    fn display(&mut self) {
        trace!("Display updated.");
        let screen = self.lynx.screen_rgba().clone();
        let _ = self.update_display_tx.try_send(screen).is_ok();
    }

    fn inputs(&mut self) -> bool {
        if self.input_rx.is_disconnected() {
            return true;
        } else if let Ok(Some((joy, sw))) = self.input_rx.try_recv() {
            self.lynx.set_joystick_u8(joy);
            self.lynx.set_switches_u8(sw);
        }
        false
    }
}

impl RunnerThread for PerFrameRunnerThread {
    fn initialize(&mut self) -> Result<(), &str> {
        if let Some(rom) = self.config.rom() {
            let data = std::fs::read(rom);
            if data.is_err() {
                return Err("Couldn't not load ROM file.");
            }
            if self.lynx.load_rom_from_slice(&data.unwrap()).is_err() {
                return Err("Couldn't not load ROM file.");
            }
            trace!("ROM loaded.");
        }

        match self.config.cartridge() {
            None => panic!("A cartridge is required."),
            Some(cart) => {
                let data = std::fs::read(cart);
                if data.is_err() {
                    return Err("Couldn't not load Cartridge file.");
                }
                if self.lynx.load_cart_from_slice(&data.unwrap()).is_err() {
                    return Err("Couldn't not load Cartridge file.");
                }
                trace!("ROM loaded.");
            }
        }

        trace!("Cart loaded.");
        self.rotation_tx.send(self.lynx.rotation()).unwrap();

        Ok(())
    }

    fn run(&mut self) {
        let mut rf: f64;

        let sound_ringbuf = HeapRb::<i16>::new(SAMPLE_BUFFER_SIZE * 2);
        let (mut sound_buffer, sound_consumer) = sound_ringbuf.split();

        if !self.config.mute() {
            let stream_handle = rodio::OutputStreamBuilder::from_default_device()
                .expect("open default audio device")
                .with_buffer_size(rodio::cpal::BufferSize::Fixed(SAMPLE_BUFFER_SIZE as u32))
                .open_stream()
                .expect("open audio stream");

            let source = SoundSource::new(sound_consumer);
            stream_handle.mixer().add(source);
            self.stream = Some(stream_handle);
        }

        loop {
            if self.inputs() {
                return;
            }

            while !self.lynx.redraw_requested() {
                self.lynx.tick();
                self.sound(&mut sound_buffer);
            }

            rf = self.lynx.display_refresh_rate();
            if rf != self.last_refresh_rate {
                self.last_refresh_rate = rf;
                self.frame_time =
                    Duration::from_micros((1000000f64 / self.last_refresh_rate) as u64);
                trace!("set refresh rate to {} ({:?})", rf, self.frame_time);
            }
            self.display();

            while self.next_lcd_refresh > Instant::now() {}
            self.next_lcd_refresh = Instant::now() + self.frame_time;
        }
    }
}
