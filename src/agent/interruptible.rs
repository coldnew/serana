//! Interruptible API calls with cancellation support.
//!
//! Wraps HTTP requests to allow cancellation mid-flight.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::oneshot;

/// Cancel token for interrupting operations.
#[derive(Debug, Clone)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    /// Create a new cancel token.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Cancel the operation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

/// A future that can be cancelled.
pin_project_lite::pin_project! {
    pub struct InterruptibleFuture<F> {
        #[pin]
        inner: F,
        cancel_token: CancelToken,
        cancel_receiver: oneshot::Receiver<()>,
    }
}

impl<F, T, E> InterruptibleFuture<F>
where
    F: Future<Output = Result<T, E>> + Send,
{
    pub fn new(inner: F, cancel_token: CancelToken) -> Self {
        let (tx, cancel_receiver) = oneshot::channel();
        let cancel_token_clone = cancel_token.clone();

        // Spawn a task to watch for cancellation
        tokio::spawn(async move {
            while !cancel_token_clone.is_cancelled() {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
            let _ = tx.send(());
        });

        Self {
            inner,
            cancel_token,
            cancel_receiver,
        }
    }
}

impl<F, T, E> Future for InterruptibleFuture<F>
where
    F: Future<Output = Result<T, E>> + Send,
{
    type Output = Option<Result<T, E>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // Check if cancellation was requested
        if this.cancel_token.is_cancelled() {
            return Poll::Ready(None);
        }

        // Poll the cancellation receiver
        let mut recv = std::pin::pin!(this.cancel_receiver);
        if let Poll::Ready(_) = recv.as_mut().poll(cx) {
            return Poll::Ready(None);
        }

        // Poll the inner future
        match this.inner.poll(cx) {
            Poll::Ready(result) => Poll::Ready(Some(result)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Wrapper for making interruptible API calls.
pub struct InterruptibleApiCall;

impl InterruptibleApiCall {
    /// Execute an API call that can be cancelled.
    pub async fn execute<F, T, E>(future: F, cancel_token: CancelToken) -> Option<Result<T, E>>
    where
        F: Future<Output = Result<T, E>> + Send,
    {
        InterruptibleFuture::new(future, cancel_token).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cancellation() {
        let token = CancelToken::new();
        let future = async {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            Ok::<_, ()>(42)
        };

        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            token_clone.cancel();
        });

        let result = InterruptibleApiCall::execute(future, token).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_no_cancellation() {
        let token = CancelToken::new();
        let future = async { Ok::<_, ()>(42) };

        let result = InterruptibleApiCall::execute(future, token).await;
        assert_eq!(result, Some(Ok(42)));
    }
}
