//! A "ping" event source that wakes up when the user requests it to.

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod eventfd;
        use eventfd as sys;
    } else if #[cfg(unix)] {
        mod pipe;
        use pipe as sys;
    } else {
        compile_error!("The ping feature is only supported on Unix.");
    }
}

use crate::{Event, PollMode, Poller, Result, Source};
use std::sync::Arc;

/// A ping event source that wakes up when the user requests it to.
#[derive(Debug)]
pub struct Ping {
    /// The underlying source.
    source: sys::Ping,
}

/// The notifier that can be used to wake up the ping event source.
#[derive(Debug, Clone)]
pub struct Notifier {
    /// The underlying notifier.
    notifier: sys::Notify,
}

impl Ping {
    /// Creates a new ping event source.
    pub fn new() -> Result<Self> {
        Ok(Self {
            source: sys::Ping::new()?,
        })
    }

    /// Create a new notifier for this ping event source.
    pub fn notifier(&self) -> Notifier {
        Notifier {
            notifier: self.source.notify().clone(),
        }
    }
}

impl Source for Ping {
    fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.source.register(poller, interest, mode)
    }

    fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.source.reregister(poller, interest, mode)
    }

    fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
        self.source.deregister(poller)
    }

    fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
        self.source.handle_event(poller, event)
    }
}

impl Notifier {
    /// Notifies the ping event source.
    pub fn notify(&self) -> Result<()> {
        self.notifier.notify()
    }
}
