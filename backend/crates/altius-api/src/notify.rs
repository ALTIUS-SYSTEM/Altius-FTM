//! Outbound SMS / WhatsApp via a generic HTTP gateway.
//!
//! One concrete client, not a provider trait: there is exactly one gateway
//! configured at a time, and the shape below (POST JSON, bearer token) is what
//! Twilio, Wassenger and the regional Indonesian gateways all accept. A second
//! provider that genuinely does not fit is the moment to introduce an
//! abstraction — not before.

use serde_json::json;

use crate::config::NotifyConfig;
use crate::error::{ApiError, ApiResult};

/// Longest message we will relay. SMS concatenation past this is a config
/// mistake, and an unbounded body is an unbounded bill.
pub const MAX_MESSAGE_CHARS: usize = 1600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Sms,
    WhatsApp,
}

impl Channel {
    fn label(self) -> &'static str {
        match self {
            Self::Sms => "sms",
            Self::WhatsApp => "whatsapp",
        }
    }
}

/// Normalise a phone number to E.164-ish digits with a leading `+`.
///
/// Rejects rather than guesses: a wrong number is a message delivered to a
/// stranger, and the driver's daily report is not something to misroute.
pub fn normalize_msisdn(raw: &str) -> ApiResult<String> {
    let trimmed = raw.trim();
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    // Indonesian local form: 08xx -> +628xx. This assumes an ID fleet — a
    // leading 0 in another country's national format would be mis-prefixed,
    // so callers outside Indonesia should pass full E.164.
    let e164 = match digits.strip_prefix('0') {
        Some(rest) => format!("62{rest}"),
        None => digits,
    };
    if e164.len() < 8 || e164.len() > 15 {
        return Err(ApiError::BadRequest(
            "recipient is not a valid phone number".into(),
        ));
    }
    Ok(format!("+{e164}"))
}

pub struct Notifier<'a> {
    http: &'a reqwest::Client,
    config: &'a NotifyConfig,
}

impl<'a> Notifier<'a> {
    pub fn new(http: &'a reqwest::Client, config: &'a NotifyConfig) -> Self {
        Self { http, config }
    }

    pub async fn send(&self, channel: Channel, to: &str, message: &str) -> ApiResult<String> {
        if message.trim().is_empty() {
            return Err(ApiError::BadRequest("message is empty".into()));
        }
        if message.chars().count() > MAX_MESSAGE_CHARS {
            return Err(ApiError::BadRequest(format!(
                "message exceeds {MAX_MESSAGE_CHARS} characters"
            )));
        }
        let recipient = normalize_msisdn(to)?;
        let url = match channel {
            Channel::Sms => self.config.sms_url.as_deref(),
            Channel::WhatsApp => self.config.whatsapp_url.as_deref(),
        }
        .ok_or_else(|| {
            ApiError::Unavailable(format!("{} gateway not configured", channel.label()))
        })?;

        let res = self
            .http
            .post(url)
            .bearer_auth(&self.config.token)
            .json(&json!({
                "from": self.config.sender,
                "to": recipient,
                "channel": channel.label(),
                "message": message,
            }))
            .send()
            .await
            .map_err(|e| ApiError::upstream("messaging gateway", e))?;

        if !res.status().is_success() {
            // Gateway bodies routinely echo the credential or the full request;
            // log the status only and keep the client response fixed.
            tracing::error!(status = %res.status(), channel = channel.label(), "gateway rejected message");
            return Err(ApiError::Unavailable(
                "messaging gateway unavailable".into(),
            ));
        }
        // Recipient is logged, message body is not: it carries operational PII.
        tracing::info!(channel = channel.label(), %recipient, "message accepted by gateway");
        Ok(recipient)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indonesian_local_numbers_become_e164() {
        assert_eq!(
            normalize_msisdn("0812-3456-7890").unwrap(),
            "+6281234567890"
        );
        assert_eq!(
            normalize_msisdn("+62 812 3456 7890").unwrap(),
            "+6281234567890"
        );
        assert_eq!(
            normalize_msisdn(" 6281234567890 ").unwrap(),
            "+6281234567890"
        );
    }

    #[test]
    fn implausible_numbers_are_rejected_not_guessed() {
        for bad in ["", "12", "not a number", "0", &"9".repeat(20)] {
            assert!(normalize_msisdn(bad).is_err(), "should reject {bad:?}");
        }
    }
}
