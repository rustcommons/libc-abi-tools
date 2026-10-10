"""Independent evidence that Cargo selected stubs and ran only real target DSOs."""
import hashlib
import json
import pathlib
import re
import sys
from experiment import RELEASE

work = pathlib.Path(sys.argv[1]).resolve()
route = sys.argv[2]
roots = [work / route, work / "compiler-runtime"]
link_roots = [work / ("full-libraries-" + route), work / "compiler-runtime"]
link_map = (work / "logs" / ("provider-" + route + "-link.map")).read_text()
assert str(work / route / "usr/lib64/crt1.o") in link_map, "Missing target CRT"
inputs = {}
for line in link_map.splitlines():
    if line.startswith("LOAD "):
        path = pathlib.Path(line[5:]).resolve()
        if ".so" in line:
            assert any(path.is_relative_to(root) for root in link_roots), "Host shared link input: " + line
        if path.is_file():
            inputs[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
assert any("full-libraries-" in path and "libc.so.6" in path for path in inputs), "Stub libc not used"
loaded = (work / "logs" / ("provider-" + route + "-runtime-libraries.txt")).read_text()
assert str(work / route / "lib64/libc.so.6") in loaded, "Wrong runtime libc"
assert str(work / "compiler-runtime/libgcc_s.so.1") in loaded, "Wrong runtime unwinder"
for resolved in re.findall(r"=>\s+(/\S+)", loaded):
    assert any(pathlib.Path(resolved).resolve().is_relative_to(root) for root in roots), "Host runtime: " + resolved
artifact = work / ("cargo-" + route) / "x86_64-unknown-linux-gnu/debug/glibc-hello"
manifest = {"release": RELEASE, "artifact_sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(), "link_inputs_sha256": inputs}
(work / "logs" / ("provider-" + route + "-manifest.json")).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
print(route + ": Cargo stub selection and real runtime dependencies verified")
