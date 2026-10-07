import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/job_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

class _Server {
  _Server(this.status, {this.refuse = false});

  String status;
  final bool refuse;
  final List<String> retried = <String>[];
  int reads = 0;

  Map<String, dynamic> get job => <String, dynamic>{
    'id': 'j1',
    'kind': 'subtitles',
    'status': status,
    'priority': 'low',
    'created_at': '2026-10-07T07:00:00Z',
    'updated_at': '2026-10-07T07:00:00Z',
    'cancellable': status == 'queued' || status == 'running',
    'retryable': status == 'failed' || status == 'cancelled',
  };

  http.Response route(http.Request req) {
    final String path = req.url.path;
    if (path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'admin',
        }),
        200,
      );
    }
    if (path == '/api/v1/admin/jobs/j1/retry' && req.method == 'POST') {
      retried.add('j1');
      if (refuse) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'error': <String, dynamic>{
              'code': 'not_retryable',
              'message': 'job is not retryable',
            },
          }),
          409,
        );
      }
      status = 'queued';
      return http.Response(jsonEncode(job), 200);
    }
    if (path == '/api/v1/admin/jobs/j1') {
      reads++;
      return http.Response(jsonEncode(job), 200);
    }
    if (path == '/api/v1/admin/jobs/j1/logs') {
      return http.Response(
        jsonEncode(<String, dynamic>{'lines': <String>[]}),
        200,
      );
    }
    if (path == '/api/v1/admin/jobs/j1/children') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': <dynamic>[],
          'total': 0,
          'offset': 0,
          'limit': 50,
        }),
        200,
      );
    }
    return http.Response('{}', 200);
  }
}

Future<_Server> _pump(
  WidgetTester tester,
  String status, {
  bool refuse = false,
}) async {
  tester.view.physicalSize = const Size(1400, 1000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final _Server server = _Server(status, refuse: refuse);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async => server.route(req)),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: JobPage(api: api, id: 'j1'),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return server;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a failed job offers a retry instead of a cancel', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 'failed');

    expect(find.text(Strings.retryJob), findsOneWidget);
    expect(find.text(Strings.cancelJob), findsNothing);
  });

  testWidgets('a running job offers a cancel and no retry', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 'running');

    expect(find.text(Strings.cancelJob), findsOneWidget);
    expect(find.text(Strings.retryJob), findsNothing);
  });

  testWidgets('retrying queues the job again and reloads it', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester, 'failed');
    final int reads = server.reads;

    await tester.tap(find.text(Strings.retryJob));
    await tester.pumpAndSettle();

    expect(server.retried, <String>['j1']);
    expect(server.reads, reads + 1);
    expect(find.text(Strings.toastJobRetried), findsOneWidget);
    expect(find.text(Strings.retryJob), findsNothing);
    expect(find.text(Strings.cancelJob), findsOneWidget);
  });

  testWidgets('a refused retry says why and keeps the action', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 'failed', refuse: true);

    await tester.tap(find.text(Strings.retryJob));
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.reasonNotRetryable), findsOneWidget);
    expect(find.text(Strings.retryJob), findsOneWidget);
  });
}
