import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/catalog/subtitle_candidate.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/model/job/job_node.dart';
import 'package:shadowmask/model/job/jobs_feed.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/model/session/now_playing.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/page.dart';

class AdminApi {
  AdminApi(this._api);

  final ApiClient _api;

  Future<JobsFeed> jobs({
    int offset = 0,
    int? limit,
    String? filter,
    bool activeOnly = false,
  }) => _api.getJson(
    withQuery('/api/v1/admin/jobs', <String, String?>{
      ..._pageParams(offset, limit),
      if (activeOnly) 'state': 'active',
      if (filter != null && filter.isNotEmpty) 'filter': filter,
    }),
    JobsFeed.fromJson,
  );

  Future<Job> job(String id) =>
      _api.getJson('/api/v1/admin/jobs/${_enc(id)}', Job.fromJson);

  Future<Paged<JobNode>> jobChildren(String id, {int offset = 0, int? limit}) =>
      _api.getPage(
        withQuery(
          '/api/v1/admin/jobs/${_enc(id)}/children',
          _pageParams(offset, limit),
        ),
        JobNode.fromJson,
      );

  Future<void> cancelJob(String id) =>
      _api.sendVoid('POST', '/api/v1/admin/jobs/${_enc(id)}/cancel');

  Future<void> retryJob(String id) =>
      _api.sendVoid('POST', '/api/v1/admin/jobs/${_enc(id)}/retry');

  Future<List<String>> jobLog(String id, {String? level}) => _api.getJson(
    withQuery('/api/v1/admin/jobs/${_enc(id)}/logs', <String, String?>{
      'level': level,
    }),
    (Map<String, dynamic> json) => (json['lines'] as List<dynamic>)
        .map((dynamic e) => e as String)
        .toList(),
  );

  Future<void> wipeJobLog(String id) =>
      _api.sendVoid('DELETE', '/api/v1/admin/jobs/${_enc(id)}/logs');

  Future<Paged<NowPlaying>> activity({int offset = 0, int? limit}) =>
      _api.getPage(
        withQuery('/api/v1/users/activity', _pageParams(offset, limit)),
        NowPlaying.fromJson,
      );

  Future<Paged<Version>> versions({
    int offset = 0,
    int? limit,
    String? filter,
  }) => _api.getPage(
    withQuery('/api/v1/admin/versions', <String, String?>{
      ..._pageParams(offset, limit),
      if (filter != null && filter.isNotEmpty) 'filter': filter,
    }),
    Version.fromJson,
  );

  Future<List<VersionJob>> versionJobs(String id) => _api.getJsonArray(
    '/api/v1/versions/${_enc(id)}/jobs',
    VersionJob.fromJson,
  );

  Future<void> transcribe(
    String id, {
    int? audioTrackIndex,
    String? sourceLanguage,
  }) => _api.sendVoid(
    'POST',
    withQuery(
      '/api/v1/versions/${_enc(id)}/subtitles/transcribe',
      <String, String?>{
        'audio_track_index': audioTrackIndex?.toString(),
        'source_language': sourceLanguage,
      },
    ),
  );

  Future<void> translate(
    String id, {
    required String sourceSubtitleId,
    required String targetLanguage,
  }) => _api.sendVoid(
    'POST',
    withQuery(
      '/api/v1/versions/${_enc(id)}/subtitles/${_enc(sourceSubtitleId)}'
      '/translate',
      <String, String?>{'target_language': targetLanguage},
    ),
  );

  Future<void> upscale(String id, {required int targetHeight}) => _api.sendVoid(
    'POST',
    withQuery('/api/v1/versions/${_enc(id)}/upscale', <String, String?>{
      'target_height': '$targetHeight',
    }),
  );

  Future<void> combineSubtitles(
    String id, {
    required String topSubtitleId,
    required String bottomSubtitleId,
  }) => _api.sendVoid(
    'POST',
    withQuery(
      '/api/v1/versions/${_enc(id)}/subtitles/${_enc(topSubtitleId)}/combine',
      <String, String?>{'bottom_subtitle_id': bottomSubtitleId},
    ),
  );

  Future<void> deleteVersion(String id) =>
      _api.sendVoid('DELETE', '/api/v1/versions/${_enc(id)}');

  Future<void> deleteMovie(String id) =>
      _api.sendVoid('DELETE', '/api/v1/admin/movies/${_enc(id)}');

