import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class Pagination extends StatelessWidget {
  const Pagination({
    super.key,
    required this.basePath,
    required this.params,
    required this.total,
    required this.offset,
    required this.limit,
    required this.count,
    this.onOffset,
  });

  final String basePath;
  final Map<String, String?> params;
  final int total;
  final int offset;
  final int limit;
  final int count;
  final void Function(int offset)? onOffset;

  void _go(BuildContext context, int newOffset) {
    final void Function(int offset)? inPlace = onOffset;
    if (inPlace != null) {
      inPlace(newOffset);
      return;
    }
    Navigator.of(context).pushReplacementNamed(
      withQuery(basePath, <String, String?>{
        ...params,
        'offset': newOffset <= 0 ? null : newOffset.toString(),
      }),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (total == 0) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    final int end = offset + count;
    final bool hasPrev = offset > 0;
    final bool hasNext = end < total;
    final int from = offset + 1;
    return SelectionContainer.disabled(
      child: Padding(
        padding: const EdgeInsets.only(top: Space.s4),
        child: Center(
          child: Wrap(
            alignment: WrapAlignment.center,
            crossAxisAlignment: WrapCrossAlignment.center,
            runSpacing: Space.s2,
            children: <Widget>[
              _PagerButton(
                label: Strings.previous,
                onPressed: hasPrev
                    ? () => _go(context, (offset - limit).clamp(0, offset))
                    : null,
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: Space.s4),
                child: Text(
                  Strings.pagerRange(from, end, total),
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: t.muted,
                    fontFeatures: const <FontFeature>[
                      FontFeature.tabularFigures(),
                    ],
                  ),
                ),
              ),
              _PagerButton(
                label: Strings.next,
                onPressed: hasNext ? () => _go(context, offset + limit) : null,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _PagerButton extends StatelessWidget {
  const _PagerButton({required this.label, required this.onPressed});

  final String label;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return OutlinedButton(
      onPressed: onPressed,
      style: OutlinedButton.styleFrom(
        backgroundColor: t.surface,
        foregroundColor: t.text,
        disabledForegroundColor: t.muted,
        side: BorderSide(color: t.border),
        textStyle: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
      ),
      child: Text(label),
    );
  }
}
