//! Timer wheels.

use crate::ping::{Notifier, Ping};
use crate::{Event, PollMode, Poller, Result, Source};

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// A timer wheel that contains timers.
#[derive(Debug)]
pub struct TimerWheel {
    /// The timers in the wheel.
    timers: BTreeMap<(Instant, usize), Notifier>,

    /// The last ID that was assigned to a timer.
    last_id: usize,
}

/// A timer that can be used to wake up the timer wheel.
#[derive(Debug)]
pub struct Timer {
    /// The current ID of the timer.
    id: usize,

    /// The underlying ping event source.
    ping: Ping,

    /// The timeout of the timer.
    deadline: Option<Instant>,

    /// The interval of the timer.
    interval: Duration,
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerWheel {
    /// Creates a new timer wheel.
    pub fn new() -> Self {
        Self {
            timers: BTreeMap::new(),
            last_id: 1,
        }
    }

    /// Create a new timer that fires after the given duration.
    pub fn after(&mut self, duration: Duration) -> Result<Timer> {
        Instant::now()
            .checked_add(duration)
            .map(|deadline| self.at(deadline))
            .unwrap_or_else(Timer::never)
    }

    /// Create a new timer that fires at this instant.
    pub fn at(&mut self, deadline: Instant) -> Result<Timer> {
        self.interval_at(deadline, Duration::MAX)
    }

    /// Create a timer that fires on an interval.
    pub fn interval(&mut self, interval: Duration) -> Result<Timer> {
        self.interval_at(Instant::now(), interval)
    }

    /// Create a new timer that fires after the given duration, at the given interval.
    pub fn interval_at(&mut self, start: Instant, interval: Duration) -> Result<Timer> {
        // Create a new ping event source.
        let ping = Ping::new()?;
        let notifier = ping.notifier();

        // Create a new timer.
        let timer = Timer {
            id: self.last_id,
            ping,
            deadline: start.checked_add(interval),
            interval,
        };
        self.last_id += 1;

        // Register the timer.
        if let Some(deadline) = timer.deadline {
            self.timers.insert((deadline, timer.id), notifier);
        }

        Ok(timer)
    }

    /// Fire all pending timers.
    pub fn fire_timers(&mut self) -> Result<Option<Duration>> {
        // Get the current time.
        let now = Instant::now();
        let mut notifiers = vec![];

        // Get all timers that have expired.
        let mut expired = self.timers.split_off(&(now, 0));
        std::mem::swap(&mut self.timers, &mut expired);

        // Fire all expired timers.
        notifiers.extend(expired.into_values());

        // See how long we need to wait for the next timer.
        let next = if self.timers.is_empty() {
            None
        } else {
            Some(
                self.timers
                    .keys()
                    .next()
                    .unwrap()
                    .0
                    .saturating_duration_since(now),
            )
        };

        // Notify all expired timers.
        for notifier in notifiers {
            notifier.notify()?;
        }

        Ok(next)
    }
}

impl Timer {
    /// Create a timer that never fires.
    pub fn never() -> Result<Self> {
        Ok(Self {
            id: 0,
            ping: Ping::new()?,
            deadline: None,
            interval: Duration::MAX,
        })
    }

    /// Insert this timer back into the timer wheel.
    pub fn handle_wheel(&mut self, wheel: &mut TimerWheel) -> Result<()> {
        // Re-insert the timer into the wheel.
        if let Some(deadline) = self.deadline {
            wheel
                .timers
                .insert((deadline, self.id), self.ping.notifier());
        }

        Ok(())
    }
}

impl Source for Timer {
    fn deregister(&mut self, poller: &std::sync::Arc<Poller>) -> Result<()> {
        self.ping.deregister(poller)
    }

    fn handle_event(&mut self, poller: &std::sync::Arc<Poller>, event: Event) -> Result<()> {
        self.ping.handle_event(poller, event)?;

        // If this is a timer that fires on an interval, bump up the duration.
        if self.deadline.is_some() {
            self.deadline = self
                .deadline
                .and_then(|deadline| deadline.checked_add(self.interval));
        }

        Ok(())
    }

    fn register(
        &mut self,
        poller: &std::sync::Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        self.ping.register(poller, interest, mode)
    }

    fn reregister(
        &mut self,
        poller: &std::sync::Arc<Poller>,
        interest: Event,
        mode: PollMode,
    ) -> Result<()> {
        self.ping.reregister(poller, interest, mode)
    }
}
