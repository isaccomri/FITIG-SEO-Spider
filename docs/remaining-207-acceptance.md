# Remaining acceptance to qualify all 207 rows

Current partial-tested coverage: 196/207. Qualifying all rows is distinct from full functional equivalence, because existing qualified rows have documented limitations. No status may be changed by relabeling or excluding a failed prerequisite.

| Row | Required work and evidence | Dependency |
|---|---|---|
| 155 | Enumerate installed report choices, independently implement each required variant, compare headers/rows/filters on controlled fixtures | Complete variant inventory |
| 166 | Authorized provider configuration, bounded requests, failure paths, secret protection and real provider request verified | OpenAI connector reauthentication/key setup remains unresolved |
| 172 | Translate every user-facing workflow and error; preserve technical URL direction, keyboard access and saved preferences; native RTL review | Translation implementation and full string inventory |
| 177 | Sign executable and installer with trusted publisher chain; verify timestamp, signature and packaged payload | User confirmed 2026-10-29 there is no existing signing certificate/service. External trusted signing prerequisite remains open; no self-signed substitution |
| 179 | Version-specific ordered full menu/submenu inventory and accelerators, including unavailable entries and observation scope | Local licensed 24.3 observed; only selected root menus captured so far |
| 180 | Enumerate every column per installed tab/mode and map observed semantics with fixtures | Per-tab inventory and behavior evidence |
| 181 | Enumerate every installed tab filter; exercise positive, negative and unknown cases against controlled results | Per-filter inventory and fixtures |
| 182 | Read every relevant default in an isolated/default configuration; distinguish observed saved user settings from factory defaults | Controlled configuration evidence; never reset user settings to infer defaults |
| 183 | Enumerate every export variant and review output shape, encoding and limits, including absent/unknown data | Installed export evidence and independent parsers |
| 184 | Exercise licensed workflows within authorized access and document disabled/error paths | Licensed title is observed but does not prove workflow acceptance |
| 202 | Resolve the historical flagged compiler experiment through evidence-based security review, without rerunning flagged files or adding antivirus exclusions | Historical detection remains unresolved; current official NSIS packaging passes targeted scans |

Latest concrete work is recorded in each release note. The installed program and existing crawl data must remain protected by backups and byte-comparison checks during upgrades.

Row 145 is partially tested in 0.1.54 for explicit standard HTTP forms. Interactive browser forms, JavaScript/SSO/MFA and ongoing expiry detection remain unimplemented; qualification is not full equivalence.

0.1.58 adds six dedicated redirect/canonical report variants with native table/filter/CSV verification. Row 155 remains pending because the remaining report taxonomy and exact competitor outputs are not yet implemented/certified.
