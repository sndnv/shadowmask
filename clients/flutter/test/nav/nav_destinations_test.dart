import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_destination.dart';
import 'package:shadowmask/nav/nav_destinations.dart';
import 'package:shadowmask/nav/nav_section.dart';

SelfUser _user(UserRole role) =>
    SelfUser(id: 'u1', username: 'pat', role: role);

bool _hasAdmin(Iterable<NavDestination> destinations) =>
    destinations.any((NavDestination d) => d.section == NavSection.admin);

void main() {
  test('a player session is offered no admin destinations', () {
    expect(
      _hasAdmin(destinationsFor(_user(UserRole.player))),
      isFalse,
      reason:
          'the server reports the session role on /users/self, so a linked '
          'device on an admin account arrives here as a player',
    );
    expect(_hasAdmin(bottomNavSplit(_user(UserRole.player)).overflow), isFalse);
  });

  test('an admin session still gets them', () {
    expect(_hasAdmin(destinationsFor(_user(UserRole.admin))), isTrue);
  });

  test('an ordinary user and an unknown user get none', () {
    expect(_hasAdmin(destinationsFor(_user(UserRole.user))), isFalse);
    expect(_hasAdmin(destinationsFor(null)), isFalse);
  });

  test('every account has Playlists, in the More sheet on a phone', () {
    for (final UserRole role in <UserRole>[UserRole.user, UserRole.player]) {
      final ({List<NavDestination> overflow, List<NavDestination> tabs}) split =
          bottomNavSplit(_user(role));
      final NavDestination playlists = destinationsFor(
        _user(role),
      ).singleWhere((NavDestination d) => d.section == NavSection.playlists);

      expect(playlists.route, '/playlists');
      expect(split.tabs, isNot(contains(playlists)));
      expect(split.overflow, contains(playlists));
    }
  });
}
