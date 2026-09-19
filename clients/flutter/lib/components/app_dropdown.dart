import 'package:flutter/material.dart';

import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/menu_option.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class AppDropdown<T> extends StatefulWidget {
  const AppDropdown({
    super.key,
    required this.value,
    required this.items,
    required this.onChanged,
    this.label,
    this.icon,
    this.showLabel = false,
    this.height = kControlHeight,
    this.width,
    this.tapGroupId,
    this.enabled = true,
  });

  final T value;
  final List<(T, String)> items;
  final ValueChanged<T> onChanged;
  final String? label;
  final IconData? icon;
  final bool showLabel;
  final double height;
  final double? width;
  final bool enabled;

  final Object? tapGroupId;

  @override
  State<AppDropdown<T>> createState() => _AppDropdownState<T>();
}

class _AppDropdownState<T> extends State<AppDropdown<T>> {
  final MenuController _controller = MenuController();
  bool _open = false;

  (T, String) get _current => widget.items.firstWhere(
    ((T, String) e) => e.$1 == widget.value,
    orElse: () => widget.items.first,
  );

  Widget _grouped(Widget child) {
    final Object? group = widget.tapGroupId;
    return group == null ? child : TapRegion(groupId: group, child: child);
  }

  Widget _described(Widget field, {required bool open}) {
    final String? label = widget.label;
    if (label == null) {
      return field;
    }
    final Widget described = MergeSemantics(
      child: Semantics(label: label, expanded: open, child: field),
    );
    return widget.showLabel
        ? described
        : Tooltip(message: label, child: described);
  }

  String get _fieldText {
    final String? label = widget.label;
    return widget.showLabel && label != null
        ? '$label: ${_current.$2}'
        : _current.$2;
  }

  Widget _label(Tokens t) {
    final Widget text = Text(
      _fieldText,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: TextStyle(
        color: widget.enabled ? t.text : t.muted,
        fontSize: 14,
        fontWeight: FontWeight.w600,
      ),
    );
    final IconData? glyph = widget.icon;
    if (glyph == null) {
      return text;
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(glyph, size: 16, color: t.muted),
        const SizedBox(width: Space.s2),
        Flexible(child: text),
      ],
    );
  }

  double? _menuWidth(BoxConstraints constraints) {
    if (widget.width == null && !constraints.hasTightWidth) {
      return null;
    }
    final double width = constraints.constrainWidth(
      widget.width ?? double.infinity,
    );
    return width.isFinite ? width : null;
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    if (widget.items.isEmpty) {
      return const SizedBox.shrink();
    }
    if (!widget.enabled) {
      return _described(
        MenuField(
          open: false,
          enabled: false,
          height: widget.height,
          width: widget.width,
          onTap: () {},
          child: _label(t),
        ),
        open: false,
      );
    }
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final double? menuWidth = _menuWidth(constraints);
        return MenuAnchor(
          controller: _controller,
          onOpen: () => setState(() => _open = true),
          onClose: () => setState(() => _open = false),
          alignmentOffset: kMenuOffset,
          crossAxisUnconstrained: menuWidth == null,
          style: appMenuStyle(t, minWidth: menuWidth),
          menuChildren: <Widget>[
            for (final (T value, String label) in widget.items)
              _grouped(
                MenuOption(
                  label: label,
                  selected: value == _current.$1,
                  leading: value == _current.$1
                      ? Icon(Icons.check, size: 16, color: t.accent)
                      : const SizedBox.shrink(),
                  onTap: () {
                    _controller.close();
                    if (value != widget.value) {
                      widget.onChanged(value);
                    }
                  },
                ),
              ),
          ],
          builder:
              (BuildContext context, MenuController controller, Widget? _) =>
                  _described(
                    MenuField(
                      open: _open,
                      height: widget.height,
                      width: widget.width,
                      onTap: () => controller.isOpen
                          ? controller.close()
                          : controller.open(),
                      child: _label(t),
                    ),
                    open: _open,
                  ),
        );
      },
    );
  }
}
