//! A homegrown dataloader. This was created because the one in crates.io
//! has a proliferation of boxes that makes in unsuitable for references and
//! so on. No caching for now.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
use std::mem;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_timer::Delay;

use Poll::{Pending, Ready};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Token(usize);

/// A set of keys passed into a batch loader function. Use the `keys` method
/// to get the set of keys, all of which will be unique, so that you can
/// execute your request. Then, use the `into_key_values` function to
/// transform your response data into a ValueSet, which is handed back to the
/// batch loader.
#[derive(Debug)]
pub struct KeySet<Key: Hash + Eq> {
    keys: HashMap<Key, (Token, u32)>,
}

impl<Key: Hash + Eq> KeySet<Key> {
    #[inline]
    fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    #[inline]
    fn add_key(&mut self, key: Key) -> Token {
        let new_token = Token(self.keys.len());

        let (token, _) = self
            .keys
            .entry(key)
            .and_modify(|&mut (_, ref mut count)| *count += 1)
            .or_insert((new_token, 0));

        token
    }

    #[inline]
    fn take(&mut self) -> Self {
        mem::take(&mut self.keys)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Get an iterator over all the keys in this keyset. These are guaranteed
    /// to be:
    ///
    /// - Unique
    /// - Between 1 and the configured max_keys of the related BatchRules
    /// - In an arbitrary order
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &Key> + Debug + Clone {
        self.keys.keys()
    }

    /// After you've complete your request, use this method to pair each value
    /// in your result with its key. This is the only way to create a ValueSet,
    /// which is then returned from your batch function.
    #[inline]
    pub fn into_values<Value>(self, get_value: impl FnMut(&Key) -> Value) -> ValueSet<Value> {
        enum Never {}

        self.try_into_values(move |key| -> Result<Value, Never> { Ok(get_value(key)) })
    }

    /// Fallible version of into_values. Same as into_values, but will return
    /// an error the first time `get_value` returns an error.
    #[inline]
    pub fn try_into_values<Value, Error>(
        self,
        get_value: impl FnMut(&Key) -> Result<Value, Error>,
    ) -> Result<ValueSet<Value>, Error> {
        ValueSet {
            values: self
                .keys
                .into_iter()
                .map(move |(key, (token, count))| (token, (get_value(&key)?, count)))
                .collect()?,
        }
    }
}

/// A value set is an opaque data structure that contains the result of a batch
/// operation. It is created with `KeySet::into_values`, and is used by the
/// Batcher functionality to distribute the values to the correct waiting
/// futures.
///
/// In particular, this type helps with duplicate keys. If more than one future
/// requests the same `Key`, that key will only be present once in the `KeySet`,
/// but the `Value` will be cloned for each extra future that was waiting for
/// it.
#[derive(Debug)]
pub struct ValueSet<Value> {
    values: HashMap<Token, (Value, u32)>,
}

impl<Value: Clone> ValueSet<Value> {
    fn take(&mut self, token: Token) -> Option<Value> {
        // TODO: Replace this with RawEntry
        match self.values.entry(token) {
            Entry::Vacant(..) => None,
            Entry::Occupied(entry) => match entry.get_mut() {
                &mut (_, 0) => Some(entry.remove()),
                &mut (value, count) => {
                    *count -= 1;
                    Some(value.clone())
                }
            },
        }
    }
}

/// The result of trying to add a key to a BatchState. This operation will
/// fail if the batchstate has already launched and is inprogress or done. If
/// the BatchState is still accumulating keys, it will succeed, but if it hits
/// the key limit, it will immediately launch the request and not accept any
/// more keys, indicated by AddedLast.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AddKeyResult<Key> {
    Added(Token),
    AddedLast(Token),
    Fail(Key),
}

/// A BatchState is a Future-like object that encodes the state of a single
/// collection of keys through its lifespan of accumulating keys, issuing
/// a single batched request, and distributing the results to the individual
/// futures.
///
/// A set of BatchFutures shares ownerhip of a single BatchState. There is
/// no background execution; all the polling is driven by the individual
/// futures.
enum BatchState<
    'a,
    Key: Hash + Eq,
    Value: Clone,
    Error: Clone,
    Load: Fn(KeySet<Key>) -> Fut,
    Fut: Future<Output = Result<ValueSet<Value>, Error>>,
