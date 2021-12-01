set -eu
cd "$(dirname $0)"

cargo test

cargo build -p integration-tests-bindings
cd integration_tests
dart test
