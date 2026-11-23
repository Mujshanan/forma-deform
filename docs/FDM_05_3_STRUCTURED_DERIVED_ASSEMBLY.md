# FDM-05.3 — Structured/Derived Output Assembly

**Status:** implementation in progress  
**Branch:** `feat/fdm-05-3-structured-derived-assembly`  
**Parent:** FDM-05.1/05.2 PR #57 at `edb524bc46248b755fa2b0646aaa9cac72fcc479`

## Purpose

FDM-05.3 introduces one immutable assembly boundary for:
- the authoritative, command-preserving evaluated `Path2D`; and
- the refined output-space curve samples produced by FDM-05.2.

The constructor performs refinement itself from the same validated source, binding, topology, rest cage, current cage, and outside-cage policy used to verify the structured output. This avoids accepting a separately-created sample set whose provenance cannot be proven by the current FDM-05.2 sample type.

## Assembly invariants

- Preserve all original Path2D commands, order, subpath boundaries, and Bézier handles.
- Produce exactly one refined segment for every original quadratic/cubic command, in command order.
- Preserve original command and subpath indices.
- Require endpoint-inclusive samples (`t=0` and `t=1`) and strictly increasing finite parameters.
- Reject missing, extra, reordered, or misidentified refined segments.
- Validate policy version, finite deviation diagnostics, and sample-count accounting.
- Verify source snapshot, validated binding stamp, rest-cage fingerprint, and current-cage fingerprint before assembling.
- Never flatten, mutate, or replace the structured Path2D.
- No AniCraft UI/host integration, persistence, rendering, apply/bake, or Smart Controller composition.

## Accuracy semantics

The assembled derived samples retain FDM-05.2's three-probe deviation estimate. Assembly does not upgrade it into a certified maximum-error bound.

## Tests

The initial suite covers mixed line/quadratic/cubic/close commands across multiple subpaths, complete command coverage, wrong/missing/extra command identities, monotonic parameter order, and sample-count accounting. The full PR CI must additionally validate integration with the existing workspace and all release WASM targets before this milestone is complete.
