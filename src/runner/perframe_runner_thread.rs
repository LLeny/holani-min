use std::time::{Duration, Instant};
use holani::{cartridge::lnx_header::LNXRotation, Lynx};
use log::trace;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};

use super::{RunnerConfig, RunnerThread, CRYSTAL_FREQUENCY, SAMPLE_RATE};
const TICKS_PER_AUDIO_SAMPLE: u64 = CRYSTAL_FREQUENCY as u64 / SAMPLE_RATE as u64;

pub(crate) struct PerFrameRunnerThread {
    lynx: Lynx,
    sound_tick: u64,
    sound_sample: Vec<i16>,
    config: RunnerConfig,
    input_rx: kanal::Receiver<(u8, u8)>,
    update_display_tx: kanal::Sender<Vec<u8>>,
    rotation_tx: kanal::Sender<LNXRotation>,
    frame_time: Duration,
    next_lcd_refresh: Instant,
    last_refresh_rate: f64,
    sink: Option<Sink>,
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
            sound_sample: vec![],
            frame_time: Duration::from_millis(16),
            last_refresh_rate: 0f64,
            next_lcd_refresh: Instant::now(),
            sink: None,
            stream: None,
        }
    }

    fn sound(&mut self) {
        if self.config.mute() {
            return;
        }

        self.sound_tick += 1;

        if self.sound_tick != TICKS_PER_AUDIO_SAMPLE {
            return;
        }

        self.sound_tick = 0;
        let (l, r) = self.lynx.audio_sample();
        self.sound_sample.push(l);
        self.sound_sample.push(r);        
    }

    fn display(&mut self) {
        trace!("Display updated.");
        let screen = self.lynx.screen_rgb().clone();
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
    fn initialize(&mut self) {
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

    fn run(&mut self) {

        let mut rf: f64;

        if !self.config.mute() {
            let (stream, stream_handle) = OutputStream::try_default().unwrap();
            self.stream = Some(stream);
            self.sink = Some(Sink::try_new(&stream_handle).unwrap());
        }

        loop {
            if self.inputs() {
                return;
            }

            while !self.lynx.redraw_requested() {
                self.lynx.tick();
                self.sound();
            }

            if !self.sound_sample.is_empty() {
               self.sink.as_mut().unwrap().append(SamplesBuffer::new(2,SAMPLE_RATE, self.sound_sample.clone()));
               self.sound_sample.clear();
            }

            rf = self.lynx.display_refresh_rate();
            if rf != self.last_refresh_rate {                
                self.last_refresh_rate = rf;
                self.frame_time = Duration::from_micros((1000000f64 / self.last_refresh_rate) as u64);
                trace!("set refresh rate to {} ({:?})", rf, self.frame_time);
            } 
            self.display();

            while self.next_lcd_refresh > Instant::now() {}
            self.next_lcd_refresh = Instant::now() + self.frame_time;
        }
    }
}