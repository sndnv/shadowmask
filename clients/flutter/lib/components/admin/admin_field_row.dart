import 'package:flutter/material.dart';

import 'package:shadowmask/theme/space.dart';

const double _kMinFlexWidth = 180;
const double _kMinIntrinsicWidth = 120;

class AdminField {
  const AdminField(this.child, {this.flex = 0, this.width});

  final Widget child;
  final int flex;
  final double? width;

  double get _needs {
    if (flex > 0) {
      return _kMinFlexWidth;
    }
    return width ?? _kMinIntrinsicWidth;
  }
}

class AdminFieldRow extends StatelessWidget {
  const AdminFieldRow({
    super.key,
    required this.fields,
    this.crossAxisAlignment = CrossAxisAlignment.end,
  });

  final List<AdminField> fields;
  final CrossAxisAlignment crossAxisAlignment;

  double get _needed {
    double total = 0;
    for (int i = 0; i < fields.length; i++) {
      total += (i > 0 ? Space.s3 : 0) + fields[i]._needs;
    }
    return total;
  }

  @override
  Widget build(BuildContext context) {
    final double needed = _needed;
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth < needed) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              for (int i = 0; i < fields.length; i++) ...<Widget>[
                if (i > 0) const SizedBox(height: Space.s3),
                fields[i].child,
              ],
            ],
          );
        }
        return Row(
          crossAxisAlignment: crossAxisAlignment,
          children: <Widget>[
            for (int i = 0; i < fields.length; i++) ...<Widget>[
              if (i > 0) const SizedBox(width: Space.s3),
              _sized(fields[i]),
            ],
          ],
        );
      },
    );
  }

  Widget _sized(AdminField field) {
    if (field.flex > 0) {
      return Expanded(flex: field.flex, child: field.child);
    }
    final double? width = field.width;
    return width == null
        ? field.child
        : SizedBox(width: width, child: field.child);
  }
}
