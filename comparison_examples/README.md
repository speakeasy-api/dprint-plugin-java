# PJF Comparison Examples (PR #3837)

These diff files show examples of formatting differences between dprint-plugin-java and Spotless:PJF on the `sdk-javav2` repository.

## Files

1. **blank_lines_SDK.diff** - Shows the primary gap: blank lines between class members
   - Missing blank line after package header comment
   - Missing blank lines between fields/methods
   - Extra blank line after imports

2. **arglist_wrapping_TypesTests.diff** - Shows argument list wrapping differences
   - PJF wraps nested builder patterns in arguments
   - PJF wraps long method chains in variable assignments

## How to View

```bash
# View with syntax highlighting
bat blank_lines_SDK.diff

# View side-by-side
diff -y normalized/dprint/SDK.java normalized/spotless/SDK.java | less
```

## Full Comparison Location

All comparison artifacts are in `/tmp/fmt-comparison/`:
- `dprint/` - dprint-formatted output
- `spotless-runner/src/` - PJF-formatted output
- `normalized/` - Both outputs after import sorting and whitespace normalization

## Summary

See `PJF_COMPARISON_REPORT_PR3837.md` in the repository root for the full analysis.
