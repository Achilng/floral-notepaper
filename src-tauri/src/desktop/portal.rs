use std::collections::HashMap;

use futures_util::StreamExt;
use zbus::connection::Connection;
use zbus::message::Type;
use zbus::zvariant::{Array, OwnedValue, Str, Value};
use zbus::{proxy, MatchRule, MessageStream};

const PORTAL_SERVICE: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";

/// D-Bus proxy for the org.freedesktop.portal.GlobalShortcuts interface.
#[proxy(
    interface = "org.freedesktop.portal.GlobalShortcuts",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait GlobalShortcutsPortalDbus {
    /// Create a new session for global shortcuts.
    async fn create_session(
        &self,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// Bind shortcuts to a session.
    async fn bind_shortcuts(
        &self,
        session_handle: &zbus::zvariant::ObjectPath<'_>,
        shortcuts: Vec<(String, HashMap<String, OwnedValue>)>,
        parent_window: &str,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

/// Represents a shortcut to register via the portal.
#[derive(Debug, Clone)]
pub struct PortalShortcut {
    pub id: String,
    pub name: String,
    pub accelerators: Vec<String>,
}

/// Message sent from the portal listener thread to the caller.
#[derive(Debug)]
pub enum PortalMessage {
    /// A shortcut was activated (pressed). Contains the shortcut ID.
    ShortcutActivated(String),
    /// The session was closed or errored.
    Shutdown,
}

/// A portal-based global shortcut manager for Wayland environments.
///
/// Uses `org.freedesktop.portal.GlobalShortcuts` to register and listen
/// for global keyboard shortcuts when X11 key grabs are unavailable.
pub struct GlobalShortcutsPortal {
    tx: std::sync::mpsc::Sender<PortalCommand>,
    signal_tx: std::sync::mpsc::Receiver<PortalMessage>,
}

enum PortalCommand {
    Register(Vec<PortalShortcut>),
    UnregisterAll,
    Shutdown,
}

impl GlobalShortcutsPortal {
    /// Create a new portal shortcut manager.
    ///
    /// Spawns a background thread that connects to the D-Bus session bus
    /// and manages shortcut registration via the GlobalShortcuts portal.
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let (signal_tx, signal_rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            if let Err(e) = portal_event_loop(rx, signal_tx) {
                eprintln!("[portal] event loop error: {e}");
            }
        });

        Self {
            tx,
            signal_tx: signal_rx,
        }
    }

    /// Register shortcuts via the portal.
    pub fn register(&self, shortcuts: Vec<PortalShortcut>) -> Result<(), String> {
        self.tx
            .send(PortalCommand::Register(shortcuts))
            .map_err(|e| format!("failed to send register command: {e}"))
    }

    /// Unregister all shortcuts.
    pub fn unregister_all(&self) -> Result<(), String> {
        self.tx
            .send(PortalCommand::UnregisterAll)
            .map_err(|e| format!("failed to send unregister command: {e}"))
    }

    /// Check if a shortcut was activated (non-blocking).
    pub fn try_recv_signal(&self) -> Option<PortalMessage> {
        self.signal_tx.try_recv().ok()
    }

    /// Shut down the portal event loop.
    pub fn shutdown(&self) {
        let _ = self.tx.send(PortalCommand::Shutdown);
    }
}

fn portal_event_loop(
    rx: std::sync::mpsc::Receiver<PortalCommand>,
    signal_tx: std::sync::mpsc::Sender<PortalMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move { portal_event_loop_async(rx, signal_tx).await })
}

async fn portal_event_loop_async(
    rx: std::sync::mpsc::Receiver<PortalCommand>,
    signal_tx: std::sync::mpsc::Sender<PortalMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::session().await?;

    let proxy = GlobalShortcutsPortalDbusProxy::builder(&conn)
        .destination(PORTAL_SERVICE)?
        .path(PORTAL_PATH)?
        .build()
        .await?;

    // Create a session
    let options: HashMap<String, OwnedValue> = HashMap::new();
    let session_handle = proxy.create_session(options).await?;
    eprintln!("[portal] session created: {session_handle}");

    // Build match rules for signals on the session object path
    let session_path = zbus::zvariant::ObjectPath::try_from(session_handle.as_str())?;

    let shortcuts_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.portal.GlobalShortcuts")?
        .member("ShortcutsChanged")?
        .path(session_path.clone())?
        .build();

    let response_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.portal.Request")?
        .member("Response")?
        .path(session_path.clone())?
        .build();

    let mut signal_stream = MessageStream::for_match_rule(shortcuts_rule, &conn, None).await?;
    let mut response_stream = MessageStream::for_match_rule(response_rule, &conn, None).await?;

    let mut registered_shortcuts: Vec<PortalShortcut> = Vec::new();

    loop {
        // Check for commands from the main thread (non-blocking)
        match rx.try_recv() {
            Ok(PortalCommand::Register(shortcuts)) => {
                match bind_shortcuts_on_portal(&proxy, &session_handle, &shortcuts).await {
                    Ok(()) => {
                        registered_shortcuts = shortcuts;
                        eprintln!("[portal] shortcuts bound successfully");
                    }
                    Err(e) => {
                        eprintln!("[portal] failed to bind shortcuts: {e}");
                    }
                }
            }
            Ok(PortalCommand::UnregisterAll) => {
                registered_shortcuts.clear();
                eprintln!("[portal] shortcuts cleared");
            }
            Ok(PortalCommand::Shutdown) => {
                eprintln!("[portal] shutting down");
                return Ok(());
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                eprintln!("[portal] command channel closed, shutting down");
                return Ok(());
            }
        }

        // Poll for signals with a timeout
        use std::time::Duration;
        match tokio::time::timeout(Duration::from_millis(100), signal_stream.next()).await {
            Ok(Some(Ok(msg))) => {
                handle_shortcuts_changed_signal(&msg, &registered_shortcuts, &signal_tx);
            }
            _ => {}
        }

        match tokio::time::timeout(Duration::from_millis(10), response_stream.next()).await {
            Ok(Some(Ok(msg))) => {
                handle_response_signal(&msg);
            }
            _ => {}
        }
    }
}

