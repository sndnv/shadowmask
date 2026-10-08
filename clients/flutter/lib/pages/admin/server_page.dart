import 'dart:async';

import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/server_admin_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_header_split.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/benchmark_version_dialog.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/components/muted_note.dart';
import 'package:shadowmask/components/outline_pill.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/progress_meter.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/server/benchmark_report.dart';
import 'package:shadowmask/model/server/benchmark_run.dart';
import 'package:shadowmask/model/server/benchmark_source.dart';
import 'package:shadowmask/model/server/capabilities_report.dart';
import 'package:shadowmask/model/server/delivery_totals.dart';
import 'package:shadowmask/model/server/encoder_totals.dart';
import 'package:shadowmask/model/server/ffmpeg_capabilities.dart';
import 'package:shadowmask/model/server/ffmpeg_component.dart';
import 'package:shadowmask/model/server/hardware_capabilities.dart';
import 'package:shadowmask/model/server/hardware_test.dart';
import 'package:shadowmask/model/server/host_capabilities.dart';
import 'package:shadowmask/model/server/transcode_totals.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';

const Duration kServerPoll = Duration(seconds: 2);
const double kCapabilityColumnsBreakpoint = 900;
const double _kFactLabelWidth = 180;
const double _kCapabilityLabelWidth = 120;
const double _kBenchmarkInfoWidth = 380;
const Object _kRecheck = #recheck;
const Object _kBenchmark = #benchmark;

class ServerPage extends StatelessWidget {
  const ServerPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _ServerBody(server: ServerAdminApi(api), admin: AdminApi(api))
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _ServerBody extends StatefulWidget {
  const _ServerBody({required this.server, required this.admin});

  final ServerAdminApi server;
  final AdminApi admin;

  @override
  State<_ServerBody> createState() => _ServerBodyState();
}

class _ServerBodyState extends State<_ServerBody> with Mutations<_ServerBody> {
  final GlobalKey<_PolledState<CapabilitiesReport>> _capabilities =
      GlobalKey<_PolledState<CapabilitiesReport>>();
  final GlobalKey<_PolledState<BenchmarkReport>> _benchmark =
      GlobalKey<_PolledState<BenchmarkReport>>();
  final GlobalKey<_PolledState<TranscodeTotals>> _totals =
      GlobalKey<_PolledState<TranscodeTotals>>();

  Future<void> _recheck() => mutate(
    key: _kRecheck,
    () => widget.server.recheck(),
    errorText: Strings.errorRecheck,
    then: () => _capabilities.currentState?.refresh(),
  );

  Future<void> _start(Version version) async {
    await mutate(
      key: _kBenchmark,
      () => widget.server.startBenchmark(version.id),
      successText: Strings.toastBenchmarkStarted,
      errorText: Strings.errorStartBenchmark,
    );
    _benchmark.currentState?.refresh();
  }

  Future<void> _chooseVersion() async {
    final Version? version = await showBenchmarkVersionDialog(
      context,
      admin: widget.admin,
    );
    if (version != null && mounted) {
      await _start(version);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(<Crumb>[
          Crumb(Strings.adminHeading, route: adminRoute()),
          const Crumb(Strings.adminServer),
        ]),
        const SizedBox(height: Space.s4),
        SectionBlock(
          title: Strings.capabilitiesHeading,
          actionItems: <PageAction>[
            PageAction(
              icon: Icons.refresh,
              label: Strings.recheck,
              onPressed: busy(_kRecheck) ? null : _recheck,
            ),
          ],
          child: _Polled<CapabilitiesReport>(
            key: _capabilities,
            fetch: widget.server.capabilities,
            busy: (CapabilitiesReport r) => r.state == CheckState.checking,
            errorText: Strings.couldNotLoadCapabilities,
            builder: (BuildContext context, CapabilitiesReport r) =>
                _CapabilitiesView(report: r),
          ),
        ),
        SectionBlock(
          title: Strings.benchmarkHeading,
          actionItems: <PageAction>[
            PageAction(
              icon: Icons.speed,
              label: Strings.benchmarkAction,
              onPressed: busy(_kBenchmark) ? null : _chooseVersion,
            ),
          ],
          child: _Polled<BenchmarkReport>(
            key: _benchmark,
            fetch: widget.server.benchmark,
            busy: (BenchmarkReport r) => r.state == BenchmarkState.running,
            errorText: Strings.couldNotLoadBenchmark,
            builder: (BuildContext context, BenchmarkReport r) =>
                _BenchmarkView(report: r),
          ),
        ),
        SectionBlock(
          title: Strings.totalsHeading,
          actionItems: <PageAction>[
            PageAction(
              icon: Icons.refresh,
              label: Strings.refresh,
              onPressed: () => _totals.currentState?.refresh(),
            ),
          ],
          child: _Polled<TranscodeTotals>(
            key: _totals,
            fetch: widget.server.totals,
            busy: (TranscodeTotals _) => false,
            errorText: Strings.couldNotLoadTotals,
            builder: (BuildContext context, TranscodeTotals totals) =>
                _TotalsView(totals: totals),
          ),
        ),
      ],
    );
  }
}

