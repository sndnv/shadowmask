import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/library_api.dart';
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
  LibraryKind _kind = LibraryKind.movie;
  String? _libraryId;
  bool _submitting = false;
  String? _urlError;
  String? _titleError;
  String? _yearError;
  String? _libraryError;
  String? _seasonError;
  String? _episodeError;

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

  Future<void> _submit() async {
    final String url = _sourceUrl.text.trim();
    final String title = _title.text.trim();
    final String? libraryId = _libraryId;
    final bool tv = _kind == LibraryKind.tv;
    final int? season = int.tryParse(_season.text.trim());
    final int? episode = int.tryParse(_episode.text.trim());
    final String yearText = _year.text.trim();
    final int? year = int.tryParse(yearText);
    final bool yearValid = year != null && year >= 1000 && year <= 9999;
    setState(() {
      _urlError = url.isEmpty ? Strings.requiredUrl : null;
      _titleError = title.isEmpty ? Strings.requiredTitle : null;
      _yearError = !tv && yearText.isNotEmpty && !yearValid
          ? Strings.invalidYear
          : null;
      _libraryError = libraryId == null ? Strings.requiredLibrary : null;
      _seasonError = tv && season == null ? Strings.requiredSeason : null;
      _episodeError = tv && episode == null ? Strings.requiredEpisode : null;
    });
    if (_urlError != null ||
        _titleError != null ||
        _yearError != null ||
        _libraryError != null ||
        _seasonError != null ||
        _episodeError != null) {
      return;
    }
    final String externalId = _externalId.text.trim();
    final Map<String, dynamic> body = <String, dynamic>{
      'source_url': url,
      'kind': libraryKindWire(_kind),
      'library_id': libraryId,
      'title': title,
      if (!tv && yearValid) 'year': year,
      if (externalId.isNotEmpty) 'external_id': externalId,
      if (tv) 'season': season,
      if (tv) 'episode': episode,
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
        final bool tv = _kind == LibraryKind.tv;
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
                            label: Strings.fieldTitle,
                            help: Strings.fetchTitleHelp,
                            error: _titleError,
                            onChanged: (_) => _clear(() => _titleError = null),
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
                              onChanged: (_) => _clear(() => _yearError = null),
                            ),
                          ),
                        ],
                      ],
                    ),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Expanded(
                          child: LabelledDropdown<LibraryKind>(
                            label: Strings.fieldKind,
                            help: Strings.fetchKindHelp,
                            value: _kind,
                            items: <(LibraryKind, String)>[
                              for (final LibraryKind k in LibraryKind.values)
                                (k, libraryKindLabel(k)),
                            ],
                            onChanged: (LibraryKind v) =>
                                setState(() => _kind = v),
                          ),
                        ),
                        const SizedBox(width: Space.s3),
                        Expanded(
                          child: LabelledDropdown<String>(
                            label: Strings.fieldTargetLibrary,
                            help: Strings.targetLibraryHelp,
                            value: _libraryId ?? '',
                            items: <(String, String)>[
                              for (final Library l in external) (l.id, l.name),
                            ],
                            error: _libraryError,
                            onChanged: (String v) => setState(() {
                              _libraryId = v;
                              _libraryError = null;
                            }),
                          ),
                        ),
                      ],
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
                        ],
                      ),
                    const SizedBox(height: Space.s2),
                    Align(
                      alignment: Alignment.centerLeft,
                      child: FilledButton.icon(
                        onPressed: _submitting ? null : _submit,
                        icon: const Icon(Icons.cloud_download_outlined),
                        label: const Text(Strings.fetch),
                      ),
                    ),
                  ],
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
