import 'package:flutter/material.dart';

import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_ext.dart';

extension TokensContext on BuildContext {
  Tokens get tokens => Theme.of(this).extension<TokensExt>()!.t;
}
