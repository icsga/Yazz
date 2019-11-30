use serde::{Serialize, Deserialize};
use super::SoundData;

use std::fs;
use std::fs::File;
use std::io::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct SoundBankInfo {
    sound_data_version: String,
    synth_engine_version: String,
}

#[derive(Serialize, Deserialize)]
pub struct SoundBank {
    info: SoundBankInfo,
    sounds: Vec<SoundData>,
}

impl SoundBank {
    pub fn new(sound_data_version: &'static str, synth_engine_version: &'static str) -> SoundBank {
        let info = SoundBankInfo{sound_data_version: sound_data_version.to_string(),
                                 synth_engine_version: synth_engine_version.to_string()};
        let mut sounds = vec!(SoundData{..Default::default()}; 128);
        for sound in sounds.iter_mut() {
            sound.init();
        }
        SoundBank{info, sounds}
    }

    pub fn load_bank(&mut self, filename: &str) -> std::io::Result<()> {
        let mut file = File::open(filename)?;
        let mut serialized = String::new();
        file.read_to_string(&mut serialized)?;
        self.sounds = serde_json::from_str(&serialized).unwrap();
        Ok(())
    }

    pub fn save_bank(&self, filename: &str) -> std::io::Result<()> {
        let serialized = serde_json::to_string_pretty(&self.sounds).unwrap();
        fs::write(filename, serialized)?;
        Ok(())
    }

    pub fn load_sound(&mut self, sound_index: u32, filename: &str) {
    }

    pub fn save_sound(&self, sound_index: u32, filename: &str) {
    }

    pub fn get_sound(&self, sound_index: usize) -> &SoundData {
        &self.sounds[sound_index]
    }

    pub fn set_sound(&mut self, sound_index: usize, to_sound: &SoundData) {
        self.sounds[sound_index] = *to_sound;
    }
}
