import 'dart:async';
import 'dart:math';

import 'package:flutter/foundation.dart';

import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/negotiation.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/player/actors/lifecycle.dart';
import 'package:shadowmask/player/actors/player.dart';
import 'package:shadowmask/player/actors/server.dart';
import 'package:shadowmask/player/actors/viewer.dart';
import 'package:shadowmask/player/playback_history.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/player/state/playback_title.dart';
import 'package:shadowmask/player/state/player_tracks.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/player/watches/buffer_watch.dart';
import 'package:shadowmask/player/watches/stall_watch.dart';
import 'package:shadowmask/view/episode_neighbours.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';

const int kRequestTimeoutS = 120;
const int kLoadTimeoutMarginS = 5;

class PlaybackRunner {
  PlaybackRunner({
    required this._api,
    required this._player,
    required this._saveLocal,
    required this._platform,
    required String versionId,
    required PlaybackControls controls,
    required bool playing,
    PlaybackHistory? history,
  }) : history = history ?? PlaybackHistory() {
    this.history.add('opened');
    final (
      PlaybackState opened,
      List<PlaybackCommand> commands,
    ) = lifecycleOpens(
      versionId: versionId,
      local: LocalSettings(
        fullscreen: _player.fullscreen.value,
        pictureInPicture: _player.pictureInPicture.value,
        mutedByPolicy: _player.mutedByPolicy.value,
      ),
      controls: controls,
      playing: playing,
      mixesSound: _player.mixesSound,
    );
    _state = ValueNotifier<PlaybackState>(opened);
    _inbox.stream.listen(_handle);
    _player.snapshot.addListener(_onReading);
    _player.fullscreen.addListener(_onFullscreen);
    _player.pictureInPicture.addListener(_onPictureInPicture);
    _player.volume.addListener(_onSound);
    _player.muted.addListener(_onSound);
    _player.mutedByPolicy.addListener(_onSound);
    _errors = _player.errors.listen(_onError);
    _lost = _player.lost.listen((_) => _onLost());
    _settle(PlaybackState(versionId: versionId), opened, commands);
  }

  final PlaybackHistory history;
  final PlaybackApi _api;
  final PlayerController _player;
  final Future<void> Function(LocalSettings local) _saveLocal;
  late int _profileVersion;
  final String _platform;
  final StreamController<PlaybackWrite> _inbox =
      StreamController<PlaybackWrite>();
  final Completer<void> _left = Completer<void>();
  late final ValueNotifier<PlaybackState> _state;
  late final StreamSubscription<String> _errors;
  late final StreamSubscription<void> _lost;
  LocalSettings _applied = const LocalSettings();
  Future<void> _ending = Future<void>.value();
  StallWatch _stall = kStallWatchStart;
  BufferWatch _buffer = kBufferWatchStart;
  (String, int)? _beat;
  Timer? _heartbeat;
  Timer? _slow;
  Timer? _timeout;
  Timer? _settling;
  Timer? _ticker;

  ValueListenable<PlaybackState> get state => _state;

  void opened({
    required PlaybackTitle title,
    required LocalSettings local,
    required int resumeMs,
    required int profileVersion,
  }) {
    _profileVersion = profileVersion;
    _post(
      (PlaybackState s) =>
          lifecycleGathered(s, title: title, local: local, resumeMs: resumeMs),
    );
  }

  void seek(int positionMs) =>
      _post((PlaybackState s) => viewerSeeks(s, positionMs));

  void seekBy(int deltaMs) =>
      _post((PlaybackState s) => viewerSeeksBy(s, deltaMs));

  void play() => _post(viewerPlays);

  void pause() => _post(viewerPauses);

  void replay() => _post(viewerReplays);

  void toggle() => _post(viewerToggles);

  void seekToFraction(double fraction) =>
      _post((PlaybackState s) => viewerSeeksToFraction(s, fraction));

  void changeRemote(PlaybackControls controls) {
    history.add('chose', <String, Object?>{
      'controls': controls.toUpdateBody(),
    });
    _post((PlaybackState s) => viewerChangesRemote(s, controls));
  }

  void changeLocal(LocalSettings Function(LocalSettings local) change) =>
      _post((PlaybackState s) => viewerChangesLocal(s, change(s.local)));

  void retry() => _post(viewerRetries);

  void keepWaiting() {
    _stall = kStallWatchStart;
    _post(viewerKeepsWaiting);
  }

