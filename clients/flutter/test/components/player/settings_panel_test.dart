import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
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
    // A switch row is one tappable strip that already merges the label in,
    // so naming the switch as well would say it twice.
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
}