class _Polled<T extends Object> extends StatefulWidget {
  const _Polled({
    super.key,
    required this.fetch,
    required this.busy,
    required this.errorText,
    required this.builder,
  });

  final Future<T> Function() fetch;
  final bool Function(T data) busy;
  final String errorText;
  final DataBuilder<T> builder;

  @override
  State<_Polled<T>> createState() => _PolledState<T>();
}

class _PolledState<T extends Object> extends State<_Polled<T>> {
  AsyncSnapshot<T> _snapshot = AsyncSnapshot<T>.waiting();
  Timer? _timer;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    refresh();
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> refresh() async {
    _timer?.cancel();
    final int generation = ++_generation;
    try {
      final T data = await widget.fetch();
      if (!mounted || generation != _generation) {
        return;
      }
      setState(() {
        _snapshot = AsyncSnapshot<T>.withData(ConnectionState.done, data);
      });
      if (widget.busy(data)) {
        _timer = Timer(kServerPoll, refresh);
      }
    } catch (error) {
      if (!mounted || generation != _generation) {
        return;
      }
      setState(() {
        _snapshot = AsyncSnapshot<T>.withError(ConnectionState.done, error);
      });
    }
  }

  @override
  Widget build(BuildContext context) => buildSnapshot<T>(
    context,
    _snapshot,
    builder: widget.builder,
    errorText: widget.errorText,
    onRetry: refresh,
  );
}

class _CapabilitiesView extends StatelessWidget {
  const _CapabilitiesView({required this.report});

