"""Explicit experiment release selection; bootstrap files are not target runtimes."""
import os
import pathlib
import re
import tomllib

RELEASE = os.environ.get("ABI_GLIBC_RELEASE", "2.17")
if RELEASE == "2.17":
    ROUTES = ("prebuilt", "source")
    CONFIGURATION = "--prefix=/usr --disable-werror --enable-kernel=2.6.32"
else:
    # Any later release needs its own pinned inputs; there is no fallback.
    inputs = pathlib.Path(__file__).resolve().parents[1] / ("glibc-" + RELEASE) / "INPUTS.toml"
    if not re.fullmatch(r"2\.[0-9]+", RELEASE) or not inputs.is_file():
        raise SystemExit("Experiment release must be 2.17 or have examples/glibc-RELEASE/INPUTS.toml")
    pinned = tomllib.loads(inputs.read_text(encoding="utf-8"))
    if pinned.get("release") != RELEASE:
        raise SystemExit("INPUTS.toml release does not match " + RELEASE)
    ROUTES = ("source",)
    CONFIGURATION = pinned["configuration"]
