import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mi_garaje/main.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  testWidgets('empty garage shows onboarding and section navigation', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    await tester.pumpWidget(const GarageApp());
    await tester.pumpAndSettle();

    expect(find.text('Tu garaje empieza aquí'), findsOneWidget);
    expect(find.text('Agregar mi primera moto'), findsOneWidget);
    expect(find.text('Servicio'), findsOneWidget);
  });

  testWidgets('first motorcycle button opens and saves the motorcycle form', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    await tester.pumpWidget(const GarageApp());
    await tester.pumpAndSettle();

    final addButton = find.text('Agregar mi primera moto');
    await tester.ensureVisible(addButton);
    await tester.tap(addButton);
    await tester.pumpAndSettle();

    expect(find.text('Agregar vehículo'), findsOneWidget);
    await tester.enterText(find.byType(TextFormField).at(0), 'Honda');
    await tester.enterText(find.byType(TextFormField).at(1), 'CB 125');
    await tester.tap(find.text('Guardar'));
    await tester.pumpAndSettle();

    expect(find.text('Honda CB 125'), findsOneWidget);
  });

  testWidgets('floating voice button opens the maintenance form', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    await tester.pumpWidget(const GarageApp());
    await tester.pumpAndSettle();

    await tester.tap(find.text('Agregar mi primera moto'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextFormField).at(0), 'Honda');
    await tester.enterText(find.byType(TextFormField).at(1), 'CB 125');
    await tester.tap(find.text('Guardar'));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('nav-Servicio')));
    await tester.pumpAndSettle();
    expect(find.text('Dictar mantenimiento'), findsOneWidget);

    await tester.tap(find.text('Dictar mantenimiento'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 500));

    expect(find.text('Registrar mantenimiento'), findsNWidgets(2));
    expect(find.text('Registro por voz'), findsOneWidget);
  });
}
