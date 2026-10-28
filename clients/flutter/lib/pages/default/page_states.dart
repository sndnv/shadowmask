import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/api/authentication_failure.dart';
import 'package:shadowmask/api/authorization_failure.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';

typedef DataBuilder<T> = Widget Function(BuildContext context, T data);

const Duration kLoadingRevealDelay = Duration(milliseconds: 180);

class LoadingShape extends InheritedWidget {
  const LoadingShape({super.key, required this.shape, required super.child});

  final Widget? shape;

  static Widget? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LoadingShape>()?.shape;

  @override
  bool updateShouldNotify(LoadingShape oldWidget) => shape != oldWidget.shape;
}

class DelayedReveal extends StatefulWidget {
  const DelayedReveal({
    super.key,
    required this.child,
    this.delay = kLoadingRevealDelay,
  });

  final Widget child;
  final Duration delay;

  @override
  State<DelayedReveal> createState() => _DelayedRevealState();
}

class _DelayedRevealState extends State<DelayedReveal> {
  Timer? _timer;
  bool _visible = false;

  @override
  void initState() {
    super.initState();
    _timer = Timer(widget.delay, () {
      if (mounted) {
        setState(() => _visible = true);
      }
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      _visible ? widget.child : const SizedBox.shrink();
}

Widget buildBlock<T>({
  required Future<T> future,
  required DataBuilder<T> builder,
  String? errorText,
  VoidCallback? onRetry,
  Widget? loading,
  bool failureClearsBackdrop = false,
}) {
  return FutureBuilder<T>(
    future: future,
    builder: (BuildContext context, AsyncSnapshot<T> snapshot) =>
        buildSnapshot<T>(
          context,
          snapshot,
          builder: builder,
          errorText: errorText,
          onRetry: onRetry,
          loading: loading,
          failureClearsBackdrop: failureClearsBackdrop,
        ),
  );
}

Widget buildSnapshot<T>(
  BuildContext context,
  AsyncSnapshot<T> snapshot, {
  required DataBuilder<T> builder,
  String? errorText,
  VoidCallback? onRetry,
  Widget? loading,
  bool failureClearsBackdrop = false,
}) {
  if (snapshot.connectionState != ConnectionState.done) {
    return DelayedReveal(
      child: loading ?? LoadingShape.maybeOf(context) ?? const SkeletonLines(),
    );
  }
  final Object? error = snapshot.error;
  if (error == null) {
    return builder(context, snapshot.data as T);
  }
  final Widget failure = _failed(context, error, errorText, onRetry);
  return failureClearsBackdrop ? PageBackdrop.none(child: failure) : failure;
}

Widget _failed(
  BuildContext context,
  Object error,
  String? errorText,
  VoidCallback? onRetry,
) {
  if (error is AuthenticationFailure) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (context.mounted) {
        Navigator.of(
          context,
        ).pushNamedAndRemoveUntil('/', (Route<dynamic> r) => false);
      }
    });
    return const StatusText(Strings.signInRequired, live: true);
  }
  if (error is AuthorizationFailure) {
    return const StatusText(Strings.notAuthorized, live: true);
  }
  return _Failure(text: errorText ?? Strings.couldNotLoad, onRetry: onRetry);
}

class _Failure extends StatelessWidget {
  const _Failure({required this.text, this.onRetry});

  final String text;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    if (onRetry == null) {
      return StatusText(text, live: true);
    }
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: <Widget>[
        StatusText(text, live: true),
        OutlinedButton.icon(
          onPressed: onRetry,
          icon: const Icon(Icons.refresh, size: 18),
          label: const Text(Strings.retry),
        ),
      ],
    );
  }
}
