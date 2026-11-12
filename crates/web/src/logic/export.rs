//! Turns a Quill Delta into Markdown, the inverse of the server's Markdown import.

use serde_json::Value;

/// One document line: its text and the block format carried by the newline that ended it.
struct Line {
    runs: Vec<Run>,
    block: Block,
}

/// A stretch of text with one inline format. Underline has no Markdown form, so it is not kept.
#[derive(PartialEq)]
struct Run {
    text: String,
    bold: bool,
    italic: bool,
}

impl Line {
    fn is_blank(&self) -> bool {
        self.runs.iter().all(|r| r.text.trim().is_empty())
    }
}

fn flag(attributes: Option<&Value>, name: &str) -> bool {
    attributes.and_then(|a| a.get(name)).and_then(Value::as_bool).unwrap_or(false)
}

/// Appends text to the line being built, joining it to the previous run when the format is the same.
fn push_run(runs: &mut Vec<Run>, text: &str, bold: bool, italic: bool) {
    if text.is_empty() {
        return;
    }
    match runs.last_mut() {
        Some(last) if last.bold == bold && last.italic == italic => last.text.push_str(text),
        _ => runs.push(Run { text: text.to_string(), bold, italic }),
    }
}

fn inline(runs: &[Run]) -> String {
    runs.iter().map(run_markdown).collect()
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Escapes a first character that would turn plain text into a heading, quote or list item.
fn escape_line_start(text: String) -> String {
    if text.starts_with(['#', '>', '-', '+']) {
        format!("\\{text}")
    } else {
        text
    }
}

fn run_markdown(run: &Run) -> String {
    let text = escape(&run.text);
    let marker = match (run.bold, run.italic) {
        (false, false) => return text,
        (true, false) => "**",
        (false, true) => "*",
        (true, true) => "***",
    };
    let inner = text.trim();
    if inner.is_empty() {
        return text;
    }
    let start = text.len() - text.trim_start().len();
    let end = text.trim_end().len();
    format!("{}{marker}{inner}{marker}{}", &text[..start], &text[end..])
}

enum Block {
    Paragraph,
    Heading(usize),
    /// Carries the nesting level.
    Bullet(usize),
    Ordered(usize),
}

/// The editor offers a handful of levels; a larger value in a hand-made delta must not blow up the output.
const MAX_INDENT: u64 = 8;

fn block_of(attributes: Option<&Value>) -> Block {
    let header = attributes.and_then(|a| a.get("header")).and_then(Value::as_u64);
    let list = attributes.and_then(|a| a.get("list")).and_then(Value::as_str);
    let indent = attributes.and_then(|a| a.get("indent")).and_then(Value::as_u64).unwrap_or(0).min(MAX_INDENT) as usize;
    match (header, list) {
        (Some(n @ 1..=6), _) => Block::Heading(n as usize),
        (_, Some("bullet")) => Block::Bullet(indent),
        (_, Some("ordered")) => Block::Ordered(indent),
        _ => Block::Paragraph,
    }
}

fn lines(delta: &Value) -> Vec<Line> {
    let mut out = Vec::new();
    let mut current: Vec<Run> = Vec::new();
    let ops = delta.get("ops").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    for op in ops {
        let Some(text) = op.get("insert").and_then(Value::as_str) else { continue };
        let attributes = op.get("attributes");
        let (bold, italic) = (flag(attributes, "bold"), flag(attributes, "italic"));
        let mut parts = text.split('\n');
        push_run(&mut current, parts.next().unwrap_or(""), bold, italic);
        for part in parts {
            out.push(Line { runs: std::mem::take(&mut current), block: block_of(attributes) });
            push_run(&mut current, part, bold, italic);
        }
    }
    if !current.is_empty() {
        out.push(Line { runs: current, block: Block::Paragraph });
    }
    out
}

/// `counters[n]` is the last number used by an ordered item at indent `n` in the current list.
fn render(line: &Line, counters: &mut Vec<usize>) -> String {
    let text = inline(&line.runs);
    match line.block {
        Block::Paragraph => {
            counters.clear();
            escape_line_start(text)
        }
        Block::Heading(n) => {
            counters.clear();
            format!("{} {}", "#".repeat(n), text)
        }
        Block::Bullet(indent) => {
            // Items below a bullet start over if their list resumes later.
            counters.truncate(indent);
            format!("{}- {}", "    ".repeat(indent), escape_line_start(text))
        }
        Block::Ordered(indent) => {
            counters.truncate(indent + 1);
            counters.resize(indent + 1, 0);
            counters[indent] += 1;
            format!("{}{}. {}", "    ".repeat(indent), counters[indent], escape_line_start(text))
        }
    }
}

pub fn delta_to_markdown(delta: &Value) -> String {
    let mut out = String::new();
    let mut counters = Vec::new();
    let mut previous_was_list = false;
    for line in lines(delta).iter().filter(|l| !l.is_blank()) {
        let is_list = matches!(line.block, Block::Bullet(_) | Block::Ordered(_));
        if !out.is_empty() {
            out.push_str(if is_list && previous_was_list { "\n" } else { "\n\n" });
        }
        out.push_str(&render(line, &mut counters));
        previous_was_list = is_list;
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// A title as a file name that every common file system accepts.
pub fn export_file_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '-' } else { c })
        .collect();
    let cut: String = cleaned.trim().chars().take(100).collect();
    let stem = cut.trim();
    format!("{}.md", if stem.is_empty() { "document" } else { stem })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plain_paragraphs_are_separated_by_one_blank_line() {
        let delta = json!({"ops": [{"insert": "one\ntwo\n"}]});
        assert_eq!(delta_to_markdown(&delta), "one\n\ntwo\n");
    }

    #[test]
    fn headings_get_one_hash_per_level() {
        let delta = json!({"ops": [
            {"insert": "Title"}, {"insert": "\n", "attributes": {"header": 1}},
            {"insert": "Sub"}, {"insert": "\n", "attributes": {"header": 3}},
            {"insert": "body\n"},
        ]});
        assert_eq!(delta_to_markdown(&delta), "# Title\n\n### Sub\n\nbody\n");
    }

    #[test]
    fn bold_italic_and_both_use_markers_and_underline_is_dropped() {
        let delta = json!({"ops": [
            {"insert": "a ", "attributes": {"bold": true}},
            {"insert": "b", "attributes": {"italic": true}},
            {"insert": " c"},
            {"insert": "d", "attributes": {"bold": true, "italic": true}},
            {"insert": " e", "attributes": {"underline": true}},
            {"insert": "\n"},
        ]});
        assert_eq!(delta_to_markdown(&delta), "**a** *b* c***d*** e\n");
    }

    #[test]
    fn edge_spaces_of_a_formatted_run_stay_outside_the_markers() {
        let delta = json!({"ops": [
            {"insert": " x ", "attributes": {"bold": true}},
            {"insert": "y"},
            {"insert": "  ", "attributes": {"italic": true}},
            {"insert": "z\n"},
        ]});
        assert_eq!(delta_to_markdown(&delta), " **x** y  z\n");
    }

    #[test]
    fn runs_that_differ_only_by_underline_share_one_pair_of_markers() {
        let delta = json!({"ops": [
            {"insert": "a", "attributes": {"bold": true, "underline": true}},
            {"insert": "b", "attributes": {"bold": true}},
            {"insert": "\n"},
        ]});
        assert_eq!(delta_to_markdown(&delta), "**ab**\n");
    }

    #[test]
    fn bullet_items_have_no_blank_line_between_them_but_one_around_the_list() {
        let delta = json!({"ops": [
            {"insert": "intro\nx"}, {"insert": "\n", "attributes": {"list": "bullet"}},
            {"insert": "y"}, {"insert": "\n", "attributes": {"list": "bullet"}},
            {"insert": "outro\n"},
        ]});
        assert_eq!(delta_to_markdown(&delta), "intro\n\n- x\n- y\n\noutro\n");
    }

    #[test]
    fn ordered_items_are_numbered_and_restart_after_another_kind_of_line() {
        let delta = json!({"ops": [
            {"insert": "a"}, {"insert": "\n", "attributes": {"list": "ordered"}},
            {"insert": "b"}, {"insert": "\n", "attributes": {"list": "ordered"}},
            {"insert": "mid"}, {"insert": "\n", "attributes": {"header": 2}},
            {"insert": "c"}, {"insert": "\n", "attributes": {"list": "ordered"}},
            {"insert": "d"}, {"insert": "\n", "attributes": {"list": "bullet"}},
            {"insert": "e"}, {"insert": "\n", "attributes": {"list": "ordered"}},
        ]});
        assert_eq!(delta_to_markdown(&delta), "1. a\n2. b\n\n## mid\n\n1. c\n- d\n1. e\n");
    }

    #[test]
    fn nested_items_indent_four_spaces_per_level() {
        let delta = json!({"ops": [
            {"insert": "a"}, {"insert": "\n", "attributes": {"list": "bullet"}},
            {"insert": "b"}, {"insert": "\n", "attributes": {"list": "bullet", "indent": 1}},
            {"insert": "c"}, {"insert": "\n", "attributes": {"list": "ordered", "indent": 2}},
            {"insert": "d"}, {"insert": "\n", "attributes": {"list": "ordered", "indent": 2}},
            {"insert": "e"}, {"insert": "\n", "attributes": {"list": "bullet"}},
        ]});
        assert_eq!(delta_to_markdown(&delta), "- a\n    - b\n        1. c\n        2. d\n- e\n");
    }

    #[test]
    fn ordered_numbering_is_kept_per_indent_level() {
        let delta = json!({"ops": [
            {"insert": "a"}, {"insert": "\n", "attributes": {"list": "ordered"}},
            {"insert": "b"}, {"insert": "\n", "attributes": {"list": "ordered", "indent": 1}},
            {"insert": "c"}, {"insert": "\n", "attributes": {"list": "ordered", "indent": 1}},
            {"insert": "d"}, {"insert": "\n", "attributes": {"list": "ordered"}},
            {"insert": "e"}, {"insert": "\n", "attributes": {"list": "ordered", "indent": 1}},
        ]});
        assert_eq!(delta_to_markdown(&delta), "1. a\n    1. b\n    2. c\n2. d\n    1. e\n");
    }

    #[test]
    fn blank_document_lines_add_no_extra_blank_lines() {
        let delta = json!({"ops": [
            {"insert": "one\n\n\n\ntwo\n\n"},
            {"insert": "x"}, {"insert": "\n", "attributes": {"list": "bullet"}},
            {"insert": "\n"},
            {"insert": "y"}, {"insert": "\n", "attributes": {"list": "bullet"}},
        ]});
        assert_eq!(delta_to_markdown(&delta), "one\n\ntwo\n\n- x\n- y\n");
    }

    #[test]
    fn markdown_characters_in_text_are_escaped() {
        let delta = json!({"ops": [
            {"insert": "a*b_c`d[e]f\\g", "attributes": {"bold": true}},
            {"insert": " 2*3\n"},
        ]});
        assert_eq!(delta_to_markdown(&delta), "**a\\*b\\_c\\`d\\[e\\]f\\\\g** 2\\*3\n");
    }

    #[test]
    fn a_paragraph_that_would_read_as_a_block_is_escaped_at_its_start() {
        let delta = json!({"ops": [
            {"insert": "# not a heading\n> not a quote\n- not a list\n+ nor this\nmid # dash - plus +\n"},
            {"insert": "- item"}, {"insert": "\n", "attributes": {"list": "bullet"}},
        ]});
        assert_eq!(
            delta_to_markdown(&delta),
            "\\# not a heading\n\n\\> not a quote\n\n\\- not a list\n\n\\+ nor this\n\nmid # dash - plus +\n\n- \\- item\n"
        );
    }

    #[test]
    fn unexpected_ops_are_skipped_and_unknown_attributes_ignored() {
        let delta = json!({"ops": [
            {"insert": {"image": "x.png"}},
            {"insert": "hi", "attributes": {"color": "red", "bold": "yes", "header": "x", "indent": -1}},
            {"insert": 5},
            {"retain": 3},
            {"insert": "\n", "attributes": {"list": "checked", "header": 9}},
        ]});
        assert_eq!(delta_to_markdown(&delta), "hi\n");
        assert_eq!(delta_to_markdown(&json!("nope")), "");
        assert_eq!(delta_to_markdown(&json!({"ops": "nope"})), "");
    }

    #[test]
    fn file_name_replaces_forbidden_characters_and_adds_the_extension() {
        assert_eq!(export_file_name("Plan"), "Plan.md");
        assert_eq!(export_file_name("  a/b\\c:d*e?f\"g<h>i|j  "), "a-b-c-d-e-f-g-h-i-j.md");
    }

    #[test]
    fn file_name_is_cut_to_100_characters_and_never_empty() {
        let long = "é".repeat(150);
        assert_eq!(export_file_name(&long), format!("{}.md", "é".repeat(100)));
        assert_eq!(export_file_name(""), "document.md");
        assert_eq!(export_file_name("   "), "document.md");
    }

    #[test]
    fn a_small_mixed_document_converts_as_a_whole() {
        let delta = json!({"ops": [
            {"insert": "Notes"}, {"insert": "\n", "attributes": {"header": 1}},
            {"insert": "Some "}, {"insert": "bold", "attributes": {"bold": true}},
            {"insert": " and "}, {"insert": "italic", "attributes": {"italic": true}},
            {"insert": " text.\n"},
            {"insert": "fruit"}, {"insert": "\n", "attributes": {"list": "bullet"}},
            {"insert": "apple"}, {"insert": "\n", "attributes": {"list": "bullet", "indent": 1}},
            {"insert": "first"}, {"insert": "\n", "attributes": {"list": "ordered"}},
            {"insert": "second"}, {"insert": "\n", "attributes": {"list": "ordered"}},
        ]});
        assert_eq!(
            delta_to_markdown(&delta),
            "# Notes\n\nSome **bold** and *italic* text.\n\n- fruit\n    - apple\n1. first\n2. second\n"
        );
    }

    #[test]
    fn empty_documents_give_an_empty_string() {
        assert_eq!(delta_to_markdown(&json!({"ops": []})), "");
        assert_eq!(delta_to_markdown(&json!({})), "");
        assert_eq!(delta_to_markdown(&json!({"ops": [{"insert": "\n"}]})), "");
    }
}
