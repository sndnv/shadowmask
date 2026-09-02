import 'package:shared_preferences/shared_preferences.dart';

import 'package:shadowmask/theme/app_theme_variant.dart';

class ThemeStore {
  const ThemeStore();

  static const String _key = 'shadowmask.theme';
  static const String _contrastKey = 'shadowmask.highContrast';

  Future<AppThemeVariant> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final String? raw = prefs.getString(_key);
    return AppThemeVariant.values.firstWhere(
      (AppThemeVariant v) => v.name == raw,
      orElse: () => AppThemeVariant.dark,
    );
  }

  Future<void> save(AppThemeVariant variant) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setString(_key, variant.name);
  }

  Future<bool?> loadHighContrast() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    return prefs.getBool(_contrastKey);
  }

  Future<void> saveHighContrast(bool value) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_contrastKey, value);
  }
}
