//! A channel that can be kneaded into a [`Poller`].

use crate::future::PollFuture;
use crate::{Event, PollMode, Poller, Result, Source};

use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::{fmt, io};

type GenFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Create a new, unbounded channel.
pub fn unbounded<T: Send + 'static>() -> Result<(Sender<T>, Receiver<T>)> {
    let (sender, receiver) = async_channel::unbounded();
    from_channel(sender, receiver)
}

fn from_channel<T: Send + 'static>(
    sender: async_channel::Sender<T>,
    receiver: async_channel::Receiver<T>,
) -> Result<(Sender<T>, Receiver<T>)> {
    let sender = Sender { inner: sender };

    let receiver = Receiver {
        future: PollFuture::new(Box::pin({
            let receiver = receiver.clone();
            async move { receiver.recv().await.ok() }
        }) as GenFuture<Option<T>>)?,
        inner: receiver,
    };

    Ok((sender, receiver))
}

/// The sender side of a channel.
#[derive(Debug)]
pub struct Sender<T> {
    inner: async_channel::Sender<T>,
}

/// The receiver side of a channel.
pub struct Receiver<T> {
    future: PollFuture<GenFuture<Option<T>>>,
    inner: async_channel::Receiver<T>,
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver").finish_non_exhaustive()
    }
}

impl<T> Sender<T> {
    /// Send a value into the channel.
    pub fn send(&self, value: T) -> Result<()> {
        self.inner
            .try_send(value)
            .map_err(|_| io::Error::from(io::ErrorKind::Other))
    }
}

impl<T: Send + 'static> Receiver<T> {
    /// Receive a value from the channel.
    pub fn recv(&mut self) -> Option<T> {
        match self.future.poll_unpin() {
            Poll::Ready(value) => value,
            _ => None,
        }
    }
}

impl<T: Send + 'static> Source for Receiver<T> {
    fn register(
        &mut self,
        poller: &std::sync::Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        self.future.register(poller, interest, mode)
    }

    fn reregister(
        &mut self,
        poller: &std::sync::Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        self.future.reregister(poller, interest, mode)?;

        // Reset the future.
        *self.future.future_mut() = Box::pin({
            let receiver = self.inner.clone();
            async move { receiver.recv().await.ok() }
        }) as GenFuture<Option<T>>;

        Ok(())
    }

    fn deregister(&mut self, poller: &std::sync::Arc<Poller>) -> Result<()> {
        self.future.deregister(poller)
    }

    fn handle_event(&mut self, poller: &std::sync::Arc<Poller>, event: Event) -> Result<()> {
        self.future.handle_event(poller, event)
    }
}
