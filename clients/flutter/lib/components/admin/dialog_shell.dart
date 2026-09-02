import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class DialogShell extends StatelessWidget {
  const DialogShell({
    super.key,
    required this.title,
    required this.child,
    this.footer,
    this.onClose,
    this.enableClose = true,
    this.width = 480,
  });

  final String title;
  final Widget child;
  final Widget? footer;
  final VoidCallback? onClose;
  final bool enableClose;
  final double width;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final double maxHeight = MediaQuery.of(context).size.height * 0.7;
    return Dialog(
      backgroundColor: Colors.transparent,
      elevation: 0,
      insetPadding: const EdgeInsets.all(Space.s5),
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: width, maxHeight: maxHeight),
        child: Container(
          clipBehavior: Clip.antiAlias,
          decoration: BoxDecoration(
            color: t.surface,
            borderRadius: const BorderRadius.all(Radii.md),
            border: Border.all(color: t.border),
            boxShadow: <BoxShadow>[
              BoxShadow(
                color: Colors.black.withValues(alpha: 0.5),
                blurRadius: 24,
                offset: const Offset(0, 8),
              ),
            ],
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              _Header(
                title: title,
                onClose: !enableClose
                    ? null
                    : (onClose ?? () => Navigator.of(context).pop()),
              ),
              Flexible(
                child: SingleChildScrollView(
                  padding: const EdgeInsets.all(Space.s4),
                  child: SelectionArea(child: child),
                ),
              ),
              if (footer != null)
                DecoratedBox(
                  decoration: BoxDecoration(
                    border: Border(top: BorderSide(color: t.border)),
                  ),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: Space.s4,
                      vertical: Space.s3,
                    ),
                    child: footer!,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.title, required this.onClose});

  final String title;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border(bottom: BorderSide(color: t.border)),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(Space.s4, 10, Space.s3, 10),
        child: Row(
          children: <Widget>[
            Expanded(
              child: Text(
                title,
                style: TextStyle(
                  color: t.text,
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            const SizedBox(width: Space.s3),
            IconButton(
              onPressed: onClose,
              tooltip: Strings.close,
              icon: const Icon(Icons.close, size: 18),
              color: t.muted,
              hoverColor: t.surfaceAlt,
              padding: const EdgeInsets.all(Space.s1),
              constraints: const BoxConstraints(),
              visualDensity: VisualDensity.compact,
            ),
          ],
        ),
      ),
    );
  }
}
