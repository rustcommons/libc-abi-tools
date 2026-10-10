"""Strong isolated-DSO relocation probes; never execute generated artifacts."""
import csv
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
from experiment import RELEASE, ROUTES

LIBRARIES = dict(zip(
    ("c", "m", "ld", "resolv", "pthread", "dl", "rt", "util"),
    ("libc.so.6", "libm.so.6", "ld-linux-x86-64.so.2", "libresolv.so.2",
     "libpthread.so.0", "libdl.so.2", "librt.so.1", "libutil.so.1")))


def rows(path):
    with path.open(encoding="utf-8") as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def assembly(symbol, version, kind):
    require(re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", symbol), "Unsafe symbol")
    if version:
        require(re.fullmatch(r"GLIBC_[0-9]+(?:\.[0-9]+){1,2}", version), "Unsafe version")
    alias = "probe_reference" if version else symbol
    text = '.symver probe_reference,' + symbol + '@' + version + '\n' if version else ''
    if kind.startswith("T:"):
        # General-dynamic TLS relocation pair: no TLS resolver call is needed
        # to establish that the DSO provides this TLS definition.
        text += '.text\n.globl probe\nprobe:\n leaq ' + alias + '@tlsgd(%rip), %rax\n ret\n'
    else:
        # An address relocation covers functions (including IFUNC) and objects
        # without inventing signatures or accessing unknown object storage.
        text += '.data\n.globl probe\nprobe:\n .quad ' + alias + '\n'
    return text + '.section .note.GNU-stack,"",@progbits\n'


def main():
    work = pathlib.Path(sys.argv[1]).resolve()
    matrix_path = work / "elf-metadata/matrix.tsv"
    matrix = rows(matrix_path)
    require(matrix and RELEASE in matrix[0], "Unknown release evidence")
    keys = [(r["library"], r["symbol"], r["version"]) for r in matrix]
    require(len(keys) == len(set(keys)), "Duplicate matrix interfaces")
    require(all(r["target"] == "x86_64-unknown-linux-gnu" and r["library"] in LIBRARIES
                for r in matrix), "Unsupported matrix scope")
    output = work / "link-matrix"
    output.mkdir(exist_ok=True)
    environment = dict(os.environ, LC_ALL="C")
    # The current historical union has no TLS rows. Exercise the relocation
    # machinery independently so that this branch is still a tested contract.
    tls_source = output / "tls.S"
    tls_object = output / "tls.o"
    tls_library = output / "tls.so"
    tls_map = output / "tls.map"
    tls_source.write_text('.section .tbss,"awT",@nobits\n.globl fixture_tls\n'
                          '.type fixture_tls,@tls_object\n.size fixture_tls,8\nfixture_tls:\n.zero 8\n', encoding="utf-8")
    tls_map.write_text('GLIBC_2.17 { global: fixture_tls; local: *; };\n', encoding="utf-8")
    subprocess.run(["as", "--64", str(tls_source), "-o", str(tls_object)], check=True, env=environment)
    subprocess.run(["ld", "-shared", "--version-script=" + str(tls_map), str(tls_object),
                    "-o", str(tls_library)], check=True, env=environment)
    for version, accepted in (("GLIBC_2.17", True), ("GLIBC_2.18", False)):
        tls_source.write_text(assembly("fixture_tls", version, "T:8"), encoding="utf-8")
        subprocess.run(["as", "--64", str(tls_source), "-o", str(tls_object)], check=True, env=environment)
        result = subprocess.run(["ld", "-shared", "-z", "defs", str(tls_object), str(tls_library),
                                 "-o", str(output / "tls-probe.so")], capture_output=True,
                                text=True, env=environment)
        if accepted:
            require(result.returncode == 0, "TLS positive fixture: " + result.stderr)
            relocations = subprocess.check_output(["readelf", "--wide", "--relocs", str(output / "tls-probe.so")],
                                                  text=True, env=environment)
            require("R_X86_64_DTPMOD64" in relocations and "R_X86_64_DTPOFF64" in relocations,
                    "TLS fixture did not produce general-dynamic relocations")
        else:
            require(result.returncode != 0 and "undefined reference" in result.stderr
                    and "fixture_tls@GLIBC_2.18" in result.stderr, "TLS negative fixture failed")
    report = {"schema": 1, "release": RELEASE,
              "scope": "strong references against one explicit DSO; no execution",
              "matrix_sha256": hashlib.sha256(matrix_path.read_bytes()).hexdigest(),
              "synthetic_tls": "accepted GLIBC_2.17, rejected GLIBC_2.18; DTPMOD64/DTPOFF64 verified",
              "tools": {}, "routes": {}}
    for tool in ("as", "ld", "readelf"):
        report["tools"][tool] = subprocess.check_output([tool, "--version"], env=environment, text=True).splitlines()[0]
    for route in ROUTES:
        counts = {"accept": 0, "reject": 0, "default": 0, "tls": 0}
        details = []
        generated = work / ("full-libraries-" + route)
        directory = output / route
        directory.mkdir(exist_ok=True)

        def probe(library, symbol, version, kind, expected):
            source, obj, linked = (directory / name for name in ("probe.S", "probe.o", "probe.so"))
            source.write_text(assembly(symbol, version, kind), encoding="utf-8")
            assembled = subprocess.run(["as", "--64", str(source), "-o", str(obj)],
                                       capture_output=True, text=True, env=environment)
            require(assembled.returncode == 0, assembled.stderr)
            command = ["ld", "-shared", "-z", "defs", "--build-id=none", "-o", str(linked),
                       str(obj), str(generated / LIBRARIES[library])]
            result = subprocess.run(command, capture_output=True, text=True, env=environment)
            identity = symbol + ("@" + version if version else "")
            if expected:
                require(result.returncode == 0, route + "/" + library + "/" + identity + ": " + result.stderr)
                exports = subprocess.check_output(["readelf", "--wide", "--dyn-syms", str(linked)],
                                                  text=True, env=environment)
                require(any(" UND " in line and (identity in line.split() if version else
                            any(token.split("@")[0] == symbol for token in line.split()))
                            for line in exports.splitlines()), "Missing resolved import: " + identity)
            else:
                require(result.returncode != 0 and "undefined reference" in result.stderr
                        and identity in result.stderr, "Wrong rejection: " + identity + ": " + result.stderr)
            return result.stderr

        for row in matrix:
            cell = row[RELEASE]
            require(cell == "absent" or re.fullmatch(r"F|[DT]:[0-9]+", cell), "Invalid matrix kind")
            # For absent interfaces use the first observed kind. Kind changes
            # remain explicitly recorded in the matrix; this is an absence test.
            kind = cell if cell != "absent" else next(row[v] for v in matrix[0]
                if re.fullmatch(r"2\.[0-9]+", v) and row[v] != "absent")
            accepted = cell != "absent"
            diagnostic = probe(row["library"], row["symbol"], row["version"], kind, accepted)
            counts["accept" if accepted else "reject"] += 1
            counts["tls"] += kind.startswith("T:")
            details.append({"library": row["library"], "interface": row["symbol"] + "@" + row["version"],
                            "kind": kind, "expectation": "accept" if accepted else "reject",
                            "diagnostic": diagnostic})
        for library in LIBRARIES:
            # Defaults come from the actual measured input, never the largest
            # numeric namespace in the historical union.
            for row in rows(work / "elf-metadata" / (route + "-" + library + ".tsv")):
                if row["default"] != "yes" or not re.fullmatch(r"GLIBC_[0-9]+(?:\.[0-9]+){1,2}", row["version"]):
                    continue
                kind = "T:" + row["size"] if row["type"] == "TLS" else "F"
                # Unversioned assembly, followed by checking the bound version.
                probe(library, row["symbol"], None, kind, True)
                text = subprocess.check_output(["readelf", "--wide", "--dyn-syms", str(directory / "probe.so")],
                                               text=True, env=environment)
                identity = row["symbol"] + "@" + row["version"]
                require(any(" UND " in line and identity in line.split() for line in text.splitlines()),
                        "Wrong default binding: " + identity)
                counts["default"] += 1
                details.append({"library": library, "symbol": row["symbol"],
                                "expected_default": identity, "expectation": "default"})
        (directory / "results.json").write_text(json.dumps(details, indent=2) + "\n", encoding="utf-8")
        report["routes"][route] = {"counts": counts, "libraries": {
            library: {"stub_sha256": hashlib.sha256((generated / soname).read_bytes()).hexdigest(),
                      "native_metadata_sha256": hashlib.sha256(
                          (work / "elf-metadata" / (route + "-" + library + ".tsv")).read_bytes()).hexdigest()}
            for library, soname in LIBRARIES.items()}}
        print(route, counts, flush=True)
    (output / "manifest.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
