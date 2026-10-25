import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/diagnostics/report_summary.dart';
import 'package:shadowmask/pages/admin/report_page.dart';
import 'package:shadowmask/pages/admin/reports_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shared_preferences/shared_preferences.dart';

const String _kReports = '/api/v1/admin/diagnostics/reports';
const String _kSent = '2026-10-24T09:00:05.123Z';

const Map<String, dynamic> _payload = <String, dynamic>{
  'schema': 1,
  'history': <dynamic>[
    <String, dynamic>{'at': 't1', 'event': 'opened'},
  ],
  'log': <String>['09:00:01 waiting'],
};

Map<String, dynamic> _summary(String id, {String? note}) => <String, dynamic>{
  'id': id,
  'created_at': _kSent,
  'user_id': 'u7',
  'player_id': 'player-$id',
  'app': 'flutter',
  'platform': 'android 14',
  'app_version': '0.0.7',
  'note': note,
};

class _Server {
  _Server({this.role = 'admin', this.failDelete = false});

  final String role;
  final bool failDelete;
  final List<http.Request> requests = <http.Request>[];

  http.Response handle(http.Request req) {
    requests.add(req);
    final String path = req.url.path;
    if (path == '/api/v1/users/self') {
      return _json(<String, dynamic>{
        'id': 'u1',
        'username': 'pat',
        'role': role,
      });
    }
    if (path == '/api/v1/users') {
      return _json(<String, dynamic>{
        'items': <dynamic>[
          <String, dynamic>{
            'id': 'u7',
            'username': 'sam',
            'role': 'user',
            'created_at': '2026-08-01T00:00:00Z',
            'updated_at': '2026-08-01T00:00:00Z',
          },
        ],
        'total': 1,
        'offset': 0,
        'limit': 200,
      });
    }
    if (path == _kReports) {
      final String filter = req.url.queryParameters['filter'] ?? '';
      final bool kept = 'player-r1 u7 flutter android 14 0.0.7'.contains(
        filter.toLowerCase(),
      );
      return _json(<String, dynamic>{
        'items': <dynamic>[
          if (kept) _summary('r1', note: 'stutters after a seek'),
        ],
        'total': kept ? 1 : 0,
        'offset': 0,
        'limit': 50,
      });
    }
    if (path == '$_kReports/r1' && req.method == 'DELETE') {
      return http.Response('', failDelete ? 500 : 204);
    }
    if (path == '$_kReports/r1') {
      return _json(<String, dynamic>{
        ..._summary('r1', note: 'stutters after a seek'),
        'payload': _payload,
      });
    }
    return http.Response('{}', 200);
  }

  List<http.Request> named(String method, String path) => requests
      .where((http.Request r) => r.method == method && r.url.path == path)
      .toList();
}

http.Response _json(Map<String, dynamic> body) =>
    http.Response(jsonEncode(body), 200);

