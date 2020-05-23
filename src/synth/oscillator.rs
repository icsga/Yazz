use super::Float;
use super::{WtOsc, WtOscData};
use wavetable::WavetableRef;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum OscType {
    Wavetable,
    Noise
}

impl OscType {
    pub fn from_int(param: usize) -> OscType {
        match param {
            0 => OscType::Wavetable,
            1 => OscType::Noise,
            _ => panic!(),
        }
    }

    pub fn to_int(&self) -> usize {
        match self {
            OscType::Wavetable => 0,
            OscType::Noise => 1,
        }
    }
}

impl Default for OscType {
    fn default() -> Self {
        OscType::Wavetable
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct OscData {
    pub level: Float,
    pub tune_halfsteps: i64,
    pub tune_cents: Float,
    pub freq_offset: Float, // Value derived from tune_halfsteps and tune_cents
    pub sync: i64,
    pub key_follow: i64,
    pub osc_type: OscType,

    // Oscillator-specific data
    pub wt_osc_data: WtOscData,
}

impl OscData {
    pub fn init(&mut self) {
        self.level = 0.5;
        self.set_halfsteps(0);
        self.set_cents(0.0);
        self.sync = 0;
        self.key_follow = 1;
        self.wt_osc_data.init();
    }

    /** Coarse tuning of oscillator (+/- 2 octaves). */
    pub fn set_halfsteps(&mut self, halfsteps: i64) {
        self.tune_halfsteps = halfsteps;
        self.calc_freq_offset();
    }

    /** Fine tuning of oscillator (+/- 1 halfsteps). */
    pub fn set_cents(&mut self, cents: Float) {
        self.tune_cents = cents;
        self.calc_freq_offset();
    }

    /** Calculate resulting frequence of tuning settings. */
    fn calc_freq_offset(&mut self) {
        let inc: Float = 1.059463;
        self.freq_offset = inc.powf(self.tune_halfsteps as Float + self.tune_cents);
    }
}

pub struct Oscillator {
    last_update: i64,
    last_sample: Float,
    last_complete: bool,

    wt_osc: WtOsc,
}

impl Oscillator {
    pub fn new(sample_rate: u32, default_wt: WavetableRef) -> Self {
        Oscillator{
            last_update: 0,
            last_sample: 0.0,
            last_complete: false,
            wt_osc: WtOsc::new(sample_rate, default_wt)
        }
    }

    pub fn get_sample(&mut self, frequency: Float, sample_clock: i64, data: &OscData, reset: bool) -> (Float, bool) {
        if reset {
            self.reset(sample_clock - 1);
        }

        // Check if we already calculated a matching value
        // TODO: Check if we also need to test the frequency here.
        if sample_clock == self.last_update {
            return (self.last_sample, self.last_complete);
        }

        let dt = sample_clock - self.last_update;
        let (result, complete) = match data.osc_type {
            OscType::Wavetable => self.wt_osc.get_sample(frequency, dt, &data.wt_osc_data),
            OscType::Noise => (Oscillator::get_sample_noise(), false),
        };

        self.last_update += dt;
        self.last_sample = result;
        self.last_complete = complete;
        (result, complete)
    }

    pub fn reset(&mut self, sample_clock: i64) {
        self.wt_osc.reset();
        self.last_update = sample_clock;
    }

    fn get_sample_noise() -> Float {
        (rand::random::<Float>() * 2.0) - 1.0
    }

    pub fn set_wavetable(&mut self, wavetable: WavetableRef) {
        self.wt_osc.set_wavetable(wavetable);
    }
}

