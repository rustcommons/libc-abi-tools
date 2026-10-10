"""Transfer qualified CI inputs; this is not a published/authenticated SDK format."""
import gzip
import hashlib
import io
import json
import os
import pathlib
import posixpath
import stat
import sys
import tarfile
from experiment import RELEASE, ROUTES

REPO = pathlib.Path(__file__).resolve().parents[2]
SONAMES = ("libc.so.6", "libm.so.6", "ld-linux-x86-64.so.2", "libresolv.so.2",
           "libpthread.so.0", "libdl.so.2", "librt.so.1", "libutil.so.1")
FORMAT = "libc-abi-ci-bundle-v1"
MAX_BYTES = 2 * 1024 * 1024 * 1024


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def safe(name):
    return bool(name) and not name.startswith("/") and "\\" not in name and all(
        part not in ("", ".", "..") for part in name.split("/"))


def link_target(name, target):
    require(target and not target.startswith("/") and "\\" not in target, "Absolute/invalid bundle symlink: " + name + " -> " + target)
    result = posixpath.normpath(posixpath.join(posixpath.dirname(name), target))
    require(safe(result), "Bundle symlink escapes extraction root")
    return result


def pack(work, archive):
    require(not archive.exists() and not pathlib.Path(str(archive) + ".sha256").exists(), "Archive already exists")
    entries = {}
    sources = {}

    def add(name, allowed=None, dereference=False):
        require(safe(name), "Unsafe bundle path")
        if name in entries:
            return
        path = work / name
        if allowed is None:
            allowed = path.resolve().parent
        require(path.resolve(strict=True).is_relative_to(allowed.resolve()), "Input symlink escaped declared subtree: " + name)
        if path.is_symlink() and not dereference:
            target = os.readlink(path)
            entries[name] = {"type": "symlink", "target": target, "mode": 0o777}
            add(link_target(name, target), allowed)
        elif path.is_dir():
            # Directories are implicit; store only regular files and symlinks.
            for child in sorted(path.iterdir()):
                add(name + "/" + child.name, allowed)
        else:
            require(path.is_file(), "Nonregular bundle input")
            entries[name] = {"type": "file", "sha256": digest(path), "size": path.stat().st_size,
                             "mode": stat.S_IMODE(path.stat().st_mode)}
            sources[name] = path

    for route in ROUTES:
        root = work / route
        add(route + "/usr/include", root)
        for filename in ("crt1.o", "crti.o", "crtn.o", "libc_nonshared.a", "libpthread_nonshared.a"):
            add(route + "/usr/lib64/" + filename, root)
        for soname in SONAMES:
            add(route + "/lib64/" + soname, root)
        # openpty/grantpt can perform NSS group lookup. These real modules are
        # runtime dependencies, not additional libraries in the ABI matrix.
        for pattern in ("libnss_*.so.[0-9]*", "libnsl.so.[0-9]*"):
            for module in sorted((root / "lib64").glob(pattern)):
                add(route + "/lib64/" + module.name, root)
        add("full-libraries-" + route, work / ("full-libraries-" + route))
        for filename in (work / "logs").glob("full-" + route + "-*-exports.txt"):
            add("logs/" + filename.name)
    add("compiler-runtime", work / "compiler-runtime")
    add("prebuilt/usr/lib64/libgcc_s.so.1", work / "prebuilt")
    add("elf-metadata", work / "elf-metadata")
    for filename in ("packages.txt", "builder-gcc.txt", "source-adjustment.patch", "abi-manifest.json"):
        add("logs/" + filename)
    add("link-matrix/manifest.json")
    add("glibc-" + RELEASE + ".tar.xz", dereference=True)
    for name, path in (("licenses/PROJECT-MIT", REPO / "LICENSE"),
                       ("licenses/ABI-DATA-COPYING", REPO / "data/glibc/COPYING.abilists"),
                       ("licenses/NOTICE.md", REPO / "NOTICE.md")):
        entries[name] = {"type": "file", "sha256": digest(path), "size": path.stat().st_size, "mode": 0o644}
        sources[name] = path
    require(len(entries) <= 20000 and sum(e.get("size", 0) for e in entries.values()) <= MAX_BYTES,
            "CI bundle exceeds bounds")
    abi = json.loads((work / "logs/abi-manifest.json").read_text())
    require(abi["release"] == RELEASE and abi["stage"] == "--abi-only" and
            len(abi["libraries"]) == 8 * len(ROUTES), "Missing preparation evidence")
    probes = json.loads((work / "link-matrix/manifest.json").read_text())
    require(probes["release"] == RELEASE and set(probes["routes"]) == set(ROUTES), "Missing exhaustive-link evidence")
    manifest = {"format": FORMAT, "release": RELEASE, "routes": list(ROUTES),
                "target": "x86_64-unknown-linux-gnu",
                "upstream_pin_sha256": digest(REPO / "data/glibc/UPSTREAM.toml"),
                "producer_commit": os.environ.get("GITHUB_SHA", "local-validation"), "entries": entries}
    payload = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode()
    with archive.open("xb") as output, gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode="w") as bundle:
            info = tarfile.TarInfo("bundle-manifest.json")
            info.size, info.mode = len(payload), 0o644
            bundle.addfile(info, io.BytesIO(payload))
            for name, entry in sorted(entries.items()):
                info = tarfile.TarInfo(name)
                info.mode = entry["mode"]
                if entry["type"] == "symlink":
                    info.type, info.linkname = tarfile.SYMTYPE, entry["target"]
                    bundle.addfile(info)
                else:
                    info.size = entry["size"]
                    with sources[name].open("rb") as stream:
                        bundle.addfile(info, stream)
    pathlib.Path(str(archive) + ".sha256").write_text(digest(archive) + "\n")
    print("Packed", RELEASE, len(entries), "entries into", archive)


