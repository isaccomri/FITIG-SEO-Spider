"""Independent standard-library verification of real crawl report files."""
import csv,json,sys,zipfile,xml.etree.ElementTree as E
from pathlib import Path
folder=Path(sys.argv[1]);expected=json.loads((folder/'expected.json').read_text(encoding='utf-8'))
rows=list(csv.DictReader((folder/'crawl.csv').open(encoding='utf-8-sig',newline='')))
assert len(rows)==expected['count'];root=next(row for row in rows if row['URL']==expected['origin']+'/')
assert root['Title']==expected['title'] and root['Status']=='200'
hebrew=next(row for row in rows if row['URL']==expected['hebrew'])
assert 'עברית' in hebrew['URL (Decoded)'] and '%D7' in hebrew['URL']
assert (folder/'crawl.csv').read_bytes().startswith(b'\xef\xbb\xbf')
html=(folder/'crawl.html').read_text(encoding='utf-8')
assert '<meta charset=utf-8>' in html and '&lt;script&gt;' in html and '<script>' not in html
assert 'שלום' in html and 'עברית' in html
root_xml=E.parse(folder/'crawl.xml').getroot()
locations=[node.text for node in root_xml.iter() if node.tag.endswith('loc')]
assert len(locations)==8 and expected['origin']+'/oversize' not in locations
assert expected['origin']+'/missing' in locations and expected['origin']+'/broken' not in locations
ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
with zipfile.ZipFile(folder/'crawl.xlsx') as archive:
 strings=E.fromstring(archive.read('xl/sharedStrings.xml'))
 texts=[''.join(node.itertext()) for node in strings.findall('s:si',ns)]
 sheet=E.fromstring(archive.read('xl/worksheets/sheet1.xml'))
 cells={cell.attrib['r']:cell for cell in sheet.findall('.//s:c',ns)}
 def value(cell):
  raw=cell.find('s:v',ns).text
  return texts[int(raw)] if cell.attrib.get('t')=='s' else raw
 root_row=next(int(key[1:]) for key,cell in cells.items() if key.startswith('A') and value(cell)==expected['origin']+'/')
 assert cells[f'C{root_row}'].attrib['t']=='s' and value(cells[f'C{root_row}'])==expected['title']
 assert cells[f'B{root_row}'].attrib.get('t') not in ('s','inlineStr') and value(cells[f'B{root_row}'])=='200'
 for column in ('F','G'):assert cells[f'{column}{root_row}'].attrib.get('t') not in ('s','inlineStr')
 assert not sheet.findall('.//s:f',ns), 'Fixture formula-like titles must remain text'
 assert len(sheet.findall('.//s:row',ns))==expected['count']+1
print(json.dumps({'csv':True,'html':True,'xlsx':True,'sitemap':True,'rows':len(rows),'formula_title_is_text':True}))
