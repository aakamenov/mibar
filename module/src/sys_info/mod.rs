pub mod battery;
mod date;
mod cpu;
mod ram;

pub use date::*;

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::{
    task,
    time::{Interval, MissedTickBehavior, Duration, interval}
};

const CPU_POLL_INTERVAL: u64 = 800;
const RAM_POLL_INTERVAL: u64 = 800;

static mut CPU_USAGE: AtomicU64 = AtomicU64::new(0);
static mut RAM: RamUsage = RamUsage { total: 0, available: 0 };

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct RamUsage {
    pub total: u64,
    pub available: u64
}

pub fn init() {
    task::spawn(async {
        cpu::poll_usage(create_interval(CPU_POLL_INTERVAL)).await;
    });

    task::spawn(async {
        ram::poll(create_interval(RAM_POLL_INTERVAL)).await;
    });
}

#[inline]
pub fn cpu_usage() -> f64 {
    let usage = unsafe {
        CPU_USAGE.load(Ordering::Acquire)
    };

    f64::from_bits(usage)
}

#[inline]
pub fn ram_usage() -> RamUsage {
    unsafe { RAM }
}

#[inline]
fn create_interval(ms: u64) -> Interval {
    let mut interval = interval(Duration::from_millis(ms));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    interval
}

impl RamUsage {
    #[inline]
    pub fn used(&self) -> u64 {
        self.total - self.available
    }

    #[inline]
    pub fn used_percentage(&self) -> f64 {
        (self.used() as f64 / self.total as f64) * 100f64
    }
}
