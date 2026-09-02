import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/library_api.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/admin/library_form_dialog.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

Library _existing() => const Library(
  id: 'lib1',
  name: 'Films',
  kind: LibraryKind.movie,
  origin: LibraryOrigin.external,
  roots: <String>['/media/films'],
  watcher: WatcherStrategy.manual,
  createdAt: '2026-08-17T09:00:00Z',
  updatedAt: '2026-08-17T09:00:00Z',
);

Future<List<http.Request>> _pump(
  WidgetTester tester, {
  Library? existing,
}) async {
  final List<http.Request> seen = <http.Request>[];
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen.add(req);
      return http.Response('', 204);
    }),
  );
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: ToastHost(
          child: LibraryFormDialog(
            libraries: LibraryApi(api),
            existing: existing,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return seen;
}

Finder _originField() => find.descendant(
  of: find.byType(LabelledDropdown<LibraryOrigin>),
  matching: find.byType(MenuField),
);

Finder _originText(String label) => find.descendant(
  of: find.byType(LabelledDropdown<LibraryOrigin>),
  matching: find.text(label),
);

Finder _kindField() => find.descendant(
  of: find.byType(LabelledDropdown<LibraryKind>),
  matching: find.byType(MenuField),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('kind and origin sit at the same height', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(
      tester.getTopLeft(_kindField()).dy,
      tester.getTopLeft(_originField()).dy,
    );
  });

  testWidgets('kind explains itself without claiming to be permanent', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(
      find
          .descendant(
            of: find.byType(LabelledDropdown<LibraryKind>),
            matching: find.byTooltip(Strings.whatIsThis),
          )
          .first,
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.kindHelp), findsOneWidget);
    expect(Strings.kindHelp, isNot(contains('cannot be changed')));
  });

  testWidgets('an empty name reports inline and does not submit', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester);

    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(find.text(Strings.requiredName), findsOneWidget);
    expect(seen, isEmpty);
    expect(find.byType(SnackBar), findsNothing);
  });

  testWidgets('typing a name clears the inline error', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();
    expect(find.text(Strings.requiredName), findsOneWidget);

    await tester.enterText(fieldNamed(Strings.fieldName), 'Films');
    await tester.pumpAndSettle();

    expect(find.text(Strings.requiredName), findsNothing);
  });

  testWidgets('the fields sit in the six requested rows', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    Offset at(String label) => tester.getTopLeft(find.text(label));

    expect(at(Strings.fieldName).dy, lessThan(at(Strings.fieldKind).dy));
    expect(at(Strings.fieldKind).dy, closeTo(at(Strings.fieldOrigin).dy, 8));
    expect(at(Strings.fieldKind).dx, lessThan(at(Strings.fieldOrigin).dx));
    expect(at(Strings.fieldKind).dy, lessThan(at(Strings.fieldWatcher).dy));
    expect(
      at(Strings.fieldWatcher).dy,
      closeTo(at(Strings.fieldScanSchedule).dy, 8),
    );
    expect(
      at(Strings.fieldWatcher).dx,
      lessThan(at(Strings.fieldScanSchedule).dx),
    );
    expect(at(Strings.fieldWatcher).dy, lessThan(at(Strings.fieldRoots).dy));
    expect(
      at(Strings.fieldRoots).dy,
      lessThan(at(Strings.fieldMetadataSources).dy),
    );
    expect(
      at(Strings.fieldMetadataSources).dy,
      lessThan(at(Strings.fieldSortArticles).dy),
    );
  });

  testWidgets('creating offers origin as a live dropdown', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(_originText('Local'), findsOneWidget);

    await tester.tap(_originField());
    await tester.pumpAndSettle();

    expect(find.text('External'), findsWidgets);
  });

  testWidgets('editing keeps origin in place but inert', (
    WidgetTester tester,
  ) async {
    await _pump(tester, existing: _existing());

    expect(find.text(Strings.fieldOrigin), findsOneWidget);
    expect(_originText('External'), findsOneWidget);

    await tester.tap(_originField());
    await tester.pumpAndSettle();

    expect(_originText('Local'), findsNothing);
    expect(_originText('External'), findsOneWidget);
  });

  testWidgets('the disabled origin field is not an ink response', (
    WidgetTester tester,
  ) async {
    await _pump(tester, existing: _existing());

    expect(tester.widget<MenuField>(_originField()).enabled, isFalse);
  });

  testWidgets('editing does not send an origin the server cannot change', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester, existing: _existing());

    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(seen, hasLength(1));
    expect(seen.single.body, isNot(contains('origin')));

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });
}
