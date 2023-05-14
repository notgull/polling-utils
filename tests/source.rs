use polling_utils::{Event, PollMode, Poller, Socket, Source};

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn oneshot() {
    let poller = Arc::new(Poller::new().unwrap());
    let (reader, mut writer) = tcp_pipe();
    let mut reader = Socket::new(reader);

    // Register the source in the poller.
    reader
        .register(&poller, Event::readable(0), PollMode::Oneshot)
        .unwrap();

    // No events.
    let mut events = vec![];
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());

    // Write some data.
    writer.write_all(b"hello").unwrap();

    // Wait for the event.
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));

    // No more events.
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());

    // Re-register and go again.
    reader
        .reregister(&poller, Event::readable(0), PollMode::Oneshot)
        .unwrap();
    writer.write_all(b"hello").unwrap();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));

    // De-register and go again.
    reader.deregister(&poller).unwrap();
    writer.write_all(b"hello").unwrap();
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());
}

#[test]
fn level() {
    let poller = Arc::new(Poller::new().unwrap());
    if !poller.supports_level() {
        return;
    }

    let (reader, mut writer) = tcp_pipe();
    let mut reader = Socket::new(reader);

    // Register the source in the poller.
    reader
        .register(&poller, Event::readable(0), PollMode::Level)
        .unwrap();

    // No events.
    let mut events = vec![];
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());

    // Write some data.
    writer.write_all(b"hello").unwrap();

    // Wait for the event.
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));

    // If we wait again the event should still be there.
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));

    // Re-register and go again.
    reader
        .reregister(&poller, Event::readable(0), PollMode::Level)
        .unwrap();
    writer.write_all(b"hello").unwrap();
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));

    // De-register and go again.
    reader.deregister(&poller).unwrap();
    writer.write_all(b"hello").unwrap();
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());
}

fn tcp_pipe() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stream1 = TcpStream::connect(addr).unwrap();
    let stream2 = listener.accept().unwrap().0;
    (stream1, stream2)
}
