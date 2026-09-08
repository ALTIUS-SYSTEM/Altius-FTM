import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'core/data/work_store.dart';
import 'core/l10n/strings.dart';
import 'core/services/route_demo.dart';
import 'core/theme/app_theme.dart';
import 'features/work/work_cubit.dart';

/// Altius Field demo driver app. All data stays on-device in SQLite;
/// no authentication, sync transport, GPS sampling or map provider is configured.
class AltiusApp extends StatelessWidget {
  const AltiusApp({super.key, required this.cubit, required this.environment});
  final WorkCubit cubit;
  final String environment;
  @override
  Widget build(BuildContext context) {
    return BlocProvider.value(
      value: cubit,
      child: BlocBuilder<WorkCubit, WorkState>(builder: (context, state) {
        return MaterialApp(
          title: 'Altius Field · Demo',
          theme: AppTheme.light,
          supportedLocales: Strings.locales,
          locale: Strings.locales[Strings.codes.indexOf(state.language).clamp(0, 6)],
          localizationsDelegates: const [GlobalMaterialLocalizations.delegate, GlobalWidgetsLocalizations.delegate, GlobalCupertinoLocalizations.delegate],
          home: state.session ? const HomeShell() : const LoginScreen(),
          debugShowCheckedModeBanner: false,
          onGenerateTitle: (_) => 'Altius Field · ${environment.toUpperCase()}',
        );
      }),
    );
  }
}

void _errorSnack(BuildContext context, WorkState state) {
  final error = state.error;
  if (error == null) return;
  final s = Strings(state.language);
  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(s(error))));
}

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});
  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _server = TextEditingController();
  final _email = TextEditingController();
  final _password = TextEditingController();
  bool _busy = false;
  @override
  Widget build(BuildContext context) {
    final cubit = context.read<WorkCubit>();
    final s = Strings(cubit.state.language);
    return Scaffold(
      body: SafeArea(child: ListView(padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 32), children: [
        const SizedBox(height: 24),
        SvgPicture.asset('assets/altius-logo.svg', width: 64, height: 64),
        const SizedBox(height: 20),
        Text('Altius Field', style: Theme.of(context).textTheme.headlineLarge),
        Text(s('driverApp'), style: Theme.of(context).textTheme.titleMedium?.copyWith(color: AppTheme.teal)),
        const SizedBox(height: 12),
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(color: const Color(0xFFFDF1DF), borderRadius: BorderRadius.circular(14)),
          child: Text('${s('demo')}\n${s('demoNotice')}', style: const TextStyle(fontSize: 12, color: Color(0xFF573300), height: 1.5)),
        ),
        const SizedBox(height: 28),
        TextField(controller: _server, keyboardType: TextInputType.url, decoration: const InputDecoration(labelText: 'Server URL', hintText: 'https://api.altius.example'), onChanged: (_) => cubit.store.saveDraft('server', _server.text)),
        const SizedBox(height: 6),
        Text('Kosongkan Server URL untuk masuk mode demo.', style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Colors.grey, fontSize: 12)),
        const SizedBox(height: 14),
        TextField(controller: _email, keyboardType: TextInputType.emailAddress, decoration: InputDecoration(labelText: s('email'), hintText: 'driver@altius.test'), onChanged: (_) => cubit.store.saveDraft('email', _email.text)),
        const SizedBox(height: 14),
        TextField(controller: _password, obscureText: true, decoration: InputDecoration(labelText: s('password'))),
        const SizedBox(height: 10),
        Align(alignment: Alignment.centerLeft, child: TextButton(
          onPressed: () => showDialog<void>(context: context, builder: (_) => AlertDialog(title: Text(s('authHelp')), content: Text(Strings.fallback['authHelpBody']!), actions: [TextButton(onPressed: () => Navigator.pop(context), child: Text(s('cancel')))])),
          child: Text(s('authHelp')),
        )),
        const SizedBox(height: 8),
        FilledButton(
          onPressed: _busy ? null : () async {
            setState(() => _busy = true);
            final server = _server.text.trim();
            if (server.isEmpty) {
              await cubit.act(() => cubit.store.session(true));
            } else {
              await cubit.act(() => cubit.store.login(server, _email.text, _password.text));
            }
            if (mounted) setState(() => _busy = false);
          },
          child: Text(s('enter')),
        ),
        const SizedBox(height: 20),
        DropdownButtonFormField<String>(
          initialValue: cubit.state.language,
          decoration: InputDecoration(labelText: s('language')),
          items: [for (var i = 0; i < Strings.codes.length; i++) DropdownMenuItem(value: Strings.codes[i], child: Text(Strings.names[i]))],
          onChanged: (v) { if (v != null) cubit.act(() => cubit.store.language(v)); },
        ),
        const SizedBox(height: 24),
        Text('© 2026 Altius · Demo v0.1', style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Colors.grey)),
      ])),
    );
  }
}

