import 'package:flutter/material.dart';

import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/genre_dropdown.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class FilterChoice {
  const FilterChoice({
    required this.sort,
    required this.order,
    required this.genres,
    this.library,
  });

  final String sort;
  final String order;
  final List<String> genres;
  final String? library;
}

int activeFilterCount({
  required List<String> genres,
  required String? library,
}) => genres.length + (library == null ? 0 : 1);

Future<FilterChoice?> showFilterSheet(
  BuildContext context, {
  required List<Genre> genres,
  required List<Library> libraries,
  required String sort,
  required String order,
  required List<String> selectedGenres,
  String? library,
}) {
  final Tokens t = context.tokens;
  return showModalBottomSheet<FilterChoice>(
    context: context,
    backgroundColor: t.surface,
    showDragHandle: true,
    isScrollControlled: true,
    builder: (BuildContext sheet) => _FilterSheet(
      genres: genres,
      libraries: libraries,
      sort: sort,
      order: order,
      selectedGenres: selectedGenres,
      library: library,
    ),
  );
}

class _FilterSheet extends StatefulWidget {
  const _FilterSheet({
    required this.genres,
    required this.libraries,
    required this.sort,
    required this.order,
    required this.selectedGenres,
    this.library,
  });

  final List<Genre> genres;
  final List<Library> libraries;
  final String sort;
  final String order;
  final List<String> selectedGenres;
  final String? library;

  @override
  State<_FilterSheet> createState() => _FilterSheetState();
}

class _FilterSheetState extends State<_FilterSheet> {
  late String _sort = widget.sort;
  late String _order = widget.order;
  late Set<String> _genres = widget.selectedGenres.toSet();
  late String _library = widget.library ?? '';

  void _close() => Navigator.of(context).pop(
    FilterChoice(
      sort: _sort,
      order: _order,
      genres: _genres.toList(),
      library: _library.isEmpty ? null : _library,
    ),
  );

  void _clear() => setState(() {
    _genres = <String>{};
    _library = '';
  });

  Widget _row(String label, Widget control) => Padding(
    padding: const EdgeInsets.only(bottom: Space.s3),
    child: Row(
      children: <Widget>[
        SizedBox(
          width: 96,
          child: Text(
            label,
            style: TextStyle(color: context.tokens.muted, fontSize: 14),
          ),
        ),
        Expanded(child: control),
      ],
    ),
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(Space.s4, 0, Space.s4, Space.s4),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              Strings.filtersHeading,
              style: TextStyle(
                color: t.text,
                fontSize: 16,
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: Space.s4),
            _row(
              Strings.sortLabel,
              AppDropdown<String>(
                value: _sort,
                items: const <(String, String)>[
                  ('added_at', Strings.sortAdded),
                  ('title', Strings.sortTitle),
                  ('year', Strings.sortYear),
                ],
                onChanged: (String v) => setState(() => _sort = v),
              ),
            ),
            _row(
              Strings.orderLabel,
              AppDropdown<String>(
                value: _order,
                items: const <(String, String)>[
                  ('desc', Strings.orderDescending),
                  ('asc', Strings.orderAscending),
                ],
                onChanged: (String v) => setState(() => _order = v),
              ),
            ),
            if (widget.libraries.isNotEmpty)
              _row(
                Strings.libraryLabel,
                AppDropdown<String>(
                  value: _library,
                  items: <(String, String)>[
                    const ('', Strings.filterAll),
                    for (final Library l in widget.libraries) (l.id, l.name),
                  ],
                  onChanged: (String v) => setState(() => _library = v),
                ),
              ),
            if (widget.genres.isNotEmpty)
              _row(
                Strings.genresLabel,
                GenreDropdown(
                  genres: widget.genres,
                  selected: _genres.toList(),
                  showLabel: false,
                  live: true,
                  onApply: (List<String> values) =>
                      setState(() => _genres = values.toSet()),
                ),
              ),
            const SizedBox(height: Space.s4),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: <Widget>[
                TextButton(
                  onPressed: _clear,
                  child: const Text(Strings.clearAction),
                ),
                FilledButton(
                  onPressed: _close,
                  child: const Text(Strings.applyAction),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
