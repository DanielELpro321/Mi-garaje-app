import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;
import 'dart:ui';

import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:image/image.dart' as img;
import 'package:image_picker/image_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'maintenance_voice.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(
      statusBarColor: Colors.transparent,
      statusBarIconBrightness: Brightness.light,
      statusBarBrightness: Brightness.dark,
      systemNavigationBarColor: Colors.transparent,
      systemNavigationBarIconBrightness: Brightness.light,
      systemNavigationBarContrastEnforced: false,
    ),
  );
  runApp(const GarageApp());
}

const _background = Color(0xFF07090D);
const _surface = Color(0xFF12161D);
const _surface2 = Color(0xFF1A2029);
const _line = Color(0xFF2A3340);
const _red = Color(0xFFE53935);
const _redGlow = Color(0xFFFF6B61);
const _muted = Color(0xFF98A1AE);
const _green = Color(0xFF43B65A);
const _dataKey = 'mi_garaje_data_v1';
const _systemOverlay = SystemUiOverlayStyle(
  statusBarColor: Colors.transparent,
  statusBarIconBrightness: Brightness.light,
  statusBarBrightness: Brightness.dark,
  systemNavigationBarColor: Colors.transparent,
  systemNavigationBarIconBrightness: Brightness.light,
  systemNavigationBarContrastEnforced: false,
);

String _id() => DateTime.now().microsecondsSinceEpoch.toString();

String? _autoVehicleImage(String brand, String model) {
  final b = brand.trim().toLowerCase();
  final m = model.trim().toLowerCase();
  if (b == 'yamaha' && m.startsWith('r15')) {
    return 'assets/images/yamaha_r15.png';
  }
  return null;
}

Future<String?> _pickPngAndPersist() async {
  String? originalPath;

  // 1) Intentar galeria/camara con image_picker (convierte HEIC a JPEG).
  try {
    final picker = ImagePicker();
    final picked = await picker.pickImage(
      source: ImageSource.gallery,
      imageQuality: 90,
    );
    if (picked != null) {
      final dir = await getApplicationDocumentsDirectory();
      final dest = File('${dir.path}/vehicle_${DateTime.now().microsecondsSinceEpoch}.jpg');
      await File(picked.path).copy(dest.path);
      originalPath = dest.path;
    }
  } catch (_) {
    // cae al selector de archivos
  }

  // 2) Respaldo: selector de archivos (PNG/JPG/JPEG/HEIC/HEIF).
  if (originalPath == null) {
    try {
      final file = await FilePicker.pickFile(
        type: FileType.custom,
        allowedExtensions: const ['png', 'jpg', 'jpeg', 'heic', 'heif'],
      );
      if (file == null) return null;
      final path = file.path;
      if (path == null) return null;
      final dir = await getApplicationDocumentsDirectory();
      final ext = (file.extension ?? 'png').toLowerCase();
      final safeExt = (ext == 'heic' || ext == 'heif') ? 'jpg' : ext;
      final dest = File('${dir.path}/vehicle_${DateTime.now().microsecondsSinceEpoch}.$safeExt');
      await File(path).copy(dest.path);
      originalPath = dest.path;
    } catch (_) {
      return null;
    }
  }

  // 3) Intentar quitar el fondo automaticamente; si falla, conservar original.
  debugPrint('[PICK] originalPath=$originalPath');
  final processed = await _removeBackground(originalPath);
  debugPrint('[PICK] processed=$processed');
  final result = processed ?? originalPath;
  debugPrint('[PICK] final returned=$result');
  return result;
}

Future<String?> _removeBackground(String sourcePath) async {
  debugPrint('[BG-REMOVE] start: $sourcePath');
  try {
    final bytes = await File(sourcePath).readAsBytes();
    debugPrint('[BG-REMOVE] read bytes: ${bytes.length}');
    final image = img.decodeImage(bytes);
    if (image == null) {
      debugPrint('[BG-REMOVE] decodeImage returned null');
      return null;
    }
    debugPrint('[BG-REMOVE] decoded: ${image.width}x${image.height}');

    final bg = _cornerColor(image);
    debugPrint('[BG-REMOVE] corner color: r=${bg.r} g=${bg.g} b=${bg.b}');

    // 1) Flood-fill desde TODOS los pixeles del borde: elimina el fondo conectado
    //    a los bordes (preserva la moto aunque comparta colores con el fondo).
    final visited = List.generate(image.height, (_) => List<bool>.filled(image.width, false));
    final queue = <(int, int)>[];
    void enqueue(int x, int y) {
      if (x < 0 || y < 0 || x >= image.width || y >= image.height) return;
      if (visited[y][x]) return;
      if (!_similar(image.getPixel(x, y), bg)) return;
      visited[y][x] = true;
      queue.add((x, y));
    }
    for (int x = 0; x < image.width; x++) {
      enqueue(x, 0);
      enqueue(x, image.height - 1);
    }
    for (int y = 0; y < image.height; y++) {
      enqueue(0, y);
      enqueue(image.width - 1, y);
    }
    int removed = 0;
    while (queue.isNotEmpty) {
      final (x, y) = queue.removeLast();
      image.setPixelRgba(x, y, 0, 0, 0, 0);
      removed++;
      enqueue(x + 1, y);
      enqueue(x - 1, y);
      enqueue(x, y + 1);
      enqueue(x, y - 1);
    }
    debugPrint('[BG-REMOVE] flood-fill removed: $removed');

    // 2) Eliminar islas pequenas de fondo restantes (manchas) con umbral mas estricto.
    //    Solo elimina pixeles muy cercanos al color de fondo que formen componentes
    //    pequenos, para no comerse la moto.
    final strictThreshold = 40;
    bool strictSimilar(img.Pixel px) {
      final dr = (px.r - bg.r).abs();
      final dg = (px.g - bg.g).abs();
      final db = (px.b - bg.b).abs();
      return (dr + dg + db) < strictThreshold;
    }
    // Marcar componentes conectados de pixeles "fondo estricto".
    final compId = List.generate(image.height, (_) => List<int>.filled(image.width, -1));
    int compCount = 0;
    final compSizes = <int>[];
    for (int sy = 0; sy < image.height; sy++) {
      for (int sx = 0; sx < image.width; sx++) {
        if (compId[sy][sx] != -1) continue;
        final px = image.getPixel(sx, sy);
        if (px.a == 0) {
          compId[sy][sx] = -2; // transparente ya
          continue;
        }
        if (!strictSimilar(px)) {
          compId[sy][sx] = -3; // no fondo
          continue;
        }
        // BFS componente
        final stack = <(int, int)>[(sx, sy)];
        compId[sy][sx] = compCount;
        int size = 0;
        while (stack.isNotEmpty) {
          final (cx, cy) = stack.removeLast();
          size++;
          for (final (nx, ny) in [(cx+1,cy),(cx-1,cy),(cx,cy+1),(cx,cy-1)]) {
            if (nx < 0 || ny < 0 || nx >= image.width || ny >= image.height) continue;
            if (compId[ny][nx] != -1) continue;
            final npx = image.getPixel(nx, ny);
            if (npx.a == 0) { compId[ny][nx] = -2; continue; }
            if (!strictSimilar(npx)) { compId[ny][nx] = -3; continue; }
            compId[ny][nx] = compCount;
            stack.add((nx, ny));
          }
        }
        compSizes.add(size);
        compCount++;
      }
    }
    // Eliminar componentes pequenos (islas) < 2% del area total
    final totalArea = image.width * image.height;
    final smallThreshold = totalArea ~/ 50;
    int extraRemoved = 0;
    for (int y = 0; y < image.height; y++) {
      for (int x = 0; x < image.width; x++) {
        final id = compId[y][x];
        if (id >= 0 && id < compSizes.length && compSizes[id] < smallThreshold) {
          image.setPixelRgba(x, y, 0, 0, 0, 0);
          extraRemoved++;
        }
      }
    }
    debugPrint('[BG-REMOVE] extra small-islands removed: $extraRemoved');

    final dir = await getApplicationDocumentsDirectory();
    final dest = File('${dir.path}/vehicle_${DateTime.now().microsecondsSinceEpoch}.png');
    final pngBytes = img.encodePng(image);
    await dest.writeAsBytes(pngBytes);
    debugPrint('[BG-REMOVE] saved: ${dest.path} (${pngBytes.length} bytes)');
    return dest.path;
  } catch (e, st) {
    debugPrint('[BG-REMOVE] error: $e');
    debugPrint('[BG-REMOVE] stack: $st');
    return null;
  }
}

img.Color _cornerColor(img.Image image) {
  // Promedio de las 4 esquinas.
  final c1 = image.getPixel(0, 0);
  final c2 = image.getPixel(image.width - 1, 0);
  final c3 = image.getPixel(0, image.height - 1);
  final c4 = image.getPixel(image.width - 1, image.height - 1);
  final r = (c1.r + c2.r + c3.r + c4.r) ~/ 4;
  final g = (c1.g + c2.g + c3.g + c4.g) ~/ 4;
  final b = (c1.b + c2.b + c3.b + c4.b) ~/ 4;
  return img.ColorRgba8(r, g, b, 255);
}

bool _similar(img.Pixel pixel, img.Color bg) {
  final dr = (pixel.r - bg.r).abs();
  final dg = (pixel.g - bg.g).abs();
  final db = (pixel.b - bg.b).abs();
  return (dr + dg + db) < 80;
}


Widget _vehicleImage(Vehicle vehicle) {
  final path = vehicle.imagePath;
  if (path == null || path.isEmpty) {
    return Icon(Icons.two_wheeler_rounded, size: 34, color: _muted);
  }
  if (path.startsWith('assets/')) {
    return Image.asset(
      path,
      fit: BoxFit.contain,
      errorBuilder: (_, _, _) =>
          const Icon(Icons.two_wheeler_rounded, size: 34, color: _muted),
    );
  }
  return Image.file(
    File(path),
    fit: BoxFit.contain,
    errorBuilder: (_, _, _) =>
        const Icon(Icons.two_wheeler_rounded, size: 34, color: _muted),
  );
}
String _today() {
  final d = DateTime.now();
  return '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
}

