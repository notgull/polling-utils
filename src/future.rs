//! Poll a future using the `polling` loop.

use std::future::Future;
use std::sync::Arc;
use std::pin::Pin;
use std::task::{Poll, Context, Waker, Wake};

use crate::ping::{Ping, Notifier};
use crate::{Source, Event, PollMode, Poller, Result};

pin_project_lite::pin_project! {
    /// A wrapper around a future to be polled.
    #[derive(Debug)]
    pub struct PollFuture<F: ?Sized> {
        // The ping event source.
        ping: Ping,

        // The waker to be used to wake up the poll loop.
        waker: Waker,

        // The future to be polled.
        #[pin]
        future: F,
    }
}

impl<F: Future + ?Sized> PollFuture<F> {
    /// Creates a new future to be polled.
    pub fn new(future: F) -> Result<Self> where F: Sized {
        let ping = Ping::new()?;
        let waker = Waker::from(Arc::new(Notify(ping.notifier())));
        Ok(Self {
            ping,
            waker,
            future,
        })
    }

    /// Get a reference to the future.
    pub fn future(&self) -> &F {
        &self.future
    }

    /// Get a mutable reference to the future.
    pub fn future_mut(&mut self) -> &mut F {
        &mut self.future
    }

    /// Get a pinned reference to the future.
    pub fn future_pin_mut(self: Pin<&mut Self>) -> Pin<&mut F> {
        self.project().future
    }

    /// Poll this future to completion.
    pub fn poll(self: Pin<&mut Self>) -> Poll<F::Output> {
        let this = self.project();
        let mut cx = Context::from_waker(this.waker);
        this.future.poll(&mut cx)
    }

    /// Poll this future to completion, but without pinning.
    pub fn poll_unpin(&mut self) -> Poll<F::Output> where F: Unpin {
        Pin::new(self).poll()
    }
}

impl<F: Future + ?Sized> Source for PollFuture<F> {
    fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.ping.register(poller, interest, mode)?;

        // We want to be polled right off the bat, so wake us up once.
        self.waker.wake_by_ref();
        Ok(())
    }

    fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.ping.reregister(poller, interest, mode)
    }

    fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
        self.ping.deregister(poller)
    }

    fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
        self.ping.handle_event(poller, event)
    }
}

struct Notify(Notifier);

impl Wake for Notify {
    fn wake(self: Arc<Self>) {
        self.0.notify().expect("failed to notify");
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.notify().expect("failed to notify");
    }
}
