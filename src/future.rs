//! Poll a future using the `polling` loop.

#[cfg(feature = "futures-io")]
macro_rules! cfg_futures_io {
    ($($i:item)*) => {$($i)*};
}

#[cfg(not(feature = "futures-io"))]
macro_rules! cfg_futures_io {
    ($($i:item)*) => {};
}

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use pin_project_lite::pin_project;

use crate::ping::{Notifier, Ping};
use crate::{Event, PollMode, Poller, Result, Source};

cfg_futures_io! {
    use futures_io::{AsyncRead, AsyncWrite, AsyncSeek};
    use std::io::SeekFrom;
}

pin_project! {
    /// A wrapper around a future to be polled.
    #[derive(Debug)]
    pub struct PollFuture<F: ?Sized> {
        #[pin]
        inner: PollFutureWithArg<F>,
    }
}

pin_project! {
    /// A wrapper around a future to be polled.
    #[derive(Debug)]
    pub(crate) struct PollFutureWithArg<F: ?Sized> {
        // The ping event source.
        ping: Ping,

        // The waker to be used to wake up the poll loop.
        waker: Waker,

        // The future to be polled.
        #[pin]
        future: F,
    }
}

cfg_futures_io! {
    pin_project! {
        /// A wrapper around an asynchronous reader.
        #[derive(Debug)]
        pub struct PollRead<R: ?Sized> {
            #[pin]
            inner: PollFutureWithArg<ReadPoller<R>>
        }
    }

    pin_project! {
        /// A wrapper around an asynchronous writer.
        #[derive(Debug)]
        pub struct PollWrite<W: ?Sized> {
            #[pin]
            inner: PollFutureWithArg<WritePoller<W>>
        }
    }

    pin_project! {
        /// A wrapper around an asynchronous seeker.
        #[derive(Debug)]
        pub struct PollSeek<S: ?Sized> {
            #[pin]
            inner: PollFutureWithArg<SeekPoller<S>>
        }
    }
}

impl<F: FutureWithArg + ?Sized> PollFutureWithArg<F> {
    /// Creates a new future to be polled.
    pub(crate) fn new_with_arg(future: F) -> Result<Self>
    where
        F: Sized,
    {
        let ping = Ping::new()?;
        let waker = Waker::from(Arc::new(Notify(ping.notifier())));
        Ok(Self {
            ping,
            waker,
            future,
        })
    }

    /// Get a reference to the future.
    pub(crate) fn future(&self) -> &F {
        &self.future
    }

    /// Get a mutable reference to the future.
    pub(crate) fn future_mut(&mut self) -> &mut F {
        &mut self.future
    }

    /// Get a pinned reference to the future.
    pub(crate) fn future_pin_mut(self: Pin<&mut Self>) -> Pin<&mut F> {
        self.project().future
    }

    /// Poll this future to completion.
    pub(crate) fn poll(self: Pin<&mut Self>, arg: &mut F::Argument<'_>) -> Poll<F::Output> {
        let this = self.project();
        let mut cx = Context::from_waker(this.waker);
        this.future.poll_with_arg(&mut cx, arg)
    }

    /// Poll this future to completion, but without pinning.
    pub(crate) fn poll_unpin(&mut self, arg: &mut F::Argument<'_>) -> Poll<F::Output>
    where
        F: Unpin,
    {
        Pin::new(self).poll(arg)
    }

    pub(crate) fn register(
        self: Pin<&mut Self>,
        poller: &Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        let this = self.project();
        this.ping.register(poller, interest, mode)?;

        // We want to be polled right off the bat, so wake us up once.
        this.waker.wake_by_ref();
        Ok(())
    }

    pub(crate) fn reregister(
        self: Pin<&mut Self>,
        poller: &Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        let this = self.project();
        this.ping.reregister(poller, interest, mode)
    }

    pub(crate) fn deregister(self: Pin<&mut Self>, poller: &Arc<Poller>) -> Result<()> {
        let this = self.project();
        this.ping.deregister(poller)
    }

    pub(crate) fn handle_event(
        self: Pin<&mut Self>,
        poller: &Arc<Poller>,
        event: Event,
    ) -> Result<()> {
        let this = self.project();
        this.ping.handle_event(poller, event)
    }
}

impl<F: Future + ?Sized> PollFuture<F> {
    /// Creates a new future to be polled.
    pub fn new(future: F) -> Result<Self>
    where
        F: Sized,
    {
        Ok(Self {
            inner: PollFutureWithArg::new_with_arg(future)?,
        })
    }

    /// Get a reference to the future.
    pub fn future(&self) -> &F {
        self.inner.future()
    }

