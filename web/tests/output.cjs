const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const context={self:{}};vm.runInNewContext(fs.readFileSync('public/engines/output.js','utf8'),context);
const out=context.self.createCounterOutput();
for(let i=0;i<80;i++)out.append('out','diagnostic '.repeat(8192));
out.append('out','c s exact arb frac 1/3+-2/7i');
for(let i=0;i<80;i++)out.append('err','warning '.repeat(8192));
assert(out.stdout().includes('c s exact arb frac 1/3+-2/7i'));
assert(out.stdout().length<33000);assert(out.stderr().length<=32768);
// Do not hide multiple results: the Rust parser rejects ambiguity.
out.append('out','c s exact arb frac 0+0i');
assert.equal(out.stdout().match(/c s exact arb frac /g).length,2);
assert.throws(()=>out.append('out','c s exact arb frac '+'1'.repeat(4_000_000)),/output capacity/);
console.log('Bounded diagnostics / exact-result retention PASS');
