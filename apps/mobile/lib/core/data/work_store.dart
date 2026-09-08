import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'package:drift/drift.dart';
import 'package:drift/native.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import '../services/auth_service.dart';

/// Where bearer credentials live. Backed by the platform keystore/keychain in
/// production; the SQLite `preferences` table is app-private but unencrypted,
/// so a stolen device, an unencrypted backup, or root access reads it directly.
abstract class TokenStore {
  Future<String> read(String key);
  Future<void> write(String key, String value);
  Future<void> clear();
}

class SecureTokenStore implements TokenStore {
  const SecureTokenStore();
  static const _keys = ['accessToken', 'refreshToken', 'accessExpiresAt'];
  static const _secure = FlutterSecureStorage(
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock_this_device),
  );
  @override
  Future<String> read(String key) async => await _secure.read(key: key) ?? '';
  @override
  Future<void> write(String key, String value) => _secure.write(key: key, value: value);
  @override
  Future<void> clear() async {
    for (final k in _keys) {
      await _secure.delete(key: k);
    }
  }
}

/// Rejects anything that is not HTTPS with a host. A plain `http://` base sends
/// the driver's password and every later bearer token in the clear.
Uri parseServerUrl(String raw) {
  final uri = Uri.tryParse(raw.trim());
  if (uri == null || uri.scheme != 'https' || uri.host.isEmpty) {
    throw ArgumentError('invalidServer');
  }
  return uri;
}

enum TaskStage { assigned, arrived, working, done }

class FieldTask {
  const FieldTask(this.id, this.title, this.address, this.stage, this.etaMinutes, {this.lat, this.lng});
  final String id;
  final String title;
  final String address;
  final TaskStage stage;
  final int etaMinutes;
  /// Stop position from the API. Null for tasks created on-device, which have
  /// only an address until something geocodes it.
  final double? lat;
  final double? lng;
  bool get isLocated => lat != null && lng != null;
}

class WorkEvent {
  WorkEvent(QueryRow row)
    : id = row.read<String>('id'),
      entity = row.read<String>('entity'),
      kind = row.read<String>('kind'),
      utc = DateTime.parse(row.read<String>('utc')),
      offsetMinutes = row.read<int>('offset_minutes'),
      day = row.read<String>('day'),
      delivery = row.read<String>('delivery'),
      requestKey = row.read<String?>('request_key'),
      payload = Map.unmodifiable(jsonDecode(row.read<String>('payload')) as Map<String, dynamic>);
  final String id;
  final String entity;
  final String kind;
  final DateTime utc;
  final int offsetMinutes;
  final String day;
  final String delivery;
  final String? requestKey;
  final Map<String, dynamic> payload;
}

class CostEntry {
  CostEntry(QueryRow row)
    : category = row.read<String>('category'),
      amount = row.read<int>('amount'),
      note = row.read<String>('note'),
      day = row.read<String>('day');
  final String category;
  final int amount;
  final String note;
  final String day;
}

class DailyReport {
  DailyReport(QueryRow row)
    : day = row.read<String>('day'),
      total = row.read<int>('total'),
      completed = row.read<int>('completed'),
      visited = row.read<int>('visited'),
      driverName = row.read<String?>('driver_name') ?? '',
      vehicleNumber = row.read<String?>('vehicle_number') ?? '',
      odometerStart = row.read<int?>('odometer_start') ?? 0,
      odometerEnd = row.read<int?>('odometer_end') ?? 0,
      notes = row.read<String?>('notes') ?? '';
  final String day;
  final int total;
  final int completed;
  final int visited;
  final String driverName;
  final String vehicleNumber;
  final int odometerStart;
  final int odometerEnd;
  final String notes;
}

