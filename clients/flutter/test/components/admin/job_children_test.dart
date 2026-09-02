import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/job_children.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/job_node.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/view/page.dart';

JobNode _node(String id, JobKind kind, int depth) => JobNode(
  job: Job(
    id: id,
    kind: kind,
    status: JobStatus.succeeded,
    priority: JobPriority.normal,
    createdAt: '2026-08-17T07:00:00Z',
    updatedAt: '2026-08-17T07:00:00Z',
  ),
  depth: depth,
);

Future<List<int>> _pump(
  WidgetTester tester,
  Future<Paged<JobNode>> future,
) async {
  final List<int> paged = <int>[];
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? Scaffold(
                  body: SizedBox(
                    width: 1200,
                    child: JobChildren(future: future, onOffset: paged.add),
                  ),
                )
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return paged;
}

Future<List<int>> _pumpNodes(
  WidgetTester tester,
  List<JobNode> nodes, {
  int total = 0,
  int offset = 0,
  int limit = 2,
}) => _pump(
  tester,
  Future<Paged<JobNode>>.value(
    Paged<JobNode>(
      items: nodes,
      total: total == 0 ? nodes.length : total,
      offset: offset,
      limit: limit,
    ),
  ),
);

void main() {
  testWidgets('each level is indented by its server-given depth', (
    WidgetTester tester,
  ) async {
    await _pumpNodes(tester, <JobNode>[
      _node('kid', JobKind.metadata, 0),
      _node('grandkid', JobKind.artwork, 1),
      _node('greatgrandkid', JobKind.trickplay, 2),
    ]);

    final double kidX = tester.getTopLeft(find.text('kid')).dx;
    final double grandkidX = tester.getTopLeft(find.text('grandkid')).dx;
    final double deepestX = tester.getTopLeft(find.text('greatgrandkid')).dx;

    expect(grandkidX, greaterThan(kidX));
    expect(deepestX, greaterThan(grandkidX));
  });

  testWidgets('children are laid out in the shared admin table', (
    WidgetTester tester,
  ) async {
    await _pumpNodes(tester, <JobNode>[_node('kid', JobKind.metadata, 0)]);

    expect(find.byType(AdminTable<JobNode>), findsOneWidget);
    expect(find.text(Strings.columnKind.toUpperCase()), findsOneWidget);
    expect(find.text(Strings.columnStatus.toUpperCase()), findsOneWidget);
  });

  testWidgets('a job with no children hides the whole section', (
    WidgetTester tester,
  ) async {
    await _pumpNodes(tester, <JobNode>[]);

    expect(find.text(Strings.jobChildren), findsNothing);
    expect(find.byType(AdminTable<JobNode>), findsNothing);
    expect(find.text(Strings.next), findsNothing);
  });

  testWidgets('a job with children keeps the section heading', (
    WidgetTester tester,
  ) async {
    await _pumpNodes(tester, <JobNode>[_node('kid', JobKind.metadata, 0)]);

    expect(find.text(Strings.jobChildren), findsOneWidget);
  });

  testWidgets('a failed lookup says so instead of hiding', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      Future<Paged<JobNode>>.delayed(
        Duration.zero,
        () => throw Exception('nope'),
      ),
    );

    expect(find.text(Strings.couldNotLoadJobChildren), findsOneWidget);
  });

  testWidgets('the pager reports the next offset in place', (
    WidgetTester tester,
  ) async {
    final List<int> paged = await _pumpNodes(tester, <JobNode>[
      _node('a', JobKind.metadata, 0),
      _node('b', JobKind.artwork, 0),
    ], total: 5);

    await tester.tap(find.text(Strings.next));
    await tester.pumpAndSettle();

    expect(paged, <int>[2]);
  });

  testWidgets('a child row opens that job', (WidgetTester tester) async {
    await _pumpNodes(tester, <JobNode>[_node('kid', JobKind.metadata, 0)]);

    await tester.tap(find.text('Metadata'));
    await tester.pumpAndSettle();

    expect(find.text('went to /admin/job?id=kid'), findsOneWidget);
  });
}
