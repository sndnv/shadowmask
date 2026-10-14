import 'package:shared_preferences/shared_preferences.dart';

typedef PlayOnSettings = ({bool enabled, int? port});

class PlayOnStore {
  const PlayOnStore();

  static const String _enabledKey = 'shadowmask.playOn.enabled';
  static const String _portKey = 'shadowmask.playOn.port';

  Future<PlayOnSettings> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    return (
      enabled: prefs.getBool(_enabledKey) ?? false,
      port: prefs.getInt(_portKey),
    );
  }

  Future<void> saveEnabled(bool enabled) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_enabledKey, enabled);
  }

  Future<void> savePort(int? port) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    if (port == null) {
      await prefs.remove(_portKey);
      return;
    }
    await prefs.setInt(_portKey, port);
  }
}
