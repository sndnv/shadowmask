import 'package:flutter/material.dart';

import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/card_aspect.dart';

const Duration kShimmerDuration = Duration(milliseconds: 1400);
const Curve kShimmerCurve = Cubic(0.2, 0, 0, 1);

const double _kBarHeight = 12;
const double _kBarGap = 9;
const Radius _kBarRadius = Radius.circular(4);
const List<double> _kLineWidths = <double>[1, 0.82, 0.58];

class SkeletonPulse extends StatefulWidget {
  const SkeletonPulse({super.key, required this.child});

  final Widget child;

  static Animation<double>? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<_PulseScope>()?.sweep;

  @override
  State<SkeletonPulse> createState() => _SkeletonPulseState();
}

class _SkeletonPulseState extends State<SkeletonPulse>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: kShimmerDuration,
  )..repeat();

  late final Animation<double> _sweep = CurvedAnimation(
    parent: _controller,
    curve: kShimmerCurve,
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) =>
      _PulseScope(sweep: _sweep, child: widget.child);
}

class _PulseScope extends InheritedWidget {
  const _PulseScope({required this.sweep, required super.child});

  final Animation<double> sweep;

  @override
  bool updateShouldNotify(_PulseScope oldWidget) => sweep != oldWidget.sweep;
}

class Skeleton extends StatefulWidget {
  const Skeleton({
    super.key,
    this.width,
    this.widthFactor,
    this.height = _kBarHeight,
    this.radius = _kBarRadius,
    this.aspectRatio,
  });

  final double? width;
  final double? widthFactor;
  final double height;
  final Radius radius;
  final double? aspectRatio;

  @override
  State<Skeleton> createState() => _SkeletonState();
}

class _SkeletonState extends State<Skeleton>
    with SingleTickerProviderStateMixin {
  AnimationController? _fallback;
  late Animation<double> _sweep;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final Animation<double>? shared = SkeletonPulse.maybeOf(context);
    if (shared != null) {
      _sweep = shared;
      return;
    }
    _fallback ??= AnimationController(vsync: this, duration: kShimmerDuration)
      ..repeat();
    _sweep = CurvedAnimation(parent: _fallback!, curve: kShimmerCurve);
  }

  @override
  void dispose() {
    _fallback?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Color base = t.surfaceAlt;
    final Color highlight = Color.alphaBlend(
      t.muted.withValues(alpha: 0.22),
      base,
    );
    final BorderRadius radius = BorderRadius.all(widget.radius);
    final Widget bar = MediaQuery.disableAnimationsOf(context)
        ? DecoratedBox(
            decoration: BoxDecoration(color: base, borderRadius: radius),
          )
        : AnimatedBuilder(
            animation: _sweep,
            builder: (BuildContext context, Widget? _) {
              final double slide = -1 + 3 * _sweep.value;
              return DecoratedBox(
                decoration: BoxDecoration(
                  borderRadius: radius,
                  gradient: LinearGradient(
                    begin: Alignment(slide - 1, 0),
                    end: Alignment(slide + 1, 0),
                    colors: <Color>[base, highlight, base],
                  ),
                ),
              );
            },
          );
    final double? ratio = widget.aspectRatio;
    final Widget sized = ratio != null
        ? AspectRatio(aspectRatio: ratio, child: bar)
        : SizedBox(width: widget.width, height: widget.height, child: bar);
    final double? factor = widget.widthFactor;
    if (factor == null) {
      return sized;
    }
    return FractionallySizedBox(
      alignment: Alignment.centerLeft,
      widthFactor: factor,
      child: sized,
    );
  }
}

class SkeletonLines extends StatelessWidget {
  const SkeletonLines({super.key, this.lines = 3, this.padded = true});

  final int lines;
  final bool padded;

  @override
  Widget build(BuildContext context) {
    final Widget column = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        for (int i = 0; i < lines; i++)
          Padding(
            padding: EdgeInsets.only(top: i == 0 ? 0 : _kBarGap),
            child: Skeleton(widthFactor: _kLineWidths[i % _kLineWidths.length]),
          ),
      ],
    );
    return SkeletonPulse(
      child: padded
          ? Padding(padding: const EdgeInsets.all(Space.s5), child: column)
          : column,
    );
  }
}

class SkeletonCards extends StatelessWidget {
  const SkeletonCards({
    super.key,
    this.count = 6,
    this.aspect = CardAspect.poster,
  });

  final int count;
  final CardAspect aspect;

  @override
  Widget build(BuildContext context) {
    return SkeletonPulse(
      child: LayoutBuilder(
        builder: (BuildContext context, BoxConstraints constraints) {
          final double width = fittedCardWidth(constraints.maxWidth, aspect);
          return Wrap(
            spacing: Space.s4,
            runSpacing: Space.s4,
            children: <Widget>[
              for (int i = 0; i < count; i++)
                SkeletonCard(aspect: aspect, width: width),
            ],
          );
        },
      ),
    );
  }
}

