"""Demo cockpit: both emulated Flexes side by side, clickable, with the live
APDU wire between them.

The stock Speculos web UI only forwards Nano-style buttons, so this page turns
clicks into /finger touches. It also acts as the relay's wire: demo_steps.py
sends its APDUs through /a/apdu and /b/apdu, and every exchange is logged and
rendered in the center feed: the audience sees exactly the bytes the untrusted
relay sees (and nothing secret is among them: that is the thesis).

    python3 relay/cockpit.py   ->   http://localhost:5050
"""

import itertools
import threading
import time

import requests
from flask import Flask, Response, jsonify, request

DEVICES = {"a": "http://127.0.0.1:5001", "b": "http://127.0.0.1:5002"}

INS_NAMES = {
    0x01: "GET_INFO", 0x10: "CUT", 0x21: "PAIR_COMMIT", 0x22: "PAIR_RESPOND",
    0x23: "PAIR_REVEAL", 0x24: "PAIR_FINISH", 0x25: "PAIR_SAS",
    0x30: "GET_ALBUM", 0x31: "PRESS_REQUEST", 0x32: "PRESS_OFFER",
    0x33: "PRESS_LOAD_ALBUM", 0x34: "PRESS_ACCEPT", 0x40: "GET_BUNDLE",
    0x41: "CHALLENGE", 0x50: "RESET_MASTER",
}

SW_NAMES = {
    "9000": "OK", "6985": "DENIED", "b101": "BAD_STATE", "b102": "BAD_MAC",
    "b103": "BAD_CERT", "b104": "SOLD_OUT", "b105": "NO_MASTER",
    "b106": "HAS_MASTER", "b109": "TOO_MANY_ATTEMPTS",
}

app = Flask(__name__)
log_lock = threading.Lock()
log_entries = []
log_ids = itertools.count(1)


def log(dev, direction, label, hexstr):
    with log_lock:
        log_entries.append({
            "id": next(log_ids),
            "t": time.strftime("%H:%M:%S"),
            "dev": dev.upper(),
            "dir": direction,
            "label": label,
            "hex": hexstr,
        })
        del log_entries[:-200]


PAGE = """<!doctype html>
<meta charset="utf-8">
<title>presse cockpit</title>
<style>
  body { font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
         background:#f4efe6; color:#2b2b2b; margin:0; padding:20px;
         display:flex; gap:24px; justify-content:center; align-items:flex-start; }
  figure { margin:0; text-align:center; flex:0 0 auto; }
  figcaption { margin-bottom:10px; font-size:14px; }
  img { width:320px; border:10px solid #1a1a1a; border-radius:18px; cursor:pointer;
        background:#fff; display:block; }
  #wire { flex:1 1 420px; max-width:520px; }
  #wire h3 { margin:0 0 6px; font-size:13px; font-weight:normal; color:#7a6a55; }
  #feed { background:#fffdf8; border:1px solid #d8cdbb; border-radius:8px;
          height:640px; overflow-y:auto; padding:8px 10px; font-size:12px; }
  .row { padding:3px 0; border-bottom:1px dotted #eee4d4; }
  .cmd  { color:#8a4b2d; }
  .resp { color:#3d6b4f; }
  .hex { color:#b0a48f; word-break:break-all; font-size:10px; display:block; }
</style>
<figure>
  <figcaption>Flex A - master</figcaption>
  <img id="a" alt="Flex A screen">
</figure>
<div id="wire">
  <h3>the wire: every byte this untrusted relay carries</h3>
  <div id="feed"></div>
</div>
<figure>
  <figcaption>Flex B - receiver</figcaption>
  <img id="b" alt="Flex B screen">
</figure>
<script>
const W = 480, H = 600;
for (const id of ["a", "b"]) {
  const img = document.getElementById(id);
  const refresh = () => { img.src = `/${id}/screenshot?ts=${Date.now()}`; };
  setInterval(refresh, 400);
  refresh();
  img.addEventListener("click", async (ev) => {
    const r = img.getBoundingClientRect();
    const x = Math.round((ev.clientX - r.left) / r.width * W);
    const y = Math.round((ev.clientY - r.top) / r.height * H);
    await fetch(`/${id}/finger`, {
      method: "POST",
      headers: {"Content-Type": "application/json"},
      body: JSON.stringify({x, y, action: "press-and-release"}),
    });
  });
}
let lastId = 0;
async function poll() {
  const r = await fetch(`/log?after=${lastId}`);
  const rows = await r.json();
  const feed = document.getElementById("feed");
  for (const e of rows) {
    lastId = e.id;
    const div = document.createElement("div");
    div.className = `row ${e.dir === ">" ? "cmd" : "resp"}`;
    const arrow = e.dir === ">" ? "&rarr;" : "&larr;";
    const short = e.hex.length > 120 ? e.hex.slice(0, 120) + `&hellip; (${e.hex.length/2} bytes)` : e.hex;
    div.innerHTML = `${e.t} ${arrow} ${e.dev}  <b>${e.label}</b><span class="hex">${short}</span>`;
    feed.appendChild(div);
  }
  if (rows.length) feed.scrollTop = feed.scrollHeight;
}
setInterval(poll, 350);
</script>
"""


@app.get("/")
def index():
    return PAGE


@app.get("/log")
def get_log():
    after = int(request.args.get("after", 0))
    with log_lock:
        return jsonify([e for e in log_entries if e["id"] > after])


@app.get("/<dev>/screenshot")
def screenshot(dev):
    r = requests.get(f"{DEVICES[dev]}/screenshot", timeout=5)
    return Response(r.content, mimetype="image/png")


@app.get("/<dev>/events")
def events(dev):
    r = requests.get(f"{DEVICES[dev]}/events", timeout=5)
    return jsonify(r.json())


@app.post("/<dev>/finger")
def finger(dev):
    r = requests.post(f"{DEVICES[dev]}/finger", json=request.get_json(), timeout=5)
    return Response(status=r.status_code)


@app.post("/<dev>/apdu")
def apdu(dev):
    payload = request.get_json()
    raw = bytes.fromhex(payload["data"])
    ins_name = INS_NAMES.get(raw[1], f"INS_{raw[1]:02x}") if len(raw) > 1 else "?"
    log(dev, ">", ins_name, payload["data"])
    r = requests.post(f"{DEVICES[dev]}/apdu", json=payload, timeout=600)
    resp_hex = r.json().get("data", "")
    sw = resp_hex[-4:]
    log(dev, "<", f"{ins_name}: {SW_NAMES.get(sw, sw)}", resp_hex)
    return jsonify(r.json())


if __name__ == "__main__":
    app.run(host="127.0.0.1", port=5050, threaded=True)
