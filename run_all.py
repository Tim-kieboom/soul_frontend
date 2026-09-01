import os
import subprocess
import sys

def run_all(command: str, root_dir: str = "."):
    for dirpath, dirnames, filenames in os.walk(root_dir):
        if "Cargo.toml" in filenames:
            print(f"Running 'cargo {command}' in: {dirpath}")
            try:
                result = subprocess.run(
                    ["cargo", command],
                    cwd=dirpath,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                )
                print(result.stdout, end="")
                if result.returncode != 0:
                    print(
                        f"cargo {command} failed in {dirpath} with exit code {result.returncode}",
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
    if len(sys.argv) == 2:
        sys.exit(run_all(sys.argv[1]))
    elif len(sys.argv) == 3:
        sys.exit(run_all(sys.argv[1], sys.argv[2]))
    else:
        print("error: not arguments should be `python run_all.py {cmd} {optional root_dir}`")