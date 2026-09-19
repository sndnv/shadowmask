import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/count_pill.dart';
import 'package:shadowmask/components/filter_sheet.dart';
import 'package:shadowmask/components/genre_dropdown.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/viewer/list_prefs_store.dart';
import 'package:shadowmask/theme/breakpoints.dart';
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
    this.libraries = const <Library>[],
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
  final List<Library> libraries;
  final String? sortScope;
  final int? limit;
  final Widget? trailing;

  void _apply(
    BuildContext context, {
    String? sort,
    String? order,
    List<String>? genres,
    String? library,
    bool libraryChanged = false,
  }) {
    final List<String> chosen = genres ?? selectedGenres;
    final String nextSort = sort ?? this.sort;
    final String nextOrder = order ?? this.order;
    final String? nextLibrary = libraryChanged ? library : this.library;
    if (sortScope != null && (sort != null || order != null)) {
      unawaited(ListPrefsStore(sortScope!).save(nextSort, nextOrder));
    }
    Navigator.of(context).pushReplacementNamed(
      withQuery(basePath, <String, String?>{
        'sort': nextSort,
        'order': nextOrder,
        'genres': chosen.isEmpty ? null : chosen.join(','),
        'library': nextLibrary,
        'limit': limit?.toString(),
      }),
    );
  }

  Future<void> _openSheet(BuildContext context) async {
    final FilterChoice? picked = await showFilterSheet(
      context,
      genres: genres,
      libraries: libraries,
      sort: sort,
      order: order,
      selectedGenres: selectedGenres,
      library: library,
    );
    if (picked == null || !context.mounted) {
      return;
    }
    _apply(
      context,
      sort: picked.sort,
      order: picked.order,
      genres: picked.genres,
      library: picked.library,
      libraryChanged: true,
    );
  }

  @override
  Widget build(BuildContext context) {
    final bool compact = compactViewport(context);
    if (compact) {
      return Padding(
        padding: const EdgeInsets.only(bottom: Space.s4),
        child: Row(
          children: <Widget>[
            _FiltersButton(
              count: activeFilterCount(
                genres: selectedGenres,
                library: library,
              ),
              onTap: () => _openSheet(context),
            ),
            const Spacer(),
            ?trailing,
          ],
        ),
      );
    }
    final Widget filters = Wrap(
      spacing: Space.s4,
      runSpacing: Space.s3,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: <Widget>[
        AppDropdown<String>(
          value: sort,
          label: Strings.sortLabel,
          icon: Icons.sort,
          showLabel: true,
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
          GenreDropdown(
            genres: genres,
            selected: selectedGenres,
            onApply: (List<String> values) => _apply(context, genres: values),
          ),
        if (libraries.isNotEmpty)
          AppDropdown<String>(
            value: library ?? '',
            label: Strings.libraryLabel,
            icon: Icons.folder_outlined,
            showLabel: true,
            items: <(String, String)>[
              const ('', Strings.filterAll),
              for (final Library l in libraries) (l.id, l.name),
            ],
            onChanged: (String v) => _apply(
              context,
              library: v.isEmpty ? null : v,
              libraryChanged: true,
            ),
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

class _FiltersButton extends StatelessWidget {
  const _FiltersButton({required this.count, required this.onTap});

  final int count;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return MenuField(
      open: false,
      onTap: onTap,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(Icons.filter_alt_outlined, size: 16, color: t.muted),
          const SizedBox(width: Space.s2),
          Text(
            Strings.filtersHeading,
            style: TextStyle(
              color: t.text,
              fontSize: 14,
              fontWeight: FontWeight.w600,
            ),
          ),
          if (count > 0) ...<Widget>[
            const SizedBox(width: Space.s2),
            CountPill(count: count),
          ],
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
      highlight: false,
      height: kControlHeight,
      icon: Icons.expand_less,
      filledIcon: Icons.expand_more,
      pressed: descending,
      label: Strings.orderTooltip(state),
      onToggle: onFlip,
    );
  }
}
