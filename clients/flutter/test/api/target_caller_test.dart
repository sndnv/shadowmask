import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/target_caller.dart';
import 'package:shadowmask/model/auth/device_target.dart';
import 'package:shadowmask/model/auth/play_command.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';

const String _running = '''
<?xml version="1.0"?>
<active-app>
  <app id="dev" type="appl" version="0.0.5" ui-location="dev">Shadowmask</app>
</active-app>
''';

const String _home = '''
<?xml version="1.0" encoding="UTF-8" ?>
<active-app>
  <app>Roku</app>
</active-app>
''';

const DeviceTarget _roku = DeviceTarget(
  kind: TargetKind.roku,
  addresses: <String>['10.0.0.2', '10.0.0.3'],
  port: 8060,
  appId: 'dev',
);

const DeviceTarget _android = DeviceTarget(
  kind: TargetKind.android,
  addresses: <String>['10.0.0.5', '10.0.0.6'],
  port: 7777,
);

PlayCommand _command(DeviceTarget target) => PlayCommand(
  token: 'tok+en/1=',
  expiresAt: '2026-10-14T18:03:11Z',
  target: target,
);

Version _version(TitleKind kind) => Version(
  id: 'v1',
  title: TitleRef(type: kind, id: 't1'),
  libraryId: 'l1',
  quality: Quality.fhd,
  container: 'mkv',
);

class _Lan {
  _Lan(this.answer);

  final Future<http.Response> Function(http.Request req) answer;
  final List<http.Request> requests = <http.Request>[];

  TargetCaller caller({Duration play = const Duration(milliseconds: 50)}) =>
      TargetCaller(
        client: MockClient((http.Request req) {
          requests.add(req);
          return answer(req);
        }),
        ecpTimeout: const Duration(milliseconds: 50),
        playTimeout: play,
      );

  List<String> get calls => <String>[
    for (final http.Request r in requests)
      '${r.method} ${r.url.host}${r.url.path}',
  ];
}

Future<http.Response> _never() => Completer<http.Response>().future;

Future<http.Response> _ok(String body) async => http.Response(body, 200);

Future<http.Response> _status(int status) async => http.Response('', status);

Future<http.Response> _roku200(http.Request req, String app) =>
    req.method == 'GET' ? _ok(app) : _status(200);

