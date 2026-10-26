class ApiException implements Exception {
  ApiException(
    this.message, {
    this.status,
    this.code,
    this.jobs = const <Map<String, dynamic>>[],
  });

  final String message;
  final int? status;
  final String? code;
  final List<Map<String, dynamic>> jobs;

  @override
  String toString() => message;
}
