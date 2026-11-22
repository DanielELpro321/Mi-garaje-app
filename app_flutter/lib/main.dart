import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'maintenance_voice.dart';

void main() => runApp(const GarageApp());

const _background = Color(0xFF090B0E);
const _surface = Color(0xFF14181F);
const _surface2 = Color(0xFF1B212A);
const _line = Color(0xFF29313C);
const _red = Color(0xFFE53935);
const _muted = Color(0xFF929AA6);
const _green = Color(0xFF43B65A);
const _dataKey = 'mi_garaje_data_v1';

String _id() => DateTime.now().microsecondsSinceEpoch.toString();
String _today() {
  final d = DateTime.now();
  return '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
}

String _money(num value) {
  final digits = value.round().toString();
  final grouped = digits.replaceAllMapped(
    RegExp(r'\B(?=(\d{3})+(?!\d))'),
    (_) => '.',
  );
  return '\$ $grouped';
}

String _dateLabel(String value) {
  final d = DateTime.tryParse(value);
  if (d == null) return value;
  return '${d.day.toString().padLeft(2, '0')}/${d.month.toString().padLeft(2, '0')}/${d.year}';
}

class Vehicle {
  Vehicle({
    String? id,
    required this.brand,
    required this.model,
    required this.year,
    required this.cc,
    required this.km,
    this.plate = '',
    this.color = '',
  }) : id = id ?? _id();

  final String id;
  String brand;
  String model;
  int year;
  int cc;
  int km;
  String plate;
  String color;

  String get title => '$brand $model'.trim();

  Map<String, Object?> toJson() => {
        'id': id,
        'brand': brand,
        'model': model,
        'year': year,
        'cc': cc,
        'km': km,
        'plate': plate,
        'color': color,
      };

  factory Vehicle.fromJson(Map<String, dynamic> json) => Vehicle(
        id: json['id'] as String?,
        brand: json['brand'] as String? ?? '',
        model: json['model'] as String? ?? '',
        year: (json['year'] as num?)?.toInt() ?? DateTime.now().year,
        cc: (json['cc'] as num?)?.toInt() ?? 150,
        km: (json['km'] as num?)?.toInt() ?? 0,
        plate: json['plate'] as String? ?? '',
        color: json['color'] as String? ?? '',
      );
}

class FuelEntry {
  FuelEntry({
    String? id,
    required this.vehicleId,
    required this.date,
    required this.station,
    required this.amount,
    required this.pricePerGallon,
    required this.km,
  }) : id = id ?? _id();

  final String id;
  String vehicleId;
  String date;
  String station;
  double amount;
  double pricePerGallon;
  int km;

  Map<String, Object?> toJson() => {
        'id': id,
        'vehicleId': vehicleId,
        'date': date,
        'station': station,
        'amount': amount,
        'pricePerGallon': pricePerGallon,
        'km': km,
      };

  factory FuelEntry.fromJson(Map<String, dynamic> json) => FuelEntry(
        id: json['id'] as String?,
        vehicleId: json['vehicleId'] as String? ?? '',
        date: json['date'] as String? ?? _today(),
        station: json['station'] as String? ?? '',
        amount: (json['amount'] as num?)?.toDouble() ?? 0,
        pricePerGallon: (json['pricePerGallon'] as num?)?.toDouble() ?? 0,
        km: (json['km'] as num?)?.toInt() ?? 0,
      );
}

class ServiceEntry {
  ServiceEntry({
    String? id,
    required this.vehicleId,
    required this.name,
    required this.intervalKm,
    required this.lastKm,
    required this.cost,
    required this.date,
  }) : id = id ?? _id();

  final String id;
  String vehicleId;
  String name;
  int intervalKm;
  int lastKm;
  double cost;
  String date;

  int remaining(int currentKm) => intervalKm - (currentKm - lastKm);

  Map<String, Object?> toJson() => {
        'id': id,
        'vehicleId': vehicleId,
        'name': name,
        'intervalKm': intervalKm,
        'lastKm': lastKm,
        'cost': cost,
        'date': date,
      };

  factory ServiceEntry.fromJson(Map<String, dynamic> json) => ServiceEntry(
        id: json['id'] as String?,
        vehicleId: json['vehicleId'] as String? ?? '',
        name: json['name'] as String? ?? '',
        intervalKm: (json['intervalKm'] as num?)?.toInt() ?? 3000,
        lastKm: (json['lastKm'] as num?)?.toInt() ?? 0,
        cost: (json['cost'] as num?)?.toDouble() ?? 0,
        date: json['date'] as String? ?? _today(),
      );
}

class VehicleDocument {
  VehicleDocument({
    String? id,
    required this.vehicleId,
    required this.name,
    required this.expiry,
  }) : id = id ?? _id();

  final String id;
  String vehicleId;
  String name;
  String expiry;

  Map<String, Object?> toJson() => {
        'id': id,
        'vehicleId': vehicleId,
        'name': name,
        'expiry': expiry,
      };

  factory VehicleDocument.fromJson(Map<String, dynamic> json) => VehicleDocument(
        id: json['id'] as String?,
        vehicleId: json['vehicleId'] as String? ?? '',
        name: json['name'] as String? ?? '',
        expiry: json['expiry'] as String? ?? _today(),
      );
}

class GarageData {
  GarageData();

  List<Vehicle> vehicles = [];
  List<FuelEntry> fuel = [];
  List<ServiceEntry> services = [];
  List<VehicleDocument> documents = [];
  String owner = '';
  String city = '';
  double pricePerGallon = 16350;
  bool notifications = true;
  bool reminders = true;

  Map<String, Object?> toJson() => {
        'vehicles': vehicles.map((e) => e.toJson()).toList(),
        'fuel': fuel.map((e) => e.toJson()).toList(),
        'services': services.map((e) => e.toJson()).toList(),
        'documents': documents.map((e) => e.toJson()).toList(),
        'owner': owner,
        'city': city,
        'pricePerGallon': pricePerGallon,
        'notifications': notifications,
        'reminders': reminders,
      };

  factory GarageData.fromJson(Map<String, dynamic> json) {
    final data = GarageData()
      ..vehicles = (json['vehicles'] as List? ?? [])
          .whereType<Map>()
          .map((e) => Vehicle.fromJson(Map<String, dynamic>.from(e)))
          .toList()
      ..fuel = (json['fuel'] as List? ?? [])
          .whereType<Map>()
          .map((e) => FuelEntry.fromJson(Map<String, dynamic>.from(e)))
          .toList()
      ..services = (json['services'] as List? ?? [])
          .whereType<Map>()
          .map((e) => ServiceEntry.fromJson(Map<String, dynamic>.from(e)))
          .toList()
      ..documents = (json['documents'] as List? ?? [])
          .whereType<Map>()
          .map((e) => VehicleDocument.fromJson(Map<String, dynamic>.from(e)))
          .toList()
      ..owner = json['owner'] as String? ?? ''
      ..city = json['city'] as String? ?? ''
      ..pricePerGallon = (json['pricePerGallon'] as num?)?.toDouble() ?? 16350
      ..notifications = json['notifications'] as bool? ?? true
      ..reminders = json['reminders'] as bool? ?? true;
    return data;
  }
}

