import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/admin/form_dialog.dart';
import 'package:shadowmask/components/admin/trickplay_sheets.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/version_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _movieCard() => <String, dynamic>{
  'type': 'movie',
  'id': 'm1',
  'title': 'Blade Runner',
  'year': 1982,
};

Map<String, dynamic> _episodeCard() => <String, dynamic>{
  'type': 'episode',
  'id': 'e1',
  'season_id': 's1',
  'series_id': 'sh1',
  'series_title': 'The Expanse',
  'season_number': 2,
  'number': 4,
  'title': 'Home',
};

Map<String, dynamic> _detail({
  required bool episode,
  required List<Map<String, dynamic>> subtitleFiles,
  bool available = true,
}) => <String, dynamic>{
  'id': 'v1',
  'title': <String, dynamic>{
    'type': episode ? 'episode' : 'movie',
    'id': episode ? 'e1' : 'm1',
  },
  'library_id': 'lib',
  'quality': 'fhd',
  'container': 'mkv',
  'duration_ms': 100000,
  'available': available,
  'video': <dynamic>[
    <String, dynamic>{
      'index': 0,
      'codec': 'h264',
      'width': 1280,
      'height': 720,
      'bit_depth': 8,
      'frame_rate': 24,
    },
  ],
  'audio': <dynamic>[
    <String, dynamic>{
      'index': 1,
      'codec': 'aac',
      'channels': 6,
      'language': 'eng',
    },
  ],
  'subtitles': <dynamic>[
    <String, dynamic>{
      'index': 2,
      'language': 'eng',
      'format': 'srt',
      'forced': false,
      'default': true,
    },
  ],
  'subtitle_files': subtitleFiles,
  'trickplay': <dynamic>[
    <String, dynamic>{
      'interval_ms': 10000,
      'columns': 10,
      'rows': 10,
      'tile_width': 160,
      'tile_height': 90,
      'sheets': 2,
    },
  ],
};

const String _pixel =
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQ'
    'GAhKmMIQAAAABJRU5ErkJggg==';

Map<String, dynamic> _file(String id, String language) => <String, dynamic>{
  'id': id,
  'language': language,
  'format': 'srt',
  'source': 'external',
};

Map<String, dynamic> _episode() => <String, dynamic>{
  ..._episodeCard(),
  'added_at': '2026-08-17T09:00:00Z',
  'updated_at': '2026-08-17T09:00:00Z',
  'artwork': <String, dynamic>{},
};

final List<String> _refreshBodies = <String>[];

Future<({List<String> paths, List<String> routes})> _pump(
  WidgetTester tester, {
  bool episode = false,
  bool resolveSeries = true,
  bool targetEdited = false,
  bool available = true,
  List<Map<String, dynamic>>? subtitleFiles,
  Size size = const Size(1600, 1600),
}) async {
  final List<String> seen = <String>[];
  final List<String> routes = <String>[];
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen.add(req.url.path);
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
      if (req.url.path.startsWith('/api/v1/trickplay/')) {
        return http.Response.bytes(base64Decode(_pixel), 200);
      }
      if (req.method == 'GET' && req.url.path.contains('/subtitles/')) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'content': '1\n00:00:01 --> 00:00:02\nHallo',
          }),
          200,
        );
      }
      if (req.url.path.startsWith('/api/v1/admin/versions/')) {
        return http.Response('', 204);
      }
      if (req.url.path.endsWith('/refresh')) {
        _refreshBodies.add(req.body);
        return http.Response('', 202);
      }
      if (req.url.path.contains('/episodes/')) {
        return resolveSeries
            ? http.Response(jsonEncode(_episode()), 200)
            : http.Response('{}', 404);
      }
      if (req.url.path == '/api/v1/movies/m1' ||
          req.url.path == '/api/v1/series/sh1') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': episode ? 'sh1' : 'm1',
            'title': episode ? 'The Expanse' : 'Blade Runner',
            'manually_edited': targetEdited,
          }),
          200,
        );
      }
      if (req.url.path == '/api/v1/titles/batch') {
        return http.Response(
          jsonEncode(<dynamic>[episode ? _episodeCard() : _movieCard()]),
          200,
        );
      }
      if (req.url.path.startsWith('/api/v1/versions/')) {
        return http.Response(
          jsonEncode(
            _detail(
              episode: episode,
              available: available,
              subtitleFiles:
                  subtitleFiles ?? <Map<String, dynamic>>[_file('sf1', 'nld')],
            ),
          ),
          200,
        );
      }
      return http.Response('{}', 404);
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
        onGenerateRoute: (RouteSettings settings) {
          routes.add(settings.name ?? '');
          final bool first = routes.length == 1;
          return MaterialPageRoute<void>(
            settings: settings,
            builder: (_) => first
                ? VersionPage(api: api, id: 'v1')
                : const SizedBox.shrink(),
          );
        },
      ),
    ),
  );
  await tester.pumpAndSettle();
  return (paths: seen, routes: routes);
}

