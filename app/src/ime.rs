//! Puente con el teclado del sistema Android.
//!
//! En Android no dibujamos un teclado propio: una Activity Java
//! (`MiGarajeActivity`) se conecta al IME del sistema y nos reenvía cada
//! carácter por JNI. Aquí viven la cola de eventos que lee la interfaz y las
//! llamadas Rust→Java para mostrar u ocultar el teclado.
//!
//! Toda la comunicación se hace con `jni-sys` (JNI crudo) para no arrastrar el
//! crate `jni` completo.

use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};
use std::sync::Mutex;

use jni_sys::*;

/// Qué teclado conviene mostrar para el campo activo.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Teclado {
    Texto,
    Numero,
}

/// Lo que el usuario hizo en el teclado del sistema.
pub enum Evento {
    /// Escribió texto (puede ser más de un carácter).
    Texto(String),
    /// Borró un carácter.
    Borrar,
    /// Pulsó "Listo"/Intro: se cierra el foco.
    Listo,
    /// La caja inteligente recibió voz: texto reconocido.
    Voz(String),
    /// La caja inteligente no pudo reconocer: motivo del fallo.
    VozError(String),
}

static VM: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static ACTIVIDAD: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static COLA: Mutex<Vec<Evento>> = Mutex::new(Vec::new());

/// Zonas que ocupan las barras del sistema (arriba/abajo), en píxeles físicos.
/// Las escribe Java desde el hilo de interfaz; las lee el dibujo.
static INSET_ARRIBA: AtomicI32 = AtomicI32::new(0);
static INSET_ABAJO: AtomicI32 = AtomicI32::new(0);

/// Guarda la máquina virtual y la Activity para poder llamarlas después.
///
/// `actividad` llega como referencia global de JNI *sin poseer* que
/// `android-activity` libera cuando destruye el `AndroidApp` (por ejemplo al
/// recrearse la Activity). Guardarla tal cual dejaba una referencia obsoleta y
/// la app se cerraba con `JNI ERROR ... accessed stale Global`. Por eso hacemos
/// nuestra propia copia con `NewGlobalRef`, que vive mientras viva el proceso.
pub fn fijar(vm: *mut c_void, actividad: *mut c_void) {
    VM.store(vm, Ordering::SeqCst);
    unsafe {
        let Some(env) = env_actual() else {
            return;
        };
        let tabla = &(**env).v1_6;
        let previa = ACTIVIDAD.swap(std::ptr::null_mut(), Ordering::SeqCst) as jobject;
        let nueva = (tabla.NewGlobalRef)(env, actividad as jobject);
        ACTIVIDAD.store(nueva as *mut c_void, Ordering::SeqCst);
        if !previa.is_null() {
            (tabla.DeleteGlobalRef)(env, previa);
        }
    }
}

/// Espacio que ocupan las barras del sistema (arriba, abajo) en píxeles.
pub fn insets() -> (i32, i32) {
    (
        INSET_ARRIBA.load(Ordering::Relaxed),
        INSET_ABAJO.load(Ordering::Relaxed),
    )
}

/// Devuelve lo que el usuario escribió desde el último fotograma.
pub fn drenar() -> Vec<Evento> {
    match COLA.lock() {
        Ok(mut q) => std::mem::take(&mut *q),
        Err(_) => Vec::new(),
    }
}

fn empujar(e: Evento) {
    if let Ok(mut q) = COLA.lock() {
        q.push(e);
    }
}

/// Pide al sistema que muestre el teclado para el campo activo.
pub fn mostrar(t: Teclado) {
    let tipo = match t {
        Teclado::Texto => 0,
        Teclado::Numero => 1,
    };
    // Construir la unión es seguro; la llamada no.
    unsafe { llamar("mostrarTeclado", "(I)V", Some(jvalue { i: tipo })) };
}

/// Oculta el teclado del sistema.
pub fn ocultar() {
    unsafe { llamar("ocultarTeclado", "()V", None) };
}

/// Arranca el reconocimiento de voz del sistema (la caja inteligente). El
/// resultado llega después como `Evento::Voz` o `Evento::VozError`.
pub fn escuchar() {
    unsafe { llamar("escuchar", "()V", None) };
}

