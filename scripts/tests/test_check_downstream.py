"""Unit tests for the downstream Cargo metadata source checker."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


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
        self.budget_id = "path+file:///rs-budget#qubit-budget@0.9.0"
        self.metadata = {
            "packages": [
                self.package(self.root_id, "root", self.root_manifest),
                self.package(self.io_id, "qubit-io", self.io_manifest),
                self.package(self.budget_id, "qubit-budget", self.budget_manifest),
            ],
            "resolve": {
                "nodes": [
                    {"id": self.root_id, "deps": [{"name": "io-alias", "pkg": self.io_id}]},
                    {"id": self.io_id, "deps": [{"name": "budget", "pkg": self.budget_id}]},
                    {"id": self.budget_id, "deps": []},
                ]
            },
        }
        self.expected = {"qubit-io": self.io_manifest, "qubit-budget": self.budget_manifest}

    def tearDown(self) -> None:
        self.temp.cleanup()

    @staticmethod
    def package(package_id: str, name: str, manifest: Path, source: str | None = None) -> dict:
        return {
            "id": package_id,
            "name": name,
            "version": "1.0.0",
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

    def test_registry_budget_source_fails(self) -> None:
        self.metadata["packages"][2]["source"] = "registry+https://example.invalid"
        errors = checker.inspect_graph(self.metadata, self.root_manifest, self.expected, {"qubit-budget"})
        self.assertTrue(any("qubit-budget: registry/other source" in error for error in errors))

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
            for dependency in (config_root / "io/Cargo.toml", config_root / "budget/Cargo.toml"):
                dependency.parent.mkdir(parents=True, exist_ok=True)
                dependency.touch()
            config = scripts / "downstream.toml"
            config.write_text(
                '[project]\nrepository_root = "."\nio_manifest = "io/Cargo.toml"\n'
                'budget_manifest = "budget/Cargo.toml"\n'
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
            expected_manifests = (
                common / "rs-io/Cargo.toml",
                common / "rs-budget/Cargo.toml",
                common / "rs-json/Cargo.toml",
            )
            for manifest in expected_manifests:
                manifest.parent.mkdir(parents=True, exist_ok=True)
                manifest.touch()
            config = scripts / "downstream.toml"
            config.write_text(
                '[project]\nrepository_root = ".."\nio_manifest = "rs-io/Cargo.toml"\n'
                'budget_manifest = "rs-budget/Cargo.toml"\n'
                '[[manifest]]\npath = "rs-json/Cargo.toml"\n',
                encoding="utf-8",
            )
            root, expected, manifests = checker.load_config(config)
            self.assertEqual(root, common.resolve())
            self.assertEqual(expected["qubit-io"], expected_manifests[0].resolve())
            self.assertEqual(manifests[0][0], expected_manifests[2].resolve())


if __name__ == "__main__":
    unittest.main()
