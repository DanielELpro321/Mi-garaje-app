"""Ensambla, alinea y firma el APK de Mi Garaje.

Uso:
    python android/build_apk.py

Requiere que las bibliotecas nativas ya estén compiladas en
android/app/src/main/jniLibs/<abi>/libmi_garaje.so (las genera cargo-ndk).
"""

import os
import re
import shutil
import subprocess
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
APP = os.path.dirname(HERE)
BUILD = os.path.join(HERE, "build")
DIST = os.path.join(HERE, "dist")
JNI = os.path.join(HERE, "app", "src", "main", "jniLibs")
LIB_NAME = "libmi_garaje.so"


def find_sdk():
    for key in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        value = os.environ.get(key)
        if value and os.path.isdir(value):
            return value
    local = os.environ.get("LOCALAPPDATA")
    if local:
        candidate = os.path.join(local, "Android", "Sdk")
        if os.path.isdir(candidate):
            return candidate
    sys.exit("No encontré el Android SDK (define ANDROID_HOME).")


def version_key(name):
    return [int(p) for p in re.findall(r"\d+", name)] or [0]


SDK = find_sdk()
TOOLS = os.path.join(SDK, "build-tools")
BUILD_TOOLS = os.path.join(
    TOOLS, max((d for d in os.listdir(TOOLS) if os.path.isdir(os.path.join(TOOLS, d))), key=version_key)
)
PLATFORMS = os.path.join(SDK, "platforms")
PLATFORM = os.path.join(
    PLATFORMS,
    max((d for d in os.listdir(PLATFORMS) if os.path.isdir(os.path.join(PLATFORMS, d))), key=version_key),
)
AAPT2 = os.path.join(BUILD_TOOLS, "aapt2.exe")
ZIPALIGN = os.path.join(BUILD_TOOLS, "zipalign.exe")
APKSIGNER = os.path.join(BUILD_TOOLS, "apksigner.bat")
D8 = os.path.join(BUILD_TOOLS, "d8.bat")
ANDROID_JAR = os.path.join(PLATFORM, "android.jar")

KEYSTORE = os.path.join(os.path.expanduser("~"), ".android", "debug.keystore")

JAVA_DIR = os.path.join(HERE, "java")
JAVA_SRC = os.path.join(JAVA_DIR, "com", "migaraje", "app", "MiGarajeActivity.java")

for label, path in [
    ("aapt2", AAPT2),
    ("zipalign", ZIPALIGN),
    ("apksigner", APKSIGNER),
    ("d8", D8),
    ("android.jar", ANDROID_JAR),
    ("debug.keystore", KEYSTORE),
    ("MiGarajeActivity.java", JAVA_SRC),
]:
    if not os.path.exists(path):
        sys.exit(f"Falta {label}: {path}")


def find_java_home():
    """Localiza un JDK con `javac` (Android Studio trae uno en `jbr`)."""
    candidatos = []
    if os.environ.get("JAVA_HOME"):
        candidatos.append(os.environ["JAVA_HOME"])
    for base in (
        os.path.join(os.environ.get("ProgramFiles", r"C:\Program Files"), "Android", "Android Studio", "jbr"),
        os.path.join(os.environ.get("LOCALAPPDATA", ""), "Programs", "Android Studio", "jbr"),
        os.path.join(os.environ.get("ProgramFiles", r"C:\Program Files"), "Android", "Android Studio", "jre"),
    ):
        candidatos.append(base)
    for base in candidatos:
        if base and os.path.exists(os.path.join(base, "bin", "javac.exe")):
            return base
    return None


def run(cmd, **kwargs):
    print(">", " ".join(str(c) for c in cmd))
    subprocess.run(cmd, check=True, **kwargs)


