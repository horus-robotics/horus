# MDX Writing Guidelines for HORUS Documentation

**CRITICAL REFERENCE**: Read this before editing any `.mdx` files to avoid common rendering errors.

## Common MDX Pitfalls (MUST AVOID)

### 1. Less-Than Symbol Before Numbers or Letters

**WRONG**:
```markdown
- Performance: <1% variance
- Latency: <5μs
- Error rate: <0.1%
```

**CORRECT**:
```markdown
- Performance: &lt;1% variance
- Latency: &lt;5μs
- Error rate: &lt;0.1%
```

**Why**: MDX interprets `<1` as the start of a JSX component tag `<1>`, which is invalid HTML/JSX (tags cannot start with numbers).

### 2. Generic Type Syntax

**WRONG**:
```markdown
### Error: "Failed to create Hub<T>"
```

**CORRECT**:
```markdown
### Error: "Failed to create `Hub<T>`"
```

**Why**: MDX interprets `Hub<T>` as a JSX component. Always wrap generic types in backticks.

### 3. Comparison Operators in Text

**WRONG**:
```markdown
Values <100 are considered low
Use values >50 for best results
```

**CORRECT**:
```markdown
Values &lt;100 are considered low
Use values &gt;50 for best results
```

**Or use backticks for code**:
```markdown
Values `<100` are considered low
Use values `>50` for best results
```

### 4. Headings Starting with Numbers

**AVOID**:
```markdown
### 1. First Step
### 2. Second Step
```

**BETTER**:
```markdown
### Step 1: First Step
### Step 2: Second Step
```

**Why**: While we have a fix in `lib/mdx.tsx` that prefixes numeric IDs, it's cleaner to avoid starting headings with numbers.

## Safe Patterns

### In Tables

```markdown
| Metric | Value | Notes |
|--------|-------|-------|
| Latency | &lt;5μs | Always escape |
| Variance | &lt;1% | Use HTML entities |
```

### In Code Blocks

Inside code blocks, you can use `<` and `>` normally:

```markdown
\`\`\`rust
let hub: Hub<f32> = Hub::new("topic")?;
if value < 100 {
    // This is fine inside code blocks
}
\`\`\`
```

### In Inline Code

Inside backticks, you can use `<` and `>`:

```markdown
Use `Hub<T>` for pub-sub messaging.
Values `<100` are considered low.
```

## HTML Entities Reference

| Character | Entity | Use Case |
|-----------|--------|----------|
| `<` | `&lt;` | Less than comparisons, generic types |
| `>` | `&gt;` | Greater than comparisons |
| `&` | `&amp;` | Ampersands in text |
| `"` | `&quot;` | Quotes in attributes |

## Quick Checklist Before Committing

- [ ] Search file for `<[0-9]` patterns outside code blocks
- [ ] Search file for `<[A-Z]` patterns (like `Hub<T>`) outside backticks
- [ ] Check that all comparison operators use HTML entities
- [ ] Verify headings don't start with raw numbers
- [ ] Build locally: `npm run build` should show no MDX errors

## Testing

Before committing documentation changes:

```bash
cd docs-site
rm -rf .next
npm run build 2>&1 | grep "Error loading doc"
```

If you see "Error loading doc" messages, you likely have an MDX syntax issue.

## Common Error Messages

### "Unexpected character `1` before name"
**Cause**: Using `<1` in text (e.g., "latency <1ms")
**Fix**: Change to `&lt;1`

### "Unexpected character before name, expected a letter"
**Cause**: Using generic types without backticks (e.g., `Hub<T>`)
**Fix**: Wrap in backticks: `` `Hub<T>` ``

## Files to Check

If you're editing these files, be extra careful:
- `benchmarks.mdx` - Contains many performance metrics with `<` symbols
- `rtos-integration.mdx` - Contains latency measurements
- `troubleshooting-runtime.mdx` - Contains error messages with generic types
- Any file with tables showing performance data

## Auto-Fix Script

If you need to bulk-fix a file:

```bash
# Replace common patterns
sed -i 's/: <\([0-9]\)/: \&lt;\1/g' content/docs/yourfile.mdx
sed -i 's/| <\([0-9]\)/| \&lt;\1/g' content/docs/yourfile.mdx
```

**Note**: Always review changes manually after running auto-fix scripts.