void main() {
  group('runningAppId', () {
    test('reads the id of the running app', () {
      expect(runningAppId(_running), 'dev');
    });

    test('reads an id that follows other attributes, on one line', () {
      expect(
        runningAppId(
          '<active-app><app type="appl" id="12345" version="2">Other</app></active-app>',
        ),
        '12345',
      );
    });

    test('the home screen has no id', () {
      expect(runningAppId(_home), isNull);
    });

    test('a garbled or empty answer has no id', () {
      expect(runningAppId('<active-app><app id=dev>'), isNull);
      expect(runningAppId('not xml'), isNull);
      expect(runningAppId(''), isNull);
    });

    test('an attribute that only ends in id is not the id', () {
      expect(runningAppId('<app appid="dev">x</app>'), isNull);
    });
  });

  group('Roku', () {
    test('a running channel gets the command as input', () async {
      final _Lan lan = _Lan((http.Request req) => _roku200(req, _running));

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku),
        _version(TitleKind.episode),
      );

      expect(outcome, PlayOnOutcome.playing);
      expect(lan.calls, <String>[
        'GET 10.0.0.2/query/active-app',
        'POST 10.0.0.2/input',
      ]);
      expect(lan.requests.last.url.port, 8060);
      expect(lan.requests.last.url.queryParameters, <String, String>{
        'contentId': 'v1',
        'mediaType': 'episode',
        'command': 'tok+en/1=',
      });
    });

    test('the home screen gets a launch of the channel', () async {
      final _Lan lan = _Lan((http.Request req) => _roku200(req, _home));

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.playing);
      expect(lan.calls.last, 'POST 10.0.0.2/launch/dev');
      expect(lan.requests.last.url.queryParameters['mediaType'], 'movie');
      expect(lan.requests.last.url.queryParameters['command'], 'tok+en/1=');
    });

    test('another running channel gets a launch of ours', () async {
      final _Lan lan = _Lan(
        (http.Request req) =>
            _roku200(req, '<active-app><app id="12">Other</app></active-app>'),
      );

      await lan.caller().send(_command(_roku), _version(TitleKind.movie));

      expect(lan.calls.last, 'POST 10.0.0.2/launch/dev');
    });

    test('an answer it cannot read gets a launch', () async {
      final _Lan lan = _Lan((http.Request req) => _roku200(req, '<<<'));

      await lan.caller().send(_command(_roku), _version(TitleKind.movie));

      expect(lan.calls.last, 'POST 10.0.0.2/launch/dev');
    });

    test('a silent first address moves on to the next one', () async {
      final _Lan lan = _Lan(
        (http.Request req) =>
            req.url.host == '10.0.0.2' ? _never() : _roku200(req, _running),
      );

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.playing);
      expect(lan.calls, <String>[
        'GET 10.0.0.2/query/active-app',
        'GET 10.0.0.3/query/active-app',
        'POST 10.0.0.3/input',
      ]);
    });

    test('a failed connection moves on to the next address', () async {
      final _Lan lan = _Lan(
        (http.Request req) => req.url.host == '10.0.0.2'
            ? Future<http.Response>.error(http.ClientException('refused'))
            : _roku200(req, _home),
      );

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.playing);
      expect(lan.calls.last, 'POST 10.0.0.3/launch/dev');
    });

    test('a launch that fails moves on to the next address', () async {
      final _Lan lan = _Lan(
        (http.Request req) => req.method == 'GET'
            ? _ok(_home)
            : _status(req.url.host == '10.0.0.2' ? 500 : 200),
      );

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.playing);
      expect(lan.calls.last, 'POST 10.0.0.3/launch/dev');
    });

    test('a refused launch stops and tries no other address', () async {
      final _Lan lan = _Lan(
        (http.Request req) => req.method == 'GET' ? _ok(_home) : _status(403),
      );

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.refused);
      expect(lan.calls, <String>[
        'GET 10.0.0.2/query/active-app',
        'POST 10.0.0.2/launch/dev',
      ]);
    });

    test('a refused query stops before any command', () async {
      final _Lan lan = _Lan((http.Request req) => _status(403));

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.refused);
      expect(lan.calls, <String>['GET 10.0.0.2/query/active-app']);
    });

    test('no answer from any address is unreachable', () async {
      final _Lan lan = _Lan((http.Request req) => _never());

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.unreachable);
      expect(lan.calls, <String>[
        'GET 10.0.0.2/query/active-app',
        'GET 10.0.0.3/query/active-app',
      ]);
    });

    test('a Roku report without a channel id is unreachable', () async {
      final _Lan lan = _Lan((http.Request req) => _roku200(req, _running));

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_roku.copyWith(appId: null)),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.unreachable);
      expect(lan.requests, isEmpty);
    });
  });

  group('Android', () {
    test('posts the token to the listener', () async {
      final _Lan lan = _Lan((http.Request req) => _status(202));

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_android),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.playing);
      expect(lan.calls, <String>['POST 10.0.0.5/play']);
      expect(lan.requests.single.url.port, 7777);
      expect(
        lan.requests.single.headers['Content-Type'],
        startsWith('application/json'),
      );
      expect(jsonDecode(lan.requests.single.body), <String, String>{
        'token': 'tok+en/1=',
      });
    });

    for (final int status in <int>[401, 403]) {
      test('a [$status] is refused and tries no other address', () async {
        final _Lan lan = _Lan((http.Request req) => _status(status));

        final PlayOnOutcome outcome = await lan.caller().send(
          _command(_android),
          _version(TitleKind.movie),
        );

        expect(outcome, PlayOnOutcome.refused);
        expect(lan.calls, <String>['POST 10.0.0.5/play']);
      });
    }

    test('an error or silence moves on to the next address', () async {
      final _Lan lan = _Lan(
        (http.Request req) =>
            req.url.host == '10.0.0.5' ? _status(500) : _never(),
      );

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_android),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.unreachable);
      expect(lan.calls, <String>['POST 10.0.0.5/play', 'POST 10.0.0.6/play']);
    });

    test('the phone gets longer than a Roku to answer', () async {
      final _Lan lan = _Lan(
        (http.Request req) => Future<http.Response>.delayed(
          const Duration(milliseconds: 150),
          () => http.Response('', 202),
        ),
      );

      final PlayOnOutcome outcome = await lan
          .caller(play: const Duration(seconds: 2))
          .send(_command(_android), _version(TitleKind.movie));

      expect(outcome, PlayOnOutcome.playing);
      expect(lan.calls, <String>['POST 10.0.0.5/play']);
    });
  });

  test(
    'a kind this app does not know is unreachable and calls nothing',
    () async {
      final _Lan lan = _Lan((http.Request req) => _status(202));

      final PlayOnOutcome outcome = await lan.caller().send(
        _command(_android.copyWith(kind: null)),
        _version(TitleKind.movie),
      );

      expect(outcome, PlayOnOutcome.unreachable);
      expect(lan.requests, isEmpty);
    },
  );

  test('the default caller gives a Roku 2 seconds and a phone 10', () {
    final TargetCaller caller = TargetCaller();

    expect(caller.ecpTimeout, const Duration(seconds: 2));
    expect(caller.playTimeout, const Duration(seconds: 10));
  });
}
