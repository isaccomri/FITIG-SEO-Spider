# FITIG SEO Spider 0.1.48

Installed and checked on 2026-10-27. Partial-tested feature coverage is 193/206 (93.7%), not full Screaming Frog equivalence.

## Semantic vector search

Crawl Analysis > Semantic vector search imports explicitly supplied embeddings for existing crawl URLs. The corpus declares version 1, a compatible model identifier and URL/vector documents. Vectors are normalized with scaling to handle finite extreme values safely. Queries accept either a vector from the same declared model or a recorded vector URL; similar-URL mode excludes itself. Results use cosine similarity, minimum score -1 to 1, top K 1–200 and deterministic URL ordering for ties. Scores are not probabilities, and a matching model string cannot prove provenance or relevance.

Corpus input is bounded to 16 MiB, 10,000 recorded URLs, 2–4,096 dimensions and one million numeric components. Unknown/duplicate/credential-bearing/fragment URLs, zero/nonfinite/mismatched vectors and incompatible queries are rejected before replacing saved results. Import explicitly saves the corpus and clears its prior report. Search explicitly saves the report. Invalid operations preserve previous data. Saved reads do not create or migrate databases. Query vectors are not retained in the report; the corpus remains saved locally. No provider, network request or model download is part of this feature. Generating neural embeddings, natural-language query embedding and live-provider verification remain pending.

The results table supports original-URL inspection and UTF-8 BOM CSV with readable Hebrew addresses plus their original URL identity. Spreadsheet-like text remains CSV text; XLSX/formula-execution safety is not newly certified here. CLI commands are `semantic-import --db CRAWL --file CORPUS.json`, `semantic-search --db CRAWL --file QUERY.json` and read-only `semantic-results --db CRAWL`. Imports/searches are explicit writes; reading results is read-only.

Corpus example:
```json
{"version":1,"model":"your-compatible-model","documents":[{"url":"https://example.com/","vector":[1,0]}]}
```
Query example:
```json
{"model":"your-compatible-model","query_vector":[1,0],"label":"שאילתה","top_k":25,"minimum_score":0}
```
For similar-URL queries use query_url instead of query_vector. These illustrative vectors test mathematics; they are not evidence of natural-language semantic quality.

## Verification and preservation

216 engine unit/integration/CLI tests, 95 UI tests, 7 Node model tests and 9 Tauri tests passed: 327 total. Six installed-native suites passed 122 checks: semantic 18, workspace basics 23, response hash 12, core contract 40, batch 21 and concurrency 8. Semantic fixtures independently expect cosines 1, 0.8, 0 and -1; test thresholds, top-K matched counts, self exclusion, Unicode, input rejection, unchanged crawl observations, Python CSV parsing and database overwrite guards. A fresh owned desktop process restores the saved corpus/report, and the standalone release CLI reads it without modifying the file. Reimport clears stale results. The first restart test encountered a closing CDP endpoint; assigning a separate debugging port to the second owned process fixed the harness race and the complete rerun passed.

The installer/CLI passed targeted ESET scans with zero detections/cleaned files. Upgrade from 0.1.47 succeeded; executable/loader match the installer payload and all 15 original app-data files remain byte-identical. Backup of prior program files is retained locally. Installer SHA-256: F4019EF9EB0FD0240FA21457FDEA1262A09D196D8B92A3B925D7519E860B883A.

The independent GitHub repository has a successful 0.1.47 Windows CI run. A new source commit and CI result are tracked separately; no 0.1.48 GitHub success is inferred from local tests.

Full translation, form login, Sheets export, AI-provider execution, complete installed competitor inventory/account verification, signing and other partial-feature limits remain open. Full parity is not claimed.
