import 'package:flutter/material.dart';

import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const int kLinkCodeGroup = 4;

String groupedLinkCode(String code) {
  final StringBuffer out = StringBuffer();
  for (int i = 0; i < code.length; i++) {
    if (i > 0 && i % kLinkCodeGroup == 0) {
      out.write(' ');
    }
    out.write(code[i]);
  }
  return out.toString();
}

bool _isDigit(String ch) {
  final int c = ch.codeUnitAt(0);
  return c >= 0x30 && c <= 0x39;
}

class LinkCodeText extends StatelessWidget {
  const LinkCodeText(this.code, {super.key});

  final String code;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: Space.s3,
        vertical: Space.s2,
      ),
      decoration: BoxDecoration(
        color: t.surfaceAlt,
        borderRadius: const BorderRadius.all(Radii.sm),
        border: Border.all(color: t.border),
      ),
      child: Text.rich(
        TextSpan(
          children: <InlineSpan>[
            for (final String ch in groupedLinkCode(code).split(''))
              TextSpan(
                text: ch,
                style: TextStyle(color: _isDigit(ch) ? t.accent : t.text),
              ),
          ],
        ),
        semanticsLabel: code.split('').join(' '),
        style: monoStyle.copyWith(
          fontSize: 22,
          fontWeight: FontWeight.w700,
          letterSpacing: 3,
          height: 1.2,
        ),
      ),
    );
  }
}
