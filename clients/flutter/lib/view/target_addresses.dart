import 'package:shadowmask/util/command_listener_types.dart';

const int kMaxTargetAddresses = 4;

bool isPrivateIpv4(List<int> octets) =>
    octets.length == 4 &&
    (octets[0] == 10 ||
        (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31) ||
        (octets[0] == 192 && octets[1] == 168));

bool _onLocalNetwork(InterfaceAddress a) =>
    a.name.startsWith('wlan') || a.name.startsWith('eth');

List<String> targetAddresses(List<InterfaceAddress> found) {
  final List<InterfaceAddress> private = found
      .where((InterfaceAddress a) => isPrivateIpv4(a.octets))
      .toList();
  return <String>{
    for (final InterfaceAddress a in private)
      if (_onLocalNetwork(a)) a.address,
    for (final InterfaceAddress a in private)
      if (!_onLocalNetwork(a)) a.address,
  }.take(kMaxTargetAddresses).toList();
}