  final CapabilitiesReport report;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final FfmpegCapabilities? ffmpeg = report.ffmpeg;
    final HardwareCapabilities? hardware = report.hardware;
    final bool failed = hardware?.test.outcome == HardwareTestOutcome.failed;
    final String? detail = failed ? hardware?.test.detail : null;
    final String? lowPowerDetail = failed
        ? hardware?.test.lowPowerDetail
        : null;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        _CapabilityColumns(<Widget>[
          _host(),
          if (hardware != null) _hardware(t, hardware),
          if (ffmpeg != null) _ffmpeg(t, ffmpeg),
        ]),
        if (detail != null || lowPowerDetail != null)
          const SizedBox(height: Space.s3),
        if (detail != null)
          _Fact(
            Strings.factFfmpegError,
            detail,
            mono: true,
            labelWidth: _kCapabilityLabelWidth,
          ),
        if (lowPowerDetail != null)
          _Fact(
            Strings.factLowPowerError,
            lowPowerDetail,
            mono: true,
            labelWidth: _kCapabilityLabelWidth,
          ),
      ],
    );
  }

  Widget _column(String heading, List<Widget> facts) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: <Widget>[_Subheading(heading), ...facts],
  );

  Widget _host() {
    final HostCapabilities? host = report.host;
    return _column(Strings.hostHeading, <Widget>[
      _Fact(Strings.factChecked, switch (report.state) {
        CheckState.unchecked => Strings.capabilitiesNotYet,
        CheckState.checking => Strings.capabilitiesChecking,
        CheckState.ready =>
          dateTimeText(report.checkedAt) ?? Strings.unknownValue,
      }, labelWidth: _kCapabilityLabelWidth),
      if (host != null) ...<Widget>[
        _Fact(
          Strings.factCpu,
          host.cpu ?? Strings.unknownValue,
          labelWidth: _kCapabilityLabelWidth,
        ),
        _Fact(
          Strings.factCores,
          host.cores?.toString() ?? Strings.unknownValue,
          labelWidth: _kCapabilityLabelWidth,
        ),
        _Fact(
          Strings.factThreads,
          '${host.threads}',
          labelWidth: _kCapabilityLabelWidth,
        ),
      ],
    ]);
  }

  Widget _ffmpeg(Tokens t, FfmpegCapabilities ffmpeg) {
    final String? version = ffmpeg.version;
    final String? error = ffmpeg.error;
    return _column(Strings.ffmpegHeading, <Widget>[
      if (version != null)
        _Fact(Strings.factVersion, version, labelWidth: _kCapabilityLabelWidth),
      if (error != null)
        _Fact(
          Strings.factError,
          error,
          color: t.danger,
          labelWidth: _kCapabilityLabelWidth,
        ),
      _Pills(
        Strings.factAccelerations,
        Strings.helpAccelerations,
        <(String, bool)>[
          for (final String method in ffmpeg.hwaccels) (method, true),
        ],
      ),
      _Pills(Strings.encodersHeading, Strings.helpEncoders, <(String, bool)>[
        for (final FfmpegComponent c in ffmpeg.encoders) (c.name, c.present),
      ]),
      _Pills(Strings.filtersHeading, Strings.helpFilters, <(String, bool)>[
        for (final FfmpegComponent c in ffmpeg.filters) (c.name, c.present),
      ]),
    ]);
  }

  Widget _hardware(Tokens t, HardwareCapabilities hardware) {
    final HardwareTest test = hardware.test;
    return _column(Strings.hardwareHeading, <Widget>[
      _Fact(
        Strings.factMode,
        hardware.mode,
        help: Strings.helpMode,
        labelWidth: _kCapabilityLabelWidth,
      ),
      _Fact(
        Strings.factDevice,
        hardware.device,
        mono: true,
        labelWidth: _kCapabilityLabelWidth,
      ),
      _Fact(
        Strings.factDevicePresent,
        hardware.devicePresent ? Strings.yes : Strings.no,
        color: hardware.devicePresent ? t.ok : t.danger,
        labelWidth: _kCapabilityLabelWidth,
      ),
      _Fact(
        Strings.factInUse,
        hardware.inUse ? Strings.yes : Strings.no,
        help: Strings.helpInUse,
        labelWidth: _kCapabilityLabelWidth,
      ),
      _Fact(
        Strings.factTestEncode,
        switch (test.outcome) {
          HardwareTestOutcome.works => Strings.testWorks(
            '${test.elapsedMs ?? 0} ms',
            lowPower: test.lowPower ?? false,
          ),
          HardwareTestOutcome.failed => Strings.testFailed,
          HardwareTestOutcome.skipped => Strings.testSkipped(test.detail),
        },
        color: switch (test.outcome) {
          HardwareTestOutcome.works => t.ok,
          HardwareTestOutcome.failed => t.danger,
          HardwareTestOutcome.skipped => null,
        },
        help: Strings.helpTestEncode,
        labelWidth: _kCapabilityLabelWidth,
      ),
    ]);
  }
}

class _Pills extends StatelessWidget {
  const _Pills(this.label, this.help, this.items);

  final String label;
  final String help;
  final List<(String, bool)> items;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: Space.s1),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        _Label(label, help: help),
        const SizedBox(height: Space.s1),
        if (items.isEmpty)
          const Text(Strings.noneFound)
        else
          Wrap(
            spacing: Space.s2,
            runSpacing: Space.s1,
            children: <Widget>[
              for (final (String name, bool present) in items)
                Tooltip(
                  message: present
                      ? Strings.componentPresent
                      : Strings.componentMissing,
                  child: OutlinePill(
                    icon: present ? Icons.check : Icons.close,
                    text: name,
                    active: present,
                  ),
                ),
            ],
          ),
      ],
    ),
  );
}

