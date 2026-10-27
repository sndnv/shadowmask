import 'package:shared_preferences/shared_preferences.dart';

import 'package:shadowmask/api/child_mode_scope.dart';

class ChildModeStore {
  const ChildModeStore();

  static const String _key = 'shadowmask.childMode.age';

  Future<int?> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    return offeredChildAge(prefs.getInt(_key));
  }

  Future<void> save(int? age) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    if (age == null) {
      await prefs.remove(_key);
      return;
    }
    await prefs.setInt(_key, age);
  }
}
