import 'package:shared_preferences/shared_preferences.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/subtitle_size.dart';

const int kAutoplayOff = 0;
const int kAutoplayImmediately = -1;
const List<int> kAutoplayDelays = <int>[
  kAutoplayOff,
  kAutoplayImmediately,
  5,
  10,
  15,
  30,
];

String autoplayLabel(int value) => switch (value) {
  kAutoplayOff => Strings.playerAutoplayOff,
  kAutoplayImmediately => Strings.playerAutoplayImmediately,
  _ => Strings.playerAutoplayDelay(value),
};
const List<int> kNetworkTimeouts = <int>[10, 30, 60, 90];
const List<int> kBufferTargets = <int>[30, 60, 120, 300, 600];
const List<int> kBufferSizes = <int>[
  64 * 1024 * 1024,
  128 * 1024 * 1024,
  256 * 1024 * 1024,
  512 * 1024 * 1024,
  1024 * 1024 * 1024,
];

class PlayerPrefsStore {
  const PlayerPrefsStore();

  static const LocalSettings _defaults = LocalSettings();

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
  static const String _deliveryKey = 'shadowmask.player.delivery';
  static const String _subtitleSizeKey = 'shadowmask.player.subtitle_size';

  Future<LocalSettings> load() async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    final double volume = (prefs.getDouble(_volumeKey) ?? _defaults.volume)
        .clamp(0.0, 1.0);
    final int autoplay = prefs.getInt(_autoplayKey) ?? _defaults.autoplayNextS;
    final int timeout = prefs.getInt(_timeoutKey) ?? _defaults.networkTimeoutS;
    final int buffer = prefs.getInt(_bufferKey) ?? _defaults.bufferS;
    final int bytes = prefs.getInt(_bufferBytesKey) ?? _defaults.bufferBytes;
    final String? delivery = prefs.getString(_deliveryKey);
    final String? subtitleSize = prefs.getString(_subtitleSizeKey);
    return LocalSettings(
      volume: volume,
      muted: prefs.getBool(_mutedKey) ?? _defaults.muted,
      remaining: prefs.getBool(_remainingKey) ?? _defaults.remaining,
      diagnostics: prefs.getBool(_diagnosticsKey) ?? _defaults.diagnostics,
      wide: prefs.getBool(_wideKey) ?? _defaults.wide,
      autoplayNextS: kAutoplayDelays.contains(autoplay)
          ? autoplay
          : _defaults.autoplayNextS,
      networkTimeoutS: kNetworkTimeouts.contains(timeout)
          ? timeout
          : _defaults.networkTimeoutS,
      bufferS: kBufferTargets.contains(buffer) ? buffer : _defaults.bufferS,
      bufferBytes: kBufferSizes.contains(bytes) ? bytes : _defaults.bufferBytes,
      waitForBuffer: prefs.getBool(_waitKey) ?? _defaults.waitForBuffer,
      defaultDelivery: delivery == null
          ? _defaults.defaultDelivery
          : DeliveryPreference.fromWire(delivery),
      subtitleSize: subtitleSize == null
          ? _defaults.subtitleSize
          : SubtitleSize.fromWire(subtitleSize),
    );
  }

  Future<void> saveChanged(LocalSettings was, LocalSettings now) async {
    await Future.wait(<Future<void>>[
      if (now.volume != was.volume) saveVolume(now.volume),
      if (now.muted != was.muted) saveMuted(now.muted),
      if (now.remaining != was.remaining) saveRemaining(now.remaining),
      if (now.diagnostics != was.diagnostics) saveDiagnostics(now.diagnostics),
      if (now.autoplayNextS != was.autoplayNextS)
        saveAutoplaySeconds(now.autoplayNextS),
      if (now.networkTimeoutS != was.networkTimeoutS)
        saveNetworkTimeoutSeconds(now.networkTimeoutS),
      if (now.bufferS != was.bufferS) saveBufferSeconds(now.bufferS),
      if (now.bufferBytes != was.bufferBytes) saveBufferBytes(now.bufferBytes),
      if (now.waitForBuffer != was.waitForBuffer)
        saveWaitForBuffer(now.waitForBuffer),
      if (now.subtitleSize != was.subtitleSize)
        saveSubtitleSize(now.subtitleSize),
    ]);
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

  Future<void> saveDelivery(DeliveryPreference delivery) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setString(_deliveryKey, delivery.wire);
  }

  Future<void> saveSubtitleSize(SubtitleSize size) async {
    final SharedPreferences prefs = await SharedPreferences.getInstance();
    await prefs.setString(_subtitleSizeKey, size.wire);
  }
}
