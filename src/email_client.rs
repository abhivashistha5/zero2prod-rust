use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: &str,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
    ) -> Self {
        Self {
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
            base_url: base_url.into(),
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        let _ = self
            .http_client
            .post(url)
            .json(&request_body)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use super::EmailClient;

    use claims::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use secrecy::Secret;
    use wiremock::{
        matchers::{header, header_exists, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::domain::SubscriberEmail;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    #[tokio::test]
    async fn send_email_succeeds_if_server_returns_200() {
        let mock_server = MockServer::start().await;

        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri().as_str(),
            sender,
            Secret::new(Faker.fake()),
        );

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let outcome = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn end_email_fails_if_server_returns_500() {
        let mock_server = MockServer::start().await;

        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri().as_str(),
            sender,
            Secret::new(Faker.fake()),
        );

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let outcome = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }
}
