//! Layout primitives — `page()`, `row()`, `col()`, `grid()`, `tabs()`,
//! `card()`.
//!
//! Each constructor returns a [`Component`] directly (or a small
//! builder for shapes that need optional fields). The container
//! types all accept their children via `.children([...])` so the call
//! shape reads naturally.

use starter_ui_ir::{
    Component, FlexAlign, FlexJustify, NodeStyle, RowBreakpoints, RowLayout, Tab,
};

// =====================================================================
// Page
// =====================================================================

/// Construct the root [`Component::Page`]. Every layout has exactly
/// one page node at its root.
pub fn page(id: impl Into<String>, title: impl Into<String>) -> PageBuilder {
    PageBuilder {
        id: id.into(),
        title: Some(title.into()),
        header_actions: Vec::new(),
        children: Vec::new(),
        default_max_width: None,
    }
}

/// Builder for [`Component::Page`].
#[derive(Debug, Clone)]
pub struct PageBuilder {
    id: String,
    title: Option<String>,
    header_actions: Vec<starter_ui_ir::ToolbarAction>,
    children: Vec<Component>,
    default_max_width: Option<String>,
}

impl PageBuilder {
    /// Set children. Replaces any previously set children.
    pub fn children<I: IntoIterator<Item = Component>>(mut self, kids: I) -> Self {
        self.children = kids.into_iter().collect();
        self
    }

    /// Append a single child.
    pub fn child(mut self, c: Component) -> Self {
        self.children.push(c);
        self
    }

    /// Set the page-level header actions (buttons next to the title).
    pub fn header_actions(mut self, actions: Vec<starter_ui_ir::ToolbarAction>) -> Self {
        self.header_actions = actions;
        self
    }

    /// Override the page's max-width container. Pass any CSS length
    /// (`"100%"`, `"none"`, `"1600px"`, …). When unset the studio
    /// shell defaults to `max-w-7xl` (1280 px).
    pub fn max_width(mut self, value: impl Into<String>) -> Self {
        self.default_max_width = Some(value.into());
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Page {
            id: self.id,
            title: self.title,
            header_actions: self.header_actions,
            children: self.children,
            style: None,
            default_row_gap: None,
            default_column_gap: None,
            default_page_padding: None,
            default_max_width: self.default_max_width,
        }
    }
}

// =====================================================================
// Row / Col / Grid
// =====================================================================

/// Construct a horizontal flex [`Component::Row`].
pub fn row(id: impl Into<String>) -> ContainerBuilder {
    ContainerBuilder::new(ContainerKind::Row, Some(id.into()))
}

/// Construct a vertical flex [`Component::Col`].
pub fn col(id: impl Into<String>) -> ContainerBuilder {
    ContainerBuilder::new(ContainerKind::Col, Some(id.into()))
}

/// Construct a CSS-grid [`Component::Grid`]. Pass `columns()` to set
/// the `grid-template-columns` value.
pub fn grid(id: impl Into<String>) -> ContainerBuilder {
    ContainerBuilder::new(ContainerKind::Grid, Some(id.into()))
}

#[derive(Debug, Clone, Copy, Default)]
enum ContainerKind {
    #[default]
    Row,
    Col,
    Grid,
}

/// Builder shared by [`Component::Row`], [`Component::Col`], and
/// [`Component::Grid`]. Setters that don't apply to the chosen kind
/// are silently dropped at `build()` time — keeps the call site
/// uniform without per-kind builder duplication.
#[derive(Debug, Clone, Default)]
pub struct ContainerBuilder {
    kind: ContainerKind,
    id: Option<String>,
    children: Vec<Component>,
    gap: Option<String>,
    columns: Option<String>,
    stack_below: Option<String>,
    layout: Option<RowLayout>,
    align: Option<FlexAlign>,
    justify: Option<FlexJustify>,
    wrap: Option<bool>,
    span: Option<u8>,
    style: Option<NodeStyle>,
}

