import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/view/player_shortcuts.dart';

void main() {
  test('the common transport keys map to their actions', () {
    expect(shortcutFor(LogicalKeyboardKey.space), PlayerShortcut.playPause);
    expect(shortcutFor(LogicalKeyboardKey.keyK), PlayerShortcut.playPause);
    expect(shortcutFor(LogicalKeyboardKey.arrowLeft), PlayerShortcut.seekBack);
    expect(
      shortcutFor(LogicalKeyboardKey.arrowRight),
      PlayerShortcut.seekForward,
    );
    expect(shortcutFor(LogicalKeyboardKey.keyM), PlayerShortcut.mute);
    expect(shortcutFor(LogicalKeyboardKey.keyF), PlayerShortcut.fullscreen);
    expect(shortcutFor(LogicalKeyboardKey.keyN), PlayerShortcut.nextEpisode);
  });

  test('an unbound key claims nothing', () {
    expect(shortcutFor(LogicalKeyboardKey.keyZ), isNull);
    expect(shortcutFor(LogicalKeyboardKey.escape), isNull);
  });

  test('the legend has a key of its own', () {
    // Shift produces "?" on most layouts, so the slash it sits on counts too.
    expect(shortcutFor(LogicalKeyboardKey.question), PlayerShortcut.legend);
    expect(shortcutFor(LogicalKeyboardKey.slash), PlayerShortcut.legend);
  });

  test('the digits map to their tenth of the runtime', () {
    expect(seekFractionFor(LogicalKeyboardKey.digit0), 0);
    expect(seekFractionFor(LogicalKeyboardKey.digit5), 5);
    expect(seekFractionFor(LogicalKeyboardKey.digit9), 9);
    expect(seekFractionFor(LogicalKeyboardKey.keyA), isNull);
  });

  test('every action in the enum is listed in the legend', () {
    final Set<PlayerShortcut> listed = kPlayerShortcuts
        .map((ShortcutRow r) => r.action)
        .toSet();

    expect(
      listed,
      PlayerShortcut.values.toSet(),
      reason:
          'the legend is the only documentation of these keys, so an action '
          'added to the enum without a row would be undiscoverable',
    );
  });

  test('no two rows claim the same action', () {
    expect(
      kPlayerShortcuts.map((ShortcutRow r) => r.action).toSet().length,
      kPlayerShortcuts.length,
    );
  });
}
