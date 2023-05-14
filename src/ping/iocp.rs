//! A "ping" event source that uses IOCP to wake up the event loop.

use crate::{Event, PollMode, Poller, Result};
use std::{
    io,
    sync::{Arc, Mutex, MutexGuard, Weak},
};

use polling::os::iocp::{CompletionPacket, PollerIocpExt};

macro_rules! lock {
    ($e:expr) => {{
        ($e).unwrap_or_else(|x| x.into_inner())
    }};
}

/// A ping event source that wakes up when the user requests it to.
#[derive(Debug)]
pub(super) struct Ping(Notify);

/// The notifier that can be used to wake up the ping event source.
#[derive(Debug, Clone)]
pub(super) struct Notify(Arc<Mutex<Inner>>);

#[derive(Debug)]
struct Inner {
    /// The registered interest.
    interest: Option<Interest>,

    /// The number of times we have been notified.
    notified: usize,
}

// TODO: CompletionPacket is thread safe
unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

#[derive(Debug)]
struct Interest {
    /// The underlying poller.
    poller: Weak<Poller>,

    /// The underlying completion packet.
    packet: CompletionPacket,

    /// The polling mode we are using.
    mode: PollMode,
}

impl Ping {
    /// Creates a new ping event source.
    pub fn new() -> Result<Self> {
        Ok(Self(Notify(Arc::new(Mutex::new(Inner {
            interest: None,
            notified: 0,
        })))))
    }

    pub fn notify(&self) -> &Notify {
        &self.0
    }

    pub(super) fn register(
        &mut self,
        poller: &Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        self.reregister(poller, interest, mode)
    }

    pub(super) fn reregister(
        &mut self,
        poller: &Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        match mode {
            PollMode::Oneshot | PollMode::Level => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "unsupported polling mode for IOCP",
                ))
            }
        }

        // Set the interest.
        let mut inner = self.lock();
        inner.interest = Some(Interest {
            packet: CompletionPacket::new(interest),
            poller: Arc::downgrade(poller),
            mode,
        });

        Ok(())
    }

    pub(super) fn deregister(&mut self, _poller: &Arc<Poller>) -> Result<()> {
        // Clear the interest.
        let mut inner = self.lock();
        inner.interest = None;

        Ok(())
    }

    pub(super) fn handle_event(&mut self, _poller: &Arc<Poller>, _event: Event) -> Result<()> {
        // We are no longer in port.
        let mut inner = self.lock();
        inner.notified = inner.notified.saturating_sub(1);
        Ok(())
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        lock!((self.0).0.lock())
    }
}

impl Notify {
    pub(super) fn notify(&self) -> Result<()> {
        let mut inner = lock!(self.0.lock());
        inner.notified = inner.notified.saturating_add(1);
        inner.wake()
    }
}

impl Inner {
    fn wake(&mut self) -> Result<()> {
        if let Some(interest) = &self.interest {
            let poller = match interest.poller.upgrade() {
                Some(poller) => poller,
                None => return Ok(()),
            };

            poller.post(interest.packet.clone())?;

            // If we are in oneshot mode, remove future interest.
            if matches!(interest.mode, PollMode::EdgeOneshot | PollMode::Oneshot) {
                self.interest = None;
            }
        }

        Ok(())
    }
}
