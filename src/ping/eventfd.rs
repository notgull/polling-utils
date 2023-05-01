//! A ping event source built on a Linux eventfd.

use rustix::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};
use rustix::io::{eventfd, read, write, EventfdFlags};

use crate::{Event, PollMode, Poller, Result, Socket, Source};

use std::sync::Arc;

#[derive(Debug)]
pub(super) struct Ping {
    /// The eventfd.
    eventfd: Socket<Notify>,
}

#[derive(Debug, Clone)]
pub(super) struct Notify(Arc<OwnedFd>);

impl AsRawFd for Notify {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl AsFd for Notify {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Ping {
    pub(super) fn new() -> Result<Self> {
        let efd = eventfd(
            0,
            EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK | EventfdFlags::SEMAPHORE,
        )?;
        Ok(Self {
            eventfd: Socket::new(Notify(Arc::new(efd))),
        })
    }

    pub(super) fn notify(&self) -> &Notify {
        self.eventfd.socket()
    }

    pub(super) fn register(
        &mut self,
        poller: &Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        self.eventfd.register(poller, interest, mode)
    }

    pub(super) fn reregister(
        &mut self,
        poller: &Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        self.eventfd.reregister(poller, interest, mode)
    }

    pub(super) fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
        self.eventfd.deregister(poller)
    }

    pub(super) fn handle_event(&mut self, poller: &Arc<Poller>, interest: Event) -> Result<()> {
        // Drain the eventfd.
        read(self.eventfd.socket(), &mut [0u8; 8])?;
        self.eventfd.handle_event(poller, interest)
    }
}

impl Notify {
    pub(super) fn notify(&self) -> Result<()> {
        write(self, &1u64.to_ne_bytes())?;
        Ok(())
    }
}
