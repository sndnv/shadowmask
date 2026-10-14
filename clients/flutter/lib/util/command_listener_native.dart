import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:shadowmask/util/command_listener_types.dart';

const int kCommandBodyLimit = 8192;
const Duration kCommandReadTimeout = Duration(seconds: 5);

Future<CommandListener> bindCommandListener(
  int port,
  CommandHandler handler, {
  InternetAddress? address,
  Duration readTimeout = kCommandReadTimeout,
}) async {
  final HttpServer server = await HttpServer.bind(
    address ?? InternetAddress.anyIPv4,
    port,
  );
  server.listen(
    (HttpRequest req) => unawaited(_serve(req, handler, readTimeout)),
  );
  return _HttpCommandListener(server);
}

Future<List<InterfaceAddress>> interfaceAddresses() async => <InterfaceAddress>[
  for (final NetworkInterface i in await NetworkInterface.list(
    type: InternetAddressType.IPv4,
  ))
    for (final InternetAddress a in i.addresses)
      (name: i.name, address: a.address, octets: a.rawAddress),
];

class _HttpCommandListener implements CommandListener {
  _HttpCommandListener(this._server);

  final HttpServer _server;

  @override
  int get port => _server.port;

  @override
  Future<void> close() => _server.close(force: true);
}

Future<void> _serve(
  HttpRequest req,
  CommandHandler handler,
  Duration readTimeout,
) async {
  try {
    req.response.statusCode = await _status(req, handler, readTimeout);
    await req.response.close();
  } catch (_) {
    await req.response.close().catchError((Object _) {});
  }
}

Future<int> _status(
  HttpRequest req,
  CommandHandler handler,
  Duration readTimeout,
) async {
  if (req.uri.path != '/play') {
    return HttpStatus.notFound;
  }
  if (req.method != 'POST') {
    return HttpStatus.methodNotAllowed;
  }
  final Uint8List? body;
  try {
    body = await _body(req).timeout(readTimeout);
  } on TimeoutException {
    return HttpStatus.requestTimeout;
  }
  if (body == null) {
    return HttpStatus.requestEntityTooLarge;
  }
  final String? token = _token(body);
  return token == null ? HttpStatus.badRequest : handler(token);
}

Future<Uint8List?> _body(HttpRequest req) async {
  final BytesBuilder bytes = BytesBuilder(copy: false);
  await for (final Uint8List chunk in req) {
    bytes.add(chunk);
    if (bytes.length > kCommandBodyLimit) {
      return null;
    }
  }
  return bytes.takeBytes();
}

String? _token(Uint8List body) {
  try {
    final Object? json = jsonDecode(utf8.decode(body));
    final Object? token = json is Map<String, dynamic> ? json['token'] : null;
    return token is String && token.isNotEmpty ? token : null;
  } on FormatException {
    return null;
  }
}
