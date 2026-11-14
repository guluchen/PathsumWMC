#!/usr/bin/env python3
"""Package corresponding source for the unmodified official GANAK artifact.
Run on a build host, not a timing host. All revisions come from run 42495537837.
"""
import hashlib,json,os,pathlib,shutil,subprocess,tarfile,urllib.request
root=pathlib.Path(__file__).resolve().parents[1]
out=root/'web/public/engines'
if (out/'sources/ganak-corresponding-source.tar.gz').exists() and (out/'GANAK-LICENSES.txt').exists():
 print('Using pinned corresponding-source cache');raise SystemExit(0)
base=pathlib.Path(os.environ.get('RUNNER_TEMP','/tmp'))/'pathsum-ganak-corresponding'
base.mkdir(exist_ok=True)
pins={
 'cadical':('meelgroup/cadical','818c9562f114b315a9246ced943b66b60b38e8fb'),
 'cadiback':('meelgroup/cadiback','802e1f6d34b6e613828e591a8359298cd99e6429'),
 'cryptominisat':('msoos/cryptominisat','97c6efeae45d93c45d2ebc3e4e83c2014c79513d'),
 'treedecomp':('meelgroup/treedecomp','0495b40c220b0b97a0d7c8e8d86480b4060476a8'),
 'sbva':('meelgroup/SBVA','bf157b1fea71c155251d2991b5fe0de37c57efba'),
 'arjun':('meelgroup/arjun','c86963a05108092cba6fa10e35403053bd175e4d'),
 'approxmc':('meelgroup/approxmc','aae0b47038d9ab52f2e4cc7f466cafb7219d8812'),
 'project':('meelgroup/ganak','f262be5374f312b4bf7c99c6cca26e51182158e5'),
}
notices=['GANAK corresponding-source ledger','Official build: https://github.com/meelgroup/ganak/actions/runs/42495537837',
 'Unmodified upstream WASM, exact complex-rational mode 2, probabilistic cache disabled.',
 'GMP, MPFR and FLINT retain their LGPL/GPL notices; other components retain their own licenses.',
 'Sources and build instructions are provided for rebuilding with modified dependencies.']
for name,(repo,sha) in pins.items():
 p=base/name
 if not (p/'.git').exists():
  subprocess.run(['git','init',str(p)],check=True,stdout=subprocess.DEVNULL)
  subprocess.run(['git','-C',str(p),'remote','add','origin','https://github.com/'+repo],check=True)
  subprocess.run(['git','-C',str(p),'fetch','--depth','1','origin',sha],check=True)
  subprocess.run(['git','-C',str(p),'checkout','--detach','FETCH_HEAD'],check=True)
  subprocess.run(['git','-C',str(p),'submodule','update','--init','--recursive','--depth','1'],check=True)
 notices.append(f'{name}: https://github.com/{repo}/tree/{sha}')
 for lic in p.iterdir():
  if lic.is_file() and any(lic.name.upper().startswith(x) for x in ['LICENSE','COPYING','NOTICE']):
   notices+=['',str(lic.relative_to(base)),lic.read_text(errors='replace')]
for name,url in {
 'gmp-6.3.0.tar.xz':'https://ftp.gnu.org/gnu/gmp/gmp-6.3.0.tar.xz',
 'mpfr-4.2.1.tar.xz':'https://ftp.gnu.org/gnu/mpfr/mpfr-4.2.1.tar.xz',
 'flint-3.4.0.tar.gz':'https://github.com/flintlib/flint/releases/download/v3.4.0/flint-3.4.0.tar.gz',
}.items():
 p=base/name
 if not p.exists(): urllib.request.urlretrieve(url,p)
 notices.append(f'{url} SHA256 {hashlib.sha256(p.read_bytes()).hexdigest()}')
 # Retain the actual GNU package license texts next to the browser binary too.
 with tarfile.open(p) as t:
  for item in t:
   if item.isfile() and item.name.count('/')==1 and pathlib.PurePosixPath(item.name).name.startswith('COPYING'):
    notices+=['',item.name,t.extractfile(item).read().decode(errors='replace')]
shutil.copy(root/'ports/ganak/upstream-build.yml',base/'upstream-build.yml')
(base/'REBUILD.txt').write_text('Use the pinned source checkouts in this archive with upstream-build.yml, preserving the checkout paths. Replace the actions/checkout steps by these local directories. Set Emscripten to upstream binary revision 6afbd83493d647051772d491124187d4e9a0cda9. The original workflow is the build recipe, including all configure flags and the FLINT Emscripten patch. GMP 6.3.0, MPFR 4.2.1 and FLINT 3.4.0 source tarballs are included. Upstream-generated JS and WASM are unmodified. Engine invocation and the ball adapter are in the MIT PathsumWMC repository.\n')
(base/'revisions.json').write_text(json.dumps(pins,indent=2))
(out/'sources').mkdir(exist_ok=True)
def keep(info): return None if '/.git' in info.name else info
with tarfile.open(out/'sources/ganak-corresponding-source.tar.gz','w:gz') as t:
 t.add(base,arcname='ganak-corresponding-source',filter=keep)
(out/'GANAK-LICENSES.txt').write_text('\n'.join(notices))
