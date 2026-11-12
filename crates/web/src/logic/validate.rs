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
}
