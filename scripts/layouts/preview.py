"""Render the layouts and screenshot them as one sheet, for checking by eye.

Usage: python scripts/layouts/preview.py <out dir> [name prefix]
"""

import os
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).parent
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

out = pathlib.Path(sys.argv[1]).resolve()
prefix = sys.argv[2] if len(sys.argv) > 2 else ""
subprocess.run([sys.executable, str(HERE / "render.py"), str(out)], check=True)
files = sorted(f for f in os.listdir(out) if f.endswith(".svg") and f.startswith(prefix))
cells = "".join(
    f'<div><div>{f}</div><img src="{f}" width="300"></div>' for f in files
)
(out / "view.html").write_text(
    '<html><body style="margin:0;background:#000;display:flex;flex-wrap:wrap;'
    f'gap:6px;font:12px sans-serif;color:#fc8">{cells}</body></html>',
    encoding="utf8",
)
rows = (len(files) + 2) // 3
subprocess.run(
    [EDGE, "--headless=new", "--disable-gpu", "--hide-scrollbars",
     f"--user-data-dir={out / '.edge'}", f"--window-size=930,{rows * 326 + 10}",
     f"--screenshot={out / 'sheet.png'}", (out / "view.html").as_uri()],
    check=True, capture_output=True,
)
print(out / "sheet.png")
