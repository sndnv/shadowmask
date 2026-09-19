import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/server/server_info.dart';
import 'package:shadowmask/view/profile_version.dart';

void main() {
  test('a server on the profile this build negotiates is not warned about', () {
    expect(profileVersionMatches(kExpectedProfileVersion), isTrue);
  });

  test('either side being newer is worth saying', () {
    expect(profileVersionMatches(kExpectedProfileVersion + 1), isFalse);
    expect(profileVersionMatches(kExpectedProfileVersion - 1), isFalse);
  });

  test('a server that reports nothing is taken as speaking ours', () {
    expect(profileVersionMatches(const ServerInfo().profileVersion), isTrue);
  });

  test(
    'the warning names both versions, since either could be the old one',
    () {
      final String message = Strings.errorProfileVersion(3, 1);

      expect(message, contains('3'));
      expect(message, contains('1'));
      expect(message, contains('older'));
    },
  );
}
