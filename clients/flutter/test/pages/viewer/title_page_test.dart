import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/components/progress_bar.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/version_menu.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/title_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _version(String id, String label) => <String, dynamic>{
  'id': id,
  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
  'library_id': 'lib1',
  'path': '/media/$id.mkv',
  'container': 'mkv',
  'quality': label,
  'available': true,
  'added_at': '2026-08-17T09:00:00Z',
  'updated_at': '2026-08-17T09:00:00Z',
};

Map<String, dynamic> _inProgress(String versionId) => <String, dynamic>{
  'card': <String, dynamic>{
    'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
    'display_title': 'Arrival',
    'artwork': <String, dynamic>{},
    'duration_ms': 6000000,
    'progress_percent': 40,
  },
  'progress': <String, dynamic>{'version_id': versionId},
};

Map<String, dynamic> _entry(String id, String title) => <String, dynamic>{
  'title_id': id,
  'kind': 'movie',
  'display_title': title,
  'role': 'actor',
};

bool _isActorProbe(String path) =>
    path.startsWith('/api/v1/people/') && path != '/api/v1/people/batch';

Map<String, dynamic> _member(String id, String title) => <String, dynamic>{
  'id': id,
  'title': title,
  'artwork': <String, dynamic>{},
};

Map<String, dynamic> _collection(
  String id,
  String name,
  List<Map<String, dynamic>> items,
) => <String, dynamic>{
  'id': id,
  'name': name,
  'movies': items.map((Map<String, dynamic> m) => m['id']).toList(),
  'items': items,
  'artwork': <String, dynamic>{},
};

Map<String, dynamic> _actor(String id, String name, int order) =>
    <String, dynamic>{
      'person': <String, dynamic>{'id': id, 'name': name},
      'role': 'actor',
      'order': order,
    };

String _actorName(List<Map<String, dynamic>> credits, String id) {
  for (final Map<String, dynamic> c in credits) {
    final Map<String, dynamic> person = c['person'] as Map<String, dynamic>;
    if (person['id'] == id) {
      return person['name'] as String;
    }
  }
  return id;
}

ApiClient _api({
  required List<Map<String, dynamic>> versions,
  List<Map<String, dynamic>> resuming = const <Map<String, dynamic>>[],
  List<Map<String, dynamic>> credits = const <Map<String, dynamic>>[],
  Map<String, List<Map<String, dynamic>>> filmographies =
      const <String, List<Map<String, dynamic>>>{},
  List<Map<String, dynamic>> collections = const <Map<String, dynamic>>[],
  List<String>? seen,
}) {
  bool cleared = false;
  return ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen?.add(req.url.path);
      if (req.url.path == '/api/v1/people/batch') {
        return http.Response(
          jsonEncode(
            credits.map((Map<String, dynamic> c) {
              final Map<String, dynamic> p =
                  c['person'] as Map<String, dynamic>;
              return <String, dynamic>{
                'id': p['id'],
                'name': p['name'],
                'artwork': <String, dynamic>{},
              };
            }).toList(),
          ),
          200,
        );
      }
      if (req.url.path.startsWith('/api/v1/people/')) {
        final String id = req.url.pathSegments.last;
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': id,
            'name': _actorName(credits, id),
            'filmography': filmographies[id] ?? const <Map<String, dynamic>>[],
          }),
          200,
        );
      }
      if (req.url.path == '/api/v1/users/self') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'u1',
            'username': 'pat',
            'role': 'user',
          }),
          200,
        );
      }
      if (req.url.path.endsWith('/collections')) {
        return http.Response(jsonEncode(collections), 200);
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
            'credits': credits,
          }),
          200,
        );
      }
      if (req.url.path.endsWith('/continue')) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'now_playing': <dynamic>[],
            'in_progress': cleared ? <dynamic>[] : resuming,
            'next_episodes': <dynamic>[],
          }),
          200,
        );
      }
      if (req.url.path.contains('/progress/')) {
        if (req.method == 'DELETE') {
          cleared = true;
          return http.Response('', 204);
        }
        return http.Response(
          jsonEncode(<String, dynamic>{
            'position_ms': cleared || resuming.isEmpty ? 0 : 2400000,
          }),
          200,
        );
      }
      return http.Response('[]', 200);
    }),
  );
}

