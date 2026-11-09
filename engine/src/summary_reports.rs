use crate::{path_reports::Report, Page, Result};
use std::collections::{BTreeMap, BTreeSet};

pub const KINDS: [&str; 12] = [
    "crawl-overview", "issues-overview", "header-summary", "serp-summary",
    "insecure-content", "orphan-pages", "json-ld-findings", "json-ld-parse-errors", "json-ld-summary",
    "console-summary", "mobile-summary", "accessibility-summary",
];

pub fn load(db: &str, kind: &str) -> Result<Report> {
    if matches!(kind, "json-ld-findings" | "json-ld-parse-errors" | "json-ld-summary") {
        let audit = crate::structured_data::analyze(db, Default::default())?;
        let (headers, rows, truncated) = if kind == "json-ld-summary" {
            let summary = audit.summary;
            let rows = [
                ("Pages with JSON-LD", summary.pages_with_json_ld),
                ("JSON-LD blocks", summary.blocks),
                ("Entities", summary.entities),
                ("Parse or document errors", summary.parse_errors),
                ("Errors", summary.errors),
                ("Warnings", summary.warnings),
                ("Unvalidated terms", summary.unvalidated_terms),
            ].into_iter().map(|(metric, count)| vec![metric.into(), count.to_string()]).collect();
            (vec!["Metric", "Count"], rows, false)
        } else {
            let rows = audit.findings.into_iter().filter(|finding| {
                kind != "json-ld-parse-errors" || matches!(finding.code.as_str(), "PARSE_ERROR" | "INVALID_DOCUMENT")
            }).map(|finding| vec![
                finding.url, finding.block.to_string(), finding.pointer, finding.severity,
                finding.code, finding.term, finding.message,
            ]).collect();
            (vec!["URL", "Block", "JSON pointer", "Severity", "Code", "Term", "Message"], rows, audit.summary.truncated)
        };
        return Ok(Report {
            kind: kind.into(),
            headers: headers.into_iter().map(String::from).collect(),
            rows, truncated,
            scope: format!("Saved JSON-LD. {} {}", audit.note, audit.attribution),
        });
    }
    if kind == "orphan-pages" {
        let analysis = crate::link_analysis::project(db)?;
        let mut rows: Vec<_> = analysis.rows.into_iter().filter(|row| row.orphan_candidate)
            .take(50_001)
            .map(|row| vec![
                row.url, row.status.map(|status| status.to_string()).unwrap_or_default(),
                row.sources.join(", "), row.unique_inlinks.to_string(), row.evidence,
            ]).collect();
        let truncated = rows.len() > 50_000;
        rows.truncate(50_000);
        return Ok(Report {
            kind: kind.into(),
            headers: ["URL", "Status", "Discovery sources", "Unique inlinks", "Evidence"]
                .into_iter().map(String::from).collect(),
            rows, truncated, scope: analysis.note,
        });
    }
    build(&crate::load_project_data(db)?.pages, kind)
}

