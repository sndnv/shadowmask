import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kTabTextSize = 11;
const double kTabSlant = 16;
const double kTabTracking = 0.16;
const EdgeInsets kTabPadding = EdgeInsets.fromLTRB(12, 8, 30, 7);

TextStyle tabTextStyle(Tokens t, TextScaler scaler) => TextStyle(
  color: t.accentContrast,
  fontSize: kTabTextSize,
  fontWeight: FontWeight.w700,
  letterSpacing: scaler.scale(kTabTextSize) * kTabTracking,
  height: 1,
);

double tabRuleGap(BuildContext context) =>
    context.shapes.cut &&
        Theme.of(context).materialTapTargetSize ==
            MaterialTapTargetSize.shrinkWrap
    ? Space.s1
    : 0;

class TabSlantClipper extends CustomClipper<Path> {
  const TabSlantClipper();

  @override
  Path getClip(Size size) => Path()
    ..addPolygon(<Offset>[
      Offset.zero,
      Offset(size.width - kTabSlant, 0),
      Offset(size.width, size.height),
      Offset(0, size.height),
    ], true);

  @override
  bool shouldReclip(TabSlantClipper oldClipper) => false;
}

class SlantedTab extends StatelessWidget {
  const SlantedTab(
    this.text, {
    super.key,
    this.link,
    this.lit = false,
    this.maxLines,
    this.overflow,
  });

  final String text;
  final String? link;
  final bool lit;
  final int? maxLines;
  final TextOverflow? overflow;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    final String? tail = link;
    return ClipPath(
      clipper: const TabSlantClipper(),
      child: ColoredBox(
        color: t.accent,
        child: Padding(
          padding: kTabPadding,
          child: Text.rich(
            TextSpan(
              text: shapes.caps(text),
              children: <InlineSpan>[
                if (tail != null)
                  TextSpan(
                    text: shapes.caps(tail),
                    style: TextStyle(
                      decoration: lit ? TextDecoration.underline : null,
                      decorationColor: t.accentContrast,
                    ),
                  ),
              ],
            ),
            semanticsLabel: shapes.capsLabel(text + (tail ?? '')),
            maxLines: maxLines,
            overflow: overflow,
            style: tabTextStyle(t, MediaQuery.textScalerOf(context)),
          ),
        ),
      ),
    );
  }
}

class TabHeading extends StatelessWidget {
  const TabHeading(
    this.title, {
    super.key,
    this.count,
    this.style,
    this.maxLines,
    this.overflow,
  });

  final String title;
  final int? count;
  final TextStyle? style;
  final int? maxLines;
  final TextOverflow? overflow;

  @override
  Widget build(BuildContext context) {
    final Shapes shapes = context.shapes;
    final int? total = count;
    if (!shapes.cut) {
      return Text(
        total == null ? title : Strings.countLabel(title, total),
        style: style ?? Theme.of(context).textTheme.headlineMedium,
        maxLines: maxLines,
        overflow: overflow,
      );
    }
    final Widget tab = SlantedTab(
      title,
      maxLines: maxLines,
      overflow: overflow,
    );
    if (total == null) {
      return tab;
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.end,
      children: <Widget>[
        Flexible(child: tab),
        Padding(
          padding: const EdgeInsets.only(left: Space.s3, bottom: 6),
          child: Text(
            '$total',
            style: shapes.mono(
              TextStyle(color: context.tokens.accent, fontSize: 12, height: 1),
            ),
          ),
        ),
      ],
    );
  }
}

class RuledHeading extends StatelessWidget {
  const RuledHeading(this.title, {super.key, this.count});

  final String title;
  final int? count;

  @override
  Widget build(BuildContext context) {
    final Widget heading = TabHeading(title, count: count);
    return TabRule(
      child: context.shapes.cut
          ? Align(alignment: Alignment.bottomLeft, child: heading)
          : heading,
    );
  }
}

class TabRule extends StatelessWidget {
  const TabRule({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final Shapes shapes = context.shapes;
    if (!shapes.cut) {
      return child;
    }
    return SizedBox(
      width: double.infinity,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(
            bottom: BorderSide(color: shapes.line(context.tokens.border)),
          ),
        ),
        child: child,
      ),
    );
  }
}
