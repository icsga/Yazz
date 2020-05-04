use super::VAOnePole;

struct Oberheim {
    lpf1: VAOnePole,
    lpf2: VAOnePole,
    lpf3: VAOnePole,
    lpf4: VAOnePole,

    K: Float,
    gamma: Float,
    alpha0: Float,
    Q: Float,
    saturation: Float,
    oberheimCoefs: [Float; 5],
}

impl Oberheim {
    // Oberheim

    pub fn new(sample_rate: Float) -> Self {
        Oberheim{
           lpf1: VAOnePole::new(sample_rate),
           lpf2: VAOnePole::new(sample_rate),
           lpf3: VAOnePole::new(sample_rate),
           lpf4: VAOnePole::new(sample_rate),
           K: 0.0,
           gamma: 0.0,
           alpha0: 0.0,
           Q: 3.0,
           saturation: 1.0,
           oberheimCoefs: [0.0, 0.0, 0.0, 0.0, 0.0]
        }
	// SetCutoff(1000.f);
	// SetResonance(0.1f);
    }

    pub fn process(&mut self, sample: Float, data: &FilterData) -> Float {
        let mut input = sample;

        double sigma =
                self.lpf1->get_feedback_output() +
                self.lpf2->get_feedback_output() +
                self.lpf3->get_feedback_output() +
                self.lpf4->get_feedback_output();

        input *= 1.0 + self.K;

        // calculate input to first filter
        let mut u: Float = (input - self.K * self.sigma) * self.alpha0;

        u = (self.saturation * u).tanh();

        stage1 = lpf1.process(u);
        stage2 = lpf2.process(stage1);
        stage3 = lpf3.process(stage2);
        stage4 = lpf4.process(stage3);

        // Oberheim variations
        // TODO: Optimize this if the coeffs are mostly 0 anyways
        oberheimCoefs[0] * u +
        oberheimCoefs[1] * stage1 +
        oberheimCoefs[2] * stage2 +
        oberheimCoefs[3] * stage3 +
        oberheimCoefs[4] * stage4;
    }

    pub fn update(&mut self, data: &FilterData, freq: Float) {
        self.set_resonance(data.resonance);
        self.set_cutoff(freq);
    }

    fn set_resonance(&mut self, r: Float) {
        // TODO: Adjust for Yazz range
        // this maps resonance = 1->10 to K = 0 -> 4
        self.K = (4.0) * (r - 1.0)/(10.0 - 1.0);
    }

    fn set_cutoff(&mut self, c: Float) {
        let cutoff = c;

        // prewarp for BZT
        let wd = 2.0 * std::F64::PI * cutoff;
        let T = 1.0 / self.sample_rate;
        let wa = (2.0 / T) * tan(wd * T / 2.0);
        let g = wa * T / 2.0;

        // Feedforward coeff
        let G = g / (1.0 + g);

        self.lpf1->set_alpha(G);
        self.lpf2->set_alpha(G);
        self.lpf3->set_alpha(G);
        self.lpf4->set_alpha(G);

        self.lpf1->set_beta(G * G * G / (1.0 + g));
        self.lpf2->set_beta(G * G / (1.0 + g));
        self.lpf3->set_beta(G / (1.0 + g));
        self.lpf4->set_beta(1.0 / (1.0 + g));

        self.gamma = G * G * G * G;
        self.alpha0 = 1.0 / (1.0 + self.K * self.gamma);

        // Oberheim variations / LPF4
        self.oberheimCoefs[0] = 0.0;
        self.oberheimCoefs[1] = 0.0;
        self.oberheimCoefs[2] = 0.0;
        self.oberheimCoefs[3] = 0.0;
        self.oberheimCoefs[4] = 1.0;
    }
}
