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
