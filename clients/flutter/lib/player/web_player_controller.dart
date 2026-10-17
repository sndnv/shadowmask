import 'dart:async';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:ui_web' as ui_web;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:web/web.dart' as web;

import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/player/playback_log.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/view/subtitle_size.dart';

@JS('Hls')
extension type _Hls._(JSObject _) implements JSObject {
  external factory _Hls([_HlsConfig config]);
  external static bool isSupported();
  @JS('Events')
  external static _HlsEvents get events;
  external void loadSource(String url);
  external void attachMedia(web.HTMLVideoElement media);
  external void on(String event, JSFunction callback);
  external void destroy();
  external double get bandwidthEstimate;
  external int get currentLevel;
  external bool get autoLevelEnabled;
  external JSArray<_HlsLevel> get levels;
  external _HlsConfig get config;
}

extension type _HlsConfig._(JSObject _) implements JSObject {
  external factory _HlsConfig({
    int maxBufferLength,
    int maxMaxBufferLength,
    int maxBufferSize,
    int manifestLoadingTimeOut,
    int levelLoadingTimeOut,
    int fragLoadingTimeOut,
  });
  external set maxBufferLength(int value);
  external set maxMaxBufferLength(int value);
  external set maxBufferSize(int value);
  external set manifestLoadingTimeOut(int value);
  external set levelLoadingTimeOut(int value);
  external set fragLoadingTimeOut(int value);
}

extension type _HlsEvents._(JSObject _) implements JSObject {
  @JS('MANIFEST_PARSED')
  external String get manifestParsed;
  @JS('ERROR')
  external String get error;
  @JS('BUFFER_EOS')
  external String get bufferEos;
}

extension type _HlsErrorData._(JSObject _) implements JSObject {
  external bool get fatal;
  external String get type;
  external String get details;
}

extension type _HlsLevel._(JSObject _) implements JSObject {
  external int get height;
  external int get bitrate;
}

int _viewCounter = 0;

Future<void> preparePlayer() async {}

PlayerController makePlayerController(String baseUrl) =>
    WebPlayerController(baseUrl);

class WebPlayerController implements PlayerController {
  WebPlayerController(this._baseUrl)
    : _viewType = 'sm-player-${_viewCounter++}' {
    _video = web.document.createElement('video') as web.HTMLVideoElement
      ..id = _viewType
      ..controls = false
      ..setAttribute('playsinline', '')
      ..style.width = '100%'
      ..style.height = '100%'
      ..style.backgroundColor = 'black';
    ui_web.platformViewRegistry.registerViewFactory(
      _viewType,
      (int _) => _video,
    );
    _onFullscreenChange = ((web.Event _) {
      _fullscreen.value = web.document.fullscreenElement != null;
    }).toJS;
    web.document.addEventListener('fullscreenchange', _onFullscreenChange);
    _video.addEventListener(
      'enterpictureinpicture',
      ((web.Event _) => _pip.value = true).toJS,
    );
    _video.addEventListener(
      'leavepictureinpicture',
      ((web.Event _) => _pip.value = false).toJS,
    );
    _video.addEventListener(
      'volumechange',
      ((web.Event _) {
        _muted.value = _video.muted;
        _volume.value = _video.volume;
      }).toJS,
    );
    _video.addEventListener('play', ((web.Event _) => _holdScreen(true)).toJS);
    _video.addEventListener(
      'pause',
      ((web.Event _) {
        _holdScreen(false);
        _event('pause');
      }).toJS,
    );
    _video.addEventListener(
      'ended',
      ((web.Event _) {
        _holdScreen(false);
        _event('ended');
      }).toJS,
    );
    for (final String name in <String>[
      'waiting',
      'stalled',
      'durationchange',
    ]) {
      _video.addEventListener(name, ((web.Event _) => _event(name)).toJS);
    }
    _video.addEventListener('error', ((web.Event _) => _onVideoError()).toJS);
    _onVisibilityChange = ((web.Event _) {
      if (_awake && web.document.visibilityState == 'visible') {
        _acquireScreen();
      }
    }).toJS;
    web.document.addEventListener('visibilitychange', _onVisibilityChange);
  }

