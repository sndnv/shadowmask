import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/server/server_info.dart';

void main() {
  test('ServerInfo.fromJson defaults profile_version to 1', () {
    expect(ServerInfo.fromJson(<String, dynamic>{}).profileVersion, 1);
    expect(
      ServerInfo.fromJson(<String, dynamic>{
        'profile_version': 3,
      }).profileVersion,
      3,
    );
  });
}
