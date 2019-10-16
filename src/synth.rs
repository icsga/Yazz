use super::{SynthMessage, UiMessage};
use super::EnvelopeData;
use super::{MessageType, MidiMessage};
use super::{MultiOscData, MultiOscillator};
use super::{Parameter, ParameterValue, SynthParam};
use super::SoundData;
use super::voice::Voice;
use super::SampleGenerator;

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crossbeam_channel::unbounded;
use crossbeam_channel::{Sender, Receiver};

pub enum Synth2UIMessage {
    Param(SynthParam),
    Control(u32),
    Log(String)
}

pub struct Synth {
    sample_rate: u32,
    sound: Arc<Mutex<SoundData>>,
    voice: [Voice; 32],
    keymap: [f32; 127],
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
        let voice = [
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
            Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate), Voice::new(sample_rate),
        ];
        let mut keymap: [f32; 127] = [0.0; 127];
        Synth::calculate_keymap(&mut keymap, 440.0);
        let num_voices_triggered = 0;
        let voices_playing = 0;
        let trigger_seq = 0;
        let last_clock = 0i64;
        Synth{sample_rate, sound, voice, keymap, num_voices_triggered, voices_playing, trigger_seq, last_clock, sender}
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
                    SynthMessage::WaveBuffer(m) => locked_synth.handle_wave_buffer(m),
                }
            }
        });
        handler
    }

    /* Called by the audio engine to get the next sample to be output. */
    pub fn get_sample(&mut self, sample_clock: i64) -> f32 {
        let mut value: f32 = 0.0;
        if self.voices_playing > 0 {
            let sound = &self.sound.lock().unwrap();
            for i in 0..32 {
                if self.voices_playing & (1 << i) > 0 {
                    value += self.voice[i].get_sample(sample_clock, sound);
                }
            }
        }
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
    fn calculate_keymap(map: &mut[f32; 127], reference_pitch: f32) {
        for i in 0..127 {
           map[i] = (reference_pitch / 32.0) * (2.0f32.powf((i as f32 - 9.0) / 12.0));
        }
    }

    /* Handles a message received from the UI. */
    fn handle_ui_message(&mut self, msg: SynthParam) {
        let mut sound = self.sound.lock().unwrap();
        sound.set_parameter(msg);
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
    fn handle_wave_buffer(&mut self, mut buffer: Vec<f32>) {
        let len = buffer.capacity();
        let mut osc = MultiOscillator::new(44100, 0);
        let sound = self.sound.lock().unwrap();
        for i in 0..buffer.capacity() {
            let (sample, complete) = osc.get_sample(440.0, i as i64, &sound, false);
            buffer[i] = sample;
        }
        self.sender.send(UiMessage::WaveBuffer(buffer)).unwrap();
    }
}
