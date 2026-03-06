#!/usr/bin/env python3

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path


PACKAGES = (
    ("hamr", Path("pkg/aur")),
    ("hamr-bin", Path("pkg/aur-bin")),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render AUR package metadata from templates.",
    )
    parser.add_argument(
        "--version", required=True, help="Release version, for example 1.0.18"
    )
    parser.add_argument(
        "--repo-root",
        default=Path(__file__).resolve().parent.parent,
        type=Path,
        help="Repository root containing pkg/",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="Output directory for rendered package folders",
    )
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="Render into pkg/aur and pkg/aur-bin inside the repository",
    )
    args = parser.parse_args()

    if args.output_dir is None and not args.in_place:
        parser.error("one of --output-dir or --in-place is required")

    if args.output_dir is not None and args.in_place:
        parser.error("--output-dir and --in-place are mutually exclusive")

    return args


def render_template(text: str, version: str) -> str:
    return text.replace("@VERSION@", version).replace("@TAG@", f"v{version}")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def render_package(
    repo_root: Path,
    package_name: str,
    source_dir: Path,
    version: str,
    destination: Path,
) -> None:
    template_dir = repo_root / source_dir
    destination.mkdir(parents=True, exist_ok=True)

    for template_name, output_name in (
        ("PKGBUILD.in", "PKGBUILD"),
        (".SRCINFO.in", ".SRCINFO"),
    ):
        rendered = render_template(
            (template_dir / template_name).read_text(encoding="utf-8"), version
        )
        write_text(destination / output_name, rendered)

    shutil.copy2(template_dir / "hamr.install", destination / "hamr.install")
    print(f"rendered {package_name} metadata to {destination}")


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()

    for package_name, source_dir in PACKAGES:
        if args.in_place:
            destination = repo_root / source_dir
        else:
            destination = args.output_dir.resolve() / package_name
        render_package(repo_root, package_name, source_dir, args.version, destination)

    return 0


if __name__ == "__main__":
    sys.exit(main())
