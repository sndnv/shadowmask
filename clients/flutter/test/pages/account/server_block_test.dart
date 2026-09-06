import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/server_block.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Widget _host({String? address, bool scoped = true}) {
  const Widget block = SingleChildScrollView(child: ServerBlock());
  final Widget app = MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    home: const Scaffold(body: block),
  );
  return ThemeScope(
    variant: AppThemeVariant.dark,
    setVariant: (_) {},
    child: scoped
        ? ServerScope(
            address: address,
            setAddress: (_) async {},
            probe: (_) async => true,
            child: app,
          )
        : app,
  );
}

void main() {
  testWidgets('the block names the server this device is talking to', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(address: 'http://host:8080'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.serverHeading), findsOneWidget);
    expect(find.text('http://host:8080'), findsOneWidget);
    expect(find.text(Strings.changeServerHelp), findsOneWidget);
  });

  testWidgets('the block opens the server screen with the address filled in', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(address: 'http://host:8080'));
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.changeServer));
    await tester.pumpAndSettle();

    expect(find.byType(ServerPage), findsOneWidget);
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller?.text,
      'http://host:8080',
    );
  });

  testWidgets('a client whose address is built in shows nothing at all', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(scoped: false));
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.serverHeading),
      findsNothing,
      reason: 'on the web the address is the origin, so there is no choice',
    );
  });
}
