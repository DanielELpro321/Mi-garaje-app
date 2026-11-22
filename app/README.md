# Mi Garaje

Garaje digital para tu moto, con estética automotriz "dark premium":
negro carbón, tarjetas gris oscuro, un solo acento rojo deportivo y
valores colombianos (pesos, galones, kilómetros).

## Cómo abrirla

```
cargo run
```

Abre en una ventana con forma de teléfono (400 x 866) y se adapta a
cualquier tamaño manteniendo la proporción.

## Captura automática

```
cargo run -- --screenshot shots/ahora.png
```

Dibuja 60 fotogramas (para que terminen las animaciones), guarda la imagen y
se cierra. También acepta `--tab inicio|garaje|manto|stats|perfil`,
`--detail` (ficha del vehículo) y `--sheet tanqueo|nuevo`.

## Estructura

* **Barra inferior** con cinco secciones: Inicio, Mi garaje, Mantenimiento,
  Estadísticas y Perfil. La sección activa se marca con una píldora roja.
* **Inicio**: saludo corto, nombre de la moto como titular con su estado, la
  foto de tu moto grande y flotando sobre el fondo (sin recuadro ni marco, los
  bordes se funden en el negro), tres cifras del mes, accesos rápidos
  (tanquear, servicio, gastos, documentos), dos servicios con barra de avance y
  la actividad reciente.
* **Mi garaje**: tarjetas de cada vehículo con foto, año, placa, kilometraje y
  estado; botón para agregar un vehículo (hoja con marca y año); resumen,
  próximo servicio y documentación.
* **Mantenimiento**: estado del plan, próximos servicios con barra y botón
  MARCAR, documentación vigente y recordatorios.
* **Estadísticas**: gasto del mes, gráfico de barras de siete meses, consumo,
  costo por kilómetro y evolución del kilometraje.
* **Perfil**: datos del usuario, ajustes con interruptores, cuenta y cierre de
  sesión.
* **Ficha del vehículo**: foto grande y cuatro apartados (Resumen, Técnico,
  Gastos, Fotos) con información general, datos técnicos, kilometraje,
  historial, repuestos, gastos, documentación y galería.

## Animaciones

Entrada de cada pantalla (fundido y desplazamiento), línea de carga roja al
cambiar de sección, barras de progreso y del gráfico que crecen al aparecer,
píldora de la barra inferior que se desliza, hojas que suben desde abajo,
botones que se oscurecen al presionar y aviso verde cuando se guarda algo.

## Tu foto de la moto

La app busca `Assets/Images/mi_moto.jpg` (o `.png`). La foto se muestra grande y
sin recuadro: se toma la franja central (donde está la moto), se dibuja a lo
ancho de la pantalla y los cuatro bordes se disuelven hacia el negro de la app,
así que la moto parece flotar sobre el fondo en lugar de ir metida en un marco.
Sin foto, dibuja una silueta plateada de marcador de posición.

## Valores colombianos

* Pesos con punto de miles: `$ 85.000`.
* Galones y kilómetros; el galón parte en `$ 16.350` (Bogotá).
* Estaciones Terpel, Primax, Texaco y Biomax; SOAT y tecnomecánica; Bogotá D.C.

## Android (APK)

La misma interfaz se compila como biblioteca nativa (`libmi_garaje.so`) y se
empaqueta como APK en `MiGaraje.apk`.

* Paquete: `com.migaraje.app` · versión 1.0 · minSdk 24 (Android 7.0).
* Arquitecturas: `arm64-v8a` (teléfonos) y `x86_64` (emulador).
* Pantalla vertical y foto de la moto incrustada en la app.
* Firma con la clave de depuración de Android (`~/.android/debug.keystore`).

Instalar por USB:

```
adb install -r MiGaraje.apk
```

O copiar el APK al teléfono y abrirlo (hay que permitir "instalar apps
desconocidas" para el explorador de archivos).

Recompilar el APK:

```
powershell -ExecutionPolicy Bypass -File android\build_all.ps1
```

Requiere Rust con los targets de Android, `cargo-ndk`, el SDK/NDK de Android
(con `build-tools` y una plataforma) y Python con Pillow para el icono.

## Notas

`.cargo/config.toml` fija el compilador C que Rust necesita en este equipo.
