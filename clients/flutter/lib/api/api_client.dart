import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:http/http.dart' as http;
import 'package:uuid/uuid.dart';

import 'package:shadowmask/model/auth/auth_tokens.dart';
import 'package:shadowmask/model/auth/issued_token.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/api/authentication_failure.dart';
import 'package:shadowmask/api/authorization_failure.dart';
import 'package:shadowmask/api/token_store.dart';

class ApiClient {
  ApiClient({
    String? baseUrl,
    http.Client? httpClient,
    TokenStore? tokenStore,
    String Function()? newPlayerId,
  }) : baseUrl = baseUrl ?? _defaultBaseUrl,
       _http = httpClient ?? http.Client(),
       _store = tokenStore ?? TokenStore(),
       _newPlayerId = newPlayerId ?? const Uuid().v4;

  static const String _defaultBaseUrl = String.fromEnvironment(
    'SHADOWMASK_API_BASE',
    defaultValue: 'http://localhost:8080',
  );

  static const Set<int> _refusedRefresh = <int>{400, 401, 403};

  final String baseUrl;
  final http.Client _http;
  final TokenStore _store;
  final String Function() _newPlayerId;
  final StreamController<void> _sessionChanged =
      StreamController<void>.broadcast();
  Future<SelfUser>? _self;
  Future<String?>? _renewing;
  String? _unsavedPlayerId;
  int? childAge;

  Stream<void> get sessionChanged => _sessionChanged.stream;

  Future<void> _saveTokens(AuthTokens tokens) async {
    await _store.save(tokens);
    _sessionChanged.add(null);
  }

  Future<void> _clearTokens() async {
    _unsavedPlayerId = null;
    await _store.clear();
    _sessionChanged.add(null);
  }

  String _playerIdOf(AuthTokens? current) {
    if (current != null && current.playerId.isNotEmpty) {
      return current.playerId;
    }
    return _unsavedPlayerId ??= _newPlayerId();
  }

  Future<String> playerId() async {
    final AuthTokens? stored = await _store.load();
    if (stored == null) {
      return '';
    }
    return stored.deviceId.isNotEmpty ? stored.deviceId : _playerIdOf(stored);
  }

  Uri _uri(String path) => Uri.parse('$baseUrl$path');

  Future<http.Response> _raw(
    String method,
    String path, {
    Object? body,
    String? token,
  }) {
    final Map<String, String> headers = <String, String>{
      'Accept': 'application/json',
    };
    if (body != null) {
      headers['Content-Type'] = 'application/json';
    }
    if (token != null) {
      headers['Authorization'] = 'Bearer $token';
    }
    final int? age = childAge;
    if (age != null) {
      headers['Prefer'] = 'safe; age=$age';
    }
    final String? encoded = body == null ? null : jsonEncode(body);
    final Uri uri = _uri(path);
    switch (method) {
      case 'GET':
        return _http.get(uri, headers: headers);
      case 'POST':
        return _http.post(uri, headers: headers, body: encoded);
      case 'PUT':
        return _http.put(uri, headers: headers, body: encoded);
      case 'DELETE':
        return _http.delete(uri, headers: headers, body: encoded);
      default:
        throw ArgumentError('unsupported method $method');
    }
  }

  Future<http.Response> _send(
    String method,
    String path, {
    Object? body,
  }) async {
    final AuthTokens? stored = await _store.load();
    http.Response res = await _raw(
      method,
      path,
      body: body,
      token: stored?.accessToken,
    );
    if (res.statusCode == 401 && stored != null && stored.canRefresh) {
      final String? refreshed = await _renew(stored);
      if (refreshed != null) {
        res = await _raw(method, path, body: body, token: refreshed);
      }
    }
    if (res.statusCode == 401) {
      await _clearTokens();
      throw AuthenticationFailure();
    }
    if (res.statusCode == 403) {
      throw AuthorizationFailure();
    }
    if (res.statusCode >= 400) {
      throw _errorFrom(res);
    }
    if (method != 'GET') {
      forgetUser();
    }
    return res;
  }

  Future<String?> _renew(AuthTokens used) =>
      _renewing ??= _refresh(used).whenComplete(() => _renewing = null);

