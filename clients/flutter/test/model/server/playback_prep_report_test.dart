import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/server/library_prep.dart';
import 'package:shadowmask/model/server/playback_prep_report.dart';

void main() {
  test('a report reads the running ffmpeg and each library owed a re-read', () {
    final PlaybackPrepReport r = PlaybackPrepReport.fromJson(<String, dynamic>{
      'ffmpeg': '9.0.2',
      'libraries': <dynamic>[
        <String, dynamic>{
          'id': 'films',
          'name': 'Films',
          'outdated': 30,
          'unprepared': 4,
          'prepared_with': <String>['5.1.9-0+deb12u1'],
        },
      ],
    });

    expect(r.ffmpeg, '9.0.2');
    final LibraryPrep films = r.libraries.single;
    expect(films.id, 'films');
    expect(films.name, 'Films');
    expect(films.outdated, 30);
    expect(films.unprepared, 4);
    expect(films.preparedWith, <String>['5.1.9-0+deb12u1']);
  });

  test('an empty report has no ffmpeg and no libraries', () {
    final PlaybackPrepReport r = PlaybackPrepReport.fromJson(
      <String, dynamic>{},
    );

    expect(r.ffmpeg, isNull);
    expect(r.libraries, isEmpty);
  });
}
