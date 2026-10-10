"""Independent readelf audit of expanded stubs and retained real-runtime inputs."""
import csv
import itertools
import hashlib
import json
import pathlib
import re
import subprocess
import sys

work = pathlib.Path(sys.argv[1]).resolve()
repo = pathlib.Path(sys.argv[2]).resolve()
manifest = {"schema": 1, "scope": "experimental selected exports",
            "target_release_label": "2.17", "artifacts": []}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def readelf(path):
    return subprocess.check_output(["readelf", "--wide", "--dyn-syms", "--dynamic",
                                    "--section-headers", str(path)], text=True)


def exports(text):
    entries = {}
    nodes = set(re.findall(r"\b(GLIBC_[\d.]+)\b", text))
    for line in text.splitlines():
        fields = line.split()
        if len(fields) < 8 or not re.fullmatch(r"\d+:", fields[0]):
            continue
        _, value, size, kind, binding, visibility, section, name = fields[:8]
        if binding == "LOCAL" or section == "UND" or (section == "ABS" and name in nodes):
            continue
        if "@@" in name:
            name, version = name.split("@@", 1)
            default = "yes"
        elif "@" in name:
            name, version = name.split("@", 1)
            default = "no"
        else:
            version, default = "-", "-"
        entries[(name, version)] = (default, kind, binding, visibility, int(size), section, int(value, 16))
    return entries


def require(condition, message):
    if not condition:
        raise SystemExit(message)


for label in ("prebuilt", "source", "fixture"):
    directory = work / ("expanded-fixture" if label == "fixture" else "expanded-stub-" + label)
    soname = "libfixture.so.1" if label == "fixture" else "libc.so.6"
    text = readelf(directory / soname)
    (work / "logs" / ("expanded-" + label + "-exports.txt")).write_text(text)
    actual = exports(text)
    with (directory / "selected-elf.tsv").open() as stream:
        selected = list(csv.DictReader(stream, delimiter="\t"))
    expected_keys = {(s["symbol"], s["version"]) for s in selected}
    require(set(actual) == expected_keys, "Unexpected expanded exports: " + label)
    require("(NEEDED)" not in text and "[" + soname + "]" in text, "Wrong expanded DSO dependencies/SONAME")
    alignments = {}
    for line in text.splitlines():
        match = re.match(r"\s*\[\s*(\d+)\]\s+(.+)$", line)
        if match:
            alignments[match[1]] = int(match[2].split()[-1])
    for symbol in selected:
        key = (symbol["symbol"], symbol["version"])
        properties = actual[key]
        require(properties[:4] == (symbol["default"], symbol["type"], symbol["binding"], symbol["visibility"]),
                "Expanded symbol property mismatch: " + str(key))
        size = 2 if symbol["type"] in ("FUNC", "IFUNC") else int(symbol["size"])
        require(properties[4] == size, "Expanded symbol size mismatch: " + str(key))
        require(alignments[properties[5]] >= max(1, int(symbol["section_alignment"])), "Insufficient section alignment")
    for left, right in itertools.combinations(selected, 2):
        observed = (left["section"], left["value"], left["type"], left["size"]) == \
                   (right["section"], right["value"], right["type"], right["size"])
        a = actual[(left["symbol"], left["version"])]
        b = actual[(right["symbol"], right["version"])]
        retained = (a[1], a[4], a[5], a[6]) == (b[1], b[4], b[5], b[6])
        require(observed == retained, "Shared range was lost or invented: " + label)
    reference = repo / "tests/fixtures/elf/symbols.so" if label == "fixture" else work / label / "lib64/libc.so.6"
    manifest["artifacts"].append({
        "label": label, "soname": soname, "definitions": len(selected),
        "reference_sha256": digest(reference),
        "selected_elf_sha256": digest(directory / "selected-elf.tsv"),
        "generated_sha256": digest(directory / soname),
        "assembly_sha256": digest(directory / ("fixture.S" if label == "fixture" else "libc.S")),
        "version_script_sha256": digest(directory / ("fixture.map" if label == "fixture" else "libc.map")),
    })
    if label != "fixture":
        executable = (work / "logs" / ("expanded-" + label + "-executable.txt")).read_text()
        require("memcpy@GLIBC_2.2.5" in executable and "memcpy@GLIBC_2.14" in executable,
                "Missing executable memcpy versions")
        require("R_X86_64_COPY" in executable and "stdout@GLIBC_2.2.5" in executable,
                "Object copy-relocation was not exercised")
        require("(RPATH)" not in executable and "(RUNPATH)" not in executable, "Stub path embedded in executable")
        loaded = (work / "logs" / ("expanded-" + label + "-runtime-libraries.txt")).read_text()
        require(str(work / label / "lib64/libc.so.6") in loaded and "expanded-stub" not in loaded,
                "Wrong expanded runtime libc")
        link_map = (work / "logs" / ("expanded-" + label + "-link.map")).read_text()
        require("libc_nonshared.a(elf-init.oS)" in link_map, "Missing real nonshared startup code")
        for line in link_map.splitlines():
            if line.startswith("LOAD ") and ".so" in line:
                require(line == "LOAD " + str(directory / soname), "Unexpected real shared link input")
    print(label + ": verified versions, kinds, sizes, binding, visibility, alignment, and shared ranges")

for route in ("prebuilt", "source"):
    executable = (work / "logs" / ("expanded-fixture-" + route + "-executable.txt")).read_text()
    require("R_X86_64_TPOFF64" in executable and "tls_data@GLIBC_2.2.5" in executable,
            "Fixture did not exercise TLS relocation")
    require("R_X86_64_COPY" in executable and "shared_data@GLIBC_2.2.5" in executable,
            "Fixture did not exercise alias copy-relocation")
    require("foo@GLIBC_2.2.5" in executable and "foo@GLIBC_2.14" in executable,
            "Fixture versions missing")
    require("GLIBC_2.34" not in executable and "(RPATH)" not in executable and "(RUNPATH)" not in executable,
            "Host startup or generated paths in fixture executable")
    loaded = (work / "logs" / ("expanded-fixture-" + route + "-runtime-libraries.txt")).read_text()
    require(str(work / route / "lib64/libc.so.6") in loaded and
            str(work / "fixture-runtime/libfixture.so.1") in loaded and "expanded-" not in loaded,
            "Fixture loaded generated or wrong runtime libraries")
    link_map = (work / "logs" / ("expanded-fixture-" + route + "-link.map")).read_text()
    allowed = {"LOAD " + str(work / ("expanded-stub-" + route) / "libc.so.6"),
               "LOAD " + str(work / "expanded-fixture/libfixture.so.1")}
    require("libc_nonshared.a(elf-init.oS)" in link_map, "Missing fixture nonshared startup")
    for line in link_map.splitlines():
        if line.startswith("LOAD ") and ".so" in line:
            require(line in allowed, "Unexpected fixture shared link input")
    print(route + ": verified old-runtime fixture TLS, versions, IFUNC, and data alias copy-relocation")

manifest["fixture_source_sha256"] = digest(repo / "tests/fixtures/elf/symbols.S")
manifest["fixture_versions_sha256"] = digest(repo / "tests/fixtures/elf/symbols.map")
manifest["upstream_pin_sha256"] = digest(repo / "data/glibc/UPSTREAM.toml")
manifest["real_input_metadata_manifest_sha256"] = digest(work / "elf-metadata/manifest.json")
manifest["executables"] = {name: digest(work / name) for name in
                           ("expanded-prebuilt", "expanded-source", "fixture-prebuilt", "fixture-source")}
(work / "logs/expanded-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