class GarageStore {
  Future<GarageData> load() async {
    final prefs = await SharedPreferences.getInstance();
    final raw = prefs.getString(_dataKey);
    if (raw == null) return GarageData();
    return GarageData.fromJson(jsonDecode(raw) as Map<String, dynamic>);
  }

  Future<void> save(GarageData data) async {
    final prefs = await SharedPreferences.getInstance();
    final saved = await prefs.setString(_dataKey, jsonEncode(data.toJson()));
    if (!saved) throw StateError('El teléfono no pudo guardar los cambios.');
  }
}

class GarageApp extends StatefulWidget {
  const GarageApp({super.key});

  @override
  State<GarageApp> createState() => _GarageAppState();
}

class _GarageAppState extends State<GarageApp> {
  final _store = GarageStore();
  final _maintenanceVoice = MaintenanceVoiceController();
  final _navigatorKey = GlobalKey<NavigatorState>();
  GarageData _data = GarageData();
  bool _loading = true;
  String? _loadError;
  int _tab = 0;
  String? _selectedVehicleId;

  static const _tabs = [
    (Icons.home_rounded, 'Inicio'),
    (Icons.two_wheeler_rounded, 'Garaje'),
    (Icons.build_rounded, 'Servicio'),
    (Icons.bar_chart_rounded, 'Gastos'),
    (Icons.person_rounded, 'Perfil'),
  ];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final data = await _store.load();
      if (!mounted) return;
      setState(() {
        _data = data;
        _selectedVehicleId =
            data.vehicles.isEmpty ? null : data.vehicles.first.id;
        _loading = false;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _loadError = 'No se pudieron leer los datos guardados: $error';
        _loading = false;
      });
    }
  }

  Vehicle? get _vehicle {
    for (final item in _data.vehicles) {
      if (item.id == _selectedVehicleId) return item;
    }
    return _data.vehicles.isEmpty ? null : _data.vehicles.first;
  }

  BuildContext get _navigatorContext {
    final navigatorContext = _navigatorKey.currentContext;
    if (navigatorContext == null) {
      throw StateError('La navegación todavía no está disponible.');
    }
    return navigatorContext;
  }

  Future<T?> _showDialog<T>({required WidgetBuilder builder}) async {
    final navigator = _navigatorKey.currentState;
    if (navigator == null) {
      throw StateError('La navegación todavía no está disponible.');
    }
    final route = DialogRoute<T>(
      context: navigator.context,
      builder: builder,
      barrierDismissible: true,
    );
    final result = await navigator.push<T>(route);
    await route.completed;
    return result;
  }

  Future<void> _save() async {
    try {
      await _store.save(_data);
    } catch (error) {
      if (mounted) _message('Error al guardar: $error', isError: true);
    }
    if (mounted) setState(() {});
  }

  void _message(String text, {bool isError = false}) {
    ScaffoldMessenger.of(_navigatorContext)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(
        content: Text(text),
        backgroundColor: isError ? _red : _surface2,
      ));
  }

  @override
  Widget build(BuildContext context) {
    final theme = ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      scaffoldBackgroundColor: _background,
      colorScheme: ColorScheme.fromSeed(
        seedColor: _red,
        brightness: Brightness.dark,
        surface: _surface,
      ),
      fontFamily: 'Roboto',
      textTheme: const TextTheme(
        headlineMedium: TextStyle(fontWeight: FontWeight.w800, letterSpacing: -0.8),
        titleLarge: TextStyle(fontWeight: FontWeight.w700),
        titleMedium: TextStyle(fontWeight: FontWeight.w600),
        bodyMedium: TextStyle(color: Color(0xFFD8DCE2)),
        labelSmall: TextStyle(color: _muted, fontWeight: FontWeight.w600),
      ),
      snackBarTheme: const SnackBarThemeData(behavior: SnackBarBehavior.floating),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: _surface2,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(14),
          borderSide: BorderSide.none,
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(14),
          borderSide: const BorderSide(color: _red),
        ),
      ),
    );

    return MaterialApp(
      navigatorKey: _navigatorKey,
      title: 'Mi Garaje',
      debugShowCheckedModeBanner: false,
      theme: theme,
      home: _loading
          ? const Scaffold(
              body: Center(child: CircularProgressIndicator(color: _red)),
            )
          : _loadError != null
              ? Scaffold(
                  body: Center(
                    child: Padding(
                      padding: const EdgeInsets.all(24),
                      child: Text(_loadError!, textAlign: TextAlign.center),
                    ),
                  ),
                )
              : Scaffold(
                  body: SafeArea(
                    bottom: false,
                    child: IndexedStack(
                      index: _tab,
                      children: [
                        _home(),
                        _garage(),
                        _maintenance(),
                        _statistics(),
                        _profile(),
                      ],
                    ),
                  ),
                  bottomNavigationBar: NavigationBar(
                    selectedIndex: _tab,
                    onDestinationSelected: (index) => setState(() => _tab = index),
                    backgroundColor: _surface,
                    indicatorColor: _red.withValues(alpha: 0.18),
                    destinations: [
                      for (final item in _tabs)
                        NavigationDestination(
                          icon: Icon(item.$1),
                          label: item.$2,
                        ),
                    ],
                  ),
                ),
    );
  }

  Widget _home() {
    final vehicle = _vehicle;
    final currentMonth = DateTime.now();
    final monthSpend = _data.fuel
        .where((e) {
          final d = DateTime.tryParse(e.date);
          return d != null &&
              d.year == currentMonth.year &&
              d.month == currentMonth.month;
        })
        .fold<double>(0, (sum, e) => sum + e.amount);
    final services = _data.services
        .where((e) => e.vehicleId == vehicle?.id)
        .toList()
      ..sort((a, b) => a.remaining(vehicle?.km ?? 0)
          .compareTo(b.remaining(vehicle?.km ?? 0)));
    final activity = _data.fuel
        .where((e) => e.vehicleId == vehicle?.id)
        .toList()
      ..sort((a, b) => b.date.compareTo(a.date));

    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(18, 18, 18, 28),
      children: [
        _greeting(),
        const SizedBox(height: 26),
        if (vehicle == null)
          _emptyVehicleCard()
        else ...[
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(vehicle.title, style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w800)),
                    const SizedBox(height: 5),
                    Text(
                      '${vehicle.year} · ${vehicle.cc} cc · ${vehicle.plate.isEmpty ? 'Sin placa' : vehicle.plate} · ${vehicle.km} km',
                      style: const TextStyle(color: _muted, fontSize: 12),
                    ),
                  ],
                ),
              ),
              _pill(vehicle.km == 0 ? 'Nueva' : 'Activa', vehicle.km == 0 ? _red : _green),
            ],
          ),
          const SizedBox(height: 10),
          _hero(vehicle),
          const SizedBox(height: 12),
          Row(
            children: [
              _metric('Próximo servicio', services.isEmpty ? '—' : '${math.max(0, services.first.remaining(vehicle.km))} km'),
              _metric('Gasto del mes', _money(monthSpend)),
              _metric('Consumo', _consumption(vehicle)),
            ],
          ),
          const SizedBox(height: 24),
          _sectionTitle('Accesos rápidos'),
          const SizedBox(height: 12),
          Row(
            children: [
              _quickAction(Icons.local_gas_station_rounded, 'Combustible', _addFuel),
              _quickAction(Icons.build_rounded, 'Servicio', _addService),
              _quickAction(Icons.bar_chart_rounded, 'Gastos', () => setState(() => _tab = 3)),
              _quickAction(Icons.description_rounded, 'Documentos', _addDocument),
            ],
          ),
          const SizedBox(height: 24),
          _sectionTitle('Mantenimiento', action: 'Ver todo', onAction: () => setState(() => _tab = 2)),
          const SizedBox(height: 10),
          if (services.isEmpty)
            _emptyCard('Aún no has agregado servicios')
          else
            ...services.take(2).map((service) => Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: _serviceTile(service, vehicle),
                )),
          const SizedBox(height: 18),
          _sectionTitle('Actividad reciente'),
          const SizedBox(height: 10),
          if (activity.isEmpty)
            _emptyCard('Sin movimientos todavía')
          else
            ...activity.take(3).map((entry) => Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: _activityTile(entry),
                )),
        ],
      ],
    );
  }

  Widget _greeting() {
    final now = DateTime.now();
    final hello = now.hour < 12
        ? 'Buenos días'
        : now.hour < 19
            ? 'Buenas tardes'
            : 'Buenas noches';
    final name = _data.owner.trim().isEmpty ? 'Tu garaje' : _data.owner.trim();
    return Row(
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('$hello,', style: const TextStyle(color: _muted)),
              Text(name, style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
              const SizedBox(height: 3),
              Text(_dateLabel(_today()), style: const TextStyle(color: _muted, fontSize: 12)),
            ],
          ),
        ),
        IconButton.filledTonal(
          onPressed: () => setState(() => _tab = 4),
          icon: const Icon(Icons.person_outline_rounded),
        ),
      ],
    );
  }

  Widget _hero(Vehicle vehicle) => SizedBox(
        height: 282,
        child: Stack(
          alignment: Alignment.center,
          children: [
            Positioned.fill(
              child: DecoratedBox(
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(28),
                  gradient: RadialGradient(
                    center: const Alignment(0, 0.25),
                    radius: 0.85,
                    colors: [
                      _red.withValues(alpha: 0.22),
                      const Color(0xFF171018).withValues(alpha: 0.20),
                      Colors.transparent,
                    ],
                  ),
                ),
              ),
            ),
            Positioned(
              width: 238,
              height: 238,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  border: Border.all(color: _red.withValues(alpha: 0.24)),
                  color: _red.withValues(alpha: 0.045),
                ),
              ),
            ),
            Positioned(
              width: 192,
              height: 192,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  border: Border.all(color: Colors.white.withValues(alpha: 0.08)),
                ),
              ),
            ),
            Positioned.fill(
              child: CustomPaint(painter: _HeroGridPainter()),
            ),
            Image.asset(
              'assets/images/mi_moto.png',
              fit: BoxFit.contain,
              errorBuilder: (_, _, _) => const Icon(
                Icons.two_wheeler_rounded,
                size: 150,
                color: _muted,
              ),
            ),
            Positioned(
              bottom: 4,
              child: Text(
                'TU MOTO, SIEMPRE A PUNTO',
                style: TextStyle(
                  color: Colors.white.withValues(alpha: 0.56),
                  fontSize: 10,
                  letterSpacing: 2,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ],
        ),
      );

  Widget _emptyVehicleCard() => _Panel(
        padding: const EdgeInsets.all(22),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Icon(Icons.two_wheeler_rounded, color: _red, size: 34),
            const SizedBox(height: 12),
            const Text('Tu garaje empieza aquí', style: TextStyle(fontSize: 19, fontWeight: FontWeight.w800)),
            const SizedBox(height: 6),
            const Text('Agrega tu moto para registrar combustible, servicios y documentos.', style: TextStyle(color: _muted)),
            const SizedBox(height: 18),
            FilledButton.icon(
              onPressed: _addVehicle,
              icon: const Icon(Icons.two_wheeler_rounded),
              label: const Text('Agregar mi primera moto'),
            ),
          ],
        ),
      );

  Widget _metric(String title, String value) => Expanded(
        child: _Panel(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 13),
          child: Column(
            children: [
              Text(title, textAlign: TextAlign.center, style: const TextStyle(color: _muted, fontSize: 10)),
              const SizedBox(height: 7),
              FittedBox(
                fit: BoxFit.scaleDown,
                child: Text(value, style: const TextStyle(fontWeight: FontWeight.w800)),
              ),
            ],
          ),
        ),
      );

  Widget _quickAction(IconData icon, String label, VoidCallback action) => Expanded(
        child: InkWell(
          onTap: action,
          borderRadius: BorderRadius.circular(16),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Column(
              children: [
                CircleAvatar(
                  radius: 25,
                  backgroundColor: _surface2,
                  child: Icon(icon, color: label == 'Combustible' ? _red : Colors.white),
                ),
                const SizedBox(height: 7),
                FittedBox(
                  fit: BoxFit.scaleDown,
                  child: Text(label, style: const TextStyle(color: _muted, fontSize: 11)),
                ),
              ],
            ),
          ),
        ),
      );

  Widget _garage() {
    final vehicle = _vehicle;
    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(18, 20, 18, 30),
      children: [
        _pageHeader('Mi garaje', '${_data.vehicles.length} ${_data.vehicles.length == 1 ? 'vehículo' : 'vehículos'}'),
        const SizedBox(height: 18),
        FilledButton.icon(
          onPressed: _addVehicle,
          icon: const Icon(Icons.add),
          label: const Text('Agregar vehículo'),
        ),
        const SizedBox(height: 16),
        if (_data.vehicles.isEmpty)
          _emptyVehicleCard()
        else
          ..._data.vehicles.map((item) => Padding(
                padding: const EdgeInsets.only(bottom: 12),
                child: _Panel(
                  child: InkWell(
                    borderRadius: BorderRadius.circular(20),
                    onTap: () => setState(() => _selectedVehicleId = item.id),
                    child: Row(
                      children: [
                        Container(
                          width: 78,
                          height: 84,
                          decoration: BoxDecoration(
                            borderRadius: BorderRadius.circular(16),
                            color: _surface2,
                          ),
                          child: Image.asset('assets/images/mi_moto.png', fit: BoxFit.contain),
                        ),
                        const SizedBox(width: 14),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(item.brand.toUpperCase(), style: const TextStyle(color: _muted, fontSize: 10, fontWeight: FontWeight.w700)),
                              Text(item.model, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w800)),
                              Text('${item.year} · ${item.cc} cc · ${item.km} km', style: const TextStyle(color: _muted, fontSize: 12)),
                              if (item.plate.isNotEmpty)
                                Text('Placa ${item.plate}', style: const TextStyle(color: _muted, fontSize: 12)),
                            ],
                          ),
                        ),
                        PopupMenuButton<String>(
                          onSelected: (value) {
                            if (value == 'edit') _editVehicle(item);
                            if (value == 'delete') _deleteVehicle(item);
                          },
                          itemBuilder: (_) => const [
                            PopupMenuItem(value: 'edit', child: Text('Editar')),
                            PopupMenuItem(value: 'delete', child: Text('Eliminar')),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
              )),
        if (vehicle != null) ...[
          const SizedBox(height: 8),
          _sectionTitle('Documentación'),
          const SizedBox(height: 10),
          ..._data.documents.where((d) => d.vehicleId == vehicle.id).map(
                (document) => Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: _documentTile(document),
                ),
              ),
          OutlinedButton.icon(
            onPressed: _addDocument,
            icon: const Icon(Icons.description_outlined),
            label: const Text('Agregar documento'),
          ),
        ],
      ],
    );
  }

  Widget _maintenance() {
    final vehicle = _vehicle;
    final services = _data.services.where((e) => e.vehicleId == vehicle?.id).toList();
    final docs = _data.documents.where((e) => e.vehicleId == vehicle?.id).toList();
    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(18, 20, 18, 30),
      children: [
        _pageHeader('Mantenimiento', 'Cuida tu moto, kilómetro a kilómetro'),
        const SizedBox(height: 18),
        if (vehicle == null)
          _emptyVehicleCard()
        else ...[
          _Panel(
            child: Row(
              children: [
                const CircleAvatar(backgroundColor: Color(0x223FB950), child: Icon(Icons.speed_rounded, color: _green)),
                const SizedBox(width: 12),
                Expanded(child: Text('${vehicle.km} km registrados', style: const TextStyle(fontWeight: FontWeight.w700))),
                _pill('${services.length} servicios', _red),
              ],
            ),
          ),
          const SizedBox(height: 18),
          _sectionTitle('Plan de mantenimiento'),
          const SizedBox(height: 10),
          if (services.isEmpty)
            _emptyCard('Todavía no tienes servicios programados')
          else
            ...services.map((service) => Padding(
                  padding: const EdgeInsets.only(bottom: 9),
                  child: _serviceTile(service, vehicle, actionable: true),
                )),
          const SizedBox(height: 8),
          FilledButton.icon(
            onPressed: _addService,
            icon: const Icon(Icons.add),
            label: const Text('Registrar mantenimiento'),
          ),
          const SizedBox(height: 24),
          _sectionTitle('Documentos y vencimientos'),
          const SizedBox(height: 10),
          if (docs.isEmpty)
            _emptyCard('Agrega SOAT, tecnomecánica u otro documento')
          else
            ...docs.map((d) => Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: _documentTile(d),
                )),
          OutlinedButton.icon(
            onPressed: _addDocument,
            icon: const Icon(Icons.add),
            label: const Text('Agregar documento'),
          ),
        ],
      ],
    );
  }

  Widget _statistics() {
    final now = DateTime.now();
    final monthly = List<double>.generate(6, (index) {
      final month = DateTime(now.year, now.month - 5 + index);
      return _data.fuel.where((entry) {
        final date = DateTime.tryParse(entry.date);
        return date != null && date.year == month.year && date.month == month.month;
      }).fold<double>(0, (sum, entry) => sum + entry.amount);
    });
    final maxValue = monthly.fold<double>(1, math.max);
    final monthTotal = monthly.last;
    final vehicle = _vehicle;
    final fuelCount = _data.fuel.where((e) => e.vehicleId == vehicle?.id).length;
    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(18, 20, 18, 30),
      children: [
        _pageHeader('Estadísticas', 'Resumen de tus gastos'),
        const SizedBox(height: 18),
        _Panel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Gasto de este mes', style: TextStyle(color: _muted)),
              const SizedBox(height: 8),
              Text(_money(monthTotal), style: const TextStyle(fontSize: 30, fontWeight: FontWeight.w800)),
              const SizedBox(height: 20),
              SizedBox(
                height: 150,
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    for (var i = 0; i < monthly.length; i++)
                      Expanded(
                        child: Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 5),
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.end,
                            children: [
                              Expanded(
                                child: Align(
                                  alignment: Alignment.bottomCenter,
                                  child: FractionallySizedBox(
                                    heightFactor: (monthly[i] / maxValue).clamp(0.06, 1),
                                    child: Container(
                                      decoration: BoxDecoration(
                                        borderRadius: BorderRadius.circular(8),
                                        gradient: LinearGradient(
                                          begin: Alignment.topCenter,
                                          end: Alignment.bottomCenter,
                                          colors: [_red, _red.withValues(alpha: 0.34)],
                                        ),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                              const SizedBox(height: 8),
                              Text(_monthName(DateTime(now.year, now.month - 5 + i).month), style: const TextStyle(color: _muted, fontSize: 10)),
                            ],
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 14),
        Row(
          children: [
            _metric('Cargas este mes', '${_data.fuel.where((e) {
              final d = DateTime.tryParse(e.date);
              return d != null && d.year == now.year && d.month == now.month;
            }).length}',),
            _metric('Registros', '$fuelCount'),
          ],
        ),
        const SizedBox(height: 14),
        _Panel(
          child: Row(
            children: [
              const Icon(Icons.local_gas_station_rounded, color: _red),
              const SizedBox(width: 12),
              Expanded(child: Text('Precio configurado por galón: ${_money(_data.pricePerGallon)}')),
            ],
          ),
        ),
      ],
    );
  }

  Widget _profile() => ListView(
        physics: const BouncingScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(18, 20, 18, 30),
        children: [
          _pageHeader('Perfil', 'Tu cuenta y preferencias'),
          const SizedBox(height: 18),
          _Panel(
            child: Row(
              children: [
                CircleAvatar(
                  radius: 29,
                  backgroundColor: _surface2,
                  child: Text(
                    _data.owner.trim().isEmpty
                        ? 'MG'
                        : _data.owner.trim().split(RegExp(r'\s+')).take(2).map((e) => e[0]).join().toUpperCase(),
                    style: const TextStyle(fontWeight: FontWeight.w800),
                  ),
                ),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(_data.owner.isEmpty ? 'Motociclista' : _data.owner, style: const TextStyle(fontWeight: FontWeight.w800, fontSize: 17)),
                      Text(_data.city.isEmpty ? 'Configura tu ciudad' : _data.city, style: const TextStyle(color: _muted)),
                    ],
                  ),
                ),
                IconButton(onPressed: _editProfile, icon: const Icon(Icons.edit_outlined)),
              ],
            ),
          ),
          const SizedBox(height: 22),
          _sectionTitle('Preferencias'),
          const SizedBox(height: 8),
          _Panel(
            padding: EdgeInsets.zero,
            child: Column(
              children: [
                SwitchListTile(
                  value: _data.notifications,
                  onChanged: (v) async {
                    _data.notifications = v;
                    await _save();
                  },
                  title: const Text('Notificaciones'),
                  activeThumbColor: _red,
                ),
                const Divider(height: 1, indent: 16, endIndent: 16, color: _line),
                SwitchListTile(
                  value: _data.reminders,
                  onChanged: (v) async {
                    _data.reminders = v;
                    await _save();
                  },
                  title: const Text('Recordatorios de mantenimiento'),
                  activeThumbColor: _red,
                ),
                const Divider(height: 1, indent: 16, endIndent: 16, color: _line),
                ListTile(
                  title: const Text('Precio del galón'),
                  subtitle: Text(_money(_data.pricePerGallon)),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: _editPrice,
                ),
              ],
            ),
          ),
          const SizedBox(height: 22),
          _Panel(
            child: const Row(
              children: [
                Icon(Icons.lock_outline_rounded, color: _green),
                SizedBox(width: 12),
                Expanded(child: Text('Tus datos se guardan localmente en este dispositivo.')),
              ],
            ),
          ),
        ],
      );

  Widget _pageHeader(String title, String subtitle) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: Theme.of(context).textTheme.headlineMedium),
          const SizedBox(height: 4),
          Text(subtitle, style: const TextStyle(color: _muted)),
        ],
      );

  Widget _sectionTitle(String title, {String? action, VoidCallback? onAction}) => Row(
        children: [
          Expanded(child: Text(title.toUpperCase(), style: const TextStyle(color: _muted, fontSize: 11, letterSpacing: 0.8, fontWeight: FontWeight.w800))),
          if (action != null)
            TextButton(
              onPressed: onAction,
              child: Text(action, style: const TextStyle(color: _red)),
            ),
        ],
      );

  Widget _pill(String text, Color color) => Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.16),
          borderRadius: BorderRadius.circular(30),
          border: Border.all(color: color.withValues(alpha: 0.28)),
        ),
        child: Text(text, style: TextStyle(color: color, fontSize: 11, fontWeight: FontWeight.w700)),
      );

  Widget _emptyCard(String text) => _Panel(
        child: Row(
          children: [
            const Icon(Icons.info_outline_rounded, color: _muted),
            const SizedBox(width: 12),
            Expanded(child: Text(text, style: const TextStyle(color: _muted))),
          ],
        ),
      );

  Widget _serviceTile(ServiceEntry service, Vehicle vehicle, {bool actionable = false}) {
    final remaining = service.remaining(vehicle.km);
    final overdue = remaining <= 0;
    final progress = (1 - remaining / math.max(service.intervalKm, 1)).clamp(0.0, 1.0);
    return _Panel(
      child: Row(
        children: [
          CircleAvatar(
            radius: 18,
            backgroundColor: (overdue ? _red : _green).withValues(alpha: 0.14),
            child: Icon(Icons.build_rounded, size: 17, color: overdue ? _red : _green),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(service.name, style: const TextStyle(fontWeight: FontWeight.w700)),
                const SizedBox(height: 4),
                Text(overdue ? 'Vencido · ${service.intervalKm} km' : 'Faltan $remaining km', style: TextStyle(color: overdue ? _red : _muted, fontSize: 12)),
                const SizedBox(height: 8),
                ClipRRect(
                  borderRadius: BorderRadius.circular(10),
                  child: LinearProgressIndicator(
                    value: progress,
                    minHeight: 5,
                    backgroundColor: _line,
                    color: overdue ? _red : _green,
                  ),
                ),
              ],
            ),
          ),
          if (actionable)
            PopupMenuButton<String>(
              onSelected: (value) async {
                if (value == 'done') {
                  service.lastKm = vehicle.km;
                  service.date = _today();
                  await _save();
                  _message('Servicio actualizado');
                } else if (value == 'delete') {
                  _data.services.removeWhere((e) => e.id == service.id);
                  await _save();
                }
              },
              itemBuilder: (_) => const [
                PopupMenuItem(value: 'done', child: Text('Marcar realizado')),
                PopupMenuItem(value: 'delete', child: Text('Eliminar')),
              ],
            ),
        ],
      ),
    );
  }

  Widget _documentTile(VehicleDocument document) {
    final expiry = DateTime.tryParse(document.expiry);
    final expired = expiry != null && expiry.isBefore(DateTime.now());
    return _Panel(
      child: Row(
        children: [
          CircleAvatar(
            backgroundColor: (expired ? _red : _green).withValues(alpha: 0.15),
            child: Icon(Icons.verified_user_outlined, color: expired ? _red : _green, size: 19),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(document.name, style: const TextStyle(fontWeight: FontWeight.w700)),
                Text('Vence ${_dateLabel(document.expiry)}', style: const TextStyle(color: _muted, fontSize: 12)),
              ],
            ),
          ),
          Text(expired ? 'Vencido' : 'Vigente', style: TextStyle(color: expired ? _red : _green, fontSize: 11, fontWeight: FontWeight.w700)),
          PopupMenuButton<String>(
            onSelected: (value) async {
              if (value == 'delete') {
                _data.documents.removeWhere((e) => e.id == document.id);
                await _save();
              }
            },
            itemBuilder: (_) => const [PopupMenuItem(value: 'delete', child: Text('Eliminar'))],
          ),
        ],
      ),
    );
  }

  Widget _activityTile(FuelEntry entry) => _Panel(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            const Icon(Icons.local_gas_station_rounded, color: _red, size: 19),
            const SizedBox(width: 10),
            Expanded(child: Text(entry.station.isEmpty ? 'Carga de combustible' : entry.station)),
            Text(_money(entry.amount), style: const TextStyle(fontWeight: FontWeight.w700)),
          ],
        ),
      );

  String _consumption(Vehicle vehicle) {
    final entries = _data.fuel.where((e) => e.vehicleId == vehicle.id).toList()
      ..sort((a, b) => a.km.compareTo(b.km));
    if (entries.length < 2) return '—';
    final distance = entries.last.km - entries.first.km;
    final gallons = entries.skip(1).fold<double>(
      0,
      (sum, e) => sum + (e.pricePerGallon <= 0 ? 0 : e.amount / e.pricePerGallon),
    );
    if (distance <= 0 || gallons <= 0) return '—';
    return '${(distance / gallons).round()} km/gal';
  }

  String _monthName(int month) => const [
        'ENE', 'FEB', 'MAR', 'ABR', 'MAY', 'JUN',
        'JUL', 'AGO', 'SEP', 'OCT', 'NOV', 'DIC',
      ][month - 1];

  Future<void> _addVehicle() async {
    final form = GlobalKey<FormState>();
    final brand = TextEditingController();
    final model = TextEditingController();
    final year = TextEditingController(text: '${DateTime.now().year}');
    final cc = TextEditingController(text: '150');
    final km = TextEditingController(text: '0');
    final plate = TextEditingController();
    final saved = await _showDialog<Vehicle>(
      builder: (context) => _formDialog(
        title: 'Agregar vehículo',
        formKey: form,
        fields: [
          _field(brand, 'Marca', required: true),
          _field(model, 'Modelo', required: true),
          Row(children: [
            Expanded(child: _field(year, 'Año', numeric: true, required: true)),
            const SizedBox(width: 10),
            Expanded(child: _field(cc, 'Cilindraje (cc)', numeric: true, required: true)),
          ]),
          _field(km, 'Kilometraje actual', numeric: true, required: true),
          _field(plate, 'Placa (opcional)'),
        ],
        onSubmit: () {
          if (!form.currentState!.validate()) return;
          final y = int.parse(year.text);
          final c = int.parse(cc.text);
          final k = int.parse(km.text);
          if (y < 1950 || y > DateTime.now().year || c <= 0 || k < 0) {
            _message('Revisa año, cilindraje y kilometraje.', isError: true);
            return;
          }
          Navigator.pop(
            context,
            Vehicle(
              brand: brand.text.trim(),
              model: model.text.trim(),
              year: y,
              cc: c,
              km: k,
              plate: plate.text.trim().toUpperCase(),
            ),
          );
        },
      ),
    );
    for (final controller in [brand, model, year, cc, km, plate]) {
      controller.dispose();
    }
    if (saved == null) return;
    _data.vehicles.add(saved);
    _selectedVehicleId = saved.id;
    await _save();
  }

  Future<void> _editVehicle(Vehicle vehicle) async {
    final form = GlobalKey<FormState>();
    final brand = TextEditingController(text: vehicle.brand);
    final model = TextEditingController(text: vehicle.model);
    final year = TextEditingController(text: '${vehicle.year}');
    final cc = TextEditingController(text: '${vehicle.cc}');
    final km = TextEditingController(text: '${vehicle.km}');
    final plate = TextEditingController(text: vehicle.plate);
    final saved = await _showDialog<bool>(
      builder: (context) => _formDialog(
        title: 'Editar vehículo',
        formKey: form,
        fields: [
          _field(brand, 'Marca', required: true),
          _field(model, 'Modelo', required: true),
          Row(children: [
            Expanded(child: _field(year, 'Año', numeric: true, required: true)),
            const SizedBox(width: 10),
            Expanded(child: _field(cc, 'Cilindraje (cc)', numeric: true, required: true)),
          ]),
          _field(km, 'Kilometraje actual', numeric: true, required: true),
          _field(plate, 'Placa'),
        ],
        onSubmit: () {
          if (!form.currentState!.validate()) return;
          final y = int.parse(year.text);
          final c = int.parse(cc.text);
          final k = int.parse(km.text);
          if (y < 1950 || y > DateTime.now().year || c <= 0 || k < 0) {
            _message('Revisa año, cilindraje y kilometraje.', isError: true);
            return;
          }
          vehicle
            ..brand = brand.text.trim()
            ..model = model.text.trim()
            ..year = y
            ..cc = c
            ..km = k
            ..plate = plate.text.trim().toUpperCase();
          Navigator.pop(context, true);
        },
      ),
    );
    for (final controller in [brand, model, year, cc, km, plate]) {
      controller.dispose();
    }
    if (saved == true) await _save();
  }

  Future<void> _deleteVehicle(Vehicle vehicle) async {
    final ok = await _confirm('¿Eliminar ${vehicle.title}?', 'También se eliminarán sus cargas, servicios y documentos.');
    if (!ok) return;
    _data.vehicles.removeWhere((e) => e.id == vehicle.id);
    _data.fuel.removeWhere((e) => e.vehicleId == vehicle.id);
    _data.services.removeWhere((e) => e.vehicleId == vehicle.id);
    _data.documents.removeWhere((e) => e.vehicleId == vehicle.id);
    if (_selectedVehicleId == vehicle.id) {
      _selectedVehicleId = _data.vehicles.isEmpty ? null : _data.vehicles.first.id;
    }
    await _save();
  }

  Future<void> _addFuel() async {
    final vehicle = _vehicle;
    if (vehicle == null) {
      _message('Agrega primero una moto.');
      return;
    }
    final form = GlobalKey<FormState>();
    final amount = TextEditingController();
    final price = TextEditingController(text: _data.pricePerGallon.round().toString());
    final km = TextEditingController(text: vehicle.km.toString());
    final station = TextEditingController();
    final saved = await _showDialog<FuelEntry>(
      builder: (context) => _formDialog(
        title: 'Registrar combustible',
        formKey: form,
        fields: [
          _field(amount, 'Total pagado (COP)', numeric: true, required: true),
          _field(price, 'Precio por galón', numeric: true, required: true),
          _field(km, 'Kilometraje del tablero', numeric: true, required: true),
          _field(station, 'Estación (opcional)'),
        ],
        onSubmit: () {
          if (!form.currentState!.validate()) return;
          final value = double.tryParse(amount.text.replaceAll(',', '.')) ?? 0;
          final gallonPrice = double.tryParse(price.text.replaceAll(',', '.')) ?? 0;
          final odometer = int.tryParse(km.text) ?? -1;
          if (value <= 0 || gallonPrice <= 0 || odometer < vehicle.km) {
            _message('El monto y el precio deben ser positivos; el kilometraje no puede retroceder.', isError: true);
            return;
          }
          Navigator.pop(
            context,
            FuelEntry(
              vehicleId: vehicle.id,
              date: _today(),
              station: station.text.trim(),
              amount: value,
              pricePerGallon: gallonPrice,
              km: odometer,
            ),
          );
        },
      ),
    );
    for (final controller in [amount, price, km, station]) {
      controller.dispose();
    }
    if (saved == null) return;
    _data.fuel.add(saved);
    vehicle.km = saved.km;
    _data.pricePerGallon = saved.pricePerGallon;
    await _save();
    _message('Combustible registrado');
  }

  Future<void> _addService() async {
    final vehicle = _vehicle;
    if (vehicle == null) {
      _message('Agrega primero una moto.');
      return;
    }
    final saved = await _showDialog<ServiceEntry>(
      builder: (_) => _MaintenanceServiceDialog(
        vehicle: vehicle,
        voiceController: _maintenanceVoice,
      ),
    );
    if (saved == null) return;
    _data.services.add(saved);
    await _save();
    _message('Mantenimiento registrado');
  }

  Future<void> _addDocument() async {
    final vehicle = _vehicle;
    if (vehicle == null) {
      _message('Agrega primero una moto.');
      return;
    }
    final name = TextEditingController();
    DateTime expiry = DateTime.now().add(const Duration(days: 365));
    final saved = await _showDialog<VehicleDocument>(
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: const Text('Agregar documento'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(controller: name, decoration: const InputDecoration(labelText: 'Nombre (SOAT, tecnomecánica...)')),
              const SizedBox(height: 14),
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.calendar_month_rounded, color: _red),
                title: const Text('Fecha de vencimiento'),
                subtitle: Text(_dateLabel(expiry.toIso8601String())),
                onTap: () async {
                  final date = await showDatePicker(
                    context: context,
                    initialDate: expiry,
                    firstDate: DateTime(2000),
                    lastDate: DateTime(2200),
                  );
                  if (date != null) setDialogState(() => expiry = date);
                },
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () {
                if (name.text.trim().isEmpty) {
                  _message('Escribe el nombre del documento.', isError: true);
                  return;
                }
                Navigator.pop(
                  context,
                  VehicleDocument(
                    vehicleId: vehicle.id,
                    name: name.text.trim(),
                    expiry: '${expiry.year}-${expiry.month.toString().padLeft(2, '0')}-${expiry.day.toString().padLeft(2, '0')}',
                  ),
                );
              },
              child: const Text('Guardar'),
            ),
          ],
        ),
      ),
    );
    name.dispose();
    if (saved == null) return;
    _data.documents.add(saved);
    await _save();
  }

  Future<void> _editProfile() async {
    final form = GlobalKey<FormState>();
    final name = TextEditingController(text: _data.owner);
    final city = TextEditingController(text: _data.city);
    final saved = await _showDialog<bool>(
      builder: (context) => _formDialog(
        title: 'Editar perfil',
        formKey: form,
        fields: [_field(name, 'Tu nombre'), _field(city, 'Ciudad')],
        onSubmit: () {
          _data
            ..owner = name.text.trim()
            ..city = city.text.trim();
          Navigator.pop(context, true);
        },
      ),
    );
    name.dispose();
    city.dispose();
    if (saved == true) await _save();
  }

  Future<void> _editPrice() async {
    final form = GlobalKey<FormState>();
    final price = TextEditingController(text: _data.pricePerGallon.round().toString());
    final saved = await _showDialog<bool>(
      builder: (context) => _formDialog(
        title: 'Precio del galón',
        formKey: form,
        fields: [_field(price, 'Precio en COP', numeric: true, required: true)],
        onSubmit: () {
          final value = double.tryParse(price.text) ?? 0;
          if (!form.currentState!.validate() || value <= 0) return;
          _data.pricePerGallon = value;
          Navigator.pop(context, true);
        },
      ),
    );
    price.dispose();
    if (saved == true) await _save();
  }

  Widget _field(
    TextEditingController controller,
    String label, {
    bool numeric = false,
    bool required = false,
  }) =>
      Padding(
        padding: const EdgeInsets.only(bottom: 12),
        child: TextFormField(
          controller: controller,
          keyboardType: numeric
              ? const TextInputType.numberWithOptions(decimal: true)
              : TextInputType.text,
          textCapitalization: TextCapitalization.sentences,
          decoration: InputDecoration(labelText: label),
          validator: required
              ? (value) => value == null || value.trim().isEmpty ? 'Este campo es obligatorio' : null
              : null,
        ),
      );

  Widget _formDialog({
    required String title,
    required GlobalKey<FormState> formKey,
    required List<Widget> fields,
    required VoidCallback onSubmit,
  }) =>
      AlertDialog(
        title: Text(title),
        content: Form(
          key: formKey,
          child: SingleChildScrollView(
            child: Column(mainAxisSize: MainAxisSize.min, children: fields),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => _navigatorKey.currentState!.pop(),
            child: const Text('Cancelar'),
          ),
          FilledButton(onPressed: onSubmit, child: const Text('Guardar')),
        ],
      );

  Future<bool> _confirm(String title, String body) async =>
      await _showDialog<bool>(
        builder: (context) => AlertDialog(
          title: Text(title),
          content: Text(body),
          actions: [
            TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancelar')),
            FilledButton(
              style: FilledButton.styleFrom(backgroundColor: _red),
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Eliminar'),
            ),
          ],
        ),
      ) ??
      false;
}

