//! Proc-macros for `starter-ext-sdk`.
//!
//! Per SCOPE.md **R3** ("Dispatch is manifest-driven"): the extension's
//! `block.yaml` is the single source of truth for what the extension
//! provides. `#[derive(Extension)]` reads that file at the extension's
//! compile time, validates it against the workspace's `Manifest` schema, and
//! emits:
//!
//! - An impl of `starter_ext_sdk::ExtensionMeta` exposing `id()`, `version()`,
//!   `manifest_yaml()` (the raw bundled bytes — never templated, R7), and
//!   `manifest_static()` (lazy-parsed `Manifest`).
//! - A per-extension `<Struct>ToolHandlers` trait whose method set is
//!   determined by the manifest's `contributes.tools` list. The extension
//!   must implement this trait.
//!   - Missing handler ⇒ compile error ("not all trait items implemented").
//!   - Extra handler in the impl ⇒ compile error ("`foo` is not a member of
//!     trait …").
//!
//!   That is the **R3** "compile error in the extension, not a runtime error
//!   in the host" guarantee.
//!
//! Usage:
//!
//! ```ignore
//! use starter_ext_sdk::Extension;
//!
//! #[derive(Extension)]
//! #[extension(manifest = "block.yaml")] // path relative to CARGO_MANIFEST_DIR
//! pub struct Weather;
//! ```
//!
//! The proc-macro deliberately does *not* keep its own copy of the manifest
//! schema. It deserialises into [`starter_ext_spi::Manifest`] so any field
//! the host validates is also validated here.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::path::PathBuf;
use syn::{parse_macro_input, DeriveInput, LitStr};

use starter_ext_spi::Manifest;

