import 'dart:ui';

import 'package:shared_preferences/shared_preferences.dart';

class WindowStore {
  const WindowStore();

  static const String _key = 'shadowmask.window';

  Future<Rect?> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final List<String>? raw = prefs.getStringList(_key);
    if (raw == null || raw.length != 4) {
      return null;
    }
    final List<double?> values = raw.map(double.tryParse).toList();
    if (values.any((double? value) => value == null)) {
      return null;
    }
    return Rect.fromLTWH(values[0]!, values[1]!, values[2]!, values[3]!);
  }

  Future<void> save(Rect bounds) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setStringList(_key, <String>[
      bounds.left.toString(),
      bounds.top.toString(),
      bounds.width.toString(),
      bounds.height.toString(),
    ]);
  }
}