> {
    /// We're still in the window where new requests are coming in
    Accumulating {
        load: &'a Load,
        keys: KeySet<Key>,
        delay: Delay,
    },

    /// The request has been sent as is pending
    InProgress(Fut),

    /// The request completed
    Done(Result<ValueSet<Value>, Error>),
}

impl<
        'a,
        Key: Hash + Eq + Debug,
        Value: Clone,
        Error: Clone,
        Load: Fn(KeySet<Key>) -> Fut,
        Fut: Future<Output = Result<ValueSet<Value>, Error>>,
    > BatchState<'a, Key, Value, Error, Load, Fut>
{
    /// Create a new BatchState for a set of keys. In order to fullfill our
    /// interface contracts, ensure that `keys` has at least one key when
    /// creating a BatchState.
    ///
    /// Note that the `duration` timer will start as soon as this method is
    /// called; it does not wait until an .await to start the countdown.
    ///
    // TODO: change `keys` to `initial_key`. Need to make sure we return the
    // token in this case.
    #[inline]
    fn new(load: &'a Load, duration: Duration, keys: KeySet<Key>) -> Self {
        let delay = Delay::new(duration);

        BatchState::Accumulating { load, keys, delay }
    }

    /// Attempt to add a key to an accumulating batch state. Returns the result
    /// of the addition:
    ///
    /// - If the key couldn't be added for some reason, return a failure.
    /// - If the key was added and the max was reached, return AddedLast.
    ///    - In this case, the internal timer will be reset so that the batch
    ///      is launched immediately.
    /// - If the key was added but there's still room, return Added.
    ///
    /// This method will panic if it somehow managed to add a key above
    /// max_keys, because doing so is almost certainly a logic error (probably
    /// a max_keys that is too small (0 or 1), or somehow an initial keyset
    /// was added with too many keys.). The easiest way to avoid this panic
    /// is to drop your SharedBatchState as soon as you see an AddedLast.
    fn add_key(&mut self, key: Key, max_keys: usize) -> AddKeyResult<Key> {
        match self {
            BatchState::Accumulating {
                ref mut keys,
                ref mut delay,
                ..
            } => {
                let token = keys.add_key(key);

                if keys.len() < max_keys {
                    AddKeyResult::Added(token)
                } else if keys.len() == max_keys {
                    delay.reset(Duration::from_secs(0));
                    AddKeyResult::AddedLast(token)
                } else {
                    panic!("Somehow added too many keys to a BatchFuture. This shouldn't be possible. keys: {:?}", key)
                }
            }
            _ => AddKeyResult::Fail(key),
        }
    }

    /// Execute a poll operation for a particular Token. This is pretty
    /// straightforward: wait for the timer, then use `load` to launch
    /// the request, then wait for the response. Ensure that `self` is updated
    /// appropriately throughout this process.
    fn poll_token(&mut self, ctx: &mut Context, token: Token) -> Poll<Result<Value, Error>> {
        // TODO: find a way to make this an async fn. The trouble is that our
        // clients need to be able to modify keys while we're in the accumulating
        // state.
        use BatchState::*;

        match self {
            Accumulating { keys, delay, load } => match delay.poll(ctx) {
                Pending => Pending,
                Ready(()) => {
                    let keys = keys.take();
                    let fut = load(keys);
                    match fut.poll(ctx) {
                        Pending => {
                            *self = InProgress(fut);
                            Pending
                        }
                        Ready(batch_result) => {
                            let result = batch_result
                                .as_mut()
                                .map(|values| values.take(token).unwrap())
                                .map_err(|err| err.clone());

                            self = Done(batch_result);
                            Ready(result)
                        }
                    }
                }
            },
            InProgress(fut) => match fut.poll(ctx) {
                Pending => Pending,
                Ready(batch_result) => {
                    let result = batch_result
                        .as_mut()
                        .map(|values| values.take(token).unwrap())
                        .map_err(|err| err.clone());

                    self = Done(batch_result);
                    Ready(result)
                }
            },
            Done(batch_result) => Ready(
                batch_result
                    .map(|values| values.take(token).unwrap())
                    .map_err(|err| err.clone()),
            ),
        }
    }
}

/// An shared pointer to a BatchState (specifically, an Option<Arc<BatchState>>).
/// Contains various helper methods that forward to BatchState.
struct SharedBatchState<
    'a,
    Key: Hash + Eq,
    Value: Clone,
    Error: Clone,
    Load: Fn(KeySet<Key>) -> Fut,
    Fut: Future<Output = Result<ValueSet<Value>, Error>>,
