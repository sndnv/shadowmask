import 'package:flutter/material.dart';

import 'package:shadowmask/components/count_pill.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/menu_option.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class GenreDropdown extends StatefulWidget {
  const GenreDropdown({
    super.key,
    required this.genres,
    required this.selected,
    required this.onApply,
    this.showLabel = true,
    this.live = false,
  });

  final List<Genre> genres;
  final List<String> selected;
  final ValueChanged<List<String>> onApply;
  final bool showLabel;
  final bool live;

  @override
  State<GenreDropdown> createState() => _GenreDropdownState();
}

class _GenreDropdownState extends State<GenreDropdown> {
  final MenuController _controller = MenuController();
  late Set<String> _pending = widget.selected.toSet();
  bool _menuOpen = false;

  void _open() {
    setState(() => _pending = widget.selected.toSet());
    _controller.open();
  }

  String get _text {
    if (!widget.showLabel) {
      return widget.selected.isEmpty ? Strings.filterAll : '';
    }
    return widget.selected.isEmpty
        ? '${Strings.genresLabel}: ${Strings.filterAll}'
        : '${Strings.genresLabel}:';
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return MenuAnchor(
      controller: _controller,
      onOpen: () => setState(() => _menuOpen = true),
      onClose: () => setState(() => _menuOpen = false),
      alignmentOffset: kMenuOffset,
      style: appMenuStyle(t),
      menuChildren: <Widget>[
        ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 320, minWidth: 220),
          child: SingleChildScrollView(
            primary: false,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                for (final Genre g in widget.genres)
                  _GenreCheck(
                    label: g.name,
                    checked: _pending.contains(g.name),
                    onChanged: (bool v) {
                      setState(() {
                        if (v) {
                          _pending.add(g.name);
                        } else {
                          _pending.remove(g.name);
                        }
                      });
                      if (widget.live) {
                        widget.onApply(_pending.toList());
                      }
                    },
                  ),
              ],
            ),
          ),
        ),
        if (!widget.live)
          Padding(
            padding: const EdgeInsets.fromLTRB(
              Space.s3,
              Space.s2,
              Space.s3,
              Space.s1,
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: <Widget>[
                TextButton(
                  onPressed: () {
                    _controller.close();
                    widget.onApply(const <String>[]);
                  },
                  child: const Text(Strings.clearAction),
                ),
                FilledButton(
                  onPressed: () {
                    _controller.close();
                    widget.onApply(_pending.toList());
                  },
                  child: const Text(Strings.applyAction),
                ),
              ],
            ),
          ),
      ],
      builder: (BuildContext context, MenuController controller, Widget? _) {
        final String text = _text;
        return MenuField(
          open: _menuOpen,
          onTap: () => controller.isOpen ? controller.close() : _open(),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (widget.showLabel) ...<Widget>[
                Icon(Icons.filter_alt_outlined, size: 16, color: t.muted),
                const SizedBox(width: Space.s2),
              ],
              if (text.isNotEmpty)
                Flexible(
                  child: Text(
                    text,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      color: t.text,
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              if (widget.selected.isNotEmpty) ...<Widget>[
                if (text.isNotEmpty) const SizedBox(width: Space.s2),
                CountPill(count: widget.selected.length),
              ],
            ],
          ),
        );
      },
    );
  }
}

class _GenreCheck extends StatelessWidget {
  const _GenreCheck({
    required this.label,
    required this.checked,
    required this.onChanged,
  });

  final String label;
  final bool checked;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    return MenuOption(
      label: label,
      selected: checked,
      onTap: () => onChanged(!checked),
      leading: Checkbox(
        value: checked,
        visualDensity: VisualDensity.compact,
        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
        onChanged: (bool? v) => onChanged(v ?? false),
      ),
    );
  }
}
