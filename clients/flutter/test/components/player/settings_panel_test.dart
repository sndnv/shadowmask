import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/player/player_sheet.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/util/bundled_fonts.dart';
import 'package:shadowmask/util/bundled_licenses.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_choice.dart';
import 'package:shadowmask/view/subtitle_size.dart';

VersionDetail _version() => VersionDetail.fromJson(<String, dynamic>{
  'id': 'v1',
  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
  'library_id': 'l',
  'quality': 'fhd',
  'container': 'mkv',
  'subtitles': <dynamic>[
    <String, dynamic>{
      'index': 0,
      'language': 'en',
      'format': 'pgs',
      'forced': false,
      'default': true,
    },
  ],
});

Future<void> _pump(
  WidgetTester tester, {
  required String delivery,
  required bool burn,
  bool dense = false,
  ValueChanged<SubtitleSize>? onSubtitleSize,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: SizedBox(
          width: 420,
          child: PlayerSettingsPanel(
            panel: PlayerPanel.subtitles,
            controls: PlaybackControls(
              burn: burn,
              subtitle: const SubtitleChoice.on(SubtitleSelection.embedded(0)),
              offsetMs: 250,
            ),
            version: _version(),
            speed: 1,
            diagnostics: false,
            subtitleDelivery: delivery,
            onControls: (_) {},
            onSpeed: (_) {},
            onDiagnostics: (_) {},
            onClose: () {},
            dense: dense,
            subtitleSize: SubtitleSize.medium,
            onSubtitleSize: onSubtitleSize,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _pumpSheet(
  WidgetTester tester, {
  required PlayerPanel panel,
  required List<PlayerChoice> opened,
  required List<PlaybackControls> applied,
  int offsetMs = 0,
  SubtitleChoice subtitle = const SubtitleChoice.on(
    SubtitleSelection.embedded(0),
  ),
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: SizedBox(
          width: 320,
          height: 600,
          child: PlayerSettingsPanel(
            panel: panel,
            controls: PlaybackControls(subtitle: subtitle, offsetMs: offsetMs),
            version: _version(),
            speed: 1,
            diagnostics: false,
            subtitleDelivery: 'hls_vtt',
            onControls: applied.add,
            onSpeed: (_) {},
            onDiagnostics: (_) {},
            onClose: () {},
            openChoice: opened.add,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('in the sheet a dropdown is a row that opens its choices', (
    WidgetTester tester,
  ) async {
    final List<PlayerChoice> opened = <PlayerChoice>[];
    final List<PlaybackControls> applied = <PlaybackControls>[];
    await _pumpSheet(
      tester,
      panel: PlayerPanel.video,
      opened: opened,
      applied: applied,
    );

    expect(find.byType(AppDropdown<int?>), findsNothing);

    await tester.tap(find.text(Strings.playerQuality));
    await tester.pump();

    final PlayerChoice choice = opened.single;
    expect(choice.title, Strings.playerQuality);
    expect(choice.labels.first, Strings.playerOriginal);
    expect(choice.selected, 0);

    choice.onPick(choice.labels.indexOf(Strings.qualityRung(720)));
    await tester.pump();

    expect(applied.single.height, 720);
  });

  testWidgets('a change elsewhere leaves the subtitle as the panel was given '
      'it', (WidgetTester tester) async {
    final List<PlayerChoice> opened = <PlayerChoice>[];
    final List<PlaybackControls> applied = <PlaybackControls>[];
    await _pumpSheet(
      tester,
      panel: PlayerPanel.video,
      opened: opened,
      applied: applied,
      subtitle: const SubtitleChoice.auto(),
    );

    await tester.tap(find.text(Strings.playerQuality));
    await tester.pump();
    opened.single.onPick(
      opened.single.labels.indexOf(Strings.qualityRung(720)),
    );
    await tester.pump();

    expect(
      applied.single.subtitle,
      const SubtitleChoice.auto(),
      reason:
          'no subtitle is showing, but the server may still pick one on the '
          'next start; only choosing Off turns that off',
    );
  });

  testWidgets('choosing no subtitles turns them off', (
    WidgetTester tester,
  ) async {
    final List<PlayerChoice> opened = <PlayerChoice>[];
    final List<PlaybackControls> applied = <PlaybackControls>[];
    await _pumpSheet(
      tester,
      panel: PlayerPanel.subtitles,
      opened: opened,
      applied: applied,
    );

    await tester.tap(find.text(Strings.playerSubtitles));
    await tester.pump();
    opened.single.onPick(
      opened.single.labels.indexOf(Strings.playerNoSubtitles),
    );
    await tester.pump();

    expect(applied.last.subtitle, const SubtitleChoice.off());
  });

  testWidgets(
    'in the sheet every value and its chevron sit at the right edge',
    (WidgetTester tester) async {
      const double edge = 320 - 16;
      await _pumpSheet(
        tester,
        panel: PlayerPanel.video,
        opened: <PlayerChoice>[],
        applied: <PlaybackControls>[],
      );

      final Finder chevrons = find.byIcon(Icons.chevron_right);
      expect(chevrons, findsNWidgets(2));
      for (int i = 0; i < 2; i++) {
        expect(tester.getTopRight(chevrons.at(i)).dx, edge);
      }

      await _pumpSheet(
        tester,
        panel: PlayerPanel.subtitles,
        opened: <PlayerChoice>[],
        applied: <PlaybackControls>[],
        offsetMs: 250,
      );

      expect(
        tester.getTopRight(find.text(Strings.playerOffsetValue(250))).dx,
        edge,
      );
    },
  );

  testWidgets('in the sheet the offset comes last, after burn-in', (
    WidgetTester tester,
  ) async {
    await _pumpSheet(
      tester,
      panel: PlayerPanel.subtitles,
      opened: <PlayerChoice>[],
      applied: <PlaybackControls>[],
    );

    expect(
      tester.getTopLeft(find.text(Strings.playerOffset)).dy,
      greaterThan(tester.getTopLeft(find.text(Strings.playerBurnIn)).dy),
    );
  });

  testWidgets('in the sheet the offset buttons sit right under their label', (
    WidgetTester tester,
  ) async {
    await _pumpSheet(
      tester,
      panel: PlayerPanel.subtitles,
      opened: <PlayerChoice>[],
      applied: <PlaybackControls>[],
      offsetMs: 250,
    );

    final double label = tester
        .getBottomLeft(find.text(Strings.playerOffset))
        .dy;
    final double buttons = tester
        .getTopLeft(find.byTooltip(Strings.playerOffsetEarlier))
        .dy;

    expect(buttons - label, lessThan(Space.s2));
  });

  testWidgets('in the sheet the offset steps by a quarter second', (
    WidgetTester tester,
  ) async {
    final List<PlaybackControls> applied = <PlaybackControls>[];
    await _pumpSheet(
      tester,
      panel: PlayerPanel.subtitles,
      opened: <PlayerChoice>[],
      applied: applied,
      offsetMs: 250,
    );

    expect(find.text(Strings.playerOffsetValue(250)), findsOneWidget);

    await tester.tap(find.byTooltip(Strings.playerOffsetLater));
    await tester.pump();
    await tester.tap(find.byTooltip(Strings.playerOffsetEarlier));
    await tester.pump();
    await tester.tap(find.byTooltip(Strings.playerOffsetEarlier));
    await tester.pump();
    await tester.tap(find.byTooltip(Strings.playerOffsetEarlier));
    await tester.pump();
    await tester.tap(find.text(Strings.playerOffsetReset));
    await tester.pump();

    expect(applied.map((PlaybackControls c) => c.offsetMs), <int>[
      500,
      250,
      0,
      -250,
      0,
    ]);
  });

  test('an offset step stops at thirty seconds either way', () {
    expect(steppedOffset(29900, kOffsetStepMs), kOffsetBoundMs);
    expect(steppedOffset(-kOffsetBoundMs, -kOffsetStepMs), -kOffsetBoundMs);
    expect(steppedOffset(0, kOffsetStepMs), 250);
  });

  testWidgets('a soft stream offers the subtitle size and reports a change', (
    WidgetTester tester,
  ) async {
    final List<SubtitleSize> chosen = <SubtitleSize>[];
    await _pump(
      tester,
      delivery: 'hls_vtt',
      burn: false,
      onSubtitleSize: chosen.add,
    );

    expect(find.text(Strings.playerSubtitleSize), findsOneWidget);
    expect(find.text(Strings.subtitleSize), findsNothing);

    await tester.tap(find.text(Strings.subtitleSizeMedium));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.subtitleSizeSmall).last);
    await tester.pumpAndSettle();

    expect(chosen, <SubtitleSize>[SubtitleSize.small]);
  });

  testWidgets('a burned stream offers no size it cannot change', (
    WidgetTester tester,
  ) async {
    await _pump(tester, delivery: 'burned', burn: true, onSubtitleSize: (_) {});

    expect(find.text(Strings.playerSubtitleSize), findsNothing);
  });

  testWidgets('a soft stream offers the offset', (WidgetTester tester) async {
    await _pump(tester, delivery: 'hls_vtt', burn: false);

    expect(find.text(Strings.playerOffset), findsOneWidget);
    expect(find.text(Strings.playerBurnIn), findsOneWidget);
    expect(find.text(Strings.playerSubtitlesImageTrack), findsNothing);
  });

  testWidgets('a burned stream hides the offset it cannot move', (
    WidgetTester tester,
  ) async {
    await _pump(tester, delivery: 'burned', burn: true);

    expect(find.text(Strings.playerOffset), findsNothing);
    expect(find.text(Strings.playerBurnIn), findsOneWidget);
    expect(find.text(Strings.playerSubtitlesImageTrack), findsNothing);
  });

  testWidgets('a burn the user did not ask for explains itself', (
    WidgetTester tester,
  ) async {
    await _pump(tester, delivery: 'burned', burn: false);

    expect(find.text(Strings.playerOffset), findsNothing);
    expect(find.text(Strings.playerSubtitlesImageTrack), findsOneWidget);
  });

  testWidgets('every control answers to the row label beside it', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester, delivery: 'hls_vtt', burn: false);

    // The label is a sibling Text in the row, so the control itself is
    // otherwise anonymous.
    expect(
      tester.getSemantics(find.byType(TextField)).label,
      Strings.playerOffset,
    );
    // A switch row is one tappable strip that merges the label in, so naming
    // the switch as well would say it twice. Adding a help icon breaks that
    // merge, because the icon is its own tappable node, which is why a row
    // with help names its control explicitly instead.
    expect(
      tester.getSemantics(find.byType(Switch)).label,
      Strings.playerBurnIn,
    );

    semantics.dispose();
  });

  testWidgets('dense keeps every row and spends less height on them', (
    WidgetTester tester,
  ) async {
    await _pump(tester, delivery: 'hls_vtt', burn: false);
    final double roomy = tester
        .getSize(find.byType(PlayerSettingsPanel))
        .height;

    await _pump(tester, delivery: 'hls_vtt', burn: false, dense: true);
    final double packed = tester
        .getSize(find.byType(PlayerSettingsPanel))
        .height;

    expect(packed, lessThan(roomy));
    expect(find.text(Strings.playerOffset), findsOneWidget);
    expect(find.text(Strings.playerBurnIn), findsOneWidget);
    expect(
      tester.getRect(find.byType(Switch)).top -
          tester.getRect(find.byType(TextField)).bottom,
      greaterThanOrEqualTo(4),
      reason: 'dense is tighter spacing, not controls stacked flush together',
    );
  });

  testWidgets('the advanced section reports the mode and the container', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_settings(mode: 'remux', container: 'fmp4'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerAdvanced), findsOneWidget);
    expect(find.text('remux'), findsOneWidget);
    expect(find.text('fmp4'), findsOneWidget);
  });

  // A direct session produces no segments, so there is no container to name.
  testWidgets('a session with no segments says so', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_settings(mode: 'direct'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerContainerNone), findsOneWidget);
  });

  testWidgets('choosing a delivery preference reports it back', (
    WidgetTester tester,
  ) async {
    PlaybackControls? sent;
    await tester.pumpWidget(
      _settings(
        mode: 'direct',
        onControls: (PlaybackControls value) => sent = value,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerDeliveryAuto));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerDeliveryAlways).last);
    await tester.pumpAndSettle();

    expect(sent?.delivery, DeliveryPreference.always);
  });

  // Both controls are optional, so a player that cannot buffer must not show
  // knobs that do nothing.
  testWidgets('the buffer controls appear only when they are wired', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_settings(mode: 'direct'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerBufferTarget), findsNothing);
    expect(find.text(Strings.playerBufferLimit), findsNothing);
    expect(find.text(Strings.playerWaitForBuffer), findsNothing);
  });

  // The limit is what actually binds on high bitrate video, so it has to be
  // reachable rather than a constant the viewer cannot see.
  testWidgets('the memory limit is selectable and reports back', (
    WidgetTester tester,
  ) async {
    int? limit;
    await tester.pumpWidget(
      _settings(
        mode: 'direct',
        bufferBytes: 256 * 1024 * 1024,
        onBufferBytes: (int value) => limit = value,
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerBufferingSection), findsOneWidget);

    await tester.tap(find.text(Strings.playerBufferSize(256 * 1024 * 1024)));
    await tester.pumpAndSettle();
    await tester.tap(
      find.text(Strings.playerBufferSize(1024 * 1024 * 1024)).last,
    );
    await tester.pumpAndSettle();

    expect(limit, 1024 * 1024 * 1024);
  });

  test('the limit reads as megabytes until it reaches a gigabyte', () {
    expect(Strings.playerBufferSize(64 * 1024 * 1024), '64 MB');
    expect(Strings.playerBufferSize(512 * 1024 * 1024), '512 MB');
    expect(Strings.playerBufferSize(1024 * 1024 * 1024), '1 GB');
  });

  testWidgets('choosing a buffer target and asking to wait both report back', (
    WidgetTester tester,
  ) async {
    int? target;
    bool? wait;
    await tester.pumpWidget(
      _settings(
        mode: 'direct',
        bufferSeconds: 60,
        onBufferSeconds: (int value) => target = value,
        waitForBuffer: false,
        onWaitForBuffer: (bool value) => wait = value,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerBufferDuration(60)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerBufferDuration(600)).last);
    await tester.pumpAndSettle();

    expect(target, 600);

    // The switch ignores pointers and the row around it carries the tap, so
    // the row is what a viewer actually presses.
    await tester.tap(find.text(Strings.playerWaitForBuffer));
    await tester.pumpAndSettle();

    expect(wait, isTrue);
  });

  // The help icon sits inside a row whose own tap toggles the switch, so it
  // has to win the gesture rather than flipping the setting it explains.
  testWidgets('help opens on a control without changing it', (
    WidgetTester tester,
  ) async {
    bool? wait;
    await tester.pumpWidget(
      _settings(
        mode: 'direct',
        bufferSeconds: 60,
        onBufferSeconds: (_) {},
        waitForBuffer: false,
        onWaitForBuffer: (bool value) => wait = value,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.descendant(
        of: find
            .ancestor(
              of: find.text(Strings.playerWaitForBuffer),
              matching: find.byType(Row),
            )
            .first,
        matching: find.byType(FieldHelp),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerWaitForBufferHelp), findsOneWidget);
    expect(
      wait,
      isNull,
      reason: 'asking what a setting does must not change it',
    );
  });

  testWidgets('the subtitles panel offers one button to add subtitles', (
    WidgetTester tester,
  ) async {
    int opened = 0;
    await tester.pumpWidget(_subtitles(onAddSubtitles: () => opened++));
    await tester.pumpAndSettle();

    expect(find.text(Strings.searchSubtitles), findsNothing);
    expect(find.text(Strings.transcribe), findsNothing);
    final Rect label = tester.getRect(find.text(Strings.moreSubtitles));
    final Rect button = tester.getRect(
      find.widgetWithText(TextButton, Strings.addSubtitles),
    );
    expect(button.left, greaterThan(label.right));
    expect(button.center.dy, closeTo(label.center.dy, 2));
    expect(
      find.descendant(
        of: find
            .ancestor(
              of: find.text(Strings.moreSubtitles),
              matching: find.byType(Row),
            )
            .first,
        matching: find.byType(FieldHelp),
      ),
      findsOneWidget,
    );

    await tester.tap(find.text(Strings.addSubtitles));
    await tester.pump();

    expect(opened, 1);
  });

  testWidgets('no button is offered without a way to add subtitles', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_subtitles());
    await tester.pumpAndSettle();

    expect(find.text(Strings.addSubtitles), findsNothing);
  });

  testWidgets('the playback log sits under diagnostics and opens on Show', (
    WidgetTester tester,
  ) async {
    int shown = 0;
    await tester.pumpWidget(_settings(mode: 'hls', onShowLog: () => shown++));
    await tester.pumpAndSettle();

    expect(
      tester.getTopLeft(find.text(Strings.playerLog)).dy,
      greaterThan(tester.getTopLeft(find.text(Strings.playerDiagnostics)).dy),
    );

    await tester.ensureVisible(find.text(Strings.playerLogShow));
    await tester.tap(find.text(Strings.playerLogShow));
    await tester.pump();

    expect(shown, 1);
  });

  testWidgets('no playback log row without a way to show it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_settings(mode: 'hls'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerLog), findsNothing);
  });

  testWidgets('while a change is in flight the quality is blocked and the '
      'speed is not', (WidgetTester tester) async {
    final List<PlaybackControls> sent = <PlaybackControls>[];
    final List<double> speeds = <double>[];
    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.video,
        busy: true,
        onControls: sent.add,
        onSpeed: speeds.add,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerOriginal), warnIfMissed: false);
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerOriginal), findsOneWidget);
    await tester.tap(find.text(Strings.playerNormalSpeed));
    await tester.pumpAndSettle();
    await tester.tap(find.text('1.5x').last);
    await tester.pumpAndSettle();

    expect(sent, isEmpty);
    expect(speeds, <double>[1.5]);
  });

  testWidgets('while a change is in flight the subtitle settings are blocked '
      'and their size is not', (WidgetTester tester) async {
    final List<PlaybackControls> sent = <PlaybackControls>[];
    final List<SubtitleSize> sizes = <SubtitleSize>[];
    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.subtitles,
        busy: true,
        onControls: sent.add,
        onSubtitleSize: sizes.add,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerBurnIn), warnIfMissed: false);
    await tester.pumpAndSettle();
    await tester.tap(find.text(SubtitleSize.medium.label));
    await tester.pumpAndSettle();
    await tester.tap(find.text(SubtitleSize.large.label).last);
    await tester.pumpAndSettle();

    expect(sent, isEmpty);
    expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);
    expect(sizes, <SubtitleSize>[SubtitleSize.large]);
  });

  testWidgets('while a change is in flight the converting choice is blocked '
      'and diagnostics are not', (WidgetTester tester) async {
    final List<PlaybackControls> sent = <PlaybackControls>[];
    final List<bool> diagnostics = <bool>[];
    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.settings,
        busy: true,
        onControls: sent.add,
        onDiagnostics: diagnostics.add,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.text(Strings.playerDeliveryAuto),
      warnIfMissed: false,
    );
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerDeliveryAlways), findsNothing);
    await tester.tap(find.text(Strings.playerDiagnostics));
    await tester.pumpAndSettle();

    expect(sent, isEmpty);
    expect(diagnostics, <bool>[true]);
  });

  testWidgets('settings changed from outside show while the panel is open', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.subtitles,
        controls: const PlaybackControls(
          subtitle: SubtitleChoice.on(SubtitleSelection.embedded(0)),
          offsetMs: 250,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller?.text,
      '250',
    );

    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.subtitles,
        controls: const PlaybackControls(
          subtitle: SubtitleChoice.on(SubtitleSelection.embedded(0)),
          burn: true,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      tester.widget<TextField>(find.byType(TextField)).controller?.text,
      '',
    );
    expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);
  });

  testWidgets('a choice the parent does not take goes back to what it has', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_panel(panel: PlayerPanel.audio));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerStereoDownmix));
    await tester.pumpAndSettle();
    expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);

    await tester.pumpWidget(_panel(panel: PlayerPanel.audio));
    await tester.pumpAndSettle();

    expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);
  });

  testWidgets('a rebuild with the same settings keeps what is being typed', (
    WidgetTester tester,
  ) async {
    const PlaybackControls controls = PlaybackControls(
      subtitle: SubtitleChoice.on(SubtitleSelection.embedded(0)),
    );
    await tester.pumpWidget(
      _panel(panel: PlayerPanel.subtitles, controls: controls),
    );
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '500');

    await tester.pumpWidget(
      _panel(panel: PlayerPanel.subtitles, controls: controls),
    );
    await tester.pumpAndSettle();

    expect(
      tester.widget<TextField>(find.byType(TextField)).controller?.text,
      '500',
    );
  });

  testWidgets('a converting choice that was never set is not sent back as '
      'one', (WidgetTester tester) async {
    final List<PlaybackControls> sent = <PlaybackControls>[];
    await tester.pumpWidget(
      _panel(panel: PlayerPanel.audio, onControls: sent.add),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerStereoDownmix));
    await tester.pumpAndSettle();

    expect(sent.single.downmix, isTrue);
    expect(sent.single.delivery, isNull);
  });

  testWidgets('the audio panel shows the sound rows that apply', (
    WidgetTester tester,
  ) async {
    const List<String> labels = <String>[
      Strings.playerStereoDownmix,
      Strings.playerPreventDistortion,
      Strings.playerDialogue,
      Strings.playerNightMode,
    ];
    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.audio,
        sound: (downmix: true, normalize: true, dialogue: true, night: true),
      ),
    );
    await tester.pumpAndSettle();
    for (final String label in labels) {
      expect(find.text(label), findsOneWidget, reason: label);
    }

    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.audio,
        sound: (
          downmix: false,
          normalize: false,
          dialogue: false,
          night: false,
        ),
      ),
    );
    await tester.pumpAndSettle();
    for (final String label in labels) {
      expect(find.text(label), findsNothing, reason: label);
    }
  });

  test('Converting and every sound row say what they do in one sentence', () {
    for (final String help in <String>[
      Strings.playerDeliveryHelp,
      Strings.playerStereoDownmixHelp,
      Strings.playerPreventDistortionHelp,
      Strings.playerDialogueHelp,
      Strings.playerNightModeHelp,
    ]) {
      expect(help, endsWith('.'), reason: help);
      expect(help.indexOf('.'), help.length - 1, reason: help);
    }
  });

  testWidgets('prevent distortion starts on and turning it off is sent', (
    WidgetTester tester,
  ) async {
    final List<PlaybackControls> sent = <PlaybackControls>[];
    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.audio,
        onControls: sent.add,
        sound: (downmix: false, normalize: true, dialogue: false, night: false),
      ),
    );
    await tester.pumpAndSettle();
    expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);

    await tester.tap(find.text(Strings.playerPreventDistortion));
    await tester.pumpAndSettle();

    expect(sent.single.normalize, isFalse);
  });

  testWidgets('the dialogue lift and night mode are chosen in the panel', (
    WidgetTester tester,
  ) async {
    final List<PlaybackControls> sent = <PlaybackControls>[];
    await tester.pumpWidget(
      _panel(
        panel: PlayerPanel.audio,
        onControls: sent.add,
        sound: (downmix: false, normalize: false, dialogue: true, night: true),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byType(AppDropdown<DialogueLift>));
    await tester.pumpAndSettle();
    await tester.tap(find.text(DialogueLift.db9.label).last);
    await tester.pumpAndSettle();
    expect(sent.last.dialogue, DialogueLift.db9);

    await tester.tap(find.byType(AppDropdown<NightMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text(NightMode.light.label).last);
    await tester.pumpAndSettle();
    expect(sent.last.night, NightMode.light);
    expect(sent.last.dialogue, DialogueLift.db9);
  });

  testWidgets('no label is cut on the narrowest sheet or panel', (
    WidgetTester tester,
  ) async {
    await tester.runAsync(
      () => loadBundledFonts(BundlePlatform.linux, rootBundle),
    );
    tester.view.physicalSize = const Size(1200, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final List<(double, bool, bool)> layouts = <(double, bool, bool)>[
      (kSheetMinWidth, true, false),
      (340, false, false),
      (340, false, true),
    ];
    for (final (double width, bool sheet, bool dense) in layouts) {
      for (final PlayerPanel panel in PlayerPanel.values) {
        await tester.pumpWidget(
          _everyRow(panel: panel, width: width, sheet: sheet, dense: dense),
        );
        await tester.pumpAndSettle();

        final Iterable<RenderParagraph> cut = tester
            .renderObjectList<RenderParagraph>(find.byType(RichText))
            .where((RenderParagraph p) => p.didExceedMaxLines);
        expect(
          cut.map((RenderParagraph p) => p.text.toPlainText()),
          isEmpty,
          reason: '${panel.name} at $width, sheet $sheet, dense $dense',
        );
      }
    }
  });
}

Widget _everyRow({
  required PlayerPanel panel,
  required double width,
  required bool sheet,
  required bool dense,
}) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(
        width: width,
        height: 2000,
        child: PlayerSettingsPanel(
          panel: panel,
          controls: const PlaybackControls(
            subtitle: SubtitleChoice.on(SubtitleSelection.embedded(0)),
          ),
          version: _version(),
          speed: 1,
          diagnostics: false,
          mode: 'transcode',
          container: 'mpegts',
          subtitleDelivery: 'hls_vtt',
          pictureInPicture: false,
          onPictureInPicture: () {},
          autoplaySeconds: 10,
          onAutoplaySeconds: (_) {},
          subtitleSize: SubtitleSize.medium,
          onSubtitleSize: (_) {},
          networkTimeoutSeconds: 30,
          onNetworkTimeoutSeconds: (_) {},
          bufferSeconds: 30,
          onBufferSeconds: (_) {},
          bufferBytes: 150 * 1024 * 1024,
          onBufferBytes: (_) {},
          waitForBuffer: false,
          onWaitForBuffer: (_) {},
          onShortcuts: () {},
          onShowLog: () {},
          onAddSubtitles: () {},
          openChoice: sheet ? (_) {} : null,
          dense: dense,
          onControls: (_) {},
          onSpeed: (_) {},
          onDiagnostics: (_) {},
          onClose: () {},
        ),
      ),
    ),
  ),
);

