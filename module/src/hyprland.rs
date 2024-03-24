use std::{
    sync::{RwLock, Mutex, atomic::{AtomicUsize, Ordering}},
    collections::HashMap,
    env::{self, VarError},
    str
};

use once_cell::sync::Lazy;
use mibar_core::{ValueSender, window::WindowId};
use tokio::{
    runtime,
    net::UnixStream,
    io::{AsyncReadExt, AsyncWriteExt},
    task::JoinHandle
};

use crate::{sender_key::SenderKey, StaticPtr};

static SUB_COUNT: AtomicUsize = AtomicUsize::new(0);
static HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None); 

static WORKSPACES_SUBS: Lazy<RwLock<HashMap<SenderKey, ValueSender<WorkspacesChanged>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static KEYBOARD_SUBS: Lazy<RwLock<HashMap<SenderKey, KeyboardSubscriber>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static LAYOUTS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        ("Albanian", "al"),
        ("Amharic", "et"),
        ("Armenian", "am"),
        ("Arabic", "ara"),
        ("Arabic (Egypt)", "eg"),
        ("Arabic (Iraq)", "iq"),
        ("Arabic (Morocco)", "ma"),
        ("Arabic (Syria)", "sy"),
        ("Azerbaijani", "az"),
        ("Bambara", "ml"),
        ("Bangla", "bd"),
        ("Belarusian", "by"),
        ("Belgian", "be"),
        ("Berber (Algeria, Latin)", "dz"),
        ("Bosnian", "ba"),
        ("Braille", "brai"),
        ("Bulgarian", "bg"),
        ("Burmese", "mm"),
        ("Chinese", "cn"),
        ("Croatian", "hr"),
        ("Czech", "cz"),
        ("Danish", "dk"),
        ("Dari", "af"),
        ("Dhivehi", "mv"),
        ("Dutch", "nl"),
        ("Dzongkha", "bt"),
        ("English (Australia)", "au"),
        ("English (Cameroon)", "cm"),
        ("English (Ghana)", "gh"),
        ("English (New Zealand)", "nz"),
        ("English (Nigeria)", "ng"),
        ("English (South Africa)", "za"),
        ("English (UK)", "gb"),
        ("English (US)", "us"),
        ("Esperanto", "epo"),
        ("Estonian", "ee"),
        ("Faroese", "fo"),
        ("Filipino", "ph"),
        ("Finnish", "fi"),
        ("French", "fr"),
        ("French (Canada)", "ca"),
        ("French (Democratic Republic of the Congo)", "cd"),
        ("French (Togo)", "tg"),
        ("Georgian", "ge"),
        ("German", "de"),
        ("German (Austria)", "at"),
        ("German (Switzerland)", "ch"),
        ("Greek", "gr"),
        ("Hebrew", "il"),
        ("Hungarian", "hu"),
        ("Icelandic", "is"),
        ("Indian", "in"),
        ("Indonesian (Latin)", "id"),
        ("Irish", "ie"),
        ("Italian", "it"),
        ("Japanese", "jp"),
        ("Kazakh", "kz"),
        ("Khmer (Cambodia)", "kh"),
        ("Korean", "kr"),
        ("Kyrgyz", "kg"),
        ("Lao", "la"),
        ("Latvian", "lv"),
        ("Lithuanian", "lt"),
        ("Macedonian", "mk"),
        ("Malay (Jawi, Arabic Keyboard)", "my"),
        ("Maltese", "mt"),
        ("Moldavian", "md"),
        ("Mongolian", "mn"),
        ("Montenegrin", "me"),
        ("Nepali", "np"),
        ("N'Ko (AZERTY)", "gn"),
        ("Norwegian", "no"),
        ("Persian", "ir"),
        ("Polish", "pl"),
        ("Portuguese", "pt"),
        ("Portuguese (Brazil)", "br"),
        ("Romanian", "ro"),
        ("Russian", "ru"),
        ("Serbian", "rs"),
        ("Sinhala (phonetic)", "lk"),
        ("Slovak", "sk"),
        ("Slovenian", "si"),
        ("Spanish", "es"),
        ("Spanish (Latin American)", "latam"),
        ("Swahili (Kenya)", "ke"),
        ("Swahili (Tanzania)", "tz"),
        ("Swedish", "se"),
        ("Taiwanese", "tw"),
        ("Tajik", "tj"),
        ("Thai", "th"),
        ("Tswana", "bw"),
        ("Turkmen", "tm"),
        ("Turkish", "tr"),
        ("Ukrainian", "ua"),
        ("Urdu (Pakistan)", "pk"),
        ("Uzbek", "uz"),
        ("Vietnamese", "vn"),
        ("Wolof", "sn"),
        ("A user-defined custom Layout", "custom")
    ])
});

