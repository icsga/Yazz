/* A wavetable representing a collection of waveshapes.
 *
 * A wavetable consists of a collection of waveshapes. Every waveshape in the
 * wavetable itself contains multiple bandlimited tables for use in different
 * octaves.
 *
 * In memory, the table is stored as a vector of vectors. The inner vector
 * holds the samples of a single waveshape, with the different octave tables
 * octaves stored as a contiguous piece of memory. The outer vector holds the
 * different waveshapes.
 */

use super::Float;

use log::{info, debug, trace, warn};

use std::sync::Arc;

pub struct Wavetable {
    pub num_tables: usize,  // Number of different waveshapes
    pub num_octaves: usize, // Number of octave tables to generate per waveshape
    pub num_values: usize,  // Length of a single octave table, including duplicated first element (usually 2049)
    pub num_samples: usize, // Length of a single octave table - 1, actual number of unique values (usually 2048)
    pub table: Vec<Vec<Float>>, // Vector of vectors holding all tables
}

pub type WavetableRef = Arc<Wavetable>;

impl Wavetable {
    pub fn new(num_tables: usize, num_octaves: usize, num_samples: usize) -> Wavetable {
        let num_values = num_samples + 1;
        let table = vec!(vec!(0.0; num_values * num_octaves); num_tables);
        info!("New Wavetable: {} tables for {} octaves, {} samples",
              num_tables, num_octaves, num_samples);
        Wavetable {
            num_tables,
            num_octaves,
            num_values,
            num_samples,
            table
        }
    }

    pub fn new_from_vector(num_tables: usize, num_octaves: usize, num_samples: usize, table: Vec<Vec<Float>>) -> WavetableRef {
        let num_values = num_samples + 1;
        info!("New Wavetable: {} tables for {} octaves, {} samples",
              num_tables, num_octaves, num_samples);
        Arc::new(Wavetable {
            num_tables,
            num_octaves,
            num_values,
            num_samples,
            table
        })
    }

    /** Return a table vector for the selected waveshape. */
    pub fn get_wave(&mut self, wave_id: usize) -> &mut Vec<Float> {
        &mut self.table[wave_id]
    }

    // -----------------------------------------
    // Functions for constructing wavetable data
    // -----------------------------------------

    /** Calculates the number of non-aliasing harmonics for one octave.
     *
     * Calculates all the harmonics for the octave starting at base_freq that
     * do not exceed the Nyquist frequency.
     */
    pub fn calc_num_harmonics(base_freq: Float, sample_freq: Float) -> usize {
        debug!("Calculating harmonics for frequency {} Hz with sample frequency {} Hz", base_freq, sample_freq);
        let nyquist_freq = sample_freq / 2.0;
        let mut part_freq = base_freq * 2.0;
        let mut prev_part = part_freq;
        let mut num_harmonics = 0.0;
        while part_freq < nyquist_freq {
            num_harmonics += 1.0;
            prev_part = part_freq;
            part_freq = base_freq * (num_harmonics + 2.0);
        }
        debug!("Got {} harmonics, highest at {} Hz ", num_harmonics as usize, prev_part);
        num_harmonics as usize
    }

    /** Add a wave with given frequency to the wave in a table.
     *
     * Frequency is relative to the buffer length, so a value of 1 will put one
     * wave period into the table. The values are added to the values already
     * in the table. Giving a negative amplitude will subtract the values.
     *
     * The last sample in the table receives the same value as the first, to
     * allow more efficient interpolation (eliminates the need for index
     * wrapping).
     *
     * wave_func is a function receiving an input in the range [0:1] and
     * returning a value in the same range.
     */
    fn add_wave(table: &mut [Float], freq: Float, amplitude: Float, wave_func: fn(Float) -> Float) {
        let num_samples = table.len() - 1;
        let mult = freq * 2.0 * std::f32::consts::PI;
        let mut position: Float;
        for i in 0..num_samples {
            position = mult * (i as Float / num_samples as Float);
            table[i] = table[i] + wave_func(position) * amplitude;
        }
        table[table.len() - 1] = table[0]; // Add extra sample for interpolation
    }

    /** Add a sine wave with given frequency and amplitude to the buffer. */
    pub fn add_sine_wave(table: &mut [Float], freq: Float, amplitude: Float) {
        Wavetable::add_wave(table, freq, amplitude, f32::sin);
    }

    /** Add a cosine wave with given frequency and amplitude to the buffer. */
    pub fn add_cosine_wave(table: &mut [Float], freq: Float, amplitude: Float) {
        Wavetable::add_wave(table, freq, amplitude, f32::cos);
    }

    /** Create octave tables with given insert function.
     *
     * Divides the given table into NUM_TABLES subtables and uses the given
     * insert function to insert waveforms into them. Each table serves the
     * frequency range of one octave.
     */
    pub fn create_tables(&mut self,
                         table_id: usize,
                         start_freq: Float,
                         sample_freq: Float,
                         insert_wave: fn(&mut [Float], Float, Float)) {
        info!("Creating table {}", table_id);
        let num_octaves = self.num_octaves;
        let num_values = self.num_values;
        let table = self.get_wave(table_id);
        let mut current_freq = start_freq;
        for i in 0..num_octaves {
            let from = i * num_values;
            let to = (i + 1) * num_values;
            insert_wave(&mut table[from..to], current_freq, sample_freq);
            current_freq *= 2.0; // Next octave
        }
    }

    /** Normalizes samples in a buffer to the range [-1.0,1.0] */
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

    pub fn shift(table: &mut [Float], num_values: usize, offset: usize) {
        let mut temp = vec!(0.0; num_values);
        let mut offset = offset;
        for i in 0..num_values {
            temp[offset] = table[i];
            offset += 1;
            if offset == num_values {
                offset = 0;
            }
        }
        for i in 0..num_values {
            table[i] = temp[i]; // Copy back
        }
        table[num_values] = table[0];
    }
}

// TODO: Add tests for wave generation
