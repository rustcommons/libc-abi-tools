"""Audit the complete pinned numeric GLIBC ABI and explicit Rust inputs/runtime."""
import csv
import hashlib
import itertools
import json
import pathlib
import re
import subprocess
import sys
from experiment import RELEASE, ROUTES

work = pathlib.Path(sys.argv[1]).resolve()
repo = pathlib.Path(sys.argv[2]).resolve()
libraries = dict(zip(
    ("c", "m", "ld", "resolv", "pthread", "dl", "rt", "util"),
    ("libc.so.6", "libm.so.6", "ld-linux-x86-64.so.2", "libresolv.so.2",
     "libpthread.so.0", "libdl.so.2", "librt.so.1", "libutil.so.1"),
))


def rows(path):
    with path.open() as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def exports(path):
    text = path.read_text()
    entries, alignments = {}, {}
    for line in text.splitlines():
        match = re.match(r"\s*\[\s*(\d+)\]\s+(.+)$", line)
        if match:
            alignments[match[1]] = int(match[2].split()[-1])
        fields = line.split()
        if len(fields) < 8 or not re.fullmatch(r"\d+:", fields[0]):
            continue
        _, value, size, kind, binding, visibility, section, name = fields[:8]
        if section == "UND" or binding == "LOCAL" or (section == "ABS" and name.startswith("GLIBC_")):
            continue
        separator = "@@" if "@@" in name else "@"
        require(separator in name, "Unexpected unversioned generated export")
        name, version = name.split(separator, 1)
        key = (name, version)
        require(key not in entries, "Duplicate generated export")
        entries[key] = (separator == "@@", kind, binding, visibility, int(size), section, int(value, 16))
    return text, entries, alignments


def overlaps(a, b):
    return a[0] == b[0] and a[1] < b[2] and b[1] < a[2]


pinned = rows(work / ("elf-metadata/pinned-" + RELEASE + ".tsv"))
manifest = {"schema": 1, "scope": "complete pinned public numeric GLIBC ABI, experimental environment",
            "release": RELEASE, "libraries": [], "executables": {},
            "compiler_runtime_sha256": digest(work / "compiler-runtime/libgcc_s.so.1"),
            "compiler_runtime_source_sha256": digest(work / "prebuilt/usr/lib64/libgcc_s.so.1"),
            "upstream_pin_sha256": digest(repo / "data/glibc/UPSTREAM.toml"),
            "real_input_manifest_sha256": digest(work / "elf-metadata/manifest.json")}