String _dateIso(DateTime date) =>
    '${date.year}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}';

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
    this.imagePath,
  }) : id = id ?? _id();

  final String id;
  String brand;
  String model;
  int year;
  int cc;
  int km;
  String plate;
  String color;
  String? imagePath;

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
    'imagePath': imagePath,
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
    imagePath: json['imagePath'] as String?,
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
    this.intervalKm = 0,
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

  bool get isPeriodic => intervalKm > 0;

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
    intervalKm: (json['intervalKm'] as num?)?.toInt() ?? 0,
    lastKm: (json['lastKm'] as num?)?.toInt() ?? 0,
    cost: (json['cost'] as num?)?.toDouble() ?? 0,
    date: json['date'] as String? ?? _today(),
  );
}

class ExpenseEntry {
  ExpenseEntry({
    String? id,
    required this.vehicleId,
    required this.description,
    required this.amount,
    required this.date,
  }) : id = id ?? _id();

  final String id;
  String vehicleId;
  String description;
  double amount;
  String date;

  Map<String, Object?> toJson() => {
    'id': id,
    'vehicleId': vehicleId,
    'description': description,
    'amount': amount,
    'date': date,
  };

  factory ExpenseEntry.fromJson(Map<String, dynamic> json) => ExpenseEntry(
    id: json['id'] as String?,
    vehicleId: json['vehicleId'] as String? ?? '',
    description: json['description'] as String? ?? '',
    amount: (json['amount'] as num?)?.toDouble() ?? 0,
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

  factory VehicleDocument.fromJson(Map<String, dynamic> json) =>
      VehicleDocument(
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
  List<ExpenseEntry> expenses = [];
  String owner = '';
  String nickname = '';
  String city = '';
  String? avatarPath;
  double pricePerGallon = 16350;
  bool notifications = true;
  bool reminders = true;

  Map<String, Object?> toJson() => {
    'vehicles': vehicles.map((e) => e.toJson()).toList(),
    'fuel': fuel.map((e) => e.toJson()).toList(),
    'services': services.map((e) => e.toJson()).toList(),
    'documents': documents.map((e) => e.toJson()).toList(),
    'expenses': expenses.map((e) => e.toJson()).toList(),
    'owner': owner,
    'nickname': nickname,
    'city': city,
    'avatarPath': avatarPath,
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
      ..expenses = (json['expenses'] as List? ?? [])
          .whereType<Map>()
          .map((e) => ExpenseEntry.fromJson(Map<String, dynamic>.from(e)))
          .toList()
      ..owner = json['owner'] as String? ?? ''
      ..nickname = json['nickname'] as String? ?? ''
      ..city = json['city'] as String? ?? ''
      ..avatarPath = json['avatarPath'] as String?
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
        _selectedVehicleId = data.vehicles.isEmpty
            ? null
            : data.vehicles.first.id;
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
    final route = PageRouteBuilder<T>(
      opaque: false,
      barrierDismissible: true,
      barrierColor: Colors.black.withValues(alpha: .52),
      barrierLabel: 'Cerrar',
      transitionDuration: const Duration(milliseconds: 300),
      reverseTransitionDuration: const Duration(milliseconds: 220),
      pageBuilder: (context, animation, secondaryAnimation) => builder(context),
      transitionsBuilder: (context, animation, secondaryAnimation, child) {
        final curved = CurvedAnimation(
          parent: animation,
          curve: Curves.easeOutCubic,
          reverseCurve: Curves.easeInCubic,
        );
        return FadeTransition(
          opacity: curved,
          child: ScaleTransition(
            scale: Tween<double>(begin: .94, end: 1).animate(curved),
            child: child,
          ),
        );
      },
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
      ..showSnackBar(
        SnackBar(
          content: Text(text),
          backgroundColor: isError ? _red : _surface2,
        ),
      );
  }

  @override
  Widget build(BuildContext context) {
    final theme = ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      scaffoldBackgroundColor: _background,
      splashFactory: NoSplash.splashFactory,
      colorScheme:
          ColorScheme.fromSeed(
            seedColor: _red,
            brightness: Brightness.dark,
            surface: _surface,
          ).copyWith(
            primary: _red,
            onPrimary: Colors.white,
            secondary: _redGlow,
            onSurface: const Color(0xFFF2F5F9),
            outline: Colors.white.withValues(alpha: .18),
          ),
      textTheme: const TextTheme(
        headlineMedium: TextStyle(
          fontWeight: FontWeight.w800,
          letterSpacing: -1,
          color: Color(0xFFF5F7FA),
        ),
        headlineSmall: TextStyle(
          fontWeight: FontWeight.w800,
          letterSpacing: -.5,
          color: Color(0xFFF5F7FA),
        ),
        titleLarge: TextStyle(
          fontWeight: FontWeight.w700,
          color: Color(0xFFF5F7FA),
        ),
        titleMedium: TextStyle(
          fontWeight: FontWeight.w600,
          color: Color(0xFFF5F7FA),
        ),
        bodyMedium: TextStyle(color: Color(0xFFD8DCE2)),
        labelSmall: TextStyle(color: _muted, fontWeight: FontWeight.w600),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: ButtonStyle(
          foregroundColor: const WidgetStatePropertyAll(Colors.white),
          backgroundColor: const WidgetStatePropertyAll(Colors.transparent),
          elevation: const WidgetStatePropertyAll(0),
          padding: const WidgetStatePropertyAll(
            EdgeInsets.symmetric(horizontal: 22, vertical: 14),
          ),
          textStyle: const WidgetStatePropertyAll(
            TextStyle(fontWeight: FontWeight.w700, letterSpacing: .2),
          ),
          shape: const WidgetStatePropertyAll(StadiumBorder()),
          overlayColor: WidgetStateProperty.resolveWith(
            (states) => states.contains(WidgetState.pressed)
                ? Colors.white.withValues(alpha: .16)
                : null,
          ),
          backgroundBuilder: (context, states, child) => DecoratedBox(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(30),
              gradient: LinearGradient(
                begin: Alignment.topLeft,
                end: Alignment.bottomRight,
                colors: states.contains(WidgetState.disabled)
                    ? [_red.withValues(alpha: .28), _red.withValues(alpha: .18)]
                    : const [_redGlow, _red, Color(0xFFB71C1C)],
              ),
              border: Border.all(
                color: Colors.white.withValues(alpha: .22),
                width: .8,
              ),
              boxShadow: [
                if (!states.contains(WidgetState.disabled))
                  BoxShadow(
                    color: _red.withValues(alpha: .30),
                    blurRadius: 22,
                    offset: const Offset(0, 9),
                    spreadRadius: -4,
                  ),
              ],
            ),
            child: child,
          ),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: Colors.white,
          backgroundColor: Colors.white.withValues(alpha: .045),
          side: BorderSide(color: Colors.white.withValues(alpha: .16)),
          shape: const StadiumBorder(),
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 13),
          textStyle: const TextStyle(fontWeight: FontWeight.w600),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          foregroundColor: Colors.white.withValues(alpha: .92),
          shape: const StadiumBorder(),
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          textStyle: const TextStyle(fontWeight: FontWeight.w600),
          overlayColor: Colors.white.withValues(alpha: .08),
        ),
      ),
      iconButtonTheme: IconButtonThemeData(
        style: IconButton.styleFrom(
          foregroundColor: Colors.white,
          highlightColor: Colors.white.withValues(alpha: .08),
        ),
      ),
      dividerTheme: DividerThemeData(
        color: Colors.white.withValues(alpha: .08),
        thickness: .7,
      ),
      switchTheme: SwitchThemeData(
        thumbColor: WidgetStateProperty.resolveWith(
          (states) =>
              states.contains(WidgetState.selected) ? Colors.white : _muted,
        ),
        trackColor: WidgetStateProperty.resolveWith(
          (states) => states.contains(WidgetState.selected)
              ? _red.withValues(alpha: .78)
              : Colors.white.withValues(alpha: .10),
        ),
        trackOutlineColor: WidgetStatePropertyAll(
          Colors.white.withValues(alpha: .16),
        ),
      ),
      dialogTheme: DialogThemeData(
        backgroundColor: Colors.transparent,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(30)),
      ),
      snackBarTheme: SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        backgroundColor: _surface2.withValues(alpha: .96),
        contentTextStyle: const TextStyle(
          color: Colors.white,
          fontWeight: FontWeight.w500,
        ),
        elevation: 0,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(18)),
      ),
      popupMenuTheme: PopupMenuThemeData(
        color: _surface2.withValues(alpha: .98),
        surfaceTintColor: Colors.transparent,
        elevation: 16,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(20),
          side: BorderSide(color: Colors.white.withValues(alpha: .10)),
        ),
        textStyle: const TextStyle(color: Colors.white),
      ),
      datePickerTheme: DatePickerThemeData(
        backgroundColor: _surface,
        surfaceTintColor: Colors.transparent,
        headerBackgroundColor: Colors.transparent,
        headerForegroundColor: Colors.white,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: Colors.white.withValues(alpha: .055),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(18),
          borderSide: BorderSide(color: Colors.white.withValues(alpha: .10)),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(18),
          borderSide: BorderSide(color: Colors.white.withValues(alpha: .10)),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(18),
          borderSide: const BorderSide(color: _red, width: 1.1),
        ),
        labelStyle: const TextStyle(color: _muted),
        floatingLabelStyle: const TextStyle(color: _muted),
        hintStyle: TextStyle(color: _muted.withValues(alpha: .7)),
      ),
      progressIndicatorTheme: const ProgressIndicatorThemeData(color: _red),
    );

    return MaterialApp(
      navigatorKey: _navigatorKey,
      title: 'Mi Garaje',
      debugShowCheckedModeBanner: false,
      theme: theme,
      builder: (context, child) => AnnotatedRegion<SystemUiOverlayStyle>(
        value: _systemOverlay,
        child: child ?? const SizedBox.shrink(),
      ),
      home: _loading
          ? _bootScreen(const CircularProgressIndicator(color: _red))
          : _loadError != null
          ? _bootScreen(
              Padding(
                padding: const EdgeInsets.all(24),
                child: Text(
                  _loadError!,
                  textAlign: TextAlign.center,
                  style: const TextStyle(color: _muted),
                ),
              ),
            )
          : Scaffold(
              extendBody: true,
              backgroundColor: Colors.transparent,
              body: Stack(
                children: [
                  const Positioned.fill(child: _AmbientBackground()),
                  SafeArea(
                    bottom: false,
                    child: Center(
                      child: ConstrainedBox(
                        constraints: const BoxConstraints(maxWidth: 600),
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
                    ),
                  ),
                  Positioned(
                    left: 0,
                    right: 0,
                    bottom: 0,
                    height: 180,
                    child: IgnorePointer(
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          gradient: LinearGradient(
                            begin: Alignment.topCenter,
                            end: Alignment.bottomCenter,
                            colors: [
                              Colors.transparent,
                              _background.withValues(alpha: .55),
                              _background.withValues(alpha: .96),
                            ],
                            stops: const [0, .55, 1],
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
              floatingActionButton: Padding(
                padding: const EdgeInsets.only(bottom: 96),
                child: switch (_tab) {
                2 => _LiquidFab(
                  icon: Icons.mic_rounded,
                  label: 'Dictar mantenimiento',
                  onTap: () => _addService(startVoiceImmediately: true),
                ),
                3 => _LiquidFab(
                  icon: Icons.local_gas_station_rounded,
                  label: 'Dictar combustible',
                  onTap: () => _addFuel(startVoiceImmediately: true),
                ),
                _ => null,
              },
              ),
              floatingActionButtonLocation:
                  FloatingActionButtonLocation.endFloat,
              bottomNavigationBar: _LiquidNavBar(
                index: _tab,
                tabs: _tabs,
                onSelected: (index) => setState(() => _tab = index),
              ),
            ),
    );
  }

  Widget _bootScreen(Widget child) => Scaffold(
    backgroundColor: Colors.transparent,
    body: Stack(
      children: [
        const Positioned.fill(child: _AmbientBackground()),
        Center(child: child),
      ],
    ),
  );

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
    final services =
        _data.services.where((e) => e.vehicleId == vehicle?.id).toList()..sort(
          (a, b) => a
              .remaining(vehicle?.km ?? 0)
              .compareTo(b.remaining(vehicle?.km ?? 0)),
        );
    final activity =
        _data.fuel.where((e) => e.vehicleId == vehicle?.id).toList()
          ..sort((a, b) => b.date.compareTo(a.date));

    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(18, 18, 18, 140),
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
                    Text(
                      vehicle.title,
                      style: Theme.of(context).textTheme.headlineSmall
                          ?.copyWith(
                            fontWeight: FontWeight.w800,
                            color: const Color(0xFFF5F7FA),
                          ),
                    ),
                    const SizedBox(height: 5),
                    Text(
                      '${vehicle.year} · ${vehicle.cc} cc · ${vehicle.plate.isEmpty ? 'Sin placa' : vehicle.plate} · ${vehicle.km} km',
                      style: const TextStyle(color: _muted, fontSize: 12),
                    ),
                  ],
                ),
              ),
              _pill(
                vehicle.km == 0 ? 'Nueva' : 'Activa',
                vehicle.km == 0 ? _red : _green,
              ),
            ],
          ),
          const SizedBox(height: 10),
          _hero(vehicle),
          const SizedBox(height: 20),
          Row(
            children: [
              _metric(
                'Próximo servicio',
                services.where((s) => s.isPeriodic).isEmpty
                    ? '—'
                    : '${math.max(0, services.where((s) => s.isPeriodic).reduce((a, b) => a.remaining(vehicle.km) < b.remaining(vehicle.km) ? a : b).remaining(vehicle.km))} km',
              ),
              const SizedBox(width: 10),
              _metric('Gasto del mes', _money(monthSpend)),
              const SizedBox(width: 10),
              _metric('Consumo', _consumption(vehicle)),
            ],
          ),
          const SizedBox(height: 24),
          _sectionTitle('Accesos rápidos'),
          const SizedBox(height: 12),
          Row(
            children: [
              _quickAction(
                Icons.local_gas_station_rounded,
                'Combustible',
                _addFuel,
              ),
              _quickAction(Icons.build_rounded, 'Servicio', _addService),
              _quickAction(
                Icons.bar_chart_rounded,
                'Gastos',
                () => setState(() => _tab = 3),
              ),
              _quickAction(
                Icons.description_rounded,
                'Documentos',
                _addDocument,
              ),
            ],
          ),
          const SizedBox(height: 24),
          _sectionTitle(
            'Mantenimiento',
            action: 'Ver todo',
            onAction: () => setState(() => _tab = 2),
          ),
          const SizedBox(height: 10),
          if (services.isEmpty)
            _emptyCard('Aún no has agregado servicios')
          else
            ...services
                .take(2)
                .map(
                  (service) => Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: _serviceTile(service, vehicle),
                  ),
                ),
          const SizedBox(height: 18),
          _sectionTitle('Actividad reciente'),
          const SizedBox(height: 10),
          if (activity.isEmpty)
            _emptyCard('Sin movimientos todavía')
          else
            ...activity
                .take(3)
                .map(
                  (entry) => Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: _activityTile(entry),
                  ),
                ),
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
              Text(
                name,
                style: const TextStyle(
                  fontSize: 24,
                  fontWeight: FontWeight.w800,
                ),
              ),
              const SizedBox(height: 3),
              Text(
                _dateLabel(_today()),
                style: const TextStyle(color: _muted, fontSize: 12),
              ),
            ],
          ),
        ),
        _Glass(
          radius: 24,
          blur: 12,
          tint: _surface,
          opacity: .58,
          borderAlpha: .16,
          child: Material(
            type: MaterialType.transparency,
            child: InkWell(
              customBorder: const CircleBorder(),
              onTap: () => setState(() => _tab = 4),
              child: const SizedBox(
                width: 48,
                height: 48,
                child: Icon(Icons.person_outline_rounded, size: 21),
              ),
            ),
          ),
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
        Positioned.fill(child: CustomPaint(painter: _HeroGridPainter())),
        _heroImage(
          vehicle,
          onPick: () async {
            final path = await _pickPngAndPersist();
            if (path != null) {
              vehicle.imagePath = path;
              await _save();
              if (mounted) setState(() {});
            }
          },
        ),
        Positioned(
          bottom: 2,
          child: _Glass(
            radius: 20,
            blur: 14,
            tint: _background,
            opacity: .46,
            borderAlpha: .14,
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
            child: Text(
              'TU MOTO, SIEMPRE A PUNTO',
              style: TextStyle(
                color: Colors.white.withValues(alpha: .70),
                fontSize: 9.5,
                letterSpacing: 1.8,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
        ),
      ],
    ),
  );

  Widget _heroImage(Vehicle vehicle, {VoidCallback? onPick}) {
    final path = vehicle.imagePath;
    final isR15 = _autoVehicleImage(vehicle.brand, vehicle.model) != null;
    if (path == null || path.isEmpty) {
      if (isR15) {
        return Image.asset(
          'assets/images/yamaha_r15.png',
          fit: BoxFit.contain,
          errorBuilder: (_, _, _) => Image.asset(
            'assets/images/mi_moto.png',
            fit: BoxFit.contain,
          ),
        );
      }
      return Center(
        child: FilledButton.icon(
          onPressed: onPick,
          icon: const Icon(Icons.upload_file_rounded),
          label: const Text('Elegir foto'),
        ),
      );
    }
    if (path.startsWith('assets/')) {
      return Image.asset(
        path,
        fit: BoxFit.contain,
        errorBuilder: (_, _, _) => Image.asset(
          'assets/images/mi_moto.png',
          fit: BoxFit.contain,
        ),
      );
    }
    return Image.file(
      File(path),
      fit: BoxFit.contain,
      errorBuilder: (_, _, _) => Image.asset(
        'assets/images/mi_moto.png',
        fit: BoxFit.contain,
      ),
    );
  }

  Widget _emptyVehicleCard() => _Panel(
    padding: const EdgeInsets.all(22),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Icon(Icons.two_wheeler_rounded, color: _red, size: 34),
        const SizedBox(height: 12),
        const Text(
          'Tu garaje empieza aquí',
          style: TextStyle(fontSize: 19, fontWeight: FontWeight.w800),
        ),
        const SizedBox(height: 6),
        const Text(
          'Agrega tu moto para registrar combustible, servicios y documentos.',
          style: TextStyle(color: _muted),
        ),
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
    child: SizedBox(
      height: 78,
      child: _Glass(
        radius: 16,
        tint: _surface,
        opacity: .56,
        borderAlpha: .13,
        highlight: .09,
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 10),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text(
              title,
              textAlign: TextAlign.center,
              style: const TextStyle(color: _muted, fontSize: 10),
            ),
            const SizedBox(height: 6),
            FittedBox(
              fit: BoxFit.scaleDown,
              child: Text(
                value,
                style: const TextStyle(fontWeight: FontWeight.w800),
              ),
            ),
          ],
        ),
      ),
    ),
  );

  Widget _quickAction(
    IconData icon,
    String label,
    VoidCallback action,
  ) => Expanded(
    child: InkWell(
      onTap: action,
      borderRadius: BorderRadius.circular(22),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Column(
          children: [
            Container(
              width: 52,
              height: 52,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(18),
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [
                    Colors.white.withValues(alpha: .14),
                    Colors.white.withValues(alpha: .035),
                  ],
                ),
                border: Border.all(color: Colors.white.withValues(alpha: .14)),
                boxShadow: [
                  BoxShadow(
                    color: (label == 'Combustible' ? _red : Colors.black)
                        .withValues(alpha: label == 'Combustible' ? .26 : .32),
                    blurRadius: 18,
                    offset: const Offset(0, 8),
                    spreadRadius: -4,
                  ),
                ],
              ),
              child: Icon(
                icon,
                size: 22,
                color: label == 'Combustible' ? _redGlow : Colors.white,
              ),
            ),
            const SizedBox(height: 8),
            FittedBox(
              fit: BoxFit.scaleDown,
              child: Text(
                label,
                style: const TextStyle(color: _muted, fontSize: 10.5),
              ),
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
      padding: const EdgeInsets.fromLTRB(18, 20, 18, 140),
      children: [
        _pageHeader(
          'Mi garaje',
          '${_data.vehicles.length} ${_data.vehicles.length == 1 ? 'vehículo' : 'vehículos'}',
        ),
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
          ..._data.vehicles.map(
            (item) => Padding(
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
                        child: _vehicleImage(item),
                      ),
                      const SizedBox(width: 14),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              item.brand.toUpperCase(),
                              style: const TextStyle(
                                color: _muted,
                                fontSize: 10,
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                            Text(
                              item.model,
                              style: const TextStyle(
                                fontSize: 18,
                                fontWeight: FontWeight.w800,
                              ),
                            ),
                            Text(
                              '${item.year} · ${item.cc} cc · ${item.km} km',
                              style: const TextStyle(
                                color: _muted,
                                fontSize: 12,
                              ),
                            ),
                            if (item.plate.isNotEmpty)
                              Text(
                                'Placa ${item.plate}',
                                style: const TextStyle(
                                  color: _muted,
                                  fontSize: 12,
                                ),
                              ),
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
                          PopupMenuItem(
                            value: 'delete',
                            child: Text('Eliminar'),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        if (vehicle != null) ...[
          const SizedBox(height: 8),
          _sectionTitle('Documentación'),
          const SizedBox(height: 10),
          ..._data.documents
              .where((d) => d.vehicleId == vehicle.id)
              .map(
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
    final services = _data.services
        .where((e) => e.vehicleId == vehicle?.id)
        .toList();
    final docs = _data.documents
        .where((e) => e.vehicleId == vehicle?.id)
        .toList();
    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(18, 20, 18, 140),
      children: [
        _pageHeader('Mantenimiento', 'Cuida tu moto, kilómetro a kilómetro'),
        const SizedBox(height: 18),
        if (vehicle == null)
          _emptyVehicleCard()
        else ...[
          _Panel(
            child: Row(
              children: [
                const CircleAvatar(
                  backgroundColor: Color(0x223FB950),
                  child: Icon(Icons.speed_rounded, color: _green),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    '${vehicle.km} km registrados',
                    style: const TextStyle(fontWeight: FontWeight.w700),
                  ),
                ),
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
            ...services.map(
              (service) => Padding(
                padding: const EdgeInsets.only(bottom: 9),
                child: _serviceTile(service, vehicle, actionable: true),
              ),
            ),
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
            ...docs.map(
              (d) => Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: _documentTile(d),
              ),
            ),
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
      return _data.fuel
          .where((entry) {
            final date = DateTime.tryParse(entry.date);
            return date != null &&
                date.year == month.year &&
                date.month == month.month;
          })
          .fold<double>(0, (sum, entry) => sum + entry.amount);
    });
    final maxValue = monthly.fold<double>(1, math.max);
    final monthTotal = monthly.last;
    final vehicle = _vehicle;
    final fuelCount = _data.fuel
        .where((e) => e.vehicleId == vehicle?.id)
        .length;
    return ListView(
      physics: const BouncingScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(18, 20, 18, 140),
      children: [
        _pageHeader('Estadísticas', 'Resumen de tus gastos'),
        const SizedBox(height: 18),
        _Panel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Gasto de este mes', style: TextStyle(color: _muted)),
              const SizedBox(height: 8),
              Text(
                _money(monthTotal),
                style: const TextStyle(
                  fontSize: 30,
                  fontWeight: FontWeight.w800,
                ),
              ),
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
                                    heightFactor: (monthly[i] / maxValue).clamp(
                                      0.06,
                                      1,
                                    ),
                                    child: Container(
                                      decoration: BoxDecoration(
                                        borderRadius: BorderRadius.circular(8),
                                        gradient: LinearGradient(
                                          begin: Alignment.topCenter,
                                          end: Alignment.bottomCenter,
                                          colors: [
                                            _red,
                                            _red.withValues(alpha: 0.34),
                                          ],
                                        ),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                              const SizedBox(height: 8),
                              Text(
                                _monthName(
                                  DateTime(now.year, now.month - 5 + i).month,
                                ),
                                style: const TextStyle(
                                  color: _muted,
                                  fontSize: 10,
                                ),
                              ),
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
            _metric(
              'Cargas este mes',
              '${_data.fuel.where((e) {
                final d = DateTime.tryParse(e.date);
                return d != null && d.year == now.year && d.month == now.month;
              }).length}',
            ),
            _metric('Registros', '$fuelCount'),
          ],
        ),
        const SizedBox(height: 14),
        _Panel(
          child: Row(
            children: [
              const Icon(Icons.local_gas_station_rounded, color: _red),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  'Precio configurado por galón: ${_money(_data.pricePerGallon)}',
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 22),
        _sectionTitle('Gastos y reparaciones', action: 'Agregar', onAction: _addExpense),
        const SizedBox(height: 10),
        if (_data.expenses.isEmpty)
          _emptyCard('Aún no has registrado gastos')
        else
          ..._data.expenses
              .where((e) => e.vehicleId == vehicle?.id)
              .toList()
              .reversed
              .map(
                (e) => Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: _Panel(
                    padding: const EdgeInsets.all(12),
                    child: Row(
                      children: [
                        Container(
                          width: 40,
                          height: 40,
                          decoration: BoxDecoration(
                            borderRadius: BorderRadius.circular(12),
                            color: _red.withValues(alpha: .12),
                          ),
                          child: const Icon(Icons.build_rounded, color: _red, size: 20),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                e.description,
                                style: const TextStyle(fontWeight: FontWeight.w700),
                              ),
                              Text(
                                _dateLabel(e.date),
                                style: const TextStyle(color: _muted, fontSize: 11),
                              ),
                            ],
                          ),
                        ),
                        Text(
                          _money(e.amount),
                          style: const TextStyle(fontWeight: FontWeight.w800),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
      ],
    );
  }

  Widget _profile() => ListView(
    physics: const BouncingScrollPhysics(),
    padding: const EdgeInsets.fromLTRB(18, 20, 18, 140),
    children: [
      _pageHeader('Perfil', 'Tu cuenta y preferencias'),
      const SizedBox(height: 18),
      _Panel(
        child: Row(
          children: [
            Container(
              width: 58,
              height: 58,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: _surface2,
                border: Border.all(color: Colors.white.withValues(alpha: .14)),
              ),
              child: ClipOval(
                child: _data.avatarPath == null
                    ? Center(
                        child: Text(
                          _data.owner.trim().isEmpty
                              ? 'MG'
                              : _data.owner
                                    .trim()
                                    .split(RegExp(r'\\s+'))
                                    .take(2)
                                    .map((e) => e[0])
                                    .join()
                                    .toUpperCase(),
                          style: const TextStyle(fontWeight: FontWeight.w800),
                        ),
                      )
                    : (_data.avatarPath!.startsWith('assets/')
                          ? Image.asset(_data.avatarPath!, fit: BoxFit.cover)
                          : Image.file(File(_data.avatarPath!), fit: BoxFit.cover)),
              ),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    _data.owner.isEmpty ? 'Motociclista' : _data.owner,
                    style: const TextStyle(
                      fontWeight: FontWeight.w800,
                      fontSize: 17,
                    ),
                  ),
                  if (_data.nickname.isNotEmpty)
                    Text(
                      '@${_data.nickname}',
                      style: const TextStyle(color: _muted, fontSize: 12),
                    ),
                  Text(
                    _data.city.isEmpty ? 'Configura tu ciudad' : _data.city,
                    style: const TextStyle(color: _muted),
                  ),
                ],
              ),
            ),
            IconButton(
              onPressed: _editProfile,
              icon: const Icon(Icons.edit_outlined),
            ),
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
            Expanded(
              child: Text(
                'Tus datos se guardan localmente en este dispositivo.',
              ),
            ),
          ],
        ),
      ),
    ],
  );

  Widget _pageHeader(String title, String subtitle) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text(
        title,
        style: Theme.of(context).textTheme.headlineMedium
            ?.copyWith(color: const Color(0xFFF5F7FA)),
      ),
      const SizedBox(height: 4),
      Text(subtitle, style: const TextStyle(color: _muted)),
    ],
  );

  Widget _sectionTitle(
    String title, {
    String? action,
    VoidCallback? onAction,
  }) => Row(
    children: [
      Expanded(
        child: Text(
          title.toUpperCase(),
          style: const TextStyle(
            color: _muted,
            fontSize: 11,
            letterSpacing: 0.8,
            fontWeight: FontWeight.w800,
          ),
        ),
      ),
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
      borderRadius: BorderRadius.circular(30),
      gradient: LinearGradient(
        begin: Alignment.topLeft,
        end: Alignment.bottomRight,
        colors: [color.withValues(alpha: .22), color.withValues(alpha: .07)],
      ),
      border: Border.all(color: color.withValues(alpha: .32)),
      boxShadow: [
        BoxShadow(
          color: color.withValues(alpha: .16),
          blurRadius: 14,
          offset: const Offset(0, 4),
        ),
      ],
    ),
    child: Text(
      text,
      style: TextStyle(color: color, fontSize: 11, fontWeight: FontWeight.w700),
    ),
  );

  Widget _emptyCard(String text) => _Panel(
    child: Row(
      children: [
        const Icon(Icons.info_outline_rounded, color: _muted),
        const SizedBox(width: 12),
        Expanded(
          child: Text(text, style: const TextStyle(color: _muted)),
        ),
      ],
    ),
  );

  Widget _serviceTile(
    ServiceEntry service,
    Vehicle vehicle, {
    bool actionable = false,
  }) {
    final remaining = service.remaining(vehicle.km);
    final overdue = remaining <= 0;
    final progress = (1 - remaining / math.max(service.intervalKm, 1)).clamp(
      0.0,
      1.0,
    );
    return _Panel(
      child: Row(
        children: [
          CircleAvatar(
            radius: 18,
            backgroundColor: (overdue ? _red : _green).withValues(alpha: 0.14),
            child: Icon(
              Icons.build_rounded,
              size: 17,
              color: overdue ? _red : _green,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  service.name,
                  style: const TextStyle(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 4),
                Text(
                  overdue
                      ? 'Vencido · ${service.intervalKm} km'
                      : 'Faltan $remaining km',
                  style: TextStyle(
                    color: overdue ? _red : _muted,
                    fontSize: 12,
                  ),
                ),
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
            child: Icon(
              Icons.verified_user_outlined,
              color: expired ? _red : _green,
              size: 19,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  document.name,
                  style: const TextStyle(fontWeight: FontWeight.w700),
                ),
                Text(
                  'Vence ${_dateLabel(document.expiry)}',
                  style: const TextStyle(color: _muted, fontSize: 12),
                ),
              ],
            ),
          ),
          Text(
            expired ? 'Vencido' : 'Vigente',
            style: TextStyle(
              color: expired ? _red : _green,
              fontSize: 11,
              fontWeight: FontWeight.w700,
            ),
          ),
          PopupMenuButton<String>(
            onSelected: (value) async {
              if (value == 'delete') {
                _data.documents.removeWhere((e) => e.id == document.id);
                await _save();
              }
            },
            itemBuilder: (_) => const [
              PopupMenuItem(value: 'delete', child: Text('Eliminar')),
            ],
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
        Expanded(
          child: Text(
            entry.station.isEmpty ? 'Carga de combustible' : entry.station,
          ),
        ),
        Text(
          _money(entry.amount),
          style: const TextStyle(fontWeight: FontWeight.w700),
        ),
      ],
    ),
  );

  String _consumption(Vehicle vehicle) {
    final entries = _data.fuel.where((e) => e.vehicleId == vehicle.id).toList()
      ..sort((a, b) => a.km.compareTo(b.km));
    if (entries.length < 2) return '—';
    final distance = entries.last.km - entries.first.km;
    final gallons = entries
        .skip(1)
        .fold<double>(
          0,
          (sum, e) =>
              sum + (e.pricePerGallon <= 0 ? 0 : e.amount / e.pricePerGallon),
        );
    if (distance <= 0 || gallons <= 0) return '—';
    return '${(distance / gallons).round()} km/gal';
  }

  String _monthName(int month) => const [
    'ENE',
    'FEB',
    'MAR',
    'ABR',
    'MAY',
    'JUN',
    'JUL',
    'AGO',
    'SEP',
    'OCT',
    'NOV',
    'DIC',
  ][month - 1];

  Future<void> _addVehicle() async {
    final form = GlobalKey<FormState>();
    final brand = TextEditingController();
    final model = TextEditingController();
    final year = TextEditingController(text: '${DateTime.now().year}');
    final cc = TextEditingController(text: '150');
    final km = TextEditingController(text: '0');
    final plate = TextEditingController();
    String? pickedImagePath;
    final saved = await _showDialog<Vehicle>(
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => _GlassDialog(
          title: const Text('Agregar vehículo'),
          content: Form(
            key: form,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  _field(brand, 'Marca', required: true),
                  _field(model, 'Modelo', required: true),
                  Row(
                    children: [
                      Expanded(
                        child: _field(year, 'Año', numeric: true, required: true),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: _field(
                          cc,
                          'Cilindraje (cc)',
                          numeric: true,
                          required: true,
                        ),
                      ),
                    ],
                  ),
                  _field(km, 'Kilometraje actual', numeric: true, required: true),
                  _field(plate, 'Placa (opcional)'),
                  const SizedBox(height: 8),
                  _imagePickerRow(
                    pickedImagePath,
                    onPick: () async {
                      final path = await _pickPngAndPersist();
                      if (path != null) {
                        setDialogState(() => pickedImagePath = path);
                      }
                    },
                    onClear: () => setDialogState(() => pickedImagePath = null),
                  ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () {
                if (!form.currentState!.validate()) return;
                final y = int.tryParse(year.text) ?? 0;
                final c = int.tryParse(cc.text) ?? 0;
                final k = int.tryParse(km.text) ?? 0;
                if (y < 1950 || y > DateTime.now().year || c <= 0 || k < 0) {
                  _message('Revisa año, cilindraje y kilometraje.', isError: true);
                  return;
                }
                final auto = _autoVehicleImage(brand.text.trim(), model.text.trim());
                Navigator.pop(
                  context,
                  Vehicle(
                    brand: brand.text.trim(),
                    model: model.text.trim(),
                    year: y,
                    cc: c,
                    km: k,
                    plate: plate.text.trim().toUpperCase(),
                    imagePath: auto ?? pickedImagePath,
                  ),
                );
              },
              child: const Text('Guardar'),
            ),
          ],
        ),
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
    final oldAuto = _autoVehicleImage(vehicle.brand, vehicle.model);
    String? pickedImagePath =
        (vehicle.imagePath == oldAuto) ? null : vehicle.imagePath;
    final saved = await _showDialog<bool>(
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => _GlassDialog(
          title: const Text('Editar vehículo'),
          content: Form(
            key: form,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  _field(brand, 'Marca', required: true),
                  _field(model, 'Modelo', required: true),
                  Row(
                    children: [
                      Expanded(
                        child: _field(year, 'Año', numeric: true, required: true),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: _field(
                          cc,
                          'Cilindraje (cc)',
                          numeric: true,
                          required: true,
                        ),
                      ),
                    ],
                  ),
                  _field(km, 'Kilometraje actual', numeric: true, required: true),
                  _field(plate, 'Placa'),
                  const SizedBox(height: 8),
                  _imagePickerRow(
                    pickedImagePath,
                    onPick: () async {
                      final path = await _pickPngAndPersist();
                      if (path != null) {
                        setDialogState(() => pickedImagePath = path);
                      }
                    },
                    onClear: () => setDialogState(() => pickedImagePath = null),
                  ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () {
                if (!form.currentState!.validate()) return;
                final y = int.tryParse(year.text) ?? 0;
                final c = int.tryParse(cc.text) ?? 0;
                final k = int.tryParse(km.text) ?? 0;
                if (y < 1950 || y > DateTime.now().year || c <= 0 || k < 0) {
                  _message('Revisa año, cilindraje y kilometraje.', isError: true);
                  return;
                }
                final auto = _autoVehicleImage(brand.text.trim(), model.text.trim());
                vehicle
                  ..brand = brand.text.trim()
                  ..model = model.text.trim()
                  ..year = y
                  ..cc = c
                  ..km = k
                  ..plate = plate.text.trim().toUpperCase()
                  ..imagePath = auto ?? pickedImagePath;
                Navigator.pop(context, true);
              },
              child: const Text('Guardar'),
            ),
          ],
        ),
      ),
    );
    for (final controller in [brand, model, year, cc, km, plate]) {
      controller.dispose();
    }
    if (saved == true) await _save();
  }

  Future<void> _deleteVehicle(Vehicle vehicle) async {
    final ok = await _confirm(
      '¿Eliminar ${vehicle.title}?',
      'También se eliminarán sus cargas, servicios y documentos.',
    );
    if (!ok) return;
    _data.vehicles.removeWhere((e) => e.id == vehicle.id);
    _data.fuel.removeWhere((e) => e.vehicleId == vehicle.id);
    _data.services.removeWhere((e) => e.vehicleId == vehicle.id);
    _data.documents.removeWhere((e) => e.vehicleId == vehicle.id);
    if (_selectedVehicleId == vehicle.id) {
      _selectedVehicleId = _data.vehicles.isEmpty
          ? null
          : _data.vehicles.first.id;
    }
    await _save();
  }

  Future<void> _addFuel({bool startVoiceImmediately = false}) async {
    final vehicle = _vehicle;
    if (vehicle == null) {
      _message('Agrega primero una moto.');
      return;
    }
    final saved = await _showDialog<FuelEntry>(
      builder: (_) => _FuelVoiceDialog(
        vehicle: vehicle,
        defaultPrice: _data.pricePerGallon,
        voiceController: _maintenanceVoice,
        startVoiceImmediately: startVoiceImmediately,
      ),
    );
    if (saved == null) return;
    _data.fuel.add(saved);
    vehicle.km = saved.km;
    _data.pricePerGallon = saved.pricePerGallon;
    await _save();
    _message('Combustible registrado');
  }

  Future<void> _addService({bool startVoiceImmediately = false}) async {
    final vehicle = _vehicle;
    if (vehicle == null) {
      _message('Agrega primero una moto.');
      return;
    }
    final saved = await _showDialog<ServiceEntry>(
      builder: (_) => _MaintenanceServiceDialog(
        vehicle: vehicle,
        voiceController: _maintenanceVoice,
        startVoiceImmediately: startVoiceImmediately,
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
        builder: (context, setDialogState) => _GlassDialog(
          title: const Text('Agregar documento'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: name,
                decoration: const InputDecoration(
                  labelText: 'Nombre (SOAT, tecnomecánica...)',
                ),
              ),
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
                    expiry:
                        '${expiry.year}-${expiry.month.toString().padLeft(2, '0')}-${expiry.day.toString().padLeft(2, '0')}',
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

  Future<void> _addExpense() async {
    final vehicle = _vehicle;
    if (vehicle == null) {
      _message('Agrega primero una moto.', isError: true);
      return;
    }
    final desc = TextEditingController();
    final amount = TextEditingController();
    DateTime date = DateTime.now();
    final saved = await _showDialog<ExpenseEntry>(
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => _GlassDialog(
          title: const Text('Agregar gasto / reparación'),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: desc,
                  textCapitalization: TextCapitalization.sentences,
                  decoration: const InputDecoration(
                    labelText: 'Descripción (ej. pegar cables)',
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: amount,
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(labelText: 'Monto (COP)'),
                ),
                const SizedBox(height: 12),
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: const Icon(Icons.calendar_month_rounded, color: _red),
                  title: const Text('Fecha'),
                  subtitle: Text(_dateLabel(date.toIso8601String())),
                  onTap: () async {
                    final d = await showDatePicker(
                      context: context,
                      initialDate: date,
                      firstDate: DateTime(2000),
                      lastDate: DateTime(2200),
                    );
                    if (d != null) setDialogState(() => date = d);
                  },
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () {
                if (desc.text.trim().isEmpty) {
                  _message('Escribe la descripción.', isError: true);
                  return;
                }
                final value = double.tryParse(amount.text.replaceAll(',', '.')) ?? 0;
                if (value <= 0) {
                  _message('Ingresa un monto válido.', isError: true);
                  return;
                }
                Navigator.pop(
                  context,
                  ExpenseEntry(
                    vehicleId: vehicle.id,
                    description: desc.text.trim(),
                    amount: value,
                    date: _dateIso(date),
                  ),
                );
              },
              child: const Text('Guardar'),
            ),
          ],
        ),
      ),
    );
    desc.dispose();
    amount.dispose();
    if (saved == null) return;
    _data.expenses.add(saved);
    await _save();
    _message('Gasto registrado');
  }

  Future<void> _editProfile() async {
    final form = GlobalKey<FormState>();
    final name = TextEditingController(text: _data.owner);
    final nickname = TextEditingController(text: _data.nickname);
    final city = TextEditingController(text: _data.city);
    String? avatarPath = _data.avatarPath;
    final saved = await _showDialog<bool>(
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => _GlassDialog(
          title: const Text('Editar perfil'),
          content: Form(
            key: form,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  _profileAvatarPicker(
                    avatarPath,
                    onPick: () async {
                      final path = await _pickPngAndPersist();
                      if (path != null) {
                        setDialogState(() => avatarPath = path);
                      }
                    },
                    onClear: () => setDialogState(() => avatarPath = null),
                  ),
                  const SizedBox(height: 14),
                  _field(name, 'Tu nombre', required: true),
                  _field(nickname, 'Apodo / usuario'),
                  _field(city, 'Ciudad'),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancelar'),
            ),
            FilledButton(
              onPressed: () {
                if (!form.currentState!.validate()) return;
                if (name.text.trim().isEmpty) {
                  _message('El nombre es obligatorio.', isError: true);
                  return;
                }
                _data
                  ..owner = name.text.trim()
                  ..nickname = nickname.text.trim()
                  ..city = city.text.trim()
                  ..avatarPath = avatarPath;
                Navigator.pop(context, true);
              },
              child: const Text('Guardar'),
            ),
          ],
        ),
      ),
    );
    name.dispose();
    nickname.dispose();
    city.dispose();
    if (saved == true) {
      await _save();
      _message('Perfil actualizado');
    }
  }

  Future<void> _editPrice() async {
    final form = GlobalKey<FormState>();
    final price = TextEditingController(
      text: _data.pricePerGallon.round().toString(),
    );
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
  }) => Padding(
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

  Widget _profileAvatarPicker(
    String? currentPath, {
    required VoidCallback onPick,
    required VoidCallback onClear,
  }) =>
      Center(
        child: Stack(
          alignment: Alignment.bottomRight,
          children: [
            Container(
              width: 96,
              height: 96,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: _surface2,
                border: Border.all(color: Colors.white.withValues(alpha: .14)),
              ),
              child: ClipOval(
                child: currentPath == null
                    ? Center(
                        child: Text(
                          _data.owner.trim().isEmpty
                              ? 'MG'
                              : _data.owner
                                    .trim()
                                    .split(RegExp(r'\s+'))
                                    .take(2)
                                    .map((e) => e[0])
                                    .join()
                                    .toUpperCase(),
                          style: const TextStyle(
                            fontWeight: FontWeight.w800,
                            fontSize: 26,
                          ),
                        ),
                      )
                    : (currentPath.startsWith('assets/')
                          ? Image.asset(currentPath, fit: BoxFit.cover)
                          : Image.file(File(currentPath), fit: BoxFit.cover)),
              ),
            ),
            Material(
              color: _red,
              shape: const CircleBorder(),
              child: InkWell(
                customBorder: const CircleBorder(),
                onTap: onPick,
                child: const Padding(
                  padding: EdgeInsets.all(8),
                  child: Icon(Icons.camera_alt_rounded, size: 18, color: Colors.white),
                ),
              ),
            ),
            if (currentPath != null)
              Positioned(
                top: 0,
                right: 0,
                child: Material(
                  color: _surface2,
                  shape: const CircleBorder(),
                  child: InkWell(
                    customBorder: const CircleBorder(),
                    onTap: onClear,
                    child: const Padding(
                      padding: EdgeInsets.all(4),
                      child: Icon(Icons.close_rounded, size: 14, color: _muted),
                    ),
                  ),
                ),
              ),
          ],
        ),
      );

  Widget _imagePickerRow(
    String? currentPath, {
    required VoidCallback onPick,
    required VoidCallback onClear,
  }) =>
      _Glass(
        radius: 14,
        tint: _surface2,
        opacity: .4,
        borderAlpha: .1,
        padding: const EdgeInsets.all(10),
        child: Row(
          children: [
            Container(
              width: 56,
              height: 56,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(12),
                color: _surface,
              ),
              child: currentPath == null
                  ? const Icon(Icons.image_outlined, color: _muted)
                  : (currentPath.startsWith('assets/')
                      ? Image.asset(currentPath, fit: BoxFit.cover)
                      : Image.file(File(currentPath), fit: BoxFit.cover)),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    'Imagen de la moto (PNG)',
                    style: TextStyle(fontSize: 12, fontWeight: FontWeight.w700),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    currentPath == null
                        ? 'Opcional. Se asigna automáticamente para Yamaha R15.'
                        : 'Imagen seleccionada',
                    style: const TextStyle(color: _muted, fontSize: 10),
                  ),
                ],
              ),
            ),
            IconButton(
              tooltip: 'Seleccionar PNG',
              onPressed: onPick,
              icon: const Icon(Icons.upload_file_rounded, color: _red),
            ),
            if (currentPath != null)
              IconButton(
                tooltip: 'Quitar imagen',
                onPressed: onClear,
                icon: const Icon(Icons.close_rounded, color: _muted),
              ),
          ],
        ),
      );

  Widget _formDialog({
    required String title,
    required GlobalKey<FormState> formKey,
    required List<Widget> fields,
    required VoidCallback onSubmit,
  }) => _GlassDialog(
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
        builder: (context) => _GlassDialog(
          title: Text(title),
          content: Text(body),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('Cancelar'),
            ),
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

class _FuelVoiceDialog extends StatefulWidget {
  const _FuelVoiceDialog({
    required this.vehicle,
    required this.defaultPrice,
    required this.voiceController,
    this.startVoiceImmediately = false,
  });

  final Vehicle vehicle;
  final double defaultPrice;
  final MaintenanceVoiceController voiceController;
  final bool startVoiceImmediately;

  @override
  State<_FuelVoiceDialog> createState() => _FuelVoiceDialogState();
}

class _FuelVoiceDialogState extends State<_FuelVoiceDialog> {
  final _formKey = GlobalKey<FormState>();
  final _amount = TextEditingController();
  late final TextEditingController _price = TextEditingController(
    text: widget.defaultPrice.round().toString(),
  );
  late final TextEditingController _km = TextEditingController(
    text: '${widget.vehicle.km}',
  );
  final _station = TextEditingController();
  DateTime _fuelDate = DateTime.now();
  _VoiceState _voiceState = _VoiceState.idle;
  String _voiceMessage =
      'Ejemplo: hoy pagué veinte mil pesos a 12.500 por galón a los 15.000 km';

  @override
  void initState() {
    super.initState();
    if (widget.startVoiceImmediately) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) unawaited(_toggleDictation());
      });
    }
  }

  @override
  void dispose() {
    widget.voiceController.clearListeners();
    if (widget.voiceController.isListening) {
      unawaited(widget.voiceController.cancel());
    }
    _amount.dispose();
    _price.dispose();
    _km.dispose();
    _station.dispose();
    super.dispose();
  }

  Future<void> _toggleDictation() async {
    if (_voiceState == _VoiceState.listening) {
      await widget.voiceController.stop();
      return;
    }
    setState(() {
      _voiceState = _VoiceState.processing;
      _voiceMessage = 'Preparando el dictado...';
    });
    try {
      final available = await widget.voiceController.initialize(
        onStatus: (_) {},
        onError: (error) {
          if (!mounted) return;
          setState(() {
            _voiceState = _VoiceState.error;
            _voiceMessage = 'No se pudo escuchar: $error';
          });
        },
      );
      if (!available) {
        setState(() {
          _voiceState = _VoiceState.error;
          _voiceMessage = 'El reconocimiento de voz no está disponible.';
        });
        return;
      }
      final localeId = await widget.voiceController.spanishLocaleId();
      if (localeId == null) {
        setState(() {
          _voiceState = _VoiceState.error;
          _voiceMessage =
              'Instala o activa el reconocimiento de voz en español.';
        });
        return;
      }
      setState(() {
        _voiceState = _VoiceState.listening;
        _voiceMessage = 'Escuchando combustible...';
      });
      await widget.voiceController.listen(
        localeId: localeId,
        onResult: (words, isFinal) {
          if (isFinal) _applyTranscript(words);
        },
      );
    } catch (error) {
      setState(() {
        _voiceState = _VoiceState.error;
        _voiceMessage = 'No se pudo iniciar el dictado: $error';
      });
    }
  }

  void _applyTranscript(String words) {
    final draft = parseFuelVoiceText(words);
    if (draft.amount != null) {
      _amount.text = draft.amount!.toStringAsFixed(
        draft.amount! % 1 == 0 ? 0 : 2,
      );
    }
    if (draft.pricePerGallon != null) {
      _price.text = draft.pricePerGallon!.toStringAsFixed(
        draft.pricePerGallon! % 1 == 0 ? 0 : 2,
      );
    }
    if (draft.kilometers != null) _km.text = '${draft.kilometers}';
    if (draft.station != null) _station.text = draft.station!;
    setState(() {
      _fuelDate = draft.date;
      _voiceState = draft.amount == null
          ? _VoiceState.error
          : _VoiceState.ready;
      _voiceMessage = draft.amount == null
          ? 'No se detectó el monto. Revisa o vuelve a dictar.'
          : 'Combustible interpretado. Revisa y guarda.';
    });
  }

  void _save() {
    if (!_formKey.currentState!.validate()) return;
    final amount = double.tryParse(_amount.text.replaceAll(',', '.')) ?? 0;
    final price = double.tryParse(_price.text.replaceAll(',', '.')) ?? 0;
    final odometer = int.tryParse(_km.text) ?? -1;
    if (amount <= 0 || price <= 0 || odometer < widget.vehicle.km) {
      setState(
        () => _voiceMessage = 'El monto y el precio deben ser positivos y el kilometraje no puede retroceder.',
      );
      return;
    }
    Navigator.of(context).pop(
      FuelEntry(
        vehicleId: widget.vehicle.id,
        date:
            '${_fuelDate.year}-${_fuelDate.month.toString().padLeft(2, '0')}-${_fuelDate.day.toString().padLeft(2, '0')}',
        station: _station.text.trim(),
        amount: amount,
        pricePerGallon: price,
        km: odometer,
      ),
    );
  }

  Future<void> _pickFuelDate() async {
    final selected = await showDatePicker(
      context: context,
      initialDate: _fuelDate,
      firstDate: DateTime(2000),
      lastDate: DateTime(2200),
    );
    if (selected != null) setState(() => _fuelDate = selected);
  }

  Widget _fuelField(
    TextEditingController controller,
    String label, {
    bool numeric = false,
    bool required = false,
  }) => Padding(
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

  @override
  Widget build(BuildContext context) => _GlassDialog(
    title: const Text('Registrar combustible'),
    content: SizedBox(
      width: double.maxFinite,
      child: Form(
        key: _formKey,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              _Glass(
                radius: 16,
                tint: _surface2,
                opacity: .55,
                borderAlpha: .12,
                padding: const EdgeInsets.all(12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Registro por voz',
                      style: TextStyle(fontWeight: FontWeight.w700),
                    ),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            _voiceMessage,
                            style: const TextStyle(color: _muted, fontSize: 12),
                          ),
                        ),
                        IconButton.filled(
                          tooltip: 'Dictar por voz',
                          onPressed: _toggleDictation,
                          icon: Icon(
                            _voiceState == _VoiceState.listening
                                ? Icons.stop_rounded
                                : Icons.mic_rounded,
                          ),
                          style: IconButton.styleFrom(backgroundColor: _red),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 14),
              _fuelField(
                _amount,
                'Total pagado (COP)',
                numeric: true,
                required: true,
              ),
              _fuelField(
                _price,
                'Precio por galón',
                numeric: true,
                required: true,
              ),
              _fuelField(
                _km,
                'Kilometraje del tablero',
                numeric: true,
                required: true,
              ),
              _fuelField(_station, 'Estación (opcional)'),
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.calendar_month_rounded, color: _red),
                title: const Text('Fecha de la carga'),
                subtitle: Text(
                  _dateLabel(
                    '${_fuelDate.year}-${_fuelDate.month.toString().padLeft(2, '0')}-${_fuelDate.day.toString().padLeft(2, '0')}',
                  ),
                ),
                onTap: _pickFuelDate,
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

}
class _MaintenanceServiceDialog extends StatefulWidget {
  const _MaintenanceServiceDialog({
    required this.vehicle,
    required this.voiceController,
    this.startVoiceImmediately = false,
  });

  final Vehicle vehicle;
  final MaintenanceVoiceController voiceController;
  final bool startVoiceImmediately;

  @override
  State<_MaintenanceServiceDialog> createState() =>
      _MaintenanceServiceDialogState();
}

class _MaintenanceServiceDialogState extends State<_MaintenanceServiceDialog> {
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
  bool _isPeriodic = true;

  @override
  void initState() {
    super.initState();
    if (widget.startVoiceImmediately) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) unawaited(_toggleDictation());
      });
    }
  }

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
          _voiceMessage = 'No se concedió el permiso o el reconocimiento de voz no está disponible. Revisa los permisos en Ajustes.';
        });
        return;
      }

      final localeId = await widget.voiceController.spanishLocaleId();
      if (!mounted) return;
      if (localeId == null) {
        setState(() {
          _voiceState = _VoiceState.error;
          _voiceMessage = 'El dispositivo no tiene reconocimiento de voz en español disponible.';
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
    if (draft.cost != null) {
      _cost.text = draft.cost!.toStringAsFixed(draft.cost! % 1 == 0 ? 0 : 2);
    }
    _isPeriodic = draft.isPeriodic;
    if (draft.intervalSuggested != null) {
      _interval.text = '${draft.intervalSuggested}';
    } else if (!_isPeriodic) {
      _interval.text = '';
    } else if (_interval.text.trim().isEmpty) {
      _interval.text = '3000';
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
    final isPermissionError =
        normalized.contains('permission') ||
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
      _voiceState = permissionDenied
          ? _VoiceState.permissionDenied
          : _VoiceState.error;
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
    final name = _name.text.trim();
    final last = int.tryParse(_lastKm.text) ?? -1;
    if (name.isEmpty) {
      setState(() {
        _formError = 'Describe el trabajo realizado antes de guardar.';
      });
      return;
    }
    if (last < 0 || last > widget.vehicle.km) {
      setState(() {
        _formError =
            'El kilometraje no puede superar los ${widget.vehicle.km} km registrados en la moto.';
      });
      return;
    }
    var interval = 0;
    if (_isPeriodic) {
      interval = int.tryParse(_interval.text) ?? 0;
      if (interval <= 0) interval = 3000;
    }
    Navigator.of(context).pop(
      ServiceEntry(
        vehicleId: widget.vehicle.id,
        name: _name.text.trim(),
        intervalKm: interval,
        lastKm: last,
        cost: double.tryParse(_cost.text.replaceAll(',', '.')) ?? 0,
        date: _dateIso(_serviceDate),
      ),
    );
  }

  String _dateIso(DateTime date) =>
      '${date.year}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}';

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
  Widget build(BuildContext context) => _GlassDialog(
    title: const Text('Registrar mantenimiento'),
    content: SizedBox(
      width: double.maxFinite,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _voicePanel(),
            const SizedBox(height: 16),
            // Periodic toggle
            _Glass(
              radius: 16,
              tint: _surface2,
              opacity: .45,
              borderAlpha: .10,
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
              child: Row(
                children: [
                  const Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Mantenimiento periódico',
                          style: TextStyle(fontWeight: FontWeight.w700),
                        ),
                        SizedBox(height: 2),
                        Text(
                          'Activa para definir un intervalo de km.',
                          style: TextStyle(color: _muted, fontSize: 11),
                        ),
                      ],
                    ),
                  ),
                  Switch(
                    value: _isPeriodic,
                    onChanged: (value) => setState(() {
                      _isPeriodic = value;
                      if (!value) {
                        _interval.text = '';
                      } else if (_interval.text.trim().isEmpty) {
                        _interval.text = '3000';
                      }
                    }),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 14),
            Text(
              'Datos reconocidos (revisa y corrige)',
              style: const TextStyle(
                color: _muted,
                fontSize: 11,
                fontWeight: FontWeight.w700,
                letterSpacing: .6,
              ),
            ),
            const SizedBox(height: 8),
            _previewField(_name, 'Trabajo realizado', required: true),
            _previewField(
              _lastKm,
              'Kilometraje del servicio',
              numeric: true,
            ),
            _previewField(_cost, 'Costo del servicio (opcional)', numeric: true),
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: const Icon(Icons.calendar_month_rounded, color: _red),
              title: const Text('Fecha del mantenimiento'),
              subtitle: Text(_dateLabel(_serviceDate.toIso8601String())),
              onTap: _pickDate,
            ),
            if (_isPeriodic)
              _previewField(
                _interval,
                'Intervalo de mantenimiento (km)',
                numeric: true,
              )
            else
              _Glass(
                radius: 16,
                tint: _surface2,
                opacity: .35,
                borderAlpha: .08,
                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                child: const Row(
                  children: [
                    Icon(Icons.event_repeat_rounded, color: _muted, size: 18),
                    SizedBox(width: 10),
                    Expanded(
                      child: Text(
                        'Intervalo: No aplica / Eventual',
                        style: TextStyle(color: _muted, fontSize: 12),
                      ),
                    ),
                  ],
                ),
              ),
            if (_formError != null)
              Align(
                alignment: Alignment.centerLeft,
                child: Padding(
                  padding: const EdgeInsets.only(top: 8, bottom: 4),
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
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('Cancelar'),
      ),
      FilledButton(onPressed: _save, child: const Text('Guardar')),
    ],
  );

  Widget _previewField(
    TextEditingController controller,
    String label, {
    bool numeric = false,
    bool required = false,
  }) => Padding(
    padding: const EdgeInsets.only(bottom: 10),
    child: TextField(
      controller: controller,
      keyboardType: numeric
          ? const TextInputType.numberWithOptions(decimal: true)
          : TextInputType.text,
      textCapitalization: TextCapitalization.sentences,
      decoration: InputDecoration(
        labelText: label,
        filled: true,
        fillColor: Colors.white.withValues(alpha: .04),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(14),
          borderSide: BorderSide(color: Colors.white.withValues(alpha: .10)),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(14),
          borderSide: BorderSide(color: Colors.white.withValues(alpha: .10)),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(14),
          borderSide: const BorderSide(color: _red, width: 1.1),
        ),
      ),
    ),
  );

  Widget _voicePanel() => _Glass(
    radius: 18,
    tint: _surface2,
    opacity: .55,
    borderAlpha: .12,
    padding: const EdgeInsets.all(12),
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
        Text(_voiceMessage, style: TextStyle(color: _voiceColor, fontSize: 12)),
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
  Widget build(BuildContext context) => _Glass(
    radius: 22,
    tint: _surface,
    opacity: .56,
    borderAlpha: .13,
    highlight: .09,
    padding: padding,
    child: child,
  );
}

/// A soft, frosted surface inspired by iOS 26 Liquid Glass: a translucent
/// tint, a backdrop blur, a rim light and a diagonal specular sheen.
class _Glass extends StatelessWidget {
  const _Glass({
    required this.child,
    this.radius = 24,
    this.blur = 0,
    this.tint = _surface,
    this.opacity = .58,
    this.padding = EdgeInsets.zero,
    this.borderAlpha = .14,
    this.highlight = .10,
    this.shadow = false,
    this.glow,
  });

  final Widget child;
  final double radius;
  final double blur;
  final Color tint;
  final double opacity;
  final EdgeInsetsGeometry padding;
  final double borderAlpha;
  final double highlight;
  final bool shadow;
  final Color? glow;

  @override
  Widget build(BuildContext context) {
    final borderRadius = BorderRadius.circular(radius);
    final midTint = Color.lerp(tint, Colors.white, .03) ?? tint;

    Widget surface = DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: borderRadius,
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            Colors.white.withValues(alpha: highlight),
            midTint.withValues(alpha: opacity),
            tint.withValues(alpha: opacity * .82),
          ],
          stops: const [0, .38, 1],
        ),
        border: Border.all(
          color: Colors.white.withValues(alpha: borderAlpha),
          width: .8,
        ),
      ),
      child: Stack(
        children: [
          Positioned.fill(
            child: IgnorePointer(
              child: DecoratedBox(
                decoration: BoxDecoration(
                  borderRadius: borderRadius,
                  gradient: LinearGradient(
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                    colors: [
                      Colors.white.withValues(alpha: .10),
                      Colors.transparent,
                      Colors.white.withValues(alpha: .025),
                    ],
                    stops: const [0, .45, 1],
                  ),
                ),
              ),
            ),
          ),
          Positioned(
            top: 0,
            left: radius * .8,
            right: radius * .8,
            child: Container(
              height: .9,
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  colors: [
                    Colors.transparent,
                    Colors.white.withValues(alpha: .45),
                    Colors.transparent,
                  ],
                ),
              ),
            ),
          ),
          Padding(padding: padding, child: child),
        ],
      ),
    );

    if (blur > 0) {
      surface = BackdropFilter(
        filter: ImageFilter.blur(sigmaX: blur, sigmaY: blur),
        child: surface,
      );
    }
    surface = ClipRRect(borderRadius: borderRadius, child: surface);

    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: borderRadius,
        boxShadow: [
          if (shadow)
            BoxShadow(
              color: Colors.black.withValues(alpha: .48),
              blurRadius: 28,
              offset: const Offset(0, 14),
              spreadRadius: -6,
            ),
          if (glow != null)
            BoxShadow(
              color: glow!.withValues(alpha: .20),
              blurRadius: 30,
              offset: const Offset(0, 10),
              spreadRadius: -10,
            ),
        ],
      ),
      child: surface,
    );
  }
}

