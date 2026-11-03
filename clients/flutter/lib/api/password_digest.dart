import 'dart:convert';

import 'package:convert/convert.dart';
import 'package:cryptography/cryptography.dart';

typedef PasswordDigest =
    Future<String> Function(String username, String password);

final Argon2id _argon2 = Argon2id(
  memory: 19456,
  parallelism: 1,
  iterations: 2,
  hashLength: 32,
);

Future<String> argon2PasswordDigest(String username, String password) async {
  final SecretKey key = await _argon2.deriveKey(
    secretKey: SecretKey(utf8.encode(password)),
    nonce: utf8.encode('shadowmask-$username'),
  );
  return hex.encode(await key.extractBytes());
}
