use std::thread::JoinHandle;
use comlynx_runner_thread::ComlynxRunnerThread;
use holani::cartridge::lnx_header::LNXRotation;
use log::trace;
use perframe_runner_thread::PerFrameRunnerThread;
use runner_config::RunnerConfig;
use thread_priority::*;

pub(crate) mod runner_config;
pub(crate) mod comlynx_runner_thread;
pub(crate) mod perframe_runner_thread;

pub const CRYSTAL_FREQUENCY: u32 = 16_000_000;
pub const SAMPLE_RATE: u32 = 16_000;
pub const SAMPLE_TICKS: u32 = CRYSTAL_FREQUENCY / SAMPLE_RATE;

pub(crate) trait RunnerThread {
    fn initialize(&mut self) -> Result<(), &str>;
    fn run(&mut self);
}

pub(crate) struct Runner {
    runner_thread: Option<JoinHandle<()>>,
    config: RunnerConfig,
    input_tx: Option<kanal::Sender<u8>>,
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

    pub fn initialize_thread(&mut self) -> (kanal::Sender<(u8, u8)>, kanal::Receiver<Vec<u8>>, LNXRotation) {
        let (input_tx, input_rx) = kanal::unbounded::<(u8, u8)>();
        let (update_display_tx, update_display_rx) = kanal::unbounded::<Vec<u8>>();
        let (rotation_tx, rotation_rx) = kanal::unbounded::<LNXRotation>();

        let conf = self.config.clone();

        self.runner_thread = Some(
            std::thread::Builder::new()
            .name("Core".to_string())
            .spawn_with_priority(ThreadPriority::Max, move |_| {
                let mut thread: Box<dyn RunnerThread> = match conf.comlynx() {
                    true => Box::new(ComlynxRunnerThread::new(conf, input_rx, update_display_tx, rotation_tx)),
                    false => Box::new(PerFrameRunnerThread::new(conf, input_rx, update_display_tx, rotation_tx)),
                };
                trace!("Runner started.");
                thread.initialize().unwrap_or_else(|err| {
                    println!("Error: {}", err);
                    std::process::exit(1);
                });
                thread.run();
            })
            .expect("Could not create the main core runner thread.")
        );

        let rotation = rotation_rx.recv().unwrap();
       
        (input_tx, update_display_rx, rotation)
    }
}
