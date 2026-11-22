package com.migaraje.app;

import android.app.NativeActivity;
import android.content.Context;
import android.os.Bundle;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import android.widget.FrameLayout;

/**
 * Activity de Mi Garaje.
 *
 * Extiende {@link NativeActivity} para conservar el motor de dibujo nativo
 * (el táctil y la superficie van por el camino nativo gracias a
 * {@code takeSurface}/{@code takeInputQueue}). Lo único que añade es una vista
 * enfocada que sirve de puente con el <b>teclado del sistema Android</b>: cada
 * carácter que escribe el usuario se reenvía a Rust por JNI y allí se guarda en
 * el campo de texto activo.
 */
public class MiGarajeActivity extends NativeActivity {

    static {
        // NativeActivity carga la biblioteca por su cuenta (con dlopen), pero
        // ART solo resuelve los métodos `native` de las bibliotecas que además
        // se registran con el cargador de clases por medio de loadLibrary.
        // Llamarla aquí (antes de onCreate) es idempotente: dlopen lleva la
        // cuenta y reutiliza el mismo binario.
        System.loadLibrary("mi_garaje");
    }

    // Tipos de teclado que puede pedir Rust: 0 = texto, 1 = número, 2 = fecha.
    private static final int TEXTO = 0;
    private static final int NUMERO = 1;
    private static final int FECHA = 2;

    private int mTipo = TEXTO;
    private boolean mQuiereTeclado = false;
    private int mIntentos = 0;
    private Entrada mEntrada;

    /**
     * Reintenta mostrar el teclado hasta que el sistema "sirva" la vista. El
     * foco se entrega de forma asíncrona, así que las primeras peticiones
     * pueden caer en el vacío; se insiste un poco y se para en cuanto el IME
     * queda conectado a la vista.
     */
    private final Runnable mReintento = new Runnable() {
        @Override
        public void run() {
            if (!mQuiereTeclado || mEntrada == null) {
                return;
            }
            InputMethodManager imm = imm();
            if (imm == null) {
                return;
            }
            pedir(imm);
            mIntentos++;
            if (!imm.isActive(mEntrada) && mIntentos < 10) {
                mEntrada.postDelayed(this, 150);
            }
        }
    };

    private static native void nativeChar(int codePoint);

    private static native void nativeBorrar();

    private static native void nativeListo();

    private static native void nativeInsets(int top, int bottom);

    private static native void nativeVozTexto(String texto);

    private static native void nativeVozError(String error);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // Antes de `super.onCreate`: a partir de ahí arranca el hilo nativo, y
        // queremos haber leído y borrado el rastro anterior sin carreras.
        instalarCapturaDeFallos();
        marca("J1 onCreate");
        final String rastroPrevio = leerMarca();
        limpiarArchivoMarca();

