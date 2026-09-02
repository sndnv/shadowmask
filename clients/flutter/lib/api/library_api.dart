import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/library/duplicate_candidate.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/model/library/resolve_candidate.dart';
import 'package:shadowmask/model/library/scan_state.dart';
import 'package:shadowmask/model/library/unmatched_file.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/page.dart';

class LibraryApi {
  LibraryApi(this._api);

  final ApiClient _api;

  Future<List<Library>> libraries() =>
      _api.getJsonArray('/api/v1/libraries', Library.fromJson);

  Future<Library> library(String id) =>
      _api.getJson('/api/v1/libraries/${_enc(id)}', Library.fromJson);

  Future<void> createLibrary(Map<String, dynamic> body) =>
      _api.sendVoid('POST', '/api/v1/libraries', body: body);

  Future<void> updateLibrary(String id, Map<String, dynamic> body) =>
      _api.sendVoid('PUT', '/api/v1/libraries/${_enc(id)}', body: body);

  Future<void> deleteLibrary(String id) =>
      _api.sendVoid('DELETE', '/api/v1/libraries/${_enc(id)}');

  Future<ScanState> scanState(String id) =>
      _api.getJson('/api/v1/libraries/${_enc(id)}/scan', ScanState.fromJson);

  Future<void> triggerScan(String id) =>
      _api.sendVoid('POST', '/api/v1/libraries/${_enc(id)}/scan');

  Future<void> refreshMetadata(String id) =>
      _api.sendVoid('POST', '/api/v1/libraries/${_enc(id)}/refresh-metadata');

  Future<Paged<DuplicateCandidate>> duplicates(
    String id, {
    int offset = 0,
    int? limit,
  }) => _api.getPage(
    withQuery(
      '/api/v1/libraries/${_enc(id)}/duplicates',
      _pageParams(offset, limit),
    ),
    DuplicateCandidate.fromJson,
  );

  Future<void> dismissDuplicate(String id, String did) => _api.sendVoid(
    'POST',
    '/api/v1/libraries/${_enc(id)}/duplicates/${_enc(did)}/dismiss',
  );

  Future<Paged<UnmatchedFile>> unmatched(
    String id, {
    int offset = 0,
    int? limit,
  }) => _api.getPage(
    withQuery(
      '/api/v1/libraries/${_enc(id)}/unmatched',
      _pageParams(offset, limit),
    ),
    UnmatchedFile.fromJson,
  );

  Future<Paged<Version>> versions(String id, {int offset = 0, int? limit}) =>
      _api.getPage(
        withQuery(
          '/api/v1/libraries/${_enc(id)}/versions',
          _pageParams(offset, limit),
        ),
        Version.fromJson,
      );

  Future<List<ResolveCandidate>> unmatchedCandidates(String id, String uid) =>
      _api.getJsonArray(
        '/api/v1/libraries/${_enc(id)}/unmatched/${_enc(uid)}/candidates',
        ResolveCandidate.fromJson,
      );

  Future<void> resolveUnmatched(
    String id,
    String uid,
    Map<String, dynamic> target,
  ) => _api.sendVoid(
    'POST',
    '/api/v1/libraries/${_enc(id)}/unmatched/${_enc(uid)}/resolve',
    body: <String, dynamic>{'target': target},
  );

  Map<String, String?> _pageParams(int offset, int? limit) => <String, String?>{
    if (offset > 0) 'offset': offset.toString(),
    if (limit != null) 'limit': limit.toString(),
  };

  String _enc(String s) => Uri.encodeComponent(s);
}
