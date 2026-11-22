import 'package:flutter_test/flutter_test.dart';
import 'package:mi_garaje/maintenance_voice.dart';

void main() {
  group('parseMaintenanceVoiceText', () {
    final today = DateTime(2026, 11, 22);

    test('extracts work, grouped numeric kilometers, and today', () {
      final draft = parseMaintenanceVoiceText(
        'Hoy hice cambio de aceite a los 15.000 km',
        now: today,
      );

      expect(draft.description, 'cambio de aceite');
      expect(draft.kilometers, 15000);
      expect(draft.date, DateTime(2026, 11, 22));
    });

    test('extracts spoken kilometers and a Spanish date', () {
      final draft = parseMaintenanceVoiceText(
        'Lubriqué la cadena el 22 de mayo de 2025 con quince mil kilómetros',
        now: today,
      );

      expect(draft.description, 'Lubriqué la cadena');
      expect(draft.kilometers, 15000);
      expect(draft.date, DateTime(2025, 5, 22));
    });

    test('uses today when no date is mentioned and accepts slash dates', () {
      final defaultDate = parseMaintenanceVoiceText(
        'Revisión de frenos',
        now: today,
      );
      final explicitDate = parseMaintenanceVoiceText(
        'Cambio de filtro el 04/10/2026 a 12 500 km',
        now: today,
      );

      expect(defaultDate.date, DateTime(2026, 11, 22));
      expect(explicitDate.date, DateTime(2026, 10, 4));
      expect(explicitDate.kilometers, 12500);
      expect(explicitDate.description, 'Cambio de filtro');
    });
  });
}
