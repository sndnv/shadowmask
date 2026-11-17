import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/tab_heading.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';

class SectionHeading extends StatelessWidget {
  const SectionHeading({
    super.key,
    required this.title,
    this.count,
    this.trailing,
  });

  final String title;
  final int? count;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    if (trailing == null) {
      return RuledHeading(title, count: count);
    }
    return TabRule(
      child: Row(
        crossAxisAlignment: context.shapes.cut
            ? CrossAxisAlignment.end
            : CrossAxisAlignment.center,
        children: <Widget>[
          Flexible(child: TabHeading(title, count: count)),
          const SizedBox(width: Space.s2),
          Padding(
            padding: EdgeInsets.only(bottom: tabRuleGap(context)),
            child: trailing,
          ),
        ],
      ),
    );
  }
}
