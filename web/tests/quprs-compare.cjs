// Same-host original QuPRS / certified native / frozen WASM comparison.
const fs=require('node:fs'),path=require('node:path'),http=require('node:http'),os=require('node:os'),crypto=require('node:crypto');
const assert=require('node:assert/strict'),{spawn}=require('node:child_process');
const {chromium}=require(process.env.PATHSUM_PLAYWRIGHT_MODULE||'playwright');
const manifest=require('../../benchmarks/paper-2026/manifest.json');
const sha=x=>crypto.createHash('sha256').update(x).digest('hex');
const dist=path.resolve(process.env.PATHSUM_DIST),prefix='/PathsumWMC/';
const mime={'.html':'text/html','.js':'text/javascript','.css':'text/css','.wasm':'application/wasm'};
const out=process.env.PATHSUM_BENCH_OUT;fs.mkdirSync(out,{recursive:true});
const smoke=process.env.PATHSUM_COMPARE_SMOKE==='1';
const pilot=process.env.PATHSUM_COMPARE_PILOT==='true';
const pilotPlan=require('../../benchmarks/quprs-compare/performance-pilot.json');
const shard=Number(process.env.PATHSUM_SHARD||0),shards=Number(process.env.PATHSUM_SHARDS||24);
const limit=200000;
const server=http.createServer((req,res)=>{
 const url=new URL(req.url,'http://localhost');
 if(!url.pathname.startsWith(prefix)){res.writeHead(404);res.end();return}
 const file=path.resolve(dist,decodeURIComponent(url.pathname.slice(prefix.length))||'index.html');
 if(!file.startsWith(dist+path.sep)||!fs.existsSync(file)||!fs.statSync(file).isFile()){res.writeHead(404);res.end();return}
 res.writeHead(200,{'Content-Type':mime[path.extname(file)]||'application/octet-stream'});fs.createReadStream(file).pipe(res);
});
function subprocess(command,args,request,isQuprs){
 return new Promise(resolve=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'pathsum-compare-'));
  const tick=performance.now();let stdout='',stderr='',timedOut=false,overflow=false;
  const child=spawn('prlimit',['--as=4294967296','--stack=16777216',command,...args],{
   detached:true,env:{...process.env,TMPDIR:dir,TMP:dir,TEMP:dir,OPENBLAS_NUM_THREADS:'1',OMP_NUM_THREADS:'1',PYTHONHASHSEED:'0'},stdio:['pipe','pipe','pipe']});
  const kill=()=>{if(child.pid)try{process.kill(-child.pid,'SIGKILL')}catch{}};
  const timer=setTimeout(()=>{timedOut=true;kill()},limit);
  child.stdin.on('error',()=>{});child.stdin.end(JSON.stringify(request));
  child.stdout.on('data',chunk=>{stdout+=chunk;if(stdout.length>8_000_000){overflow=true;kill()}});
  child.stderr.on('data',chunk=>{stderr=(stderr+chunk).slice(-64000)});
  let launchError;child.on('error',e=>{launchError=String(e)});
  child.on('close',(code,signal)=>{
   clearTimeout(timer);kill();fs.rmSync(dir,{recursive:true,force:true});
   const process_ms=performance.now()-tick;
   if(timedOut){resolve({status:'TIMEOUT',process_ms,request_ms:null,error:'200 s whole-process cap',stderr});return}
   let data;
   try{const line=isQuprs?stdout.split('\n').find(l=>l.startsWith('PATHSUM_COMPARISON_RESULT '))?.slice('PATHSUM_COMPARISON_RESULT '.length):stdout.trim();data=JSON.parse(line)}catch{}
   // Prefix length is deliberately validated in the smoke test.
   if(!data||data.error||code!==0||overflow||launchError){resolve({status:'ERROR',process_ms,request_ms:data?.request_ms??null,error:data?.error||launchError||`exit=${code} signal=${signal} output_overflow=${overflow}`,stderr,stdout_tail:stdout.slice(-8000),traceback:data?.traceback});return}
   const verdict=data.report?.equivalent;
   const status=isQuprs?(['equivalent','equivalent*','not_equivalent'].includes(verdict)?'DECIDED':verdict==='Timeout'?'TIMEOUT':verdict==='MemoryOut'?'MEMORY_LIMIT':'UNKNOWN'):
    data.report?.frobenius_distance?'BOUND':data.report?.verdict==='unknown'?'UNKNOWN':'INCONCLUSIVE';
   resolve({status,process_ms,...data,stderr});
  });
 });
}
(async()=>{
 let browser,page;
 try{
  await new Promise(r=>server.listen(0,'127.0.0.1',r));const base=`http://127.0.0.1:${server.address().port}${prefix}`;
  const launch=async()=>{browser=await chromium.launch({args:['--js-flags=--max-old-space-size=2048']});page=await browser.newPage();await page.goto(base)};
  await launch();
  const workerFile=fs.readdirSync(path.join(dist,'assets')).find(x=>/^solver\.worker-.*\.js$/.test(x));assert(workerFile);
  const workerUrl=new URL('assets/'+workerFile,base).href;
  const binaries=[process.env.PATHSUM_NATIVE_FRONTEND,process.env.PATHSUM_NATIVE_GPMC,process.env.PATHSUM_ORIGINAL_GPMC];
  const metadata={revision:process.env.GITHUB_SHA,artifact_run:process.env.PATHSUM_BUILD_RUN,original_revision:manifest.revision,
   original_profile:'Unmodified Python API, backend=python, safe_mode=false; separate Difference default and Proportional profiles; source/transpiled direction unchanged.',
   shard,shards,smoke,pilot,pilot_plan:pilot?pilotPlan:null,cpu:os.cpus()[0].model,browser:browser.version(),
   hashes:Object.fromEntries([...binaries,...['engines/gpmc.wasm',...fs.readdirSync(path.join(dist,'assets')).filter(n=>n.endsWith('.wasm')).map(n=>'assets/'+n)].map(n=>path.join(dist,n))].map(n=>[n,sha(fs.readFileSync(n))])),
   policy:'Serial on one pinned CPU. Native/original: 4 GiB AS, 16 MiB stack; WASM: 512 MiB/module. 200 s whole-process/worker request cap. Post-import API and whole-process times separate. Five repeats only when the first completed cell takes <2 s whole-process/request; otherwise one screening trial. No unresolved-time imputation; different numerical guarantees.'};
  fs.writeFileSync(path.join(out,'metadata.json'),JSON.stringify(metadata,null,2));
  const qasm=gates=>`OPENQASM 2.0; include "qelib1.inc"; qreg q[1]; ${gates}`;
  const cases=smoke?[{id:'smoke/HH',family:'MQTBench',a:qasm('h q[0]; h q[0];'),b:qasm('x q[0]; x q[0];')},
   {id:'smoke/Tself',family:'MQTBench',a:qasm('t q[0];'),b:qasm('t q[0];')},
   {id:'smoke/near_phase',family:'MQTBench',a:qasm('u1(0.000001) q[0]; x q[0]; x q[0];'),b:qasm('x q[0]; x q[0];')}]:manifest.cases;
  for(let idx=0;idx<cases.length;idx++){
   if(!smoke&&idx%shards!==shard)continue;const c=cases[idx];
   if(!smoke&&pilot&&!pilotPlan.case_ids.includes(c.id))continue;let a=c.a,b=c.b;
   if(!smoke){a=fs.readFileSync(path.join(process.env.PATHSUM_PAPER_ROOT,c.source),'utf8');b=fs.readFileSync(path.join(process.env.PATHSUM_PAPER_ROOT,c.transpiled),'utf8');assert.equal(sha(a),c.sha256.source);assert.equal(sha(b),c.sha256.transpiled)}
   for(const mode of c.family==='Feynman'?['reduction']:pilot?['hybrid']:['hybrid','wmc']){
    const systems=['quprs-default','quprs-proportional','ours-native','ours-wasm'];
    const rotated=systems.slice(idx%4).concat(systems.slice(0,idx%4));
    for(const system of rotated){
     let trials=1;
     for(let trial=0;trial<trials;trial++){
      const request={circuit1:a,circuit2:b,mode,counter:'gpmc',precision:128,max_precision:128,max_nodes:200000};let r;
      if(system==='ours-native')r=await subprocess(process.env.PATHSUM_NATIVE_FRONTEND,[process.env.PATHSUM_NATIVE_GPMC],request,false);
      else if(system.startsWith('quprs-'))r=await subprocess('python3',[path.resolve('benchmarks/quprs-compare/quprs_runner.py'),system==='quprs-default'?'Difference':'Proportional'],request,true);
      else{
       try{r=await page.evaluate(async({workerUrl,base,request,limit})=>{
        const {createWorkerHost}=await import(new URL('engines/host.js',base).href);const start=performance.now();
        const root=new Worker(workerUrl,{type:'module'}),host=createWorkerHost(root,new URL('engines/',base).href,limit);root.postMessage({request});
        try{const d=await host.result;return {status:d.report?.frobenius_distance?'BOUND':d.report?.verdict==='unknown'?'UNKNOWN':'INCONCLUSIVE',report:d.report,request_ms:performance.now()-start,process_ms:null}}
        catch(e){return {status:e.code==='TIMEOUT'?'TIMEOUT':'ERROR',error:String(e),request_ms:performance.now()-start,process_ms:null}}
       },{workerUrl,base,request,limit})}catch(e){r={status:'BROWSER_CRASH',error:String(e),request_ms:null,process_ms:null};await browser.close().catch(()=>{});await launch()}
      }
      const row={id:c.id,family:c.family,mode,system,trial,...r};fs.appendFileSync(path.join(out,'rows.jsonl'),JSON.stringify(row)+'\n');
      console.log(c.id,mode,system,trial,r.status,((r.process_ms??r.request_ms??0)/1000).toFixed(3));
      if(smoke){assert(!['ERROR','BROWSER_CRASH','TIMEOUT','MEMORY_LIMIT'].includes(r.status),JSON.stringify(row));if(c.id!=='smoke/near_phase')assert(system.startsWith('quprs-')?['equivalent','equivalent*'].includes(r.report.equivalent):r.report.frobenius_distance?.upper_exact==='0',JSON.stringify(row));}
      if(!smoke&&trial===0&&['BOUND','DECIDED'].includes(r.status)&&(r.process_ms??r.request_ms)<2000)trials=5;
     }
    }
   }
  }
 }finally{if(browser)await browser.close();server.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
