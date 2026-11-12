//! Helpers for deleting a document from the list.

use shared::DocumentSummary;
use uuid::Uuid;

/// Text of the browser confirm dialog shown before a delete.
pub fn delete_confirm_message(title: &str) -> String {
    format!("Delete \"{title}\"? This cannot be undone.")
}

/// The list without the given document, keeping the order of the rest.
pub fn without_document(docs: Vec<DocumentSummary>, id: Uuid) -> Vec<DocumentSummary> {
    docs.into_iter().filter(|d| d.id != id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::User;

    fn summary(title: &str) -> DocumentSummary {
        DocumentSummary {
            id: Uuid::new_v4(),
            title: title.to_string(),
            owner: User { id: Uuid::new_v4(), email: "o@ajaia.test".to_string(), name: "O".to_string() },
            is_owner: true,
            updated_at: chrono::Utc::now(),
        }
    }

    fn titles(docs: &[DocumentSummary]) -> Vec<&str> {
        docs.iter().map(|d| d.title.as_str()).collect()
    }

    #[test]
    fn the_confirm_message_names_the_document_and_warns_it_cannot_be_undone() {
        assert_eq!(
            delete_confirm_message("Q3 plan"),
            "Delete \"Q3 plan\"? This cannot be undone."
        );
    }

    #[test]
    fn removing_a_document_keeps_the_order_of_the_rest() {
        let docs = vec![summary("a"), summary("b"), summary("c")];
        let b = docs[1].id;
        assert_eq!(titles(&without_document(docs, b)), ["a", "c"]);
    }

    #[test]
    fn removing_an_unknown_id_leaves_the_list_unchanged() {
        let docs = vec![summary("a"), summary("b")];
        assert_eq!(titles(&without_document(docs, Uuid::new_v4())), ["a", "b"]);
    }
}
