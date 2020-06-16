//! Maps MIDI controllers to synth parameters.

use super::{ParamId, MenuItem};
use super::SoundData;
use super::SynthParam;
use super::ValueRange;

use log::{info, trace};
use serde::{Serialize, Deserialize};

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum MappingType {
    None,
    Absolute,
    Relative
}

#[derive(Clone, Copy, Debug)]
pub struct CtrlMapEntry {
    id: ParamId,
    map_type: MappingType,
    val_range: &'static ValueRange,
}

/// ValueRange contains a reference, so it can't be stored easily. Instead we
/// store only ParamId and MappingType and rely on the TUI to look up the
/// value range when loading the data.
///
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct CtrlMapStorageEntry {
    id: ParamId,
    map_type: MappingType,
}

type CtrlHashMap = HashMap<u64, CtrlMapEntry>;
type CtrlHashMapStorage = HashMap<u64, CtrlMapStorageEntry>;

pub struct CtrlMap {
    map: Vec<CtrlHashMap>
}

impl CtrlMap {
    pub fn new() -> CtrlMap {
        // 36 sets of controller mappings (0-9, a-z)
        CtrlMap{map: vec!(CtrlHashMap::new(); 36)}
    }

    /// Reset the map, removing all controller assignments.
    pub fn reset(&mut self) {
        for m in &mut self.map {
            m.clear();
        }
    }

    /// Add a new mapping entry for a controller.
    ///
    /// set: The selected controller map set
    /// controller: The controller number to add
    /// map_type: Type of value change (absolute or relative)
    /// parameter: The parameter changed with this controller
    /// val_range: The valid values for the parameter
    ///
    pub fn add_mapping(&mut self,
                      set: usize,
                      controller: u64,
                      map_type: MappingType,
                      parameter: ParamId,
                      val_range: &'static ValueRange) {
        trace!("add_mapping: Set {}, ctrl {}, type {:?}, param {:?}, val range {:?}",
            set, controller, map_type, parameter, val_range);
        self.map[set].insert(controller,
                             CtrlMapEntry{id: parameter,
                                          map_type: map_type,
                                          val_range: val_range});
    }

    /// Delete all mappings for a parameter.
    ///
    /// Returns true if one or more mappings were deleted, false otherwise
    pub fn delete_mapping(&mut self,
                          set: usize,
                          parameter: ParamId) -> bool {
        trace!("delete_mapping: Set {}, param {:?}", set, parameter);
        let mut controller: u64;
        let mut found = false;
        loop {
            controller = 0;
            for (key, val) in self.map[set].iter() {
                if val.id == parameter {
                    controller = *key;
                }
            }
            if controller > 0 {
                self.map[set].remove(&controller);
                found = true;
            } else {
                break;
            }
        }
        found
    }

    /// Update a value according to the controller value.
    ///
    /// Uses the parameter's val_range to translate the controller value into
    /// a valid parameter value.
    ///
    /// set: The selected controller map set
    /// controller: The controller number to look up
    /// value: New value of the controller
    /// sound: Currently active sound
    /// result: SynthParam that receives the changed value
    ///
    /// Return true if result was updated, false otherwise
    ///
    pub fn get_value(&self,
                    set: usize,
                    controller: u64,
                    value: u64,
                    sound: &SoundData) -> Result<SynthParam, ()> {
        // Get mapping
        if !self.map[set].contains_key(&controller) {
            return Err(());
        }
        let mapping = &self.map[set][&controller];
        let mut result = SynthParam::new_from(&mapping.id);
        match mapping.map_type {
            MappingType::Absolute => {
                // For absolute: Translate value
                let trans_val = mapping.val_range.translate_value(value);
                result.value = trans_val;
            }
            MappingType::Relative => {
                // For relative: Increase/ decrease value
                let sound_value = sound.get_value(&mapping.id);
                let delta = if value >= 64 { -1 } else { 1 };
                result.value = mapping.val_range.add_value(sound_value, delta);
            }
            MappingType::None => panic!(),
        };
        Ok(result)
    }

    // Load controller mappings from file
    pub fn load(&mut self, filename: &str) -> std::io::Result<()> {
        info!("Reading controller mapping from {}", filename);
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        self.reset();
        let mut serialized = String::new();
        reader.read_to_string(&mut serialized)?;
        let storage_map: Vec<CtrlHashMapStorage> = serde_json::from_str(&serialized).unwrap();

        for i in 0..storage_map.len() {
            for (key, value) in &storage_map[i] {
                let val_range = MenuItem::get_val_range(value.id.function, value.id.parameter);
                self.map[i].insert(*key, CtrlMapEntry{id: value.id, map_type: value.map_type, val_range: val_range});
            }
        }
        Ok(())
    }

