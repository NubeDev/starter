//! Alert-rule CRUD handlers.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::alert::{AlertRuleDetail, CreateAlertRuleRequest, UpdateAlertRuleRequest};
use nexus_store::alert::{rule, NewRule, RulePatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::rule_to_detail;
use crate::authz::{self, ACTION_DELETE, ACTION_EDIT, ACTION_VIEW, KIND_ALERT_RULE};
use crate::middleware::tenant::{caller, tenant_of};
use crate::state::AppState;

#[utoipa::path(get, path = "/api/v1/alerts/rules", tag = "alerts", operation_id = "list_alert_rules",
    responses((status = 200, description = "Rules", body = [AlertRuleDetail])))]
pub async fn list_rules(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match rule::list(&state.metadata, &tenant).await {
        Ok(rs) => Json(rs.iter().map(rule_to_detail).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/alerts/rules", tag = "alerts", operation_id = "create_alert_rule",
    request_body = CreateAlertRuleRequest,
    responses((status = 200, description = "Created", body = AlertRuleDetail)))]
pub async fn create_rule(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateAlertRuleRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let new = NewRule {
        name: req.name,
        datasource_id: req.datasource_id,
        query: req.query,
        op: req.op,
        threshold: req.threshold,
        for_secs: req.for_secs.unwrap_or(0),
        interval_secs: req.interval_secs.unwrap_or(60),
        enabled: req.enabled.unwrap_or(true),
        channel_ids: req.channel_ids.unwrap_or_default(),
        conditions: super::convert::conditions_to_json(req.conditions),
        combinator: req.combinator.unwrap_or_else(|| "and".to_string()),
        no_data_policy: req.no_data_policy.unwrap_or_else(|| "ok".to_string()),
        exec_error_policy: req.exec_error_policy.unwrap_or_else(|| "ok".to_string()),
        message_template: req.message_template,
    };
    match rule::insert(&state.metadata, &tenant, &new).await {
        Ok(r) => Json(rule_to_detail(&r)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/alerts/rules/{id}", tag = "alerts", operation_id = "get_alert_rule",
    params(("id" = Uuid, Path, description = "Rule id")),
    responses((status = 200, description = "Rule", body = AlertRuleDetail), (status = 404, description = "Not found")))]
pub async fn get_rule(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let r = match rule::get(&state.metadata, &tenant, id).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(state.engine.as_ref(), caller, ACTION_VIEW, KIND_ALERT_RULE, &id.to_string(), &tenant).await {
        return resp;
    }
    Json(rule_to_detail(&r)).into_response()
}

#[utoipa::path(put, path = "/api/v1/alerts/rules/{id}", tag = "alerts", operation_id = "update_alert_rule",
    params(("id" = Uuid, Path, description = "Rule id")), request_body = UpdateAlertRuleRequest,
    responses((status = 204, description = "Updated"), (status = 404, description = "Not found")))]
pub async fn update_rule(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAlertRuleRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(state.engine.as_ref(), caller, ACTION_EDIT, KIND_ALERT_RULE, &id.to_string(), &tenant).await {
        return resp;
    }
    let patch = RulePatch {
        name: req.name,
        query: req.query,
        op: req.op,
        threshold: req.threshold,
        for_secs: req.for_secs,
        interval_secs: req.interval_secs,
        enabled: req.enabled,
        channel_ids: req.channel_ids,
        conditions: super::convert::conditions_to_json(req.conditions),
        combinator: req.combinator,
        no_data_policy: req.no_data_policy,
        exec_error_policy: req.exec_error_policy,
        message_template: req.message_template,
    };
    match rule::update(&state.metadata, &tenant, id, &patch).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/alerts/rules/{id}", tag = "alerts", operation_id = "delete_alert_rule",
    params(("id" = Uuid, Path, description = "Rule id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")))]
pub async fn delete_rule(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(state.engine.as_ref(), caller, ACTION_DELETE, KIND_ALERT_RULE, &id.to_string(), &tenant).await {
        return resp;
    }
    match rule::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
