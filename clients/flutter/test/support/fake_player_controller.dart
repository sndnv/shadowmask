import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/state/player_tracks.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_size.dart';

class WatchedNotifier<T> extends ValueNotifier<T> {
  WatchedNotifier(super.value);

  bool get watched => hasListeners;
}

class FakePlayerController implements PlayerController {
  FakePlayerController({bool fullscreen = false})
    : _fullscreen = WatchedNotifier<bool>(fullscreen);

  final WatchedNotifier<PlayerSnapshot> _snapshot =
      WatchedNotifier<PlayerSnapshot>(
        const PlayerSnapshot(durationMs: 100000, ready: true),
      );
  final WatchedNotifier<bool> _fullscreen;
  final WatchedNotifier<bool> _mutedByPolicy = WatchedNotifier<bool>(false);
  final WatchedNotifier<bool> _muted = WatchedNotifier<bool>(false);
  final WatchedNotifier<double> _volume = WatchedNotifier<double>(1);
  final WatchedNotifier<bool> _pip = WatchedNotifier<bool>(false);
  final StreamController<String> _errors = StreamController<String>.broadcast();
  final StreamController<void> _lost = StreamController<void>.broadcast();
  final List<String> calls = <String>[];
  int? seekedMs;
  double? rate;
  int? networkTimeout;
  SubtitleSize? subtitleSize;
  int? bufferSeconds;
  int? bufferBytes;
  String? attached;
  int? attachedPositionMs;
  bool? attachedAutoplay;
  bool streamSeeks = true;
  final Set<String?> unseekable = <String?>{};
  bool hlsBuffers = true;
  bool playsAtOnce = true;
  Completer<void>? attachGate;
  int failedAttaches = 0;
  int _streams = 0;
  final List<PlayerTracks> attachedTracks = <PlayerTracks>[];

  List<bool> get attachedSubtitles => <bool>[
    for (final PlayerTracks tracks in attachedTracks) tracks.rendition,
  ];

  @override
  ValueListenable<PlayerSnapshot> get snapshot => _snapshot;

  @override
  ValueListenable<bool> get fullscreen => _fullscreen;

  @override
  Stream<String> get errors => _errors.stream;

  void fail(String message) => _errors.add(message);

  @override
  Stream<void> get lost => _lost.stream;

  void lose() => _lost.add(null);

  Map<String, bool> get listened => <String, bool>{
    'snapshot': _snapshot.watched,
    'fullscreen': _fullscreen.watched,
    'mutedByPolicy': _mutedByPolicy.watched,
    'muted': _muted.watched,
    'volume': _volume.watched,
    'pictureInPicture': _pip.watched,
    'errors': _errors.hasListener,
    'lost': _lost.hasListener,
  };

  @override
  ValueListenable<bool> get mutedByPolicy => _mutedByPolicy;

  @override
  ValueListenable<bool> get muted => _muted;

  @override
  ValueListenable<double> get volume => _volume;

  @override
  ValueListenable<bool> get pictureInPicture => _pip;

  @override
  bool get supportsPictureInPicture => true;

  @override
  bool seeksWithin(String? container) =>
      streamSeeks && !unseekable.contains(container);

  @override
  void unmute() {
    if (!_mutedByPolicy.value) {
      return;
    }
    setMuted(false);
    calls.add('unmute');
  }

  @override
  void setMuted(bool muted) {
    _muted.value = muted;
    if (!muted) {
      _mutedByPolicy.value = false;
    }
    calls.add('muted:$muted');
  }

  @override
  void setVolume(double volume) {
    _volume.value = volume;
    calls.add('volume:$volume');
    if (volume > 0 && _muted.value) {
      setMuted(false);
    }
  }

  @override
  Future<void> togglePictureInPicture() async {
    _pip.value = !_pip.value;
    calls.add('pip');
  }

  void mutePolicy() {
    _muted.value = true;
    _mutedByPolicy.value = true;
  }

