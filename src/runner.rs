use std::thread::JoinHandle;
use holani::cartridge::lnx_header::LNXRotation;
use log::trace;
use crate::{runner_config::RunnerConfig, runner_thread::RunnerThread, sound_source::SoundSource};
use rodio::{OutputStream, OutputStreamHandle, Sink};

pub(crate) struct Runner {
    runner_thread: Option<JoinHandle<()>>,
    config: RunnerConfig,
    input_tx: Option<kanal::Sender<u8>>,
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Option<Sink>,
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
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        Self {
            config,
            runner_thread: None,
            input_tx: None,
            _stream,
            stream_handle, 
            sink: None,
        }
    }

    pub fn initialize_thread(&mut self) -> (kanal::Sender<u8>, kanal::Receiver<Vec<u8>>, LNXRotation) {
        let (input_tx, input_rx) = kanal::unbounded::<u8>();
        let (update_display_tx, update_display_rx) = kanal::unbounded::<Vec<u8>>();
        let (rotation_tx, rotation_rx) = kanal::unbounded::<LNXRotation>();
        let (sample_req_tx, sample_req_rx) = kanal::unbounded::<()>();
        let (sample_rec_tx, sample_rec_rx) = kanal::unbounded::<(i16, i16)>();

        let conf = self.config.clone();

        self.runner_thread = Some(
            std::thread::Builder::new()
            .name("Core".to_string())
            .spawn(move || {
                let mut thread = RunnerThread::new(conf, input_rx, update_display_tx, rotation_tx, sample_req_rx, sample_rec_tx);
                trace!("Runner started.");
                thread.initialize();
                thread.run();
            })
            .expect("Could not create the main core runner thread.")
        );

        let rotation = rotation_rx.recv().unwrap();

        if !self.config.mute() {
            let sound_source = SoundSource::new(sample_req_tx, sample_rec_rx);
            let s = Sink::try_new(&self.stream_handle).unwrap();
            s.append(sound_source);
            self.sink = Some(s);
        }
        
        (input_tx, update_display_rx, rotation)
    }
}
