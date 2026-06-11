//! Per-call resource caps and capability-grant set.
//!
//! SCOPE.md **R8**:
//!
//! > The WASM host (`starter-ext-wasm`) starts every component with **no
//! > WASI capabilities**. The manifest's `capabilities:` block lists
//! > explicit grants. Capability *categories* are typed (`http_out:
//! > ["api.example.com"]`, not `fs: "/some/path"`). Per-call fuel,
//! > memory, and wall-clock caps are configured host-side, not in the
//! > manifest.
//!
//! Two halves to that rule:
//!
//! - **Categories** of WASI imports the host links into the linker. The
//!   set comes from the manifest's `capabilities:` list (translated by
//!   [`WasiCategory::from_manifest`] below). Default = empty set =
//!   nothing linked.
//! - **Caps** the host enforces around every dispatch call:
//!   - **Fuel**: hard upper bound on instructions executed per call.
//!     Translated into wasmtime's fuel mechanism on the per-call
//!     `Store`.
//!   - **Memory**: hard upper bound on each linear memory's bytes.
//!     Enforced through a [`crate::MemoryLimiter`] installed on the
//!     `Store`.
//!   - **Wall-clock deadline**: max real time the call may consume.
//!     Implemented via wasmtime's epoch interruption — a background
//!     task ticks the engine's epoch every `tick_ms`; the per-call
//!     store sets its deadline `(deadline_ms / tick_ms) + 1` epochs out
//!     and traps when reached.
//!
//! All three are mandatory in v0.1. There is no "unlimited" mode —
//! "unlimited" is a footgun for a substrate whose isolation story is
//! "the host trusts wasmtime to enforce limits", and a buggy host that
//! handed a component infinite fuel would expose a denial-of-service
//! surface that no manifest could close.

use std::time::Duration;

use starter_ext_spi::Capability;

/// Per-call resource caps applied to every [`crate::WasmHost::dispatch_tool`].
///
/// Cheap to clone; the WasmHost holds one and clones into each `Store`.
#[derive(Debug, Clone)]
pub struct Caps {
    /// Maximum number of wasmtime "fuel units" the call may consume
    /// before wasmtime traps with `Trap::OutOfFuel`. The unit is
    /// roughly "one wasm instruction", but the cost model is up to
    /// wasmtime — operators tune by measurement, not by spec.
    pub max_fuel: u64,

    /// Maximum bytes a single linear memory in the call's component may
    /// grow to. Enforced by [`crate::MemoryLimiter`]. Multi-memory
    /// components see the cap applied per-memory (matching wasmtime's
    /// `StoreLimitsBuilder::memory_size` semantics).
    pub max_memory_bytes: usize,

    /// Wall-clock deadline. The host installs an epoch deadline on the
    /// per-call `Store` and traps the call when the deadline expires.
    /// Setting this to `Duration::ZERO` is rejected at construction
    /// time — see [`Caps::new`].
    pub wall_clock_deadline: Duration,

    /// Set of WASI capability categories the manifest's `capabilities:`
    /// list granted. The linker only wires WASI interfaces for
    /// categories in this set; everything else stays unimported and a
    /// component that depended on it fails instantiation.
    ///
    /// Empty by default — SCOPE R8 "default-deny".
    pub wasi_categories: WasiCategorySet,
}

impl Caps {
    /// Build a typed [`Caps`] from the host-facing knobs.
    ///
    /// Returns `None` on a value that would silently disable
    /// enforcement (`max_fuel = 0`, `max_memory_bytes = 0`,
    /// `wall_clock_deadline = ZERO`). Callers must supply real values —
    /// silently defaulting would let a host think isolation is on when
    /// every cap is no-op.
    pub fn new(
        max_fuel: u64,
        max_memory_bytes: usize,
        wall_clock_deadline: Duration,
    ) -> Option<Self> {
        if max_fuel == 0 || max_memory_bytes == 0 || wall_clock_deadline.is_zero() {
            return None;
        }
        Some(Self {
            max_fuel,
            max_memory_bytes,
            wall_clock_deadline,
            wasi_categories: WasiCategorySet::default(),
        })
    }

    /// Replace the WASI-category set. Returns `self` for builder-style
    /// chaining.
    pub fn with_wasi_categories(mut self, set: WasiCategorySet) -> Self {
        self.wasi_categories = set;
        self
    }
}

/// One typed WASI capability category. Stable; adding a variant is
/// additive within the workspace minor.
///
/// The set of categories the manifest can grant is the same set
/// `starter-ext-supervisor`'s `CapabilityGate` uses (see
/// `CAPABILITY_HOST_METHODS` over there). Keeping the names aligned is
/// SCOPE R1's "identical method signatures across flavours" — an author
/// who declares `requires!(http_out)` gets the same category name on
/// builtin, process, and wasm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WasiCategory {
    /// Outbound HTTP. Links `wasi:http/outgoing-handler`.
    HttpOut,
    /// Filesystem access. Links `wasi:filesystem/types` +
    /// `wasi:filesystem/preopens` (the preopened directories list is
    /// host-side configuration).
    Fs,
    /// Wall-clock + monotonic time. Links `wasi:clocks/wall-clock` +
    /// `wasi:clocks/monotonic-clock`.
    WallClock,
}

