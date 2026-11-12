import 'dart:convert';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/play_button.dart';
import 'package:shadowmask/components/play_on_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/auth/play_on_device.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../support/play_on.dart';

Version _version(String id, {bool available = true}) => Version(
  id: id,
  title: const TitleRef(type: TitleKind.movie, id: 'm1'),
  libraryId: 'l1',
  quality: Quality.fhd,
  container: 'mkv',
  durationMs: 600000,
  available: available,
);

Map<String, dynamic> _resumeEntry(String versionId) => <String, dynamic>{
  'card': <String, dynamic>{
    'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
    'display_title': 'Big Buck Bunny',
    'duration_ms': 600000,
    'progress_percent': 40,
  },
  'progress': <String, dynamic>{'version_id': versionId},
};

Finder _resume() => find.widgetWithText(FilledButton, Strings.resumeAction);

Finder _dismiss() => find.widgetWithIcon(FilledButton, Icons.close);

Finder _versions() => find.byIcon(Icons.playlist_play);

BorderRadius _corners(ButtonStyle? style) =>
    (style!.shape!.resolve(<WidgetState>{})! as RoundedRectangleBorder)
            .borderRadius
        as BorderRadius;

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
  List<Version> versions, {
  List<String> inProgress = const <String>[],
  bool dismissible = false,
  List<String>? cleared,
  PlayOnServer? server,
  bool Function()? feedDown,
}) async {
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      final http.Response? playOn = server?.answer(req);
      if (playOn != null) {
        return playOn;
      }
      if (req.method == 'DELETE' && req.url.path.contains('/progress/')) {
        cleared?.add(req.url.pathSegments.last);
        return http.Response('', 204);
      }
      if (req.url.path.endsWith('/continue')) {
        if (feedDown?.call() ?? false) {
          return http.Response('', 500);
        }
        return http.Response(
          jsonEncode(<String, dynamic>{
            'now_playing': <dynamic>[],
            'in_progress': inProgress.map(_resumeEntry).toList(),
            'next_episodes': <dynamic>[],
            'next_movies': <dynamic>[],
          }),
          200,
          headers: <String, String>{'content-type': 'application/json'},
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
      onGenerateRoute: (RouteSettings s) => MaterialPageRoute<void>(
        settings: s,
        builder: (BuildContext _) => Scaffold(
          body: s.name == '/'
              ? PlayButton(
                  catalog: CatalogApi(api),
                  userId: 'u1',
                  versions: versions,
                  playback: dismissible ? PlaybackApi(api) : null,
                  playOn: server?.api(api),
                )
              : const SizedBox.shrink(),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  pushes.names.clear();
  return pushes;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('one available version is the target', () {
    expect(
      resolvePlayTarget(<Version>[_version('v1')], const <String>{})?.id,
      'v1',
    );
  });

  test('no available versions has no target', () {
    expect(resolvePlayTarget(<Version>[], const <String>{}), isNull);
  });

  test('the version in progress is the target over the top one', () {
    final List<Version> vs = <Version>[
      _version('v1'),
      _version('v2'),
      _version('v3'),
    ];
    expect(resolvePlayTarget(vs, <String>{'v2'})?.id, 'v2');
  });

  test('with nothing in progress the top version is the target', () {
    final List<Version> vs = <Version>[_version('v1'), _version('v2')];
    expect(resolvePlayTarget(vs, const <String>{})?.id, 'v1');
  });

  test('of several in progress the top one is the target', () {
    final List<Version> vs = <Version>[
      _version('v1'),
      _version('v2'),
      _version('v3'),
    ];
    expect(resolvePlayTarget(vs, <String>{'v3', 'v2'})?.id, 'v2');
  });

  testWidgets('a single version plays without a prompt', (
    WidgetTester tester,
  ) async {
    final _Pushes pushes = await _pump(tester, <Version>[_version('v1')]);

    await tester.tap(find.text(Strings.play));
    await tester.pumpAndSettle();

    expect(pushes.names.last, '/watch?version=v1');
  });

  testWidgets('no available version disables the button', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Version>[_version('v1', available: false)]);

    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNull,
    );
  });

  testWidgets('a single in-progress version reads Resume', (
    WidgetTester tester,
  ) async {
    final _Pushes pushes = await _pump(
      tester,
      <Version>[_version('v1')],
      inProgress: <String>['v1'],
    );

    expect(find.text(Strings.resumeAction), findsOneWidget);
    expect(find.text(Strings.play), findsNothing);
    expect(
      find.byTooltip('Resume at 40%'),
      findsOneWidget,
      reason: 'the percent the continue feed reported lives in the tooltip',
    );

    await tester.tap(_resume());
    await tester.pumpAndSettle();

    expect(pushes.names.last, '/watch?version=v1');
  });

  testWidgets('the resume button hugs its label instead of filling the row', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Version>[_version('v1')], inProgress: <String>['v1']);

    final double button = tester.getSize(_resume()).width;
    final double available = tester.getSize(find.byType(Scaffold)).width;

    expect(
      button,
      lessThan(available / 2),
      reason: 'the resume button must not stretch to the full row',
    );
    expect(
      tester.getCenter(find.text(Strings.resumeAction)).dy,
      moreOrLessEquals(tester.getCenter(_resume()).dy, epsilon: 1),
      reason: 'the label sits in the middle of the button, not at its top',
    );
  });

  testWidgets('resume and dismiss meet as one shape', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      <Version>[_version('v1')],
      inProgress: <String>['v1'],
      dismissible: true,
    );

    expect(_dismiss(), findsOneWidget);
    expect(
      tester.getRect(_resume()).right,
      moreOrLessEquals(tester.getRect(_dismiss()).left, epsilon: 0.01),
      reason: 'the two halves share an edge instead of sitting apart',
    );
    expect(
      tester.getRect(_resume()).height,
      moreOrLessEquals(tester.getRect(_dismiss()).height, epsilon: 0.01),
    );

    final BorderRadius left = _corners(
      tester.widget<FilledButton>(_resume()).style,
    );
    final BorderRadius right = _corners(
      tester.widget<FilledButton>(_dismiss()).style,
    );
    expect(
      <Radius>[left.topLeft, left.topRight, right.topLeft, right.topRight],
      <Radius>[Radii.sm, Radius.zero, Radius.zero, Radii.sm],
      reason: 'only the outer corners are rounded, so the pair reads as one',
    );
  });

  testWidgets('dismissing clears the resume point and falls back to Play', (
    WidgetTester tester,
  ) async {
    final List<String> cleared = <String>[];
    await _pump(
      tester,
      <Version>[_version('v1')],
      inProgress: <String>['v1'],
      dismissible: true,
      cleared: cleared,
    );

    expect(
      find.byTooltip(Strings.dismissResume),
      findsOneWidget,
      reason: 'the icon-only button names itself through its tooltip',
    );

    await tester.tap(_dismiss());
    await tester.pumpAndSettle();

    expect(cleared, <String>['v1']);
    expect(_resume(), findsNothing);
    expect(find.text(Strings.play), findsOneWidget);
    expect(_dismiss(), findsNothing);
  });

  testWidgets('without a playback client there is no dismiss half', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Version>[_version('v1')], inProgress: <String>['v1']);

    expect(_resume(), findsOneWidget);
    expect(_dismiss(), findsNothing);
  });

  testWidgets('a single unwatched version reads Play', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Version>[_version('v1')]);

    expect(find.text(Strings.play), findsOneWidget);
    expect(_resume(), findsNothing);
  });

  testWidgets('the one in-progress version resumes without a prompt', (
    WidgetTester tester,
  ) async {
    final _Pushes pushes = await _pump(
      tester,
      <Version>[_version('v1'), _version('v2')],
      inProgress: <String>['v2'],
    );

    expect(_resume(), findsOneWidget);

    await tester.tap(_resume());
    await tester.pumpAndSettle();

    expect(pushes.names.last, '/watch?version=v2');
  });

  testWidgets('several versions with nothing in progress play the top one', (
    WidgetTester tester,
  ) async {
    final _Pushes pushes = await _pump(tester, <Version>[
      _version('v2'),
      _version('v1').copyWith(quality: Quality.uhd),
    ]);

    await tester.tap(find.text(Strings.play));
    await tester.pumpAndSettle();

    expect(pushes.names.last, '/watch?version=v1');
    expect(find.byType(MenuItemButton), findsNothing);
  });

  testWidgets('Play reads the continue feed again at the press', (
    WidgetTester tester,
  ) async {
    final List<String> progress = <String>[];
    final _Pushes pushes = await _pump(tester, <Version>[
      _version('v1'),
      _version('v2'),
    ], inProgress: progress);
    progress.add('v2');

    await tester.tap(find.text(Strings.play));
    await tester.pumpAndSettle();

    expect(
      pushes.names.last,
      '/watch?version=v2',
      reason:
          'starting v1 from what the page loaded would delete the v2 progress',
    );
  });

  testWidgets('a feed that fails at the press keeps what the button knew', (
    WidgetTester tester,
  ) async {
    bool down = false;
    final _Pushes pushes = await _pump(
      tester,
      <Version>[_version('v1'), _version('v2')],
      inProgress: <String>['v2'],
      feedDown: () => down,
    );
    down = true;

    await tester.tap(_resume());
    await tester.pumpAndSettle();

    expect(pushes.names.last, '/watch?version=v2');
  });

  testWidgets('a top version without a file is passed over', (
    WidgetTester tester,
  ) async {
    final _Pushes pushes = await _pump(tester, <Version>[
      _version('v1', available: false),
      _version('v2'),
      _version('v3'),
    ]);

    await tester.tap(find.text(Strings.play));
    await tester.pumpAndSettle();

    expect(pushes.names.last, '/watch?version=v2');
  });

  testWidgets('a single version has no version button', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Version>[
      _version('v1'),
      _version('v2', available: false),
    ]);

    expect(_versions(), findsNothing);
  });

  testWidgets('the version button plays the version picked', (
    WidgetTester tester,
  ) async {
    final _Pushes pushes = await _pump(tester, <Version>[
      _version('v1'),
      _version('v2'),
      _version('v3', available: false),
    ]);

    expect(find.byTooltip(Strings.chooseVersion), findsOneWidget);

    await tester.tap(_versions());
    await tester.pumpAndSettle();

    expect(pushes.names, isEmpty);
    expect(find.byType(MenuItemButton), findsNWidgets(2));

    await tester.tap(find.byType(MenuItemButton).last);
    await tester.pumpAndSettle();

    expect(pushes.names.last, '/watch?version=v2');
  });

  testWidgets('the menu reads duration, quality, container and size', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Version>[
      _version('v1').copyWith(sizeBytes: 42 * 1073741824),
      _version('v2').copyWith(sizeBytes: 700 * 1048576),
      _version('v3').copyWith(durationMs: 0),
    ]);

    await tester.tap(_versions());
    await tester.pumpAndSettle();

    expect(
      find.text('1 · 10m · FHD · mkv · 42.0 GB', findRichText: true),
      findsOne,
    );
    expect(
      find.text('2 · 10m · FHD · mkv · 0.7 GB', findRichText: true),
      findsOne,
    );
    expect(find.text('3 · FHD · mkv', findRichText: true), findsOne);
  });

  group('Play on', () {
    testWidgets('a device that can play puts Play on after Play', (
      WidgetTester tester,
    ) async {
      await _pump(tester, <Version>[_version('v1')], server: PlayOnServer());

      expect(find.byType(PlayOnButton), findsOneWidget);
      expect(
        tester.getRect(find.byType(PlayOnButton)).left,
        greaterThanOrEqualTo(tester.getRect(find.text(Strings.play)).right),
      );
    });

    testWidgets('no device that can play leaves Play alone', (
      WidgetTester tester,
    ) async {
      await _pump(tester, <Version>[
        _version('v1'),
      ], server: PlayOnServer(devices: const []));

      expect(find.byType(PlayOnButton), findsNothing);
    });

    testWidgets('a device shared with the user alone puts Play on', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        <Version>[_version('v1')],
        server: PlayOnServer(
          devices: const [],
          shared: const <PlayOnDevice>[kSharedRoku],
        ),
      );

      await tester.tap(find.byIcon(Icons.devices));
      await tester.pumpAndSettle();

      expect(find.text('Den (admin)'), findsOneWidget);
    });

    testWidgets('a device list that fails leaves Play alone', (
      WidgetTester tester,
    ) async {
      await _pump(tester, <Version>[
        _version('v1'),
      ], server: PlayOnServer(devicesStatus: 500));

      expect(find.byType(PlayOnButton), findsNothing);
    });

    testWidgets('nothing playable offers no Play on', (
      WidgetTester tester,
    ) async {
      await _pump(tester, <Version>[
        _version('v1', available: false),
      ], server: PlayOnServer());

      expect(find.byType(PlayOnButton), findsNothing);
    });

    testWidgets('Play on sends the version Play would resume', (
      WidgetTester tester,
    ) async {
      final PlayOnServer server = PlayOnServer();
      await _pump(
        tester,
        <Version>[_version('v1'), _version('v2')],
        inProgress: <String>['v2'],
        server: server,
      );

      await tester.tap(find.byIcon(Icons.devices));
      await tester.pumpAndSettle();

      expect(find.byType(SubmenuButton), findsNothing);

      await tester.tap(find.text('Pixel'));
      await tester.pumpAndSettle();

      expect(server.mints, <Mint>[(device: 'd2', version: 'v2')]);
    });

    testWidgets('Play on sends the top version when nothing is in progress', (
      WidgetTester tester,
    ) async {
      final PlayOnServer server = PlayOnServer();
      await _pump(tester, <Version>[
        _version('v1'),
        _version('v2'),
      ], server: server);

      await tester.tap(find.byIcon(Icons.devices));
      await tester.pumpAndSettle();

      expect(find.byType(SubmenuButton), findsNothing);

      await tester.tap(find.text('Pixel'));
      await tester.pumpAndSettle();

      expect(server.mints, <Mint>[(device: 'd2', version: 'v1')]);
    });

    testWidgets('Resume, its dismiss and Play on share one row', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        <Version>[_version('v1')],
        inProgress: <String>['v1'],
        dismissible: true,
        server: PlayOnServer(),
      );

      expect(_resume(), findsOneWidget);
      expect(_dismiss(), findsOneWidget);
      expect(
        tester.getRect(find.byType(PlayOnButton)).left,
        greaterThanOrEqualTo(tester.getRect(_dismiss()).right),
      );
    });

    testWidgets('the version button sits between dismiss and Play on', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        <Version>[_version('v1'), _version('v2')],
        inProgress: <String>['v2'],
        dismissible: true,
        server: PlayOnServer(),
      );

      final Rect versions = tester.getRect(_versions());
      expect(
        tester.getRect(_resume()).right,
        moreOrLessEquals(tester.getRect(_dismiss()).left, epsilon: 0.01),
        reason: 'Resume and its dismiss stay one shape',
      );
      expect(
        versions.left,
        greaterThan(tester.getRect(_dismiss()).right),
        reason: 'a gap separates the version button from the dismiss half',
      );
      expect(
        tester.getRect(find.byType(PlayOnButton)).left,
        greaterThanOrEqualTo(versions.right),
      );
    });
  });
}
