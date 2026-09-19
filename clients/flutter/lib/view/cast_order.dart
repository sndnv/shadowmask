import 'package:shadowmask/model/catalog/detail_dimensions.dart';

List<PersonRef> creditedAs(List<Credit> credits, CreditRole role) {
  final List<Credit> held = credits
      .where((Credit c) => c.role == role)
      .toList();
  held.sort((Credit a, Credit b) {
    final int byOrder = a.order.compareTo(b.order);
    return byOrder != 0 ? byOrder : a.person.id.compareTo(b.person.id);
  });
  final Set<String> seen = <String>{};
  return <PersonRef>[
    for (final Credit c in held)
      if (seen.add(c.person.id)) c.person,
  ];
}

List<PersonRef> billedActors(List<Credit> credits) =>
    creditedAs(credits, CreditRole.actor);
