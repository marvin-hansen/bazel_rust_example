# Rust with Bzlmod

This document describes how to use rules_rust with Bazel's new external dependency
subsystem [Bzlmod](https://bazel.build/external/overview#bzlmod), which is meant to replace `WORKSPACE` files over time.
Usages of rules_rust in `BUILD` files are not affected by this; refer to
the [existing documentation on rules](https://bazelbuild.github.io/rules_rust/#rules) and configuration options for
them.

# Table of Contents

1. [Supported bazel versions](#supported-bazel-versions)
2. [Supported platforms](#supported-platforms)
3. [Setup](#Setup)
4. [Rust SDK](#rust-sdk)
5. [Dependencies](#Dependencies)

## Supported bazel versions

The oldest version of Bazel the `main` branch is tested against is `6.3.0`. Previous versions may still be functional in
certain environments, but this is the minimum version we strive to fully support. We test these rules against the latest
rolling releases of Bazel, and aim for compatibility with them, but prioritise stable releases over rolling releases
where necessary.

## Supported platforms

We aim to support Linux and macOS.

We do not have sufficient maintainer expertise to support Windows. Most things probably work, but we have had to disable
many tests in CI because we lack the expertise to fix them. We welcome contributions to help improve its support.
Windows support for some features requires `--enable_runfiles` to be passed to Bazel, we recommend putting it in your
bazelrc. See [Using Bazel on Windows](https://bazel.build/configure/windows) for more Windows-specific recommendations.

## Setup

Add the following lines to your `MODULE.bazel` file to define a minimal Rust setup.
The latest versions of rules_rust are listed on https://registry.bazel.build/modules/rules_rust.

```starlark
bazel_dep(name = "rules_rust", version = "0.46.0")
```

## Rust SDK

A basic setup will pick automatically the stable version for the selected Rust edition.

```starlark
rust = use_extension("@rules_rust//rust:extensions.bzl", "rust")
rust.toolchain(edition = "2021")
use_repo(rust, "rust_toolchains")
register_toolchains("@rust_toolchains//:all")
```

To register a specific version of the Rust SDK, add it to the toolchain declaration.

```starlark
# Rust toolchain
# https://forge.rust-lang.org/
RUST_EDITION = "2021"
RUST_VERSION = "1.79.0"

rust = use_extension("@rules_rust//rust:extensions.bzl", "rust")
rust.toolchain(
    edition = RUST_EDITION,
    versions = [RUST_VERSION],
)
use_repo(rust, "rust_toolchains")
register_toolchains("@rust_toolchains//:all")
```

If you want to build against multiple Rust versions, ensure you have all versions declared in the toolchain.

```starlark
# Rust toolchain
# https://forge.rust-lang.org/
RUST_EDITION = "2021"
RUST_STABLE = "1.79.0"
RUST_BETA = "1.80.0"
RUST_NIGHTLY = "1.81.0"

rust = use_extension("@rules_rust//rust:extensions.bzl", "rust")
rust.toolchain(
    edition = RUST_EDITION,
    versions = [
        RUST_STABLE,
        RUST_BETA,
        RUST_NIGHTLY,
    ],
)
use_repo(rust, "rust_toolchains")
register_toolchains("@rust_toolchains//:all")
```

As long as you specify the `version` of an SDK, it will be downloaded when it is needed during a build. The usual
rules of [toolchain resolution](https://bazel.build/extending/toolchains#toolchain-resolution) apply, with SDKs
registered in the root module taking precedence over those registered in dependencies.

## Dependencies

Crate Universe is a set of Bazel rule for generating Rust targets using Cargo.

### Setup

After loading rules_rust in your `MODULE.bazel` file, set the following to use crate_universe:

```starlark
crate = use_extension("@rules_rust//crate_universe:extension.bzl", "crate")
```

External dependencies can be declared in one of two ways:

1) Cargo Workspaces
2) Direct Packages

### Cargo Workspaces

One of the simpler ways to wire up dependencies would be to first structure your project into a Cargo workspace. The
crates_repository rule can ingest a root Cargo.toml file and generate Bazel dependencies from there.

```starlark
crate.from_cargo(
    name = "crates",
    cargo_lockfile = "//:Cargo.lock",
    manifests = ["//:Cargo.toml"],
)
use_repo(crate, "crates")
```

The generated crates_repository contains helper macros which make collecting dependencies for Bazel targets simpler.
Notably, the all_crate_deps and aliases macros (
see [Dependencies API](https://bazelbuild.github.io/rules_rust/crate_universe.html#dependencies-api)) commonly allow the
Cargo.toml files to be the single source of truth for dependencies.
Since these macros come from the generated repository, the dependencies and alias definitions they return will
automatically update BUILD targets. In your BUILD files, you use these macros as shown below:

```starlark
load("@crate_index//:defs.bzl", "aliases", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "lib",
    aliases = aliases(),
    deps = all_crate_deps(
        normal = True,
    ),
    proc_macro_deps = all_crate_deps(
        proc_macro = True,
    ),
)

rust_test(
    name = "unit_test",
    crate = ":lib",
    aliases = aliases(
        normal_dev = True,
        proc_macro_dev = True,
    ),
    deps = all_crate_deps(
        normal_dev = True,
    ),
    proc_macro_deps = all_crate_deps(
        proc_macro_dev = True,
    ),
)
```

For a code example, see
the [all_crate_deps example](https://github.com/bazelbuild/rules_rust/tree/main/examples/bzlmod/all_crate_deps).

It is important to know that when you add new dependencies to your Cargo.toml, you need to re-generate the Bazel targets
by running:

```Bash
# Syncs Cargo dependencies to Bazel index
CARGO_BAZEL_REPIN=1 bazel sync --only=crates
```

For more details, see the section about [repinning / updating Dependencies.](#repinning--updating-dependencies)

### Direct Packages

In cases where Rust targets have heavy interractions with other Bazel targests (
[Cc](https://docs.bazel.build/versions/main/be/c-cpp.html), [Proto](https://rules-proto-grpc.com/en/4.5.0/lang/rust.html),
etc.), maintaining Cargo.toml files may have diminishing returns as things like rust-analyzer begin to be confused about
missing targets or environment variables defined only in Bazel. In workspaces like this, it may be desirable to have a
“Cargo free” setup. crates_repository supports this through the packages attribute.

```starlark
# External crates
crate.spec(
    package = "serde", 
    features = ["derive"], 
    version = "1.0
 )
 
crate.spec(
    package = "serde_json", 
    version = "1.0"
)

crate.spec(
    package = "tokio", 
    default_features=False, 
    features = ["macros", "net", "rt-multi-thread", "signal"], version = "1.38"
)

crate.from_specs()
use_repo(crate, "crates")
```

Consuming dependencies may be more ergonomic in this case through the aliases defined in the new repository. In your
BUILD files, you use direct dependencies as shown below:

```starlark
rust_binary(
    name = "bin",
    crate_root = "src/main.rs",
    srcs = glob([
        "src/*.rs",
    ]),
    deps = [
        # External crates
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:tokio",
    ],
    visibility = ["//visibility:public"],
)
```

For a code example, see
the [hello_world_no_cargo](https://github.com/bazelbuild/rules_rust/tree/main/examples/bzlmod/hello_world_no_cargo)
example.

Notice, direct dependencies do not need repining. Only a cargo workspace needs updating whenever the underlying
Cargo.toml file changed.

### Repinning / Updating Dependencies

Dependency syncing and updating is done in the repository rule which means it's done during the
analysis phase of builds. As mentioned in the environments variable table above, the `CARGO_BAZEL_REPIN`
(or `REPIN`) environment variables can be used to force the rule to update dependencies and potentially
render a new lockfile. Given an instance of this repository rule named `crates`, the easiest way to
repin dependencies is to run:

```shell
CARGO_BAZEL_REPIN=1 bazel sync --only=crates
```

This will result in all dependencies being updated for a project. The `CARGO_BAZEL_REPIN` environment variable
can also be used to customize how dependencies are updated. The following table shows translations from environment
variable values to the equivilant [cargo update](https://doc.rust-lang.org/cargo/commands/cargo-update.html) command
that is called behind the scenes to update dependencies.

| Value                                          | Cargo command                                                |
|------------------------------------------------|--------------------------------------------------------------|
| Any of [`true`, `1`, `yes`, `on`, `workspace`] | `cargo update --workspace`                                   |
| Any of [`full`, `eager`, `all`]                | `cargo update`                                               |
| `package_name`                                 | `cargo upgrade --package package_name`                       |
| `package_name@1.2.3`                           | `cargo upgrade --package package_name@1.2.3`                 |
| `package_name@1.2.3=4.5.6`                     | `cargo upgrade --package package_name@1.2.3 --precise=4.5.6` |

If the `crates_repository` is used multiple times in the same Bazel workspace (e.g. for multiple independent
Rust workspaces), it may additionally be useful to use the `CARGO_BAZEL_REPIN_ONLY` environment variable, which
limits execution of the repinning to one or multiple instances of the `crates_repository` rule via a comma-delimited
allowlist:

```shell
CARGO_BAZEL_REPIN=1 CARGO_BAZEL_REPIN_ONLY=crates bazel sync --only=crates