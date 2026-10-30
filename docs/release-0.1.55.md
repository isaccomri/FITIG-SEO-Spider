# 0.1.55 — observed image sitemap export

Adds an optional image extension to sitemap ZIP packages through the desktop exporter and `export-sitemap-package --images`. Existing options default to page-only output. Hebrew URLs remain readable; CDN images are retained, fragments removed and image URLs deduplicated per page. Invalid image URLs or more than 1,000 images on a page fail without replacing the previous export. Export uses saved observations and performs no network requests.

Validation before packaging: 232 Rust tests, 103 UI tests, TypeScript/Vite build, three actual native image-export assertions and 16 existing native sitemap regression assertions passed. The XML is independently parsed in the native test; database bytes and request count are checked unchanged during export. See `docs/image-sitemap-development-20261030.md` for details and limitations.

This does not implement every competitor sitemap control. Noindex-image filtering, image relevance thresholds, CDN regex selection, hreflang extensions and segments remain unimplemented in this exporter. Publication and search indexing are not performed. It remains 196/207 partially tested matrix rows, not 100% functional parity. Trusted installer signing remains unavailable without a certificate/service.

Installer verification: official NSIS build succeeded, targeted ESET scan passed with zero detections, installed payload hashes matched and 15 original app-data files remained byte-identical. All 19 native assertions passed again on installed 0.1.55. Installer SHA256: 4D58F3E3BE9FE308067BD82C3A9316831A3260320E8C58371C8418084EAF1C24. Evidence: acceptance/installed-upgrade-0.1.55.json; acceptance/image-sitemap-installed-055; acceptance/sitemap-regression-installed-055.
