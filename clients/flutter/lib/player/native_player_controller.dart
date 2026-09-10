import 'dart:async';
import 'dart:io';
import 'dart:ui' show FlutterView;

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:window_manager/window_manager.dart';

import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';

const Duration kOrientationTimeout = Duration(milliseconds: 1200);
const Duration kOrientationPoll = Duration(milliseconds: 32);

Future<void> preparePlayer() async {
  MediaKit.ensureInitialized();
}

PlayerController makePlayerController(String baseUrl) =>
    NativePlayerController(baseUrl);

class NativePlayerController implements PlayerController {
  NativePlayerController(this._baseUrl)
    : _player = Player(
        configuration: const PlayerConfiguration(logLevel: MPVLogLevel.warn),
      ) {
    _video = VideoController(_player);
    unawaited(
      _video.platform.future.then<void>(
        (_) {},
        onError: (Object error) => _onError(error.toString()),
      ),
    );
    _errors = _player.stream.error.listen(_onError);
    _logs = _player.stream.log.listen(_onLog);
    if (_handheld) {
      _fullscreen.value = true;
      unawaited(_enterStage());
      return;
    }
    windowManager.addListener(_window);
    unawaited(_syncFullscreen());
  }

  static final bool _handheld = Platform.isAndroid || Platform.isIOS;

  final String _baseUrl;
  final Player _player;
  late final VideoController _video;
  late final StreamSubscription<String> _errors;
  late final StreamSubscription<PlayerLog> _logs;
  final List<String> _recentLogs = <String>[];
  late final _FullscreenListener _window = _FullscreenListener(
    (bool value) => _fullscreen.value = value,
  );

  Timer? _timer;
  PlaybackMode _mode = PlaybackMode.direct;
  String? _error;
  bool _seeked = false;
  int _generation = 0;
  double _beforeMute = 1;
  String _decoder = 'unknown';
  String _dropped = 'n/a';
  String _seekable = 'unknown';
  String _start = 'unknown';
  String _fps = 'unknown';
  String _timeout = 'unknown';
  int _networkTimeout = 10;
  int _bufferSeconds = kDefaultBufferSeconds;
  int _bufferBytes = kDefaultBufferBytes;

  final ValueNotifier<PlayerSnapshot> _snapshot = ValueNotifier<PlayerSnapshot>(
    const PlayerSnapshot(),
  );
  final ValueNotifier<bool> _fullscreen = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _mutedByPolicy = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _muted = ValueNotifier<bool>(false);
  final ValueNotifier<double> _volume = ValueNotifier<double>(1);
  final ValueNotifier<bool> _pip = ValueNotifier<bool>(false);

  @override
  ValueListenable<PlayerSnapshot> get snapshot => _snapshot;

  @override
  ValueListenable<bool> get fullscreen => _fullscreen;

  @override
  ValueListenable<bool> get mutedByPolicy => _mutedByPolicy;

  @override
  ValueListenable<bool> get muted => _muted;

  @override
  ValueListenable<double> get volume => _volume;

  @override
  ValueListenable<bool> get pictureInPicture => _pip;

  @override
  bool get supportsPictureInPicture => false;

  @override
  bool get seeksWithinStream => false;

  @override
  Widget get view => Video(
    controller: _video,
    controls: NoVideoControls,
    fit: BoxFit.contain,
    pauseUponEnteringBackgroundMode: false,
  );

