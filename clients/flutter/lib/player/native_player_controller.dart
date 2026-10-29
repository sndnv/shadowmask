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
import 'package:shadowmask/player/mpv_tracks.dart';
import 'package:shadowmask/player/playback_log.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/state/player_tracks.dart';
import 'package:shadowmask/player/watches/segment_watch.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_size.dart';

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
        onError: (Object error) => _onOutputFailure(error.toString()),
      ),
    );
    _mpvErrors = _player.stream.error.listen(_onError);
    _logs = _player.stream.log.listen(_onLog);
    _audio = _player.stream.audioParams.listen(
      (_) => unawaited(_readOutputChannels()),
    );
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
  late final StreamSubscription<String> _mpvErrors;
  late final StreamSubscription<PlayerLog> _logs;
  late final StreamSubscription<AudioParams> _audio;
  int? _outputChannels;
  final StreamController<String> _errors = StreamController<String>.broadcast();
  final StreamController<void> _lost = StreamController<void>.broadcast();
  final PlaybackLog _log = PlaybackLog();
  late final _FullscreenListener _window = _FullscreenListener(
    (bool value) => _fullscreen.value = value,
  );

  Timer? _timer;
  PlaybackMode _mode = PlaybackMode.direct;
  int _generation = 0;
  Completer<void> _replaced = Completer<void>();
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
  String? _outputFailure;
  SegmentWatch _segments = kSegmentWatchStart;

  final ValueNotifier<PlayerSnapshot> _snapshot = ValueNotifier<PlayerSnapshot>(
    const PlayerSnapshot(),
  );
  final ValueNotifier<bool> _fullscreen = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _mutedByPolicy = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _muted = ValueNotifier<bool>(false);
  final ValueNotifier<double> _volume = ValueNotifier<double>(1);
  final ValueNotifier<bool> _pip = ValueNotifier<bool>(false);
  final ValueNotifier<SubtitleSize> _subtitleSize = ValueNotifier<SubtitleSize>(
    SubtitleSize.deviceDefault,
  );
  static final TextStyle _subtitleStyle =
      const SubtitleViewConfiguration().style;

  @override
  ValueListenable<PlayerSnapshot> get snapshot => _snapshot;

  @override
  ValueListenable<bool> get fullscreen => _fullscreen;

  @override
  Stream<String> get errors => _errors.stream;

  @override
  Stream<void> get lost => _lost.stream;

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
  bool seeksWithin(String? container) => container != kFmp4Segments;

  @override
  List<String> get subtitleDeliveries => const <String>[
    kEmbeddedDelivery,
    kImageSubtitles,
    kExternalSubtitles,
  ];

  @override
  bool get selectsAudioTracks => true;

  @override
  bool get mixesSound => true;

  @override
  int? get outputChannels => _outputChannels;

  @override
  Future<void> tuneSound({
    required bool normalize,
    required DialogueLift dialogue,
  }) async {
    for (final MapEntry<String, String> option in mpvSound(
      normalize: normalize,
      dialogue: dialogue,
    ).entries) {
      await _setProperty(option.key, option.value);
    }
  }

  Future<void> _readOutputChannels() async {
    final PlatformPlayer? platform = _player.platform;
    if (platform is! NativePlayer) {
      return;
    }
    try {
      _outputChannels = int.tryParse(
        await platform.getProperty('audio-out-params/channel-count'),
      );
    } catch (_) {}
  }

  @override
  Widget get view => ValueListenableBuilder<SubtitleSize>(
    valueListenable: _subtitleSize,
    builder: (BuildContext context, SubtitleSize size, Widget? _) =>
        LayoutBuilder(
          builder: (BuildContext context, BoxConstraints c) => Video(
            controller: _video,
            controls: NoVideoControls,
            fit: BoxFit.contain,
            pauseUponEnteringBackgroundMode: false,
            subtitleViewConfiguration: SubtitleViewConfiguration(
              style: _subtitleStyle.copyWith(
                fontSize: subtitleFontSize(c.maxHeight, size),
              ),
              textScaler: TextScaler.noScaling,
            ),
          ),
        ),
  );

  @override
  void setSubtitleSize(SubtitleSize size) => _subtitleSize.value = size;

  @override
  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
    required PlayerTracks tracks,
    int positionMs = 0,
    bool autoplay = true,
  }) async {
    _mode = mode;
    _replace();
    final int generation = _generation;
    final Future<void> replaced = _replaced.future;
    _startTimer();
    await _setProperty('start', 'none');
    await _setProperty('sid', tracks.rendition ? 'auto' : 'no');
    await _setProperty('sub-visibility', tracks.subtitleImage ? 'yes' : 'no');
    await _setProperty('network-timeout', '$_networkTimeout');
    await _applyBuffer();
    _current(generation);
    await _player.open(Media(_absolute(manifestUrl)), play: false);
    if (_outputFailure case final String failure) {
      _onError(failure);
    }
    await Future.any(<Future<void>>[_lengthKnown(), replaced]);
    _current(generation);
    _segments = kSegmentWatchArmed;
    await _select(tracks);
    _current(generation);
    if (positionMs > 0) {
      await _player.seek(Duration(milliseconds: positionMs));
      _current(generation);
    }
    if (autoplay) {
      await _player.play();
    }
  }

  String _absolute(String url) =>
      url.startsWith('http') ? url : '$_baseUrl$url';

  Future<void> _select(PlayerTracks tracks) async {
    final PlatformPlayer? platform = _player.platform;
    if (platform is! NativePlayer) {
      return;
    }
    try {
      final List<MpvTrack> listed = await _listTracks(platform);
      if (tracks.subtitleIndex case final int index) {
        await _selectTrack(platform, listed, 'sub', 'sid', index);
      }
      if (tracks.subtitleUrl case final String url) {
        await platform.command(<String>['sub-add', _absolute(url), 'select']);
      }
      if (tracks.audioIndex case final int index) {
        await _selectTrack(platform, listed, 'audio', 'aid', index);
      }
    } catch (error) {
      _log.add(redactStreamTokens('selecting tracks failed: $error'));
    }
  }

  Future<List<MpvTrack>> _listTracks(NativePlayer platform) async {
    final int count =
        int.tryParse(await platform.getProperty('track-list/count')) ?? 0;
    final List<MpvTrack> listed = <MpvTrack>[];
    for (int i = 0; i < count; i++) {
      final MpvTrack? track = mpvTrackOf(
        id: await platform.getProperty('track-list/$i/id'),
        type: await platform.getProperty('track-list/$i/type'),
        ffIndex: await platform.getProperty('track-list/$i/ff-index'),
        external: await platform.getProperty('track-list/$i/external'),
      );
      if (track != null) {
        listed.add(track);
      }
    }
    return listed;
  }

  Future<void> _selectTrack(
    NativePlayer platform,
    List<MpvTrack> listed,
    String type,
    String property,
    int ffIndex,
  ) async {
    final int? id = mpvTrackId(listed, type, ffIndex);
    if (id == null) {
      _log.add('no $type track for stream $ffIndex');
      return;
    }
    await platform.setProperty(property, '$id');
  }

  void _replace() {
    _generation++;
    _segments = kSegmentWatchStart;
    if (!_replaced.isCompleted) {
      _replaced.complete();
    }
    _replaced = Completer<void>();
  }

  void _current(int generation) {
    if (generation != _generation) {
      throw StateError('superseded by another stream');
    }
  }

  Future<void> _lengthKnown() => _player.state.duration > Duration.zero
      ? Future<void>.value()
      : _player.stream.duration.firstWhere((Duration d) => d > Duration.zero);

  @override
  Future<void> detach() async {
    _replace();
    _timer?.cancel();
    _timer = null;
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
      ..._log.recent(),
    ]);
  }

  @override
  List<String> log() => _log.lines;

  @override
  Future<void> dispose() async {
    _replace();
    _timer?.cancel();
    _timer = null;
    if (_handheld) {
      await _leaveStage();
    } else {
      windowManager.removeListener(_window);
    }
    await _mpvErrors.cancel();
    await _logs.cancel();
    await _audio.cancel();
    await _player.dispose();
    await _errors.close();
    await _lost.close();
    _snapshot.dispose();
    _fullscreen.dispose();
    _mutedByPolicy.dispose();
    _muted.dispose();
    _volume.dispose();
    _pip.dispose();
    _subtitleSize.dispose();
  }

  void _onError(String message) {
    if (!_errors.isClosed) {
      _errors.add(redactStreamTokens(message));
    }
  }

  void _onOutputFailure(String message) {
    _outputFailure = message;
    _log.add(redactStreamTokens('video output failed: $message'));
    _onError(message);
  }

  void _onLog(PlayerLog log) {
    final String text = log.text.trim();
    _log.add(redactStreamTokens('${log.level[0]} ${log.prefix}: $text'));
    if (log.level == 'fatal') {
      _onError(text);
    }
    if (failedSegment(text) case final int segment) {
      _onFailedSegment(segment);
    }
  }

  void _onFailedSegment(int segment) {
    _segments = segmentFailed(_segments, segment);
    if (!segmentsLost(_segments)) {
      return;
    }
    _segments = kSegmentWatchArmed;
    _log.add(segmentsLostLine(segment));
    if (!_lost.isClosed) {
      _lost.add(null);
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
