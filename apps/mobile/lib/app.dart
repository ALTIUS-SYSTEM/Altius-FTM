import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:share_plus/share_plus.dart';
import 'core/data/work_store.dart';
import 'core/l10n/strings.dart';
import 'core/services/route_service.dart';
import 'core/theme/app_theme.dart';
import 'features/work/work_cubit.dart';

/// Altius Field driver app. Live mode uses PKCE against Keycloak and syncs
/// to the Altius API; demo mode keeps all data on-device in SQLite.
class AltiusApp extends StatelessWidget {
  const AltiusApp({super.key, required this.cubit, required this.environment});
  final WorkCubit cubit;
  final String environment;
  @override
  Widget build(BuildContext context) {
    final demoMode = environment != 'prod';
    return BlocProvider.value(
      value: cubit,
      child: BlocBuilder<WorkCubit, WorkState>(builder: (context, state) {
        return MaterialApp(
          title: 'Altius Field',
          theme: AppTheme.light,
          supportedLocales: Strings.locales,
          locale: Strings.locales[Strings.codes.indexOf(state.language).clamp(0, 6)],
          localizationsDelegates: const [GlobalMaterialLocalizations.delegate, GlobalWidgetsLocalizations.delegate, GlobalCupertinoLocalizations.delegate],
          home: state.session ? HomeShell(demoMode: demoMode) : LoginScreen(demoMode: demoMode),
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
  const LoginScreen({super.key, this.demoMode = true});
  final bool demoMode;
  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _apiBase = TextEditingController();
  final _issuer = TextEditingController();
  final _clientId = TextEditingController();
  final _redirectUri = TextEditingController();
  final _username = TextEditingController();
  final _password = TextEditingController();
  bool _busy = false;

  static const _defaultApiBase = String.fromEnvironment('API_BASE', defaultValue: '');
  static const _defaultIssuer = String.fromEnvironment('KEYCLOAK_ISSUER', defaultValue: '');
  static const _defaultClientId = String.fromEnvironment('KEYCLOAK_CLIENT_ID', defaultValue: '');
  /// Must match Android/iOS AppAuth redirect registration.
  static const _defaultRedirectUri = String.fromEnvironment(
    'KEYCLOAK_REDIRECT_URI',
    defaultValue: 'com.altius.altiusfield:/oauthredirect',
  );

  /// Compile-time IdP + API wiring — when present, Keycloak is the primary path.
  static bool get _liveConfigured =>
      _defaultApiBase.isNotEmpty && _defaultIssuer.isNotEmpty && _defaultClientId.isNotEmpty;

  @override
  void initState() {
    super.initState();
    _apiBase.text = _defaultApiBase;
    _issuer.text = _defaultIssuer;
    _clientId.text = _defaultClientId;
    _redirectUri.text = _defaultRedirectUri;
  }

  bool get _wantsKeycloak {
    if (_liveConfigured || !widget.demoMode) return true;
    return _apiBase.text.trim().isNotEmpty ||
        _issuer.text.trim().isNotEmpty ||
        _clientId.text.trim().isNotEmpty;
  }

  @override
  Widget build(BuildContext context) {
    final cubit = context.read<WorkCubit>();
    final s = Strings(cubit.state.language);
    final livePrimary = _liveConfigured || !widget.demoMode;
    return Scaffold(
      body: SafeArea(child: ListView(padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 32), children: [
        const SizedBox(height: 24),
        SvgPicture.asset('assets/altius-logo.svg', width: 64, height: 64),
        const SizedBox(height: 20),
        Text('Altius Field', style: Theme.of(context).textTheme.headlineLarge),
        Text(s('driverApp'), style: Theme.of(context).textTheme.titleMedium?.copyWith(color: AppTheme.teal)),
        const SizedBox(height: 12),
        if (!livePrimary)
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(color: const Color(0xFFFDF1DF), borderRadius: BorderRadius.circular(14)),
            child: Text(s('demoNotice'), style: const TextStyle(fontSize: 12, color: Color(0xFF573300), height: 1.5)),
          )
        else
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(color: const Color(0xFFEFF4F8), borderRadius: BorderRadius.circular(14)),
            child: Text(s('liveNotice'), style: const TextStyle(fontSize: 12, color: Color(0xFF1A3A4A), height: 1.5)),
          ),
        const SizedBox(height: 28),
        if (livePrimary || _wantsKeycloak) ...[
          TextField(controller: _username, keyboardType: TextInputType.emailAddress, autocorrect: false, decoration: const InputDecoration(labelText: 'Email', hintText: 'admin@altiussystem.com')),
          const SizedBox(height: 14),
          TextField(controller: _password, obscureText: true, decoration: const InputDecoration(labelText: 'Password')),
          const SizedBox(height: 10),
        ],
        // Surface login errors inline — the login screen is not wrapped in the
        // BlocListener that shows SnackBars, so a failed sign-in looked like a
        // dead button.
        if (cubit.state.error != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Text(s(cubit.state.error!), style: const TextStyle(color: Color(0xFFB3261E), fontSize: 13)),
          ),
        Align(alignment: Alignment.centerLeft, child: TextButton(
          onPressed: () => showDialog<void>(context: context, builder: (_) => AlertDialog(title: Text(s('authHelp')), content: Text(livePrimary ? Strings.fallback['authHelpLiveBody']! : Strings.fallback['authHelpBody']!), actions: [TextButton(onPressed: () => Navigator.pop(context), child: Text(s('cancel')))])),
          child: Text(s('authHelp')),
        )),
        const SizedBox(height: 8),
        FilledButton(
          onPressed: _busy ? null : () async {
            debugPrint('[login] tap wantsKeycloak=$_wantsKeycloak busy=${cubit.state.busy}');
            setState(() => _busy = true);
            await cubit.act(() async {
              if (_wantsKeycloak) {
                await cubit.store.configureAuth(
                  apiBase: _apiBase.text.trim(),
                  issuer: _issuer.text.trim(),
                  clientId: _clientId.text.trim(),
                  redirectUri: _redirectUri.text.trim(),
                );
                await cubit.store.signInWithPassword(
                  _username.text.trim(),
                  _password.text,
                );
              } else {
                await cubit.store.enterDemoWorkspace();
              }
            });
            if (mounted) setState(() => _busy = false);
          },
          child: Text(_wantsKeycloak ? s('signIn') : s('enter')),
        ),
        if (!livePrimary && !_wantsKeycloak) ...[
          const SizedBox(height: 12),
          TextButton(
            onPressed: _busy ? null : () => setState(() {
              _apiBase.text = _defaultApiBase.isEmpty ? 'http://127.0.0.1:8080' : _defaultApiBase;
              _issuer.text = _defaultIssuer.isEmpty ? 'http://127.0.0.1:8081/realms/altius' : _defaultIssuer;
              _clientId.text = _defaultClientId.isEmpty ? 'altius-mobile' : _defaultClientId;
            }),
            child: Text(s('useKeycloak')),
          ),
        ],
        const SizedBox(height: 20),
        DropdownButtonFormField<String>(
          initialValue: cubit.state.language,
          decoration: InputDecoration(labelText: s('language')),
          items: [for (var i = 0; i < Strings.codes.length; i++) DropdownMenuItem(value: Strings.codes[i], child: Text(Strings.names[i]))],
          onChanged: (v) { if (v != null) cubit.act(() => cubit.store.language(v)); },
        ),
        const SizedBox(height: 24),
        Text('© 2026 Altius', style: Theme.of(context).textTheme.bodySmall?.copyWith(color: Colors.grey)),
      ])),
    );
  }
}

class HomeShell extends StatefulWidget {
  const HomeShell({super.key, this.demoMode = true});
  final bool demoMode;
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
        
        final pages = [const TasksTab(), const RouteTab(), const ReportTab(), const VehicleCheckTab(), const SettingsTab()];
        return Scaffold(
          appBar: AppBar(title: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('hello'), style: Theme.of(context).textTheme.titleMedium),
            Text(
              widget.demoMode && state.organization == 'Altius Demo'
                  ? '${state.organization} · ${state.hub} · ${s('demo')}'
                  : '${state.organization} · ${state.hub}',
              style: const TextStyle(fontSize: 11, color: AppTheme.teal),
            ),
          ])),
          body: pages[_tab],
          bottomNavigationBar: NavigationBar(
            selectedIndex: _tab, onDestinationSelected: (i) => setState(() => _tab = i),
            destinations: [
              NavigationDestination(icon: const Icon(Icons.task_alt_rounded), label: s('tasks')),
              NavigationDestination(icon: const Icon(Icons.route_rounded), label: s('route')),
              NavigationDestination(icon: const Icon(Icons.description_rounded), label: s('report')),
              NavigationDestination(icon: const Icon(Icons.fact_check_rounded), label: s('vehicleCheck')),
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

/// Outbox delivery state → translation key. An event that left `pending` is
/// never retried, so a rejected one must not keep reading as "pending".
String _deliveryLabel(String delivery) => switch (delivery) {
  'accepted' => 'synced',
  'rejected' => 'rejectedEvent',
  'local' => 'localOnly',
  _ => 'pending',
};

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
              // Show the real delivery state: hard-coding "pending" hid every
              // server-rejected event, which is never retried.
              title: Text(e.kind), subtitle: Text('${e.utc.toLocal()} · ${s(_deliveryLabel(e.delivery))}'),
            )),
          ]),
        );
      },
    );
  }
}