/// As long as this token is alive the widget will receive new values
/// **unless** the event listener has exited unexpectedly.
/// Dropping the token will automatically unsubscribe the widget. 
pub struct SubscriptionToken<T> {
    handle: StaticPtr<RwLock<HashMap<SenderKey, T>>>,
    key: SenderKey
}
    
#[derive(Clone, Copy, Debug)]
pub struct KeyboardLayoutChanged {
    pub device: &'static str,
    pub layout: Option<&'static str> 
}

#[derive(Clone, Debug)]
pub struct WorkspacesChanged {
    pub current: u8,
    pub workspaces: Vec<Workspace>
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Workspace {
    pub id: u8,
    pub num_windows: u8
}

pub struct KeyboardSubscriber {
    sender: ValueSender<KeyboardLayoutChanged>,
    device: &'static str
}

enum SocketType {
    Read,
    Write
}

pub fn subscribe_workspaces(
    handle: &runtime::Handle,
    window_id: WindowId,
    sender: ValueSender<WorkspacesChanged>
) -> SubscriptionToken<ValueSender<WorkspacesChanged>> {
    let key = SenderKey::new(window_id, &sender);

    let mut map = WORKSPACES_SUBS.write().unwrap();
    map.insert(key, sender);

    drop(map);

    start_or_increment(handle);

    SubscriptionToken {
        handle: StaticPtr::new(Lazy::force(&WORKSPACES_SUBS)),
        key
    }
}

pub fn subscribe_keyboard_layout(
    handle: &runtime::Handle,
    window_id: WindowId,
    sender: ValueSender<KeyboardLayoutChanged>,
    device: &'static str
) -> SubscriptionToken<KeyboardSubscriber> {
    let key = SenderKey::new(window_id, &sender);

    let mut map = KEYBOARD_SUBS.write().unwrap();
    map.insert(
        key,
        KeyboardSubscriber { sender, device }
    );

    drop(map);

    start_or_increment(handle);

    SubscriptionToken {
        handle: StaticPtr::new(Lazy::force(&KEYBOARD_SUBS)),
        key
    }
}

#[inline]
pub async fn change_workspace(id: u8) {
    let cmd = format!("dispatch workspace {id}");
    dispatch_command(cmd.as_bytes()).await;
}

#[inline]
pub async fn move_workspace_next() {
    const CMD: &str = "dispatch workspace e+1";
    dispatch_command(CMD.as_bytes()).await;
}

#[inline]
pub async fn move_workspace_prev() {
    const CMD: &str = "dispatch workspace e-1";
    dispatch_command(CMD.as_bytes()).await;
}

pub async fn active_workspace() -> Option<Workspace> {
    const CMD: &str = "activeworkspace";

    let Some(mut stream) = dispatch(CMD.as_bytes()).await else {
        return None;
    };

    let mut buf = [0u8; 256];
    match stream.read(&mut buf).await {
        Ok(read) => {
            let resp = unsafe {
                str::from_utf8_unchecked(&buf[0..read])
            };

            parse_workspaces(&resp).pop()
        },
        Err(err) => {
            eprintln!("Error reading Hyprland response: {err}");

            None
        }
    }
}

#[inline]
pub async fn keyboard_layout_next(device: &'static str) {
    let cmd = format!("switchxkblayout {} next", device);

    dispatch_command(cmd.as_bytes()).await;
}

#[inline]
pub async fn keyboard_layout_prev(device: &'static str) {
    let cmd = format!("switchxkblayout {} prev", device);

    dispatch_command(cmd.as_bytes()).await;
}

pub async fn current_layout(device: &'static str) -> Option<&'static str> {
    const CMD: &str = "devices";
    const ACTIVE_KEYMAP: &str = "active keymap: ";

    let Some(mut stream) = dispatch(CMD.as_bytes()).await else {
        return None;
    };

    let mut buf = Vec::with_capacity(512);

    match stream.read_to_end(&mut buf).await {
        Ok(_) => {
            let resp = unsafe {
                str::from_utf8_unchecked(&buf)
            };

            let Some(index) = resp.find(device) else {
                return None;
            };

            let index = index + device.len();

            let line = resp[index..].lines().nth(2)?.trim_start();
            let layout = &line[ACTIVE_KEYMAP.len()..];

            get_keyboard_layout(layout)
        }
        Err(err) => {
            eprintln!("Error reading Hyprland response: {err}");

            None
        }
    }
}

fn start_or_increment(handle: &runtime::Handle) {
    if SUB_COUNT.fetch_add(1, Ordering::AcqRel) == 0 {
        let handle = handle.spawn(start());
        let mut lock = HANDLE.lock().unwrap();

        // This shouldn't happen.
        if let Some(prev) = lock.as_ref() {
            prev.abort();
            drop_all_subs();
        }

        *lock = Some(handle);
    }
}

async fn start() {
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
            current: active_workspace().await.map(|x| x.id).unwrap_or(current),
            workspaces
        };

        send_workspaces(event);
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
        const KEYBOARD_LAYOUT: &str = "activelayout>>";

        let mut updated_workspaces = false;
        for line in text.lines() {
            let event_name = match line.find(">>") {
                Some(pos) => &line[..(pos + 2)],
                None => continue
            };
            
            match event_name {
                WORKSPACE => {
                    let Some(new_current) = parse_u8(line, &mut 0, '\n' as u8, WORKSPACE) else {
                        continue;
                    };

                    current = new_current;

                    if !updated_workspaces {
                        if let Some(workspaces) = get_workspaces(&mut buf, &mut workspaces_bytes).await {
                            send_workspaces(WorkspacesChanged { current, workspaces });
                            updated_workspaces = true;
                        }
                    }
                }
                OPEN_WINDOW | CLOSE_WINDOW if !updated_workspaces => {
                    let Some(workspaces) = get_workspaces(&mut buf, &mut workspaces_bytes).await else {
                        continue;
                    };

                    send_workspaces(WorkspacesChanged { current, workspaces });
                    updated_workspaces = true;
                }
                KEYBOARD_LAYOUT => {
                    let mut iter = line[KEYBOARD_LAYOUT.len()..].split(",");

                    let (Some(device), Some(layout)) = (iter.next(), iter.next()) else {
                        eprintln!("Couldn't parse the {KEYBOARD_LAYOUT} event.");

                        continue;
                    };

                    send_keyboard_layout(device, layout);
                }
                _ => { }
            }
        }
    }

