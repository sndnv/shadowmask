import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/admin/log_line.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class LogView extends StatelessWidget {
  const LogView({super.key, required this.entries});

  final List<String> entries;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Container(
      width: double.infinity,
      constraints: const BoxConstraints(maxHeight: 360),
      padding: const EdgeInsets.all(Space.s3),
      decoration: context.shapes.box(
        Corners.inset,
        color: t.bg,
        side: BorderSide(color: context.shapes.lineSoft(t.border)),
      ),
      child: SelectionArea(
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              for (final String entry in entries) LogLine(entry),
            ],
          ),
        ),
      ),
    );
  }
}