class _Database extends GeneratedDatabase {
  _Database(super.executor);
  @override
  int get schemaVersion => 5;
  @override
  Iterable<TableInfo<Table, Object?>> get allTables => const [];
  @override
  List<DatabaseSchemaEntity> get allSchemaEntities => const [];
  Future<void> _upgrade() async {
    await customStatement('ALTER TABLE events ADD COLUMN request_key TEXT');
    await customStatement('CREATE UNIQUE INDEX event_request ON events(request_key)');
    await customStatement("CREATE TRIGGER events_no_update BEFORE UPDATE ON events BEGIN SELECT RAISE(ABORT, 'immutable event'); END");
    await customStatement("CREATE TRIGGER events_no_delete BEFORE DELETE ON events BEGIN SELECT RAISE(ABORT, 'immutable event'); END");
    await customStatement("CREATE TRIGGER reports_no_update BEFORE UPDATE ON reports BEGIN SELECT RAISE(ABORT, 'immutable report'); END");
    await customStatement("CREATE TRIGGER reports_no_delete BEFORE DELETE ON reports BEGIN SELECT RAISE(ABORT, 'immutable report'); END");
  }
  Future<void> _upgrade3() async {
    await customStatement('DROP TRIGGER IF EXISTS events_no_update');
    await customStatement("""CREATE TRIGGER events_no_update BEFORE UPDATE ON events
      WHEN NOT (
        OLD.id IS NEW.id AND OLD.entity IS NEW.entity AND OLD.kind IS NEW.kind
        AND OLD.utc IS NEW.utc AND OLD.offset_minutes IS NEW.offset_minutes
        AND OLD.day IS NEW.day AND OLD.payload IS NEW.payload
        AND OLD.request_key IS NEW.request_key
      ) BEGIN SELECT RAISE(ABORT, 'immutable event'); END""");
  }
  Future<void> _upgrade4() async {
    await customStatement('ALTER TABLE tasks ADD COLUMN lat REAL');
    await customStatement('ALTER TABLE tasks ADD COLUMN lng REAL');
  }
  Future<void> _upgrade5() async {
    await customStatement('ALTER TABLE reports ADD COLUMN driver_name TEXT NOT NULL DEFAULT ""');
    await customStatement('ALTER TABLE reports ADD COLUMN vehicle_number TEXT NOT NULL DEFAULT ""');
    await customStatement('ALTER TABLE reports ADD COLUMN odometer_start INTEGER NOT NULL DEFAULT 0');
    await customStatement('ALTER TABLE reports ADD COLUMN odometer_end INTEGER NOT NULL DEFAULT 0');
    await customStatement('ALTER TABLE reports ADD COLUMN notes TEXT NOT NULL DEFAULT ""');
  }
  @override
  MigrationStrategy get migration => MigrationStrategy(onCreate: (m) async {
    await customStatement('CREATE TABLE tasks (id TEXT PRIMARY KEY, title TEXT NOT NULL, address TEXT NOT NULL, stage INTEGER NOT NULL DEFAULT 0 CHECK(stage BETWEEN 0 AND 3), eta INTEGER NOT NULL)');
    await customStatement('CREATE TABLE preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL)');
    await customStatement("CREATE TABLE events (id TEXT PRIMARY KEY, entity TEXT NOT NULL, kind TEXT NOT NULL, utc TEXT NOT NULL, offset_minutes INTEGER NOT NULL, day TEXT NOT NULL, delivery TEXT NOT NULL DEFAULT 'pending', payload TEXT NOT NULL)");
    await customStatement('CREATE TABLE costs (id TEXT PRIMARY KEY, category TEXT NOT NULL, amount INTEGER NOT NULL CHECK(amount > 0), note TEXT NOT NULL, day TEXT NOT NULL)');
    await customStatement('CREATE TABLE reports (day TEXT PRIMARY KEY, total INTEGER NOT NULL, completed INTEGER NOT NULL, visited INTEGER NOT NULL, driver_name TEXT NOT NULL DEFAULT "", vehicle_number TEXT NOT NULL DEFAULT "", odometer_start INTEGER NOT NULL DEFAULT 0, odometer_end INTEGER NOT NULL DEFAULT 0, notes TEXT NOT NULL DEFAULT "")');
    await _upgrade();
    await _upgrade3();
    await _upgrade4();
  }, onUpgrade: (m, from, to) async {
    if (from < 2) { await _upgrade(); }
    if (from < 3) { await _upgrade3(); }
    if (from < 4) { await _upgrade4(); }
    if (from < 5) { await _upgrade5(); }
  });
}

