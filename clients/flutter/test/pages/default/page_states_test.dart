import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/authorization_failure.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Future<String> _fails(Object error) async {
  await Future<void>.delayed(Duration.zero);
  throw error;
}

Future<void> _pump(WidgetTester tester, Future<String> future) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: buildBlock<String>(
          future: future,
          errorText: Strings.couldNotLoadMovies,
          builder: (BuildContext context, String data) => Text(data),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('a block that fails says so out loud', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester, _fails(Exception('nope')));

    // The block swaps a skeleton for this text with no other signal, so
    // nothing announces the failure unless the text itself does.
    expect(
      tester.getSemantics(find.text(Strings.couldNotLoadMovies)),
      matchesSemantics(label: Strings.couldNotLoadMovies, isLiveRegion: true),
    );

    semantics.dispose();
  });

  testWidgets('a refused block says so out loud as well', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester, _fails(AuthorizationFailure()));

    expect(
      tester.getSemantics(find.text(Strings.notAuthorized)),
      matchesSemantics(label: Strings.notAuthorized, isLiveRegion: true),
    );

    semantics.dispose();
  });

  testWidgets('a block that loads stays quiet', (WidgetTester tester) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester, Future<String>.value('Movies'));

    expect(
      tester.getSemantics(find.text('Movies')),
      matchesSemantics(label: 'Movies'),
    );

    semantics.dispose();
  });
}
