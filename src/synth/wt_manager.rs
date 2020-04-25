/* Manages wavetables.
 *
 * Has a cache with wavetables, handing references out to wt_oscillators asking
 * for a table.
 */

use super::Float;
use super::Wavetable;
use super::WtReader;

use log::{info, trace, warn};

//use std::collections::HashMap;
use std::sync::Arc;
//use std::rc::Weak;

pub struct WtManager {
    sample_rate: Float,
    default_table: Arc<Wavetable>, // Table with default waveshapes

    // Can't have a HashMap here, as it's not thread safe. Need to find a
    // better way.
    //cache: HashMap<String, Weak<Wavetable>>,
}

impl WtManager {
    pub fn new(sample_rate: Float) -> Arc<WtManager> {
        let default_table = WtManager::initialize_default_tables(sample_rate);
        //let cache = HashMap::new();
        //let def_copy = Arc::clone(&default_table);
        let wt = WtManager{sample_rate, default_table};
        //wt.add_to_cache(def_copy);
        Arc::new(wt)
    }

    /** Get a single wavetable by name. */
    pub fn get_table(&self, table_name: &str) -> Option<Arc<Wavetable>> {
        /*
        if self.cache.contains_key(table_name) {
            let weak_ref = self.cache.get(table_name).unwrap();
            weak_ref.upgrade()
        } else {
            // TODO: Call Wavetable to construct new table
            // (search for file etc.)
            None
        }
        */
        return Some(Arc::clone(&self.default_table));
    }

    /*
    fn add_to_cache(&mut self, wt: Arc<Wavetable>) {
        self.cache.insert(wt.name.clone(), Arc::downgrade(&wt));
    }
    */

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
    fn initialize_default_tables(sample_rate: Float) -> Arc<Wavetable> {
        info!("Initializing default waveshapes");
        let name = "Basic".to_string();
        let mut wt = Wavetable::new(&name, 4, 11, 2048);
        let two: Float = 2.0;
        let start_freq = (440.0 / 32.0) * (two.powf((-9.0) / 12.0));
        wt.create_tables(0, start_freq, sample_rate, WtManager::insert_sine);
        wt.create_tables(1, start_freq, sample_rate, WtManager::insert_tri);
        wt.create_tables(2, start_freq, sample_rate, WtManager::insert_saw);
        wt.create_tables(3, start_freq, sample_rate, WtManager::insert_square);
        info!("Finished");
        Arc::new(wt)
        /*
        let result = WtReader::read_file("data/ESW Analog - 80's PWM.wav");
        if let Ok(wt) = result { wt } else { panic!(); }
        */
    }
}

