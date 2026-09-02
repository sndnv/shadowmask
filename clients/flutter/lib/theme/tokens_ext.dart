import 'package:flutter/material.dart';

import 'package:shadowmask/theme/tokens.dart';

class TokensExt extends ThemeExtension<TokensExt> {
  const TokensExt(this.t);

  final Tokens t;

  @override
  TokensExt copyWith({Tokens? t}) => TokensExt(t ?? this.t);

  @override
  TokensExt lerp(ThemeExtension<TokensExt>? other, double v) => this;
}
