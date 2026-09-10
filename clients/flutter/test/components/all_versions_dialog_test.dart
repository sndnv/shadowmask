import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/all_versions_dialog.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/version_picker.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/util/downloads.dart';
import 'package:shadowmask/view/version_order.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _json(String id, String quality) => <String, dynamic>{
  'id': id,
  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
  'library_id': 'l1',
  'quality': quality,
  'container': 'mkv',
  'size_bytes': 8 * 1048576,
  'duration_ms': 7200000,
  'available': true,
};

final List<Version> _shuffled = <Version>[
  Version.fromJson(_json('v-hd', 'hd')),
  Version.fromJson(_json('v-uhd', 'uhd')),
  Version.fromJson(_json('v-fhd', 'fhd')),
];

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    for (final Version v in _shuffled) {
      if (req.url.path.endsWith('/versions/${v.id}')) {
        return http.Response(
          jsonEncode(_json(v.id, v.quality.name)..remove('available')),
          200,
        );
      }
    }
    return http.Response('{}', 200);
  }),
);

Future<void> _pump(WidgetTester tester, Widget child, {Size? size}) async {
  if (size != null) {
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
  }
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: ToastHost(
          child: Scaffold(body: SingleChildScrollView(child: child)),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

List<String> _lines(WidgetTester tester) => tester
    .widgetList<Text>(find.byType(Text))
    .map((Text t) => t.data ?? t.textSpan?.toPlainText() ?? '')
    .toList();

bool _numbered(List<String> lines, String number, String quality) =>
    lines.any((String l) => l.startsWith('$number · $quality'));

void main() {
  final DownloadStarter platform = startDownload;

  setUp(() {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    startDownload = (String url, String filename) async => true;
  });
  tearDown(() => startDownload = platform);

  testWidgets('the dialog numbers versions the same way the picker does', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _api();
    await _pump(
      tester,
      VersionPicker(
        catalog: CatalogApi(api),
        playback: PlaybackApi(api),
        userId: 'u1',
        versions: _shuffled,
      ),
    );
    final List<String> picker = _lines(tester);
    expect(_numbered(picker, '1', 'UHD'), isTrue);
    expect(_numbered(picker, '2', 'FHD'), isTrue);
    expect(_numbered(picker, '3', 'HD'), isTrue);

    await _pump(
      tester,
      AllVersionsDialog(
        catalog: CatalogApi(api),
        playback: PlaybackApi(api),
        userId: 'u1',
        versions: orderedVersions(_shuffled),
      ),
    );
    final List<String> dialog = _lines(tester);
    expect(_numbered(dialog, '1', 'UHD'), isTrue);
    expect(_numbered(dialog, '2', 'FHD'), isTrue);
    expect(_numbered(dialog, '3', 'HD'), isTrue);
  });

  testWidgets('the dialog hosts the real picker, not a second list', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _api();
    await _pump(
      tester,
      AllVersionsDialog(
        catalog: CatalogApi(api),
        playback: PlaybackApi(api),
        userId: 'u1',
        versions: orderedVersions(_shuffled),
      ),
    );

    expect(find.byType(VersionPicker), findsOneWidget);
    expect(
      _lines(tester).where((String l) => l.startsWith('Versions (')).length,
      1,
      reason:
          'only the dialog title carries the count, the picker heading is hidden',
    );
  });

  testWidgets('a phone row shows only what fits and stays expandable', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _api();
    await _pump(
      tester,
      SizedBox(
        width: 280,
        child: VersionPicker(
          catalog: CatalogApi(api),
          playback: PlaybackApi(api),
          userId: 'u1',
          versions: _shuffled,
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    expect(_numbered(_lines(tester), '1', 'UHD'), isTrue);
    // Play and download move into the expanded body so the row cannot overflow.
    expect(find.text(Strings.play), findsNothing);
    expect(find.byIcon(Icons.download_outlined), findsNothing);
    expect(find.byIcon(Icons.expand_more), findsNWidgets(3));

    await tester.tap(find.byIcon(Icons.expand_more).first);
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text(Strings.play), findsOneWidget);
    expect(find.byIcon(Icons.download_outlined), findsOneWidget);
    expect(
      tester.getRect(find.byIcon(Icons.download_outlined)).left,
      greaterThan(tester.getRect(find.text(Strings.play)).right),
      reason: 'play leads the row and download sits at the far end',
    );
  });

  testWidgets('a wide row keeps play and download in place', (
    WidgetTester tester,
  ) async {
    final ApiClient api = _api();
    await _pump(
      tester,
      SizedBox(
        width: 760,
        child: VersionPicker(
          catalog: CatalogApi(api),
          playback: PlaybackApi(api),
          userId: 'u1',
          versions: _shuffled,
        ),
      ),
    );

    expect(find.text(Strings.play), findsNWidgets(3));
    expect(find.byIcon(Icons.download_outlined), findsNWidgets(3));
  });
}
