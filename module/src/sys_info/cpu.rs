use std::{io::SeekFrom, pin::Pin, future::Future};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    fs::File,
    sync::watch::Sender
};

use crate::system_monitor::Listener;

use super::create_interval;

const POLL_INTERVAL: u64 = 800;

pub struct CpuListener;

#[derive(Clone, Copy, Default, Debug)]
struct CpuTime {
    work_time: u64,
    total_time: u64
}

impl Listener for CpuListener {
    type Value = f64;

    fn initial_value() -> Self::Value {
        0f64
    }

    fn run(tx: Sender<Self::Value>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(poll_usage(tx))
    }
}

async fn poll_usage(tx: Sender<f64>) {
    let mut file = match File::open("/proc/stat").await {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Error retrieving CPU stats: {err}");
            
            return;
        }
    };

    let mut interval = create_interval(POLL_INTERVAL);

    let mut buf = [0u8; 256];
    let mut new = CpuTime::default();

    loop {
        interval.tick().await;

        if file.seek(SeekFrom::Start(0)).await.is_err() {
            continue;
        }

        match file.read(&mut buf).await {
            Ok(read) => {
                const CPU_LINE: &[u8] = b"cpu  ";

                let buf = &buf[..read];
                if !buf.starts_with(CPU_LINE) {
                    continue;
                }

                let buf = unsafe {
                    std::str::from_utf8_unchecked(&buf[CPU_LINE.len()..])
                };
                
                let Some(index) = buf.find('\n') else {
                    continue;
                };

                let mut entries = buf[0..=index].split(" ").filter(|x| !x.is_empty());

                // Values from: https://man7.org/linux/man-pages/man5/proc.5.html
                let user = parse_num(entries.next());
                let nice = parse_num(entries.next());
                let system = parse_num(entries.next());
                let idle = parse_num(entries.next());
                let iowait = parse_num(entries.next());
                let irq = parse_num(entries.next());
                let softirq = parse_num(entries.next());
                let steal = parse_num(entries.next());
                let guest = parse_num(entries.next());
                let guest_nice = parse_num(entries.next());

                let user = user.saturating_sub(guest);
                let nice = nice.saturating_sub(guest_nice);

                let work_time = user
                    .saturating_add(nice)
                    .saturating_add(system)
                    .saturating_add(irq)
                    .saturating_add(softirq);

                let total_time = work_time
                    .saturating_add(idle)
                    .saturating_add(iowait)
                    .saturating_add(guest)
                    .saturating_add(guest_nice)
                    .saturating_add(steal);

                let old = new;
                new = CpuTime {
                    work_time,
                    total_time
                };

                let work_time = new.work_time.saturating_sub(old.work_time) as f64;
                let total_time = new.total_time.saturating_sub(old.total_time).max(1) as f64;
                let usage = ((work_time / total_time) * 100f64).min(100f64);

                if tx.send(usage).is_err() {
                    return;
                }
            }
            Err(err) => eprintln!("Error retrieving CPU stats: {err}")
        }
    }
}

#[inline]
fn parse_num(slice: Option<&str>) -> u64 {
    slice.map(|x| x.parse::<u64>().ok()).flatten().unwrap_or(0)
}
