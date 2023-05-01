//! Access to a thread pool.

use crate::future::PollFuture;
use crate::{Event, PollMode, Poller, Result, Source};

use blocking::Task;
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