/// Deja una línea en el rastro de arranque del teléfono. Sirve para saber hasta
/// dónde llegó la app cuando se cierra de golpe (el archivo sobrevive).
pub fn marca(etapa: &str) {
    let limpio = solo_ascii(etapa);
    unsafe { llamar_texto("marca", "(Ljava/lang/String;)V", &limpio) };
}

/// Borra el rastro de arranque y quita de la pantalla el aviso de diagnóstico.
pub fn limpiar() {
    unsafe { llamar("limpiarMarca", "()V", None) };
}

/// Deja solo caracteres ASCII imprimibles: `NewStringUTF` no admite cualquier
/// cosa y así evitamos sorpresas.
fn solo_ascii(msg: &str) -> String {
    msg.chars()
        .map(|c| if c.is_ascii() && c != '\0' { c } else { '?' })
        .collect()
}

/// Muestra en pantalla un aviso de diagnóstico y lo deja guardado en el
/// teléfono. Devuelve `true` si consiguió hablar con la parte Java (para saber
/// si vale la pena mantener la app abierta para que se lea).
pub fn avisar(msg: &str) -> bool {
    unsafe { llamar_texto("mostrarAviso", "(Ljava/lang/String;)V", &solo_ascii(msg)) }
}

// ------------------------------------------------------------ llamadas JNI
/// Devuelve el `JNIEnv*` del hilo actual, adjuntándolo si hiciera falta.
unsafe fn env_actual() -> Option<*mut JNIEnv> {
    // `VM` guarda el asidero de la JVM tal como lo entrega `AndroidApp::vm_as_ptr()`:
    // un `JavaVM*`, es decir un puntero a la ranura que contiene la tabla de
    // funciones. Para llegar a la tabla hay que desreferenciar DOS veces (igual
    // que hace el crate `jni` en `java_vm_call_unchecked!`). Con una sola
    // desreferencia se lee un puntero a función de una dirección equivocada y se
    // acaba llamando a `0` (el cierre instantáneo que veíamos).
    let pvm = VM.load(Ordering::SeqCst) as *mut JavaVM;
    if pvm.is_null() {
        return None;
    }

    let mut env: *mut JNIEnv = std::ptr::null_mut();
    let penv = (&mut env) as *mut *mut JNIEnv as *mut *mut c_void;
    let g = ((*(*pvm)).v1_2.GetEnv)(pvm, penv, JNI_VERSION_1_6);
    if g == JNI_OK && !env.is_null() {
        return Some(env);
    }

    let mut env: *mut JNIEnv = std::ptr::null_mut();
    let penv = (&mut env) as *mut *mut JNIEnv as *mut *mut c_void;
    let a = ((*(*pvm)).v1_2.AttachCurrentThread)(pvm, penv, std::ptr::null_mut());
    if a == JNI_OK && !env.is_null() {
        Some(env)
    } else {
        None
    }
}

/// Llama a un método de la Activity (por ejemplo `mostrarTeclado`).
unsafe fn llamar(nombre: &str, firma: &str, arg: Option<jvalue>) {
    let Some(env) = env_actual() else {
        return;
    };
    let actividad = ACTIVIDAD.load(Ordering::SeqCst) as jobject;
    if actividad.is_null() {
        return;
    }
    let tabla = &(**env).v1_6;
    let clase = (tabla.GetObjectClass)(env, actividad);
    if clase.is_null() {
        return;
    }
    let (Ok(nombre), Ok(firma)) = (
        std::ffi::CString::new(nombre),
        std::ffi::CString::new(firma),
    ) else {
        return;
    };
    let metodo = (tabla.GetMethodID)(env, clase, nombre.as_ptr(), firma.as_ptr());
    if metodo.is_null() {
        (tabla.DeleteLocalRef)(env, clase);
        return;
    }
    match arg {
        Some(v) => {
            (tabla.CallVoidMethodA)(env, actividad, metodo, &v as *const jvalue);
        }
        None => {
            (tabla.CallVoidMethodA)(env, actividad, metodo, std::ptr::null());
        }
    }
    // Este hilo queda "adjunto" toda la vida de la app, así que las referencias
    // locales no se liberan solas: las borramos a mano para no llenar la tabla.
    (tabla.DeleteLocalRef)(env, clase);
}

