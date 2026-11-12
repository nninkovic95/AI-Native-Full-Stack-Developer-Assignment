//! Mapping HTTP results and form values to API shapes, without touching the network.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FailureKind {
    Unauthorized,
    NotFound,
    /// The request never got a response (status 0).
    Network,
    Other,
}

impl FailureKind {
    pub fn from_status(status: u16) -> Self {
        match status {
            401 => FailureKind::Unauthorized,
            404 => FailureKind::NotFound,
            0 => FailureKind::Network,
            _ => FailureKind::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApiFailure {
    pub kind: FailureKind,
    pub status: u16,
    pub message: String,
}

/// Builds a failure from a status and the raw response body, which should be a `shared::ApiError`.
pub fn failure_from_response(status: u16, body: &str) -> ApiFailure {
    let kind = FailureKind::from_status(status);
    let message = match serde_json::from_str::<shared::ApiError>(body) {
        Ok(e) => e.error,
        Err(_) if kind == FailureKind::Network => "Could not reach the server".to_string(),
        Err(_) => format!("Request failed (HTTP {status})"),
    };
    ApiFailure { kind, status, message }
}

/// Body for renaming a document: the title only, normalised.
pub fn title_update(raw_title: &str) -> shared::UpdateDocument {
    shared::UpdateDocument {
        title: Some(crate::logic::validate::normalize_title(raw_title)),
        content: None,
    }
}

/// Body for saving editor content from the Delta JSON string the editor returns.
pub fn content_update(delta_json: &str) -> Result<shared::UpdateDocument, String> {
    let content = serde_json::from_str(delta_json).map_err(|e| format!("Invalid editor content: {e}"))?;
    Ok(shared::UpdateDocument { title: None, content: Some(content) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes_map_to_failure_kinds() {
        assert_eq!(FailureKind::from_status(401), FailureKind::Unauthorized);
        assert_eq!(FailureKind::from_status(404), FailureKind::NotFound);
        assert_eq!(FailureKind::from_status(500), FailureKind::Other);
        assert_eq!(FailureKind::from_status(0), FailureKind::Network);
    }

    #[test]
    fn the_message_comes_from_the_error_body_or_a_generic_fallback() {
        let f = failure_from_response(401, r#"{"error":"Wrong email or password"}"#);
        assert_eq!(f.kind, FailureKind::Unauthorized);
        assert_eq!(f.message, "Wrong email or password");

        let f = failure_from_response(502, "<html>Bad Gateway</html>");
        assert_eq!(f.kind, FailureKind::Other);
        assert_eq!(f.message, "Request failed (HTTP 502)");

        let f = failure_from_response(0, "");
        assert_eq!(f.message, "Could not reach the server");
    }

    #[test]
    fn update_bodies_carry_only_the_changed_field() {
        let t = serde_json::to_value(title_update("  New name ")).unwrap();
        assert_eq!(t, serde_json::json!({"title": "New name"}));

        let c = serde_json::to_value(content_update(r#"{"ops":[{"insert":"hi\n"}]}"#).unwrap()).unwrap();
        assert_eq!(c, serde_json::json!({"content": {"ops": [{"insert": "hi\n"}]}}));

        assert!(content_update("not json").is_err());
    }
}
