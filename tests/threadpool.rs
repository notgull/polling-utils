use polling_utils::threadpool::UnblockFn;
use polling_utils::{Event, PollMode, Poller, Source};

use std::sync::{mpsc, Arc};
use std::task::Poll;
use std::thread;
use std::time::Duration;

#[test]
fn unblock_fn() {
    let poller = Arc::new(Poller::new().unwrap());
    let (send, recv) = mpsc::channel();
    let mut unblocked = UnblockFn::new(move || {
        // Send the thread handle back and start waiting.
        send.send(thread::current()).unwrap();
        thread::park();
        5
    })
    .unwrap();
    let handle = recv.recv().unwrap();

    // Register the source in the poller.
    unblocked
        .register(&poller, Event::readable(0), PollMode::Level)
        .unwrap();

    // One event to start polling.
    let mut events = vec![];
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));
    unblocked.handle_event(&poller, events[0]).unwrap();

    // After this, the thread should be parked.
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());

    // Unblock the thread.
    handle.unpark();
    thread::sleep(Duration::from_millis(100));

    // One event to stop polling.
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));
    unblocked.handle_event(&poller, events[0]).unwrap();

    // No more events.
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());

    // Resolved now.
    assert_eq!(unblocked.result(), Poll::Ready(5));
}