class _CapabilityColumns extends StatelessWidget {
  const _CapabilityColumns(this.columns);

  final List<Widget> columns;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (BuildContext context, BoxConstraints constraints) {
      if (constraints.maxWidth < kCapabilityColumnsBreakpoint) {
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            for (int i = 0; i < columns.length; i++) ...<Widget>[
              if (i > 0) const SizedBox(height: Space.s4),
              columns[i],
            ],
          ],
        );
      }
      return Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          for (int i = 0; i < columns.length; i++) ...<Widget>[
            if (i > 0) const SizedBox(width: Space.s6),
            Expanded(child: columns[i]),
          ],
        ],
      );
    },
  );
}

class _BenchmarkView extends StatelessWidget {
  const _BenchmarkView({required this.report});

  final BenchmarkReport report;

  @override
  Widget build(BuildContext context) {
    final BenchmarkReport r = report;
    if (r.state == BenchmarkState.idle) {
      return const MutedNote(Strings.benchmarkIdle);
    }
    final Tokens t = context.tokens;
    return AdminHeaderSplit(
      startWidth: _kBenchmarkInfoWidth,
      breakpoint: kCapabilityColumnsBreakpoint,
      start: _info(t, r),
      end: _runs(t, r),
    );
  }

  Widget _info(Tokens t, BenchmarkReport r) {
    final String? versionId = r.versionId;
    final String? finished = dateTimeText(r.finishedAt);
    final BenchmarkSource? source = r.source;
    final String? keyframesError = r.keyframesError;
    final int? keyframesMs = r.keyframesMs;
    final int total = r.progress.total;
    final bool running = r.state == BenchmarkState.running;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        _Fact(
          Strings.factStatus,
          running
              ? Strings.benchmarkRunning(r.progress.done, total)
              : Strings.benchmarkDone,
          labelWidth: _kCapabilityLabelWidth,
        ),
        if (running)
          _Fact.widget(
            Strings.factProgress,
            Align(
              alignment: Alignment.centerLeft,
              child: ProgressMeter(
                percent: total == 0 ? 0 : r.progress.done * 100 ~/ total,
              ),
            ),
            labelWidth: _kCapabilityLabelWidth,
          ),
        if (versionId != null)
          _Fact(
            Strings.factVersion,
            versionId,
            mono: true,
            route: versionRoute(versionId),
            labelWidth: _kCapabilityLabelWidth,
          ),
        _Fact(
          Strings.columnStarted,
          dateTimeText(r.startedAt) ?? Strings.unknownValue,
          labelWidth: _kCapabilityLabelWidth,
        ),
        if (finished != null)
          _Fact(
            Strings.columnFinished,
            finished,
            labelWidth: _kCapabilityLabelWidth,
          ),
        if (source != null)
          _Fact(
            Strings.factSource,
            _sourceText(source),
            labelWidth: _kCapabilityLabelWidth,
          ),
        if (keyframesError != null)
          _Fact(
            Strings.factKeyframeRead,
            keyframesError,
            color: t.danger,
            help: Strings.helpKeyframeRead,
            labelWidth: _kCapabilityLabelWidth,
          )
        else if (keyframesMs != null)
          _Fact(
            Strings.factKeyframeRead,
            secondsText(keyframesMs.toDouble()),
            help: Strings.helpKeyframeRead,
            labelWidth: _kCapabilityLabelWidth,
          ),
      ],
    );
  }

  Widget _runs(Tokens t, BenchmarkReport r) => AdminTable<BenchmarkRun>(
    rows: r.runs,
    emptyText: Strings.emptyBenchmarkRuns,
    minWidth: 760,
    rowColor: (BenchmarkRun run) => switch (run.outcome) {
      BenchmarkOutcome.ok => null,
      BenchmarkOutcome.tooSlow => t.rowWarn,
      BenchmarkOutcome.failed => t.rowDanger,
    },
    columns: <AdminColumn<BenchmarkRun>>[
      AdminColumn<BenchmarkRun>(
        label: Strings.columnSegment,
        essential: true,
        help: Strings.helpSegment,
        cell: (BuildContext context, BenchmarkRun run) =>
            Text(Strings.segmentAt(run.segment, clock(run.startMs))),
      ),
      AdminColumn<BenchmarkRun>(
        label: Strings.columnEncoder,
        size: AdminColumnSize.small,
        essential: true,
        help: Strings.helpEncoder,
        cell: (BuildContext context, BenchmarkRun run) => Text(
          run.encoder,
          style: TextStyle(color: _encoderColor(t, run.encoder)),
        ),
      ),
      AdminColumn<BenchmarkRun>(
        label: Strings.columnOutcome,
        size: AdminColumnSize.small,
        help: Strings.helpOutcome,
        cell: (BuildContext context, BenchmarkRun run) =>
            Text(_outcomeText(run.outcome)),
      ),
      AdminColumn<BenchmarkRun>(
        label: Strings.columnTime,
        size: AdminColumnSize.small,
        align: AdminColumnAlign.end,
        help: Strings.helpTime,
        cell: (BuildContext context, BenchmarkRun run) =>
            Text(secondsText(run.elapsedMs.toDouble())),
      ),
      AdminColumn<BenchmarkRun>(
        label: Strings.columnRealtime,
        size: AdminColumnSize.small,
        essential: true,
        align: AdminColumnAlign.end,
        help: Strings.helpRealtime,
        cell: (BuildContext context, BenchmarkRun run) =>
            run.outcome == BenchmarkOutcome.ok
            ? _Realtime(run.realtime)
            : const Text(Strings.noValue),
      ),
      AdminColumn<BenchmarkRun>(
        label: Strings.columnDetail,
        size: AdminColumnSize.large,
        cell: (BuildContext context, BenchmarkRun run) {
          final String? detail = run.detail;
          return detail == null ? const SizedBox.shrink() : _DetailLink(detail);
        },
      ),
    ],
  );
}

