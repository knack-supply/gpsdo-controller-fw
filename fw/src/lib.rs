#![no_std]
#![feature(asm)]
#![feature(proc_macro_hygiene)]
#![feature(clamp)]

//#[macro_use]
extern crate ufmt;

pub mod bus;
pub mod control;
pub mod filter;
pub mod freq_counter;
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
