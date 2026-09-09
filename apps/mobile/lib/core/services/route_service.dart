import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

/// Planned route for the day's remaining stops.
class PlannedRoute {
  const PlannedRoute({required this.order, required this.legMinutes, required this.totalKm, required this.live, this.polyline});

  /// Indices into the stops passed to [RouteService.optimize], in visit order.
  /// When a separate [origin] was supplied, these index the full `waypoints`
  /// list; otherwise they index `waypoints.skip(1)` (the legacy contract).
  final List<int> order;
  final List<int> legMinutes;
  final double totalKm;

  /// True when Google Directions supplied road geometry; false for the
  /// server's straight-line fallback, which must not be shown as road routing.
  final bool live;
  final String? polyline;

  int get totalMinutes => legMinutes.fold(0, (a, b) => a + b);
}

/// Calls the Altius route endpoints. The Google key lives on the server —
/// the app never holds a maps credential and never talks to Google directly.
class RouteService {
  const RouteService();

  Future<Map<String, dynamic>> _post(String baseUrl, String token, String path, Object body) async {
    final client = HttpClient();
    try {
      final req = await client.postUrl(Uri.parse('$baseUrl$path'));
      req.headers.contentType = ContentType.json;
      req.headers.set('authorization', 'Bearer $token');
      req.write(jsonEncode(body));
      final res = await req.close().timeout(const Duration(seconds: 15));
      final text = await res.transform(utf8.decoder).join();
      if (res.statusCode != 200) throw HttpException('route ${res.statusCode}');
      return jsonDecode(text) as Map<String, dynamic>;
    } finally {
      client.close();
    }
  }

  Future<PlannedRoute> optimize({
    required String baseUrl,
    required String token,
    required List<({double lat, double lng})> waypoints,
    ({double lat, double lng})? origin,
  }) async {
    // With a separate origin (driver's live GPS), all waypoints are stops.
    // Without one, the first waypoint is the origin and the rest are stops —
    // the legacy contract that keeps the call valid for callers that don't
    // have a GPS fix.
    final actualOrigin = origin ?? waypoints.first;
    final stops = (origin == null ? waypoints.skip(1) : waypoints)
        .map((w) => {'lat': w.lat, 'lng': w.lng})
        .toList();
    if (stops.isEmpty) throw ArgumentError('needs at least one stop beyond the origin');
    final body = {'origin': {'lat': actualOrigin.lat, 'lng': actualOrigin.lng}, 'waypoints': stops};
    final data = (await _post(baseUrl, token, '/api/v3/route/optimize', body))['data'] as Map<String, dynamic>;

    final order = ((data['order'] as List?) ?? const []).map((e) => (e as num).toInt()).toList();
    final legs = ((data['leg_seconds'] as List?) ?? const [])
        .map((e) => ((e as num) / 60).round())
        .toList();
    return PlannedRoute(
      order: order,
      legMinutes: legs,
      totalKm: ((data['total_meters'] as num?)?.toDouble() ?? 0) / 1000,
      live: data['source'] == 'live',
      polyline: data['polyline'] as String?,
    );
  }

  /// Route rendered as a PNG by the backend proxy. Null when maps are not
  /// configured — the caller shows the stop list without an image.
  Future<Uint8List?> mapImage({
    required String baseUrl,
    required String token,
    required List<({double lat, double lng})> markers,
    String? polyline,
    int width = 640,
    int height = 320,
  }) async {
    final client = HttpClient();
    try {
      final req = await client.postUrl(Uri.parse('$baseUrl/api/v3/route/static-map'));
      req.headers.contentType = ContentType.json;
      req.headers.set('authorization', 'Bearer $token');
      req.write(jsonEncode({
        'markers': markers.map((m) => {'lat': m.lat, 'lng': m.lng}).toList(),
        'polyline': polyline,
        'width': width,
        'height': height,
      }));
      final res = await req.close().timeout(const Duration(seconds: 15));
      if (res.statusCode != 200) {
        await res.drain<void>();
        return null;
      }
      final chunks = await res.toList();
      return Uint8List.fromList(chunks.expand((c) => c).toList());
    } on Object {
      return null;
    } finally {
      client.close();
    }
  }
}