class HomeShell extends StatefulWidget {
  const HomeShell({super.key});
  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _tab = 0;
  @override
  Widget build(BuildContext context) {
    return BlocConsumer<WorkCubit, WorkState>(
      listenWhen: (a, b) => a.error != b.error,
      listener: _errorSnack,
      builder: (context, state) {
        final s = Strings(state.language);
        
        final pages = [const TasksTab(), const RouteTab(), const ReportTab(), const SettingsTab()];
        return Scaffold(
          appBar: AppBar(title: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('hello'), style: Theme.of(context).textTheme.titleMedium),
            Text('${state.organization} · ${state.hub} · ${s('demo')}', style: const TextStyle(fontSize: 11, color: AppTheme.teal)),
          ])),
          body: pages[_tab],
          bottomNavigationBar: NavigationBar(
            selectedIndex: _tab, onDestinationSelected: (i) => setState(() => _tab = i),
            destinations: [
              NavigationDestination(icon: const Icon(Icons.task_alt_rounded), label: s('tasks')),
              NavigationDestination(icon: const Icon(Icons.route_rounded), label: s('route')),
              NavigationDestination(icon: const Icon(Icons.description_rounded), label: s('report')),
              NavigationDestination(icon: const Icon(Icons.settings_rounded), label: s('settings')),
            ]),
        );
      },
    );
  }
}

class TasksTab extends StatelessWidget {
  const TasksTab({super.key});
  @override
  Widget build(BuildContext context) {
    return BlocBuilder<WorkCubit, WorkState>(builder: (context, state) {
        final s = Strings(state.language);
      
      final cubit = context.read<WorkCubit>();
      final ongoing = state.tasks.where((t) => t.stage != TaskStage.done).toList();
      final done = state.tasks.where((t) => t.stage == TaskStage.done).toList();
      return ListView(padding: const EdgeInsets.all(16), children: [
        Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(s('today'), style: Theme.of(context).textTheme.titleLarge),
          const SizedBox(height: 8),
          Text('${done.length}/${state.tasks.length} ${s('done')} · ${s(state.activeTrip ? 'tripActive' : 'tripReady')}'),
          const SizedBox(height: 14),
          SizedBox(width: double.infinity, child: state.activeTrip
            ? FilledButton.tonalIcon(icon: const Icon(Icons.stop_rounded), onPressed: state.busy ? null : () => cubit.act(() => cubit.store.endTrip()), label: Text(s('endTrip')))
            : FilledButton.icon(icon: const Icon(Icons.play_arrow_rounded), onPressed: state.busy ? null : () => cubit.act(() => cubit.store.startTrip()), label: Text(s('startTrip')))),
        ]))),
        if (ongoing.isEmpty && done.isEmpty) Padding(padding: const EdgeInsets.all(40), child: Center(child: Text(s('empty')))),
        ...ongoing.map((t) => TaskCard(task: t, s: s)),
        if (done.isNotEmpty) ...[Padding(padding: const EdgeInsets.only(top: 12, bottom: 6), child: Text(s('done'), style: Theme.of(context).textTheme.titleMedium)), ...done.map((t) => TaskCard(task: t, s: s))],
      ]);
    });
  }
}

