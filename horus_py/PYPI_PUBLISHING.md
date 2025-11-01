# Publishing horus-robotics to PyPI

This guide walks through publishing the `horus-robotics` Python package to PyPI, enabling users to install via `pip install horus-robotics`.

## Current Status

✅ **Package is ready for PyPI publishing!**

- ✅ pyproject.toml configured with complete metadata
- ✅ LICENSE file included (Apache-2.0)
- ✅ README.md comprehensive (498 lines)
- ✅ Wheel builds successfully (611KB)
- ✅ Package name: `horus-robotics`

**Built wheel**: `horus_robotics-0.1.3-cp39-abi3-manylinux_2_34_x86_64.whl`

---

## Prerequisites

### 1. PyPI Account Setup

First-time setup (one-time only):

1. **Create PyPI account**: https://pypi.org/account/register/
2. **Verify email**: Check your email and click verification link
3. **Enable 2FA** (recommended): https://pypi.org/manage/account/

### 2. Create API Token

**IMPORTANT**: Use API tokens instead of passwords for publishing.

1. Go to https://pypi.org/manage/account/token/
2. Click "Add API token"
3. Name: "horus-robotics-publishing"
4. Scope: "Entire account" (first release) or "Project: horus-robotics" (after first release)
5. **Copy the token** (starts with `pypi-...`)
6. Store securely - you won't see it again!

### 3. Configure Token

Save token in `~/.pypirc`:

```bash
cat > ~/.pypirc << 'EOF'
[distutils]
index-servers =
    pypi
    testpypi

[pypi]
repository = https://upload.pypi.org/legacy/
username = __token__
password = pypi-YOUR_TOKEN_HERE

[testpypi]
repository = https://test.pypi.org/legacy/
username = __token__
password = pypi-YOUR_TESTPYPI_TOKEN_HERE
EOF

chmod 600 ~/.pypirc  # Secure permissions
```

Replace `pypi-YOUR_TOKEN_HERE` with your actual API token.

---

## Publishing Workflow

### Step 1: Test Build Locally

```bash
cd /home/lord-patpak/horus/HORUS/horus_py

# Clean previous builds
rm -rf target/wheels/*.whl

# Build wheel
maturin build --release

# Verify wheel
ls -lh target/wheels/
unzip -l target/wheels/*.whl
```

**Expected output**:
```
📦 Built wheel for abi3 Python ≥ 3.9 to /home/lord-patpak/horus/HORUS/target/wheels/horus_robotics-0.1.3-cp39-abi3-manylinux_2_34_x86_64.whl
```

### Step 2: Test on TestPyPI (RECOMMENDED)

Always test on TestPyPI before publishing to real PyPI!

```bash
# Build and publish to TestPyPI
maturin publish --repository testpypi

# Or manually upload
twine upload --repository testpypi target/wheels/*.whl
```

**Verify on TestPyPI**:
```bash
# Install from TestPyPI
pip install --index-url https://test.pypi.org/simple/ horus-robotics

# Test import
python3 -c "import horus; print(horus.__version__)"
```

### Step 3: Publish to PyPI

Once TestPyPI works, publish to real PyPI:

```bash
cd /home/lord-patpak/horus/HORUS/horus_py

# Build and publish in one command
maturin publish

# Or build first, then publish
maturin build --release
twine upload target/wheels/*.whl
```

**You'll see**:
```
📦 Built wheel for abi3 Python ≥ 3.9 to /home/lord-patpak/horus/HORUS/target/wheels/horus_robotics-0.1.3-cp39-abi3-manylinux_2_34_x86_64.whl
🔐 Uploading to PyPI...
✅ Published horus-robotics 0.1.3 to https://pypi.org/project/horus-robotics/
```

### Step 4: Verify Publication

```bash
# Wait 1-2 minutes for PyPI to index

# Install from PyPI
pip install horus-robotics

# Verify
python3 -c "import horus; print('HORUS version:', horus.__version__)"
python3 -c "import horus; node = horus.Node(pubs='test', tick=lambda n: n.send('test', 42)); print('✅ Success!')"
```