class _AmbientBackground extends StatelessWidget {
  const _AmbientBackground();

  @override
  Widget build(BuildContext context) => IgnorePointer(
    child: DecoratedBox(
      decoration: const BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [Color(0xFF0C1016), Color(0xFF07090D), Color(0xFF05070A)],
          stops: [0, .42, 1],
        ),
      ),
      child: Stack(
        children: [
          Positioned(
            top: -190,
            right: -130,
            child: _Glow(color: _red.withValues(alpha: .30), size: 420),
          ),
          Positioned(
            top: 150,
            left: -190,
            child: _Glow(
              color: const Color(0xFF2F6BFF).withValues(alpha: .13),
              size: 380,
            ),
          ),
          Positioned(
            bottom: -180,
            right: -120,
            child: _Glow(
              color: const Color(0xFF8E2DE2).withValues(alpha: .10),
              size: 380,
            ),
          ),
          Positioned(
            bottom: 60,
            left: -120,
            child: _Glow(
              color: const Color(0xFF00C2A8).withValues(alpha: .07),
              size: 320,
            ),
          ),
        ],
      ),
    ),
  );
}

class _Glow extends StatelessWidget {
  const _Glow({required this.color, required this.size});

  final Color color;
  final double size;

  @override
  Widget build(BuildContext context) => Container(
    width: size,
    height: size,
    decoration: BoxDecoration(
      shape: BoxShape.circle,
      gradient: RadialGradient(colors: [color, color.withValues(alpha: 0)]),
    ),
  );
}

