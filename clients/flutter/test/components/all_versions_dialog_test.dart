import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/all_versions_dialog.dart';
import 'package:shadowmask/components/outline_pill.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/version_picker.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/util/downloads.dart';
import 'package:shadowmask/view/version_order.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _json(String id, String quality) => <String, dynamic>{
  'id': id,
  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
  'library_id': 'l1',
  'quality': quality,
  'container': 'mkv',
  'size_bytes': 8 * 1048576,
  'duration_ms': 7200000,
  'available': true,
};

final List<Version> _shuffled = <Version>[
  Version.fromJson(_json('v-hd', 'hd')),
  Version.fromJson(_json('v-uhd', 'uhd')),
  Version.fromJson(_json('v-fhd', 'fhd')),
];

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    for (final Version v in _shuffled) {
      if (req.url.path.endsWith('/versions/${v.id}')) {
        return http.Response(
          jsonEncode(_json(v.id, v.quality.name)..remove('available')),
          200,
        );
      }
    }
    return http.Response('{}', 200);
  }),
);

Future<void> _pump(WidgetTester tester, Widget child, {Size? size}) async {
  if (size != null) {
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
  }
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: ToastHost(
          child: Scaffold(body: SingleChildScrollView(child: child)),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

List<String> _lines(WidgetTester tester) => tester
    .widgetList<Text>(find.byType(Text))
    .map((Text t) => t.data ?? t.textSpan?.toPlainText() ?? '')
    .toList();

bool _numbered(List<String> lines, String number, String quality) =>
    lines.contains('$number · 2h 00m · $quality · mkv · 8 MB');

void main() {
  final DownloadStarter platform = startDownload;

  setUp(() {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    startDownload = (String url, String filename) async => true;
  });
  tearDown(() => startDownload = platform);

  testWidgets('the dialog numbers versions the same way the picker does', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _api();
    await _pump(
      tester,
      VersionPicker(
        catalog: CatalogApi(api),
        playback: PlaybackApi(api),
        userId: 'u1',
        versions: _shuffled,
      ),
    );
    final List<String> picker = _lines(tester);
    expect(_numbered(picker, '1', 'UHD'), isTrue);
    expect(_numbered(picker, '2', 'FHD'), isTrue);
    expect(_numbered(picker, '3', 'HD'), isTrue);

    await _pump(
      tester,
      AllVersionsDialog(
        catalog: CatalogApi(api),
        playback: PlaybackApi(api),
        userId: 'u1',
        versions: orderedVersions(_shuffled),
      ),
    );
    final List<String> dialog = _lines(tester);
    expect(_numbered(dialog, '1', 'UHD'), isTrue);
    expect(_numbered(dialog, '2', 'FHD'), isTrue);
    expect(_numbered(dialog, '3', 'HD'), isTrue);
  });

  testWidgets('the dialog hosts the real picker, not a second list', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _api();
    await _pump(
      tester,
      AllVersionsDialog(
        catalog: CatalogApi(api),
        playback: PlaybackApi(api),
        userId: 'u1',
        versions: orderedVersions(_shuffled),
      ),
    );

    expect(find.byType(VersionPicker), findsOneWidget);
    expect(
      _lines(tester).where((String l) => l.startsWith('Versions (')).length,
      1,
      reason:
          'only the dialog title carries the count, the picker heading is hidden',
    );
  });

  testWidgets('a phone row shows only what fits and stays expandable', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _api();
    await _pump(
      tester,
      SizedBox(
        width: 280,
        child: VersionPicker(
          catalog: CatalogApi(api),
          playback: PlaybackApi(api),
          userId: 'u1',
          versions: _shuffled,
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    expect(_numbered(_lines(tester), '1', 'UHD'), isTrue);
    // Play and download move into the expanded body so the row cannot overflow.
    expect(find.text(Strings.play), findsNothing);
    expect(find.byIcon(Icons.download_outlined), findsNothing);
    expect(find.byIcon(Icons.expand_more), findsNWidgets(3));

    await tester.tap(find.byIcon(Icons.expand_more).first);
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text(Strings.play), findsOneWidget);
    expect(find.byIcon(Icons.download_outlined), findsOneWidget);
    expect(
      tester.getRect(find.byIcon(Icons.download_outlined)).left,
      greaterThan(tester.getRect(find.text(Strings.play)).right),
      reason: 'play leads the row and download sits at the far end',
    );
  });

  testWidgets('a wide row keeps play and download in place', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _api();
    await _pump(
      tester,
      SizedBox(
        width: 760,
        child: VersionPicker(
          catalog: CatalogApi(api),
          playback: PlaybackApi(api),
          userId: 'u1',
          versions: _shuffled,
        ),
      ),
    );

    expect(find.text(Strings.play), findsNWidgets(3));
    expect(find.byIcon(Icons.download_outlined), findsNWidgets(3));
  });

  group('admin actions', () {
    final Version single = Version.fromJson(_json('v-uhd', 'uhd'));

    ApiClient adminApi({required bool withFiles, List<String>? reads}) {
      bool held = withFiles;
      return ApiClient(
        baseUrl: 'http://test',
        httpClient: MockClient((http.Request req) async {
          if (req.url.path.endsWith('/subtitles/search')) {
            return http.Response(
              jsonEncode(<dynamic>[
                <String, dynamic>{
                  'file_id': '7',
                  'language': 'fr',
                  'release_name': 'Fresh.Release',
                  'format': 'srt',
                },
              ]),
              200,
            );
          }
          if (req.url.path.endsWith('/subtitles/download')) {
            held = true;
            return http.Response('', 204);
          }
          if (req.url.path.endsWith('/versions/v-uhd')) {
            reads?.add(req.url.path);
            return http.Response(
              jsonEncode(<String, dynamic>{
                ..._json('v-uhd', 'uhd')..remove('available'),
                'audio': <dynamic>[
                  <String, dynamic>{
                    'index': 1,
                    'codec': 'eac3',
                    'channels': 6,
                    'language': 'en',
                  },
                  <String, dynamic>{
                    'index': 2,
                    'codec': 'aac',
                    'channels': 2,
                    'language': 'es',
                  },
                ],
                'subtitle_files': <dynamic>[
                  if (held)
                    <String, dynamic>{
                      'id': 'sf1',
                      'language': 'en',
                      'format': 'srt',
                      'source': 'external',
                    },
                ],
              }),
              200,
            );
          }
          return http.Response('{}', 200);
        }),
      );
    }

    Future<List<String>> pumpPicker(
      WidgetTester tester, {
      required ApiClient api,
      required bool admin,
      double width = 760,
    }) async {
      final List<String> routes = <String>[];
      tester.view.physicalSize = const Size(1200, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        ThemeScope(
          variant: AppThemeVariant.dark,
          setVariant: (_) {},
          child: MaterialApp(
            theme: buildTheme(AppThemeVariant.dark),
            onGenerateRoute: (RouteSettings settings) {
              routes.add(settings.name ?? '');
              return MaterialPageRoute<void>(
                builder: (_) => settings.name == '/'
                    ? ToastHost(
                        child: Scaffold(
                          body: SingleChildScrollView(
                            child: SizedBox(
                              width: width,
                              child: VersionPicker(
                                catalog: CatalogApi(api),
                                playback: PlaybackApi(api),
                                userId: 'u1',
                                versions: <Version>[single],
                                admin: admin ? AdminApi(api) : null,
                              ),
                            ),
                          ),
                        ),
                      )
                    : Text('went to ${settings.name}'),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      return routes;
    }

    testWidgets('a viewer sees no admin actions', (WidgetTester tester) async {
      await pumpPicker(tester, api: adminApi(withFiles: true), admin: false);

      expect(find.byTooltip(Strings.openVersion), findsNothing);
      expect(find.byTooltip(Strings.subtitlesHeading), findsNothing);
    });

    testWidgets('an admin opens the version admin page from its row', (
      WidgetTester tester,
    ) async {
      final List<String> routes = await pumpPicker(
        tester,
        api: adminApi(withFiles: true),
        admin: true,
      );

      await tester.tap(find.byTooltip(Strings.openVersion));
      await tester.pumpAndSettle();

      expect(routes.last, '/version?id=v-uhd');
      expect(find.text('went to /version?id=v-uhd'), findsOneWidget);
    });

    testWidgets('the subtitles menu transcribes per audio track', (
      WidgetTester tester,
    ) async {
      await pumpPicker(tester, api: adminApi(withFiles: true), admin: true);

      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();

      expect(find.text(Strings.searchSubtitles), findsOneWidget);
      expect(find.text(Strings.translate), findsOneWidget);

      await tester.tap(find.text(Strings.transcribe));
      await tester.pumpAndSettle();

      expect(find.text('English · eac3 · 6ch'), findsOneWidget);
      expect(find.text('Spanish · aac · 2ch'), findsOneWidget);
    });

    testWidgets('translate has nothing to offer without a subtitle file', (
      WidgetTester tester,
    ) async {
      await pumpPicker(tester, api: adminApi(withFiles: false), admin: true);
      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();

      expect(
        tester
            .widget<SubmenuButton>(
              find.widgetWithText(SubmenuButton, Strings.translate),
            )
            .menuChildren,
        isEmpty,
      );
    });

    testWidgets('translate offers each subtitle file', (
      WidgetTester tester,
    ) async {
      await pumpPicker(tester, api: adminApi(withFiles: true), admin: true);
      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();

      expect(
        tester
            .widget<SubmenuButton>(
              find.widgetWithText(SubmenuButton, Strings.translate),
            )
            .menuChildren,
        hasLength(1),
      );
    });

    testWidgets('a subtitle downloaded from the menu reloads the row', (
      WidgetTester tester,
    ) async {
      final List<String> reads = <String>[];
      await pumpPicker(
        tester,
        api: adminApi(withFiles: false, reads: reads),
        admin: true,
      );
      Finder cc(String text) => find.descendant(
        of: find.byType(OutlinePill),
        matching: find.text(text),
      );
      expect(cc(Strings.ccNone), findsOneWidget);
      final int before = reads.length;

      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.searchSubtitles));
      await tester.pumpAndSettle();
      await tester.tap(
        find.widgetWithText(FilledButton, Strings.searchSubtitles),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(TextButton, Strings.download));
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(reads, hasLength(before + 1));
      expect(cc('1'), findsOneWidget);
    });

    testWidgets('a phone row keeps the admin actions in the expanded body', (
      WidgetTester tester,
    ) async {
      await pumpPicker(
        tester,
        api: adminApi(withFiles: true),
        admin: true,
        width: 280,
      );

      expect(find.byTooltip(Strings.openVersion), findsNothing);

      await tester.tap(find.byIcon(Icons.expand_more));
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(find.byTooltip(Strings.openVersion), findsOneWidget);
      expect(find.byTooltip(Strings.subtitlesHeading), findsOneWidget);
    });
  });
}
