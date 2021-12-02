/// Support for doing something awesome.
///
/// More dartdocs go here.
library integration_tests;

import 'dart:ffi' show NativeApi, NativePort;
import 'dart:isolate' show ReceivePort, SendPort;
import 'package:integration_tests/src/load_lib.dart' show ffi;

/// ffi bool as dart bool
///
/// ffigen might depending on factors outside of it's version
/// sometimes generate a bool returning function an sometimes an
/// integer returning function.
bool ffiBool(Object val) {
  if (val is int) {
    assert(val == 1 || val == 0);
    return val == 1;
  }
  assert(val is bool);
  return val as bool;
}

Future<void> initialize() async {
  if (ffiBool(ffi.initialize(NativeApi.initializeApiDLData))) {
    await Commander._getInstance();
    return;
  }
  throw Exception('failed to initialize');
}

class Commander {
  static Commander? _instance;

  final SendPort _chan;

  Commander._(this._chan);

  static Future<Commander> _getInstance() async {
    final instance = Commander._instance;
    if (instance != null) {
      return instance;
    } else {
      final port = ReceivePort();
      if (!ffiBool(ffi.setup_cmd_handler(port.sendPort.nativePort))) {
        throw Exception('failed to setup');
      }
      final dynamic chan = await port.first;
      final newInstance = Commander._(chan as SendPort);
      Commander._instance = newInstance;
      return newInstance;
    }
  }

  /// Same constraints as sending through send port.
  ///
  static Future<dynamic> sendCmd(
    String name, [
    List<Object?> params = const [],
  ]) async {
    final self = await _getInstance();
    final port = ReceivePort();
    final allParams = <Object?>[port.sendPort, name];
    allParams.addAll(params);
    self._chan.send(allParams);
    return port.first;
  }
}