  Future<void> deleteSeries(String id) =>
      _api.sendVoid('DELETE', '/api/v1/admin/series/${_enc(id)}');

  Future<void> deleteSeason(String id) =>
      _api.sendVoid('DELETE', '/api/v1/admin/seasons/${_enc(id)}');

  Future<void> deleteEpisode(String id) =>
      _api.sendVoid('DELETE', '/api/v1/admin/episodes/${_enc(id)}');

  Future<void> relinkSeries(String id, Map<String, dynamic> target) =>
      _api.sendVoid(
        'POST',
        '/api/v1/series/${_enc(id)}/relink',
        body: <String, dynamic>{'target': target},
      );

  Future<void> relink(String id, Map<String, dynamic> target) => _api.sendVoid(
    'POST',
    '/api/v1/versions/${_enc(id)}/relink',
    body: <String, dynamic>{'target': target},
  );

  Future<void> refreshMetadata(
    TitleKind kind,
    String id, {
    Map<String, String>? externalId,
    bool force = false,
  }) {
    final String collection = switch (kind) {
      TitleKind.movie => 'movies',
      TitleKind.series => 'series',
      _ => throw ArgumentError.value(kind, 'kind', 'has no refresh route'),
    };
    final Map<String, dynamic> body = <String, dynamic>{
      'external_id': ?externalId,
      if (force) 'force': true,
    };
    return _api.sendVoid(
      'POST',
      '/api/v1/$collection/${_enc(id)}/refresh',
      body: body.isEmpty ? null : body,
    );
  }

  Future<void> editMovie(String id, Map<String, dynamic> body) =>
      _api.sendVoid('PUT', '/api/v1/movies/${_enc(id)}', body: body);

  Future<void> editSeries(String id, Map<String, dynamic> body) =>
      _api.sendVoid('PUT', '/api/v1/series/${_enc(id)}', body: body);

  Future<void> editEpisode(
    String seriesId,
    String seasonId,
    String id,
    Map<String, dynamic> body,
  ) => _api.sendVoid(
    'PUT',
    '/api/v1/series/${_enc(seriesId)}/seasons/${_enc(seasonId)}'
        '/episodes/${_enc(id)}',
    body: body,
  );

  Future<List<SubtitleCandidate>> subtitleSearch(
    String id, {
    String? query,
    String? language,
  }) => _api.getJsonArray(
    withQuery(
      '/api/v1/versions/${_enc(id)}/subtitles/search',
      <String, String?>{
        if (query != null && query.isNotEmpty) 'q': query,
        if (language != null && language.isNotEmpty) 'language': language,
      },
    ),
    SubtitleCandidate.fromJson,
  );

  Future<bool> subtitleDownload(
    String id, {
    required String fileId,
    String? language,
    String? releaseName,
  }) async =>
      await _api.sendStatus(
        'POST',
        '/api/v1/versions/${_enc(id)}/subtitles/download',
        body: <String, dynamic>{
          'file_id': fileId,
          if (language != null && language.isNotEmpty) 'language': language,
          if (releaseName != null && releaseName.isNotEmpty)
            'release_name': releaseName,
        },
      ) !=
      200;

  Future<String> subtitleText(String id, String sid) => _api.getJson(
    '/api/v1/versions/${_enc(id)}/subtitles/${_enc(sid)}',
    (Map<String, dynamic> json) => json['content'] as String? ?? '',
  );

  Future<void> renameSubtitle(String id, String sid, {String? language}) =>
      _api.sendVoid(
        'PUT',
        '/api/v1/versions/${_enc(id)}/subtitles/${_enc(sid)}',
        body: <String, dynamic>{
          if (language != null && language.isNotEmpty) 'language': language,
        },
      );

  Future<void> deleteSubtitle(String id, String sid) => _api.sendVoid(
    'DELETE',
    '/api/v1/versions/${_enc(id)}/subtitles/${_enc(sid)}',
  );

  Future<void> createFetch(Map<String, dynamic> body) =>
      _api.sendVoid('POST', '/api/v1/admin/fetch', body: body);

  Map<String, String?> _pageParams(int offset, int? limit) => <String, String?>{
    if (offset > 0) 'offset': offset.toString(),
    if (limit != null) 'limit': limit.toString(),
  };

  String _enc(String s) => Uri.encodeComponent(s);
}
