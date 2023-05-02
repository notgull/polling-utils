//! A "ping" event source that uses IOCP to wake up the event loop.

use crate::{Event, PollMode, Poller, Result, Source};
use std::{sync::{Arc, Mutex, Weak, MutexGuard}, io};

use polling::os::iocp::{PollerIocpExt, CompletionPacket};

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

    /// Are we currently in the IOCP thread?
    in_port: bool,

    /// Did we *just* receive this event?
    edge_trigger: bool,
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
            in_port: false,
            edge_trigger: false,
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
        // Set the interest.
        let mut inner = self.lock();
        inner.interest = Some(Interest {
            packet: CompletionPacket::new(interest),
            poller: Arc::downgrade(poller),
            mode,
        });

        let is_edge = match mode {
            PollMode::Oneshot | PollMode::Level => false,
            PollMode::Edge | PollMode::EdgeOneshot => true, 
            _ => return Err(io::Error::new(
                io::ErrorKind::Other,
                "unsupported polling mode for IOCP",
            ))
        };

        // If we aren't currently in-port and we're ready to be notified, then notify the poller.
        if !is_edge && (!inner.in_port && inner.notified > 0) {
            inner.wake()?;
        }

        Ok(())
    }

    pub(super) fn deregister(&mut self, _poller: &Arc<Poller>) -> Result<()> {
        // Clear the interest.
        let mut inner = self.lock();
        inner.interest = None;
        inner.in_port = false;
        inner.edge_trigger = false;

        Ok(())
    }

    pub(super) fn handle_event(&mut self, _poller: &Arc<Poller>, _event: Event) -> Result<()> {
        // We are no longer in port.
        let mut inner = self.lock();
        inner.notified = inner.notified.saturating_sub(1);
        inner.in_port = false;
        inner.edge_trigger = false;
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
        inner.edge_trigger = true;
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
            self.in_port = true;
        }

        Ok(())
    }
}
