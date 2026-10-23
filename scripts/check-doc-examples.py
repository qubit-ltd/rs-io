#!/usr/bin/env python3
"""Extract and validate Rust examples embedded in the product Markdown."""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Callable, Sequence


ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "scripts" / "doc-examples.toml"
MARKER = re.compile(
    r"^<!-- example: ([a-z0-9][a-z0-9_-]*) profile=([a-z0-9_-]+) "
    r"mode=(run|compile|skip)(?: reason=\"([^\"]+)\")? -->$"
)
ID = re.compile(r"^[a-z0-9][a-z0-9_-]*$")


@dataclasses.dataclass(frozen=True)
class Example:
    document: str
    id: str
    profile: str
    mode: str
    reason: str | None
    line: int
    code: str


class ExampleError(Exception):
    pass


def load_config(path: Path = MANIFEST) -> dict:
    try:
        config = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ExampleError(f"{path}: cannot read configuration: {error}") from error
    if config.get("version") != 1:
        raise ExampleError(f"{path}: expected version = 1")
    return config


def extract(document: str, text: str, profiles: dict) -> list[Example]:
    lines = text.splitlines()
    found: list[Example] = []
    seen: set[str] = set()
    pending: tuple[re.Match[str], int] | None = None
    in_fence = False
    code: list[str] = []
    fence_line = 0
    rust_fence = False
    for number, line in enumerate(lines, 1):
        if not in_fence:
            marker = MARKER.fullmatch(line)
            if line.startswith("<!-- example:"):
                if marker is None:
                    raise ExampleError(f"{document}:{number}: malformed example marker")
                if pending is not None:
                    raise ExampleError(f"{document}:{number}: previous example marker is not followed by a Rust fence")
                pending = (marker, number)
            if line.startswith("```"):
                rust_fence = line.strip() in ("```rust", "```rs")
                in_fence = True
                fence_line = number
                code = []
                if rust_fence and pending is None:
                    raise ExampleError(f"{document}:{number}: Rust fence has no example marker")
                if not rust_fence and pending is not None:
                    raise ExampleError(f"{document}:{pending[1]}: marker is not followed by a Rust fence")
            continue
        if line.startswith("```"):
            if rust_fence:
                assert pending is not None
                marker, marker_line = pending
                example_id, profile, mode, reason = marker.groups()
                if example_id in seen:
                    raise ExampleError(f"{document}:{marker_line}: duplicate example id {example_id!r}")
                if profile not in profiles:
                    raise ExampleError(f"{document}:{marker_line}: unknown profile {profile!r}")
                if mode == "skip" and not reason:
                    raise ExampleError(f"{document}:{marker_line}: skipped example needs a reason")
                if mode != "skip" and reason:
                    raise ExampleError(f"{document}:{marker_line}: reason is only valid for mode=skip")
                seen.add(example_id)
                found.append(Example(document, example_id, profile, mode, reason,
                                     fence_line, "\n".join(code) + "\n"))
                pending = None
            in_fence = False
            continue
        code.append(line)
    if in_fence:
        raise ExampleError(f"{document}:{fence_line}: unclosed Markdown fence")
    if pending is not None:
        raise ExampleError(f"{document}:{pending[1]}: orphan example marker")
    return found


def validate_documents(config: dict, root: Path = ROOT) -> list[Example]:
    profiles = config.get("profiles", {})
    documents = config.get("documents", [])
    if len(documents) != 4 or len(set(documents)) != 4:
        raise ExampleError("configuration must list four unique product documents")
    all_examples: list[Example] = []
    by_document: dict[str, list[Example]] = {}
    for relative in documents:
        candidate = (root / relative).resolve()
        if not candidate.is_relative_to(root.resolve()):
            raise ExampleError(f"{relative}: document path escapes repository root")
        try:
            content = candidate.read_text(encoding="utf-8")
        except OSError as error:
            raise ExampleError(f"{relative}: cannot read document: {error}") from error
        examples = extract(relative, content, profiles)
        by_document[relative] = examples
        all_examples.extend(examples)
        validate_toml_profiles(relative, content, config["profiles"])
    for left, right in config.get("pairs", []):
        lmap = {item.id: (item.profile, item.mode) for item in by_document[left]}
        rmap = {item.id: (item.profile, item.mode) for item in by_document[right]}
        if lmap != rmap:
            left_only = sorted(lmap.keys() - rmap.keys())
            right_only = sorted(rmap.keys() - lmap.keys())
            changed = sorted(key for key in lmap.keys() & rmap.keys() if lmap[key] != rmap[key])
            raise ExampleError(
                f"{left} / {right}: bilingual inventory mismatch; "
                f"left-only={left_only}, right-only={right_only}, changed={changed}"
            )
    return all_examples


