//! Saved-source lexical analysis; no network requests or model-provider calls.
use crate::Result;
use serde::{Deserialize,Serialize};
use std::collections::{BTreeSet,HashMap,HashSet};
#[derive(Clone,Deserialize)]
#[serde(default)]
pub struct Options{pub n:usize,pub limit:usize,pub min_frequency:usize,pub min_documents:usize,pub scope:String,pub max_pages:usize,pub include_pattern:String,pub stop_words:Vec<String>,pub threshold:f64,pub shingle_size:usize}
impl Default for Options{fn default()->Self{Self{n:2,limit:1000,min_frequency:2,min_documents:1,scope:"content".into(),max_pages:0,include_pattern:String::new(),stop_words:Vec::new(),threshold:0.9,shingle_size:5}}}
#[derive(Serialize)]pub struct SourceSummary{pub processed_pages:usize,pub missing_sources:Vec<String>,pub insufficient_text:Vec<String>,pub note:String}
struct Document{url:String,tokens:Vec<String>}
fn validate(options:&Options)->Result<()>{
 if !(1..=6).contains(&options.n)||!(1..=10).contains(&options.shingle_size)||!(1..=10000).contains(&options.limit)||options.max_pages>1000000||!options.threshold.is_finite()||options.threshold<=0.0||options.threshold>1.0{return Err("Use N=1–6, shingles=1–10, result limit=1–10000, maximum pages=0 (all matching) or at most 1000000, similarity greater than 0 and at most 1".into())}
 if !["content","body"].contains(&options.scope.as_str())||options.include_pattern.len()>8192||options.stop_words.len()>1000||options.stop_words.iter().any(|word|word.len()>200){return Err("Invalid text-analysis scope, include expression or stop words".into())}Ok(())
}
fn tokenize(text:&str)->Vec<String>{
 static TOKEN:std::sync::OnceLock<regex::Regex>=std::sync::OnceLock::new();
 TOKEN.get_or_init(||regex::Regex::new(r"[\p{L}\p{N}][\p{L}\p{N}\p{M}]*(?:['’׳״־-][\p{L}\p{N}][\p{L}\p{N}\p{M}]*)*").unwrap()).find_iter(text).map(|word|word.as_str().to_lowercase()).collect()
}
fn load(db:&str,options:&Options)->Result<(Vec<Document>,SourceSummary)>{
 validate(options)?;let include=regex::Regex::new(if options.include_pattern.is_empty(){".*"}else{&options.include_pattern})?;
 let mut connection=crate::open_read(db)?;let transaction=connection.transaction()?;
 let mut statement=transaction.prepare("SELECT json FROM pages ORDER BY url")?;
 let pages=statement.query_map([],|row|row.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?.into_iter().map(|json|serde_json::from_str::<crate::Page>(&json)).collect::<std::result::Result<Vec<_>,_>>()?;
 let pages:Vec<_>=pages.into_iter().filter(|page|page.internal&&(200..300).contains(&page.status)&&page.content_type.contains("html")&&include.is_match(&page.url)).collect();
 if options.max_pages>0&&pages.len()>options.max_pages{return Err(format!("{} eligible HTML pages exceed the analysis maximum of {}; narrow the URL include expression, increase the cap or use 0 for all matching pages. Crawl size is not limited.",pages.len(),options.max_pages).into())}
 let tables:BTreeSet<String>={let mut query=transaction.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;let names=query.query_map([],|row|row.get(0))?.collect::<std::result::Result<BTreeSet<_>,_>>()?;names};
 let mut documents=Vec::new();let mut summary=SourceSummary{processed_pages:0,missing_sources:Vec::new(),insufficient_text:Vec::new(),note:"Lexical analysis uses stored decoded HTML (rendered DOM when recorded), Unicode letter/number/mark tokens and Unicode lowercase. Content scope excludes nav/footer/script/style/template/form-value containers; CSS visibility is not evaluated. Missing sources are reported rather than fetched. Analysis resource limits: 100 MiB HTML and 2 million tokens. Page cap 0 includes all matching pages within those resource limits; URL include can select a smaller scope.".into()};
 let mut bytes=0;let mut token_count=0;
 for page in pages{
  let table=if page.rendered{"rendered_documents"}else{"documents"};
  let html=if tables.contains(table){use rusqlite::OptionalExtension;transaction.query_row(&format!("SELECT html FROM {table} WHERE url=?1"),[&page.url],|row|row.get::<_,String>(0)).optional()?}else{None};
  let Some(html)=html else{summary.missing_sources.push(page.url);continue};
  bytes+=html.len();if bytes>100*1024*1024{return Err("Text analysis exceeds 100 MiB stored HTML; narrow the URL include scope".into())}
  let mut parsed=scraper::Html::parse_document(&html);
  let controls:Vec<_>=parsed.select(&scraper::Selector::parse("textarea,select").unwrap()).map(|element|element.id()).collect();for id in controls{if let Some(mut node)=parsed.tree.get_mut(id){node.detach()}}
  let selector=scraper::Selector::parse("body").unwrap();
  let text=parsed.select(&selector).next().map(|body|crate::extracted_text(body,options.scope=="content")).unwrap_or_default();
  let tokens=tokenize(&text);token_count+=tokens.len();if token_count>2000000{return Err("Text analysis exceeds two million tokens; narrow the URL include scope".into())}
  documents.push(Document{url:page.url,tokens});
 }
 summary.processed_pages=documents.len();if documents.is_empty(){return Err("No stored successful HTML sources match this scope. Load a crawl with saved HTML or run a new crawl with Store HTML enabled.".into())}Ok((documents,summary))
}
#[derive(Debug,Serialize)]pub struct NgramRow{pub phrase:String,pub occurrences:usize,pub documents:usize}
#[derive(Serialize)]pub struct NgramReport{pub summary:SourceSummary,pub n:usize,pub rows:Vec<NgramRow>,pub matching_phrases:usize,pub truncated:bool}
fn count_ngrams(documents:&[Document],options:&Options)->Result<Vec<NgramRow>>{
 let stop:HashSet<_>=options.stop_words.iter().map(|word|word.to_lowercase()).collect();let mut counts:HashMap<String,(usize,usize)>=HashMap::new();
 for document in documents{let mut seen=HashSet::new();for window in document.tokens.windows(options.n){if window.iter().any(|token|stop.contains(token)){continue}let phrase=window.join(" ");let entry=counts.entry(phrase.clone()).or_default();entry.0+=1;if seen.insert(phrase){entry.1+=1}if counts.len()>250000{return Err("N-gram vocabulary exceeds 250,000 phrases; narrow the URL scope or reduce N".into())}}}
 let mut rows:Vec<_>=counts.into_iter().filter(|(_,counts)|counts.0>=options.min_frequency&&counts.1>=options.min_documents).map(|(phrase,(occurrences,documents))|NgramRow{phrase,occurrences,documents}).collect();rows.sort_by(|a,b|b.occurrences.cmp(&a.occurrences).then(b.documents.cmp(&a.documents)).then(a.phrase.cmp(&b.phrase)));Ok(rows)
}
pub fn ngrams(db:&str,options:Options)->Result<NgramReport>{let(documents,summary)=load(db,&options)?;let mut rows=count_ngrams(&documents,&options)?;let matching_phrases=rows.len();rows.truncate(options.limit);let truncated=rows.len()<matching_phrases;Ok(NgramReport{summary,n:options.n,rows,matching_phrases,truncated})}
#[derive(Debug,Serialize)]pub struct DuplicateRow{pub left:String,pub right:String,pub similarity:f64,pub shared_shingles:usize,pub union_shingles:usize}
#[derive(Serialize)]pub struct DuplicateReport{pub summary:SourceSummary,pub rows:Vec<DuplicateRow>,pub matching_pairs:usize,pub truncated:bool,pub threshold:f64,pub shingle_size:usize,pub method:String}
fn shingles(tokens:&[String],size:usize)->HashSet<u64>{tokens.windows(size).map(|window|{let mut hash=0xcbf29ce484222325u64;for token in window{for byte in token.bytes().chain(std::iter::once(0)){hash^=byte as u64;hash=hash.wrapping_mul(0x100000001b3)}}hash}).collect()}
fn duplicate_pairs(documents:&[Document],options:&Options)->Result<(Vec<DuplicateRow>,Vec<String>)>{
 let sets:Vec<_>=documents.iter().map(|document|shingles(&document.tokens,options.shingle_size)).collect();let insufficient=documents.iter().zip(&sets).filter(|(_,set)|set.is_empty()).map(|(document,_)|document.url.clone()).collect();
 let mut inverted:HashMap<u64,Vec<usize>>=HashMap::new();for(index,set)in sets.iter().enumerate(){for shingle in set{inverted.entry(*shingle).or_default().push(index)}}
 let mut shared:HashMap<(usize,usize),usize>=HashMap::new();let mut budget=0usize;
 for postings in inverted.values(){for position in 0..postings.len(){for &right in &postings[position+1..]{budget+=1;if budget>10000000{return Err("Near-duplicate shared-shingle budget exceeded; narrow the URL scope".into())}*shared.entry((postings[position],right)).or_default()+=1}}}
 let mut rows=Vec::new();for((left,right),intersection)in shared{let union=sets[left].len()+sets[right].len()-intersection;let similarity=intersection as f64/union as f64;if similarity>=options.threshold{rows.push(DuplicateRow{left:documents[left].url.clone(),right:documents[right].url.clone(),similarity,shared_shingles:intersection,union_shingles:union})}}
 rows.sort_by(|a,b|b.similarity.total_cmp(&a.similarity).then(a.left.cmp(&b.left)).then(a.right.cmp(&b.right)));Ok((rows,insufficient))
}
pub fn duplicates(db:&str,options:Options)->Result<DuplicateReport>{let(documents,mut summary)=load(db,&options)?;let(mut rows,insufficient)=duplicate_pairs(&documents,&options)?;summary.insufficient_text=insufficient;let matching_pairs=rows.len();rows.truncate(options.limit);let truncated=rows.len()<matching_pairs;Ok(DuplicateReport{summary,rows,matching_pairs,truncated,threshold:options.threshold,shingle_size:options.shingle_size,method:"Lexical Jaccard similarity of unique word shingles (64-bit FNV-1a hashes); exact shared-set ratios for generated candidates, not semantic similarity and not a certification of Frog's similarity metric. Only positive-overlap pairs are candidates; zero is not an accepted similarity threshold. Comparison budget: 10 million shared-shingle pair updates.".into()})}
#[cfg(test)]mod tests{
 use super::*;
 fn doc(url:&str,text:&str)->Document{Document{url:url.into(),tokens:tokenize(text)}}
 #[test]fn unicode_ngrams_count_occurrences_and_documents(){let docs=[doc("a","שלום עולם שלום עולם"),doc("b","שלום עולם חדש")];let options=Options{min_frequency:1,..Default::default()};let rows=count_ngrams(&docs,&options).unwrap();let row=&rows[0];assert_eq!(row.phrase,"שלום עולם");assert_eq!(row.occurrences,3);assert_eq!(row.documents,2);assert_eq!(tokenize("שָׁלוֹם Hello HELLO"),vec!["שָׁלוֹם","hello","hello"]);}
 #[test]fn stop_words_do_not_invent_adjacent_phrases(){let rows=count_ngrams(&[doc("a","one and two one two")],&Options{min_frequency:1,stop_words:vec!["AND".into()],..Default::default()}).unwrap();assert_eq!(rows.iter().find(|row|row.phrase=="one two").unwrap().occurrences,1);assert!(!rows.iter().any(|row|row.phrase.contains("and")));}
 #[test]fn duplicate_pairs_use_set_jaccard_and_report_short_documents(){let docs=[doc("a","one two three four"),doc("b","one two three five"),doc("c","unrelated totally different words"),doc("d","short")];let options=Options{shingle_size:2,threshold:0.5,..Default::default()};let(rows,short)=duplicate_pairs(&docs,&options).unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].similarity,0.5);assert_eq!(rows[0].shared_shingles,2);assert_eq!(rows[0].union_shingles,4);assert_eq!(short,vec!["d"]);}
 #[test]fn duplicate_repetition_is_not_extra_similarity_and_identical_is_one(){let docs=[doc("a","שלום עולם שלום עולם"),doc("b","שלום עולם שלום עולם שלום עולם")];let(rows,_)=duplicate_pairs(&docs,&Options{shingle_size:2,..Default::default()}).unwrap();assert_eq!(rows[0].similarity,1.0);assert_eq!(rows[0].shared_shingles,2);}
 #[test]fn invalid_options_rejected(){assert!(validate(&Options{n:0,..Default::default()}).is_err());assert!(validate(&Options{threshold:f64::NAN,..Default::default()}).is_err());assert!(validate(&Options{max_pages:1000001,..Default::default()}).is_err());}
 #[test]fn saved_source_snapshot_prefers_rendered_and_reports_missing_sources(){
  let folder=std::env::temp_dir().join(format!("fitig-text-analysis-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&folder).unwrap();let path=folder.join("source.sqlite");let db=path.to_str().unwrap();
  {let connection=crate::open(db).unwrap();connection.execute("INSERT INTO metadata VALUES('seed','https://e.test/')",[]).unwrap();for name in ["a","b","c"]{let page=crate::Page{url:format!("https://e.test/{name}"),status:200,content_type:"text/html".into(),rendered:name=="b",..Default::default()};connection.execute("INSERT INTO pages VALUES(?1,?2)",rusqlite::params![page.url,serde_json::to_string(&page).unwrap()]).unwrap();}
   connection.execute("INSERT INTO documents VALUES('https://e.test/a',?1)",["<body><nav>menu noise</nav><main>שלום עולם שלום עולם חדש</main><textarea>private control</textarea></body>"]).unwrap();connection.execute("INSERT INTO documents VALUES('https://e.test/b','<body>raw unrelated content</body>')",[]).unwrap();connection.execute("INSERT INTO rendered_documents VALUES('https://e.test/b','<body>שלום עולם שלום עולם חדש</body>')",[]).unwrap();}
  let before=std::fs::read(&path).unwrap();let report=ngrams(db,Options::default()).unwrap();assert_eq!(report.summary.processed_pages,2);assert_eq!(report.summary.missing_sources,vec!["https://e.test/c"]);assert_eq!(report.rows[0].phrase,"שלום עולם");assert_eq!(report.rows[0].occurrences,4);assert_eq!(report.rows[0].documents,2);assert!(!report.rows.iter().any(|row|row.phrase.contains("private")||row.phrase.contains("menu")));
  let near=duplicates(db,Options::default()).unwrap();assert_eq!(near.rows.len(),1);assert_eq!(near.rows[0].similarity,1.0);
  assert!(ngrams(db,Options{max_pages:1,..Default::default()}).is_err());assert!(ngrams(db,Options{include_pattern:"/c$".into(),..Default::default()}).is_err());assert_eq!(ngrams(db,Options{include_pattern:"/a$".into(),max_pages:1,..Default::default()}).unwrap().summary.processed_pages,1);assert_eq!(std::fs::read(&path).unwrap(),before);std::fs::remove_dir_all(folder).unwrap();
 }
}
