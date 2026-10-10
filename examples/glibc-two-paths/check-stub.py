"""Check the deliberately narrow two-function ELF contract using GNU readelf."""
import pathlib
import hashlib
import subprocess
import sys


def output(*args):
    return subprocess.check_output(args, text=True)


def symbols(path):
    result = {}
    for line in output("readelf", "--dyn-syms", "--wide", str(path)).splitlines():
        fields = line.split()
        if len(fields) >= 8 and fields[0].rstrip(":").isdigit():
            result[fields[7]] = fields[3:7]  # Type, binding, visibility, index.
    return result


def require(condition, message):
    if not condition:
        raise SystemExit(message)


work = pathlib.Path(sys.argv[1]).resolve()
stub = work / "generated-stub/libc.so.6"
names = ("puts", "__libc_start_main")
exports = symbols(stub)
defined = {name for name, info in exports.items() if info[3] != "UND" and name}
require(defined == {"GLIBC_2.2.5", *(name + "@@GLIBC_2.2.5" for name in names)},
        "Unexpected stub exports")
dynamic = output("readelf", "-dW", str(stub))
require("(SONAME)" in dynamic and "[libc.so.6]" in dynamic, "Wrong stub SONAME")
require("(NEEDED)" not in dynamic, "Stub depends on a runtime library")
evidence = ["route\tlibc_sha256\tsymbol\ttype\tbinding\tvisibility"]
for route in ("prebuilt", "source"):
    real_path = work / route / "lib64/libc.so.6"
    real = symbols(real_path)
    checksum = hashlib.sha256(real_path.read_bytes()).hexdigest()
    binary = work / ("hello-stub-" + route)
    references = symbols(binary)
    for name in names:
        versioned = name + "@@GLIBC_2.2.5"
        for table in (real, exports):
            require(versioned in table, "Missing default version: " + versioned)
            binding = "WEAK" if name == "puts" else "GLOBAL"
            require(table[versioned][:3] == ["FUNC", binding, "DEFAULT"],
                    "Unexpected ELF properties: " + versioned + " " + str(table[versioned]))
        require(references.get(name + "@GLIBC_2.2.5") == ["FUNC", "GLOBAL", "DEFAULT", "UND"],
                "Wrong executable symbol requirement: " + name)
        evidence.append("\t".join([route, checksum, versioned, *real[versioned][:3]]))
    binary_dynamic = output("readelf", "-dW", str(binary))
    needed = [line for line in binary_dynamic.splitlines() if "(NEEDED)" in line]
    require(len(needed) == 1 and "[libc.so.6]" in needed[0], "Wrong runtime dependencies")
    require("(RPATH)" not in binary_dynamic and "(RUNPATH)" not in binary_dynamic,
            "Executable embeds a library search path")
    require("/lib64/ld-linux-x86-64.so.2" in output("readelf", "-lW", str(binary)),
            "Wrong runtime interpreter")
    link_map = (work / "logs" / ("stub-" + route + "-link.map")).read_text()
    require(str(stub) in link_map, "Generated libc was not linked")
    require("libc_nonshared.a(elf-init.oS)" in link_map, "Missing real nonshared startup code")
    for line in link_map.splitlines():
        if line.startswith("LOAD ") and ".so" in line:
            require(line == "LOAD " + str(stub), "Unexpected shared library input: " + line)
    loaded = (work / "logs" / ("stub-" + route + "-runtime-libraries.txt")).read_text()
    require(str(work / route / "lib64/libc.so.6") in loaded, "Wrong runtime libc")
    require("generated-stub" not in loaded, "Runtime loaded the stub")
(work / "logs/stub-real-elf-evidence.tsv").write_text("\n".join(evidence) + "\n")
print("Verified both routes: default versions, ELF properties, SONAME, real inputs, and runtime libc.")
