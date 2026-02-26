import os
import subprocess
import sys

def main(root_dir: str = "."):
    for dirpath, dirnames, filenames in os.walk(root_dir):
        if "Cargo.toml" in filenames:
            print(f"Running 'cargo fmt' in: {dirpath}")
            try:
                result = subprocess.run(
                    ["cargo", "fmt"],
                    cwd=dirpath,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                )
                print(result.stdout, end="")
                if result.returncode != 0:
                    print(
                        f"cargo fmt failed in {dirpath} with exit code {result.returncode}",
                        file=sys.stderr,
                    )
            except FileNotFoundError:
                print(
                    "Error: 'cargo' command not found. Ensure Rust is installed and in PATH.",
                    file=sys.stderr,
                )
                return 1
            except Exception as e:
                print(f"Unexpected error in {dirpath}: {e}", file=sys.stderr)
                return 1
    return 0

if __name__ == "__main__":
    # If a directory is given, use it; otherwise use current directory
    if len(sys.argv) == 2:
        sys.exit(main(sys.argv[1]))
    else:
        sys.exit(main("."))
