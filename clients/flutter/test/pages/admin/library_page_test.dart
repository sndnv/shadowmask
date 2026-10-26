import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/pages/admin/library_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _library() => <String, dynamic>{
  'id': 'lib1',
  'name': 'Films',
  'kind': 'movie',
  'origin': 'local',
  'roots': <String>['/media/films'],
  'watcher': 'manual',
  'metadata_sources': <String>[],
  'sort_articles': <String>['the'],
  'created_at': '2026-08-17T09:00:00Z',
  'updated_at': '2026-08-17T09:00:00Z',
};

Future<void> _pump(
  WidgetTester tester,
  ApiClient api, {
  double width = 1600,
}) async {
  tester.view.physicalSize = Size(width, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (_) => MaterialPageRoute<void>(
          builder: (_) => LibraryPage(api: api, libraryId: 'lib1'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

http.Response _detailRoute(http.Request req) {
  final RegExp detail = RegExp(r'^/api/v1/libraries/[^/]*$');
  if (detail.hasMatch(req.url.path)) {
    return http.Response(jsonEncode(_library()), 200);
  }
  if (req.url.path == '/api/v1/libraries/lib1/scan') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'library_id': 'lib1',
        'status': 'idle',
        'progress': 0,
        'last_scanned_at': '2026-08-17T09:24:11Z',
      }),
      200,
    );
  }
  return http.Response(jsonEncode(<dynamic>[]), 200);
}

