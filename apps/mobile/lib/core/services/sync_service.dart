import 'dart:async';

import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:path_provider/path_provider.dart';
import 'package:workmanager/workmanager.dart';

import '../data/work_store.dart';
import 'auth_service.dart';

const String _backgroundSyncTask = 'com.altius.altius_field.backgroundSync';

/// Callback dispatcher for WorkManager background sync.
@pragma('vm:entry-point')
void _callbackDispatcher() {
  Workmanager().executeTask((task, inputData) async {
    if (task == _backgroundSyncTask) {
      try {
        final dir = await getApplicationSupportDirectory();
        final store = WorkStore.open('${dir.path}/altius_field.sqlite', demoWorkspace: false);
        await store.initialize();
        final apiBase = await AuthService().apiBase();
        final token = await AuthService().accessToken();
        if (apiBase != null && token != null) {
          await store.syncNow(baseUrl: apiBase, accessToken: token);
        }
        return true;
      } catch (_) {
        return false;
      }
    }
    return true;
  });
}

/// Coordinates automatic and on-demand synchronization of the outbox.
class SyncService {
  final WorkStore _store;
  final AuthService _auth;
  final Connectivity _connectivity;
  StreamSubscription<List<ConnectivityResult>>? _subscription;

  SyncService({required WorkStore store, AuthService? auth, Connectivity? connectivity})
      : _store = store, // ignore: prefer_initializing_formals
        _auth = auth ?? AuthService(),
        _connectivity = connectivity ?? Connectivity();

  /// Initialize background sync task and foreground connectivity listener.
  Future<void> start() async {
    await Workmanager().initialize(_callbackDispatcher);
    await _scheduleBackground();
    _subscription = _connectivity.onConnectivityChanged.listen(_onConnectivityChanged);
  }

  Future<void> _scheduleBackground() async {
    await Workmanager().registerPeriodicTask(
      _backgroundSyncTask,
      _backgroundSyncTask,
      frequency: const Duration(minutes: 15),
      constraints: Constraints(networkType: NetworkType.connected),
      existingWorkPolicy: ExistingPeriodicWorkPolicy.replace,
    );
  }

  Future<void> _onConnectivityChanged(List<ConnectivityResult> results) async {
    if (results.any((r) => r != ConnectivityResult.none)) {
      await syncIfConfigured();
    }
  }

  /// Flush the outbox once if the driver is authenticated.
  Future<int> syncIfConfigured() async {
    final apiBase = await _auth.apiBase();
    final token = await _auth.accessToken();
    if (apiBase == null || token == null) return 0;
    return _store.syncNow(baseUrl: apiBase, accessToken: token);
  }

  /// Cancel background work and stop the connectivity listener.
  Future<void> stop() async {
    await _subscription?.cancel();
    await Workmanager().cancelByUniqueName(_backgroundSyncTask);
  }
}