class TaskCard extends StatelessWidget {
  const TaskCard({super.key, required this.task, required this.s});
  final FieldTask task;
  final Strings s;
  @override
  Widget build(BuildContext context) {
    final stageKeys = ['assigned', 'arrived', 'working', 'done'];
    final done = task.stage == TaskStage.done;
    return Card(child: InkWell(
      borderRadius: BorderRadius.circular(28),
      onTap: () => Navigator.push(context, MaterialPageRoute<void>(builder: (_) => TaskDetailScreen(task: task))),
      child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(children: [
          Expanded(child: Text(task.title, style: Theme.of(context).textTheme.titleMedium)),
          Chip(label: Text(s(stageKeys[task.stage.index]), style: const TextStyle(fontSize: 11)), backgroundColor: done ? const Color(0xFFDFF2E8) : const Color(0xFFFDF1DF), side: BorderSide.none),
        ]),
        const SizedBox(height: 4),
        Text(task.address, style: Theme.of(context).textTheme.bodySmall),
        const SizedBox(height: 8),
        Row(children: [const Icon(Icons.schedule_rounded, size: 14, color: AppTheme.teal), const SizedBox(width: 4), Text('${s('eta')} · ${task.etaMinutes} ${s('min')}', style: const TextStyle(fontSize: 12, color: AppTheme.teal, fontWeight: FontWeight.w600))]),
      ])),
    ));
  }
}

class TaskDetailScreen extends StatelessWidget {
  const TaskDetailScreen({super.key, required this.task});
  final FieldTask task;
  @override
  Widget build(BuildContext context) {
    return BlocConsumer<WorkCubit, WorkState>(
      listenWhen: (a, b) => a.error != b.error,
      listener: _errorSnack,
      builder: (context, state) {
        final s = Strings(state.language);
        
        final cubit = context.read<WorkCubit>();
        final current = state.tasks.firstWhere((t) => t.id == task.id, orElse: () => task);
        final taskEvents = state.events.where((e) => e.entity == task.id).toList();
        Widget action(TaskStage next, IconData icon, String key) {
          final enabled = !state.busy && state.activeTrip && current.stage.index == next.index - 1;
          return SizedBox(width: double.infinity, child: FilledButton.icon(
            icon: Icon(icon),
            onPressed: enabled ? () => cubit.act(() => cubit.store.advance(task.id, next, requestId: cubit.store.newRequestId())) : null,
            label: Text(s(key)),
          ));
        }
        return Scaffold(
          appBar: AppBar(title: Text(s('detail'))),
          body: ListView(padding: const EdgeInsets.all(16), children: [
            Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(current.title, style: Theme.of(context).textTheme.titleLarge),
              const SizedBox(height: 4),
              Text(current.address, style: Theme.of(context).textTheme.bodyMedium),
              const SizedBox(height: 12),
              Container(padding: const EdgeInsets.all(12), decoration: BoxDecoration(color: const Color(0xFFEFF4F8), borderRadius: BorderRadius.circular(12)),
                child: Text(Strings.fallback['deliveryBody']!, style: const TextStyle(fontSize: 12, height: 1.5))),
            ]))),
            action(TaskStage.arrived, Icons.place_rounded, 'arrival'),
            const SizedBox(height: 10),
            action(TaskStage.working, Icons.play_circle_outline_rounded, 'activity'),
            const SizedBox(height: 10),
            action(TaskStage.done, Icons.check_circle_outline_rounded, 'complete'),
            if (!state.activeTrip) Padding(padding: const EdgeInsets.only(top: 8), child: Text(s('startTripFirst'), style: const TextStyle(color: AppTheme.amber))),
            const SizedBox(height: 16),
            Text(s('timeline'), style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 6),
            if (taskEvents.isEmpty) Text(s('empty'), style: Theme.of(context).textTheme.bodySmall),
            ...taskEvents.map((e) => ListTile(
              dense: true, leading: const Icon(Icons.bolt_rounded, size: 18, color: AppTheme.teal),
              title: Text(e.kind), subtitle: Text('${e.utc.toLocal()} · ${s('pending')}'),
            )),
          ]),
        );
      },
    );
  }
}

