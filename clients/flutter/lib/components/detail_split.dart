import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/edged_box.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class DetailSplit extends StatelessWidget {
  const DetailSplit({
    super.key,
    required this.poster,
    required this.info,
    this.headline,
    this.actions,
    this.posterWidth = 220,
    double? compactPosterWidth,
  }) : compactPosterWidth = compactPosterWidth ?? posterWidth;

  final Widget poster;
  final Widget info;
  final Widget? headline;
  final Widget? actions;
  final double posterWidth;
  final double compactPosterWidth;

  Widget _framedPoster(BuildContext context, double width) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    return EdgedBox(
      width: width,
      corner: Corners.card,
      edge: BorderSide(color: shapes.lineSoft(t.border)),
      inset: false,
      clipBehavior: shapes.cut ? Clip.none : Clip.antiAlias,
      child: poster,
    );
  }

  Widget _stacked(BuildContext context, BoxConstraints constraints) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: <Widget>[
      _framedPoster(context, constraints.constrainWidth(posterWidth)),
      const SizedBox(height: Space.s4),
      info,
    ],
  );

  Widget _compact(BuildContext context, BoxConstraints constraints) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: <Widget>[
      _framedPoster(context, constraints.constrainWidth(compactPosterWidth)),
      const SizedBox(height: Space.s4),
      headline!,
      if (actions != null) ...<Widget>[
        const SizedBox(height: Space.s4),
        actions!,
      ],
      const SizedBox(height: Space.s4),
      info,
    ],
  );

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth < Breakpoints.sm) {
          return headline == null
              ? _stacked(context, constraints)
              : _compact(context, constraints);
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            _framedPoster(context, posterWidth),
            const SizedBox(width: Space.s5),
            Expanded(
              child: headline == null
                  ? info
                  : Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        headline!,
                        const SizedBox(height: Space.s3),
                        info,
                        if (actions != null) ...<Widget>[
                          const SizedBox(height: Space.s4),
                          actions!,
                        ],
                      ],
                    ),
            ),
          ],
        );
      },
    );
  }
}
