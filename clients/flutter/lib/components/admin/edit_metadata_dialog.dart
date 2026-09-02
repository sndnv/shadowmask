import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/components/admin/admin_field_row.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/edited_notice.dart';
import 'package:shadowmask/components/admin/form_dialog.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/movie_detail.dart';
import 'package:shadowmask/model/catalog/series_detail.dart';
import 'package:shadowmask/model/server/rating_system.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/failure_reason.dart';

enum EditTarget { movie, series, episode }

final RegExp _airDatePattern = RegExp(r'^\d{4}-\d{2}-\d{2}$');

String? shortAirDate(String? raw) {
  if (raw == null || raw.length < 10) {
    return raw;
  }
  return raw.substring(0, 10);
}

String externalIdOf(List<ExternalId> ids, String source) => ids
    .firstWhere(
      (ExternalId id) => id.source == source,
      orElse: () => const ExternalId(source: '', value: ''),
    )
    .value;

Future<bool> showEditMovieDialog(
  BuildContext context, {
  required AdminApi admin,
  required MovieDetail movie,
  required List<RatingSystem> ratingSystems,
}) => _show(
  context,
  EditMetadataDialog(
    admin: admin,
    target: EditTarget.movie,
    id: movie.id,
    title: movie.title,
    year: movie.year,
    overview: movie.overview,
    runtimeMinutes: movie.runtimeMinutes,
    ratingSystem: movie.contentRating?.system,
    ratingCode: movie.contentRating?.code,
    ratingSystems: ratingSystems,
    tmdbId: externalIdOf(movie.externalIds, 'tmdb'),
    imdbId: externalIdOf(movie.externalIds, 'imdb'),
    manuallyEdited: movie.manuallyEdited,
  ),
);

Future<bool> showEditSeriesDialog(
  BuildContext context, {
  required AdminApi admin,
  required SeriesDetail series,
  required List<RatingSystem> ratingSystems,
}) => _show(
  context,
  EditMetadataDialog(
    admin: admin,
    target: EditTarget.series,
    id: series.id,
    title: series.title,
    year: series.year,
    overview: series.overview,
    ratingSystem: series.contentRating?.system,
    ratingCode: series.contentRating?.code,
    ratingSystems: ratingSystems,
    tmdbId: externalIdOf(series.externalIds, 'tmdb'),
    imdbId: externalIdOf(series.externalIds, 'imdb'),
    manuallyEdited: series.manuallyEdited,
  ),
);

Future<bool> showEditEpisodeDialog(
  BuildContext context, {
  required AdminApi admin,
  required Episode episode,
  required String seriesId,
}) => _show(
  context,
  EditMetadataDialog(
    admin: admin,
    target: EditTarget.episode,
    id: episode.id,
    seriesId: seriesId,
    seasonId: episode.seasonId,
    title: episode.title,
    overview: episode.overview,
    runtimeMinutes: episode.runtimeMinutes,
    airDate: shortAirDate(episode.airDate),
    manuallyEdited: episode.manuallyEdited,
  ),
);

Future<bool> _show(BuildContext context, EditMetadataDialog dialog) async {
  final bool? saved = await showDialog<bool>(
    context: context,
    builder: (BuildContext _) => dialog,
  );
  return saved ?? false;
}

class EditMetadataDialog extends StatefulWidget {
  const EditMetadataDialog({
    super.key,
    required this.admin,
    required this.target,
    required this.id,
    required this.title,
    this.seriesId,
    this.seasonId,
    this.year,
    this.overview,
    this.runtimeMinutes,
    this.airDate,
    this.ratingSystem,
    this.ratingCode,
    this.ratingSystems = const <RatingSystem>[],
    this.tmdbId,
    this.imdbId,
    this.manuallyEdited = false,
  });

  final AdminApi admin;
  final EditTarget target;
  final String id;
  final bool manuallyEdited;
  final String? seriesId;
  final String? seasonId;
  final String title;
  final int? year;
  final String? overview;
  final int? runtimeMinutes;
  final String? airDate;
  final String? ratingSystem;
  final String? ratingCode;
  final List<RatingSystem> ratingSystems;
  final String? tmdbId;
  final String? imdbId;

