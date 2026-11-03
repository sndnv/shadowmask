import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/password_digest.dart';

void main() {
  test('the digest matches what the server derives', () async {
    expect(
      await argon2PasswordDigest('testUser', 'correct horse'),
      '760e3fe1125f79fc2dc3634c38f5b816953a2d76ebe52b8c2b4588f5b803d0e1',
    );
  });

  test('the same password digests differently for another user', () async {
    expect(
      await argon2PasswordDigest('otherUser', 'correct horse'),
      isNot(await argon2PasswordDigest('testUser', 'correct horse')),
    );
  });
}
