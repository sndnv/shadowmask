import 'package:shadowmask/model/catalog/detail_dimensions.dart';

List<PersonRef> billedActors(List<Credit> credits) {
  final List<Credit> cast = credits
      .where((Credit c) => c.role == CreditRole.actor)
      .toList();
  cast.sort((Credit a, Credit b) {
    final int byOrder = a.order.compareTo(b.order);
    return byOrder != 0 ? byOrder : a.person.id.compareTo(b.person.id);
  });
  final Set<String> seen = <String>{};
  return <PersonRef>[
    for (final Credit c in cast)
      if (seen.add(c.person.id)) c.person,
  ];
}