    // Store controller mappings to file
    pub fn save(&self, filename: &str) -> std::io::Result<()> {
        info!("Writing controller mapping to {}", filename);

        // Transfer data into serializable format
        let mut storage_map = vec!(CtrlHashMapStorage::new(); 36);
        for i in 0..self.map.len() {
            for (key, value) in &self.map[i] {
                storage_map[i].insert(*key, CtrlMapStorageEntry{id: value.id, map_type: value.map_type});
            }
        }

        let mut file = File::create(filename)?;
        let serialized = serde_json::to_string_pretty(&storage_map).unwrap();
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
}

// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

#[cfg(test)]
mod tests {

use super::{CtrlMap, MappingType};
use super::super::Float;
use super::super::{Parameter, ParamId, ParameterValue, MenuItem};
use super::super::{SoundData, SoundPatch};
use super::super::SynthParam;

use std::cell::RefCell;
use std::rc::Rc;

struct TestContext {
    map: CtrlMap,
    sound: SoundPatch,
    sound_data: Rc<RefCell<SoundData>>,
    param_id: ParamId,
    synth_param: SynthParam,
}

impl TestContext {
    fn new() -> TestContext {
        let map = CtrlMap::new();
        let sound = SoundPatch::new();
        let sound_data = Rc::new(RefCell::new(sound.data));
        let param_id = ParamId::new(Parameter::Oscillator, 1, Parameter::Level);
        let synth_param = SynthParam::new(Parameter::Oscillator,
                                          1,
                                          Parameter::Level,
                                          ParameterValue::Float(0.0));
        TestContext{map, sound, sound_data, param_id, synth_param}
    }

    pub fn add_controller(&mut self, ctrl_no: u64, ctrl_type: MappingType) {
        let val_range = MenuItem::get_val_range(self.param_id.function, self.param_id.parameter);
        self.map.add_mapping(1,
                             ctrl_no,
                             ctrl_type,
                             self.param_id,
                             val_range);
    }

    pub fn handle_controller(&mut self, ctrl_no: u64, value: u64) -> bool {
        let sound_data = &mut self.sound_data.borrow_mut();
        match self.map.get_value(1, ctrl_no, value, sound_data) {
            Ok(result) => {
                sound_data.set_parameter(&result);
                true
            }
            Err(()) => false
        }
    }

    pub fn has_value(&mut self, value: Float) -> bool {
        let pval = self.sound_data.borrow().get_value(&self.param_id);
        if let ParameterValue::Float(x) = pval {
            println!("\nIs: {} Expected: {}", x, value);
            x == value
        } else {
            false
        }
    }

    pub fn delete_controller(&mut self) -> bool {
        self.map.delete_mapping(1, self.param_id)
    }
}

#[test]
fn controller_without_mapping_returns_no_value() {
    let mut context = TestContext::new();
    assert_eq!(context.handle_controller(1, 50), false);
}

#[test]
fn absolute_controller_can_be_added() {
    let mut context = TestContext::new();
    context.add_controller(1, MappingType::Absolute);
    assert_eq!(context.handle_controller(1, 50), true);
}

#[test]
fn value_can_be_changed_absolute() {
    let mut context = TestContext::new();
    assert_eq!(context.has_value(50.0), true);
    context.add_controller(1, MappingType::Absolute);
    assert_eq!(context.handle_controller(1, 0), true);
    assert_eq!(context.has_value(0.0), true);
}

#[test]
fn relative_controller_can_be_added() {
    let mut context = TestContext::new();
    context.add_controller(1, MappingType::Relative);
    assert_eq!(context.handle_controller(1, 50), true);
}

#[test]
fn value_can_be_changed_relative() {
    let mut context = TestContext::new();
    assert_eq!(context.has_value(50.0), true);
    context.add_controller(1, MappingType::Relative);

    // Increase value
    assert_eq!(context.handle_controller(1, 0), true);
    assert_eq!(context.has_value(51.0), true);

    // Decrease value
    assert_eq!(context.handle_controller(1, 127), true);
    assert_eq!(context.has_value(50.0), true);
}

#[test]
fn mapping_can_be_deleted() {
    let mut context = TestContext::new();
    context.add_controller(1, MappingType::Relative);
    assert_eq!(context.handle_controller(1, 127), true);
    assert_eq!(context.delete_controller(), true);
    assert_eq!(context.handle_controller(1, 127), false);
}

#[test]
fn nonexisting_mapping_isnt_deleted() {
    let mut context = TestContext::new();
    assert_eq!(context.delete_controller(), false);
}

} // mod tests
