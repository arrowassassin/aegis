//! App state. One `Store` of `Copy` signals shared via context — the Rust
//! analogue of the original component's `this.state`.

use dioxus::prelude::*;
use crate::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Held,
    Feed,
    Provenance,
    Recorder,
    Audit,
    Policy,
    Snapshots,
    Settings,
    Verified,
    Capability,
    Fleet,
}

impl Screen {
    /// Title + subtitle shown in the title bar and topbar.
    pub fn meta(self) -> (&'static str, &'static str) {
        use Screen::*;
        match self {
            Dashboard => ("Home", "A calm overview of what your agents are doing"),
            Held => ("Needs review", "One command is paused, waiting for your decision"),
            Feed => ("Activity", "Everything your agents do, as it happens"),
            Provenance => ("Where it came from", "How untrusted content reached a risky command"),
            Recorder => ("Recorder", "Also records what you type in the terminal — no AI needed"),
            Audit => ("History", "A complete, tamper-proof record you can trust"),
            Policy => ("Rules", "What gets allowed, paused, or blocked"),
            Snapshots => ("Undo", "Restore points saved before anything destructive"),
            Settings => ("Settings", "Protection, recording, and how agents are connected"),
            Verified => ("Verified gate", "Proven-correct safety guarantees — planned for V2"),
            Capability => ("Capability scopes", "Give each tool only what it needs — planned for V2"),
            Fleet => ("Team & fleet", "Manage protection across a whole team — planned for V2"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModelStatus {
    None,
    Downloading,
    Installed,
}

/// All signals are `Copy`, so the whole store is `Copy` and can be pulled from
/// context anywhere with `use_store()` and mutated directly.
#[derive(Clone, Copy)]
pub struct Store {
    pub screen: Signal<Screen>,
    pub nav_open: Signal<bool>,
    pub theme: Signal<Theme>,

    // auth
    pub unlocked: Signal<bool>,
    pub pw: Signal<String>,
    pub pw_error: Signal<bool>,
    pub attempts: Signal<u32>,

    // held command
    pub held_resolved: Signal<Option<&'static str>>, // "denied" | "allowed" | "always"
    pub panic: Signal<bool>,

    // activity feed
    pub feed_filter: Signal<&'static str>, // all | held | blocked | tainted
    pub feed_search: Signal<String>,
    pub feed_page: Signal<usize>,

    // settings
    pub recording: Signal<bool>,
    pub watchdog: Signal<bool>,
    pub fail_closed: Signal<bool>,
    pub require_pw: Signal<bool>,
    pub autostart: Signal<bool>,

    // local model
    pub model_status: Signal<ModelStatus>,
    pub model_id: Signal<Option<&'static str>>,
    pub model_progress: Signal<f64>,
    pub model_search: Signal<String>,
}

pub const MASTER_PW: &str = "kintsugi";
pub const FEED_PAGE_SIZE: usize = 9;

impl Store {
    pub fn new() -> Self {
        Store {
            screen: Signal::new(Screen::Dashboard),
            nav_open: Signal::new(true),
            theme: Signal::new(Theme::Dark),
            unlocked: Signal::new(false),
            pw: Signal::new(String::new()),
            pw_error: Signal::new(false),
            attempts: Signal::new(0),
            held_resolved: Signal::new(None),
            panic: Signal::new(false),
            feed_filter: Signal::new("all"),
            feed_search: Signal::new(String::new()),
            feed_page: Signal::new(1),
            recording: Signal::new(true),
            watchdog: Signal::new(true),
            fail_closed: Signal::new(true),
            require_pw: Signal::new(true),
            autostart: Signal::new(true),
            model_status: Signal::new(ModelStatus::None),
            model_id: Signal::new(None),
            model_progress: Signal::new(0.0),
            model_search: Signal::new(String::new()),
        }
    }

    pub fn try_unlock(&mut self) {
        if *self.attempts.read() >= 5 {
            return;
        }
        if self.pw.read().as_str() == MASTER_PW {
            self.unlocked.set(true);
            self.pw.set(String::new());
            self.pw_error.set(false);
            self.attempts.set(0);
        } else {
            self.pw_error.set(true);
            self.pw.set(String::new());
            let a = *self.attempts.read() + 1;
            self.attempts.set(a);
        }
    }

    pub fn lock(&mut self) {
        self.unlocked.set(false);
        self.pw.set(String::new());
        self.pw_error.set(false);
        self.screen.set(Screen::Dashboard);
    }
}

pub fn use_store() -> Store {
    use_context::<Store>()
}
