from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check-doc-examples.py"
SPEC = importlib.util.spec_from_file_location("check_doc_examples", SCRIPT)
assert SPEC and SPEC.loader
checker = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = checker
SPEC.loader.exec_module(checker)


PROFILES = {
    "core": {"features": [], "dependencies": {}},
    "digest": {"features": ["digest"], "dependencies": {"sha2": '"0.11"'}},
    "tokio": {
        "features": ["tokio"],
        "dependencies": {"tokio": '{ version = "1", features = ["rt"] }'},
    },
}


def rust(marker: str, body: str = "fn main() {}") -> str:
    return f"{marker}\n```rust\n{body}\n```\n"


class ExtractTests(unittest.TestCase):
    def test_extract_success_and_skip(self) -> None:
        doc = rust("<!-- example: first profile=core mode=run -->")
        doc += rust('<!-- example: sketch profile=core mode=skip reason="incomplete" -->', "fn f() {}")
        examples = checker.extract("README.md", doc, PROFILES)
        self.assertEqual([item.mode for item in examples], ["run", "skip"])

    def test_rejects_unmarked_and_duplicate_fences(self) -> None:
        with self.assertRaisesRegex(checker.ExampleError, "no example marker"):
            checker.extract("README.md", "```rust\nfn main() {}\n```", PROFILES)
        duplicate = rust("<!-- example: same profile=core mode=run -->") * 2
        with self.assertRaisesRegex(checker.ExampleError, "duplicate example id"):
            checker.extract("README.md", duplicate, PROFILES)

    def test_rejects_bad_profile_and_skip_reason(self) -> None:
        with self.assertRaisesRegex(checker.ExampleError, "unknown profile"):
            checker.extract("README.md", rust("<!-- example: x profile=all-features mode=run -->"), PROFILES)
        with self.assertRaisesRegex(checker.ExampleError, "needs a reason"):
            checker.extract("README.md", rust("<!-- example: x profile=core mode=skip -->"), PROFILES)

    def test_bilingual_ids_and_modes_must_match(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "a.md").write_text(rust("<!-- example: a profile=core mode=run -->"))
            (root / "b.md").write_text(rust("<!-- example: b profile=core mode=run -->"))
            (root / "c.md").write_text(rust("<!-- example: c profile=core mode=run -->"))
            (root / "d.md").write_text(rust("<!-- example: d profile=core mode=run -->"))
            config = {
                "documents": ["a.md", "b.md", "c.md", "d.md"],
                "profiles": PROFILES,
                "pairs": [["a.md", "b.md"]],
            }
            with self.assertRaisesRegex(checker.ExampleError, "bilingual inventory mismatch"):
                checker.validate_documents(config, root)

    def test_rejects_path_traversal_ids(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaisesRegex(checker.ExampleError, "invalid path id"):
                checker.safe_example_path("README.md", "../escape", Path(temp))

    def test_rejects_profile_feature_drift(self) -> None:
        text = """```toml
[dependencies]
qubit-io = { version = \"0.19\", features = [\"digest\"] }
sha2 = \"0.11\"
```
<!-- example: sample profile=core mode=run -->
```rust
fn main() {}
```
"""
        with self.assertRaisesRegex(checker.ExampleError, "do not match profile"):
            checker.validate_toml_profiles("README.md", text, PROFILES)

    def validate_dependency(self, declaration: str, profile: str = "digest", profiles: dict = PROFILES) -> None:
        text = (
            '```toml\n[dependencies]\n'
            'qubit-io = { version = "0.19", features = ["digest"] }\n'
            f"sha2 = {declaration}\n```\n"
            f"<!-- example: sample profile={profile} mode=run -->\n"
            "```rust\nfn main() {}\n```\n"
        )
        checker.validate_toml_profiles("README.md", text, profiles)

    def test_rejects_incompatible_zero_minor_version(self) -> None:
        with self.assertRaisesRegex(checker.ExampleError, "sha2 version"):
            self.validate_dependency('"0.10"')

    def test_accepts_matching_zero_minor_version(self) -> None:
        profiles = {**PROFILES, "digest": {"features": ["digest"], "dependencies": {"sha2": '"0.11"'}}}
        text = (
            '```toml\n[dependencies]\n'
            'qubit-io = { version = "0.19", features = ["digest"] }\n'
            'sha2 = " 0.11 "\n```\n'
            '<!-- example: sample profile=digest mode=run -->\n'
            '```rust\nfn main() {}\n```\n'
        )
        checker.validate_toml_profiles("README.md", text, profiles)

    def test_accepts_matching_string_and_table_versions(self) -> None:
        self.validate_dependency('{ version = "0.11", features = [] }')

    def test_rejects_stable_major_width_mismatch(self) -> None:
        profiles = {**PROFILES, "tokio": {"features": ["tokio"], "dependencies": {"tokio": '"1"'}}}
        text = (
            '```toml\n[dependencies]\n'
            'qubit-io = { version = "0.19", features = ["tokio"] }\n'
            'tokio = "1.54"\n```\n'
            '<!-- example: sample profile=tokio mode=run -->\n'
            '```rust\nfn main() {}\n```\n'
        )
        with self.assertRaisesRegex(checker.ExampleError, "tokio version"):
            checker.validate_toml_profiles("README.md", text, profiles)

    def test_accepts_same_complex_requirement(self) -> None:
        profiles = {**PROFILES, "digest": {"features": ["digest"], "dependencies": {"sha2": '"^0.11, < 0.12"'}}}
        self.validate_dependency('"^0.11, < 0.12"', profiles=profiles)

    def test_rejects_dependency_without_version(self) -> None:
        profiles = {**PROFILES, "digest": {"features": ["digest"], "dependencies": {"sha2": '"0.11"'}}}
        text = (
            '```toml\n[dependencies]\n'
            'qubit-io = { version = "0.19", features = ["digest"] }\n'
            'sha2 = { features = [] }\n```\n'
            '<!-- example: sample profile=digest mode=run -->\n'
            '```rust\nfn main() {}\n```\n'
        )
        with self.assertRaisesRegex(checker.ExampleError, r"README.md:1:.*sha2.*has no version.*profile.*requires one"):
            checker.validate_toml_profiles("README.md", text, profiles)


class CargoExecutionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.config = {"profiles": PROFILES}

    def tearDown(self) -> None:
        self.temp.cleanup()

    def example(self, mode: str = "run") -> checker.Example:
        return checker.Example("README.md", "sample", "core", mode, None, 10, "fn main() {}\n")

    def test_compile_failure_is_reported_with_location_and_stderr(self) -> None:
        def fail(args, **kwargs):
            self.assertIn("check", args)
            return subprocess.CompletedProcess(args, 101, "", "error[E0432]: unresolved import")

        with self.assertRaisesRegex(checker.ExampleError, r"(?s)README.md:10 \[sample\].*unresolved import"):
            checker.run_one(self.example("compile"), self.config, False, fail, self.root)

    def test_compile_fragment_gets_a_main_wrapper(self) -> None:
        example = checker.Example("README.md", "fragment", "core", "compile", None, 11, "fn helper() {}\n")

        def succeed(args, **kwargs):
            generated = (self.root / "target/doc-examples/readme/fragment/src/main.rs").read_text()
            self.assertTrue(generated.endswith("fn main() {}\n"))
            return subprocess.CompletedProcess(args, 0, "", "")

        checker.run_one(example, self.config, False, succeed, self.root)

    def test_runtime_assertion_failure_is_reported(self) -> None:
        def fail(args, **kwargs):
            self.assertIn("run", args)
            return subprocess.CompletedProcess(args, 101, "", "assertion failed")

        with self.assertRaisesRegex(checker.ExampleError, "assertion failed"):
            checker.run_one(self.example(), self.config, False, fail, self.root)

    def test_timeout_is_reported(self) -> None:
        def timeout(args, **kwargs):
            self.assertEqual(kwargs["timeout"], 180)
            raise subprocess.TimeoutExpired(args, 180, stderr="hung")

        with self.assertRaisesRegex(checker.ExampleError, "timed out after 180s"):
            checker.run_one(self.example(), self.config, False, timeout, self.root)

    def test_success_passes_cargo_arguments_as_list_and_offline(self) -> None:
        def succeed(args, **kwargs):
            self.assertIsInstance(args, list)
            self.assertIn("--offline", args)
            self.assertEqual(kwargs["timeout"], 180)
            self.assertIn("doc-examples-build", kwargs["env"]["CARGO_TARGET_DIR"])
            self.assertTrue((self.root / "target/doc-examples/readme/sample/src/main.rs").is_file())
            return subprocess.CompletedProcess(args, 0, "", "")

        checker.run_one(self.example(), self.config, True, succeed, self.root)


if __name__ == "__main__":
    unittest.main()
