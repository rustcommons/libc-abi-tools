"""Audit library behavior executables and real loader resolution independently."""
import hashlib
import json
import pathlib
import re
import sys
from experiment import RELEASE, ROUTES

work = pathlib.Path(sys.argv[1]).resolve()
sonames = {"m": "libm.so.6", "dl": "libdl.so.2", "rt": "librt.so.1",
           "util": "libutil.so.1", "resolv": "libresolv.so.2"}
required = {"m": {"sin", "cos", "sqrt", "pow"}, "dl": {"dlopen", "dlsym", "dlvsym", "dlclose"},
            "rt": {"clock_gettime", "clock_getres", "timer_create", "timer_delete"},
            # These target headers map the public dn_expand spelling to __dn_expand.
            "util": {"openpty"}, "resolv": {"ns_initparse", "ns_parserr", "__dn_expand"}}
ceiling = tuple(map(int, RELEASE.split("."))) + (0,)
report = {"release": RELEASE, "scope": "selected real library behavior", "cases": []}


def require(condition, message):
    if not condition:
        raise SystemExit(message)


for route in ROUTES:
    real = (work / route).resolve()
    plugin = work / "behavior" / route / "plugin.so"
    plugin_text = (work / "logs" / ("behavior-" + route + "-plugin-elf.txt")).read_text()
    require("(NEEDED)" not in plugin_text and "fixture_value@@FIXTURE_1.0" in plugin_text,
            "Plugin fixture has unexpected dependencies or export")
    for library, soname in sonames.items():
        prefix = work / "logs" / ("behavior-" + route + "-" + library)
        text = pathlib.Path(str(prefix) + "-elf.txt").read_text()
        imported = {line.split()[7].split("@")[0] for line in text.splitlines()
                    if " UND " in line and len(line.split()) >= 8}
        require(required[library] <= imported, "Missing expected imports: " + library)
        require("[" + soname + "]" in text and "(RPATH)" not in text and "(RUNPATH)" not in text,
                "Wrong dependency or search path: " + library)
        for version in re.findall(r"GLIBC_([0-9]+(?:\.[0-9]+){1,2})", text):
            parts = tuple(map(int, version.split(".")))
            require(parts + (0,) * (3 - len(parts)) <= ceiling, "Too-new behavior import")
        listing = pathlib.Path(str(prefix) + "-libraries.txt").read_text()
        paths = re.findall(r"(?:=>\s+)?(/\S+)\s+\(", listing)
        require(paths, "Missing runtime dependency paths")
        for path in paths:
            require(pathlib.Path(path).resolve().is_relative_to(real), "Runtime escaped real root: " + path)
        require(any(pathlib.Path(p).name == soname for p in paths), "Test library not loaded: " + library)
        trace = pathlib.Path(str(prefix) + "-runtime-trace.txt").read_text()
        modules = sorted(set(re.findall(r"calling (?:init|fini):\s+(/\S+)", trace)))
        require(modules, "Missing actual runtime initialization trace")
        fixture_root = work / "behavior" / route
        executable = fixture_root / library
        for path in modules:
            resolved = pathlib.Path(path).resolve()
            require(resolved.is_relative_to(real) or resolved in (plugin.resolve(), executable.resolve()),
                    "Dynamically loaded module escaped real tree: " + path)
        if library == "dl":
            require(str(plugin) in modules, "Controlled plugin not seen in runtime trace")
        linkmap = pathlib.Path(str(prefix) + ".map").read_text()
        stub = work / ("full-libraries-" + route)
        require(str(stub / ("lib" + library + ".so")) in linkmap and str(stub / "libc.so") in linkmap,
                "Behavior program did not link against selected stubs")
        report["cases"].append({"route": route, "library": library,
                                "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
                                "plugin_sha256": hashlib.sha256(plugin.read_bytes()).hexdigest(),
                                "dynamic_module_paths": modules,
                                "dynamic_module_sha256": {p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
                                                          for p in modules},
                                "runtime_paths": paths, "imports": sorted(imported),
                                "runtime_sha256": {p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
                                                   for p in paths}})
(work / "logs/behavior-manifest.json").write_text(json.dumps(report, indent=2) + "\n")
print("Verified", len(report["cases"]), "library behavior cases")