  @override
  State<EditMetadataDialog> createState() => _EditMetadataDialogState();
}

class _EditMetadataDialogState extends State<EditMetadataDialog> {
  late final TextEditingController _title = TextEditingController(
    text: widget.title,
  );
  late final TextEditingController _year = TextEditingController(
    text: widget.year?.toString() ?? '',
  );
  late final TextEditingController _overview = TextEditingController(
    text: widget.overview ?? '',
  );
  late final TextEditingController _runtime = TextEditingController(
    text: widget.runtimeMinutes?.toString() ?? '',
  );
  late final TextEditingController _airDate = TextEditingController(
    text: widget.airDate ?? '',
  );
  late final TextEditingController _tmdb = TextEditingController(
    text: widget.tmdbId?.isNotEmpty == true
        ? widget.tmdbId!
        : Strings.noneRecorded,
  );
  late final TextEditingController _imdb = TextEditingController(
    text: widget.imdbId?.isNotEmpty == true
        ? widget.imdbId!
        : Strings.noneRecorded,
  );

  late String _system = (widget.ratingSystem ?? '').toLowerCase();
  late String _code = (widget.ratingCode ?? '').toLowerCase();

  bool _submitting = false;
  String? _titleError;
  String? _yearError;
  String? _runtimeError;
  String? _airDateError;

  bool get _hasYear => widget.target != EditTarget.episode;
  bool get _hasRuntime => widget.target != EditTarget.series;
  bool get _hasRating => widget.target != EditTarget.episode;
  bool get _hasAirDate => widget.target == EditTarget.episode;
  bool get _hasIds => widget.target != EditTarget.episode;

  List<String> get _systemKeys {
    final List<String> known = widget.ratingSystems
        .map((RatingSystem s) => s.system)
        .toList();
    if (_system.isNotEmpty && !known.contains(_system)) {
      known.insert(0, _system);
    }
    return known;
  }

  List<String> get _codes {
    final List<String> known = widget.ratingSystems
        .where((RatingSystem s) => s.system == _system)
        .expand((RatingSystem s) => s.codes)
        .toList();
    if (_code.isNotEmpty && !known.contains(_code)) {
      known.insert(0, _code);
    }
    return known;
  }

  @override
  void dispose() {
    _title.dispose();
    _year.dispose();
    _overview.dispose();
    _runtime.dispose();
    _airDate.dispose();
    _tmdb.dispose();
    _imdb.dispose();
    super.dispose();
  }

  String? _trimmedOrNull(TextEditingController controller) {
    final String value = controller.text.trim();
    return value.isEmpty ? null : value;
  }

  void _pickSystem(String system) {
    setState(() {
      _system = system;
      _code = '';
    });
  }

  bool _validate() {
    String? titleError;
    String? yearError;
    String? runtimeError;
    String? airDateError;

    if (_title.text.trim().isEmpty) {
      titleError = Strings.requiredTitle;
    }
    final String year = _year.text.trim();
    if (_hasYear && year.isNotEmpty && int.tryParse(year) == null) {
      yearError = Strings.invalidYear;
    }
    final String runtime = _runtime.text.trim();
    if (_hasRuntime && runtime.isNotEmpty) {
      final int? parsed = int.tryParse(runtime);
      if (parsed == null || parsed < 0) {
        runtimeError = Strings.invalidRuntime;
      }
    }
    final String airDate = _airDate.text.trim();
    if (_hasAirDate &&
        airDate.isNotEmpty &&
        !_airDatePattern.hasMatch(airDate)) {
      airDateError = Strings.invalidAirDate;
    }

    setState(() {
      _titleError = titleError;
      _yearError = yearError;
      _runtimeError = runtimeError;
      _airDateError = airDateError;
    });
    return titleError == null &&
        yearError == null &&
        runtimeError == null &&
        airDateError == null;
  }

