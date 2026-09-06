import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/detail_split.dart';
import 'package:shadowmask/components/facts_row.dart';
import 'package:shadowmask/components/overview_text.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/person_profile.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/pages/viewer/catalog_support.dart';

class PersonPage extends StatelessWidget {
  const PersonPage({super.key, required this.api, required this.id});

  final ApiClient api;
  final String id;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.none,
      errorText: Strings.couldNotLoadPerson,
      loading: const SkeletonPage(
        child: SkeletonDetail(aspect: CardAspect.person),
      ),
      bodyBuilder: (BuildContext context, SelfUser user) =>
          _PersonBody(api: api, id: id, userId: user.id),
    );
  }
}

class _PersonBody extends StatefulWidget {
  const _PersonBody({
    required this.api,
    required this.id,
    required this.userId,
  });

  final ApiClient api;
  final String id;
  final String userId;

  @override
  State<_PersonBody> createState() => _PersonBodyState();
}

class _PersonBodyState extends State<_PersonBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final Future<(PersonProfile, List<CatalogCard>)> _future = _load();

  Future<(PersonProfile, List<CatalogCard>)> _load() async {
    final PersonProfile p = await _catalog.person(widget.id);
    final List<CatalogCard> cards = p.filmography
        .map(
          (FilmographyEntry e) =>
              CatalogCard.fromFilmography(e, withCredit: true),
        )
        .toList();
    await tagWatched(_catalog, widget.userId, cards);
    return (p, cards);
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<(PersonProfile, List<CatalogCard>)>(
      future: _future,
      errorText: Strings.couldNotLoadPerson,
      builder: (BuildContext context, (PersonProfile, List<CatalogCard>) data) {
        final PersonProfile p = data.$1;
        final List<CatalogCard> cards = data.$2;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              const Crumb(Strings.navigationPeople),
              Crumb(p.name),
            ]),
            DetailSplit(
              poster: CardArt(
                artwork: p.artwork,
                aspect: CardAspect.person,
                imageBase: _catalog.imageBase,
              ),
              info: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    p.name,
                    style: Theme.of(context).textTheme.headlineLarge,
                  ),
                  const SizedBox(height: Space.s3),
                  FactsRow(
                    FactsRow.of(<(String, String?)>[
                      (Strings.factBorn, dateText(p.birthday)),
                      (Strings.factDied, dateText(p.deathday)),
                      (Strings.factBirthplace, p.placeOfBirth),
                    ]),
                  ),
                  if (p.alsoKnownAs.isNotEmpty) ...<Widget>[
                    const SizedBox(height: Space.s2),
                    Text(
                      '${Strings.alsoKnownAs}: ${p.alsoKnownAs.join(', ')}',
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: context.tokens.muted,
                      ),
                    ),
                  ],
                  if (p.biography != null) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    OverviewText(p.biography!),
                  ],
                ],
              ),
            ),
            if (cards.isNotEmpty) ...<Widget>[
              const SizedBox(height: Space.s5),
              Text(
                Strings.filmographyHeading,
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: Space.s3),
              CardGrid(cards: cards, imageBase: _catalog.imageBase),
            ],
          ],
        );
      },
    );
  }
}