/// Igual que `llamar` pero pasando un `String` de Java como argumento.
unsafe fn llamar_texto(nombre: &str, firma: &str, texto: &str) -> bool {
    let Some(env) = env_actual() else {
        return false;
    };
    let actividad = ACTIVIDAD.load(Ordering::SeqCst) as jobject;
    if actividad.is_null() {
        return false;
    }
    let tabla = &(**env).v1_6;
    let clase = (tabla.GetObjectClass)(env, actividad);
    if clase.is_null() {
        return false;
    }
    let (Ok(nombre), Ok(firma)) = (
        std::ffi::CString::new(nombre),
        std::ffi::CString::new(firma),
    ) else {
        return false;
    };
    let metodo = (tabla.GetMethodID)(env, clase, nombre.as_ptr(), firma.as_ptr());
    if metodo.is_null() {
        (tabla.DeleteLocalRef)(env, clase);
        return false;
    }
    let Ok(cstr) = std::ffi::CString::new(texto) else {
        (tabla.DeleteLocalRef)(env, clase);
        return false;
    };
    let jmsg = (tabla.NewStringUTF)(env, cstr.as_ptr());
    if jmsg.is_null() {
        (tabla.DeleteLocalRef)(env, clase);
        return false;
    }
    let arg = jvalue { l: jmsg };
    (tabla.CallVoidMethodA)(env, actividad, metodo, &arg as *const jvalue);
    (tabla.DeleteLocalRef)(env, jmsg);
    (tabla.DeleteLocalRef)(env, clase);
    true
}

// ------------------------------------------------------- callbacks Java→Rust
#[no_mangle]
pub extern "system" fn Java_com_migaraje_app_MiGarajeActivity_nativeChar(
    _env: *mut JNIEnv,
    _obj: jobject,
    cp: jint,
) {
    if let Some(ch) = char::from_u32(cp as u32) {
        let mut buf = [0u8; 4];
        empujar(Evento::Texto(ch.encode_utf8(&mut buf).to_string()));
    }
}

#[no_mangle]
pub extern "system" fn Java_com_migaraje_app_MiGarajeActivity_nativeBorrar(
    _env: *mut JNIEnv,
    _obj: jobject,
) {
    empujar(Evento::Borrar);
}

#[no_mangle]
pub extern "system" fn Java_com_migaraje_app_MiGarajeActivity_nativeListo(
    _env: *mut JNIEnv,
    _obj: jobject,
) {
    empujar(Evento::Listo);
}

/// Java entrega el texto reconocido por voz.
#[no_mangle]
pub extern "system" fn Java_com_migaraje_app_MiGarajeActivity_nativeVozTexto(
    env: *mut JNIEnv,
    _obj: jobject,
    texto: jstring,
) {
    if texto.is_null() {
        return;
    }
    if let Some(s) = unsafe { leer_string(env, texto) } {
        empujar(Evento::Voz(s));
    }
}

/// Java avisa que el reconocimiento de voz falló o no está disponible.
#[no_mangle]
pub extern "system" fn Java_com_migaraje_app_MiGarajeActivity_nativeVozError(
    env: *mut JNIEnv,
    _obj: jobject,
    texto: jstring,
) {
    let s = if texto.is_null() {
        "No pude escuchar".to_string()
    } else {
        unsafe { leer_string(env, texto) }.unwrap_or_else(|| "No pude escuchar".to_string())
    };
    empujar(Evento::VozError(s));
}

/// Convierte un `jstring` en `String` de Rust.
unsafe fn leer_string(env: *mut JNIEnv, texto: jstring) -> Option<String> {
    let tabla = &(**env).v1_6;
    let ptr = (tabla.GetStringUTFChars)(env, texto, std::ptr::null_mut());
    if ptr.is_null() {
        return None;
    }
    let out = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
    (tabla.ReleaseStringUTFChars)(env, texto, ptr);
    Some(out)
}

/// Java avisa del espacio que ocupan las barras del sistema (status y
/// navegación). Lo usamos para no dibujar debajo de ellas.
#[no_mangle]
pub extern "system" fn Java_com_migaraje_app_MiGarajeActivity_nativeInsets(
    _env: *mut JNIEnv,
    _obj: jobject,
    arriba: jint,
    abajo: jint,
) {
    INSET_ARRIBA.store(arriba.max(0), Ordering::Relaxed);
    INSET_ABAJO.store(abajo.max(0), Ordering::Relaxed);
}
