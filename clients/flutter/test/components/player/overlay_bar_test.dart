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
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';

import '../../support/shape_guard.dart';

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
  AppThemeVariant variant = AppThemeVariant.dark,
  bool highContrast = false,
}) async {
  tester.view.physicalSize = Size(width, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ThemeScope(
      variant: variant,
      setVariant: (_) {},
      highContrast: highContrast,
      child: MaterialApp(
        theme: buildTheme(variant, highContrast: highContrast),
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

  testWidgets('hologram sets the playing time in monospace', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 1200, variant: AppThemeVariant.hologram);

    expect(
      tester.widget<Text>(find.textContaining(' / ')).style?.fontFamily,
      kMonoFamily,
    );
  });

  for (final AppThemeVariant variant in AppThemeVariant.values) {
    testWidgets('the player bar draws the ${variant.name} shapes', (
      WidgetTester tester,
    ) async {
      await _pump(tester, width: 1200, onPlayPause: () {}, variant: variant);

      expectShapesOf(tester, variant);
    });
  }

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

  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.hologram,
  ]) {
    testWidgets('${variant.name} touch targets reach the platform minimum', (
      WidgetTester tester,
    ) async {
      await _pump(tester, width: 1200, touch: true, variant: variant);

      final Size play = tester.getSize(_button(Icons.play_arrow));
      expect(play.width, greaterThanOrEqualTo(44));
      expect(play.height, greaterThanOrEqualTo(44));
    });
  }

  testWidgets('hologram plays from a hexagon on the soft line', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      width: 1200,
      onPlayPause: () {},
      variant: AppThemeVariant.hologram,
    );
    final Material plate = _plate(tester, Icons.play_arrow);

    expect(tester.getSize(_button(Icons.play_arrow)), Marks.playButton);
    expect(plate.shape, Shapes.hexagon(Marks.playButton));
    expect(plate.color, Tokens.hologram.accent.withValues(alpha: 0.26));
  });

  for (final bool highContrast in <bool>[false, true]) {
    testWidgets('hologram${highContrast ? ' high contrast' : ''} draws the '
        'transport icons so they read on their plates', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        width: 1200,
        onPlayPause: () {},
        variant: AppThemeVariant.hologram,
        highContrast: highContrast,
      );
      const Tokens t = Tokens.hologram;

      expect(
        _plate(tester, Icons.play_arrow).color,
        highContrast ? t.accent : t.accent.withValues(alpha: 0.26),
      );
      for (final IconData icon in <IconData>[
        Icons.play_arrow,
        Icons.skip_previous,
        Icons.skip_next,
      ]) {
        expect(
          _ink(tester, icon),
          highContrast ? t.accentContrast : Colors.white,
        );
      }
      expect(
        _plate(tester, Icons.skip_next).color,
        highContrast ? t.accent : t.accent.withValues(alpha: 0.26),
      );
    });
  }

  testWidgets('a hologram play button that cannot act keeps its plate', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 1200, variant: AppThemeVariant.hologram);

    expect(
      tester.widget<IconButton>(_button(Icons.play_arrow)).onPressed,
      isNull,
    );
    expect(
      _plate(tester, Icons.play_arrow).color,
      Tokens.hologram.accent.withValues(alpha: 0.26),
    );
  });

  testWidgets('a touch hexagon keeps the mock\'s size inside its tap area', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      width: 1200,
      touch: true,
      onPlayPause: () {},
      variant: AppThemeVariant.hologram,
    );

    expect(
      tester.getSize(
        find.descendant(
          of: _button(Icons.play_arrow),
          matching: find.byType(Material),
        ),
      ),
      Marks.playButton,
    );
  });

  testWidgets('hologram puts previous and next on smaller hexagons', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      width: 1200,
      onPlayPause: () {},
      variant: AppThemeVariant.hologram,
    );

    for (final IconData icon in <IconData>[
      Icons.skip_previous,
      Icons.skip_next,
    ]) {
      final Material plate = _plate(tester, icon);
      expect(tester.getSize(_button(icon)), Marks.skipButton);
      expect(plate.shape, Shapes.hexagon(Marks.skipButton));
      expect(plate.color, Tokens.hologram.accent.withValues(alpha: 0.26));
    }
    expect(Marks.skipButton, const Size(40, 35));
  });

  testWidgets('a touch skip hexagon keeps its size inside its tap area', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      width: 1200,
      touch: true,
      variant: AppThemeVariant.hologram,
    );

    for (final IconData icon in <IconData>[
      Icons.skip_previous,
      Icons.skip_next,
    ]) {
      expect(tester.getSize(_plateBox(icon)), Marks.skipButton);
      expect(tester.getSize(_button(icon)).height, greaterThanOrEqualTo(44));
    }
  });

  for (final bool touch in <bool>[false, true]) {
    testWidgets('hologram${touch ? ' touch' : ''} keeps the plates apart', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        width: 1200,
        touch: touch,
        onPlayPause: () {},
        variant: AppThemeVariant.hologram,
      );

      expect(
        tester.getTopLeft(_plateBox(Icons.play_arrow)).dx -
            tester.getTopRight(_plateBox(Icons.skip_previous)).dx,
        Space.s1,
      );
      expect(
        tester.getTopLeft(_plateBox(Icons.skip_next)).dx -
            tester.getTopRight(_plateBox(Icons.play_arrow)).dx,
        Space.s1,
      );
    });
  }

  testWidgets('hologram leaves the other bar buttons as they were', (
    WidgetTester tester,
  ) async {
    await _pump(tester, width: 1200, onPlayPause: () {});
    final Size before = tester.getSize(_button(Icons.fullscreen));

    await _pump(
      tester,
      width: 1200,
      onPlayPause: () {},
      variant: AppThemeVariant.hologram,
    );

    expect(tester.getSize(_button(Icons.fullscreen)), before);
    expect(_plate(tester, Icons.fullscreen).color, Colors.transparent);
  });

  for (final AppThemeVariant variant in AppThemeVariant.values.where(
    (AppThemeVariant v) => v != AppThemeVariant.hologram,
  )) {
    testWidgets('${variant.name} keeps the plain play button', (
      WidgetTester tester,
    ) async {
      await _pump(tester, width: 1200, onPlayPause: () {}, variant: variant);

      expect(
        tester.getSize(_button(Icons.play_arrow)),
        tester.getSize(_button(Icons.skip_next)),
      );
      for (final IconData icon in <IconData>[
        Icons.skip_previous,
        Icons.play_arrow,
        Icons.skip_next,
      ]) {
        expect(_plate(tester, icon).color, Colors.transparent);
      }
      expect(
        tester.getTopLeft(_button(Icons.play_arrow)).dx,
        tester.getTopRight(_button(Icons.skip_previous)).dx,
      );
    });
  }

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

Finder _button(IconData icon) =>
    find.ancestor(of: find.byIcon(icon), matching: find.byType(IconButton));

Finder _plateBox(IconData icon) =>
    find.descendant(of: _button(icon), matching: find.byType(Material));

Material _plate(WidgetTester tester, IconData icon) =>
    tester.widget<Material>(_plateBox(icon));

Color? _ink(WidgetTester tester, IconData icon) => tester
    .widget<RichText>(
      find.descendant(of: find.byIcon(icon), matching: find.byType(RichText)),
    )
    .text
    .style
    ?.color;