def compilar_java(java_home):
    """Compila `MiGarajeActivity.java` y lo convierte a `classes.dex`."""
    javac = os.path.join(java_home, "bin", "javac.exe")
    clases = os.path.join(BUILD, "clases")
    os.makedirs(clases, exist_ok=True)
    run(
        [
            javac,
            "-source",
            "8",
            "-target",
            "8",
            "-bootclasspath",
            ANDROID_JAR,
            "-classpath",
            ANDROID_JAR,
            "-encoding",
            "UTF-8",
            "-Xlint:-options",
            "-d",
            clases,
            JAVA_SRC,
        ]
    )

    archivos = []
    for raiz, _dirs, nombres in os.walk(clases):
        for nombre in nombres:
            if nombre.endswith(".class"):
                archivos.append(os.path.join(raiz, nombre))
    if not archivos:
        sys.exit("javac no generó ninguna clase.")

    entorno = dict(os.environ)
    entorno["JAVA_HOME"] = java_home
    dex = os.path.join(BUILD, "dex")
    os.makedirs(dex, exist_ok=True)
    run(
        ["cmd.exe", "/c", D8, "--min-api", "24", "--lib", ANDROID_JAR, "--output", dex] + archivos,
        env=entorno,
    )
    return os.path.join(dex, "classes.dex")


def main():
    java_home = find_java_home()
    if not java_home:
        sys.exit("No encontré un JDK con javac (instala Android Studio o define JAVA_HOME).")
    print(f"JDK: {java_home}")

    shutil.rmtree(BUILD, ignore_errors=True)
    os.makedirs(BUILD, exist_ok=True)
    os.makedirs(DIST, exist_ok=True)

    # 0. código Java (el puente con el teclado del sistema)
    classes_dex = compilar_java(java_home)

    # 1. recursos
    res_zip = os.path.join(BUILD, "resources.zip")
    run([AAPT2, "compile", "--dir", os.path.join(HERE, "res"), "-o", res_zip])

    # 2. manifiesto + recursos -> APK base
    base = os.path.join(BUILD, "base.apk")
    run(
        [
            AAPT2,
            "link",
            "-o",
            base,
            "-I",
            ANDROID_JAR,
            "--manifest",
            os.path.join(HERE, "AndroidManifest.xml"),
            "-R",
            res_zip,
            "--auto-add-overlay",
        ]
    )

    # 3. bibliotecas nativas
    libs = []
    if os.path.isdir(JNI):
        for abi in sorted(os.listdir(JNI)):
            so = os.path.join(JNI, abi, LIB_NAME)
            if os.path.isfile(so):
                libs.append((abi, so))
    if not libs:
        sys.exit(f"No hay bibliotecas nativas en {JNI}. Compila antes con cargo-ndk.")

    # 4. empaquetar todo (las .so van sin comprimir)
    unsigned = os.path.join(BUILD, "unsigned.apk")
    with zipfile.ZipFile(base) as src, zipfile.ZipFile(unsigned, "w") as out:
        for item in src.infolist():
            out.writestr(item, src.read(item.filename))
        with open(classes_dex, "rb") as fh:
            out.writestr("classes.dex", fh.read())
        print(f"  + classes.dex ({os.path.getsize(classes_dex) // 1024} KB)")
        for abi, so in libs:
            info = zipfile.ZipInfo(f"lib/{abi}/{LIB_NAME}")
            info.compress_type = zipfile.ZIP_STORED
            info.external_attr = 0o644 << 16
            with open(so, "rb") as fh:
                out.writestr(info, fh.read())
            print(f"  + lib/{abi}/{LIB_NAME} ({os.path.getsize(so) // 1024} KB)")

    # 5. alinear
    aligned = os.path.join(BUILD, "aligned.apk")
    run([ZIPALIGN, "-f", "-p", "4", unsigned, aligned])

    # 6. firmar con la clave de depuración
    final = os.path.join(DIST, "MiGaraje.apk")
    run(
        [
            "cmd.exe",
            "/c",
            APKSIGNER,
            "sign",
            "--ks",
            KEYSTORE,
            "--ks-key-alias",
            "androiddebugkey",
            "--ks-pass",
            "pass:android",
            "--key-pass",
            "pass:android",
            "--out",
            final,
            aligned,
        ]
    )

    # 7. verificar
    run(["cmd.exe", "/c", APKSIGNER, "verify", "--print-certs", final])

    shutil.copyfile(final, os.path.join(APP, "MiGaraje.apk"))
    size = os.path.getsize(final) / (1024 * 1024)
    print(f"\nListo: {final} ({size:.1f} MB)")
    print(f"Copia: {os.path.join(APP, 'MiGaraje.apk')}")


if __name__ == "__main__":
    main()
