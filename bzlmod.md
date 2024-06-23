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
    * [Cargo Workspace](#cargo-workspace)
    * [Direct Packages](#direct-packages)
4. [Rust Proto / gRPC](#rust-proto)
5. [Compiler Optimization](#compiler-optimization)
6. [Cross Compilation](#cross-compilation)
7. [MUSL Scratch Container](#musl-scratch-container)

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

### Cargo Workspace

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

## Compiler Optimization

By default, rules_rust do not provide a mechanism to apply various Rust compiler optimization flags you would usually
define in a Cargo.toml file. However, you can define compiler option pass through for each binary target. This takes
just three steps:

1) In your root folder BUILD.bazel, add the following entry:

```Starlark
config_setting(
    name = "release",
    values = {
        "compilation_mode": "opt",
    },
)
```

2) In your binary target, add the opt flags & strip settings prefixed with -C:

```Starlark 
# Build binary
rust_binary(
    name = "bin",
    crate_root = "src/main.rs",
    srcs = glob([
        "src/*.rs",
    ]),
    # Compiler optimization
    rustc_flags = select({
       "//:release": [
            "-Clto",
            "-Ccodegen-units=1",
            "-Cpanic=abort",
            "-Copt-level=3",
            "-Cstrip=symbols",
            ],
        "//conditions:default":
        [
           "-Copt-level=0",
        ],
    }),

    deps = [   ],
    visibility = ["//visibility:public"],
)
```

3) Run bazel build with optimization

`bazel build -c opt //...`

## Cross Compilation

For cross compilation, you have to specify a custom platform to let Bazel know that you are compiling for a different
platform than the default host platform.
The example code is setup to cross compile from the following hosts to the the following targets:

* {linux, x86_64} -> {linux, aarch64}
* {linux, aarch64} -> {linux, x86_64}
* {darwin, x86_64} -> {linux, x86_64}
* {darwin, x86_64} -> {linux, aarch64}
* {darwin, aarch64 (Apple Silicon)} -> {linux, x86_64}
* {darwin, aarch64 (Apple Silicon)} -> {linux, aarch64}

The LLVM setup for cross compilation is the same for MUSL compilation since MUSL technically counts a cross compilation
target hence requires the same LLVM setup.

Also, before you start with the setup, please ensure you have a working c/c++ compiler installed (gcc on linux, clang /
Xcode on Mac) on your system to ensure all required libraries are present.

### Setup

The setup requires three steps, first declare dependencies and toolchains in your MODULE.bazel, second configure LLVM
and Rust for cross compilation, and third the configuration of the cross compilation platforms so you can use it binary
targets.

**Dependencies Configuration**

You add the required rules for cross compilation to your MODULE.bazel as shown below.

```Starlark
# Rules for cross compilation
# https://github.com/bazelbuild/platforms/releases
bazel_dep(name = "platforms", version = "0.0.10")
# https://github.com/bazel-contrib/toolchains_llvm
bazel_dep(name = "toolchains_llvm", version = "1.0.0")
```

**LLVM Configuration**

Next, you have to configure the LLVM toolchain because rules_rust still needs a cpp toolchain for cross compilation and
you have to add the specific platform triplets to the Rust toolchain. Suppose you want to compile a Rust binary that
supports linux on both, X86 and ARM. In that case, you have to setup three LLVM toolchains:

1) LLVM for the host
2) LLVM for X86
3) LLVM for ARM (aarch64)

For the host LLVM, you just specify a LLVM version and then register the toolchain as usual. The target LLVM toolchains,
however, have dependencies on system libraries for the target platform. Therefore, it is requires to download a so
called sysroot that contains a root file system with all those system libraries for the specific target platform. In
this case, you have to use the WORKSPACE.bzlmod file that bridges between the legacy WORKSPACE format and the newer
MODULE.bazel format.
Either crate a new WORKSPACE.bzlmod file if you don't have one yet or open an existing one and add
the following:

