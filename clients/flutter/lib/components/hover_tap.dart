import 'package:flutter/material.dart';

import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kFocusRing = 2;

class HoverTap extends StatefulWidget {
  const HoverTap({
    super.key,
    required this.onTap,
    required this.child,
    this.semanticsLabel,
    this.onFocusChange,
    this.focusRing = true,
    this.borderRadius,
  });

  final VoidCallback onTap;
  final Widget child;
  final String? semanticsLabel;
  final ValueChanged<bool>? onFocusChange;
  final bool focusRing;
  final BorderRadius? borderRadius;

  @override
  State<HoverTap> createState() => _HoverTapState();
}

class _HoverTapState extends State<HoverTap> {
  bool _focused = false;

  void _onFocusChange(bool focused) {
    setState(() => _focused = focused);
    widget.onFocusChange?.call(focused);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Widget tapped = GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: widget.onTap,
      child: widget.child,
    );
    return Semantics(
      button: true,
      label: widget.semanticsLabel,
      child: FocusableActionDetector(
        mouseCursor: SystemMouseCursors.click,
        onFocusChange: _onFocusChange,
        actions: <Type, Action<Intent>>{
          ActivateIntent: CallbackAction<ActivateIntent>(
            onInvoke: (ActivateIntent intent) {
              widget.onTap();
              return null;
            },
          ),
        },
        child: widget.focusRing
            ? DecoratedBox(
                position: DecorationPosition.foreground,
                decoration: BoxDecoration(
                  borderRadius: widget.borderRadius,
                  border: Border.all(
                    color: _focused ? t.accent : Colors.transparent,
                    width: kFocusRing,
                  ),
                ),
                child: tapped,
              )
            : tapped,
      ),
    );
  }
}
