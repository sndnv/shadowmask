import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:shadowmask/util/command_listener_types.dart';
import 'package:shared_preferences/shared_preferences.dart';

const List<InterfaceAddress> kWifi = <InterfaceAddress>[
  (name: 'wlan0', address: '192.168.1.20', octets: <int>[192, 168, 1, 20]),
];

class FakeListener implements CommandListener {
  FakeListener(this.port, this.handler);

  @override
  final int port;
  final CommandHandler handler;
  bool closed = false;

  @override
  Future<void> close() async => closed = true;
}

class FakeBinder {
  final List<int> asked = <int>[];
  final List<FakeListener> made = <FakeListener>[];
  Object? error;

  FakeListener get last => made.last;

  Future<CommandListener> call(int port, CommandHandler handler) async {
    asked.add(port);
    final Object? failure = error;
    if (failure != null) {
      throw failure;
    }
    final FakeListener listener = FakeListener(
      port == 0 ? 41000 + made.length : port,
      handler,
    );
    made.add(listener);
    return listener;
  }
}

class TargetServer {
  TargetServer({
    this.reportStatus = 204,
    this.clearStatus = 204,
    this.redeemStatus = 200,
  });

  int reportStatus;
  int clearStatus;
  int redeemStatus;
  String Function(String token)? versionFor;
  final List<http.Request> requests = <http.Request>[];

  List<Map<String, dynamic>> get reports => <Map<String, dynamic>>[
    for (final http.Request r in requests)
      if (r.method == 'PUT' && r.url.path == '/api/v1/devices/self/target')
        jsonDecode(r.body) as Map<String, dynamic>,
  ];

  int get clears => requests
      .where(
        (http.Request r) =>
            r.method == 'DELETE' && r.url.path == '/api/v1/devices/self/target',
      )
      .length;

  List<String> get redeemed => <String>[
    for (final http.Request r in requests)
      if (r.url.path == '/api/v1/devices/self/commands/redeem')
        (jsonDecode(r.body) as Map<String, dynamic>)['token'] as String,
  ];

  http.Response? answer(http.Request req) {
    if (req.url.path == '/api/v1/devices/self/target') {
      requests.add(req);
      return http.Response(
        '',
        req.method == 'PUT' ? reportStatus : clearStatus,
      );
    }
    if (req.url.path == '/api/v1/devices/self/commands/redeem') {
      requests.add(req);
      final String token =
          (jsonDecode(req.body) as Map<String, dynamic>)['token'] as String;
      return redeemStatus == 200
          ? http.Response(
              jsonEncode(<String, String>{
                'version_id': versionFor?.call(token) ?? 'v1',
              }),
              200,
            )
          : http.Response('', redeemStatus);
    }
    return null;
  }
}

void linkedDevice({bool enabled = false, int? port}) {
  SharedPreferences.setMockInitialValues(<String, Object>{
    'shadowmask.tokens': jsonEncode(<String, String>{
      'access_token': 'smk_device',
      'device_id': 'dev-7',
    }),
    'shadowmask.playOn.enabled': enabled,
    'shadowmask.playOn.port': ?port,
  });
}
