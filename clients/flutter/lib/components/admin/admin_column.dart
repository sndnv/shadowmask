import 'package:flutter/widgets.dart';

typedef AdminCell<T> = Widget Function(BuildContext context, T row);
typedef AdminSortKey<T> = Comparable<dynamic> Function(T row);

enum AdminColumnSize {
  small(3),
  medium(4),
  large(6);

  const AdminColumnSize(this.flex);

  final int flex;
}

enum AdminColumnAlign { start, end }

class AdminColumn<T> {
  const AdminColumn({
    required this.label,
    required this.cell,
    this.size = AdminColumnSize.medium,
    this.fixedWidth,
    this.align = AdminColumnAlign.start,
    this.sortKey,
    this.essential = false,
  });

  final String label;
  final AdminCell<T> cell;
  final AdminColumnSize size;
  final double? fixedWidth;
  final AdminColumnAlign align;
  final AdminSortKey<T>? sortKey;
  final bool essential;
}
