import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/corner_brackets.dart';
import 'package:shadowmask/components/progress_bar.dart';
import 'package:shadowmask/components/resume_ring.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/touch_platform.dart';

class PosterPlay extends StatefulWidget {
  const PosterPlay({
    super.key,
    required this.child,
    required this.label,
    required this.onTap,
    this.progressPercent,
    this.resumePercent,
  });

  final Widget child;
  final String label;
  final VoidCallback? onTap;
  final int? progressPercent;
  final int? resumePercent;

  @override
  State<PosterPlay> createState() => _PosterPlayState();
}

const Key _tapKey = ValueKey<String>('poster-tap');

class _PosterPlayState extends State<PosterPlay> {
  bool _hovered = false;
  bool _focused = false;

  Widget _art() {
    final int? percent = widget.progressPercent;
    if (percent == null) {
      return widget.child;
    }
    final double inset = context.shapes.cut ? Space.s1 : 0;
    return context.shapes.clip(
      Corners.card,
      child: Stack(
        fit: StackFit.passthrough,
        children: <Widget>[
          widget.child,
          Positioned(
            left: inset,
            right: inset,
            bottom: inset,
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
    final Shapes shapes = context.shapes;
    final bool lit = _hovered || _focused;
    return Semantics(
      button: true,
      label: widget.label,
      child: Stack(
        fit: StackFit.passthrough,
        clipBehavior: shapes.cut ? Clip.none : Clip.hardEdge,
        children: <Widget>[
          _art(),
          if (lit)
            Positioned.fill(
              child: IgnorePointer(
                child: DecoratedBox(
                  decoration: shapes.box(
                    Corners.card,
                    color: t.bg.withValues(alpha: 0.45),
                    side: BorderSide(color: shapes.line(t.accent), width: 2),
                  ),
                  child: Center(
                    child: shapes.cut
                        ? Padding(
                            padding: const EdgeInsets.all(Space.s3),
                            child: FittedBox(
                              fit: BoxFit.scaleDown,
                              child: ResumeRing(
                                percent:
                                    widget.resumePercent ??
                                    widget.progressPercent,
                              ),
                            ),
                          )
                        : Container(
                            width: 56,
                            height: 56,
                            decoration: shapes.roundBox(color: t.accent),
                            child: Icon(
                              Icons.play_arrow,
                              size: 32,
                              color: t.accentContrast,
                            ),
                          ),
                  ),
                ),
              ),
            ),
          Positioned.fill(
            key: _tapKey,
            child: Material(
              type: MaterialType.transparency,
              child: InkWell(
                customBorder: shapes.border(Corners.card),
                onTap: widget.onTap,
                onHover: (bool hovered) => setState(() => _hovered = hovered),
                onFocusChange: (bool focused) =>
                    setState(() => _focused = focused),
              ),
            ),
          ),
          if (lit && shapes.cut)
            Positioned(
              left: -kBracketReach,
              top: -kBracketReach,
              right: -kBracketReach,
              bottom: -kBracketReach,
              child: CornerBrackets(color: t.accent),
            ),
        ],
      ),
    );
  }
}
