//! Tests for `rm2k::diag` - the diagnostic kinds, their text, and the sinks that collect them.

use rm2k::diag::{DiagKind, DiagSink, Diagnostic, NullSink, SliceSink, StrictSink};

const ALL_KINDS: [DiagKind; 9] = [
    DiagKind::UnknownChunk,
    DiagKind::ShortChunk,
    DiagKind::TrailingBytes,
    DiagKind::BadVarint,
    DiagKind::ChunkOverrun,
    DiagKind::BadMagic,
    DiagKind::EventListCorrupt,
    DiagKind::RaggedArray,
    DiagKind::EntryBudgetExhausted,
];

fn diag(offset: u32, kind: DiagKind) -> Diagnostic {
    Diagnostic {
        offset,
        chunk_id: 0x2a,
        struct_name: "Enemy",
        kind,
    }
}

#[test]
fn every_kind_has_distinct_text() {
    let texts: Vec<&str> = ALL_KINDS.iter().map(|kind| kind.as_str()).collect();
    for (i, text) in texts.iter().enumerate() {
        assert!(!text.is_empty());
        assert!(!texts[..i].contains(text), "duplicate text: {text}");
    }
}

#[test]
fn diagnostic_display_names_the_struct_chunk_and_offset() {
    assert_eq!(
        diag(5, DiagKind::ShortChunk).to_string(),
        "Enemy: chunk too short for field (chunk 0x2a, offset 5)"
    );
}

#[test]
fn slice_sink_counts_what_does_not_fit() {
    let mut slots = [Diagnostic::EMPTY; 2];
    let mut sink = SliceSink::new(&mut slots);
    for offset in 0..3 {
        sink.push(diag(offset, DiagKind::BadVarint));
    }
    assert_eq!(sink.as_slice().len(), 2);
    assert_eq!(sink.dropped(), 1);
}

#[test]
fn null_sink_never_aborts_and_latches_nothing() {
    let mut sink = NullSink;
    sink.push(diag(0, DiagKind::BadMagic));
    assert!(!sink.should_abort());
    assert_eq!(sink.first(), None);
}

#[test]
fn strict_sink_keeps_only_the_first_problem() {
    let mut sink = StrictSink::new();
    assert!(!sink.should_abort());
    sink.push(diag(1, DiagKind::ShortChunk));
    sink.push(diag(9, DiagKind::BadVarint));
    assert!(sink.should_abort());
    assert_eq!(sink.first(), Some(diag(1, DiagKind::ShortChunk)));
}

#[cfg(feature = "alloc")]
#[test]
fn a_vec_sink_records_everything_in_order() {
    let mut sink: Vec<Diagnostic> = Vec::new();
    DiagSink::push(&mut sink, diag(1, DiagKind::ShortChunk));
    DiagSink::push(&mut sink, diag(2, DiagKind::RaggedArray));
    assert_eq!(sink.iter().map(|d| d.offset).collect::<Vec<_>>(), [1, 2]);
}
