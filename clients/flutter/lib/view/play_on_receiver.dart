import 'dart:async';

import 'package:flutter/foundation.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/authentication_failure.dart';
import 'package:shadowmask/api/authorization_failure.dart';
import 'package:shadowmask/api/play_on_api.dart';
import 'package:shadowmask/api/play_on_store.dart';
import 'package:shadowmask/util/command_listener.dart';
import 'package:shadowmask/view/target_addresses.dart';

const int kMinListenPort = 1024;
const int kMaxListenPort = 65535;
const int kRememberedCommands = 8;
const int kTargetRetries = 3;
const Duration kTargetRetryDelay = Duration(seconds: 5);

enum PlayOnStatus { off, listening, noNetwork, bindFailed, reportFailed }

class PlayOnReceiver extends ChangeNotifier {
  PlayOnReceiver({
    required ApiClient api,
    required this.onPlay,
    required this.onRefused,
    this.store = const PlayOnStore(),
    this.retryDelay = kTargetRetryDelay,
    ListenerBinder? bind,
    AddressLister? addresses,
  }) : _api = api,
       _playOn = PlayOnApi(api),
       _bind = bind ?? bindCommandListener,
       _addresses = addresses ?? interfaceAddresses {
    _session = api.sessionChanged.listen((_) => unawaited(refresh()));
  }

  final ApiClient _api;
  final PlayOnApi _playOn;
  final ListenerBinder _bind;
  final AddressLister _addresses;
  final PlayOnStore store;
  final Duration retryDelay;
  final Future<void> Function(String versionId) onPlay;
  final VoidCallback onRefused;

  late final StreamSubscription<void> _session;
  final Map<String, Future<int>> _commands = <String, Future<int>>{};
  bool _loaded = false;
  bool _linked = false;
  bool _enabled = false;
  bool _foreground = true;
  bool _disposed = false;
  bool _stale = false;
  int? _port;
  PlayOnStatus _status = PlayOnStatus.off;
  List<String> _listeningOn = const <String>[];
  int? _boundPort;
  CommandListener? _listener;
  Timer? _retry;
  int _retries = 0;
  Future<void> _queue = Future<void>.value();

  bool get loaded => _loaded;
  bool get linked => _linked;
  bool get enabled => _enabled;
  int? get port => _port;
  PlayOnStatus get status => _status;
  List<String> get listeningOn => _listeningOn;
  int? get boundPort => _boundPort;

  Future<void> load() => _serial(() async {
    final PlayOnSettings settings = await store.load();
    _enabled = settings.enabled;
    _port = settings.port;
    _loaded = true;
    await _apply();
  });

  Future<void> refresh() => _serial(_apply);

  Future<void> resume() => _serial(() {
    _foreground = true;
    return _apply();
  });

  Future<void> pause() => _serial(() {
    _foreground = false;
    return _apply();
  });

  Future<void> setEnabled(bool on) => _serial(() async {
    _enabled = on;
    _stale = !on;
    _notify();
    await store.saveEnabled(on);
    await _apply();
  });

  Future<void> setPort(int? port) => _serial(() async {
    _port = port;
    await store.savePort(port);
    await _apply();
  });

  Future<void> _serial(Future<void> Function() op) {
    final Future<void> next = _queue.then((_) => op());
    _queue = next.catchError((Object _) {});
    return next;
  }

  Future<void> _apply({bool retrying = false}) async {
    _retry?.cancel();
    _retry = null;
    if (!retrying) {
      _retries = 0;
    }
    _linked = (await _api.linkedDeviceId()).isNotEmpty;
    await _close();
    if (!_linked) {
      _stale = false;
      if (_loaded && _enabled) {
        _enabled = false;
        await store.saveEnabled(false);
      }
    }
    bool again = false;
    if (_disposed || !_loaded || !_enabled || !_linked) {
      _status = PlayOnStatus.off;
      if (_stale && !_disposed) {
        again = !await _clearReport();
      }
    } else if (_foreground) {
      await _start();
      again =
          _status == PlayOnStatus.noNetwork ||
          _status == PlayOnStatus.reportFailed;
    }
    if (again) {
      _scheduleRetry();
    }
    _notify();
  }

  void _scheduleRetry() {
    if (_disposed || _retries >= kTargetRetries) {
      return;
    }
    _retry = Timer(retryDelay, () {
      _retries += 1;
      unawaited(_serial(() => _apply(retrying: true)));
    });
  }

  Future<bool> _clearReport() async {
    try {
      await _playOn.clearTarget();
      _stale = false;
      return true;
    } catch (_) {
      return false;
    }
  }

  Future<bool> _mayListen() async {
    final bool linked = (await _api.linkedDeviceId()).isNotEmpty;
    return linked && !_disposed && _loaded && _enabled && _foreground;
  }

  Future<void> _start() async {
    final List<String> found = targetAddresses(
      await _addresses().catchError((Object _) => const <InterfaceAddress>[]),
    );
    if (found.isEmpty) {
      _status = PlayOnStatus.noNetwork;
      return;
    }
    if (!await _mayListen()) {
      _status = PlayOnStatus.off;
      return;
    }
    try {
      _listener = await _bind(_port ?? 0, _handle);
    } catch (_) {
      _status = PlayOnStatus.bindFailed;
      return;
    }
    if (!await _mayListen()) {
      await _close();
      _status = PlayOnStatus.off;
      return;
    }
    final int bound = _listener!.port;
    try {
      await _playOn.reportTarget(found, bound);
    } catch (_) {
      await _close();
      _status = PlayOnStatus.reportFailed;
      return;
    }
    _status = PlayOnStatus.listening;
    _listeningOn = found;
    _boundPort = bound;
  }

  Future<void> _close() async {
    final CommandListener? listener = _listener;
    _listener = null;
    await listener?.close();
  }

  Future<int> _handle(String token) {
    final Future<int>? seen = _commands[token];
    if (seen != null) {
      return seen;
    }
    final Future<int> answer = _answer(token);
    _commands[token] = answer;
    while (_commands.length > kRememberedCommands) {
      _commands.remove(_commands.keys.first);
    }
    return answer;
  }

  Future<int> _answer(String token) async {
    if (!await _mayListen()) {
      return 403;
    }
    try {
      final String version = await _playOn.redeem(token);
      if (!await _mayListen()) {
        return 403;
      }
      unawaited(onPlay(version));
      return 202;
    } on AuthenticationFailure {
      onRefused();
      return 401;
    } on AuthorizationFailure {
      onRefused();
      return 403;
    } catch (_) {
      onRefused();
      return 502;
    }
  }

  void _notify() {
    if (!_disposed) {
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _retry?.cancel();
    unawaited(_session.cancel());
    unawaited(_serial(_close));
    super.dispose();
  }
}
