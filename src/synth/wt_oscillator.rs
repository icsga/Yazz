use super::Float;
use super::SampleGenerator;
use super::sound::SoundData;
use super::{Wavetable, WtManager};

use rand::prelude::*;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use log::{info, trace, warn};

const MAX_VOICES: usize = 7;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
pub struct WtOscData {
    pub level: Float,
    pub num_voices: i64,
    pub voice_spread: Float,
    pub tune_halfsteps: i64,
    pub tune_cents: Float,
    pub freq_offset: Float, // Value derived from tune_halfsteps and tune_cents
    pub wave_index: Float, // Index into the wave tables
    pub sync: i64,
    pub key_follow: i64,
}

impl WtOscData {
    pub fn init(&mut self) {
        self.level = 0.92;
        self.set_voice_num(1);
        self.set_halfsteps(0);
        self.wave_index = 0.0;
        self.sync = 0;
        self.key_follow = 1;
    }

    /** Number of detuned voices per oscillator. */
    pub fn set_voice_num(&mut self, voices: i64) {
        self.num_voices = if voices > MAX_VOICES as i64 { MAX_VOICES as i64 } else { voices };
    }

    /** Detune amount per voice. */
    pub fn set_voice_spread(&mut self, spread: Float) {
        self.voice_spread = spread;
    }

    /** Coarse tuning of oscillator (+/- 2 octaves). */
    pub fn set_halfsteps(&mut self, halfsteps: i64) {
        self.tune_halfsteps = halfsteps;
        self.calc_freq_offset();
    }

    /** Fine tuning of oscillator (0 - 1 octave). */
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

const NUM_SAMPLES_PER_TABLE: usize = 2048;
const NUM_VALUES_PER_TABLE: usize = (NUM_SAMPLES_PER_TABLE + 1); // Add one sample for easier interpolation on last sample

#[derive(Copy, Clone)]
struct State {
    last_pos: Float,
    freq_shift: Float, // Percentage this voice is shifted from center frequency
    level_shift: Float, // Decrease in level compared to main voice (TODO)
}

pub struct WtOsc {
    sample_rate: Float,
    pub id: usize,
    last_update: i64, // Time of last sample generation
    last_sample: Float,
    last_complete: bool,
    state: [State; MAX_VOICES], // State for up to MAX_VOICES oscillators running in sync
    wt_manager: Arc<WtManager>,
    wave: Arc<Wavetable>,
}

/** Wavetable oscillator implementation.
 *
 * The WT oscillator uses multiple tables per waveform to avoid aliasing. Each
 * table is filled by adding all harmonics that will not exceed the Nyquist
 * frequency for the given usable range of the table (one octave).
 */
impl WtOsc {

    /** Create a new wavetable oscillator.
     *
     * \param sample_rate The global sample rate of the synth
     * \param id The voice ID of the oscillator (0 - 2)
     */
    pub fn new(sample_rate: u32, id: usize, wt_manager: Arc<WtManager>) -> WtOsc {
        let sample_rate = sample_rate as Float;
        let last_update = 0;
        let last_sample = 0.0;
        let last_complete = false;
        let last_pos = 0.0;
        let freq_shift = 0.0;
        let level_shift = 1.0;
        let state = [State{last_pos, freq_shift, level_shift}; MAX_VOICES];
        let wave = wt_manager.get_table("default").unwrap();
        WtOsc{sample_rate,
              id,
              last_update,
              last_sample,
              last_complete,
              state,
              wt_manager,
              wave}
    }

    /** Interpolate between two sample values with the given ratio. */
    fn interpolate(val_a: Float, val_b: Float, ratio: Float) -> Float {
        val_a + ((val_b - val_a) * ratio)
    }

    /** Get a sample from the given table at the given position.
     *
     * Uses linear interpolation for positions that don't map directly to a
     * table index.
     */
    fn get_sample(table: &[Float], table_index: usize, position: Float) -> Float {
        let floor_pos = position as usize;
        let frac = position - floor_pos as Float;
        let position = floor_pos + table_index * NUM_VALUES_PER_TABLE;
        if frac > 0.9 {
            // Close to upper sample
            table[position + 1]
        } else if frac < 0.1 {
            // Close to lower sample
            table[position]
        } else {
            // Interpolate for differences > 10%
            let value_left = table[position];
            let value_right = table[position + 1];
            WtOsc::interpolate(value_left, value_right, frac)
        }
    }

    fn get_sample_noise() -> Float {
        (rand::random::<Float>() * 2.0) - 1.0
    }