def validate_toml_profiles(document: str, text: str, profiles: dict) -> None:
    """Tie dependency snippets to profiles using exact, whitespace-trimmed versions.

    This intentionally requires matching declarations even when Cargo would treat
    different requirement strings as compatible. Update the profile whenever a
    published snippet changes its dependency requirement.
    """
    rust_markers = list(re.finditer(
        r"(?m)^<!-- example: ([a-z0-9][a-z0-9_-]*) profile=([a-z0-9_-]+) mode=",
        text,
    ))
    for block in re.finditer(r"(?ms)^```toml\s*\n(.*?)^```\s*$", text):
        following = next((item for item in rust_markers if item.start() > block.end()), None)
        if following is None:
            continue
        profile_name = following.group(2)
        try:
            manifest = tomllib.loads(block.group(1))
        except tomllib.TOMLDecodeError as error:
            line = text.count("\n", 0, block.start()) + 1
            raise ExampleError(f"{document}:{line}: invalid Cargo TOML snippet: {error}") from error
        dependencies = manifest.get("dependencies", {})
        qubit = dependencies.get("qubit-io", {})
        declared_features = qubit.get("features", []) if isinstance(qubit, dict) else []
        profile = profiles[profile_name]
        if sorted(declared_features) != sorted(profile.get("features", [])):
            line = text.count("\n", 0, block.start()) + 1
            raise ExampleError(
                f"{document}:{line}: qubit-io features {declared_features!r} "
                f"do not match profile {profile_name!r}"
            )
        external = set(dependencies) - {"qubit-io"}
        expected = set(profile.get("dependencies", {}))
        if external != expected:
            line = text.count("\n", 0, block.start()) + 1
            raise ExampleError(
                f"{document}:{line}: external dependencies {sorted(external)} "
                f"do not match profile {profile_name!r} ({sorted(expected)})"
            )
        for name in expected:
            line = text.count("\n", 0, block.start()) + 1
            source_spec = dependencies[name]
            configured = tomllib.loads(f"value = {profile['dependencies'][name]}")["value"]
            observed = source_spec.get("version") if isinstance(source_spec, dict) else source_spec
            expected_version = configured.get("version") if isinstance(configured, dict) else configured
            if not isinstance(observed, str) or not isinstance(expected_version, str):
                raise ExampleError(
                    f"{document}:{line}: {name} declaration {source_spec!r} has no version; "
                    f"profile {profile_name!r} declaration {configured!r} requires one"
                )
            if observed.strip() != expected_version.strip():
                raise ExampleError(
                    f"{document}:{line}: {name} version {observed!r} does not match "
                    f"profile {profile_name!r} ({expected_version!r})"
                )
            if isinstance(configured, dict):
                expected_features = set(configured.get("features", []))
                observed_features = set(source_spec.get("features", [])) if isinstance(source_spec, dict) else set()
                if expected_features != observed_features:
                    line = text.count("\n", 0, block.start()) + 1
                    raise ExampleError(
                        f"{document}:{line}: {name} features {sorted(observed_features)} "
                        f"do not match profile {profile_name!r} ({sorted(expected_features)})"
                    )


