//! The import converter seam: (file name, bytes) -> title + Quill Delta, or a rejection.
//! Expected Deltas are written out by hand in Quill's canonical form.

use serde_json::json;
use server::import::convert;

#[test]
fn a_text_file_becomes_a_document_titled_after_the_file() {
    let doc = convert("Meeting notes.txt", b"First line\r\nSecond line").unwrap();

    assert_eq!(doc.title, "Meeting notes");
    assert_eq!(doc.content, json!({ "ops": [{ "insert": "First line\nSecond line\n" }] }));
}

#[test]
fn markdown_headings_become_header_lines() {
    let doc = convert("plan.md", b"# Q3 plan\n\nIntro paragraph.\n\n## Goals\n").unwrap();

    assert_eq!(doc.title, "plan");
    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Q3 plan" },
            { "insert": "\n", "attributes": { "header": 1 } },
            { "insert": "Intro paragraph.\nGoals" },
            { "insert": "\n", "attributes": { "header": 2 } }
        ]})
    );
}
