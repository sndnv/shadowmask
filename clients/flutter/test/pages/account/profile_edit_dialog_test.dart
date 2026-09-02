import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/server/rating_system.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/pages/account/profile_edit_dialog.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

const AccountProfile _profile = AccountProfile(
  id: 'u1',
  username: 'pat',
  role: UserRole.user,
  createdAt: '2026-08-17T09:00:00Z',
  updatedAt: '2026-08-17T09:00:00Z',
);

const List<RatingSystem> _systems = <RatingSystem>[
  RatingSystem(system: 'mpaa', codes: <String>['g', 'pg', 'pg-13']),
  RatingSystem(system: 'bbfc', codes: <String>['u', '12a', '15']),
];

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the pickers write codes, a rating and byte values', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(900, 1400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    Map<String, dynamic>? sent;
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        if (req.method == 'PUT') {
          sent = jsonDecode(req.body) as Map<String, dynamic>;
          return http.Response('{}', 200);
        }
        return http.Response('{}', 200);
      }),
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: Scaffold(
          body: ProfileEditDialog(
            catalog: CatalogApi(api),
            userId: 'u1',
            profile: _profile,
            ratingSystems: _systems,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.optionNoPreference).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Japanese').last);
    await tester.pumpAndSettle();

    expect(find.text('Japanese'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(sent?['preferred_audio'], <String>['jpn']);
    expect(sent?['preferred_subtitle'], <String>[]);
    expect(sent?['max_content_rating'], isNull);
    expect(sent?['concurrent_stream_limit'], isNull);
    expect(sent?['bitrate_cap'], isNull);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the rating code list follows the chosen system', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(900, 1400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient(
        (http.Request _) async => http.Response('{}', 200),
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        home: Scaffold(
          body: ProfileEditDialog(
            catalog: CatalogApi(api),
            userId: 'u1',
            profile: _profile,
            ratingSystems: _systems,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.optionNoLimit).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('BBFC').last);
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.optionNoLimit).first);
    await tester.pumpAndSettle();

    expect(find.text('12A'), findsOneWidget);
    expect(find.text('PG-13'), findsNothing);
  });
}