def validate_profile_dependencies(config: dict) -> None:
    """Check every published dependency snippet against its declared profile."""
    # Profiles encode the only external requirements allowed for generated projects.
    for name, profile in config["profiles"].items():
        features = profile.get("features", [])
        if not isinstance(features, list) or any(not isinstance(item, str) for item in features):
            raise ExampleError(f"profile {name}: features must be a string list")
        if "all-features" in features:
            raise ExampleError(f"profile {name}: all-features is not an example profile")


def safe_example_path(document: str, example_id: str, base: Path) -> Path:
    if not ID.fullmatch(example_id):
        raise ExampleError(f"invalid path id {example_id!r}")
    document_path = Path(document)
    if document_path.is_absolute() or ".." in document_path.parts:
        raise ExampleError(f"invalid document path {document!r}")
    doc_name = re.sub(r"[^a-z0-9_-]", "-", document_path.stem.lower())
    destination = (base / doc_name / example_id).resolve()
    if not destination.is_relative_to(base.resolve()):
        raise ExampleError(f"example path escapes output directory: {destination}")
    return destination


def command_for(project: Path, example: Example,
                cargo: str = "cargo", offline: bool = False) -> list[str]:
    args = [cargo, "run" if example.mode == "run" else "check", "--manifest-path",
            str(project / "Cargo.toml"), "--quiet"]
    if offline:
        args.append("--offline")
    return args


def run_one(example: Example, config: dict, offline: bool,
            runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
            root: Path = ROOT, output_base: Path | None = None) -> None:
    if example.mode == "skip":
        return
    output_base = output_base or root / "target" / "doc-examples"
    doc_dir = safe_example_path(example.document, example.id, output_base)
    doc_dir.mkdir(parents=True, exist_ok=True)
    project = doc_dir
    features = config["profiles"][example.profile].get("features", [])
    dependencies = [f'qubit-io = {{ path = {json.dumps(str(root.resolve()))}, features = {json.dumps(features)} }}']
    external = config["profiles"][example.profile].get("dependencies", {})
    dependencies.extend(f'{name} = {spec}' for name, spec in external.items())
    manifest = "\n".join([
        "[package]", f'name = "doc-example-{example.id}"', 'version = "0.0.0"',
        'edition = "2024"', "", "[dependencies]", *dependencies, "", "[workspace]", "",
    ])
    (project / "Cargo.toml").write_text(manifest, encoding="utf-8")
    (project / "src").mkdir(exist_ok=True)
    source = example.code
    if example.mode == "compile" and not re.search(r"\bfn\s+main\s*\(", source):
        source += "\nfn main() {}\n"
    (project / "src" / "main.rs").write_text(source, encoding="utf-8")
    args = command_for(project, example, offline=offline)
    cargo_target = root / "target" / "doc-examples-build"
    cargo_target.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(cargo_target)
    try:
        result = runner(args, cwd=root, text=True, capture_output=True,
                        timeout=180, check=False, env=environment)
    except subprocess.TimeoutExpired as error:
        stderr = error.stderr.decode() if isinstance(error.stderr, bytes) else error.stderr or ""
        raise ExampleError(f"{example.document}:{example.line} [{example.id}] timed out after 180s\n{stderr}") from error
    except OSError as error:
        raise ExampleError(f"{example.document}:{example.line} [{example.id}] could not start Cargo: {error}") from error
    if result.returncode:
        raise ExampleError(
            f"{example.document}:{example.line} [{example.id}] {example.mode} failed "
            f"(exit {result.returncode})\n{result.stderr}"
        )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--inventory", action="store_true")
    group.add_argument("--check", action="store_true")
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args(argv)
    try:
        config = load_config()
        validate_profile_dependencies(config)
        examples = validate_documents(config)
        counts = {mode: sum(item.mode == mode for item in examples) for mode in ("run", "compile", "skip")}
        print(f"inventory: {len(examples)} Rust fences; " + ", ".join(f"{key}={value}" for key, value in counts.items()))
        if args.check:
            for example in examples:
                run_one(example, config, args.offline)
                if example.mode != "skip":
                    print(f"passed: {example.document}:{example.line} [{example.id}] ({example.mode})")
        return 0
    except ExampleError as error:
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
