import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/user/self_user.dart';

SelfUser _user(String role, [String? account]) =>
    SelfUser.fromJson(<String, dynamic>{
      'id': 'u1',
      'username': 'someone',
      'role': role,
      'account_role': ?account,
    });

void main() {
  test('the account role is read beside the session role', () {
    final SelfUser user = _user('player', 'admin');

    expect(user.role, UserRole.player);
    expect(user.accountRole, UserRole.admin);
  });

  test('an older server without the account role still parses', () {
    expect(_user('user').accountRole, isNull);
  });

  test('version work is for admins and devices linked to an admin', () {
    expect(_user('admin', 'admin').worksOnVersions, isTrue);
    expect(_user('player', 'admin').worksOnVersions, isTrue);
    expect(_user('player', 'user').worksOnVersions, isFalse);
    expect(_user('player').worksOnVersions, isFalse);
    expect(_user('user', 'user').worksOnVersions, isFalse);
    expect(_user('user', 'admin').worksOnVersions, isFalse);
  });
}