class RouteTab extends StatefulWidget {
  const RouteTab({super.key});
  @override
  State<RouteTab> createState() => _RouteTabState();
}

class _RouteTabState extends State<RouteTab> {
  static const _service = RouteService();
  PlannedRoute? _route;
  Uint8List? _map;
  String? _error;
  bool _busy = false;
  String _plannedFor = '';

  /// Re-plan when the set of remaining located stops changes, not on every
  /// rebuild — each plan is two billed Google calls.
  Future<void> _plan(List<FieldTask> located) async {
    final key = located.map((t) => t.id).join(',');
    if (_busy || key == _plannedFor || located.length < 2) return;
    final store = context.read<WorkCubit>().store;
    final base = await store.preference('apiBase');
    final token = await store.accessToken();
    // No session or no server configured: show the stop list, plan nothing.
    if (base.isEmpty || token == null || token.isEmpty) return;

    setState(() { _busy = true; _error = null; });
    final points = [for (final t in located) (lat: t.lat!, lng: t.lng!)];
    try {
      final route = await _service.optimize(baseUrl: base, token: token, waypoints: points);
      final ordered = <({double lat, double lng})>[
        points.first,
        for (final i in route.order) if (i + 1 < points.length) points[i + 1],
      ];
      final image = await _service.mapImage(
        baseUrl: base, token: token, markers: ordered, polyline: route.polyline);
      if (!mounted) return;
      setState(() { _route = route; _map = image; _plannedFor = key; });
    } on Object {
      if (mounted) setState(() => _error = 'routeUnavailable');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder<WorkCubit, WorkState>(builder: (context, state) {
      final s = Strings(state.language);
      final remaining = state.tasks.where((t) => t.stage != TaskStage.done).toList();
      final located = remaining.where((t) => t.isLocated).toList();
      final unlocated = remaining.length - located.length;
      WidgetsBinding.instance.addPostFrameCallback((_) => _plan(located));

      final route = _route;
      final ordered = route == null
          ? remaining
          : <FieldTask>[
              located.first,
              for (final i in route.order) if (i + 1 < located.length) located[i + 1],
            ];

      return ListView(padding: const EdgeInsets.all(16), children: [
        Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(s('planned'), style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 10),
          if (_busy) const Padding(padding: EdgeInsets.symmetric(vertical: 24), child: Center(child: CircularProgressIndicator()))
          else if (_map != null) ClipRRect(borderRadius: BorderRadius.circular(12), child: Image.memory(_map!, fit: BoxFit.cover))
          else Container(height: 120, alignment: Alignment.center,
            decoration: BoxDecoration(color: const Color(0xFFEFF4F8), borderRadius: BorderRadius.circular(12)),
            child: Text(s('noMap'), style: const TextStyle(fontSize: 12, color: Colors.grey))),
          const SizedBox(height: 10),
          if (_error != null) Text(s(_error!), style: const TextStyle(fontSize: 12, color: AppTheme.amber))
          else if (route != null) Text(
            '${route.totalKm.toStringAsFixed(1)} km · ${route.totalMinutes} ${s('min')} · ${s(route.live ? 'roadRouting' : 'straightLine')}',
            style: const TextStyle(fontSize: 12, height: 1.5))
          else if (located.length < 2) Text(s('needCoords'), style: const TextStyle(fontSize: 12, color: Colors.grey)),
          if (unlocated > 0) Padding(padding: const EdgeInsets.only(top: 6),
            child: Text('$unlocated ${s('stopsNoPosition')}', style: const TextStyle(fontSize: 11, color: Colors.grey))),
        ]))),
        Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(s('next'), style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          if (ordered.isEmpty) Text(s('empty'))
          else ...ordered.asMap().entries.map((e) => ListTile(
            contentPadding: EdgeInsets.zero,
            leading: CircleAvatar(radius: 14, backgroundColor: AppTheme.cyan, child: Text('${e.key + 1}', style: const TextStyle(fontSize: 11, color: AppTheme.teal))),
            title: Text(e.value.title),
            subtitle: Text('${s('eta')}: ${route != null && e.key < route.legMinutes.length ? route.legMinutes[e.key] : e.value.etaMinutes} ${s('min')} · ${s('ata')}: —'),
          )),
        ]))),
        Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(s('safety'), style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Container(padding: const EdgeInsets.all(12), decoration: BoxDecoration(color: const Color(0xFFFDF1DF), borderRadius: BorderRadius.circular(12)),
            child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [const Icon(Icons.warning_amber_rounded, color: AppTheme.amber), const SizedBox(width: 10), Expanded(child: Text(Strings.fallback['safetyBody']!, style: const TextStyle(fontSize: 12, height: 1.5)))])),
        ]))),
      ]);
    });
  }
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
  final _driverName = TextEditingController();
  final _vehicleNumber = TextEditingController();
  final _odometerStart = TextEditingController();
  final _odometerEnd = TextEditingController();
  final _reportNotes = TextEditingController();
  @override
  void dispose() {
    _amount.dispose();
    _note.dispose();
    _driverName.dispose();
    _vehicleNumber.dispose();
    _odometerStart.dispose();
    _odometerEnd.dispose();
    _reportNotes.dispose();
    super.dispose();
  }
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
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('lhsDetails'), style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            TextField(controller: _driverName, enabled: !locked, decoration: InputDecoration(labelText: s('driverName'))),
            const SizedBox(height: 10),
            TextField(controller: _vehicleNumber, enabled: !locked, decoration: InputDecoration(labelText: s('vehicleNumber'))),
            const SizedBox(height: 10),
            Row(children: [
              Expanded(child: TextField(controller: _odometerStart, keyboardType: TextInputType.number, enabled: !locked, decoration: InputDecoration(labelText: s('odometerStart')))),
              const SizedBox(width: 12),
              Expanded(child: TextField(controller: _odometerEnd, keyboardType: TextInputType.number, enabled: !locked, decoration: InputDecoration(labelText: s('odometerEnd')))),
            ]),
            const SizedBox(height: 10),
            TextField(controller: _reportNotes, enabled: !locked, maxLines: 3, decoration: InputDecoration(labelText: s('reportNotes'))),
          ]))),
          SizedBox(width: double.infinity, child: FilledButton.icon(
            icon: const Icon(Icons.send_rounded),
            onPressed: locked || state.busy || state.activeTrip ? null : () async {
              final confirm = await showDialog<bool>(context: context, builder: (_) => AlertDialog(
                title: Text(s('submit')), content: Text(s('confirmReport')),
                actions: [TextButton(onPressed: () => Navigator.pop(context, false), child: Text(s('cancel'))), FilledButton(onPressed: () => Navigator.pop(context, true), child: Text(s('submit')))],
              ));
              if (confirm == true) {
                await cubit.act(() => cubit.store.submitReport(
                  driverName: _driverName.text,
                  vehicleNumber: _vehicleNumber.text,
                  odometerStart: int.tryParse(_odometerStart.text) ?? 0,
                  odometerEnd: int.tryParse(_odometerEnd.text) ?? 0,
                  notes: _reportNotes.text,
                ));
              }
            },
            label: Text(locked ? s('submitted') : s('submit')),
          )),
          if (locked) Padding(padding: const EdgeInsets.only(top: 12), child: Row(children: [
            // The driver's own copy of the record. A clipboard button meets the
            // "send the LHS to a supervisor" need without a messaging vendor.
            Expanded(child: OutlinedButton.icon(
              icon: const Icon(Icons.copy_rounded, size: 18),
              onPressed: () async {
                final text = await cubit.store.reportText(today);
                await Clipboard.setData(ClipboardData(text: text));
                if (context.mounted) ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(s('copied'))));
              },
              label: Text(s('copyText')),
            )),
            const SizedBox(width: 10),
            Expanded(child: OutlinedButton.icon(
              icon: const Icon(Icons.data_object_rounded, size: 18),
              onPressed: () async {
                final json = await cubit.store.reportJson(today);
                await Clipboard.setData(ClipboardData(text: json));
                if (context.mounted) ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(s('copied'))));
              },
              label: Text(s('copyJson')),
            )),
            const SizedBox(width: 10),
            Expanded(child: OutlinedButton.icon(
              icon: const Icon(Icons.share_rounded, size: 18),
              onPressed: () async {
                final text = await cubit.store.reportText(today);
                await SharePlus.instance.share(ShareParams(text: text, subject: 'Laporan Harian Sopir'));
              },
              label: Text(s('share')),
            )),
          ])),
          const SizedBox(height: 16),
          ...state.reports.map((r) => Card(child: ListTile(leading: const Icon(Icons.lock_rounded, color: AppTheme.teal),
            title: Text('${r.day} · ${s('submitted')}'),
            subtitle: Text('${r.vehicleNumber} · ${r.odometerStart}-${r.odometerEnd} km · ${r.visited} ${s('visited')} · ${r.completed} ${s('done')} · IDR ${r.total} · ${r.notes}')))),
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

