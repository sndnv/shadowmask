import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/cast_order.dart';

class CrewGroup {
  const CrewGroup(this.label, this.people);

  final String label;
  final List<PersonRef> people;
}

List<CrewGroup> crewGroups(List<Credit> credits) {
  final List<CrewGroup> groups = <CrewGroup>[];
  for (final (String label, CreditRole role) in <(String, CreditRole)>[
    (Strings.directedBy, CreditRole.director),
    (Strings.writtenBy, CreditRole.writer),
  ]) {
    final List<PersonRef> people = creditedAs(credits, role);
    if (people.isNotEmpty) {
      groups.add(CrewGroup(label, people));
    }
  }
  return groups;
}

class CrewLine extends StatelessWidget {
  const CrewLine({super.key, required this.credits});

  final List<Credit> credits;

  @override
  Widget build(BuildContext context) {
    final List<CrewGroup> groups = crewGroups(credits);
    if (groups.isEmpty) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    return Wrap(
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: Space.s1,
      runSpacing: Space.s1,
      children: <Widget>[
        for (final (int index, CrewGroup group) in groups.indexed) ...<Widget>[
          if (index > 0) _muted('·', t),
          _muted(group.label, t),
          for (final (int at, PersonRef person) in group.people.indexed)
            _PersonLink(
              person: person,
              trailing: at < group.people.length - 1 ? ',' : '',
            ),
        ],
      ],
    );
  }

  Widget _muted(String text, Tokens t) =>
      Text(text, style: TextStyle(color: t.muted, fontSize: 13));
}

class _PersonLink extends StatelessWidget {
  const _PersonLink({required this.person, required this.trailing});

  final PersonRef person;
  final String trailing;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return InkWell(
      onTap: () => Navigator.of(context).pushNamed(personRoute(person.id)),
      child: Text(
        '${person.name}$trailing',
        style: TextStyle(
          color: t.accent,
          fontSize: 13,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}