/// `#[derive(Extension)]` — reads `block.yaml` at compile time and emits
/// the per-extension metadata + handler-trait scaffolding.
///
/// Accepts one optional attribute: `#[extension(manifest = "<path>")]`. The
/// path is interpreted relative to `CARGO_MANIFEST_DIR` (the extension
/// crate's root). Default is `block.yaml`.
#[proc_macro_derive(Extension, attributes(extension))]
pub fn derive_extension(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let struct_name = input.ident.clone();
    let manifest_rel = extract_manifest_path(&input)?;

    // Resolve against CARGO_MANIFEST_DIR. We do *not* fall back to the
    // current working directory: `cargo build` sets CARGO_MANIFEST_DIR for
    // every invocation (build, check, rustdoc), so an unset variable is a
    // real misconfiguration, not a normal build.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        syn::Error::new_spanned(
            &struct_name,
            "CARGO_MANIFEST_DIR not set; #[derive(Extension)] must be invoked from a cargo build",
        )
    })?;
    let manifest_abs = PathBuf::from(&manifest_dir).join(&manifest_rel);

    let yaml_str = std::fs::read_to_string(&manifest_abs).map_err(|e| {
        syn::Error::new_spanned(
            &struct_name,
            format!("failed to read manifest {}: {}", manifest_abs.display(), e),
        )
    })?;

    // Parse against the *shared* Manifest type. This is the same validator
    // the host runs at load time — keeping it identical is R3.
    let manifest: Manifest = serde_yaml::from_str(&yaml_str).map_err(|e| {
        syn::Error::new_spanned(
            &struct_name,
            format!("manifest {} failed to parse: {}", manifest_abs.display(), e),
        )
    })?;

    // Per R4: every contributed tool id must be `id` or a dotted descendant.
    // Catching it at the extension's build (not at host load) is the
    // earliest possible signal.
    for t in &manifest.contributes.tools {
        if !manifest.id.owns(&t.id) {
            return Err(syn::Error::new_spanned(
                &struct_name,
                format!(
                    "tool id {:?} escapes the extension's namespace {:?} (SCOPE R4)",
                    t.id,
                    manifest.id.as_str()
                ),
            ));
        }
    }

    let id_str = manifest.id.as_str().to_string();
    let version_str = manifest.version.to_string();
    let yaml_lit = LitStr::new(&yaml_str, proc_macro2::Span::call_site());
    let manifest_rel_lit = LitStr::new(&manifest_rel, proc_macro2::Span::call_site());

    let handlers_trait = format_ident!("{}ToolHandlers", struct_name);

    let tool_id_lits: Vec<LitStr> = manifest
        .contributes
        .tools
        .iter()
        .map(|t| LitStr::new(&t.id, proc_macro2::Span::call_site()))
        .collect();
    let tool_methods: Vec<proc_macro2::Ident> = manifest
        .contributes
        .tools
        .iter()
        .map(|t| handler_ident(&t.id))
        .collect();

    let trait_method_defs = tool_methods
        .iter()
        .zip(tool_id_lits.iter())
        .map(|(m, id_lit)| {
            quote! {
                #[doc = concat!("Handler for tool `", #id_lit, "`. Declared in `block.yaml`.")]
                fn #m(
                    &self,
                    ctx: &Self::Ctx,
                    params: ::starter_ext_sdk::serde_json::Value,
                ) -> ::starter_ext_sdk::Result<::starter_ext_sdk::serde_json::Value>;
            }
        });

    let dispatch_arms = tool_methods
        .iter()
        .zip(tool_id_lits.iter())
        .map(|(m, id_lit)| {
            quote! {
                #id_lit => <Self as #handlers_trait>::#m(self, ctx, params),
            }
        });

    Ok(quote! {
        // Tell rustc that a change to `block.yaml` invalidates this crate.
        // `include_bytes!` is the stable workaround for "track an external
        // file" (the unstable `proc_macro::tracked_path` is the future fix).
        //
        // The path is rooted at the extension's CARGO_MANIFEST_DIR — same
        // anchor the proc-macro itself used to read the file — so the
        // resolution is independent of the *.rs file the derive macro was
        // invoked from (test files, build scripts, examples all work).
        const _: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #manifest_rel_lit));

        impl ::starter_ext_sdk::ExtensionMeta for #struct_name {
            fn id() -> &'static ::starter_ext_sdk::ExtensionId {
                static ID: ::std::sync::OnceLock<::starter_ext_sdk::ExtensionId> =
                    ::std::sync::OnceLock::new();
                ID.get_or_init(|| {
                    ::starter_ext_sdk::ExtensionId::new(#id_str)
                        .expect("derive(Extension) validated id at compile time")
                })
            }

            fn version() -> &'static ::starter_ext_sdk::semver::Version {
                static V: ::std::sync::OnceLock<::starter_ext_sdk::semver::Version> =
                    ::std::sync::OnceLock::new();
                V.get_or_init(|| {
                    #version_str
                        .parse()
                        .expect("derive(Extension) validated version at compile time")
                })
            }

            fn manifest_yaml() -> &'static str {
                #yaml_lit
            }

            fn manifest_static() -> &'static ::starter_ext_sdk::Manifest {
                static M: ::std::sync::OnceLock<::starter_ext_sdk::Manifest> =
                    ::std::sync::OnceLock::new();
                M.get_or_init(|| {
                    ::starter_ext_sdk::serde_yaml::from_str(<#struct_name as ::starter_ext_sdk::ExtensionMeta>::manifest_yaml())
                        .expect("derive(Extension) validated manifest at compile time")
                })
            }
        }

        #[doc = concat!(
            "Tool-handler trait generated by `#[derive(Extension)]` for `",
            stringify!(#struct_name),
            "`. One method per `contributes.tools` entry in `block.yaml`. ",
            "Missing or extra handler is a compile error (SCOPE R3)."
        )]
        pub trait #handlers_trait {
            /// Per-extension `Ctx` newtype emitted by `requires!{}`. Bound
            /// here (rather than on `ExtensionBehavior`) so the trait the
            /// proc-macro generates is self-contained.
            type Ctx;

            #( #trait_method_defs )*
        }

        impl ::starter_ext_sdk::ExtensionDispatch for #struct_name
        where
            #struct_name: #handlers_trait,
        {
            type Ctx = <Self as #handlers_trait>::Ctx;

            fn declared_tool_ids() -> &'static [&'static str] {
                &[ #( #tool_id_lits ),* ]
            }

            fn dispatch_tool(
                &self,
                tool_id: &str,
                ctx: &Self::Ctx,
                params: ::starter_ext_sdk::serde_json::Value,
            ) -> ::starter_ext_sdk::Result<::starter_ext_sdk::serde_json::Value> {
                match tool_id {
                    #( #dispatch_arms )*
                    other => Err(::starter_ext_sdk::Error::validation(format!(
                        "tool {:?} not declared in manifest",
                        other
                    ))),
                }
            }
        }
    })
}

/// Sanitise a manifest-declared id into a Rust identifier safe for use as a
/// trait method name. `com.acme.weather.current` → `handle_com_acme_weather_current`.
fn handler_ident(tool_id: &str) -> proc_macro2::Ident {
    let safe: String = tool_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format_ident!("handle_{}", safe)
}

/// Extract `#[extension(manifest = "…")]`. Defaults to `block.yaml`.
fn extract_manifest_path(input: &DeriveInput) -> syn::Result<String> {
    for attr in &input.attrs {
        if !attr.path().is_ident("extension") {
            continue;
        }
        let mut found: Option<String> = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("manifest") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                found = Some(lit.value());
                Ok(())
            } else {
                Err(meta.error("unknown #[extension(...)] key"))
            }
        })?;
        if let Some(p) = found {
            return Ok(p);
        }
    }
    Ok("block.yaml".to_string())
}
