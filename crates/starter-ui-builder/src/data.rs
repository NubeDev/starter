//! Data-display primitive — `table()`.
//!
//! `table()` produces a [`starter_ui_ir::Component::Table`] with a
//! typed RSQL source. Live updates default off — call
//! [`TableBuilder::live`] to opt in.

use starter_ui_ir::{
    Action, ColumnRender, Component, RowAction, TableColumn, TableSource, ToolbarAction,
};

use crate::rsql::RsqlBuilder;

/// Construct a [`Component::Table`] with a typed RSQL source. The
/// query is rendered via [`RsqlBuilder::build`]; live updates default
/// off — call [`TableBuilder::live`] to opt in.
pub fn table(id: impl Into<String>, query: RsqlBuilder) -> TableBuilder {
    TableBuilder {
        id: id.into(),
        query: query.build(),
        subscribe: None,
        columns: Vec::new(),
        row_actions: Vec::new(),
        toolbar_actions: Vec::new(),
        page_size: None,
        searchable: false,
        row_action: None,
    }
}

/// Builder for [`Component::Table`].
#[derive(Debug, Clone)]
pub struct TableBuilder {
    id: String,
    query: String,
    subscribe: Option<bool>,
    columns: Vec<TableColumn>,
    row_actions: Vec<RowAction>,
    toolbar_actions: Vec<ToolbarAction>,
    page_size: Option<u32>,
    searchable: bool,
    row_action: Option<Action>,
}

impl TableBuilder {
    /// Subscribe the table's row set to live updates. Server emits a
    /// `SubscriptionPlan` keyed on the table's id.
    pub fn live(mut self) -> Self {
        self.subscribe = Some(true);
        self
    }

    /// Add a sortable column.
    pub fn column(mut self, title: impl Into<String>, field: impl Into<String>) -> Self {
        self.columns.push(TableColumn {
            title: title.into(),
            field: field.into(),
            sortable: Some(true),
            render: None,
        });
        self
    }

    /// Add a sortable column with a cell-level render hint.
    pub fn column_render(
        mut self,
        title: impl Into<String>,
        field: impl Into<String>,
        render: ColumnRender,
    ) -> Self {
        self.columns.push(TableColumn {
            title: title.into(),
            field: field.into(),
            sortable: Some(true),
            render: Some(render),
        });
        self
    }

    /// Cap the per-page row count.
    pub fn page_size(mut self, n: u32) -> Self {
        self.page_size = Some(n);
        self
    }

    /// Enable the search bar above the table.
    pub fn searchable(mut self) -> Self {
        self.searchable = true;
        self
    }

    /// Append a per-row action button.
    pub fn row_action(mut self, action: RowAction) -> Self {
        self.row_actions.push(action);
        self
    }

    /// Append a page-level button rendered above the table.
    pub fn toolbar_action(mut self, action: ToolbarAction) -> Self {
        self.toolbar_actions.push(action);
        self
    }

    /// Whole-row click action — fires when the user clicks anywhere
    /// on a row that isn't a `row_action` button.
    pub fn on_row_click(mut self, action: Action) -> Self {
        self.row_action = Some(action);
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Table {
            id: Some(self.id),
            source: TableSource {
                query: self.query,
                subscribe: self.subscribe,
            },
            columns: self.columns,
            row_action: self.row_action,
            row_actions: self.row_actions,
            toolbar_actions: self.toolbar_actions,
            page_size: self.page_size,
            searchable: self.searchable,
            style: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rsql::rsql;

    #[test]
    fn table_with_columns_emits_wire_shape() {
        let t = table("t", rsql().kind("com.acme.task"))
            .live()
            .column("Path", "path")
            .column("Gate", "slots.settings.gate")
            .build();
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["type"], "table");
        assert_eq!(v["source"]["query"], "kind==com.acme.task");
        assert_eq!(v["source"]["subscribe"], true);
        assert_eq!(v["columns"].as_array().unwrap().len(), 2);
    }
}
