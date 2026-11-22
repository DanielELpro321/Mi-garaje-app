# Mi Garaje

Aplicación móvil para registrar motos, combustible, mantenimiento y documentos.
Los datos se guardan localmente en el dispositivo.

## Aplicaciones

- `app_flutter/`: versión Flutter para iOS y Android.
- `app/`: implementación original en Rust/egui para escritorio y Android.

## Compilar iOS en Codemagic

El workflow `mi-garaje-ios-unsigned` compila en `mac_mini_m2`, usando Flutter
stable, Xcode latest y CocoaPods default. Ejecuta `flutter build ios --release
--no-codesign` y empaqueta `Runner.app` como `MiGaraje-unsigned.ipa`.

El IPA sin firma sirve como artefacto de compilación. Para instalarlo en un
iPhone será necesario volver a firmarlo con Sideloadly o AltStore usando un
Apple ID; no hace falta una cuenta de desarrollador de pago para la firma
personal de sideload.

## Ejecutar Flutter

Desde `app_flutter/`:

```sh
flutter pub get
flutter run
flutter test
```
