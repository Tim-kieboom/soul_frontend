#!/usr/bin/env python3
"""Codegen exe-correctness test runner.

Builds `soul_tester` once up front, then for each `.soul` file under
`soul_tester/soul/src/codegen_tests/`:
  1. point soul_tester's config.json at it and run the already-built
     soul_tester exe directly (soul -> MIR -> LLVM IR, written to
     soul_tester/soul/output/codegen/module.ll)
  2. invoke clang (LLVM 16, matching mir_codegen's target) to compile+link
     module.ll into a native exe
  3. run the exe and compare its process exit code against the `// expect: N`
     comment on the file's first line

This is the M1 codegen correctness oracle: MIR/AST faults only ever proved
the compiler didn't crash, never that generated code computes the right
answer — this is the first stage where that's actually checked, by running
real produced machine code.

Runs the built exe directly rather than `cargo run` per test: profiling
showed `cargo run` cost ~2.2s per invocation (vs a ~0.24s baseline with no
source change) because rewriting config.json used to force a recompile —
`config.json` is now read at runtime (see `soul_tester::config::config_path`)
specifically so this script can build once and invoke the exe 20+ times
without paying that cost each time.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SOUL_TESTER_DIR = REPO_ROOT / "soul_tester"
CONFIG_PATH = SOUL_TESTER_DIR / "config.json"
TESTS_DIR = SOUL_TESTER_DIR / "soul" / "src" / "codegen_tests"
LL_PATH = SOUL_TESTER_DIR / "soul" / "output" / "codegen" / "module.ll"
SOUL_TESTER_EXE = REPO_ROOT / "target" / "debug" / "soul_tester.exe"
CLANG = Path(r"C:\llvm-16\bin\clang.exe")

EXPECT_RE = re.compile(r"//\s*expect:\s*(\d+)")
EXPECT_STDOUT_RE = re.compile(r"//\s*expect_stdout:\s*(.+)")


def _leading_comment_lines(soul_file: Path) -> list[str]:
    lines = []
    for line in soul_file.read_text(encoding="utf-8").splitlines():
        if not line.strip().startswith("//"):
            break
        lines.append(line)
    return lines


def read_expected(soul_file: Path) -> int:
    for line in _leading_comment_lines(soul_file):
        match = EXPECT_RE.search(line)
        if match:
            return int(match.group(1))
    raise ValueError(f"{soul_file}: missing a leading '// expect: N' comment")


def read_expected_stdout(soul_file: Path) -> list[str]:
    return [
        match.group(1).strip()
        for line in _leading_comment_lines(soul_file)
        if (match := EXPECT_STDOUT_RE.search(line))
    ]


def set_main_path(relative_path: str) -> None:
    config = json.loads(CONFIG_PATH.read_text(encoding="utf-8"))
    config["mainPath"] = relative_path
    CONFIG_PATH.write_text(json.dumps(config, indent=4) + "\n", encoding="utf-8")


def build_soul_tester() -> None:
    """Builds soul_tester once, up front — an explicit, hard-stop step
    rather than letting a compile failure surface 20+ times as a confusing
    per-test error once the loop starts invoking the (nonexistent/stale)
    exe directly."""
    result = subprocess.run(
        ["cargo", "build", "-p", "soul_tester"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("soul_tester failed to build:", file=sys.stderr)
        print(result.stdout, file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        raise SystemExit(1)


def run_soul_tester() -> None:
    result = subprocess.run(
        [str(SOUL_TESTER_EXE)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or "success" not in result.stdout:
        raise RuntimeError(
            f"soul_tester did not report success:\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    if "codegen skipped" in result.stderr:
        raise RuntimeError(f"codegen was skipped:\n{result.stderr}")


def build_and_run_exe(exe_path: Path) -> tuple[int, str]:
    compile_result = subprocess.run(
        [str(CLANG), str(LL_PATH), "-o", str(exe_path)],
        capture_output=True,
        text=True,
    )
    if compile_result.returncode != 0:
        raise RuntimeError(
            f"clang failed to build the exe:\n{compile_result.stdout}\n{compile_result.stderr}"
        )

    run_result = subprocess.run([str(exe_path)], capture_output=True, text=True)
    return run_result.returncode, run_result.stdout


def main() -> int:
    test_files = sorted(TESTS_DIR.glob("*.soul"))
    if not test_files:
        print(f"no test files found under {TESTS_DIR}", file=sys.stderr)
        return 1

    build_soul_tester()

    original_config = CONFIG_PATH.read_text(encoding="utf-8")
    failures = []
    try:
        for soul_file in test_files:
            name = soul_file.name
            expected = read_expected(soul_file)
            relative_path = f"codegen_tests/{name}"

            expected_stdout = read_expected_stdout(soul_file)

            try:
                set_main_path(relative_path)
                run_soul_tester()
                exe_path = soul_file.with_suffix(".exe")
                actual, stdout = build_and_run_exe(exe_path)
                exe_path.unlink(missing_ok=True)

                exit_ok = actual == expected
                missing_stdout = [s for s in expected_stdout if s not in stdout]

                if exit_ok and not missing_stdout:
                    detail = f"exit code {actual}"
                    if expected_stdout:
                        detail += ", stdout matched"
                    print(f"PASS  {name}: {detail}")
                else:
                    if not exit_ok:
                        print(f"FAIL  {name}: expected exit code {expected}, got {actual}")
                    for s in missing_stdout:
                        print(f"FAIL  {name}: expected stdout to contain {s!r}, got {stdout!r}")
                    failures.append(name)
            except Exception as exc:  # noqa: BLE001 - report and keep going
                print(f"ERROR {name}: {exc}")
                failures.append(name)
    finally:
        CONFIG_PATH.write_text(original_config, encoding="utf-8")

    print()
    if failures:
        print(f"{len(failures)}/{len(test_files)} test(s) failed: {', '.join(failures)}")
        return 1

    print(f"all {len(test_files)} test(s) passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
