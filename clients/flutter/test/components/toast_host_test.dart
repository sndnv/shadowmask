import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/bottom_chrome.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';

Rect _cardRect(WidgetTester tester, String message) => tester.getRect(
  find.ancestor(of: find.text(message), matching: find.byType(Container)).first,
);

Color? _groundOf(WidgetTester tester, String message) {
  final Container card = tester.widget<Container>(
    find
        .ancestor(of: find.text(message), matching: find.byType(Container))
        .first,
  );
  return (card.decoration as BoxDecoration?)?.color;
}

void main() {
  testWidgets('shows a toast then auto-dismisses after its duration', (
    WidgetTester tester,
  ) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).show('Added.');
    await tester.pump();
    expect(find.text('Added.'), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
    expect(find.text('Added.'), findsNothing);
  });

  testWidgets('stacks multiple toasts', (WidgetTester tester) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).show('Added.');
    Toasts.of(ctx).show('Removed.');
    await tester.pump();
    expect(find.text('Added.'), findsOneWidget);
    expect(find.text('Removed.'), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a dialog route reaches a host mounted above the navigator', (
    WidgetTester tester,
  ) async {
    late BuildContext pageContext;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: Scaffold(
          body: Builder(
            builder: (BuildContext c) {
              pageContext = c;
              return const SizedBox.expand();
            },
          ),
        ),
      ),
    );

    unawaited(
      showDialog<void>(
        context: pageContext,
        builder: (BuildContext dialogContext) => TextButton(
          onPressed: () => Toasts.of(dialogContext).show('Saved.'),
          child: const Text('save'),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('save'));
    await tester.pump();

    expect(find.text('Saved.'), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('each severity paints its own ground', (
    WidgetTester tester,
  ) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).success('Saved.');
    Toasts.of(ctx).error('Save failed.');
    Toasts.of(ctx).warning('Partly done.');
    Toasts.of(ctx).info('Queued.');
    await tester.pump();

    expect(_groundOf(tester, 'Saved.'), Tokens.dark.ok);
    expect(_groundOf(tester, 'Save failed.'), Tokens.dark.danger);
    expect(_groundOf(tester, 'Partly done.'), Tokens.dark.warn);
    expect(_groundOf(tester, 'Queued.'), Tokens.dark.text);

    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a plain show stays neutral', (WidgetTester tester) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).show('Added.');
    await tester.pump();

    expect(_groundOf(tester, 'Added.'), Tokens.dark.text);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('showing a toast without a host is a no-op', (
    WidgetTester tester,
  ) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Builder(
          builder: (BuildContext c) {
            ctx = c;
            return const SizedBox.expand();
          },
        ),
      ),
    );

    Toasts.of(ctx).show('Added.');
    await tester.pump();

    expect(find.text('Added.'), findsNothing);
  });

  testWidgets('a toast is announced, errors more insistently', (
    WidgetTester tester,
  ) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).success('Added.');
    await tester.pump();

    // The toast is the only report of a mutation, and it is gone in 3.2s, so
    // it has to be spoken rather than merely drawn.
    List<CapturedAccessibilityAnnouncement> spoken = tester.takeAnnouncements();
    expect(spoken.single.message, 'Added.');
    expect(spoken.single.assertiveness, Assertiveness.polite);

    Toasts.of(ctx).error('Could not delete the user.');
    await tester.pump();

    spoken = tester.takeAnnouncements();
    expect(spoken.single.message, 'Could not delete the user.');
    expect(spoken.single.assertiveness, Assertiveness.assertive);

    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('an error outlives the other toasts and can be closed early', (
    WidgetTester tester,
  ) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).success('Saved.');
    Toasts.of(ctx).error('Save failed.');
    await tester.pump();

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
    expect(find.text('Saved.'), findsNothing);
    expect(
      find.text('Save failed.'),
      findsOneWidget,
      reason: 'a failure that vanishes in 3.2s is a failure nobody read',
    );

    await tester.tap(find.byIcon(Icons.close));
    await tester.pump();

    expect(find.text('Save failed.'), findsNothing);
  });

  testWidgets('only an error offers a way to close it', (
    WidgetTester tester,
  ) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).success('Saved.');
    await tester.pump();

    expect(find.byIcon(Icons.close), findsNothing);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a roomy window stacks toasts under the header, top right', (
    WidgetTester tester,
  ) async {
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).show('First.');
    await tester.pump();
    Toasts.of(ctx).show('Second.');
    await tester.pump();

    final Rect first = _cardRect(tester, 'First.');
    final Rect second = _cardRect(tester, 'Second.');
    final Size window = tester.view.physicalSize / tester.view.devicePixelRatio;

    expect(first.top, lessThan(window.height / 2));
    expect(second.top, lessThan(first.top), reason: 'newest first');
    expect(window.width - first.right, closeTo(Space.s4, 1));
    expect(
      second.top,
      greaterThanOrEqualTo(kShellHeaderHeight),
      reason: 'the shell nav bar is up there',
    );

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a phone puts them along the bottom instead', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(390, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).show('Added.');
    await tester.pump();

    // Where Android puts them, and clear of a thumb on the way to the nav.
    final Rect card = tester.getRect(find.text('Added.'));
    expect(card.top, greaterThan(400));
    expect(tester.takeException(), isNull);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a toast clears the bottom nav bar as well as the gesture bar', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(390, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: BottomChrome(
              height: 48,
              child: Builder(
                builder: (BuildContext c) {
                  ctx = c;
                  return const SizedBox.expand();
                },
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    Toasts.of(ctx).show('Added.');
    await tester.pumpAndSettle();

    expect(_cardRect(tester, 'Added.').bottom, lessThanOrEqualTo(800 - 48));

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a toast clears the gesture bar it would otherwise hide under', (
    WidgetTester tester,
  ) async {
    // The app draws edge to edge from Android 15, so a bottom offset that
    // ignores the inset puts most of the card under the navigation bar.
    tester.view.physicalSize = const Size(390, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    const EdgeInsets safe = EdgeInsets.only(bottom: 48);

    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: MediaQuery(
          data: const MediaQueryData(size: Size(390, 800), padding: safe),
          child: Scaffold(
            body: ToastHost(
              child: Builder(
                builder: (BuildContext c) {
                  ctx = c;
                  return const SizedBox.expand();
                },
              ),
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).show('Added.');
    await tester.pump();

    final Rect card = _cardRect(tester, 'Added.');
    expect(card.bottom, lessThanOrEqualTo(800 - safe.bottom));

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the toast stack is a live region', (WidgetTester tester) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    late BuildContext ctx;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: Builder(
              builder: (BuildContext c) {
                ctx = c;
                return const SizedBox.expand();
              },
            ),
          ),
        ),
      ),
    );

    Toasts.of(ctx).show('Added.');
    await tester.pump();

    expect(
      tester.getSemantics(find.text('Added.')),
      matchesSemantics(label: 'Added.', isLiveRegion: true),
    );

    semantics.dispose();
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });
}
