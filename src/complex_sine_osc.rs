/*
// number of elapsed samples before stabilization occurs
const stabilizationPeriod = 500

// TrigGenerator implements a trigonometric generator using a recursively updated complex
// phasor
type TrigGenerator struct {
	// The last known generator timecode
	t Timecode
	// Time of last stabilization
	s Timecode
	// A phasor holding the current state of the trig generator
	z complex128
	// The sample rate for generation
	sampleRate Frequency
}

// NewTrigGenerator creates a new TrigGenerator
func NewTrigGenerator(f Frequency) *TrigGenerator {
	gen := new(TrigGenerator)
	gen.t = 0
	gen.s = 0
	gen.sampleRate = f
	gen.z = complex(1, 0)
	return gen
}

// Sine emits a sine wave for a given frequency and timecode
// http://dsp.stackexchange.com/a/1087
func (gen *TrigGenerator) Sine(tʹ Timecode, f Frequency) Amplitude {
	// determine how many samples we have to advance
	// typically this is one - more than one implies time dialation
	Δt := tʹ - gen.t

	// compute the angular frequency of the oscilator in radians
	ω := 2 * math.Pi * f / sampleRate

	// compute the complex angular coeficient
	Ω := cmplx.Exp(complex(0, ω))

	// advance the phasor Δt units
	for i := Δt; i > 0; i-- {
		gen.z = gen.z * Ω
	}

	// stabilize the phasor's amplitude every once in a while
	// the amplitude can drift due to rounding errors
	// since z is a unity phasor, adjust its amplitude back towards unity
	if gen.s > stabilizationPeriod {
		a := real(gen.z)
		b := imag(gen.z)
		c := (3 - math.Pow(a, 2) - math.Pow(b, 2)) / 2
		gen.z = gen.z * complex(c, 0)
		gen.s = 0
	}

	// advance time
	gen.t += Δt
	gen.s += Δt
	// return the 'sine' component of the phasor
	return Amplitude(imag(gen.z))
}
*/

extern crate num;

use super::oscillator::Oscillator;
use std::cell::RefCell;

pub struct ComplexSineOscillator {
    freq: f32,
    sample_rate: u32,
    state: RefCell<CompSineOscState>
}

struct CompSineOscState {
    last_update: u64, // Time of last sample generation
    last_value: f32,
    last_stabilization: u64, // Time of last stabilization
    phasor: num::complex::Complex<f32>, // Phasor with current state
}

impl ComplexSineOscillator {
    pub fn new(sample_rate: u32) -> ComplexSineOscillator {
        let freq = 440.0;
        let last_update = 0;
        let last_value = 0.0;
        let last_stabilization = 0;
        let phasor = num::complex::Complex::new(1.0, 0.0);
        let state = RefCell::new(CompSineOscState{last_update, last_value, last_stabilization, phasor});
        let osc = ComplexSineOscillator{freq, sample_rate, state};
        osc
    }
}

impl Oscillator for ComplexSineOscillator {
    fn set_freq(&mut self, freq: f32) {
        self.freq = freq;
    }

    fn get_freq(&self) -> f32 {
        self.freq
    }

    fn get_sample(&self, sample_clock: u64, freq:f32) -> f32 {
        let mut state = self.state.borrow_mut();
        if sample_clock != state.last_update {
            let dt = sample_clock - state.last_update;

            // compute the angular frequency of the oscilator in radians
            let ang_freq = 2.0 * 3.141592 * freq / self.sample_rate as f32;

            // compute the complex angular coeficient
            let omega = num::complex::Complex::new(0.0, ang_freq);
            let coefficient = omega.exp();

            // advance the phasor Δt units
            for _ in 0..dt {
                    state.phasor *= coefficient;
            }

            // stabilize the phasor's amplitude every once in a while
            // the amplitude can drift due to rounding errors
            // since z is a unity phasor, adjust its amplitude back towards unity
            if state.last_stabilization > 500 {
                    let a = state.phasor.re;
                    let b = state.phasor.im;
                    let c = (3.0 - a.powi(2) - b.powi(2)) / 2.0;
                    let stab = num::complex::Complex::new(c, 0.0);
                    state.phasor = state.phasor * stab;
                    state.last_stabilization = 0;
            }

            // advance time
            state.last_update += dt;
            state.last_stabilization += dt;
            // return the 'sine' component of the phasor
            state.last_value = state.phasor.im;
        }
        state.last_value
    }

    fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

