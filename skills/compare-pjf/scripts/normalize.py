#!/usr/bin/env python3
"""Compare dprint vs PJF output with import normalization.

Usage: normalize.py <dprint_dir> <pjf_dir> [total_file_count]

Normalization:
  - Strips java.lang.* imports (PJF removes these; our formatter keeps them)
  - Sorts import blocks alphabetically
  - Strips trailing whitespace from all lines
"""

import difflib
import os
import re
import sys


def normalize_java(content: str) -> str:
    """Normalize a Java file for comparison."""
    lines = content.splitlines()
    result = []
    import_block = []
    in_imports = False

    for line in lines:
        stripped = line.rstrip()

        # Detect import lines
        if re.match(r"^import\s+(static\s+)?", stripped):
            # Skip java.lang.* imports (non-static, non-subpackage)
            if re.match(r"^import\s+java\.lang\.[A-Z]\w*;$", stripped):
                continue
            import_block.append(stripped)
            in_imports = True
            continue

        # End of import block — flush sorted
        if in_imports and not re.match(r"^\s*$", stripped):
            in_imports = False
            result.extend(sorted(import_block))
            import_block = []

        # Preserve blank lines within import region
        if in_imports and re.match(r"^\s*$", stripped):
            continue

        result.append(stripped)

    # Flush any trailing import block
    if import_block:
        result.extend(sorted(import_block))

    return "\n".join(result) + "\n" if result else ""


def find_java_files(directory: str) -> dict[str, str]:
    """Find all .java files and return {relative_path: abs_path}."""
    files = {}
    for root, _, filenames in os.walk(directory):
        for fname in filenames:
            if fname.endswith(".java"):
                abs_path = os.path.join(root, fname)
                rel_path = os.path.relpath(abs_path, directory)
                files[rel_path] = abs_path
    return files


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <dprint_dir> <pjf_dir> [total_file_count]")
        sys.exit(1)

    dprint_dir = sys.argv[1]
    pjf_dir = sys.argv[2]
    expected_count = int(sys.argv[3]) if len(sys.argv) > 3 else None

    dprint_files = find_java_files(dprint_dir)
    pjf_files = find_java_files(pjf_dir)

    # Use intersection of files present in both
    common = sorted(set(dprint_files) & set(pjf_files))
    total = len(common)

    if expected_count and total != expected_count:
        print(f"Warning: expected {expected_count} files but found {total} in common")

    matching = 0
    differing = []

    for rel_path in common:
        with open(dprint_files[rel_path]) as f:
            dprint_content = normalize_java(f.read())
        with open(pjf_files[rel_path]) as f:
            pjf_content = normalize_java(f.read())

        if dprint_content == pjf_content:
            matching += 1
        else:
            differing.append(rel_path)

    # Report
    pct = (matching / total * 100) if total > 0 else 0
    print(f"=== PJF Comparison Results ===")
    print(f"Total files: {total}")
    print(f"Matching:    {matching} ({pct:.1f}%)")
    print(f"Differing:   {len(differing)}")

    if not differing:
        print("\nAll files match!")
        return

    # Show diffs for mismatching files
    print(f"\n=== Diffs ({len(differing)} files) ===\n")
    for rel_path in differing:
        with open(dprint_files[rel_path]) as f:
            dprint_lines = normalize_java(f.read()).splitlines(keepends=True)
        with open(pjf_files[rel_path]) as f:
            pjf_lines = normalize_java(f.read()).splitlines(keepends=True)

        diff = list(difflib.unified_diff(
            pjf_lines, dprint_lines,
            fromfile=f"pjf/{rel_path}",
            tofile=f"dprint/{rel_path}",
            n=3,
        ))
        if diff:
            print(f"--- {rel_path} ---")
            hunks = sum(1 for line in diff if line.startswith("@@"))
            print(f"    {hunks} hunk(s)")
            sys.stdout.writelines(diff)
            print()


if __name__ == "__main__":
    main()
