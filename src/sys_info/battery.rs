use std::io;

use tokio::{io::AsyncReadExt, fs::File};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Status {
    Charging,
    Full,
    Discharging
}

pub async fn capacity(device_name: &str) -> io::Result<u8> {
    let mut file = File::open(
        format!("/sys/class/power_supply/{device_name}/capacity")
    ).await?;

    let mut buf = [0u8; 4];
    let read = file.read(&mut buf).await?;

    let text = unsafe {
        std::str::from_utf8_unchecked(&buf[..read])
    }.trim_end();

    Ok(text.parse().unwrap_or_default())
}

pub async fn status(device_name: &str) -> io::Result<Status> {
    let mut file = File::open(
        format!("/sys/class/power_supply/{device_name}/status")
    ).await?;

    let mut buf = [0u8; 16];
    let read = file.read(&mut buf).await?;

    let text = unsafe {
        std::str::from_utf8_unchecked(&buf[..read])
    }.trim_end();

    Ok(match text {
        "Charging" => Status::Charging,
        "Full" => Status::Full,
        "Discharging" => Status::Discharging,
        _ => {
            eprintln!("Unknown battery status: {text}");

            Status::Discharging
        }
    })
}