const _equipmentItems = ['STNK', 'KIUR / KIR', 'BAN SEREP', 'TOOLS (Kunci-Kunci)', 'DONGKRAK', 'SEGITIGA PENGAMAN', 'OBAT P3K', 'APAR'];
const _inspectionItems = ['OLI MESIN', 'OLI GARDAN', 'OLI PERSNELING', 'AIR RADIATOR', 'AIR ACCU / AKI', 'BAN', 'REM', 'KOPLING', 'BODY SCREENING', 'LAMPU-LAMPU', 'KACA SPION'];

class VehicleCheckTab extends StatefulWidget {
  const VehicleCheckTab({super.key});
  @override
  State<VehicleCheckTab> createState() => _VehicleCheckTabState();
}

class _VehicleCheckTabState extends State<VehicleCheckTab> {
  final _driverName = TextEditingController();
  final _licensePlate = TextEditingController();
  final _vehicleType = TextEditingController();
  final _kmStart = TextEditingController();
  final _kmEnd = TextEditingController();
  final _notes = TextEditingController();
  final _serviceDate = TextEditingController();
  final _kirDate = TextEditingController();
  final _stnkDate = TextEditingController();
  final Map<String, String> _items = {};
  bool _conditionGood = true;

  @override
  void dispose() {
    _driverName.dispose();
    _licensePlate.dispose();
    _vehicleType.dispose();
    _kmStart.dispose();
    _kmEnd.dispose();
    _notes.dispose();
    _serviceDate.dispose();
    _kirDate.dispose();
    _stnkDate.dispose();
    super.dispose();
  }

