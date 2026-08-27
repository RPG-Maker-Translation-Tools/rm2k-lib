//! Tests for `rm2k::sink` - the `Sink` trait and its `SliceSink`/`CountSink` implementations.

use rm2k::{
    WriteError,
    sink::{CountSink, Sink, SliceSink},
};

#[test]
fn slice_sink_fills_then_refuses() {
    let mut buf = [0u8; 4];
    let mut sink = SliceSink::new(&mut buf);
    assert!(sink.write(&[1, 2]).is_ok());
    assert!(sink.write(&[3, 4]).is_ok());
    assert_eq!(sink.write(&[5]), Err(WriteError::BufferFull));
    assert_eq!(sink.as_slice(), &[1, 2, 3, 4]);
}

#[test]
fn count_sink_agrees_with_slice_sink() {
    let mut counter = CountSink::new();
    let mut buf = [0u8; 16];
    let mut sink = SliceSink::new(&mut buf);
    for chunk in [&b"abc"[..], b"de", b"", b"fghij"] {
        counter.write(chunk).unwrap();
        sink.write(chunk).unwrap();
    }
    assert_eq!(counter.len(), sink.len() as u64);
}