ApiClient _api(http.Response Function(http.Request req) route) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    if (req.url.path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'admin',
        }),
        200,
      );
    }
    return route(req);
  }),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the detail page carries its actions on the library card', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(_detailRoute));

    final Finder card = find.ancestor(
      of: find.text('Films'),
      matching: find.byType(SectionBlock),
    );
    for (final String label in <String>[
      Strings.delete,
      Strings.purge,
      Strings.clean,
      Strings.cancelLibraryJobs,
      Strings.refreshMetadata,
      Strings.edit,
    ]) {
      expect(
        find.descendant(
          of: card,
          matching: find.widgetWithText(OutlinedButton, label),
        ),
        findsOneWidget,
        reason: 'missing the $label action',
      );
    }
    expect(find.byType(PageActions), findsNothing);
  });

  testWidgets('an episode version is named by its series and code', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == '/api/v1/libraries/lib1/versions') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'items': <dynamic>[
                <String, dynamic>{
                  'id': 'v9',
                  'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
                  'library_id': 'lib1',
                  'quality': 'hd',
                  'container': 'mkv',
                  'size_bytes': 1258291200,
                  'path': '/media/films/skyline.mkv',
                },
              ],
              'total': 1,
              'offset': 0,
              'limit': 50,
            }),
            200,
          );
        }
        if (req.url.path == '/api/v1/titles/batch') {
          return http.Response(
            jsonEncode(<dynamic>[
              <String, dynamic>{
                'type': 'episode',
                'id': 'e1',
                'season_id': 'se2',
                'series_id': 's1',
                'number': 2,
                'title': 'Earth',
                'series_title': 'Skyline',
                'season_number': 2,
              },
            ]),
            200,
          );
        }
        return _detailRoute(req);
      }),
    );

    expect(find.text('Skyline · S02E02'), findsOneWidget);
  });

  testWidgets('the three card actions still fit a narrow window', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(_detailRoute), width: 900);

    expect(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
      findsOneWidget,
    );
  });

  testWidgets('Edit library opens the shared form prefilled', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(_detailRoute));

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.edit));
    await tester.pumpAndSettle();

    expect(find.text(Strings.editLibrary), findsWidgets);
    expect(find.widgetWithText(TextField, 'Films'), findsOneWidget);
  });

  testWidgets('Refresh metadata confirms, then posts and does not scan', (
    WidgetTester tester,
  ) async {
    final List<String> posted = <String>[];
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.method == 'POST') {
          posted.add(req.url.path);
          return http.Response('', 202);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmRefreshLibraryBody), findsOneWidget);

    await tester.tap(
      find.widgetWithText(FilledButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(posted, <String>['/api/v1/libraries/lib1/refresh-metadata']);
    expect(find.text(Strings.toastMetadataQueued), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('deleting from the detail page returns to the library list', (
    WidgetTester tester,
  ) async {
    bool deleted = false;
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.method == 'DELETE') {
          deleted = true;
          return http.Response('', 204);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.delete));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.delete).last);
    await tester.pumpAndSettle();

    expect(deleted, isTrue);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the scan block refreshes its state without queueing a scan', (
    WidgetTester tester,
  ) async {
    final List<String> requested = <String>[];
    await _pump(
      tester,
      _api((http.Request req) {
        requested.add('${req.method} ${req.url.path}');
        return _detailRoute(req);
      }),
    );

    const String scan = 'GET /api/v1/libraries/lib1/scan';
    expect(requested.where((String r) => r == scan), hasLength(1));

    final Finder block = find.ancestor(
      of: find.text(Strings.scanHeading),
      matching: find.byType(SectionBlock),
    );
    await tester.tap(
      find.descendant(
        of: block,
        matching: find.widgetWithText(OutlinedButton, Strings.refresh),
      ),
    );
    await tester.pumpAndSettle();

    expect(requested.where((String r) => r == scan), hasLength(2));
    expect(
      requested.where((String r) => r == 'POST /api/v1/libraries/lib1/scan'),
      isEmpty,
      reason: 'refresh must not queue a scan',
    );
  });

  testWidgets('Re-read files confirms, then queues a scan that re-reads', (
    WidgetTester tester,
  ) async {
    final List<String> bodies = <String>[];
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.method == 'POST' &&
            req.url.path == '/api/v1/libraries/lib1/scan') {
          bodies.add(req.body);
          return http.Response('', 202);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.rereadFiles));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmRereadFilesBody), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, Strings.rereadFiles));
    await tester.pumpAndSettle();

    expect(bodies, hasLength(1));
    expect(jsonDecode(bodies.single), <String, dynamic>{'reread': true});
    expect(find.text(Strings.toastRereadQueued), findsOneWidget);

    await tester.tap(find.text(Strings.scan).last);
    await tester.pumpAndSettle();

    expect(bodies, hasLength(2));
    expect(bodies.last, isEmpty, reason: 'a plain scan sends no options');

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a cancelled re-read queues nothing', (
    WidgetTester tester,
  ) async {
    final List<String> posted = <String>[];
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.method == 'POST') {
          posted.add(req.url.path);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.rereadFiles));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.cancel).last);
    await tester.pumpAndSettle();

    expect(posted, isEmpty);
  });

  testWidgets('the last scan reads as a date rather than as sent', (
    WidgetTester tester,
  ) async {
    // The server sends ISO 8601 and the client owns the rendering, the same
    // way it owns the words.
    await _pump(tester, _api(_detailRoute));

    expect(find.textContaining('2026-08-17T09:24'), findsNothing);
    expect(find.text(dateTimeText('2026-08-17T09:24:11Z')!), findsOneWidget);
  });

  testWidgets('a duplicate offers Dismiss and nothing else', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == '/api/v1/libraries/lib1/duplicates') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'items': <Map<String, dynamic>>[
                <String, dynamic>{
                  'id': 'dup1',
                  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
                  'paths': <String>['/media/a.mkv', '/media/b.mkv'],
                },
              ],
              'total': 1,
              'offset': 0,
              'limit': 50,
            }),
            200,
          );
        }
        return _detailRoute(req);
      }),
    );

    final Finder block = find.ancestor(
      of: find.text(Strings.duplicatesHeading),
      matching: find.byType(SectionBlock),
    );
    expect(
      find.descendant(
        of: block,
        matching: find.widgetWithText(TextButton, Strings.dismiss),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: block,
        matching: find.widgetWithText(TextButton, Strings.resolve),
      ),
      findsNothing,
      reason: 'Resolve did the same thing as Dismiss and was removed',
    );
    expect(
      find.text(Strings.duplicateSplitAcross(2)),
      findsNothing,
      reason: 'two rips of one film are duplicates, and deleting one is right',
    );
  });

  testWidgets(
    'a title split across files says so instead of reading as a duplicate',
    (WidgetTester tester) async {
      await _pump(
        tester,
        _api((http.Request req) {
          if (req.url.path == '/api/v1/libraries/lib1/duplicates') {
            return http.Response(
              jsonEncode(<String, dynamic>{
                'items': <Map<String, dynamic>>[
                  <String, dynamic>{
                    'id': 'dup1',
                    'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
                    'kind': 'multi_part',
                    'paths': <String>[
                      '/media/film.CD1.avi',
                      '/media/film.CD2.avi',
                    ],
                  },
                ],
                'total': 1,
                'offset': 0,
                'limit': 50,
              }),
              200,
            );
          }
          return _detailRoute(req);
        }),
      );

      expect(
        find.text(Strings.duplicateSplitAcross(2)),
        findsOneWidget,
        reason: 'deleting one half of a film is the opposite of the right fix',
      );
      expect(find.byIcon(Icons.warning_amber_rounded), findsOneWidget);
    },
  );

  testWidgets('the versions heading counts every version, not the page', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == '/api/v1/libraries/lib1/versions') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'items': <Map<String, dynamic>>[
                <String, dynamic>{
                  'id': 'v1',
                  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
                  'library_id': 'lib1',
                  'quality': 'fhd',
                  'container': 'mkv',
                  'path': '/media/a.mkv',
                  'size_bytes': 1,
                  'available': true,
                },
              ],
              'total': 12,
              'offset': 0,
              'limit': 1,
            }),
            200,
          );
        }
        return _detailRoute(req);
      }),
    );

    expect(find.text(Strings.versionsWithCount(12)), findsOneWidget);
    expect(find.text(Strings.versionsWithCount(1)), findsNothing);
  });

  testWidgets('a purge keeps user data unless asked, toasts what went and '
      'reloads the blocks', (WidgetTester tester) async {
    final List<String> requested = <String>[];
    final List<dynamic> bodies = <dynamic>[];
    await _pump(
      tester,
      _api((http.Request req) {
        requested.add('${req.method} ${req.url.path}');
        if (req.url.path == _purgePath) {
          bodies.add(jsonDecode(req.body));
          return http.Response(jsonEncode(_purged(wipedUsers: 0)), 200);
        }
        return _detailRoute(req);
      }),
    );
    final int scansBefore = requested.where(_isScanRead).length;

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.purge));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmPurgeLibrary('Films')), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, Strings.purge));
    await tester.pumpAndSettle();

    expect(bodies, <dynamic>[
      <String, dynamic>{'wipe_user_data': false},
    ]);
    expect(
      find.text('Purged: 1 series, 3 seasons, 32 episodes, 40 subtitles.'),
      findsOneWidget,
    );
    expect(requested.where(_isScanRead).length, scansBefore + 1);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('ticking the box asks for user data to be wiped', (
    WidgetTester tester,
  ) async {
    final List<dynamic> bodies = <dynamic>[];
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == _purgePath) {
          bodies.add(jsonDecode(req.body));
          return http.Response(jsonEncode(_purged(wipedUsers: 4)), 200);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.purge));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(CheckboxListTile));
    await tester.pump();
    await tester.tap(find.widgetWithText(FilledButton, Strings.purge));
    await tester.pumpAndSettle();

    expect(bodies, <dynamic>[
      <String, dynamic>{'wipe_user_data': true},
    ]);
    expect(find.textContaining('User data wiped for 4 users.'), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a purge cancelled in its dialog sends nothing', (
    WidgetTester tester,
  ) async {
    final List<String> posted = <String>[];
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.method == 'POST') {
          posted.add(req.url.path);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.purge));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, Strings.cancel));
    await tester.pumpAndSettle();

    expect(posted, isEmpty);
  });

  testWidgets('a busy library names the jobs and cancels them from there', (
    WidgetTester tester,
  ) async {
    final List<String> posted = <String>[];
    await _pump(tester, _api(_busyRoute(posted)));

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.purge));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.purge));
    await tester.pumpAndSettle();

    expect(find.text(Strings.libraryBusy), findsOneWidget);
    expect(find.text('Library scan'), findsOneWidget);
    expect(
      find.byType(ListTile),
      findsOneWidget,
      reason: 'a job this client cannot read is left out',
    );
    await tester.tap(
      find.widgetWithText(FilledButton, Strings.cancelThisLibrarysJobs),
    );
    await tester.pumpAndSettle();

    expect(posted, <String>[
      _purgePath,
      '$_cancelPath {"blocking":"purge"}',
    ], reason: 'only what blocks the purge is cancelled');
    expect(
      find.text('Cancelled 2, stopped 0, 1 still running.'),
      findsOneWidget,
    );

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a job in the busy list opens its page', (
    WidgetTester tester,
  ) async {
    final List<String> posted = <String>[];
    await _pump(tester, _api(_busyRoute(posted)));

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.purge));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.purge));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Library scan'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.libraryBusy), findsNothing);
    expect(posted, <String>[_purgePath], reason: 'nothing is cancelled');
  });

  testWidgets('Cancel jobs confirms, then toasts what it stopped', (
    WidgetTester tester,
  ) async {
    final List<String> posted = <String>[];
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == _cancelPath) {
          posted.add('${req.url.path} ${req.body}');
          return http.Response(jsonEncode(_cancelled()), 200);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.cancelLibraryJobs),
    );
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmCancelLibraryJobs), findsOneWidget);
    await tester.tap(
      find.widgetWithText(FilledButton, Strings.cancelLibraryJobs),
    );
    await tester.pumpAndSettle();

    expect(posted, <String>[
      '$_cancelPath ',
    ], reason: 'no body: every job of the library');
    expect(
      find.text('Cancelled 2, stopped 0, 1 still running.'),
      findsOneWidget,
    );

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a purge that fails for another reason says it failed', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == _purgePath) {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'error': <String, dynamic>{
                'code': 'internal',
                'message': 'internal server error',
              },
            }),
            500,
          );
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.purge));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.purge));
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.errorPurge), findsOneWidget);
    expect(find.text(Strings.libraryBusy), findsNothing);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a purge refused by a clean says so, with no busy list', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == _purgePath) {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'error': <String, dynamic>{
                'code': 'library_cleaning',
                'message': 'the library is being cleaned',
              },
            }),
            409,
          );
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.purge));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.purge));
    await tester.pumpAndSettle();

    expect(
      find.text('${Strings.errorPurge} ${Strings.reasonLibraryCleaning}'),
      findsOneWidget,
    );
    expect(find.text(Strings.libraryBusy), findsNothing);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a library being purged reads Purging', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == '/api/v1/libraries/lib1/scan') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'library_id': 'lib1',
              'status': 'purging',
              'progress': 0,
            }),
            200,
          );
        }
        return _detailRoute(req);
      }),
    );

    expect(find.text('Purging'), findsOneWidget);
  });

  testWidgets('a clean keeps user data unless asked, toasts what went and '
      'reloads the blocks', (WidgetTester tester) async {
    final List<String> requested = <String>[];
    final List<dynamic> bodies = <dynamic>[];
    await _pump(
      tester,
      _api((http.Request req) {
        requested.add('${req.method} ${req.url.path}');
        if (req.url.path == _cleanPath) {
          bodies.add(jsonDecode(req.body));
          return http.Response(jsonEncode(_cleaned(versions: 12)), 200);
        }
        return _detailRoute(req);
      }),
    );
    final int scansBefore = requested.where(_isScanRead).length;

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.clean));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmCleanLibrary('Films')), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, Strings.clean));
    await tester.pumpAndSettle();

    expect(bodies, <dynamic>[
      <String, dynamic>{'wipe_user_data': false},
    ]);
    expect(find.text('Cleaned: 12 versions, 3 movies.'), findsOneWidget);
    expect(requested.where(_isScanRead).length, scansBefore + 1);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a clean with the box ticked wipes user data', (
    WidgetTester tester,
  ) async {
    final List<dynamic> bodies = <dynamic>[];
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == _cleanPath) {
          bodies.add(jsonDecode(req.body));
          return http.Response(
            jsonEncode(_cleaned(versions: 2, movies: 0, wipedUsers: 1)),
            200,
          );
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.clean));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(CheckboxListTile));
    await tester.pump();
    await tester.tap(find.widgetWithText(FilledButton, Strings.clean));
    await tester.pumpAndSettle();

    expect(bodies, <dynamic>[
      <String, dynamic>{'wipe_user_data': true},
    ]);
    expect(
      find.text('Cleaned: 2 versions. User data wiped for 1 user.'),
      findsOneWidget,
    );

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a busy clean names the jobs and cancels them from there', (
    WidgetTester tester,
  ) async {
    final List<String> posted = <String>[];
    await _pump(tester, _api(_busyRoute(posted, path: _cleanPath)));

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.clean));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.clean));
    await tester.pumpAndSettle();

    expect(find.text(Strings.cleanBusyBody), findsOneWidget);
    expect(find.text('Library scan'), findsOneWidget);
    await tester.tap(
      find.widgetWithText(FilledButton, Strings.cancelThisLibrarysJobs),
    );
    await tester.pumpAndSettle();

    expect(posted, <String>[
      _cleanPath,
      '$_cancelPath {"blocking":"clean"}',
    ], reason: 'only what blocks the clean is cancelled');

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a clean that fails for another reason says it failed', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == _cleanPath) {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'error': <String, dynamic>{
                'code': 'internal',
                'message': 'internal server error',
              },
            }),
            500,
          );
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.clean));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.clean));
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.errorClean), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a library being cleaned reads Cleaning', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == '/api/v1/libraries/lib1/scan') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'library_id': 'lib1',
              'status': 'cleaning',
              'progress': 0,
            }),
            200,
          );
        }
        return _detailRoute(req);
      }),
    );

    expect(find.text('Cleaning'), findsOneWidget);
  });
}