enum _VoiceState { idle, listening, processing, ready, error, permissionDenied }

class _MaintenanceServiceDialog extends StatefulWidget {
  const _MaintenanceServiceDialog({
    required this.vehicle,
    required this.voiceController,
  });

  final Vehicle vehicle;
  final MaintenanceVoiceController voiceController;

  @override
  State<_MaintenanceServiceDialog> createState() =>
      _MaintenanceServiceDialogState();
}

class _MaintenanceServiceDialogState extends State<_MaintenanceServiceDialog> {
  final _formKey = GlobalKey<FormState>();
  final _name = TextEditingController();
  final _interval = TextEditingController(text: '3000');
  late final TextEditingController _lastKm = TextEditingController(
    text: '${widget.vehicle.km}',
  );
  final _cost = TextEditingController();
  DateTime _serviceDate = DateTime.now();
  _VoiceState _voiceState = _VoiceState.idle;
  String _voiceMessage =
      'Dicta el trabajo realizado para rellenar el formulario.';
  String _latestTranscript = '';
  String? _formError;
  bool _handledTranscript = false;

  @override
  void dispose() {
    widget.voiceController.clearListeners();
    if (widget.voiceController.isListening) {
      unawaited(widget.voiceController.cancel());
    }
    _name.dispose();
    _interval.dispose();
    _lastKm.dispose();
    _cost.dispose();
    super.dispose();
  }