class WorkStore {
  WorkStore.open(
    String path, {
    DateTime Function()? now,
    int Function()? offset,
    TokenStore? tokens,
    AuthService? auth,
    this._demoWorkspace = true,
  })  : _db = _Database(NativeDatabase(File(path), setup: (db) {
          db.execute('PRAGMA journal_mode=WAL');
          db.execute('PRAGMA synchronous=FULL');
        })),
        _now = now ?? DateTime.now,
        _offset = offset ?? (() => DateTime.now().timeZoneOffset.inMinutes),
        _tokens = tokens ?? const SecureTokenStore(),
        _auth = auth ?? AuthService();
  final _Database _db;
  final TokenStore _tokens;
  final AuthService _auth;
  final DateTime Function() _now;
  final int Function() _offset;
  final bool _demoWorkspace;
  Future<void> _tail = Future.value();
  final Random _random = Random.secure();
  String get today => _day(_now().toUtc(), _offset());
  String _day(DateTime utc, int offset) => utc.add(Duration(minutes: offset)).toIso8601String().substring(0, 10);
  String newRequestId() => List.generate(16, (_) => _random.nextInt(256).toRadixString(16).padLeft(2, '0')).join();

  Future<T> _write<T>(Future<T> Function() action) {
    final next = _tail.then((_) => _db.transaction(action));
    _tail = next.then<void>((_) {}, onError: (Object e0, StackTrace s0) {});
    return next;
  }

  Future<void> initialize() async {
    await _db.customSelect('SELECT 1').get();
    await _write(() async {
      await _db.customStatement("INSERT OR IGNORE INTO preferences VALUES ('organization', 'Altius Demo')");
      await _db.customStatement("INSERT OR IGNORE INTO preferences VALUES ('hub', 'Jakarta')");
      await _db.customStatement("INSERT OR IGNORE INTO preferences VALUES ('language', 'id')");
      if (_demoWorkspace) {
        for (final task in [
          ['JKT-001', 'Nusantara Market', 'Jl. Sudirman 24, Jakarta Selatan', 18],
          ['JKT-002', 'Cendana Distribution', 'Jl. Gatot Subroto 18, Jakarta Selatan', 32],
          ['JKT-003', 'Meridian Fresh', 'Jl. Rasuna Said 8, Jakarta Selatan', 47],
          ['JKT-004', 'Taman Sari Store', 'Jl. Kuningan 12, Jakarta Selatan', 61],
        ]) {
          await _db.customStatement('INSERT OR IGNORE INTO tasks (id,title,address,eta) VALUES (?,?,?,?)', task);
        }
      }
    });
  }

  Future<String> preference(String key) async => (await _db.customSelect('SELECT value FROM preferences WHERE key = ?', variables: [Variable(key)]).getSingleOrNull())?.read<String>('value') ?? '';
  Future<void> _pref(String key, String value) => _db.customStatement('INSERT OR REPLACE INTO preferences VALUES (?,?)', [key, value]);
  Future<void> saveDraft(String key, String value) => _write(() => _pref('draft:$key', value));
  Future<String> draft(String key) => preference('draft:$key');
  Future<void> session(bool active) => _write(() => _pref('session', active ? 'true' : ''));

  /// Bearer token for outbound calls. Delegates to the secure auth service.
  Future<String?> accessToken() => _auth.accessToken();

  /// Persist the Keycloak / API configuration used by the PKCE flow.
  Future<void> configureAuth({
    required String apiBase,
    required String issuer,
    required String clientId,
    required String redirectUri,
  }) async {
    final url = parseServerUrl(apiBase).toString().replaceAll(RegExp(r'/+$'), '');
    await _auth.configure(issuer: issuer, clientId: clientId, redirectUri: redirectUri, apiBase: url);
    await _pref('apiBase', url);
  }

