use super::SoundData;
use super::WtInfo;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SoundBankInfo {
    sound_data_version: String,
    synth_engine_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundBank {
    info: SoundBankInfo,     // Binary and sound version
    sounds: Vec<SoundPatch>, // List of sound patches
    pub wt_list: Vec<WtInfo>     // List of available wavetables
}

impl SoundBank {
    pub fn new(sound_data_version: &'static str, synth_engine_version: &'static str) -> SoundBank {
        let info = SoundBankInfo{sound_data_version: sound_data_version.to_string(),
                                 synth_engine_version: synth_engine_version.to_string()};
        let sounds = vec!(SoundPatch{..Default::default()}; 128);
        let wt_list: Vec<WtInfo> = Vec::new();
        SoundBank{info, sounds, wt_list}
    }

    pub fn load_bank(&mut self, filename: &str) -> std::io::Result<()> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        let mut serialized = String::new();
        reader.read_to_string(&mut serialized)?;
        let result: Result<SoundBank, serde_json::error::Error> = serde_json::from_str(&serialized);
        if let Ok(data) = result {
            *self = data;
        }
        Ok(())
    }

    pub fn save_bank(&self, filename: &str) -> std::io::Result<()> {
        let mut file = File::create(filename)?;
        let serialized = serde_json::to_string_pretty(&self).unwrap();
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }

    pub fn get_sound(&self, sound_index: usize) -> &SoundPatch {
        &self.sounds[sound_index]
    }

    pub fn set_sound(&mut self, sound_index: usize, to_sound: &SoundPatch) {
        self.sounds[sound_index].name = to_sound.name.clone();
        self.sounds[sound_index].data = to_sound.data;
    }
}
