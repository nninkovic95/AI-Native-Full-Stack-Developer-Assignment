//! The import converter seam: (file name, bytes) -> title + Quill Delta, or a rejection.
//! Expected Deltas are written out by hand in Quill's canonical form.

use serde_json::json;
use server::import::{convert, ImportError};

#[test]
fn a_text_file_becomes_a_document_titled_after_the_file() {
    let doc = convert("Meeting notes.txt", b"First line\r\nSecond line").unwrap();

    assert_eq!(doc.title, "Meeting notes");
    assert_eq!(doc.content, json!({ "ops": [{ "insert": "First line\nSecond line\n" }] }));
}

#[test]
fn a_byte_order_mark_does_not_hide_the_first_heading() {
    let doc = convert("windows.md", b"\xEF\xBB\xBF# Title\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [{ "insert": "Title" }, { "insert": "\n", "attributes": { "header": 1 } }] })
    );
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

#[test]
fn markdown_bold_and_italic_become_inline_attributes() {
    let doc = convert("notes.md", b"Hello **bold** and *slanted* text.\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Hello " },
            { "insert": "bold", "attributes": { "bold": true } },
            { "insert": " and " },
            { "insert": "slanted", "attributes": { "italic": true } },
            { "insert": " text.\n" }
        ]})
    );
}

#[test]
fn italic_nested_in_bold_carries_both_attributes() {
    let doc = convert("notes.md", b"**loud and *leaning***\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "loud and ", "attributes": { "bold": true } },
            { "insert": "leaning", "attributes": { "bold": true, "italic": true } },
            { "insert": "\n" }
        ]})
    );
}

#[test]
fn a_bullet_list_becomes_bullet_lines() {
    let doc = convert("shopping.md", b"- Apples\n- Pears\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Apples" },
            { "insert": "\n", "attributes": { "list": "bullet" } },
            { "insert": "Pears" },
            { "insert": "\n", "attributes": { "list": "bullet" } }
        ]})
    );
}

#[test]
fn a_numbered_list_after_a_paragraph_becomes_ordered_lines() {
    let doc = convert("recipe.md", b"Steps:\n\n1. Mix\n2. Bake\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Steps:\nMix" },
            { "insert": "\n", "attributes": { "list": "ordered" } },
            { "insert": "Bake" },
            { "insert": "\n", "attributes": { "list": "ordered" } }
        ]})
    );
}

#[test]
fn a_nested_list_item_is_indented_under_its_parent() {
    let doc = convert("food.md", b"- Fruit\n  - Apple\n  - Pear\n- Veg\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Fruit" },
            { "insert": "\n", "attributes": { "list": "bullet" } },
            { "insert": "Apple" },
            { "insert": "\n", "attributes": { "list": "bullet", "indent": 1 } },
            { "insert": "Pear" },
            { "insert": "\n", "attributes": { "list": "bullet", "indent": 1 } },
            { "insert": "Veg" },
            { "insert": "\n", "attributes": { "list": "bullet" } }
        ]})
    );
}

#[test]
fn a_loose_list_has_no_blank_line_between_items() {
    let doc = convert("todo.md", b"- Call the bank\n\n- Renew passport\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Call the bank" },
            { "insert": "\n", "attributes": { "list": "bullet" } },
            { "insert": "Renew passport" },
            { "insert": "\n", "attributes": { "list": "bullet" } }
        ]})
    );
}

#[test]
fn a_code_block_inside_a_list_item_does_not_swallow_the_items_text() {
    let doc = convert("setup.md", b"- Install:\n  ```\n  npm install\n  ```\n- Run\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Install:" },
            { "insert": "\n", "attributes": { "list": "bullet" } },
            { "insert": "npm install\nRun" },
            { "insert": "\n", "attributes": { "list": "bullet" } }
        ]})
    );
}

#[test]
fn text_after_a_code_block_in_a_list_item_stays_in_that_item() {
    let doc = convert(
        "setup.md",
        b"1. Install:\n   ```\n   npm install\n   ```\n   then restart\n2. Run\n",
    )
    .unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Install:" },
            { "insert": "\n", "attributes": { "list": "ordered" } },
            { "insert": "npm install\nthen restart" },
            { "insert": "\n", "attributes": { "list": "ordered" } },
            { "insert": "Run" },
            { "insert": "\n", "attributes": { "list": "ordered" } }
        ]})
    );
}

#[test]
fn a_list_item_that_is_only_a_code_block_leaves_no_empty_item() {
    let doc = convert("setup.md", b"- ```\n  npm install\n  ```\n- Run\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "npm install\nRun" },
            { "insert": "\n", "attributes": { "list": "bullet" } }
        ]})
    );
}

