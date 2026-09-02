import 'package:flutter/widgets.dart';

import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/when_visible.dart';
import 'package:shadowmask/pages/default/page_states.dart';

class LazyBlock<T> extends StatefulWidget {
  const LazyBlock({
    super.key,
    required this.load,
    required this.builder,
    this.placeholder,
    this.loading,
    this.errorText,
    this.margin = kVisibilityMargin,
  });

  final Future<T> Function() load;
  final DataBuilder<T> builder;
  final Widget? placeholder;
  final Widget? loading;
  final String? errorText;
  final double margin;

  @override
  State<LazyBlock<T>> createState() => _LazyBlockState<T>();
}

class _LazyBlockState<T> extends State<LazyBlock<T>> {
  Future<T>? _future;

  void _start() {
    if (_future != null) {
      return;
    }
    _retry();
  }

  void _retry() {
    final Future<T> started = widget.load();
    started.then<void>((T _) {}, onError: (Object _, StackTrace _) {});
    setState(() {
      _future = started;
    });
  }

  Widget _idle(BuildContext context) =>
      widget.placeholder ??
      widget.loading ??
      LoadingShape.maybeOf(context) ??
      const SkeletonLines();

  @override
  Widget build(BuildContext context) {
    final Future<T>? future = _future;
    if (future == null) {
      return WhenVisible(
        onVisible: _start,
        margin: widget.margin,
        child: _idle(context),
      );
    }
    return buildBlock<T>(
      future: future,
      builder: widget.builder,
      errorText: widget.errorText,
      onRetry: _retry,
      loading: widget.loading,
    );
  }
}
