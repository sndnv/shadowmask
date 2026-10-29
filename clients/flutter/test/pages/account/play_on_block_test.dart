import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/play_on_host.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/play_on_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/util/command_listener_types.dart';
import 'package:shadowmask/view/play_on_receiver.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/play_on_target.dart';
import '../../support/setting_rows.dart';

class _Block {
  _Block({List<InterfaceAddress> addresses = kWifi}) {
    receiver = PlayOnReceiver(
      api: ApiClient(
        baseUrl: 'http://test',
        httpClient: MockClient(
          (http.Request req) async =>
              server.answer(req) ?? http.Response('', 404),
        ),
      ),
      onPlay: (String _) async {},
      onRefused: () {},
      retryDelay: Duration.zero,
      bind: binder.call,
      addresses: () async => addresses,
    );
  }

  final TargetServer server = TargetServer();
  final FakeBinder binder = FakeBinder();
  late final PlayOnReceiver receiver;

  Future<void> pump(WidgetTester tester) async {
    addTearDown(receiver.dispose);
    receiver.load();
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: ToastHost(
          child: PlayOnScope(
            receiver: receiver,
            child: const Scaffold(
              body: SingleChildScrollView(child: PlayOnBlock()),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }
}

Future<void> _toggle(WidgetTester tester) async {
  await tester.tap(find.byType(Switch));
  await tester.pumpAndSettle();
}

Future<void> _savePort(WidgetTester tester, String text) async {
  await tester.enterText(find.byType(TextField), text);
  await tester.tap(find.text(Strings.save));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('nothing shows without a receiver', (WidgetTester tester) async {
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: PlayOnBlock())),
    );

    expect(find.text(Strings.playOnHeading), findsNothing);
  });

  testWidgets('nothing shows on a device that is not linked', (
    WidgetTester tester,
  ) async {
    await _Block().pump(tester);

    expect(find.text(Strings.playOnHeading), findsNothing);
  });

  testWidgets('a linked device shows the switch off with its help', (
    WidgetTester tester,
  ) async {
    linkedDevice();
    await _Block().pump(tester);

    expect(find.text(Strings.playOnHeading), findsOneWidget);
    expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);
    expect(find.text(Strings.playOnAllowHelp), findsOneWidget);
    expect(find.text(Strings.playOnPort), findsNothing);
  });

  testWidgets('every setting row is laid out alike, however long its help', (
    WidgetTester tester,
  ) async {
    linkedDevice(enabled: true);
    await _Block().pump(tester);

    final String listening = Strings.playOnListening(<String>[
      '192.168.1.20',
    ], 41000);
    for (final (String title, String help) in <(String, String)>[
      (Strings.playOnAllow, listening),
      (Strings.playOnPort, Strings.playOnPortHelp),
    ]) {
      expectSettingRow(tester, title, help);
    }
  });

  testWidgets('turning it on shows where it listens and the port field', (
    WidgetTester tester,
  ) async {
    linkedDevice();
    final _Block block = _Block();
    await block.pump(tester);

    await _toggle(tester);

    expect(
      find.text(Strings.playOnListening(<String>['192.168.1.20'], 41000)),
      findsOneWidget,
    );
    expect(find.text(Strings.playOnPort), findsOneWidget);
    expect(find.text(Strings.playOnPortHelp), findsOneWidget);
    expect(find.text(Strings.toastSettingSaved), findsOneWidget);

    await _toggle(tester);

    expect(find.text(Strings.playOnAllowHelp), findsOneWidget);
    expect(find.text(Strings.playOnPort), findsNothing);
    expect(block.server.clears, 1);
  });

  testWidgets('a port out of range is refused and not used', (
    WidgetTester tester,
  ) async {
    linkedDevice(enabled: true);
    final _Block block = _Block();
    await block.pump(tester);

    await _savePort(tester, '80');

    expect(find.text(Strings.playOnPortInvalid), findsOneWidget);
    expect(block.binder.asked, <int>[0]);

    await _savePort(tester, '65536');

    expect(find.text(Strings.playOnPortInvalid), findsOneWidget);
    expect(block.binder.asked, <int>[0]);
  });

  testWidgets('a port in range is used, and an empty field goes back to any', (
    WidgetTester tester,
  ) async {
    linkedDevice(enabled: true);
    final _Block block = _Block();
    await block.pump(tester);

    await _savePort(tester, '9000');

    expect(block.binder.asked.last, 9000);
    expect(
      find.text(Strings.playOnListening(<String>['192.168.1.20'], 9000)),
      findsOneWidget,
    );
    expect(find.text(Strings.playOnPortInvalid), findsNothing);

    await _savePort(tester, '');

    expect(block.binder.asked.last, 0);
  });

  testWidgets('the field shows the saved port', (WidgetTester tester) async {
    linkedDevice(enabled: true, port: 8765);
    await _Block().pump(tester);

    expect(find.widgetWithText(TextField, '8765'), findsOneWidget);
  });

  testWidgets('a field submitted from the keyboard saves too', (
    WidgetTester tester,
  ) async {
    linkedDevice(enabled: true);
    final _Block block = _Block();
    await block.pump(tester);

    await tester.enterText(find.byType(TextField), '9100');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();

    expect(block.binder.asked.last, 9100);
  });

  testWidgets('a fixed port that cannot be used names it', (
    WidgetTester tester,
  ) async {
    linkedDevice(enabled: true, port: 8765);
    final _Block block = _Block();
    block.binder.error = StateError('in use');
    await block.pump(tester);

    expect(find.text(Strings.playOnBindFailed(8765)), findsOneWidget);
  });

  testWidgets('any port that cannot be used says so', (
    WidgetTester tester,
  ) async {
    linkedDevice(enabled: true);
    final _Block block = _Block();
    block.binder.error = StateError('no');
    await block.pump(tester);

    expect(find.text(Strings.playOnBindFailed(null)), findsOneWidget);
  });

  testWidgets('no local network says so', (WidgetTester tester) async {
    linkedDevice(enabled: true);
    await _Block(addresses: const <InterfaceAddress>[]).pump(tester);

    expect(find.text(Strings.playOnNoNetwork), findsOneWidget);
  });

  testWidgets('a report the server refuses says so', (
    WidgetTester tester,
  ) async {
    linkedDevice(enabled: true);
    final _Block block = _Block();
    block.server.reportStatus = 500;
    await block.pump(tester);

    expect(find.text(Strings.playOnReportFailed), findsOneWidget);
  });
}
