import 'package:flutter/services.dart';

import 'package:shadowmask/l10n/strings.dart';

enum PlayerShortcut {
  playPause,
  seekBack,
  seekForward,
  volumeUp,
  volumeDown,
  mute,
  fullscreen,
  wide,
  previousEpisode,
  nextEpisode,
  remaining,
  legend,
}

typedef ShortcutRow = ({PlayerShortcut action, String keys, String label});

const int kSeekStepMs = 10000;
const double kVolumeStep = 0.05;

const List<ShortcutRow> kPlayerShortcuts = <ShortcutRow>[
  (
    action: PlayerShortcut.playPause,
    keys: 'Space  K',
    label: Strings.shortcutPlayPause,
  ),
  (
    action: PlayerShortcut.seekBack,
    keys: '←  J',
    label: Strings.shortcutSeekBack,
  ),
  (
    action: PlayerShortcut.seekForward,
    keys: '→  L',
    label: Strings.shortcutSeekForward,
  ),
  (action: PlayerShortcut.volumeUp, keys: '↑', label: Strings.shortcutVolumeUp),
  (
    action: PlayerShortcut.volumeDown,
    keys: '↓',
    label: Strings.shortcutVolumeDown,
  ),
  (action: PlayerShortcut.mute, keys: 'M', label: Strings.shortcutMute),
  (
    action: PlayerShortcut.fullscreen,
    keys: 'F',
    label: Strings.shortcutFullscreen,
  ),
  (action: PlayerShortcut.wide, keys: 'W', label: Strings.shortcutWide),
  (
    action: PlayerShortcut.previousEpisode,
    keys: 'P',
    label: Strings.shortcutPreviousEpisode,
  ),
  (
    action: PlayerShortcut.nextEpisode,
    keys: 'N',
    label: Strings.shortcutNextEpisode,
  ),
  (
    action: PlayerShortcut.remaining,
    keys: 'T',
    label: Strings.shortcutRemaining,
  ),
  (action: PlayerShortcut.legend, keys: '?', label: Strings.shortcutLegend),
];

PlayerShortcut? shortcutFor(LogicalKeyboardKey key) {
  if (key == LogicalKeyboardKey.space || key == LogicalKeyboardKey.keyK) {
    return PlayerShortcut.playPause;
  }
  if (key == LogicalKeyboardKey.arrowLeft || key == LogicalKeyboardKey.keyJ) {
    return PlayerShortcut.seekBack;
  }
  if (key == LogicalKeyboardKey.arrowRight || key == LogicalKeyboardKey.keyL) {
    return PlayerShortcut.seekForward;
  }
  if (key == LogicalKeyboardKey.arrowUp) {
    return PlayerShortcut.volumeUp;
  }
  if (key == LogicalKeyboardKey.arrowDown) {
    return PlayerShortcut.volumeDown;
  }
  if (key == LogicalKeyboardKey.keyM) {
    return PlayerShortcut.mute;
  }
  if (key == LogicalKeyboardKey.keyF) {
    return PlayerShortcut.fullscreen;
  }
  if (key == LogicalKeyboardKey.keyW) {
    return PlayerShortcut.wide;
  }
  if (key == LogicalKeyboardKey.keyP) {
    return PlayerShortcut.previousEpisode;
  }
  if (key == LogicalKeyboardKey.keyN) {
    return PlayerShortcut.nextEpisode;
  }
  if (key == LogicalKeyboardKey.keyT) {
    return PlayerShortcut.remaining;
  }
  if (key == LogicalKeyboardKey.question ||
      key == LogicalKeyboardKey.slash ||
      key == LogicalKeyboardKey.numpadDivide) {
    return PlayerShortcut.legend;
  }
  return null;
}

int? seekFractionFor(LogicalKeyboardKey key) {
  const List<LogicalKeyboardKey> digits = <LogicalKeyboardKey>[
    LogicalKeyboardKey.digit0,
    LogicalKeyboardKey.digit1,
    LogicalKeyboardKey.digit2,
    LogicalKeyboardKey.digit3,
    LogicalKeyboardKey.digit4,
    LogicalKeyboardKey.digit5,
    LogicalKeyboardKey.digit6,
    LogicalKeyboardKey.digit7,
    LogicalKeyboardKey.digit8,
    LogicalKeyboardKey.digit9,
  ];
  final int at = digits.indexOf(key);
  return at < 0 ? null : at;
}