  void emit({bool ended = false, bool playing = false}) {
    _snapshot.value = PlayerSnapshot(
      durationMs: 100000,
      positionMs: ended ? 100000 : 0,
      ready: true,
      ended: ended,
      playing: playing,
    );
  }

  void opening() {
    _snapshot.value = const PlayerSnapshot(durationMs: 100000, buffering: true);
  }

  void at({required int positionMs, int bufferedAheadMs = 0}) {
    _snapshot.value = PlayerSnapshot(
      durationMs: 100000,
      positionMs: positionMs,
      bufferedAheadMs: bufferedAheadMs,
      ready: true,
      playing: true,
    );
  }

  void pausedAt({required int positionMs}) {
    _snapshot.value = PlayerSnapshot(
      durationMs: 100000,
      positionMs: positionMs,
      ready: true,
    );
  }

  void endedAt({required int positionMs}) {
    _snapshot.value = PlayerSnapshot(
      durationMs: 100000,
      positionMs: positionMs,
      ready: true,
      ended: true,
    );
  }

  void stall({
    required int positionMs,
    bool ready = true,
    int bufferedAheadMs = 0,
  }) {
    _snapshot.value = PlayerSnapshot(
      durationMs: 100000,
      positionMs: positionMs,
      bufferedAheadMs: bufferedAheadMs,
      ready: ready,
      buffering: true,
      playing: true,
    );
  }

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

  int? output;

  @override
  int? get outputChannels => output;

  final List<(bool, DialogueLift)> tunings = <(bool, DialogueLift)>[];

  @override
  Future<void> tuneSound({
    required bool normalize,
    required DialogueLift dialogue,
  }) async => tunings.add((normalize, dialogue));

  @override
  Widget get view => const SizedBox.shrink();

  @override
  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
    required PlayerTracks tracks,
    int positionMs = 0,
    bool autoplay = true,
  }) async {
    calls.add('attach:${mode.name}');
    attachedTracks.add(tracks);
    final int stream = ++_streams;
    _snapshot.value = const PlayerSnapshot(durationMs: 100000, ready: true);
    await attachGate?.future;
    if (stream != _streams) {
      throw StateError('superseded by another stream');
    }
    if (failedAttaches > 0) {
      failedAttaches--;
      throw Exception('no stream');
    }
    attached = manifestUrl;
    attachedPositionMs = positionMs;
    attachedAutoplay = autoplay;
  }

  @override
  Future<void> detach() async {
    _streams++;
    _snapshot.value = const PlayerSnapshot();
    calls.add('detach');
  }

  @override
  void play() {
    calls.add('play');
    _follow(playing: true);
  }

  @override
  void pause() {
    calls.add('pause');
    _follow(playing: false);
  }

  void _follow({required bool playing}) {
    if (playsAtOnce) {
      _snapshot.value = _snapshot.value.copyWith(playing: playing);
    }
  }

  @override
  void togglePlay() => calls.add('toggle');

  @override
  void seekTo(int positionMs) {
    seekedMs = positionMs;
    calls.add('seek');
  }

  @override
  void setRate(double value) => rate = value;

  @override
  void setNetworkTimeout(int seconds) {
    networkTimeout = seconds;
    calls.add('timeout:$seconds');
  }

  @override
  void setSubtitleSize(SubtitleSize size) => subtitleSize = size;

  @override
  bool get buffersAhead => hlsBuffers;

  @override
  void setBuffer({required int seconds, required int bytes}) {
    bufferSeconds = seconds;
    bufferBytes = bytes;
    calls.add('buffer:$seconds');
  }

  @override
  void toggleFullscreen() {
    _fullscreen.value = !_fullscreen.value;
    calls.add('fullscreen');
  }

  @override
  PlayerDiagnostics diagnostics() => const PlayerDiagnostics(<String>[]);

  List<String> logLines = <String>[];

  @override
  List<String> log() => logLines;

  @override
  Future<void> dispose() async {}
}
