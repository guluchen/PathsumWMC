#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
BUILD=${PATHSUM_PORT_BUILD:-$ROOT/.port-build}
OUT=${PATHSUM_PORT_OUT:-$ROOT/web/public/engines}
mkdir -p "$BUILD" "$OUT"
if [ -n "${PATHSUM_GANAK_SOURCE:-}" ]; then
 SRC=$(cd "$PATHSUM_GANAK_SOURCE" && pwd)
else
 python3 "$ROOT/scripts/package-ganak-sources.py"
 cd "$BUILD"
 tar xf "$OUT/sources/ganak-corresponding-source.tar.gz"
 SRC=$BUILD/ganak-corresponding-source
fi
PREFIX=$BUILD/ganak-prefix
mkdir -p "$PREFIX"
export PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig"
export PKG_CONFIG_PATH="$PKG_CONFIG_LIBDIR"
# Use the same portable static GMP build as GPMC (or build it first).
if [ ! -f "$BUILD/prefix/lib/libgmp.a" ]; then
 if [ ! -d "$BUILD/gmp-6.3.0" ]; then tar xf "$SRC/gmp-6.3.0.tar.xz" -C "$BUILD"; fi
 (cd "$BUILD/gmp-6.3.0" && emconfigure ./configure --disable-assembly --disable-shared --enable-cxx --host=wasm32 --prefix="$BUILD/prefix" CFLAGS='-O2' CXXFLAGS='-O2' && emmake make -j"$(nproc)" && emmake make install)
fi
cp -R "$BUILD/prefix/include" "$BUILD/prefix/lib" "$PREFIX/"
cd "$SRC"
if [ ! -f "$PREFIX/lib/libmpfr.a" ]; then
 if [ ! -d mpfr-4.2.1 ]; then tar xf mpfr-4.2.1.tar.xz; fi
 (cd mpfr-4.2.1 && emconfigure ./configure --host=wasm32 --disable-shared --with-gmp="$PREFIX" --prefix="$PREFIX" CFLAGS='-O2 -fPIC' && emmake make -j"$(nproc)" && emmake make install)
fi
if [ ! -f "$PREFIX/lib/libflint.a" ]; then
 if [ ! -d flint-3.4.0 ]; then tar xf flint-3.4.0.tar.gz; fi
 sed -i 's/#if (defined(__unix__) \&\& !defined(__CYGWIN__)) || defined(__APPLE__)/#if (defined(__unix__) \&\& !defined(__CYGWIN__) \&\& !defined(__EMSCRIPTEN__)) || defined(__APPLE__)/' flint-3.4.0/src/profiler.h
 (cd flint-3.4.0 && emconfigure ./configure --host=wasm32 --disable-assembly --disable-shared --with-gmp="$PREFIX" --with-mpfr="$PREFIX" --prefix="$PREFIX" CFLAGS='-O2 -fPIC' && emmake make -j"$(nproc)" && emmake make install)
fi
python3 - "$SRC" <<'PY'
from pathlib import Path
import sys
root=Path(sys.argv[1])
# Arjun's Emscripten option overrides BUILD_SHARED_LIBS=OFF. Remove only
# that override; retain the actual upstream preprocessing/counting algorithms.
for p in root.rglob('CMakeLists.txt'):
 s=p.read_text().replace('set(BUILD_SHARED_LIBS ON)','set(BUILD_SHARED_LIBS OFF)')
 p.write_text(s)
PY
COMMON=(-DCMAKE_BUILD_TYPE=Release -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DBUILD_SHARED_LIBS=OFF -DSTATIC_BINARY=ON -DENABLE_TESTING=OFF -DNOZLIB=ON -DCMAKE_INSTALL_PREFIX="$PREFIX" -DCMAKE_PREFIX_PATH="$PREFIX" -DCMAKE_C_FLAGS="-O2 -I$PREFIX/include" -DCMAKE_CXX_FLAGS="-O2 -I$PREFIX/include" -DCMAKE_CXX_STANDARD_LIBRARIES="-lgmpxx -lgmp" -DCMAKE_EXE_LINKER_FLAGS="-L$PREFIX/lib -sALLOW_MEMORY_GROWTH=1 -sMAXIMUM_MEMORY=536870912 -sEXIT_RUNTIME=1")
DEPS=()
for name in cadical cadiback cryptominisat treedecomp sbva arjun approxmc project; do
 emcmake cmake -S "$name" -B "$name/build" "${COMMON[@]}" "${DEPS[@]}"
 cmake --build "$name/build" -j"$(nproc)"
 if [ "$name" != project ]; then cmake --install "$name/build"; fi
 case "$name" in
  cryptominisat) DEPS+=("-Dcryptominisat5_DIR=$SRC/$name/build");;
  project) ;;
  *) DEPS+=("-D${name}_DIR=$SRC/$name/build");;
 esac
done
cp project/build/ganak.js project/build/ganak.wasm "$OUT/"
# Include the patched tree, exact commands and all upstream source/archives.
mkdir -p "$SRC/PathsumWMC-build"
cp "$ROOT/scripts/build-ganak-wasm.sh" "$ROOT/scripts/build-gpmc-wasm.sh" "$ROOT/scripts/package-ganak-sources.py" "$SRC/PathsumWMC-build/"
cp -R "$ROOT/ports" "$SRC/PathsumWMC-build/"
# Static archive: users can rebuild/relink with modified GMP/MPFR/FLINT.
tar --exclude="*/build" --exclude="*.o" --exclude="*.a" --exclude="*.lo" --exclude="*.la" --exclude="*.bc" -czf "$OUT/sources/ganak-static-port-source.tar.gz" -C "$BUILD" ganak-corresponding-source
sha256sum "$OUT/ganak.js" "$OUT/ganak.wasm" > "$OUT/ganak-SHA256SUMS.txt"