  void describe({
    required String? name,
    required EpisodeLink? previous,
    required EpisodeLink? next,
  }) => _post(
    (PlaybackState s) =>
        serverDescribesTitle(s, name: name, previous: previous, next: next),
  );

  void refreshVersion(VersionDetail version) =>
      _post((PlaybackState s) => serverRefreshesVersion(s, version));

  void background() => _post(lifecycleBackgrounds);

  void foreground({required bool playing}) =>
      _post((PlaybackState s) => lifecycleForegrounds(s, playing: playing));

  Future<void> leave() {
    _post(lifecycleLeaves);
    return _left.future;
  }

  void _post(PlaybackWrite write) => _inbox.add(write);

  void _handle(PlaybackWrite write) {
    final PlaybackState before = _state.value;
    final (PlaybackState after, List<PlaybackCommand> commands) = write(before);
    _settle(before, after, commands);
  }

  void _settle(
    PlaybackState before,
    PlaybackState after,
    List<PlaybackCommand> commands,
  ) {
    _state.value = after;
    _arm(before, after, commands);
    commands.forEach(_run);
    final bool open = after.status.life == PlaybackLife.open;
    final bool plays = commands.any((PlaybackCommand c) => c is PlayPlayer);
    if (open &&
        !plays &&
        before.status.sent != null &&
        after.status.sent == null) {
      _post((PlaybackState s) => playerReads(s, _player.snapshot.value));
      if (after.status.holding) {
        _tick();
      }
    }
    if (after.status.life == PlaybackLife.closed &&
        before.status.life != PlaybackLife.closed) {
      _close();
    }
  }

  void _arm(
    PlaybackState before,
    PlaybackState after,
    List<PlaybackCommand> commands,
  ) {
    final bool open = after.status.life == PlaybackLife.open;
    final PlaybackStream? stream = open ? after.stream : null;
    final (String, int)? beat = stream == null
        ? null
        : (stream.sessionId, stream.heartbeatIntervalS);
    if (beat != _beat) {
      _beat = beat;
      _heartbeat?.cancel();
      _heartbeat = beat == null
          ? null
          : Timer.periodic(
              Duration(seconds: max(1, beat.$2)),
              (_) => _post(heartbeatDue),
            );
    }
    final SentChange? sent = after.status.sent;
    if (sent?.id != before.status.sent?.id) {
      _slow?.cancel();
      _slow = sent == null
          ? null
          : Timer(
              const Duration(seconds: kStallSeconds),
              () => _post((PlaybackState s) => changeSlow(s, sent.id)),
            );
    }
    if (sent != before.status.sent) {
      _timeout?.cancel();
      _timeout = sent == null ? null : _timeoutFor(sent, after.local);
    }
    final bool settles =
        commands.any((PlaybackCommand c) => c is SeekPlayer) ||
        before.status.sent != null;
    final int? target = after.timeline.seekingToMs;
    if (!open || sent != null) {
      _settling?.cancel();
      _settling = null;
    } else if (settles) {
      _settling?.cancel();
      _settling = target == null
          ? null
          : Timer(
              const Duration(milliseconds: kSeekSettleWaitMs),
              () => _post((PlaybackState s) => seekSettled(s, target)),
            );
    }
    if (open) {
      _ticker ??= Timer.periodic(const Duration(seconds: 1), (_) => _tick());
    } else {
      _ticker?.cancel();
      _ticker = null;
    }
  }

  Timer _timeoutFor(SentChange sent, LocalSettings local) {
    final int seconds = sent is SentLoad
        ? local.networkTimeoutS + kLoadTimeoutMarginS
        : kRequestTimeoutS;
    final String message = sent is SentStart
        ? Strings.couldNotStartPlayback
        : Strings.playerFailed;
    return Timer(
      Duration(seconds: seconds),
      () =>
          _post((PlaybackState s) => changeTimedOut(s, sent, message: message)),
    );
  }

  void _tick() {
    final PlaybackState s = _state.value;
    final PlayerSnapshot reading = _player.snapshot.value;
    if (!s.watching) {
      _stall = stallIdle(reading, errored: s.status.error != null);
      _buffer = kBufferWatchStart;
      return;
    }
    if (s.status.holding) {
      final (BufferWatch buffer, bool ready) = bufferTick(
        _buffer,
        reading,
        bufferS: s.local.bufferS,
        buffersAhead: _player.buffersAhead,
      );
      _buffer = buffer;
      if (ready) {
        _post(playerBufferReady);
      }
      return;
    }
    _buffer = kBufferWatchStart;
    final StallTick tick = stallTick(
      _stall,
      reading,
      errored: s.status.error != null,
    );
    _stall = tick.watch;
    if (tick.movedPastError) {
      _post(playerMovedPastError);
    }
    if (tick.stalled case final bool stalled) {
      _post((PlaybackState s) => stallChanged(s, stalled));
    }
  }

