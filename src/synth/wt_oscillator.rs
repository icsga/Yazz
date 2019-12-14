use super::Float;
use super::SampleGenerator;
use super::sound::SoundData;

use rand::prelude::*;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use log::{info, trace, warn};

const MAX_VOICES: usize = 7;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
pub struct WavetableOscData {
    pub level: Float,
    pub phase: Float,
    pub sine_ratio: Float,
    pub tri_ratio: Float,
    pub saw_ratio: Float,
    pub square_ratio: Float,
    pub noise_ratio: Float,
    pub num_voices: i64,
    pub voice_spread: Float,
    pub tune_halfsteps: i64,
    pub tune_cents: Float,
    pub freq_offset: Float, // Value derived from tune_halfsteps and tune_cents
    pub sync: i64,
    pub key_follow: i64,
}

impl WavetableOscData {
    pub fn init(&mut self) {
        self.level = 0.92;
        self.phase = 0.5;
        self.select_wave(0);
        self.set_voice_num(1);
        self.set_halfsteps(0);
        self.sync = 0;
        self.key_follow = 1;
    }

    /** Select a single waveform. */
    pub fn select_wave(&mut self, value: usize) {
        match value {
            0 => self.set_ratios(1.0, 0.0, 0.0, 0.0, 0.0),
            1 => self.set_ratios(0.0, 1.0, 0.0, 0.0, 0.0),
            2 => self.set_ratios(0.0, 0.0, 1.0, 0.0, 0.0),
            3 => self.set_ratios(0.0, 0.0, 0.0, 1.0, 0.0),
            4 => self.set_ratios(0.0, 0.0, 0.0, 0.0, 1.0),
            _ => {}
        }
    }

    /** Select a free mix of all waveforms. */
    pub fn set_ratios(&mut self, sine_ratio: Float, tri_ratio: Float, saw_ratio: Float, square_ratio: Float, noise_ratio: Float) {
        self.sine_ratio = sine_ratio;
        self.tri_ratio = tri_ratio;
        self.saw_ratio = saw_ratio;
        self.square_ratio = square_ratio;
        self.noise_ratio = noise_ratio;
    }

    /** Select a mix of up to two waveforms. */
    pub fn set_ratio(&mut self, ratio: Float) {
        if ratio <= 1.0 {
            self.set_ratios(1.0 - ratio, ratio, 0.0, 0.0, 0.0);
        } else if ratio <= 2.0 {
            self.set_ratios(0.0, 1.0 - (ratio - 1.0), ratio - 1.0, 0.0, 0.0);
        } else if ratio <= 3.0 {
            self.set_ratios(0.0, 0.0, 1.0 - (ratio - 2.0), ratio - 2.0, 0.0);
        } else if ratio <= 4.0 {
            self.set_ratios(0.0, 0.0, 0.0, 1.0 - (ratio - 3.0), ratio - 3.0);
        }
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

    pub fn get_waveform(&self) -> i64 {
        if self.sine_ratio > 0.0 {
            0
        } else if self.tri_ratio > 0.0 {
            1
        } else if self.saw_ratio > 0.0 {
            2
        } else if self.square_ratio > 0.0 {
            3
        } else if self.noise_ratio > 0.0 {
            4
        } else {
            0
        }
    }

    pub fn get_ratio(&self) -> Float {
        if self.sine_ratio > 0.0 {
            self.tri_ratio
        } else if self.tri_ratio > 0.0 {
            self.saw_ratio + 1.0
        } else if self.saw_ratio > 0.0 {
            self.square_ratio + 2.0
        } else if self.square_ratio > 0.0 {
            self.noise_ratio + 3.0
        } else {
            0.0
        }
    }
}

const NUM_TABLES: usize = 11;
const NUM_SAMPLES_PER_TABLE: usize = (2048 + 1); // Add one sample for easier interpolation on last sample
const TABLE_SIZE: usize = NUM_SAMPLES_PER_TABLE * NUM_TABLES;

#[derive(Copy, Clone)]
struct State {
    last_pos: Float,
    freq_shift: Float, // Percentage this voice is shifted from center frequency
    level_shift: Float, // Decrease in level compared to main voice (TODO)
}

pub struct WavetableOscillator {
    sample_rate: Float,
    id: usize,
    last_update: i64, // Time of last sample generation
    state: [State; MAX_VOICES], // State for up to MAX_VOICES oscillators running in sync
    table_sine: Vec<Float>,
    table_tri: Vec<Float>,
    table_saw: Vec<Float>,
    table_square: Vec<Float>,
}

impl WavetableOscillator {
    pub fn new(sample_rate: u32, id: usize) -> WavetableOscillator {
        let sample_rate = sample_rate as Float;
        let last_update = 0;
        let last_pos = 0.0;
        let freq_shift = 0.0;
        let level_shift = 1.0;
        let state = [State{last_pos, freq_shift, level_shift}; MAX_VOICES];
        let table_sine = vec![0.0; TABLE_SIZE];
        let table_tri = vec![0.0; TABLE_SIZE];
        let table_saw = vec![0.0; TABLE_SIZE];
        let table_square = vec![0.0; TABLE_SIZE];
        let mut osc = WavetableOscillator{sample_rate,
                                          id,
                                          last_update,
                                          state,
                                          table_sine,
                                          table_tri,
                                          table_saw,
                                          table_square
                                          };
        osc.initialize_tables();
        osc
    }

