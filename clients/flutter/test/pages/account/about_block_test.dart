import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/about_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Widget _host({VersionLoader? loadVersion}) {
  final Widget block = SingleChildScrollView(
    child: AboutBlock(loadVersion: loadVersion ?? () async => '1.2.3'),
  );
  return ThemeScope(
    variant: AppThemeVariant.dark,
    setVariant: (_) {},
    child: MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(body: block),
    ),
  );
}

void main() {
  testWidgets('the block names the application and its version', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.text(Strings.accountAboutHeading), findsOneWidget);
    expect(find.text('${Strings.appTitle} 1.2.3'), findsOneWidget);
    expect(find.text(Strings.aboutLegalese), findsOneWidget);
    expect(find.text(Strings.aboutBundledHelp), findsOneWidget);
  });

  testWidgets('the block credits the content providers', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.text(Strings.aboutMetadataHeading), findsOneWidget);
    expect(find.text(Strings.aboutTmdbNotice), findsOneWidget);
    expect(find.text(Strings.aboutSubtitleProvider), findsOneWidget);
    expect(find.text(Strings.aboutOmdbNotice), findsOneWidget);
  });

  test('the OMDb credit names its licence', () {
    expect(Strings.aboutOmdbNotice, contains('OMDb'));
    expect(Strings.aboutOmdbNotice, contains('CC BY-NC 4.0'));
  });

  testWidgets('the TMDB logo renders and is bundled as an asset', (
    WidgetTester tester,
  ) async {
    final ByteData data = await rootBundle.load(
      'assets/attribution/tmdb-logo.png',
    );
    expect(data.lengthInBytes, greaterThan(0));

    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.byType(Image), findsOneWidget);
  });

  test('the TMDB notice keeps the wording TMDB requires', () {
    expect(
      Strings.aboutTmdbNotice,
      'This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB.',
    );
  });

  testWidgets('the block opens the licence page', (WidgetTester tester) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.viewThirdPartyLicenses),
    );
    await tester.pumpAndSettle();

    expect(find.byType(LicensePage), findsOneWidget);
  });

  testWidgets('a version that cannot be read still leaves the page reachable', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(loadVersion: () async => throw Exception('no platform')),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.appTitle), findsOneWidget);

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.viewThirdPartyLicenses),
    );
    await tester.pumpAndSettle();

    expect(find.byType(LicensePage), findsOneWidget);
  });
}