```Starlark
###############################################################################
# rule http_archive
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

###############################################################################
# SYSROOT FOR LLVM CROSS COMPILATION
# https://github.com/bazel-contrib/toolchains_llvm/tree/master?tab=readme-ov-file#sysroots
###############################################################################

_BUILD_FILE_CONTENT = """
filegroup(
  name = "{name}",
  srcs = glob(["*/**"]),
  visibility = ["//visibility:public"],
)
"""

http_archive(
    name = "org_chromium_sysroot_linux_x64",
    build_file_content = _BUILD_FILE_CONTENT.format(name = "sysroot"),
    sha256 = "84656a6df544ecef62169cfe3ab6e41bb4346a62d3ba2a045dc5a0a2ecea94a3",
    urls = ["https://commondatastorage.googleapis.com/chrome-linux-sysroot/toolchain/2202c161310ffde63729f29d27fe7bb24a0bc540/debian_stretch_amd64_sysroot.tar.xz"],
)

http_archive(
    name = "org_chromium_sysroot_linux_aarch64",
    build_file_content = _BUILD_FILE_CONTENT.format(name = "sysroot"),
    sha256 = "902d1a40a5fd8c3764a36c8d377af5945a92e3d264c6252855bda4d7ef81d3df",
    urls = ["https://commondatastorage.googleapis.com/chrome-linux-sysroot/toolchain/41a6c8dec4c4304d6509e30cbaf9218dffb4438e/debian_bullseye_arm64_sysroot.tar.xz"],
)
```

Here, we declare to new http downloads that retrieve the sysroot for linux_x64 and linux_aarch64. Note, these are only
sysroots, that means you have to configure LLVM next to use these files. As mentioned earlier, three LLVM toolchains
needs to be configured and to do that, please add the following to your MODULE.bazel

```Starlark
###############################################################################
# L L V M
# https://github.com/bazel-contrib/toolchains_llvm/blob/master/tests/MODULE.bazel
###############################################################################
llvm = use_extension("@toolchains_llvm//toolchain/extensions:llvm.bzl", "llvm")
LLVM_VERSIONS = {
    "": "16.0.0",
}

# Setup for cross compile & MUSL static binary compile.
# Both, cross compilation and MUSL still need a C/C++ toolchain with sysroot.
# https://github.com/bazel-contrib/toolchains_llvm/tree/0d302de75f6ace071ac616fb274481eedcc20e5a?tab=readme-ov-file#sysroots

#
# Host LLVM toolchain.
llvm.toolchain(
    name = "llvm_toolchain",
    llvm_versions = LLVM_VERSIONS,
)
use_repo(llvm, "llvm_toolchain", "llvm_toolchain_llvm")

#
# X86 LLVM Toolchain with sysroot.
# https://github.com/bazel-contrib/toolchains_llvm/blob/master/tests/WORKSPACE.bzlmod
llvm.toolchain(
    name = "llvm_toolchain_x86_with_sysroot",
    llvm_versions = LLVM_VERSIONS,
)

llvm.sysroot(
    name = "llvm_toolchain_x86_with_sysroot",
    targets = ["linux-x86_64"],
    label = "@@org_chromium_sysroot_linux_x64//:sysroot",
)
use_repo(llvm, "llvm_toolchain_x86_with_sysroot")

#
# ARM (aarch64) LLVM Toolchain with sysroot.
# https://github.com/bazelbuild/rules_rust/blob/main/examples/bzlmod/cross_compile/WORKSPACE.bzlmod
llvm.toolchain(
    name = "llvm_toolchain_aarch64_with_sysroot",
    llvm_versions = LLVM_VERSIONS,
)

llvm.sysroot(
    name = "llvm_toolchain_aarch64_with_sysroot",
    targets = ["linux-x86_64"],
    label = "@@org_chromium_sysroot_linux_aarch64//:sysroot",
)
use_repo(llvm, "llvm_toolchain_aarch64_with_sysroot")

# Register all LLVM toolchains
register_toolchains("@llvm_toolchain//:all")
```

