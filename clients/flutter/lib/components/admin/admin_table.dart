import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kTableBorder = 1;

class AdminTable<T> extends StatefulWidget {
  const AdminTable({
    super.key,
    required this.columns,
    required this.rows,
    required this.emptyText,
    this.onRowTap,
    this.rowLabel,
    this.rowColor,
    this.minWidth = 720,
    this.initialSortColumn,
    this.initialSortAscending = true,
  });

  final List<AdminColumn<T>> columns;
  final List<T> rows;
  final String emptyText;
  final void Function(T row)? onRowTap;
  final String? rowLabel;
  final Color? Function(T row)? rowColor;
  final double minWidth;
  final int? initialSortColumn;
  final bool initialSortAscending;

  @override
  State<AdminTable<T>> createState() => _AdminTableState<T>();
}

class _AdminTableState<T> extends State<AdminTable<T>> {
  late int? _sortColumn = widget.initialSortColumn;
  late bool _ascending = widget.initialSortAscending;
  List<T>? _cache;
  int? _cacheColumn;

  @override
  void didUpdateWidget(covariant AdminTable<T> oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.rows, widget.rows)) {
      _cache = null;
    }
  }

  void _onSort(int index) {
    setState(() {
      if (_sortColumn == index) {
        _ascending = !_ascending;
      } else {
        _sortColumn = index;
        _ascending = true;
      }
      _cache = null;
    });
  }

  List<int> _visible(bool narrow) {
    final List<int> all = <int>[
      for (int i = 0; i < widget.columns.length; i++) i,
    ];
    if (!narrow) {
      return all;
    }
    final List<int> essential = all
        .where((int i) => widget.columns[i].essential)
        .toList();
    return essential.isEmpty ? all : essential;
  }

  int? _sortTarget(List<int> visible) {
    final int? current = _sortColumn;
    if (current == null || visible.contains(current)) {
      return current;
    }
    for (final int i in visible) {
      if (widget.columns[i].sortKey != null) {
        return i;
      }
    }
    return null;
  }

  List<T> _sorted(int? index) {
    final List<T>? cached = _cache;
    if (cached != null && _cacheColumn == index) {
      return cached;
    }
    final List<T> rows = List<T>.of(widget.rows);
    final AdminSortKey<T>? key = index == null
        ? null
        : widget.columns[index].sortKey;
    if (key != null) {
      rows.sort((T a, T b) {
        final int c = key(a).compareTo(key(b));
        return _ascending ? c : -c;
      });
    }
    _cache = rows;
    _cacheColumn = index;
    return rows;
  }

  @override
  Widget build(BuildContext context) {
    if (widget.rows.isEmpty) {
      return StatusText(widget.emptyText);
    }
    final Tokens t = context.tokens;
    return SelectionContainer.disabled(
      child: LayoutBuilder(
        builder: (BuildContext context, BoxConstraints constraints) {
          final double inner = math.max(
            0,
            constraints.maxWidth - kTableBorder * 2,
          );
          final bool narrow = inner < Breakpoints.sm;
          final List<int> visible = _visible(narrow);
          final int? sortColumn = _sortTarget(visible);
          final List<T> rows = _sorted(sortColumn);
          final double width = narrow
              ? inner
              : math.max(inner, widget.minWidth);
          return Container(
            clipBehavior: Clip.antiAlias,
            decoration: BoxDecoration(
              color: t.surface,
              borderRadius: const BorderRadius.all(Radii.md),
              border: Border.all(color: t.border, width: kTableBorder),
            ),
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              physics: narrow ? const NeverScrollableScrollPhysics() : null,
              child: SizedBox(
                width: width,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    _header(t, visible, sortColumn),
                    for (int i = 0; i < rows.length; i++)
                      _row(
                        context,
                        t,
                        rows[i],
                        visible,
                        last: i == rows.length - 1,
                      ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _header(Tokens t, List<int> visible, int? sortColumn) {
    final TextStyle style = TextStyle(
      color: t.muted,
      fontSize: 11,
      fontWeight: FontWeight.w600,
      letterSpacing: 0.5,
    );
    return DecoratedBox(
      decoration: BoxDecoration(
        color: t.surfaceAlt,
        border: Border(bottom: BorderSide(color: t.border)),
      ),
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 34),
        child: Row(
          children: <Widget>[
            for (final int i in visible)
              _slot(widget.columns[i], _headerContent(t, style, i, sortColumn)),
          ],
        ),
      ),
    );
  }

  Widget _headerContent(Tokens t, TextStyle style, int index, int? sortColumn) {
    final AdminColumn<T> col = widget.columns[index];
    final Text label = Text(
      col.label.toUpperCase(),
      style: style,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    );
    if (col.sortKey == null) {
      return label;
    }
    return InkWell(
      onTap: () => _onSort(index),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Flexible(child: label),
          if (sortColumn == index) ...<Widget>[
            const SizedBox(width: Space.s1),
            Icon(
              _ascending ? Icons.arrow_upward : Icons.arrow_downward,
              size: 14,
              color: t.muted,
            ),
          ],
        ],
      ),
    );
  }

  Widget _row(
    BuildContext context,
    Tokens t,
    T row,
    List<int> visible, {
    required bool last,
  }) {
    final void Function(T row)? tap = widget.onRowTap;
    return _TableRow(
      tint: widget.rowColor?.call(row),
      last: last,
      label: widget.rowLabel,
      onTap: tap == null ? null : () => tap(row),
      children: <Widget>[
        for (final int i in visible)
          _slot(widget.columns[i], widget.columns[i].cell(context, row)),
      ],
    );
  }

  Widget _slot(AdminColumn<T> col, Widget child) {
    final Widget inner = col.align == AdminColumnAlign.end
        ? Align(alignment: Alignment.centerRight, child: child)
        : child;
    final Widget padded = Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: Space.s3,
        vertical: Space.s1 + 2,
      ),
      child: inner,
    );
    return col.fixedWidth != null
        ? SizedBox(width: col.fixedWidth, child: padded)
        : Expanded(flex: col.size.flex, child: padded);
  }
}

class _TableRow extends StatefulWidget {
  const _TableRow({
    required this.tint,
    required this.last,
    required this.label,
    required this.onTap,
    required this.children,
  });

  final Color? tint;
  final bool last;
  final String? label;
  final VoidCallback? onTap;
  final List<Widget> children;

  @override
  State<_TableRow> createState() => _TableRowState();
}

class _TableRowState extends State<_TableRow> {
  bool _hovered = false;

  Color? _fill(Tokens t) {
    if (!_hovered) {
      return widget.tint;
    }
    final Color? tint = widget.tint;
    final Color base = tint == null
        ? t.surface
        : Color.alphaBlend(tint, t.surface);
    return Color.alphaBlend(t.rowHover, base);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Widget content = DecoratedBox(
      decoration: BoxDecoration(
        color: _fill(t),
        border: widget.last
            ? null
            : Border(bottom: BorderSide(color: t.border)),
      ),
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: Row(children: widget.children),
      ),
    );
    final VoidCallback? tap = widget.onTap;
    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: tap == null
          ? content
          : HoverTap(onTap: tap, semanticsLabel: widget.label, child: content),
    );
  }
}
