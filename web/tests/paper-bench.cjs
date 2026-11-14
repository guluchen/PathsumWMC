const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const assert = require('node:assert/strict');
const {chromium} = require(process.env.PATHSUM_PLAYWRIGHT_MODULE || 'playwright');
const dist = path.resolve(process.env.PATHSUM_DIST || path.join(__dirname, '../dist'));
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
const os=require('node:os');
const crypto=require('node:crypto');
const {execFileSync}=require('node:child_process');
const manifest=require('../../benchmarks/paper-2026/manifest.json');
const sourceRoot=process.env.PATHSUM_PAPER_ROOT;
const shard=Number(process.env.PATHSUM_SHARD||0),shards=Number(process.env.PATHSUM_SHARDS||12);
const out=process.env.PATHSUM_BENCH_OUT||'test-results/paper';fs.mkdirSync(out,{recursive:true});
const sha=s=>crypto.createHash('sha256').update(s).digest('hex');
(async()=>{
 let browser;
 const rows=[];
 try {
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  const base=`http://127.0.0.1:${server.address().port}${prefix}`;
  browser=await chromium.launch({args:['--js-flags=--max-old-space-size=2048']});
  let page=await browser.newPage();await page.goto(base);
  const filename=fs.readdirSync(path.join(dist,'assets')).find(n=>/^solver\.worker-.*\.js$/.test(n));
  assert(filename,'Compiled solver worker is missing');
  const workerUrl=new URL('assets/'+filename,base).href;
  const metadata={revision:process.env.GITHUB_SHA,artifact_run:process.env.PATHSUM_BUILD_RUN,source_revision:manifest.revision,native_gpmc_sha256:process.env.PATHSUM_NATIVE_GPMC?sha(fs.readFileSync(process.env.PATHSUM_NATIVE_GPMC)):null,
   binary_sha256:Object.fromEntries(['engines/gpmc.wasm','engines/ganak.wasm',...fs.readdirSync(path.join(dist,'assets')).filter(n=>n.endsWith('.wasm')).map(n=>'assets/'+n)].map(n=>[n,sha(fs.readFileSync(path.join(dist,n)))])),
   shard,shards,cpu:os.cpus()[0].model,logical_cpus:os.cpus().length,total_memory:os.totalmem(),browser:browser.version(),
   scope:'Serial fresh browser workers on one pinned CPU; local HTTP assets, not Internet download latency. Whole-request 200 s cap. One screening trial, plus four repeat trials only when the first finite bound completes under 2 s. Timings are not paper-hardware speedup claims.'};
  fs.writeFileSync(path.join(out,'metadata.json'),JSON.stringify(metadata,null,2));
  for(let idx=0;idx<manifest.cases.length;idx++) {
   if(idx%shards!==shard)continue;
   const c=manifest.cases[idx];
   const a=fs.readFileSync(path.join(sourceRoot,c.source)),b=fs.readFileSync(path.join(sourceRoot,c.transpiled));
   assert.equal(sha(a),c.sha256.source);assert.equal(sha(b),c.sha256.transpiled);
   let cells=c.family==='Feynman'?[['reduction','dpll']]:[['hybrid','gpmc'],['hybrid','ganak'],['wmc','gpmc'],['wmc','ganak']];
   cells=cells.slice(idx%cells.length).concat(cells.slice(0,idx%cells.length));
   for(const [mode,counter] of cells) {
    let trials=1;
    for(let trial=0;trial<trials;trial++) {
     const request={circuit1:a.toString(),circuit2:b.toString(),mode,counter,precision:128,max_precision:128,max_nodes:200000};
     let response;
     try {
      response=await page.evaluate(async ({workerUrl,base,request})=>{
       const {createWorkerHost}=await import(new URL('engines/host.js',base).href);
       const start=performance.now(),worker=new Worker(workerUrl,{type:'module'});
       const host=createWorkerHost(worker,new URL('engines/',base).href,200000);
       worker.postMessage({request,diagnostics:true});
       try {const data=await host.result;return {diagnostics:data.diagnostics,status:data.report?.frobenius_distance?'BOUND':data.report?.verdict==='unknown'?'UNKNOWN':'INCONCLUSIVE',report:data.report,wall_ms:performance.now()-start}}
       catch(e){return {status:e.code==='TIMEOUT'?'TIMEOUT':'ERROR',error:String(e),wall_ms:performance.now()-start}}
      },{workerUrl,base,request});
     } catch(e) {
      response={status:'BROWSER_CRASH',error:String(e),wall_ms:null};
      await browser.close().catch(()=>{});browser=await chromium.launch({args:['--js-flags=--max-old-space-size=2048']});
      page=await browser.newPage();await page.goto(base);
     }
     const diag=response.diagnostics;delete response.diagnostics;
     if(trial===0 && counter==='gpmc' && diag && response.wall_ms<30000 && process.env.PATHSUM_NATIVE_GPMC) {
      const dir=fs.mkdtempSync(path.join(os.tmpdir(),'pathsum-native-counter-'));
      const native={kind:'Same GPMC exact-rational adapter, native whole-process time; comparison excludes circuits taking >=30 s in WASM.',input_sha256:[],elapsed_ms:0,exact_count_match:true};
      try {
       for(let j=0;j<diag.cnfs.length;j++) {
        const file=path.join(dir,`job-${j}.cnf`);fs.writeFileSync(file,diag.cnfs[j]);native.input_sha256.push(sha(diag.cnfs[j]));
        const start=performance.now();
        const stdout=execFileSync('prlimit',['--as=4294967296',process.env.PATHSUM_NATIVE_GPMC,'-mode=1','-cs=64',file],{encoding:'utf8',timeout:200000,maxBuffer:4_000_000});
        native.elapsed_ms+=performance.now()-start;
        const count=s=>s.match(/^c s exact arb frac (.+)$/m)?.[1].replace(/\s/g,'');
        native.exact_count_match &&= count(stdout)===count(diag.stdout[j]);
       }
      } catch(e) {native.error=String(e)} finally {fs.rmSync(dir,{recursive:true,force:true})}
      response.native_gpmc=native;
     }
     const row={id:c.id,family:c.family,mode,counter,trial,...response};rows.push(row);
     fs.appendFileSync(path.join(out,'rows.jsonl'),JSON.stringify(row)+'\n');
     console.log(`${c.id} ${mode}/${counter} #${trial}: ${row.status} ${row.wall_ms===null?'unknown':(row.wall_ms/1000).toFixed(3)}s ${row.report?.frobenius_distance?.upper||row.error||''}`);
     if(trial===0 && row.status==='BOUND' && row.wall_ms<2000)trials=5;
    }
   }
  }
 } finally {
  if(browser)await browser.close();server.close();
  fs.writeFileSync(path.join(out,'summary.json'),JSON.stringify({rows:rows.length,statuses:rows.reduce((a,r)=>(a[r.status]=(a[r.status]||0)+1,a),{})},null,2));
 }
})().catch(e=>{console.error(e);process.exitCode=1});
