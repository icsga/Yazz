use super::parameter::{Selection, ParameterValue, FUNCTIONS};

#[derive(Copy, Clone, PartialEq, Debug)]
enum SelectionState {
    Function,
    FunctionIndex,
    Param,
    Value,
}

pub struct ParamSelection {
    state: SelectionState,
    function_list: &'static [Selection],
    param_list: &'static [Selection],
    pub value: ParameterValue, // ID or value of the selected item
}

impl ParamSelection {
    pub fn new() -> ParamSelection {
        ParamSelection{
            state: SelectionState::Function,
            function_list: &FUNCTIONS,
            param_list: &[], // Will be set when selecting a function
            value: ParameterValue::Int(0),
        }
    }

}

