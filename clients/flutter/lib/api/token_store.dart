import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import 'package:shadowmask/model/auth/auth_tokens.dart';

class TokenStore {
  static const String _key = 'shadowmask.tokens';

  Future<AuthTokens?> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final String? raw = prefs.getString(_key);
    if (raw == null) {
      return null;
    }
    try {
      return AuthTokens.fromJson(jsonDecode(raw) as Map<String, dynamic>);
    } catch (_) {
      return null;
    }
  }

  Future<void> save(AuthTokens tokens) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setString(_key, jsonEncode(tokens.toJson()));
  }

  Future<void> clear() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.remove(_key);
  }
}
