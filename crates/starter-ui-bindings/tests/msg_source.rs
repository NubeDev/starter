//! G6/C7 — `$msg.<key>` binding source. An in-memory `MessageBag`
//! returns per-locale strings; `{{$msg.greeting}}` substitutes
//! correctly per `EvalContext.locale`.

use std::collections::HashMap;

use serde_json::{json, Value as JsonValue};

use starter_ui_bindings::{substitute_text, EvalContext, MessageBag, NullGraph};

struct StaticBag {
    en: HashMap<String, JsonValue>,
    es: HashMap<String, JsonValue>,
}

impl MessageBag for StaticBag {
    fn lookup(&self, key: &str, locale: &str) -> Option<JsonValue> {
        let bag = match locale {
            "es" => &self.es,
            _ => &self.en,
        };
        bag.get(key).cloned()
    }
}

fn make_bag() -> StaticBag {
    let mut en = HashMap::new();
    en.insert("greeting".into(), json!("hello"));
    let mut es = HashMap::new();
    es.insert("greeting".into(), json!("hola"));
    StaticBag { en, es }
}

#[test]
fn msg_substitutes_per_locale() {
    let g = NullGraph;
    let stack = HashMap::new();
    let user = serde_json::Map::new();
    let page = serde_json::Map::new();
    let bag = make_bag();

    let en_ctx = EvalContext {
        graph: &g,
        target: None,
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: None,
        item: None,
        index: None,
        catalogue: &bag,
        locale: "en",
    };
    assert_eq!(
        substitute_text("{{$msg.greeting}}!", &en_ctx).unwrap(),
        "hello!"
    );

    let es_ctx = EvalContext {
        graph: &g,
        target: None,
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: None,
        item: None,
        index: None,
        catalogue: &bag,
        locale: "es",
    };
    assert_eq!(
        substitute_text("{{$msg.greeting}}!", &es_ctx).unwrap(),
        "hola!"
    );
}

#[test]
fn missing_msg_key_errors_unless_optional() {
    let g = NullGraph;
    let stack = HashMap::new();
    let user = serde_json::Map::new();
    let page = serde_json::Map::new();
    let bag = make_bag();
    let ctx = EvalContext {
        graph: &g,
        target: None,
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: None,
        item: None,
        index: None,
        catalogue: &bag,
        locale: "en",
    };
    assert!(substitute_text("{{$msg.unknown}}", &ctx).is_err());
    // Optional collapses to empty.
    assert_eq!(substitute_text("x={{$msg.unknown?}}", &ctx).unwrap(), "x=");
}
