# FITIG crawl engine

Independent Rust implementation. Requires a Rust toolchain and Windows C/C++ build tools for bundled SQLite.

```powershell
cargo test
cargo run -- crawl --url https://example.com/ --db crawl.sqlite --limit 500 --delay-ms 200
cargo run -- inspect --db crawl.sqlite
cargo run -- analyze --db crawl.sqlite
cargo run -- export --db crawl.sqlite --out crawl.csv --format csv
cargo run -- export --db crawl.sqlite --out crawl.html --format html
cargo run -- export --db crawl.sqlite --out crawl.xlsx --format xlsx
cargo run -- compare --db baseline.sqlite --other current.sqlite
cargo run -- crawl --url https://example.com/ --db crawl.sqlite --limit 1000 --resume
```

The crawl command emits one JSON object per line, with `event`, `completed`, `queued`, and optional `page`. Errors go to stderr as JSON and exit code 1. A fresh crawl clears the specified database's crawl tables. Use a separate database per crawl and use `--resume` to continue. Resume validates the original seed URL.

Creating `<database-path>.stop` requests a stop after the current request; remove it before resuming. Every completed result and newly discovered URL are committed together. There is a 20-second request timeout and a 10 MiB response extraction limit. Fetching is sequential, with a configurable delay.

Implemented extraction: HTML title and description; H1–H6; canonical; meta and HTTP robots directives; hreflang pairs; anchors; scripts; stylesheets; image URLs and alt text; JSON-LD syntax; body word count and SHA-256; raw response headers; status; redirect targets; request errors; HTTP URLs and basic missing metadata issues. HTML extraction treats input as UTF-8 with replacement for invalid bytes. XML sitemap `loc` entries are discovered from XML responses. Same-origin anchor/script/style/image/sitemap URLs and same-origin redirects enter the frontier. Same-origin image URLs are fetched automatically. External URLs are inventoried but are not fetched.

Robots is fetched before a crawl; 401/403 disallow all, 404 allows all, unexpected status/network errors stop the crawl. Rules support prefix and `*` patterns. Groups select the most specific matching user agent and merge equally specific groups; rules support terminal $ and allow wins ties. Percent normalization is not implemented. Scope uses URL origin (scheme, hostname, effective port). Redirects are stored as individual rows rather than silently followed. Every request uses `FITIGBot/0.1`.

Analysis produces exact body-hash groups, inlinks and links to fetched HTTP error pages. Compare tracks added/removed URLs and changes to status, title, description, canonical, and body hash. CSV export follows normal CSV quoting. HTML export escapes user content. XLSX export uses rust_xlsxwriter (default-enabled optional xlsx feature), preserves numeric status/depth/word-count cells and writes textual cells explicitly as strings. Build with --no-default-features to exclude XLSX.

Not implemented here: Chromium rendering, browser cookies/login, GA4/GSC/PSI APIs, near-duplicate scoring, full structured-data schema validation, semantic hreflang validation, gzip sitemap decompression, response charset detection, render screenshots or full-text custom extraction. Rust tests remain unverified: cargo test was attempted on 17 October 2026, but Windows Application Control rejected execution of markup5ever build-script-build with OS error 4551. The policy was not bypassed. The crate has not completed compilation.

Tests include Unicode metadata extraction, link normalization, missing image alt, malformed JSON-LD, unsafe scheme exclusion, robots prefixes, HTML escaping and a local HTTP fixture exercising SQLite persistence, crawl resume, robots blocking, broken-link analysis, compare identity and CSV/HTML export.