For simplicity, all toolchains are pinned to version LLVM 16 because it is one of the few releases that supports the
host (apple-darwin / Ubuntu), and the two targets. For a
complete [list off all LLVM releases and supported platforms, see this list.](https://github.com/bazel-contrib/toolchains_llvm/blob/master/toolchain/internal/llvm_distributions.bzl)
It is possible to pin different targets to different LLVM
versions; [see the documentation for details](https://github.com/bazel-contrib/toolchains_llvm/tree/master?tab=readme-ov-file#per-host-architecture-llvm-version).

**LLVM Troubleshooting**

On older linux distributions (Ubuntu 16.04) you may encounter an error that C++ versions before C++ 14 are no longer
supported. In this case, just install gcc version 7 or newer. This is rare corner case, but there are gcc backports for
older distributions, so please upgrade your compiler if you ever see this error.

On Ubuntu 20.04 you may see an error that a shared library called libtinfo.so.5 is missing. In that case, just install
libtinfo via apt-get since its in the official 20.04 repo. To so, open a terminal and type:

`
apt update && apt install -y libtinfo5
`

The libtinfo5 library may have different package names on other distributions, but it is a well known
issue. [See this SO discussion](https://stackoverflow.com/questions/48674104/clang-error-while-loading-shared-libraries-libtinfo-so-5-cannot-open-shared-o)
for various solutions.

On MacOX, it is sufficient to have the Apple Clang compiler installed.
I don't recommend installing the full Xcode package unless you're developing software for an Apple device. Instead, the
Xcode Command Line Tools provide everything you need at a much smaller download size. In most cases, a simple:

`xcode-select --install`

From a terminal triggers the installation process. For details and alternative
options, [read this article on freebootcamp.](https://www.freecodecamp.org/news/install-xcode-command-line-tools/)

Windows is not directly supported, but you can use Linux on Windows with WSL to setup an Ubuntu environment within
Windows. Please refer to
the [official WSL documentation for details.](https://learn.microsoft.com/en-us/windows/wsl/install)

**Rust Toolchain Configuration**

The Rust toolchain only need to know the additional platform triplets to download the matching toolchains. To do so, add
or or modify your MODULE.bazel with the following entry:

```Starlark
# Rust toolchain
RUST_EDITION = "2021"
RUST_VERSION = "1.79.0"

rust = use_extension("@rules_rust//rust:extensions.bzl", "rust")
rust.toolchain(
    edition = RUST_EDITION,
    versions = [RUST_VERSION],
    extra_target_triples = [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
    ],
)
use_repo(rust, "rust_toolchains")
register_toolchains("@rust_toolchains//:all")
```

You find the exact platform triplets in
the [Rust platform support documentation](https://doc.rust-lang.org/nightly/rustc/platform-support.html).
Next, you have to configure the target platform.

**Platform Configuration**

Once the dependencies are loaded, create an empty BUILD file to define the cross compilation toolchain targets.
As mentioned earlier, it is best practice to put all custom rules, toolchains, and platform into one folder.
Suppose you have the empty BUILD file in the following path:

`build/platforms/BUILD.bazel`

Then you add the following content to the BUILD file:

```Starlark
package(default_visibility = ["//visibility:public"])

platform(
    name = "linux-aarch64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:aarch64",
    ],
)

platform(
    name = "linux-x86_64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ],
)
```

The default visibility at the top of the file means that all targets in this BUILD file will be public by default, which
is sensible because cross-compilation targets are usually used across the entire project.

It is important to recognize that the platform rules use the constraint values to map those constraints to the target
triplets of the Rust toolchain. If you somehow see errors that says some crate couldn't be found with triple xyz, then
one of two things happened.

Either you forgot to add a triple to the Rust toolchain. Unfortunately, the error message
doesn't always tell you the correct triple that is missing. However, in that case you have to double check if for each
specified platform a corresponding Rust extra_target_triples has been added. If one is missing, add it and the error
goes away.

A second source of error is if the platform declaration contains a typo, for example,
cpu:arch64 instead of cpu:aarch64. You have to be meticulous in the platform declaration to make everything work
smoothly.

With the platform configuration out of the way, you are free to configure your binary targets for the specified
platforms.

### Usage

Suppose you have a simple hello world that is defined in a single main.rs file. Conventionally, you declare a minimum
binary target as shown below.

```Starlark
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "hello_world_host",
    srcs = ["src/main.rs"],
    deps = [],
)
```

Bazel compiles this target to the same platform as the host. To cross-compile the same source file to a different
platform, you simply add one of the platforms previously declared, as shown below.

```Starlark
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "hello_world_x86_64",
    srcs = ["src/main.rs"],
    platform = "//build/platforms:linux-x86_64",
    deps = [],
)

rust_binary(
    name = "hello_world_aarch64",
    srcs = ["src/main.rs"],
    platform = "//build/platforms:linux-aarch64",
    deps = [],
)
```

You than cross-compile by calling the target.

`bazel build //hello_cross:hello_world_x86_64`

You may have to make the target public when see an access error.

However, when you build for multiple targets, it is sensible to group all of them in a filegroup.

```Starlark
filegroup(
    name = "bin",
    srcs = [
        ":hello_world_host",
        ":hello_world_x86_64",
        ":hello_world_aarch64",
    ],
    visibility = ["//visibility:public"],
)
```

Then you build for all platforms by calling the filegroup target:

`bazel build //hello_cross:bin`

## MUSL Scratch Container

Rust increasingly becomes a popular choice for building microservice for web application.
In this context, security and performance are important considerations. Because containerization has
become the de-facto deployment option, container security starts with choosing a minimal base image.
Golang established the concept of scratch images, a basically empty container image that only holds a statically
compiled binary. In Golang, this works because C compatibility is optional and the Go standard library can be compiled
statically without any calls to an underlying std c lib i.e. glibc on Linux.

In Rust, however, because of its deep interoperability with C, a few more steps and workarounds are required to build a
static binary packaged in a scratch container.

### Setup

The initial setup is similar to the previous cross compilation example. However, in addition to LLVM and platform
support, we also add the MUSL toolchain, and a bunch of other rules used throughout this example,

```Starlark
# https://github.com/bazelbuild/bazel-skylib/releases/
bazel_dep(name = "bazel_skylib", version = "1.7.1")
# https://github.com/aspect-build/bazel-lib/releases
bazel_dep(name = "aspect_bazel_lib", version = "2.7.7")
# https://github.com/bazel-contrib/rules_oci/releases
bazel_dep(name = "rules_oci", version = "1.7.6")
# https://github.com/bazelbuild/rules_pkg/releases
bazel_dep(name = "rules_pkg", version = "0.10.1")

# MUSL toolchain
# https://github.com/bazel-contrib/musl-toolchain/releases
bazel_dep(name = "toolchains_musl", version = "0.1.16", dev_dependency = True)
# Rules for cross compilation
# https://github.com/bazelbuild/platforms/releases
bazel_dep(name = "platforms", version = "0.0.10")
# https://github.com/bazel-contrib/toolchains_llvm
bazel_dep(name = "toolchains_llvm", version = "1.0.0")
```

Then, you have to configure LLVM and the RUST toolchain.
For the LLVM toolchain setup, please refer to
the [LLVM section in the cross compilation documentation](#cross-compilation) as the setup is identical for MUSL.
Once the LLVM setup is complete, you have to add the MUSL triplets to the Rust toolchain configuration
so that it downloads the additional toolchains for MUSL targets.

```Starlark
rust = use_extension("@rules_rust//rust:extensions.bzl", "rust")
rust.toolchain(
    edition = RUST_EDITION,
    versions = [RUST_VERSION],
    extra_target_triples = [
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
    ],
)
use_repo(rust, "rust_toolchains")
register_toolchains("@rust_toolchains//:all")
```

Before the MUSL platform can be configured, we need to add a custom linker configuration to redirect the linker to the
MUSL linker. To do so, add an empty BUILD file in the following path:

`build/linker/BUILD.bazel`

Then add the following content to configure the linker for MUSL.

```Starlark
package(default_visibility = ["//visibility:public"])

constraint_setting(
    name = "linker",
    default_constraint_value = ":unknown",
)

constraint_value(
    name = "musl",
    constraint_setting = ":linker",
)

# Default linker for anyone not setting the linker to `musl`.
# You shouldn't ever need to set this value manually.
constraint_value(
    name = "unknown",
    constraint_setting = ":linker",
)
```

Then, you edit your platform configuration, assumed to be in the following path:

`build/platforms/BUILD.bazel`

Add the following entries to configure MUSL:

```Starlark
package(default_visibility = ["//visibility:public"])

platform(
    name = "linux_x86_64_musl",
    constraint_values = [
        "@//build/linker:musl",
        "@platforms//cpu:x86_64",
        "@platforms//os:linux",
    ],
)

platform(
    name = "linux_arm64_musl",
    constraint_values = [
        "@//build/linker:musl",
        "@platforms//cpu:arm64",
        "@platforms//os:linux",
    ],
)
```

Notice that the path of the linker is set to `//build/linker` so if you chose a different folder,
you have to update that path accordingly. At this point, you might be tempted to just add the platform to a binary
target similar to the the cross compilation example. This might work when the binary is the final delivery.
However, when a scratch container is the deliverable, a few more steps are required.

### Custom Memory allocator.

There is a long-standing multi threading performance issue in MUSL's default memory allocator
that causes a
significant [performance drop of at least 10x or more compared to the default memory allocator in Linux.](https://www.linkedin.com/pulse/testing-alternative-c-memory-allocators-pt-2-musl-mystery-gomes)
The real source of the performance degradation is thread contention is in the malloc implementation of musl.
One known workaround is
to [patch the memory allocator in place](https://www.tweag.io/blog/2023-08-10-rust-static-link-with-mimalloc/)
using a rather lesser known assembly tool.
A unique alternative Rust offers is the global_allocator trait that, once overwritten with
a custom allocator, simply replaces the memory allocator Rust uses.

There are like three alternative memory allocators available for Rust,

* [jemallocator](https://crates.io/crates/jemallocator)
* [mimalloc](https://lib.rs/crates/mimalloc)
* [snmalloc](https://lib.rs/crates/snmalloc-rs)

Notice, Jemalloc has
a[ known segfault issue when you target embedded platforms](https://github.com/clux/muslrust/issues/142#issuecomment-2152638811)
where the memory page size varies.
Specifically, if you compile with Jemalloc on an Apple Silicon for usage on a Raspberry Pi,
Jemalloc may segfault on the Raspberry Pi due to different memory page sizes because
Jemalloc bakes the memory page size into the final binary.
Mimalloc doesn't have this problem, and has performance comparable to Jemalloc.
Therefore, for embedded devices, Mimalloc is the best choice.

However, on x86 (Intel / AMD), this issue does not exists, and any of the memory allocators listed above works just
fine.[A benchmarks show that both](https://github.com/rust-lang/rust-analyzer/issues/1441), Jemalloc and Mimalloc
demonstrate comparable performance so for X86, you can pick any of the two.

For this example, I chose Jemalloc from the Free/NetBSD distro because
it is a robust and battle tested memory allocators out there that still delivers excellent performance under heavy
multi-threading workload.

Make sure jemallocator is declared a dependency in your MODULE.bazelmod file:

```Starlark
###############################################################################
# R U S T  C R A T E S
###############################################################################
crate = use_extension("@rules_rust//crate_universe:extension.bzl", "crate")
#
# Custom Memory Allocator
crate.spec(package = "jemallocator", version = "0.5.4")
# ... other crate dependencies.
```

Next, you add a new memory allocator by adding the following lines to your main.rs file:

```Rust
use jemallocator::Jemalloc;

// Jemalloc overwrites the default memory allocator. This fixes a performance issue in the MUSL.
// https://www.linkedin.com/pulse/testing-alternative-c-memory-allocators-pt-2-musl-mystery-gomes
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() {
  // ...
}
```

Rust produces inflated debug symbols, therefore, is sensible to add [compiler optimization ](#compiler-optimization) to
build a small and fast binary. To so so, add the following to your binary target.

```Starlark
# Build binary
rust_binary(
    name = "bin",
    crate_root = "src/main.rs",
    srcs = glob([
        "src/*/*.rs",
        "src/*.rs",
    ]),
    # Compiler optimization
    rustc_flags = select({
       "//:release": [
            "-Clto",
            "-Ccodegen-units=1",
            "-Cpanic=abort",
            "-Copt-level=3",
            "-Cstrip=symbols",
            ],
        "//conditions:default":
        [
           "-Copt-level=0",
        ],
    }),

    deps = [
        # Jemallocator Memory Allocator fixes a concurrency performance issue in MUSL
        # https://www.linkedin.com/pulse/testing-alternative-c-memory-allocators-pt-2-musl-mystery-gomes
        "@crates//:jemallocator",
        # External dependencies
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:tokio",
        # ...
    ],
    tags = ["service", "musl-tokio"],
    visibility = ["//visibility:public"],
)
```

Make sure jemallocator is declared a dependency in your MODULE.bazelmod file:

```Starlark
###############################################################################
# R U S T  C R A T E S
###############################################################################
crate = use_extension("@rules_rust//crate_universe:extension.bzl", "crate")
#
# Custom Memory Allocator
crate.spec(package = "jemallocator", version = "0.5.4")
# ... other crate dependencies.
```

Also, for the compiler optimization to take effect, make sure you have the release mode mapping in your root BUILD file:

```Starlark
config_setting(
    name = "release",
    values = {
        "compilation_mode": "opt",
    },
)
```

At this point, you want to run a full build and check for any errors.

`bazel build //...`

Also run a full release build to double check that the optimization settings work:

`bazel build -c opt //...`

### Scratch image

The new rules_oci build container images in Bazel without Docker. Before you build a container,
you have to add base image.
Previous examples have used the lightweight Distroless containers,
but since the binary has been compiled statically, all you need is a scratch image.
To declare a scratch image, add the following to your MODULE.bazel file:

```Starlark
###############################################################################
#  O C I  B A S E  I M A G E
###############################################################################
oci = use_extension("@rules_oci//oci:extensions.bzl", "oci")
#
# https://hub.docker.com/r/hansenmarvin/rust-scratch/tags
oci.pull(
    name = "scratch",
    digest = "sha256:c6d1c2b62a454d6c5606645b5adfa026516e3aa9213a6f7648b8e9b3cc520f76",
    image = "index.docker.io/hansenmarvin/rust-scratch",
    platforms = ["linux/amd64", "linux/arm64"],
)
use_repo(oci, "scratch")
```

In this example, a custom scratch image is used. You can inspect the Docker build file on
[its public repository](https://github.com/marvin-hansen/rust-scratch). As you can the in
the [Dockerfile](https://github.com/marvin-hansen/rust-scratch/blob/main/Dockerfile),
SSL certificates are copied from the base image to ensure encrypted connections
work as expected. However, this is also a prime example of how an
attacker could sneak in bogus certificates via sloppy supply chain security.

Therefore, it is generally recommended to build and use your own scratch image
instead of relying on unknown third parties.

The process to build a multi_arch scratch image to hold your statically linked binary takes a few steps:

1) Compress the Rust binary as tar
2) Build container image from the tar
3) Build a multi_arch image for the designated platform(s)
4) Generate a oci_image_index
5) Tag the final multi_arch image

Building a multi_arch image, however, requires a platform transition. Without much ado,
just create new empty BUILD file in a folder containing all your custom BAZEL rules and toolchains, say:

`build/transition.bzl`

And then add the following content:

```Starlark
"a rule transitioning an oci_image to multiple platforms"

def _multiarch_transition(settings, attr):
    return [
        {"//command_line_option:platforms": str(platform)}
        for platform in attr.platforms
    ]

multiarch_transition = transition(
    implementation = _multiarch_transition,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)

def _impl(ctx):
    return DefaultInfo(files = depset(ctx.files.image))

multi_arch = rule(
    implementation = _impl,
    attrs = {
        "image": attr.label(cfg = multiarch_transition),
        "platforms": attr.label_list(),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
```

Next, you need a custom rule to tag your container. In a hermetic build, you can't rely on timestamps because these
changes regardless of whether the build has changed. Strictly speaking, timestamps as tags could be made possible in
Bazel, but it is commonly discouraged. Also, immutable container tags are increasingly encouraged to prevent accidental
pulling of a different image that has the same tag as the previous one but contains breaking changes relative to the
previous image. Instead, you want unique tags that only change when the underlying artifact has changed. Turned out,
rules_oci already generates a sha256 for each OCI image so a simple tag rule would be to extract this has and trim to,
say 7 characters and use this short hash as unique and immutable tag.

To crate this rule, crate new file, say,

`build/container.bzl`

Then add the following rule:

```Starlark
def _build_sha265_tag_impl(ctx):

    # Both the input and output files are specified by the BUILD file.
    in_file = ctx.file.input
    out_file = ctx.outputs.output

    # No need to return anything telling Bazel to build `out_file` when
    # building this target -- It's implied because the output is declared
    # as an attribute rather than with `declare_file()`.
    ctx.actions.run_shell(
        inputs = [in_file],
        outputs = [out_file],
        arguments = [in_file.path, out_file.path],
        command = "sed -n 's/.*sha256:\\([[:alnum:]]\\{7\\}\\).*/\\1/p' < \"$1\" > \"$2\"",
    )

build_sha265_tag = rule(
    doc = "Extracts a 7 characters long short hash from the image digest.",
    implementation = _build_sha265_tag_impl,
    attrs = {
        "image": attr.label(
            allow_single_file = True,
            mandatory = True,
        ),
        "input": attr.label(
            allow_single_file = True,
            mandatory = True,
            doc = "The image digest file. Usually called image.json.sha256",
        ),
        "output": attr.output(
            doc = "The generated tag file. Usually named _tag.txt"
        ),
    },
)

```

Then, you import this rule together with the multi_arch and some others rules
to build a container for your binary target.

```Starlark
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_doc", "rust_doc_test")
# OCI Container Rules
load("@rules_pkg//pkg:tar.bzl", "pkg_tar")
load("@rules_oci//oci:defs.bzl", "oci_image", "oci_push",  "oci_image_index")
# Custom container macro
load("//:build/container.bzl", "build_sha265_tag")
# Custom platform transition macro
load("//:build/transition.bzl", "multi_arch")
```

Remember, the steps to build a multi_arch image are the following:

1) Compress the Rust binary as tar
2) Build container image from the tar
3) Build a multi_arch image for the designated platform(s)
4) Generate a oci_image_index
5) Tag the final multi_arch image

Let's start with the first three steps. Add the following to your binary target:

```Starlark
# Compress binary to a layer using pkg_tar
pkg_tar(
    name = "tar",
    srcs = [":bin"],
)

# Build container image
# https://github.com/bazel-contrib/rules_oci/blob/main/docs/image.md
oci_image(
    name = "image",
    base = "@scratch",
    tars = [":tar"],
    entrypoint = ["/bin"],
    exposed_ports = ["3232"],
    visibility = ["//visibility:public"],
)

# Build multi-arch images
multi_arch(
    name = "multi_arch_images",
    image = ":image",
    platforms = [
        "//build/platforms:linux_x86_64_musl",
        "//build/platforms:linux_arm64_musl",
    ],
)
```

A few notes:

1) Make sure the tar package references the binary.
2) Make sure the container image exposes the exact same ports as the binary uses.
3) The base image, scratch, of the container.
4) Make sure the path and labels used of the platforms in the multi_arch match exactly the folder structure you have
   defined in the previous steps.

Next, lets add the remaining two steps plus a declaration to push the final image to a container registry.

```Starlark
# Build a container image index.
oci_image_index(
    name = "image_index",
    images = [
        ":multi_arch_images",
    ],
    visibility = ["//visibility:public"],
)

# Build an unique and immutable image tag based on the image SHA265 digest.
build_sha265_tag(
    name = "tags",
    image = ":image_index",
    input = "image.json.sha256",
    output = "_tag.txt",
)

# Publish multi-arch with image index to registry
oci_push(
    name = "push",
    image = ":image_index",
    repository = "my.registry.com/musl",
    remote_tags = ":tags",
    visibility = ["//visibility:public"],
)
```

Important details:

1) The oci_image_index always references the multi_arch rule even if you only compile for one platform.
2) The oci_image_index is public because that target is what you call when you build the container without publishing
   it.
3) The build_sha265_tag rule uses the image.json.sha256 file from the original image. This is on purpose because the
   sha265 is only generated for images during the build, but not for the index file.