class RouteTab extends StatelessWidget {
  const RouteTab({super.key});
  @override
  Widget build(BuildContext context) {
    return BlocBuilder<WorkCubit, WorkState>(builder: (context, state) {
        final s = Strings(state.language);
      
      final distance = RouteDemo.shortestMinutes(RouteDemo.graph, 'hub');
      final remaining = state.tasks.where((t) => t.stage != TaskStage.done).toList();
      return ListView(padding: const EdgeInsets.all(16), children: [
        Container(padding: const EdgeInsets.all(14), decoration: BoxDecoration(color: const Color(0xFFEFF4F8), borderRadius: BorderRadius.circular(16)),
          child: Row(children: [const Icon(Icons.info_outline_rounded, color: AppTheme.teal), const SizedBox(width: 10), Expanded(child: Text(s('schematic'), style: const TextStyle(fontSize: 12)))])),
        const SizedBox(height: 14),
        Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(s('planned'), style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 10),
          SizedBox(height: 170, child: CustomPaint(painter: _RoutePainter(), size: Size.infinite)),
          const SizedBox(height: 8),
          Text(Strings.fallback['routeBody']!, style: const TextStyle(fontSize: 11, color: Colors.grey, height: 1.5)),
        ]))),
        Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(s('next'), style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          if (remaining.isEmpty) Text(s('empty'))
          else ...remaining.map((t) => ListTile(
            contentPadding: EdgeInsets.zero, leading: CircleAvatar(radius: 14, backgroundColor: AppTheme.cyan, child: Text('${remaining.indexOf(t) + 1}', style: const TextStyle(fontSize: 11, color: AppTheme.teal))),
            title: Text(t.title), subtitle: Text('${s('eta')}: ${distance[t.id] ?? t.etaMinutes} ${s('min')} · ${s('ata')}: —'),
          )),
        ]))),
        Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(s('safety'), style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Container(padding: const EdgeInsets.all(12), decoration: BoxDecoration(color: const Color(0xFFFDF1DF), borderRadius: BorderRadius.circular(12)),
            child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [const Icon(Icons.warning_amber_rounded, color: AppTheme.amber), const SizedBox(width: 10), Expanded(child: Text(Strings.fallback['safetyBody']!, style: const TextStyle(fontSize: 12, height: 1.5)))])),
          const SizedBox(height: 8),
          Text(s('noGps'), style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600)),
        ]))),
      ]);
    });
  }
}

class _RoutePainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final bg = Paint()..color = const Color(0xFFF5FAFD);
    canvas.drawRect(Offset.zero & size, bg);
    final planned = Paint()..color = AppTheme.teal..strokeWidth = 3..style = PaintingStyle.stroke;
    final simulated = Paint()..color = AppTheme.amber..strokeWidth = 2..style = PaintingStyle.stroke;
    final p = Path()..moveTo(24, size.height - 24)..lineTo(size.width * .35, size.height * .55)..lineTo(size.width * .62, size.height * .6)..lineTo(size.width * .8, size.height * .3)..lineTo(size.width - 24, size.height * .18);
    canvas.drawPath(p, planned);
    final d = Path()..moveTo(24, size.height - 24)..lineTo(size.width * .35, size.height * .55)..lineTo(size.width * .62, size.height * .6)..lineTo(size.width * .78, size.height * .42)..lineTo(size.width - 24, size.height * .18);
    for (var i = 0.0; i < 1.0; i += 0.06) {
      final metric = d.computeMetrics().first;
      final seg = metric.extractPath(metric.length * i, metric.length * (i + 0.03));
      canvas.drawPath(seg, simulated);
    }
    final dot = Paint()..color = AppTheme.cyan;
    for (final o in [Offset(24, size.height - 24), Offset(size.width * .35, size.height * .55), Offset(size.width * .62, size.height * .6), Offset(size.width * .8, size.height * .3), Offset(size.width - 24, size.height * .18)]) {
      canvas.drawCircle(o, 6, dot);
    }
  }
  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class ReportTab extends StatefulWidget {
  const ReportTab({super.key});
  @override
  State<ReportTab> createState() => _ReportTabState();
}

