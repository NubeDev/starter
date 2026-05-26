//! Page-options vocabulary shared by every backend.
//!
//! These types are deliberately serializable and `utoipa`-friendly
//! (when the `axum-router` feature is on) so the same struct can be
//! posted from a browser, deserialized in a CLI, or stored as a saved
//! report preset.

use serde::{Deserialize, Serialize};

#[cfg(feature = "axum-router")]
use utoipa::ToSchema;

/// Page orientation. Defaults to [`Orientation::Portrait`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "axum-router", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    /// Tall page (height ≥ width).
    #[default]
    Portrait,
    /// Wide page (width > height).
    Landscape,
}

/// Named page sizes plus an escape hatch for custom dimensions.
///
/// All dimensions resolved by [`PageSize::dimensions_mm`] are in
/// **millimetres**; the named variants use the ISO / ANSI standards.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "axum-router", derive(ToSchema))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PageSize {
    /// 210 × 297 mm.
    A4,
    /// 297 × 420 mm.
    A3,
    /// 148 × 210 mm.
    A5,
    /// 215.9 × 279.4 mm (8.5" × 11").
    Letter,
    /// 215.9 × 355.6 mm (8.5" × 14").
    Legal,
    /// 279.4 × 431.8 mm (11" × 17").
    Tabloid,
    /// Arbitrary size in millimetres.
    Custom {
        /// Width in mm.
        width_mm: f32,
        /// Height in mm.
        height_mm: f32,
    },
}

#[allow(clippy::derivable_impls)]
impl Default for PageSize {
    fn default() -> Self {
        Self::A4
    }
}

impl PageSize {
    /// Returns `(width_mm, height_mm)` in **portrait** orientation.
    /// Apply [`Orientation::Landscape`] by swapping the pair.
    pub fn dimensions_mm(self) -> (f32, f32) {
        match self {
            Self::A4 => (210.0, 297.0),
            Self::A3 => (297.0, 420.0),
            Self::A5 => (148.0, 210.0),
            Self::Letter => (215.9, 279.4),
            Self::Legal => (215.9, 355.6),
            Self::Tabloid => (279.4, 431.8),
            Self::Custom {
                width_mm,
                height_mm,
            } => (width_mm, height_mm),
        }
    }
}

/// Page margins in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "axum-router", derive(ToSchema))]
pub struct Margins {
    /// Top margin (mm).
    pub top_mm: f32,
    /// Right margin (mm).
    pub right_mm: f32,
    /// Bottom margin (mm).
    pub bottom_mm: f32,
    /// Left margin (mm).
    pub left_mm: f32,
}

impl Default for Margins {
    /// Sensible printer-safe defaults: 15 mm all round.
    fn default() -> Self {
        Self {
            top_mm: 15.0,
            right_mm: 15.0,
            bottom_mm: 15.0,
            left_mm: 15.0,
        }
    }
}

impl Margins {
    /// Same value on every side.
    pub const fn uniform(mm: f32) -> Self {
        Self {
            top_mm: mm,
            right_mm: mm,
            bottom_mm: mm,
            left_mm: mm,
        }
    }
}

/// Aggregate of every page-level choice a backend needs.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "axum-router", derive(ToSchema))]
pub struct PageOptions {
    /// Page size. Defaults to [`PageSize::A4`].
    #[serde(default)]
    pub size: PageSize,
    /// Page orientation. Defaults to [`Orientation::Portrait`].
    #[serde(default)]
    pub orientation: Orientation,
    /// Page margins. Defaults to 15 mm uniform.
    #[serde(default)]
    pub margins: Margins,
}

impl PageOptions {
    /// Width × height in millimetres **after** applying orientation.
    pub fn dimensions_mm(&self) -> (f32, f32) {
        let (w, h) = self.size.dimensions_mm();
        match self.orientation {
            Orientation::Portrait => (w, h),
            Orientation::Landscape => (h, w),
        }
    }
}