    /* Look up the octave table matching the current frequency. */
    fn get_table_index(num_octaves: usize, freq: Float) -> usize {
        let two: Float = 2.0;
        let mut compare_freq = (440.0 / 32.0) * (two.powf((-9.0) / 12.0));
        let i: usize = 0;
        for i in 0..num_octaves {
            if freq < compare_freq * 2.0 {
                return i;
            }
            compare_freq *= 2.0;
        }
        0
    }
}

impl SampleGenerator for WtOsc {
    fn get_sample(&mut self, frequency: Float, sample_clock: i64, data: &SoundData, reset: bool) -> (Float, bool) {
        if reset {
            self.reset(sample_clock - 1);
        }

        // Check if we already calculated a matching value
        // TODO: Check if we also need to test the frequency here
        if sample_clock == self.last_update {
            return (self.last_sample, self.last_complete);
        }

        let data = data.get_osc_data(self.id);
        let dt = sample_clock - self.last_update;
        let dt_f = dt as Float;
        let mut result = 0.0;
        let mut complete = false;

        for i in 0..data.num_voices {
            let state: &mut State = &mut self.state[i as usize];
            let freq_diff = (frequency / 100.0) * (data.voice_spread * i as Float) * (1 - (i & 0x01 * 2)) as Float;
            let frequency = frequency + freq_diff;
            let freq_speed = frequency * (NUM_SAMPLES_PER_TABLE as Float / self.sample_rate);
            let diff = freq_speed * dt_f;
            state.last_pos += diff;
            if state.last_pos > (NUM_SAMPLES_PER_TABLE as Float) {
                // Completed one wave cycle
                state.last_pos -= NUM_SAMPLES_PER_TABLE as Float;
                complete = true; // Sync signal for other oscillators
            }

            let lower_wave = data.wave_index as usize;
            let lower_wave_float = lower_wave as Float;
            let lower_fract: Float = 1.0 - (data.wave_index - lower_wave_float);
            let upper_fract: Float = if lower_fract != 1.0 { 1.0 - lower_fract } else { 0.0 };

            let table: &Vec<Float>;
            let table_index = WtOsc::get_table_index(self.wave.num_octaves, frequency);

            let mut voice_result = WtOsc::get_sample(&self.wave.table[lower_wave], table_index, state.last_pos) * lower_fract;
            if upper_fract > 0.0 {
                voice_result += WtOsc::get_sample(&self.wave.table[lower_wave + 1], table_index, state.last_pos) * upper_fract;
            }
            result += voice_result;
        }
        self.last_update += dt;
        if result > 1.0 {
            result = 1.0;
        }
        self.last_sample = result;
        self.last_complete = complete;
        (result, complete)
    }

    fn reset(&mut self, sample_clock: i64) {
        for state in self.state.iter_mut() {
            state.last_pos = 0.0;
        }
        self.last_update = sample_clock;
    }
}

/*
#[cfg(test)]
#[test]
fn test_calc_num_harmonics() {
    // Base frequency: 2 Hz
    // Sample frequency 20 Hz
    // Nyquist: 10 Hz
    // Num harmonics: [2,] 4, 6, 8 = 3
    assert_eq!(WtOsc::calc_num_harmonics(2.0, 20.0), 3);
}

#[test]
fn test_get_table_index() {
    assert_eq!(WtOsc::get_table_index(10.0), 0);
    assert_eq!(WtOsc::get_table_index(20.0), 1);
    assert_eq!(WtOsc::get_table_index(40.0), 2);
    assert_eq!(WtOsc::get_table_index(80.0), 3);
    assert_eq!(WtOsc::get_table_index(160.0), 4);
    assert_eq!(WtOsc::get_table_index(320.0), 5);
    assert_eq!(WtOsc::get_table_index(640.0), 6);
    assert_eq!(WtOsc::get_table_index(1280.0), 7);
    assert_eq!(WtOsc::get_table_index(2560.0), 8);
    assert_eq!(WtOsc::get_table_index(5120.0), 9);
    assert_eq!(WtOsc::get_table_index(10240.0), 10);
    assert_eq!(WtOsc::get_table_index(20480.0), 10);
}

#[test]
fn test_interpolate() {
    assert_eq!(WtOsc::interpolate(2.0, 3.0, 0.0), 2.0); // Exactly left value
    assert_eq!(WtOsc::interpolate(2.0, 3.0, 1.0), 3.0); // Exactly right value
    assert_eq!(WtOsc::interpolate(2.0, 3.0, 0.5), 2.5); // Middle
}

#[test]
fn test_get_sample() {
    //fn get_sample(table: &[Float], table_index: usize, position: Float) -> Float{
    let mut table = [0.0; NUM_VALUES_PER_TABLE];
    table[0] = 2.0;
    table[1] = 3.0;
    assert_eq!(WtOsc::get_sample(&table, 0, 0.0), 2.0); // Exactly first value
    assert_eq!(WtOsc::get_sample(&table, 0, 1.0), 3.0); // Exactly second value
    assert_eq!(WtOsc::get_sample(&table, 0, 0.5), 2.5); // Middle
    assert_eq!(WtOsc::get_sample(&table, 0, 0.09), 2.0); // Close to first
    assert_eq!(WtOsc::get_sample(&table, 0, 0.99), 3.0); // Close to second
}
*/
