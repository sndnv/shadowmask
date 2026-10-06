import 'package:flutter/material.dart';

import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/util/format.dart';

List<InlineSpan> versionLabelSpans(
  Tokens t,
  Version v,
  String number, {
  bool available = true,
}) => <InlineSpan>[
  TextSpan(
    text: '$number · ',
    style: TextStyle(color: t.muted),
  ),
  if (v.durationMs > 0) TextSpan(text: '${durationText(v.durationMs)} · '),
  TextSpan(
    text: v.quality.label,
    style: TextStyle(color: available ? t.accent : t.muted),
  ),
  TextSpan(text: ' · ${v.container}'),
  if (v.sizeBytes > 0) TextSpan(text: ' · ${gigabytes(v.sizeBytes)}'),
];