  @override
  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
    int positionMs = 0,
    bool autoplay = true,
  }) async {
    _mode = mode;
    _error = null;
    _seeked = false;
    final int generation = ++_generation;
    _startTimer();
    final String url = manifestUrl.startsWith('http')
        ? manifestUrl
        : '$_baseUrl$manifestUrl';
    await _setProperty('start', '0');
    await _setProperty('network-timeout', '$_networkTimeout');
    await _applyBuffer();
    await _player.open(Media(url), play: autoplay && positionMs <= 0);
    if (positionMs > 0) {
      await _resumeAt(positionMs, generation, autoplay);
    }
  }

  Future<void> _resumeAt(int positionMs, int generation, bool autoplay) async {
    final bool ready = await _durationArrives();
    if (generation != _generation) {
      return;
    }
    if (ready && !_seeked) {
      await _player.seek(Duration(milliseconds: positionMs));
    }
    if (autoplay) {
      await _player.play();
    }
  }

  Future<bool> _durationArrives() async {
    if (_player.state.duration > Duration.zero) {
      return true;
    }
    return _player.stream.duration
        .firstWhere((Duration d) => d > Duration.zero)
        .then((_) => true)
        .timeout(const Duration(seconds: 10), onTimeout: () => false);
  }

  @override
  Future<void> detach() async {
    _generation++;
    _timer?.cancel();
    _timer = null;
    _seeked = false;
    _error = null;
    await _player.stop();
    _snapshot.value = const PlayerSnapshot();
  }

  @override
  void play() {
    unmute();
    _player.play();
  }

  @override
  void pause() => _player.pause();

  @override
  void togglePlay() {
    unmute();
    _player.playOrPause();
  }

  @override
  void unmute() {
    if (!_mutedByPolicy.value) {
      return;
    }
    setMuted(false);
  }

  @override
  void setMuted(bool muted) {
    if (muted) {
      _beforeMute = _volume.value;
      _muted.value = true;
      _player.setVolume(0);
      return;
    }
    _muted.value = false;
    _mutedByPolicy.value = false;
    setVolume(_beforeMute > 0 ? _beforeMute : 1);
  }

  @override
  void setVolume(double volume) {
    final double level = volume.clamp(0.0, 1.0);
    _volume.value = level;
    _player.setVolume(level * 100);
    if (level > 0 && _muted.value) {
      _muted.value = false;
      _mutedByPolicy.value = false;
    }
  }

  @override
  Future<void> togglePictureInPicture() async {}

  @override
  void seekTo(int positionMs) {
    unmute();
    _seeked = true;
    _player.seek(Duration(milliseconds: positionMs));
  }

  @override
  void setRate(double rate) => _player.setRate(rate);

  @override
  void setNetworkTimeout(int seconds) {
    _networkTimeout = seconds;
    unawaited(_setProperty('network-timeout', '$seconds'));
  }

  @override
  bool get buffersAhead => true;

  @override
  void setBuffer({required int seconds, required int bytes}) {
    _bufferSeconds = seconds;
    _bufferBytes = bytes;
    unawaited(_applyBuffer());
  }

  Future<void> _applyBuffer() async {
    await _setProperty('cache', 'yes');
    await _setProperty('cache-secs', '$_bufferSeconds');
    await _setProperty('demuxer-readahead-secs', '$_bufferSeconds');
    await _setProperty('demuxer-max-bytes', '$_bufferBytes');
    await _setProperty('demuxer-max-back-bytes', '${_bufferBytes ~/ 4}');
  }

  @override
  void toggleFullscreen() {
    if (_handheld) {
      return;
    }
    unawaited(_toggleFullscreen());
  }

  Future<void> _toggleFullscreen() async {
    await windowManager.setFullScreen(!await windowManager.isFullScreen());
    await _syncFullscreen();
  }

  Future<void> _enterStage() async {
    await SystemChrome.setPreferredOrientations(<DeviceOrientation>[
      DeviceOrientation.landscapeLeft,
      DeviceOrientation.landscapeRight,
    ]);
    await SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky);
  }

  Future<void> _leaveStage() async {
    await SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    await SystemChrome.setPreferredOrientations(const <DeviceOrientation>[]);
    await SystemChrome.setPreferredOrientations(<DeviceOrientation>[
      DeviceOrientation.portraitUp,
    ]);
    await _portraitLands();
    await SystemChrome.setPreferredOrientations(const <DeviceOrientation>[]);
  }

  Future<void> _portraitLands() async {
    final FlutterView? view =
        WidgetsBinding.instance.platformDispatcher.implicitView;
    if (view == null) {
      await Future<void>.delayed(kOrientationTimeout);
      return;
    }
    final Stopwatch waited = Stopwatch()..start();
    while (waited.elapsed < kOrientationTimeout) {
      final Size size = view.physicalSize;
      if (size.height >= size.width) {
        return;
      }
      await Future<void>.delayed(kOrientationPoll);
    }
  }

  Future<void> _syncFullscreen() async {
    _fullscreen.value = await windowManager.isFullScreen();
  }

  @override
  PlayerDiagnostics diagnostics() {
    unawaited(_refreshNativeMetrics());
    final PlayerState state = _player.state;
    return PlayerDiagnostics(<String>[
      'mode: ${_mode.name}',
      'resolution: ${state.width ?? 0}x${state.height ?? 0}',
      'fps: $_fps',
      'buffer: ${(_bufferedAheadMs(state) / 1000).toStringAsFixed(1)}s',
      'target: ${_bufferSeconds}s',
      'decoder: $_decoder',
      'bandwidth: n/a (native)',
      'level: native',
      'frames: $_dropped dropped',
      'seekable: $_seekable  start: $_start',
      'net timeout: $_timeout',
      ..._recentLogs,
    ]);
  }

  @override
  Future<void> dispose() async {
    _timer?.cancel();
    _timer = null;
    if (_handheld) {
      await _leaveStage();
    } else {
      windowManager.removeListener(_window);
    }
    await _errors.cancel();
    await _logs.cancel();
    await _player.dispose();
    _snapshot.dispose();
    _fullscreen.dispose();
    _mutedByPolicy.dispose();
    _muted.dispose();
    _volume.dispose();
    _pip.dispose();
  }

  void _onError(String message) {
    _error = redactStreamTokens(message);
    _tick();
  }

  void _onLog(PlayerLog log) {
    _recentLogs.add(
      redactStreamTokens('${log.level[0]} ${log.prefix}: ${log.text.trim()}'),
    );
    while (_recentLogs.length > 8) {
      _recentLogs.removeAt(0);
    }
    if (log.level == 'fatal' && _error == null) {
      _onError(log.text.trim());
    }
  }

  void _startTimer() {
    _timer ??= Timer.periodic(
      const Duration(milliseconds: 250),
      (_) => _tick(),
    );
  }

  void _tick() {
    final PlayerState state = _player.state;
    if (!_muted.value) {
      _volume.value = (state.volume / 100).clamp(0.0, 1.0);
    }
    _snapshot.value = PlayerSnapshot(
      positionMs: state.position.inMilliseconds,
      durationMs: state.duration.inMilliseconds,
      bufferedAheadMs: _bufferedAheadMs(state),
      playing: state.playing,
      ready: state.duration > Duration.zero,
      buffering: state.buffering,
      bufferingPercent: state.bufferingPercentage,
      ended: state.completed,
      error: _error,
    );
  }

  int _bufferedAheadMs(PlayerState state) {
    final int ahead =
        state.buffer.inMilliseconds - state.position.inMilliseconds;
    return ahead > 0 ? ahead : 0;
  }

  Future<void> _setProperty(String property, String value) async {
    final PlatformPlayer? platform = _player.platform;
    if (platform is! NativePlayer) {
      return;
    }
    try {
      await platform.setProperty(property, value);
    } catch (_) {}
  }

  Future<void> _refreshNativeMetrics() async {
    final PlatformPlayer? platform = _player.platform;
    if (platform is! NativePlayer) {
      return;
    }
    try {
      final String decoder = await platform.getProperty('hwdec-current');
      final String dropped = await platform.getProperty('frame-drop-count');
      final String seekable = await platform.getProperty('seekable');
      final String start = await platform.getProperty('start');
      final String fps = await platform.getProperty('estimated-vf-fps');
      final String container = await platform.getProperty('container-fps');
      final String timeout = await platform.getProperty('network-timeout');
      _timeout = timeout.isEmpty ? 'unknown' : '${timeout}s';
      _decoder = decoder.isEmpty ? 'unknown' : decoder;
      _dropped = dropped.isEmpty ? 'n/a' : dropped;
      _seekable = seekable.isEmpty ? 'unknown' : seekable;
      _start = start.isEmpty ? 'unknown' : start;
      _fps = _framerate(fps, container);
    } catch (_) {}
  }

  String _framerate(String estimated, String container) {
    final double? actual = double.tryParse(estimated);
    final double? declared = double.tryParse(container);
    if (actual == null && declared == null) {
      return 'unknown';
    }
    final String left = actual != null ? actual.toStringAsFixed(1) : 'unknown';
    return declared != null
        ? '$left (source ${declared.toStringAsFixed(1)})'
        : left;
  }
}

class _FullscreenListener with WindowListener {
  _FullscreenListener(this._onChange);

  final ValueChanged<bool> _onChange;

  @override
  void onWindowEnterFullScreen() => _onChange(true);

  @override
  void onWindowLeaveFullScreen() => _onChange(false);
}
