use std::{collections::VecDeque, io::{Read, Write}, time::{Duration, Instant}};
use holani::{cartridge::lnx_header::LNXRotation, lynx::Lynx};
use log::trace;
use rodio::{OutputStream, Sink};
use crate::sound_source::SoundSource;

use super::{RunnerConfig, RunnerThread, CRYSTAL_FREQUENCY, SAMPLE_TICKS};

const TICK_GROUP: u32 = 16;
const TICK_LENGTH: Duration = Duration::from_nanos((1_000_000_000f32 / CRYSTAL_FREQUENCY as f32 * TICK_GROUP as f32) as u64);

pub(crate) struct ComlynxRunnerThread {
    lynx: Lynx,
    next_ticks_trigger: Instant,
    sound_tick: u32,
    sound_sample: VecDeque<(i16, i16)>,
    sample_ticks: u32,
    config: RunnerConfig,
    input_rx: kanal::Receiver<(u8, u8)>,
    update_display_tx: kanal::Sender<Vec<u8>>,
    rotation_tx: kanal::Sender<LNXRotation>,
    sink: Option<Sink>,
    stream: Option<OutputStream>,
}

impl ComlynxRunnerThread {
    pub(crate) fn new(
        config: RunnerConfig, 
        input_rx: kanal::Receiver<(u8, u8)>, 
        update_display_tx: kanal::Sender<Vec<u8>>, 
        rotation_tx: kanal::Sender<LNXRotation>,
    ) -> Self {
        Self {
            lynx: Lynx::new(),
            next_ticks_trigger: Instant::now(),
            config,
            input_rx,
            update_display_tx,
            rotation_tx,
            sound_tick: 0,
            sound_sample: VecDeque::new(),
            sample_ticks: SAMPLE_TICKS,
            sink: None,
            stream: None,            
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
        } else if let Ok(Some((joy, sw))) = self.input_rx.try_recv() {
            self.lynx.set_joystick_u8(joy);
            self.lynx.set_switches_u8(sw);
        }
        false
    }   
}

impl RunnerThread for ComlynxRunnerThread {
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

        let (sample_req_tx, sample_req_rx) = kanal::unbounded::<()>();
        let (sample_rec_tx, sample_rec_rx) = kanal::unbounded::<(i16, i16)>();

        let bindto = format!("0.0.0.0:{}", self.config.comlynx_port());
        let tcpsock = std::net::TcpListener::bind(bindto).unwrap();
        println!("Comlynx TCP server running at 0.0.0.0:{}", self.config.comlynx_port());

        let (mut stream, _) = tcpsock.accept().unwrap();
        stream.set_nonblocking(true).unwrap();
        println!("Comlynx client connected.");

        let mut buffer = [0; 64];

        if !self.config.mute() {
            let (stream, stream_handle) = OutputStream::try_default().unwrap();
            self.stream = Some(stream);
            let sink = Sink::try_new(&stream_handle).unwrap();
            let sound_source = SoundSource::new(sample_req_tx, sample_rec_rx);
            sink.append(sound_source);
            self.sink = Some(sink);
        }

        loop {
            while Instant::now() < self.next_ticks_trigger {
                if let Ok(Some(())) = sample_req_rx.try_recv() {
                    sample_rec_tx.send(self.sound_sample.pop_front().unwrap_or((0, 0))).unwrap();
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

            if let Ok(len) = stream.read(&mut buffer) {
                if len > 0 {
                    // println!("Received {:02X}", buffer[0]);
                    self.lynx.comlynx_ext_rx(buffer[0]);
                }                
            }

            if let Some(tx) = self.lynx.comlynx_ext_tx() {
                // println!("Sending {:02X}", rx);
                let _ = stream.write_all(&[tx]);
            }

            self.display();
        }
    }
}
