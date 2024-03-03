use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    convert::TryFrom,
    ops::Deref
};

use tokio::task::JoinSet;
use smallvec::SmallVec;
use zbus::{
    Connection, MessageHeader, SignalContext, Interface,
    names::{BusName, UniqueName},
    zvariant::ObjectPath,
    fdo::{self, DBusProxy, Properties},
    interface,
    export::ordered_stream::OrderedStreamExt
};

pub const DEFAULT_OBJ_PATH: &str = "/StatusNotifierItem";

// This type name is going places...
// The hash map consists of a unique bus name as the key (eg. ":1.33").
// Values are strings in the format "{UNIQUE_NAME}{OBJECT_PATH}" (eg. ":1.33/StatusNotifierItem")
// where the UNIQUE_NAME part is the same as the key because StatusNotifierHosts need it.
// We use a SmallVec because a connection can declare multiple SNI instances (though it's unlikely).
type BusTable = Arc<RwLock<HashMap<String, SmallVec<[String; 2]>>>>;

#[derive(Clone, Debug)]
pub enum ServicePath<'a> {
    Bus(BusName<'a>),
    Obj(ObjectPath<'a>)
}

#[derive(Default, Debug)]
pub struct Watcher {
    hosts: Arc<RwLock<HashSet<String>>>,
    snis: BusTable,
    join_set: JoinSet<()>
}

impl Watcher {
    pub fn listen_for_exit(
        &mut self,
        conn: Connection,
        ctx: SignalContext<'static>
    ) {
        let hosts = self.hosts.clone();
        let snis = self.snis.clone();

        let conn_clone = conn.clone();
        let ctx_clone = ctx.clone();

        self.join_set.spawn(async move {
            loop {
                if let Err(err) = monitor_host_exit(
                    &conn_clone,
                    &ctx_clone,
                    hosts.clone()
                ).await {
                    eprintln!("D-Bus: host service monitor loop crashed: {err}\nRestarting...");
                }
            }
        });

        self.join_set.spawn(async move {
            loop {
                if let Err(err) = monitor_item_exit(
                    &conn,
                    &ctx,
                    snis.clone()
                ).await {
                    eprintln!("D-Bus: item service monitor loop crashed: {err}\nRestarting...");
                }
            }
        });
    }

    // Attempt to re-construct the "{UNIQUE_NAME}{OBJECT_PATH}" string that we created
    // in the register_status_notifier_item method below. We also do our best to parse
    // the strings that other implementations might return... because that's the best we can do.
    // Who cares about specs anyway -_-
    pub fn parse_sni_service_string<'a>(service: &'a str) -> zbus::Result<(BusName<'a>, ObjectPath<'a>)> {
        fn err(service: &str) -> zbus::Error {
            zbus::Error::Address(
                format!("Couldn't parse StatusNotifierItem encoded path in the \"{{UNIQUE_NAME}}{{OBJECT_PATH}}\" form. Got: {service}")
            )
        }

        if !service.is_ascii() {
            return Err(zbus::Error::Address(format!("Not ASCII: {service}")));
        }

        if let Some(index) = service.find('/') {
            // This branch is the best case scenario. It should run if we are using our implementation. 
            let (name, path) = unsafe {
                let bytes = service.as_bytes();

                (
                    std::str::from_utf8_unchecked(&bytes[0..index]),
                    std::str::from_utf8_unchecked(&bytes[index..])
                )
            };

            let name = BusName::try_from(name).map_err(|_| err(service))?;
            let path = ObjectPath::try_from(path).map_err(|_| err(service))?;

            Ok((name, path))
        } else if let Some(index) = service.find(':') {
            // Anything is possible at this point. We try to extract the first unique name and roll with that.
            let name = unsafe {
                let bytes = service.as_bytes();

                std::str::from_utf8_unchecked(&bytes[index..])
            };

            let name = UniqueName::try_from(name).map_err(|_| err(service))?;
            let path = ObjectPath::from_static_str_unchecked(DEFAULT_OBJ_PATH);

            Ok((BusName::Unique(name), path))
        } else {
            Err(err(service))
        }
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    /// RegisterStatusNotifierHost method
    async fn register_status_notifier_host(
        &mut self,
        service: &str,
        #[zbus(header)] 
        header: MessageHeader<'_>,
        #[zbus(signal_context)]
        ctx: SignalContext<'_>
    ) -> fdo::Result<()> {
        let sender = if service.starts_with(":") {
            UniqueName::try_from(service).ok()
        } else {
            header.sender().cloned()
        };

        let Some(sender) = sender else {
            return Err(fdo::Error::InvalidArgs("Bus sender parameter is missing or the provided argument is an invalid unique name.".into()));
        };

        let should_notify = {
            let mut hosts = self.hosts.write().unwrap();

            if !hosts.insert(sender.as_str().into()) {
                return Ok(());
            }

            hosts.len() == 1
        };

        if should_notify {
            self.is_status_notifier_host_registered_changed(&ctx).await?;
        }

        Self::status_notifier_host_registered(&ctx).await?;

        Ok(())
    }

    /// RegisterStatusNotifierItem method
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] 
        header: MessageHeader<'_>,
        #[zbus(signal_context)]
        ctx: SignalContext<'_>
    ) -> fdo::Result<()> {
        let Some(sender) = header.sender() else {
            return Err(fdo::Error::InvalidArgs("Bus sender name is missing or is invalid.".into()));
        };

        let path = ServicePath::try_from(service)?;
        let value = match path {
            ServicePath::Obj(_) => format!("{sender}{service}"),
            ServicePath::Bus(BusName::Unique(_)) => format!("{service}{DEFAULT_OBJ_PATH}"),
            _ => format!("{sender}{DEFAULT_OBJ_PATH}")
        };

        {
            let mut snis = self.snis.write().unwrap();
            let entry = snis.entry(sender.as_str().into()).or_default();

            if entry.contains(&value) {
                return Ok(());
            }

            entry.push(value.clone());
        }

        self.registered_status_notifier_items_changed(&ctx).await?;

        Self::status_notifier_item_registered(&ctx, &value).await?;

        Ok(())
    }

    /// StatusNotifierHostRegistered signal
    #[zbus(signal)]
    async fn status_notifier_host_registered(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// StatusNotifierHostUnregistered signal
    #[zbus(signal)]
    async fn status_notifier_host_unregistered(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// StatusNotifierItemRegistered signal
    #[zbus(signal)]
    async fn status_notifier_item_registered(ctx: &SignalContext<'_>, service: &str) -> zbus::Result<()>;

    /// StatusNotifierItemUnregistered signal
    #[zbus(signal)]
    async fn status_notifier_item_unregistered(ctx: &SignalContext<'_>, service: &str) -> zbus::Result<()>;

    /// IsStatusNotifierHostRegistered property
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        !self.hosts.read().unwrap().is_empty()
    }

    /// RegisteredStatusNotifierItems property
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        let snis = self.snis.read().unwrap();
        let mut result = Vec::with_capacity(snis.len());

        for entry in snis.values() {
            result.extend_from_slice(&entry);
        }

        result
    }

    /// ProtocolVersion property
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

async fn monitor_host_exit(
    conn: &Connection,
    ctx: &SignalContext<'static>,
    table: Arc<RwLock<HashSet<String>>>
) -> fdo::Result<()> {
    let dbus = DBusProxy::new(conn).await?;
    let mut stream = dbus.receive_name_owner_changed().await?;

    while let Some(sig) = stream.next().await {
        let Ok(args) = sig.args() else {
            eprintln!("D-Bus: Couldn't retrieve NameOwnerChanged signal args.");

            continue;
        };

        // The new owner being None means the connection shut down.
        // We check if the connection had created any host objects
        // (that we know of) and remove them if they did. 
        let (should_notify, is_empty) = if let (Some(old_owner), None) = (
            args.old_owner().deref(),
            args.new_owner().deref()
        ) {
            let mut table = table.write().unwrap();

            let notify = table.remove(old_owner.as_str());

            (notify, table.is_empty())
        } else {
            (false, false)
        };

        if should_notify {
            Watcher::status_notifier_host_unregistered(ctx).await?;
        }

        if is_empty {
            Properties::properties_changed(
                ctx,
                Watcher::name(),
                &HashMap::new(),
                &["IsStatusNotifierHostRegistered"]
            ).await?
        }
    }

    Ok(())
}

async fn monitor_item_exit(
    conn: &Connection,
    ctx: &SignalContext<'static>,
    table: BusTable
) -> fdo::Result<()> {
    let dbus = DBusProxy::new(conn).await?;
    let mut stream = dbus.receive_name_owner_changed().await?;

    while let Some(sig) = stream.next().await {
        let Ok(args) = sig.args() else {
            eprintln!("D-Bus: Couldn't retrieve NameOwnerChanged signal args.");

            continue;
        };

        // The new owner being None means the connection shut down.
        // We check if the connection had created any item objects
        // (that we know of) and remove them if they did. 
        let items = if let (Some(old_owner), None) = (
            args.old_owner().deref(),
            args.new_owner().deref()
        ) {
            let mut table = table.write().unwrap();

            table.remove(old_owner.as_str())
        } else {
            None
        };

        if let Some(items) = items {
            Properties::properties_changed(
                ctx,
                Watcher::name(),
                &HashMap::new(),
                &["RegisteredStatusNotifierItems"]
            ).await?;

            for item in &items {
                Watcher::status_notifier_item_unregistered(ctx, item).await?;
            }
        }
    }

    Ok(())
}

impl<'a> TryFrom<&'a str> for ServicePath<'a> {
    type Error = fdo::Error;

    #[inline]
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        fn err(service: &str) -> fdo::Error {
            fdo::Error::InvalidArgs(format!("Invalid StatusNotifierItem service path: {service}"))
        }

        Ok(if value.starts_with("/") {
            ServicePath::Obj(ObjectPath::try_from(value).map_err(|_| err(value))?)
        } else {
            ServicePath::Bus(BusName::try_from(value).map_err(|_| err(value))?)
        })
    }
}
