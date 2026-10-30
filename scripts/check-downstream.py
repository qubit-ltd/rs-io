#!/usr/bin/env python3
"""Check local qubit-io and qubit-budget package sources in downstream graphs."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from collections import deque
from pathlib import Path
from typing import Any


CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"


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
    *,
    expected_versions: dict[str, str] | None = None,
    registry_packages: set[str] | None = None,
) -> list[str]:
    expected_versions = expected_versions or {}
    registry_packages = registry_packages or set()
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
    for name in required:
        if not any(packages[package_id]["name"] == name for package_id in reachable):
            errors.append(f"{name}: required package is not reachable")
    for name in set(expected) | registry_packages:
        expected_manifest = expected.get(name)
        found = [packages[package_id] for package_id in reachable if packages[package_id]["name"] == name]
        if len(found) > 1 and name not in registry_packages:
            ids = ", ".join(package["id"] for package in found)
            errors.append(f"{name}: expected one reachable package ID, found {len(found)} ({ids})")
        for package in found:
            manifest_path = Path(package["manifest_path"]).resolve()
            if name in registry_packages:
                source = package.get("source") or "path"
                if source != CRATES_IO_SOURCE:
                    errors.append(f"{name}: expected crates.io registry source, found {source}")
            else:
                if package.get("source") is not None:
                    errors.append(f"{name}: registry/other source {package['source']} at {manifest_path}")
                if expected_manifest is not None and manifest_path != expected_manifest:
                    errors.append(f"{name}: canonical manifest is {manifest_path}, expected {expected_manifest}")
            expected_version = expected_versions.get(name)
            if expected_version and name not in registry_packages:
                actual_version = package.get("version", "")
                matches = actual_version == expected_version or (
                    expected_version.count(".") == 1
                    and actual_version.startswith(expected_version + ".")
                )
                if not matches:
                    errors.append(f"{name}: expected version {expected_version}, found {actual_version}")
        if name in registry_packages and found and expected_versions.get(name):
            wanted = expected_versions[name]
            if not any(package.get("version") == wanted for package in found):
                versions = ", ".join(sorted({package.get("version", "") for package in found}))
                errors.append(f"{name}: expected crates.io version {wanted}, found {versions}")
    return errors


def load_local_patches(repository_root: Path, config: dict[str, Any]) -> dict[str, Path]:
    configured = config["local_patches"]
    if not isinstance(configured, dict) or not configured:
        raise ValueError("project.local_patches must be a non-empty table")
    required_names = {"qubit-fs", "qubit-fs-testkit"}
    if set(configured) != required_names:
        missing = sorted(required_names - set(configured))
        unexpected = sorted(set(configured) - required_names)
        raise ValueError(f"project.local_patches keys mismatch: missing={missing}, unexpected={unexpected}")
    patches: dict[str, Path] = {}
    for name, relative_manifest in configured.items():
        manifest = repository_root / relative_manifest
        manifest = canonical(manifest)
        try:
            package = tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]
            package_name = package["name"]
        except (OSError, tomllib.TOMLDecodeError, KeyError, TypeError) as error:
            raise ValueError(f"invalid local patch manifest for {name}: {manifest}: {error}") from error
        if package_name != name:
            raise ValueError(f"local patch {name} points to package {package_name} at {manifest}")
        patches[name] = manifest
    return patches


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
    registry_versions = project.get("registry_versions")
    if not isinstance(registry_versions, dict) or set(registry_versions) != {"qubit-budget"}:
        raise ValueError(
            f"{config_path}: project.registry_versions must contain exactly qubit-budget"
        )
    for name, version in registry_versions.items():
        if not isinstance(version, str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
            raise ValueError(
                f"{config_path}: project.registry_versions.{name} must be a three-part numeric version"
            )
    repository_root = canonical(canonical_project_root / project["repository_root"])
    expected = load_local_patches(repository_root, project)
    # Keep the configured sibling manifest as an independent repository identity check.
    io_manifest = canonical(repository_root / project["io_manifest"])
    try:
        io_package = tomllib.loads(io_manifest.read_text(encoding="utf-8"))["package"]["name"]
    except (OSError, tomllib.TOMLDecodeError, KeyError, TypeError) as error:
        raise ValueError(f"invalid io_manifest {io_manifest}: {error}") from error
    if io_package != "qubit-io":
        raise ValueError(f"io_manifest must identify qubit-io, found {io_package}")
    manifests = []
    for item in data["manifest"]:
        path = (repository_root / item["path"]).resolve()
        settings = dict(item)
        settings["io_manifest"] = io_manifest
        settings["registry_versions"] = dict(registry_versions)
        manifests.append((path, settings))
    return repository_root, expected, manifests


def metadata_command(
    manifest: Path,
    all_features: bool,
    patches: dict[str, Path],
    allow_lock_update: bool = False,
) -> list[str]:
    command = ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(manifest)]
    if not allow_lock_update:
        command.append("--locked")
    if all_features:
        command.append("--all-features")
    for name, package_manifest in sorted(patches.items()):
        value = json.dumps(str(package_manifest.parent))
        command.extend(["--config", f"patch.crates-io.{name}.path={value}"])
    return command


def locate_workspace(manifest: Path) -> Path:
    result = subprocess.run(
        ["cargo", "locate-project", "--workspace", "--message-format", "plain", "--manifest-path", str(manifest)],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"cargo locate-project failed for {manifest}: {detail}")
    return canonical(result.stdout.strip())


def check_patch_conflicts(
    manifest: Path,
    patches: dict[str, Path],
    workspace_locator=locate_workspace,
) -> list[str]:
    workspace_manifest = workspace_locator(manifest)
    try:
        data = tomllib.loads(workspace_manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        return [f"cannot inspect Cargo workspace manifest {workspace_manifest}: {error}"]
    configured = data.get("patch", {}).get("crates-io", {})
    errors: list[str] = []
    for name, expected_manifest in patches.items():
        if name not in configured:
            continue
        entry = configured[name]
        path_value = entry.get("path") if isinstance(entry, dict) else None
        actual_path = None
        if isinstance(path_value, str):
            actual_path = (workspace_manifest.parent / path_value / "Cargo.toml").resolve()
        if actual_path != expected_manifest:
            source = f"path {actual_path}" if actual_path else f"source {entry}"
            errors.append(
                f"Cargo workspace {workspace_manifest} has conflicting patch {name}: {source}; "
                f"expected path {expected_manifest}"
            )
    return errors


def run_metadata(
    manifest: Path,
    all_features: bool,
    patches: dict[str, Path],
    allow_lock_update: bool = False,
) -> dict[str, Any]:
    command = metadata_command(manifest, all_features, patches, allow_lock_update)
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"cargo metadata failed for {manifest}: {detail}")
    return json.loads(result.stdout)


def check_config(
    config_path: Path,
    *,
    allow_lock_update: bool = False,
    metadata_runner=run_metadata,
    workspace_locator=locate_workspace,
) -> list[str]:
    _, expected, manifests = load_config(config_path)
    errors: list[str] = []
    for manifest, settings in manifests:
        if not manifest.is_file():
            errors.append(f"missing configured manifest: {manifest}")
            continue
        manifest_patches = expected
        try:
            conflicts = check_patch_conflicts(manifest, manifest_patches, workspace_locator)
        except RuntimeError as error:
            errors.append(f"{manifest}: {error}")
            continue
        if conflicts:
            errors.extend(f"{manifest}: {error}" for error in conflicts)
            continue
        for all_features, mode in ((False, "default"), (True, "all-features")):
            required = set(settings.get("required_all_features" if all_features else "required_default", []))
            try:
                expected_sources = {"qubit-io": settings["io_manifest"]}
                graph_paths = {**manifest_patches, **expected_sources}
                metadata = metadata_runner(
                    manifest, all_features, manifest_patches, allow_lock_update
                )
                registry_versions = settings["registry_versions"]
                graph_errors = inspect_graph(
                    metadata, manifest, graph_paths, required,
                    expected_versions={"qubit-io": "0.19", **registry_versions},
                    registry_packages=set(registry_versions),
                )
            except (RuntimeError, ValueError, KeyError, json.JSONDecodeError) as error:
                errors.append(f"{manifest} [{mode}]: {error}")
                continue
            packages = {package["id"]: package for package in metadata["packages"]}
            root_id = next(
                package_id for package_id, package in packages.items()
                if Path(package["manifest_path"]).resolve() == manifest
            )
            active = reachable_ids(metadata, root_id)
            for name in set(manifest_patches) | {"qubit-io", "qubit-budget"}:
                records = [packages[package_id] for package_id in active if packages[package_id]["name"] == name]
                for record in records:
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
    parser.add_argument("--allow-lock-update", action="store_true", help="allow Cargo metadata to create or update downstream lockfiles")
    args = parser.parse_args()
    if not args.metadata_only:
        parser.error("only --metadata-only is supported")
    config_path = args.config.resolve()
    try:
        if args.allow_lock_update:
            print("WARNING: Cargo metadata may create or update downstream Cargo.lock files.", file=sys.stderr)
        errors = check_config(config_path, allow_lock_update=args.allow_lock_update)
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
