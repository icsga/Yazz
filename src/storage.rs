use super::SoundData;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct SoundBankInfo {
    sound_data_version: String,
    synth_engine_version: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SoundPatch {
    pub name: String,
    pub data: SoundData
}

impl SoundPatch {
    pub fn new() -> SoundPatch {
        Default::default()
    }
}

impl Default for SoundPatch {
    fn default() -> Self {
        let name = "Init".to_string();
        let mut data = SoundData{..Default::default()};
        data.init();
        SoundPatch{name, data}
    }
}

pub struct SoundBank {
    info: SoundBankInfo,
    sounds: Vec<SoundPatch>,
}

impl SoundBank {
    pub fn new(sound_data_version: &'static str, synth_engine_version: &'static str) -> SoundBank {
        let info = SoundBankInfo{sound_data_version: sound_data_version.to_string(),
                                 synth_engine_version: synth_engine_version.to_string()};
        let sounds = vec!(SoundPatch{..Default::default()}; 128);
        SoundBank{info, sounds}
    }

    pub fn load_bank(&mut self, filename: &str) -> std::io::Result<()> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        self.info.sound_data_version.clear();
        let v = &mut self.info.sound_data_version;
        reader.read_line(v)?;
        if v.ends_with('\n') {
            v.pop();
            if v.ends_with('\r') {
                v.pop();
            }
        }
        info!("Read sound file version {}", self.info.sound_data_version);
        let mut serialized = String::new();
        reader.read_to_string(&mut serialized)?;
        self.sounds = serde_json::from_str(&serialized).unwrap();
        Ok(())
    }

    pub fn save_bank(&self, filename: &str) -> std::io::Result<()> {
        let mut file = File::create(filename)?;
        file.write_all(self.info.sound_data_version.as_bytes())?;
        file.write_all("\n".as_bytes())?;
        let serialized = serde_json::to_string_pretty(&self.sounds).unwrap();
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }

    pub fn load_sound(&mut self, sound_index: u32, filename: &str) {
    }

    pub fn save_sound(&self, sound_index: u32, filename: &str) {
    }

    pub fn get_sound(&self, sound_index: usize) -> &SoundPatch {
        &self.sounds[sound_index]
    }

    pub fn set_sound(&mut self, sound_index: usize, to_sound: &SoundPatch) {
        self.sounds[sound_index].name = to_sound.name.clone();
        self.sounds[sound_index].data = to_sound.data;
    }
}
