use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use super::{Violation, check_store};
use crate::memory::MemoryStore;

/// The output of `check`, which a local store finishes on its first poll.
fn now(check: impl Future<Output = Result<(), Violation>>) -> Result<(), Violation> {
    match pin!(check).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(checked) => checked,
        Poll::Pending => Err(Violation {
            check: "a local store's calls are done when made",
            detail: "the check did not finish".to_owned(),
        }),
    }
}

#[test]
fn memory_store_keeps_the_contract() -> Result<(), Violation> {
    let store = MemoryStore::default();
    now(check_store(&store, &store.clone()))
}
