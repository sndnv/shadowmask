import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/play_on_store.dart';
import 'package:shadowmask/util/command_listener_types.dart';
import 'package:shadowmask/view/play_on_receiver.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../support/play_on_target.dart';

class _Harness {
  _Harness({
    List<InterfaceAddress> addresses = kWifi,
    Future<List<InterfaceAddress>> Function()? lister,
    ListenerBinder? bind,
    Duration retryDelay = const Duration(days: 1),
  }) {
    api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        if (req.url.path == '/api/v1/auth/login') {
          return http.Response(
            jsonEncode(<String, String>{
              'access_token': 'a',
              'refresh_token': 'r',
            }),
            200,
          );
        }
        if (req.url.path.endsWith('/redeem')) {
          await redeemGate?.future;
        }
        return server.answer(req) ?? http.Response('', 404);
      }),
    );
    receiver = PlayOnReceiver(
      api: api,
      onPlay: (String version) async => played.add(version),
      onRefused: () => refused += 1,
      retryDelay: retryDelay,
      bind: bind ?? binder.call,
      addresses: lister ?? () async => addresses,
    );
  }

  final TargetServer server = TargetServer();
  final FakeBinder binder = FakeBinder();
  final List<String> played = <String>[];
  int refused = 0;
  Completer<void>? redeemGate;
  late final ApiClient api;
  late final PlayOnReceiver receiver;
}

Future<void> _forgetTokens() async =>
    (await SharedPreferences.getInstance()).remove('shadowmask.tokens');

