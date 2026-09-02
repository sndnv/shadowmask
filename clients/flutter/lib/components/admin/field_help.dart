import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double _kHelpIconSize = 16;

const double kFieldHeadingHeight = _kHelpIconSize + Space.s1 * 2;

class FieldHelp extends StatelessWidget {
  const FieldHelp({super.key, required this.title, required this.body});

  final String title;
  final String body;

  void _open(BuildContext context) {
    showDialog<void>(
      context: context,
      builder: (BuildContext ctx) => DialogShell(
        title: title,
        width: 420,
        child: Text(
          body,
          style: Theme.of(
            ctx,
          ).textTheme.bodyMedium?.copyWith(color: ctx.tokens.text),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Tooltip(
      message: Strings.whatIsThis,
      child: InkWell(
        onTap: () => _open(context),
        borderRadius: const BorderRadius.all(Radii.pill),
        child: Padding(
          padding: const EdgeInsets.all(Space.s1),
          child: Icon(Icons.help_outline, size: _kHelpIconSize, color: t.muted),
        ),
      ),
    );
  }
}

class FieldLabel extends StatelessWidget {
  const FieldLabel({super.key, required this.label, this.help, this.child});

  final String label;
  final String? help;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final String? body = help;
    final Widget heading = SizedBox(
      height: kFieldHeadingHeight,
      child: Row(
        children: <Widget>[
          Flexible(
            child: Text(
              label,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: t.muted,
                fontSize: 12,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          if (body != null) ...<Widget>[
            const SizedBox(width: Space.s1),
            FieldHelp(title: label, body: body),
          ],
        ],
      ),
    );
    final Widget? field = child;
    if (field == null) {
      return heading;
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        heading,
        const SizedBox(height: Space.s2),
        field,
      ],
    );
  }
}
