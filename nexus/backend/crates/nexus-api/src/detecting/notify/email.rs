//! Email delivery over SMTP.
//!
//! The channel config carries the SMTP server, the envelope addresses, and an
//! optional username/password — the password is the secret and is never returned
//! to the client (the read path redacts it). The message is sent as a multipart
//! body (plain text plus a minimal HTML rendering) so it is readable in any
//! client. TLS is via rustls. A transport or auth failure is returned as a
//! message for the event record and the retry layer — it never panics the runner.

use lettre::message::{header::ContentType, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;
use serde_json::Value;

use super::Notification;

/// Email channel config. `host`/`port` address the SMTP server; `from`/`to` are
/// the envelope; `username`/`password` are optional SMTP auth (password is a
/// secret). `starttls` selects STARTTLS over implicit TLS when true.
#[derive(Debug, Deserialize)]
struct EmailConfig {
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    from: String,
    to: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    starttls: bool,
}

fn default_port() -> u16 {
    587
}

/// Send the notification as an email. Builds a text+HTML message and dispatches
/// it over an async SMTP transport with rustls TLS.
pub async fn deliver(config: &Value, notification: &Notification) -> Result<(), String> {
    let cfg: EmailConfig =
        serde_json::from_value(config.clone()).map_err(|e| format!("invalid email config: {e}"))?;
    let message = build_message(&cfg, notification)?;
    let transport = build_transport(&cfg)?;
    transport
        .send(message)
        .await
        .map(|_| ())
        .map_err(|e| format!("smtp send failed: {e}"))
}

/// Assemble the RFC 5322 message: a subject derived from the detection +
/// transition, and a multipart body so plain-text-only clients still get it.
fn build_message(cfg: &EmailConfig, n: &Notification) -> Result<Message, String> {
    let subject = format!("[{}] {}", n.transition, n.detection_name);
    let text = n.message.clone();
    let html = format!("<p>{}</p>", html_escape(&n.message));
    Message::builder()
        .from(cfg.from.parse().map_err(|e| format!("bad from: {e}"))?)
        .to(cfg.to.parse().map_err(|e| format!("bad to: {e}"))?)
        .subject(subject)
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(text),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html),
                ),
        )
        .map_err(|e| format!("email build failed: {e}"))
}

/// Build the async SMTP transport for `cfg`, choosing STARTTLS or implicit TLS
/// and attaching credentials when supplied.
fn build_transport(cfg: &EmailConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    let builder = if cfg.starttls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
            .map_err(|e| format!("smtp tls setup failed: {e}"))?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
            .map_err(|e| format!("smtp tls setup failed: {e}"))?
    }
    .port(cfg.port);
    let builder = match (&cfg.username, &cfg.password) {
        (Some(u), Some(p)) => builder.credentials(Credentials::new(u.clone(), p.clone())),
        _ => builder,
    };
    Ok(builder.build())
}

/// Escape the few characters that would break out of the HTML body.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn note() -> Notification {
        Notification {
            detection_name: "Disk".into(),
            transition: "opened".into(),
            target: json!({ "host": "h1" }),
            value: Some(5.0),
            context: json!({}),
            message: "Detection Disk opened".into(),
        }
    }

    #[test]
    fn config_defaults_port_and_starttls() {
        let cfg: EmailConfig = serde_json::from_value(json!({
            "host": "smtp.example.com",
            "from": "a@example.com",
            "to": "b@example.com",
        }))
        .unwrap();
        assert_eq!(cfg.port, 587);
        assert!(!cfg.starttls);
    }

    #[test]
    fn message_builds_with_subject_from_transition() {
        let cfg: EmailConfig = serde_json::from_value(json!({
            "host": "smtp.example.com",
            "from": "a@example.com",
            "to": "b@example.com",
        }))
        .unwrap();
        assert!(build_message(&cfg, &note()).is_ok());
    }

    #[test]
    fn html_escape_neutralises_markup() {
        assert_eq!(html_escape("<b>&</b>"), "&lt;b&gt;&amp;&lt;/b&gt;");
    }
}
