import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/catalog/person_profile.dart';

String creditRoleLabel(CreditRole role) => switch (role) {
  CreditRole.actor => Strings.roleActor,
  CreditRole.director => Strings.roleDirector,
  CreditRole.writer => Strings.roleWriter,
};

String filmographyCaption(List<FilmographyEntry> credits) {
  final Map<CreditRole, List<String>> characters = <CreditRole, List<String>>{};
  for (final FilmographyEntry credit in credits) {
    final List<String> named = characters.putIfAbsent(
      credit.role,
      () => <String>[],
    );
    final String? character = credit.character;
    if (character != null &&
        character.isNotEmpty &&
        !named.contains(character)) {
      named.add(character);
    }
  }
  return characters.entries
      .map(
        (MapEntry<CreditRole, List<String>> role) => role.value.isEmpty
            ? creditRoleLabel(role.key)
            : '${creditRoleLabel(role.key)} · ${role.value.join(' / ')}',
      )
      .join(', ');
}