const String _purgePath = '/api/v1/libraries/lib1/purge';
const String _cleanPath = '/api/v1/libraries/lib1/clean';

Map<String, dynamic> _cleaned({
  required int versions,
  int movies = 3,
  int wipedUsers = 0,
}) => <String, dynamic>{
  'versions': versions,
  'movies': movies,
  'series': 0,
  'seasons': 0,
  'episodes': 0,
  'wiped_users': wipedUsers,
};
const String _cancelPath = '/api/v1/libraries/lib1/jobs/cancel';

bool _isScanRead(String request) =>
    request == 'GET /api/v1/libraries/lib1/scan';

Map<String, dynamic> _purged({required int wipedUsers}) => <String, dynamic>{
  'movies': 0,
  'series': 1,
  'seasons': 3,
  'episodes': 32,
  'subtitles': 40,
  'wiped_users': wipedUsers,
};

Map<String, dynamic> _cancelled() => <String, dynamic>{
  'cancelled': 2,
  'signalled': 0,
  'running': 1,
};

Map<String, dynamic> _job(String id, String kind) => <String, dynamic>{
  'id': id,
  'kind': kind,
  'status': 'queued',
  'priority': 'normal',
  'created_at': '2026-10-25T09:00:00Z',
  'updated_at': '2026-10-25T09:00:00Z',
};

http.Response Function(http.Request req) _busyRoute(
  List<String> posted, {
  String path = _purgePath,
}) => (http.Request req) {
  if (req.url.path == path) {
    posted.add(req.url.path);
    return http.Response(
      jsonEncode(<String, dynamic>{
        'error': <String, dynamic>{
          'code': 'library_busy',
          'message': 'the library is busy',
        },
        'jobs': <dynamic>[
          _job('scan-1', 'library_scan'),
          _job('later-1', 'from_a_newer_server'),
        ],
      }),
      409,
    );
  }
  if (req.url.path == _cancelPath) {
    posted.add('${req.url.path} ${req.body}');
    return http.Response(jsonEncode(_cancelled()), 200);
  }
  return _detailRoute(req);
};