4) The oci_push references the image_index to ensure a multi arch image will be published.
5) oci_push is public because that is the target you call to publish you container.

For details of how to configure a container registry,
please [consult the official documentation.](https://github.com/bazel-contrib/rules_oci/blob/main/docs/push.md)

### Custom Container Macro

The scratch image configuration feels quite verbose and this configuration becomes quickly tedious
when you build a large number of containers that roughly follow the same blueprint and only differ
by a handful of parameters such as exposed ports, the specific platform(s) and similar.
In that case, it is advisable to write a custom macro that reduces the boilerplate code to a
bare minimum.

In short, open or add a file in

`build/container.bzl`

And add the following content:

```Starlark
load("@rules_pkg//pkg:tar.bzl", "pkg_tar")
load("@rules_oci//oci:defs.bzl", "oci_image", "oci_image_index")
load("//:build/transition.bzl", "multi_arch")

# Build a Bazel Macro
# https://belov.nz/posts/bazel-rules-macros/

def build_multi_arch_image(
    name,
    base,
    srcs,
    exposed_ports = [],
    platforms = [],
    visibility=None
    ):

    # https://codilime.com/blog/bazel-build-system-build-containerized-applications/
    entry_point = "bin"
    layer_name = "tar_layer"

    # Compress binary to a layer using pkg_tar
    pkg_tar(
        name = layer_name,
        srcs = srcs,
    )

    # Build container image
    oci_image(
        name = "image",
        base = base,
        tars = [layer_name],
        entrypoint = ["/{}".format(entry_point)],
        exposed_ports = exposed_ports,
    )

    # Build multi-arch images for the provided platforms
    multi_arch(
        name = "multi_arch_images",
        image = ":image",
        platforms = platforms,
    )
    
    # Build a container image index.
    oci_image_index(
        name = name,
        images = [
            ":multi_arch_images",
        ],
        visibility = visibility,
    )
```

This macro rule turns the previous boilerplate into a template you can import and use
to build your custom MUSL scratch image for your binary targets. This usually simplifies maintenance
because the bulk of changes can be made in the macro instead of each targets. Note, if you want
to enforce a specific base image, say for security reasons, you can declare it in the macro instead of using a
parameter. You still need the tag rule from before because the tags apply to the push rule.
With the new macro in place, you import the macro and the tag rule in your target BUILD:

```Starlark
# Normal Rust rules 
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_doc", "rust_doc_test")
# OCI Push rule
load("@rules_oci//oci:defs.bzl",  "oci_push")
# Custom container macro
load("//:build/container.bzl", "build_multi_arch_image", "build_sha265_tag")

```

With these imports in place, you then use the rules as shown below:

```Starlark
# Build binary
rust_binary(
    name = "bin",
    # ...
)

# 1) Build musl multi arch container image
build_multi_arch_image(
    name = "image_index",
    base = "@scratch",
    srcs = [":bin"],
    exposed_ports = ["7070", "8080"],
    platforms = [ "//build/platforms:linux_x86_64_musl",],
    visibility = ["//visibility:public"],
)

# 2) Tag image based on the image SHA265 digest.
build_sha265_tag(
    name = "remote_tag",
    image = ":image_index",
    input = "image.json.sha256",
    output = "_tag.txt",
)

# 3) Publish multi-arch with image index to registry
oci_push(
    name = "push",
    image = ":image_index",
    repository = "my.registry.com/musl",
    remote_tags = ":tags",
    visibility = ["//visibility:public"],
)
```

With the macro, building a multi-arch container is a three step process, build, tag, and push.
As stated before, this approach only makes sense when you have either a larger number
of similar container builds or you have to enforce a number of (security) polices across the entire project.