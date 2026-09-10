import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/playback_controls.dart';

VersionDetail _version() => VersionDetail.fromJson(<String, dynamic>{
  'id': 'v1',
  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
  'library_id': 'l',
  'quality': 'fhd',
  'container': 'mkv',
  'subtitles': <dynamic>[
    <String, dynamic>{
      'index': 0,
      'language': 'eng',
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
              subtitle: const SubtitleSelection.embedded(0),
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
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
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
}

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
        onControls: onControls ?? (_) {},
        onSpeed: (_) {},
        onDiagnostics: (_) {},
        onClose: () {},
      ),
    ),
  ),
);
