use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        let is_empty = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_chars = ['/', '(', ')', '"', '<', '>', '\\', '[', ']', '{', '}'];

        let contains_forbidden_chars = s.chars().any(|g| forbidden_chars.contains(&g));

        if is_empty || is_too_long || contains_forbidden_chars {
            Err(format!("{} is not valid subscriber name", s))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn name_smaller_than_or_eq_to_256_grapheme_is_valid() {
        let name = "आ".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn name_longer_than_256_grapheme_is_rejected() {
        let name = "आ".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_name_is_rejected() {
        let name = "    ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_char_is_rejected() {
        for forbidden_chars in &['/', '(', ')', '"', '<', '>', '\\', '[', ']', '{', '}'] {
            let name = forbidden_chars.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn valid_name_is_parsed_successfully() {
        let name = "Bruce Wayne".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
