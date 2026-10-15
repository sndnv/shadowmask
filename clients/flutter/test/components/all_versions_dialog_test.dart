import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/all_versions_dialog.dart';
import 'package:shadowmask/components/long_menu_item.dart';
import 'package:shadowmask/components/outline_pill.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/version_picker.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/util/downloads.dart';
import 'package:shadowmask/view/version_jobs_watch.dart';
import 'package:shadowmask/view/version_order.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../support/play_on.dart';

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

ApiClient _api([PlayOnServer? server]) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    final http.Response? playOn = server?.answer(req);
    if (playOn != null) {
      return playOn;
    }
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

Finder _more() => find.byTooltip(Strings.columnActions);

Future<void> _expand(WidgetTester tester, [int at = 0]) async {
  await tester.tap(find.byIcon(Icons.expand_more).at(at));
  await tester.pumpAndSettle();
}

Future<void> _openMore(WidgetTester tester, [int at = 0]) async {
  await tester.tap(_more().at(at));
  await tester.pumpAndSettle();
}

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
    expect(_more(), findsNothing);
    expect(find.byIcon(Icons.expand_more), findsNWidgets(3));

    await tester.tap(find.byIcon(Icons.expand_more).first);
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text(Strings.play), findsOneWidget);
    expect(_more(), findsOneWidget);
    expect(
      tester.getRect(_more()).left,
      greaterThan(tester.getRect(find.text(Strings.play)).right),
      reason: 'play leads the row and the actions sit at the far end',
    );
  });

  testWidgets('a wide row reads like a phone row: actions once it opens', (
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

    expect(find.text(Strings.play), findsNothing);
    expect(_more(), findsNothing);
    expect(find.byType(OutlinePill), findsNWidgets(3));
    expect(find.byIcon(Icons.expand_more), findsNWidgets(3));

    await _expand(tester, 1);

    expect(find.text(Strings.play), findsOneWidget);
    expect(_more(), findsOneWidget);
    expect(find.byIcon(Icons.download_outlined), findsNothing);
  });

  testWidgets('a menu holds download, play on and the version page', (
    WidgetTester tester,
  ) async {
    final PlayOnServer server = PlayOnServer();
    final ApiClient api = _api(server);
    await _pump(
      tester,
      SizedBox(
        width: 760,
        child: VersionPicker(
          catalog: CatalogApi(api),
          playback: PlaybackApi(api),
          userId: 'u1',
          versions: <Version>[_shuffled.first],
          adminPages: true,
          playOn: server.api(api),
        ),
      ),
    );

    await _expand(tester);
    await _openMore(tester);

    expect(
      tester.widgetList<MenuItemButton>(find.byType(MenuItemButton)).length,
      2,
    );
    expect(find.text(Strings.downloadVersion), findsOneWidget);
    expect(find.widgetWithText(SubmenuButton, Strings.playOn), findsOneWidget);
    expect(find.text(Strings.openVersion), findsOneWidget);
    expect(
      tester.getRect(find.text(Strings.downloadVersion)).top,
      lessThan(tester.getRect(find.text(Strings.playOn)).top),
    );
    expect(
      tester.getRect(find.text(Strings.playOn)).top,
      lessThan(tester.getRect(find.text(Strings.openVersion)).top),
    );
  });

  testWidgets('a phone row in progress fits every action an admin has', (
    WidgetTester tester,
  ) async {
    final PlayOnServer server = PlayOnServer();
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        final http.Response? playOn = server.answer(req);
        if (playOn != null) {
          return playOn;
        }
        if (req.url.path.contains('/progress/')) {
          return http.Response(
            jsonEncode(<String, dynamic>{'position_ms': 2880000}),
            200,
          );
        }
        if (req.url.path.endsWith('/jobs')) {
          return http.Response('[]', 200);
        }
        if (req.url.path.endsWith('/versions/v-uhd')) {
          return http.Response(
            jsonEncode(_json('v-uhd', 'uhd')..remove('available')),
            200,
          );
        }
        return http.Response('{}', 200);
      }),
    );
    await _pump(
      tester,
      Builder(
        builder: (BuildContext context) => TextButton(
          onPressed: () => AllVersionsDialog.show(
            context,
            catalog: CatalogApi(api),
            playback: PlaybackApi(api),
            userId: 'u1',
            versions: <Version>[_shuffled[1]],
            admin: AdminApi(api),
            adminPages: true,
            playOn: server.api(api),
          ),
          child: const Text('open'),
        ),
      ),
      size: const Size(412, 915),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(find.byType(OutlinePill), findsOneWidget);

    await _expand(tester);

    expect(tester.takeException(), isNull);
    final RenderParagraph resume = tester.renderObject<RenderParagraph>(
      find.descendant(
        of: find.text(Strings.resumeAction),
        matching: find.byType(RichText),
      ),
    );
    expect(
      resume.size.width,
      greaterThanOrEqualTo(resume.getMaxIntrinsicWidth(double.infinity)),
      reason: 'the label shows in full',
    );
    expect(find.byTooltip(Strings.dismissResume), findsNothing);
    expect(find.byTooltip(Strings.subtitlesHeading), findsOneWidget);

    await _openMore(tester);
    await tester.tap(find.text(Strings.dismissResume));
    await tester.pumpAndSettle();

    expect(find.text(Strings.resumeAction), findsNothing);
    expect(find.text(Strings.play), findsOneWidget);
  });

  testWidgets('a wide row in progress dismisses from the menu too', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        if (req.url.path.contains('/progress/')) {
          return http.Response(
            jsonEncode(<String, dynamic>{'position_ms': 2880000}),
            200,
          );
        }
        return http.Response('{}', 200);
      }),
    );
    await _pump(
      tester,
      SizedBox(
        width: 760,
        child: VersionPicker(
          catalog: CatalogApi(api),
          playback: PlaybackApi(api),
          userId: 'u1',
          versions: <Version>[_shuffled.first],
        ),
      ),
    );

    await _expand(tester);

    expect(find.text(Strings.resumeAction), findsOneWidget);
    expect(find.byTooltip(Strings.dismissResume), findsNothing);
    await _openMore(tester);
    expect(find.text(Strings.dismissResume), findsOneWidget);
  });

  group('admin actions', () {
    final Version single = Version.fromJson(_json('v-uhd', 'uhd'));

    ApiClient adminApi({
      required bool withFiles,
      bool twoFiles = false,
      List<String>? reads,
      List<Uri>? combines,
      List<List<Map<String, dynamic>>>? jobs,
      List<String>? jobReads,
      bool heldAfterWork = false,
    }) {
      bool held = withFiles;
      int polls = 0;
      return ApiClient(
        baseUrl: 'http://test',
        httpClient: MockClient((http.Request req) async {
          if (req.url.path.endsWith('/versions/v-uhd/jobs')) {
            jobReads?.add(req.url.path);
            if (heldAfterWork && polls > 0) {
              held = true;
            }
            final List<List<Map<String, dynamic>>> script =
                jobs ?? const <List<Map<String, dynamic>>>[];
            final List<Map<String, dynamic>> rows = script.isEmpty
                ? const <Map<String, dynamic>>[]
                : script[polls < script.length ? polls : script.length - 1];
            polls++;
            return http.Response(jsonEncode(rows), 200);
          }
          if (req.url.path.endsWith('/combine')) {
            combines?.add(req.url);
            return http.Response(
              jsonEncode(<String, String>{'job_id': 'j1'}),
              202,
            );
          }
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
            return http.Response(
              jsonEncode(<String, dynamic>{
                'id': 'opensubtitles:v-uhd:7',
                'language': 'fr',
                'format': 'srt',
                'source': 'open_subtitles',
                'pinned': true,
              }),
              201,
            );
          }
          if (req.url.path.endsWith('/versions/v-uhd')) {
            reads?.add(req.url.path);
            if (heldAfterWork && held) {
              await Future<void>.delayed(const Duration(seconds: 2));
            }
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
                  if (twoFiles)
                    <String, dynamic>{
                      'id': 'sf2',
                      'language': 'fr',
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
      bool? adminPages,
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
                                adminPages: adminPages ?? admin,
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
      await _expand(tester);

      expect(find.byTooltip(Strings.subtitlesHeading), findsNothing);
      await _openMore(tester);
      expect(find.text(Strings.downloadVersion), findsOneWidget);
      expect(find.text(Strings.openVersion), findsNothing);
    });

    testWidgets('an admin opens the version admin page from its row', (
      WidgetTester tester,
    ) async {
      final List<String> routes = await pumpPicker(
        tester,
        api: adminApi(withFiles: true),
        admin: true,
      );

      await _expand(tester);
      await _openMore(tester);
      await tester.tap(find.text(Strings.openVersion));
      await tester.pumpAndSettle();

      expect(routes.last, '/version?id=v-uhd');
      expect(find.text('went to /version?id=v-uhd'), findsOneWidget);
    });

    testWidgets('the subtitles menu transcribes per audio track', (
      WidgetTester tester,
    ) async {
      await pumpPicker(tester, api: adminApi(withFiles: true), admin: true);

      await _expand(tester);
      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();

      expect(find.text(Strings.search), findsOneWidget);
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
      await _expand(tester);
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
      await _expand(tester);
      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();

      final List<Widget> files = tester
          .widget<SubmenuButton>(
            find.widgetWithText(SubmenuButton, Strings.translate),
          )
          .menuChildren;
      expect(files, hasLength(1));
      expect(
        files.single,
        isA<LongMenuItem>(),
        reason: 'a long file name is cut and reads in full on a long press',
      );
    });

    testWidgets('combine needs two subtitle files', (
      WidgetTester tester,
    ) async {
      await pumpPicker(tester, api: adminApi(withFiles: true), admin: true);
      await _expand(tester);
      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();

      expect(
        tester
            .widget<MenuItemButton>(
              find.widgetWithText(MenuItemButton, Strings.combine),
            )
            .onPressed,
        isNull,
      );
    });

    testWidgets('combine from the menu queues the top file and reloads', (
      WidgetTester tester,
    ) async {
      final List<String> reads = <String>[];
      final List<Uri> combines = <Uri>[];
      await pumpPicker(
        tester,
        api: adminApi(
          withFiles: true,
          twoFiles: true,
          reads: reads,
          combines: combines,
        ),
        admin: true,
      );
      final int before = reads.length;

      await _expand(tester);
      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.combine));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, Strings.save));
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(combines, hasLength(1));
      expect(combines.single.pathSegments.skip(2), <String>[
        'versions',
        'v-uhd',
        'subtitles',
        'sf1',
        'combine',
      ]);
      expect(combines.single.queryParameters, <String, String>{
        'bottom_subtitle_id': 'sf2',
      });
      expect(reads, hasLength(before + 1));

      await tester.pump(kToastDuration + const Duration(milliseconds: 100));
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

      await _expand(tester);
      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.search));
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

    testWidgets('an admin\'s linked device works on subtitles but not pages', (
      WidgetTester tester,
    ) async {
      await pumpPicker(
        tester,
        api: adminApi(withFiles: true),
        admin: true,
        adminPages: false,
      );
      await _expand(tester);

      expect(find.byTooltip(Strings.subtitlesHeading), findsOneWidget);
      await _openMore(tester);
      expect(find.text(Strings.openVersion), findsNothing);
    });

    testWidgets('subtitle work shows under its version row', (
      WidgetTester tester,
    ) async {
      await pumpPicker(
        tester,
        api: adminApi(
          withFiles: true,
          jobs: <List<Map<String, dynamic>>>[
            <Map<String, dynamic>>[
              <String, dynamic>{
                'id': 'j1',
                'kind': 'translation',
                'status': 'queued',
                'ahead': 2,
                'created_at': '2026-10-06T10:00:00Z',
                'language': 'es',
                'subtitle_id': 'machine:v-uhd:es',
              },
              <String, dynamic>{
                'id': 'j2',
                'kind': 'trickplay',
                'status': 'queued',
                'ahead': 0,
                'created_at': '2026-10-06T10:00:00Z',
              },
            ],
          ],
        ),
        admin: true,
      );

      expect(find.text('Translation · Spanish'), findsOneWidget);
      expect(find.text('Queued · 2 jobs ahead'), findsOneWidget);
      expect(find.text('Trickplay'), findsNothing);
    });

    testWidgets('an open row keeps its work below the details', (
      WidgetTester tester,
    ) async {
      await pumpPicker(
        tester,
        api: adminApi(
          withFiles: true,
          jobs: <List<Map<String, dynamic>>>[
            <Map<String, dynamic>>[
              <String, dynamic>{
                'id': 'j1',
                'kind': 'transcription',
                'status': 'queued',
                'ahead': 0,
                'created_at': '2026-10-06T10:00:00Z',
              },
            ],
          ],
        ),
        admin: true,
      );
      final Finder line = find.text('Queued · next in line');
      final Finder details = find.text(Strings.audioHeading.toUpperCase());
      expect(details, findsNothing);
      final double closedTop = tester.getTopLeft(line).dy;

      await tester.tap(find.byIcon(Icons.expand_more));
      await tester.pumpAndSettle();

      expect(details, findsWidgets);
      final double lastDetail = tester.getRect(details.last).bottom;
      expect(tester.getTopLeft(line).dy, greaterThan(lastDetail));
      expect(tester.getTopLeft(line).dy, greaterThan(closedTop));
    });

    testWidgets('finished work reloads the row and reads as ready', (
      WidgetTester tester,
    ) async {
      final List<String> reads = <String>[];
      Map<String, dynamic> job(String status) => <String, dynamic>{
        'id': 'j1',
        'kind': 'translation',
        'status': status,
        'ahead': 0,
        'created_at': '2026-10-06T10:00:00Z',
        'language': 'en',
        'subtitle_id': 'sf1',
      };
      await pumpPicker(
        tester,
        api: adminApi(
          withFiles: true,
          reads: reads,
          jobs: <List<Map<String, dynamic>>>[
            <Map<String, dynamic>>[job('queued')],
            <Map<String, dynamic>>[job('succeeded')],
          ],
        ),
        admin: true,
      );
      final int before = reads.length;
      expect(find.text('Queued · next in line'), findsOneWidget);

      await tester.pump(kJobsBusyPoll);
      await tester.pumpAndSettle();

      expect(reads, hasLength(before + 1));
      expect(find.text('Ready · English (external) added'), findsOneWidget);
    });

    testWidgets('finished work never reads as empty before the row reloads', (
      WidgetTester tester,
    ) async {
      Map<String, dynamic> job(String status) => <String, dynamic>{
        'id': 'j1',
        'kind': 'transcription',
        'status': status,
        'ahead': 0,
        'created_at': '2026-10-06T10:00:00Z',
        'language': 'en',
        'subtitle_id': 'sf1',
      };
      await pumpPicker(
        tester,
        api: adminApi(
          withFiles: false,
          heldAfterWork: true,
          jobs: <List<Map<String, dynamic>>>[
            <Map<String, dynamic>>[job('queued')],
            <Map<String, dynamic>>[job('succeeded')],
          ],
        ),
        admin: true,
      );
      expect(find.text('Queued · next in line'), findsOneWidget);

      final Finder ready = find.text('Ready · English (external) added');
      await tester.pump(kJobsBusyPoll);
      for (int frame = 0; frame < 60 && ready.evaluate().isEmpty; frame++) {
        expect(find.text('Queued · next in line'), findsOneWidget);
        expect(find.text(Strings.jobNoSubtitle), findsNothing);
        await tester.pump(const Duration(milliseconds: 50));
      }
      expect(ready, findsOneWidget);
    });

    testWidgets('with nothing running the row reads its work once', (
      WidgetTester tester,
    ) async {
      final List<String> jobReads = <String>[];
      await pumpPicker(
        tester,
        api: adminApi(withFiles: true, jobReads: jobReads),
        admin: true,
      );
      await tester.pump(const Duration(minutes: 5));

      expect(jobReads, hasLength(1));
    });

    testWidgets('starting work reads the row\'s work at once', (
      WidgetTester tester,
    ) async {
      final List<String> jobReads = <String>[];
      await pumpPicker(
        tester,
        api: adminApi(withFiles: true, twoFiles: true, jobReads: jobReads),
        admin: true,
      );
      expect(jobReads, hasLength(1));

      await _expand(tester);
      await tester.tap(find.byTooltip(Strings.subtitlesHeading));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.combine));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, Strings.save));
      await tester.pumpAndSettle();

      expect(jobReads, hasLength(2));

      await tester.pump(kToastDuration + const Duration(milliseconds: 100));
    });

    testWidgets('a viewer never asks for version work', (
      WidgetTester tester,
    ) async {
      final List<String> paths = <String>[];
      await pumpPicker(
        tester,
        api: ApiClient(
          baseUrl: 'http://test',
          httpClient: MockClient((http.Request req) async {
            paths.add(req.url.path);
            return http.Response('{}', 200);
          }),
        ),
        admin: false,
      );
      await tester.pump(const Duration(minutes: 5));

      expect(paths.where((String p) => p.endsWith('/jobs')), isEmpty);
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

      expect(_more(), findsNothing);
      expect(find.byTooltip(Strings.subtitlesHeading), findsNothing);

      await tester.tap(find.byIcon(Icons.expand_more));
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(find.byTooltip(Strings.subtitlesHeading), findsOneWidget);
      await _openMore(tester);
      expect(find.text(Strings.openVersion), findsOneWidget);
    });
  });

  group('Play on', () {
    final List<Version> withMissing = <Version>[
      ..._shuffled.take(2),
      Version.fromJson(_json('v-hd', 'hd')..['available'] = false),
    ];

    Widget picker(
      PlayOnServer server, {
      double width = 760,
      List<Version>? versions,
    }) {
      final ApiClient api = _api(server);
      return SizedBox(
        width: width,
        child: VersionPicker(
          catalog: CatalogApi(api),
          playback: PlaybackApi(api),
          userId: 'u1',
          versions: versions ?? _shuffled,
          playOn: server.api(api),
        ),
      );
    }

    testWidgets('each available version offers Play on', (
      WidgetTester tester,
    ) async {
      await _pump(tester, picker(PlayOnServer(), versions: withMissing));
      await _expand(tester);
      await _expand(tester);
      await _expand(tester);

      expect(_more(), findsNWidgets(2));
      expect(tester.takeException(), isNull);
      for (final int row in <int>[0, 1]) {
        await _openMore(tester, row);
        expect(find.text(Strings.playOn), findsOneWidget);
        await tester.tapAt(Offset.zero);
        await tester.pumpAndSettle();
      }
    });

    testWidgets('no device that can play leaves Play on out of the menu', (
      WidgetTester tester,
    ) async {
      await _pump(tester, picker(PlayOnServer(devices: const [])));
      await _expand(tester);

      expect(find.text(Strings.play), findsOneWidget);
      await _openMore(tester);
      expect(find.text(Strings.downloadVersion), findsOneWidget);
      expect(find.text(Strings.playOn), findsNothing);
    });

    testWidgets('Play on in a row sends that row version', (
      WidgetTester tester,
    ) async {
      final PlayOnServer server = PlayOnServer();
      await _pump(tester, picker(server));

      await _expand(tester, 1);
      await _openMore(tester);
      await tester.tap(find.text(Strings.playOn));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Pixel'));
      await tester.pumpAndSettle();

      expect(server.mints, <Mint>[(device: 'd2', version: 'v-fhd')]);
      expect(find.text(Strings.playOnPlaying('Pixel')), findsOneWidget);
      await tester.pump(kToastDuration);
    });

    testWidgets('a phone row offers Play on in its details', (
      WidgetTester tester,
    ) async {
      await _pump(tester, picker(PlayOnServer(), width: 280));

      expect(_more(), findsNothing);

      await tester.tap(find.byIcon(Icons.expand_more).first);
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      await _openMore(tester);
      expect(find.text(Strings.playOn), findsOneWidget);
    });

    testWidgets('the dialog hands Play on to its picker', (
      WidgetTester tester,
    ) async {
      final PlayOnServer server = PlayOnServer();
      final ApiClient api = _api(server);
      await _pump(
        tester,
        Builder(
          builder: (BuildContext context) => TextButton(
            onPressed: () => AllVersionsDialog.show(
              context,
              catalog: CatalogApi(api),
              playback: PlaybackApi(api),
              userId: 'u1',
              versions: orderedVersions(_shuffled),
              playOn: server.api(api),
            ),
            child: const Text('open'),
          ),
        ),
        size: const Size(1200, 900),
      );

      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();

      await _expand(tester);
      await _openMore(tester);
      expect(find.text(Strings.playOn), findsOneWidget);
    });
  });
}
