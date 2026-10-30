# Image sitemap extension — verified development change

This records development validation of the image sitemap extension, subsequently packaged and installed in 0.1.55. It is not a claim of full competitor parity. See release-0.1.55.md for installed acceptance.

The export UI offers **Include observed images**. CLI `export-sitemap-package` accepts `--images`. The option defaults to false, including when old options JSON omits it. Existing page eligibility, gzip, index splitting and optional observed hints apply. Image entries use Google's image namespace and required location elements. UTF-8 Hebrew remains readable by default. Cross-origin CDN image URLs are retained; fragments are removed and parsed URL identities deduplicated per page. Invalid/credential-bearing/non-HTTP image URLs and more than 1,000 distinct images on a page fail the export without replacing previous output. No network requests are made by export. No image downloads, publication or indexing outcome are implied.

Specification: [Google image sitemaps](https://developers.google.com/search/docs/crawling-indexing/sitemaps/image-sitemaps), inspected 2026-10-30. Deprecated caption/title/license extensions are omitted.

Validation:

- Existing nine sitemap Rust tests plus two new image cases passed together (11/11).
- Additional boundary case passed: exactly 1,000 images and UTF-8 byte splitting with the image namespace overhead.
- Final engine full suite passed with all three new tests (232 tests). 103 UI tests passed; TypeScript/Vite build passed.
- Release executable compiled successfully. Three native assertions passed through the actual UI: fixture crawl, independently parsed XML with Hebrew CDN deduplication, and byte-identical crawl DB/no additional HTTP requests.
- Evidence: `acceptance/image-sitemap-development-054/result.json`; local logs `work/image-sitemap-*.log` at the task workspace root.
- All 16 existing native sitemap-package regression assertions passed on the same development executable; evidence: `acceptance/sitemap-regression-development-054` and `work/image-sitemap-regression.log`.

Observed competitor gap remains: installed 24.3 image sitemap dialog also has noindex-image inclusion, relevant-image inlink threshold (loaded value 10), CDN regex configuration, hreflang and segments. These were observed from a copied reference crawl, not certified as factory defaults. They are not implemented by this change. Images are only those already recorded in `Page.images`; no CSS/background or new browser discovery is introduced.

Rollback: restore `engine/src/sitemap_export.rs` and `ui/src/SitemapExport.tsx` from task-local `work/before-image-sitemap`, remove the new native test, and remove the `--images` option wiring from `engine/src/main.rs`. During initial development the installed application remained unchanged; the later 0.1.55 upgrade has a separate program backup and verified original data preservation.
