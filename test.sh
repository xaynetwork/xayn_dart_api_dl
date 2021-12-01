set -eu
cd "$(dirname $0)"

cargo test

cargo build
cd integration_tests
dart test
