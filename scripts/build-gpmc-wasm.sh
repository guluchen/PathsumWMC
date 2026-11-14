#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
BUILD=${PATHSUM_PORT_BUILD:-$ROOT/.port-build}
OUT=${PATHSUM_PORT_OUT:-$ROOT/web/public/engines}
mkdir -p "$BUILD" "$OUT/sources"
cd "$BUILD"
if [ ! -f gmp-6.3.0.tar.xz ]; then curl -fL --retry 3 https://ftp.gnu.org/gnu/gmp/gmp-6.3.0.tar.xz -o gmp-6.3.0.tar.xz; fi
echo 'a3c2b80201b89e68616f4ad30bc66aee4927c3ce50e33929ca819d5c43538898  gmp-6.3.0.tar.xz' | sha256sum -c -
if [ ! -f prefix/lib/libgmp.a ]; then
  tar xf gmp-6.3.0.tar.xz
  (cd gmp-6.3.0 && emconfigure ./configure --disable-assembly --disable-shared --enable-cxx --host=wasm32 --prefix="$BUILD/prefix" CFLAGS='-O2' CXXFLAGS='-O2' && emmake make -j2 && emmake make install)
fi
if [ ! -d gpmc/.git ]; then git clone https://github.com/System-Verification-Lab/GPMC gpmc; fi
git -C gpmc checkout df1aea7769887b62f59b803293678a1bbc5fe06d
git -C gpmc reset --hard df1aea7769887b62f59b803293678a1bbc5fe06d
python3 - "$BUILD/gpmc" "$ROOT" <<'PY'
from pathlib import Path
import sys, shutil
src,root=map(Path,sys.argv[1:])
# Only explicit floating-point instantiations and includes are removed.
# The actual GPMC solver/preprocessor/component-cache code is retained.
for pattern in ('*.cc','*.h'):
 for p in src.rglob(pattern):
  if 'mpfr' in p.parts or 'c-mpfr' in p.parts: continue
  lines=p.read_text().splitlines(True)
  lines=[s for s in lines if not ('mpfr/mpreal.h' in s or (s.startswith('template ') and 'mpfr::mpreal' in s))]
  p.write_text(''.join(lines).replace('mpfr::bits2digits(mpfr::mpreal::get_default_prec())','30'))
shutil.copy(root/'ports/gpmc/ComplexMPFR.h',src/'c-mpfr/ComplexMPFR.h')
(src/'c-mpfr/ComplexMPFR.cc').write_text('#include "c-mpfr/ComplexMPFR.h"\n')
shutil.copy(root/'ports/gpmc/Main.cc',src/'core/Main.cc')
p=src/'CMakeLists.txt'
s=p.read_text().replace('gpmc flowcutter z mpfr gmp gmpxx','gpmc flowcutter gmpxx gmp')
s+='\nset_target_properties(gpmc-bin PROPERTIES SUFFIX ".js")\n'
p.write_text(s)
PY
emcmake cmake -S gpmc -B gpmc-build -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_FLAGS="-I$BUILD/prefix/include -std=c++14 -sUSE_ZLIB=1" -DCMAKE_EXE_LINKER_FLAGS="-L$BUILD/prefix/lib -sUSE_ZLIB=1 -sALLOW_MEMORY_GROWTH=1 -sMAXIMUM_MEMORY=536870912 -sENVIRONMENT=worker -sEXPORTED_RUNTIME_METHODS=FS,callMain -sEXIT_RUNTIME=1"
cmake --build gpmc-build -j2
cp gpmc-build/gpmc.js gpmc-build/gpmc.wasm "$OUT/"
cp gmp-6.3.0.tar.xz "$OUT/sources/"
mkdir -p source-bundle
cp -R "$ROOT/ports" source-bundle/
cp "$ROOT/scripts/build-gpmc-wasm.sh" source-bundle/
git -C gpmc archive HEAD > source-bundle/gpmc-upstream.tar
cp -R gpmc source-bundle/gpmc-patched
rm -rf source-bundle/gpmc-patched/.git
tar czf "$OUT/sources/gpmc-port-source.tar.gz" source-bundle
cp gpmc/LICENSE* "$OUT/" || true
cp prefix/share/doc/gmp/COPYING* "$OUT/" || cp gmp-6.3.0/COPYING* "$OUT/"
sha256sum "$OUT/gpmc.js" "$OUT/gpmc.wasm" > "$OUT/gpmc-SHA256SUMS.txt"
