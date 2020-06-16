use super::Delay;
use super::{SynthMessage, UiMessage};
use super::Envelope;
use super::Lfo;
use super::MidiMessage;
use super::{Parameter, ParamId, SynthParam, MenuItem};
use super::SoundData;
use super::voice::Voice;
use super::Oscillator;
use super::Float;

use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crossbeam_channel::{Sender, Receiver};
use log::{info, error};
use serde::{Serialize, Deserialize};
use wavetable::{WtManager, WavetableRef, WtInfo};

const NUM_VOICES: usize = 32;
const NUM_KEYS: usize = 128;
pub const NUM_MODULATORS: usize = 16;
pub const NUM_GLOBAL_LFOS: usize = 2;
const REF_FREQUENCY: Float = 440.0;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum PlayMode {
    Poly,   // Polyphonic
    Mono,   // Monophonic, retrigger on key-on-event
    Legato  // Monphonic, no retrigger until key-off-event
}

impl Default for PlayMode {
    fn default() -> Self { PlayMode::Poly }
}

impl PlayMode {
    pub fn from_int(param: usize) -> PlayMode {
        match param {
            0 => PlayMode::Poly,
            1 => PlayMode::Mono,
            2 => PlayMode::Legato,
            _ => panic!(),
        }
    }

    pub fn to_int(&self) -> usize {
        match self {
            PlayMode::Poly   => 0,
            PlayMode::Mono   => 1,
            PlayMode::Legato => 2,
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum FilterRouting {
    Parallel, // both filters get summed as output
    Serial    // filter1 goes into filter2, filter2 goes to out
}

impl Default for FilterRouting {
    fn default() -> Self { FilterRouting::Parallel }
}

impl FilterRouting {
    pub fn from_int(param: usize) -> FilterRouting {
        match param {
            0 => FilterRouting::Parallel,
            1 => FilterRouting::Serial,
            _ => panic!(),
        }
    }

    pub fn to_int(&self) -> usize {
        match self {
            FilterRouting::Parallel => 0,
            FilterRouting::Serial => 1,
        }
    }
}

// Data of the currently selected sound patch
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
pub struct PatchData {
    pub level: Float,
    pub drive: Float,
    pub pitchbend: Float, // Range of the pitchwheel
    pub vel_sens: Float,  // Velocity sensitivity
    pub env_depth: Float, // Mod depth of env1 to volume. TODO: Move to Env1 menu
    pub play_mode: PlayMode,
    pub filter_routing: FilterRouting,
    pub bpm: Float,       // Patch tempo for synced settings (LFO, delay)
}

impl PatchData {
    pub fn init(&mut self) {
        self.level = 0.5;
        self.drive = 0.0;
        self.pitchbend = 2.0;
        self.vel_sens = 1.0;
        self.env_depth = 1.0;
        self.play_mode = PlayMode::Poly;
    }
}

/** Global synth state.
 *
 * Holds dynamically calculated parameters like pitch offset.
 */
pub struct SynthState {
    pub freq_factor: Float,
}

pub struct Synth {
    // Configuration
    sample_rate: u32,
    sound: SoundData,        // Sound patch as loaded from disk
    sound_global: SoundData, // Sound with global modulators applied
    sound_local: SoundData,  // Sound with voice-local modulators applied
    keymap: [Float; NUM_KEYS],
    wt_manager: WtManager,

    // Signal chain
    voice: [Voice; NUM_VOICES],
    delay: Delay,
    glfo: [Lfo; NUM_GLOBAL_LFOS],

    // Current state
    num_voices_triggered: u32,
    voices_playing: u32, // Bitmap with currently playing voices
    trigger_seq: u64,
    last_clock: i64,
    pitch_bend: Float,
    mod_wheel: Float,
    aftertouch: Float,
    sustain_pedal: Float, // Use a float, so that we can use it as mod source
    sender: Sender<UiMessage>,
    global_state: SynthState,
    key_stack: Vec<u16>, // List of currently pressed keys (for Mono/ Legato modes)

    // Extra oscillators to display the waveshape
    samplebuff_osc: Oscillator,
    samplebuff_env: Envelope,
    samplebuff_lfo: Lfo,
    osc_wave: [WavetableRef; 3],
}

impl Synth {
    pub fn new(sample_rate: u32, sender: Sender<UiMessage>) -> Self {
        let mut sound = SoundData::new();
        let mut sound_global = SoundData::new();
        let mut sound_local = SoundData::new();
        sound.init();
        sound_global.init();
        sound_local.init();
        let mut wt_manager = WtManager::new(sample_rate as Float, "data");
        wt_manager.add_basic_tables(0);
        wt_manager.add_pwm_tables(1, 64);
        let default_table = wt_manager.get_table(0).unwrap(); // Table 0
        let voice = [
            Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()),
            Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()),
            Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()),
            Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()),
            Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()),
            Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()),
            Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()),
            Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()), Voice::new(sample_rate, default_table.clone()),
        ];
        let glfo = [
            Lfo::new(sample_rate), Lfo::new(sample_rate)
        ];
        let mut keymap: [Float; NUM_KEYS] = [0.0; NUM_KEYS];
        Synth::calculate_keymap(&mut keymap, REF_FREQUENCY);
        let osc_wave = [default_table.clone(), default_table.clone(), default_table.clone()];
        Synth{
            sample_rate,
            sound,
            sound_global,
            sound_local,
            keymap,
            wt_manager,
            voice,
            delay: Delay::new(sample_rate),
            glfo,
            num_voices_triggered: 0,
            voices_playing: 0,
            trigger_seq: 0,
            last_clock: 0i64,
            pitch_bend: 0.0,
            mod_wheel: 0.0,
            aftertouch: 0.0,
            sustain_pedal: 0.0,
            sender,
            global_state: SynthState{freq_factor: 1.0},
            key_stack: vec!(0; 128),
            samplebuff_osc: Oscillator::new(sample_rate, default_table.clone()),
            samplebuff_env: Envelope::new(sample_rate as Float),
            samplebuff_lfo: Lfo::new(sample_rate),
            osc_wave,
        }
    }

    /// Starts a thread for receiving UI and MIDI messages.
    pub fn run(synth: Arc<Mutex<Synth>>, synth_receiver: Receiver<SynthMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            let mut keep_running = true;
            while keep_running {
                let msg = synth_receiver.recv().unwrap();
                let mut locked_synth = synth.lock().unwrap();
                match msg {
                    SynthMessage::Param(m) => locked_synth.handle_ui_message(m),
                    SynthMessage::Midi(m)  => locked_synth.handle_midi_message(m),
                    SynthMessage::Sound(s) => locked_synth.handle_sound_update(&s),
                    SynthMessage::Wavetable(i) => locked_synth.handle_wavetable_info(i),
                    SynthMessage::SampleBuffer(m, p) => locked_synth.handle_sample_buffer(m, p),
                    SynthMessage::Bpm(b) => locked_synth.handle_bpm(b),
                    SynthMessage::Exit     => {
                        keep_running = false;
                        locked_synth.exit();
                    }
                }
            }
        });
        handler
    }

    fn exit(&mut self) {
        // Do exit stuff here
        info!("Stopping synth engine");
    }

    fn reset(&mut self) {
        self.voice.iter_mut().for_each(|v| v.reset());
        self.delay.reset();
        self.key_stack.clear();
    }

    // Get global modulation values.
    //
    // Calculates the values for global modulation sources and applies them to
    // the global sound data.
    //
    fn get_mod_values(&mut self, sample_clock: i64) {
        // Reset the global sound copy to the main sound, discarding values that
        // were modulated for the previous sample. Complete copy is faster than
        // looping over the modulators.
        self.sound_global = self.sound;

        // Then apply global modulators
        let mut param_id = ParamId{..Default::default()};
        let mut synth_param = SynthParam{..Default::default()};
        for m in self.sound.modul.iter() {
            if !m.active || !m.is_global {
                continue;
            }

            // Get modulator source output
            let mod_val: Float = match m.source_func {
                Parameter::GlobalLfo => {
                    let (val, _) = self.glfo[m.source_func_id - 1].get_sample(sample_clock, &self.sound_global.glfo[m.source_func_id - 1], false);
                    val
                },
                Parameter::Aftertouch => self.aftertouch,
                Parameter::Pitchbend => self.pitch_bend,
                Parameter::ModWheel => self.mod_wheel,
                Parameter::SustainPedal => self.sustain_pedal,
                _ => 0.0,
            } * m.scale;


            // Get current value of target parameter
            param_id.set(m.target_func, m.target_func_id, m.target_param);
            let mut current_val = self.sound_global.get_value(&param_id);
            let mut val = current_val.as_float();

            // Update value
            let dest_range = MenuItem::get_val_range(param_id.function, param_id.parameter);
            val = dest_range.safe_add(val, mod_val);

            // Update parameter in global sound data
            current_val.set_from_float(val);
            synth_param.set(m.target_func, m.target_func_id, m.target_param, current_val);
            self.sound_global.set_parameter(&synth_param);
        }
    }

    /// Called by the audio engine to get the next sample to be output.
    pub fn get_sample(&mut self, sample_clock: i64) -> (Float, Float) {
        let mut value: Float = 0.0;

        self.get_mod_values(sample_clock);

        // Get sample of all active voices
        if self.voices_playing > 0 {
            for i in 0..32 {
                if self.voices_playing & (1 << i) > 0 {
                    value += self.voice[i].get_sample(sample_clock, &self.sound_global, &mut self.sound_local, &self.global_state);
                }
            }
        }

        // Apply clipping
        if self.sound_global.patch.drive > 0.0 {
            value = (value * self.sound_global.patch.drive).tanh();
        }

        // Pass sample into global effects
        let (mut value_l, mut value_r) = self.delay.process(value, sample_clock, &self.sound_global.delay);

        value_l *= self.sound_global.patch.level;
        value_r *= self.sound_global.patch.level;

        self.last_clock = sample_clock;
        (value_l, value_r)
    }

    /// Update the bitmap with currently active voices.
    pub fn update(&mut self) {
        self.voices_playing = 0;
        for (i, v) in self.voice.iter_mut().enumerate() {
            if v.is_running() {
                self.voices_playing |= 1 << i;
            }
        }
    }

    // Calculates the frequencies for the default keymap with equal temperament.
    fn calculate_keymap(map: &mut[Float; 128], reference_pitch: Float) {
        for i in 0..128 {
            map[i] = Synth::calculate_frequency(i as Float, reference_pitch);
        }
    }

    fn calculate_frequency(key: Float, reference_pitch: Float) -> Float {
        const TWO: Float = 2.0;
        (reference_pitch / 32.0) * (TWO.powf((key - 9.0) / 12.0))
    }

    fn handle_ui_message(&mut self, msg: SynthParam) {
        info!("handle_ui_message - {:?}", msg);
        self.sound.set_parameter(&msg);

        // Let components check if they need to react to a changed
        // parameter.
        match msg.function {
            Parameter::Oscillator => {
                match msg.parameter {
                    Parameter::Wavetable => {
                        // New wavetable has been selected, update all oscillators
                        let osc_id = msg.function_id - 1;
                        self.update_wavetable(osc_id);
                    }
                    Parameter::Routing => {
                        // Oscillator routing has changed
                        let osc_id = msg.function_id - 1;
                        self.update_routing(osc_id);
                    }
                    _ => ()
                }
            }
            Parameter::Delay => {
                match msg.parameter {
                    Parameter::Tone => self.delay.update(&self.sound.delay),
                    Parameter::Sync => self.delay.update_bpm(&mut self.sound.delay, self.sound.patch.bpm),
                    _ => ()
                }
            }
            Parameter::Patch => {
                match msg.parameter {
                    Parameter::Bpm => {
                        self.delay.update_bpm(&mut self.sound.delay, self.sound.patch.bpm);
                    }
                    _ => ()
                }
            }
            _ => ()
        }
    }

    fn update_delay_speed(&mut self) {
    }

    // The assigned wavetable of an oscillator has changed.
    fn update_wavetable(&mut self, osc_id: usize) {
        let id = self.sound.osc[osc_id].wt_osc_data.wavetable;
        info!("Updating oscillator {} to wavetable {}", osc_id, id);
        let result = self.wt_manager.get_table(id);
        match result {
            Some(wt) => {
                self.voice.iter_mut().for_each(|v| v.set_wavetable(osc_id, wt.clone()));
                self.osc_wave[osc_id] = wt.clone();
            }
            None => error!("Unable to find wavetable {}",id),
        }
    }

    fn update_routing(&mut self, osc_id: usize) {
        for v in self.voice.iter_mut() {
            v.update_routing(osc_id, &self.sound.osc[osc_id]);
        }
    }

    fn handle_midi_message(&mut self, msg: MidiMessage) {
        match msg {
            MidiMessage::NoteOn{channel: _, key, velocity} => self.handle_note_on(key, velocity),
            MidiMessage::NoteOff{channel: _, key, velocity} => self.handle_note_off(key, velocity),
            MidiMessage::KeyAT{channel: _, key: _, pressure: _} => (), // Polyphonic aftertouch not supported yet
            MidiMessage::ChannelAT{channel: _, pressure} => self.handle_channel_aftertouch(pressure),
            MidiMessage::Pitchbend{channel: _, pitch} => self.handle_pitch_bend(pitch),
            MidiMessage::ControlChg{channel: _, controller, value} => self.handle_controller(controller, value),
            MidiMessage::ProgramChg{channel: _, program: _} => (), // This shouldn't get here, it's a UI event
            MidiMessage::SongPos{position: _} => (),
            MidiMessage::TimingClock => (),
            MidiMessage::Start => (),
            MidiMessage::Continue => (),
            MidiMessage::Stop => (),
            MidiMessage::ActiveSensing => (),
            MidiMessage::Reset => (),
        }
    }

    fn handle_sound_update(&mut self, sound: &SoundData) {
        self.reset();
        self.sound = *sound;
        self.sound_global = self.sound;
        self.sound_local = self.sound;
        self.update_wavetable(0);
        self.update_wavetable(1);
        self.update_wavetable(2);
        self.update_routing(0);
        self.update_routing(1);
        self.update_routing(2);
    }

    fn handle_wavetable_info(&mut self, mut wt_info: WtInfo) {
        self.wt_manager.load_table(&mut wt_info, self.wt_manager.get_table(0).unwrap(), false);
    }

    /// Received updated BPM by TimingClock MIDI message
    fn handle_bpm(&mut self, bpm: Float) {
        self.sound.patch.bpm = bpm;
        self.delay.update_bpm(&mut self.sound.delay, bpm);
    }

    fn handle_note_on(&mut self, key: u8, velocity: u8) {
        info!("Note: {}", key);
        let freq = self.keymap[key as usize];
        let voice_id = self.select_voice();
        let voice = &mut self.voice[voice_id];
        voice.set_key(key);
        voice.set_freq(freq);
        voice.set_velocity(velocity, self.sound.patch.vel_sens);
        voice.trigger(self.trigger_seq, self.last_clock, &self.sound);
        self.key_stack.push((velocity as u16) << 8 | (key as u16));
        self.num_voices_triggered += 1;
        self.trigger_seq += 1;
        self.voices_playing |= 1 << voice_id;
    }

    fn handle_note_off(&mut self, key: u8, velocity: u8) {
        self.key_stack.remove(self.key_stack.iter().position(|x| *x as u8 == key).expect("Key not found on stack"));
        for v in &mut self.voice {
            if v.is_triggered() && v.key == key {
                if self.sound.patch.play_mode == PlayMode::Poly || self.key_stack.len() == 0 {
                    // In poly mode, or if no other notes are held, we release
                    // the voice.
                    self.num_voices_triggered -= 1;
                    v.key_release(velocity, self.sustain_pedal > 0.0, &self.sound);
                } else {
                    // For Mono and Legato play modes, we continue playing an
                    // older note still on the stack (still triggered).
                    if let Some(new_key) = self.key_stack.pop() {
                        self.handle_note_on(new_key as u8, (new_key >> 8) as u8);
                    } else {
                        panic!("Retrieving note from stack failed.");
                    }
                }
                break;
            }
        }
    }

    fn handle_channel_aftertouch(&mut self, pressure: u8) {
        self.aftertouch = pressure as Float;
    }

    fn handle_pitch_bend(&mut self, value: i16) {
        self.pitch_bend = (value + (value & 0x01)) as Float / 8192.0;
        let inc: Float = 1.059463;
        self.global_state.freq_factor = inc.powf(self.pitch_bend * self.sound.patch.pitchbend);
    }

    // Map controllers with a special function to dedicated parameters
    fn handle_controller(&mut self, ctrl: u8, value: u8) {
        match ctrl {
            0x01 => {
                // Controller 1 = Modulation wheel
                // The ModWheel gets a special handling in that it can both be
                // used as a general purpose controller (handled in Tui) and as a
                // dedicated global modulation source (handled here).
                self.mod_wheel = value as Float;
            }
            0x40 => {
                // Controller 64 = Sustain pedal
                if value >= 64 {
                    self.sustain_pedal = 1.0;
                } else {
                    self.sustain_pedal = 0.0;
                    self.handle_pedal_release();
                }
            }
            _ => (),
        }
    }

    // If any voices have still-running envelopes, trigger the release.
    fn handle_pedal_release(&mut self) {
        for v in &mut self.voice {
            if v.is_running() {
                v.pedal_release(&self.sound);
            }
        }
    }

    // Decide which voice gets to play the next note.
    fn select_voice(&mut self) -> usize {
        match self.sound.patch.play_mode {
            PlayMode::Poly => self.select_voice_poly(),
            PlayMode::Mono => 0,   // Monophonic modes always use voice 0
            PlayMode::Legato => 0,
        }
    }

    fn select_voice_poly(&mut self) -> usize {
        let mut min_trigger_seq = std::u64::MAX;
        let mut min_id = 0;
        for (i, v) in self.voice.iter().enumerate() {
            if !v.is_running() {
                return i;
            }
            if v.trigger_seq < min_trigger_seq {
                min_trigger_seq = v.trigger_seq;
                min_id = i;
            }
        }
        min_id
    }

    // Fill a received buffer with samples from the model oscillator/ envelope.
    //
    // This puts one wave cycle of the currently selected oscillator or
    // envelope or LFO into the buffer.
    //
    fn handle_sample_buffer(&mut self, mut buffer: Vec<Float>, param: SynthParam) {
        let len = buffer.capacity();
        let freq = self.sample_rate as Float / len as Float;
        match param.function {
            Parameter::Oscillator => {
                let osc = &mut self.samplebuff_osc;
                osc.reset(0);
                let osc_id = param.function_id - 1;
                osc.set_wavetable(self.osc_wave[osc_id].clone());
                for i in 0..len {
                    let (mut sample, _) = osc.get_sample(freq, i as i64, &self.sound.osc[osc_id], false);

                    // Apply clipping
                    if self.sound_global.patch.drive > 0.0 {
                        sample = (sample * self.sound_global.patch.drive).tanh();
                    }

                    buffer[i] = sample * self.sound.osc[param.function_id - 1].level;
                }
            },
            Parameter::Envelope => {
                let env_data = &mut self.sound.env[param.function_id - 1];
                let mut len_total = env_data.delay + env_data.attack + env_data.decay + env_data.release;
                if !env_data.looping {
                    len_total += len_total / 3.0; // Add 25% duration for sustain, value is in ms
                }
                let mut release_point = len_total - env_data.release;
                len_total *= 44.1; // Samples per second
                release_point *= 44.1;
                let samples_per_slot = (len_total / len as Float) as usize; // Number of samples per slot in the buffer
                let mut index: usize = 0;
                let mut counter: usize = 0;
                let len_total = len_total as usize;
                let release_point = release_point as usize;
                let mut sample = 0.0;
                let env = &mut self.samplebuff_env;
                env.trigger(0, env_data);
                for i in 0..len_total {
                    if i == release_point {
                        env.release(i as i64, env_data);
                    }
                    sample += env.get_sample(i as i64, env_data);
                    counter += 1;
                    if counter == samples_per_slot {
                        sample /= samples_per_slot as Float;
                        buffer[index] = sample;
                        index += 1;
                        if index == len {
                            index -= 1;
                        }
                        sample = 0.0;
                        counter = 0;
                    }
                }
            },
            Parameter::Lfo | Parameter::GlobalLfo => {
                let lfo = &mut self.samplebuff_lfo;
                let mut sound_copy = if let Parameter::Lfo = param.function {
                    self.sound.lfo[param.function_id - 1]
                } else {
                    self.sound.glfo[param.function_id - 1]
                };
                lfo.reset(0, sound_copy.phase);
                sound_copy.frequency = freq;
                // Get first sample explicitly to reset LFO (for S&H)
                let (sample, _) = lfo.get_sample(0, &sound_copy, true);
                buffer[0] = sample;
                for i in 1..len {
                    let (sample, _) = lfo.get_sample(i as i64, &sound_copy, false);
                    buffer[i] = sample;
                }
            },
            _ => {},
        }
        self.sender.send(UiMessage::SampleBuffer(buffer, param)).unwrap();
    }
}
