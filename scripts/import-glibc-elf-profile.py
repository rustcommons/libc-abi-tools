"""Import a deliberately collected, readelf-qualified profile without replacing history."""
import argparse
import csv
import hashlib
import io
import json
import pathlib
import re


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", required=True, type=pathlib.Path)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--route", choices=("source", "prebuilt"), required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[a-z0-9][a-z0-9.-]{0,127}", args.profile), "Invalid profile identifier")
    repo = pathlib.Path(__file__).resolve().parents[1]
    output = repo / "data/glibc-history"
    provenance = json.loads((output / "PROVENANCE.json").read_text())
    require(args.profile not in provenance["elf_profiles"], "Profile already exists; deliberate replacement requires review")
    metadata = args.work / "elf-metadata"
    manifest_path = metadata / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    release = manifest["release_label"]
    require(re.fullmatch(r"2\.[0-9]+", release) and manifest["target"] == "x86_64-unknown-linux-gnu",
            "Unsupported profile release/target")
    files = [f for f in manifest["files"] if f["route"] == args.route]
    require(len(files) == 8 and {f["library"] for f in files} ==
            {"c", "m", "ld", "resolv", "pthread", "dl", "rt", "util"}, "Incomplete library profile")
    require(manifest["upstream_pin_sha256"] == digest(repo / "data/glibc/SOURCE.tsv"), "Different ABI source pin")
    require(manifest["package_manifest_sha256"] == digest(args.work / "logs/packages.txt"), "Package evidence changed")
    if args.route == "source":
        require(manifest["source_archive_sha256"] == digest(args.work / ("glibc-" + release + ".tar.xz")),
                "Source archive changed")
        require(manifest["source_adjustment_sha256"] == digest(args.work / "logs/source-adjustment.patch"),
                "Source adjustment changed")
    with (output / "elf-evidence.tsv").open(newline="") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        fields, records = reader.fieldnames, list(reader)
    with (output / "symbols.tsv").open() as stream:
        matrix = list(csv.DictReader(stream, delimiter="\t"))
    require(matrix and release in matrix[0], "No complete matrix evidence for this release")
    for item in files:
        require(item["pinned_differences"] == 0, "Profile has unresolved ABI differences")
        library, soname = item["library"], item["soname"]
        require("/" not in soname and "\\" not in soname, "Unsafe SONAME")
        path = metadata / (args.route + "-" + library + ".tsv")
        require(item["metadata_sha256"] == digest(path), "Native metadata changed")
        require(item["input_sha256"] == digest(args.work / args.route / "lib64" / soname), "Real DSO changed")
        with path.open() as stream:
            public = [r for r in csv.DictReader(stream, delimiter="\t")
                      if re.fullmatch(r"GLIBC_[0-9]+\.[0-9]+(?:\.[0-9]+)?", r["version"])
                      and r["visibility"] in ("DEFAULT", "PROTECTED")]
        expected = {(r["symbol"], r["version"]): r[release] for r in matrix
                    if r["library"] == library and r[release] != "absent"}
        actual = {(r["symbol"], r["version"]): "F" if r["type"] in ("FUNC", "IFUNC")
                  else {"OBJECT": "D:", "TLS": "T:"}.get(r["type"], "unsupported:") + r["size"]
                  for r in public}
        require(len(actual) == len(public) and actual == expected, "Matrix mismatch: " + library)
        for row in public:
            records.append({"profile": args.profile, "release": release, "target": manifest["target"],
                            "library": library, **{k: row[k] for k in fields if k in row},
                            "input_sha256": item["input_sha256"], "metadata_sha256": item["metadata_sha256"]})
    provenance["elf_profiles"][args.profile] = {
        "release": release, "route": args.route, "target": manifest["target"],
        "source_manifest_sha256": digest(manifest_path), "input_source": manifest["input_sources"][args.route],
        "source_archive_sha256": manifest["source_archive_sha256"] if args.route == "source" else None,
        "source_adjustment_sha256": manifest["source_adjustment_sha256"] if args.route == "source" else None,
        "package_manifest_sha256": manifest["package_manifest_sha256"], "files": files,
        "evidence_kind": "native ELF metadata independently compared with GNU readelf in the local qualified experiment"}
    records.sort(key=lambda r: (r["profile"], r["library"], r["symbol"],
                               tuple(map(int, r["version"][6:].split(".")))))
    buffer = io.StringIO(newline="")
    writer = csv.DictWriter(buffer, fieldnames=fields, delimiter="\t", lineterminator="\n")
    writer.writeheader()
    writer.writerows(records)
    (output / "elf-evidence.tsv").write_text(buffer.getvalue(), encoding="utf-8", newline="\n")
    (output / "PROVENANCE.json").write_text(json.dumps(provenance, indent=2, sort_keys=True) + "\n",
                                          encoding="utf-8", newline="\n")
    print("Imported", args.profile, "with", sum(r["profile"] == args.profile for r in records), "public definitions")


if __name__ == "__main__":
    main()