Future<void> _pump(WidgetTester tester, ApiClient api) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? TitlePage(api: api, kind: 'movie', id: 'm1')
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the poster opens the version menu when nothing resumes', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[
          _version('v1', 'fhd'),
          _version('v2', 'sd'),
        ],
      ),
    );

    expect(find.byType(VersionMenuItem), findsNothing);

    await tester.tap(find.byType(PosterPlay));
    await tester.pumpAndSettle();

    expect(find.byType(VersionMenuItem), findsNWidgets(2));
  });

  testWidgets('picking a version from the poster menu plays it', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[
          _version('v1', 'fhd'),
          _version('v2', 'sd'),
        ],
      ),
    );

    await tester.tap(find.byType(PosterPlay));
    await tester.pumpAndSettle();

    await tester.tap(
      find.descendant(
        of: find.byType(VersionMenuItem),
        matching: find.textContaining('SD'),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('went to /watch?version=v2'), findsOneWidget);
  });

  testWidgets('a single version plays straight from the poster', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(versions: <Map<String, dynamic>>[_version('v1', 'fhd')]),
    );

    await tester.tap(find.byType(PosterPlay));
    await tester.pumpAndSettle();

    expect(find.text('went to /watch?version=v1'), findsOneWidget);
  });

  testWidgets('dismissing a resume turns the page back to Play', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        resuming: <Map<String, dynamic>>[_inProgress('v1')],
      ),
    );

    expect(find.text(Strings.resumeAction), findsOneWidget);
    expect(find.text(Strings.play), findsNothing);

    await tester.tap(find.byTooltip(Strings.dismissResume));
    await tester.pumpAndSettle();

    expect(find.text(Strings.resumeAction), findsNothing);
    expect(find.byTooltip(Strings.dismissResume), findsNothing);
    expect(find.text(Strings.play), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a resumable title shows its progress on the poster', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        resuming: <Map<String, dynamic>>[_inProgress('v1')],
      ),
    );

    expect(
      tester.widget<PosterPlay>(find.byType(PosterPlay)).progressPercent,
      40,
      reason: 'the progress moved off the button and onto the artwork',
    );
    expect(find.byType(ProgressBar), findsOneWidget);
  });

  testWidgets('no cast means no actor rail at all', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(
      tester,
      _api(versions: <Map<String, dynamic>>[_version('v1', 'fhd')], seen: seen),
    );

    expect(find.textContaining('More with'), findsNothing);
    expect(seen.any(_isActorProbe), isFalse);
  });

  testWidgets('the rail walks past a thin actor to one worth showing', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        credits: <Map<String, dynamic>>[
          _actor('p2', 'Jeremy Renner', 4),
          _actor('p1', 'Amy Adams', 0),
        ],
        filmographies: <String, List<Map<String, dynamic>>>{
          'p1': <Map<String, dynamic>>[
            _entry('m1', 'Arrival'),
            _entry('m2', 'Nocturnal Animals'),
            _entry('m3', 'Doubt'),
          ],
          'p2': <Map<String, dynamic>>[
            _entry('m4', 'The Hurt Locker'),
            _entry('m5', 'Wind River'),
            _entry('m6', 'The Town'),
          ],
        },
      ),
    );

    expect(find.text(Strings.moreWithActor('Jeremy Renner')), findsOneWidget);
    expect(find.text(Strings.moreWithActor('Amy Adams')), findsNothing);
    expect(find.text('Wind River'), findsOneWidget);
  });

  testWidgets('nobody worth showing means no rail at all', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        credits: <Map<String, dynamic>>[_actor('p1', 'Amy Adams', 0)],
        filmographies: <String, List<Map<String, dynamic>>>{
          'p1': <Map<String, dynamic>>[
            _entry('m1', 'Arrival'),
            _entry('m2', 'Nocturnal Animals'),
          ],
        },
      ),
    );

    expect(find.textContaining('More with'), findsNothing);
    expect(find.text('Nocturnal Animals'), findsNothing);
  });

  testWidgets('probing carries on past the first burst and then stops', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        credits: <Map<String, dynamic>>[
          for (int n = 1; n <= 7; n++) _actor('p$n', 'Actor $n', n),
        ],
        filmographies: <String, List<Map<String, dynamic>>>{
          'p7': <Map<String, dynamic>>[
            _entry('m4', 'Late Arrival'),
            _entry('m5', 'Wind River'),
            _entry('m6', 'The Town'),
          ],
        },
        seen: seen,
      ),
    );

    final int probes = seen.where(_isActorProbe).length;
    expect(
      probes,
      greaterThan(kMaxActorProbes),
      reason: 'a burst is bounded, but the block keeps going past one burst',
    );
    expect(probes, 7, reason: 'every actor is probed once and never twice');
    expect(find.text(Strings.moreWithActor('Actor 7')), findsOneWidget);
    expect(find.text('Wind River'), findsOneWidget);
  });

  testWidgets('every actor worth showing gets a rail of their own', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        credits: <Map<String, dynamic>>[
          _actor('p1', 'Amy Adams', 0),
          _actor('p2', 'Jeremy Renner', 1),
        ],
        filmographies: <String, List<Map<String, dynamic>>>{
          'p1': <Map<String, dynamic>>[
            _entry('m2', 'Nocturnal Animals'),
            _entry('m3', 'Doubt'),
            _entry('m4', 'The Master'),
          ],
          'p2': <Map<String, dynamic>>[
            _entry('m5', 'The Hurt Locker'),
            _entry('m6', 'Wind River'),
            _entry('m7', 'The Town'),
          ],
        },
      ),
    );

    expect(find.text(Strings.moreWithActor('Amy Adams')), findsOneWidget);
    expect(find.text(Strings.moreWithActor('Jeremy Renner')), findsOneWidget);
  });

  testWidgets('a title shown in one rail does not repeat in the next', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        credits: <Map<String, dynamic>>[
          _actor('p1', 'Amy Adams', 0),
          _actor('p2', 'Jeremy Renner', 1),
        ],
        filmographies: <String, List<Map<String, dynamic>>>{
          'p1': <Map<String, dynamic>>[
            _entry('m2', 'Nocturnal Animals'),
            _entry('m3', 'Doubt'),
            _entry('m4', 'The Master'),
          ],
          'p2': <Map<String, dynamic>>[
            _entry('m2', 'Nocturnal Animals'),
            _entry('m3', 'Doubt'),
            _entry('m4', 'The Master'),
            _entry('m5', 'The Hurt Locker'),
            _entry('m6', 'Wind River'),
            _entry('m7', 'The Town'),
          ],
        },
      ),
    );

    expect(find.text(Strings.moreWithActor('Jeremy Renner')), findsOneWidget);
    expect(
      find.text('Nocturnal Animals'),
      findsOneWidget,
      reason: 'the shared title stays in the first rail that showed it',
    );
    expect(find.text('Wind River'), findsOneWidget);
  });

  testWidgets('a rail left too thin by the dedup is skipped', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        credits: <Map<String, dynamic>>[
          _actor('p1', 'Amy Adams', 0),
          _actor('p2', 'Jeremy Renner', 1),
        ],
        filmographies: <String, List<Map<String, dynamic>>>{
          'p1': <Map<String, dynamic>>[
            _entry('m2', 'Nocturnal Animals'),
            _entry('m3', 'Doubt'),
            _entry('m4', 'The Master'),
          ],
          'p2': <Map<String, dynamic>>[
            _entry('m2', 'Nocturnal Animals'),
            _entry('m3', 'Doubt'),
            _entry('m4', 'The Master'),
            _entry('m5', 'The Hurt Locker'),
          ],
        },
      ),
    );

    expect(find.text(Strings.moreWithActor('Amy Adams')), findsOneWidget);
    expect(find.text(Strings.moreWithActor('Jeremy Renner')), findsNothing);
    expect(
      find.text('The Hurt Locker'),
      findsNothing,
      reason: 'one fresh title is not a rail, so it is dropped with the rail',
    );
  });

  testWidgets('a title credited twice to one actor is shown once', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        credits: <Map<String, dynamic>>[_actor('p1', 'Amy Adams', 0)],
        filmographies: <String, List<Map<String, dynamic>>>{
          'p1': <Map<String, dynamic>>[
            _entry('m2', 'Nocturnal Animals'),
            _entry('m2', 'Nocturnal Animals'),
            _entry('m3', 'Doubt'),
            _entry('m4', 'The Master'),
          ],
        },
      ),
    );

    expect(find.text(Strings.moreWithActor('Amy Adams')), findsOneWidget);
    expect(find.text('Nocturnal Animals'), findsOneWidget);
  });

  testWidgets('the collection rail is never filtered but filters below it', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        credits: <Map<String, dynamic>>[_actor('p1', 'Amy Adams', 0)],
        filmographies: <String, List<Map<String, dynamic>>>{
          'p1': <Map<String, dynamic>>[
            _entry('m2', 'Sicario'),
            _entry('m3', 'Dune'),
            _entry('m4', 'Doubt'),
          ],
        },
        collections: <Map<String, dynamic>>[
          _collection('c1', 'Villeneuve', <Map<String, dynamic>>[
            _member('m1', 'This Very Movie'),
            _member('m2', 'Sicario'),
            _member('m3', 'Dune'),
          ]),
        ],
      ),
    );

    expect(find.text(Strings.moreInCollection('Villeneuve')), findsOneWidget);
    expect(find.text('Sicario'), findsOneWidget);
    expect(find.text('Dune'), findsOneWidget);
    expect(
      find.text(Strings.moreWithActor('Amy Adams')),
      findsNothing,
      reason: 'the collection kept both titles, leaving the actor rail thin',
    );
  });

  testWidgets('the collection rail leaves out the movie being viewed', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        collections: <Map<String, dynamic>>[
          _collection('c1', 'Saga', <Map<String, dynamic>>[
            _member('m1', 'This Very Movie'),
            _member('m2', 'Sicario'),
          ]),
        ],
      ),
    );

    expect(find.text(Strings.moreInCollection('Saga')), findsOneWidget);
    expect(find.text('Sicario'), findsOneWidget);
    expect(
      find.text('This Very Movie'),
      findsNothing,
      reason: 'the movie being viewed is dropped from its own collection rail',
    );
  });

  testWidgets('a movie in two collections gets a rail for each', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[_version('v1', 'fhd')],
        collections: <Map<String, dynamic>>[
          _collection('c1', 'Arrivals', <Map<String, dynamic>>[
            _member('m1', 'This Very Movie'),
            _member('m2', 'Sicario'),
          ]),
          _collection('c2', 'Villeneuve', <Map<String, dynamic>>[
            _member('m1', 'This Very Movie'),
            _member('m3', 'Dune'),
          ]),
        ],
      ),
    );

    expect(find.text(Strings.moreInCollection('Arrivals')), findsOneWidget);
    expect(find.text(Strings.moreInCollection('Villeneuve')), findsOneWidget);
    expect(find.text('Sicario'), findsOneWidget);
    expect(find.text('Dune'), findsOneWidget);
  });

  testWidgets('no collection means no rail', (WidgetTester tester) async {
    await _pump(
      tester,
      _api(versions: <Map<String, dynamic>>[_version('v1', 'fhd')]),
    );

    expect(find.textContaining('More in'), findsNothing);
  });

  testWidgets('a title with nothing to play says so', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(versions: <Map<String, dynamic>>[]));

    // The poster goes inert and Play greys out; without this the page gives
    // no reason for either.
    expect(find.text(Strings.noVersionsAvailable), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(find.widgetWithText(FilledButton, Strings.play))
          .onPressed,
      isNull,
    );
  });

  testWidgets('a title whose files are all missing says so too', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        versions: <Map<String, dynamic>>[
          <String, dynamic>{..._version('v1', 'fhd'), 'available': false},
        ],
      ),
    );

    expect(find.text(Strings.noVersionsAvailable), findsOneWidget);
  });

  testWidgets('a playable title says nothing of the sort', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(versions: <Map<String, dynamic>>[_version('v1', 'fhd')]),
    );

    expect(find.text(Strings.noVersionsAvailable), findsNothing);
  });
}
