import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/menu_option.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/viewer/list_prefs_store.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class SectionToolbar extends StatelessWidget {
  const SectionToolbar({
    super.key,
    required this.basePath,
    required this.genres,
    required this.sort,
    required this.order,
    required this.selectedGenres,
    this.library,
    this.sortScope,
    this.limit,
    this.trailing,
  });

  final String basePath;
  final List<Genre> genres;
  final String sort;
  final String order;
  final List<String> selectedGenres;
  final String? library;
  final String? sortScope;
  final int? limit;
  final Widget? trailing;

  void _apply(
    BuildContext context, {
    String? sort,
    String? order,
    List<String>? genres,
  }) {
    final List<String> chosen = genres ?? selectedGenres;
    final String nextSort = sort ?? this.sort;
    final String nextOrder = order ?? this.order;
    if (sortScope != null && (sort != null || order != null)) {
      unawaited(ListPrefsStore(sortScope!).save(nextSort, nextOrder));
    }
    Navigator.of(context).pushReplacementNamed(
      withQuery(basePath, <String, String?>{
        'sort': nextSort,
        'order': nextOrder,
        'genres': chosen.isEmpty ? null : chosen.join(','),
        'library': library,
        'limit': limit?.toString(),
      }),
    );
  }

  @override
  Widget build(BuildContext context) {
    final Widget filters = Wrap(
      spacing: Space.s4,
      runSpacing: Space.s3,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: <Widget>[
        AppDropdown<String>(
          value: sort,
          label: Strings.sortLabel,
          items: const <(String, String)>[
            ('added_at', Strings.sortAdded),
            ('title', Strings.sortTitle),
            ('year', Strings.sortYear),
          ],
          onChanged: (String v) => _apply(context, sort: v),
        ),
        _OrderToggle(
          descending: order == 'desc',
          onFlip: () =>
              _apply(context, order: order == 'desc' ? 'asc' : 'desc'),
        ),
        if (genres.isNotEmpty)
          _GenreDropdown(
            genres: genres,
            selected: selectedGenres,
            onApply: (List<String> values) => _apply(context, genres: values),
          ),
      ],
    );
    return Padding(
      padding: const EdgeInsets.only(bottom: Space.s4),
      child: trailing == null
          ? filters
          : Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Expanded(child: filters),
                trailing!,
              ],
            ),
    );
  }
}

class _OrderToggle extends StatelessWidget {
  const _OrderToggle({required this.descending, required this.onFlip});

  final bool descending;
  final VoidCallback onFlip;

  @override
  Widget build(BuildContext context) {
    final String state = descending
        ? Strings.orderDescending
        : Strings.orderAscending;
    return ToggleButton(
      compact: true,
      highlight: false,
      height: kControlHeight,
      icon: Icons.arrow_upward,
      filledIcon: Icons.arrow_downward,
      pressed: descending,
      label: state,
      tooltip: Strings.orderTooltip(state),
      onToggle: onFlip,
    );
  }
}

class _GenreDropdown extends StatefulWidget {
  const _GenreDropdown({
    required this.genres,
    required this.selected,
    required this.onApply,
  });

  final List<Genre> genres;
  final List<String> selected;
  final ValueChanged<List<String>> onApply;

  @override
  State<_GenreDropdown> createState() => _GenreDropdownState();
}

class _GenreDropdownState extends State<_GenreDropdown> {
  final MenuController _controller = MenuController();
  late Set<String> _pending = widget.selected.toSet();
  bool _menuOpen = false;

  void _open() {
    setState(() => _pending = widget.selected.toSet());
    _controller.open();
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
                    onChanged: (bool v) => setState(() {
                      if (v) {
                        _pending.add(g.name);
                      } else {
                        _pending.remove(g.name);
                      }
                    }),
                  ),
              ],
            ),
          ),
        ),
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
        return MenuField(
          open: _menuOpen,
          onTap: () => controller.isOpen ? controller.close() : _open(),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Text(
                Strings.genresLabel,
                style: TextStyle(
                  color: t.text,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
              if (widget.selected.isNotEmpty) ...<Widget>[
                const SizedBox(width: Space.s2),
                _CountPill(count: widget.selected.length),
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

class _CountPill extends StatelessWidget {
  const _CountPill({required this.count});

  final int count;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 1),
      decoration: BoxDecoration(
        color: t.accent,
        borderRadius: const BorderRadius.all(Radii.pill),
      ),
      child: Text(
        '$count',
        style: monoStyle.copyWith(
          color: t.accentContrast,
          fontSize: 11,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}