  /// Authenticate with Keycloak PKCE and hydrate the workspace from the API.
  Future<void> signInWithKeycloak() async {
    final token = await _auth.login();
    final base = await _auth.apiBase();
    if (base == null || base.isEmpty) throw StateError('serverError');
    final client = HttpClient();
    String? subject;
    String? organization;
    String? hub;
    List<Map<String, dynamic>>? taskRows;
    try {
      final meReq = await client.getUrl(Uri.parse('$base/api/v3/auth/me'));
      meReq.headers.set('authorization', 'Bearer $token');
      final meRes = await meReq.close().timeout(const Duration(seconds: 15));
      final meBody = await meRes.transform(utf8.decoder).join();
      if (meRes.statusCode != 200) throw StateError('unauthorized');
      final meData = jsonDecode(meBody) as Map<String, dynamic>;
      final profile = meData['data'] as Map<String, dynamic>?;
      subject = profile?['subject'] as String?;
      organization = profile?['organization'] as String?;
      hub = profile?['hub'] as String?;

      final tReq = await client.getUrl(Uri.parse('$base/api/v3/tasks'));
      tReq.headers.set('authorization', 'Bearer $token');
      final tRes = await tReq.close().timeout(const Duration(seconds: 15));
      final tBody = await tRes.transform(utf8.decoder).join();
      if (tRes.statusCode == 200) {
        final tData = jsonDecode(tBody) as Map<String, dynamic>;
        organization ??= (tData['meta'] as Map<String, dynamic>?)?['organization'] as String?;
        taskRows = (tData['data'] as List?)?.cast<Map<String, dynamic>>();
      } else {
        throw StateError('serverError');
      }
    } finally {
      client.close();
    }
    final tasks = _parseTasks(taskRows ?? []);
    await _write(() async {
      await _db.customStatement('DELETE FROM tasks');
      for (final t in tasks) {
        await _db.customStatement('INSERT OR REPLACE INTO tasks (id,title,address,stage,eta,lat,lng) VALUES (?,?,?,?,?,?,?)', t);
      }
      await _pref('apiBase', base);
      if (subject != null && subject.isNotEmpty) await _pref('driverId', subject);
      if (organization != null && organization.isNotEmpty) await _pref('organization', organization);
      if (hub != null && hub.isNotEmpty) await _pref('hub', hub);
      await _pref('session', 'true');
    });
  }

  /// Coordinates arrive as numbers or numeric strings depending on the shape
  /// TypeDB fetched. Anything else is absent, not zero — 0,0 is a real place.
  static double? _finite(Object? v) {
    final n = v is num ? v.toDouble() : (v is String ? double.tryParse(v) : null);
    return (n != null && n.isFinite) ? n : null;
  }

  List<List<Object?>> _parseTasks(List<Map<String, dynamic>> rows) {
    final out = <List<Object?>>[];
    for (final row in rows) {
      final id = row['id'] as String? ?? '';
      final title = row['title'] as String? ?? '';
      final stops = (row['stops'] as List?)?.cast<Map<String, dynamic>>() ?? const <Map<String, dynamic>>[];
      String address = '';
      var stage = TaskStage.assigned;
      double? lat;
      double? lng;
      if (stops.isNotEmpty) {
        final stop = stops.firstWhere(
          (s) => (s['stage'] as String?) != 'completed' && (s['stage'] as String?) != 'departed' && (s['stage'] as String?) != 'skipped',
          orElse: () => stops.last,
        );
        address = stop['address'] as String? ?? '';
        stage = _mapStopStage(stop['stage'] as String? ?? 'pending');
        lat = _finite(stop['latitude']);
        lng = _finite(stop['longitude']);
      }
      final eta = (row['eta_minutes'] as num?)?.toInt() ?? (stops.isNotEmpty ? (stops.first['eta_minutes'] as num?)?.toInt() ?? 0 : 0);
      if (id.isNotEmpty) out.add([id, title, address, stage.index, eta, lat, lng]);
    }
    return out;
  }

  TaskStage _mapStopStage(String stage) => switch (stage) {
    'arrived' => TaskStage.arrived,
    'working' => TaskStage.working,
    'done' || 'completed' || 'departed' => TaskStage.done,
    _ => TaskStage.assigned,
  };

