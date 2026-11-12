//! Autosave state machine: which state the editor is in and what to show.

#[derive(Debug, Clone, PartialEq)]
pub enum SaveState {
    Idle,
    Dirty,
    /// `dirty` is true when the user edited again while the request was in flight.
    Saving { dirty: bool },
    Saved,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SaveEvent {
    Changed,
    SaveStarted,
    SaveOk,
    SaveErr(String),
}

impl SaveState {
    pub fn next(self, event: SaveEvent) -> SaveState {
        match (self, event) {
            (SaveState::Saving { .. }, SaveEvent::Changed) => SaveState::Saving { dirty: true },
            (_, SaveEvent::Changed) => SaveState::Dirty,
            (_, SaveEvent::SaveStarted) => SaveState::Saving { dirty: false },
            (SaveState::Saving { dirty: true }, SaveEvent::SaveOk) => SaveState::Dirty,
            (_, SaveEvent::SaveOk) => SaveState::Saved,
            (_, SaveEvent::SaveErr(msg)) => SaveState::Failed(msg),
        }
    }

    /// True when a debounced save should fire now.
    pub fn needs_save(&self) -> bool {
        matches!(self, SaveState::Dirty)
    }

    pub fn label(&self) -> String {
        match self {
            SaveState::Idle => String::new(),
            SaveState::Dirty => "Unsaved changes".to_string(),
            SaveState::Saving { .. } => "Saving…".to_string(),
            SaveState::Saved => "Saved".to_string(),
            SaveState::Failed(msg) => format!("Save failed: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_marks_a_clean_document_dirty() {
        let s = SaveState::Idle.next(SaveEvent::Changed);
        assert_eq!(s, SaveState::Dirty);
    }

    #[test]
    fn a_successful_save_ends_saved() {
        let s = SaveState::Dirty
            .next(SaveEvent::SaveStarted)
            .next(SaveEvent::SaveOk);
        assert_eq!(s, SaveState::Saved);
        assert_eq!(s.label(), "Saved");
    }

    #[test]
    fn an_edit_during_a_save_stays_dirty_afterwards() {
        let s = SaveState::Dirty
            .next(SaveEvent::SaveStarted)
            .next(SaveEvent::Changed)
            .next(SaveEvent::SaveOk);
        assert_eq!(s, SaveState::Dirty);
        assert!(s.needs_save());
    }

    #[test]
    fn a_failed_save_shows_the_error_and_a_new_edit_retries() {
        let failed = SaveState::Dirty
            .next(SaveEvent::SaveStarted)
            .next(SaveEvent::SaveErr("title too long".to_string()));
        assert_eq!(failed.label(), "Save failed: title too long");
        assert!(!failed.needs_save());
        assert_eq!(failed.next(SaveEvent::Changed), SaveState::Dirty);
    }

    #[test]
    fn labels_for_the_in_progress_states() {
        assert_eq!(SaveState::Idle.label(), "");
        assert_eq!(SaveState::Dirty.label(), "Unsaved changes");
        assert_eq!(SaveState::Saving { dirty: false }.label(), "Saving…");
        assert_eq!(SaveState::Saving { dirty: true }.label(), "Saving…");
    }
}