        super.onCreate(savedInstanceState);
        marca("J2 super.onCreate ok");
        fijarOrientacion();
        try {
            getWindow().setStatusBarColor(0xFF0B0D10);
            getWindow().setNavigationBarColor(0xFF15191F);
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.P) {
                android.view.WindowManager.LayoutParams attrs = getWindow().getAttributes();
                attrs.layoutInDisplayCutoutMode =
                        android.view.WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
                getWindow().setAttributes(attrs);
            }
        } catch (Throwable t) {
        }

        // Si el arranque anterior no llegó a completarse, enseñamos su rastro:
        // así sabemos en qué paso murió aunque el proceso desapareciera de golpe.
        if (rastroPrevio != null && rastroPrevio.trim().length() > 0) {
            mostrarTexto(rastroPrevio, "RASTRO DEL ARRANQUE ANTERIOR", false);
        }

        mEntrada = new Entrada(this);
        ViewGroup content = findViewById(android.R.id.content);
        if (content != null) {
            content.addView(mEntrada, new FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT));
            marca("J3 vista de teclado anadida");
        } else {
            marca("J3 SIN content view");
        }
        // No pedimos el foco al arrancar: el teclado solo debe salir cuando el
        // usuario toque un campo (Rust llama a `mostrarTeclado`).
        instalarInsets();
    }

    /**
     * Teléfono: se queda en vertical, como hasta ahora. Tablet (pantalla
     * ancha, smallestScreenWidthDp >= 600): se deja que el sistema decida,
     * para poder girar y aprovechar el ancho con la barra lateral.
     */
    private void fijarOrientacion() {
        try {
            if (getResources().getConfiguration().smallestScreenWidthDp < 600) {
                setRequestedOrientation(
                        android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);
            }
        } catch (Throwable t) {
        }
    }

    /**
     * Mide las barras del sistema y las comunica a Rust.
     *
     * Cuando la ventana ocupa toda la pantalla (edge-to-edge, que es como
     * arranca esta Activity), Android dibuja la barra de estado y la de
     * navegación <b>encima</b> del contenido: por eso la barra inferior de la
     * app quedaba tapada por los botones táctiles. Si la ventana ya respeta
     * esas barras (no es edge-to-edge) no hay nada que apartar.
     */
    private void instalarInsets() {
        try {
            final View decor = getWindow().getDecorView();
            decor.setOnApplyWindowInsetsListener(new View.OnApplyWindowInsetsListener() {
                @Override
                public android.view.WindowInsets onApplyWindowInsets(View v,
                        android.view.WindowInsets insets) {
                    medirInsets(insets);
                    return insets;
                }
            });
            decor.post(new Runnable() {
                @Override
                public void run() {
                    try {
                        decor.requestApplyInsets();
                    } catch (Throwable t) {
                    }
                }
            });
        } catch (Throwable t) {
        }
    }

    private void medirInsets(android.view.WindowInsets insets) {
        int arriba = 0;
        int abajo = 0;
        try {
            if (insets != null) {
                if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                    android.graphics.Insets barras =
                            insets.getInsets(android.view.WindowInsets.Type.systemBars());
                    arriba = barras.top;
                    abajo = barras.bottom;
                } else {
                    arriba = insets.getSystemWindowInsetTop();
                    abajo = insets.getSystemWindowInsetBottom();
                }
            }
        } catch (Throwable t) {
        }
        nativeInsets(arriba, abajo);
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus && mQuiereTeclado && mEntrada != null) {
            mEntrada.requestFocus();
            mostrarIme();
        }
    }

    /** Llamado desde Rust cuando el usuario toca un campo de texto. */
    void mostrarTeclado(final int tipo) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                mTipo = tipo;
                mQuiereTeclado = true;
                if (mEntrada != null) {
                    mEntrada.requestFocus();
                    mostrarIme();
                }
            }
        });
    }

    /** Llamado desde Rust cuando el campo activo pierde el foco. */
    void ocultarTeclado() {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                mQuiereTeclado = false;
                if (mEntrada == null) {
                    return;
                }
                InputMethodManager imm = imm();
                if (imm != null) {
                    imm.hideSoftInputFromWindow(mEntrada.getWindowToken(), 0);
                }
            }
        });
    }

    /** Llamado desde Rust para activar la escucha por voz. */
    void escuchar() {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                try {
                    android.content.Intent intent = new android.content.Intent(
                            android.speech.RecognizerIntent.ACTION_RECOGNIZE_SPEECH);
                    intent.putExtra(android.speech.RecognizerIntent.EXTRA_LANGUAGE_MODEL,
                            android.speech.RecognizerIntent.LANGUAGE_MODEL_FREE_FORM);
                    intent.putExtra(android.speech.RecognizerIntent.EXTRA_LANGUAGE, "es-CO");
                    intent.putExtra(android.speech.RecognizerIntent.EXTRA_PROMPT, "Di qué hiciste (ej: tanqueé 50.000)");
                    startActivityForResult(intent, 1001);
                } catch (Throwable t) {
                    nativeVozError("Reconocimiento de voz no disponible");
                }
            }
        });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, android.content.Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode == 1001) {
            if (resultCode == RESULT_OK && data != null) {
                java.util.ArrayList<String> matches = data.getStringArrayListExtra(
                        android.speech.RecognizerIntent.EXTRA_RESULTS);
                if (matches != null && !matches.isEmpty()) {
                    nativeVozTexto(matches.get(0));
                    return;
                }
            }
            nativeVozError("No se reconoció voz");
        }
    }

    private InputMethodManager imm() {
        return (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
    }

    /** Guarda un texto en `files/error.txt` para poder consultarlo después. */
    private void guardarError(String texto) {
        try {
            java.io.FileOutputStream fos = openFileOutput("error.txt", Context.MODE_PRIVATE);
            fos.write(texto.getBytes("UTF-8"));
            fos.close();
        } catch (Throwable t) {
            // da igual si no se puede escribir
        }
    }

    /** Agrega una línea al rastro de arranque (`files/arranque.txt`). */
    void marca(String texto) {
        byte[] bytes;
        try {
            bytes = (texto + "\n").getBytes("UTF-8");
        } catch (Throwable t) {
            return;
        }
        try {
            java.io.FileOutputStream fos = openFileOutput("arranque.txt", Context.MODE_APPEND);
            fos.write(bytes);
            fos.close();
        } catch (Throwable t) {
        }
        // Copia en una carpeta que sí se puede abrir con el gestor de archivos
        // del teléfono, por si la app muere antes de poder enseñar nada.
        try {
            java.io.File f = archivoPublico();
            if (f != null) {
                java.io.FileOutputStream fos = new java.io.FileOutputStream(f, true);
                fos.write(bytes);
                fos.close();
            }
        } catch (Throwable t) {
        }
    }

    /** `Android/media/<paquete>/mi-garaje-rastro.txt` (accesible sin permisos). */
    private java.io.File archivoPublico() {
        try {
            java.io.File dir = new java.io.File(
                    android.os.Environment.getExternalStorageDirectory(),
                    "Android/media/" + getPackageName());
            if (!dir.exists()) {
                dir.mkdirs();
            }
            return new java.io.File(dir, "mi-garaje-rastro.txt");
        } catch (Throwable t) {
            return null;
        }
    }

    /** Lee el rastro del arranque anterior (vacío si no hay). */
    private String leerMarca() {
        try {
            java.io.FileInputStream fis = openFileInput("arranque.txt");
            byte[] buf = new byte[16384];
            int n = fis.read(buf);
            fis.close();
            if (n <= 0) {
                return "";
            }
            return new String(buf, 0, n, "UTF-8");
        } catch (Throwable t) {
            return "";
        }
    }

    private void limpiarArchivoMarca() {
        try {
            deleteFile("arranque.txt");
        } catch (Throwable t) {
        }
        try {
            java.io.File f = archivoPublico();
            if (f != null) {
                f.delete();
            }
        } catch (Throwable t) {
        }
    }

    /** Verdadero en cuanto Rust dibuja el primer fotograma (arranque correcto). */
    private volatile boolean mArranqueOk = false;

    /**
     * Lo llama Rust cuando ya dibujó el primer fotograma: el arranque fue bien,
     * así que se borra el rastro y se quita el aviso de la pantalla.
     */
    void limpiarMarca() {
        mArranqueOk = true;
        limpiarArchivoMarca();
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                quitarAviso();
            }
        });
    }

    private android.view.View mAviso;

    private void quitarAviso() {
        try {
            if (mAviso != null && mAviso.getParent() instanceof ViewGroup) {
                ((ViewGroup) mAviso.getParent()).removeView(mAviso);
            }
        } catch (Throwable t) {
        }
        mAviso = null;
    }

    /**
     * Pinta un recuadro rojo seleccionable sobre la pantalla.
     *
     * @param esError si es `false` (rastro de arranque) el aviso se descarta
     *                cuando la app consigue arrancar de verdad.
     */
    private void mostrarTexto(final String texto, final String titulo, final boolean esError) {
        Runnable pintar = new Runnable() {
            @Override
            public void run() {
                try {
                    if (!esError && mArranqueOk) {
                        return; // la app sí arrancó: no tapamos la pantalla
                    }
                    quitarAviso();
                    ViewGroup content = findViewById(android.R.id.content);
                    if (content == null) {
                        return;
                    }
                    android.widget.TextView tv = new android.widget.TextView(MiGarajeActivity.this);
                    tv.setText(titulo + "\n\n" + texto
                            + "\n\n(manten pulsado para copiar este texto)");
                    tv.setTextColor(0xFFFF6666);
                    tv.setTextSize(12f);
                    tv.setBackgroundColor(0xF00B0D10);
                    tv.setPadding(24, 60, 24, 24);
                    tv.setTextIsSelectable(true);
                    android.widget.ScrollView sv = new android.widget.ScrollView(MiGarajeActivity.this);
                    sv.addView(tv);
                    content.addView(sv, new FrameLayout.LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT,
                            ViewGroup.LayoutParams.MATCH_PARENT));
                    mAviso = sv;
                } catch (Throwable t) {
                }
            }
        };
        // Se publica en el hilo principal. Si la app acaba arrancando bien, el
        // aviso se descarta (bandera `mArranqueOk`) o se retira enseguida.
        android.os.Handler h = new android.os.Handler(android.os.Looper.getMainLooper());
        h.post(pintar);
    }

    /**
     * Muestra en pantalla un aviso de diagnóstico (lo llama Rust cuando el
     * motor nativo falla) y lo deja guardado en el teléfono.
     */
    void mostrarAviso(final String msg) {
        final String texto = (msg == null || msg.isEmpty()) ? "(sin mensaje)" : msg;
        guardarError(texto);
        android.os.Handler h = new android.os.Handler(android.os.Looper.getMainLooper());
        h.post(new Runnable() {
            @Override
            public void run() {
                try {
                    android.widget.Toast.makeText(MiGarajeActivity.this, texto,
                            android.widget.Toast.LENGTH_LONG).show();
                } catch (Throwable t) {
                }
            }
        });
        mostrarTexto(texto, "DIAGNOSTICO", true);
    }

    /** Deja escrito cualquier fallo no controlado de Java. */
    private void instalarCapturaDeFallos() {
        final Thread.UncaughtExceptionHandler previo =
                Thread.getDefaultUncaughtExceptionHandler();
        Thread.setDefaultUncaughtExceptionHandler(new Thread.UncaughtExceptionHandler() {
            @Override
            public void uncaughtException(Thread hilo, Throwable t) {
                try {
                    java.io.StringWriter sw = new java.io.StringWriter();
                    t.printStackTrace(new java.io.PrintWriter(sw));
                    guardarError("FALLO JAVA:\n" + sw.toString());
                } catch (Throwable ignora) {
                }
                if (previo != null) {
                    previo.uncaughtException(hilo, t);
                }
            }
        });
    }

    /** Lanza el reintento acotado que muestra el teclado del sistema. */
    private void mostrarIme() {
        if (mEntrada == null) {
            return;
        }
        mIntentos = 0;
        mEntrada.removeCallbacks(mReintento);
        mEntrada.post(mReintento);
    }

    private void pedir(InputMethodManager imm) {
        mEntrada.requestFocus();
        imm.restartInput(mEntrada);
        imm.showSoftInput(mEntrada, 0);
    }

    private void enviar(CharSequence texto) {
        if (texto == null) {
            return;
        }
        for (int i = 0; i < texto.length(); ) {
            int cp = Character.codePointAt(texto, i);
            nativeChar(cp);
            i += Character.charCount(cp);
        }
    }

    /** Vista invisible que solo existe para conectar con el teclado del sistema. */
    private final class Entrada extends View {

        Entrada(Context ctx) {
            super(ctx);
            setFocusable(true);
            setFocusableInTouchMode(true);
        }

        @Override
        public boolean onCheckIsTextEditor() {
            return true;
        }

        @Override
        public InputConnection onCreateInputConnection(EditorInfo out) {
            out.inputType = tipoEntrada();
            out.imeOptions = EditorInfo.IME_ACTION_DONE
                    | EditorInfo.IME_FLAG_NO_FULLSCREEN
                    | EditorInfo.IME_FLAG_NO_EXTRACT_UI;
            out.initialSelStart = 0;
            out.initialSelEnd = 0;
            return new BaseInputConnection(Entrada.this, false) {
                // Texto que el IME tiene "en composición" (aún sin confirmar).
                // Lo vamos escribiendo en el campo para que se vea, y lo
                // quitamos cuando el IME lo reemplaza. Así funcionan tanto los
                // teclados que confirman directamente (commitText) como los que
                // van componiendo la palabra (setComposingText).
                private String mComposicion = "";

                /** Borra del campo el texto que el IME tenía en composición. */
                private void quitarComposicion() {
                    int n = mComposicion.length();
                    for (int i = 0; i < n; i++) {
                        nativeBorrar();
                    }
                    mComposicion = "";
                }

                @Override
                public boolean commitText(CharSequence text, int newCursorPosition) {
                    android.util.Log.i("MiGarajeIME", "commit: " + text);
                    // `commitText` sustituye la composición por el texto que se
                    // confirma (es lo que hace `BaseInputConnection`): quitamos
                    // lo que estuviera en composición y escribimos el texto.
                    quitarComposicion();
                    enviar(text);
                    return true;
                }

                @Override
                public boolean setComposingText(CharSequence text, int newCursorPosition) {
                    android.util.Log.i("MiGarajeIME", "composing: " + text);
                    String nuevo = (text == null) ? "" : text.toString();
                    if (nuevo.equals(mComposicion)) {
                        return true;
                    }
                    quitarComposicion();
                    enviar(nuevo);
                    mComposicion = nuevo;
                    return true;
                }

                @Override
                public boolean setComposingRegion(int start, int end) {
                    return true;
                }

                @Override
                public boolean finishComposingText() {
                    mComposicion = "";
                    return true;
                }

                @Override
                public boolean deleteSurroundingText(int beforeLength, int afterLength) {
                    int n = beforeLength + afterLength;
                    for (int i = 0; i < n; i++) {
                        nativeBorrar();
                    }
                    // El cursor siempre está al final, así que lo borrado sale
                    // de la composición (si la había).
                    if (beforeLength >= mComposicion.length()) {
                        mComposicion = "";
                    } else {
                        mComposicion = mComposicion.substring(0, mComposicion.length() - beforeLength);
                    }
                    return true;
                }

                @Override
                public boolean sendKeyEvent(KeyEvent event) {
                    if (event.getAction() == KeyEvent.ACTION_DOWN) {
                        int code = event.getKeyCode();
                        if (code == KeyEvent.KEYCODE_DEL) {
                            nativeBorrar();
                            if (mComposicion.length() > 0) {
                                mComposicion = mComposicion.substring(0, mComposicion.length() - 1);
                            }
                            return true;
                        }
                        if (code == KeyEvent.KEYCODE_ENTER) {
                            nativeListo();
                            return true;
                        }
                        // Algunos teclados numéricos (p. ej. el de MIUI) envían
                        // las cifras como eventos de tecla en vez de texto.
                        int cifra = -1;
                        if (code >= KeyEvent.KEYCODE_0 && code <= KeyEvent.KEYCODE_9) {
                            cifra = code - KeyEvent.KEYCODE_0;
                        } else if (code >= KeyEvent.KEYCODE_NUMPAD_0
                                && code <= KeyEvent.KEYCODE_NUMPAD_9) {
                            cifra = code - KeyEvent.KEYCODE_NUMPAD_0;
                        }
                        if (cifra >= 0) {
                            quitarComposicion();
                            enviar(String.valueOf((char) ('0' + cifra)));
                            return true;
                        }
                        if (code == KeyEvent.KEYCODE_PERIOD || code == KeyEvent.KEYCODE_NUMPAD_DOT) {
                            quitarComposicion();
                            enviar(".");
                            return true;
                        }
                        if (code == KeyEvent.KEYCODE_COMMA || code == KeyEvent.KEYCODE_NUMPAD_COMMA) {
                            quitarComposicion();
                            enviar(",");
                            return true;
                        }
                    }
                    return super.sendKeyEvent(event);
                }

                @Override
                public boolean performEditorAction(int actionCode) {
                    nativeListo();
                    return true;
                }
            };
        }

        private int tipoEntrada() {
            switch (mTipo) {
                case NUMERO:
                    return InputType.TYPE_CLASS_NUMBER | InputType.TYPE_NUMBER_FLAG_DECIMAL;
                case FECHA:
                    return InputType.TYPE_CLASS_DATETIME | InputType.TYPE_DATETIME_VARIATION_DATE;
                default:
                    return InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS;
            }
        }
    }
}
