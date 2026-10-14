import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/api/play_on_api.dart';
import 'package:shadowmask/components/play_on_button.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../support/play_on.dart';

Version _version(String id, {bool available = true}) => Version(
  id: id,
  title: const TitleRef(type: TitleKind.movie, id: 'm1'),
  libraryId: 'l1',
  quality: Quality.fhd,
  container: 'mkv',
  durationMs: 600000,
  available: available,
);

Future<PlayChoice?> Function() _one(Version v) =>
    () async => (ordered: <Version>[v], target: v);

PlayOnApi _playOn(
  PlayOnServer server, {
  Future<http.Response> Function(http.Request req)? targets,
}) => server.api(
  ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient(
      (http.Request req) async => server.answer(req) ?? http.Response('', 404),
    ),
  ),
  targets: targets,
);

Future<void> _pump(
  WidgetTester tester,
  PlayOnApi playOn,
  Future<PlayChoice?> Function() choose, {
  bool plain = false,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: ToastHost(
        child: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: plain
                ? PlayOnButton.icon(
                    playOn: playOn,
                    devices: kPlayOnDevices,
                    choose: choose,
                  )
                : PlayOnButton(
                    playOn: playOn,
                    devices: kPlayOnDevices,
                    choose: choose,
                  ),
          ),
        ),
      ),
    ),
  );
}

Future<void> _open(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.devices));
  await tester.pumpAndSettle();
}

