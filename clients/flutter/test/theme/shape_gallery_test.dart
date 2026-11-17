import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/action_field.dart';
import 'package:shadowmask/components/admin/edited_notice.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/admin/job_status_chip.dart';
import 'package:shadowmask/components/admin/log_view.dart';
import 'package:shadowmask/components/admin/match_row.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/count_pill.dart';
import 'package:shadowmask/components/detail_split.dart';
import 'package:shadowmask/components/diagnostics/report_view.dart';
import 'package:shadowmask/components/link_chip.dart';
import 'package:shadowmask/components/link_code_text.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/outline_pill.dart';
import 'package:shadowmask/components/player/shortcuts_dialog.dart';
import 'package:shadowmask/components/player/trickplay_thumb.dart';
import 'package:shadowmask/components/player/up_next_card.dart';
import 'package:shadowmask/components/progress_meter.dart';
import 'package:shadowmask/components/segmented_tabs.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/components/watched_marker.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/pages/account/appearance_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';

import '../support/shape_guard.dart';

Future<void> _pump(
  WidgetTester tester,
  AppThemeVariant variant,
  Widget child,
) async {
  tester.view.physicalSize = const Size(1200, 3200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: variant,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(variant),
        home: Scaffold(body: SingleChildScrollView(child: child)),
      ),
    ),
  );
  await tester.pump();
}

Widget _gallery() => Column(
  crossAxisAlignment: CrossAxisAlignment.start,
  children: <Widget>[
    const LogView(entries: <String>['INFO server started']),
    MatchRow(label: 'Alpha (2021)', tooltip: 'Pick', onTap: () {}),
    const LinkCodeText('ABCD1234'),
    const EditedNotice(),
    const ReportView(
      payload: <String, Object?>{
        'log': <Object?>['started'],
      },
    ),
    const TrickplayThumb(tile: null, positionMs: 61000),
    UpNextCard(
      label: 'Next episode',
      remainingSeconds: 5,
      onPlay: () {},
      onCancel: () {},
    ),
    const SkeletonCard(aspect: CardAspect.poster),
    const SkeletonRows(rows: 2),
    const SkeletonDetail(),
    const SkeletonPage(toolbar: true, child: SizedBox(height: 10)),
    const CountPill(count: 3),
    const OutlinePill(icon: Icons.closed_caption, text: 'CC 2'),
    const OutlinePill(icon: Icons.closed_caption, text: 'CC 0', active: false),
    LinkChip(label: 'Drama', onTap: () {}),
    const LinkChip(label: 'Rating', value: '7.4'),
    const ProgressMeter(percent: 40),
    const WatchedMarker(),
    const JobStatusChip(JobStatus.running),
    ToggleButton(
      icon: Icons.bookmark_border,
      filledIcon: Icons.bookmark,
      label: 'Watchlist',
      pressed: true,
      onToggle: () {},
    ),
    ToggleButton(
      icon: Icons.check,
      filledIcon: Icons.check,
      label: 'Watched',
      pressed: false,
      onToggle: () {},
    ),
    SegmentedTabs<String>(
      current: 'a',
      tabs: const <(String, String)>[('a', 'Active'), ('b', 'All')],
      onChanged: (_) {},
    ),
    ActionField(
      controller: TextEditingController(),
      enabled: true,
      label: 'Search',
      hintText: 'Find a title',
      onSubmitted: () {},
      actionIcon: Icons.search,
      actionTooltip: 'Search',
    ),
    MenuField(open: false, onTap: () {}, child: const Text('Any')),
    InputChip(label: const Text('Alpha'), onDeleted: () {}),
    const AppearanceBlock(),
    const DetailSplit(poster: SizedBox(height: 120), info: Text('Info')),
    const FieldHelp(title: 'Format', body: 'Which file.', example: 'best'),
    SizedBox(
      width: 180,
      child: CatalogCardTile(
        card: CatalogCard(
          ref: const TitleRef(type: TitleKind.series, id: 's1'),
          route: '/title?type=series&id=s1',
          title: 'Gamma',
        ),
        imageBase: 'http://test',
        width: 180,
        badge: 'S2',
      ),
    ),
  ],
);

void main() {
  for (final AppThemeVariant variant in AppThemeVariant.values) {
    testWidgets('the component gallery draws the ${variant.name} shapes', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant, _gallery());

      expect(find.byType(UpNextCard), findsOneWidget);
      expectShapesOf(tester, variant);
    });

    testWidgets('${variant.name} sets pills and times in its label face', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant, _gallery());

      final String? expected = variant == AppThemeVariant.hologram
          ? kMonoFamily
          : null;
      expect(
        tester.widget<Text>(find.text('CC 2')).style?.fontFamily,
        expected,
      );
      expect(
        tester.widget<Text>(find.text('1:01')).style?.fontFamily,
        expected,
      );
      expect(tester.widget<Text>(find.text('S2')).style?.fontFamily, expected);
    });

    testWidgets('a help dialog with an example draws the ${variant.name} '
        'shapes', (WidgetTester tester) async {
      await _pump(
        tester,
        variant,
        const FieldHelp(title: 'Format', body: 'Which file.', example: 'best'),
      );

      await tester.tap(find.byTooltip(Strings.whatIsThis));
      await tester.pumpAndSettle();

      expect(find.text('best'), findsOneWidget);
      expectShapesOf(tester, variant);
    });

    testWidgets('the shortcuts dialog draws the ${variant.name} shapes', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        variant,
        Builder(
          builder: (BuildContext context) => TextButton(
            onPressed: () => showShortcutsDialog(context),
            child: const Text('open'),
          ),
        ),
      );

      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();

      expect(
        find.text(
          variant == AppThemeVariant.hologram
              ? Strings.shortcutsHeading.toUpperCase()
              : Strings.shortcutsHeading,
        ),
        findsOneWidget,
      );
      expectShapesOf(tester, variant);
    });
  }
}
