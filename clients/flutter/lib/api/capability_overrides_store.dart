import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import 'package:shadowmask/model/session/capability_overrides.dart';

class CapabilityOverridesStore {
  const CapabilityOverridesStore();

  static const String _key = 'shadowmask.capabilities';

  Future<CapabilityOverrides> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final String? raw = prefs.getString(_key);
    if (raw == null || raw.isEmpty) {
      return const CapabilityOverrides();
    }
    try {
      final Object? decoded = jsonDecode(raw);
      return decoded is Map<String, dynamic>
          ? CapabilityOverrides.fromJson(decoded)
          : const CapabilityOverrides();
    } catch (_) {
      return const CapabilityOverrides();
    }
  }

  Future<void> save(CapabilityOverrides overrides) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    if (overrides.isEmpty) {
      await prefs.remove(_key);
      return;
    }
    await prefs.setString(_key, jsonEncode(overrides.toJson()));
  }
}
