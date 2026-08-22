use super::LineIndex;

#[test]
fn counts_physical_lines_without_a_phantom_terminal_line() {
    assert_eq!(LineIndex::new("").physical_lines(), 0);
    assert_eq!(LineIndex::new("one").physical_lines(), 1);
    assert_eq!(LineIndex::new("one\n").physical_lines(), 1);
    assert_eq!(LineIndex::new("one\n\n").physical_lines(), 2);
    assert_eq!(LineIndex::new("one\r\ntwo\r\n").physical_lines(), 2);
}

#[test]
fn counts_every_line_touched_by_an_exclusive_span() {
    let text = "fn x() {\n\n    work();\n}\n";
    let index = LineIndex::new(text);
    assert_eq!(index.span_line_count(0, text.len() - 1), 4);
    assert_eq!(index.span_line_count(0, 0), 0);
}

#[test]
fn reports_unicode_columns_and_byte_offsets() {
    let text = "let café = 1;\n";
    let index = LineIndex::new(text);
    let start = text.find('é').expect("accented character");
    let span = index.span(text, start, start + 'é'.len_utf8());
    assert_eq!(span.byte_start, 7);
    assert_eq!(span.byte_end, 9);
    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 8);
    assert_eq!(span.end.column, 9);
}

#[test]
fn returns_lines_without_line_endings() {
    let text = "one\r\ntwo\nthree";
    let index = LineIndex::new(text);
    assert_eq!(index.line_text(text, 1), Some("one"));
    assert_eq!(index.line_text(text, 2), Some("two"));
    assert_eq!(index.line_text(text, 3), Some("three"));
    assert_eq!(index.line_text(text, 4), None);
}
