import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/tab_heading.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class SectionHeaderHeight extends InheritedWidget {
  const SectionHeaderHeight({
    super.key,
    required this.minHeight,
    required super.child,
  });

  final double minHeight;

  static double of(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<SectionHeaderHeight>()
          ?.minHeight ??
      0;

  @override
  bool updateShouldNotify(SectionHeaderHeight old) =>
      old.minHeight != minHeight;
}

class SectionBlock extends StatelessWidget {
  const SectionBlock({
    super.key,
    required this.title,
    required this.child,
    this.actions = const <Widget>[],
    this.actionItems = const <PageAction>[],
    this.framed = true,
  });

  final String title;
  final Widget child;
  final List<Widget> actions;
  final List<PageAction> actionItems;
  final bool framed;

  bool get _hasActions => actions.isNotEmpty || actionItems.isNotEmpty;

  Widget _header(BuildContext context, double width, bool narrow) => TabRule(
    child: Wrap(
      alignment: WrapAlignment.spaceBetween,
      crossAxisAlignment: context.shapes.cut
          ? WrapCrossAlignment.end
          : WrapCrossAlignment.center,
      runAlignment: context.shapes.cut
          ? WrapAlignment.end
          : WrapAlignment.center,
      spacing: Space.s3,
      runSpacing: Space.s2,
      children: <Widget>[
        ConstrainedBox(
          constraints: BoxConstraints(maxWidth: width),
          child: TabHeading(
            title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        if (_hasActions)
          Padding(
            padding: EdgeInsets.only(bottom: tabRuleGap(context)),
            child: Wrap(
              spacing: Space.s2,
              runSpacing: Space.s2,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: <Widget>[
                ...actions,
                ...pageActionButtons(context, actionItems, iconOnly: narrow),
              ],
            ),
          ),
      ],
    ),
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        ConstrainedBox(
          constraints: BoxConstraints(
            minHeight: SectionHeaderHeight.of(context),
          ),
          child: LayoutBuilder(
            builder: (BuildContext context, BoxConstraints constraints) =>
                SizedBox(
                  width: constraints.maxWidth,
                  child: _header(
                    context,
                    constraints.maxWidth,
                    constraints.maxWidth < Breakpoints.sm ||
                        compactViewport(context),
                  ),
                ),
          ),
        ),
        const SizedBox(height: Space.s3),
        if (framed)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(Space.s4),
            decoration: context.shapes.box(
              Corners.panel,
              color: context.shapes.panel(t.surface),
              side: BorderSide(color: context.shapes.line(t.border)),
            ),
            child: child,
          )
        else
          SizedBox(width: double.infinity, child: child),
        const SizedBox(height: Space.s6),
      ],
    );
  }
}
