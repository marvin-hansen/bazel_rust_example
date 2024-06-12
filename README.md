# Bazel Rust Example

Example of how to build a mini Rust monorepo using Bazel based on
the [hello world Prost gRPC example](https://github.com/hyperium/tonic/blob/master/examples/README.md).

The project covers quite a bit of groundwork:

* Cargo & Bazel config side by side
* Bazel direct dependencies (not generated from Cargo.toml)
* Builds two crates in a workspace where one depends on the other
* Builds proto bindings for gRPC with prost
* Shares proto definitions between client and server
* Applies the full-swing of compiler optimization & binary size reduction using pass-through options
* Builds and tags OCI images docker-less (I wrote some custom macros to simplify these tasks)

Publishing the container image to a container registry, however, is disabled until the target registry is configured.
Please run `make release` to see details which config to update.

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
    make fix            Fix linting issues as reported by clippy.
    make format         Format call code according to cargo fmt style.
    make test           Test all crates.
```

The scripts called by each make command are located in the [script folder.](scripts)

## Bazelmod support

The Bazel project decided to change the main configuration from the previous WORKSPACE format to the
current MODULE (a.k.a bazelmod) format. Since Bazel 7, the Bazelmod format has been set as the new default. This demo
project uses the current bazelmod, but also comes with a working[ WORKSPACE configuration](config/workspace/WORKSPACE).
This may help people who are
trying to convert an existing Bazel project from the previous format to the new Bazelmod configuration format.

## Bazel configuration

Conventionally, a Bazel project uses three configuration files and an optional bazeliskrc configuration. These config
files are:

1) [.bazeliskrc](.bazeliskrc) - Optional, but recommended.
2) [.bazelrc](.bazelrc)
3) [MODULE.bazel](MODULE.bazel)
4) [Root BUILD.bazel](BUILD.bazel)

### .bazeliskrc

It is generally recommended to *NOT* install Bazel with a system package manager (apt, brew etc.) because any major
Bazel update may break your project build in unexpected ways. This project is pinned to Bazel 7, an LTS release
that [guarantees no breaking changes until Dec 2026](https://bazel.build/release).

Instead, set the desired Bazel version in the .bazeliskrc config file, let Bazelisk download Bazel for you and let
things run smoothly. If you ever want to update to a newer Bazel version, just bump up the version number in the
.bazeliskrc config file and test if everything builds. If you encounter any unresolvable error, just revert back to the
previous version, fill a bug or subscribe to an issue and wait until its solved.

### .bazelrc

The .bazelrc configures Bazel itself. At the beginning, it is sensible to ignore this file as much as possible because
of the large number of options that can be configured. As you learn more about Bazel, you will unavoidably customize
this file over time. However,
this project ships with a sensible set of default settings that irons out some kinks, adds some performance tweaks, and
enables some Rust tools. By default, Bazel generates 3 to 4 folders, but these have been re-mapped to sub-folders in *
*target-bzl/** so that it sits right next to the **target/** folder of cargo. If you want a different output folder
structure, you have to edit the "--symlink_prefix" setting in the .bazelrc file.

### MODULE.bazel (ex. WORKSPACE)

One major weakness of the previous WORKSPACE format was that it used a very complicated and verbose syntax, resulting in
very large config files that are hard to maintain over time. There were many more issues, but most boiled down to
maintainability.The new MODULE file indeed simplifies a lot. Conventionally you configure at least three sections for a
Rust project and then some custom tools. Specifically, you need:

1) Bazel build rules from the Bazel Central Registry.
2) Toolchains, at least the Rust toolchain is required
3) Workspace dependencies.

In addition, this project also configures rules_oci to build and publish container images without Docker.

Bazel builds many different programming languages and to support any particular languages
you need to add something similar to a plugin, which are called rules in Bazel. However, rules not only build a specific
programming language, but also provide utils, or just anything that can be configured as a Bazel target. At a bare
minimum, you need rules Rust, register the Rust toolchain (that is the compiler and related tools), and load the crate
universe extension to declare workspace dependencies. A minimal MODULE.bazel is shown below:

```
# B A Z E L  C E N T R A L  R E G I S T R Y
# https://github.com/bazelbuild/rules_rust/releases
bazel_dep(name = "rules_rust", version = "0.46.0")

# T O O L C H A I N S
rust = use_extension("@rules_rust//rust:extensions.bzl", "rust")
rust.toolchain(edition = "2021")
use_repo(rust, "rust_toolchains")

register_toolchains("@rust_toolchains//:all")

# R U S T  C R A T E S
crate = use_extension("@rules_rust//crate_universe:extension.bzl", "crate")
crate.spec(
    package = "anyhow",
    version = "1.0.77",
)
crate.from_specs()
use_repo(crate, "crates")
```

In general, Github pages of most Bazel rules put code snippets on their release pages that
show how to configure Bazel to use them, which is important for toolchains as these usually
require some configuration. Thus project, for example, configures the protobuf toolchain to compile proto files and the
prost toolchain to generate Rust bindings for those proto files.

### Root BUILD.bazel

Every folder that contains code Bazel builds needs a BUILD.bazel file. In practice, though,
the rules_rust generate a fair amount of these files for source sub-folders during the build stage. However, you still
need to configure one BUILD.bazel file for roughly each crate in your project and you need one BUILD.bazel file in the
root folder of your project.
Because Bazel does not have the notion of a Crate, build artifacts are called "targets". In Bazel, everything that is
produced by some rules is a target, a binary, a library, a generated source folder, a tar file, or a container image.
BUILD files either declare these targets, or aggregate them in logical groups. Furthermore, targets can have tags, for
example tests targets can be tagged as "unit-tests", and Bazel can query for all targets with a certain tag before
building or running them.

The root BUILD.bazel file serves two purposes, for once it declares configurations shared across all targets and second
it often aggregates some targets. For example, in this project, one shared setting maps Cargo's "release" flag to
Bazel's "opt" flag. This is used across all binary targets.

Furthermore, the root BUILD.bazel also declares a push target that pushes all container images to a defined container
registry. This uses the multi_run rule that allows to run commands in parallel meaning, if you have multiple container
images, Bazel would push them all at the same time. Note, before you can use external rules, you have to import them
at the top of the file using the load command, for example:

`
load("@rules_multirun//:defs.bzl", "command", "multirun")
`

This imports the command and multirun command from rules_multirun.
The rules_multirun have been declares as dependency in the MODULE.bazel file.

If you ever see an error rule_xyz not defined in .../BUILD.bazel, you most likely didn't import the rule set.

### Target BUILD.bazel

You configure a build target in Bazel in two steps:

1) Import the rules
2) Declare the target according to the rules

In this project, there are two crates:

1) grpc_server
2) proto_bindings

The proto_bindings have been put into a dedicated crate to simplify code sharing.
The rust bindings for the protos are generated by Prost. In Bazel, that means you import the rules for both, proto and
Prost, and configure the targets accordingly. You import the rules by simply calling load:

```
load("@rules_proto//proto:defs.bzl", "proto_library")
load("@rules_rust//proto/prost:defs.bzl", "rust_prost_library")
```

Next, you configure the targets by declaring them according to the rules:

```
proto_library(
    name = "proto_bindings",
    srcs = [
          "proto/helloworld.proto",
    ],
)

rust_prost_library(
    name = "rust_proto",
    proto = ":proto_bindings",
    visibility = ["//visibility:public"],
)
```

Notice, dependencies between targets that are in the same file are declared using
the colon:target syntax, a shorthand that says this target in in the same file.
That way, rust_proto depends on proto_bindings. Furthermore, visibility must be declared because by default Bazel
targets are private. That means, rust_proto is public because it declared its visibility as public, but its dependency
proto_bindings is private because of the default setting in absence of a declared visibility.

Once the target is declared, you can build it by running:

`
bazel build //crates/proto_bindings:rust_proto
`

However, if you want to build all targets, bazel has a shortcut:

`
bazel build //...
`

The double slash refers to the project root and the three dots simply means any target.

The same shortcut also applies to tests:

`
bazel test //...
`

