import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/applications_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/util/desktop_entry_native.dart';

class _FakeEntry extends DesktopEntry {
  _FakeEntry({this.added = false, this.failing = false})
    : super(appImage: '/a.AppImage', appDir: '/mnt', dataHome: '/data');

  bool added;
  final bool failing;
  Completer<void>? hold;
  int adds = 0;
  int removes = 0;

  @override
  Future<bool> installed() async => added;

  @override
  Future<void> add() async {
    adds++;
    await hold?.future;
    if (failing) {
      throw const FileSystemException('read-only');
    }
    added = true;
  }

  @override
  Future<void> remove() async {
    removes++;
    added = false;
  }
}

Widget _host(DesktopEntryLoader load) => ThemeScope(
  variant: AppThemeVariant.dark,
  setVariant: (_) {},
  child: MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    home: Scaffold(
      body: ToastHost(
        child: SingleChildScrollView(child: ApplicationsBlock(load: load)),
      ),
    ),
  ),
);

OutlinedButton _button(WidgetTester tester) =>
    tester.widget<OutlinedButton>(find.byType(OutlinedButton));

void main() {
  testWidgets('outside an AppImage the block shows nothing', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(() => null));
    await tester.pumpAndSettle();

    expect(find.text(Strings.applicationsMenuHeading), findsNothing);
    expect(find.byType(OutlinedButton), findsNothing);
  });

  testWidgets('an AppImage not yet in the menu offers to add it', (
    WidgetTester tester,
  ) async {
    final _FakeEntry entry = _FakeEntry();
    await tester.pumpWidget(_host(() => entry));
    await tester.pumpAndSettle();

    expect(find.text(Strings.applicationsMenuHeading), findsOneWidget);
    expect(find.text(Strings.applicationsMenuHelp), findsOneWidget);

    await tester.tap(find.text(Strings.addToApplicationsMenu));
    await tester.pumpAndSettle();

    expect(entry.adds, 1);
    expect(find.text(Strings.addedToApplicationsMenu), findsOneWidget);
    expect(find.text(Strings.removeFromApplicationsMenu), findsOneWidget);
  });

  testWidgets('an AppImage already in the menu offers to remove it', (
    WidgetTester tester,
  ) async {
    final _FakeEntry entry = _FakeEntry(added: true);
    await tester.pumpWidget(_host(() => entry));
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.removeFromApplicationsMenu));
    await tester.pumpAndSettle();

    expect(entry.removes, 1);
    expect(find.text(Strings.removedFromApplicationsMenu), findsOneWidget);
    expect(find.text(Strings.addToApplicationsMenu), findsOneWidget);
  });

  testWidgets('the button waits while the menu is being updated', (
    WidgetTester tester,
  ) async {
    final _FakeEntry entry = _FakeEntry()..hold = Completer<void>();
    await tester.pumpWidget(_host(() => entry));
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.addToApplicationsMenu));
    await tester.pump();

    expect(_button(tester).onPressed, isNull);

    entry.hold!.complete();
    await tester.pumpAndSettle();

    expect(_button(tester).onPressed, isNotNull);
  });

  testWidgets('a failed update says so and keeps the offer', (
    WidgetTester tester,
  ) async {
    final _FakeEntry entry = _FakeEntry(failing: true);
    await tester.pumpWidget(_host(() => entry));
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.addToApplicationsMenu));
    await tester.pumpAndSettle();

    expect(find.text(Strings.applicationsMenuFailed), findsOneWidget);
    expect(find.text(Strings.addToApplicationsMenu), findsOneWidget);
    expect(_button(tester).onPressed, isNotNull);
  });
}
