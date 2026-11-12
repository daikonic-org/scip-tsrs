"""Package a built scip-tsrs executable and the licenses of its dependencies."""

import argparse
import hashlib
import io
from pathlib import Path
import subprocess
import tarfile

from release import cargo_metadata, check_tag


def dependency_notices(metadata):
    root = metadata["resolve"]["root"]
    parts = []
    for package in sorted(metadata["packages"], key=lambda pkg: (pkg["name"], pkg["version"])):
        if package["id"] == root or (package.get("source") or "").startswith("git+https://github.com/oxc-project/tsrs"):
            continue
        manifest = Path(package["manifest_path"])
        files = {file for file in manifest.parent.iterdir() if file.is_file()
                 and file.name.upper().startswith(("LICENSE", "COPYING", "NOTICE", "UNLICENSE"))}
        if package.get("license_file"):
            files.add(manifest.parent / package["license_file"])
        parts.append(f"=== {package['name']} {package['version']} ({package['license'] or 'see notices'}) ===\n")
        if package.get("repository"):
            parts.append(f"Source: {package['repository']}\n")
        for file in sorted(files):
            parts.append(f"\n--- {file.name} ---\n{file.read_text(errors='replace')}\n")
        parts.append("\n")
    return "".join(parts).encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, default=Path("dist"))
    parser.add_argument("--tag", help="Require that the tag matches the Cargo package version")
    args = parser.parse_args()
    metadata, package = cargo_metadata(args.target, dependencies=True)
    version = package["version"]
    if args.tag:
        check_tag(args.tag, version)
    version_output = subprocess.check_output([str(args.binary.resolve()), "--version"], text=True).strip()
    if version_output != f"scip-tsrs {version}":
        raise SystemExit(f"Unexpected version output from {args.binary}: {version_output}")
    args.output.mkdir(parents=True, exist_ok=True)
    basename = f"scip-tsrs-{version}-{args.target}"
    archive = args.output / f"{basename}.tar.gz"
    with tarfile.open(archive, "w:gz") as tar:
        tar.add(args.binary.resolve(), arcname=f"{basename}/scip-tsrs")
        for file in ("README.md", "LICENSE", "NOTICE", "NOTICE-tsrs"):
            tar.add(file, arcname=f"{basename}/{file}")
        notices = dependency_notices(metadata)
        info = tarfile.TarInfo(f"{basename}/THIRD-PARTY-LICENSES.txt")
        info.size = len(notices)
        tar.addfile(info, io.BytesIO(notices))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + ".sha256").write_text(f"{digest}  {archive.name}\n")
    print(archive)


if __name__ == "__main__":
    main()
