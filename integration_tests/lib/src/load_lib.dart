// Copyright 2021 Xayn AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import 'dart:ffi' show DynamicLibrary;
import 'dart:io' show Platform;

import 'package:integration_tests/src/genesis.ffigen.dart'
    show IntegrationTestsFfi;

/// Opens the platform dependent Rust library.
DynamicLibrary _open() {
  if (Platform.isLinux) {
    return DynamicLibrary.open(
      '../target/debug/libintegration_tests_bindings.so',
    );
  }
  if (Platform.isMacOS) {
    return DynamicLibrary.open(
      '../target/debug/libintegration_tests_bindings.dylib',
    );
  }
  throw UnsupportedError('Unsupported platform.');
}

/// The handle to the C-FFI of the Rust library.
final ffi = IntegrationTestsFfi(_open());