    SUB_COUNT.store(0, Ordering::Release);

    drop_all_subs();
}

#[inline]
fn send_workspaces(event: WorkspacesChanged) {
    let map = WORKSPACES_SUBS.read().unwrap();

    for sender in map.values() {
        sender.send(event.clone());
    }
}

#[inline]
fn send_keyboard_layout(device: &str, layout: &str) {
    let layout = get_keyboard_layout(layout);
    let map = KEYBOARD_SUBS.read().unwrap();

    for entry in map.values() {
        if device == entry.device {
            entry.sender.send(KeyboardLayoutChanged {
                device: entry.device,
                layout
            });
        }
    }
}

fn drop_all_subs() {
    // We drop the old maps because we don't want to keep the allocations.
    let mut map = KEYBOARD_SUBS.write().unwrap();
    *map = HashMap::new();

    let mut map = WORKSPACES_SUBS.write().unwrap();
    *map = HashMap::new();
}

async fn dispatch_command(cmd: &[u8]) {
    let Some(mut write_stream) = dispatch(cmd).await else {
        return;
    };

    let mut buf = [0u8; 64];
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

fn get_keyboard_layout(layout: &str) -> Option<&'static str> {
    // We first try the layout string as is and then with the parenthesis removed.
    // For example, "Armenian" and "Armenian (phonetic)" should both evaluate to "am".
    // However "French (Canada)" and "French" evaluate to "ca" and "fr" respectively.

    let result = LAYOUTS.get(layout).copied();

    if result.is_some() {
        return result;
    }

    let index = layout.find(" (")?;
    let layout = &layout[..index];

    LAYOUTS.get(layout).copied()
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

impl<T> Drop for SubscriptionToken<T> {
    fn drop(&mut self) {
        if SUB_COUNT.fetch_sub(1, Ordering::AcqRel) == 1 {
            let mut handle = HANDLE.lock().unwrap();
            if let Some(handle) = handle.as_mut() {
                handle.abort();
            }

            *handle = None;
            drop(handle);

            drop_all_subs();
        } else {
            let mut map = self.handle.write().unwrap();
            map.remove(&self.key);
        }
    }
}
