import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:altius_field/core/data/work_store.dart';
import 'package:altius_field/features/work/work_cubit.dart';
import 'package:altius_field/app.dart';

void main() {
  late Directory directory;
  late WorkStore store;
  final clock = DateTime.parse('2026-09-07T02:00:00Z');

  setUp(() async {
    directory = await Directory.systemTemp.createTemp('altius-widget-');
    store = WorkStore.open('${directory.path}/w.sqlite', now: () => clock, offset: () => 420);
    await store.initialize();
  });
  tearDown(() async {
    await store.close();
    await directory.delete(recursive: true);
  });

  testWidgets('login screen shows demo notice and enters workspace', (tester) async {
    final cubit = WorkCubit(store);
    await cubit.refresh();
    await tester.pumpWidget(AltiusApp(cubit: cubit, environment: 'test'));
    for (var i = 0; i < 10; i++) { await tester.pump(const Duration(milliseconds: 100)); }
    expect(find.text('Altius Field'), findsOneWidget);
    expect(find.text('Masuk ruang kerja demo', skipOffstage: false), findsOneWidget);
  });

  testWidgets('task list and trip controls render after session', (tester) async {
    await tester.runAsync(() async {
      await store.session(true);
      final cubit = WorkCubit(store);
      await cubit.refresh();
      await tester.pumpWidget(AltiusApp(cubit: cubit, environment: 'test'));
      for (var i = 0; i < 10; i++) { await tester.pump(const Duration(milliseconds: 100)); }
      expect(find.text('Nusantara Market'), findsOneWidget);
      expect(find.text('Mulai perjalanan'), findsOneWidget);
    });
  });
}
