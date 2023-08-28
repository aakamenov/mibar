mod date;
mod cpu;

pub use date::*;

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::{
    task,
    time::{Interval, MissedTickBehavior, Duration, interval}
};

const CPU_POLL_INTERVAL: u64 = 800;

static mut CPU_USAGE: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    task::spawn(async {
        cpu::poll_usage(create_interval(CPU_POLL_INTERVAL)).await;
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
fn create_interval(ms: u64) -> Interval {
    let mut interval = interval(Duration::from_millis(ms));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    interval
}
