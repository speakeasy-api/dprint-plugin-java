#!/usr/bin/env python3
"""Update spec test files with actual formatter output."""

import subprocess
import sys
from pathlib import Path

def update_spec_file(spec_path):
    """Update a single spec file with actual formatter output."""
    content = spec_path.read_text()

    if "== input ==" not in content or "== output ==" not in content:
        return False

    # Extract input section
    parts = content.split("== input ==")
    if len(parts) != 2:
        return False

    before_input = parts[0]
    after_input = parts[1]

    output_parts = after_input.split("== output ==")
    if len(output_parts) != 2:
        return False

    input_code = output_parts[0].strip() + "\n"

    # Run formatter on input
    try:
        result = subprocess.run(
            ["cargo", "run", "--release", "--", "fmt"],
            input=input_code,
            capture_output=True,
            text=True,
            timeout=5
        )

        if result.returncode != 0:
            print(f"Error formatting {spec_path}: {result.stderr}")
            return False

        formatted_output = result.stdout

        # Reconstruct spec file
        new_content = f"{before_input}== input ==\n{input_code}== output ==\n{formatted_output}"

        if new_content != content:
            spec_path.write_text(new_content)
            print(f"Updated {spec_path}")
            return True

    except subprocess.TimeoutExpired:
        print(f"Timeout formatting {spec_path}")
        return False
    except Exception as e:
        print(f"Error with {spec_path}: {e}")
        return False

    return False

def main():
    specs_dir = Path("tests/specs")
    updated = 0

    for spec_file in specs_dir.rglob("*.txt"):
        if update_spec_file(spec_file):
            updated += 1

    print(f"\nUpdated {updated} spec files")

if __name__ == "__main__":
    main()