impl ContainerBuilder {
    fn new(kind: ContainerKind, id: Option<String>) -> Self {
        Self {
            kind,
            id,
            ..Self::default()
        }
    }

    /// Set children. Replaces any previously set children.
    pub fn children<I: IntoIterator<Item = Component>>(mut self, kids: I) -> Self {
        self.children = kids.into_iter().collect();
        self
    }

    /// Append a single child.
    pub fn child(mut self, c: Component) -> Self {
        self.children.push(c);
        self
    }

    /// CSS gap value (e.g. `"8px"`, `"1rem"`). Honoured by Row / Col.
    pub fn gap(mut self, g: impl Into<String>) -> Self {
        self.gap = Some(g.into());
        self
    }

    /// CSS `grid-template-columns` value. Honoured by Grid only.
    pub fn columns(mut self, cols: impl Into<String>) -> Self {
        self.columns = Some(cols.into());
        self
    }

    /// V1.9 responsive — collapse to a column below this breakpoint
    /// (`"sm"` or `"md"`). Honoured by Row only.
    pub fn stack_below(mut self, bp: impl Into<String>) -> Self {
        self.stack_below = Some(bp.into());
        self
    }

    /// Layout primitive — `RowLayout::Grid` (12-track, default) or
    /// `RowLayout::Auto` (flex-row, content-sized children).
    /// Honoured by Row only.
    pub fn layout(mut self, layout: RowLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Cross-axis alignment of children. Honoured by Row / Col.
    pub fn align(mut self, a: FlexAlign) -> Self {
        self.align = Some(a);
        self
    }

    /// Main-axis distribution of children. Honoured by Row / Col.
    pub fn justify(mut self, j: FlexJustify) -> Self {
        self.justify = Some(j);
        self
    }

    /// When `false`, children stay on a single line. Honoured by Row
    /// only, and only meaningful when `layout` is `Auto` (grid never
    /// wraps).
    pub fn wrap(mut self, on: bool) -> Self {
        self.wrap = Some(on);
        self
    }

    /// 12-grid span when this column is a child of a row. `1..=12`.
    /// Values outside the range are clamped at build time. Honoured
    /// by Col only.
    pub fn span(mut self, span: u8) -> Self {
        self.span = Some(span.clamp(1, 12));
        self
    }

    /// Attach a [`NodeStyle`]. Honoured by all three kinds.
    pub fn style(mut self, s: NodeStyle) -> Self {
        self.style = Some(s);
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        match self.kind {
            ContainerKind::Row => Component::Row {
                id: self.id,
                children: self.children,
                gap: self.gap,
                layout: self.layout,
                breakpoints: self.stack_below.map(|s| RowBreakpoints {
                    stack_below: Some(s),
                }),
                align: self.align,
                justify: self.justify,
                wrap: self.wrap,
                style: self.style,
            },
            ContainerKind::Col => Component::Col {
                id: self.id,
                children: self.children,
                gap: self.gap,
                span: self.span,
                align: self.align,
                justify: self.justify,
                style: self.style,
            },
            ContainerKind::Grid => Component::Grid {
                id: self.id,
                children: self.children,
                columns: self.columns,
                style: self.style,
            },
        }
    }
}

// =====================================================================
// Tabs
// =====================================================================

/// Construct a [`Component::Tabs`] container.
pub fn tabs(id: impl Into<String>) -> TabsBuilder {
    TabsBuilder {
        id: Some(id.into()),
        tabs: Vec::new(),
        lazy: false,
        url_param: None,
    }
}

/// Builder for [`Component::Tabs`].
#[derive(Debug, Clone)]
pub struct TabsBuilder {
    id: Option<String>,
    tabs: Vec<Tab>,
    lazy: bool,
    url_param: Option<String>,
}

impl TabsBuilder {
    /// Add a tab. Tabs render in insertion order.
    pub fn tab(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        children: impl IntoIterator<Item = Component>,
    ) -> Self {
        self.tabs.push(Tab {
            id: Some(id.into()),
            label: label.into(),
            icon: None,
            children: children.into_iter().collect(),
        });
        self
    }