Future<_Server> _pump(
  WidgetTester tester,
  _Server server,
  Widget Function(ApiClient api) page,
) async {
  tester.view.physicalSize = const Size(1600, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async => server.handle(req)),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? page(api)
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return server;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the list names the user and opens a report', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _Server(), (ApiClient api) => ReportsPage(api: api));

    final AdminTable<ReportSummary> table = tester
        .widget<AdminTable<ReportSummary>>(
          find.byType(AdminTable<ReportSummary>),
        );
    expect(
      table.columns.map((AdminColumn<ReportSummary> c) => c.label),
      <String>[
        Strings.reportSent,
        Strings.reportPlayer,
        Strings.reportUser,
        Strings.reportApp,
        Strings.reportAppVersion,
        Strings.reportPlatform,
      ],
    );
    expect(find.text('sam'), findsOneWidget);
    expect(find.text('u7'), findsNothing);
    expect(find.text('android 14'), findsOneWidget);
    expect(find.text('stutters after a seek'), findsNothing, reason: 'no note');
    expect(
      find.text(preciseDateTimeText(_kSent)!),
      findsOneWidget,
      reason: 'reports a minute apart are told apart',
    );

    await tester.tap(find.text('android 14'));
    await tester.pumpAndSettle();

    expect(find.text('went to /admin/report?id=r1'), findsOneWidget);
  });

  testWidgets('the filter asks the server again from the first page', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(
      tester,
      _Server(),
      (ApiClient api) => ReportsPage(api: api),
    );

    await tester.enterText(find.byType(TextField), 'roku');
    await tester.pump(kFilterDebounce);
    await tester.pumpAndSettle();

    final http.Request asked = server.named('GET', _kReports).last;
    expect(asked.url.queryParameters['filter'], 'roku');
    expect(asked.url.queryParameters.containsKey('offset'), isFalse);
    expect(find.text(Strings.noMatchingReports), findsOneWidget);

    await tester.enterText(find.byType(TextField), 'ANDROID');
    await tester.pump(kFilterDebounce);
    await tester.pumpAndSettle();

    expect(find.text('android 14'), findsOneWidget);
  });

  testWidgets('a report is one field: the whole of it as formatted JSON', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(
      tester,
      _Server(),
      (ApiClient api) => ReportPage(api: api, id: 'r1'),
    );

    final String shown = tester
        .widget<Text>(find.textContaining('"player_id"'))
        .data!;
    expect(jsonDecode(shown), <String, dynamic>{
      ..._summary('r1', note: 'stutters after a seek'),
      'payload': _payload,
    });
    expect(shown, startsWith('{\n  "id": "r1",'), reason: 'indented');
    expect(find.text(Strings.reportHistory), findsNothing, reason: 'not split');
    expect(server.named('GET', '/api/v1/users'), isEmpty);
    expect(find.text(preciseDateTimeText(_kSent)!), findsOneWidget);
  });

  testWidgets('copy puts the whole report on the clipboard', (
    WidgetTester tester,
  ) async {
    final List<String> copied = <String>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (MethodCall call) async {
        if (call.method == 'Clipboard.setData') {
          copied.add(
            (call.arguments as Map<Object?, Object?>)['text']! as String,
          );
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    await _pump(
      tester,
      _Server(),
      (ApiClient api) => ReportPage(api: api, id: 'r1'),
    );

    await tester.tap(find.text(Strings.copyReport));
    await tester.pumpAndSettle();

    final Map<String, dynamic> json =
        jsonDecode(copied.single) as Map<String, dynamic>;
    expect(json['id'], 'r1');
    expect(json['player_id'], 'player-r1');
    expect((json['payload'] as Map<String, dynamic>)['log'], <String>[
      '09:00:01 waiting',
    ]);
    expect(find.text(Strings.toastReportCopied), findsOneWidget);
  });

  testWidgets('delete asks first, then goes back to the list', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(
      tester,
      _Server(),
      (ApiClient api) => ReportPage(api: api, id: 'r1'),
    );

    await tester.tap(find.text(Strings.delete));
    await tester.pumpAndSettle();
    expect(find.text(Strings.deleteReportTitle), findsOneWidget);
    expect(server.named('DELETE', '$_kReports/r1'), isEmpty);

    await tester.tap(find.text(Strings.delete).last);
    await tester.pumpAndSettle();

    expect(server.named('DELETE', '$_kReports/r1'), hasLength(1));
    expect(find.text('went to /admin/reports'), findsOneWidget);
  });

  testWidgets('a delete that fails stays on the report', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _Server(failDelete: true),
      (ApiClient api) => ReportPage(api: api, id: 'r1'),
    );

    await tester.tap(find.text(Strings.delete));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.delete).last);
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.errorDeleteReport), findsOneWidget);
    expect(find.textContaining('"player_id": "player-r1"'), findsOneWidget);
  });

  testWidgets('a non-admin is not authorized and nothing is read', (
    WidgetTester tester,
  ) async {
    final _Server list = await _pump(
      tester,
      _Server(role: 'user'),
      (ApiClient api) => ReportsPage(api: api),
    );
    expect(find.text(Strings.notAuthorized), findsOneWidget);

    final _Server one = await _pump(
      tester,
      _Server(role: 'user'),
      (ApiClient api) => ReportPage(api: api, id: 'r1'),
    );
    expect(find.text(Strings.notAuthorized), findsOneWidget);

    for (final _Server server in <_Server>[list, one]) {
      expect(
        server.requests.where(
          (http.Request r) => r.url.path.startsWith(_kReports),
        ),
        isEmpty,
      );
    }
  });
}
