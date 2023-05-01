// SPDX-License-Identifer: MIT OR Apache-2.0

//! A handful of utilities to make writing code with the [`polling`] crate ergonomic.
//!
//! When writing code in Rust to react to incoming events, it is common to use the `async`/`await`
//! style of programming as a way of abstracting over the event loop pattern. However, sometimes it is
//! preferred to use the more traditional event loop style of programming, where the user is responsible
//! for polling the event loop and handling events as they come in. This crate provides a selection
//! of utilities to make writing code in this style using the [`polling`] crate more ergonomic.
//!
//! It is heavily recommended to use `async`/`await` with the [`smol`] crate instead of using this
//! crate. This crate is only provided for those who cannot use `async`/`await` for whatever reason.
//!
//! [`polling`]: https://docs.rs/polling
//! [`smol`]: https://docs.rs/smol

#![forbid(unsafe_code)]

use polling::Source as PSource;
#[doc(inline)]
pub use polling::{Event, PollMode, Poller};

use std::io::Result;
use std::sync::Arc;

#[cfg(feature = "future")]
pub mod future;
#[cfg(feature = "ping")]
pub mod ping;
#[cfg(feature = "threadpool")]
pub mod threadpool;

/// A source that can be registered into a [`Poller`].
///
/// [`Poller`]: polling::Poller
pub trait Source {
    /// Registers the source into the given [`Poller`].
    ///
    /// [`Poller`]: polling::Poller
    fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()>;

    /// Re-registers the source into the given [`Poller`].
    ///
    /// [`Poller`]: polling::Poller
    fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()>;

    /// Deregisters the source from the given [`Poller`].
    fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()>;

    /// Handles an event that was received from the given [`Poller`].
    fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()>;
}

/// The typical socket source registed into the [`Poller`].
#[derive(Debug)]
pub struct Socket<T: ?Sized> {
    /// The event that we are interested in.
    interest: Option<Interest>,

    /// The underlying socket.
    socket: T,
}

#[derive(Debug)]
struct Interest {
    event: Event,
    mode: PollMode,
}

impl<T> Socket<T> {
    /// Creates a new socket source.
    pub fn new(socket: T) -> Self {
        Self {
            interest: None,
            socket,
        }
    }

    /// Get a reference to the underlying socket.
    pub fn socket(&self) -> &T {
        &self.socket
    }

    /// Get a mutable reference to the underlying socket.
    pub fn socket_mut(&mut self) -> &mut T {
        &mut self.socket
    }

    /// Convert the socket into the underlying socket.
    pub fn into_socket(self) -> T {
        self.socket
    }
}

impl<T> Source for Socket<T>
where
    for<'a> &'a T: PSource,
{
    fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.interest = Some(Interest {
            event: interest,
            mode,
        });
        poller.add_with_mode(&self.socket, interest, mode)
    }

    fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
        self.interest = Some(Interest {
            event: interest,
            mode,
        });
        poller.modify_with_mode(&self.socket, interest, mode)
    }

    fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
        if self.interest.take().is_some() {
            poller.delete(&self.socket)
        } else {
            Ok(())
        }
    }

    fn handle_event(&mut self, _poller: &Arc<Poller>, _event: Event) -> Result<()> {
        Ok(())
    }
}
