import 'package:flutter/material.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/model/auth/shared_with.dart';
import 'package:shadowmask/components/entry_row.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/pages/account/share_device_dialog.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';

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

  void _reload() => setState(() {
    _future = _account.devices(widget.userId);
  });

  Future<void> _revoke(String deviceId) => mutate(
    key: deviceId,
    () => _account.revokeDevice(widget.userId, deviceId),
    successText: Strings.toastDeviceRevoked,
    errorText: Strings.errorRevoke,
    then: _reload,
  );

  Future<void> _share(Device device) async {
    final bool? shared = await ShareDeviceDialog.show(
      context,
      api: widget.api,
      device: device,
    );
    if (shared == true && mounted) {
      _reload();
    }
  }

  Future<void> _unshare(String deviceId, SharedWith share) => mutate(
    key: '$deviceId/${share.userId}',
    () => _account.unshareDevice(deviceId, share.userId),
    successText: Strings.toastShareRemoved,
    errorText: Strings.errorRemove,
    then: _reload,
  );

  Widget _shareRow(BuildContext context, Device d, SharedWith s) => Padding(
    padding: const EdgeInsetsDirectional.only(start: Space.s4),
    child: EntryRow(
      label: Text(
        Strings.sharedWith(s.username),
        style: Theme.of(
          context,
        ).textTheme.bodySmall?.copyWith(color: context.tokens.muted),
      ),
      action: TextButton(
        onPressed: busy('${d.id}/${s.userId}') ? null : () => _unshare(d.id, s),
        child: const Text(Strings.remove),
      ),
    ),
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
              for (final Device d in devices) ...<Widget>[
                EntryRow(
                  label: Text(
                    '${d.name} · ${d.platform}',
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                  meta: sinceText(d.lastSeen) == null
                      ? null
                      : Text(
                          Strings.lastSeen(sinceText(d.lastSeen)!),
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(color: context.tokens.muted),
                        ),
                  action: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      TextButton(
                        onPressed: busy(d.id) ? null : () => _share(d),
                        child: const Text(Strings.share),
                      ),
                      TextButton(
                        onPressed: busy(d.id) ? null : () => _revoke(d.id),
                        child: const Text(Strings.revoke),
                      ),
                    ],
                  ),
                ),
                for (final SharedWith s in d.sharedWith)
                  _shareRow(context, d, s),
              ],
            ],
          );
        },
      ),
    );
  }
}
