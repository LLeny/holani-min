use super::{RunnerConfig, RunnerThread, CRYSTAL_FREQUENCY, SAMPLE_TICKS};
use crate::{runner::SAMPLE_RATE, sound_source::SoundSource};
use holani::{cartridge::lnx_header::LNXRotation, lynx::Lynx};
use log::trace;
use ringbuf::{
    traits::{Producer, Split as _},
    HeapProd, HeapRb,
};
use rodio::{OutputStream, Sink};
use std::time::{Duration, Instant};

const TICK_GROUP: u32 = 8;
const TICK_LENGTH: Duration =
    Duration::from_nanos((1_000_000_000f32 / CRYSTAL_FREQUENCY as f32 * TICK_GROUP as f32) as u64);

pub(crate) struct ComlynxRunnerThread {
    lynx: Lynx,
    next_ticks_trigger: Instant,
    sound_tick: u32,
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
            sink: None,
            stream: None,
        }
    }

    fn sound(&mut self, prod: &mut HeapProd<i16>) {
        if self.config.mute() {
            return;
        }

        self.sound_tick += 1;
        if self.sound_tick < SAMPLE_TICKS {
            return;
        }

        self.sound_tick = 0;
        let (l, r) = self.lynx.audio_sample();
        prod.push_slice(&[l, r]);
    }

    fn display(&mut self) {
        if !self.lynx.redraw_requested() {
            return;
        }
        trace!("Display updated.");
        let screen = self.lynx.screen_rgba().clone();
        let _ = self.update_display_tx.try_send(screen);
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
        let sound_ringbuf = HeapRb::<i16>::new(SAMPLE_RATE as usize * 2); // 1 second buffer
        let (mut sound_buffer, sound_consumer) = sound_ringbuf.split();

        #[cfg(feature = "comlynx_external")]
        let (tcp_conn_tx, tcp_conn_rx) = kanal::unbounded::<TcpStream>();
        #[cfg(feature = "comlynx_external")]
        let port = self.config.comlynx_port();
        #[cfg(feature = "comlynx_external")]
        let _tcplistener = std::thread::Builder::new()
            .name("TCPListener".to_string())
            .spawn_with_priority(ThreadPriority::Min, move |_| {
                let bindto = format!("0.0.0.0:{}", port);
                let tcpsock = std::net::TcpListener::bind(bindto).unwrap();
                println!("Comlynx TCP server running at 0.0.0.0:{}", port);

                for stream in tcpsock.incoming() {
                    match stream {
                        Ok(stream) => {
                            stream.set_nonblocking(true).unwrap();
                            tcp_conn_tx.send(stream).unwrap();
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            })
            .expect("Could not create the TCP listener thread.");

        #[cfg(feature = "comlynx_external")]
        let mut buffer = [0; 128];
        #[cfg(feature = "comlynx_external")]
        let mut stream: Option<TcpStream> = None;

        if !self.config.mute() {
            let stream_handle = rodio::OutputStreamBuilder::open_default_stream()
                .expect("open default audio stream");
            self.stream = Some(stream_handle);
            let sink = rodio::Sink::connect_new(&self.stream.as_ref().unwrap().mixer());
            let sound_source = SoundSource::new(sound_consumer);
            sink.append(sound_source);
            self.sink = Some(sink);
        }

        loop {
            while Instant::now() < self.next_ticks_trigger {}

            self.next_ticks_trigger = Instant::now() + TICK_LENGTH;

            if self.inputs() {
                return;
            }

            for _ in 0..TICK_GROUP {
                self.lynx.tick();
                self.sound(&mut sound_buffer);
            }

            #[cfg(feature = "comlynx_external")]
            {
                if let Ok(Some(s)) = tcp_conn_rx.try_recv() {
                    stream = Some(s);
                    self.lynx.set_comlynx_cable_present(true);
                    println!("Comlynx client connected.");
                }

                match stream.as_ref().map(|mut s| s.read(&mut buffer)) {
                    Some(Err(_)) => (),
                    Some(Ok(0)) => {
                        let _ = stream.take();
                        self.lynx.set_comlynx_cable_present(false);
                        println!("Comlynx client disconnected.");
                    }
                    Some(Ok(len)) => {
                        for data in buffer.iter().take(len) {
                            self.lynx.comlynx_ext_rx(*data);
                        }
                    }
                    None => (),
                }

                if let Some(tx) = self.lynx.comlynx_ext_tx() {
                    let _ = stream.as_ref().map(|mut s| s.write_all(&[tx]));
                }
            }

            self.display();
        }
    }
}
