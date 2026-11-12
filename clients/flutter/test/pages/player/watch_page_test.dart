import 'dart:convert';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/player/watch_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

ApiClient _api({required bool present, bool queueFails = false}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    final String path = req.url.path;
    if (queueFails && path == '/api/v1/users/u1/queue') {
      return http.Response('{"message":"prefs.db is busy"}', 500);
    }
    if (path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'user',
        }),
        200,
      );
    }
    if (path == '/api/v1/users/u1/queue') {
      return http.Response(
        jsonEncode(<dynamic>[
          if (present)
            <String, dynamic>{
              'id': 'q2',
              'card': <String, dynamic>{
                'type': 'movie',
                'id': 'm5',
                'title': 'Fifth',
              },
            },
        ]),
        200,
      );
    }
    if (path == '/api/v1/movies/m5/versions') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': <dynamic>[
            <String, dynamic>{
              'id': 'v5',
              'title': <String, dynamic>{'type': 'movie', 'id': 'm5'},
              'library_id': 'lib',
              'quality': 'hd',
              'container': 'mp4',
              'available': true,
            },
          ],
          'total': 1,
        }),
        200,
      );
    }
    return http.Response('', 500);
  }),
);

Future<List<String>> _open(WidgetTester tester, ApiClient api) async {
  final List<String> opened = <String>[];
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: WatchPage(api: api, versionId: '', queueEntryId: 'q2'),
        onGenerateRoute: (RouteSettings settings) {
          opened.add(settings.name ?? '');
          return MaterialPageRoute<void>(
            settings: settings,
            builder: (BuildContext _) => const SizedBox(),
          );
        },
      ),
    ),
  );
  await tester.pumpAndSettle();
  return opened;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a queue entry without a version opens its title in queue mode', (
    WidgetTester tester,
  ) async {
    final List<String> opened = await _open(tester, _api(present: true));

    expect(opened, <String>[watchRoute('v5', queue: 'q2')]);
  });

  testWidgets('no version and no queue entry has nothing to play', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: WatchPage(api: _api(present: false), versionId: ''),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.noVersionSpecified), findsOneWidget);
  });

  testWidgets('a queue that cannot be read opens the playlists', (
    WidgetTester tester,
  ) async {
    final List<String> opened = await _open(
      tester,
      _api(present: true, queueFails: true),
    );

    expect(opened, <String>[playlistsRoute()]);
  });

  testWidgets('a queue entry that has left the queue opens the playlists', (
    WidgetTester tester,
  ) async {
    final List<String> opened = await _open(tester, _api(present: false));

    expect(opened, <String>[playlistsRoute()]);
  });
}
