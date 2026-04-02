# GitHub Actions Release Workflow — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a GitHub Actions workflow that builds cross-platform executables and publishes a GitHub Release with download links when a version tag is pushed.

**Architecture:** Single workflow file with three sequential jobs: `build` (matrix of 4 OS targets), `release` (creates GitHub Release with artifacts), and `update-readme` (updates download links in README on main).

**Tech Stack:** GitHub Actions, `dtolnay/rust-toolchain`, `actions/upload-artifact`, `actions/download-artifact`, `softprops/action-gh-release`

---

### Task 1: Create the release workflow file with build matrix

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create `.github/workflows/` directory**

```bash
mkdir -p .github/workflows
```

- [ ] **Step 2: Write the workflow file with trigger, build matrix, and packaging**

Create `.github/workflows/release.yml` with the following content:

```yaml
name: Release

on:
  push:
    tags: ['v*.*.*']

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.name }}
    runs-on: ${{ matrix.runner }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - name: linux-x86_64
            runner: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            archive: rewind-x86_64-linux.tar.gz
            binary: rewind
          - name: windows-x86_64
            runner: windows-latest
            target: x86_64-pc-windows-msvc
            archive: rewind-x86_64-windows.zip
            binary: rewind.exe
          - name: macos-x86_64
            runner: macos-13
            target: x86_64-apple-darwin
            archive: rewind-x86_64-macos.tar.gz
            binary: rewind
          - name: macos-aarch64
            runner: macos-14
            target: aarch64-apple-darwin
            archive: rewind-aarch64-macos.tar.gz
            binary: rewind

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build release binary
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package (tar.gz)
        if: runner.os != 'Windows'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../${{ matrix.archive }} ${{ matrix.binary }}

      - name: Package (zip)
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          Compress-Archive -Path "target/${{ matrix.target }}/release/${{ matrix.binary }}" -DestinationPath "${{ matrix.archive }}"

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.archive }}
          path: ${{ matrix.archive }}
```

- [ ] **Step 3: Verify the YAML is valid**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "Valid YAML"
```

Expected: `Valid YAML`

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add release workflow with cross-platform build matrix"
```

### Task 2: Add the release job

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Add the release job after the build job**

Append the following job to `.github/workflows/release.yml`, inside the `jobs:` block after `build`:

```yaml
  release:
    name: Create Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Extract tag message
        id: tag_message
        run: |
          TAG=${GITHUB_REF#refs/tags/}
          MESSAGE=$(git tag -l --format='%(contents)' "$TAG")
          {
            echo "body<<EOF"
            echo "$MESSAGE"
            echo "EOF"
          } >> "$GITHUB_OUTPUT"

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Collect archives
        run: |
          mkdir release-assets
          find artifacts -type f \( -name '*.tar.gz' -o -name '*.zip' \) -exec mv {} release-assets/ \;

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          name: ${{ github.ref_name }}
          body: ${{ steps.tag_message.outputs.body }}
          files: release-assets/*
```

- [ ] **Step 2: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "Valid YAML"
```

Expected: `Valid YAML`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add release job to create GitHub Release with artifacts"
```

### Task 3: Add the update-readme job

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `README.md` (by the workflow at runtime — lines 99-104)

- [ ] **Step 1: Add the update-readme job after the release job**

Append the following job to `.github/workflows/release.yml`, inside the `jobs:` block after `release`:

```yaml
  update-readme:
    name: Update README Download Links
    needs: release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          ref: main

      - name: Update download links
        run: |
          TAG=${GITHUB_REF#refs/tags/}
          BASE_URL="https://github.com/yanekyuk/rewind/releases/download/${TAG}"

          sed -i "s#| Windows (64-bit) | .*#| Windows (64-bit) | [\`rewind-x86_64-windows.zip\`](${BASE_URL}/rewind-x86_64-windows.zip) |#" README.md
          sed -i "s#| macOS (Apple Silicon) | .*#| macOS (Apple Silicon) | [\`rewind-aarch64-macos.tar.gz\`](${BASE_URL}/rewind-aarch64-macos.tar.gz) |#" README.md
          sed -i "s#| macOS (Intel) | .*#| macOS (Intel) | [\`rewind-x86_64-macos.tar.gz\`](${BASE_URL}/rewind-x86_64-macos.tar.gz) |#" README.md
          sed -i "s#| Linux (64-bit) | .*#| Linux (64-bit) | [\`rewind-x86_64-linux.tar.gz\`](${BASE_URL}/rewind-x86_64-linux.tar.gz) |#" README.md

      - name: Commit and push
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
          git diff --quiet README.md || {
            git add README.md
            git commit -m "docs: update download links for ${GITHUB_REF#refs/tags/}"
            git push origin main
          }
```

- [ ] **Step 2: Validate YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "Valid YAML"
```

Expected: `Valid YAML`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add update-readme job to update download links on release"
```

### Task 4: Validate complete workflow

**Files:**
- Read: `.github/workflows/release.yml`

- [ ] **Step 1: Read back the full workflow and verify structure**

Verify the file has:
- `on.push.tags` trigger
- `permissions.contents: write`
- 3 jobs: `build`, `release`, `update-readme`
- `release` has `needs: build`
- `update-readme` has `needs: release`

- [ ] **Step 2: Validate YAML one final time**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "Valid YAML"
```

- [ ] **Step 3: Push to main**

```bash
git push origin main
```
