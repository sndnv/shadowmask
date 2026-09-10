import 'package:flutter/material.dart';

bool touchPlatform(BuildContext context) {
  final TargetPlatform platform = Theme.of(context).platform;
  return platform == TargetPlatform.android || platform == TargetPlatform.iOS;
}
