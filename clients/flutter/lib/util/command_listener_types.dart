typedef CommandHandler = Future<int> Function(String token);

typedef InterfaceAddress = ({String name, String address, List<int> octets});

typedef ListenerBinder =
    Future<CommandListener> Function(int port, CommandHandler handler);

typedef AddressLister = Future<List<InterfaceAddress>> Function();

abstract interface class CommandListener {
  int get port;

  Future<void> close();
}