  void _run(PlaybackCommand command) {
    switch (command) {
      case SendStart(
        :final int id,
        :final PlaybackControls controls,
        :final int positionMs,
      ):
        _start(id, controls, positionMs);
      case SendUpdate(
        :final int id,
        :final String sessionId,
        :final PlaybackControls controls,
        :final int positionMs,
      ):
        history.add('sent_update', <String, Object?>{
          'session': sessionId,
          'controls': controls.toUpdateBody(),
          'position_ms': positionMs,
        });
        unawaited(
          _api
              .update(sessionId, controls, positionMs)
              .then(
                (Negotiation n) {
                  history.add(
                    'answered',
                    negotiatedFacts('update', sessionId, n),
                  );
                  _post(
                    (PlaybackState s) => serverUpdated(
                      s,
                      id: id,
                      negotiation: n,
                      seeksInStream: _player.seeksWithin(n.container),
                    ),
                  );
                },
                onError: (Object error) {
                  history.add('failed', failedFacts('update', error));
                  _post((PlaybackState s) => serverFailed(s, id));
                },
              ),
        );
      case SendSeek(
        :final int id,
        :final String sessionId,
        :final int positionMs,
      ):
        history.add('sent_seek', <String, Object?>{
          'session': sessionId,
          'position_ms': positionMs,
        });
        unawaited(
          _api
              .seek(sessionId, positionMs)
              .then(
                (Negotiation n) {
                  history.add(
                    'answered',
                    negotiatedFacts('seek', sessionId, n),
                  );
                  _post(
                    (PlaybackState s) => serverSeeked(
                      s,
                      id: id,
                      negotiation: n,
                      seeksInStream: _player.seeksWithin(n.container),
                    ),
                  );
                },
                onError: (Object error) {
                  history.add('failed', failedFacts('seek', error));
                  _post((PlaybackState s) => serverFailed(s, id));
                },
              ),
        );
      case SendProgress(
        :final String sessionId,
        :final int positionMs,
        :final bool playing,
      ):
        unawaited(_api.progress(sessionId, positionMs, _wire(playing)));
      case EndSession(
        :final String sessionId,
        :final int? positionMs,
        :final bool playing,
      ):
        history.add('ended', <String, Object?>{'session': sessionId});
        _ending = _afterEnds().then(
          (_) => _end(sessionId, positionMs, playing),
        );
      case LoadStream(
        :final int id,
        :final PlaybackStream stream,
        :final int offsetMs,
        :final PlayerTracks tracks,
      ):
        history.add('load', <String, Object?>{
          'session': stream.sessionId,
          'offset_ms': offsetMs,
        });
        _tune(_state.value.remote);
        unawaited(
          _player
              .attach(
                stream.manifestUrl,
                mode: stream.mode,
                tracks: tracks,
                positionMs: offsetMs,
                autoplay: false,
              )
              .then(
                (_) {
                  history.add('loaded');
                  _post((PlaybackState s) => playerLoaded(s, id));
                },
                onError: (Object error) {
                  history.add('load_failed', <String, Object?>{
                    'message': redactStreamTokens(error.toString()),
                  });
                  _post(
                    (PlaybackState s) =>
                        playerFailedToLoad(s, id, Strings.playerFailed),
                  );
                },
              ),
        );
      case SeekPlayer(:final int offsetMs):
        _player.seekTo(offsetMs);
      case PlayPlayer():
        _player.play();
      case PausePlayer():
        _player.pause();
      case UnloadPlayer():
        unawaited(_player.detach());
      case ApplyLocal(:final LocalSettings local):
        _apply(local);
      case SaveLocal(:final LocalSettings local):
        unawaited(_saveLocal(local));
      case TuneSound(:final bool normalize, :final DialogueLift dialogue):
        unawaited(_player.tuneSound(normalize: normalize, dialogue: dialogue));
    }
  }

  void _tune(PlaybackControls controls) => unawaited(
    _player.tuneSound(
      normalize: controls.normalize ?? true,
      dialogue: controls.dialogue ?? DialogueLift.none,
    ),
  );

