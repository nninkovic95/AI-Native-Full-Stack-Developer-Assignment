//! Colour theme choice, kept free of browser types so it is unit-tested natively.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn from_stored(saved: Option<&str>, system_prefers_dark: bool) -> Theme {
        match saved {
            Some("dark") => Theme::Dark,
            Some("light") => Theme::Light,
            _ if system_prefers_dark => Theme::Dark,
            _ => Theme::Light,
        }
    }

    pub fn toggled(self) -> Theme {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// Label of the toggle button while this theme is active: it names the mode the click switches to.
    pub fn toggle_label(self) -> &'static str {
        match self {
            Theme::Light => "Dark mode",
            Theme::Dark => "Light mode",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_choice_wins_over_system_preference() {
        assert_eq!(Theme::from_stored(Some("dark"), false), Theme::Dark);
        assert_eq!(Theme::from_stored(Some("light"), true), Theme::Light);
    }

    #[test]
    fn toggling_switches_to_the_other_theme() {
        assert_eq!(Theme::Light.toggled(), Theme::Dark);
        assert_eq!(Theme::Dark.toggled(), Theme::Light);
    }

    #[test]
    fn as_str_is_the_value_stored_and_set_on_the_page() {
        assert_eq!(Theme::Light.as_str(), "light");
        assert_eq!(Theme::Dark.as_str(), "dark");
    }

    #[test]
    fn toggle_label_names_the_mode_the_click_switches_to() {
        assert_eq!(Theme::Light.toggle_label(), "Dark mode");
        assert_eq!(Theme::Dark.toggle_label(), "Light mode");
    }

    #[test]
    fn missing_or_unknown_saved_value_follows_the_system() {
        assert_eq!(Theme::from_stored(None, true), Theme::Dark);
        assert_eq!(Theme::from_stored(None, false), Theme::Light);
        assert_eq!(Theme::from_stored(Some("sepia"), true), Theme::Dark);
        assert_eq!(Theme::from_stored(Some(""), false), Theme::Light);
    }
}
