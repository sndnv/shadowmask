import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/edit_metadata_dialog.dart';
import 'package:shadowmask/components/admin/edited_notice.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/model/server/rating_system.dart';
import 'package:shadowmask/components/toast_host.dart'
    show ToastHost, kToastDuration;
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

Future<List<http.Request>> _pump(
  WidgetTester tester,
  EditMetadataDialog Function(AdminApi admin) build,
) async {
  final List<http.Request> seen = <http.Request>[];
  tester.view.physicalSize = const Size(900, 1600);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);
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
      builder: (BuildContext context, Widget? child) =>
          ToastHost(child: child ?? const SizedBox.shrink()),
      home: Scaffold(body: build(AdminApi(api))),
    ),
  );
  await tester.pumpAndSettle();
  return seen;
}

const List<RatingSystem> _systems = <RatingSystem>[
  RatingSystem(system: 'mpaa', codes: <String>['pg-13', 'r']),
  RatingSystem(system: 'bbfc', codes: <String>['12a', '15']),
];

EditMetadataDialog _movie(AdminApi admin) => EditMetadataDialog(
  admin: admin,
  target: EditTarget.movie,
  id: 'm1',
  title: 'Alpha',
  year: 2020,
  overview: 'first',
  runtimeMinutes: 100,
  ratingSystem: 'mpaa',
  ratingCode: 'R',
  ratingSystems: _systems,
  tmdbId: 'movie/603',
  imdbId: 'tt0133093',
);

EditMetadataDialog _series(AdminApi admin) => EditMetadataDialog(
  admin: admin,
  target: EditTarget.series,
  id: 's1',
  title: 'Gamma',
  year: 2019,
  ratingSystems: _systems,
);

EditMetadataDialog _episode(AdminApi admin) => EditMetadataDialog(
  admin: admin,
  target: EditTarget.episode,
  id: 'e1',
  seriesId: 's1',
  seasonId: 'se1',
  title: 'Pilot',
  runtimeMinutes: 42,
  airDate: '2010-10-01',
);

Finder _dropdownNamed(String label) => find.descendant(
  of: find.ancestor(of: find.text(label), matching: find.byType(FieldLabel)),
  matching: find.byType(MenuField),
);

Finder _confirmSave() => find.descendant(
  of: find.ancestor(
    of: find.text(Strings.confirmSaveEditBody),
    matching: find.byType(DialogShell),
  ),
  matching: find.widgetWithText(FilledButton, Strings.save),
);

