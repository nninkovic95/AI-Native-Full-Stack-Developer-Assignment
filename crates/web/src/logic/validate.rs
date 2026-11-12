//! Client-side form checks. The server stays the authority; these only avoid pointless requests.

pub fn validate_login(email: &str, password: &str) -> Result<(), String> {
    let email = email.trim();
    if email.is_empty() {
        return Err("Enter your email.".to_string());
    }
    if !email.contains('@') {
        return Err("Enter a valid email address.".to_string());
    }
    if password.is_empty() {
        return Err("Enter your password.".to_string());
    }
    Ok(())
}

/// Trims a title, cuts it to the server limit and falls back to the default when blank.
pub fn normalize_title(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return shared::DEFAULT_TITLE.to_string();
    }
    trimmed.chars().take(shared::MAX_TITLE_LEN).collect()
}

/// Checks a file picked for import by name and size; the server repeats these checks.
pub fn validate_import(file_name: &str, size_bytes: u64) -> Result<(), String> {
    let supported = file_name
        .rsplit_once('.')
        .is_some_and(|(_, ext)| shared::IMPORT_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)));
    if !supported {
        return Err("Only .txt and .md files can be imported".to_string());
    }
    if size_bytes == 0 {
        return Err("The file is empty".to_string());
    }
    if size_bytes > shared::MAX_IMPORT_BYTES as u64 {
        return Err("The file is larger than 1 MB".to_string());
    }
    Ok(())
}

/// Value for the file input's `accept` attribute, e.g. `.txt,.md`.
pub fn import_accept() -> String {
    shared::IMPORT_EXTENSIONS.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(",")
}

/// Short line telling the user which files can be imported and how big they may be.
pub fn import_hint() -> String {
    let types = shared::IMPORT_EXTENSIONS.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(" or ");
    format!("Import a {types} file, up to {} MB", shared::MAX_IMPORT_BYTES / (1024 * 1024))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_needs_an_email_with_an_at_sign() {
        assert_eq!(
            validate_login("", "ajaia-demo"),
            Err("Enter your email.".to_string())
        );
        assert_eq!(
            validate_login("alice", "ajaia-demo"),
            Err("Enter a valid email address.".to_string())
        );
    }

    #[test]
    fn login_needs_a_password_and_accepts_a_complete_form() {
        assert_eq!(
            validate_login("alice@ajaia.test", ""),
            Err("Enter your password.".to_string())
        );
        assert_eq!(validate_login(" alice@ajaia.test ", "ajaia-demo"), Ok(()));
    }

    #[test]
    fn title_is_trimmed_and_falls_back_to_the_default() {
        assert_eq!(normalize_title("  Quarterly plan  "), "Quarterly plan");
        assert_eq!(normalize_title("   "), shared::DEFAULT_TITLE);
    }

    #[test]
    fn title_is_cut_to_the_server_limit_by_characters() {
        let long = "é".repeat(shared::MAX_TITLE_LEN + 20);
        assert_eq!(normalize_title(&long).chars().count(), shared::MAX_TITLE_LEN);
    }

    #[test]
    fn import_refuses_unsupported_file_types() {
        let msg = "Only .txt and .md files can be imported".to_string();
        assert_eq!(validate_import("report.docx", 10), Err(msg.clone()));
        assert_eq!(validate_import("noextension", 10), Err(msg.clone()));
        assert_eq!(validate_import("notes.txt.exe", 10), Err(msg));
    }

    #[test]
    fn import_accepts_txt_and_md_in_any_letter_case() {
        assert_eq!(validate_import("notes.txt", 5), Ok(()));
        assert_eq!(validate_import("README.MD", 5), Ok(()));
        assert_eq!(validate_import("my.plan.Md", 5), Ok(()));
    }

    #[test]
    fn import_refuses_an_empty_file() {
        assert_eq!(validate_import("notes.txt", 0), Err("The file is empty".to_string()));
    }

    #[test]
    fn import_allows_exactly_the_limit_and_refuses_one_byte_more() {
        let limit = 1024 * 1024;
        assert_eq!(validate_import("big.md", limit), Ok(()));
        assert_eq!(
            validate_import("big.md", limit + 1),
            Err("The file is larger than 1 MB".to_string())
        );
    }

    #[test]
    fn the_import_hint_names_the_types_and_the_size_limit() {
        assert_eq!(import_hint(), "Import a .txt or .md file, up to 1 MB");
    }

    #[test]
    fn the_file_picker_filter_lists_every_supported_extension() {
        assert_eq!(import_accept(), ".txt,.md");
    }
}
