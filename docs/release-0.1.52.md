# 0.1.52 — Meta Keywords workspace

Local observation confirmed Screaming Frog 24.3 Licensed exposes Meta Keywords and associated columns. FITIG now independently collects all case-insensitive `meta[name=keywords]` occurrences in document order, decodes HTML entities, trims boundary ASCII whitespace, and persists the values in page JSON. Known absence is an empty array; old/unavailable observations remain null. No migration rewrites old crawls.

The new Meta Keywords tab displays first value, Unicode code-point length, element count and all values. Filters provide observed absence (no non-whitespace values), exact nonempty first-value duplicates, multiple tags and unknown legacy records. It excludes known non-HTML or unavailable HTML extraction. Hebrew search, sort and existing current-table export use the recorded values. This does not assert competitor-equivalent length semantics, thresholds, duplicate policy, generic report exports or change detection for the new field.

Validation on 2026-10-29:

- 223 engine tests passed after the final data-transfer fix, including legacy deserialization and occurrence extraction.
- 102 UI tests, seven Node model tests and production frontend build passed.
- Six native assertions passed against the release executable and installed application, checking the real Rust crawl, Hebrew observations, duplicate/multiple/missing table filters and byte-preserving reload.
- Six installed sitemap-to-List regression assertions passed.
- The first native run caught missing extraction-to-crawl field transfer. This was fixed and the engine suite, build and native acceptance rerun; the initial run is not reported as a pass.
- NSIS installer built. ESET scan: zero detections and zero cleaned objects. Installed executable/loader match payload hashes and all 15 original app-data files remain unchanged. Backup: `work/installed-program-0.1.51-before-0.1.52`.

Evidence: `acceptance/meta-keywords-installed-052/result.json`, `acceptance/sitemap-list-installed-052/result.json`, `acceptance/installed-upgrade-0.1.52.json`; workspace logs `work/keywords-engine-tests-r2.log`, `work/keywords-ui-tests.log`, `work/core-052-tests.log`, `work/desktop-052-release-r2.log`.

Installer SHA-256: `DDE95AB6D7C208ED9B522FA4411F4524F9E1E6CBF216798A5E4F7EFA7F6063ED`.

The matrix adds previously omitted row 207: 195/207 (94.2%) partially tested rows. This is a bounded coverage metric, not exhaustive functional parity. Installed workspace observation revealed additional unqualified areas; see `installed-workspace-observation-20261029.md`. Full submenu, column, filter and default enumeration remains pending. Tauri unit tests were not rerun for this release.
