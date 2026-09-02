import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/content_rating.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/model/user/self_user.dart';

void main() {
  test('AccountProfile parses snake_case fields and nested rating', () {
    final AccountProfile p = AccountProfile.fromJson(<String, dynamic>{
      'id': 'u1',
      'username': 'pat',
      'role': 'admin',
      'max_content_rating': <String, dynamic>{
        'system': 'MPAA',
        'code': 'PG-13',
      },
      'preferred_audio': <String>['eng', 'spa'],
      'preferred_subtitle': <String>['eng'],
      'concurrent_stream_limit': 2,
      'bitrate_cap': 8000000,
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-02-01T00:00:00Z',
    });

    expect(p.role, UserRole.admin);
    expect(p.maxContentRating?.label, 'MPAA PG-13');
    expect(
      const ContentRating(system: 'mpaa', code: 'pg-13').label,
      'MPAA PG-13',
      reason: 'the table stores lower case, the UI always presents upper case',
    );
    expect(p.preferredAudio, <String>['eng', 'spa']);
    expect(p.concurrentStreamLimit, 2);
    expect(p.bitrateCap, 8000000);
  });

  test('AccountProfile tolerates missing optional fields', () {
    final AccountProfile p = AccountProfile.fromJson(<String, dynamic>{
      'id': 'u1',
      'username': 'pat',
      'role': 'player',
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-01-01T00:00:00Z',
    });

    expect(p.role, UserRole.player);
    expect(p.maxContentRating, isNull);
    expect(p.preferredAudio, isEmpty);
    expect(p.bitrateCap, isNull);
  });
}
