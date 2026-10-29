# 0.1.53 — reset table columns

View → Reset Columns for All Tables restores default column order and widths across every supported tab. It persists the reset while preserving theme, language and panel geometry. It does not modify crawl data or initiate requests. This is independently implemented from observed installed 24.3 View menu behavior; complete menu/submenu equivalence remains unverified.

Validation on 2026-10-29: 103 UI tests and seven Node model tests passed; production frontend and NSIS build passed. Six native assertions passed against the release executable and again against the installed application: seeded nondefault order, visible reset, all-tab persisted reset, preserved other preferences, inactive-tab reset, and unchanged database bytes/no network. A syntax error in the new test script was corrected before the successful runs. Engine/Tauri suites were not rerun for this UI-only feature.

ESET installer scan found zero detections/cleaned objects. Installed executable/loader match extracted payloads; all 15 original app-data files remain unchanged. Prior program backup: `work/installed-program-0.1.52-before-0.1.53`.

Installer SHA-256: `209E6021F6FA6F94FAD39E7490312B39B002121295301D0B8D2F62AABFE4283F`.

Evidence: `acceptance/reset-columns-installed-053/result.json`, `acceptance/installed-upgrade-0.1.53.json`; workspace logs `work/reset-columns-ui-tests.log`, `work/core-053-tests.log`, `work/desktop-053-release.log`.

Coverage stays 195/207 partially tested rows (94.2%). See `remaining-207-acceptance.md` for the 12 open rows. The user confirmed no signing certificate/service is currently available; row 177 remains pending. Full parity has not been achieved.