  String _status(String name) => _items.putIfAbsent(name, () => _isEquipment(name) ? 'present' : 'good');

  bool _isEquipment(String name) => _equipmentItems.contains(name);

  List<Map<String, Object?>> _buildItems() {
    final out = <Map<String, Object?>>[];
    for (final name in _equipmentItems) {
      out.add({'name': name, 'category': 'equipment', 'status': _items[name] ?? 'present', 'note': ''});
    }
    for (final name in _inspectionItems) {
      out.add({'name': name, 'category': 'inspection', 'status': _items[name] ?? 'good', 'note': ''});
    }
    return out;
  }

  @override
  Widget build(BuildContext context) {
    return BlocConsumer<WorkCubit, WorkState>(
      listenWhen: (a, b) => a.error != b.error,
      listener: _errorSnack,
      builder: (context, state) {
        final s = Strings(state.language);
        final cubit = context.read<WorkCubit>();
        final todayCheck = state.vehicleChecks.where((c) => c.day == cubit.store.today).firstOrNull;

        return ListView(padding: const EdgeInsets.all(16), children: [
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('vehicleCheck'), style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 10),
            if (todayCheck != null) ...[
              Text(s('vehicleCheckSubmitted'), style: const TextStyle(color: AppTheme.teal, fontWeight: FontWeight.w600)),
              const SizedBox(height: 6),
              Text('${todayCheck.licensePlate} · ${todayCheck.vehicleType} · ${todayCheck.condition == 'good' ? s('conditionGood') : s('conditionBad')}'),
            ],
            TextField(controller: _driverName, decoration: InputDecoration(labelText: s('driverName'))),
            const SizedBox(height: 10),
            TextField(controller: _licensePlate, decoration: InputDecoration(labelText: s('licensePlate'))),
            const SizedBox(height: 10),
            TextField(controller: _vehicleType, decoration: InputDecoration(labelText: s('vehicleType'))),
            const SizedBox(height: 10),
            Row(children: [
              Expanded(child: TextField(controller: _kmStart, keyboardType: TextInputType.number, decoration: InputDecoration(labelText: s('kmStart')))),
              const SizedBox(width: 12),
              Expanded(child: TextField(controller: _kmEnd, keyboardType: TextInputType.number, decoration: InputDecoration(labelText: s('kmEnd')))),
            ]),
            const SizedBox(height: 10),
            Row(children: [
              ChoiceChip(label: Text(s('conditionGood')), selected: _conditionGood, onSelected: (_) => setState(() => _conditionGood = true)),
              const SizedBox(width: 8),
              ChoiceChip(label: Text(s('conditionBad')), selected: !_conditionGood, onSelected: (_) => setState(() => _conditionGood = false)),
            ]),
          ]))),
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('checklist'), style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            ...[..._equipmentItems, ..._inspectionItems].map((name) {
              final equipment = _isEquipment(name);
              final status = _status(name);
              return ListTile(
                contentPadding: EdgeInsets.zero,
                title: Text(name, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
                subtitle: Wrap(spacing: 8, children: [
                  for (final opt in equipment ? ['present', 'missing'] : ['good', 'damaged'])
                    ChoiceChip(
                      label: Text(opt),
                      selected: status == opt,
                      onSelected: (_) => setState(() => _items[name] = opt),
                    ),
                ]),
              );
            }),
          ]))),
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('reportNotes'), style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            TextField(controller: _notes, maxLines: 3, decoration: const InputDecoration(hintText: 'Keterangan')),
          ]))),
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('serviceSchedule'), style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            TextField(controller: _serviceDate, decoration: const InputDecoration(labelText: 'Service (YYYY-MM-DD)')),
            const SizedBox(height: 10),
            TextField(controller: _kirDate, decoration: const InputDecoration(labelText: 'KIR (YYYY-MM-DD)')),
            const SizedBox(height: 10),
            TextField(controller: _stnkDate, decoration: const InputDecoration(labelText: 'STNK (YYYY-MM-DD)')),
          ]))),
          SizedBox(width: double.infinity, child: FilledButton.icon(
            icon: const Icon(Icons.save_rounded),
            onPressed: state.busy || todayCheck != null ? null : () => cubit.act(() => cubit.store.submitVehicleCheck(
              driverName: _driverName.text,
              licensePlate: _licensePlate.text,
              vehicleType: _vehicleType.text,
              kmStart: int.tryParse(_kmStart.text) ?? 0,
              kmEnd: int.tryParse(_kmEnd.text) ?? 0,
              condition: _conditionGood ? 'good' : 'not_good',
              items: _buildItems(),
              notes: _notes.text,
              serviceDate: _serviceDate.text,
              kirDate: _kirDate.text,
              stnkDate: _stnkDate.text,
            )),
            label: Text(s('save')),
          )),
          if (todayCheck != null) Padding(padding: const EdgeInsets.all(16), child: Text(s('vehicleCheckSubmitted'), textAlign: TextAlign.center, style: const TextStyle(color: AppTheme.teal))),
        ]);
      },
    );
  }
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
        // Rejected events are terminal and never retried. Counting only
        // 'pending' made them vanish from the driver's view entirely, so a
        // lost proof-of-service record looked like a clean outbox.
        final rejected = state.events.where((e) => e.delivery == 'rejected').length;
        return ListView(padding: const EdgeInsets.all(16), children: [
          Card(child: Padding(padding: const EdgeInsets.all(18), child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(s('workspace'), style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            DropdownButtonFormField<String>(initialValue: state.organization, decoration: InputDecoration(labelText: s('organization')),
              // Live orgs come from the backend (e.g. `altius`), not the demo
              // list — include the current value so the assertion holds.
              items: {if (state.organization.isNotEmpty) state.organization, 'Altius Demo', 'Altius Training'}.map((o) => DropdownMenuItem(value: o, child: Text(o))).toList(),
              onChanged: (v) { if (v != null) cubit.act(() => cubit.store.selectWorkspace(v, state.hub)); }),
            DropdownButtonFormField<String>(initialValue: state.hub, decoration: InputDecoration(labelText: s('hub')),
              items: {if (state.hub.isNotEmpty) state.hub, 'Jakarta', 'Bandung', 'Surabaya'}.map((h) => DropdownMenuItem(value: h, child: Text(h))).toList(),
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
            ListTile(leading: const Icon(Icons.outbox_rounded), title: Text(s('history')), subtitle: Text(rejected > 0 ? '$pending ${s('pending')} · $rejected ${s('rejectedEvent')}' : '$pending ${s('pending')}'), trailing: TextButton(onPressed: () => cubit.act(() async {
              final base = await cubit.store.preference('apiBase');
              final token = await cubit.store.accessToken();
              if (base.isNotEmpty && token != null && token.isNotEmpty) {
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
