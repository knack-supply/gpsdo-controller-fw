use crate::filter::ExponentialAverageFilter;
use libm::F64Ext;

pub struct FeedbackControl {
    frequency: f64,

    dac_code: u16,

    target_frequency: f64,
    control_sensitivity: f64,
    i_factor: f64,

    frequency_filter: ExponentialAverageFilter,
    i_error: f64,
    p_factor: f64,
    i_error_dead_zone: f64,
}

impl FeedbackControl {
    pub fn new(
        dac_code: u16,
        frequency: f64,
        target_frequency: f64,
        control_sensitivity: f64,
        i_factor: f64,
        p_factor: f64,
        i_error_dead_zone: f64,
    ) -> Self {
        Self {
            target_frequency,
            frequency,
            dac_code,
            control_sensitivity,
            frequency_filter: ExponentialAverageFilter::new(3600, frequency),
            i_error: Default::default(),
            i_factor,
            p_factor,
            i_error_dead_zone,
        }
    }

    pub fn set_frequency(&mut self, frequency: f64) {
        self.frequency = frequency;
    }

    fn nullify_dead_zone(adj: f64, dead_zone: f64) -> f64 {
        let mut mag = adj.abs();
        let sign = if adj.is_sign_positive() { 1.0 } else { -1.0 };

        mag -= dead_zone;
        if mag < 0.0 {
            mag = 0.0;
        }

        libm::copysign(mag, sign)
    }

    pub fn tick(&mut self) {
        let set_point_correction = 1.0;

        self.frequency_filter.add(self.frequency);

        let p_error = self.target_frequency - self.get_filtered_frequency();
        let raw_p_error = self.target_frequency - self.frequency;
        self.i_error += raw_p_error;

        let i_error_adj = Self::nullify_dead_zone(self.i_error, self.i_error_dead_zone) *
            self.i_factor / self.control_sensitivity;
        let p_error_adj = (p_error * self.p_factor) / self.control_sensitivity;

        let adj = (i_error_adj + p_error_adj).round();

        if adj != 0.0 {
            self.dac_code = (self.dac_code as i32 + adj as i32).clamp(0, 0xffff) as u16;

            self.frequency_filter.apply_adjustment(
                adj * self.control_sensitivity * set_point_correction
            );
        }
    }

    pub fn get_dac_code(&self) -> u16 {
        self.dac_code
    }

    pub fn get_filtered_frequency(&self) -> f64 {
        self.frequency_filter.get()
    }

    pub fn get_i_error(&self) -> f64 { self.i_error }
}


#[cfg(test)]
mod tests {
    use rand::Rng;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use rand_distr::StandardNormal;

    use assert_approx_eq::assert_approx_eq;
    use statrs::statistics::Statistics;
    use crate::control::FeedbackControl;

    use std::prelude::v1::*;

