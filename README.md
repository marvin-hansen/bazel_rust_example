# Bazel Rust Example

Rust example code configured for Bazel in the new Bazelmod format.
For documentation about how to use Rust with Bazelmod, [see this document.](bzlmod.md)
For a general introduction of how setup and and configure Bazel for a project, [see this document.](bzl_intro.md)

Examples:

* [gRPC Client](grpc_client)
* [gRRC Server (with container build)](grpc_server)
* [Proto Prost Bindings](proto_bindings)
* [Cross compilation](hello_cross)
* [MUSL cross compilation (with scratch container build)](musl_container)
* [Tokio REST API (No Cargo, with container build)](rest_tokio)

The project covers quite a bit of groundwork:

* Cargo & Bazel config side by side
* Bazel direct dependencies (not generated from Cargo.toml)
* Builds two crates in a workspace where one depends on the other
* Builds proto bindings for gRPC with prost
* Shares proto definitions between client and server
* Applies compiler optimization & binary size reduction using pass-through options
* Builds and tags OCI images docker-less (See below)
* Cross compile example in the hello-cross crate.

## Acknowledgement

Special thanks to [Daniel Wagner-Hall](https://github.com/illicitonion) for resolving the prost toolchain issue.
If you're feeling this repo adds value, please
consider[ donating to codeyourfuture]( https://codeyourfuture.io/donate/).

## Requirements

* Cargo & Rust
* C compiler (gcc, or clang)
* Bazelisk ([Link to installation](https://bazel.build/install/bazelisk))

Optional:

* Docker (see [container section](#container-build))

## Cross Compilation

The example code is setup to cross compile from the following hosts to the the following targets:

* {linux, x86_64} -> {linux, aarch64}
* {linux, aarch64} -> {linux, x86_64}
* {darwin, x86_64} -> {linux, x86_64}
* {darwin, x86_64} -> {linux, aarch64}
* {darwin, aarch64 (Apple Silicon)} -> {linux, x86_64}
* {darwin, aarch64 (Apple Silicon)} -> {linux, aarch64}

The LLVM setup for cross compilation is the same for MUSL compilation since MUSL technically counts a cross compilation
target hence requires the same LLVM setup. For more information on LLVM cross
compilation, [see the LLVM documentation](https://github.com/bazel-contrib/toolchains_llvm/tree/master?tab=readme-ov-file#sysroots).

To make cross compilation work on your system, please ensure you have a working c/c++ compiler installed (gcc on linux,
clang / Xcode on Mac) on your system to ensure all required libraries are present.

On older linux distros (Ubuntu 16.04) you may encounter an error that a C++ versions before C++ 14 are no longer
supported. In this case, just install gcc version 7 or newer. This is rare corner case, but there are gcc backports for
older distributions, so please upgrade your compiler if you ever see this error.

On Ubuntu 20.04 you may see an error that a shared library called libtinfo.so.5 is missing. In that case, just install
libtinfo via apt-get since its in the official 20.04 repo. The library may have different package names on other
distributions, but it is a well known
issue. [See this SO discussion](https://stackoverflow.com/questions/48674104/clang-error-while-loading-shared-libraries-libtinfo-so-5-cannot-open-shared-o)
for various solutions.

On MacOX, it is usually sufficient to have the Apple Clang compiler installed.
I don't recommend installing the full Xcode package unless you're developing software for an Apple device. Instead, the
Xcode Command Line Tools provide everything you need at a much smaller download size. In most cases, a simple:

`xcode-select --install`

From a terminal triggers the installation process. For details and alternative
options, [read this article on freebootcamp.](https://www.freecodecamp.org/news/install-xcode-command-line-tools/)

Windows is not directly supported, but you can use Linux on Windows with WSL to setup an Ubuntu environment within
Windows. Please refer to
the [official WSL documentation for details.](https://learn.microsoft.com/en-us/windows/wsl/install)

## Container build

Containers are build and published without Docker thanks
to [rules_oci](https://github.com/bazel-contrib/rules_oci/tree/main). This is very favorable for CI builds as it
accelerates container build times significantly. However, if you want to run these containers locally, you need a Docker
installation to pull and run a container image just as you would normally do. Cross compilation binaries don't have
container builds, but these can easily be added following the examples given.

## Bazelmod support

The Bazel project decided to change the main configuration from the previous WORKSPACE format to the
current MODULE (a.k.a bazelmod) format. Since Bazel 7, the Bazelmod format has been set as the new default. This demo
project uses the current bazelmod, but also comes with a working[ WORKSPACE configuration](config/workspace/WORKSPACE).
This may help people who are
trying to convert an existing Bazel project from the previous format to the new Bazelmod configuration format.

## Bazel Commands

### Build

* **Build everything:** `bazel build //...`
* **Build grpc client example:** `bazel build //grpc_client:bin`
* **Build grpc server example:** `bazel build //grpc_server:bin`
* **Build cross compile example:** `bazel build //hello_cross:bin`
* **Build MUSL scratch example:** `bazel build //musl_container:bin`
* **Build tokio rest example:** `bazel build //rest_tokio:bin`

### Optimize

Applies compiler optimization similar to the Rust release mode to binaries. These optimization must be defined in each
binary target. Bazel's `-c opt` flag can be added to any build, test, or run target. However, please be consistent
because a change of that flag triggers a complete rebuild of the target.

* **Optimize all binaries:** `bazel build -c opt //...`
* **Optimize only one example:** `bazel build -c opt //grpc_client:bin`

### Test

* **Test everything:** `bazel test //...`
* **Test only unit tests:** `bazel test //... --test_tag_filters=unit`

### Doc

* **Generate all documentation:** `bazel build //... --build_tag_filters=doc`
* **Build all doc tests:** `bazel build //... --build_tag_filters=doc-test`
* **Run all doc tests:** `bazel test //... --test_tag_filters=doc-test`

### Run

* **Run grpc client example:** `bazel run //grpc_client:bin`
* **Run grpc server example:** `bazel run //grpc_server:bin`
* **Run tokio rest example:**  `bazel run //rest_tokio:bin`

Note, you cannot run the cross compiled binaries unless you copy them to a machine with a matching
architecture i.e. ARM or x86. For the MUSL example, it's best to build the container image, push it to a registry,
and then just use Docker to pull and run it just as you would normally do with any other container.

### Container

Debug

* **Build all container images in debug mode:** `bazel build//:image`
* **Push all debug mode images to container registry:** `command bazel run //:push`

Release (optimized) mode

* **Build all container images in release mode:** `bazel build -c opt //:image`
* **Push all release mode images to container registry:** `command bazel run -c opt //:push`

Note: To enable push to a container registry, you have to configure a container registry for each container. As a
side-effect, you can push different containers to different registries. To do so, please edit the push target in the
following files:

* [gRPC Server/BUILD.bazel](grpc_server/BUILD.bazel)
* [rest_tokio/BUILD.bazel](rest_tokio/BUILD.bazel)

For details how to configure push, please refer to
the [official rules_oci documentation.](https://github.com/bazel-contrib/rules_oci/blob/main/docs/push.md)
