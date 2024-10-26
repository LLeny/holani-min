use std::{num::NonZeroU32, thread::{self, JoinHandle}, time::Duration};
use governor::{Quota, RateLimiter};
use holani::{cartridge::lnx_header::LNXRotation, Lynx};
use log::trace;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use crate::runner_config::RunnerConfig;

const CRYSTAL_FREQUENCY: u32 = 16_000_000;
const SAMPLE_RATE: u32 = 16000;
const SAMPLE_TICKS: u32 = CRYSTAL_FREQUENCY / SAMPLE_RATE;
const SAMPLE_BUFFER_SIZE: u32 = SAMPLE_RATE / 6;
const SLEEP: Duration = Duration::from_nanos(20);

pub(crate) struct Runner {
    runner_thread: Option<JoinHandle<()>>,
    config: RunnerConfig,
    input_tx: Option<kanal::Sender<u8>>
}

impl Drop for Runner {
    fn drop(&mut self) {
        if let Some(tx) = self.input_tx.take() {
            tx.close().unwrap();
            if let Some(handle) = self.runner_thread.take() {
                handle.join().unwrap();
            }
        }
    }
}

impl Runner {

    pub fn new(config: RunnerConfig) -> Self {
        Self {
            config,
            runner_thread: None,
            input_tx: None,
        }
    }

    pub fn initialize_thread(&mut self) -> (kanal::Sender<u8>, kanal::Receiver<Vec<u8>>, LNXRotation) {
        let (input_tx, input_rx) = kanal::unbounded::<u8>();
        let (update_display_tx, update_display_rx) = kanal::unbounded::<Vec<u8>>();
        let (rotation_tx, rotation_rx) = kanal::unbounded::<LNXRotation>();
        let conf = self.config.clone();

        self.runner_thread = Some(
            std::thread::Builder::new()
            .name("Core".to_string())
            .spawn(move || {
                trace!("Runner started.");
                let lim = RateLimiter::direct(Quota::per_second(NonZeroU32::new(CRYSTAL_FREQUENCY).unwrap()));
                let mut lynx: Lynx = Lynx::new();

                if let Some(rom) = conf.rom() {
                    if lynx.load_rom_from_slice(&std::fs::read(rom).unwrap()).is_err() {
                        panic!("Couldn't not load ROM file.");
                    }
                    trace!("ROM loaded.");
                }

                match conf.cartridge() {
                    None => panic!("A cartridge is required."),
                    Some(cart) => if lynx.load_cart_from_slice(&std::fs::read(cart).unwrap()).is_err() {
                        panic!("Couldn't not load Cartridge file.");
                    }
                }

                trace!("Cart loaded.");
                rotation_tx.send(lynx.rotation()).unwrap();

                let mut sound_tick = 0;
                let mut sound_buffer: Vec<i16> = vec![];
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                let sink = Sink::try_new(&stream_handle).unwrap();

                loop {
                    while lim.check().is_err() {
                        thread::sleep(SLEEP);
                    }

                    if input_rx.is_disconnected() {
                        return;
                    } else if let Ok(Some(joy)) = input_rx.try_recv() {
                        lynx.set_joystick_u8(joy);
                    }

                    lynx.tick();

                    if !conf.mute() {
                        sound_tick += 1;
                        if sound_tick == SAMPLE_TICKS {        
                            sound_tick = 0;
                            let (l, r) = lynx.audio_sample();
                            sound_buffer.push(l);
                            sound_buffer.push(r);
                            if sound_buffer.len() > SAMPLE_BUFFER_SIZE as usize {
                                sink.append(SamplesBuffer::new(2, SAMPLE_RATE, sound_buffer.clone()));
                                sound_buffer.clear();
                            }
                        }
                    }

                    if lynx.redraw_requested() {
                        trace!("Display updated.");
                        let screen = lynx.screen_rgb().clone();
                        if update_display_tx.try_send(screen).is_ok() {  }
                    }
                }
            })
            .expect("Could not create the main core runner thread.")
        );

        let rotation = rotation_rx.recv().unwrap();

        (input_tx, update_display_rx, rotation)
    }
}
