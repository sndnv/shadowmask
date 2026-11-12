import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/library_api.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/admin/job_status_chip.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/jobs_feed.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/library_labels.dart';
import 'package:shadowmask/view/failure_reason.dart';

class FetchPage extends StatelessWidget {
  const FetchPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _FetchBody(admin: AdminApi(api), libraries: LibraryApi(api))
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _FetchBody extends StatefulWidget {
  const _FetchBody({required this.admin, required this.libraries});

  final AdminApi admin;
  final LibraryApi libraries;

  @override
  State<_FetchBody> createState() => _FetchBodyState();
}

const int kFetchListLimit = 50;

typedef _FetchList = ({List<Job> jobs, bool truncated});

class _FetchBodyState extends State<_FetchBody> {
  late final Future<List<Library>> _libs = widget.libraries.libraries();
  late Future<_FetchList> _fetches = _loadFetches();

  final TextEditingController _sourceUrl = TextEditingController();
  final TextEditingController _title = TextEditingController();
  final TextEditingController _year = TextEditingController();
  final TextEditingController _externalId = TextEditingController();
  final TextEditingController _season = TextEditingController();
  final TextEditingController _episode = TextEditingController();
  final TextEditingController _episodeTitle = TextEditingController();
  final TextEditingController _format = TextEditingController();
  final TextEditingController _skipNote = TextEditingController();
  bool _preferH264 = false;
  bool _siteSubtitles = false;
  bool _chapters = false;
  bool _optionsOpen = false;
  String? _libraryId;
  bool _submitting = false;
  String? _urlError;
  String? _titleError;
  String? _yearError;
  String? _libraryError;
  String? _seasonError;
  String? _episodeError;
  String? _episodeTitleError;
  String? _skipNoteError;

  void _clear(VoidCallback clear) => setState(clear);

  void _reloadFetches() {
    setState(() {
      _fetches = _loadFetches();
    });
  }

  @override
  void dispose() {
    _sourceUrl.dispose();
    _title.dispose();
    _year.dispose();
    _externalId.dispose();
    _season.dispose();
    _episode.dispose();
    _episodeTitle.dispose();
    _format.dispose();
    _skipNote.dispose();
    super.dispose();
  }

  Future<_FetchList> _loadFetches() async {
    final JobsFeed feed = await widget.admin.jobs(
      filter: 'fetch',
      limit: kFetchListLimit,
    );
    final List<Job> jobs = feed.page.items
        .where((Job j) => j.kind == JobKind.fetch)
        .toList();
    return (jobs: jobs, truncated: feed.page.total > feed.page.items.length);
  }

  Future<void> _submit(bool tv) async {
    final String url = _sourceUrl.text.trim();
    final String title = _title.text.trim();
    final String? libraryId = _libraryId;
    final int? season = int.tryParse(_season.text.trim());
    final int? episode = int.tryParse(_episode.text.trim());
    final String episodeTitle = _episodeTitle.text.trim();
    final String yearText = _year.text.trim();
    final int? year = int.tryParse(yearText);
    final bool yearValid = year != null && year >= 1000 && year <= 9999;
    final String format = _format.text.trim();
    final String skipNote = _skipNote.text.trim();
    setState(() {
      _urlError = url.isEmpty ? Strings.requiredUrl : null;
      _titleError = title.isEmpty
          ? (tv ? Strings.requiredSeries : Strings.requiredTitle)
          : null;
      _yearError = !tv && yearText.isNotEmpty && !yearValid
          ? Strings.invalidYear
          : null;
      _libraryError = libraryId == null ? Strings.requiredLibrary : null;
      _seasonError = tv && season == null ? Strings.requiredSeason : null;
      _episodeError = tv && episode == null ? Strings.requiredEpisode : null;
      _episodeTitleError = tv && episodeTitle.isEmpty
          ? Strings.requiredEpisodeTitle
          : null;
      _skipNoteError = format.isNotEmpty && skipNote.isNotEmpty
          ? Strings.formatOrSkipNote
          : null;
      if (_skipNoteError != null) {
        _optionsOpen = true;
      }
    });
    if (_urlError != null ||
        _titleError != null ||
        _yearError != null ||
        _libraryError != null ||
        _seasonError != null ||
        _episodeError != null ||
        _episodeTitleError != null ||
        _skipNoteError != null) {
      return;
    }
    final String externalId = _externalId.text.trim();
    final Map<String, dynamic> body = <String, dynamic>{
      'source_url': url,
      'library_id': libraryId,
      'title': title,
      if (!tv && yearValid) 'year': year,
      if (externalId.isNotEmpty) 'external_id': externalId,
      if (tv) 'season': season,
      if (tv) 'episode': episode,
      if (tv) 'episode_title': episodeTitle,
      if (format.isNotEmpty) 'format': format,
      if (skipNote.isNotEmpty) 'skip_note': skipNote,
      if (_preferH264) 'prefer_h264': true,
      if (_siteSubtitles) 'site_subtitles': true,
      if (_chapters) 'chapters': true,
    };
    setState(() => _submitting = true);
    try {
      await widget.admin.createFetch(body);
      if (mounted) {
        Toasts.of(context).success(Strings.toastFetchQueued);
        _reloadFetches();
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
      }
    } finally {
      if (mounted) {
        setState(() => _submitting = false);
      }
    }
  }

