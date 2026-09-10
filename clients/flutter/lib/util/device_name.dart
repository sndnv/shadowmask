import 'package:device_info_plus/device_info_plus.dart';

Future<String?> deviceDisplayName() async {
  try {
    return _pick(await DeviceInfoPlugin().deviceInfo);
  } catch (_) {
    return null;
  }
}

String? _pick(BaseDeviceInfo info) => switch (info) {
  AndroidDeviceInfo(:final String name, :final String model) => _first(<String>[
    name,
    model,
  ]),
  IosDeviceInfo(:final String modelName, :final String name) => _first(<String>[
    modelName,
    name,
  ]),
  MacOsDeviceInfo(:final String computerName) => _first(<String>[computerName]),
  WindowsDeviceInfo(:final String computerName) => _first(<String>[
    computerName,
  ]),
  LinuxDeviceInfo(:final String prettyName) => _first(<String>[prettyName]),
  WebBrowserInfo(:final BrowserName browserName) => _first(<String>[
    browserName.name,
  ]),
  _ => null,
};

String? _first(List<String> candidates) {
  for (final String candidate in candidates) {
    final String trimmed = candidate.trim();
    if (trimmed.isNotEmpty) {
      return trimmed;
    }
  }
  return null;
}
