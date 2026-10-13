"""Independent evidence that Cargo selected stubs and ran only real target DSOs.

Usage: check-provider.py WORK LABEL RUNTIME_ROUTE DESCRIPTOR. LABEL names the
logs and Cargo directories; the descriptor names the provider's sysroot, stubs
and compiler runtime; RUNTIME_ROUTE is the work tree's build the program ran on.
"""
import hashlib
import json
import pathlib
import re
import sys
from experiment import RELEASE

work = pathlib.Path(sys.argv[1]).resolve()
label, runtime = sys.argv[2], sys.argv[3]
fields = dict(line.split("\t", 1) for line in pathlib.Path(sys.argv[4]).read_text().splitlines()[1:])
sysroot = pathlib.Path(fields["sysroot"]).resolve()
stubs = pathlib.Path(fields["stubs"]).resolve()
roots = [(work / runtime).resolve(), (work / "run-runtime").resolve()]
link_roots = [stubs, pathlib.Path(fields["compiler-runtime"]).resolve()]
link_map = (work / "logs" / ("provider-" + label + "-link.map")).read_text()
assert any(line.startswith("LOAD ") and pathlib.Path(line[5:]).resolve() ==
           (sysroot / "usr/lib64/crt1.o") for line in link_map.splitlines()), "Missing target CRT"
inputs = {}
for line in link_map.splitlines():
    if line.startswith("LOAD "):
        path = pathlib.Path(line[5:]).resolve()
        if ".so" in line:
            assert any(path.is_relative_to(root) for root in link_roots), "Host shared link input: " + line
        if path.is_file():
            inputs[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
assert any(pathlib.Path(path).is_relative_to(stubs) and path.endswith("libc.so.6") for path in inputs), "Stub libc not used"
loaded = (work / "logs" / ("provider-" + label + "-runtime-libraries.txt")).read_text()
assert str(work / runtime / "lib64/libc.so.6") in loaded, "Wrong runtime libc"
assert str(work / "run-runtime/libgcc_s.so.1") in loaded, "Wrong runtime unwinder"
assert "compiler-runtime" not in loaded, "The link-only libgcc_s stub was loaded"
for resolved in re.findall(r"=>\s+(/\S+)", loaded):
    assert any(pathlib.Path(resolved).resolve().is_relative_to(root) for root in roots), "Host runtime: " + resolved
artifact = work / ("cargo-" + label) / "x86_64-unknown-linux-gnu/debug/glibc-hello"
manifest = {"release": RELEASE, "artifact_sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(), "link_inputs_sha256": inputs}
(work / "logs" / ("provider-" + label + "-manifest.json")).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
print(label + ": Cargo stub selection and real runtime dependencies verified")
