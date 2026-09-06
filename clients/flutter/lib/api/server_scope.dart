import 'package:flutter/widgets.dart';

class ServerScope extends InheritedWidget {
  const ServerScope({
    super.key,
    required this.address,
    required this.setAddress,
    required this.probe,
    required super.child,
  });

  final String? address;
  final Future<void> Function(String address) setAddress;
  final Future<bool> Function(String address) probe;

  static ServerScope? of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<ServerScope>();

  @override
  bool updateShouldNotify(ServerScope oldWidget) =>
      address != oldWidget.address;
}