  void _start(int id, PlaybackControls controls, int positionMs) {
    final PlaybackState s = _state.value;
    unawaited(
      _afterEnds()
          .then((_) {
            history.add('sent_start', <String, Object?>{
              'position_ms': positionMs,
              'controls': controls.toStartBody(),
              'capabilities': <String, Object?>{
                'platform': _platform,
                'profile_version': _profileVersion,
                'decoding': s.local.decoding?.toJson(),
                'subtitle_deliveries': _player.subtitleDeliveries,
                'audio_tracks': _player.selectsAudioTracks,
                'audio_downmix': _player.mixesSound,
              },
            });
            return _api.startSession(
              versionId: s.versionId,
              startPositionMs: positionMs,
              profileVersion: _profileVersion,
              platform: _platform,
              decoding: s.local.decoding,
              subtitleDeliveries: _player.subtitleDeliveries,
              selectsAudioTracks: _player.selectsAudioTracks,
              mixesSound: _player.mixesSound,
              controls: controls,
            );
          })
          .then(
            (PlaybackSession session) {
              history.add('answered', startedFacts(session));
              _post(
                (PlaybackState s) => serverStarted(
                  s,
                  id: id,
                  session: session,
                  positionMs: positionMs,
                  seeksInStream: _player.seeksWithin(session.container),
                ),
              );
            },
            onError: (Object error) {
              history.add('failed', failedFacts('start', error));
              _post(
                (PlaybackState s) =>
                    serverRefusedStart(s, id, Strings.couldNotStartPlayback),
              );
            },
          ),
    );
  }

  Future<void> _afterEnds() => _ending.timeout(
    Duration(seconds: _state.value.local.networkTimeoutS),
    onTimeout: () {},
  );

  Future<void> _end(String sessionId, int? positionMs, bool playing) async {
    await Future.wait(<Future<void>>[
      if (positionMs != null)
        _api.progress(sessionId, positionMs, _wire(playing)),
      _api.endSession(sessionId),
    ]);
  }

  String _wire(bool playing) => playing ? 'playing' : 'paused';

  void _apply(LocalSettings local) {
    final LocalSettings was = _applied;
    _applied = local;
    if (local.speed != was.speed) {
      _player.setRate(local.speed);
    }
    if (local.volume != was.volume) {
      _player.setVolume(local.volume);
    }
    if (local.muted != was.muted) {
      _player.setMuted(local.muted);
    }
    if (local.subtitleSize != was.subtitleSize) {
      _player.setSubtitleSize(local.subtitleSize);
    }
    if (local.networkTimeoutS != was.networkTimeoutS) {
      _player.setNetworkTimeout(local.networkTimeoutS);
    }
    if (local.bufferS != was.bufferS || local.bufferBytes != was.bufferBytes) {
      _player.setBuffer(seconds: local.bufferS, bytes: local.bufferBytes);
    }
  }

  void _onReading() {
    final PlayerSnapshot reading = _player.snapshot.value;
    _post((PlaybackState s) => playerReads(s, reading));
  }

  void _onError(String message) {
    _stall = stallErred(_stall, _player.snapshot.value);
    _post((PlaybackState s) => playerErrs(s, message));
  }

  void _onLost() =>
      _post((PlaybackState s) => playerLostStream(s, Strings.playerFailed));

  void _onFullscreen() => _report(
    (LocalSettings l) => l.copyWith(fullscreen: _player.fullscreen.value),
  );

  void _onPictureInPicture() => _report(
    (LocalSettings l) =>
        l.copyWith(pictureInPicture: _player.pictureInPicture.value),
  );

  void _onSound() => _report(
    (LocalSettings l) => l.copyWith(
      volume: _player.volume.value,
      muted: _player.mutedByPolicy.value ? l.muted : _player.muted.value,
      mutedByPolicy: _player.mutedByPolicy.value,
    ),
  );

  void _report(LocalSettings Function(LocalSettings local) change) =>
      _post((PlaybackState s) {
        _applied = change(_applied);
        return playerReportsLocal(s, change(s.local));
      });

  void _close() {
    _player.snapshot.removeListener(_onReading);
    _player.fullscreen.removeListener(_onFullscreen);
    _player.pictureInPicture.removeListener(_onPictureInPicture);
    _player.volume.removeListener(_onSound);
    _player.muted.removeListener(_onSound);
    _player.mutedByPolicy.removeListener(_onSound);
    unawaited(_errors.cancel());
    unawaited(_lost.cancel());
    _left.complete();
  }
}
