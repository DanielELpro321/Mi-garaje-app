import 'package:speech_to_text/speech_recognition_result.dart';
import 'package:speech_to_text/speech_to_text.dart';

class MaintenanceVoiceController {
  final SpeechToText _speech = SpeechToText();
  bool _initialized = false;
  bool _available = false;
  void Function(String)? _statusListener;
  void Function(String)? _errorListener;

  bool get isListening => _speech.isListening;

  Future<bool> initialize({
    required void Function(String) onStatus,
    required void Function(String) onError,
  }) async {
    _statusListener = onStatus;
    _errorListener = onError;
    if (_initialized) return _available;

    _available = await _speech.initialize(
      onStatus: (status) => _statusListener?.call(status),
      onError: (error) => _errorListener?.call(error.errorMsg),
    );
    _initialized = true;
    return _available;
  }

  Future<String?> spanishLocaleId() async {
    final systemLocale = await _speech.systemLocale();
    if (systemLocale?.localeId.toLowerCase().startsWith('es') ?? false) {
      return systemLocale!.localeId;
    }

    for (final locale in await _speech.locales()) {
      if (locale.localeId.toLowerCase().startsWith('es')) {
        return locale.localeId;
      }
    }
    return null;
  }

  Future<void> listen({
    required String localeId,
    required void Function(String words, bool isFinal) onResult,
  }) => _speech.listen(
    onResult: (SpeechRecognitionResult result) =>
        onResult(result.recognizedWords, result.finalResult),
    listenOptions: SpeechListenOptions(
      localeId: localeId,
      listenFor: const Duration(seconds: 45),
      pauseFor: const Duration(seconds: 4),
      listenMode: ListenMode.dictation,
      partialResults: true,
      cancelOnError: true,
    ),
  );

  Future<void> stop() => _speech.stop();

  Future<void> cancel() => _speech.cancel();

  void clearListeners() {
    _statusListener = null;
    _errorListener = null;
  }
}

class MaintenanceVoiceDraft {
  const MaintenanceVoiceDraft({
    required this.description,
    required this.kilometers,
    required this.date,
    required this.cost,
    this.isPeriodic = false,
    this.intervalSuggested,
  });

  final String description;
  final int? kilometers;
  final DateTime date;
  final double? cost;
  final bool isPeriodic;
  final int? intervalSuggested;
}

class FuelVoiceDraft {
  const FuelVoiceDraft({
    required this.amount,
    required this.kilometers,
    required this.date,
    required this.pricePerGallon,
    required this.station,
  });

  final double? amount;
  final int? kilometers;
  final DateTime date;
  final double? pricePerGallon;
  final String? station;
}

const _spanishNumberWordPattern =
    r'(?:cero|un|uno|una|dos|tres|cuatro|cinco|seis|siete|ocho|nueve|'
    r'diez|once|doce|trece|catorce|quince|dieciseis|diecisiete|'
    r'dieciocho|diecinueve|veinte|veintiun|veintiuno|veintiuna|'
    r'veintidos|veintitres|veinticuatro|veinticinco|veintiseis|'
    r'veintisiete|veintiocho|veintinueve|treinta|cuarenta|cincuenta|'
    r'sesenta|setenta|ochenta|noventa|cien|ciento|doscientos|'
    r'trescientos|cuatrocientos|quinientos|seiscientos|setecientos|'
    r'ochocientos|novecientos|mil|y)';

