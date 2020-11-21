//! This library provides macros for (de)multiplexing Rusty streams.
//!
//! See [our GitHub repository](https://github.com/Hirrolot/mux-stream) for a high-level overview.

#![deny(unsafe_code)]

pub mod error_handler;

#[doc(hidden)]
pub use mux_stream_macros as macros;

/// Multiplexes several streams into one.
///
/// Accepts a list of variants in the form `MyEnumPath {VariantName0, ...,
/// VariantNameN}`. All enumeration variants shall be defined as variants taking
/// a single unnamed parameter.
///
/// Expands to:
///
/// ```ignore
/// |input_stream_0, ..., error_handler: Box<dyn Fn(tokio::sync::mpsc::error::SendError<_>)
///     -> futures::future::BoxFuture<'static, ()> + Send + Sync + 'static>| {
///     /* ... */
/// }
/// ```
///
/// This, the returned closure accepts many input streams of type
/// [`tokio::sync::mpsc::UnboundedReceiver`] parameterised by the corresponding
/// variant's type and an error handler. The count of input streams is the count
/// of variants specified. An error handler is invoked when the multiplexer
/// fails to propagate an update from one of input streams to an output stream.
/// See also [our default error handlers].
///
/// It propagates updates into the result stream in any order, simultaneously
/// from all the provided input streams (in a separate [Tokio task]). Updates
/// into the output stream are being redirected as long as at least one input
/// stream is active.
///
/// ```
/// use mux_stream::{error_handler, mux};
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
/// let result: UnboundedReceiver<MyEnum> = mux!(MyEnum { A, B, C })(
///     stream::iter(i32_values.clone()),
///     stream::iter(u8_values.clone()),
///     stream::iter(str_values.clone()),
///     error_handler::panicking(),
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
/// [Tokio task]: tokio::task
/// [our default error handlers]: crate::error_handler
#[macro_export]
macro_rules! mux {
    ($enum_ty:path { $($variant:ident),+ $(,)? }) => {
        mux_stream::macros::mux!($enum_ty { $($variant),+ })
    };
}

/// Demultiplexes a stream into several others.
///
/// Accepts a non-empty list of variants in the form `MyEnumPath {VariantName0,
/// ..., VariantNameN}`. `..` can be appended to input if you wish
/// non-exhaustive demultiplexing (e.g. just ignore unspecified variants).
///
/// Expands to:
///
/// ```ignore
/// |input_stream, error_handler: Box<dyn Fn(tokio::sync::mpsc::error::SendError<_>)
///     -> futures::future::BoxFuture<'static, ()> + Send + Sync + 'static>| {
///     { /* ... */ }
/// }
/// ```
///
/// This closure returns `(tokio::sync::mpsc::UnboundedReceiver<T[1]>, ...,
/// tokio::sync::mpsc::UnboundedReceiver<T[n]>)`, where `T[i]` is a type of
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
/// use mux_stream::{demux, error_handler};
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
///     demux!(MyEnum { A, B, C })(stream, error_handler::panicking());
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
/// [Tokio task]: tokio::task
/// [our default error handlers]: crate::error_handler
#[macro_export]
macro_rules! demux {
    ($enumeration:path { $($variant:ident),+ $(,)? } $($dot2:tt)?) => {
        mux_stream::macros::demux!($enumeration { $($variant),+ } $($dot2)?)
    };
}

/// Propagate streams into multiple asynchronous functions.
///
/// This is just a shortcut for passing the results (futures) of the specified
/// functions into [`tokio::join!`]. Returns a tuple of results of executing the
/// specified functions concurrently.
///
/// See [`examples/admin_panel.rs`] as an example.
/// TODO: provide a concise example.
///
/// [`examples/admin_panel.rs`]: https://github.com/Hirrolot/mux-stream/blob/master/examples/admin_panel.rs
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
