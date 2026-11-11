import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double _kHelpIconSize = 16;

const double kFieldHeadingHeight = _kHelpIconSize + Space.s1 * 2;

class FieldHelp extends StatelessWidget {
  const FieldHelp({
    super.key,
    required this.title,
    required this.body,
    this.example,
  });

  final String title;
  final String body;
  final String? example;

  void _open(BuildContext context) {
    final String? shown = example;
    showDialog<void>(
      context: context,
      builder: (BuildContext ctx) {
        final Text text = Text(
          body,
          style: Theme.of(
            ctx,
          ).textTheme.bodyMedium?.copyWith(color: ctx.tokens.text),
        );
        return DialogShell(
          title: title,
          width: 420,
          child: shown == null
              ? text
              : Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  spacing: Space.s3,
                  children: <Widget>[text, _Example(shown)],
                ),
        );
      },
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

class _Example extends StatelessWidget {
  const _Example(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Wrap(
      spacing: Space.s2,
      runSpacing: Space.s1,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: <Widget>[
        Text(
          Strings.forExample,
          style: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(color: t.muted),
        ),
        DecoratedBox(
          decoration: BoxDecoration(
            color: t.surfaceAlt,
            border: Border.all(color: t.border),
            borderRadius: const BorderRadius.all(Radii.sm),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(
              horizontal: Space.s2,
              vertical: Space.s1,
            ),
            child: Text(
              text,
              style: monoStyle.copyWith(
                color: t.text,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class FieldLabel extends StatelessWidget {
  const FieldLabel({
    super.key,
    required this.label,
    this.help,
    this.example,
    this.child,
  });

  final String label;
  final String? help;
  final String? example;
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
            FieldHelp(title: label, body: body, example: example),
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
