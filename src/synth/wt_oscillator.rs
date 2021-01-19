use super::Float;
use wavetable::WavetableRef;

use serde::{Serialize, Deserialize};

const MAX_VOICES: usize = 7;

/// Sound data for the wavetable oscillator
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct WtOscData {
    pub num_voices: i64,
    pub voice_spread: Float,
    pub wave_index: Float, // Index into the wave tables
    pub wavetable: usize,
}

impl WtOscData {
    pub fn init(&mut self) {
        self.set_voice_num(1);
        self.wave_index = 0.0;
    }

    /** Number of detuned voices per oscillator. */
    pub fn set_voice_num(&mut self, voices: i64) {
        self.num_voices = if voices > MAX_VOICES as i64 { MAX_VOICES as i64 } else { voices };
    }

    /** Detune amount per voice. */
    pub fn set_voice_spread(&mut self, spread: Float) {
        self.voice_spread = spread;
    }
}

const NUM_SAMPLES_PER_TABLE: usize = 2048;
const NUM_VALUES_PER_TABLE: usize = NUM_SAMPLES_PER_TABLE + 1; // Add one sample for easier interpolation on last sample

pub struct WtOsc {
    pub sample_rate: Float,
    last_pos: [Float; MAX_VOICES], // State for up to MAX_VOICES oscillators running in sync
    wave: WavetableRef,
}

/// Wavetable oscillator implementation.
///
/// The WT oscillator uses multiple tables per waveform to avoid aliasing. Each
/// table is filled by adding all harmonics that will not exceed the Nyquist
/// frequency for the given usable range of the table (one octave).
///
impl WtOsc {

    /// Create a new wavetable oscillator.
    ///
    pub fn new(sample_rate: u32, wave: WavetableRef) -> WtOsc {
        let sample_rate = sample_rate as Float;
        let last_pos = [0.0; MAX_VOICES];
        WtOsc{sample_rate,
              last_pos,
              wave}
    }

    pub fn set_wavetable(&mut self, wavetable: WavetableRef) {
        self.wave = wavetable;
    }

    // Interpolate between two sample values with the given ratio.
    fn interpolate(val_a: Float, val_b: Float, ratio: Float) -> Float {
        val_a + ((val_b - val_a) * ratio)
    }

    // Get a sample from the given table at the given position.
    //
    // Uses linear interpolation for positions that don't map directly to a
    // table index.
    //
    fn get_wave_sample(table: &[Float], table_index: usize, position: Float) -> Float {
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

    pub fn get_sample(&mut self, frequency: Float, dt: i64, data: &WtOscData) -> (Float, bool) {
        let dt_f = dt as Float;
        let mut result = 0.0;
        let mut complete = false;

        for i in 0..data.num_voices {
            let mut last_pos = self.last_pos[i as usize];
            let freq_diff = (frequency / 100.0) * (data.voice_spread * i as Float) * (1 - ((i & 0x01) * 2)) as Float;
            let frequency = frequency + freq_diff;
            let freq_speed = frequency * (NUM_SAMPLES_PER_TABLE as Float / self.sample_rate);
            let diff = freq_speed * dt_f;
            last_pos += diff;
            if last_pos > (NUM_SAMPLES_PER_TABLE as Float) {
                // Completed one wave cycle
                last_pos -= NUM_SAMPLES_PER_TABLE as Float;
                complete = true; // Sync signal for other oscillators
            }

            let translated_index = (self.wave.table.len() - 1) as Float * data.wave_index;
            let lower_wave = translated_index as usize;
            let lower_wave_float = lower_wave as Float;
            let lower_fract: Float = 1.0 - (translated_index - lower_wave_float);
            let upper_fract: Float = if lower_fract != 1.0 { 1.0 - lower_fract } else { 0.0 };

            let table_index = WtOsc::get_table_index(self.wave.num_octaves, frequency);

            let mut voice_result = WtOsc::get_wave_sample(&self.wave.table[lower_wave], table_index, last_pos) * lower_fract;
            if upper_fract > 0.0 {
                voice_result += WtOsc::get_wave_sample(&self.wave.table[lower_wave + 1], table_index, last_pos) * upper_fract;
            }
            result += voice_result;
            self.last_pos[i as usize] = last_pos;
        }
        (result, complete)
    }

    // Look up the octave table matching the current frequency.
    fn get_table_index(num_octaves: usize, freq: Float) -> usize {
        let two: Float = 2.0;
        let mut compare_freq = (440.0 / 32.0) * (two.powf((-9.0) / 12.0));
        for i in 0..num_octaves {
            if freq < compare_freq * 2.0 {
                return i;
            }
            compare_freq *= 2.0;
        }
        num_octaves - 1
    }

    pub fn reset(&mut self) {
        for i in 0..MAX_VOICES {
            self.last_pos[i] = 0.0;
        }
    }

}

#[cfg(test)]
#[test]
fn test_get_table_index() {
    assert_eq!(WtOsc::get_table_index(11, 10.0), 0);
    assert_eq!(WtOsc::get_table_index(11, 20.0), 1);
    assert_eq!(WtOsc::get_table_index(11, 40.0), 2);
    assert_eq!(WtOsc::get_table_index(11, 80.0), 3);
    assert_eq!(WtOsc::get_table_index(11, 160.0), 4);
    assert_eq!(WtOsc::get_table_index(11, 320.0), 5);
    assert_eq!(WtOsc::get_table_index(11, 640.0), 6);
    assert_eq!(WtOsc::get_table_index(11, 1280.0), 7);
    assert_eq!(WtOsc::get_table_index(11, 2560.0), 8);
    assert_eq!(WtOsc::get_table_index(11, 5120.0), 9);
    assert_eq!(WtOsc::get_table_index(11, 10240.0), 10);
    assert_eq!(WtOsc::get_table_index(11, 20480.0), 10);
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
    assert_eq!(WtOsc::get_wave_sample(&table, 0, 0.0), 2.0); // Exactly first value
    assert_eq!(WtOsc::get_wave_sample(&table, 0, 1.0), 3.0); // Exactly second value
    assert_eq!(WtOsc::get_wave_sample(&table, 0, 0.5), 2.5); // Middle
    assert_eq!(WtOsc::get_wave_sample(&table, 0, 0.09), 2.0); // Close to first
    assert_eq!(WtOsc::get_wave_sample(&table, 0, 0.99), 3.0); // Close to second
}
