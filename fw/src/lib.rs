#![no_std]
#![feature(asm)]
#![feature(proc_macro_hygiene)]
#![feature(clamp)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(const_fn)]
#![feature(async_await)]

extern crate ufmt;
#[macro_use]
extern crate pin_utils;
#[macro_use]
extern crate bitfield;
extern crate nb;

#[macro_use]
pub mod util;

pub mod ads1018;
pub mod allocator;
pub mod bus;
pub mod control;
pub mod filter;
pub mod freq_counter;
pub mod futures;
pub mod hal;
pub mod lfsr;
pub mod max5216;
pub mod picosoc;
pub mod reactor;

#[cfg(test)]
#[macro_use]
extern crate std;
#[cfg(test)]
#[macro_use]
extern crate assert_approx_eq;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

extern crate alloc;
