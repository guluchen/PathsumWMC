#!/usr/bin/env python3
"""Record branch-only candidate bytes; never relabel permanent-release assets."""
import hashlib,json,pathlib,sys
base=pathlib.Path('web/public/engines');p=base/'PROVENANCE.json';d=json.loads(p.read_text());c=d['gpmc']
c['sha256']={n:hashlib.sha256((base/n).read_bytes()).hexdigest() for n in ['gpmc.js','gpmc.wasm']}
c['source_sha256']={n:hashlib.sha256((base/n).read_bytes()).hexdigest() for n in c['source']}
c.pop('release',None);c['build_run']='https://github.com/guluchen/PathsumWMC/actions/runs/'+sys.argv[1]
c['adapter']='Exact dyadic complex mantissas with a general GMP rational fallback; counting/search unchanged.'
p.write_text(json.dumps(d,indent=2)+'\n')
