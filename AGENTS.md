# Arca: notes for working in this repository

## Build environment (do not probe for toolchains)

- The host is Windows with `git.exe`. It has **no** Rust, Cargo, Android SDK/NDK, and **no** `gh` CLI.
- All compiling, linting and testing happens inside the Docker container `arca-dev` (from `ubuntu:24.04`,
  repository bind-mounted at `/src`, `CARGO_TARGET_DIR=/target`, rustup in `/root/.cargo/bin`, which is **not**
  on the `PATH` of a login shell). Docker Desktop (`%LOCALAPPDATA%\Programs\DockerDesktop`) may be stopped, and
  the container may be exited: start both first (`docker start arca-dev`).
- The container has `/opt/android-sdk` with platforms and cmdline-tools, but no build-tools, no android.jars
  (`/opt/jars`), no `cargo-deny` and no `cargo-audit`: `tools/android/check-android.sh` and APK packaging fail
  there until those are installed (`ARCA_BUILD_TOOLS=…`, `ARCA_ANDROID_JARS=…`). CI runs them on every push.
- From Git Bash:

  ```bash
  MSYS_NO_PATHCONV=1 docker exec arca-dev bash -c 'export PATH=/root/.cargo/bin:$PATH; cd /src && cargo check --workspace'
  ```

- The malware-lab samples and snapshots (`tests/malware-test/`) describe real exploits, and antivirus software may
  remove them from the working copy. If a snapshot is missing (or excluded in `.git/info/sparse-checkout`),
  `tools/check-malware-lab.sh` fails locally only; add an antivirus exclusion for the folder and restore the file.

- `cargo fmt` cannot rewrite files on the bind mount (os error 22): copy `crates/` to `/tmp/fmt` in the container, run
  `cargo fmt --all` there, copy changed files back. `cargo fmt --all --check` works in place.
- Heredocs with quotes in the Bash tool sometimes mis-parse; write scripts with the Write tool and run them.
- Fast loop: `cargo check --workspace`. Run the full chain (`tools/build-guest.sh`, `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `tools/check-unsafe.sh`,
  `tools/check-malware-lab.sh`, `tools/android/check-android.sh`) once, before the release commit.

## Line endings

`.gitattributes` sets `* text=auto eol=lf` (plus explicit `eol=lf` for `*.sh`, `*.rs`, `*.toml`); `core.autocrlf` is `false`.
The working copy is LF, so `tools/check-unsafe.sh` passes on it. The script also strips `\r` when it reads manifests.

## Releases

- Release secrets (`ARCA_KEYSTORE_B64`, `ARCA_KEYSTORE_PASS`, `ARCA_KEY_ALIAS`) live in the GitHub `release` environment.
  No local keystore password is needed.
- Pushing `main` and then, as a separate command, the annotated tag `vX.Y.Z` starts `.github/workflows/android.yml`; its
  `package-release` job signs with the permanent key and publishes the GitHub release.
- Release notes live in `docs/releases/vX.Y.Z.md`.
