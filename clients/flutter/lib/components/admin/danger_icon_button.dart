import 'package:flutter/material.dart';

class DangerIconButton extends StatelessWidget {
  const DangerIconButton({
    super.key,
    required this.icon,
    required this.tooltip,
    required this.onPressed,
    this.compact = false,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;
  final bool compact;

  @override
  Widget build(BuildContext context) => IconButton(
    tooltip: tooltip,
    onPressed: onPressed,
    color: Theme.of(context).colorScheme.error,
    visualDensity: compact ? VisualDensity.compact : null,
    icon: Icon(icon),
  );
}