    /** Calculates the number of non-aliasing partials for one octave. */
    fn calc_num_partials(base_freq: Float, sample_freq: Float) -> usize {
        info!("Calculating partials for frequency {} Hz with sample frequency {} Hz", base_freq, sample_freq);
        let nyquist_freq = sample_freq / 2.0;
        let mut part_freq = base_freq * 2.0;
        let mut prev_part = part_freq;
        let mut num_partials = 0.0;
        while part_freq < nyquist_freq {
            num_partials += 1.0;
            prev_part = part_freq;
            part_freq = base_freq * (num_partials + 2.0);
        }
        info!("Got {} partials, highest at {} Hz ", num_partials as usize, prev_part);
        num_partials as usize
    }

    /** Add a sine wave with given frequency and amplitude to the buffer.
     *
     * Frequency is relative to the buffer length. The last sample of the table
     * gets the same value as the first for faster interpolation.
     */
    pub fn add_sine_wave(table: &mut [Float], freq: Float, amplitude: Float) {
        let num_samples = table.len() - 1;
        let mult = freq * 2.0 * std::f32::consts::PI;
        let mut position: Float;
        for i in 0..num_samples {
            position = mult * (i as Float / num_samples as Float);
            table[i] = table[i] + f32::sin(position) * amplitude;
        }
        table[table.len() - 1] = table[0]; // Add extra sample for interpolation
    }

    /** Add a cosine wave with given frequency and amplitude to the buffer.
     *
     * Frequency is relative to the buffer length. The last sample of the table
     * gets the same value as the first for faster interpolation.
     */
    pub fn add_cosine_wave(table: &mut [Float], freq: Float, amplitude: Float) {
        let num_samples = table.len() - 1;
        let mult = freq * 2.0 * std::f32::consts::PI;
        let mut position: Float;
        for i in 0..num_samples {
            position = mult * (i as Float / num_samples as Float);
            table[i] = table[i] + f32::cos(position) * amplitude;
        }
        table[table.len() - 1] = table[0]; // Add extra sample for interpolation
    }

    /** Normalizes samples in a buffer to the range [1.0,-1.0] */
    pub fn normalize(table: &mut [Float]) {
        let mut max = 0.0;
        let mut current: Float;
        for i in 0..table.len() {
            current = table[i].abs();
            if current > max {
                max = current;
            }
        }
        for i in 0..table.len() {
            table[i] = table[i] / max;
        }
    }

    pub fn insert_sine(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        WavetableOscillator::add_sine_wave(table, 1.0, 1.0);
    }

    pub fn insert_saw(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        let num_partials = WavetableOscillator::calc_num_partials(start_freq * 2.0, sample_freq);
        let mut sign: Float;
        for i in 1..num_partials + 1 {
            sign = if (i & 1) == 0 { 1.0 } else { -1.0 };
            WavetableOscillator::add_sine_wave(table, i as Float, 1.0 / i as Float * sign);
        }
        WavetableOscillator::normalize(table);
    }

    pub fn insert_saw_2(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        let num_partials = WavetableOscillator::calc_num_partials(start_freq * 2.0, sample_freq);
        for i in 1..num_partials + 1 {
            WavetableOscillator::add_sine_wave(table, i as Float, 1.0 / i as Float);
        }
        WavetableOscillator::normalize(table);
    }

    pub fn insert_tri(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        let num_partials = WavetableOscillator::calc_num_partials(start_freq * 2.0, sample_freq);
        for i in (1..num_partials + 1).step_by(2) {
            WavetableOscillator::add_cosine_wave(table, i as Float, 1.0 / ((i * i) as Float));
        }
        WavetableOscillator::normalize(table);
    }

    pub fn insert_square(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        let num_partials = WavetableOscillator::calc_num_partials(start_freq * 2.0, sample_freq);
        for i in (1..num_partials + 1).step_by(2) {
            WavetableOscillator::add_sine_wave(table, i as Float, 1.0 / i as Float);
        }
        WavetableOscillator::normalize(table);
    }

    pub fn create_tables(table: &mut [Float],
                         start_freq: Float,
                         sample_freq: Float,
                         func: fn(&mut [Float], Float, Float)) {
        let mut current_freq = start_freq;
        for i in 0..NUM_TABLES {
            let from = i * NUM_SAMPLES_PER_TABLE;
            let to = (i + 1) * NUM_SAMPLES_PER_TABLE;
            func(&mut table[from..to], current_freq, sample_freq);
            current_freq *= 2.0; // Next octave
        }
    }