    /// Defer mounting each tab's children until first activation.
    pub fn lazy(mut self) -> Self {
        self.lazy = true;
        self
    }

    /// Mirror the active tab id into the URL query string under
    /// `?<param>=<id>`.
    pub fn url_param(mut self, param: impl Into<String>) -> Self {
        self.url_param = Some(param.into());
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Tabs {
            id: self.id,
            tabs: self.tabs,
            lazy: self.lazy,
            url_param: self.url_param,
            default: None,
        }
    }
}

// =====================================================================
// Card
// =====================================================================

/// Construct a [`Component::Card`] — a titled container. The
/// constructor enforces a title at the type level so the V1.5
/// `lead` / `trailing` slots are always meaningful.
pub fn card(id: impl Into<String>, title: impl Into<String>) -> CardBuilder {
    CardBuilder {
        id: id.into(),
        title: title.into(),
        subtitle: None,
        intent: None,
        lead: None,
        trailing: None,
        body: Vec::new(),
    }
}

/// Builder for [`Component::Card`].
#[derive(Debug, Clone)]
pub struct CardBuilder {
    id: String,
    title: String,
    subtitle: Option<String>,
    intent: Option<String>,
    lead: Option<Box<Component>>,
    trailing: Option<Box<Component>>,
    body: Vec<Component>,
}

impl CardBuilder {
    /// Optional subtitle rendered under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Semantic intent — `"info"` | `"success"` | `"warning"` |
    /// `"danger"`.
    pub fn intent(mut self, intent: impl Into<String>) -> Self {
        self.intent = Some(intent.into());
        self
    }

    /// V1.5 lead slot — a single component (typically an icon or
    /// avatar) rendered to the left of the title.
    pub fn lead(mut self, lead: Component) -> Self {
        self.lead = Some(Box::new(lead));
        self
    }

    /// V1.5 trailing slot — a single component (status badge or
    /// metadata pill) rendered at the top-right of the header.
    pub fn trailing(mut self, trailing: Component) -> Self {
        self.trailing = Some(Box::new(trailing));
        self
    }

    /// Set the body content (the V1.5 `content` slot).
    pub fn children<I: IntoIterator<Item = Component>>(mut self, kids: I) -> Self {
        self.body = kids.into_iter().collect();
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Card {
            id: Some(self.id),
            title: Some(self.title),
            subtitle: self.subtitle,
            intent: self.intent,
            lead: self.lead,
            trailing: self.trailing,
            actions: Vec::new(),
            children: self.body,
            style: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Component {
        Component::Text {
            id: None,
            content: s.into(),
            intent: None,
            style: None,
        }
    }

    #[test]
    fn page_with_children_round_trips() {
        let p = page("p", "Title")
            .children([row("r").child(text("hi")).build()])
            .build();
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["type"], "page");
        assert_eq!(v["title"], "Title");
        assert_eq!(v["children"][0]["type"], "row");
        assert_eq!(v["children"][0]["children"][0]["type"], "text");
    }

    #[test]
    fn row_carries_stack_below_breakpoint() {
        let r = row("r").stack_below("md").child(text("hi")).build();
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["breakpoints"]["stack_below"], "md");
    }

    #[test]
    fn grid_carries_columns() {
        let g = grid("g").columns("1fr 1fr").build();
        let v = serde_json::to_value(&g).unwrap();
        assert_eq!(v["columns"], "1fr 1fr");
    }

    #[test]
    fn tabs_with_two_tabs() {
        let t = tabs("t")
            .tab("a", "Alpha", [text("A")])
            .tab("b", "Beta", [text("B")])
            .build();
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["type"], "tabs");
        assert_eq!(v["tabs"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn card_emits_wire_card_variant() {
        let c = card("c", "Title")
            .subtitle("desc")
            .intent("danger")
            .children([text("body")])
            .build();
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "card");
        assert_eq!(v["title"], "Title");
        assert_eq!(v["subtitle"], "desc");
        assert_eq!(v["intent"], "danger");
    }
}
