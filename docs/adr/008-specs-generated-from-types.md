# ADR-008: specs generated from types

Date: 2026-08-08 · Status: Accepted

## Context
adr/003 made the `howto` generated but left `docs/specs/` (the JSON I/O contracts)
**human-maintained** — the one remaining doc surface that can drift from code.
With adr/007 now forcing every command to carry `///` docs, an example block, and a
paired test, the `*Spec`/`Data` serde types and their representative examples are a
reliable single source to generate the specs from.

## Decision
`xtask gen-specs` generates every envelope/spec **table** in-place inside
`docs/specs/*.md`:
- **spec inputs** (02–07) from the `*Spec` serde structs — serialize → example YAML
  + a field table. ([adr/014](014-yaml-spec-input.md): specs are YAML-only; the
  spec-input examples serialize via `serde_yml`, output-contract examples stay JSON.)
- **output contracts** (08–19, 21, 22, 23) from the `Data` enum variants — serialize
  → the `data` column; the input/arg column comes from clap introspection (the same
  `///` surface adr/007 enforces).
- Each type carries a representative example in a **co-located `mod examples`**
  (single source, next to the struct — like a `Default`/doctest), exposed to the
  xtask via one `pub` registry fn (so `missing_docs` hits that one item, not each
  example).

### Region-with-markers
gen-specs replaces **only** the table between `<!-- BEGIN GENERATED -->` and
`<!-- END GENERATED -->` markers. Hand-written narrative outside the markers is
preserved — specs keep their context prose (e.g. `09-lesson`'s full-tree
description) alongside a machine-faithful table. Pure-prose files with no generated
region: `01-envelope`, `20-helper-contract`.

`howto.gen.md` stays whole-file generated. The marker regions join it on the
**never hand-edit** list; `cargo xtask build` regenerates them; drift fails
`cargo test` (see Current status).

This **supersedes** the adr/003 consequence that "detailed JSON I/O contracts live in
`docs/specs/` (human-maintained)."

### Current status
Shipped: `xtask gen-specs` generates every spec table in-place between the
markers; all specs except the pure-prose `01-envelope` and `20-helper-contract`
carry a `<!-- BEGIN GENERATED -->`…`<!-- END GENERATED -->` region. Drift is
caught inside `cargo test` (not `git diff` in CI): `howto_gen_md_is_fresh` and
`specs_marker_regions_are_fresh` regenerate each surface to a buffer and assert
byte-equality with the committed file.

## Consequences
+ Specs can no longer drift from code — the typed `*Spec`/`Data` shapes **are** the
  contract.
+ The mandated `<command>_*` tests (adr/007) are the goldens that validate the
  generated envelopes end-to-end.
+ One atom (the type + its `mod examples`) feeds both enforcement (compile) and the
  generated docs — fully DRY.
− Each `*Spec`/`Data` type must carry an example value in its `mod examples` — small
  authoring cost, co-located with the type.
− Generated markdown tables are less narrative than hand prose. Accepted:
  machine-faithful > pretty.
