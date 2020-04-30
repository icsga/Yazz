/* Manages wavetables.
 *
 * Has a cache with wavetables, handing references out to wt_oscillators asking
 * for a table.
 */

use super::Float;
use super::{Wavetable, WavetableRef};
use super::WtReader;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

use std::collections::HashMap;
use std::sync::Arc;
//use std::rc::Weak;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WtInfo {
    pub id: usize,       // Index to wavetable used in the sound data
    pub valid: bool,     // True if file exists
    pub name: String,    // Name of wavetable as shown in the UI
    pub filename: String // Wavetable filename, empty if internal table
}

pub struct WtManager {
    sample_rate: Float,
    default_table: WavetableRef, // Table with default waveshapes
    cache: HashMap<usize, WavetableRef>,
    reader: WtReader,
}

impl WtManager {
    pub fn new(sample_rate: Float) -> WtManager {
        let default_table = WtManager::initialize_default_tables(sample_rate);
        let cache = HashMap::new();
        let def_copy = default_table.clone();
        let reader = WtReader::new("data/");
        let mut wt = WtManager{sample_rate, default_table, cache, reader};
        wt.add_to_cache(0, def_copy);
        wt
    }

    /** Receives information about a wavetable to load.
     *
     * Tries to load the table from the given file and put it into the cache.
     * If loading the file fails, the default table is inserted instead.
     */
    pub fn add_table(&mut self, wt: WtInfo) {
        let result = self.reader.read_file(&wt.filename);
        let table = if let Ok(wt) = result { wt } else { self.default_table.clone() };
        self.add_to_cache(wt.id, table);
    }

    /** Get a single wavetable by id. */
    pub fn get_table(&self, id: usize) -> Option<WavetableRef> {
        if self.cache.contains_key(&id) {
            Some(self.cache.get(&id).unwrap().clone())
        } else {
            None
        }
    }

    fn add_to_cache(&mut self, id: usize, wt: WavetableRef) {
        self.cache.insert(id, wt);
    }

    // ------------------
    // Default waveshapes
    // ------------------

    /** Insert a sine wave into the given table. */
    fn insert_sine(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        Wavetable::add_sine_wave(table, 1.0, 1.0);
    }

    /** Insert a saw wave into the given table.
     *
     * Adds all odd harmonics, subtracts all even harmonics, with reciprocal
     * amplitude.
     */
    fn insert_saw(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        let num_harmonics = Wavetable::calc_num_harmonics(start_freq * 2.0, sample_freq);
        let mut sign: Float;
        for i in 1..num_harmonics + 1 {
            sign = if (i & 1) == 0 { 1.0 } else { -1.0 };
            Wavetable::add_sine_wave(table, i as Float, 1.0 / i as Float * sign);
        }
        Wavetable::normalize(table);
        // Shift by 180 degrees to keep it symmetrical to Sine wave
        Wavetable::shift(table, table.len() & 0xFFFFFFFC, table.len() / 2);
    }

    /** Insert a saw wave into the given table.
     *
     * Adds all harmonics. Should be wrong, but sounds the same.
     */
    fn insert_saw_2(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        let num_harmonics = Wavetable::calc_num_harmonics(start_freq * 2.0, sample_freq);
        for i in 1..num_harmonics + 1 {
            Wavetable::add_sine_wave(table, i as Float, 1.0 / i as Float);
        }
        Wavetable::normalize(table);
    }

    /** Insert a triangular wave into the given table.
     *
     * Adds odd cosine harmonics with squared odd reciprocal amplitude.
     */
    fn insert_tri(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        let num_harmonics = Wavetable::calc_num_harmonics(start_freq * 2.0, sample_freq);
        for i in (1..num_harmonics + 1).step_by(2) {
            Wavetable::add_cosine_wave(table, i as Float, 1.0 / ((i * i) as Float));
        }
        Wavetable::normalize(table);
        // Shift by 90 degrees to keep it symmetrical to Sine wave
        Wavetable::shift(table, table.len() & 0xFFFFFFFC, table.len() / 4);
    }

    /** Insert a square wave into the given table.
     *
     * Adds odd sine harmonics with odd reciprocal amplitude.
     */
    fn insert_square(table: &mut [Float], start_freq: Float, sample_freq: Float) {
        let num_harmonics = Wavetable::calc_num_harmonics(start_freq * 2.0, sample_freq);
        for i in (1..num_harmonics + 1).step_by(2) {
            Wavetable::add_sine_wave(table, i as Float, 1.0 / i as Float);
        }
        Wavetable::normalize(table);
    }

    /** Create tables of common waveforms (sine, triangle, square, saw). */
    fn initialize_default_tables(sample_rate: Float) -> WavetableRef {
        info!("Initializing default waveshapes");
        let mut wt = Wavetable::new(4, 11, 2048);
        let two: Float = 2.0;
        let start_freq = (440.0 / 32.0) * (two.powf((-9.0) / 12.0));
        wt.create_tables(0, start_freq, sample_rate, WtManager::insert_sine);
        wt.create_tables(1, start_freq, sample_rate, WtManager::insert_tri);
        wt.create_tables(2, start_freq, sample_rate, WtManager::insert_saw);
        wt.create_tables(3, start_freq, sample_rate, WtManager::insert_square);
        info!("Finished");
        Arc::new(wt)
    }
}