> {
    state: Option<Arc<Mutex<BatchState<'a, Key, Value, Error, Load, Fut>>>>,
}

impl<
        'a,
        Key: Hash + Eq,
        Value: Clone,
        Error: Clone,
        Load: Fn(KeySet<Key>) -> Fut,
        Fut: Future<Output = Result<ValueSet<Value>, Error>>,
    > SharedBatchState<'a, Key, Value, Error, Load, Fut>
{
    // TODO: several different concerns are represented among the methods here.
    // Split up SharedBatchState into several types, each with their own
    // correct and minimal method set. In particular, we have:
    //
    // - Methods related to the correct operation of the future, which doesn't
    // need the ability to add keys to the state
    // - Methods related to the correct creation of new futures, which doesn't
    // need the ability to do polling
    //
    // Relatedly, the BatchState held by Dispatcher should probably be a Weak
    // pointer, anyway.

    /// Poll this state. Panics if the pointer has been nulled. If the poll
    /// returns ready; the state is discarded; this ensures we don't
    /// attempt to poll the ValueSet with a key it doesn't have, which in turn
    /// means that that method is allowed to assume that all requested keys
    /// definitely exist.
    fn poll_token(&mut self, ctx: &mut Context, token: Token) -> Poll<Result<Value, Error>> {
        // Note that this lock only exists for the duration of a poll, not an
        // entire await, and polls by definition are very quick (so as to be
        // nonblocking). We assume that whatever async runtime we're using
        // doesn't have a lot of threads, and mutexes are generally very fast
        // in a low contention environment, so this should be fine.
        //
        // The main way this goes bad is if there are any panics. All the
        // panics this library can emit are well-defined as logic errors– for
        // instance, polling a completed future, trying to send too many key
        // into a BatchState, etc.
        let state_lock = self
            .state
            .expect("Can't poll a completed BatchFuture")
            .lock()
            .unwrap();

        match state_lock.poll_token(ctx, token) {
            Pending => Pending,
            Ready(result) => {
                self.state = None;
                Ready(result)
            }
        }
    }

    /// Unconditionally create a new BatchState from a key. The future is
    /// returned, and this SharedBatchState is updated to share that future's
    /// state.
    fn add_key_new_state(
        &mut self,
        key: Key,
        load: &'a Load,
        window: Duration,
    ) -> BatchFuture<'a, Key, Value, Error, Load, Fut> {
        let keys = KeySet::new();
        let token = keys.add(key);
        let state = BatchState::new(load, window, keys);
        let arc = Arc::new(Mutex::new(state));

        self.state = Some(arc.clone());
        BatchFuture::new(token, arc)
    }

    /// Attempt to add a key to this Batch State. Several things can happen
    /// here:
    ///
    /// - The state is Accumulating. Add this key to the set of interesting
    ///   keys. If this makes the set full, dispatch it immediately (by
    ///   resetting its local timer).
    /// - The state is either null, or not accumulating. Add create a new state.
    ///
    /// The new SharedBatchState is returned, and can be used to create a new
    /// BatchFuture.
    ///
    /// This function does need to take &mut self, because it will change
    /// the local pointer as needed. In the future hopefully this can be
    /// managed with an Atomic.
    fn add_key(
        &mut self,
        key: Key,
        rules: &'a BatchRules<Key, Value, Error, Load, Fut>,
    ) -> BatchFuture<'a, Key, Value, Error, Load, Fut> {
        use AddKeyResult::*;

        // This take is very imporant when combined with the mutex block
        // futher down. We need to make sure that it's not possible to
        // accidentally add too many keys to BatchState, which can result in
        // panics and widespread mutex poisonings.
        match self.state.take() {
            None => self.add_key_new_state(key, rules.load, rules.max_keys),
            Some(arc) => {
                let state_lock = arc.lock().unwrap();
                match state_lock.add_key(key, rules.max_keys) {
                    Added(token) => {
                        self.state = Some(arc.clone());
                        BatchFuture::new(token, arc)
                    }
                    AddedLast(token) => BatchFuture::new(token, arc),
                    Fail(key) => self.add_key_new_state(key, rules.load, rules.max_keys),
                }
            }
        }
    }
}

