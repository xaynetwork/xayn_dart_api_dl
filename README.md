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

Expect for the [./dart-src](./dart-src) folder all other parts are
licensed as following:

    Copyright 2021 Xayn AG

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0 or LICENSE-APACHE

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.

The contents in the [./dart-src](./dart-src) are extracted from the dart language
project and are licensed as defined by the [license file](./dart-src/LICENSE) in that folder.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.