  Future<void> _toggleDictation() async {
    if (_voiceState == _VoiceState.listening) {
      setState(() {
        _voiceState = _VoiceState.processing;
        _voiceMessage = 'Procesando el dictado...';
      });
      try {
        await widget.voiceController.stop();
        await _finishAfterPause();
      } catch (error) {
        _showVoiceError('No se pudo detener el reconocimiento: $error');
      }
      return;
    }

    _handledTranscript = false;
    _latestTranscript = '';
    setState(() {
      _voiceState = _VoiceState.processing;
      _voiceMessage = 'Solicitando acceso al micrófono...';
      _formError = null;
    });

    try {
      final available = await widget.voiceController.initialize(
        onStatus: _onVoiceStatus,
        onError: _onVoiceError,
      );
      if (!mounted) return;
      if (!available) {
        setState(() {
          _voiceState = _VoiceState.permissionDenied;
          _voiceMessage =
              'No se concedió el permiso o el reconocimiento de voz no está disponible. Revisa los permisos en Ajustes.';
        });
        return;
      }

      final localeId = await widget.voiceController.spanishLocaleId();
      if (!mounted) return;
      if (localeId == null) {
        setState(() {
          _voiceState = _VoiceState.error;
          _voiceMessage =
              'El dispositivo no tiene reconocimiento de voz en español disponible.';
        });
        return;
      }

      setState(() {
        _voiceState = _VoiceState.listening;
        _voiceMessage = 'Escuchando... Pulsa el botón para terminar.';
      });
      await widget.voiceController.listen(
        localeId: localeId,
        onResult: _onVoiceResult,
      );
    } catch (error) {
      _showVoiceError('No se pudo iniciar el reconocimiento de voz: $error');
    }
  }