Future<void> _settle() =>
    Future<void>.delayed(const Duration(milliseconds: 80));

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('an unlinked device starts nothing and turns the setting off', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.playOn.enabled': true,
    });
    final _Harness h = _Harness();

    await h.receiver.load();

    expect(h.receiver.linked, isFalse);
    expect(h.receiver.enabled, isFalse);
    expect((await const PlayOnStore().load()).enabled, isFalse);
    expect(h.receiver.status, PlayOnStatus.off);
    expect(h.binder.asked, isEmpty);
  });

  test('off by default on a linked device', () async {
    linkedDevice();
    final _Harness h = _Harness();

    await h.receiver.load();

    expect(h.receiver.loaded, isTrue);
    expect(h.receiver.linked, isTrue);
    expect(h.receiver.enabled, isFalse);
    expect(h.binder.asked, isEmpty);
  });

  test('turning it on listens on any port and reports it', () async {
    linkedDevice();
    final _Harness h = _Harness();
    await h.receiver.load();
    int notified = 0;
    h.receiver.addListener(() => notified += 1);

    await h.receiver.setEnabled(true);

    expect(h.binder.asked, <int>[0]);
    expect(h.server.reports, <Map<String, dynamic>>[
      <String, dynamic>{
        'kind': 'android',
        'addresses': <String>['192.168.1.20'],
        'port': 41000,
      },
    ]);
    expect(h.receiver.status, PlayOnStatus.listening);
    expect(h.receiver.listeningOn, <String>['192.168.1.20']);
    expect(h.receiver.boundPort, 41000);
    expect(notified, greaterThan(0));
    expect((await const PlayOnStore().load()).enabled, isTrue);
  });

  test('the setting and the port survive a restart', () async {
    linkedDevice(enabled: true, port: 8765);
    final _Harness h = _Harness();

    await h.receiver.load();

    expect(h.binder.asked, <int>[8765]);
    expect(h.receiver.port, 8765);
    expect(h.server.reports.single['port'], 8765);
  });

  test('turning it off stops listening and deletes the report', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();

    await h.receiver.setEnabled(false);

    expect(h.binder.last.closed, isTrue);
    expect(h.server.clears, 1);
    expect(h.receiver.status, PlayOnStatus.off);
    expect((await const PlayOnStore().load()).enabled, isFalse);
  });

  test('turning it off unlinked deletes nothing', () async {
    final _Harness h = _Harness();
    await h.receiver.load();

    await h.receiver.setEnabled(false);

    expect(h.server.clears, 0);
  });

  test(
    'the background stops it and the foreground restarts and reports',
    () async {
      linkedDevice(enabled: true);
      final _Harness h = _Harness();
      await h.receiver.load();

      await h.receiver.pause();

      expect(h.binder.last.closed, isTrue);
      expect(h.binder.made, hasLength(1));

      await h.receiver.resume();

      expect(h.binder.made, hasLength(2));
      expect(h.binder.last.closed, isFalse);
      expect(h.server.reports, hasLength(2));
    },
  );

  test('a refresh after sign-out stops listening', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.playOn.enabled': true,
    });

    await h.receiver.refresh();

    expect(h.receiver.linked, isFalse);
    expect(h.binder.last.closed, isTrue);
    expect(h.receiver.status, PlayOnStatus.off);
    expect((await const PlayOnStore().load()).enabled, isFalse);
  });

  test('signing out through the client stops it at once', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();

    await h.api.logout();
    await pumpEventQueue();

    expect(h.receiver.linked, isFalse);
    expect(h.receiver.enabled, isFalse);
    expect(h.binder.last.closed, isTrue);
  });

  test('a password login after signing out binds nothing', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();

    await h.api.logout();
    await h.api.login('ana', 'secret');
    await pumpEventQueue();
    await h.receiver.resume();

    expect(h.binder.made, hasLength(1));
    expect(h.binder.last.closed, isTrue);
    expect(h.receiver.status, PlayOnStatus.off);
  });

  test('a password login with the setting still on binds nothing', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.tokens': jsonEncode(<String, String>{
        'access_token': 'a',
        'refresh_token': 'r',
      }),
      'shadowmask.playOn.enabled': true,
    });
    final _Harness h = _Harness();

    await h.receiver.load();
    await h.receiver.setEnabled(true);

    expect(h.binder.asked, isEmpty);
    expect(h.server.reports, isEmpty);
    expect(h.receiver.enabled, isFalse);
  });

  test('signed out while the addresses are listed binds nothing', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness(
      lister: () async {
        await _forgetTokens();
        return kWifi;
      },
    );

    await h.receiver.load();

    expect(h.binder.asked, isEmpty);
    expect(h.server.reports, isEmpty);
    expect(h.receiver.status, PlayOnStatus.off);
  });

  test('signed out while binding closes it and reports nothing', () async {
    linkedDevice(enabled: true);
    late final _Harness h;
    h = _Harness(
      bind: (int port, CommandHandler handler) async {
        final CommandListener listener = await h.binder.call(port, handler);
        await _forgetTokens();
        return listener;
      },
    );

    await h.receiver.load();

    expect(h.binder.made, hasLength(1));
    expect(h.binder.last.closed, isTrue);
    expect(h.server.reports, isEmpty);
    expect(h.receiver.status, PlayOnStatus.off);
  });

  test('a new port restarts on it, and no port goes back to any', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();

    await h.receiver.setPort(9000);

    expect(h.binder.asked, <int>[0, 9000]);
    expect(h.binder.made.first.closed, isTrue);
    expect(h.server.reports.last['port'], 9000);
    expect((await const PlayOnStore().load()).port, 9000);

    await h.receiver.setPort(null);

    expect(h.binder.asked.last, 0);
    expect((await const PlayOnStore().load()).port, isNull);
  });

  test('a port that cannot be bound says so and reports nothing', () async {
    linkedDevice(enabled: true, port: 8765);
    final _Harness h = _Harness();
    h.binder.error = StateError('in use');

    await h.receiver.load();

    expect(h.receiver.status, PlayOnStatus.bindFailed);
    expect(h.server.reports, isEmpty);
  });

  test('no private address says so and binds nothing', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness(
      addresses: <InterfaceAddress>[
        (name: 'rmnet0', address: '8.8.4.4', octets: <int>[8, 8, 4, 4]),
      ],
    );

    await h.receiver.load();

    expect(h.receiver.status, PlayOnStatus.noNetwork);
    expect(h.binder.asked, isEmpty);
  });

  test('an address list that fails counts as no network', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness(
      lister: () => Future<List<InterfaceAddress>>.error(StateError('no')),
    );

    await h.receiver.load();

    expect(h.receiver.status, PlayOnStatus.noNetwork);
  });

  test('a report the server refuses closes the listener', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    h.server.reportStatus = 500;

    await h.receiver.load();

    expect(h.receiver.status, PlayOnStatus.reportFailed);
    expect(h.binder.last.closed, isTrue);
  });

  test('a command is redeemed, answered and played', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();

    final int status = await h.binder.last.handler('tok');
    await pumpEventQueue();

    expect(status, 202);
    expect(h.server.redeemed, <String>['tok']);
    expect(h.played, <String>['v1']);
    expect(h.refused, 0);
  });

  for (final (int redeem, int answer) in <(int, int)>[
    (403, 403),
    (401, 401),
    (500, 502),
  ]) {
    test(
      'a redeem answered [$redeem] is answered [$answer] and shown',
      () async {
        linkedDevice(enabled: true);
        final _Harness h = _Harness();
        await h.receiver.load();
        h.server.redeemStatus = redeem;

        final int status = await h.binder.last.handler('tok');

        expect(status, answer);
        expect(h.played, isEmpty);
        expect(h.refused, 1);
      },
    );
  }

  test('a command once signed out is refused without a redeem', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();
    await _forgetTokens();

    final int status = await h.binder.last.handler('tok');

    expect(status, 403);
    expect(h.server.redeemed, isEmpty);
    expect(h.played, isEmpty);
    expect(h.refused, 0);
  });

  test('a redeem that ends after a pause plays nothing', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();
    h.redeemGate = Completer<void>();

    final Future<int> answer = h.binder.last.handler('tok');
    await pumpEventQueue();
    await h.receiver.pause();
    h.redeemGate!.complete();

    expect(await answer, 403);
    expect(h.server.redeemed, <String>['tok']);
    expect(h.played, isEmpty);
    expect(h.refused, 0);
  });

  test('the same token twice is redeemed and played once', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();
    h.redeemGate = Completer<void>();

    final Future<int> first = h.binder.last.handler('tok');
    final Future<int> second = h.binder.last.handler('tok');
    h.redeemGate!.complete();
    final int later = await h.binder.last.handler('tok');
    await pumpEventQueue();

    expect(<int>[await first, await second, later], <int>[202, 202, 202]);
    expect(h.server.redeemed, <String>['tok']);
    expect(h.played, <String>['v1']);
  });

  test('only the last few tokens are remembered', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();

    for (int i = 0; i <= kRememberedCommands; i++) {
      await h.binder.last.handler('tok-$i');
    }
    await h.binder.last.handler('tok-0');
    await h.binder.last.handler('tok-$kRememberedCommands');

    expect(h.server.redeemed, hasLength(kRememberedCommands + 2));
    expect(h.server.redeemed.last, 'tok-0');
  });

  test('no network at first is tried again until it listens', () async {
    linkedDevice(enabled: true);
    int listed = 0;
    final _Harness h = _Harness(
      retryDelay: const Duration(milliseconds: 5),
      lister: () async => listed++ == 0 ? <InterfaceAddress>[] : kWifi,
    );

    await h.receiver.load();
    expect(h.receiver.status, PlayOnStatus.noNetwork);
    await _settle();

    expect(listed, 2);
    expect(h.receiver.status, PlayOnStatus.listening);
    expect(h.server.reports, hasLength(1));
  });

  test('a report that keeps failing is tried three more times', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness(retryDelay: const Duration(milliseconds: 5));
    h.server.reportStatus = 500;

    await h.receiver.load();
    await _settle();

    expect(h.server.reports, hasLength(1 + kTargetRetries));
    expect(h.receiver.status, PlayOnStatus.reportFailed);
    expect(h.binder.made.every((FakeListener l) => l.closed), isTrue);
  });

  test('a pause cancels a retry that is waiting', () async {
    linkedDevice(enabled: true);
    int listed = 0;
    final _Harness h = _Harness(
      retryDelay: const Duration(milliseconds: 20),
      lister: () async {
        listed += 1;
        return <InterfaceAddress>[];
      },
    );

    await h.receiver.load();
    await h.receiver.pause();
    await _settle();

    expect(listed, 1);
  });

  test('a report that cannot be deleted is deleted on a retry', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness(retryDelay: const Duration(milliseconds: 5));
    await h.receiver.load();
    h.server.clearStatus = 500;

    await h.receiver.setEnabled(false);
    expect(h.server.clears, 1);
    h.server.clearStatus = 204;
    await _settle();

    expect(h.server.clears, 2);
  });

  test('disposing closes the listener', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();
    await h.receiver.load();

    h.receiver.dispose();
    await pumpEventQueue();

    expect(h.binder.last.closed, isTrue);
  });

  test('a start still queued when disposed binds nothing', () async {
    linkedDevice(enabled: true);
    final _Harness h = _Harness();

    unawaited(h.receiver.load());
    h.receiver.dispose();
    await pumpEventQueue();

    expect(h.binder.asked, isEmpty);
    expect(h.receiver.status, PlayOnStatus.off);
  });
}
