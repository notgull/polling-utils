//! Access to a thread pool.

use crate::future::{PollFuture, PollRead, PollWrite};
use crate::{Event, PollMode, Poller, Result, Source};

use blocking::{Task, Unblock};

use std::io;
use std::sync::Arc;
use std::task::Poll;

/// Wait for a function to complete in a thread pool.
pub struct UnblockFn<T> {
    inner: PollFuture<Task<T>>,
}

impl<T: Send + 'static> UnblockFn<T> {
    /// Create a new `UnblockFn` that will run the given function in a thread pool.
    pub fn new<F>(f: F) -> Result<Self>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let task = blocking::unblock(f);
        Ok(Self {
            inner: PollFuture::new(task)?,
        })
    }

    /// Get the result of the function.
    pub fn result(&mut self) -> Poll<T> {
        self.inner.poll_unpin()
    }
}

impl<T: Send + 'static> Source for UnblockFn<T> {
    fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.inner.register(poller, interest, mode)
    }

    fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.inner.reregister(poller, interest, mode)
    }

    fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
        self.inner.deregister(poller)
    }

    fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
        self.inner.handle_event(poller, event)
    }
}

/// Waits for data to be read from a reader in a threadpool.
pub struct UnblockReader<R>(PollRead<Unblock<R>>);

impl<R: io::Read + Send + 'static> UnblockReader<R> {
    /// Create a new `UnblockReader` that will read from the given reader in a threadpool.
    pub fn new(reader: R) -> Result<Self> {
        let unblock = Unblock::new(reader);
        Ok(Self(PollRead::new(unblock)?))
    }

    /// Create a new `UnblockReader` with a given pipe capacity.
    pub fn with_capacity(reader: R, capacity: usize) -> Result<Self> {
        let unblock = Unblock::with_capacity(capacity, reader);
        Ok(Self(PollRead::new(unblock)?))
    }

    /// Read from the reader.
    pub fn read(&mut self, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        self.0.poll_unpin(buf)
    }
}

impl<R: io::Read + Send + 'static> Source for UnblockReader<R> {
    fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
        self.0.deregister(poller)
    }

    fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
        self.0.handle_event(poller, event)
    }

    fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.0.register(poller, interest, mode)
    }

    fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.0.reregister(poller, interest, mode)
    }
}

/// Waits for data to be written to a writer in a threadpool.
pub struct UnblockWriter<W>(PollWrite<Unblock<W>>);

impl<W: io::Write + Send + 'static> UnblockWriter<W> {
    /// Create a new `UnblockWriter` that will write to the given writer in a threadpool.
    pub fn new(writer: W) -> Result<Self> {
        let unblock = Unblock::new(writer);
        Ok(Self(PollWrite::new(unblock)?))
    }

    /// Create a new `UnblockWriter` with a given pipe capacity.
    pub fn with_capacity(writer: W, capacity: usize) -> Result<Self> {
        let unblock = Unblock::with_capacity(capacity, writer);
        Ok(Self(PollWrite::new(unblock)?))
    }

    /// Write to the writer.
    pub fn write(&mut self, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.0.poll_unpin(buf)
    }
}

impl<W: io::Write + Send + 'static> Source for UnblockWriter<W> {
    fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
        self.0.deregister(poller)
    }

    fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
        self.0.handle_event(poller, event)
    }

    fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.0.register(poller, interest, mode)
    }

    fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.0.reregister(poller, interest, mode)
    }
}
