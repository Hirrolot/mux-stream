//! This library provides macros for (de)multiplexing Rusty streams.
//!
//! See [our GitHub repository](https://github.com/Hirrolot/mux-stream) for a high-level overview.

#![deny(unsafe_code)]

use futures::{future::BoxFuture, FutureExt};
use tokio::sync::mpsc::error::SendError;

/// Multiplexes several streams into one.
///
/// Accepts a non-empty list of paths to variants of an enumeration, possibly
/// with a trailing comma. All enumeration variants shall be defined as variants
/// taking a single unnamed parameter.
///
/// Expands to:
///
/// ```ignore
/// |error_handler: Box<dyn Fn(tokio::sync::mpsc::error::SendError<_>) -> futures::future::BoxFuture<'static, ()> + Send + Sync + 'static>| {
///     |input_stream0: futures::stream::BoxStream<'static, _>, ...  | { /* ... */ }
/// }
/// ```
///
/// Thus, the returned closure is [curried]. After applying the first argument
/// to it, you obtain a closure of as many formal arguments as variants
/// specified, each parameterised by the corresponding type of variant's
/// parameter. After applying the second argument, you obtain
/// [`UnboundedReceiver`] of your enumeration type.
///
/// It propagates updates into the result stream in any order, simultaneously
/// from all the provided input streams (in a separate [Tokio task]). Updates
/// into the output stream are being redirected as long as at least one input
/// stream is active.
///
/// ```
/// use mux_stream::{mux, panicking};
///
/// use std::{collections::HashSet, iter::FromIterator};
///
/// use futures::{FutureExt, StreamExt};
/// use tokio::{stream, sync::mpsc::UnboundedReceiver};
///
/// #[derive(Debug)]
/// enum MyEnum {
///     A(i32),
///     B(u8),
///     C(&'static str),
/// }
///
/// # #[tokio::main]
/// # async fn main_() {
///
/// let i32_values = HashSet::from_iter(vec![123, 811]);
/// let u8_values = HashSet::from_iter(vec![88]);
/// let str_values = HashSet::from_iter(vec!["Hello", "ABC"]);
///
/// let result: UnboundedReceiver<MyEnum> = mux!(MyEnum { A, B, C })(panicking())(
///     stream::iter(i32_values.clone()).boxed(),
///     stream::iter(u8_values.clone()).boxed(),
///     stream::iter(str_values.clone()).boxed(),
/// );
///
/// let (i32_results, u8_results, str_results) = result
///     .fold(
///         (HashSet::new(), HashSet::new(), HashSet::new()),
///         |(mut i32_results, mut u8_results, mut str_results), update| async move {
///             match update {
///                 MyEnum::A(x) => i32_results.insert(x),
///                 MyEnum::B(x) => u8_results.insert(x),
///                 MyEnum::C(x) => str_results.insert(x),
///             };
///
///             (i32_results, u8_results, str_results)
///         },
///     )
///     .await;
///
/// assert_eq!(i32_results, i32_values);
/// assert_eq!(u8_results, u8_values);
/// assert_eq!(str_results, str_values);
/// # }
/// ```
///
/// Hash sets are used here owing to the obvious absence of order preservation
/// of updates from input streams.
///
/// [curried]: https://en.wikipedia.org/wiki/Currying
/// [`UnboundedReceiver`]: https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.UnboundedReceiver.html
/// [Tokio task]: https://docs.rs/tokio/latest/tokio/task/index.html
#[macro_export]
macro_rules! mux {
    // TODO: make the same syntax in mux-stream-macros.
    ($enum_ty:ty { $($variant:ident),+ $(,)? }) => {
        mux_stream_macros::mux!($(<$enum_ty>::$variant),+)
    };
}

