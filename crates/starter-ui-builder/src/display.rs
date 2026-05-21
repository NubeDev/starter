//! Display primitives — `heading()`, `text()`, `badge()`.
//!
//! Each constructor returns a tiny builder so optional fields
//! (`intent`, `level`, `subtitle`) stay opt-in.

use starter_ui_ir::Component;

/// Construct a [`Component::Heading`]. Level defaults to 2 (`<h2>`)
/// at render time.
pub fn heading(content: impl Into<String>) -> HeadingBuilder {
    HeadingBuilder {
        id: None,
        content: content.into(),
        subtitle: None,
        level: None,
    }
}

/// Builder for [`Component::Heading`].
#[derive(Debug, Clone)]
pub struct HeadingBuilder {
    id: Option<String>,
    content: String,
    subtitle: Option<String>,
    level: Option<u8>,
}

impl HeadingBuilder {
    /// Set the heading id.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set a subtitle displayed below the heading text.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the level (1–6).
    pub fn level(mut self, level: u8) -> Self {
        self.level = Some(level);
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Heading {
            id: self.id,
            content: self.content,
            subtitle: self.subtitle,
            level: self.level,
            style: None,
        }
    }
}

/// Construct a [`Component::Text`] span.
pub fn text(content: impl Into<String>) -> TextBuilder {
    TextBuilder {
        id: None,
        content: content.into(),
        intent: None,
    }
}

/// Builder for [`Component::Text`].
#[derive(Debug, Clone)]
pub struct TextBuilder {
    id: Option<String>,
    content: String,
    intent: Option<String>,
}

impl TextBuilder {
    /// Set the id.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the semantic intent — `"info"`, `"success"`, `"warning"`,
    /// or `"danger"`.
    pub fn intent(mut self, intent: impl Into<String>) -> Self {
        self.intent = Some(intent.into());
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Text {
            id: self.id,
            content: self.content,
            intent: self.intent,
            style: None,
        }
    }
}

/// Construct a [`Component::Badge`].
pub fn badge(label: impl Into<String>) -> BadgeBuilder {
    BadgeBuilder {
        id: None,
        label: label.into(),
        intent: None,
    }
}

/// Builder for [`Component::Badge`].
#[derive(Debug, Clone)]
pub struct BadgeBuilder {
    id: Option<String>,
    label: String,
    intent: Option<String>,
}

impl BadgeBuilder {
    /// Set the id.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the semantic intent.
    pub fn intent(mut self, intent: impl Into<String>) -> Self {
        self.intent = Some(intent.into());
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Badge {
            id: self.id,
            label: self.label,
            intent: self.intent,
            style: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_with_level() {
        let h = heading("Title").level(1).build();
        let v = serde_json::to_value(&h).unwrap();
        assert_eq!(v["type"], "heading");
        assert_eq!(v["content"], "Title");
        assert_eq!(v["level"], 1);
    }

    #[test]
    fn text_with_intent() {
        let t = text("hello").intent("warning").build();
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["type"], "text");
        assert_eq!(v["intent"], "warning");
    }

    #[test]
    fn badge_minimal() {
        let b = badge("new").build();
        let v = serde_json::to_value(&b).unwrap();
        assert_eq!(v["type"], "badge");
        assert_eq!(v["label"], "new");
    }
}
