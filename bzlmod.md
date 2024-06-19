# Rust with Bzlmod

This document describes how to use rules_rust with Bazel's new external dependency
subsystem [Bzlmod](https://bazel.build/external/overview#bzlmod), which is meant to replace `WORKSPACE` files over time.
Usages of rules_rust in `BUILD` files are not affected by this; refer to
the [existing documentation on rules](https://bazelbuild.github.io/rules_rust/#rules) and configuration options for
them.

# Table of Contents

1. [Setup](#Setup)
2. [Rust SDK](#rust-sdk)
3. [Dependencies](#Dependencies)
4. [Rust Proto / gRPC](#rust-proto)

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
can also be used to customize how dependencies are updated. For more details about
repin, [please refer to the documentation](https://bazelbuild.github.io/rules_rust/crate_universe.html#crates_vendor).

## Rust Proto

These build rules are used for building protobufs/gRPC in Rust with Bazel.

The prost and tonic rules do not specify a default toolchain in order to avoid mismatched dependency issues. While the
tonic toolchain works out of the box
when its dependencies are matched, however, Prost requires a custom toolchain you have to define before you can build
proto files with rules_rust.

### Setup

The setup requires three steps to complete before you can configure proto targets.

1. Configure rules and dependencies in MODULE.bazel
2. Configure a custom Prost toolchain
3. Register custom Prost toolchain.

**1) Configure rules and dependencies**

In your MODULE.bazel, you add three new entries:

```starlark
# 1 Register rules for proto
###############################################################################

# https://github.com/bazelbuild/rules_proto/releases
bazel_dep(name = "rules_proto", version = "6.0.2")
# https://github.com/aspect-build/toolchains_protoc/releases
bazel_dep(name = "toolchains_protoc", version = "0.3.1")
# https://registry.bazel.build/modules/protobuf
bazel_dep(name = "protobuf", version = "27.1")

# 2 Register Proto toolchain 
###############################################################################
# Proto toolchain
register_toolchains("@rules_rust//proto/protobuf:default-proto-toolchain")

# 3 Register proto / prost / tonic crates 
###############################################################################
crate = use_extension("@rules_rust//crate_universe:extension.bzl", "crate")

# protobufs / gRPC
crate.spec(package = "prost", version = "0.12")
crate.spec(package = "prost-types", default_features = False, version = "0.12")
crate.spec(package = "tonic", features = ["transport"], version = "0.11")
crate.spec(package = "tonic-build", version = "0.11")
crate.spec(package = "protoc-gen-prost", version = "0.3.1")
crate.spec(package = "protoc-gen-tonic", version = "0.4.0")

crate.annotation(
    crate = "protoc-gen-prost",
    gen_binaries = ["protoc-gen-prost"],
)

crate.annotation(   
    crate = "protoc-gen-tonic",   
    gen_binaries = ["protoc-gen-tonic"],
)

# Other Rust dependencies ... 

crate.from_specs()
use_repo(crate, "crates")
```

**2) Configure a custom Prost toolchain**

Configuring a custom Prost toolchain is straightforward, you create a new folder with an empty BUILD.bazl file, and add
the toolchain definition.
As your Bazel setup grows over time, it is a best practice to put all custom macros, rules, and toolchains in a
dedicated folder, for example: `build/`.

Suppose you have your BUILD.bazl file in `build/prost_toolchain/BUILD.bazel`, then add the following content:

```starlark
load("@rules_rust//proto/prost:defs.bzl", "rust_prost_toolchain")
load("@rules_rust//rust:defs.bzl", "rust_library_group")

rust_library_group(
    name = "prost_runtime",
    deps = [
        "@crates//:prost",
    ],
)

rust_library_group(
    name = "tonic_runtime",
    deps = [
        ":prost_runtime",
        "@crates//:tonic",
    ],
)

rust_prost_toolchain(
    name = "prost_toolchain_impl",
    prost_plugin = "@crates//:protoc-gen-prost__protoc-gen-prost",
    prost_runtime = ":prost_runtime",
    prost_types = "@crates//:prost-types",
    proto_compiler = "@protobuf//:protoc",
    tonic_plugin = "@crates//:protoc-gen-tonic__protoc-gen-tonic",
    tonic_runtime = ":tonic_runtime",
)

toolchain(
    name = "prost_toolchain",
    toolchain = "prost_toolchain_impl",
    toolchain_type = "@rules_rust//proto/prost:toolchain_type",
)
```

The Prost and Tonic dependencies are pulled from the previously configured
crate dependencies in the MODULE file. With this custom toolchain in place, the last step is to register it.

**3. Register custom Prost toolchain.**

In your MODULE.bazel file, locate your toolchains and add the following entry right below the proto toolchain.

```starlark
# Custom Prost toolchain
register_toolchains("@//build/prost_toolchain")
```

Pay attention to the path, `build/prost_toolchain` because if your toolchain
is in a different folder, you have to update this path to make the build work.

### Usage

Once the setup has been completed, you use the proto & prost targets as you normally do. For example, to configure rust
bindings for a proto file, just add the target:

```starlark
load("@rules_proto//proto:defs.bzl", "proto_library")
load("@rules_rust//proto/prost:defs.bzl", "rust_prost_library")

# Build proto files
# https://bazelbuild.github.io/rules_rust/rust_proto.html#rust_proto_library
proto_library(
    name = "proto_bindings",
    srcs = [
          "proto/helloworld.proto",
    ],
)

# Generate Rust bindings from the generated proto files
# https://bazelbuild.github.io/rules_rust/rust_proto.html#rust_prost_library
rust_prost_library(
    name = "rust_proto",
    proto = ":proto_bindings",
    visibility = ["//visibility:public"],
)
```

From there, you
just [follow the target documentation](https://bazelbuild.github.io/rules_rust/rust_proto.html#rust_proto_library). 