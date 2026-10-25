import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/log_line.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/pages/admin/server_logs_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

const String _kLogs = '/api/v1/admin/server/logs';

Map<String, dynamic> _page(
  List<String> entries, {
  String? before,
  String? after,
  bool more = false,
}) => <String, dynamic>{
  'entries': entries,
  'before': before,
  'after': after,
  'more': more,
};

List<String> _named(String prefix, int count) => <String>[
  for (int n = 0; n < count; n++) '2026-10-24T09:00:00Z  INFO x: $prefix $n',
];

class _Logs {
  _Logs({
    this.role = 'admin',
    List<Object>? newest,
    List<Object>? after,
    List<Object>? before,
  }) : newest = newest ?? <Object>[_page(_named('new', 3), after: 'h:3')],
       after = after ?? <Object>[_page(<String>[], after: 'h:3')],
       before = before ?? <Object>[_page(<String>[])];

  final String role;
  final List<Object> newest;
  final List<Object> after;
  final List<Object> before;
  final List<http.Request> requests = <http.Request>[];
  Future<void>? beforeGate;

  List<Map<String, String>> get reads => requests
      .where((http.Request r) => r.url.path == _kLogs)
      .map((http.Request r) => r.url.queryParameters)
      .toList();

  List<Map<String, String>> readsWith(String key) =>
      reads.where((Map<String, String> q) => q.containsKey(key)).toList();

  Object _next(List<Object> queue) =>
      queue.length > 1 ? queue.removeAt(0) : queue.first;

  http.Response handle(http.Request req) {
    requests.add(req);
    if (req.url.path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': role,
        }),
        200,
      );
    }
    if (req.url.path == _kLogs) {
      final Map<String, String> q = req.url.queryParameters;
      final Object answer = q.containsKey('after')
          ? _next(after)
          : q.containsKey('before')
          ? _next(before)
          : _next(newest);
      return answer is int
          ? http.Response('{}', answer)
          : http.Response(jsonEncode(answer), 200);
    }
    return http.Response('{}', 200);
  }
}

Future<_Logs> _pump(WidgetTester tester, _Logs logs) async {
  tester.view.physicalSize = const Size(1600, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      final http.Response answer = logs.handle(req);
      final Future<void>? gate = logs.beforeGate;
      if (gate != null && req.url.queryParameters.containsKey('before')) {
        await gate;
      }
      return answer;
    }),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        navigatorObservers: <NavigatorObserver>[appRouteObserver],
        home: ServerLogsPage(api: api),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return logs;
}

Future<void> _tick(WidgetTester tester) async {
  await tester.pump(kServerLogPoll);
  await tester.pumpAndSettle();
}

Future<void> _close(WidgetTester tester) =>
    tester.pumpWidget(const SizedBox.shrink());

Finder _logList() => find.descendant(
  of: find.byType(SelectionArea),
  matching: find.byType(CustomScrollView),
);

