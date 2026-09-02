import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/view/player_shortcuts.dart';

const Duration _kPreviewLinger = Duration(milliseconds: 900);

class Timeline extends StatefulWidget {
  const Timeline({
    super.key,
    required this.fraction,
    required this.bufferedFraction,
    required this.durationMs,
    required this.onSeek,
    this.thumbBuilder,
  });

  final double fraction;
  final double bufferedFraction;
  final int durationMs;
  final void Function(double fraction) onSeek;
  final Widget Function(int positionMs)? thumbBuilder;

  @override
  State<Timeline> createState() => _TimelineState();
}

class _TimelineState extends State<Timeline> {
  double? _scrub;
  bool _hovering = false;
  Timer? _linger;

  @override
  void dispose() {
    _linger?.cancel();
    super.dispose();
  }

  void _showAt(double fraction) {
    setState(() => _scrub = fraction);
    _linger?.cancel();
    if (_hovering) {
      return;
    }
    _linger = Timer(_kPreviewLinger, () {
      if (mounted) {
        setState(() => _scrub = null);
      }
    });
  }

  void _hide() {
    _linger?.cancel();
    if (_scrub != null) {
      setState(() => _scrub = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final double width = constraints.maxWidth;
        double fractionAt(double dx) => (dx / width).clamp(0.0, 1.0);
        void preview(double dx) => _showAt(fractionAt(dx));
        void clear() => _hide();
        final double shown = _scrub ?? widget.fraction;
        final int scrubMs = (shown * widget.durationMs).round();
        return Semantics(
          slider: true,
          label: Strings.timelineLabel,
          value: _reading(widget.fraction),
          increasedValue: _reading(_stepped(1)),
          decreasedValue: _reading(_stepped(-1)),
          onIncrease: () => widget.onSeek(_stepped(1)),
          onDecrease: () => widget.onSeek(_stepped(-1)),
          child: MouseRegion(
            onEnter: (_) {
              _hovering = true;
              _linger?.cancel();
            },
            onHover: (PointerHoverEvent e) => preview(e.localPosition.dx),
            onExit: (_) {
              _hovering = false;
              clear();
            },
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTapDown: (TapDownDetails d) => preview(d.localPosition.dx),
              onTapUp: (TapUpDetails d) {
                widget.onSeek(fractionAt(d.localPosition.dx));
                clear();
              },
              onTapCancel: clear,
              onHorizontalDragUpdate: (DragUpdateDetails d) =>
                  preview(d.localPosition.dx),
              onHorizontalDragEnd: (_) {
                final double? at = _scrub;
                if (at != null) {
                  widget.onSeek(at);
                }
                clear();
              },
              onHorizontalDragCancel: clear,
              child: SizedBox(
                height: 28,
                child: Stack(
                  clipBehavior: Clip.none,
                  alignment: Alignment.centerLeft,
                  children: <Widget>[
                    CustomPaint(
                      size: Size(width, 28),
                      painter: _TrackPainter(
                        fraction: shown,
                        buffered: widget.bufferedFraction,
                        track: Colors.white.withValues(alpha: 0.28),
                        bufferedColor: Colors.white.withValues(alpha: 0.45),
                        fill: t.accent,
                      ),
                    ),
                    if (_scrub != null && widget.thumbBuilder != null)
                      Positioned(
                        left: (shown * width) - 84,
                        bottom: 34,
                        child: IgnorePointer(
                          child: widget.thumbBuilder!(scrubMs),
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  double _stepped(int direction) {
    if (widget.durationMs <= 0) {
      return widget.fraction;
    }
    final double step = kSeekStepMs / widget.durationMs;
    return (widget.fraction + step * direction).clamp(0.0, 1.0);
  }

  String _reading(double fraction) =>
      clock((fraction * widget.durationMs).round());
}

class _TrackPainter extends CustomPainter {
  _TrackPainter({
    required this.fraction,
    required this.buffered,
    required this.track,
    required this.bufferedColor,
    required this.fill,
  });

  final double fraction;
  final double buffered;
  final Color track;
  final Color bufferedColor;
  final Color fill;

  @override
  void paint(Canvas canvas, Size size) {
    final double y = size.height / 2;
    final Paint paint = Paint()
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 4;
    canvas.drawLine(Offset(0, y), Offset(size.width, y), paint..color = track);
    if (buffered > 0) {
      canvas.drawLine(
        Offset(0, y),
        Offset(size.width * buffered.clamp(0.0, 1.0), y),
        paint..color = bufferedColor,
      );
    }
    final double x = size.width * fraction.clamp(0.0, 1.0);
    canvas.drawLine(Offset(0, y), Offset(x, y), paint..color = fill);
    canvas.drawCircle(
      Offset(x, y),
      9,
      Paint()..color = const Color(0x4D000000),
    );
    canvas.drawCircle(Offset(x, y), 6, Paint()..color = fill);
  }

  @override
  bool shouldRepaint(_TrackPainter oldDelegate) =>
      oldDelegate.fraction != fraction || oldDelegate.buffered != buffered;
}
