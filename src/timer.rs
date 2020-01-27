//! A simple sleep timer. This timer is runtime agnostic; it users a single
//! global background thread with a binary heap of wakers to wake tasks as
//! needed.

use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd, Reverse};
use std::collections::BinaryHeap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex, Once, Weak};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

use lazy_static::lazy_static;

#[derive(Debug)]
struct SleepingTask {
    wake_at: Instant,
    waker: Weak<Waker>,
}

impl SleepingTask {
    /// Wake the inner waker. Doesn't check should_wake.
    #[inline]
    fn wake(self) {
        if let Some(waker) = Weak::upgrade(&self.waker) {
            waker.wake_by_ref();
        }
    }

    #[inline]
    fn needs_wakeup(&self, cutoff: &Instant) -> bool {
        cutoff >= &self.wake_at
    }
}

impl PartialEq<Self> for SleepingTask {
    #[inline]
    fn eq(&self, rhs: &Self) -> bool {
        self.wake_at == rhs.wake_at
    }
}

impl Eq for SleepingTask {}

impl PartialOrd<Self> for SleepingTask {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SleepingTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wake_at.cmp(&other.wake_at)
    }
}

#[derive(Debug, Default)]
struct SleepTable {
    // TODO: Consider replacing this with a BTreeMap<Instant, Vec<Weak<Waker>>>.
    // This might have better performance, especially for several wakers all
    // with the same timeout, especially since a single task is something
    // like 24 bytes.
    sleepers: BinaryHeap<Reverse<SleepingTask>>,
}

impl SleepTable {
    #[inline]
    fn new() -> Self {
        Self::default()
    }

    /// Adds a new task to the queue. Returns true if this task needs to
    /// wake up earlier than any others.
    #[inline]
    fn add(&mut self, task: SleepingTask) -> bool {
        let is_new_earliest = match self.next_wakeup() {
            Some(current_wakeup) => &task.wake_at < current_wakeup,
            None => true,
        };

        self.sleepers.push(Reverse(task));

        is_new_earliest
    }

    #[inline]
    fn next_wakeup(&self) -> Option<&Instant> {
        self.sleepers.peek().map(|sleeper| &sleeper.0.wake_at)
    }

    #[inline]
    fn needs_wakeup(&self, cutoff: &Instant) -> bool {
        match self.sleepers.peek() {
            Some(sleeper) => sleeper.0.needs_wakeup(cutoff),
            None => false,
        }
    }

    // Wake every sleeper with a wake_at <= the cutoff
    #[inline]
    fn awaken(&mut self, cutoff: &Instant) {
        while self.needs_wakeup(cutoff) {
            self.sleepers.pop().unwrap().0.wake()
        }
    }
}

fn global_schedule(wake_at: Instant, waker: Weak<Waker>) {
    // TODO: currently, this has to be a lazy_static, because there's no const
    // initializer for Mutex.
    lazy_static! {
        static ref SLEEPERS: Mutex<SleepTable> = Mutex::new(SleepTable::new());
        static ref ALARM_CLOCK: Condvar = Condvar::new();
    }

    // The first time global_schedule, we spawn the thread that listens for
    // scheduled sleepers and awakens them as necessary.
    static SPAWN_THREAD: Once = Once::new();

    // There's no way to stop this thread once it's started. We just let it
    // die when main returns.
    SPAWN_THREAD.call_once(|| {
        thread::spawn(|| {
            let mut locked_sleepers = SLEEPERS.lock().unwrap();

            loop {
                // Note: we could add a needs_wakeup condition in a loop here,
                // to deal with spurious wakeups. However, that condition is
                // already checked by awaken, so we don't need it.
                locked_sleepers = match locked_sleepers.next_wakeup() {
                    Some(alarm_time) => {
                        let duration = alarm_time.duration_since(Instant::now());
                        ALARM_CLOCK
                            .wait_timeout(locked_sleepers, duration)
                            .unwrap()
                            .0
                    }
                    None => ALARM_CLOCK.wait(locked_sleepers).unwrap(),
                };

                locked_sleepers.awaken(&Instant::now());
            }
        });
    });

    let is_new_earliest = {
        let mut locked_sleepers = SLEEPERS.lock().unwrap();
        locked_sleepers.add(SleepingTask { wake_at, waker })
    };

    // Only need to notify if the sleep timer changed.
    if is_new_earliest {
        ALARM_CLOCK.notify_one();
    }
}

#[derive(Debug, Clone)]
pub struct SleepUntil {
    wake_at: Instant,
    waker: Option<Arc<Waker>>,
}

impl Future for SleepUntil {
    type Output = ();

    #[inline]
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<()> {
        let now = Instant::now();
        let wake_at = self.wake_at;

        if now >= wake_at {
            // Drop our stored waker, to ensure any cleanup is done
            let _waker = self.waker.take();
            Poll::Ready(())
        } else {
            let ctx_waker = ctx.waker();

            match self.waker.as_mut() {
                // We've already been scheduled, and out saved waker matches
                // our context's waker, so there's no need to reschedule.
                Some(waker) if waker.will_wake(ctx_waker) => {}

                // Either we haven't yet been scheduled, or we've previously
                // scheduled ourselves but our context waker doesn't match the
                // stored waker. Either way, schedule ourselves.
                _ => {
                    let waker = Arc::new(ctx_waker.clone());
                    global_schedule(wake_at, Arc::downgrade(&waker));
                    self.waker = Some(waker);
                }
            }

            Poll::Pending
        }
    }
}

/// Create a future that completes when a given Instant is reached. Awaiting
/// this future will schedule a wakeup at the given time.
pub fn sleep_until(wake_at: Instant) -> SleepUntil {
    SleepUntil {
        wake_at,
        waker: None,
    }
}

/// Create a future that completes after a given duration. The duration
/// calculation is made as soon as this function is called; it does not wait
/// until a future await.
pub fn sleep(duration: Duration) -> SleepUntil {
    sleep_until(Instant::now() + duration)
}

/// TODO: some future timeouts. Should be easy to compose with sleep_until.
