# Perfpal Privacy Statement

Effective date: 2026-09-24

## Summary

Perfpal collects no telemetry, has no analytics, has no accounts, and makes no
network calls. It runs entirely offline.

## What Perfpal accesses

Perfpal reads the directory or file you point it at, and writes the output report
directory (`perfpal-report/` by default).

## What the report contains (local only)

The report records run provenance — the scanned root path (`root_path`), the
start time (`started_at`), and the duration (`duration_ms`) — plus code-derived
findings. This data stays on your machine and is never transmitted.

## What leaves your machine

Nothing.

## Verification

The tool's source has no network client and no telemetry code (verified against
`src/`).

## Release pipeline (separate)

The GitHub Actions release pipeline downloads and builds tooling in CI. That
pipeline is not part of the tool, and the tool itself never transmits anything.

## Contact

<!-- FOUNDER: the contact address is intentionally DEFERRED (session 34 /
     beta-release B6) — the founder will obtain a dedicated address before
     tagging. This placeholder MUST be replaced with a real address before
     v0.2.0-beta.1 is tagged. -->
Contact: [licensor contact email]
