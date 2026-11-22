# Mi Garaje (Flutter)

Versión móvil multiplataforma. Incluye Inicio, Garaje, Mantenimiento,
Estadísticas y Perfil; permite administrar vehículos, cargas de combustible,
servicios y documentos. La información se almacena localmente.
En Mantenimiento puedes dictar el trabajo, kilometraje y fecha; la app rellena
el formulario para que lo revises antes de guardar. El dictado requiere los
permisos de micrófono y reconocimiento de voz, y un idioma español disponible
en el dispositivo.

## Desarrollo

```sh
flutter pub get
flutter run
flutter test
```

La carpeta `ios/` se genera y se compila en macOS. El workflow de Codemagic está
en la raíz del repositorio y construye `MiGaraje-unsigned.ipa` sin certificados
de distribución.

## Getting Started

This project is a starting point for a Flutter application.

A few resources to get you started if this is your first Flutter project:

- [Learn Flutter](https://docs.flutter.dev/get-started/learn-flutter)
- [Write your first Flutter app](https://docs.flutter.dev/get-started/codelab)
- [Flutter learning resources](https://docs.flutter.dev/reference/learning-resources)

For help getting started with Flutter development, view the
[online documentation](https://docs.flutter.dev/), which offers tutorials,
samples, guidance on mobile development, and a full API reference.
