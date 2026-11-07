import 'package:shadowmask/model/common/title_ref.dart';

class TitleLink {
  const TitleLink({
    this.kind = TitleKind.episode,
    required this.id,
    this.seasonId,
    required this.label,
    this.queueEntryId,
  });

  final TitleKind kind;
  final String id;
  final String? seasonId;
  final String label;
  final String? queueEntryId;
}