Widget _panel({
  required PlayerPanel panel,
  PlaybackControls controls = const PlaybackControls(
    subtitle: SubtitleChoice.on(SubtitleSelection.embedded(0)),
  ),
  bool busy = false,
  ValueChanged<PlaybackControls>? onControls,
  ValueChanged<double>? onSpeed,
  ValueChanged<bool>? onDiagnostics,
  ValueChanged<SubtitleSize>? onSubtitleSize,
  SoundRows sound = (
    downmix: true,
    normalize: false,
    dialogue: false,
    night: false,
  ),
}) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: SizedBox(
      width: 420,
      child: PlayerSettingsPanel(
        panel: panel,
        controls: controls,
        version: _version(),
        speed: 1,
        diagnostics: false,
        subtitleDelivery: 'hls_vtt',
        subtitleSize: SubtitleSize.medium,
        onSubtitleSize: onSubtitleSize ?? (_) {},
        sound: sound,
        busy: busy,
        onControls: onControls ?? (_) {},
        onSpeed: onSpeed ?? (_) {},
        onDiagnostics: onDiagnostics ?? (_) {},
        onClose: () {},
      ),
    ),
  ),
);

Widget _subtitles({VoidCallback? onAddSubtitles}) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: SizedBox(
      width: 420,
      child: PlayerSettingsPanel(
        panel: PlayerPanel.subtitles,
        controls: const PlaybackControls(),
        version: _version(),
        speed: 1,
        diagnostics: false,
        onAddSubtitles: onAddSubtitles,
        onControls: (_) {},
        onSpeed: (_) {},
        onDiagnostics: (_) {},
        onClose: () {},
      ),
    ),
  ),
);

Widget _settings({
  required String mode,
  String? container,
  ValueChanged<PlaybackControls>? onControls,
  int? bufferSeconds,
  ValueChanged<int>? onBufferSeconds,
  int? bufferBytes,
  ValueChanged<int>? onBufferBytes,
  bool? waitForBuffer,
  ValueChanged<bool>? onWaitForBuffer,
  VoidCallback? onShowLog,
}) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: SizedBox(
      width: 420,
      child: PlayerSettingsPanel(
        panel: PlayerPanel.settings,
        controls: const PlaybackControls(),
        version: _version(),
        speed: 1,
        diagnostics: false,
        mode: mode,
        container: container,
        bufferSeconds: bufferSeconds,
        onBufferSeconds: onBufferSeconds,
        bufferBytes: bufferBytes,
        onBufferBytes: onBufferBytes,
        waitForBuffer: waitForBuffer,
        onWaitForBuffer: onWaitForBuffer,
        onShowLog: onShowLog,
        onControls: onControls ?? (_) {},
        onSpeed: (_) {},
        onDiagnostics: (_) {},
        onClose: () {},
      ),
    ),
  ),
);