  void _onVoiceStatus(String status) {
    if (!mounted) return;
    if (status == 'listening') {
      setState(() {
        _voiceState = _VoiceState.listening;
        _voiceMessage = 'Escuchando... Pulsa el botón para terminar.';
      });
    } else if ((status == 'notListening' || status == 'done') &&
        _voiceState == _VoiceState.listening) {
      setState(() {
        _voiceState = _VoiceState.processing;
        _voiceMessage = 'Procesando el dictado...';
      });
      unawaited(_finishAfterPause());
    }
  }

  void _onVoiceResult(String words, bool isFinal) {
    if (!mounted || _handledTranscript) return;
    _latestTranscript = words;
    setState(() {});
    if (isFinal && words.trim().isNotEmpty) {
      _applyTranscript(words);
    }
  }

  Future<void> _finishAfterPause() async {
    await Future<void>.delayed(const Duration(milliseconds: 350));
    if (!mounted || _handledTranscript) return;
    if (_latestTranscript.trim().isEmpty) {
      setState(() {
        _voiceState = _VoiceState.error;
        _voiceMessage = 'No se detectó voz. Intenta dictar de nuevo.';
      });
      return;
    }
    _applyTranscript(_latestTranscript);
  }

  void _applyTranscript(String words) {
    if (_handledTranscript) return;
    _handledTranscript = true;
    final draft = parseMaintenanceVoiceText(words);
    if (draft.description.isNotEmpty) {
      _name.text = draft.description;
    }
    if (draft.kilometers != null) {
      _lastKm.text = '${draft.kilometers}';
    }
    setState(() {
      _serviceDate = draft.date;
      _voiceState = draft.description.isEmpty
          ? _VoiceState.error
          : _VoiceState.ready;
      _voiceMessage = draft.description.isEmpty
          ? 'No se identificó el trabajo realizado. Escríbelo antes de guardar.'
          : 'Dictado interpretado. Revisa y corrige los campos antes de guardar.';
    });
  }

