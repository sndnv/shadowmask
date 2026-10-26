import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

const Set<String> _entryPaths = <String>{'/', '/link'};

bool _opensEntry(Uri from, Uri to) {
  if (_entryPaths.contains(from.path) || _entryPaths.contains(to.path)) {
    return false;
  }
  return from.path != to.path ||
      from.queryParameters['id'] != to.queryParameters['id'];
}

int? _stampOf(Object? state) {
  final Object? stamp = state is Map ? state['stamp'] : null;
  return stamp is num ? stamp.toInt() : null;
}

bool _isPage(Route<dynamic> route) =>
    route is PageRoute && route.settings.name != null;

class BrowserHistoryParser extends RouteInformationParser<RouteInformation> {
  const BrowserHistoryParser();

  @override
  Future<RouteInformation> parseRouteInformation(
    RouteInformation routeInformation,
  ) => SynchronousFuture<RouteInformation>(routeInformation);

  @override
  RouteInformation restoreRouteInformation(RouteInformation configuration) =>
      configuration;
}

class BrowserHistoryProvider extends PlatformRouteInformationProvider {
  BrowserHistoryProvider({String? initialRoute})
    : super(
        initialRouteInformation: RouteInformation(
          uri: Uri.parse(
            initialRoute ??
                WidgetsBinding.instance.platformDispatcher.defaultRouteName,
          ),
        ),
      );

  @override
  void routerReportsNewRouteInformation(
    RouteInformation routeInformation, {
    RouteInformationReportingType type = RouteInformationReportingType.none,
  }) {
    final int? before = _stampOf(value.state);
    final bool entry =
        before != null && _stampOf(routeInformation.state) != before;
    super.routerReportsNewRouteInformation(
      routeInformation,
      type: entry
          ? RouteInformationReportingType.navigate
          : RouteInformationReportingType.neglect,
    );
  }
}

class BrowserHistory extends RouterDelegate<RouteInformation>
    with ChangeNotifier, PopNavigatorRouterDelegateMixin<RouteInformation> {
  BrowserHistory({
    required this.navigatorKey,
    required this.onGenerateRoute,
    required this.onGenerateInitialRoutes,
    List<NavigatorObserver> observers = const <NavigatorObserver>[],
  }) {
    _observers = <NavigatorObserver>[...observers, _stack];
  }

  @override
  final GlobalKey<NavigatorState> navigatorKey;
  final RouteFactory onGenerateRoute;
  final List<Route<dynamic>> Function(String initialRoute)
  onGenerateInitialRoutes;

  late final _PageStack _stack = _PageStack(_changed);
  late final List<NavigatorObserver> _observers;
  String _initial = Navigator.defaultRouteName;
  Uri? _uri;
  int? _stamp;
  bool _ordered = true;
  bool _following = false;

  int _next() =>
      math.max(DateTime.now().microsecondsSinceEpoch, (_stamp ?? 0) + 1);

  @override
  RouteInformation? get currentConfiguration {
    final Uri? uri = _uri;
    if (uri == null) {
      return null;
    }
    return RouteInformation(
      uri: uri,
      state: <String, Object?>{'stamp': _stamp},
    );
  }

  @override
  Future<void> setInitialRoutePath(RouteInformation configuration) {
    _initial = configuration.uri.toString();
    _stamp = _stampOf(configuration.state);
    _ordered = _stamp != null;
    return SynchronousFuture<void>(null);
  }

  @override
  Future<void> setNewRoutePath(RouteInformation configuration) {
    final NavigatorState? navigator = navigatorKey.currentState;
    if (navigator == null) {
      return setInitialRoutePath(configuration);
    }
    final Uri target = configuration.uri;
    final int? stamp = _stampOf(configuration.state);
    final int? current = _stamp;
    final bool newer =
        _ordered && stamp != null && current != null && stamp > current;
    _uri = target;
    _stamp = stamp ?? _next();
    _ordered = _ordered && stamp != null;
    navigator.popUntil(_isPage);
    final List<Uri> pages = _stack.pages;
    if (pages.isNotEmpty && pages.last == target) {
      return SynchronousFuture<void>(null);
    }
    if (pages.contains(target)) {
      navigator.popUntil(
        (Route<dynamic> route) =>
            _isPage(route) && Uri.parse(route.settings.name!) == target,
      );
    } else if (newer) {
      navigator.pushNamed(target.toString());
    } else {
      navigator.pushReplacementNamed(target.toString());
    }
    return SynchronousFuture<void>(null);
  }

  void _changed() {
    if (_following) {
      return;
    }
    _following = true;
    scheduleMicrotask(_follow);
  }

  void _follow() {
    _following = false;
    final List<Uri> pages = _stack.pages;
    if (pages.isEmpty) {
      return;
    }
    final Uri top = pages.last;
    final Uri? before = _uri;
    if (top == before && _stamp != null) {
      return;
    }
    if (before != null && _stamp != null && _opensEntry(before, top)) {
      _stamp = _next();
      _ordered = true;
    }
    _stamp ??= _next();
    _uri = top;
    notifyListeners();
  }

  @override
  Widget build(BuildContext context) => FocusScope(
    debugLabel: 'Navigator Scope',
    autofocus: true,
    child: Navigator(
      key: navigatorKey,
      clipBehavior: Clip.none,
      initialRoute: _initial,
      onGenerateRoute: onGenerateRoute,
      onGenerateInitialRoutes: (NavigatorState _, String initialRoute) =>
          onGenerateInitialRoutes(initialRoute),
      observers: _observers,
      routeTraversalEdgeBehavior: kIsWeb
          ? TraversalEdgeBehavior.leaveFlutterView
          : TraversalEdgeBehavior.parentScope,
    ),
  );
}

class _PageStack extends NavigatorObserver {
  _PageStack(this.onChange);

  final VoidCallback onChange;
  final List<Route<dynamic>> _routes = <Route<dynamic>>[];

  List<Uri> get pages => <Uri>[
    for (final Route<dynamic> route in _routes)
      if (_isPage(route)) Uri.parse(route.settings.name!),
  ];

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) {
    _routes.add(route);
    onChange();
  }

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) {
    _routes.remove(route);
    onChange();
  }

  @override
  void didRemove(Route<dynamic> route, Route<dynamic>? previousRoute) {
    _routes.remove(route);
    onChange();
  }

  @override
  void didReplace({Route<dynamic>? newRoute, Route<dynamic>? oldRoute}) {
    final int at = oldRoute == null ? -1 : _routes.indexOf(oldRoute);
    if (newRoute == null) {
      _routes.remove(oldRoute);
    } else if (at < 0) {
      _routes.add(newRoute);
    } else {
      _routes[at] = newRoute;
    }
    onChange();
  }
}
