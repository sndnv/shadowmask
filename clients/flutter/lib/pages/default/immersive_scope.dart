import 'package:flutter/widgets.dart';

import 'package:shadowmask/util/scoped_value.dart';

class ImmersiveScope extends InheritedWidget {
  const ImmersiveScope({
    super.key,
    required this.immersive,
    required super.child,
  });

  final ScopedValue<bool> immersive;

  static ScopedValue<bool>? of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<ImmersiveScope>()?.immersive;

  @override
  bool updateShouldNotify(ImmersiveScope old) => old.immersive != immersive;
}
