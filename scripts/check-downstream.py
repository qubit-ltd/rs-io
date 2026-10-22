#!/usr/bin/env python3
"""Check local qubit-io and qubit-budget package sources in downstream graphs."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from collections import deque
from pathlib import Path
from typing import Any


def canonical(path: str | Path) -> Path:
    return Path(path).expanduser().resolve(strict=True)


def reachable_ids(metadata: dict[str, Any], root_id: str) -> set[str]:
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    if root_id not in nodes:
        raise ValueError(f"root package ID is absent from resolve.nodes: {root_id}")
    found: set[str] = set()
    queue = deque([root_id])
    while queue:
        package_id = queue.popleft()
        if package_id in found:
            continue
        found.add(package_id)
        for dependency in nodes[package_id].get("deps", []):
            target = dependency.get("pkg")
            if target in nodes and target not in found:
                queue.append(target)
    return found


def inspect_graph(
    metadata: dict[str, Any],
    root_manifest: Path,
    expected: dict[str, Path],
    required: set[str],
) -> list[str]:
    packages = {package["id"]: package for package in metadata["packages"]}
    roots = [
        package_id
        for package_id, package in packages.items()
        if Path(package["manifest_path"]).resolve() == root_manifest
    ]
    if len(roots) != 1:
        raise ValueError(
            f"expected exactly one package for {root_manifest}, found {len(roots)}"
        )
    reachable = reachable_ids(metadata, roots[0])
    errors: list[str] = []
    for name, expected_manifest in expected.items():
        found = [packages[package_id] for package_id in reachable if packages[package_id]["name"] == name]
        if name in required and not found:
            errors.append(f"{name}: required package is not reachable")
        if len(found) > 1:
            ids = ", ".join(package["id"] for package in found)
            errors.append(f"{name}: expected one reachable package ID, found {len(found)} ({ids})")
        for package in found:
            manifest_path = Path(package["manifest_path"]).resolve()
            if package.get("source") is not None:
                errors.append(f"{name}: registry/other source {package['source']} at {manifest_path}")
            if manifest_path != expected_manifest:
                errors.append(f"{name}: canonical manifest is {manifest_path}, expected {expected_manifest}")
    return errors


def load_config(config_path: Path) -> tuple[Path, dict[str, Path], list[tuple[Path, dict[str, Any]]]]:
    data = tomllib.loads(config_path.read_text(encoding="utf-8"))
    script_root = config_path.resolve().parent.parent
    git_file = script_root / ".git"
    if git_file.is_dir():
        canonical_project_root = git_file.parent
    elif git_file.is_file():
        gitdir_line = git_file.read_text(encoding="utf-8").strip()
        if not gitdir_line.startswith("gitdir:"):
            raise ValueError(f"invalid worktree .git file: {git_file}")
        gitdir = (script_root / gitdir_line.partition(":")[2].strip()).resolve()
        if gitdir.parent.name == "worktrees" and gitdir.parent.parent.name == ".git":
            canonical_project_root = gitdir.parent.parent.parent
        else:
            canonical_project_root = script_root
    else:
        # Unit tests may use a temporary tree; a real invocation is expected in Git.
        canonical_project_root = script_root
    project = data["project"]
    repository_root = canonical(canonical_project_root / project["repository_root"])
    expected = {
        "qubit-io": canonical(repository_root / project["io_manifest"]),
        "qubit-budget": canonical(repository_root / project["budget_manifest"]),
    }
    manifests = []
    for item in data["manifest"]:
        path = (repository_root / item["path"]).resolve()
        manifests.append((path, item))
    return repository_root, expected, manifests


def run_metadata(manifest: Path, all_features: bool) -> dict[str, Any]:
    command = ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(manifest)]
    if all_features:
        command.append("--all-features")
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"cargo metadata failed for {manifest}: {detail}")
    return json.loads(result.stdout)


def check_config(config_path: Path, *, metadata_runner=run_metadata) -> list[str]:
    _, expected, manifests = load_config(config_path)
    errors: list[str] = []
    for manifest, settings in manifests:
        if not manifest.is_file():
            errors.append(f"missing configured manifest: {manifest}")
            continue
        for all_features, mode in ((False, "default"), (True, "all-features")):
            required = set(settings.get("required_all_features" if all_features else "required_default", []))
            try:
                metadata = metadata_runner(manifest, all_features)
                graph_errors = inspect_graph(metadata, manifest, expected, required)
            except (RuntimeError, ValueError, KeyError, json.JSONDecodeError) as error:
                errors.append(f"{manifest} [{mode}]: {error}")
                continue
            packages = {package["id"]: package for package in metadata["packages"]}
            root_id = next(
                package_id for package_id, package in packages.items()
                if Path(package["manifest_path"]).resolve() == manifest
            )
            active = reachable_ids(metadata, root_id)
            for name in expected:
                records = [packages[package_id] for package_id in active if packages[package_id]["name"] == name]
                if records:
                    record = records[0]
                    print(
                        f"{manifest} [{mode}] {name} {record['version']} "
                        f"source={record.get('source') or 'path'} manifest={Path(record['manifest_path']).resolve()}"
                    )
            errors.extend(f"{manifest} [{mode}]: {error}" for error in graph_errors)
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metadata-only", action="store_true", help="check Cargo metadata without compiling")
    parser.add_argument("--config", type=Path, default=Path(__file__).with_name("downstream.toml"))
    args = parser.parse_args()
    if not args.metadata_only:
        parser.error("only --metadata-only is supported")
    config_path = args.config.resolve()
    try:
        errors = check_config(config_path)
    except (OSError, tomllib.TOMLDecodeError, KeyError, ValueError) as error:
        print(f"configuration error: {error}", file=sys.stderr)
        return 1
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("downstream metadata source check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
