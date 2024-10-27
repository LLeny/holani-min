use std::thread::JoinHandle;
use holani::cartridge::lnx_header::LNXRotation;
use log::trace;
use crate::{runner_config::RunnerConfig, runner_thread::RunnerThread};



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
                let mut thread = RunnerThread::new(conf, input_rx, update_display_tx, rotation_tx);
                trace!("Runner started.");
                thread.initialize();
                thread.run();
            })
            .expect("Could not create the main core runner thread.")
        );

        let rotation = rotation_rx.recv().unwrap();
        (input_tx, update_display_rx, rotation)
    }
}
