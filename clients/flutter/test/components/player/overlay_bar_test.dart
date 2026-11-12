import 'package:material_ui/material_ui.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/player/overlay_bar.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

final List<PlayerPanel> _opened = <PlayerPanel>[];

Future<void> _pump(
  WidgetTester tester, {
  required double width,
  bool episode = true,
  bool canToggleWide = true,
  bool touch = false,
  EdgeInsets safe = EdgeInsets.zero,
  VoidCallback? onOpenSheet,
  VoidCallback? onPlayPause,
}) async {
  tester.view.physicalSize = Size(width, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: MediaQuery(
          data: MediaQueryData(padding: safe),
          child: Scaffold(
            body: Align(
              alignment: Alignment.bottomCenter,
              child: OverlayBar(
                snapshot: const PlayerSnapshot(positionMs: 10, durationMs: 100),
                timeline: const SizedBox(height: 4),
                fullscreen: false,
                wide: false,
                muted: false,
                volume: 0.5,
                remaining: false,
                onPlayPause: onPlayPause,
                onOpenPanel: _opened.add,
                onToggleFullscreen: () {},
                onToggleWide: canToggleWide ? () {} : null,
                onToggleMute: () {},
                onVolume: (_) {},
                onToggleRemaining: () {},
                previousEpisode: episode
                    ? (tooltip: 'Previous', onPressed: () {})
                    : null,
                nextEpisode: episode
                    ? (tooltip: 'Next', onPressed: () {})
                    : null,
                touch: touch,
                onOpenSheet: onOpenSheet,
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(_opened.clear);

  testWidgets('the play button acts when wired and is disabled when not', (
    WidgetTester tester,
  ) async {
    int pressed = 0;
    await _pump(tester, width: 1200, onPlayPause: () => pressed++);
    await tester.tap(find.byTooltip(Strings.playerPlay));
    expect(pressed, 1);

    await _pump(tester, width: 1200);
    final IconButton button = tester.widget<IconButton>(
      find.ancestor(
        of: find.byIcon(Icons.play_arrow),
        matching: find.byType(IconButton),
      ),
    );
    expect(button.onPressed, isNull);
  });

  testWidgets('a wide bar shows the volume slider and every panel button', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 1200);

    expect(find.byType(Slider), findsOneWidget);
    expect(find.byIcon(Icons.settings), findsNothing);
    for (final PlayerPanel panel in PlayerPanel.values) {
      expect(
        find.byWidgetPredicate(
          (Widget w) => w is PanelIcon && w.panel == panel,
        ),
        findsOneWidget,
      );
    }
  });

  testWidgets('the audio button reads as its name, not as an image', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester, width: 1200);

    final SemanticsNode audio = tester.getSemantics(
      find.byTooltip(Strings.playerAudio),
    );
    expect(audio.tooltip, Strings.playerAudio);
    expect(audio.flagsCollection.isImage, isFalse);
    semantics.dispose();
  });

  testWidgets('each panel button wears the mark the Roku bar uses', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 1200);

    expect(find.byIcon(Icons.video_settings), findsOneWidget);
    expect(find.byIcon(Icons.subtitles), findsOneWidget);
    expect(find.byIcon(Icons.tune), findsOneWidget);
    final SvgPicture audio = tester.widget<SvgPicture>(
      find.descendant(
        of: find.byTooltip(Strings.playerAudio),
        matching: find.byType(SvgPicture),
      ),
    );
    final String glyph = (audio.bytesLoader as SvgAssetLoader).assetName;
    expect(glyph, PlayerPanel.audio.glyph);
    expect(await rootBundle.loadString(glyph), contains('<svg'));
  });

  testWidgets('an episode bar fits a phone without overflowing', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 360);

    expect(
      tester.takeException(),
      isNull,
      reason: 'eleven rigid children needed 440 and a phone stage has 304',
    );
    expect(find.byType(Slider), findsNothing);
    expect(find.byIcon(Icons.settings), findsOneWidget);
    expect(
      find.byIcon(Icons.volume_up),
      findsOneWidget,
      reason: 'the slider goes, muting stays reachable',
    );
  });

  testWidgets('no wide control when the page is already forced wide', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 360, canToggleWide: false);

    expect(find.byIcon(Icons.width_wide), findsNothing);
    expect(
      find.byIcon(Icons.fullscreen),
      findsOneWidget,
      reason: 'fullscreen is a separate control and still belongs there',
    );
  });

  testWidgets('the folded menu still reaches all four panels', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 360);
    await tester.tap(find.byIcon(Icons.settings));
    await tester.pumpAndSettle();

    for (final PlayerPanel panel in PlayerPanel.values) {
      expect(find.text(panel.title), findsOneWidget);
    }

    await tester.tap(find.text(Strings.playerSubtitles));
    await tester.pumpAndSettle();

    expect(_opened, <PlayerPanel>[PlayerPanel.subtitles]);
  });

  testWidgets('on touch the options button opens the sheet, not a menu', (
    WidgetTester tester,
  ) async {
    int opened = 0;
    await _pump(tester, width: 1200, touch: true, onOpenSheet: () => opened++);

    await tester.tap(find.byIcon(Icons.settings));
    await tester.pumpAndSettle();

    expect(opened, 1);
    expect(find.text(PlayerPanel.audio.title), findsNothing);
  });

  testWidgets('a touch bar folds its panels and drops the screen controls', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 1200, touch: true);

    expect(
      find.byIcon(Icons.settings),
      findsOneWidget,
      reason: 'a wide phone in landscape still wants one big target, not five',
    );
    expect(find.byType(Slider), findsNothing);
    expect(find.byIcon(Icons.fullscreen), findsNothing);
    expect(find.byIcon(Icons.width_wide), findsNothing);
    expect(
      find.byIcon(Icons.volume_up),
      findsOneWidget,
      reason: 'mute stays; only the slider goes to the hardware keys',
    );
  });

  testWidgets('touch targets reach the platform minimum', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 1200, touch: true);

    final Size play = tester.getSize(
      find.ancestor(
        of: find.byIcon(Icons.play_arrow),
        matching: find.byType(IconButton),
      ),
    );
    expect(play.width, greaterThanOrEqualTo(44));
    expect(play.height, greaterThanOrEqualTo(44));
  });

  // The bar owns its own safe-area inset so its scrim can reach the screen
  // edge. Insetting it from outside left an unshaded strip under the shadow.
  testWidgets('the bar reaches the edge and its controls clear the indicator', (
    WidgetTester tester,
  ) async {
    const EdgeInsets safe = EdgeInsets.fromLTRB(48, 0, 48, 24);
    await _pump(tester, width: 1200, touch: true, safe: safe);

    final Rect bar = tester.getRect(find.byType(OverlayBar));
    final Rect play = tester.getRect(
      find.ancestor(
        of: find.byIcon(Icons.play_arrow),
        matching: find.byType(IconButton),
      ),
    );

    expect(bar.bottom, closeTo(800, 0.5));
    expect(play.bottom, lessThanOrEqualTo(bar.bottom - safe.bottom));
    expect(play.left, greaterThanOrEqualTo(bar.left + safe.left));
  });
}