  Future<void> logout() async {
    await _auth.logout();
    await _tokens.clear();
    await _write(() async {
      await _pref('session', '');
      await _pref('apiBase', '');
      await _pref('driverId', '');
      // Login drafts hold the previous driver's email and the private API
      // hostname. On a pool device they outlive the session otherwise — and a
      // leftover draft also blocks `selectWorkspace` with a misleading error.
      await _db.customStatement("DELETE FROM preferences WHERE key LIKE 'draft:%'");
      await _pref('organization', 'Altius Demo');
      await _pref('hub', 'Jakarta');
    });
  }
  Future<void> language(String value) => _write(() async {
    if (!['en', 'id', 'th', 'ja', 'zh', 'fil', 'vi'].contains(value)) { throw ArgumentError('required'); }
    await _pref('language', value);
  });
  Future<bool> tripActive() async => await preference('trip') == 'active';
  Future<List<FieldTask>> tasks() async => (await _db.customSelect('SELECT * FROM tasks ORDER BY id').get()).map((r) => FieldTask(
    r.read<String>('id'), r.read<String>('title'), r.read<String>('address'),
    TaskStage.values[r.read<int>('stage')], r.read<int>('eta'),
    lat: r.read<double?>('lat'), lng: r.read<double?>('lng'),
  )).toList();
  Future<List<WorkEvent>> events() async => (await _db.customSelect('SELECT * FROM events ORDER BY rowid').get()).map(WorkEvent.new).toList();
  Future<List<CostEntry>> costs() async => (await _db.customSelect('SELECT * FROM costs ORDER BY rowid').get()).map(CostEntry.new).toList();
  Future<List<DailyReport>> reports() async => (await _db.customSelect('SELECT * FROM reports ORDER BY day DESC').get()).map(DailyReport.new).toList();

  Future<bool> _replayed(String? key, String kind, Map<String, Object?> input) async {
    if (key == null) { return false; }
    if (key.isEmpty || key.length > 160) { throw ArgumentError('required'); }
    final row = await _db.customSelect('SELECT * FROM events WHERE request_key = ?', variables: [Variable(key)]).getSingleOrNull();
    if (row == null) { return false; }
    final event = WorkEvent(row);
    if (event.kind != kind || jsonEncode(event.payload['input']) != jsonEncode(input)) { throw StateError('requestConflict'); }
    return true;
  }

  Future<void> _event(String entity, String kind, {Map<String, Object?> payload = const {}, String? requestId, Map<String, Object?> input = const {}}) async {
    final utc = _now().toUtc();
    final offset = _offset();
    if (offset < -840 || offset > 840) { throw StateError('clockChanged'); }
    final previous = await _db.customSelect('SELECT utc FROM events ORDER BY rowid DESC LIMIT 1').getSingleOrNull();
    if (previous != null && utc.isBefore(DateTime.parse(previous.read<String>('utc')))) { throw StateError('clockChanged'); }
    await _db.customStatement('INSERT INTO events (id,entity,kind,utc,offset_minutes,day,payload,request_key) VALUES (?,?,?,?,?,?,?,?)', [
      newRequestId(), entity, kind, utc.toIso8601String(), offset, _day(utc, offset),
      jsonEncode({...payload, 'input': input, 'organization': await preference('organization'), 'hub': await preference('hub'), 'source': 'demo-device-action', 'gps': null, 'gpsValidity': 'unavailable', 'gpsVerified': false}), requestId,
    ]);
  }

  Future<void> _ensureDraft() async {
    if ((await reports()).any((r) => r.day == today)) { throw StateError('reportLocked'); }
  }

  Future<void> startTrip() => _write(() async {
    await _ensureDraft();
    if (await tripActive()) { throw StateError('invalidSequence'); }
    await _pref('trip', 'active');
    await _event('trip', 'tripStarted');
  });

  Future<void> endTrip() => _write(() async {
    if (!await tripActive() || (await tasks()).any((t) => t.stage == TaskStage.arrived || t.stage == TaskStage.working)) { throw StateError('finishActivity'); }
    await _pref('trip', 'ended');
    await _event('trip', 'tripEnded');
  });

