"""Exercise transfer integrity and extraction boundaries with original tiny fixtures."""
import hashlib
import importlib.util
import io
import json
import pathlib
import stat
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("ci_bundle", pathlib.Path(__file__).with_name("ci-bundle.py"))
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class TransferTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="libc-abi-bundle-test-")
        self.root = pathlib.Path(self.temporary.name)
        self.archive = self.root / "provider.tar.gz"
        self.work = self.root / "restored"
        self.payload = b"original test fixture\n"
        self.entries = {
            "source/lib64/fixture.so.1": {"type": "file", "sha256": hashlib.sha256(self.payload).hexdigest(),
                                           "size": len(self.payload), "mode": 0o755},
            "source/lib64/fixture.so": {"type": "symlink", "target": "fixture.so.1", "mode": 0o777}}

    def tearDown(self):
        self.temporary.cleanup()

    def write(self, mutation=None, extra=None):
        manifest = {"format": bundle.FORMAT, "release": bundle.RELEASE, "routes": list(bundle.ROUTES),
                    "target": "x86_64-unknown-linux-gnu",
                    "producer_commit": bundle.os.environ.get("GITHUB_SHA", "local-validation"),
                    "upstream_pin_sha256": bundle.digest(bundle.REPO / "data/glibc/UPSTREAM.toml"),
                    "entries": self.entries}
        if mutation:
            mutation(manifest)
        with tarfile.open(self.archive, "w:gz") as output:
            payload = json.dumps(manifest).encode()
            member = tarfile.TarInfo("bundle-manifest.json")
            member.size = len(payload)
            output.addfile(member, io.BytesIO(payload))
            for name, entry in self.entries.items():
                member = tarfile.TarInfo(name)
                member.mode = entry["mode"]
                if entry["type"] == "symlink":
                    member.type, member.linkname = tarfile.SYMTYPE, entry["target"]
                    output.addfile(member)
                else:
                    member.size = len(self.payload)
                    output.addfile(member, io.BytesIO(self.payload))
            if extra:
                output.addfile(tarfile.TarInfo(extra))
        pathlib.Path(str(self.archive) + ".sha256").write_text(bundle.digest(self.archive) + "\n")

    def rejected(self, message):
        with self.assertRaisesRegex(SystemExit, message):
            bundle.unpack(self.archive, self.work)
        self.assertFalse(self.work.exists(), "Rejected bundle created an extraction directory")

    def test_round_trip_preserves_bytes_permissions_and_relative_alias(self):
        self.write()
        bundle.unpack(self.archive, self.work)
        file = self.work / "source/lib64/fixture.so.1"
        alias = self.work / "source/lib64/fixture.so"
        self.assertEqual(file.read_bytes(), self.payload)
        self.assertEqual(stat.S_IMODE(file.stat().st_mode), 0o755)
        self.assertTrue(alias.is_symlink())
        self.assertEqual(alias.resolve(), file)
        with self.assertRaisesRegex(SystemExit, "must not exist"):
            bundle.unpack(self.archive, self.work)

    def test_archive_checksum_rejects_changed_bytes(self):
        self.write()
        with self.archive.open("ab") as stream:
            stream.write(b"changed")
        self.rejected("Archive checksum mismatch")

    def test_inventory_hash_rejects_changed_file(self):
        self.entries["source/lib64/fixture.so.1"]["sha256"] = "0" * 64
        self.write()
        self.rejected("file checksum mismatch")

    def test_wrong_release_and_pin_fail_before_extraction(self):
        for field, value, message in (("release", "2.99", "Wrong bundle"),
                                      ("upstream_pin_sha256", "0" * 64, "another ABI source")):
            self.write(lambda m: m.update({field: value}))
            self.rejected(message)

    def test_path_escape_and_unlisted_members_are_rejected(self):
        self.write(extra="../outside")
        self.rejected("Unsafe/duplicate archive path")
        self.assertFalse((self.root / "outside").exists())
        self.write(extra="unlisted")
        self.rejected("inventory mismatch")

    def test_symlink_escape_is_rejected(self):
        for target in ("/etc/passwd", "../../../outside"):
            self.entries["source/lib64/fixture.so"]["target"] = target
            self.write()
            self.rejected("[Bb]undle symlink")

    def test_symlink_cycle_and_duplicate_member_are_rejected(self):
        self.entries["source/lib64/fixture.so"]["target"] = "fixture.so"
        self.write()
        self.rejected("symlink cycle")
        self.entries["source/lib64/fixture.so"]["target"] = "fixture.so.1"
        self.write(extra="source/lib64/fixture.so.1")
        self.rejected("Unsafe/duplicate archive path")

    def test_privileged_permissions_are_rejected(self):
        self.entries["source/lib64/fixture.so.1"]["mode"] = 0o4755
        self.write()
        self.rejected("permissions")


if __name__ == "__main__":
    unittest.main()
