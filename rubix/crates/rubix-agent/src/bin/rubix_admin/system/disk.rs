//! `rubix-admin system disk [--json]` — in-process disk probe.
//!
//! Contract: this CLI is an in-process consumer of the same
//! `probe()` the REST handler calls — it MUST NOT open a TCP
//! connection back to a running rubix-agent. The shared seam is
//! [`rubix_tools::system::disk::probe`]; both surfaces flow
//! through it.
//!
//! Rendering follows the CLI rule from
//! [docs/design/i18n-prefs/](../../../../../docs/design/i18n-prefs/README.md):
//! pick the catalogue from `$LANG`, render the `Diagnostic`
//! server-side via [`MessageBundle::render_diagnostic`], and print.
//! `--json` skips the render and dumps the raw `Diagnostic` +
//! structured data so the output can be piped into `jq` or stored.

use anyhow::Result;
use clap::Args as ClapArgs;
use rubix_agent::boot::mcp::prefs_from_locale;
use rubix_spi::dto::system::disk::DiskUsageRequest;
use rubix_tools::system::disk::probe;
use starter_i18n::bundle::MessageBundle;
use starter_spi::i18n::LanguageTag;

/// CLI flags for `system disk`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Filesystem mount point to probe; defaults to the CLI's CWD
    /// disk (same default the REST handler uses, by passing through
    /// to [`rubix_tools::system::disk::probe`]).
    #[arg(long)]
    mount: Option<String>,
    /// Dump the raw `Diagnostic` (`{ summary: { code, params },
    /// ...DiskUsageResponse fields }`) as JSON instead of rendering
    /// the human-readable string. Stable shape for piping into `jq`.
    #[arg(long)]
    json: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let req = DiskUsageRequest { mount: args.mount };
    let response = probe(req)?;

    if args.json {
        // `DiskUsageResponse` serialises as
        // `{ "summary": { "code": "...", "params": { ... } },
        //   "mount": ..., "total_bytes": ..., ... }` — i.e. the
        // structured Diagnostic is already nested under `summary`.
        // Piping through `to_value` keeps the field order stable.
        let payload = serde_json::to_value(&response)?;
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let bundle = rubix_spi::i18n::rubix_bundle()?;
    let lang = host_language_tag();
    let prefs = prefs_from_locale(&lang);
    let rendered = render(&bundle, &lang, &response.summary, &prefs);
    println!("{rendered}");
    Ok(())
}

fn render(
    bundle: &MessageBundle,
    lang: &LanguageTag,
    diag: &starter_spi::i18n::Diagnostic,
    prefs: &starter_spi::preferences::ResolvedPreferences,
) -> String {
    bundle.render_diagnostic(lang, diag, prefs)
}

/// Pick a BCP-47 [`LanguageTag`] from `$LANG` (POSIX form:
/// `lang_REGION.codeset`). Falls back to `"en"` so the CLI always
/// renders *something* even on a stripped-down host. Anything
/// the parser rejects also falls through to `"en"`.
///
/// Exposed for tests; not part of any public contract.
pub fn host_language_tag() -> LanguageTag {
    let raw = std::env::var("LANG").unwrap_or_default();
    let trimmed = raw.split('.').next().unwrap_or("").replace('_', "-");
    if trimmed.is_empty() {
        return LanguageTag::parse("en").expect("'en' parses");
    }
    LanguageTag::parse(&trimmed)
        .or_else(|_| LanguageTag::parse(trimmed.split('-').next().unwrap_or("en")))
        .unwrap_or_else(|_| LanguageTag::parse("en").expect("'en' parses"))
}

// Invariant: the CLI must NOT open a TCP socket to the agent's
// REST port. It calls probe() in-process. The companion grep guard
// in tests/cli_disk_test.rs scans this file for HTTP-client crate
// names and fails on regression.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_language_tag_parses_posix_lang() {
        // SAFETY: tests in this module are not `#[parallel]`-marked
        // and set_var/remove_var on LANG don't race other suites
        // because each integration-test binary runs in its own
        // process under cargo.
        unsafe {
            std::env::set_var("LANG", "es_AR.UTF-8");
        }
        let tag = host_language_tag();
        assert_eq!(tag.as_str(), "es-AR");
    }

    #[test]
    fn host_language_tag_falls_back_to_en_when_unset() {
        unsafe {
            std::env::remove_var("LANG");
        }
        let tag = host_language_tag();
        assert_eq!(tag.as_str(), "en");
    }
}
