import 'package:flutter/material.dart';

import 'package:shadowmask/components/hex_texture.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class BarePage extends StatelessWidget {
  const BarePage({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Scaffold(
      backgroundColor: t.bg,
      body: Stack(
        children: <Widget>[
          const Positioned.fill(child: HexTexture()),
          Positioned.fill(child: child),
        ],
      ),
    );
  }
}
