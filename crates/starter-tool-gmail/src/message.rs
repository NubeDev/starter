//! RFC 5322 message construction for `users.messages.send`.
//!
//! Lifted (with naming adapted to the `Gmail*` prefix) from
//! `codeless/crates/codeless-tools/src/email/message.rs`. The shape
//! is deliberately narrow: one text body, one html body, or both as
//! a `multipart/alternative`. Attachments and inline images are
//! out-of-scope for v0.1 — every transport target (Gmail REST today,
//! SMTP later) accepts a raw RFC 5322 blob, so adding
//! `multipart/mixed` later is a [`GmailMessage::to_rfc5322`] change
//! with no trait churn.
//!
//! The builder is part of the public surface so a consumer can
//! produce the bytes once and inspect / log them (the rendered MIME
//! is *not* a secret, the access token is — they should never appear
//! in the same log line).

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

/// An RFC 5322 mailbox (`addr-spec` plus an optional display name).
///
/// Display names go through [`Self::encode_header`] which falls back
/// to MIME encoded-word form for any non-ASCII or quote-bearing
/// name. The caller does not have to know about header escaping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMailbox {
    /// The email address (`local@domain`).
    pub address: String,
    /// Optional display name. Folded into MIME encoded-word form if
    /// it contains characters that would otherwise need quoting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl GmailMailbox {
    /// Address-only mailbox.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            name: None,
        }
    }

    /// Mailbox with a display name. See [`Self::encode_header`].
    pub fn with_name(address: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            name: Some(name.into()),
        }
    }

    /// Encode as a header value. Display names with non-ASCII or
    /// quote characters are wrapped in MIME encoded-word form so we
    /// never have to reason about header quoting subtleties.
    pub fn encode_header(&self) -> String {
        match &self.name {
            None => self.address.clone(),
            Some(name) if name.is_ascii() && !name.contains(['"', '\\', '<', '>']) => {
                format!("\"{}\" <{}>", name, self.address)
            }
            Some(name) => {
                let b64 = base64_standard(name.as_bytes());
                format!("=?UTF-8?B?{}?= <{}>", b64, self.address)
            }
        }
    }
}

/// The message the caller wants Gmail to send.
///
/// Either [`Self::text`] or [`Self::html`] (or both) must be set;
/// at least one of [`Self::to`] / [`Self::cc`] / [`Self::bcc`] must
/// be populated. Those two invariants are checked by
/// [`Self::to_rfc5322`] and surface as [`MessageError`] / the
/// [`crate::GmailError::Build`] variant.
///
/// `Bcc` is deliberately omitted from the rendered header block —
/// Gmail's `users.messages.send` honours `Bcc` via the SMTP-style
/// envelope it constructs from the headers it *does* see, plus an
/// explicit `Bcc:` header is widely treated as a footgun. We match
/// what every mainstream MTA expects.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GmailMessage {
    /// `From:` mailbox. When omitted, Gmail fills it in from the
    /// authenticated account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<GmailMailbox>,
    /// `To:` recipients.
    #[serde(default)]
    pub to: Vec<GmailMailbox>,
    /// `Cc:` recipients.
    #[serde(default)]
    pub cc: Vec<GmailMailbox>,
    /// `Bcc:` recipients. Routed by Gmail; not rendered into
    /// headers.
    #[serde(default)]
    pub bcc: Vec<GmailMailbox>,
    /// Optional `Reply-To:` mailbox.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<GmailMailbox>,
    /// `Subject:` line.
    pub subject: String,
    /// Plain-text body. UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// HTML body. UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
}

impl GmailMessage {
    /// Render the message as an RFC 5322 byte blob, suitable for
    /// base64url-wrapping and stuffing into the `raw` field of
    /// `users.messages.send`.
    pub fn to_rfc5322(&self) -> Result<Vec<u8>, MessageError> {
        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err(MessageError::NoRecipients);
        }
        if self.text.is_none() && self.html.is_none() {
            return Err(MessageError::NoBody);
        }

        let mut out = String::new();

        if let Some(from) = &self.from {
            writeln_crlf(&mut out, &format!("From: {}", from.encode_header()));
        }
        if !self.to.is_empty() {
            writeln_crlf(&mut out, &format!("To: {}", join_mailboxes(&self.to)));
        }
        if !self.cc.is_empty() {
            writeln_crlf(&mut out, &format!("Cc: {}", join_mailboxes(&self.cc)));
        }
        // Bcc deliberately omitted from the header block — Gmail
        // handles the recipient list separately.
        if let Some(reply_to) = &self.reply_to {
            writeln_crlf(&mut out, &format!("Reply-To: {}", reply_to.encode_header()));
        }
        writeln_crlf(
            &mut out,
            &format!("Subject: {}", encode_subject(&self.subject)),
        );
        writeln_crlf(&mut out, "MIME-Version: 1.0");

