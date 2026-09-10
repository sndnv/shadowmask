import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/player/player_controller_stub.dart'
    if (dart.library.js_interop) 'web_player_controller.dart'
    if (dart.library.io) 'native_player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';

abstract class PlayerController {
  ValueListenable<PlayerSnapshot> get snapshot;
  ValueListenable<bool> get fullscreen;

  ValueListenable<bool> get mutedByPolicy;

  ValueListenable<bool> get muted;
  ValueListenable<double> get volume;
  ValueListenable<bool> get pictureInPicture;

  bool get supportsPictureInPicture;

  bool get seeksWithinStream;

  Widget get view;

  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
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
  void setBuffer({required int seconds, required int bytes});
  bool get buffersAhead;
  void toggleFullscreen();
  PlayerDiagnostics diagnostics();
  Future<void> dispose();
}

const int kDefaultBufferBytes = 256 * 1024 * 1024;
const int kDefaultBufferSeconds = 60;

typedef PlayerControllerFactory = PlayerController Function(String baseUrl);

PlayerController createPlayerController(String baseUrl) =>
    makePlayerController(baseUrl);

Future<void> initializePlayer() => preparePlayer();
