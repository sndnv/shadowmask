import 'package:flutter/material.dart';

import 'package:shadowmask/components/progress_bar.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/touch_platform.dart';

class PosterPlay extends StatefulWidget {
  const PosterPlay({
    super.key,
    required this.child,
    required this.tooltip,
    required this.onTap,
    this.icon = Icons.play_arrow,
    this.progressPercent,
  });

  final Widget child;
  final String tooltip;
  final IconData icon;
  final VoidCallback? onTap;
  final int? progressPercent;

  @override
  State<PosterPlay> createState() => _PosterPlayState();
}

class _PosterPlayState extends State<PosterPlay> {
  bool _hovered = false;
  bool _focused = false;

  Widget _art() {
    final int? percent = widget.progressPercent;
    if (percent == null) {
      return widget.child;
    }
    return ClipRRect(
      borderRadius: const BorderRadius.all(Radii.md),
      child: Stack(
        fit: StackFit.passthrough,
        children: <Widget>[
          widget.child,
          Positioned(
            left: 0,
            right: 0,
            bottom: 0,
            child: ProgressBar(percent: percent),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (widget.onTap == null || touchPlatform(context)) {
      return _art();
    }
    final Tokens t = context.tokens;
    final bool lit = _hovered || _focused;
    return Tooltip(
      message: widget.tooltip,
      child: Semantics(
        button: true,
        label: widget.tooltip,
        child: Stack(
          fit: StackFit.passthrough,
          children: <Widget>[
            _art(),
            if (lit)
              Positioned.fill(
                child: IgnorePointer(
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: t.bg.withValues(alpha: 0.45),
                      border: Border.all(color: t.accent, width: 2),
                      borderRadius: const BorderRadius.all(Radii.md),
                    ),
                    child: Center(
                      child: Container(
                        width: 56,
                        height: 56,
                        decoration: BoxDecoration(
                          color: t.accent,
                          shape: BoxShape.circle,
                        ),
                        child: Icon(
                          widget.icon,
                          size: 32,
                          color: t.accentContrast,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            Positioned.fill(
              child: Material(
                type: MaterialType.transparency,
                child: InkWell(
                  borderRadius: const BorderRadius.all(Radii.md),
                  onTap: widget.onTap,
                  onHover: (bool hovered) => setState(() => _hovered = hovered),
                  onFocusChange: (bool focused) =>
                      setState(() => _focused = focused),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
