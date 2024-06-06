# Bazel Rust Example 

Example of how to build a mini Rust monorepo using Bazel.

The Cargo config uses a standard workspace with central dependencies to inherited to two crates.
The Bazel config roughly replicates the idea, but also adds Docker-less OCI image building & publishing.
Publishing container images to a container registry, however, is disabled until the target registry is configured.
Run `make release` to see details which config to update. 

## Requirements

* Cargo & Rust
* C compiler (gcc, or clang)
* Bazelisk ([Link to installation](https://bazel.build/install/bazelisk))

## 🛠️ Cargo, Bazel & Make

Cargo and Bazel work as expected, but in addition, a makefile exists
that abstracts over Bazel and Cargo to simplify working on the repo.

```bash 
 Run Services:
    make run            Run the default binary.

 Development:
    make build          Build the code base incrementally (fast) for dev.
    make rebuild        Sync dependencies and builds the code base from scratch (slow).
    make release        Build & test binaries and then build & publish container images (slow).
    make container      Build the container images.
    make doc            Build documentation for the project.
    make fix            Fix linting issues as reported by clippy.
    make format         Format call code according to cargo fmt style.
    make test           Test all crates.
```

The scripts called by each make command are located in the [script folder.](scripts)

### Footnote on Bazel install

Do *NOT* install Bazel directly or with a system package manager (apt etc.) because any major Bazel update
may break your project build in unexpected ways. This project is pinned to Bazel 7, an LTS release
that [guarantees no breaking changes until Dec 2026](https://bazel.build/release).

Instead, set the desired Bazel version in the .bazeliskrc config file, let Bazelisk download Bazel for you and let
things for smoothly. If you ever want to update to a newer Bazel version, just bump up the version number in the
.bazeliskrc config file and test if everything builds. If you encounter any unresolvable error, just revert back to the
previous version, fill a bug or subscribe to an issue and wait until its solved. 
