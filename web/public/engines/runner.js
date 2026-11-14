// One exact upstream counter invocation per disposable classic worker.
// MIT licensed adapter; the loaded engines retain their own licenses.
self.onmessage = async ({data}) => {
  if (!['gpmc','ganak'].includes(data.engine) || typeof data.cnf !== 'string') {
    self.postMessage({error:'Invalid counter request.'}); return;
  }
  let stdout='', stderr='';
  const append=(which,line)=>{
    if (stdout.length+stderr.length > 4_000_000) throw new Error('Counter log exceeds 4 MB.');
    if(which==='out') stdout+=line+'\n'; else stderr+=line+'\n';
  };
  const start=performance.now();
  self.Module={
    noInitialRun:true,
    locateFile:name=>new URL(name,self.location.href).href,
    print:line=>append('out',line),
    printErr:line=>append('err',line),
    onAbort:reason=>self.postMessage({error:`Counter aborted: ${reason}`}),
    onRuntimeInitialized:()=>{
      const loaded=performance.now();
      try {
        Module.FS.writeFile('/input.cnf',data.cnf);
        const args=data.engine==='gpmc'
          ? ['-mode=1','-cs=64','/input.cnf']
          : ['--verb','0','--mode','2','--prob','0','/input.cnf'];
        let status=0;
        try { status=Module.callMain(args) || 0; }
        catch(e) { if(e && e.name==='ExitStatus') status=e.status; else throw e; }
        if(status!==0) throw new Error(`Counter exited with status ${status}. ${stderr}`);
        self.postMessage({stdout,stderr,elapsed_ms:performance.now()-loaded,load_ms:loaded-start,
          heap_bytes:typeof HEAPU8!=='undefined'?HEAPU8.buffer.byteLength:null,engine:data.engine});
      } catch(e) { self.postMessage({error:String(e),stdout,stderr}); }
    }
  };
  try { importScripts(`./${data.engine}.js`); }
  catch(e) { self.postMessage({error:String(e)}); }
};
