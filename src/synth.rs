use super::Delay;
use super::{SynthMessage, UiMessage};
use super::{Envelope, EnvelopeData};
use super::Lfo;
use super::{MessageType, MidiMessage};
use super::{Modulator, ModData};
use super::{MultiOscData, MultiOscillator};
use super::{Parameter, ParameterValue, SynthParam};
use super::SoundData;
use super::voice::Voice;
use super::SampleGenerator;
use super::Float;

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

use log::{info, trace, warn};

const NUM_VOICES: usize = 32;
const NUM_KEYS: usize = 127;
const REF_FREQUENCY: Float = 440.0;
const NUM_MODULATORS: usize = 20;
pub const NUM_GLOBAL_LFOS: usize = 2;

pub enum Synth2UIMessage {
    Param(SynthParam),
    Control(u32),
    Log(String)
}

pub struct Synth {
    // Configuration
    sample_rate: u32,
    sound: Arc<Mutex<SoundData>>, // Sound patch as loaded from disk
    sound_global: SoundData,      // Sound with global modulators applied
    sound_local: SoundData,       // Sound with voice-local modulators applied
    modulators: Vec<Modulator>,
    keymap: [Float; NUM_KEYS],

    // Signal chain
    voice: [Voice; NUM_VOICES],
    delay: Delay,
    glfo: [Lfo; NUM_GLOBAL_LFOS],

    // Current state
    num_voices_triggered: u32,
    voices_playing: u32, // Bitmap with currently playing voices
    trigger_seq: u64,
    last_clock: i64,
    sender: Sender<UiMessage>,
}

impl Synth {
    pub fn new(sample_rate: u32, sender: Sender<UiMessage>) -> Self {
        let mut sound = SoundData::new();
        sound.init();
        let sound = Arc::new(Mutex::new(sound));
        let mut sound_global = SoundData::new();
        let mut sound_local = SoundData::new();
        sound_global.init();
        sound_local.init();
        let modulators = vec!{};
        let voice = [
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
        ];
        let delay = Delay::new(sample_rate);
        let glfo = [
            Lfo::new(sample_rate), Lfo::new(sample_rate)
        ];
        let mut keymap: [Float; NUM_KEYS] = [0.0; NUM_KEYS];
        Synth::calculate_keymap(&mut keymap, REF_FREQUENCY);
        let num_voices_triggered = 0;
        let voices_playing = 0;
        let trigger_seq = 0;
        let last_clock = 0i64;
        let mut synth = Synth{sample_rate,
                          sound,
                          sound_global,
                          sound_local,
                          modulators,
                          keymap,
                          voice,
                          delay,
                          glfo,
                          num_voices_triggered,
                          voices_playing,
                          trigger_seq,
                          last_clock,
                          sender};

        // Add test modulator
        let mod_data = ModData{
            source_func: Parameter::Lfo,
            source_func_id: 1,
            dest_func: Parameter::Oscillator,
            dest_func_id: 1,
            dest_param: Parameter::Blend,
            amount: 0.2
        };
        synth.add_modulator(&mod_data);
        let mod_data2 = ModData{
            source_func: Parameter::GlobalLfo,
            source_func_id: 1,
            dest_func: Parameter::Delay,
            dest_func_id: 1,
            dest_param: Parameter::Time,
            amount: 0.001
        };
        //synth.add_modulator(&mod_data2);
        synth
    }

    /* Starts a thread for receiving UI and MIDI messages. */
    pub fn run(synth: Arc<Mutex<Synth>>, synth_receiver: Receiver<SynthMessage>) -> std::thread::JoinHandle<()> {
        let handler = spawn(move || {
            loop {
                let msg = synth_receiver.recv().unwrap();
                let mut locked_synth = synth.lock().unwrap();
                match msg {
                    SynthMessage::Param(m) => locked_synth.handle_ui_message(m),
                    SynthMessage::ParamQuery(m) => locked_synth.handle_ui_query(m),
                    SynthMessage::Midi(m)  => locked_synth.handle_midi_message(m),
                    SynthMessage::SampleBuffer(m, p) => locked_synth.handle_sample_buffer(m, p),
                }
            }
        });
        handler
    }

    fn add_modulator(&mut self, data: &ModData) {
        let m = Modulator::new(data);
        self.modulators.push(m);
        let sound = &self.sound.lock().unwrap();
        self.sound_global = **sound; // Initialize values to current sound. TODO: Only needed once if parameter updates update all three sounds
        self.sound_local = **sound;
    }

