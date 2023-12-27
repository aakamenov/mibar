pub mod battery;
mod date;
mod cpu;
mod ram;

pub use date::*;
pub use ram::RamUsage;

use cpu::CpuListener;
use ram::RamListener;

use tokio::time::{Interval, MissedTickBehavior, Duration, interval};

use crate::system_monitor::SystemMonitor;

pub static CPU: SystemMonitor<CpuListener> = SystemMonitor::new();
pub static RAM: SystemMonitor<RamListener> = SystemMonitor::new();

#[inline]
fn create_interval(ms: u64) -> Interval {
    let mut interval = interval(Duration::from_millis(ms));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    interval
}
