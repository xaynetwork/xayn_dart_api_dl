# Dart Api Dl

Rust bindings to the `dart_api_dl.h` interface (which is used with the mechanism
provided by the `dart:ffi` package).

## dart-api-dl-sys

The sys bindings to `dart_api_dl.h`.

## dart-api-dl

Safer bindings  around `dart-api-dl-sys`, including:

- safe auto-dropping creation of CObjects for the various CObject variants
- thread safe api initialization
- safe ways to create native ports including a safe abstraction over the
  native ports message handlers
- support for externally typed data to avoid unnecessary copies

## License

See the [NOTICE](NOTICE) file.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the this project by you, shall be licensed as Apache-2.0, without any additional
terms or conditions.
