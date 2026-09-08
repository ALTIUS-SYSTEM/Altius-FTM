import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'package:drift/drift.dart';
import 'package:drift/native.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

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
  const FieldTask(this.id, this.title, this.address, this.stage, this.etaMinutes);
  final String id;
  final String title;
  final String address;
  final TaskStage stage;
  final int etaMinutes;
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
      visited = row.read<int>('visited');
  final String day;
  final int total;
  final int completed;
  final int visited;
}

class _Database extends GeneratedDatabase {
  _Database(super.executor);
  @override
  int get schemaVersion => 3;
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
  @override
  MigrationStrategy get migration => MigrationStrategy(onCreate: (m) async {
    await customStatement('CREATE TABLE tasks (id TEXT PRIMARY KEY, title TEXT NOT NULL, address TEXT NOT NULL, stage INTEGER NOT NULL DEFAULT 0 CHECK(stage BETWEEN 0 AND 3), eta INTEGER NOT NULL)');
    await customStatement('CREATE TABLE preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL)');
    await customStatement("CREATE TABLE events (id TEXT PRIMARY KEY, entity TEXT NOT NULL, kind TEXT NOT NULL, utc TEXT NOT NULL, offset_minutes INTEGER NOT NULL, day TEXT NOT NULL, delivery TEXT NOT NULL DEFAULT 'pending', payload TEXT NOT NULL)");
    await customStatement('CREATE TABLE costs (id TEXT PRIMARY KEY, category TEXT NOT NULL, amount INTEGER NOT NULL CHECK(amount > 0), note TEXT NOT NULL, day TEXT NOT NULL)');
    await customStatement('CREATE TABLE reports (day TEXT PRIMARY KEY, total INTEGER NOT NULL, completed INTEGER NOT NULL, visited INTEGER NOT NULL)');
    await _upgrade();
    await _upgrade3();
  }, onUpgrade: (m, from, to) async {
    if (from < 2) { await _upgrade(); }
    if (from < 3) { await _upgrade3(); }
  });
}

