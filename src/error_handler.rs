//! Common error handlers for (de)multiplexing.

use futures::future::BoxFuture;
use tokio::sync::mpsc::error::SendError;

pub type ErrorHandler<T> =
    Box<dyn Fn(SendError<T>) -> BoxFuture<'static, ()> + Send + Sync + 'static>;

/// A panicking error handler.
pub fn panicking<T>() -> ErrorHandler<T>
where
    T: Send + 'static,
{
    Box::new(|error| Box::pin(async move { panic!(error) }))
}

/// An error handler that ignores an error.
pub fn ignoring<T>() -> ErrorHandler<T>
where
    T: Send + 'static,
{
    Box::new(|_error| Box::pin(async move {}))
}

/// A logging error handler.
#[cfg(feature = "logging")]
pub fn logging<T>() -> ErrorHandler<T>
where
    T: Send + 'static,
{
    Box::new(|error| {
        Box::pin(async move {
            log::error!("{}", error);
        })
    })
}
