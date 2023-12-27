use std::{io::SeekFrom, pin::Pin, future::Future};

use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    fs::File,
    sync::watch::Sender
};

use crate::system_monitor::Listener;
use super::create_interval;

const POLL_INTERVAL: u64 = 800;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct RamUsage {
    pub total: u64,
    pub available: u64
}

pub struct RamListener;

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

impl Listener for RamListener {
    type Value = RamUsage;

    fn initial_value() -> Self::Value {
        RamUsage::default()
    }

    fn run(tx: Sender<Self::Value>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(poll(tx))
    }
}

async fn poll(tx: Sender<RamUsage>) {
    let mut file = match File::open("/proc/meminfo").await {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Error retrieving RAM info: {err}");

            return;
        }
    };

    let mut interval = create_interval(POLL_INTERVAL);

    let mut buf = [0u8; 2048];
    let mut old = RamUsage::default();

    loop {
        interval.tick().await;

        if file.seek(SeekFrom::Start(0)).await.is_err() {
            continue;
        }

        match file.read(&mut buf).await {
            Ok(read) => {
                let text = unsafe {
                    std::str::from_utf8_unchecked(&buf[..read])
                };

                let total = find_value(text, "MemTotal").unwrap_or(0);
                let available = find_value(text, "MemAvailable").unwrap_or(0);

                let new = RamUsage { total, available };

                if new != old {
                    old = new;

                    if tx.send(new).is_err() {
                        return;
                    }   
                }
            }
            Err(err) => eprintln!("Error retrieving RAM info: {err}")
        }
    }
}

fn find_value(text: &str, field: &'static str) -> Option<u64> {
    // +1 for the : character
    let mut num_start = text.find(field)? + field.len() + 1;
    let bytes = text.as_bytes();

    while num_start < text.len() {
        if bytes[num_start] != b' ' {
            break;
        }

        num_start += 1;
    }

    let mut num_end = num_start;

    while num_end < text.len() {
        let char = bytes[num_end];
        if char < 48 || char > 57 {
            break;
        }

        num_end += 1;
    }

    num_end -= 1;

    text[num_start..=num_end].parse().ok()
}
