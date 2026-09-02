import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/title_heading.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/title_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _version(String id, {bool available = true}) =>
    <String, dynamic>{
      'id': id,
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'library_id': 'lib1',
      'path': '/media/$id.mkv',
      'container': 'mkv',
      'quality': 'fhd',
      'available': available,
      'added_at': '2026-08-17T09:00:00Z',
      'updated_at': '2026-08-17T09:00:00Z',
    };

ApiClient _api({
  List<Map<String, dynamic>> versions = const <Map<String, dynamic>>[],
  List<Map<String, dynamic>> seasons = const <Map<String, dynamic>>[],
  int episodesTotal = 0,
  int episodesPlayable = 0,
  List<String>? deleted,
}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    if (req.method == 'DELETE') {
      deleted?.add(req.url.path);
      return http.Response('', 204);
    }
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
    if (req.url.path.endsWith('/seasons')) {
      return http.Response(jsonEncode(seasons), 200);
    }
    if (req.url.path.contains('/versions')) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': versions,
          'total': versions.length,
          'offset': 0,
          'limit': 20,
        }),
        200,
      );
    }
    if (req.url.path.contains('/movies')) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'm1',
          'title': 'Arrival',
          'artwork': <String, dynamic>{},
          'genres': <String>[],
          'credits': <dynamic>[],
        }),
        200,
      );
    }
    if (req.url.path.contains('/series')) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 's1',
          'title': 'Stargate',
          'artwork': <String, dynamic>{},
          'genres': <String>[],
          'credits': <dynamic>[],
          'episodes_total': episodesTotal,
          'episodes_with_available_version': episodesPlayable,
        }),
        200,
      );
    }
    if (req.url.path.endsWith('/continue')) {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'now_playing': <dynamic>[],
          'in_progress': <dynamic>[],
          'next_episodes': <dynamic>[],
        }),
        200,
      );
    }
    return http.Response('[]', 200);
  }),
);

Future<void> _pump(WidgetTester tester, ApiClient api, {String? route}) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        initialRoute: route ?? '/',
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) =>
              settings.name != null && settings.name!.startsWith('/title')
              ? TitlePage(api: api)
              : (settings.name == null || settings.name == '/'
                    ? TitlePage(api: api)
                    : Text('went to ${settings.name}')),
        ),
        theme: buildTheme(AppThemeVariant.dark),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

bool _enabled(WidgetTester tester, String tooltip) {
  final IconButton button = tester.widget<IconButton>(
    find
        .ancestor(
          of: find.byTooltip(tooltip),
          matching: find.byType(IconButton),
        )
        .first,
  );
  return button.onPressed != null;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a movie with every file present can be relinked', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(versions: <Map<String, dynamic>>[_version('v1')]));

    expect(find.byTooltip(Strings.relink), findsOneWidget);
    expect(_enabled(tester, Strings.relink), isTrue);
  });

  testWidgets('a movie with a missing file cannot be relinked, and says why', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[
          _version('v1'),
          _version('v2', available: false),
        ],
      ),
    );

    expect(find.byTooltip(Strings.relink), findsNothing);
    final Finder blocked = find.byTooltip(Strings.relinkBlockedMovie(1));
    expect(blocked, findsOneWidget, reason: 'the count names the blocker');
    expect(
      _enabled(tester, Strings.relinkBlockedMovie(1)),
      isFalse,
      reason: 'the control stays visible but refuses the doomed relink',
    );
  });

  testWidgets('a movie still holding versions cannot be deleted', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(versions: <Map<String, dynamic>>[_version('v1'), _version('v2')]),
    );

    expect(find.byTooltip(Strings.deleteMovie), findsNothing);
    expect(_enabled(tester, Strings.blockedByVersions(2)), isFalse);
  });

  testWidgets('a movie with nothing left is deleted and leaves the page', (
    WidgetTester tester,
  ) async {
    final List<String> deleted = <String>[];
    await _pump(tester, _api(deleted: deleted));

    expect(_enabled(tester, Strings.deleteMovie), isTrue);
    await tester.tap(find.byTooltip(Strings.deleteMovie));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.delete).last);
    await tester.pumpAndSettle();

    expect(
      deleted.single,
      startsWith('/api/v1/admin/movies/'),
      reason: 'the id itself comes from Uri.base, which a VM test cannot set',
    );
    expect(find.text('went to /movies'), findsOneWidget);
  });

  testWidgets('a blocked delete stays on the page rather than disappearing', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(versions: <Map<String, dynamic>>[_version('v1')]));

    expect(
      find.byIcon(Icons.delete_outline),
      findsOneWidget,
      reason: 'the admin can see the action exists and read why it is off',
    );
  });

  testWidgets('delete reads in the danger colour once it is actionable', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api());

    final Icon icon = tester.widget<Icon>(find.byIcon(Icons.delete_outline));
    expect(icon.color, Tokens.dark.danger);
  });

  testWidgets('a blocked delete is not red, so it does not shout for nothing', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(versions: <Map<String, dynamic>>[_version('v1')]));

    final Icon icon = tester.widget<Icon>(find.byIcon(Icons.delete_outline));
    expect(icon.color, Tokens.dark.muted);
  });

  testWidgets('the actions run read, then writes, then the destructive one', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(versions: <Map<String, dynamic>>[_version('v1')]));

    final List<IconData> order = tester
        .widgetList<Icon>(
          find.descendant(
            of: find.byType(TitleHeading),
            matching: find.byType(Icon),
          ),
        )
        .map((Icon i) => i.icon!)
        .toList();

    expect(
      order,
      <IconData>[
        Icons.layers,
        Icons.refresh,
        Icons.edit,
        Icons.link,
        Icons.delete_outline,
      ],
      reason: 'read only, then provider refresh, hand edit, identity, removal',
    );
  });
}
