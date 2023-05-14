use polling_utils::ping::Ping;
use polling_utils::{Event, PollMode, Poller, Source};

use std::sync::Arc;
use std::time::Duration;

#[test]
fn oneshot() {
    let poller = Arc::new(Poller::new().unwrap());
    let mut ping = Ping::new().unwrap();
    let notifier = ping.notifier();

    // Register the source in the poller.
    ping.register(&poller, Event::readable(0), PollMode::Oneshot)
        .unwrap();

    // No events.
    let mut events = vec![];
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());

    // Notify.
    notifier.notify().unwrap();

    // Wait for the event.
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));
    ping.handle_event(&poller, events[0]).unwrap();

    // No more events.
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());

    // Re-register and go again.
    ping.reregister(&poller, Event::readable(0), PollMode::Oneshot)
        .unwrap();
    notifier.notify().unwrap();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Event::readable(0));
    ping.handle_event(&poller, events[0]).unwrap();

    // De-register and go again.
    ping.deregister(&poller).unwrap();
    notifier.notify().unwrap();
    events.clear();
    poller
        .wait(&mut events, Some(Duration::from_millis(100)))
        .unwrap();
    assert!(events.is_empty());
}
