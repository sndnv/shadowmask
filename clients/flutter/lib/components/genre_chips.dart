import 'package:flutter/material.dart';

import 'package:shadowmask/components/link_chip.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/space.dart';

class GenreChips extends StatelessWidget {
  const GenreChips(this.genres, {super.key, this.basePath});

  final List<Genre> genres;
  final String? basePath;

  String _route(Genre g) =>
      withQuery(basePath!, <String, String?>{'genres': g.name});

  @override
  Widget build(BuildContext context) {
    if (genres.isEmpty) {
      return const SizedBox.shrink();
    }
    return Wrap(
      spacing: Space.s2,
      runSpacing: Space.s2,
      children: <Widget>[
        for (final Genre g in genres)
          LinkChip(
            label: g.name,
            onTap: basePath == null
                ? null
                : () => Navigator.of(context).pushNamed(_route(g)),
          ),
      ],
    );
  }
}
