import 'package:flutter/widgets.dart';

class ScopedValue<T> extends ValueNotifier<T> {
  ScopedValue(super.value);

  bool _disposed = false;
  Object? _owner;

  bool get disposed => _disposed;

  void publish(T next, {Object? owner}) {
    if (_disposed) {
      return;
    }
    if (owner != null) {
      _owner = owner;
    }
    if (value != next) {
      value = next;
    }
  }

  void releaseAfterFrame(T held, T fallback, {Object? owner}) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_disposed) {
        return;
      }
      if (owner != null && !identical(_owner, owner)) {
        return;
      }
      _owner = null;
      if (value == held) {
        value = fallback;
      }
    });
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}
