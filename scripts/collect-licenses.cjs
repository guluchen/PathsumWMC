const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const modules = path.join(root, 'web/node_modules');
const dist = path.join(root, 'web/dist');
const packages = [];
for (const name of fs.readdirSync(modules)) {
  if (name.startsWith('.')) continue;
  const dir = path.join(modules, name);
  if (name.startsWith('@')) for (const sub of fs.readdirSync(dir)) packages.push(path.join(dir, sub));
  else packages.push(dir);
}
const notices = ['GPMC and GANAK run as separate WebAssembly workers. Engine versions and complete corresponding source downloads: engines/PROVENANCE.json. GPMC notices: engines/LICENSE.md and engines/COPYING*. GANAK and dependency notices: engines/GANAK-LICENSES.txt.', '', 'PathsumWMC third-party notices', '', 'PathsumWMC is MIT licensed. Bundled dependencies and fonts retain their own licenses.', 'The licenses directory also includes build-dependency notices.'];
for (const dir of packages) {
  const manifest = path.join(dir, 'package.json');
  if (!fs.existsSync(manifest)) continue;
  const pkg = JSON.parse(fs.readFileSync(manifest, 'utf8'));
  notices.push('', `${pkg.name}@${pkg.version} — ${JSON.stringify(pkg.license || 'see package license')}`);
  for (const file of fs.readdirSync(dir)) {
    if (!/^(licen[cs]e|notice|copying)([. -]|$)/i.test(file) || !fs.statSync(path.join(dir,file)).isFile()) continue;
    const rel = path.join('licenses', pkg.name, file);
    fs.mkdirSync(path.dirname(path.join(dist, rel)), {recursive:true});
    fs.copyFileSync(path.join(dir,file), path.join(dist,rel));
    notices.push(rel);
  }
}
fs.copyFileSync(path.join(root, 'LICENSE'), path.join(dist, 'LICENSE.txt'));
fs.writeFileSync(path.join(dist, 'THIRD_PARTY_NOTICES.txt'), notices.join('\n')+'\n');
