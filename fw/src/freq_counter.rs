use crate::lfsr::{reverse_clk, reverse_sig};
use volatile_register::RO;
use core::future::Future;
use core::task::{Context, Poll, Waker};
use core::pin::Pin;
use typenum::U8;
use alloc::rc::Rc;
use core::cell::UnsafeCell;
use picorv32::interrupt;

type LFSR32 = lfsr::galois::Galois32;

#[repr(C)]
pub struct FrequencyCounter {
    pub ref_sys: RO<u32>,
    pub ref_sig: RO<u32>,
    pub sig_sys: RO<u32>,
    pub epoch: RO<u32>,
}

#[derive(Copy, Clone, Debug)]
pub struct FrequencyCounters {
    ref_sys: u32,
    ref_sig: u32,
    sig_sys: u32,
    pub epoch: u8,
}

impl FrequencyCounters {
    pub fn get_frequency(&self, ref_hz: f64) -> f64 {
        if self.sig_sys == 0 {
            return 0.0;
        }

        ref_hz * (((self.ref_sys as u64) * (self.ref_sig as u64)) as f64) / (self.sig_sys as f64)
    }
}

pub struct FrequencyCountersToleranceCheck {
    pub target_sig_cnt: u32,
    pub sig_cnt_tolerance: u32,
    pub target_clk: u32,
    pub clk_tolerance: u32,
}

impl FrequencyCountersToleranceCheck {
    pub fn check_tolerance(&self, counters: &FrequencyCounters) -> bool {
        counters.ref_sig >= self.target_sig_cnt - self.sig_cnt_tolerance
            && counters.ref_sig <= self.target_sig_cnt + self.sig_cnt_tolerance
            && counters.sig_sys >= self.target_clk - self.clk_tolerance
            && counters.sig_sys <= self.target_clk + self.clk_tolerance
            && counters.ref_sys >= self.target_clk - self.clk_tolerance
            && counters.ref_sys <= self.target_clk + self.clk_tolerance
    }
}

impl FrequencyCounter {
    fn ptr() -> *const Self {
        0x03000004 as *const _
    }

    fn get_counters() -> Result<FrequencyCounters, ()> {
        loop {
            let epoch = unsafe { (*Self::ptr()).epoch.read() } as u8;
            let ref_sys = unsafe { (*Self::ptr()).ref_sys.read() };
            let ref_sig = unsafe { (*Self::ptr()).ref_sig.read() };
            let sig_sys = unsafe { (*Self::ptr()).sig_sys.read() };
            let epoch2 = unsafe { (*Self::ptr()).epoch.read() } as u8;

            if epoch == epoch2 {
                return match (
                    reverse_clk(LFSR32::new(ref_sys)),
                    reverse_sig(LFSR32::new(ref_sig)),
                    reverse_clk(LFSR32::new(sig_sys)),
                ) {
                    (Some(ref_sys), Some(ref_sig), Some(sig_sys)) => Ok(FrequencyCounters {
                        ref_sys,
                        ref_sig,
                        sig_sys,
                        epoch,
                    }),
                    _ => Err(()),
                };
            }
        }
    }
}

pub struct FrequencyCountersFuture {
    state: Rc<UnsafeCell<FrequencyCountersFutureState>>,
}

struct FrequencyCountersFutureState {
    ready: bool,
    waker: Option<Waker>,
}

impl FrequencyCountersFuture {
    pub fn new() -> FrequencyCountersFuture {
        FrequencyCountersFuture {
            state: Rc::new(UnsafeCell::new(FrequencyCountersFutureState {
                ready: false,
                waker: None,
            }))
        }
    }
}

impl Future for FrequencyCountersFuture {
    type Output = Result<FrequencyCounters, ()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {

        if interrupt::free(|_cs| {
            let state = unsafe { self.state.get().as_mut().unwrap() };
            if !state.ready {
                if state.waker.as_ref().filter(|w| w.will_wake(cx.waker())).is_none() {
                    state.waker = Some(cx.waker().clone());
                }
                unsafe {
                    // requires critical section
                    FrequencyCounterInterruptHandler::register(Rc::clone(&self.state));
                }
            }
            state.ready
        }) {
            Poll::Ready(FrequencyCounter::get_counters())
        } else {
            Poll::Pending
        }
    }
}

pub struct FrequencyCounterInterruptHandler {
    queue: heapless::spsc::Queue<Rc<UnsafeCell<FrequencyCountersFutureState>>, U8, u8, heapless::spsc::SingleCore>,
}

impl FrequencyCounterInterruptHandler {
    /// Requires a critical section
    unsafe fn register(future: Rc<UnsafeCell<FrequencyCountersFutureState>>) {
        FREQUENCY_COUNTER_INTERRUPT_HANDLER.queue.enqueue(future).ok().unwrap();
    }

    pub unsafe fn handle_interrupt() {
        let queue = &mut FREQUENCY_COUNTER_INTERRUPT_HANDLER.queue;
        while let Some(state) = queue.dequeue() {
            let state: &mut FrequencyCountersFutureState = state.get().as_mut().unwrap();
            state.ready = true;
            for waker in state.waker.take() {
                waker.wake();
            }
        };
    }
}

static mut FREQUENCY_COUNTER_INTERRUPT_HANDLER: FrequencyCounterInterruptHandler = FrequencyCounterInterruptHandler {
    queue: heapless::spsc::Queue::<Rc<UnsafeCell<FrequencyCountersFutureState>>, U8, u8, heapless::spsc::SingleCore>(
        unsafe { heapless::i::Queue::u8_sc() }
    ),
};