#[test]
fn a_quote_inside_a_list_item_gets_its_own_line() {
    let doc = convert("shopping.md", b"- Buy milk\n  > from the shop\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Buy milk" },
            { "insert": "\n", "attributes": { "list": "bullet" } },
            { "insert": "from the shop" },
            { "insert": "\n", "attributes": { "list": "bullet" } }
        ]})
    );
}

#[test]
fn a_line_break_inside_a_markdown_paragraph_becomes_a_space() {
    let doc = convert("wrapped.md", b"First line\nstill the same paragraph.\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [{ "insert": "First line still the same paragraph.\n" }] })
    );
}

#[test]
fn a_markdown_hard_line_break_keeps_the_two_lines_apart() {
    let doc = convert("address.md", b"Ada Lovelace  \n12 Example Street\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [{ "insert": "Ada Lovelace\n12 Example Street\n" }] })
    );
}

#[test]
fn a_hard_line_break_in_a_list_item_does_not_split_the_item() {
    let doc = convert("people.md", b"1. Ada Lovelace  \n   12 Example Street\n2. Grace Hopper\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Ada Lovelace 12 Example Street" },
            { "insert": "\n", "attributes": { "list": "ordered" } },
            { "insert": "Grace Hopper" },
            { "insert": "\n", "attributes": { "list": "ordered" } }
        ]})
    );
}

#[test]
fn a_hard_line_break_in_a_heading_does_not_split_the_heading() {
    let doc = convert("report.md", b"Annual report  \nfor 2024\n===\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [
            { "insert": "Annual report for 2024" },
            { "insert": "\n", "attributes": { "header": 1 } }
        ]})
    );
}

#[test]
fn links_and_inline_code_keep_their_words_as_plain_text() {
    let doc = convert("howto.md", b"See [the docs](https://example.com) and run `cargo test`.\n").unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [{ "insert": "See the docs and run cargo test.\n" }] })
    );
}

#[test]
fn quotes_html_and_code_blocks_keep_their_words_as_plain_lines() {
    let doc = convert(
        "mixed.md",
        b"> Quoted words\n\n<div>Raw words</div>\n\n```\nlet x = 1;\n```\n\nLast\n",
    )
    .unwrap();

    assert_eq!(
        doc.content,
        json!({ "ops": [{ "insert": "Quoted words\n<div>Raw words</div>\nlet x = 1;\nLast\n" }] })
    );
}

#[test]
fn the_file_extension_is_matched_in_any_letter_case() {
    let markdown = convert("NOTES.MD", b"# Hi\n").unwrap();
    let text = convert("a.TXT", b"# Hi\n").unwrap();

    assert_eq!(markdown.title, "NOTES");
    assert_eq!(
        markdown.content,
        json!({ "ops": [{ "insert": "Hi" }, { "insert": "\n", "attributes": { "header": 1 } }] })
    );
    assert_eq!(text.title, "a");
    assert_eq!(text.content, json!({ "ops": [{ "insert": "# Hi\n" }] }));
}

#[test]
fn the_title_is_the_file_name_without_surrounding_spaces() {
    let doc = convert("  Trip plan .txt", b"Pack bags").unwrap();

    assert_eq!(doc.title, "Trip plan");
}

#[test]
fn a_file_with_no_name_before_the_extension_gets_the_default_title() {
    let doc = convert(".md", b"Some words\n").unwrap();

    assert_eq!(doc.title, "Untitled document");
}

#[test]
fn a_very_long_file_name_is_cut_to_200_characters() {
    let name = format!("{}.txt", "é".repeat(250));

    let doc = convert(&name, b"Some words").unwrap();

    assert_eq!(doc.title, "é".repeat(200));
}

#[test]
fn files_that_are_not_txt_or_md_are_rejected() {
    assert_eq!(convert("report.docx", b"Some words"), Err(ImportError::UnsupportedType));
    assert_eq!(convert("README", b"Some words"), Err(ImportError::UnsupportedType));
}

#[test]
fn a_file_over_one_megabyte_is_rejected_and_one_of_exactly_that_size_is_not() {
    let one_megabyte = 1024 * 1024;

    assert_eq!(convert("big.txt", &vec![b'a'; one_megabyte + 1]), Err(ImportError::TooLarge));
    assert!(convert("fits.txt", &vec![b'a'; one_megabyte]).is_ok());
}

#[test]
fn a_file_that_is_not_utf8_text_is_rejected() {
    assert_eq!(convert("latin1.txt", b"caf\xe9"), Err(ImportError::NotUtf8));
}

#[test]
fn a_file_with_no_text_in_it_is_rejected() {
    assert_eq!(convert("blank.txt", b""), Err(ImportError::Empty));
    assert_eq!(convert("spaces.md", b" \r\n\t\n"), Err(ImportError::Empty));
}
