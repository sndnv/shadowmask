import 'dart:async';
import 'dart:js_interop';
import 'dart:ui_web' as ui_web;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:web/web.dart' as web;

import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';

@JS('Hls')
extension type _Hls._(JSObject _) implements JSObject {
  external factory _Hls();
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
}

extension type _HlsEvents._(JSObject _) implements JSObject {
  @JS('MANIFEST_PARSED')
  external String get manifestParsed;
}

extension type _HlsLevel._(JSObject _) implements JSObject {
  external int get height;
  external int get bitrate;
}

int _viewCounter = 0;

PlayerController makePlayerController(String baseUrl) =>
    WebPlayerController(baseUrl);

class WebPlayerController implements PlayerController {
  WebPlayerController(this._baseUrl)
    : _viewType = 'sm-player-${_viewCounter++}' {
    _video = web.document.createElement('video') as web.HTMLVideoElement
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
  }

  final String _baseUrl;
  final String _viewType;
  late final web.HTMLVideoElement _video;
  late final JSFunction _onFullscreenChange;
  _Hls? _hls;
  Timer? _timer;
  double _rate = 1;
  PlaybackMode _mode = PlaybackMode.direct;

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
  Widget get view => HtmlElementView(viewType: _viewType);

  @override
  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
    int positionMs = 0,
  }) async {
    _mode = mode;
    _teardownHls();
    _startTimer();
    final String url = manifestUrl.startsWith('http')
        ? manifestUrl
        : '$_baseUrl$manifestUrl';
    if (mode == PlaybackMode.direct) {
      _video.src = url;
      _seekOnReady(positionMs);
      return;
    }
    if (_Hls.isSupported()) {
      final _Hls hls = _Hls();
      _hls = hls;
      hls.loadSource(url);
      hls.attachMedia(_video);
      hls.on(
        _Hls.events.manifestParsed,
        (() {
          if (positionMs > 0) {
            _video.currentTime = positionMs / 1000;
          }
          _autoplay();
        }).toJS,
      );
      return;
    }
    if (_video.canPlayType('application/vnd.apple.mpegurl').isNotEmpty) {
      _video.src = url;
      _seekOnReady(positionMs);
      return;
    }
    _snapshot.value = const PlayerSnapshot(
      error: 'This browser cannot play HLS video.',
    );
  }

  void _seekOnReady(int positionMs) {
    late final JSFunction handler;
    handler = ((web.Event _) {
      if (positionMs > 0) {
        _video.currentTime = positionMs / 1000;
      }
      _autoplay();
      _video.removeEventListener('loadedmetadata', handler);
    }).toJS;
    _video.addEventListener('loadedmetadata', handler);
  }

  Future<void> _autoplay() async {
    _video.defaultPlaybackRate = _rate;
    _video.playbackRate = _rate;
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
  void play() {
    unmute();
    _autoplay();
  }

  @override
  void pause() => _video.pause();

  @override
  void togglePlay() {
    unmute();
    _video.paused ? _autoplay() : _video.pause();
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
    return PlayerDiagnostics(lines);
  }

  void _teardownHls() {
    _hls?.destroy();
    _hls = null;
  }

  @override
  Future<void> dispose() async {
    _timer?.cancel();
    _timer = null;
    web.document.removeEventListener('fullscreenchange', _onFullscreenChange);
    _teardownHls();
    _video.pause();
    _video.removeAttribute('src');
    _video.load();
    _snapshot.dispose();
    _fullscreen.dispose();
    _mutedByPolicy.dispose();
    _muted.dispose();
    _volume.dispose();
    _pip.dispose();
  }
}
