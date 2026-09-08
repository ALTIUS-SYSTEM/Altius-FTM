import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:altius_field/core/data/work_store.dart';

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

  test('server URL must be https with a host', () {
    // A plain http:// base sends the driver's password, and every later bearer
    // token, in the clear — so this guard is the whole defence.
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
}
