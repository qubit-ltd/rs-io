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


def load_local_patches(
    repository_root: Path, config: dict[str, Any], candidate_manifest: Path
) -> dict[str, Path]:
    configured = config["local_patches"]
    if not isinstance(configured, dict) or not configured:
        raise ValueError("project.local_patches must be a non-empty table")
    required_names = {"qubit-io", "qubit-budget", "qubit-fs", "qubit-fs-testkit"}
    if set(configured) != required_names:
        missing = sorted(required_names - set(configured))
        unexpected = sorted(set(configured) - required_names)
        raise ValueError(f"project.local_patches keys mismatch: missing={missing}, unexpected={unexpected}")
    patches: dict[str, Path] = {}
    for name, relative_manifest in configured.items():
        manifest = candidate_manifest if name == "qubit-io" else repository_root / relative_manifest
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
    repository_root = canonical(canonical_project_root / project["repository_root"])
    candidate_manifest = canonical(script_root / "Cargo.toml")
    expected = load_local_patches(repository_root, project, candidate_manifest)
    # Keep the configured sibling manifest as an independent repository identity check.
    if canonical(repository_root / project["io_manifest"]) != canonical(repository_root / project["local_patches"]["qubit-io"]):
        raise ValueError("qubit-io local_patches entry must match io_manifest")
    if expected.get("qubit-io") != candidate_manifest:
        raise ValueError("qubit-io local patch must use the candidate checkout Cargo.toml")
    if "qubit-budget" in expected and expected["qubit-budget"] != canonical(repository_root / project["budget_manifest"]):
        raise ValueError("qubit-budget local patch must match budget_manifest")
    manifests = []
    for item in data["manifest"]:
        path = (repository_root / item["path"]).resolve()
        settings = dict(item)
        overrides = settings.get("patch_overrides", {})
        if not isinstance(overrides, dict):
            raise ValueError(f"patch_overrides must be a table for {path}")
        resolved_overrides: dict[str, Path] = {}
        for name, relative_manifest in overrides.items():
            if name not in expected:
                raise ValueError(f"unknown patch override {name} for {path}")
            if not isinstance(relative_manifest, str) or Path(relative_manifest).is_absolute():
                raise ValueError(f"patch override path for {name} must be relative to repository_root")
            override_manifest = canonical(repository_root / relative_manifest)
            try:
                package = tomllib.loads(override_manifest.read_text(encoding="utf-8"))["package"]
                package_name = package["name"]
            except (OSError, tomllib.TOMLDecodeError, KeyError, TypeError) as error:
                raise ValueError(f"invalid patch override for {name}: {override_manifest}: {error}") from error
            if package_name != name:
                raise ValueError(
                    f"patch override {name} points to package {package_name} at {override_manifest}"
                )
            resolved_overrides[name] = override_manifest
        settings["patch_overrides"] = resolved_overrides
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
        manifest_patches = {**expected, **settings.get("patch_overrides", {})}
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
                metadata = metadata_runner(manifest, all_features, manifest_patches, allow_lock_update)
                graph_errors = inspect_graph(metadata, manifest, manifest_patches, required)
            except (RuntimeError, ValueError, KeyError, json.JSONDecodeError) as error:
                errors.append(f"{manifest} [{mode}]: {error}")
                continue
            packages = {package["id"]: package for package in metadata["packages"]}
            root_id = next(
                package_id for package_id, package in packages.items()
                if Path(package["manifest_path"]).resolve() == manifest
            )
            active = reachable_ids(metadata, root_id)
            for name in manifest_patches:
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
