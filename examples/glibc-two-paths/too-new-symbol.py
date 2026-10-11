"""Choose a libc function newer than RELEASE from the matrix for too-new link tests.

Keeps memfd_create (GLIBC_2.27) while it is newer, so earlier evidence stays
comparable; otherwise picks the earliest later libc function absent from every
library and version in RELEASE. With no later release in the matrix, falls back
to a name absent from every snapshot and says so on stderr.
"""
import csv
import re
import sys

FALLBACK = "glibc_abi_tools_absent_probe"
path, release = sys.argv[1:3]
with open(path, encoding="utf-8") as stream:
    reader = csv.DictReader(stream, delimiter="\t")
    labels = [f for f in reader.fieldnames if re.fullmatch(r"2\.[0-9]+", f)]
    if release not in labels:
        raise SystemExit("Unknown release evidence; cannot choose a too-new symbol")
    matrix = list(reader)
present = {r["symbol"] for r in matrix if r[release] != "absent"}
later = labels[labels.index(release) + 1:]
candidates = sorted(
    (later.index(r["first_observed_snapshot"]), r["symbol"]) for r in matrix
    if r["library"] == "c" and r["first_observed_snapshot"] in later
    and r[r["first_observed_snapshot"]] == "F" and r["symbol"] not in present)
if "memfd_create" not in present and any(s == "memfd_create" for _, s in candidates):
    print("memfd_create")
elif candidates:
    print(candidates[0][1])
else:
    if any(r["symbol"] == FALLBACK for r in matrix):
        raise SystemExit("Fallback probe name unexpectedly exists")
    print("No later release in the matrix; using a name absent from every snapshot", file=sys.stderr)
    print(FALLBACK)
