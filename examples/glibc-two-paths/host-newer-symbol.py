"""Choose a libc function whose symbol version is newer than the host's glibc.

Prints the function with the lowest such GLIBC version that RELEASE provides
(then by name), or nothing when RELEASE defines no version beyond HOST. A
program using it requires a version the host's libc.so.6 cannot define, which
is what the runner's host check must refuse. Functions merely absent from the
host under an old version would be a symbol lookup failure instead, which the
version check does not cover.
"""
import csv
import re
import sys

path, release, host = sys.argv[1:4]
host = tuple(map(int, host.split(".")[:2]))
with open(path, encoding="utf-8") as stream:
    reader = csv.DictReader(stream, delimiter="\t")
    if release not in reader.fieldnames:
        raise SystemExit("Unknown release evidence")
    matrix = list(reader)


def number(version):
    match = re.fullmatch(r"GLIBC_([0-9]+)\.([0-9]+)(?:\.([0-9]+))?", version)
    return tuple(int(part or 0) for part in match.groups()) if match else None


candidates = sorted(
    (number(r["version"]), r["symbol"]) for r in matrix
    if r["library"] == "c" and r[release] == "F" and number(r["version"])
    and number(r["version"])[:2] > host)
if candidates:
    print(candidates[0][1])
