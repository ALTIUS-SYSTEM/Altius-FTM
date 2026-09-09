import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:altius_field/core/data/work_store.dart';
import 'package:sqlite3/sqlite3.dart' as sqlite3;

void main() {
  late Directory directory;
  late WorkStore store;
  final clock = DateTime.parse('2026-09-07T02:00:00Z');

  setUp(() async {
    directory = await Directory.systemTemp.createTemp('altius-test-');
    store = WorkStore.open('${directory.path}/work.sqlite', now: () => clock, offset: () => 420);
    await store.initialize();
  });
  tearDown(() async {
    await store.close();
    await directory.delete(recursive: true);
  });

  test('server URL must be https with a host (http only on loopback)', () {
    // A plain http:// base to a remote host sends bearer tokens in the clear.
    for (final bad in [
      'http://api.altius.test',
      'api.altius.test',
      'https://',
      'ftp://api.altius.test',
      'javascript:alert(1)',
      '',
    ]) {
      expect(() => parseServerUrl(bad), throwsArgumentError, reason: bad);
    }
    expect(parseServerUrl('  https://api.altius.test  ').host, 'api.altius.test');
    // Debug builds reach a dev server through `adb reverse`, which is a local
    // socket. Release builds get no exception — `kDebugMode` is false there.
    expect(parseServerUrl('http://localhost:8080').host, 'localhost');
    expect(() => parseServerUrl('http://api.altius.test'), throwsArgumentError);
    expect(parseServerUrl('http://127.0.0.1:8080').host, '127.0.0.1');
    expect(parseServerUrl('http://localhost:8080').scheme, 'http');
  });

  test('API task envelope parser reads Postgres and TypeDB shapes', () {
    final pg = store.debugParseTasks([
      {
        'task': {
          'id': 'T-1',
          'title': 'Depot drop',
          'hub_id': 'jakarta',
          'stage': 'assigned',
          'day': '2026-09-07',
        },
        'stops': [
          {
            'id': 'S-1',
            'address': 'Jl. Sudirman',
            'lat': -6.2,
            'lng': 106.8,
            'stage': 'pending',
            'sequence': 0,
          },
        ],
      },
    ]);
    expect(pg.single[0], 'T-1');
    expect(pg.single[2], 'Jl. Sudirman');
    expect(pg.single[5], -6.2);
    expect(pg.single[6], 106.8);
    expect(pg.single[7], 'S-1');

    final typedb = store.debugParseTasks([
      {
        'task': {
          'task-id': {'value': 'T-2'},
          'title': {'value': 'Pickup'},
          'hub-id': {'value': 'bandung'},
        },
        'stops': [
          {
            'stop': {
              'stop-id': {'value': 'S-2'},
              'address': {'value': 'Jl. Asia Afrika'},
              'latitude': {'value': '-6.9'},
              'longitude': {'value': '107.6'},
              'stage': {'value': 'arrived'},
            },
          },
        ],
      },
    ]);
    expect(typedb.single[0], 'T-2');
    expect(typedb.single[3], TaskStage.arrived.index);
    expect(typedb.single[7], 'S-2');
  });

  test('migration from v6 adds stop_id on tasks', () async {
    await store.close();
    await _seedSchema(directory.path, 'work.sqlite', 6);
    final reopened = WorkStore.open(
      '${directory.path}/work.sqlite',
      now: () => clock,
      offset: () => 420,
      demoWorkspace: false,
    );
    await reopened.initialize();
    expect((await reopened.tasks()).single.stopId, isNull);
    await reopened.close();
  });

  test('odometer readings are validated before the report becomes immutable', () async {
    // The report row cannot be updated or deleted once written, and the
    // odometer delta is what distance and fuel are reconciled against.
    await expectLater(store.submitReport(odometerStart: 100, odometerEnd: 40), throwsStateError);
    await expectLater(store.submitReport(odometerStart: -1, odometerEnd: 10), throwsStateError);
    await expectLater(store.submitReport(odometerStart: 0, odometerEnd: 99999), throwsStateError);
    expect(await store.reports(), isEmpty, reason: 'no bad row may be persisted');

    await store.submitReport(driverName: 'Adi', vehicleNumber: 'B 1234 XYZ', odometerStart: 100, odometerEnd: 340);
    final saved = await store.reports();
    expect(saved.single.odometerEnd - saved.single.odometerStart, 240);
  });

  test('report text and json carry the day\'s costs and odometer', () async {
    await store.addCost('fuel', 150000, 'Pertamina Kuningan', requestId: store.newRequestId());
    await store.submitReport(driverName: 'Adi', vehicleNumber: 'B 1234 XYZ', odometerStart: 100, odometerEnd: 340);

    final text = await store.reportText(store.today);
    expect(text, contains('B 1234 XYZ'));
    expect(text, contains('100 → 340 (240 km)'));
    expect(text, contains('Pertamina Kuningan'));
    expect(text, contains('Total: IDR 150000'));

    final json = jsonDecode(await store.reportJson(store.today)) as Map<String, dynamic>;
    expect(json['odometerEnd'], 340);
    expect(json['total'], 150000);
    expect((json['costs'] as List).single['amount'], 150000);
  });

  test('arrival, activity, completion require trip and exact sequence', () async {
    await expectLater(store.advance('JKT-001', TaskStage.arrived), throwsStateError);
    await store.startTrip();
    await expectLater(store.advance('JKT-001', TaskStage.working), throwsStateError);
    await store.advance('JKT-001', TaskStage.arrived);
    await expectLater(store.endTrip(), throwsStateError);
    await store.advance('JKT-001', TaskStage.working);
    await store.advance('JKT-001', TaskStage.done);
    await store.endTrip();
    expect((await store.tasks()).first.stage, TaskStage.done);
    final events = await store.events();
    expect(events.length, 5);
    expect(events.map((e) => e.id).toSet().length, 5);
    expect(events.every((e) => e.utc.isUtc && e.offsetMinutes == 420), isTrue);
    expect(events.every((e) => e.delivery == 'pending'), isTrue);
  });

  test('concurrent duplicate taps write only one event', () async {
    await store.startTrip();
    final results = await Future.wait(List.generate(2, (_) async {
      try { await store.advance('JKT-001', TaskStage.arrived); return true; }
      on StateError { return false; }
    }));
    expect(results.where((e) => e).length, 1);
    expect((await store.events()).length, 2);
  });

  test('committed work and outbox survive reopening offline', () async {
    await store.startTrip();
    await store.advance('JKT-001', TaskStage.arrived);
    await store.addCost('fuel', 125000, 'Receipt 12');
    await store.close();
    store = WorkStore.open('${directory.path}/work.sqlite', now: () => clock, offset: () => 420);
    await store.initialize();
    expect((await store.tasks()).first.stage, TaskStage.arrived);
    expect(await store.tripActive(), isTrue);
    expect((await store.costs()).single.amount, 125000);
    expect((await store.events()).length, 3);
    await expectLater(store.selectWorkspace('Altius Demo', 'Bandung'), throwsStateError);
  });

  test('daily submission is immutable, unique and retained pending', () async {
    await expectLater(store.addCost('fuel', -1, ''), throwsArgumentError);
    await store.addCost('toll', 25000, 'Gate 1');
    await store.submitReport();
    await expectLater(store.submitReport(), throwsStateError);
    await expectLater(store.addCost('fuel', 5000, ''), throwsStateError);
    final report = (await store.reports()).single;
    expect(report.total, 25000);
    expect(report.day, '2026-09-07');
    expect((await store.events()).last.kind, 'reportSubmitted');
  });

  test('submission blocks active trip and incomplete visited tasks', () async {
    await store.startTrip();
    await expectLater(store.submitReport(), throwsStateError);
    await store.endTrip();
    await store.submitReport();
    expect((await store.reports()).length, 1);
  });

  test('migration from v3 adds task coordinates, report vehicle fields and vehicle_checks', () async {
    await store.close();
    await _seedSchema(directory.path, 'work.sqlite', 3);
    final reopened = WorkStore.open(
      '${directory.path}/work.sqlite',
      now: () => clock,
      offset: () => 420,
      demoWorkspace: false,
    );
    await reopened.initialize();
    final task = await reopened.tasks();
    expect(task.single, isA<FieldTask>());
    // lat/lng added by _upgrade4.
    expect(task.single.lat, isNull);
    expect(task.single.lng, isNull);
    final report = await reopened.reports();
    expect(report.single, isA<DailyReport>());
    expect(report.single.driverName, '');
    expect(report.single.vehicleNumber, '');
    expect(report.single.odometerStart, 0);
    expect(report.single.odometerEnd, 0);
    final checks = await reopened.vehicleChecks();
    expect(checks, isEmpty);
  });

  test('migration from v4 adds report vehicle fields and vehicle_checks', () async {
    await store.close();
    await _seedSchema(directory.path, 'work.sqlite', 4);
    final reopened = WorkStore.open(
      '${directory.path}/work.sqlite',
      now: () => clock,
      offset: () => 420,
      demoWorkspace: false,
    );
    await reopened.initialize();
    expect((await reopened.tasks()).single, isA<FieldTask>());
    expect((await reopened.reports()).single.odometerStart, 0);
    expect((await reopened.vehicleChecks()).length, 0);
  });

  test('migration from v5 creates vehicle_checks', () async {
    await store.close();
    await _seedSchema(directory.path, 'work.sqlite', 5);
    final reopened = WorkStore.open(
      '${directory.path}/work.sqlite',
      now: () => clock,
      offset: () => 420,
      demoWorkspace: false,
    );
    await reopened.initialize();
    expect((await reopened.reports()).single, isA<DailyReport>());
    expect((await reopened.vehicleChecks()).length, 0);
  });

  test('v3 data is preserved through migration to current schema', () async {
    await store.close();
    await _seedSchema(directory.path, 'work.sqlite', 3, seedTrip: true);
    final reopened = WorkStore.open(
      '${directory.path}/work.sqlite',
      now: () => clock,
      offset: () => 420,
      demoWorkspace: false,
    );
    await reopened.initialize();
    expect((await reopened.tasks()).length, 1);
    expect((await reopened.tasks()).single.stage, TaskStage.arrived);
    expect((await reopened.events()).length, greaterThanOrEqualTo(2));
    expect((await reopened.tripActive()), isTrue);
  });

  test('schematic ETA matches 30 km/h straight-line fallback', () {
    // Same point → 0 minutes.
    expect(WorkStore.schematicEtaMinutes((lat: -6.2, lng: 106.8), (lat: -6.2, lng: 106.8)), 0);
    // Blok M → Kuningan ≈ 5 km straight line → ~10 minutes at 30 km/h.
    final eta = WorkStore.schematicEtaMinutes((lat: -6.2446, lng: 106.8067), (lat: -6.1944, lng: 106.8289));
    expect(eta, greaterThan(5));
    expect(eta, lessThan(20));
    // Antipodean distance should be large but finite.
    final far = WorkStore.schematicEtaMinutes((lat: 0, lng: 0), (lat: 0, lng: 180));
    expect(far, greaterThan(10000));
  });

  test('geofence radius is 50m, matching GPS accuracy threshold', () {
    expect(WorkStore.geofenceRadiusMeters, 50.0);
  });
}

