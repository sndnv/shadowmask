import 'package:flutter/material.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class DevicesBlock extends StatefulWidget {
  const DevicesBlock({super.key, required this.api, required this.userId});

  final ApiClient api;
  final String userId;

  @override
  State<DevicesBlock> createState() => _DevicesBlockState();
}

class _DevicesBlockState extends State<DevicesBlock>
    with Mutations<DevicesBlock> {
  late final AccountApi _account = AccountApi(widget.api);
  late Future<List<Device>> _future = _account.devices(widget.userId);

  Future<void> _revoke(String deviceId) => mutate(
    key: deviceId,
    () => _account.revokeDevice(widget.userId, deviceId),
    successText: Strings.toastDeviceRevoked,
    errorText: Strings.errorRevoke,
    then: () => setState(() {
      _future = _account.devices(widget.userId);
    }),
  );

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.accountDevicesHeading,
      child: buildBlock<List<Device>>(
        future: _future,
        errorText: Strings.couldNotLoadAccount,
        loading: const SkeletonRows(rows: 3),
        builder: (BuildContext context, List<Device> devices) {
          if (devices.isEmpty) {
            return Text(
              Strings.emptyDevices,
              style: TextStyle(color: context.tokens.muted),
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: Space.s2,
            children: <Widget>[
              for (final Device d in devices)
                Row(
                  children: <Widget>[
                    Expanded(
                      child: Text(
                        '${d.name} · ${d.platform}',
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                    ),
                    if (d.lastSeen != null)
                      Text(
                        Strings.lastSeen(d.lastSeen!),
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: context.tokens.muted,
                        ),
                      ),
                    TextButton(
                      onPressed: busy(d.id) ? null : () => _revoke(d.id),
                      child: const Text(Strings.revoke),
                    ),
                  ],
                ),
            ],
          );
        },
      ),
    );
  }
}
