import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/ring_steps.dart';

const double kResumeRingSize = 136;
const double _kRingRadius = 60;
const double _kSegmentStroke = 8;
const double _kSegmentLength = 7;
const double _kDashRadius = 51;
const double _kDashLength = 2;
const int _kDashes = 64;
const double _kCoreRadius = 45;
const double _kWordSize = 13;

class ResumeRing extends StatelessWidget {
  const ResumeRing({super.key, this.percent});

  final int? percent;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    final TextStyle ink = TextStyle(color: t.accentContrast, height: 1);
    final int? percent = this.percent;
    return ExcludeSemantics(
      child: MediaQuery.withNoTextScaling(
        child: SizedBox.square(
          dimension: kResumeRingSize,
          child: CustomPaint(
            painter: _RingPainter(
              filled: percent == null ? 0 : ringFilledSteps(percent),
              fill: t.accent,
              track: shapes.track(t.border),
              dash: shapes.line(t.border),
            ),
            child: Center(
              child: Container(
                width: _kCoreRadius * 2,
                height: _kCoreRadius * 2,
                decoration: BoxDecoration(
                  color: t.accent,
                  shape: BoxShape.circle,
                ),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: <Widget>[
                    Icon(Icons.play_arrow, size: 22, color: t.accentContrast),
                    const SizedBox(height: 4),
                    Text(
                      percent == null ? Strings.play : Strings.resumeAction,
                      style: ink.copyWith(
                        fontSize: _kWordSize,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    if (percent != null) ...<Widget>[
                      const SizedBox(height: 4),
                      Text(
                        '$percent%',
                        style: shapes.mono(
                          ink.copyWith(
                            fontSize: 12,
                            color: t.accentContrast.withValues(alpha: 0.85),
                          ),
                        ),
                      ),
                    ],
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

class _RingPainter extends CustomPainter {
  const _RingPainter({
    required this.filled,
    required this.fill,
    required this.track,
    required this.dash,
  });

  final int filled;
  final Color fill;
  final Color track;
  final Color dash;

  @override
  void paint(Canvas canvas, Size size) {
    final Offset centre = size.center(Offset.zero);
    final Rect ring = Rect.fromCircle(center: centre, radius: _kRingRadius);
    for (int i = 0; i < kRingSteps; i++) {
      canvas.drawArc(
        ring,
        -math.pi / 2 + i * 2 * math.pi / kRingSteps,
        _kSegmentLength / _kRingRadius,
        false,
        Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = _kSegmentStroke
          ..color = i < filled ? fill : track,
      );
    }
    final Rect inner = Rect.fromCircle(center: centre, radius: _kDashRadius);
    final Paint dashes = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..color = dash;
    for (int i = 0; i < _kDashes; i++) {
      canvas.drawArc(
        inner,
        -math.pi / 2 + i * 2 * math.pi / _kDashes,
        _kDashLength / _kDashRadius,
        false,
        dashes,
      );
    }
  }

  @override
  bool shouldRepaint(_RingPainter oldDelegate) =>
      oldDelegate.filled != filled ||
      oldDelegate.fill != fill ||
      oldDelegate.track != track ||
      oldDelegate.dash != dash;
}