/// Create a legacy SQLite file at the given version and set `user_version`.
///
/// These schemas reproduce the tables as they existed at that version,
/// including the triggers added by earlier upgrades, so the current
/// `onUpgrade` can apply the remaining deltas.
Future<void> _seedSchema(String dir, String file, int version, {bool seedTrip = false, bool seedData = true}) async {
  final path = '$dir/$file';
  if (File(path).existsSync()) { File(path).deleteSync(); }
  final db = sqlite3.sqlite3.open(path);
  db.execute('''
    CREATE TABLE tasks (
      id TEXT PRIMARY KEY,
      title TEXT NOT NULL,
      address TEXT NOT NULL,
      stage INTEGER NOT NULL DEFAULT 0 CHECK(stage BETWEEN 0 AND 3),
      eta INTEGER NOT NULL
      ${version >= 4 ? ', lat REAL, lng REAL' : ''}
    );
  ''');
  db.execute('CREATE TABLE preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL)');
  db.execute('''
    CREATE TABLE events (
      id TEXT PRIMARY KEY,
      entity TEXT NOT NULL,
      kind TEXT NOT NULL,
      utc TEXT NOT NULL,
      offset_minutes INTEGER NOT NULL,
      day TEXT NOT NULL,
      delivery TEXT NOT NULL DEFAULT 'pending',
      payload TEXT NOT NULL,
      request_key TEXT
    );
  ''');
  db.execute('CREATE UNIQUE INDEX IF NOT EXISTS event_request ON events(request_key)');
  db.execute('''
    CREATE TRIGGER events_no_update BEFORE UPDATE ON events
    WHEN NOT (
      OLD.id IS NEW.id AND OLD.entity IS NEW.entity AND OLD.kind IS NEW.kind
      AND OLD.utc IS NEW.utc AND OLD.offset_minutes IS NEW.offset_minutes
      AND OLD.day IS NEW.day AND OLD.payload IS NEW.payload
      AND OLD.request_key IS NEW.request_key
    ) BEGIN SELECT RAISE(ABORT, 'immutable event'); END;
  ''');
  db.execute('CREATE TRIGGER events_no_delete BEFORE DELETE ON events BEGIN SELECT RAISE(ABORT, \'immutable event\'); END');
  db.execute('''
    CREATE TABLE costs (
      id TEXT PRIMARY KEY,
      category TEXT NOT NULL,
      amount INTEGER NOT NULL CHECK(amount > 0),
      note TEXT NOT NULL,
      day TEXT NOT NULL
    );
  ''');
  db.execute('''
    CREATE TABLE reports (
      day TEXT PRIMARY KEY,
      total INTEGER NOT NULL,
      completed INTEGER NOT NULL,
      visited INTEGER NOT NULL
      ${version >= 5 ? ', driver_name TEXT NOT NULL DEFAULT ""' : ''}
      ${version >= 5 ? ', vehicle_number TEXT NOT NULL DEFAULT ""' : ''}
      ${version >= 5 ? ', odometer_start INTEGER NOT NULL DEFAULT 0' : ''}
      ${version >= 5 ? ', odometer_end INTEGER NOT NULL DEFAULT 0' : ''}
      ${version >= 5 ? ', notes TEXT NOT NULL DEFAULT ""' : ''}
    );
  ''');
  db.execute('CREATE TRIGGER reports_no_update BEFORE UPDATE ON reports BEGIN SELECT RAISE(ABORT, \'immutable report\'); END');
  db.execute('CREATE TRIGGER reports_no_delete BEFORE DELETE ON reports BEGIN SELECT RAISE(ABORT, \'immutable report\'); END');
  if (version >= 6) {
    db.execute('''
      CREATE TABLE vehicle_checks (
        id TEXT PRIMARY KEY,
        day TEXT NOT NULL,
        driver_name TEXT NOT NULL DEFAULT "",
        license_plate TEXT NOT NULL DEFAULT "",
        vehicle_type TEXT NOT NULL DEFAULT "",
        km_start INTEGER NOT NULL DEFAULT 0,
        km_end INTEGER NOT NULL DEFAULT 0,
        condition TEXT NOT NULL DEFAULT "good",
        items TEXT NOT NULL DEFAULT "[]",
        notes TEXT NOT NULL DEFAULT "",
        service_date TEXT NOT NULL DEFAULT "",
        kir_date TEXT NOT NULL DEFAULT "",
        stnk_date TEXT NOT NULL DEFAULT "",
        delivery TEXT NOT NULL DEFAULT "local"
      );
    ''');
    db.execute('CREATE TRIGGER vehicle_checks_no_update BEFORE UPDATE ON vehicle_checks BEGIN SELECT RAISE(ABORT, \'immutable vehicle check\'); END');
    db.execute('CREATE TRIGGER vehicle_checks_no_delete BEFORE DELETE ON vehicle_checks BEGIN SELECT RAISE(ABORT, \'immutable vehicle check\'); END');
  }
  if (seedData) {
    if (version >= 4) {
      db.execute("INSERT INTO tasks VALUES ('JKT-001', 'Nusantara Market', 'Jl. Sudirman', 0, 18, NULL, NULL)");
    } else {
      db.execute("INSERT INTO tasks VALUES ('JKT-001', 'Nusantara Market', 'Jl. Sudirman', 0, 18)");
    }
    if (version >= 5) {
      db.execute("INSERT INTO reports VALUES ('2026-09-07', 0, 0, 0, '', '', 0, 0, '')");
    } else {
      db.execute("INSERT INTO reports VALUES ('2026-09-07', 0, 0, 0)");
    }
  }
  if (seedTrip) {
    db.execute("INSERT OR REPLACE INTO preferences VALUES ('organization', 'Altius Demo'), ('hub', 'Jakarta'), ('language', 'id'), ('trip', 'active')");
    db.execute("UPDATE tasks SET stage = 1 WHERE id = 'JKT-001'");
    db.execute("INSERT INTO events VALUES ('ev-1','JKT-001','tripStarted','2026-09-07T02:00:00.000Z',420,'2026-09-07','pending','{}',NULL)");
    db.execute("INSERT INTO events VALUES ('ev-2','JKT-001','arrived','2026-09-07T02:05:00.000Z',420,'2026-09-07','pending','{}',NULL)");
  }
  db.execute('PRAGMA user_version = $version');
  db.dispose();
}