def unpack(archive, work):
    require(not work.exists(), "Extraction directory must not exist")
    require(digest(archive) == pathlib.Path(str(archive) + ".sha256").read_text().strip(), "Archive checksum mismatch")
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        require(len(members) <= 20001 and sum(m.size for m in members) <= MAX_BYTES, "Archive exceeds bounds")
        names = [m.name for m in members]
        require(len(names) == len(set(names)) and all(safe(n) for n in names), "Unsafe/duplicate archive path")
        meta = bundle.getmember("bundle-manifest.json")
        require(meta.isfile() and meta.size <= 8 * 1024 * 1024, "Invalid bundle manifest")
        manifest = json.load(bundle.extractfile(meta))
        require(manifest["format"] == FORMAT and manifest["release"] == RELEASE and manifest["routes"] == list(ROUTES),
                "Wrong bundle format/release/routes")
        require(manifest.get("target") == "x86_64-unknown-linux-gnu", "Wrong bundle target")
        expected_commit = os.environ.get("GITHUB_SHA")
        require(not expected_commit or manifest.get("producer_commit") == expected_commit, "Bundle came from another commit")
        require(manifest["upstream_pin_sha256"] == digest(REPO / "data/glibc/UPSTREAM.toml"), "Bundle uses another ABI source")
        entries = manifest["entries"]
        require(set(names) == set(entries) | {"bundle-manifest.json"}, "Bundle inventory mismatch")
        for member in members:
            if member.name == "bundle-manifest.json":
                continue
            entry = entries[member.name]
            require(member.mode == entry["mode"] and member.mode & ~0o777 == 0, "Invalid bundle permissions")
            require(not any(parent in entries and entries[parent]["type"] == "symlink"
                            for parent in (str(p) for p in pathlib.PurePosixPath(member.name).parents)),
                    "Archive entry nested beneath symlink")
            if entry["type"] == "symlink":
                require(member.issym() and member.linkname == entry["target"] and
                        link_target(member.name, member.linkname) in entries, "Invalid bundle symlink")
                target = member.name
                visited = set()
                while entries[target]["type"] == "symlink":
                    require(target not in visited, "Bundle symlink cycle")
                    visited.add(target)
                    target = link_target(target, entries[target]["target"])
                    require(target in entries, "Missing symlink target")
                require(entries[target]["type"] == "file", "Symlink target is not a regular file")
            else:
                require(entry["type"] == "file" and member.isfile() and member.size == entry["size"], "Invalid file inventory")
                result = hashlib.sha256()
                stream = bundle.extractfile(member)
                for block in iter(lambda: stream.read(1024 * 1024), b""):
                    result.update(block)
                require(result.hexdigest() == entry["sha256"], "Bundle file checksum mismatch: " + member.name)
        # Validate every member before creating the extraction directory.
        work.mkdir(parents=True)
        bundle.extractall(work, filter="data")
        for name, entry in entries.items():
            path = work / name
            if entry["type"] == "symlink":
                require(path.resolve(strict=True).is_relative_to(work.resolve()), "Extracted symlink escaped root")
            else:
                require(digest(path) == entry["sha256"] and stat.S_IMODE(path.stat().st_mode) == entry["mode"],
                        "Extracted bytes/permissions changed")
    print("Verified and extracted", RELEASE, "into", work)


if __name__ == "__main__":
    require(len(sys.argv) == 4 and sys.argv[1] in ("pack", "unpack"), "Usage: ci-bundle.py pack WORK ARCHIVE | unpack ARCHIVE NEW_WORK")
    first, second = (pathlib.Path(p).absolute() for p in sys.argv[2:])
    (pack if sys.argv[1] == "pack" else unpack)(first, second)
