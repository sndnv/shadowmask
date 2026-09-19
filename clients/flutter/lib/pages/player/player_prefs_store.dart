import 'package:shared_preferences/shared_preferences.dart';

import 'package:shadowmask/player/player_controller.dart';

typedef PlayerPrefs = ({
  double volume,
  bool muted,
  bool remaining,
  bool diagnostics,
  bool wide,
  int autoplaySeconds,
  int networkTimeoutSeconds,
  int bufferSeconds,
  int bufferBytes,
  bool waitForBuffer,
});

const List<int> kAutoplayDelays = <int>[0, 5, 10, 15, 30];
const List<int> kNetworkTimeouts = <int>[10, 30, 60, 90];
const List<int> kBufferTargets = <int>[30, 60, 120, 300, 600];
const List<int> kBufferSizes = <int>[
  64 * 1024 * 1024,
  128 * 1024 * 1024,
  256 * 1024 * 1024,
  512 * 1024 * 1024,
  1024 * 1024 * 1024,
];

const PlayerPrefs kDefaultPlayerPrefs = (
  volume: 1.0,
  muted: false,
  remaining: false,
  diagnostics: false,
  wide: false,
  autoplaySeconds: 10,
  networkTimeoutSeconds: kDefaultNetworkTimeoutSeconds,
  bufferSeconds: kDefaultBufferSeconds,
  bufferBytes: kDefaultBufferBytes,
  waitForBuffer: false,
);

class PlayerPrefsStore {
  const PlayerPrefsStore();

  static const String _volumeKey = 'shadowmask.player.volume';
  static const String _mutedKey = 'shadowmask.player.muted';
  static const String _remainingKey = 'shadowmask.player.remaining';
  static const String _diagnosticsKey = 'shadowmask.player.diagnostics';
  static const String _wideKey = 'shadowmask.player.wide';
  static const String _autoplayKey = 'shadowmask.player.autoplay';
  static const String _timeoutKey = 'shadowmask.player.network_timeout';
  static const String _bufferKey = 'shadowmask.player.buffer_seconds';
  static const String _bufferBytesKey = 'shadowmask.player.buffer_bytes';
  static const String _waitKey = 'shadowmask.player.wait_for_buffer';

  Future<PlayerPrefs> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final double volume = (prefs.getDouble(_volumeKey) ?? 1).clamp(0.0, 1.0);
    final int autoplay =
        prefs.getInt(_autoplayKey) ?? kDefaultPlayerPrefs.autoplaySeconds;
    final int timeout =
        prefs.getInt(_timeoutKey) ?? kDefaultPlayerPrefs.networkTimeoutSeconds;
    final int buffer =
        prefs.getInt(_bufferKey) ?? kDefaultPlayerPrefs.bufferSeconds;
    final int bytes =
        prefs.getInt(_bufferBytesKey) ?? kDefaultPlayerPrefs.bufferBytes;
    return (
      volume: volume,
      muted: prefs.getBool(_mutedKey) ?? false,
      remaining: prefs.getBool(_remainingKey) ?? false,
      diagnostics: prefs.getBool(_diagnosticsKey) ?? false,
      wide: prefs.getBool(_wideKey) ?? false,
      autoplaySeconds: kAutoplayDelays.contains(autoplay)
          ? autoplay
          : kDefaultPlayerPrefs.autoplaySeconds,
      networkTimeoutSeconds: kNetworkTimeouts.contains(timeout)
          ? timeout
          : kDefaultPlayerPrefs.networkTimeoutSeconds,
      bufferSeconds: kBufferTargets.contains(buffer)
          ? buffer
          : kDefaultPlayerPrefs.bufferSeconds,
      bufferBytes: kBufferSizes.contains(bytes)
          ? bytes
          : kDefaultPlayerPrefs.bufferBytes,
      waitForBuffer:
          prefs.getBool(_waitKey) ?? kDefaultPlayerPrefs.waitForBuffer,
    );
  }

  Future<void> saveMuted(bool muted) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_mutedKey, muted);
  }

  Future<void> saveWide(bool wide) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_wideKey, wide);
  }

  Future<void> saveVolume(double volume) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setDouble(_volumeKey, volume.clamp(0.0, 1.0));
  }

  Future<void> saveRemaining(bool remaining) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_remainingKey, remaining);
  }

  Future<void> saveDiagnostics(bool on) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_diagnosticsKey, on);
  }

  Future<void> saveAutoplaySeconds(int seconds) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_autoplayKey, seconds);
  }

  Future<void> saveNetworkTimeoutSeconds(int seconds) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_timeoutKey, seconds);
  }

  Future<void> saveBufferSeconds(int seconds) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_bufferKey, seconds);
  }

  Future<void> saveBufferBytes(int bytes) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_bufferBytesKey, bytes);
  }

  Future<void> saveWaitForBuffer(bool wait) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setBool(_waitKey, wait);
  }
}
