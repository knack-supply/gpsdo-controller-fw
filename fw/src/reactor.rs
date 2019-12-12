
pub struct Reactor {
    dhcp_renew_timer: Option<u64>,
    frequency_counter_ready: bool,
    ntp_request_ready: bool,
    other_net_ready: bool,
}

impl Reactor {
    fn now() -> u64 {
        riscv::register::mcycle::read64()
    }

    fn next_wakeup(&self) -> Option<u64> {
        let mut ret = None;
        if let Some(t) = self.dhcp_renew_timer {
            let v = ret.get_or_insert(t);
            if t < *v {
                *v = t;
            }
        }
        ret
    }

    pub fn run(&mut self) {
        loop {
            let now = Self::now();
            if self.ntp_request_ready {
                self.ntp_request_ready = false;
                self.service_ntp_request();
                continue;
            }

            if self.frequency_counter_ready {
                self.frequency_counter_ready = false;
                self.service_frequency_counter();
                continue;
            }

            if self.other_net_ready {
                self.other_net_ready = false;
                self.service_other_net();
                continue;
            }

            if self.dhcp_renew_timer.filter(|t| (*t - now) as i64 <= 0).is_some() {
                self.dhcp_renew_timer = None;
                self.service_dhcp_renew();
                continue;
            }

            if let Some(next_wakeup) = self.next_wakeup() {
                Self::sleep((next_wakeup - now).min(core::u32::MAX as u64) as u32);
            } else {
                Self::sleep_wfi();
            }
        }
    }

    fn sleep(ticks: u32) {
        if ticks > 0 {
            unsafe {
                picorv32::asm::timer(ticks);
                picorv32::asm::waitirq();
            }
        }
    }

    fn sleep_wfi() {
        unsafe {
            picorv32::asm::waitirq();
        }
    }

    fn service_dhcp_renew(&mut self) {

    }

    fn service_ntp_request(&mut self) {

    }

    fn service_other_net(&mut self) {

    }

    fn service_frequency_counter(&mut self) {

    }

    pub fn schedule_dhcp_renew(&mut self, ticks: u64) {
        self.dhcp_renew_timer = Some(ticks);
    }

    pub fn invoke_dhcp_renew(&mut self) {
        self.dhcp_renew_timer = Some(Self::now());
    }

    pub fn invoke_ntp_request(&mut self) {
        self.ntp_request_ready = true;
    }

    pub fn invoke_frequency_counter(&mut self) {
        self.frequency_counter_ready = true;
    }

    pub fn invoke_other_net(&mut self) {
        self.other_net_ready = true;
    }
}
