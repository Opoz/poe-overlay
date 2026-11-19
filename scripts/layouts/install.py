"""Render our zone layouts into the app's layouts folder (a `poe-overlay`
subfolder, so the player's own images stay separate).

Usage: python scripts/layouts/install.py   (or: pnpm layouts)
"""

import os
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).parent
target = pathlib.Path(os.environ["APPDATA"]) / "com.opoz.poe-overlay" / "layouts" / "poe-overlay"
subprocess.run([sys.executable, str(HERE / "render.py"), str(target)], check=True)