  Future<void> advance(String id, TaskStage next, {String? requestId}) => _write(() async {
    final input = <String, Object?>{'task': id, 'stage': next.name};
    if (await _replayed(requestId, next.name, input)) { return; }
    await _ensureDraft();
    if (!await tripActive()) { throw StateError('startTripFirst'); }
    final task = (await tasks()).where((t) => t.id == id).firstOrNull;
    if (task == null || next.index != task.stage.index + 1) { throw StateError('invalidSequence'); }
    if (next == TaskStage.arrived && (await tasks()).any((t) => t.id != id && (t.stage == TaskStage.arrived || t.stage == TaskStage.working))) { throw StateError('finishActivity'); }
    await _db.customStatement('UPDATE tasks SET stage = ? WHERE id = ?', [next.index, id]);
    await _event(id, next.name, requestId: requestId, input: input, payload: {'proofDraft': await draft('proof:$id')});
  });

  Future<void> createTask(String title, String address, {required String requestId}) => _write(() async {
    final input = <String, Object?>{'title': title.trim(), 'address': address.trim()};
    if (await _replayed(requestId, 'taskCreated', input)) { return; }
    await _ensureDraft();
    if (title.trim().isEmpty || address.trim().isEmpty || title.length > 100 || address.length > 240) { throw ArgumentError('required'); }
    final id = 'LOCAL-${newRequestId().substring(0, 8)}';
    await _db.customStatement('INSERT INTO tasks (id,title,address,eta) VALUES (?,?,?,?)', [id, title.trim(), address.trim(), 0]);
    await _event(id, 'taskCreated', requestId: requestId, input: input);
    await _pref('draft:task', '');
  });

  Future<void> addCost(String category, int amount, String note, {String? requestId}) => _write(() async {
    final input = <String, Object?>{'category': category, 'amount': amount, 'note': note.trim()};
    if (await _replayed(requestId, 'costAdded', input)) { return; }
    if (!['fuel', 'toll', 'parking', 'other'].contains(category) || amount <= 0 || amount > 100000000 || note.length > 240) { throw ArgumentError('invalidCost'); }
    await _ensureDraft();
    final id = newRequestId();
    await _db.customStatement('INSERT INTO costs VALUES (?,?,?,?,?)', [id, category, amount, note.trim(), today]);
    await _event(id, 'costAdded', payload: {...input, 'currency': 'IDR'}, requestId: requestId, input: input);
    await _pref('draft:cost', '');
  });

  Future<void> submitReport({
    String driverName = '',
    String vehicleNumber = '',
    int odometerStart = 0,
    int odometerEnd = 0,
    String notes = '',
  }) => _write(() async {
    await _ensureDraft();
    if (await tripActive()) { throw StateError('endTripFirst'); }
    // The report row is immutable once written, and the odometer delta is what
    // fleet distance and fuel reconciliation are billed on — so reject nonsense
    // here rather than storing it forever.
    if (odometerStart < 0 || odometerEnd < 0) { throw StateError('odometerNegative'); }
    if (odometerEnd < odometerStart) { throw StateError('odometerBackwards'); }
    if (odometerEnd - odometerStart > maxDailyKm) { throw StateError('odometerImplausible'); }
    final dailyEvents = (await events()).where((e) => e.day == today).toList();
    final completed = dailyEvents.where((e) => e.kind == 'done').map((e) => e.entity).toSet().length;
    final visited = dailyEvents.where((e) => e.kind == 'arrived').map((e) => e.entity).toSet().length;
    final total = (await costs()).where((e) => e.day == today).fold(0, (sum, e) => sum + e.amount);
    await _db.customStatement(
      'INSERT INTO reports VALUES (?,?,?,?,?,?,?,?,?)',
      [today, total, completed, visited, driverName.trim(), vehicleNumber.trim(), odometerStart, odometerEnd, notes.trim()],
    );
    await _event(
      today,
      'reportSubmitted',
      payload: {
        'total': total,
        'currency': 'IDR',
        'completed': completed,
        'visited': visited,
        'driverName': driverName.trim(),
        'vehicleNumber': vehicleNumber.trim(),
        'odometerStart': odometerStart,
        'odometerEnd': odometerEnd,
        'notes': notes.trim(),
        'eventIds': dailyEvents.map((e) => e.id).toList(),
        'odometerKm': odometerEnd - odometerStart,
        'routeSource': 'directions',
        'gpsDistanceMeters': null,
      },
    );
  });

