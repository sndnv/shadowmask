import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/components/brand_mark.dart';
import 'package:shadowmask/components/hex_texture.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

ApiClient _unreachable() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient(
    (http.Request _) async => throw http.ClientException('down'),
  ),
);

Widget _host(ApiClient api, {String? address}) {
  final Widget app = MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    home: SectionPage(
      api: api,
      section: NavSection.movies,
      errorText: Strings.couldNotLoadMovies,
      bodyBuilder: (BuildContext context, _) => const Text('body'),
    ),
  );
  return ThemeScope(
    variant: AppThemeVariant.dark,
    setVariant: (_) {},
    child: address == null
        ? app
        : ServerScope(
            address: address,
            setAddress: (_) async {},
            probe: (_) async => true,
            child: app,
          ),
  );
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a server that never answers takes the whole page', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(_unreachable()));
    await tester.pumpAndSettle();

    // Navigation to a server that is not there is not worth offering, so the
    // shell goes with it.
    expect(find.text(Strings.serverUnreachableHeading), findsOneWidget);
    expect(find.byType(BrandMark), findsNothing);
    expect(
      find.byType(HexTexture),
      findsOneWidget,
      reason: 'the nav goes, the app still looks like itself',
    );
    expect(find.text(Strings.retry), findsOneWidget);
    expect(find.text('body'), findsNothing);
  });

  testWidgets('a device that picks its own server can pick another one', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(_unreachable(), address: 'http://test'));
    await tester.pumpAndSettle();

    // Signing in and linking both offer this, but a signed-in device never
    // reaches either page, so without it the only way off this screen is a
    // reinstall.
    await tester.tap(find.widgetWithText(TextButton, Strings.changeServer));
    await tester.pumpAndSettle();

    expect(find.byType(ServerPage), findsOneWidget);
    expect(
      tester.widget<ServerPage>(find.byType(ServerPage)).initialAddress,
      'http://test',
      reason: 'the address that failed is the one worth editing',
    );
  });

  testWidgets('a device with no say in its server is not offered one', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(_unreachable()));
    await tester.pumpAndSettle();

    // The web bundle is served next to the one backend that allows its origin,
    // so there is no ServerScope and nothing to change.
    expect(find.text(Strings.changeServer), findsNothing);
    expect(find.text(Strings.retry), findsOneWidget);
  });

  testWidgets('retrying asks the server again', (WidgetTester tester) async {
    int calls = 0;
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request _) async {
        calls++;
        throw http.ClientException('down');
      }),
    );
    await tester.pumpWidget(_host(api));
    await tester.pumpAndSettle();
    final int before = calls;

    await tester.tap(find.text(Strings.retry));
    await tester.pumpAndSettle();

    expect(calls, greaterThan(before));
  });
}
