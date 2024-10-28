use rodio::Source;

const CHANNELS: u16 = 2;
const SAMPLE_RATE: u32 = 11_025;

pub(crate) struct SoundSource {
    sample: Vec<i16>,
    sample_req_tx: kanal::Sender<()>,
    sample_rec_rx: kanal::Receiver<(i16, i16)>,
}

impl SoundSource {
    pub(crate) fn new(sample_req_tx: kanal::Sender<()>, sample_rec_rx: kanal::Receiver<(i16, i16)>) -> Self {
        Self { 
            sample: vec![], 
            sample_req_tx, 
            sample_rec_rx 
        }
    }
}

impl Iterator for SoundSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.sample.is_empty() {
            self.sample_req_tx.send(()).unwrap();
            if let Ok((l, r)) = self.sample_rec_rx.recv() {
                self.sample.push(l);
                self.sample.push(r);
            }
        }
        self.sample.pop()
    }
}

impl Source for SoundSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}