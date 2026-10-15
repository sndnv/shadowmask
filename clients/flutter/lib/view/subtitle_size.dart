import 'package:shadowmask/l10n/strings.dart';

const double kSubtitleHeightShare = 0.045;

enum SubtitleSize {
  deviceDefault('default', 1, Strings.subtitleSizeDefault),
  small('small', 0.8, Strings.subtitleSizeSmall),
  medium('medium', 1, Strings.subtitleSizeMedium),
  large('large', 1.25, Strings.subtitleSizeLarge);

  const SubtitleSize(this.wire, this.factor, this.label);

  final String wire;
  final double factor;
  final String label;

  static SubtitleSize fromWire(String? value) => values.firstWhere(
    (SubtitleSize s) => s.wire == value,
    orElse: () => SubtitleSize.deviceDefault,
  );
}

double subtitleFontSize(double height, SubtitleSize size) =>
    height * kSubtitleHeightShare * size.factor;

String? cueFontSize(SubtitleSize size) => size == SubtitleSize.deviceDefault
    ? null
    : '${(size.factor * 100).round()}%';