class _DetailLink extends StatelessWidget {
  const _DetailLink(this.detail);

  final String detail;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Align(
      alignment: Alignment.centerLeft,
      child: HoverTap(
        onTap: () => showDialog<void>(
          context: context,
          builder: (BuildContext _) => DialogShell(
            title: Strings.factFfmpegError,
            width: 720,
            child: Text(
              detail,
              style: monoStyle.copyWith(color: t.text, fontSize: 12),
            ),
          ),
        ),
        child: Text(
          detail.split('\n').first,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(
            color: t.accent,
            decoration: TextDecoration.underline,
            decorationColor: t.accent,
          ),
        ),
      ),
    );
  }
}

String _sourceText(BenchmarkSource s) {
  final int? width = s.width;
  final int? height = s.height;
  return <String>[
    ?s.codec?.toUpperCase(),
    if (width != null && height != null) '$width×$height',
    s.hdr?.name.toUpperCase() ?? Strings.sdr,
    durationText(s.durationMs),
  ].join(' · ');
}

String _outcomeText(BenchmarkOutcome outcome) => switch (outcome) {
  BenchmarkOutcome.ok => Strings.outcomeOk,
  BenchmarkOutcome.failed => Strings.outcomeFailed,
  BenchmarkOutcome.tooSlow => Strings.outcomeTooSlow,
};

String _average(double? ms) => ms == null ? Strings.noValue : secondsText(ms);

Color? _encoderColor(Tokens t, String encoder) => switch (encoder) {
  'vaapi' => t.ok,
  'software' => t.danger,
  _ => null,
};

class _Realtime extends StatelessWidget {
  const _Realtime(this.factor);

  final double factor;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Color color = factor < 1
        ? t.danger
        : factor < 2
        ? t.warn
        : t.ok;
    return Text(realtimeText(factor), style: TextStyle(color: color));
  }
}

class _TotalsView extends StatelessWidget {
  const _TotalsView({required this.totals});

  final TranscodeTotals totals;

