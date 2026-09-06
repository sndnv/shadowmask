import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Widget _host({
  required Future<bool> Function(String) probe,
  required List<String> saved,
  String? address,
  Widget? home,
}) => ThemeScope(
  variant: AppThemeVariant.dark,
  setVariant: (_) {},
  child: ServerScope(
    address: address,
    setAddress: (String a) async => saved.add(a),
    probe: probe,
    child: MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: home ?? const ServerPage(),
    ),
  ),
);

Widget _pushesServerPage(String address) => Builder(
  builder: (BuildContext context) => TextButton(
    onPressed: () => Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (BuildContext context) => ServerPage(initialAddress: address),
      ),
    ),
    child: const Text('open'),
  ),
);

Future<void> _connect(WidgetTester tester, String address) async {
  await tester.enterText(find.byType(TextField), address);
  await tester.tap(find.widgetWithText(FilledButton, Strings.connect));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('the first run has nowhere to go back to, so it offers no way', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(probe: (_) async => true, saved: <String>[]));
    await tester.pumpAndSettle();

    expect(find.text(Strings.cancel), findsNothing);
  });

  testWidgets(
    'an address that is not one is refused before anything is dialled',
    (WidgetTester tester) async {
      final List<String> probed = <String>[];
      final List<String> saved = <String>[];
      await tester.pumpWidget(
        _host(
          probe: (String a) async {
            probed.add(a);
            return true;
          },
          saved: saved,
        ),
      );
      await tester.pumpAndSettle();
      await _connect(tester, 'ftp://host');

      expect(find.text(Strings.serverAddressInvalid), findsOneWidget);
      expect(probed, isEmpty);
      expect(saved, isEmpty);
    },
  );

  testWidgets('an address nothing answers on is reported and not kept', (
    WidgetTester tester,
  ) async {
    final List<String> saved = <String>[];
    await tester.pumpWidget(_host(probe: (_) async => false, saved: saved));
    await tester.pumpAndSettle();
    await _connect(tester, 'host:9999');

    expect(find.text(Strings.serverAddressUnanswered), findsOneWidget);
    expect(saved, isEmpty);
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNotNull,
      reason: 'the entry stays usable so the address can be corrected',
    );
  });

  testWidgets('an address that answers is normalised on the way to the store', (
    WidgetTester tester,
  ) async {
    final List<String> saved = <String>[];
    await tester.pumpWidget(_host(probe: (_) async => true, saved: saved));
    await tester.pumpAndSettle();
    await _connect(tester, '192.168.1.10:8080/');

    expect(saved, <String>['http://192.168.1.10:8080']);
  });

  testWidgets('changing an address already in use can be abandoned', (
    WidgetTester tester,
  ) async {
    final List<String> saved = <String>[];
    await tester.pumpWidget(
      _host(
        probe: (_) async => true,
        saved: saved,
        address: 'http://one',
        home: _pushesServerPage('http://one'),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.cancel), findsOneWidget);
    await tester.tap(find.text(Strings.cancel));
    await tester.pumpAndSettle();

    expect(find.text('open'), findsOneWidget);
    expect(saved, isEmpty);
  });

  testWidgets(
    're-entering the address in use goes back rather than resetting',
    (WidgetTester tester) async {
      final List<String> saved = <String>[];
      await tester.pumpWidget(
        _host(
          probe: (_) async => true,
          saved: saved,
          address: 'http://one',
          home: _pushesServerPage('http://one'),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, Strings.connect));
      await tester.pumpAndSettle();

      expect(find.text('open'), findsOneWidget);
      expect(
        saved,
        isEmpty,
        reason: 'nothing changed, so the sign in state is worth keeping',
      );
    },
  );
}