  final String _baseUrl;
  final String _viewType;
  late final web.HTMLVideoElement _video;
  late final JSFunction _onFullscreenChange;
  late final JSFunction _onVisibilityChange;
  web.WakeLockSentinel? _screen;
  bool _awake = false;
  _Hls? _hls;
  JSFunction? _onReady;
  Completer<void>? _attaching;
  int _bufferSeconds = kDefaultBufferSeconds;
  int _bufferBytes = kDefaultBufferBytes;
  int _networkTimeoutSeconds = kDefaultNetworkTimeoutSeconds;
  Timer? _timer;
  double _rate = 1;
  PlaybackMode _mode = PlaybackMode.direct;
  final StreamController<String> _errors = StreamController<String>.broadcast();
  final PlaybackLog _log = PlaybackLog();
  web.HTMLStyleElement? _cueStyle;

  final ValueNotifier<PlayerSnapshot> _snapshot = ValueNotifier<PlayerSnapshot>(
    const PlayerSnapshot(),
  );
  final ValueNotifier<bool> _fullscreen = ValueNotifier<bool>(
    web.document.fullscreenElement != null,
  );
  final ValueNotifier<bool> _mutedByPolicy = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _muted = ValueNotifier<bool>(false);
  final ValueNotifier<double> _volume = ValueNotifier<double>(1);
  final ValueNotifier<bool> _pip = ValueNotifier<bool>(false);

  @override
  ValueListenable<PlayerSnapshot> get snapshot => _snapshot;

  @override
  ValueListenable<bool> get fullscreen => _fullscreen;

  @override
  Stream<String> get errors => _errors.stream;

  @override
  ValueListenable<bool> get mutedByPolicy => _mutedByPolicy;

  @override
  ValueListenable<bool> get muted => _muted;

  @override
  ValueListenable<double> get volume => _volume;

  @override
  ValueListenable<bool> get pictureInPicture => _pip;

  @override
  bool get supportsPictureInPicture => web.document.pictureInPictureEnabled;

  @override
  bool seeksWithin(String? container) => true;

  @override
  Widget get view => HtmlElementView(viewType: _viewType);

