import 'package:shadowmask/util/command_listener_types.dart';

Future<CommandListener> bindCommandListener(int port, CommandHandler handler) =>
    Future<CommandListener>.error(UnsupportedError('no command listener'));

Future<List<InterfaceAddress>> interfaceAddresses() async =>
    const <InterfaceAddress>[];
