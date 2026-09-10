import 'package:flutter/material.dart';

import 'package:shadowmask/theme/breakpoints.dart';

class EntryRow extends StatelessWidget {
  const EntryRow({
    super.key,
    required this.label,
    required this.action,
    this.meta,
  });

  final Widget label;
  final Widget action;
  final Widget? meta;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (BuildContext context, BoxConstraints constraints) {
      if (constraints.maxWidth >= Breakpoints.sm) {
        return Row(
          children: <Widget>[
            Expanded(child: label),
            if (meta != null) Expanded(child: meta!),
            action,
          ],
        );
      }
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          label,
          Row(
            children: <Widget>[
              if (meta != null) Expanded(child: meta!) else const Spacer(),
              action,
            ],
          ),
        ],
      );
    },
  );
}