Future<void> _save(WidgetTester tester) async {
  await tester.tap(find.widgetWithText(FilledButton, Strings.save));
  await tester.pumpAndSettle();
  if (_confirmSave().evaluate().isNotEmpty) {
    await tester.tap(_confirmSave());
    await tester.pumpAndSettle();
  }
  await tester.pump(kToastDuration + const Duration(milliseconds: 100));
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a movie form shows year and runtime but no air date', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _movie);

    expect(fieldNamed(Strings.fieldYear), findsOneWidget);
    expect(fieldNamed(Strings.fieldRuntimeMinutes), findsOneWidget);
    expect(find.byType(LabelledDropdown<String>), findsNWidgets(2));
    expect(fieldNamed(Strings.fieldAirDate), findsNothing);
  });

  testWidgets('the fields run title, overview, then the paired rows', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _movie);

    double topOf(String label) => tester.getTopLeft(fieldNamed(label)).dy;

    expect(topOf(Strings.fieldTitle), lessThan(topOf(Strings.fieldOverview)));
    expect(topOf(Strings.fieldOverview), lessThan(topOf(Strings.fieldYear)));
    expect(topOf(Strings.fieldYear), lessThan(topOf(Strings.fieldTmdbId)));
    expect(
      topOf(Strings.fieldYear),
      topOf(Strings.fieldRuntimeMinutes),
      reason: 'year and runtime share a row',
    );
    expect(
      topOf(Strings.fieldTmdbId),
      topOf(Strings.fieldImdbId),
      reason: 'the two ids share the last row',
    );
  });

  testWidgets('the ids are shown but cannot be typed into', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _movie);

    final TextField tmdb = tester.widget<TextField>(
      fieldNamed(Strings.fieldTmdbId),
    );
    final TextField imdb = tester.widget<TextField>(
      fieldNamed(Strings.fieldImdbId),
    );
    expect(tmdb.readOnly, isTrue);
    expect(imdb.readOnly, isTrue);
    expect(tmdb.controller?.text, 'movie/603');
    expect(imdb.controller?.text, 'tt0133093');
  });

  testWidgets('a missing id reads as none recorded rather than empty', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      (AdminApi admin) => EditMetadataDialog(
        admin: admin,
        target: EditTarget.movie,
        id: 'm1',
        title: 'Alpha',
        ratingSystems: _systems,
      ),
    );

    final TextField tmdb = tester.widget<TextField>(
      fieldNamed(Strings.fieldTmdbId),
    );
    expect(tmdb.controller?.text, Strings.noneRecorded);
  });

  testWidgets('an episode has no id row, since it carries no external ids', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _episode);

    expect(fieldNamed(Strings.fieldTmdbId), findsNothing);
    expect(fieldNamed(Strings.fieldImdbId), findsNothing);
  });

  testWidgets('a rating stored in upper case still selects in the dropdowns', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (AdminApi admin) => EditMetadataDialog(
        admin: admin,
        target: EditTarget.movie,
        id: 'm1',
        title: 'Alpha',
        ratingSystem: 'MPAA',
        ratingCode: 'R',
        ratingSystems: _systems,
      ),
    );

    expect(find.text('MPAA'), findsOneWidget);
    expect(find.text('R'), findsOneWidget);
    expect(
      find.text(Strings.optionNone),
      findsNothing,
      reason: 'the provider stores MPAA/R, the table keys are mpaa/r',
    );

    await _save(tester);

    final Map<String, dynamic> body =
        jsonDecode(seen.single.body) as Map<String, dynamic>;
    expect(body['content_rating'], <String, String>{
      'system': 'mpaa',
      'code': 'r',
    });
  });

  testWidgets('a rating the server does not know is offered and kept', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (AdminApi admin) => EditMetadataDialog(
        admin: admin,
        target: EditTarget.movie,
        id: 'm1',
        title: 'Alpha',
        ratingSystem: 'eirin',
        ratingCode: 'r15+',
        ratingSystems: _systems,
      ),
    );

    expect(find.text('EIRIN'), findsOneWidget);
    expect(find.text('R15+'), findsOneWidget);

    await _save(tester);

    final Map<String, dynamic> body =
        jsonDecode(seen.single.body) as Map<String, dynamic>;
    expect(body['content_rating'], <String, String>{
      'system': 'eirin',
      'code': 'r15+',
    }, reason: 'opening the form must not silently drop an unknown rating');
  });

  testWidgets('an empty rating reads as None, not as No limit', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      (AdminApi admin) => EditMetadataDialog(
        admin: admin,
        target: EditTarget.movie,
        id: 'm1',
        title: 'Alpha',
        ratingSystems: _systems,
      ),
    );

    expect(find.text(Strings.optionNone), findsNWidgets(2));
    expect(find.text(Strings.optionNoLimit), findsNothing);
  });

  testWidgets('choosing a rating system clears the code that no longer fits', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _movie);

    expect(find.text('R'), findsOneWidget);

    await tester.tap(_dropdownNamed(Strings.fieldRatingSystem));
    await tester.pumpAndSettle();
    await tester.tap(find.text('BBFC').last);
    await tester.pumpAndSettle();

    expect(
      find.text('R'),
      findsNothing,
      reason: 'an MPAA code cannot survive a switch to BBFC',
    );

    await tester.tap(_dropdownNamed(Strings.fieldRatingCode));
    await tester.pumpAndSettle();

    expect(find.text('12A'), findsWidgets);
    expect(find.text('PG-13'), findsNothing);
  });

  testWidgets('a series form drops runtime, which it has no column for', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _series);

    expect(fieldNamed(Strings.fieldYear), findsOneWidget);
    expect(fieldNamed(Strings.fieldRuntimeMinutes), findsNothing);
  });

  testWidgets('an episode form drops year and rating, and offers air date', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _episode);

    expect(fieldNamed(Strings.fieldAirDate), findsOneWidget);
    expect(fieldNamed(Strings.fieldRuntimeMinutes), findsOneWidget);
    expect(fieldNamed(Strings.fieldYear), findsNothing);
    expect(find.byType(LabelledDropdown<String>), findsNothing);
  });

  testWidgets('an empty title reports inline and does not submit', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester, _movie);

    await tester.enterText(fieldNamed(Strings.fieldTitle), '');
    await _save(tester);

    expect(find.text(Strings.requiredTitle), findsOneWidget);
    expect(seen, isEmpty);
  });

  testWidgets('typing a title clears the inline error', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _movie);

    await tester.enterText(fieldNamed(Strings.fieldTitle), '');
    await _save(tester);
    expect(find.text(Strings.requiredTitle), findsOneWidget);

    await tester.enterText(fieldNamed(Strings.fieldTitle), 'Alpha');
    await tester.pumpAndSettle();

    expect(find.text(Strings.requiredTitle), findsNothing);
  });

  testWidgets('a year that is not a number reports inline', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester, _movie);

    await tester.enterText(fieldNamed(Strings.fieldYear), 'nineteen');
    await _save(tester);

    expect(find.text(Strings.invalidYear), findsOneWidget);
    expect(seen, isEmpty);
  });

  testWidgets('a negative runtime reports inline', (WidgetTester tester) async {
    final List<http.Request> seen = await _pump(tester, _movie);

    await tester.enterText(fieldNamed(Strings.fieldRuntimeMinutes), '-5');
    await _save(tester);

    expect(find.text(Strings.invalidRuntime), findsOneWidget);
    expect(seen, isEmpty);
  });

  testWidgets('a malformed air date reports inline', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester, _episode);

    await tester.enterText(fieldNamed(Strings.fieldAirDate), 'last thursday');
    await _save(tester);

    expect(find.text(Strings.invalidAirDate), findsOneWidget);
    expect(seen, isEmpty);
  });

  testWidgets('saving a movie PUTs every editable field', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester, _movie);

    await tester.enterText(fieldNamed(Strings.fieldTitle), 'Alpha Edited');
    await _save(tester);

    expect(seen, hasLength(1));
    expect(seen.single.method, 'PUT');
    expect(seen.single.url.path, '/api/v1/movies/m1');
    final Map<String, dynamic> body =
        jsonDecode(seen.single.body) as Map<String, dynamic>;
    expect(body['title'], 'Alpha Edited');
    expect(body['year'], 2020);
    expect(body['runtime_minutes'], 100);
    expect(body['overview'], 'first');
    expect(body['content_rating'], <String, String>{
      'system': 'mpaa',
      'code': 'r',
    });
    expect(
      body.containsKey('tmdb_id'),
      isFalse,
      reason: 'external ids are shown but never sent',
    );
  });

  testWidgets('saving asks for confirmation and stops if it is declined', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester, _movie);

    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmSaveEditBody), findsOneWidget);

    await tester.tap(find.byTooltip(Strings.close).last);
    await tester.pumpAndSettle();

    expect(seen, isEmpty);
  });

  testWidgets('an emptied field is sent as null so the server clears it', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester, _movie);

    await tester.enterText(fieldNamed(Strings.fieldYear), '');
    await tester.enterText(fieldNamed(Strings.fieldOverview), '');
    await _save(tester);

    final Map<String, dynamic> body =
        jsonDecode(seen.single.body) as Map<String, dynamic>;
    expect(body.containsKey('year'), isTrue);
    expect(body['year'], isNull);
    expect(body['overview'], isNull);
  });

  testWidgets('a rating needs both halves or it is sent as null', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (AdminApi admin) => EditMetadataDialog(
        admin: admin,
        target: EditTarget.movie,
        id: 'm1',
        title: 'Alpha',
        ratingSystem: 'mpaa',
        ratingSystems: _systems,
      ),
    );

    await _save(tester);

    final Map<String, dynamic> body =
        jsonDecode(seen.single.body) as Map<String, dynamic>;
    expect(body['content_rating'], isNull);
  });

  testWidgets('saving an episode PUTs under its series and season', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(tester, _episode);

    await _save(tester);

    expect(seen.single.url.path, '/api/v1/series/s1/seasons/se1/episodes/e1');
    final Map<String, dynamic> body =
        jsonDecode(seen.single.body) as Map<String, dynamic>;
    expect(body['air_date'], '2010-10-01');
    expect(body.containsKey('year'), isFalse);
    expect(body.containsKey('content_rating'), isFalse);
  });

  test('a served timestamp is trimmed to the date the form edits', () {
    expect(shortAirDate('2010-10-01T00:00:00Z'), '2010-10-01');
    expect(shortAirDate('2010-10-01'), '2010-10-01');
    expect(shortAirDate(null), isNull);
  });

  testWidgets('a hand-edited record says so before the first field', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      (AdminApi admin) => EditMetadataDialog(
        admin: admin,
        target: EditTarget.movie,
        id: 'm1',
        title: 'Alpha',
        ratingSystems: _systems,
        manuallyEdited: true,
      ),
    );

    expect(find.byType(EditedNotice), findsOneWidget);
    expect(
      tester.getTopLeft(find.byType(EditedNotice)).dy,
      lessThan(tester.getTopLeft(find.text(Strings.fieldTitle)).dy),
      reason: 'the warning has to be read before the fields it qualifies',
    );
  });

  testWidgets('an untouched record carries no warning', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _movie);

    expect(find.byType(EditedNotice), findsNothing);
  });
}
