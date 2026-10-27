# FITIG SEO Spider 0.1.47

Installed and checked on 2026-10-27. Partial-tested matrix coverage: 192/206 (93.2%). This measures scoped implementation/test evidence, not full Screaming Frog parity.

## HTTP timeout

The interactive configuration and CLI crawl profile support `request_timeout_secs`, integer 1–300, default 20. Batch manifest jobs support the same optional property. The limit applies per HTTP attempt including response-body reading; transient errors before response headers may retry up to three times with the existing backoff. Interrupted bodies are not retried. Crawl duration is not limited to one timeout interval; retries, robots requests, scheduling and rendering have their own work. Chromium retains a separate render timeout.

Timeouts are stored in SQLite profiles and restored when opening a crawl. Resume rejects a changed profile without rewriting it. Old profiles default to 20 seconds. Invalid bounds are rejected before creating a project or batch output. Failed reads keep unknown body hashes, preserve recorded failure evidence and do not diagnose content as successfully extracted. Body-read errors now retain their underlying cause instead of only the generic top-level message.

## Validation

208 engine unit/integration/CLI tests, 95 UI tests, 7 Node model tests and 9 Tauri tests passed (319 total). Five installed-native suites passed 104 checks: workspace basics 23, response hash 12, core contract 40, batch 21 and concurrency 8. New owned fixtures test slow headers, stalled response bodies, retry bounds, fast pages, actual UI setting/profile restoration, pre-write validation, and a real batch timeout. An initial assertion expected the generic body error string to expose the timeout; the error reporting was fixed and the test now verifies the actual cause. Header retries and interrupted-body behavior are tested separately.

Scoped UI evidence also closes previously unverified tracker entries for default timestamped projects, the Site Structure directory tree and Feature availability. Two generated projects remain distinct and can be reopened through the actual File menu. The recorded-URL directory tree shows Hebrew, collapses/expands branches and selects a URL in the results. Help shows capability rows and explicit pending account-verification limits, and closes with Escape. Inspection preserves SQLite bytes and sends no HTTP. These checks do not certify all competitor options or simultaneous multi-process project naming.

## Packaging and preservation

The official NSIS installer and CLI passed targeted ESET scans with zero detections and zero cleaned files. Upgrade from 0.1.46 exited successfully. The executable and WebView2 loader match the installer payload. All 15 original app-data files remain byte-identical after acceptance. Native tests also created two new owned automatic projects; their paths are recorded in the local acceptance result. Original data was not deleted. Prior program files were backed up locally.

Installer SHA-256: 00B8C8BFF0C48C5E4C0FDA4BC33CDEE02E8C84911B416F00740EFDA207E824CB.

Full translation, form login, Sheets export, AI/semantic providers, complete installed competitor inventory/account verification and code signing remain incomplete. Source and build results must be tracked independently of the existing successful GitHub 0.1.46 CI run; no 0.1.47 CI success is inferred.
