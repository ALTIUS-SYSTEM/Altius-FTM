import 'package:flutter_bloc/flutter_bloc.dart';
import '../../core/data/work_store.dart';

class WorkState {
  const WorkState({this.tasks = const [], this.events = const [], this.costs = const [], this.reports = const [], this.activeTrip = false, this.session = false, this.busy = false, this.language = 'id', this.organization = 'Altius Demo', this.hub = 'Jakarta', this.error});
  final List<FieldTask> tasks;
  final List<WorkEvent> events;
  final List<CostEntry> costs;
  final List<DailyReport> reports;
  final bool activeTrip;
  final bool session;
  final bool busy;
  final String language;
  final String organization;
  final String hub;
  final String? error;
  WorkState processing() => WorkState(tasks: tasks, events: events, costs: costs, reports: reports, activeTrip: activeTrip, session: session, busy: true, language: language, organization: organization, hub: hub);
}

class WorkCubit extends Cubit<WorkState> {
  WorkCubit(this.store) : super(const WorkState());
  final WorkStore store;
  Future<void> refresh({String? error}) async {
    final sessionPref = await store.preference('session');
    final session = sessionPref == 'true' || sessionPref == 'demo';
    emit(WorkState(tasks: await store.tasks(), events: await store.events(), costs: await store.costs(), reports: await store.reports(), activeTrip: await store.tripActive(), session: session, language: await store.preference('language'), organization: await store.preference('organization'), hub: await store.preference('hub'), error: error));
  }
  Future<bool> act(Future<void> Function() action) async {
    if (state.busy) { return false; }
    emit(state.processing());
    try {
      await action();
      await refresh();
      return true;
    } catch (error) {
      final key = error is StateError ? error.message : error is ArgumentError ? error.message?.toString() : 'storageError';
      await refresh(error: key ?? 'storageError');
      return false;
    }
  }
}