Future<void> _pick(WidgetTester tester, String name) async {
  await tester.tap(find.text(name));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('one version lists the devices and sends to the one picked', (
    WidgetTester tester,
  ) async {
    final PlayOnServer server = PlayOnServer();
    await _pump(tester, _playOn(server), _one(_version('v1')));

    await _open(tester);

    expect(find.byType(SubmenuButton), findsNothing);
    expect(
      find.widgetWithIcon(MenuItemButton, Icons.tv),
      findsOneWidget,
      reason: 'the Roku',
    );
    expect(
      find.widgetWithIcon(MenuItemButton, Icons.phone_android),
      findsOneWidget,
      reason: 'the Android app',
    );

    await _pick(tester, 'Living room');

    expect(server.mints, <Mint>[(device: 'd1', version: 'v1')]);
    expect(server.lanCalls, <String>[
      'GET 10.0.0.2/query/active-app',
      'POST 10.0.0.2/launch/dev',
    ]);
    expect(find.text(Strings.playOnPlaying('Living room')), findsOneWidget);
  });

  testWidgets('several versions list each version with the devices under it', (
    WidgetTester tester,
  ) async {
    final PlayOnServer server = PlayOnServer();
    await _pump(
      tester,
      _playOn(server),
      () async => (
        ordered: <Version>[
          _version('v1'),
          _version('v2'),
          _version('v3', available: false),
        ],
        target: null,
      ),
    );

    await _open(tester);

    expect(find.byType(SubmenuButton), findsNWidgets(2));
    expect(find.text('Pixel'), findsNothing);

    await tester.tap(find.byType(SubmenuButton).last);
    await tester.pumpAndSettle();
    await _pick(tester, 'Pixel');

    expect(server.mints, <Mint>[(device: 'd2', version: 'v2')]);
    expect(server.lanCalls, <String>['POST 10.0.0.5/play']);
    expect(find.text(Strings.playOnPlaying('Pixel')), findsOneWidget);
  });

  testWidgets('a device that cannot be reached says so', (
    WidgetTester tester,
  ) async {
    final PlayOnServer server = PlayOnServer();
    await _pump(
      tester,
      _playOn(
        server,
        targets: (http.Request req) =>
            Future<http.Response>.error(http.ClientException('down')),
      ),
      _one(_version('v1')),
    );

    await _open(tester);
    await _pick(tester, 'Living room');

    expect(find.text(Strings.playOnUnreachable('Living room')), findsOneWidget);
  });

  testWidgets('a device that refuses says so', (WidgetTester tester) async {
    final PlayOnServer server = PlayOnServer();
    await _pump(
      tester,
      _playOn(
        server,
        targets: (http.Request req) async => http.Response('', 403),
      ),
      _one(_version('v1')),
    );

    await _open(tester);
    await _pick(tester, 'Pixel');

    expect(find.text(Strings.playOnRefused('Pixel')), findsOneWidget);
  });

  testWidgets('a command the server refuses shows its reason', (
    WidgetTester tester,
  ) async {
    final PlayOnServer server = PlayOnServer(mintStatus: 404);
    await _pump(tester, _playOn(server), _one(_version('v1')));

    await _open(tester);
    await _pick(tester, 'Living room');

    expect(server.lan, isEmpty);
    expect(
      find.text(
        failureText(
          Strings.errorAction,
          ApiException('', code: 'version_not_found'),
        ),
      ),
      findsOneWidget,
    );
  });

  testWidgets('nothing to play says so and opens no menu', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _playOn(PlayOnServer()), () async => null);

    await _open(tester);

    expect(find.text(Strings.nothingToPlay), findsOneWidget);
    expect(find.text('Living room'), findsNothing);
  });

  testWidgets('versions without a file say there is nothing to play', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _playOn(PlayOnServer()),
      () async =>
          (ordered: <Version>[_version('v1', available: false)], target: null),
    );

    await _open(tester);

    expect(find.text(Strings.nothingToPlay), findsOneWidget);
  });

  testWidgets('a lookup that fails shows the action error', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _playOn(PlayOnServer()),
      () async => throw Exception('boom'),
    );

    await _open(tester);

    expect(find.text(Strings.errorAction), findsOneWidget);
  });

  testWidgets('the button is busy until the device answers', (
    WidgetTester tester,
  ) async {
    final Completer<http.Response> answer = Completer<http.Response>();
    final PlayOnServer server = PlayOnServer();
    await _pump(
      tester,
      _playOn(server, targets: (http.Request req) => answer.future),
      _one(_version('v1')),
    );

    await _open(tester);
    await tester.tap(find.text('Pixel'));
    await tester.pump();
    await tester.pump();

    expect(tester.widget<ToggleButton>(find.byType(ToggleButton)).busy, isTrue);

    answer.complete(http.Response('', 202));
    await tester.pumpAndSettle();

    expect(
      tester.widget<ToggleButton>(find.byType(ToggleButton)).busy,
      isFalse,
    );
    expect(find.text(Strings.playOnPlaying('Pixel')), findsOneWidget);
  });

  testWidgets('a second press closes the open menu', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _playOn(PlayOnServer()), _one(_version('v1')));

    await _open(tester);
    expect(find.text('Living room'), findsOneWidget);

    await _open(tester);
    expect(find.text('Living room'), findsNothing);
  });

  testWidgets('a lookup that outlives the button does nothing', (
    WidgetTester tester,
  ) async {
    final Completer<PlayChoice?> choice = Completer<PlayChoice?>();
    await _pump(tester, _playOn(PlayOnServer()), () => choice.future);

    await tester.tap(find.byIcon(Icons.devices));
    await tester.pump();
    await tester.pumpWidget(const SizedBox.shrink());
    choice.complete((ordered: <Version>[_version('v1')], target: null));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
  });

  testWidgets('the action look matches the toggles beside it', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _playOn(PlayOnServer()), _one(_version('v1')));

    final ToggleButton button = tester.widget<ToggleButton>(
      find.byType(ToggleButton),
    );
    expect(button.compact, isTrue);
    expect(button.highlight, isFalse);
    expect(find.byTooltip(Strings.playOn), findsOneWidget);
    expect(
      tester.getRect(find.byType(ToggleButton)).left,
      greaterThan(0),
      reason: 'a gap separates it from the Play button before it',
    );
  });

  testWidgets('the dialog look is a plain icon button that sends too', (
    WidgetTester tester,
  ) async {
    final PlayOnServer server = PlayOnServer();
    await _pump(tester, _playOn(server), _one(_version('v1')), plain: true);

    expect(find.byType(ToggleButton), findsNothing);
    expect(find.widgetWithIcon(IconButton, Icons.devices), findsOneWidget);
    expect(find.byTooltip(Strings.playOn), findsOneWidget);

    await _open(tester);
    await _pick(tester, 'Pixel');

    expect(server.mints, <Mint>[(device: 'd2', version: 'v1')]);
  });
}