MaintenanceVoiceDraft parseMaintenanceVoiceText(String text, {DateTime? now}) {
  final today = now ?? DateTime.now();
  final normalized = _fold(text);
  final costMatch = RegExp(
    r'\b(?:me\s+)?(?:cost[oó]|pagu[eé]|vale|por)\s*(?:cop|\$|pesos?)?\s*'
    r'((?:\d+(?:[.,]\d{1,3})?\s*(?:mil|millones?)?|'
    r'(?:veinte|treinta|cuarenta|cincuenta|sesenta|setenta|ochenta|'
    r'noventa|ciento|cien)\s+(?:mil|pesos)'
    r'|(?:cero|un|uno|una|dos|tres|cuatro|cinco|seis|siete|ocho|nueve|'
    r'diez|once|doce|trece|catorce|quince|diecis[eé]is|diecisiete|'
    r'dieciocho|diecinueve|veinte|treinta|cuarenta|cincuenta|'
    r'sesenta|setenta|ochenta|noventa|cien|mil)\b))',
    caseSensitive: false,
  ).firstMatch(normalized);
  final cost = costMatch == null
      ? null
      : _parseVoiceAmount(costMatch.group(1)!);
  final numericKm = RegExp(
    r'\b(\d+(?:[.,\s]\d{3})*|\d+\s+mil(?:\s+\d{1,3})?)\s*(?:km|kilometros?)\b',
  ).firstMatch(normalized);
  final spokenKm = RegExp(
    r'\b((' +
        _spanishNumberWordPattern +
        r')(?:\s+' +
        _spanishNumberWordPattern +
        r'){0,5})\s*(?:km|kilometros?)\b',
  ).firstMatch(normalized);
  final kmMatch = _earliestMatch(numericKm, spokenKm);
  final kilometers = kmMatch == null
      ? null
      : _parseSpanishNumber(kmMatch.group(1)!);

  final date = _extractDate(normalized, today);
  var description = text;
  final costStart = RegExp(
    r'\b(?:me\s+)?(?:cost[oó]|pagu[eé]|vale)\b',
    caseSensitive: false,
  ).firstMatch(description);
  if (costStart != null) {
    description = description.substring(0, costStart.start);
  }
  if (spokenKm != null) {
    final conIndex = description.toLowerCase().lastIndexOf(' con ');
    if (conIndex >= 0) {
      description = description.substring(0, conIndex);
    }
  }
  description = description.replaceAll(
    RegExp(r'\s+con\s+.*\b(?:km|kilometros?)\b.*$', caseSensitive: false),
    '',
  );
  description = description.replaceAll(
    RegExp(
      r'\s+(?:con|a\s+los|a)\s+.*(?:km|kilometros?)\b.*$',
      caseSensitive: false,
    ),
    '',
  );
  description = description
      .replaceAll(
        RegExp(r'\bcon\s+.+?\s+kilometros?\b', caseSensitive: false),
        ' ',
      )
      .replaceAll(
        RegExp(r'\b(?:a\s+los|a|con)\s+.+?\s+km\b', caseSensitive: false),
        ' ',
      )
      .replaceAll(
        RegExp(
          r'\bcon\s+(?:un\s+|una\s+|dos\s+|tres\s+|cuatro\s+|cinco\s+|seis\s+|siete\s+|ocho\s+|nueve\s+|diez\s+|once\s+|doce\s+|trece\s+|catorce\s+|quince\s+|diecisiete\s+|dieciocho\s+|diecinueve\s+|veinte\s+|treinta\s+|cuarenta\s+|cincuenta\s+|sesenta\s+|setenta\s+|ochenta\s+|noventa\s+|cien\s+|ciento\s+)?(?:mil\s+)?(?:kilometros?|km)\b',
          caseSensitive: false,
        ),
        ' ',
      )
      .replaceAll(
        RegExp(
          r'\b(?:con\s+)?quince\s+mil\s*(?:kilometros?|km)\b',
          caseSensitive: false,
        ),
        ' ',
      )
      .replaceAll(RegExp(r'\s*,\s*pesos\b', caseSensitive: false), '')
      .replaceAll(
        RegExp(
          r'\b(?:me\s+)?(?:cost[oó]|pagu[eé]|vale)\b.*$',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(
          r'\b(?:a\s+los|con|a)\s+(?:[\w.,]+\s*){0,8}(?:km|kilometros?)\b',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(
          r'\b(?:\d+(?:[.,\s]\d{3})*|\d+\s+mil(?:\s+\d{1,3})?)\s*'
          r'(?:km|kilometros?)\b',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(
          r'\b(?:con\s+)?(?:' +
              _spanishNumberWordPattern +
              r')(?:\s+' +
              _spanishNumberWordPattern +
              r'){0,5}\s*(?:km|kilometros?)\b',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(
          r'\bcon\s+(?:[a-z]+\s*)+\b(?:km|kilometros?)\b',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(RegExp(r'\s*,\s*pesos\b', caseSensitive: false), '');
  description = description
      .replaceAll(
        RegExp(
          r'\b(?:me\s+)?(?:cost[oó]|pagu[eé]|vale)\b.*$',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(
          r'\b(?:hoy|ayer|anteayer)\b|'
          r'\b(?:el\s+)?\d{4}-\d{1,2}-\d{1,2}\b|'
          r'\b(?:el\s+)?\d{1,2}[/-]\d{1,2}(?:[/-]\d{2,4})?\b|'
          r'\b(?:el\s+)?\d{1,2}\s+de\s+'
          r'(?:enero|febrero|marzo|abril|mayo|junio|julio|agosto|'
          r'septiembre|setiembre|octubre|noviembre|diciembre)'
          r'(?:\s+de\s+\d{4})?\b',
          caseSensitive: false,
        ),
        ' ',
      )
      .replaceAll(
        RegExp(
          r'\b(?:me\s+)?(?:cost[oó]|pagu[eé]|vale)\s*(?:cop|\$|pesos?)?\s*'
          r'\d+(?:[.,]\d{1,3})?\s*(?:mil|millones?)?|'
          r'\b(?:me\s+)?(?:cost[oó]|pagu[eé]|vale)\s*(?:cop|\$|pesos?)?\s*'
          r'(?:veinte|treinta|cuarenta|cincuenta|sesenta|setenta|ochenta|'
          r'noventa|cien|ciento)\s+(?:mil|pesos)',
          caseSensitive: false,
        ),
        ' ',
      )
      .replaceAll(
        RegExp(
          r'\b(?:me\s+)?(?:cost[oó]|pagu[eé]|vale)\b.*$',
          caseSensitive: false,
        ),
        '',
      )
      .replaceFirst(
        RegExp(
          r'^\s*(?:(?:el|la)\s+)?(?:hice|realic[eé]|registr[eé]|'
          r'anot[eé]|se\s+(?:hizo|realiz[oó]))\s+',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(RegExp(r'\s+'), ' ')
      .replaceAll(RegExp(r'^[\s,.;:–-]+|[\s,.;:–-]+$'), '');
  description = _removeTrailingFiller(description);

  final intervalSuggested = _extractInterval(normalized);
  final periodic = _isPeriodicMaintenance(description, intervalSuggested);

  return MaintenanceVoiceDraft(
    description: description.trim(),
    kilometers: kilometers,
    date: date,
    cost: cost,
    isPeriodic: periodic,
    intervalSuggested: intervalSuggested,
  );
}

FuelVoiceDraft parseFuelVoiceText(String text, {DateTime? now}) {
  final today = now ?? DateTime.now();
  final normalized = _fold(text);
  final date = _extractDate(normalized, today);

  final numericKm = RegExp(
    r'\b(\d+(?:[.,\s]\d{3})*|\d+\s+mil(?:\s+\d{1,3})?)\s*(?:km|kilometros?)\b',
  ).firstMatch(normalized);
  final spokenKm = RegExp(
    r'\b((' +
        _spanishNumberWordPattern +
        r')(?:\s+' +
        _spanishNumberWordPattern +
        r'){0,5})\s*(?:km|kilometros?)\b',
  ).firstMatch(normalized);
  final kmMatch = _earliestMatch(numericKm, spokenKm);
  final kilometers = kmMatch == null
      ? null
      : _parseSpanishNumber(kmMatch.group(1)!);

  final amountMatch = RegExp(
    r'\b(?:le\s+)?(?:eche|eche|ech[eé]|pagu[eé]|cargu[eé]|gasolina\s+por|'
    r'por)\s*(?:cop|\$|pesos?)?\s*'
    r'((?:\d+(?:[.,]\d{1,3})?\s*(?:mil|millones?)?)|'
    r'(?:(?:veinte|treinta|cuarenta|cincuenta|sesenta|setenta|ochenta|noventa|'
    r'ciento|cien)\s+(?:mil|pesos)|'
    r'(?:cero|un|uno|una|dos|tres|cuatro|cinco|seis|siete|ocho|nueve|'
    r'diez|once|doce|trece|catorce|quince|diecisiete|dieciocho|diecinueve)'
    r'(?:\s+(?:mil|pesos))?))',
    caseSensitive: false,
  ).firstMatch(normalized);
  final amount = amountMatch == null
      ? null
      : _parseVoiceAmount(amountMatch.group(1)!);

  final priceMatch = RegExp(
    r'(?:precio\s+(?:de\s+)?(?:galon|gal[oó]n)|a\s+)\s*(?:de\s+)?'
    r'(?:cop|\$|pesos?)?\s*(\d+(?:[.,]\d{1,3})?)',
    caseSensitive: false,
  ).firstMatch(normalized);
  final price = priceMatch == null
      ? null
      : _parseVoiceAmount(priceMatch.group(1)!);

  final stationMatch = RegExp(
    r'\b(?:en|de)\s+(?:la\s+)?(?:estaci[oó]n|gasolinera)\s+'
    r'([a-z0-9\s]+)',
    caseSensitive: false,
  ).firstMatch(text);
  var station = stationMatch?.group(1)?.trim();
  if (station != null) {
    station = station
        .replaceAll(
          RegExp(
            r'\b(?:hoy|ayer|a|los|kilometros?|km)\b.*$',
            caseSensitive: false,
          ),
          '',
        )
        .trim();
  }

  return FuelVoiceDraft(
    amount: amount,
    kilometers: kilometers,
    date: date,
    pricePerGallon: price,
    station: station?.isEmpty == true ? null : station,
  );
}

double? _parseVoiceAmount(String value) {
  final normalized = _fold(value).trim();
  final spokenNormalized = normalized
      .replaceAll(RegExp(r'\b(?:pesos|cop|por galon|galon)\b'), '')
      .replaceAll(RegExp(r'\s+'), ' ')
      .trim();
  final grouped = RegExp(r'^(\d+(?:[.,]\d+)?)\s*(mil|millones?)?$')
      .firstMatch(spokenNormalized);
  if (grouped != null) {
    var numberText = grouped.group(1)!;
    double? number;
    if (numberText.contains('.') &&
        !numberText.contains(',') &&
        RegExp(r'\.\d{3}$').hasMatch(numberText)) {
      number = double.tryParse(numberText.replaceAll('.', ''));
    } else if (numberText.contains(',') && !numberText.contains('.')) {
      number = double.tryParse(numberText.replaceAll(',', '.'));
    } else {
      number = double.tryParse(numberText);
    }
    if (number != null) {
      final suffix = grouped.group(2);
      if (suffix == 'mil') return number * 1000;
      if (suffix == 'millones' || suffix == 'millon') {
        return number * 1000000;
      }
      return number;
    }
  }
  final spoken = _parseSpanishNumber(spokenNormalized);
  if (spoken != null) return spoken.toDouble();

  final cleaned = normalized
      .replaceAll('pesos', '')
      .replaceAll('cop', '')
      .trim();
  final numeric = cleaned.replaceAll('.', '').replaceAll(',', '.');
  final parsed = double.tryParse(numeric);
  if (parsed != null) return parsed;
  return null;
}


int? _extractInterval(String normalized) {
  final match = RegExp(
    r'\bcada\s+(\d+(?:[.,]\s?\d{3})*|\d+)\s*(?:mil\s+)?(?:km|kilometros?)\b',
    caseSensitive: false,
  ).firstMatch(normalized);
  if (match != null) {
    final raw = match.group(1)!.replaceAll(RegExp(r'[.,\s]'), '');
    final value = int.tryParse(raw);
    if (value != null) {
      // If "mil" appears after the number, multiply.
      if (RegExp(r'\bcada\s+\d+\s+mil\b').hasMatch(normalized)) {
        return value * 1000;
      }
      return value;
    }
  }
  return null;
}

bool _isPeriodicMaintenance(String description, int? intervalSuggested) {
  final lower = description.toLowerCase();
  const eventualKeywords = [
    'ajuste',
    'modificacion',
    'modificación',
    'accesorio',
    'reparacion',
    'reparación',
    'arreglo',
    'pintura',
    'calcomania',
    'calcomanía',
    'personalizacion',
    'personalización',
    'mod',
  ];
  for (final keyword in eventualKeywords) {
    if (lower.contains(keyword)) return false;
  }
  if (intervalSuggested != null) return true;
  const periodicKeywords = [
    'aceite',
    'filtro',
    'freno',
    'cadena',
    'bujia',
    'bujía',
    'llanta',
    'neumatico',
    'neumático',
    'kit',
    'arrastre',
    'valvula',
    'válvula',
    'refrigerante',
    'coolant',
    'bateria',
    'batería',
    'lubric',
    'revision',
    'revisión',
  ];
  for (final keyword in periodicKeywords) {
    if (lower.contains(keyword)) return true;
  }
  return false;
}

String _fold(String value) => value
    .toLowerCase()
    .replaceAll(RegExp('[áàä]'), 'a')
    .replaceAll(RegExp('[éèë]'), 'e')
    .replaceAll(RegExp('[íìï]'), 'i')
    .replaceAll(RegExp('[óòö]'), 'o')
    .replaceAll(RegExp('[úùü]'), 'u')
    .replaceAll('ñ', 'n');

RegExpMatch? _earliestMatch(RegExpMatch? first, RegExpMatch? second) {
  if (first == null) return second;
  if (second == null) return first;
  return first.start <= second.start ? first : second;
}

String _removeTrailingFiller(String value) {
  var result = value;
  final filler = RegExp(
    r'\b(?:a\s+los|a|con|en|del|de)\s*$',
    caseSensitive: false,
  );
  while (filler.hasMatch(result)) {
    result = result.replaceFirst(filler, '').trimRight();
  }
  return result.replaceAll(RegExp(r'\s+'), ' ').trim();
}

int? _parseSpanishNumber(String value) {
  final normalized = _fold(value).trim();
  if (RegExp(r'^\d[\d.,\s]*$').hasMatch(normalized)) {
    final digits = normalized.replaceAll(RegExp(r'[.,\s]'), '');
    return int.tryParse(digits);
  }

  const small = {
    'cero': 0,
    'un': 1,
    'uno': 1,
    'una': 1,
    'dos': 2,
    'tres': 3,
    'cuatro': 4,
    'cinco': 5,
    'seis': 6,
    'siete': 7,
    'ocho': 8,
    'nueve': 9,
    'diez': 10,
    'once': 11,
    'doce': 12,
    'trece': 13,
    'catorce': 14,
    'quince': 15,
    'dieciseis': 16,
    'diecisiete': 17,
    'dieciocho': 18,
    'diecinueve': 19,
    'veinte': 20,
    'veintiun': 21,
    'veintiuno': 21,
    'veintiuna': 21,
    'veintidos': 22,
    'veintitres': 23,
    'veinticuatro': 24,
    'veinticinco': 25,
    'veintiseis': 26,
    'veintisiete': 27,
    'veintiocho': 28,
    'veintinueve': 29,
    'treinta': 30,
    'cuarenta': 40,
    'cincuenta': 50,
    'sesenta': 60,
    'setenta': 70,
    'ochenta': 80,
    'noventa': 90,
    'cien': 100,
    'ciento': 100,
    'doscientos': 200,
    'trescientos': 300,
    'cuatrocientos': 400,
    'quinientos': 500,
    'seiscientos': 600,
    'setecientos': 700,
    'ochocientos': 800,
    'novecientos': 900,
  };

  var total = 0;
  var current = 0;
  var recognizedNumber = false;
  for (final token in normalized.split(RegExp(r'\s+'))) {
    if (token == 'y') continue;
    if (token == 'mil') {
      total += (current == 0 ? 1 : current) * 1000;
      current = 0;
      recognizedNumber = true;
      continue;
    }
    final number = int.tryParse(token) ?? small[token];
    if (number == null) return null;
    current += number;
    recognizedNumber = true;
  }
  final result = total + current;
  return recognizedNumber && result > 0 ? result : null;
}

DateTime _extractDate(String normalized, DateTime today) {
  if (RegExp(r'\bhoy\b').hasMatch(normalized)) return _dateOnly(today);
  if (RegExp(r'\bayer\b').hasMatch(normalized)) {
    return _dateOnly(today.subtract(const Duration(days: 1)));
  }
  if (RegExp(r'\banteayer\b').hasMatch(normalized)) {
    return _dateOnly(today.subtract(const Duration(days: 2)));
  }

  final iso = RegExp(r'\b(\d{4})-(\d{1,2})-(\d{1,2})\b').firstMatch(normalized);
  if (iso != null) {
    return _validDate(
          int.parse(iso.group(1)!),
          int.parse(iso.group(2)!),
          int.parse(iso.group(3)!),
        ) ??
        _dateOnly(today);
  }

  final named = RegExp(
    r'\b(?:el\s+)?(\d{1,2})\s+de\s+'
    r'(enero|febrero|marzo|abril|mayo|junio|julio|agosto|'
    r'septiembre|setiembre|octubre|noviembre|diciembre)'
    r'(?:\s+de\s+(\d{4}))?\b',
  ).firstMatch(normalized);
  if (named != null) {
    const months = {
      'enero': 1,
      'febrero': 2,
      'marzo': 3,
      'abril': 4,
      'mayo': 5,
      'junio': 6,
      'julio': 7,
      'agosto': 8,
      'septiembre': 9,
      'setiembre': 9,
      'octubre': 10,
      'noviembre': 11,
      'diciembre': 12,
    };
    final month = months[named.group(2)]!;
    final explicitYear = named.group(3);
    final year = explicitYear == null ? today.year : int.parse(explicitYear);
    final candidate = _validDate(year, month, int.parse(named.group(1)!));
    if (candidate == null) return _dateOnly(today);
    if (explicitYear == null && candidate.isAfter(_dateOnly(today))) {
      return _validDate(year - 1, month, candidate.day) ?? _dateOnly(today);
    }
    return candidate;
  }

  final numericDate = RegExp(r'\b(\d{1,2})[/-](\d{1,2})(?:[/-](\d{2,4}))?\b')
      .firstMatch(normalized);
  if (numericDate != null) {
    final explicitYear = numericDate.group(3);
    var year = explicitYear == null ? today.year : int.parse(explicitYear);
    if (year < 100) year += year < 50 ? 2000 : 1900;
    final candidate = _validDate(
      year,
      int.parse(numericDate.group(2)!),
      int.parse(numericDate.group(1)!),
    );
    if (candidate == null) return _dateOnly(today);
    if (explicitYear == null && candidate.isAfter(_dateOnly(today))) {
      return _validDate(year - 1, candidate.month, candidate.day) ??
          _dateOnly(today);
    }
    return candidate;
  }
  return _dateOnly(today);
}

DateTime? _validDate(int year, int month, int day) {
  if (month < 1 || month > 12 || day < 1 || day > 31) return null;
  final date = DateTime(year, month, day);
  if (date.year != year || date.month != month || date.day != day) return null;
  return _dateOnly(date);
}

DateTime _dateOnly(DateTime date) => DateTime(date.year, date.month, date.day);