total = 0
for route in ROUTES:
    generated = work / ("full-libraries-" + route)
    for library, soname in libraries.items():
        selected = rows(generated / (library + ".elf.tsv"))
        text, actual, alignments = exports(work / "logs" / ("full-" + route + "-" + library + "-exports.txt"))
        expected = {(s["symbol"], s["version"]): s for s in pinned if s["library"] == library}
        require(set(actual) == set(expected) == {(s["symbol"], s["version"]) for s in selected},
                "Full ABI membership mismatch: " + route + "/" + library)
        require("(NEEDED)" not in text and "(SONAME)" in text and "[" + soname + "]" in text,
                "Wrong full DSO identity/dependencies")
        forward, inverse = {}, {}
        data = []
        for symbol in selected:
            key = (symbol["symbol"], symbol["version"])
            properties = actual[key]
            require(properties[:4] == (symbol["default"] == "yes", symbol["type"], symbol["binding"], symbol["visibility"]),
                    "Full export property mismatch: " + str(key))
            size = 2 if symbol["type"] in ("FUNC", "IFUNC") else int(symbol["size"])
            require(properties[4] == size, "Full export size mismatch: " + str(key))
            require(alignments[properties[5]] >= max(1, int(symbol["section_alignment"])), "Lost section alignment")
            abi = expected[key]
            kind = {"FUNC": "F", "IFUNC": "F", "OBJECT": "D", "TLS": "T"}[properties[1]]
            require(abi["kind"] == kind and (kind == "F" or int(abi["size"]) == size), "Pinned kind/size mismatch")
            original = (symbol["section"], int(symbol["value"], 16), symbol["type"], int(symbol["size"]))
            emitted = (properties[5], properties[6], properties[1], properties[4])
            if original[3]:
                require(forward.setdefault(original, emitted) == emitted, "Lost exact shared range")
                require(inverse.setdefault(emitted, original) == original, "Invented exact shared range")
            if kind in ("D", "T"):
                data.append((symbol, properties))
        for (a, x), (b, y) in itertools.combinations(data, 2):
            original_a = ((a["section"], a["type"]), int(a["value"], 16), int(a["value"], 16) + int(a["size"]))
            original_b = ((b["section"], b["type"]), int(b["value"], 16), int(b["value"], 16) + int(b["size"]))
            emitted_a = ((x[5], x[1]), x[6], x[6] + x[4])
            emitted_b = ((y[5], y[1]), y[6], y[6] + y[4])
            shared = overlaps(original_a, original_b)
            require(shared == overlaps(emitted_a, emitted_b), "Lost/invented data-view overlap")
            if shared:
                require(original_b[1] - original_a[1] == emitted_b[1] - emitted_a[1], "Wrong data-view offset")
        total += len(selected)
        manifest["libraries"].append({"route": route, "library": library, "soname": soname,
                                      "definitions": len(selected), "reference_sha256": digest(work / route / "lib64" / soname),
                                      "generated_sha256": digest(generated / soname),
                                      "selected_elf_sha256": digest(generated / (library + ".elf.tsv"))})
    executable = (work / "logs" / ("full-" + route + "-rust-executable.txt")).read_text()
    required = [tuple(map(int, value.split("."))) for value in re.findall(r"Name: GLIBC_([\d.]+)", executable)]
    require(required and max(required) <= tuple(map(int, RELEASE.split('.'))), "Rust requires newer glibc")
    require("(RPATH)" not in executable and "(RUNPATH)" not in executable, "Runtime search path embedded")
    loaded = (work / "logs" / ("full-" + route + "-rust-runtime-libraries.txt")).read_text()
    require(str(work / route / "lib64/libc.so.6") in loaded and
            str(work / "compiler-runtime/libgcc_s.so.1") in loaded and "full-libraries" not in loaded,
            "Wrong Rust runtime libc/unwinder")
    # Every explicitly resolved dependency must stay in the approved real trees.
    for resolved in re.findall(r"=>\s+(/\S+)", loaded):
        require(any(pathlib.Path(resolved).resolve().is_relative_to(root.resolve()) for root in
                    (work / route, work / "compiler-runtime")), "Unexpected host runtime: " + resolved)
    link_map = (work / "logs" / ("full-" + route + "-rust-link.map")).read_text()
    require("libc_nonshared.a" in link_map and str(work / route / "usr/lib64/crt1.o") in link_map,
            "Missing explicit target startup")
    for line in link_map.splitlines():
        if line.startswith("LOAD ") and ".so" in line:
            path = pathlib.Path(line[5:])
            require(path.resolve().is_relative_to(generated.resolve()) or
                    path.resolve().is_relative_to((work / "compiler-runtime").resolve()),
                    "Unexpected shared link input: " + line)
    manifest["executables"][route] = {"rust_sha256": digest(work / ("rust-full-" + route)),
                                      "c_sha256": digest(work / ("c-full-" + route)),
                                      "data_views_sha256": digest(work / ("data-views-full-" + route)),
                                      "required_glibc": [".".join(map(str, v)) for v in sorted(set(required))],
                                      "rust_link_arguments_sha256": digest(work / "logs" / ("full-" + route + "-rust-link-arguments.txt"))}
    views = (work / "logs" / ("full-" + route + "-data-views-executable.txt")).read_text()
    require("sys_errlist@GLIBC_2.2.5" in views and "sys_errlist@GLIBC_2.12" in views and "R_X86_64_COPY" in views,
            "Historical data-view relocations not exercised")
    require((work / "logs" / ("full-" + route + "-data-views-run.txt")).read_text().strip() ==
            "Historical array sizes and values work!", "Historical data view runtime failed")
    inputs = {}
    for line in link_map.splitlines():
        if line.startswith("LOAD "):
            path = pathlib.Path(line[5:])
            if path.is_file():
                inputs[line[5:]] = digest(path)
    manifest["executables"][route]["persistent_link_inputs_sha256"] = inputs
    expected_default = [r["version"] for r in rows(work / "elf-metadata" / (route + "-c.tsv"))
                        if r["symbol"] == "fmemopen" and r["default"] == "yes"]
    require(len(expected_default) == 1, "Missing/ambiguous measured fmemopen default")
    bound = (work / "logs" / ("full-" + route + "-fmemopen-default-executable.txt")).read_text()
    imported = set(re.findall(r"\bUND\s+fmemopen@(GLIBC_[\d.]+)", bound))
    require(imported == set(expected_default), "Unversioned fmemopen did not bind to the measured default")
    manifest["executables"][route]["fmemopen_default"] = expected_default[0]
    manifest["executables"][route]["fmemopen_default_sha256"] = digest(work / ("fmemopen-default-" + route))
    if RELEASE == "2.22":
        versions = (work / "logs" / ("full-" + route + "-fmemopen-executable.txt")).read_text()
        require("fmemopen@GLIBC_2.22" in versions and "fmemopen@GLIBC_2.2.5" in versions,
                "Both fmemopen interfaces must be imported")
        require((work / "logs" / ("full-" + route + "-fmemopen-run.txt")).read_text().strip() ==
                "Both fmemopen versions work!", "fmemopen runtime failed")
        manifest["executables"][route]["fmemopen_sha256"] = digest(work / ("fmemopen-full-" + route))
    else:
        require("fmemopen@GLIBC_2.22" in (work / "logs" / ("full-" + route + "-fmemopen-negative.txt")).read_text(),
                "2.17 must reject the 2.22 interface")
    print(route + ": full ABI exports, data views, Rust target requirements and real runtime inputs verified")
manifest["rust_source_sha256"] = digest(repo / "examples/glibc-two-paths/rust-hello.rs")
manifest["rust_compiler"] = (work / "logs/full-rustc.txt").read_text().strip()
manifest["definitions_checked"] = total
(work / "logs/full-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
print(str(total) + " generated definitions verified across " + str(8 * len(ROUTES)) + " DSOs")
