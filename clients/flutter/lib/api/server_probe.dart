import 'package:http/http.dart' as http;

const Duration kServerProbeTimeout = Duration(seconds: 5);

Future<bool> serverAnswers(String baseUrl, {http.Client? httpClient}) async {
  final http.Client client = httpClient ?? http.Client();
  try {
    final http.Response res = await client
        .get(Uri.parse('$baseUrl/api/v1/server/info'))
        .timeout(kServerProbeTimeout);
    return res.statusCode == 200 || res.statusCode == 401;
  } catch (_) {
    return false;
  } finally {
    if (httpClient == null) {
      client.close();
    }
  }
}
