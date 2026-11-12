"""Checks for release boundaries which must fail before publication."""

import io
from pathlib import Path
import tarfile
import tempfile
import unittest

from release import check_registry_package, check_tag, extract_package


class ReleaseTests(unittest.TestCase):
    def test_only_the_exact_version_tag_is_accepted(self):
        check_tag("v0.2.2-alpha.1", "0.2.2-alpha.1")
        for tag in ["main", "v0.2.1", "0.2.2-alpha.1", "v0.2.2"]:
            with self.subTest(tag=tag), self.assertRaises(SystemExit):
                check_tag(tag, "0.2.2-alpha.1")

    def test_git_and_path_dependencies_cannot_reach_publication(self):
        package = {
            "publish": ["crates-io"],
            "dependencies": [{
                "name": "compiler",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
            }],
        }
        check_registry_package(package)
        for source in [None, "git+https://example.com/compiler", "registry+https://example.com/index"]:
            package["dependencies"][0]["source"] = source
            with self.subTest(source=source), self.assertRaises(SystemExit):
                check_registry_package(package)
        package["dependencies"] = []
        package["publish"] = False
        with self.assertRaises(SystemExit):
            check_registry_package(package)

    def test_source_archive_is_confined_to_its_package_directory(self):
        for name, symlink in [
            ("example-1.0/src/main.rs", False),
            ("example-1.0/../escape", False),
            ("/escape", False),
            ("other-package/src/main.rs", False),
            ("example-1.0/link", True),
            ("example-1.0/.runs/README.md", False),
            ("example-1.0/.infigraph/graph.db", False),
            ("example-1.0/scripts/__pycache__/release.pyc", False),
        ]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                folder = Path(temporary)
                archive = folder / "example.crate"
                with tarfile.open(archive, "w:gz") as tar:
                    member = tarfile.TarInfo(name)
                    member.size = 2
                    if symlink:
                        member.type = tarfile.SYMTYPE
                        member.linkname = "../../escape"
                    tar.addfile(member, io.BytesIO(b"ok"))
                output = folder / "extracted"
                if name == "example-1.0/src/main.rs":
                    extracted = extract_package(archive, output, "example-1.0")
                    self.assertEqual((extracted / "src/main.rs").read_bytes(), b"ok")
                else:
                    with self.assertRaises(SystemExit):
                        extract_package(archive, output, "example-1.0")
                    self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
