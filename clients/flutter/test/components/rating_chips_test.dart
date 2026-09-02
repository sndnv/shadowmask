import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/link_chip.dart';
import 'package:shadowmask/components/rating_chips.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

Future<void> _pump(WidgetTester tester, List<Rating> ratings) =>
    tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(body: RatingChips(ratings)),
      ),
    );

Tokens _tokens(WidgetTester tester) =>
    tester.element(find.byType(RatingChips)).tokens;

Text _text(WidgetTester tester, String value) =>
    tester.widget<Text>(find.text(value));

void main() {
  testWidgets('each source keeps its own scale', (WidgetTester tester) async {
    await _pump(tester, const <Rating>[
      Rating(source: 'Internet Movie Database', value: 8.8),
      Rating(source: 'Rotten Tomatoes', value: 87),
      Rating(source: 'Metacritic', value: 74),
    ]);

    expect(find.text('IMDb:'), findsOneWidget);
    expect(find.text('8.8/10'), findsOneWidget);
    expect(find.text('Rotten Tomatoes:'), findsOneWidget);
    expect(find.text('87%'), findsOneWidget);
    expect(find.text('Metacritic:'), findsOneWidget);
    expect(find.text('74/100'), findsOneWidget);
  });

  testWidgets('the score is bolder than its source', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const <Rating>[
      Rating(source: 'Metacritic', value: 74),
    ]);

    expect(_text(tester, '74/100').style?.fontWeight, FontWeight.w700);
    expect(_text(tester, 'Metacritic:').style?.fontWeight, FontWeight.w500);
  });

  testWidgets('the colour names the source, and the score stays neutral', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const <Rating>[
      Rating(source: 'Internet Movie Database', value: 8.8),
      Rating(source: 'Rotten Tomatoes', value: 87),
      Rating(source: 'Metacritic', value: 74),
    ]);
    final Tokens t = _tokens(tester);

    expect(_text(tester, 'IMDb:').style?.color, t.warn);
    expect(_text(tester, 'Rotten Tomatoes:').style?.color, t.danger);
    expect(_text(tester, 'Metacritic:').style?.color, t.ok);

    expect(_text(tester, '8.8/10').style?.color, t.text);
    expect(_text(tester, '87%').style?.color, t.text);
    expect(_text(tester, '74/100').style?.color, t.text);
  });

  testWidgets('a low score does not change its source colour', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const <Rating>[
      Rating(source: 'Metacritic', value: 12),
    ]);

    expect(_text(tester, 'Metacritic:').style?.color, _tokens(tester).ok);
  });

  testWidgets('an unknown source falls back to the plain chip', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const <Rating>[Rating(source: 'Letterboxd', value: 4)]);

    expect(_text(tester, 'Letterboxd:').style?.color, _tokens(tester).text);
    expect(tester.widget<LinkChip>(find.byType(LinkChip)).labelColor, isNull);
  });

  testWidgets('a score chip goes nowhere, so it never lights', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const <Rating>[
      Rating(source: 'Internet Movie Database', value: 8.8),
    ]);

    expect(tester.widget<LinkChip>(find.byType(LinkChip)).onTap, isNull);
  });

  testWidgets('no scores renders nothing', (WidgetTester tester) async {
    await _pump(tester, const <Rating>[]);

    expect(find.byType(LinkChip), findsNothing);
  });
}
