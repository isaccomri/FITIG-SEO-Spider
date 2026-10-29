# 0.1.54 — standard HTTP form authentication

Configuration → Form authentication independently implements an explicit standard UTF-8 HTML form workflow: GET the configured login URL, optionally read exactly one named CSRF input, submit the configured username/password fields once using URL-encoded POST, then verify a successful same-origin URL contains the configured non-sensitive success marker. The POST redirect is never followed. Failed verification prevents creation or mutation of the crawl project. Existing-project ownership/resume checks still precede authentication requests.

Login, verification and crawl seed require the same exact origin. HTTPS is required except literal localhost/loopback HTTP. Form login requires HTTP-only rendering, enabled cookies, direct proxy mode and no simultaneous Basic authentication. Saving explicitly selects those settings. Login POSTs are not retried; authentication exchanges precede ordinary paced crawl requests.

Credentials remain in application memory and are cleared when opening a saved project. Only configuration is saved; avoid sensitive values in configuration URLs or the verification marker. Login bodies and CSRF values are not persisted. Cookies are Windows-user encrypted and restricted to the exact login origin, including scheme/port. Scoped checkpoints use `windows-dpapi-origin-v2` so older readers reject the format instead of dropping the restriction. Existing unscoped cookie checkpoints retain their compatible format and normal host/path/port behavior.

Limits: 1 MiB UTF-8 response bodies; bounded credential/field/marker values; configured HTTP request timeout. This does not implement interactive browser login, JavaScript forms, SSO, MFA, CAPTCHA, automatic form-action/hidden-field discovery beyond the named CSRF input, CLI credential input, browser session transfer, or continuous signed-in-state detection. A marker verifies the configured response only; use one unique to the authenticated page. Each new or resumed form crawl requires credentials again.

Validation on 2026-10-30:

- 229 engine tests passed, including six new unit/integration cases covering configuration, CSRF, one submission/no redirected target, authenticated robots/pages, wrong credentials, failed verification/no project, secret omission and exact-origin encrypted checkpoint restore. A focused final checkpoint test also passed after explicitly asserting the scoped format marker.
- 103 UI tests and seven Node model tests passed. Production TypeScript/Vite and NSIS packaging built. Tauri unit tests were not rerun; native IPC/UI was exercised directly.
- Nine form-login assertions passed on the final release executable and installed application, covering the actual settings UI, no network on configuration, authenticated Hebrew crawl, one POST, redirect suppression, robots session, secret omission and credentials cleared on project reload.
- All 26 installed-native cookie regression assertions passed, including stop/resume, header redaction, DPAPI persistence, normal unscoped port sharing, metadata search/clear and invalid-state preservation.
- Early full-test failures came from a nonblocking fixture socket and an incomplete dummy project schema; those fixtures were corrected and the full engine suite passed again. The strengthened scoped checkpoint format was built and accepted in the final package.
- Targeted ESET scan: zero detections and zero cleaned objects. Installed executable and WebView2 loader match extracted payloads; all 15 original app-data files remain byte-identical. Backup: `work/installed-program-0.1.53-before-0.1.54`.

Installer SHA-256: `8BA3B05E2BAF541B578263B7A8BA58BC42EBFD33F24739E68BDC401631EDF998`.

Evidence: `acceptance/form-auth-installed-054/result.json`, `acceptance/cookies-installed-054/result.json`, `acceptance/installed-upgrade-0.1.54.json`; workspace logs `work/form-auth-engine-tests-final.log`, `work/form-checkpoint-final-test.log`, `work/form-auth-ui-tests-r2.log`, `work/core-054-tests.log`, `work/desktop-054-final.log`.

Row 145 is now PARTIAL_TESTED for this standard HTTP subset. Coverage is 196/207 (94.7%) partially tested rows; full functional parity remains incomplete.
