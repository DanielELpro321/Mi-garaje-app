import 'package:flutter_test/flutter_test.dart';
import 'package:mi_garaje/main.dart';

void main() {
  test('GarageData conserva registros al serializar y restaurar JSON', () {
    final data = GarageData()
      ..vehicles.add(
        Vehicle(
          id: 'moto-1',
          brand: 'Yamaha',
          model: 'R15',
          year: 2020,
          cc: 155,
          km: 12000,
          plate: 'ABC123',
        ),
      )
      ..fuel.add(
        FuelEntry(
          id: 'fuel-1',
          vehicleId: 'moto-1',
          date: '2026-11-22',
          station: 'Terpel',
          amount: 50000,
          pricePerGallon: 16350,
          km: 12000,
        ),
      )
      ..services.add(
        ServiceEntry(
          id: 'service-1',
          vehicleId: 'moto-1',
          name: 'Aceite',
          intervalKm: 3000,
          lastKm: 11000,
          cost: 85000,
          date: '2026-11-22',
        ),
      )
      ..documents.add(
        VehicleDocument(
          id: 'doc-1',
          vehicleId: 'moto-1',
          name: 'SOAT',
          expiry: '2027-11-22',
        ),
      )
      ..owner = 'Daniel'
      ..pricePerGallon = 16350;

    final restored = GarageData.fromJson(data.toJson());

    expect(restored.vehicles.single.title, 'Yamaha R15');
    expect(restored.vehicles.single.km, 12000);
    expect(restored.fuel.single.station, 'Terpel');
    expect(restored.services.single.remaining(12000), 2000);
    expect(restored.documents.single.name, 'SOAT');
    expect(restored.owner, 'Daniel');
  });
}
