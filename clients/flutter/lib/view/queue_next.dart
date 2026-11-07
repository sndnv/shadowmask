import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/playlist/queue_entry.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/title_link.dart';

QueueEntry? queueNext(List<QueueEntry> entries, String entryId) {
  final int at = entries.indexWhere((QueueEntry e) => e.id == entryId);
  if (at < 0) {
    return entries.firstOrNull;
  }
  return at + 1 < entries.length ? entries[at + 1] : null;
}

String queueLabel(CatalogCard card) => card.ref.type == TitleKind.episode
    ? <String?>[card.title, card.subtitle, card.caption]
          .whereType<String>()
          .where((String part) => part.trim().isNotEmpty)
          .join(' · ')
    : card.title;

TitleLink queueLink(QueueEntry entry) => TitleLink(
  kind: entry.card.ref.type,
  id: entry.card.ref.id,
  label: queueLabel(entry.card),
  queueEntryId: entry.id,
);
