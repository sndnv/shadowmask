import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/main.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the toast host sits above the navigator', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      ShadowmaskApp(
        api: ApiClient(
          baseUrl: 'http://test',
          httpClient: MockClient(
            (http.Request _) async => http.Response('{}', 200),
          ),
        ),
      ),
    );
    await tester.pump();

    final Finder host = find.byType(ToastHost);
    expect(host, findsOneWidget);
    expect(
      find.descendant(of: host, matching: find.byType(Navigator)),
      findsWidgets,
    );
  });
}