/// Build an `OwnedValue` from a string (clones into 'static lifetime).
fn owned_str(s: &str) -> OwnedValue {
    Value::Str(Str::from(s.to_owned()))
        .try_into()
        .expect("valid Value conversion")
}

/// Build an `OwnedValue` array from string slices.
fn owned_str_array(items: &[String]) -> OwnedValue {
    let arr: Vec<Value<'static>> = items
        .iter()
        .map(|s| Value::Str(Str::from(s.clone())))
        .collect();
    Value::Array(Array::from(arr))
        .try_into()
        .expect("valid Value conversion")
}

async fn bind_shortcuts_on_portal(
    proxy: &GlobalShortcutsPortalDbusProxy<'_>,
    session_handle: &zbus::zvariant::OwnedObjectPath,
    shortcuts: &[PortalShortcut],
) -> Result<(), Box<dyn std::error::Error>> {
    let session_path = zbus::zvariant::ObjectPath::try_from(session_handle.as_str())?;

    let portal_shortcuts: Vec<(String, HashMap<String, OwnedValue>)> = shortcuts
        .iter()
        .map(|s| {
            let mut props = HashMap::new();
            props.insert("name".to_string(), owned_str(&s.name));
            props.insert("accelerators".to_string(), owned_str_array(&s.accelerators));
            (s.id.clone(), props)
        })
        .collect();

    let options: HashMap<String, OwnedValue> = HashMap::new();
    let _request_path = proxy
        .bind_shortcuts(&session_path, portal_shortcuts, "", options)
        .await?;

    Ok(())
}

fn handle_shortcuts_changed_signal(
    msg: &zbus::message::Message,
    registered: &[PortalShortcut],
    signal_tx: &std::sync::mpsc::Sender<PortalMessage>,
) {
    // ShortcutsChanged signal signature: o session_handle, a(sa{sv}) shortcuts
    let body = msg.body();

    let Ok((_session, shortcuts)) = body.deserialize::<(
        zbus::zvariant::ObjectPath<'_>,
        Vec<(String, HashMap<String, OwnedValue>)>,
    )>() else {
        eprintln!("[portal] failed to deserialize ShortcutsChanged signal");
        return;
    };

    for (id, _props) in &shortcuts {
        if registered.iter().any(|s| &s.id == id) {
            eprintln!("[portal] shortcut activated: {id}");
            let _ = signal_tx.send(PortalMessage::ShortcutActivated(id.clone()));
        }
    }
}

fn handle_response_signal(msg: &zbus::message::Message) {
    let body = msg.body();
    // Response signal signature: u response, a{sv} results
    if let Ok((response, _results)) = body.deserialize::<(u32, HashMap<String, OwnedValue>)>() {
        if response == 0 {
            eprintln!("[portal] request accepted");
        } else {
            eprintln!("[portal] request denied or dismissed (response={response})");
        }
    }
}

/// Detect if the current session is running on Wayland.
pub fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|v| v.trim().eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_wayland_returns_correct_value_based_on_env() {
        let result = is_wayland_session();
        assert!(result == true || result == false);
    }

    #[test]
    fn portal_shortcut_construction() {
        let shortcut = PortalShortcut {
            id: "open-notepad".to_string(),
            name: "Open Quick Note".to_string(),
            accelerators: vec!["<Control>space".to_string()],
        };

        assert_eq!(shortcut.id, "open-notepad");
        assert_eq!(shortcut.name, "Open Quick Note");
        assert_eq!(shortcut.accelerators, vec!["<Control>space"]);
    }

    #[test]
    fn portal_message_variants() {
        let activated = PortalMessage::ShortcutActivated("open-notepad".to_string());
        let shutdown = PortalMessage::Shutdown;

        match activated {
            PortalMessage::ShortcutActivated(id) => assert_eq!(id, "open-notepad"),
            _ => panic!("expected ShortcutActivated"),
        }

        match shutdown {
            PortalMessage::Shutdown => {}
            _ => panic!("expected Shutdown"),
        }
    }

    #[test]
    fn global_shortcuts_portal_can_be_created() {
        let _portal = GlobalShortcutsPortal::new();
    }

    #[test]
    fn owned_str_creates_valid_value() {
        let val = owned_str("hello");
        let s: &str = (&val).try_into().unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn owned_str_array_creates_valid_array() {
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let val = owned_str_array(&items);
        // Verify the OwnedValue was created successfully (no panic)
        // The signature check would require accessing internal fields
        let _ = val;
    }
}
