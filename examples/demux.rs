use mux_stream::{demux, error_handler};

use futures::StreamExt;
use tokio::stream;

#[tokio::main]
async fn main() {
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
        demux!(MyEnum { A, B, C })(stream, error_handler::panicking());

    assert_eq!(i32_stream.next().await, Some(123));
    assert_eq!(i32_stream.next().await, Some(811));
    assert_eq!(i32_stream.next().await, None);

    assert_eq!(f64_stream.next().await, Some(24.241));
    assert_eq!(f64_stream.next().await, None);

    assert_eq!(str_stream.next().await, Some("Hello"));
    assert_eq!(str_stream.next().await, Some("ABC"));
    assert_eq!(str_stream.next().await, None);
}
