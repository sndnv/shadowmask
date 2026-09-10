import 'package:flutter/widgets.dart';

import 'package:shadowmask/util/scoped_value.dart';

class BottomChromeScope extends InheritedWidget {
  const BottomChromeScope({
    super.key,
    required this.height,
    required super.child,
  });

  final ScopedValue<double> height;

  static ScopedValue<double>? of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<BottomChromeScope>()?.height;

  @override
  bool updateShouldNotify(BottomChromeScope old) => old.height != height;
}

class BottomChrome extends StatefulWidget {
  const BottomChrome({super.key, required this.height, required this.child});

  final double height;
  final Widget child;

  @override
  State<BottomChrome> createState() => _BottomChromeState();
}

class _BottomChromeState extends State<BottomChrome> {
  ScopedValue<double>? _scope;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _scope = BottomChromeScope.of(context);
    _publish();
  }

  @override
  void didUpdateWidget(BottomChrome old) {
    super.didUpdateWidget(old);
    if (old.height != widget.height) {
      _publish();
    }
  }

  @override
  void dispose() {
    _scope?.releaseAfterFrame(widget.height, 0, owner: this);
    super.dispose();
  }

  void _publish() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _scope?.publish(widget.height, owner: this);
      }
    });
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
