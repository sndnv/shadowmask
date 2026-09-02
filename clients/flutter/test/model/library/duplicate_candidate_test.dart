import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/library/duplicate_candidate.dart';

void main() {
  test('DuplicateCandidate parses title ref and paths', () {
    final DuplicateCandidate d = DuplicateCandidate.fromJson(<String, dynamic>{
      'id': 'd1',
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'paths': <String>['/a/1.mkv', '/b/1.mkv'],
    });

    expect(d.id, 'd1');
    expect(d.title.type, TitleKind.movie);
    expect(d.title.id, 'm1');
    expect(d.paths, <String>['/a/1.mkv', '/b/1.mkv']);
  });

  test('DuplicateCandidate defaults paths to empty', () {
    final DuplicateCandidate d = DuplicateCandidate.fromJson(<String, dynamic>{
      'id': 'd2',
      'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
    });

    expect(d.paths, isEmpty);
    expect(d.title.type, TitleKind.episode);
  });
}
