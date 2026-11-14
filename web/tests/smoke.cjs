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
(async()=>{
 let browser;
 try{
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const base=`http://127.0.0.1:${server.address().port}`;
  browser=await chromium.launch();
  const page=await browser.newPage({viewport:{width:1280,height:900}});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(base+'/PathsumWMC');assert.equal(page.url(),base+prefix);
  await page.getByText('A Cursor reimplementation of our TACAS 2026 paper, using ball arithmetic.',{exact:true}).waitFor();
  const check=page.getByRole('button',{name:'Compute distance',exact:true});
  await page.waitForFunction(()=>!Array.from(document.querySelectorAll('button')).find(b=>b.textContent==='Compute distance')?.disabled);
  await page.locator('#circuit-a').fill('OPENQASM 2.0; qreg q[1]; h q[0]; h q[0];');
  await page.locator('#circuit-b').fill('OPENQASM 2.0; qreg q[1]; id q[0];');
  await check.click();await page.waitForFunction(()=>document.querySelector('output[aria-label="Frobenius distance"]')?.textContent==='0');
  await page.locator('#circuit-a').fill('OPENQASM 2.0; qreg q[1]; h q[0];');
  await check.click();await page.waitForFunction(()=>document.querySelector('output[aria-label="Frobenius distance"]')?.textContent.includes('1.41421'));
  await page.locator('#mode').click();await page.getByRole('option',{name:'Model counting only',exact:true}).click();
  await page.locator('#precision').fill('32');await page.locator('#max-precision').fill('32');
  await page.locator('#circuit-a').fill('OPENQASM 2.0; qreg q[1]; id q[0];');
  await page.locator('#circuit-b').fill('OPENQASM 2.0; qreg q[1]; p(1/1000000000000000000000000000000) q[0];');
  await check.click();await page.waitForFunction(()=>document.querySelector('output[aria-label="Frobenius distance"]')?.textContent.startsWith('[0,'));
  assert.equal(await page.getByRole('link',{name:'Source code',exact:true}).getAttribute('href'),'https://github.com/guluchen/PathsumWMC');
  for(const file of ['LICENSE.txt','THIRD_PARTY_NOTICES.txt']){
   const response=await page.request.get(base+prefix+file);assert.equal(response.status(),200);
  }
  fs.mkdirSync('test-results',{recursive:true});await page.screenshot({path:'test-results/desktop.png',fullPage:true});
  await page.setViewportSize({width:390,height:844});
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth+1),'mobile horizontal overflow');
  await page.screenshot({path:'test-results/mobile.png',fullPage:true});assert.deepEqual(errors,[]);
  fs.writeFileSync('test-results/browser.json',JSON.stringify({pass:true,checks:['PathsumWMC subpath/redirect','Cursor/paper/ball attribution','WASM load','exact distance zero','trace-zero distance sqrt2','tiny-phase distance enclosure','license notices','source link','390px layout','no page errors']},null,2));
  console.log('Browser smoke checks PASS');
 }finally{if(browser)await browser.close();server.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
