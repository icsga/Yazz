use super::Delay;
use super::{SynthMessage, UiMessage};
use super::{Envelope, EnvelopeData};
use super::Lfo;
use super::MidiMessage;
use super::ModData;
use super::{Parameter, ParameterValue, ParamId, SynthParam, MenuItem};
use super::SoundData;
use super::voice::Voice;
use super::SampleGenerator;
use super::Float;
use super::{WtOsc, WtManager, Wavetable, WavetableRef, WtInfo};

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};
use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

const NUM_VOICES: usize = 32;
const NUM_KEYS: usize = 128;
pub const NUM_MODULATORS: usize = 16;
pub const NUM_GLOBAL_LFOS: usize = 2;
const REF_FREQUENCY: Float = 440.0;

pub enum Synth2UIMessage {
    Param(SynthParam),
    Control(u32),
    Log(String)
}

// Data of the currently selected sound patch
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
pub struct PatchData {
    pub level: Float,
    pub drive: Float,
    pub pitchbend: Float, // Range of the pitchwheel
    pub vel_sens: Float,  // Velocity sensitivity
}

impl PatchData {
    pub fn init(&mut self) {
        self.level = 90.0;
        self.drive = 0.0;
        self.pitchbend = 2.0;
        self.vel_sens = 1.0;
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
    sender: Sender<UiMessage>,
    global_state: SynthState,

    // Extra oscillators to display the waveshape
    samplebuff_osc: WtOsc,
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
        let wt_manager = WtManager::new(sample_rate as Float);
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
            sample_rate: sample_rate,
            sound: sound,
            sound_global: sound_global,
            sound_local: sound_local,
            keymap: keymap,
            wt_manager: wt_manager,
            voice: voice,
            delay: Delay::new(sample_rate),
            glfo:glfo,
            num_voices_triggered: 0,
            voices_playing: 0,
            trigger_seq: 0,
            last_clock: 0i64,
            pitch_bend: 0.0,
            mod_wheel: 0.0,
            aftertouch: 0.0,
            sender: sender,
            global_state: SynthState{freq_factor: 1.0},
            samplebuff_osc: WtOsc::new(sample_rate, 0, default_table.clone()),
            samplebuff_env: Envelope::new(sample_rate as Float),
            samplebuff_lfo: Lfo::new(sample_rate),
            osc_wave: osc_wave,
        }
    }

    /* Starts a thread for receiving UI and MIDI messages. */
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

    /* Get global modulation values.
     *
     * Calculates the values for global modulation sources and applies them to
     * the global sound data.
     */
    fn get_mod_values(&mut self, sample_clock: i64) {
        // Reset the global sound copy to the main sound, discarding values that
        // were modulated for the previous sample. Complete copy is faster than
        // looping over the modulators.
        self.sound_global = self.sound;

        // Then apply global modulators
        for m in self.sound.modul.iter() {
            if !m.active || !m.is_global {
                continue;
            }

            // Get modulator source output
            let mod_val: Float = match m.source_func {
                Parameter::GlobalLfo => {
                    let (val, reset) = self.glfo[m.source_func_id - 1].get_sample(sample_clock, &self.sound_global.glfo[m.source_func_id - 1], false);
                    val
                },
                Parameter::Aftertouch => self.aftertouch,
                Parameter::Pitchbend => self.pitch_bend,
                Parameter::ModWheel => self.mod_wheel,
                _ => 0.0,
            } * m.scale;


            // Get current value of target parameter
            let param = ParamId{function: m.target_func, function_id: m.target_func_id, parameter: m.target_param};
            let mut current_val = self.sound_global.get_value(&param);
            let mut val = current_val.as_float();

            // Update value
            let dest_range = MenuItem::get_val_range(param.function, param.parameter);
            val = dest_range.safe_add(val, mod_val);

            // Update parameter in global sound data
            current_val.set_from_float(val);
            let param = SynthParam{function: m.target_func, function_id: m.target_func_id, parameter: m.target_param, value: current_val};
            self.sound_global.set_parameter(&param);
        }
    }

    /* Called by the audio engine to get the next sample to be output. */
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
        value = self.delay.process(value, sample_clock, &self.sound_global.delay);

        value *= self.sound_global.patch.level;

        self.last_clock = sample_clock;
        (value, value)
    }

    /* Update the bitmap with currently active voices. */
    pub fn update(&mut self) {
        self.voices_playing = 0;
        for (i, v) in self.voice.iter_mut().enumerate() {
            if v.is_running() {
                self.voices_playing |= 1 << i;
            }
        }
    }

    /* Calculates the frequencies for the default keymap with equal temperament. */
    fn calculate_keymap(map: &mut[Float; 128], reference_pitch: Float) {
        for i in 0..128 {
            let two: Float = 2.0;
            map[i] = (reference_pitch / 32.0) * (two.powf((i as Float - 9.0) / 12.0));
        }
    }

    fn handle_ui_message(&mut self, msg: SynthParam) {
        info!("handle_ui_message - {:?}", msg);
        self.sound.set_parameter(&msg);

        // Let components check if they need to react to a changed
        // parameter.
        match msg.function {
            Parameter::Oscillator => {
                if msg.parameter == Parameter::Wavetable {
                    // New wavetable has been selected, update all oscillators
                    let osc_id = msg.function_id - 1;
                    let id = self.sound.osc[osc_id].wavetable;
                    info!("Updating oscillator {} to wavetable {}", osc_id, id);
                    let wt = self.wt_manager.get_table(id).unwrap();
                    for v in self.voice.iter_mut() {
                        v.set_wavetable(osc_id, wt.clone());
                    }
                    self.osc_wave[osc_id] = wt.clone();
                }
            }
            Parameter::Delay => self.delay.update(&self.sound.delay),
            _ => ()
        }
    }

    fn handle_midi_message(&mut self, msg: MidiMessage) {
        match msg {
            MidiMessage::NoteOn{channel, key, velocity} => self.handle_note_on(key, velocity),
            MidiMessage::NoteOff{channel, key, velocity} => self.handle_note_off(key, velocity),
            MidiMessage::KeyAT{channel, key, pressure} => (), // Polyphonic aftertouch not supported yet
            MidiMessage::ChannelAT{channel, pressure} => self.handle_channel_aftertouch(pressure),
            MidiMessage::Pitchbend{channel, pitch} => self.handle_pitch_bend(pitch),
            MidiMessage::ControlChg{channel, controller, value} => self.handle_controller(controller, value),
            MidiMessage::ProgramChg{channel, program} => (), // This shouldn't get here, it's a UI event
        }
    }

    fn handle_sound_update(&mut self, sound: &SoundData) {
        self.sound = *sound;
        self.sound_global = self.sound;
        self.sound_local = self.sound;
    }

    fn handle_wavetable_info(&mut self, wt_info: WtInfo) {
        self.wt_manager.add_table(wt_info);
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
        self.num_voices_triggered += 1;
        self.trigger_seq += 1;
        self.voices_playing |= 1 << voice_id;
    }

    fn handle_note_off(&mut self, key: u8, velocity: u8) {
        for (i, v) in self.voice.iter_mut().enumerate() {
            if v.is_triggered() && v.key == key {
                self.num_voices_triggered -= 1;
                v.release(velocity, &self.sound);
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

    fn handle_controller(&mut self, ctrl: u8, value: u8) {
        if ctrl == 0x01 {
            // The ModWheel gets a special handling in that it can both be
            // used as a general purpose controller (handled in Tui) and as a
            // dedicated global modulation source (handled here).
            self.mod_wheel = value as Float;
        }
    }

    /* Decide which voice gets to play the next note. */
    fn select_voice(&mut self) -> usize {
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

    /* Fill a received buffer with samples from the model oscillator.
     *
     * This puts one wave cycle of the currently active sound into the buffer.
     */
    fn handle_sample_buffer(&mut self, mut buffer: Vec<Float>, param: SynthParam) {
        let len = buffer.capacity();
        let freq = self.sample_rate as Float / len as Float;
        match param.function {
            Parameter::Oscillator => {
                let osc = &mut self.samplebuff_osc;
                osc.reset(0);
                osc.id = param.function_id - 1;
                osc.set_wavetable(self.osc_wave[osc.id].clone());
                for i in 0..len {
                    let (mut sample, complete) = osc.get_sample(freq, i as i64, &self.sound, false);

                    // Apply clipping
                    if self.sound_global.patch.drive > 0.0 {
                        sample = (sample * self.sound_global.patch.drive).tanh();
                    }

                    buffer[i] = sample * self.sound.osc[param.function_id - 1].level;
                }
            },
            Parameter::Envelope => {
                let env_data = &mut self.sound.env[param.function_id - 1];
                let mut len_total = env_data.attack + env_data.decay + env_data.release;
                len_total += len_total / 3.0; // Add 25% duration for sustain, value is in ms
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
            Parameter::Lfo => {
                let lfo = &mut self.samplebuff_lfo;
                lfo.reset(0);
                let mut sound_copy = self.sound.lfo[param.function_id - 1];
                sound_copy.frequency = freq;
                // Get first sample explicitly to reset LFO (for S&H)
                let (sample, complete) = lfo.get_sample(0, &sound_copy, true);
                buffer[0] = sample;
                for i in 1..len {
                    let (sample, complete) = lfo.get_sample(i as i64, &sound_copy, false);
                    buffer[i] = sample;
                }
            },
            Parameter::GlobalLfo => {
                let lfo = &mut self.samplebuff_lfo;
                lfo.reset(0);
                let mut sound_copy = self.sound.glfo[param.function_id - 1];
                sound_copy.frequency = freq;
                // Get first sample explicitly to reset LFO (for S&H)
                let (sample, complete) = lfo.get_sample(0, &sound_copy, true);
                buffer[0] = sample;
                for i in 1..len {
                    let (sample, complete) = lfo.get_sample(i as i64, &sound_copy, false);
                    buffer[i] = sample;
                }
            },
            _ => {},
        }
        self.sender.send(UiMessage::SampleBuffer(buffer, param)).unwrap();
    }
}