    const RNG_SEED: [u8; 32] = [1, 0, 0, 0, 23, 0, 0, 0, 200, 1, 0, 0, 210, 30, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    struct OCXO {
        v_control: f64,
        freq: f64,
    }

    impl OCXO {
        fn new() -> Self {
            Self {
                v_control: 2.5,
                freq: 10_000_000.0,
            }
        }

        fn set_v_control(&mut self, v_control: f64) {
            self.v_control = v_control;
        }

        fn tick(&mut self) {
            let target_freq = 10_000_000.0 + (self.v_control - 2.5) * Self::get_control_sensitivity_hz_per_v();
            self.freq = target_freq;
        }

        fn get_frequency(&self) -> f64 {
            self.freq
        }

        pub fn get_control_sensitivity_hz_per_v() -> f64 {
            2.5
        }
    }

    struct DAC16 {
        code: u16,
        v_out: f64,
        v_ref: f64,
    }

    impl DAC16 {
        pub fn new() -> Self {
            Self {
                code: 0,
                v_out: 0.0,
                v_ref: 0.0,
            }
        }

        fn set_code(&mut self, code: u16) {
            self.code = code;
        }

        fn set_v_ref(&mut self, v_ref: f64) {
            self.v_ref = v_ref;
        }

        fn tick(&mut self) {
            let target_v_out = self.v_ref * (self.code as f64 / 65536.0);
            self.v_out = target_v_out;
        }

        fn get_v_out(&self) -> f64 {
            self.v_out
        }
    }

    struct DACFilter {
        v_in: f64,
        v_out: f64,
        c_out: f64,
        r_out: f64,
        r_next_in: f64,
        v_next_in: f64,
    }

    impl DACFilter {
        pub fn new() -> Self {
            Self {
                v_in: 0.0,
                v_out: 0.0,
                c_out: 2.2e-6,
                r_out: 100_000.0,
                r_next_in: 100_000.0,
                v_next_in: 2.5,
            }
        }

        pub fn set_v_in(&mut self, v_in: f64) {
            self.v_in = v_in;
        }

        pub fn init_steady_state(&mut self) {
            self.v_out = self.target_voltage();
        }

        fn target_voltage(&self) -> f64 {
            self.v_next_in * (self.r_out / (self.r_out + self.r_next_in)) +
                self.v_in * (self.r_next_in / (self.r_out + self.r_next_in))
        }

        pub fn tick(&mut self) {
            let tau = self.c_out * 1.0 /
                (1.0 / self.r_out + 1.0 / self.r_next_in);

            self.v_out = self.v_out +
                (self.target_voltage() - self.v_out) * (1.0 - (-1.0 / tau).exp());
        }

        pub fn get_v_out(&self) -> f64 {
            self.v_out
        }
    }

    struct PPS {
        last_error: f64,
        error: f64,
        jitter_rms: f64,
        rng: StdRng,
    }

    impl PPS {
        pub fn new(jitter_rms: f64) -> Self {
            Self {
                last_error: Default::default(),
                error: Default::default(),
                jitter_rms,
                rng: StdRng::from_seed(RNG_SEED),
            }
        }

        pub fn tick(&mut self) {
            self.last_error = self.error;
            self.error = self.rng.sample::<f64, _>(StandardNormal) * self.jitter_rms;
        }

        pub fn get_seconds(&self) -> f64 {
            1.0 + self.error - self.last_error
        }
    }

    struct FrequencyCounter {
        clk_frequency: f64,
        pps_seconds: f64,
        ocxo_frequency: f64,
        reported_frequency: f64,
        clk_slack: f64,
        ocxo_slack: f64,
        ocxo_clk_slack: f64,
    }

    impl FrequencyCounter {
        pub fn new() -> Self {
            Self {
                clk_frequency: 201_000_000.0,
                pps_seconds: Default::default(),
                ocxo_frequency: Default::default(),
                reported_frequency: Default::default(),
                clk_slack: Default::default(),
                ocxo_slack: Default::default(),
                ocxo_clk_slack: Default::default(),
            }
        }

        pub fn set_pps_seconds(&mut self, pps_seconds: f64) {
            self.pps_seconds = pps_seconds;
        }

        pub fn set_ocxo_frequency(&mut self, ocxo_frequency: f64) {
            self.ocxo_frequency = ocxo_frequency;
        }

        pub fn tick(&mut self) {
            let clk_period: f64 = self.clk_frequency.recip();
            let ocxo_period: f64 = self.ocxo_frequency.recip();

            let clk_slack_old = self.clk_slack;
            let clk_cycles = (self.pps_seconds - clk_slack_old) * self.clk_frequency;
            let clk_cycles_seen = clk_cycles.ceil() as u32;
            self.clk_slack = (clk_cycles_seen as f64 - clk_cycles) * clk_period;
            assert!(self.clk_slack >= 0.0);
            assert!(self.clk_slack < clk_period);

            let time_seen = (clk_cycles_seen as f64) * clk_period;
            assert_approx_eq!(time_seen, self.pps_seconds + self.clk_slack - clk_slack_old, 1e-15);

            let ocxo_slack_old = self.ocxo_slack;
            let ocxo_cycles = (self.pps_seconds - ocxo_slack_old) * self.ocxo_frequency;
            let mut ocxo_cycles_seen = ocxo_cycles.ceil() as u32;
            self.ocxo_slack = (ocxo_cycles_seen as f64 - ocxo_cycles) * ocxo_period;
            while self.ocxo_slack < self.clk_slack {
                ocxo_cycles_seen += 1;
                self.ocxo_slack += ocxo_period;
            }

            assert!(self.ocxo_slack >= self.clk_slack);
            assert!(self.ocxo_slack - ocxo_period < self.clk_slack);

            let ocxo_time_seen = (ocxo_cycles_seen as f64) * ocxo_period;
            assert_approx_eq!(ocxo_time_seen, self.pps_seconds + self.ocxo_slack - ocxo_slack_old, 1e-10);

            let ocxo_clk_slack_old = self.ocxo_clk_slack;
            let ocxo_clk_cycles = (self.pps_seconds - ocxo_clk_slack_old) * self.clk_frequency;
            let mut ocxo_clk_cycles_seen = ocxo_clk_cycles.ceil() as u32;
            self.ocxo_clk_slack = (ocxo_clk_cycles_seen as f64 - ocxo_clk_cycles) * clk_period;

            while self.ocxo_clk_slack < self.ocxo_slack {
                ocxo_clk_cycles_seen += 1;
                self.ocxo_clk_slack += clk_period;
            }
            assert!(self.ocxo_clk_slack >= self.ocxo_slack);
            assert!(self.ocxo_clk_slack - clk_period < self.ocxo_slack);

            let ocxo_clk_time_seen = (ocxo_clk_cycles_seen as f64) * clk_period;
            assert_approx_eq!(ocxo_clk_time_seen, self.pps_seconds + self.ocxo_clk_slack - ocxo_clk_slack_old, 1e-10);

            self.reported_frequency = clk_cycles_seen as f64
                * ocxo_cycles_seen as f64
                / ocxo_clk_cycles_seen as f64;
        }

        pub fn get_reported_frequency(&self) -> f64 {
            self.reported_frequency
        }
    }

    struct System {
        ocxo: OCXO,
        dac: DAC16,
        pps: PPS,
        frequency_counter: FrequencyCounter,
        feedback_control: FeedbackControl,
    }

    impl System {
        pub fn new() -> Self {
            let mut dac = DAC16::new();
            dac.set_v_ref(5.0);
            Self {
                ocxo: OCXO::new(),
                dac,
                pps: PPS::new(7.0e-9),
                frequency_counter: FrequencyCounter::new(),
                feedback_control: FeedbackControl::new(
                    32768,
                    10e6,
                    10e6,
                    OCXO::get_control_sensitivity_hz_per_v() / 65536.0 * 5.0,
                    0.0005,
                    0.1,
                    0.25,
                )
            }
        }

        pub fn tick(&mut self) {
            self.dac.set_code(self.feedback_control.get_dac_code());
            self.dac.tick();

            self.ocxo.set_v_control(self.dac.v_out);
            self.ocxo.tick();

            self.pps.tick();

            self.frequency_counter.set_ocxo_frequency(self.ocxo.get_frequency());
            self.frequency_counter.set_pps_seconds(self.pps.get_seconds());
            self.frequency_counter.tick();

            self.feedback_control.set_frequency(self.frequency_counter.get_reported_frequency());
            self.feedback_control.tick();
        }

        pub fn get_reported_frequency(&self) -> f64 {
            self.frequency_counter.get_reported_frequency()
        }

        pub fn metrics(&self) -> SystemMetrics {
            SystemMetrics {
                dac_code: self.dac.code,
                dac_v_out: self.dac.get_v_out(),
                dac_v_ref: self.dac.v_ref,
                ocxo_v_control: self.ocxo.v_control,
                ocxo_frequency: self.ocxo.get_frequency(),
                pps_seconds: self.pps.get_seconds(),
                reported_frequency: self.frequency_counter.get_reported_frequency(),
                filtered_frequency: self.feedback_control.get_filtered_frequency(),
                control_i_error: self.feedback_control.get_i_error(),
            }
        }

        pub fn set_v_ref(&mut self, v_ref: f64) {
            self.dac.set_v_ref(v_ref);
        }
    }

    #[derive(Serialize)]
    struct SystemMetrics {
        dac_code: u16,
        dac_v_out: f64,
        dac_v_ref: f64,
        ocxo_v_control: f64,
        ocxo_frequency: f64,
        pps_seconds: f64,
        reported_frequency: f64,
        filtered_frequency: f64,
        control_i_error: f64,
    }

    #[test]
    fn counter_no_error_for_exact_frequency() {
        let mut counter = FrequencyCounter::new();

        counter.set_ocxo_frequency(10e6);
        counter.set_pps_seconds(1.0);
        counter.tick();

        for _ in 0..10000 {
            counter.tick();

            assert_eq!(counter.get_reported_frequency(), 10e6);
        }
    }

    /// PPS used: ublox NEO-7N (spec'd at 30ns RMS jitter, in fact it's about 7ns RMS)
    /// OCXO used: Connor Winfield OH200-71005SV
    #[test]
    fn error_distribution_for_test_pps_and_ocxo_looks_alright() {
        let mut ocxo = OCXO::new();
        let mut pps = PPS::new(7.0e-9);
        let mut counter = FrequencyCounter::new();

        ocxo.set_v_control(2.5);

        ocxo.tick();
        pps.tick();

        counter.set_ocxo_frequency(ocxo.get_frequency());
        counter.set_pps_seconds(pps.get_seconds());
        counter.tick();

        let mut freq = vec![];
        let mut wtr = csv::WriterBuilder::new()
            .from_path("sim/data/counter_error_distribution.csv").unwrap();
        wtr.write_record(&["reported_frequency"]).unwrap();

        for _ in 0..10000 {
            ocxo.tick();
            pps.tick();

            counter.set_ocxo_frequency(ocxo.get_frequency());
            counter.set_pps_seconds(pps.get_seconds());
            counter.tick();

            freq.push(counter.get_reported_frequency());
            wtr.write_record(&[counter.get_reported_frequency().to_string()]).unwrap();
        }

        assert_approx_eq!(10e6, freq.clone().mean(), 0.001);
        // 0.1Hz @ 10MHz = 10ppb (10ns @ 1s)
        assert!(freq.clone().std_dev() < 0.1);
    }

    #[test]
    fn closed_loop_control_steady_state_stability() {
        let mut system = System::new();

        let mut wtr = csv::WriterBuilder::new()
            .from_path("sim/data/closed_loop_control_steady_state.csv").unwrap();

        for _ in 0..10000 {
            system.tick();
            wtr.serialize(system.metrics()).unwrap();
        }

        for _ in 0..50 {
            let mut freq = vec![];
            for _ in 0..1000 {
                system.tick();
                freq.push(system.get_reported_frequency());
                wtr.serialize(system.metrics()).unwrap();
            }

            assert_approx_eq!(10e6, freq.clone().mean(), 0.001);
            assert!(freq.clone().std_dev() < 0.1);
        }
    }

    #[test]
    fn closed_loop_control_v_ref_step_stability() {
        let mut system = System::new();
        system.set_v_ref(4.9);

        let mut wtr = csv::WriterBuilder::new()
            .from_path("sim/data/closed_loop_control_v_ref_step.csv").unwrap();

        for _ in 0..40000 {
            system.tick();
            wtr.serialize(system.metrics()).unwrap();
        }

//        system.set_v_ref(5.0);
//        for _ in 0..1 {
//            let mut freq = vec![];
//            for _ in 0..1000 {
//                system.tick();
//                freq.push(system.get_reported_frequency());
//                wtr.serialize(system.metrics()).unwrap();
//            }
//
////            assert_approx_eq!(10e6, freq.clone().mean(), 0.001);
////            assert!(freq.clone().std_dev() < 0.1);
//        }
    }
}