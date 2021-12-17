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

*Except* for the [./dart-src](./dart-src) folder all other parts are
licensed under:

- Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

The contents in the [./dart-src](./dart-src) are extracted from the dart language
project and are licensed as defined by the [license file](./dart-src/LICENSE) in that folder.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.