  Widget _downloadOptions() => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    spacing: Space.s3,
    children: <Widget>[
      _OptionsToggle(
        open: _optionsOpen,
        onPressed: () => setState(() => _optionsOpen = !_optionsOpen),
      ),
      if (_optionsOpen) ...<Widget>[
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Expanded(
              child: LabelledTextField(
                controller: _format,
                label: Strings.fieldFormat,
                help: Strings.fetchFormatHelp,
                example: Strings.fetchFormatExample,
                onChanged: (_) => _clear(() => _skipNoteError = null),
              ),
            ),
            const SizedBox(width: Space.s3),
            Expanded(
              child: LabelledTextField(
                controller: _skipNote,
                label: Strings.fieldSkipNote,
                help: Strings.fetchSkipNoteHelp,
                example: Strings.fetchSkipNoteExample,
                error: _skipNoteError,
                onChanged: (_) => _clear(() => _skipNoteError = null),
              ),
            ),
          ],
        ),
        Wrap(
          spacing: Space.s4,
          runSpacing: Space.s1,
          children: <Widget>[
            _OptionBox(
              label: Strings.fieldPreferH264,
              help: Strings.fetchPreferH264Help,
              value: _preferH264,
              onChanged: (bool v) => setState(() => _preferH264 = v),
            ),
            _OptionBox(
              label: Strings.fieldSiteSubtitles,
              help: Strings.fetchSiteSubtitlesHelp,
              value: _siteSubtitles,
              onChanged: (bool v) => setState(() => _siteSubtitles = v),
            ),
            _OptionBox(
              label: Strings.fieldChapters,
              help: Strings.fetchChaptersHelp,
              value: _chapters,
              onChanged: (bool v) => setState(() => _chapters = v),
            ),
          ],
        ),
      ],
    ],
  );

  @override
  Widget build(BuildContext context) {
    return buildBlock<List<Library>>(
      future: _libs,
      errorText: Strings.couldNotLoadLibraries,
      builder: (BuildContext context, List<Library> libraries) {
        final List<Library> external = libraries
            .where((Library l) => l.origin == LibraryOrigin.external)
            .toList();
        if (external.isNotEmpty) {
          _libraryId ??= external.first.id;
        }
        final bool tv =
            external
                .where((Library l) => l.id == _libraryId)
                .firstOrNull
                ?.kind ==
            LibraryKind.tv;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.adminHeading, route: adminRoute()),
              Crumb(Strings.adminFetch),
            ]),
            if (external.isEmpty)
              const SectionBlock(
                title: Strings.fetch,
                child: Padding(
                  padding: EdgeInsets.symmetric(vertical: Space.s3),
                  child: StatusText(Strings.noExternalLibraries),
                ),
              )
            else
              SectionBlock(
                title: Strings.fetch,
                child: Material(
                  type: MaterialType.transparency,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    spacing: Space.s3,
                    children: <Widget>[
                      LabelledTextField(
                        controller: _sourceUrl,
                        label: Strings.fieldSourceUrl,
                        help: Strings.sourceUrlHelp,
                        error: _urlError,
                        onChanged: (_) => _clear(() => _urlError = null),
                      ),
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          Expanded(
                            flex: 3,
                            child: LabelledTextField(
                              controller: _title,
                              label: tv
                                  ? Strings.fieldSeries
                                  : Strings.fieldTitle,
                              help: tv
                                  ? Strings.fetchSeriesHelp
                                  : Strings.fetchTitleHelp,
                              error: _titleError,
                              onChanged: (_) =>
                                  _clear(() => _titleError = null),
                            ),
                          ),
                          if (!tv) ...<Widget>[
                            const SizedBox(width: Space.s3),
                            Expanded(
                              child: LabelledTextField(
                                controller: _year,
                                label: Strings.fieldYear,
                                help: Strings.fetchYearHelp,
                                keyboardType: TextInputType.number,
                                error: _yearError,
                                onChanged: (_) =>
                                    _clear(() => _yearError = null),
                              ),
                            ),
                          ],
                        ],
                      ),
                      LabelledDropdown<String>(
                        label: Strings.fieldTargetLibrary,
                        help: Strings.targetLibraryHelp,
                        value: _libraryId ?? '',
                        items: <(String, String)>[
                          for (final Library l in external)
                            (l.id, '${l.name} (${libraryKindLabel(l.kind)})'),
                        ],
                        error: _libraryError,
                        onChanged: (String v) => setState(() {
                          _libraryId = v;
                          _libraryError = null;
                          _titleError = null;
                          _yearError = null;
                          _seasonError = null;
                          _episodeError = null;
                          _episodeTitleError = null;
                        }),
                      ),
                      LabelledTextField(
                        controller: _externalId,
                        label: Strings.fieldExternalId,
                        help: Strings.fetchExternalIdHelp,
                      ),
                      if (tv)
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            Expanded(
                              child: LabelledTextField(
                                controller: _season,
                                label: Strings.fieldSeason,
                                help: Strings.seasonEpisodeHelp,
                                keyboardType: TextInputType.number,
                                error: _seasonError,
                                onChanged: (_) =>
                                    _clear(() => _seasonError = null),
                              ),
                            ),
                            const SizedBox(width: Space.s3),
                            Expanded(
                              child: LabelledTextField(
                                controller: _episode,
                                label: Strings.fieldEpisode,
                                help: Strings.seasonEpisodeHelp,
                                keyboardType: TextInputType.number,
                                error: _episodeError,
                                onChanged: (_) =>
                                    _clear(() => _episodeError = null),
                              ),
                            ),
                            const SizedBox(width: Space.s3),
                            Expanded(
                              flex: 3,
                              child: LabelledTextField(
                                controller: _episodeTitle,
                                label: Strings.fieldEpisodeTitle,
                                help: Strings.fetchEpisodeTitleHelp,
                                error: _episodeTitleError,
                                onChanged: (_) =>
                                    _clear(() => _episodeTitleError = null),
                              ),
                            ),
                          ],
                        ),
                      _downloadOptions(),
                      const SizedBox(height: Space.s2),
                      Align(
                        alignment: Alignment.centerLeft,
                        child: FilledButton.icon(
                          onPressed: _submitting ? null : () => _submit(tv),
                          icon: const Icon(Icons.cloud_download_outlined),
                          label: const Text(Strings.fetch),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            SectionBlock(
              title: Strings.queuedFetchesHeading,
              actionItems: <PageAction>[
                PageAction(
                  icon: Icons.refresh,
                  label: Strings.refresh,
                  onPressed: _reloadFetches,
                ),
                PageAction(
                  icon: Icons.open_in_new,
                  label: Strings.viewJobs,
                  onPressed: () =>
                      Navigator.of(context).pushNamed(adminJobsRoute()),
                ),
              ],
              child: _QueuedFetches(future: _fetches),
            ),
          ],
        );
      },
    );
  }
}

