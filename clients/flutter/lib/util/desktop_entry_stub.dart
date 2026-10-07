class DesktopEntry {
  Future<bool> installed() async => false;

  Future<void> add() async {}

  Future<void> remove() async {}
}

DesktopEntry? currentDesktopEntry() => null;

Future<void> refreshDesktopEntry() async {}
