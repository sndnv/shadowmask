import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/player/player_sheet.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Future<void> _pump(
  WidgetTester tester, {
  List<int>? picked,
  VoidCallback? onClose,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Align(
          alignment: Alignment.centerRight,
          child: SizedBox(
            width: 360,
            height: 400,
            child: PlayerSheet(
              section: (PlayerPanel panel, ChoiceOpener open) => Column(
                children: <Widget>[
                  Text('section ${panel.name}'),
                  TextButton(
                    onPressed: () => open(
                      PlayerChoice(
                        title: 'Pick one',
                        labels: const <String>['first', 'second'],
                        selected: 0,
                        onPick: (int i) => picked?.add(i),
                      ),
                    ),
                    child: const Text('choose'),
                  ),
                ],
              ),
              summary: (PlayerPanel panel) => 'now ${panel.name}',
              onClose: onClose ?? () {},
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('the first page lists the four sections with their values', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.playerOptions), findsOneWidget);
    expect(find.byTooltip(Strings.back), findsNothing);
    for (final PlayerPanel panel in PlayerPanel.values) {
      expect(find.text(panel.title), findsOneWidget);
      expect(find.text('now ${panel.name}'), findsOneWidget);
      expect(
        tester
            .getSize(
              find
                  .ancestor(
                    of: find.text(panel.title),
                    matching: find.byType(InkWell),
                  )
                  .first,
            )
            .height,
        kSheetSectionHeight,
      );
    }
  });

  testWidgets('a section opens in place and the back arrow returns', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(find.text(PlayerPanel.audio.title));
    await tester.pumpAndSettle();

    expect(find.text('section audio'), findsOneWidget);
    expect(find.text(PlayerPanel.audio.title), findsOneWidget);

    await tester.tap(find.byTooltip(Strings.back));
    await tester.pumpAndSettle();

    expect(find.text('section audio'), findsNothing);
    expect(find.text(Strings.playerOptions), findsOneWidget);
  });

  testWidgets('a choice opens a list, and a pick applies it and returns', (
    WidgetTester tester,
  ) async {
    final List<int> picked = <int>[];
    await _pump(tester, picked: picked);
    await tester.tap(find.text(PlayerPanel.video.title));
    await tester.pumpAndSettle();

    await tester.tap(find.text('choose'));
    await tester.pumpAndSettle();

    expect(find.text('Pick one'), findsOneWidget);
    expect(find.text('section video'), findsNothing);
    expect(find.byIcon(Icons.check), findsOneWidget);

    await tester.tap(find.text('second'));
    await tester.pumpAndSettle();

    expect(picked, <int>[1]);
    expect(find.text('section video'), findsOneWidget);
  });

  testWidgets('back from a choice list returns to its section', (
    WidgetTester tester,
  ) async {
    final List<int> picked = <int>[];
    await _pump(tester, picked: picked);
    await tester.tap(find.text(PlayerPanel.video.title));
    await tester.pumpAndSettle();
    await tester.tap(find.text('choose'));
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip(Strings.back));
    await tester.pumpAndSettle();

    expect(picked, isEmpty);
    expect(find.text('section video'), findsOneWidget);
  });

  testWidgets('the close button closes the sheet', (WidgetTester tester) async {
    int closed = 0;
    await _pump(tester, onClose: () => closed++);

    await tester.tap(find.byTooltip(Strings.close));

    expect(closed, 1);
  });

  test('the sheet takes 40 % of the width, from 320 to 420', () {
    expect(playerSheetWidth(600), kSheetMinWidth);
    expect(playerSheetWidth(900), 360);
    expect(playerSheetWidth(1400), kSheetMaxWidth);
  });
}