/// A batch future is a request for a single Key-Value lookup, which shares
/// its request with several other batch futures such that the request can
/// be Batched as [Keys] -> [Values]. It is created from a `Dispatcher`,
/// and when awaited, it will wait along with its other group of futures until
/// the window has passed, then execute the request and return the Value for
/// the specific key.
pub struct BatchFuture<
    'a,
    Key: Hash + Eq,
    Value: Clone,
    Error: Clone,
    Load: Fn(KeySet<Key>) -> Fut,
    Fut: Future<Output = Result<ValueSet<Value>, Error>>,
> {
    token: Token,
    state: SharedBatchState<'a, Key, Value, Error, Load, Fut>,
}

impl<
        'a,
        Key: Hash + Eq,
        Value: Clone,
        Error: Clone,
        Load: Fn(KeySet<Key>) -> Fut,
        Fut: Future<Output = Result<HashMap<Key, Value>, Error>>,
    > BatchFuture<'a, Key, Value, Error, Load, Fut>
{
    /// Note: make sure the BatchState invariants are upheld before calling
    /// this method. In paricular, each BatchFuture is guaranteed by the
    /// contract of this library to have an associated key in the BatchState.
    fn new(token: Token, state: Arc<Mutex<BatchState<'a, Key, Value, Error, Load, Fut>>>) {
        Self {
            token,
            state: Some(state),
        }
    }
}

impl<
        'a,
        Key: Hash + Eq,
        Value: Clone,
        Error: Clone,
        Load: Fn(KeySet<Key>) -> Fut,
        Fut: Future<Output = Result<HashMap<Key, Value>, Error>>,
    > Future for BatchFuture<'a, Key, Value, Error, Load, Fut>
{
    type Output = Result<Value, Error>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.state.poll_token(self.token) {
            Pending => Pending,
            Ready(result) => {
                self.state.reset();
                result
            }
        }
    }
}

/// A set of configuration rules for a batcher. This defines the batch loading
/// async fn, as well as the durating of time to wait for keys
pub struct BatchRules<
    Key: Hash,
    Value: Clone,
    Error: Clone,
    Load: Fn(HashMap<Key, u32>) -> Fut,
    Fut: Future<Output = Result<HashMap<Key, Value>, Error>>,
> {
    max_keys: u32,
    window: Duration,
    load: Load,
}

impl<
        Key: Hash,
        Value: Clone,
        Error: Clone,
        Load: Fn(HashMap<Key, u32>) -> Fut,
        Fut: Future<Output = Result<HashMap<Key, Value>, Error>>,
    > BatchRules<Key, Value, Error, Load, Fut>
{
    fn new(max_keys: usize, window: Duration, load: Load) -> Self {
        Self {
            max_keys,
            window,
            load,
        }
    }

    fn dispatcher<'a>(&'a self) -> Dispatcher<'a, Key, Value, Error, Load, Fut> {
        Dispatcher {
            rules: self,
            state: Mutex::new(SharedBatchState { state: None }),
        }
    }
}

/// A dispatcher is the entry point for creating BatchFutures. It maintains
/// a "currently accumulating" state, and each time you call Dispatcher::load,
/// the key is added to that state, until:
///
/// - the states accumulation timer runs out (this is usually very short)
/// - the state reaches its maxumum keys
///
/// At this point the state will be "launched"– that is, it will be detached
/// from this dispatcher and replaced with a fresh ones. The futures associated
/// with the old state share ownership of it and drive it to completion,
/// independent of the dispatcher.
pub struct Dispatcher<
    'a,
    Key: Hash + Eq,
    Value: Clone,
    Error: Clone,
    Load: Fn(HashMap<Key, u32>) -> Fut,
    Fut: Future<Output = Result<HashMap<Key, Value>, Error>>,
> {
    rules: &'a BatchRules<Key, Value, Error, Load, Fut>,

    // TODO: replace this with an atomic pointer. Also, probably make it weak?
    // If all the futures drop their state references, there's no reason for
    // dispatcher to keep it around.
    state: Mutex<SharedBatchState<'a, Key, Value, Error, Load, Fut>>,
}

impl<
        'a,
        Key: Hash + Eq,
        Value: Clone,
        Error: Clone,
        Load: Fn(HashMap<Key, u32>) -> Fut,
        Fut: Future<Output = Result<HashMap<Key, Value>, Error>>,
    > Dispatcher<'a, Key, Value, Error, Load, Fut>
{
    fn load(&self, key: Key) -> BatchFuture<'a, Key, Value, Error, Load, Fut> {
        let state_lock = self.state.lock().unwrap();

        state_lock.add_key(key, self.rules)
    }
}