/// Demultiplexes a stream into several others.
///
/// Accepts a non-empty list of paths to variants of an enumeration, possibly
/// with a trailing comma. All enumeration variants shall be defined as variants
/// taking a single unnamed parameter. `..` can be prepended to an input list if
/// you wish non-exhaustive demultiplexing (e.g. just ignore unspecified
/// variants).
///
/// Expands to:
///
/// ```ignore
/// |error_handler: Box<dyn Fn(tokio::sync::mpsc::error::SendError<_>) -> futures::future::BoxFuture<'static, ()> + Send + Sync + 'static>| {
///     |input_stream: futures::stream::BoxStream<'static, _>| { /* ... */ }
/// }
/// ```
///
/// Thus, the returned closure is [curried]. After applying two arguments to it
/// (`(...)(...)`), you obtain `(tokio::sync::mpsc::UnboundedReceiver<T[1]>,
/// ..., tokio::sync::mpsc::UnboundedReceiver<T[n]>)`, where `T[i]` is a type of
/// a single unnamed parameter of the corresponding provided variant.
///
/// `input_stream` is a stream of your enumeration to be demiltiplexed. Each
/// coming update from `input_stream` will be pushed into the corresponding
/// output stream immediately, in a separate [Tokio task].
///
/// `error_handler` is invoked when a demultiplexer fails to send an update
/// from `input_stream` into one of receivers. See also [our default error
/// handlers].
///
/// # Example
/// ```
/// use mux_stream::{demux, panicking};
///
/// use futures::{future::FutureExt, StreamExt};
/// use tokio::stream;
///
/// #[derive(Debug)]
/// enum MyEnum {
///     A(i32),
///     B(f64),
///     C(&'static str),
/// }
///
/// # #[tokio::main]
/// # async fn main_() {
/// let stream = stream::iter(vec![
///     MyEnum::A(123),
///     MyEnum::B(24.241),
///     MyEnum::C("Hello"),
///     MyEnum::C("ABC"),
///     MyEnum::A(811),
/// ]);
///
/// let (mut i32_stream, mut f64_stream, mut str_stream) =
///     demux!(MyEnum { A, B, C })(panicking())(stream.boxed());
///
/// assert_eq!(i32_stream.next().await, Some(123));
/// assert_eq!(i32_stream.next().await, Some(811));
/// assert_eq!(i32_stream.next().await, None);
///
/// assert_eq!(f64_stream.next().await, Some(24.241));
/// assert_eq!(f64_stream.next().await, None);
///
/// assert_eq!(str_stream.next().await, Some("Hello"));
/// assert_eq!(str_stream.next().await, Some("ABC"));
/// assert_eq!(str_stream.next().await, None);
/// # }
/// ```
///
/// [curried]: https://en.wikipedia.org/wiki/Currying
/// [Tokio task]: https://docs.rs/tokio/latest/tokio/task/index.html
/// [our default error handlers]: https://docs.rs/mux-stream
#[macro_export]
macro_rules! demux {
    // TODO: make the same syntax in mux-stream-macros.
    ($enum_ty:ty { $($variant:ident),+ $(,)? } $(dot2:tt)?) => {
        mux_stream_macros::demux!($dot2 $(<$enum_ty>::$variant),+)
    };
}

/// Propagate streams into multiple asynchronous functions.
///
/// This is just a shortcut for passing the results (futures) of the specified
/// functions into [`tokio::join!`]. Returns a tuple of results of executing the
/// specified functions concurrently.
///
/// See [`examples/hierarchical_update_dispatch.rs`] as an example.
/// TODO: provide a concise example.
///
/// [`tokio::join!`]: https://docs.rs/tokio/latest/tokio/macro.join.html
/// [`examples/hierarchical_update_dispatch.rs`]: https://github.com/Hirrolot/mux-stream/blob/master/examples/hierarchical_update_dispatch.rs
#[macro_export]
macro_rules! dispatch {
    ($updates:ident => $( $func:ident ),+ $(,)?) => {
        paste::paste! {
            {
                let ($( [<$func stream>] ),+) = $updates;
                tokio::join!( $( $func([<$func stream>]) ),+ )
            }
        }
    };
}

#[macro_use]
mod private_macros {
    macro_rules! ErrorHandlerRetTy {
        () => {
            Box<dyn Fn(SendError<T>) -> BoxFuture<'static, ()> + Send + Sync + 'static>
        };
    }
}

/// A panicking error handler.
pub fn panicking<T>() -> ErrorHandlerRetTy!()
where
    T: Send + 'static,
{
    Box::new(|error| async move { panic!(error) }.boxed())
}

/// An error handler that ignores an error.
pub fn ignoring<T>() -> ErrorHandlerRetTy!()
where
    T: Send + 'static,
{
    Box::new(|_error| async move {}.boxed())
}

/// A logging error handler.
#[cfg(feature = "logging")]
pub fn logging<T>() -> ErrorHandlerRetTy!()
where
    T: Send + 'static,
{
    Box::new(|error| {
        async move {
            log::error!("{}", error);
        }
        .boxed()
    })
}
