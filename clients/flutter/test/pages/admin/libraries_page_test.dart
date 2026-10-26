import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/libraries_page.dart';
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

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('deleting a library reports success and reloads the list', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    bool deleted = false;
    int listCalls = 0;
    final ApiClient api = ApiClient(
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
        if (req.url.path == '/api/v1/libraries' && req.method == 'GET') {
          listCalls++;
          return http.Response(
            jsonEncode(deleted ? <dynamic>[] : <dynamic>[_library()]),
            200,
          );
        }
        if (req.url.path == '/api/v1/libraries/lib1' &&
            req.method == 'DELETE') {
          deleted = true;
          return http.Response('', 204);
        }
        return http.Response(jsonEncode(<dynamic>[]), 200);
      }),
    );

    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          builder: (BuildContext context, Widget? child) =>
              ToastHost(child: child ?? const SizedBox.shrink()),
          onGenerateRoute: (_) =>
              MaterialPageRoute<void>(builder: (_) => LibrariesPage(api: api)),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Films'), findsOneWidget);
    expect(listCalls, 1);

    await tester.tap(find.byIcon(Icons.delete_outline));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmDeleteLibrary('Films')), findsOneWidget);
    expect(
      find.ancestor(
        of: find.text(Strings.confirmDeleteLibrary('Films')),
        matching: find.byType(SelectionArea),
      ),
      findsOneWidget,
    );
    await tester.tap(find.widgetWithText(FilledButton, Strings.delete));
    await tester.pumpAndSettle();

    expect(find.text(Strings.toastLibraryDeleted), findsOneWidget);
    expect(find.text(Strings.errorDelete), findsNothing);
    expect(listCalls, 2);
    expect(find.text('Films'), findsNothing);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the crumb counts the libraries and the list sorts by name', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final ApiClient api = ApiClient(
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
        if (req.url.path == '/api/v1/libraries' && req.method == 'GET') {
          return http.Response(
            jsonEncode(<dynamic>[
              <String, dynamic>{..._library(), 'id': 'lib2', 'name': 'Shows'},
              _library(),
            ]),
            200,
          );
        }
        return http.Response(jsonEncode(<dynamic>[]), 200);
      }),
    );

    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          builder: (BuildContext context, Widget? child) =>
              ToastHost(child: child ?? const SizedBox.shrink()),
          onGenerateRoute: (_) =>
              MaterialPageRoute<void>(builder: (_) => LibrariesPage(api: api)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.countLabel(Strings.adminLibraries, 2)),
      findsOneWidget,
    );
    expect(
      tester.getTopLeft(find.text('Films')).dy,
      lessThan(tester.getTopLeft(find.text('Shows')).dy),
    );
  });

  testWidgets('Clean all confirms, sends the wipe choice, toasts what went '
      'and reloads the list', (WidgetTester tester) async {
    final List<dynamic> bodies = <dynamic>[];
    int listCalls = 0;
    await _pumpLibraries(tester, (http.Request req) {
      if (req.url.path == '/api/v1/libraries' && req.method == 'GET') {
        listCalls++;
      }
      if (req.url.path == _cleanAllPath) {
        bodies.add(jsonDecode(req.body));
        return http.Response(
          jsonEncode(<String, dynamic>{
            'versions': 2,
            'movies': 1,
            'series': 1,
            'seasons': 2,
            'episodes': 5,
            'wiped_users': 0,
          }),
          200,
        );
      }
      return null;
    });

    await tester.tap(find.text(Strings.cleanAll));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmCleanAll), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, Strings.clean));
    await tester.pumpAndSettle();

    expect(bodies, <dynamic>[
      <String, dynamic>{'wipe_user_data': false},
    ]);
    expect(
      find.text(
        'Cleaned: 2 versions, 1 movie, 1 series, 2 seasons, 5 episodes.',
      ),
      findsOneWidget,
    );
    expect(listCalls, 2);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a busy clean of every library lists the jobs, with no cancel', (
    WidgetTester tester,
  ) async {
    await _pumpLibraries(tester, (http.Request req) {
      if (req.url.path == _cleanAllPath) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'error': <String, dynamic>{
              'code': 'library_busy',
              'message': 'the library is busy',
            },
            'jobs': <dynamic>[
              <String, dynamic>{
                'id': 'scan-1',
                'kind': 'library_scan',
                'status': 'running',
                'priority': 'normal',
                'created_at': '2026-10-25T09:00:00Z',
                'updated_at': '2026-10-25T09:00:00Z',
              },
            ],
          }),
          409,
        );
      }
      return null;
    });

    await tester.tap(find.text(Strings.cleanAll));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.clean));
    await tester.pumpAndSettle();

    expect(find.text(Strings.cleanBusyBody), findsOneWidget);
    expect(find.text('Library scan'), findsOneWidget);
    expect(find.text(Strings.cancelThisLibrarysJobs), findsNothing);
    await tester.tap(find.widgetWithText(OutlinedButton, Strings.close));
    await tester.pumpAndSettle();
    expect(find.text(Strings.cleanBusyBody), findsNothing);
  });
}

const String _cleanAllPath = '/api/v1/admin/clean';

Future<void> _pumpLibraries(
  WidgetTester tester,
  http.Response? Function(http.Request req) route,
) async {
  tester.view.physicalSize = const Size(1600, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
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
      final http.Response? routed = route(req);
      if (routed != null) {
        return routed;
      }
      if (req.url.path == '/api/v1/libraries' && req.method == 'GET') {
        return http.Response(jsonEncode(<dynamic>[_library()]), 200);
      }
      return http.Response(jsonEncode(<dynamic>[]), 200);
    }),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (_) =>
            MaterialPageRoute<void>(builder: (_) => LibrariesPage(api: api)),
      ),
    ),
  );
  await tester.pumpAndSettle();
}
