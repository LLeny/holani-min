use ringbuf::{traits::Consumer, HeapCons};
use rodio::Source;

use crate::runner::SAMPLE_RATE;
const CHANNELS: u16 = 2;

pub(crate) struct SoundSource {
    sample_buffer: HeapCons<i16>,
}

impl SoundSource {
    pub(crate) fn new(sample_buffer: HeapCons<i16>) -> Self {
        Self { sample_buffer }
    }
}

impl Iterator for SoundSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.sample_buffer
            .try_pop()
            .map(|s| dasp_sample::conv::i16::to_f32(s))
            .or(Some(0.))
    }
}

impl Source for SoundSource {
    fn channels(&self) -> u16 {
        CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }

    fn current_span_len(&self) -> Option<usize> {
        None
    }
}
