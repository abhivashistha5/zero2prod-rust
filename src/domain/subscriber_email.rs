use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if ValidateEmail::validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claims::{assert_err, assert_ok};
    use fake::{faker::internet::en::SafeEmail, Fake};

    use crate::domain::SubscriberEmail;

    #[test]
    fn valid_email_is_parsed_correctly() {
        let email = SafeEmail().fake();
        assert_ok!(SubscriberEmail::parse(email));
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = String::from("");
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_symbol_is_rejected() {
        let email = String::from("brucewayne.com");
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = String::from("@wayne.com");
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_domain_is_rejected() {
        let email = String::from("bruce@");
        assert_err!(SubscriberEmail::parse(email));
    }
}