  @override
  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
    required bool subtitles,
    int positionMs = 0,
    bool autoplay = true,
  }) {
    _mode = mode;
    _dropReady();
    _settle('superseded by another stream');
    _teardownHls();
    _startTimer();
    final Completer<void> attached = Completer<void>();
    _attaching = attached;
    _load(manifestUrl, positionMs, autoplay);
    return attached.future;
  }

  void _load(String manifestUrl, int positionMs, bool autoplay) {
    final String url = manifestUrl.startsWith('http')
        ? manifestUrl
        : '$_baseUrl$manifestUrl';
    if (_mode == PlaybackMode.direct) {
      _video.src = url;
      _seekOnReady(positionMs, autoplay);
      return;
    }
    if (_Hls.isSupported()) {
      final _Hls hls = _Hls(
        _HlsConfig(
          maxBufferLength: _bufferSeconds,
          maxMaxBufferLength: _bufferSeconds,
          maxBufferSize: _bufferBytes,
          manifestLoadingTimeOut: _networkTimeoutSeconds * 1000,
          levelLoadingTimeOut: _networkTimeoutSeconds * 1000,
          fragLoadingTimeOut: _networkTimeoutSeconds * 1000,
        ),
      );
      _hls = hls;
      hls.loadSource(url);
      hls.attachMedia(_video);
      hls.on(
        _Hls.events.manifestParsed,
        (() => _ready(positionMs, autoplay)).toJS,
      );
      hls.on(
        _Hls.events.error,
        ((JSAny? _, _HlsErrorData data) => _report(
          '${data.type}: ${data.details}',
          fatal: data.fatal,
        )).toJS,
      );
      hls.on(
        _Hls.events.bufferEos,
        (() => _event('hls end of stream buffered')).toJS,
      );
      return;
    }
    if (_video.canPlayType('application/vnd.apple.mpegurl').isNotEmpty) {
      _video.src = url;
      _seekOnReady(positionMs, autoplay);
      return;
    }
    _report('This browser cannot play HLS video.', fatal: true);
  }

  @override
  void setSubtitleSize(SubtitleSize size) {
    final String? fontSize = cueFontSize(size);
    web.HTMLStyleElement? style = _cueStyle;
    if (style == null) {
      style = web.document.createElement('style') as web.HTMLStyleElement;
      web.document.head?.appendChild(style);
      _cueStyle = style;
    }
    style.textContent = fontSize == null
        ? ''
        : '#$_viewType::cue { font-size: $fontSize; }';
  }

  void _onVideoError() {
    final web.MediaError? error = _video.error;
    if (error == null) {
      return;
    }
    _report(
      error.message.isEmpty ? 'media error ${error.code}' : error.message,
      fatal: true,
    );
  }

  void _report(String message, {required bool fatal}) {
    final String text = redactStreamTokens(message);
    _log.add(text);
    if (!fatal) {
      return;
    }
    _settle(text);
    if (!_errors.isClosed) {
      _errors.add(text);
    }
  }

  void _settle([String? failure]) {
    final Completer<void>? pending = _attaching;
    _attaching = null;
    if (pending == null) {
      return;
    }
    if (failure == null) {
      pending.complete();
    } else {
      pending.completeError(Exception(failure));
    }
  }

  void _dropReady() {
    final JSFunction? ready = _onReady;
    _onReady = null;
    if (ready != null) {
      _video.removeEventListener('loadedmetadata', ready);
    }
  }

  void _event(String name) {
    final web.TimeRanges ranges = _video.buffered;
    _log.add(
      playbackEventLine(
        name,
        positionS: _video.currentTime,
        durationS: _video.duration,
        buffered: <(double, double)>[
          for (int i = 0; i < ranges.length; i++)
            (ranges.start(i), ranges.end(i)),
        ],
        hidden: web.document.visibilityState == 'hidden',
      ),
    );
  }

  void _seekOnReady(int positionMs, bool autoplay) {
    final JSFunction ready = ((web.Event _) {
      _dropReady();
      _ready(positionMs, autoplay);
    }).toJS;
    _onReady = ready;
    _video.addEventListener('loadedmetadata', ready);
  }

  void _ready(int positionMs, bool autoplay) {
    if (positionMs > 0) {
      _video.currentTime = positionMs / 1000;
    }
    _resume(autoplay);
    _settle();
  }

  Future<void> _resume(bool autoplay) async {
    _video.defaultPlaybackRate = _rate;
    _video.playbackRate = _rate;
    if (!autoplay) {
      return;
    }
    try {
      await _video.play().toDart;
    } catch (_) {
      setMuted(true);
      _mutedByPolicy.value = true;
      try {
        await _video.play().toDart;
      } catch (_) {}
    }
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
    _video.muted = muted;
    _muted.value = muted;
    if (!muted) {
      _mutedByPolicy.value = false;
      if (_video.volume == 0) {
        setVolume(1);
      }
    }
  }

  @override
  void setVolume(double volume) {
    final double level = volume.clamp(0.0, 1.0);
    _video.volume = level;
    _volume.value = level;
    if (level > 0 && _video.muted) {
      setMuted(false);
    }
  }

  @override
  Future<void> togglePictureInPicture() async {
    if (!supportsPictureInPicture) {
      return;
    }
    try {
      if (web.document.pictureInPictureElement != null) {
        await web.document.exitPictureInPicture().toDart;
      } else {
        await _video.requestPictureInPicture().toDart;
      }
    } catch (_) {}
  }

  void _startTimer() {
    _timer ??= Timer.periodic(
      const Duration(milliseconds: 250),
      (_) => _tick(),
    );
  }

  void _tick() {
    final double duration = _video.duration;
    _snapshot.value = PlayerSnapshot(
      positionMs: (_video.currentTime * 1000).round(),
      durationMs: duration.isFinite ? (duration * 1000).round() : 0,
      bufferedAheadMs: (_bufferedAhead() * 1000).round(),
      playing: !_video.paused,
      ready: _video.readyState >= 1,
      buffering: _video.readyState < 3 && !_video.ended,
      ended: _video.ended,
    );
  }

  double _bufferedAhead() {
    final web.TimeRanges ranges = _video.buffered;
    final double at = _video.currentTime;
    for (int i = 0; i < ranges.length; i++) {
      if (at >= ranges.start(i) - 0.5 && at <= ranges.end(i)) {
        final double ahead = ranges.end(i) - at;
        return ahead > 0 ? ahead : 0;
      }
    }
    return 0;
  }

  @override
  Future<void> detach() async {
    _dropReady();
    _settle('detached');
    _teardownHls();
    _timer?.cancel();
    _timer = null;
    _video.pause();
    _video.removeAttribute('src');
    _video.load();
    _snapshot.value = const PlayerSnapshot();
  }

  @override
  void play() {
    unmute();
    _resume(true);
  }

  @override
  void pause() => _video.pause();

  @override
  void togglePlay() {
    unmute();
    _video.paused ? _resume(true) : _video.pause();
  }

  @override
  void seekTo(int positionMs) {
    unmute();
    _video.currentTime = positionMs / 1000;
  }

  @override
  void setRate(double rate) {
    _rate = rate;
    _video.defaultPlaybackRate = rate;
    _video.playbackRate = rate;
  }

  @override
  void setNetworkTimeout(int seconds) {
    _networkTimeoutSeconds = seconds;
    final _Hls? hls = _hls;
    if (hls == null) {
      return;
    }
    hls.config
      ..manifestLoadingTimeOut = seconds * 1000
      ..levelLoadingTimeOut = seconds * 1000
      ..fragLoadingTimeOut = seconds * 1000;
  }

  @override
  bool get buffersAhead => _hls != null;

  @override
  void setBuffer({required int seconds, required int bytes}) {
    _bufferSeconds = seconds;
    _bufferBytes = bytes;
    final _Hls? hls = _hls;
    if (hls == null) {
      return;
    }
    hls.config
      ..maxBufferLength = seconds
      ..maxMaxBufferLength = seconds
      ..maxBufferSize = bytes;
  }

  @override
  void toggleFullscreen() {
    if (web.document.fullscreenElement != null) {
      web.document.exitFullscreen();
    } else {
      web.document.documentElement?.requestFullscreen();
    }
  }

  @override
  PlayerDiagnostics diagnostics() {
    final List<String> lines = <String>[
      'mode: ${_mode.name}',
      'resolution: ${_video.videoWidth}x${_video.videoHeight}',
      'buffer: ${_bufferedAhead().toStringAsFixed(1)}s',
      _hls == null
          ? 'target: browser controlled'
          : 'target: ${_bufferSeconds}s',
    ];
    final _Hls? hls = _hls;
    if (hls != null) {
      final double bw = hls.bandwidthEstimate;
      lines.add(
        'bandwidth: ${bw > 0 ? '${(bw / 1e6).toStringAsFixed(2)} Mbps' : 'n/a'}',
      );
      final List<_HlsLevel> levels = hls.levels.toDart;
      final int idx = hls.currentLevel;
      if (idx >= 0 && idx < levels.length) {
        final _HlsLevel level = levels[idx];
        lines.add(
          'level: ${level.height}p ${(level.bitrate / 1000).round()}kbps'
          '${hls.autoLevelEnabled ? ' (auto)' : ''}',
        );
      } else {
        lines.add('level: auto');
      }
    } else {
      lines.add('bandwidth: n/a (native)');
      lines.add('level: native');
    }
    final web.VideoPlaybackQuality q = _video.getVideoPlaybackQuality();
    lines.add(
      'frames: ${q.droppedVideoFrames} dropped / ${q.totalVideoFrames}',
    );
    lines.addAll(_log.recent());
    return PlayerDiagnostics(lines);
  }

  @override
  List<String> log() => _log.lines;

  void _teardownHls() {
    _hls?.destroy();
    _hls = null;
  }

  void _holdScreen(bool hold) {
    _awake = hold;
    if (hold) {
      _acquireScreen();
      return;
    }
    final web.WakeLockSentinel? held = _screen;
    _screen = null;
    held?.release();
  }

  void _acquireScreen() {
    final web.WakeLockSentinel? held = _screen;
    if (held != null && !held.released) {
      return;
    }
    _screen = null;
    if (!(web.window.navigator as JSObject).has('wakeLock')) {
      return;
    }
    web.window.navigator.wakeLock
        .request('screen')
        .toDart
        .then((web.WakeLockSentinel sentinel) {
          if (_awake) {
            _screen = sentinel;
          } else {
            sentinel.release();
          }
        })
        .catchError((Object _) {});
  }

  @override
  Future<void> dispose() async {
    _dropReady();
    _settle('disposed');
    _timer?.cancel();
    _timer = null;
    web.document.removeEventListener('fullscreenchange', _onFullscreenChange);
    web.document.removeEventListener('visibilitychange', _onVisibilityChange);
    _holdScreen(false);
    _teardownHls();
    _video.pause();
    _video.removeAttribute('src');
    _video.load();
    _cueStyle?.remove();
    await _errors.close();
    _snapshot.dispose();
    _fullscreen.dispose();
    _mutedByPolicy.dispose();
    _muted.dispose();
    _volume.dispose();
    _pip.dispose();
  }
}