  /// A day's driving above this is a typo, not a shift.
  static const maxDailyKm = 2000;

  /// Plain-text daily report for pasting into WhatsApp.
  ///
  /// The driver's own copy of the record: the operational need Phase 6 would
  /// otherwise meet with an SMS/WhatsApp gateway, a vendor account and a
  /// delivery-failure path. A clipboard button costs none of that.
  Future<String> reportText(String day) async {
    final r = (await reports()).where((e) => e.day == day).firstOrNull;
    final dayCosts = (await costs()).where((c) => c.day == day).toList();
    final org = await preference('organization');
    final hub = await preference('hub');
    final lines = <String>[
      'LAPORAN HARIAN SOPIR',
      '$org · $hub',
      'Tanggal: $day',
      if (r != null && r.driverName.isNotEmpty) 'Sopir: ${r.driverName}',
      if (r != null && r.vehicleNumber.isNotEmpty) 'Kendaraan: ${r.vehicleNumber}',
      '',
      'Kunjungan: ${r?.visited ?? 0}',
      'Selesai: ${r?.completed ?? 0}',
    ];
    if (r != null && r.odometerEnd > r.odometerStart) {
      lines.add('Odometer: ${r.odometerStart} → ${r.odometerEnd} (${r.odometerEnd - r.odometerStart} km)');
    }
    if (dayCosts.isNotEmpty) {
      lines..add('')..add('Biaya operasional:');
      for (final c in dayCosts) {
        lines.add('- ${c.category}: IDR ${c.amount}${c.note.isEmpty ? '' : ' (${c.note})'}');
      }
    }
    lines..add('')..add('Total: IDR ${r?.total ?? dayCosts.fold<int>(0, (a, c) => a + c.amount)}');
    if (r != null && r.notes.isNotEmpty) lines..add('')..add('Catatan: ${r.notes}');
    return lines.join('\n');
  }

  /// Same record as JSON, for pasting into a spreadsheet or ticket.
  Future<String> reportJson(String day) async {
    final r = (await reports()).where((e) => e.day == day).firstOrNull;
    final dayCosts = (await costs()).where((c) => c.day == day).toList();
    return const JsonEncoder.withIndent('  ').convert({
      'day': day,
      'organization': await preference('organization'),
      'hub': await preference('hub'),
      'driverName': r?.driverName ?? '',
      'vehicleNumber': r?.vehicleNumber ?? '',
      'odometerStart': r?.odometerStart ?? 0,
      'odometerEnd': r?.odometerEnd ?? 0,
      'visited': r?.visited ?? 0,
      'completed': r?.completed ?? 0,
      'currency': 'IDR',
      'total': r?.total ?? 0,
      'notes': r?.notes ?? '',
      'costs': [for (final c in dayCosts) {'category': c.category, 'amount': c.amount, 'note': c.note}],
    });
  }

  Future<void> retrySync({required bool offline}) => _write(() async {
    await _pref('lastSyncAttempt', _now().toUtc().toIso8601String());
    await _pref('syncError', offline ? 'offline' : 'providerMissing');
  });

  /// Maps a local event kind to the backend StopAction contract.
  /// Returns null for local-only events (trip lifecycle, drafts, reports).
  static String? _actionFor(String kind) => switch (kind) {
        'arrived' => 'arrive',
        'working' => 'start_activity',
        'done' => 'complete_activity',
        _ => null,
      };

