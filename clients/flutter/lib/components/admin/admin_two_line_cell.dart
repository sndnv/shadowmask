import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/tokens_context.dart';

class AdminTwoLineCell extends StatelessWidget {
  const AdminTwoLineCell({super.key, required this.first, this.below});

  AdminTwoLineCell.text(String text, {super.key, this.below})
    : first = Text(text, maxLines: 1, overflow: TextOverflow.ellipsis);

  final Widget first;
  final String? below;

  @override
  Widget build(BuildContext context) {
    final String? line = below;
    if (line == null) {
      return first;
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        first,
        Text(
          line,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(color: context.tokens.muted),
        ),
      ],
    );
  }
}
