# FITIG SEO Spider 0.1.49

Installed and checked on 2026-10-27. Partial-tested coverage is 194/206 (94.2%); this is not full Screaming Frog parity.

## Google Sheets export

Reports > Export Google Sheets previews a snapshot of the current filtered table and column order, with a separate original-URL column. Explicit upload consent plus an authorized manual Google OAuth access token are required to create one new spreadsheet. The payload includes the initial grid data in the creation request; existing spreadsheets are not overwritten or appended. Tokens are masked, cleared after attempts and not stored in projects. There is no automatic retry, OAuth sign-in/refresh or Drive-folder placement. Modal dismissal is disabled during the creation request; closing the application itself can still leave the outcome uncertain. The receipt is shown only for the session, so copy its URL. On an ambiguous failure, check Drive before trying again.

Creation is implemented against the official fixed HTTPS endpoint, with redirects disabled and a 30-second HTTP timeout. Response bodies and credentials are omitted from errors. Valid returned spreadsheet IDs produce a computed Google Docs URL; arbitrary returned URLs are not followed. A per-process backend gate prevents overlapping creation requests and releases on failed preflight. Preview sends nothing to Google and explicitly invalidates the prior approval snapshot when its title/layout changes.

The export currently supports 1–64 columns, at least one result row and at most 10,000 cells including headers, 512 KiB source text and a 1 MiB JSON request. Header rows are bold/frozen. Numeric observations use numberValue, unknown numeric values remain empty, booleans are typed and text uses stringValue even when beginning with =. Large integer cells outside JavaScript's exact range remain strings. Hebrew text/URLs are preserved and right-to-left sheet layout is optional. These are scoped export bounds, not crawl/license limits; chunking/large-sheet exports remain pending.

## Verification

220 engine unit/integration/CLI tests, 98 UI tests, 7 Node model tests and 9 Tauri tests passed: 334 total. Seven installed-native suites passed 140 checks: Google Sheets 18, semantic 18, workspace basics 23, response hash 12, core contract 40, batch 21 and concurrency 8.

New Rust tests inspect independently expected string/number/bool/null/large-integer payloads and enforce bounds/consent. An owned local HTTP server verifies a real POST, bearer header, Unicode grid body, safe receipt URL, redirect rejection and error redaction. Three UI model tests check filtered order, original/readable Hebrew URLs, unknown numeric values, precision and limits. Eighteen installed checks exercise the actual Reports workspace, filtered two-row snapshot, column order, formula-like title as text, number cells, Hebrew/RTL, masked input, disabled consent/credential paths, clearing a rejected credential, request-gate recovery, stale-preview invalidation and unchanged crawl bytes.

No Google account request or real spreadsheet creation was performed during acceptance. Live account permissions, enabled API, actual cloud spreadsheet contents and account workflow remain unverified. The matrix entry is partial-tested, not complete. No credential is committed or included in the source archive.

## Packaging

Installer and CLI targeted ESET scans reported zero detections and zero cleaned files. Silent upgrade from 0.1.48 succeeded, executable/loader match the installer payload and all 15 original app-data files remain byte-identical. Prior program files are backed up locally. Installer SHA-256: 7F251C84C19A44F03D86B946DD23065AF4E83C1717EAC4B58E3F0E5470D7390F.

The independent repository has a successful 0.1.48 Windows CI run. A later 0.1.49 commit/check is separate evidence; local acceptance does not certify remote CI or a new machine. Full translation, form login, AI-provider execution, complete competitor inventory/report coverage, account verification and code signing remain incomplete.

## Primary contract references

- [Create a spreadsheet](https://developers.google.com/workspace/sheets/api/reference/rest/v4/spreadsheets/create)
- [Sheet and grid data](https://developers.google.com/workspace/sheets/api/reference/rest/v4/spreadsheets/sheets)
- [Extended cell values](https://developers.google.com/workspace/sheets/api/reference/rest/v4/spreadsheets/other#ExtendedValue)
