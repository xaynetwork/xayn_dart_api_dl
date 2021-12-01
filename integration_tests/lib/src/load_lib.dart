import 'dart:ffi' show DynamicLibrary;
import 'dart:io' show Platform;

import './genesis.dart' show IntegrationTestsFfi;

/// Opens the platform dependent Rust library.
DynamicLibrary _open() {
  if (Platform.isAndroid) {
    return DynamicLibrary.open('libintegration_tests_bindings.so');
  }
  if (Platform.isIOS) {
    return DynamicLibrary.process();
  }
  if (Platform.isLinux) {
    return DynamicLibrary.open(
        '../target/debug/libintegration_tests_bindings.so');
  }
  if (Platform.isMacOS) {
    return DynamicLibrary.open(
        '../target/debug/libintegration_tests_bindings.dylib');
  }
  throw UnsupportedError('Unsupported platform.');
}

/// The handle to the C-FFI of the Rust library.
final ffi = IntegrationTestsFfi(_open());
