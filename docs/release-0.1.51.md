# 0.1.51 — prepare a crawl from sitemap membership

The sitemap validation panel now offers **Prepare sitemap URL crawl…**. It normalizes and deduplicates the recorded inventory, opens the existing List review, and clears the prior output path/resume selection so a subsequent Start creates a separate project. All supplied origins are included and disclosed. Configuration and profile settings otherwise remain selected; crawl limits and robots rules continue to apply. It does not automatically fetch a sitemap, start requests, append to a running frontier or rewrite old observations.

Preparation rejects empty, oversized (over 250,000 entries), relative, non-HTTP, credential-bearing or fragment-bearing locations. Unicode and percent-encoded Hebrew identities deduplicate. The button is disabled while importing or crawling.

Validation on 2026-10-29:

- 100 UI tests and seven Node model tests passed; production TypeScript/Vite build passed.
- Six native assertions passed against the release executable and again against the installed copy: canonical Hebrew deduplication, no HTTP during review/confirmation, separate project, persistent List inventory, robots enforcement and exclusion of discovered links from the queue. The original fixture database remained byte-identical.
- All 19 installed-native robots/sitemap discovery regression assertions passed, including nested roots, gzip, selection, failure preservation, cycles and rejected redirects/HTML challenges.
- Installer build succeeded after retrying a packaging file lock caused by prematurely launching the executable during the first packaging run. The first native attempt also exposed a test locator issue; the corrected dialog-scoped textarea locator passed.
- Installer ESET scan: zero detections/cleaned objects; installed payload hashes match; all 15 original user-data files remain identical. Prior program backup: `work/installed-program-0.1.50-before-0.1.51`.

Evidence: `acceptance/sitemap-list-installed-051/result.json`, `acceptance/robots-sitemaps-installed-051/result.json`, `acceptance/installed-upgrade-0.1.51.json`. Workspace logs: `work/sitemap-list-ui-tests.log`, `work/sitemap-list-ui-build.log`, `work/core-051-tests.log`, `work/desktop-051-release-retry.log`.

Installer SHA-256: `23FE93B5C311F4B390DD14AD516F5695D189C0A33743A2DA89C8F96A849D8312`.

Matrix remains 194/206 (94.2%) partially tested rows. This improves row 200 without certifying full competitor sitemap behavior or full application parity. Engine and Tauri test suites were not rerun for this UI-only behavior; their prior results are historical.