    pub fn initialize_tables(&mut self) {
        let two: Float = 2.0;
        let start_freq = (440.0 / 32.0) * (two.powf((-9.0) / 12.0));
        info!("Start frequency: {}", start_freq);
        WavetableOscillator::create_tables(&mut self.table_sine, start_freq, self.sample_rate, WavetableOscillator::insert_sine);
        WavetableOscillator::create_tables(&mut self.table_tri, start_freq, self.sample_rate, WavetableOscillator::insert_tri);
        WavetableOscillator::create_tables(&mut self.table_square, start_freq, self.sample_rate, WavetableOscillator::insert_square);
        WavetableOscillator::create_tables(&mut self.table_saw, start_freq, self.sample_rate, WavetableOscillator::insert_saw);
    }

    fn get_sample(table: &[Float], table_index: usize, position: Float) -> Float{
        let position = position as usize + table_index * NUM_SAMPLES_PER_TABLE;
        table[position]
    }

    fn get_sample_noise() -> Float {
        (rand::random::<Float>() * 2.0) - 1.0
    }

    fn get_table_index(freq: Float) -> usize {
        let two: Float = 2.0;
        let mut compare_freq = (440.0 / 32.0) * (two.powf((-9.0) / 12.0));
        let i: usize = 0;
        for i in 0..NUM_TABLES {
            if freq < compare_freq * 2.0 {
                return i;
            }
            compare_freq *= 2.0;
        }
        NUM_TABLES - 1
    }
}

impl SampleGenerator for WavetableOscillator {
    fn get_sample(&mut self, frequency: Float, sample_clock: i64, data: &SoundData, reset: bool) -> (Float, bool) {
        let data = data.get_osc_data(self.id);
        let dt = sample_clock - self.last_update;
        let dt_f = dt as Float;
        let mut result = 0.0;
        let mut complete = false;
        if reset {
            self.reset(sample_clock - 1);
        }

        for i in 0..data.num_voices {
            let state: &mut State = &mut self.state[i as usize];
            let freq_diff = (frequency / 100.0) * (data.voice_spread * i as Float) * (1 - (i & 0x01 * 2)) as Float;
            let frequency = frequency + freq_diff;
            let freq_speed = frequency * ((NUM_SAMPLES_PER_TABLE - 1) as Float / self.sample_rate);
            let diff = freq_speed * dt_f;
            let mut voice_result = 0.0;
            state.last_pos += diff;
            if state.last_pos > (NUM_SAMPLES_PER_TABLE as Float) {
                // Completed one wave cycle
                state.last_pos -= NUM_SAMPLES_PER_TABLE as Float;
                complete = true; // Sync signal for other oscillators
            }

            let table_index = WavetableOscillator::get_table_index(frequency);

            if data.sine_ratio > 0.0 {
                voice_result += WavetableOscillator::get_sample(&self.table_sine, table_index, state.last_pos) * data.sine_ratio;
            }
            if data.tri_ratio > 0.0 {
                voice_result += WavetableOscillator::get_sample(&self.table_tri, table_index, state.last_pos) * data.tri_ratio;
            }
            if data.saw_ratio > 0.0 {
                voice_result += WavetableOscillator::get_sample(&self.table_saw, table_index, state.last_pos) * data.saw_ratio;
            }
            if data.square_ratio > 0.0 {
                voice_result += WavetableOscillator::get_sample(&self.table_square, table_index, state.last_pos) * data.square_ratio;
            }
            if data.noise_ratio > 0.0 {
                voice_result += WavetableOscillator::get_sample_noise() * data.noise_ratio;
            }

            //voice_result *= 1.0 - (i as Float * 0.1);
            result += voice_result;
        }
        self.last_update += dt;
        //result /= data.num_voices as Float; // TODO: Scale level to number of active voices
        result *= data.level;
        if result > 1.0 {
            result = 1.0;
        }
        (result, complete)
    }

    fn reset(&mut self, sample_clock: i64) {
        for state in self.state.iter_mut() {
            state.last_pos = 0.0;
        }
        self.last_update = sample_clock;
    }
}

#[cfg(test)]
#[test]
fn test_calc_num_partials() {
    /* Base frequency: 2 Hz
     * Sample frequency 20 Hz
     * Nyquist: 10 Hz
     * Num partials: 2, 4, 6, 8 = 3
     */
    assert_eq!(WavetableOscillator::calc_num_partials(2.0, 20.0), 3);
}

#[test]
fn test_get_table_index() {
    assert_eq!(WavetableOscillator::get_table_index(10.0), 0);
    assert_eq!(WavetableOscillator::get_table_index(20.0), 1);
    assert_eq!(WavetableOscillator::get_table_index(40.0), 2);
    assert_eq!(WavetableOscillator::get_table_index(80.0), 3);
    assert_eq!(WavetableOscillator::get_table_index(160.0), 4);
    assert_eq!(WavetableOscillator::get_table_index(320.0), 5);
    assert_eq!(WavetableOscillator::get_table_index(640.0), 6);
    assert_eq!(WavetableOscillator::get_table_index(1280.0), 7);
    assert_eq!(WavetableOscillator::get_table_index(2560.0), 8);
    assert_eq!(WavetableOscillator::get_table_index(5120.0), 9);
    assert_eq!(WavetableOscillator::get_table_index(10240.0), 10);
    assert_eq!(WavetableOscillator::get_table_index(20480.0), 10);
}