  /// Push pending task-stage events to `POST {baseUrl}/api/v3/events` and mark
  /// each row by its receipt. Local-only events are stamped 'local' so they
  /// stop counting toward the outbox. Requires [accessToken] (Keycloak JWT
  /// with the `driver` role). Throws on transport/HTTP failure.
  Future<int> syncNow({required String baseUrl, required String? accessToken}) =>
      _write(() async {
        if (accessToken == null || accessToken.isEmpty) throw StateError('unauthorized');
        await _pref('lastSyncAttempt', _now().toUtc().toIso8601String());
        final pending = await _db
            .customSelect("SELECT * FROM events WHERE delivery = 'pending' ORDER BY rowid")
            .get();
        final events = pending.map(WorkEvent.new).toList();

        var sent = 0;
        final batch = <Map<String, Object?>>[];
        final syncable = <WorkEvent>[];
        for (final e in events) {
          final action = _actionFor(e.kind);
          if (action == null) {
            await _db.customStatement(
              "UPDATE events SET delivery = 'local' WHERE id = ?",
              [e.id],
            );
            continue;
          }
          batch.add({
            'event_id': e.id,
            'idempotency_key': e.requestKey ?? e.id,
            'tenant_id': await preference('organization'),
            'hub_id': await preference('hub'),
            'driver_id': await preference('driverId').then((v) => v.isEmpty ? 'demo-driver' : v),
            'device_id': 'demo-device',
            'task_id': e.entity,
            'stop_id': null,
            'action': action,
            'time': {
              'utc': e.utc.toUtc().toIso8601String(),
              'offset_minutes': e.offsetMinutes,
            },
            'payload': e.payload,
          });
          syncable.add(e);
          sent++;
        }

        if (batch.isEmpty) {
          await _pref('syncError', '');
          return 0;
        }

        try {
          final client = HttpClient();
          try {
            final req = await client.postUrl(Uri.parse('$baseUrl/api/v3/events'));
            req.headers.contentType = ContentType.json;
            req.headers.set('authorization', 'Bearer $accessToken');
            req.write(jsonEncode(batch));
            final res = await req.close().timeout(const Duration(seconds: 15));
            final body = await res.transform(utf8.decoder).join();
            if (res.statusCode != 200) {
              await _pref('syncError', 'http${res.statusCode}');
              throw HttpException('sync failed: ${res.statusCode}');
            }
            final decoded = jsonDecode(body) as Map<String, dynamic>;
            final receipts = (decoded['data'] as List?) ?? const [];
            // Match receipts by identity, never by array position. A reordered,
            // truncated or fabricated response would otherwise stamp one event's
            // delivery status onto another — and a row that leaves 'pending' is
            // never retried, so that is permanent loss of proof-of-service.
            final byId = {for (final e in syncable) e.id: e};
            final seen = <String>{};
            for (final entry in receipts) {
              if (entry is! Map<String, dynamic>) continue;
              final id = entry['event_id'] ?? entry['server_event_id'];
              if (id is! String || !byId.containsKey(id)) continue;
              // Only the statuses the contract defines are terminal. Anything
              // unrecognised ('retry', a missing key, a partial response) stays
              // pending so the event is resent, rather than being silently
              // dropped as 'rejected'.
              final raw = entry['status'];
              final status = raw == 'accepted'
                  ? 'accepted'
                  : raw == 'rejected'
                      ? 'rejected'
                      : null;
              if (status == null) continue;
              seen.add(id);
              await _db.customStatement(
                'UPDATE events SET delivery = ? WHERE id = ?',
                [status, id],
              );
            }
            // An event we sent but heard nothing about is still pending, not
            // delivered. Surface the shortfall instead of reporting a clean run.
            if (seen.length != syncable.length) {
              await _pref('syncError', 'partial');
            }
          } finally {
            client.close();
          }
          if (await preference('syncError') == 'partial') {
            // Keep the partial marker; do not report success.
          } else {
            await _pref('syncError', '');
          }
        } on Object {
          if (await preference('syncError') == '') {
            await _pref('syncError', 'transport');
          }
          rethrow;
        }
        return sent;
      });

  Future<void> selectWorkspace(String organization, String hub) => _write(() async {
    if (!['Altius Demo', 'Altius Training'].contains(organization) || !['Jakarta', 'Bandung', 'Surabaya'].contains(hub)) { throw ArgumentError('invalidWorkspace'); }
    if (organization == await preference('organization') && hub == await preference('hub')) { return; }
    if (await tripActive() || (await events()).any((e) => e.delivery == 'pending')) { throw StateError('unsyncedProtected'); }
    final drafts = await _db.customSelect("SELECT value FROM preferences WHERE key LIKE 'draft:%' AND value != ''").get();
    if (drafts.isNotEmpty) { throw StateError('unsyncedProtected'); }
    await _pref('organization', organization);
    await _pref('hub', hub);
  });

  Future<void> close() async { await _tail; await _db.close(); }
}
