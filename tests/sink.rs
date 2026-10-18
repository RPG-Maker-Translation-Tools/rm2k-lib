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

#[test]
fn slice_sink_exposes_its_written_prefix_by_value_and_by_reference() {
    let mut buf = [0u8; 4];
    let mut sink = SliceSink::new(&mut buf);
    assert!(sink.is_empty());
    sink.write(&[9, 8]).unwrap();
    assert!(!sink.is_empty());
    assert_eq!(sink.into_slice(), &[9, 8]);
}

#[test]
fn count_sink_reports_empty_until_something_is_written() {
    let mut counter = CountSink::new();
    assert!(counter.is_empty());
    counter.write(&[1]).unwrap();
    assert!(!counter.is_empty());
}

#[cfg(feature = "std")]
#[test]
fn io_sink_forwards_every_byte_to_the_writer() {
    let mut sink = rm2k::sink::IoSink(Vec::new());
    sink.write(b"ab").unwrap();
    sink.write(b"c").unwrap();
    assert_eq!(sink.0, b"abc");
}
