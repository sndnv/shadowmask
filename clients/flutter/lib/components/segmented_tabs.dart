import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class SegmentedTabs<T> extends StatefulWidget {
  const SegmentedTabs({
    super.key,
    required this.current,
    required this.tabs,
    required this.onChanged,
  });

  final T current;
  final List<(T, String)> tabs;
  final ValueChanged<T> onChanged;

  @override
  State<SegmentedTabs<T>> createState() => _SegmentedTabsState<T>();
}

class _SegmentedTabsState<T> extends State<SegmentedTabs<T>> {
  final Map<T, FocusNode> _nodes = <T, FocusNode>{};

  @override
  void dispose() {
    for (final FocusNode node in _nodes.values) {
      node.dispose();
    }
    super.dispose();
  }

  FocusNode _nodeFor(T value) =>
      _nodes.putIfAbsent(value, () => FocusNode(debugLabel: 'tab'));

  void _move(int delta) {
    final int count = widget.tabs.length;
    if (count < 2) {
      return;
    }
    final int at = widget.tabs.indexWhere(
      ((T, String) e) => e.$1 == widget.current,
    );
    final int next = ((at < 0 ? 0 : at) + delta + count) % count;
    widget.onChanged(widget.tabs[next].$1);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _nodeFor(widget.current).requestFocus();
      }
    });
  }

  KeyEventResult _onKey(FocusNode _, KeyEvent event) {
    if (event is KeyUpEvent) {
      return KeyEventResult.ignored;
    }
    if (event.logicalKey == LogicalKeyboardKey.arrowRight) {
      _move(1);
      return KeyEventResult.handled;
    }
    if (event.logicalKey == LogicalKeyboardKey.arrowLeft) {
      _move(-1);
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SelectionContainer.disabled(
      child: SizedBox(
        width: double.infinity,
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: Border(bottom: BorderSide(color: t.border)),
          ),
          child: Focus(
            canRequestFocus: false,
            skipTraversal: true,
            onKeyEvent: _onKey,
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Semantics(
                role: SemanticsRole.tabBar,
                explicitChildNodes: true,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: <Widget>[
                    for (final (T value, String label) in widget.tabs)
                      _Tab<T>(
                        label: label,
                        active: value == widget.current,
                        focusNode: _nodeFor(value),
                        onTap: () => widget.onChanged(value),
                      ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class TabPanel extends StatelessWidget {
  const TabPanel({super.key, required this.label, required this.child});

  final String label;
  final Widget child;

  @override
  Widget build(BuildContext context) =>
      Semantics(role: SemanticsRole.tabPanel, label: label, child: child);
}

class _Tab<T> extends StatelessWidget {
  const _Tab({
    required this.label,
    required this.active,
    required this.focusNode,
    required this.onTap,
  });

  final String label;
  final bool active;
  final FocusNode focusNode;
  final VoidCallback onTap;

  static const BorderRadius _radius = BorderRadius.only(
    topLeft: Radii.md,
    topRight: Radii.md,
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Widget label = Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: 13,
        vertical: Space.s2 - 1,
      ),
      child: Text(
        this.label,
        style: TextStyle(
          color: active ? t.text : t.muted,
          fontSize: 13.5,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
    final Widget tap = InkWell(
      onTap: onTap,
      focusNode: focusNode,
      canRequestFocus: active,
      borderRadius: _radius,
      child: label,
    );
    final Widget tab = Semantics(
      role: SemanticsRole.tab,
      selected: active,
      child: active
          ? Transform.translate(
              offset: const Offset(0, 1),
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: t.border,
                  borderRadius: _radius,
                ),
                child: Padding(
                  padding: const EdgeInsets.only(top: 1, left: 1, right: 1),
                  child: Material(
                    color: t.surface,
                    borderRadius: _radius,
                    child: tap,
                  ),
                ),
              ),
            )
          : tap,
    );
    return Padding(padding: const EdgeInsets.only(right: 6), child: tab);
  }
}
