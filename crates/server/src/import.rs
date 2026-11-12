//! Turns an uploaded file into a new document: a title and a Quill Delta.

use axum::{
    extract::{
        multipart::{MultipartError, MultipartRejection},
        Multipart, State,
    },
    http::StatusCode,
    Json,
};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde_json::{json, Map, Value};
use shared::{Document, DEFAULT_TITLE, IMPORT_EXTENSIONS, MAX_IMPORT_BYTES, MAX_TITLE_LEN};

use crate::{auth::CurrentUser, docs, error::AppResult, AppError, AppState};

/// `POST /api/documents/import`: the multipart field `file` becomes a new document.
pub async fn upload(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    multipart: Result<Multipart, MultipartRejection>,
) -> AppResult<(StatusCode, Json<Document>)> {
    // Taken as a Result so a body that is not a form upload gets the JSON error shape too.
    let mut multipart = multipart.map_err(|_| AppError::BadRequest("No file was uploaded".into()))?;
    while let Some(field) = multipart.next_field().await.map_err(unreadable)? {
        if field.name() != Some("file") {
            continue;
        }
        let file_name = field.file_name().unwrap_or_default().to_owned();
        let bytes = field.bytes().await.map_err(unreadable)?;
        let imported = convert(&file_name, &bytes)?;
        let doc = docs::insert(&state.pool, &user, &imported.title, imported.content).await?;
        return Ok((StatusCode::CREATED, Json(doc)));
    }
    Err(AppError::BadRequest("No file was uploaded".into()))
}

/// A body cut off by the route's size limit surfaces here, while the field is being read.
fn unreadable(e: MultipartError) -> AppError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ImportError::TooLarge.into()
    } else {
        AppError::BadRequest("The upload could not be read".into())
    }
}

#[derive(Debug, PartialEq)]
pub struct Imported {
    pub title: String,
    pub content: Value,
}

#[derive(Debug, PartialEq)]
pub enum ImportError {
    UnsupportedType,
    TooLarge,
    NotUtf8,
    Empty,
}

pub fn convert(file_name: &str, bytes: &[u8]) -> Result<Imported, ImportError> {
    let (stem, ext) = file_name.rsplit_once('.').unwrap_or((file_name, ""));
    let ext = ext.to_ascii_lowercase();
    if !IMPORT_EXTENSIONS.contains(&ext.as_str()) {
        return Err(ImportError::UnsupportedType);
    }
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(ImportError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ImportError::NotUtf8)?;
    // Windows editors often prefix UTF-8 files with a byte-order mark, which would
    // otherwise stop the first line from being read as Markdown.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text).replace("\r\n", "\n");
    if text.trim().is_empty() {
        return Err(ImportError::Empty);
    }
    let content = match ext.as_str() {
        "md" => markdown_to_delta(&text),
        _ => plain_text_to_delta(&text),
    };
    let title = match stem.trim() {
        "" => DEFAULT_TITLE,
        name => name,
    };
    Ok(Imported {
        title: title.chars().take(MAX_TITLE_LEN).collect(),
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
    let mut inline = Map::new();
    let mut lists: Vec<&str> = Vec::new();
    let mut item_line_open = false;
    let mut in_heading = false;
    for event in Parser::new(text) {
        match event {
            // Markup with no Quill counterpart here is kept as its text, so no words are lost.
            Event::Text(t) | Event::Code(t) | Event::Html(t) | Event::InlineHtml(t) => {
                delta.push(&t, inline.clone())
            }
            Event::SoftBreak => delta.push(" ", inline.clone()),
            Event::HardBreak if lists.is_empty() && !in_heading => delta.push("\n", Map::new()),
            // Quill has no break inside a line, and a new line here would split the list
            // item or the heading in two.
            Event::HardBreak => delta.push(" ", inline.clone()),
            Event::Start(Tag::Strong) => {
                inline.insert("bold".to_owned(), json!(true));
            }
            Event::End(TagEnd::Strong) => {
                inline.remove("bold");
            }
            Event::Start(Tag::Emphasis) => {
                inline.insert("italic".to_owned(), json!(true));
            }
            Event::End(TagEnd::Emphasis) => {
                inline.remove("italic");
            }
            Event::End(TagEnd::Paragraph) if lists.is_empty() => delta.push("\n", Map::new()),
            // A loose list wraps each item's text in a paragraph; that paragraph is the item's line.
            Event::End(TagEnd::Paragraph) => {
                delta.push("\n", list_line(&lists));
                item_line_open = false;
            }
            Event::Start(Tag::List(first_number)) => {
                // A nested list starts inside its parent item, whose own line must end first.
                if item_line_open || in_item_text(&delta, &lists) {
                    delta.push("\n", list_line(&lists));
                    item_line_open = false;
                }
                lists.push(if first_number.is_some() { "ordered" } else { "bullet" });
            }
            // A tight list has no paragraph around the item's text, so a block that follows
            // it in the same item would otherwise continue the item's line. An item that
            // opens with a block has no text of its own and gets no line.
            Event::Start(
                tag @ (Tag::CodeBlock(_) | Tag::BlockQuote(_) | Tag::Heading { .. } | Tag::HtmlBlock),
            ) => {
                if in_item_text(&delta, &lists) {
                    delta.push("\n", list_line(&lists));
                }
                item_line_open = false;
                in_heading = matches!(tag, Tag::Heading { .. });
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Item) => item_line_open = true,
            Event::End(TagEnd::Item) => {
                if item_line_open || in_item_text(&delta, &lists) {
                    delta.push("\n", list_line(&lists));
                    item_line_open = false;
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                delta.push("\n", attrs([("header", json!(level as u8))]));
                in_heading = false;
            }
            _ => {}
        }
    }
    delta.into_value()
}

/// Whether a list item has text that no line ending has closed yet, which happens
/// when the text follows a block inside the same tight item.
fn in_item_text(delta: &Delta, lists: &[&str]) -> bool {
    !lists.is_empty() && delta.mid_line()
}

/// Line attributes of an item in the innermost open list.
fn list_line(lists: &[&str]) -> Map<String, Value> {
    let mut line = attrs([("list", json!(lists.last().copied().unwrap_or("bullet")))]);
    if lists.len() > 1 {
        line.insert("indent".to_owned(), json!(lists.len() - 1));
    }
    line
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

    fn mid_line(&self) -> bool {
        self.ops.last().is_some_and(|(text, _)| !text.ends_with('\n'))
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