  Map<String, dynamic> _body() => <String, dynamic>{
    'title': _title.text.trim(),
    'overview': _trimmedOrNull(_overview),
    if (_hasYear)
      'year': _year.text.trim().isEmpty ? null : int.parse(_year.text.trim()),
    if (_hasRuntime)
      'runtime_minutes': _runtime.text.trim().isEmpty
          ? null
          : int.parse(_runtime.text.trim()),
    if (_hasAirDate) 'air_date': _trimmedOrNull(_airDate),
    if (_hasRating)
      'content_rating': _system.isEmpty || _code.isEmpty
          ? null
          : <String, String>{'system': _system, 'code': _code},
  };

  Future<void> _send(Map<String, dynamic> body) {
    switch (widget.target) {
      case EditTarget.movie:
        return widget.admin.editMovie(widget.id, body);
      case EditTarget.series:
        return widget.admin.editSeries(widget.id, body);
      case EditTarget.episode:
        return widget.admin.editEpisode(
          widget.seriesId ?? '',
          widget.seasonId ?? '',
          widget.id,
          body,
        );
    }
  }

  Future<void> _submit() async {
    if (!_validate()) {
      return;
    }
    final bool ok = await confirmDialog(
      context,
      title: Strings.confirmSaveEditHeading,
      message: Strings.confirmSaveEditBody,
      confirmLabel: Strings.save,
      danger: false,
    );
    if (!ok || !mounted) {
      return;
    }
    setState(() => _submitting = true);
    try {
      await _send(_body());
      if (mounted) {
        Toasts.of(context).success(Strings.toastMetadataSaved);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorSave, e));
        setState(() => _submitting = false);
      }
    }
  }

  void _clearTitleError(String _) {
    if (_titleError != null) {
      setState(() => _titleError = null);
    }
  }

  Widget _pair(Widget left, Widget right) => AdminFieldRow(
    crossAxisAlignment: CrossAxisAlignment.start,
    fields: <AdminField>[AdminField(flex: 1, left), AdminField(flex: 1, right)],
  );

  @override
  Widget build(BuildContext context) {
    return FormDialog(
      title: Strings.editDetails,
      submitting: _submitting,
      onSubmit: _submit,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        spacing: Space.s3,
        children: <Widget>[
          if (widget.manuallyEdited) const EditedNotice(),
          LabelledTextField(
            controller: _title,
            label: Strings.fieldTitle,
            error: _titleError,
            onChanged: _clearTitleError,
          ),
          LabelledTextField(
            controller: _overview,
            label: Strings.fieldOverview,
            maxLines: 4,
          ),
          if (_hasYear && _hasRuntime)
            _pair(_yearField(), _runtimeField())
          else if (_hasYear)
            _yearField()
          else if (_hasRuntime)
            _runtimeField(),
          if (_hasAirDate)
            LabelledTextField(
              controller: _airDate,
              label: Strings.fieldAirDate,
              hint: Strings.airDateHint,
              error: _airDateError,
            ),
          if (_hasRating)
            _pair(
              LabelledDropdown<String>(
                label: Strings.fieldRatingSystem,
                value: _system,
                enabled: !_submitting,
                items: <(String, String)>[
                  ('', Strings.optionNone),
                  for (final String s in _systemKeys) (s, s.toUpperCase()),
                ],
                onChanged: _pickSystem,
              ),
              LabelledDropdown<String>(
                label: Strings.fieldRatingCode,
                value: _code,
                enabled: !_submitting,
                items: <(String, String)>[
                  ('', Strings.optionNone),
                  for (final String code in _codes) (code, code.toUpperCase()),
                ],
                onChanged: (String v) => setState(() => _code = v),
              ),
            ),
          if (_hasIds)
            _pair(
              LabelledTextField(
                controller: _tmdb,
                label: Strings.fieldTmdbId,
                help: Strings.idsAreNotEditable,
                readOnly: true,
              ),
              LabelledTextField(
                controller: _imdb,
                label: Strings.fieldImdbId,
                readOnly: true,
              ),
            ),
        ],
      ),
    );
  }

  Widget _yearField() => LabelledTextField(
    controller: _year,
    label: Strings.fieldYear,
    error: _yearError,
    keyboardType: TextInputType.number,
  );

  Widget _runtimeField() => LabelledTextField(
    controller: _runtime,
    label: Strings.fieldRuntimeMinutes,
    error: _runtimeError,
    keyboardType: TextInputType.number,
  );
}
