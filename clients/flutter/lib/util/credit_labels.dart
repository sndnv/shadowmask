import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';

String creditRoleLabel(CreditRole role) => switch (role) {
  CreditRole.actor => Strings.roleActor,
  CreditRole.director => Strings.roleDirector,
  CreditRole.writer => Strings.roleWriter,
};

String creditCaption(CreditRole role, String? character) {
  final String label = creditRoleLabel(role);
  final String? named = character == null || character.isEmpty
      ? null
      : character;
  return named == null ? label : '$label · $named';
}
