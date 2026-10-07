import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/episode_play_button.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/next_episode.dart';
import 'package:shared_preferences/shared_preferences.dart';

NextEpisode _next({int progress = 0, String title = 'Three'}) => (
  episode: Episode(id: 'e3', seasonId: 'se2', number: 3, title: title),
  season: 2,
  progressPercent: progress,
);

Map<String, dynamic> _version(String id, {bool available = true}) =>
    <String, dynamic>{
      'id': id,
      'title': <String, dynamic>{'type': 'episode', 'id': 'e3'},
      'library_id': 'lib1',
      'container': 'mkv',
      'quality': 'fhd',
      'available': available,
    };

Map<String, dynamic> _resumeEntry(String versionId) => <String, dynamic>{
  'card': <String, dynamic>{
    'title': <String, dynamic>{'type': 'episode', 'id': 'e3'},
    'display_title': 'Three',
    'duration_ms': 600000,
    'progress_percent': 40,
  },
  'progress': <String, dynamic>{'version_id': versionId},
};

class _Pushes extends NavigatorObserver {
  final List<String> names = <String>[];

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) {
    final String? name = route.settings.name;
    if (name != null) {
      names.add(name);
    }
  }
}

Future<_Pushes> _pump(
  WidgetTester tester,
  Future<NextEpisode?> next, {
  List<Map<String, dynamic>> versions = const <Map<String, dynamic>>[],
  List<String> inProgress = const <String>[],
  List<String>? seen,
  double width = 800,
  bool withSeason = true,
}) async {
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen?.add(req.url.path);
      if (req.url.path.endsWith('/continue')) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'now_playing': <dynamic>[],
            'in_progress': inProgress.map(_resumeEntry).toList(),
            'next_episodes': <dynamic>[],
          }),
          200,
        );
      }
      if (req.url.path.endsWith('/versions')) {
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
      return http.Response('', 404);
    }),
  );
  final _Pushes pushes = _Pushes();
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      navigatorObservers: <NavigatorObserver>[pushes],
      builder: (BuildContext context, Widget? child) =>
          ToastHost(child: child ?? const SizedBox.shrink()),
      onGenerateRoute: (RouteSettings s) => MaterialPageRoute<void>(
        settings: s,
        builder: (BuildContext _) => Scaffold(
          body: s.name == '/'
              ? Align(
                  alignment: Alignment.topLeft,
                  child: SizedBox(
                    width: width,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        EpisodePlayButton(
                          catalog: CatalogApi(api),
                          userId: 'u1',
                          seriesId: 's1',
                          next: next,
                          withSeason: withSeason,
                        ),
                        NextEpisodeNote(next: next, withSeason: withSeason),
                        const Text('below'),
                      ],
                    ),
                  ),
                )
              : const SizedBox.shrink(),
        ),
      ),
    ),
  );
  await tester.pump();
  pushes.names.clear();
  return pushes;
}

