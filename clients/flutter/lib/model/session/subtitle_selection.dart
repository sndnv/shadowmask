enum SubtitleKind { embedded, file }

class SubtitleSelection {
  const SubtitleSelection.embedded(this.index)
    : kind = SubtitleKind.embedded,
      id = null;

  const SubtitleSelection.file(this.id)
    : kind = SubtitleKind.file,
      index = null;

  final SubtitleKind kind;
  final int? index;
  final String? id;

  factory SubtitleSelection.fromJson(Map<String, dynamic> json) {
    final String type = json['type'] as String? ?? 'file';
    if (type == 'embedded') {
      return SubtitleSelection.embedded((json['index'] as num?)?.toInt() ?? 0);
    }
    return SubtitleSelection.file(json['id'] as String? ?? '');
  }

  Map<String, dynamic> toJson() => kind == SubtitleKind.embedded
      ? <String, dynamic>{'type': 'embedded', 'index': index}
      : <String, dynamic>{'type': 'file', 'id': id};

  String toWire() =>
      kind == SubtitleKind.embedded ? 'embedded:$index' : 'file:$id';

  static SubtitleSelection? fromWire(String? value) {
    if (value == null || value.isEmpty) {
      return null;
    }
    final int sep = value.indexOf(':');
    if (sep < 0) {
      return null;
    }
    final String kind = value.substring(0, sep);
    final String key = value.substring(sep + 1);
    return kind == 'embedded'
        ? SubtitleSelection.embedded(int.tryParse(key) ?? 0)
        : SubtitleSelection.file(key);
  }

  @override
  bool operator ==(Object other) =>
      other is SubtitleSelection &&
      other.kind == kind &&
      other.index == index &&
      other.id == id;

  @override
  int get hashCode => Object.hash(kind, index, id);
}
