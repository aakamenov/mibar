use std::{env::{self, VarError}, str};

use tokio::{
    net::UnixStream,
    io::{AsyncReadExt, AsyncWriteExt}
};

use crate::ui::ValueSender;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Workspace {
    id: u8,
    num_windows: u8
}

enum SocketType {
    Read,
    Write
}

pub async fn start_listener_loop(sender: ValueSender<Vec<Workspace>>) {
    // Opening the write stream here together with the read stream causes the
    // compositor to freeze, so we open it every time we want to dispatch events instead.
    let Some(mut read_stream) = open_stream(SocketType::Read).await else {
        return;
    };

    let mut retries = 0;
    let mut buf = [0; 2048];
    let mut bytes = vec![];

    // Send an initial value.
    if let Some(workspaces) = get_workspaces(&mut buf, &mut bytes).await {
        sender.send(workspaces).await;
    }

    loop {
        match read_all(&mut read_stream, &mut buf, &mut bytes).await {
            Ok(()) => retries = 0,
            Err(err) => {
                retries += 1;
                eprintln!("Error while reading from Hyprland socket: {err}");
                
                if retries == 3 {
                    eprintln!("Closing listener...");
                    break;
                }

                continue;
            }
        };

        let text = unsafe {
            str::from_utf8_unchecked(&bytes)
        };

        const WORKSPACE: &str = "workspace>>";

        if !text.contains(WORKSPACE) {
            continue;
        }

        if let Some(workspaces) = get_workspaces(&mut buf, &mut bytes).await {
            sender.send(workspaces).await;
        }
    }
}

async fn get_workspaces(buf: &mut [u8], bytes: &mut Vec<u8>) -> Option<Vec<Workspace>> {
    const CMD: &str = "/workspaces";

    let Some(mut write_stream) = open_stream(SocketType::Write).await else {
        return None;
    };

    if let Err(err) = write_stream.write_all(CMD.as_bytes()).await {
        eprintln!("Failed to write to Hyprland command socket: {err}");

        return None;
    }

    match read_all(&mut write_stream, buf, bytes).await {
        Ok(_) => {
            let text = unsafe {
                str::from_utf8_unchecked(&bytes)
            };

            Some(parse_workspaces(text))
        },
        Err(err) => {
            eprintln!("Error while reading Hyprland workspaces: {err}");

            None
        }
    }
}

fn parse_workspaces(text: &str) -> Vec<Workspace> {
    const WORKSPACE_DEF: &str = "workspace ID ";
    const WINDOWS_FIELD: &str = "windows: ";

    let mut result = Vec::with_capacity(6);
    let mut cursor = 0;

    while let Some(id) = parse_u8(text, &mut cursor, ' ' as u8, WORKSPACE_DEF) {
        match parse_u8(text, &mut cursor, '\n' as u8, WINDOWS_FIELD) {
            Some(num_windows) =>
                result.push(Workspace { id, num_windows }),
            None =>
                eprintln!("Error parsing workspace number of windows field. Hyprland output:\n{text}")
        }
    }

    result
}

fn parse_u8(
    text: &str,
    cursor: &mut usize,
    stop_char: u8,
    pat: &'static str
) -> Option<u8> {
    let Some(index) = text[*cursor..].find(pat) else {
        return None;
    };

    *cursor += index + pat.len();
    let (_, num_start) = text.split_at(*cursor);

    let bytes = num_start.as_bytes();
    let read = advance_until(bytes, stop_char);
    *cursor += read;

    let num = unsafe {
        str::from_utf8_unchecked(&bytes[..read])
    };

    match num.parse::<u8>() {
        Ok(num) => Some(num),
        Err(_) => {
            eprintln!("Error parsing u8. Hyprland output:\n{text}");

            None
        }
    }
}

#[inline]
fn advance_until(bytes: &[u8], stop_char: u8) -> usize {
    let mut index = 0;
        
    while bytes[index] != stop_char {
        index += 1;
    }

    index
}

async fn read_all(
    stream: &mut UnixStream,
    buf: &mut [u8],
    bytes: &mut Vec<u8>
) -> Result<(), std::io::Error> {
    bytes.clear();

    'outer: loop {
        match stream.read(buf).await {
            Ok(read) => {
                bytes.extend_from_slice(&buf[..read]);

                if read < buf.len() {
                    break 'outer Ok(());
                }
            },
            Err(err) => return Err(err)
        }
    }
}

async fn open_stream(ty: SocketType) -> Option<UnixStream> {
    let Some(socket) = hyprland_socket(ty) else {
        return None;
    };

    let stream = match UnixStream::connect(socket).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Error opening Hyprland socket: {err}");
            return None;
        }
    };

    Some(stream)
}

fn hyprland_socket(ty: SocketType) -> Option<String> {
    const ENV_VAR: &str = "HYPRLAND_INSTANCE_SIGNATURE";

    match env::var(ENV_VAR) {
        Ok(var) => {
            let ext = match ty {
                SocketType::Write => ".socket.sock",
                SocketType::Read => ".socket2.sock"
            };
            let path = format!("/tmp/hypr/{var}/{ext}");

            Some(path)
        },
        Err(VarError::NotPresent) => {
            eprintln!("Hyprland envrionment variable ({ENV_VAR}) not present.");

            None
        }
        Err(VarError::NotUnicode(_)) => {
            eprintln!("Hyprland envrionment variable ({ENV_VAR}) is present but is not valid unicode.");

            None
        }
    }
}
