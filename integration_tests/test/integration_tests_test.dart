import 'package:integration_tests/integration_tests.dart'
    show Commander, initialize;
import 'package:test/test.dart';

void main() async {
  setUpAll(() async {
    await initialize();
  });

  test('hy works', () async {
    dynamic res = await Commander.sendCmd('hy');
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
    dynamic res = await Commander.sendCmd('send etd');
    expect(res, equals([1, 12, 33]));
  });
}
