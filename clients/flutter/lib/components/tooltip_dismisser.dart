import 'package:flutter/material.dart';

class TooltipDismisser extends StatefulWidget {
  const TooltipDismisser({super.key, required this.child});

  final Widget child;

  @override
  State<TooltipDismisser> createState() => _TooltipDismisserState();
}

class _TooltipDismisserState extends State<TooltipDismisser>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeMetrics() {
    Tooltip.dismissAllToolTips();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
