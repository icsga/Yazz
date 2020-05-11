use super::ParamId;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkerManager {
    marker: HashMap<char, ParamId>,
}

impl MarkerManager {
    pub fn new() -> Self {
        let mut mm = MarkerManager{marker: HashMap::new()};
        mm.load().unwrap();
        mm
    }

    pub fn add(&mut self, marker: char, param: ParamId) {
        self.marker.insert(marker, param);
        self.store().unwrap();
    }

    pub fn get(&self, marker: char) -> Option<ParamId> {
        if self.marker.contains_key(&marker) {
            Some(self.marker[&marker])
        } else {
            None
        }
    }

    pub fn load(&mut self) -> std::io::Result<()> {
        let result = File::open("Yazz_Markers.ysn");
        match result {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let mut serialized = String::new();
                reader.read_to_string(&mut serialized)?;
                let result: Result<MarkerManager, serde_json::error::Error> = serde_json::from_str(&serialized);
                if let Ok(data) = result {
                    *self = data;
                }
            }
            Err(err) => info!("Error loading marker file: {}", err),
        }
        Ok(())
    }

    pub fn store(&self) -> std::io::Result<()> {
        let mut file = File::create("Yazz_Markers.ysn")?;
        let serialized = serde_json::to_string_pretty(&self).unwrap();
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
}