Future<void> _press(WidgetTester tester, String label) async {
  await tester.tap(find.text(label));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets(
    'the note names the episode once known, and nothing below it moves',
    (WidgetTester tester) async {
      final Completer<NextEpisode?> next = Completer<NextEpisode?>();
      await _pump(tester, next.future);

      final Rect below = tester.getRect(find.text('below'));
      expect(find.text('Up next: S02E03 · Three'), findsNothing);
      expect(find.byType(Tooltip), findsNothing);

      next.complete(_next());
      await tester.pumpAndSettle();

      expect(find.text(Strings.play), findsOneWidget);
      expect(find.text('Up next: S02E03 · Three'), findsOneWidget);
      expect(tester.getRect(find.text('below')), below);
    },
  );

  testWidgets('the episode after Up next is bold and the lead is not', (
    WidgetTester tester,
  ) async {
    await _pump(tester, Future<NextEpisode?>.value(_next()));
    await tester.pump();

    final Text note = tester.widget<Text>(find.text('Up next: S02E03 · Three'));
    final List<InlineSpan> parts = (note.textSpan! as TextSpan).children!;
    final TextSpan lead = parts.first as TextSpan;
    final TextSpan episode = parts.last as TextSpan;
    expect(lead.text, Strings.upNextLead);
    expect(lead.style?.fontWeight, isNull);
    expect(episode.text, 'S02E03 · Three');
    expect(episode.style?.fontWeight, FontWeight.w700);
  });

  testWidgets('hovering the button shows the same note', (
    WidgetTester tester,
  ) async {
    await _pump(tester, Future<NextEpisode?>.value(_next()));
    await tester.pump();

    expect(find.byTooltip('Up next: S02E03 · Three'), findsOneWidget);
  });

  testWidgets('the button keeps the action height of the buttons beside it', (
    WidgetTester tester,
  ) async {
    await _pump(tester, Future<NextEpisode?>.value(_next()));
    await tester.pump();

    expect(
      tester.getSize(find.byType(FilledButton)).height,
      kActionControlHeight,
    );
  });

  testWidgets('an episode with progress reads Resume, and the note is spoken', (
    WidgetTester tester,
  ) async {
    await _pump(tester, Future<NextEpisode?>.value(_next(progress: 40)));
    await tester.pump();

    expect(find.text(Strings.resumeAction), findsOneWidget);
    expect(find.text('Up next: S02E03 · Three'), findsOneWidget);
    expect(
      find.bySemanticsLabel('Up next: Season 2, Episode 3, Three'),
      findsOneWidget,
      reason: 'a screen reader hears the season spelled out, not S02',
    );
  });

  testWidgets('the season page leaves the season out of the note', (
    WidgetTester tester,
  ) async {
    await _pump(tester, Future<NextEpisode?>.value(_next()), withSeason: false);
    await tester.pump();

    expect(find.text('Up next: E03 · Three'), findsOneWidget);
    expect(find.byTooltip('Up next: E03 · Three'), findsOneWidget);
  });

  testWidgets('a long note ends in an ellipsis within its row', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      Future<NextEpisode?>.value(
        _next(
          title: 'A title far too long to fit under any play button at all',
        ),
      ),
      width: 240,
    );
    await tester.pump();

    final Finder note = find.textContaining('A title far too long');
    final Text caption = tester.widget<Text>(note);
    expect(caption.maxLines, 1);
    expect(caption.overflow, TextOverflow.ellipsis);
    expect(tester.getSize(note).width, lessThanOrEqualTo(240));
  });

  testWidgets('a press before the episode is known waits for it, then plays', (
    WidgetTester tester,
  ) async {
    final Completer<NextEpisode?> next = Completer<NextEpisode?>();
    final List<String> seen = <String>[];
    final _Pushes pushes = await _pump(
      tester,
      next.future,
      versions: <Map<String, dynamic>>[_version('v1')],
      seen: seen,
    );

    await tester.tap(find.text(Strings.play));
    await tester.pump();
    expect(pushes.names, isEmpty);

    next.complete(_next());
    await tester.pumpAndSettle();

    expect(
      seen,
      contains('/api/v1/series/s1/seasons/se2/episodes/e3/versions'),
    );
    expect(pushes.names.last, '/watch?version=v1');
  });

  testWidgets('the one in-progress version resumes without a prompt', (
    WidgetTester tester,
  ) async {
    final _Pushes pushes = await _pump(
      tester,
      Future<NextEpisode?>.value(_next(progress: 40)),
      versions: <Map<String, dynamic>>[_version('v1'), _version('v2')],
      inProgress: <String>['v2'],
    );
    await tester.pump();

    await _press(tester, Strings.resumeAction);

    expect(pushes.names.last, '/watch?version=v2');
  });

  testWidgets('several versions with nothing in progress open the menu', (
    WidgetTester tester,
  ) async {
    final _Pushes pushes = await _pump(
      tester,
      Future<NextEpisode?>.value(_next()),
      versions: <Map<String, dynamic>>[
        _version('v1'),
        _version('v2'),
        _version('v3', available: false),
      ],
    );
    await tester.pump();

    await _press(tester, Strings.play);

    expect(pushes.names, isEmpty);
    expect(find.byType(MenuItemButton), findsNWidgets(2));

    await tester.tap(find.byType(MenuItemButton).last);
    await tester.pumpAndSettle();

    expect(pushes.names.last, '/watch?version=v2');
  });

  testWidgets(
    'an episode with no playable file says there is nothing to play',
    (WidgetTester tester) async {
      final _Pushes pushes = await _pump(
        tester,
        Future<NextEpisode?>.value(_next()),
        versions: <Map<String, dynamic>>[_version('v1', available: false)],
      );
      await tester.pump();

      await _press(tester, Strings.play);

      expect(pushes.names, isEmpty);
      expect(find.text(Strings.nothingToPlay), findsOneWidget);
      await tester.pump(kToastDuration + const Duration(milliseconds: 100));
    },
  );

  testWidgets('no episode to resolve says there is nothing to play', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, Future<NextEpisode?>.value(), seen: seen);
    await tester.pump();

    await _press(tester, Strings.play);

    expect(find.text(Strings.nothingToPlay), findsOneWidget);
    expect(seen, isEmpty);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });
}
