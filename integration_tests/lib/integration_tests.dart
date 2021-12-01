/// Support for doing something awesome.
///
/// More dartdocs go here.
library integration_tests;

import 'dart:ffi' show NativeApi, NativePort;
import 'dart:isolate' show ReceivePort, SendPort;
import 'src/load_lib.dart' show ffi;

Future<void> initialize() async {
  if (ffi.initialize(NativeApi.initializeApiDLData) != 0) {
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
      if (ffi.setup_cmd_handler(port.sendPort.nativePort) == 0) {
        throw Exception('failed to setup');
      }
      dynamic chan = await port.first;
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
    return await port.first;
  }
}
