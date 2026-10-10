"""Membership oracle shared by every provider; unknown evidence fails the gate."""
import csv
import sys

path, release, library, symbol, version = sys.argv[1:6]
with open(path, encoding="utf-8") as stream:
    reader = csv.DictReader(stream, delimiter="\t")
    if release not in reader.fieldnames or library not in ("c", "m", "ld", "resolv", "pthread", "dl", "rt", "util"):
        raise SystemExit("Unknown release/library evidence; cannot choose an acceptance test")
    matches = [r for r in reader if (r["library"], r["symbol"], r["version"]) == (library, symbol, version)]
if len(matches) > 1:
    raise SystemExit("Ambiguous matrix interface")
result = "reject" if not matches or matches[0][release] == "absent" else "accept"
if result == "accept" and matches[0][release] != "F":
    raise SystemExit("This function probe requires function-kind evidence")
if len(sys.argv) == 7 and result != sys.argv[6]:
    raise SystemExit("Probe expectation disagrees with the matrix: " + symbol + "@" + version)
print(result)
