"""Assemble the full-demo storyboard page (screenshots + wire log embedded)."""

import base64
import json
import os

ROOT = os.path.join(os.path.dirname(__file__), "..")
SRC = os.path.join(ROOT, "docs", "screens", "full-demo")
OUT = os.environ.get(
    "DEMO_PAGE_OUT", os.path.join(ROOT, "docs", "screens", "full-demo", "demo.html")
)


def img64(name):
    with open(os.path.join(SRC, name), "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode()


def wire_rows():
    with open(os.path.join(SRC, "wire.json")) as f:
        entries = json.load(f)
    rows = []
    for e in entries:
        arrow = "&rarr;" if e["dir"] == ">" else "&larr;"
        cls = "cmd" if e["dir"] == ">" else "resp"
        h = e["hex"]
        if len(h) > 96:
            h = h[:96] + f"&hellip; ({len(e['hex']) // 2} bytes)"
        rows.append(
            f'<div class="wrow {cls}"><span class="wmeta">{e["t"]} {arrow} '
            f'{e["dev"]}</span> <b>{e["label"]}</b><span class="whex">{h}</span></div>'
        )
    return "\n".join(rows), len(entries)


def beat(num, title, does, screens, relay=None):
    imgs = "".join(
        f'<figure><img src="{img64(f)}" alt="{cap}"><figcaption>{cap}</figcaption></figure>'
        for f, cap in screens
    )
    relay_html = f'<pre class="relay">{relay}</pre>' if relay else ""
    return f"""
<section class="beat">
  <div class="beat-head"><span class="beat-num">{num}</span>
    <div><h2>{title}</h2><p class="does">{does}</p></div></div>
  <div class="screens">{imgs}</div>
  {relay_html}
</section>"""


wire_html, wire_count = wire_rows()

beats = [
    beat("1", "Deux Flex vierges",
         "Toi&nbsp;: <code>emu-up.sh</code> puis <code>cockpit.sh</code>, et les deux écrans apparaissent sur "
         "<code>localhost:5050</code>. Aucun master, aucune pressing, deux identités déjà nées dans le silicium.",
         [("01-a-home.png", "Flex A"), ("02-b-home.png", "Flex B")]),
    beat("2", "Cut : graver le master",
         "Toi&nbsp;: <code>demo_steps.py cut \"Nuits Roses\" 5</code>. Flex A affiche la revue&nbsp;; "
         "tu cliques <b>Cut the master</b> sur son écran. Le 5 est gravé pour toujours.",
         [("03-a-cut-review.png", "Flex A - revue de cut")],
         relay='master of "Nuits Roses" cut, edition of 5: now physics.'),
    beat("3", "Pairing : les 4 mots",
         "Toi&nbsp;: <code>demo_steps.py pair</code>. Le laptop relaie l'échange de clés (il ne voit "
         "aucun secret), puis <b>les deux écrans affichent les mêmes 4 mots</b>. Tu compares à voix haute, "
         "tu cliques <b>Words match</b> des deux côtés. Un relais menteur = mots différents.",
         [("05-a-sas.png", "Flex A"), ("06-b-sas.png", "Flex B - mots identiques")],
         relay="Flex A shows: eclipse / silver / phantom / bagpipe\n"
               "Flex B shows: eclipse / silver / phantom / bagpipe\nchannel authenticated."),
    beat("4", "Press : la copie 1 sur 5",
         "Toi&nbsp;: <code>demo_steps.py press</code>. A confirme le pressage (compteur 5&rarr;4 dans la puce, "
         "avant que le certificat sorte), B confirme la réception. La pressing est liée à la clé de B, à jamais.",
         [("07-a-press-offer.png", "Flex A - presser"), ("08-b-receive.png", "Flex B - recevoir")],
         relay="pressed. 4 remain in the master."),
    beat("5", "Verify : offline, sans confiance",
         "Toi&nbsp;: <code>demo_steps.py verify</code> (wifi coupé si tu veux). Chaîne de certificats vérifiée par "
         "un code indépendant + challenge-response prouvant que la clé vit dans B, maintenant.",
         [],
         relay='GENUINE: pressing 1 of 5 of "Nuits Roses", bound to Flex B, possession proven live.'),
]

html = f"""<title>presse - full demo</title>
<style>
:root {{
  --paper: #f4efe6; --paper2: #fffdf8; --ink: #2b2b2b; --muted: #7a6a55;
  --vinyl: #1a1a1a; --line: #d8cdbb; --press: #8a4b2d; --ok: #3d6b4f;
}}
@media (prefers-color-scheme: dark) {{ :root {{
  --paper: #171512; --paper2: #201d18; --ink: #e8e2d6; --muted: #a3947c;
  --vinyl: #000; --line: #3a352c; --press: #d08a63; --ok: #7fb894;
}} }}
:root[data-theme="dark"] {{
  --paper: #171512; --paper2: #201d18; --ink: #e8e2d6; --muted: #a3947c;
  --vinyl: #000; --line: #3a352c; --press: #d08a63; --ok: #7fb894;
}}
:root[data-theme="light"] {{
  --paper: #f4efe6; --paper2: #fffdf8; --ink: #2b2b2b; --muted: #7a6a55;
  --vinyl: #1a1a1a; --line: #d8cdbb; --press: #8a4b2d; --ok: #3d6b4f;
}}
* {{ box-sizing: border-box; }}
body {{ background: var(--paper); color: var(--ink); margin: 0;
  font: 16px/1.6 Georgia, "Times New Roman", serif; }}
main {{ max-width: 880px; margin: 0 auto; padding: 40px 20px 80px; }}
code, .relay, .wrow, .beat-num, figcaption, .eyebrow {{
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace; }}
.eyebrow {{ color: var(--muted); font-size: 12px; letter-spacing: .14em;
  text-transform: uppercase; }}
h1 {{ font-size: 34px; line-height: 1.15; margin: 6px 0 4px; text-wrap: balance; }}
.lede {{ color: var(--muted); margin: 0 0 36px; max-width: 62ch; }}
.beat {{ border-top: 1px solid var(--line); padding: 26px 0 30px; }}
.beat-head {{ display: flex; gap: 16px; align-items: baseline; }}
.beat-num {{ flex: 0 0 auto; width: 34px; height: 34px; border-radius: 50%;
  background: var(--vinyl); color: var(--paper); display: flex;
  align-items: center; justify-content: center; font-size: 15px; }}
h2 {{ font-size: 21px; margin: 0 0 6px; }}
.does {{ margin: 0; max-width: 62ch; }}
code {{ background: var(--paper2); border: 1px solid var(--line);
  border-radius: 4px; padding: 1px 6px; font-size: 13.5px; }}
.screens {{ display: flex; gap: 28px; flex-wrap: wrap; margin: 22px 0 0 50px; }}
figure {{ margin: 0; }}
figure img {{ width: 240px; max-width: 100%; display: block; background: #fff;
  border: 8px solid var(--vinyl); border-radius: 16px; }}
figcaption {{ font-size: 12px; color: var(--muted); margin-top: 8px; }}
.relay {{ background: var(--paper2); border: 1px solid var(--line); border-radius: 8px;
  color: var(--ok); font-size: 13px; padding: 12px 14px; margin: 18px 0 0 50px;
  white-space: pre-wrap; overflow-x: auto; }}
details {{ border-top: 1px solid var(--line); padding: 24px 0; }}
summary {{ cursor: pointer; font-size: 17px; }}
summary .eyebrow {{ margin-left: 10px; }}
.wire {{ background: var(--paper2); border: 1px solid var(--line); border-radius: 8px;
  margin-top: 16px; padding: 10px 14px; max-height: 420px; overflow-y: auto; }}
.wrow {{ font-size: 12px; padding: 4px 0; border-bottom: 1px dotted var(--line); }}
.wrow.cmd {{ color: var(--press); }}
.wrow.resp {{ color: var(--ok); }}
.wmeta {{ color: var(--muted); }}
.whex {{ display: block; color: var(--muted); opacity: .7; font-size: 10.5px;
  word-break: break-all; }}
.close {{ border-top: 1px solid var(--line); padding-top: 26px; max-width: 62ch; }}
</style>
<main>
  <p class="eyebrow">presse - demo run du 21 juillet 2026, deux Flex émulés, relay non-fiable</p>
  <h1>Une édition finie, pressée dans le silicium</h1>
  <p class="lede">Chaque écran ci-dessous est une capture réelle de la démo, exécutée de bout en bout
  par les vraies commandes. Cinq temps&nbsp;: cut, pairing, press, verify, et le fil d'octets
  que le laptop a transportés sans jamais rien pouvoir falsifier.</p>
  {"".join(beats)}
  <details>
    <summary>Le fil complet<span class="eyebrow">{wire_count} échanges APDU, tout ce que le relais a vu</span></summary>
    <div class="wire">{wire_html}</div>
  </details>
  <p class="close">Ce qui rend la démo vraie&nbsp;: le compteur 5&rarr;4 vit dans l'élément sécurisé et
  décrémente avant l'émission du certificat&nbsp;; les 4 mots divergent si le relais triche (testé,
  suite adversariale M4)&nbsp;; la vérification finale tourne sans réseau, contre la seule
  cryptographie. Même flow, mêmes écrans sur les deux Flex physiques&nbsp;: il ne manque que
  l'installation de usbipd-win.</p>
</main>
"""

with open(OUT, "w", encoding="utf-8") as f:
    f.write(html)
print(f"wrote {OUT} ({os.path.getsize(OUT) // 1024} KB)")
