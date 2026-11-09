use crate::{path_reports::Report, Page, Result};
use std::collections::{BTreeMap, BTreeSet};

pub const KINDS: [&str; 4] = ["crawl-overview", "issues-overview", "header-summary", "serp-summary"];

pub fn load(db: &str, kind: &str) -> Result<Report> {
    build(&crate::load_project_data(db)?.pages, kind)
}

fn build(pages: &[Page], kind: &str) -> Result<Report> {
    let (headers, mut rows, scope) = match kind {
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
        _ => return Err("Unknown summary report".into()),
    };
    let truncated = rows.len() > 50_000;
    rows.truncate(50_000);
    Ok(Report { kind: kind.into(), headers: headers.into_iter().map(String::from).collect(), rows, truncated, scope: scope.into() })
}
