use super::voice::Voice;

pub struct Synth {
    sample_rate: u32,
    voice: Voice
}

impl Synth {
    pub fn new(sample_rate: u32) -> Self {
        //let voices = [Voice::new(sample_rate); 12];
        let voice = Voice::new(sample_rate);
        Synth{sample_rate, voice}
    }

    pub fn get_sample(&mut self, sample_clock: u64) -> f32 {
        self.voice.get_sample(sample_clock)
    }
}