  void _onVoiceError(String error) {
    final normalized = error.toLowerCase();
    final isPermissionError = normalized.contains('permission') ||
        normalized.contains('denied') ||
        normalized.contains('not_authorized') ||
        normalized.contains('insufficient');
    final message = isPermissionError
        ? 'Permiso de micrófono o reconocimiento denegado. Actívalo en Ajustes para dictar.'
        : normalized.contains('no_match')
            ? 'No se entendió el dictado. Intenta de nuevo.'
            : 'Error de reconocimiento de voz: $error';
    _showVoiceError(message, permissionDenied: isPermissionError);
  }

  void _showVoiceError(String message, {bool permissionDenied = false}) {
    if (!mounted) return;
    setState(() {
      _voiceState =
          permissionDenied ? _VoiceState.permissionDenied : _VoiceState.error;
      _voiceMessage = message;
    });
  }

  Future<void> _pickDate() async {
    final selected = await showDatePicker(
      context: context,
      initialDate: _serviceDate,
      firstDate: DateTime(2000),
      lastDate: DateTime(2200),
    );
    if (selected != null) setState(() => _serviceDate = selected);
  }

  void _save() {
    if (!_formKey.currentState!.validate()) return;
    final every = int.tryParse(_interval.text) ?? 0;
    final last = int.tryParse(_lastKm.text) ?? -1;
    if (every <= 0 || last < 0 || last > widget.vehicle.km) {
      setState(() {
        _formError =
            'El intervalo debe ser positivo y el kilometraje no puede superar los ${widget.vehicle.km} km registrados en la moto.';
      });
      return;
    }
    Navigator.of(context).pop(
      ServiceEntry(
        vehicleId: widget.vehicle.id,
        name: _name.text.trim(),
        intervalKm: every,
        lastKm: last,
        cost: double.tryParse(_cost.text.replaceAll(',', '.')) ?? 0,
        date: _dateIso(_serviceDate),
      ),
    );
  }

