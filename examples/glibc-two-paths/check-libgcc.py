"""Check the generated libgcc_s stub, the real run-time libgcc_s, and executables.

check-libgcc.py stub WORK REPO
    compiler-runtime/libgcc_s.so.1 (the link-only stub) exports exactly the recorded
    GCC 4.8.5 interface, with GCC's version chain, no dependencies and no code that
    can run. run-runtime/libgcc_s.so.1 (the builder's real library, used to run the
    tests) defines every recorded definition; when it is GCC 4.8.5's own library
    (CentOS 7 builders), its exports must equal the record exactly.
check-libgcc.py newer REPO LIBGCC_S
    Prints a function LIBGCC_S (the host's runtime) defines under a version GCC
    4.8.5's runtime lacks, for the negative link test.
check-libgcc.py imports REPO READELF_OUTPUT...
    Each executable (readelf --dyn-syms --version-info) imports from libgcc_s only
    definitions GCC 4.8.5's runtime provides, and needs only its versions.
"""
import pathlib
import re
import subprocess
import sys

GCC_4_8_5_SHA256 = "8f148d8b7ed050f93064fafca6df434706844cf8346d5feed8ecce8c67f39c68"


def require(condition, message):
    if not condition:
        raise SystemExit("check-libgcc.py: " + message)


def interface(repo):
    """The recorded table: (symbol, version) -> (default, type, size)."""
    lines = [l for l in (repo / "src/libgcc_s-gcc-4.8.5.tsv").read_text().splitlines() if not l.startswith("#")]
    require(lines[0] == "symbol\tversion\tdefault\ttype\tsize\talignment", "unexpected interface header")
    table = {}
    for line in lines[1:]:
        symbol, version, default, kind, size, _ = line.split("\t")
        table[(symbol, version)] = (default == "yes", kind, int(size))
    require(len(table) == 138, "interface is not GCC 4.8.5's 138 definitions")
    return table


def chain(table):
    def key(version):
        return tuple(int(part) for part in version[len("GCC_"):].split("."))
    return sorted({version for _, version in table}, key=key)


def readelf(*args):
    return subprocess.run(["readelf", "--wide", *args], check=True, capture_output=True, text=True).stdout


def exports(path):
    """(symbol, version) -> (default, type, size) for defined global symbols,
    without the absolute symbols that name each version."""
    result = {}
    for line in readelf("--dyn-syms", str(path)).splitlines():
        fields = line.split()
        if len(fields) < 8 or not re.fullmatch(r"\d+:", fields[0]):
            continue
        _, _, size, kind, binding, _, section, name = fields[:8]
        if section in ("UND", "ABS") or binding == "LOCAL":
            continue
        separator = "@@" if "@@" in name else "@"
        require(separator in name, "unversioned export " + name + " in " + str(path))
        symbol, version = name.split(separator, 1)
        result[(symbol, version)] = (separator == "@@", kind, int(size))
    return result


def definitions(path):
    """Version definitions in order, each with its parent (or None)."""
    text = readelf("--version-info", str(path))
    section = text.split("Version definition section", 1)
    require(len(section) == 2, "no version definitions in " + str(path))
    section = section[1].split("Version needs section", 1)[0].split("Version symbols section", 1)[0]
    nodes, current = [], None
    for line in section.splitlines():
        match = re.search(r"Flags: (\S+)\s+Index: \d+\s+Cnt: \d+\s+Name: (\S+)", line)
        if match:
            current = [match[2], None]
            if match[1] != "BASE":
                nodes.append(current)
            continue
        match = re.search(r"Parent 1: (\S+)", line)
        if match and current:
            current[1] = match[1]
    return [tuple(node) for node in nodes]


