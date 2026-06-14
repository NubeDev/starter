//! Wire types describing a configurable ArkFlow component kind.
//!
//! The UI renders a form from these: one [`ComponentKind`] per `type:` value,
//! each carrying the fields a user fills in.

use serde::Serialize;

/// One selectable component (e.g. the `kafka` input or the `sql` processor).
#[derive(Debug, Clone, Serialize)]
pub struct ComponentKind {
    /// The `type:` discriminator written into the config.
    pub r#type: String,
    /// Human label for the picker.
    pub label: String,
    /// One-line description.
    pub summary: String,
    /// The configurable fields, in display order.
    pub fields: Vec<Field>,
}

/// A single configurable field on a component.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    pub required: bool,
    pub placeholder: Option<String>,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    Number,
    Duration,
    Bool,
    /// A multi-line value (SQL, JSON document).
    Code,
    /// A comma-or-newline separated list of strings.
    List,
}

impl Field {
    pub fn new(name: &str, kind: FieldKind, required: bool) -> Self {
        Self {
            name: name.to_string(),
            kind,
            required,
            placeholder: None,
            help: None,
        }
    }

    pub fn with(mut self, placeholder: &str, help: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self.help = Some(help.to_string());
        self
    }
}

impl ComponentKind {
    pub fn new(r#type: &str, label: &str, summary: &str, fields: Vec<Field>) -> Self {
        Self {
            r#type: r#type.to_string(),
            label: label.to_string(),
            summary: summary.to_string(),
            fields,
        }
    }
}
