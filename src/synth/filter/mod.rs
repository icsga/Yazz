pub mod filter;
pub mod onepole;

mod korg35;
mod ober_moog;
mod sem;
mod va_onepole;
//mod moog_improved;
//mod reson_z;
//mod rlpf;

pub use filter::{Filter, FilterData, FilterType};
pub use onepole::OnePole;

use va_onepole::VAOnePole;
