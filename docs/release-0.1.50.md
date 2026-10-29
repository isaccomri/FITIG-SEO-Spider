# 0.1.50 — inherited link targets

The crawler now records the first `base[target]` value on anchors and image-map areas that lack their own target. An explicit target, including the empty string, overrides the default. The first href-bearing base remains independent. Resource links do not inherit a navigation target. Stored targets are extracted metadata, not a claim to emulate browser navigation or target sanitization. Existing saved observations are not rewritten.

Validation on 2026-10-29:

- 222 engine tests passed (191 library and 31 integration/CLI).
- 98 UI tests and 7 Node model tests passed.
- Production frontend and Windows NSIS installer built successfully.
- Six native assertions passed against the release executable and again against the installed application: six fixture URLs, Hebrew inherited targets, explicit overrides, independent href base, reload persistence and unchanged database bytes.
- Twelve installed-native response-hash regression assertions passed, including actual table filtering, compare and request pacing.
- Installer ESET scan: zero detections and zero cleaned objects. Installed executable and loader match extracted payload hashes; all 15 original app-data files remain unchanged.

Installer SHA-256: `330DC3FD87374FC1740B4664922CDF68E6631EC71E8AF909B11DC72B80D8D2AD`.

Evidence: `acceptance/base-target-installed-050/result.json`, `acceptance/response-hash-installed-050/result.json`, `acceptance/installed-upgrade-0.1.50.json`; workspace logs `work/base-target-engine-tests.log`, `work/ui-050-tests.log`, `work/core-050-tests.log`, `work/desktop-050-release.log`.

Coverage stays 194/206 (94.2%) partially tested rows. This closes a bounded gap in row 203; it does not establish full competitor parity. OpenAI credential setup, form authentication, complete translation, exhaustive competitor inventories and trusted signing remain open. The previous installed program is backed up in `work/installed-program-0.1.49-before-0.1.50`.
