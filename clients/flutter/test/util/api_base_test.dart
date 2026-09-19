import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/api_base.dart';

void main() {
  test('a dart-define beats the env file', () {
    expect(
      apiBaseFrom(define: 'http://defined', env: 'http://from-file'),
      'http://defined',
      reason:
          'web/assets/.env is a gitignored local file every developer is told '
          'to create, so it must not shadow an address passed on the command '
          'line',
    );
  });

  test('the env file is used when nothing was defined', () {
    expect(
      apiBaseFrom(define: '', env: 'http://from-file'),
      'http://from-file',
    );
  });

  test('neither source leaves the caller to its own default', () {
    expect(apiBaseFrom(define: '', env: null), isNull);
  });

  test('an empty env value counts as unset', () {
    expect(apiBaseFrom(define: '', env: ''), isNull);
  });
}