class _ReportTabState extends State<ReportTab> {
  String _category = 'fuel';
  final _amount = TextEditingController();
  final _note = TextEditingController();
  @override
  Widget build(BuildContext context) {
    return BlocConsumer<WorkCubit, WorkState>(
      listenWhen: (a, b) => a.error != b.error,
      listener: _errorSnack,
      builder: (context, state) {
        final s = Strings(state.language);
        
        final cubit = context.read<WorkCubit>();
        final today = cubit.store.today;
        final todayCosts = state.costs.where((c) => c.day == today).toList();
        final todayEvents = state.events.where((e) => e.day == today).toList();
        final visited = todayEvents.where((e) => e.kind == 'arrived').map((e) => e.entity).toSet().length;
        final completed = todayEvents.where((e) => e.kind == 'done').map((e) => e.entity).toSet().length;
        final total = todayCosts.fold(0, (sum, c) => sum + c.amount);
        final locked = state.reports.any((r) => r.day == today);
        return ListView(padding: const EdgeInsets.all(16), children: [
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('report'), style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 10),
            Row(children: [
              _Stat(label: s('visited'), value: '$visited'),
              _Stat(label: s('done'), value: '$completed'),
              _Stat(label: s('total'), value: 'IDR ${(total / 1000).toStringAsFixed(0)}k'),
            ]),
          ]))),
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('costs'), style: Theme.of(context).textTheme.titleMedium),
            ...todayCosts.map((c) => ListTile(contentPadding: EdgeInsets.zero, leading: const Icon(Icons.receipt_long_rounded, size: 20, color: AppTheme.teal),
              title: Text('${s(c.category)} · IDR ${c.amount}'), subtitle: Text(c.note))),
            const Divider(),
            Row(children: [
              Expanded(child: DropdownButtonFormField<String>(initialValue: _category, decoration: InputDecoration(labelText: s('addCost')),
                items: ['fuel', 'toll', 'parking', 'other'].map((c) => DropdownMenuItem(value: c, child: Text(s(c)))).toList(),
                onChanged: locked ? null : (v) => setState(() => _category = v ?? 'fuel'))),
            ]),
            const SizedBox(height: 10),
            TextField(controller: _amount, keyboardType: TextInputType.number, enabled: !locked, decoration: InputDecoration(labelText: s('amount'))),
            const SizedBox(height: 10),
            TextField(controller: _note, enabled: !locked, decoration: InputDecoration(labelText: s('note'))),
            const SizedBox(height: 12),
            SizedBox(width: double.infinity, child: OutlinedButton.icon(
              icon: const Icon(Icons.add_rounded),
              onPressed: locked || state.busy ? null : () => cubit.act(() => cubit.store.addCost(_category, int.tryParse(_amount.text) ?? 0, _note.text, requestId: cubit.store.newRequestId())).then((ok) { if (ok) { _amount.clear(); _note.clear(); } }),
              label: Text(s('addCost')),
            )),
          ]))),
          SizedBox(width: double.infinity, child: FilledButton.icon(
            icon: const Icon(Icons.send_rounded),
            onPressed: locked || state.busy || state.activeTrip ? null : () async {
              final confirm = await showDialog<bool>(context: context, builder: (_) => AlertDialog(
                title: Text(s('submit')), content: Text(s('confirmReport')),
                actions: [TextButton(onPressed: () => Navigator.pop(context, false), child: Text(s('cancel'))), FilledButton(onPressed: () => Navigator.pop(context, true), child: Text(s('submit')))],
              ));
              if (confirm == true) await cubit.act(() => cubit.store.submitReport());
            },
            label: Text(locked ? s('submitted') : s('submit')),
          )),
          const SizedBox(height: 16),
          ...state.reports.map((r) => Card(child: ListTile(leading: const Icon(Icons.lock_rounded, color: AppTheme.teal),
            title: Text('${r.day} · ${s('submitted')}'),
            subtitle: Text('${r.visited} ${s('visited')} · ${r.completed} ${s('done')} · IDR ${r.total}')))),
        ]);
      },
    );
  }
}

class _Stat extends StatelessWidget {
  const _Stat({required this.label, required this.value});
  final String label;
  final String value;
  @override
  Widget build(BuildContext context) => Expanded(child: Column(children: [
    Text(value, style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontFeatures: const [FontFeature.tabularFigures()])),
    const SizedBox(height: 2),
    Text(label, style: Theme.of(context).textTheme.bodySmall, textAlign: TextAlign.center),
  ]));
}

