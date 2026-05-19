## Done

- Reviewed stage-0/1 diff (SCOPE.md decisions only; no code yet) against Layer-1 invariants R1, R2, R4, R5 and wire-format rule R10.
- Confirmed JSON-RPC 2.0 envelope + streaming sub-protocol live in starter-ext-spi, adapter applies per-entry auth, version negotiation deferred to v0.2 host_capabilities.
- Emitted PASS sentinel for the gate.

## Next

- (none — fresh session picks up stage 3.)

## What you need to know

- PASS: Stage 0/1 only edited DOCS/extensions/scope/SCOPE.md; design preserves R1 trait/flavour split, R2 spi-only-depends-on-starter-spi arrow, R4 reverse-DNS ids, R5 stateless behaviours, and leaves the JSON-RPC 2.0 wire format intact with streaming notifications co-located in starter-ext-spi.
- No commit was needed on this review stage (working tree clean; sentinel is the deliverable).

## Open questions

- (none)
