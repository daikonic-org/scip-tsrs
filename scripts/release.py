"""Validate release tags and test the actual crates.io source package."""

import argparse
import json
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parents[1]


def cargo_metadata(target=None, *, dependencies=False):
    command = ["cargo", "metadata", "--locked", "--format-version", "1"]
    if not dependencies:
        command.append("--no-deps")
    if target:
        command.extend(["--filter-platform", target])
    metadata = json.loads(subprocess.check_output(command, cwd=ROOT))
    members = metadata["workspace_default_members"]
    if len(members) != 1:
        raise SystemExit("Release tooling requires exactly one default workspace package")
    package = next(pkg for pkg in metadata["packages"] if pkg["id"] == members[0])
    return metadata, package


def check_tag(tag, version):
    if tag != f"v{version}":
        raise SystemExit(f"Release tag {tag} does not match package version {version}")


def check_registry_package(package):
    if package["publish"] != ["crates-io"]:
        raise SystemExit("The package must explicitly allow publishing to crates-io")
    for dependency in package["dependencies"]:
        if dependency.get("source") != "registry+https://github.com/rust-lang/crates.io-index":
            raise SystemExit(f"Dependency {dependency['name']} is not from crates.io")


def extract_package(archive, directory, name):
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        for member in members:
            path = PurePosixPath(member.name)
            if (
                path.is_absolute()
                or not path.parts
                or path.parts[0] != name
                or ".." in path.parts
                or any(part in {".runs", ".infigraph", "target", "__pycache__"} for part in path.parts[1:])
                or not (member.isfile() or member.isdir())
            ):
                raise SystemExit(f"Unexpected source-package member: {member.name}")
        for member in members:
            path = directory.joinpath(*PurePosixPath(member.name).parts)
            if member.isdir():
                path.mkdir(parents=True, exist_ok=True)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                with tar.extractfile(member) as source, path.open("wb") as destination:
                    shutil.copyfileobj(source, destination)
    return directory / name


def verify_package(target):
    metadata, package = cargo_metadata(target)
    check_registry_package(package)
    subprocess.run(["cargo", "package", "--locked", "--no-verify"], cwd=ROOT, check=True)
    name = f"{package['name']}-{package['version']}"
    target_directory = Path(metadata["target_directory"])
    archive = target_directory / "package" / f"{name}.crate"
    with tempfile.TemporaryDirectory(prefix="scip-tsrs-package-") as temporary:
        extracted = extract_package(archive, Path(temporary), name)
        subprocess.run(
            [
                "cargo", "test", "--locked", "--release", "--target", target,
                "--target-dir", str(target_directory),
                "--manifest-path", str(extracted / "Cargo.toml"),
            ],
            cwd=extracted,
            check=True,
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("version")
    tag = commands.add_parser("check-tag")
    tag.add_argument("tag")
    package = commands.add_parser("verify-package")
    package.add_argument("--target", required=True)
    args = parser.parse_args()
    if args.command == "verify-package":
        verify_package(args.target)
    else:
        _, package = cargo_metadata()
        if args.command == "check-tag":
            check_tag(args.tag, package["version"])
        print(package["version"])


if __name__ == "__main__":
    main()
