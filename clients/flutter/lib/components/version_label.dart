import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/util/format.dart';

String _asWritten(String text) => text;

List<InlineSpan> versionLabelSpans(
  Tokens t,
  Version v,
  String number, {
  bool available = true,
  String Function(String) caps = _asWritten,
}) => <InlineSpan>[
  TextSpan(
    text: '$number · ',
    style: TextStyle(color: t.muted),
  ),
  if (v.durationMs > 0)
    TextSpan(text: caps('${durationText(v.durationMs)} · ')),
  TextSpan(
    text: caps(v.quality.label),
    style: TextStyle(color: available ? t.accent : t.muted),
  ),
  TextSpan(text: caps(' · ${v.container}')),
  if (v.sizeBytes > 0) TextSpan(text: caps(' · ${gigabytes(v.sizeBytes)}')),
];

String versionLabelText(Version v, String number) => <String>[
  number,
  if (v.durationMs > 0) durationText(v.durationMs),
  v.quality.label,
  v.container,
  if (v.sizeBytes > 0) gigabytes(v.sizeBytes),
].join(' · ');