  @override
  Widget build(BuildContext context) {
    if (totals.isEmpty) {
      return const MutedNote(Strings.totalsEmpty);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        if (totals.encoders.isNotEmpty) ...<Widget>[
          AdminTable<EncoderTotals>(
            rows: totals.encoders,
            emptyText: Strings.totalsEmpty,
            minWidth: 560,
            columns: <AdminColumn<EncoderTotals>>[
              AdminColumn<EncoderTotals>(
                label: Strings.columnEncoder,
                essential: true,
                help: Strings.helpEncoder,
                cell: (BuildContext context, EncoderTotals e) => Text(
                  e.encoder,
                  style: TextStyle(
                    color: _encoderColor(context.tokens, e.encoder),
                  ),
                ),
              ),
              AdminColumn<EncoderTotals>(
                label: Strings.columnSegments,
                size: AdminColumnSize.small,
                essential: true,
                align: AdminColumnAlign.end,
                help: Strings.helpSegments,
                cell: (BuildContext context, EncoderTotals e) =>
                    Text('${e.produced}'),
              ),
              AdminColumn<EncoderTotals>(
                label: Strings.columnFailed,
                size: AdminColumnSize.small,
                align: AdminColumnAlign.end,
                help: Strings.helpSegmentsFailed,
                cell: (BuildContext context, EncoderTotals e) =>
                    Text('${e.failed}'),
              ),
              AdminColumn<EncoderTotals>(
                label: Strings.columnAverage,
                size: AdminColumnSize.small,
                align: AdminColumnAlign.end,
                help: Strings.helpAverage,
                cell: (BuildContext context, EncoderTotals e) =>
                    Text(_average(e.averageMs)),
              ),
              AdminColumn<EncoderTotals>(
                label: Strings.columnRealtime,
                size: AdminColumnSize.small,
                essential: true,
                align: AdminColumnAlign.end,
                help: Strings.helpRealtime,
                cell: (BuildContext context, EncoderTotals e) {
                  final double? realtime = e.realtime;
                  return realtime == null
                      ? const Text(Strings.noValue)
                      : _Realtime(realtime);
                },
              ),
            ],
          ),
          const SizedBox(height: Space.s3),
        ],
        if (totals.deliveries.isNotEmpty) ...<Widget>[
          AdminTable<DeliveryTotals>(
            rows: totals.deliveries,
            emptyText: Strings.totalsEmpty,
            minWidth: 480,
            columns: <AdminColumn<DeliveryTotals>>[
              AdminColumn<DeliveryTotals>(
                label: Strings.columnDelivery,
                essential: true,
                help: Strings.helpDelivery,
                cell: (BuildContext context, DeliveryTotals d) =>
                    Text(d.delivery),
              ),
              AdminColumn<DeliveryTotals>(
                label: Strings.columnSessions,
                size: AdminColumnSize.small,
                essential: true,
                align: AdminColumnAlign.end,
                help: Strings.helpSessions,
                cell: (BuildContext context, DeliveryTotals d) =>
                    Text('${d.sessions}'),
              ),
              AdminColumn<DeliveryTotals>(
                label: Strings.columnStreamStarts,
                size: AdminColumnSize.small,
                essential: true,
                align: AdminColumnAlign.end,
                help: Strings.helpStreamStarts,
                cell: (BuildContext context, DeliveryTotals d) =>
                    Text(d.streams ? '${d.started}' : Strings.noValue),
              ),
              AdminColumn<DeliveryTotals>(
                label: Strings.columnFailed,
                size: AdminColumnSize.small,
                align: AdminColumnAlign.end,
                help: Strings.helpStreamsFailed,
                cell: (BuildContext context, DeliveryTotals d) =>
                    Text(d.streams ? '${d.failed}' : Strings.noValue),
              ),
              AdminColumn<DeliveryTotals>(
                label: Strings.columnFirstSegment,
                size: AdminColumnSize.small,
                essential: true,
                align: AdminColumnAlign.end,
                help: Strings.helpFirstSegment,
                cell: (BuildContext context, DeliveryTotals d) => Text(
                  d.streams
                      ? _average(d.averageFirstSegmentMs)
                      : Strings.noValue,
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.s3),
        ],
        AdminTable<TranscodeTotals>(
          rows: <TranscodeTotals>[totals],
          emptyText: Strings.totalsEmpty,
          minWidth: 1080,
          columns: <AdminColumn<TranscodeTotals>>[
            AdminColumn<TranscodeTotals>(
              label: Strings.columnFallbacks,
              essential: true,
              align: AdminColumnAlign.end,
              help: Strings.helpFallbacks,
              cell: (BuildContext context, TranscodeTotals t) =>
                  Text('${t.fallbacks}'),
            ),
            AdminColumn<TranscodeTotals>(
              label: Strings.columnAbandoned,
              align: AdminColumnAlign.end,
              help: Strings.helpAbandoned,
              cell: (BuildContext context, TranscodeTotals t) =>
                  Text('${t.abandoned}'),
            ),
            AdminColumn<TranscodeTotals>(
              label: Strings.columnKeyframeReads,
              essential: true,
              align: AdminColumnAlign.end,
              help: Strings.helpKeyframeReads,
              cell: (BuildContext context, TranscodeTotals t) =>
                  Text('${t.keyframeReads}'),
            ),
            AdminColumn<TranscodeTotals>(
              label: Strings.columnKeyframeAverage,
              align: AdminColumnAlign.end,
              help: Strings.helpKeyframeAverage,
              cell: (BuildContext context, TranscodeTotals t) =>
                  Text(_average(t.averageKeyframeMs)),
            ),
            AdminColumn<TranscodeTotals>(
              label: Strings.columnKeyframeFailures,
              align: AdminColumnAlign.end,
              help: Strings.helpKeyframeFailures,
              cell: (BuildContext context, TranscodeTotals t) =>
                  Text('${t.keyframeFailures}'),
            ),
            AdminColumn<TranscodeTotals>(
              label: Strings.columnDirectBytes,
              essential: true,
              align: AdminColumnAlign.end,
              help: Strings.helpDirectBytes,
              cell: (BuildContext context, TranscodeTotals t) =>
                  Text(gigabytes(t.directBytes)),
            ),
            AdminColumn<TranscodeTotals>(
              label: Strings.columnSegmentBytes,
              essential: true,
              align: AdminColumnAlign.end,
              help: Strings.helpSegmentBytes,
              cell: (BuildContext context, TranscodeTotals t) =>
                  Text(gigabytes(t.segmentBytes)),
            ),
          ],
        ),
      ],
    );
  }
}

class _Subheading extends StatelessWidget {
  const _Subheading(this.text);

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: Space.s1),
    child: Text(text, style: Theme.of(context).textTheme.titleMedium),
  );
}