class SettingsTab extends StatelessWidget {
  const SettingsTab({super.key});
  @override
  Widget build(BuildContext context) {
    return BlocConsumer<WorkCubit, WorkState>(
      listenWhen: (a, b) => a.error != b.error,
      listener: _errorSnack,
      builder: (context, state) {
        final s = Strings(state.language);
        
        final cubit = context.read<WorkCubit>();
        final pending = state.events.where((e) => e.delivery == 'pending').length;
        return ListView(padding: const EdgeInsets.all(16), children: [
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('workspace'), style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            DropdownButtonFormField<String>(initialValue: state.organization, decoration: InputDecoration(labelText: s('organization')),
              items: ['Altius Demo', 'Altius Training'].map((o) => DropdownMenuItem(value: o, child: Text(o))).toList(),
              onChanged: (v) { if (v != null) cubit.act(() => cubit.store.selectWorkspace(v, state.hub)); }),
            const SizedBox(height: 10),
            DropdownButtonFormField<String>(initialValue: state.hub, decoration: InputDecoration(labelText: s('hub')),
              items: ['Jakarta', 'Bandung', 'Surabaya'].map((h) => DropdownMenuItem(value: h, child: Text(h))).toList(),
              onChanged: (v) { if (v != null) cubit.act(() => cubit.store.selectWorkspace(state.organization, v)); }),
            const SizedBox(height: 6),
            Text(Strings.fallback['workspaceBody']!, style: const TextStyle(fontSize: 11, color: Colors.grey)),
          ]))),
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('language'), style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            DropdownButtonFormField<String>(initialValue: state.language,
              items: [for (var i = 0; i < Strings.codes.length; i++) DropdownMenuItem(value: Strings.codes[i], child: Text(Strings.names[i]))],
              onChanged: (v) { if (v != null) cubit.act(() => cubit.store.language(v)); }),
          ]))),
          Card(child: Column(children: [
            ListTile(leading: const Icon(Icons.outbox_rounded), title: Text(s('history')), subtitle: Text('$pending ${s('pending')}'), trailing: TextButton(onPressed: () => cubit.act(() async {
              final base = await cubit.store.preference('apiBase');
              final token = await cubit.store.accessToken();
              if (base.isNotEmpty && token.isNotEmpty) {
                try {
                  await cubit.store.syncNow(baseUrl: base, accessToken: token);
                } catch (_) {
                  await cubit.store.retrySync(offline: true);
                  rethrow;
                }
              } else {
                await cubit.store.retrySync(offline: true);
              }
            }), child: Text(s('submit')))),
            const Divider(height: 1),
            ...state.events.reversed.take(6).map((e) => ListTile(dense: true, leading: const Icon(Icons.bolt_rounded, size: 16), title: Text(e.kind), subtitle: Text('${e.day} · ${s('localSaved')}'))),
          ])),
          Card(child: Column(children: [
            ListTile(leading: const Icon(Icons.shield_outlined), title: Text(s('permissions')), subtitle: Text(Strings.fallback['permissionsBody']!, style: const TextStyle(fontSize: 11))),
            ListTile(leading: const Icon(Icons.privacy_tip_outlined), title: Text(s('privacy')), subtitle: Text(Strings.fallback['privacyBody']!, style: const TextStyle(fontSize: 11))),
            ListTile(leading: const Icon(Icons.build_outlined), title: Text(s('help')), subtitle: Text(Strings.fallback['helpBody']!, style: const TextStyle(fontSize: 11))),
          ])),
          SizedBox(width: double.infinity, child: OutlinedButton.icon(
            icon: const Icon(Icons.logout_rounded),
            onPressed: () => cubit.act(() => cubit.store.logout()),
            label: Text(s('logout')),
          )),
          Padding(padding: const EdgeInsets.all(16), child: Text(s('protected'), textAlign: TextAlign.center, style: const TextStyle(fontSize: 12, color: Colors.grey))),
        ]);
      },
    );
  }
}
