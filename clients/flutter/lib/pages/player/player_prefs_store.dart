import 'package:shared_preferences/shared_preferences.dart';

typedef PlayerPrefs = ({
  double volume,
  bool muted,
  bool remaining,
  bool wide,
  int autoplaySeconds,
  int networkTimeoutSeconds,
});

const List<int> kAutoplayDelays = <int>[0, 5, 10, 15, 30];
const List<int> kNetworkTimeouts = <int>[10, 30, 60, 90];

const PlayerPrefs kDefaultPlayerPrefs = (
  volume: 1.0,
  muted: false,
  remaining: false,
  wide: false,
  autoplaySeconds: 10,
  networkTimeoutSeconds: 10,
);

class PlayerPrefsStore {
  const PlayerPrefsStore();

  static const String _volumeKey = 'shadowmask.player.volume';
  static const String _mutedKey = 'shadowmask.player.muted';
  static const String _remainingKey = 'shadowmask.player.remaining';
  static const String _wideKey = 'shadowmask.player.wide';
  static const String _autoplayKey = 'shadowmask.player.autoplay';
  static const String _timeoutKey = 'shadowmask.player.network_timeout';

  Future<PlayerPrefs> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final double volume = (prefs.getDouble(_volumeKey) ?? 1).clamp(0.0, 1.0);
    final int autoplay =
        prefs.getInt(_autoplayKey) ?? kDefaultPlayerPrefs.autoplaySeconds;
    final int timeout =
        prefs.getInt(_timeoutKey) ?? kDefaultPlayerPrefs.networkTimeoutSeconds;
    return (
      volume: volume,
      muted: prefs.getBool(_mutedKey) ?? false,
      remaining: prefs.getBool(_remainingKey) ?? false,
      wide: prefs.getBool(_wideKey) ?? false,
      autoplaySeconds: kAutoplayDelays.contains(autoplay)
          ? autoplay
          : kDefaultPlayerPrefs.autoplaySeconds,
      networkTimeoutSeconds: kNetworkTimeouts.contains(timeout)
          ? timeout
          : kDefaultPlayerPrefs.networkTimeoutSeconds,
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

  Future<void> saveAutoplaySeconds(int seconds) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_autoplayKey, seconds);
  }

  Future<void> saveNetworkTimeoutSeconds(int seconds) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_timeoutKey, seconds);
  }
}
