from pathlib import Path
import json,hashlib,argparse
parser=argparse.ArgumentParser(description="Derive the licensed Schema.org 30.1 name inventory from its official downloaded JSON-LD; never performs network requests.")
parser.add_argument("--source",type=Path,required=True)
parser.add_argument("--out",type=Path,default=Path(__file__).resolve().parents[1]/"engine/data/schema-vocabulary-30.1.json")
args=parser.parse_args()
source=args.source
assert hashlib.sha256(source.read_bytes()).hexdigest()=='0a4b8c4c910fc831ec8695196510973390906a5ee34dc6b862dfec4d21a12f74','Expected the verified official Schema.org 30.1 vocabulary'
data=json.loads(source.read_text(encoding='utf-8'));types=[];properties=[];superseded={}
for node in data['@graph']:
 ident=node.get('@id','')
 if not ident.startswith('schema:'):continue
 name=ident.removeprefix('schema:');kind=node.get('@type',[]);kind=[kind] if isinstance(kind,str) else kind
 if 'rdfs:Class' in kind:types.append(name)
 if 'rdf:Property' in kind:properties.append(name)
 replacement=node.get('schema:supersededBy',[]);replacement=[replacement] if isinstance(replacement,dict) else replacement
 values=[item.get('@id','').removeprefix('schema:') for item in replacement if isinstance(item,dict)]
 if values:superseded[name]=values
out={'version':'30.1','source':'https://schema.org/version/30.1/schemaorg-current-https.jsonld','source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'retrieved':'2026-10-18','license':'https://creativecommons.org/licenses/by-sa/3.0/','attribution':'Schema.org sponsors: Google, Microsoft, Yahoo and Yandex. Derived term-name inventory; definitions/comments omitted.','types':sorted(set(types)),'properties':sorted(set(properties)),'superseded':superseded}
args.out.write_text(json.dumps(out,ensure_ascii=False,separators=(',',':'))+'\n',encoding='utf-8');print(json.dumps({key:len(out[key]) for key in ['types','properties','superseded']}))