    /* Called by the audio engine to get the next sample to be output. */
    pub fn get_sample(&mut self, sample_clock: i64) -> Float {
        let mut value: Float = 0.0;
        let sound = &self.sound.lock().unwrap();

        // Prepare modulation values
        // =========================
        for m in self.modulators.iter() {

            // Get modulator source output
            let mod_val: Float = match m.source_func {
                Parameter::GlobalLfo => {
                    let (val, reset) = self.glfo[m.source_func_id].get_sample(sample_clock, &self.sound_global.glfo[m.source_func_id], false);
                    val
                },
                _ => 0.0, // TODO: This also sets non-global vars, optimize that
            } * m.scale + m.offset;

            // Get current value of target parameter
            let param = SynthParam{function: m.dest_func, function_id: m.dest_func_id, parameter: m.dest_param, value: ParameterValue::NoValue};
            let current_val = sound.get_value(&param);
            let mut val = match current_val {
                ParameterValue::Int(x) => x as Float,
                ParameterValue::Float(x) => x,
                _ => panic!()
            };

            // Update value if mod source is global
            if m.is_global {
                val += mod_val;
            }

            // Write value to global sound data
            let param = SynthParam{function: m.dest_func, function_id: m.dest_func_id, parameter: m.dest_param, value: ParameterValue::Float(val)};
            self.sound_global.set_parameter(&param);
        }
        //info!("{:?}\n{:?}\n{:?}", sound, self.sound_global, self.sound_local);

        if self.voices_playing > 0 {
            for i in 0..32 {
                if self.voices_playing & (1 << i) > 0 {
                    value += self.voice[i].get_sample(sample_clock, &self.modulators, &mut self.sound_local, &self.sound_global);
                }
            }
        }
        value = self.delay.process(value, sample_clock, &self.sound_global.delay);
        //value = self.delay.process(value, sample_clock, &sound.delay);
        self.last_clock = sample_clock;
        value
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
    fn calculate_keymap(map: &mut[Float; 127], reference_pitch: Float) {
        for i in 0..127 {
            let two: Float = 2.0;
            map[i] = (reference_pitch / 32.0) * (two.powf((i as Float - 9.0) / 12.0));
        }
    }

    /* Handles a message received from the UI. */
    fn handle_ui_message(&mut self, msg: SynthParam) {
        let mut sound = self.sound.lock().unwrap();
        sound.set_parameter(&msg);
        self.sound_global = *sound;
        self.sound_local = *sound;
        //info!("handle_ui_message - {:?}\n{:?}\n{:?}", sound, self.sound_global, self.sound_local);
        // Let all components check if they need to react to a changed
        // parameter. This allows us to keep the processing out of the
        // audio engine thread.
        self.voice[0].filter[0].update(&mut sound.filter[0]);
    }

    /* Handles a parameter query received from the UI. */
    fn handle_ui_query(&mut self, mut msg: SynthParam) {
        {
            let sound = self.sound.lock().unwrap();
            sound.insert_value(&mut msg);
        }
        self.sender.send(UiMessage::Param(msg)).unwrap();
    }

    /* Handles a received MIDI message. */
    fn handle_midi_message(&mut self, msg: MidiMessage) {
        let channel = msg.mtype & 0x0F;
        let mtype: u8 = msg.mtype & 0xF0;
        match mtype {
            0x90 => { // NoteOn
                let freq = self.keymap[msg.param as usize];
                let voice_id = self.select_voice();
                let voice = &mut self.voice[voice_id];
                voice.set_key(msg.param);
                voice.set_freq(freq);
                {
                    let sound = self.sound.lock().unwrap();
                    voice.trigger(self.trigger_seq, self.last_clock, &sound);
                }
                self.num_voices_triggered += 1;
                self.trigger_seq += 1;
                self.voices_playing |= 1 << voice_id;
            }
            0x80 => { // NoteOff
                for (i, v) in self.voice.iter_mut().enumerate() {
                    if v.is_triggered() && v.key == msg.param {
                        self.num_voices_triggered -= 1;
                        let sound = self.sound.lock().unwrap();
                        v.release(&sound);
                        break;
                    }
                }
            }
            _ => ()
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
        match param.function {
            Parameter::Oscillator => {
                let mut osc = MultiOscillator::new(44100, param.function_id - 1);
                osc.reset(0);
                let sound = self.sound.lock().unwrap();
                for i in 0..buffer.capacity() {
                    let (sample, complete) = osc.get_sample(440.0, i as i64, &sound, false);
                    buffer[i] = sample;
                }
            },
            Parameter::Envelope => {
                // Calculate lenth of envelope
                let mut sound = self.sound.lock().unwrap();
                let env_data = &mut sound.env[param.function_id - 1];
                let mut len_total = env_data.attack + env_data.decay + env_data.release;
                len_total += len_total / 3.0; // Add 25% duration for sustain, value is in ms
                let mut release_point = len_total - env_data.release;
                len_total *= 44.1; // Samples per second
                release_point *= 44.1;
                let samples_per_slot = (len_total / buffer.capacity() as Float) as usize; // Number of samples per slot in the buffer
                let mut index: usize = 0;
                let mut counter: usize = 0;
                let mut env = Envelope::new(44100.0);
                let len_total = len_total as usize;
                let release_point = release_point as usize;
                let mut sample = 0.0;
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
                        if index == buffer.capacity() {
                            index -= 1;
                        }
                        sample = 0.0;
                        counter = 0;
                    }
                }
            },
            _ => {},
        }
        self.sender.send(UiMessage::SampleBuffer(buffer, param)).unwrap();
    }
}
