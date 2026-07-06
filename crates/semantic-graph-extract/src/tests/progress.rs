use crate::progress::render_progress_line;

#[test]
fn progress_line_renders_partial_progress() {
    assert_eq!(
        render_progress_line(12, 24, "rust symbols"),
        "[##########----------] 12/24 50% rust symbols"
    );
}

#[test]
fn progress_line_renders_complete_progress() {
    assert_eq!(
        render_progress_line(24, 24, "csharp calls"),
        "[####################] 24/24 100% csharp calls"
    );
}

#[test]
fn progress_line_renders_zero_total_as_complete() {
    assert_eq!(
        render_progress_line(0, 0, "fts files"),
        "[####################] 0/0 100% fts files"
    );
}

#[test]
fn progress_line_is_ascii_only() {
    let line = render_progress_line(1, 2, "soul λ symbols");

    assert!(line.is_ascii());
    assert_eq!(line, "[##########----------] 1/2 50% soul ? symbols");
}
