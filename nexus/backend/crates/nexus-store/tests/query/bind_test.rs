//! The C2 binder, proven pure (no database): macros and variables become bound
//! `$N` args, the only text inserted is a vetted identifier, and the injection +
//! tenant-isolation guarantees hold by construction.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use nexus_store::{bind, BindCtx, BindError, HostTokens, ScalarValue, SqlValue, TimeRange, VarValue};

fn range() -> TimeRange {
    TimeRange {
        from: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        to: Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap(),
    }
}

#[test]
fn raw_sql_passes_through_with_no_args() {
    let bound = bind("SELECT 1 FROM t WHERE x = 5", &BindCtx::default()).expect("bind");
    assert_eq!(bound.sql, "SELECT 1 FROM t WHERE x = 5");
    assert!(bound.args.is_empty());
    assert!(bound.validated_identifiers.is_empty());
}

#[test]
fn time_filter_binds_both_bounds_and_validates_the_column() {
    let ctx = BindCtx {
        time_range: Some(range()),
        ..Default::default()
    };
    let bound = bind("SELECT * FROM t WHERE $__timeFilter(ts)", &ctx).expect("bind");
    assert_eq!(bound.sql, "SELECT * FROM t WHERE ts >= $1 AND ts < $2");
    assert_eq!(bound.args.len(), 2);
    assert!(matches!(bound.args[0], SqlValue::Timestamp(_)));
    assert!(matches!(bound.args[1], SqlValue::Timestamp(_)));
    // The column is the only inserted text, and it was vetted on each insertion
    // (the macro references it twice, once per bound).
    assert_eq!(
        bound.validated_identifiers,
        vec!["ts".to_string(), "ts".to_string()]
    );
}

#[test]
fn time_filter_without_a_range_is_rejected() {
    let err = bind("WHERE $__timeFilter(ts)", &BindCtx::default()).unwrap_err();
    assert!(matches!(err, BindError::MissingContext { .. }));
}

#[test]
fn time_group_renders_the_postgres_bucket() {
    let ctx = BindCtx {
        interval: Some(Duration::from_secs(300)),
        ..Default::default()
    };
    let bound = bind("SELECT $__timeGroup(ts, $__interval) FROM t", &ctx).expect("bind");
    assert!(bound.sql.contains("date_bin('300 seconds', ts, TIMESTAMPTZ 'epoch')"));
    assert!(bound.args.is_empty());
}

#[test]
fn time_group_accepts_a_literal_interval() {
    let bound = bind("SELECT $__timeGroup(ts, '1h') FROM t", &BindCtx::default()).expect("bind");
    assert!(bound.sql.contains("date_bin('3600 seconds', ts, TIMESTAMPTZ 'epoch')"));
}

#[test]
fn time_from_and_to_bind_single_instants() {
    let ctx = BindCtx {
        time_range: Some(range()),
        ..Default::default()
    };
    let bound = bind("SELECT * FROM t WHERE a > $__timeFrom AND a < $__timeTo", &ctx).expect("bind");
    assert_eq!(bound.sql, "SELECT * FROM t WHERE a > $1 AND a < $2");
    assert_eq!(bound.args.len(), 2);
}

#[test]
fn single_variable_binds_one_placeholder() {
    let mut variables = BTreeMap::new();
    variables.insert(
        "region".to_string(),
        VarValue::Single(ScalarValue::Text("Site-A".to_string())),
    );
    let ctx = BindCtx {
        variables,
        ..Default::default()
    };
    let bound = bind("SELECT * FROM t WHERE region = $region", &ctx).expect("bind");
    assert_eq!(bound.sql, "SELECT * FROM t WHERE region = $1");
    assert_eq!(bound.args, vec![SqlValue::Text("Site-A".to_string())]);
}

