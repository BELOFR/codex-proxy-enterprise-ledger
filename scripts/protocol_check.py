"""使用公开 v7 帧启动真实插件进程，模拟宿主状态及分批用量"""

import argparse
import base64
import gzip
import copy
import datetime as dt
import http.server
import json
import pathlib
import queue
import struct
import subprocess
import tarfile
import threading
import time
import urllib.parse
import uuid
import zipfile
import io

ROOT = pathlib.Path(__file__).resolve().parents[1]


def encoded(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()


def fixture():
    return {
        "timeZone": "Asia/Shanghai",
        "assignments": [{"id": "mapping-a", "keyId": "key-a", "person": "张三", "department": "研发部", "project": "审计助手", "startDate": "2026-10-01", "endDate": None}],
        "pools": [{"id": "pool-a", "name": "共享账号", "basis": "tokens", "accountIds": [], "keyIds": [], "provider": None, "shares": []}],
        "payments": [{"id": "payment-a", "paidDate": "2026-10-01", "supplier": "示例供应商", "voucher": "DEMO-001", "kind": "subscription", "poolId": "pool-a", "currency": "CNY", "paidAmount": "720.00", "allocationBase": "720.00", "startDate": "2026-10-01", "endDate": "2026-11-01", "originalPaymentId": None, "notes": "仅协议测试数据"}],
        "budgets": [{"id": "budget-a", "scope": "department", "name": "研发部", "currency": "CNY", "amount": "800.00", "startDate": "2026-10-01", "endDate": "2026-11-01"}],
    }


class Peer:
    def __init__(self, command, manifest):
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.frames = queue.Queue()
        self.errors = []
        self.lock = threading.RLock()
        self.sequence = 0
        self.version = 1
        self.state = {"encoding": "gzip-base64", "payload": base64.b64encode(gzip.compress(encoded({"settings": fixture(), "closedReports": []}))).decode()}
        self.scan_count = 0
        self.fail_next = False
        self.incarnation = "ledger-test-" + uuid.uuid4().hex
        self.records = []
        self.cursor = 0
        threading.Thread(target=self._reader, daemon=True).start()
        threading.Thread(target=lambda: self.errors.append(self.process.stderr.read().decode(errors="replace")), daemon=True).start()
        self.send({"type": "hello", "handshake": {"protocol_version": 7, "artifact_sha256": "a" * 64, "plugin_id": "belofr.enterprise-ledger", "instance_id": "ledger-test", "generation": 1, "incarnation": self.incarnation, "contributes": manifest["contributes"]}}, {})
        ready = self.receive()[0]
        assert ready["type"] == "ready", ready
        self.call("management.register", "registration", {})

    def _exact(self, length):
        value = bytearray()
        while len(value) < length:
            chunk = self.process.stdout.read(length - len(value))
            if not chunk:
                raise EOFError(f"plugin closed ({self.process.poll()})")
            value.extend(chunk)
        return bytes(value)

    def _reader(self):
        try:
            while True:
                mlen, plen = struct.unpack(">IQ", self._exact(12))
                if mlen > 65536 or plen > 128 * 1024 * 1024:
                    raise ValueError("invalid frame length")
                meta = json.loads(self._exact(mlen))
                raw = self._exact(plen)
                offset = meta.get("request_context_bytes", 0)
                size = meta.get("data_bytes", 0)
                data = json.loads(raw[offset:offset + size]) if size else None
                self.frames.put((meta, data, raw[offset + size:]))
        except Exception as error:
            self.frames.put(error)

    def send(self, meta, data=None, payload=b""):
        meta = copy.deepcopy(meta)
        body = b"" if data is None else encoded(data)
        if body:
            meta["data_bytes"] = len(body)
        metadata = encoded(meta)
        self.process.stdin.write(struct.pack(">IQ", len(metadata), len(body) + len(payload)) + metadata + body + payload)
        self.process.stdin.flush()

    def receive(self):
        item = self.frames.get(timeout=90)
        if isinstance(item, Exception):
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                pass
            time.sleep(0.1)
            raise RuntimeError(f"{item}: {''.join(self.errors)[-1200:]}")
        return item

    def _usage(self, start, end):
        records = []
        source_start = int(dt.datetime(2026, 10, 1, tzinfo=dt.timezone(dt.timedelta(hours=8))).timestamp() * 1000)
        for i in range(1500):
            records.append({
                "kind": "record", "value": {
                    "id": f"request-{i:05}",
                    "data": {"client_key_ref": "key-a" if i % 2 == 0 else "key-b", "provider": "openai", "account_ref": "account-a", "requested_model": "alias", "model": "model-alpha" if i % 3 else "model-beta", "protocol": "openai", "operation": "responses", "endpoint": "/v1/responses", "client_transport": "http", "service_tier": None, "started_at_ms": source_start + (i + 1) * 1000, "completed_at_ms": source_start + (i + 1) * 1000 + 1, "outcome": "succeeded", "tokens": {"input_tokens": "100", "output_tokens": "20", "cached_input_tokens": "40", "cache_write_input_tokens": "0", "reasoning_output_tokens": "0", "image_input_tokens": "0", "image_output_tokens": "0", "total_tokens": "120"}, "cost": {"kind": "known", "source": "calculated", "amount": "0.001", "currency": "USD"}},
                    "user_agent": None, "reasoning_effort": None, "subagent_kind": None, "request_kind": None, "routing_scope": None, "routing_group_refs": [], "recovered": False, "request_counted": True, "usage_counted": True,
                }})
        return [r for r in records if start <= r["value"]["data"]["started_at_ms"] < end]

    def _batch(self):
        end = min(self.cursor + 1024, len(self.records))
        items = self.records[self.cursor:end]
        self.cursor = end
        return {"observation": {"generated_at_ms": 1790870400000, "analytics": None, "total_groups": None}, "items": items, "complete": end == len(self.records)}

    def callback(self, method, params, payload):
        if method == "host.state.get":
            return {"record": {"value": self.state, "version": self.version, "schema_version": 1}}, b""
        if method == "host.state.put":
            if params["expected_version"] != self.version:
                raise ValueError("unexpected CAS version")
            self.state = params["value"]
            self.version += 1
            return {"version": self.version}, b""
        if method == "host.keys.list":
            return {"keys": [{"id": "key-a", "name": "张三 Key", "enabled": True}, {"id": "key-b", "name": "待归属 Key", "enabled": True}], "next_cursor": None}, b""
        if method == "host.usage.records.scan":
            data = json.loads(payload)
            self.records = self._usage(data["filter"]["records"]["start_ms"], data["filter"]["records"]["end_ms"])
            self.scan_count += 1
            self.cursor = 0
            batch = self._batch()
            return {}, encoded({"reader": None if batch["complete"] else "reader-a", "batch": batch})
        if method == "host.usage.read.next":
            if self.fail_next:
                return {"__fault": "timeout"}, b""
            return {}, encoded(self._batch())
        if method == "host.usage.read.close":
            return {}, encoded(None)
        raise ValueError(f"unexpected callback {method}")

    def call(self, method, stage, params, payload=b""):
        with self.lock:
            self.sequence += 2
            call_id = self.sequence - 1
            timeout = 5000 if stage == "registration" else 120000 if stage == "public_management" else 600000
            context = {"call_id": call_id, "instance_id": "ledger-test", "generation": 1, "incarnation": self.incarnation, "stage": stage, "timeout_ms": timeout, "resource_stream": False, "resource_scope_id": "test-scope"}
            self.send({"type": "call", "id": call_id, "method": method, "context": context}, params, payload)
            while True:
                meta, data, body = self.receive()
                kind = meta["type"]
                if kind == "callback":
                    result, callback_body = self.callback(meta["method"], data, body)
                    if "__fault" in result:
                        self.send({"type": "error", "id": meta["id"], "error": {"code": "timeout", "message": "test timeout", "send_state": "not_sent"}})
                    else:
                        self.send({"type": "result", "id": meta["id"]}, result, callback_body)
                elif kind == "resume":
                    self.send({"type": "resumed", "id": meta["id"], "epoch": meta["epoch"]})
                elif kind in ("suspend", "callback_consumed"):
                    pass
                elif kind == "result":
                    assert meta["id"] == call_id
                    return data, body
                elif kind == "error":
                    raise RuntimeError(meta)
                else:
                    raise RuntimeError(f"unexpected message {meta}")

    def api(self, path, data=None):
        meta, body = self.call("management.handle", "management", {"method": "GET" if data is None else "POST", "path": path, "query": "", "content_type": None if data is None else "application/json", "headers": []}, b"" if data is None else encoded(data))
        return meta["status"], json.loads(body)

    def download(self, file_id):
        return self.call("management.callback", "public_management", {"method": "GET", "path": "download", "query": "state=test&id=" + file_id, "content_type": None, "headers": []})

    def close(self):
        try:
            self.send({"type": "shutdown"})
            self.process.stdin.close()
            self.process.wait(timeout=10)
        except Exception:
            self.process.terminate()


def verify(peer, output):
    status, state = peer.api("api/state")
    assert status == 200 and state["settings"]["payments"][0]["paidAmount"] == "720.00"
    query = {"startDate": "2026-10-01", "endDate": "2026-10-31"}
    status, report = peer.api("api/report", query)
    assert status == 200, report
    assert report["recordCount"] == 1500 and len(report["records"]) == 100
    assert report["metrics"]["totalTokens"] == "180000"
    status, file = peer.api("api/export", {"reportId": report["id"]})
    assert status == 200, file
    meta, content = peer.download(file["fileId"])
    assert meta["status"] == 200 and content.startswith(b"PK")
    archive = zipfile.ZipFile(io.BytesIO(content))
    assert b"request-01499" in archive.read("xl/sharedStrings.xml")
    meta, _ = peer.download(file["fileId"])
    assert meta["status"] == 410
    output.mkdir(parents=True, exist_ok=True)
    (output / "protocol-export.xlsx").write_bytes(content)
    status, closed = peer.api("api/close", {"reportId": report["id"], "label": "测试月结", "reason": ""})
    assert status == 200, closed
    scans = peer.scan_count
    status, reopened = peer.api("api/open", {"reportId": closed["id"]})
    assert status == 200 and reopened["recordCount"] == 1500 and peer.scan_count == scans
    status, _ = peer.api("api/state", {"expectedVersion": 1, "settings": fixture()})
    assert status == 409
    peer.fail_next = True
    status, partial = peer.api("api/report", query)
    assert status == 502 and "error" in partial
    peer.fail_next = False
    status, current = peer.api("api/state")
    assert status == 200
    csv = "id,paidDate,supplier,voucher,kind,poolId,currency,paidAmount,allocationBase,startDate,endDate,originalPaymentId,notes\nimport-check,2026-10-15,CSV supplier,CSV-001,recharge,pool-a,CNY,10.00,0.00,2026-10-01,2026-11-01,,fixture\n"
    status, imported = peer.api("api/import-payments", {"expectedVersion": current["version"], "csv": csv})
    assert status == 200 and imported["imported"] == 1, imported
    status, duplicate = peer.api("api/import-payments", {"expectedVersion": imported["version"], "csv": csv})
    assert status == 400, duplicate
    status, after = peer.api("api/state")
    assert status == 200 and after["version"] == imported["version"] and len(after["settings"]["payments"]) == 2
    # Remove only a disposable second snapshot, leaving the first for browser checks.
    status, fresh = peer.api("api/report", query)
    assert status == 200
    status, disposable = peer.api("api/close", {"reportId": fresh["id"], "label": "可移除测试版本", "reason": "验证删除"})
    assert status == 200, disposable
    status, _ = peer.api("api/delete-close", {"reportId": disposable["id"], "expectedVersion": imported["version"]})
    assert status == 409
    status, removed = peer.api("api/delete-close", {"reportId": disposable["id"], "expectedVersion": disposable["version"]})
    assert status == 200, removed
    status, after = peer.api("api/state")
    assert status == 200 and len(after["closedReports"]) == 1 and len(after["settings"]["payments"]) == 2
    result = {"protocol": 7, "records": 1500, "previewRows": 100, "completeScan": True, "singleUseDownload": True, "snapshotNoRescan": True, "staleStateRejected": True, "partialScanRejected": True, "csvImportAndDuplicateRejection": True, "archiveRemovalPreservesSettings": True}
    (output / "protocol-results.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False))


def serve(peer, port):
    tickets = {}
    class Handler(http.server.BaseHTTPRequestHandler):
        def reply(self, status, kind, body):
            self.send_response(status)
            self.send_header("Content-Type", kind)
            self.send_header("Access-Control-Allow-Origin", "*")
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            data = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            if self.path == "/ticket":
                token = uuid.uuid4().hex
                tickets[token] = time.monotonic() + 120
                return self.reply(200, "application/json", encoded({"callbackUrl": f"http://127.0.0.1:{port}/download?state={token}"}))
            status, value = peer.api(data["path"], data.get("data"))
            self.reply(status, "application/json", encoded(value))

        def do_GET(self):
            path = urllib.parse.urlparse(self.path)
            if path.path == "/__shutdown":
                self.reply(200, "text/plain", b"stopping test host")
                threading.Thread(target=self.server.shutdown, daemon=True).start()
                return
            if path.path == "/download":
                values = urllib.parse.parse_qs(path.query)
                token = values.get("state", [""])[0]
                if tickets.pop(token, 0) < time.monotonic():
                    return self.reply(410, "application/json", encoded({"error": "ticket expired"}))
                meta, body = peer.download(values.get("id", [""])[0])
                self.send_response(meta["status"])
                self.send_header("Content-Type", meta["content_type"])
                for header in meta.get("headers", []):
                    self.send_header(header["name"], bytes(header["value"]).decode())
                self.end_headers()
                self.wfile.write(body)
                return
            if path.path == "/":
                parent = '''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>台账协议验证</title><body style="margin:20px;background:#f4f6f8;font-family:Arial"><h2>企业台账 · 隔离测试宿主</h2><p>1500条虚构请求，真实插件进程处理</p><iframe id="plugin" sandbox="allow-scripts" src="/plugin" style="width:100%;height:2800px;border:0"></iframe><script>addEventListener('message',async e=>{if(e.source!==document.querySelector('iframe').contentWindow)return;const m=e.data;try{let value;if(m.kind==='request'){const r=await fetch('/bridge',{method:'POST',body:JSON.stringify({path:m.input.path,...(m.input.body?{data:JSON.parse(m.input.body)}:{})})});value={status:r.status,contentType:r.headers.get('content-type'),bytes:Array.from(new Uint8Array(await r.arrayBuffer()))}}else if(m.kind==='ticket'){value=await(await fetch('/ticket',{method:'POST',body:'{}'})).json()}else if(m.kind==='open'){window.open(m.input,'_blank');value=null}e.source.postMessage({id:m.id,value},'*')}catch(err){e.source.postMessage({id:m.id,error:String(err)},'*')}})</script></body></html>'''
                return self.reply(200, "text/html", parent.encode())
            if path.path == "/plugin":
                html = (ROOT / "frontend/dist/index.html").read_text(encoding="utf-8")
                theme = "dark" if urllib.parse.parse_qs(path.query).get("theme") == ["dark"] else "light"
                theme_file = ROOT / ".runtime" / f"theme-{theme}.css"
                if theme_file.exists():
                    html = html.replace("<head>", "<head><style>" + theme_file.read_text(encoding="utf-8") + "</style>")
                    html = html.replace('<html lang="zh-CN">', f'<html lang="zh-CN" data-theme="{theme}">')
                bridge = '''<script>let seq=0;const pending=new Map();function invoke(kind,input){return new Promise((resolve,reject)=>{const id=++seq;pending.set(id,{resolve,reject});parent.postMessage({id,kind,input},'*')})}addEventListener('message',e=>{if(e.source!==parent)return;const p=pending.get(e.data.id);if(!p)return;pending.delete(e.data.id);if(e.data.error)p.reject(Error(e.data.error));else p.resolve(e.data.value)});window.codexProxyPlugin={request:async input=>{const r=await invoke('request',input);return{status:r.status,contentType:r.contentType,body:new Uint8Array(r.bytes).buffer}},callbackTicket:input=>invoke('ticket',input),openExternal:url=>invoke('open',url)};</script>'''
                return self.reply(200, "text/html", html.replace("<head>", "<head>" + bridge).encode())
            if path.path in ("/app.js", "/app.css"):
                target = ROOT / "frontend/dist" / path.path[1:]
                return self.reply(200, "text/javascript" if target.suffix == ".js" else "text/css", target.read_bytes())
            self.reply(404, "text/plain", b"not found")

        def log_message(self, *args):
            pass

    print(f"http://127.0.0.1:{port}", flush=True)
    http.server.ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default=str(ROOT / "backend/target/debug/enterprise-ledger.exe"))
    parser.add_argument("--manifest", help="已打包的 plugin.json；默认从 dist 安装包读取")
    parser.add_argument("--docker", action="store_true")
    parser.add_argument("--serve", action="store_true")
    parser.add_argument("--port", type=int, default=8769)
    options = parser.parse_args()
    command = [options.binary]
    if options.docker:
        command = ["docker", "run", "--rm", "-i", "--name", "ledger-protocol-check", "--mount", f"type=bind,src={ROOT},dst=/work,readonly", "rust:1.99.1-bookworm", "/work/backend/target-linux/release/enterprise-ledger"]
    if options.manifest:
        manifest = json.loads(pathlib.Path(options.manifest).read_text(encoding="utf-8-sig"))
    else:
        packages = sorted((ROOT / "dist").glob("*.tar.gz"), key=lambda p: p.stat().st_mtime)
        if not packages:
            parser.error("请先打包，或使用 --manifest 指定已打包的清单")
        with tarfile.open(packages[-1], "r:gz") as package:
            member = next(m for m in package.getmembers() if m.name == "plugin.json")
            manifest = json.load(package.extractfile(member))
    peer = Peer(command, manifest)
    try:
        verify(peer, ROOT / ".runtime" / ("docker" if options.docker else "native"))
        if options.serve:
            serve(peer, options.port)
    finally:
        peer.close()
