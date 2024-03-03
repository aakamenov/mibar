use std::{
    sync::RwLock,
    collections::HashMap,
    time::Duration,
    hash::{Hash, Hasher},
    path::Path,
    process,
    io
};

use zbus::{
    fdo::RequestNameFlags,
    names::WellKnownName,
    export::{
        ordered_stream::OrderedStreamExt,
        futures_util::stream::StreamExt
    },
    Connection, Interface
};
use tokio::{
    task::{JoinSet, JoinHandle},
    runtime,
    time::sleep,
    fs
};
use mibar_core::{window::WindowId, ValueSender};
use once_cell::sync::Lazy;
use ahash::AHasher;

use super::{
    watcher::Watcher,
    status_notifier_watcher::StatusNotifierWatcherProxy,
    status_notifier_item::StatusNotifierItemProxy
};

use crate::sender_key::SenderKey;

static STATE: Lazy<RwLock<State>> = Lazy::new(|| {
    RwLock::new(State {
        handle: None,
        subs: HashMap::new()
    })
});

/// As long as this token is alive the widget will receive new values.
/// Dropping the token will automatically unsubscribe the widget. 
pub struct SubscriptionToken(SenderKey);

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct SniId(u64);

#[derive(Clone, Debug)]
pub enum Event {
    New {
        id: SniId,
        icon_data: Option<Vec<u8>>
    },
    Remove(SniId),
    Crash
}

struct State {
    handle: Option<JoinHandle<()>>,
    subs: HashMap<SenderKey, ValueSender<Event>>
}

pub fn subscribe(
    handle: &runtime::Handle,
    window_id: WindowId,
    sender: ValueSender<Event>
) -> SubscriptionToken {
    let key = SenderKey::new(window_id, &sender);

    let mut state = STATE.write().unwrap();
    state.subs.insert(key, sender);

    if state.handle.is_none() {
        let handle = handle.spawn(async {
            loop {
                match start().await {
                    Ok(_) => unreachable!("D-Bus system tray task should only terminate if an error occurred."),
                    Err(err) => {
                        eprintln!("D-Bus system tray task exited with error: {err}\nRestarting...");

                        {
                            let state = STATE.read().unwrap();
                            state.send(Event::Crash);
                        }

                        // We don't want to spam retries.
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });

        state.handle = Some(handle);
    }

    SubscriptionToken(key)
}

async fn start() -> zbus::Result<()> {
    let conn = Connection::session().await?;

    let watcher = init(&conn).await?;
    let mut new_item_stream = watcher.receive_status_notifier_item_registered().await?;

    // JoinSet will drop all child SNI tasks when this function exits or is itself aborted.
    let mut item_tasks = JoinSet::<()>::new();

    for item in watcher.registered_status_notifier_items().await? {
        if let Err(err) = create_sni(&conn, &mut item_tasks, &item).await {
            eprintln!("Failed to create a StatusNotifierItem task: {err}");
        }
    }

    while let Some(signal) = OrderedStreamExt::next(&mut new_item_stream).await {
        let Ok(args) = signal.args() else {
            eprintln!("New StatusNotifierItem signal: no arguments present.");

            continue;
        };

        if let Err(err) = create_sni(&conn, &mut item_tasks, args.service).await {
            eprintln!("Failed to create a StatusNotifierItem task: {err}");
        }
    }

    Ok(())
}

async fn init(conn: &Connection) -> zbus::Result<StatusNotifierWatcherProxy<'static>> {
    if conn.object_server().at("/StatusNotifierWatcher", Watcher::default()).await? {
        let flags = RequestNameFlags::DoNotQueue;
        match conn.request_name_with_flags(Watcher::name().as_str(), flags.into()).await {
            Ok(_) => {
                let watcher = conn.object_server().interface::<_, Watcher>("/StatusNotifierWatcher").await?;
                
                let ctx = watcher.signal_context().clone();
                watcher.get_mut().await.listen_for_exit(conn.clone(), ctx);
            },
            // There's already a watcher instance so we connect to it.
            Err(zbus::Error::NameTaken) => { }
            Err(e) => return Err(e)
        }
    }

    let watcher = StatusNotifierWatcherProxy::new(&conn).await?;
    let name = format!("org.freedesktop.StatusNotifierHost-{}", process::id());
    let name = WellKnownName::from_string_unchecked(name);
    println!("name: {name}");
    
    conn.request_name_with_flags(name.clone(), RequestNameFlags::ReplaceExisting.into()).await?;
    watcher.register_status_notifier_host(&name).await?;

    Ok(watcher)
}

async fn create_sni(
    conn: &Connection,
    set: &mut JoinSet<()>,
    service: &str
) -> zbus::Result<()> {
    let (dest, path) = Watcher::parse_sni_service_string(service)?;
    let item = StatusNotifierItemProxy::builder(conn)
        .destination(dest.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let id = SniId::new(service);

    set.spawn(async move {
        let mut status_signal = item.receive_status_changed().await;
        let Ok(mut exit_signal) = item.inner().receive_owner_changed().await else {
            eprintln!("Tray: couldn't connect to OwnerChanged signal.");

            return;
        };

        let icon_name = item.icon_name().await.ok();
        let icon_path = item.icon_theme_path().await.ok();
        let icon_pixmaps = item.icon_pixmap().await.ok();
        println!(
            "path: {:?}, name: {:?}",
            icon_path,
            icon_name
        );

        if let Some(pixmaps) = &icon_pixmaps {
            for pixmap in pixmaps {
                println!("width: {}, height: {}, count: {}", pixmap.0, pixmap.1, pixmap.2.len());
            }
        }

        let icon_data = if let (Some(icon_path), Some(icon_name)) = (icon_path, icon_name) {
            let path = Path::new(&icon_path).to_path_buf().join(icon_name);

            match fs::read(path).await {
                Ok(data) => Some(data),
                Err(err) if matches!(err.kind(), io::ErrorKind::NotFound) => None,
                Err(err) => {
                    eprintln!("Tray: error loading icon: {err}");

                    None
                } 
            }
        } else {
            None
        };

        // Visualizations are encouraged to prefer icon names over icon pixmaps if both are available.
        let icon_data = icon_data.or(
            icon_pixmaps.map(|mut x| x.pop()).map(|x| x.map(|x| x.2)).flatten()
        );

        send(Event::New { id, icon_data });

        loop {
            tokio::select! {
                Some(status) = status_signal.next() => {
                    println!("new status: {:?}", status.get().await);
                }
                Some(owner) = exit_signal.next() => {
                    if owner.is_none() {
                        break;
                    }
                }
            }
        }
    });

    Ok(())
}

#[inline]
fn send(event: Event) {
    let state = STATE.read().unwrap();
    state.send(event);
}

impl State {
    #[inline]
    fn send(&self, event: Event) {
        for sender in self.subs.values() {
            sender.send(event.clone());
        }
    }

    fn reset(&mut self) {
        // We drop the old map because we don't want to keep the allocations.
        self.subs = HashMap::new();

        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

impl SniId {
    #[inline]
    fn new(service: &str) -> Self {
        let mut hasher = AHasher::default();
        service.hash(&mut hasher);

        Self(hasher.finish())
    }
}

impl Drop for SubscriptionToken {
    fn drop(&mut self) {
        let mut state = STATE.write().unwrap();
        state.subs.remove(&self.0);

        if state.subs.is_empty() {
            state.reset();
        }
    }
}
