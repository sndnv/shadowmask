import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/player/player_controller_stub.dart'
    if (dart.library.js_interop) 'web_player_controller.dart'
    if (dart.library.io) 'native_player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/state/player_tracks.dart';
import 'package:shadowmask/view/subtitle_size.dart';

const String kFmp4Segments = 'fmp4';

abstract class PlayerController {
  ValueListenable<PlayerSnapshot> get snapshot;
  ValueListenable<bool> get fullscreen;

  Stream<String> get errors;

  ValueListenable<bool> get mutedByPolicy;

  ValueListenable<bool> get muted;
  ValueListenable<double> get volume;
  ValueListenable<bool> get pictureInPicture;

  bool get supportsPictureInPicture;

  bool seeksWithin(String? container);

  List<String> get subtitleDeliveries;

  bool get selectsAudioTracks;

  Widget get view;

  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
    required PlayerTracks tracks,
    int positionMs = 0,
    bool autoplay = true,
  });

  Future<void> detach();

  void play();
  void pause();
  void togglePlay();

  void unmute();

  void setMuted(bool muted);
  void setVolume(double volume);
  Future<void> togglePictureInPicture();
  void seekTo(int positionMs);
  void setRate(double rate);
  void setNetworkTimeout(int seconds);
  void setSubtitleSize(SubtitleSize size);
  void setBuffer({required int seconds, required int bytes});
  bool get buffersAhead;
  void toggleFullscreen();
  PlayerDiagnostics diagnostics();
  List<String> log();
  Future<void> dispose();
}

const int kDefaultBufferBytes = 256 * 1024 * 1024;
const int kDefaultBufferSeconds = 60;
const int kDefaultNetworkTimeoutSeconds = 10;
const int kSeekReachMs = 30000;
const int kSeekSettleMs = 1000;
const int kSeekSettleWaitMs = 2000;

typedef PlayerControllerFactory = PlayerController Function(String baseUrl);

PlayerController createPlayerController(String baseUrl) =>
    makePlayerController(baseUrl);

Future<void> initializePlayer() => preparePlayer();
