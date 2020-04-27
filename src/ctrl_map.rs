/** Maps MIDI controllers to synth parameters. */

use super::Float;
use super::{Parameter, ParamId, ParameterValue, ValueRange};
use super::{SoundData, SoundPatch};
use super::SynthParam;

use log::{info, trace, warn};
use serde::{Serialize, Deserialize};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum MappingType {
    Absolute,
    Relative
}

#[derive(Clone, Copy, Debug)]
pub struct CtrlMapEntry {
    id: ParamId,
    map_type: MappingType,
    val_range: ValueRange,
}

type CtrlHashMap = HashMap<u64, CtrlMapEntry>;

pub struct CtrlMap {
    map: Vec<CtrlHashMap>
}

impl CtrlMap {
    pub fn new() -> CtrlMap {
        // 36 sets of controller mappings (0-9, a-z)
        CtrlMap{map: vec!(CtrlHashMap::new(); 36)}
    }

    /** Reset the map, removing all controller assignments. */
    pub fn reset(&mut self) {
        for m in &mut self.map {
            m.clear();
        }
    }

    /** Add a new mapping entry for a controller.
     *
     * \param set The selected controller map set
     * \param controller The controller number to add
     * \param map_type Type of value change (absolute or relative)
     * \param parameter The parameter changed with this controller
     * \param val_range The valid values for the parameter
     */
    pub fn add_mapping(&mut self,
                      set: usize,
                      controller: u64,
                      map_type: MappingType,
                      parameter: ParamId,
                      val_range: ValueRange) {
        trace!("add_mapping: Set {}, ctrl {}, type {:?}, param {:?}, val range {:?}",
            set, controller, map_type, parameter, val_range);
        self.map[set].insert(controller,
                                 CtrlMapEntry{id: parameter,
                                              map_type: map_type,
                                              val_range: val_range});
    }

    /** Update a value according to the controller value.
     *
     * Uses the parameter's val_range to translate the controller value into
     * a valid parameter value.
     *
     * \param set The selected controller map set
     * \param controller The controller number to look up
     * \param value New value of the controller
     * \param sound Currently active sound
     * \param result SynthParam that receives the changed value
     *
     * \return true if result was updated, false otherwise
     */
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
                let delta = if value >= 126 { -1 } else { 1 };
                result.value = mapping.val_range.add_value(sound_value, delta);
            }
        };
        Ok(result)
    }
}

// ----------------------------------------------
//                  Unit tests
// ----------------------------------------------

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
        self.map.add_mapping(1,
                             ctrl_no,
                             ctrl_type,
                             self.param_id,
                             ValueRange::FloatRange(0.0, 100.0, 1.0));
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
}

#[test]
fn test_controller_without_mapping_returns_no_value() {
    let mut context = TestContext::new();
    assert_eq!(context.handle_controller(1, 50), false);
}

#[test]
fn test_absolute_controller_can_be_added() {
    let mut context = TestContext::new();
    context.add_controller(1, MappingType::Absolute);
    assert_eq!(context.handle_controller(1, 50), true);
}

#[test]
fn test_value_can_be_changed_absolute() {
    let mut context = TestContext::new();
    assert_eq!(context.has_value(92.0), true);
    context.add_controller(1, MappingType::Absolute);
    assert_eq!(context.handle_controller(1, 0), true);
    assert_eq!(context.has_value(0.0), true);
}

#[test]
fn test_relative_controller_can_be_added() {
    let mut context = TestContext::new();
    context.add_controller(1, MappingType::Relative);
    assert_eq!(context.handle_controller(1, 50), true);
}

#[test]
fn test_value_can_be_changed_relative() {
    let mut context = TestContext::new();
    assert_eq!(context.has_value(92.0), true);
    context.add_controller(1, MappingType::Relative);

    // Increase value
    assert_eq!(context.handle_controller(1, 0), true);
    assert_eq!(context.has_value(93.0), true);

    // Decrease value
    assert_eq!(context.handle_controller(1, 127), true);
    assert_eq!(context.has_value(92.0), true);
}

