/*
- Controller input is absolute or relative
- Relative translates to +/ - keys
- Map is from controller number to parameter ID
- This is unrelated to selector
- Must set sound parameter directly, similar to parameter UI
- Must set both synth sound and tui sound (see tui::send_event())

API:
- Reset map
- Add a mapping (controller number to synth parameter, abs or rel)
- Translate a controller value into a parameter
    In: Controller number, value
    Out: Option(Synth parameter, controller value(abs or rel))
*/

use super::{Parameter, ParamId, ParameterValue};
use super::{SoundData, SoundPatch};
use super::SynthParam;

use serde::{Serialize, Deserialize};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
enum MappingType {
    Absolute,
    Relative
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct CtrlMapEntry {
    id: ParamId,
    map_type: MappingType
}

pub type CtrlHashMap = HashMap<u64, CtrlMapEntry>;

struct CtrlMap {
    map: Vec<CtrlHashMap>
}

impl CtrlMap {
    pub fn new() -> CtrlMap {
        CtrlMap{map: vec!(CtrlHashMap::new(); 128)}
    }

    pub fn reset(&mut self) {
        for m in &mut self.map {
            m.clear();
        }
    }

    pub fn add_mapping(&mut self,
                      program: usize,
                      controller: u64,
                      map_type: MappingType,
                      parameter: &ParamId) {
        self.map[program].insert(controller, CtrlMapEntry{id: *parameter, map_type: map_type});
    }

    /** Update a value according to the controller value.
     *
     * \param program The number of the active sound patch
     * \param controller The controller number to look up
     * \param value New value of the controller
     * \param sound Currently active sound
     * \param result SynthParam that receives the changed value
     *
     * \return true if result was updated, false otherwise
     */
    pub fn get_value(&self,
                    program: usize,
                    controller: u64,
                    value: u64,
                    sound: &SoundData,
                    result: &mut SynthParam) -> bool {
        // Get mapping
        // For absolute: Translate value
        // For relative: Increase/ decrease value
        self.map[program].contains_key(&controller)
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
        let synth_param = SynthParam::new(Parameter::Oscillator, 1, Parameter::Level, ParameterValue::Float(0.0));
        TestContext{map, sound, sound_data, param_id, synth_param}
    }

    pub fn add_controller(&mut self, ctrl_no: u64, ctrl_type: MappingType) {
        self.map.add_mapping(1, ctrl_no, ctrl_type, &self.param_id);
    }

    pub fn handle_controller(&mut self, ctrl_no: u64, value: u64) -> bool {
        self.map.get_value(1, ctrl_no, value, &self.sound_data.borrow(), &mut self.synth_param)
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
