const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const assert = require('node:assert/strict');
const {chromium} = require(process.env.PATHSUM_PLAYWRIGHT_MODULE || 'playwright');
const dist = path.resolve(__dirname, '../dist');
const prefix = '/PathsumWMC/';
const mime = {'.html':'text/html','.js':'text/javascript','.css':'text/css','.wasm':'application/wasm','.svg':'image/svg+xml','.woff2':'font/woff2','.txt':'text/plain'};
const server = http.createServer((req,res)=>{
  const url = new URL(req.url, 'http://localhost');
  if(url.pathname === '/PathsumWMC'){res.writeHead(301,{Location:prefix});res.end();return;}
  if(!url.pathname.startsWith(prefix)){res.writeHead(404);res.end();return;}
  const file = path.resolve(dist, decodeURIComponent(url.pathname.slice(prefix.length)) || 'index.html');
  if(!file.startsWith(dist+path.sep) || !fs.existsSync(file) || !fs.statSync(file).isFile()){res.writeHead(404);res.end();return;}
  res.writeHead(200,{'Content-Type':mime[path.extname(file)]||'application/octet-stream'});fs.createReadStream(file).pipe(res);
});
const cases=require('./fixtures/exact-counters.json');
function rational(s) {const [n,d='1']=s.split('/');return [BigInt(n),BigInt(d)]}
function eq(a,b) {const [an,ad]=rational(a),[bn,bd]=rational(b);return an*bd===bn*ad}
(async()=>{
 let browser;
 try {
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  const base=`http://127.0.0.1:${server.address().port}${prefix}`;
  browser=await chromium.launch();
  const page=await browser.newPage();await page.goto(base);
  const results=[];
  for(const engine of ['gpmc','ganak']) for(const c of cases) {
   const result=await page.evaluate(({engine,cnf})=>new Promise((resolve,reject)=>{
    const worker=new Worker(new URL('engines/runner.js',location.href));
    const t=setTimeout(()=>{worker.terminate();reject(new Error('Counter timeout'))},15000);
    worker.onerror=e=>{clearTimeout(t);worker.terminate();reject(new Error(e.message))};
    worker.onmessage=({data})=>{clearTimeout(t);worker.terminate();resolve(data)};
    worker.postMessage({engine,cnf});
   }),{engine,cnf:c[engine]});
   assert(!result.error,`${engine}/${c.id}: ${JSON.stringify(result)}`);
   const m=result.stdout.match(/^c s exact arb frac (.+)$/m);assert(m,JSON.stringify(result));
   const token=m[1].replace(/\s/g,'');
   const z=token.match(/^([+-]?\d+(?:\/\d+)?)([+-](?:-)?\d+(?:\/\d+)?)i$/);
   if(z) {assert(eq(z[1],c.real));assert(eq(z[2].replace(/^\+/,''),c.imag))}
   else {assert(eq(token,c.real));assert(eq('0',c.imag))}
   results.push({engine,id:c.id,load_ms:result.load_ms,solve_ms:result.elapsed_ms,heap_bytes:result.heap_bytes});
  }
  // Forced WMC prevents symbolic simplification from hiding an unavailable engine.
  await page.getByRole('button',{name:'Compute distance',exact:true}).waitFor();
  await page.waitForFunction(()=>!document.querySelector('button:disabled')?.textContent?.includes('Compute distance'));
  await page.locator('#mode').click();await page.getByRole('option',{name:'Model counting only',exact:true}).click();
  await page.locator('#precision').fill('128');await page.locator('#max-precision').fill('128');
  const check=page.getByRole('button',{name:'Compute distance',exact:true});
  for(const engine of ['GPMC','GANAK']) {
   await page.locator('#counter').click();await page.getByRole('option',{name:engine,exact:true}).click();
   await page.locator('#circuit-a').fill('OPENQASM 2.0; qreg q[1]; h q[0]; h q[0];');
   await page.locator('#circuit-b').fill('OPENQASM 2.0; qreg q[1]; id q[0];');
   await check.click();await page.waitForFunction(()=>document.querySelector('output[aria-label="Frobenius distance"]')?.textContent==='0');
   await page.locator('#circuit-a').fill('OPENQASM 2.0; qreg q[1]; rx(1/5) q[0];');
   await page.locator('#circuit-b').fill('OPENQASM 2.0; qreg q[1]; rx(1/6) q[0];');
   await check.click();await page.waitForFunction(()=>document.querySelector('output[aria-label="Frobenius distance"]')?.textContent==='[1.66664e-2, 1.66665e-2]');
  }
  fs.mkdirSync('test-results',{recursive:true});fs.writeFileSync('test-results/counters.json',JSON.stringify({pass:true,exact_weighted_cnfs:results.length,forced_wmc_distance_checks:4,results},null,2));
  console.log('96 exact counter CNFs and 4 forced-WMC distances PASS');
 } finally {if(browser) await browser.close();server.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