impl WasiCategory {
    /// All known categories. Used by the linker setup to iterate the
    /// granted set without hard-coding each variant.
    pub const ALL: &'static [WasiCategory] = &[
        WasiCategory::HttpOut,
        WasiCategory::Fs,
        WasiCategory::WallClock,
    ];

    /// Stable string name matching the manifest's `capabilities:`
    /// category. Surface for tracing.
    pub fn as_str(self) -> &'static str {
        match self {
            WasiCategory::HttpOut => "http_out",
            WasiCategory::Fs => "fs",
            WasiCategory::WallClock => "wall_clock",
        }
    }

    /// Translate from the manifest's [`Capability`] enum to a category.
    /// `None` for [`Capability::Custom`] and [`Capability::Secrets`] —
    /// neither has a public WASI binding the wasmtime linker speaks in
    /// v0.1. The reserved import names live in the WIT package (`kv`
    /// for v0.2, the streaming notifications today) but their host-side
    /// implementations are not wired through the `WasiCategorySet`.
    pub fn from_capability(c: &Capability) -> Option<Self> {
        match c {
            Capability::HttpOut { .. } => Some(WasiCategory::HttpOut),
            Capability::Fs { .. } => Some(WasiCategory::Fs),
            Capability::WallClock { .. } => Some(WasiCategory::WallClock),
            // Host-side grants without a WASI binding: warehouse
            // read/write goes through the host RPC surface; the event
            // bus, scheduler, and KV store are reserved WIT imports
            // whose host implementations are not wired through the
            // wasmtime linker in v0.1. Secrets / Custom likewise have
            // no public WASI binding. All collapse to `None`; the
            // capability gate still enforces them at the host call
            // boundary.
            Capability::WarehouseRead { .. }
            | Capability::WarehouseWrite { .. }
            | Capability::EventBus { .. }
            | Capability::DashboardRead { .. }
            | Capability::DashboardWrite { .. }
            | Capability::AuthzCheck { .. }
            | Capability::Ingest { .. }
            | Capability::Secrets { .. }
            | Capability::Custom { .. } => None,
        }
    }
}

/// Set of [`WasiCategory`] grants. Models the manifest's
/// `capabilities:` block after translation. Empty by default — every
/// component is instantiated with no WASI access (R8 default-deny).
///
/// Backed by a bit-field rather than a `HashSet` because the universe
/// of categories is tiny and known at compile time; this keeps the
/// per-call store-data lookup cheap and free of allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WasiCategorySet {
    mask: u8,
}

impl WasiCategorySet {
    /// Empty set — default-deny.
    pub const fn empty() -> Self {
        Self { mask: 0 }
    }

    /// Set containing one category.
    pub const fn just(cat: WasiCategory) -> Self {
        Self {
            mask: 1u8 << (cat as u8),
        }
    }

    /// Insert a category. Idempotent.
    pub fn insert(&mut self, cat: WasiCategory) {
        self.mask |= 1u8 << (cat as u8);
    }

    /// `true` iff the category is granted.
    pub fn contains(self, cat: WasiCategory) -> bool {
        self.mask & (1u8 << (cat as u8)) != 0
    }

    /// Build from the manifest's `capabilities:` slice. Unknown
    /// variants ([`Capability::Custom`], [`Capability::Secrets`]) are
    /// ignored — they have no WASI counterpart to link.
    pub fn from_manifest(caps: &[Capability]) -> Self {
        let mut set = Self::empty();
        for c in caps {
            if let Some(cat) = WasiCategory::from_capability(c) {
                set.insert(cat);
            }
        }
        set
    }

    /// Iterator over granted categories. Order is canonical (matches
    /// [`WasiCategory::ALL`]) so traces from different runs compare
    /// cleanly.
    pub fn iter(self) -> impl Iterator<Item = WasiCategory> {
        WasiCategory::ALL
            .iter()
            .copied()
            .filter(move |c| self.contains(*c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_new_refuses_zero_values() {
        // R8 "Per-call fuel + memory + wall-clock caps configured
        // host-side": silently allowing zero would defeat the rule.
        assert!(Caps::new(0, 1 << 20, Duration::from_millis(100)).is_none());
        assert!(Caps::new(1_000, 0, Duration::from_millis(100)).is_none());
        assert!(Caps::new(1_000, 1 << 20, Duration::ZERO).is_none());
    }

    #[test]
    fn caps_new_accepts_realistic_values() {
        let c = Caps::new(1_000_000, 16 * 1024 * 1024, Duration::from_millis(500))
            .expect("realistic caps must build");
        assert_eq!(c.max_fuel, 1_000_000);
        assert_eq!(c.max_memory_bytes, 16 * 1024 * 1024);
        assert!(c.wasi_categories.iter().next().is_none());
    }

    #[test]
    fn default_wasi_category_set_is_empty() {
        let set = WasiCategorySet::default();
        for cat in WasiCategory::ALL {
            assert!(
                !set.contains(*cat),
                "default-deny: {cat:?} must not be granted"
            );
        }
    }

    #[test]
    fn from_manifest_translates_known_categories_and_ignores_others() {
        let caps = vec![
            Capability::HttpOut {
                authorities: vec![],
            },
            Capability::WallClock { granted: true },
            Capability::Secrets {
                prefixes: vec!["weather:*".into()],
            },
            Capability::Custom {
                name: "kv".into(),
                params: serde_json::Value::Null,
            },
        ];
        let set = WasiCategorySet::from_manifest(&caps);
        assert!(set.contains(WasiCategory::HttpOut));
        assert!(set.contains(WasiCategory::WallClock));
        assert!(!set.contains(WasiCategory::Fs));
    }

    #[test]
    fn wasi_categories_iter_is_canonical_order() {
        let mut set = WasiCategorySet::empty();
        set.insert(WasiCategory::WallClock);
        set.insert(WasiCategory::HttpOut);
        set.insert(WasiCategory::Fs);
        let names: Vec<_> = set.iter().map(WasiCategory::as_str).collect();
        // ALL = [HttpOut, Fs, WallClock] → iteration order matches.
        assert_eq!(names, vec!["http_out", "fs", "wall_clock"]);
    }
}
