import 'package:flutter/material.dart';

import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class DetailSplit extends StatelessWidget {
  const DetailSplit({
    super.key,
    required this.poster,
    required this.info,
    this.posterWidth = 220,
  });

  final Widget poster;
  final Widget info;
  final double posterWidth;

  Widget _framedPoster(BuildContext context, double width) {
    final Tokens t = context.tokens;
    return Container(
      width: width,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        border: Border.all(color: t.border),
        borderRadius: const BorderRadius.all(Radii.md),
      ),
      child: poster,
    );
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth < Breakpoints.sm) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              _framedPoster(context, constraints.constrainWidth(posterWidth)),
              const SizedBox(height: Space.s4),
              info,
            ],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            _framedPoster(context, posterWidth),
            const SizedBox(width: Space.s5),
            Expanded(child: info),
          ],
        );
      },
    );
  }
}
