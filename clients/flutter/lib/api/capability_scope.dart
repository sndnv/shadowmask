import 'package:flutter/widgets.dart';

import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shadowmask/model/session/client_decoding.dart';

class CapabilityScope extends InheritedWidget {
  const CapabilityScope({
    super.key,
    required this.platform,
    required this.measured,
    this.overrides = const CapabilityOverrides(),
    this.setOverrides,
    required super.child,
  });

  final String platform;
  final ClientDecoding? measured;
  final CapabilityOverrides overrides;
  final ValueChanged<CapabilityOverrides>? setOverrides;

  ClientDecoding? get decoding => overrides.applyTo(measured);

  static CapabilityScope? of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<CapabilityScope>();

  @override
  bool updateShouldNotify(CapabilityScope oldWidget) =>
      platform != oldWidget.platform ||
      measured != oldWidget.measured ||
      overrides != oldWidget.overrides;
}
