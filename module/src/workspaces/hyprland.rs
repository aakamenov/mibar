use std::{env::{self, VarError}, str};

use tokio::{
    net::UnixStream,
    io::{AsyncReadExt, AsyncWriteExt}
};

use mibar_core::ValueSender;

pub struct WorkspacesChanged {
    pub current: u8,
    pub workspaces: Vec<Workspace>
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Workspace {
    pub id: u8,
    pub num_windows: u8
}

enum SocketType {
    Read,
    Write
}

pub async fn start_listener_loop(sender: ValueSender<WorkspacesChanged>) {
    // Opening the write stream here together with the read stream causes the
    // compositor to freeze, so we open it every time we want to dispatch events instead.
    let Some(mut read_stream) = open_stream(SocketType::Read).await else {
        return;
    };

    let mut retries = 0;
    let mut buf = [0; 2048];
    let mut event_bytes = Vec::with_capacity(256);
    let mut workspaces_bytes = Vec::with_capacity(1024);

    // Last known selected workspace ID. We store this because
    // only the workspace>> event notifies us of a change.
    let mut current = 1;

    // Send an initial value.
    if let Some(workspaces) = get_workspaces(&mut buf, &mut workspaces_bytes).await {
        let event = WorkspacesChanged {
            current,
            workspaces
        };

        sender.send(event).await;
    }

    loop {
        match read_all(&mut read_stream, &mut buf, &mut event_bytes).await {
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
            str::from_utf8_unchecked(&event_bytes)
        };

        const WORKSPACE: &str = "workspace>>";
        const OPEN_WINDOW: &str = "openwindow>>";
        const CLOSE_WINDOW: &str = "closewindow>>";

        let mut updated_workspaces = false;
        for line in text.lines() {
            // Check if the event explicitly starts with workspace>>
            // otherwise we erroneously parse events like destroyworkspace>>
            if let Some(new_current) = line.starts_with(WORKSPACE)
                .then(|| parse_u8(line, &mut 0, '\n' as u8, WORKSPACE))
                .flatten()
            {
                current = new_current;

                if !updated_workspaces {
                    if let Some(workspaces) = get_workspaces(&mut buf, &mut workspaces_bytes).await {
                        let event = WorkspacesChanged {
                            current,
                            workspaces
                        };

                        sender.send(event).await;
                    }
                }

                break;

            } else if !updated_workspaces &&
                (line.starts_with(OPEN_WINDOW) ||
                line.starts_with(CLOSE_WINDOW))
            {
                if let Some(workspaces) = get_workspaces(&mut buf, &mut workspaces_bytes).await {
                    let event = WorkspacesChanged {
                        current,
                        workspaces
                    };

                    sender.send(event).await;
                    updated_workspaces = true;
                }
            }   
        }
    }
}

#[inline]
pub async fn change_workspace(id: u8) {
    let cmd = format!("dispatch workspace {id}");
    dispatch_command(cmd.as_bytes()).await;
}

#[inline]
pub async fn move_workspace_next() {
    let cmd = format!("dispatch workspace e+1");
    dispatch_command(cmd.as_bytes()).await;
}

#[inline]
pub async fn move_workspace_prev() {
    let cmd = format!("dispatch workspace e-1");
    dispatch_command(cmd.as_bytes()).await;
}

async fn dispatch_command(cmd: &[u8]) {
    let mut buf = [0u8; 64];

    let Some(mut write_stream) = dispatch(cmd).await else {
        return;
    };

    match write_stream.read(&mut buf).await {
        Ok(read) => {
            let resp = unsafe {
                str::from_utf8_unchecked(&buf[0..read])
            };

            if resp != "ok" {
                eprintln!("Received error from Hyprland dispatch: {resp}");
            }
        },
        Err(err) => {
            eprintln!("Error reading Hyprland response: {err}")
        }
    }
}

async fn get_workspaces(buf: &mut [u8], bytes: &mut Vec<u8>) -> Option<Vec<Workspace>> {
    const CMD: &str = "/workspaces";

    let mut write_stream = dispatch(CMD.as_bytes()).await?;
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

async fn dispatch(cmd: &[u8]) -> Option<UnixStream> {
    let mut write_stream = open_stream(SocketType::Write).await?;
    if let Err(err) = write_stream.write_all(cmd).await {
        eprintln!("Failed to write to Hyprland command socket: {err}");

        return None;
    }

    Some(write_stream)
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
    let index = text[*cursor..].find(pat)?;

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
        
    while index < bytes.len() && bytes[index] != stop_char {
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
    let socket = hyprland_socket(ty)?;
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
