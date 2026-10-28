import 'dart:async';

import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

typedef Prepare<T> = Future<bool> Function(T value);

class PageDirector<T> extends ChangeNotifier {
  PageDirector(this.none, {this._prepare}) : _shown = none;

  final T none;
  final Prepare<T>? _prepare;
  final List<Route<dynamic>> _routes = <Route<dynamic>>[];
  final Map<Route<dynamic>, T> _requests = <Route<dynamic>, T>{};
  final Map<Route<dynamic>, T> _fallbacks = <Route<dynamic>, T>{};
  late final NavigatorObserver observer = _PageWatch<T>(this);
  T _shown;
  ({T value})? _target;
  int _turn = 0;
  bool _deferred = false;
  bool _queued = false;
  bool _disposed = false;

  T get shown => _shown;

  void request(Route<dynamic> page, T value) {
    if (_routes.contains(page)) {
      _requests[page] = value;
      _settleFor(page);
    }
  }

  void fallback(Route<dynamic> page, T value) {
    if (_routes.contains(page)) {
      _fallbacks[page] = value;
      _settleFor(page);
    }
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }

  Route<dynamic>? get _page {
    for (int i = _routes.length - 1; i >= 0; i--) {
      if (_routes[i] is PageRoute) {
        return _routes[i];
      }
    }
    return null;
  }

  ({T value})? _wanted(Route<dynamic> page) {
    if (_requests.containsKey(page)) {
      return (value: _requests[page] as T);
    }
    if (_fallbacks.containsKey(page)) {
      return (value: _fallbacks[page] as T);
    }
    return null;
  }

  void _pushed(Route<dynamic> route) {
    _routes.add(route);
    _settleSoon();
  }

  void _removed(Route<dynamic> route) {
    _routes.remove(route);
    _requests.remove(route);
    _fallbacks.remove(route);
    _settleSoon();
  }

  void _replaced(Route<dynamic>? next, Route<dynamic>? old) {
    final int at = old == null ? -1 : _routes.indexOf(old);
    if (old != null) {
      _requests.remove(old);
      _fallbacks.remove(old);
    }
    if (next == null) {
      _routes.remove(old);
    } else if (at < 0) {
      _routes.add(next);
    } else {
      _routes[at] = next;
    }
    _settleSoon();
  }

  void _settleSoon() {
    if (!_queued) {
      _queued = true;
      scheduleMicrotask(() {
        _queued = false;
        _settle();
      });
    }
  }

  void _settleFor(Route<dynamic> page) {
    if (identical(page, _page)) {
      _settle();
    }
  }

  void _settle() {
    if (_disposed) {
      return;
    }
    if (SchedulerBinding.instance.schedulerPhase ==
        SchedulerPhase.persistentCallbacks) {
      if (!_deferred) {
        _deferred = true;
        SchedulerBinding.instance.addPostFrameCallback((_) {
          _deferred = false;
          _settle();
        });
      }
      return;
    }
    final Route<dynamic>? page = _page;
    final ({T value})? wanted = page == null ? null : _wanted(page);
    if (wanted != null && wanted == _target) {
      return;
    }
    _target = wanted;
    final int turn = ++_turn;
    if (wanted == null) {
      return;
    }
    final T next = wanted.value;
    final Prepare<T>? prepare = _prepare;
    if (next == _shown || next == none || prepare == null) {
      _show(next);
      return;
    }
    prepare(next).then((bool ready) {
      if (turn == _turn) {
        _show(ready ? next : none);
      }
    });
  }

  void _show(T value) {
    if (_disposed || value == _shown) {
      return;
    }
    _shown = value;
    notifyListeners();
  }
}

class _PageWatch<T> extends NavigatorObserver {
  _PageWatch(this._director);

  final PageDirector<T> _director;

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      _director._pushed(route);

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      _director._removed(route);

  @override
  void didRemove(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      _director._removed(route);

  @override
  void didReplace({Route<dynamic>? newRoute, Route<dynamic>? oldRoute}) =>
      _director._replaced(newRoute, oldRoute);
}
