import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/jobs_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _job(
  String id,
  String kind,
  String status, {
  String? parent,
  String created = '2026-08-17T07:00:00Z',
}) => <String, dynamic>{
  'id': id,
  'kind': kind,
  'status': status,
  'priority': 'normal',
  'progress': 0.5,
  'attempts': 1,
  'cancellable': status == 'queued' || status == 'running',
  'parent_id': ?parent,
  'created_at': created,
  'updated_at': created,
};

bool _isActive(Map<String, dynamic> job) =>
    job['status'] == 'queued' || job['status'] == 'running';

bool _hit(Map<String, dynamic> job, String needle) =>
    '${job['id']} ${job['kind']} ${job['status']}'
        .replaceAll('_', ' ')
        .toLowerCase()
        .contains(needle.replaceAll('_', ' ').toLowerCase());

class _Server {
  _Server(this.jobs);

  final List<Map<String, dynamic>> jobs;
  final List<Uri> seen = <Uri>[];

  http.Response route(http.Request req) {
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
    if (req.url.path != '/api/v1/admin/jobs') {
      return http.Response('{}', 200);
    }
    seen.add(req.url);
    final String needle = req.url.queryParameters['filter'] ?? '';
    final bool activeOnly = req.url.queryParameters['state'] == 'active';
    final int offset =
        int.tryParse(req.url.queryParameters['offset'] ?? '') ?? 0;
    final int limit = int.tryParse(req.url.queryParameters['limit'] ?? '') ?? 2;
    final List<Map<String, dynamic>> filtered = jobs
        .where((Map<String, dynamic> j) => needle.isEmpty || _hit(j, needle))
        .toList();
    final List<Map<String, dynamic>> scoped = filtered
        .where((Map<String, dynamic> j) => !activeOnly || _isActive(j))
        .toList();
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': scoped.skip(offset).take(limit).toList(),
        'total': scoped.length,
        'offset': offset,
        'limit': limit,
        'active_total': filtered.where(_isActive).length,
        'all_total': filtered.length,
      }),
      200,
    );
  }
}

Future<_Server> _pump(
  WidgetTester tester,
  List<Map<String, dynamic>> jobs,
) async {
  tester.view.physicalSize = const Size(1800, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final _Server server = _Server(jobs);
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
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? JobsPage(api: api)
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return server;
}

Future<void> _type(WidgetTester tester, String text) async {
  await tester.enterText(find.byType(AdminFilterField), text);
  await tester.pump(kFilterDebounce + const Duration(milliseconds: 50));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('Active is the default and asks the server for active only', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester, <Map<String, dynamic>>[
      _job('parent', 'library_scan', 'running'),
      _job('kid', 'metadata', 'succeeded', parent: 'parent'),
    ]);

    expect(server.seen.single.queryParameters['state'], 'active');
    expect(find.text(Strings.jobsActiveCount(1)), findsOneWidget);
    expect(find.text(Strings.jobsAllCount(2)), findsOneWidget);
    expect(find.text('Library scan'), findsOneWidget);
    expect(find.text('Metadata'), findsNothing);
  });

  testWidgets('cancelling names the job kind and says what is kept', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Map<String, dynamic>>[
      _job('run', 'library_scan', 'running'),
    ]);

    await tester.tap(find.byIcon(Icons.cancel_outlined));
    await tester.pumpAndSettle();

    expect(find.text(Strings.cancelJob), findsWidgets);
    expect(find.text(Strings.confirmCancelJob('Library scan')), findsOneWidget);
  });

  testWidgets('the tab counts come from the server, not the loaded page', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Map<String, dynamic>>[
      for (int i = 0; i < 5; i++) _job('run$i', 'artwork', 'running'),
      _job('done', 'dedup', 'succeeded'),
    ]);

    expect(find.text(Strings.jobsActiveCount(5)), findsOneWidget);
    expect(find.text(Strings.jobsAllCount(6)), findsOneWidget);
  });

  testWidgets('switching to All re-asks without the active predicate', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester, <Map<String, dynamic>>[
      _job('parent', 'library_scan', 'running'),
      _job('kid', 'metadata', 'succeeded', parent: 'parent'),
    ]);

    await tester.tap(find.text(Strings.jobsAllCount(2)));
    await tester.pumpAndSettle();

    expect(server.seen.last.queryParameters.containsKey('state'), isFalse);
    expect(find.text('Metadata'), findsOneWidget);
  });

  testWidgets('the filter is sent to the server once typing settles', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester, <Map<String, dynamic>>[
      _job('parent', 'library_scan', 'queued'),
      _job('kid', 'artwork', 'queued', parent: 'parent'),
    ]);

    await _type(tester, 'artwork');

    expect(server.seen.last.queryParameters['filter'], 'artwork');
    expect(find.text('Artwork'), findsOneWidget);
    expect(find.text('Library scan'), findsNothing);
  });

  testWidgets('the filter keeps focus while the reload it triggered runs', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Map<String, dynamic>>[
      _job('parent', 'library_scan', 'queued'),
      _job('kid', 'artwork', 'queued', parent: 'parent'),
    ]);

    await _type(tester, 'artwork');

    final EditableText field = tester.widget<EditableText>(
      find.descendant(
        of: find.byType(AdminFilterField),
        matching: find.byType(EditableText),
      ),
    );
    expect(field.focusNode.hasFocus, isTrue);
    expect(field.controller.text, 'artwork');
  });

  testWidgets('a filter with no matches reports it', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Map<String, dynamic>>[
      _job('parent', 'library_scan', 'queued'),
    ]);

    await _type(tester, 'nothing here');

    expect(find.text(Strings.noMatchingJobs), findsOneWidget);
  });

  testWidgets('paging keeps the filter and asks for the next offset', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester, <Map<String, dynamic>>[
      for (int i = 0; i < 4; i++) _job('art$i', 'artwork', 'queued'),
    ]);

    await _type(tester, 'artwork');
    await tester.tap(find.text(Strings.next));
    await tester.pumpAndSettle();

    expect(server.seen.last.queryParameters['offset'], '2');
    expect(server.seen.last.queryParameters['filter'], 'artwork');
    expect(find.byType(AdminFilterField), findsOneWidget);
  });

  testWidgets('a child row links to its parent job', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Map<String, dynamic>>[
      _job('kid', 'metadata', 'queued', parent: 'parent'),
    ]);

    await tester.tap(find.text('parent'));
    await tester.pumpAndSettle();

    expect(find.text('went to /admin/job?id=parent'), findsOneWidget);
  });
}
