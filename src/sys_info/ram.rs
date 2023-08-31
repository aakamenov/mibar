use std::io::SeekFrom;
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    fs::File,
    time::Interval
};

use super::{RAM, RamUsage};

pub async fn poll(mut interval: Interval) {
    let mut file = match File::open("/proc/meminfo").await {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Error retrieving RAM info: {err}");
            return;
        }
    };

    let mut buf = [0u8; 2048];

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

                unsafe {
                    RAM = RamUsage { total, available };
                }
            }
            Err(err) => {
                eprintln!("Error retrieving RAM info: {err}");
                continue;
            }
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