class _LiquidNavBar extends StatelessWidget {
  const _LiquidNavBar({
    required this.index,
    required this.tabs,
    required this.onSelected,
  });

  final int index;
  final List<(IconData, String)> tabs;
  final ValueChanged<int> onSelected;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 0, 16, 10),
    child: SafeArea(
      top: false,
      minimum: const EdgeInsets.only(bottom: 2),
      child: Center(
        heightFactor: 1,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: _Glass(
            radius: 34,
            blur: 30,
            tint: const Color(0xFF0A0D12),
            opacity: .60,
            borderAlpha: .20,
            highlight: .13,
            shadow: true,
            glow: _red,
            child: Padding(
              padding: const EdgeInsets.all(6),
              child: Row(
                children: [
                  for (var i = 0; i < tabs.length; i++)
                    Expanded(
                      child: _NavItem(
                        key: ValueKey('nav-${tabs[i].$2}'),
                        icon: tabs[i].$1,
                        label: tabs[i].$2,
                        selected: i == index,
                        onTap: () => onSelected(i),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

class _NavItem extends StatelessWidget {
  const _NavItem({
    super.key,
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Semantics(
    button: true,
    selected: selected,
    label: label,
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () {
        HapticFeedback.selectionClick();
        onTap();
      },
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 320),
        curve: Curves.easeOutCubic,
        height: 54,
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(27),
          gradient: selected
              ? LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [
                    _red.withValues(alpha: .30),
                    _red.withValues(alpha: .10),
                  ],
                )
              : null,
          border: Border.all(
            color: selected ? _red.withValues(alpha: .42) : Colors.transparent,
          ),
          boxShadow: selected
              ? [
                  BoxShadow(
                    color: _red.withValues(alpha: .22),
                    blurRadius: 18,
                    offset: const Offset(0, 6),
                    spreadRadius: -4,
                  ),
                ]
              : null,
        ),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            ExcludeSemantics(child: Icon(icon, size: 20, color: selected ? Colors.white : _muted)),
            const SizedBox(height: 2),
            Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 9.5,
                letterSpacing: .1,
                fontWeight: selected ? FontWeight.w700 : FontWeight.w600,
                color: selected ? Colors.white : _muted,
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class _LiquidFab extends StatelessWidget {
  const _LiquidFab({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Semantics(
    button: true,
    label: label,
    child: _Glass(
      radius: 28,
      blur: 24,
      tint: _red,
      opacity: .34,
      borderAlpha: .34,
      highlight: .18,
      glow: _red,
      shadow: true,
      child: Material(
        type: MaterialType.transparency,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(28),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 14),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(icon, size: 19, color: Colors.white),
                const SizedBox(width: 8),
                Text(
                  label,
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 13.5,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    ),
  );
}

class _GlassDialog extends StatelessWidget {
  const _GlassDialog({this.title, this.content, this.actions});

  final Widget? title;
  final Widget? content;
  final List<Widget>? actions;

  @override
  Widget build(BuildContext context) {
    final maxHeight = MediaQuery.sizeOf(context).height * .78;
    final buttons = actions;
    return Dialog(
      backgroundColor: Colors.transparent,
      elevation: 0,
      insetPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 28),
      child: SizedBox(
        width: double.maxFinite,
        child: ConstrainedBox(
          constraints: BoxConstraints(maxWidth: 440, maxHeight: maxHeight),
          child: _Glass(
            radius: 30,
            blur: 32,
            tint: const Color(0xFF0A0D12),
            opacity: .78,
            borderAlpha: .20,
            highlight: .14,
            shadow: true,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(20, 20, 20, 14),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (title != null)
                    DefaultTextStyle(
                      style: const TextStyle(
                        color: Color(0xFFF5F7FA),
                        fontSize: 20,
                        fontWeight: FontWeight.w700,
                        letterSpacing: -.3,
                      ),
                      child: title!,
                    ),
                  if (title != null) const SizedBox(height: 16),
                  if (content != null)
                    Flexible(
                      child: SingleChildScrollView(
                        physics: const BouncingScrollPhysics(),
                        child: content!,
                      ),
                    ),
                  if (buttons != null && buttons.isNotEmpty) ...[
                    const SizedBox(height: 10),
                    Wrap(
                      alignment: WrapAlignment.end,
                      spacing: 4,
                      runSpacing: 4,
                      children: buttons,
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _HeroGridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white.withValues(alpha: 0.045)
      ..strokeWidth = 0.7;
    for (var i = 0; i < 6; i++) {
      final x = size.width * (0.08 + i * 0.16);
      canvas.drawLine(
        Offset(x, size.height),
        Offset(size.width * (0.43 + i * 0.035), 0),
        paint,
      );
    }
    for (var i = 1; i < 5; i++) {
      final y = size.height * i / 5;
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
