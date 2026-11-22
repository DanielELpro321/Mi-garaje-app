import 'package:flutter_test/flutter_test.dart';
import 'package:mi_garaje/main.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  testWidgets('empty garage shows onboarding and section navigation', (tester) async {
    SharedPreferences.setMockInitialValues({});
    await tester.pumpWidget(const GarageApp());
    await tester.pumpAndSettle();

    expect(find.text('Tu garaje empieza aquí'), findsOneWidget);
    expect(find.text('Agregar mi primera moto'), findsOneWidget);
    expect(find.text('Servicio'), findsOneWidget);
  });
}
