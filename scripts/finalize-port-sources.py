#!/usr/bin/env python3
"""Attach rebuildable source layout, removing only compiled intermediate files.
The corresponding upstream/patched sources and original GNU tarballs remain.
"""
import io,pathlib,tarfile,tempfile
root=pathlib.Path(__file__).resolve().parents[1]
base=root/'web/public/engines/sources'
for filename,top in [('gpmc-port-source.tar.gz','source-bundle'),('ganak-static-port-source.tar.gz','ganak-corresponding-source')]:
 source=base/filename
 with tempfile.TemporaryDirectory(prefix='pathsum-source-layout-') as tmp:
  target=pathlib.Path(tmp)/'source.tar.gz'
  with tarfile.open(source) as src,tarfile.open(target,'w:gz') as dst:
   for member in src:
    if member.name.endswith(('.o','.a','.lo','.la','.bc')):continue
    if '/PathsumWMC-build/' in member.name or member.name.endswith('/PathsumWMC-build'):continue
    dst.addfile(member,src.extractfile(member) if member.isfile() else None)
   for name in ['build-gpmc-wasm.sh','build-ganak-wasm.sh','package-ganak-sources.py']:
    dst.add(root/'scripts'/name,arcname=f'{top}/PathsumWMC-build/scripts/{name}')
   dst.add(root/'ports',arcname=f'{top}/PathsumWMC-build/ports')
   readme=b'''Rebuilding and modifying the WebAssembly counters

Use Emscripten 4.0.23, CMake >=3.5, make, Python 3, and a Unix build host.
The PathsumWMC-build directory has the scripts/ and ports/ layout expected by
the build scripts. All source, original GNU tarballs and notices are retained.
GPMC's exact-rational adapter replaces only the numeric class and CLI entry;
Ganak's CMake shared-library override is removed and GMP C++ is linked explicitly.
No model-counter search algorithm is replaced by a built-in counter.\nBrowserTime.cc supplies a monotonic wall-time / allocated-linear-memory proxy\nfor browser resource budgets; it does not claim OS CPU/RSS measurements.\nThe C++ runtime uses a 16 MiB stack with overflow checks.

For GANAK, point PATHSUM_GANAK_SOURCE to the extracted ganak-corresponding-source
directory, set PATHSUM_PORT_BUILD and PATHSUM_PORT_OUT to new absolute scratch
and output directories, then run bash PathsumWMC-build/scripts/build-ganak-wasm.sh.
This uses the supplied tree, so dependency source changes are preserved.
For MPFR/FLINT modifications, edit the extracted mpfr-4.2.1/flint-3.4.0 sources
and remove the corresponding cached library from PATHSUM_PORT_BUILD/ganak-prefix.
For GMP modifications, extract its supplied tarball to PATHSUM_PORT_BUILD,
edit that source tree, and remove cached prefix/ganak-prefix GMP libraries.
The scripts can thus rebuild/relink against user-modified numerical libraries.

For GPMC, use the supplied gpmc-patched tree and GMP 6.3.0 tarball. Build GMP
with emconfigure ./configure --disable-assembly --disable-shared --enable-cxx
--host=wasm32 --prefix=/absolute/prefix CFLAGS=-O2 CXXFLAGS=-O2, then emmake make
and emmake make install. Configure gpmc-patched with emcmake cmake, adding
-I/absolute/prefix/include -std=c++14 -sUSE_ZLIB=1 to CMAKE_CXX_FLAGS, and
-L/absolute/prefix/lib -sUSE_ZLIB=1 -sALLOW_MEMORY_GROWTH=1
-sMAXIMUM_MEMORY=536870912 -sINITIAL_MEMORY=33554432\n-sSTACK_SIZE=16777216 -sSTACK_OVERFLOW_CHECK=2 -sENVIRONMENT=worker
-sEXPORTED_RUNTIME_METHODS=FS,callMain -sEXIT_RUNTIME=1 to the executable linker
flags. Use CMAKE_BUILD_TYPE=Release and CMAKE_POLICY_VERSION_MINIMUM=3.5.
The complete automated clean-source recipe is build-gpmc-wasm.sh.

The UI/Rust adapter source is MIT licensed at https://github.com/guluchen/PathsumWMC.
The counters and their dependencies retain all bundled licenses. There is no
restriction on rebuilding with modified dependencies for personal use.
'''
   info=tarfile.TarInfo(f'{top}/REBUILD-PathsumWMC.txt');info.size=len(readme);info.mode=0o644
   dst.addfile(info,io.BytesIO(readme))
  source.write_bytes(target.read_bytes())

notices=root/'web/public/engines/GANAK-LICENSES.txt'
text=notices.read_text().replace('Official build:', 'Upstream reference build (not the shipped binary):').replace('Unmodified upstream WASM, exact complex-rational mode 2, probabilistic cache disabled.','Pinned GANAK source rebuilt as self-contained static WASM with Emscripten 4.0.23; exact complex-rational mode 2, probabilistic cache disabled. The Arjun shared-library override is removed and GMP C++ is linked explicitly. See PROVENANCE.json for the actual build and binary hashes.')
notices.write_text(text)
