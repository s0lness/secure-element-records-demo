"""Demo cockpit: both emulated Flexes side by side, clickable.

The stock Speculos web UI only forwards Nano-style buttons, not touch, so a
Flex screen is view-only there. This page polls each device's /screenshot and
turns clicks into /finger touches, proxied same-origin (no CORS games).

    python3 relay/cockpit.py   ->   http://localhost:5050
"""

import requests
from flask import Flask, Response, request

DEVICES = {"a": "http://127.0.0.1:5001", "b": "http://127.0.0.1:5002"}

app = Flask(__name__)

PAGE = """<!doctype html>
<meta charset="utf-8">
<title>presse cockpit</title>
<style>
  body { font-family: ui-monospace, monospace; background:#f4efe6; color:#2b2b2b;
         display:flex; gap:40px; justify-content:center; padding:24px; }
  figure { margin:0; text-align:center; }
  figcaption { margin-bottom:10px; font-size:15px; }
  img { width:360px; border:10px solid #1a1a1a; border-radius:18px; cursor:pointer;
        background:#fff; display:block; }
</style>
<figure>
  <figcaption>Flex A - master</figcaption>
  <img id="a" alt="Flex A screen">
</figure>
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
</script>
"""


@app.get("/")
def index():
    return PAGE


@app.get("/<dev>/screenshot")
def screenshot(dev):
    r = requests.get(f"{DEVICES[dev]}/screenshot", timeout=5)
    return Response(r.content, mimetype="image/png")


@app.post("/<dev>/finger")
def finger(dev):
    r = requests.post(f"{DEVICES[dev]}/finger", json=request.get_json(), timeout=5)
    return Response(status=r.status_code)


if __name__ == "__main__":
    app.run(host="127.0.0.1", port=5050)
