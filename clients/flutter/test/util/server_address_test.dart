import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/server_address.dart';

void main() {
  test('an address typed without a scheme is assumed to be plain http', () {
    expect(
      normalizeServerAddress('192.168.1.10:8080'),
      'http://192.168.1.10:8080',
    );
  });

  test('a trailing slash is dropped', () {
    expect(
      normalizeServerAddress('https://media.example.com/'),
      'https://media.example.com',
      reason: 'the base is concatenated with paths that already start with /',
    );
  });

  test('a base path is kept, only its trailing slash goes', () {
    expect(
      normalizeServerAddress('https://example.com/shadowmask//'),
      'https://example.com/shadowmask',
    );
  });

  test('surrounding whitespace is not part of the address', () {
    expect(normalizeServerAddress('  http://host:8080  '), 'http://host:8080');
  });

  test('an explicit https address survives untouched', () {
    expect(normalizeServerAddress('https://host'), 'https://host');
  });

  test(
    'nothing, a scheme we cannot speak, or a hostless address is refused',
    () {
      expect(normalizeServerAddress(''), isNull);
      expect(normalizeServerAddress('   '), isNull);
      expect(normalizeServerAddress('ftp://host'), isNull);
      expect(normalizeServerAddress('http://'), isNull);
    },
  );
}
