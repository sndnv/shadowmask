import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:shadowmask/nav/route_observer.dart';

mixin RefreshOnReturn<T extends StatefulWidget> on State<T>, RouteAware {
  Future<void> refreshOnReturn();

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final ModalRoute<dynamic>? route = ModalRoute.of(context);
    if (route is PageRoute<dynamic>) {
      appRouteObserver.subscribe(this, route);
    }
  }

  @override
  void didPopNext() {
    if (mounted) {
      unawaited(refreshOnReturn());
    }
  }

  @override
  void dispose() {
    appRouteObserver.unsubscribe(this);
    super.dispose();
  }
}
