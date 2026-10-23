"""Unit tests for the downstream Cargo metadata source checker."""

from __future__ import annotations

import importlib.util
import io
import json
import shutil
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "check-downstream.py"
SPEC = importlib.util.spec_from_file_location("check_downstream", SCRIPT)
assert SPEC and SPEC.loader
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class MetadataGraphTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.root_manifest = self.root / "root" / "Cargo.toml"
        self.io_manifest = self.root / "rs-io" / "Cargo.toml"
        self.budget_manifest = self.root / "rs-budget" / "Cargo.toml"
        for path in (self.root_manifest, self.io_manifest, self.budget_manifest):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.touch()
        self.root_id = "path+file:///root#root@1.0.0"
        self.io_id = "path+file:///rs-io#qubit-io@0.19.0"
        self.budget_id = "registry+https://github.com/rust-lang/crates.io-index#qubit-budget@0.9.1"
        self.metadata = {
            "packages": [
                self.package(self.root_id, "root", self.root_manifest),
                self.package(self.io_id, "qubit-io", self.io_manifest),
                self.package(self.budget_id, "qubit-budget", self.budget_manifest, "registry+https://github.com/rust-lang/crates.io-index"),
            ],
            "resolve": {
                "nodes": [
                    {"id": self.root_id, "deps": [{"name": "io-alias", "pkg": self.io_id}]},
                    {"id": self.io_id, "deps": [{"name": "budget", "pkg": self.budget_id}]},
                    {"id": self.budget_id, "deps": []},
                ]
            },
        }
        self.expected = {"qubit-io": self.io_manifest}

    def tearDown(self) -> None:
        self.temp.cleanup()

    @staticmethod
    def package(package_id: str, name: str, manifest: Path, source: str | None = None) -> dict:
        return {
            "id": package_id,
            "name": name,
            "version": {"qubit-io": "0.19.0", "qubit-budget": "0.9.1"}.get(name, "1.0.0"),
            "source": source,
            "manifest_path": str(manifest),
        }

    def test_alias_edge_reaches_local_dependencies_and_cycle_terminates(self) -> None:
        self.metadata["resolve"]["nodes"][2]["deps"] = [
            {"name": "root-cycle", "pkg": self.root_id}
        ]
        self.assertEqual(
            checker.inspect_graph(self.metadata, self.root_manifest, self.expected, {"qubit-io"}),
            [],
        )

    def test_two_reachable_io_package_ids_fail(self) -> None:
        other_id = "registry+https://example.invalid#qubit-io@0.17.0"
        other_manifest = self.root / "registry" / "qubit-io" / "Cargo.toml"
        other_manifest.parent.mkdir(parents=True)
        other_manifest.touch()
        self.metadata["packages"].append(
            self.package(other_id, "qubit-io", other_manifest, "registry+https://example.invalid")
        )
        self.metadata["resolve"]["nodes"].append({"id": other_id, "deps": []})
        self.metadata["resolve"]["nodes"][0]["deps"].append({"name": "qubit-io", "pkg": other_id})
        errors = checker.inspect_graph(self.metadata, self.root_manifest, self.expected, set())
        self.assertTrue(any("expected one reachable package ID, found 2" in error for error in errors))

    def test_crates_io_budget_091_is_accepted_alongside_transitive_old_version(self) -> None:
        old_id = "registry+https://github.com/rust-lang/crates.io-index#qubit-budget@0.8.0"
        old_manifest = self.root / "old-budget/Cargo.toml"
        old_manifest.parent.mkdir()
        old_manifest.touch()
        self.metadata["packages"].append(self.package(
            old_id, "qubit-budget", old_manifest, "registry+https://github.com/rust-lang/crates.io-index"
        ))
        self.metadata["resolve"]["nodes"].append({"id": old_id, "deps": []})
        self.metadata["resolve"]["nodes"][0]["deps"].append({"name": "old", "pkg": old_id})
        self.assertEqual(checker.inspect_graph(
            self.metadata, self.root_manifest, self.expected, {"qubit-budget"},
            expected_versions={"qubit-io": "0.19", "qubit-budget": "0.9.1"},
            registry_packages={"qubit-budget"},
        ), [])

    def test_registry_budget_source_and_version_are_enforced(self) -> None:
        self.metadata["packages"][2]["source"] = "path+file:///somewhere"
        errors = checker.inspect_graph(
            self.metadata, self.root_manifest, self.expected, {"qubit-budget"},
            expected_versions={"qubit-io": "0.19", "qubit-budget": "0.9.1"},
            registry_packages={"qubit-budget"},
        )
        self.assertTrue(any("qubit-budget" in error and "registry" in error for error in errors))

    def test_registry_budget_rejects_a_non_crates_io_registry_id(self) -> None:
        self.metadata["packages"][2]["source"] = "registry+https://attacker.invalid/crates.io-index"
        errors = checker.inspect_graph(
            self.metadata, self.root_manifest, self.expected, {"qubit-budget"},
            expected_versions={"qubit-budget": "0.9.1"},
            registry_packages={"qubit-budget"},
        )
        self.assertTrue(any("expected crates.io registry source" in error for error in errors))

    def test_registry_budget_requires_exact_current_version(self) -> None:
        self.metadata["packages"][2]["version"] = "0.9.0"
        errors = checker.inspect_graph(
            self.metadata, self.root_manifest, self.expected, {"qubit-budget"},
            expected_versions={"qubit-budget": "0.9.1"},
            registry_packages={"qubit-budget"},
        )
        self.assertTrue(any("expected crates.io version 0.9.1" in error for error in errors))

    def test_io_version_constraint_is_enforced(self) -> None:
        self.metadata["packages"][1]["version"] = "0.18.9"
        errors = checker.inspect_graph(
            self.metadata, self.root_manifest, self.expected, {"qubit-io"},
            expected_versions={"qubit-io": "0.19"},
        )
        self.assertTrue(any("qubit-io" in error and "expected version 0.19" in error for error in errors))

    def test_wrong_canonical_manifest_fails(self) -> None:
        wrong = self.root / "wrong" / "Cargo.toml"
        wrong.parent.mkdir()
        wrong.touch()
        self.metadata["packages"][1]["manifest_path"] = str(wrong)
        errors = checker.inspect_graph(self.metadata, self.root_manifest, self.expected, {"qubit-io"})
        self.assertTrue(any("canonical manifest" in error for error in errors))

    def test_unactivated_optional_package_is_allowed(self) -> None:
        self.metadata["resolve"]["nodes"][0]["deps"] = []
        self.assertEqual(
            checker.inspect_graph(self.metadata, self.root_manifest, self.expected, set()),
            [],
        )

    def test_default_graph_may_omit_budget(self) -> None:
        self.metadata["resolve"]["nodes"][0]["deps"] = [
            {"name": "io-alias", "pkg": self.io_id}
        ]
        self.metadata["resolve"]["nodes"][1]["deps"] = []
        self.assertEqual(
            checker.inspect_graph(self.metadata, self.root_manifest, self.expected, {"qubit-io"}),
            [],
        )

    def test_all_features_policy_can_require_budget(self) -> None:
        self.metadata["resolve"]["nodes"][0]["deps"] = [
            {"name": "io-alias", "pkg": self.io_id}
        ]
        self.metadata["resolve"]["nodes"][1]["deps"] = []
        errors = checker.inspect_graph(
            self.metadata,
            self.root_manifest,
            self.expected,
            {"qubit-io", "qubit-budget"},
        )
        self.assertTrue(any("qubit-budget: required package is not reachable" in error for error in errors))

    def test_required_missing_package_fails(self) -> None:
        self.metadata["resolve"]["nodes"][0]["deps"] = []
        errors = checker.inspect_graph(self.metadata, self.root_manifest, self.expected, {"qubit-io"})
        self.assertTrue(any("qubit-io: required package is not reachable" in error for error in errors))

    def test_missing_configured_manifest_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            config_root = Path(temporary)
            scripts = config_root / "scripts"
            scripts.mkdir()
            for dependency in (config_root / "io/Cargo.toml", config_root / "budget/Cargo.toml",
                               config_root / "fs/Cargo.toml", config_root / "fs-testkit/Cargo.toml"):
                dependency.parent.mkdir(parents=True, exist_ok=True)
                package = {
                    "io": "qubit-io", "budget": "qubit-budget",
                    "fs": "qubit-fs", "fs-testkit": "qubit-fs-testkit",
                }[dependency.parent.name]
                dependency.write_text(f'[package]\nname = "{package}"\nversion = "1.0.0"\n', encoding="utf-8")
            (config_root / "Cargo.toml").write_text(
                '[package]\nname = "qubit-io"\nversion = "0.19.0"\n', encoding="utf-8")
            config = scripts / "downstream.toml"
            config.write_text(
                '[project]\nrepository_root = "."\nio_manifest = "io/Cargo.toml"\n'
                '[project.local_patches]\n'
                'qubit-fs = "fs/Cargo.toml"\nqubit-fs-testkit = "fs-testkit/Cargo.toml"\n'
                '[[manifest]]\npath = "missing/Cargo.toml"\n',
                encoding="utf-8",
            )
            errors = checker.check_config(
                config,
                metadata_runner=lambda *_args: self.fail("metadata must not run for a missing manifest"),
            )
            self.assertTrue(any("missing configured manifest" in error for error in errors))

    def test_worktree_config_uses_canonical_repository_siblings(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            common = Path(temporary) / "rust-common"
            main_repo = common / "rs-io"
            gitdir = main_repo / ".git/worktrees/worker"
            worktree = Path(temporary) / "worktree"
            scripts = worktree / "scripts"
            scripts.mkdir(parents=True)
            gitdir.mkdir(parents=True)
            (worktree / ".git").write_text(f"gitdir: {gitdir}\n", encoding="utf-8")
            expected_manifests = (common / "rs-io/Cargo.toml", common / "rs-budget/Cargo.toml", common / "rs-json/Cargo.toml")
            for manifest in expected_manifests:
                manifest.parent.mkdir(parents=True, exist_ok=True)
                name = {"rs-io": "qubit-io", "rs-budget": "qubit-budget", "rs-json": "qubit-json"}[manifest.parent.name]
                manifest.write_text(f'[package]\nname = "{name}"\nversion = "1.0.0"\n', encoding="utf-8")
            for directory, name in (("rs-fs", "qubit-fs"), ("rs-fs-testkit", "qubit-fs-testkit")):
                manifest = common / directory / "Cargo.toml"
                manifest.parent.mkdir(parents=True, exist_ok=True)
                manifest.write_text(f'[package]\nname = "{name}"\nversion = "1.0.0"\n', encoding="utf-8")
            (worktree / "Cargo.toml").write_text(
                '[package]\nname = "qubit-io"\nversion = "0.19.0"\n', encoding="utf-8")
            config = scripts / "downstream.toml"
            config.write_text(
                '[project]\nrepository_root = ".."\nio_manifest = "rs-io/Cargo.toml"\n'
                '[project.local_patches]\n'
                'qubit-fs = "rs-fs/Cargo.toml"\n'
                'qubit-fs-testkit = "rs-fs-testkit/Cargo.toml"\n'
                '[[manifest]]\npath = "rs-json/Cargo.toml"\n',
                encoding="utf-8",
            )
            root, expected, manifests = checker.load_config(config)
            self.assertEqual(root, common.resolve())
            self.assertNotIn("qubit-io", expected)
            self.assertEqual(manifests[0][0], expected_manifests[2].resolve())
            self.assertEqual(manifests[0][1]["io_manifest"], expected_manifests[0].resolve())


class DownstreamCommandTests(unittest.TestCase):
    def create_source_config(self, root: Path) -> tuple[Path, Path, Path]:
        scripts = root / "scripts"
        scripts.mkdir(parents=True)
        for relative, name in (("rs-io", "qubit-io"), ("rs-budget", "qubit-budget"),
                               ("rs-fs", "qubit-fs"), ("rs-fs-testkit", "qubit-fs-testkit")):
            manifest = root / relative / "Cargo.toml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text(f'[package]\nname = "{name}"\nversion = "1.0.0"\n', encoding="utf-8")
        candidate = root / "Cargo.toml"
        candidate.write_text('[package]\nname = "qubit-io"\nversion = "0.19.0"\n', encoding="utf-8")
        regular = root / "consumer/Cargo.toml"
        s3 = root / "s3-contract/Cargo.toml"
        for manifest, name in ((regular, "consumer"), (s3, "s3-contract")):
            manifest.parent.mkdir(parents=True)
            manifest.write_text(f'[package]\nname = "{name}"\nversion = "1.0.0"\n', encoding="utf-8")
            (manifest.parent / "src").mkdir()
            (manifest.parent / "src/lib.rs").write_text("", encoding="utf-8")
        config = scripts / "downstream.toml"
        config.write_text(
            '[project]\nrepository_root = "."\nio_manifest = "rs-io/Cargo.toml"\n'
            '[project.local_patches]\nqubit-fs = "rs-fs/Cargo.toml"\n'
            'qubit-fs-testkit = "rs-fs-testkit/Cargo.toml"\n'
            '[[manifest]]\npath = "consumer/Cargo.toml"\n'
            '[[manifest]]\npath = "s3-contract/Cargo.toml"\n'
            '', encoding="utf-8")
        return config, candidate.resolve(), (root / "rs-io/Cargo.toml").resolve()

    def test_metadata_command_quotes_all_local_patches_and_lock_policy(self) -> None:
        patches = {
            "qubit-fs": Path("/tmp/rust common/rs-fs/Cargo.toml"),
            "qubit-fs-testkit": Path("/tmp/rust common/rs-fs-testkit/Cargo.toml"),
        }
        command = checker.metadata_command(Path("/tmp/downstream/Cargo.toml"), False, patches, False)
        self.assertIn("--locked", command)
        self.assertNotIn("--all-features", command)
        config_args = [command[index + 1] for index, arg in enumerate(command[:-1]) if arg == "--config"]
        self.assertEqual(len(config_args), 2)
        self.assertEqual(
            config_args,
            [
                f"patch.crates-io.{name}.path={json.dumps(str(patches[name].parent))}"
                for name in sorted(patches)
            ],
        )
        update_command = checker.metadata_command(Path("/tmp/downstream/Cargo.toml"), True, patches, True)
        self.assertNotIn("--locked", update_command)
        self.assertIn("--all-features", update_command)

    def test_worktree_io_expectation_uses_canonical_sibling_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            common = root / "rust-common"
            main_repo = common / "rs-io"
            worktree = root / "candidate checkout"
            scripts = worktree / "scripts"
            scripts.mkdir(parents=True)
            gitdir = main_repo / ".git/worktrees/worker"
            gitdir.mkdir(parents=True)
            (worktree / ".git").write_text(f"gitdir: {gitdir}\n", encoding="utf-8")
            for base, name in ((worktree, "qubit-io"), (common / "rs-budget", "qubit-budget"),
                               (common / "rs-fs", "qubit-fs"),
                               (common / "rs-fs-testkit", "qubit-fs-testkit")):
                manifest = base / "Cargo.toml"
                manifest.parent.mkdir(parents=True, exist_ok=True)
                manifest.write_text(f'[package]\nname = "{name}"\nversion = "1.0.0"\n', encoding="utf-8")
            (main_repo / "Cargo.toml").write_text(
                '[package]\nname = "qubit-io"\nversion = "0.19.0"\n', encoding="utf-8")
            config = scripts / "downstream.toml"
            config.write_text(
                '[project]\nrepository_root = ".."\nio_manifest = "rs-io/Cargo.toml"\n'
                '[project.local_patches]\nqubit-fs = "rs-fs/Cargo.toml"\n'
                'qubit-fs-testkit = "rs-fs-testkit/Cargo.toml"\n'
                '[[manifest]]\npath = "rs-json/Cargo.toml"\n', encoding="utf-8")
            _, expected, manifests = checker.load_config(config)
            patches = checker.load_local_patches(common, checker.tomllib.loads(config.read_text())["project"])
            self.assertNotIn("qubit-io", expected)
            self.assertNotIn("qubit-io", patches)
            self.assertEqual(manifests[0][1]["io_manifest"], (main_repo / "Cargo.toml").resolve())
            self.assertNotEqual(manifests[0][1]["io_manifest"], (worktree / "Cargo.toml").resolve())

    def test_auxiliary_local_package_source_is_checked_when_reachable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            metadata = {
                "packages": [
                    {"id": "root", "name": "root", "manifest_path": str(root / "root/Cargo.toml"), "source": None},
                    {"id": "other", "name": "qubit-fs", "manifest_path": str(root / "registry/Cargo.toml"), "source": "registry+https://example.invalid"},
                ],
                "resolve": {"nodes": [{"id": "root", "deps": [{"pkg": "other"}]}, {"id": "other", "deps": []}]},
            }
            errors = checker.inspect_graph(metadata, (root / "root/Cargo.toml").resolve(),
                {"qubit-fs": (root / "local/Cargo.toml").resolve()}, set())
            self.assertTrue(any("qubit-fs" in error and "registry" in error for error in errors))

    def test_local_patch_configuration_rejects_missing_and_mismatched_packages(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = {"local_patches": {
                "qubit-fs": "rs-fs/Cargo.toml", "qubit-fs-testkit": "rs-fs-testkit/Cargo.toml",
            }}
            for relative, name in (("rs-budget", "qubit-budget"), ("rs-fs", "qubit-fs")):
                manifest = root / relative / "Cargo.toml"
                manifest.parent.mkdir(parents=True)
                manifest.write_text(f'[package]\nname = "{name}"\nversion = "1.0.0"\n', encoding="utf-8")
            candidate = root / "candidate/Cargo.toml"
            with self.assertRaises((FileNotFoundError, ValueError)):
                checker.load_local_patches(root, config)
            testkit = root / "rs-fs-testkit/Cargo.toml"
            testkit.parent.mkdir(parents=True)
            testkit.write_text('[package]\nname = "qubit-fs-testkit"\nversion = "1.0.0"\n', encoding="utf-8")
            wrong = root / "wrong/Cargo.toml"
            wrong.parent.mkdir()
            wrong.write_text('[package]\nname = "not-qubit-io"\nversion = "1.0.0"\n', encoding="utf-8")
            config["local_patches"]["qubit-fs"] = "wrong/Cargo.toml"
            candidate.parent.mkdir()
            candidate.write_text('[package]\nname = "qubit-io"\nversion = "1.0.0"\n', encoding="utf-8")
            with self.assertRaises(ValueError):
                checker.load_local_patches(root, config)

    def test_workspace_patch_conflict_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "member/Cargo.toml"
            manifest.parent.mkdir()
            manifest.write_text('[package]\nname = "consumer"\nversion = "1.0.0"\n', encoding="utf-8")
            (manifest.parent / "src").mkdir()
            (manifest.parent / "src/lib.rs").write_text("", encoding="utf-8")
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["member"]\n\n[patch.crates-io]\n'
                'qubit-io = { path = "../other-rs-io" }\n', encoding="utf-8")
            conflict = checker.check_patch_conflicts(manifest, {"qubit-io": root / "candidate/Cargo.toml"})
            self.assertTrue(any("qubit-io" in error and str(root / "Cargo.toml") in error for error in conflict))

    def test_matching_workspace_patch_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "member/Cargo.toml"
            manifest.parent.mkdir()
            manifest.write_text('[package]\nname = "consumer"\nversion = "1.0.0"\n', encoding="utf-8")
            (manifest.parent / "src").mkdir()
            (manifest.parent / "src/lib.rs").write_text("", encoding="utf-8")
            candidate = root / "candidate"
            candidate.mkdir()
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["member"]\n\n[patch.crates-io]\n'
                'qubit-io = { path = "candidate" }\n', encoding="utf-8")
            self.assertEqual(
                checker.check_patch_conflicts(manifest, {"qubit-io": candidate / "Cargo.toml"}), []
            )

    def test_check_config_passes_patch_and_lock_policy_to_fake_runner(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            scripts = root / "scripts"
            scripts.mkdir()
            for relative, name in (("rs-io", "qubit-io"), ("rs-budget", "qubit-budget"), ("rs-fs", "qubit-fs"),
                                   ("rs-fs-testkit", "qubit-fs-testkit")):
                manifest = root / relative / "Cargo.toml"
                manifest.parent.mkdir()
                manifest.write_text(f'[package]\nname = "{name}"\nversion = "1.0.0"\n', encoding="utf-8")
            candidate = root / "Cargo.toml"
            candidate.write_text('[package]\nname = "qubit-io"\nversion = "0.19.0"\n', encoding="utf-8")
            consumer = root / "consumer/Cargo.toml"
            consumer.parent.mkdir()
            consumer.write_text('[package]\nname = "consumer"\nversion = "1.0.0"\n', encoding="utf-8")
            (consumer.parent / "src").mkdir()
            (consumer.parent / "src/lib.rs").write_text("", encoding="utf-8")
            config = scripts / "downstream.toml"
            config.write_text(
                '[project]\nrepository_root = "."\nio_manifest = "rs-io/Cargo.toml"\n'
                '[project.local_patches]\nqubit-fs = "rs-fs/Cargo.toml"\n'
                'qubit-fs-testkit = "rs-fs-testkit/Cargo.toml"\n'
                '[[manifest]]\npath = "consumer/Cargo.toml"\n', encoding="utf-8")
            calls = []

            def fake_runner(manifest, all_features, patches, allow_lock_update):
                calls.append((manifest, all_features, patches, allow_lock_update))
                package_id = "consumer"
                return {
                    "packages": [{"id": package_id, "name": "consumer", "manifest_path": str(manifest), "source": None}],
                    "resolve": {"nodes": [{"id": package_id, "deps": []}]},
                }

            errors = checker.check_config(
                config, allow_lock_update=True, metadata_runner=fake_runner,
                workspace_locator=lambda _manifest: consumer,
            )
            self.assertEqual(errors, [])
            self.assertEqual([call[1] for call in calls], [False, True])
            self.assertTrue(all("qubit-io" not in call[2] for call in calls))
            self.assertTrue(all(call[3] for call in calls))

    def test_all_manifests_expect_the_canonical_sibling_io_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config, candidate, sibling = self.create_source_config(root)
            calls = []

            def fake_runner(manifest, all_features, patches, _allow_lock_update):
                io_manifest = sibling
                calls.append((manifest, all_features, patches, io_manifest))
                root_id = manifest.stem
                io_id = f"io-{root_id}"
                return {
                    "packages": [
                        {"id": root_id, "name": root_id, "version": "1.0.0", "manifest_path": str(manifest), "source": None},
                        {"id": io_id, "name": "qubit-io", "version": "0.19.0", "manifest_path": str(io_manifest), "source": None},
                    ],
                    "resolve": {"nodes": [
                        {"id": root_id, "deps": [{"pkg": io_id}]}, {"id": io_id, "deps": []},
                    ]},
                }

            errors = checker.check_config(
                config, metadata_runner=fake_runner,
                workspace_locator=lambda manifest: manifest,
            )
            self.assertEqual(errors, [])
            self.assertEqual([path for path, _, _, _ in calls], [root / "consumer/Cargo.toml"] * 2 + [root / "s3-contract/Cargo.toml"] * 2)
            self.assertTrue(all("qubit-io" not in patches for _, _, patches, _ in calls))
            self.assertEqual([io for _, _, _, io in calls], [sibling, sibling, sibling, sibling])

    def test_patch_conflict_checks_only_actual_local_patch_names(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config, _, _ = self.create_source_config(root)
            s3_manifest = root / "s3-contract/Cargo.toml"
            s3_manifest.write_text(
                '[package]\nname = "s3-contract"\nversion = "1.0.0"\n'
                '[patch.crates-io]\nqubit-fs = { path = ".." }\n', encoding="utf-8")

            def fake_runner(manifest, *_args):
                if manifest == s3_manifest:
                    self.fail("metadata must not run after patch conflict")
                return {
                    "packages": [{"id": "consumer", "name": "consumer", "version": "1.0.0",
                                  "manifest_path": str(manifest), "source": None}],
                    "resolve": {"nodes": [{"id": "consumer", "deps": []}]},
                }

            errors = checker.check_config(
                config,
                metadata_runner=fake_runner,
                workspace_locator=lambda manifest: manifest,
            )
            self.assertTrue(any("qubit-fs" in error for error in errors))

    def test_allow_lock_update_prints_warning_before_check(self) -> None:
        stderr = io.StringIO()
        with patch.object(checker.sys, "argv", [str(SCRIPT), "--metadata-only", "--allow-lock-update"]), \
             patch.object(checker, "check_config", return_value=[]), redirect_stderr(stderr):
            self.assertEqual(checker.main(), 0)
        self.assertIn("may create or update downstream Cargo.lock", stderr.getvalue())

    @unittest.skipUnless(shutil.which("cargo"), "Cargo is required for the offline integration fixture")
    def test_cargo_path_metadata_passes_checker_source_and_version_assertions(self) -> None:
        with tempfile.TemporaryDirectory(prefix="downstream fixture ") as temporary:
            root = Path(temporary)
            candidate = root / "candidate qubit-io"
            consumer = root / "consumer"
            candidate.mkdir()
            consumer.mkdir()
            candidate_manifest = candidate / "Cargo.toml"
            candidate_manifest.write_text(
                '[package]\nname = "qubit-io"\nversion = "0.19.0"\nedition = "2021"\n'
                '[features]\nextra = []\n', encoding="utf-8")
            (candidate / "src").mkdir()
            (candidate / "src/lib.rs").write_text("pub fn fixture() {}\n", encoding="utf-8")
            consumer_manifest = consumer / "Cargo.toml"
            consumer_manifest.write_text(
                '[package]\nname = "consumer"\nversion = "0.1.0"\nedition = "2021"\n'
                f'[dependencies]\nqubit-io = {{ version = "0.19", path = {json.dumps(str(candidate))}, features = [] }}\n', encoding="utf-8")
            (consumer / "src").mkdir()
            (consumer / "src/lib.rs").write_text("pub fn consumer() {}\n", encoding="utf-8")
            patches = {}
            lock_command = ["cargo", "generate-lockfile", "--manifest-path", str(consumer_manifest), "--offline"]
            lock_result = subprocess.run(lock_command, check=False, capture_output=True, text=True)
            self.assertEqual(lock_result.returncode, 0, lock_result.stderr)
            for all_features in (False, True):
                result = subprocess.run(checker.metadata_command(consumer_manifest, all_features, patches),
                                        check=True, capture_output=True, text=True)
                metadata = json.loads(result.stdout)
                package = next(item for item in metadata["packages"] if item["name"] == "qubit-io")
                self.assertIsNone(package["source"])
                self.assertEqual(Path(package["manifest_path"]).resolve(), candidate_manifest.resolve())
                self.assertEqual(
                    checker.inspect_graph(
                        metadata,
                        consumer_manifest.resolve(),
                        {"qubit-io": candidate_manifest.resolve()},
                        {"qubit-io"},
                        expected_versions={"qubit-io": "0.19"},
                    ),
                    [],
                )


if __name__ == "__main__":
    unittest.main()
