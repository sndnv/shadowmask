import 'package:flutter/material.dart';

import 'package:shadowmask/theme/space.dart';

class LongMenuItem extends StatelessWidget {
  const LongMenuItem({
    super.key,
    required this.label,
    required this.onPressed,
    this.leadingIcon,
  });

  final String label;
  final VoidCallback? onPressed;
  final Widget? leadingIcon;

  @override
  Widget build(BuildContext context) => Tooltip(
    message: label,
    child: ConstrainedBox(
      constraints: BoxConstraints(
        maxWidth: MediaQuery.sizeOf(context).width - 2 * Space.s2,
      ),
      child: MenuItemButton(
        onPressed: onPressed,
        leadingIcon: leadingIcon,
        child: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
      ),
    ),
  );
}
