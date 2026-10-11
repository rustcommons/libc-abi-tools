"""Explicit experiment release selection; bootstrap files are not target runtimes."""
import os

RELEASE = os.environ.get("ABI_GLIBC_RELEASE", "2.17")
if RELEASE not in ("2.17", "2.18", "2.19", "2.22"):
    raise SystemExit("Experiment release must be 2.17, 2.18, 2.19 or 2.22")
ROUTES = ("prebuilt", "source") if RELEASE == "2.17" else ("source",)
CONFIGURATION = "--prefix=/usr --disable-werror --enable-kernel=2.6.32" if RELEASE == "2.17" else "--prefix=/usr --libdir=/usr/lib64 --disable-werror --enable-kernel=3.2"
