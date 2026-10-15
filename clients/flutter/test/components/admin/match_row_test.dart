import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/match_row.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

const String _kLong =
    '1080p · mkv · /media/movies/Paper Skies (2019)/'
    'Paper.Skies.2019.REMASTERED.1080p.BluRay.x264-SOMEGROUP.mkv';

Future<void> _pump(WidgetTester tester, {VoidCallback? onTap}) =>
    tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: SizedBox(
            width: 240,
            child: Column(
              children: <Widget>[
                MatchRow(label: 'Short', tooltip: 'Benchmark', onTap: () {}),
                MatchRow(
                  label: _kLong,
                  tooltip: 'Benchmark',
                  onTap: onTap ?? () {},
                ),
              ],
            ),
          ),
        ),
      ),
    );

void main() {
  testWidgets('a long label wraps onto more lines instead of being cut', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(
      tester.getSize(find.text(_kLong)).height,
      greaterThan(tester.getSize(find.text('Short')).height * 2),
    );
  });

  testWidgets('a long press still names the action', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.longPress(find.text(_kLong));
    await tester.pumpAndSettle();

    expect(find.text('Benchmark'), findsOneWidget);
  });

  testWidgets('a tap on a wrapped row picks it', (WidgetTester tester) async {
    int taps = 0;
    await _pump(tester, onTap: () => taps++);

    await tester.tap(find.text(_kLong));

    expect(taps, 1);
  });
}
