import 'package:flutter/widgets.dart';

import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/view/failure_reason.dart';

const Object _kSelf = #self;

mixin Mutations<T extends StatefulWidget> on State<T> {
  final Set<Object> _inFlight = <Object>{};

  bool busy([Object? key]) =>
      key == null ? _inFlight.isNotEmpty : _inFlight.contains(key);

  Future<void> runBusy(Future<void> Function() action, {Object? key}) async {
    final Object token = key ?? _kSelf;
    if (_inFlight.contains(token)) {
      return;
    }
    setState(() => _inFlight.add(token));
    try {
      await action();
    } finally {
      _inFlight.remove(token);
      if (mounted) {
        setState(() {});
      }
    }
  }

  Future<void> mutate(
    Future<void> Function() action, {
    required String errorText,
    Object? key,
    String? successText,
    String? emphasis,
    VoidCallback? then,
  }) => runBusy(key: key, () async {
    try {
      await action();
      if (!mounted) {
        return;
      }
      final String? success = successText;
      if (success != null) {
        Toasts.of(context).success(success, emphasis: emphasis);
      }
      then?.call();
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(errorText, e));
      }
    }
  });
}
