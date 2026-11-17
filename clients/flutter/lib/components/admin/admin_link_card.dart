import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kAdminLinkCardWidth = 260;

class AdminLinkCard extends StatefulWidget {
  const AdminLinkCard({
    super.key,
    required this.title,
    required this.description,
    required this.icon,
    required this.onTap,
  });

  final String title;
  final String description;
  final IconData icon;
  final VoidCallback onTap;

  @override
  State<AdminLinkCard> createState() => _AdminLinkCardState();
}

class _AdminLinkCardState extends State<AdminLinkCard> {
  bool _hovered = false;
  bool _focused = false;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) => SizedBox(
        width: constraints.maxWidth < Breakpoints.sm
            ? constraints.maxWidth
            : kAdminLinkCardWidth,
        child: _card(context),
      ),
    );
  }

  Widget _card(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    return Material(
      color: shapes.panel(t.surface),
      shape: shapes.border(Corners.hubCard),
      child: InkWell(
        onTap: widget.onTap,
        onHover: (bool hovered) => setState(() => _hovered = hovered),
        onFocusChange: (bool focused) => setState(() => _focused = focused),
        customBorder: shapes.border(Corners.hubCard),
        child: Container(
          padding: const EdgeInsets.all(Space.s4),
          decoration: shapes.box(
            Corners.hubCard,
            side: BorderSide(
              color: _hovered || _focused
                  ? shapes.line(t.border)
                  : shapes.lineSoft(t.border),
            ),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Icon(widget.icon, color: t.accent),
              const SizedBox(width: Space.s3),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      widget.title,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: Space.s1),
                    Text(
                      widget.description,
                      style: Theme.of(
                        context,
                      ).textTheme.bodySmall?.copyWith(color: t.muted),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
