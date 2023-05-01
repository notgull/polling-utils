//! A ping event source built on a pipe.

use rustix::fd::OwnedFd;
use rustix::io::{pipe, fcntl_getfd, fcntl_setfd, FdFlags, pipe_with, PipeFlags, write, read};

use crate::{Socket, PollMode, Event, Poller, Result, Source};
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct Ping {
    /// The read end of the pipe.
    reader: Socket<OwnedFd>,

    /// The write end of the pipe.
    writer: Notify,
}

#[derive(Debug, Clone)]
pub(super) struct Notify(Arc<OwnedFd>);

impl Ping {
    pub(super) fn new() -> Result<Self> {
        // Create a new pipe.
        let (reader, writer) = pipe_with(PipeFlags::CLOEXEC)
            .or_else(|_| {
                // If we failed to atomically create a pipe with the `CLOEXEC` flag, we try to
                // create a pipe without it and then set the flag manually.
                let (reader, writer) = pipe()?;

                // Set the `CLOEXEC` flag on the writer end.
                fcntl_setfd(&writer, fcntl_getfd(&writer)? | FdFlags::CLOEXEC)?;
                
                // Set the `CLOEXEC` flag on the reader end.
                fcntl_setfd(&reader, fcntl_getfd(&reader)? | FdFlags::CLOEXEC)?;

                Result::Ok((reader, writer))
            })?;

        Ok(Self {
            reader: Socket::new(reader),
            writer: Notify(Arc::new(writer)),
        })
    }

    pub(super) fn notify(&self) -> &Notify {
        &self.writer
    }

    pub(super) fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.reader.register(poller, interest, mode)
    }

    pub(super) fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.reader.reregister(poller, interest, mode)
    }

    pub(super) fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
        self.reader.deregister(poller)
    }

    pub(super) fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
        read(self.reader.socket(), &mut [0u8])?;
        self.reader.handle_event(poller, event)
    }
}

impl Notify {
    pub(super) fn notify(&self) -> Result<()> {
        write(&self.0, &[0u8])?;
        Ok(())
    }
}
