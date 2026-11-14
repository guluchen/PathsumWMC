#!/usr/bin/env python3
"""Verify both shipped counter binaries and their permanent corresponding sources."""
import hashlib,json,pathlib
base=pathlib.Path(__file__).resolve().parents[1]/'web/public/engines'
manifest=json.loads((base/'PROVENANCE.json').read_text())
checked=[]
for name,cfg in manifest.items():
 for group in ['sha256','source_sha256']:
  for file,expected in cfg.get(group,{}).items():
   h=hashlib.sha256()
   with (base/file).open('rb') as f:
    for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
   assert h.hexdigest()==expected,(name,file,'Counter/source provenance mismatch')
   checked.append(file)
print('Counter provenance PASS:',', '.join(checked))
