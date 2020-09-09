# mux-stream
[![Continious integration](https://github.com/Hirrolot/mux-stream/workflows/Rust/badge.svg)](https://github.com/Hirrolot/mux-stream/actions)
[![Crates.io](https://img.shields.io/crates/v/mux-stream.svg)](https://crates.io/crates/mux-stream)
[![Docs.rs](https://docs.rs/mux-stream/badge.svg)](https://docs.rs/mux-stream)

This crate empahises the [first-class] nature of [asynchronous streams] in Rust by deriving the _value construction_ & _pattern matching_ operations from [ADTs], depicted by the following correspondence:

| ADTs | Streams |
|----------|----------|
| [Value construction] | [Multiplexing] |
| [Pattern matching] | [Demultiplexing] |

[first-class]: https://en.wikipedia.org/wiki/First-class_citizen
[asynchronous streams]: https://docs.rs/futures/latest/futures/stream/index.html
[ADTs]: https://en.wikipedia.org/wiki/Algebraic_data_type

[Value construction]: https://en.wikipedia.org/wiki/Algebraic_data_type
[Multiplexing]: https://en.wikipedia.org/wiki/Multiplexing
[Pattern matching]: https://en.wikipedia.org/wiki/Pattern_matching
[Demultiplexing]: https://en.wikipedia.org/wiki/Multiplexer#Digital_demultiplexers

## Table of contents

 - [Motivation](#motivation)
 - [Demultiplexing](#demultiplexing)
 - [Multiplexing](#multiplexing)
 - [FAQ](#faq)

## Motivation

In many problem domains, we encounter the need to process incoming hierarchical structures. Suppose you're writing a social network, and the following kinds of updates might come at any moment:

<div align="center">
    <img src="https://raw.githubusercontent.com/Hirrolot/mux-stream/master/media/UPDATE_HIERARCHY.png" />
</div>

<br>
In terms of Rust, you might want to express such updates via [sum types]:

```rust
enum UserReq {
    SendMsg(SendMsgReq),
    Follow(FollowReq),
    MuteFriend(MuteFriendReq)
}

enum SendMsgReq {
    Photo(...),
    Video(...),
    Text(...)
}

struct FollowReq {
    ...
}

enum MuteFriendReq {
    Forever(...),
    ForInterval(...)
}
```

This is where the story begins: now you need to process user requests. Let's formulate some general requirements of requests-processing code:

 - **Conciseness.** Avoid boilerplate where possible. Life is too short to write boilerplate code.
 - [**Single-responsibility principle (SRP).**](https://en.wikipedia.org/wiki/Single-responsibility_principle) For our needs it means that each processor must be responsible for exactly one kind of request. No less and no more.
 - **Compatible with other Rusty code.** Our requests-processing solution must be able to be easily integrated into existing code bases.
 - **Stay Rusty.** [eDSLs] implemented via macros are fun, but be ready for confusing compilation errors when business logic is expressed in terms of such eDSLs. What is more, they are computer languages on their own -- it takes some time to become familiar with them.
 - **Type safety.** Do not spread the pain of upcasting/downcasting types that you're already aware of.

This crate addresses the problem of dispatching hierarchical updates, accommodating all of the requirements above. The approach is based upon the [functional reactive paradigm], a declarative paradigm concerned with asynchronous data streams. Augmenting asynchronous streams with [patten matching], your code would reflect the following structure (concerning with the example of a social network):

<div align="center">
    <img src="https://raw.githubusercontent.com/Hirrolot/mux-stream/master/media/STREAM_UPDATE_DISPATCH_STRUCTURE.png" />
</div>

[eDSLs]: https://en.wikipedia.org/wiki/Domain-specific_language
[sum types]: https://en.wikipedia.org/wiki/Tagged_union
[functional reactive paradigm]: https://en.wikipedia.org/wiki/Functional_reactive_programming

## Demultiplexing

Given `Stream<T1 | ... | Tn>`, demultiplexing produces `Stream<T1>, ..., Stream<Tn>`. See the illustration below, in which every circle is an item of a stream and has a type (its colour):

<div align="center">
    <img src="https://raw.githubusercontent.com/Hirrolot/mux-stream/master/media/DEMUX.png" />
</div>

That is, once an update from an input stream is available, it's pushed into the corresponding output stream in a separate [Tokio task]. No output stream can slow down another one.

[Tokio task]: https://docs.rs/tokio/0.2.22/tokio/task/index.html

### Example

[[`examples/demux.rs`](https://github.com/Hirrolot/mux-stream/blob/master/examples/demux.rs)]
```rust
use mux_stream::{demux, panicking};

use futures::{future::FutureExt, StreamExt};
use tokio::stream;

#[derive(Debug)]
enum MyEnum {
    A(i32),
    B(f64),
    C(&'static str),
}

let stream = stream::iter(vec![
    MyEnum::A(123),
    MyEnum::B(24.241),
    MyEnum::C("Hello"),
    MyEnum::C("ABC"),
    MyEnum::A(811),
]);

let (mut i32_stream, mut f64_stream, mut str_stream) =
    demux!(MyEnum::A, MyEnum::B, MyEnum::C)(panicking())(stream.boxed());

assert_eq!(i32_stream.next().await, Some(123));
assert_eq!(i32_stream.next().await, Some(811));
assert_eq!(i32_stream.next().await, None);

assert_eq!(f64_stream.next().await, Some(24.241));
assert_eq!(f64_stream.next().await, None);

assert_eq!(str_stream.next().await, Some("Hello"));
assert_eq!(str_stream.next().await, Some("ABC"));
assert_eq!(str_stream.next().await, None);
```

## Multiplexing

Multiplexing is the opposite of demultiplexing: given `Stream<T1>, ..., Stream<Tn>`, it produces `Stream<T1 | ... | Tn>`. Again, the process is illustrated below:

<div align="center">
    <img src="https://raw.githubusercontent.com/Hirrolot/mux-stream/master/media/MUX.png" />
</div>

That is, once an update from any input streams is available, it's pushed into the output stream. Again, this work is performed asynchronously in a separate [Tokio task].

### Example

[[`examples/mux.rs`](https://github.com/Hirrolot/mux-stream/blob/master/examples/mux.rs)]
```rust
use mux_stream::{mux, panicking};

use std::{collections::HashSet, iter::FromIterator};

use futures::{FutureExt, StreamExt};
use tokio::{stream, sync::mpsc::UnboundedReceiver};

#[derive(Debug)]
enum MyEnum {
    A(i32),
    B(u8),
    C(&'static str),
}


let i32_values = HashSet::from_iter(vec![123, 811]);
let u8_values = HashSet::from_iter(vec![88]);
let str_values = HashSet::from_iter(vec!["Hello", "ABC"]);

let result: UnboundedReceiver<MyEnum> = mux!(MyEnum::A, MyEnum::B, MyEnum::C)(panicking())(
    stream::iter(i32_values.clone()).boxed(),
    stream::iter(u8_values.clone()).boxed(),
    stream::iter(str_values.clone()).boxed(),
);

let (i32_results, u8_results, str_results) = result
    .fold(
        (HashSet::new(), HashSet::new(), HashSet::new()),
        |(mut i32_results, mut u8_results, mut str_results), update| async move {
            match update {
                MyEnum::A(x) => i32_results.insert(x),
                MyEnum::B(x) => u8_results.insert(x),
                MyEnum::C(x) => str_results.insert(x),
            };

            (i32_results, u8_results, str_results)
        },
    )
    .await;

assert_eq!(i32_results, i32_values);
assert_eq!(u8_results, u8_values);
assert_eq!(str_results, str_values);
```

Hash sets are used here owing to the obvious absence of order preservation of updates from input streams.

## FAQ

Q: Is only Tokio supported now?

A: Yes. I have no plans yet to support other asynchronous runtimes.
