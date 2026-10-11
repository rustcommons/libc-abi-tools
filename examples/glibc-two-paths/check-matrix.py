"""Membership oracle shared by every provider; unknown evidence fails the gate.

LIBRARY may list several libraries separated by commas (for example "pthread,c"
for interfaces that glibc 2.34 moved into libc); the interface is accepted when
any listed library provides it in RELEASE.
"""
import csv
import sys

path, release, libraries, symbol, version = sys.argv[1:6]
libraries = libraries.split(",")
with open(path, encoding="utf-8") as stream:
    reader = csv.DictReader(stream, delimiter="\t")
    if release not in reader.fieldnames or not all(
            library in ("c", "m", "ld", "resolv", "pthread", "dl", "rt", "util") for library in libraries):
        raise SystemExit("Unknown release/library evidence; cannot choose an acceptance test")
    matches = [r for r in reader if r["library"] in libraries and (r["symbol"], r["version"]) == (symbol, version)]
if len({r["library"] for r in matches}) != len(matches):
    raise SystemExit("Ambiguous matrix interface")
present = [r for r in matches if r[release] != "absent"]
if len(present) > 1:
    raise SystemExit("Interface present in more than one listed library")
result = "accept" if present else "reject"
if result == "accept" and present[0][release] != "F":
    raise SystemExit("This function probe requires function-kind evidence")
if len(sys.argv) == 7 and result != sys.argv[6]:
    raise SystemExit("Probe expectation disagrees with the matrix: " + symbol + "@" + version)
print(result)
