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

import 'dart:isolate' show TransferableTypedData;
import 'dart:typed_data' show Uint8List;

import 'package:integration_tests/integration_tests.dart'
    show Commander, initialize;
import 'package:test/test.dart';

Future<void> main() async {
  setUpAll(() async {
    await initialize();
  });

  test('hy works', () async {
    final dynamic res = await Commander.sendCmd('hy');
    expect(res, equals('hy hy ho'));
  });

  test('add works', () async {
    dynamic res = await Commander.sendCmd('add', [12, 44]);
    expect(res, equals(56));

    res = await Commander.sendCmd('add', [-16, 16]);
    expect(res, equals(0));

    res = await Commander.sendCmd('add', [12, 34]);
    expect(res, equals('Error: Adder was shutdown'));
  });

  test('dart recv external typed data', () async {
    final dynamic res = await Commander.sendCmd('send etd');
    expect(res, equals([1, 12, 33]));
    expect(
      res.runtimeType.toString(),
      equals(Uint8List(0).runtimeType.toString()),
    );
  });

  test('send TransferTypedData to rust', () async {
    final data = TransferableTypedData.fromList([
      Uint8List.fromList([33, 44, 12, 123])
    ]);
    await Commander.sendCmd('recv ttd', [data]);
  });

  test('panic catching works', () async {
    final dynamic res = await Commander.sendCmd('panic');
    expect(res, equals('IT IS A PANIC'));
  });
}
