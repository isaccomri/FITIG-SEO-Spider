//! Similarity over explicitly imported embeddings. No model, network or API use.
use crate::{Page, Result, Url};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
const CORPUS: &str = "semantic_corpus_v1";
const REPORT: &str = "semantic_report_v1";
const MAX_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document { pub url: String, pub vector: Vec<f64> }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus { pub version: u32, pub model: String, pub documents: Vec<Document> }
#[derive(Debug, Serialize)]
pub struct Summary { pub model: String, pub dimensions: usize, pub documents: usize, pub sha256: String, pub urls: Vec<String> }
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query { pub model: String, pub query_vector: Option<Vec<f64>>, pub query_url: Option<String>, pub label: String, pub top_k: usize, pub minimum_score: f64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Row { pub url: String, pub title: String, pub score: f64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report { pub version: u32, pub model: String, pub dimensions: usize, pub corpus_sha256: String, pub documents: usize, pub query_kind: String, pub query_label: String, pub top_k: usize, pub minimum_score: f64, pub matched: usize, pub rows: Vec<Row>, pub note: String }
#[derive(Serialize)]
pub struct State { pub corpus: Option<Summary>, pub report: Option<Report> }
fn model(value: &str) -> Result<()> { if value.trim().is_empty() || value.trim() != value || value.len() > 128 || value.chars().any(char::is_control) { return Err("Use a model identifier of 1–128 bytes without control characters or surrounding whitespace".into()); } Ok(()) }
fn identity(raw: &str) -> Result<String> { if raw.len() > 8192 { return Err("Embedding URL exceeds 8192 bytes".into()); } let url = Url::parse(raw)?; if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() { return Err("Embedding URLs must be absolute HTTP(S), without credentials or fragments".into()); } Ok(url.to_string()) }
fn normalize(vector: &[f64]) -> Result<Vec<f64>> {
 if !(2..=4096).contains(&vector.len()) || vector.iter().any(|v| !v.is_finite()) { return Err("Vectors require 2–4096 finite numbers".into()); }
 let scale = vector.iter().fold(0.0_f64, |a, b| a.max(b.abs())); if scale == 0.0 { return Err("Zero vectors are not valid embeddings".into()); }
 let norm = vector.iter().map(|v| (v / scale).powi(2)).sum::<f64>().sqrt(); Ok(vector.iter().map(|v| (v / scale) / norm).collect())
}
fn known(pages: &[Page]) -> Result<BTreeMap<String, &Page>> { pages.iter().map(|p| Ok((identity(&p.url)?, p))).collect() }
fn validate_corpus(corpus: &mut Corpus, pages: &[Page], imported: bool) -> Result<()> {
 model(&corpus.model)?; if corpus.version != 1 || corpus.documents.is_empty() || corpus.documents.len() > 10000 { return Err("Embedding corpus version 1 supports 1–10,000 recorded URLs".into()); }
 let known = known(pages)?; let mut seen = BTreeSet::new(); let dimensions = corpus.documents[0].vector.len(); let mut components = 0usize;
 for row in &mut corpus.documents { components += row.vector.len(); if components > 1_000_000 { return Err("Embedding corpus exceeds one million numeric components".into()); } if row.vector.len() != dimensions { return Err("All document vectors must have equal dimensions".into()); }
  let url = identity(&row.url)?; let page = known.get(&url).ok_or("Each embedding URL must exist in the current crawl")?; if !seen.insert(url) { return Err("Duplicate embedding URL".into()); }
  let normalized = normalize(&row.vector)?; if imported { row.url = page.url.clone(); row.vector = normalized; } else if row.vector.iter().zip(normalized).any(|(a,b)| (a-b).abs() > 1e-8) { return Err("Saved embedding vectors are not normalized; reimport the corpus".into()); }
 }
 if imported { corpus.documents.sort_by(|a,b| a.url.cmp(&b.url)); } Ok(())
}
fn summary(corpus: &Corpus, raw: &str) -> Summary { Summary { model: corpus.model.clone(), dimensions: corpus.documents[0].vector.len(), documents: corpus.documents.len(), sha256: format!("{:x}", Sha256::digest(raw.as_bytes())), urls: corpus.documents.iter().map(|r| r.url.clone()).collect() } }
fn write(db: &str) -> Result<rusqlite::Connection> { drop(crate::open_read(db)?); Ok(rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?) }
fn read_corpus(db: &str, pages: &[Page]) -> Result<Option<(Corpus, String)>> {
 let connection = crate::open_read(db)?; let raw: Option<String> = connection.query_row("SELECT value FROM metadata WHERE key=?1", [CORPUS], |r| r.get(0)).optional()?;
 raw.map(|raw| { if raw.len() > MAX_BYTES { return Err("Saved embeddings exceed 16 MiB".into()); } let mut corpus: Corpus = serde_json::from_str(&raw)?; validate_corpus(&mut corpus, pages, false)?; Ok((corpus, raw)) }).transpose()
}
pub fn import(db: &str, raw: &str) -> Result<Summary> {
 if raw.len() > MAX_BYTES { return Err("Embedding import exceeds 16 MiB".into()); } let mut corpus: Corpus = serde_json::from_str(raw).map_err(|_| "Invalid embedding corpus JSON; input omitted")?;
 validate_corpus(&mut corpus, &crate::load_pages(db)?, true)?; let raw = serde_json::to_string(&corpus)?; if raw.len() > MAX_BYTES { return Err("Normalized embedding corpus exceeds 16 MiB".into()); }
 let result = summary(&corpus, &raw); let mut connection = write(db)?; let transaction = connection.transaction()?; transaction.execute("INSERT OR REPLACE INTO metadata(key,value) VALUES(?1,?2)", params![CORPUS, raw])?; transaction.execute("DELETE FROM metadata WHERE key=?1", [REPORT])?; transaction.commit()?; Ok(result)
}
fn rank(corpus: &Corpus, raw: &str, pages: &[Page], query: Query) -> Result<Report> {
 model(&query.model)?; if query.model != corpus.model { return Err("Query and document vectors must declare the same model identifier".into()); }
 if query.label.len() > 512 || query.label.chars().any(char::is_control) || !(1..=200).contains(&query.top_k) || !query.minimum_score.is_finite() || !(-1.0..=1.0).contains(&query.minimum_score) { return Err("Use top K 1–200, minimum cosine -1 to 1 and a label up to 512 bytes without control characters".into()); }
 let (vector, excluded, kind) = match (query.query_vector, query.query_url) {
  (Some(vector), None) => (normalize(&vector)?, None, "Imported query vector"),
  (None, Some(url)) => { let identity = identity(&url)?; let row = corpus.documents.iter().find(|r| crate::Url::parse(&r.url).map(|u| u.as_str() == identity).unwrap_or(false)).ok_or("Query URL has no imported vector")?; (row.vector.clone(), Some(row.url.clone()), "Similar recorded URL (self excluded)") },
  _ => return Err("Choose exactly one query vector or recorded query URL".into()),
 };
 if vector.len() != corpus.documents[0].vector.len() { return Err("Query dimensions differ from the imported corpus".into()); }
 let pages: BTreeMap<_,_> = pages.iter().map(|p| (p.url.as_str(),p)).collect(); let mut rows = Vec::new();
 for document in &corpus.documents { if excluded.as_ref() == Some(&document.url) { continue; } let score = vector.iter().zip(&document.vector).map(|(a,b)| a*b).sum::<f64>().clamp(-1.0,1.0); if score >= query.minimum_score { let page = pages.get(document.url.as_str()).ok_or("Embedding URL is no longer recorded in the crawl")?; rows.push(Row { url: document.url.clone(), title: page.title.chars().take(512).collect(), score }); } }
 rows.sort_by(|a,b| b.score.total_cmp(&a.score).then_with(|| a.url.cmp(&b.url))); let matched = rows.len(); rows.truncate(query.top_k);
 Ok(Report { version:1, model:corpus.model.clone(), dimensions:vector.len(), corpus_sha256:summary(corpus,raw).sha256, documents:corpus.documents.len(), query_kind:kind.into(), query_label:query.label, top_k:query.top_k, minimum_score:query.minimum_score, matched, rows, note:"Cosine similarity of explicitly imported embeddings, not generated or independently verified by FITIG. Both inputs must come from compatible embedding models; matching declared identifiers cannot prove provenance. Scores are not probabilities or a certification of relevance. No API, model download or network request is performed. Import and Search explicitly save their corpus/report into this project; reading prior results is read-only. Query vectors are not retained in the saved report.".into() })
}
pub fn search(db: &str, query: Query) -> Result<Report> {
 let pages = crate::load_pages(db)?; let (corpus, raw) = read_corpus(db, &pages)?.ok_or("Import an embedding corpus for this crawl first")?; let report = rank(&corpus, &raw, &pages, query)?;
 let raw = serde_json::to_string(&report)?; if raw.len() > 2*1024*1024 { return Err("Search report exceeds 2 MiB; lower top K".into()); } write(db)?.execute("INSERT OR REPLACE INTO metadata(key,value) VALUES(?1,?2)", params![REPORT, raw])?; Ok(report)
}
pub fn load(db: &str) -> Result<State> {
 let pages = crate::load_pages(db)?; let corpus = read_corpus(db, &pages)?; let connection = crate::open_read(db)?; let raw: Option<String> = connection.query_row("SELECT value FROM metadata WHERE key=?1",[REPORT],|r|r.get(0)).optional()?;
 let report = raw.map(|raw| -> Result<Report> { if raw.len()>2*1024*1024 { return Err("Saved semantic report exceeds 2 MiB".into()); } let report: Report=serde_json::from_str(&raw)?; let (source,encoded)=corpus.as_ref().ok_or("Saved report has no corpus")?; if report.version!=1 || report.model!=source.model || report.corpus_sha256!=summary(source,encoded).sha256 || report.dimensions!=source.documents[0].vector.len() || report.documents!=source.documents.len() || !(1..=200).contains(&report.top_k) || !report.minimum_score.is_finite() || !(-1.0..=1.0).contains(&report.minimum_score) || report.query_label.len()>512 || report.query_label.chars().any(char::is_control) || report.matched>report.documents || report.rows.len()!=report.matched.min(report.top_k) || report.rows.iter().map(|r|&r.url).collect::<BTreeSet<_>>().len()!=report.rows.len() || report.rows.iter().any(|r| !r.score.is_finite() || !(-1.0..=1.0).contains(&r.score) || r.score<report.minimum_score || !source.documents.iter().any(|p|p.url==r.url)) { return Err("Saved semantic report is inconsistent with this project; run Search again".into()); } Ok(report) }).transpose()?;
 Ok(State { corpus:corpus.as_ref().map(|(corpus,raw)| summary(corpus,raw)), report })
}

#[cfg(test)] mod tests {
 use super::*;
 struct Fixture { folder:std::path::PathBuf, db:String }
 impl Fixture { fn new(legacy:bool)->Self { let folder=std::env::temp_dir().join(format!("fitig-semantic-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&folder).unwrap();let db=folder.join("project.sqlite").to_string_lossy().into_owned();let c=if legacy{let c=rusqlite::Connection::open(&db).unwrap();c.execute_batch("CREATE TABLE pages(url TEXT PRIMARY KEY,json TEXT);CREATE TABLE frontier(url TEXT PRIMARY KEY,depth INTEGER,state TEXT);CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT);").unwrap();c}else{crate::open(&db).unwrap()};c.execute("INSERT INTO metadata VALUES('seed','https://e.test/')",[]).unwrap();for suffix in ["a","b","c","d","עברית"]{let page=Page{url:Url::parse(&format!("https://e.test/{suffix}")).unwrap().to_string(),title:format!("כותרת {suffix}"),..Default::default()};c.execute("INSERT INTO pages VALUES(?1,?2)",params![page.url,serde_json::to_string(&page).unwrap()]).unwrap();}Self{folder,db} } }
 impl Drop for Fixture {fn drop(&mut self){assert!(self.folder.starts_with(std::env::temp_dir()));std::fs::remove_dir_all(&self.folder).unwrap();}}
 fn corpus()->String {serde_json::json!({"version":1,"model":"owned-test-model","documents":[{"url":"https://e.test/a","vector":[1,0]},{"url":"https://e.test/b","vector":[4,3]},{"url":"https://e.test/c","vector":[0,1]},{"url":"https://e.test/d","vector":[-1,0]},{"url":"https://e.test/עברית","vector":[4,3]}]}).to_string()}
 fn query()->Query {Query{model:"owned-test-model".into(),query_vector:Some(vec![1.0,0.0]),query_url:None,label:"שאילתה".into(),top_k:200,minimum_score:-1.0}}
 #[test]fn independent_cosines_order_threshold_and_top_k(){let f=Fixture::new(false);let summary=import(&f.db,&corpus()).unwrap();assert_eq!((summary.documents,summary.dimensions),(5,2));let report=search(&f.db,query()).unwrap();assert_eq!(report.rows.iter().map(|r|r.score).collect::<Vec<_>>(),vec![1.0,0.8,0.8,0.0,-1.0]);assert!(report.rows[1].url<report.rows[2].url);let mut q=query();q.top_k=1;q.minimum_score=0.5;let report=search(&f.db,q).unwrap();assert_eq!(report.matched,3);assert_eq!(report.rows.len(),1);assert_eq!(report.rows[0].url,"https://e.test/a");}
 #[test]fn similar_url_excludes_itself_and_unicode_identity_matches(){let f=Fixture::new(false);import(&f.db,&corpus()).unwrap();let mut q=query();q.query_vector=None;q.query_url=Some("https://e.test/עברית".into());let r=search(&f.db,q).unwrap();assert_eq!(r.rows.len(),4);assert!(!r.rows.iter().any(|r|r.url.contains("%D7")));assert_eq!(r.rows[0].url,"https://e.test/b");assert!((r.rows[0].score-1.0).abs()<1e-12);}
 #[test]fn rejects_incompatible_or_ambiguous_queries_without_replacing_report(){let f=Fixture::new(false);import(&f.db,&corpus()).unwrap();search(&f.db,query()).unwrap();let before=std::fs::read(&f.db).unwrap();for kind in 0..7 {let mut q=query();match kind {0=>q.model="other-model".into(),1=>q.query_vector=Some(vec![1.0,0.0,0.0]),2=>q.query_vector=Some(vec![0.0,0.0]),3=>q.query_url=Some("https://e.test/a".into()),4=>q.minimum_score=f64::NAN,5=>q.top_k=201,_=>q.query_vector=None};assert!(search(&f.db,q).is_err());assert_eq!(std::fs::read(&f.db).unwrap(),before);} }
 #[test]fn invalid_imports_preserve_prior_corpus_and_report(){let f=Fixture::new(false);import(&f.db,&corpus()).unwrap();search(&f.db,query()).unwrap();let before=std::fs::read(&f.db).unwrap();for kind in 0..7 {let mut v:serde_json::Value=serde_json::from_str(&corpus()).unwrap();match kind{0=>v["documents"][0]["url"]="https://outside.test/".into(),1=>v["documents"][0]["vector"]=serde_json::json!([0,0]),2=>v["documents"][0]["vector"]=serde_json::json!([1]),3=>v["documents"][0]["url"]="https://e.test/b".into(),4=>v["documents"][0]["url"]="https://u:p@e.test/a".into(),5=>v["version"]=2.into(),_=>v["model"]="".into()};assert!(import(&f.db,&v.to_string()).is_err());assert_eq!(std::fs::read(&f.db).unwrap(),before);}assert!(import(&f.db,&" ".repeat(MAX_BYTES+1)).is_err());assert_eq!(std::fs::read(&f.db).unwrap(),before);}
 #[test]fn saved_results_read_only_on_legacy_project_and_new_import_clears_them(){let f=Fixture::new(true);let before=std::fs::read(&f.db).unwrap();assert!(load(&f.db).unwrap().corpus.is_none());assert_eq!(std::fs::read(&f.db).unwrap(),before);import(&f.db,&corpus()).unwrap();search(&f.db,query()).unwrap();let before=std::fs::read(&f.db).unwrap();assert_eq!(load(&f.db).unwrap().report.unwrap().rows.len(),5);assert_eq!(std::fs::read(&f.db).unwrap(),before);assert_eq!(crate::open_read(&f.db).unwrap().query_row("PRAGMA user_version",[],|r|r.get::<_,u32>(0)).unwrap(),0);import(&f.db,&corpus()).unwrap();assert!(load(&f.db).unwrap().report.is_none());}
 #[test]fn finite_extremes_normalize_without_overflow_and_missing_db_stays_missing(){assert_eq!(normalize(&[1e308,0.0]).unwrap(),vec![1.0,0.0]);assert_eq!(normalize(&[1e-308,0.0]).unwrap(),vec![1.0,0.0]);assert!(normalize(&[f64::INFINITY,1.0]).is_err());let f=Fixture::new(false);let missing=f.folder.join("missing.sqlite");assert!(load(missing.to_str().unwrap()).is_err());assert!(!missing.exists());}
}
#[cfg(test)] mod limit_tests {
 use super::*;
 #[test] fn corpus_component_document_and_dimension_bounds_are_enforced() {
  assert!(normalize(&vec![1.0;4096]).is_ok()); assert!(normalize(&vec![1.0;4097]).is_err());
  let pages:Vec<_>=(0..250).map(|i|Page{url:format!("https://e.test/{i}"),..Default::default()}).collect();
  let mut corpus=Corpus{version:1,model:"bounded-model".into(),documents:pages.iter().map(|p|Document{url:p.url.clone(),vector:vec![1.0;4096]}).collect()};
  assert!(validate_corpus(&mut corpus,&pages,true).unwrap_err().to_string().contains("one million"));
  corpus.documents=vec![Document{url:"https://e.test/0".into(),vector:vec![1.0,0.0]};10001];
  assert!(validate_corpus(&mut corpus,&pages,true).unwrap_err().to_string().contains("10,000"));
 }
}
