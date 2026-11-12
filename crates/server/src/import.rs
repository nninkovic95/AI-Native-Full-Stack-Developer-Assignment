//! Turns an uploaded file into a new document: a title and a Quill Delta.

use pulldown_cmark::{Event, Parser, TagEnd};
use serde_json::{json, Map, Value};

#[derive(Debug, PartialEq)]
pub struct Imported {
    pub title: String,
    pub content: Value,
}

#[derive(Debug, PartialEq)]
pub enum ImportError {}

pub fn convert(file_name: &str, bytes: &[u8]) -> Result<Imported, ImportError> {
    let (stem, ext) = file_name.rsplit_once('.').unwrap_or((file_name, ""));
    let text = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
    let content = match ext {
        "md" => markdown_to_delta(&text),
        _ => plain_text_to_delta(&text),
    };
    Ok(Imported {
        title: stem.to_owned(),
        content,
    })
}

fn plain_text_to_delta(text: &str) -> Value {
    let mut delta = Delta::default();
    delta.push(&format!("{}\n", text.trim_end_matches('\n')), Map::new());
    delta.into_value()
}

fn markdown_to_delta(text: &str) -> Value {
    let mut delta = Delta::default();
    for event in Parser::new(text) {
        match event {
            Event::Text(t) => delta.push(&t, Map::new()),
            Event::End(TagEnd::Paragraph) => delta.push("\n", Map::new()),
            Event::End(TagEnd::Heading(level)) => {
                delta.push("\n", attrs([("header", json!(level as u8))]))
            }
            _ => {}
        }
    }
    delta.into_value()
}

fn attrs<const N: usize>(pairs: [(&str, Value); N]) -> Map<String, Value> {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// Builds a Delta in Quill's canonical form: neighbouring inserts with the same
/// attributes are merged, as Quill itself does.
#[derive(Default)]
struct Delta {
    ops: Vec<(String, Map<String, Value>)>,
}

impl Delta {
    fn push(&mut self, text: &str, attributes: Map<String, Value>) {
        match self.ops.last_mut() {
            Some((last, last_attrs)) if *last_attrs == attributes => last.push_str(text),
            _ => self.ops.push((text.to_owned(), attributes)),
        }
    }

    fn into_value(self) -> Value {
        let ops: Vec<Value> = self
            .ops
            .into_iter()
            .map(|(insert, attributes)| {
                if attributes.is_empty() {
                    json!({ "insert": insert })
                } else {
                    json!({ "insert": insert, "attributes": attributes })
                }
            })
            .collect();
        json!({ "ops": ops })
    }
}
