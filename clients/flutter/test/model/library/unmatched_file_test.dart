import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/library/unmatched_file.dart';

void main() {
  test('UnmatchedFile parses candidates', () {
    final UnmatchedFile f = UnmatchedFile.fromJson(<String, dynamic>{
      'id': 'u1',
      'library_id': 'l1',
      'path': '/incoming/unknown.mkv',
      'candidates': <dynamic>[
        <String, dynamic>{
          'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
          'confidence': 0.8,
          'label': 'The Matrix (1999)',
        },
      ],
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-01-01T00:00:00Z',
    });

    expect(f.libraryId, 'l1');
    expect(f.path, '/incoming/unknown.mkv');
    expect(f.candidates, hasLength(1));
    final MatchCandidate c = f.candidates.first;
    expect(c.title.type, TitleKind.movie);
    expect(c.confidence, 0.8);
    expect(c.label, 'The Matrix (1999)');
  });

  test('UnmatchedFile defaults candidates to empty', () {
    final UnmatchedFile f = UnmatchedFile.fromJson(<String, dynamic>{
      'id': 'u2',
      'library_id': 'l1',
      'path': '/incoming/other.mkv',
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-01-01T00:00:00Z',
    });

    expect(f.candidates, isEmpty);
  });
}