class _Label extends StatelessWidget {
  const _Label(this.text, {this.help});

  final String text;
  final String? help;

  @override
  Widget build(BuildContext context) {
    final Text label = Text(
      text,
      style: Theme.of(
        context,
      ).textTheme.bodySmall?.copyWith(color: context.tokens.muted),
    );
    final String? body = help;
    if (body == null) {
      return label;
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Flexible(child: label),
        FieldHelp(title: text, body: body),
      ],
    );
  }
}

class _Fact extends StatelessWidget {
  const _Fact(
    this.label,
    this.value, {
    this.color,
    this.mono = false,
    this.route,
    this.help,
    this.labelWidth = _kFactLabelWidth,
  }) : child = null;

  const _Fact.widget(
    this.label,
    Widget this.child, {
    this.labelWidth = _kFactLabelWidth,
  }) : value = '',
       color = null,
       mono = false,
       route = null,
       help = null;

  final String label;
  final String value;
  final Widget? child;
  final Color? color;
  final bool mono;
  final String? route;
  final String? help;
  final double labelWidth;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.s1),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          SizedBox(
            width: labelWidth,
            child: _Label(label, help: help),
          ),
          Expanded(child: child ?? _value(context, t)),
        ],
      ),
    );
  }

  Widget _value(BuildContext context, Tokens t) {
    final String? target = route;
    final TextStyle? base = mono
        ? monoStyle.copyWith(color: t.text, fontSize: 12)
        : Theme.of(context).textTheme.bodyMedium;
    if (target == null) {
      return Text(value, style: base?.copyWith(color: color));
    }
    return Align(
      alignment: Alignment.centerLeft,
      child: HoverTap(
        onTap: () => Navigator.of(context).pushNamed(target),
        child: Text(
          value,
          style: base?.copyWith(
            color: t.accent,
            decoration: TextDecoration.underline,
            decorationColor: t.accent,
          ),
        ),
      ),
    );
  }
}
