import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/link_codes_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Future<void> _pump(
  WidgetTester tester, {
  required List<String> seen,
  required Completer<void> revoke,
}) async {
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      if (req.method == 'DELETE') {
        seen.add(req.url.path);
        await revoke.future;
        return http.Response('', 204);
      }
      return http.Response(
        jsonEncode(<dynamic>[
          <String, dynamic>{
            'code': 'ABC123',
            'expires_at': '2026-12-01T00:00:00Z',
          },
        ]),
        200,
      );
    }),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: SingleChildScrollView(
              child: LinkCodesBlock(api: api, userId: 'u1'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('revoking twice on a slow connection sends one request', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    final Completer<void> revoke = Completer<void>();
    await _pump(tester, seen: seen, revoke: revoke);

    // Shown in groups of four so it can be read off a screen onto a remote.
    expect(find.text('ABC1 23'), findsOneWidget);

    await tester.tap(find.widgetWithText(TextButton, Strings.revoke));
    await tester.pump();

    // The create button next to it was already guarded; this one was not,
    // eight lines away in the same widget.
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, Strings.revoke))
          .onPressed,
      isNull,
    );

    await tester.tap(
      find.widgetWithText(TextButton, Strings.revoke),
      warnIfMissed: false,
    );
    await tester.pump();

    expect(seen.length, 1);

    revoke.complete();
    await tester.pumpAndSettle();
    expect(find.text(Strings.toastCodeRevoked), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });
}
