import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/fact.dart';
import 'package:shadowmask/components/fact_readout.dart';
import 'package:shadowmask/components/facts_row.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

const List<Fact> _movie = <Fact>[
  Fact(Strings.factYear, '2019'),
  Fact(Strings.factRuntime, '2h 8m'),
  Fact(Strings.factRating, 'PG-13'),
  Fact(Strings.factQuality, 'UHD'),
];

List<String> _values(List<Fact> facts) => <String>[
  for (final Fact f in facts) f.value,
];

Future<void> _pump(WidgetTester tester, AppThemeVariant variant) =>
    tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(variant),
        home: const Scaffold(
          body: FactsRow(_movie, labels: false, readouts: true),
        ),
      ),
    );

void main() {
  group('readoutsOf', () {
    test('year and quality become read-outs and the rest stays text', () {
      final ({List<Fact> readouts, List<Fact> rest}) split =
          FactsRow.readoutsOf(_movie);

      expect(_values(split.readouts), <String>['2019', 'UHD']);
      expect(_values(split.rest), <String>['2h 8m', 'PG-13']);
    });

    test('facts without a year or quality stay text', () {
      final ({List<Fact> readouts, List<Fact> rest}) split =
          FactsRow.readoutsOf(const <Fact>[
            Fact(Strings.factRating, 'TV-14'),
            Fact(Strings.watchedLabel, '3 of 10'),
          ]);

      expect(split.readouts, isEmpty);
      expect(_values(split.rest), <String>['TV-14', '3 of 10']);
    });

    test('a year alone is the only read-out', () {
      final ({List<Fact> readouts, List<Fact> rest}) split =
          FactsRow.readoutsOf(const <Fact>[
            Fact(Strings.factYear, '2016'),
            Fact(Strings.factRating, 'TV-14'),
          ]);

      expect(_values(split.readouts), <String>['2016']);
      expect(_values(split.rest), <String>['TV-14']);
    });
  });

  testWidgets('hologram reads year and quality out of hexagons', (
    WidgetTester tester,
  ) async {
    await _pump(tester, AppThemeVariant.hologram);

    expect(
      tester
          .widgetList<FactReadout>(find.byType(FactReadout))
          .map((FactReadout r) => r.fact.value),
      <String>['2019', 'UHD'],
    );
    expect(find.text('2h 8m'), findsOneWidget);
    expect(find.text('PG-13'), findsOneWidget);
  });

  testWidgets(
    'hologram keeps facts as text where read-outs are not asked for',
    (WidgetTester tester) async {
      await tester.pumpWidget(
        MaterialApp(
          theme: buildTheme(AppThemeVariant.hologram),
          home: const Scaffold(body: FactsRow(_movie, labels: false)),
        ),
      );

      expect(find.byType(FactReadout), findsNothing);
      for (final String value in _values(_movie)) {
        expect(find.text(value), findsOneWidget);
      }
    },
  );

  for (final AppThemeVariant variant in AppThemeVariant.values.where(
    (AppThemeVariant v) => v != AppThemeVariant.hologram,
  )) {
    testWidgets('${variant.name} keeps every fact on the text line', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant);

      expect(find.byType(FactReadout), findsNothing);
      for (final String value in _values(_movie)) {
        expect(find.text(value), findsOneWidget);
      }
    });
  }
}