class _OptionBox extends StatelessWidget {
  const _OptionBox({
    required this.label,
    required this.help,
    required this.value,
    required this.onChanged,
  });

  final String label;
  final String help;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) => Row(
    mainAxisSize: MainAxisSize.min,
    children: <Widget>[
      Flexible(
        child: MergeSemantics(
          child: InkWell(
            canRequestFocus: false,
            onTap: () => onChanged(!value),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Checkbox(
                  value: value,
                  onChanged: (bool? v) => onChanged(v ?? false),
                ),
                Flexible(child: Text(label)),
                const SizedBox(width: Space.s1),
              ],
            ),
          ),
        ),
      ),
      FieldHelp(title: label, body: help),
    ],
  );
}

class _OptionsToggle extends StatelessWidget {
  const _OptionsToggle({required this.open, required this.onPressed});

  final bool open;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) => Semantics(
    button: true,
    expanded: open,
    child: InkWell(
      onTap: onPressed,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: Space.s3),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Flexible(
              child: Text(
                Strings.downloadOptionsHeading,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            const SizedBox(width: Space.s1),
            Icon(
              open ? Icons.expand_less : Icons.expand_more,
              color: context.tokens.muted,
            ),
          ],
        ),
      ),
    ),
  );
}

class _QueuedFetches extends StatelessWidget {
  const _QueuedFetches({required this.future});

  final Future<_FetchList> future;

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<_FetchList>(
      future: future,
      builder: (BuildContext context, AsyncSnapshot<_FetchList> snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const Padding(
            padding: EdgeInsets.all(Space.s4),
            child: Center(child: CircularProgressIndicator()),
          );
        }
        final _FetchList? data = snapshot.data;
        final List<Job> jobs = data?.jobs ?? const <Job>[];
        if (snapshot.hasError || jobs.isEmpty) {
          return const StatusText(Strings.emptyFetches);
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            if (data?.truncated ?? false)
              Padding(
                padding: const EdgeInsets.only(bottom: Space.s2),
                child: StatusText(Strings.showingNewest(jobs.length)),
              ),
            for (final Job j in jobs)
              Padding(
                padding: const EdgeInsets.only(bottom: Space.s2),
                child: Row(
                  children: <Widget>[
                    Expanded(
                      child: Text(j.id, overflow: TextOverflow.ellipsis),
                    ),
                    JobStatusChip(j.status),
                    TextButton(
                      onPressed: () =>
                          Navigator.of(context).pushNamed(jobRoute(j.id)),
                      child: const Text(Strings.view),
                    ),
                  ],
                ),
              ),
          ],
        );
      },
    );
  }
}