    /// Get a mutable reference to the future.
    pub fn future_mut(&mut self) -> &mut F {
        self.inner.future_mut()
    }

    /// Get a pinned reference to the future.
    pub fn future_pin_mut(self: Pin<&mut Self>) -> Pin<&mut F> {
        self.project().inner.future_pin_mut()
    }

    /// Poll this future to completion.
    pub fn poll(self: Pin<&mut Self>) -> Poll<F::Output> {
        self.project().inner.poll(&mut ())
    }

    /// Poll this future to completion, but without pinning.
    pub fn poll_unpin(&mut self) -> Poll<F::Output>
    where
        F: Unpin,
    {
        self.inner.poll_unpin(&mut ())
    }
}

cfg_futures_io! {
    impl<R: AsyncRead + ?Sized> PollRead<R> {
        /// Creates a new reader to be polled.
        pub fn new(reader: R) -> Result<Self>
        where
            R: Sized,
        {
            Ok(Self {
                inner: PollFutureWithArg::new_with_arg(ReadPoller { reader })?,
            })
        }

        /// Get a reference to the reader.
        pub fn reader(&self) -> &R {
            &self.inner.future().reader
        }

        /// Get a mutable reference to the reader.
        pub fn reader_mut(&mut self) -> &mut R {
            &mut self.inner.future_mut().reader
        }

        /// Get a pinned reference to the reader.
        pub fn reader_pin_mut(self: Pin<&mut Self>) -> Pin<&mut R> {
            self.project().inner.future_pin_mut().project().reader
        }

        /// Poll this reader to completion.
        pub fn poll(self: Pin<&mut Self>, buf: &mut [u8]) -> Poll<Result<usize>> {
            self.project().inner.poll(buf)
        }

        /// Poll this reader to completion, but without pinning.
        pub fn poll_unpin(&mut self, buf: &mut [u8]) -> Poll<Result<usize>>
        where
            R: Unpin,
        {
            self.inner.poll_unpin(buf)
        }
    }

    impl<W: AsyncWrite + ?Sized> PollWrite<W> {
        /// Creates a new writer to be polled.
        pub fn new(writer: W) -> Result<Self>
        where
            W: Sized,
        {
            Ok(Self {
                inner: PollFutureWithArg::new_with_arg(WritePoller { writer })?,
            })
        }

        /// Get a reference to the writer.
        pub fn writer(&self) -> &W {
            &self.inner.future().writer
        }

        /// Get a mutable reference to the writer.
        pub fn writer_mut(&mut self) -> &mut W {
            &mut self.inner.future_mut().writer
        }

        /// Get a pinned reference to the writer.
        pub fn writer_pin_mut(self: Pin<&mut Self>) -> Pin<&mut W> {
            self.project().inner.future_pin_mut().project().writer
        }

        /// Poll this writer to completion.
        pub fn poll(self: Pin<&mut Self>, mut buf: &[u8]) -> Poll<Result<usize>> {
            self.project().inner.poll(&mut buf)
        }

        /// Poll this writer to completion, but without pinning.
        pub fn poll_unpin(&mut self, mut buf: &[u8]) -> Poll<Result<usize>>
        where
            W: Unpin,
        {
            self.inner.poll_unpin(&mut buf)
        }
    }

    impl<S: AsyncSeek + ?Sized> PollSeek<S> {
        /// Creates a new seeker to be polled.
        pub fn new(seeker: S) -> Result<Self>
        where
            S: Sized,
        {
            Ok(Self {
                inner: PollFutureWithArg::new_with_arg(SeekPoller { seeker })?,
            })
        }

        /// Get a reference to the seeker.
        pub fn seeker(&self) -> &S {
            &self.inner.future().seeker
        }

        /// Get a mutable reference to the seeker.
        pub fn seeker_mut(&mut self) -> &mut S {
            &mut self.inner.future_mut().seeker
        }

        /// Get a pinned reference to the seeker.
        pub fn seeker_pin_mut(self: Pin<&mut Self>) -> Pin<&mut S> {
            self.project().inner.future_pin_mut().project().seeker
        }

        /// Poll this seeker to completion.
        pub fn poll(self: Pin<&mut Self>, mut pos: SeekFrom) -> Poll<Result<u64>> {
            self.project().inner.poll(&mut pos)
        }

        /// Poll this seeker to completion, but without pinning.
        pub fn poll_unpin(&mut self, mut pos: SeekFrom) -> Poll<Result<u64>>
        where
            S: Unpin,
        {
            self.inner.poll_unpin(&mut pos)
        }
    }
}

macro_rules! wrapper_around_inner {
    (
        impl <$($param:ident: $gen:ident)?> Source for $ty:ty { .. }
    ) => {
        impl <$($param: $gen + Unpin + ?Sized)?> Source for $ty {
            fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
                Pin::new(self)
                    .project()
                    .inner
                    .register(poller, interest, mode)
            }

            fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
                Pin::new(self)
                    .project()
                    .inner
                    .reregister(poller, interest, mode)
            }

            fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
                Pin::new(self).project().inner.deregister(poller)
            }

            fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
                Pin::new(self).project().inner.handle_event(poller, event)
            }
        }

        impl<$($param: $gen + ?Sized)?> Source for Pin<&mut $ty> {
            fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
                self.as_mut().project().inner.deregister(poller)
            }

            fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
                self.as_mut().project().inner.handle_event(poller, event)
            }

            fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
                self.as_mut()
                    .project()
                    .inner
                    .register(poller, interest, mode)
            }

            fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
                self.as_mut()
                    .project()
                    .inner
                    .reregister(poller, interest, mode)
            }
        }

        impl<$($param: $gen + ?Sized)?> Source for Pin<Box<$ty>> {
            fn deregister(&mut self, poller: &Arc<Poller>) -> Result<()> {
                self.as_mut().project().inner.deregister(poller)
            }

            fn handle_event(&mut self, poller: &Arc<Poller>, event: Event) -> Result<()> {
                self.as_mut().project().inner.handle_event(poller, event)
            }

            fn register(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
                self.as_mut()
                    .project()
                    .inner
                    .register(poller, interest, mode)
            }

            fn reregister(&mut self, poller: &Arc<Poller>, interest: Event, mode: PollMode) -> Result<()> {
                self.as_mut()
                    .project()
                    .inner
                    .reregister(poller, interest, mode)
            }
        }
    }
}

wrapper_around_inner! {
    impl<F: Future> Source for PollFuture<F> { .. }
}

cfg_futures_io! {
    wrapper_around_inner! {
        impl<R: AsyncRead> Source for PollRead<R> { .. }
    }

    wrapper_around_inner! {
        impl<W: AsyncWrite> Source for PollWrite<W> { .. }
    }

    wrapper_around_inner! {
        impl<S: AsyncSeek> Source for PollSeek<S> { .. }
    }
}

/// Poll an async future with an argument.
///
/// Good for wrappers like `PollRead` and `PollWrite`.
pub(crate) trait FutureWithArg {
    type Output;
    type Argument<'a>: 'a + ?Sized;

    fn poll_with_arg(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        arg: &mut Self::Argument<'_>,
    ) -> Poll<Self::Output>;
}

impl<F: Future + ?Sized> FutureWithArg for F {
    type Argument<'a> = ();
    type Output = F::Output;

    fn poll_with_arg(self: Pin<&mut Self>, cx: &mut Context<'_>, _: &mut ()) -> Poll<Self::Output> {
        self.poll(cx)
    }
}

cfg_futures_io! {
    pin_project! {
        #[derive(Debug)]
        struct ReadPoller<R: ?Sized> {
            #[pin]
            reader: R,
        }
    }

    impl<R: AsyncRead + ?Sized> FutureWithArg for ReadPoller<R> {
        type Argument<'a> = [u8];
        type Output = Result<usize>;

        fn poll_with_arg(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            arg: &mut [u8],
        ) -> Poll<Self::Output> {
            let this = self.project();
            this.reader.poll_read(cx, arg)
        }
    }

    pin_project! {
        #[derive(Debug)]
        struct WritePoller<W: ?Sized> {
            #[pin]
            writer: W,
        }
    }

    impl<W: AsyncWrite + ?Sized> FutureWithArg for WritePoller<W> {
        type Argument<'a> = &'a [u8];
        type Output = Result<usize>;

        fn poll_with_arg(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            arg: &mut &[u8],
        ) -> Poll<Self::Output> {
            let this = self.project();
            this.writer.poll_write(cx, arg)
        }
    }

    pin_project! {
        #[derive(Debug)]
        struct SeekPoller<S: ?Sized> {
            #[pin]
            seeker: S,
        }
    }

    impl<S: AsyncSeek + ?Sized> FutureWithArg for SeekPoller<S> {
        type Argument<'a> = SeekFrom;
        type Output = Result<u64>;

        fn poll_with_arg(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            arg: &mut Self::Argument<'_>,
        ) -> Poll<Self::Output> {
            let this = self.project();
            this.seeker.poll_seek(cx, *arg)
        }
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