fn build(pages: &[Page], kind: &str) -> Result<Report> {
    let (headers, mut rows, scope) = match kind {
        "console-summary" | "mobile-summary" | "accessibility-summary" => {
            let (primary, secondary) = match kind {
                "console-summary" => ("Captured entries", "Errors and exceptions"),
                "mobile-summary" => ("Findings", ""),
                _ => ("Violation rules", "Rules needing review"),
            };
            let rows = pages.iter().filter_map(|page| {
                let (count, extra, error, partial) = match kind {
                    "console-summary" => (page.console_count, page.console_errors, &page.console_error, false),
                    "mobile-summary" => (page.mobile_findings, None, &page.mobile_error, page.mobile_truncated),
                    _ => (page.accessibility_violations, page.accessibility_review, &page.accessibility_error, page.accessibility_truncated),
                };
                if count.is_none() && extra.is_none() && error.is_empty() && !partial {
                    return None;
                }
                let mut row = vec![page.url.clone(), count.map(|value| value.to_string()).unwrap_or_default()];
                if !secondary.is_empty() {
                    row.push(extra.map(|value| value.to_string()).unwrap_or_default());
                }
                row.extend([error.clone(), if partial { "Partial details" } else { "" }.into()]);
                Some(row)
            }).take(50_001).collect();
            let mut headers = vec!["URL", primary];
            if !secondary.is_empty() {
                headers.push(secondary);
            }
            headers.extend(["Capture diagnostic", "Detail coverage"]);
            (headers, rows, "Saved browser observations. Pages without recorded observations are omitted. Blank counts mean unavailable; zero means a measured count of zero.")
        }
        "crawl-overview" => {
            let mut counts = BTreeMap::new();
            counts.insert(("Scope", "All URLs".to_owned()), pages.len());
            for page in pages {
                let scope = if page.internal { "Internal" } else { "External" };
                *counts.entry(("Scope", scope.to_owned())).or_default() += 1;
                let status = if page.status == 0 { "No response".to_owned() } else { page.status.to_string() };
                *counts.entry(("Response", status)).or_default() += 1;
                let media_type = page.content_type.split(';').next().unwrap_or("").trim();
                let media_type = if media_type.is_empty() { "Unknown" } else { media_type };
                *counts.entry(("Content type", media_type.to_ascii_lowercase())).or_default() += 1;
            }
            let rows = counts.into_iter().map(|((section, metric), count)| vec![section.into(), metric, count.to_string()]).collect::<Vec<_>>();
            (vec!["Section", "Metric", "URLs"], rows, "Counts use saved crawl records, including external URLs.")
        }
        "issues-overview" => {
            let mut counts = BTreeMap::<&str, usize>::new();
            for page in pages {
                for issue in page.issues.iter().map(String::as_str).collect::<BTreeSet<_>>() {
                    *counts.entry(issue).or_default() += 1;
                }
            }
            let rows = counts.into_iter().map(|(issue, count)| vec![issue.into(), count.to_string()]).collect();
            (vec!["Recorded issue", "Affected URLs"], rows, "Each recorded issue counts once per URL.")
        }
        "header-summary" => {
            let mut counts = BTreeMap::<String, usize>::new();
            for page in pages {
                for name in page.headers.keys().map(|name| name.to_ascii_lowercase()).collect::<BTreeSet<_>>() {
                    *counts.entry(name).or_default() += 1;
                }
            }
            let rows = counts.into_iter().map(|(name, count)| vec![name, count.to_string()]).collect();
            (vec!["Response header name", "URLs with recorded header"], rows, "Counts use saved header names. Header values are omitted.")
        }
        "serp-summary" => {
            let rows = pages.iter().filter(|page| {
                page.internal && page.status == 200
                    && page.content_type.split(';').next().is_some_and(|value| value.trim().eq_ignore_ascii_case("text/html"))
                    && page.html_extracted != Some(false)
            }).take(50_001).map(|page| vec![
                page.url.clone(), page.title.clone(), page.description.clone(),
                page.title.chars().count().to_string(), page.description.chars().count().to_string(),
            ]).collect();
            (vec!["URL", "Title", "Meta description", "Title characters", "Description characters"], rows,
             "Saved metadata for internal HTTP 200 HTML pages. Lengths count Unicode characters. Search engines can display different text.")
        }
        "insecure-content" => {
            let mut rows = Vec::new();
            'pages: for page in pages.iter().filter(|page| {
                url::Url::parse(&page.url).is_ok_and(|url| url.scheme() == "https")
            }) {
                let references = page.links.iter().map(|link| (link.url.as_str(), link.kind.as_str()))
                    .chain(page.images.iter().map(|(url, _)| (url.as_str(), "image")));
                let mut seen = BTreeSet::new();
                for (target, relation) in references {
                    if url::Url::parse(target).is_ok_and(|url| url.scheme() == "http")
                        && seen.insert((target, relation))
                    {
                        rows.push(vec![page.url.clone(), relation.into(), target.into()]);
                        if rows.len() == 50_001 {
                            break 'pages;
                        }
                    }
                }
            }
            (vec!["HTTPS source", "Reference type", "HTTP target"], rows,
             "Saved HTTP references from HTTPS pages. Reference type distinguishes navigation links from resources. Browser loading behavior is outside this report.")
        }
        _ => return Err("Unknown summary report".into()),
    };
    let truncated = rows.len() > 50_000;
    rows.truncate(50_000);
    Ok(Report { kind: kind.into(), headers: headers.into_iter().map(String::from).collect(), rows, truncated, scope: scope.into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_summaries_preserve_zero_and_unavailable_counts() {
        let unmeasured = Page { url: "https://example.test/unknown".into(), ..Default::default() };
        let measured = Page {
            url: "https://example.test/measured".into(),
            console_count: Some(0), console_errors: Some(0),
            mobile_error: "Audit timed out".into(),
            accessibility_violations: Some(0), accessibility_review: Some(2),
            accessibility_truncated: true,
            ..Default::default()
        };
        let pages = [unmeasured, measured];
        let console = build(&pages, "console-summary").unwrap();
        assert_eq!(console.rows.len(), 1);
        assert_eq!(&console.rows[0][1..3], ["0", "0"]);
        let mobile = build(&pages, "mobile-summary").unwrap();
        assert_eq!(&mobile.rows[0][1..3], ["", "Audit timed out"]);
        let accessibility = build(&pages, "accessibility-summary").unwrap();
        assert_eq!(&accessibility.rows[0][1..], ["0", "2", "", "Partial details"]);
    }

    #[test]
    fn insecure_report_keeps_unique_http_references_from_https_pages() {
        let mut page = crate::extract(
            "https://example.test/",
            r#"<a href="http://example.test/next">Next</a><img src="http://example.test/a.png"><img src="http://example.test/a.png"><img src="https://example.test/secure.png">"#,
            0,
        ).unwrap();
        let report = build(&[page.clone()], "insecure-content").unwrap();
        assert_eq!(report.rows.len(), 2);
        assert!(report.rows.iter().any(|row| row[2] == "http://example.test/a.png"));
        page.url = "http://example.test/".into();
        assert!(build(&[page], "insecure-content").unwrap().rows.is_empty());
    }
}