Finder _section(String title) =>
    find.ancestor(of: find.text(title), matching: find.byType(SectionBlock));

Finder _inSection(String title, Finder matching) =>
    find.descendant(of: _section(title), matching: matching);

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    _refreshBodies.clear();
  });

  testWidgets('the admin page no longer offers playback', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.play), findsNothing);
  });

  testWidgets('the header is two columns on a wide screen', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final Rect summary = tester.getRect(find.byType(SectionBlock).first);
    final Rect linked = tester.getRect(_section(Strings.linkedTitleHeading));

    expect(linked.left, greaterThan(summary.right - 1));
    expect(linked.top, summary.top);
  });

  testWidgets('the linked title sets its facts beside the poster when wide', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final Rect poster = tester.getRect(
      _inSection(Strings.linkedTitleHeading, find.byType(CardArt)),
    );
    final Rect id = tester.getRect(
      _inSection(Strings.linkedTitleHeading, find.text(Strings.factId)),
    );

    expect(id.left, greaterThan(poster.right - 1));
    expect(id.top, lessThan(poster.bottom));
  });

  testWidgets('a phone renders the whole page without overflowing', (
    WidgetTester tester,
  ) async {
    await _pump(tester, size: const Size(420, 1600));

    // Before R4 this page could not be rendered narrow at all: the track
    // tables forced a 720px row and overflowed.
    expect(tester.takeException(), isNull);
    final Rect summary = tester.getRect(find.byType(SectionBlock).first);
    final Rect linked = tester.getRect(_section(Strings.linkedTitleHeading));
    expect(linked.top, greaterThan(summary.top));
    expect(linked.left, summary.left);
  });

  testWidgets('every action sits in the section it acts on', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(
      _inSection(
        Strings.videoHeading,
        find.widgetWithText(OutlinedButton, Strings.upscale),
      ),
      findsOneWidget,
    );
    expect(
      _inSection(Strings.audioHeading, find.byTooltip(Strings.transcribe)),
      findsOneWidget,
      reason: 'a row action is an icon, so its tooltip is what names it',
    );
    for (final String label in <String>[
      Strings.combineSubtitles,
      Strings.searchSubtitles,
    ]) {
      expect(
        _inSection(
          Strings.subtitlesHeading,
          find.widgetWithText(OutlinedButton, label),
        ),
        findsOneWidget,
        reason: 'missing the $label action',
      );
    }
    expect(
      _inSection(Strings.subtitlesHeading, find.byTooltip(Strings.translate)),
      findsOneWidget,
    );
    expect(
      _inSection(
        Strings.linkedTitleHeading,
        find.widgetWithText(OutlinedButton, Strings.relink),
      ),
      findsOneWidget,
    );
    for (final String label in <String>[
      Strings.upscale,
      Strings.combineSubtitles,
      Strings.searchSubtitles,
      Strings.relink,
    ]) {
      expect(
        find.text(label),
        findsOneWidget,
        reason: '$label is offered more than once',
      );
    }
    // The row actions are icons, so their tooltip is the only place the name
    // appears.
    for (final String label in <String>[
      Strings.transcribe,
      Strings.translate,
    ]) {
      expect(
        find.byTooltip(label),
        findsOneWidget,
        reason: '$label is offered more than once',
      );
    }
  });

  testWidgets('transcribe opens on the track its row belongs to', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(find.byTooltip(Strings.transcribe));
    await tester.pumpAndSettle();

    expect(find.text(Strings.fieldSourceLanguage), findsOneWidget);
    expect(find.text(Strings.fieldTargetHeight), findsNothing);
  });

  testWidgets('only subtitle files carry row actions', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.byTooltip(Strings.viewText), findsOneWidget);
    expect(find.byTooltip(Strings.translate), findsOneWidget);
    expect(find.byTooltip(Strings.rename), findsOneWidget);
    expect(find.byTooltip(Strings.delete), findsOneWidget);
  });

  testWidgets('viewing a subtitle reads its text from the server', (
    WidgetTester tester,
  ) async {
    final ({List<String> paths, List<String> routes}) run = await _pump(tester);

    await tester.tap(find.byTooltip(Strings.viewText));
    await tester.pumpAndSettle();

    expect(find.textContaining('Hallo'), findsOneWidget);
    expect(
      run.paths,
      contains('/api/v1/admin/versions/v1/subtitles/sf1'),
      reason: 'the dialog must read the file it was opened on',
    );
  });

  testWidgets('translate takes its source from the row it opens on', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(find.byTooltip(Strings.translate));
    await tester.pumpAndSettle();

    expect(find.text(Strings.fieldTargetLanguage), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(FormDialog),
        matching: find.byType(FieldLabel),
      ),
      findsOneWidget,
    );
  });

  testWidgets('with no files there is nothing to translate or combine', (
    WidgetTester tester,
  ) async {
    await _pump(tester, subtitleFiles: <Map<String, dynamic>>[]);

    final OutlinedButton combine = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, Strings.combineSubtitles),
    );
    final OutlinedButton search = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, Strings.searchSubtitles),
    );

    expect(find.widgetWithText(TextButton, Strings.translate), findsNothing);
    expect(combine.onPressed, isNull);
    expect(search.onPressed, isNotNull);
  });

  testWidgets('upscale offers only the heights above this version', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.upscale));
    await tester.pumpAndSettle();

    expect(find.text('${Strings.currentResolution}: 1280×720'), findsOneWidget);
    expect(find.text(Strings.height480), findsNothing);
    expect(find.text(Strings.height1080), findsOneWidget);
  });

  testWidgets('the linked title section names the movie it is attached to', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(
      _inSection(Strings.linkedTitleHeading, find.byType(CardArt)),
      findsOneWidget,
    );
    expect(find.text('Blade Runner'), findsOneWidget);
    expect(find.text(Strings.kindMovie), findsOneWidget);
    expect(find.text('1982'), findsOneWidget);
    expect(find.text('m1'), findsOneWidget);
    expect(find.widgetWithText(TextButton, Strings.openTitle), findsOneWidget);
  });

  testWidgets('an episode names its series and number, and drops the year', (
    WidgetTester tester,
  ) async {
    await _pump(tester, episode: true);

    expect(find.text('The Expanse · S02E04 · Home'), findsOneWidget);
    expect(find.text(Strings.kindEpisode), findsOneWidget);
    expect(find.text(Strings.factYear), findsNothing);
  });

  testWidgets('refreshing a movie version targets the movie', (
    WidgetTester tester,
  ) async {
    final ({List<String> paths, List<String> routes}) run = await _pump(tester);

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(run.paths, contains('/api/v1/movies/m1/refresh'));

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('refreshing an episode version targets its series, not a movie', (
    WidgetTester tester,
  ) async {
    final ({List<String> paths, List<String> routes}) run = await _pump(
      tester,
      episode: true,
    );

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(run.paths, contains('/api/v1/series/sh1/refresh'));
    expect(
      run.paths.where((String p) => p.startsWith('/api/v1/movies/')),
      isEmpty,
      reason: 'an episode must never be refreshed through the movie route',
    );

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a hand-edited movie offers the discard choice here too', (
    WidgetTester tester,
  ) async {
    await _pump(tester, targetEdited: true);

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(
      find.widgetWithText(TextButton, Strings.forceRefresh),
      findsOneWidget,
      reason:
          'the same action must not behave differently depending on which page it is invoked from',
    );
    expect(
      find.widgetWithText(FilledButton, Strings.keepEdits),
      findsOneWidget,
    );
  });

  testWidgets('a hand-edited series behind an episode offers the choice', (
    WidgetTester tester,
  ) async {
    await _pump(tester, episode: true, targetEdited: true);

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(
      find.widgetWithText(TextButton, Strings.forceRefresh),
      findsOneWidget,
      reason:
          'the flag that matters is the series one, since the series is the target',
    );
  });

  testWidgets('an unedited target still gets the plain confirmation', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(
      find.widgetWithText(TextButton, Strings.forceRefresh),
      findsNothing,
      reason:
          'there are no edits to discard, so offering the choice would be noise',
    );
  });

  testWidgets('discarding edits sends force and reloads the page', (
    WidgetTester tester,
  ) async {
    final ({List<String> paths, List<String> routes}) run = await _pump(
      tester,
      targetEdited: true,
    );
    final int before = run.paths
        .where((String p) => p.startsWith('/api/v1/versions/'))
        .length;

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(TextButton, Strings.forceRefresh));
    await tester.pumpAndSettle();

    expect(_refreshBodies.single, contains('"force":true'));
    expect(
      run.paths.where((String p) => p.startsWith('/api/v1/versions/')).length,
      before + 1,
      reason:
          'the flag has just changed server-side, so the page has to re-read it',
    );

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('an episode with no resolvable series cannot be refreshed', (
    WidgetTester tester,
  ) async {
    await _pump(tester, episode: true, resolveSeries: false);

    final OutlinedButton refresh = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );

    expect(refresh.onPressed, isNull);
    expect(
      find.byTooltip(Strings.refreshUnavailable),
      findsOneWidget,
      reason: 'a disabled button with no explanation reads as a bug',
    );
  });

  testWidgets('the available fact carries the versions table colour', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final Text yes = tester.widget<Text>(
      _inSection('FHD mkv', find.text(Strings.yes)),
    );

    expect(yes.style?.color, Tokens.dark.ok);
  });

  testWidgets('an unavailable version reads in the danger colour', (
    WidgetTester tester,
  ) async {
    await _pump(tester, available: false);

    final Text no = tester.widget<Text>(
      _inSection('FHD mkv', find.text(Strings.no)),
    );

    expect(no.style?.color, Tokens.dark.danger);
  });

  testWidgets('a version with no file cannot be relinked, and says why', (
    WidgetTester tester,
  ) async {
    await _pump(tester, available: false);

    final OutlinedButton relink = tester.widget<OutlinedButton>(
      _inSection(
        Strings.linkedTitleHeading,
        find.widgetWithText(OutlinedButton, Strings.relink),
      ),
    );

    expect(
      relink.onPressed,
      isNull,
      reason: 'a relink would only fail at the probe',
    );
    expect(
      find.byTooltip(Strings.relinkBlockedVersion),
      findsOneWidget,
      reason: 'the block names removing the version as the way out',
    );
  });

  testWidgets('the summary card offers delete, and it leaves the page', (
    WidgetTester tester,
  ) async {
    final ({List<String> paths, List<String> routes}) run = await _pump(tester);

    expect(
      _inSection(
        'FHD mkv',
        find.widgetWithText(OutlinedButton, Strings.delete),
      ),
      findsOneWidget,
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.delete));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.remove));
    await tester.pumpAndSettle();

    expect(run.paths, contains('/api/v1/admin/versions/v1'));
    expect(run.routes.last, contains('m1'));

    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
  });

  testWidgets('the trickplay section renders every sheet, not one cell', (
    WidgetTester tester,
  ) async {
    final ({List<String> paths, List<String> routes}) run = await _pump(tester);

    expect(
      _inSection(Strings.versionTrickplay, find.byType(TrickplaySheets)),
      findsOneWidget,
    );
    expect(find.textContaining('160×90'), findsOneWidget);

    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 50)),
    );
    await tester.pump();

    expect(run.paths, contains('/api/v1/trickplay/v1/1'));
    expect(run.paths, contains('/api/v1/trickplay/v1/2'));
    expect(
      find.descendant(
        of: find.byType(TrickplaySheets),
        matching: find.byType(CustomPaint),
      ),
      findsNWidgets(2),
    );
  });

  testWidgets('cancelling a dialog does not refetch the page', (
    WidgetTester tester,
  ) async {
    final ({List<String> paths, List<String> routes}) run = await _pump(tester);
    final int before = run.paths.length;

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.upscale));
    await tester.pumpAndSettle();
    expect(find.text(Strings.height1080), findsOneWidget);

    await tester.tap(find.byIcon(Icons.close).last);
    await tester.pumpAndSettle();

    expect(
      run.paths.length,
      before,
      reason: 'a dismissed dialog changed nothing, so nothing needs reloading',
    );
  });

  testWidgets('a phone folds the subtitle actions so the row still reads', (
    WidgetTester tester,
  ) async {
    // Four icon buttons took 232 of a 360px phone and squeezed the format and
    // language cells to zero width, which put every character on its own line.
    await _pump(tester, size: const Size(360, 2400));

    expect(
      find.byIcon(Icons.more_vert),
      findsWidgets,
      reason: 'the four actions fold into one menu',
    );
    expect(
      find.byIcon(Icons.drive_file_rename_outline),
      findsNothing,
      reason: 'and are reached through it rather than sitting in the row',
    );

    for (final String value in <String>['SRT', 'ENG', 'NLD']) {
      final Size cell = tester.getSize(find.text(value).first);
      expect(cell.width, greaterThan(0), reason: value);
      expect(cell.height, lessThan(22), reason: '$value wrapped');
    }
  });

  testWidgets('delete stays red once it is inside the folded menu', (
    WidgetTester tester,
  ) async {
    await _pump(tester, size: const Size(360, 2400));

    await tester.tap(find.byIcon(Icons.more_vert).first);
    await tester.pumpAndSettle();

    final Finder item = find.ancestor(
      of: find.text(Strings.delete),
      matching: find.byType(MenuItemButton),
    );
    final BuildContext context = tester.element(item);
    final Icon icon = tester.widget<Icon>(
      find.descendant(of: item, matching: find.byIcon(Icons.delete_outline)),
    );

    expect(icon.color, Theme.of(context).colorScheme.error);
  });

  testWidgets('a roomy window keeps every subtitle action in the row', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.byIcon(Icons.drive_file_rename_outline), findsWidgets);
    expect(find.byIcon(Icons.more_vert), findsNothing);
  });
}
