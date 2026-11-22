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
  });

  final String description;
  final int? kilometers;
  final DateTime date;
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
  if (kmMatch != null) {
    description = _replaceRangeWithSpace(
      description,
      kmMatch.start,
      kmMatch.end,
    );
  }
  description = description
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

  return MaintenanceVoiceDraft(
    description: description.trim(),
    kilometers: kilometers,
    date: date,
  );
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

String _replaceRangeWithSpace(String value, int start, int end) =>
    '${value.substring(0, start)} ${value.substring(end)}';

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
