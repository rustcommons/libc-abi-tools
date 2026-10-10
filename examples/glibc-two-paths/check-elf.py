"""Independent GNU readelf comparison and provenance for the real-input experiment."""
import collections
import csv
import hashlib
import json
import pathlib
import re
import sys

work = pathlib.Path(sys.argv[1]).resolve()
repo = pathlib.Path(sys.argv[2]).resolve()
out = work / "elf-metadata"
libraries = dict(zip(
    ("c", "m", "ld", "resolv", "pthread", "dl", "rt", "util"),
    ("libc.so.6", "libm.so.6", "ld-linux-x86-64.so.2", "libresolv.so.2",
     "libpthread.so.0", "libdl.so.2", "librt.so.1", "libutil.so.1"),
))


def rows(path):
    with path.open() as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def reference(path):
    text = path.read_text()
    versions = set(re.findall(r"\bName: (\S+)", text))
    alignments = {}
    for line in text.splitlines():
        match = re.match(r"\s*\[\s*(\d+)\]\s+(.+)$", line)
        if match:
            alignments[match[1]] = int(match[2].split()[-1])
    entries = []
    for line in text.splitlines():
        match = re.fullmatch(r"\s*\d+:\s+(\S+)\s+(\S+)\s+(.+?)\s+(GLOBAL|WEAK|LOCAL|UNIQUE)\s+(DEFAULT|INTERNAL|HIDDEN|PROTECTED)\s+(UND|ABS|COM|\d+)\s*(\S*)(?:\s+\(\d+\))?\s*", line)
        if not match:
            continue
        value, size, kind, binding, visibility, section, name = match.groups()
        # GNU readelf leaves type 10 unnamed for old DSOs using System V OSABI.
        if kind == "<OS specific>: 10":
            kind = "IFUNC"
        if section == "UND" or binding == "LOCAL":
            continue
        if section == "ABS" and name in versions:
            continue
        if "@@" in name:
            name, version = name.split("@@", 1)
            default = "yes"
        elif "@" in name:
            name, version = name.split("@", 1)
            default = "no"
        else:
            version, default = "-", "-"
        entries.append((name, version, default, kind,
                        "GNU_UNIQUE" if binding == "UNIQUE" else binding, visibility,
                        {"ABS": 65521, "COM": 65522}.get(section, int(section) if section.isdigit() else -1),
                        alignments.get(section, 0), int(value, 16), int(size, 16) if size.startswith("0x") else int(size)))
    return collections.Counter(entries)


pinned = rows(out / "pinned-2.17.tsv")
manifest = {
    "schema": 1,
    "target": "x86_64-unknown-linux-gnu",
    "release_label": "2.17",
    "release_label_is_asserted_from_input_provenance": True,
    "upstream_pin_sha256": digest(repo / "data/glibc/UPSTREAM.toml"),
    "package_manifest_sha256": digest(work / "logs/packages.txt"),
    "source_archive_sha256": digest(work / "glibc-2.17.tar.xz"),
    "source_adjustment_sha256": digest(work / "logs/source-adjustment.patch"),
    "input_sources": {
        "prebuilt": {
            "builder_image": "docker.io/library/centos@sha256:dead07b4d8ed7e29e98de0f4504d87e8880d4347859d839686a31da35a3b532f",
            "development_package_repository": "https://vault.centos.org/7.9.2009/os/x86_64/",
            "packages": "../logs/packages.txt",
        },
        "source": {
            "archive": "https://ftp.gnu.org/gnu/glibc/glibc-2.17.tar.xz",
            "configuration": "--prefix=/usr --disable-werror --enable-kernel=2.6.32",
            "compiler": "../logs/builder-gcc.txt",
            "adjustment": "../logs/source-adjustment.patch",
        },
    },
    "files": [],
}
differences = []
for route in ("prebuilt", "source"):
    for library, soname in libraries.items():
        metadata = rows(out / (route + "-" + library + ".tsv"))
        native = collections.Counter(
            (s["symbol"], s["version"], s["default"], s["type"], s["binding"], s["visibility"],
             int(s["section"]), int(s["section_alignment"]), int(s["value"], 16), int(s["size"])) for s in metadata
        )
        expected = reference(out / (route + "-" + library + ".readelf.txt"))
        if native != expected:
            raise SystemExit("readelf mismatch: " + route + "/" + library + " " +
                             str({"native_only": list((native - expected).elements())[:5],
                                  "readelf_only": list((expected - native).elements())[:5]}))
        if not metadata or any(s["soname"] != soname for s in metadata):
            raise SystemExit("Unexpected or empty SONAME: " + route + "/" + library)
        actual = {}
        for s in metadata:
            if re.fullmatch(r"GLIBC_\d+\.\d+(?:\.\d+)?", s["version"]) and s["visibility"] in ("DEFAULT", "PROTECTED"):
                kind = {"FUNC": "F", "IFUNC": "F", "OBJECT": "D", "TLS": "T"}.get(s["type"], s["type"])
                actual[(s["symbol"], s["version"])] = (kind, s["size"] if kind in ("D", "T") else "-")
        historical = {(s["symbol"], s["version"]): (s["kind"], s["size"])
                      for s in pinned if s["library"] == library}
        changes = [{"symbol": key[0], "version": key[1], "pinned": historical.get(key), "elf": actual.get(key)}
                   for key in sorted(actual.keys() | historical.keys()) if actual.get(key) != historical.get(key)]
        differences.append({"route": route, "library": library, "differences": changes})
        path = work / route / "lib64" / soname
        counts = dict(sorted(collections.Counter(s["type"] for s in metadata).items()))
        manifest["files"].append({"route": route, "library": library, "soname": soname,
                                  "input_sha256": digest(path), "metadata_sha256": digest(out / (route + "-" + library + ".tsv")),
                                  "defined_symbols": len(metadata), "types": counts,
                                  "same_range_groups": len({s["same_range"] for s in metadata if s["same_range"] != "-"}),
                                  "pinned_differences": len(changes)})
        print(f"{route}/{library}: {len(metadata)} definitions match readelf; {len(changes)} pinned ABI differences")
(out / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
(out / "pinned-differences.json").write_text(json.dumps(differences, indent=2, sort_keys=True) + "\n")