List<String> _shown(WidgetTester tester) => tester
    .widgetList<LogLine>(find.byType(LogLine))
    .map((LogLine line) => line.entry.split(': ').last)
    .toList();

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('it opens on the newest entries at info and above', (
    WidgetTester tester,
  ) async {
    final _Logs logs = await _pump(tester, _Logs());

    expect(_shown(tester), <String>['new 0', 'new 1', 'new 2']);
    expect(logs.reads, <Map<String, String>>[
      <String, String>{'limit': '$kServerLogPage', 'level': 'INFO'},
    ]);
    await _close(tester);
  });

  testWidgets('every 2 s it asks for what came after and appends it', (
    WidgetTester tester,
  ) async {
    final _Logs logs = await _pump(
      tester,
      _Logs(
        after: <Object>[
          _page(_named('more', 1), after: 'h:4'),
          _page(<String>[], after: 'h:4'),
        ],
      ),
    );

    await _tick(tester);
    expect(_shown(tester), <String>['new 0', 'new 1', 'new 2', 'more 0']);

    await _tick(tester);
    expect(
      logs.readsWith('after').map((Map<String, String> q) => q['after']),
      <String>['h:3', 'h:4'],
    );
    await _close(tester);
  });

  testWidgets('a read that stopped at the limit is followed at once', (
    WidgetTester tester,
  ) async {
    final _Logs logs = await _pump(
      tester,
      _Logs(
        after: <Object>[
          _page(_named('a', 1), after: 'h:4', more: true),
          _page(_named('b', 1), after: 'h:5'),
          _page(<String>[], after: 'h:5'),
        ],
      ),
    );

    await _tick(tester);

    expect(logs.readsWith('after').length, 2);
    expect(_shown(tester).sublist(3), <String>['a 0', 'b 0']);
    await _close(tester);
  });

  testWidgets('pause stops the polling and resume starts it again', (
    WidgetTester tester,
  ) async {
    final _Logs logs = await _pump(tester, _Logs());

    await tester.tap(find.byIcon(Icons.pause));
    await tester.pump();
    await _tick(tester);
    await _tick(tester);
    expect(logs.readsWith('after'), isEmpty);

    await tester.tap(find.byIcon(Icons.play_arrow));
    await tester.pump();
    await _tick(tester);
    expect(logs.readsWith('after').length, 1);
    await _close(tester);
  });

  testWidgets('a page opened over it stops the polling until it is back on '
      'top', (WidgetTester tester) async {
    final _Logs logs = await _pump(tester, _Logs());
    final NavigatorState nav = tester.state<NavigatorState>(
      find.byType(Navigator),
    );

    nav.push(
      MaterialPageRoute<void>(
        builder: (_) => const Scaffold(body: Text('another page')),
      ),
    );
    await tester.pumpAndSettle();
    await _tick(tester);
    await _tick(tester);
    expect(logs.readsWith('after'), isEmpty);

    nav.pop();
    await tester.pumpAndSettle();
    expect(logs.readsWith('after').length, 1, reason: 'it catches up at once');

    await _tick(tester);
    expect(logs.readsWith('after').length, 2);
    await _close(tester);
  });

  testWidgets('scrolling to the top loads the entries before, keeping the '
      'place', (WidgetTester tester) async {
    final _Logs logs = await _pump(
      tester,
      _Logs(
        newest: <Object>[
          _page(_named('new', 200), before: 'h:100', after: 'h:300'),
        ],
        before: <Object>[_page(_named('old', 200))],
      ),
    );
    expect(logs.readsWith('before'), isEmpty, reason: 'nothing asked yet');

    await tester.drag(_logList(), const Offset(0, 6000));
    await tester.pump();
    final Offset place = tester.getTopLeft(find.textContaining('new 0'));
    await tester.pumpAndSettle();

    expect(logs.readsWith('before'), <Map<String, String>>[
      <String, String>{
        'limit': '$kServerLogPage',
        'level': 'INFO',
        'before': 'h:100',
      },
    ]);
    expect(tester.getTopLeft(find.textContaining('new 0')), place);

    await tester.drag(_logList(), const Offset(0, 6000));
    await tester.pumpAndSettle();
    expect(find.textContaining('old 0'), findsOneWidget);
    expect(find.text(Strings.startOfLogs), findsOneWidget);
    expect(logs.readsWith('before').length, 1, reason: 'nothing more is kept');
    await _close(tester);
  });

  testWidgets('earlier entries that cannot be read can be asked again', (
    WidgetTester tester,
  ) async {
    final _Logs logs = await _pump(
      tester,
      _Logs(
        newest: <Object>[
          _page(_named('new', 200), before: 'h:100', after: 'h:300'),
        ],
        before: <Object>[500, _page(_named('old', 3))],
      ),
    );

    await tester.drag(_logList(), const Offset(0, 6000));
    await tester.pumpAndSettle();
    expect(logs.readsWith('before').length, 1);
    await tester.drag(_logList(), const Offset(0, 300));
    await tester.pumpAndSettle();
    expect(logs.readsWith('before').length, 1, reason: 'a failure waits');
    await tester.tap(find.text(Strings.couldNotLoadOlderLogs));
    await tester.pumpAndSettle();

    expect(logs.readsWith('before').length, 2);
    expect(find.textContaining('old 2'), findsOneWidget);
    await _close(tester);
  });

  testWidgets('a level change reads the newest again at that level', (
    WidgetTester tester,
  ) async {
    final _Logs logs = await _pump(
      tester,
      _Logs(
        newest: <Object>[
          _page(_named('new', 3), after: 'h:3'),
          _page(<String>['2026-10-24T09:00:00Z ERROR x: boom'], after: 'h:9'),
        ],
      ),
    );

    await tester.tap(find.text('Info'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Error').last);
    await tester.pumpAndSettle();

    expect(logs.reads.last, <String, String>{
      'limit': '$kServerLogPage',
      'level': 'ERROR',
    });
    expect(_shown(tester), <String>['boom']);
    await _tick(tester);
    expect(logs.readsWith('after').single['after'], 'h:9');
    await _close(tester);
  });

  testWidgets('a level change while earlier entries load lets them be read '
      'again', (WidgetTester tester) async {
    final Completer<void> held = Completer<void>();
    final _Logs logs = _Logs(
      newest: <Object>[
        _page(_named('new', 200), before: 'h:100', after: 'h:300'),
        _page(_named('err', 200), before: 'h:50', after: 'h:300'),
      ],
      before: <Object>[_page(_named('stale', 3)), _page(_named('old', 3))],
    )..beforeGate = held.future;
    await _pump(tester, logs);

    await tester.drag(_logList(), const Offset(0, 6000));
    await tester.pumpAndSettle();
    expect(logs.readsWith('before').length, 1);
    await tester.tap(find.text('Info'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Error').last);
    await tester.pumpAndSettle();
    logs.beforeGate = null;
    held.complete();
    await tester.pumpAndSettle();

    await tester.drag(_logList(), const Offset(0, 6000));
    await tester.pumpAndSettle();
    expect(logs.readsWith('before').last['before'], 'h:50');
    await tester.drag(_logList(), const Offset(0, 6000));
    await tester.pumpAndSettle();

    expect(find.textContaining('old 0'), findsOneWidget);
    expect(find.textContaining('stale 0'), findsNothing);
    expect(find.text(Strings.loadingOlderLogs), findsNothing);
    await _close(tester);
  });

  testWidgets('a poll that fails keeps the entries and says it will try '
      'again', (WidgetTester tester) async {
    await _pump(
      tester,
      _Logs(
        after: <Object>[
          500,
          _page(_named('more', 1), after: 'h:4'),
        ],
      ),
    );

    await _tick(tester);
    expect(find.text(Strings.couldNotRefreshLogs), findsOneWidget);
    expect(_shown(tester).length, 3);

    await _tick(tester);
    expect(find.text(Strings.couldNotRefreshLogs), findsNothing);
    expect(_shown(tester).last, 'more 0');
    await _close(tester);
  });

  testWidgets('a server that has logged nothing yet fills in once it does', (
    WidgetTester tester,
  ) async {
    final _Logs logs = await _pump(
      tester,
      _Logs(
        newest: <Object>[
          _page(<String>[]),
          _page(_named('first', 1), after: 'h:1'),
        ],
      ),
    );
    expect(find.text(Strings.emptyLogs), findsOneWidget);

    await _tick(tester);

    expect(_shown(tester), <String>['first 0']);
    expect(logs.readsWith('after'), isEmpty);
    await _close(tester);
  });

  testWidgets('a first read that fails says so', (WidgetTester tester) async {
    await _pump(tester, _Logs(newest: <Object>[500]));

    expect(find.text(Strings.couldNotLoadLogs), findsOneWidget);
    await _close(tester);
  });

  testWidgets('a non-admin is not authorized', (WidgetTester tester) async {
    final _Logs logs = await _pump(tester, _Logs(role: 'user'));

    expect(find.text(Strings.notAuthorized), findsOneWidget);
    expect(logs.reads, isEmpty);
    await _close(tester);
  });
}