**Check PyPI page**: https://pypi.org/project/horus-robotics/

---

## Version Bumping

For future releases, update version in **TWO files**:

1. **horus_py/pyproject.toml**:
   ```toml
   [project]
   name = "horus-robotics"
   version = "0.1.3"  # <- Update here
   ```

2. **horus_py/Cargo.toml**:
   ```toml
   [package]
   name = "horus_py"
   version = "0.1.3"  # <- Update here
   ```

Then rebuild and republish:

```bash
maturin build --release
maturin publish
```

---

## Troubleshooting

### Error: "File already exists"

You can't replace a version once published. Bump the version and republish.

```bash
# Edit version in pyproject.toml and Cargo.toml
# Then republish
maturin publish
```

### Error: "Invalid authentication"

Check your API token:
1. Verify token in `~/.pypirc` is correct
2. Ensure no extra spaces/newlines
3. Token should start with `pypi-`

### Error: "Package name conflict"

If `horus-robotics` is taken, use alternative:
- `horus-py`
- `horus-framework`
- `pyhorus`

Update `name` in `pyproject.toml`:
```toml
[project]
name = "your-alternative-name"
```

### Build Errors

Ensure Rust toolchain is up to date:
```bash
rustup update stable
cargo --version  # Should be 1.70+
```

---

## CI/CD Automation (Future)

For automated releases, add to GitHub Actions:

```yaml
name: Publish to PyPI

on:
  release:
    types: [published]

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.9'

      - name: Install maturin
        run: pip install maturin

      - name: Build and publish
        working-directory: horus_py
        env:
          MATURIN_PYPI_TOKEN: ${{ secrets.PYPI_API_TOKEN }}
        run: maturin publish
```

**Setup**:
1. Add PyPI API token to GitHub Secrets as `PYPI_API_TOKEN`
2. Create GitHub release → triggers automatic PyPI publish

---

## Multi-Platform Wheels (Future Enhancement)

Currently building for: **Linux x86_64**

To support ARM (Raspberry Pi, Jetson):

```bash
# Install cross-compilation tools
cargo install cross

# Build ARM wheel
cross build --target aarch64-unknown-linux-gnu --release
maturin build --release --target aarch64-unknown-linux-gnu

# Publish all wheels
maturin publish
```

Platforms to support:
- ✅ Linux x86_64 (done)
- ⏳ Linux ARM64 (Raspberry Pi 4/5, Jetson)
- ⏳ Linux ARMv7 (Raspberry Pi 3)

---

## Updating horus_manager

After publishing to PyPI, update `horus_manager` to use pip install:

**Before** (manual install):
```bash
cd horus_py && maturin develop --release
```

**After** (pip install):
```bash
pip install horus-robotics
```

Update `horus_manager/src/registry.rs` to auto-install:

```rust
// When installing a Python project
if package.language == "python" {
    // Auto-install horus-robotics from PyPI
    run_command(&["pip", "install", "horus-robotics"]);
}
```

---

## Quick Reference

```bash
# Build wheel
cd horus_py && maturin build --release

# Test locally
maturin develop --release
python3 -c "import horus; print('✅ Works!')"

# Publish to TestPyPI
maturin publish --repository testpypi

# Publish to PyPI
maturin publish

# Install from PyPI
pip install horus-robotics

# Uninstall
pip uninstall horus-robotics
```

---

## Checklist for First Release

- [ ] PyPI account created and verified
- [ ] API token generated and saved in `~/.pypirc`
- [ ] Wheel builds successfully (`maturin build --release`)
- [ ] Tested on TestPyPI
- [ ] Version is correct in both `pyproject.toml` and `Cargo.toml`
- [ ] README.md is comprehensive
- [ ] LICENSE file is included
- [ ] Published to PyPI (`maturin publish`)
- [ ] Verified installation (`pip install horus-robotics`)
- [ ] Updated main HORUS docs to mention `pip install horus-robotics`

---

## Support

**Questions?** Open an issue: https://github.com/horus-org/horus/issues

**Documentation**: https://docs.horus.rs