  Future<String?> _refresh(AuthTokens used) async {
    final AuthTokens? current = await _store.load();
    if (current == null) {
      return null;
    }
    if (current.accessToken != used.accessToken) {
      return current.accessToken;
    }
    final http.Response res = await _raw(
      'POST',
      '/api/v1/auth/refresh',
      body: <String, String>{'refresh_token': used.refreshToken},
    );
    if (_refusedRefresh.contains(res.statusCode)) {
      final AuthTokens? latest = await _store.load();
      final bool replaced =
          latest != null && latest.accessToken != used.accessToken;
      return replaced ? latest.accessToken : null;
    }
    if (res.statusCode >= 400) {
      throw _errorFrom(res);
    }
    final AuthTokens next = AuthTokens.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    ).copyWith(playerId: _playerIdOf(current));
    await _saveTokens(next);
    return next.accessToken;
  }

  ApiException _errorFrom(http.Response res) {
    String message = 'HTTP ${res.statusCode}';
    String? code;
    List<Map<String, dynamic>> jobs = const <Map<String, dynamic>>[];
    try {
      final dynamic body = jsonDecode(res.body);
      if (body is Map && body['error'] is Map) {
        final Map<dynamic, dynamic> err =
            body['error'] as Map<dynamic, dynamic>;
        if (err['message'] is String) {
          message = err['message'] as String;
        }
        if (err['code'] is String) {
          code = err['code'] as String;
        }
      }
      if (body is Map && body['jobs'] is List) {
        jobs = (body['jobs'] as List<dynamic>)
            .whereType<Map<String, dynamic>>()
            .toList();
      }
    } catch (_) {}
    return ApiException(
      message,
      status: res.statusCode,
      code: code,
      jobs: jobs,
    );
  }

  Map<String, dynamic> _asJson(http.Response res) => res.body.isEmpty
      ? <String, dynamic>{}
      : jsonDecode(res.body) as Map<String, dynamic>;

  List<Map<String, dynamic>> _asArray(http.Response res) => res.body.isEmpty
      ? const <Map<String, dynamic>>[]
      : (jsonDecode(res.body) as List<dynamic>)
            .cast<Map<String, dynamic>>()
            .toList();

  Future<T> getJson<T>(
    String path,
    T Function(Map<String, dynamic>) fromJson,
  ) async {
    final http.Response res = await _send('GET', path);
    return fromJson(_asJson(res));
  }

  Future<List<T>> getJsonArray<T>(
    String path,
    T Function(Map<String, dynamic>) fromJson,
  ) async {
    final http.Response res = await _send('GET', path);
    return _asArray(res).map(fromJson).toList();
  }

  Future<Paged<T>> getPage<T>(
    String path,
    T Function(Map<String, dynamic>) fromJson,
  ) async {
    final http.Response res = await _send('GET', path);
    return Paged<T>.fromJson(_asJson(res), fromJson);
  }

  Future<List<T>> postJsonArray<T>(
    String path,
    Object? body,
    T Function(Map<String, dynamic>) fromJson,
  ) async {
    final http.Response res = await _send('POST', path, body: body);
    return _asArray(res).map(fromJson).toList();
  }

  Future<T> postJson<T>(
    String path,
    Object? body,
    T Function(Map<String, dynamic>) fromJson,
  ) async {
    final http.Response res = await _send('POST', path, body: body);
    return fromJson(_asJson(res));
  }

  Future<void> sendVoid(String method, String path, {Object? body}) async {
    await _send(method, path, body: body);
  }

  Future<int> sendStatus(String method, String path, {Object? body}) async =>
      (await _send(method, path, body: body)).statusCode;

  Future<Uint8List> bytes(String path) async {
    final http.Response res = await _send('GET', path);
    return res.bodyBytes;
  }

  Future<String> publicText(String path) async {
    final http.Response res = await _raw('GET', path);
    if (res.statusCode >= 400) {
      throw _errorFrom(res);
    }
    return res.body;
  }

  Future<AuthTokens?> currentTokens() => _store.load();

  Future<void> login(String username, String password) async {
    final http.Response res = await _raw(
      'POST',
      '/api/v1/auth/login',
      body: <String, String>{'username': username, 'password': password},
    );
    if (res.statusCode >= 400) {
      throw _errorFrom(res);
    }
    forgetUser();
    _unsavedPlayerId = null;
    await _saveTokens(
      AuthTokens.fromJson(
        jsonDecode(res.body) as Map<String, dynamic>,
      ).copyWith(playerId: _playerIdOf(null)),
    );
  }

  Future<void> redeemLinkCode(
    String code, {
    required String deviceName,
    required String platform,
  }) async {
    final http.Response res = await _raw(
      'POST',
      '/api/v1/auth/link',
      body: <String, dynamic>{
        'code': code,
        'device': <String, String>{'name': deviceName, 'platform': platform},
      },
    );
    if (res.statusCode >= 400) {
      throw _errorFrom(res);
    }
    final IssuedToken issued = IssuedToken.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
    forgetUser();
    await _saveTokens(
      AuthTokens(accessToken: issued.token, deviceId: issued.deviceId),
    );
  }

  Future<String> linkedDeviceId() async =>
      (await _store.load())?.deviceId ?? '';

  Future<void> logout() async {
    final AuthTokens? stored = await _store.load();
    if (stored != null && stored.canRefresh) {
      try {
        await _raw(
          'POST',
          '/api/v1/auth/logout',
          body: <String, String>{'refresh_token': stored.refreshToken},
        );
      } catch (_) {}
    }
    forgetUser();
    await _clearTokens();
  }

  void forgetUser() => _self = null;

  Future<SelfUser> currentUser() {
    final Future<SelfUser>? cached = _self;
    if (cached != null) {
      return cached;
    }
    final Future<SelfUser> pending = getJson(
      '/api/v1/users/self',
      SelfUser.fromJson,
    );
    _self = pending;
    unawaited(
      pending.then<void>((SelfUser _) {}, onError: (Object _) => forgetUser()),
    );
    return pending;
  }
}
