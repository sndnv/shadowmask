import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/genre_chips.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

const List<Genre> _genres = <Genre>[Genre(id: 'g1', name: 'Science Fiction')];

Future<String?> _tapChip(WidgetTester tester, {String? basePath}) async {
  String? pushed;
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings s) {
          pushed ??= s.name == '/' ? null : s.name;
          return MaterialPageRoute<void>(
            builder: (_) =>
                Scaffold(body: GenreChips(_genres, basePath: basePath)),
          );
        },
      ),
    ),
  );
  await tester.tap(find.text('Science Fiction'));
  await tester.pumpAndSettle();
  return pushed;
}

void main() {
  testWidgets('a chip opens the section list filtered on that genre', (
    WidgetTester tester,
  ) async {
    expect(
      await _tapChip(tester, basePath: moviesRoute()),
      '/movies?genres=Science+Fiction',
    );
  });

  testWidgets('a chip without a base path does not navigate', (
    WidgetTester tester,
  ) async {
    expect(await _tapChip(tester), isNull);
  });
}
