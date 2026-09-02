import 'package:flutter/widgets.dart';
import 'package:flutter_svg/flutter_svg.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const List<String> kPhosphor = <String>['#FF5D5D', '#5BE88A', '#5AA0FF'];

const String kBrandDotOpacity = '0.55';

String brandMarkSvg(String accent, {required bool retro}) {
  final StringBuffer dots = StringBuffer();
  for (int row = 0; row < 5; row++) {
    final double cy = 7 + row * 2.5;
    for (int col = 0; col < 5; col++) {
      final double cx = 5 + col * 3.5;
      final String fill = retro ? kPhosphor[(col + row) % 3] : accent;
      dots.write('<circle cx="$cx" cy="$cy" r="1" fill="$fill"/>');
    }
  }
  return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">'
      '<defs><clipPath id="sm-tube">'
      '<rect x="2" y="4" width="20" height="16" rx="4"/>'
      '</clipPath></defs>'
      '<rect x="2" y="4" width="20" height="16" rx="4" fill="none" '
      'stroke="$accent" stroke-width="1.6"/>'
      '<g clip-path="url(#sm-tube)" fill-opacity="$kBrandDotOpacity">$dots</g>'
      '</svg>';
}

class BrandMark extends StatelessWidget {
  const BrandMark({super.key, this.size = 20});

  final double size;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final bool retro = ThemeScope.of(context).variant == AppThemeVariant.retro;
    return SvgPicture.string(
      brandMarkSvg(_hex(t.accent), retro: retro),
      width: size,
      height: size,
      semanticsLabel: Strings.appName,
    );
  }

  static String _hex(Color c) {
    String channel(double v) =>
        (v * 255.0).round().clamp(0, 255).toRadixString(16).padLeft(2, '0');
    return '#${channel(c.r)}${channel(c.g)}${channel(c.b)}';
  }
}
