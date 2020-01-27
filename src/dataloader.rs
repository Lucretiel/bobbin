//! A homegrown dataloader. This was created because the one in crates.io
//! has a proliferation of boxes that makes in unsuitable for references and
//! so on. No caching for now.

use std::fmt::Display;
use std::future::Future;

/// The state for a single batched request. Shared among several futures.
enum BatchState<BatchFn, BatchFut> {
	Accruing(BatchFn),
	Pending(BatchFut),
	Finished,
}

#[derive(Debug)]
struct BatchFuture<Key, Value, Batch> {
	state: Arc<BatchState<Load>>,
}

impl Future for

#[derive(Debug, Clone, Default)]
pub struct Loader<Load> {
    batch_load: Load,
}

impl<Key, Value, Error, Load, Finisher, Fut> Loader<Load>
where
    Error: Display,
    Load: Fn(&[Key], Finisher) -> Fut,
    Finisher: Fn(&Key, Value),
    Fut: Future<Output = Result<(), Error>>,
{
}
