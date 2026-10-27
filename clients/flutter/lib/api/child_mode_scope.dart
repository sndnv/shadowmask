import 'package:flutter/widgets.dart';

import 'package:shadowmask/l10n/strings.dart';

const List<int> kChildModeAges = <int>[0, 8, 13, 17];

int? offeredChildAge(int? stored) {
  if (stored == null || stored < 0) {
    return null;
  }
  return kChildModeAges.lastWhere((int age) => age <= stored);
}

String childModeLabel(int age) {
  final int index = kChildModeAges.indexOf(age);
  final int? next = index >= 0 && index + 1 < kChildModeAges.length
      ? kChildModeAges[index + 1]
      : null;
  return Strings.childModeRange(age, next);
}

String childModeToast(int? age) =>
    Strings.toastChildMode(age == null ? null : childModeLabel(age));

class ChildModeScope extends InheritedWidget {
  const ChildModeScope({
    super.key,
    required this.age,
    required this.setAge,
    required super.child,
  });

  final int? age;
  final ValueChanged<int?> setAge;

  static ChildModeScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<ChildModeScope>();

  @override
  bool updateShouldNotify(ChildModeScope oldWidget) => age != oldWidget.age;
}