  String _dateIso(DateTime date) =>
      '${date.year}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}';

  Widget _textField(
    TextEditingController controller,
    String label, {
    bool numeric = false,
    bool required = false,
  }) =>
      Padding(
        padding: const EdgeInsets.only(bottom: 12),
        child: TextFormField(
          controller: controller,
          keyboardType: numeric
              ? const TextInputType.numberWithOptions(decimal: true)
              : TextInputType.text,
          textCapitalization: TextCapitalization.sentences,
          decoration: InputDecoration(labelText: label),
          validator: required
              ? (value) => value == null || value.trim().isEmpty
                  ? 'Este campo es obligatorio'
                  : null
              : null,
        ),
      );

  Color get _voiceColor {
    switch (_voiceState) {
      case _VoiceState.listening:
      case _VoiceState.permissionDenied:
      case _VoiceState.error:
        return _red;
      case _VoiceState.processing:
        return _muted;
      case _VoiceState.ready:
        return _green;
      case _VoiceState.idle:
        return _muted;
    }
  }

  IconData get _voiceIcon => switch (_voiceState) {
        _VoiceState.listening => Icons.stop_rounded,
        _VoiceState.processing => Icons.hourglass_top_rounded,
        _VoiceState.ready => Icons.check_circle_outline_rounded,
        _VoiceState.permissionDenied => Icons.mic_off_rounded,
        _VoiceState.error => Icons.error_outline_rounded,
        _VoiceState.idle => Icons.mic_none_rounded,
      };