#[test]
fn sql_in_expands_a_multi_value_to_a_bound_list() {
    let mut variables = BTreeMap::new();
    variables.insert(
        "region".to_string(),
        VarValue::Multi(vec![
            ScalarValue::Text("a".to_string()),
            ScalarValue::Text("b".to_string()),
        ]),
    );
    let ctx = BindCtx {
        variables,
        ..Default::default()
    };
    let bound = bind("SELECT * FROM t WHERE region IN $__sqlIn(region)", &ctx).expect("bind");
    assert_eq!(bound.sql, "SELECT * FROM t WHERE region IN ($1, $2)");
    assert_eq!(bound.args.len(), 2);
}

#[test]
fn empty_multi_select_stays_inert() {
    let mut variables = BTreeMap::new();
    variables.insert("region".to_string(), VarValue::Multi(vec![]));
    let ctx = BindCtx {
        variables,
        ..Default::default()
    };
    let bound = bind("SELECT * FROM t WHERE region IN $__sqlIn(region)", &ctx).expect("bind");
    // A well-formed, always-false guard rather than a syntax error.
    assert_eq!(bound.sql, "SELECT * FROM t WHERE region IN ($1)");
    assert_eq!(bound.args, vec![SqlValue::Null]);
}

#[test]
fn malicious_variable_value_lands_as_an_inert_bound_arg() {
    let mut variables = BTreeMap::new();
    let attack = "'); DROP TABLE users; --".to_string();
    variables.insert(
        "region".to_string(),
        VarValue::Single(ScalarValue::Text(attack.clone())),
    );
    let ctx = BindCtx {
        variables,
        ..Default::default()
    };
    let bound = bind("SELECT * FROM t WHERE region = $region", &ctx).expect("bind");
    // The attack string never appears in the SQL text — it is a bound value.
    assert_eq!(bound.sql, "SELECT * FROM t WHERE region = $1");
    assert!(!bound.sql.contains("DROP"));
    assert_eq!(bound.args, vec![SqlValue::Text(attack)]);
}

#[test]
fn a_bad_identifier_in_a_macro_is_rejected() {
    let ctx = BindCtx {
        time_range: Some(range()),
        ..Default::default()
    };
    // A non-identifier inside the macro's column slot — the one text path — is
    // rejected by the allowlist before any text is inserted.
    let err = bind("WHERE $__timeFilter(ts); DROP TABLE x)", &ctx);
    // The macro arg `ts` is valid (the trailing `; DROP …` is the author's own
    // raw SQL, the existing read-only-guarded path). An *injected column* is
    // what the allowlist must reject:
    assert!(err.is_ok());
    let err = bind("WHERE $__timeFilter(ts WHERE 1=1)", &ctx).unwrap_err();
    assert!(matches!(err, BindError::InvalidIdentifier(_)));
}

