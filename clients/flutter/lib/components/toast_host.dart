import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';

import 'package:shadowmask/components/bottom_chrome.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/scoped_value.dart';

const Duration kToastDuration = Duration(milliseconds: 3200);
const Duration kErrorToastDuration = Duration(milliseconds: 8000);

const double kToastMinWidth = 320;
const double kToastMaxWidth = 400;
const double kToastMinHeight = 56;

enum ToastSeverity { success, error, warning, info }

class ToastHost extends StatefulWidget {
  const ToastHost({super.key, required this.child});

  final Widget child;

  @override
  State<ToastHost> createState() => _ToastHostState();
}

class _ToastHostState extends State<ToastHost> {
  final List<_ToastEntry> _toasts = <_ToastEntry>[];
  final Map<int, Timer> _timers = <int, Timer>{};
  final ScopedValue<double> _bottomChrome = ScopedValue<double>(0);
  int _seq = 0;

  @override
  void dispose() {
    for (final Timer timer in _timers.values) {
      timer.cancel();
    }
    _timers.clear();
    _bottomChrome.dispose();
    super.dispose();
  }

  void _show(String message, String? emphasis, ToastSeverity severity) {
    final int id = _seq++;
    final bool failed = severity == ToastSeverity.error;
    setState(() => _toasts.add(_ToastEntry(id, message, emphasis, severity)));
    SemanticsService.sendAnnouncement(
      View.of(context),
      message,
      Directionality.of(context),
      assertiveness: failed ? Assertiveness.assertive : Assertiveness.polite,
    );
    _timers[id] = Timer(
      failed ? kErrorToastDuration : kToastDuration,
      () => _dismiss(id),
    );
  }

  void _dismiss(int id) {
    _timers.remove(id)?.cancel();
    if (!mounted) {
      return;
    }
    setState(() => _toasts.removeWhere((_ToastEntry e) => e.id == id));
  }

  @override
  Widget build(BuildContext context) {
    final bool narrow = MediaQuery.sizeOf(context).width < Breakpoints.sm;
    final EdgeInsets safe = MediaQuery.paddingOf(context);
    final List<_ToastEntry> ordered = narrow
        ? _toasts
        : _toasts.reversed.toList();
    return _ToastScope(
      state: this,
      child: Stack(
        children: <Widget>[
          BottomChromeScope(height: _bottomChrome, child: widget.child),
          ValueListenableBuilder<double>(
            valueListenable: _bottomChrome,
            builder: (BuildContext context, double chrome, Widget? toasts) =>
                Positioned(
                  top: narrow ? null : kShellHeaderHeight + Space.s4 + safe.top,
                  bottom: narrow ? Space.s4 + safe.bottom + chrome : null,
                  left: narrow ? Space.s4 + safe.left : null,
                  right: Space.s4 + safe.right,
                  child: toasts!,
                ),
            child: Material(
              type: MaterialType.transparency,
              child: Semantics(
                container: true,
                liveRegion: true,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: narrow
                      ? CrossAxisAlignment.center
                      : CrossAxisAlignment.end,
                  children: <Widget>[
                    for (final _ToastEntry e in ordered)
                      Padding(
                        padding: const EdgeInsets.only(top: Space.s2),
                        child: _ToastCard(
                          message: e.message,
                          emphasis: e.emphasis,
                          severity: e.severity,
                          onDismiss: e.severity == ToastSeverity.error
                              ? () => _dismiss(e.id)
                              : null,
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ToastEntry {
  const _ToastEntry(this.id, this.message, this.emphasis, this.severity);

  final int id;
  final String message;
  final String? emphasis;
  final ToastSeverity severity;
}

class _ToastScope extends InheritedWidget {
  const _ToastScope({required this.state, required super.child});

  final _ToastHostState state;

  @override
  bool updateShouldNotify(_ToastScope oldWidget) => false;
}

class Toasts {
  const Toasts._(this._state);

  final _ToastHostState? _state;

  static Toasts of(BuildContext context) {
    final _ToastScope? scope = context
        .getInheritedWidgetOfExactType<_ToastScope>();
    return Toasts._(scope?.state);
  }

  void show(
    String message, {
    String? emphasis,
    ToastSeverity severity = ToastSeverity.info,
  }) => _state?._show(message, emphasis, severity);

  void success(String message, {String? emphasis}) =>
      show(message, emphasis: emphasis, severity: ToastSeverity.success);

  void error(String message, {String? emphasis}) =>
      show(message, emphasis: emphasis, severity: ToastSeverity.error);

  void warning(String message, {String? emphasis}) =>
      show(message, emphasis: emphasis, severity: ToastSeverity.warning);

  void info(String message, {String? emphasis}) =>
      show(message, emphasis: emphasis, severity: ToastSeverity.info);
}

class _ToastCard extends StatelessWidget {
  const _ToastCard({
    required this.message,
    required this.severity,
    this.emphasis,
    this.onDismiss,
  });

  final String message;
  final String? emphasis;
  final ToastSeverity severity;
  final VoidCallback? onDismiss;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final (Color ground, Color ink) = switch (severity) {
      ToastSeverity.success => (t.ok, t.accentContrast),
      ToastSeverity.error => (t.danger, t.accentContrast),
      ToastSeverity.warning => (t.warn, t.accentContrast),
      ToastSeverity.info => (t.text, t.surface),
    };
    final VoidCallback? dismiss = onDismiss;
    final Widget card = Container(
      constraints: const BoxConstraints(
        minWidth: kToastMinWidth,
        maxWidth: kToastMaxWidth,
        minHeight: kToastMinHeight,
      ),
      padding: EdgeInsets.fromLTRB(
        Space.s4,
        Space.s3,
        dismiss == null ? Space.s4 : Space.s2,
        Space.s3,
      ),
      decoration: BoxDecoration(
        color: ground,
        borderRadius: const BorderRadius.all(Radii.md),
        boxShadow: <BoxShadow>[
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.5),
            blurRadius: 28,
            offset: const Offset(0, 10),
          ),
        ],
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          Expanded(child: _content(TextStyle(color: ink, height: 1.4))),
          if (dismiss != null)
            Padding(
              padding: const EdgeInsets.only(left: Space.s2),
              child: IconButton(
                onPressed: dismiss,
                visualDensity: VisualDensity.compact,
                iconSize: 18,
                color: ink,
                icon: const Icon(Icons.close, semanticLabel: Strings.dismiss),
              ),
            ),
        ],
      ),
    );
    return dismiss == null ? IgnorePointer(child: card) : card;
  }

  Widget _content(TextStyle style) {
    final String? term = emphasis;
    final int index = term == null || term.isEmpty ? -1 : message.indexOf(term);
    if (index < 0) {
      return Text(message, style: style);
    }
    return Text.rich(
      TextSpan(
        style: style,
        children: <InlineSpan>[
          TextSpan(text: message.substring(0, index)),
          TextSpan(
            text: message.substring(index, index + term!.length),
            style: const TextStyle(fontWeight: FontWeight.w700),
          ),
          TextSpan(text: message.substring(index + term.length)),
        ],
      ),
    );
  }
}
