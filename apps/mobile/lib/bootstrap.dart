import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'core/data/work_store.dart';
import 'core/theme/app_theme.dart';
import 'features/work/work_cubit.dart';
import 'app.dart';

Future<void> bootstrap({String environment = 'dev'}) async {
  WidgetsFlutterBinding.ensureInitialized();
  try {
    final directory = await getApplicationSupportDirectory();
    final store = WorkStore.open('${directory.path}/altius_$environment.sqlite');
    await store.initialize();
    final cubit = WorkCubit(store);
    await cubit.refresh();
    runApp(AltiusApp(cubit: cubit, environment: environment));
  } catch (_) {
    runApp(MaterialApp(theme: AppTheme.light, home: Scaffold(body: SafeArea(child: Padding(padding: const EdgeInsets.all(32), child: Column(mainAxisAlignment: MainAxisAlignment.center, crossAxisAlignment: CrossAxisAlignment.start, children: [const Icon(Icons.storage_rounded, size: 56), const SizedBox(height: 24), const Text('Altius · DEMO', style: TextStyle(fontSize: 28, fontWeight: FontWeight.bold)), const SizedBox(height: 16), const Text('[EN] Local storage could not open. No data was reset. Restart the app or free device storage. Do not uninstall if you have pending work.'), const SizedBox(height: 24), FilledButton(onPressed: () => bootstrap(environment: environment), child: const Text('Retry / Coba lagi'))]))))));
  }
}