#[test]
fn host_tokens_bind_from_context_never_from_input() {
    let ctx = BindCtx {
        host_tokens: HostTokens {
            caller_tenant_id: Some("tenant-7".to_string()),
            caller_user_id: Some("user-9".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let bound = bind(
        "SELECT * FROM t WHERE tenant = $caller_tenant_id AND owner = $caller_user_id",
        &ctx,
    )
    .expect("bind");
    assert_eq!(
        bound.sql,
        "SELECT * FROM t WHERE tenant = $1 AND owner = $2"
    );
    assert_eq!(
        bound.args,
        vec![
            SqlValue::Text("tenant-7".to_string()),
            SqlValue::Text("user-9".to_string()),
        ]
    );
}

#[test]
fn a_caller_supplied_host_token_value_is_rejected() {
    let mut variables = BTreeMap::new();
    variables.insert(
        "region".to_string(),
        VarValue::Single(ScalarValue::Text("$caller_tenant_id".to_string())),
    );
    let ctx = BindCtx {
        variables,
        ..Default::default()
    };
    let err = bind("WHERE region = $region", &ctx).unwrap_err();
    assert!(matches!(err, BindError::HostTokenInInput(_)));
}

#[test]
fn a_missing_host_token_is_rejected_not_silently_skipped() {
    let err = bind("WHERE tenant = $caller_tenant_id", &BindCtx::default()).unwrap_err();
    assert!(matches!(err, BindError::MissingContext { .. }));
}

#[test]
fn kind_param_binds_as_an_argument() {
    let mut params = BTreeMap::new();
    params.insert("limit".to_string(), ScalarValue::Int(50));
    let ctx = BindCtx {
        params,
        ..Default::default()
    };
    let bound = bind("SELECT * FROM t LIMIT $limit", &ctx).expect("bind");
    assert_eq!(bound.sql, "SELECT * FROM t LIMIT $1");
    assert_eq!(bound.args, vec![SqlValue::Int(50)]);
}

#[test]
fn an_undefined_variable_is_rejected() {
    let err = bind("WHERE x = $nope", &BindCtx::default()).unwrap_err();
    assert!(matches!(err, BindError::UndefinedVariable(_)));
}

#[test]
fn a_literal_dollar_placeholder_passes_through() {
    // `$$`-style dollar quoting / a bare `$` with no identifier is verbatim.
    let bound = bind("SELECT '$' || x FROM t", &BindCtx::default()).expect("bind");
    assert_eq!(bound.sql, "SELECT '$' || x FROM t");
    assert!(bound.args.is_empty());
}

// ---------------------------------------------------------------------------
// Comment / string-literal / dollar-quote awareness (WS-14 follow-on fix).
//
// A `$token` inside a comment, a `'...'` literal, or a `$tag$...$tag$` body is
// plain text. Without the skip, a kind whose documentation header merely
// MENTIONS `$caller_tenant_id` demands the host token at bind time (the
// shipped `com.nexus.hello.ping` kind is exactly that shape), and a literal
// like `'price in $USD'` 4xxes as an undefined variable.

#[test]
fn token_in_line_comment_is_plain_text() {
    let sql = "-- guarded by $caller_tenant_id elsewhere\nSELECT 1";
    let bound = bind(sql, &BindCtx::default()).expect("a commented token binds nothing");
    assert_eq!(bound.sql, sql);
    assert!(bound.args.is_empty());
}

#[test]
fn token_in_block_comment_is_plain_text_including_nested() {
    let sql = "/* outer /* $caller_tenant_id */ still comment $region */ SELECT 1";
    let bound = bind(sql, &BindCtx::default()).expect("block-comment tokens bind nothing");
    assert_eq!(bound.sql, sql);
    assert!(bound.args.is_empty());
}

#[test]
fn token_in_string_literal_is_plain_text() {
    let sql = "SELECT 'price in $USD', 'it''s $5' AS label";
    let bound = bind(sql, &BindCtx::default()).expect("string-literal tokens bind nothing");
    assert_eq!(bound.sql, sql);
    assert!(bound.args.is_empty());
}

#[test]
fn dollar_quoted_body_is_plain_text() {
    let sql = "SELECT $tag$ has $vars and $__timeFrom inside $tag$, $$ also $x $$";
    let bound = bind(sql, &BindCtx::default()).expect("dollar-quoted tokens bind nothing");
    assert_eq!(bound.sql, sql);
    assert!(bound.args.is_empty());
}

#[test]
fn tokens_after_a_comment_still_expand() {
    let ctx = BindCtx {
        host_tokens: HostTokens {
            caller_tenant_id: Some("t1".into()),
            caller_user_id: None,
        },
        ..Default::default()
    };
    let sql = "-- $caller_tenant_id is host-bound\nSELECT * FROM t WHERE tenant_id = $caller_tenant_id";
    let bound = bind(sql, &ctx).expect("bind");
    assert!(bound.sql.ends_with("WHERE tenant_id = $1"));
    assert!(bound.sql.starts_with("-- $caller_tenant_id is host-bound"));
    assert_eq!(bound.args.len(), 1);
}
