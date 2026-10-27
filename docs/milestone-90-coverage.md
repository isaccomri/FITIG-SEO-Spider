# 90% partial-coverage milestone

User-authorized stop threshold: at least 90% in the existing partial-tested-entry metric. Result: 186/206 = 90.3%. Full parity is NOT achieved; partially tested rows still contain explicit limitations.

32 newly qualified entries, with actual installed assertions:

| ID | Feature | Scope |
|---|---|---|
| FITIG-025 | Bounded response body | Owned 11 MiB HTTP response retains at most 10 MiB plus one sentinel byte, skips extraction/storage and records an explicit issue. This is a retained-payload bound, not total process/wire-memory certification. |
| FITIG-028 | Response headers | Known HTTP headers and selected native Headers pane; duplicate/complex protocol behavior is certified only where separately tested. |
| FITIG-046 | XML sitemap loc discovery | Fetched XML URL sitemap loc entity decoding, actual frontier discovery and depth-two fetch; not every XML/index/namespace variant. |
| FITIG-051 | H3 | All H3 occurrences, nested text and Hebrew in saved observations and actual H3–H6 cells. |
| FITIG-052 | H4 | H4 Hebrew text in saved observations and actual result cells. |
| FITIG-053 | H5 | H5 Hebrew/entity-decoded text in saved observations and actual result cells. |
| FITIG-054 | H6 | H6 Hebrew text in saved observations and actual result cells. |
| FITIG-058 | Hreflang declarations | HTML he/x-default hreflang declarations, resolved Unicode URL and actual tab membership; declarations are not validation of language/target correctness. |
| FITIG-059 | Image URL and alt | Resolved img src URLs, Hebrew/entity-decoded alt and absent alt text; srcset/lazy/CSS completeness is not certified. |
| FITIG-060 | Anchor text | Nested anchor text with normalized Hebrew whitespace; complete ARIA/image-derived accessible naming is not certified. |
| FITIG-061 | Link rel | Raw nofollow/ugc rel and target retained separately from anchor text. |
| FITIG-065 | Response size | Independently counted UTF-8 HTML and PNG retained payload lengths; not compressed wire size. |
| FITIG-066 | Response elapsed time | Per-page elapsed time includes an independently held delay; not pure network latency calibration. |
| FITIG-067 | Missing title | Absent/whitespace title diagnosis and actual Missing filter, excluding known unavailable/oversized/error HTML extraction. |
| FITIG-068 | Missing description | Absent/whitespace description diagnosis and actual Missing filter, excluding known unavailable/oversized/error HTML extraction. |
| FITIG-069 | Missing H1 | Absent and empty H1 diagnosis and actual Missing filter; nonempty mixed occurrences are not falsely missing. |
| FITIG-070 | Multiple H1 | Two H1 occurrences retained, expected Multiple H1 issue and actual filter membership. |
| FITIG-071 | Missing image alt | Missing image alt text issue on source plus actual resource filter; empty decorative alt versus absent-attribute taxonomy is not fully certified. |
| FITIG-073 | HTTP error issue | Observed HTTP 404 issue without invented extracted missing metadata; not exhaustive status/auth policy coverage. |
| FITIG-074 | Plain HTTP issue | Observed plain HTTP fixture issues; HTTPS/TLS security certification is not implied. |
| FITIG-076 | Inlink relationships | Repeated link occurrences, two distinct sources, analysis and actual selected Inlinks evidence; not full link-origin taxonomy. |
| FITIG-077 | Broken internal link report | Broken internal anchor source/observed target/404 report; uncrawled targets are not certified as broken. |
| FITIG-086 | Missing HSTS | Actual Missing HSTS filter follows observed header presence on owned HTTP fixtures; this does not prove browser TLS/HSTS enforcement. |
| FITIG-087 | Missing CSP | Actual Missing CSP filter follows observed header presence; no complete CSP efficacy validation. |
| FITIG-094 | Read-only load | Read-only project load/snapshot byte preservation without website refetch. |
| FITIG-095 | Project seed guard | Wrong resume seed is rejected before database mutation or HTTP. |
| FITIG-096 | Queued count query | Held-response nonzero pending/inflight queue and independently enumerated final zero queue/13 observed URLs. |
| FITIG-097 | CSV report | Independent standard-library CSV parsing: all rows, UTF-8 BOM, Hebrew and decoded/original URL identity. CSV remains raw text; spreadsheet formula safety is separately certified for XLSX. |
| FITIG-098 | HTML report | Independent HTML inspection: UTF-8 and escaped markup-like Hebrew title; full browser report layout is not certified. |
| FITIG-099 | Typed XLSX report | Independent XLSX ZIP/XML inspection: numeric status/depth/word cells and string formula-like title with no formula cells; not all Excel limits/features. |
| FITIG-100 | CLI JSONL progress | Real standalone CLI crawl emits independently counted JSONL page/completion events with numeric counters; not every error/flag variant. |
| FITIG-101 | Tauri command bridge | Actual main-webview commands remain responsive during held HTTP and round-trip saved snapshots; remote-webview denial has separate existing regression coverage. |

Remaining unqualified entries (partial rows also have remaining scope):

| ID | Feature | Status |
|---|---|---|
| FITIG-026 | Request timeout | SOURCE_UNVERIFIED |
| FITIG-064 | Exact body hash | SOURCE_UNVERIFIED |
| FITIG-075 | Exact duplicate bodies | SOURCE_UNVERIFIED |
| FITIG-102 | Default timestamped project | SOURCE_UNVERIFIED |
| FITIG-120 | Site Structure directory tree | MODEL_TESTED_UI_UNVERIFIED |
| FITIG-145 | Form authentication | PENDING |
| FITIG-155 | Export all competitor report variants | PENDING |
| FITIG-157 | Google Sheets export | PENDING |
| FITIG-161 | Semantic search | PENDING |
| FITIG-166 | AI providers | PENDING |
| FITIG-172 | Full Hebrew translation | PENDING |
| FITIG-177 | Signed installer | PENDING |
| FITIG-179 | Complete installed menu tree | PENDING_EVIDENCE |
| FITIG-180 | Every installed column | PENDING_EVIDENCE |
| FITIG-181 | Every installed tab filter | PENDING_EVIDENCE |
| FITIG-182 | Every settings default | PENDING_EVIDENCE |
| FITIG-183 | Every export variant | PENDING_EVIDENCE |
| FITIG-184 | Licensed workflow availability | PENDING_EVIDENCE |
| FITIG-187 | Feature availability view | IMPLEMENTED_UI_UNVERIFIED |
| FITIG-202 | Native installer compiler investigation | BLOCKED_SECURITY_REVIEW |
