import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

const Duration kEdgeScroll = Duration(milliseconds: 240);
const double kPageKeyOverlap = 0.9;

bool isTyping() {
  final BuildContext? focused = FocusManager.instance.primaryFocus?.context;
  if (focused == null) {
    return false;
  }
  return focused.widget is EditableText ||
      focused.findAncestorWidgetOfExactType<EditableText>() != null;
}

double? scrollTargetFor(LogicalKeyboardKey key, ScrollPosition at) {
  if (key == LogicalKeyboardKey.home) {
    return at.minScrollExtent;
  }
  if (key == LogicalKeyboardKey.end) {
    return at.maxScrollExtent;
  }
  final double page = at.viewportDimension * kPageKeyOverlap;
  if (key == LogicalKeyboardKey.pageUp) {
    return (at.pixels - page).clamp(at.minScrollExtent, at.maxScrollExtent);
  }
  if (key == LogicalKeyboardKey.pageDown) {
    return (at.pixels + page).clamp(at.minScrollExtent, at.maxScrollExtent);
  }
  return null;
}