  String get _voiceButtonLabel => switch (_voiceState) {
        _VoiceState.listening => 'Detener dictado',
        _VoiceState.processing => 'Procesando dictado',
        _VoiceState.ready => 'Volver a dictar',
        _VoiceState.permissionDenied => 'Reintentar dictado',
        _VoiceState.error => 'Reintentar dictado',
        _VoiceState.idle => 'Dictar mantenimiento',
      };

  @override
  Widget build(BuildContext context) => AlertDialog(
        title: const Text('Registrar mantenimiento'),
        content: SizedBox(
          width: double.maxFinite,
          child: Form(
            key: _formKey,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  _voicePanel(),
                  const SizedBox(height: 14),
                  _textField(_name, 'Trabajo realizado', required: true),
                  _textField(
                    _interval,
                    'Intervalo de mantenimiento (km)',
                    numeric: true,
                    required: true,
                  ),
                  _textField(
                    _lastKm,
                    'Kilometraje del servicio',
                    numeric: true,
                    required: true,
                  ),
                  ListTile(
                    contentPadding: EdgeInsets.zero,
                    leading:
                        const Icon(Icons.calendar_month_rounded, color: _red),
                    title: const Text('Fecha del mantenimiento'),
                    subtitle: Text(_dateLabel(_serviceDate.toIso8601String())),
                    onTap: _pickDate,
                  ),
                  _textField(_cost, 'Costo del servicio (opcional)',
                      numeric: true),
                  if (_formError != null)
                    Align(
                      alignment: Alignment.centerLeft,
                      child: Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: Text(
                          _formError!,
                          style: const TextStyle(color: _red, fontSize: 12),
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancelar'),
          ),
          FilledButton(onPressed: _save, child: const Text('Guardar')),
        ],
      );

  Widget _voicePanel() => Container(
        width: double.infinity,
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: _surface2,
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: _line),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    'Registro por voz',
                    style: TextStyle(
                      color: _voiceColor,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                IconButton.filled(
                  tooltip: _voiceButtonLabel,
                  onPressed: _voiceState == _VoiceState.processing
                      ? null
                      : _toggleDictation,
                  style: IconButton.styleFrom(
                    backgroundColor: _voiceState == _VoiceState.listening
                        ? _red
                        : _line,
                    foregroundColor: Colors.white,
                  ),
                  icon: Icon(_voiceIcon),
                ),
              ],
            ),
            if (_voiceState == _VoiceState.processing)
              const Padding(
                padding: EdgeInsets.only(bottom: 8),
                child: LinearProgressIndicator(minHeight: 3),
              ),
            Text(
              _voiceMessage,
              style: TextStyle(color: _voiceColor, fontSize: 12),
            ),
            if (_latestTranscript.isNotEmpty) ...[
              const SizedBox(height: 6),
              Text(
                'Escuchado: “$_latestTranscript”',
                style: const TextStyle(color: _muted, fontSize: 12),
              ),
            ],
          ],
        ),
      );
}

class _Panel extends StatelessWidget {
  const _Panel({required this.child, this.padding = const EdgeInsets.all(15)});

  final Widget child;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) => Material(
        color: _surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(18),
          side: BorderSide(color: _line.withValues(alpha: 0.76)),
        ),
        clipBehavior: Clip.antiAlias,
        child: Padding(padding: padding, child: child),
      );
}

class _HeroGridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white.withValues(alpha: 0.045)
      ..strokeWidth = 0.7;
    for (var i = 0; i < 6; i++) {
      final x = size.width * (0.08 + i * 0.16);
      canvas.drawLine(Offset(x, size.height), Offset(size.width * (0.43 + i * 0.035), 0), paint);
    }
    for (var i = 1; i < 5; i++) {
      final y = size.height * i / 5;
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
