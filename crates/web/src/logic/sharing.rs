//! Sharing helpers: splitting the document list, labelling ownership and checking the share form.

use shared::{DocumentSummary, User};
use uuid::Uuid;

/// Ownership line shown next to a document: the viewer's own, or who shared it.
pub fn owner_label(is_owner: bool, owner_name: &str) -> String {
    if is_owner {
        "Owned by you".to_string()
    } else {
        format!("Shared by {owner_name}")
    }
}

/// Splits the list response into (owned by the caller, shared with the caller), keeping the order.
pub fn split_documents(docs: Vec<DocumentSummary>) -> (Vec<DocumentSummary>, Vec<DocumentSummary>) {
    docs.into_iter().partition(|d| d.is_owner)
}

/// The share list without one person, keeping the order of the rest.
pub fn without_user(users: Vec<User>, id: Uuid) -> Vec<User> {
    users.into_iter().filter(|u| u.id != id).collect()
}

/// Accessible name of a person's Remove button, so screen readers can tell the buttons apart.
pub fn remove_label(name: &str) -> String {
    format!("Remove {name}")
}

/// Checks the share form input and returns the email the server compares against: trimmed and lower-cased.
/// The server repeats these checks.
pub fn validate_share_email(input: &str, own_email: &str) -> Result<String, String> {
    let email = input.trim().to_lowercase();
    if email.is_empty() {
        return Err("Enter an email address".to_string());
    }
    if !email.contains('@') {
        return Err("That does not look like an email address".to_string());
    }
    if email == own_email.trim().to_lowercase() {
        return Err("You already own this document".to_string());
    }
    Ok(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(title: &str, is_owner: bool) -> DocumentSummary {
        DocumentSummary {
            id: uuid::Uuid::new_v4(),
            title: title.to_string(),
            owner: User { id: uuid::Uuid::new_v4(), email: "o@ajaia.test".to_string(), name: "O".to_string() },
            is_owner,
            updated_at: chrono::Utc::now(),
        }
    }

    fn titles(docs: &[DocumentSummary]) -> Vec<&str> {
        docs.iter().map(|d| d.title.as_str()).collect()
    }

    #[test]
    fn documents_split_into_owned_and_shared_keeping_their_order() {
        let docs = vec![summary("a", true), summary("b", false), summary("c", true), summary("d", false)];
        let (owned, shared) = split_documents(docs);
        assert_eq!(titles(&owned), ["a", "c"]);
        assert_eq!(titles(&shared), ["b", "d"]);
    }

    #[test]
    fn a_blank_share_email_is_refused() {
        let msg = Err("Enter an email address".to_string());
        assert_eq!(validate_share_email("", "alice@ajaia.test"), msg);
        assert_eq!(validate_share_email("   ", "alice@ajaia.test"), msg);
    }

    #[test]
    fn a_share_email_without_an_at_sign_is_refused() {
        assert_eq!(
            validate_share_email("bob", "alice@ajaia.test"),
            Err("That does not look like an email address".to_string())
        );
    }

    #[test]
    fn the_owners_own_email_is_refused_in_any_letter_case() {
        let msg = Err("You already own this document".to_string());
        assert_eq!(validate_share_email("alice@ajaia.test", "alice@ajaia.test"), msg);
        assert_eq!(validate_share_email(" ALICE@Ajaia.test ", "alice@ajaia.test"), msg);
    }

    #[test]
    fn a_valid_share_email_comes_back_trimmed_and_lower_cased() {
        assert_eq!(
            validate_share_email("  Bob@Ajaia.TEST ", "alice@ajaia.test"),
            Ok("bob@ajaia.test".to_string())
        );
    }

    #[test]
    fn an_empty_list_splits_into_two_empty_lists() {
        let (owned, shared) = split_documents(Vec::new());
        assert!(owned.is_empty() && shared.is_empty());
    }

    fn person(name: &str) -> User {
        User { id: uuid::Uuid::new_v4(), email: format!("{name}@ajaia.test"), name: name.to_string() }
    }

    #[test]
    fn removing_a_person_keeps_the_order_of_the_others() {
        let people = vec![person("Bob"), person("Carol"), person("Dan")];
        let carol = people[1].id;
        let left = without_user(people, carol);
        let names: Vec<&str> = left.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(names, ["Bob", "Dan"]);
    }

    #[test]
    fn the_remove_label_names_the_person() {
        assert_eq!(remove_label("Bob"), "Remove Bob");
    }

    #[test]
    fn the_owner_label_says_who_owns_the_document() {
        assert_eq!(owner_label(true, "Alice"), "Owned by you");
        assert_eq!(owner_label(false, "Bob"), "Shared by Bob");
    }
}
