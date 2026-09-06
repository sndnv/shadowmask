import 'package:shared_preferences/shared_preferences.dart';

class ServerStore {
  const ServerStore();

  static const String _key = 'shadowmask.server';

  Future<String?> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final String? raw = prefs.getString(_key);
    return raw == null || raw.isEmpty ? null : raw;
  }

  Future<void> save(String address) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setString(_key, address);
  }
}