class SkeletonCard extends StatelessWidget {
  const SkeletonCard({super.key, required this.aspect, this.width});

  final CardAspect aspect;
  final double? width;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SizedBox(
      width: width ?? cardWidthFor(aspect),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: t.surface,
          borderRadius: const BorderRadius.all(Radii.md),
          border: Border.all(color: t.border),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Skeleton(
              aspectRatio: aspect == CardAspect.landscape ? 16 / 9 : 2 / 3,
              radius: Radius.zero,
            ),
            SizedBox(
              height: kCardTextBlockHeight,
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: Space.s2,
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    const Skeleton(widthFactor: 0.82),
                    const Spacer(),
                    Skeleton(
                      widthFactor: 0.42,
                      height: kCardSubtitleBlockHeight - 7,
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class SkeletonRail extends StatelessWidget {
  const SkeletonRail({
    super.key,
    this.count = 5,
    this.aspect = CardAspect.poster,
  });

  final int count;
  final CardAspect aspect;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final double width = fittedCardWidth(constraints.maxWidth, aspect);
        final double art = aspect == CardAspect.landscape
            ? width * 9 / 16
            : width * 3 / 2;
        return SizedBox(
          height: art + kCardTextBlockHeight,
          child: SkeletonPulse(
            child: ListView.separated(
              scrollDirection: Axis.horizontal,
              physics: const NeverScrollableScrollPhysics(),
              itemCount: count,
              separatorBuilder: (BuildContext _, int _) =>
                  const SizedBox(width: Space.s4),
              itemBuilder: (BuildContext _, int _) =>
                  SkeletonCard(aspect: aspect, width: width),
            ),
          ),
        );
      },
    );
  }
}

class SkeletonRows extends StatelessWidget {
  const SkeletonRows({super.key, this.rows = 5});

  final int rows;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SkeletonPulse(
      child: Container(
        clipBehavior: Clip.antiAlias,
        decoration: BoxDecoration(
          color: t.surface,
          borderRadius: const BorderRadius.all(Radii.md),
          border: Border.all(color: t.border),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            DecoratedBox(
              decoration: BoxDecoration(
                color: t.surfaceAlt,
                border: Border(bottom: BorderSide(color: t.border)),
              ),
              child: const SizedBox(
                height: 34,
                child: Padding(
                  padding: EdgeInsets.symmetric(horizontal: Space.s3),
                  child: Align(child: Skeleton(widthFactor: 0.3, height: 8)),
                ),
              ),
            ),
            for (int i = 0; i < rows; i++)
              DecoratedBox(
                decoration: BoxDecoration(
                  border: i == rows - 1
                      ? null
                      : Border(bottom: BorderSide(color: t.border)),
                ),
                child: SizedBox(
                  height: 36,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: Space.s3),
                    child: Align(
                      child: Skeleton(
                        widthFactor: _kLineWidths[i % _kLineWidths.length],
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class SkeletonDetail extends StatelessWidget {
  const SkeletonDetail({super.key, this.aspect = CardAspect.poster});

  final CardAspect aspect;

  @override
  Widget build(BuildContext context) {
    return SkeletonPulse(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          SizedBox(
            width: cardWidthFor(aspect),
            child: Skeleton(
              aspectRatio: aspect == CardAspect.landscape ? 16 / 9 : 2 / 3,
              radius: Radii.md,
            ),
          ),
          const SizedBox(width: Space.s5),
          const Expanded(
            child: Padding(
              padding: EdgeInsets.only(top: Space.s2),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Skeleton(widthFactor: 0.55, height: 24),
                  SizedBox(height: Space.s4),
                  SkeletonLines(lines: 5, padded: false),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class SkeletonPage extends StatelessWidget {
  const SkeletonPage({super.key, required this.child, this.toolbar = false});

  final Widget child;
  final bool toolbar;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SkeletonPulse(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          const Padding(
            padding: EdgeInsets.only(bottom: Space.s3),
            child: Skeleton(width: 220, height: 13),
          ),
          if (toolbar)
            Padding(
              padding: const EdgeInsets.only(bottom: Space.s4),
              child: Row(
                children: <Widget>[
                  for (int i = 0; i < 3; i++)
                    Padding(
                      padding: const EdgeInsets.only(right: Space.s4),
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          borderRadius: const BorderRadius.all(Radii.sm),
                          border: Border.all(color: t.border),
                        ),
                        child: const SizedBox(
                          width: 120,
                          height: 34,
                          child: Padding(
                            padding: EdgeInsets.symmetric(horizontal: Space.s3),
                            child: Align(child: Skeleton(height: 9)),
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ),
          child,
        ],
      ),
    );
  }
}
