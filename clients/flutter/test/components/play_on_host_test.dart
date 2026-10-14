import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/play_on_host.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/play_on_receiver.dart';

import '../support/play_on_target.dart';

class _Pushes extends NavigatorObserver {
  final List<String> names = <String>[];

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) {
    final String? name = route.settings.name;
    if (name != null) {
      names.add(name);
    }
  }
}

Map<String, dynamic> _detail(String type, String id) => <String, dynamic>{
  'id': 'v1',
  'title': <String, dynamic>{'type': type, 'id': id},
  'library_id': 'l1',
  'quality': 'fhd',
  'container': 'mkv',
};

class _Host {
  final TargetServer server = TargetServer();
  final FakeBinder binder = FakeBinder();
  final _Pushes pushes = _Pushes();
  final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
  Map<String, dynamic>? detail = _detail('movie', 'm1');
  Completer<void>? slowLookup;

  Future<void> pump(WidgetTester tester, {Widget? page}) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        final http.Response? target = server.answer(req);
        if (target != null) {
          return target;
        }
        if (req.url.path == '/api/v1/versions/v2') {
          return http.Response(jsonEncode(_detail('movie', 'm2')), 200);
        }
        final Map<String, dynamic>? found = detail;
        if (req.url.path == '/api/v1/versions/v1' && found != null) {
          await slowLookup?.future;
          return http.Response(jsonEncode(found), 200);
        }
        return http.Response('', 404);
      }),
    );
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        navigatorKey: navigator,
        navigatorObservers: <NavigatorObserver>[pushes],
        builder: (BuildContext context, Widget? child) => ToastHost(
          child: PlayOnHost(
            api: api,
            navigator: navigator,
            bind: binder.call,
            addresses: () async => kWifi,
            child: child!,
          ),
        ),
        onGenerateRoute: (RouteSettings s) => MaterialPageRoute<void>(
          settings: s,
          builder: (BuildContext _) =>
              Scaffold(body: page ?? Text('at ${s.name}')),
        ),
      ),
    );
    await tester.pumpAndSettle();
    pushes.names.clear();
  }

  Future<int> send(WidgetTester tester) async {
    final Future<int> status = binder.last.handler('tok');
    await tester.pumpAndSettle();
    return status;
  }
}

void main() {
  setUp(() => linkedDevice(enabled: true));

  testWidgets('a movie command opens home, the movie, then the player', (
    WidgetTester tester,
  ) async {
    final _Host host = _Host();
    await host.pump(tester);

    expect(await host.send(tester), 202);
    await tester.pumpAndSettle();

    expect(host.pushes.names, <String>[
      '/home',
      '/title?type=movie&id=m1',
      '/watch?version=v1',
    ]);
    expect(find.text('at /watch?version=v1'), findsOneWidget);
  });

  testWidgets('an episode command opens its episode page under the player', (
    WidgetTester tester,
  ) async {
    final _Host host = _Host()..detail = _detail('episode', 'e1');
    await host.pump(tester);

    await host.send(tester);
    await tester.pumpAndSettle();

    expect(host.pushes.names, <String>[
      '/home',
      '/episode?id=e1',
      '/watch?version=v1',
    ]);
  });

  testWidgets('a newer command wins over one still being looked up', (
    WidgetTester tester,
  ) async {
    final _Host host = _Host()..slowLookup = Completer<void>();
    host.server.versionFor = (String token) => token == 'old' ? 'v1' : 'v2';
    await host.pump(tester);

    final Future<int> older = host.binder.last.handler('old');
    await tester.pump();
    final Future<int> newer = host.binder.last.handler('new');
    await tester.pumpAndSettle();
    host.slowLookup!.complete();
    await tester.pumpAndSettle();

    expect(<int>[await older, await newer], <int>[202, 202]);
    expect(host.pushes.names, <String>[
      '/home',
      '/title?type=movie&id=m2',
      '/watch?version=v2',
    ]);
    expect(find.text(Strings.playOnCommandRefused), findsNothing);
  });

  testWidgets('a refused command says so on this device', (
    WidgetTester tester,
  ) async {
    final _Host host = _Host();
    host.server.redeemStatus = 403;
    await host.pump(tester);

    expect(await host.send(tester), 403);

    expect(host.pushes.names, isEmpty);
    expect(find.text(Strings.playOnCommandRefused), findsOneWidget);
  });

  testWidgets('a version that cannot be read says so on this device', (
    WidgetTester tester,
  ) async {
    final _Host host = _Host()..detail = null;
    await host.pump(tester);

    expect(await host.send(tester), 202);
    await tester.pumpAndSettle();

    expect(host.pushes.names, isEmpty);
    expect(find.text(Strings.playOnCommandRefused), findsOneWidget);
  });

  testWidgets('the background stops listening and the foreground restarts', (
    WidgetTester tester,
  ) async {
    final _Host host = _Host();
    await host.pump(tester);
    expect(host.binder.made, hasLength(1));

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pumpAndSettle();

    expect(host.binder.last.closed, isTrue);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();

    expect(host.binder.made, hasLength(2));
    expect(host.binder.last.closed, isFalse);
  });

  testWidgets('pages below reach the receiver through the scope', (
    WidgetTester tester,
  ) async {
    PlayOnReceiver? found;
    final _Host host = _Host();
    await host.pump(
      tester,
      page: Builder(
        builder: (BuildContext context) {
          found = PlayOnScope.maybeOf(context);
          return const SizedBox.shrink();
        },
      ),
    );

    expect(found, isNotNull);
    expect(found!.status, PlayOnStatus.listening);
  });

  testWidgets('leaving the app closes the listener', (
    WidgetTester tester,
  ) async {
    final _Host host = _Host();
    await host.pump(tester);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();

    expect(host.binder.last.closed, isTrue);
  });
}