        match (&self.text, &self.html) {
            (Some(text), None) => {
                writeln_crlf(&mut out, "Content-Type: text/plain; charset=UTF-8");
                writeln_crlf(&mut out, "Content-Transfer-Encoding: 8bit");
                writeln_crlf(&mut out, "");
                out.push_str(&normalise_crlf(text));
            }
            (None, Some(html)) => {
                writeln_crlf(&mut out, "Content-Type: text/html; charset=UTF-8");
                writeln_crlf(&mut out, "Content-Transfer-Encoding: 8bit");
                writeln_crlf(&mut out, "");
                out.push_str(&normalise_crlf(html));
            }
            (Some(text), Some(html)) => {
                let boundary = "----=_starter_gmail_alt_boundary_b2f0";
                writeln_crlf(
                    &mut out,
                    &format!(
                        "Content-Type: multipart/alternative; boundary=\"{}\"",
                        boundary
                    ),
                );
                writeln_crlf(&mut out, "");
                let _ = write!(out, "--{}\r\n", boundary);
                writeln_crlf(&mut out, "Content-Type: text/plain; charset=UTF-8");
                writeln_crlf(&mut out, "Content-Transfer-Encoding: 8bit");
                writeln_crlf(&mut out, "");
                out.push_str(&normalise_crlf(text));
                out.push_str("\r\n");
                let _ = write!(out, "--{}\r\n", boundary);
                writeln_crlf(&mut out, "Content-Type: text/html; charset=UTF-8");
                writeln_crlf(&mut out, "Content-Transfer-Encoding: 8bit");
                writeln_crlf(&mut out, "");
                out.push_str(&normalise_crlf(html));
                out.push_str("\r\n");
                let _ = write!(out, "--{}--\r\n", boundary);
            }
            (None, None) => unreachable!("guarded above"),
        }

        Ok(out.into_bytes())
    }
}

/// Why a [`GmailMessage`] failed to render.
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    /// The message has no `To:`, `Cc:`, or `Bcc:` recipient — Gmail
    /// would reject the call.
    #[error("message has no recipients")]
    NoRecipients,
    /// The message has neither a plain-text nor an HTML body.
    #[error("message has neither a text nor an html body")]
    NoBody,
}

fn writeln_crlf(buf: &mut String, line: &str) {
    buf.push_str(line);
    buf.push_str("\r\n");
}

fn join_mailboxes(list: &[GmailMailbox]) -> String {
    list.iter()
        .map(GmailMailbox::encode_header)
        .collect::<Vec<_>>()
        .join(", ")
}

fn encode_subject(s: &str) -> String {
    if s.is_ascii() {
        s.to_string()
    } else {
        format!("=?UTF-8?B?{}?=", base64_standard(s.as_bytes()))
    }
}

/// Normalise lone LF / CR into CRLF so user-provided bodies don't
/// accidentally break MIME framing.
fn normalise_crlf(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\r' {
            out.push('\r');
            out.push('\n');
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
        } else if b == b'\n' {
            out.push('\r');
            out.push('\n');
            i += 1;
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

fn base64_standard(bytes: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_simple_text_message() {
        let msg = GmailMessage {
            from: Some(GmailMailbox::new("a@example.com")),
            to: vec![GmailMailbox::new("b@example.com")],
            subject: "hello".into(),
            text: Some("hi there".into()),
            ..Default::default()
        };
        let raw = String::from_utf8(msg.to_rfc5322().unwrap()).unwrap();
        assert!(raw.contains("From: a@example.com\r\n"));
        assert!(raw.contains("To: b@example.com\r\n"));
        assert!(raw.contains("Subject: hello\r\n"));
        assert!(raw.contains("Content-Type: text/plain"));
        assert!(raw.ends_with("hi there"));
    }

    #[test]
    fn rejects_empty_recipients() {
        let msg = GmailMessage {
            subject: "x".into(),
            text: Some("y".into()),
            ..Default::default()
        };
        assert!(matches!(msg.to_rfc5322(), Err(MessageError::NoRecipients)));
    }

    #[test]
    fn rejects_empty_body() {
        let msg = GmailMessage {
            to: vec![GmailMailbox::new("a@example.com")],
            subject: "x".into(),
            ..Default::default()
        };
        assert!(matches!(msg.to_rfc5322(), Err(MessageError::NoBody)));
    }

    #[test]
    fn bcc_is_not_in_headers() {
        let msg = GmailMessage {
            to: vec![GmailMailbox::new("a@example.com")],
            bcc: vec![GmailMailbox::new("secret@example.com")],
            subject: "x".into(),
            text: Some("y".into()),
            ..Default::default()
        };
        let raw = String::from_utf8(msg.to_rfc5322().unwrap()).unwrap();
        assert!(!raw.to_lowercase().contains("bcc:"));
    }

    #[test]
    fn multipart_alternative_when_both_bodies_present() {
        let msg = GmailMessage {
            to: vec![GmailMailbox::new("a@example.com")],
            subject: "x".into(),
            text: Some("plain".into()),
            html: Some("<p>html</p>".into()),
            ..Default::default()
        };
        let raw = String::from_utf8(msg.to_rfc5322().unwrap()).unwrap();
        assert!(raw.contains("multipart/alternative"));
        assert!(raw.contains("plain"));
        assert!(raw.contains("<p>html</p>"));
    }

    #[test]
    fn non_ascii_name_uses_encoded_word() {
        let m = GmailMailbox::with_name("a@example.com", "Zoë");
        let h = m.encode_header();
        assert!(h.starts_with("=?UTF-8?B?"));
        assert!(h.ends_with(" <a@example.com>"));
    }
}