def check_stub(work, repo):
    table = interface(repo)
    stub = work / "compiler-runtime/libgcc_s.so.1"
    actual = exports(stub)
    require(set(actual) == set(table), "stub exports differ from the GCC 4.8.5 interface")
    for key, (default, kind, size) in table.items():
        require(actual[key][:2] == (default, kind), "stub export property differs: " + "@".join(key))
        if kind == "OBJECT":
            require(actual[key][2] == size, "stub object size differs: " + "@".join(key))
    versions = chain(table)
    expected = [(version, versions[index - 1] if index else None) for index, version in enumerate(versions)]
    require(definitions(stub) == expected, "stub version chain differs from GCC 4.8.5's")
    dynamic = readelf("--dynamic", str(stub))
    require("(NEEDED)" not in dynamic and "Library soname: [libgcc_s.so.1]" in dynamic, "stub identity or dependencies")
    # Function bodies are traps: no code of the real runtime is present.
    disassembly = subprocess.run(["objdump", "-d", "--section=.text", str(stub)], check=True,
                                 capture_output=True, text=True).stdout
    # "ADDRESS:\tBYTES\tINSTRUCTION"; lines with only bytes continue the previous one.
    instructions = [l.split("\t")[2].strip() for l in disassembly.splitlines()
                    if re.match(r"\s+[0-9a-f]+:\t", l) and len(l.split("\t")) >= 3]
    # Besides one ud2 per function, only the assembler's alignment padding.
    padding = [i for i in instructions if "nop" in i or re.fullmatch(r"xchg\s+%ax,%ax", i)]
    traps = [i for i in instructions if i == "ud2"]
    functions = sum(1 for (default, kind, _) in table.values() if kind == "FUNC")
    require(len(traps) + len(padding) == len(instructions) and len(traps) == functions,
            "stub contains code other than traps")
    require((work / "compiler-runtime/libgcc_s.so").resolve() == stub.resolve(), "libgcc_s.so must name the stub")

    real = work / "run-runtime/libgcc_s.so.1"
    provided = exports(real)
    missing = sorted("@".join(key) for key in table if key not in provided)
    require(not missing, "run-time libgcc_s lacks GCC 4.8.5 definitions: " + ", ".join(missing))
    sha256 = subprocess.run(["sha256sum", str(real)], check=True, capture_output=True, text=True).stdout.split()[0]
    if sha256 == GCC_4_8_5_SHA256:
        require(provided == table, "GCC 4.8.5's own libgcc_s differs from the recorded interface")
        require(definitions(real) == expected, "GCC 4.8.5's own version chain differs from the record")
        print("libgcc_s stub: equals the recorded GCC 4.8.5 interface and GCC 4.8.5's own library")
    else:
        print("libgcc_s stub: equals the recorded GCC 4.8.5 interface; the run-time library provides all of it")


def check_imports(repo, outputs):
    table = interface(repo)
    versions = set(chain(table))
    for output in outputs:
        text = pathlib.Path(output).read_text()
        imports = set()
        for line in text.splitlines():
            fields = line.split()
            if len(fields) >= 8 and re.fullmatch(r"\d+:", fields[0]) and fields[6] == "UND":
                match = re.fullmatch(r"(\S+)@(GCC_[\d.]+)(?: \(\d+\))?", " ".join(fields[7:9]))
                if match:
                    imports.add((match[1], match[2]))
        needs = re.findall(r"File: libgcc_s\.so\.1\s+Cnt: \d+((?:\s+0x[0-9a-f]+:\s+Name: \S+\s+Flags: \S+\s+Version: \d+)+)", text)
        needed = set(re.findall(r"Name: (\S+)", " ".join(needs)))
        require(imports, output + ": no libgcc_s import; the check would be vacuous")
        require(needed <= versions, output + ": needs libgcc_s versions beyond GCC 4.8.5: " + ", ".join(sorted(needed - versions)))
        beyond = sorted("@".join(i) for i in imports if i not in table)
        require(not beyond, output + ": imports beyond GCC 4.8.5's libgcc_s: " + ", ".join(beyond))
        print(output + ": " + str(len(imports)) + " libgcc_s imports, all in GCC 4.8.5's runtime")


def newer(repo, library):
    """A default definition of LIBRARY under a version GCC 4.8.5's runtime lacks:
    the oldest such version, then the first name."""
    versions = set(chain(interface(repo)))
    candidates = [(tuple(int(p) for p in version[len("GCC_"):].split(".")), symbol)
                  for (symbol, version), (default, kind, _) in exports(pathlib.Path(library)).items()
                  if default and kind == "FUNC" and version.startswith("GCC_") and version not in versions]
    require(candidates, library + " has no libgcc_s interface newer than GCC 4.8.5's")
    print(min(candidates)[1])


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "stub":
        check_stub(pathlib.Path(sys.argv[2]).resolve(), pathlib.Path(sys.argv[3]).resolve())
    elif len(sys.argv) == 4 and sys.argv[1] == "newer":
        newer(pathlib.Path(sys.argv[2]).resolve(), sys.argv[3])
    elif len(sys.argv) >= 4 and sys.argv[1] == "imports":
        check_imports(pathlib.Path(sys.argv[2]).resolve(), sys.argv[3:])
    else:
        raise SystemExit(__doc__)
