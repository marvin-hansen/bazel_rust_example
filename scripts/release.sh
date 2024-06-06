# bin/sh
set -o errexit
set -o nounset
set -o pipefail

echo "=========================="
echo "Format all code with cargo fmt"
echo "=========================="
command command cargo fmt --all
echo "Done"

echo "=========================="
echo "Compile project in release mode"
echo "=========================="
command bazel build  -c opt //...

echo "=========================="
echo "Run all tests"
echo "=========================="
command bazel test -c opt //...

echo "=========================="
echo "Build all docs and run doc tests"
echo "=========================="
command bazel build  -c opt //:doc

echo "=========================="
echo "Build all container images in release mode"
echo "=========================="
command bazel build -c opt //:image

echo "=========================="
echo  "Container image publication DISABLED."
echo "=========================="

echo  " * Configure registry in crates/grpc_server/BUILD.bazel"
echo  " * Uncomment last command in scripts/release.sh"
echo  " * Re-run make release"
# Pushes all tagged images to registry
# command bazel run -c opt //:push

