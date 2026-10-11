"""Choose a libc function that RELEASE provides but the host's glibc does not.

Prints the earliest such function (by first snapshot, then name), or nothing
when RELEASE adds no libc function beyond HOST. Used to check that the runner
refuses a program the host's glibc cannot load.
"""
import csv
import re
import sys

path, release, host = sys.argv[1:4]
with open(path, encoding="utf-8") as stream:
    reader = csv.DictReader(stream, delimiter="\t")
    labels = [f for f in reader.fieldnames if re.fullmatch(r"2\.[0-9]+", f)]
    if release not in labels:
        raise SystemExit("Unknown release evidence")
    matrix = list(reader)
host = tuple(map(int, host.split(".")[:2]))
older = [label for label in labels if tuple(map(int, label.split("."))) <= host]
if not older:
    raise SystemExit("Host glibc predates the matrix; cannot choose a symbol")
known = {r["symbol"] for r in matrix if r[older[-1]] != "absent"}
candidates = sorted(
    (labels.index(r["first_observed_snapshot"]), r["symbol"]) for r in matrix
    if r["library"] == "c" and r[release] == "F" and r["symbol"] not in known)
if candidates:
    print(candidates[0][1])
