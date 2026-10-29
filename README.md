# FITIG SEO Spider

Independent Windows SEO crawl workbench using Rust, Tauri 2, React, TypeScript and SQLite. Source baseline: **0.1.52**.

## Status

195 of 207 tracked entries (94.2%) have partial implementation/test evidence. This measures matrix coverage, **not full Screaming Frog parity**. See [feature matrix](docs/feature-parity-matrix.md) and [scope and limitations](docs/milestone-90-coverage.md). This repository snapshot does not include private crawl data, credentials, installed competitor files, acceptance recordings or local build logs.

The local 0.1.52 release ran 223 engine, 102 UI and 7 Node tests plus 12 installed-native assertions. See [release notes](docs/release-0.1.52.md). Tauri unit tests were not rerun. Recent releases add inherited link targets, reviewed sitemap-to-List preparation and Meta Keywords. Complete Hebrew translation, form authentication, provider verification, trusted signing and exhaustive competitor behavior remain incomplete. Installed workspace observation confirms additional gaps; the percentage is not a full parity score.

## Build on Windows 11

Install current stable Rust (MSVC toolchain), Visual Studio C++ build tools, WebView2 Runtime and Node.js 24. From this directory:

```powershell
npm ci
npm --prefix ui ci
cargo test --locked --manifest-path engine/Cargo.toml
npm --prefix ui test
npm --prefix ui run test:core
npm --prefix ui run build
npm exec -- tauri build --bundles nsis --ci
cargo build --release --locked --manifest-path engine/Cargo.toml
```

The installer is under `src-tauri/target/release/bundle/nsis`. The CLI is `engine/target/release/fitig-engine.exe`. Binaries are unsigned. Test native Windows desktop features in an interactive session; CI builds cannot replace GUI acceptance and licensed/account service checks. Some OS-provider tests require Windows spelling services and per-user credential storage.

Development: `npm run dev` (UI on port 1420). Crawls are user-owned SQLite files; do not commit them.

## Rights and dependencies

This is an independent implementation. No proprietary competitor code, artwork, branding or licensing mechanism is distributed. No open-source license grant for original FITIG code has been selected; all rights are reserved. Third-party assets retain their own licenses and notices under `assets/third-party` (axe-core MPL-2.0; AMP validator Apache-2.0). Rust/npm dependencies retain their respective licenses.

## Latest release

[0.1.52 evidence and practical limits](docs/release-0.1.52.md): Meta Keywords, reviewed sitemap crawl preparation and inherited link targets. Local installed acceptance passed; account integrations, full competitor inventory and trusted signing remain incomplete. CI is independently reported by GitHub Actions.
