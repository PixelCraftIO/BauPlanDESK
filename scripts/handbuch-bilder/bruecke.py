"""Brücke für Handbuchaufnahmen: liefert apps/desktop/dist auf :1420 (devUrl der
Debug-App) mit einem injizierten Fernsteuerskript aus. Über :1421 nimmt sie
JavaScript entgegen, führt es im Fenster aus und gibt das Ergebnis zurück.
Nur für lokale Aufnahmen; nicht Teil der Anwendung."""
import http.server, threading, queue, json, sys, os, uuid
DIST=sys.argv[1]
cmds={}   # label -> Queue
res={}    # id -> (Event, value)
lock=threading.Lock()
INJECT=b"""<script>
(function(){
 const label=()=>{try{return window.__TAURI_INTERNALS__.metadata.currentWindow.label}catch(e){return 'main'}};
 async function loop(){
  for(;;){
   try{
    const r=await fetch('/__cmd?l='+label());
    if(r.status===204) continue;
    const c=await r.json();
    let out;
    try{ out={ok:true,v:await (new Function('return (async()=>{'+c.js+'})()'))()}; }
    catch(e){ out={ok:false,v:String(e)+' | '+String(e&&e.stack||'')}; }
    let body; try{body=JSON.stringify({id:c.id,...out})}catch(e){body=JSON.stringify({id:c.id,ok:false,v:'unserialisierbar'})}
    await fetch('/__res',{method:'POST',body});
   }catch(e){ await new Promise(r=>setTimeout(r,500)); }
  }
 }
 setTimeout(loop,300);
})();
</script>"""
def q(label):
  with lock:
    return cmds.setdefault(label,queue.Queue())
class App(http.server.SimpleHTTPRequestHandler):
  def __init__(s,*a,**k): super().__init__(*a,directory=DIST,**k)
  def log_message(s,*a): pass
  def end_headers(s):
    s.send_header('Cache-Control','no-store'); super().end_headers()
  def do_GET(s):
    if s.path.startswith('/__cmd'):
      label=s.path.split('l=')[-1]
      try: c=q(label).get(timeout=20)
      except queue.Empty:
        s.send_response(204); s.end_headers(); return
      b=json.dumps(c).encode(); s.send_response(200); s.send_header('Content-Type','application/json'); s.end_headers(); s.wfile.write(b); return
    p=s.path.split('?')[0]
    if p in ('/','/index.html') or (not os.path.exists(os.path.join(DIST,p.lstrip('/'))) and '.' not in p.rsplit('/',1)[-1]):
      html=open(os.path.join(DIST,'index.html'),'rb').read().replace(b'<head>',b'<head>'+INJECT,1)
      s.send_response(200); s.send_header('Content-Type','text/html'); s.end_headers(); s.wfile.write(html); return
    super().do_GET()
  def do_POST(s):
    if s.path=='/__res':
      d=json.loads(s.rfile.read(int(s.headers['Content-Length'])))
      with lock: ev=res.get(d['id'])
      if ev: ev[1].append(d); ev[0].set()
      s.send_response(200); s.end_headers(); return
class Ctl(http.server.BaseHTTPRequestHandler):
  def log_message(s,*a): pass
  def do_POST(s):
    d=json.loads(s.rfile.read(int(s.headers['Content-Length'])))
    i=str(uuid.uuid4()); ev=(threading.Event(),[])
    with lock: res[i]=ev
    q(d.get('label','main')).put({'id':i,'js':d['js']})
    ok=ev[0].wait(d.get('timeout',60))
    out=ev[1][0] if ok else {'ok':False,'v':'timeout'}
    b=json.dumps(out).encode(); s.send_response(200); s.end_headers(); s.wfile.write(b)
http.server.ThreadingHTTPServer.daemon_threads=True
threading.Thread(target=http.server.ThreadingHTTPServer(('127.0.0.1',1420),App).serve_forever,daemon=True).start()
http.server.ThreadingHTTPServer(('127.0.0.1',1421),Ctl).serve_forever()