class WorkStore {
  WorkStore.open(String path, {DateTime Function()? now, int Function()? offset, TokenStore? tokens})
    : _db = _Database(NativeDatabase(File(path), setup: (db) {
        db.execute('PRAGMA journal_mode=WAL');
        db.execute('PRAGMA synchronous=FULL');
      })),
      _now = now ?? DateTime.now,
      _offset = offset ?? (() => DateTime.now().timeZoneOffset.inMinutes),
      _tokens = tokens ?? const SecureTokenStore();
  final _Database _db;
  final TokenStore _tokens;
  final DateTime Function() _now;
  final int Function() _offset;
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
      for (final task in [
        ['JKT-001', 'Nusantara Market', 'Jl. Sudirman 24, Jakarta Selatan', 18],
        ['JKT-002', 'Cendana Distribution', 'Jl. Gatot Subroto 18, Jakarta Selatan', 32],
        ['JKT-003', 'Meridian Fresh', 'Jl. Rasuna Said 8, Jakarta Selatan', 47],
        ['JKT-004', 'Taman Sari Store', 'Jl. Kuningan 12, Jakarta Selatan', 61],
      ]) {
        await _db.customStatement('INSERT OR IGNORE INTO tasks (id,title,address,eta) VALUES (?,?,?,?)', task);
      }
    });
  }

  Future<String> preference(String key) async => (await _db.customSelect('SELECT value FROM preferences WHERE key = ?', variables: [Variable(key)]).getSingleOrNull())?.read<String>('value') ?? '';
  Future<void> _pref(String key, String value) => _db.customStatement('INSERT OR REPLACE INTO preferences VALUES (?,?)', [key, value]);
  Future<void> saveDraft(String key, String value) => _write(() => _pref('draft:$key', value));
  Future<String> draft(String key) => preference('draft:$key');
  Future<void> session(bool active) => _write(() => _pref('session', active ? 'true' : ''));

  /// Bearer token for outbound calls. Never stored in the SQLite preferences.
  Future<String> accessToken() => _tokens.read('accessToken');

  Future<void> login(String baseUrl, String username, String password) async {
    if (username.isEmpty || password.isEmpty) throw ArgumentError('required');
    final url = parseServerUrl(baseUrl).toString().replaceAll(RegExp(r'/+$'), '');
    final client = HttpClient();
    String? accessToken;
    String? refreshToken;
    int? expiresIn;
    String? organization;
    String? hub;
    try {
      final req = await client.postUrl(Uri.parse('$url/api/v3/auth/login'));
      req.headers.contentType = ContentType.json;
      req.write(jsonEncode({'username': username, 'password': password}));
      final res = await req.close().timeout(const Duration(seconds: 15));
      final body = await res.transform(utf8.decoder).join();
      if (res.statusCode == 401 || res.statusCode == 403) throw StateError('unauthorized');
      if (res.statusCode != 200) throw StateError('serverError');
      final data = jsonDecode(body) as Map<String, dynamic>;
      final tokenData = data['data'] as Map<String, dynamic>?;
      accessToken = tokenData?['accessToken'] as String?;
      refreshToken = tokenData?['refreshToken'] as String?;
      expiresIn = (tokenData?['expiresIn'] as int?) ?? (tokenData?['expiresIn'] as num?)?.toInt();
      if (accessToken == null || accessToken.isEmpty) throw StateError('serverError');

      final tReq = await client.getUrl(Uri.parse('$url/api/v3/tasks'));
      tReq.headers.set('authorization', 'Bearer $accessToken');
      final tRes = await tReq.close().timeout(const Duration(seconds: 15));
      final tBody = await tRes.transform(utf8.decoder).join();
      if (tRes.statusCode == 200) {
        final tData = jsonDecode(tBody) as Map<String, dynamic>;
        organization = (tData['meta'] as Map<String, dynamic>?)?['organization'] as String?;
        final tasks = (tData['data'] as List?)?.cast<Map<String, dynamic>>() ?? [];
        if (tasks.isNotEmpty) hub = tasks.first['hub_id'] as String?;
      }
    } finally {
      client.close();
    }
    await _tokens.write('accessToken', accessToken);
    await _tokens.write('refreshToken', refreshToken ?? '');
    if (expiresIn != null) {
      await _tokens.write(
        'accessExpiresAt',
        _now().toUtc().add(Duration(seconds: expiresIn)).toIso8601String(),
      );
    }
    await _write(() async {
      await _pref('apiBase', url);
      await _pref('driverId', username);
      if (organization != null && organization.isNotEmpty) await _pref('organization', organization);
      if (hub != null && hub.isNotEmpty) await _pref('hub', hub);
      await _pref('session', 'true');
    });
  }

  Future<void> logout() async {
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
  Future<List<FieldTask>> tasks() async => (await _db.customSelect('SELECT * FROM tasks ORDER BY id').get()).map((r) => FieldTask(r.read<String>('id'), r.read<String>('title'), r.read<String>('address'), TaskStage.values[r.read<int>('stage')], r.read<int>('eta'))).toList();
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

  Future<void> submitReport() => _write(() async {
    await _ensureDraft();
    if (await tripActive()) { throw StateError('endTripFirst'); }
    final dailyEvents = (await events()).where((e) => e.day == today).toList();
    final completed = dailyEvents.where((e) => e.kind == 'done').map((e) => e.entity).toSet().length;
    final visited = dailyEvents.where((e) => e.kind == 'arrived').map((e) => e.entity).toSet().length;
    final total = (await costs()).where((e) => e.day == today).fold(0, (sum, e) => sum + e.amount);
    await _db.customStatement('INSERT INTO reports VALUES (?,?,?,?)', [today, total, completed, visited]);
    await _event(today, 'reportSubmitted', payload: {'total': total, 'currency': 'IDR', 'completed': completed, 'visited': visited, 'eventIds': dailyEvents.map((e) => e.id).toList(), 'routeSource': 'demo-graph', 'gpsDistanceMeters': null});
  });

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
  Future<int> syncNow({required String baseUrl, required String accessToken}) =>
      _write(() async {
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
            for (var i = 0; i < receipts.length && i < syncable.length; i++) {
              final r = receipts[i] as Map<String, dynamic>;
              final status = r['status'] == 'accepted' ? 'accepted' : 'rejected';
              await _db.customStatement(
                'UPDATE events SET delivery = ? WHERE id = ?',
                [status, syncable[i].id],
              );
            }
          } finally {
            client.close();
          }
          await _pref('syncError', '');
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
