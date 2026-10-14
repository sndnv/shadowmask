import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/command_listener_types.dart';
import 'package:shadowmask/view/target_addresses.dart';

InterfaceAddress _at(String name, String address) => (
  name: name,
  address: address,
  octets: address.split('.').map(int.parse).toList(),
);

void main() {
  group('isPrivateIpv4', () {
    for (final String address in <String>[
      '10.0.0.1',
      '10.255.255.255',
      '172.16.0.1',
      '172.31.255.255',
      '192.168.0.1',
    ]) {
      test('[$address] is private', () {
        expect(isPrivateIpv4(_at('wlan0', address).octets), isTrue);
      });
    }

    for (final String address in <String>[
      '172.15.255.255',
      '172.32.0.1',
      '192.169.0.1',
      '127.0.0.1',
      '169.254.10.10',
      '8.8.8.8',
    ]) {
      test('[$address] is not private', () {
        expect(isPrivateIpv4(_at('wlan0', address).octets), isFalse);
      });
    }

    test('anything but four octets is not private', () {
      expect(isPrivateIpv4(<int>[10, 0, 0]), isFalse);
      expect(isPrivateIpv4(List<int>.filled(16, 10)), isFalse);
    });
  });

  group('targetAddresses', () {
    test('keeps private addresses and drops the rest', () {
      expect(
        targetAddresses(<InterfaceAddress>[
          _at('lo', '127.0.0.1'),
          _at('wlan0', '192.168.1.20'),
          _at('rmnet0', '100.64.0.9'),
        ]),
        <String>['192.168.1.20'],
      );
    });

    test('puts Wi-Fi and Ethernet before other interfaces', () {
      expect(
        targetAddresses(<InterfaceAddress>[
          _at('rmnet_data0', '10.140.2.7'),
          _at('tun0', '10.8.0.2'),
          _at('eth0', '172.16.0.5'),
          _at('wlan0', '192.168.1.20'),
        ]),
        <String>['172.16.0.5', '192.168.1.20', '10.140.2.7', '10.8.0.2'],
      );
    });

    test('reports at most four, each once', () {
      expect(
        targetAddresses(<InterfaceAddress>[
          _at('wlan0', '192.168.1.20'),
          _at('wlan1', '192.168.1.20'),
          _at('wlan2', '192.168.2.20'),
          _at('eth0', '10.0.0.2'),
          _at('eth1', '10.0.0.3'),
          _at('rmnet0', '10.140.2.7'),
        ]),
        <String>['192.168.1.20', '192.168.2.20', '10.0.0.2', '10.0.0.3'],
      );
    });

    test('nothing private is an empty report', () {
      expect(targetAddresses(<InterfaceAddress>[]), isEmpty);
      expect(
        targetAddresses(<InterfaceAddress>[_at('wlan0', '8.8.8.8')]),
        isEmpty,
      );
    });
  });
}
